use super::{clipboard, color_card::shades_button, persist, window_header, AppInner};
use crate::color::studio::{
    export_palette, extract_colors, fractions, hex, render_png, GradientStop, Palette,
    PaletteFormat, Rgb, Swatch,
};
use gtk::prelude::*;
use gtk::{
    Align, ApplicationWindow, Box as GtkBox, Button, ColorButton, ComboBoxText, Dialog,
    DialogFlags, DrawingArea, Entry, FileChooserAction, FileChooserNative, FileFilter, Label,
    Orientation, PolicyType, ResponseType, ScrolledWindow, SpinButton,
};
use std::cell::RefCell;
use std::rc::Rc;

type State = Rc<RefCell<AppInner>>;

pub(super) fn add_pages(
    tabs: &gtk::Notebook,
    window: &ApplicationWindow,
    inner: &State,
) -> Rc<PaletteEditor> {
    let (palette, editor) = palette_page(window, inner);
    tabs.append_page(&palette, Some(&Label::new(Some("Palettes"))));
    tabs.append_page(
        &gradient_page(window, inner),
        Some(&Label::new(Some("Gradients"))),
    );
    let extract = extract_page(
        window,
        {
            let editor = editor.clone();
            move |name, colors| editor.add_palette(Palette { name, colors })
        },
        tabs,
    );
    tabs.append_page(&extract, Some(&Label::new(Some("Image colors"))));
    editor
}

fn page() -> GtkBox {
    let page = GtkBox::new(Orientation::Vertical, 10);
    page.set_margin_top(12);
    page.set_margin_bottom(12);
    page.set_margin_start(12);
    page.set_margin_end(12);
    page
}

fn scrolled(child: &impl IsA<gtk::Widget>) -> ScrolledWindow {
    let scroll = ScrolledWindow::new();
    scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
    scroll.set_min_content_height(120);
    scroll.set_child(Some(child));
    scroll
}

fn label(text: &str) -> Label {
    let label = Label::new(Some(text));
    label.set_xalign(0.0);
    label.set_wrap(true);
    label
}

fn swatch(rgb: Rgb) -> DrawingArea {
    let sample = DrawingArea::new();
    sample.set_size_request(48, 34);
    sample.set_draw_func(move |_, cr, _, _| {
        let [r, g, b] = fractions(rgb);
        cr.set_source_rgb(r, g, b);
        let _ = cr.paint();
    });
    sample
}

fn color_button(rgb: Rgb) -> ColorButton {
    ColorButton::with_rgba(&rgba(rgb))
}

fn rgba(rgb: Rgb) -> gdk::RGBA {
    let [r, g, b] = fractions(rgb);
    gdk::RGBA::new(r as f32, g as f32, b as f32, 1.0)
}

fn parse_hex_color(text: &str) -> Option<Rgb> {
    let text = text.trim();
    let digits = text.strip_prefix('#').unwrap_or(text);
    if digits.len() != 6 || !digits.bytes().all(|digit| digit.is_ascii_hexdigit()) {
        return None;
    }
    let [_, r, g, b] = u32::from_str_radix(digits, 16).ok()?.to_be_bytes();
    Some([r, g, b])
}

fn selected_rgb(button: &ColorButton) -> Rgb {
    let color = button.rgba();
    [color.red(), color.green(), color.blue()].map(|value| (value * 255.0).round() as u8)
}

fn dialog(parent: &ApplicationWindow, title: &str, accept: &str) -> Dialog {
    let dialog = Dialog::with_buttons(
        Some(title),
        Some(parent),
        DialogFlags::MODAL | DialogFlags::DESTROY_WITH_PARENT,
        &[
            ("Cancel", ResponseType::Cancel),
            (accept, ResponseType::Accept),
        ],
    );
    dialog.add_css_class("luti");
    dialog.set_titlebar(Some(&window_header(title)));
    dialog.set_default_response(ResponseType::Accept);
    dialog.set_default_size(320, -1);
    dialog
}

