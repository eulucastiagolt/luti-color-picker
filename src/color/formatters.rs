use super::convert::{rgb_to_hsl, rgb_to_lab};

pub static FORMAT_NAMES: &[&str] = &[
    "CSS hex",
    "CSS uppercase hex",
    "CSS rgb",
    "CSS rgba",
    "CSS rgb (new-style)",
    "CSS rgba (new-style)",
    "CSS lab",
    "CSS hsl",
    "GDK.RGBA",
    "QML Qt.rgba",
    "Android resource",
];

pub fn format_color(format_name: &str, r: u8, g: u8, b: u8, color_name: &str) -> String {
    match format_name {
        "CSS hex" => format!("#{r:02x}{g:02x}{b:02x}"),
        "CSS uppercase hex" => format!("#{r:02X}{g:02X}{b:02X}"),
        "CSS rgb" => format!("rgb({r}, {g}, {b})"),
        "CSS rgba" => format!("rgba({r}, {g}, {b}, 1)"),
        "CSS rgb (new-style)" => format!("rgb({r}  {g}  {b})"),
        "CSS rgba (new-style)" => format!("rgb({r}  {g}  {b} / 100%)"),
        "CSS lab" => {
            let lab = rgb_to_lab(r, g, b);
            format!("lab({:.0}%  {:.0}  {:.0} / 100%)", lab[0], lab[1], lab[2])
        }
        "CSS hsl" => {
            let (h, s, l) = rgb_to_hsl(r, g, b);
            format!("hsl({h:.0}deg  {s:.0}%  {l:.0}%)")
        }
        "GDK.RGBA" => format!(
            "Gdk.RGBA({:.3}, {:.3}, {:.3}, 1.0)",
            r as f64 / 255.0,
            g as f64 / 255.0,
            b as f64 / 255.0
        ),
        "QML Qt.rgba" => format!(
            "Qt.rgba({:.3}, {:.3}, {:.3}, 1.0)",
            r as f64 / 255.0,
            g as f64 / 255.0,
            b as f64 / 255.0
        ),
        "Android resource" => {
            let clean_name = color_name
                .to_lowercase()
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '_' })
                .collect::<String>();
            format!("<color name=\"{clean_name}\">#{r:02x}{g:02x}{b:02x}</color>")
        }
        _ => format!("rgb({r}, {g}, {b})"),
    }
}
