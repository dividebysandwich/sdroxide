# STANAG 4285 — the mode, and the brief to build it

**Read [`STANAG4285-EDITION1.md`](STANAG4285-EDITION1.md) first.** It is the
standard itself — Annexes A and F, transcribed from a declassified 68-page
Edition 1 — and it settles the transcoding, both generators, the frame and the
acquisition method. This file is the state of our own work, the measurements that
constrain it, and a prompt for the session that builds the mode.

## Where the work stands

**The state, briefly (2026-10-22, third session).** The receiver exists
(`crates/sdroxide-dsp/src/stanag4285.rs`), it locks real off-air captures with the
reference blocks collapsed, the mode is wired (`Mode::Stanag4285`, a panel of
measurements, `PROTO_VERSION` 227), and **Annex E is transcribed and built**
(`stanag4285_fec.rs`): the convolutional code, the interleaver, the SOM/EOM framing
and an ITA2 reading, proven end to end on a loopback. What is still missing is a
*real coded transmission* to decode — the night's captures are uncoded or idle, and
the daytime ones were not 4285 at all. See the last section.
`tools/s4285-probe/` remains the Python measurement log, deliberately outside the
build.

What the probes established, and what each was measured on:

| question | answer | evidence |
|---|---|---|
| Is the frame geometry real? | **yes** | ACF at 1280 samples = 106.67 ms on MultiPSK's `4285.WAV`, 0.594 against a 0.065 median |
| Can it be acquired without the preamble? | **yes, by folding** | 0.749 over 8 frames on the same file |
| Does the standard's 80-symbol preamble appear in the shipped fixtures? | **no** | best correlation 0.291 against a 0.11 floor, over 20 shift-register conventions and all 256 rotations |
| Do the 48 reference symbols collapse? | **not on those files** | R = 0.529, but the permutation null on the same file is 0.601 — larger |
| Does a real station fold? | **yes** | FUO Toulon 5215.6 kHz, 0.70 over 16 frames, at +636 Hz of ramp |
| Did that capture show the layout? | **no** | it was *idle* — 222 of 256 symbols in two adjacent 45° sectors |
| Is the transcription right? | **yes** | `s4285_sim.py` round-trips exactly at 30, 40 and 60 dB |

So the position is narrow and clear: **the instrument is sound and the material
is not.** Both shipped recordings are somebody's demo file (MultiPSK's, SkySweep
Pro's), neither carries Edition 1's preamble, and one real capture turned out to
be a station sitting idle. Everything up to the payload is solved; the payload has
never been read from the air.

## What a recording can and cannot tell you

Two nulls had to be measured, and both were mistakes first:

* **A searched statistic needs its own null, on the same material.** The
  reference-concentration score (descramble, and the 48 all-zero reference symbols
  should collapse to one phase) reported R = 0.529 on `4285.WAV`, which looks like
  a frame in hand. Shuffle that file's own frames so nothing can repeat, run the
  identical search, and the null reaches **0.601** — higher than the "answer". The
  score is the size of the search, not a signal, until there is enough averaging
  or known content.
* **The fold's noise floor is about 0.44 with eight frames.** Measured across
  eight empty frequencies on one evening: 0.42–0.46. So `s4285_fold.py`'s own
  0.30 significance threshold reports noise as a lock, and a real signal is 0.70
  and up. Treat anything under ~0.55 as no signal.

And two things a fold cannot do: it does not prove the *layout* (an idle station
folds perfectly and shows one constant symbol), and a **phase ramp across the
frame** is a centre error rather than a carrier offset — the fold searches ±75 Hz
per frame and cannot absorb it, which is why the Toulon capture needed +636 Hz of
ramp correction before anything was coherent.

## The waveform, in the numbers a receiver needs

Full detail and provenance in [`STANAG4285-EDITION1.md`](STANAG4285-EDITION1.md);
this is the working set.

| | |
|---|---|
| sub-carrier | 1800 Hz, 2400 baud always, phases at n·π/4, symbol `n` = e^(jnπ/4) |
| occupied bandwidth | 3000 Hz after filtering |
| frame | 256 symbols = 106.6 ms: **80 sync + 48 reference + 128 data** |
| data blocks | 4 × 32 data; the first three are each followed by 16 reference symbols; the last has none |
| reference symbols | **all symbol 0** |
| sync | 80 symbols, 2PSK, PN of length 31 (2 periods + the first 18), generator **x⁵ + x² + 1**, register set to **`11010`** each frame, **first symbol is the LSB**, then 79 clocks |
| scrambling | **x⁹ + x⁴ + 1**, set to 1 each frame, **3 bits per symbol**, `n = 4x₂ + 2x₁ + x₀`, `Bk = e^(jnπ/4)`, register shifted **three** positions per symbol |
| scrambling applies to | reference **and** data symbols — **never** the sync |
| receiver tolerance | **±75 Hz** carrier, up to **3.5 Hz/s** of rate |

| data rate | constellation | coding |
|---|---|---|
| 3600 | 8PSK | none |
| 2400 | QPSK | none |
| 1200 | BPSK | none |
| 2400 | 8PSK | 2/3 |
| 1200 | QPSK | 1/2 |
| 600 / 300 / 150 / 75 | BPSK | 1/2, 1/4, 1/8, 1/16 |

