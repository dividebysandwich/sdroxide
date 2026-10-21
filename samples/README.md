# Our own transmissions

Every file here was **sent by us and received by us** — fldigi keying the CRT
SS9900v through VOX, the SDRplay RSP1 with a stub on its antenna listening at
27.415 MHz, `tools/rsp1-capture` recording. Nothing here is somebody else's
material, which is why it can live in the repository at all.

That matters more than it sounds. Two days of mode work were spent on recordings
whose text nobody knew; these have a known answer, so a decoder that fails on them
has failed, and a decoder that reads them has worked.

## `fldigi/`

One recording per mode fldigi can send, from `tools/bench/sweep.py`. Each is 8 kHz
mono, extracted from the RF capture and band-limited to the mode.

The text is, verbatim, spaces included:

```
 AAA TEST TEST THE EAGLE HAS LANDED 12345 SDRBROWN ZZZ
```

and for a mode too slow to carry it in a reasonable time:

```
 AAA SDRBROWN TEST
```

`manifest.jsonl` is the database: one JSON object per mode with the mode fldigi
reported, tones and bandwidth for our decoders, the measured audio centre and
level, the text the sample actually carried (`text`, and `shortened` when it was
the short one), how long the carrier was up, and the verdict.

**`unverified` is the honest state and the common one.** Nothing has decoded those
samples yet, so nothing confirms the text went out. `verify.jsonl` holds what the
decoders we have did manage — the word count, not just a pass.

A mode that fails there is not proved impossible; it means the decoder we have
today does not read it. Several of fldigi's PSK variants carry framing and FEC we
have never written.

## `ardop/`

ARDOP is not in `fldigi/` because fldigi cannot send it. These four recordings
were made by the reference implementation, `ardopcf` (MIT), run headless with
`--writetxwav`, `NOSOUND` and a `TXFRAME` host command; the data is our own
known string, so the files carry no third-party material.

| file | frame | text |
|---|---|---|
| `4psk-1car_known-text.wav` | 0x40 4PSK, 1 carrier | `TEST TEST THE EAGLE HAS LANDED 12345 SDRBROWN ZZZ` |
| `16qam-8car_known-text.wav` | 0x74 16QAM, 8 carriers | same |
| `4fsk-100bd_known-text.wav` | 0x4A 4-FSK, 100 baud | same |
| `conreq-200m.wav` | a leader/tune artefact, **not** a frame | — |

All are 12 kHz mono. The `conreq` file is kept as the record of a dead end: the
`TXFRAME ConReq200M` path wrote 1.77 s of leader and a sweep, with no header, so
it cannot gate control-frame detection.

## `bench/`

The hand-made fixtures from the session that started this, each with its text
recorded in `our-captures/README.md` before they moved here:

| file | mode | text |
|---|---|---|
| `hell_feld_cb_27415.wav` | Feld Hell | `TEST TEST 12345` |
| `rtty45_cb_27415.wav` | RTTY 45.45/170 | `TEST TEST RTTY 12345` ×3 |
| `psk31_cb_27415.wav` | PSK31 | `THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG 0123456789` |
| `olivia_16-500_cb_27415.wav` | Olivia 16/500 | `TEST TEST THE EAGLE HAS LANDED 12345` |
| `olivia_32-1k_cb_knowntext.wav` | Olivia 32/1k | `THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG 0123456789` |
| `mfsk16_cb_eagle.wav` | MFSK16 | `TEST TEST THE EAGLE HAS LANDED 1234567890` |

## Using them

The decoders' tests take a path from the environment, and default to here:

```sh
cargo test -p sdroxide-olivia --release -- --ignored --nocapture
cargo test -p sdroxide-dsp --release --lib -- --ignored --nocapture
```

`tools/bench/verify.py` walks the whole corpus and grades it.

## The one bit of state that is not in here

`fldigi`'s own configuration: the callsign it nags about, its capture device, and
its output device **by name rather than index** (the index moves between restarts
and will point at the graphics card's HDMI). `tools/bench/README.md` has the four
traps written out.
