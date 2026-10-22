# STANAG 4285, Edition 1 — what the standard itself says

**This is a transcription, not a guess.** It exists so that the STANAG 4285 work
stops being pinned to somebody's description of the standard: the transcoding, the
two generators and the acquisition method below are read off the document.

## Provenance, and what was read

* **Document:** STANAG 4285 (**Edition 1**), *Characteristics of 1200/2400/3600 bits
  per second single tone modulators/demodulators for HF radio links*, Military
  Agency for Standardization, promulgated **16 February 1989**. Marked **NATO
  UNCLASSIFIED** on every page (the older "NATO RESTRICTED" stamp is struck
  through — it is a declassified copy).
* **The copy:** the full **68-page scan**, obtained from the Internet Archive's
  snapshot of `jedi-sec.us/downloads/MISC_PDF/4285Eed01a2.pdf` (that URL is now a
  404; the Wayback copies of 2022 and 2023 both serve the file):

  ```
  https://web.archive.org/web/2023id_/https://www.jedi-sec.us/downloads/MISC_PDF/4285Eed01a2.pdf
  ```

  The file is **not** committed here — it is a third-party scan of a third party's
  document, kept beside the tree the way an off-air capture is. `curl` the line
  above and it is 1.4 MB.
* **A second copy** of the front matter plus Annex A pages A-1 and A-2 (8 pages)
  sits at `n2ckh.com/MARS_ALE_FORUM/s4285.PDF`; A-1 and A-2 below are read from
  **that** copy, because the 68-page scan's first annex pages are cropped.
* **The scan has no text layer.** `pdftotext` returns an empty file; every page was
  read as an image (`pdftoppm -r 130 -png`). **Page map, measured:** PDF page 8 =
  A-3, 9 = A-4, 10 = A-5, 64 = E-3-3 (the end of Annex E), 66 = F-1, 68 = F-3 —
  which is the last page of the document.
* **Annexes A and F are transcribed here. Annex E (error correction, interleaving,
  message protocols) is not** — it is present in the scan between the pages above
  and marked "for information only"; the next session that needs the FEC should
  read it there rather than trust a summary.

## Annex A — modulation (A-1)

The modulation is phase shift of a sub-carrier of **1800 Hz**. The modulation speed
is **2400 bauds**, to a minimum accuracy of 1 part in 10⁵, and the clock linked with
the 1800 Hz generation has the same accuracy. The phase shift relative to the
unmodulated reference takes one of eight values:

| symbol | phase |
|---|---|
| 0 | 0 |
| 1 | π/4 |
| 2 | π/2 |
| 3 | 3π/4 |
| 4 | π |
| 5 | 5π/4 |
| 6 | 3π/2 |
| 7 | 7π/4 |

**The complex number e^(jnπ/4) is linked with symbol number n.**

## Annex A — transcoding (A-2)

Transcoding links a symbol to a group of bits. **Oldest bit first.**

**1200 bps — one bit per symbol:**

| bit | symbol |
|---|---|
| 0 | 0 |
| 1 | 4 |

**2400 bps — a dibit per symbol:**

| dibit | symbol |
|---|---|
| 00 | 0 |
| 01 | 2 |
| 10 | 6 |
| 11 | 4 |

**3600 bps — a tribit per symbol:**

| tribit | symbol |
|---|---|
| 000 | 1 |
| 001 | 0 |
| 010 | 2 |
| 011 | 3 |
| 100 | 6 |
| 101 | 7 |
| 110 | 5 |
| 111 | 4 |

## Annex A — power spectral density (A-3, ¶4)

After filtering and transposition to 1800 Hz the density is as Figure A-2 (in
Appendix 3). **The total bandwidth occupied is 3000 Hz.**

## Annex A — frame structure (A-3, ¶5)

* Frames are **recurrent, 106.6 ms** long.
* Bits per frame: **128 at 1200 bps, 256 at 2400 bps, 384 at 3600 bps**.
* **A frame is 256 symbols: 80 synchronisation, 48 reference, 128 data.**
* The **176 reference and data symbols are scrambled** by a scrambling sequence with
  eight phase states, **length 176** (see ¶7).