Transcoding, oldest bit first: 1200 — `0 → 0`, `1 → 4`. 2400 — `00 → 0`, `01 → 2`,
`10 → 6`, `11 → 4`. 3600 — `000 → 1`, `001 → 0`, `010 → 2`, `011 → 3`, `100 → 6`,
`101 → 7`, `110 → 5`, `111 → 4`.

**The three uncoded rates need no FEC at all, and they are the first target.** The
coded rates are Annex E, which is in the scan and not yet transcribed.

## The traps, each of which cost a round

* **A mixer without a filter corrupts the decision.** Mixing to baseband leaves
  the mirror at −2·centre, inside the resampled band; it cost about half the
  symbols before the filter went straight after the mixer.
* **The reference symbols cannot resolve the sampling instant.** Sixteen symbols
  in a row with the same phase score equally well anywhere across them, so a frame
  search finds the *boundary*. Timing has to come from the data, by how tightly it
  lands on constellation points.
* **The frame index is ambiguous by design.** Every frame carries the same
  references, so a search finds *a* frame, not frame zero. What has to match is the
  data.
* **Linear interpolation is not a resampler.** The probe gives up past a few
  hundred ppm of clock error (decision error roughly doubles); a real receiver
  wants polyphase or cubic.
* **Scrambling hides the constellation.** The data symbols are uniformly spread
  over all eight phases *until* they are descrambled, so a rate/constellation test
  has to run after descrambling, on the data symbols only.
* **An idle station folds perfectly and proves nothing.** Check for it: one tight
  cluster with a flat magnitude is an idle carrier, not a frame.

## The licence boundary

* The standard is **NATO Unclassified** and we implement from it. Its **text is
  not redistributed**: `STANAG4285-EDITION1.md` is our own transcription of the
  functional parameters, and the 68-page scan stays out of the tree.
* **No third-party recordings in the tree.** MultiPSK's and SkySweep Pro's files
  are demos from closed products; they are used locally, never committed, and
  their known text is recorded in the probe's README instead.
* **Do not read, port or reimplement G4GUO's KiwiSDR extension**
  (`extensions/s4285/`) — it still says `// (License TBD)` and may not be copied.
  Nothing needs it: the standard is public.
* **Do not use Sigmira as an oracle**, and do not ship its bundled example
  recording: its EULA forbids using its results "for your own competing software
  development activities".

## The bench capture, which is the operator's part

The one thing a session cannot do for itself. The French Navy runs **unencrypted
test tapes** — RYRY loops and a station ID — on these frequencies, in USB, ~2.75
kHz wide, read at 1800 Hz:

| station | place | kHz | heard (UTC) |
|---|---|---|---|
| **FUF** | Fort-de-France | 13031.2 / 8478.5 / 4232.0 | 1846 / 1203 / 0146 |
| **FUV** | Djibouti | 8467.8 | — |
| **FUO** | Toulon | 5215.6 | — |
| **6WW** | Dakar | 12857.0 / 13411.9 | — |
| **FUG** | La Régine | 12873.9 | — |
| **FUE** | Brest | 6348.0 | — |

```sh
cd tools/rsp1-capture && cargo build --release
./target/release/rspcap 5215600 48000 90 /tmp/opencode/fuo-5215.raw 45
```

`rspcap` takes **Hz**, and writes CF32 interleaved. Note the UTC time and the
station; the ID is the known part. A minute and a half is enough, and if the
station is idle the fold will say so — 0.70 with one tight cluster means "try
again later", not "the decoder is broken".

**What the tape sends, and this is the gate.** Two independent people running
decoders on these transmissions report the same content:

> `VOYEZ VOUS LE BRICK GEANT QUE J EXAMINE PRES DU GRAND WHARF`

— the French quick-brown-fox, repeated, with `RYRYRY` and `SGSGSG` as diddles, an
`ALL DE FUE … TESTING` opener and an ID in the shape `FI DE FUE … MCI COOP … AR`,
closing `NNNN`. **Baudot is upper case only and has no accents**, so match `GEANT`,
not `GÉANT`, and expect the spacing to vary between tapes.

**If the RSP1 cannot hear it, a receiver closer to the transmitter can.** The tree
already speaks **KiwiSDR** (`Backend::KiwiSdr`), and a Kiwi's stream is ~12 kHz —
the modem's own rate — so this is a first-class route rather than a workaround:

```sh
KIWI_HOST=f6abj-kiwihf.ddns.net:8073 KIWI_FREQ=6348200 KIWI_SECS=10 \
  cargo run -p sdroxide-kiwisdr --example probe
```

That prints the receiver's identity, its rate, its S-meter and its waterfall's
strongest bins, and it says plainly when a host refuses non-browser apps — two
French receivers did on 2026-10-21 while a third streamed. Then add it as a radio
(`Backend::KiwiSdr`, that host) and record with the program's own recorder.

**But "closer" is not automatically better, and 6 MHz is the reason.** At night the
F-layer critical frequency drops below 6 MHz over Europe, so a path of a few hundred
kilometres from Issoudun can sit *inside* the skip zone while a longer one works —
which is consistent with the RSP1 measuring Rayleigh noise at 20:10 UTC on the day
of the sweep. The receiver that wins is the one whose waterfall shows a 2.75 kHz
buzzsaw, not the one that is nearest; try a few, at different hours.

A Kiwi's audio is a third party's receiver, like the demo recordings: fine as a
local measurement, never committed.

