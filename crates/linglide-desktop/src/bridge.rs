//! Async/UI communication bridge
//!
//! Provides channels for communication between the egui UI thread
//! and the tokio async runtime running the server.

use linglide_auth::device::Device;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};

/// Events from the server/async side to the UI
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum UiEvent {
    /// Server started successfully
    ServerStarted {
        url: String,
        fingerprint: String,
        paired_devices: Vec<Device>,
        pin: String,
    },
    /// Server stopped
    ServerStopped,
    /// Persistent PIN was refreshed
    PinRefreshed { pin: String },
    /// Server failed to start
    ServerError { message: String },
    /// New device connected
    DeviceConnected { device: Device },
    /// Device disconnected
    DeviceDisconnected { device_id: String },
    /// Pairing session started
    PairingStarted {
        session_id: String,
        pin: String,
        expires_in: i64,
    },
    /// Pairing succeeded
    PairingSuccess { device: Device },
    /// Pairing failed
    PairingFailed { reason: String },
    /// mDNS advertisement status changed
    MdnsStatus { active: bool },
    /// USB/ADB status changed
    UsbStatus {
        connected: bool,
        device_count: usize,
    },
}

/// Commands from the UI to the server/async side
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum UiCommand {
    /// Start the server
    StartServer,
    /// Stop the server
    StopServer,
    /// Start a new pairing session
    StartPairing,
    /// Cancel current pairing session
    CancelPairing,
    /// Revoke a paired device
    RevokeDevice { device_id: String },
    /// Enable/disable mDNS advertisement
    SetMdns { enabled: bool },
    /// Enable/disable USB/ADB forwarding
    SetUsb { enabled: bool },
    /// Refresh the persistent PIN
    RefreshPin,
    /// Shutdown the application
    Shutdown,
}

/// Server status for display in UI
#[derive(Debug, Clone, Default)]
pub struct ServerStatus {
    pub running: bool,
    pub url: Option<String>,
    pub pin: Option<String>,
    pub connected_devices: Vec<Device>,
    pub paired_device_count: usize,
    pub mdns_active: bool,
    pub usb_active: bool,
    pub usb_device_count: usize,
}

/// Current pairing session state
#[derive(Debug, Clone, Default)]
pub struct PairingState {
    pub active: bool,
    pub session_id: Option<String>,
    pub pin: Option<String>,
    pub expires_in: i64,
}

/// Shared state between UI and async runtime
#[allow(dead_code)]
pub struct BridgeState {
    pub server_status: RwLock<ServerStatus>,
    pub pairing_state: RwLock<PairingState>,
}

impl BridgeState {
    pub fn new() -> Self {
        Self {
            server_status: RwLock::new(ServerStatus::default()),
            pairing_state: RwLock::new(PairingState::default()),
        }
    }
}

impl Default for BridgeState {
    fn default() -> Self {
        Self::new()
    }
}

/// Communication bridge between UI and async runtime
#[allow(dead_code)]
pub struct UiBridge {
    /// Channel to send commands to the async runtime
    pub command_tx: mpsc::Sender<UiCommand>,
    /// Channel to receive events from the async runtime
    pub event_rx: broadcast::Receiver<UiEvent>,
    /// Shared state
    pub state: Arc<BridgeState>,
}

/// Handle for the async runtime to communicate with the UI
#[allow(dead_code)]
pub struct AsyncBridge {
    /// Channel to receive commands from the UI
    pub command_rx: mpsc::Receiver<UiCommand>,
    /// Channel to send events to the UI
    pub event_tx: broadcast::Sender<UiEvent>,
    /// Shared state
    pub state: Arc<BridgeState>,
}

/// Create a new bridge pair for UI and async communication
pub fn create_bridge() -> (UiBridge, AsyncBridge) {
    let (command_tx, command_rx) = mpsc::channel::<UiCommand>(32);
    let (event_tx, event_rx) = broadcast::channel::<UiEvent>(64);
    let state = Arc::new(BridgeState::new());

    let ui_bridge = UiBridge {
        command_tx,
        event_rx,
        state: state.clone(),
    };

    let async_bridge = AsyncBridge {
        command_rx,
        event_tx,
        state,
    };

    (ui_bridge, async_bridge)
}
