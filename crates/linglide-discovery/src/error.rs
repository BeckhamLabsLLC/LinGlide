//! Discovery error types

use thiserror::Error;

/// Errors that can occur during service discovery
#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("mDNS error: {0}")]
    Mdns(String),

    #[error("ADB not found in PATH")]
    AdbNotFound,

    #[error("ADB command failed: {0}")]
    AdbCommand(String),

    #[error("No Android device connected")]
    NoDeviceConnected,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type DiscoveryResult<T> = Result<T, DiscoveryError>;
