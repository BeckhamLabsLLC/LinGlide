//! USB/ADB support for Android devices
//!
//! Manages ADB port forwarding to allow Android devices connected via USB
//! to access the LinGlide server without network configuration.

use crate::error::{DiscoveryError, DiscoveryResult};
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Manages USB connections for Android devices via ADB
pub struct UsbConnectionManager {
    port: u16,
    forward_active: bool,
}

impl UsbConnectionManager {
    /// Create a new USB connection manager
    ///
    /// # Arguments
    /// * `port` - The port the LinGlide server is running on
    pub fn new(port: u16) -> Self {
        Self {
            port,
            forward_active: false,
        }
    }

    /// Check if ADB is available in PATH
    pub async fn is_adb_available(&self) -> bool {
        match Command::new("adb").arg("version").output().await {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    /// List connected Android devices
    pub async fn list_devices(&self) -> DiscoveryResult<Vec<String>> {
        let output = Command::new("adb")
            .arg("devices")
            .output()
            .await
            .map_err(|_| DiscoveryError::AdbNotFound)?;

        if !output.status.success() {
            return Err(DiscoveryError::AdbCommand(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let devices: Vec<String> = stdout
            .lines()
            .skip(1) // Skip "List of devices attached" header
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 && parts[1] == "device" {
                    Some(parts[0].to_string())
                } else {
                    None
                }
            })
            .collect();

        debug!("ADB: Found {} device(s)", devices.len());
        Ok(devices)
    }

    /// Setup ADB reverse port forwarding
    ///
    /// This allows Android devices connected via USB to access the server
    /// at localhost:PORT on the device side.
    pub async fn setup_forwarding(&mut self) -> DiscoveryResult<()> {
        if self.forward_active {
            debug!("ADB: Forwarding already active");
            return Ok(());
        }

        // Check for connected devices first
        let devices = self.list_devices().await?;
        if devices.is_empty() {
            return Err(DiscoveryError::NoDeviceConnected);
        }

        // Setup reverse port forwarding: device:PORT -> host:PORT
        let output = Command::new("adb")
            .args([
                "reverse",
                &format!("tcp:{}", self.port),
                &format!("tcp:{}", self.port),
            ])
            .output()
            .await
            .map_err(|_| DiscoveryError::AdbNotFound)?;

        if !output.status.success() {
            return Err(DiscoveryError::AdbCommand(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        self.forward_active = true;
        info!(
            "ADB: Reverse port forwarding enabled (device:{} -> host:{})",
            self.port, self.port
        );

        Ok(())
    }

    /// Remove ADB reverse port forwarding
    pub async fn remove_forwarding(&mut self) -> DiscoveryResult<()> {
        if !self.forward_active {
            return Ok(());
        }

        let output = Command::new("adb")
            .args(["reverse", "--remove", &format!("tcp:{}", self.port)])
            .output()
            .await
            .map_err(|_| DiscoveryError::AdbNotFound)?;

        if !output.status.success() {
            warn!(
                "ADB: Failed to remove forwarding: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        self.forward_active = false;
        info!("ADB: Reverse port forwarding removed");

        Ok(())
    }

    /// Check if forwarding is currently active
    pub fn is_forward_active(&self) -> bool {
        self.forward_active
    }

    /// Get the port being forwarded
    pub fn port(&self) -> u16 {
        self.port
    }
}

impl Drop for UsbConnectionManager {
    fn drop(&mut self) {
        if self.forward_active {
            // Try to clean up forwarding synchronously
            // Note: This is best-effort since we can't await in drop
            let port = self.port;
            std::process::Command::new("adb")
                .args(["reverse", "--remove", &format!("tcp:{}", port)])
                .output()
                .ok();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_usb_manager_creation() {
        let manager = UsbConnectionManager::new(8443);
        assert_eq!(manager.port(), 8443);
        assert!(!manager.is_forward_active());
    }
}
