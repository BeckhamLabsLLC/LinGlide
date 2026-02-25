<p align="center">
  <img src="LinGlideFlow.png" alt="LinGlide" width="128">
  <br>
  <strong>LinGlide</strong>
</p>

<h3 align="center">Use your phone as a second monitor on Linux</h3>

<p align="center">
  <a href="https://github.com/BeckhamLabsLLC/LinGlide/actions/workflows/ci.yml"><img src="https://github.com/BeckhamLabsLLC/LinGlide/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License: MIT"></a>
  <a href="https://github.com/BeckhamLabsLLC/LinGlide/releases"><img src="https://img.shields.io/github/v/release/BeckhamLabsLLC/LinGlide" alt="Release"></a>
  <a href="https://github.com/BeckhamLabsLLC/LinGlide/stargazers"><img src="https://img.shields.io/github/stars/BeckhamLabsLLC/LinGlide?style=social" alt="GitHub stars"></a>
</p>

<p align="center">
  <em>Turn any phone or tablet into a wireless extended display with touch control.<br>No app install needed — runs entirely in the browser. Built with Rust.</em>
</p>

---

Windows and macOS have tools like Duet Display, Sidecar, and GlideX. Linux has had nothing — until now.

**LinGlide** is a native Linux application that creates a real virtual display on your system and streams it to any mobile device over WiFi. Your phone becomes a true second monitor: drag windows to it, use touch to interact, and it shows up in your display settings like any other screen.

<!-- TODO: Replace with an actual demo GIF or screenshot of LinGlide in action -->
<!-- <p align="center">
  <img src="demo.gif" alt="LinGlide demo" width="600">
</p> -->

## Why LinGlide?

| | LinGlide | VNC-based scripts | Deskreen |
|---|---|---|---|
| **True extended display** | Virtual monitor via EVDI | xrandr hacks | Mirror only |
| **Touch input** | Native tap, scroll, drag, stylus | No | No |
| **App install required** | No (browser PWA) | VNC viewer app | No |
| **Latency** | Low (H.264 + WebCodecs) | High (VNC) | Medium (WebRTC) |
| **Written in** | Rust | Bash/Python | Electron |
| **Linux native** | Yes | Yes | Cross-platform |
| **Secure pairing** | PIN + token auth + TLS | None | None |

## Features

- **Extended display** — real virtual monitor via EVDI kernel module, visible in display settings
- **Mirror mode** — share your existing screen without a virtual display
- **Full touch control** — tap, scroll, drag, and stylus input via uinput
- **Zero install on mobile** — works in Chrome, Edge, Safari (PWA for app-like experience)
- **Low latency** — hardware-accelerated H.264 encoding with OpenH264
- **Secure** — 6-digit PIN pairing, token authentication, TLS encryption
- **Auto-discovery** — devices find LinGlide automatically via mDNS
- **USB support** — ADB port forwarding for wired Android connections
- **Desktop GUI** — system tray app with QR code for easy pairing
- **CLI** — fully scriptable for power users

## Quick Start

```bash
git clone https://github.com/BeckhamLabsLLC/LinGlide.git
cd LinGlide
sudo ./scripts/install.sh
```

Log out and back in, then:

1. Launch **LinGlide** from your application menu (or run `linglide`)
2. Click **Start Server**
3. On your phone, open the URL shown or scan the QR code
4. Enter the PIN — done

## System Requirements

- Linux with X11 (Wayland support planned)
- EVDI kernel module (installed automatically by the install script)
- Mobile browser with WebCodecs support:
  - Chrome/Chromium 94+
  - Edge 94+
  - Safari 16.4+
  - Firefox is **not** supported (no WebCodecs)

## Build from Source

```bash
# Install dependencies (Ubuntu/Debian)
sudo apt install build-essential pkg-config libssl-dev libx11-dev \
    libxcb1-dev libxcb-shm0-dev libxcb-randr0-dev \
    libudev-dev evdi-dkms

# Build
cargo build --release -p linglide-desktop

# Run
./target/release/linglide-gui
```

## How It Works

```
┌─────────────────────────────────────────────────────────────┐
│                    LINUX HOST (Rust)                        │
├─────────────────────────────────────────────────────────────┤
│  EVDI         →   Frame      →   H.264 Encoder             │
│  (Virtual         Capture        (OpenH264)                 │
│   Display)                                                  │
│                                                             │
│  uinput       ←   Event      ←   WebSocket Server          │
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

<details>
<summary><strong>EVDI module not loading</strong></summary>

If you see "Failed to add EVDI device", run:
```bash
sudo modprobe -r evdi && sudo modprobe evdi initial_device_count=1
```
The install script configures this automatically for future boots.
</details>

<details>
<summary><strong>Permission denied for uinput</strong></summary>

Log out and log back in after installation for group permissions to take effect.
</details>

<details>
<summary><strong>Browser not supported</strong></summary>

WebCodecs is required. Use Chrome/Chromium 94+, Edge 94+, or Safari 16.4+. Firefox does not support WebCodecs.
</details>

## Contributing

Contributions are welcome! Feel free to open issues for bugs or feature requests, or submit pull requests.

## Uninstall

```bash
sudo ./scripts/uninstall.sh
```

## License

MIT License — see [LICENSE](LICENSE) for details.

---

<p align="center">
  <a href="https://beckhamlabs.com">BeckhamLabs</a>
</p>
