//! Device pairing with PIN/QR code verification
//!
//! Implements a secure pairing flow:
//! 1. Server generates 6-digit PIN with 60-second validity
//! 2. Client enters PIN (or scans QR with embedded PIN)
//! 3. Upon successful verification, server issues auth token
//! 4. Token is used for subsequent WebSocket connections

use crate::device::{Device, DeviceType};
use crate::storage::{DeviceStorage, StorageResult};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

/// Default PIN validity duration in seconds
pub const PIN_VALIDITY_SECONDS: i64 = 60;

/// Pairing errors
#[derive(Debug, Error)]
pub enum PairingError {
    #[error("Invalid or expired PIN")]
    InvalidPin,
    #[error("Session not found or expired")]
    SessionNotFound,
    #[error("Invalid token")]
    InvalidToken,
    #[error("Storage error: {0}")]
    Storage(#[from] crate::storage::StorageError),
}

pub type PairingResult<T> = Result<T, PairingError>;

/// A pairing session awaiting PIN verification
#[derive(Debug, Clone)]
struct PairingSession {
    /// Session ID for tracking
    session_id: String,
    /// The 6-digit PIN
    pin: String,
    /// When the session expires
    expires_at: DateTime<Utc>,
}

impl PairingSession {
    fn new() -> Self {
        let mut rng = rand::thread_rng();
        let pin: u32 = rng.gen_range(0..1_000_000);
        let now = Utc::now();

        Self {
            session_id: Uuid::new_v4().to_string(),
            pin: format!("{:06}", pin),
            expires_at: now + Duration::seconds(PIN_VALIDITY_SECONDS),
        }
    }

    fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    fn verify_pin(&self, pin: &str) -> bool {
        !self.is_expired() && self.pin == pin
    }
}

/// Response when starting a pairing session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingStartResponse {
    /// Session ID for the pairing flow
    pub session_id: String,
    /// The 6-digit PIN to display/share
    pub pin: String,
    /// Seconds until this PIN expires
    pub expires_in: i64,
}

/// Request to verify a PIN
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingVerifyRequest {
    /// The session ID from start
    pub session_id: String,
    /// The PIN entered by user
    pub pin: String,
    /// Device name provided by client
    pub device_name: String,
    /// Device type hint
    #[serde(default)]
    pub device_type: Option<String>,
}

/// Response after successful PIN verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingVerifyResponse {
    /// The device ID assigned to this device
    pub device_id: String,
    /// Auth token for future connections
    pub token: String,
}

/// QR code data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QrCodeData {
    /// Server URL to connect to
    pub url: String,
    /// The pairing PIN
    pub pin: String,
    /// Session ID
    pub session_id: String,
    /// Certificate fingerprint (first 20 chars) for TLS verification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    /// Server version for compatibility checking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Manages device pairing and authentication
pub struct PairingManager {
    /// Active pairing sessions
    sessions: Arc<RwLock<HashMap<String, PairingSession>>>,
    /// Device storage
    storage: Arc<DeviceStorage>,
    /// Server URL for QR codes
    server_url: String,
    /// Certificate fingerprint for QR codes
    cert_fingerprint: Option<String>,
}

