//! LinGlide Discovery
//!
//! Provides network discovery and USB connection support for LinGlide:
//!
//! - **mDNS/DNS-SD**: Advertises the LinGlide server on the local network
//!   using the `_linglide._tcp.local.` service type, allowing mobile devices
//!   to automatically discover available servers.
//!
//! - **USB/ADB**: Manages ADB reverse port forwarding for Android devices
//!   connected via USB, enabling direct connections without network setup.

mod error;
mod mdns;
mod usb;

pub use error::{DiscoveryError, DiscoveryResult};
pub use mdns::{ServiceAdvertiser, SERVICE_NAME_PREFIX, SERVICE_TYPE};
pub use usb::UsbConnectionManager;

/// Discovery service information returned by the API
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiscoveryInfo {
    /// The mDNS service type (e.g., "_linglide._tcp.local.")
    pub service_type: &'static str,
    /// The instance name (e.g., "LinGlide-hostname")
    pub instance_name: String,
    /// The server port
    pub port: u16,
    /// TLS certificate fingerprint (first 20 chars)
    pub fingerprint: Option<String>,
    /// Available IP addresses
    pub addresses: Vec<String>,
    /// Server version
    pub version: String,
}

impl DiscoveryInfo {
    /// Create new discovery info
    pub fn new(
        instance_name: String,
        port: u16,
        fingerprint: Option<String>,
        addresses: Vec<String>,
        version: String,
    ) -> Self {
        Self {
            service_type: SERVICE_TYPE,
            instance_name,
            port,
            fingerprint,
            addresses,
            version,
        }
    }
}
