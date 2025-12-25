//! Server Controller
//!
//! Manages the LinGlide server lifecycle and communicates with the UI via the bridge.

use crate::bridge::{AsyncBridge, UiCommand, UiEvent};
use anyhow::Result;
use linglide_auth::{DeviceStorage, PairingManager};
use linglide_capture::{Frame, ScreenCapture, VirtualDisplay};
use linglide_core::{Config, DisplayPosition};
use linglide_discovery::ServiceAdvertiser;
use linglide_encoder::pipeline::StreamSegment;
use linglide_encoder::EncodingPipeline;
use linglide_input::{mouse::RelativeMouse, VirtualMouse, VirtualStylus, VirtualTouchscreen};
use linglide_server::{
    broadcast::AppState, create_router, create_rustls_config, CertificateManager,
};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, oneshot, RwLock};
use tracing::{info, warn};

/// Server configuration
#[derive(Clone)]
#[allow(dead_code)]
pub struct ServerConfig {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub port: u16,
    pub bitrate: u32,
    pub mirror_mode: bool,
    pub position: DisplayPosition,
    pub enable_mdns: bool,
    pub enable_usb: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fps: 60,
            port: 8443,
            bitrate: 8000,
            mirror_mode: false,
            position: DisplayPosition::RightOf,
            enable_mdns: true,
            enable_usb: false,
        }
    }
}

/// Handle to stop a running server
struct ServerHandle {
    shutdown_tx: oneshot::Sender<()>,
}

/// Shared server context accessible during runtime
#[allow(dead_code)]
pub struct ServerContext {
    pub pairing_manager: Arc<PairingManager>,
    pub device_storage: Arc<DeviceStorage>,
    pub fingerprint: String,
}

/// Server controller that manages the LinGlide server
pub struct ServerController {
    bridge: AsyncBridge,
    config: ServerConfig,
    server_handle: Option<ServerHandle>,
    context: Option<Arc<RwLock<ServerContext>>>,
}

impl ServerController {
    pub fn new(bridge: AsyncBridge) -> Self {
        Self {
            bridge,
            config: ServerConfig::default(),
            server_handle: None,
            context: None,
        }
    }

    /// Run the controller - listens for commands and manages server
    pub async fn run(mut self) -> Result<()> {
        info!("Server controller started");

        while let Some(command) = self.bridge.command_rx.recv().await {
            match command {
                UiCommand::StartServer => {
                    if self.server_handle.is_none() {
                        self.start_server().await;
                    }
                }
                UiCommand::StopServer => {
                    self.stop_server().await;
                }
                UiCommand::StartPairing => {
                    self.start_pairing().await;
                }
                UiCommand::CancelPairing => {
                    // Pairing sessions expire automatically
                }
                UiCommand::RevokeDevice { device_id } => {
                    self.revoke_device(&device_id).await;
                }
                UiCommand::SetMdns { enabled: _ } => {
                    // Would need to restart server to change mDNS
                }
                UiCommand::SetUsb { enabled: _ } => {
                    // Would need to restart server to change USB
                }
                UiCommand::RefreshPin => {
                    self.refresh_pin().await;
                }
                UiCommand::Shutdown => {
                    info!("Shutdown requested");
                    self.stop_server().await;
                    break;
                }
            }
        }

        info!("Server controller stopped");
        Ok(())
    }

