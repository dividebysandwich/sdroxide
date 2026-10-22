# STANAG 4285 — the receiver, in Rust

**Paste this into a session that owns `crates/sdroxide-dsp`.** It is deliberately
narrow: a receiver and a probe example, nothing on the wire and nothing in the UI.

> **This is not to be run in the same checkout as the PSK coded-families work.**
> That session is live in `sdroxide-dsp` and in `MODE-PROGRESS.md`. The collision
> map below is short and it matters: the only shared file is
> `crates/sdroxide-dsp/src/lib.rs`, and a night's work is worth less than a trivial
> merge.

## What you are building

A **receive-only** STANAG 4285 modem in `crates/sdroxide-dsp/src/stanag4285.rs`,
plus a committed probe example that runs it on a WAV. The three **uncoded** data
rates first — 3600 8PSK, 2400 QPSK, 1200 BPSK — because they need no FEC at all.
The coded rates are Annex E and out of scope; say so rather than half-building it.

No `Mode` variant, no panel, no `sdroxide-types` change, no `PROTO_VERSION` bump. A
mode whose only evidence is a loopback is what the house rules forbid, and the
wiring is cheap once a real capture reads.

## Read first, in this order

1. [`STANAG4285-EDITION1.md`](STANAG4285-EDITION1.md) — the standard, transcribed:
   the frame, both generators, the transcoding, Annex F.
2. [`STANAG4285-HANDOVER.md`](STANAG4285-HANDOVER.md) — the state, the capture, and
   the traps that cost the last two days.
3. `tools/s4285-probe/README.md` — the measurements, including the two nulls that
   had to be measured before any statistic could be believed.
4. [`AGENTS.md`](AGENTS.md) — house rules, the bench, and what is out of scope.

## The collision map — read this before you touch anything

The PSK coded-families session is live in this checkout and owns:

`crates/sdroxide-dsp/src/psk.rs` · `crates/sdroxide-dsp/examples/psk_coded_probe.rs`
(untracked, theirs) · `MODE-PROGRESS.md` · `AGENTS.md` · the PSK parts of
`sdroxide-types` and `sdroxide-ui`.

**Yours:** new files only.

| file | what you do |
|---|---|
| `crates/sdroxide-dsp/src/stanag4285.rs` | new — the receiver |
| `crates/sdroxide-dsp/examples/s4285_probe.rs` | new — runs it on a WAV, prints the diagnostics |
| `crates/sdroxide-dsp/src/lib.rs` | **one line**: `pub mod stanag4285;` in alphabetical order. This is the only shared file; keep the diff to that line |
| `STANAG4285-HANDOVER.md` | your journal line, at the end |

Do **not** touch `psk.rs`, `mode.rs`, `digi.rs`, `sdroxide-proto`,
`sdroxide-digi`, the UI, or `MODE-PROGRESS.md`. If you think you need one of them,
stop and say why instead.

## What is already known — do not re-derive it

From the standard, transcribed in `STANAG4285-EDITION1.md`:

* **frame** 256 symbols at 2400 baud = **106.67 ms**: 80 sync, then 176 scrambled
  (four 32-symbol data blocks; the first three are each followed by **16 reference
  symbols**, the last has none). Reference symbols are **all symbol 0** before
  scrambling.
* **sync** 80 symbols, 2PSK, PN31 (two periods plus the first 18), generator
  **x⁵ + x² + 1**, register set to **`11010`** each frame, **first symbol is the
  LSB**, then 79 clocks.
* **scrambling** **x⁹ + x⁴ + 1**, set to 1 each frame, **three bits per symbol**
  (`n = 4x₂ + 2x₁ + x₀`, `Bk = e^(jnπ/4)`), register shifted **three** positions,
  applied to the 176 reference and data symbols and **never** to the sync.
* **tolerances** ±75 Hz carrier, up to 3.5 Hz/s.
* transcoding, oldest bit first: 1200 — `0→0`, `1→4`; 2400 — `00→0`, `01→2`,
  `10→6`, `11→4`; 3600 — `000→1`, `001→0`, `010→2`, `011→3`, `100→6`, `101→7`,
  `110→5`, `111→4`.

From the two nights of measurement:

* **Detection must be differential.** An 80-symbol coherent integration is 33 ms,
  and 30 Hz — well inside the allowance — turns a whole circle inside it.
  Measured: 0.55–0.61 differential against 0.40 coherent, which is the sequence's
  own bias.
* **The frame ACF of a correct 4285 is about 80/256 = 0.31**, because only the sync
  repeats and the other 176 symbols are random. A capture measuring **0.347** is
  what Edition 1 looks like; one measuring **0.03 while carrying a 47 dB PSK
  carrier** is a different mode. That one number separates the two cheaply.
* **The reference symbols collapse per block, not onto one point.** They are
  sixteen symbols after each of the first three data blocks, so a channel that
  rotates inside 106.67 ms separates the blocks while leaving each coherent. The
  probe's single-point test is why `s4285_lock.py` declines files that do carry a
  frame.