**The Twente WebSDR is the best *eyes* and cannot be the recorder.** It is a
PA3FWM WebSDR rather than a KiwiSDR, so it is browser-only — no backend here, no
scripted capture. What its band config actually says (read 2026-10-21) is one band
spanning **0–29.16 MHz** on a wide-band receiver: a 29 159.6 kHz sample rate, a
default tuning of 14 579.8 kHz. **Those numbers are in kHz**, and reading the span
as 29 kHz is the mistake this paragraph made first — it concluded the receiver was
parked on 20 m and left the operator looking at nothing. 6348.2 is covered and
tunable.

Use it to decide *whether*: tune **6348.2 USB**, open the filter to ~4 kHz, and the
tape is an unmistakable 2.75 kHz flat-top buzzsaw. **But an idle channel looks
exactly like it** — a flat band across the filter — so confirm on a recording with
`tools/s4285-probe/s4285_verdict.py` rather than by eye.

That is known text on the air, which is exactly what the two shipped fixtures could
not supply — and `FUE Brest` is **Issoudun**, so 6348 kHz is its channel. Reported
rather than verified by us: **the first capture that reads it is the proof.**

**Do not rip the demo videos either.** There are several on YouTube of MultiPSK and
Sorcerer decoding a French Navy tape, and they are somebody's re-encoded screen
capture of a 2018 transmission — not the RF, and not ours. The recording to make is
our own, off the bench.

---

## The build brief

**The prompt is now its own file: [`STANAG4285-PROMPT.md`](STANAG4285-PROMPT.md).**
It is a self-contained brief to paste into a session that owns `sdroxide-dsp` —
the mission, the collision map against the PSK session working in the same crate,
the API, the six steps, the gates and the traps. It began as a section of this
file; it moved so a session can be handed it whole without the history around it.

## Watching for it overnight

Nobody can sit on 6348 kHz until the tape starts, and the tapes are intermittent:
`tools/s4285-watch/` does it instead. It holds the radio, sweeps a list of
frequencies, and scores **the 106.67 ms frame period** on each — the sync is sent
every frame, so the frame is there whether the station is sending text, a test
tape, or idling, and noise does not have one. On a match it writes the dwell it
holds plus thirty seconds as a WAV and moves on; when nothing is there, nothing is
written.

```sh
cd tools/s4285-watch && cargo build --release
./target/release/s4285watch 6348000,5215600,5155000,4232000,12857000 62500 /tmp/opencode/s4285
```

Ask for **62500**, not 48000 — an RSP1 refuses 48 kHz and silently keeps its own
2 Msps, which overflows the read buffer. It prints a line per frequency, so a
night that finds nothing leaves a log that says so.

`s4285-watch/README.md` carries the frequency table, the thresholds and the
caveats; `s4285-probe/s4285_verdict.py` reads a file back and says whether it is a
tape or an idle carrier. **Not yet proved on a real tape** — that is what the next
watch is for.

## The first night's watch (2026-10-21/22)

The watcher ran from 22:17Z to 04:08Z on ten frequencies and wrote **nine
captures**. One of them is the material this file has been asking for.

| capture | what it is |
|---|---|
| **8467.800 kHz, 22:53Z** | **the frame is there**: |ACF| at 106.67 ms **0.347** against a null of 0.015 (22.7x), 8PSK carrier line **47 dB**, and a visible signal across 1–4 kHz with the fading and drift a night-time HF path has |
| 5215.600 kHz, 02:45Z | frame 0.427 but the null is 0.173, so 2.5x — a carrier rather than a frame; the spectrogram is flat |
| 4232.000 kHz x3 | 8PSK carrier **47 dB** with **no** 106.67 ms frame (0.03–0.05): a PSK signal of some other shape, not Edition 1 |
| 6348.000 kHz x4 | a flat mess across the whole 6 kHz band with a 100 ms stripe — **the capture was at fault, not the band** |

Two things learned, both now fixed or recorded:

* **The WAV writer aliased.** It decimated by averaging groups of five samples, a
  box filter whose stopband is only about −13 dB, so everything strong in the
  rest of the 62.5 kHz band folded into the audio. That is what the 6348 files
  are: a 3 kHz mode that arrives as 6 kHz of junk, with a 10 Hz stripe from
  whatever was strong elsewhere. It now runs a 127-tap windowed-sinc low-pass
  first. **A capture that looks impossible is worth suspecting the instrument
  over the band** — the same lesson as the probe's 0.900 sync.
* **`s4285_lock.py` will not call this a frame, and it may be right to wait.**
  On the 8467.8 capture it reports fold 0.483, reference concentration R = 0.378
  against a 0.12 noise ceiling, and then "no frame in hand". The reference
  symbols are 16 per block and repeat three times a frame, so on a rotating
  channel they collapse *per block*, not onto one point — the single-point test
  may simply be too strict for a real night-time path. That is the next thing to
  measure, and it is measurable now.

The gain is 45 on this build, which is **attenuation-ordered** (35 is louder than
50), and with the 1:49 EFHW the peaks sit about 8 dB below full scale there.

## The keepers (2026-10-22)

The night produced 35 captures. **22 are usable**, kept in
`~/s4285-captures/keepers/` with a `MANIFEST.md` beside them — off-air material, so
out of the tree, the same rule as every other recording here.

The test is the frame ACF at 106.67 ms: **at least 0.25 and at least 15x its own
null**. It was chosen because the frame period is *carrier-offset independent*, so
it is the one number that can be compared across captures with different tunings and
different paths.

