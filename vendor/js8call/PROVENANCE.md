# JS8 protocol constants — provenance

Every JS8 constant in `crates/sdroxide-digi/src/js8/` is transcribed from the
JS8Call source, not reconstructed. This file records where each one came from,
so a future reader can re-derive any of them without guessing.

**Upstream:** <https://github.com/js8call/js8call>
**Commit:** `a7ff1be0b389d287fdc56e2ea0d06962aa68127d` (2025-12-05)
**License:** GPL-3.0-or-later — compatible with sdroxide, which is GPL-3.0-or-later
because `sdroxide-digi` links `mfsk-core`. Everything derived from JS8Call lives
in `sdroxide-digi` for the same reason: the permissive crates and the wasm client
must link none of it.

Paths below are relative to a JS8Call checkout.

## Submode parameters

| Constant | Source |
|---|---|
| `NSPS` per submode (1920/1200/600/3840/384) | `commons.h:36-54` (`JS8{A,B,C,E,I}_SYMBOL_SAMPLES`) |
| Slot period (15/10/6/30/4 s) | `commons.h:37-53` (`JS8*_TX_SECONDS`) |
| Start delay (500/200/100/500/100 ms) | `commons.h:38-54` (`JS8*_START_DELAY_MS`), = `ASTART` in `lib/js8/js8*_params.f90` |
| Decode window `NTXDUR` (15/10/6/**28**/4 s) | `lib/js8/js8{a,b,c,e,i}_params.f90` |
| `NDOWNSPS` (32/20/12/32/12) | `lib/js8/js8*_params.f90` |
| Frame geometry `KK=87, ND=58, NS=21, NN=79` | `lib/js8/js8a_params.f90:11-14` |
| Bandwidth = `8 * 12000 / NSPS` | `JS8Submode.cpp` (`m_bandwidth`) |
| Sync minimum 1.5 | `ASYNCMIN`, `lib/js8/js8*_params.f90` |
| Max candidates 300 | `NMAXCAND`, `lib/js8/js8a_params.f90` |
| Which submodes ship enabled | `commons.h:30-34` — `JS8_ENABLE_JS8I 0`, so Ultra is built but disabled |
| Submode bit values (A=0 B=1 C=2 E=4 I=8) | `varicode.h:27-31` (`Varicode::SubModeType`) |

Note the two quirks: **start delay differs per submode**, and **Slow analyses a
28 s window inside a 30 s cycle** — its `NTXDUR` is not its period.

## Waveform

| Constant | Source |
|---|---|
| Costas arrays, ORIGINAL set (Normal) | `lib/js8/genjs8.f90:25-27` — all three blocks identical |
| Costas arrays, MODIFIED set (Fast/Turbo/Slow/Ultra) | `lib/js8/genjs8.f90:29-31` — three distinct arrays |
| Costas block positions (symbols 0/36/72) | `lib/js8/genjs8.f90:67-69` |
| Which set each submode uses | `JS8Submode.cpp` (`Costas::Type::{ORIGINAL,MODIFIED}`), values in `JS8.hpp:29-45` |
| **No Gray coding** | `lib/js8/genjs8.f90:75` writes `itone(k)=indx` directly; FT8's `genft8.f90` writes `graymap(indx)` here. JS8's tone map is the identity. |

The Costas values are corroborated four ways: `genjs8.f90`, `syncjs8d.f90:22-28`,
`js8dec.f90:33-39` and `JS8.hpp:29-45` all agree. `JS8.hpp:18` *claims* Normal
reuses FT8's array; it does not — FT8's is `[3,1,4,0,6,5,2]`. Trust the values.

## Message framing

| Constant | Source |
|---|---|
| 87 info bits = 72 message + 3 frame-type + 12 CRC | `lib/js8/genjs8.f90:54-57` (format `12b6.6,b3.3,b12.12`) |
| Frame-type codes | `varicode.h:50-57` |
| Directed command table | `varicode.cpp:46+` (`directed_cmds`) |
| Reserved callsigns / groups (`<....>` … `@QRO`, 54 entries) | `varicode.cpp:214-280` (`basecalls`) |
| `NUSERGRID` = `nbasegrid + 10` | `varicode.cpp:211` |
| Compound-directed command packing | `varicode.cpp:1166-1205` (`packCmd` / `unpackCmd`), placed above `nusergrid` by `packCompoundMessage` at `varicode.cpp:1425` |
| Compound framing for custom groups and compound calls | `varicode.cpp:2100-2140` (`buildMessageFrames`, cases 1-3): `Compound(from, grid)` then `CompoundDirected(to, cmd)` |
| Which calls need the compound form | `varicode.cpp:1232` (`isValidCompoundCallsign`), `:1292` (`isCompoundCallsign`) |
| Huffman free-text table | `varicode.cpp:158+` (`hufftable`) |
| JSC dictionary (262144 entries + 103 prefixes) | `jsc.h`, `jsc_list.cpp`, `jsc_map.cpp` |

