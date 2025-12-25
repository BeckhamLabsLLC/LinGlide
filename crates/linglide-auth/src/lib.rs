//! LinGlide Auth - Device pairing and authentication
//!
//! Provides secure device pairing via PIN/QR codes and token-based authentication
//! for WebSocket connections.
//!
//! # Pairing Flow
//!
//! 1. Server calls `PairingManager::start_pairing()` to generate a 6-digit PIN
//! 2. PIN is displayed on server (or encoded in QR code)
//! 3. Client enters PIN and device info via `POST /api/pair/verify`
//! 4. Upon success, client receives an auth token
//! 5. Client uses token for WebSocket connections via `Authorization` header
//!
//! # Example
//!
//! ```no_run
//! use linglide_auth::{PairingManager, DeviceStorage};
//! use std::sync::Arc;
//!
//! async fn example() {
//!     let storage = Arc::new(DeviceStorage::new().await.unwrap());
//!     let manager = PairingManager::new(storage, "https://192.168.1.100:8443".to_string());
//!
//!     // Start pairing session
//!     let session = manager.start_pairing().await;
//!     println!("Enter PIN on device: {}", session.pin);
//!
//!     // Later, when validating a WebSocket connection
//!     let token = "..."; // From client header
//!     if let Ok(device) = manager.validate_token(token).await {
//!         println!("Device {} connected", device.name);
//!     }
//! }
//! ```

pub mod device;
pub mod pairing;
pub mod storage;

pub use device::{Device, DeviceId, DeviceInfo, DeviceType};
pub use pairing::{
    hash_token, PairingError, PairingManager, PairingResult, PairingStartResponse,
    PairingVerifyRequest, PairingVerifyResponse, QrCodeData, PIN_VALIDITY_SECONDS,
};
pub use storage::{DeviceStorage, StorageError, StorageResult};
