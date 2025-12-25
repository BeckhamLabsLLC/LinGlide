//! Virtual display management using EVDI
//!
//! Creates true virtual displays using the EVDI kernel module,
//! similar to how DisplayLink works.

use crate::Frame;
use evdi::prelude::*;
use linglide_core::{Config, Error, Result};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{info, warn};

/// EVDI-based virtual display
pub struct VirtualDisplay {
    /// Display configuration
    config: Config,
    /// EVDI handle (connected)
    handle: Option<Arc<Mutex<Handle>>>,
    /// Current buffer ID
    buffer_id: Option<BufferId>,
    /// Current mode
    mode: Option<Mode>,
    /// Frame sequence counter
    sequence: AtomicU64,
    /// Whether the display is active
    running: AtomicBool,
}

impl VirtualDisplay {
    /// Create a new EVDI virtual display
    pub fn new(config: Config) -> Result<Self> {
        info!(
            "Creating EVDI virtual display: {}x{} @ {} Hz",
            config.width, config.height, config.fps
        );

        // Check kernel module status
        match evdi::check_kernel_mod() {
            KernelModStatus::NotInstalled => {
                return Err(Error::VirtualDisplayCreation(
                    "EVDI kernel module not installed. Run: sudo modprobe evdi".to_string(),
                ));
            }
            KernelModStatus::Outdated => {
                return Err(Error::VirtualDisplayCreation(
                    "EVDI kernel module is outdated".to_string(),
                ));
            }
            KernelModStatus::Compatible => {
                info!("EVDI kernel module is compatible");
            }
        }

        Ok(Self {
            config,
            handle: None,
            buffer_id: None,
            mode: None,
            sequence: AtomicU64::new(0),
            running: AtomicBool::new(false),
        })
    }

    /// Enable the virtual display
    #[allow(clippy::arc_with_non_send_sync)] // Handle contains raw pointers from evdi; used on dedicated thread
    pub fn enable(&mut self) -> Result<()> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        // Get or add a device
        let device = match DeviceNode::get() {
            Some(d) => {
                info!("Found existing EVDI device");
                d
            }
            None => {
                info!("No EVDI device found, attempting to add one...");
                if !DeviceNode::add() {
                    return Err(Error::VirtualDisplayCreation(
                        "Failed to add EVDI device. Run: sudo modprobe -r evdi && sudo modprobe evdi initial_device_count=1".to_string(),
                    ));
                }
                DeviceNode::get().ok_or_else(|| {
                    Error::VirtualDisplayCreation(
                        "Failed to get EVDI device after adding".to_string(),
                    )
                })?
            }
        };

        info!("Opening EVDI device {:?}", device);

        // Open the device (unsafe as per evdi crate)
        let unconnected = unsafe {
            device.open().map_err(|e| {
                Error::VirtualDisplayCreation(format!("Failed to open EVDI device: {}", e))
            })?
        };

        // Connect with sample device config (contains EDID data)
        let device_config = DeviceConfig::sample();
        let handle = unconnected.connect(&device_config);

        info!("EVDI device connected, waiting for mode...");

        self.handle = Some(Arc::new(Mutex::new(handle)));
        self.running.store(true, Ordering::SeqCst);

        info!(
            "Virtual display enabled: {}x{} @ {} Hz",
            self.config.width, self.config.height, self.config.fps
        );

        Ok(())
    }

    /// Initialize the buffer after mode is received (call this from async context)
    pub async fn init_buffer(&mut self) -> Result<()> {
        let handle = self
            .handle
            .as_ref()
            .ok_or_else(|| Error::CaptureError("Virtual display not enabled".to_string()))?;

        let mut handle_guard = handle.lock().await;

        // Wait for the compositor to send us a mode
        // User may need to enable the display in GNOME Settings > Displays
        info!("");
        info!("=======================================================");
        info!("  VIRTUAL DISPLAY READY - ACTION REQUIRED");
        info!("=======================================================");
        info!("  1. Open Settings > Displays");
        info!("  2. You should see a new display (may show as 'Unknown')");
        info!("  3. Enable it and position it as desired");
        info!("  4. Click 'Apply' to activate");
        info!("");
        info!("  Waiting up to 60 seconds for display configuration...");
        info!("=======================================================");
        info!("");

        let timeout = Duration::from_secs(60);
        let mode = handle_guard
            .events
            .await_mode(timeout)
            .await
            .map_err(|e| Error::CaptureError(format!("Timeout waiting for display mode. Did you enable the display in Settings > Displays? Error: {:?}", e)))?;

        info!(
            "Received mode from compositor: {}x{}",
            mode.width, mode.height
        );

        // Create a buffer for this mode
        let buffer_id = handle_guard.new_buffer(&mode);

        self.mode = Some(mode);
        self.buffer_id = Some(buffer_id);

        Ok(())
    }

    /// Disable the virtual display
    pub fn disable(&mut self) -> Result<()> {
        if !self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        info!("Disabling virtual display");
        self.running.store(false, Ordering::SeqCst);

        // Drop the handle to disconnect
        self.handle = None;
        self.buffer_id = None;
        self.mode = None;

        info!("Virtual display disabled");
        Ok(())
    }

    /// Capture a frame from the virtual display (async)
    pub async fn capture_async(&mut self) -> Result<Frame> {
        let handle = self
            .handle
            .as_ref()
            .ok_or_else(|| Error::CaptureError("Virtual display not enabled".to_string()))?;

        let buffer_id = self.buffer_id.ok_or_else(|| {
            Error::CaptureError("Buffer not initialized. Call init_buffer() first".to_string())
        })?;

        let mut handle_guard = handle.lock().await;

        // Request an update - timeout is OK, we'll use the last buffer content
        // EVDI only sends updates when there are actual changes on screen
        let timeout = Duration::from_millis(50);
        let _ = handle_guard.request_update(buffer_id, timeout).await;
        // Ignore timeout errors - buffer still has valid data from last update

        // Get the buffer data (may be from a previous update if timeout)
        let buffer = handle_guard
            .get_buffer(buffer_id)
            .ok_or_else(|| Error::CaptureError("Buffer not found".to_string()))?;

        let bytes = buffer.bytes();
        let mode = self
            .mode
            .as_ref()
            .ok_or_else(|| Error::CaptureError("Mode not set".to_string()))?;

        let width = mode.width;
        let height = mode.height;

        // Copy the frame data
        let data = bytes.to_vec();

        let seq = self.sequence.fetch_add(1, Ordering::SeqCst);
        Ok(Frame::new(data, width, height, seq))
    }

    /// Get the display offset (for input coordinate mapping)
    pub fn get_offset(&self) -> Result<(i32, i32)> {
        // Query actual position would require compositor integration
        // For now, assume right-of primary
        // TODO: Get actual position from GNOME/compositor
        Ok((1920, 0))
    }

    /// Check if the display is active
    pub fn is_active(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Get the output name
    pub fn output(&self) -> &str {
        "EVDI-1"
    }
}

impl Drop for VirtualDisplay {
    fn drop(&mut self) {
        if self.running.load(Ordering::SeqCst) {
            if let Err(e) = self.disable() {
                warn!("Failed to disable virtual display on drop: {}", e);
            }
        }
    }
}
