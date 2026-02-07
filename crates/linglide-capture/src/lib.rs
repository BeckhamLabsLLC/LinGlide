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
use xcb::Xid;

/// Detect the primary display resolution using XCB RandR.
/// Falls back to (1920, 1080) if detection fails.
pub fn detect_primary_display() -> (u32, u32) {
    match detect_primary_display_xcb() {
        Some(dims) => dims,
        None => {
            tracing::warn!("Could not detect primary display, using default 1920x1080");
            (1920, 1080)
        }
    }
}

fn detect_primary_display_xcb() -> Option<(u32, u32)> {
    let (conn, screen_num) = xcb::Connection::connect(None).ok()?;
    let setup = conn.get_setup();
    let screen = setup.roots().nth(screen_num as usize)?;

    let cookie = conn.send_request(&xcb::randr::GetScreenResourcesCurrent {
        window: screen.root(),
    });
    let reply = conn.wait_for_reply(cookie).ok()?;

    // Try to find the primary output first
    let primary_cookie = conn.send_request(&xcb::randr::GetOutputPrimary {
        window: screen.root(),
    });
    let primary_reply = conn.wait_for_reply(primary_cookie).ok();
    let primary_output = primary_reply.map(|r| r.output());

    for output in reply.outputs() {
        let info_cookie = conn.send_request(&xcb::randr::GetOutputInfo {
            output: *output,
            config_timestamp: reply.config_timestamp(),
        });
        let info = match conn.wait_for_reply(info_cookie) {
            Ok(i) => i,
            Err(_) => continue,
        };

        // Skip disconnected outputs
        if info.connection() != xcb::randr::Connection::Connected {
            continue;
        }

        let crtc = info.crtc();
        if crtc.is_none() {
            continue;
        }

        let crtc_cookie = conn.send_request(&xcb::randr::GetCrtcInfo {
            crtc,
            config_timestamp: reply.config_timestamp(),
        });
        let crtc_info = match conn.wait_for_reply(crtc_cookie) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let w = crtc_info.width() as u32;
        let h = crtc_info.height() as u32;

        if w == 0 || h == 0 {
            continue;
        }

        // If this is the primary output, return immediately
        if let Some(primary) = primary_output {
            if *output == primary {
                tracing::info!("Detected primary display: {}x{}", w, h);
                return Some((w, h));
            }
        }
    }

    // Fallback: return the first connected output with a valid mode
    for output in reply.outputs() {
        let info_cookie = conn.send_request(&xcb::randr::GetOutputInfo {
            output: *output,
            config_timestamp: reply.config_timestamp(),
        });
        let info = match conn.wait_for_reply(info_cookie) {
            Ok(i) => i,
            Err(_) => continue,
        };

        if info.connection() != xcb::randr::Connection::Connected {
            continue;
        }

        let crtc = info.crtc();
        if crtc.is_none() {
            continue;
        }

        let crtc_cookie = conn.send_request(&xcb::randr::GetCrtcInfo {
            crtc,
            config_timestamp: reply.config_timestamp(),
        });
        let crtc_info = match conn.wait_for_reply(crtc_cookie) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let w = crtc_info.width() as u32;
        let h = crtc_info.height() as u32;

        if w > 0 && h > 0 {
            tracing::info!("Detected first connected display: {}x{}", w, h);
            return Some((w, h));
        }
    }

    None
}

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
