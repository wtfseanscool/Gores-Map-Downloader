//! Shared context menu for map items (used by both grid and list views)

use super::App;
use crate::theme;
use eframe::egui;

pub(crate) struct MapAction {
    pub preview: Option<Vec<String>>,
    pub download: bool,
}

impl App {
    pub(crate) fn map_context_menu(
        &mut self,
        ui: &mut egui::Ui,
        map_idx: usize,
        map_name: &str,
    ) -> MapAction {
        let mut action = MapAction { preview: None, download: false };
        ui.spacing_mut().item_spacing.y = 2.0;
        let selected_count = self.selected_indices.len();

        let labels: Vec<String> = if selected_count > 1 {
            vec![
                format!("{}  Preview {} maps", egui_phosphor::regular::EYE, selected_count),
                format!("{}  Download {} maps", egui_phosphor::regular::DOWNLOAD_SIMPLE, selected_count),
                format!("{}  Deselect All", egui_phosphor::regular::X_SQUARE),
            ]
        } else {
            vec![
                format!("{}  Preview", egui_phosphor::regular::EYE),
                format!("{}  Download", egui_phosphor::regular::DOWNLOAD_SIMPLE),
                format!("{}  Deselect All", egui_phosphor::regular::X_SQUARE),
            ]
        };
        let label_refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
        theme::set_menu_width(ui, &label_refs);

        if selected_count > 1 {
            if theme::menu_item(ui, egui_phosphor::regular::EYE, &format!("Preview {} maps", selected_count)) {
                let mut names: Vec<String> = self
                    .selected_indices
                    .iter()
                    .filter_map(|&i| self.maps.get(i).map(|m| m.name.clone()))
                    .collect();
                names.sort();
                if let Some(pos) = names.iter().position(|n| n == map_name) {
                    let clicked = names.remove(pos);
                    names.insert(0, clicked);
                }
                action.preview = Some(names);
                ui.close_menu();
            }
            if theme::menu_item(ui, egui_phosphor::regular::DOWNLOAD_SIMPLE, &format!("Download {} maps", selected_count)) {
                action.download = true;
                ui.close_menu();
            }
        } else {
            if theme::menu_item(ui, egui_phosphor::regular::EYE, "Preview") {
                action.preview = Some(vec![map_name.to_string()]);
                ui.close_menu();
            }
            if theme::menu_item(ui, egui_phosphor::regular::DOWNLOAD_SIMPLE, "Download") {
                self.selected_indices.clear();
                self.selected_indices.insert(map_idx);
                action.download = true;
                ui.close_menu();
            }
        }
        ui.separator();
        if theme::menu_item(ui, egui_phosphor::regular::X_SQUARE, "Deselect All") {
            self.selected_indices.clear();
            self.last_selected = None;
            ui.close_menu();
        }

        action
    }
}
