#!/usr/bin/env python3
"""Round-trip a STANAG 4285 frame through the spec, with no recording involved.

Usage: s4285_sim.py [--centre 1800] [--rate 12000] [--ppm 0] [--snr 30]

This builds a frame *from the transcription in MODE-DECODERS-HANDOVER.md §6b*
and then decodes it with the same steps the probe uses. It exists because the
two off-air fixtures disagree with each other and neither of them locks, and a
fixture that will not lock cannot tell you whether the transcription is wrong or
the recording is. A round trip can: if this decodes with no bit errors, the
frame, the scrambler and the sync generator are as the standard describes, and
anything that then fails on a recording is the recording.

What the modulator does, in the spec's own terms:

  * 2400 baud, eight phases at n*pi/4, on a 1800 Hz sub-carrier;
  * 256 symbols per frame: 80 sync, 48 reference, 128 data;
  * reference and data symbols multiplied by the 176-symbol scrambler;
  * the sync is BPSK on the same 2400 baud clock, from x^5+x^2+1 seeded 11010.

The decoder has to find the frame from the reference symbols alone, recover the
carrier, choose the sampling instant, descramble, and read the data back.
"""

import sys
import numpy as np

BAUD = 2400.0
SPS = 8
ROLLOFF = 0.35
SPAN = 8          # symbols each side of the root-raised-cosine
SYNC = 80
FRAME = 256
REF_AT = [(32, 48), (80, 96), (128, 144)]
DATA_AT = [(0, 32), (48, 80), (96, 128), (144, 176)]


def lfsr(taps, n, init):
    state = [(init >> i) & 1 for i in range(max(taps) + 1)]
    out = []
    for _ in range(n):
        out.append(state[0])
        fb = 0
        for t in taps:
            fb ^= state[t]
        state = state[1:] + [fb]
    return np.array(out)


def sync_bits():
    return lfsr([2, 5], SYNC, 0b11010)


def scrambler_phase():
    bits = lfsr([4, 9], 511 * 3, 0b1).reshape(-1, 3)
    n = bits[:, 0] + 2 * bits[:, 1] + 4 * bits[:, 2]
    return np.exp(1j * np.pi / 4 * n[:176])


def ref_positions():
    return np.array([k for lo, hi in REF_AT for k in range(lo, hi)])


def data_positions():
    return np.array([k for lo, hi in DATA_AT for k in range(lo, hi)])


def rrc():
    """Root-raised-cosine, the shape the standard's spectrum implies."""
    n = np.arange(-SPAN * SPS, SPAN * SPS + 1)
    t = n / (BAUD * SPS)
    b = ROLLOFF
    with np.errstate(divide="ignore", invalid="ignore"):
        h = np.sinc(t * BAUD) * np.cos(np.pi * b * t * BAUD) / (1 - (2 * b * t * BAUD) ** 2)
    h[~np.isfinite(h)] = 0.0
    h[np.abs(1 - (2 * b * t * BAUD) ** 2) < 1e-9] = (np.pi / 4) * np.sinc(1 / (2 * b))
    return h / np.sqrt(np.sum(h ** 2))


def modulate(data_syms, frames=4, centre=1800.0, rate=12000.0, ppm=0.0):
    """Audio for `frames` frames carrying `data_syms` (176 per frame, 128 used)."""
    bits = sync_bits()
    scr = scrambler_phase()
    out = []
    for f in range(frames):
        sync = np.where(bits > 0, 1.0, np.exp(1j * np.pi))  # symbol 0 or 4
        body = np.ones(176, dtype=complex)                  # reference: symbol 0
        d = data_syms[f]
        for lo, hi in DATA_AT:
            body[lo:hi] = np.exp(1j * np.pi / 4 * d[lo:hi])
        body *= scr
        out.append(np.concatenate([sync, body]))
    syms = np.concatenate(out)

    # Upsample to SPS per symbol and shape, so there is no rectangular ISI for
    # the receiver to fight.
    up = np.zeros(len(syms) * SPS, dtype=complex)
    up[::SPS] = syms
    base = np.convolve(up, rrc(), "same")
    # then to the requested audio rate
    nsrc = np.arange(len(base))
    n_out = int(len(base) * rate / (BAUD * SPS))
    dst = np.arange(n_out) * (BAUD * SPS) / rate
    base = np.interp(dst, nsrc, base.real) + 1j * np.interp(dst, nsrc, base.imag)
    n = np.arange(len(base))
    audio = (base * np.exp(1j * 2 * np.pi * centre * n / rate)).real
    # played back on a clock that is `ppm` off ours
    return audio, rate * (1 + ppm * 1e-6)


def demod(audio, rate_in, centre=1800.0):
    """Mix down, filter, resample to SPS per symbol."""
    src = np.arange(len(audio))
    bb = audio * np.exp(-2j * np.pi * centre * src / rate_in)
    # Reject what the mixer put outside the channel. Without this the mirror at
    # -2*centre sits inside the resampled band and corrupts the decision.
    spec = np.fft.fft(bb)
    f = np.fft.fftfreq(len(bb), 1 / rate_in)
    spec[np.abs(f) > 1500.0] = 0.0
    bb = np.fft.ifft(spec)
    n_out = int(len(audio) * BAUD * SPS / rate_in)
    dst = np.arange(n_out) * rate_in / (BAUD * SPS)
    bb = np.interp(dst, src, bb.real) + 1j * np.interp(dst, src, bb.imag)
    return np.convolve(bb, rrc(), "same")   # matched filter


