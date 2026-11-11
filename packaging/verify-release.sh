#!/usr/bin/env bash
# Print the SHA-256 of the binaries inside a published sdroxide release, or
# compare them with ones you built yourself.
#
#   packaging/verify-release.sh v1.8.0
#       the hash of every reproducible binary in the release
#   packaging/verify-release.sh v1.8.0 linux-x86_64-compat=target/rebuild/sdroxide
#       MATCH or DIFFER for your rebuild (exit status 1 on DIFFER)
#   packaging/verify-release.sh v1.8.0 web=crates/sdroxide-web/dist
#       the web client, file by file
#
# Options, before the tag:
#   --repo OWNER/NAME  where the release is (default dividebysandwich/sdroxide)
#   --keep DIR         leave the released binaries in DIR/<platform>/, for
#                      diffoscope when something differs
#
# It is the binaries that are compared, not the release assets: a .tar.gz or
# .zip is checked against the release's SHA256SUMS and then opened, and what is
# compared is the sdroxide (or sdroxide.exe) inside it. That is what a rebuild
# reproduces. Only the standard (non-SoapySDR) build of these platforms is
# expected to reproduce — the ones with a pinned build environment:
#
#   linux-x86_64-compat linux-aarch64-compat macos-aarch64 macos-x86_64
#   windows-x86_64 web
#
# The .dmg, .msi, .AppImage and .deb are not: the tools that make them stamp
# them with the time and a random ID. They carry the same binary, though.
#
# How to make the rebuild to compare with: docs/reproducible-builds.md.
#
# Needs curl (or an authenticated gh), tar, and unzip or python3. Written for
# the bash 3.2 macOS ships; on Windows, run it from Git Bash or MSYS2.
set -euo pipefail

all_platforms="linux-x86_64-compat linux-aarch64-compat macos-aarch64 macos-x86_64 windows-x86_64 web"
repo=dividebysandwich/sdroxide
keep=""

die() { echo "verify-release: $*" >&2; exit 2; }
usage() { sed -n '2,/^set -euo/p' "$0" | sed '$d; s/^# \{0,1\}//' >&2; exit 2; }

while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo) repo=${2:?--repo needs OWNER/NAME}; shift 2 ;;
    --keep) keep=${2:?--keep needs a directory}; shift 2 ;;
    -h | --help) usage ;;
    -*) die "unknown option $1" ;;
    *) break ;;
  esac