* The modem **uses the synchronisation sequence** to detect the presence of the
  signal and to correct the frequency shift from Doppler or the transmit/receive
  pilot difference, bit synchronisation, and either equaliser training (recursive
  filtering) or HF channel estimation (maximum likelihood).
* The reference and data symbols form **4 blocks**: the first three are **32 data
  symbols followed by 16 reference symbols**, the last is 32 data symbols. **The
  reference symbols are all symbol number 0.**

## Annex A — synchronisation sequence generator (A-3 ¶6c, A-4 ¶6c–e)

* The synchronisation is **80 symbols, 2PSK, 2400 bauds**, transmitted every
  106.6 ms.
* It is a **pseudo-random sequence of length 31**, repeated within the window: **2
  full periods of 31 plus the first 18 symbols of another**.
* Generator polynomial: **x⁵ + x² + 1**
* Initial value at the start of every frame: **1 1 0 1 0**
* **The first symbol is the least significant bit of that initial value**; the
  remaining 79 are obtained by clocking 79 times.

## Annex A — data block scrambling (A-4 ¶7, A-5 ¶8)

* The scrambling sequence is **176 symbols**, repeated every 106.6 ms, transmitted
  in **eight-phase-state modulation at 2400 bauds**.
* Generator: a pseudo-random code of **length 511**, polynomial **x⁹ + x⁴ + 1**,
  **initialised to 1 at the start of each frame**.
* A symbol comes from the **last three bits of the PN register**, `x0`, `x1`, `x2`:

  ```
  Bk = e^(j·n·π/4),   n = 4·x2 + 2·x1 + x0      (x0, x1, x2 ∈ {0, 1})
  ```

* **Generation advances by shifting the register three positions** per symbol.
* The operation is carried out **on the reference and data symbols only, not on the
  synchronisation sequence**, and it is **modulo 8 addition of the data symbol to the
  scrambling symbol** — i.e. a complex multiplication of the data symbol by the
  scrambling symbol.

## Annex A — what the receiver must tolerate (A-5 ¶9)

* A frequency error of **±75 Hz** between the transmitting and receiving carriers
  (transmitter/receiver error *and* Doppler shift included).
* A **rate of frequency change up to 3.5 Hz** (per second).

## Annex F — detection, acquisition and tracking (F-1, F-3)

Annex F is marked "for information only" and is the standard's own answer to the
question this project's entry in `MODE-ROADMAP.md` calls the acquisition fold.

* **At the start of a reception the synchronisation sequence is used for:** signal
  detection, Doppler acquisition, synchronisation acquisition, channel acquisition.
  During data its periodic repetition allows the reception algorithms to be
  re-initialised.
* The sequence is a **cut-out of a periodically repeated PN sequence**, 2PSK, period
  31 symbols, repeated **80/31** times to fill the 80-symbol window.
* **Detection is a matched filter**, not a search over the data:

  ```
  v(i) = g(i) ∘ xpnref(i)                        (F-1)
  ```

  where `g(i)` is the received signal in the low-pass domain and `∘` is continuous
  correlation (matched filtering). **`xpnref(i)` must contain at least
  2·1500/2400 samples per PN symbol** — the sampling theorem against a low-pass
  bandwidth of 1500 Hz. `v(i)` is the correlator output.
* **Detection is accepted on an error count**: the sync symbols are received and
  their errors counted; **only if the number of errors is below a threshold** is a
  valid synchronisation declared — that is the standard's answer to unwanted
  periodic interference.
* **The reference PN may be a cyclic shift** of the transmitted one: a cyclic shift
  produces a non-cyclic shift of the correlator output `v(i)`, so the position can be
  chosen to minimise disturbance from preceding noise, AGC and the data that
  follows.
* **Acquisition:** synchronisation from the position of the correlation (e.g. where
  the channel impulse response sits mid-window); the channel impulse response from
  the stored `v(i)`; and the **frequency offset (or Doppler) from the phase shift
  between the two channel impulse responses**. Because the allowance is ±75 Hz,
  those calculations are performed **in three separate Doppler channels**.

