#!/usr/bin/env bash
# Render the opt-in TLS proxy's Caddyfile and quadlet (#89).
#
#   deploy/podman/tls/render.sh <out-dir> <port> <name> [<name>...]
#
# <port>  the HTTPS port to publish on the host: unprivileged (1024-65535), and not one
#         NockNock or Strata already uses.
# <name>  every hostname or IPv4 address the dashboard will be opened as, exactly as it
#         is typed on the phone (for example: chudmax chudmax.local 192.168.1.20). The
#         certificate covers these names and no others.
#
# Writes <out-dir>/Caddyfile and <out-dir>/nocknock-tls.container. Touches nothing else,
# so it can be run anywhere to see what a deploy would install. deploy.sh calls it only
# when NOCKNOCK_TLS=1.
set -euo pipefail
export LC_ALL=C

usage() { sed -n '2,13p' "$0" | sed 's/^# \{0,1\}//' >&2; exit 2; }
[ "$#" -ge 3 ] || usage

OUT="$1"; PORT="$2"; shift 2
HERE="$(cd "$(dirname "$0")" && pwd)"

case "$PORT" in
  ''|*[!0-9]*) echo "render.sh: port '$PORT' is not a number" >&2; exit 1 ;;
  # 08080 is 8080 to Podman but not to the string comparisons below, so it would get past
  # the reserved-port check. More than five digits can only be out of range.
  0*|??????*) echo "render.sh: write the port as a plain number from 1024 to 65535 (got '$PORT')" >&2; exit 1 ;;
esac
if [ "$PORT" -lt 1024 ] || [ "$PORT" -gt 65535 ]; then
  echo "render.sh: port $PORT is outside 1024-65535 (rootless Podman cannot publish a privileged port)" >&2
  exit 1
fi
case "$PORT" in
  8080) echo "render.sh: host port 8080 belongs to Strata; pick another" >&2; exit 1 ;;
  3000|8081) echo "render.sh: host port $PORT is NockNock's own plain-HTTP port; pick another" >&2; exit 1 ;;
esac

SITES=""
FIRST_IP=""
for name in "$@"; do
  # Hostnames and IPv4 addresses only. Anything else (a scheme, a port, a path, a space)
  # would change the meaning of the Caddyfile line it lands in.
  case "$name" in
    ''|*[!A-Za-z0-9.-]*|[.-]*|*[.-])
      echo "render.sh: '$name' is not a hostname or IPv4 address (give names only: no https://, no port, no path)" >&2
      exit 1 ;;
  esac
  SITES="${SITES:+$SITES, }https://$name:$PORT"
  case "$name" in
    *[!0-9.]*) ;;
    *) [ -n "$FIRST_IP" ] || FIRST_IP="$name" ;;
  esac
done

# A browser opening https://<ip>:<port> sends no server name; default_sni tells Caddy
# which certificate to answer with then: the first IP address given, if any.
if [ -n "$FIRST_IP" ]; then
  DEFAULT_SNI="default_sni $FIRST_IP"
else
  DEFAULT_SNI="# (no IP address was given, so https://<ip> is not served: open the dashboard by name)"
fi

mkdir -p "$OUT"
sed -e "s|@SITES@|$SITES|" -e "s|@DEFAULT_SNI@|$DEFAULT_SNI|" "$HERE/Caddyfile.template" > "$OUT/Caddyfile"
sed "s|@PORT@|$PORT|g" "$HERE/nocknock-tls.container.template" > "$OUT/nocknock-tls.container"

if grep -q '@[A-Z]*@' "$OUT/Caddyfile" "$OUT/nocknock-tls.container"; then
  echo "render.sh: a placeholder was left unrendered" >&2
  exit 1
fi
echo "rendered $OUT/Caddyfile and $OUT/nocknock-tls.container for: $SITES" >&2
