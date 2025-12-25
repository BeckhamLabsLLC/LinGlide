//! H.264 encoder using x264

use linglide_core::{Error, Result};
use tracing::{debug, info};
use x264::{Colorspace, Encoder, Image, Plane, Setup, Tune};

/// H.264 encoder wrapper with low-latency settings
pub struct H264Encoder {
    encoder: Encoder,
    width: u32,
    height: u32,
    frame_count: i64,
    yuv_buffer: Vec<u8>,
}

impl H264Encoder {
    /// Create a new H.264 encoder
    pub fn new(width: u32, height: u32, fps: u32, bitrate: u32) -> Result<Self> {
        // Use Tune::None with zero_latency=true for low-latency streaming
        let encoder = Setup::preset(x264::Preset::Superfast, Tune::None, false, true)
            .fps(fps, 1)
            .bitrate(bitrate as i32)
            .build(Colorspace::I420, width as i32, height as i32)
            .map_err(|_| Error::EncoderError("Failed to create encoder".to_string()))?;

        // Pre-allocate YUV buffer (I420 format: Y + U/4 + V/4)
        let yuv_size = (width * height * 3 / 2) as usize;
        let yuv_buffer = vec![0u8; yuv_size];

        info!(
            "H.264 encoder initialized: {}x{} @ {} fps, {} kbps",
            width, height, fps, bitrate
        );

        Ok(Self {
            encoder,
            width,
            height,
            frame_count: 0,
            yuv_buffer,
        })
    }

    /// Convert BGRA to YUV420 (I420) format
    fn bgra_to_yuv420(&mut self, bgra: &[u8]) {
        let width = self.width as usize;
        let height = self.height as usize;
        let y_size = width * height;
        let uv_size = y_size / 4;

        // Split buffer into planes safely
        let (y_plane, uv_planes) = self.yuv_buffer.split_at_mut(y_size);
        let (u_plane, v_plane) = uv_planes.split_at_mut(uv_size);

        // Convert each pixel
        for y in 0..height {
            for x in 0..width {
                let bgra_idx = (y * width + x) * 4;
                if bgra_idx + 2 >= bgra.len() {
                    continue;
                }
                let b = bgra[bgra_idx] as i32;
                let g = bgra[bgra_idx + 1] as i32;
                let r = bgra[bgra_idx + 2] as i32;

                // BT.601 conversion
                let y_val = ((66 * r + 129 * g + 25 * b + 128) >> 8) + 16;
                y_plane[y * width + x] = y_val.clamp(0, 255) as u8;

                // Subsample U and V (2x2 blocks)
                if y % 2 == 0 && x % 2 == 0 {
                    let uv_idx = (y / 2) * (width / 2) + (x / 2);
                    let u_val = ((-38 * r - 74 * g + 112 * b + 128) >> 8) + 128;
                    let v_val = ((112 * r - 94 * g - 18 * b + 128) >> 8) + 128;
                    u_plane[uv_idx] = u_val.clamp(0, 255) as u8;
                    v_plane[uv_idx] = v_val.clamp(0, 255) as u8;
                }
            }
        }
    }

    /// Encode a frame from BGRA data
    pub fn encode(&mut self, bgra: &[u8]) -> Result<EncodedFrame> {
        // Convert BGRA to YUV420
        self.bgra_to_yuv420(bgra);

        let width = self.width as usize;
        let height = self.height as usize;
        let y_size = width * height;
        let uv_size = y_size / 4;

        // Create picture with plane data
        let y_stride = width as i32;
        let uv_stride = (width / 2) as i32;

        let image = Image::new(
            Colorspace::I420,
            self.width as i32,
            self.height as i32,
            &[
                Plane {
                    stride: y_stride,
                    data: &self.yuv_buffer[0..y_size],
                },
                Plane {
                    stride: uv_stride,
                    data: &self.yuv_buffer[y_size..y_size + uv_size],
                },
                Plane {
                    stride: uv_stride,
                    data: &self.yuv_buffer[y_size + uv_size..],
                },
            ],
        );

        let pts = self.frame_count;

        // Encode the frame
        let (data, _pic) = self
            .encoder
            .encode(pts, image)
            .map_err(|_| Error::EncoderError("Encoding failed".to_string()))?;

        let bytes = data.entirety().to_vec();
        let is_keyframe = self.check_keyframe(&bytes);

        // Debug first frame to understand NAL format
        if self.frame_count == 0 {
            let preview: Vec<String> = bytes
                .iter()
                .take(32)
                .map(|b| format!("{:02x}", b))
                .collect();
            debug!("First frame NAL preview: {}", preview.join(" "));
        }

        debug!(
            "Encoded frame {}: {} bytes, keyframe={}",
            self.frame_count,
            bytes.len(),
            is_keyframe
        );

        let frame = EncodedFrame {
            data: bytes,
            pts,
            is_keyframe,
        };

        self.frame_count += 1;

        Ok(frame)
    }

    /// Check if NAL data contains a keyframe
    fn check_keyframe(&self, bytes: &[u8]) -> bool {
        let mut nal_types = Vec::new();
        let mut has_idr = false;
        let mut has_sps = false;

        // Look for NAL units with 4-byte start code
        for i in 0..bytes.len().saturating_sub(4) {
            if bytes[i] == 0
                && bytes[i + 1] == 0
                && bytes[i + 2] == 0
                && bytes[i + 3] == 1
                && i + 4 < bytes.len()
            {
                let nal_type = bytes[i + 4] & 0x1F;
                nal_types.push(nal_type);
                if nal_type == 5 {
                    has_idr = true;
                }
                if nal_type == 7 {
                    has_sps = true;
                }
            }
        }

        // Also check 3-byte start codes
        for i in 0..bytes.len().saturating_sub(3) {
            // Make sure this isn't part of a 4-byte start code
            if bytes[i] == 0
                && bytes[i + 1] == 0
                && bytes[i + 2] == 1
                && (i == 0 || bytes[i - 1] != 0)
                && i + 3 < bytes.len()
            {
                let nal_type = bytes[i + 3] & 0x1F;
                if !nal_types.contains(&nal_type) {
                    nal_types.push(nal_type);
                }
                if nal_type == 5 {
                    has_idr = true;
                }
                if nal_type == 7 {
                    has_sps = true;
                }
            }
        }

        if self.frame_count == 0 || (has_idr && self.frame_count > 0) {
            debug!("Frame {} NAL types: {:?}", self.frame_count, nal_types);
        }

        // Frame is a keyframe if it has SPS (usually means IDR follows) or explicit IDR
        has_idr || has_sps
    }

    /// Get encoder headers (SPS/PPS)
    pub fn get_headers(&mut self) -> Result<Vec<u8>> {
        let headers = self
            .encoder
            .headers()
            .map_err(|_| Error::EncoderError("Failed to get headers".to_string()))?;
        Ok(headers.entirety().to_vec())
    }

    /// Get frame count
    pub fn frame_count(&self) -> i64 {
        self.frame_count
    }
}

/// Represents an encoded video frame
#[derive(Clone)]
pub struct EncodedFrame {
    /// Encoded NAL data
    pub data: Vec<u8>,
    /// Presentation timestamp
    pub pts: i64,
    /// Whether this is a keyframe
    pub is_keyframe: bool,
}

impl std::fmt::Debug for EncodedFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EncodedFrame")
            .field("size", &self.data.len())
            .field("pts", &self.pts)
            .field("is_keyframe", &self.is_keyframe)
            .finish()
    }
}
