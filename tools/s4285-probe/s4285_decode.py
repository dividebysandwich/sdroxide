#!/usr/bin/env python3
"""Decode a locked STANAG 4285 recording, frame by frame.

Usage: s4285_decode.py <file.wav> [centre_hz]

Acquisition is by folding the recording onto its own frame period (see
`s4285_fold.py`), which gives both the frame phase and the carrier offset per
block of frames — both change, because a sound-card clock is never exact and the
ionosphere moves the carrier.

Then, for every frame: derotate, descramble with the 176-symbol sequence, and use
the 48 reference symbols to take out the residual phase. Two things are reported
and they are the whole diagnosis:

  * **reference concentration** — the 48 reference symbols are all symbol 0
    before scrambling, so after a correct descramble they must collapse onto one
    point. R near 1.0 means the frame, the scrambler and the carrier are all
    right; R near 1/sqrt(48) = 0.14 means they are not.
  * **the data constellation** — how many phases the data symbols fall into
    identifies the rate: two is uncoded 1200 bps, four is uncoded 2400, eight is
    uncoded 3600. A spread means a *coded* rate, whose symbols carry FEC output
    rather than message bits.
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


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return 1
    path = sys.argv[1]
    centre = float(sys.argv[2]) if len(sys.argv) > 2 else 1800.0
    from readwav import read_wav
    import s4285_fold as F

    a, rate, meta = read_wav(path)
    sym = F.baseband(a, rate, centre)[::SPS]
    print(f"{path}: {len(sym)/BAUD:.1f}s at {rate} Hz, centre {centre:.0f} Hz")

    scr = scrambler()
    term = np.ones(FRAME, dtype=complex)
    term[SYNC:] = np.conj(scr)
    ref_pos = np.array([k for lo, hi in REF_AT for k in range(lo, hi)])
    data_pos = np.array([k for lo, hi in DATA_AT for k in range(lo, hi)])
    t = np.arange(FRAME) / BAUD

    # The fold cannot say *where* in the frame it is: rotating the frame leaves
    # the score unchanged, since the score is the mean over all 256 positions.
    # So the fold gives the grid and the carrier offset, and the 48 reference
    # symbols — which are all symbol 0 — say where the frame begins. That is
    # what they are for.
    block = 32
    probe = sym[:block * FRAME]
    (score0, where0, folded), _ = F.fold(probe, block)
    if where0 is None:
        print("nothing to fold")
        return 1
    tau0, w0 = where0
    best_rot = (0.0, 0)
    for r in range(FRAME):
        S = np.roll(folded, -r) * np.exp(-2j * np.pi * w0 * t)
        z = S * term
        ref = z[SYNC + ref_pos]
        m = np.abs(ref)
        if m.mean() < 1e-12:
            continue
        c = float(np.abs(np.mean(ref / m)))
        if c > best_rot[0]:
            best_rot = (c, r)
    print(f"frame grid at {tau0}, offset {w0:+.0f} Hz; the references put the "
          f"frame start {best_rot[1]} symbols later (R {best_rot[0]:.3f})")
    tau0 = (tau0 + best_rot[1]) % FRAME

    phases, refs, data, used = [], [], [], 0
    for c0 in range(0, (len(sym) // FRAME) // block):
        seg = sym[c0 * block * FRAME:(c0 + 1) * block * FRAME]
        (score, where, _folded), _ = F.fold(seg, block)
        if where is None or score < 0.30:
            continue
        _, w = where
        tau = tau0
        for f in range(c0 * block, (c0 + 1) * block):
            st = f * FRAME + tau
            if st + FRAME > len(sym):
                break
            z = sym[st:st + FRAME] * term * np.exp(-2j * np.pi * w * t)
            ref = z[SYNC + ref_pos]
            m = np.abs(ref)
            if m.mean() < 1e-9:
                continue
            r = float(np.abs(np.mean(ref / m)))
            if r < 0.5:
                continue
            rot = np.exp(-1j * np.angle(np.sum(ref / m)))
            data.append(z[SYNC + data_pos] * rot)
            used += 1
            phases.append(r)

    if not data:
        print("no frames above the lock threshold")
        return 1
    rs = np.array(phases)
    d = np.concatenate(data)
    amp = np.abs(d)
    d = d[amp > np.percentile(amp, 50)]     # the stronger half, fading is severe
    print(f"{used} frames decoded; reference concentration median {np.median(rs):.3f}, "
          f"best {rs.max():.3f}  (noise 0.14)")
    hist, _ = np.histogram(np.angle(d), bins=8, range=(-np.pi, np.pi))
    tot = hist.sum()
    print("data-symbol phase histogram (45 deg sectors, centred on n*45):")
    for i, h in enumerate(hist):
        print(f"  {-180+45*i+22.5:+7.1f} deg: {h:6d} {h/tot:5.1%} {'#'*int(50*h/max(hist.max(),1))}")
    for k, why in ((2, "uncoded 1200 (BPSK)"), (4, "uncoded 2400 (QPSK)"), (8, "uncoded 3600 (8-PSK)")):
        nz = np.sort(hist)[::-1][:k].sum() / tot
        print(f"  top {k} sectors hold {nz:5.1%}  -> {why}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