fn edit_name(window: &ApplicationWindow, title: &str, initial: &str) -> Option<String> {
    let dialog = dialog(window, title, "Save");
    let content = page();
    let name = Entry::new();
    name.set_text(initial);
    name.set_placeholder_text(Some("Palette name"));
    name.set_activates_default(true);
    content.append(&name);
    dialog.content_area().append(&content);
    let response = glib::MainContext::default().block_on(dialog.run_future());
    let name = name.text().trim().to_owned();
    dialog.close();
    (response == ResponseType::Accept && !name.is_empty()).then_some(name)
}

fn edit_swatch(window: &ApplicationWindow, mut color: Swatch) -> Option<Swatch> {
    let dialog = dialog(window, "Palette color", "Save");
    let content = page();
    let name = Entry::new();
    name.set_text(&color.name);
    name.set_placeholder_text(Some("Color name"));
    name.set_activates_default(true);
    let picker = color_button(color.rgb);
    picker.set_halign(Align::Start);
    let code = Entry::new();
    code.set_text(&hex(color.rgb));
    code.set_placeholder_text(Some("HEX color"));
    code.set_activates_default(true);
    code.set_tooltip_text(Some("Enter a six-digit HEX color, such as #23C7B2"));
    let preview = picker.downgrade();
    let editor_dialog = dialog.downgrade();
    code.connect_changed(move |entry| {
        let rgb = parse_hex_color(&entry.text());
        if let (Some(picker), Some(rgb)) = (preview.upgrade(), rgb) {
            picker.set_rgba(&rgba(rgb));
            entry.remove_css_class("error");
        } else {
            entry.add_css_class("error");
        }
        if let Some(dialog) = editor_dialog.upgrade() {
            dialog.set_response_sensitive(ResponseType::Accept, rgb.is_some());
        }
    });
    let code_entry = code.downgrade();
    picker.connect_color_set(move |picker| {
        if let Some(code) = code_entry.upgrade() {
            code.set_text(&hex(selected_rgb(picker)));
        }
    });
    content.append(&name);
    content.append(&code);
    content.append(&picker);
    dialog.content_area().append(&content);
    let response = glib::MainContext::default().block_on(dialog.run_future());
    color.name = name.text().trim().to_owned();
    let rgb = parse_hex_color(&code.text());
    dialog.close();
    if response != ResponseType::Accept {
        return None;
    }
    color.rgb = rgb?;
    Some(color)
}

fn save_file(
    window: &ApplicationWindow,
    filename: &str,
    data: Result<Vec<u8>, String>,
    status: &Label,
) {
    let data = match data {
        Ok(data) => data,
        Err(error) => {
            status.set_text(&error);
            return;
        }
    };
    let chooser = FileChooserNative::new(
        Some("Export"),
        Some(window),
        FileChooserAction::Save,
        Some("Save"),
        Some("Cancel"),
    );
    chooser.set_current_name(filename);
    if glib::MainContext::default().block_on(chooser.run_future()) == ResponseType::Accept {
        if let Some(path) = chooser.file().and_then(|file| file.path()) {
            match glib::file_set_contents(&path, &data) {
                Ok(()) => status.set_text(&format!("Saved {}", path.display())),
                Err(error) => status.set_text(&format!("Could not save: {error}")),
            }
        } else {
            status.set_text("Choose a local file to export.");
        }
    }
}

pub(super) struct PaletteEditor {
    window: glib::WeakRef<ApplicationWindow>,
    inner: State,
    selector: glib::WeakRef<ComboBoxText>,
    list: glib::WeakRef<GtkBox>,
    controls: glib::WeakRef<GtkBox>,
    palette_actions: glib::WeakRef<GtkBox>,
}

impl PaletteEditor {
    pub(super) fn add_shade(self: &Rc<Self>, rgb: Rgb) {
        let [r, g, b] = rgb;
        let name = self.inner.borrow_mut().color_db.closest_name(r, g, b).to_owned();
        self.add_history_color(Swatch { name, rgb });
    }

