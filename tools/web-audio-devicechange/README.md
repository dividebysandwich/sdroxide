# Web output-device harness

**Does the browser client follow a headset connected after the page opened?**

    ./tools/web-audio-devicechange/run.sh      # or run.ps1 on Windows

The report: the web client plays through the speakers, a Bluetooth headset is
connected while the page is open, and the sound is then heard in neither. A
context that is already running is not moved onto a new default output —
Chromium leaves it on the device it was built with, which the OS has by then
deactivated — while the worklet goes on rendering into nothing, so the page is
silent with no symptom at all.

A headless browser cannot be given a headset, so the harness drives the
**mechanism** the cure rests on, against the **real** bridge scripts
(`crates/sdroxide-web/assets/audio_bridge.js` and `pcm_worklet.js`) rather than a
copy of them:

1. the page takes a gesture and the bridge starts playing, as it does on a real
   first click;
2. a `devicechange` from `navigator.mediaDevices` stands in for the headset
   arriving, and the page must build a fresh context — the only way to follow the
   new default — which must be rendering;
3. where the browser exposes `setSinkId`, that context must ask for the default
   sink (`""`) rather than trust the device it happened to open with;
4. a device change while the page is hidden must be deferred to the moment it
   comes back, not rebuilt into a background tab the browser will undo.

Every assertion is on the browser's own `AudioContext`, seen through wrappers the
harness installs before the bridge loads, so this fails on a build that does
nothing rather than only on one that cannot report. On the build before the fix
it ends at:

    FAIL the output device changed and the bridge did not rebuild: {"contexts":1,...}

Nothing is written inside the tree: the scripts are copied to a scratch
directory (`$TMPDIR/sdroxide-web-devicechange`) and served from there.

Needs `python3` and a Chromium-family browser (Chrome, Chromium or Edge); the
browser runs with autoplay allowed, since a synthetic gesture cannot grant real
transient activation.
