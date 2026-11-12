use gdk::prelude::*;
use gdk_pixbuf::Pixbuf;
use gtk::prelude::*;
use gtk::{
    Align, Box as GtkBox, Button, Dialog, DialogFlags, DrawingArea, GestureClick, Label,
    Orientation, PolicyType, ResponseType, ScrolledWindow, Window,
};
use std::cell::{Cell, RefCell};
use std::f64::consts::PI;
use std::rc::Rc;

use crate::color::studio::{hex, Rgb};
use crate::ui::clipboard::copy_to_clipboard;

pub(super) fn shades_button(
    base: Rgb,
    sound: Rc<Cell<bool>>,
    add_to_palette: impl Fn(Rgb) + 'static,
) -> Button {
    let button = Button::with_label("Shades");
    let add_to_palette = Rc::new(add_to_palette);
    button.connect_clicked(move |button| {
        let Some(parent) = button.root().and_downcast::<Window>() else {
            return;
        };
        let title = format!("Shades of {}", hex(base));
        let dialog = Dialog::with_buttons(
            Some(&title),
            Some(&parent),
            DialogFlags::MODAL | DialogFlags::DESTROY_WITH_PARENT,
            &[("Close", ResponseType::Close)],
        );
        dialog.add_css_class("luti");
        dialog.set_titlebar(Some(&super::window_header(&title)));

        let list = GtkBox::new(Orientation::Vertical, 8);
        list.set_margin_top(12);
        list.set_margin_bottom(12);
        list.set_margin_start(12);
        list.set_margin_end(12);

        let scroll = ScrolledWindow::new();
        scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
        scroll.set_min_content_height(180);
        scroll.set_max_content_height(450);
        scroll.set_propagate_natural_height(true);
        scroll.set_vexpand(true);
        scroll.set_child(Some(&list));
        dialog.content_area().append(&scroll);
        dialog.set_default_size(440, -1);
        populate_shades(&list, base, &sound, &dialog, &add_to_palette);
        dialog.connect_response(|dialog, _| dialog.close());
        dialog.present();
    });
    button
}

fn populate_shades(
    list: &GtkBox,
    base: Rgb,
    sound: &Rc<Cell<bool>>,
    dialog: &Dialog,
    add_to_palette: &Rc<impl Fn(Rgb) + 'static>,
) {
    for (target, amount, description) in [
        (255.0, 0.45, "Lightest"),
        (255.0, 0.22, "Lighter"),
        (255.0, 0.0, "Original"),
        (0.0, 0.22, "Darker"),
        (0.0, 0.45, "Darkest"),
    ] {
        let rgb = base.map(|channel| {
            let channel = f64::from(channel);
            (channel + (target - channel) * amount).round() as u8
        });
        let code = hex(rgb);
        let row = GtkBox::new(Orientation::Horizontal, 10);
        row.add_css_class("color-card");
        row.append(&color_sample(rgb));

        let label = Label::new(Some(&format!("{description}\n{code}")));
        label.set_hexpand(true);
        label.set_xalign(0.0);
        row.append(&label);

        let copy = Button::with_label("Copy");
        copy.set_valign(Align::Center);
        copy.set_tooltip_text(Some(&format!("Copy {code}")));
        let sound = sound.clone();
        copy.connect_clicked(move |_| {
            copy_to_clipboard(&code);
            if sound.get() {
                crate::audio::Feedback::Copy.play();
            }
        });
        row.append(&copy);

        let add = Button::with_label("Add to palette");
        add.set_valign(Align::Center);
        add.set_tooltip_text(Some(&format!("Add {} to palette", hex(rgb))));
        let dialog = dialog.downgrade();
        let add_to_palette = add_to_palette.clone();
        add.connect_clicked(move |_| {
            if let Some(dialog) = dialog.upgrade() {
                dialog.close();
                add_to_palette(rgb);
            }
        });
        row.append(&add);
        list.append(&row);
    }
}

fn color_sample([r, g, b]: Rgb) -> DrawingArea {
    let area = DrawingArea::new();
    area.set_size_request(60, 60);
    area.set_draw_func(move |_, ctx, _, _| {
        rounded_path(ctx, 60.0, 60.0);
        ctx.set_source_rgb(
            f64::from(r) / 255.0,
            f64::from(g) / 255.0,
            f64::from(b) / 255.0,
        );
        let _ = ctx.fill_preserve();
        ctx.set_line_width(2.0);
        ctx.set_source_rgba(0.0, 0.0, 0.0, 0.1);
        let _ = ctx.stroke();
    });
    area
}