    pub(super) fn add_history_color(self: &Rc<Self>, color: Swatch) {
        let choose_palette = self.inner.borrow().state.settings.choose_palette_for_history;
        if !choose_palette && self.selected().is_some() {
            self.edit_colors(|colors| colors.push(color));
            return;
        }

        let (Some(window), Some(selector)) = (self.window.upgrade(), self.selector.upgrade())
        else {
            return;
        };
        let choices = ComboBoxText::new();
        let count = {
            let inner = self.inner.borrow();
            for palette in &inner.state.palettes {
                choices.append_text(&palette.name);
            }
            inner.state.palettes.len()
        };
        choices.append_text("New palette…");
        choices.set_active(Some(self.selected().unwrap_or(0) as u32));

        let target = if count == 0 {
            Some(0)
        } else {
            let dialog = dialog(&window, "Add to palette", "Add");
            let content = page();
            let preview = GtkBox::new(Orientation::Horizontal, 8);
            preview.append(&swatch(color.rgb));
            preview.append(&label(&format!("{} · {}", color.name, hex(color.rgb))));
            content.append(&preview);
            content.append(&choices);
            dialog.content_area().append(&content);
            let response = glib::MainContext::default().block_on(dialog.run_future());
            let selected = choices.active().map(|index| index as usize);
            dialog.close();
            if response == ResponseType::Accept {
                selected
            } else {
                None
            }
        };

        match target {
            Some(index) if index < count => {
                selector.set_active(Some(index as u32));
                self.edit_colors(|colors| colors.push(color));
            }
            Some(_) => {
                if let Some(name) = edit_name(&window, "New palette", "") {
                    self.add_palette(Palette {
                        name,
                        colors: vec![color],
                    });
                }
            }
            None => {}
        }
    }

    fn selected(&self) -> Option<usize> {
        self.selector
            .upgrade()?
            .active()
            .map(|index| index as usize)
    }

    fn add_palette(&self, palette: Palette) {
        let Some(selector) = self.selector.upgrade() else {
            return;
        };
        selector.append_text(&palette.name);
        let index = {
            let mut inner = self.inner.borrow_mut();
            inner.state.palettes.push(palette);
            persist(&inner.state);
            inner.state.palettes.len() - 1
        };
        selector.set_active(Some(index as u32));
    }

    fn edit_colors(self: &Rc<Self>, update: impl FnOnce(&mut Vec<Swatch>)) {
        let Some(index) = self.selected() else { return };
        {
            let mut inner = self.inner.borrow_mut();
            update(&mut inner.state.palettes[index].colors);
            persist(&inner.state);
        }
        self.refresh();
    }

    fn refresh(self: &Rc<Self>) {
        let (Some(list), Some(controls), Some(palette_actions)) = (
            self.list.upgrade(),
            self.controls.upgrade(),
            self.palette_actions.upgrade(),
        ) else {
            return;
        };
        while let Some(child) = list.first_child() {
            list.remove(&child);
        }
        let selected = self.selected();
        controls.set_sensitive(selected.is_some());
        palette_actions.set_sensitive(selected.is_some());
        let Some(index) = selected else {
            list.append(&label("Create a palette to start adding colors."));
            return;
        };
        let colors = self.inner.borrow().state.palettes[index].colors.clone();
        if colors.is_empty() {
            list.append(&label("Add a color or use the latest screen capture."));
        }
        for (index, color) in colors.into_iter().enumerate() {
            let row = GtkBox::new(Orientation::Horizontal, 8);
            row.append(&swatch(color.rgb));
            let description = if color.name.is_empty() {
                hex(color.rgb)
            } else {
                format!("{}\n{}", color.name, hex(color.rgb))
            };
            let description = label(&description);
            description.set_hexpand(true);
            row.append(&description);
            let copy = Button::with_label("Copy");
            let code = hex(color.rgb);
            let sound = self.inner.borrow().sound.clone();
            copy.connect_clicked(move |_| {
                clipboard::copy_to_clipboard(&code);
                if sound.get() {
                    crate::audio::Feedback::Copy.play();
                }
            });
            row.append(&copy);
            let editor = Rc::downgrade(self);
            let shades = shades_button(color.rgb, self.inner.borrow().sound.clone(), move |rgb| {
                if let Some(editor) = editor.upgrade() {
                    editor.add_shade(rgb);
                }
            });
            row.append(&shades);
            let edit = Button::with_label("Edit");
            row.append(&edit);
            let remove = Button::with_label("Remove");
            row.append(&remove);
            let editor = self.clone();
            edit.connect_clicked(move |_| {
                if let Some(window) = editor.window.upgrade() {
                    if let Some(updated) = edit_swatch(&window, color.clone()) {
                        editor.edit_colors(|colors| colors[index] = updated);
                    }
                }
            });
            let editor = self.clone();
            remove.connect_clicked(move |_| {
                editor.edit_colors(|colors| {
                    colors.remove(index);
                })
            });
            list.append(&row);
        }
    }
}

