#!/bin/bash
# LinGlide uinput permission setup script
# Run this once to enable input injection without root

set -e

echo "LinGlide uinput Permission Setup"
echo "================================="
echo

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "This script needs to be run with sudo."
    echo "Usage: sudo ./setup-uinput.sh"
    exit 1
fi

# Get the actual user (not root)
ACTUAL_USER="${SUDO_USER:-$USER}"

echo "Setting up uinput permissions for user: $ACTUAL_USER"
echo

# Create udev rule
RULES_FILE="/etc/udev/rules.d/99-linglide-uinput.rules"
echo "Creating udev rule at $RULES_FILE..."

cat > "$RULES_FILE" << 'EOF'
# LinGlide uinput access rule
# Allows members of the input group to create virtual input devices
KERNEL=="uinput", GROUP="input", MODE="0660"
EOF

echo "Created udev rule."

# Add user to input group
echo "Adding $ACTUAL_USER to input group..."
usermod -a -G input "$ACTUAL_USER"

# Reload udev rules
echo "Reloading udev rules..."
udevadm control --reload-rules
udevadm trigger

echo
echo "Setup complete!"
echo
echo "IMPORTANT: You need to log out and log back in for the group"
echo "membership to take effect."
echo
echo "After logging back in, you can run LinGlide without sudo:"
echo "  ./linglide --width 1920 --height 1080"
echo
