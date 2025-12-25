//! LinGlide Server - Axum-based HTTP and WebSocket server
//!
//! This crate provides the web server for serving the viewer and handling input.

pub mod broadcast;
pub mod http;
pub mod tls;
pub mod websocket;

pub use http::create_router;
pub use tls::{
    generate_self_signed_cert, create_rustls_config, create_rustls_config_from_files,
    CertificateManager, calculate_cert_fingerprint,
};
pub use websocket::{handle_video_socket, handle_input_socket};
