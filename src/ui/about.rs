use gtk::prelude::*;
use gtk::{
    Align, Box as GtkBox, Dialog, DialogFlags, Label, LinkButton, Notebook, Orientation,
    ResponseType, Window,
};

pub fn show_about_dialog(parent: Option<&Window>) {
    let title = format!("About {}", crate::APP_NAME);
    let dialog = Dialog::with_buttons(
        Some(&title),
        parent,
        DialogFlags::MODAL | DialogFlags::DESTROY_WITH_PARENT,
        &[("Close", ResponseType::Close)],
    );
    dialog.add_css_class("luti");
    dialog.set_titlebar(Some(&super::window_header(&title)));
    dialog.set_default_size(360, -1);
    let tabs = Notebook::new();
    dialog.content_area().append(&tabs);

    let overview = page();
    let icon = super::brand_icon(64);
    overview.append(&icon);
    let title = Label::new(Some(crate::APP_NAME));
    title.style_context().add_class("title");
    overview.append(&title);
    overview.append(&Label::new(Some(env!("CARGO_PKG_VERSION"))));
    overview.append(&LinkButton::with_label(
        env!("CARGO_PKG_REPOSITORY"),
        "GitHub",
    ));
    overview.append(&LinkButton::with_label(
        "https://opensource.org/license/mit",
        "MIT License",
    ));
    tabs.append_page(&overview, Some(&Label::new(Some("About"))));

    let credits = page();
    credits.append(&Label::new(Some("Author")));
    credits.append(&Label::new(Some(env!("CARGO_PKG_AUTHORS"))));
    tabs.append_page(&credits, Some(&Label::new(Some("Credits"))));

    dialog.connect_response(|dialog, _| dialog.close());
    dialog.present();
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
