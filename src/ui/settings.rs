use super::{about, shortcuts, AppInner};
use crate::autostart;
use crate::storage::{save_state, Settings, MAX_ZOOM, MIN_ZOOM};
use gtk::prelude::*;
use gtk::{
    ApplicationWindow, CheckButton, Dialog, DialogFlags, Label, Orientation, ResponseType,
    SpinButton,
};
use std::cell::RefCell;
use std::rc::Rc;

pub(super) fn show(
    window: &ApplicationWindow,
    inner: &Rc<RefCell<AppInner>>,
    shortcut: &Rc<shortcuts::GlobalShortcut>,
) {
    let dialog = Dialog::with_buttons(
        Some("Settings"),
        Some(window),
        DialogFlags::MODAL | DialogFlags::DESTROY_WITH_PARENT,
        &[
            ("About", ResponseType::Help),
            ("Cancel", ResponseType::Cancel),
            ("Save", ResponseType::Apply),
        ],
    );
    dialog.add_css_class("luti");
    dialog.set_default_response(ResponseType::Apply);

    let current = inner.borrow().state.settings.clone();
    let auto_copy = CheckButton::with_label("Copy color automatically after capture");
    let sound = CheckButton::with_label("Play capture and copy sounds");
    let choose_palette = CheckButton::with_label("Ask which palette to add history colors to");
    let background = CheckButton::with_label("Keep running in the tray when the window closes");
    let start_at_login = CheckButton::with_label("Start at login");
    let global_shortcut = CheckButton::with_label("Register a global shortcut to capture a color");
    global_shortcut.set_active(current.global_shortcut);
    let autostart_supported = autostart::supported();
    start_at_login.set_sensitive(autostart_supported);
    start_at_login.set_active(autostart_supported && autostart::enabled());
    if !autostart_supported {
        start_at_login.set_tooltip_text(Some("Automatic startup is unavailable in this package."));
    }
    auto_copy.set_active(current.auto_copy);
    sound.set_active(current.sound);
    choose_palette.set_active(current.choose_palette_for_history);
    background.set_active(current.background);

    let content = dialog.content_area();
    content.set_spacing(10);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(16);
    content.set_margin_end(16);
    content.append(&auto_copy);
    content.append(&sound);
    content.append(&choose_palette);
    let zoom_range = current.zoom_range();
    let min_zoom = zoom_input(&content, "_Minimum magnifier zoom (×)", *zoom_range.start());
    let max_zoom = zoom_input(&content, "Ma_ximum magnifier zoom (×)", *zoom_range.end());
    content.append(&background);
    content.append(&start_at_login);
    content.append(&global_shortcut);
    let shortcut_status = Label::new(None);
    shortcut_status.set_xalign(0.0);
    shortcut_status.set_wrap(true);
    shortcut_status.set_max_width_chars(48);
    shortcut
        .status
        .bind_property("text", &shortcut_status, "label")
        .sync_create()
        .build();
    content.append(&shortcut_status);
    let manual_shortcut = Label::new(Some(
        "To set a shortcut manually, add this command in your desktop’s keyboard settings:",
    ));
    manual_shortcut.set_xalign(0.0);
    manual_shortcut.set_wrap(true);
    manual_shortcut.set_max_width_chars(48);
    content.append(&manual_shortcut);
    let command = Label::new(Some(&shortcuts::command()));
    command.set_xalign(0.0);
    command.set_selectable(true);
    command.set_wrap(true);
    command.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    command.set_max_width_chars(48);
    content.append(&command);
    let error_label = Label::new(None);
    error_label.set_wrap(true);
    error_label.set_max_width_chars(48);
    error_label.hide();
    content.append(&error_label);

    let inner = inner.clone();
    let shortcut = shortcut.clone();
    dialog.connect_response(move |dialog, response| {
        if response == ResponseType::Help {
            about::show_about_dialog(Some(dialog.upcast_ref()));
            return;
        }
        if response == ResponseType::Apply {
            min_zoom.update();
            max_zoom.update();
            if min_zoom.value_as_int() > max_zoom.value_as_int() {
                error_label.set_text("Minimum zoom must be less than or equal to maximum zoom.");
                error_label.show();
                return;
            }
            let mut inner = inner.borrow_mut();
            let settings = Settings {
                auto_copy: auto_copy.is_active(),
                sound: sound.is_active(),
                background: background.is_active(),
                global_shortcut: global_shortcut.is_active(),
                min_zoom: min_zoom.value_as_int(),
                max_zoom: max_zoom.value_as_int(),
                choose_palette_for_history: choose_palette.is_active(),
            };
            if settings != inner.state.settings {
                let mut updated = inner.state.clone();
                updated.settings = settings;
                if let Err(error) = save_state(&updated) {
                    error_label.set_text(&format!("Could not save settings: {error}"));
                    error_label.show();
                    return;
                }
                inner.sound.set(updated.settings.sound);
                inner.state = updated;
            }
            shortcut.set_enabled(inner.state.settings.global_shortcut);
            if autostart_supported && start_at_login.is_active() != autostart::enabled() {
                if let Err(error) = autostart::set_enabled(start_at_login.is_active()) {
                    error_label.set_text(&format!("Could not change automatic startup: {error}"));
                    error_label.show();
                    return;
                }
            }
        }
        dialog.close();
    });
    dialog.present();
}

fn zoom_input(content: &gtk::Box, title: &str, value: i32) -> SpinButton {
    let row = gtk::Box::new(Orientation::Horizontal, 8);
    let label = Label::with_mnemonic(title);
    label.set_xalign(0.0);
    label.set_hexpand(true);
    let input = SpinButton::with_range(f64::from(MIN_ZOOM), f64::from(MAX_ZOOM), 1.0);
    input.set_numeric(true);
    input.set_value(f64::from(value));
    label.set_mnemonic_widget(Some(&input));
    row.append(&label);
    row.append(&input);
    content.append(&row);
    input
}
