//! About Section Component
//!
//! Displays application info, version, credits, and links.

use crate::components::{card, link_button};
use crate::theme::{colors, rounding, spacing, typography};
use egui::{RichText, TextureHandle, Ui, Vec2};

/// About section state
#[derive(Default)]
pub struct AboutSection {
    /// Cached logo texture
    logo_texture: Option<TextureHandle>,
    /// Whether we've attempted to load the logo
    logo_load_attempted: bool,
    /// Cached BeckhamLabs logo texture
    beckhamlabs_texture: Option<TextureHandle>,
    /// Whether we've attempted to load the BeckhamLabs logo
    beckhamlabs_load_attempted: bool,
}

impl AboutSection {
    pub fn new() -> Self {
        Self::default()
    }

    /// Show the about section content
    pub fn show(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(spacing::SECTION);

                    // Logo
                    self.show_logo(ui, ctx);

                    ui.add_space(12.0);

                    // App name
                    ui.label(
                        RichText::new("LinGlide")
                            .font(typography::heading())
                            .color(colors::TEXT_PRIMARY)
                            .strong(),
                    );

                    ui.add_space(4.0);

                    // Version
                    ui.label(
                        RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION")))
                            .font(typography::body())
                            .color(colors::TEXT_SECONDARY),
                    );

                    ui.add_space(spacing::SECTION);
                });

                // Description card
                card(ui, None, |ui| {
                    ui.label(
                        RichText::new(
                            "High-performance Linux native screen sharing for mobile devices. \
                             Use your phone or tablet as an extended display with touch control.",
                        )
                        .font(typography::body())
                        .color(colors::TEXT_SECONDARY),
                    );
                });

                ui.add_space(spacing::CARD_MARGIN);

                // Credits card
                card(ui, Some("Credits"), |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Developed by")
                                .font(typography::body())
                                .color(colors::TEXT_SECONDARY),
                        );
                    });
                    ui.add_space(8.0);
                    self.show_beckhamlabs_logo(ui, ctx);
                });

                ui.add_space(spacing::CARD_MARGIN);

                // Links card
                card(ui, Some("Links"), |ui| {
                    ui.horizontal_wrapped(|ui| {
                        link_button(
                            ui,
                            "GitHub Repository",
                            "https://github.com/BeckhamLabs/linglide",
                        );
                        ui.add_space(16.0);
                        link_button(
                            ui,
                            "Report Issue",
                            "https://github.com/BeckhamLabs/linglide/issues",
                        );
                    });
                });

                ui.add_space(spacing::CARD_MARGIN);

                // License card
                card(ui, Some("License"), |ui| {
                    ui.label(
                        RichText::new("MIT License")
                            .font(typography::body())
                            .color(colors::TEXT_PRIMARY),
                    );

                    ui.add_space(4.0);

                    ui.label(
                        RichText::new("Copyright (c) 2024-2025 BeckhamLabs")
                            .font(typography::caption())
                            .color(colors::TEXT_MUTED),
                    );

                    ui.add_space(8.0);

                    ui.label(
                        RichText::new(
                            "Permission is hereby granted, free of charge, to any person \
                             obtaining a copy of this software and associated documentation \
                             files, to deal in the Software without restriction, including \
                             without limitation the rights to use, copy, modify, merge, \
                             publish, distribute, sublicense, and/or sell copies of the \
                             Software...",
                        )
                        .font(typography::caption())
                        .color(colors::TEXT_MUTED),
                    );

                    ui.add_space(8.0);

                    if ui.small_button("View Full License").clicked() {
                        let _ = open::that("https://opensource.org/licenses/MIT");
                    }
                });

                ui.add_space(spacing::SECTION);

                // Footer
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new("Made with care for the Linux community")
                            .font(typography::caption())
                            .color(colors::TEXT_MUTED)
                            .italics(),
                    );
                });

                ui.add_space(spacing::SECTION);
            });
    }

    /// Load and display the logo
    fn show_logo(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        // Try to load logo texture if not yet attempted
        if !self.logo_load_attempted {
            self.logo_load_attempted = true;
            self.logo_texture = load_logo_texture(ctx);
        }

        if let Some(ref texture) = self.logo_texture {
            let size = Vec2::splat(80.0);
            ui.add(egui::Image::new(texture).fit_to_exact_size(size));
        } else {
            // Fallback: show a stylized text logo
            show_fallback_logo(ui);
        }
    }

    /// Load and display the BeckhamLabs logo
    fn show_beckhamlabs_logo(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        // Try to load BeckhamLabs logo texture if not yet attempted
        if !self.beckhamlabs_load_attempted {
            self.beckhamlabs_load_attempted = true;
            self.beckhamlabs_texture = load_beckhamlabs_texture(ctx);
        }

        if let Some(ref texture) = self.beckhamlabs_texture {
            // Display as clickable image that opens the website
            let size = Vec2::new(180.0, 45.0);
            let response = ui.add(
                egui::Image::new(texture)
                    .fit_to_exact_size(size)
                    .sense(egui::Sense::click()),
            );
            if response.clicked() {
                let _ = open::that("https://beckhamlabs.com");
            }
            response.on_hover_cursor(egui::CursorIcon::PointingHand);
        } else {
            // Fallback to text link
            link_button(ui, "BeckhamLabs", "https://beckhamlabs.com");
        }
    }
}

