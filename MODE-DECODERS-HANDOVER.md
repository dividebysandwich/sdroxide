# Mode decoders — where they stand, and the plan

**Local only. Not pushed, not merged.** Written 2026-10-19 while `main` is frozen
for F4NRM. Every branch named here is to be pushed **one mode at a time**, so he
can test each on its own.

Read this before touching Olivia, THOR, Hell, DSC, NAVTEX or WEFAX, and before
starting MFSK or STANAG-4285. It carries the measurements, the two traps that
made the first sweep lie, and the licence boundary — which for one of the two new
modes is the whole answer.

---

## 1. The instrument

`/tmp/opencode/mpsk/harness/` — a reader harness in its own workspace, path-
depending on the tree. **Not in the repository**, deliberately: it is a measuring
instrument, and the repository is frozen.

```
mpsk-harness <file.wav> <wefax|olivia|sstv|digi|navtex|dsc|hell> [options]
  --af N      the AF centre; default read out of the file name
  --sweep     walk the geometry and score each result
  --mode N    the digi family (PSK|RTTY|RTTYFM|THOR|OLIVIA|FSQ)
  --baud/--shift/--thor/--tones/--bw   the family's variant
  --variant N Hell variant
  --out F     where to write the picture
```

It takes any WAV, any rate, mono or stereo, 8- or 16-bit. `--sweep` walks the
geometry and prints a **score** for each result: the mean correlation between
adjacent rows of the decoded picture. That is the number that separates a chart
from noise — a chart's neighbouring lines are alike (0.5–0.9), noise's are not.
**A run that produces more lines proves nothing**; a run that produces structure
does.

`/tmp/opencode/mpsk/sweep.py` drives a whole directory of recordings through it.

**The corpus**: the MultiPSK sound-file collection, 88 files in
`~/Downloads/sounds`, one per mode, published by F6CTE as training material. Every
file is named with its **AF centre** (`BPSK31-1000_Hz`, `FAX_Hambourg-2250_Hz`) —
and a decoder listening somewhere else hears noise, which is why the harness reads
the name.

## 2. The two traps that made the first sweep lie

Both are worth reading before the next sweep, because both produced a *false*
"everything fails" result and both cost an hour.

1. **`DigiStatus::transcript` is not the decoded text.** It is the QSO exchange.
   The received text is **`text_rx`**, which is where the panel reads it from.
   Reading `transcript` made every keyboard mode in the tree answer `""`.
2. **Each family's variant is a config field, and the default is only right for
   the file named for it.** `rtty_baud` (45.45), `rtty_shift_hz` (170),
   `thor_mode` (Thor16), `olivia_tones`/`olivia_bw_hz` (32/1000),
   `hell_variant` (Feld). A sweep that does not set them is measuring the
   **defaults**, not the decoder — which is why BPSK31 and RTTY45 passed while
   their siblings "failed".

**And the reason this corpus matters at all:** our round-trip tests feed each
decoder its own encoder's output, so they pass on modes that cannot read anyone
else's signal. This is the first time any of these decoders has been pointed at a
signal it did not generate.

## 3. Where the corpus says we stand

| | files | |
|---|---|---|
| **Decoded** | 3 | BPSK31, RTTY45, SSTV Martin 1 |
| **We have the mode, it fails** | 9 | Olivia, THOR11, Feld Hell, Hell 80, FM Hell 245, PSK Hell, GMDSS (DSC), SITOR-A (NAVTEX), WEFAX |
| **We have the mode, no harness path** | ~20 | CW, FT8, FT4, JT65, WSPR, HFDL, VDL2, APRS/packet, ALE ×2, SYNOP, QPSK31/63/125/250, BPSK63/125/250, RTTY50/75, SSTV Robot 24 B/W, RS_ID, CALL_ID, RDS |
| **No such mode** | ~50 | MFSK, MT63, DominoEX/F, Contestia, RTTYM, THROB/X, PAX/PAX2, Coquelet, Voice, Lentus, CHIP64 · STANAG-4285, 110A · D-STAR, C4FM, DMR, P25 · POCSAG, SELCAL, DTMF, DGPS, BIIS, DFM, 1382, IEC-870-5, NWR/SAME, ORBCOMM, EGC, M10, LMS6, IMET4, RS41, EPIRB, Extended Message, CCW, QRSS, NDB, PACTOR |

