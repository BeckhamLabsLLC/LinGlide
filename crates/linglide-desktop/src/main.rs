//! LinGlide Desktop Application
//!
//! A standalone Linux desktop application with system tray for controlling
//! the LinGlide screen sharing server.

mod app;
mod bridge;
mod components;
mod controller;
mod theme;
mod tray;
mod windows;

use app::LinGlideApp;
use bridge::create_bridge;
use controller::ServerController;
use tracing::info;
use tracing_subscriber::EnvFilter;

fn main() -> anyhow::Result<()> {
    // Initialize logging with filter to suppress noisy EVDI buffer timeout warnings
    // EVDI logs warnings every 50ms when screen content hasn't changed (expected behavior)
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,evdi=error"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();

    info!("LinGlide Desktop v{}", env!("CARGO_PKG_VERSION"));

    // Create communication bridge
    let (ui_bridge, async_bridge) = create_bridge();

    // Spawn async runtime with server controller
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create async runtime");

        rt.block_on(async move {
            let controller = ServerController::new(async_bridge);
            if let Err(e) = controller.run().await {
                tracing::error!("Server controller error: {}", e);
            }
        });
    });

    // Run the GUI
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([450.0, 500.0])
            .with_min_inner_size([350.0, 400.0])
            .with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "LinGlide",
        native_options,
        Box::new(|cc| Ok(Box::new(LinGlideApp::new(cc, ui_bridge)))),
    )
    .map_err(|e| anyhow::anyhow!("Failed to run application: {}", e))?;

    Ok(())
}

/// Load application icon from PNG file
fn load_icon() -> egui::IconData {
    // Try to load from PNG file
    let icon_paths = [
        // Development path (relative to crate)
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/icons/linglide-icon.png"
        ),
        // Installed paths (Linux standard locations)
        "/usr/share/icons/hicolor/256x256/apps/linglide.png",
        "/usr/share/pixmaps/linglide.png",
    ];

    for path in icon_paths {
        if let Ok(icon_data) = load_icon_from_file(path) {
            tracing::debug!("Loaded icon from: {}", path);
            return icon_data;
        }
    }

    tracing::warn!("Could not load icon from file, using fallback");
    generate_fallback_icon()
}

/// Try to load icon from a PNG file
fn load_icon_from_file(path: &str) -> Result<egui::IconData, Box<dyn std::error::Error>> {
    let image_data = std::fs::read(path)?;
    let img = image::load_from_memory(&image_data)?;
    let rgba = img.to_rgba8();

    Ok(egui::IconData {
        rgba: rgba.as_raw().clone(),
        width: rgba.width(),
        height: rgba.height(),
    })
}

/// Generate a simple fallback icon if PNG loading fails
fn generate_fallback_icon() -> egui::IconData {
    let size = 64;
    let mut rgba = Vec::with_capacity(size * size * 4);

    for y in 0..size {
        for x in 0..size {
            let cx = size as f32 / 2.0;
            let cy = size as f32 / 2.0;
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist < (size as f32 / 2.0 - 4.0) {
                // Blue color matching theme
                rgba.push(59); // R
                rgba.push(130); // G
                rgba.push(246); // B
                rgba.push(255); // A
            } else {
                rgba.push(0);
                rgba.push(0);
                rgba.push(0);
                rgba.push(0);
            }
        }
    }

    egui::IconData {
        rgba,
        width: size as u32,
        height: size as u32,
    }
}
