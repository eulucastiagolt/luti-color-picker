pub mod about;
pub mod clipboard;
pub mod color_card;
pub mod empty_state;
pub mod magnifier_overlay;
mod settings;
mod studio;
pub mod tray;

use crate::capture;
use crate::color::formatters::{format_color, FORMAT_NAMES};
use crate::color::ColorDatabase;
use crate::storage::{load_state, save_state, AppState, HistoryColorItem, WindowMetrics};
use color_card::ColorCard;
use gdk::prelude::*;
use gdk::EventMask;
use gdk_pixbuf::Pixbuf;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Button, CellRendererText, ComboBox, CssProvider,
    Frame, HeaderBar, Image, Label, ListStore, Orientation, Revealer, RevealerTransitionType,
    StateFlags, STYLE_PROVIDER_PRIORITY_APPLICATION,
};
use magnifier_overlay::MagnifierOverlay;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct MainWindow {
    pub window: ApplicationWindow,
    _tray_handle: Option<ksni::blocking::Handle<tray::PickTray>>,
    grab: Rc<dyn Fn(Option<String>)>,
}

struct AppInner {
    state: AppState,
    color_db: ColorDatabase,
    sound: Rc<Cell<bool>>,
    cards: Vec<ColorCard>,
    palette_editor: std::rc::Weak<studio::PaletteEditor>,
    container_vb: gtk::Box,
    history_stack: gtk::Stack,
    btn_clear: Button,
    highlight_hex: String,
    lowlight_hex: String,
    window_metrics_restored: bool,
    resize_timeout_id: Option<glib::SourceId>,
}

impl MainWindow {
    pub fn capture(&self) {
        let grab = self.grab.clone();
        glib::idle_add_local_once(move || grab(None));
    }

