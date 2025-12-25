//! Error types for LinGlide

use thiserror::Error;

/// Main error type for LinGlide operations
#[derive(Error, Debug)]
pub enum Error {
    #[error("X11 connection error: {0}")]
    X11Connection(String),

    #[error("X11 extension not available: {0}")]
    X11ExtensionMissing(String),

    #[error("Failed to create virtual display: {0}")]
    VirtualDisplayCreation(String),

    #[error("No disconnected output found for virtual display")]
    NoDisconnectedOutput,

    #[error("Screen capture failed: {0}")]
    CaptureError(String),

    #[error("Video encoding error: {0}")]
    EncoderError(String),

    #[error("Input injection error: {0}")]
    InputError(String),

    #[error("Failed to create uinput device: {0}")]
    UinputCreation(String),

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("Server error: {0}")]
    Server(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Command execution failed: {command} - {message}")]
    CommandFailed { command: String, message: String },

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Operation cancelled")]
    Cancelled,

    #[error("{0}")]
    Other(String),
}

/// Result type alias using LinGlide's Error
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Create a command execution error
    pub fn command_failed(command: impl Into<String>, message: impl Into<String>) -> Self {
        Error::CommandFailed {
            command: command.into(),
            message: message.into(),
        }
    }
}
