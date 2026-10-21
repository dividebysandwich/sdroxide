#!/usr/bin/env python3
"""Record every mode fldigi can send, with known text, into a corpus.

One run, one directory, one manifest. For each mode: set it on fldigi over
XML-RPC, transmit the fixed test string, record the RSP1, save the audio at
8 kHz with its measured centre and level, and record what we know about it.

    ./sweep.py --only "^OLIVIA" --tx-seconds 40
    ./sweep.py --only "^(MFSK|THOR|DOMEX|DOMX)" --tx-seconds 30
    ./sweep.py --all --tx-seconds 30

The manifest is `manifest.jsonl` — one JSON object per mode, appended as each
finishes — so a sweep that dies at mode 90 keeps the 89 before it and can be
resumed with `--resume`. A readable summary is written to `manifest.md`.

Modes that carry a picture rather than text are skipped with a reason recorded,
not silently dropped.
"""

import argparse
import json
import os
import re
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
OUT = os.path.join(HOME, "Downloads", "sounds", "our-captures", "fldigi")
XMLRPC = "http://127.0.0.1:7362"

# The operator's text, exactly. Leading and trailing spaces are kept: they give
# the decoder something to settle on at both ends.
TEXT = " AAA TEST TEST THE EAGLE HAS LANDED 12345 SDRBROWN ZZZ "

# Modes that do not carry text, with the reason recorded in the manifest.
NOT_TEXT = {
    "ANALYSIS": "spectrum analyser, no data",
    "FMT": "frequency measurement tone",
    "WWV": "time standard tones",
    "SSB": "no modem at all; a pass-through",
    "NULL": "no modem",
    "RSID_EOT": "end-of-transmission marker",
    "IFKP": "image mode",
    "WEFAX288": "image mode",
    "WEFAX576": "image mode",
    # Picture-bearing MFSK variants are named with a trailing P or L in fldigi's
    # own tables; the plain ones are text and are kept.
}


class TimeoutTransport(xmlrpc.client.Transport):
    """A socket timeout, so a modal dialog cannot hang a whole batch.

    A dialog steals fldigi's main loop and XML-RPC answers nothing; without this
    the sweep waits for ever rather than moving on.
    """

    def make_connection(self, host):
        conn = super().make_connection(host)
        conn.timeout = self.hang_up_after
        return conn

    hang_up_after = 60


def proxy():
    try:
        return xmlrpc.client.ServerProxy(XMLRPC, transport=TimeoutTransport(), allow_none=True)
    except Exception as exc:  # noqa: BLE001
        sys.exit(f"fldigi XML-RPC unreachable: {exc}")


def text_modes(p):
    names = p.modem.get_names()
    names = names[0] if isinstance(names, tuple) else names
    return [n for n in names if n not in NOT_TEXT and n != "NULL"]


def syllables(mode):
    """(tones, bandwidth) for our decoder, from fldigi's mode name.

    Only Olivia is decodable today — the engine we vendor is Jalocha's MFSK
    receiver, and we have written no other modem against it yet. The rest are
    recorded with `decoder: null` so the manifest says plainly which modes we can
    read and which we cannot, which is the whole point of the exercise.
    """
    m = re.match(r"OLIVIA-(\d+)/(\d+)K?$", mode)
    if not m:
        return None
    tones = int(m.group(1))
    bw = m.group(2)
    bw = int(bw) * (1000 if mode.endswith("K") else 1) if len(bw) == 1 else int(bw)
    return tones, bw


def tx_window(p, text, fallback):
    """Seconds needed to send `text` on the mode fldigi is currently in.

    fldigi reports a per-character time over XML-RPC, and it is right for most
    modes — 0.384 s for BPSK31, 0.448 for Olivia 16/500, 0.114 for Feld Hell. It
    is nonsense for a few (Olivia 4/125 reports 0.0, THOR4 a small negative), so
    it is only trusted inside a plausible range and the caller's fallback covers
    the rest. A fixed window was the first version and it silently truncated every
    slow mode: twenty seconds of Olivia 4/125 is twenty characters.
    """
    secs = 0.0
    try:
        rates = p.main.get_char_rates()
        rates = rates[0] if isinstance(rates, tuple) else rates
        secs = abs(float(rates[0]))
    except Exception:  # noqa: BLE001
        secs = 0.0
    if not (0.02 <= secs <= 2.0):
        secs = fallback
    return min(300.0, max(12.0, secs * len(text) * 1.3 + 6.0))


