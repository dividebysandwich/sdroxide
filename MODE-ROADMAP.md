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

### QPSK: what it needs, read off fldigi (2026-10-19)

The FEC is in place — `ConvCode::PSK_QPSK` is K=5, rate 1/2, and the trellis size
now follows the code. Four things remain, and the first is the one that would
have cost a day to discover:

0. **A Costas error for QPSK — this is the one that is actually missing.** The
   loop in `BpskCore` is *decision-directed BPSK*: `if y.re >= 0 { y.im } else
   { -y.im }`. That is correct for BPSK and cannot track a four-phase carrier, so
   everything downstream of it sees a wandering phase and the differential
   decision is noise. Found on 2026-10-20 by sweeping the symbol conventions
   against `QPSK31-1000_Hz` — all eight quadrant rotations and bit orders scored
   zero, which rules the mapping out and points upstream of it. Until this is
   fixed nothing else about QPSK matters.
1. ~~**A streaming Viterbi.**~~ **Done** — `ViterbiStream`, in `fec.rs`. `viterbi_soft` is a *block* decoder: it starts every
   call from state 0 and traces back from the best final state. That is right for
   THOR and QO-100, which decode a finished block, and wrong for a continuous
   mode — a trellis restarted every block is a trellis with a discontinuity in
   it. QPSK needs survivor history carried across symbols with a traceback depth
   of a few constraint lengths, and the tail bits only at the end of an over.
2. **One data bit per symbol, not two.** `psk::tx_bit` does
   `sym = enc->encode(bit) & 3`, so a QPSK symbol *is* the encoder's output pair.
   Two channel bits per symbol at rate 1/2 is one data bit — which is why QPSK31
   carries text at the same rate as BPSK31 with half the errors.
3. **The mapping.** Transmit `sym` straight into the 2-bit phase; receive
   `bits = (4 - bits) & 3` unless `reverse` is set, then two soft symbols with
   **the second inverted** (`sym[1] = (bits & 2) ? 0 : 255`) — the inversion is
   how the standard guarantees transitions on an all-zero message.
4. **A selector.** `PskSpeed` names the four rates; QPSK is a *second axis* on
   top of them, so it wants its own field or a wider enum, and a wire bump.

Recordings: `QPSK31/63/125/250-1000_Hz` in `~/Downloads/sounds`, named for a
*tone* rather than a centre (the lesson from §1) — measure before trusting 1000.

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

### Olivia proven on the air, 2026-10-20 (night)

Live proof, not a fixture: fldigi transmitted Olivia 16/500 from the CRT SS9900v
through VOX, the RSP1 recorded 27.415 MHz, and the vendored engine decoded it.

```
band 1451-1958 Hz = 507 Hz wide, centre 1705, 109x the noise, zero clipping
reverse on:  "j#P}TEST TEST THE EAGLE HAS LANDED 12345/B%e,@0M@|$E)I5&9FCXFweq..."
reverse off: "+"|AUAR!DUaUDaDFDaIRaMOD!p3r5tAAnVCddI-qL}eDhHt'x..."
```

The phrase is exact, the junk either side is idle and lock, and the negative half
— the same audio with the switch off — is noise. **The mode works, receive, in
the program users get**, because the band menu and this test run the same engine.

Two things the run showed as well:

- **Reverse was needed again**, on a second, independent transmission. It is not
  an artifact of how the captures were extracted; it is this rig's path. The chip
  is load-bearing and the hover text says what it is for.
- **The carrier stayed up for about ninety seconds** after the text finished,
  Olivia's idle filling the gap — the same "fldigi holds TX until released" that
  cost an evening earlier. Use the macro that ends with `<RX>`, or press RX.

### Olivia, 2026-10-20 (done): the vendored engine decodes it — and the capture was inverted

`crates/sdroxide-olivia` vendors Pawel Jalocha's `MFSK_Receiver` (fldigi's
`src/include/jalocha/pj_*.h`, GPL-2.0-or-later, unmodified) behind a small C++
shim, with `OlivaDecoder::new(rate, centre, tones, bw, reverse)`.

**It reads the fixture**: `TEST TEST THE EAGLE HAS LANDED 12345`, exactly, with
a couple of junk characters at the start while the synchroniser locks. It also
reads the MultiPSK 32/1k recording our own decoder has failed on for days — 121
characters of readable English.

**And it produced the finding that explains two days of confusion: our captures
were spectrally inverted.** The engine decodes your over only with `reverse` set,
and the same was true of the first RTTY capture — which needed `reverse` too and
was recorded at the time as a decoder polarity quirk. It was not: the recordings
were extracted as **USB** from a rig that was almost certainly in **LSB** on 11 m.
Our own `OliviaRx` has no reverse at all, so it could never have read them, and
the MultiPSK sample — which reads fine with reverse *off* — was the one case where
the inversion was not in play, which is why that one looked like a decoder fault
and this one looked like a signal fault.

