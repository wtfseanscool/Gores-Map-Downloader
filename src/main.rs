#![windows_subsystem = "windows"]
//! Gores Map Downloader - Main entry point

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod app;
mod constants;
mod db;
mod settings;
mod theme;
mod types;
mod ui;
mod utils;

use app::App;
use constants::*;
use db::Database;
use eframe::egui;
use std::collections::HashSet;
use std::path::PathBuf;
use tracing::{debug, error, info, warn};
use types::*;
use ui::components::{format_release_date, render_stars};
use utils::{format_bytes, get_cache_dir};

/// Initialize file logging. Returns a guard that must be held for the app lifetime.
fn init_logging(data_dir: &std::path::Path) -> tracing_appender::non_blocking::WorkerGuard {
    use tracing_subscriber::{fmt, EnvFilter, prelude::*};

    let logs_dir = data_dir.join("logs");
    std::fs::create_dir_all(&logs_dir).ok();

    let file_appender = tracing_appender::rolling::daily(&logs_dir, "gores-map-downloader.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,gores_map_downloader=debug"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_target(true)
                .with_thread_ids(false)
                .with_file(true)
                .with_line_number(true),
        )
        .init();

    guard
}

fn main() -> eframe::Result<()> {
    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Gores Map Downloader");

    std::fs::create_dir_all(&data_dir).ok();

    // Initialize logging - guard must live for entire app lifetime
    let _log_guard = init_logging(&data_dir);

    info!(version = APP_VERSION, "Gores Map Downloader starting");

    let db_path = data_dir.join("maps.db");
    let db = match Database::open(&db_path) {
        Ok(db) => {
            info!(path = %db_path.display(), "Database opened");
            db
        }
        Err(e) => {
            error!(error = %e, path = %db_path.display(), "Failed to open database");
            panic!("Failed to open database: {}", e);
        }
    };

    // Load initial data if database is empty
    if db.map_count().unwrap_or(0) == 0 {
        info!("Database empty, fetching initial manifest");
        if let Ok(response) = reqwest::blocking::get(MANIFEST_URL) {
            if let Ok(manifest) = response.json::<Manifest>() {
                let imported = db.import_maps(&manifest.maps).unwrap_or(0);
                db.set_db_version(&manifest.version).ok();
                info!(count = imported, "Imported maps from manifest");
            }
        }
    }

    // Load saved window position/size
    let settings = settings::Settings::load(&data_dir);
    let win_pos = match (settings.window_x, settings.window_y) {
        (Some(x), Some(y)) => Some(egui::pos2(x, y)),
        _ => None,
    };
    let win_size = match (settings.window_w, settings.window_h) {
        (Some(w), Some(h)) => Some(egui::vec2(w, h)),
        _ => None,
    };

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size(win_size.unwrap_or(egui::vec2(1450.0, 800.0)))
        .with_min_inner_size([1330.0, 720.0])
        .with_title("Gores Map Downloader");

    // Set window/taskbar icon from PNG
    {
        let icon_data = include_bytes!("../assets/icon.png");
        let icon_img = image::load_from_memory(icon_data).unwrap().to_rgba8();
        let (w, h) = (icon_img.width(), icon_img.height());
        let icon = egui::IconData { rgba: icon_img.into_raw(), width: w, height: h };
        viewport = viewport.with_icon(std::sync::Arc::new(icon));
    }

    let needs_center = win_pos.is_none();

    if let Some(pos) = win_pos {
        viewport = viewport.with_position(pos);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Gores Map Downloader",
        options,
        Box::new(move |cc| {
            let mut app = App::new(cc, db, settings, data_dir);
            app.needs_center = needs_center;
            Ok(Box::new(app))
        }),
    )
}

