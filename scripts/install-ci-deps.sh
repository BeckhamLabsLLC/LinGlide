#!/usr/bin/env bash
# Install system dependencies for CI builds
set -euo pipefail

sudo apt-get update
sudo apt-get install -y \
    build-essential \
    pkg-config \
    libxcb1-dev \
    libxcb-shm0-dev \
    libxcb-randr0-dev \
    libxcb-render0-dev \
    libpipewire-0.3-dev \
    libspa-0.2-dev \
    libclang-dev \
    libevdev-dev \
    libgtk-3-dev \
    libssl-dev \
    libdrm-dev \
    libxdo-dev \
    evdi-dkms
