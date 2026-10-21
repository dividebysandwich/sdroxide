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