fn palette_page(window: &ApplicationWindow, inner: &State) -> (GtkBox, Rc<PaletteEditor>) {
    let root = page();
    let top = GtkBox::new(Orientation::Horizontal, 6);
    let selector = ComboBoxText::new();
    for palette in &inner.borrow().state.palettes {
        selector.append_text(&palette.name);
    }
    selector.set_hexpand(true);
    top.append(&selector);
    let new_palette = Button::with_label("New");
    top.append(&new_palette);
    let palette_actions = GtkBox::new(Orientation::Horizontal, 6);
    let rename = Button::with_label("Rename");
    let delete = Button::with_label("Delete");
    palette_actions.append(&rename);
    palette_actions.append(&delete);
    top.append(&palette_actions);
    root.append(&top);

    let list = GtkBox::new(Orientation::Vertical, 6);
    let list_scroll = scrolled(&list);
    list_scroll.set_vexpand(true);
    root.append(&list_scroll);

    let controls = GtkBox::new(Orientation::Vertical, 6);
    let add_row = GtkBox::new(Orientation::Horizontal, 6);
    let add = Button::with_label("Add color");
    let latest = Button::with_label("Add latest capture");
    add_row.append(&add);
    add_row.append(&latest);
    controls.append(&add_row);
    let export_row = GtkBox::new(Orientation::Horizontal, 6);
    let formats = ComboBoxText::new();
    for format in PaletteFormat::ALL {
        formats.append_text(format.label());
    }
    formats.set_active(Some(0));
    let export = Button::with_label("Export palette");
    formats.set_hexpand(true);
    export_row.append(&formats);
    export_row.append(&export);
    controls.append(&export_row);
    root.append(&controls);
    let status = label("");
    root.append(&status);

    let editor = Rc::new(PaletteEditor {
        window: window.downgrade(),
        inner: inner.clone(),
        selector: selector.downgrade(),
        list: list.downgrade(),
        controls: controls.downgrade(),
        palette_actions: palette_actions.downgrade(),
    });
    selector.connect_changed({
        let editor = editor.clone();
        let status = status.clone();
        move |_| {
            status.set_text("");
            editor.refresh();
        }
    });
    new_palette.connect_clicked({
        let editor = editor.clone();
        move |_| {
            if let Some(window) = editor.window.upgrade() {
                if let Some(name) = edit_name(&window, "New palette", "") {
                    editor.add_palette(Palette {
                        name,
                        colors: Vec::new(),
                    });
                }
            }
        }
    });
    rename.connect_clicked({
        let editor = editor.clone();
        move |_| {
            let (Some(index), Some(window), Some(selector)) = (
                editor.selected(),
                editor.window.upgrade(),
                editor.selector.upgrade(),
            ) else {
                return;
            };
            let current = editor.inner.borrow().state.palettes[index].name.clone();
            if let Some(name) = edit_name(&window, "Rename palette", &current) {
                {
                    let mut inner = editor.inner.borrow_mut();
                    inner.state.palettes[index].name = name.clone();
                    persist(&inner.state);
                }
                selector.remove(index as i32);
                selector.insert_text(index as i32, &name);
                selector.set_active(Some(index as u32));
            }
        }
    });
    delete.connect_clicked({
        let editor = editor.clone();
        move |_| {
            let (Some(index), Some(window), Some(selector)) = (
                editor.selected(),
                editor.window.upgrade(),
                editor.selector.upgrade(),
            ) else {
                return;
            };
            let confirm = dialog(&window, "Delete palette", "Delete");
            let content = page();
            content.append(&label("Delete this palette and all its colors?"));
            confirm.content_area().append(&content);
            let response = glib::MainContext::default().block_on(confirm.run_future());
            confirm.close();
            if response != ResponseType::Accept {
                return;
            }
            let count = {
                let mut inner = editor.inner.borrow_mut();
                inner.state.palettes.remove(index);
                persist(&inner.state);
                inner.state.palettes.len()
            };
            selector.remove(index as i32);
            selector.set_active(count.checked_sub(1).map(|last| index.min(last) as u32));
            editor.refresh();
        }
    });
    add.connect_clicked({
        let editor = editor.clone();
        move |_| {
            let Some(window) = editor.window.upgrade() else {
                return;
            };
            let color = Swatch {
                name: String::new(),
                rgb: [82, 56, 222],
            };
            if let Some(color) = edit_swatch(&window, color) {
                editor.edit_colors(|colors| colors.push(color));
            }
        }
    });
    latest.connect_clicked({
        let editor = editor.clone();
        let status = status.clone();
        move |_| {
            let color = editor
                .inner
                .borrow()
                .state
                .colours
                .last()
                .map(|item| item.colour);
            if let Some(rgb) = color {
                editor.edit_colors(|colors| {
                    colors.push(Swatch {
                        name: String::new(),
                        rgb,
                    })
                });
                status.set_text("Latest capture added.");
            } else {
                status.set_text("Capture a screen color first.");
            }
        }
    });
    export.connect_clicked({
        let editor = editor.clone();
        move |_| {
            let (Some(index), Some(window), Some(format)) = (
                editor.selected(),
                editor.window.upgrade(),
                formats
                    .active()
                    .and_then(|i| PaletteFormat::ALL.get(i as usize)),
            ) else {
                return;
            };
            let data = export_palette(&editor.inner.borrow().state.palettes[index], *format);
            save_file(&window, format.filename(), data, &status);
        }
    });
    if !inner.borrow().state.palettes.is_empty() {
        selector.set_active(Some(0));
    }
    editor.refresh();
    (root, editor)
}

