#!/usr/bin/env python3
"""Find a STANAG 4285 frame without trusting the sync sequence.

Usage: s4285_frame.py <file.wav> [centre_hz]

The frame carries 48 **reference** symbols that are all symbol 0 before
scrambling, in three groups of sixteen. That is a much better handle than the
sync sequence: if the frame phase, the scrambler and the carrier offset are all
right, those 48 symbols collapse onto a single point in the complex plane, and
if any of the three is wrong they scatter. So the score here is their
concentration,

    R = |mean_k exp(j (phi_k - omega t_k))|

over the 48, which is 1.0 for a perfect lock and about 1/sqrt(48) = 0.14 for
noise. It needs no knowledge of the sync pattern at all, so it cannot be fooled
by a plausible-looking correlation.

The search is over frame phase, sub-phase within a symbol, carrier offset (the
standard allows +/-75 Hz) and the sign convention of the scrambler.
"""

import sys
import numpy as np

BAUD = 2400.0
SPS = 8
SYNC = 80
FRAME = 256
REF_AT = [(32, 48), (80, 96), (128, 144)]  # within the scrambled 176


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
    """x^9 + x^4 + 1, three bits per symbol, n = 4*x2 + 2*x1 + x0."""
    bits = lfsr([4, 9], 511 * 3, 0b1).reshape(-1, 3)
    n = bits[:, 0] + 2 * bits[:, 1] + 4 * bits[:, 2]
    return np.exp(1j * np.pi / 4 * n[:176])


def baseband(samples, rate, centre):
    n_out = int(len(samples) * BAUD * SPS / rate)
    src = np.arange(len(samples))
    dst = np.arange(n_out) * rate / (BAUD * SPS)
    bb = samples * np.exp(-2j * np.pi * centre * src / rate)
    return np.interp(dst, src, bb.real) + 1j * np.interp(dst, src, bb.imag)


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return 1
    path = sys.argv[1]
    centre = float(sys.argv[2]) if len(sys.argv) > 2 else 1800.0
    # Optional hint from s4285_sync.py: searching 97k start positions is fine for
    # a 14 s fixture and far too slow for a minute of one.
    hint = int(sys.argv[3]) if len(sys.argv) > 3 else None
    span = int(sys.argv[4]) if len(sys.argv) > 4 else 400
    from readwav import read_wav
    a, rate, meta = read_wav(path)
    print(f"{path}: {len(a)/rate:.1f}s at {rate} Hz {meta[2]}bit, centre {centre:.0f} Hz")
    bb = baseband(a, rate, centre)
    scr = scrambler()
    ref_pos = np.array([k for lo, hi in REF_AT for k in range(lo, hi)])
    w_grid = np.arange(-75.0, 75.1, 1.0)

    best = (0.0, None)
    for sub in range(SPS):
        sym = bb[sub::SPS]
        n = len(sym)
        if n < FRAME * 2:
            continue
        t = np.arange(FRAME) / BAUD
        tref = t[SYNC + ref_pos]
        for sign in (+1, -1):
            desc = np.ones(FRAME, dtype=complex)
            desc[SYNC:] = scr ** sign
            starts = (range(max(0, hint - span), max(0, hint - span) + 2 * span)
                      if hint is not None else range(0, n - FRAME, 1))
            for start in starts:
                z = sym[start:start + FRAME] * np.conj(desc)
                ref = z[SYNC + ref_pos]
                if np.abs(ref).mean() < 1e-9:
                    continue
                ph = ref / np.abs(ref)
                # R(omega) for a grid of carrier offsets, one expansion of the phasors
                e = np.exp(-1j * np.outer(w_grid, tref))
                r = np.abs(ph @ e.T) / len(ref)
                i = int(np.argmax(r))
                if r[i] > best[0]:
                    best = (float(r[i]), (start, sub, sign, float(w_grid[i])))

    r, where = best
    start, sub, sign, w = where
    print(f"best reference concentration R = {r:.3f}  (1.0 perfect, ~0.14 noise)")
    print(f"  frame start symbol {start}, sub-phase {sub}, scrambler sign {sign:+d}, offset {w:+.0f} Hz")
    print(f"  start time {start/BAUD*1000:.2f} ms; frame offset within period {start % FRAME}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
