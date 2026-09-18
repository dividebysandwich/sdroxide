#!/usr/bin/env bash
# Regenerate every rasterised icon from sdroxide.svg.
#
#   ./packaging/icons/render.sh
#
# Needs librsvg (rsvg-convert) and ImageMagick. The outputs are committed, so
# builds and packaging never depend on either tool being installed.
set -euo pipefail
cd "$(dirname "$0")"

SIZES=(16 24 32 48 64 128 192 256 512)

for s in "${SIZES[@]}"; do
  rsvg-convert -w "$s" -h "$s" sdroxide.svg -o "sdroxide-${s}.png"
done

# Windows: multi-resolution .ico for the MSI's Start-menu shortcut and the
# Add/Remove Programs entry. 256 is stored as PNG, the rest as BMP.
magick sdroxide-16.png sdroxide-24.png sdroxide-32.png sdroxide-48.png \
       sdroxide-64.png sdroxide-128.png sdroxide-256.png \
       -colors 256 ../windows/sdroxide.ico

# macOS: .icns for the .app bundle. Built by hand because iconutil is macOS
# only and ImageMagick's ICNS writer does not emit the retina variants.
python3 - <<'PY'
import struct

# (icns OSType, pixel size). Only the ic07..ic14 PNG-payload types are used —
# the older icp4/icp5 slots render incorrectly on some macOS versions when fed
# PNG, and Finder scales the 128pt entry down for the small sizes anyway.
ENTRIES = [
    (b"ic07", 128),  # 128pt @1x
    (b"ic08", 256),  # 256pt @1x
    (b"ic09", 512),  # 512pt @1x
    (b"ic11", 32),   # 16pt  @2x
    (b"ic12", 64),   # 32pt  @2x
    (b"ic13", 256),  # 128pt @2x
    (b"ic14", 512),  # 256pt @2x
]

chunks = b""
for ostype, size in ENTRIES:
    with open(f"sdroxide-{size}.png", "rb") as fh:
        payload = fh.read()
    chunks += ostype + struct.pack(">I", len(payload) + 8) + payload

with open("../macos/sdroxide.icns", "wb") as out:
    out.write(b"icns" + struct.pack(">I", len(chunks) + 8) + chunks)
PY

echo "rendered: ${SIZES[*]} + ../windows/sdroxide.ico + ../macos/sdroxide.icns"
