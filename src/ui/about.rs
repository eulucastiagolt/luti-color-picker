use gtk::prelude::*;
use gtk::{
    Align, Box as GtkBox, Dialog, DialogFlags, Image, Label, LinkButton, Notebook, Orientation,
    ResponseType, Window,
};

pub fn show_about_dialog(parent: Option<&Window>) {
    let dialog = Dialog::with_buttons(
        Some(&format!("About {}", crate::APP_NAME)),
        parent,
        DialogFlags::MODAL | DialogFlags::DESTROY_WITH_PARENT,
        &[("Close", ResponseType::Close)],
    );
    dialog.set_default_size(360, -1);
    let tabs = Notebook::new();
    dialog.content_area().add(&tabs);

    let overview = page();
    let icon = Image::from_icon_name(Some(crate::APP_ID), gtk::IconSize::Dialog);
    icon.set_pixel_size(64);
    overview.add(&icon);
    let title = Label::new(Some(crate::APP_NAME));
    title.style_context().add_class("title");
    overview.add(&title);
    overview.add(&Label::new(Some(env!("CARGO_PKG_VERSION"))));
    overview.add(&LinkButton::with_label(
        env!("CARGO_PKG_REPOSITORY"),
        "GitHub",
    ));
    overview.add(&LinkButton::with_label(
        "https://opensource.org/license/mit",
        "MIT License",
    ));
    tabs.append_page(&overview, Some(&Label::new(Some("About"))));

    let credits = page();
    credits.add(&Label::new(Some("Author")));
    credits.add(&Label::new(Some(env!("CARGO_PKG_AUTHORS"))));
    tabs.append_page(&credits, Some(&Label::new(Some("Credits"))));

    dialog.connect_response(|dialog, _| unsafe { dialog.destroy() });
    dialog.show_all();
}

fn page() -> GtkBox {
    let page = GtkBox::new(Orientation::Vertical, 8);
    page.set_margin_top(16);
    page.set_margin_bottom(16);
    page.set_margin_start(16);
    page.set_margin_end(16);
    page.set_halign(Align::Fill);
    page
}