struct GradientEditor {
    inner: State,
    list: glib::WeakRef<GtkBox>,
    preview: glib::WeakRef<DrawingArea>,
    css: glib::WeakRef<Entry>,
}

impl GradientEditor {
    fn refresh_preview(&self) {
        if let Some(preview) = self.preview.upgrade() {
            preview.queue_draw();
        }
        if let Some(css) = self.css.upgrade() {
            css.set_text(&format!(
                "background: {};",
                self.inner.borrow().state.gradient.css()
            ));
        }
    }

    fn edit(&self, update: impl FnOnce(&mut crate::color::studio::Gradient)) {
        {
            let mut inner = self.inner.borrow_mut();
            update(&mut inner.state.gradient);
            persist(&inner.state);
        }
        self.refresh_preview();
    }

    fn render_stops(self: &Rc<Self>) {
        let Some(list) = self.list.upgrade() else {
            return;
        };
        while let Some(child) = list.first_child() {
            list.remove(&child);
        }
        let stops = self.inner.borrow().state.gradient.stops.clone();
        let removable = stops.len() > 2;
        for (index, stop) in stops.into_iter().enumerate() {
            let row = GtkBox::new(Orientation::Horizontal, 8);
            let picker = color_button(stop.rgb);
            picker.set_title("Stop color");
            row.append(&picker);
            let value = Entry::new();
            value.set_text(&hex(stop.rgb));
            value.set_width_chars(8);
            value.set_tooltip_text(Some("Enter a six-digit HEX color, such as #23C7B2"));
            row.append(&value);
            let spacer = GtkBox::new(Orientation::Horizontal, 0);
            spacer.set_hexpand(true);
            row.append(&spacer);
            let position = SpinButton::with_range(0.0, 100.0, 1.0);
            position.set_value(f64::from(stop.percent));
            position.set_tooltip_text(Some("Stop position (%)"));
            row.append(&position);
            row.append(&Label::new(Some("%")));
            let remove = Button::with_label("Remove");
            remove.set_sensitive(removable);
            row.append(&remove);
            let editor = self.clone();
            let entry = value.downgrade();
            picker.connect_color_set(move |picker| {
                let rgb = selected_rgb(picker);
                if let Some(entry) = entry.upgrade() {
                    entry.set_text(&hex(rgb));
                }
                editor.edit(|gradient| gradient.stops[index].rgb = rgb);
            });
            let editor = self.clone();
            value.connect_changed(move |entry| {
                let Some(rgb) = parse_hex_color(&entry.text()) else {
                    entry.add_css_class("error");
                    return;
                };
                entry.remove_css_class("error");
                if selected_rgb(&picker) != rgb {
                    picker.set_rgba(&rgba(rgb));
                    editor.edit(|gradient| gradient.stops[index].rgb = rgb);
                }
            });
            let editor = self.clone();
            position.connect_value_changed(move |position| {
                editor
                    .edit(|gradient| gradient.stops[index].percent = position.value_as_int() as u8);
            });
            let editor = self.clone();
            remove.connect_clicked(move |_| {
                editor.edit(|gradient| {
                    gradient.stops.remove(index);
                });
                editor.render_stops();
            });
            list.append(&row);
        }
    }
}

