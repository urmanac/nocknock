#!/usr/bin/env bash
# Copy the TLS proxy's root certificate to this machine and print its fingerprint (#89).
#
#   deploy/podman/tls/export-root-ca.sh [host] [out-file]
#       (defaults: chudmax, ./nocknock-root-ca.crt)
#
# This is the PUBLIC certificate of the private CA that Caddy created on the host. It is
# what a phone installs to trust https://<host>:<port>. The CA's private key stays in
# the nocknock-tls-data volume on the host and is never copied.
#
# Compare the SHA-256 fingerprint printed here with the one the phone shows before you
# trust the certificate there.
set -euo pipefail

HOST="${1:-chudmax}"
OUT="${2:-nocknock-root-ca.crt}"

ssh "$HOST" 'podman exec nocknock-tls cat /data/caddy/pki/authorities/local/root.crt' > "$OUT"
grep -q 'BEGIN CERTIFICATE' "$OUT" || { echo "no certificate came back from $HOST (is nocknock-tls running?)" >&2; rm -f "$OUT"; exit 1; }
echo "wrote $OUT"
openssl x509 -in "$OUT" -noout -subject -enddate -fingerprint -sha256
