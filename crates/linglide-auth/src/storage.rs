//! Persistent storage for paired devices
//!
//! Uses JSON file storage in ~/.config/linglide/devices.json

use crate::device::{Device, DeviceId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Storage errors
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Device not found: {0}")]
    NotFound(String),
    #[error("Configuration directory not found")]
    NoConfigDir,
}

/// Result type for storage operations
pub type StorageResult<T> = Result<T, StorageError>;

/// Stored data structure
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct StoredData {
    /// Paired devices indexed by ID
    devices: HashMap<String, Device>,
}

/// Device storage manager with file persistence
pub struct DeviceStorage {
    /// Path to the storage file
    path: PathBuf,
    /// In-memory cache of devices
    data: Arc<RwLock<StoredData>>,
}

impl DeviceStorage {
    /// Create a new device storage instance
    ///
    /// Loads existing data from disk if present.
    pub async fn new() -> StorageResult<Self> {
        let path = Self::default_path()?;
        Self::with_path(path).await
    }

    /// Create storage at a specific path
    pub async fn with_path(path: PathBuf) -> StorageResult<Self> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Load existing data or create empty
        let data = if path.exists() {
            let contents = std::fs::read_to_string(&path)?;
            match serde_json::from_str(&contents) {
                Ok(data) => {
                    info!("Loaded device storage from {:?}", path);
                    data
                }
                Err(e) => {
                    warn!("Failed to parse device storage, starting fresh: {}", e);
                    StoredData::default()
                }
            }
        } else {
            debug!("No existing device storage, creating new");
            StoredData::default()
        };

        Ok(Self {
            path,
            data: Arc::new(RwLock::new(data)),
        })
    }

    /// Get the default storage path (~/.config/linglide/devices.json)
    fn default_path() -> StorageResult<PathBuf> {
        let config_dir = dirs::config_dir().ok_or(StorageError::NoConfigDir)?;
        Ok(config_dir.join("linglide").join("devices.json"))
    }

    /// Save current state to disk
    async fn save(&self) -> StorageResult<()> {
        let data = self.data.read().await;
        let json = serde_json::to_string_pretty(&*data)?;
        std::fs::write(&self.path, json)?;
        debug!("Saved device storage to {:?}", self.path);
        Ok(())
    }

    /// Add or update a device
    pub async fn save_device(&self, device: Device) -> StorageResult<()> {
        let id = device.id.to_string();
        {
            let mut data = self.data.write().await;
            data.devices.insert(id.clone(), device);
        }
        self.save().await?;
        info!("Saved device {}", id);
        Ok(())
    }

    /// Get a device by ID
    pub async fn get_device(&self, id: &DeviceId) -> Option<Device> {
        let data = self.data.read().await;
        data.devices.get(&id.to_string()).cloned()
    }

    /// Get a device by token hash
    pub async fn get_device_by_token_hash(&self, token_hash: &str) -> Option<Device> {
        let data = self.data.read().await;
        data.devices
            .values()
            .find(|d| d.token_hash == token_hash)
            .cloned()
    }

    /// List all paired devices
    pub async fn list_devices(&self) -> Vec<Device> {
        let data = self.data.read().await;
        data.devices.values().cloned().collect()
    }

    /// Remove a device by ID
    pub async fn remove_device(&self, id: &DeviceId) -> StorageResult<()> {
        let id_str = id.to_string();
        {
            let mut data = self.data.write().await;
            if data.devices.remove(&id_str).is_none() {
                return Err(StorageError::NotFound(id_str));
            }
        }
        self.save().await?;
        info!("Removed device {}", id_str);
        Ok(())
    }

    /// Update a device's last_seen timestamp
    pub async fn touch_device(&self, id: &DeviceId) -> StorageResult<()> {
        {
            let mut data = self.data.write().await;
            if let Some(device) = data.devices.get_mut(&id.to_string()) {
                device.touch();
            } else {
                return Err(StorageError::NotFound(id.to_string()));
            }
        }
        self.save().await
    }

    /// Get the number of paired devices
    pub async fn device_count(&self) -> usize {
        let data = self.data.read().await;
        data.devices.len()
    }

    /// Check if any devices are paired
    pub async fn has_devices(&self) -> bool {
        let data = self.data.read().await;
        !data.devices.is_empty()
    }

    /// Clear all paired devices
    pub async fn clear(&self) -> StorageResult<()> {
        {
            let mut data = self.data.write().await;
            data.devices.clear();
        }
        self.save().await?;
        info!("Cleared all paired devices");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::DeviceType;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_storage_crud() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_devices.json");

        let storage = DeviceStorage::with_path(path.clone()).await.unwrap();

        // Create device
        let device = Device::new(
            "Test".to_string(),
            DeviceType::Browser,
            "hash123".to_string(),
        );
        let id = device.id.clone();

        // Save
        storage.save_device(device).await.unwrap();

        // Read
        let loaded = storage.get_device(&id).await.unwrap();
        assert_eq!(loaded.name, "Test");

        // List
        let all = storage.list_devices().await;
        assert_eq!(all.len(), 1);

        // Remove
        storage.remove_device(&id).await.unwrap();
        assert!(storage.get_device(&id).await.is_none());
    }

    #[tokio::test]
    async fn test_storage_persistence() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_devices.json");

        let device_id;
        {
            let storage = DeviceStorage::with_path(path.clone()).await.unwrap();
            let device = Device::new(
                "Persistent".to_string(),
                DeviceType::Ios,
                "hash456".to_string(),
            );
            device_id = device.id.clone();
            storage.save_device(device).await.unwrap();
        }

        // Reload from disk
        let storage = DeviceStorage::with_path(path).await.unwrap();
        let loaded = storage.get_device(&device_id).await.unwrap();
        assert_eq!(loaded.name, "Persistent");
    }
}
