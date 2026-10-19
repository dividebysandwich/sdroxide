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
| **Decoded** | 5 | BPSK31, RTTY45, SSTV Martin 1 — **plus NAVTEX and WEFAX, which §3b clears** |
| **We have the mode, it fails** | 6 | Olivia, THOR11, Feld Hell, Hell 80, FM Hell 245, PSK Hell |
| **Not a decoder fault after all** | 3 | NAVTEX (the corpus file was SITOR-A, a different mode), WEFAX (the corpus file has a non-standard shift), DSC (still open — see §3b) |
| **We have the mode, no harness path** | ~20 | CW, FT8, FT4, JT65, WSPR, HFDL, VDL2, APRS/packet, ALE ×2, SYNOP, QPSK31/63/125/250, BPSK63/125/250, RTTY50/75, SSTV Robot 24 B/W, RS_ID, CALL_ID, RDS |
| **No such mode** | ~50 | MFSK, MT63, DominoEX/F, Contestia, RTTYM, THROB/X, PAX/PAX2, Coquelet, Voice, Lentus, CHIP64 · STANAG-4285, 110A · D-STAR, C4FM, DMR, P25 · POCSAG, SELCAL, DTMF, DGPS, BIIS, DFM, 1382, IEC-870-5, NWR/SAME, ORBCOMM, EGC, M10, LMS6, IMET4, RS41, EPIRB, Extended Message, CCW, QRSS, NDB, PACTOR |

**The nine failures are not scattered.** Five of the six modes — Olivia, THOR and
all four Hell variants — are the ones whose decode depends on **precise symbol or
column timing recovery**. The three that pass have neither: BPSK31 and RTTY45 use
a plain bit-clock PLL, SSTV is frame-synced. So the fault class is the clock, not
the demodulation and not the character coding — BPSK31 and RTTY45 prove those
work.

## 3b. A second corpus: SkySweep Pro's own recordings (2026-10-19)

The operator had an old installer in `~/Downloads/sw` — an **InstallShield 3**
package whose `SETUP.INI` says `AppName=SkySwePro`. It is **SkySweep Pro 5.12**
by SkySweep Technologies, the decoder Hugh Stegman recommends for STANAG 4285
("SkySweeper is fussier to set up, but it also works"). The company is gone.

Installed into a throwaway Wine prefix (`/tmp/opencode/winesky`, nothing touched
in `~/.wine`), which reveals **22 test recordings at 11025 Hz mono**, one per mode
it decodes, plus the manual (`SKYSWEEP.HLP`), the per-mode configurations
(`*.cfg`, plain text) and three Hell fonts (`SKYHELL1/2/3.FON`).

**Licence: the manual's own notice is "SkySweeper is a registered trademark of
SkySweep Technologies. All rights reserved."** No licence file ships at all, and
with the company gone there is nobody to ask. So this is the RadioScript
position: **instrument and documentation, never source.** Its recordings are test
material in the same sense the MultiPSK collection is — used locally, never
redistributed. The Hell fonts are in the tree's neighbourhood too, and the rule
for them is the same: observe the *convention* from the program's behaviour, do
not copy glyph art.

### What the recordings settle

Centre frequencies measured from the audio (loudest 3 s, spectral extent), not
guessed from the file names:

