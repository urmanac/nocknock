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
#
# Opt-in HTTPS for phones (issue #89; off unless asked for):
#   NOCKNOCK_TLS=1 deploy/podman/deploy.sh [host]
#       also installs and starts the Caddy proxy in deploy/podman/tls/ on an
#       unprivileged port. NOCKNOCK_TLS_PORT (default 3443) picks the port and
#       NOCKNOCK_TLS_NAMES (default: the host argument) lists every name or IPv4 address
#       the phone will type, separated by spaces.
#   NOCKNOCK_TLS=remove deploy/podman/deploy.sh [host]
#       stops the proxy and removes its quadlet; its CA volume is kept.
# With NOCKNOCK_TLS unset this script does not install, start, restart or remove it.
# See docs/kb/how-to-phone-install-notifications-tls.md before turning it on.
set -euo pipefail

HOST="${1:-chudmax}"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CONFIG="${NOCKNOCK_CONFIG:-$ROOT/ingestion-worker/config.yaml}"
cd "$ROOT"

[ -f "$CONFIG" ] || { echo "missing $CONFIG (copy config.yaml.example and add your Alchemy key)" >&2; exit 1; }

# Opt-in TLS proxy (#89). Checked and rendered here, before anything is shipped, so a
# bad port or name stops the deploy instead of half-finishing it.
TLS="${NOCKNOCK_TLS:-}"
case "$TLS" in
  ""|remove) ;;
  1)
    TLS_PORT="${NOCKNOCK_TLS_PORT:-3443}"
    read -r -a TLS_NAMES <<< "${NOCKNOCK_TLS_NAMES:-$HOST}"
    [ "${#TLS_NAMES[@]}" -gt 0 ] || { echo "NOCKNOCK_TLS_NAMES is blank: list at least one name or address" >&2; exit 1; }
    TLS_DIR="$(mktemp -d)"
    trap 'rm -rf "$TLS_DIR"' EXIT
    deploy/podman/tls/render.sh "$TLS_DIR" "$TLS_PORT" "${TLS_NAMES[@]}"
    ;;
  *) echo "NOCKNOCK_TLS must be unset, 1 or remove (got '$TLS')" >&2; exit 1 ;;
esac

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

if [ "$TLS" = "1" ]; then
  echo "==> installing the opt-in TLS proxy on port $TLS_PORT for: ${TLS_NAMES[*]}"
  ssh "$HOST" 'mkdir -p ~/nocknock/tls && cat > ~/nocknock/tls/Caddyfile' < "$TLS_DIR/Caddyfile"
  ssh "$HOST" 'cat > ~/.config/containers/systemd/nocknock-tls.container' < "$TLS_DIR/nocknock-tls.container"
  ssh "$HOST" "set -e
    cp ~/nocknock/src/deploy/podman/tls/nocknock-tls-data.volume ~/.config/containers/systemd/
    systemctl --user daemon-reload
    systemctl --user restart nocknock-tls.service
    for i in \$(seq 1 30); do curl -fsSk -m 3 --connect-to '${TLS_NAMES[0]}:$TLS_PORT:127.0.0.1:$TLS_PORT' 'https://${TLS_NAMES[0]}:$TLS_PORT/health' && break; sleep 2; done; echo
    systemctl --user --no-pager --lines=0 status nocknock-tls | grep -E '●|Active:'
    echo 'root CA SHA-256 fingerprint (compare it on the phone before you trust the certificate):'
    podman exec nocknock-tls cat /data/caddy/pki/authorities/local/root.crt | openssl x509 -noout -fingerprint -sha256 || echo '  not read; run deploy/podman/tls/export-root-ca.sh'"
elif [ "$TLS" = "remove" ]; then
  echo "==> removing the TLS proxy (the nocknock-tls-data volume, which holds its CA, is kept)"
  ssh "$HOST" 'systemctl --user stop nocknock-tls.service 2>/dev/null || true
    rm -f ~/.config/containers/systemd/nocknock-tls.container ~/.config/containers/systemd/nocknock-tls-data.volume
    systemctl --user daemon-reload'
fi

echo "==> dashboard: http://$HOST:3000   API/MCP: http://$HOST:8081"
if [ "$TLS" = "1" ]; then
  echo "==> HTTPS dashboard: https://${TLS_NAMES[0]}:$TLS_PORT   (trusted only after the root CA is installed on the device)"
fi
