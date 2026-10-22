#!/usr/bin/env python3
"""Does this recording carry Edition 1's frame, and where does it start?

Usage: s4285_frame_check.py <file.wav> [carrier_hz]

The question `s4285_lock.py` could not answer on the 8467.8 kHz capture. That
script scores the 48 reference symbols as if they must collapse onto **one**
point, which only holds on a channel that does not move inside 106.67 ms. The
standard puts them in **three blocks of sixteen** — after each of the first three
32-symbol data blocks — so on a real path they collapse *per block*, and a
single-point test can reject a frame that is perfectly good.

What this does instead, in the order the frame has to be found:

  1. Mix the carrier down, resample to 8 samples per symbol, and take the
     differential symbol stream. A 33 ms coherent integration cannot survive the
     +/-75 Hz the standard allows, so differential is the only detector that
     works before the offset is known.
  2. Correlate against the differentially-encoded PN31 and ask whether the peak
     **repeats every 256 symbols**. One peak means nothing (a maximum over
     half a million offsets reaches 0.74 on noise, see s4285_probe/README.md);
     a peak that recurs at the frame period is the signal.
  3. Descramble the 176 symbols after the sync -- x^9 + x^4 + 1, seeded to 1 each
     frame, three bits per symbol.
  4. Score the three reference blocks **separately**, and then the data symbols
     against the rotation each block's own references give.

A frame that is right here shows: a sync peak repeating at 256 symbols, three
reference blocks each collapsing, and data symbols landing on 2, 4 or 8 phase
points after derotation -- which also tells the data rate.
"""

import sys

import numpy as np

BAUD = 2400.0
SPS = 8
RATE = BAUD * SPS          # 19200
SYNC = 80
FRAME = 256
# Within the 176 symbols after the sync: 32 data, 16 ref, x3, then 32 data.
REF_BLOCKS = [(32, 48), (80, 96), (128, 144)]


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


def sync_pattern():
    return np.where(lfsr([2, 5], SYNC, 0b11010) > 0, 1.0, -1.0)


def scrambler():
    """176 symbols: x^9 + x^4 + 1, seeded to 1, three bits per symbol."""
    state = [1] + [0] * 8
    out = []
    for _ in range(176):
        x0, x1, x2 = state[0], state[1], state[2]
        n = 4 * x2 + 2 * x1 + x0
        out.append(np.exp(1j * n * np.pi / 4))
        for _ in range(3):
            fb = state[0] ^ state[4]
            state = state[1:] + [fb]
    return np.array(out)


def baseband(samples, rate, carrier):
    n_out = int(len(samples) * RATE / rate)
    src = np.arange(len(samples))
    dst = np.arange(n_out) * rate / RATE
    bb = samples * np.exp(-2j * np.pi * carrier * src / rate)
    return np.interp(dst, src, bb.real) + 1j * np.interp(dst, src, bb.imag)


def sync_profile(bb):
    """Normalised differential correlation with the PN, per sub-phase."""
    prn = sync_pattern()
    dprn = prn[1:] * prn[:-1]
    best = None
    for sub in range(SPS):
        sym = bb[sub::SPS]
        if len(sym) < 8 * FRAME:
            continue
        d = sym[1:] * np.conj(sym[:-1])
        d = d / (np.abs(d) + 1e-12)
        mag = np.convolve(np.abs(d), np.ones(len(dprn)), "valid") + 1e-12
        c = np.convolve(d, dprn, "valid") / mag
        # Does the profile repeat at the frame period? That, not its maximum.
        n = len(c) - FRAME
        if n < FRAME:
            continue
        repeat = abs(np.sum(c[:n] * np.conj(c[FRAME:FRAME + n]))) / (
            np.sqrt(np.sum(np.abs(c[:n]) ** 2) * np.sum(np.abs(c[FRAME:FRAME + n]) ** 2)) + 1e-12)
        peak = float(np.max(np.abs(c[:n])))
        if best is None or repeat > best[0]:
            best = (repeat, peak, sub, d, c)
    return best


def try_carrier(samples, rate, carrier):
    bb = baseband(samples, rate, carrier)
    got = sync_profile(bb)
    if got is None:
        return None
    repeat, peak, sub, d, c = got
    return repeat, peak, sub, d, c, bb