## Annex E — error correction, interleaving and the message protocol

Transcribed 2026-10-22 from the same scan (PDF pages 50–64 = E-1 … E-3-1;
Appendix 1 is E-1-1 and E-1-2, and Appendices 2 and 3 are performance tables).
**None of this is cryptography.** It is forward error correction, an interleaver
and a start/end-of-message convention; the traffic's own encryption, where there
is any, sits above this layer and is not described by the standard.

### The code (E-1, E-2)

* **Constraint length 7, rate 1/2 convolutional encoder**, both summing nodes
  modulo-2 — the ordinary NASA/CCSDS code.
* Generator polynomials, **T1 first**:
  `G1 = x⁶ + x⁴ + x³ + x + 1` = **133 octal**;
  `G2 = x⁶ + x⁵ + x³ + x² + 1` = **171 octal**.
* The output switch runs at **twice** the input bit rate, T1 then T2.
* For rates below 600 bps the lower coded rate is made by **repeating the
  *pairs* of output bits** — T1,T2,T1,T2,… — never T1 repeated then T2 repeated.

| coded data rate | waveform | effective code rate | how |
|---|---|---|---|
| 2400 bps | 8-phase (3600 bps) | 2/3 | rate 1/2 **punctured** to 2/3 |
| 1200 bps | 4-phase (2400 bps) | 1/2 | unmodified |
| 600 bps | 2-phase (1200 bps) | 1/2 | unmodified |
| 300 bps | 2-phase (1200 bps) | 1/4 | rate 1/2 repeated 2× |
| 150 bps | 2-phase (1200 bps) | 1/8 | rate 1/2 repeated 4× |
| 75 bps | 2-phase (1200 bps) | 1/16 | rate 1/2 repeated 8× |

So a **BPSK (1200 bps) transmission is 600, 300, 150 or 75 bps of data** — the
constellation cannot tell which, and the SOM's correlation at the decoder output
is what does.

The puncturing to 2/3 happens **after** the interleaver, so a 2400 bps link puts
4800 bps into the interleaver and takes 3600 out. (The scan does not say *which*
bit of each four is dropped; that is the one parameter here that is still a
guess.)

### The interleaver (E-2 … E-5)

A convolutional interleaver, slightly modified:

* **32 rows, I = 32, at every rate.**
* Row `r` holds **r·j bits** — "each shift-register has j more bits of storage
  than the one above it", row 0 having none.
* **j**, i.e. the delay increment per row:

| data rate | long interleave (10.24 s) | short interleave (0.853 s) |
|---|---|---|
| 2400 bps | **48** | 4 |
| 1200 bps | **24** | (scaled by rate) |
| 600–75 bps | **12** | (scaled by rate) |

* **The commutators at the interleaver's input and the deinterleaver's output
  are non-sequential**: the normal 0…31 multiplied by 9 modulo 32 —
  `0, 9, 18, 27, 4, 13, 22, 31, 8, 17, 26, 3, 12, 21, 30, 7, 16, 25, 2, 11,
  20, 29, 6, 15, 24, 1, 10, 19, 28, 5, 14, 23`.
* **The output commutator is the normal 0…31** at every rate except 2400, where
  it takes 24 bits out of the 32 rows per cycle — every fourth row skipped:
  `0,1,2,4,5,6,8,9,10,12,13,14,16,17,18,20,21,22,24,25,26,28,29,30` — repeated for
  every eight 8-PSK symbols.
* **All rows shift together** after each complete cycle of the commutators, which
  is what keeps the input and output bit timing related.
* **Synchronisation:** the top (0th) row is always the starting position for the
  first bit of every data frame of the starting waveform, and **every frame of
  the transmitted waveform holds an integer multiple of 32 data bits**. That is
  the only synchronisation the interleaver needs.

### Message framing (E-5, E-6)

