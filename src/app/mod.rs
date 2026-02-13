//! App module - contains the main application state and logic

mod context_menu;
mod downloads;
mod filters;
mod modals;
mod thumbnails;
mod updates;
mod views;

use crate::constants::*;
use crate::db::{Database, Map};
use crate::settings::Settings;
use crate::theme;
use crate::types::*;
use crate::utils::{get_cache_dir, process_cache_refresh};
use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

// ============================================================================
// APP STATE
// ============================================================================

pub struct App {
    pub(crate) db: Database,
    pub(crate) maps: Vec<Map>,
    pub(crate) filtered_indices: Vec<usize>,
    pub(crate) search_query: String,
    pub(crate) focus_search: bool,
    pub(crate) logo_texture: Option<egui::TextureHandle>,
    pub(crate) selected_indices: HashSet<usize>,
    pub(crate) last_selected: Option<usize>,
    pub(crate) last_clicked_item: Option<usize>,
    pub(crate) map_list_focused: bool,
    // Column visibility settings
    pub(crate) show_category: bool,
    pub(crate) show_stars: bool,
    pub(crate) show_points: bool,
    pub(crate) show_author: bool,
    pub(crate) show_release_date: bool,
    pub(crate) show_settings: bool,
    // View mode
    pub(crate) compact_view: bool,
    pub(crate) large_thumbnails: bool,
    // Column widths (resizable)
    pub(crate) col_widths: [f32; 6],
    // Column order (indices into col_widths)
    pub(crate) col_order: Vec<usize>,
    // Dragging state
    pub(crate) dragging_col: Option<usize>,
    pub(crate) resizing_col: Option<usize>,
    // Filters
    pub(crate) filter_categories: [bool; 8],
    pub(crate) category_mode_range: bool,
    pub(crate) category_range: (u8, u8),
    pub(crate) filter_stars: [bool; 5],
    pub(crate) stars_mode_range: bool,
    pub(crate) stars_range: (u8, u8),
    pub(crate) filter_downloaded: u8,
    pub(crate) year_mode_range: bool,
    pub(crate) year_range: Option<(i32, i32)>,
    pub(crate) filter_years: HashSet<i32>,
    pub(crate) available_years: Vec<i32>,
    pub(crate) show_filters: bool,
    // Download state
    pub(crate) download_state: Arc<Mutex<DownloadState>>,
    pub(crate) download_path: PathBuf,
    pub(crate) download_path_str: String,
    pub(crate) runtime: tokio::runtime::Runtime,
    // Thumbnail cache
    pub(crate) thumbnail_cache: HashMap<String, Option<egui::TextureHandle>>,
    pub(crate) prefetch_started: bool,
    pub(crate) cache_dir: PathBuf,
    // Preview viewer state (multi-tab)
    pub(crate) preview_maps: Vec<String>,
    pub(crate) preview_active_tab: usize,
    pub(crate) preview_textures: HashMap<String, Option<egui::TextureHandle>>,
    pub(crate) preview_loading: HashSet<String>,
    pub(crate) preview_zoom: f32,
    pub(crate) preview_offset: egui::Vec2,
    pub(crate) preview_dragging: bool,
    pub(crate) preview_needs_fit: bool,
    // Sorting
    pub(crate) sort_column: Option<SortColumn>,
    pub(crate) sort_direction: SortDirection,
    pub(crate) saved_sort: Option<(Option<SortColumn>, SortDirection)>,
    // Indexed scrollbar
    pub(crate) scroll_index_markers: Vec<ScrollIndexMarker>,
    pub(crate) scroll_target_row: Option<usize>,
    pub(crate) main_scroll_offset: f32,
    pub(crate) main_content_height: f32,
    pub(crate) main_viewport_height: f32,
    pub(crate) scroll_sync_item: Option<usize>,
    // Central panel rect for toast positioning
    pub(crate) central_panel_rect: Option<egui::Rect>,
    // Auto-update state
    pub(crate) update_check_done: bool,
    pub(crate) app_update_available: Option<String>,
    pub(crate) app_update_body: Option<String>,
    pub(crate) show_app_update_dialog: bool,
    pub(crate) update_in_progress: bool,
    pub(crate) app_update_error: Option<String>,
    pub(crate) app_update_success: Option<String>,
    // Toast notification
    pub(crate) toast_message: Option<String>,
    pub(crate) toast_start: Option<std::time::Instant>,
    // Download modal state
    pub(crate) show_download_modal: bool,
    pub(crate) show_download_log: bool,
    pub(crate) download_log_filter: Option<&'static str>,
    pub(crate) cancel_token: Option<CancellationToken>,
    // Settings
    pub(crate) play_sound_on_complete: bool,
    pub(crate) window_pos: Option<egui::Pos2>,
    pub(crate) window_size: Option<egui::Vec2>,
    pub(crate) was_downloading: bool,
    pub(crate) needs_center: bool,
    pub(crate) data_dir: PathBuf,
    pub(crate) view_switch_count: u32,
    pub(crate) list_row_height: f32,
    pub(crate) grid_scroll_target: Option<f32>,
    pub(crate) grid_scroll_to_row: Option<usize>,
}