impl PairingManager {
    /// Create a new pairing manager
    pub fn new(storage: Arc<DeviceStorage>, server_url: String) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            storage,
            server_url,
            cert_fingerprint: None,
        }
    }

    /// Create a new pairing manager with certificate fingerprint
    pub fn with_fingerprint(
        storage: Arc<DeviceStorage>,
        server_url: String,
        fingerprint: Option<String>,
    ) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            storage,
            server_url,
            cert_fingerprint: fingerprint,
        }
    }

    /// Set the certificate fingerprint
    pub fn set_fingerprint(&mut self, fingerprint: Option<String>) {
        self.cert_fingerprint = fingerprint;
    }

    /// Start a new pairing session
    pub async fn start_pairing(&self) -> PairingStartResponse {
        let session = PairingSession::new();
        let response = PairingStartResponse {
            session_id: session.session_id.clone(),
            pin: session.pin.clone(),
            expires_in: PIN_VALIDITY_SECONDS,
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session.session_id.clone(), session);

        // Clean up expired sessions
        sessions.retain(|_, s| !s.is_expired());

        info!("Started pairing session");
        response
    }

    /// Verify a PIN and complete pairing
    pub async fn verify_pin(
        &self,
        request: PairingVerifyRequest,
    ) -> PairingResult<PairingVerifyResponse> {
        // Find and validate session
        let session = {
            let sessions = self.sessions.read().await;
            sessions.get(&request.session_id).cloned()
        };

        let session = session.ok_or(PairingError::SessionNotFound)?;

        if !session.verify_pin(&request.pin) {
            warn!("Invalid PIN attempt for session {}", request.session_id);
            return Err(PairingError::InvalidPin);
        }

        // Generate auth token
        let token = generate_token();
        let token_hash = hash_token(&token);

        // Create device
        let device_type = request
            .device_type
            .as_deref()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DeviceType::Unknown);

        let device = Device::new(request.device_name, device_type, token_hash);
        let device_id = device.id.to_string();

        // Save device
        self.storage.save_device(device).await?;

        // Remove used session
        {
            let mut sessions = self.sessions.write().await;
            sessions.remove(&request.session_id);
        }

        info!("Device {} paired successfully", device_id);

        Ok(PairingVerifyResponse { device_id, token })
    }

    /// Get QR code data for a session
    pub async fn get_qr_data(&self, session_id: &str) -> Option<QrCodeData> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).map(|s| {
            // Truncate fingerprint to first 20 chars for QR code
            let fp = self.cert_fingerprint.as_ref().map(|f| {
                if f.len() > 20 {
                    f[..20].to_string()
                } else {
                    f.clone()
                }
            });

            QrCodeData {
                url: self.server_url.clone(),
                pin: s.pin.clone(),
                session_id: s.session_id.clone(),
                fingerprint: fp,
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }
        })
    }

    /// Validate an auth token and return the device
    pub async fn validate_token(&self, token: &str) -> PairingResult<Device> {
        let token_hash = hash_token(token);
        self.storage
            .get_device_by_token_hash(&token_hash)
            .await
            .ok_or(PairingError::InvalidToken)
    }

    /// Update last_seen for a device
    pub async fn touch_device(&self, token: &str) -> PairingResult<()> {
        let device = self.validate_token(token).await?;
        self.storage.touch_device(&device.id).await?;
        Ok(())
    }

    /// List all paired devices
    pub async fn list_devices(&self) -> Vec<Device> {
        self.storage.list_devices().await
    }

    /// Revoke a device by ID
    pub async fn revoke_device(&self, device_id: &str) -> StorageResult<()> {
        let id = crate::device::DeviceId::parse(device_id)
            .map_err(|_| crate::storage::StorageError::NotFound(device_id.to_string()))?;
        self.storage.remove_device(&id).await
    }

    /// Check if any devices are currently paired
    pub async fn has_paired_devices(&self) -> bool {
        self.storage.has_devices().await
    }

    /// Get session info for display
    pub async fn get_session_info(&self, session_id: &str) -> Option<(String, i64)> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).map(|s| {
            let remaining = (s.expires_at - Utc::now()).num_seconds().max(0);
            (s.pin.clone(), remaining)
        })
    }
}

/// Generate a secure random token
fn generate_token() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    BASE64.encode(bytes)
}

/// Hash a token for storage
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let result = hasher.finalize();
    BASE64.encode(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{tempdir, TempDir};

    async fn create_test_manager() -> (PairingManager, TempDir) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_devices.json");
        let storage = Arc::new(DeviceStorage::with_path(path).await.unwrap());
        let manager = PairingManager::new(storage, "https://localhost:8443".to_string());
        (manager, dir)
    }

    #[tokio::test]
    async fn test_pairing_flow() {
        let (manager, _dir) = create_test_manager().await;

        // Start pairing
        let start = manager.start_pairing().await;
        assert_eq!(start.pin.len(), 6);
        assert_eq!(start.expires_in, PIN_VALIDITY_SECONDS);

        // Verify PIN
        let request = PairingVerifyRequest {
            session_id: start.session_id,
            pin: start.pin,
            device_name: "Test Device".to_string(),
            device_type: Some("browser".to_string()),
        };

        let response = manager.verify_pin(request).await.unwrap();
        assert!(!response.device_id.is_empty());
        assert!(!response.token.is_empty());

        // Validate token
        let device = manager.validate_token(&response.token).await.unwrap();
        assert_eq!(device.name, "Test Device");
    }

    #[tokio::test]
    async fn test_invalid_pin() {
        let (manager, _dir) = create_test_manager().await;

        let start = manager.start_pairing().await;

        let request = PairingVerifyRequest {
            session_id: start.session_id,
            pin: "000000".to_string(), // Wrong PIN
            device_name: "Test".to_string(),
            device_type: None,
        };

        let result = manager.verify_pin(request).await;
        assert!(matches!(result, Err(PairingError::InvalidPin)));
    }

    #[tokio::test]
    async fn test_session_not_found() {
        let (manager, _dir) = create_test_manager().await;

        let request = PairingVerifyRequest {
            session_id: "nonexistent".to_string(),
            pin: "123456".to_string(),
            device_name: "Test".to_string(),
            device_type: None,
        };

        let result = manager.verify_pin(request).await;
        assert!(matches!(result, Err(PairingError::SessionNotFound)));
    }

    #[test]
    fn test_token_hashing() {
        let token = "test_token_123";
        let hash1 = hash_token(token);
        let hash2 = hash_token(token);
        assert_eq!(hash1, hash2);

        let different_hash = hash_token("different_token");
        assert_ne!(hash1, different_hash);
    }
}
