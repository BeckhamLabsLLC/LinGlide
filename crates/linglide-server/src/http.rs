//! HTTP request handlers
//!
//! Includes static file serving and authentication API endpoints.

use axum::{
    extract::{Path as AxumPath, Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use image::ImageFormat;
use linglide_auth::{DeviceInfo, PairingStartResponse, PairingVerifyRequest, PairingVerifyResponse};
use linglide_discovery::DiscoveryInfo;
use linglide_web::Assets;
use qrcode::QrCode;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::net::UdpSocket;
use std::sync::Arc;
use tracing::debug;

use crate::broadcast::AppState;

/// Create the main application router
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Static files
        .route("/", get(index_handler))
        .route("/assets/*path", get(asset_handler))
        .route("/*path", get(static_handler))
        // WebSocket endpoints
        .route("/ws/video", get(crate::websocket::video_ws_handler))
        .route("/ws/input", get(crate::websocket::input_ws_handler))
        // Pairing API
        .route("/api/pair/start", post(pair_start_handler))
        .route("/api/pair/verify", post(pair_verify_handler))
        .route("/api/pair/qr", get(pair_qr_handler))
        .route("/api/pair/status", get(pair_status_handler))
        // Device management API
        .route("/api/devices", get(list_devices_handler))
        .route("/api/devices/:id", delete(revoke_device_handler))
        // Server info
        .route("/api/info", get(server_info_handler))
        .route("/api/discovery", get(discovery_handler))
        .with_state(state)
}

/// Serve the main index page
async fn index_handler() -> impl IntoResponse {
    match Assets::get("index.html") {
        Some(content) => Html(content.data.to_vec()).into_response(),
        None => (StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}

/// Serve static assets with proper content types
async fn static_handler(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> impl IntoResponse {
    serve_asset(&path)
}

/// Serve assets from /assets/ path
async fn asset_handler(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> impl IntoResponse {
    serve_asset(&path)
}

fn serve_asset(path: &str) -> Response {
    let path = path.trim_start_matches('/');

    debug!("Serving asset: {}", path);

    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path)
                .first_or_octet_stream()
                .to_string();

            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, mime)],
                content.data.to_vec(),
            )
                .into_response()
        }
        None => (StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}

// ============================================================================
// Pairing API Handlers
// ============================================================================

/// Start a new pairing session
///
/// Returns a 6-digit PIN and session ID. The PIN is valid for 60 seconds.
async fn pair_start_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<PairingStartResponse>, (StatusCode, String)> {
    let response = state.pairing_manager.start_pairing().await;
    Ok(Json(response))
}

/// Verify a pairing PIN and complete device registration
async fn pair_verify_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<PairingVerifyRequest>,
) -> Result<Json<PairingVerifyResponse>, (StatusCode, String)> {
    state
        .pairing_manager
        .verify_pin(request)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))
}

/// Query parameters for QR code generation
#[derive(Debug, Deserialize)]
pub struct QrQuery {
    /// Session ID from pair/start
    session_id: String,
    /// QR code size in pixels (default 200)
    #[serde(default = "default_qr_size")]
    size: u32,
}

fn default_qr_size() -> u32 {
    200
}

/// Generate a QR code image for pairing
///
/// The QR code contains: `linglide://pair?url=<server>&pin=<pin>&session=<id>`
async fn pair_qr_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<QrQuery>,
) -> Result<Response, (StatusCode, String)> {
    // Get session data
    let qr_data = state
        .pairing_manager
        .get_qr_data(&query.session_id)
        .await
        .ok_or((StatusCode::NOT_FOUND, "Session not found".to_string()))?;

    // Create pairing URL with enhanced fields
    let mut pairing_url = format!(
        "linglide://pair?url={}&pin={}&session={}",
        urlencoding(&qr_data.url),
        qr_data.pin,
        qr_data.session_id
    );

    // Add fingerprint if available
    if let Some(ref fp) = qr_data.fingerprint {
        pairing_url.push_str(&format!("&fp={}", fp));
    }

    // Add version if available
    if let Some(ref version) = qr_data.version {
        pairing_url.push_str(&format!("&v={}", version));
    }

    // Generate QR code
    let code = QrCode::new(pairing_url.as_bytes())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let image = code.render::<image::Luma<u8>>().build();

    // Resize if needed
    let resized = image::imageops::resize(
        &image,
        query.size,
        query.size,
        image::imageops::FilterType::Nearest,
    );

    // Encode as PNG
    let mut buffer = Cursor::new(Vec::new());
    resized
        .write_to(&mut buffer, ImageFormat::Png)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "image/png")],
        buffer.into_inner(),
    )
        .into_response())
}

