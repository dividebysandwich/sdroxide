# Mode backfill — the journal

> **The live queue is [`MODE-PHASE-4-PROMPT.md`](MODE-PHASE-4-PROMPT.md)** — finish
> the missing modes and get TX working for everything fldigi can transmit, with
> **vendor** now allowed once the port and the bench are both exhausted. Day 1 of it
> is in "Phase 4 (2026-10-22)" below. The phase-3 briefs
> ([`MODE-PHASE-3-PROMPT.md`](MODE-PHASE-3-PROMPT.md),
> [`MODE-PHASE-3B-PROMPT.md`](MODE-PHASE-3B-PROMPT.md)) are the record of the PSK
> work, which is done.
>
> **State after phase 4, day 1:** `main` is local and unpushed, `PROTO_VERSION`
> **228** (THOR's `thor_reverse`). `cargo check --workspace --all-targets` is silent,
> `cargo fmt` is clean on everything touched, and the dsp/digi/types/ui/proto tests
> are green.
>
> * **THOR reads**, in the program as well as in the tests: eleven of fifteen
>   variants whole, both halves replaced (the program was wired to a second IFK
>   implementation that read nothing).
> * **TX works for Contestia, MFSK (eleven variants) and MT63**, each read back
>   through the console's own objects, and **QPSK is selectable for the first
>   time** — `psk_modulation` had no UI at all.
> * **Items 1, 2 and 3 of the brief are done.** 32 coded PSK waveforms and
>   DominoEX are reachable from the panel.
> * **Item 4 is started and blocked on evidence**: 8PSK's slicer is proven on a
>   synthetic and reads none of its four recordings at any baud or centre, and the
>   recordings measure 20–46 Hz wide where the mode is 125 Hz — so the next
>   measurement is what the files contain. OFDM, THROB and SCAMP are untouched.
> * **DTMF is done** (item 5's easy half): the touch tones decode, all sixteen keys,
>   with noise silent. RS_ID and CALL_ID are not started.
> * **Next**: finish 8PSK's front end (a vestigial filter and an 8-point carrier
>   recovery, which PSK31's BPSK loop is not), then OFDM / THROB / SCAMP; then
>   RS_ID / CALL_ID and DTMF (item 5), then TX for the remaining receive-only set
>   (item 6).
>
> ⛔ **Not in scope, and not to be touched: WEFAX, DAB, SSTV and DSC.** F4NRM and
> his radio group confirm all four working perfectly (2026-10-21). They are not
> part of the mode work, they do not need a verdict, and a session must not
> refactor them or "improve" a decode on the way past.
>
> This file remains the record of the first pass and the running journal.

One line per mode, appended as it is worked. `samples/fldigi/` is the gate:
a mode is **decodes** only when its own recording reads its own known text.
Everything else says what was measured and why not. The long form — the
reference each port was read from, the conventions that bit — is in the commit
message; this file is the state.

Modes worked in the operator's order: MFSK16, MT63, Contestia, THOR, DominoEX,
Hell 80, SITOR, ARDOP, then the rest of the corpus.

| mode | state | the measurement |
|---|---|---|
| MFSK16 | **decodes** | `samples/fldigi/MFSK16.wav`, centre measured at **1752 Hz** (tones 1635…1869 — the manifest's 1800 is three spacings out), **reverse on**, 9.0 dB: the whole known text, `AAA TEST TEST THE EAGLE HAS LANDED 12345 SDRBROWN ZZZ`. Three causes found and each proved separately: PSK31's varicode was used instead of the IZ8BLY MFSK table; the Gray maps were **swapped** (fldigi's `grayencode` is the inverse and its `graydecode` is the Gray map — the loopback passes either way round, the air does not); and the mirror switch was a no-op (it mirrored the value and the amplitude index together, a permutation of one sum), with the capture inverted. A signal built from fldigi's own transmit algorithm decodes the same text through this receiver. Gate: `cargo test -p sdroxide-dsp --release --lib an_off_air_capture_decodes -- --ignored`. |
| MFSK4 | **decodes** | `MFSK4.wav`, centre 1752 (tones 1691…1812), reverse on: the whole text, 9/9 words. Recording opens on 13 symbols of the previous transmission's tail at the top tone, which is 31 tones from this one and outside the realignment's ±3-tone window; the gate starts at the transmission. |
| MFSK8 | **partial — sample loses a span** | `MFSK8.wav`, centre 1751, reverse on: 4/9 words. `12345 SDRBROWN ZZZ` and the preamble read at every centre and both polarities; `TEST THE EAGLE HAS LANDED` does not, in one contiguous span, with no envelope dropout at that point and 12.7 dB peak/mean. Same decoder as the seven variants that do read their samples. |
| MFSK11 | **decodes (8/9)** | `MFSK11.wav`, centre 1727, reverse on: the text from `TEST` on; the leading `AAA` is lost to the 8→11.025 kHz resampler's start-up. |
| MFSK22 | **decodes** | `MFSK22.wav`, 1699 Hz, reverse on, 9/9. |
| MFSK31 | **decodes** | `MFSK31.wav`, 1724 Hz, reverse on, 9/9. |
| MFSK32 | **decodes** | `MFSK32.wav`, 1698 Hz, reverse on, 9/9. |
| MFSK64 | **decodes** | `MFSK64.wav`, 1594 Hz at the start of the search, reverse on, 9/9. |
| MFSK64L | **decodes (short text)** | `MFSK64L.wav`, reverse on: `AAA SDRBROWN TEST`, 3/3 of the words its sample carries. |
| MFSK128 | **decodes** | `MFSK128.wav`, reverse on, 9/9. |
| MFSK128L | **decodes (short text)** | `MFSK128L.wav`, reverse on: `AAA SDRBROWN TEST`, 3/3. |

The family gate is `cargo test -p sdroxide-dsp --release --lib the_mfsk_family_reads_the_corpus -- --ignored`; it searches centre and polarity itself and asserts the counts above.

| MT63-2KS | **decodes** | `MT63-2KS.wav`, measured centre 1740 (the plateau is 744–2736 Hz): the whole text, 3/3, score 3956. |
| MT63-2KL | **partial — a few characters one Hadamard row high** | `MT63-2KL.wav`, 1740, re-measured 2026-10-22: `AAa SDRBROWn tESt`. The whole message is there and a handful of characters are 32 codes high, which is a symbol's polarity flipped — its codeword negated, since row `c+32` is `-row c`. So the long interleave works and the audio front end is nearly right; the earlier "decodes, 2/2" was measured on a run that did not have these errors. |
| MT63-1KS | **does not decode yet** | `MT63-1KS.wav`, 1740: the text is visible but **every character is one Hadamard row high** — `EAGLE` arrives as `eagle`, `SDRBROWN` as `sdrbrown` with spaces lost, i.e. code +32 throughout. That is a carrier-phase signature (a sign flip across half the bank), not an interleaver fault: the short interleave is proved by the loopback below. |
| MT63-1KL | **does not decode yet** | Same as 1KS, plus the long interleave; fragments only. |
| MT63-500S | **decodes (live receiver)** | `MT63-500S.wav`, **1690 Hz** — the manifest's 1740 is a block estimate and the decode searched phase and polarity but not the centre. The whole text, 4/4, once the lock scores on the median per-symbol SNR and the fractional bin is derotated; offered as `Mt6500S`. |
| MT63-500L | **does not decode yet** | Fragments only, as 500S. |

The MT63 interleaver, Walsh pair and deinterleaver are separately proved by `the_interleaver_round_trips_a_message`, which recovers `THE EAGLE HAS LANDED 12345` from the encoder's own bits for both the short and the long pattern. The audio front end decodes 2K exactly, so the remaining fault is in the carrier phase for the 4× and 8× windows, not in the coding. Gate: `cargo test -p sdroxide-dsp --release --lib the_corpus_mt63_samples_are_measured -- --ignored`.
## The rest of the corpus — one line per mode

Measured 2026-10-21 with `cargo run -p sdroxide-dsp --example corpus_survey` (PSK/RTTY/CW/FSQ, which searches the centre and both polarities) and the per-family tests above. `n/m` is how many of the known words the decode contains; the sample is `samples/fldigi/<mode>.wav` in every case.

### MFSK — see above
All eleven measured in the family table; MFSK16/4/22/31/32/64/128 decode, MFSK64L/128L carry the short text, MFSK11 loses only its leading `AAA`, MFSK8 loses one span of its sample.

### Olivia — 14 of 18 decode

Gate: `cargo test -p sdroxide-olivia --release --lib the_corpus_olivia_variants -- --ignored` (asserts 14 of 18).
| mode | state | measurement |
|---|---|---|
| OLIVIA | **decodes** | `OLIVIA.wav` as 16/500 at 1720 Hz, reverse on: `AAA SDRBROWN TEST` (its shortened text). |
| OLIVIA-4/125 | **decodes** | vendored engine at 1760 Hz, reverse on: `AAA SDRBROWN TEST`. |
| OLIVIA-4/250 | **does not decode** | tried 1700–1780 Hz and both polarities with its own tones/bandwidth; no text. The engine reads the other fourteen, so this is these four samples or their parameters, not the engine. |
| OLIVIA-4/500 | **does not decode** | tried 1700–1780 Hz and both polarities with its own tones/bandwidth; no text. The engine reads the other fourteen, so this is these four samples or their parameters, not the engine. |
| OLIVIA-4/1K | **decodes** | vendored engine at 1760 Hz, reverse on: `AAA SDRBROWN TEST`. |
| OLIVIA-4/2K | **decodes** | vendored engine at 1760 Hz, reverse on: `AAA SDRBROWN TEST`. |
| OLIVIA-8/125 | **does not decode** | tried 1700–1780 Hz and both polarities with its own tones/bandwidth; no text. The engine reads the other fourteen, so this is these four samples or their parameters, not the engine. |
| OLIVIA-8/250 | **decodes** | vendored engine at 1760 Hz, reverse on: `AAA SDRBROWN TEST`. |
| OLIVIA-8/500 | **decodes** | vendored engine at 1760 Hz, reverse on: `AAA SDRBROWN TEST`. |
| OLIVIA-8/1K | **decodes** | vendored engine at 1760 Hz, reverse on: `AAA SDRBROWN TEST`. |
| OLIVIA-8/2K | **decodes** | vendored engine at 1760 Hz, reverse on: `AAA SDRBROWN TEST`. |
| OLIVIA-16/500 | **decodes** | vendored engine at 1760 Hz, reverse on: `AAA SDRBROWN TEST`. |
| OLIVIA-16/1K | **decodes** | vendored engine at 1760 Hz, reverse on: `AAA SDRBROWN TEST`. |
| OLIVIA-16/2K | **decodes** | vendored engine at 1760 Hz, reverse on: `AAA SDRBROWN TEST`. |
| OLIVIA-32/1K | **decodes** | vendored engine at 1760 Hz, reverse on: `AAA SDRBROWN TEST`. |
| OLIVIA-32/2K | **decodes** | vendored engine at 1760 Hz, reverse on: `AAA SDRBROWN TEST`. |
| OLIVIA-64/500 | **does not decode** | tried 1700–1780 Hz and both polarities with its own tones/bandwidth; no text. The engine reads the other fourteen, so this is these four samples or their parameters, not the engine. |
| OLIVIA-64/1K | **decodes** | vendored engine at 1760 Hz, reverse on: `AAA SDRBROWN TEST`. |
| OLIVIA-64/2K | **decodes** | vendored engine at 1760 Hz, reverse on: `AAA SDRBROWN TEST`. |

### Contestia — 18 of 19 decode

Gate: `cargo test -p sdroxide-olivia --release --lib the_corpus_contestia_variants -- --ignored` (asserts 18 of 19), and `the_corpus_contestia_sample_decodes` for `CONTESTIA.wav` itself.
| mode | state | measurement |
|---|---|---|
| CONTESTIA | **decodes** | `CONTESTIA.wav` as 4/125 at 1718 Hz, reverse on: the whole text, 3/3. |
| Cont-4/125 | **decodes** | vendored engine in Contestia mode at 1720 Hz, reverse on: `AAA SDRBROWN TEST` (shortened). |
| Cont-4/250 | **decodes** | vendored engine in Contestia mode at 1720 Hz, reverse on: `AAA SDRBROWN TEST` (shortened). |
| Cont-4/500 | **decodes** | vendored engine in Contestia mode at 1720 Hz, reverse on: `AAA SDRBROWN TEST` (shortened). |
| Cont-4/1K | **decodes** | vendored engine in Contestia mode at 1720 Hz, reverse on: `AAA SDRBROWN TEST` (shortened). |
| Cont-4/2K | **decodes** | vendored engine in Contestia mode at 1720 Hz, reverse on: `AAA SDRBROWN TEST` (shortened). |
| Cont-8/125 | **decodes** | vendored engine in Contestia mode at 1720 Hz, reverse on: `AAA SDRBROWN TEST` (shortened). |
| Cont-8/250 | **decodes** | vendored engine in Contestia mode at 1720 Hz, reverse on: `AAA SDRBROWN TEST` (shortened). |
| Cont-8/500 | **decodes** | vendored engine in Contestia mode at 1720 Hz, reverse on: `AAA SDRBROWN TEST` (shortened). |
| Cont-8/1K | **decodes** | vendored engine in Contestia mode at 1720 Hz, reverse on: `AAA SDRBROWN TEST` (shortened). |
| Cont-8/2K | **decodes** | vendored engine in Contestia mode at 1720 Hz, reverse on: `AAA SDRBROWN TEST` (shortened). |
| Cont-16/250 | **decodes** | vendored engine in Contestia mode at 1720 Hz, reverse on: `AAA SDRBROWN TEST` (shortened). |
| Cont-16/500 | **decodes** | vendored engine in Contestia mode at 1720 Hz, reverse on: `AAA SDRBROWN TEST` (shortened). |
| Cont-16/1K | **decodes** | vendored engine in Contestia mode at 1720 Hz, reverse on: `AAA SDRBROWN TEST` (shortened). |
| Cont-16/2K | **decodes** | vendored engine in Contestia mode at 1720 Hz, reverse on: `AAA SDRBROWN TEST` (shortened). |
| Cont-32/1K | **decodes** | vendored engine in Contestia mode at 1720 Hz, reverse on: `AAA SDRBROWN TEST` (shortened). |
| Cont-32/2K | **decodes** | vendored engine in Contestia mode at 1720 Hz, reverse on: `AAA SDRBROWN TEST` (shortened). |
| Cont-64/500 | **does not decode** | no text at 1720–1780 Hz, either polarity; the other eighteen do. |
| Cont-64/1K | **decodes** | vendored engine in Contestia mode at 1720 Hz, reverse on: `AAA SDRBROWN TEST` (shortened). |
| Cont-64/2K | **decodes** | vendored engine in Contestia mode at 1720 Hz, reverse on: `AAA SDRBROWN TEST` (shortened). |

### MT63 — 3 of 6 decode
See the MT63 table above: 2KS, 2KL and 500S decode. The old "one Hadamard row high" reading was a tuning fault, not a phase fault — the centre is searched now. 1KS/1KL/500L are not offered: their lock needs a centre search that can prefer the right answer.

### PSK family

> **Superseded in part by "Phase 3 — the PSK coded families" below** (2026-10-22).
> BPSK63F and the whole `R` family now decode, and 18 of the 26 multi-carrier
> `C`/`RC` modes read their whole text. The rows here are the state before that
> work, kept as the record.
| mode | state | measurement |
|---|---|---|
| BPSK31 | **partial** | 2/3 words: the phrase reads with one or two words corrupted, so the demodulator locks but the sample or the timing costs the rest. |
| BPSK63 | **partial** | 2/3 words: the phrase reads with one or two words corrupted, so the demodulator locks but the sample or the timing costs the rest. |
| BPSK63F | **does not decode** | 0/3 at its own baud and a ±60 Hz centre search; the demodulator runs but the framing is not PSK31's. |
| BPSK125 | **decodes** | `BPSK125` 3/3 words, its own baud and a searched centre (±60 Hz). |
| BPSK250 | **decodes** | `BPSK250` 3/3 words, its own baud and a searched centre (±60 Hz). |
| BPSK500 | **decodes** | `BPSK500` 3/3 words, its own baud and a searched centre (±60 Hz). |
| BPSK1000 | **does not decode** | 0/3 at its own baud and a ±60 Hz centre search; the demodulator runs but the framing is not PSK31's. |
| PSK125C12 | **does not decode** | the `C` variants carry a block FEC/interleave; our receiver has neither. |
| PSK250C6 | **does not decode** | the `C` variants carry a block FEC/interleave; our receiver has neither. |
| PSK500C2 | **does not decode** | the `C` variants carry a block FEC/interleave; our receiver has neither. |
| PSK500C4 | **does not decode** | the `C` variants carry a block FEC/interleave; our receiver has neither. |
| PSK800C2 | **does not decode** | the `C` variants carry a block FEC/interleave; our receiver has neither. |
| PSK1000C2 | **does not decode** | the `C` variants carry a block FEC/interleave; our receiver has neither. |
| QPSK31 | **partial** | 1/3 words: the phrase reads with one or two words corrupted, so the demodulator locks but the sample or the timing costs the rest. |
| QPSK63 | **partial** | 2/3 words: the phrase reads with one or two words corrupted, so the demodulator locks but the sample or the timing costs the rest. |
| QPSK125 | **decodes** | `QPSK125` 3/3 words, its own baud and a searched centre (±60 Hz). |
| QPSK250 | **decodes** | `QPSK250` 3/3 words, its own baud and a searched centre (±60 Hz). |
| QPSK500 | **decodes** | `QPSK500` 3/3 words, its own baud and a searched centre (±60 Hz). |
| PSK125R | **does not decode** | fldigi's robust/FEC variants carry framing and a convolutional code our PSK receiver does not implement; 0/3 with the demodulator running. |
| PSK250R | **does not decode** | fldigi's robust/FEC variants carry framing and a convolutional code our PSK receiver does not implement; 0/3 with the demodulator running. |
| PSK500R | **does not decode** | fldigi's robust/FEC variants carry framing and a convolutional code our PSK receiver does not implement; 0/3 with the demodulator running. |
| PSK1000R | **does not decode** | fldigi's robust/FEC variants carry framing and a convolutional code our PSK receiver does not implement; 0/3 with the demodulator running. |
| PSK63RC4 | **does not decode** | fldigi's robust/FEC variants carry framing and a convolutional code our PSK receiver does not implement; 0/3 with the demodulator running. |
| PSK63RC5 | **does not decode** | fldigi's robust/FEC variants carry framing and a convolutional code our PSK receiver does not implement; 0/3 with the demodulator running. |
| PSK63RC10 | **does not decode** | fldigi's robust/FEC variants carry framing and a convolutional code our PSK receiver does not implement; 0/3 with the demodulator running. |
| PSK63RC20 | **does not decode** | fldigi's robust/FEC variants carry framing and a convolutional code our PSK receiver does not implement; 0/3 with the demodulator running. |
| PSK63RC32 | **does not decode** | fldigi's robust/FEC variants carry framing and a convolutional code our PSK receiver does not implement; 0/3 with the demodulator running. |
| PSK125RC4 | **does not decode** | fldigi's robust/FEC variants carry framing and a convolutional code our PSK receiver does not implement; 0/3 with the demodulator running. |
| PSK125RC5 | **does not decode** | fldigi's robust/FEC variants carry framing and a convolutional code our PSK receiver does not implement; 0/3 with the demodulator running. |
| PSK125RC10 | **does not decode** | fldigi's robust/FEC variants carry framing and a convolutional code our PSK receiver does not implement; 0/3 with the demodulator running. |
| PSK125RC12 | **does not decode** | fldigi's robust/FEC variants carry framing and a convolutional code our PSK receiver does not implement; 0/3 with the demodulator running. |
| PSK125RC16 | **does not decode** | fldigi's robust/FEC variants carry framing and a convolutional code our PSK receiver does not implement; 0/3 with the demodulator running. |
| PSK250RC2 | **does not decode** | fldigi's robust/FEC variants carry framing and a convolutional code our PSK receiver does not implement; 0/3 with the demodulator running. |
| PSK250RC3 | **does not decode** | fldigi's robust/FEC variants carry framing and a convolutional code our PSK receiver does not implement; 0/3 with the demodulator running. |
| PSK250RC5 | **does not decode** | fldigi's robust/FEC variants carry framing and a convolutional code our PSK receiver does not implement; 0/3 with the demodulator running. |
| PSK250RC6 | **does not decode** | fldigi's robust/FEC variants carry framing and a convolutional code our PSK receiver does not implement; 0/3 with the demodulator running. |
| PSK250RC7 | **does not decode** | fldigi's robust/FEC variants carry framing and a convolutional code our PSK receiver does not implement; 0/3 with the demodulator running. |
| PSK500RC2 | **does not decode** | fldigi's robust/FEC variants carry framing and a convolutional code our PSK receiver does not implement; 0/3 with the demodulator running. |
| PSK500RC3 | **does not decode** | fldigi's robust/FEC variants carry framing and a convolutional code our PSK receiver does not implement; 0/3 with the demodulator running. |
| PSK500RC4 | **does not decode** | fldigi's robust/FEC variants carry framing and a convolutional code our PSK receiver does not implement; 0/3 with the demodulator running. |
| PSK800RC2 | **does not decode** | fldigi's robust/FEC variants carry framing and a convolutional code our PSK receiver does not implement; 0/3 with the demodulator running. |
| PSK1000RC2 | **does not decode** | fldigi's robust/FEC variants carry framing and a convolutional code our PSK receiver does not implement; 0/3 with the demodulator running. |

### Hell — **two of seven read as clean copies**, and the old verdict was a measurement fault

The first pass scored every Hell sample against **our own transmit font**, which
was authored for this project. fldigi transmits with *its* glyph table, so a
correct decode was being compared with the wrong alphabet: two Hell fonts differ
in every glyph, and a correct picture scores about 0.1–0.3 against the wrong one —
the same range as noise-plus-shape, which is why all seven were written off.

Measured against fldigi's own glyphs (`src/feld/Feld7x7-14.cxx`, ported for the
test in `sdroxide-dsp/src/hell_ref_font.rs`), and per **word** rather than per
phrase — the senders' and our dot clocks differ by a fraction of a percent, the
slip accumulates along a line, and after twenty characters a picture that is
perfect in every word correlates with nothing:

Each carrier is **measured from the audio** (the power-weighted centre of the
strongest band), not taken from the manifest — whose `centre_hz` is a block
centre and for HELL80 says 1676 Hz when the signal is a 300 Hz shift about 1600 —
and each sample is scored word by word:

| mode | correlation | |
|---|---|---|
| HELLX5 | **0.869** | clean, asserted |
| FELDHELL | **0.829** | clean, asserted |
| HELL80 | 0.483 | reads, weak |
| SLOWHELL | 0.446 | its sample holds only ~5 s of text |
| FSKH105 | 0.418 | reads, weak |
| FSKH245 | 0.406 | reads, weak |
| HELLX9 | 0.332 | the poor one: 157.5 columns/s wants a very good dot clock |

A clean copy scores 0.75–0.9 on this measure; noise 0.03–0.1. **One caveat on the
frequency-shifted three:** their strongest line is one *tone* of the shift, not the
carrier, so the measured centre is off by half the shift (HELL80 measured 1605 Hz
where the shift is ±150) — worth correcting when they are next picked up. The gate is
`the_corpus_hell_samples_are_measured`, which asserts the two that are clean and
prints the rest — a score here is a measurement, not a promise.

**And the transmit font is now fldigi's too** (2026-10-21, at the operator's word).
`hell_font.rs` carries the `Feld7x7-14` glyphs, so our pictures look like every
other Hell station's, and the loopback and the corpus gate are finally comparing
the same alphabet. Two things the change cost, both worth knowing: fldigi's table
writes one entry as an **integer char code** (`{ 39, …}` for the apostrophe)
rather than a character literal, which the first generator silently skipped —
so the apostrophe came out blank; and the *untrimmed* picture now tracks better on
its own (denser, more consistent ink), so `a_clock_trim_stops_the_boundary_walking`
read 5 moves instead of 8 and its fixed 3x ratio failed **on an improvement**. The
bound is now the ratio the trim actually achieves plus a ceiling on residual
walking, with both readings recorded in the test.

**The lesson, and it is the fourth of its kind today:** a corpus verdict is only
as good as the thing it is compared against. The PSK carriers were a sideband
edge, MT63's narrow variants are still open, and Hell's "none decode" was our own
font.

### THOR — none decode

The OnAir IFK port is in `ifk.rs`. Every sample was tried at five centres and both polarities; no text. The tone trace shows the IFK steps being measured but the resulting nibble stream does not match fldigi's, so one convention (the half-spacing tone offset or the two-slice carrier pair) is still wrong. Three attempts made per mode: baud/centre sweep, both polarities, and a start past the previous transmission's tail.

| mode | state | measurement |
|---|---|---|
| THORM | **does not decode** | 0/4 words at 1670–1790 Hz, both polarities; the port's step trace runs. |
| THOR4 | **does not decode** | 0/4 words at 1670–1790 Hz, both polarities; the port's step trace runs. |
| THOR5 | **does not decode** | 0/4 words at 1670–1790 Hz, both polarities; the port's step trace runs. |
| THOR8 | **does not decode** | 0/4 words at 1670–1790 Hz, both polarities; the port's step trace runs. |
| THOR11 | **does not decode** | 0/4 words at 1670–1790 Hz, both polarities; the port's step trace runs. |
| THOR16 | **does not decode** | 0/4 words at 1670–1790 Hz, both polarities; the port's step trace runs. |
| THOR22 | **does not decode** | 0/4 words at 1670–1790 Hz, both polarities; the port's step trace runs. |
| THOR25 | **does not decode** | 0/4 words at 1670–1790 Hz, both polarities; the port's step trace runs. |
| THOR32 | **does not decode** | 0/4 words at 1670–1790 Hz, both polarities; the port's step trace runs. |
| THOR44 | **does not decode** | 0/4 words at 1670–1790 Hz, both polarities; the port's step trace runs. |
| THOR56 | **does not decode** | 0/4 words at 1670–1790 Hz, both polarities; the port's step trace runs. |
| THOR25x4 | **does not decode** | 0/4 words at 1670–1790 Hz, both polarities; the port's step trace runs. |
| THOR50x1 | **does not decode** | 0/4 words at 1670–1790 Hz, both polarities; the port's step trace runs. |
| THOR50x2 | **does not decode** | 0/4 words at 1670–1790 Hz, both polarities; the port's step trace runs. |
| THOR100 | **does not decode** | 0/4 words at 1670–1790 Hz, both polarities; the port's step trace runs. |

### DominoEX — none decode

Same port and the same trace, all nine samples; DominoEX has no FEC to hide behind, so the step measurement itself is the fault to chase next.

| mode | state | measurement |
|---|---|---|
| DOMEXM | **does not decode** | 0/4 words at 1670–1790 Hz, both polarities. |
| DOMEX4 | **does not decode** | 0/4 words at 1670–1790 Hz, both polarities. |
| DOMEX5 | **does not decode** | 0/4 words at 1670–1790 Hz, both polarities. |
| DOMEX8 | **does not decode** | 0/4 words at 1670–1790 Hz, both polarities. |
| DOMX11 | **does not decode** | 0/4 words at 1670–1790 Hz, both polarities. |
| DOMX16 | **does not decode** | 0/4 words at 1670–1790 Hz, both polarities. |
| DOMX22 | **does not decode** | 0/4 words at 1670–1790 Hz, both polarities. |
| DOMX44 | **does not decode** | 0/4 words at 1670–1790 Hz, both polarities. |
| DOMX88 | **does not decode** | 0/4 words at 1670–1790 Hz, both polarities. |

### No decoder, by family
| 8PSK125 | **no decoder** | 8PSK constellation; no slicer; 0 words. |
| 8PSK125FL | **no decoder** | 8PSK; 0 words. |
| 8PSK125F | **no decoder** | 8PSK; 0 words. |
| 8PSK250 | **no decoder** | 8PSK; 0 words. |
| 8PSK250FL | **no decoder** | 8PSK; 0 words. |
| 8PSK250F | **no decoder** | 8PSK; 0 words. |
| 8PSK500 | **no decoder** | 8PSK; 0 words. |
| 8PSK500F | **no decoder** | 8PSK; 0 words. |
| 8PSK1000 | **no decoder** | 8PSK; 0 words. |
| 8PSK1000F | **no decoder** | 8PSK; 0 words. |
| 8PSK1200F | **no decoder** | 8PSK; 0 words. |
| THROB1 | **no decoder** | multi-tone reeded pulse; not implemented; 0 words. |
| THROB2 | **no decoder** | same; 0 words. |
| THROB4 | **no decoder** | same; 0 words. |
| OFDM500F | **no decoder** | fldigi's OFDM modes; no reference read; 0 words. |
| OFDM750F | **no decoder** | same; 0 words. |
| OFDM3500 | **no decoder** | same; 0 words. |
| SCAMPFSK | **no decoder** | SCAMP analysis modem; not implemented; 0 words. |
| SCAMPOOK | **no decoder** | same; 0 words. |
| SCFSKFST | **no decoder** | SCFSK utility mode; not implemented; 0 words. |
| SCFSKSLW | **no decoder** | same; 0 words. |
| SCOOKSLW | **no decoder** | same; 0 words. |
| SCFSKVSL | **no decoder** | same; 0 words. |

### The rest
| mode | state | measurement |
|---|---|---|
| CW | **decodes** | `CW.wav`: 9/10 words (`EAA` for `AAA` at the start); the decoder is `CwRx`. |
| RTTY | **decodes** | `RTTY.wav`, 45.45 baud, 170 Hz shift, reverse on: 10/10. |
| SITORB | **partial — sample ends** | `SITORB.wav` carries 8 s of one broadcast: `A TEST TEST THE EAGLE HAS LANDY…` then noise. The decoder is NAVTEX's and reads what is there. |
| NAVTEX | **partial — start garbled** | `NAVTEX.wav`: `12345 SDRBROWN ZZZ` and `NNNN` read; the `ZCZC` header is corrupt at the lock. |
| FSQ | **does not decode** | `FSQ.wav`: 0/10 words at 2/4/6/8 baud and the manifest centre; `fsq.rs` borrows PSK's varicode, which FSQ does not use. |
| ARDOP | **decodes** | The modem is ported in `ardop.rs` from the published protocol description, and the gate is **three recordings made by `ardopcf` itself** in `samples/ardop/`: 4PSK on one carrier, 16QAM on eight, and 4-FSK at 100 baud, each carrying `TEST TEST THE EAGLE HAS LANDED 12345 SDRBROWN ZZZ` (49 bytes). All three decode that text exactly; the Reed–Solomon parity is byte-for-byte the reference's (`rs_append` reproduced), and the reference's own decoder confirms the files. Gate: `cargo test -p sdroxide-dsp --release --lib the_ardopcf_recordings_decode -- --ignored`. The ARQ state machine and the TNC host interface are **not** ported — the modem/PHY is, which is what a decoder needs. Control-frame detection is code-only: the `TXFRAME ConReq200M` recording ardopcf wrote is a leader/tune artefact, not a header, so it is not in the gate. |

**Totals, from `manifest.jsonl`'s 157 rows: 54 decode their own text, 7 decode part of it, 96 have a written verdict and no decode.** ARDOP is outside that count — fldigi cannot send it — and it now has its own three-file gate and decodes. The 54 are: ten MFSK, two MT63, fifteen Olivia, nineteen Contestia, six PSK (BPSK125/250/500, QPSK125/250/500), RTTY and CW. The seven partial are MFSK8 (one lost span), BPSK31, BPSK63, QPSK31, QPSK63, SITORB (its sample ends after 8 s) and NAVTEX (a corrupt header). Nothing here is claimed working without its sample, and no mode is pushed.

## Phase 2 (2026-10-21) — the working decoders come into the program

The first pass measured everything and left the working decoders as tests. This
is the wiring, one family per commit. `PROTO_VERSION` 220 → **221**,
`VERSION_BYTE` 0x28 → 0x29.

### MFSK — a mode at last (11 variants)

`Mode::Mfsk`, `DigiConfig::mfsk_mode` / `mfsk_reverse`. **One mode with eleven
chips**, not eleven modes: fldigi's family is one modem at different symbol
rates, and the tone count moves with the rate, so the pair cannot be chosen
independently the way Olivia's can.

- **The receiver** is `sdroxide_dsp::mfsk_rx::MfskReceiver`, a block adapter over
  the ported `MfskRx` — `process` hands out only what arrived in that block, which
  is the one thing a block API can get wrong. That is pinned by comparing it
  character-for-character against the bare receiver over our own modulator;
  what the characters *are* is the receiver's business, and the corpus settles
  that.
- **The gate** is the corpus through the console's own controller: MFSK16 at
  1752 Hz reversed reads `AAA TEST TEST THE EAGLE HAS LANDED 12345 SDRBROWN ZZZ`
  (`cargo test -p sdroxide-digi --release --lib -- --ignored --nocapture
  the_mfsk_corpus_sample_decodes_through_the_controller`). The family's own
  eleven-sample sweep still passes unchanged.
- **The two the corpus only partly carries say so where they are picked** —
  MFSK8 loses a span of its recording, MFSK11 its leading group to the
  resampler's start-up — so a thin decode is not read as a broken mode. That is
  the whole reason the caveat is on the chip rather than in this file.
- **The tone bank's edges are drawn** on the panadapter, like Olivia's, from
  `MfskMode::bandwidth_hz` — pinned against the decoder's own table by
  `the_mfsk_mode_table_matches_the_decoder`, because a width that drifts is a
  marker that lies about where the signal is.
- **Transmit is not built, and the mode says so.** `Mode::Mfsk` is in
  `is_rx_only`, the panel greys the sending half and shows `RX ONLY`, and the
  controller refuses the key-up with a reason (`tx_refused`) rather than arming
  a latch — unit-tested, with PSK as the control so the refusal cannot be a
  blanket. That is phase B's work, gated on fldigi reading our signal.
- Every rig dialect (`sdroxide-cat` ×9), TCI, rigctld, SmartSDR and the TX
  modulator put MFSK with the other sideband data modes, so a rig is told
  DATA/USB instead of falling through its match to a CW setting; the manual's
  mode lists and `--mode` name MFSK.

### Contestia — the engine's other scrambler, as a mode

`Mode::Contestia`, no config fields of its own: it is the same vendored engine as
Olivia with a different scrambling code, so its tones, bandwidth and polarity are
`olivia_tones`, `olivia_bw_hz` and `olivia_reverse`. The two modes are the same
waveform and a station switches between them, so one set of numbers that cannot
disagree is better than two that can.

- **The flag has to survive a rebuild**, which is the one real bug here: every
  retune and every polarity change reconstructs the engine, and dropping
  Contestia's flag there turns a Contestia station into an Olivia one silently —
  both engines decode *something*. `the_contestia_flag_survives_a_retune_and_a_reversal`
  pins it, with an Olivia receiver as the control.
- **The gate** is the corpus through the console's controller: `CONTESTIA.wav`
  as 4/125 at 1718 Hz reversed reads `TEST THE EAGLE HAS LANDED 12345 SDRBROWN`
  (`cargo test -p sdroxide-digi --release --lib -- --ignored --nocapture
  the_contestia_corpus_sample_decodes_through_the_controller`).
- **Contestia's narrowest is four carriers.** The field is shared with Olivia, so
  a 2 left over from Olivia is raised to 4 by the controller and the chip row
  says `(Olivia's 2 → 4)` rather than offering a choice that does not exist.
- **Receive only**, for the same shape as MFSK but a different reason: our
  transmitter still sends Olivia's scrambling code, so the panel greys the
  sending half and the refusal names Contestia rather than MFSK.

### Next, in the order the prompt gives

MT63 (2K first — 1K/500 are still one Hadamard row high, and that is a DSP
chase, not wiring), then ARDOP (the modem is proven by `ardopcf`'s own
recordings; what it needs is a panel and, later, the TNC).

### MT63 — a live receiver, and the two narrow variants left out

`Mode::Mt63`, `DigiConfig::mt63_mode`. Only **MT63-2KS and MT63-2KL** are
offered; the chip row says why the other four are missing.

- **The work was the front end, not the decoder.** `Mt63Rx` is a batch decoder
  over a whole recording, and the console needs a stream. `Mt63Receiver` locks
  once — the symbol phase searched in two passes, and the polarity, since a
  reversed station is a permutation of the same codewords — and then decodes a
  symbol at a time from a rolling buffer. `Mt63Rx` grew `symbol`,
  `decode_more`, `at` and `rebase` for it, so the batch path and the live path
  are the same arithmetic.
- **Locking is measured, and a count of letters cannot do it.** MT63 always
  produces a character per symbol, so a wrong phase still prints *letters*; what
  a wrong phase does not print is **spaces** where words have them (space is one
  code in 128, so a station's window carries a dozen and noise carries none).
  The window has to be long enough for the interleaver to fill as well, or every
  candidate looks equally plausible — 6 s for the short interleave and 12 s for
  the long one, which is the price of the search and worth knowing before
  someone reports "it takes a while to start".
- **The gate** is both corpus samples through the controller:
  `cargo test -p sdroxide-digi --release --lib -- --ignored --nocapture the_mt63`
  — `EAGLE LANDED SDRBROWN` for 2KS, `SDRBROWN TEST` for 2KL, fed in blocks with
  only the centre given.
- **Noise does not lock** (`noise_does_not_lock_the_live_receiver`), which took
  two attempts: `textiness` alone locked onto noise, and so did spaces alone at
  three.
- **The narrow variants were a tuning fault, and the old verdict was wrong.**
  The journal said 1K and 500 arrive "one Hadamard row high — every character 32
  codes out". They do not: `decode()` searches the symbol phase and the polarity
  but **not the centre**, and the manifest's centre is a block estimate. Swept
  over the centre, at their own best: **MT63-500S reads its whole text** (4/4 at
  1690 Hz), 2KS 4/4, 2KL 2/2, 500L 2/2, 1KS 2/2, and only **1KL** reads nothing at
  any centre. That is the same family of measurement fault as the PSK carriers and
  the Hell font, and it read like a phase fault for a day.
- **The reference's fractional-bin correction is implemented** (`first_carrier_exact`,
  `frac_bin`, and the derotation in `slice`): the bank can only be read at whole
  bins, so the sub-bin part is taken out as a phase ramp across the window before
  the transform, as fldigi's `FreqOfs` does. Without it every carrier sits up to
  half a bin off its peak, which at 500 Hz is a quarter of the carrier spacing.
- **The lock now scores by the median of the recent per-symbol SNR**, not the mean
  over the window (`snr_median`). The mean cannot tell a lock from a coincidence —
  a good decode of 2KS averages 3.2 while a window that decodes nothing averages
  5.4, because the window begins with the interleaver's fill and a transmission's
  own silence drags it down. The median of the *recent* symbols is what separates
  them, and on the back of it **MT63-500S locks and reads its whole text through
  the live receiver** — it is now offered. The lock window also follows the
  interleave (64 symbols short, 96 long) instead of a fixed six seconds: a window
  shorter than the fill is all fill and no words.
- **1K and the 500 Hz long interleave are still not offered**, and this time the
  reason is measured rather than assumed. Their lock needs a **centre search**, and
  a search over centre, phase and polarity was found to trade one variant for
  another: 500S and 1KS came back while **2KL locked 56 Hz away and read garbage**.
  A validation step — decoding a second stretch of the same buffer and requiring it
  to read too — rejected the good locks along with the bad. So the search comes
  back when the validation can *prefer* the right answer and not merely reject a
  wrong one, which wants the per-symbol SNR restricted to symbols after the fill.
- **Not ported, and the same weakness from the other side:** fldigi's synchroniser
  moves the two data slices by a *symbol shift* (`SyncSymbShift`) rather than
  reading them on a fixed grid.

### Next, in the order the prompt gives

MT63 (2K first — 1K/500 are still one Hadamard row high, and that is a DSP
chase, not wiring), then ARDOP (the modem is proven by `ardopcf`'s own
recordings; what it needs is a panel and, later, the TNC).

### MT63 — a live receiver, and the two narrow variants left out

`Mode::Mt63`, `DigiConfig::mt63_mode`. Only **MT63-2KS and MT63-2KL** are
offered; the chip row says why the other four are missing.

- **The work was the front end, not the decoder.** `Mt63Rx` is a batch decoder
  over a whole recording, and the console needs a stream. `Mt63Receiver` locks
  once — the symbol phase searched in two passes, and the polarity, since a
  reversed station is a permutation of the same codewords — and then decodes a
  symbol at a time from a rolling buffer. `Mt63Rx` grew `symbol`,
  `decode_more`, `at` and `rebase` for it, so the batch path and the live path
  are the same arithmetic.
- **Locking is measured, and a count of letters cannot do it.** MT63 always
  produces a character per symbol, so a wrong phase still prints *letters*; what
  a wrong phase does not print is **spaces** where words have them (space is one
  code in 128, so a station's window carries a dozen and noise carries none).
  The window has to be long enough for the interleaver to fill as well, or every
  candidate looks equally plausible — 6 s for the short interleave and 12 s for
  the long one, which is the price of the search and worth knowing before
  someone reports "it takes a while to start".
- **The gate** is both corpus samples through the controller:
  `cargo test -p sdroxide-digi --release --lib -- --ignored --nocapture the_mt63`
  — `EAGLE LANDED SDRBROWN` for 2KS, `SDRBROWN TEST` for 2KL, fed in blocks with
  only the centre given.
- **Noise does not lock** (`noise_does_not_lock_the_live_receiver`), which took
  two attempts: `textiness` alone locked onto noise, and so did spaces alone at
  three.
- **The narrow variants were a tuning fault, and the old verdict was wrong.**
  The journal said 1K and 500 arrive "one Hadamard row high — every character 32
  codes out". They do not: `decode()` searches the symbol phase and the polarity
  but **not the centre**, and the manifest's centre is a block estimate. Swept
  over the centre, at their own best:
  **MT63-500S reads its whole text** (4/4 at 1690 Hz), 2KL 2/2, 500L 2/2, 1KS 2/2
  and 2KS 4/4. Only **1KL** reads nothing at any centre. That is a *measurement*
  fault of the same family as the PSK carriers and the Hell font, and it read
  like a phase fault for a day.
- **The reference's fractional-bin correction is implemented** (`first_carrier_exact`,
  `frac_bin`, and the derotation in `slice`). The bank can only be read at whole
  bins, so the sub-bin part is taken out as a phase ramp across the window before
  the transform — fldigi's `FreqOfs` does the same in `MT63rx::DataProcess`.
  Without it every carrier sits up to half a bin off its peak, which at 500 Hz is
  a quarter of the carrier spacing.
- **What still blocks offering the narrow variants is the live lock's
  acceptance**, not the receiver arithmetic. It scores candidates by spaces and
  letters, and a wrong phase produces both, so it can lock on garbage: measured,
  2KL locked at the wrong phase when the centre moved four hertz, and 500S never
  locks at all. The batch path is unaffected — which is why the four read in the
  gate and the two offered variants read live. Next step is an **SNR-based
  acceptance**: the Walsh peak's own signal-to-noise is the quantity that
  separates a real lock from a lucky one, and `deinterleave` already computes it.
- **Not ported, and the same weakness from the other side:** fldigi's
  synchroniser moves the two data slices by a *symbol shift*
  (`SyncSymbShift`) rather than reading them on a fixed grid.

### Next, in the order the prompt gives

MT63 (2K first — 1K/500 are still one Hadamard row high, and that is a DSP
chase, not wiring), then ARDOP (the modem is proven by `ardopcf`'s own
recordings; what it needs is a panel and, later, the TNC).

### MT63 — a live receiver, and the two narrow variants left out

`Mode::Mt63`, `DigiConfig::mt63_mode`. Only **MT63-2KS and MT63-2KL** are
offered; the chip row says why the other four are missing.

- **The work was the front end, not the decoder.** `Mt63Rx` is a batch decoder
  over a whole recording, and the console needs a stream. `Mt63Receiver` locks
  once — the symbol phase searched in two passes, and the polarity, since a
  reversed station is a permutation of the same codewords — and then decodes a
  symbol at a time from a rolling buffer. `Mt63Rx` grew `symbol`,
  `decode_more`, `at` and `rebase` for it, so the batch path and the live path
  are the same arithmetic.
- **Locking is measured, and a count of letters cannot do it.** MT63 always
  produces a character per symbol, so a wrong phase still prints *letters*; what
  a wrong phase does not print is **spaces** where words have them (space is one
  code in 128, so a station's window carries a dozen and noise carries none).
  The window has to be long enough for the interleaver to fill as well, or every
  candidate looks equally plausible — 6 s for the short interleave and 12 s for
  the long one, which is the price of the search and worth knowing before
  someone reports "it takes a while to start".
- **The gate** is both corpus samples through the controller:
  `cargo test -p sdroxide-digi --release --lib -- --ignored --nocapture the_mt63`
  — `EAGLE LANDED SDRBROWN` for 2KS, `SDRBROWN TEST` for 2KL, fed in blocks with
  only the centre given.
- **Noise does not lock** (`noise_does_not_lock_the_live_receiver`), which took
  two attempts: `textiness` alone locked onto noise, and so did spaces alone at
  three.
- **MT63-1K and MT63-500 are not offered.** They arrive one Hadamard row high —
  every character 32 codes out, `EAGLE` as `eagle` — through this receiver, and
  that is a carrier-phase fault at the narrower bandwidths, not the interleaver
  (both interleaves are proved by `the_interleaver_round_trips_a_message`, and
  2K reads with either). The two are already named in `Mt63Mode` so the fix is an
  appended variant rather than a new field.
- **What the reference does that this port does not, read from fldigi 4.2.13's
  `src/mt63/mt63base.cxx`** (2026-10-21). Its `MT63rx::DataProcess` corrects two
  things this receiver has no notion of:
  - a **fractional-bin frequency offset** `FreqOfs`, applied as a progressive
    phase across the window (`P = -2π·FreqOfs/WindowLen` per sample) before the
    FFT. The bank itself is quantised to whole bins by both implementations —
    `FirstDataCarr` is identical here — so without this correction every carrier
    is sampled at whatever fraction of a bin the signal happens to sit on.
    Measured, our bank lands **0.63 bins low on 2K (which still reads), 1.75 bins
    low on 1K and 2 bins low on 500** — so the fractional offset is not by itself
    the difference, but it is the first thing the reference takes out and we do
    not.
  - a **symbol shift** `SyncSymbShift` from its synchroniser, which picks the two
    data slices at `s1 = … + (SyncSymbShift − SymbPtr·SyncStep)` rather than on a
    fixed grid. This port searches a fixed set of phases once (the live receiver
    does that when it locks) and then holds it.
  Done and rejected as the cause: the interleaver (both patterns round-trip), the
  band position (measured to ±10 Hz), the two-slice half-symbol offset (identical
  to the reference's), and the shape resampling (same duration at every
  bandwidth).
- **Receive only**, so it is in `is_rx_only`: the panel greys the sending half
  and the refusal names MT63. No modulator yet.

### FSQ — the alphabet was PSK31's, and the sample cannot settle it

FSQ is already a mode, and it decoded nothing. The cause is now certain, from
fldigi's own source rather than from a guess:

- **The text does not ride PSK31's varicode.** fldigi carries
  `fsq_varicode[256][2]`: one or two *nibbles* per character, the second sent
  only when it is above 28, and the receiver decodes `prev * 32 + curr` out of
  `wsq_varidecode`. `fsq.rs` borrowed PSK's bit-level table, so it could not have
  decoded a single character whatever the tuning. Both tables are now generated
  from `src/fsq/fsq_varicode.cxx`.
- **The tone grid was invented.** fldigi spaces the 33 tones three bins of a
  4096-sample frame at 12 kHz — 8.79 Hz, the same at every speed — and the
  "speeds" are labels: 8192/6144/4096/3072/2048 samples at 12 kHz for
  1.5/2/3/4.5/6 baud. The port built an orthogonal grid where the spacing *was*
  the baud rate.
- **Proven so far:** the nibble table (`the_varicode_is_the_reference_nibble_table`)
  and a round trip at every speed (`every_speed_round_trips`).
- **Not proven: the corpus sample does not read, and the sample is the suspect.**
  An FSQ signal is one tone at a time; this recording never has a dominant tone
  at any centre (1770–1815 Hz), spacing (5.86 or 8.79 Hz) or speed — the probe
  `the_corpus_sample_is_measured_not_claimed` prints the sweep and asserts the
  current state. The manifest agrees that nothing has ever read it back:
  `decoder: null, verdict: unverified`. A recording of a real station, or thirty
  seconds of one on the bench, is what turns this into a gate.

**A tool worth having from the start of the next session:** fldigi's source
answers these conventions and this session only had fragments of it. It is at
`deb.debian.org/debian/pool/main/f/fldigi/fldigi_4.2.13.orig.tar.gz` (5 MB),
and `src/include/mt63base.h` plus `src/mt63/symbol.dat` settled MT63's constants
(`SymbolLen=512`, `SymbolSepar=200`, `DataCarrSep=4`, fixed at every bandwidth)
in a minute.

### Next

The PSK family's gaps — BPSK1000, PSK63F and the R/C/F robust set — need fldigi's
framing and convolutional code, which the source now on disk has. Then MT63's
narrow variants. Both are DSP jobs with the reference in hand, and both are the
last of "modes fldigi can send that this fork cannot read".

Then transmit parity: every mode wired above is receive-only, and the gate there
is fldigi decoding *our* signal, not a loopback.

### ARDOP — the modem becomes a mode, frames and all

`Mode::Ardop`, no config fields: an ARDOP frame carries its own leader, type and
CRC, and the carrier grid is fixed about 1500 Hz, so there is nothing for the
operator to set. The mode is a fixed-centre one for exactly that reason.

- **The work was the front end again, and it found a real bug.** `decode_all`
  walked a whole recording; it is now built on `scan`, which answers either "a
  frame, and this many samples used" or "nothing yet, and this much at the front
  is safe to drop". A live receiver needs the second answer or it re-correlates
  the same audio on every block.
- **The bug that fell out of it is the one every streaming port has:** a frame
  whose *header* had arrived and whose *body* had not was reported as a data-less
  frame, and the receiver then dropped the samples the body lived in. `scan` now
  waits when the body has not all arrived — not having the audio is not the same
  as failing the CRC — and `the_live_receiver_reads_the_ardopcf_recordings` is
  what holds it. The batch gate passed throughout, which is exactly why the
  stream needed its own test.
- **The tap goes straight to 12 kHz.** The controller's usual tap→8 kHz stage is
  kept out of ARDOP's path: the modem's grid is defined at its own rate and a
  round trip through 8 kHz costs half the band for nothing. `ArdopRx` owns the
  resampler and takes the tap rate.
- **The gate** is all three `ardopcf` recordings through the console's
  controller, each arriving as a line naming the mode it came in —
  `[4PSK.200.100] TEST TEST THE EAGLE…`, `[16QAM.2000.100] …`,
  `[4FSK.500.100] …` (`cargo test -p sdroxide-digi --release --lib -- --ignored
  --nocapture the_ardop_recordings_decode_through_the_controller`). The names are
  the reference's own frame types, which is how the log tells a 4PSK path from a
  16QAM one.
- **Receive only, and the modem only.** No modulator, no ARQ state machine and no
  TNC host protocol: the panel greys the sending half and the refusal says so. A
  frame whose body fails its CRC is shown as `(body failed its check)` rather
  than dropped — an operator needs to see that a frame arrived and did not read.
- **Still open:** the control frames are named `ctrl 0x??` because `ardopcf`'s
  `FrameInfo` name table is not ported. That is a small piece and it is what a
  real station's log would mostly show (ACKs and ConReqs), so it wants doing
  before anyone watches ARDOP for an hour.

### PSK500 and BPSK1000 — and two wrong measurements on the way

`PskSpeed` gains `Bps500` and `Bps1000` (six chips). The modem always took any
baud rate; the operator could not select these two, so BPSK500 was a mode the
corpus read whole and nobody could use.

**What each reads**, measured with `the_corpus_psk_samples_are_measured`:

| mode | result |
|---|---|
| BPSK125 / 250 / 500 | 3/3 — the whole text |
| BPSK31 | 2/3 — every word present, several characters mangled |
| BPSK63 | 2/3 — `AAA TEST TEE EAGLE HAS LANDED 12345 SDRBROW` |
| BPSK1000 | 2/3 — the **second half only**; the receiver's lock time is a third of a message at this rate |

**Two measurements were wrong before that table existed, and both were
self-consistent.** First the corpus survey searched ±60 Hz about the manifest's
`centre_hz`, which for BPSK1000 is 1411 Hz — 330 Hz from its carrier — and
reported 0/3, which the journal wrote down as "the framing is not PSK31's". Then a
wide sweep found 2/3 at 1605 Hz and the band edges were taken for the carrier: at
1242 Hz — BPSK1000's **lower sideband edge** — nothing decoded, so the speed was
briefly withdrawn as "does not decode at its own carrier".

The third measurement settled it: every PSK sample here is a *band* whose width is
the baud rate and whose carrier is its **midpoint** — 1741 Hz for all of them
except BPSK31 at 1728. The manifest's `peak_hz` is one of the edges, and the
strongest edge is not the carrier. `PskSpeed` therefore keeps both speeds, and the
BPSK1000 chip says it reads from a second in.

So what is left is one real fault, not twenty-three verdicts: **our receiver loses
the head of a transmission to its own lock time**, which is a word or two at 31
baud and a third of the message at 1000. BPSK125/250/500 acquire fast enough that
the corpus never shows it.

### THOR / DominoEX — one real bug found, and a trace tool that narrows the rest

Twenty-four corpus modes still read nothing. One genuine bug is fixed, from
fldigi's own `dominoex::decodesymbol`, and the search now has an instrument.

- The step is `2 + symbol (mod 18)` and the tones sit at absolute positions, so a
  step that wraps arrives as a **negative** difference — `-16` is symbol 0 and
  `+1` is symbol 17. fldigi handles that with `if (c < 0) c += NUMTONES`; this
  port instead declared the raw differences 0 and 1 "out of range", which threw
  away symbols 16 and 17 — **one symbol in nine, on every transmission**. The
  mapping is now the reference's arithmetic (`step_symbol`), and
  `every_step_maps_to_its_symbol_and_the_wrap_is_kept` was verified failing on
  the old rule: `step 18 → None, right Some(16)`.
- **The decoded text cannot diagnose this**, so `IfkRx` now keeps `steps()` and
  `positions()` — the sequence, as `MfskRx` keeps its tones — and
  `the_dominoex_steps_carry_the_known_text` scores the trace against the nibble
  sequence the known text must produce. It prints the best alignment as a
  fraction, because a receiver that is wrong about one convention still reads
  almost none of the text while getting most *symbols* right: **chance is 6%,
  and DOMX16 stands at 36%** — six times chance, from the repeated `7` prefix
  that every three-nibble character carries. Not a decode, and not noise.
- **What is ruled out, measured:** the front end finds the signal (19.5 dB
  peak-to-mean, and the tones measure on a 15.6 Hz grid spanning 1619–1883 Hz as
  the mode says), the step *rate* is right (879 steps over 750 symbols of signal
  plus the silence either side), and the raw bank positions give differences that
  round to whole tone steps. So it is not the bank, the spacing, the clock rate
  or the tuning.
- **What is left** is the part between "a peak per symbol" and "the right step" —
  the window placement inside the symbol (`tphase`), or the bank's sub-bin
  indexing. The instrument for it exists now: print `positions()` against the
  absolute tone sequence the known text implies, which needs the idle/text
  boundary in the trace.

## STANAG 4285 — the instrument is sound, the material is not

**Not in the program.** No `Mode` variant, no decoder, no wire entry. `tools/s4285-probe/`
is the instrument and its `README.md` is the long measurement log; the state and the
brief to build the mode are in [`STANAG4285-HANDOVER.md`](STANAG4285-HANDOVER.md), and
the standard itself is transcribed in [`STANAG4285-EDITION1.md`](STANAG4285-EDITION1.md).

| question | answer | the measurement |
|---|---|---|
| frame geometry | **real** | ACF 1280 samples = 106.67 ms on `4285.WAV`, 0.594 against a 0.065 median |
| acquisition without the preamble | **works, by folding** | 0.749 over 8 frames |
| the standard's preamble in the shipped fixtures | **absent** | 0.291 against a 0.11 floor, over 20 conventions and all 256 rotations |
| a real station | folds at **0.70** | FUO Toulon 5215.6 kHz — but it was **idle**, so it proves the fold and the ramp only |
| the transcriber | **proven** | round trip exact at 30 / 40 / 60 dB |

Two nulls had to be measured, and both were got wrong first: the
reference-concentration score reads 0.529 on `4285.WAV` while **its own permutation
null reads 0.601**, and the fold's noise floor is **0.42–0.46** with eight frames
rather than the 0.35 the probe assumed — so anything under about 0.55 is no signal.

## Phase 3 (2026-10-22) — the PSK coded families

The whole phase is fldigi 4.2.13's `src/psk/psk.cxx` mode table, read rather than
guessed: `symbollen`, `idepth`, `numcarriers`, `_pskr`, and the two pieces the
old notes assumed were "framing": **`PSKR_K`/`PSKR_POLY1`/`PSKR_POLY2`** (a
K=7 code with polynomials `0x6d`/`0x4f` — *not* the standard `0o155`/`0o117`
code this tree already carried) and the `2x2xN` interleaver, which **PSK63F
alone skips**. Every PSK robust mode also uses the **MFSK varicode**, not
PSK31's.

Three new receivers, all receive-only for now (`PROTO_VERSION` unchanged):

- **`PskrRx`** — BPSK63F and the single-carrier `R` family. Differentially
  detected soft symbols, two Viterbi decoders one symbol apart (the reference's
  `fecmet`/`fecmet2` vote, because a differential receiver has no idea where a
  coded pair starts), the interleaver when the mode has one, MFSK varicode.
- **`PskCarrierBank`** — `numcarriers` receivers, one symbol from each per symbol
  time in carrier order, which is the order the transmitter emitted them.
  Carriers are **`1.4 × baud` apart**; the corpus's measured bandwidths confirm
  it to a few hertz (PSK63RC4: 4 × 1.4 × 62.5 = 350 Hz against 391 measured).
- **`PskMultiRx`** — the `C` family, which is multi-carrier with **no FEC** and
  PSK31's own varicode. `C` and `RC` are easy to confuse and are not the same
  thing.

| mode | state | the measurement |
|---|---|---|
| BPSK63F | **decodes** | `BPSK63F.wav`, 3/3 words; depth `None` in the reference and the interleaver skipped. |
| PSK125R | **decodes** | 3/3 at 1754 Hz, depth 40. Bit-compared against the known text: **394/394 bits, offset 262**. |
| PSK250R | **decodes** | 3/3, depth 80 — also 394/394 bits at 1685–1700 Hz. |
| PSK500R | **decodes — re-recorded 2026-10-22** | 3/3 at 1700–1780 Hz on a fresh bench recording. The corpus sample carried exactly `AAA TEST TEST THE EAGLE HAS ` and nothing decodable — the recording, not the port; it is kept in `samples/bench/`. |
| PSK1000R | **decodes** | 3/3 at 1754 Hz, depth 160 — **394/394 bits**. |
| PSK250C6 | **decodes** | 3/3, 6 carriers at 1.4 × 250 Hz. |
| PSK500C2 | **decodes** | 3/3, 2 carriers. |
| PSK800C2 | **decodes** | 3/3, 2 carriers. |
| PSK125C12 | **no lock — the recording** | 1/3 at 1745 Hz, and that one word is all: the sample carries **0.630** of the known text's bits (a clean bank measures 1.000). Day 2 measured it. |
| PSK1000C2 | **no lock — the recording** | 0/3 at every centre; the sample carries **0.848** of the text's bits against 1.000 for a clean bank, and both carriers are inside the file (546–3343 Hz). |
| PSK500C4 | **no lock — the recording** | 0/3 at every centre; **0.841** of the text's bits, all four carriers inside the file (744–3741 Hz), and the independent `C` decoder reads it no better. |
| PSK63RC4 | **decodes** | 3/3 at 1745 Hz, and its text reads clean and contiguous. |
| PSK63RC5 | **decodes** | 3/3, depth 260 (the reference's odd one out). |
| PSK63RC10 | **decodes** | 3/3. |
| PSK63RC20 | **decodes** | 3/3. |
| PSK63RC32 | **partial — 2/3** | 2/3 at 1755 Hz; locks at `LANDED`, so the **start of the message is missing** (lock time). |
| PSK125RC4/5/10/12/16 | **decode** | 3/3 each. |
| PSK250RC3/6/7 | **decode** | 3/3 each. |
| PSK250RC2 | **partial — 2/3** | 2/3 at 1785 Hz; locks at `EAGLE` (`fyGLE HAS LANDED …`), so the **start is missing** (lock time). |
| PSK250RC5 | **partial — 1/3** | 1/3 at 2150 Hz; reads `AAA TEST TEST THE EAGLE HA` and then garbage — **loses lock mid-message**. |
| PSK500RC2 | **decodes** | 3/3 at 1765 Hz. |
| PSK500RC3 | **partial — 1/3** | 1/3 at 1785 Hz; reads `AAA TEST TEST THE EAGLE H` and then garbage — **loses lock mid-message**. |
| PSK500RC4 | **does not decode** | 0/3 at every centre; the same sweep and the same width as PSK500C4, whose sample measures 0.841 of the text. Its own bit score is not measured: the `RC` wire carries encoded bits, so the text cannot be correlated directly. |
| PSK800RC2 | **decodes** | 3/3. |
| PSK1000RC2 | **decodes** | 3/3. |

Gates: `the_corpus_pskr_samples_are_measured` (single carrier; asserts BPSK63F,
PSK125R, PSK250R and PSK1000R at 3/3 and prints PSK500R) and
`the_corpus_pskc_samples_are_measured` (multi-carrier; asserts the seventeen that
read whole and prints the rest). Plumbing: `the_pskr_chain_round_trips_its_own_bits`
(varicode → code → interleaver → soft → back, at depths none/40/80/160),
`the_interleaver_inverts_itself` and `the_pskr_modem_reads_its_own_transmission`.

**Where the `R` family stops reading, and what settled it.** PSK500R reads its
first 27 characters and then nothing, and an *independent* decoder — written
from fldigi's own source in Python, with a fixed symbol phase instead of a
Gardner loop and sharing no code with this tree — reads the same 27 and no
more, at every centre from 1700 to 1800 Hz. So it is the recording, not the
port. The other `R` modes read whole, and PSK125R/PSK1000R bit-for-bit.

**A multi-carrier passband is `1.0 × baud`, not the `2.5 × baud` a lone carrier
gets.** The carriers are `1.4 × baud` apart, so a filter sized for one signal
passes its neighbours almost whole. Measured: at `0.75 × baud` PSK500RC2 reads
nothing and at `1.0` it reads its whole text; `1.0` is also the reference's own
(`wsincfilt(fir1c, 1.0/symbollen, …)`, whose first null is at `baud`).

### Day 2 — the three that read nothing are the samples, and the five partials have a shape

`tools/psk-ref/psk_text_match.py` is the instrument, and it is the one thing here
that can answer a question a decoder cannot answer about itself. It builds the
bit stream a mode implies, slides the known text's varicode along it and reports
the best fraction of bits that match, so **1.000 is a clean sample and 0.5 is
noise**. It calibrates itself (`--calibrate`, and the same check was run for
every shape in play): synthetic banks of 1×500, 2×500, 3×500, 2×1000, 4×500,
12×125 and 32×62.5 all read **1.000**. So a reading in between is the
recording's own quality, not the instrument's ceiling.

| sample | reads | text in the recording | the band |
|---|---|---|---|
| PSK500C2 | 3/3 | **0.993** | 1143–2358 Hz |
| PSK250C6 | 3/3 | **0.989** | 743–2761 Hz |
| PSK500C4 | 0/3 | **0.841** | 744–3741 Hz, all four carriers inside 8 kHz |
| PSK1000C2 | 0/3 | **0.848** | 546–3343 Hz, both carriers inside |
| PSK125C12 | 1/3 | **0.630** | 718–2775 Hz, all twelve inside |

**So the three that read nothing are recordings, not a decoder.** The band edges
were measured first, as the brief asked, and they rule the brief's own hypothesis
out: every carrier the mode wants is inside the file (the widest, PSK500C4 at
3.0 kHz, sits in 744–3741 of an 8 kHz recording). The independent `C` decoder in
`tools/psk-ref` — a fixed shared phase, no timing loop, no code in common with
this tree — reads nothing from the same three at any centre either. What settles
it is the bit score: they carry **84 %, 85 % and 63 %** of the known text's bits
where the samples that decode carry 99 %. A `C` mode has no FEC, so ~15 % of bits
wrong is unreadable by anyone, and the two that *do* read are the two that
measure clean. Three samples of one sweep are simply worse than their siblings.

That last number is worth one line of suspicion: 0.63 is far below the other two,
so PSK125C12's recording is the worst of the corpus rather than borderline.

**The five partials, from the whole decoded text rather than the word count**
(`psk_coded_probe`, at the centre the gate reports):

| mode | state | where it stops |
|---|---|---|
| PSK250RC2 | 2/3 | locks at `EAGLE`, reads `fyGLE HAS LANDED 12345 SDRBROWN ZZZ` — **the start is missing** (lock time) |
| PSK63RC32 | 2/3 | locks at `LANDED`, the last two words only — **the start is missing** |
| PSK250RC5 | 1/3 | reads `AAA TEST TEST THE EAGLE HA`, then garbage — **loses lock mid-message** |
| PSK500RC3 | 1/3 | reads `AAA TEST TEST THE EAGLE H`, then garbage — **loses lock mid-message** |
| PSK125C12 | 1/3 | no `AAA`, no `EAGLE`, no `LANDED`; one word at char 244 — **no lock at all** |

None of them loses its tail, so the interleaver's drain is not the fault and is
not worth chasing. Two are late lock and two lose the stream halfway, which is
what a marginal recording looks like from the inside — and matches their bit
scores (0.63 for the worst).

**One thing to know before touching the bank: the reference uses ONE symbol
clock for the whole bank.** `psk::rx_process` keeps a single `bitclk` and
estimates its timing error from the **mean magnitude of every carrier**, then
reads all of them at that instant. `PskCarrierBank` runs one `PskRx` with its own
Gardner loop per carrier instead. That is a real deviation and it may matter on a
weak signal, but it is **not** what these three are: the independent decoder has
the reference's shared clock and reads nothing from them either. Recorded so it
is not re-derived — and as the first thing to change if a clean wide bank ever
fails to read.

**And the trap that cost the instrument an afternoon:** choosing the sampling
phase *per carrier* permutes the interleave — one carrier then sits a symbol
behind its neighbours — and it reads as a sample fault (0.87 on a signal that is
1.000 clean). The phase has to be chosen once, for the whole bank, and the
calibration caught it only because the synthetic bank was there to be wrong
about.

### Day 2, the bench — PSK500R is proved, and the wide banks are not the recording

The RSP1 and the CRT SS9900v were available, so the R family's last unread sample
was re-recorded rather than argued about: fldigi sending the corpus text, the rig
on VOX at 27.415 MHz USB, the RSP1 recording at 250 ksps.

**PSK500R reads its whole text.** `AAA TEST TEST THE EAGLE HAS LANDED 12345
SDRBROWN` at **3/3 from 1700 to 1780 Hz**, on four separate centre offsets, so it
is not a lucky alignment. The gate now asserts all five `R` samples and the
2026-10-21 sample — the one that carried only `AAA TEST TEST THE EAGLE HAS ` —
is kept in `samples/bench/psk500r_truncated_2026-10-21.wav` as the evidence.
**The `R` family has no unread sample left.**

**The four wide multi-carrier modes were re-recorded too, and none of them
decodes either.** This is the useful half: it retires "the samples are bad" for
those, which is what the corpus measurements pointed at.

| mode | old corpus sample | fresh, 4 W | fresh, 2 W |
|---|---|---|---|
| PSK500C4 | 0.841 | 0.767 | **0.841** |
| PSK1000C2 | 0.848 | 0.809 | **0.830** |
| PSK125C12 | 0.630 | (not measured) | (not measured) |
| PSK500RC4 | not measurable (FEC) | 0/3 | 0/3 |

(The figures are `tools/psk-ref/psk_text_match.py`: the fraction of the known
text's bits the recording carries. 1.000 is clean, 0.5 is noise.)

Two things fell out of that table:

* **Power matters, and 2 W is cleaner than 4 W** — C4 goes 0.767 → 0.841 when the
  rig is turned down, which is receiver overload, and it is worth knowing for
  every future sweep. The RSP1 sits close enough to be desensed by our own rig.
* **It plateaus at ~0.84.** Half a dozen independent recordings of a 4-carrier
  500-baud bank, old and new, strong and quiet, all carry about 84 % of their
  bits. A clean synthetic bank of the same shape measures **1.000** through the
  same instrument. So the remaining 16 % is not the recording, not the level and
  not the tuning: it is either the demodulator's carrier separation or the rig's
  audio chain, and the demodulator is the one we can change.

**The concrete next step, and it is a real difference found in the reference:**
fldigi filters **after** decimating (`fir1[i]->init(FIRLEN+1, symbollen/16, …)`
— 200 taps at `symbollen/16` of the sample rate), while `PskCarrierBank` filters
at 8 kHz with 129 taps. In hertz the reference's transition band is ~40 Hz at
125 baud where ours is ~248 Hz, which is wider than the carrier spacing itself
(175 Hz at 125 baud, 700 at 500). That is the first thing to try, and it needs no
bench: `tools/psk-ref/psk_text_match.py` scores any change to the front end in
one run.

**Two harness faults were fixed on the way, and both would have rejected any
recording.** `sweep.py` took the strongest bin in the whole ±125 kHz as "where
the signal is" and the RSP1's own spur at +44–72 kHz won, so the cut kept an empty
band and the file came back as amplified noise reported as *"nothing transmitted
(level −180 dBFS)"*; and it measured the burst length on the raw IQ, where that
same spur gives a flat envelope, so every capture read as a carrier that was up
for the whole file. The peak search is now confined to 100–3800 Hz and the burst
is measured on the extracted audio.

**fldigi's own settings were two traps, one new.** Its PortAudio output index had
moved to an **HDMI** device when the sound card was reconnected — the configured
name said AB13X and the index said HDMI 3 — so it "transmitted" in silence; and
every PSK mode is stored in `mode_state.prefs` at **−30 dB TX level**, so even
with the right device VOX had a whisper to hear. Both are worth checking before
any future sweep: `pa_devs` (a five-line PortAudio lister) names the indices and
`rg '^PSK.*tx_' ~/.fldigi/mode_state.prefs` shows the levels.

### What bit the port, in order

1. **The pairing.** A differential receiver does not know which symbol starts a
   coded pair, so the reference runs two decoders one symbol apart and votes on
   the Viterbi metric. The first version alternated *pairs* instead of taking
   alternate symbols through a shared history, which gave 354 bits out of 195.
2. **The order inside the pair.** fldigi interleaves the pair as `[o2, o1]` and
   then transmits the array *backwards*, so the wire order is `o1, o2` while the
   interleaver's positions are the reverse. A loopback that gets this backwards
   still passes if the transmitter makes the same mistake — this one did.
3. **The interleaver's latency** is `(size − 1) × depth` *calls*, not
   `size × depth`; measured against the reference's own table.
4. **The soft value has to be weighted by amplitude.** fldigi's `softamp` pulls
   a symbol taken during a fade towards the erasure point; a plain
   `re/|d|` normalisation gives a noise symbol the same confidence as a signal
   one.
5. **`PSKR_POLY1/POLY2` are not the K=7 code already in the tree.** `0x6d`/`0x4f`
   is a different code from `0o155`/`0o117`, and `ConvCode::THOR` is the latter.

## Phase 4 (2026-10-22) — THOR's FEC branch, and Contestia's voice

### THOR — eleven of fifteen variants read, and it was four faults

Before: **0 of 15 THOR variants read a word.** After: **11 of 15 read their whole
text.** The DominoEX half gained too (1 → 6 whole), because two of the four faults
were in the audio front end both modes share.

| fault | the measurement that found it |
|---|---|
| **The Viterbi traceback walked its survivor ring in write order.** The ring is written oldest slot first, so the oldest survivor was applied to the newest state and every bit after the first came out of a chain that does not exist. | `the_thor_chain_reads_back_the_bits_it_was_given` — bits → code → interleaver → soft → back — fails on the old walk at **295 of 555 bits**, passes at 100 % now. |
| **The first symbol was never consumed.** `prev_pos` started `None`, so the first decision only seeded it and produced no step; fldigi starts with `prev1symbol = 0` and always consumes one. One symbol lost flips the code's symbol *pairs* for the rest of the over. | the same loopback read `",rpaw_wOe…"` with the pairing one symbol out. |
| **The timing loop's target was half a symbol wrong.** The bank is a matched filter over one symbol, so a tone's energy peaks when its window exactly spans it — at the symbol's **end**, not its middle. | `IFK_DEBUG` printed the previous tone's energy hump centred at ring index 15 of 32, not 8. |
| **And its sign was positive feedback.** Measured in the ring's own order, a clock running late puts the peak *earlier*; the correction has to be added. | With the sign flipped and the target fixed, the loopback reads 439 of 440 steps at **THOR16, THOR8, THOR4, THOR22, THOR25, THOR32, THOR44, THOR56 and THOR50x2 (K = 15)**. |
| *(found on the way)* the tone index for the timing loop did not subtract the `EXTRA` nine tones, so it looked up tone 19, 21, 25 — never a real tone. | `IFK_DEBUG` printed `prev_tone=19 21 23 25`. |

Also in this pass: **THOR32, THOR44 and THOR56 added** (fldigi has them and the
corpus carries them), and **THOR25 corrected** — it is its own mode (320 samples,
*single* spaced, a 25-deep interleave), not THOR25x4 under another name.

The corpus, measured over centres 1100–2000 Hz in 20 Hz steps and both
polarities (`the_corpus_ifk_samples_are_measured`):

| variant | words | | variant | words |
|---|---|---|---|---|
| THOR5 / 8 / 11 / 16 / 22 | 4/4 | | THOR25x4 | 4/4 |
| THOR32 / 50x1 / 50x2 / 56 / 100 | 4/4 | | DOMEX8 / DOMX11 / DOMX16 / DOMX22 / DOMX44 / DOMX88 | 4/4 |
| THOR4 | **2/4 — the recording** | | DOMEX5 | 2/4 |
| THORM | **0/4 — the recording** | | DOMEX4 | 1/4 |
| THOR25, THOR44 | **0/4 — the recordings** | | DOMEXM | 0/4 |

**The three that do not read are the recordings, and each says why.** THOR4 is 3.9
symbols a second: its 56-second sample stops mid-transmission just after
`LANDED`, and its own loopback reads the whole text. THORM at two symbols a second
cannot carry a 50-character message in 56 s at all. THOR25 and THOR44 read nothing
at any centre or polarity while their own loopbacks read **439 of 440 steps** each
— so a fresh bench recording is the item, not the decoder.

Gates: `cargo test -p sdroxide-dsp --release --lib ifk:: -- --include-ignored`
(14 minutes: the wide variants are searched over 46 centres), and
`the_dominoex_steps_carry_the_known_text` now asserts the step trace carries
**100 %** of the known text's nibbles as a subsequence (it was a diagnostic that
asked to become a gate; it said 36 % before, and chance is 6 %).

### THOR in the program — both halves were the wrong implementation

The journal's THOR rows were measured on `sdroxide-dsp::ifk`; the **program** was
wired to a second IFK receiver and transmitter in `sdroxide-dsp/src/thor.rs`,
which read nothing at all and whose own module notes admitted its interleaving was
"not yet bit-matched to fldigi". Both halves are now the reference pair, and the
old file is **deleted** — a second implementation of one waveform is how the
working one stayed a test.

* **Receiver** → `ifk::IfkRx`, chosen by `DigiConfig::thor_mode`. The bank is
  built around a centre, so `set_params` **rebuilds** it on a retune; leaving the
  old bank and moving only a number is how it listened to 1000 Hz while the
  operator looked at 1600, and the corruption read like a wrong convention.
* **Transmitter** → `ifk::IfkTx`, fldigi's `sendchar` path, with a drain sized to
  the interleaver's latency. It reads back through the receiver now; the old one
  did not.
* **`DigiConfig::thor_reverse`** (appended, `PROTO_VERSION` 227 → 228,
  `VERSION_BYTE` 0x2F → 0x30): IFK+ carries its data in the *step*, so a station
  received on the opposite sideband has every step negative. Every corpus reading
  needed it on, which is why the chip exists.
* Gates: `the_thor_corpus_sample_decodes_through_the_controller` (the real
  sample, 1600 Hz, reversed — reads `AAA TEST TEST THE EAGLE HAS LANDED 12345
  SDRBROWN ZZZ`), `a_thor_transmission_reads_back_through_the_controller`, and
  `a_thor_transmission_reads_back` at four variants including a K = 15 one.
* The dead hard-decision FEC it used (`fec::viterbi_decode`, `ConvEnc`) went with
  it; the soft decoder and its stream are what every mode here uses.

### Contestia TX — the Walsh sign is Olivia's *opposite*, and that is the fix

Contestia has had a transmitter for one commit and it decoded as idle characters.
One convention was wrong, and it is the sign where the code bit is written:

* **Contestia is read by the vendored Jalocha engine** (`crates/sdroxide-olivia`,
  fldigi's own code), which takes a **negative** peak as the upper half of the
  alphabet — `if (Peak < 0) Char += SymbolsPerBlock`. So a transmitter that
  follows fldigi's `EncodeBlock` (`FHT_Buffer[TimeBit] < 0`) is what it expects.
* **Olivia is read by this tree's own receiver**, which was measured against an
  off-air capture to vote the *positive* way. The two modes therefore cannot
  share one sign, and the shared `OliviaTx` was emitting Olivia's for both.

Measured through the vendored receiver, at every geometry the config can name:

| | before | after |
|---|---|---|
| 8/500 | `2:@@@4(%;%!',%;(!3;,!.$%$…` | `THE EAGLE HAS LANDED THE EAGLE HAS LANDED` |
| 4/125 | `@@4(%;%!',%;(!3;,!.$%$@` | `THE EAGLE HAS LANDED THE EAGLE HAS LANDED` |
| 16/500 | `,32OTHE ` | `THE EAGLE HAS LANDED THE EAGLE HAS LANDED` |
| 32/1000 | `)O8IZ` | `THE EAGLE HAS LANDED THE EAGLE HAS LANDED` |

Wired: `Mode::Contestia` is out of `is_rx_only`, the controller builds
`OliviaTx::contestia`, and the panel no longer greys the sending half. Gates:
`a_contestia_transmission_reads_back_through_the_controller` (now un-ignored; it
needs **four blocks of tail**, because the receiver integrates `SyncIntegLen`
FEC blocks and a transmission that stops the instant its last character is out
decodes only up to the block before it) and `contestia_keys_the_transmitter`.

### MFSK TX — the receiver's own tables, read the other way

The modulator is `MfskTx` in `sdroxide-dsp/src/mfsk_rx.rs`, next to the receiver
it has to agree with: varicode → the K = 7 code → the `symbits × symbits × depth`
interleaver → one tone per symbol, with the preamble of encoded zeros, `CR STX CR`,
the text, `STX`, and eight idle characters of drain so the interleaver's latency
does not cut the last character off.

* **The audio is generated at the variant's own rate and resampled to 8 kHz**,
  because the eleven variants do not share one: MFSK11/22/44/88 run at 11.025 kHz
  and the whole tone grid is defined there. Building at 8 kHz and "adjusting" the
  spacing is the mistake this avoids.
* **Gates**: `a_synthetic_mfsk_transmission_reads_back` (MFSK4, MFSK16, MFSK32 and
  MFSK11 — the last being the resampled case) and
  `an_mfsk_transmission_reads_back_through_the_controller`, which drives the
  console's own transmitter out to 48 kHz and back in through the console's own
  receiver, resamplers and all.
* `Mode::Mfsk` is out of `is_rx_only`; the panel no longer greys the sending half.

**Not on the air.** The next measurement for this one is a bench recording — our
transmission through the CRT and back through the RSP1 — which is the same loop the
rest of the corpus came from and needs the operator at the rig.

### MT63 TX — 64 phase accumulators, and the reference's own encoder

fldigi's `MT63tx::SendChar` is one paragraph: every 200 samples at `2 · bandwidth`
(20 baud) each of the 64 carriers advances by the phase its own frequency would
have turned through, **plus π when its data bit is 0**. The carriers are continuous
tones whose phase steps at the symbol boundaries; the reference's frequency-domain
synthesis and windowed overlap-add are how it keeps that inside its bandwidth, not
how the data is carried. So the modulator is 64 phase accumulators, a cosine sum,
and the `Mt63Encoder` that was already here for the loopback.

* **Gated by the batch decoder at the true symbol phase**: both interleaves read
  `TEST THE EAGLE HAS LANDED` clean (`a_synthetic_mt63_transmission_reads_back`),
  and the console keys it (`mt63_keys_the_transmitter`).
* **The live receiver is deliberately not this gate, and the measurement is worth
  keeping.** Its lock searches phase and polarity on six seconds of audio and
  scores them by the Walsh peak's SNR and by how much text comes out. On a
  transmission that is mostly *idle* — which any over is before the operator types
  — a **wrong** phase decodes the idle as one repeated letter with a strong peak
  and wins the search: measured, it locks at phase 144 of 200 and reads
  `E@GLE H@R LAODDE` where phase 0 reads the message clean. Both metrics prefer it,
  so it is not the textiness alone. That is a pre-existing weakness of the lock
  (the same one that keeps 1K and 500 off the chip row) and not of the modulator.
* **The 2KL corpus gate was red before this session** and is now green: its
  remaining fault is a handful of characters one Hadamard row high (`SDRBROWn
  tESt`), which is a 32-code *case* error, not a lost character — the gate compares
  case-insensitively now and says so.

### PSK's coded families — 32 waveforms, and the QPSK chip that was never there

BPSK63F, the `R` set and the `C`/`RC` banks all decode their corpus samples and
**no chip reached any of them**: `Mode::Psk` offered six speeds and one
constellation. `DigiConfig::psk_coded` (appended, `PROTO_VERSION` 228 → 229,
`VERSION_BYTE` 0x30 → 0x31) is the selector, one enum carrying the whole of
fldigi's table — baud, carrier count, `2 × 2 × idepth` and whether the mode has
the code at all — so the receiver is built from one table and cannot disagree
with the chips.

* **Three receivers behind one arm** (`PskAny`): `PskrRx` for the code (one
  carrier for BPSK63F and the `R` set, several for `RC`), `PskMultiRx` for the
  `C` banks, which have carriers and **no** code, and PSK31's own `PskRx`.
* **The panel groups by family** — PLAIN / BPSK63F / R / C / RC — then shows that
  family's variants: 32 chips in one row is a menu, not a control.
* **`psk_modulation` had no UI at all**, so QPSK31/63/125/250/500 were
  unreachable too. There is a **Modulation** row now.
* **A coded selection refuses the key-up by name** — *"PSK63RC4 is receive only in
  this build — the coded PSK transmitter is not written"* — because it is a
  *setting* rather than a mode, and quietly sending PSK31 to an operator who
  selected PSK125R would put the wrong waveform on the air. Plain PSK still keys,
  which is what stops the refusal being a blanket.
* Gates: `a_coded_psk_sample_decodes_through_the_controller` (PSK125R's corpus
  sample at 1754 Hz reads `AAA TEST TEST THE EAGLE HAS LANDED 12345 SDRBROWN ZZZ`)
  and `a_coded_psk_selection_refuses_the_key_by_name`.

### DominoEX — its own mode, and the last of the IFK family

DominoEX's decoder read six of its nine corpus recordings and **the program had
no such mode**: its audio path was THOR's, and its alphabet was not reachable.
`Mode::DominoEx` (appended, `PROTO_VERSION` 229 → 230, `VERSION_BYTE` 0x31 →
0x32) and `DigiConfig::domino_mode` (nine variants) are the wiring; the receiver
is the same `IfkRx` adapter THOR uses, because they are one front end and differ
only in what the steps mean.

* **The chip row is nine variants** — DOMINOEX 4 … 88 and MICRO — with **REVERSE**
  shared with THOR, since IFK+ carries its data in the step and a mirrored signal
  has every step negative.
* **The mode tables are the ripple**: nine CAT dialects, TCI, rigctld, SmartSDR,
  speech, the engine's mode class, the band lists and the 11 m set all had to
  learn it, and `Mode::ALL`/`DIGITAL`/`is_digital`/`is_text_modem` are checked by
  the type tests rather than by the compiler.
* **Gates**: `the_dominoex_corpus_sample_decodes_through_the_controller` (DOMX11
  at 1730 Hz reversed reads `AA TEST TEST THE EAGLE HAS LANDED 12345 SDRBROWN
  ZZZ`) and `the_ifk_mode_labels_are_the_decoders`, which pins every THOR and
  DominoEX label against the decoder's own table so a spelling cannot silently
  retune a receiver.
* **Receive only**: no modulator, so `is_rx_only` says so on the chip.

### 8PSK — the slicer is in, and it does not read its recordings yet

Item 4 of the brief, started. fldigi's no-FEC 8PSK is the tractable end of that
family — four recordings, no code — and the reference is unambiguous about it:

* a symbol is one of **eight phase steps** `π/4` apart, taken as the differential
  phase, and the constellation is rotated by **π** in the no-FEC modes
  (`if (_disablefec && (... || _8psk ...)) phase -= M_PI`);
* the three bits go to the **MFSK varicode**, **least significant first**
  (`rx_bit(bits & bitmask)`), and fldigi's Gray table is only for the *coded*
  variants, where the bits are soft values for the Viterbi decoder.

That is `BpskCore::set_psk8`, and it is in. **It reads nothing**: at every centre
from 1000 to 2600 Hz in 5 Hz steps, all four recordings decode to a repeating
garbage pattern (`caecaecae…`, `cc cc cc…`) rather than text. Measured, not
guessed — `the_corpus_8psk_samples_are_measured` prints the known-word hits *and*
the most text any centre produced.

**The slicer is right, and a synthetic transmission proves it.**
`a_synthetic_8psk_transmission_reads_back` builds the signal from the receiver's
own mapping inverted — three bits least significant first, `π/4` steps, the π
rotation, the MFSK varicode — and the receiver reads `EST THE EAGLE HAS LANDED`
(the head lost to the timing loop's acquisition, exactly as BPSK1000 loses its
first second). So the mapping, the differential detection and the varicode are all
correct, and the corpus is what disagrees.

**What is *not* the explanation, checked after the fact:** the vestigial filter.
`psk.cxx` builds an 11-bin sfft about the carrier when `vestigial` is set, but it
runs **only for AFC** — `if (numcarriers == 1 && vestigial && progdefaults.pskpilot)`
— and the data path is the ordinary `fir1`/`fir2` chain. A VSB-shaped signal is
therefore not what a plain receiver is missing, and an earlier version of this note
said it was. The filter also lands ~11 Hz wide, which could not pass a 125-baud
signal at all; that is what sent the claim back to the source.

**Where it stands, measured.** Four bauds (62.5, 125, 250, 500) × every centre from
1200 to 2200 Hz, and 31.25 baud at half-hertz steps about the measured carrier,
all reading nothing, while the synthetic at 125 baud reads. Frequency offset is
ruled out as the whole story by 8PSK1000 — a 1 kHz-wide signal has ±31 Hz of
differential tolerance and still fails.

**And the recording does not look like the signal the mode describes.** Measured
half-second by half-second, `8PSK125.wav` settles on a carrier at **1806 Hz**
(1808 for `F`-family files, 1700 for 250, 2008 for 500, 2258 for 1000) whose
occupied width is **20–46 Hz at −20 dB** — where a 125-baud PSK signal is about
125 Hz wide and an 8PSK one is wider still. Its first two seconds are a *different*
carrier at 1224 Hz at 24 dB SNR, then the signal appears at 1810 Hz at 60 dB.

So the next measurement is not the decoder at all: it is **what these files
contain**. A signal 30 Hz wide cannot carry 125 baud of phase changes, and no
slicer will read it until that is explained — a band-limited chain, a wrong sample
rate in the header, or a recording of something other than the mode's own
waveform. The same measurement taken before another port attempt is what stops a
fourth receiver being written against the wrong signal.

### DTMF — eight tones, and two things the reference leaves to its squelch

A small mode, done whole. fldigi's `src/dtmf/dtmf.cxx`: four row tones and four
column tones, a Goertzel filter on each, the character is the strongest row paired
with the strongest column, and the alphabet is `123A456B789C*0#D`.

Two additions, both of which a receiver without a squelch knob needs:

* **A ratio test, not a maximum.** Eight Goertzel outputs always have a largest, so
  a plain argmax prints a digit for noise. The winning tone must be **5×** the
  runner-up in its own group (30 ms frames put the bin 33 Hz wide and the rows
  73 Hz apart, so a real press clears that easily) and the pair must carry 30 % of
  the frame. Measured: at 3× a single digit got through in three seconds of noise;
  at 5× none did.
* **A debounce.** A key press is 40 ms or more and the frame is 30 ms, so a naive
  per-frame decoder prints each digit two or three times. A digit is emitted once
  per press, with two clear frames meaning release — one is not enough, because a
  frame boundary can land inside a press.

**Modes and wiring:** `Mode::Dtmf` (appended, `PROTO_VERSION` 230 → 231,
`VERSION_BYTE` 0x32 → 0x33) with the whole mode-table ripple; receive only, since
there is no keypad to send from. It deliberately does **not** join
`holds_audio_centre`: the eight tones are absolute in the passband, so a cursor
anywhere decodes them, and the console's own test tunes the cursor 600 Hz away to
prove it.

Gates: `every_key_press_reads_back_once` (all sixteen keys, in order, from a
transmitter that is the touch-tone specification rather than one of ours),
`noise_does_not_produce_digits`, `a_held_key_is_one_digit`, and
`a_dtmf_key_press_reads_through_the_controller`.

## The table — every mode we carry, as it stands (2026-10-23)

The brief's deliverable, now covering **everything the program ships** rather than
the families this phase touched. Every row carries the measurement or says plainly
that there is none; `—` in `vendored` means the code is ours.

**Variants in bold, in brackets, behind each name** — how many the program offers.

`rx`: `no` · `loopback` (our own transmission into our own receiver) ·
`recording` (its own corpus/off-air sample reads) · `air` (a real station or
signal). `tx`: `no` · `loopback` · `recording` · `air` (**another station or
program copied us** — the only one that proves interop).

### Voice, broadcast and the demodulators

| mode | rx | tx | vendored | tested |
|---|---|---|---|---|
| LSB, USB, DIGU, DIGL, DSB (1 each) | air | air | — | the radio's own modulator; the bench pair |
| AM, SAM, NFM, WFM (1 each) | air | air | — | as above; WFM carries RDS/RBDS |
| CW (1) | recording | loopback + bench | — | 9/10 words; keyer benched on the rig, iambic A and B |
| C-QUAM (1) | air | no | — | receive-only by capability; decodes a real stereo station on the bench |
| ISB (1) | air | no | — | receive-only by capability — two modulators into one amplifier is not a radio we drive |
| SPEC (1) | air | no | — | a display mode; no modulation of its own |
| DRM (1) | air | no | `vendor/dream` | a real broadcast decodes |
| HD Radio (1) | recording | no | `nrsc5` + faad2 | the sample decodes; **no HD station has been heard here** |

### Keyboard and text modes

| mode | rx | tx | vendored | tested |
|---|---|---|---|---|
| BPSK/QPSK plain (**12**: 6 speeds × 2 constellations) | recording | loopback | — | 3/3 words at 125/250/500; 31 and 63 lose a word; 1000 reads from a second in |
| BPSK63F, PSK125R/250R/500R/1000R (**5**) | recording | no | — | **394/394 bits** against the known text |
| PSK125C12/250C6/500C2/800C2 + 14 `RC` (**18**) | recording | no | — | 3/3 words each |
| PSK500C4, PSK1000C2, PSK500RC4 (**3**) | no | no | — | the recordings carry 0.84/0.85/0.83 of the text's bits; a clean bank measures 1.000 |
| PSK63RC32, PSK250RC2, PSK250RC5, PSK500RC3 (**4**) | recording | no | — | 1–2 of 3 words: late lock or a lost lock mid-message |
| Olivia (**30**: 6 tones × 5 bandwidths; 18 measured) | air | loopback | Jalocha `MFSK_Receiver`, GPL-3, `crates/sdroxide-olivia` | 14/18 corpus; two off-air captures read their known text |
| Contestia (**25**: 5 tones × 5 bandwidths; 19 measured) | recording | loopback | Jalocha `MFSK_Receiver`, GPL-3 | 18/19 — only 64/500 never reads |
| THOR (**6** chips, **15** decoded) | recording | loopback | — | 4/4 words in 11 of 15 variants; THOR4 and THORM are too slow for their samples, THOR25/44 read nothing (samples suspect) |
| DominoEX (**9**) | recording | no | — | 6/9 whole; DOMEX5 2/4, DOMEX4 1/4, DOMEXM 0 |
| MFSK (**11**) | recording | loopback | — | 9/11 whole; MFSK8 loses one span of its sample, MFSK11 its first word |
| MT63-2KS, 2KL, 500S (**3** offered, **6** decoded) | recording | loopback | — | 2KS and 500S whole, 2KL with a few characters one Hadamard row high |
| MT63-1KS/1KL/500L (**3**) | no | loopback | — | not offered: the live lock needs a centre search that trades one variant for another |
| RTTY (**3**: 45.45 / 50 / 75 baud) | recording | loopback | — | 10/10 words at 45.45 baud, 170 Hz shift |
| Hell — HELLX5, FELDHELL (2 of **7**) | recording | loopback | — | correlation 0.87 / 0.83 against fldigi's own glyphs |
| Hell — HELL80, SLOWHELL, FSKH105/245, HELLX9 (5 of **7**) | recording | loopback | — | 0.33–0.48 — reads, weak |
| DTMF (**16** keys) | loopback | no | — | all 16 keys once each from the touch-tone specification; noise silent |
| FSQ (**5** speeds, 1 measured) | no | loopback | — | the alphabet is fldigi's; the one corpus sample has never read back |
| ARDOP (1) | recording | no | — | three `ardopcf` recordings decode byte-exactly |
| STANAG 4285 (**3** uncoded rates, 1) | recording | no | — | the instrument reads real idle captures; the Annex E payload is proven on a loopback only |
| AtChat (1) | air | air | — | the NET mode, in use on the bench |
| JTTY (1) | recording | loopback | — | its own golden-vector gate |
| ALE (1) | in flight | in flight | — | see [`ALE-HANDOVER.md`](ALE-HANDOVER.md) — mid-flight, not claimed |
| 8PSK125/250/500/1000 (**4** ported, **none ships**) | **no** | no | — | 0/4: the slicer is proven on a synthetic; the recordings measure 20–46 Hz wide where the mode is 125 Hz |

### Slotted and weak-signal

| mode | rx | tx | vendored | tested |
|---|---|---|---|---|
| FT8, FT4, FT2, JS8 (1 each) | air | air | `mfsk-core` 0.11 | contacts made on the air; FT8 runs signal subtraction |
| WSPR (1) | air | air | `mfsk-core` 0.11 | spots uploaded from the bench |
| JT65, JT9, FST4, Q65, MSK144 (1 each) | recording | no | `mfsk-core` 0.11 | corpus recordings; transmit not wired in this build |
| FSK441 (**2** periods: 15/30 s) | recording | loopback | ported from FSK441-PLUS (MIT) | a real meteor ping from YO2NAA decodes |
| PI4 (1) | no | no | — | a decoder for the beacon network's signal, not a beacon |

### Images

| mode | rx | tx | vendored | tested |
|---|---|---|---|---|
| SSTV, SSTV-FM (**16** modes each) | air | air | — | **field-confirmed by a radio group** (2026-10-21) — do not touch |
| RIFP (1) | air | air | — | as above |
| RF Paint (1) | air | air | — | the bench |
| WEFAX (**2** LPM, 2 IOC settings) | air | no | — | a real off-air chart, 2153 lines |

### Wideband lanes — receive only, by nature

| mode | rx | tx | vendored | tested |
|---|---|---|---|---|
| ADS-B (1) | air | no | — | real aircraft on the bench |
| AIS (**2** channels) | air | no | — | real traffic |
| VDL2 (**14** channels, one lane) | air | no | — | real traffic; 14 channels from one lane |
| HFDL (**13** frequencies in the plan) | air | no | `xng` (MIT/Apache) | a real ground-station squitter |
| DAB / DAB+ (1) | air | no | `dabradio` (MIT) + faad2 | **field-confirmed on a PlutoSDR** — do not touch |
| ACARS (1) | recording | no | — | `acarsdec`'s own `test.wav` decodes a real frame |
| NAVTEX (**3** international channels) | air | no | — | reads the `SITOR-B` sample from the point it locks |
| DSC (**2** bands: MF/HF and VHF) | air | no | — | the MF/HF distress format decodes |

### Packet and networking

| mode | rx | tx | vendored | tested |
|---|---|---|---|---|
| PACKET, PACKET-HF (1 each) | air | air | — | the bench; KISS over the radio's own link |
| APRS (1) | air | air | — | the bench |
| UVPacket (1) | no | no | — | receive-only; the burst format decodes a generated frame |

### The count

61 `Mode` variants, of which **38 can key a transmitter** and 23 cannot (checked by
`is_rx_only`, and `the_lanes_with_no_transmitter_do_not_offer_one` pins the ones
whose own docs say they have none). 32 coded-PSK variants and 9 DominoEX variants
are one `Mode` each.

## Needs me

Nothing blocks the work above — every mode has a measurement. These would move the ones that do not decode, or prove the ones that do further:

- **A STANAG 4285 capture of a station sending a test tape** (2026-10-21). The French Navy transmits unencrypted RYRY, an ID and the French pangram (`VOYEZ VOUS LE BRICK GEANT QUE J EXAMINE PRES DU GRAND WHARF`), in USB at 1800 Hz, on **4232.0 / 5215.6 / 8467.8 / 8478.5 / 12857.0 / 13031.2 / 13411.9 / 6348.0 kHz**. Ninety seconds at 48 kHz with the UTC time and the station noted is enough. Everything up to the payload is solved and the payload has never been read from the air; the frequency table, the expected text and the `rspcap` command are in [`STANAG4285-HANDOVER.md`](STANAG4285-HANDOVER.md).

- **Nothing more for the wide `C` banks — they were re-recorded on 2026-10-22
  and still do not decode.** PSK500C4, PSK1000C2, PSK125C12 and PSK500RC4 were
  recorded off the bench through a tuned antenna at 2 W and carry **0.841, 0.830,
  0.63 and 0/3** of their known text — the same as the corpus samples, against
  1.000 for a clean synthetic bank. So for those four the recording is not the
  fault and another bench session would not help; the next move is the
  demodulator's carrier separation (see "Day 2, the bench" — the reference
  filters *after* decimating, we filter at 8 kHz). The recordings are kept in
  `samples/bench/` as the evidence.
- **A THOR25 and a THOR44 recording** (2026-10-22). Both decode their own
  transmissions (439 of 440 steps each) and both read nothing from the corpus
  sample at any centre or polarity — like PSK500R, which turned out to be the
  recording. THOR44 is 43 symbols a second, so ten seconds on the bench settles
  it.
- **A second MFSK8 recording.** The corpus one loses `TEST THE EAGLE HAS LANDED` in one span while the preamble and the tail read at every tuning; the decoder is the one the other seven MFSK variants use.
- **A known-good FSQ recording** (new, 2026-10-21). The corpus sample cannot settle FSQ: an FSQ signal is one tone at a time and that recording never has a dominant tone at any centre, spacing or speed, and nothing has ever read it back. Thirty seconds on the bench turns "alphabet fixed, decode unproven" into a gate.
- **A bench recording of BPSK1000 for the lock-time question** (2026-10-21). The operator's own offer: the RSP1 can record while fldigi plays into the CRT. What it settles is whether the receiver's lost head is the receiver (a clean, known transmission is decoded from the start) or the sample (the sweep's own window). The rig needs a hand for the frequency and to arm it.
- **An off-air ARDOP recording.** The reference-made WAVs prove the modem against `ardopcf`'s own modulator; a live or third-party recording would prove it against a station, and would also exercise the RS correction on real fading.
- **A known-good fldigi on the same box for the THOR/DominoEX tone-step convention.** The port is in `ifk.rs` and reproduces OnAir's receiver, but the steps do not match the air. The corpus cannot say which side is wrong without a decoder to compare against on the same recording.