    pub fn new(app: &Application, snark_mode: bool, force_background: bool) -> Self {
        let window = ApplicationWindow::new(app);
        window.set_title(crate::APP_NAME);
        window.set_size_request(400, 440);
        window.set_default_size(480, 570);

        // HeaderBar
        let header = HeaderBar::new();
        header.set_show_close_button(true);
        header.set_title(Some(crate::APP_NAME));
        window.set_titlebar(Some(&header));

        let btn_grab = Button::new();
        let icon = Image::from_icon_name(Some("color-picker"), gtk::IconSize::Button);
        btn_grab.add(&icon);
        btn_grab.set_tooltip_text(Some("Capture color"));
        header.pack_start(&btn_grab);
        let btn_settings = Button::from_icon_name(Some("preferences-system-symbolic"), gtk::IconSize::Button);
        btn_settings.set_tooltip_text(Some("Settings"));
        header.pack_end(&btn_settings);

        // Colors setup from theme
        let style_ctx = window.style_context();
        let fg_col = style_ctx.color(StateFlags::NORMAL);
        let highlight_hex = format!(
            "#{:02x}{:02x}{:02x}",
            (fg_col.red() * 255.0) as u8,
            (fg_col.green() * 255.0) as u8,
            (fg_col.blue() * 255.0) as u8
        );
        let lowlight_hex = "#888888".to_string();

        let avg_highlight = (fg_col.red() + fg_col.green() + fg_col.blue()) / 3.0;
        let row_bg_col = if avg_highlight > 0.5 {
            "rgba(0, 0, 0, 0.6)"
        } else {
            "rgba(255, 255, 255, 0.6)"
        };

        // CSS
        let css = format!(
            r#"
            label {{ transition: 250ms ease-in-out; }}
            label.highlighted {{ background-color: rgba(255, 255, 0, 0.4); }}
            #empty-heading {{ font-size: 200%; }}
            frame {{
                background-color: {row_bg_col};
            }}
            eventbox frame {{
                border-width: 0 0 1px 0;
                padding: 6px 0;
            }}
            "#
        );
        let css_provider = CssProvider::new();
        let _ = css_provider.load_from_data(css.as_bytes());
        gtk::StyleContext::add_provider_for_screen(
            &gdk::Screen::default().expect("Default screen"),
            &css_provider,
            STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        // Main Box
        let main_vb = gtk::Box::new(Orientation::Vertical, 0);

        // Format Selector
        let hb_format = gtk::Box::new(Orientation::Horizontal, 12);
        let lbl_format = Label::new(Some("Format:"));
        hb_format.pack_start(&lbl_format, false, false, 12);

        let format_store = ListStore::new(&[glib::Type::STRING]);
        for &fmt in FORMAT_NAMES {
            format_store.set(&format_store.append(), &[(0, &fmt)]);
        }
        let format_combo = ComboBox::with_model(&format_store);
        let renderer = CellRendererText::new();
        format_combo.pack_start(&renderer, true);
        format_combo.add_attribute(&renderer, "text", 0);
        hb_format.pack_start(&format_combo, false, false, 12);
        main_vb.pack_start(&hb_format, false, false, 12);

        // History container
        let hb_history = gtk::Box::new(Orientation::Horizontal, 0);
        let history_frame = Frame::new(None);
        let container_vb = gtk::Box::new(Orientation::Vertical, 0);
        let history_scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        history_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        history_scroll.set_min_content_height(130);
        history_scroll.add(&container_vb);
        history_frame.add(&history_scroll);
        hb_history.pack_start(&history_frame, true, true, 12);
        main_vb.pack_start(&hb_history, true, true, 0);

        // Bottom bar: Clear history
        let hb_bottom = gtk::Box::new(Orientation::Horizontal, 0);
        let btn_clear = Button::with_label("Clear history");
        btn_clear.set_sensitive(false);
        hb_bottom.pack_end(&btn_clear, false, false, 12);
        main_vb.pack_end(&hb_bottom, false, false, 12);

        // Empty state
        let empty_box = empty_state::build_empty_state();
        let history_stack = gtk::Stack::new();
        history_stack.add_named(&empty_box, "empty");
        history_stack.add_named(&main_vb, "history");
        let tabs = gtk::Notebook::new();
        tabs.set_scrollable(true);
        tabs.append_page(&history_stack, Some(&Label::new(Some("History"))));
        window.add(&tabs);

        let initial_state = load_state();

        let inner = Rc::new(RefCell::new(AppInner {
            sound: Rc::new(Cell::new(initial_state.settings.sound)),
            state: initial_state,
            color_db: ColorDatabase::new(snark_mode),
            cards: Vec::new(),
            palette_editor: std::rc::Weak::new(),
            container_vb: container_vb.clone(),
            history_stack,
            btn_clear: btn_clear.clone(),
            highlight_hex,
            lowlight_hex,
            window_metrics_restored: false,
            resize_timeout_id: None,
        }));

        {
            let inner = inner.clone();
            let window = window.downgrade();
            btn_settings.connect_clicked(move |_| {
                if let Some(window) = window.upgrade() {
                    settings::show(&window, &inner);
                }
            });
        }

        let palette_editor = studio::add_pages(&tabs, &window, &inner);
        inner.borrow_mut().palette_editor = Rc::downgrade(&palette_editor);

        // Set active formatter from state
        {
            let inner_borrow = inner.borrow();
            let active_idx = FORMAT_NAMES
                .iter()
                .position(|&f| f == inner_borrow.state.formatter)
                .unwrap_or(2); // default CSS rgb
            format_combo.set_active(Some(active_idx as u32));
        }

        // Format combo changed
        {
            let inner_clone = inner.clone();
            format_combo.connect_changed(move |cb| {
                if let Some(iter) = cb.active_iter() {
                    let model = cb.model().expect("ComboBox model");
                    let val = model.value(&iter, 0);
                    if let Ok(fmt_str) = val.get::<&str>() {
                        let mut inner_borrow = inner_clone.borrow_mut();
                        inner_borrow.state.formatter = fmt_str.to_string();
                        let active_fmt = inner_borrow.state.formatter.clone();
                        let hi = inner_borrow.highlight_hex.clone();
                        let lo = inner_borrow.lowlight_hex.clone();

                        for card in &inner_borrow.cards {
                            let formatted = format_color(&active_fmt, card.r, card.g, card.b, &card.name);
                            card.update_formatted(&formatted, &hi, &lo);
                        }

                        persist(&inner_borrow.state);
                    }
                }
            });
        }

        // Clear history button clicked
        {
            let inner_clone = inner.clone();
            btn_clear.connect_clicked(move |_| {
                let mut inner_borrow = inner_clone.borrow_mut();
                inner_borrow.state.colours.clear();
                inner_borrow.cards.clear();

                for child in inner_borrow.container_vb.children() {
                    inner_borrow.container_vb.remove(&child);
                }

                inner_borrow.history_stack.set_visible_child_name("empty");
                inner_borrow.btn_clear.set_sensitive(false);
                persist(&inner_borrow.state);
            });
        }

        // Window events
        window.add_events(EventMask::KEY_PRESS_MASK | EventMask::ENTER_NOTIFY_MASK);

        // Ensure default pointer cursor is immediately restored when mouse enters the app window
        window.connect_enter_notify_event(|w, _| {
            if let Some(gw) = w.window() {
                if let Some(display) = gdk::Display::default() {
                    let arrow = gdk::Cursor::from_name(&display, "default");
                    gw.set_cursor(arrow.as_ref());
                } else {
                    gw.set_cursor(None);
                }
            }
            glib::Propagation::Proceed
        });

        // Pick state tracking
        let is_picking = Rc::new(RefCell::new(false));

        // Pick / Magnifier Action (Per-monitor aligned overlay)
        let grab_fn = {
            let inner_clone = inner.clone();
            let win_weak = window.downgrade();
            let is_picking_clone = is_picking.clone();
            let btn_grab_weak = btn_grab.downgrade();
            Rc::new(move |token: Option<String>| {
                if *is_picking_clone.borrow() {
                    return;
                }
                *is_picking_clone.borrow_mut() = true;
                if let Some(btn) = btn_grab_weak.upgrade() {
                    btn.set_sensitive(false);
                }

                let (screen, monitor_idx, geom) = {
                    let scr = gdk::Screen::default().expect("Default screen");
                    let display = scr.display();
                    let seat = display.default_seat();
                    let pointer = seat.and_then(|s| s.pointer());
                    let mon = if let Some(ptr) = pointer {
                        let (_, px, py) = ptr.position();
                        display.monitor_at_point(px, py).unwrap_or_else(|| display.monitor(0).unwrap())
                    } else if let Some(win) = win_weak.upgrade() {
                        if let Some(gw) = win.window() {
                            display.monitor_at_window(&gw).unwrap_or_else(|| display.monitor(0).unwrap())
                        } else {
                            display.monitor(0).unwrap()
                        }
                    } else {
                        display.monitor(0).unwrap()
                    };

                    let mut idx = 0;
                    for i in 0..display.n_monitors() {
                        if let Some(m) = display.monitor(i) {
                            if m == mon {
                                idx = i;
                                break;
                            }
                        }
                    }
                    (scr, idx, mon.geometry())
                };

                if let Some(win) = win_weak.upgrade() {
                    win.hide();
                    screen.display().sync();
                }

                let screenshot_opt = capture::capture_fullscreen();

                let is_picking_end = is_picking_clone.clone();
                let btn_grab_end = btn_grab_weak.clone();
                let win_end = win_weak.clone();
                let reset_picker = move || {
                    *is_picking_end.borrow_mut() = false;
                    if let Some(btn) = btn_grab_end.upgrade() {
                        btn.set_sensitive(true);
                    }
                    if let Some(win) = win_end.upgrade() {
                        win.present();
                        if let Some(gw) = win.window() {
                            if let Some(display) = gdk::Display::default() {
                                let arrow = gdk::Cursor::from_name(&display, "default");
                                gw.set_cursor(arrow.as_ref());
                            } else {
                                gw.set_cursor(None);
                            }
                        }
                    }
                };

                if let Some(full_screenshot) = screenshot_opt {
                    let inner_c = inner_clone.clone();
                    let fmt = inner_clone.borrow().state.formatter.clone();

                    // Crop to the active monitor geometry (resolves multi-monitor offset and "below the screen" issues)
                    let crop_x = geom.x().clamp(0, full_screenshot.width() - 1);
                    let crop_y = geom.y().clamp(0, full_screenshot.height() - 1);
                    let crop_w = geom.width().min(full_screenshot.width() - crop_x);
                    let crop_h = geom.height().min(full_screenshot.height() - crop_y);

                    let monitor_screenshot = full_screenshot.new_subpixbuf(crop_x, crop_y, crop_w, crop_h);

                    let reset_picker_1 = reset_picker.clone();
                    let reset_picker_2 = reset_picker;

                    MagnifierOverlay::start(
                        monitor_screenshot,
                        &screen,
                        monitor_idx,
                        fmt,
                        snark_mode,
                        token,
                        move |r, g, b, crop| {
                            reset_picker_1();
                            Self::finish_capture(&inner_c, (r, g, b), Some(crop));
                        },
                        move || {
                            reset_picker_2();
                        },
                    );
                } else {
                    // Fallback to portal if screenshot tool is missing
                    let inner_c = inner_clone.clone();
                    glib::MainContext::default().spawn_local(async move {
                        let color = capture::pick_color_portal().await;
                        reset_picker();
                        if let Some((r, g, b)) = color {
                            let swatch = capture::create_color_swatch_pixbuf(r, g, b, 60, 60);
                            Self::finish_capture(&inner_c, (r, g, b), swatch);
                        }
                    });
                }
            })
        };

        // Connect Grab button
        {
            let grab_fn_clone = grab_fn.clone();
            btn_grab.connect_clicked(move |_| {
                grab_fn_clone(None);
            });
        }

        // Configure event (Debounced window metrics save)
        {
            let inner_clone = inner.clone();
            window.connect_configure_event(move |win, ev| {
                let mut inner_borrow = inner_clone.borrow_mut();
                if !inner_borrow.window_metrics_restored {
                    return false;
                }

                if let Some(source_id) = inner_borrow.resize_timeout_id.take() {
                    source_id.remove();
                }

                let (x, y) = ev.position();
                let (w, h) = ev.size();
                let inner_weak = Rc::downgrade(&inner_clone);
                let win_weak = win.downgrade();

                let source_id = glib::timeout_add_seconds_local(1, move || {
                    if let (Some(inner_rc), Some(wnd)) = (inner_weak.upgrade(), win_weak.upgrade()) {
                        if let Some(screen) = gtk::prelude::GtkWindowExt::screen(&wnd) {
                            if let Some(root) = screen.root_window() {
                                let sw = root.width() as f64;
                                let sh = root.height() as f64;
                                let mut inner_b = inner_rc.borrow_mut();
                                inner_b.state.metrics = Some(WindowMetrics {
                                    ww: Some(w as f64 / sw),
                                    wh: Some(h as f64 / sh),
                                    wx: Some(x as f64 / sw),
                                    wy: Some(y as f64 / sh),
                                });
                                persist(&inner_b.state);
                                inner_b.resize_timeout_id = None;
                            }
                        }
                    }
                    glib::ControlFlow::Break
                });
                inner_borrow.resize_timeout_id = Some(source_id);

                false
            });
        }

        // Load existing history
        {
            let mut inner_borrow = inner.borrow_mut();
            let history_items = inner_borrow.state.colours.clone();

            inner_borrow.history_stack.show_all();
            if history_items.is_empty() {
                inner_borrow.history_stack.set_visible_child_name("empty");
            } else {
                inner_borrow.history_stack.set_visible_child_name("history");
                for item in history_items {
                    let pb = capture::base64_png_to_pixbuf(&item.imgdata);
                    Self::add_color_item_view(
                        &mut inner_borrow,
                        item.colour[0],
                        item.colour[1],
                        item.colour[2],
                        pb,
                    );
                }
                inner_borrow.btn_clear.set_sensitive(true);
            }

            // Restore metrics
            if let Some(metrics) = &inner_borrow.state.metrics {
                if let (Some(wx_val), Some(wy_val), Some(ww_val), Some(wh_val)) = (metrics.wx, metrics.wy, metrics.ww, metrics.wh) {
                    if let Some(screen) = gtk::prelude::GtkWindowExt::screen(&window) {
                        if let Some(root) = screen.root_window() {
                            let sw = root.width() as f64;
                            let sh = root.height() as f64;
                            let wx = (sw * wx_val) as i32;
                            let wy = (sh * wy_val) as i32;
                            let ww = (sw * ww_val) as i32;
                            let wh = (sh * wh_val) as i32;

                            let win_weak = window.downgrade();
                            glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                                if let Some(win) = win_weak.upgrade() {
                                    win.move_(wx, wy);
                                    win.resize(ww, wh);
                                }
                                glib::ControlFlow::Break
                            });
                        }
                    }
                }
            }
            inner_borrow.window_metrics_restored = true;
        }

