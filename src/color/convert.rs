/// Color space conversions matching the original Python implementation

#[inline]
pub fn round_4(val: f64) -> f64 {
    (val * 10000.0).round() / 10000.0
}

/// Convert sRGB [0..255] to CIE L*a*b* using D65 illuminant (2 degree observer)
pub fn rgb_to_lab(r: u8, g: u8, b: u8) -> [f64; 3] {
    let mut rgb = [0.0; 3];
    let inputs = [r, g, b];
    for (i, &val) in inputs.iter().enumerate() {
        let v = val as f64 / 255.0;
        let v = if v > 0.04045 {
            ((v + 0.055) / 1.055).powf(2.4)
        } else {
            v / 12.92
        };
        rgb[i] = v * 100.0;
    }

    let x = rgb[0] * 0.4124 + rgb[1] * 0.3576 + rgb[2] * 0.1805;
    let y = rgb[0] * 0.2126 + rgb[1] * 0.7152 + rgb[2] * 0.0722;
    let z = rgb[0] * 0.0193 + rgb[1] * 0.1192 + rgb[2] * 0.9505;

    let mut xyz = [
        round_4(x) / 95.047,
        round_4(y) / 100.0,
        round_4(z) / 108.883,
    ];

    for val in &mut xyz {
        if *val > 0.008856 {
            *val = val.powf(1.0 / 3.0);
        } else {
            *val = (7.787 * *val) + (16.0 / 116.0);
        }
    }

    let l = (116.0 * xyz[1]) - 16.0;
    let a = 500.0 * (xyz[0] - xyz[1]);
    let b = 200.0 * (xyz[1] - xyz[2]);

    [round_4(l), round_4(a), round_4(b)]
}

/// Convert sRGB [0..255] to HLS [Hue: 0..360, Lightness: 0..100, Saturation: 0..100]
/// Matches Python's `colorsys.rgb_to_hls` used in Pick
pub fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let rf = r as f64 / 255.0;
    let gf = g as f64 / 255.0;
    let bf = b as f64 / 255.0;

    let maxc = rf.max(gf).max(bf);
    let minc = rf.min(gf).min(bf);

    let l = (minc + maxc) / 2.0;

    if (minc - maxc).abs() < f64::EPSILON {
        return (0.0, 0.0, l * 100.0);
    }

    let s = if l <= 0.5 {
        (maxc - minc) / (maxc + minc)
    } else {
        (maxc - minc) / (2.0 - maxc - minc)
    };

    let rc = (maxc - rf) / (maxc - minc);
    let gc = (maxc - gf) / (maxc - minc);
    let bc = (maxc - bf) / (maxc - minc);

    let mut h = if (rf - maxc).abs() < f64::EPSILON {
        bc - gc
    } else if (gf - maxc).abs() < f64::EPSILON {
        2.0 + rc - bc
    } else {
        4.0 + gc - rc
    };

    h = (h / 6.0).rem_euclid(1.0);

    // Pick converts:
    // h = hls[0] * 360
    // s = hls[2] * 100
    // l = hls[1] * 100
    (h * 360.0, s * 100.0, l * 100.0)
}
