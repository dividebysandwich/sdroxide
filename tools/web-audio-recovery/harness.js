// Does the browser client get its sound back after a long spell in the
// background?
//
// The page drives the real bridge the way the wasm client does — a 960-sample
// block every 20 ms, which is one frame of receive audio from
// `RemoteController::poll_event` — and then does what thirty minutes in the
// taskbar does to it: the page goes hidden, the playback context dies while
// nobody is looking, and the page comes back. The bridge must be playing again
// afterwards.
//
// Every assertion is on the browser's own AudioContext, measured through the
// wrapper in `page.html`, so this fails on a build that has no way to recover
// rather than only on a build that cannot report.

(function () {
    const MAX_MS = 22000;
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
    // in a real page, so the harness has to stand it up itself: everything the
    // bridge does with visibility runs off this.
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

    function snapshot() {
        const h = window.sdroxideAudio.health ? window.sdroxideAudio.health() : null;
        return {
            contexts: window.__contexts.length,
            states: window.__contexts.map(state),
            health: h,
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
        await wait(1500);

        const t0 = first.currentTime;
        await wait(1000);
        if (!(first.currentTime > t0)) {
            return finish(false, "the playback context is running but not rendering");
        }
        say("ok playing: " + JSON.stringify(snapshot()));

        // --- the report: hidden for a long spell, and the context dies ------
        setHidden(true);
        const killed = first;
        try {
            killed.close();
        } catch (e) {}
        await wait(2500); // the bridge's watchdog ticks once a second
        if (window.__contexts.length > 1) {
            say("note a rebuild was attempted while the page was hidden");
        }
        say("ok hidden with a dead context: " + JSON.stringify(snapshot()));

        // --- the window comes back -----------------------------------------
        setHidden(false);
        const back = await until(() => {
            const c = live();
            return c && c !== killed && state(c) === "running";
        }, 8000);
        if (!back) {
            return finish(
                false,
                "the window came back and the sound did not: " +
                    JSON.stringify(snapshot())
            );
        }
        const fresh = live();
        startPushing();
        const c0 = fresh.currentTime;
        await wait(1200);
        if (!(fresh.currentTime > c0)) {
            return finish(false, "a new context was made but it is not rendering");
        }
        if (window.sdroxideAudio.health) {
            const h = window.sdroxideAudio.health();
            if (h.state !== "running" || (h.renderAgoMs >= 0 && h.renderAgoMs > 1000)) {
                return finish(false, "audio is rendering but health() says " + JSON.stringify(h));
            }
        }
        say("ok recovered: " + JSON.stringify(snapshot()));
        finish(true, "sound resumed on a new context after a spell in the background");
    }

    setTimeout(() => finish(false, "the harness ran out of time"), MAX_MS);
    main().catch((e) => finish(false, "harness error: " + e));
})();