def report(path, samples, rate):
    # The signal sits somewhere in the audio; find it from the spectrum rather
    # than assuming the dial is on the carrier.
    n = min(len(samples), int(rate * 4))
    S = np.abs(np.fft.rfft(samples[:n] * np.hanning(n))) ** 2
    f = np.fft.rfftfreq(n, 1 / rate)
    db = 10 * np.log10(S + 1e-20)
    inside = np.where(db > np.percentile(db, 99.0) - 12)[0]
    lo, hi = f[inside[0]], f[inside[-1]]
    centre = (lo + hi) / 2
    print(f"{path}\n  band {lo:.0f}-{hi:.0f} Hz, centre {centre:.0f} Hz")

    best = None
    for carrier in np.arange(max(lo - 400, 100), hi - 2000, 40.0):
        got = try_carrier(samples, rate, carrier)
        if got and (best is None or got[0] > best[0][0]):
            best = (got, carrier)
    if best is None:
        print("  no usable buffer")
        return
    (repeat, peak, sub, d, c, bb), carrier = best
    print(f"  best carrier {carrier:.0f} Hz, sub-phase {sub}: "
          f"sync repeat at 256 symbols {repeat:.3f}, single peak {peak:.3f}")
    if repeat < 0.25:
        print("  -> the sync does not recur clearly. But check the frame ACF before")
        print("     calling it: if the 80-symbol sync is the only part that repeats,")
        print("     the frame ACF is about 80/256 = 0.31, and a 36 s fold needs a")
        print("     carrier stable to a fraction of a hertz, which a night path is")
        print("     not. That case wants per-frame tracking, not a longer fold.")
        return

    # Frame start: the strongest peak of the profile that is consistent with the
    # recurrence -- take the best position modulo the frame.
    prof = np.abs(c[: len(c) - FRAME])
    folded = np.zeros(FRAME)
    for i, v in enumerate(prof):
        folded[i % FRAME] = max(folded[i % FRAME], v)
    k0 = int(np.argmax(folded))
    print(f"  frame start at symbol {k0} of the profile (frame phase {k0 % FRAME})")

    scram = scrambler()
    sym = bb[sub::SPS]
    sym = sym / (np.abs(sym) + 1e-12)
    starts = [k0 + FRAME * j for j in range(4, (len(sym) - FRAME) // FRAME - 4)]
    overall, per_block = [], [[] for _ in REF_BLOCKS]
    data_phase = []
    for s in starts:
        frame = sym[s:s + FRAME]
        if len(frame) < FRAME:
            break
        body = frame[SYNC:] * np.conj(scram)
        for bi, (a, b) in enumerate(REF_BLOCKS):
            ref = body[a:b]
            per_block[bi].append(abs(np.mean(ref)))
            # Derotate that block's data by its own reference mean.
            rot = np.mean(ref)
            if abs(rot) > 1e-9:
                data_phase.extend(np.angle(body[a - 32:a] * np.conj(rot / abs(rot))))
        overall.append(abs(np.mean(np.concatenate([body[a:b] for a, b in REF_BLOCKS]))))
    print(f"  reference concentration: overall {np.mean(overall):.3f}, "
          f"per block " + " ".join(f"{np.mean(b):.3f}" for b in per_block))
    ph = np.array(data_phase)
    if len(ph):
        h, _ = np.histogram((ph + np.pi / 8) % (2 * np.pi), bins=8, range=(0, 2 * np.pi))
        h = h / h.sum()
        print("  data symbols after derotation, 8 sectors: "
              + " ".join(f"{v:.2f}" for v in h))
        print(f"  concentration R = {abs(np.mean(np.exp(1j * ph))):.3f} "
              "(0 uniform, 1 all on one point)")


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    sys.path.insert(0, ".")
    from readwav import read_wav
    a, rate, meta = read_wav(sys.argv[1])
    print(f"{len(a)/rate:.1f} s at {rate:.0f} Hz {meta[2]}bit")
    report(sys.argv[1], a, rate)
    return 0


if __name__ == "__main__":
    sys.exit(main())
