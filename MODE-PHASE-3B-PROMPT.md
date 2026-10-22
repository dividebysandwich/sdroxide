# Mode phase 3, day 2 — the PSK500R recording, then the rest of the PSK family

**Paste this whole file into a fresh session.** It continues
[`MODE-PHASE-3-PROMPT.md`](MODE-PHASE-3-PROMPT.md), which is still the phase's
brief — read that too, it has the fldigi reference, the sample policy and the
rules. This file is what changed yesterday and what to do first today.

## Read first, in this order

1. [`AGENTS.md`](AGENTS.md) — house rules, the bench, the wire.
2. [`MODE-PHASE-3-PROMPT.md`](MODE-PHASE-3-PROMPT.md) — the phase brief.
3. [`MODE-PROGRESS.md`](MODE-PROGRESS.md) — **"Phase 3 (2026-10-22)"** is
   yesterday's journal: the state, the measurements and what bit the port.
4. `crates/sdroxide-dsp/src/psk.rs` — the three receivers.
5. `tools/psk-ref/` — the independent decoder, and **the instrument for today's
   first job**.

## Current truth, so nothing is re-derived

`main` carries the PSK coded families, **receive-only and not wired into the
program** — `PROTO_VERSION` is still **226**, `VERSION_BYTE` **0x2E**,
**116 commits ahead of `origin`, none pushed**. The phase-3 commits so far:
`5f1498b1` (the receivers), `9fbe3d68` (the multi-carrier passband), `1da2ce8c`
and `cbfd1a3d` (journal), `4686683e` (`tools/psk-ref`), `13f2c105` (this brief).
Check the count rather than trusting it — another session has been committing in
this same checkout.

In `psk.rs`, and exported from `sdroxide-dsp`:

| type | what it is |
|---|---|
| `PskrRx` | BPSK63F and the single-carrier `R` set: soft differential symbols, two Viterbi decoders one symbol apart voting on their metrics, the `2x2xN` interleaver, MFSK varicode |
| `PskCarrierBank` | `numcarriers` receivers `1.4 × baud` apart, one symbol from each per symbol time |
| `PskMultiRx` | the `C` family — multi-carrier, **no FEC**, PSK31's varicode |

**What decodes today** (`cargo test -p sdroxide-dsp --release --lib the_corpus_psk
-- --ignored --nocapture`):

- BPSK63F, PSK125R, PSK250R, PSK1000R — whole text; 125R and 1000R bit-for-bit
  (394 of 394 against the known text).
- 18 of the 26 `C`/`RC` modes — whole text.
- **PSK500R reads 27 characters and stops**, and that is the recording: an
  independent decoder (`tools/psk-ref`) reads the same 27 at every centre from
  1700 to 1800 Hz.
- Still short: `PSK125C12` 1/3, `PSK63RC32` 2/3, `PSK250RC2` 2/3, `PSK250RC5`
  1/3, `PSK500RC3` 1/3; `PSK500C4`, `PSK500RC4`, `PSK1000C2` 0/3.

## The order for today

### 1. The PSK500R recording — **the RSP1 was busy yesterday, so this is first**

It is a bench job and it needs hands. It answers one question: does a *fresh*
PSK500R transmission carry the whole text? If it does, yesterday's verdict about
the corpus sample stands as a sample fault and this becomes the R family's last
air gate; if it does not, our receiver is at fault after 27 characters and there
is a real bug to chase.

The harness already exists and records one mode on its own:

```sh
cd ~/sdroxide/tools/bench
python3 sweep.py --only '^PSK500R$' \
    --out /tmp/psk500r --manifest /tmp/psk500r/manifest.jsonl
```

- **`--out` is redirected on purpose**: the default overwrites
  `samples/fldigi/PSK500R.wav`, and the old recording is the evidence for the
  verdict — compare it, do not replace it until the new one is read.
- `--only` is a regex, so `^PSK500R$` and not `PSK500R` — that matches
  `PSK500RC2/3/4` as well.
- Prerequisites, from `tools/bench/README.md`: fldigi running with XML-RPC
  (`CHKUSEXMLRPCIS` in `fldigi_def.xml`), the rig on **VOX** with its sound
  card's `PCM` level not at zero (`amixer -c 1 sget PCM` — this has cost an
  evening by itself), **sdroxide closed** because the RSP1 must be free, and
  `tools/rsp1-capture/target/release/rspcap` built.
