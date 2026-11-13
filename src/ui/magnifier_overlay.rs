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

type PickedColor = (u8, u8, u8, Pixbuf);
type FinishCapture = Rc<dyn Fn(Option<PickedColor>)>;

pub struct MagnifierOverlay;

impl MagnifierOverlay {
    pub fn start<F, C>(
        screens: Vec<(Pixbuf, gdk::Monitor)>,
        formatter: String,
        zoom_range: RangeInclusive<i32>,
        snark_mode: bool,
        activation_token: Option<String>,
        on_pick: F,
        on_close: C,
    ) where
        F: Fn(u8, u8, u8, Pixbuf) + 'static,
        C: Fn() + 'static,
    {
        let windows = Rc::new(RefCell::new(Vec::<glib::WeakRef<Window>>::new()));
        let finish: FinishCapture = {
            let windows = windows.clone();
            let finished = Cell::new(false);
            Rc::new(move |color| {
                if finished.replace(true) {
                    return;
                }
                if let Some((r, g, b, crop)) = color {
                    on_pick(r, g, b, crop);
                }
                for window in windows.borrow().iter().filter_map(glib::WeakRef::upgrade) {
                    window.destroy();
                }
                on_close();
            })
        };
        let zoom = Rc::new(Cell::new(3.clamp(*zoom_range.start(), *zoom_range.end())));
        let formatter: Rc<str> = formatter.into();
        let color_db = Rc::new(RefCell::new(ColorDatabase::new(snark_mode)));
        let overlays: Vec<_> = screens
            .into_iter()
            .map(|(screenshot, monitor)| {
                let window = Self::build_window(
                    screenshot,
                    formatter.clone(),
                    color_db.clone(),
                    zoom.clone(),
                    zoom_range.clone(),
                    finish.clone(),
                );
                window.fullscreen_on_monitor(&monitor);
                windows.borrow_mut().push(window.downgrade());
                window
            })
            .collect();
        if let (Some(window), Some(token)) = (overlays.last(), activation_token) {
            window.set_startup_id(&token);
        }
        for window in overlays {
            window.present();
        }
    }

