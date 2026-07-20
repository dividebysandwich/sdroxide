// Audio bridge for the sdroxide web client.
//
// Downlink: wasm pushes mono 48 kHz PCM (Float32Array) -> playback worklet.
// Uplink: capture worklet posts mic blocks here; wasm polls pullMic().
//
// The AudioContext can only start after a user gesture, so everything is
// initialized lazily on the first click/keydown.

(function () {
    let ctx = null;
    let player = null;
    let micChunks = [];
    let micStarted = false;
    let initStarted = false;

    async function init() {
        if (initStarted) return;
        initStarted = true;
        try {
            ctx = new AudioContext({ sampleRate: 48000 });
            await ctx.audioWorklet.addModule("pcm_worklet.js");
            player = new AudioWorkletNode(ctx, "pcm-player", {
                outputChannelCount: [1],
            });
            player.connect(ctx.destination);
            if (ctx.state === "suspended") {
                await ctx.resume();
            }
            console.log("sdroxide audio: playback ready at", ctx.sampleRate, "Hz");
        } catch (e) {
            console.warn("sdroxide audio init failed:", e);
        }
        startMic();
    }

    async function startMic() {
        if (micStarted || !ctx) return;
        micStarted = true;
        try {
            const stream = await navigator.mediaDevices.getUserMedia({
                audio: { sampleRate: 48000, channelCount: 1 },
            });
            const src = ctx.createMediaStreamSource(stream);
            const capture = new AudioWorkletNode(ctx, "mic-capture");
            capture.port.onmessage = (ev) => {
                micChunks.push(ev.data);
                // Bound: ~1 s of backlog.
                while (micChunks.length > 400) micChunks.shift();
            };
            src.connect(capture);
            console.log("sdroxide audio: mic ready");
        } catch (e) {
            console.warn("sdroxide audio: no microphone:", e);
        }
    }

    window.addEventListener("click", init, { once: false });
    window.addEventListener("keydown", init, { once: false });

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
    };
})();
