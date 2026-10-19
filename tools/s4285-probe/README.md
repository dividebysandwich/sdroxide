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

They take the recording's path in the source; `readwav.py` is a small helper that
lives beside them in `/tmp/opencode` and is not committed (it is four lines of
`wave` + `numpy`).

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