fn gradient_page(window: &ApplicationWindow, inner: &State) -> GtkBox {
    let root = page();
    let preview = DrawingArea::new();
    preview.set_size_request(-1, 104);
    preview.set_draw_func({
        let inner = inner.clone();
        move |_, cr, width, height| {
            let _ = inner
                .borrow()
                .state
                .gradient
                .draw(cr, f64::from(width), f64::from(height));
        }
    });
    root.append(&preview);
    let controls = GtkBox::new(Orientation::Horizontal, 8);
    controls.append(&Label::new(Some("Angle (°)")));
    let angle = SpinButton::with_range(0.0, 359.0, 1.0);
    angle.set_value(f64::from(inner.borrow().state.gradient.angle));
    controls.append(&angle);
    let add = Button::with_label("Add stop");
    add.set_hexpand(true);
    add.set_halign(Align::End);
    controls.append(&add);
    root.append(&controls);
    let list = GtkBox::new(Orientation::Vertical, 6);
    let list_scroll = scrolled(&list);
    list_scroll.set_vexpand(true);
    root.append(&list_scroll);
    let css = Entry::new();
    css.set_editable(false);
    css.set_width_chars(1);
    root.append(&css);
    let actions = GtkBox::new(Orientation::Horizontal, 6);
    let copy = Button::with_label("Copy CSS");
    let export = Button::with_label("Export PNG");
    actions.append(&copy);
    actions.append(&export);
    root.append(&actions);
    let status = label("");
    root.append(&status);

    let editor = Rc::new(GradientEditor {
        inner: inner.clone(),
        list: list.downgrade(),
        preview: preview.downgrade(),
        css: css.downgrade(),
    });
    angle.connect_value_changed({
        let editor = editor.clone();
        move |angle| editor.edit(|gradient| gradient.angle = angle.value_as_int() as u16)
    });
    add.connect_clicked({
        let editor = editor.clone();
        move |_| {
            editor.edit(|gradient| {
                gradient.stops.push(GradientStop {
                    rgb: [243, 146, 58],
                    percent: 50,
                })
            });
            editor.render_stops();
        }
    });
    copy.connect_clicked({
        let status = status.clone();
        let sound = inner.borrow().sound.clone();
        move |_| {
            clipboard::copy_to_clipboard(&css.text());
            status.set_text("CSS copied.");
            if sound.get() {
                crate::audio::Feedback::Copy.play();
            }
        }
    });
    export.connect_clicked({
        let window = window.downgrade();
        let inner = inner.clone();
        move |_| {
            if let Some(window) = window.upgrade() {
                let gradient = inner.borrow().state.gradient.clone();
                let data = render_png(1200, 400, |cr| gradient.draw(cr, 1200.0, 400.0));
                save_file(&window, "gradient.png", data, &status);
            }
        }
    });
    editor.render_stops();
    editor.refresh_preview();
    root
}