| frequency | kept | frame ACF |
|---|---|---|
| 6348.000 kHz | 16 | 0.26 – 0.51 |
| 8467.800 kHz (FUV) | 3 | 0.34 – 0.40 |
| 8478.500 kHz (FUF) | 2 | 0.33 – 0.37 |

**The best single file is `6348.000kHz-20261022-050450Z.wav`** — frame 0.505 at 30x
its null, and its spectrogram is the clean 2.6 kHz plateau a 4285 should have, with
none of the aliasing the pre-04:11 files still carry.

Three rejections worth keeping, because each is a different lesson:

* **5215.6 kHz had the highest frame ACF of the night, 0.554, and was rejected** —
  only 2.4x its null, because a steady carrier autocorrelates at every lag. A high
  number is not a frame; the ratio is.
* **4232.0 kHz carries a 47 dB PSK carrier and no frame at all** (0.06 – 0.09). A
  strong carrier is not a 4285, which is the same lesson the 80/256 rule gives from
  the other side.
* **The four 6348 captures from before 04:11 are the aliased ones** — the box
  decimation, not the band.

Read any of them with `tools/s4285-probe/s4285_verdict.py`, and prefer these over
the demo recordings: they are ours, they are on the air, and the frame is measured.

## 2026-10-22 — the receiver exists, and it locks on both the signal and the control

`crates/sdroxide-dsp/src/stanag4285.rs` (new, receive-only, uncoded rates only) and
`examples/s4285_probe.rs`, with the standard's two generators transcribed:
PN31 from `x⁵ + x² + 1` at `11010` (80 symbols, LSB first) and the 176-symbol
scrambler from `x⁹ + x⁴ + 1`, three bits per symbol, `Bk = e^(jnπ/4)`. Front end
mixes down, **filters at the input rate**, then resamples to 5 samples/symbol;
detection is differential and a frame is only accepted when the correlation peak
**recurs 256 symbols later**.

**It locks on the real capture** — `8467.800kHz-20261021-225334Z.wav` at 1800 Hz
gives sync **0.787**, offset −0.9 Hz, 70 frames. And **it locks on the negative
control too**: `4232.000kHz-20261021-232139Z.wav` gives sync 0.461. So the
recurrence test as written does not discriminate, and by the handover's own
standard (frame ACF 0.347 against 0.03) it is not yet a detector.

Two things the numbers already say about why, both worth picking up first:

* **The reference concentrations are no higher on the real capture** (0.26 / 0.47
  / 0.35) than on the control (0.39 / 0.29 / 0.47), and the README's explanation
  is the missing piece: a capture carrying a *ramp* smears each 16-symbol block,
  so the concentration has to be measured **with a phase-ramp correction inside
  the block**, which this does not do yet. Without it, three numbers that should
  be 0.7–0.9 sit at 0.3 on both files.
* The recurrence gate is a ratio against a creeping floor and 0.461 passes it. The
  discriminator that was measured to work is the **frame-period ACF**, reported
  beside the sync correlation — 0.347 on the signal, ~0.03 on the control — and
  the lock should be gated on that, not on the correlation alone.

Text is decoded through `BaudotRx` and is nonsense on both, which is the expected
consequence of the concentrations above; the raw bits are on the frame event and
`bits()` so a mis-decode is visible as bits rather than as silence.

### Later the same day — the lock discriminates, and both generators were wrong

Two fixes, and the second is the one that mattered.

**The generators had one stage too many.** `x⁵ + x² + 1` is **five** stages and
`x⁹ + x⁴ + 1` is **nine** — not "highest tap + 1". With ten stages the scrambler
emptied out and stayed at zero, so it was a constant; with six, the sync sequence
had period 16 and stopped being PN31. Nothing failed loudly: the degenerate sync
correlated *better* (0.814 against 0.547 for the true one) and the references
simply never collapsed, which read as "no frame here". `the_generators_are_maximal_
and_match_the_standard` now pins the 31-symbol period, the first ten sync symbols
and a near-uniform scrambler, so it cannot happen quietly again.

**The lock is gated on the frame ACF**, not on the sync correlation: 80 of a
frame's 256 symbols repeat, so a real 4285 measures about 0.31 and another mode
about 0.03. Both are reported whether or not a lock is accepted, because a file
that does not lock has to say why.

Measured after both fixes, on the two files the handover named:

| | frame ACF | lock | sync | reference blocks |
|---|---|---|---|---|
| **8467.800 kHz** (FUV Djibouti) | **0.400** | yes, 163 frames | 0.547 | 0.485 / 0.281 / 0.299 |
| **4232.000 kHz** (control) | **0.042** | **no** | — | — |

The ACF reproduces the two documented figures (0.347 on this file, ~0.03 on the
control) closely enough to be the same measurement, and the control is now
refused. The offset estimate lands at −106 Hz, near the edge of the ±150 Hz
search, and the reference concentrations are 0.28–0.49 where a clean lock should
give 0.7–0.9 — so the **frame phase and the block alignment are the next thing to
check**, not the generators. Text is still nonsense, which follows from the
concentrations: the payload cannot be right if the references do not collapse.

### The night's captures, measured with the receiver (2026-10-22)

Thirty-five files in `~/s4285-captures/`, and **27 lock**. Frame ACF, best first:
`6348.000kHz-…045545Z` **0.588**, `…054415Z` 0.512, `…051325Z` 0.476,
`…051457Z` 0.430, `…051154Z` 0.422 (sync 0.911 is on `…051457Z`, and a reference
block already reaches 0.715 on `…051154Z`), `8467.800-…225334Z` 0.400,
`8478.500-…063227Z` 0.395, `5215.600-…024501Z` 0.392.

