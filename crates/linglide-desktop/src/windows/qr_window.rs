//! QR code display window
//!
//! Shows a large QR code for device pairing along with PIN and expiration.

use crate::bridge::{PairingState, UiCommand};
use egui::{Color32, ColorImage, RichText, TextureHandle, TextureOptions, Vec2};
use qrcode::QrCode;
use tokio::sync::mpsc;

/// QR code window state
#[derive(Default)]
pub struct QrWindow {
    /// Cached QR code texture
    qr_texture: Option<TextureHandle>,
    /// The data that was encoded in the cached texture
    cached_data: Option<String>,
}

impl QrWindow {
    pub fn new() -> Self {
        Self::default()
    }

    /// Show the QR code inline within a UI (not as separate window)
    pub fn show_inline(
        &mut self,
        ui: &mut egui::Ui,
        pairing: &PairingState,
        server_url: Option<&str>,
        fingerprint: Option<&str>,
        command_tx: &mpsc::Sender<UiCommand>,
    ) {
        if !pairing.active {
            return;
        }

        // Build QR code data
        let qr_data = if let (Some(url), Some(pin), Some(session_id)) =
            (server_url, &pairing.pin, &pairing.session_id)
        {
            let mut data = format!(
                "linglide://pair?url={}&pin={}&session={}",
                url, pin, session_id
            );
            if let Some(fp) = fingerprint {
                data.push_str(&format!("&fp={}", &fp[..fp.len().min(20)]));
            }
            data.push_str(&format!("&v={}", env!("CARGO_PKG_VERSION")));
            Some(data)
        } else {
            None
        };

        ui.vertical_centered(|ui| {
            // Generate and display QR code
            if let Some(ref data) = qr_data {
                // Check if we need to regenerate the texture
                if self.cached_data.as_ref() != Some(data) {
                    if let Some(texture) = self.generate_qr_texture(ui.ctx(), data) {
                        self.qr_texture = Some(texture);
                        self.cached_data = Some(data.clone());
                    }
                }

                if let Some(ref texture) = self.qr_texture {
                    let size = Vec2::splat(200.0);
                    ui.add(egui::Image::new(texture).fit_to_exact_size(size));
                }
            }

            ui.add_space(10.0);

            // Display PIN
            if let Some(pin) = &pairing.pin {
                ui.label("PIN:");
                ui.label(
                    RichText::new(pin)
                        .size(32.0)
                        .strong()
                        .color(Color32::from_rgb(100, 200, 100))
                        .monospace(),
                );
            }

            ui.add_space(8.0);

            // Expiration countdown
            let expires_in = pairing.expires_in;
            let color = if expires_in < 15 {
                Color32::from_rgb(255, 100, 100)
            } else if expires_in < 30 {
                Color32::from_rgb(255, 200, 100)
            } else {
                Color32::from_rgb(150, 150, 150)
            };

            ui.colored_label(color, format!("Expires in {}s", expires_in));

            ui.add_space(8.0);

            // Action buttons
            ui.horizontal(|ui| {
                if ui.button("Refresh").clicked() {
                    let _ = command_tx.try_send(UiCommand::StartPairing);
                }
                if ui.button("Cancel").clicked() {
                    let _ = command_tx.try_send(UiCommand::CancelPairing);
                }
            });
        });
    }

    /// Generate a QR code texture from data
    fn generate_qr_texture(&self, ctx: &egui::Context, data: &str) -> Option<TextureHandle> {
        let code = QrCode::new(data.as_bytes()).ok()?;

        // Convert to pixel data
        let qr_image = code.render::<image::Luma<u8>>().build();
        let width = qr_image.width() as usize;
        let height = qr_image.height() as usize;

        let pixels: Vec<Color32> = qr_image
            .pixels()
            .map(|p| {
                if p.0[0] == 0 {
                    Color32::BLACK
                } else {
                    Color32::WHITE
                }
            })
            .collect();

        let color_image = ColorImage {
            size: [width, height],
            pixels,
        };

        Some(ctx.load_texture(
            "qr_code",
            color_image,
            TextureOptions::NEAREST,
        ))
    }
}
