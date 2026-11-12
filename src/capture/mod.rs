use gdk::prelude::*;
use gdk_pixbuf::{InterpType, Pixbuf};
use std::f64::consts::PI;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::ConnectionExt;

pub const SNAP_SIZE: i32 = 120;

/// Checks if running on a Wayland display server
pub fn is_wayland() -> bool {
    if let Some(display) = gdk::Display::default() {
        return display.type_().name().contains("Wayland");
    }
    std::env::var("WAYLAND_DISPLAY").is_ok()
}

/// Captures the full workspace across all monitors seamlessly
pub fn capture_fullscreen() -> Option<Pixbuf> {
    if !is_wayland() {
        return capture_x11();
    }

    let tmp_path = format!(
        "/tmp/luti-color-picker-{}_{}.png",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );

    // 1. Try spectacle (KDE Plasma default)
    let spectacle_status = std::process::Command::new("spectacle")
        .args(&["-b", "-n", "-f", "-o", &tmp_path])
        .status();

    if let Ok(st) = spectacle_status {
        if st.success() {
            if let Ok(pb) = Pixbuf::from_file(&tmp_path) {
                let _ = std::fs::remove_file(&tmp_path);
                return Some(pb);
            }
        }
        let _ = std::fs::remove_file(&tmp_path);
    }

    // 2. Try grim (wlroots/Wayland)
    let grim_status = std::process::Command::new("grim").arg(&tmp_path).status();

    if let Ok(st) = grim_status {
        if st.success() {
            if let Ok(pb) = Pixbuf::from_file(&tmp_path) {
                let _ = std::fs::remove_file(&tmp_path);
                return Some(pb);
            }
        }
        let _ = std::fs::remove_file(&tmp_path);
    }

    None
}

fn capture_x11() -> Option<Pixbuf> {
    let (connection, screen) = x11rb::connect(None).ok()?;
    let screen = &connection.setup().roots[screen];
    let width = screen.width_in_pixels;
    let height = screen.height_in_pixels;
    let (image, visual_id) =
        x11rb::image::Image::get(&connection, screen.root, 0, 0, width, height).ok()?;
    let visual = screen
        .allowed_depths
        .iter()
        .flat_map(|depth| &depth.visuals)
        .find(|visual| visual.visual_id == visual_id)?;
    let channels = [visual.red_mask, visual.green_mask, visual.blue_mask].map(|mask| {
        let shift = mask.trailing_zeros().min(31);
        (mask, shift, u64::from(mask >> shift).max(1))
    });
    let mut pixels = Vec::with_capacity(usize::from(width) * usize::from(height) * 3);
    for y in 0..height {
        for x in 0..width {
            let pixel = image.get_pixel(x, y);
            pixels.extend(channels.map(|(mask, shift, max)| {
                ((u64::from((pixel & mask) >> shift) * 255 + max / 2) / max) as u8
            }));
        }
    }
    Some(Pixbuf::from_mut_slice(
        pixels,
        gdk_pixbuf::Colorspace::Rgb,
        false,
        8,
        i32::from(width),
        i32::from(height),
        i32::from(width) * 3,
    ))
}

pub fn x11_pointer() -> Option<(i32, i32)> {
    let (connection, screen) = x11rb::connect(None).ok()?;
    let root = connection.setup().roots[screen].root;
    let pointer = connection.query_pointer(root).ok()?.reply().ok()?;
    pointer
        .same_screen
        .then_some((pointer.root_x.into(), pointer.root_y.into()))
}

/// Extracts RGB pixel from the center of the snapshot
pub fn get_center_pixel(pb: &Pixbuf) -> (u8, u8, u8) {
    let center_x = SNAP_SIZE / 2;
    let center_y = SNAP_SIZE / 2;
    let n_channels = pb.n_channels();
    let rowstride = pb.rowstride();

    let pixels = unsafe { pb.pixels() };
    let offset = (rowstride * center_y + center_x * n_channels) as usize;

    if offset + 2 < pixels.len() {
        (pixels[offset], pixels[offset + 1], pixels[offset + 2])
    } else {
        (0, 0, 0)
    }
}

/// Creates a 60x60 contextual crop of the center of the snapshot
pub fn create_context_crop(pb: &Pixbuf) -> Option<Pixbuf> {
    pb.scale_simple(SNAP_SIZE / 2, SNAP_SIZE / 2, InterpType::Tiles)
}

/// Creates a pure color swatch thumbnail (60x60) for portal captures
pub fn create_color_swatch_pixbuf(r: u8, g: u8, b: u8, w: i32, h: i32) -> Option<Pixbuf> {
    let surface = cairo::ImageSurface::create(cairo::Format::Rgb24, w, h).ok()?;
    let ctx = cairo::Context::new(&surface).ok()?;
    ctx.set_source_rgb(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0);
    let _ = ctx.paint();
    drop(ctx);
    gdk::pixbuf_get_from_surface(&surface, 0, 0, w, h)
}

/// Encodes a Pixbuf to PNG in Base64
pub fn pixbuf_to_base64_png(pb: &Pixbuf) -> Option<String> {
    let buffer = pb.save_to_bufferv("png", &[]).ok()?;
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    Some(STANDARD.encode(buffer))
}

