//! LinGlide Theme System
//!
//! Centralized design tokens for consistent visual design across the desktop app.

use egui::Stroke;

/// LinGlide color palette - dark theme optimized
#[allow(dead_code)]
pub mod colors {
    use egui::Color32;

    // Brand colors
    pub const PRIMARY: Color32 = Color32::from_rgb(59, 130, 246); // Blue-500
    pub const PRIMARY_HOVER: Color32 = Color32::from_rgb(37, 99, 235); // Blue-600
    pub const PRIMARY_LIGHT: Color32 = Color32::from_rgb(96, 165, 250); // Blue-400

    // Status colors
    pub const SUCCESS: Color32 = Color32::from_rgb(34, 197, 94); // Green-500
    pub const SUCCESS_DARK: Color32 = Color32::from_rgb(22, 163, 74); // Green-600
    pub const WARNING: Color32 = Color32::from_rgb(251, 191, 36); // Amber-400
    pub const ERROR: Color32 = Color32::from_rgb(239, 68, 68); // Red-500
    pub const ERROR_DARK: Color32 = Color32::from_rgb(220, 38, 38); // Red-600

    // Neutral colors (dark theme)
    pub const BG_PRIMARY: Color32 = Color32::from_rgb(17, 24, 39); // Gray-900
    pub const BG_SECONDARY: Color32 = Color32::from_rgb(31, 41, 55); // Gray-800
    pub const BG_TERTIARY: Color32 = Color32::from_rgb(55, 65, 81); // Gray-700
    pub const SURFACE: Color32 = Color32::from_rgb(75, 85, 99); // Gray-600

    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(249, 250, 251); // Gray-50
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(156, 163, 175); // Gray-400
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(107, 114, 128); // Gray-500

    pub const BORDER: Color32 = Color32::from_rgb(75, 85, 99); // Gray-600
    pub const BORDER_LIGHT: Color32 = Color32::from_rgb(55, 65, 81); // Gray-700

    /// Get a semi-transparent version of a color for backgrounds
    pub fn with_alpha(color: Color32, alpha: u8) -> Color32 {
        Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
    }
}

/// Spacing constants
pub mod spacing {
    use egui::Vec2;

    pub const ITEM: Vec2 = Vec2::new(10.0, 8.0);
    pub const SECTION: f32 = 16.0;
    pub const CARD_PADDING: f32 = 16.0;
    pub const CARD_MARGIN: f32 = 8.0;
    pub const BUTTON_PADDING: Vec2 = Vec2::new(16.0, 8.0);
}

/// Typography helpers
#[allow(dead_code)]
pub mod typography {
    use egui::FontId;

    pub fn heading() -> FontId {
        FontId::proportional(20.0)
    }

    pub fn subheading() -> FontId {
        FontId::proportional(16.0)
    }

    pub fn body() -> FontId {
        FontId::proportional(14.0)
    }

    pub fn caption() -> FontId {
        FontId::proportional(12.0)
    }

    pub fn mono() -> FontId {
        FontId::monospace(13.0)
    }

    pub fn mono_large() -> FontId {
        FontId::monospace(28.0)
    }
}

/// Rounding constants
#[allow(dead_code)]
pub mod rounding {
    use egui::Rounding;

    pub const NONE: Rounding = Rounding::ZERO;
    pub const SMALL: Rounding = Rounding::same(4.0);
    pub const MEDIUM: Rounding = Rounding::same(8.0);
    pub const LARGE: Rounding = Rounding::same(12.0);
    pub const FULL: Rounding = Rounding::same(999.0);
}

/// Apply LinGlide theme to egui context
pub fn apply_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    // Spacing
    style.spacing.item_spacing = spacing::ITEM;
    style.spacing.button_padding = spacing::BUTTON_PADDING;
    style.spacing.window_margin = egui::Margin::same(16.0);
    style.spacing.menu_margin = egui::Margin::same(8.0);

    // Visuals - window and panel backgrounds
    style.visuals.window_fill = colors::BG_PRIMARY;
    style.visuals.panel_fill = colors::BG_PRIMARY;
    style.visuals.window_rounding = rounding::MEDIUM;
    style.visuals.window_stroke = Stroke::new(1.0, colors::BORDER_LIGHT);

    // Extreme background (behind everything)
    style.visuals.extreme_bg_color = colors::BG_PRIMARY;
    style.visuals.faint_bg_color = colors::BG_SECONDARY;

    // Widgets - non-interactive (labels, etc.)
    style.visuals.widgets.noninteractive.bg_fill = colors::BG_SECONDARY;
    style.visuals.widgets.noninteractive.weak_bg_fill = colors::BG_TERTIARY;
    style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, colors::TEXT_SECONDARY);
    style.visuals.widgets.noninteractive.rounding = rounding::SMALL;
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::NONE;

    // Widgets - inactive (buttons at rest)
    style.visuals.widgets.inactive.bg_fill = colors::BG_TERTIARY;
    style.visuals.widgets.inactive.weak_bg_fill = colors::BG_SECONDARY;
    style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, colors::TEXT_PRIMARY);
    style.visuals.widgets.inactive.rounding = rounding::SMALL;
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, colors::BORDER_LIGHT);

    // Widgets - hovered
    style.visuals.widgets.hovered.bg_fill = colors::SURFACE;
    style.visuals.widgets.hovered.weak_bg_fill = colors::BG_TERTIARY;
    style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, colors::TEXT_PRIMARY);
    style.visuals.widgets.hovered.rounding = rounding::SMALL;
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, colors::PRIMARY);

    // Widgets - active (being clicked)
    style.visuals.widgets.active.bg_fill = colors::PRIMARY;
    style.visuals.widgets.active.weak_bg_fill = colors::PRIMARY_HOVER;
    style.visuals.widgets.active.fg_stroke = Stroke::new(1.0, colors::TEXT_PRIMARY);
    style.visuals.widgets.active.rounding = rounding::SMALL;
    style.visuals.widgets.active.bg_stroke = Stroke::new(1.0, colors::PRIMARY_LIGHT);

    // Widgets - open (dropdown menus, etc.)
    style.visuals.widgets.open.bg_fill = colors::BG_TERTIARY;
    style.visuals.widgets.open.weak_bg_fill = colors::BG_SECONDARY;
    style.visuals.widgets.open.fg_stroke = Stroke::new(1.0, colors::TEXT_PRIMARY);
    style.visuals.widgets.open.rounding = rounding::SMALL;
    style.visuals.widgets.open.bg_stroke = Stroke::new(1.0, colors::PRIMARY);

    // Selection colors
    style.visuals.selection.bg_fill = colors::with_alpha(colors::PRIMARY, 100);
    style.visuals.selection.stroke = Stroke::new(1.0, colors::PRIMARY);

    // Hyperlink color
    style.visuals.hyperlink_color = colors::PRIMARY_LIGHT;

    // Separator color
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, colors::BORDER_LIGHT);

    // Dark mode
    style.visuals.dark_mode = true;

    ctx.set_style(style);
}