done
[[ $# -ge 1 ]] || usage
tag=$1
shift
[[ $# -gt 0 ]] || set -- $all_platforms
for arg in "$@"; do
  case " $all_platforms " in
    *" ${arg%%=*} "*) ;;
    *) die "unknown platform '${arg%%=*}' (one of: $all_platforms)" ;;
  esac
done

if command -v sha256sum > /dev/null 2>&1; then
  hash_of() { sha256sum -- "$1" | cut -d' ' -f1; }
elif command -v shasum > /dev/null 2>&1; then
  hash_of() { shasum -a 256 -- "$1" | cut -d' ' -f1; }
else
  die "needs sha256sum or shasum"
fi

work=$(mktemp -d "${TMPDIR:-/tmp}/verify-release.XXXXXX")
trap 'rm -rf "$work"' EXIT

# gh when it is logged in (no rate limit, works for drafts), else plain curl:
# release assets of a public repository need no authentication.
try_fetch() {
  local asset=$1
  [[ -f "$work/$asset" ]] && return 0
  if command -v gh > /dev/null 2>&1 && gh auth status > /dev/null 2>&1; then
    if gh release download "$tag" --repo "$repo" --pattern "$asset" --dir "$work" > /dev/null 2>&1; then
      return 0
    fi
  fi
  curl -fsSL --retry 3 -o "$work/$asset" \
    "https://github.com/$repo/releases/download/$tag/$asset" 2> /dev/null
}
fetch() {
  try_fetch "$1" || die "could not download $1 from $repo release $tag"
}

# Git Bash on Windows has no unzip; the runners and most machines have one of
# the others.
unpack_zip() {
  if command -v unzip > /dev/null 2>&1; then
    unzip -q "$1" -d "$2"
  elif command -v python3 > /dev/null 2>&1; then
    python3 -I -m zipfile -e "$1" "$2"
  elif command -v python > /dev/null 2>&1; then
    python -I -m zipfile -e "$1" "$2"
  elif command -v 7z > /dev/null 2>&1; then
    7z x -y -bd "-o$2" "$1" > /dev/null
  else
    die "needs unzip, python3 or 7z to open $1"
  fi
}

# Downloads an asset, checks it against SHA256SUMS and unpacks it into
# $work/<platform>/. Every archive is checked, so a corrupted or tampered
# download fails here rather than as a confusing DIFFER further down.
# Releases from before reproducible builds have none; their binaries were not
# built to reproduce either, but the hashes can still be printed.
sums=1
if ! try_fetch "SHA256SUMS"; then
  echo "verify-release: $repo release $tag has no SHA256SUMS; the downloads are not checked" >&2
  sums=0
fi
open_asset() {
  local platform=$1 asset=$2 want got
  fetch "$asset"
  [[ $sums == 1 ]] || { unpack "$platform" "$asset"; return; }
  want=$(awk -v a="$asset" '{ n = $2; sub(/^\*/, "", n) } n == a { print $1 }' "$work/SHA256SUMS")
  [[ -n "$want" ]] || die "$asset is not in the release's SHA256SUMS"
  got=$(hash_of "$work/$asset")
  [[ "$got" == "$want" ]] || die "$asset does not match SHA256SUMS ($got, expected $want)"
  unpack "$platform" "$asset"
}
unpack() {
  local platform=$1 asset=$2
  mkdir -p "$work/$platform"
  case "$asset" in
    *.tar.gz) tar -xzf "$work/$asset" -C "$work/$platform" ;;
    *.zip) unpack_zip "$work/$asset" "$work/$platform" ;;
  esac
}

# "<hash>  <relative path>" for every file under a directory, sorted by path.
hash_tree() {
  (cd "$1" && find . -type f | LC_ALL=C sort | while IFS= read -r f; do
    printf '%s  %s\n' "$(hash_of "$f")" "${f#./}"
  done)
}

status=0
for arg in "$@"; do
  platform=${arg%%=*}
  mine=""
  [[ "$arg" == *=* ]] && mine=${arg#*=}
  case "$platform" in
    web) asset="sdroxide-web-$tag.zip"; inner="" ;;
    windows-*) asset="sdroxide-$tag-$platform.zip"; inner="sdroxide/sdroxide.exe" ;;
    *) asset="sdroxide-$tag-$platform.tar.gz"; inner="sdroxide/sdroxide" ;;
  esac
  open_asset "$platform" "$asset"

  if [[ -n "$keep" ]]; then
    mkdir -p "$keep/$platform"
    if [[ -z "$inner" ]]; then
      cp -R "$work/$platform/." "$keep/$platform/"
    else
      cp "$work/$platform/$inner" "$keep/$platform/"
    fi
  fi

  if [[ -z "$inner" ]]; then
    released=$(hash_tree "$work/$platform")
    if [[ -z "$mine" ]]; then
      printf '%s\n' "$released" | sed "s|  |  $platform/|"
      continue
    fi
    [[ -d "$mine" ]] || die "$mine is not a directory (web= takes the dist directory)"
    if diff -u <(printf '%s\n' "$released") <(hash_tree "$mine") > "$work/web.diff"; then
      echo "MATCH   $platform ($(printf '%s\n' "$released" | wc -l | tr -d ' ') files)"
    else
      echo "DIFFER  $platform  (- release, + yours)"
      sed '1,2d' "$work/web.diff"
      status=1
    fi
    continue
  fi

  released=$(hash_of "$work/$platform/$inner")
  if [[ -z "$mine" ]]; then
    echo "$released  $platform/${inner#sdroxide/}"
    continue
  fi
  [[ -f "$mine" ]] || die "$mine: no such file"
  yours=$(hash_of "$mine")
  if [[ "$yours" == "$released" ]]; then
    echo "MATCH   $platform  $released"
  else
    echo "DIFFER  $platform  release $released, yours $yours"
    status=1
  fi
done
exit $status
