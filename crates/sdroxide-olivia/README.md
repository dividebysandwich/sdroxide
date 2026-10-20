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