// ============================================================================
// APP INITIALIZATION & HELPERS
// ============================================================================

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, db: Database, settings: Settings, data_dir: PathBuf) -> Self {
        // Force dark theme
        cc.egui_ctx.set_theme(egui::Theme::Dark);

        // Add Phosphor icons font
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

        // Add regular font (with Proportional fallbacks for icons)
        fonts.font_data.insert(
            "regular".to_owned(),
            std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
                "../../assets/fonts/Ubuntu-Regular.ttf"
            ))),
        );
        let mut regular_fonts = vec!["regular".to_owned()];
        // Add all Proportional fallbacks so icons (Phosphor) render
        if let Some(proportional) = fonts.families.get(&egui::FontFamily::Proportional) {
            regular_fonts.extend(proportional.clone());
        }
        fonts
            .families
            .insert(egui::FontFamily::Name("Regular".into()), regular_fonts);

        cc.egui_ctx.set_fonts(fonts);

        // Apply theme from theme.rs
        theme::apply_visuals(&cc.egui_ctx);

        let maps = db.get_all_maps().unwrap_or_default();
        let filtered_indices: Vec<usize> = (0..maps.len()).collect();

        let download_path = settings.download_path_or_default();

        let cache_dir = get_cache_dir();
        std::fs::create_dir_all(&cache_dir).ok();

        // Process cache refresh for version upgrades
        process_cache_refresh(&cache_dir);

        let mut app = Self {
            db,
            maps,
            filtered_indices,
            search_query: String::new(),
            focus_search: false,
            logo_texture: None,
            selected_indices: HashSet::new(),
            last_selected: None,
            last_clicked_item: None,
            map_list_focused: true,
            show_category: settings.col_category,
            show_stars: settings.col_stars,
            show_points: settings.col_points,
            show_author: settings.col_author,
            show_release_date: settings.col_release_date,
            show_settings: false,
            compact_view: settings.compact_view,
            large_thumbnails: settings.large_thumbnails,
            col_widths: [
                settings.col_w_name,
                settings.col_w_category,
                settings.col_w_stars,
                settings.col_w_points,
                settings.col_w_author,
                settings.col_w_date,
            ],
            col_order: settings.col_order,
            dragging_col: None,
            resizing_col: None,
            filter_categories: [true; 8],
            category_mode_range: true,
            category_range: (0, 4),
            filter_stars: [true; 5],
            stars_mode_range: true,
            stars_range: (1, 5),
            show_filters: true,
            download_state: Arc::new(Mutex::new(DownloadState::default())),
            download_path: download_path.clone(),
            download_path_str: download_path.to_string_lossy().to_string(),
            runtime: tokio::runtime::Runtime::new().unwrap(),
            thumbnail_cache: HashMap::new(),
            prefetch_started: false,
            cache_dir,
            preview_maps: Vec::new(),
            preview_active_tab: 0,
            preview_textures: HashMap::new(),
            preview_loading: HashSet::new(),
            preview_zoom: 1.0,
            preview_offset: egui::Vec2::ZERO,
            preview_dragging: false,
            preview_needs_fit: false,
            sort_column: Some(SortColumn::Name),
            sort_direction: SortDirection::Ascending,
            saved_sort: None,
            scroll_index_markers: Vec::new(),
            scroll_target_row: None,
            main_scroll_offset: 0.0,
            main_content_height: 0.0,
            main_viewport_height: 0.0,
            scroll_sync_item: None,
            central_panel_rect: None,
            update_check_done: false,
            app_update_available: None,
            app_update_body: None,
            show_app_update_dialog: false,
            update_in_progress: false,
            app_update_error: None,
            app_update_success: None,
            toast_message: None,
            toast_start: None,
            show_download_modal: false,
            show_download_log: false,
            download_log_filter: None,
            cancel_token: None,
            play_sound_on_complete: settings.play_sound,
            window_pos: None,
            window_size: None,
            filter_downloaded: 0,
            year_mode_range: true,
            year_range: None,
            filter_years: HashSet::new(),
            available_years: Vec::new(),
            was_downloading: false,
            needs_center: false,
            data_dir,
            view_switch_count: 0,
            list_row_height: 29.0,
            grid_scroll_target: None,
            grid_scroll_to_row: None,
        };

        // Compute available years from maps
        let mut years: Vec<i32> = app
            .maps
            .iter()
            .filter_map(|m| {
                let date = &m.release_date;
                if date.len() >= 4 && date.chars().take(4).all(|c| c.is_ascii_digit()) {
                    date[..4].parse().ok()
                } else {
                    None
                }
            })
            .collect();
        years.sort();
        years.dedup();
        app.available_years = years.clone();
        app.filter_years = years.into_iter().collect();

        // Build initial scroll index
        app.build_scroll_index();
        app
    }

    pub fn save_settings(&self) {
        let settings = Settings {
            window_x: self.window_pos.map(|p| p.x),
            window_y: self.window_pos.map(|p| p.y),
            window_w: self.window_size.map(|s| s.x),
            window_h: self.window_size.map(|s| s.y),
            col_category: self.show_category,
            col_stars: self.show_stars,
            col_points: self.show_points,
            col_author: self.show_author,
            col_release_date: self.show_release_date,
            col_w_name: self.col_widths[0],
            col_w_category: self.col_widths[1],
            col_w_stars: self.col_widths[2],
            col_w_points: self.col_widths[3],
            col_w_author: self.col_widths[4],
            col_w_date: self.col_widths[5],
            col_order: self.col_order.clone(),
            compact_view: self.compact_view,
            large_thumbnails: self.large_thumbnails,
            download_path: Some(self.download_path_str.clone()),
            play_sound: self.play_sound_on_complete,
        };
        settings.save(&self.data_dir);
    }

    /// Backwards-compatible alias
    pub fn save_column_settings(&self) {
        self.save_settings();
    }

    pub fn is_col_visible(&self, col_idx: usize) -> bool {
        match col_idx {
            0 => true,
            1 => self.show_category,
            2 => self.show_stars,
            3 => self.show_points,
            4 => self.show_author,
            5 => self.show_release_date,
            _ => false,
        }
    }

    pub fn col_name(&self, col_idx: usize) -> &'static str {
        match col_idx {
            0 => "NAME",
            1 => "CATEGORY",
            2 => "STARS",
            3 => "POINTS",
            4 => "AUTHOR",
            5 => "RELEASED",
            _ => "",
        }
    }

    pub fn category_index(cat: &str) -> Option<usize> {
        match cat {
            "Easy" => Some(0),
            "Main" => Some(1),
            "Hard" => Some(2),
            "Insane" => Some(3),
            "Extreme" => Some(4),
            "Solo" => Some(5),
            "Mod" => Some(6),
            "Extra" => Some(7),
            _ => None,
        }
    }

    pub const CATEGORY_NAMES: [&'static str; 8] = [
        "Easy", "Main", "Hard", "Insane", "Extreme", "Solo", "Mod", "Extra",
    ];

    pub fn get_map_url(map: &Map) -> String {
        format!(
            "{}/{}/{}star/{}.map",
            MAPS_BASE_URL, map.category, map.stars, map.name
        )
    }
}