// ============================================================================
// MAIN UPDATE LOOP & UI RENDERING
// ============================================================================

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        // Track window position/size for saving on exit
        ctx.input(|i| {
            if let Some(rect) = i.viewport().outer_rect {
                self.window_pos = Some(rect.min);
            }
            if let Some(rect) = i.viewport().inner_rect {
                self.window_size = Some(rect.size());
            }
        });

        // Global keyboard capture: type anywhere to search (when no modal open)
        if !self.show_settings && !self.show_download_modal && !ctx.wants_keyboard_input() {
            let mut typed_text = String::new();
            let mut backspace = false;
            ctx.input(|i| {
                for event in &i.events {
                    if let egui::Event::Text(text) = event {
                        if !text.is_empty() && text.chars().all(|c| !c.is_control()) {
                            typed_text.push_str(text);
                        }
                    }
                    if let egui::Event::Key { key: egui::Key::Backspace, pressed: true, .. } = event {
                        backspace = true;
                    }
                }
            });
            if !typed_text.is_empty() {
                self.search_query.push_str(&typed_text);
                self.focus_search = true;
            }
            if backspace && !self.search_query.is_empty() {
                self.search_query.pop();
                self.focus_search = true;
                self.apply_filters();
            }
        }

        // Start thumbnail prefetch on first frame
        if !self.prefetch_started {
            self.prefetch_started = true;
            self.start_thumbnail_prefetch(ctx);
            self.check_for_updates(ctx);
        }

        // Center window on first launch
        if self.needs_center {
            self.needs_center = false;
            if let Some(cmd) = egui::ViewportCommand::center_on_screen(ctx) {
                ctx.send_viewport_cmd(cmd);
            }
        }

        // Check for update results from background threads
        self.poll_update_results(ctx);

        // Render update dialogs
        self.render_update_dialogs(ctx);

        // Render download modal
        self.render_download_modal(ctx);

        // Left sidebar - filters (must be added BEFORE CentralPanel)
        egui::SidePanel::left("filter_panel")
            .exact_width(260.0)
            .max_width(260.0)
            .min_width(260.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(theme::BG_BASE)
                    .inner_margin(egui::Margin { left: 16, right: 0, top: 0, bottom: 0 }),
            )
            .show(ctx, |ui| {
                // Capture panel rect at start for absolute positioning of bottom buttons
                let panel_max_rect = ui.max_rect();

                ui.set_max_width(244.0); // 260 - 16 (left margin only)
                // Header with logo, centered
                let avail_w = ui.available_width();

                ui.add_space(21.0);
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    let texture = self.logo_texture.get_or_insert_with(|| {
                        let (pixels, w, h) = utils::rasterize_logo(avail_w as u32 * 2);
                        ctx.load_texture(
                            "logo",
                            egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &pixels),
                            egui::TextureOptions::LINEAR,
                        )
                    });

                    let aspect = texture.size()[1] as f32 / texture.size()[0] as f32;
                    let logo_w = avail_w * 0.5;
                    let logo_size = egui::vec2(logo_w, logo_w * aspect);
                    ui.image(egui::load::SizedTexture::new(texture.id(), logo_size));

                    ui.add_space(4.0);
                    ui.add(egui::Label::new(
                        egui::RichText::new("GORES MAP DOWNLOADER")
                            .size(11.0)
                            .color(theme::TEXT_DIM),
                    ).selectable(false));
                });
                ui.add_space(11.0);

                // Search box with border style
                let search_frame_resp = egui::Frame::none()
                    .fill(theme::BG_INPUT)
                    .stroke(egui::Stroke::new(1.0, theme::BORDER_SUBTLE))
                    .corner_radius(4.0)
                    .inner_margin(egui::Margin::symmetric(8, 8))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(egui_phosphor::regular::MAGNIFYING_GLASS)
                                        .size(14.0)
                                        .color(theme::TEXT_DIM),
                                )
                                .selectable(false),
                            );
                            let search_id = ui.make_persistent_id("search_box");
                            let search_response = ui.add(
                                egui::TextEdit::singleline(&mut self.search_query)
                                    .id(search_id)
                                    .hint_text("Search map / author...")
                                    .frame(false)
                                    .desired_width(ui.available_width()),
                            );
                            if self.focus_search {
                                self.focus_search = false;
                                search_response.request_focus();
                                if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), search_id) {
                                    let ccursor = egui::text::CCursor::new(self.search_query.len());
                                    state.cursor.set_char_range(Some(egui::text::CCursorRange::one(ccursor)));
                                    state.store(ui.ctx(), search_id);
                                }
                                self.apply_filters();
                            }
                            if search_response.changed() {
                                self.apply_filters();
                            }
                            if search_response.has_focus() {
                                self.map_list_focused = false;
                            }
                        });
                    });
                // Clear button overlaid on right side of search frame
                if !self.search_query.is_empty() {
                    let frame_rect = search_frame_resp.response.rect;
                    let btn_size = 16.0;
                    let btn_rect = egui::Rect::from_center_size(
                        egui::pos2(frame_rect.right() - 14.0, frame_rect.center().y),
                        egui::vec2(btn_size, btn_size),
                    );
                    let clear_resp = ui.interact(btn_rect, ui.id().with("search_clear"), egui::Sense::click());
                    let color = if clear_resp.hovered() { theme::TEXT_MUTED } else { theme::TEXT_DIM };
                    if clear_resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    ui.painter().text(
                        btn_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        egui_phosphor::regular::X,
                        egui::FontId::proportional(12.0),
                        color,
                    );
                    if clear_resp.clicked() {
                        self.search_query.clear();
                        self.apply_filters();
                    }
                }

                ui.add_space(12.0);

                // Calculate space for bottom buttons (with padding above)
                let bottom_height = 28.0 + 32.0 + 4.0 + 6.0 + 36.0 + 6.0 + 14.0 + 8.0;
                let padding_above_buttons = 16.0;
                let available_for_filters =
                    ui.available_height() - bottom_height - padding_above_buttons;

                if self.show_filters {
                    let mut filters_changed = false;

                    // Scrollable filter area
                    let scroll_output = egui::ScrollArea::vertical()
                        .max_height(available_for_filters)
                        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                        .show(ui, |ui| {
                            // CATEGORY section
                            theme::section_frame().show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new("CATEGORY").color(theme::TEXT_DIM).size(11.0),
                                        )
                                        .selectable(false),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if theme::segmented_toggle_style1(
                                                ui,
                                                "Range",
                                                "Individual",
                                                &mut self.category_mode_range,
                                            ) {
                                                filters_changed = true;
                                            }
                                        },
                                    );
                                });

                                ui.add_space(8.0);

                                const DIFF_NAMES: [&str; 5] =
                                    ["Easy", "Main", "Hard", "Insane", "Extreme"];

                                if self.category_mode_range {
                                    // Labels row with fixed positions
                                    let (row_rect, _) = ui.allocate_exact_size(
                                        egui::vec2(ui.available_width(), 18.0),
                                        egui::Sense::hover(),
                                    );
                                    let painter = ui.painter();
                                    painter.text(
                                        row_rect.left_center(),
                                        egui::Align2::LEFT_CENTER,
                                        DIFF_NAMES[self.category_range.0 as usize],
                                        egui::FontId::proportional(12.0),
                                        egui::Color32::WHITE,
                                    );
                                    painter.text(
                                        row_rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        "to",
                                        egui::FontId::proportional(12.0),
                                        theme::TEXT_DIM,
                                    );
                                    painter.text(
                                        row_rect.right_center(),
                                        egui::Align2::RIGHT_CENTER,
                                        DIFF_NAMES[self.category_range.1 as usize],
                                        egui::FontId::proportional(12.0),
                                        egui::Color32::WHITE,
                                    );

                                    // Dual-handle range slider
                                    let slider_rect = ui.available_rect_before_wrap();
                                    let slider_rect = egui::Rect::from_min_size(
                                        slider_rect.min,
                                        egui::vec2(ui.available_width(), 20.0),
                                    );
                                    let (rect, response) = ui.allocate_exact_size(
                                        slider_rect.size(),
                                        egui::Sense::click_and_drag(),
                                    );
                                    if response.hovered() || response.dragged() {
                                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                    }

                                    let track_y = rect.center().y;
                                    let track_left = rect.left() + 8.0;
                                    let track_right = rect.right() - 8.0;
                                    let track_width = track_right - track_left;

                                    // Draw track background
                                    let painter = ui.painter();
                                    painter.line_segment(
                                        [
                                            egui::pos2(track_left, track_y),
                                            egui::pos2(track_right, track_y),
                                        ],
                                        egui::Stroke::new(4.0, theme::BORDER_SUBTLE),
                                    );

                                    // Calculate handle positions
                                    let min_x = track_left
                                        + (self.category_range.0 as f32 / 4.0) * track_width;
                                    let max_x = track_left
                                        + (self.category_range.1 as f32 / 4.0) * track_width;

                                    // Draw active range
                                    painter.line_segment(
                                        [egui::pos2(min_x, track_y), egui::pos2(max_x, track_y)],
                                        egui::Stroke::new(4.0, theme::SLIDER_TRAIL),
                                    );

                                    // Draw handles
                                    painter.circle_filled(
                                        egui::pos2(min_x, track_y),
                                        8.0,
                                        theme::SLIDER_HEAD,
                                    );
                                    painter.circle_filled(
                                        egui::pos2(max_x, track_y),
                                        8.0,
                                        theme::SLIDER_HEAD,
                                    );

                                    // Handle dragging
                                    if response.dragged() {
                                        if let Some(pos) = response.interact_pointer_pos() {
                                            let rel_x = ((pos.x - track_left) / track_width)
                                                .clamp(0.0, 1.0);
                                            let val = (rel_x * 4.0).round() as u8;

                                            // Determine which handle to move
                                            let dist_min = (pos.x - min_x).abs();
                                            let dist_max = (pos.x - max_x).abs();

                                            if dist_min < dist_max {
                                                if val <= self.category_range.1 {
                                                    self.category_range.0 = val;
                                                    filters_changed = true;
                                                }
                                            } else {
                                                if val >= self.category_range.0 {
                                                    self.category_range.1 = val;
                                                    filters_changed = true;
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    // Individual selection - 3 rows (3/3/2 layout)
                                    let names = [
                                        "Easy", "Main", "Hard", "Insane", "Extreme", "Solo", "Mod",
                                        "Extra",
                                    ];
                                    let selected_fill = theme::TOGGLE_SELECTED;
                                    let unselected_fill = theme::TOGGLE_UNSELECTED;
                                    let btn_width_3 = ((ui.available_width() - 8.0) / 3.0).floor();
                                    let btn_width_2 = ((ui.available_width() - 4.0) / 2.0).floor();

                                    // Row 1: Easy, Main, Hard
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 4.0;
                                        for i in 0..3 {
                                            let fill = if self.filter_categories[i] {
                                                selected_fill
                                            } else {
                                                unselected_fill
                                            };
                                            let (rect, response) = ui.allocate_exact_size(
                                                egui::vec2(btn_width_3, 24.0),
                                                egui::Sense::click(),
                                            );
                                            if response.hovered() {
                                                ui.ctx().set_cursor_icon(
                                                    egui::CursorIcon::PointingHand,
                                                );
                                            }
                                            if ui.is_rect_visible(rect) {
                                                let (fill, draw_rect) = theme::button_visual(&response, fill, rect);
                                                ui.painter().rect_filled(draw_rect, 4.0, fill);
                                                ui.painter().text(
                                                    draw_rect.center(),
                                                    egui::Align2::CENTER_CENTER,
                                                    names[i],
                                                    egui::FontId::proportional(11.0),
                                                    egui::Color32::WHITE,
                                                );
                                            }
                                            if response.clicked() {
                                                self.filter_categories[i] =
                                                    !self.filter_categories[i];
                                                filters_changed = true;
                                            }
                                        }
                                    });
                                    ui.add_space(4.0);
                                    // Row 2: Insane, Extreme, Solo
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 4.0;
                                        for i in 3..6 {
                                            let fill = if self.filter_categories[i] {
                                                selected_fill
                                            } else {
                                                unselected_fill
                                            };
                                            let (rect, response) = ui.allocate_exact_size(
                                                egui::vec2(btn_width_3, 24.0),
                                                egui::Sense::click(),
                                            );
                                            if response.hovered() {
                                                ui.ctx().set_cursor_icon(
                                                    egui::CursorIcon::PointingHand,
                                                );
                                            }
                                            if ui.is_rect_visible(rect) {
                                                let (fill, draw_rect) = theme::button_visual(&response, fill, rect);
                                                ui.painter().rect_filled(draw_rect, 4.0, fill);
                                                ui.painter().text(
                                                    draw_rect.center(),
                                                    egui::Align2::CENTER_CENTER,
                                                    names[i],
                                                    egui::FontId::proportional(11.0),
                                                    egui::Color32::WHITE,
                                                );
                                            }
                                            if response.clicked() {
                                                self.filter_categories[i] =
                                                    !self.filter_categories[i];
                                                filters_changed = true;
                                            }
                                        }
                                    });
                                    ui.add_space(4.0);
                                    // Row 3: Mod, Extra
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 4.0;
                                        for i in 6..8 {
                                            let fill = if self.filter_categories[i] {
                                                selected_fill
                                            } else {
                                                unselected_fill
                                            };
                                            let (rect, response) = ui.allocate_exact_size(
                                                egui::vec2(btn_width_2, 24.0),
                                                egui::Sense::click(),
                                            );
                                            if response.hovered() {
                                                ui.ctx().set_cursor_icon(
                                                    egui::CursorIcon::PointingHand,
                                                );
                                            }
                                            if ui.is_rect_visible(rect) {
                                                let (fill, draw_rect) = theme::button_visual(&response, fill, rect);
                                                ui.painter().rect_filled(draw_rect, 4.0, fill);
                                                ui.painter().text(
                                                    draw_rect.center(),
                                                    egui::Align2::CENTER_CENTER,
                                                    names[i],
                                                    egui::FontId::proportional(11.0),
                                                    egui::Color32::WHITE,
                                                );
                                            }
                                            if response.clicked() {
                                                self.filter_categories[i] =
                                                    !self.filter_categories[i];
                                                filters_changed = true;
                                            }
                                        }
                                    });
                                }
                            });

                            ui.add_space(4.0);

                            // STARS section
                            // Stars 4-5 only available for Solo (5), Mod (6), or Extra (7)
                            // In Range mode for categories, Solo/Mod/Extra are excluded, so 4-5 stars disabled
                            let has_solo_mod_extra = if self.category_mode_range {
                                false // Range mode excludes Solo/Mod/Extra
                            } else {
                                self.filter_categories[5]
                                    || self.filter_categories[6]
                                    || self.filter_categories[7]
                            };
                            let max_stars: u8 = if has_solo_mod_extra { 5 } else { 3 };

                            // Clamp current values if needed
                            if self.stars_range.1 > max_stars {
                                self.stars_range.1 = max_stars;
                                if self.stars_range.0 > max_stars {
                                    self.stars_range.0 = max_stars;
                                }
                                filters_changed = true;
                            }

                            theme::section_frame().show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new("STARS").color(theme::TEXT_DIM).size(11.0),
                                        )
                                        .selectable(false),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            let was_range = self.stars_mode_range;
                                            if theme::segmented_toggle(
                                                ui,
                                                "Range",
                                                "Individual",
                                                &mut self.stars_mode_range,
                                            ) {
                                                filters_changed = true;
                                            }
                                        },
                                    );
                                });

                                ui.add_space(8.0);

                                if self.stars_mode_range {
                                    // Labels row with fixed positions
                                    let (row_rect, _) = ui.allocate_exact_size(
                                        egui::vec2(ui.available_width(), 18.0),
                                        egui::Sense::hover(),
                                    );
                                    let painter = ui.painter();
                                    painter.text(
                                        row_rect.left_center(),
                                        egui::Align2::LEFT_CENTER,
                                        format!("{}★", self.stars_range.0),
                                        egui::FontId::proportional(12.0),
                                        egui::Color32::WHITE,
                                    );
                                    painter.text(
                                        row_rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        "to",
                                        egui::FontId::proportional(12.0),
                                        theme::TEXT_DIM,
                                    );
                                    painter.text(
                                        row_rect.right_center(),
                                        egui::Align2::RIGHT_CENTER,
                                        format!("{}★", self.stars_range.1),
                                        egui::FontId::proportional(12.0),
                                        egui::Color32::WHITE,
                                    );

                                    // Dual-handle range slider
                                    let (rect, response) = ui.allocate_exact_size(
                                        egui::vec2(ui.available_width(), 20.0),
                                        egui::Sense::click_and_drag(),
                                    );
                                    if response.hovered() || response.dragged() {
                                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                    }

                                    let track_y = rect.center().y;
                                    let track_left = rect.left() + 8.0;
                                    let track_right = rect.right() - 8.0;
                                    let track_width = track_right - track_left;
                                    let steps = (max_stars - 1) as f32;

                                    let painter = ui.painter();
                                    painter.line_segment(
                                        [
                                            egui::pos2(track_left, track_y),
                                            egui::pos2(track_right, track_y),
                                        ],
                                        egui::Stroke::new(4.0, theme::BORDER_SUBTLE),
                                    );

                                    let min_x = track_left
                                        + ((self.stars_range.0 - 1) as f32 / steps) * track_width;
                                    let max_x = track_left
                                        + ((self.stars_range.1 - 1) as f32 / steps) * track_width;

                                    painter.line_segment(
                                        [egui::pos2(min_x, track_y), egui::pos2(max_x, track_y)],
                                        egui::Stroke::new(4.0, theme::SLIDER_TRAIL),
                                    );

                                    painter.circle_filled(
                                        egui::pos2(min_x, track_y),
                                        8.0,
                                        theme::SLIDER_HEAD,
                                    );
                                    painter.circle_filled(
                                        egui::pos2(max_x, track_y),
                                        8.0,
                                        theme::SLIDER_HEAD,
                                    );

                                    if response.dragged() {
                                        if let Some(pos) = response.interact_pointer_pos() {
                                            let rel_x = ((pos.x - track_left) / track_width)
                                                .clamp(0.0, 1.0);
                                            let val =
                                                ((rel_x * steps).round() as u8 + 1).min(max_stars);

                                            let dist_min = (pos.x - min_x).abs();
                                            let dist_max = (pos.x - max_x).abs();

                                            if dist_min < dist_max {
                                                if val <= self.stars_range.1 {
                                                    self.stars_range.0 = val;
                                                    filters_changed = true;
                                                }
                                            } else {
                                                if val >= self.stars_range.0 {
                                                    self.stars_range.1 = val;
                                                    filters_changed = true;
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    // Individual selection - 5 buttons in a row
                                    let selected_fill = theme::TOGGLE_SELECTED;
                                    let unselected_fill = theme::TOGGLE_UNSELECTED;
                                    let disabled_fill = egui::Color32::from_rgb(0x1a, 0x1a, 0x1a);
                                    let btn_width = ((ui.available_width() - 16.0) / 5.0).floor();

                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 4.0;
                                        ui.spacing_mut().item_spacing.y = 0.0;
                                        for i in 0..5 {
                                            let enabled = i < 3 || has_solo_mod_extra;
                                            let fill = if !enabled {
                                                disabled_fill
                                            } else if self.filter_stars[i] {
                                                selected_fill
                                            } else {
                                                unselected_fill
                                            };
                                            let (rect, response) = ui.allocate_exact_size(
                                                egui::vec2(btn_width, 24.0),
                                                egui::Sense::click(),
                                            );
                                            if response.hovered() {
                                                ui.ctx().set_cursor_icon(if enabled {
                                                    egui::CursorIcon::PointingHand
                                                } else {
                                                    egui::CursorIcon::NotAllowed
                                                });
                                            }
                                            if ui.is_rect_visible(rect) {
                                                let (fill, draw_rect) = if enabled { theme::button_visual(&response, fill, rect) } else { (fill, rect) };
                                                ui.painter().rect_filled(draw_rect, 4.0, fill);
                                                ui.painter().text(
                                                    draw_rect.center(),
                                                    egui::Align2::CENTER_CENTER,
                                                    format!("{}", i + 1),
                                                    egui::FontId::proportional(11.0),
                                                    egui::Color32::WHITE,
                                                );
                                            }
                                            if enabled && response.clicked() {
                                                self.filter_stars[i] = !self.filter_stars[i];
                                                filters_changed = true;
                                            }
                                            if !enabled && self.filter_stars[i] {
                                                self.filter_stars[i] = false;
                                                filters_changed = true;
                                            }
                                        }
                                    });
                                }
                            });

                            ui.add_space(4.0);

                            // YEAR section
                            theme::section_frame().show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new("YEAR").color(theme::TEXT_DIM).size(11.0),
                                        )
                                        .selectable(false),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if theme::segmented_toggle_style1(
                                                ui,
                                                "Range",
                                                "Individual",
                                                &mut self.year_mode_range,
                                            ) {
                                                filters_changed = true;
                                            }
                                        },
                                    );
                                });
                                ui.add_space(8.0);

                                let years = &self.available_years;

                                if years.len() >= 2 {
                                    let min_year = *years.first().unwrap();
                                    let max_year = *years.last().unwrap();

                                    if self.year_mode_range {
                                        // Range mode - 2-handle slider
                                        let (cur_min, cur_max) =
                                            self.year_range.unwrap_or((min_year, max_year));

                                        // Labels row
                                        let (row_rect, _) = ui.allocate_exact_size(
                                            egui::vec2(ui.available_width(), 18.0),
                                            egui::Sense::hover(),
                                        );
                                        let painter = ui.painter();
                                        painter.text(
                                            row_rect.left_center(),
                                            egui::Align2::LEFT_CENTER,
                                            cur_min.to_string(),
                                            egui::FontId::proportional(12.0),
                                            egui::Color32::WHITE,
                                        );
                                        painter.text(
                                            row_rect.center(),
                                            egui::Align2::CENTER_CENTER,
                                            "to",
                                            egui::FontId::proportional(12.0),
                                            theme::TEXT_DIM,
                                        );
                                        painter.text(
                                            row_rect.right_center(),
                                            egui::Align2::RIGHT_CENTER,
                                            cur_max.to_string(),
                                            egui::FontId::proportional(12.0),
                                            egui::Color32::WHITE,
                                        );

                                        // Dual-handle range slider
                                        let (rect, response) = ui.allocate_exact_size(
                                            egui::vec2(ui.available_width(), 20.0),
                                            egui::Sense::click_and_drag(),
                                        );
                                        if response.hovered() || response.dragged() {
                                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                        }

                                        let track_y = rect.center().y;
                                        let track_left = rect.left() + 8.0;
                                        let track_right = rect.right() - 8.0;
                                        let track_width = track_right - track_left;
                                        let steps = (years.len() - 1) as f32;

                                        let painter = ui.painter();
                                        painter.line_segment(
                                            [
                                                egui::pos2(track_left, track_y),
                                                egui::pos2(track_right, track_y),
                                            ],
                                            egui::Stroke::new(4.0, theme::BORDER_SUBTLE),
                                        );

                                        // Find indices of cur_min and cur_max in available_years
                                        let min_idx =
                                            years.iter().position(|&y| y >= cur_min).unwrap_or(0);
                                        let max_idx = years
                                            .iter()
                                            .rposition(|&y| y <= cur_max)
                                            .unwrap_or(years.len() - 1);

                                        let min_x = if steps > 0.0 {
                                            track_left + (min_idx as f32 / steps) * track_width
                                        } else {
                                            track_left
                                        };
                                        let max_x = if steps > 0.0 {
                                            track_left + (max_idx as f32 / steps) * track_width
                                        } else {
                                            track_right
                                        };

                                        painter.line_segment(
                                            [
                                                egui::pos2(min_x, track_y),
                                                egui::pos2(max_x, track_y),
                                            ],
                                            egui::Stroke::new(4.0, theme::SLIDER_TRAIL),
                                        );

                                        painter.circle_filled(
                                            egui::pos2(min_x, track_y),
                                            8.0,
                                            theme::SLIDER_HEAD,
                                        );
                                        painter.circle_filled(
                                            egui::pos2(max_x, track_y),
                                            8.0,
                                            theme::SLIDER_HEAD,
                                        );

                                        if response.dragged() && years.len() > 1 {
                                            if let Some(pos) = response.interact_pointer_pos() {
                                                let rel_x = ((pos.x - track_left) / track_width)
                                                    .clamp(0.0, 1.0);
                                                // Snap to nearest valid year in available_years
                                                let idx = (rel_x * (years.len() - 1) as f32).round()
                                                    as usize;
                                                let val = years[idx.min(years.len() - 1)];

                                                let dist_min = (pos.x - min_x).abs();
                                                let dist_max = (pos.x - max_x).abs();

                                                let (new_min, new_max) = if dist_min < dist_max {
                                                    (val.min(cur_max), cur_max)
                                                } else {
                                                    (cur_min, val.max(cur_min))
                                                };

                                                if new_min != cur_min || new_max != cur_max {
                                                    self.year_range = Some((new_min, new_max));
                                                    filters_changed = true;
                                                }
                                            }
                                        }
                                    } else {
                                        // Individual mode - grid of year buttons
                                        let cols = 4;
                                        let spacing = 4.0;
                                        let btn_width = (ui.available_width()
                                            - spacing * (cols as f32 - 1.0))
                                            / cols as f32;
                                        let btn_height = 26.0;

                                        let selected_fill = theme::TOGGLE_SELECTED;
                                        let unselected_fill = theme::TOGGLE_UNSELECTED;

                                        let years_clone = years.clone();
                                        for row in years_clone.chunks(cols) {
                                            ui.horizontal(|ui| {
                                                ui.spacing_mut().item_spacing.x = spacing;
                                                for &year in row {
                                                    let selected =
                                                        self.filter_years.contains(&year);
                                                    let fill = if selected {
                                                        selected_fill
                                                    } else {
                                                        unselected_fill
                                                    };
                                                    let (rect, response) = ui.allocate_exact_size(
                                                        egui::vec2(btn_width, btn_height),
                                                        egui::Sense::click(),
                                                    );
                                                    if response.hovered() {
                                                        ui.ctx().set_cursor_icon(
                                                            egui::CursorIcon::PointingHand,
                                                        );
                                                    }
                                                    if ui.is_rect_visible(rect) {
                                                        let (fill, draw_rect) = theme::button_visual(&response, fill, rect);
                                                ui.painter().rect_filled(draw_rect, 4.0, fill);
                                                        // Show 2-digit year with apostrophe
                                                        let label = format!("'{}", year % 100);
                                                        ui.painter().text(
                                                            draw_rect.center(),
                                                            egui::Align2::CENTER_CENTER,
                                                            label,
                                                            egui::FontId::proportional(12.0),
                                                            egui::Color32::WHITE,
                                                        );
                                                    }
                                                    if response.clicked() {
                                                        if selected {
                                                            self.filter_years.remove(&year);
                                                        } else {
                                                            self.filter_years.insert(year);
                                                        }
                                                        filters_changed = true;
                                                    }
                                                }
                                            });
                                            ui.add_space(2.0);
                                        }
                                    }
                                }
                            });

                            ui.add_space(4.0);

                            // STATUS section (Downloaded filter)
                            theme::section_frame().show(ui, |ui| {
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new("STATUS").color(theme::TEXT_DIM).size(11.0),
                                    )
                                    .selectable(false),
                                );
                                ui.add_space(8.0);

                                let selected_fill = theme::TOGGLE_SELECTED;
                                let unselected_fill = theme::TOGGLE_UNSELECTED;
                                let btn_width = ((ui.available_width() - 8.0) / 3.0).floor();

                                // Icons with tooltips for equal-width buttons
                                let icons = [
                                    (egui_phosphor::regular::CIRCLE, "All"),
                                    (egui_phosphor::regular::CHECK_CIRCLE, "Downloaded"),
                                    (egui_phosphor::regular::X_CIRCLE, "Not Downloaded"),
                                ];

                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 4.0;
                                    for (i, (icon, tooltip)) in icons.iter().enumerate() {
                                        let fill = if self.filter_downloaded == i as u8 {
                                            selected_fill
                                        } else {
                                            unselected_fill
                                        };
                                        let (rect, response) = ui.allocate_exact_size(
                                            egui::vec2(btn_width, 24.0),
                                            egui::Sense::click(),
                                        );
                                        if response.hovered() {
                                            ui.ctx()
                                                .set_cursor_icon(egui::CursorIcon::PointingHand);
                                        }
                                        if ui.is_rect_visible(rect) {
                                            let (fill, draw_rect) = theme::button_visual(&response, fill, rect);
                                                ui.painter().rect_filled(draw_rect, 4.0, fill);
                                            ui.painter().text(
                                                rect.center(),
                                                egui::Align2::CENTER_CENTER,
                                                *icon,
                                                egui::FontId::proportional(14.0),
                                                egui::Color32::WHITE,
                                            );
                                        }
                                        if response.clicked() && self.filter_downloaded != i as u8 {
                                            self.filter_downloaded = i as u8;
                                            filters_changed = true;
                                        }
                                        response.on_hover_text(*tooltip);
                                    }
                                });
                            });
                        });

                    // Show "more below" indicator only when content is clipped
                    let content_height = scroll_output.content_size.y;
                    let viewport_height = scroll_output.inner_rect.height();
                    let scroll_offset = scroll_output.state.offset.y;
                    let has_more_below = content_height > viewport_height
                        && (scroll_offset + viewport_height) < content_height - 1.0;

                    if has_more_below {
                        ui.vertical_centered(|ui| {
                            ui.add(
                                egui::Label::new(egui::RichText::new("• • •").size(9.0).color(theme::TEXT_DIM))
                                    .selectable(false),
                            );
                        });
                    }

                    if filters_changed {
                        self.apply_filters();
                    }
                }

                // Bottom buttons - fixed at absolute bottom of panel
                let bottom_rect = egui::Rect::from_min_max(
                    egui::pos2(
                        panel_max_rect.left(),
                        panel_max_rect.bottom() - bottom_height,
                    ),
                    egui::pos2(panel_max_rect.right(), panel_max_rect.bottom()),
                );

                ui.allocate_ui_at_rect(bottom_rect, |ui| {
                    ui.set_min_width(bottom_rect.width());
                    ui.spacing_mut().item_spacing.y = 0.0; // Remove default vertical spacing

                    // Clear / Select All buttons (same line)
                    let btn_width = (ui.available_width() - 4.0) / 2.0;
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;

                        let clear_text = format!("{} Clear", egui_phosphor::regular::SQUARE);
                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(btn_width, 28.0),
                            egui::Sense::click(),
                        );
                        if response.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        let (fill, draw_rect) = theme::button_visual(&response, theme::BORDER_SUBTLE, rect);
                        ui.painter().rect_filled(draw_rect, 4.0, fill);
                        ui.painter().text(
                            draw_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            &clear_text,
                            egui::FontId::proportional(13.0),
                            egui::Color32::WHITE,
                        );
                        if response.clicked() {
                            self.selected_indices.clear();
                        }
                        response.on_hover_text("Escape");

                        let select_text =
                            format!("{} Select All", egui_phosphor::regular::CHECK_SQUARE);
                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(btn_width, 28.0),
                            egui::Sense::click(),
                        );
                        if response.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        let (fill, draw_rect) = theme::button_visual(&response, theme::BORDER_SUBTLE, rect);
                        ui.painter().rect_filled(draw_rect, 4.0, fill);
                        ui.painter().text(
                            draw_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            &select_text,
                            egui::FontId::proportional(13.0),
                            egui::Color32::WHITE,
                        );
                        if response.clicked() {
                            for &idx in &self.filtered_indices {
                                self.selected_indices.insert(idx);
                            }
                        }
                        response.on_hover_text("Ctrl+A");
                    });

                    ui.add_space(4.0);

                    // Preview button (full width, centered text)
                    let selected_count = self.selected_indices.len();
                    let preview_enabled = selected_count > 0;
                    let preview_rect = ui.available_rect_before_wrap();
                    let preview_rect = egui::Rect::from_min_size(
                        preview_rect.min,
                        egui::vec2(preview_rect.width(), 36.0),
                    );
                    let preview_response = ui.allocate_rect(preview_rect, egui::Sense::click());

                    let disabled_fill = egui::Color32::from_rgb(0x1a, 0x1a, 0x1a);
                    let preview_fill = if preview_enabled {
                        theme::BORDER_SUBTLE
                    } else {
                        disabled_fill
                    };
                    let (preview_fill, preview_draw) = if preview_enabled { theme::button_visual(&preview_response, preview_fill, preview_rect) } else { (preview_fill, preview_rect) };
                    ui.painter().rect_filled(preview_draw, 4.0, preview_fill);
                    let preview_text = format!("{} Preview Selected", egui_phosphor::regular::EYE);
                    let text_color = if preview_enabled {
                        egui::Color32::WHITE
                    } else {
                        theme::TEXT_DIM
                    };
                    ui.painter().text(
                        preview_draw.center(),
                        egui::Align2::CENTER_CENTER,
                        &preview_text,
                        egui::FontId::proportional(14.0),
                        text_color,
                    );

                    if preview_response.hovered() {
                        if preview_enabled {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        } else {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::NotAllowed);
                        }
                    }
                    let preview_clicked = preview_enabled && preview_response.clicked();
                    if preview_enabled {
                        preview_response.on_hover_text("Enter");
                    }
                    if preview_clicked {
                        let names: Vec<String> = self
                            .selected_indices
                            .iter()
                            .filter_map(|&idx| self.maps.get(idx).map(|m| m.name.clone()))
                            .collect();
                        if !names.is_empty() {
                            self.open_preview_multi(ctx, names);
                        }
                    }

                    ui.add_space(6.0);

                    // Download button (full width, centered text)
                    let download_state = self.download_state.lock().unwrap();
                    let is_downloading = download_state.active_count > 0;
                    drop(download_state);
                    let download_enabled = !is_downloading && selected_count > 0;

                    let download_rect = ui.available_rect_before_wrap();
                    let download_rect = egui::Rect::from_min_size(
                        download_rect.min,
                        egui::vec2(download_rect.width(), 40.0),
                    ); // 36
                    let download_response = ui.allocate_rect(download_rect, egui::Sense::click());

                    let download_fill = if download_enabled {
                        theme::BTN_ACCENT
                    } else {
                        disabled_fill
                    };
                    let (download_fill, download_draw) = if download_enabled { theme::button_visual(&download_response, download_fill, download_rect) } else { (download_fill, download_rect) };
                    ui.painter().rect_filled(download_draw, 4.0, download_fill);
                    let download_text_color = if download_enabled {
                        egui::Color32::from_rgb(0x04, 0x2f, 0x2e) // teal-950
                    } else {
                        theme::TEXT_DIM
                    };
                    {
                        let download_text = format!(
                            "{} Download Selected ({})",
                            egui_phosphor::regular::DOWNLOAD_SIMPLE,
                            selected_count
                        );
                        let text_font = if download_enabled {
                            egui::FontId::new(14.0, egui::FontFamily::Name("Regular".into()))
                        } else {
                            egui::FontId::proportional(14.0)
                        };
                        ui.painter().text(
                            download_draw.center(),
                            egui::Align2::CENTER_CENTER,
                            &download_text,
                            text_font,
                            download_text_color,
                        );
                    }

                    if download_response.hovered() {
                        if download_enabled {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        } else {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::NotAllowed);
                        }
                    }
                    let download_clicked = download_enabled && download_response.clicked();
                    if download_enabled {
                        download_response.on_hover_text("Ctrl+D");
                    }
                    if download_clicked {
                        self.download_selected(ctx);
                    }

                    ui.add_space(4.0);

                    // Version and credit at very bottom, justified
                    let version_color = egui::Color32::from_rgb(0x45, 0x45, 0x4c);
                    let bottom_y = ui.cursor().top();
                    let left_x = ui.cursor().left();
                    let right_x = panel_max_rect.right();
                    let font = egui::FontId::proportional(10.0);
                    let version_text = format!("v{}", APP_VERSION);
                    let credit_text = "made with ♥ by CAKExSNIFFERx42";
                    let version_galley = ui.painter().layout_no_wrap(version_text.clone(), font.clone(), version_color);
                    let credit_galley = ui.painter().layout_no_wrap(credit_text.to_string(), font.clone(), version_color);
                    let version_right = left_x + version_galley.size().x;
                    let credit_left = right_x - credit_galley.size().x;
                    let sep_x = (version_right + credit_left) / 2.0;
                    ui.painter().galley(egui::pos2(left_x, bottom_y), version_galley, version_color);
                    ui.painter().text(
                        egui::pos2(sep_x, bottom_y),
                        egui::Align2::CENTER_TOP,
                        "•",
                        font.clone(),
                        version_color,
                    );
                    ui.painter().galley(egui::pos2(right_x - credit_galley.size().x, bottom_y), credit_galley, version_color);
                });
            });

        // Settings modal (centered overlay)
        if self.show_settings {
            let modal_response = egui::Modal::new(egui::Id::new("settings_modal"))
                .backdrop_color(egui::Color32::from_black_alpha(120))
                .frame(egui::Frame::new()
                    .fill(egui::Color32::from_rgb(0x1a, 0x1a, 0x1e))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(0x2a, 0x2a, 0x2e)))
                    .corner_radius(8.0)
                    .inner_margin(egui::Margin::same(20)))
                .show(ctx, |ui| {
                    ui.set_width(320.0);

                    // Title bar with close button (matches preview window style)
                    ui.horizontal(|ui| {
                        ui.add(egui::Label::new(
                            egui::RichText::new("Settings").size(16.0).strong(),
                        ).selectable(false));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let close_size = 24.0;
                            let (rect, response) = ui.allocate_exact_size(
                                egui::vec2(close_size, close_size),
                                egui::Sense::click(),
                            );
                            let close_color = if response.hovered() {
                                ui.painter().rect_filled(rect, 4.0, theme::BG_SURFACE);
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                theme::STATUS_ERROR
                            } else {
                                theme::TEXT_DIM
                            };
                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                egui_phosphor::regular::X,
                                egui::FontId::proportional(16.0),
                                close_color,
                            );
                            if response.clicked() {
                                self.show_settings = false;
                            }
                        });
                    });
                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(theme::SPACING_SM);

                    let mut changed = false;

                    // — View —
                    ui.add(egui::Label::new(
                        egui::RichText::new("View").size(13.0).color(theme::ACCENT),
                    ).selectable(false));
                    ui.add_space(2.0);
                    if theme::settings_checkbox(ui, self.large_thumbnails, "Large Thumbnails", true) {
                        self.large_thumbnails = !self.large_thumbnails;
                    }

                    ui.add_space(theme::SPACING_MD);
                    ui.separator();
                    ui.add_space(theme::SPACING_SM);

                    // — Columns —
                    ui.add(egui::Label::new(
                        egui::RichText::new("Info Visibility").size(13.0).color(theme::ACCENT),
                    ).selectable(false));
                    ui.add_space(2.0);
                    theme::settings_checkbox(ui, true, "Name", false); // Always enabled, dimmed
                    for (val, label) in [
                        (&mut self.show_category, "Category"),
                        (&mut self.show_stars, "Stars"),
                        (&mut self.show_points, "Points"),
                        (&mut self.show_author, "Author"),
                        (&mut self.show_release_date, "Release Date"),
                    ] {
                        if theme::settings_checkbox(ui, *val, label, true) {
                            *val = !*val;
                            changed = true;
                        }
                    }

                    if changed {
                        self.save_column_settings();
                    }

                    ui.add_space(theme::SPACING_MD);
                    ui.separator();
                    ui.add_space(theme::SPACING_SM);

                    // — Notifications —
                    ui.add(egui::Label::new(
                        egui::RichText::new("Notifications").size(13.0).color(theme::ACCENT),
                    ).selectable(false));
                    ui.add_space(2.0);
                    if theme::settings_checkbox(ui, self.play_sound_on_complete, "Play sound on download complete", true) {
                        self.play_sound_on_complete = !self.play_sound_on_complete;
                    }

                    ui.add_space(theme::SPACING_MD);
                    ui.separator();
                    ui.add_space(theme::SPACING_SM);

                    // — Download Path —
                    ui.add(egui::Label::new(
                        egui::RichText::new("Download Path").size(13.0).color(theme::ACCENT),
                    ).selectable(false));
                    ui.add_space(2.0);

                    let path_changed = ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        let browse_width = 28.0 + 4.0; // button + spacing
                        let frame_padding = 12.0 + 2.0; // inner_margin (6*2) + stroke (1*2)
                        let text_width = (ui.available_width() - browse_width - frame_padding).max(40.0);
                        // Text input styled like search box
                        let te = egui::Frame::new()
                            .fill(theme::BG_INPUT)
                            .stroke(egui::Stroke::new(1.0, theme::BORDER_SUBTLE))
                            .corner_radius(4.0)
                            .inner_margin(egui::Margin::symmetric(6, 4))
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.download_path_str)
                                        .frame(false)
                                        .desired_width(text_width)
                                        .font(egui::FontId::proportional(13.0)),
                                )
                            }).inner;
                        // Browse button (aligned to text input height)
                        let (rect, resp) = ui.allocate_exact_size(
                            egui::vec2(28.0, 28.0), egui::Sense::click(),
                        );
                        if resp.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            ui.painter().rect_filled(rect, 4.0, theme::BG_SURFACE);
                        }
                        ui.painter().text(
                            rect.center(), egui::Align2::CENTER_CENTER,
                            egui_phosphor::regular::FOLDER_OPEN,
                            egui::FontId::proportional(16.0), theme::TEXT_SECONDARY,
                        );
                        let open_browser = resp.clicked() || te.double_clicked();
                        if open_browser {
                            std::fs::create_dir_all(&self.download_path).ok();
                            if let Some(path) = rfd::FileDialog::new()
                                .set_directory(&self.download_path)
                                .pick_folder()
                            {
                                self.download_path = path;
                                self.download_path_str = self.download_path.to_string_lossy().to_string();
                                self.save_settings();
                            }
                        }
                        te.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    }).inner;

                    if path_changed {
                        self.download_path = PathBuf::from(&self.download_path_str);
                        self.save_settings();
                    }

                    ui.add_space(4.0);
                    // Open Folder button
                    let base = theme::BTN_DEFAULT;
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(120.0, 26.0), egui::Sense::click(),
                    );
                    if response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    let (fill, draw_rect) = theme::button_visual(&response, base, rect);
                    ui.painter().rect_filled(draw_rect, 4.0, fill);
                    ui.painter().text(
                        draw_rect.center(), egui::Align2::CENTER_CENTER,
                        &format!("{}  Open Folder", egui_phosphor::regular::FOLDER_OPEN), egui::FontId::proportional(12.0),
                        egui::Color32::WHITE,
                    );
                    if response.clicked() {
                        std::fs::create_dir_all(&self.download_path).ok();
                        let _ = open::that(&self.download_path);
                    }

                    ui.add_space(theme::SPACING_MD);
                    ui.separator();
                    ui.add_space(theme::SPACING_SM);

                    // — Cache —
                    ui.add(egui::Label::new(
                        egui::RichText::new("Cache").size(13.0).color(theme::ACCENT),
                    ).selectable(false));
                    ui.add_space(2.0);
                    let base = theme::BTN_DANGER;
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(120.0, 26.0), egui::Sense::click(),
                    );
                    if response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    let (fill, draw_rect) = theme::button_visual(&response, base, rect);
                    ui.painter().rect_filled(draw_rect, 4.0, fill);
                    ui.painter().text(
                        draw_rect.center(), egui::Align2::CENTER_CENTER,
                        &format!("{}  Clear Cache", egui_phosphor::regular::TRASH), egui::FontId::proportional(12.0),
                        egui::Color32::WHITE,
                    );
                    if response.clicked() {
                        let _ = std::fs::remove_dir_all(self.cache_dir.join("thumbnails"));
                        let _ = std::fs::remove_dir_all(self.cache_dir.join("full"));
                        self.thumbnail_cache.clear();
                        self.preview_textures.clear();
                        self.start_thumbnail_prefetch(ui.ctx());
                    }
                });

            if modal_response.should_close() {
                self.show_settings = false;
            }
        }

        // Right panel for scroll index (jump markers) and scrollbar
        let index_panel_width = 44.0; // 20 for markers + 8 padding + 12 scrollbar + 4 padding
        egui::SidePanel::right("scroll_index_panel")
            .resizable(false)
            .exact_width(index_panel_width)
            .frame(egui::Frame::new().fill(theme::BG_BASE))
            .show(ctx, |ui| {
                let panel_rect = ui.available_rect_before_wrap();
                let total_rows = self.filtered_indices.len();

                // Get current row - use pending jump target if set (side panel renders before central panel updates memory)
                let current_row = self.scroll_target_row.unwrap_or_else(|| {
                    ui.ctx().memory(|mem| {
                        mem.data
                            .get_temp::<usize>("scroll_index_current_row".into())
                            .unwrap_or(0)
                    })
                });

                // Layout: [markers 20px] [padding 4px] [scrollbar 12px] [padding 4px]
                let markers_width = 20.0;
                let scrollbar_width = 12.0;
                let padding = 4.0;

                // Index markers on the left side of panel
                let index_rect = egui::Rect::from_min_max(
                    egui::pos2(panel_rect.min.x, panel_rect.min.y + theme::SPACING_MD),
                    egui::pos2(panel_rect.min.x + markers_width, panel_rect.max.y),
                );
                if let Some(target_row) =
                    self.render_scroll_index(ui, index_rect, total_rows, current_row)
                {
                    self.scroll_target_row = Some(target_row);
                }

                // Scrollbar on the right side of panel
                let scrollbar_rect = egui::Rect::from_min_max(
                    egui::pos2(
                        panel_rect.max.x - scrollbar_width - padding,
                        panel_rect.min.y,
                    ),
                    egui::pos2(panel_rect.max.x - padding, panel_rect.max.y),
                );

                // Only show scrollbar if content exceeds viewport
                if self.main_content_height > self.main_viewport_height
                    && self.main_viewport_height > 0.0
                {
                    let max_scroll =
                        (self.main_content_height - self.main_viewport_height).max(0.0);
                    let scroll_ratio = self.main_viewport_height / self.main_content_height;
                    let thumb_height = (scrollbar_rect.height() * scroll_ratio).max(20.0);
                    let track_height = scrollbar_rect.height() - thumb_height;
                    let thumb_offset = if max_scroll > 0.0 {
                        track_height * (self.main_scroll_offset / max_scroll)
                    } else {
                        0.0
                    };

                    // Draw track
                    ui.painter().rect_filled(
                        scrollbar_rect,
                        1.0,
                        theme::BORDER_SUBTLE,
                    );

                    // Draw thumb
                    let thumb_rect = egui::Rect::from_min_size(
                        egui::pos2(scrollbar_rect.min.x, scrollbar_rect.min.y + thumb_offset),
                        egui::vec2(scrollbar_width, thumb_height),
                    );

                    let thumb_response = ui.interact(
                        thumb_rect,
                        ui.id().with("scrollbar_thumb"),
                        egui::Sense::drag(),
                    );
                    let thumb_color = if thumb_response.dragged() || thumb_response.hovered() {
                        theme::TEXT_DIM
                    } else {
                        egui::Color32::from_rgb(0x52, 0x52, 0x56)
                    };
                    ui.painter().rect_filled(thumb_rect, 1.0, thumb_color);

                    // Handle drag
                    if thumb_response.dragged() {
                        let delta_y = thumb_response.drag_delta().y;
                        if track_height > 0.0 {
                            self.main_scroll_offset += delta_y * (max_scroll / track_height);
                            self.main_scroll_offset =
                                self.main_scroll_offset.clamp(0.0, max_scroll);
                        }
                    }

                    // Handle click on track
                    let track_response = ui.interact(
                        scrollbar_rect,
                        ui.id().with("scrollbar_track"),
                        egui::Sense::click(),
                    );
                    if track_response.clicked() {
                        if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                            let click_ratio =
                                (pos.y - scrollbar_rect.min.y) / scrollbar_rect.height();
                            self.main_scroll_offset = (click_ratio * self.main_content_height
                                - self.main_viewport_height / 2.0)
                                .clamp(0.0, max_scroll);
                        }
                    }
                }
            });

        // Central panel - map list (MUST be added LAST after all side/top/bottom panels)
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme::BG_BASE)
                    .inner_margin(egui::Margin::same(16)),
            )
            .show(ctx, |ui| {
                // Store panel rect for toast positioning
                self.central_panel_rect = Some(ui.max_rect());
                
                // Header bar with "Showing X of Y maps" and icons
                ui.horizontal(|ui| {
                    let status_text = format!(
                        "Showing {} of {} maps",
                        self.filtered_indices.len(),
                        self.maps.len()
                    );
                    let selected_count = self.selected_indices.len();
                    let full_text = if selected_count > 0 {
                        format!("{} • {} selected", status_text, selected_count)
                    } else {
                        status_text
                    };
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(full_text)
                                .color(theme::TEXT_DIM),
                        )
                        .selectable(false),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Settings gear
                        if ui
                            .add(egui::Button::new(egui_phosphor::regular::GEAR).frame(false))
                            .on_hover_text("Settings")
                            .clicked()
                        {
                            self.show_settings = !self.show_settings;
                        }

                        // View toggle (list/grid) - show icon for the view we'll switch TO
                        let view_icon = if self.compact_view {
                            egui_phosphor::regular::SQUARES_FOUR
                        } else {
                            egui_phosphor::regular::LIST
                        };
                        let view_tooltip = if self.compact_view {
                            "Switch to Grid view"
                        } else {
                            "Switch to List view"
                        };
                        if ui
                            .add(egui::Button::new(view_icon).frame(false))
                            .on_hover_text(view_tooltip)
                            .clicked()
                        {
                            // Capture top visible item index for scroll sync
                            let top_item = if self.compact_view {
                                // List view: item index from scroll offset using actual row height
                                // Add half row to land solidly in the current row, not the boundary
                                ((self.main_scroll_offset + self.list_row_height * 0.5) / self.list_row_height).floor() as usize
                            } else {
                                // Grid view: stored in memory
                                ui.ctx().memory(|mem| {
                                    mem.data
                                        .get_temp::<usize>("scroll_index_current_row".into())
                                        .unwrap_or(0)
                                })
                            };
                            self.scroll_sync_item = Some(top_item);
                            self.compact_view = !self.compact_view;
                            self.view_switch_count += 1;
                            self.save_column_settings();
                        }

                        // Open download folder
                        if ui
                            .add(
                                egui::Button::new(egui_phosphor::regular::FOLDER_OPEN).frame(false),
                            )
                            .on_hover_text("Open download folder")
                            .clicked()
                        {
                            let _ = open::that(&self.download_path);
                        }
                    });
                });

                ui.add_space(4.0);

                // Handle keyboard input - only when map list is focused
                let modifiers = ui.input(|i| i.modifiers);
                let mut nav_delta: i32 = 0;
                let mut select_all = false;
                let mut deselect_all = false;
                let mut download_shortcut = false;
                let mut preview_shortcut = false;

                ui.input(|i| {
                    if i.key_pressed(egui::Key::ArrowDown) {
                        nav_delta = 1;
                    } else if i.key_pressed(egui::Key::ArrowUp) {
                        nav_delta = -1;
                    }
                    if self.map_list_focused && i.modifiers.ctrl && i.key_pressed(egui::Key::A) {
                        select_all = true;
                    }
                    if i.key_pressed(egui::Key::Escape) {
                        deselect_all = true;
                    }
                    // Ctrl+D to download selected
                    if i.modifiers.ctrl
                        && i.key_pressed(egui::Key::D)
                        && !self.selected_indices.is_empty()
                    {
                        download_shortcut = true;
                    }
                    // Enter to open preview
                    if i.key_pressed(egui::Key::Enter) && !self.selected_indices.is_empty() {
                        preview_shortcut = true;
                    }
                });

                if deselect_all {
                    self.selected_indices.clear();
                    self.last_selected = None;
                }

                if select_all {
                    for &idx in &self.filtered_indices {
                        self.selected_indices.insert(idx);
                    }
                }

                if nav_delta != 0 && !self.filtered_indices.is_empty() {
                    let current_pos = self
                        .last_selected
                        .and_then(|sel| self.filtered_indices.iter().position(|&i| i == sel))
                        .unwrap_or(0);

                    let new_pos = (current_pos as i32 + nav_delta)
                        .max(0)
                        .min(self.filtered_indices.len() as i32 - 1)
                        as usize;

                    let new_idx = self.filtered_indices[new_pos];

                    if modifiers.shift {
                        self.selected_indices.insert(new_idx);
                    } else {
                        self.selected_indices.clear();
                        self.selected_indices.insert(new_idx);
                    }
                    self.last_selected = Some(new_idx);
                }

                // Handle keyboard shortcuts
                if download_shortcut {
                    self.download_selected(ctx);
                }
                if preview_shortcut {
                    let names: Vec<String> = self
                        .selected_indices
                        .iter()
                        .filter_map(|&idx| self.maps.get(idx).map(|m| m.name.clone()))
                        .collect();
                    if !names.is_empty() {
                        self.open_preview_multi(ctx, names);
                    }
                }

                if self.filtered_indices.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(ui.available_height() / 3.0);
                        ui.label(
                            egui::RichText::new(egui_phosphor::regular::FUNNEL_X)
                                .size(48.0)
                                .color(theme::TEXT_DIM),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new("No maps match your filters")
                                .size(16.0)
                                .color(theme::TEXT_MUTED),
                        );
                        ui.add_space(16.0);
                        if ui.add(theme::button(format!("{}  Clear Filters", egui_phosphor::regular::FUNNEL_X))).clicked() {
                            self.search_query.clear();
                            self.filter_categories = [true; 8];
                            self.category_mode_range = true;
                            self.category_range = (0, 4);
                            self.filter_stars = [true; 5];
                            self.stars_mode_range = true;
                            self.stars_range = (1, 5);
                            self.filter_downloaded = 0;
                            self.year_mode_range = true;
                            self.year_range = None;
                            self.filter_years = self.available_years.iter().copied().collect();
                            self.apply_filters();
                        }
                    });
                } else if self.compact_view {
                    let (preview, download) = self.render_list_view(ui, ctx);
                    if let Some(names) = preview {
                        self.open_preview_multi(ctx, names);
                    }
                    if download {
                        self.download_selected(ctx);
                    }
                } else {
                    self.render_grid_view(ui, ctx);
                }
            });

        // Render preview window if open
        self.render_preview_window(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        info!("Application shutting down");
        self.save_settings();
    }
}

