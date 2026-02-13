//! Centralized theme constants for Gores Map Downloader
//! All colors, sizes, and styling should reference these constants

use egui::Color32;

// =============================================================================
// COLORS - Backgrounds
// =============================================================================
pub const BG_BASE: Color32 = Color32::from_rgb(0x09, 0x09, 0x0b); // zinc-950
pub const BG_ELEVATED: Color32 = Color32::from_rgb(0x18, 0x18, 0x1b); // zinc-900
pub const BG_INPUT: Color32 = Color32::from_rgb(0x14, 0x14, 0x18); // input field background
pub const BG_SURFACE: Color32 = Color32::from_rgb(0x27, 0x27, 0x2a); // zinc-800
pub const BG_HOVER: Color32 = Color32::from_rgb(0x0f, 0x1a, 0x19); // subtle teal hover
pub const BG_HOVER_SUBTLE: Color32 = Color32::from_rgb(0x1f, 0x1f, 0x22); // subtle hover

// =============================================================================
// COLORS - Accent (Teal)
// =============================================================================
pub const ACCENT: Color32 = Color32::from_rgb(0x2d, 0xd4, 0xbf); // teal-400
pub const ACCENT_MUTED: Color32 = Color32::from_rgba_premultiplied(0x1F, 0x95, 0x86, 0xB3); // teal-400 70% alpha
pub const ACCENT_LIGHT: Color32 = Color32::from_rgb(0x5e, 0xea, 0xd4); // teal-300

// =============================================================================
// COLORS - Text
// =============================================================================
pub const TEXT_PRIMARY: Color32 = Color32::WHITE;
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0xe4, 0xe4, 0xe7); // zinc-200
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0xa1, 0xa1, 0xaa); // zinc-400
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x71, 0x71, 0x7a); // zinc-500

// =============================================================================
// COLORS - Borders
// =============================================================================
pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(0x27, 0x27, 0x2a); // zinc-800 - faint gray for outlines
pub const BORDER_DEFAULT: Color32 = Color32::from_rgb(0x3f, 0x3f, 0x46); // zinc-700
pub const BORDER_STRONG: Color32 = Color32::from_rgb(0x52, 0x52, 0x5b); // zinc-600

// =============================================================================
// COLORS - Selection
// =============================================================================
// Note: Selection color defined in apply_visuals() due to alpha
pub const TABLE_ROW_SELECTED: Color32 = Color32::from_rgb(0x0f, 0x1a, 0x19); // teal selection for table rows
pub const SELECTION_SCROLL_ACTIVE: Color32 = Color32::from_rgba_premultiplied(0x0e, 0x42, 0x3b, 80); // teal-400 @ 31%
// =============================================================================
// COLORS - Status
// =============================================================================
pub const STATUS_SUCCESS: Color32 = Color32::from_rgb(0x34, 0xd3, 0x99); // emerald-400
pub const STATUS_WARNING: Color32 = Color32::from_rgb(0xfb, 0xbf, 0x24); // amber-400
pub const STATUS_ERROR: Color32 = Color32::from_rgb(0xf8, 0x71, 0x71); // red-400

// =============================================================================
// COLORS - Stars
// =============================================================================
pub const STAR_FILLED: Color32 = Color32::from_rgb(0xfb, 0xbf, 0x24); // amber-400
pub const STAR_EMPTY: Color32 = Color32::from_rgb(0x4b, 0x4b, 0x5c);

// =============================================================================
// COLORS - Sliders
// =============================================================================
pub const SLIDER_HEAD: Color32 = Color32::from_rgb(0x2d, 0xd4, 0xbf); // teal-400
pub const SLIDER_TRAIL: Color32 = Color32::from_rgb(0x11, 0x5e, 0x59); // teal-800

// =============================================================================
// COLORS - Filter/Toggle Selection
// =============================================================================
pub const TOGGLE_SELECTED: Color32 = Color32::from_rgb(0x11, 0x5e, 0x59); // teal-800 - selected filter buttons
pub const TOGGLE_UNSELECTED: Color32 = Color32::from_rgb(0x27, 0x27, 0x2a); // zinc-800 - unselected filter buttons
pub const TOGGLE_GLOW: Color32 = Color32::from_rgb(0x0f, 0x76, 0x6e); // teal glow for segmented toggles

