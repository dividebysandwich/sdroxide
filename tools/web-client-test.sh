#!/bin/sh
# **Serve the browser client so it can be driven and looked at.**
#
# The web client is the same UI compiled to wasm, and it is the one nobody here
# looks at: the desktop build gets every screenshot, the browser gets tested when
# somebody happens to open it. Reported from the browser on 2026-10-18 (fork
# discussion #90): labels drawn on top of each other and notes running off the
# panel — on a page that is fine in the desktop build.
#
# So this builds it fresh and serves it **beside** the running program, on a
# config of its own, so the real radios stay with the GUI:
#
#   ./tools/web-client-test.sh
#
# Then open http://localhost:4951/ — no password, the station has none. The
# test station carries a **HackRF** radio, which is the one the browser
# screenshots were of, and its other radios are switched to `none` so this
# process cannot take the RSP1 or the CRT away from the program you are using.
#
# Re-run it after a change: it rebuilds the wasm and restarts the server.
set -eu

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

dist="crates/sdroxide-web/dist"
config="${SDROXIDE_WEBTEST_CONFIG:-$HOME/.config/sdroxide-webtest}"
port="${SDROXIDE_WEBTEST_PORT:-4951}"

echo "== building the browser client"
(cd crates/sdroxide-web && trunk build --release)

echo "== preparing a test station in $config"
python3 - "$config" <<'PY'
import json, os, sys

test = sys.argv[1]
os.makedirs(test, exist_ok=True)

# **A station of its own, with one HackRF on it.** That is the radio the browser
# screenshots were of (LNA/VGA gain, the baseband filter, the tone sections),
# and it is deliberately *not* a copy of the live station: the program you are
# using keeps its radios, and this process cannot take one of them away.
radios = {"radios": [{"id": 0, "name": "HackRF (web test)"}], "next_id": 1, "order": [0]}
json.dump(radios, open(os.path.join(test, "radios.json"), "w"), indent=2)

r0 = os.path.join(test, "radio-0")
os.makedirs(r0, exist_ok=True)
radio = os.path.join(r0, "radio.json")
if not os.path.exists(radio):
    json.dump({"backend": "HackRf"}, open(radio, "w"), indent=2)
    print("   wrote radio-0 as a HackRF")
print("   test station ready (no password, one HackRF, not attached)")
PY

echo "== serving it on http://localhost:$port/"
echo "   (Ctrl-C to stop; the program you are using is untouched)"
SDROXIDE_CONFIG_DIR="$config" exec sdroxide --server --web-root "$dist" --port "$port"
