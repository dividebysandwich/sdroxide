#!/usr/bin/env python3
"""One command instead of an evening: send a mode from fldigi, record it on the
RSP1, decode it with the engine the program ships, and say whether the text came
back.

The point is not convenience. Every wasted round in the 2026-10-20 bench session
was one of four things — fldigi in the wrong mode, fldigi left in TX, the wrong
audio centre, or a capture taken at a gain that saturated the receiver. All four
are decided here instead of by hand.

    ./bench.py --mode OLIVIA-16/500 --tones 16 --bw 500 \
               --text "TEST TEST THE EAGLE HAS LANDED 12345"

Requirements, all of which are already on the bench:

  * fldigi running with XML-RPC enabled (`CHKUSEXMLRPCIS` = 1, port 7362)
  * the CRT SS9900v on VOX, its PCM level not at zero (`amixer -c 1 sget PCM`)
  * the RSP1 free — **sdroxide must be closed**, since it holds the device
  * `tools/rsp1-capture/target/release/rspcap` built

Nothing here touches the radio's frequency: tune the transmitter by hand and pass
`--hz` to match.
"""

import argparse
import os
import subprocess
import sys
import time
import wave
import xmlrpc.client

import numpy as np

HOME = os.path.expanduser("~")
ROOT = os.path.join(HOME, "sdroxide")
RSPCAP = os.path.join(ROOT, "tools", "rsp1-capture", "target", "release", "rspcap")
DECODER = [os.path.join(ROOT, "target", "release", "examples", "decode_file")]
XMLRPC = "http://127.0.0.1:7362"


def fldigi():
    try:
        return xmlrpc.client.ServerProxy(XMLRPC, allow_none=True)
    except Exception as exc:  # noqa: BLE001
        sys.exit(f"fldigi XML-RPC is not reachable on {XMLRPC}: {exc}")


def record(hz, rate, seconds, gain, path):
    """Start rspcap detached; it writes the whole run at the end."""
    args = [RSPCAP, str(hz), str(rate), str(seconds), path, str(gain)]
    if not os.path.exists(RSPCAP):
        sys.exit(f"missing {RSPCAP} — build tools/rsp1-capture first")
    if os.path.exists(path):
        os.remove(path)
    return subprocess.Popen(args, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def transmit(p, text, tx_seconds):
    """Key fldigi, let the text out, then release TX ourselves.

    Olivia fills the gap after the text with idle tones and fldigi holds TX until
    it is told otherwise, so waiting for the state to fall back to RX never
    finishes — that is what left the carrier up for ninety seconds last time.
    """
    p.main.rx()
    time.sleep(0.3)
    p.text.clear_tx()
    p.text.add_tx(text)
    p.main.tx()
    deadline = time.time() + tx_seconds
    while time.time() < deadline:
        time.sleep(0.5)
    p.main.rx()
    time.sleep(0.3)


def audio_from_iq(path, rate, tones, bandwidth):
    """The USB audio of a capture, band-limited and resampled to 8 kHz.

    A USB receiver's audio is the real part of the baseband; the signal is found
    by looking for the strongest block of the mode's own width rather than by
    assuming a centre — the audio offset depends on where the operator set the
    dial, and getting it wrong is a silent failure.
    """
    x = np.fromfile(path, dtype=np.complex64)
    if x.size == 0:
        sys.exit("the capture is empty — is the RSP1 free?")
    a = x.real.astype(np.float64)
    a -= a.mean()
    spec = np.abs(np.fft.rfft(a * np.hanning(a.size))) ** 2
    f = np.fft.rfftfreq(a.size, 1.0 / rate)
    win = int(bandwidth * 1.2 / (f[1] - f[0])) + 1
    energy = np.convolve(spec, np.ones(win) / win, "same")
    centre = f[int(np.argmax(energy))]

    keep = (f >= centre - bandwidth) & (f <= centre + bandwidth)
    b = np.fft.irfft(np.where(keep, np.fft.rfft(a), 0), n=a.size)
    out = int(round(b.size * 8000.0 / rate))
    X = np.fft.rfft(b)
    Y = np.zeros(out // 2 + 1, dtype=complex)
    k = min(len(Y), len(X))
    Y[:k] = X[:k]
    wav = np.fft.irfft(Y, n=out)
    peak = np.abs(wav).max()
    if peak > 0:
        wav = wav / peak * 0.7
    return centre, wav


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--mode", default="OLIVIA-16/500", help="fldigi mode name")
    ap.add_argument("--tones", type=int, default=16)
    ap.add_argument("--bw", type=int, default=500)
    ap.add_argument("--reverse", type=int, default=1, choices=[0, 1])
    ap.add_argument("--text", default="TEST TEST THE EAGLE HAS LANDED 12345")
    ap.add_argument("--hz", type=float, default=27_415_000.0, help="transmitter dial")
    ap.add_argument("--rate", type=int, default=250_000, help="RSP1 sample rate")
    ap.add_argument("--gain", type=int, default=55, help="RSP1 gain; 20 loudest, 59 quietest")
    ap.add_argument("--tx-seconds", type=float, default=25.0)
    ap.add_argument("--seconds", type=float, default=40.0, help="capture length")
    ap.add_argument("--keep", default="/tmp/bench_last", help="where the capture and wav go")
    args = ap.parse_args()

    p = fldigi()
    p.modem.set_by_name(args.mode)
    time.sleep(0.5)
    print(f"fldigi mode {p.modem.get_name()}, carrier {p.modem.get_carrier()}")

    iq = f"{args.keep}.bin"
    rec = record(args.hz, args.rate, args.seconds, args.gain, iq)
    time.sleep(1.5)
    transmit(p, args.text, args.tx_seconds)

    print("waiting for the recorder…")
    rec.wait(timeout=args.seconds + 30)

    centre, wav = audio_from_iq(iq, args.rate, args.tones, args.bw)
    wavpath = f"{args.keep}.wav"
    with wave.open(wavpath, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(8000)
        w.writeframes(np.clip(wav * 32767, -32768, 32767).astype("<i2").tobytes())
    level = 20 * np.log10(max(wav.std(), 1e-9) / 0.101)  # rough dBFS on speech
    print(f"signal found at {centre:.0f} Hz of audio, level {level:.0f} dBFS, {wav.size/8000:.1f}s")

    result = subprocess.run(
        DECODER
        + [wavpath, str(args.tones), str(args.bw), f"{centre:.0f}", str(args.reverse), *args.text.split()],
        capture_output=True,
        text=True,
    )
    text = result.stdout.strip()
    print(f"decoded: {text!r}")
    if result.returncode == 0:
        print("PASS")
        return 0
    print(result.stderr.strip() or "(no text)")
    print("FAIL")
    return 1


if __name__ == "__main__":
    sys.exit(main())
