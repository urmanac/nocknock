#!/usr/bin/env bash
# Regenerate the home-screen icons from the concept art already in the repo.
#
#   frontend/scripts/make-icons.sh
#
# The repo holds no logo: the only brand assets are the two concept images in assets/.
# The icons are a crop of the "N" roundel in assets/image_6cc4c3c4.jpg (the Sovereign
# Treasury panel), scaled up. Nothing is downloaded and nothing is drawn. To use a real
# logo instead, point SRC at a square image and set the crops to its full size.
#
# Needs `sips` (ships with macOS). The PNGs it writes are committed, so a build never
# runs this script.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SRC="${SRC:-$ROOT/assets/image_6cc4c3c4.jpg}"
OUT="$ROOT/frontend/public/icons"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

command -v sips >/dev/null || { echo "sips not found (macOS only)" >&2; exit 1; }
mkdir -p "$OUT"

# The roundel is about 50 px across, centred near x=1287, y=385 in the 1408x768 source.
# sips takes the crop size as height width and the offset as top left.
#  - tight: 64 px square, the roundel fills ~80% of it (regular icons).
#  - loose: 84 px square, the roundel fills ~60% of it, inside the 80% safe zone a
#    maskable icon must keep clear (Android crops these to a circle or squircle).
sips -c 64 64 --cropOffset 353 1255 "$SRC" -s format png --out "$TMP/tight.png" >/dev/null
sips -c 84 84 --cropOffset 346 1245 "$SRC" -s format png --out "$TMP/loose.png" >/dev/null

resize() { # <source> <pixels> <name>
  sips -z "$2" "$2" "$1" --out "$OUT/$3" >/dev/null
  echo "wrote $OUT/$3 (${2}x${2})"
}

resize "$TMP/tight.png" 32  favicon-32.png
resize "$TMP/tight.png" 180 apple-touch-icon.png
resize "$TMP/tight.png" 192 icon-192.png
resize "$TMP/tight.png" 512 icon-512.png
resize "$TMP/loose.png" 192 icon-maskable-192.png
resize "$TMP/loose.png" 512 icon-maskable-512.png
