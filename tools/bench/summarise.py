#!/usr/bin/env python3
"""Turn the sweep's manifest into a table you can read.

The manifest is JSON lines, appended as each mode finishes, so it survives a
sweep that dies. This says what it contains: which modes we recorded, what
parameters each was measured at, and — the only column that really matters —
whether our decoder got the text back.

    ./summarise.py                  # the table
    ./summarise.py --markdown       # also write manifest.md beside the manifest
    ./summarise.py --failed         # only the ones that did not decode
"""

import argparse
import json
import os
import sys

OUT = os.path.join(os.path.expanduser("~"), "Downloads", "sounds", "our-captures", "fldigi")


def load(path):
    rows = []
    if not os.path.exists(path):
        sys.exit(f"no manifest at {path} — run sweep.py first")
    with open(path) as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                rows.append(json.loads(line))
            except json.JSONDecodeError:
                # A half-written last line is normal if a sweep was interrupted.
                continue
    return rows


def state(row):
    if row.get("skipped"):
        return "skipped"
    if row.get("error"):
        return "error"
    if row.get("passed") is True:
        return "PASS"
    if row.get("passed") is False:
        return "fail"
    if row.get("decoded"):
        return "text"
    return "recorded"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--manifest", default=os.path.join(OUT, "manifest.jsonl"))
    ap.add_argument("--failed", action="store_true")
    ap.add_argument("--markdown", action="store_true")
    args = ap.parse_args()

    rows = load(args.manifest)
    if args.failed:
        rows = [r for r in rows if state(r) in ("fail", "error")]

    rows.sort(key=lambda r: r.get("mode", ""))
    width = max((len(r.get("mode", "")) for r in rows), default=8)
    lines = []
    counts = {}
    for r in rows:
        s = state(r)
        counts[s] = counts.get(s, 0) + 1
        parts = [
            f"{r.get('mode',''):{width}}",
            f"{s:8}",
            f"{r.get('tones') or '-':>4}/{r.get('bandwidth') or '-':<5}",
            f"{r.get('centre_hz') or '-':>6}",
            f"{r.get('level_dbfs') or '-':>7}",
        ]
        if s == "PASS":
            parts.append(f"reverse {r.get('reverse')}  {r.get('decoded','')[:60]!r}")
        elif s == "fail":
            parts.append(f"attempts {len(r.get('attempts',[]))}")
        elif r.get("skipped"):
            parts.append(r["skipped"])
        elif r.get("error"):
            parts.append(r["error"])
        lines.append("  ".join(parts))

    body = "\n".join(lines)
    print(body)
    print()
    print("  ".join(f"{k}={v}" for k, v in sorted(counts.items())))

    if args.markdown:
        md = [
            "# fldigi mode sweep",
            "",
            f"{len(rows)} modes recorded. Generated from `manifest.jsonl` — do not hand-edit.",
            "",
            "| mode | state | tones/bw | centre | level | note |",
            "|---|---|---|---|---|---|",
        ]
        for r in rows:
            note = ""
            if r.get("passed") is True:
                note = f"reverse {r.get('reverse')}: `{r.get('decoded','')[:50]}`"
            elif r.get("skipped"):
                note = r["skipped"]
            elif r.get("error"):
                note = r["error"]
            elif r.get("passed") is False:
                note = "no decoder output"
            md.append(
                f"| {r.get('mode','')} | {state(r)} | {r.get('tones') or '-'}/{r.get('bandwidth') or '-'} "
                f"| {r.get('centre_hz') or '-'} | {r.get('level_dbfs') or '-'} | {note} |"
            )
        path = os.path.join(os.path.dirname(args.manifest), "manifest.md")
        with open(path, "w") as f:
            f.write("\n".join(md) + "\n")
        print(f"\nwrote {path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
