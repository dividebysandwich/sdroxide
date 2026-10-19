// Does the browser client follow the output device when it changes under the
// page?
//
// The report: the web client plays through the speakers, a Bluetooth headset is
// connected while the page is open, and the sound is then heard in neither --
// the AudioContext is left on the device it was built with, which the OS has
// deactivated, while the worklet goes on rendering into nothing.
//
// A headless browser cannot be given a headset, so what is driven here is the
// mechanism the cure rests on. A `devicechange` from `navigator.mediaDevices`
// must make the page build a fresh context -- the only way to follow the new
// default -- and that context must ask for the default sink when the browser
// exposes `setSinkId`. Every assertion is on the browser's own AudioContext,
// measured through the wrappers in `page.html`, so this fails on a build that
// does nothing rather than only on one that cannot report.

(function () {
    const MAX_MS = 30000;
    let done = false;

    function say(line) {
        fetch("/result?line=" + encodeURIComponent(line));
    }

    function finish(ok, why) {
        if (done) return;
        done = true;
        say((ok ? "PASS" : "FAIL") + " " + why);
        fetch("/result?done=1");
    }

    const wait = (ms) => new Promise((r) => setTimeout(r, ms));

    async function until(fn, ms) {
        const t0 = performance.now();
        while (performance.now() - t0 < ms) {
            if (fn()) return true;
            await wait(50);
        }
        return fn();
    }

    const live = () => window.__contexts[window.__contexts.length - 1] || null;
    const state = (c) => (c ? c.state : "none");

    // 960 samples of a keyed tone every 20 ms: exactly what the wasm client
    // pushes for one receive frame.
    let phase = 0;
    let timer = null;
    function startPushing() {
        if (timer) return;
        timer = setInterval(() => {
            const b = new Float32Array(960);
            for (let i = 0; i < 960; i++) {
                b[i] = 0.4 * Math.sin((2 * Math.PI * 700 * (phase * 960 + i)) / 48000);
            }
            phase++;
            window.sdroxideAudio.pushPcm(b);
        }, 20);
    }

    // The tab going to the background. `document.hidden` is a read-only getter
    // in a real page, so the harness has to stand it up itself.
    let hidden = false;
    Object.defineProperty(document, "hidden", {
        configurable: true,
        get: () => hidden,
    });
    Object.defineProperty(document, "visibilityState", {
        configurable: true,
        get: () => (hidden ? "hidden" : "visible"),
    });
    function setHidden(v) {
        hidden = v;
        document.dispatchEvent(new Event("visibilitychange"));
    }

    // The output default moving under the page, as the browser reports it.
    function deviceChanged() {
        if (!(navigator.mediaDevices && navigator.mediaDevices.dispatchEvent)) {
            return false;
        }
        navigator.mediaDevices.dispatchEvent(new Event("devicechange"));
        return true;
    }

    function snapshot() {
        return {
            contexts: window.__contexts.length,
            states: window.__contexts.map(state),
            sinks: window.__sinkCalls.map((c) => c.id),
            health: window.sdroxideAudio.health ? window.sdroxideAudio.health() : null,
        };
    }

    async function main() {
        // A gesture is what starts the bridge in a real page. The browser is
        // launched with autoplay allowed so the same path runs here without a
        // pointer.
        setHidden(false);
        window.dispatchEvent(new KeyboardEvent("keydown", { code: "KeyA", bubbles: true }));
        window.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));

        if (!(await until(() => live() && state(live()) === "running", 6000))) {
            return finish(false, "the playback context never started (" + state(live()) + ")");
        }
        const first = live();
        startPushing();
        await wait(1400);

        const t0 = first.currentTime;
        await wait(800);
        if (!(first.currentTime > t0)) {
            return finish(false, "the playback context is running but not rendering");
        }
        if (window.__contexts.length !== 1) {
            return finish(false, "expected one context to start, saw " + window.__contexts.length);
        }
        say("ok playing: " + JSON.stringify(snapshot()));

        // --- a headset is connected while the page is open ------------------
        const before = window.__contexts.length;
        // A sentinel listener, registered the same way the bridge registers
        // its own, says whether a synthetic `devicechange` is delivered at all
        // before anything is blamed on the bridge.
        let sentinel = false;
        let canListen = false;
        try {
            canListen = !!(navigator.mediaDevices && navigator.mediaDevices.addEventListener);
            if (canListen) {
                navigator.mediaDevices.addEventListener("devicechange", () => { sentinel = true; });
            }
        } catch (e) {}
        if (!deviceChanged()) {
            return finish(false, "this browser has no navigator.mediaDevices, so a device change cannot be reported");
        }
        await wait(100);
        say("note sentinel canListen=" + canListen + " seen=" + sentinel +
            " mdPresents=" + window.__mdPresents + " mdAdds=" + JSON.stringify(window.__mdAdds));
        const rebuilt = await until(() => {
            const c = live();
            return window.__contexts.length > before && c && c !== first && state(c) === "running";
        }, 6000);
        if (!rebuilt) {
            return finish(
                false,
                "the output device changed and the bridge did not rebuild: " + JSON.stringify(snapshot())
            );
        }
        const fresh = live();
        const c0 = fresh.currentTime;
        await wait(1000);
        if (!(fresh.currentTime > c0)) {
            return finish(false, "a new context was built for the device change but it is not rendering");
        }
        if (window.__sinkSupported && !window.__sinkCalls.some((c) => c.id === "")) {
            return finish(
                false,
                "setSinkId exists but the default sink was never asked for: " + JSON.stringify(snapshot())
            );
        }
        const h = window.sdroxideAudio.health ? window.sdroxideAudio.health() : null;
        if (!h || !("sink" in h)) {
            return finish(false, "health() does not report the output device: " + JSON.stringify(h));
        }
        say("ok rerouted: " + JSON.stringify(snapshot()));

        // --- a change while the page is hidden is owed to the way back ------
        setHidden(true);
        const beforeHidden = window.__contexts.length;
        deviceChanged();
        await wait(1600); // well past the settle time, and a watchdog tick
        if (window.__contexts.length > beforeHidden) {
            return finish(false, "the page rebuilt while it was hidden: " + JSON.stringify(snapshot()));
        }
        say("ok deferred while hidden: " + JSON.stringify(snapshot()));

        setHidden(false);
        const deferred = await until(
            () => window.__contexts.length > beforeHidden && state(live()) === "running",
            6000
        );
        if (!deferred) {
            return finish(
                false,
                "the page came back with a device change owing and did not rebuild: " + JSON.stringify(snapshot())
            );
        }
        say("ok deferred reroute applied: " + JSON.stringify(snapshot()));
        finish(true, "a device change is followed by a fresh context on the current default output");
    }

    setTimeout(() => finish(false, "the harness ran out of time"), MAX_MS);
    main().catch((e) => finish(false, "harness error: " + e));
})();