// ============================================================================
// VIEW RENDERING (List, Grid, Scroll Index)
// ============================================================================

impl App {
    /// Render indexed scrollbar overlay and handle click-to-jump
    /// Returns row_index if a marker was clicked
    fn render_scroll_index(
        &mut self,
        ui: &mut egui::Ui,
        scroll_rect: egui::Rect,
        total_rows: usize,
        current_row: usize,
    ) -> Option<usize> {
        if self.scroll_index_markers.is_empty() || total_rows == 0 {
            return None;
        }

        let markers = &self.scroll_index_markers;
        let scrollbar_width = 14.0;
        let marker_height = 16.0;

        // Calculate scrollbar track area (right side of scroll_rect)
        let track_rect = egui::Rect::from_min_max(
            egui::pos2(scroll_rect.max.x - scrollbar_width, scroll_rect.min.y),
            scroll_rect.max,
        );

        // Available height for markers
        let track_height = track_rect.height();
        let total_marker_height = markers.len() as f32 * marker_height;

        // If markers would overflow, reduce spacing
        let actual_marker_height = if total_marker_height > track_height {
            (track_height / markers.len() as f32).max(10.0)
        } else {
            marker_height
        };

        // Calculate current section from scroll position
        let current_section = markers
            .iter()
            .enumerate()
            .rev()
            .find(|(_, m)| current_row >= m.row_index)
            .map(|(i, _)| i)
            .unwrap_or(0);

        let mut clicked_row: Option<usize> = None;
        let painter = ui.painter();

        // Draw markers
        for (i, marker) in markers.iter().enumerate() {
            let y_pos = track_rect.min.y + (i as f32 * actual_marker_height);
            let marker_rect = egui::Rect::from_min_size(
                egui::pos2(track_rect.min.x, y_pos),
                egui::vec2(scrollbar_width, actual_marker_height),
            );

            // Check if this marker is hovered/clicked
            let response = ui.interact(
                marker_rect,
                ui.id().with(("scroll_idx", i)),
                egui::Sense::click(),
            );

            let is_current = i == current_section;
            let is_hovered = response.hovered();

            // DEBUG: Log which marker is being highlighted
            // Background for current/hovered
            if is_current || is_hovered {
                let bg_color = if is_current {
                    theme::SELECTION_SCROLL_ACTIVE
                } else {
                    egui::Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, 30)
                };
                painter.rect_filled(marker_rect, 2.0, bg_color);
            }

            // Text color
            let text_color = if is_current {
                egui::Color32::WHITE
            } else if is_hovered {
                egui::Color32::from_rgb(0xcc, 0xcc, 0xcc)
            } else {
                egui::Color32::from_rgb(0x80, 0x80, 0x88)
            };

            // Draw label (centered)
            let font_size = if actual_marker_height < 14.0 {
                8.0
            } else {
                10.0
            };
            painter.text(
                marker_rect.center(),
                egui::Align2::CENTER_CENTER,
                &marker.label,
                egui::FontId::proportional(font_size),
                text_color,
            );

            // Handle click - return row_index for scrolling
            if response.clicked() {
                clicked_row = Some(marker.row_index);
            }
        }

