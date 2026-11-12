use gdk::prelude::*;
use gdk_pixbuf::Pixbuf;
use gtk::prelude::*;
use gtk::{
    DrawingArea, EventControllerKey, EventControllerMotion, EventControllerScroll,
    EventControllerScrollFlags, GestureClick, Window,
};
use std::cell::{Cell, RefCell};
use std::f64::consts::PI;
use std::ops::RangeInclusive;
use std::rc::Rc;

use crate::color::formatters::format_color;
use crate::color::ColorDatabase;

pub struct OverlayState {
    pub screenshot: Pixbuf,
    pub mouse_pos: Option<(f64, f64)>,
    pub zoom_level: i32,
    pub formatter: String,
    pub color_db: ColorDatabase,
}

pub struct MagnifierOverlay;

impl MagnifierOverlay {
    pub fn start<F, C>(
        screenshot: Pixbuf,
        monitor: &gdk::Monitor,
        formatter: String,
        zoom_range: RangeInclusive<i32>,
        snark_mode: bool,
        activation_token: Option<String>,
        on_pick: F,
        on_cancel: C,
    ) where
        F: Fn(u8, u8, u8, Pixbuf) + 'static,
        C: Fn() + 'static,
    {
        let window = Window::new();
        if let Some(token) = activation_token.as_deref() {
            window.set_startup_id(token);
        }
        window.set_title(Some(crate::APP_NAME));
        window.set_decorated(false);
        window.set_cursor_from_name(Some("none"));
        window.fullscreen_on_monitor(monitor);
        let state = Rc::new(RefCell::new(OverlayState {
            screenshot,
            mouse_pos: None,
            zoom_level: 3.clamp(*zoom_range.start(), *zoom_range.end()),
            formatter,
            color_db: ColorDatabase::new(snark_mode),
        }));

        let drawing_area = DrawingArea::new();
        window.set_child(Some(&drawing_area));

        // Draw event
        {
            let state_clone = state.clone();
            drawing_area.set_draw_func(move |area, ctx, width, height| {
                let mut st = state_clone.borrow_mut();
                ctx.scale(
                    f64::from(width) / f64::from(st.screenshot.width()),
                    f64::from(height) / f64::from(st.screenshot.height()),
                );
                let zoom = st.zoom_level as f64;
                let radius = 70.0; // 140px diameter loupe

                // 1. Paint background screenshot 1:1 on the monitor
                ctx.set_source_pixbuf(&st.screenshot, 0.0, 0.0);
                let _ = ctx.paint();

                if st.mouse_pos.is_none() {
                    st.mouse_pos = area.native().and_then(|native| {
                        let surface = native.surface()?;
                        let pointer = surface.display().default_seat()?.pointer()?;
                        let (x, y, _) = surface.device_position(&pointer)?;
                        Some((x, y))
                    });
                }
                let Some((x, y)) = st.mouse_pos else {
                    return;
                };
                let (mx, my) = screenshot_position(&st.screenshot, area, x, y);

                // 2. Extract current pixel RGB
                let px = (mx as i32).clamp(0, st.screenshot.width() - 1);
                let py = (my as i32).clamp(0, st.screenshot.height() - 1);
                let n_channels = st.screenshot.n_channels();
                let rowstride = st.screenshot.rowstride();
                let pixels = unsafe { st.screenshot.pixels() };
                let offset = (rowstride * py + px * n_channels) as usize;

                let (r, g, b) = if offset + 2 < pixels.len() {
                    (pixels[offset], pixels[offset + 1], pixels[offset + 2])
                } else {
                    (0, 0, 0)
                };

                let col_name = st.color_db.closest_name(r, g, b);
                let formatted_text = format_color(&st.formatter, r, g, b, col_name);

                // 3. Draw circular loupe with nearest-neighbor zoom
                let _ = ctx.save();
                ctx.arc(mx, my, radius, 0.0, 2.0 * PI);
                ctx.clip_preserve();

                ctx.translate(mx, my);
                ctx.scale(zoom, zoom);
                ctx.translate(-mx, -my);

                ctx.set_source_pixbuf(&st.screenshot, 0.0, 0.0);
                let pattern = ctx.source();
                pattern.set_filter(cairo::Filter::Nearest);
                let _ = ctx.paint();
                let _ = ctx.restore();

                // 4. Draw outer border of magnifier
                ctx.arc(mx, my, radius, 0.0, 2.0 * PI);
                ctx.set_source_rgba(0.1, 0.1, 0.1, 1.0);
                ctx.set_line_width(4.0);
                let _ = ctx.stroke();

                // 5. Draw inner red reticle box around target pixel
                let reticle_half = (zoom / 2.0).round().max(1.0);
                ctx.set_source_rgba(1.0, 0.0, 0.0, 0.85);
                ctx.set_line_width(1.5);
                ctx.rectangle(
                    mx - reticle_half,
                    my - reticle_half,
                    reticle_half * 2.0 + 1.0,
                    reticle_half * 2.0 + 1.0,
                );
                let _ = ctx.stroke();

                // Small crosshair guide lines
                ctx.set_source_rgba(1.0, 0.0, 0.0, 0.4);
                ctx.set_line_width(1.0);
                ctx.move_to(mx - radius + 10.0, my);
                ctx.line_to(mx - reticle_half - 3.0, my);
                ctx.move_to(mx + reticle_half + 3.0, my);
                ctx.line_to(mx + radius - 10.0, my);
                ctx.move_to(mx, my - radius + 10.0);
                ctx.line_to(mx, my - reticle_half - 3.0);
                ctx.move_to(mx, my + reticle_half + 3.0);
                ctx.line_to(mx, my + radius - 10.0);
                let _ = ctx.stroke();

                // 6. Draw color badge at the bottom of the loupe
                // If loupe is near the bottom edge of monitor, flip badge to top
                let badge_h = 24.0;
                let badge_y = if my + radius + 35.0 > st.screenshot.height() as f64 {
                    my - radius - badge_h - 10.0
                } else {
                    my + radius + 10.0
                };
                let padding = 8.0;

                ctx.set_font_size(12.0);
                let text_w = ctx
                    .text_extents(&formatted_text)
                    .map(|e| e.width())
                    .unwrap_or(80.0);
                let badge_w = text_w + 32.0 + (padding * 2.0);
                let badge_x = (mx - (badge_w / 2.0)).clamp(
                    10.0,
                    (st.screenshot.width() as f64 - badge_w - 10.0).max(10.0),
                );

                // Badge background
                ctx.rectangle(badge_x, badge_y, badge_w, badge_h);
                ctx.set_source_rgba(0.0, 0.0, 0.0, 0.85);
                let _ = ctx.fill();

                // Badge border
                ctx.rectangle(badge_x, badge_y, badge_w, badge_h);
                ctx.set_source_rgba(1.0, 1.0, 1.0, 0.3);
                ctx.set_line_width(1.0);
                let _ = ctx.stroke();

                // Swatch inside badge
                ctx.rectangle(badge_x + 4.0, badge_y + 4.0, 16.0, 16.0);
                ctx.set_source_rgb(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0);
                let _ = ctx.fill();

                ctx.rectangle(badge_x + 4.0, badge_y + 4.0, 16.0, 16.0);
                ctx.set_source_rgba(0.0, 0.0, 0.0, 0.5);
                ctx.set_line_width(1.0);
                let _ = ctx.stroke();

                // Text inside badge
                ctx.set_source_rgba(1.0, 1.0, 1.0, 1.0);
                ctx.move_to(badge_x + 26.0, badge_y + 17.0);
                let _ = ctx.show_text(&formatted_text);
            });
        }

        let motion = EventControllerMotion::new();
        {
            let state = state.clone();
            let area = drawing_area.clone();
            let update_position = move |_: &EventControllerMotion, x, y| {
                state.borrow_mut().mouse_pos = Some((x, y));
                area.queue_draw();
            };
            motion.connect_enter(update_position.clone());
            motion.connect_motion(update_position);
        }
        window.add_controller(motion);

        let scroll = EventControllerScroll::new(
            EventControllerScrollFlags::VERTICAL | EventControllerScrollFlags::DISCRETE,
        );
        {
            let state = state.clone();
            let area = drawing_area.clone();
            scroll.connect_scroll(move |_, _, dy| {
                let mut state = state.borrow_mut();
                if dy != 0.0 {
                    state.zoom_level = (state.zoom_level - dy.signum() as i32)
                        .clamp(*zoom_range.start(), *zoom_range.end());
                    area.queue_draw();
                }
                glib::Propagation::Stop
            });
        }
        window.add_controller(scroll);

        let finished = Rc::new(Cell::new(false));
        window.connect_close_request({
            let finished = finished.clone();
            move |_| {
                if !finished.replace(true) {
                    on_cancel();
                }
                glib::Propagation::Proceed
            }
        });

        let keys = EventControllerKey::new();
        let weak_window = window.downgrade();
        keys.connect_key_pressed(move |_, key, _, _| {
            if key == gdk::Key::Escape {
                if let Some(window) = weak_window.upgrade() {
                    window.close();
                }
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        window.add_controller(keys);

        let click = GestureClick::new();
        click.set_button(0);
        let weak_window = window.downgrade();
        click.connect_pressed(move |gesture, count, x, y| {
            let Some(window) = weak_window.upgrade() else {
                return;
            };
            match gesture.current_button() {
                1 if count == 1 && !finished.replace(true) => {
                    let state = state.borrow();
                    let screenshot = &state.screenshot;
                    let (x, y) = screenshot_position(screenshot, &drawing_area, x, y);
                    let x = (x as i32).clamp(0, screenshot.width() - 1);
                    let y = (y as i32).clamp(0, screenshot.height() - 1);
                    let pixels = screenshot.read_pixel_bytes();
                    let offset =
                        (screenshot.rowstride() * y + screenshot.n_channels() * x) as usize;
                    let [r, g, b] = [pixels[offset], pixels[offset + 1], pixels[offset + 2]];
                    let width = 60.min(screenshot.width());
                    let height = 60.min(screenshot.height());
                    let crop = screenshot.new_subpixbuf(
                        (x - width / 2).clamp(0, screenshot.width() - width),
                        (y - height / 2).clamp(0, screenshot.height() - height),
                        width,
                        height,
                    );
                    window.close();
                    on_pick(r, g, b, crop);
                }
                3 => window.close(),
                _ => {}
            }
        });
        window.add_controller(click);
        window.present();
    }
}

fn screenshot_position(screenshot: &Pixbuf, area: &DrawingArea, x: f64, y: f64) -> (f64, f64) {
    (
        x * f64::from(screenshot.width()) / f64::from(area.width().max(1)),
        y * f64::from(screenshot.height()) / f64::from(area.height().max(1)),
    )
}
