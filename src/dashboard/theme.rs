//! Dashboard theme and styling
//!
//! Dark gaming-inspired theme for the dashboard UI.

use egui::{Color32, FontFamily, FontId, Rounding, Stroke, Style, TextStyle, Visuals};

/// Gaming-inspired dark color palette
pub struct ThemeColors;

impl ThemeColors {
    // Background colors
    pub const BG_DARK: Color32 = Color32::from_rgb(18, 18, 24);
    pub const BG_MEDIUM: Color32 = Color32::from_rgb(28, 28, 36);
    pub const BG_LIGHT: Color32 = Color32::from_rgb(38, 38, 48);
    pub const BG_HOVER: Color32 = Color32::from_rgb(48, 48, 60);

    // Accent colors
    pub const ACCENT_PRIMARY: Color32 = Color32::from_rgb(88, 166, 255);
    pub const ACCENT_SECONDARY: Color32 = Color32::from_rgb(136, 87, 255);
    pub const ACCENT_SUCCESS: Color32 = Color32::from_rgb(46, 204, 113);
    pub const ACCENT_WARNING: Color32 = Color32::from_rgb(255, 193, 7);
    pub const ACCENT_ERROR: Color32 = Color32::from_rgb(231, 76, 60);

    // Text colors
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(240, 240, 245);
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(160, 160, 175);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(100, 100, 115);

    // Border colors
    pub const BORDER: Color32 = Color32::from_rgb(50, 50, 65);
    pub const BORDER_FOCUS: Color32 = Color32::from_rgb(88, 166, 255);

    // Status colors
    pub const STATUS_RUNNING: Color32 = Color32::from_rgb(46, 204, 113);
    pub const STATUS_STOPPED: Color32 = Color32::from_rgb(160, 160, 175);
    pub const STATUS_ERROR: Color32 = Color32::from_rgb(231, 76, 60);
}

/// Apply the gaming theme to egui
pub fn apply_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    // Configure visuals
    let mut visuals = Visuals::dark();

    // Window and panel backgrounds
    visuals.window_fill = ThemeColors::BG_MEDIUM;
    visuals.panel_fill = ThemeColors::BG_DARK;
    visuals.faint_bg_color = ThemeColors::BG_LIGHT;
    visuals.extreme_bg_color = ThemeColors::BG_DARK;

    // Widget colors
    visuals.widgets.noninteractive.bg_fill = ThemeColors::BG_MEDIUM;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, ThemeColors::TEXT_SECONDARY);
    visuals.widgets.noninteractive.rounding = Rounding::same(6.0);

    visuals.widgets.inactive.bg_fill = ThemeColors::BG_LIGHT;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, ThemeColors::TEXT_PRIMARY);
    visuals.widgets.inactive.rounding = Rounding::same(6.0);

    visuals.widgets.hovered.bg_fill = ThemeColors::BG_HOVER;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, ThemeColors::TEXT_PRIMARY);
    visuals.widgets.hovered.rounding = Rounding::same(6.0);

    visuals.widgets.active.bg_fill = ThemeColors::ACCENT_PRIMARY;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, ThemeColors::TEXT_PRIMARY);
    visuals.widgets.active.rounding = Rounding::same(6.0);

    visuals.widgets.open.bg_fill = ThemeColors::BG_HOVER;
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, ThemeColors::TEXT_PRIMARY);
    visuals.widgets.open.rounding = Rounding::same(6.0);

    // Selection and interaction
    visuals.selection.bg_fill = color_with_alpha(ThemeColors::ACCENT_PRIMARY, 77); // ~0.3 alpha
    visuals.selection.stroke = Stroke::new(1.0, ThemeColors::ACCENT_PRIMARY);

    // Hyperlinks
    visuals.hyperlink_color = ThemeColors::ACCENT_PRIMARY;

    // Window appearance
    visuals.window_rounding = Rounding::same(8.0);
    visuals.window_shadow.blur = 8.0;
    visuals.window_stroke = Stroke::new(1.0, ThemeColors::BORDER);

    // Popup and menu appearance
    visuals.popup_shadow.blur = 4.0;
    visuals.menu_rounding = Rounding::same(6.0);

    style.visuals = visuals;

    // Spacing
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(16.0);

    // Font sizes - larger for better readability
    style.text_styles = [
        (TextStyle::Small, FontId::new(13.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(16.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(15.0, FontFamily::Monospace)),
        (TextStyle::Button, FontId::new(16.0, FontFamily::Proportional)),
        (TextStyle::Heading, FontId::new(22.0, FontFamily::Proportional)),
    ]
    .into();

    // Apply style
    ctx.set_style(style);
}

/// Helper to create a color with modified alpha
pub fn color_with_alpha(color: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

/// Get a styled button for primary actions
#[allow(dead_code)]
pub fn primary_button_style() -> egui::style::WidgetVisuals {
    egui::style::WidgetVisuals {
        bg_fill: ThemeColors::ACCENT_PRIMARY,
        weak_bg_fill: color_with_alpha(ThemeColors::ACCENT_PRIMARY, 204), // ~0.8 alpha
        bg_stroke: Stroke::NONE,
        fg_stroke: Stroke::new(1.0, Color32::WHITE),
        rounding: Rounding::same(6.0),
        expansion: 0.0,
    }
}

/// Get a styled button for secondary actions
#[allow(dead_code)]
pub fn secondary_button_style() -> egui::style::WidgetVisuals {
    egui::style::WidgetVisuals {
        bg_fill: ThemeColors::BG_LIGHT,
        weak_bg_fill: ThemeColors::BG_MEDIUM,
        bg_stroke: Stroke::new(1.0, ThemeColors::BORDER),
        fg_stroke: Stroke::new(1.0, ThemeColors::TEXT_PRIMARY),
        rounding: Rounding::same(6.0),
        expansion: 0.0,
    }
}

/// Get a styled button for danger actions
#[allow(dead_code)]
pub fn danger_button_style() -> egui::style::WidgetVisuals {
    egui::style::WidgetVisuals {
        bg_fill: ThemeColors::ACCENT_ERROR,
        weak_bg_fill: color_with_alpha(ThemeColors::ACCENT_ERROR, 204), // ~0.8 alpha
        bg_stroke: Stroke::NONE,
        fg_stroke: Stroke::new(1.0, Color32::WHITE),
        rounding: Rounding::same(6.0),
        expansion: 0.0,
    }
}
