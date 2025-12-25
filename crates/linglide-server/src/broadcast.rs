//! Broadcast channel management for video frames and state

use linglide_auth::PairingManager;
use linglide_core::{protocol::InputEvent, Config};
use linglide_encoder::pipeline::StreamSegment;
use std::sync::{Arc, RwLock};
use tokio::sync::{broadcast, mpsc};

/// Codec configuration for WebCodecs
pub struct CodecConfig {
    pub codec_string: String,
    pub avcc_data: Vec<u8>,
}

/// Shared application state
pub struct AppState {
    /// Configuration
    pub config: Config,
    /// Video segment broadcast sender
    pub video_tx: broadcast::Sender<StreamSegment>,
    /// Input event sender
    pub input_tx: mpsc::Sender<InputEvent>,
    /// fMP4 init segment (moov box with codec config)
    pub init_segment: RwLock<Option<Vec<u8>>>,
    /// Codec configuration for WebCodecs
    pub codec_config: RwLock<Option<CodecConfig>>,
    /// Most recent keyframe segment (for new clients)
    pub keyframe_segment: RwLock<Option<Vec<u8>>>,
    /// Pairing manager for device authentication
    pub pairing_manager: Arc<PairingManager>,
    /// Whether authentication is required for connections
    pub auth_required: bool,
    /// Certificate fingerprint for verification
    pub cert_fingerprint: Option<String>,
}

impl AppState {
    /// Create a new application state
    pub fn new(
        config: Config,
        video_tx: broadcast::Sender<StreamSegment>,
        input_tx: mpsc::Sender<InputEvent>,
        pairing_manager: Arc<PairingManager>,
        auth_required: bool,
        cert_fingerprint: Option<String>,
    ) -> Self {
        Self {
            config,
            video_tx,
            input_tx,
            init_segment: RwLock::new(None),
            codec_config: RwLock::new(None),
            keyframe_segment: RwLock::new(None),
            pairing_manager,
            auth_required,
            cert_fingerprint,
        }
    }

    /// Set the init segment
    pub fn set_init_segment(&self, segment: Vec<u8>) {
        if let Ok(mut guard) = self.init_segment.write() {
            *guard = Some(segment);
        }
    }

    /// Get the init segment
    pub fn get_init_segment(&self) -> Option<Vec<u8>> {
        self.init_segment.read().ok().and_then(|g| g.clone())
    }

    /// Set the codec configuration
    pub fn set_codec_config(&self, codec_string: String, avcc_data: Vec<u8>) {
        if let Ok(mut guard) = self.codec_config.write() {
            *guard = Some(CodecConfig {
                codec_string,
                avcc_data,
            });
        }
    }

    /// Get the codec configuration
    pub fn get_codec_config(&self) -> Option<CodecConfig> {
        self.codec_config.read().ok().and_then(|g| {
            g.as_ref().map(|c| CodecConfig {
                codec_string: c.codec_string.clone(),
                avcc_data: c.avcc_data.clone(),
            })
        })
    }

    /// Set the most recent keyframe segment
    pub fn set_keyframe_segment(&self, segment: Vec<u8>) {
        if let Ok(mut guard) = self.keyframe_segment.write() {
            *guard = Some(segment);
        }
    }

    /// Get the most recent keyframe segment
    pub fn get_keyframe_segment(&self) -> Option<Vec<u8>> {
        self.keyframe_segment.read().ok().and_then(|g| g.clone())
    }

    /// Validate an authentication token
    pub async fn validate_token(&self, token: &str) -> bool {
        if !self.auth_required {
            return true;
        }
        self.pairing_manager.validate_token(token).await.is_ok()
    }
}
