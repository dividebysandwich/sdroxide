# Mode phase 4 — finish the modes, then give them a voice

**Paste this whole file into a fresh session.** It follows the PSK phase, which is done; what that phase left behind is in
[`MODE-PROGRESS.md`](MODE-PROGRESS.md).

## The goal, in the operator's words

**Finish the missing modes, and get TX working for every mode that fldigi can
transmit.**

Three things follow, and they are the operating rules rather than the sentiment:

1. **If a mode will not work after the port and the bench have both been
   exhausted, vendor it.** A vendored mode that works is better than a mode that
   does not ship. This *changes* the standing rule of 2026-10-21 — see "The rule
   that changed" below — and the change is the operator's.
2. **Work autonomously.** Decide, measure, record. Come back with "Needs me" only
   for something no session can do itself: a bench recording, a licence question,
   a demodulator that has been shown to need hardware.
3. **Keep the progress in [`MODE-PROGRESS.md`](MODE-PROGRESS.md)**, one entry per
   stage, each carrying its measurement. That file is the deliverable as much as
   the code is.

And when the work is done, **the table** — every mode we ship, five columns:
**name, rx, tx, vendored, tested**. Its shape is at the end of this file. It is
the thing the operator asked for, so it is not an afterthought.

## Read first, in this order

1. [`MODE-PROGRESS.md`](MODE-PROGRESS.md) — the journal. One line per mode, plus
   the day-by-day sections at the end. **Everything measured is there**; this file
   summarises it and will go stale.
2. [`MODE-ROADMAP.md`](MODE-ROADMAP.md) — the queue, the reasoning, and the
   per-mode reference notes. Note that its "port it, do not vendor it" rule is
   superseded here.
3. [`AGENTS.md`](AGENTS.md) — house rules, the bench, the wire, the release rules.
4. The per-mode references: [`STANAG4285-HANDOVER.md`](STANAG4285-HANDOVER.md),
   [`OLIVIA-HANDOVER.md`](OLIVIA-HANDOVER.md), [`CW-HANDOVER.md`](CW-HANDOVER.md).

## Current truth (measured 2026-10-22 — re-measure, do not trust this list)

`main` is **149 commits ahead of `origin`**, nothing pushed, no tag.
`PROTO_VERSION` **227**. `cargo check --workspace --all-targets` is silent.

### What decodes its own recording

| family | state |
|---|---|
| PSK plain | BPSK/QPSK 125, 250, 500 whole; 31 and 63 lose a word; BPSK1000 reads its second half (lock time) |
| PSK coded | **BPSK63F and the whole single-carrier `R` family (125/250/500/1000)**: 394/394 bits |
| PSK multi-carrier | **18 of 26** `C`/`RC` read whole; 4 partial (lock time or lose lock); 4 read nothing, and three of those are proved to be the recordings by `tools/psk-ref/psk_text_match.py` |
| MFSK | 10 of 11 whole; MFSK8 loses one span *of the recording*, MFSK11 its leading group |
| Olivia | **14 of 18**; 4/250, 4/500, 8/125, 64/500 read nothing while the engine reads the other fourteen |
| Contestia | **18 of 19**; only 64/500 never reads |
| MT63 | **2KS whole; 2KL partial** (`AAa SDRBROWn tESt` — a few characters 32 codes high); 500S whole in the live receiver; 1KS/1KL/500L not offered |
| Hell | **HELLX5 0.87 and FELDHELL 0.83** clean; HELL80, SLOWHELL, FSKH105/245 read weak (0.40–0.48); HELLX9 0.33 |
| DominoEX | **DOMX11 4/4** (`AAA TEST SqST THE EAGLE HAS LANDED 12345 SDRBROWN ZZZ`); DOMX22 and DOMX88 3/4; DOMEX5/DOMX16/DOMX44 2/4; DOMEX8 1/4; DOMEX4 and DOMEXM nothing — **not selectable in the program yet** |
| THOR | **every variant 0/4 — it has never read a real signal** |
| RTTY | 45/50/75 at their own shifts; SYNOP prints a whole bulletin (the parser is not wired to it) |
| other | FSQ alphabet fixed, decode unproven; ARDOP reads its reference WAVs but not a station; NAVTEX, DSC, WEFAX, SSTV, ACARS, ADS-B, AIS, VDL2, HFDL, DAB, DRM, HD Radio, RADE, packet, APRS, JS8, FT8/FT4/FT2, JT65/JT9, FST4, Q65, MSK144, FSK441, UVPacket, JTTY, ALE all have their own state in the journal |

