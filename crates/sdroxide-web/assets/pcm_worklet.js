// AudioWorklet processors for the sdroxide web client.

// Plays queued Float32Array blocks with a small jitter buffer.
class PcmPlayer extends AudioWorkletProcessor {
    constructor() {
        super();
        this.queue = [];
        this.queued = 0;
        this.offset = 0;
        // Wait for ~60 ms before starting playback (jitter buffer).
        this.primed = false;
        this.port.onmessage = (ev) => {
            this.queue.push(ev.data);
            this.queued += ev.data.length;
            // Bound total buffering to ~250 ms to cap latency.
            while (this.queued > 12000 && this.queue.length > 1) {
                const dropped = this.queue.shift();
                this.queued -= dropped.length;
                this.offset = 0;
            }
        };
    }

    process(_inputs, outputs) {
        const out = outputs[0][0];
        if (!this.primed) {
            if (this.queued >= 2880) this.primed = true;
            else {
                out.fill(0);
                return true;
            }
        }
        let i = 0;
        while (i < out.length && this.queue.length > 0) {
            const head = this.queue[0];
            const take = Math.min(out.length - i, head.length - this.offset);
            out.set(head.subarray(this.offset, this.offset + take), i);
            i += take;
            this.offset += take;
            this.queued -= take;
            if (this.offset >= head.length) {
                this.queue.shift();
                this.offset = 0;
            }
        }
        if (i < out.length) {
            out.fill(0, i);
            this.primed = false; // underrun: re-prime
        }
        return true;
    }
}

// Forwards mic input blocks to the main thread.
class MicCapture extends AudioWorkletProcessor {
    process(inputs, _outputs) {
        const input = inputs[0];
        if (input.length > 0 && input[0].length > 0) {
            this.port.postMessage(new Float32Array(input[0]));
        }
        return true;
    }
}

registerProcessor("pcm-player", PcmPlayer);
registerProcessor("mic-capture", MicCapture);