        #[allow(deprecated)]
        let (tray_tx, tray_rx) = glib::MainContext::channel::<tray::TrayCommand>(glib::Priority::default());
        let tray_handle = tray::spawn_tray(tray_tx);

        {
            let inner = inner.clone();
            let can_hide = tray_handle.is_some();
            let app = app.downgrade();
            let hold = RefCell::new(None);
            window.connect_delete_event(move |win, _| {
                if can_hide && (force_background || inner.borrow().state.settings.background) {
                    if let Some(app) = app.upgrade() {
                        hold.borrow_mut().get_or_insert_with(|| app.hold());
                    }
                    win.hide();
                    glib::Propagation::Stop
                } else {
                    hold.borrow_mut().take();
                    glib::Propagation::Proceed
                }
            });
        }

        {
            let grab_fn_tray = grab_fn.clone();
            let win_weak_tray = window.downgrade();
            let app_weak_tray = app.downgrade();
            tray_rx.attach(None, move |cmd| {
                match cmd {
                    tray::TrayCommand::PickColor(token) => {
                        grab_fn_tray(token);
                    }
                    tray::TrayCommand::ShowWindow(token) => {
                        if let Some(win) = win_weak_tray.upgrade() {
                            if let Some(tok) = token.as_deref() {
                                win.set_startup_id(tok);
                            }
                            win.show_all();
                            win.present();
                        }
                    }
                    tray::TrayCommand::ShowAbout => {
                        if let Some(win) = win_weak_tray.upgrade() {
                            about::show_about_dialog(Some(win.upcast_ref::<gtk::Window>()));
                        }
                    }
                    tray::TrayCommand::Quit => {
                        if let Some(app) = app_weak_tray.upgrade() {
                            app.quit();
                        } else {
                            std::process::exit(0);
                        }
                    }
                }
                glib::ControlFlow::Continue
            });
        }

