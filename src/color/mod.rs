pub mod convert;
pub mod delta_e;
pub mod formatters;
pub mod names;
pub mod snark;
pub mod studio;

use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NamedColor {
    pub rgb: (u8, u8, u8),
    pub lab: [f64; 3],
    pub name: &'static str,
}

pub struct ColorDatabase {
    palette: &'static [NamedColor],
    cache: HashMap<(u8, u8, u8), &'static str>,
}

impl ColorDatabase {
    pub fn new(snark_mode: bool) -> Self {
        let palette = if snark_mode {
            snark::SNARK_NAMES
        } else {
            names::COLOUR_NAMES
        };
        Self {
            palette,
            cache: HashMap::new(),
        }
    }

    pub fn closest_name(&mut self, r: u8, g: u8, b: u8) -> &'static str {
        if let Some(&name) = self.cache.get(&(r, g, b)) {
            return name;
        }

        let target_lab = convert::rgb_to_lab(r, g, b);
        let mut min_de = f64::MAX;
        let mut closest_name = "Unknown";

        for item in self.palette {
            let de = delta_e::delta_e(&target_lab, &item.lab);
            if de < min_de {
                min_de = de;
                closest_name = item.name;
            }
        }

        self.cache.insert((r, g, b), closest_name);
        closest_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_to_lab_pure_white() {
        let lab = convert::rgb_to_lab(255, 255, 255);
        // White L should be ~100.0, a ~0, b ~0
        assert!((lab[0] - 100.0).abs() < 0.1);
        assert!(lab[1].abs() < 0.1);
        assert!(lab[2].abs() < 0.1);
    }

    #[test]
    fn test_rgb_to_lab_pure_black() {
        let lab = convert::rgb_to_lab(0, 0, 0);
        assert!(lab[0].abs() < 0.001);
        assert!(lab[1].abs() < 0.001);
        assert!(lab[2].abs() < 0.001);
    }

    #[test]
    fn test_closest_name() {
        let mut db = ColorDatabase::new(false);
        assert_eq!(db.closest_name(0, 0, 0), "Black");
        assert_eq!(db.closest_name(255, 255, 255), "White");
        assert_eq!(db.closest_name(255, 255, 0), "Yellow");
        assert_eq!(db.closest_name(255, 0, 0), "Red");
    }
}