// =============================================================================
// COLORS - Buttons
// =============================================================================
// Default (gray) button
pub const BTN_DEFAULT: Color32 = Color32::from_rgb(0x3f, 0x3f, 0x46); // zinc-700
pub const BTN_DEFAULT_HOVER: Color32 = Color32::from_rgb(0x52, 0x52, 0x5b); // zinc-600
pub const BTN_DEFAULT_ACTIVE: Color32 = Color32::from_rgb(0x27, 0x27, 0x2a); // zinc-800

// Accent (teal) button
pub const BTN_ACCENT: Color32 = Color32::from_rgb(0x2d, 0xd4, 0xbf); // teal-400
pub const BTN_ACCENT_HOVER: Color32 = Color32::from_rgb(0x14, 0xb8, 0xa6); // teal-500
pub const BTN_ACCENT_ACTIVE: Color32 = Color32::from_rgb(0x0d, 0x94, 0x88); // teal-600

// Danger (red) button
pub const BTN_DANGER: Color32 = Color32::from_rgb(0xdc, 0x26, 0x26); // red-600
pub const BTN_DANGER_HOVER: Color32 = Color32::from_rgb(0xb9, 0x1c, 0x1c); // red-700

// Disabled state
pub const BTN_DISABLED: Color32 = Color32::from_rgb(0x27, 0x27, 0x2a); // zinc-800
pub const BTN_DISABLED_TEXT: Color32 = Color32::from_rgb(0x71, 0x71, 0x7a); // zinc-500

// =============================================================================
// COLORS - Categories
// =============================================================================
pub fn category_colors(category: &str) -> (Color32, Color32) {
    // Returns (bg_color ~6% alpha, text_color)
    match category {
        "Easy" => (
            Color32::from_rgba_unmultiplied(0x38, 0xbd, 0xf8, 10),
            Color32::from_rgb(0x38, 0xbd, 0xf8),
        ),
        "Main" => (
            Color32::from_rgba_unmultiplied(0x34, 0xd3, 0x99, 10),
            Color32::from_rgb(0x34, 0xd3, 0x99),
        ),
        "Hard" => (
            Color32::from_rgba_unmultiplied(0xfb, 0xbf, 0x24, 10),
            Color32::from_rgb(0xfb, 0xbf, 0x24),
        ),
        "Insane" => (
            Color32::from_rgba_unmultiplied(0xfb, 0x92, 0x3c, 10),
            Color32::from_rgb(0xfb, 0x92, 0x3c),
        ),
        "Extreme" => (
            Color32::from_rgba_unmultiplied(0xf8, 0x71, 0x71, 10),
            Color32::from_rgb(0xf8, 0x71, 0x71),
        ),
        "Solo" => (
            Color32::from_rgba_unmultiplied(0x22, 0xd3, 0xee, 10),
            Color32::from_rgb(0x22, 0xd3, 0xee),
        ),
        "Mod" => (
            Color32::from_rgba_unmultiplied(0xf4, 0x72, 0xb6, 10),
            Color32::from_rgb(0xf4, 0x72, 0xb6),
        ),
        _ => (
            Color32::from_rgba_unmultiplied(0xa1, 0xa1, 0xaa, 10),
            Color32::from_rgb(0xa1, 0xa1, 0xaa),
        ),
    }
}

// =============================================================================
// TYPOGRAPHY - Font Sizes
// =============================================================================
pub const FONT_TITLE: f32 = 18.0;
pub const FONT_HEADING: f32 = 16.0;
pub const FONT_BODY: f32 = 14.0;
pub const FONT_LABEL: f32 = 13.0;
pub const FONT_SECTION: f32 = 12.0;
pub const FONT_SMALL: f32 = 11.0;
pub const FONT_CAPTION: f32 = 10.0;

// =============================================================================
// DIMENSIONS - Layout
// =============================================================================
pub const SIDEBAR_WIDTH: f32 = 260.0;
pub const SETTINGS_PANEL_WIDTH: f32 = 250.0;
pub const ROW_HEIGHT: f32 = 36.0;
pub const BOTTOM_BAR_HEIGHT: f32 = 110.0;

