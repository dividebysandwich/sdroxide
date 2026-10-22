# STANAG 4285 — the mode, and the brief to build it

**Read [`STANAG4285-EDITION1.md`](STANAG4285-EDITION1.md) first.** It is the
standard itself — Annexes A and F, transcribed from a declassified 68-page
Edition 1 — and it settles the transcoding, both generators, the frame and the
acquisition method. This file is the state of our own work, the measurements that
constrain it, and a prompt for the session that builds the mode.

## Where the work stands

**None of STANAG 4285 is in the program.** There is no `Mode` variant, no
decoder, no panel, no `PROTO_VERSION` entry. What exists is an instrument:
`tools/s4285-probe/` — Python probes, deliberately outside the build, whose
`README.md` is the measurement log. Read it before running anything; it is longer
than this file and it is where the numbers live.

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