    async fn start_server(&mut self) {
        info!("Starting server...");

        let config = self.config.clone();
        let event_tx = self.bridge.event_tx.clone();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        // Get local IP and server URL
        let local_ip = get_local_ip().unwrap_or_else(|| "localhost".to_string());
        let server_url = format!("https://{}:{}", local_ip, config.port);

        // Setup TLS and get certificate fingerprint
        info!("Setting up TLS...");
        let cert_manager = match CertificateManager::new() {
            Ok(cm) => cm,
            Err(e) => {
                let _ = event_tx.send(UiEvent::ServerError {
                    message: format!("Failed to create cert manager: {}", e),
                });
                return;
            }
        };

        let hostnames = vec![local_ip.clone(), "localhost".to_string()];
        let (cert_pem, key_pem, fingerprint) = match cert_manager.load_or_generate(&hostnames) {
            Ok(certs) => certs,
            Err(e) => {
                let _ = event_tx.send(UiEvent::ServerError {
                    message: format!("Failed to load/generate cert: {}", e),
                });
                return;
            }
        };

        // Initialize device storage and pairing manager
        info!("Initializing device storage...");
        let device_storage = match DeviceStorage::new().await {
            Ok(ds) => Arc::new(ds),
            Err(e) => {
                let _ = event_tx.send(UiEvent::ServerError {
                    message: format!("Failed to initialize device storage: {}", e),
                });
                return;
            }
        };

        let pairing_manager = Arc::new(PairingManager::new(
            device_storage.clone(),
            server_url.clone(),
        ));
        let paired_devices = pairing_manager.list_devices().await;
        info!(
            "Authentication: ENABLED ({} paired devices)",
            paired_devices.len()
        );

        // Create shared context
        let context = Arc::new(RwLock::new(ServerContext {
            pairing_manager: pairing_manager.clone(),
            device_storage: device_storage.clone(),
            fingerprint: fingerprint.clone(),
        }));
        self.context = Some(context);

        // Spawn server task with pre-created resources
        let pm_clone = pairing_manager.clone();
        let ds_clone = device_storage.clone();
        let fp_clone = fingerprint.clone();
        let devices_clone = paired_devices.clone();
        let persistent_pin = pairing_manager.get_persistent_pin().await;
        tokio::spawn(async move {
            if let Err(e) = run_server(
                config,
                event_tx.clone(),
                shutdown_rx,
                pm_clone,
                ds_clone,
                cert_pem,
                key_pem,
                fp_clone,
                local_ip,
                devices_clone,
                persistent_pin,
            )
            .await
            {
                warn!("Server error: {}", e);
                let _ = event_tx.send(UiEvent::ServerError {
                    message: e.to_string(),
                });
            }
            let _ = event_tx.send(UiEvent::ServerStopped);
        });

        self.server_handle = Some(ServerHandle { shutdown_tx });
    }

    async fn stop_server(&mut self) {
        if let Some(handle) = self.server_handle.take() {
            info!("Stopping server...");
            let _ = handle.shutdown_tx.send(());
        }
        self.context = None;
    }

    async fn start_pairing(&mut self) {
        if let Some(ref ctx) = self.context {
            let ctx = ctx.read().await;
            let response = ctx.pairing_manager.start_pairing().await;
            let _ = self.bridge.event_tx.send(UiEvent::PairingStarted {
                session_id: response.session_id,
                pin: response.pin,
                expires_in: response.expires_in,
            });
        } else {
            warn!("Cannot start pairing: server not running");
        }
    }

    async fn revoke_device(&mut self, device_id: &str) {
        if let Some(ref ctx) = self.context {
            let ctx = ctx.read().await;
            if let Err(e) = ctx.pairing_manager.revoke_device(device_id).await {
                warn!("Failed to revoke device: {}", e);
            }
        }
    }

    async fn refresh_pin(&mut self) {
        if let Some(ref ctx) = self.context {
            let ctx = ctx.read().await;
            let new_pin = ctx.pairing_manager.refresh_persistent_pin().await;
            let _ = self
                .bridge
                .event_tx
                .send(UiEvent::PinRefreshed { pin: new_pin });
        } else {
            warn!("Cannot refresh PIN: server not running");
        }
    }
}

/// Get the local IP address
fn get_local_ip() -> Option<String> {
    use std::net::UdpSocket;
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let addr = socket.local_addr().ok()?;
    Some(addr.ip().to_string())
}

