use gtk::prelude::*;
use gtk::{Align, Box as GtkBox, Button, Label, Orientation};

pub fn build_empty_state(capture: &Button) -> GtkBox {
    let content = GtkBox::new(Orientation::Vertical, 12);
    content.set_valign(Align::Center);
    content.set_halign(Align::Center);
    content.add_css_class("empty-state");

    let icon = super::brand_icon(72);
    content.append(&icon);

    let heading = Label::new(Some("No colors captured"));
    heading.add_css_class("empty-heading");
    content.append(&heading);

    let button = Button::with_label("Capture color");
    button.add_css_class("capture");
    button.set_halign(Align::Center);
    let capture = capture.downgrade();
    button.connect_clicked(move |_| {
        if let Some(capture) = capture.upgrade() {
            if capture.is_sensitive() {
                capture.emit_clicked();
            }
        }
    });
    content.append(&button);
    content
}