The 6-bit alphabet in `genjs8.f90:35` is a transport encoding for passing 72 bits
across the C++/Fortran boundary as 12 characters, not a text alphabet. Rust packs
the bits directly.

## CRC-12

| Constant | Source |
|---|---|
| Polynomial `0xc06`, `boost::augmented_crc<12, …>` | `lib/crc12.cpp` |
| **`icrc12 = xor(icrc12, 42)`** | `lib/js8/genjs8.f90:48` |

The XOR-42 is JS8-specific and has no counterpart in WSJT-X. Omitting it makes
every correct codeword fail verification — the decoder finds signals, converges
on them, and reports nothing.

## LDPC(174,87)

| Table | Source |
|---|---|
| `Mn` — 3 checks per bit, 0-based | `JS8.cpp:661-698` |
| `Nm` — bits per check with `valid_neighbors`, 0-based | `JS8.cpp:700-742` |
| Generator parity rows (87 × 22 hex chars) | `JS8.cpp:990+` (`Data`) |
| Reference BP decoder | `JS8.cpp:744-830` (`bpdecode174`) |
| Original Fortran tables | `lib/ft8/ldpc_174_87_params.f90` (`g`, `colorder`) |
| Fortran encoder | `lib/ft8/encode174.f90` |
| OSD | `lib/ft8/osd174.f90` |

### Two findings that shaped the Rust

**The C++ generator rows already have `colorder` applied.** `encode174.f90`
computes `itmp = [parity(87), message(87)]` then scatters it through `colorder`.
JS8.cpp reorders the generator rows instead and skips the permutation. Verified
by construction (`tools/gen_js8_ldpc.py`): over 200 random information words,
`f90-row-order + colorder` and `cpp-row-order + no colorder` each produce
codewords satisfying all 87 parity checks, and the other two combinations produce
none. We use the C++ order, so `colorder` is not needed and is not transcribed.

**The codeword is not systematic-prefix.** Message bits occupy `cw[87..174]`;
parity occupies `cw[0..87]`. This is the reverse of `mfsk_core`'s `FecCodec`
convention ("the first K bits of codeword must equal info"), which is why
`Ldpc174_87` documents the deviation rather than silently reordering.

### Why the table transcription is trustworthy

`Mn`/`Nm` and the generator hex rows are *independent* representations of the
same code — one is the parity-check matrix, the other the generator. Asserting
that `H · encode(info) == 0` therefore cross-checks two separately transcribed
tables against each other; it cannot pass if either was mis-transcribed. The
edge sets of `Mn` and `Nm` are also checked against each other (522 edges,
column weight uniformly 3). Both assertions run as unit tests in `js8/ldpc.rs`.

These are necessary but not sufficient — they prove the tables are
self-consistent, not that they are JS8's. The sufficient evidence is below:
frames JS8Call's own decoder reads, and vectors its own encoder produced
(`crates/sdroxide-digi/tests/js8_varicode.rs`).

## Transmit verified against JS8Call's decoder

JS8Call's standalone decoder (`js8 -8 -b <A|B|C|E> -d 3 <file.wav>`, from the
2.2.0 package) was run against WAVs written by
`cargo run -p sdroxide-digi --example js8_wav -- encode …`:

| Speed | Submode | Combinations | Exact |
|---|---|---|---|
| Normal | A | 4 messages × 5 frequencies × 4 frame types | 80/80 |
| Fast | B | 4 × 3 × 2 | 24/24 |
| Turbo | C | 4 × 3 × 2 | 24/24 |
| Slow | E | 4 × 3 × 2 | 24/24 |

"Exact" means JS8Call reported the message text, the frame type and the audio
frequency we transmitted. Every decode came back at `dt 0.0`, so the per-submode
start delays are right too.

**Shaping settles as a non-question.** The same eight frames sent as GFSK and as
CPFSK decoded with *identical* reported SNR at every speed (21/16/10/26 dB for
Normal/Fast/Turbo/Slow). The receiver measures tone energy over a symbol and does
not care how the transitions are shaped, so [`js8::modem`]'s Gaussian default
costs nothing and narrows the spectrum. `synth_cpfsk` stays for A/B checks.