// =============================================================================
// DIMENSIONS - Components
// =============================================================================
pub const TAB_HEIGHT: f32 = 28.0;
pub const TAB_MIN_WIDTH: f32 = 80.0;
pub const TAB_MAX_WIDTH: f32 = 200.0;
pub const CHECKBOX_SIZE: f32 = 18.0;
pub const LOGO_SIZE: f32 = 40.0;
pub const BADGE_WIDTH: f32 = 60.0;
pub const BADGE_HEIGHT: f32 = 22.0;
pub const BUTTON_HEIGHT: f32 = 28.0;
pub const BUTTON_HEIGHT_LARGE: f32 = 36.0;

// =============================================================================
// DIMENSIONS - Sliders
// =============================================================================
pub const SLIDER_HEIGHT: f32 = 18.0;
pub const SLIDER_HANDLE_RADIUS: f32 = 7.2;
pub const SLIDER_RAIL_HEIGHT: f32 = 4.0;

// =============================================================================
// DIMENSIONS - Grid Cards
// =============================================================================
pub const CARD_SMALL: (f32, f32) = (180.0, 80.0);
pub const CARD_LARGE: (f32, f32) = (360.0, 160.0);

// =============================================================================
// DIMENSIONS - Preview
// =============================================================================
pub const PREVIEW_IMG_HEIGHT: f32 = 600.0;
pub const PREVIEW_ASPECT_RATIO: f32 = 1920.0 / 1080.0;
pub const SCROLLBAR_WIDTH: f32 = 4.0;

// =============================================================================
// CORNER RADIUS
// =============================================================================
pub const RADIUS_SMALL: f32 = 2.0;
pub const RADIUS_DEFAULT: f32 = 4.0;
pub const RADIUS_MEDIUM: f32 = 6.0;
pub const RADIUS_LARGE: f32 = 8.0;
pub const RADIUS_LOGO: f32 = 12.0;

// =============================================================================
// STROKE WIDTHS
// =============================================================================
pub const STROKE_DEFAULT: f32 = 1.0;
pub const STROKE_MEDIUM: f32 = 1.5;
pub const STROKE_THICK: f32 = 2.0;

// =============================================================================
// SPACING
// =============================================================================
pub const SPACING_XS: f32 = 2.0;
pub const SPACING_SM: f32 = 4.0;
pub const SPACING_MD: f32 = 8.0;
pub const SPACING_LG: f32 = 12.0;
pub const SPACING_XL: f32 = 16.0;

