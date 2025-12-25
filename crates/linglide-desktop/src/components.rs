//! Reusable UI Components
//!
//! Consistent UI components for the LinGlide desktop application.

use crate::theme::{colors, rounding, spacing, typography};
use egui::{Response, RichText, Ui, Widget};

/// Status indicator types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Status {
    Success,
    Warning,
    Error,
    Inactive,
}

/// Status badge widget - colored pill showing status
pub struct StatusBadge<'a> {
    text: &'a str,
    status: Status,
}

#[allow(dead_code)]
impl<'a> StatusBadge<'a> {
    pub fn new(text: &'a str, status: Status) -> Self {
        Self { text, status }
    }

    pub fn success(text: &'a str) -> Self {
        Self::new(text, Status::Success)
    }

    pub fn warning(text: &'a str) -> Self {
        Self::new(text, Status::Warning)
    }

    pub fn error(text: &'a str) -> Self {
        Self::new(text, Status::Error)
    }

    pub fn inactive(text: &'a str) -> Self {
        Self::new(text, Status::Inactive)
    }
}

impl Widget for StatusBadge<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (bg_color, text_color) = match self.status {
            Status::Success => (colors::with_alpha(colors::SUCCESS, 40), colors::SUCCESS),
            Status::Warning => (colors::with_alpha(colors::WARNING, 40), colors::WARNING),
            Status::Error => (colors::with_alpha(colors::ERROR, 40), colors::ERROR),
            Status::Inactive => (colors::BG_TERTIARY, colors::TEXT_MUTED),
        };

        let text = RichText::new(self.text)
            .font(typography::caption())
            .color(text_color);

        egui::Frame::none()
            .fill(bg_color)
            .rounding(rounding::FULL)
            .inner_margin(egui::Margin::symmetric(10.0, 4.0))
            .show(ui, |ui| ui.label(text))
            .response
    }
}

/// Card container with optional title
pub fn card<R>(ui: &mut Ui, title: Option<&str>, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::none()
        .fill(colors::BG_SECONDARY)
        .rounding(rounding::MEDIUM)
        .inner_margin(egui::Margin::same(spacing::CARD_PADDING))
        .stroke(egui::Stroke::new(1.0, colors::BORDER_LIGHT))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if let Some(title) = title {
                ui.label(
                    RichText::new(title)
                        .font(typography::subheading())
                        .color(colors::TEXT_PRIMARY)
                        .strong(),
                );
                ui.add_space(8.0);
            }
            add_contents(ui)
        })
        .inner
}

/// Primary action button (blue, filled)
pub fn primary_button(ui: &mut Ui, text: &str) -> Response {
    let button = egui::Button::new(RichText::new(text).color(colors::TEXT_PRIMARY).strong())
        .fill(colors::PRIMARY)
        .rounding(rounding::SMALL);

    ui.add(button)
}

/// Secondary button (gray, outlined feel)
pub fn secondary_button(ui: &mut Ui, text: &str) -> Response {
    let button = egui::Button::new(RichText::new(text).color(colors::TEXT_PRIMARY))
        .fill(colors::BG_TERTIARY)
        .stroke(egui::Stroke::new(1.0, colors::BORDER))
        .rounding(rounding::SMALL);

    ui.add(button)
}

/// Danger button (red, for destructive actions)
pub fn danger_button(ui: &mut Ui, text: &str) -> Response {
    let button = egui::Button::new(RichText::new(text).color(colors::TEXT_PRIMARY).strong())
        .fill(colors::ERROR)
        .rounding(rounding::SMALL);

    ui.add(button)
}

/// Success button (green)
#[allow(dead_code)]
pub fn success_button(ui: &mut Ui, text: &str) -> Response {
    let button = egui::Button::new(RichText::new(text).color(colors::TEXT_PRIMARY).strong())
        .fill(colors::SUCCESS)
        .rounding(rounding::SMALL);

    ui.add(button)
}

/// Clickable link that opens URL in browser
pub fn link_button(ui: &mut Ui, text: &str, url: &str) -> Response {
    let response = ui.add(
        egui::Label::new(RichText::new(text).color(colors::PRIMARY_LIGHT).underline())
            .sense(egui::Sense::click()),
    );

    if response.clicked() {
        let _ = open::that(url);
    }

    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Small icon button (for actions like copy, close)
#[allow(dead_code)]
pub fn icon_button(ui: &mut Ui, icon: &str, tooltip: &str) -> Response {
    let response = ui.add(
        egui::Button::new(RichText::new(icon).size(16.0))
            .fill(egui::Color32::TRANSPARENT)
            .frame(false)
            .rounding(rounding::SMALL),
    );

    response.on_hover_text(tooltip)
}

/// Status dot indicator
pub fn status_dot(ui: &mut Ui, connected: bool) {
    let color = if connected {
        colors::SUCCESS
    } else {
        colors::TEXT_MUTED
    };

    let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.0, color);
}

/// Section header with optional description
#[allow(dead_code)]
pub fn section_header(ui: &mut Ui, title: &str, description: Option<&str>) {
    ui.label(
        RichText::new(title)
            .font(typography::subheading())
            .color(colors::TEXT_PRIMARY)
            .strong(),
    );

    if let Some(desc) = description {
        ui.label(
            RichText::new(desc)
                .font(typography::caption())
                .color(colors::TEXT_MUTED),
        );
    }
}

/// Info box with icon
pub fn info_box(ui: &mut Ui, message: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("\u{2139}").color(colors::PRIMARY)); // ℹ
        ui.add_space(4.0);
        ui.label(
            RichText::new(message)
                .font(typography::caption())
                .color(colors::TEXT_MUTED),
        );
    });
}

/// Device type icon (emoji-based for simplicity)
pub fn device_icon(device_type: &linglide_auth::device::DeviceType) -> &'static str {
    use linglide_auth::device::DeviceType;
    match device_type {
        DeviceType::Android => "\u{1F4F1}", // 📱
        DeviceType::Ios => "\u{1F34E}",     // 🍎
        DeviceType::Browser => "\u{1F310}", // 🌐
        DeviceType::Unknown => "\u{2753}",  // ❓
    }
}
