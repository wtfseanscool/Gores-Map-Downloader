//! Reusable UI components
//!
//! This module contains standalone UI components that can be used
//! throughout the application.

use crate::theme;
use eframe::egui;

/// Render a star rating display
pub fn render_stars(stars: i32) -> String {
    "★".repeat(stars as usize) + &"☆".repeat((5 - stars) as usize)
}

/// Format release date, returning "N/A" for invalid dates
pub fn format_release_date(date: &str) -> &str {
    if date.len() >= 4 && date.chars().take(4).all(|c| c.is_ascii_digit()) {
        date
    } else {
        "N/A"
    }
}

/// Custom checkbox widget with consistent styling
pub fn styled_checkbox(ui: &mut egui::Ui, selected: bool, size: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let rounding = 3.0;

        if selected {
            // Filled checkbox
            painter.rect_filled(rect, rounding, theme::ACCENT);
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                egui_phosphor::regular::CHECK,
                egui::FontId::proportional(size * 0.7),
                egui::Color32::WHITE,
            );
        } else {
            // Empty checkbox
            painter.rect_stroke(
                rect,
                rounding,
                egui::Stroke::new(1.5, theme::BORDER_DEFAULT),
                egui::StrokeKind::Inside,
            );
        }
    }

    response
}
