# Modes we do not ship — a todo, cheapest first

**Local only.** Written 2026-10-19. Read with `MODE-DECODERS-HANDOVER.md`, which
has the instruments, the measurements and the per-mode state; this file is the
queue.

## The rule the ordering comes from

A mode costs what it costs because of two things, and nothing else:

1. **Is there a licence-clean reference?** GPL-3 or MIT, or a public standard.
   Without one, the mode is not cheap at any size — it is guesswork with a
   deadline.
2. **How far is its DSP from what this tree already does?** A mode that is our
   FSK engine with different constants is an afternoon. A mode with a novel air
   interface, its own FEC and a bit-level protocol is a project.

So "easy" below does not mean "unimportant". It means "we already have the
reference and most of the machine".

**Every mode fldigi implements is licence-clean**, because fldigi is GPL-3 and so
is this fork. That covers a startling amount of the list, and it is why the top
half of this file is short.

## Already done, or one small step away

These are not new modes; they are the current tree, nearly there:

**These were checked on 2026-10-19, and the guess was wrong about two of the
three.** Measured, not assumed:

| | measured state |
|---|---|
| **Feld Hell** | **works** — MultiPSK's `FELD_HELL-1000_Hz` reads as text (French amateur copy). Confirmed a third time, on a third recording |
| **Hell 80** | **fixed** — it transmits **7 rows**, not 14 ("displayed twice, transmitted once"); the cell height is now a property of the variant, and MultiPSK's recording reads |
| **FM Hell (FSK 245)** | **works** — and my first two explanations were wrong. The file name's 1000 Hz is the **upper of its two tones**; the centre is 939, and at that tuning it decodes with no wrap and no change to the gap metric |
| **RTTY 45.45** | **works** — clean text from `RTTY45-1000_Hz` |
| **RTTY 50** | **works, but not at 170 Hz shift**: MultiPSK's file is **450 Hz**, and at that shift the text reads. `rtty_baud` and `rtty_shift_hz` both reach the demodulator, so this was a parameter, not a decoder |
| **RTTY 75** | **works** at its own baud and a 450 Hz shift — once tuned to its real centre of ~786 Hz rather than the 1000 in the name |
| **SYNOP** | **works**: 50 baud, 425 Hz shift, centre 1367, and it prints a whole bulletin — `ZCZC 277`, `SMNO01 ENMI 121800`, `AAXX`, the station groups, the `333` groups, `NNNN`. What is missing is not decoding but *wiring*: nothing in the tree runs RTTY and hands the text to the SYNOP parser |
| **BPSK63/125/250, QPSK31/63/125/250** | unchanged: `psk.rs` hardcodes 31.25 and the detector is BPSK only (`fork/psk-variants`) |

### The lesson, and it cost most of a day

**MultiPSK's file names are not centres.** They name a *tone*, and for the
two-tone modes that is the upper tone — FM Hell at 939 (not 1000), RTTY 75 at
786, SYNOP at 1367. Three of the four "failures" were a tuning error in the test
rig, and each one looked exactly like a broken decoder: the wrap in FM Hell, the
mitten-garbled text in RTTY 75, the plausible digits in SYNOP.

So the rule before judging any of these: **measure the tones in the file and tune
to the middle of them** — the same rule as measuring the centre on the Hell
recording of §3, and it has now been learned twice. What survives of the original
guess is Hell 80, which was a genuine geometry bug, and it is fixed.

## The todo, cheapest first

"Reference" is what makes a mode possible: **fl** = fldigi (GPL-3), **pub** =
public standard or declassified document, **none** = nothing licence-clean
found. "Recording" is whether our corpus already holds one.

### 1. Hours each — our engine, different constants

| mode | reference | recording | what it needs |
|---|---|---|---|
| **Contestia** (all) | fl | `CONTESTIA_32_1K` | Olivia's machinery with a different tone/framing set. **Falls out of the Olivia fix** — do it after, not before |
| **MFSK8 / MFSK16** (and 32/64) | fl | `MFSK8`, `MFSK16` | a new modem, but a small one; fldigi's is ~1 500 lines. Shortwave Radiogram uses MFSK32 weekly, so it has an audience |
| **MT63** | fl | `MT63` | 64-tone OFDM with Walsh spreading; the spreading is the whole trick |
| **DominoEX / DominoF** | fl | `DominoEX`, `DominoF` | incremental FSK, IFK+; `src/ifkp` is the same family |
| **THROB / THROBX** | fl | `THROB_2`, `THROBX_2` | multi-tone with a reeded pulse shape |
| **RS_ID / CALL_ID** | fl (`src/rsid`) | `RS_ID_QPSK31`, `CALL_ID_F6CTE_JN18` | a short watermark burst; small, and it makes every other mode nicer |
| **DTMF** | fl (`src/dtmf`) | `DTMF` | a tone decoder. Hours |
| **DGPS** (RTCM SC-104) | pub | `DGPS_100` | 100 baud FSK — the NAVTEX decoder with different constants |
| **POCSAG** | pub | `POCSAG_bits_inverted` | 512/1200/2400 FSK paging, fully documented |
| **SELCAL** | pub (CCIR tones) | `SELCAL` | tone sequences. Hours |
| **QRSS / NDB** | — (it is CW) | `QRSS_1sec`, `NDB-PHG` | our CW decoder with a much longer time constant |
| **NWR / SAME** | pub | `NWR_SAME_WASHINGTON` | the 520.83 baud SAME header — an AFSK burst with three repeats |