* **Every searched statistic needs a null measured on the same material.** A sync
  correlation is a maximum over every offset in a file: on 60 s of white noise it
  reaches **0.738**. Read as a whole-file answer it produces false positives, and
  did — a 0.900 on an empty channel.
* **The material exists.** `~/s4285-captures/8467.800kHz-20261021-225334Z.wav` —
  36 s, 12.5 kHz, FUV Djibouti, frame ACF 0.347, 8PSK line 47 dB, fading and
  drifting. `4232.000kHz-*.wav` are the negative controls: 47 dB of PSK carrier and
  no frame.

## The shape

```rust
pub struct S4285Receiver { /* … */ }

impl S4285Receiver {
    /// `rate_hz` is the audio tap rate. The carrier is the audio frequency of the
    /// suppressed carrier — 1800 Hz when the dial is on it, but a fixed-frequency
    /// log entry can be a kilohertz out, so it is a parameter (and `search` finds it).
    pub fn new(rate_hz: f64, carrier_hz: f64) -> Self;
    pub fn process(&mut self, audio: &[f32]) -> Vec<S4285Event>;
    pub fn report(&self) -> S4285Report;
    pub fn reset(&mut self);
}
```

`S4285Report` carries: locked or not, sync quality, carrier offset, frame phase,
the three per-block reference concentrations, the rate verdict with its
confidence, and how many frames have been seen. The probe example prints it; the
panel will read it later.

Six steps, in the order they have to happen:

1. **Front end.** Mix the audio down by the carrier, **low-pass while still at the
   input rate**, then resample to 12 kHz (5 samples per symbol). Filtering first
   is what makes linear interpolation acceptable — the probe's resampler trouble
   was interpolating a full-band signal.
2. **Acquisition, differential.** Differentially detect, correlate against the
   differentially-encoded PN31, and accept a frame only when the correlation
   **recurs every 256 symbols**. A single peak means nothing; the recurrence is
   the signal. The offset comes out of the correlation's phase.
3. **Per-frame tracking.** Estimate phase and amplitude from each **16-symbol
   reference block** and interpolate across the 32 data symbols that follow it.
   This is what replaces a long coherent fold: a 36 s fold needs the carrier stable
   to a fraction of a hertz, which a night path never is.
4. **Descramble** the 176 symbols and self-check: the three reference blocks should
   each collapse. Report the three numbers, not their average.
5. **Rate and constellation**, from the **descrambled data symbols** only —
   scrambling makes them uniform over all eight phases. BPSK uses {0, 4}, QPSK
   {0, 2, 4, 6}, 8PSK all eight. **All eight is 3600 uncoded or any coded rate**;
   they are indistinguishable at this layer, so say both.
6. **Output.** The payload is a synchronous bit stream; the character layer is
   ITA2 5N1 and `sdroxide_dsp::BaudotRx` already has the table. Also expose the raw
   bits, because a transmission that is not ITA2 must be visible as bits rather
   than as silence.

## Gates, in the order they are worth passing

1. **Generators pinned against literals** computed independently of the functions:
   the 80 sync symbols and the 176 scrambling symbols.
2. **A round trip in Rust** — build a frame, decode it exactly at 30 / 40 / 60 dB.
   Say in the test's own doc comment that this is plumbing, not an air gate.
3. **The 8467.8 capture**: locks, reports a frame phase, three per-block reference
   concentrations, and a rate verdict. This is the milestone that matters.
4. **The negative controls**: the 4232 captures and a stretch of empty band must
   not lock, judged against a null measured on the same buffer.
5. Every fix gets a test **verified failing on the unfixed code**, and every
   searched statistic reports its null beside its value.

## Traps worth not repeating

* **A box decimation is not a filter.** Its stopband is about −13 dB, so the rest
  of the band folds into the audio; a 3 kHz mode arrived as 6 kHz of junk and cost
  a night's captures. Filter first, then decimate.
* **An RSP1 refuses 48 kHz and silently keeps 2 Msps.** Ask for 62500, and read the
  rate back rather than trusting what you asked for.
* **`gain` is attenuation-ordered on this build.** 35 is louder than 50.
* **An idle station still sends the sync**, so "it locked" means "a 4285 is on this
  frequency", not "there is text in it".
* **A loopback proves the plumbing.** Two halves sharing one convention agree
  beautifully and can both be wrong.
* **Not in scope, and not to be touched: WEFAX, DAB, SSTV and DSC** — field-confirmed
  working; they are not part of the mode work.

## Rules

* **Commit locally. Nothing is pushed.**
* `cargo check --workspace --all-targets` silent on what you touch, `rustfmt` clean,
  the `sdroxide-dsp` tests green.
* Journal in `STANAG4285-HANDOVER.md` at the end (one line per stage with the
  measurement); `MODE-PROGRESS.md` belongs to the other session.
* Measure, don't theorise: print the intermediate stage and compare sequences, not
  decoded text.
* If something needs the operator, say so and carry on.

## Done

**A capture of a real station locks, and the report says what the frame, the three
reference blocks and the constellation are** — with the probe committed so the next
session starts from a number. If text comes out of the 8467.8 capture, better
still, but the lock and the report come first: they are the difference between "a
4285 is there" and a guess.
