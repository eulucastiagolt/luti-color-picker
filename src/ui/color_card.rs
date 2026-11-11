use gdk_pixbuf::Pixbuf;
use gtk::prelude::*;
use gtk::{
    Align, Box as GtkBox, Button, DrawingArea, EventBox, Frame, Label, Orientation,
};
use std::cell::{Cell, RefCell};
use std::f64::consts::PI;
use std::rc::Rc;

use crate::ui::clipboard::copy_to_clipboard;

fn rounded_path(ctx: &cairo::Context, w: f64, h: f64) {
    let radius = w / 10.0;
    ctx.arc(w - radius, radius, radius, -90.0 * PI / 180.0, 0.0);
    ctx.arc(w - radius, h - radius, radius, 0.0, 90.0 * PI / 180.0);
    ctx.arc(radius, h - radius, radius, 90.0 * PI / 180.0, 180.0 * PI / 180.0);
    ctx.arc(radius, radius, radius, 180.0 * PI / 180.0, 270.0 * PI / 180.0);
    ctx.close_path();
}

pub struct ColorCard {
    pub container: EventBox,
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
    ) -> Self {
        let eb = EventBox::new();
        let frame = Frame::new(None);
        let hb = GtkBox::new(Orientation::Horizontal, 0);

        eb.add(&frame);
        frame.add(&hb);

        let current_formatted = Rc::new(RefCell::new(formatted_val.to_string()));

        // 1. Context thumbnail 60x60
        let img_area = DrawingArea::new();
        img_area.set_size_request(60, 60);
        if let Some(pb) = pixbuf {
            let pb_rc = Rc::new(pb);
            img_area.connect_draw(move |_, ctx| {
                rounded_path(ctx, 60.0, 60.0);
                ctx.set_source_pixbuf(&pb_rc, 0.0, 0.0);
                ctx.clip_preserve();
                let _ = ctx.paint();
                ctx.set_line_width(2.0);
                ctx.set_source_rgba(0.0, 0.0, 0.0, 0.1);
                let _ = ctx.stroke();
                glib::Propagation::Proceed
            });
        }
        hb.pack_start(&img_area, false, false, 6);

        // 2. Pure color swatch 60x60
        let color_area = DrawingArea::new();
        color_area.set_size_request(60, 60);
        color_area.connect_draw(move |_, ctx| {
            rounded_path(ctx, 60.0, 60.0);
            ctx.set_source_rgb(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0);
            ctx.clip_preserve();
            let _ = ctx.fill_preserve();
            ctx.set_line_width(2.0);
            ctx.set_source_rgba(0.0, 0.0, 0.0, 0.1);
            let _ = ctx.stroke();
            glib::Propagation::Proceed
        });
        hb.pack_start(&color_area, false, false, 6);

        // 3. Label
        let label = Label::new(None);
        label.set_halign(Align::Start);
        Self::update_label_markup(&label, name, formatted_val, highlight_hex, lowlight_hex);
        hb.pack_start(&label, true, true, 6);

        // 4. Copy button
        let copy_btn = Button::from_icon_name(Some("edit-copy-symbolic"), gtk::IconSize::Button);
        copy_btn.set_label("Copy");
        copy_btn.set_opacity(0.0);
        hb.pack_start(&copy_btn, false, false, 6);

        // Hover / Focus effects
        let copy_btn_clone1 = copy_btn.clone();
        let copy_btn_clone2 = copy_btn.clone();
        eb.connect_enter_notify_event(move |_, _| {
            copy_btn_clone1.set_opacity(1.0);
            glib::Propagation::Proceed
        });
        eb.connect_leave_notify_event(move |_, _| {
            copy_btn_clone2.set_opacity(0.0);
            glib::Propagation::Proceed
        });

        eb.set_can_focus(true);
        let copy_btn_clone3 = copy_btn.clone();
        let copy_btn_clone4 = copy_btn.clone();
        eb.connect_focus_in_event(move |_, _| {
            copy_btn_clone3.set_opacity(1.0);
            glib::Propagation::Proceed
        });
        eb.connect_focus_out_event(move |_, _| {
            copy_btn_clone4.set_opacity(0.0);
            glib::Propagation::Proceed
        });

        // Click to copy action
        let copy_action = {
            let label = label.clone();
            let cur_fmt = current_formatted.clone();
            Rc::new(move || {
                let text = cur_fmt.borrow().clone();
                copy_to_clipboard(&text);
                label.style_context().add_class("highlighted");
                let label_weak = label.downgrade();
                glib::timeout_add_local(std::time::Duration::from_millis(300), move || {
                    if let Some(lbl) = label_weak.upgrade() {
                        lbl.style_context().remove_class("highlighted");
                    }
                    glib::ControlFlow::Break
                });
                if sound.get() {
                    crate::audio::play_sound("dialog-information");
                }
            })
        };

        {
            let copy_action_clone = copy_action.clone();
            copy_btn.connect_clicked(move |_| {
                copy_action_clone();
            });
        }

        {
            let copy_action_clone = copy_action;
            eb.connect_button_press_event(move |_, ev| {
                if ev.button() == 1 {
                    copy_action_clone();
                    return glib::Propagation::Stop;
                }
                glib::Propagation::Proceed
            });
        }

        eb.set_tooltip_text(Some("Copy to clipboard"));

        Self {
            container: eb,
            label,
            r,
            g,
            b,
            name: name.to_string(),
            current_formatted,
        }
    }

    pub fn update_formatted(
        &self,
        formatted_val: &str,
        highlight_hex: &str,
        lowlight_hex: &str,
    ) {
        *self.current_formatted.borrow_mut() = formatted_val.to_string();
        Self::update_label_markup(&self.label, &self.name, formatted_val, highlight_hex, lowlight_hex);
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
