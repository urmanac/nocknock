#!/bin/bash
set -euo pipefail

# Project startup script — runs at every agent session start.
#
# Idempotent by design: sandbox resets wipe the home directory (including
# ~/.cargo) and OS packages between sessions, while /home/retro/work persists.
# Everything needed to build and test the nocknock Rust worker is reinstalled
# here when missing. Safe to re-run at any time.

echo "🚀 Starting project: nocknock"

# --- System build deps (gcc, libc headers, pkg-config, OpenSSL headers) ---
if ! command -v cc >/dev/null 2>&1 || ! command -v pkg-config >/dev/null 2>&1 || ! ls /usr/include/openssl/ssl.h >/dev/null 2>&1; then
  echo "Installing system build deps (gcc, libc6-dev, pkg-config, libssl-dev)..."
  sudo -n apt-get update -qq || true
  sudo -n apt-get install -y -qq gcc libc6-dev pkg-config libssl-dev || true
fi

# --- Rust toolchain (rustup, minimal profile; ~/.cargo resets with the sandbox) ---
if ! command -v cargo >/dev/null 2>&1; then
  echo "Installing Rust toolchain (rustup, minimal profile, stable)..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable >/dev/null 2>&1 ||     curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable
fi
export PATH="$HOME/.cargo/bin:$PATH"
cargo --version || echo "WARN: cargo still unavailable; rerun this script or install manually"

# --- Node frontend deps (node_modules is not committed) ---
# Frontend deps install on demand: cd frontend && npm install --legacy-peer-deps

echo "✅ Project startup complete"
