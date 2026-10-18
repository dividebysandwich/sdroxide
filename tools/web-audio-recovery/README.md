# Web audio recovery harness

**Does the browser client get its sound back after a long spell in the
background?**

    ./tools/web-audio-recovery/run.sh          # or run.ps1 on Windows

A window left in the taskbar for a long spell is a page the browser stops
drawing, freezes, or drops the audio device under — and the `AudioContext` goes
with it. The page is driven in a headless browser against the **real** bridge
scripts (`crates/sdroxide-web/assets/audio_bridge.js` and `pcm_worklet.js`) so
that what is tested is the code the client runs, not a copy of it:

1. the page takes a gesture and the bridge starts playing, as it does on a real
   first click;
2. receive audio is pushed the way the wasm client pushes it, one 960-sample
   block every 20 ms;
3. the page goes hidden and the playback context dies while nobody is looking;
4. the page comes back, and the audio has to be running again.

Every assertion is on the browser's own `AudioContext`, seen through a wrapper
the harness installs before the bridge loads, so this fails on a build that has
no way to recover rather than only on one that cannot report. On the build
before the fix it ends at:

    FAIL the window came back and the sound did not: {"contexts":1,"states":["closed"]}

Nothing is written inside the tree: the scripts are copied to a scratch
directory (`$TMPDIR/sdroxide-web-audio-recovery`) and served from there.

Needs `python3` and a Chromium-family browser (Chrome, Chromium or Edge); the
browser runs with autoplay allowed, since a synthetic gesture cannot grant real
transient activation.