**Eight refuse**: the five `4232.000kHz` files (ACF 0.012–0.072), three `6348.000kHz`
files at 0317Z/0336Z/0342Z (0.020/0.047/0.007), and `6348.000kHz-…032809Z` at
**0.140** — just inside the 0.12 gate, so it is the best test the threshold has:
either a fading 4285 the gate rejects, or the other mode the gate nearly accepts.

So the material for reading text has been on the disk since last night, and the
note that the 6348 captures were "a flat mess" is history — that was the WAV
writer, before it was filtered. The reference collapse (0.2–0.7, needing 0.7–0.9)
is the remaining fault, and it is in the code.

## 2026-10-22, second session — the references collapse, and the payload is not text

The reference blocks now collapse: **0.64 / 0.75 / 0.62 averaged over 215 frames**
on `6348.000kHz-20261022-045545Z.wav`, against 0.24 / 0.63 / 0.22 from one frame of
the same file before. Two faults, both in acquisition, and neither was the
generators or the block layout.

### The fold, not the peak

The synchronisation repeats every frame, so the correlation is a **periodic**
statistic and the right question is "at which of the 256 symbol phases does this
average over *many* frames stand up". The old code took one frame's maximum over
every offset as the lock and its position as the frame head; measured, the
folded statistic reads **0.85** on a real capture against a median of **0.104**
over the 256 phases, and **0.14** on the 4232 kHz control (its own null 0.13, so
no margin at all). A single peak, by contrast, reaches 0.738 on white noise —
which is why the old lock could not discriminate.

The provenances that came with the fold, and both are measured:

* **an off-by-one in the fold.** `corr[k]` covers symbols `k-79 ..= k`, so the
  frame head is `k-79`, not `k`. Folding on the wrong index puts every block half
  a symbol out.
* **the sampling phase.** Ten sub-symbol phases are searched, and each frame
  refines its own within ±0.2. On the Djibouti capture the best phase moved the
  sync correlation from 0.71 to **0.94** — the receiver was sampling anywhere
  inside the symbol, and a tenth of a symbol is 42 µs.

### The carrier offset has to come off the symbols, not just the score

The ramp search already found **+55 Hz** on the 6348 captures, but the symbols
handed to the channel estimator still carried it. `+55 Hz` turns a whole circle
every 48 symbols, so the three reference blocks are ~2π apart and the linear fit
across the frame is meaningless without unwrapping. Applying the chosen ramp to
`z` before reading the block phases is what turns the constellation from uniform
into a readable BPSK one.

### The layout was right, and it was measured rather than trusted

Sweeping the reference-block offset over ±3 symbols gives **0.702 at offset 0**
against 0.649 at −1 and 0.684 at +1, so `32 data + 16 reference` three times then
32 data is exactly where the standard puts it.

### The night's 35 captures, measured with the fixed receiver

35 files, **16 lock**, and every one of the five `4232.000 kHz` controls refuses
(ACF 0.024–0.108, folded sync 0.00). Best first by folded sync:

| capture | folded sync | null | frame ACF | refs (mean) | rate verdict |
|---|---|---|---|---|---|
| `5215.600-…024501Z` | **0.966** | 0.090 | 0.485 | 0.71/0.79/0.69 | unknown |
| `6348.000-…050318Z` | 0.875 | 0.101 | 0.520 | 0.65/0.76/0.62 | 1200 BPSK |
| `6348.000-…051154Z` | 0.851 | 0.104 | 0.591 | 0.67/0.78/0.63 | 1200 BPSK |
| `6348.000-…045313Z` | 0.842 | 0.103 | 0.604 | 0.65/0.76/0.62 | 1200 BPSK |
| `6348.000-…045545Z` | 0.793 | 0.105 | 0.502 | 0.64/0.75/0.62 | 1200 BPSK |
| `8467.800-…225334Z` | 0.613 | 0.096 | 0.524 | 0.53/0.40/0.49 | unknown |

`5215.6 kHz` — the file the first night called "a carrier rather than a frame" —
is the **strongest** lock of the set at 0.966, so that verdict was another
consequence of the single-peak lock.

### The payload is not plain ITA2, and this is the day's hard finding

Every one of those locks produces bits, and none of them produces text:

* The constellation is **BPSK**: the 2-, 4- and 8-fold concentrations are
  typically 0.19–0.37 / 0.02–0.10 / 0.04–0.10. A 4- or 8-point lattice would show
  its own order dominating; it does not.
* The bits are **statistically random**: the 5-bit histogram is uniform
  (min 170, max 350 against 242 expected) at every alignment, the bit transition
  rate is 0.49, and there is no periodicity at 5, 10, 16, 31, 32, 64 or 128.
* One file (`…050318Z`) is **82 % ones** frame after frame, which is an idle
  transmitter sending the all-ones idle character — exactly what the ZL2AFP
  receiver's own help says these transmissions are: *"Invariably the traffic is
  encrypted, except perhaps during test transmissions."*

So a lock is "a 4285 is on this frequency", and the night's captures carry either
encrypted traffic or an idle carrier. **The material for reading text was not on
the disk**, and the previous session's inference from "27 lock" to "the material
to read text is in hand" was the mistake.

Two routes to text remain, and neither is today's:

1. **Annex E** (error correction, interleaving, protocols). Every uncoded rate
   shares its constellation with a coded one — 1200 BPSK with the 75–600 bps
   rates at 1/16…1/2 — and a test tape is normally sent at **75 baud, rate
   1/16**. A constellation can never tell those apart, and the coded payload is
   not readable without the FEC. Annex E is in the scan (`/tmp/opencode/
   s4285_full.pdf`, Annex C ends at PDF page 30, Annex E-3-3 is page 64) and is
   **not transcribed**.
2. **A capture of a tape transmitted uncoded**, if such a thing is still sent.

The panel says the honest thing rather than pretending: `1200 BPSK uncoded, or a
coded BPSK rate`, and the bit stream is exposed so a wrong demodulation shows as
bits rather than as plausible letters.

### The mode, wired

`Mode::Stanag4285` (discriminant 59) as a receive-only text modem, `PROTO_VERSION`
226 → **227** / `VERSION_BYTE` 0x2E → **0x2F**, with `DigiStatus::stanag`
appended: the folded sync and its null, the frame ACF, the three reference
concentrations, the 2/4/8 constellation concentrations, the rate string, the
offset and the frame count. The panel draws them as a wrapping row above the text
pane, each with a hover saying what the number means and what it does not. The
transmit half refuses with the reason (no modulator; the coded rates need Annex
E), the panadapter marks the standard's 3000 Hz footprint, and the SW band accepts
it.

Tests: a frame-accurate synthetic BPSK transmitter (raised cosine, 30 % excess
bandwidth) that must lock with all three references above 0.85; noise and a plain
carrier that must **not** lock; the off-air gate
`SDROXIDE_S4285_SAMPLE=… cargo test -p sdroxide-dsp --release -- --ignored`
which requires the mean references above 0.55; and the controller test that the
report reaches `DigiStatus` and the refusal names Annex E.

**Two traps met while testing, both in the fixture rather than the receiver:** a
multiplicative-hash bit generator has period 256 here and makes frame *n+1* the
exact complement of frame *n*, which cancels the sync in the frame ACF; and
`TAU * f * i` evaluated in f32 loses its fractional part within a second, so the
synthetic carrier landed at 1200 Hz and nothing could lock.

### Still open

* **Annex E**, for the coded rates — the only route to a test tape's text.
* A **single-peak lock is still used once**, inside the acquisition window, to
  pin the frame head after the fold has found its phase; the fold's index is a
  symbol-resolution answer and the per-frame timing search does the rest.
* The **carrier offset is searched per frame**, ±150 Hz in 1 Hz steps after a
  5 Hz sweep. It has never been outside ±130 Hz on these captures, but a station
  further off the dial would still read as no frame.

## 2026-10-22, third session — Annex E, transcribed and built

**The operator's question first, because it is the one that decides whether any of
this is ours to build: no, Annex E is not cryptography.** Its first page is a
constraint-length-7 rate-1/2 convolutional encoder with the published 133/171
polynomials, followed by a convolutional interleaver and a SOM/EOM message
convention — the same class of forward error correction that is in DVB, Wi-Fi and
deep-space telemetry. Encryption on these links is a separate layer the users
apply above the modem; the standard does not describe it and nothing here touches
it. The full transcription is in [`STANAG4285-EDITION1.md`](STANAG4285-EDITION1.md),
"Annex E".

### What was transcribed (PDF pages 50–64)

The functional content is E-1 … E-6 plus Appendix 1 (E-1-1, E-1-2); Appendices 2
and 3 are bit-error-rate tables. In brief:

* **Rate 1/2, K=7, G1=133, G2=171 octal, T1 first.** Below 600 bps the *pairs* of
  output bits are repeated 2, 4 and 8 times — never the first bit repeated and
  then the second.
* **A BPSK (1200 channel bit) transmission is therefore 600, 300, 150 or 75 bps**
  of data, and the constellation cannot tell which. 1200 bps data is QPSK and 2400
  is punctured 8-PSK.
* **Convolutional interleaver, 32 rows**, row `r` holding `r·j` entries and the
  deinterleaver's holding `(I-1-r)·j` — which is what makes the pair a constant
  delay. Input commutator `n·9 mod 32`; output commutator normal, except 24 of the
  32 rows at 2400 bps.
* **SOM `03873C3C`, EOM `4B65A5B2`**, then `delay + 102` zeros to flush.
* **Soft-decision Viterbi**, soft value = distance from the decision boundary,
  positive for a zero; the deinterleaver pre-filled with erasures; 4/8/16 soft
  values averaged in even and odd halves for the three lowest rates.

### What was built

`crates/sdroxide-dsp/src/stanag4285_fec.rs`: the encoder, the pair repeater and
averager, the interleaver pair, the Viterbi decoder and the SOM/EOM framer, with
an ITA2 reader for the message (`message_text`, both synchronous groups and
5N1.5 framing). Wired into the receiver: every frame above a **0.30** reference
concentration feeds 128 soft decisions into eight branches — four BPSK rates × both
signs, because the frame phase is known only up to ±π until a SOM says otherwise —
and the first branch to frame a message names the rate.

**Verified end to end on a loopback**: SOM + 128-bit payload + EOM through the
encoder, the pair repeater, the interleaver, BPSK modulation, the real receiver,
the deinterleaver, the Viterbi decoder and the framer, and the payload comes back
bit-exact with the right rate
(`a_coded_message_survives_the_whole_chain`). A loopback cannot prove the
interleaver's structure against the standard — both halves share whatever
convention they use — which is why the SOM on real traffic is the real test.