    fn build_window(
        screenshot: Pixbuf,
        formatter: Rc<str>,
        color_db: Rc<RefCell<ColorDatabase>>,
        zoom: Rc<Cell<i32>>,
        zoom_range: RangeInclusive<i32>,
        finish: FinishCapture,
    ) -> Window {
        let window = Window::new();
        window.set_title(Some(crate::APP_NAME));
        window.set_decorated(false);
        window.set_cursor_from_name(Some("none"));
        let mouse_pos = Rc::new(Cell::new(None));

        let drawing_area = DrawingArea::new();
        window.set_child(Some(&drawing_area));

        // Draw event
        {
            let screenshot = screenshot.clone();
            let mouse_pos = mouse_pos.clone();
            let zoom = zoom.clone();
            let first_draw = Cell::new(true);
            drawing_area.set_draw_func(move |area, ctx, width, height| {
                ctx.scale(
                    f64::from(width) / f64::from(screenshot.width()),
                    f64::from(height) / f64::from(screenshot.height()),
                );
                let zoom = f64::from(zoom.get());
                let radius = 70.0; // 140px diameter loupe

                // 1. Paint background screenshot 1:1 on the monitor
                ctx.set_source_pixbuf(&screenshot, 0.0, 0.0);
                let _ = ctx.paint();

                if first_draw.replace(false) && mouse_pos.get().is_none() {
                    mouse_pos.set(area.native().and_then(|native| {
                        let surface = native.surface()?;
                        let pointer = surface.display().default_seat()?.pointer()?;
                        let (x, y, _) = surface.device_position(&pointer)?;
                        (x >= 0.0 && y >= 0.0 && x < f64::from(width) && y < f64::from(height))
                            .then_some((x, y))
                    }));
                }
                let Some((x, y)) = mouse_pos.get() else {
                    return;
                };
                let (mx, my) = screenshot_position(&screenshot, area, x, y);

                // 2. Extract current pixel RGB
                let px = (mx as i32).clamp(0, screenshot.width() - 1);
                let py = (my as i32).clamp(0, screenshot.height() - 1);
                let n_channels = screenshot.n_channels();
                let rowstride = screenshot.rowstride();
                let pixels = unsafe { screenshot.pixels() };
                let offset = (rowstride * py + px * n_channels) as usize;

                let (r, g, b) = if offset + 2 < pixels.len() {
                    (pixels[offset], pixels[offset + 1], pixels[offset + 2])
                } else {
                    (0, 0, 0)
                };

                let mut color_db = color_db.borrow_mut();
                let col_name = color_db.closest_name(r, g, b);
                let formatted_text = format_color(&formatter, r, g, b, col_name);

                // 3. Draw circular loupe with nearest-neighbor zoom
                let _ = ctx.save();
                ctx.arc(mx, my, radius, 0.0, 2.0 * PI);
                ctx.clip_preserve();

                ctx.translate(mx, my);
                ctx.scale(zoom, zoom);
                ctx.translate(-mx, -my);

                ctx.set_source_pixbuf(&screenshot, 0.0, 0.0);
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
                let badge_y = if my + radius + 35.0 > screenshot.height() as f64 {
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
                let badge_x = (mx - (badge_w / 2.0))
                    .clamp(10.0, (screenshot.width() as f64 - badge_w - 10.0).max(10.0));

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
            let position = mouse_pos.clone();
            let area = drawing_area.clone();
            let update_position = move |_: &EventControllerMotion, x, y| {
                position.set(Some((x, y)));
                area.queue_draw();
            };
            motion.connect_enter(update_position.clone());
            motion.connect_motion(update_position);
            let area = drawing_area.clone();
            motion.connect_leave(move |_| {
                mouse_pos.set(None);
                area.queue_draw();
            });
        }
        window.add_controller(motion);

        let scroll = EventControllerScroll::new(
            EventControllerScrollFlags::VERTICAL | EventControllerScrollFlags::DISCRETE,
        );
        {
            let area = drawing_area.clone();
            scroll.connect_scroll(move |_, _, dy| {
                if dy != 0.0 {
                    zoom.set(
                        (zoom.get() - dy.signum() as i32)
                            .clamp(*zoom_range.start(), *zoom_range.end()),
                    );
                    area.queue_draw();
                }
                glib::Propagation::Stop
            });
        }
        window.add_controller(scroll);

        window.connect_close_request({
            let finish = finish.clone();
            move |_| {
                finish(None);
                glib::Propagation::Stop
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
        click.connect_pressed(move |gesture, count, x, y| match gesture.current_button() {
            1 if count == 1 => {
                let (x, y) = screenshot_position(&screenshot, &drawing_area, x, y);
                let x = (x as i32).clamp(0, screenshot.width() - 1);
                let y = (y as i32).clamp(0, screenshot.height() - 1);
                let pixels = screenshot.read_pixel_bytes();
                let offset = (screenshot.rowstride() * y + screenshot.n_channels() * x) as usize;
                let [r, g, b] = [pixels[offset], pixels[offset + 1], pixels[offset + 2]];
                let width = 60.min(screenshot.width());
                let height = 60.min(screenshot.height());
                let crop = screenshot.new_subpixbuf(
                    (x - width / 2).clamp(0, screenshot.width() - width),
                    (y - height / 2).clamp(0, screenshot.height() - height),
                    width,
                    height,
                );
                finish(Some((r, g, b, crop)));
            }
            3 => finish(None),
            _ => {}
        });
        window.add_controller(click);
        window
    }
}

fn screenshot_position(screenshot: &Pixbuf, area: &DrawingArea, x: f64, y: f64) -> (f64, f64) {
    (
        x * f64::from(screenshot.width()) / f64::from(area.width().max(1)),
        y * f64::from(screenshot.height()) / f64::from(area.height().max(1)),
    )
}
