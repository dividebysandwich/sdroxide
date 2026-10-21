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
# For a mode too slow to carry the whole thing: a complete message rather than a
# prefix of a longer one, so every sample has a known start and a known end.
SHORT_TEXT = " AAA SDRBROWN TEST "
# Longest transmission worth making for one sample. Past this, the text is
# shortened instead — a three-minute key-down to prove a point is not worth it.
MAX_WINDOW_S = 240.0

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


def restart_fldigi(timeout=40.0):
    """Kill and relaunch fldigi, because a modal dialog steals its main loop.

    fldigi puts up a dialog on a timer — the operator has watched it appear when
    he leaves the desk — and while it is up, XML-RPC answers nothing: every call
    times out and the sweep records a run of failures that look like radio
    faults. Whatever the dialog is, restarting is the recovery that needs no
    knowledge of it.
    """
    print("    fldigi is not answering - restarting it", flush=True)
    subprocess.run(["pkill", "-x", "fldigi"], check=False)
    for _ in range(20):
        if subprocess.run(["pgrep", "-x", "fldigi"], capture_output=True).returncode != 0:
            break
        time.sleep(0.5)
    subprocess.Popen(
        ["setsid", "fldigi"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        start_new_session=True,
    )
    deadline = time.time() + timeout
    while time.time() < deadline:
        time.sleep(1.5)
        try:
            p = proxy()
            p.modem.get_name()
            time.sleep(1.0)
            return p
        except Exception:  # noqa: BLE001
            continue
    return None


def answering(p, wait=3.0):
    """True when fldigi answers XML-RPC inside `wait` seconds."""
    try:
        p.modem.get_name()
        return True
    except Exception:  # noqa: BLE001
        return False


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
    # A tight margin, deliberately. fldigi fills every spare second with idle
    # characters, so a generous window is a keyed carrier with nothing in it -
    # which is what the operator sees as "the message has gone but it still
    # transmits". Enough to cover the text and a little jitter, no more.
    #
    # Returns the per-character time as well, because the sample is judged
    # against the *text's* own duration and not against this window's margin.
    # A truncated sample is worse than a long one, so the margin is a safety
    # factor rather than a tight fit.
    return min(MAX_WINDOW_S, max(12.0, secs * len(text) * 1.5 + 5.0)), secs


def entry_is_tx(p):
    """True while fldigi says it is transmitting. Aborting needs to know."""
    try:
        return p.main.get_trx_state() != "RX"
    except Exception:  # noqa: BLE001
        return False


def record(hz, rate, seconds, gain, path):
    if os.path.exists(path):
        os.remove(path)
    # stderr goes to a log rather than /dev/null: "the RSP1 was busy" was
    # invisible for a whole run because it was thrown away here.
    log = open(path + ".log", "wb")
    return subprocess.Popen(
        [RSPCAP, str(hz), str(rate), str(seconds), path, str(gain)],
        stdout=log,
        stderr=log,
    )


def burst_seconds(a, rate):
    """How long the carrier was up, from the envelope.

    Every sample is judged by this: a burst much shorter than the text needs was
    truncated, and one much longer is fldigi idling after the text — which is the
    "message has gone but it still transmits" the operator saw.
    """
    win = max(1, int(rate * 0.05))
    n = a.size // win
    if n == 0:
        return 0.0
    lev = np.abs(a[: n * win].reshape(n, win)).mean(axis=1)
    if lev.max() <= 0:
        return 0.0
    up = np.where(lev > lev.max() * 0.2)[0]
    if up.size == 0:
        return 0.0
    return float((up[-1] - up[0] + 1) * win / rate)


def audio_from_iq(path, rate, bandwidth):
    """USB audio, band-limited to the mode and resampled to 8 kHz.

    The centre is measured — the strongest block of the mode's own width — not
    assumed, because where the signal lands depends on the rig's audio path and
    guessing it wrong is a silent failure.
    """
    if not os.path.exists(path):
        log = path + ".log"
        why = open(log).read().strip() if os.path.exists(log) else "no log"
        raise RuntimeError(f"the recorder wrote nothing: {why[-300:]}")
    x = np.fromfile(path, dtype=np.complex64)
    if x.size == 0:
        raise RuntimeError("the recorder wrote an empty file - is the RSP1 free?")
    a = x.real.astype(np.float64)
    a -= a.mean()
    spec = np.abs(np.fft.rfft(a * np.hanning(a.size))) ** 2
    f = np.fft.rfftfreq(a.size, 1.0 / rate)

    # Two candidates, because "where the signal is" means two different things.
    # A wide mode is a block, and its centre is the middle of the strongest block
    # of its own width. A narrow one — PSK31 is thirty hertz — has no block to
    # find, and the strongest block of five hundred hertz was band noise a
    # kilohertz away, which is exactly why the first BPSK31 sample did not
    # decode. Both are recorded and the verifier tries each.
    peak_hz = float(f[int(np.argmax(spec))])
    win = max(1, int(bandwidth * 1.5 / (f[1] - f[0])))
    energy = np.convolve(spec, np.ones(win) / win, "same")
    block_hz = float(f[int(np.argmax(energy))])

    # The audio is cut generously around the peak so that whichever candidate a
    # decoder is given, the samples it needs are in the file.
    centre = peak_hz
    keep = (f >= centre - 2500) & (f <= centre + 2500)
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
    return peak_hz, block_hz, wav, 20 * np.log10(max(rms, 1e-9)), burst_seconds(a, rate)


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

    # A long interleave costs far more than the reported rate says: MFSK64L is
    # named for being fast and is one of the slowest things fldigi sends. Its own
    # rate said 0.52 s/char, the window came out at 40 s, and the operator saw a
    # few characters on the air. A trailing L, and the modes that are slow by
    # nature, are treated as slow whatever the rate claims.
    long_interleave = bool(re.search(r"L$", mode)) or mode.startswith(("OLIVIA", "Cont"))
    fallback = args.secs_per_char * (3.0 if long_interleave else 1.0)
    tx_seconds, secs_per_char = tx_window(p, args.text, fallback)
    text = args.text
    if long_interleave or tx_seconds >= MAX_WINDOW_S - 1.0:
        # Too slow for the long message: fall back to the short one, which fits
        # in every mode fldigi has.
        text = getattr(args, "short_text", None) or SHORT_TEXT
        tx_seconds, secs_per_char = tx_window(p, text, args.secs_per_char)
        entry["shortened"] = True
    entry["text"] = text
    entry["tx_seconds"] = round(tx_seconds, 1)
    capture = tx_seconds + 10.0
    rec = record(args.hz, args.rate, capture, args.gain, iqpath)
    time.sleep(1.5)
    try:
        p.main.rx()
        p.text.clear_tx()
        p.text.add_tx(text)
        p.main.tx()
        time.sleep(tx_seconds)
    finally:
        # fldigi never leaves TX by itself — Olivia's idle tones run for ever and
        # that is what left the carrier up for ninety seconds on 2026-10-20.
        for _ in range(5):
            try:
                p.main.rx()
                if p.main.get_trx_state() == "RX":
                    break
            except Exception:  # noqa: BLE001
                pass
            time.sleep(0.6)
        if entry_is_tx(p):
            try:
                p.main.abort()
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
    peak_hz, block_hz, wav, level, burst = audio_from_iq(iqpath, args.rate, bw_for_band)
    os.remove(iqpath) if os.path.exists(iqpath) else None
    if wav is None:
        entry.update(error="empty capture - is the RSP1 free?")
        return entry

    with wave.open(wavpath, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(8000)
        w.writeframes(np.clip(wav * 32767, -32768, 32767).astype("<i2").tobytes())
    expected = secs_per_char * len(text)
    if not parsed:
        # No decoder, so nothing has confirmed the text went out. The rate that
        # sized the window is the same number that would judge it, and a verdict
        # from that is circular - which is exactly how MFSK64L came out "ok" with
        # a few characters actually sent.
        verdict = "unverified"
    elif burst < expected * 0.9:
        verdict = "TRUNCATED"
    elif burst > expected * 1.8:
        verdict = "long tail"
    else:
        verdict = "ok"
    entry.update(
        wav=os.path.basename(wavpath),
        peak_hz=round(peak_hz),
        block_hz=round(block_hz),
        centre_hz=round(block_hz),
        level_dbfs=round(level, 1),
        burst_seconds=round(burst, 1),
        expected_seconds=round(expected, 1),
        verdict=verdict,
    )

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
                           *text.split()],
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
    ap.add_argument("--short-text", default=SHORT_TEXT, help="used when a mode is too slow")
    ap.add_argument("--hz", type=float, default=27_415_000.0)
    ap.add_argument("--rate", type=int, default=250_000)
    ap.add_argument("--gain", type=int, default=55)
    ap.add_argument(
        "--secs-per-char",
        type=float,
        default=0.5,
        help="fallback seconds per character when fldigi's reported rate is nonsense",
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

    print(f"{len(modes)} modes, windows from each mode's rate, text {args.text!r}", flush=True)
    done = 0
    with open(args.manifest, "a") as man:
        for i, mode in enumerate(modes, 1):
            print(f"[{i}/{len(modes)}] {mode} … ", end="", flush=True)
            try:
                if not answering(p):
                    fresh = restart_fldigi()
                    if fresh is not None:
                        p = fresh
                entry = sweep_one(p, mode, args, i, len(modes))
            except Exception as exc:  # noqa: BLE001 - one mode must not cost the batch
                # An XML-RPC timeout means a dialog, not a radio fault: restart
                # fldigi and give the mode one more go before recording a failure.
                fresh = restart_fldigi()
                if fresh is not None and "timed out" in str(exc).lower() or fresh is not None:
                    p = fresh
                    try:
                        entry = sweep_one(p, mode, args, i, len(modes))
                    except Exception as again:  # noqa: BLE001
                        entry = {"mode": mode, "error": f"{type(again).__name__}: {again}"}
                else:
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
