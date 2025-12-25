//! Async encoding pipeline

use crate::{H264Encoder, Fmp4Muxer};
use linglide_core::Result;
use linglide_capture::Frame;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, info, warn};

/// Encoded segment ready for streaming
#[derive(Clone)]
pub struct StreamSegment {
    /// The segment data
    pub data: Vec<u8>,
    /// Whether this is an initialization segment
    pub is_init: bool,
    /// Whether this contains a keyframe
    pub is_keyframe: bool,
    /// Sequence number
    pub sequence: u64,
}

/// Async encoding pipeline that processes frames and produces stream segments
pub struct EncodingPipeline {
    encoder: H264Encoder,
    muxer: Fmp4Muxer,
    frame_duration: u32,
    init_segment: Option<Vec<u8>>,
}

impl EncodingPipeline {
    /// Create a new encoding pipeline
    pub fn new(width: u32, height: u32, fps: u32, bitrate: u32) -> Result<Self> {
        let mut encoder = H264Encoder::new(width, height, fps, bitrate)?;
        let mut muxer = Fmp4Muxer::new(width, height, fps);

        // Get and parse headers
        let headers = encoder.get_headers()?;
        muxer.set_headers(&headers);

        // Pre-generate init segment
        let init_segment = muxer.create_init_segment();

        // Frame duration in timescale units
        let frame_duration = (fps * 1000) / fps; // timescale / fps

        info!("Encoding pipeline initialized");

        Ok(Self {
            encoder,
            muxer,
            frame_duration,
            init_segment: Some(init_segment),
        })
    }

    /// Get the initialization segment (call once per client)
    pub fn get_init_segment(&self) -> Option<Vec<u8>> {
        self.init_segment.clone()
    }

    /// Get the codec string for WebCodecs
    pub fn get_codec_string(&self) -> String {
        self.muxer.get_codec_string()
    }

    /// Get the avcC data for WebCodecs description
    pub fn get_avcc_data(&self) -> Vec<u8> {
        self.muxer.get_avcc_data()
    }

    /// Encode a frame and return the media segment
    pub fn encode_frame(&mut self, frame: &Frame) -> Result<StreamSegment> {
        let encoded = self.encoder.encode(frame.data())?;
        let is_keyframe = encoded.is_keyframe;
        let segment_data = self.muxer.create_media_segment(&encoded, self.frame_duration);

        Ok(StreamSegment {
            data: segment_data,
            is_init: false,
            is_keyframe,
            sequence: frame.sequence,
        })
    }

    /// Run the pipeline as an async task
    pub async fn run(
        mut self,
        mut frame_rx: mpsc::Receiver<Frame>,
        segment_tx: broadcast::Sender<StreamSegment>,
    ) {
        info!("Encoding pipeline started");

        // Note: init segment should be retrieved via get_init_segment() and sent to clients separately
        // We no longer broadcast it here since clients may not be connected yet

        while let Some(frame) = frame_rx.recv().await {
            match self.encode_frame(&frame) {
                Ok(segment) => {
                    debug!("Encoded segment: {} bytes", segment.data.len());
                    if segment_tx.send(segment).is_err() {
                        debug!("No receivers for segment");
                    }
                }
                Err(e) => {
                    warn!("Encoding error: {}", e);
                }
            }
        }

        info!("Encoding pipeline stopped");
    }
}