**The rule to carry:** when a capture will not decode, check the sideband before
suspecting the demodulator. `reverse` is not a curiosity, it is the difference
between reading a station and reading noise.

**Note, and it is the operator's:** if F4NRM (Kevin) has a working Olivia pushed
during the freeze, **this function is not pushed** — his work takes precedence and
this crate stays unpublished rather than competing with it.

**Wired, 2026-10-20:** `RxModem::Olivia` now holds `sdroxide_olivia::OliviaReceiver`,
so the mode in the band menu is decoded by the vendored engine. The receiver
answers the same three calls our `OliviaRx` did — `process`, `magnitude`,
`set_params` — so no other code moved; the S-meter is fed by the decayed peak of
the arriving audio, because the engine reports no level.

**Transmit is still ours.** `OliviaTx` has never been confirmed by another
station, and now there is a way to check it: send from this program and decode it
in fldigi. That test has never been run in either direction until tonight's
receiving work, and it is the last unknown in the mode.

**The reverse switch is in, 2026-10-20** (`PROTO_VERSION` 218 → 219,
`VERSION_BYTE` 0x26 → 0x27). `DigiConfig::olivia_reverse` is appended at the tail,
a **REVERSE** chip sits beside the tones and bandwidth chips, and it is applied to
both the receiver and our transmitter. Proved on the real capture: with it on,
`TEST TEST THE EAGLE HAS LANDED 12`; with it off, noise — the negative half is
what makes the test mean anything.

