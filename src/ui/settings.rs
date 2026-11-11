use super::{about, AppInner};
use crate::storage::{save_state, Settings};
use gtk::prelude::*;
use gtk::{ApplicationWindow, CheckButton, Dialog, DialogFlags, Label, ResponseType};
use std::cell::RefCell;
use std::rc::Rc;

pub(super) fn show(window: &ApplicationWindow, inner: &Rc<RefCell<AppInner>>) {
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
    dialog.set_default_response(ResponseType::Apply);

    let current = inner.borrow().state.settings.clone();
    let auto_copy = CheckButton::with_label("Copy color automatically after capture");
    let sound = CheckButton::with_label("Play capture and copy sounds");
    let background = CheckButton::with_label("Keep running in the tray when the window closes");
    auto_copy.set_active(current.auto_copy);
    sound.set_active(current.sound);
    background.set_active(current.background);

    let content = dialog.content_area();
    content.set_spacing(10);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(16);
    content.set_margin_end(16);
    content.add(&auto_copy);
    content.add(&sound);
    content.add(&background);
    let error_label = Label::new(None);
    error_label.set_line_wrap(true);
    error_label.set_max_width_chars(48);
    error_label.set_no_show_all(true);
    content.add(&error_label);

    let inner = inner.clone();
    dialog.connect_response(move |dialog, response| {
        if response == ResponseType::Help {
            about::show_about_dialog(Some(dialog.upcast_ref()));
            return;
        }
        if response == ResponseType::Apply {
            let mut inner = inner.borrow_mut();
            let mut updated = inner.state.clone();
            updated.settings = Settings {
                auto_copy: auto_copy.is_active(),
                sound: sound.is_active(),
                background: background.is_active(),
            };
            if let Err(error) = save_state(&updated) {
                error_label.set_text(&format!("Could not save settings: {error}"));
                error_label.show();
                return;
            }
            inner.sound.set(updated.settings.sound);
            inner.state = updated;
        }
        dialog.close();
    });
    dialog.show_all();
}