// =============================================================================
// HELPER - Apply global visuals
// =============================================================================
pub fn apply_visuals(ctx: &egui::Context) {
    ctx.set_visuals(egui::Visuals {
        dark_mode: true,
        panel_fill: BG_BASE,
        window_fill: Color32::from_rgb(0x1a, 0x1a, 0x1e), // Slightly elevated for popups/menus
        extreme_bg_color: BG_BASE,
        faint_bg_color: BG_ELEVATED,
        hyperlink_color: ACCENT,
        selection: egui::style::Selection {
            bg_fill: Color32::from_rgb(0x3a, 0x3a, 0x3f), // Neutral gray selection (for text highlighting)
            stroke: egui::Stroke::NONE,
        },
        widgets: egui::style::Widgets {
            noninteractive: egui::style::WidgetVisuals {
                bg_fill: BG_ELEVATED,
                weak_bg_fill: BG_SURFACE,
                bg_stroke: egui::Stroke::new(STROKE_DEFAULT, BORDER_SUBTLE),
                fg_stroke: egui::Stroke::new(STROKE_DEFAULT, TEXT_PRIMARY),
                corner_radius: RADIUS_DEFAULT.into(),
                expansion: 0.0,
            },
            inactive: egui::style::WidgetVisuals {
                bg_fill: Color32::TRANSPARENT,
                weak_bg_fill: BG_ELEVATED,
                bg_stroke: egui::Stroke::new(STROKE_DEFAULT, BORDER_SUBTLE),
                fg_stroke: egui::Stroke::new(STROKE_DEFAULT, TEXT_SECONDARY),
                corner_radius: RADIUS_DEFAULT.into(),
                expansion: 0.0,
            },
            hovered: egui::style::WidgetVisuals {
                bg_fill: BG_HOVER,
                weak_bg_fill: Color32::from_rgb(0x30, 0x30, 0x35),
                bg_stroke: egui::Stroke::NONE,
                fg_stroke: egui::Stroke::new(STROKE_MEDIUM, TEXT_PRIMARY),
                corner_radius: RADIUS_DEFAULT.into(),
                expansion: 0.0,
            },
            active: egui::style::WidgetVisuals {
                bg_fill: Color32::from_rgb(0x2e, 0x2e, 0x33),
                weak_bg_fill: Color32::from_rgb(0x2e, 0x2e, 0x33),
                bg_stroke: egui::Stroke::NONE,
                fg_stroke: egui::Stroke::new(STROKE_DEFAULT, TEXT_PRIMARY),
                corner_radius: RADIUS_DEFAULT.into(),
                expansion: -2.0,
            },
            open: egui::style::WidgetVisuals {
                bg_fill: BG_SURFACE,
                weak_bg_fill: BG_ELEVATED,
                bg_stroke: egui::Stroke::new(STROKE_DEFAULT, BORDER_SUBTLE),
                fg_stroke: egui::Stroke::new(STROKE_DEFAULT, TEXT_PRIMARY),
                corner_radius: RADIUS_DEFAULT.into(),
                expansion: 0.0,
            },
        },
        striped: false,
        slider_trailing_fill: false,
        interact_cursor: Some(egui::CursorIcon::PointingHand),
        popup_shadow: egui::epaint::Shadow {
            offset: [0, 4],
            blur: 12,
            spread: 0,
            color: Color32::from_black_alpha(80),
        },
        window_stroke: egui::Stroke::new(1.0, Color32::from_rgb(0x2a, 0x2a, 0x2e)),
        window_corner_radius: egui::CornerRadius::same(8),
        menu_corner_radius: egui::CornerRadius::same(8),
        ..egui::Visuals::dark()
    });

    ctx.style_mut(|style| {
        style.interaction.selectable_labels = false;
        style.spacing.menu_margin = egui::Margin::symmetric(6, 4);
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(12.0, 6.0);
        style.spacing.scroll.bar_inner_margin = 2.0;
        style.spacing.scroll.bar_width = 6.0;
        style.spacing.scroll.bar_outer_margin = 2.0;
        style.spacing.scroll.handle_min_length = 20.0;
        style.spacing.scroll.floating_allocated_width = 0.0;
        style.spacing.scroll.floating = false;
    });
}

// =============================================================================
// HELPER - Card frame
// =============================================================================
pub fn card_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::from_rgba_unmultiplied(0x18, 0x18, 0x1b, 150))
        .stroke(egui::Stroke::new(STROKE_DEFAULT, BORDER_SUBTLE))
        .corner_radius(RADIUS_LARGE)
        .inner_margin(egui::Margin::same(SPACING_LG as i8))
}

// =============================================================================
// HELPER - Sidebar frame
// =============================================================================
pub fn sidebar_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(BG_BASE)
        .inner_margin(egui::Margin::same(0))
        .stroke(egui::Stroke::new(STROKE_DEFAULT, BORDER_SUBTLE))
}

// =============================================================================
// HELPER - Modal frame
// =============================================================================
pub fn modal_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::from_rgb(0x12, 0x12, 0x14))
        .stroke(egui::Stroke::new(STROKE_DEFAULT, BORDER_SUBTLE))
        .corner_radius(RADIUS_LARGE)
        .inner_margin(SPACING_XL)
}

// =============================================================================
// HELPER - Button styles
// =============================================================================

/// Default gray button
pub fn button(text: impl Into<String>) -> egui::Button<'static> {
    egui::Button::new(text.into())
        .fill(BTN_DEFAULT)
        .corner_radius(RADIUS_DEFAULT)
}

/// Accent teal button (for primary actions like Download)
pub fn button_accent(text: impl Into<String>) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(text.into()).color(Color32::from_rgb(0x04, 0x2f, 0x2e)))
        .fill(BTN_ACCENT)
        .corner_radius(RADIUS_DEFAULT)
}