### Decoded but **not selectable** — the cheapest win in the tree

`BPSK63F`, the single-carrier `R` family and the `C`/`RC` banks have receivers in
`sdroxide-dsp` and tests against the corpus, and **no `Mode` or chip reaches
them**. They are the shortest path from "works in a test" to "works in the
program". The shape to copy is MFSK: one mode with a variant list
(`DigiConfig::mfsk_mode`), not one mode per variant.

### Missing entirely, with recordings already in `samples/fldigi/`

**8PSK** (10 recordings), **OFDM** (3), **DominoEX** (9 — the decoder works),
**THROB** (3), **SCAMP** (6), and SITOR-B (which is NAVTEX's waveform; we have
NAVTEX and no SITOR). The manifest names all 157 recordings.

### Missing, and fldigi has them but no recording exists

`RS_ID` / `CALL_ID` (a watermark burst — small, and it makes every other mode
nicer), `DTMF`, and the `IFKP` picture family. These need a synthesised test
(fldigi's own table plus a loopback) and then a capture.

### What transmits today

RTTY, Olivia, PSK (all six speeds, both constellations), THOR, CW, FSK441, JTTY,
SSTV/SSTV-FM, FSQ, RF Paint, RADE, packet (KISS), APRS, FT8/FT4/FT2/JS8,
AtChat, Hell, ALE.

### What refuses the key-up, and why (`Mode::is_rx_only`)

| mode | reason on the chip |
|---|---|
| MFSK | modulator not written (receiver is) |
| Contestia | *scaffolding is in, does not read yet* — see below |
| MT63 | modulator not written |
| ARDOP | modem only, no ARQ |
| STANAG 4285 | no modulator, and the coded rates are not built |
| NAVTEX, DSC, ACARS, VDL2, AIS, ADS-B, HFDL, ISB, C-QUAM, HD Radio, PI4, MSK144, JT65, JT9, FST4, Q65, UVPacket | receive-only by capability or by choice — **read each mode's own doc comment before changing anything**; several are receive-only because transmitting them would put false information on a service channel |

**Contestia TX is half-built**: `OliviaTx::contestia` exists with fldigi's own
numbers (six bits per character, a 32-symbol block, `0xEDB88320`, `nShift` 5, and
the bank half a spacing lower), and its loopback reads **idle characters** — so
one convention is still wrong. The test `a_contestia_transmission_reads_back_through_the_controller`
is `#[ignore]`d with that sentence in it, and the panel still refuses the key-up.

**THOR is isolated.** Its audio front end is the one DominoEX shares and works
(418 sane steps on THOR8, with fldigi's preamble sweep at the head). What produces
*no characters* is the branch after it: the four soft bits per symbol, the
`interleave(4, idepth)` and the K = 7 Viterbi.

## The rule that changed: port → bench → **vendor**

The 2026-10-21 instruction was *"port it, do not vendor it"*, and the reason is in
[`MODE-ROADMAP.md`](MODE-ROADMAP.md): a vendored mode is a dependency nobody here
can fix. **The operator has changed it for this phase:** *"If a mode really
doesn't work, you have exhausted your resources or bench tests on it, then vendor
it. I'd rather ship a vendored mode than none."*

So the order is **port, then bench, then vendor**, and vendoring has conditions of
its own:

* **The licence must be compatible** — we are GPL-3.0-or-later. fldigi is GPL-3,
  OnAir is GPL-3, `mbelib-neo` is GPL-2.0-or-later (compatible). A project with
  **no licence at all** is not vendorable, and that is not a judgement call.
* **The engine must read its recording before it is wired.** A vendored decoder
  that does not decode is worse than none: it costs the dependency *and* the
  feature. `crates/sdroxide-olivia` is the pattern to copy — a `vendor/`-style
  submodule or a pinned copy, a `build.rs` and a `cxx`/FFI shim, and a test that
  decodes its own sample.
* **Attribution where it goes**: the README's Acknowledgements, and a comment at
  the top of the file naming what it was learned from and under which licence. No
  third-party project's name in a filename.
* **Say "vendored" in the table.** That column exists so the next person knows
  what they are maintaining.

## The order of work

**One item at a time, each ending in a measurement.** Do not start the next until
the current one has an entry in the journal.

### 1. THOR's FEC branch

The audio front end is proven (shared with DominoEX, which now reads text). What
is left is one branch: four soft bits per symbol → `interleave(4, idepth)` →
K = 7 Viterbi → MFSK varicode. **Build a synthetic THOR from fldigi's encoder and
diff it**, exactly as DominoEX was solved: fldigi's `src/thor/thor.cxx`
(`decodePairs`, `decodesymbol`, `sendMuPsk`) is on disk. THOR32 and THOR56 have
no variant in our table at all — add them while you are in there.

### 2. Contestia TX, then MFSK TX, then MT63 TX

* **Contestia** — one convention is wrong. Compare its block against the C
  receiver in `crates/sdroxide-olivia/cxx/pj_mfsk.h` (`EncodeBlock`,
  `DecodeCharacter`): the bit-interleave, the `IFHT` over 32 points, the `Char`
  mapping and the sign of the negative rows.
* **MFSK** — the receiver is done and reads its corpus; the modulator is not
  written. fldigi's `MFSK_Transmitter` is in the vendored `pj_mfsk.h` and is the
  reference; a loopback plus one off-air capture is the gate.
* **MT63** — same shape. Start with 2KS (the receiver is clean there).

### 3. Wire what already decodes

`BPSK63F`, the `R` family and the `C`/`RC` banks as chips under `Mode::Psk` — the
MFSK pattern. Then **DominoEX** as its own mode with its nine variants (its
decoder reads text; the corpus test asserts DOMX11 at 4/4).

### 4. The modes that do not exist yet

**8PSK** and **OFDM** are fldigi's own modem families with recordings in hand;
**THROB** and **SCAMP** likewise. Do them by the same discipline: read the
reference, port the modem, prove it on its recording, wire it. If a port stalls
after both a real attempt and a bench capture, **vendor it** — fldigi's C++ for
that mode family is the candidate, and `sdroxide-olivia` shows how.

### 5. RS_ID / CALL_ID, DTMF

No recordings exist, so build the test signal from fldigi's own table
(`src/rsid`, `src/dtmf`) and prove a loopback first; then ask for one capture.

### 6. TX for the rest of the receive-only set, judged one at a time

MFSK/Contestia/MT63/ARDOP/STANAG 4285 are the ones that *can* transmit and do not
yet. The others are receive-only by capability or by choice — **read the mode's
own doc comment before touching it**, and where the answer is "this station must
not transmit it" (NAVTEX, DSC), leave it alone and say why in the table.

## Instruments, and where they are

* **fldigi 4.2.13 source at `/tmp/opencode/fldigi-4.2.13`** (GPL-3). If `/tmp` has
  been cleared, get it again — it is the reference for every mode here:
  `git clone --depth 1 --branch 4.2.13 https://github.com/w1hkj/fldigi`.
* **The corpus: `samples/fldigi/`**, 157 recordings with `manifest.jsonl` carrying
  each mode's **known text**, its measured centre and its bandwidth. These are
  **bench recordings** — fldigi → the CRT SS9900v through VOX → the RSP1 with a
  stub at 27.415 MHz — so a decoder that reads one has read a real path.
* **`tools/bench/bench.py`** — fldigi over XML-RPC → RSP1 → decode, PASS or FAIL.
* **The probes that already exist**: `examples/ifk_probe` (tone steps and
  positions), `examples/s4285_probe`, `examples/s4285_test_signal` (a coded
  STANAG transmission as a WAV), `examples/s4285_fec_check`, `examples/olivia_tx_probe`,
  `examples/corpus_survey`. Write a probe when the text cannot say which symbol
  was wrong — that is what they are for.
* **The bench**: the RSP1 (SDRplay API service, `sdrplay_apiService`), the CRT
  SS9900v on 27.415 USB at 1–2 W through an 11 m antenna, `tools/rsp1-capture`.
  **The rig's SSB response is 450–2500 Hz** (its own manual) and its `TONE` menu
  does not widen it — a 2.4–2.7 kHz signal cannot be sent through it, measured on
  a STANAG 4285 loopback (references 0.9, data 0.25, no decode). Any TX test
  needing more than ~2 kHz of audio must use a different route or be a loopback.

## Do not re-derive — each of these cost a day

* **A metric that a random signal passes is not a metric.** "What share of tone
  steps are legal" is passed by **noise 89 % of the time**; it was chased for a
  long time. The only trustworthy measures are *the known text*, or the tone
  sequence **cross-correlated with the sequence the text implies**.
* **Compare against the reference's tables, not a port's.** Our DominoEX alphabet
  came from OnAir and agreed with fldigi's `src/dominoex/dominovar.cxx`
  (`varicode[][3]`, primary 256 rows) for eight entries before diverging.
* **IFK numbers that are now correct**: `paths = 5` at every variant (3 is fldigi's
  slow-CPU path), `extones` 9; Contestia is 6 bits/char, 32-symbol block,
  `0xEDB88320`, `nShift` 5, and its bank sits `bandwidth/2` below the centre.
* **IFK timing is transition-based**, `dominoex::synchronize()`: keep two symbols
  of tone energies, find where the **previous** tone peaked, filter that error and
  apply it to a **fractional** decision position. Peak sharpness has nothing to
  lock to.
* **THOR's varicode is MFSK's primary set plus a 12-bit secondary extension**
  (`src/thor/thorvaricode.cxx`).
* **MT63's +32 character shift is a codeword negation** — row `c+32` is `-row c`
  in the Sylvester construction — i.e. one symbol's polarity, not an interleaver
  fault.
* **The corpus is a bench recording**: a mode that reads its sample has read a
  real receiver's output, noise and all. A mode that does not may still be the
  decoder (DominoEX was) — build a synthetic from the reference and find out
  which side is wrong before touching either.
* **A shipping mode that does not work must say so on its chip** — Olivia, DAB
  (which was wrongly labelled experimental *after* being field-confirmed), STANAG
  4285 and THOR all carry theirs. That is this fork's rule and it is not optional.

## Rules

* **Commit locally. Nothing is pushed. No tag is cut, ever, by an agent.**
* `cargo check --workspace --all-targets` **silent**, `rustfmt` clean on what you
  touch, the touched crates' tests green, and the wasm check when the UI moves.
* **Every fix gets a test verified to fail on the unfixed code.**
* **No wire change without the register**: append at the tail, `PROTO_VERSION`
  **and** `VERSION_BYTE`, and a register line naming both.
* **Journal in [`MODE-PROGRESS.md`](MODE-PROGRESS.md)**, one entry per stage with
  the measurement in it. A verdict without a number is not an entry.
* **Not in scope and not to be touched: WEFAX, DAB, SSTV and DSC** — field-confirmed
  by a whole radio group.
* Where a mode is only partly proven, **say so where it is picked**, on the chip.
* If something needs the operator, put it under **"Needs me"** in the journal and
  carry on with the next item.

## The deliverable: the table

When the work is done, put this in [`MODE-PROGRESS.md`](MODE-PROGRESS.md) (and the
manual's mode table) with **every mode and every variant we ship**:

| mode | rx | tx | vendored | tested |
|---|---|---|---|---|
| e.g. `PSK125R` | recording | loopback | — | 394/394 bits vs known text |
| e.g. `Olivia 16/500` | air | loopback | Jalocha `MFSK_Receiver`, GPL-3, `crates/sdroxide-olivia` | reads an off-air capture |
| e.g. `THOR8` | no | no | — | 0/4 words; FEC branch open |

What each column means, so the table cannot be read two ways:

* **name** — as the chip reads it, variants included.
* **rx** — `no` · `loopback` · `recording` (its own corpus sample) · `air` (an
  off-air capture).
* **tx** — `no` · `loopback` (our receiver reads our transmitter) · `recording`
  (a recording of ours that decodes) · `air` (**another station or program copied
  us** — the only one that proves interop).
* **vendored** — `—` when the code is ours, otherwise the upstream project, its
  licence and the crate.
* **tested** — the one-line evidence, **with the number**.

## Stop when

**Every mode fldigi can send *and* receive is either shipped with that capability,
or is in the table with the measurement that says why not** — and the table, the
journal and the code agree with each other. Then write the release note in the
short style the operator asks for, and leave the tag to him: he cuts the
pre-releases, and a full release is cut when he says so.
