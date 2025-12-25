//! GUI window modules

mod about;
mod qr_window;

pub use about::AboutSection;
pub use qr_window::QrWindow;

use crate::bridge::{PairingState, ServerStatus, UiCommand};
use crate::components::{
    card, danger_button, device_icon, info_box, primary_button, secondary_button, status_dot,
    Status, StatusBadge,
};
use crate::theme::{colors, rounding, spacing, typography};
use egui::{RichText, TextureHandle, Vec2};
use linglide_auth::device::Device;
use tokio::sync::mpsc;

/// Tab selection for the main window
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Status,
    Devices,
    Settings,
    About,
}

/// Settings configuration state
#[derive(Debug, Clone)]
pub struct Settings {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bitrate: u32,
    pub port: u16,
    pub mdns_enabled: bool,
    pub usb_enabled: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fps: 60,
            bitrate: 8000,
            port: 8443,
            mdns_enabled: true,
            usb_enabled: false,
        }
    }
}

/// Unified main window containing all UI
pub struct MainWindow {
    /// Current tab selection
    pub current_tab: Tab,
    /// Settings being edited
    pub settings: Settings,
    /// Device pending revocation confirmation
    pending_revoke: Option<String>,
    /// About section state
    about_section: AboutSection,
    /// Header logo texture
    header_logo: Option<TextureHandle>,
    /// Whether we've attempted to load the header logo
    header_logo_loaded: bool,
}

impl Default for MainWindow {
    fn default() -> Self {
        Self {
            current_tab: Tab::Status,
            settings: Settings::default(),
            pending_revoke: None,
            about_section: AboutSection::new(),
            header_logo: None,
            header_logo_loaded: false,
        }
    }
}

impl MainWindow {
    pub fn new() -> Self {
        Self::default()
    }

