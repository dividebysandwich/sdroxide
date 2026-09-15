# C-QUAM (AM stereo) decoding — design note

A proposal for decoding **Motorola C-QUAM** AM stereo on the medium-wave
broadcast band, as a receive-only SWL feature of this fork. Written before any
code; the aim is to fix the signal model, the integration points and the test
plan, and to name the risks while they are still cheap to change.

> **Status:** implemented. `Mode::Cquam` (receive-only) exists, with the
> decoder in `crates/sdroxide-dsp/src/demod.rs` and synthetic round-trip,
> pilot and carrier-inversion tests that pass. It is **unverified against a
> real signal** until the 918 kHz capture below is decoded, so the blend
> thresholds and the sign conventions are still first-cut.

## Scope

- **Receive only**, first pass. C-QUAM *transmit* (an encoder, a 25 Hz pilot,
  phase modulation) is a later question and not needed for listening.
- **Broadcast use**: the medium-wave band (`MW`), where C-QUAM is actually on
  the air (US / Canada / Brazil, historically Japan). The mode stays selectable
  anywhere AM is, like every other mode.
- **Fork feature**, not an amateur-band one. Upstream may not want it in the AM
  receiver itself, so it should be its **own mode**, leaving `AM` and `SAM`
  untouched.

## The signal, briefly

C-QUAM keeps an ordinary AM envelope (so a diode detector hears mono) and hides
the stereo difference in the carrier *phase*:

```
s(t) = [1 + (L + R)] · cos(ω_c t + φ(t))
φ(t) ∝ (L − R) plus a 25 Hz pilot term
```

- An **envelope detector** — AM as we already have it — yields `L + R`. That is
  the "compatible" mono, and it is why the standard is called compatible.
- The **difference** `L − R` appears in **quadrature** after synchronous
  detection, together with a low-level **25 Hz pilot**.
- The pilot exists for one reason: without a phase reference the decoder cannot
  tell L from R — a phase-inverted carrier would swap the channels. Locking the
  25 Hz pilot fixes the sense, and its quality is the natural mono/stereo
  switch.

So a C-QUAM receiver is: carrier recovery, in-phase = `L + R`, quadrature =
`L − R` + pilot, a 25 Hz pilot PLL for the sense, and a matrix.

## Why this fits the codebase

Two of the three hard parts already exist, which is what makes this a small
feature rather than a project:

- **Carrier recovery**: [`SamDemod`](../crates/sdroxide-dsp/src/demod.rs)
  already carries a second-order PLL and coherent detection; its in-phase arm is
  exactly the `L + R` we need, and its quadrature arm is available.
- **The stereo plumbing is already generic**: the `Demodulator` trait has
  `stereo_diff` (the `(L − R)/2` channel), `stereo_locked`, `stereo_blend` and
  `set_stereo_enabled`, and `RxChain` already carries the side channel to the
  audio device (it is how WFM stereo and RDS work today).
