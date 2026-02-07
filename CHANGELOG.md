# Changelog

All notable changes to LinGlide will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2025-02-06

### Fixed
- **Touch input coordinate offset**: Touch coordinates now correctly account for display position. Primary display dimensions are auto-detected via XCB RandR, and the offset is computed from the configured display position (right-of, left-of, above, below).
- **Encoding thread silent crash**: The encoding thread now catches panics and logs errors instead of crashing silently. The tokio runtime creation uses proper error handling instead of `.unwrap()`.
- **PipeWire capture thread leak**: The PipeWire capture thread is now properly joined on drop, preventing resource leaks.
- **Nightly-only API usage**: Replaced `u64::is_multiple_of()` (nightly-only) with stable `%` operator.
- **Stale x264 references**: Updated all documentation and comments to reference OpenH264 instead of x264.

### Security
- **PIN brute-force protection**: Added rate limiting to PIN verification (max 5 attempts, 5-minute lockout per key).
- **Removed unauthenticated PIN endpoint**: `GET /api/pair/pin` no longer exposes the PIN. The `POST /api/pair/pin/refresh` endpoint now requires a valid auth token.
- **Reduced input event logging**: Touch and input events downgraded from INFO to DEBUG/TRACE to prevent log spam and avoid leaking user interaction data.

### Changed
- Deduplicated URL encoding: replaced custom implementations in `main.rs` and `http.rs` with the `urlencoding` crate (already used by `linglide-desktop`).
- Keyboard input stub now logs at DEBUG level instead of silently succeeding.
- CI system dependencies extracted to `scripts/install-ci-deps.sh`, removing ~60 lines of duplication.
- Removed `libx264-dev` from build dependencies (not needed with OpenH264).

### Added
- `detect_primary_display()` function in `linglide-capture` for auto-detecting primary display resolution via XCB RandR.
- `Config::display_offset()` method for computing display offset from position and primary display dimensions.
- `Config::with_primary_display()` builder method.
- GitHub Release workflow (`.github/workflows/release.yml`) triggered on `v*` tag push, builds release tarball and publishes to GitHub Releases.
- This CHANGELOG.

## [0.1.0] - 2025-01-15

### Added
- Initial release of LinGlide.
- Extended display support using EVDI virtual display.
- Mirror mode for screen capture without virtual display.
- H.264 video encoding with OpenH264 and fMP4 muxing.
- WebSocket-based video streaming to browser clients.
- Touch, mouse, scroll, and stylus input via uinput.
- Secure device pairing with 6-digit PIN and QR code.
- Token-based authentication for WebSocket connections.
- TLS support with self-signed certificate management.
- mDNS service discovery for automatic device detection.
- USB/ADB port forwarding for Android devices.
- Progressive Web App (PWA) client with WebCodecs decoding.
- Desktop GUI application with system tray (linglide-gui).
- CLI application (linglide).

[0.2.0]: https://github.com/BeckhamLabsLLC/LinGlide/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/BeckhamLabsLLC/LinGlide/releases/tag/v0.1.0
