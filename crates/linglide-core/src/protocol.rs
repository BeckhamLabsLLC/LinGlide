//! WebSocket protocol message types

use serde::{Deserialize, Serialize};

/// Pen/stylus button types
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PenButton {
    /// Primary tip (default)
    #[default]
    Primary,
    /// Barrel button 1 (secondary)
    Secondary,
    /// Barrel button 2 (tertiary)
    Tertiary,
    /// Eraser end
    Eraser,
}

/// Input events sent from the web client to the server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum InputEvent {
    /// Touch started
    TouchStart {
        /// Touch point identifier (0-9 for multitouch)
        id: u32,
        /// X coordinate (normalized 0.0-1.0)
        x: f64,
        /// Y coordinate (normalized 0.0-1.0)
        y: f64,
    },
    /// Touch moved
    TouchMove {
        /// Touch point identifier
        id: u32,
        /// X coordinate (normalized 0.0-1.0)
        x: f64,
        /// Y coordinate (normalized 0.0-1.0)
        y: f64,
    },
    /// Touch ended
    TouchEnd {
        /// Touch point identifier
        id: u32,
    },
    /// Touch cancelled
    TouchCancel {
        /// Touch point identifier
        id: u32,
    },
    /// Mouse button pressed
    MouseDown {
        /// Button index (0=left, 1=middle, 2=right)
        button: u8,
        /// X coordinate (normalized 0.0-1.0)
        x: f64,
        /// Y coordinate (normalized 0.0-1.0)
        y: f64,
    },
    /// Mouse button released
    MouseUp {
        /// Button index
        button: u8,
        /// X coordinate (normalized 0.0-1.0)
        x: f64,
        /// Y coordinate (normalized 0.0-1.0)
        y: f64,
    },
    /// Mouse moved
    MouseMove {
        /// X coordinate (normalized 0.0-1.0)
        x: f64,
        /// Y coordinate (normalized 0.0-1.0)
        y: f64,
    },
    /// Scroll wheel
    Scroll {
        /// Horizontal scroll delta
        dx: f64,
        /// Vertical scroll delta
        dy: f64,
    },
    /// Keyboard key pressed
    KeyDown {
        /// Key code
        key: String,
        /// Modifier keys
        modifiers: Modifiers,
    },
    /// Keyboard key released
    KeyUp {
        /// Key code
        key: String,
        /// Modifier keys
        modifiers: Modifiers,
    },
    /// Stylus/pen hovering (not touching surface)
    PenHover {
        /// X coordinate (normalized 0.0-1.0)
        x: f64,
        /// Y coordinate (normalized 0.0-1.0)
        y: f64,
        /// Pressure (0.0 for hover)
        pressure: f64,
        /// Tilt X angle in degrees (-90 to 90)
        tilt_x: f64,
        /// Tilt Y angle in degrees (-90 to 90)
        tilt_y: f64,
    },
    /// Stylus/pen touched surface
    PenDown {
        /// X coordinate (normalized 0.0-1.0)
        x: f64,
        /// Y coordinate (normalized 0.0-1.0)
        y: f64,
        /// Pressure (normalized 0.0-1.0)
        pressure: f64,
        /// Tilt X angle in degrees (-90 to 90)
        tilt_x: f64,
        /// Tilt Y angle in degrees (-90 to 90)
        tilt_y: f64,
        /// Which pen button/tool is active
        button: PenButton,
    },
    /// Stylus/pen moved while touching
    PenMove {
        /// X coordinate (normalized 0.0-1.0)
        x: f64,
        /// Y coordinate (normalized 0.0-1.0)
        y: f64,
        /// Pressure (normalized 0.0-1.0)
        pressure: f64,
        /// Tilt X angle in degrees (-90 to 90)
        tilt_x: f64,
        /// Tilt Y angle in degrees (-90 to 90)
        tilt_y: f64,
    },
    /// Stylus/pen lifted from surface
    PenUp {
        /// X coordinate (normalized 0.0-1.0)
        x: f64,
        /// Y coordinate (normalized 0.0-1.0)
        y: f64,
    },
    /// Stylus barrel button pressed/released
    PenButtonEvent {
        /// Which button
        button: PenButton,
        /// True if pressed, false if released
        pressed: bool,
    },
}

/// Keyboard modifier keys state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}

/// Server-to-client control messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// Initial configuration sent to client
    Init {
        width: u32,
        height: u32,
        fps: u32,
        /// WebCodecs codec string (e.g., "avc1.64002a")
        #[serde(skip_serializing_if = "Option::is_none")]
        codec: Option<String>,
        /// Base64-encoded avcC data for decoder configuration
        #[serde(skip_serializing_if = "Option::is_none")]
        codec_data: Option<String>,
    },
    /// Error message
    Error { message: String },
    /// Server is ready to stream
    Ready,
    /// Ping for connection keepalive
    Ping { timestamp: u64 },
}

/// Client-to-server control messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Client is ready to receive video
    Ready,
    /// Pong response to ping
    Pong { timestamp: u64 },
    /// Request quality change
    SetQuality { bitrate: u32 },
}

/// Frame metadata for video synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameMetadata {
    /// Frame sequence number
    pub sequence: u64,
    /// Timestamp in microseconds
    pub timestamp_us: u64,
    /// Whether this is a keyframe
    pub is_keyframe: bool,
}