fn rounded_path(ctx: &cairo::Context, w: f64, h: f64) {
    let radius = w / 10.0;
    ctx.arc(w - radius, radius, radius, -90.0 * PI / 180.0, 0.0);
    ctx.arc(w - radius, h - radius, radius, 0.0, 90.0 * PI / 180.0);
    ctx.arc(
        radius,
        h - radius,
        radius,
        90.0 * PI / 180.0,
        180.0 * PI / 180.0,
    );
    ctx.arc(
        radius,
        radius,
        radius,
        180.0 * PI / 180.0,
        270.0 * PI / 180.0,
    );
    ctx.close_path();
}

pub struct ColorCard {
    pub container: GtkBox,
    pub label: Label,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub name: String,
    pub current_formatted: Rc<RefCell<String>>,
}

impl ColorCard {
    pub fn new(
        r: u8,
        g: u8,
        b: u8,
        name: &str,
        formatted_val: &str,
        pixbuf: Option<Pixbuf>,
        highlight_hex: &str,
        lowlight_hex: &str,
        sound: Rc<Cell<bool>>,
        add_shade: impl Fn(Rgb) + 'static,
        add_to_palette: impl Fn() + 'static,
    ) -> Self {
        let hb = GtkBox::new(Orientation::Horizontal, 10);
        hb.add_css_class("color-card");

        let content = GtkBox::new(Orientation::Horizontal, 10);
        content.set_hexpand(true);
        hb.append(&content);

        let current_formatted = Rc::new(RefCell::new(formatted_val.to_string()));

        // 1. Context thumbnail 60x60
        let img_area = DrawingArea::new();
        img_area.set_size_request(60, 60);
        if let Some(pb) = pixbuf {
            let pb_rc = Rc::new(pb);
            img_area.set_draw_func(move |_, ctx, _, _| {
                rounded_path(ctx, 60.0, 60.0);
                ctx.set_source_pixbuf(&pb_rc, 0.0, 0.0);
                ctx.clip_preserve();
                let _ = ctx.paint();
                ctx.set_line_width(2.0);
                ctx.set_source_rgba(0.0, 0.0, 0.0, 0.1);
                let _ = ctx.stroke();
            });
            content.append(&img_area);
        }

        content.append(&color_sample([r, g, b]));

        // 3. Label
        let label = Label::new(None);
        label.set_halign(Align::Start);
        label.set_xalign(0.0);
        label.set_hexpand(true);
        label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        Self::update_label_markup(&label, name, formatted_val, highlight_hex, lowlight_hex);
        content.append(&label);

        let copy_btn = Button::with_label("Copy");
        copy_btn.set_valign(Align::Center);
        copy_btn.add_css_class("flat");
        copy_btn.set_tooltip_text(Some("Copy to clipboard"));
        hb.append(&copy_btn);

        let palette_btn = Button::with_label("Palette");
        palette_btn.set_valign(Align::Center);
        palette_btn.add_css_class("flat");
        palette_btn.set_tooltip_text(Some("Add to palette"));
        palette_btn.connect_clicked(move |_| add_to_palette());
        hb.append(&palette_btn);
        let shades = shades_button([r, g, b], sound.clone(), add_shade);
        shades.set_valign(Align::Center);
        hb.append(&shades);

        copy_btn.connect_clicked({
            let label = label.clone();
            let current_formatted = current_formatted.clone();
            move |_| {
                copy_to_clipboard(&current_formatted.borrow());
                label.add_css_class("highlighted");
                let label_weak = label.downgrade();
                glib::timeout_add_local_once(std::time::Duration::from_millis(300), move || {
                    if let Some(label) = label_weak.upgrade() {
                        label.remove_css_class("highlighted");
                    }
                });
                if sound.get() {
                    crate::audio::Feedback::Copy.play();
                }
            }
        });

        let shades_gesture = GestureClick::new();
        shades_gesture.set_button(1);
        shades_gesture.connect_released(move |_, _, _, _| shades.emit_clicked());
        content.add_controller(shades_gesture);
        content.set_tooltip_text(Some("Show shades"));

        Self {
            container: hb,
            label,
            r,
            g,
            b,
            name: name.to_string(),
            current_formatted,
        }
    }

    pub fn update_formatted(&self, formatted_val: &str, highlight_hex: &str, lowlight_hex: &str) {
        *self.current_formatted.borrow_mut() = formatted_val.to_string();
        Self::update_label_markup(
            &self.label,
            &self.name,
            formatted_val,
            highlight_hex,
            lowlight_hex,
        );
    }

    pub fn update_label_markup(
        label: &Label,
        name: &str,
        formatted_val: &str,
        highlight_hex: &str,
        lowlight_hex: &str,
    ) {
        let escaped_name = glib::markup_escape_text(name);
        let escaped_val = glib::markup_escape_text(formatted_val);
        let markup = format!(
            "<span color=\"{highlight_hex}\">{escaped_name}</span>\n<span color=\"{lowlight_hex}\">{escaped_val}</span>"
        );
        label.set_markup(&markup);
    }
}