**The nine failures are not scattered.** Five of the six modes — Olivia, THOR and
all four Hell variants — are the ones whose decode depends on **precise symbol or
column timing recovery**. The three that pass have neither: BPSK31 and RTTY45 use
a plain bit-clock PLL, SSTV is frame-synced. So the fault class is the clock, not
the demodulation and not the character coding — BPSK31 and RTTY45 prove those
work.

## 4. The timing-recovery class

### Olivia — diagnosed, not fixed

Instrumented locally (the patch was reverted byte-for-byte). **It locks** —
`locked=true` from `scount=192` — and then:

```
scount=192  tphase=13  ratio=1.68  locked=true
scount=384  tphase=15
scount=512  tphase=0     ← jumped
scount=640  tphase=11    ← jumped again
```

`tphase`, the **symbol sampling instant**, wanders instead of settling, and
`sync` is a broad two-humped wave rather than a sharp peak, so its argmax is
unstable. `best_phase` — the block phase — drifts too (62 → 52). Both wander
together, which says the **symbol/block clock is not tracking**. Pinning `tphase`
once it has settled does **not** fix the output, so the wander is a symptom and
the clock recovery is the fault.

Measured about the file itself, so the geometry is not in doubt: energy spans
539–1490 Hz with a centroid at 984 Hz (32 tones, 1000 Hz ✓), and the envelope
autocorrelates at **32.00 ms = 31.250 baud** ✓ — exactly what `spacing = bw/tones`
and `sps = rate/spacing` compute. **Our formula is right**; the clock is not.

Reference: `https://raw.githubusercontent.com/w1hkj/fldigi/master/src/include/jalocha/pj_mfsk.h`
— Jalocha's own header, which this fork's Olivia work already cites.

### Hell — the convention is split, so the fix is detection

Our font art is **14 genuine rows** (the `!` glyph has a real gap at row 8; it is
not a doubled 7-row glyph). fldigi's `src/feld/Feld7x7-14.cxx` **is** 7 rows
doubled — every entry repeated. F6CTE's Hell 80 spec says a third thing:
*"each column is displayed vertically twice (but **transmitted once**)"*.

So the row/dot convention differs between implementations, and MultiPSK appears
to be the one that transmits 7 where we sample 14 — which produces exactly the
row-alternating bars we see. **The fix is therefore to detect the dot rate, not
to flip a constant.** That was tried first and it cannot work: `hell_font`'s art
asserts its own height, so `ROWS = 7` does not even compile.

### THOR — no diagnosis yet

Garbage with `ThorMode::Thor11` set, so it is not the variant. **Diagnose before
fixing** — the same instrumentation approach as Olivia (print the symbol clock
and the confidence per block) is the first move.

## 4b. The PSK family — diagnosed, and it is the foundation for 4285

Read from the code, not inferred: **`crates/sdroxide-dsp/src/psk.rs:21` is
`const BAUD: f64 = 31.25;`** — a compile-time constant, with `SPS = 16` and
`DEC_RATE = BAUD * SPS` (500 Hz) derived from it, and four places that read it
(`:173` the TX samples-per-symbol, `:515` the RX decimation). And the detector is
**differential BPSK only**: `BpskCore::push`'s own comment says *"Decision-directed
BPSK phase error"* and *"Differential BPSK detection (immune to the Costas ±
ambiguity)"*.

That explains the corpus exactly. **BPSK31 works because it is the one file whose
speed is the constant.** BPSK63, BPSK125 and BPSK250 are 2×, 4× and 8× that
symbol rate, and all four QPSK files are a constellation the detector has no
branch for. Nothing else is wrong with them — the baud and the constellation are
both simply not expressible.

The two are independent axes, which is the part worth keeping:

| | axis | what fails |
|---|---|---|
| BPSK31 → 63/125/250 | **baud** | 31.25 → 62.5 / 125 / 250 |
| BPSK → QPSK | **constellation** | 1 bit/symbol → 2 bits/symbol, phases at π/4 spacing |

