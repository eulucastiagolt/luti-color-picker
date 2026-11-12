use gdk_pixbuf::Pixbuf;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::path::Path;

pub type Rgb = [u8; 3];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Swatch {
    pub name: String,
    pub rgb: Rgb,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Palette {
    pub name: String,
    pub colors: Vec<Swatch>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GradientStop {
    pub rgb: Rgb,
    pub percent: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Gradient {
    pub angle: u16,
    pub stops: Vec<GradientStop>,
}

impl Default for Gradient {
    fn default() -> Self {
        Self {
            angle: 90,
            stops: vec![
                GradientStop {
                    rgb: [82, 56, 222],
                    percent: 0,
                },
                GradientStop {
                    rgb: [35, 199, 178],
                    percent: 100,
                },
            ],
        }
    }
}

impl Gradient {
    pub fn css(&self) -> String {
        let mut stops = self.stops.clone();
        stops.sort_by_key(|stop| stop.percent);
        let colors: Vec<_> = stops
            .iter()
            .map(|stop| format!("{} {}%", hex(stop.rgb), stop.percent))
            .collect();
        format!("linear-gradient({}deg, {})", self.angle, colors.join(", "))
    }

    pub fn draw(&self, cr: &cairo::Context, width: f64, height: f64) -> Result<(), cairo::Error> {
        let angle = f64::from(self.angle).to_radians();
        let (dx, dy) = (angle.sin(), -angle.cos());
        let length = (width * dx.abs() + height * dy.abs()) / 2.0;
        let (x, y) = (width / 2.0, height / 2.0);
        let pattern = cairo::LinearGradient::new(
            x - dx * length,
            y - dy * length,
            x + dx * length,
            y + dy * length,
        );
        let mut stops = self.stops.clone();
        stops.sort_by_key(|stop| stop.percent);
        for stop in &stops {
            let [r, g, b] = fractions(stop.rgb);
            pattern.add_color_stop_rgb(f64::from(stop.percent) / 100.0, r, g, b);
        }
        cr.set_source(pattern)?;
        cr.paint()
    }
}

pub fn hex([r, g, b]: Rgb) -> String {
    format!("#{r:02X}{g:02X}{b:02X}")
}

pub fn fractions(rgb: Rgb) -> [f64; 3] {
    rgb.map(|channel| f64::from(channel) / 255.0)
}

#[derive(Clone, Copy, PartialEq)]
pub enum PaletteFormat {
    Png,
    Svg,
    Text,
    Css,
    Tailwind3,
    Tailwind4,
    Json,
    Gpl,
}

impl PaletteFormat {
    pub const ALL: [Self; 8] = [
        Self::Png,
        Self::Svg,
        Self::Text,
        Self::Css,
        Self::Tailwind3,
        Self::Tailwind4,
        Self::Json,
        Self::Gpl,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Png => "PNG image",
            Self::Svg => "SVG vector",
            Self::Text => "Plain text",
            Self::Css => "CSS variables",
            Self::Tailwind3 => "Tailwind CSS v3",
            Self::Tailwind4 => "Tailwind CSS v4",
            Self::Json => "JSON",
            Self::Gpl => "GIMP / Inkscape palette",
        }
    }

    pub fn filename(self) -> &'static str {
        match self {
            Self::Png => "palette.png",
            Self::Svg => "palette.svg",
            Self::Text => "palette.txt",
            Self::Css => "palette.css",
            Self::Tailwind3 => "tailwind.config.cjs",
            Self::Tailwind4 => "theme.css",
            Self::Json => "palette.json",
            Self::Gpl => "palette.gpl",
        }
    }
}

fn named_colors(palette: &Palette) -> Vec<(String, String)> {
    let mut seen = HashSet::new();
    palette
        .colors
        .iter()
        .enumerate()
        .map(|(index, swatch)| {
            let words = swatch.name.split(|c: char| !c.is_ascii_alphanumeric());
            let mut base = words
                .filter(|word| !word.is_empty())
                .collect::<Vec<_>>()
                .join("-")
                .to_ascii_lowercase();
            if base.is_empty() {
                base = format!("color-{}", index + 1);
            } else if base.starts_with(|c: char| c.is_ascii_digit()) {
                base.insert_str(0, "color-");
            }
            let mut name = base.clone();
            let mut suffix = 2;
            while !seen.insert(name.clone()) {
                name = format!("{base}-{suffix}");
                suffix += 1;
            }
            (name, hex(swatch.rgb))
        })
        .collect()
}

fn dimensions(count: usize) -> Result<(i32, i32, i32), String> {
    let columns = count.clamp(1, 5);
    let height = count
        .div_ceil(columns)
        .max(1)
        .checked_mul(100)
        .and_then(|value| i32::try_from(value).ok())
        .ok_or("Palette image is too large")?;
    Ok((columns as i32, columns as i32 * 144, height))
}

pub fn render_png(
    width: i32,
    height: i32,
    draw: impl FnOnce(&cairo::Context) -> Result<(), cairo::Error>,
) -> Result<Vec<u8>, String> {
    let mut png = Vec::new();
    let surface = cairo::ImageSurface::create(cairo::Format::Rgb24, width, height)
        .map_err(|e| e.to_string())?;
    let cr = cairo::Context::new(&surface).map_err(|e| e.to_string())?;
    draw(&cr).map_err(|e| e.to_string())?;
    surface.write_to_png(&mut png).map_err(|e| e.to_string())?;
    Ok(png)
}

pub fn export_palette(palette: &Palette, format: PaletteFormat) -> Result<Vec<u8>, String> {
    if palette.colors.is_empty() {
        return Err("Add a color before exporting.".into());
    }

    let text = match format {
        PaletteFormat::Png | PaletteFormat::Svg => {
            let (columns, width, height) = dimensions(palette.colors.len())?;
            let tiles = palette.colors.iter().enumerate().map(|(i, color)| {
                let i = i as i32;
                let [r, g, b] = fractions(color.rgb);
                let ink = if 0.299 * r + 0.587 * g + 0.114 * b > 0.57 { 0.0 } else { 1.0 };
                (i % columns * 144, i / columns * 100, color.rgb, ink)
            });
            if format == PaletteFormat::Png {
                return render_png(width, height, |cr| {
                    cr.set_source_rgb(1.0, 1.0, 1.0);
                    cr.paint()?;
                    cr.select_font_face("monospace", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
                    cr.set_font_size(15.0);
                    for (x, y, rgb, ink) in tiles {
                        let [r, g, b] = fractions(rgb);
                        cr.set_source_rgb(r, g, b);
                        cr.rectangle(f64::from(x), f64::from(y), 144.0, 100.0);
                        cr.fill()?;
                        cr.set_source_rgb(ink, ink, ink);
                        cr.move_to(f64::from(x + 12), f64::from(y + 84));
                        cr.show_text(&hex(rgb))?;
                    }
                    Ok(())
                });
            }
            let mut svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n");
            writeln!(svg, "  <title>{}</title>", glib::markup_escape_text(&palette.name)).unwrap();
            for (x, y, rgb, ink) in tiles {
                let color = hex(rgb);
                let foreground = if ink == 0.0 { "#000000" } else { "#FFFFFF" };
                writeln!(svg, "  <rect x=\"{x}\" y=\"{y}\" width=\"144\" height=\"100\" fill=\"{color}\"/>").unwrap();
                writeln!(svg, "  <text x=\"{}\" y=\"{}\" fill=\"{foreground}\" font-family=\"monospace\" font-size=\"15\">{color}</text>", x + 12, y + 84).unwrap();
            }
            svg.push_str("</svg>\n");
            svg
        }
        PaletteFormat::Text => palette.colors.iter().map(|color| format!("{}\t{}\n", hex(color.rgb), color.name)).collect(),
        PaletteFormat::Json => serde_json::to_string_pretty(&serde_json::json!({
            "name": palette.name,
            "colors": palette.colors.iter().map(|color| serde_json::json!({"name":color.name,"hex":hex(color.rgb),"rgb":color.rgb})).collect::<Vec<_>>()
        })).map_err(|e| e.to_string())?,
        PaletteFormat::Gpl => {
            let mut text = format!("GIMP Palette\nName: {}\nColumns: 5\n#\n", palette.name.replace(['\r', '\n'], " "));
            for color in &palette.colors {
                let [r, g, b] = color.rgb;
                writeln!(text, "{r:3} {g:3} {b:3}\t{}", color.name.replace(['\r', '\n'], " ")).unwrap();
            }
            text
        }
        PaletteFormat::Css | PaletteFormat::Tailwind4 => {
            let mut text: String = if format == PaletteFormat::Css { ":root {\n".into() } else { "@theme {\n".into() };
            for (name, color) in named_colors(palette) {
                writeln!(text, "  --color-{name}: {color};").unwrap();
            }
            text.push_str("}\n");
            text
        }
        PaletteFormat::Tailwind3 => {
            let mut text = String::from("module.exports = {\n  theme: {\n    extend: {\n      colors: {\n");
            for (name, color) in named_colors(palette) {
                writeln!(text, "        '{name}': '{color}',").unwrap();
            }
            text.push_str("      },\n    },\n  },\n};\n");
            text
        }
    };
    Ok(text.into_bytes())
}

pub fn extract_colors(path: &Path, limit: usize) -> Result<Vec<Rgb>, String> {
    let image = Pixbuf::from_file_at_scale(path, 256, 256, true).map_err(|e| e.to_string())?;
    let pixels = image.read_pixel_bytes();
    let channels = image.n_channels() as usize;
    let rowstride = image.rowstride() as usize;
    let width = image.width() as usize;
    let mut buckets: HashMap<[u8; 3], (u32, [u32; 3])> = HashMap::new();

    for row in 0..image.height() as usize {
        let start = row * rowstride;
        for pixel in pixels[start..start + width * channels].chunks_exact(channels) {
            if channels == 4 && pixel[3] < 128 {
                continue;
            }
            let rgb = [pixel[0], pixel[1], pixel[2]];
            let (count, sum) = buckets.entry(rgb.map(|channel| channel >> 5)).or_default();
            *count += 1;
            for (total, channel) in sum.iter_mut().zip(rgb) {
                *total += u32::from(channel);
            }
        }
    }

    let mut ranked: Vec<_> = buckets
        .into_values()
        .map(|(count, sum)| (count, sum.map(|total| ((total + count / 2) / count) as u8)))
        .collect();
    ranked.sort_unstable_by(|(a_count, a_rgb), (b_count, b_rgb)| {
        b_count.cmp(a_count).then(a_rgb.cmp(b_rgb))
    });
    Ok(ranked.into_iter().take(limit).map(|(_, rgb)| rgb).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_exports_vector_swatches_and_unique_tokens() {
        let palette = Palette {
            name: "Brand & UI".into(),
            colors: vec![
                Swatch {
                    name: "Accent color".into(),
                    rgb: [255, 0, 0],
                },
                Swatch {
                    name: "Accent color".into(),
                    rgb: [0, 0, 255],
                },
            ],
        };

        for format in PaletteFormat::ALL {
            assert!(!export_palette(&palette, format).unwrap().is_empty());
        }
        let png = export_palette(&palette, PaletteFormat::Png).unwrap();
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        let svg = String::from_utf8(export_palette(&palette, PaletteFormat::Svg).unwrap()).unwrap();
        assert!(svg.contains("<title>Brand &amp; UI</title>"));
        assert_eq!(svg.matches("<rect ").count(), 2);
        assert!(svg.contains("fill=\"#FF0000\""));
        let v3 =
            String::from_utf8(export_palette(&palette, PaletteFormat::Tailwind3).unwrap()).unwrap();
        assert!(v3.contains("extend: {"));
        assert!(v3.contains("'accent-color-2': '#0000FF'"));
        let v4 =
            String::from_utf8(export_palette(&palette, PaletteFormat::Tailwind4).unwrap()).unwrap();
        assert!(v4.starts_with("@theme {"));
        assert!(v4.contains("--color-accent-color-2: #0000FF;"));
    }

    #[test]
    fn css_gradient_sorts_stops_by_position() {
        let mut gradient = Gradient::default();
        gradient.stops.push(GradientStop {
            rgb: [255, 255, 255],
            percent: 30,
        });
        assert_eq!(
            gradient.css(),
            "linear-gradient(90deg, #5238DE 0%, #FFFFFF 30%, #23C7B2 100%)"
        );
    }

    #[test]
    fn extraction_returns_dominant_colors_from_png() {
        let bytes = render_png(30, 20, |cr| {
            cr.set_source_rgb(1.0, 0.0, 0.0);
            cr.paint()?;
            cr.set_source_rgb(0.0, 0.0, 1.0);
            cr.rectangle(0.0, 0.0, 10.0, 20.0);
            cr.fill()
        })
        .unwrap();
        use std::io::Write;
        use std::os::fd::FromRawFd;

        let (fd, path) = glib::file_open_tmp(Some("luti-extraction-XXXXXX")).unwrap();
        let mut file = unsafe { std::fs::File::from_raw_fd(fd) };
        file.write_all(&bytes).unwrap();
        drop(file);
        let colors = extract_colors(&path, 2);
        std::fs::remove_file(path).unwrap();
        assert_eq!(colors.unwrap(), vec![[255, 0, 0], [0, 0, 255]]);
    }
}
