//! WebSocket handlers for video streaming and input
//!
//! Supports token-based authentication for secure connections.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::{header, StatusCode},
    response::IntoResponse,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use futures::{SinkExt, StreamExt};
use linglide_core::protocol::{InputEvent, ServerMessage};
use serde::Deserialize;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::broadcast::AppState;

/// Query parameters for WebSocket connections
#[derive(Debug, Deserialize)]
pub struct WsQuery {
    /// Authentication token (from pairing)
    #[serde(default)]
    token: Option<String>,
}

/// Extract token from query or Authorization header
fn extract_token(query: &WsQuery, headers: &axum::http::HeaderMap) -> Option<String> {
    // Try query parameter first
    if let Some(token) = &query.token {
        return Some(token.clone());
    }

    // Try Authorization header (Bearer token)
    if let Some(auth) = headers.get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                return Some(token.to_string());
            }
        }
    }

    None
}

/// WebSocket handler for video streaming
pub async fn video_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Query(query): Query<WsQuery>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    // Validate token if auth is required
    if state.auth_required {
        let token = match extract_token(&query, &headers) {
            Some(t) => t,
            None => {
                warn!("Video WebSocket connection rejected: no token provided");
                return (StatusCode::UNAUTHORIZED, "Authentication required").into_response();
            }
        };

        if !state.validate_token(&token).await {
            warn!("Video WebSocket connection rejected: invalid token");
            return (StatusCode::UNAUTHORIZED, "Invalid token").into_response();
        }

        // Update device last_seen
        let _ = state.pairing_manager.touch_device(&token).await;
    }

    ws.on_upgrade(|socket| handle_video_socket(socket, state))
        .into_response()
}

/// WebSocket handler for input events
pub async fn input_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Query(query): Query<WsQuery>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    info!("Input WebSocket upgrade requested");

    // Validate token if auth is required
    if state.auth_required {
        let token = match extract_token(&query, &headers) {
            Some(t) => t,
            None => {
                warn!("Input WebSocket connection rejected: no token provided");
                return (StatusCode::UNAUTHORIZED, "Authentication required").into_response();
            }
        };

        if !state.validate_token(&token).await {
            warn!("Input WebSocket connection rejected: invalid token");
            return (StatusCode::UNAUTHORIZED, "Invalid token").into_response();
        }

        // Update device last_seen
        let _ = state.pairing_manager.touch_device(&token).await;
    }

    info!("Input WebSocket: upgrading connection");
    ws.on_upgrade(|socket| handle_input_socket(socket, state))
        .into_response()
}

/// Handle video WebSocket connection
pub async fn handle_video_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    info!("Video client connected");

    // Subscribe to video segments
    let mut segment_rx = state.video_tx.subscribe();

    // Send init message with display configuration and codec info
    let (codec, codec_data) = if let Some(config) = state.get_codec_config() {
        (
            Some(config.codec_string),
            Some(BASE64.encode(&config.avcc_data)),
        )
    } else {
        (None, None)
    };

    let init_msg = ServerMessage::Init {
        width: state.config.width,
        height: state.config.height,
        fps: state.config.fps,
        codec,
        codec_data,
    };

    if let Ok(json) = serde_json::to_string(&init_msg) {
        debug!("Sending init: {}", json);
        if sender.send(Message::Text(json)).await.is_err() {
            warn!("Failed to send init message");
            return;
        }
    }

    // Send ready message
    let ready_msg = ServerMessage::Ready;
    if let Ok(json) = serde_json::to_string(&ready_msg) {
        if sender.send(Message::Text(json)).await.is_err() {
            warn!("Failed to send ready message");
            return;
        }
    }

    // Send init segment (fMP4 moov box) if available
    if let Some(init_segment) = state.get_init_segment() {
        debug!("Sending init segment: {} bytes", init_segment.len());
        if sender.send(Message::Binary(init_segment)).await.is_err() {
            warn!("Failed to send init segment");
            return;
        }
    } else {
        debug!("No init segment available yet");
    }

    // Send most recent keyframe segment so client can start decoding immediately
    if let Some(keyframe_segment) = state.get_keyframe_segment() {
        debug!("Sending keyframe segment: {} bytes", keyframe_segment.len());
        if sender
            .send(Message::Binary(keyframe_segment))
            .await
            .is_err()
        {
            warn!("Failed to send keyframe segment");
            return;
        }
    } else {
        debug!("No keyframe segment available yet");
    }

    // Spawn receiver task to handle client messages
    let recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Close(_)) => break,
                Ok(Message::Ping(_)) => {
                    debug!("Received ping");
                }
                Ok(Message::Text(text)) => {
                    debug!("Received text message: {}", text);
                }
                Err(e) => {
                    warn!("WebSocket receive error: {}", e);
                    break;
                }
                _ => {}
            }
        }
    });

    // Send video segments
    let mut frames_sent = 0u64;
    loop {
        tokio::select! {
            result = segment_rx.recv() => {
                match result {
                    Ok(segment) => {
                        frames_sent += 1;
                        if frames_sent <= 5 || frames_sent.is_multiple_of(100) {
                            debug!("Sending segment {} to client: {} bytes", frames_sent, segment.data.len());
                        }
                        if sender.send(Message::Binary(segment.data)).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!("Video client lagged {} frames", n);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(30)) => {
                // Send ping for keepalive
                let ping_msg = ServerMessage::Ping {
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64,
                };
                if let Ok(json) = serde_json::to_string(&ping_msg) {
                    if sender.send(Message::Text(json)).await.is_err() {
                        break;
                    }
                }
            }
        }
    }

    recv_task.abort();
    info!("Video client disconnected");
}

/// Handle input WebSocket connection
pub async fn handle_input_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    info!("Input client connected successfully");

    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => match serde_json::from_str::<InputEvent>(&text) {
                Ok(event) => {
                    info!("Input event received: {:?}", event);
                    if state.input_tx.send(event).await.is_err() {
                        warn!("Input channel closed");
                        break;
                    }
                }
                Err(e) => {
                    warn!("Invalid input event: {} - raw: {}", e, text);
                }
            },
            Ok(Message::Close(_)) => break,
            Ok(Message::Ping(data)) => {
                if sender.send(Message::Pong(data)).await.is_err() {
                    break;
                }
            }
            Err(e) => {
                warn!("WebSocket receive error: {}", e);
                break;
            }
            _ => {}
        }
    }

    info!("Input client disconnected");
}