**And the switch's work found a bigger fault: our transmitter is not Olivia to
anyone else.** A transmission from `OliviaTx`, reversed or not, does **not**
decode in Jalocha's engine, while our own `OliviaRx` reads it perfectly. So the
modulator and our demodulator agree with each other and with nobody else — which
is what the oldest note in this file suspected ("whether a real fldigi can copy
*our* transmission") and never checked. The recorded state is the
`our_transmitter_is_not_yet_readable` test: ignored, because it asserts a known
fault rather than a fixed one.

Fixing it is the mirror of the receiver: check our modulator's tone assignment
against `MFSK_Transmitter`, with the engine as the oracle. Until then Olivia is
receive-only in practice, which for a listener fork is most of what it is for.

### Olivia, 2026-10-20 (the verdict): stop reimplementing it, vendor the engine

A faithful reimplementation of the reference front end was written and run against
the fixture — raised-cosine window two symbols long, two slices half a symbol
apart, energy-squared soft bits, and a sweep of slice parity × block offset ×
polarity. **It does not decode either** (`a_capture_decodes_with_the_reference_front_end`,
0.2 s for the whole sweep, prints what each combination produced).

So the situation is now clear, and it is not what any of the previous entries
assumed: **fldigi reads our recording perfectly, and three separate
implementations on our side — `OliviaRx`, a Python model, and a careful port of
`pj_mfsk.h` — all fail on it.** The error is in the reading of the reference, not
in the architecture of Olivia, and reading it harder has now been tried three
times.

**The next move is not another reimplementation. It is to vendor the engine.**
`pj_mfsk.h` and its companions are Pawel Jalocha's, GPL-2-or-later, which this
tree already satisfies — the same route taken for `vendor/xng` (HFDL) and
`vendor/rade_c`. A `sdroxide-olivia` crate wrapping `MFSK_Receiver` would give a
decoder that is *known* to work, because it is the one that just read our
capture, and it removes the class of bug entirely rather than continuing to
guess at a Walsh/interleave convention.

That is a decision for the operator — a vendored C++ engine against a pure-Rust
one — but it is the only route with a known-good endpoint, and the fixtures make
the result checkable in 1.8 seconds either way.

### Olivia, 2026-10-20 (final): the recording is good, the decoder is not — and here is what it lacks

**The oracle question is answered, and it went against us.** The 16/500 capture was
played into fldigi through a PipeWire null sink (fldigi's input on `default`, the
capture shifted so the bank centred on its 1500 Hz carrier) and **fldigi read it
perfectly**. So the recording is genuine Olivia, the transmitter path is clean, and
our decoder is the fault. No more blaming the signal.

Reading the receiver half of `pj_mfsk.h` — the part that was truncated the first
time — gives the three things ours does not do:

1. **An input equaliser runs before anything else.** `MFSK_InputProcessor`, an
   8192-sample window: the spectrum is peak-limited three times
   (`LimitSpectraPeaks`), averaged, and each bin scaled by `1/sqrt(energy)`. That
   whitens the band and flattens level differences before the demodulator sees
   them. We feed the raw audio straight in.
2. **Two slices per symbol, each a 512-sample `1−cos` window.** The shaped window
   is zero at both ends, so it reads the *middle* of the symbol; the second slice
   is 128 samples on and reads the other half. Which slice is used is decided per
   symbol by the decoder's own score, not by a magnitude proxy. Ours takes one
   rectangular 256-sample window and picks its phase by peak magnitude.
3. **`DecodeMargin` and a frequency-offset search.** `SoftDecode` is run for every
   offset from `−FreqOffsets/2` to `+FreqOffsets/2`, the best is chosen per symbol,
   and the block phase advances per slice.

**What this means for the work.** This is not a bug hunt any more, it is a front
end to port: `olivia.rs`'s `OliviaRx` needs the shaped window, the two slices with
scored selection, and ideally the equaliser. The fixtures make it checkable in 1.8
seconds, and **fldigi is now a usable oracle** — our own capture fed back into it
decodes, so any change can be validated against a recording whose answer two
independent implementations agree on.

### Olivia, 2026-10-20 (later): the reference is read, and the next question is the signal

The reference implementation has now been read rather than remembered
(`w1hkj/fldigi`, `src/olivia/olivia.cxx` and `src/include/jalocha/pj_mfsk.h`).
What it says, and where we differ:

| | fldigi | ours |
|---|---|---|
| window | 512 samples, raised-cosine shaped, hopped 128 | one 256-sample rectangular hop per symbol |
| spectra per symbol | **two** (the synchroniser picks one) | one |
| tone spacing | every **2** FFT bins of 512 = 31.25 Hz | 31.25 Hz, sampled directly |
| timing | searches **frequency offset × block phase** and scores each | picks a sub-phase by per-window peak magnitude |

The bit mapping matches ours: character `p` reads position `(p+i) mod planes` of
symbol `i`, scrambled with `0xE257E6D0291574EC` at `(13p+i) mod 64`, forward FHT,
peak index, `+64` when the peak is negative.

**And a from-scratch Python reimplementation following the reference — sweeping
symbol offset, slice parity, block phase, frequency offset, both polarities, both
Gray directions and both peak rules — still does not decode our fixture.** So the
remaining explanation is not the decoder: it is the **signal**.

Two observations point at the rig. A clean Olivia symbol should put most of its
energy in one tone; ours measures **0.98** peak/rest, i.e. about half, on a
capture that is otherwise clean (114× the noise, no clipping). And the
transmitter's sound card is at 96 % PCM, which may be overdriving the modulator
into intermodulation — which looks *exactly* like a flat 500 Hz block on a
spectrum, and destroys the tone structure underneath.

**The test that settles it, and it needs no analysis:** play the capture back into
fldigi and see whether fldigi reads its own transmission. It encoded the text, so
if it cannot read the recording, the recording is not Olivia. That has never been
checked because fldigi cannot transmit and receive at once.

### Olivia, 2026-10-20: the fixture is runnable, and three things are excluded

Olivia is the most **visible** fault in the tree — it is in the band menu and it
returns nothing — so it is the right place to work. What this session added is
the ability to measure it at all:

* **A runnable fixture.** `an_off_air_capture_decodes` takes
  `SDROXIDE_OLIVIA_SAMPLE` and asserts on the *content*. MultiPSK's
  `OLIVIA_32_1K-1000_Hz.WAV` converted with `ffmpeg -ar 8000 -ac 1` gives
  `decoded 0 chars: ""` — the fault, reproduced, and the gate for every future
  change. Nine minutes a run, so budget one or two per session rather than a loop.
* **A trace.** `SDROXIDE_OLIVIA_TRACE=1` prints the block confidence every eighth
  block: `best`, `second`, their ratio, the chosen phase and `tphase`.

**What the trace says, and it is the useful part:** `best/second` is
**1.00–1.03**. No phase dominates, so the block correlation is flat — the phase
and lock logic has nothing to work with, and `locked` is a fluke that never
releases. That **excludes the locking code** and points at `soft_bits` (the
per-symbol tone-to-soft-bit step) or the tone bank itself.

**And one candidate is excluded by measurement:** voting the sampling instant by
the *total* comb energy instead of the peak tone looks right on paper and breaks
`loopback_8_250`, which the peak vote reads perfectly. Written, tested and
reverted; the code says so, so it is not tried again.

So the next session starts where this one stopped: **`soft_bits` and the tone
bank**, with a nine-minute gate to check each change.

### 3. Weeks — the interesting one

**STANAG 4285.** The most interesting mode for an SWL, and the work is well past
the reading stage. What is **done and measured** (`tools/s4285-probe/`):

- **The transcription is proven.** A frame built from the declassified spec
  round-trips exactly at 30, 40 and 60 dB (`s4285_sim.py`). Frame layout, the
  176-symbol scrambler, the sync generator and the eight phases are all as the
  standard describes.
- **The fixture is confirmed.** MultiPSK's `4285.WAV` autocorrelates at **1280
  samples = 106.67 ms exactly** — 256 symbols at 2400 baud — at **0.594 against a
  0.065 median**.
- **Acquisition is solved by folding.** Folding the recording onto its own frame
  period locks at **0.749** over eight frames (floor 0.35), **0.60** over 32, at
  about **−44 Hz** of carrier offset. It assumes no sync pattern, so the lock
  cannot be an artefact of a guessed sequence.

What **blocks it**, and it is not code: the folded frame — with the SNR of eight
averaged frames — does not carry Edition 1's preamble. The best correlation
against the standard's PRN, over twenty shift-register conventions and all 256
rotations, is **0.291 against a 0.11 floor**, and the all-zero reference symbols
do not collapse. So this recording is either a **variant** (sigidwiki notes the
Thales ones are identified by "a special preamble") or a **test transmission**
whose data repeats.

**The next action is not more analysis of that file. It is a recording of a
station known to be sending Edition 1** — a naval broadcast, or one of Sigmira's
examples. Given one, the remaining work is bounded and in this order:

1. **Carrier and timing tracking per frame**, from the 48 reference symbols. Both
   move: the offset across this recording wanders by tens of Hz, and the frame
   phase drifts.
2. **A real resampler** — polyphase rather than the probe's linear
   interpolation, which is measured to give up past a few hundred ppm.
3. **Annex E** — the convolutional code and interleaver — for the **coded**
   rates. The uncoded 1 200 / 2 400 / 3 600 need none of it, and the traffic
   that is on the air is coded 600 baud long interleave. Its parameters are
   stated in STANAG 4529's conformance procedures, which are public.
4. **The output**: ITA2 5N1/5N2, which is RTTY's character set and already in
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

## The complete list of modes we do not carry

Every mode named by the two reference programs in front of us — MultiPSK's own
recordings in `~/Downloads/sounds` and SkySweeper's mode table — that this tree
does not decode today. **65 of them**, counting each distinct mode once — Pactor's three generations as
three, and the four QPSK speeds as four, because each is a setting an operator
would have to choose between.

### Teleprinters and ARQ (7)

`SITOR-A` (ARQ — the corpus file is this one, and it is *not* NAVTEX) ·
`Pactor-I` · `Pactor-II` · `Pactor-III` · `GTOR` · `IRA-ARQ` · `RSX.25`

### HF data modems (8)

`STANAG 4285` · `STANAG 4481` · `STANAG 4529` · `STANAG 4539` ·
`MIL-STD-188-110A` · `141A` (French Navy) · `1382` (French Navy) · `Link-11`

### Keyboard and MFSK modes (13)

`MFSK8` · `MFSK16` · `MFSK32` · `MFSK64` · `MT63` · `DominoEX` · `DominoF` ·
`THROB` · `THROBX` · `Contestia` · `RTTYM` · `Piccolo` · `Coquelet-8/13`

### PSK (8)

`QPSK31` · `QPSK63` · `QPSK125` · `QPSK250` (all four need the FEC, not just a
constellation) · `PSK10` · `PSK63F` · `PSK125R` · `PSK220F`

### The obscure HF modes (10)

`CHIP64` · `PAX` · `PAX2` · `Lentus` · `BIIS` · `DFM` · `IEC-870-5` · `M10` ·
`CCW` (FSK and OOK) · `skyofdm` (SkySweeper's own, undocumented)

### Selcall, paging and tones (5)

`SELCAL` (CCIR and the national tone systems) · `DTMF` · `POCSAG` · `FLEX` ·
`NWR/SAME`

### Utility and satellite (5)

`DGPS` (RTCM SC-104) · `EGC` · `ORBCOMM` · `EPIRB` · `EM` beacons (GPS and
locator)

### Weather and position (5)

`SYNOP` (decodes today but is not wired to a mode) · `SHIP` · `RS41` · `LMS6` ·
`IMET4`

### Voice codecs, and not doing them (4)

`DMR` · `P25` · `NXDN`/`dPMR` · `D-STAR`/`C4FM` — the AMBE family. The first four
are the patent case; D-STAR is the one a clean Rust crate exists for.

### Not a mode at all

`VOICE` (a recording of speech, useful for tuning) · `RDS` (a subcarrier inside
an FM broadcast, and our NFM/WSM path would have to carry it) · `NDB` and `QRSS`
(both CW, so a decoder setting rather than a mode) · `SITOR-B` (NAVTEX, shipped).

**Which of these is cheap is in the tables above**; this page exists so the
count is honest and nothing gets quietly dropped.

## A suggested order for a few days

The point of this order is that each step makes the next cheaper, and the first
two are near-free:

1. ~~**Verify the free wins**~~ — **done 2026-10-19**. All seven work; three
   were my tuning and Hell 80 was a real bug. See the table above. ~~A harness
   arm each~~ was the right instinct: it is how we
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
