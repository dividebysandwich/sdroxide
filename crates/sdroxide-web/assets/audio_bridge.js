// Audio bridge for the sdroxide web client.
//
// Downlink: wasm pushes mono 48 kHz PCM (Float32Array) -> playback worklet.
// Uplink: capture worklet posts mic blocks here; wasm polls pullMic().
//
// The AudioContext can only start after a user gesture, so everything is
// initialized lazily on the first one.
//
// **The playback context is treated as a resource that can die.** A window left
// in the taskbar for a long spell is a page the browser stops drawing, freezes,
// or drops the audio device under — and the context goes with it. It can come
// back reporting "closed", or it can go on reporting "running" on a sink the
// browser has let go, and in both cases `resume()` settles nothing: a page that
// only calls it is silent for the rest of the session, with the waterfall
// still moving and nothing on screen to say why. So the audio thread itself is
// watched (the player worklet reports how much it has rendered), the context is
// rebuilt when it stops, and when the browser will not let a rebuild start
// without a gesture the page says so and offers the one.

(function () {
    let ctx = null;
    let player = null;
    let micChunks = [];
    let micWanted = false;
    let micStarting = false;
    let micReady = false;
    let initStarted = false;
    // The microphone is opened once and re-attached to whatever context is in
    // force: a rebuild must not ask for it again (a second permission prompt,
    // or a refused one on Safari).
    let micStream = null;

    // --- watching the audio thread --------------------------------------
    //
    // `generation` marks which context a callback belongs to. Rebuilding makes
    // a new one while the old one is still closing, and its `onstatechange`
    // fires on the way out — acted on, that would rebuild forever.
    let generation = 0;
    // When the player last rendered. Both the events that mean "we were away"
    // and the worklet's own heartbeat write it.
    let lastRenderAt = 0;
    let lastRebuildAt = 0;
    // A page that is not being drawn is not expected to render anything, so a
    // gap while hidden proves nothing; the question is whether it renders from
    // the moment it is back. Two seconds of silence while visible does not.
    const STALL_MS = 2000;
    // A context that will not run without a gesture would otherwise be rebuilt
    // every second, and each rebuild is a fresh audio device.
    const REBUILD_MIN_MS = 5000;
    // A rebuild that does not restore rendering is a device the browser is not
    // giving back — rebuilding again would churn through contexts and never
    // make a sound, so after a couple of tries the operator is told instead.
    let stalledRebuilds = 0;
    // Why the restart bar is up, so its wording matches the cause rather
    // than always blaming a window that was in the background.
    let restartReason = "stopped";

    // --- following the output device ------------------------------------
    //
    // Connecting a headset after the page has opened does not move a context
    // that is already running. Chromium leaves it on the device it was built
    // with, which the OS has by then deactivated, so the worklet goes on
    // rendering into nothing and the page is silent with no symptom at all —
    // the same shape as a frozen context, and undetectable by the watchdog for
    // the same reason: the render heartbeat is still advancing.
    //
    // The one way to follow the new default is a fresh context, so a device
    // change is treated as a reason to rebuild. Where the browser exposes
    // `setSinkId`, the fresh context also asks for the default sink explicitly
    // rather than trusting whatever device it happened to open with.
    let sinkSupported = false;
    let sinkId = "";
    // A Bluetooth headset announces itself more than once, once per profile as
    // they arrive, so the rebuild waits for the flurry to settle. A change
    // while the page is hidden is owed to the moment the operator is back.
    let rerouteDue = false;
    let rerouteTimer = null;
    const REROUTE_SETTLE_MS = 400;

    // The browser's own name for the sink a context plays through: an empty
    // string means the default output, a device id names a specific one, and
    // an `{ type }` object is where an output has been refused.
    function readSink(c) {
        if (!c || !("sinkId" in c)) return "(unsupported)";
        const s = c.sinkId;
        if (typeof s === "string") return s || "(default)";
        if (s && s.type) return s.type;
        return "(default)";
    }

    // AudioWorklet and getUserMedia are both secure-context-only, and the whole
    // audio path runs through them. Over plain http a browser only treats
    // localhost as secure, so a page opened at http://<lan-ip>:4950 gets no
    // receive audio and no microphone at all. That is a browser rule, not
    // something the server can opt out of, so say so instead of failing mute.
    const INSECURE = !window.isSecureContext;

    function warnInsecure() {
        // The solar view is a silent viewer; an audio warning there is noise.
        if (location.search.includes("view=solar")) return;
        console.warn(
            "sdroxide audio: this page is not a secure context, so the browser " +
            "withholds AudioWorklet and the microphone. Receive audio and " +
            "microphone transmit are unavailable. Reach the server over HTTPS " +
            "(a reverse proxy or a VPN), or via localhost through an SSH tunnel."
        );
        const show = () => {
            const bar = document.createElement("div");
            bar.style.cssText = BAR_CSS;
            bar.textContent =
                "No audio: " + location.protocol + "//" + location.host +
                " is not a secure origin, so this browser withholds audio playback " +
                "and microphone access. Use HTTPS or an SSH tunnel to localhost. " +
                "(Click to dismiss.)";
            bar.addEventListener("click", () => bar.remove(), { once: true });
            document.body.appendChild(bar);
        };
        // This script runs from <head>, so <body> may not exist yet.
        if (document.body) show();
        else window.addEventListener("DOMContentLoaded", show, { once: true });
    }

    // A bar across the top of the page, for the two things the operator has to
    // be told about the sound.
    const BAR_CSS =
        "position:fixed;left:0;right:0;top:0;z-index:9999;padding:8px 12px;" +
        "font:13px system-ui,sans-serif;background:#4a3000;color:#ffd479;" +
        "border-bottom:1px solid #7a5000;cursor:pointer;" +
        // The page disables selection; this text is worth copying.
        "-webkit-user-select:text;user-select:text";

    // Shown when the audio path has stopped and the browser will not let the
    // page start it again on its own. The one gesture it is waiting for is a
    // click anywhere, and this is the only thing on screen that says so.
    let restartBar = null;

    function restartBarText() {
        if (restartReason === "device") {
            return "Audio output changed (new device connected). Click here to start the sound on it.";
        }
        if (restartReason === "background") {
            return "Sound stopped while this window was in the background. Click here to start it again.";
        }
        return "Sound stopped. Click here to start it again.";
    }

    function wantRestartBar(reason) {
        if (reason) restartReason = reason;
        // The Sound welcome hint is the other thing that occupies the top of a
        // page whose sound is not running; it says what to do, so this would
        // only be a second voice saying it.
        if (hint || location.search.includes("view=solar")) return;
        if (restartBar) {
            restartBar.textContent = restartBarText();
            return;
        }
        const put = () => {
            if (restartBar) {
                restartBar.textContent = restartBarText();
                return;
            }
            const bar = document.createElement("div");
            bar.style.cssText = BAR_CSS;
            bar.textContent = restartBarText();
            bar.addEventListener("click", () => {
                hideRestartBar();
                restartAudio("the sound banner was clicked");
            });
            document.body.appendChild(bar);
            restartBar = bar;
        };
        if (document.body) put();
        else window.addEventListener("DOMContentLoaded", put, { once: true });
    }

    function hideRestartBar() {
        if (restartBar) {
            restartBar.remove();
            restartBar = null;
        }
        restartReason = "stopped";
    }

    // Until the first gesture there is no context at all, so the page is
    // silent and nothing on screen said why: operators found the cure by
    // accident, on the volume slider. Name it instead, in the middle of the
    // screen in large type: a beginner reading the page as broken is the
    // failure it exists to prevent. It is shown only when the page loads (a
    // new session, or F5), never again later in the session.
    //
    // While the hint is up, the first click, tap or key starts the sound and
    // does nothing else, wherever it lands, and moving the mouse does nothing
    // at all (a move is not a gesture to the browser, so it cannot start the
    // sound). The hint is a transparent layer over the whole page with the
    // message in its middle; keys are held back by `keyGuard`. An operator coming back to a running
    // receiver must not lose the frequency to the tap that brings the sound
    // back, and the waterfall, where a tap tunes, is most of the screen. The
    // window's capture listener sees the gesture first (that is what starts
    // the context); the layer then keeps the events from the page.
    let hint = null;
    // Settings → UI → Sound welcome. The program owns the setting (it is kept
    // with the profile on the server) but loads after this page has drawn,
    // so the last answer is remembered on this device to decide the opening.
    const WELCOME_KEY = "sdroxide-sound-welcome";
    function welcomeWanted() {
        try {
            return localStorage.getItem(WELCOME_KEY) !== "0";
        } catch (e) {
            return true;
        }
    }
    // A press that began on the hint: its release and click must not reach
    // the page either, so the hint outlives the gesture that removed it.
    let hintBusy = false;
    let hintHidePending = false;

    function swallow(ev) {
        ev.stopPropagation();
        if (ev.cancelable) ev.preventDefault();
    }

    function hintPress(ev) {
        hintBusy = true;
        swallow(ev);
    }

    function hintRelease(ev) {
        swallow(ev);
        // The click (and a touch's compatibility mouse events) follow the
        // release; let them land on the hint before it goes.
        setTimeout(() => {
            hintBusy = false;
            if (hintHidePending) hideHint();
        }, 400);
    }

    function showHint() {
        hintHidePending = false;
        if (hint || location.search.includes("view=solar")) return;
        const put = () => {
            if (hint) return;
            hint = document.createElement("div");
            hint.style.cssText =
                "position:fixed;inset:0;z-index:9998;background:transparent;" +
                "cursor:pointer;touch-action:none";
            const box = document.createElement("div");
            box.style.cssText =
                "position:absolute;left:50%;top:50%;transform:translate(-50%,-50%);" +
                "padding:24px 32px;border-radius:12px;" +
                "font:bold clamp(22px,5vw,44px)/1.3 system-ui,sans-serif;" +
                "background:rgba(74,48,0,0.92);color:#ffd479;" +
                "border:2px solid #7a5000;box-sizing:border-box;" +
                "width:max-content;max-width:min(90vw,22em);" +
                "text-align:center;box-shadow:0 4px 24px rgba(0,0,0,0.6)";
            box.textContent = "\u{1F507} Tap, click or press a key anywhere to start the sound";
            hint.appendChild(box);
            const opts = { passive: false };
            for (const ev of ["pointerdown", "touchstart", "mousedown"]) {
                hint.addEventListener(ev, hintPress, opts);
            }
            for (const ev of ["pointerup", "touchend", "mouseup", "pointercancel", "touchcancel"]) {
                hint.addEventListener(ev, hintRelease, opts);
            }
            for (const ev of ["click", "dblclick", "contextmenu", "wheel",
                              "pointermove", "mousemove", "touchmove",
                              "pointerover", "pointerout", "mouseover", "mouseout"]) {
                hint.addEventListener(ev, swallow, opts);
            }
            document.body.appendChild(hint);
        };
        if (document.body) put();
        else window.addEventListener("DOMContentLoaded", put, { once: true });
    }

    function hideHint() {
        if (hintBusy) {
            hintHidePending = true;
            return;
        }
        hintHidePending = false;
        if (hint) {
            hint.remove();
            hint = null;
        }
    }

    // The player worklet says how much it has rendered; that is the page's only
    // evidence that the audio thread is still running.
    function noteRender() {
        lastRenderAt = performance.now();
        stalledRebuilds = 0;
        hideRestartBar();
    }

    /// Build the playback path from scratch: a new context, a new player, and
    /// the microphone re-attached if it is wanted.
    ///
    /// Called on the first gesture and again whenever the old context has died.
    /// The old one is silenced and closed first, and every callback it still
    /// owes is disowned by the generation check — a closing context fires
    /// `onstatechange`, and acted on that would rebuild in a loop.
    async function buildAudio() {
        const gen = ++generation;
        const old = ctx;
        ctx = null;
        player = null;
        micReady = false;
        if (old) {
            old.onstatechange = null;
            old.close().catch(() => {});
        }
        // The clock starts now: whatever the browser did while the page was
        // hidden is behind us, and the question is what happens next.
        lastRenderAt = performance.now();

        let next;
        try {
            next = new AudioContext({ sampleRate: 48000 });
        } catch (e) {
            // A sound device that will not take 48 kHz still beats no sound.
            next = new AudioContext();
        }
        ctx = next;
        sinkSupported = typeof next.setSinkId === "function";
        if (sinkSupported) {
            // A fresh context already opens the current default, but asking for
            // the empty sink is the difference between "the default when this
            // context was built" and "the default now" on a browser that keeps
            // the device it first chose. Refused is not fatal: the context
            // still plays through the default it opened with, and the rebuild
            // on the next device change is the other half of the cure.
            try {
                next.setSinkId("").then(
                    () => {
                        if (gen !== generation) return;
                        sinkId = readSink(next);
                        console.log("sdroxide audio: output device", sinkId);
                    },
                    (e) => console.warn("sdroxide audio: setSinkId(default) refused:", e)
                );
            } catch (e) {
                console.warn("sdroxide audio: setSinkId is not usable:", e);
            }
        }
        next.onstatechange = () => {
            if (gen !== generation) return;
            console.log("sdroxide audio: context state", next.state);
            if (next.state === "running") {
                hideHint();
                hideRestartBar();
            } else if (next.state === "closed") {
                // Closed, and a hidden page can neither use nor keep a context:
                // rebuilding now would be work the browser undoes, and the
                // moment to do it is the moment the operator is back.
                if (!document.hidden) {
                    restartReason = document.hidden ? "background" : "stopped";
                    rebuildAudio("the browser closed the audio context");
                }
            } else {
                resumeCtx();
            }
        };
        if (next.state === "running") hideHint();

        try {
            // Start the module fetch but do NOT await it yet: resume() has to
            // be called while the gesture's transient activation is still
            // alive, and on WebKit that is gone by the time an await resolves.
            const moduleReady = next.audioWorklet.addModule("pcm_worklet.js");
            resumeCtx();
            await moduleReady;
            if (gen !== generation) return;
            const node = new AudioWorkletNode(next, "pcm-player", {
                outputChannelCount: [1],
            });
            node.port.onmessage = () => {
                if (gen !== generation) return;
                noteRender();
            };
            node.connect(next.destination);
            player = node;
            // Second chance, now that there is something to hear.
            resumeCtx();
            console.log("sdroxide audio: playback ready at", next.sampleRate, "Hz");
            if (micWanted) startMic();
        } catch (e) {
            console.warn("sdroxide audio: playback bridge failed:", e);
            if (gen !== generation) return;
            // The same bound as a stall: a bridge that will not build is not
            // going to build on the fourth attempt either, and each one is a
            // fresh audio device.
            if (stalledRebuilds >= 2) return;
            stalledRebuilds += 1;
            rebuildAudio("the playback bridge would not start");
        }
    }

    /// Throw the dead context away and build a new one, at most every
    /// [`REBUILD_MIN_MS`] — a rebuild that cannot succeed (no gesture yet)
    /// would otherwise be attempted once a second. `force` skips that limit for
    /// a rebuild the browser has made necessary (a device change), which is not
    /// an attempt that can fail.
    function rebuildAudio(reason, force) {
        const now = performance.now();
        if (!force && now - lastRebuildAt < REBUILD_MIN_MS) return;
        lastRebuildAt = now;
        console.log("sdroxide audio: rebuilding the playback path:", reason);
        buildAudio();
    }

    /// The output device changed under a running page, so the context in force
    /// is on a device the OS has let go. A new context is the only way to
    /// follow the new default — hence the rebuild — and being a real new fact
    /// rather than a stalled context, it is not swallowed by the rebuild rate
    /// limit, which exists for a rebuild that cannot succeed.
    function rerouteAudio(reason) {
        rerouteDue = false;
        restartReason = "device";
        // No playback yet: the first context has not been built, and it will
        // open the current default when the gesture comes.
        if (!ctx) return;
        // Forced, not rate-limited: zeroing `lastRebuildAt` does not carry it,
        // because the limit is measured against page load — a change in the
        // page's first few seconds would be swallowed.
        rebuildAudio(reason, true);
    }

    /// A device change is a flurry, not an event: a headset's profiles arrive
    /// one at a time. Wait for it to settle, then rebuild once.
    function scheduleReroute() {
        if (rerouteTimer) clearTimeout(rerouteTimer);
        rerouteTimer = setTimeout(() => {
            rerouteTimer = null;
            if (!ctx || document.hidden) return;
            rerouteAudio("the output device changed");
        }, REROUTE_SETTLE_MS);
    }

    function resumeCtx() {
        if (!ctx || ctx.state === "running") return;
        // Outside a user gesture this rejects; the next gesture retries.
        ctx.resume().catch(() => {});
    }

    /// Whatever has happened to the audio path, this is the page asking it to
    /// work: rebuild a context the browser has closed, resume one it paused.
    ///
    /// Never creates the first context — that is the gesture's job, and a page
    /// that makes one on its own is a page whose sound the browser then holds
    /// back until some later click, which is the failure the hint exists to
    /// explain. Nor does it judge anything: `resume()` resolves asynchronously,
    /// so an event that has just asked for the sound cannot yet know whether it
    /// got it, and the watchdog is the one place that decides.
    function ensureAudio(reason) {
        if (!ctx) return;
        if (ctx.state === "closed") {
            rebuildAudio(reason + ": the playback context is closed");
            return;
        }
        resumeCtx();
    }

    /// The context is back and the operator is looking at it: give the audio
    /// thread a moment to render before judging it, then rebuild if it does
    /// not.
    function cameBack(reason) {
        lastRenderAt = performance.now();
        // A device change while the page was hidden is owed now, and a rebuild
        // is the answer to it rather than a resume: the context in force is on
        // a device that is gone.
        if (rerouteDue) {
            restartReason = "device";
            rerouteAudio(reason + ": the output device changed");
            return;
        }
        ensureAudio(reason);
    }

    /// One tick's judgement on the audio path, and the only place the operator
    /// is told the sound has stopped.
    ///
    /// Two failures are told apart here: a path that is not playing — the
    /// browser is holding the context, or the bridge never finished building —
    /// which only a gesture is likely to release, so say so; and a context that
    /// says it *is* running on a device the browser has let go, which has no
    /// symptom at all except that nothing is rendered. It is the second one that
    /// a page calling `resume()` is silent about.
    function watchAudio(now) {
        if (!ctx || document.hidden) return;
        if (ctx.state !== "running" || !player) {
            wantRestartBar();
            return;
        }
        if (now - lastRenderAt > STALL_MS) {
            if (stalledRebuilds >= 2) {
                wantRestartBar();
                return;
            }
            stalledRebuilds += 1;
            rebuildAudio("the audio thread stopped without the context saying so");
        }
    }

    function init() {
        if (ctx || initStarted) return;
        initStarted = true;
        buildAudio();
    }

    // Opening the microphone is deliberately deferred until the app actually
    // wants to transmit. On iOS getUserMedia switches the audio session into
    // play-and-record, which attenuates and reroutes playback -- so requesting
    // it up front makes receive audio quiet or silent for every listener who
    // never keys up, on top of prompting them for a mic they will not use.
    async function startMic() {
        if (micStarting || micReady || !ctx) return;
        micStarting = true;
        try {
            // The stream is kept across a rebuild: the permission has been
            // asked for and answered, and the device is the same one.
            const live = micStream && micStream.getAudioTracks().some(
                (t) => t.readyState === "live"
            );
            if (live) {
                console.log("sdroxide audio: microphone re-attached to a new context");
            } else {
                // Raw capture, explicitly. A browser defaults all three of these
                // ON, because its idea of a microphone is a conference call: the
                // echo canceller removes whatever the page is also playing, the
                // noise suppressor removes anything that does not look like
                // speech, and the gain control rides the level.
                //
                // Every one of them is wrong on a transmitter. A transmitter must
                // send what the microphone heard -- the suppressor alone will gate
                // a CW note or a digital tone to nothing, since neither looks like
                // speech to it. The echo canceller is worse still: this page plays
                // receive audio through the same AudioContext, so the band becomes
                // the far-end reference and the operator's voice becomes the thing
                // it is built to subtract, which is a microphone that reads open
                // and transmits silence (issue #493; receive audio is also held
                // off for the length of an over now, see `RemoteController`).
                //
                // Plain values rather than `exact`: these are preferences, and a
                // device that cannot turn its processing off should still open.
                micStream = await navigator.mediaDevices.getUserMedia({
                    audio: {
                        channelCount: 1,
                        echoCancellation: false,
                        noiseSuppression: false,
                        autoGainControl: false,
                    },
                });
                // What the browser actually gave us, named in the log so a report
                // of a silent transmitter does not need a second round trip.
                const track = micStream.getAudioTracks()[0];
                const got = (track && track.getSettings) ? track.getSettings() : {};
                console.log(
                    "sdroxide audio: microphone", (track && track.label) || "(unnamed)",
                    "rate", got.sampleRate || "(context)",
                    "channels", got.channelCount || 1,
                    "aec", got.echoCancellation, "ns", got.noiseSuppression,
                    "agc", got.autoGainControl
                );
            }
            const src = ctx.createMediaStreamSource(micStream);
            const capture = new AudioWorkletNode(ctx, "mic-capture");
            capture.port.onmessage = (ev) => {
                micChunks.push(ev.data);
                // Bound: ~1 s of backlog.
                while (micChunks.length > 400) micChunks.shift();
            };
            src.connect(capture);
            // A Web Audio node is only rendered if it has a path through to the
            // destination, and MicCapture is driven entirely by `process()` —
            // so with no onward connection it was never scheduled, its
            // `postMessage` never fired, and the mic fed nothing while logging
            // "mic ready". The processor writes no output, so this path carries
            // silence and cannot feed back.
            capture.connect(ctx.destination);
            micReady = true;
            console.log("sdroxide audio: mic ready");
        } catch (e) {
            // Safari wants transient activation for getUserMedia, and the TX
            // request reaches us a frame or more after the tap that caused it.
            // Leave micWanted set so the next gesture tries again.
            console.warn("sdroxide audio: no microphone (will retry):", e);
        } finally {
            micStarting = false;
        }
    }

    // The key that starts the sound must not also reach the page, where it
    // could be a tuning shortcut. Registered on the window, in the capture
    // phase, BEFORE onGesture: that one may start the context and remove the
    // hint at once, and this must still see it up. stopPropagation (not the
    // immediate kind) keeps the key from the page and still lets onGesture
    // run. The key's own release and repeat are held back with it.
    const heldKeys = new Set();

    function keyGuard(ev) {
        if (ev.type === "keydown" && hint) heldKeys.add(ev.code);
        if (!heldKeys.has(ev.code)) return;
        if (ev.type === "keyup") heldKeys.delete(ev.code);
        swallow(ev);
    }

    function onGesture(ev) {
        // Mark a press on the hint before init() runs: a context that starts
        // at once removes the hint synchronously, and the release and click
        // of this same tap would then fall through to the waterfall.
        if (hint && ev && hint.contains(ev.target)) hintBusy = true;
        init();
        // A gesture is also the one thing that can start a context the browser
        // has stopped, so it is the moment to put the audio path back.
        lastRebuildAt = 0;
        ensureAudio("a gesture");
        if (micWanted) startMic();
    }

    function restartAudio(reason) {
        lastRebuildAt = 0;
        if (!ctx) {
            init();
            return;
        }
        if (ctx.state === "running" && player) {
            resumeCtx();
            return;
        }
        rebuildAudio(reason);
    }

    if (INSECURE) {
        warnInsecure();
    } else {
        if (welcomeWanted()) showHint();
        // Capture phase, and not just "click": eframe's canvas handlers call
        // preventDefault() on touchstart -- which stops WebKit synthesizing the
        // compatibility mouse events, so a touch device never fires "click" at
        // all -- and stopPropagation() on touchstart and pointerdown, which
        // keeps bubble-phase window listeners from ever running. Capture runs
        // before the canvas target, so neither can hide the gesture from us.
        for (const ev of ["keydown", "keypress", "keyup"]) {
            window.addEventListener(ev, keyGuard, { capture: true, passive: false });
        }
        const opts = { capture: true, passive: true };
        for (const ev of ["pointerdown", "touchstart", "touchend", "click", "keydown"]) {
            window.addEventListener(ev, onGesture, opts);
        }
        // The output default itself can move while the page is open — a
        // headset connected mid-session is the everyday case — and a context
        // that is already running is not moved with it. Ask to be told and
        // rebuild; see the note beside `rerouteDue`.
        if (navigator.mediaDevices && navigator.mediaDevices.addEventListener) {
            navigator.mediaDevices.addEventListener("devicechange", () => {
                rerouteDue = true;
                restartReason = "device";
                if (document.hidden) return;
                scheduleReroute();
            });
        }
        // iOS suspends the context when the tab goes to the background; a
        // browser that has stopped drawing a minimised window freezes it and
        // tears its audio down. Either way the way back is the same, and it is
        // the only moment at which rebuilding is worth attempting.
        document.addEventListener("visibilitychange", () => {
            if (!document.hidden) {
                if (!rerouteDue) restartReason = "background";
                cameBack("the page was shown again");
            }
        });
        // The Page Lifecycle API: `freeze` stops a hidden page's tasks without
        // unloading it (Chrome does this to background tabs), and `resume` is
        // the way back from that — a page that never sees `visibilitychange`
        // because it was never hidden, only frozen.
        document.addEventListener("resume", () => cameBack("the page was resumed"));
        window.addEventListener("pageshow", () => cameBack("the page was shown"));
        window.addEventListener("focus", () => ensureAudio("the window was focused"));
        // The watchdog: a context that has stopped rendering says nothing, so
        // it has to be asked. One tick a second while the page is visible is
        // free; the tick itself is what notices the silence.
        setInterval(() => watchAudio(performance.now()), 1000);
    }

    window.sdroxideAudio = {
        pushPcm: function (pcm) {
            if (player) {
                // Copy: the wasm memory view is invalidated on return.
                player.port.postMessage(new Float32Array(pcm));
            }
        },
        pullMic: function () {
            if (micChunks.length === 0) return new Float32Array(0);
            let total = 0;
            for (const c of micChunks) total += c.length;
            const out = new Float32Array(total);
            let off = 0;
            for (const c of micChunks) {
                out.set(c, off);
                off += c.length;
            }
            micChunks = [];
            return out;
        },
        // The program's Sound welcome setting. Off: the message goes now and
        // is not shown at the next opening. On: remembered for the next
        // opening, never shown mid-session.
        setSoundWelcome: function (on) {
            try {
                localStorage.setItem(WELCOME_KEY, on ? "1" : "0");
            } catch (e) {}
            if (!on) {
                hintBusy = false;
                hideHint();
            }
        },
        setMicActive: function (active) {
            micWanted = !!active;
            if (micWanted) startMic();
        },
        // What the audio path is doing, for the operator's own report and for
        // the browser harness: there is no other way to ask a page whether the
        // sound it is supposed to be making is being made.
        health: function () {
            return {
                state: ctx ? ctx.state : "none",
                playing: !!player,
                generation: generation,
                // -1 means "nothing has ever rendered on this context".
                renderAgoMs: lastRenderAt ? Math.round(performance.now() - lastRenderAt) : -1,
                hidden: document.hidden,
                microphone: micReady,
                // Which output the page is playing through, and whether the
                // browser lets the page choose it at all: the two facts a
                // report of "no sound in the headset" needs.
                sink: ctx ? readSink(ctx) : "(none)",
                sinkSupported: sinkSupported,
                sinkId: sinkId,
            };
        },
        // Force the audio path back on, for the banner and for a caller that
        // knows what it is looking at.
        restart: function () {
            restartAudio("the audio path was restarted by hand");
        },
    };
})();
