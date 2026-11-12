pub mod about;
pub mod clipboard;
pub mod color_card;
pub mod empty_state;
pub mod magnifier_overlay;
mod settings;
mod shortcuts;
mod studio;
pub mod tray;

use crate::capture;
use crate::color::formatters::{format_color, FORMAT_NAMES};
use crate::color::ColorDatabase;
use crate::storage::{load_state, save_state, AppState, HistoryColorItem, WindowMetrics};
use color_card::ColorCard;
use gdk::prelude::*;
use gdk_pixbuf::Pixbuf;
use gtk::prelude::*;
use gtk::{
    Align, Application, ApplicationWindow, Button, ComboBoxText, CssProvider, HeaderBar, Image,
    Label, Orientation, Revealer, RevealerTransitionType, STYLE_PROVIDER_PRIORITY_APPLICATION,
};
use magnifier_overlay::MagnifierOverlay;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct MainWindow {
    pub window: ApplicationWindow,
    _tray_handle: Option<ksni::blocking::Handle<tray::PickTray>>,
    grab: Rc<dyn Fn(Option<String>)>,
    _shortcut: Rc<shortcuts::GlobalShortcut>,
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
}

impl MainWindow {
    pub fn capture(&self) {
        let grab = self.grab.clone();
        glib::idle_add_local_once(move || grab(None));
    }