/// Load logo from file
fn load_logo_texture(ctx: &egui::Context) -> Option<TextureHandle> {
    // Try various paths for the logo
    let logo_paths = [
        // Development path
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/icons/linglide-logo.png"
        ),
        // Installed paths
        "/usr/share/icons/hicolor/128x128/apps/linglide.png",
        "/usr/share/pixmaps/linglide.png",
    ];

    for path in logo_paths {
        if let Some(texture) = try_load_texture(ctx, path) {
            return Some(texture);
        }
    }

    None
}

fn try_load_texture(ctx: &egui::Context, path: &str) -> Option<TextureHandle> {
    let image_data = std::fs::read(path).ok()?;
    let image = image::load_from_memory(&image_data).ok()?;
    let rgba = image.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let pixels = rgba.into_raw();

    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
    Some(ctx.load_texture("linglide_logo", color_image, egui::TextureOptions::LINEAR))
}

/// Show a fallback logo when image not available
fn show_fallback_logo(ui: &mut Ui) {
    // Draw a stylized "LG" text as logo placeholder
    egui::Frame::none()
        .fill(colors::with_alpha(colors::PRIMARY, 30))
        .rounding(rounding::LARGE)
        .inner_margin(egui::Margin::same(16.0))
        .show(ui, |ui| {
            ui.label(
                RichText::new("LG")
                    .font(egui::FontId::proportional(36.0))
                    .color(colors::PRIMARY)
                    .strong(),
            );
        });
}

/// Load BeckhamLabs logo from file
fn load_beckhamlabs_texture(ctx: &egui::Context) -> Option<TextureHandle> {
    let logo_paths = [
        // Development path
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/icons/beckhamlabs-logo.png"
        ),
        // Installed path
        "/usr/share/linglide/beckhamlabs-logo.png",
    ];

    for path in logo_paths {
        if let Some(texture) = try_load_texture_named(ctx, path, "beckhamlabs_logo") {
            return Some(texture);
        }
    }

    None
}

fn try_load_texture_named(ctx: &egui::Context, path: &str, name: &str) -> Option<TextureHandle> {
    let image_data = std::fs::read(path).ok()?;
    let image = image::load_from_memory(&image_data).ok()?;
    let rgba = image.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let pixels = rgba.into_raw();

    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
    Some(ctx.load_texture(name, color_image, egui::TextureOptions::LINEAR))
}
