# Mode backfill — the journal

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
