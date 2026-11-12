use gdk::prelude::*;
use std::io::Write;
use std::process::{Command, Stdio};

/// Copies text to the system clipboard across Wayland, X11, and GTK.
pub fn copy_to_clipboard(text: &str) {
    // 1. GTK Clipboard (Selection & Primary)
    if let Some(display) = gdk::Display::default() {
        let clipboard = display.clipboard();
        clipboard.set_text(text);
        clipboard.store_async(glib::Priority::DEFAULT, gio::Cancellable::NONE, |_| {});
        display.primary_clipboard().set_text(text);
    }

    // 2. Wayland native clipboard tool (wl-copy)
    // On Wayland environments (such as KDE Plasma 6), wl-copy ensures the clipboard
    // data is immediately propagated to all Wayland and XWayland applications.
    if let Ok(mut child) = Command::new("wl-copy").stdin(Stdio::piped()).spawn() {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = child.wait();
        return;
    }

    // 3. X11 clipboard tools (xsel / xclip) fallback
    if let Ok(mut child) = Command::new("xsel")
        .args(&["-b", "-i"])
        .stdin(Stdio::piped())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = child.wait();
    } else if let Ok(mut child) = Command::new("xclip")
        .args(&["-selection", "clipboard"])
        .stdin(Stdio::piped())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = child.wait();
    }
}
