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

## Acquisition by folding, and what it found (2026-10-19)

`s4285_fold.py` folds the recording onto its own frame period and scores how much
survives, searching frame phase and carrier offset together. It needs no sync
pattern, so a lock cannot be an artefact of a guessed sequence — which is what
made the earlier correlations unreadable.

**It locks.** On MultiPSK's `4285.WAV` the same 128 symbols of every frame add
coherently: score **0.749** over 8 frames (floor 0.35), **0.60** over 32,
settling at about **-44 Hz** of carrier offset. The frame period is confirmed
independently: the autocorrelation peaks at **1280 samples = 106.67 ms exactly**,
which is 256 symbols at 2400 baud, at **0.594 against a 0.065 median**.

**But the payload is not Edition 1's.** In the folded frame — which has the SNR
of a frame averaged over eight — the 80-symbol sync does not appear: the best
correlation against the standard's PRN, over twenty shift-register conventions
and all 256 rotations, is **0.291 against a 0.11 noise floor**, and fifty-one
hundred trials of noise would reach that. The 48 all-zero reference symbols do
not collapse either. So the repeating content of this recording is *not* the
preamble the declassified document describes.

Two readings, and they are both cheap to test with the right material:

1. **It is a variant.** Sigidwiki notes variants of 4285 — the Thales Système
   3000 ones in particular — whose identifying feature is "a special preamble at
   the beginning of the transmission". A French Navy recording is a good candidate
   for exactly that.
2. **The recording is a test transmission** whose data repeats, which would fold
   well regardless of what the sync is.

Either way the conclusion for the mode is the same: **the receiver needs a
recording of a station that is known to be sending Edition 1**, or the variant's
preamble. Everything up to the payload is in place — frame period, carrier
offset, folding acquisition, and a proven transcriber.

### What the mode needs to be proven, and where to find it (2026-10-19)

**Intel from the SWL group, via the operator.** The French Navy runs
**unencrypted test tapes** — RYRY loops and a station ID — and those are the
material this work has been missing: known content, on the air, regularly.

Signals are ~2.75 kHz wide, a buzzsaw in USB, tuned at **1800 Hz** with the full
bandwidth across it. Reported:

| station | place | kHz | heard (UTC) |
|---|---|---|---|
| **FUF** | Fort-de-France, Martinique | **13031.2** | 1846 |
| | | **8478.5** | 1203 |
| | | **4232.0** | 0146 |
| **FUV** | Djibouti | **8467.8** | — |
| **FUO** | Toulon | **5215.6** | — |
| **6WW** | Dakar | **12857.0**, **13411.9** | — |
| **FUG** | La Régine | **12873.9** | — |
| **FUE** | Brest | **6348.0** | — |

A capture of any one of these, with its **RYRY and ID**, would settle the
question the two shipped fixtures could not: whether the payload of a real
Edition 1 transmission matches the declassified document. Everything up to the
payload already works — frame period (1280 samples = 106.67 ms, ACF 0.594),
folding acquisition (0.749), and a transcriber proven by round trip.

Capture with `tools/rsp1-capture` at ≥ 12 kHz, USB, full 2.75 kHz. Note the UTC
time and the station — the ID is the known part.

## FUO Toulon, 5215.6 kHz (2026-10-19, ~20:10 UTC) — and a correction

Captured with `tools/rsp1-capture` from the bench RSP1: 60 s at 250 kHz,
centred on the dial, USB.

**The frame folds at 0.70** (16 frames, floor 0.25) — a real repetition, well
clear of the noise floor. The other frequencies from the SWL list were empty at
that hour: 13 031.2, 8 478.5 and 6 348.0 all came back as Rayleigh noise
(envelope std/mean ≈ 1.0).

**The folded frame carries a phase ramp**, and that is the durable finding. The
fold searches a carrier offset of ±75 Hz per frame, which cannot absorb a
*centre* error, and a ramp across the frame spreads everything in it. Correcting
it — +636 Hz across the frame here — takes the frame from incoherent to coherent.
This is also why the round trip could never have caught it: the transmitter and
receiver share the same centre and therefore the same mistake.

**What this capture does *not* show, and an earlier note here claimed it did.**
It does not confirm the frame layout. Once the ramp is removed, **the whole
256-symbol frame sits in one tight cluster** — 222 of 256 symbols in two adjacent
45° sectors, spread 27° — with `|S|` flat across it. The 48 reference symbols
collapse to R = 0.925, but so does everything else, because the transmission was
**idle**: a constant symbol repeated, not a test tape carrying text. A capture of
idle proves the fold and the ramp and nothing about the layout.

Getting past that needs a capture taken while a station is actually sending
RYRY and an ID — see the frequency table above — and then the same corrected fold
should show 80 symbols of BPSK sync, 48 references and 128 data symbols that
differ from one another.

## The fold's noise floor is about 0.44, and that has to be measured

A sweep of the SWL list on 2026-10-19, 20:10–20:40 UTC, from the bench RSP1:

| kHz | station | fold (8 frames) | verdict |
|---|---|---|---|
| 5215.6 | FUO Toulon | **0.70** | real, but **idle** (one constant symbol) |
| 13 031.2 | FUF Fort-de-France | 0.44 | noise |
| 8 478.5 | FUF | 0.46 | noise |
| 6 348.0 | FUE Brest | — | noise (Rayleigh envelope) |
| 12 857.0 | 6WW Dakar | 0.43 | noise |
| 13 411.9 | 6WW Dakar | 0.42 | noise |
| 8 467.8 | FUV Djibouti | 0.44 | noise |
| 4 232.0 | FUF | 0.52 | marginal |
| 12 873.9 | FUG La Régine | 0.45 | noise |

**A noise-only capture folds at 0.42–0.46 with eight frames**, so the 0.35
"floor" I had been using is optimistic and `s4285_fold`'s own significance
threshold of 0.30 is wrong — it will report noise as a lock. A real signal is
0.70 and up; anything under about 0.55 should be treated as no signal.

The same caution applies to the "varying content" test: a phase spread of ~50°
looks like data and is what noise gives too (53.9° measured on a known-empty
frequency). **Both metrics need a null measured on the same capture**, not a
number carried over.

And at this hour only one of the eight frequencies had anything at all, and that
one was idle.

## Tools

| file | what it does |
|---|---|
| `s4285_sync.py` | frame-period autocorrelation and differential sync correlation |
| `s4285_frame.py` | finds the frame by reference-symbol concentration |
| `s4285_fold.py` | **acquisition by folding** — phase and carrier offset together, no pattern assumed |
| `s4285_decode.py` | frame-by-frame decode once a lock exists |
| `s4285_sim.py` | round trip from the spec; decodes exactly at 30 dB and above |
