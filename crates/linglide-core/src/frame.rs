//! Frame representation for captured screen data
//!
//! This module provides the common Frame type used by both capture and encoder crates.

use std::sync::Arc;

/// Represents a captured frame from the screen
#[derive(Clone)]
pub struct Frame {
    /// Raw pixel data in BGRA format
    data: Arc<Vec<u8>>,
    /// Frame width
    pub width: u32,
    /// Frame height
    pub height: u32,
    /// Frame sequence number
    pub sequence: u64,
    /// Timestamp in microseconds
    pub timestamp_us: u64,
}

impl Frame {
    /// Create a new frame from BGRA pixel data
    pub fn new(data: Vec<u8>, width: u32, height: u32, sequence: u64) -> Self {
        let timestamp_us = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_micros() as u64)
            .unwrap_or(0);

        Self {
            data: Arc::new(data),
            width,
            height,
            sequence,
            timestamp_us,
        }
    }

    /// Get the raw pixel data as a slice
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Get the number of bytes per row (stride)
    pub fn stride(&self) -> usize {
        (self.width * 4) as usize
    }

    /// Get total size in bytes
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Check if frame dimensions are valid
    pub fn is_valid(&self) -> bool {
        let expected_size = (self.width * self.height * 4) as usize;
        self.data.len() >= expected_size && self.width > 0 && self.height > 0
    }
}

impl std::fmt::Debug for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Frame")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("sequence", &self.sequence)
            .field("size", &self.data.len())
            .finish()
    }
}
