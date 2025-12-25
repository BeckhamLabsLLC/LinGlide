#!/bin/bash
# LinGlide Uninstall Script
#
# Usage: sudo ./scripts/uninstall.sh

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

print_step() {
    echo -e "${GREEN}[✓]${NC} $1"
}

if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}[✗]${NC} This script must be run with sudo"
    exit 1
fi

echo -e "${BLUE}Uninstalling LinGlide...${NC}"
echo ""

# Remove binary
if [ -f /usr/local/bin/linglide ]; then
    rm /usr/local/bin/linglide
    print_step "Removed /usr/local/bin/linglide"
fi

# Remove desktop entry
if [ -f /usr/share/applications/linglide.desktop ]; then
    rm /usr/share/applications/linglide.desktop
    print_step "Removed desktop entry"
fi

# Remove icon
if [ -f /usr/share/icons/hicolor/256x256/apps/linglide.png ]; then
    rm /usr/share/icons/hicolor/256x256/apps/linglide.png
    print_step "Removed application icon"
fi

# Remove EVDI config (optional - ask user)
if [ -f /etc/modprobe.d/linglide-evdi.conf ]; then
    rm /etc/modprobe.d/linglide-evdi.conf
    print_step "Removed EVDI configuration"
fi

# Remove uinput rule (optional)
if [ -f /etc/udev/rules.d/99-linglide-uinput.rules ]; then
    rm /etc/udev/rules.d/99-linglide-uinput.rules
    udevadm control --reload-rules
    print_step "Removed uinput udev rule"
fi

# Update caches
if command -v gtk-update-icon-cache &> /dev/null; then
    gtk-update-icon-cache -f /usr/share/icons/hicolor/ 2>/dev/null || true
fi
if command -v update-desktop-database &> /dev/null; then
    update-desktop-database /usr/share/applications/ 2>/dev/null || true
fi

echo ""
echo -e "${GREEN}LinGlide has been uninstalled.${NC}"
echo ""
echo "Note: User config remains in ~/.config/linglide/"
echo "      Remove manually if desired: rm -rf ~/.config/linglide"
echo ""