- The harness does not tune the radio. The dial is by hand at 27.415 MHz; the
  rig's own mode is USB and CW keying is Sound card (MCW) — see "The bench" in
  `AGENTS.md`.

Read the new file with **both** decoders before believing anything:

```sh
python3 tools/psk-ref/psk_ref.py /tmp/psk500r/PSK500R.wav 500 160 1754
cargo test -p sdroxide-dsp --release --lib the_corpus_pskr -- --ignored --nocapture
```

Then: if the new recording reads whole, copy it into `samples/fldigi/` as
`PSK500R.wav`, move the old one to `samples/bench/` as
`psk500r_truncated_2026-10-21.wav` with a line in `samples/README.md`, and change
`the_corpus_pskr_samples_are_measured` to assert 3/3 for it. If it does not read
whole, the fault is ours and the next step is to feed the *new* recording to
`tools/psk-ref` — if that one reads whole and ours does not, the difference
between the two front ends is the answer, and it is a short list: our timing
loop (Gardner) versus a fixed phase, and our `softamp` weighting versus none.

**If the sweep's own window truncated it** — the harness has cut samples before
(`MAX_WINDOW_S`, and the `burst_seconds` versus `expected_seconds` check in the
manifest is what catches it) — then this is a harness fault, and the manifest
line will say so: a burst much shorter than the text needs. Check that field
before blaming the decoder.

### 2. ✅ **DONE 2026-10-22** — the three that read nothing: `PSK500C4`, `PSK500RC4`, `PSK1000C2`

The band edges were measured first and **every carrier is inside the file** (the
widest is 744–3741 Hz of an 8 kHz recording), so the brief's own hypothesis is
out. The independent `C` decoder in `tools/psk-ref` reads nothing from them
either, and the new `tools/psk-ref/psk_text_match.py` scores what each recording
actually carries: **0.841 and 0.848** of the known text's bits against 0.993 for
a sample that decodes, and 1.000 for a clean synthetic bank. A `C` mode has no
FEC, so ~15 % of bits wrong is unreadable. **These are recordings, not the
decoder** — the verdict per mode is in `MODE-PROGRESS.md`.

All three are wide banks, and two of them put their outer carriers close to the
4 kHz edge of an 8 kHz recording — which is why the band edges were the first
thing measured. The `RC` member, `PSK500RC4`, has no bit score because its wire
carries encoded bits; it is the same sweep and the same width as `PSK500C4`,
which measures 0.841.

### 3. ✅ **DONE 2026-10-22** — the five partials

Read out in full text with `psk_coded_probe` at the centre the gate reports:
**two lock late** (`PSK250RC2` starts at `EAGLE`, `PSK63RC32` at `LANDED`),
**two lose the stream halfway** (`PSK250RC5` and `PSK500RC3` after
`THE EAGLE H`), and `PSK125C12` barely locks at all (its recording scores 0.630
of the text). **None loses its tail**, so the interleaver's drain is not the
fault. One line per mode is in `MODE-PROGRESS.md`.

### 4. Then wire it — one mode with chips, as MFSK has

Nothing above is reachable by an operator yet. The shape to copy is
`Mode::Mfsk`: one `Mode` with a chip per variant, because the family is one modem
at different rates and the parameters are not independent. What has to be decided
before writing it:

- **the config fields.** `DigiConfig` has `psk_speed` and `psk_modulation`
  today. The robust set needs the interleave depth per speed and a carrier count;
  a flat `PskMode` list of the 31 fldigi modes is the honest model, and it rides
  the wire whole, so it is a `PROTO_VERSION` bump plus a `VERSION_BYTE` bump and
  a register line.
- **receive only.** The family has no modulator here, so it goes in
  `Mode::is_rx_only`-style handling, the panel greys the sending half and the
  controller refuses with a reason — the same shape as MFSK and MT63.
- **the band width.** A 32-carrier bank is 2.8 kHz wide; the panadapter's markers
  should draw the bank the way MFSK's tones are drawn, from one table that the
  decoder also reads.

### 5. Then the loose ends in `MODE-PHASE-3-PROMPT.md` section D

