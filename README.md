# LinGlide

<p align="center">
  <img src="LinGlideFlow.png" alt="LinGlide Logo" width="128">
</p>

A high-performance Linux application that turns your mobile device into an extended display with touch control. Built with Rust for optimal performance.

**Developed by [BeckhamLabs](https://beckhamlabs.com)**

## Features

- **Extended Display**: Use your phone/tablet as a second monitor
- **Touch Control**: Full touch input support - tap, scroll, drag
- **Zero App Install**: Works in any modern browser (PWA supported)
- **Low Latency**: Hardware-accelerated H.264 streaming
- **Secure**: Device pairing with PIN verification
- **mDNS Discovery**: Devices auto-discover the server on your network

## Quick Install

One command to install everything:

```bash
git clone https://github.com/BeckhamLabsLLC/LinGlide.git
cd linglide
sudo ./scripts/install.sh
```

Then **log out and log back in** for permissions to take effect.

## Usage

1. Launch **LinGlide** from your application menu (or run `linglide` in terminal)
2. Click **Start Server**
3. On your mobile device, open the URL shown (or scan the QR code)
4. Enter the PIN to pair your device

That's it! Your mobile device is now an extended display.

## System Requirements

- Linux with X11 (Wayland support planned)
- EVDI kernel module (installed automatically)
- Modern browser with WebCodecs (Chrome 94+, Edge 94+, Safari 16.4+)

## Build from Source

If you prefer to build manually:

```bash
# Install dependencies (Ubuntu/Debian)
sudo apt install build-essential pkg-config libssl-dev libx11-dev \
    libxcb1-dev libxcb-shm0-dev libxcb-randr0-dev libx264-dev \
    libudev-dev evdi-dkms

# Build
cargo build --release -p linglide-desktop

# Run
./target/release/linglide-gui
```

## Uninstall

```bash
sudo ./scripts/uninstall.sh
```

## How It Works

```
┌─────────────────────────────────────────────────────────────┐
│                    LINUX HOST (Rust)                        │
├─────────────────────────────────────────────────────────────┤
│  EVDI         →   Frame      →   H.264 Encoder              │
│  (Virtual         Capture        (x264)                     │
│   Display)                                                  │
│                                                             │
│  uinput       ←   Event      ←   WebSocket Server           │
│  (Touch)          Handler        (Axum + TLS)               │
└─────────────────────────────────────────────────────────────┘
                              │
                    WiFi (WebSocket + fMP4)
                              │
┌─────────────────────────────────────────────────────────────┐
│                  MOBILE DEVICE (Browser/PWA)                │
├─────────────────────────────────────────────────────────────┤
│  WebCodecs      Touch Event      Fullscreen                 │
│  H.264 Decoder  Handler          Manager                    │
└─────────────────────────────────────────────────────────────┘
```

## Troubleshooting

### EVDI module not loading

If you see "Failed to add EVDI device", run:
```bash
sudo modprobe -r evdi && sudo modprobe evdi initial_device_count=1
```

The install script configures this automatically for future boots.

### Permission denied for uinput

Log out and log back in after installation for group permissions to take effect.

### Browser not supported

WebCodecs is required. Supported browsers:
- Chrome/Chromium 94+
- Edge 94+
- Safari 16.4+

Firefox does not support WebCodecs.

## License

MIT License - see [LICENSE](LICENSE) for details.

---

<p align="center">
  Made with care for the Linux community by <a href="https://beckhamlabs.com">BeckhamLabs</a>
</p>
