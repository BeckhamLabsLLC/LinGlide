//! Device identity and management
//!
//! Represents paired devices with their identity, name, and pairing metadata.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a device
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceId(pub Uuid);

impl DeviceId {
    /// Generate a new random device ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create from an existing UUID string
    pub fn parse(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl Default for DeviceId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for DeviceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A paired device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    /// Unique device identifier
    pub id: DeviceId,
    /// Human-readable device name (e.g., "iPhone 15 Pro", "Galaxy Tab S9")
    pub name: String,
    /// Device type/platform hint
    pub device_type: DeviceType,
    /// When this device was first paired
    pub paired_at: DateTime<Utc>,
    /// Last time this device connected
    pub last_seen: DateTime<Utc>,
    /// Authentication token for this device (hashed)
    pub token_hash: String,
}

impl Device {
    /// Create a new device with the given details
    pub fn new(name: String, device_type: DeviceType, token_hash: String) -> Self {
        let now = Utc::now();
        Self {
            id: DeviceId::new(),
            name,
            device_type,
            paired_at: now,
            last_seen: now,
            token_hash,
        }
    }

    /// Update the last seen timestamp
    pub fn touch(&mut self) {
        self.last_seen = Utc::now();
    }
}

/// Type of device connecting
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceType {
    /// iOS device (iPhone, iPad)
    Ios,
    /// Android device
    Android,
    /// Web browser
    Browser,
    /// Unknown/other device
    #[default]
    Unknown,
}

impl std::str::FromStr for DeviceType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ios" | "iphone" | "ipad" => Ok(Self::Ios),
            "android" => Ok(Self::Android),
            "browser" | "web" => Ok(Self::Browser),
            _ => Ok(Self::Unknown),
        }
    }
}

/// Summary information about a device for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub device_type: DeviceType,
    pub paired_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}

impl From<&Device> for DeviceInfo {
    fn from(device: &Device) -> Self {
        Self {
            id: device.id.to_string(),
            name: device.name.clone(),
            device_type: device.device_type,
            paired_at: device.paired_at,
            last_seen: device.last_seen,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_id_generation() {
        let id1 = DeviceId::new();
        let id2 = DeviceId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_device_creation() {
        let device = Device::new(
            "Test Device".to_string(),
            DeviceType::Browser,
            "hash123".to_string(),
        );
        assert_eq!(device.name, "Test Device");
        assert_eq!(device.device_type, DeviceType::Browser);
    }

    #[test]
    fn test_device_type_parsing() {
        assert_eq!("ios".parse::<DeviceType>().unwrap(), DeviceType::Ios);
        assert_eq!(
            "android".parse::<DeviceType>().unwrap(),
            DeviceType::Android
        );
        assert_eq!(
            "browser".parse::<DeviceType>().unwrap(),
            DeviceType::Browser
        );
        assert_eq!(
            "unknown".parse::<DeviceType>().unwrap(),
            DeviceType::Unknown
        );
    }
}
