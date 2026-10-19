# STANAG 4285 acquisition probe

Standalone Python probes for reading a STANAG 4285 recording. **Not part of the
build** — nothing here is compiled or tested by `cargo`, and it is here so the
measurements survive instead of living in a scratch directory.

The waveform itself is transcribed in `MODE-DECODERS-HANDOVER.md` §6b, from the
**declassified** standard (Edition 1, 16 February 1989, downgraded to NATO
UNCLASSIFIED by MAS/59-EL/4285 of 9 February 1993). Read that section first: it
has the frame, the transcoding tables, and both generators in closed form.

## What was measured, and what it says

Fixture: `sta4285.wav` from SkySweep Pro's demo recordings (11025 Hz, 8-bit,
14 s, 1800 Hz centre). A real off-air capture, and it fades hard — the envelope
varies 45× between the quiet and loud tenths.

**The frame is confirmed.** The complex autocorrelation at the frame period —
256 symbols / 2400 baud = **106.67 ms** — is **0.389** against a median of
0.054, a 7× peak, with the second harmonic at 213.4 ms just as strong. So the
recording is 4285 and the frame geometry from the spec is right.

**A coherent sync correlation does not lock**, and this is the finding worth
keeping. Swept: 75 shift-register conventions (every two-tap pair × four
readings of the `11010` initial value × both shift directions), centre from 1400
to 2200 Hz, baud from 2340 to 2460, and a carrier-offset search of ±120 Hz in
10 Hz steps. The best was **0.40**, which is the sequence's own bias rather than
a match — it does not move with any of those parameters.

**Differential detection locks.** Correlating the *differentially detected*
stream against the differentially encoded PRN gives **0.546** with the spec's own
polynomial (taps 2 and 5), against 0.40 coherent. That is the answer, and it is
the right one physically: an 80-symbol coherent integration is 33 ms long, so a
30 Hz carrier offset — well inside the ±75 Hz the standard allows — rotates a
whole turn inside it. The sync is differentially encoded precisely so that a
receiver can acquire without knowing the offset, which is also why the standard's
Annex F is about Doppler and synchronisation rather than about a correlator.

## Scripts

| file | what it does |
|---|---|
| `acf.py` | finds the frame period by autocorrelation — the first thing to run on a new capture |
| `check.py` | envelope constancy and symbol-rate lines, to say whether the fixture is a PSK signal at all |
| `probe.py` | the first full attempt: mix to baseband, resample, correlate the sync |
| `brute_prn.py` | every plausible shift-register convention, scored |
| `sync_freq.py` | the carrier-offset search |
| `diff.py` | the one that works — differential detection against the differentially encoded PRN |

`s4285_sync.py <file.wav> [centre]` is the one to run first: frame-period
autocorrelation and the differential sync correlation against the same file, so
"is this 4285" and "can we acquire it" are answered together. `readwav.py` is a
small reader that parses the RIFF chunks by hand — `wave` refuses several of
these demo files, whose headers it does not accept.

## What is left

1. **Acquire properly**: differential detection, then the offset comes out of the
   phase of the correlation, which is the value the rest of the receiver needs.
2. **Frame, descramble, transcode.** The scrambler is `x⁹ + x⁴ + 1`, three bits
   per symbol, `n = 4·x₂ + 2·x₁ + x₀`, and the 48 reference symbols per frame are
   all symbol 0 — which means **they are a free self-check**: descramble
   correctly and they must collapse onto one phase.
3. **A cleaner fixture.** This capture is faded enough that the averaged frame
   reaches only 0.275 coherence, which is consistent with half the frame
   repeating but too weak to read the pattern back. Sigmira ships example
   recordings; a live capture on a naval frequency would be better still.
4. Only then a mode in the tree, receive-only, uncoded rates first (1200 / 2400 /
   3600 need no FEC at all).

## Fixtures

Two, both off-air and both now in `~/Downloads/sounds`:

* `skysweep/sta4285.wav` — SkySweep Pro's demo recording, 14 s at 11025 Hz.
  Frame confirmed (ACF 7x), sync 0.546 differentially. Fades hard: the envelope
  varies 45x and the averaged frame only reaches 0.275 coherence.
* `4285.WAV` — MultiPSK's own sample, **64.7 s at 12000 Hz**, and a much
  healthier signal (envelope varies 7.8x). Its differential sync correlation is
  **0.610**, better than the other one, though its frame-period ACF is weak
  (0.031 against a 0.013 median). Worth a look before anything else: either the
  capture is of a variant with a different frame, or the frame is there and the
  short-lag ACF is simply diluted over 64 s of data.

The SkySweep recordings live in `~/Downloads/sounds/skysweep/`, beside the
MultiPSK collection rather than in it: the two sets collide (`hfdl.wav` exists in
both) and keeping them apart keeps their provenance obvious.

**What SkySweep does not give us.** Its decoder is closed — the manual's licence
notice is "All rights reserved" and the company is gone, so there is nobody to
ask — and the SDK sources its installer manifest lists (`demo.c`, `siggen.c`,
`control.cpp`) are not actually installed. Its manual does document the *shape*
of the need: a generic bit analyser with bit synchronisation, an interleaver and
the Baudot/ITA2 character set, which is exactly the decode path the Monitoring
Times article describes. It says nothing about Annex E's parameters.

## The round trip, and what it proves

`s4285_sim.py` builds a frame from the transcription and decodes it. This is the
instrument that settles arguments, because the two recordings disagree with each
other and neither of them locks — and a fixture that will not lock cannot tell
you whether the transcription is wrong or the recording is.

**It decodes exactly at 30, 40 and 60 dB audio SNR.** So the frame layout, the
176-symbol scrambler from `x^9 + x^4 + 1`, the sync generator `x^5 + x^2 + 1`
seeded 11010, the eight phases at `n*pi/4`, and the reference symbols being all
symbol 0 are all as the standard describes. The transcription is sound.

Two things about it cost a round each, and both are general:

* **A mixer without a filter corrupts the decision.** Mixing to baseband leaves
  the mirror at `-2*centre`, which lands inside the resampled band and cost about
  half the symbols. The filter has to come straight after the mixer.
* **The reference symbols cannot resolve the sampling instant.** Sixteen symbols
  in a row all carrying the same phase look equally good at any point across
  them, so a frame search scores best when it samples a boundary. The timing has
  to be chosen from the *data*, by how tightly it lands on constellation points.

And one thing that is not a bug: **the frame index is ambiguous by design.** Every
frame carries the same reference symbols, so a search finds *a* boundary, not
frame zero. Comparing against a fixed frame position fails for that reason alone;
what has to match is the data.

### Where it degrades, and what that costs the real receiver

| clock error | frame found | data |
|---|---|---|
| 0 ppm | R = 1.000 | exact |
| 300 ppm | R = 1.000 | decision error 0.18 rad, no exact frame |
| 1000 ppm | R = 1.000 | decision error 0.18 rad, no exact frame |
| 5000 ppm | R = 0.997 | no exact frame |

The frame lock is robust at every rate tried. The data now samples a fractional
instant — interpolated, not one of eight whole samples — and a per-frame timing
search picks it, which is what a receiver needs. What still degrades is the
*resampling*: the probe's `demod` uses linear interpolation, which is not a good
enough resampler for a band-limited signal at a few hundred ppm, and the decision
error roughly doubles there. That is a probe limitation with a known fix (a
polyphase or cubic resampler), not a property of the mode.
