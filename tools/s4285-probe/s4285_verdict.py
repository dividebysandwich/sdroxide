#!/usr/bin/env python3
"""Is this recording a STANAG 4285 transmission, and is anything there at all?

Usage:
  s4285_verdict.py <file.wav> [centre_hz]
  s4285_verdict.py --record [seconds] [centre_hz]      # capture the default monitor

`--record` needs `parec` (PulseAudio/PipeWire) and records the *monitor* of the
default sink, so whatever a browser is playing is captured without silencing it.
It writes `s4285-capture.wav` beside itself and then analyses it.

Why this file exists
--------------------
`s4285_sync.py` reports the *maximum* sync correlation over a whole recording,
and on 60 s of pure white noise that maximum reaches **0.738**. Its docstring's
"0.2-0.4 when there is nothing to find" is a single-window figure, not a
maximum-over-a-million-offsets figure, so read alone it invites a false
positive. Two statistics survive that treatment because each is compared with a
null measured on the same audio:

  * **the frame period** -- 256 symbols at 2400 baud, 106.67 ms. A real 4285
    repeats that much of itself every frame; noise does not. Reported against
    the median of nearby lags, and the best lag is printed so a *different*
    period (a WebSDR's 13 Hz audio striping, say) is visible rather than
    hidden.
  * **the 8th-power line** -- a constant-modulus 8PSK signal, squared up eight
    times, leaves a carrier. Measured on the known-good file at 51 dB over the
    median and on noise at 15 dB, so the two are not close.

A recording can pass one and fail the other, and the verdict says which.
"""

import subprocess
import sys
import tempfile
import wave

import numpy as np

BAUD = 2400.0
FRAME = 256
SPS = 8
RATE = BAUD * SPS          # 19200 Hz, the probe's working rate


def baseband(samples, rate, centre):
    n_out = int(len(samples) * RATE / rate)
    src = np.arange(len(samples))
    dst = np.arange(n_out) * rate / RATE
    bb = samples * np.exp(-2j * np.pi * centre * src / rate)
    return np.interp(dst, src, bb.real) + 1j * np.interp(dst, src, bb.imag)


def occupied_band(samples, rate):
    """Where the audio actually is, in Hz: the 10 dB-down span of the spectrum."""
    n = min(len(samples), int(rate * 8))
    s = samples[: n // 2 * 2]
    S = np.abs(np.fft.rfft(s * np.hanning(len(s)))) ** 2
    f = np.fft.rfftfreq(len(s), 1 / rate)
    db = 10 * np.log10(S + 1e-20)
    peak = np.percentile(db, 99.5)
    inside = np.where(db > peak - 10)[0]
    lo = f[inside[0]] if len(inside) else 0.0
    hi = f[inside[-1]] if len(inside) else 0.0
    return lo, hi


def acf(bb, lag):
    x, y = bb[:-lag], bb[lag:]
    return abs(np.sum(x * np.conj(y))) / (
        np.sqrt(np.sum(np.abs(x) ** 2) * np.sum(np.abs(y) ** 2)) + 1e-12)


def frame_period(bb):
    """|ACF| at 106.67 ms, its null, and the best lag anywhere in 60-160 ms."""
    period = int(round(RATE * FRAME / BAUD))
    lags = np.arange(int(RATE * 0.06), int(RATE * 0.16), 4)
    vals = np.array([acf(bb, int(l)) for l in lags])
    near = np.abs(lags - period) < RATE * 0.001
    at_frame = float(vals[near].max())
    median = float(np.median(vals))
    k = int(np.argmax(vals))
    return at_frame, median, int(lags[k]), float(vals[k])


def psK_line(bb):
    """Peak of the 8th power, in dB over its own median -- an 8PSK carrier test."""
    y = bb / (np.abs(bb) + 1e-12)
    z = y**8
    r = int(RATE)
    n = len(z) // r * r
    Z = np.abs(np.fft.fft(z[:n])) ** 2
    f = np.fft.fftfreq(n, 1 / RATE)
    half = (f >= 0) & (f < 3000)
    return float(10 * np.log10(Z[half].max() / (np.median(Z[half]) + 1e-20) + 1e-12))


def analyse(samples, rate, centre=None):
    lo, hi = occupied_band(samples, rate)
    if centre is None:
        centre = (lo + hi) / 2
    bb = baseband(samples, rate, centre)
    at_frame, null, best_lag, best = frame_period(bb)
    line = psK_line(bb)
    rms = 20 * np.log10(np.sqrt((samples**2).mean()) + 1e-12)

    print(f"  level        {rms:6.1f} dBFS")
    print(f"  audio band   {lo:6.0f} - {hi:6.0f} Hz  (centre used {centre:.0f} Hz)")
    print(f"  frame ACF    {at_frame:.3f} at 106.67 ms, null {null:.3f} -> "
          f"{at_frame / max(null, 1e-9):5.1f}x")
    print(f"  best lag     {best_lag / RATE * 1000:6.1f} ms  ({best:.3f})")
    print(f"  8PSK line    {line:6.1f} dB over median")

    framed = at_frame > 0.25 and at_frame > 3 * null
    psk = line > 25.0
    if framed and psk:
        print("  verdict      4285-shaped: the frame period and an 8PSK carrier are both there")
    elif framed:
        print("  verdict      a 106.67 ms frame without a PSK carrier -- check the centre")
    elif psk:
        print("  verdict      a PSK carrier, but no 256-symbol frame: another mode, or a variant")
    else:
        print("  verdict      nothing here -- band noise, or an idle channel")
    return framed and psk


def record(seconds, device=None):
    out = "s4285-capture.wav"
    cmd = ["parec"]
    if device:
        cmd += ["-d", device]
    else:
        cmd += ["-d", "@DEFAULT_MONITOR@"]
    cmd += ["--rate=48000", "--channels=1", "--format=s16le", "--file-format=raw"]
    print(f"recording {seconds} s of the default monitor ...")
    with tempfile.TemporaryFile() as raw:
        p = subprocess.Popen(cmd, stdout=raw)
        try:
            p.wait(timeout=seconds)
        except subprocess.TimeoutExpired:
            p.terminate()
            p.wait()
        raw.seek(0)
        data = np.frombuffer(raw.read(), dtype="<i2").astype(np.float64) / 32768.0
    with wave.open(out, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(48000)
        w.writeframes((data * 32767).astype("<i2").tobytes())
    print(f"wrote {out}: {len(data) / 48000:.1f} s\n")
    return data, 48000.0


def main():
    args = [a for a in sys.argv[1:]]
    if not args:
        print(__doc__)
        return 2
    centre = None
    if args[0] == "--record":
        args = args[1:]
        secs = 30.0
        if args and args[0].replace(".", "", 1).isdigit():
            secs = float(args.pop(0))
        if args:
            centre = float(args.pop(0))
        a, rate = record(secs)
    else:
        path = args.pop(0)
        if args:
            centre = float(args.pop(0))
        sys.path.insert(0, ".")
        from readwav import read_wav
        a, rate, meta = read_wav(path)
        print(f"{path}: {len(a) / rate:.1f}s at {rate} Hz {meta[2]}bit")
    return 0 if analyse(a, rate, centre) else 1


if __name__ == "__main__":
    sys.exit(main())
