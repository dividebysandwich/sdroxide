#!/usr/bin/env bash
# Write a .zip that depends on nothing but the files in it:
#
#   packaging/reproducible-zip.sh out/NAME.zip stage sdroxide
#
# packs `stage/sdroxide` as `sdroxide/...`, the zip counterpart of
# reproducible-tar.sh. Every file is stamped with SOURCE_DATE_EPOCH (from
# packaging/reproducible-env.sh), entries go in sorted order, directories get
# no entries of their own, and Info-ZIP's -X leaves out the extra fields that
# would carry access times and owner ids.
#
# A zip stores local time with no zone, so it is written with TZ=UTC: the same
# epoch then reads back as the same date wherever it was built. Info-ZIP `zip`
# is the `zip` package on MSYS2 and comes with macOS; 7-Zip, which the Windows
# release used before, stores files in the order the filesystem lists them and
# adds NTFS timestamps.
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: $0 OUT.zip DIR NAME" >&2
  exit 2
fi
out=$1 dir=$2 name=$3
: "${SOURCE_DATE_EPOCH:?is not set; export what packaging/reproducible-env.sh prints first}"

if ! command -v zip >/dev/null 2>&1; then
  echo "error: Info-ZIP zip not found (on MSYS2: pacman -S zip)" >&2
  exit 1
fi

# The output path is used from inside DIR, so make it absolute first.
case "$out" in
  /*) ;;
  *) out="$PWD/$out" ;;
esac
rm -f "$out"

export TZ=UTC
# `touch -t` (CCYYMMDDhhmm.SS) rather than `-d @epoch`, which BSD touch lacks.
stamp=$(date -u -r "$SOURCE_DATE_EPOCH" +%Y%m%d%H%M.%S 2>/dev/null ||
        date -u -d "@$SOURCE_DATE_EPOCH" +%Y%m%d%H%M.%S)
cd "$dir"
find "$name" -exec touch -h -t "$stamp" {} +
find "$name" -type f | LC_ALL=C sort | zip -q -X -D -9 "$out" -@
