#!/usr/bin/env python3
"""Demodulate a locked STANAG 4285 recording and report what the frame holds.

Usage: s4285_demod.py <file.wav> [centre_hz]

Assumes the frame phase and scrambler convention found by `s4285_frame.py`.
Uses each frame's own 48 reference symbols to take out the carrier phase, which
handles drift without a second search, then reports:

  * the reference concentration per frame — the health of the lock;
  * a phase histogram of the data symbols.

That histogram is what identifies the transmission. The constellation is chosen
by the data rate, so the number of clusters the data symbols fall into says
which rate it is: two phases is uncoded 1200 bps (BPSK), four is uncoded 2400
(dibits), eight is uncoded 3600 (tribits). A rate that is spread rather than
clustered is a *coded* one — 600/300/150/75 — whose symbols carry FEC output
rather than message bits, and those need Annex E, which the public copy of the
standard does not include.
"""

import sys
import numpy as np

BAUD = 2400.0
SPS = 8
SYNC = 80
FRAME = 256
REF_AT = [(32, 48), (80, 96), (128, 144)]
# data symbol positions within the 176 scrambled symbols
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
    start = int(sys.argv[3]) if len(sys.argv) > 3 else 13724
    sub = int(sys.argv[4]) if len(sys.argv) > 4 else 4
    offset = float(sys.argv[5]) if len(sys.argv) > 5 else -25.0

    from readwav import read_wav
    a, rate, meta = read_wav(path)
    bb = baseband(a, rate, centre)
    sym = bb[sub::SPS]

    scr = scrambler()
    ref_pos = np.array([k for lo, hi in REF_AT for k in range(lo, hi)])
    data_pos = np.array([k for lo, hi in DATA_AT for k in range(lo, hi)])
    desc = np.ones(FRAME, dtype=complex)
    desc[SYNC:] = np.conj(scr)

    # The carrier offset is not constant: selective fading moves it, and a
    # single value fitted over the whole recording only holds for one frame.
    # Each frame therefore gets its own, searched around the coarse figure, so
    # the estimate tracks instead of ageing.
    t = np.arange(FRAME) / BAUD
    tref = t[SYNC + ref_pos]
    w_grid = np.arange(-75.0, 75.1, 1.0)   # the standard's own allowance

    phases, rs, used = [], [], 0
    for k in range((len(sym) - start) // FRAME):
        # let the frame boundary move a little: a sound clock is never exact
        best = (0.0, None)
        for slip in range(-3, 4):
            f = start + k * FRAME + slip
            if f < 0 or f + FRAME > len(sym):
                continue
            zz = sym[f:f + FRAME] * desc
            rr = zz[SYNC + ref_pos]
            if np.abs(rr).mean() < 1e-9:
                continue
            pp = rr / (np.abs(rr) + 1e-12)
            g = np.abs(pp @ np.exp(-1j * np.outer(w_grid, tref)).T) / len(rr)
            i = int(np.argmax(g))
            if g[i] > best[0]:
                best = (float(g[i]), (f, float(w_grid[i])))
        if best[1] is None:
            continue
        f, w = best[1]
        z = sym[f:f + FRAME] * desc
        ref = z[SYNC + ref_pos]
        if np.abs(ref).mean() < 1e-9:
            continue
        r = best[0]
        rs.append(r)
        zz = ref * np.exp(-1j * 2 * np.pi * w * tref)
        rot = np.exp(-1j * np.angle(np.sum(zz)))
        data = z[SYNC + data_pos] * np.exp(-1j * 2 * np.pi * w * t[SYNC + data_pos]) * rot
        # only trust frames with a decent lock
        if r > 0.35:
            phases.append(np.angle(data))
            used += 1

    if not phases:
        print("no frames with a usable lock")
        return 1
    ph = np.concatenate(phases)
    rs = np.array(rs)
    print(f"{path}: {len(rs)} frames, {used} with R > 0.35")
    print(f"reference concentration: median {np.median(rs):.3f}, best {rs.max():.3f}")
    hist, _ = np.histogram(ph, bins=8, range=(-np.pi, np.pi))
    tot = hist.sum()
    print("data-symbol phase histogram over 8 sectors (45 deg each, centred on n*45):")
    for i, h in enumerate(hist):
        centre_deg = -180 + 45 * i + 22.5
        print(f"  {centre_deg:+7.1f} deg: {h:6d}  {h/tot:5.1%}  {'#'*int(60*h/max(hist.max(),1))}")
    # how well the data collapses onto a few phases
    for k, expect in ((2, "uncoded 1200 (BPSK)"), (4, "uncoded 2400 (QPSK)"), (8, "uncoded 3600 (8-PSK)")):
        nz = np.sort(hist)[::-1][:k].sum() / tot
        print(f"  top {k} sectors hold {nz:5.1%}  -> {expect}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
