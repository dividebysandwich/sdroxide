# Mode phase 3 — the PSK coded families, and what is left behind them

**Paste this whole file into a fresh session, or point the session at it.** It is
the brief for the work after the receive-parity pass recorded in
[`MODE-PROGRESS.md`](MODE-PROGRESS.md). That pass wired four families into the
program and corrected several corpus verdicts; this one starts the largest piece
left — the **23 PSK modes that carry FEC** — and finishes the loose ends behind it.

## Read first, in this order

1. [`AGENTS.md`](AGENTS.md) — the house rules, the bench, the wire and
   `PROTO_VERSION` rules.
2. [`MODE-PROGRESS.md`](MODE-PROGRESS.md) — the journal. **The PSK sections have
   been corrected twice**: what it says about BPSK1000 and about the MT63 narrow
   variants is the *measured* state, not the first pass's verdict.
3. [`MODE-ROADMAP.md`](MODE-ROADMAP.md) — the reference for each mode and the order.
4. `crates/sdroxide-dsp/src/psk.rs` — the receiver as it stands (differential BPSK,
   no FEC, no interleaver, one carrier).

## Get the reference source before anything else

**fldigi 4.2.13's source settles every convention in this phase, and the previous
session only had fragments of it for half the day.** It is 5 MB:

```sh
curl -o /tmp/opencode/fldigi.tgz \
  https://deb.debian.org/debian/pool/main/f/fldigi/fldigi_4.2.13.orig.tar.gz
tar xzf /tmp/opencode/fldigi.tgz -C /tmp/opencode
```

`src/psk/psk.cxx` is the whole phase: the mode table gives each variant's
`symbollen`, `idepth`, `numcarriers`, `_pskr`, `PSKviterbi` and `_puncturing`.
Do not re-derive a parameter from a name.

## Current truth, so nothing is re-derived

**The tree is green and 100 commits ahead of `origin`, none pushed** — the mode
work has deliberately stayed local, and `main` here is the reference copy. The last
fourteen are the phase-2 wiring (MFSK, Contestia, MT63, ARDOP, the PSK speeds, FSQ's
alphabet, THOR/DominoEX's trace, Hell's font and corpus verdict, MT63's sub-bin and
lock); the rest are the first backfill pass. `PROTO_VERSION` is **226**,
`VERSION_BYTE` **0x2E**.

**Wired into the program, receive only** (all in `Mode::is_rx_only`, panels grey the
sending half and the controller refuses with a reason):

| mode | variants | live receiver |
|---|---|---|
| `Mode::Mfsk` | 11 | `mfsk_rx::MfskReceiver` |
| `Mode::Contestia` | tones/bw shared with Olivia | the vendored Jalocha engine |
| `Mode::Mt63` | 2KS, 2KL, 500S | `mt63::Mt63Receiver`, locks by itself |
| `Mode::Ardop` | — | `ardop::ArdopReceiver`, a frame log |

**PSK as it stands:** `PskSpeed` has six speeds (31/63/125/250/500/1000); 125/250/500
read their corpus sample whole, 31/63 read 2 of 3 words, 1000 reads its second half
only. **Every R/C/F variant reads nothing** — that is this phase.

**Corpus state in one line per family** (detail and figures in the journal): MFSK 9
of 11 whole; Olivia 14/18; Contestia 18/19; MT63 3 of 6 through the live receiver;
Hell 2 of 7 clean; THOR/DominoEX 0 of 24 but with a step trace that says how far
off; FSQ's alphabet fixed and its sample suspect; ARDOP reads its three reference
recordings.

## The goal, in this order

**Not in scope, and not to be touched: WEFAX, DAB, SSTV and DSC.** F4NRM and his
radio group report all four working perfectly (2026-10-21). They are shipped
features, they are not in the corpus this phase works from, and a session must not
refactor them or "improve" a decode on the way past. A report against one of them
would be a new report about a specific fault, and it starts there.

### A. BPSK63F — the smallest piece of FEC, and it proves the wiring

