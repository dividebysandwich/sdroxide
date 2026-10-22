"""How much of the known text is in the recording? A decoder-independent score.

When a corpus sample will not decode, two very different things can be true: the
recording does not carry the text, or the text is there and our receiver cannot
get it. A decoder cannot answer that about itself. This can: it extracts the bit
stream the mode implies, slides the known text's varicode along it, and reports
the best fraction of bits that match. **1.000 is a clean sample, 0.5 is noise**,
and anything between is the sample's own quality.

```sh
# PSK500C4: baud, carriers, spacing, centre from, centre to, centre step
python3 tools/psk-ref/psk_text_match.py samples/fldigi/PSK500C4.wav 500 4 700 1700 2300 20

# calibrate the instrument first — a clean synthetic bank must read 1.000
python3 tools/psk-ref/psk_text_match.py --calibrate
```

Why it is trustworthy: the synthetic bank in `--calibrate` is built from the same
text through a differential BPSK modulator with fldigi's raised-cosine symbol
shape, and the same extraction must recover it at 1.000. Run that first whenever
a reading is surprising. It caught its own author once already: an early version
built the synthetic symbols from the bits instead of from the *reversals*, and
scored 0.58 on its own clean signal.
"""
import sys

import numpy as np

import psk_ref

TEXT = " AAA TEST TEST THE EAGLE HAS LANDED 12345 SDRBROWN ZZZ"


def known_bits(text=TEXT):
    bits = []
    for ch in text:
        bits.extend(int(b) for b in psk_ref.PSK31_TABLE[ord(ch)])
        bits.extend((0, 0))
    return np.array(bits, dtype=np.int8)


def best_match(stream, pattern):
    """Best fraction of `pattern` found anywhere in `stream`, and where."""
    s = 2.0 * np.asarray(stream, dtype=np.float32) - 1.0
    p = 2.0 * np.asarray(pattern, dtype=np.float32) - 1.0
    if len(s) < len(p):
        return 0.0, 0
    n = 1
    while n < len(s) + len(p):
        n *= 2
    corr = np.fft.irfft(np.fft.rfft(s, n) * np.conj(np.fft.rfft(p, n)), n)[: len(s) - len(p) + 1]
    score = (corr + len(p)) / (2.0 * len(p))
    i = int(np.argmax(score))
    return float(score[i]), i


def bits_at(x, fs, baud, f, phase):
    """Every symbol's differential bit on one carrier at one sampling phase."""
    sps = int(round(fs / baud))
    z = psk_ref.baseband(x, fs, f, baud)
    s = z[phase::sps]
    d = s[1:] * np.conj(s[:-1])
    return (d.real >= 0).astype(np.int8), float(np.mean(np.abs(d.real) / (np.abs(d) + 1e-20)))


def stream(x, fs, baud, carriers, centre, spacing):
    """The interleaved bit stream: one bit from each carrier per symbol time,
    carrier 0 first, which is the order the transmitter emitted them in.

    **One sampling phase for the whole bank**, chosen where the differential
    product is most real across *all* the carriers. That is the reference's own
    arrangement — `psk::rx_process` keeps a single `bitclk` for the bank and
    estimates its error from the mean magnitude of every carrier — and it is
    load-bearing here: a phase chosen per carrier lets one carrier sit a symbol
    behind its neighbours, which permutes the interleave and reads as a sample
    fault. It cost this instrument an afternoon.
    """
    sps = int(round(fs / baud))
    freqs = [centre + (i - (carriers - 1) / 2) * spacing for i in range(carriers)]
    best = None
    for phase in range(sps):
        per_carrier = []
        score = 0.0
        for f in freqs:
            bits, coh = bits_at(x, fs, baud, f, phase)
            per_carrier.append(bits)
            score += coh
        if best is None or score > best[0]:
            best = (score, per_carrier)
    per_carrier = best[1]
    n = min(len(b) for b in per_carrier)
    out = np.empty(n * carriers, dtype=np.int8)
    for k in range(n):
        for c in range(carriers):
            out[k * carriers + c] = per_carrier[c][k]
    return out


def synthetic(baud, carriers, centre, spacing, repeats=20):
    """A clean bank carrying the known text, for calibrating the measurement."""
    fs = 8000.0
    sps = int(round(fs / baud))
    bits = np.tile(known_bits(), repeats)
    bits = bits[: len(bits) // carriers * carriers]
    n = (len(bits) // carriers) * sps
    sig = np.zeros(n)
    for c in range(carriers):
        f = centre + (c - (carriers - 1) / 2) * spacing
        sym = 1.0
        prev = 1.0
        for k, bit in enumerate(bits[c::carriers][: n // sps]):
            if bit == 0:  # a zero is a phase reversal: fldigi's `rx_bit(!bits)`
                sym = -sym
            for i in range(sps):
                a = 0.5 - 0.5 * np.cos(np.pi * (i + 1) / sps)
                sig[k * sps + i] += (prev * (1.0 - a) + sym * a) * np.cos(
                    2 * np.pi * f * (k * sps + i) / fs
                )
            prev = sym
    return sig / np.abs(sig).max(), fs


def calibrate():
    pattern = known_bits()
    for carriers, centre, spacing in ((1, 1700.0, 0.0), (4, 1800.0, 700.0)):
        sig, fs = synthetic(500.0, carriers, centre, spacing)
        best = 0.0
        for fc in np.arange(centre - 60.0, centre + 61.0, 20.0):
            score, _ = best_match(stream(sig, fs, 500.0, carriers, fc, spacing), pattern)
            best = max(best, score)
        verdict = "OK" if best > 0.97 else "BROKEN INSTRUMENT"
        print(f"{carriers} carrier(s) at {centre:.0f} Hz: {best:.3f}  {verdict}")


def main():
    if sys.argv[1] == "--calibrate":
        calibrate()
        return
    path = sys.argv[1]
    baud = float(sys.argv[2])
    carriers = int(sys.argv[3])
    spacing = float(sys.argv[4])
    lo, hi, step = (float(v) for v in sys.argv[5:8])
    x, fs = psk_ref.read_wav(path)
    pattern = known_bits()
    best = (0.0, 0.0)
    fc = lo
    while fc <= hi:
        score, _ = best_match(stream(x, fs, baud, carriers, fc, spacing), pattern)
        if score > best[0]:
            best = (score, fc)
            print(f"   {fc:7.1f} Hz: {score:.3f}")
        fc += step
    print(f"BEST {best[0]:.3f} of the known text at {best[1]:.0f} Hz")


if __name__ == "__main__":
    main()