* The encoder and interleaver registers are **set to zero** before any message.
* **SOM = `03873C3C`** (hex, leftmost bit sent first) is inserted before the
  first message bit.
* **EOM = `4B65A5B2`** is appended after the last.
* Then **zeros equal to the interleaver delay plus 102** flush the interleaver
  and the decoder, and the transmission may stop:

| data rate | flush zeros, 10.24 s interleave | short interleave |
|---|---|---|
| 2400 | 24678 | 2150 |
| 1200 | 12390 | 1126 |
| 600 | 6246 | 614 |
| 300 | 3174 | 358 |
| 150 | 1638 | 230 |
| 75 | 870 | 166 |

(The 102 is the maximum practical traceback delay of the decoder, which is why
the count is the delay plus 102 rather than the delay.)

### Decoding (Appendix 1, E-1-1 and E-1-2)

* **Soft decisions, and a soft-decision Viterbi decoder.** The standard says so
  in as many words: "excellent performance … is achieved if maximum likelihood
  decoding (Viterbi algorithm) of soft bit decisions is utilized".
* The soft value starts as the **linear distance from the decision boundary** —
  for BPSK, the real component — and is normalised by the average squared
  distance from the ideal points, with a time constant of about **100 ms**.
  Positive means a 0, negative a 1.
* **For 2400 bps a fourth soft decision is made per 3-bit 8-PSK symbol, and its
  value is zero — a complete erasure**, standing for the punctured bit. That is
  what makes 3600 received bits 4800 soft decisions and a rate-1/2 decode.
* The **deinterleaver takes soft decisions in the normal order and gives them
  out in the ×9 order**, and is **pre-filled with zeros (erasures)**.
  2400/1200/600 take two soft decisions at a time; **300/150/75 take 4, 8 and 16
  at a time and average the even and odd ones separately** into two values.
* **The last 26 bits of the SOM are correlated against the decoder output** until
  they match; that is the start of the message. If no SOM appears within
  (flush zeros + 32) bits, the output is taken anyway.
* The output is then watched for the **EOM**; reception ends there, or when the
  80-symbol sync has not been seen for one interleaver delay.

### What performance to expect (Appendices 2 and 3)

600 bps on a single non-fading path reaches a bit-error rate of 3.4×10⁻⁴ at
**10 dB** signal-to-noise in 3 kHz, and 0 at 15 dB; 1200 bps is 3.2×10⁻² at 10 dB
and 0 at 15; 2400 bps needs about 20 dB for 4.5×10⁻⁴. Two equal Rayleigh paths
2 ms apart cost roughly 5 dB. **A rate-1/2 coded link is therefore readable at
SNRs where the raw constellation is barely readable** — which is what the
interleaver and the code are for.

## What this settles, and what it does not

**Settled against the standard itself**, so nothing here needs re-deriving from a
description of it:

* the transcoding of all three bit rates, and the `e^(jnπ/4)` symbol convention;
* both generators: the 31-symbol sync (x⁵ + x² + 1, init `11010`, LSB first) and the
  176-symbol data scrambling (x⁹ + x⁴ + 1, init 1, three bits per symbol, shift by
  three);
* the frame: 256 symbols at 106.6 ms, 80 sync + 48 reference + 128 data, the four
  32-data blocks with 16-symbol references between them, references all symbol 0;
* the receiver tolerances: ±75 Hz and 3.5 Hz of rate, and the three-Doppler-channel
  consequence;
* the acquisition method: matched filter against PN31 with a cyclic-shift reference,
  error-count threshold, and Doppler from the phase between impulse responses.

**Not settled here:** the error-correction coding, the interleaver lengths and the
message protocol — that is **Annex E**, present in the scan and not transcribed
above. The end-of-message pattern and the per-rate coding choices live in decoder
documentation rather than in the pages read for this file.

**And a recording is still wanted.** The standard says how to acquire; it cannot
say what a real station's Doppler and multipath look like on the bench. A 60-second
off-air capture settles that, and it is cleaner of licence questions than taking a
sample out of a proprietary decoder's installer.
