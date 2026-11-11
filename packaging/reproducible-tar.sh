#!/usr/bin/env bash
# Write a .tar.gz that depends on nothing but the files in it:
#
#   packaging/reproducible-tar.sh out/NAME.tar.gz stage sdroxide
#
# packs `stage/sdroxide` as `sdroxide/...`. Entries are sorted, owned by root,
# stamped with SOURCE_DATE_EPOCH (from packaging/reproducible-env.sh) and
# carry normalised permissions; gzip leaves out its own name-and-time header.
#
# GNU tar is required: it is `tar` on Linux and `gtar` on the macOS runner
# images. The macOS `tar` (bsdtar) has no --sort, and adds AppleDouble `._`
# entries and extended attributes of its own, so it is refused, not used.
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: $0 OUT.tar.gz DIR NAME" >&2
  exit 2
fi
out=$1 dir=$2 name=$3
: "${SOURCE_DATE_EPOCH:?is not set; export what packaging/reproducible-env.sh prints first}"

tar=tar
command -v gtar >/dev/null 2>&1 && tar=gtar
if ! "$tar" --version 2>/dev/null | grep -q 'GNU tar'; then
  echo "error: GNU tar not found (on macOS: brew install gnu-tar)" >&2
  exit 1
fi

"$tar" --format=gnu --sort=name --mtime="@$SOURCE_DATE_EPOCH" \
  --owner=0 --group=0 --numeric-owner --mode=u+rwX,go+rX,go-w \
  -cf - -C "$dir" "$name" | gzip -n > "$out"
