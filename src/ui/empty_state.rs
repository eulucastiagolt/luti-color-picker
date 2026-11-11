use gtk::prelude::*;
use gtk::{Align, Box as GtkBox, Image, Label, Orientation};

pub fn build_empty_state() -> GtkBox {
    let empty_box = GtkBox::new(Orientation::Vertical, 0);

    let icon = Image::from_icon_name(Some(crate::APP_ID), gtk::IconSize::Dialog);
    icon.set_valign(Align::End);
    empty_box.pack_start(&icon, true, true, 0);

    let nocol1 = Label::new(Some("No Colours"));
    nocol1.set_widget_name("empty-heading");
    empty_box.pack_start(&nocol1, false, false, 12);

    let nocol2 = Label::new(Some("You haven't picked any colours."));
    nocol2.set_valign(Align::Start);
    empty_box.pack_start(&nocol2, true, true, 0);

    empty_box
}
