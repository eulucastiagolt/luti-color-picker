use gdk::prelude::*;

pub(super) fn copy_to_clipboard(text: &str) -> bool {
    let Some(display) = gdk::Display::default() else {
        return false;
    };
    let provider = gdk::ContentProvider::for_value(&text.to_value());
    display.clipboard().set_content(Some(&provider)).is_ok()
}
