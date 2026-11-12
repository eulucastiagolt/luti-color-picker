use crate::color::studio::{Gradient, Palette};
use serde::{Deserialize, Serialize};
use std::fs;
use std::ops::RangeInclusive;
use std::path::PathBuf;

pub const MIN_ZOOM: i32 = 1;
pub const MAX_ZOOM: i32 = 20;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct HistoryColorItem {
    pub imgdata: String,
    pub colour: [u8; 3],
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct WindowMetrics {
    #[serde(default)]
    pub ww: Option<f64>,
    #[serde(default)]
    pub wh: Option<f64>,
    #[serde(default)]
    pub wx: Option<f64>,
    #[serde(default)]
    pub wy: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub auto_copy: bool,
    pub sound: bool,
    pub background: bool,
    pub global_shortcut: bool,
    pub min_zoom: i32,
    pub max_zoom: i32,
    pub choose_palette_for_history: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            auto_copy: true,
            sound: true,
            background: false,
            global_shortcut: false,
            min_zoom: 2,
            max_zoom: 7,
            choose_palette_for_history: false,
        }
    }
}

impl Settings {
    pub fn zoom_range(&self) -> RangeInclusive<i32> {
        let min = self.min_zoom.clamp(MIN_ZOOM, MAX_ZOOM);
        min..=self.max_zoom.clamp(min, MAX_ZOOM)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AppState {
    #[serde(default)]
    pub colours: Vec<HistoryColorItem>,
    #[serde(default = "default_formatter")]
    pub formatter: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metrics: Option<WindowMetrics>,
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub palettes: Vec<Palette>,
    #[serde(default)]
    pub gradient: Gradient,
}

fn default_formatter() -> String {
    "CSS rgb".to_string()
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            colours: Vec::new(),
            formatter: default_formatter(),
            metrics: None,
            settings: Settings::default(),
            palettes: Vec::new(),
            gradient: Gradient::default(),
        }
    }
}

pub fn get_cache_file_path() -> PathBuf {
    glib::user_data_dir()
        .join("luti-color-picker")
        .join("state.json")
}

pub fn load_state() -> AppState {
    let mut path = get_cache_file_path();
    let contents = fs::read_to_string(&path).or_else(|error| {
        if error.kind() != std::io::ErrorKind::NotFound {
            return Err(error);
        }
        path = glib::user_cache_dir().join("colour-picker.json");
        fs::read_to_string(&path)
    });

    match contents {
        Ok(content) => match serde_json::from_str::<AppState>(&content) {
            Ok(state) => state,
            Err(e) => {
                eprintln!(
                    "Failed to parse {}: {}, assuming defaults",
                    path.display(),
                    e
                );
                AppState::default()
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => AppState::default(),
        Err(e) => {
            eprintln!(
                "Failed to read {}: {}, assuming defaults",
                path.display(),
                e
            );
            AppState::default()
        }
    }
}

pub fn save_state(state: &AppState) -> Result<(), std::io::Error> {
    let path = get_cache_file_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json_data = serde_json::to_string_pretty(state)?;
    glib::file_set_contents(path, json_data.as_bytes()).map_err(std::io::Error::other)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serde_compatibility() {
        let json_sample = r#"{
  "colours": [
    {
      "imgdata": "iVBORw0KGgoAAAANSUhEUg==",
      "colour": [255, 128, 0]
    }
  ],
  "formatter": "CSS hex",
  "metrics": {
    "ww": 0.25,
    "wh": 0.5,
    "wx": 0.1,
    "wy": 0.2
  }
}"#;
        let state: AppState = serde_json::from_str(json_sample).unwrap();
        assert_eq!(state.colours.len(), 1);
        assert_eq!(state.colours[0].colour, [255, 128, 0]);
        assert!(state.palettes.is_empty());
        assert_eq!(state.gradient, Gradient::default());
        assert_eq!(state.formatter, "CSS hex");
        assert!(state.metrics.is_some());
        let metrics = state.metrics.unwrap();
        assert_eq!(metrics.ww, Some(0.25));
    }

    #[test]
    fn test_null_metrics_compatibility() {
        let json_sample = r#"{
  "colours": [],
  "formatter": "CSS uppercase hex",
  "metrics": {
    "ww": null,
    "wh": null,
    "wx": null,
    "wy": null
  }
}"#;
        let state: AppState = serde_json::from_str(json_sample).unwrap();
        assert_eq!(state.formatter, "CSS uppercase hex");
        assert!(state.metrics.is_some());
        let metrics = state.metrics.unwrap();
        assert_eq!(metrics.ww, None);
    }
}