And PSK is usually **differentially encoded** — the data is in the phase *change*,
not an absolute phase — which our BPSK31 path already does correctly, so that half
is proven and does not need touching.

### Why this is first: it is also the front end for STANAG-4285

STANAG-4285 is **2400 baud always**, with the constellation chosen by the data
rate, and BPSK, QPSK *and* 8-PSK all appear:

| data rate | constellation | FEC |
|---|---|---|
| 2400 bps | 8-PSK | 2/3 |
| 1200 bps | QPSK | 1/2 |
| 600 bps | BPSK | 1/2 |
| 300 bps | BPSK | 1/4 |
| 150 bps | BPSK | 1/8 |
| 75 bps | BPSK | 1/16 |
| 3600 / 2400 / 1200 uncoded | 8-PSK / QPSK / BPSK | none |

So a differential PSK demodulator with a **parameterised symbol rate and
constellation** is three things at once: the fix for seven corpus files, the front
end of 4285, and most of MIL-STD-188-110A. **That is why it leads the plan** — and
it is small: one constant becomes a parameter, and the detector gains a branch per
constellation.

## 5. The plan, one branch per mode

Each is self-contained and is to be pushed on its own so F4NRM can test it alone.
**No worktrees** — a worktree costs a whole `target/`, which is 110 GB here; a
branch costs a ref plus the objects for its own diff, kilobytes.

| branch | what | proof |
|---|---|---|
| `fork/psk-variants` | symbol rate and constellation as parameters — **the foundation** | 7 corpus files (BPSK63/125/250, QPSK31/63/125/250) + our round trip |
| `fork/mfsk` | **new mode** — MFSK8/16/32/64 | the corpus (MFSK8, MFSK16 files) + our round trip |
| `fork/olivia-timing` | fix the symbol/block clock | the corpus **and** Kevin's capture |
| `fork/thor-timing` | diagnose first, then fix | the corpus |
| `fork/hell-dot-rate` | detect the dot rate; keep both conventions | the corpus (4 Hell files) + our round trip |
| `fork/dsc-navtex` | DSC and NAVTEX on the audio path | the corpus (GMDSS, SITOR-A) |
| `fork/stanag-4285` | **parked on the licence question** — see §6 | — |

### MFSK, the clean new mode

fldigi implements the whole family: `src/mfsk/` — `mfsk.cxx`,
`mfskvaricode.cxx`, `interleave.cxx`, `mfsk-pic.cxx`. **fldigi is GPL-3 and this
fork is GPL-3**, so it is a licence-compatible reference we may work from with
attribution.

It is also a *popular* mode — **Shortwave Radiogram transmits MFSK32 weekly** — and
the corpus has MFSK8 and MFSK16 to test against.

Shape: the same as THOR, which is the precedent to copy. A modem in
`sdroxide-dsp`, a `Mode::Mfsk`, and the speed (8/16/32/64) as a **`DigiConfig`
field** exactly as `thor_mode` is — appended last, which is a wire change and
takes the next `PROTO_VERSION` and the `VERSION_BYTE` with it.

### The untested twenty are mostly cheap

Most need only a harness arm, not a decoder change: CW, FT8, FT4, JT65, WSPR,
HFDL, VDL2, APRS, ALE, SYNOP, RTTY50/75, SSTV Robot 24 B/W, RS_ID, CALL_ID, RDS.
Worth doing **before** any decoder work, because each one either confirms a mode
works or turns "untested" into a real finding. (The PSK speeds are not in this
group — §4b has them, and they are a decoder change.)

## 6. The licence boundary, stated once

- **fldigi — GPL-3.** Compatible with this fork. Usable as a reference, with
  attribution. This is what makes MFSK, and the Olivia and THOR fixes, possible.
- **MultiPSK — closed freeware, 30 €.** **Reference and test material only.**
  Never code, never a dependency. **Do not dissect the binary** — the same rule
  the ATS Mini RadioScript note sets out: do not publish a dissection of
  somebody's binary and do not treat having read one as a licence to write
  against it. Measuring a published *recording* is a different thing and is what
  §1–§3 do.
