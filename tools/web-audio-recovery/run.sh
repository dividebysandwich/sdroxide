#!/bin/sh
# **Does the browser client get its sound back after a long spell hidden?**
#
#   ./tools/web-audio-recovery/run.sh
#
# The page is driven in a headless browser against the *real* bridge scripts,
# served from a scratch directory so nothing is written inside the tree. See
# `harness.js` for what it does and what it asserts; this script only builds the
# scratch copy, serves it, and reports what the page said.
#
# Needs python3 and a Chromium-family browser (chrome, chromium or msedge).
set -eu

here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
build="${TMPDIR:-/tmp}/sdroxide-web-audio-recovery"
raw="${TMPDIR:-/tmp}/sdroxide-web-audio-recovery.txt"
port="${PORT:-8731}"

browser=""
for c in google-chrome google-chrome-stable chromium chromium-browser msedge microsoft-edge; do
    if command -v "$c" >/dev/null 2>&1; then browser="$c"; break; fi
done
if [ -z "$browser" ]; then
    echo "no Chromium-family browser found (chrome, chromium or msedge)" >&2
    exit 2
fi

rm -rf "$build" "$raw"
mkdir -p "$build"
cp "$root/crates/sdroxide-web/assets/audio_bridge.js" "$build/"
cp "$root/crates/sdroxide-web/assets/pcm_worklet.js" "$build/"
cp "$here/page.html" "$here/harness.js" "$here/serve.py" "$build/"

python3 "$build/serve.py" "$port" "$raw" &
server=$!
trap 'kill $server 2>/dev/null || true' EXIT
sleep 2

# --disable-sync: otherwise Edge opens its own sign-in panel instead of the
# page, and nothing runs.
"$browser" \
    --headless=new --disable-gpu --no-first-run --no-default-browser-check \
    --disable-sync --disable-extensions \
    --autoplay-policy=no-user-gesture-required \
    --user-data-dir="$build/profile.$$" \
    "http://127.0.0.1:$port/page.html" >/dev/null 2>&1 &
client=$!
sleep 22
kill $client 2>/dev/null || true
pkill -f "user-data-dir=$build/profile" 2>/dev/null || true
kill $server 2>/dev/null || true

echo "--- the page said ---"
cat "$raw" 2>/dev/null || echo "(nothing)"
if grep -q "^line=PASS " "$raw" 2>/dev/null; then
    echo "--- PASS ---"
    exit 0
fi
echo "--- FAIL ---" >&2
exit 1
