//! User settings stored as settings.json in the app data directory

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    // Window geometry
    pub window_x: Option<f32>,
    pub window_y: Option<f32>,
    pub window_w: Option<f32>,
    pub window_h: Option<f32>,

    // Column visibility
    pub col_category: bool,
    pub col_stars: bool,
    pub col_points: bool,
    pub col_author: bool,
    pub col_release_date: bool,

    // Column widths
    pub col_w_name: f32,
    pub col_w_category: f32,
    pub col_w_stars: f32,
    pub col_w_points: f32,
    pub col_w_author: f32,
    pub col_w_date: f32,

    // Column order
    pub col_order: Vec<usize>,

    // View
    pub compact_view: bool,
    pub large_thumbnails: bool,

    // Paths
    pub download_path: Option<String>,

    // Audio
    pub play_sound: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            window_x: None,
            window_y: None,
            window_w: None,
            window_h: None,
            col_category: true,
            col_stars: true,
            col_points: true,
            col_author: true,
            col_release_date: true,
            col_w_name: 200.0,
            col_w_category: 80.0,
            col_w_stars: 90.0,
            col_w_points: 50.0,
            col_w_author: 150.0,
            col_w_date: 100.0,
            col_order: vec![0, 1, 2, 3, 4, 5],
            compact_view: false,
            large_thumbnails: true,
            download_path: None,
            play_sound: true,
        }
    }
}

impl Settings {
    pub fn load(data_dir: &Path) -> Self {
        let path = data_dir.join("settings.json");
        match std::fs::read_to_string(&path) {
            Ok(s) => match serde_json::from_str(&s) {
                Ok(settings) => {
                    debug!(path = %path.display(), "Settings loaded");
                    settings
                }
                Err(e) => {
                    warn!(error = %e, "Failed to parse settings, using defaults");
                    Self::default()
                }
            },
            Err(_) => {
                debug!("No settings file found, using defaults");
                Self::default()
            }
        }
    }

    pub fn save(&self, data_dir: &Path) {
        let path = data_dir.join("settings.json");
        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&path, json) {
                    warn!(error = %e, "Failed to save settings");
                }
            }
            Err(e) => warn!(error = %e, "Failed to serialize settings"),
        }
    }

    pub fn download_path_or_default(&self) -> PathBuf {
        self.download_path
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                dirs::config_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("DDNet")
                    .join("maps")
                    .join("Gores Map Downloader")
            })
    }
}
