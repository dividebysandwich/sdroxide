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

# The prompt

**Paste everything below into a fresh session.** It is written to be self-contained;
the files it names are in this repository.

## Your job

Build a **receive-only** STANAG 4285 receiver for this program, and stop at the
point where you cannot prove it. It is a 2400-baud single-tone HF modem, and the
standard is transcribed in [`STANAG4285-EDITION1.md`](STANAG4285-EDITION1.md) —
read that first, then this file, then `tools/s4285-probe/README.md`, then
`AGENTS.md` for the house rules.

**Scope cut, and it is deliberate:** the **three uncoded rates only** (3600 8PSK,
2400 QPSK, 1200 BPSK). The coded rates are Annex E — convolutional code,
interleaver, message protocol — which is in the standard's scan and not yet
transcribed. Most traffic on the air is coded, so say so in the panel rather than
pretending: an operator who hears a coded station must be told what it is.

**Order of work, and it is not the usual one.** Do not add a `Mode` variant first.
The decoder is the work and its gate is a recording we may not have; the wiring is
cheap and reversible. So:

* **Stage 1 — the modem, in `crates/sdroxide-dsp/src/stanag4285.rs`**, with a
  committed probe example (`crates/sdroxide-dsp/examples/s4285_probe.rs`, the shape
  of `wefax_probe.rs`). Nothing on the wire, nothing in the UI.
* **Stage 2 — the mode**, only once a real signal reads text through it.

## Stage 1 — the receiver

```rust
pub struct S4285Receiver { /* … */ }

impl S4285Receiver {
    /// `rate_hz` is the audio tap rate; the receiver resamples to 12 kHz itself.
    pub fn new(rate_hz: f64) -> Self;
    pub fn process(&mut self, audio: &[f32]) -> Option<S4285Event>;
    pub fn reset(&mut self);
    /// What the last lock decided, for the panel and the probe.
    pub fn report(&self) -> S4285Report;
}
```

`S4285Event` is one of: `Frame { bits: Vec<bool>, rate: S4285Rate, snr_db: f32 }`,
`Sync { quality: f32, offset_hz: f32 }`, `Lost`. Keep it `pub` and plain — the panel
and the probe both read it.

**Front end.** Resample the tap to **12 kHz** (12000 / 2400 = exactly 5 samples per
symbol, and the 3000 Hz occupied bandwidth fits under Nyquist). Mix 1800 Hz to
baseband and **low-pass immediately after the mixer**, before decimation — the
mirror at −3600 Hz is otherwise inside the band and it corrupts decisions. Per the
standard the low-pass is 1500 Hz.

**Acquisition — differential, not coherent, and the measurement says why.** An
80-symbol coherent integration is 33 ms; a 30 Hz offset — well inside the ±75 Hz
the standard allows — turns a full circle inside it. The probe measured **0.546**
and **0.610** for differential correlation against **0.40** coherent on the same
files, and 0.40 is the sequence's own bias, not a match. So:

1. **Detect** by correlating the differentially-detected symbol stream against the
   differentially-encoded PN31, over ±100 Hz of offset and 0…4 samples of symbol
   timing. Score on the correlation normalised by its own noise floor.
2. **Take the offset** from the phase of that correlation, then refine it
   per frame — the standard's Annex F asks for Doppler from the phase between two
   channel impulse responses in three Doppler channels, and that is the *steady
   state* refinement; differential detection is how you get the first lock.
3. **Detect on an error count**, as Annex F says: count the mismatched sync
   symbols against the reference and accept below a threshold. Report the count —
   it is what tells a panel that a carrier is 4285-shaped but the preamble is a
   variant.

**Frame lock.** Once the sync is found at symbol *p*, frames follow every 256
symbols. Re-verify each frame's sync and track the phase across frames; a frame
that fails twice is a lost lock, and the receiver re-acquires rather than coasting.

**Descramble, then check yourself, then decide.** Take the 176 symbols after the
sync, multiply by the conjugate of the scrambling sequence (x⁹ + x⁴ + 1, register
set to 1 at the frame start, three bits per symbol — **the sync is never
scrambled**), then:

* the 48 reference symbols must land on symbol 0; report their concentration `R`,
  and compare it against the **null measured on the same buffer** rather than a
  constant — 0.529 looked like a frame until the shuffle null came back at 0.601;
* channel amplitude and phase from each 16-symbol reference block, interpolated
  across the 32 data symbols that follow it, which is what absorbs the ±75 Hz and
  the 3.5 Hz/s;
* timing from the data, never from the references.

**Rate and constellation, after descrambling.** The scrambled data is uniform over
all eight phases, so the rate cannot be read before the scrambler comes off. Once
it is off: BPSK uses {0, 4}, QPSK {0, 2, 4, 6}, 8PSK all eight. Decide over ≥8
frames, report the confidence, and be honest about what cannot be told apart —
**all eight is 3600 uncoded *or* any of the coded rates**, which differ only in
Annex E, so the panel says both rather than picking one.

