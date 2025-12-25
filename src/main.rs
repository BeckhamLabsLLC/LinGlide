//! LinGlide - Linux native screen sharing for mobile devices
//!
//! A high-performance alternative to ASUS GlideX that enables using iOS/Android
//! devices as secondary extended screens with touch control.

use anyhow::Result;
use clap::Parser;
use linglide_auth::{DeviceStorage, PairingManager};
use linglide_capture::{Frame, VirtualDisplay, ScreenCapture};
use linglide_core::{Config, DisplayPosition};
use linglide_discovery::{ServiceAdvertiser, UsbConnectionManager};
use linglide_encoder::EncodingPipeline;
use linglide_encoder::pipeline::StreamSegment;
use linglide_input::{VirtualMouse, VirtualStylus, VirtualTouchscreen, mouse::RelativeMouse};
use linglide_server::{broadcast::AppState, create_router, CertificateManager, create_rustls_config};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use tracing::{info, warn, debug, Level};
use tracing_subscriber::EnvFilter;

/// LinGlide - Use your mobile device as an extended display
#[derive(Parser, Debug)]
#[command(name = "linglide")]
#[command(version, about, long_about = None)]
struct Args {
    /// Display width in pixels
    #[arg(short = 'W', long, default_value = "1920")]
    width: u32,

    /// Display height in pixels
    #[arg(short = 'H', long, default_value = "1080")]
    height: u32,

    /// Target frame rate
    #[arg(short, long, default_value = "60")]
    fps: u32,

    /// Server port
    #[arg(short, long, default_value = "8443")]
    port: u16,

    /// Position relative to primary display
    #[arg(short = 'P', long, default_value = "right-of")]
    position: String,

    /// Video bitrate in kbps
    #[arg(short, long, default_value = "8000")]
    bitrate: u32,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Mirror mode: capture primary display instead of creating virtual display
    /// Useful for testing or when no disconnected output is available (e.g., Wayland)
    #[arg(short, long)]
    mirror: bool,

    /// Disable HTTPS (not recommended - WebCodecs requires secure context)
    #[arg(long)]
    no_tls: bool,

    /// Path to TLS certificate file (PEM format)
    #[arg(long)]
    cert: Option<String>,

    /// Path to TLS private key file (PEM format)
    #[arg(long)]
    key: Option<String>,

    /// Disable authentication (not recommended for production)
    /// When disabled, any device can connect without pairing
    #[arg(long)]
    no_auth: bool,

    /// Disable mDNS service advertisement
    /// When disabled, mobile devices cannot auto-discover this server
    #[arg(long)]
    no_mdns: bool,

    /// Custom mDNS service name (default: LinGlide-<hostname>)
    #[arg(long)]
    service_name: Option<String>,

