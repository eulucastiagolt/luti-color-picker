use gdk::prelude::*;
use gdk::EventMask;
use gdk_pixbuf::Pixbuf;
use gtk::prelude::*;
use gtk::{DrawingArea, Window, WindowType};
use std::cell::RefCell;
use std::f64::consts::PI;
use std::rc::Rc;

use crate::color::formatters::format_color;
use crate::color::ColorDatabase;

pub struct OverlayState {
    pub screenshot: Pixbuf,
    pub mouse_pos: (f64, f64),
    pub zoom_level: i32,
    pub formatter: String,
    pub color_db: ColorDatabase,
}

pub struct MagnifierOverlay;

impl MagnifierOverlay {
    pub fn start<F, C>(
        screenshot: Pixbuf,
        screen: &gdk::Screen,
        monitor_index: i32,
        formatter: String,
        snark_mode: bool,
        activation_token: Option<String>,
        on_pick: F,
        on_cancel: C,
    ) where
        F: Fn(u8, u8, u8, Pixbuf) + 'static,
        C: Fn() + 'static,
    {
        let window = Window::new(WindowType::Toplevel);
        if let Some(token) = activation_token.as_deref() {
            window.set_startup_id(token);
        }
        window.set_title(crate::APP_NAME);
        window.set_decorated(false);
        window.set_app_paintable(true);
        window.set_can_focus(true);
        window.set_screen(screen);
        window.fullscreen_on_monitor(screen, monitor_index);
        window.set_keep_above(true);

        // Hide system mouse cursor on the magnifier canvas
        if let Some(display) = gdk::Display::default() {
            let blank_cursor = gdk::Cursor::from_name(&display, "none");
            window.connect_realize(move |w| {
                if let Some(gdk_win) = w.window() {
                    gdk_win.set_cursor(blank_cursor.as_ref());
                }
            });
        }

        // Grab seat and keyboard/pointer focus on map
        window.connect_map(|w| {
            w.grab_focus();
            if let Some(gw) = w.window() {
                let display = gw.display();
                if let Some(seat) = display.default_seat() {
                    let _ = seat.grab(
                        &gw,
                        gdk::SeatCapabilities::ALL,
                        false,
                        None,
                        None,
                        None,
                    );
                }
            }
        });

        window.add_events(
            EventMask::POINTER_MOTION_MASK
                | EventMask::BUTTON_PRESS_MASK
                | EventMask::SCROLL_MASK
                | EventMask::KEY_PRESS_MASK,
        );

        let initial_mouse = {
            let display = screen.display();
            let seat = display.default_seat();
            let pointer = seat.and_then(|s| s.pointer());
            if let Some(ptr) = pointer {
                let (_, px, py) = ptr.position();
                let (gx, gy) = if let Some(m) = display.monitor(monitor_index) {
                    let g = m.geometry();
                    (g.x(), g.y())
                } else {
                    (0, 0)
                };
                let lx = (px - gx).clamp(0, (screenshot.width() - 1).max(0)) as f64;
                let ly = (py - gy).clamp(0, (screenshot.height() - 1).max(0)) as f64;
                (lx, ly)
            } else {
                (
                    screenshot.width() as f64 / 2.0,
                    screenshot.height() as f64 / 2.0,
                )
            }
        };

        let state = Rc::new(RefCell::new(OverlayState {
            screenshot,
            mouse_pos: initial_mouse,
            zoom_level: 3,
            formatter,
            color_db: ColorDatabase::new(snark_mode),
        }));

        let drawing_area = DrawingArea::new();
        window.add(&drawing_area);

        // Draw event
        {
            let state_clone = state.clone();
            drawing_area.connect_draw(move |_, ctx| {
                let mut st = state_clone.borrow_mut();
                let (mx, my) = st.mouse_pos;
                let zoom = st.zoom_level as f64;
                let radius = 70.0; // 140px diameter loupe

                // 1. Paint background screenshot 1:1 on the monitor
                ctx.set_source_pixbuf(&st.screenshot, 0.0, 0.0);
                let _ = ctx.paint();

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
                let badge_x = (mx - (badge_w / 2.0))
                    .clamp(10.0, (st.screenshot.width() as f64 - badge_w - 10.0).max(10.0));

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

                glib::Propagation::Proceed
            });
        }

        // Mouse Motion
        {
            let state_clone = state.clone();
            let da_clone = drawing_area.clone();
            window.connect_motion_notify_event(move |_, ev| {
                let (x, y) = ev.position();
                state_clone.borrow_mut().mouse_pos = (x, y);
                da_clone.queue_draw();
                glib::Propagation::Proceed
            });
        }

        // Scroll (Zoom level 2 to 7)
        {
            let state_clone = state.clone();
            let da_clone = drawing_area.clone();
            window.connect_scroll_event(move |_, ev| {
                let mut st = state_clone.borrow_mut();
                match ev.direction() {
                    gdk::ScrollDirection::Up => {
                        if st.zoom_level < 7 {
                            st.zoom_level += 1;
                        }
                    }
                    gdk::ScrollDirection::Down => {
                        if st.zoom_level > 2 {
                            st.zoom_level -= 1;
                        }
                    }
                    _ => {}
                }
                da_clone.queue_draw();
                glib::Propagation::Proceed
            });
        }

        let picked = Rc::new(RefCell::new(false));

        let ungrab = |w: &Window| {
            if let Some(gw) = w.window() {
                let display = gw.display();
                if let Some(seat) = display.default_seat() {
                    seat.ungrab();
                }
                gw.set_cursor(None);
            }
        };

        // Window destroy (Cancel cleanup and cursor restore)
        {
            let picked_clone = picked.clone();
            let on_cancel_rc = Rc::new(on_cancel);
            window.connect_destroy(move |w| {
                if let Some(gw) = w.window() {
                    let display = gw.display();
                    if let Some(seat) = display.default_seat() {
                        seat.ungrab();
                    }
                    gw.set_cursor(None);
                }
                if !*picked_clone.borrow() {
                    *picked_clone.borrow_mut() = true;
                    on_cancel_rc();
                }
            });
        }

        // Key Press (Escape to cancel)
        {
            let win_weak = window.downgrade();
            window.connect_key_press_event(move |_, ev| {
                if ev.keyval() == gdk::keys::constants::Escape {
                    if let Some(win) = win_weak.upgrade() {
                        ungrab(&win);
                        unsafe { win.destroy(); }
                    }
                    return glib::Propagation::Stop;
                }
                glib::Propagation::Proceed
            });
        }

        // Click event (Pick pixel or Cancel)
        {
            let state_clone = state;
            let win_weak = window.downgrade();
            let on_pick_rc = Rc::new(on_pick);
            let picked_clone = picked.clone();
            window.connect_button_press_event(move |_, ev| {
                // Ignore double/triple click events or non-press events
                if ev.event_type() != gdk::EventType::ButtonPress {
                    return glib::Propagation::Stop;
                }

                if ev.button() == 1 {
                    // Prevent duplicate invocations
                    if *picked_clone.borrow() {
                        return glib::Propagation::Stop;
                    }
                    *picked_clone.borrow_mut() = true;

                    let st = state_clone.borrow();
                    let (mx, my) = ev.position();
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

                    // Extract 60x60 context crop
                    let crop_size = 60;
                    let src_x = (px - crop_size / 2).clamp(0, (st.screenshot.width() - crop_size).max(0));
                    let src_y = (py - crop_size / 2).clamp(0, (st.screenshot.height() - crop_size).max(0));
                    let crop = st.screenshot.new_subpixbuf(
                        src_x,
                        src_y,
                        crop_size.min(st.screenshot.width()),
                        crop_size.min(st.screenshot.height()),
                    );

                    if let Some(win) = win_weak.upgrade() {
                        ungrab(&win);
                        unsafe { win.destroy(); }
                    }
                    on_pick_rc(r, g, b, crop);
                    return glib::Propagation::Stop;
                } else if ev.button() == 3 {
                    // Right click cancels
                    if let Some(win) = win_weak.upgrade() {
                        ungrab(&win);
                        unsafe { win.destroy(); }
                    }
                    return glib::Propagation::Stop;
                }
                glib::Propagation::Stop
            });
        }

        window.show_all();
        if let Some(token) = activation_token.as_deref() {
            window.set_startup_id(token);
        }
        window.present();
    }
}
