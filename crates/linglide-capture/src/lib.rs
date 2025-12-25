//! LinGlide Capture - Screen capture for X11 and Wayland
//!
//! This crate provides screen capture using:
//! - X11 MIT-SHM extension (for X11 sessions)
//! - PipeWire via GStreamer (for Wayland sessions)

pub mod pipewire_capture;
pub mod virtual_display;
pub mod x11_capture;

// Re-export Frame from linglide-core for backwards compatibility
pub use linglide_core::Frame;
pub use pipewire_capture::PipeWireCapture;
pub use virtual_display::VirtualDisplay;
pub use x11_capture::X11Capture;

use linglide_core::Result;

/// Detect if running under Wayland
pub fn is_wayland() -> bool {
    std::env::var("XDG_SESSION_TYPE")
        .map(|v| v == "wayland")
        .unwrap_or(false)
        || std::env::var("WAYLAND_DISPLAY").is_ok()
}

/// Unified screen capture that works on both X11 and Wayland
pub enum ScreenCapture {
    X11(X11Capture),
    PipeWire(PipeWireCapture),
}

impl ScreenCapture {
    /// Create a new screen capture instance, automatically detecting the session type
    pub fn new(width: u32, height: u32, offset_x: i32, offset_y: i32) -> Result<Self> {
        if is_wayland() {
            tracing::info!("Detected Wayland session, using PipeWire capture");
            Ok(Self::PipeWire(PipeWireCapture::new(width, height)?))
        } else {
            tracing::info!("Detected X11 session, using MIT-SHM capture");
            Ok(Self::X11(X11Capture::new(
                width, height, offset_x, offset_y,
            )?))
        }
    }

    /// Capture a single frame
    pub fn capture(&mut self) -> Result<Frame> {
        match self {
            Self::X11(cap) => cap.capture(),
            Self::PipeWire(cap) => cap.capture(),
        }
    }

    /// Get the capture dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            Self::X11(cap) => cap.dimensions(),
            Self::PipeWire(cap) => cap.dimensions(),
        }
    }
}