fn extract_page(
    window: &ApplicationWindow,
    create_palette: impl Fn(String, Vec<Swatch>) + 'static,
    tabs: &gtk::Notebook,
) -> GtkBox {
    let root = page();
    let controls = GtkBox::new(Orientation::Horizontal, 8);
    let open = Button::with_label("Open image");
    controls.append(&open);
    controls.append(&Label::new(Some("Colors")));
    let count = SpinButton::with_range(2.0, 24.0, 1.0);
    count.set_value(8.0);
    controls.append(&count);
    root.append(&controls);
    let status = label("Choose an image to extract its dominant colors.");
    root.append(&status);
    let list = GtkBox::new(Orientation::Vertical, 6);
    let list_scroll = scrolled(&list);
    list_scroll.set_vexpand(true);
    root.append(&list_scroll);
    let save = Button::with_label("Create palette");
    save.set_sensitive(false);
    save.set_halign(Align::Start);
    root.append(&save);

    let result = Rc::new(RefCell::new(None::<(String, Vec<Swatch>)>));
    let busy = Rc::new(std::cell::Cell::new(false));
    let selected_file = Rc::new(RefCell::new(None::<std::path::PathBuf>));
    let run = Rc::new({
        let result = result.clone();
        let busy = busy.clone();
        let selected_file = selected_file.clone();
        let list = list.downgrade();
        let save = save.downgrade();
        let status = status.downgrade();
        let open = open.downgrade();
        let count = count.downgrade();
        move || {
            let Some(path) = selected_file.borrow().clone() else {
                return;
            };
            let (Some(list), Some(save), Some(status), Some(open), Some(count)) = (
                list.upgrade(),
                save.upgrade(),
                status.upgrade(),
                open.upgrade(),
                count.upgrade(),
            ) else {
                return;
            };
            if busy.replace(true) {
                return;
            }
            save.set_sensitive(false);
            open.set_sensitive(false);
            count.set_sensitive(false);
            status.set_text("Extracting colors…");
            while let Some(child) = list.first_child() {
                list.remove(&child);
            }
            result.borrow_mut().take();
            let limit = count.value_as_int() as usize;
            let name = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("Image palette")
                .to_owned();
            let result = result.clone();
            let busy = busy.clone();
            glib::MainContext::default().spawn_local(async move {
                let colors = gio::spawn_blocking(move || extract_colors(&path, limit)).await;
                busy.set(false);
                open.set_sensitive(true);
                count.set_sensitive(true);
                match colors {
                    Ok(Ok(colors)) if !colors.is_empty() => {
                        let colors: Vec<_> = colors
                            .into_iter()
                            .enumerate()
                            .map(|(i, rgb)| Swatch {
                                name: format!("Color {}", i + 1),
                                rgb,
                            })
                            .collect();
                        for color in &colors {
                            let row = GtkBox::new(Orientation::Horizontal, 8);
                            row.append(&swatch(color.rgb));
                            let description = label(&hex(color.rgb));
                            description.set_hexpand(true);
                            row.append(&description);
                            list.append(&row);
                        }
                        status.set_text(&format!("{} · {} colors", name, colors.len()));
                        *result.borrow_mut() = Some((name, colors));
                        save.set_sensitive(true);
                    }
                    Ok(Ok(_)) => status.set_text("The image has no visible colors."),
                    Ok(Err(error)) => status.set_text(&format!("Could not read image: {error}")),
                    Err(_) => status.set_text("Color extraction failed."),
                }
            });
        }
    });
    open.connect_clicked({
        let window = window.downgrade();
        let run = run.clone();
        move |_| {
            let Some(window) = window.upgrade() else {
                return;
            };
            let chooser = FileChooserNative::new(
                Some("Open image"),
                Some(&window),
                FileChooserAction::Open,
                Some("Open"),
                Some("Cancel"),
            );
            let filter = FileFilter::new();
            filter.set_name(Some("Images"));
            filter.add_pixbuf_formats();
            chooser.add_filter(&filter);
            if glib::MainContext::default().block_on(chooser.run_future()) == ResponseType::Accept {
                if let Some(path) = chooser.file().and_then(|file| file.path()) {
                    *selected_file.borrow_mut() = Some(path);
                    run();
                }
            }
        }
    });
    count.connect_value_changed(move |_| run());
    save.connect_clicked({
        let tabs = tabs.downgrade();
        let window = window.downgrade();
        move |_| {
            let (Some((name, colors)), Some(window)) = (result.borrow().clone(), window.upgrade())
            else {
                return;
            };
            if let Some(name) = edit_name(&window, "New palette", &name) {
                create_palette(name, colors);
                if let Some(tabs) = tabs.upgrade() {
                    tabs.set_current_page(Some(1));
                }
            }
        }
    });
    root
}