/// Simple URL encoding for the pairing URL
fn urlencoding(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}

/// Response for pairing status check
#[derive(Debug, Serialize)]
pub struct PairingStatusResponse {
    /// Whether the session is still valid
    pub valid: bool,
    /// The PIN (for display)
    pub pin: Option<String>,
    /// Seconds remaining until expiration
    pub expires_in: i64,
}

/// Check pairing session status
async fn pair_status_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SessionQuery>,
) -> Json<PairingStatusResponse> {
    match state.pairing_manager.get_session_info(&query.session_id).await {
        Some((pin, expires_in)) => Json(PairingStatusResponse {
            valid: true,
            pin: Some(pin),
            expires_in,
        }),
        None => Json(PairingStatusResponse {
            valid: false,
            pin: None,
            expires_in: 0,
        }),
    }
}

#[derive(Debug, Deserialize)]
pub struct SessionQuery {
    session_id: String,
}

// ============================================================================
// Device Management Handlers
// ============================================================================

/// List all paired devices
async fn list_devices_handler(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<DeviceInfo>> {
    let devices = state.pairing_manager.list_devices().await;
    let infos: Vec<DeviceInfo> = devices.iter().map(DeviceInfo::from).collect();
    Json(infos)
}

/// Revoke (unpair) a device
async fn revoke_device_handler(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .pairing_manager
        .revoke_device(&id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))
}

// ============================================================================
// Server Info
// ============================================================================

/// Server information response
#[derive(Debug, Serialize)]
pub struct ServerInfo {
    /// Server version
    pub version: String,
    /// Display width
    pub width: u32,
    /// Display height
    pub height: u32,
    /// Target FPS
    pub fps: u32,
    /// Whether authentication is required
    pub auth_required: bool,
    /// Number of paired devices
    pub paired_devices: usize,
    /// Certificate fingerprint (for verification)
    pub cert_fingerprint: Option<String>,
}

/// Get server information
async fn server_info_handler(State(state): State<Arc<AppState>>) -> Json<ServerInfo> {
    let paired_count = state.pairing_manager.list_devices().await.len();

    Json(ServerInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        width: state.config.width,
        height: state.config.height,
        fps: state.config.fps,
        auth_required: state.auth_required,
        paired_devices: paired_count,
        cert_fingerprint: state.cert_fingerprint.clone(),
    })
}

// ============================================================================
// Discovery
// ============================================================================

/// Get discovery information for mDNS/network discovery
///
/// Returns service type, instance name, port, fingerprint, and available addresses.
async fn discovery_handler(State(state): State<Arc<AppState>>) -> Json<DiscoveryInfo> {
    // Get local IP addresses
    let addresses = get_local_addresses();

    // Get hostname for instance name
    let hostname = hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".to_string());

    let instance_name = format!("LinGlide-{}", hostname);

    Json(DiscoveryInfo::new(
        instance_name,
        state.config.port,
        state.cert_fingerprint.clone(),
        addresses,
        env!("CARGO_PKG_VERSION").to_string(),
    ))
}

/// Get local IP addresses for the machine
fn get_local_addresses() -> Vec<String> {
    let mut addresses = Vec::new();

    // Try to get the primary local IP by connecting to an external address
    if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = socket.local_addr() {
                addresses.push(addr.ip().to_string());
            }
        }
    }

    // Always include localhost
    addresses.push("127.0.0.1".to_string());

    addresses
}
