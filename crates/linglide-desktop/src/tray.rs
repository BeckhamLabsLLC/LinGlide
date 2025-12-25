//! System tray manager
//!
//! Handles system tray icon and menu for LinGlide.

#![allow(dead_code)]

use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

/// Tray menu item IDs
pub mod menu_ids {
    pub const SHOW_WINDOW: &str = "show_window";
    pub const START_SERVER: &str = "start_server";
    pub const STOP_SERVER: &str = "stop_server";
    pub const SHOW_QR: &str = "show_qr";
    pub const MANAGE_DEVICES: &str = "manage_devices";
    pub const SETTINGS: &str = "settings";
    pub const QUIT: &str = "quit";
}

/// System tray state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayState {
    /// Server is not running
    Idle,
    /// Server is running, waiting for connections
    Waiting,
    /// Device is connected
    Connected,
}

/// System tray manager
pub struct TrayManager {
    tray_icon: Option<TrayIcon>,
    menu: Menu,
    state: TrayState,
    // Menu items we need to update
    start_item: MenuItem,
    stop_item: MenuItem,
    qr_item: MenuItem,
}

impl TrayManager {
    /// Create a new tray manager
    pub fn new() -> anyhow::Result<Self> {
        // Create menu items
        let show_item = MenuItem::with_id(menu_ids::SHOW_WINDOW, "Show LinGlide", true, None);
        let start_item = MenuItem::with_id(menu_ids::START_SERVER, "Start Server", true, None);
        let stop_item = MenuItem::with_id(menu_ids::STOP_SERVER, "Stop Server", false, None);
        let qr_item = MenuItem::with_id(menu_ids::SHOW_QR, "Show QR Code", false, None);
        let devices_item = MenuItem::with_id(menu_ids::MANAGE_DEVICES, "Manage Devices", true, None);
        let settings_item = MenuItem::with_id(menu_ids::SETTINGS, "Settings", true, None);
        let quit_item = MenuItem::with_id(menu_ids::QUIT, "Quit", true, None);

        // Build menu
        let menu = Menu::new();
        menu.append(&show_item)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&start_item)?;
        menu.append(&stop_item)?;
        menu.append(&qr_item)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&devices_item)?;
        menu.append(&settings_item)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&quit_item)?;

        Ok(Self {
            tray_icon: None,
            menu,
            state: TrayState::Idle,
            start_item,
            stop_item,
            qr_item,
        })
    }

    /// Initialize the tray icon (must be called from main thread on some platforms)
    pub fn init(&mut self) -> anyhow::Result<()> {
        let icon = self.create_icon(TrayState::Idle)?;

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(self.menu.clone()))
            .with_tooltip("LinGlide - Screen Sharing")
            .with_icon(icon)
            .build()?;

        self.tray_icon = Some(tray_icon);
        Ok(())
    }

    /// Set the tray state and update icon/menu
    pub fn set_state(&mut self, state: TrayState) -> anyhow::Result<()> {
        if self.state == state {
            return Ok(());
        }

        self.state = state;

        // Update icon
        if let Some(ref tray_icon) = self.tray_icon {
            let icon = self.create_icon(state)?;
            tray_icon.set_icon(Some(icon))?;

            // Update tooltip
            let tooltip = match state {
                TrayState::Idle => "LinGlide - Stopped",
                TrayState::Waiting => "LinGlide - Waiting for connection",
                TrayState::Connected => "LinGlide - Device connected",
            };
            tray_icon.set_tooltip(Some(tooltip))?;
        }

        // Update menu items
        let server_running = state != TrayState::Idle;
        self.start_item.set_enabled(!server_running);
        self.stop_item.set_enabled(server_running);
        self.qr_item.set_enabled(server_running);

        Ok(())
    }

    /// Create an icon for the given state
    fn create_icon(&self, state: TrayState) -> anyhow::Result<Icon> {
        // Create a simple colored icon based on state
        // In production, these would be loaded from PNG files
        let (r, g, b) = match state {
            TrayState::Idle => (128, 128, 128),     // Gray
            TrayState::Waiting => (255, 200, 0),    // Yellow
            TrayState::Connected => (0, 200, 100),  // Green
        };

        // Create a 32x32 RGBA icon
        let size = 32;
        let mut rgba = Vec::with_capacity(size * size * 4);

        for y in 0..size {
            for x in 0..size {
                // Create a simple circle
                let cx = size as f32 / 2.0;
                let cy = size as f32 / 2.0;
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist < (size as f32 / 2.0 - 2.0) {
                    rgba.push(r);
                    rgba.push(g);
                    rgba.push(b);
                    rgba.push(255);
                } else {
                    rgba.push(0);
                    rgba.push(0);
                    rgba.push(0);
                    rgba.push(0);
                }
            }
        }

        Ok(Icon::from_rgba(rgba, size as u32, size as u32)?)
    }

    /// Get the menu event receiver
    pub fn menu_event_receiver() -> &'static MenuEvent {
        // This provides access to menu events
        // In the actual implementation, you'd use MenuEvent::receiver()
        // but for this skeleton we just return a static reference
        todo!("Menu event handling requires proper integration with event loop")
    }
}

impl Default for TrayManager {
    fn default() -> Self {
        Self::new().expect("Failed to create tray manager")
    }
}
