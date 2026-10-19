#!/usr/bin/env python3
"""Find a STANAG 4285 frame by folding the recording onto its own frame period.

Usage: s4285_fold.py <file.wav> [centre_hz] [--frames N]

Every frame carries the same 80 sync symbols and the same 48 reference symbols,
so folding the signal at 256-symbol intervals and adding frames coherently makes
those 128 symbols stand up while the 128 data symbols average towards noise. The
score is how much of the frame survives that addition:

    fold(tau, w) = mean over frames of x[tau + n*256 + k] * exp(-j w n T)

    score = mean_k |fold| / mean_k mean_n |x|

1.0 would be a frame that repeats exactly; a random frame scores about
1/sqrt(frames). Both the frame phase and the carrier offset are searched, because
a residual offset destroys the addition just as surely as a wrong phase.

This is the acquisition the standard's Annex F is about: the receiver does not
know where the frame starts or what the Doppler is, and it has to get both at
once. A search over both is the honest version, and it is cheap because the
frames do the averaging.

What it does NOT need is the sync pattern — so a lock here cannot be an artefact
of a guessed sequence, which is what made the earlier correlations unreadable.
"""

import sys
import numpy as np

BAUD = 2400.0
SPS = 8
FRAME = 256
SYNC = 80


def baseband(samples, rate, centre):
    n_out = int(len(samples) * BAUD * SPS / rate)
    src = np.arange(len(samples))
    dst = np.arange(n_out) * rate / (BAUD * SPS)
    bb = samples * np.exp(-2j * np.pi * centre * src / rate)
    spec = np.fft.fft(bb)
    f = np.fft.fftfreq(len(bb), 1 / rate)
    spec[np.abs(f) > 1500.0] = 0.0
    bb = np.fft.ifft(spec)
    return np.interp(dst, src, bb.real) + 1j * np.interp(dst, src, bb.imag)


def fold(sym, frames):
    """Best frame phase and carrier offset, and the folded frame itself."""
    span = frames * FRAME
    if len(sym) < span + FRAME:
        frames = (len(sym) - FRAME) // FRAME
        span = frames * FRAME
    n = np.arange(frames)
    w_grid = np.arange(-75.0, 75.1, 1.0)
    period = FRAME / BAUD
    # one weight row per candidate offset, applied once per frame
    W = np.exp(-1j * np.outer(w_grid, n * period)) / frames

    best = (0.0, None, None)
    for tau in range(FRAME):
        # frames x FRAME view of the recording, one sample per symbol
        idx = tau + n[:, None] * FRAME + np.arange(FRAME)[None, :]
        idx = idx[idx[:, -1] < len(sym)]
        if len(idx) < 4:
            continue
        X = sym[idx]
        scale = np.abs(X).mean()
        if scale < 1e-12:
            continue
        S = W @ X                      # offsets x FRAME: the folded frames
        mag = np.abs(S).mean(1) / scale
        i = int(np.argmax(mag))
        if mag[i] > best[0]:
            best = (float(mag[i]), (tau, float(w_grid[i])), S[i])
    return best, len(n)


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return 1
    path = sys.argv[1]
    centre = 1800.0
    frames = 64
    args = sys.argv[2:]
    if args and not args[0].startswith("--"):
        centre = float(args[0])
        args = args[1:]
    for i, a in enumerate(args):
        if a == "--frames":
            frames = int(args[i + 1])

    from readwav import read_wav
    a, rate, meta = read_wav(path)
    print(f"{path}: {len(a)/rate:.1f}s at {rate} Hz {meta[2]}bit, centre {centre:.0f} Hz")
    # One sample per symbol: `fold` indexes frames in symbols, and `baseband`
    # hands back SPS samples per symbol.
    sym = baseband(a, rate, centre)[::SPS]
    print(f"{len(sym)/BAUD:.1f}s of symbols; folding {frames} frames "
          f"(noise floor is about {1/np.sqrt(frames):.2f})")

    (score, where, folded), used = fold(sym, frames)
    if where is None:
        print("nothing to fold")
        return 1
    tau, w = where
    print(f"fold score {score:.3f} at frame phase {tau} symbols, offset {w:+.0f} Hz "
          f"({used} frames)")
    # where does the frame stand up? the sync and reference halves should
    np.save("/tmp/opencode/s4285/folded.npy", folded)
    half = [np.abs(folded[i * 64:(i + 1) * 64]).mean() for i in range(4)]
    print("folded amplitude by quarter of the frame:",
          "  ".join(f"{h:.4f}" for h in half))
    return 0


if __name__ == "__main__":
    sys.exit(main())