/// Context menu item with icon. Returns true if clicked.
pub fn menu_item(ui: &mut egui::Ui, icon: &str, label: &str) -> bool {
    let text = format!("{}  {}", icon, label);
    let w = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(w, 24.0),
        egui::Sense::click(),
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        ui.painter().rect_filled(rect, RADIUS_DEFAULT, lighten(BG_SURFACE, 0.12));
    }
    let text_pos = rect.left_center() + egui::vec2(8.0, 0.0);
    ui.painter().text(
        text_pos,
        egui::Align2::LEFT_CENTER,
        text,
        egui::FontId::proportional(13.0),
        TEXT_SECONDARY,
    );
    response.clicked()
}

/// Sets context menu width to 1.5x the widest label.
pub fn set_menu_width(ui: &mut egui::Ui, labels: &[&str]) {
    let max_text = labels.iter().map(|l| {
        ui.fonts(|f| {
            f.layout_no_wrap(l.to_string(), egui::FontId::proportional(13.0), TEXT_SECONDARY)
                .rect.width()
        })
    }).fold(0.0_f32, f32::max);
    let w = (max_text + 16.0) * 1.5;
    ui.set_min_width(w);
    ui.set_max_width(w);
}

/// Settings checkbox row matching list view style. Returns true if toggled.
pub fn settings_checkbox(ui: &mut egui::Ui, checked: bool, label: &str, enabled: bool) -> bool {
    let full_width = ui.available_width();
    let row_height = 20.0;
    let (row_rect, row_resp) = ui.allocate_exact_size(
        egui::vec2(full_width, row_height),
        egui::Sense::click(),
    );
    if enabled && row_resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let painter = ui.painter();
    let cb_size = 16.0;
    let cb_rect = egui::Rect::from_min_size(
        egui::pos2(row_rect.min.x, row_rect.center().y - cb_size / 2.0),
        egui::vec2(cb_size, cb_size),
    );
    if checked {
        painter.rect_stroke(cb_rect, 3.0, egui::Stroke::new(1.5, ACCENT), egui::StrokeKind::Inside);
        painter.rect_filled(cb_rect.shrink(3.0), 2.0, ACCENT);
    } else {
        painter.rect_stroke(cb_rect, 3.0, egui::Stroke::new(1.5, BORDER_DEFAULT), egui::StrokeKind::Inside);
    }
    let color = if enabled { TEXT_PRIMARY } else { TEXT_DIM };
    painter.text(
        egui::pos2(cb_rect.max.x + 8.0, row_rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(14.0),
        color,
    );
    enabled && row_resp.clicked()
}

/// Danger red button (for destructive actions like Cancel)
pub fn button_danger(text: impl Into<String>) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(text.into()).color(TEXT_PRIMARY))
        .fill(BTN_DANGER)
        .corner_radius(RADIUS_DEFAULT)
}


/// Returns (fill, draw_rect) for a custom-painted button with hover/press effects.
/// Lightens on hover, slightly lightens + shrinks on press.
pub fn button_visual(
    response: &egui::Response,
    base_fill: Color32,
    rect: egui::Rect,
) -> (Color32, egui::Rect) {
    if response.is_pointer_button_down_on() {
        (lighten(base_fill, 0.06), rect.shrink(1.5))
    } else if response.hovered() {
        (lighten(base_fill, 0.12), rect)
    } else {
        (base_fill, rect)
    }
}

fn lighten(c: Color32, amount: f32) -> Color32 {
    let r = (c.r() as f32 + (255.0 - c.r() as f32) * amount) as u8;
    let g = (c.g() as f32 + (255.0 - c.g() as f32) * amount) as u8;
    let b = (c.b() as f32 + (255.0 - c.b() as f32) * amount) as u8;
    Color32::from_rgb(r, g, b)
}// =============================================================================
// HELPER - Section panel frame (with border)
// =============================================================================

/// Creates a section panel frame with fill and border
pub fn section_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::from_rgb(0x14, 0x14, 0x18))
        .stroke(egui::Stroke::new(STROKE_DEFAULT, BORDER_SUBTLE))
        .corner_radius(RADIUS_DEFAULT)
        .inner_margin(egui::Margin::same(12))
}

// =============================================================================
// HELPER - Segmented toggle (pill-style)
// =============================================================================