- **The pilot-locked stereo decoder has a working template**:
  [`WfmDemod`'s `PilotPll`](../crates/sdroxide-dsp/src/demod.rs) locks a 19 kHz
  pilot, regenerates the suppressed subcarrier, recovers the side channel,
  blends to mono on poor lock, and drives the UI lamp. C-QUAM is the same shape
  at 25 Hz.

## Integration points

- **`sdroxide-types`** — a new `Mode::Cquam`:
  - **Appended at the end of the `Mode` enum**, not beside `Am`/`Sam`: `Command`
    is carried as postcard on the wire and variants are numbered by position, so
    inserting one would renumber every mode a remote client sends.
  - `Mode::ALL` (currently `[Mode; 39]`), `label()` (`"C-QUAM"`),
    `is_rx_only()` (true), the filter default (like `Am`/`Sam`, ±5 kHz), and
    whatever channel/kind classification `Am`/`Sam` use.
  - Menu order is a UI concern (below), independent of the enum position.
- **`sdroxide-dsp`** — `CquamDemod` in `demod.rs`, dispatched from `make_demod`
  alongside `Mode::Am`/`Mode::Sam`.
- **Engine** — nothing beyond the existing mode routing; the RX chain already
  handles a stereo demod.
- **UI** — the mode appears in the AM group of the mode list; a **STEREO**
  indicator (the WFM one, driven by `stereo_locked()`) and, if wanted, the blend
  as a diagnostic. `top_bar.rs` already special-cases `Mode::Wfm` for its
  stereo/RDS chips.
- **Docs** — the user manual's mode list and the README comparison table.

## Decoder design

Blocks, in order:

1. **Complex bandpass**, as `SamDemod` does (`PASSBAND_TAPS`), ±~5 kHz about the
   carrier — narrow enough to reject the adjacent channel without clipping the
   `L − R` sidebands.
2. **Carrier PLL** — the existing `SamDemod` loop. Rotate by `e^{−jθ}`; the real
   part `I` is `L + R`, the imaginary `Q` is the difference plus pilot.
3. **DC block on `I`** — the carrier term, as `SamDemod` already does.
4. **25 Hz pilot recovery from `Q`** — a narrow band-pass plus a PLL, mirroring
   `PilotPll`: track frequency and phase, report `locked()` and a `blend()`
   figure from lock quality (the way WFM derives its blend from pilot SNR).
5. **`L − R`** — multiply `Q` by the recovered pilot reference (giving the
   correct sense) and low-pass to the audio band. A degree of phase alignment
   matters: the pilot reference, not `Q` alone, is what guarantees L is L.
6. **Matrix and blend** — `L = M + S`, `R = M − S`, with `M = (L + R)/2`
   (post-DC) and `S = (L − R)/2`. Blend `S` toward zero as pilot lock falls, so
   a weak or fading signal falls back to a clean mono rather than a wrong stereo.
7. **Stereo output** — via the trait's `stereo_diff`; `stereo_locked` from the
   pilot PLL, exactly as WFM does.

Parameters to pick and document: audio low-pass corner (~5 kHz), pilot loop
natural frequency (a few Hz) and damping, lock/blend thresholds and time
constants, and the `L − R` low-pass.

## Testing

The synthesised-signal route covers nearly everything without a recording:

- **Round trip**: a small C-QUAM encoder in the test module (the pattern
  `modulator.rs` already uses for AM/SSB) encodes a known `L`/`R` tone pair; the
  decoder must recover both, with the right **channel sense** and separation.
- **Pilot loss / inversion**: with the pilot removed the decoder must fall back
  to mono and *not* emit a swapped or half-level stereo; with an inverted pilot
  the sense must stay correct. This is the case a naive quadrature decoder gets
  wrong, so it gets a test of its own.
- **Tuning and fading**: an off-tune carrier, and a two-tone selective-fading
  simulation (a moving notch), to exercise the PLL and the blend.
- **Golden WAV** in `crates/sdroxide-dsp/tests` if a stable recording is worth
  keeping.
- **Real validation**: a capture of an actual C-QUAM station. See below.

### Real-signal validation

A real signal is available: **918 kHz**, receivable between **10:00 and 20:00
local**. That is daytime, so it is a groundwave path and should be stable —
the easier case, not the night-time fading one — which makes it a good
fixture for tuning the decoder and the blend.

Capture it as **raw IQ**, not demodulated audio: the stereo lives in the
carrier's *phase*, and an AM or WFM audio recording throws exactly that away.
sdroxide already records the format it can replay (`--record-iq` writes raw
interleaved CF32; **REC** on the top strip does the same):

```
# Once the mode exists. Tune to 918 kHz, record 30-60 s, and replay it.
sdroxide --record-iq 918k.cf32            # or press REC
sdroxide --file 918k.cf32 --rate <captured> --freq 918000 --mode Cquam
```

Record at the front end's own rate — a couple of hundred kHz about 918 kHz is
plenty — and long enough to cover a stereo passage and a mono one, so the
mono/stereo switch can be heard as well as tested. The file stays a **local
fixture**: a broadcast recording is the station's copyright, so it is not
committed. The repository keeps the synthetic tests; the capture is what turns
"unverified against real signals" into "verified" on the developer's machine.

## Open questions

- **Off-air verification**: available — 918 kHz, 10:00–20:00 local, daytime and
  therefore groundwave (see above). Capture it once the decoder exists; until
  then the synthetic tests carry the weight.
- **Sense policy**: how aggressively to refuse stereo when the pilot is marginal.
  A clean mono beats a wrong or warbling stereo; the blend curve decides this.
- **Where the STEREO control lives**: a lamp only (like WFM), or a lamp plus a
  manual mono/stereo override and blend readout.
- **Transmit**: out of scope now; note that `modulator.rs` makes it a bounded
  follow-up if a transmit-capable path is ever wanted.

## Effort

A focused feature: a new demodulator (~200–300 lines) plus a pilot PLL
(~100), a small amount of mode plumbing, a UI lamp, and the test module and its
synthetic encoder (~150). One short branch. The main cost is the real-signal
validation, not the code.