**One tone sequence JS8Call cannot decode**, and it is not ours: the transport
string `-+-+-+-+-+-+` produces a frame whose second data block is 29 consecutive
symbols of almost nothing but tones 6 and 7. Our tone output for it is
byte-identical to JS8Call's own encoder, and the frame fails to decode in
JS8Call's CPFSK shaping as well as ours, so this is a property of that waveform
rather than a defect here. No varicode payload produces such a payload; the case
is noted only so a future reader does not rediscover it as a bug.

**Version note.** Constants above are cited against commit `a7ff1be`; the decoder
used for verification was the packaged 2.2.0 build. That the two interoperate
exactly is evidence the wire format has been stable across that span, which is
what one would expect — JS8's framing has not changed since 2.0.

## Receive verified against JS8Call's decoder

Both decoders were run over identical files. Where they disagree is as
interesting as where they agree, so all of it is recorded.

**Sensitivity is at parity.** Signal amplitude swept down against fixed noise,
three seeds per point:

| Speed | amp 0.030 | 0.022 | 0.016 | 0.012 | 0.008 |
|---|---|---|---|---|---|
| Normal — sdroxide | 3/3 | 3/3 | 3/3 | 2/3 | 0/3 |
| Normal — JS8Call | 3/3 | 3/3 | 3/3 | 2/3 | 0/3 |
| Slow — sdroxide | 3/3 | 3/3 | 3/3 | 3/3 | 0/3 |
| Slow — JS8Call | 3/3 | 3/3 | 3/3 | 3/3 | 0/3 |

Identical thresholds, and at Normal 0.012 both fail on the *same* seed.

**A crowded slot is not a problem either.** Ten equal-amplitude signals spread
from 400 to 2800 Hz in one 15 s slot: 10/10 for both decoders at every noise
level tried. This does not refute the coarse-sync concern documented in
`js8/decode.rs` — that one is about real busy bands with unequal signals and
QSB, which only an off-air recording can settle — but it does rule out the
synthetic case.

**SNR needed fixing.** `mfsk_core::core::llr::compute_snr_db` estimates noise
from a tone four places from the transmitted one; on a strong clean signal that
bin holds the signal's own leakage, so the estimate saturated near +3 dB where
JS8Call reported +20. Publishing that would have meant systematically
15 dB-pessimistic PSK Reporter spots. `js8::decode::snr_db` now reads the floor
off the full-slot spectrum as a median over guard bands either side of the
signal, the way `baselinejs8.f90` does, plus a per-speed offset fitted to the
measurements below. Agreement afterwards is within **3 dB worst case, 1 dB
typical**, across a 40 dB span:

| Speed | offset | worst residual |
|---|---|---|
| Normal | −2.5 dB | 2 dB |
| Fast | −6.5 dB | 2 dB |
| Turbo | −10.0 dB | 3 dB |
| Slow | −0.5 dB | 2 dB |

An intermediate attempt that took the noise from the *downsampled* baseband's
out-of-band bins is worth knowing about because it looked plausible and was
wrong: those bins sit in the anti-alias filter's stopband, so it measured the
filter rather than the band, and compressed a 30 dB span into 8 dB.

## Message layer verified by linking JS8Call's varicode

`tools/gen_js8_varicode_vectors.sh` compiles JS8Call's own `varicode.cpp`
(plus `jsc.cpp` and the dictionary) against Qt6 and runs it, emitting
`crates/sdroxide-digi/tests/js8_varicode_vectors.rs`. The expected values in the
message-layer tests are therefore produced by the reference implementation, not
by a second reading of it — which matters here more than anywhere else in the
mode, because none of this encoding is derivable: it carries per-country
workarounds, a reserved-value block for group names, and a grid encoding that
round-trips through degrees.

Three undefined symbols (`BuildMessageFramesThread`, `DecodedText`) are Qt
thread/signal machinery the packing functions never reach; linking `moc`'s
output for `varicode.h` and `decodedtext.cpp` satisfies them.

`MESSAGES` holds whole messages as `buildMessageFrames` frames them. They cover
custom groups (@MAGNET), slashed callsigns as addressee (W1AW/4) and as sender
(KN4CRD/QRP), and reserved groups added late to the table (@EMCOMM, @AMRRON).
`compound_messages_are_framed_as_js8call_frames_them` checks that our transmit
side produces them byte for byte, and
`messages_js8call_frames_reassemble_with_the_right_sender_and_addressee` that
our receive side reads them back.

**Two findings from adding it.**
- Our copy of `basecalls` stopped at `@NTS`. Every frame JS8Call sent to
  @EMCOMM, @ARES, @AMRRON or @APRSIS unpacked to "no such callsign" and was
  dropped.
- Every compound frame was read as a heartbeat, so a message to a custom group
  arrived as "HB from @MAGNET".

