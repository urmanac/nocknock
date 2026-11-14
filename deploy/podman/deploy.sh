#!/usr/bin/env bash
# Deploy NockNock to an internal host as rootless Podman quadlets (systemd user units).
#
#   deploy/podman/deploy.sh [host]        (default host: chudmax)
#
# Ships the working tree (tracked + untracked, minus anything gitignored, so no
# config.yaml / .env.local secrets end up in an image), builds both images on the
# host, installs the quadlets and (re)starts the stack. Idempotent.
#
# Host prerequisites (one-time):  sudo apt install podman
set -euo pipefail

HOST="${1:-chudmax}"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CONFIG="${NOCKNOCK_CONFIG:-$ROOT/ingestion-worker/config.yaml}"
cd "$ROOT"

[ -f "$CONFIG" ] || { echo "missing $CONFIG (copy config.yaml.example and add your Alchemy key)" >&2; exit 1; }

# Public client id baked into the bundle at build time; optional (pairing UI degrades without it).
WC_ID="${VITE_WALLETCONNECT_PROJECT_ID:-$(sed -n 's/^VITE_WALLETCONNECT_PROJECT_ID=//p' frontend/.env.local 2>/dev/null || true)}"

echo "==> syncing source to $HOST:~/nocknock/src"
ssh "$HOST" 'rm -rf ~/nocknock/src && mkdir -p ~/nocknock/src ~/.config/containers/systemd'
git ls-files -co --exclude-standard -z | COPYFILE_DISABLE=1 tar --null -T - -cf - | ssh "$HOST" 'tar -xf - -C ~/nocknock/src'

echo "==> creating secrets (existing database password is kept)"
ssh "$HOST" 'podman secret rm nocknock-config >/dev/null 2>&1 || true; podman secret create nocknock-config - >/dev/null' < "$CONFIG"
ssh "$HOST" 'set -e
  if ! podman secret exists nocknock-db-password; then
    pw=$(head -c 24 /dev/urandom | od -An -tx1 | tr -d " \n")
    printf %s "$pw" | podman secret create nocknock-db-password - >/dev/null
    printf %s "postgresql://postgres:$pw@db:5432/nocknock" | podman secret create nocknock-database-url - >/dev/null
  fi'

echo "==> building images on $HOST"
ssh "$HOST" "set -e; cd ~/nocknock/src
  podman build -t localhost/nocknock-worker:latest ingestion-worker
  podman build -t localhost/nocknock-frontend:latest --build-arg VITE_WALLETCONNECT_PROJECT_ID='$WC_ID' frontend"

echo "==> installing quadlets and restarting"
ssh "$HOST" 'set -e
  cp ~/nocknock/src/deploy/podman/*.container ~/nocknock/src/deploy/podman/*.network ~/nocknock/src/deploy/podman/*.volume ~/.config/containers/systemd/
  loginctl enable-linger
  systemctl --user daemon-reload
  systemctl --user restart nocknock-db.service nocknock-worker.service nocknock-frontend.service
  for i in $(seq 1 30); do curl -fsS -m 3 http://localhost:3000/health && break; sleep 2; done; echo
  systemctl --user --no-pager --lines=0 status nocknock-db nocknock-worker nocknock-frontend | grep -E "●|Active:"'

echo "==> dashboard: http://$HOST:3000   API/MCP: http://$HOST:8081"
