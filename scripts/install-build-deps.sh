#!/usr/bin/env bash
# System packages needed to build RustBot Creator itself (gpui's native libraries).
# Used by CI and by developers. The bots' own build needs are handled by the app (D-017, D-039).
set -euo pipefail

if command -v dnf >/dev/null; then
  dnf install -y \
    gcc gcc-c++ pkgconf-pkg-config \
    fontconfig-devel wayland-devel libxkbcommon-devel libxkbcommon-x11-devel \
    libxcb-devel libX11-devel openssl-devel libzstd-devel alsa-lib-devel
elif command -v apt-get >/dev/null; then
  apt-get update
  apt-get install -y \
    gcc g++ pkg-config \
    libfontconfig-dev libwayland-dev libxkbcommon-dev libxkbcommon-x11-dev \
    libxcb1-dev libx11-xcb-dev libssl-dev libzstd-dev libasound2-dev
else
  echo "Unsupported package manager: install the equivalents of the packages above." >&2
  exit 1
fi