    pub fn new(app: &Application, snark_mode: bool, force_background: bool) -> Self {
        let window = ApplicationWindow::new(app);
        window.set_title(Some(crate::APP_NAME));
        window.add_css_class("luti");
        window.set_size_request(460, 440);
        window.set_default_size(620, 620);

        let header = HeaderBar::new();
        let title = Label::new(Some(crate::APP_NAME));
        title.add_css_class("brand-title");
        header.set_title_widget(Some(&title));
        window.set_titlebar(Some(&header));

        let btn_grab = Button::with_label("Capture color");
        btn_grab.add_css_class("capture");
        header.pack_start(&btn_grab);
        let btn_settings = Button::from_icon_name("preferences-system-symbolic");
        btn_settings.set_tooltip_text(Some("Settings"));
        header.pack_end(&btn_settings);

        let css = CssProvider::new();
        css.load_from_data(include_str!("style.css"));
        gtk::style_context_add_provider_for_display(
            &gtk::prelude::WidgetExt::display(&window),
            &css,
            STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        let highlight_hex = "#f7efff".to_string();
        let lowlight_hex = "#bfb1cd".to_string();

        let main_vb = gtk::Box::new(Orientation::Vertical, 14);
        main_vb.set_margin_top(18);
        main_vb.set_margin_bottom(18);
        main_vb.set_margin_start(18);
        main_vb.set_margin_end(18);

        let toolbar = gtk::Box::new(Orientation::Horizontal, 10);
        toolbar.append(&Label::new(Some("Color format")));
        let format_combo = ComboBoxText::new();
        for &format in FORMAT_NAMES {
            format_combo.append(Some(format), format);
        }
        toolbar.append(&format_combo);
        let btn_clear = Button::with_label("Clear history");
        btn_clear.set_sensitive(false);
        btn_clear.set_hexpand(true);
        btn_clear.set_halign(Align::End);
        toolbar.append(&btn_clear);
        main_vb.append(&toolbar);

        let container_vb = gtk::Box::new(Orientation::Vertical, 8);
        let history_scroll = gtk::ScrolledWindow::new();
        history_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        history_scroll.set_min_content_height(130);
        history_scroll.set_vexpand(true);
        history_scroll.set_child(Some(&container_vb));
        main_vb.append(&history_scroll);

        let empty_box = empty_state::build_empty_state(&btn_grab);
        let history_stack = gtk::Stack::new();
        history_stack.add_named(&empty_box, Some("empty"));
        history_stack.add_named(&main_vb, Some("history"));
        let tabs = gtk::Notebook::new();
        tabs.set_scrollable(true);
        tabs.append_page(&history_stack, Some(&Label::new(Some("History"))));
        window.set_child(Some(&tabs));

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
        }));

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
                if let Some(format) = cb.active_id() {
                    let mut inner = inner_clone.borrow_mut();
                    inner.state.formatter = format.to_string();
                    for card in &inner.cards {
                        let formatted = format_color(&format, card.r, card.g, card.b, &card.name);
                        card.update_formatted(
                            &formatted,
                            &inner.highlight_hex,
                            &inner.lowlight_hex,
                        );
                    }
                    persist(&inner.state);
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

                while let Some(child) = inner_borrow.container_vb.first_child() {
                    inner_borrow.container_vb.remove(&child);
                }

                inner_borrow.history_stack.set_visible_child_name("empty");
                inner_borrow.btn_clear.set_sensitive(false);
                persist(&inner_borrow.state);
            });
        }

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

                let monitor = win_weak.upgrade().and_then(|window| {
                    let monitor = capture_monitor(&window);
                    window.hide();
                    gtk::prelude::WidgetExt::display(&window).sync();
                    monitor
                });

                let screenshot = monitor.and_then(|monitor| {
                    let screenshot = capture::capture_fullscreen()?;
                    let screenshot = crop_monitor(&screenshot, &monitor)?;
                    Some((screenshot, monitor))
                });

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
                    }
                };

                if let Some((monitor_screenshot, monitor)) = screenshot {
                    let inner_c = inner_clone.clone();
                    let (fmt, zoom_range) = {
                        let inner = inner_clone.borrow();
                        (
                            inner.state.formatter.clone(),
                            inner.state.settings.zoom_range(),
                        )
                    };

                    let reset_picker_1 = reset_picker.clone();
                    let reset_picker_2 = reset_picker;

                    MagnifierOverlay::start(
                        monitor_screenshot,
                        &monitor,
                        fmt,
                        zoom_range,
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

        let grab_fn: Rc<dyn Fn(Option<String>)> = grab_fn;
        let shortcut = Rc::new(shortcuts::GlobalShortcut::new(Rc::downgrade(&grab_fn)));
        shortcut.set_enabled(inner.borrow().state.settings.global_shortcut);
        {
            let inner = inner.clone();
            let window = window.downgrade();
            let shortcut = shortcut.clone();
            btn_settings.connect_clicked(move |_| {
                if let Some(window) = window.upgrade() {
                    settings::show(&window, &inner, &shortcut);
                }
            });
        }

        // Connect Grab button
        {
            let grab_fn_clone = grab_fn.clone();
            btn_grab.connect_clicked(move |_| {
                grab_fn_clone(None);
            });
        }

        // Load existing history
        {
            let mut inner_borrow = inner.borrow_mut();
            let history_items = inner_borrow.state.colours.clone();

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

            if let Some(metrics) = &inner_borrow.state.metrics {
                if let (Some(width), Some(height), Some(bounds)) = (
                    metrics.ww,
                    metrics.wh,
                    display_bounds(&gtk::prelude::WidgetExt::display(&window)),
                ) {
                    window.set_default_size(
                        (f64::from(bounds.2) * width) as i32,
                        (f64::from(bounds.3) * height) as i32,
                    );
                }
            }
        }

        let (tray_tx, tray_rx) = async_channel::unbounded();
        let tray_handle = tray::spawn_tray(tray_tx);

        {
            let inner = inner.clone();
            let can_hide = tray_handle.is_some();
            let app = app.downgrade();
            let hold = RefCell::new(None);
            window.connect_close_request(move |win| {
                if let Some((_, _, width, height)) =
                    display_bounds(&gtk::prelude::WidgetExt::display(win))
                {
                    let mut inner = inner.borrow_mut();
                    let metrics = inner.state.metrics.get_or_insert(WindowMetrics {
                        ww: None,
                        wh: None,
                        wx: None,
                        wy: None,
                    });
                    metrics.ww = Some(f64::from(win.width()) / f64::from(width));
                    metrics.wh = Some(f64::from(win.height()) / f64::from(height));
                    persist(&inner.state);
                }
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
            glib::MainContext::default().spawn_local(async move {
                while let Ok(cmd) = tray_rx.recv().await {
                    match cmd {
                        tray::TrayCommand::PickColor(token) => {
                            grab_fn_tray(token);
                        }
                        tray::TrayCommand::ShowWindow(token) => {
                            if let Some(win) = win_weak_tray.upgrade() {
                                if let Some(tok) = token.as_deref() {
                                    win.set_startup_id(tok);
                                }
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
                }
            });
        }

        MainWindow {
            window,
            _tray_handle: tray_handle,
            grab: grab_fn,
            _shortcut: shortcut,
        }
    }

    fn finish_capture(
        inner: &RefCell<AppInner>,
        (r, g, b): (u8, u8, u8),
        thumbnail: Option<Pixbuf>,
    ) {
        let mut inner = inner.borrow_mut();
        let image = thumbnail
            .as_ref()
            .and_then(capture::pixbuf_to_base64_png)
            .unwrap_or_default();
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
            crate::audio::Feedback::Capture.play();
        }
    }

    fn add_color_item_view(inner: &mut AppInner, r: u8, g: u8, b: u8, pixbuf: Option<Pixbuf>) {
        let col_name = inner.color_db.closest_name(r, g, b);
        let formatted = format_color(&inner.state.formatter, r, g, b, col_name);
        let palette_editor = inner.palette_editor.clone();
        let swatch = crate::color::studio::Swatch {
            name: col_name.to_owned(),
            rgb: [r, g, b],
        };
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
        revealer.set_child(Some(&card.container));

        inner.container_vb.prepend(&revealer);
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

fn monitors(display: &gdk::Display) -> impl Iterator<Item = gdk::Monitor> {
    let model = display.monitors();
    (0..model.n_items()).filter_map(move |index| model.item(index)?.downcast().ok())
}

fn display_bounds(display: &gdk::Display) -> Option<(i32, i32, i32, i32)> {
    let geometries: Vec<_> = monitors(display)
        .map(|monitor| monitor.geometry())
        .collect();
    let left = geometries.iter().map(|rect| rect.x()).min()?;
    let top = geometries.iter().map(|rect| rect.y()).min()?;
    let right = geometries
        .iter()
        .map(|rect| rect.x() + rect.width())
        .max()?;
    let bottom = geometries
        .iter()
        .map(|rect| rect.y() + rect.height())
        .max()?;
    Some((left, top, right - left, bottom - top))
}

fn capture_monitor(window: &ApplicationWindow) -> Option<gdk::Monitor> {
    let display = gtk::prelude::WidgetExt::display(window);
    if !capture::is_wayland() {
        if let Some((x, y)) = capture::x11_pointer() {
            if let Some((left, top, _, _)) = display_bounds(&display) {
                if let Some(monitor) = monitors(&display).find(|monitor| {
                    let geometry = monitor.geometry();
                    let scale = monitor.scale_factor();
                    let x = x / scale + left;
                    let y = y / scale + top;
                    x >= geometry.x()
                        && x < geometry.x() + geometry.width()
                        && y >= geometry.y()
                        && y < geometry.y() + geometry.height()
                }) {
                    return Some(monitor);
                }
            }
        }
    }
    let pointer_surface = display
        .default_seat()
        .and_then(|seat| seat.pointer())
        .and_then(|pointer| pointer.surface_at_position().0);
    pointer_surface
        .or_else(|| window.surface())
        .and_then(|surface| display.monitor_at_surface(&surface))
        .or_else(|| monitors(&display).next())
}

fn crop_monitor(screenshot: &Pixbuf, monitor: &gdk::Monitor) -> Option<Pixbuf> {
    let (left, top, width, height) = display_bounds(&monitor.display())?;
    let geometry = monitor.geometry();
    let scale_x = f64::from(screenshot.width()) / f64::from(width);
    let scale_y = f64::from(screenshot.height()) / f64::from(height);
    let x = (f64::from(geometry.x() - left) * scale_x).round() as i32;
    let y = (f64::from(geometry.y() - top) * scale_y).round() as i32;
    let right = x + (f64::from(geometry.width()) * scale_x).round() as i32;
    let bottom = y + (f64::from(geometry.height()) * scale_y).round() as i32;
    let width = right.min(screenshot.width()) - x;
    let height = bottom.min(screenshot.height()) - y;
    (width > 0 && height > 0).then(|| screenshot.new_subpixbuf(x, y, width, height))
}

fn brand_icon(size: i32) -> Image {
    let stream = gio::MemoryInputStream::from_bytes(&glib::Bytes::from_static(include_bytes!(
        "../../packaging/io.github.eulucastiagolt.LutiColorPicker.png"
    )));
    let icon = match Pixbuf::from_stream_at_scale(&stream, size, size, true, gio::Cancellable::NONE)
    {
        Ok(pixbuf) => Image::from_paintable(Some(&gdk::Texture::for_pixbuf(&pixbuf))),
        Err(_) => Image::from_icon_name(crate::APP_ID),
    };
    icon.set_pixel_size(size);
    icon
}