MT63's 1K/500-long first (the journal says what is missing: a per-symbol SNR
restricted to symbols after the interleaver's fill, and a lock window that scales
with the symbol time rather than a fixed 6/12 s), then THOR/DominoEX's
per-symbol positions, Hell's five weak variants, FSQ, and ARDOP's control-frame
names.

## Do not re-derive — these cost yesterday days

- **The pairing is not the bit pairing.** A differential receiver does not know
  which symbol starts a coded pair, so the reference runs two decoders one symbol
  apart and votes on the Viterbi metric. Taking pairs `(s0,s1),(s1,s2)…` instead
  of alternating symbols through a shared history produced 354 bits from 195.
- **The order inside the pair.** fldigi interleaves the pair as `[o2, o1]` and
  then transmits the array *backwards*, so the wire order is `o1, o2` while the
  interleaver's positions are the reverse. A loopback passes either way round if
  the transmitter makes the same mistake.
- **`PSKR_POLY1/POLY2` = `0x6d`/`0x4f`** is a different code from
  `ConvCode::THOR`'s `0o155`/`0o117`. Do not substitute.
- **The interleaver's latency is `(size − 1) × depth` calls**, not
  `size × depth`; `the_interleaver_inverts_itself` pins it now.
- **The soft value has to be weighted by amplitude** against the running average,
  as fldigi's `softamp` is; a plain `re/|d|` gives a noise symbol the confidence
  of a signal one.
- **A multi-carrier passband is `1.0 × baud`**, the reference's matched sinc.
  At `0.75 × baud` PSK500RC2 reads nothing and at `1.0` it reads whole. A lone
  carrier keeps `2.5 × baud`.
- **`C` and `RC` are not the same family.** `C` is multi-carrier with no FEC and
  PSK31's varicode; `RC` adds the code and the interleaver and uses MFSK's.
- **The manifest's `centre_hz` is a block estimate and `peak_hz` is a sideband
  edge.** The decoders search; the carriers measured from the audio are 1738 for
  BPSK63F and 1754–1760 for the `R` set.
- **A bank is read with ONE symbol clock.** fldigi's `psk::rx_process` keeps a
  single `bitclk` for the whole bank and estimates its error from the **mean
  magnitude of every carrier**, then reads them all at that instant;
  `PskCarrierBank` runs a Gardner loop per carrier instead. Not what makes the
  three bad samples fail (the independent decoder has the reference's clock and
  reads them no better), but the first thing to change if a *clean* wide bank
  ever fails.
- **A sampling phase chosen per carrier permutes the interleave** — one carrier
  ends up a symbol behind its neighbours — and it reads exactly like a bad
  sample. The phase is chosen once, for the whole bank. It cost the measuring
  instrument an afternoon.
- **When a sample will not read, prove which side is wrong.**
  `tools/psk-ref` exists for exactly that and it is what settled PSK500R.
  `psk_text_match.py` goes further: it says **how much** of the text the
  recording carries (1.000 clean, 0.5 noise) and calibrates itself against a
  synthetic bank first.
- **A loopback passes on two halves that share a wrong convention.** Yesterday's
  fixture fails at 250 baud where the corpus reads 394/394 — that is recorded in
  the test rather than hidden, and it means the *fixture* is the suspect there,
  not the port.

## Rules

- **Measure, don't theorise.** Print the intermediate stage; compare sequences,
  not decoded text.
- **Never claim a mode receives without its sample decoding**, and never claim it
  transmits without the reference decoder reading our signal.
- **A test that passes on the broken code is not a test.**
- `cargo check --workspace --all-targets` silent, `rustfmt` clean on what you
  touch, the touched crates' tests green, and the wasm check when the UI moves.
- **Journal in `MODE-PROGRESS.md`**: one line per mode, state, the measurement.
- **Commit locally. Nothing is pushed.** One complete thing per commit.
- **Another session has been writing in this same checkout** — `5ccfa23f` swept
  up half-finished work of ours. Stage files by name; do not `git add -A`.
- If something needs the operator, put it under **"Needs me"** and carry on.

## Needs me

- **A fresh PSK500R recording** — today's first job, and the RSP1 is free again.
- **A known-good FSQ recording**, still outstanding from yesterday's list.
- **A second MFSK8 recording**, and the THOR/DominoEX comparison recording.

## Stop when

Every PSK corpus mode either reads its whole sample or has a written verdict
saying what was measured and why not; then the wiring, then the loose ends.
