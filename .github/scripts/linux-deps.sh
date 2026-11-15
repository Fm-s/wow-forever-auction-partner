#!/usr/bin/env bash
# System libraries Tauri 2 needs on Debian/Ubuntu, plus libdbus for the Secret Service keyring backend.
set -euo pipefail
sudo apt-get update
sudo apt-get install -y --no-install-recommends \
  build-essential curl wget file pkg-config \
  libwebkit2gtk-4.1-dev libxdo-dev libssl-dev \
  libayatana-appindicator3-dev librsvg2-dev \
  libdbus-1-dev patchelf
