use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
use ashpd::zvariant::Value;
use ashpd::WindowIdentifier;
use futures_util::future::{select, Either};
use futures_util::{pin_mut, StreamExt};
use gtk::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Weak;

const CAPTURE_ID: &str = "capture-color";
const PREFERRED_TRIGGER: &str = "CTRL+ALT+p";
const GNOME_MEDIA_KEYS: &str = "org.gnome.settings-daemon.plugins.media-keys";
const GNOME_CUSTOM_KEY: &str = "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding";
const GNOME_SHORTCUT_LIST: &str = "custom-keybindings";
const GNOME_CAPTURE_PATH: &str =
    "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/luti-color-picker/";
const GNOME_TRIGGER: &str = "<Control><Alt>p";

type Capture = dyn Fn(Option<String>);

pub(super) struct GlobalShortcut {
    capture: Weak<Capture>,
    stop: RefCell<Option<async_channel::Sender<()>>>,
    pub status: gtk::EntryBuffer,
}

impl GlobalShortcut {
    pub fn new(capture: Weak<Capture>) -> Self {
        Self {
            capture,
            stop: RefCell::new(None),
            status: gtk::EntryBuffer::new(Some("")),
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        let mut stop = self.stop.borrow_mut();
        if let Some(result) = set_gnome_shortcut(enabled) {
            stop.take();
            let message = match result {
                Ok(Some(trigger)) => format!("Global shortcut in GNOME: {trigger}"),
                Ok(None) => "Desktop shortcut disabled.".into(),
                Err(error) => format!("Could not update the GNOME shortcut: {error}"),
            };
            self.status.set_text(message);
            return;
        }
        if !enabled {
            stop.take();
            self.status.set_text("Desktop shortcut disabled.");
            return;
        }
        if stop.as_ref().is_some_and(|sender| !sender.is_closed()) {
            return;
        }

        let (sender, receiver) = async_channel::bounded(1);
        *stop = Some(sender);
        self.status
            .set_text("Waiting for the desktop to register the shortcut…");

        let capture = self.capture.clone();
        let status = self.status.clone();
        glib::MainContext::default().spawn_local(async move {
            let result = run(&receiver, capture, &status).await;
            if !receiver.is_closed() {
                let message = match result {
                    Ok(()) => "The desktop closed the shortcut session.".into(),
                    Err(error) => format!("Could not register the desktop shortcut: {error}"),
                };
                status.set_text(message);
            }
        });
    }
}

async fn run(
    stop: &async_channel::Receiver<()>,
    capture: Weak<Capture>,
    status: &gtk::EntryBuffer,
) -> Result<(), ashpd::Error> {
    let portal = GlobalShortcuts::new().await?;
    if crate::autostart::supported() {
        let _ = portal
            .connection()
            .call_method(
                Some("org.freedesktop.portal.Desktop"),
                "/org/freedesktop/portal/desktop",
                Some("org.freedesktop.host.portal.Registry"),
                "Register",
                &(crate::APP_ID, HashMap::<&str, Value<'_>>::new()),
            )
            .await;
    }
    if stop.is_closed() {
        return Ok(());
    }
    let session = portal.create_session().await?;

    let listen = async {
        let mut activated = portal.receive_activated().await?;
        let mut closed = session.receive_closed().await?;
        let mut changed = portal.receive_shortcuts_changed().await?;
        let shortcuts = [
            NewShortcut::new(CAPTURE_ID, "Capture a color").preferred_trigger(PREFERRED_TRIGGER)
        ];
        let bound = portal
            .bind_shortcuts(&session, &shortcuts, &WindowIdentifier::default())
            .await?
            .response()?;
        update_status(status, bound.shortcuts());

        loop {
            let activation = activated.next();
            let change = changed.next();
            let session_closed = closed.next();
            pin_mut!(activation, change, session_closed);
            match select(select(activation, change), session_closed).await {
                Either::Left((Either::Left((Some(event), _)), _)) => {
                    if let Some(capture) = capture.upgrade() {
                        if event.shortcut_id() == CAPTURE_ID {
                            let token = event
                                .options()
                                .get("activation_token")
                                .and_then(|value| value.downcast_ref::<&str>().ok())
                                .map(str::to_owned);
                            capture(token);
                        }
                    } else {
                        break;
                    }
                }
                Either::Left((Either::Right((Some(event), _)), _)) => {
                    update_status(status, event.shortcuts());
                }
                _ => break,
            }
        }
        Ok(())
    };
    let cancel = stop.recv();
    pin_mut!(listen, cancel);
    let result = match select(cancel, listen).await {
        Either::Left(_) => Ok(()),
        Either::Right((result, _)) => result,
    };
    let _ = session.close().await;
    result
}

fn set_gnome_shortcut(enabled: bool) -> Option<Result<Option<String>, glib::BoolError>> {
    if !crate::autostart::supported()
        || !std::env::var("XDG_CURRENT_DESKTOP")
            .unwrap_or_default()
            .split(':')
            .any(|desktop| desktop.eq_ignore_ascii_case("GNOME"))
    {
        return None;
    }

    let schemas = gio::SettingsSchemaSource::default()?;
    let media_schema = schemas.lookup(GNOME_MEDIA_KEYS, true)?;
    let shortcut_schema = schemas.lookup(GNOME_CUSTOM_KEY, true)?;
    let media_keys = gio::Settings::new_full(&media_schema, gio::SettingsBackend::NONE, None);
    let shortcut = |path: &str| {
        gio::Settings::new_full(&shortcut_schema, gio::SettingsBackend::NONE, Some(path))
    };
    let mut paths = media_keys
        .strv(GNOME_SHORTCUT_LIST)
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let exists = paths.iter().any(|path| path == GNOME_CAPTURE_PATH);
    let capture = shortcut(GNOME_CAPTURE_PATH);

    Some((|| {
        if !enabled {
            if exists {
                paths.retain(|path| path != GNOME_CAPTURE_PATH);
                media_keys.set_strv(GNOME_SHORTCUT_LIST, paths)?;
                gio::Settings::sync();
            }
            return Ok(None);
        }

        if !exists {
            let trigger = gtk::accelerator_parse(GNOME_TRIGGER);
            if paths
                .iter()
                .any(|path| gtk::accelerator_parse(shortcut(path).string("binding")) == trigger)
            {
                return Err(glib::bool_error!(
                    "Ctrl+Alt+P is already assigned to a custom shortcut."
                ));
            }
            capture.set_string("name", "Luti Color Picker — Capture a color")?;
            capture.set_string("binding", GNOME_TRIGGER)?;
        }
        let launch_command = command();
        if capture.string("command") != launch_command {
            capture.set_string("command", &launch_command)?;
        }
        if !exists {
            paths.push(GNOME_CAPTURE_PATH.into());
            media_keys.set_strv(GNOME_SHORTCUT_LIST, paths)?;
        }
        gio::Settings::sync();

        let label = gtk::accelerator_parse(capture.string("binding"))
            .map(|(key, modifiers)| gtk::accelerator_get_label(key, modifiers).to_string())
            .unwrap_or_else(|| "no key combination assigned".into());
        Ok(Some(label))
    })())
}

fn update_status(
    status: &gtk::EntryBuffer,
    shortcuts: &[ashpd::desktop::global_shortcuts::Shortcut],
) {
    let trigger = shortcuts
        .iter()
        .find(|shortcut| shortcut.id() == CAPTURE_ID)
        .map(|shortcut| shortcut.trigger_description())
        .filter(|trigger| !trigger.is_empty());
    let message = match trigger {
        Some(trigger) => format!("Global shortcut: {trigger}"),
        None => "The desktop has not assigned a key combination.".into(),
    };
    status.set_text(message);
}

pub(super) fn command() -> String {
    if std::path::Path::new("/.flatpak-info").exists() {
        return format!("flatpak run {} --pick", crate::APP_ID);
    }
    if std::env::var_os("SNAP").is_some() {
        return "snap run luti-color-picker --pick".into();
    }
    let launcher = crate::autostart::launcher().unwrap_or_else(|_| "luti-color-picker".into());
    format!("{} --pick", glib::shell_quote(launcher).to_string_lossy())
}
