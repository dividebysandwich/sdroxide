#!/usr/bin/env bash
# Copy staged rs1729 *mod helpers into a package directory (beside sdroxide).
# Used by the release workflow on Linux, macOS, and Windows (MSYS bash).
#
# Set REQUIRE_SONDE=1 to fail if helpers are missing (CI packaging).
set -euo pipefail
DEST="${1:?destination directory}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$ROOT/target/sonde-mod/release"
if [[ ! -d "$SRC" ]]; then
  echo "copy-sonde-mods: no $SRC (build without --features sonde?)" >&2
  if [[ "${REQUIRE_SONDE:-}" == "1" ]]; then
    exit 1
  fi
  exit 0
fi
mkdir -p "$DEST"
copied=0
for name in rs41mod dfm09mod m10mod m10m20mod m20mod mts01mod \
            rs41mod.exe dfm09mod.exe m10mod.exe m10m20mod.exe m20mod.exe mts01mod.exe; do
  if [[ -f "$SRC/$name" ]]; then
    cp "$SRC/$name" "$DEST/"
    copied=$((copied + 1))
  fi
done
if [[ -f "$ROOT/vendor/rs1729/LICENSE" ]]; then
  cp "$ROOT/vendor/rs1729/LICENSE" "$DEST/LICENSE-rs1729.txt"
fi
if [[ -f "$ROOT/crates/sdroxide-rs41/PROVENANCE.md" ]]; then
  cp "$ROOT/crates/sdroxide-rs41/PROVENANCE.md" "$DEST/PROVENANCE-rs1729.md"
fi
# Need at least one of rs41mod / rs41mod.exe
if [[ ! -f "$DEST/rs41mod" && ! -f "$DEST/rs41mod.exe" ]]; then
  echo "copy-sonde-mods: rs41mod missing under $SRC (copied=$copied)" >&2
  if [[ "${REQUIRE_SONDE:-}" == "1" ]]; then
    ls -la "$SRC" >&2 || true
    exit 1
  fi
fi
echo "copy-sonde-mods: staged $copied helper(s) into $DEST"