`MODE_PSK63F` is **BPSK63 + a rate-1/2 convolutional code + the MFSK varicode**
(the IZ8BLY alphabet already in `mfsk_rx.rs`, not PSK31's). One mode, one gate, and
it exercises the Viterbi without the interleaver or the puncturing.

- The Viterbi is already in the tree twice: `SoftViterbi` (`ifk.rs`) and
  `ViterbiStream` (`fec.rs`). Use one; do not write a third.
- Gate: `samples/fldigi/BPSK63F.wav` reads its known text.

### B. The R family — adds the interleaver and puncturing

`PSK125R`, `PSK250R`, `PSK500R`, `PSK1000R`: `_pskr = true`, `idepth` 40/80/160/160
(the reference calls it a "2x2xN interleaver"), `_puncturing = true` on the wider
ones. Our `Interleaver` (`mfsk_rx.rs`) is the shape; whether it is the same
interleaver is what the gate decides.

- Gate: those four corpus samples read their text.
- The **PSK63F/PSK125R... speed differences are the symbol length only** — the FEC
  is shared, so a working code should light up all four at once. If it lights up
  some and not others, the fault is the interleave depth per mode, not the code.

### C. The C and RC multi-carrier modes — the front end grows

`PSK125C12`, `PSK250C6`, `PSK500C2/4`, `PSK800C2`, `PSK1000C2`, and the
`PSK*RC*` family: `numcarriers` 2, 4, 5, 8. This is a **front end** change, not a
decoder one: N independent differential receivers on a carrier grid, each with its
own FEC, interleaved across carriers.

- Gate: each corpus sample reads its text.
- 17 modes, so **one commit per family**, not one per mode.

### D. Then the loose ends, in the journal's own order

1. **MT63's 1K and 500-long** — the live lock needs a **centre search** that can
   *prefer* the right answer. Measured: searching centre, phase and polarity
   together brought 500S and 1KS back and made 2KL lock 56 Hz away on garbage; a
   "decode a second stretch and require it to read too" validation rejected the
   good ones as well. The missing piece is a per-symbol SNR restricted to symbols
   *after* the interleaver's fill.
2. **THOR/DominoEX** (24 modes) — the instrument exists:
   `the_dominoex_steps_carry_the_known_text` scores the step trace against the
   nibbles the known text implies. It reads **36 % at the best alignment against a
   6 % chance**. Everything else is ruled out by measurement (bank, spacing, step
   rate, tuning). Compare the *per-symbol positions* against the reference's
   `dominoex::decodesymbol`, not the text.
3. **Hell's five weak variants** (HELL80 0.48, SLOWHELL 0.45, FSKH105 0.42,
   FSKH245 0.41, HELLX9 0.33 against a clean copy's 0.8): the ink scale and the
   carrier were both tried and neither moved the score, so it is the dot clock or
   the frame.
4. **FSQ** — needs a real recording; the corpus sample carries no single-tone
   structure at any centre, spacing or speed, and nothing has ever read it back.
5. **ARDOP control-frame names** — `ctrl 0x??` today; `ardopcf`'s `FrameInfo` table
   is the source.
6. **STANAG 4285** (roadmap item 4) — **the state and the build brief are in
   [`STANAG4285-HANDOVER.md`](STANAG4285-HANDOVER.md)**, and the standard itself is
   transcribed in [`STANAG4285-EDITION1.md`](STANAG4285-EDITION1.md): both generators,
   the transcoding tables, the frame and Annex F's acquisition method. Nothing is
   built — the instrument is `tools/s4285-probe/` and the prompt to make the mode is
   in the handover. It still wants a **recording of a station known to be sending
   Edition 1**, made on the bench rather than taken from a proprietary decoder's
   installer.

## Sample policy

- **A sample generated by our own modulator is a plumbing test and never an air
  gate.** fldigi or the reference tool is the transmitter of record.
- **If a sample is suspect, make a new one** — the bench is available (RSP1 + CRT
  SS9900v on VOX) and three recordings are wanted: a known-good FSQ, a BPSK1000
  with a clean start, and anything for THOR.
- The corpus's known text is in `samples/README.md`, and each sample's **own** text
  is in `manifest.jsonl` (`shortened: true` on some). Use the sample's own text.

## Rules

- **Measure, don't theorise.** Print the intermediate stage; compare sequences, not
  decoded text.
- **Never claim a mode receives without its sample decoding, and never claim it
  transmits without the reference decoder reading our signal.**
- **A test that passes on the broken code is not a test.** Verify fail-first where
  a fault is being fixed.
- `cargo check --workspace --all-targets` silent, rustfmt clean on what you touch,
  the touched crates' tests green, and the wasm check when the UI moves.
- **Journal in `MODE-PROGRESS.md`:** one line per mode, state, the measurement.
  Append as you go — that is where the operator looks, not chat.
- **Commit locally. Nothing is pushed.** One complete thing per commit.
- If something needs the operator, put it under **"Needs me"** and carry on.

## Do not re-derive, it cost this phase days

Each of these is a measurement that looked solid and was wrong:

- **The manifest's `centre_hz` is a block estimate, not the signal's centre.**
  MT63-1K/500 were recorded as "one Hadamard row high" for a day because the
  decoder was only ever given that number; swept over the centre, 500S reads its
  whole text. Hell's carriers were 100 Hz out the same way.
- **The manifest's `peak_hz` is a *sideband edge*, not the carrier.** Every PSK
  sample here is a band whose width is the baud rate and whose carrier is its
  **midpoint** (1741 Hz); testing BPSK1000 at 1242 Hz — its lower edge — read
  nothing, and it was briefly withdrawn as broken because of it.
- **A score at a wrong frequency looks like a decode.** A correlation searched over
  thousands of offsets will find 0.4–0.7 by chance on a short pattern. Always
  report the unit (per word, per frame) and the alignment, and prefer a measured
  carrier to a searched one.
- **Two Hell fonts differ in every glyph**, and a correct decode scores 0.1–0.3
  against the wrong one — the same range as noise. Compare against the font that
  transmitted.
- **A mean SNR over a window cannot tell a lock from a coincidence**: a good MT63
  decode averaged 3.2 while a window that decoded nothing averaged 5.4, because the
  window begins with the interleaver's fill. The median of the recent symbols is
  the quantity that separates them.
- **fldigi's tables have entries written as integer char codes** (`{ 39, … }` for
  the apostrophe) as well as character literals; a parser that only takes literals
  silently drops that glyph.
- **A loopback passes on two halves that share the same wrong convention.**
- **The reference's own arrays/source, not a description of them.** The ARDOP
  leader is the *difference* of its two tones; fldigi's `grayencode` is the inverse
  of `graydecode`; the MT63 symbol constants are fixed at every bandwidth.

## Stop when

**Every PSK corpus mode either decodes its own sample or has a written verdict
saying what was measured and why not**, and the journal carries one line per mode.
Then take the loose ends in section D in the order given.