/// Decodes Base64 PNG into a Pixbuf
pub fn base64_png_to_pixbuf(b64: &str) -> Option<Pixbuf> {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    let bytes = STANDARD.decode(b64).ok()?;
    let loader = gdk_pixbuf::PixbufLoader::with_type("png").ok()?;
    loader.write(&bytes).ok()?;
    loader.close().ok()?;
    loader.pixbuf()
}

/// Uses XDG Desktop Portal to pick a color across any window in Wayland
pub async fn pick_color_portal() -> Option<(u8, u8, u8)> {
    let request = ashpd::desktop::Color::pick().send().await.ok()?;
    let color = request.response().ok()?;
    let r = (color.red() * 255.0).round().clamp(0.0, 255.0) as u8;
    let g = (color.green() * 255.0).round().clamp(0.0, 255.0) as u8;
    let b = (color.blue() * 255.0).round().clamp(0.0, 255.0) as u8;
    Some((r, g, b))
}

/// Draws the circular magnifier surface and returns a Pixbuf for the cursor
pub fn render_magnifier(
    latest_pb: &Pixbuf,
    zoom_level: i32,
    formatted_text: &str,
    color: (u8, u8, u8),
) -> Option<Pixbuf> {
    let scaled_pb = latest_pb.scale_simple(SNAP_SIZE * 2, SNAP_SIZE * 2, InterpType::Nearest)?;
    let scaled_subset =
        scaled_pb.new_subpixbuf(SNAP_SIZE / 2 + 1, SNAP_SIZE / 2 + 1, SNAP_SIZE, SNAP_SIZE);

    let target_width = SNAP_SIZE * zoom_level;
    let target_height = SNAP_SIZE * zoom_level;

    let base =
        cairo::ImageSurface::create(cairo::Format::ARgb32, target_width, target_height).ok()?;
    let ctx = cairo::Context::new(&base).ok()?;

    ctx.scale(zoom_level as f64, zoom_level as f64);

    // Circular path
    let radius = SNAP_SIZE as f64 / 2.0;
    ctx.arc(radius, radius, radius, 0.0, 2.0 * PI);

    // Paste in the screenshot
    ctx.set_source_pixbuf(&scaled_subset, 0.0, 0.0);
    let _ = ctx.save();
    ctx.clip_preserve();
    let _ = ctx.paint();

    // Outside border
    ctx.set_source_rgba(0.0, 0.0, 0.0, 1.0);
    ctx.set_line_width(4.0);
    let _ = ctx.stroke();
    let _ = ctx.restore();

    // Inside square border (targeting reticle)
    ctx.set_source_rgba(1.0, 0.0, 0.0, 0.5);
    ctx.set_line_width(1.0);
    let center = radius;
    ctx.move_to(center - 2.0, center - 2.0);
    ctx.rel_line_to(3.0, 0.0);
    ctx.rel_line_to(0.0, 3.0);
    ctx.rel_line_to(-3.0, 0.0);
    ctx.rel_line_to(0.0, -3.0);
    let _ = ctx.stroke();

    // Text & Color badge
    let rect_border_width = 2.0;
    let mut font_size = 6.0;
    let max_rwidth = SNAP_SIZE as f64 * 0.7;

    for _ in 0..50 {
        ctx.set_font_size(font_size);
        if let Ok(extents) = ctx.text_extents(formatted_text) {
            let rwidth = extents.width() + (2.0 * rect_border_width);
            if rwidth > max_rwidth {
                font_size -= 1.0;
                break;
            }
            font_size += 1.0;
        }
    }

    ctx.set_font_size(font_size);
    if let Ok(extents) = ctx.text_extents(formatted_text) {
        let text_draw_x = (SNAP_SIZE as f64 * 0.98) - extents.width();
        let text_draw_y = (SNAP_SIZE as f64 * 0.95) - extents.height();

        // Background box for text
        ctx.rectangle(
            text_draw_x - rect_border_width + extents.x_bearing(),
            text_draw_y - rect_border_width + extents.y_bearing(),
            extents.width() + (2.0 * rect_border_width),
            extents.height() + (2.0 * rect_border_width),
        );
        ctx.set_source_rgba(0.0, 0.0, 0.0, 0.7);
        let _ = ctx.fill();

        // Text
        ctx.set_source_rgba(1.0, 1.0, 1.0, 1.0);
        ctx.move_to(text_draw_x, text_draw_y);
        let _ = ctx.show_text(formatted_text);

        // Color swatch square next to text
        let swatch_size = extents.height() + (2.0 * rect_border_width);
        ctx.rectangle(
            text_draw_x - rect_border_width + extents.x_bearing() - swatch_size,
            text_draw_y - rect_border_width + extents.y_bearing(),
            swatch_size,
            swatch_size,
        );
        ctx.set_source_rgba(
            color.0 as f64 / 255.0,
            color.1 as f64 / 255.0,
            color.2 as f64 / 255.0,
            1.0,
        );
        let _ = ctx.fill();
    }

    drop(ctx);

    gdk::pixbuf_get_from_surface(&base, 0, 0, target_width, target_height)
}
