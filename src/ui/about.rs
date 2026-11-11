use gtk::prelude::*;
use gtk::{AboutDialog, License, Window};

pub fn show_about_dialog(parent: Option<&Window>) {
    let dialog = AboutDialog::new();
    dialog.set_transient_for(parent);
    dialog.set_destroy_with_parent(true);
    dialog.set_program_name(crate::APP_NAME);
    dialog.set_logo_icon_name(Some(crate::APP_ID));
    dialog.set_version(Some(env!("CARGO_PKG_VERSION")));
    dialog.set_authors(&["Lucas Tiago"]);
    dialog.set_website(Some(env!("CARGO_PKG_REPOSITORY")));
    dialog.set_website_label(Some("GitHub"));
    dialog.set_license_type(License::MitX11);
    dialog.connect_response(|dialog, _| dialog.close());
    dialog.show();
}
