use super::studio::Rgb;

pub fn contrast_ratio(first: Rgb, second: Rgb) -> f64 {
    let luminance = |rgb: Rgb| {
        let [r, g, b] = rgb.map(to_linear);
        0.2126 * r + 0.7152 * g + 0.0722 * b
    };
    let first = luminance(first);
    let second = luminance(second);
    (first.max(second) + 0.05) / (first.min(second) + 0.05)
}

const STEPS: [(u16, f64, f64); 11] = [
    (50, 0.98, 0.18),
    (100, 0.95, 0.28),
    (200, 0.90, 0.48),
    (300, 0.83, 0.70),
    (400, 0.74, 0.90),
    (500, 0.65, 1.00),
    (600, 0.55, 0.96),
    (700, 0.46, 0.83),
    (800, 0.37, 0.67),
    (900, 0.28, 0.51),
    (950, 0.18, 0.35),
];

pub fn shades(base: Rgb) -> [(u16, Rgb, bool); 11] {
    let [lightness, a, b] = to_oklab(base);
    let original = STEPS
        .iter()
        .enumerate()
        .min_by(|(_, x), (_, y)| (x.1 - lightness).abs().total_cmp(&(y.1 - lightness).abs()))
        .map_or(0, |(index, _)| index);
    let (_, anchor_lightness, anchor_chroma) = STEPS[original];

    std::array::from_fn(|index| {
        let (level, step_lightness, step_chroma) = STEPS[index];
        if index == original {
            return (level, base, true);
        }
        let endpoint = if index < original {
            STEPS[0].1
        } else {
            STEPS[10].1
        };
        let position = (step_lightness - anchor_lightness) / (endpoint - anchor_lightness);
        let adjusted = lightness + (endpoint - lightness) * position;
        let chroma = (step_chroma / anchor_chroma).min(2.5);
        (level, in_gamut(adjusted, a * chroma, b * chroma), false)
    })
}

fn to_linear(channel: u8) -> f64 {
    let value = f64::from(channel) / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn to_oklab(rgb: Rgb) -> [f64; 3] {
    let [r, g, b] = rgb.map(to_linear);
    let l = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b).cbrt();
    let m = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b).cbrt();
    let s = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b).cbrt();
    [
        0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
    ]
}

fn to_linear_rgb(lightness: f64, a: f64, b: f64) -> [f64; 3] {
    let l = (lightness + 0.3963377774 * a + 0.2158037573 * b).powi(3);
    let m = (lightness - 0.1055613458 * a - 0.0638541728 * b).powi(3);
    let s = (lightness - 0.0894841775 * a - 1.2914855480 * b).powi(3);
    [
        4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s,
    ]
}

fn in_gamut(lightness: f64, a: f64, b: f64) -> Rgb {
    let mut rgb = to_linear_rgb(lightness, a, b);
    if !rgb.iter().all(|value| (0.0..=1.0).contains(value)) {
        let (mut low, mut high) = (0.0, 1.0);
        rgb = to_linear_rgb(lightness, 0.0, 0.0);
        for _ in 0..16 {
            let scale = (low + high) / 2.0;
            let candidate = to_linear_rgb(lightness, a * scale, b * scale);
            if candidate.iter().all(|value| (0.0..=1.0).contains(value)) {
                low = scale;
                rgb = candidate;
            } else {
                high = scale;
            }
        }
    }
    rgb.map(|value| {
        let value = if value <= 0.0031308 {
            12.92 * value
        } else {
            1.055 * value.powf(1.0 / 2.4) - 0.055
        };
        (value * 255.0).round() as u8
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contrast_uses_linear_light() {
        assert_eq!(contrast_ratio([0; 3], [255; 3]), 21.0);
        assert_eq!(contrast_ratio([127; 3], [127; 3]), 1.0);
        assert!(contrast_ratio([118; 3], [255; 3]) >= 4.5);
        assert!(contrast_ratio([119; 3], [255; 3]) < 4.5);
    }

    #[test]
    fn shades_preserve_original_and_descend_in_lightness() {
        for base in [
            [0; 3],
            [255; 3],
            [128; 3],
            [255, 0, 0],
            [0, 255, 0],
            [0, 0, 255],
        ] {
            let colors = shades(base);
            let originals: Vec<_> = colors.iter().filter(|shade| shade.2).collect();
            assert_eq!(originals.len(), 1);
            assert_eq!(originals[0].1, base);
            assert!(
                colors
                    .windows(2)
                    .all(|pair| { to_oklab(pair[0].1)[0] > to_oklab(pair[1].1)[0] }),
                "{colors:?}"
            );
        }
    }
}