    /// Enable USB/ADB port forwarding for Android devices
    /// Allows Android devices to connect via USB without network
    #[arg(long)]
    enable_usb: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging with filter to reduce evdi spam
    let log_level = if args.verbose { Level::DEBUG } else { Level::INFO };
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .compact()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(log_level.into())
                .add_directive("evdi=error".parse().unwrap())  // Suppress evdi warnings
        )
        .finish();
    tracing::subscriber::set_global_default(subscriber).ok();

    info!("LinGlide v{}", env!("CARGO_PKG_VERSION"));

    // Parse position
    let position: DisplayPosition = args.position.parse()
        .map_err(|e: String| anyhow::anyhow!(e))?;

    // Create configuration
    let config = Config::new()
        .with_width(args.width)
        .with_height(args.height)
        .with_fps(args.fps)
        .with_port(args.port)
        .with_position(position)
        .with_bitrate(args.bitrate)
        .with_mirror_mode(args.mirror);

    // Capture setup: EVDI for virtual display, ScreenCapture for mirror mode
    let use_evdi = !config.mirror_mode;
    // TODO: For now, use offset 0 to test if touch works at all
    // On Wayland, input devices may need special handling for virtual displays
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

    // Get local IP address for display
    let local_ip = get_local_ip().unwrap_or_else(|| "localhost".to_string());

    // Setup TLS with persistent certificates
    let use_tls = !args.no_tls;
    let (tls_config, cert_fingerprint) = if use_tls {
        let (cert_pem, key_pem, fingerprint) = match (&args.cert, &args.key) {
            (Some(cert_path), Some(key_path)) => {
                info!("Loading TLS certificate from files...");
                let cert = std::fs::read_to_string(cert_path)?;
                let key = std::fs::read_to_string(key_path)?;
                let fp = linglide_server::calculate_cert_fingerprint(&cert);
                (cert, key, fp)
            }
            _ => {
                info!("Using persistent certificate storage...");
                let cert_manager = CertificateManager::new()
                    .map_err(|e| anyhow::anyhow!("Failed to create certificate manager: {}", e))?;

                let hostnames = vec![local_ip.clone(), "localhost".to_string()];
                cert_manager.load_or_generate(&hostnames)
                    .map_err(|e| anyhow::anyhow!("Failed to load/generate certificate: {}", e))?
            }
        };

        info!("Certificate fingerprint: {}", fingerprint);

        let config = create_rustls_config(&cert_pem, &key_pem)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create TLS config: {}", e))?;

        (Some(config), Some(fingerprint))
    } else {
        (None, None)
    };

    // Initialize device storage and pairing manager
    info!("Initializing device storage...");
    let device_storage = Arc::new(
        DeviceStorage::new()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to initialize device storage: {}", e))?
    );

    let protocol = if use_tls { "https" } else { "http" };
    let server_url = format!("{}://{}:{}", protocol, local_ip, config.port);
    let pairing_manager = Arc::new(PairingManager::new(device_storage.clone(), server_url.clone()));

    // Check authentication status
    let auth_required = !args.no_auth;
    let paired_count = pairing_manager.list_devices().await.len();

    if auth_required {
        info!("Authentication: ENABLED ({} paired devices)", paired_count);
    } else {
        warn!("Authentication: DISABLED (--no-auth flag set)");
    }

    // Create app state
    let state = Arc::new(AppState::new(
        config.clone(),
        segment_tx.clone(),
        input_tx,
        pairing_manager.clone(),
        auth_required,
        cert_fingerprint.clone(),
    ));

    // Create router
    let router = create_router(state.clone());

    info!("Starting server on port {}...", config.port);
    info!("");
    info!("  Access URL: {}", server_url);
    if use_tls {
        if let Some(ref fp) = cert_fingerprint {
            info!("  Cert fingerprint: {}...", &fp[..23]);
        }
        info!("");
        info!("  NOTE: You may need to accept the self-signed certificate in your browser.");
    }
    info!("");

    // Auto-start pairing session if no devices are paired
    if auth_required && paired_count == 0 {
        info!("No paired devices. Starting pairing session...");
        info!("");

        let pairing_response = pairing_manager.start_pairing().await;
        let pin = &pairing_response.pin;
        let session_id = &pairing_response.session_id;

        // Build pairing URL for QR code
        let pairing_url = format!(
            "linglide://pair?url={}&pin={}&session={}{}",
            urlencoding::encode(&server_url),
            pin,
            session_id,
            cert_fingerprint.as_ref().map(|fp| format!("&fp={}", &fp[..fp.len().min(20)])).unwrap_or_default()
        );

        // Display QR code in terminal
        display_qr_code(&pairing_url);

        info!("");
        info!("  ╔══════════════════════════════════════╗");
        info!("  ║         PAIRING PIN: {}         ║", pin);
        info!("  ╚══════════════════════════════════════╝");
        info!("");
        info!("  Scan the QR code above, or:");
        info!("    1. Open {} on your device", server_url);
        info!("    2. Enter PIN: {}", pin);
        info!("");
        info!("  PIN expires in {} seconds", pairing_response.expires_in);
        info!("");

        // Spawn task to refresh pairing session when it expires
        let pm = pairing_manager.clone();
        let url = server_url.clone();
        let fp = cert_fingerprint.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(55)).await;

                // Check if still no paired devices
                if pm.list_devices().await.is_empty() {
                    let response = pm.start_pairing().await;
                    let pairing_url = format!(
                        "linglide://pair?url={}&pin={}&session={}{}",
                        urlencoding::encode(&url),
                        response.pin,
                        response.session_id,
                        fp.as_ref().map(|f| format!("&fp={}", &f[..f.len().min(20)])).unwrap_or_default()
                    );

                    println!();
                    display_qr_code(&pairing_url);
                    println!();
                    println!("  ╔══════════════════════════════════════╗");
                    println!("  ║         PAIRING PIN: {}         ║", response.pin);
                    println!("  ╚══════════════════════════════════════╝");
                    println!();
                    println!("  PIN refreshed. Expires in {} seconds", response.expires_in);
                    println!();
                } else {
                    // Device paired, stop refreshing
                    println!();
                    println!("  ✓ Device paired successfully!");
                    println!();
                    break;
                }
            }
        });
    } else if paired_count > 0 {
        info!("  {} device(s) already paired", paired_count);
        info!("");
    }

    info!("Press Ctrl+C to stop.");
    info!("");

    // Initialize mDNS service advertisement
    let mut mdns_advertiser: Option<ServiceAdvertiser> = None;
    if !args.no_mdns {
        match ServiceAdvertiser::new(config.port, args.service_name.clone()) {
            Ok(mut advertiser) => {
                // Get IP addresses for advertisement
                let addresses: Vec<IpAddr> = get_local_ip()
                    .and_then(|ip| ip.parse().ok())
                    .into_iter()
                    .collect();

                let fp = cert_fingerprint.as_deref();
                match advertiser.start(env!("CARGO_PKG_VERSION"), fp, Some(addresses)) {
                    Ok(()) => {
                        info!("mDNS: Advertising as '{}'", advertiser.instance_name());
                        mdns_advertiser = Some(advertiser);
                    }
                    Err(e) => {
                        warn!("mDNS: Failed to start advertisement: {}", e);
                    }
                }
            }
            Err(e) => {
                warn!("mDNS: Failed to create advertiser: {}", e);
            }
        }
    } else {
        debug!("mDNS: Disabled via --no-mdns flag");
    }

    // Initialize USB/ADB port forwarding
    let mut usb_manager: Option<UsbConnectionManager> = None;
    if args.enable_usb {
        let mut manager = UsbConnectionManager::new(config.port);

        if manager.is_adb_available().await {
            match manager.setup_forwarding().await {
                Ok(()) => {
                    info!("USB: ADB port forwarding enabled");
                    usb_manager = Some(manager);
                }
                Err(e) => {
                    warn!("USB: Failed to setup ADB forwarding: {}", e);
                }
            }
        } else {
            warn!("USB: ADB not found in PATH, USB forwarding disabled");
        }
    }

    // Spawn capture task
    // EVDI uses a dedicated thread (contains raw pointers, not Send)
    // Mirror mode uses async task
    let frame_duration = Duration::from_micros(1_000_000 / config.fps as u64);
    let capture_config = config.clone();

    let capture_handle = if use_evdi {
        // EVDI capture on dedicated thread
        let _capture_thread = std::thread::spawn(move || {
            // Create runtime for this thread
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create capture runtime");

            rt.block_on(async move {
                // Create and enable virtual display
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

                // Initialize buffer (wait for mode from compositor)
                info!("Waiting for display mode from compositor...");
                if let Err(e) = vd.init_buffer().await {
                    warn!("Failed to initialize buffer: {}", e);
                    return;
                }

                info!("EVDI virtual display ready, starting capture...");

                // Capture loop
                loop {
                    let start = std::time::Instant::now();

                    match vd.capture_async().await {
                        Ok(frame) => {
                            if frame_tx.send(frame).await.is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            warn!("EVDI capture error: {}", e);
                        }
                    }

                    // Maintain frame rate
                    let elapsed = start.elapsed();
                    if elapsed < frame_duration {
                        tokio::time::sleep(frame_duration - elapsed).await;
                    }
                }

                // Cleanup
                if let Err(e) = vd.disable() {
                    warn!("Failed to disable virtual display: {}", e);
                }
            });
        });

        // Return a dummy handle that we can abort
        tokio::spawn(async move {
            // Just keep running - actual capture is on the thread
            loop {
                tokio::time::sleep(Duration::from_secs(3600)).await;
            }
        })
    } else {
        // Mirror mode: use async ScreenCapture
        info!("Mirror mode: capturing primary display");
        let mut capture = ScreenCapture::new(capture_config.width, capture_config.height, 0, 0)
            .expect("Failed to create screen capture");

        tokio::spawn(async move {
            loop {
                let start = std::time::Instant::now();

                match capture.capture() {
                    Ok(frame) => {
                        if frame_tx.send(frame).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        warn!("Capture error: {}", e);
                    }
                }

                // Maintain frame rate
                let elapsed = start.elapsed();
                if elapsed < frame_duration {
                    tokio::time::sleep(frame_duration - elapsed).await;
                }
            }
        })
    };

    // Spawn encoding task on a dedicated thread (x264 is not Send)
    // We need to create the encoder inside the thread
    let segment_tx_clone = segment_tx.clone();
    let enc_width = config.width;
    let enc_height = config.height;
    let enc_fps = config.fps;
    let enc_bitrate = config.bitrate;

    // Channel to receive init segment and codec info from encoder thread
    let (init_tx, init_rx) = std::sync::mpsc::channel::<(Vec<u8>, String, Vec<u8>)>();
    let state_clone = state.clone();

    let _encoding_handle = std::thread::spawn(move || {
        // Create encoder inside the thread
        let pipeline = match EncodingPipeline::new(enc_width, enc_height, enc_fps, enc_bitrate) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Failed to create encoder: {}", e);
                return;
            }
        };

        // Send init segment and codec info to main thread
        if let Some(init_segment) = pipeline.get_init_segment() {
            let codec_string = pipeline.get_codec_string();
            let avcc_data = pipeline.get_avcc_data();
            let _ = init_tx.send((init_segment, codec_string, avcc_data));
        }

        // Create a single-threaded runtime for this thread
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(pipeline.run(frame_rx, segment_tx_clone));
    });

    // Receive and store init segment and codec info in app state
    if let Ok((init_segment, codec_string, avcc_data)) = init_rx.recv_timeout(std::time::Duration::from_secs(5)) {
        info!("Received init segment: {} bytes, codec: {}", init_segment.len(), codec_string);
        state_clone.set_init_segment(init_segment);
        state_clone.set_codec_config(codec_string, avcc_data);
    } else {
        warn!("Failed to receive init segment from encoder");
    }

    // Spawn task to capture keyframe segments for new clients
    let keyframe_state = state.clone();
    let mut keyframe_rx = segment_tx.subscribe();
    tokio::spawn(async move {
        while let Ok(segment) = keyframe_rx.recv().await {
            if segment.is_keyframe {
                keyframe_state.set_keyframe_segment(segment.data);
            }
        }
    });

    // Spawn input handling task
    let input_handle = tokio::spawn(async move {
        use linglide_core::protocol::InputEvent;

        while let Some(event) = input_rx.recv().await {
            let result = match event {
                InputEvent::TouchStart { id, x, y } => {
                    touchscreen.touch_start(id, x, y)
                }
                InputEvent::TouchMove { id, x, y } => {
                    touchscreen.touch_move(id, x, y)
                }
                InputEvent::TouchEnd { id } => {
                    touchscreen.touch_end(id)
                }
                InputEvent::TouchCancel { id } => {
                    touchscreen.touch_cancel(id)
                }
                InputEvent::MouseDown { button, x, y } => {
                    mouse.mouse_down(button, x, y)
                }
                InputEvent::MouseUp { button, x, y } => {
                    mouse.mouse_up(button, x, y)
                }
                InputEvent::MouseMove { x, y } => {
                    mouse.mouse_move(x, y)
                }
                InputEvent::Scroll { dx, dy } => {
                    scroll_mouse.scroll(dx, dy)
                }
                InputEvent::KeyDown { .. } | InputEvent::KeyUp { .. } => {
                    // Keyboard input not implemented yet
                    Ok(())
                }
                // Stylus/pen events
                InputEvent::PenHover { x, y, pressure, tilt_x, tilt_y } => {
                    stylus.pen_hover(x, y, pressure, tilt_x, tilt_y)
                }
                InputEvent::PenDown { x, y, pressure, tilt_x, tilt_y, button } => {
                    stylus.pen_down(x, y, pressure, tilt_x, tilt_y, button)
                }
                InputEvent::PenMove { x, y, pressure, tilt_x, tilt_y } => {
                    stylus.pen_move(x, y, pressure, tilt_x, tilt_y)
                }
                InputEvent::PenUp { x, y } => {
                    stylus.pen_up(x, y)
                }
                InputEvent::PenButtonEvent { button, pressed } => {
                    stylus.pen_button(button, pressed)
                }
            };

            if let Err(e) = result {
                warn!("Input error: {}", e);
            }
        }
    });

    // Start HTTP/HTTPS server
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], config.port));

    // Run server with graceful shutdown
    if let Some(tls_config) = tls_config {
        let handle = axum_server::Handle::new();
        let shutdown_handle = handle.clone();

        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.ok();
            info!("Shutting down...");
            shutdown_handle.graceful_shutdown(Some(std::time::Duration::from_secs(5)));
        });

        axum_server::bind_rustls(addr, tls_config)
            .handle(handle)
            .serve(router.into_make_service())
            .await?;
    } else {
        let shutdown = async {
            tokio::signal::ctrl_c().await.ok();
            info!("Shutting down...");
        };

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, router)
            .with_graceful_shutdown(shutdown)
            .await?;
    }

    // Cleanup
    capture_handle.abort();
    input_handle.abort();

    // Stop mDNS advertisement
    if let Some(mut advertiser) = mdns_advertiser {
        if let Err(e) = advertiser.stop() {
            warn!("mDNS: Failed to stop advertisement: {}", e);
        }
    }

    // Remove USB/ADB forwarding
    if let Some(mut manager) = usb_manager {
        if let Err(e) = manager.remove_forwarding().await {
            warn!("USB: Failed to remove ADB forwarding: {}", e);
        }
    }

    // Note: VirtualDisplay cleanup happens via Drop when capture_handle is aborted

    info!("Goodbye!");
    Ok(())
}

/// Get the local IP address
fn get_local_ip() -> Option<String> {
    use std::net::UdpSocket;

    // Create a UDP socket and connect to an external address
    // This doesn't actually send any data but helps determine the local IP
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let addr = socket.local_addr().ok()?;
    Some(addr.ip().to_string())
}

/// Display a QR code in the terminal
fn display_qr_code(data: &str) {
    use qrcode::QrCode;

    let code = match QrCode::new(data.as_bytes()) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to generate QR code: {}", e);
            return;
        }
    };

    // Render as Unicode block characters for terminal display
    let string = code.render::<char>()
        .quiet_zone(true)
        .module_dimensions(2, 1)
        .build();

    for line in string.lines() {
        println!("  {}", line);
    }
}

/// Simple URL encoding for pairing URL
mod urlencoding {
    pub fn encode(s: &str) -> String {
        s.chars()
            .map(|c| match c {
                'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
                _ => format!("%{:02X}", c as u8),
            })
            .collect()
    }
}
