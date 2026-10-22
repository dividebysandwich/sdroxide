# STANAG 4285 — day 2. Read a real station, then wire it.

**Paste this whole file into a fresh session.** It continues
[`STANAG4285-PROMPT.md`](STANAG4285-PROMPT.md), which is still the reference for
the standard's own detail and for the six steps — but its "what is already known"
is now history, and the receiver it asked for exists. Read this file first, then
that one for the conventions, then the three below it.

## The goal, in the operator's words

**Those stations are broadcasting to nobody in particular and I want to know what
they say.** STANAG 4285 carries the French Navy's RYRY test tapes and their
identification traffic, and no program on this bench can read one. When this is
done I can tune a station, see that it is a 4285 rather than a guess, read its
rate off the panel, and watch its text arrive — and a listener who finds one on a
frequency they cannot identify gets the same.

Two things follow, and they are the criteria rather than the sentiment:

1. **A real capture of a real station reads.** Not a loopback, not a synthetic
   frame: the sound off the air, through the receiver, out as characters.
2. **The program says what it has.** A lock that is a measurement — the frame
   ACF, the three reference blocks, the rate — and not a lamp that lights on a
   strong carrier. A mode whose only evidence is a loopback is forbidden here.

Coded rates (Annex E) are **out of scope** unless the annex is transcribed first.
The panel must then say "3600 8PSK uncoded, or a coded rate" rather than pretend,
which is the honest ceiling of a constellation test.

## Read first, in this order

1. [`STANAG4285-HANDOVER.md`](STANAG4285-HANDOVER.md) — **the state**, and the two
   entries from 2026-10-22 are the ones that matter: they carry every number
   below and the two mistakes that cost the day.
2. [`STANAG4285-EDITION1.md`](STANAG4285-EDITION1.md) — the standard, transcribed.
   The transcribing tables, the two generators and Annex F.
3. [`STANAG4285-PROMPT.md`](STANAG4285-PROMPT.md) — the six steps and the shape of
   the API, still right.
4. `tools/s4285-probe/README.md` — the long measurement log and the two nulls that
   had to be measured before any statistic could be believed.
5. [`AGENTS.md`](AGENTS.md) — house rules, the bench, the wire.

## Current truth, so nothing is re-derived

`crates/sdroxide-dsp/src/stanag4285.rs` exists: receive-only, uncoded rates,
`S4285Receiver::{new, process, report, bits, set_carrier, reset}`,
`S4285Event::{Frame, Text}`, and a `S4285Report` carrying `locked`,
`sync_quality`, `correlation_floor`, `frame_acf`, `carrier_offset_hz`, `frames`,
`rate` and the three `reference` concentrations. `examples/s4285_probe.rs` runs it
on a WAV and prints all of it. One line in `sdroxide-dsp/src/lib.rs`. **Nothing is
wired** — no `Mode`, no panel, no wire change.

Measured on 2026-10-22, after the generator fix:

| file | frame ACF | lock | sync | reference blocks | offset |
|---|---|---|---|---|---|
| `~/s4285-captures/8467.800kHz-20261021-225334Z.wav` | **0.400** | yes, 163 frames | 0.547 | 0.485 / 0.281 / 0.299 | −106 Hz |
| `~/s4285-captures/4232.000kHz-*.wav` (controls) | **0.042** | **no** | — | — | — |

The capture is 36 s at 12.5 kHz, FUV Djibouti, with the fading and drift a night
path has. The controls are a strong PSK carrier of a *different* mode and no frame
at all — which is why they are the control and not a second signal.

**The one number to move next: the reference concentrations.** They are
0.28–0.49 where a clean lock gives 0.7–0.9. That is frame phase and block
alignment, not the generators (those were fixed and are now pinned by a test).
The offset estimate sitting at −106 Hz, near the edge of the ±150 Hz search, is
the same symptom seen from the other side: the ramp fit is being pulled.

## The order for today

### 1. Make the reference blocks collapse

This is the whole of today's first job and it needs no operator. The references
are **all symbol 0 before scrambling**, so after descrambling a correct block
collapses onto one point; 16 symbols of noise give 0.25 and the Djibouti capture
gives 0.48/0.28/0.30. Work through, in this order:

- **the frame phase.** One symbol out and every block is half-shifted. The probe's
  lock comes from the correlation's position, which is a symbol or two coarse;
  refine it by maximising the *mean of the three blocks* over a ±3-symbol shift
  before anything else is blamed.