def record(hz, rate, seconds, gain, path):
    if os.path.exists(path):
        os.remove(path)
    return subprocess.Popen(
        [RSPCAP, str(hz), str(rate), str(seconds), path, str(gain)],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def audio_from_iq(path, rate, bandwidth):
    """USB audio, band-limited to the mode and resampled to 8 kHz.

    The centre is measured — the strongest block of the mode's own width — not
    assumed, because where the signal lands depends on the rig's audio path and
    guessing it wrong is a silent failure.
    """
    x = np.fromfile(path, dtype=np.complex64)
    if x.size == 0:
        return None, None, 0.0
    a = x.real.astype(np.float64)
    a -= a.mean()
    spec = np.abs(np.fft.rfft(a * np.hanning(a.size))) ** 2
    f = np.fft.rfftfreq(a.size, 1.0 / rate)
    win = max(1, int(bandwidth * 1.5 / (f[1] - f[0])))
    energy = np.convolve(spec, np.ones(win) / win, "same")
    centre = float(f[int(np.argmax(energy))])

    keep = (f >= centre - bandwidth) & (f <= centre + bandwidth)
    b = np.fft.irfft(np.where(keep, np.fft.rfft(a), 0), n=a.size)
    out = int(round(b.size * 8000.0 / rate))
    X = np.fft.rfft(b)
    Y = np.zeros(out // 2 + 1, dtype=complex)
    k = min(len(Y), len(X))
    Y[:k] = X[:k]
    wav = np.fft.irfft(Y, n=out)
    peak = np.abs(wav).max()
    rms = float(wav.std())
    if peak > 0:
        wav = wav / peak * 0.7
    return centre, wav, 20 * np.log10(max(rms, 1e-9))


def sweep_one(p, mode, args, index, total):
    slug = mode.replace("/", "-").replace(" ", "")
    wavpath = os.path.join(args.out, f"{slug}.wav")
    iqpath = os.path.join("/tmp", f"sweep_{slug}.bin")
    entry = {"mode": mode, "index": index, "of": total}

    reason = NOT_TEXT.get(mode)
    if reason:
        entry.update(skipped=reason)
        return entry

    try:
        p.modem.set_by_name(mode)
    except Exception as exc:  # noqa: BLE001
        entry.update(error=f"set_by_name: {exc}")
        return entry
    time.sleep(0.4)
    entry["fldigi_mode"] = p.modem.get_name()
    if entry["fldigi_mode"] != mode:
        # fldigi refused the mode or fell back — worth knowing, not worth guessing.
        entry["note"] = f"fldigi reported {entry['fldigi_mode']}"

    tx_seconds = tx_window(p, args.text, args.tx_seconds)
    entry["tx_seconds"] = round(tx_seconds, 1)
    capture = tx_seconds + 10.0
    rec = record(args.hz, args.rate, capture, args.gain, iqpath)
    time.sleep(1.5)
    try:
        p.main.rx()
        p.text.clear_tx()
        p.text.add_tx(args.text)
        p.main.tx()
        time.sleep(tx_seconds)
    finally:
        # fldigi never leaves TX by itself — Olivia's idle tones run for ever and
        # that is what left the carrier up for ninety seconds on 2026-10-20.
        try:
            p.main.rx()
        except Exception:  # noqa: BLE001
            pass
    try:
        rec.wait(timeout=capture + 30)
    except subprocess.TimeoutExpired:
        rec.kill()
        entry.update(error="the recorder did not finish")
        return entry

    parsed = syllables(mode)
    entry["tones"], entry["bandwidth"] = parsed if parsed else (None, None)
    bw_for_band = parsed[1] if parsed else 500
    centre, wav, level = audio_from_iq(iqpath, args.rate, bw_for_band)
    os.remove(iqpath) if os.path.exists(iqpath) else None
    if wav is None:
        entry.update(error="empty capture - is the RSP1 free?")
        return entry

    with wave.open(wavpath, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(8000)
        w.writeframes(np.clip(wav * 32767, -32768, 32767).astype("<i2").tobytes())
    entry.update(wav=os.path.basename(wavpath), centre_hz=round(centre), level_dbfs=round(level, 1))

    # Silence means nothing went out: the capture is flat, and the manifest
    # should say so rather than record a file that proves nothing.
    if level < -45.0:
        entry["error"] = f"nothing transmitted (level {level:.0f} dBFS)"
        return entry

    if parsed:
        tones, bw = parsed
        for reverse in (0, 1):
            res = subprocess.run(
                DECODER + [wavpath, str(tones), str(bw), f"{centre:.0f}", str(reverse),
                           *args.text.split()],
                capture_output=True, text=True, timeout=300,
            )
            text = res.stdout.strip()
            if res.returncode == 0:
                entry.update(decoder="olivia", reverse=reverse, decoded=text[:200], passed=True)
                return entry
            entry.setdefault("attempts", []).append({"reverse": reverse, "decoded": text[:120]})
        entry.update(decoder="olivia", passed=False)
        return entry

    entry.update(decoder=None, passed=None, note="no decoder for this mode yet")
    return entry


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--only", default=None, help="regex over the mode name")
    ap.add_argument("--all", action="store_true", help="every mode, skips included")
    ap.add_argument("--limit", type=int, default=0)
    ap.add_argument("--resume", action="store_true", help="skip modes already in the manifest")
    ap.add_argument("--text", default=TEXT)
    ap.add_argument("--hz", type=float, default=27_415_000.0)
    ap.add_argument("--rate", type=int, default=250_000)
    ap.add_argument("--gain", type=int, default=55)
    ap.add_argument(
        "--tx-seconds",
        type=float,
        default=45.0,
        help="fallback seconds per character when fldigi's own rate is nonsense",
    )
    ap.add_argument("--out", default=OUT)
    ap.add_argument("--manifest", default=os.path.join(OUT, "manifest.jsonl"))
    args = ap.parse_args()

    os.makedirs(args.out, exist_ok=True)
    p = proxy()
    modes = text_modes(p)
    if args.only:
        rx = re.compile(args.only)
        modes = [m for m in modes if rx.search(m)]
    if args.resume and os.path.exists(args.manifest):
        seen = set()
        with open(args.manifest) as f:
            for line in f:
                try:
                    seen.add(json.loads(line)["mode"])
                except Exception:  # noqa: BLE001
                    pass
        modes = [m for m in modes if m not in seen]
    if args.limit:
        modes = modes[: args.limit]
    if not modes:
        sys.exit("no modes matched")

    print(f"{len(modes)} modes, {args.tx_seconds:.0f}s each, text {args.text!r}", flush=True)
    done = 0
    with open(args.manifest, "a") as man:
        for i, mode in enumerate(modes, 1):
            print(f"[{i}/{len(modes)}] {mode} … ", end="", flush=True)
            try:
                entry = sweep_one(p, mode, args, i, len(modes))
            except Exception as exc:  # noqa: BLE001 - one mode must not cost the batch
                entry = {"mode": mode, "error": f"{type(exc).__name__}: {exc}"}
            man.write(json.dumps(entry) + "\n")
            man.flush()
            if entry.get("skipped"):
                print(f"skipped ({entry['skipped']})", flush=True)
            elif entry.get("passed") is True:
                print(f"PASS {entry.get('decoded','')!r}", flush=True)
                done += 1
            elif entry.get("error"):
                print(f"ERROR {entry['error']}", flush=True)
            else:
                print(f"recorded, level {entry.get('level_dbfs')} dBFS", flush=True)
    print(f"\ndone: {done} decoded, manifest {args.manifest}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
