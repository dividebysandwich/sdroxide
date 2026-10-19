#!/usr/bin/env python3
"""Find a STANAG 4285 sync in a recording.

Usage: s4285_sync.py <file.wav> [centre_hz]

Mixes the recording to baseband at ``centre_hz`` (default 1800, the standard's
own value), resamples to eight samples per symbol, and correlates the 80-symbol
synchronisation sequence.

The correlation is **differential**, and that is not a detail. The sequence is
differentially encoded, so a coherent correlator loses it as soon as the
receiver's carrier differs from the sender's — which the standard allows by
±75 Hz, and which a 33 ms integration turns into a complete rotation at 30 Hz.
Differential detection removes the unknown offset before the correlation sees it.

Two independent checks are printed, because a correlation on its own is easy to
read too optimistically:

  * the frame-period autocorrelation — 256 symbols at 2400 baud, so 106.67 ms.
    A real 4285 signal repeats that much of itself every frame; noise does not.
  * the normalised sync correlation — above about 0.6 on a decent signal, and
    around 0.2-0.4 (the sequence's own bias) when there is nothing to find.

Needs numpy and readwav.py beside it.
"""

import sys
import numpy as np

BAUD = 2400.0
SPS = 8
SYNC = 80
FRAME = 256


def lfsr(taps, n, init):
    """Fibonacci LFSR, least significant bit first, as the standard describes."""
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
    """x^5 + x^2 + 1, initialised to 11010."""
    return np.where(lfsr([2, 5], SYNC, 0b11010) > 0, 1.0, -1.0)


def baseband(samples, rate, centre):
    n_out = int(len(samples) * BAUD * SPS / rate)
    src = np.arange(len(samples))
    dst = np.arange(n_out) * rate / (BAUD * SPS)
    bb = samples * np.exp(-2j * np.pi * centre * src / rate)
    return (np.interp(dst, src, bb.real) + 1j * np.interp(dst, src, bb.imag))


def frame_acf(bb, rate):
    """|ACF| at the 106.67 ms frame period, against the median elsewhere."""
    period = int(round(rate * FRAME / BAUD))
    def acf(lag):
        x, y = bb[:-lag], bb[lag:]
        return abs(np.sum(x * np.conj(y))) / (
            np.sqrt(np.sum(np.abs(x) ** 2) * np.sum(np.abs(y) ** 2)) + 1e-12)
    lags = np.arange(int(rate * 0.02), int(rate * 0.25))
    vals = np.array([acf(int(l)) for l in lags])
    near = np.abs(lags - period) < rate * 0.001
    return float(vals[near].max()), float(np.median(vals))


def sync_peak(bb):
    """Best normalised differential correlation, over sub-phases and offsets."""
    prn = sync_pattern()
    dprn = prn[1:] * prn[:-1]
    best = (0.0, None)
    for sub in range(SPS):
        sym = bb[sub::SPS]
        if len(sym) < SYNC + 2:
            continue
        d = sym[1:] * np.conj(sym[:-1])
        mag = np.convolve(np.abs(d), np.ones(len(dprn)), "valid") + 1e-12
        c = np.abs(np.convolve(d, dprn, "valid")) / mag
        i = int(np.argmax(c))
        if c[i] > best[0]:
            best = (float(c[i]), (i, sub))
    return best


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return 1
    path = sys.argv[1]
    centres = [float(sys.argv[2])] if len(sys.argv) > 2 else [1800.0]
    from readwav import read_wav
    a, rate, meta = read_wav(path)
    print(f"{path}: {len(a)/rate:.1f}s at {rate} Hz {meta[2]}bit")
    for centre in centres:
        bb = baseband(a, rate, centre)
        frame, median = frame_acf(bb, rate)
        peak, where = sync_peak(bb)
        at = f" at symbol {where[0]} sub-phase {where[1]}" if where else ""
        print(f"  centre {centre:5.0f} Hz: frame ACF {frame:.3f} (median {median:.3f}, "
              f"{frame/max(median,1e-9):.1f}x)   sync {peak:.3f}{at}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