/// Renders a segmented toggle with two options. Returns true if selection changed.
/// `left_active` indicates if the left option is currently selected.
/// Style 1: Clean flat design with subtle inner border
pub fn segmented_toggle(
    ui: &mut egui::Ui,
    left_label: &str,
    right_label: &str,
    left_active: &mut bool,
) -> bool {
    segmented_toggle_style1(ui, left_label, right_label, left_active)
}

/// Style 1: Container (2px) -> Glow (1px) -> Active fill
pub fn segmented_toggle_style1(
    ui: &mut egui::Ui,
    left_label: &str,
    right_label: &str,
    left_active: &mut bool,
) -> bool {
    let mut changed = false;
    let height = 29.0;
    let font_size = 11.0;
    let rounding = 4.0;

    // Teal button sizes: left=52px, right=68px (to fit text with 12px margins)
    // Border: 2px container + 1px glow = 3px on outer edges, 1px glow + 1px gap = 2px on inner edges
    // Left segment: 3 (outer) + 52 (teal) + 2 (inner) = 57px
    // Right segment: 2 (inner) + 68 (teal) + 3 (outer) = 73px
    let left_width = 57.0;
    let right_width = 73.0;
    let total_width = left_width + right_width;

    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(total_width, height), egui::Sense::click());
    let painter = ui.painter();

    let container_color = TOGGLE_UNSELECTED;
    let active_color = TOGGLE_SELECTED;
    let glow_color = TOGGLE_GLOW;
    let inactive_text = TEXT_MUTED;

    // Layer 1: Container background
    painter.rect_filled(rect, rounding + 2.0, container_color);

    let left_rect =
        egui::Rect::from_min_max(rect.min, egui::pos2(rect.min.x + left_width, rect.max.y));
    let right_rect =
        egui::Rect::from_min_max(egui::pos2(rect.min.x + left_width, rect.min.y), rect.max);
    let active_rect = if *left_active { left_rect } else { right_rect };

    // Layer 2: Glow - 2px on outer edges, 1px on inner edge (between buttons), 2px top/bottom
    let glow_rect = if *left_active {
        egui::Rect::from_min_max(
            egui::pos2(active_rect.min.x + 2.0, active_rect.min.y + 2.0),
            egui::pos2(active_rect.max.x - 1.0, active_rect.max.y - 2.0),
        )
    } else {
        egui::Rect::from_min_max(
            egui::pos2(active_rect.min.x + 1.0, active_rect.min.y + 2.0),
            egui::pos2(active_rect.max.x - 2.0, active_rect.max.y - 2.0),
        )
    };
    painter.rect_filled(glow_rect, rounding, glow_color);

    // Layer 3: Active fill (inset 1px from glow - shows 1px of glow)
    let inner_rect = glow_rect.shrink(1.0);
    painter.rect_filled(inner_rect, rounding - 1.0, active_color);

    let (left_color, right_color) = if *left_active {
        (TEXT_PRIMARY, inactive_text)
    } else {
        (inactive_text, TEXT_PRIMARY)
    };

    // Calculate inner teal rects for BOTH buttons using same logic as drawing
    let left_glow_rect = egui::Rect::from_min_max(
        egui::pos2(left_rect.min.x + 2.0, left_rect.min.y + 2.0),
        egui::pos2(left_rect.max.x - 1.0, left_rect.max.y - 2.0),
    );
    let right_glow_rect = egui::Rect::from_min_max(
        egui::pos2(right_rect.min.x + 1.0, right_rect.min.y + 2.0),
        egui::pos2(right_rect.max.x - 2.0, right_rect.max.y - 2.0),
    );
    let left_inner = left_glow_rect.shrink(1.0);
    let right_inner = right_glow_rect.shrink(1.0);

    // Center text in the inner rects
    painter.text(
        left_inner.center(),
        egui::Align2::CENTER_CENTER,
        left_label,
        egui::FontId::proportional(font_size),
        left_color,
    );
    painter.text(
        right_inner.center(),
        egui::Align2::CENTER_CENTER,
        right_label,
        egui::FontId::proportional(font_size),
        right_color,
    );

    // Show hand cursor on hover
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let clicked_left = pos.x < rect.min.x + left_width;
            if clicked_left != *left_active {
                *left_active = clicked_left;
                changed = true;
            }
        }
    }
    changed
}

