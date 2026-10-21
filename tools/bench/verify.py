#!/usr/bin/env python3
"""Run our decoders over the swept corpus and record what they manage.

`sweep.py` records; this says what we can read. It walks `manifest.jsonl`, picks
a decoder from each mode's name, runs it over the saved WAV, and appends the
result to `verify.jsonl` — one line per mode, with the decoded text quoted where
it worked.

    ./verify.py              # everything already swept
    ./verify.py --only PSK   # one family
    ./verify.py --summary    # the table, from verify.jsonl

What it does *not* claim: a mode that fails here has not been proved impossible,
only that the decoder we have today does not read it. Several of fldigi's PSK
variants carry different framing and FEC, and we have none of those implemented;
the value is knowing which is which.
"""

import argparse
import json
import os
import re
import subprocess
import sys

HOME = os.path.expanduser("~")
ROOT = os.path.join(HOME, "sdroxide")
OUT = os.path.join(HOME, "Downloads", "sounds", "our-captures", "fldigi")
DECODE_ANY = os.path.join(ROOT, "target", "release", "examples", "decode_any")
DECODE_OLIVIA = os.path.join(ROOT, "target", "release", "examples", "decode_file")
TEXT = " AAA TEST TEST THE EAGLE HAS LANDED 12345 SDRBROWN ZZZ "
WORDS = "AAA EAGLE LANDED SDRBROWN".split()


def baud_for(mode):
    """The symbol rate fldigi uses for a PSK mode, from its name."""
    m = re.search(r"(\d+)", mode)
    if not m:
        return None
    n = int(m.group(1))
    # BPSK31 is 31.25; the rest are exact multiples of it.
    return {31: 31.25}.get(n, float(n))


def decoder_for(mode):
    """(binary argv prefix, description) for a mode, or None."""
    if mode.startswith("OLIVIA-"):
        m = re.match(r"OLIVIA-(\d+)/(\d+)K?$", mode)
        if not m:
            return None
        tones = int(m.group(1))
        bw = int(m.group(2)) * (1000 if mode.endswith("K") else 1)
        return ("olivia", tones, bw)
    if re.match(r"^(BPSK|QPSK)", mode):
        baud = baud_for(mode)
        if baud is None:
            return None
        return ("psk", baud, "qpsk" if mode.startswith("QPSK") else None)
    if mode == "RTTY":
        return ("rtty", 45.45, 170.0)
    return None


def run(binary, args):
    try:
        res = subprocess.run(binary + args, capture_output=True, text=True, timeout=300)
        return res.stdout.strip()
    except Exception as exc:  # noqa: BLE001
        return f"<{type(exc).__name__}>"


def verify_one(row):
    mode = row["mode"]
    wav = os.path.join(OUT, row.get("wav", ""))
    if not os.path.exists(wav):
        return {"mode": mode, "error": "no wav"}
    centre = int(row.get("centre_hz") or 0)
    pick = decoder_for(mode)
    if pick is None:
        return {"mode": mode, "decoder": None, "note": "no decoder for this mode"}

    if pick[0] == "olivia":
        _, tones, bw = pick
        best = None
        for reverse in (0, 1):
            out = run(DECODE_OLIVIA, [wav, str(tones), str(bw), str(centre), str(reverse)])
            ok = all(w in out for w in WORDS)
            best = {"decoder": "olivia", "reverse": reverse, "decoded": out[:200], "passed": ok}
            if ok:
                return {"mode": mode, **best}
        return {"mode": mode, **best}

    if pick[0] == "psk":
        _, baud, extra = pick
        args = [wav, "psk", str(centre), str(baud)]
        if extra:
            args.append(extra)
        out = run(DECODE_ANY, args)
        return {"mode": mode, "decoder": "psk", "decoded": out[:200],
                "passed": all(w in out for w in WORDS)}

    if pick[0] == "rtty":
        _, baud, shift = pick
        best = None
        for extra in ("", "reverse"):
            args = [wav, "rtty", str(centre), str(baud), str(shift)] + ([extra] if extra else [])
            out = run(DECODE_ANY, args)
            ok = all(w in out for w in WORDS)
            best = {"decoder": "rtty", "reverse": bool(extra), "decoded": out[:200], "passed": ok}
            if ok:
                break
        return {"mode": mode, **best}

    return {"mode": mode, "decoder": None}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--only", default=None)
    ap.add_argument("--manifest", default=os.path.join(OUT, "manifest.jsonl"))
    ap.add_argument("--summary", action="store_true")
    args = ap.parse_args()

    rows = []
    with open(args.manifest) as f:
        for line in f:
            if line.strip():
                rows.append(json.loads(line))
    if args.only:
        rx = re.compile(args.only)
        rows = [r for r in rows if rx.search(r["mode"])]

    if args.summary:
        path = os.path.join(OUT, "verify.jsonl")
        if not os.path.exists(path):
            sys.exit("nothing verified yet")
        res = [json.loads(line) for line in open(path) if line.strip()]
        res.sort(key=lambda r: r["mode"])
        w = max((len(r["mode"]) for r in res), default=8)
        for r in res:
            state = "PASS" if r.get("passed") else ("none" if r.get("decoder") is None else "fail")
            note = ""
            if r.get("passed"):
                note = f"reverse {r.get('reverse')}" if "reverse" in r else ""
            print(f"{r['mode']:{w}}  {state:5}  {note}")
        passed = sum(1 for r in res if r.get("passed"))
        print(f"\n{passed} of {len(res)} verified modes decode")
        return 0

    path = os.path.join(OUT, "verify.jsonl")
    done = 0
    with open(path, "a") as out:
        for r in rows:
            if r.get("skipped") or r.get("error"):
                continue
            line = json.dumps(verify_one(r))
            out.write(line + "\n")
            out.flush()
            state = "PASS" if json.loads(line).get("passed") else "--"
            print(f"{state} {r['mode']}", flush=True)
            done += 1
    print(f"\n{done} modes verified into {path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
