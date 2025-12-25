//! LinGlide Server - Axum-based HTTP and WebSocket server
//!
//! This crate provides the web server for serving the viewer and handling input.

pub mod broadcast;
pub mod http;
pub mod tls;
pub mod websocket;

pub use http::create_router;
pub use tls::{
    calculate_cert_fingerprint, create_rustls_config, create_rustls_config_from_files,
    generate_self_signed_cert, CertificateManager,
};
pub use websocket::{handle_input_socket, handle_video_socket};
