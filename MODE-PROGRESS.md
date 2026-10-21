# Mode backfill — the journal

> **The next phase is [`MODE-PHASE-2-PROMPT.md`](MODE-PHASE-2-PROMPT.md)** — receive
> parity, then transmit parity, then wiring the working modes in. This file is the
> record of the first pass.

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
| MT63-2KL | **decodes** | `MT63-2KL.wav`, 1740: `AAA SDRBROWN TEST`, 2/2. Long interleave included. |
| MT63-1KS | **does not decode yet** | `MT63-1KS.wav`, 1740: the text is visible but **every character is one Hadamard row high** — `EAGLE` arrives as `eagle`, `SDRBROWN` as `sdrbrown` with spaces lost, i.e. code +32 throughout. That is a carrier-phase signature (a sign flip across half the bank), not an interleaver fault: the short interleave is proved by the loopback below. |
| MT63-1KL | **does not decode yet** | Same as 1KS, plus the long interleave; fragments only. |
| MT63-500S | **does not decode yet** | `MT63-500S.wav`, 1740: `AAA … EAGLE?HASLAN:ED 1B34zSDwBROWN` — most words present but corrupted; the 500 Hz mode's 4096/1600 window/hop has the same phase problem. |
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

### MT63 — 2 of 6 decode
See the MT63 table above: 2KS and 2KL decode; 1KS/1KL/500S/500L arrive one Hadamard row high.

### PSK family
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

### Hell — none decode from this corpus
| FELDHELL | **does not decode** | column correlation with the rendered phrase 0.08–0.15 (a clean copy scores 0.87; noise 0.03–0.06), or no columns at all. The bench capture `samples/bench/hell_feld_cb_27415.wav` still decodes, so the mode works and these sweep files do not. |
| HELL80 | **does not decode** | column correlation with the rendered phrase 0.08–0.15 (a clean copy scores 0.87; noise 0.03–0.06), or no columns at all. The bench capture `samples/bench/hell_feld_cb_27415.wav` still decodes, so the mode works and these sweep files do not. |
| HELLX5 | **does not decode** | column correlation with the rendered phrase 0.08–0.15 (a clean copy scores 0.87; noise 0.03–0.06), or no columns at all. The bench capture `samples/bench/hell_feld_cb_27415.wav` still decodes, so the mode works and these sweep files do not. |
| HELLX9 | **does not decode** | column correlation with the rendered phrase 0.08–0.15 (a clean copy scores 0.87; noise 0.03–0.06), or no columns at all. The bench capture `samples/bench/hell_feld_cb_27415.wav` still decodes, so the mode works and these sweep files do not. |
| SLOWHELL | **does not decode** | column correlation with the rendered phrase 0.08–0.15 (a clean copy scores 0.87; noise 0.03–0.06), or no columns at all. The bench capture `samples/bench/hell_feld_cb_27415.wav` still decodes, so the mode works and these sweep files do not. |
| FSKH105 | **does not decode** | column correlation with the rendered phrase 0.08–0.15 (a clean copy scores 0.87; noise 0.03–0.06), or no columns at all. The bench capture `samples/bench/hell_feld_cb_27415.wav` still decodes, so the mode works and these sweep files do not. |
| FSKH245 | **does not decode** | column correlation with the rendered phrase 0.08–0.15 (a clean copy scores 0.87; noise 0.03–0.06), or no columns at all. The bench capture `samples/bench/hell_feld_cb_27415.wav` still decodes, so the mode works and these sweep files do not. |

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

## Needs me

Nothing blocks the work above — every mode has a measurement. These would move the ones that do not decode, or prove the ones that do further:

- **A second MFSK8 recording.** The corpus one loses `TEST THE EAGLE HAS LANDED` in one span while the preamble and the tail read at every tuning; the decoder is the one the other seven MFSK variants use.
- **An off-air ARDOP recording.** The reference-made WAVs prove the modem against `ardopcf`'s own modulator; a live or third-party recording would prove it against a station, and would also exercise the RS correction on real fading.
- **A known-good fldigi on the same box for the THOR/DominoEX tone-step convention.** The port is in `ifk.rs` and reproduces OnAir's receiver, but the steps do not match the air. The corpus cannot say which side is wrong without a decoder to compare against on the same recording.