        window.show_all();

        MainWindow {
            window,
            _tray_handle: tray_handle,
            grab: grab_fn,
        }
    }

    fn finish_capture(
        inner: &RefCell<AppInner>,
        (r, g, b): (u8, u8, u8),
        thumbnail: Option<Pixbuf>,
    ) {
        let mut inner = inner.borrow_mut();
        let image = thumbnail.as_ref().and_then(capture::pixbuf_to_base64_png).unwrap_or_default();
        Self::add_color_item(&mut inner, r, g, b, &image, thumbnail);
        persist(&inner.state);

        if inner.state.settings.auto_copy {
            let name = inner.color_db.closest_name(r, g, b);
            let formatted = format_color(&inner.state.formatter, r, g, b, name);
            clipboard::copy_to_clipboard(&formatted);
            if let Some(card) = inner.cards.last() {
                card.label.style_context().add_class("highlighted");
                let label = card.label.downgrade();
                glib::timeout_add_local_once(std::time::Duration::from_millis(300), move || {
                    if let Some(label) = label.upgrade() {
                        label.style_context().remove_class("highlighted");
                    }
                });
            }
        }
        if inner.sound.get() {
            crate::audio::play_sound("camera-shutter");
        }
    }

    fn add_color_item_view(
        inner: &mut AppInner,
        r: u8,
        g: u8,
        b: u8,
        pixbuf: Option<Pixbuf>,
    ) {
        let col_name = inner.color_db.closest_name(r, g, b);
        let formatted = format_color(&inner.state.formatter, r, g, b, col_name);
        let palette_editor = inner.palette_editor.clone();
        let swatch = crate::color::studio::Swatch { name: col_name.to_owned(), rgb: [r, g, b] };
        let card = ColorCard::new(
            r,
            g,
            b,
            col_name,
            &formatted,
            pixbuf,
            &inner.highlight_hex,
            &inner.lowlight_hex,
            inner.sound.clone(),
            move || {
                if let Some(editor) = palette_editor.upgrade() {
                    editor.add_history_color(swatch.clone());
                }
            },
        );

        let revealer = Revealer::new();
        revealer.set_transition_type(RevealerTransitionType::SlideDown);
        revealer.add(&card.container);

        inner.container_vb.pack_start(&revealer, false, false, 0);
        inner.container_vb.reorder_child(&revealer, 0);
        revealer.show_all();
        revealer.set_reveal_child(true);

        inner.cards.push(card);
    }

    fn add_color_item(
        inner: &mut AppInner,
        r: u8,
        g: u8,
        b: u8,
        imgdata: &str,
        pixbuf: Option<Pixbuf>,
    ) {
        // Defensive check: prevent consecutive duplicate additions
        if let Some(last) = inner.state.colours.last() {
            if last.colour == [r, g, b] && last.imgdata == imgdata {
                return;
            }
        }

        inner.state.colours.push(HistoryColorItem {
            imgdata: imgdata.to_string(),
            colour: [r, g, b],
        });

        Self::add_color_item_view(inner, r, g, b, pixbuf);
        inner.history_stack.set_visible_child_name("history");
        inner.btn_clear.set_sensitive(true);
    }
}

fn persist(state: &AppState) {
    if let Err(error) = save_state(state) {
        eprintln!("Could not save Luti Color Picker data: {error}");
    }
}
