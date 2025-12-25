//! LinGlide Encoder - H.264 video encoding
//!
//! This crate provides low-latency H.264 encoding using x264.

pub mod encoder;
pub mod fmp4;
pub mod pipeline;

pub use encoder::H264Encoder;
pub use fmp4::Fmp4Muxer;
pub use pipeline::EncodingPipeline;