- **STANAG-4285 — the modulation is public, the payload detail is not.**
  The waveform is documented well enough to build the front end: **a single 1800 Hz
  tone, 2400 symbols/s always, USB, ~2.75 kHz wide**, with the constellation
  chosen by the data rate (§4b). Hugh Stegman NV6H's *"Demystifying STANAG 4285"*
  (Monitoring Times, July 2009, p.28) adds the operational facts that matter more
  than the theory: it **does not autobaud**, so it must be told the rate, the
  interleave and the framing; and *"nearly everything that we stand any chance of
  decoding will be 300 or 600 baud, long interleave… usually ITA2, same as used in
  RTTY"*. The one parameter that ties the two together is the **ACF of 106.66 ms**
  (sigidwiki), which is a synchronisation measurement, not a data one.

  **The wall is not the modulation, it is the payload detail** — the exact sync
  preamble, scrambler, convolutional code and the interleaver matrices. Those live
  in the standard (NATO/DLA, not public) and in the one open implementation, which
  is **C.H. Brain G4GUO's** in the KiwiSDR tree at `extensions/s4285/`
  (`s4285.cpp`, `s4285control.cpp`, `s4285convolutional.cpp`, ~1,400 lines). Its
  own licence file reads, in full: `// (License TBD)` — **no rights granted**, so
  it may not be copied, ported or reimplemented from. `Rhizomatica/open5066` is
  GPL-2.0 and compatible, but it is **STANAG 5066, the data-link layer** — it
  interfaces to a modem rather than containing one.

  **So the split is clean, and it is what makes a plan possible:** the PSK front
  end is ours to build from the published description (§4b), and the FEC and
  interleaving are the part that needs either a licence or air work. The three
  honest routes are **ask G4GUO or the KiwiSDR maintainer for a licence** — the
  same move the notes prescribe for Sigidwiki's content — **implement the front
  end and the uncoded modes first** — 1200/2400/3600 bps carry no FEC and no
  interleave, so they need the preamble and the scrambler but not the code or the
  interleaver matrices, which is the smaller half of the wall — or **implement
  from the public description and verify on the air.** That is the
  operator's call and it should be made before anyone spends a week on it.

  Worth knowing while deciding: sigidwiki lists MultiPSK, **Sigmira** and
  **Sorcerer** as the hobby decoders, and Sigmira is the one Stegman singles out
  because *"the parameters can be changed so quickly"* — i.e. it exposes the same
  rate/interleave/framing knobs the mode needs.
- **The recordings stay local fixtures.** Nothing from the collection goes into
  the tree, and none of it is redistributed.

## 7. What is not proven, and what is deliberately not done

- **Nothing has been adapted.** Every experiment was reverted byte-for-byte and
  `main` is clean. The only artefact is the harness in `/tmp`.
- **The nine failures are one program's recordings.** Two of the three timing
  modes decode *other* signals we know work (Olivia on Kevin's capture and the
  Wikipedia sample; Hell's geometry follows fldigi), so a change made to suit
  MultiPSK could break what works. **Kevin's capture is not on this box and is
  needed before `olivia.rs` is touched.**
- **WEFAX is waiting on a comparison, not a fix.** Ten geometries (5 LPM × 2 IOC,
  automatic and manual start) score at most 0.043 — no structure anywhere — and
  the measured signal is a ~1000 Hz shift (1400↔2400) where WEFAX is 800
  (1500/2300), with a steady ~2242 Hz carrier at points. The one-step experiment
  that settles it is F4NRM putting `FAX_Hambourg-2250_Hz.WAV` through MultiPSK: if
  his decodes it and ours does not, it is a real fault with a fixture attached; if
  his also makes noise of it, the file is the problem.
- **The 4285 mode is parked**, not scheduled, until §6 is answered.

## 8. Housekeeping

- One branch per mode, in this checkout. **No worktrees** — 110 GB each.
- Work one mode at a time: every branch switch recompiles the crates that differ,
  and the shared `target/` keeps every variant's artefacts.
- The harness and the corpus live outside the tree and stay there.