### Two faults found in the writing, both worth keeping

* **The Viterbi traceback cannot remember the input bit alone.** This code's state
  transition drops the oldest bit, so the received state and the input bit leave
  *two* possible predecessors; the first version picked the first match and decoded
  a clean signal 30 % wrong. The traceback now stores the previous state.
* **The interleaver's two halves are not the same shape.** Given both rows as
  `r·j`, the pair delays by `2r·j` and returns a permutation, not the message. The
  deinterleaver's rows are `(I-1-r)·j`, which is what the annex says in its own
  words ("j fewer bits of storage than the preceding one") and what makes the total
  `(I-1)·j` constant. Measured after the fix: one constant delay of 992 for j=1.
* And two in the fixtures: a multiplicative hash has period 256 and makes frame
  *n+1* the complement of frame *n*; and `TAU*f*i` in f32 loses its fractional part
  within a second, putting a synthetic carrier at 1200 Hz.

### Bench, 2026-10-22 daytime — the wire antenna is back, and the finding is the instrument

The operator reconnected the wire antenna to the RSP1 at 11:38 UTC, so the watcher
was run over eight frequencies. **18 captures of a 100 ms periodic signal at
12.8578 MHz**, and *none of them is a 4285*:

* the frame period in the file is **100.00 ms**, not 106.67 — exactly 15/16 of it;
* rewritten with a 11719 Hz header the ACF moves to 0.36, but the folded sync is
  only 0.42 against a null of 0.24 and the lock refuses;
* **no 2400 baud line anywhere**: the squared envelope has its strongest line at
  1825 Hz, and the night capture measured with the same method shows a clean
  2400.0 Hz line, so the method is sound;
* the spectrum peaks at 2700 Hz and spans 0..3841 Hz, and the rms is 0.0032
  against 0.017 for a night capture — 15 dB weaker.

So either the capture rate is wrong by 15/16 or the signal is not a 4285, and the
two are told apart by the baud line, which says not-4285. **The watcher's trigger
is a frame-period autocorrelation and it will happily trigger on any 100 ms
periodicity**, which is the same lesson as "a strong carrier is not a 4285" wearing
a new coat. Not fixed here: the next session should record a known 4285 (6348 kHz
at night) through the same path and confirm the rate read-back before trusting a
daytime trigger.

Also recorded: one stale note in the code said the frame ACF was taken over one
frame's worth of baseband. One frame of a modulated signal is 1280 samples of a
statistic that needs many frames — it read **0.06 where the whole buffer read
0.355**, the instrument rather than the signal. It now averages over everything
buffered.

### Are the captures coded at all? Measured: no

`examples/s4285_fec_check.rs` answers the question the constellation cannot, and
does it with its own null. Every uncoded rate shares a modulation with a coded
one, so the only evidence is whether the stream satisfies the code's trellis after
deinterleaving. The statistic is the decoder's metric against **the same values
shuffled** — shuffling destroys the trellis and nothing else.

Every branch on every night capture:

| capture | 600 (1/2) | 300 (1/4) | 150 (1/8) | 75 (1/16) |
|---|---|---|---|---|
| `6348-…045545Z` | **0.95×** | 0.98× | 0.98× | 0.99× |
| `6348-…050318Z` | **0.97×** | 0.98× | — | — |
| `5215.6-…024501Z` | **0.97×** | 0.97× | — | — |

A coded stream beats its null by a wide margin (`the_decoder_metric_separates_a_coded_stream_from_a_shuffled_one`
pins that on a synthetic one). Ratios of 0.95–0.99 are the null.

**So the night's captures are uncoded 1200 bps** — Annex A's own rate, not one of
Annex E's — carrying encrypted traffic or an idle line. Nothing about the coded
chain is implicated: it has never seen a coded transmission. When one turns up, the
branch that is right will stand well above its null, and the probe is the tool that
says so.

### The bench loopback, 2026-10-22 afternoon — the chain reads a file, not the rig

With the RSP1 back on an antenna and the bench rig on **27.415 USB at 2 W**,
`examples/s4285_test_signal.rs` generates an Annex E coded transmission as a WAV
(lead-in, SOM, ITA2 text, EOM, flush), `aplay` into the rig keys it by VOX, and
`rspcap` records it off the air. Four transmissions were made.

**Through a file the chain is exact.** Fed straight to the receiver, the same WAV
gives `RYRY TEST RYRY`, one message, `SOM 32/32`, references 0.998/0.997/0.998,
and the 112 message bits match the encoder's byte for byte. A 280-bit message does
too. That is the coded chain proved end to end — code, pair repetition,
interleaver, framing and the ITA2 reading.

**Over the air it does not decode, and this is where it stands.** Measured on the
recording:

* the frame period is **106.72 ms → 2400 baud** ✓, so the waveform and the symbol
  clock survive the rig;
* the three **reference blocks read 0.87–0.91**, and stay there across 300 Hz of
  carrier, which is the trap this file already records: sixteen identical symbols
  have no ISI to suffer, so they collapse whether or not the *data* is readable;
* the **data does not**: the 2-fold concentration sits near 0.25 where a clean
  BPSK frame reads 0.9, and the decoder's metric is 0.94–0.99 of its own shuffled
  null, i.e. no trellis structure at all;
* the rig's audio path is **not flat** — the received spectrum runs 400–3050 Hz
  with a 6–7 dB tilt — and its carrier sits **+221 Hz** off the RSP1's dial,
  measured with two tones (800→1021, 1600→1821), which is outside the receiver's
  ±150 Hz ramp search.