    /// Show the unified main window
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        status: &ServerStatus,
        pairing: &PairingState,
        paired_devices: &[Device],
        server_url: Option<&str>,
        fingerprint: Option<&str>,
        command_tx: &mpsc::Sender<UiCommand>,
        qr_window: &mut QrWindow,
    ) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(colors::BG_PRIMARY))
            .show(ctx, |ui| {
                // Header
                self.show_header(ctx, ui, status, command_tx);

                ui.add_space(8.0);

                // Tab bar
                self.show_tab_bar(ui, paired_devices.len());

                ui.separator();
                ui.add_space(spacing::CARD_MARGIN);

                // Tab content
                match self.current_tab {
                    Tab::Status => self.show_status_tab(
                        ui,
                        status,
                        pairing,
                        server_url,
                        fingerprint,
                        command_tx,
                        qr_window,
                    ),
                    Tab::Devices => {
                        self.show_devices_tab(ui, paired_devices, &status.connected_devices, command_tx)
                    }
                    Tab::Settings => self.show_settings_tab(ui, command_tx),
                    Tab::About => self.about_section.show(ui, ctx),
                }
            });
    }

    fn show_header(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, status: &ServerStatus, command_tx: &mpsc::Sender<UiCommand>) {
        // Load header logo if not yet attempted
        if !self.header_logo_loaded {
            self.header_logo_loaded = true;
            self.header_logo = load_header_logo(ctx);
        }

        egui::Frame::none()
            .fill(colors::BG_SECONDARY)
            .inner_margin(egui::Margin::symmetric(16.0, 12.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Logo image or fallback
                    if let Some(ref texture) = self.header_logo {
                        let size = Vec2::splat(32.0);
                        ui.add(egui::Image::new(texture).fit_to_exact_size(size));
                    } else {
                        // Fallback text logo
                        egui::Frame::none()
                            .fill(colors::with_alpha(colors::PRIMARY, 30))
                            .rounding(rounding::SMALL)
                            .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new("LG")
                                        .font(egui::FontId::proportional(16.0))
                                        .color(colors::PRIMARY)
                                        .strong(),
                                );
                            });
                    }

                    ui.add_space(8.0);

                    ui.label(
                        RichText::new("LinGlide")
                            .font(typography::heading())
                            .color(colors::TEXT_PRIMARY),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if status.running {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Stop Server")
                                            .color(colors::TEXT_PRIMARY)
                                            .strong(),
                                    )
                                    .fill(colors::ERROR)
                                    .rounding(rounding::SMALL),
                                )
                                .clicked()
                            {
                                let _ = command_tx.try_send(UiCommand::StopServer);
                            }
                        } else if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Start Server")
                                        .color(colors::TEXT_PRIMARY)
                                        .strong(),
                                )
                                .fill(colors::SUCCESS)
                                .rounding(rounding::SMALL),
                            )
                            .clicked()
                        {
                            let _ = command_tx.try_send(UiCommand::StartServer);
                        }
                    });
                });
            });
    }

    fn show_tab_bar(&mut self, ui: &mut egui::Ui, device_count: usize) {
        ui.horizontal(|ui| {
            ui.add_space(8.0);

            let tabs = [
                (Tab::Status, "Status".to_string()),
                (Tab::Devices, format!("Devices ({})", device_count)),
                (Tab::Settings, "Settings".to_string()),
                (Tab::About, "About".to_string()),
            ];

            for (tab, label) in tabs {
                let selected = self.current_tab == tab;
                let text_color = if selected {
                    colors::PRIMARY
                } else {
                    colors::TEXT_SECONDARY
                };

                let response = ui.add(
                    egui::SelectableLabel::new(
                        selected,
                        RichText::new(label).color(text_color),
                    )
                );

                if response.clicked() {
                    self.current_tab = tab;
                }
            }
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn show_status_tab(
        &mut self,
        ui: &mut egui::Ui,
        status: &ServerStatus,
        pairing: &PairingState,
        server_url: Option<&str>,
        fingerprint: Option<&str>,
        command_tx: &mpsc::Sender<UiCommand>,
        qr_window: &mut QrWindow,
    ) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Server status card
                card(ui, Some("Server Status"), |ui| {
                    ui.horizontal(|ui| {
                        if status.running {
                            ui.add(StatusBadge::success("Running"));
                        } else {
                            ui.add(StatusBadge::error("Stopped"));
                        }
                    });

                    if let Some(url) = &status.url {
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("URL:")
                                    .font(typography::body())
                                    .color(colors::TEXT_SECONDARY),
                            );
                            ui.add_space(4.0);
                            ui.monospace(RichText::new(url).color(colors::TEXT_PRIMARY));
                            if ui.small_button("\u{1F4CB}").on_hover_text("Copy URL").clicked() {
                                ui.output_mut(|o| o.copied_text = url.clone());
                            }
                        });
                    }

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("mDNS:")
                                .font(typography::body())
                                .color(colors::TEXT_SECONDARY),
                        );
                        ui.add_space(4.0);
                        if status.mdns_active {
                            ui.add(StatusBadge::new("Broadcasting", Status::Success));
                        } else {
                            ui.add(StatusBadge::inactive("Disabled"));
                        }
                    });
                });

                ui.add_space(spacing::CARD_MARGIN);

                // Pairing section (only when server running)
                if status.running {
                    card(ui, Some("Pair New Device"), |ui| {
                        if pairing.active {
                            qr_window.show_inline(ui, pairing, server_url, fingerprint, command_tx);
                        } else {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("Scan QR code from mobile device to connect")
                                        .color(colors::TEXT_SECONDARY),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if primary_button(ui, "Show QR Code").clicked() {
                                            let _ = command_tx.try_send(UiCommand::StartPairing);
                                        }
                                    },
                                );
                            });
                        }
                    });

                    ui.add_space(spacing::CARD_MARGIN);
                }

                // Connected devices card
                card(ui, Some("Connected Devices"), |ui| {
                    if status.connected_devices.is_empty() {
                        ui.vertical_centered(|ui| {
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new("No devices connected")
                                    .color(colors::TEXT_MUTED)
                                    .italics(),
                            );
                            ui.add_space(8.0);
                        });
                    } else {
                        for device in &status.connected_devices {
                            ui.horizontal(|ui| {
                                status_dot(ui, true);
                                ui.add_space(8.0);
                                ui.label(
                                    RichText::new(&device.name).color(colors::TEXT_PRIMARY),
                                );
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(format!("{:?}", device.device_type))
                                        .font(typography::caption())
                                        .color(colors::TEXT_MUTED),
                                );
                            });
                            ui.add_space(4.0);
                        }
                    }
                });
            });
    }

    fn show_devices_tab(
        &mut self,
        ui: &mut egui::Ui,
        devices: &[Device],
        connected_devices: &[Device],
        command_tx: &mpsc::Sender<UiCommand>,
    ) {
        let connected_ids: Vec<String> =
            connected_devices.iter().map(|d| d.id.to_string()).collect();

        if devices.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);

                // Empty state icon
                ui.label(
                    RichText::new("\u{1F4F1}")
                        .font(egui::FontId::proportional(48.0))
                        .color(colors::TEXT_MUTED),
                );

                ui.add_space(16.0);

                ui.label(
                    RichText::new("No paired devices")
                        .font(typography::subheading())
                        .color(colors::TEXT_PRIMARY),
                );

                ui.add_space(8.0);

                ui.label(
                    RichText::new("Use the QR code on the Status tab to pair a device")
                        .color(colors::TEXT_MUTED),
                );
            });
            return;
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for device in devices {
                    let device_id = device.id.to_string();
                    let is_connected = connected_ids.contains(&device_id);

                    // Device card
                    let border_color = if is_connected {
                        colors::with_alpha(colors::SUCCESS, 128)
                    } else {
                        colors::BORDER_LIGHT
                    };

                    egui::Frame::none()
                        .fill(colors::BG_SECONDARY)
                        .rounding(rounding::MEDIUM)
                        .inner_margin(egui::Margin::same(12.0))
                        .stroke(egui::Stroke::new(1.0, border_color))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                // Device type icon
                                let icon = device_icon(&device.device_type);
                                ui.label(
                                    RichText::new(icon).font(egui::FontId::proportional(28.0)),
                                );

                                ui.add_space(12.0);

                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(&device.name)
                                                .font(typography::subheading())
                                                .color(colors::TEXT_PRIMARY)
                                                .strong(),
                                        );

                                        if is_connected {
                                            ui.add_space(8.0);
                                            ui.add(StatusBadge::success("Connected"));
                                        }
                                    });

                                    ui.add_space(2.0);

                                    ui.label(
                                        RichText::new(format!(
                                            "Paired: {}",
                                            device.paired_at.format("%B %d, %Y")
                                        ))
                                        .font(typography::caption())
                                        .color(colors::TEXT_MUTED),
                                    );
                                });

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if self.pending_revoke.as_ref() == Some(&device_id) {
                                            if danger_button(ui, "Confirm").clicked() {
                                                let _ = command_tx.try_send(UiCommand::RevokeDevice {
                                                    device_id: device_id.clone(),
                                                });
                                                self.pending_revoke = None;
                                            }
                                            if secondary_button(ui, "Cancel").clicked() {
                                                self.pending_revoke = None;
                                            }
                                        } else if secondary_button(ui, "Revoke").clicked() {
                                            self.pending_revoke = Some(device_id.clone());
                                        }
                                    },
                                );
                            });
                        });

                    ui.add_space(spacing::CARD_MARGIN);
                }
            });
    }

    fn show_settings_tab(&mut self, ui: &mut egui::Ui, command_tx: &mpsc::Sender<UiCommand>) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Display Settings Section
                card(ui, Some("Display Settings"), |ui| {
                    egui::Grid::new("display_grid")
                        .num_columns(2)
                        .spacing([20.0, 8.0])
                        .show(ui, |ui| {
                            // Resolution
                            ui.label(
                                RichText::new("Resolution").color(colors::TEXT_SECONDARY),
                            );
                            ui.horizontal(|ui| {
                                let mut width = self.settings.width as i32;
                                let mut height = self.settings.height as i32;
                                ui.add(
                                    egui::DragValue::new(&mut width)
                                        .range(640..=3840)
                                        .speed(10),
                                );
                                ui.label(RichText::new("x").color(colors::TEXT_MUTED));
                                ui.add(
                                    egui::DragValue::new(&mut height)
                                        .range(480..=2160)
                                        .speed(10),
                                );
                                self.settings.width = width as u32;
                                self.settings.height = height as u32;
                            });
                            ui.end_row();

                            // Frame Rate
                            ui.label(
                                RichText::new("Frame Rate").color(colors::TEXT_SECONDARY),
                            );
                            let mut fps = self.settings.fps as i32;
                            ui.add(
                                egui::DragValue::new(&mut fps)
                                    .range(15..=120)
                                    .suffix(" FPS"),
                            );
                            self.settings.fps = fps as u32;
                            ui.end_row();

                            // Bitrate
                            ui.label(
                                RichText::new("Bitrate").color(colors::TEXT_SECONDARY),
                            );
                            let mut bitrate = self.settings.bitrate as i32;
                            ui.add(
                                egui::DragValue::new(&mut bitrate)
                                    .range(1000..=50000)
                                    .suffix(" kbps"),
                            );
                            self.settings.bitrate = bitrate as u32;
                            ui.end_row();
                        });
                });

                ui.add_space(spacing::CARD_MARGIN);

                // Network Settings Section
                card(ui, Some("Network Settings"), |ui| {
                    egui::Grid::new("network_grid")
                        .num_columns(2)
                        .spacing([20.0, 8.0])
                        .show(ui, |ui| {
                            ui.label(RichText::new("Port").color(colors::TEXT_SECONDARY));
                            let mut port = self.settings.port as i32;
                            ui.add(egui::DragValue::new(&mut port).range(1024..=65535));
                            self.settings.port = port as u16;
                            ui.end_row();
                        });
                });

                ui.add_space(spacing::CARD_MARGIN);

                // Discovery Settings Section
                card(ui, Some("Discovery"), |ui| {
                    if ui
                        .checkbox(
                            &mut self.settings.mdns_enabled,
                            RichText::new("Enable mDNS discovery").color(colors::TEXT_PRIMARY),
                        )
                        .on_hover_text(
                            "Allows mobile devices to discover this server on the local network",
                        )
                        .changed()
                    {
                        let _ = command_tx.try_send(UiCommand::SetMdns {
                            enabled: self.settings.mdns_enabled,
                        });
                    }

                    ui.add_space(4.0);

                    if ui
                        .checkbox(
                            &mut self.settings.usb_enabled,
                            RichText::new("Enable USB/ADB").color(colors::TEXT_PRIMARY),
                        )
                        .on_hover_text("Allow connections via USB cable (requires ADB)")
                        .changed()
                    {
                        let _ = command_tx.try_send(UiCommand::SetUsb {
                            enabled: self.settings.usb_enabled,
                        });
                    }
                });

                ui.add_space(spacing::SECTION);

                // Info note
                info_box(
                    ui,
                    "Display and network settings require server restart to take effect",
                );
            });
    }
}

/// Load the header logo from PNG file
fn load_header_logo(ctx: &egui::Context) -> Option<TextureHandle> {
    let icon_paths = [
        // Development path (relative to crate)
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/icons/linglide-icon.png"),
        // Installed paths
        "/usr/share/icons/hicolor/256x256/apps/linglide.png",
        "/usr/share/pixmaps/linglide.png",
    ];

    for path in icon_paths {
        if let Ok(image_data) = std::fs::read(path) {
            if let Ok(img) = image::load_from_memory(&image_data) {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let pixels = rgba.into_raw();
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                return Some(ctx.load_texture("header_logo", color_image, egui::TextureOptions::LINEAR));
            }
        }
    }

    None
}
