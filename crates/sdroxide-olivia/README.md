# sdroxide-olivia

Olivia, decoded by Pawel Jalocha's `MFSK_Receiver` — the engine behind fldigi and
SDR-J.

## Why this is vendored rather than written

Olivia was reimplemented here three times and all three failed on recordings that
fldigi decoded perfectly: `OliviaRx` in `sdroxide-dsp`, a Python model, and a
careful port of the reference front end (raised-cosine window two symbols long,
two slices per symbol, energy-squared soft bits, sweeping slice parity, block
offset and polarity). fldigi reading our own transmitted capture settled it — the
recording was genuine and the fault was ours.

Rather than guess at the Walsh, interleave and scrambler conventions a fourth
time, the working engine is used directly, the same route this tree already takes
for HFDL (`vendor/xng`) and RADE (`vendor/rade_c`).

## Provenance and licence

The sources in `cxx/` are from **fldigi**, `src/include/jalocha/`:

- `pj_mfsk.h`, `pj_fft.h`, `pj_fht.h`, `pj_cmpx.h`, `pj_struc.h`, `pj_gray.h`,
  `pj_lowpass3.h`, `pj_fifo.h`

written by **Pawel Jalocha, December 2004**, adapted into fldigi by Dave Freese
(W1HKJ) and into SDR-J by Jan van Katwijk. They are **GPL-2.0-or-later**, which
this project (GPL-3.0-or-later) satisfies. `cxx/olivia_shim.{h,cpp}` are ours.

The vendored headers are unmodified apart from nothing at all — they are as
published. `git log` on this directory shows when they arrived; the fetch URLs
are recorded in `PROVENANCE.md`.

## What the wrapper does

Only what fldigi's and SDR-J's own drivers do before starting the engine:
`SampleRate = 8000`, `InputSampleRate` = the soundcard rate, `SyncMargin = 8`,
`SyncIntegLen = 4`, no squelch, and `FirstCarrierMultiplier` derived from the
audio centre. Audio is fed in 512-sample batches, as both drivers do.

## The one setting that bites

`reverse` inverts the tone order. A recording taken on the wrong sideband — or
extracted as USB from a rig that was in LSB — reads as nonsense until it is set.
Both bench captures needed it, and the first RTTY capture did too.

## Testing

```sh
SDROXIDE_OLIVIA_SAMPLE=$HOME/Downloads/sounds/our-captures/olivia_16-500_cb_27415.wav \
SDROXIDE_OLIVIA_TONES=16 SDROXIDE_OLIVIA_BW=500 SDROXIDE_OLIVIA_HZ=1718 \
SDROXIDE_OLIVIA_REVERSE=1 SDROXIDE_OLIVIA_TEXT="EAGLE LANDED" \
  cargo test -p sdroxide-olivia --release -- --ignored --nocapture
```

## The probe

`tools/olivia_probe.cpp` dumps the demodulator's internals for a given capture, so
a Rust port can be **diffed against measured numbers** rather than written from a
reading of `pj_mfsk.h` — which is how three earlier attempts went wrong.

```sh
cd crates/sdroxide-olivia/tools
g++ -O2 -std=c++14 -I ../cxx olivia_probe.cpp -o /tmp/olivia_probe

# a WAV the decoder can read, as raw floats
ffmpeg -i capture.wav -f f32le -acodec pcm_f32le -ac 1 /tmp/capture.f32

# <file> <tones> <bw> <rate> <centre> [rev] [slice] [freq offset] [phase] [symbols]
/tmp/olivia_probe /tmp/capture.f32 16 500 8000 1718 1 0 0 0 200
```

It prints:

```
SYM <n> <slice> <foff> <soft bits...>   what the demodulator hands the decoder
CHR <n> <char codes...>                  what the decoder makes of a block
```

For our 16/500 capture it opens as `SymbolLen 512, SymbolSepar 256, FirstCarrier
126 (1968.8 Hz)`, sixteen carriers 31.25 Hz apart — the geometry every entry in
`MODE-ROADMAP.md` was trying to derive by hand.

**The next half is a matching dump on our side**: the same env var, the same
lines, from `OliviaRx`'s own front end. Then the diff says whether our fault is
in the tone extraction or in the bit mapping, in an afternoon, and a port becomes
a mechanical exercise.