**Output.** The payload is a synchronous bit stream; the character layer is
**ITA2 5N1**, and `sdroxide_dsp::BaudotRx` already does that table — reuse it, do
not write a second. Also expose the raw bit stream, because a transmission that is
not ITA2 must still be visible as bits rather than as silence.

**Proof, and it is the whole stage:**

* `s4285_sim.py`'s round trip ported into Rust unit tests: build a frame from the
  standard, decode it exactly at 30 / 40 / 60 dB. **This is a plumbing test, not an
  air gate** — say so in the test's own doc comment.
* Pin the two generators against literals computed independently of the functions:
  the 80 sync symbols from x⁵ + x² + 1 seeded `11010`, and the 176 scrambling
  symbols from x⁹ + x⁴ + 1 seeded 1.
* On `~/Downloads/sounds/4285.WAV` (12 kHz, 64.7 s): frame-period ACF at
  **1280 samples**, ≥0.5 against the file's own median; differential sync ≥0.5.
  That file's payload **will not decode** — it is a variant or a test tape — and
  the probe must say which test it failed rather than call it a decoder fault.
* **Every searched statistic reports its own null**, measured on the same buffer.

## Stage 2 — the mode, and only when a capture reads

The trigger is concrete: **a recording of a station sending Edition 1 decodes text
through the probe**. Then wire it, in the order the last four modes were wired
(`Mode::Ardop` is the freshest precedent — grep it and copy the shape):

* `crates/sdroxide-types/src/mode.rs` — `Mode::Stanag4285` **appended last**,
  label, `is_digital`, `is_rx_only`, `ALL`/`DIGITAL` membership. Appending it
  **still takes the bump**: the register at v222 says a mode variant alone does
  not move a discriminant, but both ends must agree what the number means.
  `PROTO_VERSION` **226 → 227**, `VERSION_BYTE` **0x2E → 0x2F**, with a register
  entry in `crates/sdroxide-proto/src/lib.rs`.
* No `DigiConfig` field in the first cut: the rate is auto-detected and reported.
  If measurement shows it must be forced, add it **at the tail** and bump again.
* `crates/sdroxide-types/src/band.rs` — the bands it is offered on; no band owns a
  civilian/military data frequency, so it is a general-coverage mode like the other
  utility lanes.
* `crates/sdroxide-digi/src/text_modem.rs` — an `RxModem::S4285` arm, `TxModem::Receive`,
  and the refusal text: *"STANAG 4285 is receive only in this build — the
  transmitter is not written"*. The receiver takes the **raw tap**, as `ArdopRx`
  does: `MODEM_RATE` here is 8 kHz and 8000/2400 is not a whole number of samples
  per symbol, so it resamples to 12 kHz itself and `rx_rs` stays `None`.
* The dialect tables, which every mode touches: `sdroxide-cat` (civ, elad,
  elecraft, flrig, kenwood, qrplabs, rigctld, trusdx, yaesu), `sdroxide-rigctld`,
  `sdroxide-smartsdr`, `sdroxide-tci`, `sdroxide-speech` × 2, and
  `sdroxide-ui/src/app/frame.rs` for the panadapter's frame width.
* `crates/sdroxide-ui/src/app/panels/text_modem.rs` — the panel's receive half, and
  a **status line that states the truth**: rate+constellation it locked,
  sync quality, offset, and whether the traffic is likely *coded* (in which case
  say so and name Annex E). No control that silently does nothing.
* Manual (`docs/USER_MANUAL.md`) and `README.md`'s mode table, as the other modes
  did.

## What not to do

* **Do not touch WEFAX, DAB, SSTV or DSC.** F4NRM and his radio group report all
  four working (2026-10-21); they are out of scope in this phase.
* Do not take a recording out of Sigmira's installer, MultiPSK's, or SkySweep's and
  commit it.
* Do not port G4GUO's KiwiSDR extension (`// (License TBD)`).
* Do not ship a mode whose only evidence is a loopback, and do not claim a decode
  you have not seen.
* Do not push. Commit locally, one complete thing per commit.
* Do not "improve" the probes into the build — they are the instrument and they
  stay outside it.

## Definition of done

Either **a bench capture reads its RYRY and ID and the mode is wired and green**,
or **the receiver locks, reports its diagnostics and the file says exactly which
test failed on which material** — with the probe committed so the next session
starts from a number instead of from this prose. The tape's known text is the
French pangram `VOYEZ VOUS LE BRICK GEANT QUE J EXAMINE PRES DU GRAND WHARF` with
`RYRYRY` and an `FI DE FUE` ID; matching it is what "reads" means. Write it in
`MODE-PROGRESS.md` under STANAG 4285, one line per stage, with the measurement
beside it, and add a "Needs me" line for the capture if there is none.

## Stop conditions

If the fold locks at ≥0.55 with its own null and the payload still does not
descramble, the recording is not Edition 1 — check the sync error count and say
"variant" rather than chasing the decoder. If nothing locks at all, measure the
envelope first: Rayleigh statistics mean there is no signal on that frequency at
that hour, which is the answer, not a fault.

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
