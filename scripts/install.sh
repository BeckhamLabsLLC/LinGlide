#!/bin/bash
# LinGlide Installation Script
# One-command setup for LinGlide on Linux
#
# Usage: sudo ./scripts/install.sh

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_header() {
    echo -e "${BLUE}"
    echo "╔═══════════════════════════════════════════════════════════╗"
    echo "║                LinGlide Installer                         ║"
    echo "║         Extended Display for Linux                        ║"
    echo "╚═══════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

print_step() {
    echo -e "${GREEN}[✓]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[!]${NC} $1"
}

print_error() {
    echo -e "${RED}[✗]${NC} $1"
}

check_root() {
    if [ "$EUID" -ne 0 ]; then
        print_error "This script must be run with sudo"
        echo "Usage: sudo ./scripts/install.sh"
        exit 1
    fi
}

check_dependencies() {
    echo ""
    echo "Checking dependencies..."

    ACTUAL_USER="${SUDO_USER:-$USER}"
    ACTUAL_HOME=$(getent passwd "$ACTUAL_USER" | cut -d: -f6)

    local missing=()

    # Check for cargo (might be in user's home directory)
    CARGO_BIN="$ACTUAL_HOME/.cargo/bin/cargo"
    if [ -x "$CARGO_BIN" ]; then
        CARGO_CMD="$CARGO_BIN"
    elif command -v cargo &> /dev/null; then
        CARGO_CMD="cargo"
    else
        missing+=("rust/cargo")
    fi

    if ! pkg-config --exists openssl 2>/dev/null; then
        missing+=("libssl-dev")
    fi

    if ! pkg-config --exists x11 2>/dev/null; then
        missing+=("libx11-dev")
    fi

    if [ ${#missing[@]} -ne 0 ]; then
        print_error "Missing dependencies: ${missing[*]}"
        echo ""
        echo "Install them with:"
        echo "  sudo apt install build-essential pkg-config libssl-dev libx11-dev"
        echo "  # For Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        exit 1
    fi

    print_step "All dependencies found"
}

setup_evdi() {
    echo ""
    echo "Setting up EVDI (virtual display driver)..."

    # Check if EVDI module exists
    if ! modinfo evdi &> /dev/null; then
        print_warning "EVDI kernel module not found"
        echo "  Installing EVDI from package manager..."

        # Try to install evdi-dkms
        if command -v apt &> /dev/null; then
            apt install -y evdi-dkms || {
                print_error "Failed to install evdi-dkms"
                echo "  You may need to install it manually from: https://github.com/DisplayLink/evdi"
                exit 1
            }
        elif command -v dnf &> /dev/null; then
            dnf install -y evdi-dkms || {
                print_error "Failed to install evdi"
                exit 1
            }
        elif command -v pacman &> /dev/null; then
            pacman -S --noconfirm evdi-dkms || {
                print_error "Failed to install evdi"
                exit 1
            }
        else
            print_error "Could not detect package manager. Please install evdi-dkms manually."
            exit 1
        fi
    fi

    print_step "EVDI module available"

    # Create modprobe config for EVDI
    echo "  Configuring EVDI to load with virtual device..."
    cat > /etc/modprobe.d/linglide-evdi.conf << 'EOF'
# LinGlide EVDI configuration
# Pre-allocate one virtual display device for LinGlide
options evdi initial_device_count=1
EOF
    print_step "Created /etc/modprobe.d/linglide-evdi.conf"

    # Load the module now
    echo "  Loading EVDI module..."
    modprobe -r evdi 2>/dev/null || true
    modprobe evdi initial_device_count=1
    print_step "EVDI module loaded"
}

setup_uinput() {
    echo ""
    echo "Setting up uinput permissions..."

    ACTUAL_USER="${SUDO_USER:-$USER}"

    # Create udev rule for uinput
    cat > /etc/udev/rules.d/99-linglide-uinput.rules << 'EOF'
# LinGlide uinput access rule
# Allows members of the input group to create virtual input devices
KERNEL=="uinput", GROUP="input", MODE="0660"
EOF
    print_step "Created uinput udev rule"

    # Add user to input group
    usermod -a -G input "$ACTUAL_USER"
    print_step "Added $ACTUAL_USER to input group"

    # Reload udev rules
    udevadm control --reload-rules
    udevadm trigger
    print_step "Reloaded udev rules"
}

build_linglide() {
    echo ""
    echo "Building LinGlide..."

    ACTUAL_USER="${SUDO_USER:-$USER}"
    ACTUAL_HOME=$(getent passwd "$ACTUAL_USER" | cut -d: -f6)
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

    cd "$PROJECT_DIR"

    # Build as the actual user (not root) to avoid permission issues
    # Use the user's cargo installation
    sudo -u "$ACTUAL_USER" env "PATH=$ACTUAL_HOME/.cargo/bin:$PATH" \
        cargo build --release -p linglide-desktop
    print_step "Built LinGlide desktop application"
}

install_desktop_app() {
    echo ""
    echo "Installing LinGlide..."

    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

    # Install binary
    install -Dm755 "$PROJECT_DIR/target/release/linglide-gui" /usr/local/bin/linglide
    print_step "Installed binary to /usr/local/bin/linglide"

    # Install icon
    if [ -f "$PROJECT_DIR/crates/linglide-desktop/assets/icons/linglide-icon.png" ]; then
        install -Dm644 "$PROJECT_DIR/crates/linglide-desktop/assets/icons/linglide-icon.png" \
            /usr/share/icons/hicolor/256x256/apps/linglide.png
        print_step "Installed application icon"
    fi

    # Create .desktop file
    cat > /usr/share/applications/linglide.desktop << 'EOF'
[Desktop Entry]
Name=LinGlide
Comment=Extended display for Linux - use your mobile device as a second monitor
Exec=linglide
Icon=linglide
Terminal=false
Type=Application
Categories=Utility;System;
Keywords=display;monitor;screen;extend;virtual;
StartupNotify=true
EOF
    print_step "Created desktop entry"

    # Update icon cache
    if command -v gtk-update-icon-cache &> /dev/null; then
        gtk-update-icon-cache -f /usr/share/icons/hicolor/ 2>/dev/null || true
    fi

    # Update desktop database
    if command -v update-desktop-database &> /dev/null; then
        update-desktop-database /usr/share/applications/ 2>/dev/null || true
    fi
}

print_success() {
    echo ""
    echo -e "${GREEN}╔═══════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║              Installation Complete!                       ║${NC}"
    echo -e "${GREEN}╚═══════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo "LinGlide has been installed successfully."
    echo ""
    echo -e "${YELLOW}IMPORTANT:${NC} You need to log out and log back in for"
    echo "group permissions to take effect."
    echo ""
    echo "After logging back in, you can:"
    echo "  1. Launch from your application menu (search 'LinGlide')"
    echo "  2. Run from terminal: linglide"
    echo ""
    echo "To connect your mobile device:"
    echo "  1. Open https://<your-ip>:8443 in your mobile browser"
    echo "  2. Scan the QR code shown in LinGlide"
    echo ""
    echo "For more info: https://github.com/BeckhamLabs/linglide"
    echo ""
}

# Main installation flow
main() {
    print_header
    check_root
    check_dependencies
    setup_evdi
    setup_uinput
    build_linglide
    install_desktop_app
    print_success
}

main "$@"