**And the rig's own manual answers it.** The CRT SS9900V's specifications give
**SSB frequency response 450 to 2500 Hz** — 2.05 kHz of transmit passband, against
a 2400-baud signal that occupies about 2.4–2.7 kHz and a standard that specifies
3000 Hz. So the waveform is truncated at both edges before it reaches the
antenna, which is ISI by construction, and ISI is invisible in a run of sixteen
identical symbols and fatal to the data. That is the measurement the three
symptoms above were pointing at, and it is not a fault in this receiver.

Two menu items in the same manual may help and are worth trying before any code
is written: **15 `TONE` — "Choose SSB TX bandwidth"**, and **`NRC`**, a *TX/RX*
noise-reduction processor whose own description says it is for speech. A data
signal wants the widest `TONE` and `NRC` off.

The +221 Hz carrier offset is real as well (two tones, 800→1021 and 1600→1821) and
is inside a real receiver's business — ±150 Hz is a search range this receiver
chose, not a limit of the standard, which allows ±75 Hz of carrier error *from the
transmitter*. A bench path 221 Hz off is the tuning, not the modem.

### A real bug the bench found: the decoder's chunk overlap was on the wrong end

The Viterbi is run over fixed chunks, each decoded with every metric on state zero
and traced back from the end. The bits the forced start distorts are therefore the
**opening** of a chunk, and the first version used exactly those — the overlap
was placed behind the chunk instead of in front of it. On the generated file that
put a burst of errors every 256 bits: a 112-bit message came back with bytes 3–4
wrong and everything else right, which looks like a bad demodulator and is the
decoder's own plumbing. With the overlap in front and the opening discarded, the
same file decodes exactly.

That is the sort of fault a loopback is for: it is invisible in the text (the
message is short), invisible in the references (0.998), and only shows when the
decoded bits are compared with the bits that were sent.

### Four more faults the bench found, all in the receiver

Chasing the air recording turned up four things that are wrong on any path and
invisible on a clean file, and each is now fixed:

* **the coded soft decisions ignored the channel line.** `take_frame` derotated
  the data by the per-block phase estimated from the three reference blocks; the
  Annex E chains were fed soft decisions derotated by the *ramp only*. On a clean
  file the block phases are equal and the two agree; on any real path they are
  not, so every data block after the first arrived rotated by whatever the channel
  moved by — **50 % wrong bits with the reference blocks reading 0.95**, which is
  exactly what the bench showed and what no amount of staring at the constellation
  would explain. Both paths now use one `channel_line`.
* **the symbol clock was not tracked.** The frame stride was a fixed 256 symbols.
  Two sound cards' clocks differ by far more than the standard's 10 ppm — measured
  at ~500 ppm here, one symbol every eight frames — so the receiver walked out of
  its own timing window and re-acquired every few frames. The best sampling phase
  inside a frame is now the error signal that nudges the stride, with the error
  taken **modulo one symbol**: read literally, an error of −0.6 walks the head the
  wrong way every frame and the timing runs away.
* **a frame too poor to report dropped the lock**, which restarted the coded
  chains — on a fading path that means never completing a message. A poor frame
  still proves the frame *position*, so it keeps the lock and feeds 128 erasures.
* **the phase is chosen from the data, and the channel line comes first.** A
  concentration measured on the raw data is dominated by the block rotation, so
  the sweep answered noise.

The off-air gate still passes (references 0.61/0.70/0.57 over 287 frames) and the
generated file still decodes bit-exactly.

### The timing came from the wrong place, and that is now fixed

A second fault, found while chasing the air capture and of exactly the shape this
file already warns about: **the reference symbols cannot resolve the sampling
instant.** Sixteen identical symbols score equally well anywhere across them, so a
per-frame phase search that maximises the *reference* concentration is flat in
time — it cannot tell a correct sampling instant from half a symbol out. On a
clean file it lands well by luck; on a real path a frame sampled half a symbol
late has every data symbol smeared while its reference blocks still read 0.9,
which is precisely what the air recordings showed.

The phase is now chosen from the **data**: once the sign and carrier offset are
known, the frame is re-sampled across ±0.5 symbol and the one that lands the data
symbols most tightly on a constellation wins (the best of the 2-, 4- and 8-point
concentrations, so it works whatever the order). The generated file still decodes
exactly, and its data concentration rose to **0.993**.

On the air capture the same change takes the data from 0.25 to **0.384** — better,
still not the ~0.9 a clean frame gives, so the smear is real and the path is doing
it.

### Still open

* **No real coded transmission has been decoded**, and the metric test above says
  the ones on disk are not coded. A coded station is what the branch needs; the
  panel now reports `SOM n/32` so the operator can see one being worked on.
* **The over-the-air loopback**, which needs either an equaliser (Annex C/D — a
  channel estimate at the four reference points, amplitude as well as phase) or a
  bench path with a flatter passband. The rig's transmit filter is the current
  suspect and the measurement above is what says so.
* **The punctured 2/3 rate for 2400 bps is not built**: the annex does not say
  which of each four bits is dropped, and a guessed demapper is indistinguishable
  from a decode.
* **The character layer is a best reading**, not a specification: Annex E calls
  what it carries "message bits" and names no character set. ITA2 5N1.5 is what a
  75 baud tape should be, and the reader refuses a reading that does not look like
  text rather than printing letters that mean nothing.
