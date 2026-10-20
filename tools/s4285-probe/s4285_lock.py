#!/usr/bin/env python3
"""Full acquisition chain for a STANAG 4285 recording.

Usage: s4285_lock.py <file.wav> [centre_hz] [frames]

Four steps, in the order they have to happen, and each one was discovered the
hard way:

  1. **Fold** the recording onto its own 106.67 ms frame period. Scores how much
     of a frame repeats; a real signal reaches 0.6 and up, and noise reaches
     about 0.44 at eight frames (measured, not assumed).
  2. **Refine the offset** finely. The fold's own grid is coarse in Hz, and at a
     few frames' span a coarse grid smears the addition.
  3. **Search the ramp.** The fold absorbs a *carrier offset* between frames but
     not a *centre error* within one, and a ramp spreads everything in the frame
     — including the reference symbols, which is why both shipped fixtures
     looked like they lacked the standard's preamble before this step existed.
  4. **Descramble** and score the 48 reference symbols, which are all symbol 0
     before scrambling: a correct frame, offset and scrambler collapse them onto
     one point.

The output is the fold score, the best reference concentration and the
parameters that produced it. R near 1 is a frame in hand; R near 0.12 is noise,
and that ceiling was measured on an empty frequency rather than guessed.
"""

import sys
import numpy as np

BAUD = 2400.0
SPS = 8
FRAME = 256
SYNC = 80
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


def scrambler():
    bits = lfsr([4, 9], 511 * 3, 0b1).reshape(-1, 3)
    n = bits[:, 0] + 2 * bits[:, 1] + 4 * bits[:, 2]
    return np.exp(1j * np.pi / 4 * n[:176])


def baseband(samples, rate, centre):
    """One sample per symbol at 2400 baud, filtered to the channel."""
    src = np.arange(len(samples))
    t = src / rate
    bb = samples * np.exp(-2j * np.pi * centre * t)
    spec = np.fft.fft(bb)
    f = np.fft.fftfreq(len(bb), 1 / rate)
    spec[np.abs(f) > 1500.0] = 0.0
    bb = np.fft.ifft(spec)
    n_out = int(len(samples) * BAUD / rate)
    dst = np.arange(n_out) * rate / BAUD
    return np.interp(dst, src, bb.real) + 1j * np.interp(dst, src, bb.imag)


def fold(sym, frames):
    """Best frame phase and offset by folding, with a fine refinement."""
    n = np.arange(frames)
    T = FRAME / BAUD
    best = (0.0, None, None)
    for tau in range(FRAME):
        idx = tau + n[:, None] * FRAME + np.arange(FRAME)[None, :]
        idx = idx[idx[:, -1] < len(sym)]
        if len(idx) < 4:
            continue
        X = sym[idx]
        scale = np.abs(X).mean()
        if scale < 1e-12:
            continue
        for w in np.arange(-75.0, 75.1, 1.0):
            S = (np.exp(-1j * w * n * T)[: len(X)] / len(X)) @ X
            v = np.abs(S).mean() / scale
            if v > best[0]:
                best = (float(v), (tau, float(w)), S)
    # refine the offset: a coarse grid over a few frames smears the addition
    _, (tau, w0), _ = best
    idx = tau + n[:, None] * FRAME + np.arange(FRAME)[None, :]
    idx = idx[idx[:, -1] < len(sym)]
    X = sym[idx]
    for w in np.arange(w0 - 1.5, w0 + 1.5, 0.02):
        S = (np.exp(-1j * w * n * T)[: len(X)] / len(X)) @ X
        v = np.abs(S).mean() / np.abs(X).mean()
        if v > best[0]:
            best = (float(v), (tau, float(w)), S)
    return best


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return 1
    path = sys.argv[1]
    centre = float(sys.argv[2]) if len(sys.argv) > 2 else 1800.0
    frames = int(sys.argv[3]) if len(sys.argv) > 3 else 8
    from readwav import read_wav
    a, rate, meta = read_wav(path)
    print(f"{path}: {len(a)/rate:.1f}s at {rate} Hz {meta[2]}bit, centre {centre:.0f} Hz")
    sym = baseband(a, rate, centre)

    (score, (tau, w), S), = [(fold(sym[: frames * FRAME], frames))]
    print(f"fold {score:.3f} over {frames} frames (noise ~{1/np.sqrt(frames):.2f}) "
          f"at grid {tau}, offset {w:+.2f} Hz")

    scr = scrambler()
    ref = np.array([k for lo, hi in REF_AT for k in range(lo, hi)])
    dat = np.array([k for lo, hi in DATA_AT for k in range(lo, hi)])
    k = np.arange(FRAME)
    best = (0.0, None)
    for df in np.arange(-800, 800.5, 1.0):
        S2 = S * np.exp(-1j * 2 * np.pi * df * k / BAUD)
        for sign in (+1, -1):
            for off in range(176):
                idx = (ref + off) % 176
                z = S2[SYNC + idx] * np.conj(scr[idx] ** sign)
                m = np.abs(z)
                if m.mean() < 1e-12:
                    continue
                r = float(np.abs(np.mean(z / m)))
                if r > best[0]:
                    best = (r, (df, sign, off))
    r, (df, sign, off) = best
    print(f"reference concentration R = {r:.3f} at ramp {df:+.0f} Hz, scrambler "
          f"{sign:+d}, offset {off}  (noise ceiling 0.12)")
    if r < 0.5:
        print("  -> no frame in hand; this recording does not carry the layout")
        return 0

    S2 = S * np.exp(-1j * 2 * np.pi * df * k / BAUD)
    idx = (ref + off) % 176
    rot = np.exp(-1j * np.angle(np.sum(S2[SYNC + idx] * np.conj(scr[idx] ** sign))))
    d = S2[SYNC + (dat + off) % 176] * np.conj(scr[(dat + off) % 176] ** sign) * rot
    spread = float(np.degrees(np.abs(np.angle(d))).std())
    print(f"  data symbol spread {spread:.0f} deg "
          f"({'constant — idle or a test loop' if spread < 35 else 'varying — carries content'})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