def find_frame(sym):
    """Locate the frame by the reference symbols: they are all symbol 0."""
    scr = scrambler_phase()
    term = np.ones(FRAME, dtype=complex)
    term[SYNC:] = np.conj(scr)
    rp = ref_positions()
    t = np.arange(FRAME) / BAUD
    tref = t[SYNC + rp]
    w_grid = np.arange(-75.0, 75.1, 1.0)
    best = (0.0, None)
    for sub in range(SPS):
        s = sym[sub::SPS]
        for start in range(0, len(s) - FRAME):
            z = s[start:start + FRAME] * term
            ref = z[SYNC + rp]
            m = np.abs(ref)
            if m.mean() < 1e-9:
                continue
            ph = ref / (m + 1e-12)
            g = np.abs(ph @ np.exp(-1j * np.outer(w_grid, tref)).T) / len(ref)
            i = int(np.argmax(g))
            if g[i] > best[0]:
                best = (float(g[i]), (start, sub, float(w_grid[i])))
    return best


def decode_frames(sym8, start_sample, w, tau, frames):
    """Sample a fractional instant, descramble, and slice each frame.

    `sym8` is the baseband at SPS samples per symbol. The sampling instant is
    `start + k*SPS + tau*SPS`, interpolated: a receiver that can only sample on
    whole samples is limited to one of eight phases per symbol, which is why the
    integer version fell apart at a few hundred ppm of clock error.
    """
    scr = scrambler_phase()
    term = np.ones(FRAME, dtype=complex)
    term[SYNC:] = np.conj(scr)
    rp, dp = ref_positions(), data_positions()
    t = np.arange(FRAME) / BAUD
    out = []
    for f in range(frames):
        base = start_sample + f * FRAME * SPS + tau * SPS
        pos = base + np.arange(FRAME) * SPS
        if pos[-1] >= len(sym8) or pos[0] < 0:
            break
        z = (np.interp(pos, np.arange(len(sym8)), sym8.real)
             + 1j * np.interp(pos, np.arange(len(sym8)), sym8.imag))
        z = z * term * np.exp(-2j * np.pi * w * t)
        rot = np.exp(-1j * np.angle(np.sum(z[SYNC + rp])))
        out.append(z[SYNC + dp] * rot)
    return out


def decision_error(frames_data):
    """Mean angular distance from each symbol to the nearest of the eight."""
    tot = 0.0
    for d in frames_data:
        q = np.round(np.angle(d) / (np.pi / 4)) * (np.pi / 4)
        tot += float(np.mean(np.abs(np.angle(d * np.exp(-1j * q)))))
    return tot / max(len(frames_data), 1)


def main():
    centre, rate, ppm, snr = 1800.0, 12000.0, 0.0, 40.0
    write = None
    args = sys.argv[1:]
    for i, a in enumerate(args):
        if a == "--centre":
            centre = float(args[i + 1])
        elif a == "--rate":
            rate = float(args[i + 1])
        elif a == "--ppm":
            ppm = float(args[i + 1])
        elif a == "--snr":
            snr = float(args[i + 1])
        elif a == "--write":
            write = args[i + 1]

    rng = np.random.default_rng(7)
    frames = 4
    sent = [rng.integers(0, 8, 176) for _ in range(frames)]
    audio, rate_in = modulate(sent, frames, centre, rate, ppm)
    audio = audio / np.abs(audio).max()
    if snr < 60:
        audio = audio + rng.normal(0, 10 ** (-snr / 20), len(audio))
    print(f"modulated {frames} frames: {len(audio)/rate_in:.3f}s at {rate_in:.0f} Hz, "
          f"SNRa {snr:.0f} dB, clock {ppm:+.0f} ppm")

    if write:
        import wave
        pcm = (np.clip(audio, -1, 1) * 32767).astype("<i2")
        w = wave.open(write, "wb")
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(int(round(rate_in)))
        w.writeframes(pcm.tobytes())
        w.close()
        print(f"wrote {write}")

    sym = demod(audio, rate_in, centre)
    r, where = find_frame(sym)
    if where is None:
        print("no frame found")
        return 1
    start, sub, w = where
    print(f"frame found with R = {r:.3f} at start {start}, sub-phase {sub}, offset {w:+.0f} Hz "
          f"(noise ~0.14)")

    # Timing, then. Sixteen reference symbols in a row all carry the same phase,
    # so they say nothing about *where* inside a symbol to sample — the search
    # above scores best when it lands on a boundary. The data does say, and the
    # instant is interpolated rather than chosen from the eight whole samples.
    start_sample = start * SPS + sub
    taus = np.arange(-0.5, 0.5001, 0.05)
    errs = [decision_error(decode_frames(sym, start_sample, w, tau, frames)) for tau in taus]
    tau = float(taus[int(np.argmin(errs))])
    print(f"timing: tau {tau:+.2f} samples, decision error {min(errs):.3f} rad")

    got = decode_frames(sym, start_sample, w, tau, frames)
    dp = data_positions()
    ok = 0
    for f, d in enumerate(got):
        g = np.round(np.angle(d) / (np.pi / 4)).astype(int) % 8
        hits = [i2 for i2, s_ in enumerate(sent) if np.array_equal(g, s_[dp])]
        if hits:
            ok += 1
        else:
            near = min(sent, key=lambda s_: np.sum(
                np.round(np.angle(np.exp(1j * np.pi / 4 * s_[dp]))) != g))
            print(f"  frame {f}: {np.sum(g != near[dp])} of {len(dp)} symbols wrong")
    print(f"frames decoded exactly: {ok} of {len(got)}")
    return 0 if ok == len(got) else 1


if __name__ == "__main__":
    sys.exit(main())
