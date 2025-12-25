//! Main eframe Application
//!
//! The core GUI application that manages windows and handles events.

use crate::bridge::{PairingState, ServerStatus, UiBridge, UiCommand, UiEvent};
use crate::theme;
use crate::windows::{MainWindow, QrWindow};
use linglide_auth::device::Device;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Main LinGlide GUI application
pub struct LinGlideApp {
    /// Communication bridge with async runtime
    bridge: UiBridge,
    /// Main window (contains all UI)
    main_window: MainWindow,
    /// QR code renderer
    qr_window: QrWindow,
    /// Current server status
    server_status: ServerStatus,
    /// Current pairing state
    pairing_state: PairingState,
    /// List of all paired devices
    paired_devices: Vec<Device>,
    /// Server URL for QR codes
    server_url: Option<String>,
    /// Certificate fingerprint
    cert_fingerprint: Option<String>,
    /// Last time we polled for events
    last_event_poll: Instant,
    /// Countdown update time for pairing
    last_countdown_update: Instant,
}

impl LinGlideApp {
    /// Create a new application instance
    pub fn new(cc: &eframe::CreationContext<'_>, bridge: UiBridge) -> Self {
        // Apply LinGlide theme
        theme::apply_theme(&cc.egui_ctx);

        Self {
            bridge,
            main_window: MainWindow::new(),
            qr_window: QrWindow::new(),
            server_status: ServerStatus::default(),
            pairing_state: PairingState::default(),
            paired_devices: Vec::new(),
            server_url: None,
            cert_fingerprint: None,
            last_event_poll: Instant::now(),
            last_countdown_update: Instant::now(),
        }
    }

    /// Process pending events from the async runtime
    fn process_events(&mut self) {
        // Only poll every 16ms to avoid busy-waiting
        if self.last_event_poll.elapsed() < Duration::from_millis(16) {
            return;
        }
        self.last_event_poll = Instant::now();

        // Process all pending events
        while let Ok(event) = self.bridge.event_rx.try_recv() {
            self.handle_event(event);
        }
    }

    /// Handle a single event from the async runtime
    fn handle_event(&mut self, event: UiEvent) {
        match event {
            UiEvent::ServerStarted {
                url,
                fingerprint,
                paired_devices,
            } => {
                info!("Server started: {}", url);
                self.server_status.running = true;
                self.server_url = Some(url.clone());
                self.server_status.url = Some(url);
                self.cert_fingerprint = Some(fingerprint);
                self.paired_devices = paired_devices.clone();
                self.server_status.paired_device_count = paired_devices.len();

                // Auto-show QR code if no devices are paired
                if paired_devices.is_empty() {
                    info!("No paired devices - automatically starting pairing");
                    let _ = self.bridge.command_tx.try_send(UiCommand::StartPairing);
                }
            }
            UiEvent::ServerStopped => {
                info!("Server stopped");
                self.server_status.running = false;
                self.server_status.url = None;
                self.server_status.connected_devices.clear();
                self.pairing_state = PairingState::default();
            }
            UiEvent::ServerError { message } => {
                warn!("Server error: {}", message);
                self.server_status.running = false;
            }
            UiEvent::DeviceConnected { device } => {
                info!("Device connected: {}", device.name);
                self.server_status.connected_devices.push(device);
            }
            UiEvent::DeviceDisconnected { device_id } => {
                info!("Device disconnected: {}", device_id);
                self.server_status
                    .connected_devices
                    .retain(|d| d.id.to_string() != device_id);
            }
            UiEvent::PairingStarted {
                session_id,
                pin,
                expires_in,
            } => {
                debug!("Pairing session started: {}", session_id);
                self.pairing_state.active = true;
                self.pairing_state.session_id = Some(session_id);
                self.pairing_state.pin = Some(pin);
                self.pairing_state.expires_in = expires_in;
                self.last_countdown_update = Instant::now();
            }
            UiEvent::PairingSuccess { device } => {
                info!("Pairing successful: {}", device.name);
                self.pairing_state = PairingState::default();
                self.paired_devices.push(device);
                self.server_status.paired_device_count = self.paired_devices.len();
            }
            UiEvent::PairingFailed { reason } => {
                warn!("Pairing failed: {}", reason);
                self.pairing_state = PairingState::default();
            }
            UiEvent::MdnsStatus { active } => {
                self.server_status.mdns_active = active;
            }
            UiEvent::UsbStatus {
                connected,
                device_count,
            } => {
                self.server_status.usb_active = connected;
                self.server_status.usb_device_count = device_count;
            }
        }
    }

    /// Update pairing countdown
    fn update_countdown(&mut self) {
        if !self.pairing_state.active {
            return;
        }

        let elapsed = self.last_countdown_update.elapsed().as_secs() as i64;
        if elapsed > 0 {
            self.pairing_state.expires_in = (self.pairing_state.expires_in - elapsed).max(0);
            self.last_countdown_update = Instant::now();

            // If expired, clear pairing state
            if self.pairing_state.expires_in == 0 {
                self.pairing_state.active = false;
            }
        }
    }
}

impl eframe::App for LinGlideApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process events from async runtime
        self.process_events();

        // Update pairing countdown
        self.update_countdown();

        // Request repaint to keep UI responsive
        ctx.request_repaint_after(Duration::from_millis(100));

        // Show unified main window
        self.main_window.show(
            ctx,
            &self.server_status,
            &self.pairing_state,
            &self.paired_devices,
            self.server_url.as_deref(),
            self.cert_fingerprint.as_deref(),
            &self.bridge.command_tx,
            &mut self.qr_window,
        );
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Send shutdown command
        let _ = self.bridge.command_tx.try_send(UiCommand::Shutdown);
    }
}
