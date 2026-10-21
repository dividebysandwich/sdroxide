# s4285watch — sit on a frequency until the tape starts

The French Navy test tapes are intermittent, the stations idle for hours, and a
session cannot watch 6348 kHz all night. This does it for you: it holds the radio
open, scores every frequency once per dwell, and **writes a recording only when a
STANAG 4285 frame is actually there**.

```sh
cargo build --release
./target/release/s4285watch 6348000 62500 /tmp/opencode/s4285 45 3.0 6
./target/release/s4285watch 6348000,5215600,5155000,4232000,12857e3 62500 /tmp/opencode/s4285
./target/release/s4285watch --selftest
```

| argument | meaning |
|---|---|
| `hz[,hz…]` | one frequency, or a comma-separated list to sweep |
| `rate` | sample rate — **62500**, not 48000: an RSP1 refuses 48 kHz and silently keeps its own (2 Msps), which overflows the read buffer |
| `outdir` | where the WAVs go, created if missing |
| `gain` | dB, default 40 |
| `threshold` | ACF ratio against the buffer's own null, default 3.0 |
| `dwell` | seconds per frequency, default 6 |

## How it decides

One statistic, and it is the invariant: **the 106.67 ms frame period** — 256
symbols at 2400 baud. The sync sequence is sent every frame, so a real
transmission repeats itself every 106.67 ms whatever its data rate, whether it is
coded or uncoded, and whether it is carrying text or sitting idle. Each decision
compares that period's autocorrelation with the **median autocorrelation of
nearby lags, measured on the same buffer** — a bare correlation means nothing on
a long recording, which is the trap `tools/s4285-probe/README.md` records.

`--selftest` runs the statistic on synthetic noise and on the same noise with a
frame periodicity imposed, with no radio at all:

```
noise   acf 0.001  null 0.001  0.4x
framed  acf 0.801  null 0.011  74.2x
selftest ok
```

## What it writes

On a trigger: the dwell it already holds plus thirty seconds, as a 16-bit mono
WAV named `<kHz>-<UTC>.wav`, then it moves to the next frequency. The audio is
the real part of the baseband, decimated to about 12 kHz — what a USB
demodulator produces, with the 4285 subcarrier at 1800 Hz exactly where the
receiver wants it. **Nothing is written when there is no signal**, so a quiet
night costs a few megabytes rather than tens of gigabytes.

Read a file back with the probe:

```sh
cd ../s4285-probe && ./s4285_verdict.py /tmp/opencode/s4285/6348.000kHz-20261022-031500Z.wav
```

That prints the frame period, the best lag anywhere in 60–160 ms, and an 8PSK
carrier test, each against its own null — so an idle station (frame but no data)
is told apart from a tape.

## Frequencies worth sweeping

From the SWL group's logs via the operator, plus the ones other listeners report:

| station | place | kHz |
|---|---|---|
| FUE | Brest (Issoudun) | 6348.0 |
| FUO | Toulon | 5215.6 |
| FUF | Fort-de-France | 4232.0 / 8478.5 / 13031.2 |
| FUV | Djibouti | 8467.8 |
| 6WW | Dakar | 12857.0 / 13411.9 |
| FUG | La Régine | 12873.9 |
| — | a Turkish station seen idling | 5155.0 |

sigidwiki puts 4285 anywhere between **1.89 and 22.7 MHz** and points at the UDXF
logs for month-by-month usage. Those logs are the place to look when a night
comes back empty — not the decoder.

## Caveats

* It **holds the RSP1**, so the program cannot use radio 0 meanwhile.
* An **idle station triggers** — it sends the sync continuously. That is the
  right behaviour for "is it on the air", and the verdict tool separates an idle
  carrier from a tape.
* The RSP1's rate is read back, so a refused rate is visible rather than silent;
  the WAV's rate comes from what the radio actually did.
* Two minutes of a night is not proved by the selftest: nobody has yet caught a
  tape with it. That is what the next empty night is for.