| recording | centre | our decoder | verdict |
|---|---|---|---|
| `Bpsk.wav` | 1000 Hz | PSK | **decodes**: `SKYSWEEP TECHNOLOGIES` repeating |
| `Sitorb.wav` | 1000 Hz | NAVTEX | **decodes perfectly** — a real Baltic warning, `ZCZC UA72`, firing-exercise area, coordinates |
| `hffax240.wav` | 1500/2300 shift | WEFAX | **decodes**, score 0.777, a clean calibration grid at 240 LPM / 288 IOC |
| `Hellschr.wav` | ~1000 Hz | Hell | **garbage** — dense striping, same as on the MultiPSK file |
| `qpsk.wav` | 1000 Hz | PSK | **garbage**, `0\`eo``, which is the missing QPSK constellation |
| `gmdss.wav` | ~1895 Hz | DSC | 0 messages — but the harness's DSC arm takes no centre, so this is not a fair test yet |
| `sta4285.wav` | 1800 Hz | — | no decoder in the tree; this is the recording a port should be built against |
| `110ademo.wav` | ~1800 Hz | — | MIL-STD-188-110A, same family, same gap |

### Three corrections to §3, and they are the important part of this section

1. **NAVTEX was never broken.** The "SITOR_A/NAVTEX" file in the MultiPSK
   collection is **SITOR-A, which is ARQ** — a two-station handshake mode, not the
   FEC broadcast that NAVTEX is. Put a NAVTEX decoder against a SITOR-A
   recording and it is being asked for a mode it does not claim. Against
   **SITOR-B** it reads a weather bulletin and a navigational warning perfectly.
   **A classification error in the sweep, not a fault in the decoder.**
2. **WEFAX is not broken either.** The MultiPSK `FAX_Hambourg` file swings
   1400↔2400 — a **1000 Hz shift**, where WEFAX is 800 (1500/2300). Ten
   geometries scoring at most 0.043 was the right answer to the wrong signal.
   Against a standard 1500/2300 recording the decoder produces a clean chart.
   The Hamburg file is still worth understanding (what *is* it?), but it is not
   evidence against the decoder.
3. **Hell's failure is robust.** It fails on a second, independent, vendor-made
   recording, and the output *does* change with tuning — so the receiver is
   working and the convention is wrong, exactly as §4 argues. This is now the
   best-supported fault in the file.

### The recurring trap, written up because it cost this sweep twice

**A mode name in a corpus is not a mode.** SITOR-A is not NAVTEX. A file called
`FAX_...` may not be standard WEFAX. And a file with no stated centre is where
the signal isn't — `Sitorb.wav` returned nothing at 1700, 1800, 1900 and 2000 Hz
and a whole bulletin at 1000. **Measure the recording's spectrum before
concluding anything about a decoder.** Two of the nine "failures" in §3 were this
and nothing else.

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
| `fork/dsc` | DSC only — NAVTEX already works (§3b) | the corpus, once the arm takes a centre |
| `fork/stanag-4285` | protocol from the declassified spec — uncoded first | broadcasts; needs a recording |

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
- **STANAG-4285 — the specification itself is public, so none of the above needs
  anybody's code.** See §6b. The only open decoder (G4GUO's in KiwiSDR,
  `extensions/s4285/`) still reads `// (License TBD)` and may not be copied,
  ported or reimplemented from — but it is no longer *needed*, which is the
  cleanest position available.
- **The recordings stay local fixtures.** Nothing from the collection goes into
  the tree, and none of it is redistributed.

## 6b. STANAG-4285 is a public document, and that settles the licence question

The standard is **not** behind a wall. It is **NATO UNCLASSIFIED**, formally
downgraded by **MAS/59-EL/4285 of 9 February 1993** — *"recipients of reference b.
are requested to downgrade STANAG 4285 to NATO UNCLASSIFIED"* — and a scan of the
document circulates publicly (obtained from
`http://www.n2ckh.com/MARS_ALE_FORUM/s4285.PDF`, which the UDXF notes link as
*"the unclassified specification can be downloaded here"*). It is Edition 1,
promulgated 16 February 1989, and it carries a NATO copyright line — so **we may
implement from it but must not redistribute the document or paste its text into
the tree.** The parameters below are functional facts, written out in our own
words, and that is the normal posture for a standards-based implementation.

**So nobody has to derive anything from anybody's code, and no clean room is
needed.** Read the source, write the receiver. That is also, almost certainly,
what MultiPSK and Sigmira did — which is the honest version of "people can have
the same idea": here the idea is a published document rather than a secret.

### The waveform, from Annex A (normative)

- **1800 Hz sub-carrier**, phase-shifted. **2400 baud**, held to 1 part in 10⁵,
  with the symbol clock locked to the same reference as the carrier. Occupied
  bandwidth **3000 Hz**.
- **Absolute phase**, not differential: symbol *n* maps to phase **n·π/4**, so
  the eight states are 45° apart and the receiver needs a phase reference — the
  48 reference symbols per frame are what provide it. (Our `psk.rs` is
  differential; this is the one place 4285 differs from amateur PSK.)
- Transcoding, and this is how the constellation is chosen by the data rate:

  | rate | bits per symbol | transcode |
  |---|---|---|
  | 1200 | 1 | `0 → sym 0`, `1 → sym 4` |
  | 2400 | 2 | `00→0  01→2  10→6  11→4` |
  | 3600 | 3 | `000→1 001→0 010→2 011→3 100→6 101→7 110→5 111→4` |

  Both are Gray-coded, which is why the transcoded symbol order looks shuffled.

### The frame

**256 symbols = 106.6 ms**, giving 128 bits at 1200 bps, 256 at 2400, 384 at
3600. Within a frame:

- **80 sync symbols**, then **48 reference symbols**, then **128 data symbols**.
- The reference and data symbols form **four blocks**: three of *32 data + 16
  reference*, then a last block of *32 data*. **Every reference symbol is symbol
  number 0** — an unmodulated 1800 Hz burst, 16 symbols long, three times a frame.
- The 176 reference-plus-data symbols are multiplied by an **176-symbol, 8-phase
  scrambling sequence**, so that the frame looks like 8-PSK whatever the data
  rate is.

### The generators, in closed form

Both are linear feedback shift registers, and both are in the document, so the
sequences do not have to be guessed or measured:

- **Sync sequence** — 80 symbols, BPSK, recurrence **x⁵ + x² + 1**, register
  initialised to **1 1 0 1 0** at the start of every frame. The first symbol is
  the register's least significant bit and the remaining 79 come from clocking it
  79 times. (Because the sequence is length 31, 80 symbols is two whole periods
  plus the first 18 of the next — the document says so explicitly.)
- **Data scrambler** — 176 symbols from a PN sequence of length **511**,
  recurrence **x⁹ + x⁴ + 1**, register initialised to 1 at the start of the
  frame. Each symbol takes the three lowest register bits and forms
  **n = 4·x₂ + 2·x₁ + x₀**, giving phase **n·π/4**; the register is then shifted
  **three positions** for the next symbol.

The sync is also what the receiver uses for signal detection, Doppler and
frequency correction, bit timing and equaliser training — so a 4285 receiver is
one acquisition step followed by a decision-directed tracking loop, which is a
shape this tree already has in the VDL2 and HFDL lanes.

### The only gap, and it is not in the public copy

**Annex E** — the convolutional code and the interleaver — is referenced by the
excerpt above but is not part of it. That matters only for the **coded** rates;
**1200, 2400 and 3600 bps are uncoded**, so the document as it stands is a
complete implementation of those three. And even Annex E is substantially
described in public: the interleaver's parameters are stated verbatim in
**STANAG 4529's conformance test procedures** (also on the same public MARS
forum: *"Number of rows: I = 32 for all data rates…"*), and the code is discussed
in M. B. Jorgenson's *"HF Serial-Tone Waveform Design"*
(`hflink.com/standards/HF_modems_explained_stanag.pdf`).

### What is actually on the air, and it decides the order of work

Two operational facts, both from Hugh Stegman NV6H's *"Demystifying STANAG 4285"*
(Monitoring Times, July 2009, p.28) and confirmed by the UDXF notes:

- **It does not autobaud.** Rate, interleave length and framing must be told to
  the decoder, so a receiver that guesses wastes the transmission.
- **"Nearly everything that we stand any chance of decoding will be 300 or 600
  baud, long interleave… usually ITA2, same as used in RTTY."** So the first
  useful target is not the 3600 bps technical showpiece — it is **600 bps, long
  interleave, ITA2 5N1**, which is Annex E territory.

Which means, honestly: the uncoded modes are the *easy* part to build and the
*rare* part on the air, and the common traffic needs exactly the annex we do not
have. Both are fine places to stop — but the ordering should be chosen with that
in mind rather than by what is easiest.

### Test material, without anybody's code

- **Sigmira ships example STANAG 4285 recordings** in its distribution package
  (`sigtools`), alongside HFDL, SITOR-B, CW, FLEX and others, plus a signal
  database of over two thousand entries. Freeware, so the *recordings* are usable
  as local fixtures; the program itself is closed and Windows-only.
- The UDXF notes give the French Navy net as the best-documented traffic (FUE
  Brest 4.295, FUO Toulon 2.6084, 6WW Dakar 12.587 and others, with the
  rate/interleave/framing each was copied with), which is a real receiver target.
- `~/Downloads/sounds` has no 4285 file — the MultiPSK collection does not cover
  it, so this mode needs a recording brought in before it can be judged.

## 7. What is not proven, and what is deliberately not done

- **Nothing has been adapted.** Every experiment was reverted byte-for-byte and
  `main` is clean. The only artefact is the harness in `/tmp`.
- **The nine failures are one program's recordings.** Two of the three timing
  modes decode *other* signals we know work (Olivia on Kevin's capture and the
  Wikipedia sample; Hell's geometry follows fldigi), so a change made to suit
  MultiPSK could break what works. **Kevin's capture is not on this box and is
  needed before `olivia.rs` is touched.**
- **WEFAX is answered as far as the decoder goes, and the open question is now
  about one file rather than the code.** Ten geometries on the MultiPSK Hamburg
  recording score at most 0.043, and its measured shift is ~1000 Hz
  (1400↔2400) where WEFAX is 800 (1500/2300). Against SkySweep's standard
  1500/2300 recording (§3b) the decoder produces a clean calibration grid at 240
  LPM / 288 IOC, score 0.777. **So the decoder works and the Hamburg file is not
  a standard WEFAX signal.** F4NRM's comparison is still worth having — it would
  say what that file *is* — but it is no longer a question about our WEFAX.
- **The 4285 mode is no longer parked.** Its specification is public and
  declassified (§6b), so the work can start whenever it is wanted — but it wants a
  real recording before it can be judged, and the traffic that matters needs
  Annex E, which the public copy does not include.

## 8. Housekeeping

- One branch per mode, in this checkout. **No worktrees** — 110 GB each.
- Work one mode at a time: every branch switch recompiles the crates that differ,
  and the shared `target/` keeps every variant's artefacts.
- The harness and the corpus live outside the tree and stay there.