**A finding worth keeping.** `packGrid` computes `int ilat = pair.second + 90`
— it biases the *float* and then truncates. Truncating first and then adding 90,
which is the natural way to write it in Rust, rounds the wrong way for negative
latitudes and shifts **every grid south of the equator by one square**. A
northern-hemisphere test suite would never notice. `pack_grid` does it in
upstream's order, and `southern_hemisphere_grids_are_not_off_by_one` guards it.

## Free text: two codecs, and three ways to get it subtly wrong

Free text takes one of two paths per frame — Huffman per character
(`js8::huff`) or dictionary substitution (`js8::jsc`) — and the transmitter
keeps whichever carried more characters. Both are checked against
`packDataMessage` output in `tests/js8_varicode.rs`. Three details cost frames
that decode but differ from every other station's:

1. **The tie-break favours compression.** `varicode.cpp:1808` tests
   `huffChars > compressedChars`, so an equal result picks the dictionary. The
   natural way round produces a longer frame that still decodes.
2. **The fit test is strict.** `frameBits.length() + bits.length() < frameSize`
   — a codeword that would *exactly* fill 72 bits is rejected. "THE QUICK BROWN
   FOX" fills to precisely 72, so upstream drops the last word and sends 16
   characters where a `<=` test sends 19.
3. **Compression never matches across a space.** `jsc.cpp`'s `compress` splits
   on spaces first and prefix-matches within each word. Matching across spaces
   compresses *better*, which is exactly why it is wrong here.

The dictionary is JS8Call's `jsc_map.cpp`, converted by `tools/gen_js8_jsc.py`
into a deflate-compressed blob (1.0 MB, from 12 MB of upstream C++ across two
files). Only the index-ordered table is stored; the alphabetical order for
word-to-index lookup is rebuilt at load, so the two orderings cannot disagree.

**A generator bug worth recording.** 32 entries are extended Latin-1 characters
written as escaped bytes with an explanatory comment between the string and its
index (`{"\xa1" /* … */, 1, 10704}`). A regex that does not allow for the
comment drops them silently — and because the table is positional, every index
past 10704 then shifts, corrupting roughly 96 % of the dictionary. The generator
now asserts that each entry's stated index equals its position, which turns that
into a build failure instead of garbled text on the air.

## Ordered-statistics decoding

`js8::osd` is a port of `lib/ft8/osd174.f90`, used as the fallback where belief
propagation fails. Measured against JS8Call over identical files (noise 0.25,
six seeds per point, signal amplitude swept):

| Speed | amp | BP only | BP + OSD | JS8Call |
|---|---|---|---|---|
| Normal | 0.013 | 6/6 | 6/6 | 4/6 |
| Normal | 0.011 | 1/6 | **3/6** | 1/6 |
| Slow | 0.009 | 3/6 | **4/6** | 2/6 |
| Slow | 0.008 | 0/6 | **1/6** | 0/6 |

Never worse anywhere, and 200 slots of pure noise across all four speeds still
produce zero decodes with it enabled. Cost on a busy Normal slot is ~20 ms
against 15 000 ms of audio — the sync-quality gate turns most candidates away
before the FEC runs — so it is on by default rather than an option.

**One trap, worth recording.** The first implementation CRC-gated every
candidate and kept the best survivor. An order-2 search generates 3 829
candidates; against a 12-bit CRC the chance that at least one wrong codeword
passes is about 61%, and any impostor closer to the received word than the true
codeword wins. That made OSD *remove* decodes BP had already found — 6/6 became
5/6, 3/6 became 0/6. Upstream selects by soft distance alone and checks the CRC
once, on the winner; so do we, with a hard-error ceiling as a second net.

## Regenerating any of this

None of the generators run during a normal build — their output is committed, so
neither Python, Qt nor a JS8Call checkout is needed to compile sdroxide. Run
them only when the upstream commit above changes:

```sh
python3 tools/gen_js8_ldpc.py ~/Development/js8call
python3 tools/gen_js8_jsc.py  ~/Development/js8call
./tools/gen_js8_varicode_vectors.sh ~/Development/js8call \
    > crates/sdroxide-digi/tests/js8_varicode_vectors/mod.rs
```

The first two refuse to emit anything that fails their internal cross-checks;
the third needs Qt6 development files because it links `varicode.cpp`.

Generated artefacts, all committed:

| File | From |
|---|---|
| `crates/sdroxide-digi/src/js8/ldpc_tables.rs` | `JS8.cpp` Mn/Nm/generator |
| `crates/sdroxide-digi/src/js8/jsc_dict.bin` | `jsc_map.cpp` (1.0 MB deflated) |
| `crates/sdroxide-digi/tests/js8_varicode_vectors/mod.rs` | `varicode.cpp`, linked and run |


