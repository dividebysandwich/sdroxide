#!/usr/bin/env python3
"""Generate the NR2 gain-table blob from WDSP's `calculus.c`.

WDSP's NR2 (`emnr.c`, gain method 2) does not evaluate its gain rule at run
time: it looks the answer up in two 241x241 tables, `GG` and `GGS`, generated
offline by Warren Pratt and shipped as ~3 MB of C source. The estimator's
closed form is not in WDSP — only the sampled result — so a port either copies
the tables or invents a different gain rule, and a different gain rule is a
different denoiser. We copy them, transcoded to little-endian `f32`.

    python3 tools/gen_nr2_tables.py ~/Development/wdsp

`f32` rather than the source's `f64` because the round-trip error is 5e-8,
five orders of magnitude below the ~0.4 % the 0.25 dB grid and its bilinear
interpolation cost anyway. Uncompressed because deflate only reaches 0.74 here
(smooth float surfaces), and 117 KiB does not buy a new dependency in
`sdroxide-dsp` the way JS8's 12x did in `sdroxide-digi`.

This is a transcoder and nothing more: the values are upstream's, unedited, so
the blob can be checked against WDSP byte for byte. That includes a known
artefact in the low-xi corner — see the note in `nr2.rs`.

Both tables are indexed `[241 * n_xi + n_gamma]`, each axis running -30 dB to
+30 dB in 0.25 dB steps (WDSP's `getKey`: `10*log10(v/0.001)`, 4 steps per dB).

Blob layout, all little-endian:

    magic  "NR2T"          4 bytes
    grid   u32             points per axis (241)
    GG     grid^2 x f32    gain table, xi-major
    GGS    grid^2 x f32    speech-presence table, xi-major
"""

import pathlib
import re
import struct
import sys

MAGIC = b"NR2T"
GRID = 241


def table(text: str, name: str) -> list[float]:
    """The named `double name[...] = { ... };` array from calculus.c."""
    m = re.search(r"double\s+%s\s*\[[^\]]*\]\s*=\s*\{(.*?)\}\s*;" % name, text, re.S)
    if not m:
        raise SystemExit(f"no `{name}` array in calculus.c — has WDSP changed shape?")
    vals = [float(v) for v in m.group(1).replace("\n", " ").split(",") if v.strip()]
    if len(vals) != GRID * GRID:
        raise SystemExit(f"`{name}` has {len(vals)} entries, expected {GRID * GRID}")
    return vals


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit(f"usage: {sys.argv[0]} <path-to-wdsp-checkout>")
    src = pathlib.Path(sys.argv[1]).expanduser()

    text = (src / "calculus.c").read_text(encoding="latin-1")
    gg, ggs = table(text, "GG"), table(text, "GGS")

    # The gain is an amplitude multiplier and the other is a probability; both
    # bounds are what the run-time code assumes, so check them here rather than
    # discovering a mangled parse as a strange noise floor on the air.
    if not all(v >= 0.0 for v in gg):
        raise SystemExit("GG has negative entries — a gain cannot be negative")
    if not all(0.0 < v <= 1.0 for v in ggs):
        raise SystemExit("GGS is not confined to (0, 1] — it is a probability")

    blob = MAGIC + struct.pack("<I", GRID)
    blob += struct.pack("<%df" % len(gg), *gg)
    blob += struct.pack("<%df" % len(ggs), *ggs)

    out = pathlib.Path(__file__).resolve().parent.parent / (
        "crates/sdroxide-dsp/src/nr2_tables.bin"
    )
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_bytes(blob)
    zeros = sum(1 for v in gg if v == 0.0)
    print(
        f"wrote {out}: {GRID}x{GRID} GG + GGS, {len(blob)} bytes "
        f"({len(blob) / 1024:.0f} KiB); GG max {max(gg):.4f}, {zeros} underflowed to zero"
    )


if __name__ == "__main__":
    main()