- **the block boundaries** against the standard's own layout: `32 data + 16
  reference` three times, then 32 data. `REF_BLOCKS` and `DATA_BLOCKS` in the
  module are the tables; a reader that trusts them only because they are typed is
  how a whole day goes.
- **the ramp** inside a block, which is already implemented as a search over
  ±150 Hz — check it is not riding the edge, and report where it lands.
- **the sync being 80 and the frame 256**: a lone 0.547 sync on a faded capture is
  plausible, but the references do not lie.

When the three numbers are 0.7 up, the text is the next thing to look at — and
until they are, **text is not worth reading**. Printing plausible letters off a
frame whose references do not collapse is how a wrong demodulation looks right.

### 2. Read a station — the material is already on the disk

**Thirty-five captures from the night of 2026-10-22 are in `~/s4285-captures/`,
36 s each at 12.5 kHz, and the receiver locks 27 of them.** Nothing has to be
recorded to make progress. Measured with `s4285_probe` at 1800 Hz, best first:

| capture | ACF | sync | frames | reference blocks |
|---|---|---|---|---|
| `6348.000kHz-20261022-045545Z.wav` | **0.588** | 0.862 | 180 | 0.240 / 0.626 / 0.219 |
| `6348.000kHz-20261022-054415Z.wav` | 0.512 | 0.608 | 194 | 0.362 / 0.342 / 0.413 |
| `6348.000kHz-20261022-051325Z.wav` | 0.476 | 0.727 | 187 | 0.494 / 0.372 / 0.473 |
| `6348.000kHz-20261022-051457Z.wav` | 0.430 | **0.911** | 165 | 0.411 / 0.499 / 0.509 |
| `6348.000kHz-20261022-051154Z.wav` | 0.422 | 0.532 | 172 | **0.715** / 0.226 / 0.481 |
| `8467.800kHz-20261021-225334Z.wav` | 0.400 | 0.547 | 163 | 0.485 / 0.281 / 0.299 |
| `8478.500kHz-20261022-063227Z.wav` | 0.395 | 0.663 | 150 | 0.210 / 0.481 / 0.401 |
| `5215.600kHz-20261022-024501Z.wav` | 0.392 | 0.610 | 180 | 0.401 / 0.179 / 0.580 |

**Eight refuse**, which is the other half of the measurement and the reason the
detector can be trusted: the five `4232.000kHz` files (ACF 0.012–0.072),
`6348.000kHz` at 0336Z, 0317Z and 0342Z (0.047, 0.020, 0.007), and — the one to
**look at first** — `6348.000kHz-20261022-032809Z.wav` at **0.140**, just inside
the 0.12 gate. A capture that sits on the threshold is the best test the gate has:
either it is a fading 4285 that should lock and the gate is too high, or it is the
other mode and the gate is too low, and one of those two is a measurement away.

`6348 kHz` carries the strongest captures of the night, so **the handover's note
that its captures are "a flat mess" is history** — that was before the WAV writer
was filtered. Start there.

An idle station still sends the sync (`an idle station still sends the sync`), so
a lock is "a 4285 is on this frequency" and not yet "there is text in it" — but
with 27 lockers and references already reaching 0.715, the material to read text
from is in hand. The station frequencies for a future capture are in the handover
(**4232.0 / 5215.6 / 8467.8 / 8478.5 / 12857.0 / 13031.2 / 13411.9 / 6348.0 kHz**),
and a capture is only wanted if the collapse fix needs a *cleaner* file rather
than a better receiver — which is a measurement to make first, not an assumption.

### 3. Then, and only then, wire it

The shape to copy is `Mode::Mfsk` / `Mode::Mt63`: one `Mode`, a chip per rate,
receive-only, the panel greys the sending half and the controller refuses with a
reason. What has to be decided, and decided here rather than asked:

- **the mode variant** — appended to `Mode`, so `check` the register in
  `crates/sdroxide-proto/src/lib.rs` and take the **next** `PROTO_VERSION`, with
  `VERSION_BYTE` and a register line naming both. Do not trust a number written
  in this file; it was 226 and 0x2e on 2026-10-22.
- **the panel** must show what the receiver actually measures: locked, the frame
  ACF, the three reference blocks, the rate, and the offset. A 4285 panel with a
  green lamp would be a lie in the shape of a feature.
- **receive only, and the reason on screen**: there is no modulator, so the mode
  says so rather than failing silently when somebody presses send.
- **the panadapter** already has a convention for a fixed-carrier mode (RTTY,
  NAVTEX); a 4285 sits at 1800 Hz and the width is 3000 Hz, both from the standard.

### 4. The negatives, which are part of the feature

An empty stretch of band and the 4232 captures must **not** lock, judged against a
null measured on the same buffer. A test that pins the lock without pinning the
refusal is not a test of a detector — it is a test of a lamp.

## Do not re-derive — both of these cost a day

- **A polynomial's degree is its stage count.** `x⁵ + x² + 1` is **five** stages
  with taps {0, 2}; `x⁹ + x⁴ + 1` is **nine** with taps {0, 4}. Building it as
  "highest tap + 1" empties the register and it stays at zero: the scrambler
  becomes a constant and the sync has period 16. **Nothing fails loudly** — the
  degenerate sync correlated *better* (0.814) than the true one (0.547), and the
  references simply never collapsed. `the_generators_are_maximal_and_match_the_standard`
  pins all of it now.
- **The frame ACF is the detector; the sync correlation is not.** 80 of a frame's
  256 symbols repeat, so a correct 4285 measures ~0.31 and another mode ~0.03 —
  0.400 against 0.042 on these two files. A correlation alone is a maximum over
  every offset and reaches **0.738 on 60 s of white noise**; gating a lock on it
  is how an empty channel produced a 0.900.

And from the previous session, still true: detection must be differential (±75 Hz
turns a coherent 33 ms window through a circle); filter **before** resampling; a
box decimation is not a filter; an RSP1 refuses 48 kHz and silently keeps 2 Msps,
so ask for 62500 and read the rate back; `gain` is attenuation-ordered on this
build; a loopback proves the plumbing and two halves can share one wrong
convention.

## The bench, if you need to make a capture

The rig is a **CRT SS9900v on 11 m** and is not a tool for this — these stations
are HF. What matters is the receiver: `tools/rsp1-capture/target/release/rspcap
<hz> <rate> <seconds> <out.raw> [gain]`. Known from the last session:

- **fldigi's output goes through the AB13X card** and its PortAudio index moves
  when the card is reconnected — `--` check it with a five-line `pa_devs` rather
  than trusting the name in the config, which once pointed at an HDMI device.
- **every PSK mode is stored at −30 dB TX level** in `~/.fldigi/mode_state.prefs`
  (`rg '^PSK.*tx_'`), which is enough to key nothing.
- **2 W is cleaner than 4 W** for a capture five metres from the rig: the RSP1 is
  desensed by our own transmitter, and one mode's score moved 0.767 → 0.841 when
  the power was halved.
- `tools/bench/sweep.py` no longer chases a receiver spur as "the signal", and its
  burst is measured on the extracted audio.

## Rules

- **Commit locally. Nothing is pushed. No release is tagged — ever, by an agent.**
- `cargo check --workspace --all-targets` silent, `rustfmt` clean on what you
  touch, `sdroxide-dsp`'s tests green, and the wasm check once the UI moves.
- **Every fix gets a test verified failing on the unfixed code**, and every
  searched statistic reports its null beside its value.
- **Journal in `STANAG4285-HANDOVER.md`** — one entry per stage with the
  measurement. `MODE-PROGRESS.md` belongs to the PSK session, which is finished
  with it for now; leave it alone unless the change is about a PSK mode.
- Measure, don't theorise: print the intermediate stage and compare sequences, not
  decoded text.
- **Not in scope, and not to be touched: WEFAX, DAB, SSTV and DSC** — field
  confirmed working by a whole radio group.
- If something needs the operator, add it under **"Needs me"** below and carry on.

## Needs me

Nothing, to start. The captures are on the disk, the receiver locks them, and the
reference collapse is a code fault with a number attached.

- **A capture is only worth asking for once it is shown to be the limit** — e.g. if
  the collapse fix gives 0.9 on the cleanest file and the faded ones still read
  nothing, then a stronger recording of a station sending a **test tape** (RYRY,
  an ID, the French pangram) would be the next thing. The frequencies are in the
  handover. Ask then, with that sentence as the reason.
- **The coded rates**: Annex E is not transcribed. If they are ever wanted, that is
  a transcription job first, and the panel keeps saying "3600 8PSK uncoded, or a
  coded rate" until the bits are proven.

## Stop when

**A real capture reads text** — through the receiver, with the frame ACF, the three
reference blocks and the rate all reported — **the negative controls refuse**, and
the mode is wired with a panel that shows those numbers. Then write the release
note and leave the tag to the operator.