### 2. Days each — a new modem, but the machine is known

| mode | reference | recording | what it needs |
|---|---|---|---|
| **STANAG 4481** | pub | — | 75 baud FSK teletype, the naval equivalent of RTTY. We have the engine |
| **MIL-STD-188-110A serial** | pub | `110A`, `skysweep/110ademo` | 4285's cousin; same 2 400 baud symbol stream, different preamble and interleaving |
| **STANAG 4529** | pub (conformance procedures are public) | — | 4285 at 1 240 Hz; the conformance document states the interleaver parameters 4285's Annex E does not give us |
| **141A / 1382** (French Navy) | partially pub | `141A_*`, `1382_GPS_positions` | the same family again, with a French preamble |
| **MFSK32 / 64** | fl | — | the rest of the MFSK family once 8/16 works |
| **Radiosondes** (RS41, LMS6, IMET4, M10) | pub (open decoders exist) | `RS41`, `LMS6`, `IMET4`, `M10` | GFSK at 4800 baud, no licence wall, and a genuinely different audience — weather people |
| **ORBCOMM** | partially pub | `ORBCOMM` | satellite SDPSK; needs a wideband lane and its own band |
| **STANAG 4539** | pub | — | 110B-class, own preamble and equaliser |

### 3. Weeks — the interesting one

**STANAG 4285.** The most interesting mode for an SWL, and the only one where we
have already done the hard intellectual work: the specification is
**declassified** (Edition 1, 16 February 1989, downgraded to NATO UNCLASSIFIED
in 1993) and **the transcription is proven** — a frame built from it round-trips
exactly (`tools/s4285-probe/s4285_sim.py`).

What is left is receiver engineering, in this order:

1. **A real resampler.** The probe uses linear interpolation, and that is what
   gives up past a few hundred ppm of clock error. Polyphase or cubic.
2. **Carrier tracking per frame**, from the 48 reference symbols. One offset
   fitted over a whole recording holds for a single frame — that is measured.
3. **Acquisition on a faded signal.** `~/Downloads/sounds/4285.WAV` (MultiPSK's
   own, 64.7 s at 12 kHz) is the better of the two fixtures; SkySweep's fades 45×.
4. **Annex E** — the convolutional code and interleaver — for the **coded**
   rates. The uncoded 1 200 / 2 400 / 3 600 bps need none of it, and the traffic
   that is actually on the air is coded 600 baud long interleave. Its parameters
   are stated in STANAG 4529's conformance procedures, which are public.
5. **The output**: ITA2 5N1/5N2, which is RTTY's character set and already in
   the tree.

### 4. Not ranked, because there is no clean reference yet

Each needs a research day before it can be estimated at all:

`Coquelet-8/13`, `Piccolo`, `PAX`, `PAX2`, `CHIP64`, `Lentus`, `BIIS`, `DFM`,
`IEC-870-5`, `THROB` (covered above), `Pactor-I` (documented in outline),
`GTOR`, `IRA-ARQ`, `MD-*` (Russian), `RSX.25`, `EPIRB`, `EM` beacons,
`skyofdm` (SkySweep's own OFDM, undocumented).

### 5. Not doing, and the reason is not laziness

**DMR, P25, NXDN, dPMR** — the AMBE+2 voice codec is covered by US Patent
8,359,197 until **2028-05-20**. Decided by the operator on 2026-10-11: we stay
quiet on these. The thesis is in `AGENTS.md` under "DMR: scoped to the codec, and
decided against", with the survey of what the ecosystem does (DSDcc ships without
the codec; the others are anonymous, licensed silicon, or closed).

**D-STAR / C4FM** — the same codec family, but a clean pure-Rust crate exists for
D-STAR, which changes the answer for that one mode alone. Not queued.

**Pactor II / III** — proprietary and encrypted; there is nothing to implement
against.

## A suggested order for a few days

The point of this order is that each step makes the next cheaper, and the first
two are near-free:

1. **Verify the free wins** — RTTY 50/75, SYNOP, and the Hell variants we
   already ship. A harness arm each, an hour in total, and it turns "untested"
   into either "works" or a real finding. Do this first because it is how we
   learn what the tree already does.
2. **`fork/psk-variants`** — symbol rate and constellation. It fixes seven corpus
   files, and it is the front end of both 4285 and 110A.
3. **MFSK8/16** — the cleanest *new* mode, a popular one, and it exercises the
   "add a mode" path end to end for whoever does the rest.
4. **STANAG 4285** — the resampler and carrier tracking, against `4285.WAV`.
5. **One from the hours table per day** after that: Contestia (once Olivia is
   fixed), MT63, DominoEX, RS_ID.

The two things that would change this order are a **clean recording of a coded
4285 transmission whose text is known** — that would collapse step 4's Annex E
question — and **fldigi's `src/` read end to end**, which would probably reveal
that three or four of the "days" entries are afternoons.