/// Style 2: Active fill with stroke border (no layered fills)
pub fn segmented_toggle_style2(
    ui: &mut egui::Ui,
    left_label: &str,
    right_label: &str,
    left_active: &mut bool,
) -> bool {
    let mut changed = false;
    let height = 24.0;
    let font_size = 12.0;
    let rounding = 5.0;

    let left_width = 54.0;
    let right_width = 72.0;
    let total_width = left_width + right_width;

    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(total_width, height), egui::Sense::click());
    let painter = ui.painter();

    let container_color = TOGGLE_UNSELECTED;
    let active_color = TOGGLE_SELECTED;
    let glow_color = TOGGLE_GLOW;
    let inactive_text = TEXT_MUTED;

    painter.rect_filled(rect, rounding, container_color);

    let left_rect =
        egui::Rect::from_min_max(rect.min, egui::pos2(rect.min.x + left_width, rect.max.y));
    let right_rect =
        egui::Rect::from_min_max(egui::pos2(rect.min.x + left_width, rect.min.y), rect.max);
    let active_rect = if *left_active { left_rect } else { right_rect };

    // Fill active segment
    painter.rect_filled(active_rect, rounding, active_color);
    // Add glow as inner stroke
    painter.rect_stroke(
        active_rect.shrink(0.5),
        rounding,
        egui::Stroke::new(2.0, glow_color),
        egui::StrokeKind::Inside,
    );

    let (left_color, right_color) = if *left_active {
        (TEXT_PRIMARY, inactive_text)
    } else {
        (inactive_text, TEXT_PRIMARY)
    };
    painter.text(
        left_rect.center(),
        egui::Align2::CENTER_CENTER,
        left_label,
        egui::FontId::proportional(font_size),
        left_color,
    );
    painter.text(
        right_rect.center(),
        egui::Align2::CENTER_CENTER,
        right_label,
        egui::FontId::proportional(font_size),
        right_color,
    );

    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let clicked_left = pos.x < rect.min.x + left_width;
            if clicked_left != *left_active {
                *left_active = clicked_left;
                changed = true;
            }
        }
    }
    changed
}

/// Style 3: Minimal 1px dark border, then glow, then active
pub fn segmented_toggle_style3(
    ui: &mut egui::Ui,
    left_label: &str,
    right_label: &str,
    left_active: &mut bool,
) -> bool {
    let mut changed = false;
    let height = 24.0;
    let font_size = 12.0;
    let rounding = 5.0;

    let left_width = 54.0;
    let right_width = 72.0;
    let total_width = left_width + right_width;

    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(total_width, height), egui::Sense::click());
    let painter = ui.painter();

    let container_color = TOGGLE_UNSELECTED;
    let active_color = TOGGLE_SELECTED;
    let glow_color = TOGGLE_GLOW;
    let inactive_text = TEXT_MUTED;

    painter.rect_filled(rect, rounding, container_color);

    let left_rect =
        egui::Rect::from_min_max(rect.min, egui::pos2(rect.min.x + left_width, rect.max.y));
    let right_rect =
        egui::Rect::from_min_max(egui::pos2(rect.min.x + left_width, rect.min.y), rect.max);
    let active_rect = if *left_active { left_rect } else { right_rect };

    // 1px dark border, 1px glow, rest is active
    let glow_rect = active_rect.shrink(1.0);
    painter.rect_filled(glow_rect, rounding, glow_color);
    let inner_rect = active_rect.shrink(2.0);
    painter.rect_filled(inner_rect, rounding, active_color);

    let (left_color, right_color) = if *left_active {
        (TEXT_PRIMARY, inactive_text)
    } else {
        (inactive_text, TEXT_PRIMARY)
    };
    painter.text(
        left_rect.center(),
        egui::Align2::CENTER_CENTER,
        left_label,
        egui::FontId::proportional(font_size),
        left_color,
    );
    painter.text(
        right_rect.center(),
        egui::Align2::CENTER_CENTER,
        right_label,
        egui::FontId::proportional(font_size),
        right_color,
    );

    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let clicked_left = pos.x < rect.min.x + left_width;
            if clicked_left != *left_active {
                *left_active = clicked_left;
                changed = true;
            }
        }
    }
    changed
}
