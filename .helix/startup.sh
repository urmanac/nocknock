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

# --- PostgreSQL (needed by the ingestion worker at boot) ---
if ! command -v psql >/dev/null 2>&1; then
  echo "Installing PostgreSQL..."
  sudo -n apt-get install -y -qq postgresql || true
fi
if command -v pg_isready >/dev/null 2>&1; then
  sudo -n service postgresql start >/dev/null 2>&1 || true
  sleep 1
  if ! sudo -n -u postgres psql -tAc "SELECT 1 FROM pg_database WHERE datname='nocknock'" | grep -q 1; then
    sudo -n -u postgres psql -qc "CREATE DATABASE nocknock;" || true
    sudo -n -u postgres psql -qc "ALTER USER postgres PASSWORD 'postgres';" || true
    echo "Created database nocknock"
  fi
  pg_isready -q && echo "PostgreSQL ready" || echo "WARN: PostgreSQL not ready"
fi

# --- Alchemy key status (presence only - the value is never printed) ---
if [ -n "${ALCHEMY_API_KEY:-}" ]; then
  echo "Alchemy key: present in environment (wiring below)"
else
  echo "Alchemy key: not in environment (helix secret injects at container creation)"
fi


# --- Alchemy key wiring (never echoed, never committed) ---
# Two supported delivery channels, in priority order:
#   1. Helix project secret injected as $ALCHEMY_API_KEY at container creation
#      (operator: helix secret create -n ALCHEMY_API_KEY -v <key> -p <project-id>)
#   2. A config file copied into the sandbox by the operator at
#      /home/retro/work/nocknock/ingestion-worker/config.yaml (gitignored)
# The worker reads ingestion-worker/config.yaml with higher priority than the
# example config; that path is covered by ingestion-worker/.gitignore.
CONFIG_DIR=/home/retro/work/nocknock/ingestion-worker
if [ -n "${ALCHEMY_API_KEY:-}" ] && [ ! -f "$CONFIG_DIR/config.yaml" ]; then
  echo "Wiring ALCHEMY_API_KEY from environment into gitignored ingestion-worker/config.yaml"
  {
    echo 'database_url: "postgresql://postgres:postgres@localhost:5432/nocknock"'
    echo 'alchemy_api_keys:'
    echo "  - "$ALCHEMY_API_KEY""
  } > "$CONFIG_DIR/config.yaml"
fi

# --- Node frontend deps (node_modules is not committed) ---
# Frontend deps install on demand: cd frontend && npm install --legacy-peer-deps

echo "✅ Project startup complete"