        clicked_row
    }

    fn render_list_view(
        &mut self,
        ui: &mut egui::Ui,
        _ctx: &egui::Context,
    ) -> (Option<Vec<String>>, bool) {
        use egui_extras::{Column, TableBuilder};

        let mut preview_to_open: Option<Vec<String>> = None;
        let mut download_requested = false;

        let row_height = 29.0;
        let header_height = 42.0;
        let header_bg = theme::BG_ELEVATED;

        // Store rect for index positioning (will overlay scrollbar area)
        let full_rect = ui.available_rect_before_wrap();
        // Paint header background
        let header_rect = egui::Rect::from_min_size(
            egui::pos2(full_rect.min.x - 4.0, full_rect.min.y),
            egui::vec2(full_rect.width() + 56.0, header_height), // +56 to cover index/scrollbar panel
        );
        ui.painter().rect_filled(header_rect, 0.0, header_bg);

        // Capture modifiers before entering table closure
        let modifiers = ui.input(|i| i.modifiers);

        // Handle view sync - scroll to item index
        let sync_row = self.scroll_sync_item.take();
        if let Some(item_idx) = sync_row {
            self.main_scroll_offset = item_idx as f32 * row_height;
        }

        // Build columns - full width (index overlays scrollbar)
        let available_width = ui.available_width() - 40.0; // minus checkbox column
        let ctx = ui.ctx().clone();

        let mut table = TableBuilder::new(ui)
            .striped(false)
            .resizable(false)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .sense(egui::Sense::click())
            .min_scrolled_height(0.0)
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .vertical_scroll_offset(self.main_scroll_offset);

        // Apply scroll target if set (from index click or view sync)
        let scroll_to = self.scroll_target_row.take().or(sync_row);
        if let Some(target_row) = scroll_to {
            table = table.scroll_to_row(target_row, Some(egui::Align::TOP));
            if sync_row.is_some() {
                table = table.animate_scrolling(false);
            }
        }

        // Add checkbox column first (fixed width)
        table = table.column(Column::exact(40.0));

        // Calculate proportional widths based on visible columns
        let base_parts = 8.75; // Name(2.75) + Cat(1) + Stars(1) + Points(1) + Author(3)
        let total_parts = if self.show_release_date {
            base_parts + 1.5
        } else {
            base_parts
        };
        let part = available_width / total_parts;

        for &col_idx in &self.col_order.clone() {
            if !self.is_col_visible(col_idx) {
                continue;
            }
            let width = match col_idx {
                0 => part * 2.75, // Name
                1 => part * 1.0,  // Category
                2 => part * 1.0,  // Stars
                3 => part * 1.0,  // Points
                4 => part * 3.0,  // Author
                5 => part * 1.5,  // Release Date
                _ => part,
            };
            table = table.column(Column::exact(width).clip(true));
        }

        let visible_cols: Vec<usize> = self
            .col_order
            .iter()
            .filter(|&&idx| self.is_col_visible(idx))
            .copied()
            .collect();

        let scroll_output = table
            .header(header_height, |mut header| {
                let mut sort_changed = false;

                // Checkbox column header (empty)
                header.col(|_ui| {});

                for &col_idx in &visible_cols {
                    header.col(|ui| {
                        let col = match col_idx {
                            0 => Some(SortColumn::Name),
                            1 => Some(SortColumn::Category),
                            2 => Some(SortColumn::Stars),
                            3 => Some(SortColumn::Points),
                            4 => Some(SortColumn::Author),
                            5 => Some(SortColumn::ReleaseDate),
                            _ => None,
                        };

                        if let Some(col) = col {
                            let is_sorted = self.sort_column == Some(col);
                            let icon = if is_sorted {
                                match self.sort_direction {
                                    SortDirection::Ascending => egui_phosphor::regular::CARET_UP,
                                    SortDirection::Descending => egui_phosphor::regular::CARET_DOWN,
                                }
                            } else {
                                egui_phosphor::regular::CARET_UP_DOWN
                            };
                            let color = if is_sorted {
                                egui::Color32::WHITE
                            } else {
                                egui::Color32::from_rgb(0xa0, 0xa0, 0xa0)
                            };
                            let text = format!("{} {}", self.col_name(col_idx), icon);
                            let resp = ui.add(
                                egui::Label::new(
                                    egui::RichText::new(text).size(13.0).strong().color(color),
                                )
                                .selectable(false)
                                .sense(egui::Sense::click()),
                            );

                            if resp.clicked() {
                                if self.sort_column == Some(col) {
                                    match self.sort_direction {
                                        SortDirection::Ascending => {
                                            self.sort_direction = SortDirection::Descending
                                        }
                                        SortDirection::Descending => {
                                            self.sort_column = None;
                                        }
                                    }
                                } else {
                                    self.sort_column = Some(col);
                                    self.sort_direction = SortDirection::Ascending;
                                }
                                sort_changed = true;
                            }
                        } else {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(self.col_name(col_idx))
                                        .strong()
                                        .color(egui::Color32::WHITE),
                                )
                                .selectable(false),
                            );
                        }
                    });
                }

                if sort_changed {
                    self.apply_filters();
                }
            })
            .body(|mut body| {
                // Override selection color to teal for table rows only
                body.ui_mut().visuals_mut().selection.bg_fill = theme::TABLE_ROW_SELECTED;

                let indices = self.filtered_indices.clone();

                body.rows(row_height, indices.len(), |mut row| {
                    let row_idx = row.index();

                    let map_idx = indices[row_idx];
                    let map = &self.maps[map_idx];
                    let map_name = map.name.clone();
                    let is_selected = self.selected_indices.contains(&map_idx);

                    row.set_selected(is_selected);

                    // Checkbox column - use hover sense so row hover highlight works
                    row.col(|ui| {
                        ui.centered_and_justified(|ui| {
                            let cb_size = 16.0;
                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(cb_size, cb_size),
                                egui::Sense::hover(),
                            );

                            if is_selected {
                                ui.painter().rect_stroke(
                                    rect,
                                    3.0,
                                    egui::Stroke::new(1.5, theme::ACCENT),
                                    egui::StrokeKind::Inside,
                                );
                                let inner = rect.shrink(3.0);
                                ui.painter().rect_filled(inner, 2.0, theme::ACCENT);
                            } else {
                                ui.painter().rect_stroke(
                                    rect,
                                    3.0,
                                    egui::Stroke::new(1.5, theme::BORDER_DEFAULT),
                                    egui::StrokeKind::Inside,
                                );
                            }
                        });
                    });
                    for &col_idx in &visible_cols {
                        row.col(|ui| {
                            match col_idx {
                                0 => {
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(&map.name).strong().size(14.0),
                                        )
                                        .truncate()
                                        .selectable(false),
                                    );
                                }
                                1 => {
                                    // Category badge - fixed size for all categories
                                    let (bg, fg) = theme::category_colors(&map.category);
                                    let (rect, _response) = ui.allocate_exact_size(
                                        egui::vec2(62.0, 26.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().rect_filled(rect, 3.0, bg);
                                    ui.painter().text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        &map.category,
                                        egui::FontId::proportional(12.0),
                                        fg,
                                    );
                                }
                                2 => {
                                    // Stars with filled (gold) and empty (gray) colors
                                    let stars = map.stars.max(0).min(5) as usize;
                                    let filled = "★".repeat(stars);
                                    let empty = "☆".repeat(5 - stars);
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 0.0;
                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(&filled)
                                                    .size(12.0)
                                                    .color(theme::STAR_FILLED),
                                            )
                                            .selectable(false),
                                        );
                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(&empty)
                                                    .size(12.0)
                                                    .color(theme::STAR_EMPTY),
                                            )
                                            .selectable(false),
                                        );
                                    });
                                }
                                3 => {
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(map.points.to_string())
                                                .size(12.0)
                                                .color(theme::TEXT_DIM),
                                        )
                                        .selectable(false),
                                    );
                                }
                                4 => {
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(&map.author)
                                                .size(12.0)
                                                .color(theme::TEXT_DIM),
                                        )
                                        .truncate()
                                        .selectable(false),
                                    );
                                }
                                5 => {
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(format_release_date(
                                                &map.release_date,
                                            ))
                                            .size(12.0)
                                            .color(theme::TEXT_DIM),
                                        )
                                        .selectable(false),
                                    );
                                }
                                _ => {}
                            };
                        });
                    }

                    let response = row.response();

                    // Hand cursor on hover
                    if response.hovered() {
                        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                    }

                    // Right-click: select item if not already selected
                    if response.clicked_by(egui::PointerButton::Secondary) {
                        if !self.selected_indices.contains(&map_idx) {
                            self.selected_indices.insert(map_idx);
                            self.last_selected = Some(map_idx);
                        }
                    }

                    // Left click for selection and double-click detection
                    // Double-click to preview (only if both clicks were on this same item)
                    let is_valid_double_click =
                        response.double_clicked() && self.last_clicked_item == Some(map_idx);
                    if is_valid_double_click {
                        preview_to_open = Some(vec![map_name.clone()]);
                        // Ensure item is selected after preview
                        self.selected_indices.insert(map_idx);
                    }

                    if response.clicked_by(egui::PointerButton::Primary) {
                        self.map_list_focused = true;
                        self.last_clicked_item = Some(map_idx);

                        // Skip selection toggle on double-click
                        if !is_valid_double_click {
                            // Selection behavior
                            if modifiers.shift && self.last_selected.is_some() {
                                // Shift-click: range selection
                                let last = self.last_selected.unwrap();
                                let start = last.min(map_idx);
                                let end = last.max(map_idx);
                                for i in start..=end {
                                    if indices.contains(&i) {
                                        self.selected_indices.insert(i);
                                    }
                                }
                            } else {
                                // Normal click: toggle selection
                                if self.selected_indices.contains(&map_idx) {
                                    self.selected_indices.remove(&map_idx);
                                } else {
                                    self.selected_indices.insert(map_idx);
                                }
                            }

                            self.last_selected = Some(map_idx);
                        }
                    }

                    // Context menu
                    response.context_menu(|ui| {
                        let action = self.map_context_menu(ui, map_idx, &map_name);
                        if let Some(names) = action.preview { preview_to_open = Some(names); }
                        if action.download { download_requested = true; }
                    });
                });
            });

        // Update shared scroll state from table's scroll area
        let new_offset = scroll_output.state.offset.y;
        self.main_scroll_offset = new_offset;
        self.main_viewport_height = scroll_output.inner_rect.height();
        self.main_content_height = scroll_output.content_size.y;

        // Calculate current row from scroll offset using ACTUAL row height from content
        // Add 1 pixel to offset to ensure we land IN the section at boundaries
        let total_rows = self.filtered_indices.len();
        let actual_row_height = if total_rows > 0 {
            scroll_output.content_size.y / total_rows as f32
        } else {
            row_height
        };
        self.list_row_height = actual_row_height;
        let current_row =
            ((scroll_output.state.offset.y + 5.0) / actual_row_height).floor() as usize;

        ui.ctx().memory_mut(|mem| {
            mem.data
                .insert_temp("scroll_index_current_row".into(), current_row)
        });

        (preview_to_open, download_requested)
    }

    fn render_grid_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let spacing = theme::SPACING_MD;
        let (base_w, base_h) = if self.large_thumbnails {
            theme::CARD_LARGE
        } else {
            theme::CARD_SMALL
        };
        let available = ui.available_width();
        let num_cols = ((available + spacing) / (base_w + spacing)).floor().max(3.0);
        let card_w = ((available - spacing * (num_cols - 1.0)) / num_cols).floor();
        let card_h = (base_h * (card_w / base_w)).floor();

        let mut preview_to_open: Option<Vec<String>> = None;
        let mut download_requested = false;

        // Capture modifiers before closures
        let modifiers = ui.input(|i| i.modifiers);

        // Store full rect for index positioning
        let full_rect = ui.available_rect_before_wrap();

        // Calculate scroll offset if jumping to a row
        let available_width = ui.available_width();
        let cards_per_row = ((available_width + theme::SPACING_MD) / (card_w + theme::SPACING_MD))
            .floor()
            .max(1.0) as usize;

        // Handle view sync - calculate offset from item index
        if let Some(item_idx) = self.scroll_sync_item.take() {
            let target_visual_row = item_idx / cards_per_row;
            self.main_scroll_offset = target_visual_row as f32 * (card_h + theme::SPACING_MD);
            // Force scroll area state so it picks up the new offset
            let scroll_id = ui.make_persistent_id("grid_scroll");
            let mut state = egui::scroll_area::State::default();
            state.offset.y = self.main_scroll_offset;
            ui.ctx().memory_mut(|mem| {
                mem.data.insert_persisted(scroll_id, state);
            });
        }

        // Handle scroll target from marker click
        if let Some(target_row) = self.scroll_target_row.take() {
            let target_visual_row = target_row / cards_per_row;
            self.grid_scroll_target = Some(target_visual_row as f32 * (card_h + theme::SPACING_MD));
        }

        // Animate scroll toward target with easing (exponential decay, ~0.2s feel)
        if let Some(target) = self.grid_scroll_target {
            let diff = target - self.main_scroll_offset;
            if diff.abs() < 0.5 {
                self.main_scroll_offset = target;
                self.grid_scroll_target = None;
            } else {
                let dt = ctx.input(|i| i.stable_dt).min(0.1);
                let t = 1.0 - (-10.0 * dt).exp();
                self.main_scroll_offset += diff * t;
                ctx.request_repaint();
            }
        }

        // Use shared scroll offset, hide scrollbar (it's in side panel)
        let scroll_area = egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .id_salt("grid_scroll")
            .vertical_scroll_offset(self.main_scroll_offset);

        let scroll_response = scroll_area.show(ui, |ui| {
            let mut any_card_clicked = false;

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(theme::SPACING_MD, theme::SPACING_MD);
                let indices = self.filtered_indices.clone();
                for &map_idx in &indices {
                    // Clone map data to avoid borrow issues
                    let map = self.maps[map_idx].clone();
                    let map_name = map.name.clone();
                    let is_selected = self.selected_indices.contains(&map_idx);

                    let (rect, response) =
                        ui.allocate_exact_size(egui::vec2(card_w, card_h), egui::Sense::click());

                    if ui.is_rect_visible(rect) {
                        let painter = ui.painter();

                        // Try to draw thumbnail as background
                        // Paint base background (covers corners behind sharp-cornered image)
                        painter.rect_filled(rect, theme::RADIUS_DEFAULT, theme::BG_BASE);

                        if let Some(tex) = self.load_thumbnail(ctx, &map_name) {
                            // Use a textured RectShape to clip the image to rounded corners
                            let uv = egui::Rect::from_min_max(
                                egui::pos2(0.0, 0.0),
                                egui::pos2(1.0, 1.0),
                            );
                            let brush = egui::epaint::Brush {
                                fill_texture_id: tex.id(),
                                uv,
                            };
                            let mut shape = egui::epaint::RectShape::filled(
                                rect,
                                egui::CornerRadius::same(theme::RADIUS_DEFAULT as u8),
                                egui::Color32::WHITE,
                            );
                            shape.brush = Some(std::sync::Arc::new(brush));
                            painter.add(shape);

                            // Dark overlay for text readability
                            painter.rect_filled(
                                rect,
                                theme::RADIUS_DEFAULT,
                                egui::Color32::from_rgba_unmultiplied(0, 0, 0, 160),
                            );
                        } else {
                            // Fallback solid background
                            painter.rect_filled(rect, theme::RADIUS_DEFAULT, theme::BG_ELEVATED);
                        }

                        // Selection/hover overlay (matching list view color #1b1829)
                        if is_selected {
                            painter.rect_filled(
                                rect,
                                theme::RADIUS_DEFAULT,
                                egui::Color32::from_rgba_unmultiplied(0x0f, 0x1a, 0x19, 140),
                            );
                        } else if response.hovered() {
                            painter.rect_filled(
                                rect,
                                4.0,
                                egui::Color32::from_rgba_unmultiplied(0x0f, 0x1a, 0x19, 100),
                            );
                        }

                        // Hand cursor on hover
                        if response.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }

                        let border_color = if is_selected {
                            egui::Color32::from_rgba_unmultiplied(0x2d, 0xd4, 0xbf, 140)
                        } else {
                            egui::Color32::from_rgb(0x3a, 0x35, 0x42)
                        };
                        painter.rect_stroke(
                            rect,
                            4.0,
                            egui::Stroke::new(1.0, border_color),
                            egui::StrokeKind::Outside,
                        );

                        let text_rect = rect.shrink(8.0);

                        // Name (top)
                        painter.text(
                            text_rect.left_top(),
                            egui::Align2::LEFT_TOP,
                            &map.name,
                            egui::FontId::proportional(13.0),
                            egui::Color32::WHITE,
                        );

                        // Category + Stars (middle)
                        let mut info_y = 18.0;
                        {
                            let mut parts = Vec::new();
                            if self.show_category { parts.push(map.category.clone()); }
                            if self.show_stars { parts.push(render_stars(map.stars)); }
                            if !parts.is_empty() {
                                painter.text(
                                    text_rect.left_top() + egui::vec2(0.0, info_y),
                                    egui::Align2::LEFT_TOP,
                                    parts.join(" • "),
                                    egui::FontId::proportional(10.0),
                                    egui::Color32::from_rgb(0xcc, 0xcc, 0xcc),
                                );
                                info_y += 14.0;
                            }
                        }

                        // Author (under category/stars, only for large thumbnails)
                        if self.show_author && self.large_thumbnails {
                            painter.text(
                                text_rect.left_top() + egui::vec2(0.0, info_y),
                                egui::Align2::LEFT_TOP,
                                &map.author,
                                egui::FontId::proportional(10.0),
                                egui::Color32::from_rgb(0x90, 0x90, 0x98),
                            );
                        }

                        // Points (bottom left)
                        if self.show_points {
                            painter.text(
                                text_rect.left_bottom(),
                                egui::Align2::LEFT_BOTTOM,
                                format!("{} pts", map.points),
                                egui::FontId::proportional(10.0),
                                theme::ACCENT_MUTED,
                            );
                        }

                        // Release date (bottom right, only if enabled)
                        if self.show_release_date {
                            painter.text(
                                text_rect.right_bottom(),
                                egui::Align2::RIGHT_BOTTOM,
                                format_release_date(&map.release_date),
                                egui::FontId::proportional(9.0),
                                theme::TEXT_DIM,
                            );
                        }
                    }

                    // Double-click to preview (only if both clicks were on same item)
                    let is_valid_double_click =
                        response.double_clicked() && self.last_clicked_item == Some(map_idx);
                    if is_valid_double_click {
                        preview_to_open = Some(vec![map_name.clone()]);
                        // Ensure item is selected after preview
                        self.selected_indices.insert(map_idx);
                    }

                    // Right-click: select item if not already selected
                    if response.clicked_by(egui::PointerButton::Secondary) {
                        any_card_clicked = true;
                        if !self.selected_indices.contains(&map_idx) {
                            self.selected_indices.insert(map_idx);
                            self.last_selected = Some(map_idx);
                        }
                    }

                    // Left click for selection
                    if response.clicked_by(egui::PointerButton::Primary) {
                        any_card_clicked = true;
                        self.map_list_focused = true;
                        self.last_clicked_item = Some(map_idx);

                        // Skip selection toggle on double-click
                        if !is_valid_double_click {
                            if modifiers.shift && self.last_selected.is_some() {
                                // Shift-click: range selection
                                let last = self.last_selected.unwrap();
                                let start = last.min(map_idx);
                                let end = last.max(map_idx);
                                for i in start..=end {
                                    if self.filtered_indices.contains(&i) {
                                        self.selected_indices.insert(i);
                                    }
                                }
                            } else {
                                // Normal click: toggle selection
                                if self.selected_indices.contains(&map_idx) {
                                    self.selected_indices.remove(&map_idx);
                                } else {
                                    self.selected_indices.insert(map_idx);
                                }
                            }

                            self.last_selected = Some(map_idx);
                        }
                    }

                    // Context menu
                    response.context_menu(|ui| {
                        let action = self.map_context_menu(ui, map_idx, &map_name);
                        if let Some(names) = action.preview { preview_to_open = Some(names); }
                        if action.download { download_requested = true; }
                    });
                }
            });

            any_card_clicked
        });

        // Open preview if requested
        if let Some(names) = preview_to_open {
            self.open_preview_multi(ctx, names);
        }

        // Download if requested
        if download_requested {
            self.download_selected(ctx);
        }

        // Left click on empty area to deselect (but not if preview window or download modal is open)
        if !scroll_response.inner && self.preview_maps.is_empty() && !self.show_download_modal {
            if ui.input(|i| i.pointer.button_clicked(egui::PointerButton::Primary)) {
                if scroll_response
                    .inner_rect
                    .contains(ui.input(|i| i.pointer.hover_pos().unwrap_or_default()))
                {
                    self.selected_indices.clear();
                    self.last_selected = None;
                }
            }
        }

        // Update shared scroll state from scroll area
        let new_offset = scroll_response.state.offset.y;
        self.main_scroll_offset = new_offset;
        self.main_viewport_height = scroll_response.inner_rect.height();
        self.main_content_height = scroll_response.content_size.y;

        // Store current row for scroll index panel
        let current_visual_row =
            (scroll_response.state.offset.y / (card_h + theme::SPACING_MD)).floor() as usize;
        let current_row = current_visual_row * cards_per_row;
        ctx.memory_mut(|mem| {
            mem.data
                .insert_temp("scroll_index_current_row".into(), current_row)
        });
    }

    fn poll_update_results(&mut self, ctx: &egui::Context) {
        // Check for app update available
        if self.app_update_available.is_none() {
            if let Some(version) =
                ctx.memory(|mem| mem.data.get_temp::<String>("app_update".into()))
            {
                ctx.memory_mut(|mem| {
                    mem.data.remove::<String>("app_update".into());
                });
                self.app_update_available = Some(version);
                self.app_update_body = ctx.memory(|mem| mem.data.get_temp::<String>("app_update_body".into()));
                ctx.memory_mut(|mem| {
                    mem.data.remove::<String>("app_update_body".into());
                });
                self.show_app_update_dialog = true;
            }
        }

        // Check for DB auto-update completion
        if let Some(result) = ctx.memory(|mem| mem.data.get_temp::<String>("db_auto_updated".into()))
        {
            ctx.memory_mut(|mem| mem.data.remove::<String>("db_auto_updated".into()));
            // Reload maps
            if let Ok(maps) = self.db.get_all_maps() {
                self.maps = maps;
                self.apply_filters();
            }
            // Parse result: comma-separated new map names
            let new_maps: Vec<&str> = result.split(',').filter(|s| !s.is_empty()).collect();
            let msg = if new_maps.is_empty() {
                "Database updated".to_string()
            } else if new_maps.len() == 1 {
                format!("Database updated: {}", new_maps[0])
            } else {
                format!("Database updated: {}", new_maps.join(", "))
            };
            ctx.memory_mut(|mem| mem.data.insert_temp("db_updated".into(), msg));
        }

        // Check for app update completion
        if let Some(version) =
            ctx.memory(|mem| mem.data.get_temp::<String>("app_update_done".into()))
        {
            self.update_in_progress = false;
            self.app_update_success = Some(version.clone());
            ctx.memory_mut(|mem| mem.data.remove::<String>("app_update_done".into()));
        }

        // Check for app update error
        if let Some(err) = ctx.memory(|mem| mem.data.get_temp::<String>("app_update_error".into()))
        {
            self.update_in_progress = false;
            self.app_update_error = Some(err);
            ctx.memory_mut(|mem| mem.data.remove::<String>("app_update_error".into()));
        }
    }

    fn render_update_dialogs(&mut self, ctx: &egui::Context) {
        // App update modal
        if self.show_app_update_dialog {
            if let Some(version) = &self.app_update_available.clone() {
                let body = self.app_update_body.clone();
                
                // Built-in Modal with backdrop, escape-to-close, click-outside handling
                let modal_area = egui::Modal::default_area(egui::Id::new("app_update_modal"))
                    .default_width(380.0 + theme::SPACING_XL * 2.0);
                let modal = egui::Modal::new(egui::Id::new("app_update_modal"))
                    .area(modal_area)
                    .backdrop_color(egui::Color32::from_black_alpha(180))
                    .frame(theme::modal_frame());
                let modal_response = modal.show(ctx, |ui| {
                    ui.set_min_width(380.0);
                    ui.set_max_width(380.0);

                    if let Some(new_ver) = &self.app_update_success.clone() {
                        // === Success state ===
                        ui.vertical_centered(|ui| {
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new(egui_phosphor::regular::CHECK_CIRCLE).size(36.0).color(theme::ACCENT));
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new(format!("Updated to v{}!", new_ver)).size(16.0).strong());
                            ui.add_space(4.0);
                            ui.label(egui::RichText::new("Please restart the application to use the new version.").color(theme::TEXT_MUTED));
                            ui.add_space(16.0);
                            let ok_btn = ui.add(theme::button_accent(format!("{}  OK", egui_phosphor::regular::CHECK)));
                            if ok_btn.clicked() {
                                self.show_app_update_dialog = false;
                                self.app_update_success = None;
                                self.app_update_available = None;
                                self.app_update_body = None;
                            }
                        });
                    } else {
                        // === Normal / Error / Downloading state ===
                        
                        // Version header
                        ui.vertical_centered(|ui| {
                            ui.add_space(4.0);
                            ui.label(egui::RichText::new(format!("v{}", version)).size(22.0).strong().color(theme::ACCENT));
                            ui.add_space(2.0);
                            ui.label(egui::RichText::new(format!("Current: v{}", APP_VERSION)).size(12.0).color(theme::TEXT_DIM));
                        });
                        
                        // Release notes
                        if let Some(notes) = &body {
                            if !notes.is_empty() {
                                ui.add_space(12.0);
                                ui.separator();
                                ui.add_space(6.0);
                                ui.vertical_centered(|ui| {
                                    ui.label(egui::RichText::new("Release Notes").strong().size(15.0));
                                });
                                ui.add_space(8.0);
                                egui::ScrollArea::vertical()
                                    .max_height(220.0)
                                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                                    .show(ui, |ui| {
                                        for line in notes.lines() {
                                            if let Some(heading) = line.strip_prefix("## ") {
                                                ui.add_space(6.0);
                                                ui.label(egui::RichText::new(heading).strong().size(14.0));
                                            } else if let Some(heading) = line.strip_prefix("# ") {
                                                ui.add_space(6.0);
                                                ui.label(egui::RichText::new(heading).strong().size(16.0));
                                            } else if line.starts_with("- ") {
                                                ui.label(format!("  •  {}", &line[2..]));
                                            } else if line.is_empty() {
                                                ui.add_space(2.0);
                                            } else {
                                                ui.label(line);
                                            }
                                        }
                                    });
                            }
                        }
                        
                        // Inline error
                        if let Some(err) = &self.app_update_error.clone() {
                            ui.add_space(10.0);
                            ui.scope(|ui| {
                                ui.style_mut().spacing.item_spacing.x = 0.0;
                                egui::Frame::new()
                                    .fill(egui::Color32::from_rgb(0x2d, 0x0a, 0x0a))
                                    .corner_radius(theme::RADIUS_DEFAULT)
                                    .inner_margin(egui::Margin::same(10))
                                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(0x7f, 0x1d, 0x1d)))
                                    .show(ui, |ui| {
                                        ui.set_min_width(ui.available_width());
                                        let text = format!("{}  {}", egui_phosphor::regular::WARNING, err);
                                        ui.add(egui::Label::new(egui::RichText::new(text).color(egui::Color32::from_rgb(0xfc, 0xa5, 0xa5))).wrap());
                                    });
                            });
                        }

                        ui.add_space(16.0);

                        // Button area
                        ui.horizontal(|ui| {
                            ui.set_min_height(28.0);
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if self.update_in_progress {
                                    ui.spinner();
                                    ui.label("Downloading update...");
                                } else {
                                    let update_label = if self.app_update_error.is_some() { "Retry" } else { "Update" };
                                    let update_btn = ui.add(theme::button_accent(format!("{}  {}", egui_phosphor::regular::DOWNLOAD_SIMPLE, update_label)));
                                    if update_btn.clicked() {
                                        self.perform_app_update(ctx);
                                        self.app_update_error = None;
                                    }
                                    ui.add_space(8.0);
                                    let skip_btn = ui.add(theme::button(format!("{}  Skip", egui_phosphor::regular::X)));
                                    if skip_btn.clicked() {
                                        self.show_app_update_dialog = false;
                                        self.app_update_error = None;
                                    }
                                }
                            });
                        });
                    }
                });
                if modal_response.should_close() && !self.update_in_progress {
                    self.show_app_update_dialog = false;
                    self.app_update_error = None;
                }
            }
        }

        // Check for DB update success - trigger toast
        if let Some(msg) = ctx.memory(|mem| mem.data.get_temp::<String>("db_updated".into())) {
            ctx.memory_mut(|mem| mem.data.remove::<String>("db_updated".into()));
            self.toast_message = Some(msg);
            self.toast_start = Some(std::time::Instant::now());
        }

        // Render toast notification (bottom-right of central panel, 3s visible then fade, pause on hover)
        if let (Some(msg), Some(panel_rect)) = (&self.toast_message.clone(), self.central_panel_rect) {
            let visible_duration = 3.0;
            let fade_duration = 0.5;
            let total_duration = visible_duration + fade_duration;
            let margin = 12.0;
            
            // Position at bottom-right of central panel
            let toast_pos = egui::pos2(panel_rect.right() - margin, panel_rect.bottom() - margin);
            
            let response = egui::Area::new(egui::Id::new("db_toast"))
                .fixed_pos(toast_pos)
                .pivot(egui::Align2::RIGHT_BOTTOM)
                .show(ctx, |ui| {
                    let elapsed = self.toast_start.map(|t| t.elapsed().as_secs_f32()).unwrap_or(0.0);
                    let alpha = if elapsed > visible_duration { 
                        (total_duration - elapsed) / fade_duration 
                    } else { 
                        1.0 
                    };
                    
                    egui::Frame::new()
                        .fill(egui::Color32::from_rgba_unmultiplied(0x1a, 0x1a, 0x1e, (230.0 * alpha) as u8))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(
                            theme::ACCENT.r(), theme::ACCENT.g(), theme::ACCENT.b(), (100.0 * alpha) as u8
                        )))
                        .corner_radius(6.0)
                        .inner_margin(egui::Margin::symmetric(16, 10))
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new(msg).color(
                                egui::Color32::from_rgba_unmultiplied(255, 255, 255, (255.0 * alpha) as u8)
                            ));
                        });
                });
            
            // Pause timer while hovering
            if response.response.hovered() {
                self.toast_start = Some(std::time::Instant::now());
            }
            
            let elapsed = self.toast_start.map(|t| t.elapsed().as_secs_f32()).unwrap_or(0.0);
            if elapsed >= total_duration {
                self.toast_message = None;
                self.toast_start = None;
            } else {
                ctx.request_repaint();
            }
        }
    }

    // ========================================================================
    // DOWNLOAD MODAL
    // ========================================================================

    fn render_download_modal(&mut self, ctx: &egui::Context) {
        if !self.show_download_modal {
            return;
        }

        let state = self.download_state.lock().unwrap();
        let total = state.total_queued;
        let completed = state.completed_count;
        let failed = state.failed_count;
        let skipped = state.skipped_count;
        let cancelled = state.cancelled_count;
        let total_bytes = state.total_bytes;
        let downloaded_bytes = state.downloaded_bytes;
        let pending = total.saturating_sub(completed + failed + skipped + cancelled);
        let is_downloading = state.active_count > 0
            || state
                .downloads
                .values()
                .any(|s| matches!(s, DownloadStatus::Pending));
        let download_order = state.download_order.clone();
        let downloads = state.downloads.clone();
        drop(state);

        // Play sound when downloads finish
        if self.was_downloading && !is_downloading && self.play_sound_on_complete {
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NO_WINDOW: u32 = 0x08000000;
                let _ = std::process::Command::new("powershell")
                    .args(["-c", "[System.Media.SystemSounds]::Asterisk.Play()"])
                    .creation_flags(CREATE_NO_WINDOW)
                    .spawn();
            }
        }
        self.was_downloading = is_downloading;

        // Calculate in-progress bytes from active downloads
        let in_progress_bytes: u64 = downloads
            .values()
            .filter_map(|s| {
                if let DownloadStatus::Downloading(dl, _) = s {
                    Some(*dl)
                } else {
                    None
                }
            })
            .sum();
        let current_downloaded = downloaded_bytes + in_progress_bytes;

        // Collect active downloads (currently downloading)
        let active_downloads: Vec<(usize, u64, u64)> = download_order
            .iter()
            .filter_map(|&idx| {
                if let Some(DownloadStatus::Downloading(dl, tot)) = downloads.get(&idx) {
                    Some((idx, *dl, *tot))
                } else {
                    None
                }
            })
            .collect();

        // Semi-transparent overlay - clickable to close when not downloading
        // Fixed position: compute top-left from constants to prevent any content-size jitter
        let modal_width = 400.0 + theme::SPACING_XL * 2.0;
        let modal_height = if self.show_download_log { 340.0 } else { 240.0 };
        let screen = ctx.screen_rect();
        let pos = egui::pos2(
            (screen.center().x - modal_width / 2.0).round(),
            (screen.center().y - modal_height / 2.0).round(),
        );
        let modal_area = egui::Area::new(egui::Id::new("download_modal"))
            .kind(egui::UiKind::Modal)
            .sense(egui::Sense::hover())
            .fixed_pos(pos)
            .default_width(modal_width)
            .order(egui::Order::Foreground)
            .interactable(true);
        let modal = egui::Modal::new(egui::Id::new("download_modal"))
            .area(modal_area)
            .backdrop_color(egui::Color32::from_black_alpha(180))
            .frame(theme::modal_frame());
        let modal_response = modal.show(ctx, |ui| {
            ui.set_min_width(400.0);
            ui.set_max_width(400.0);
                // Batch Summary Header
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.colored_label(theme::ACCENT, egui_phosphor::regular::DOWNLOAD_SIMPLE);
                    ui.strong(format!("Downloading {} maps", total));
                    if total_bytes > 0 {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!("{}/{}", format_bytes(current_downloaded), format_bytes(total_bytes)));
                        });
                    }
                });
                ui.add_space(4.0);

                // Fixed-height area for active downloads (4 slots)
                let row_height = 20.0;
                let slots = 4;
                let area_height = row_height * slots as f32 + ui.spacing().item_spacing.y * (slots - 1) as f32;
                ui.allocate_ui(egui::vec2(ui.available_width(), area_height), |ui| {
                if active_downloads.is_empty() {
                    if is_downloading {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Starting downloads...");
                        });
                    }
                }

                let pct_width = 32.0;
                let name_width = 140.0;
                let spacing = ui.spacing().item_spacing.x;
                for (map_idx, downloaded, total_bytes) in &active_downloads {
                    let map_name = self
                        .maps
                        .get(*map_idx)
                        .map(|m| m.name.as_str())
                        .unwrap_or("Unknown");
                    let progress = if *total_bytes > 0 {
                        *downloaded as f32 / *total_bytes as f32
                    } else {
                        0.0
                    };

                    ui.horizontal(|ui| {
                        ui.set_height(row_height);
                        // Fixed-width name column
                        let (_, name_rect) = ui.allocate_space(egui::vec2(name_width, row_height));
                        let name_galley = ui.painter().layout_no_wrap(
                            map_name.to_string(),
                            egui::FontId::proportional(13.0),
                            egui::Color32::WHITE,
                        );
                        ui.painter().galley(
                            name_rect.left_center() - egui::vec2(0.0, name_galley.size().y / 2.0),
                            name_galley,
                            egui::Color32::WHITE,
                        );
                        // Progress bar fills remaining space minus percentage
                        let bar_width = ui.available_width() - pct_width - spacing;
                        let bar = egui::ProgressBar::new(progress)
                            .desired_width(bar_width)
                            .corner_radius(3.0)
                            .fill(theme::ACCENT);
                        ui.add(bar);
                        // Fixed-width percentage
                        ui.add_sized(
                            [pct_width, row_height],
                            egui::Label::new(egui::RichText::new(format!("{}%", (progress * 100.0) as u32))
                                .color(theme::TEXT_MUTED)
                                .size(12.0)),
                        );
                    });
                }
                // Pad remaining slots so height stays constant while downloading
                if is_downloading {
                    for _ in active_downloads.len()..slots {
                        ui.allocate_space(egui::vec2(ui.available_width(), row_height));
                    }
                }
                });

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(4.0);

                // Summary Stats Bar (clickable filters)
                ui.horizontal(|ui| {
                    let filter_btn = |ui: &mut egui::Ui,
                                      icon: &str,
                                      color: egui::Color32,
                                      count: usize,
                                      filter: &'static str,
                                      current: Option<&'static str>|
                     -> Option<&'static str> {
                        let selected = current == Some(filter);
                        let resp = ui.add(
                            egui::Label::new(
                                egui::RichText::new(format!("{} {}", icon, count)).color(
                                    if selected {
                                        egui::Color32::WHITE
                                    } else {
                                        color
                                    },
                                ),
                            )
                            .sense(egui::Sense::click()),
                        );
                        if resp.clicked() {
                            if selected {
                                None
                            } else {
                                Some(filter)
                            }
                        } else {
                            current
                        }
                    };

                    self.download_log_filter = filter_btn(
                        ui,
                        egui_phosphor::regular::CHECK,
                        egui::Color32::from_rgb(0x22, 0xc5, 0x5e),
                        completed,
                        "complete",
                        self.download_log_filter,
                    );
                    ui.add_space(8.0);

                    if skipped > 0 {
                        self.download_log_filter = filter_btn(
                            ui,
                            egui_phosphor::regular::FAST_FORWARD,
                            theme::TEXT_DIM,
                            skipped,
                            "skipped",
                            self.download_log_filter,
                        );
                        ui.add_space(8.0);
                    }

                    if failed > 0 {
                        self.download_log_filter = filter_btn(
                            ui,
                            egui_phosphor::regular::X_CIRCLE,
                            egui::Color32::from_rgb(0xef, 0x44, 0x44),
                            failed,
                            "failed",
                            self.download_log_filter,
                        );
                        ui.add_space(8.0);
                    }

                    if cancelled > 0 {
                        self.download_log_filter = filter_btn(
                            ui,
                            egui_phosphor::regular::X,
                            theme::TEXT_DIM,
                            cancelled,
                            "cancelled",
                            self.download_log_filter,
                        );
                        ui.add_space(8.0);
                    }

                    if pending > 0 {
                        ui.colored_label(
                            theme::TEXT_DIM,
                            egui_phosphor::regular::CLOCK,
                        );
                        ui.label(format!("{}", pending));
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!(
                            "{}/{}",
                            completed + skipped + failed,
                            total
                        ));
                    });
                });

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                // Collapsible Log Section
                let log_icon = if self.show_download_log {
                    egui_phosphor::regular::CARET_DOWN
                } else {
                    egui_phosphor::regular::CARET_RIGHT
                };

                if ui
                    .selectable_label(false, format!("{} Show Log", log_icon))
                    .clicked()
                {
                    self.show_download_log = !self.show_download_log;
                }

                if self.show_download_log {
                    ui.add_space(4.0);
                    egui::ScrollArea::vertical()
                        .max_height(100.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for &map_idx in &download_order {
                                let status = downloads.get(&map_idx);

                                // Apply filter
                                let show = match (status, self.download_log_filter) {
                                    (Some(DownloadStatus::Complete), None | Some("complete")) => {
                                        true
                                    }
                                    (Some(DownloadStatus::Skipped), None | Some("skipped")) => true,
                                    (Some(DownloadStatus::Failed(_)), None | Some("failed")) => {
                                        true
                                    }
                                    (Some(DownloadStatus::Cancelled), None | Some("cancelled")) => {
                                        true
                                    }
                                    _ => false,
                                };
                                if !show {
                                    continue;
                                }

                                let map_name = self
                                    .maps
                                    .get(map_idx)
                                    .map(|m| m.name.as_str())
                                    .unwrap_or("Unknown");
                                let (icon, color) = match status {
                                    Some(DownloadStatus::Complete) => (
                                        egui_phosphor::regular::CHECK,
                                        egui::Color32::from_rgb(0x22, 0xc5, 0x5e),
                                    ),
                                    Some(DownloadStatus::Skipped) => (
                                        egui_phosphor::regular::FAST_FORWARD,
                                        theme::TEXT_DIM,
                                    ),
                                    Some(DownloadStatus::Cancelled) => (
                                        egui_phosphor::regular::X,
                                        theme::TEXT_DIM,
                                    ),
                                    Some(DownloadStatus::Failed(_)) => (
                                        egui_phosphor::regular::X_CIRCLE,
                                        egui::Color32::from_rgb(0xef, 0x44, 0x44),
                                    ),
                                    _ => continue,
                                };

                                ui.horizontal(|ui| {
                                    ui.colored_label(color, icon);
                                    ui.label(map_name);
                                    if let Some(DownloadStatus::Failed(err)) = status {
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.colored_label(
                                                    theme::TEXT_DIM,
                                                    err,
                                                );
                                            },
                                        );
                                    }
                                });
                            }
                        });
                }

                ui.add_space(4.0);

                // Footer buttons
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if is_downloading {
                            if ui
                                .add(theme::button_danger(format!(
                                    "{} Cancel",
                                    egui_phosphor::regular::X
                                )))
                                .clicked()
                            {
                                if let Some(token) = &self.cancel_token {
                                    token.cancel();
                                }
                            }
                        } else {
                            if ui.add(theme::button(format!("{}  Close", egui_phosphor::regular::X))).clicked() {
                                self.close_download_modal();
                            }
                            if failed > 0 {
                                if ui
                                    .add(theme::button_accent(format!(
                                        "{} Retry Failed",
                                        egui_phosphor::regular::ARROW_CLOCKWISE
                                    )))
                                    .clicked()
                                {
                                    self.retry_failed_downloads(ctx);
                                }
                            }
                        }
                    });
                });
        });
        if modal_response.should_close() && !is_downloading {
            self.close_download_modal();
        }
    }

    fn close_download_modal(&mut self) {
        self.show_download_modal = false;
        self.show_download_log = false;
        self.download_log_filter = None;
        let mut state = self.download_state.lock().unwrap();
        state.downloads.clear();
        state.download_order.clear();
        state.total_queued = 0;
        state.completed_count = 0;
        state.failed_count = 0;
        state.skipped_count = 0;
        state.cancelled_count = 0;
        state.active_count = 0;
    }

    fn render_preview_window(&mut self, ctx: &egui::Context) {
        if self.preview_maps.is_empty() {
            return;
        }

        // Ensure active tab is valid
        if self.preview_active_tab >= self.preview_maps.len() {
            self.preview_active_tab = 0;
        }

        let current_map = self.preview_maps[self.preview_active_tab].clone();
        let mut close = false;
        let mut close_tab: Option<usize> = None;

        // Try to load preview if not loaded yet
        if !self.preview_textures.contains_key(&current_map) {
            let full_path = self
                .cache_dir
                .join("full")
                .join(format!("{}.png", current_map));
            if full_path.exists() {
                let tex = image::open(&full_path).ok().map(|img| {
                    let rgba = img.to_rgba8();
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    let pixels = rgba.into_raw();
                    ctx.load_texture(
                        format!("{}_full", current_map),
                        egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
                        egui::TextureOptions::LINEAR,
                    )
                });
                self.preview_textures.insert(current_map.clone(), tex);
                self.preview_loading.remove(&current_map);
            }
        }

        // Close on Escape
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            close = true;
        }

        let title = if self.preview_maps.len() == 1 {
            current_map.clone()
        } else {
            format!("{} maps", self.preview_maps.len())
        };

        // Window sizing - header(36) + toolbar(34) + margins
        let has_tabs = self.preview_maps.len() > 1;
        let chrome_height = if has_tabs {
            36.0 + 32.0 + 34.0 + 4.0
        } else {
            36.0 + 34.0 + 4.0
        }; // header + tabs? + toolbar + margins
        let target_img_height = 525.0;
        let target_img_width = target_img_height * (16.0 / 9.0);

        // Custom frame - smaller corner radius to minimize clipping issues
        let window_frame = egui::Frame::new()
            .fill(theme::BG_ELEVATED) // Match header color so corners blend
            .stroke(egui::Stroke::new(1.0, theme::BORDER_DEFAULT))
            .corner_radius(6.0)
            .inner_margin(egui::Margin {
                left: 2,
                right: 2,
                top: 0,
                bottom: 2,
            });

        // Dim backdrop behind preview - blocks interaction with main UI
        let screen = ctx.screen_rect();
        egui::Area::new(egui::Id::new("preview_dim"))
            .fixed_pos(screen.min)
            .order(egui::Order::Middle)
            .interactable(true)
            .show(ctx, |ui| {
                ui.allocate_response(screen.size(), egui::Sense::click());
                ui.painter().rect_filled(screen, 0.0, egui::Color32::from_black_alpha(120));
            });

        let default_w = target_img_width;
        let default_h = target_img_height + chrome_height;
        let win_resp = egui::Window::new("preview_window")
            .title_bar(false)
            .collapsible(false)
            .resizable(true)
            .frame(window_frame)
            .default_size([default_w, default_h])
            .default_pos([
                (screen.width() - default_w) / 2.0,
                (screen.height() - default_h) / 2.0,
            ])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let mut fit_requested = self.preview_needs_fit;

                // ═══════════════════════════════════════════════════════════
                // HEADER BAR (36px) - uses allocate_space to advance cursor
                // ═══════════════════════════════════════════════════════════
                let header_width = ui.available_width();
                let header_height = 36.0;
                let (header_rect, _) = ui.allocate_exact_size(
                    egui::vec2(header_width, header_height),
                    egui::Sense::hover(),
                );

                // Header background (no top rounding - frame provides the rounded corners)
                ui.painter()
                    .rect_filled(header_rect, 0.0, theme::BG_ELEVATED);

                // Header content (icon + title + close button)
                let icon_x = header_rect.left() + 12.0;
                let icon_center_y = header_rect.center().y;
                ui.painter().text(
                    egui::pos2(icon_x, icon_center_y),
                    egui::Align2::LEFT_CENTER,
                    egui_phosphor::regular::IMAGE,
                    egui::FontId::proportional(16.0),
                    theme::ACCENT,
                );

                let title_x = icon_x + 24.0;
                ui.painter().text(
                    egui::pos2(title_x, icon_center_y),
                    egui::Align2::LEFT_CENTER,
                    format!("Preview: {}", title),
                    egui::FontId::proportional(14.0),
                    theme::TEXT_PRIMARY,
                );

                // Close button
                let close_size = 24.0;
                let close_rect = egui::Rect::from_center_size(
                    egui::pos2(header_rect.right() - 20.0, icon_center_y),
                    egui::vec2(close_size, close_size),
                );
                let close_response = ui.interact(
                    close_rect,
                    ui.id().with("header_close"),
                    egui::Sense::click(),
                );
                let close_color = if close_response.hovered() {
                    theme::STATUS_ERROR
                } else {
                    theme::TEXT_DIM
                };
                if close_response.hovered() {
                    ui.painter().rect_filled(close_rect, 4.0, theme::BG_SURFACE);
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                ui.painter().text(
                    close_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    egui_phosphor::regular::X,
                    egui::FontId::proportional(16.0),
                    close_color,
                );
                if close_response.clicked() {
                    close = true;
                }

                // ═══════════════════════════════════════════════════════════
                // TAB BAR (32px) - only if multiple maps
                // ═══════════════════════════════════════════════════════════
                if has_tabs {
                    let tab_bar_width = ui.available_width();
                    let tab_bar_height = 32.0;
                    let (tab_bar_rect, _) = ui.allocate_exact_size(
                        egui::vec2(tab_bar_width, tab_bar_height),
                        egui::Sense::hover(),
                    );

                    ui.painter()
                        .rect_filled(tab_bar_rect, 0.0, theme::BG_ELEVATED);

                    ui.allocate_ui_at_rect(tab_bar_rect, |ui| {
                        ui.set_clip_rect(tab_bar_rect.shrink2(egui::vec2(8.0, 0.0)));
                        // Swap vertical scroll to horizontal when hovering tab bar
                        if ui.rect_contains_pointer(tab_bar_rect) {
                            ui.input_mut(|i| {
                                if i.smooth_scroll_delta.y != 0.0 {
                                    let y = i.smooth_scroll_delta.y;
                                    i.smooth_scroll_delta.y = 0.0;
                                    i.smooth_scroll_delta.x += y;
                                }
                            });
                        }
                        // Scrollable horizontal area for tabs (hidden scrollbar, wheel scrolls horizontally)
                        egui::ScrollArea::horizontal()
                            .max_width(tab_bar_rect.width() - 8.0)
                            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.add_space(8.0);
                                    ui.spacing_mut().item_spacing.x = 4.0;

                                    for (i, name) in self.preview_maps.clone().iter().enumerate() {
                                        let is_active = i == self.preview_active_tab;

                                        // Calculate tab width based on actual text width + close button
                                        let font_id = egui::FontId::proportional(12.0);
                                        let display_name: String = if name.len() > 22 {
                                            format!("{}…", &name[..21])
                                        } else {
                                            name.clone()
                                        };
                                        let text_width = ui.fonts(|f| {
                                            f.layout_no_wrap(display_name.clone(), font_id.clone(), theme::TEXT_PRIMARY)
                                                .rect.width()
                                        });
                                        let tab_width = text_width + 36.0; // 8px left pad + 24px close btn + 4px
                                        let tab_height = 26.0;

                                        let (tab_rect, tab_response) = ui.allocate_exact_size(
                                            egui::vec2(tab_width, tab_height),
                                            egui::Sense::click(),
                                        );

                                        // Tab background
                                        let tab_bg = if is_active {
                                            theme::ACCENT
                                        } else if tab_response.hovered() {
                                            theme::BG_SURFACE
                                        } else {
                                            theme::BG_BASE
                                        };

                                        ui.painter().rect_filled(tab_rect, 4.0, tab_bg);

                                        let text_color = if is_active {
                                            egui::Color32::from_rgb(0x04, 0x2f, 0x2e) // teal-950
                                        } else {
                                            theme::TEXT_SECONDARY
                                        };

                                        // Text area (leave room for close button)
                                        let text_rect = egui::Rect::from_min_max(
                                            tab_rect.min + egui::vec2(8.0, 0.0),
                                            tab_rect.max - egui::vec2(24.0, 0.0),
                                        );

                                        ui.painter().text(
                                            egui::pos2(text_rect.left(), tab_rect.center().y),
                                            egui::Align2::LEFT_CENTER,
                                            &display_name,
                                            egui::FontId::proportional(12.0),
                                            text_color,
                                        );

                                        // Close button on tab
                                        let close_size = 16.0;
                                        let close_rect = egui::Rect::from_center_size(
                                            egui::pos2(
                                                tab_rect.right() - 12.0,
                                                tab_rect.center().y,
                                            ),
                                            egui::vec2(close_size, close_size),
                                        );

                                        let close_hovered = ui.rect_contains_pointer(close_rect);
                                        let close_color = if close_hovered {
                                            if is_active {
                                                egui::Color32::from_rgb(0x02, 0x1a, 0x19)
                                            } else {
                                                theme::STATUS_ERROR
                                            }
                                        } else if is_active {
                                            egui::Color32::from_rgba_unmultiplied(0x04, 0x2f, 0x2e, 150)
                                        } else {
                                            theme::TEXT_DIM
                                        };

                                        ui.painter().text(
                                            close_rect.center(),
                                            egui::Align2::CENTER_CENTER,
                                            egui_phosphor::regular::X,
                                            egui::FontId::proportional(11.0),
                                            close_color,
                                        );

                                        // Handle clicks
                                        if tab_response.clicked() {
                                            let click_pos = tab_response
                                                .interact_pointer_pos()
                                                .unwrap_or(tab_rect.center());

                                            if close_rect.contains(click_pos) {
                                                close_tab = Some(i);
                                            } else if i != self.preview_active_tab {
                                                self.preview_active_tab = i;
                                                self.preview_zoom = 1.0;
                                                self.preview_offset = egui::Vec2::ZERO;
                                                self.preview_needs_fit = true;
                                            }
                                        }

                                        if tab_response.hovered() {
                                            ui.ctx()
                                                .set_cursor_icon(egui::CursorIcon::PointingHand);
                                        }
                                    }

                                    ui.add_space(8.0);
                                });
                            });
                    });
                }

                // ═══════════════════════════════════════════════════════════
                // TOOLBAR
                // ═══════════════════════════════════════════════════════════
                ui.horizontal(|ui| {
                    ui.add_space(12.0);

                    // Zoom out
                    let zoom_btn_size = egui::vec2(28.0, 28.0);

                    let (minus_rect, minus_resp) =
                        ui.allocate_exact_size(zoom_btn_size, egui::Sense::click());
                    let minus_bg = if minus_resp.hovered() {
                        theme::BG_SURFACE
                    } else {
                        theme::BG_ELEVATED
                    };
                    ui.painter().rect_filled(minus_rect, 4.0, minus_bg);
                    ui.painter().text(
                        minus_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        egui_phosphor::regular::MINUS,
                        egui::FontId::proportional(14.0),
                        theme::TEXT_PRIMARY,
                    );
                    if minus_resp.clicked() {
                        self.preview_zoom = (self.preview_zoom - 0.25).max(0.1);
                    }
                    if minus_resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }

                    // Zoom percentage (fixed width to prevent layout shift)
                    let pct_text = format!("{:.0}%", self.preview_zoom * 100.0);
                    let (pct_rect, _) =
                        ui.allocate_exact_size(egui::vec2(40.0, 28.0), egui::Sense::hover());
                    ui.painter().text(
                        pct_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        pct_text,
                        egui::FontId::proportional(12.0),
                        theme::TEXT_SECONDARY,
                    );

                    // Zoom in
                    let (plus_rect, plus_resp) =
                        ui.allocate_exact_size(zoom_btn_size, egui::Sense::click());
                    let plus_bg = if plus_resp.hovered() {
                        theme::BG_SURFACE
                    } else {
                        theme::BG_ELEVATED
                    };
                    ui.painter().rect_filled(plus_rect, 4.0, plus_bg);
                    ui.painter().text(
                        plus_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        egui_phosphor::regular::PLUS,
                        egui::FontId::proportional(14.0),
                        theme::TEXT_PRIMARY,
                    );
                    if plus_resp.clicked() {
                        self.preview_zoom = (self.preview_zoom + 0.25).min(5.0);
                    }
                    if plus_resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    // Fit button
                    let (fit_rect, fit_resp) =
                        ui.allocate_exact_size(zoom_btn_size, egui::Sense::click());
                    let fit_bg = if fit_resp.hovered() {
                        theme::BG_SURFACE
                    } else {
                        theme::BG_ELEVATED
                    };
                    ui.painter().rect_filled(fit_rect, 4.0, fit_bg);
                    ui.painter().text(
                        fit_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        egui_phosphor::regular::CORNERS_IN,
                        egui::FontId::proportional(14.0),
                        theme::TEXT_PRIMARY,
                    );
                    if fit_resp.clicked() {
                        fit_requested = true;
                        self.preview_offset = egui::Vec2::ZERO;
                    }
                    if fit_resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        egui::show_tooltip(
                            ui.ctx(),
                            ui.layer_id(),
                            egui::Id::new("fit_tooltip"),
                            |ui| {
                                ui.label("Fit to window");
                            },
                        );
                    }

                    ui.add_space(4.0);

                    // 100% button
                    let (full_rect, full_resp) =
                        ui.allocate_exact_size(zoom_btn_size, egui::Sense::click());
                    let full_bg = if full_resp.hovered() {
                        theme::BG_SURFACE
                    } else {
                        theme::BG_ELEVATED
                    };
                    ui.painter().rect_filled(full_rect, 4.0, full_bg);
                    ui.painter().text(
                        full_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "1:1",
                        egui::FontId::proportional(11.0),
                        theme::TEXT_PRIMARY,
                    );
                    if full_resp.clicked() {
                        self.preview_zoom = 1.0;
                        self.preview_offset = egui::Vec2::ZERO;
                    }
                    if full_resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        egui::show_tooltip(
                            ui.ctx(),
                            ui.layer_id(),
                            egui::Id::new("full_tooltip"),
                            |ui| {
                                ui.label("Actual size (100%)");
                            },
                        );
                    }
                });

                ui.add_space(4.0);

                // ═══════════════════════════════════════════════════════════
                // IMAGE AREA
                // ═══════════════════════════════════════════════════════════
                let available = ui.available_size();
                let (rect, response) =
                    ui.allocate_exact_size(available, egui::Sense::click_and_drag());

                // Dark background for image area
                ui.painter().rect_filled(rect, 0.0, theme::BG_BASE);

                let tex_opt = self
                    .preview_textures
                    .get(&current_map)
                    .and_then(|t| t.as_ref());

                if let Some(tex) = tex_opt {
                    if fit_requested {
                        let tex_size = tex.size_vec2();
                        let scale_x = rect.width() / tex_size.x;
                        let scale_y = rect.height() / tex_size.y;
                        self.preview_zoom = scale_x.min(scale_y);
                        self.preview_needs_fit = false;
                    }

                    let tex_size = tex.size_vec2();
                    let scaled_size = tex_size * self.preview_zoom;
                    let center = rect.center() + self.preview_offset;
                    let img_rect = egui::Rect::from_center_size(center, scaled_size);

                    ui.set_clip_rect(rect);
                    ui.painter().image(
                        tex.id(),
                        img_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );

                    if response.dragged() {
                        self.preview_offset += response.drag_delta();
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                    }

                    response.context_menu(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        let mut labels = vec![
                            format!("{}  Fit to Window", egui_phosphor::regular::CORNERS_IN),
                            format!("{}  Actual Size", egui_phosphor::regular::FRAME_CORNERS),
                        ];
                        if self.preview_maps.len() > 1 {
                            labels.push(format!("{}  Close Tab", egui_phosphor::regular::X));
                        }
                        let label_refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
                        theme::set_menu_width(ui, &label_refs);
                        if theme::menu_item(ui, egui_phosphor::regular::CORNERS_IN, "Fit to Window") {
                            self.preview_needs_fit = true;
                            self.preview_offset = egui::Vec2::ZERO;
                            ui.close_menu();
                        }
                        if theme::menu_item(ui, egui_phosphor::regular::FRAME_CORNERS, "Actual Size") {
                            self.preview_zoom = 1.0;
                            self.preview_offset = egui::Vec2::ZERO;
                            ui.close_menu();
                        }
                        if self.preview_maps.len() > 1 {
                            ui.separator();
                            if theme::menu_item(ui, egui_phosphor::regular::X, "Close Tab") {
                                close_tab = Some(self.preview_active_tab);
                                ui.close_menu();
                            }
                        }
                    });

                    let scroll = ui.input(|i| i.raw_scroll_delta.y);
                    if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
                        if scroll != 0.0 && rect.contains(hover_pos) {
                            let old_zoom = self.preview_zoom;
                            let zoom_factor = 1.0 + scroll * 0.001;
                            self.preview_zoom = (self.preview_zoom * zoom_factor).clamp(0.1, 5.0);
                            let cursor_rel = hover_pos - center;
                            let zoom_change = self.preview_zoom / old_zoom;
                            self.preview_offset += cursor_rel * (1.0 - zoom_change);
                        }
                    }
                } else {
                    let is_loading = self.preview_loading.contains(&current_map);
                    let msg = if is_loading {
                        "Loading preview..."
                    } else {
                        "Preview not available"
                    };
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        msg,
                        egui::FontId::proportional(14.0),
                        theme::TEXT_DIM,
                    );
                }
            });

        // Click outside preview to close
        if let Some(inner) = &win_resp {
            if ctx.input(|i| i.pointer.any_pressed()) {
                if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                    if !inner.response.rect.contains(pos) {
                        close = true;
                    }
                }
            }
        }

        // Handle tab close
        if let Some(tab_idx) = close_tab {
            let removed_name = self.preview_maps.remove(tab_idx);
            self.preview_textures.remove(&removed_name);
            self.preview_loading.remove(&removed_name);
            if self.preview_active_tab >= self.preview_maps.len() && self.preview_active_tab > 0 {
                self.preview_active_tab -= 1;
            }
            self.preview_zoom = 1.0;
            self.preview_offset = egui::Vec2::ZERO;
            self.preview_needs_fit = true;
        }

        if close {
            self.preview_maps.clear();
            self.preview_textures.clear();
            self.preview_loading.clear();
            self.preview_active_tab = 0;
        }
    }
}