/// Run the actual server (based on main.rs logic)
#[allow(clippy::too_many_arguments)]
async fn run_server(
    config: ServerConfig,
    event_tx: broadcast::Sender<UiEvent>,
    mut shutdown_rx: oneshot::Receiver<()>,
    pairing_manager: Arc<PairingManager>,
    _device_storage: Arc<DeviceStorage>,
    cert_pem: String,
    key_pem: String,
    fingerprint: String,
    local_ip: String,
    paired_devices: Vec<linglide_auth::device::Device>,
    persistent_pin: String,
) -> Result<()> {
    let core_config = Config::new()
        .with_width(config.width)
        .with_height(config.height)
        .with_fps(config.fps)
        .with_port(config.port)
        .with_position(config.position)
        .with_bitrate(config.bitrate)
        .with_mirror_mode(config.mirror_mode);

    let use_evdi = !config.mirror_mode;
    let (offset_x, offset_y) = (0_i32, 0_i32);

    // Create channels
    let (frame_tx, frame_rx) = mpsc::channel::<Frame>(2);
    let (segment_tx, _segment_rx) = broadcast::channel::<StreamSegment>(16);
    let (input_tx, mut input_rx) = mpsc::channel(64);

    // Create input devices
    info!("Creating virtual input devices...");
    let mut touchscreen = VirtualTouchscreen::new(config.width, config.height, offset_x, offset_y)?;
    let mut mouse = VirtualMouse::new(config.width, config.height, offset_x, offset_y)?;
    let mut scroll_mouse = RelativeMouse::new()?;
    let mut stylus = VirtualStylus::new(config.width, config.height, offset_x, offset_y)?;

    // Create TLS config from provided certs
    let tls_config = create_rustls_config(&cert_pem, &key_pem)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create TLS config: {}", e))?;

    let server_url = format!("https://{}:{}", local_ip, config.port);

    // Create app state
    let state = Arc::new(AppState::new(
        core_config.clone(),
        segment_tx.clone(),
        input_tx,
        pairing_manager.clone(),
        true, // auth_required
        Some(fingerprint.clone()),
    ));

    // Create router
    let router = create_router(state.clone());

    // Check if port is available before proceeding
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], config.port));
    match std::net::TcpListener::bind(addr) {
        Ok(listener) => drop(listener), // Port is free, release it
        Err(e) => {
            return Err(anyhow::anyhow!(
                "Port {} is already in use: {}",
                config.port,
                e
            ));
        }
    }

    // Send server started event (port is available)
    let _ = event_tx.send(UiEvent::ServerStarted {
        url: server_url.clone(),
        fingerprint: fingerprint.clone(),
        paired_devices,
        pin: persistent_pin,
    });

    // Start mDNS if enabled
    let mut mdns_advertiser: Option<ServiceAdvertiser> = None;
    if config.enable_mdns {
        match ServiceAdvertiser::new(config.port, None) {
            Ok(mut advertiser) => {
                let addresses: Vec<IpAddr> = get_local_ip()
                    .and_then(|ip| ip.parse().ok())
                    .into_iter()
                    .collect();

                if advertiser
                    .start(
                        env!("CARGO_PKG_VERSION"),
                        Some(&fingerprint),
                        Some(addresses),
                    )
                    .is_ok()
                {
                    info!("mDNS: Advertising as '{}'", advertiser.instance_name());
                    let _ = event_tx.send(UiEvent::MdnsStatus { active: true });
                    mdns_advertiser = Some(advertiser);
                }
            }
            Err(e) => warn!("mDNS: Failed to create advertiser: {}", e),
        }
    }

    // Spawn capture task
    let frame_duration = Duration::from_micros(1_000_000 / config.fps as u64);
    let capture_config = core_config.clone();

    let capture_handle = if use_evdi {
        let frame_tx = frame_tx.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create capture runtime");

            rt.block_on(async move {
                info!("Creating EVDI virtual display...");
                let mut vd = match VirtualDisplay::new(capture_config) {
                    Ok(vd) => vd,
                    Err(e) => {
                        warn!("Failed to create virtual display: {}", e);
                        return;
                    }
                };

                if let Err(e) = vd.enable() {
                    warn!("Failed to enable virtual display: {}", e);
                    return;
                }

                if let Err(e) = vd.init_buffer().await {
                    warn!("Failed to initialize buffer: {}", e);
                    return;
                }

                info!("EVDI virtual display ready");

                loop {
                    let start = std::time::Instant::now();
                    match vd.capture_async().await {
                        Ok(frame) => {
                            if frame_tx.send(frame).await.is_err() {
                                break;
                            }
                        }
                        Err(e) => warn!("EVDI capture error: {}", e),
                    }
                    let elapsed = start.elapsed();
                    if elapsed < frame_duration {
                        tokio::time::sleep(frame_duration - elapsed).await;
                    }
                }

                let _ = vd.disable();
            });
        });

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(3600)).await;
            }
        })
    } else {
        let frame_tx = frame_tx.clone();
        let mut capture = ScreenCapture::new(capture_config.width, capture_config.height, 0, 0)?;

        tokio::spawn(async move {
            loop {
                let start = std::time::Instant::now();
                match capture.capture() {
                    Ok(frame) => {
                        if frame_tx.send(frame).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => warn!("Capture error: {}", e),
                }
                let elapsed = start.elapsed();
                if elapsed < frame_duration {
                    tokio::time::sleep(frame_duration - elapsed).await;
                }
            }
        })
    };

    // Spawn encoding task
    let segment_tx_clone = segment_tx.clone();
    let enc_width = config.width;
    let enc_height = config.height;
    let enc_fps = config.fps;
    let enc_bitrate = config.bitrate;

    let (init_tx, init_rx) = std::sync::mpsc::channel::<(Vec<u8>, String, Vec<u8>)>();
    let state_clone = state.clone();

    std::thread::spawn(move || {
        let pipeline = match EncodingPipeline::new(enc_width, enc_height, enc_fps, enc_bitrate) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Failed to create encoder: {}", e);
                return;
            }
        };

        if let Some(init_segment) = pipeline.get_init_segment() {
            let codec_string = pipeline.get_codec_string();
            let avcc_data = pipeline.get_avcc_data();
            let _ = init_tx.send((init_segment, codec_string, avcc_data));
        }

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(pipeline.run(frame_rx, segment_tx_clone));
    });

    // Receive init segment
    if let Ok((init_segment, codec_string, avcc_data)) =
        init_rx.recv_timeout(std::time::Duration::from_secs(5))
    {
        info!(
            "Received init segment: {} bytes, codec: {}",
            init_segment.len(),
            codec_string
        );
        state_clone.set_init_segment(init_segment);
        state_clone.set_codec_config(codec_string, avcc_data);
    }

    // Keyframe capture task
    let keyframe_state = state.clone();
    let mut keyframe_rx = segment_tx.subscribe();
    tokio::spawn(async move {
        while let Ok(segment) = keyframe_rx.recv().await {
            if segment.is_keyframe {
                keyframe_state.set_keyframe_segment(segment.data);
            }
        }
    });

    // Input handling task
    let input_handle = tokio::spawn(async move {
        use linglide_core::protocol::InputEvent;

        while let Some(event) = input_rx.recv().await {
            let result = match event {
                InputEvent::TouchStart { id, x, y } => touchscreen.touch_start(id, x, y),
                InputEvent::TouchMove { id, x, y } => touchscreen.touch_move(id, x, y),
                InputEvent::TouchEnd { id } => touchscreen.touch_end(id),
                InputEvent::TouchCancel { id } => touchscreen.touch_cancel(id),
                InputEvent::MouseDown { button, x, y } => mouse.mouse_down(button, x, y),
                InputEvent::MouseUp { button, x, y } => mouse.mouse_up(button, x, y),
                InputEvent::MouseMove { x, y } => mouse.mouse_move(x, y),
                InputEvent::Scroll { dx, dy } => scroll_mouse.scroll(dx, dy),
                InputEvent::KeyDown { .. } | InputEvent::KeyUp { .. } => Ok(()),
                InputEvent::PenHover {
                    x,
                    y,
                    pressure,
                    tilt_x,
                    tilt_y,
                } => stylus.pen_hover(x, y, pressure, tilt_x, tilt_y),
                InputEvent::PenDown {
                    x,
                    y,
                    pressure,
                    tilt_x,
                    tilt_y,
                    button,
                } => stylus.pen_down(x, y, pressure, tilt_x, tilt_y, button),
                InputEvent::PenMove {
                    x,
                    y,
                    pressure,
                    tilt_x,
                    tilt_y,
                } => stylus.pen_move(x, y, pressure, tilt_x, tilt_y),
                InputEvent::PenUp { x, y } => stylus.pen_up(x, y),
                InputEvent::PenButtonEvent { button, pressed } => {
                    stylus.pen_button(button, pressed)
                }
            };

            if let Err(e) = result {
                warn!("Input error: {}", e);
            }
        }
    });

    // Start HTTPS server
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], config.port));
    let handle = axum_server::Handle::new();
    let shutdown_handle = handle.clone();

    // Spawn server
    let server_future = axum_server::bind_rustls(addr, tls_config)
        .handle(handle)
        .serve(router.into_make_service());

    info!("Server listening on https://{}:{}", local_ip, config.port);

    // Wait for shutdown or server completion
    tokio::select! {
        result = server_future => {
            if let Err(e) = result {
                warn!("Server error: {}", e);
            }
        }
        _ = &mut shutdown_rx => {
            info!("Shutdown signal received");
            shutdown_handle.graceful_shutdown(Some(Duration::from_secs(2)));
        }
    }

    // Cleanup
    capture_handle.abort();
    input_handle.abort();

    if let Some(mut advertiser) = mdns_advertiser {
        let _ = advertiser.stop();
        let _ = event_tx.send(UiEvent::MdnsStatus { active: false });
    }

    info!("Server stopped");
    Ok(())
}
