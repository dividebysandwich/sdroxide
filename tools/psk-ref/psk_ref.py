"""An independent PSK-robust decoder, written from fldigi's own source.

Its only purpose is to answer a question the Rust receiver cannot answer about
itself: does PSK500R.wav carry the whole text? Nothing here shares code with
sdroxide.
"""
import sys
import wave
import numpy as np

MF_CODE = None


def mfsk_table():
    # (code, length) for chars 0..255, from fldigi src/mfsk/mfskvaricode.cxx
    codes = [
        0x75C, 0x760, 0x768, 0x76C, 0x770, 0x774, 0x778, 0x77C, 0x0A8, 0x780, 0x7A0, 0x7A8,
        0x7AC, 0x0AC, 0x7B0, 0x7B4, 0x7B8, 0x7BC, 0x7C0, 0x7D0, 0x7D4, 0x7D8, 0x7DC, 0x7E0,
        0x7E8, 0x7EC, 0x7F0, 0x7F4, 0x7F8, 0x7FC, 0x800, 0xA00, 0x004, 0x1C0, 0x1FC, 0x2D8,
        0x2A8, 0x2A0, 0x200, 0x1BC, 0x1F4, 0x1F0, 0x2B4, 0x1E0, 0x0A0, 0x1D8, 0x1D4, 0x1E8,
        0x0E0, 0x0F0, 0x140, 0x154, 0x174, 0x160, 0x16C, 0x1A0, 0x180, 0x1AC, 0x1EC, 0x1F8,
        0x2C0, 0x1DC, 0x2BC, 0x1D0, 0x280, 0x0BC, 0x100, 0x0D4, 0x0DC, 0x0B8, 0x0F8, 0x150,
        0x158, 0x0C0, 0x1B4, 0x17C, 0x0F4, 0x0E8, 0x0FC, 0x0D0, 0x0EC, 0x1B0, 0x0D8, 0x0B4,
        0x0B0, 0x15C, 0x1A8, 0x168, 0x170, 0x178, 0x1B8, 0x2E8, 0x2D0, 0x2EC, 0x2D4, 0x2B0,
        0x2AC, 0x014, 0x060, 0x038, 0x034, 0x008, 0x050, 0x058, 0x030, 0x018, 0x080, 0x070,
        0x02C, 0x040, 0x01C, 0x010, 0x054, 0x078, 0x020, 0x028, 0x00C, 0x03C, 0x06C, 0x068,
        0x074, 0x05C, 0x07C, 0x2DC, 0x2B8, 0x2E0, 0x2F0, 0xA80, 0xAA0, 0xAA8, 0xAAC, 0xAB0,
        0xAB4, 0xAB8, 0xABC, 0xAC0, 0xAD0, 0xAD4, 0xAD8, 0xADC, 0xAE0, 0xAE8, 0xAEC, 0xAF0,
        0xAF4, 0xAF8, 0xAFC, 0xB00, 0xB40, 0xB50, 0xB54, 0xB58, 0xB5C, 0xB60, 0xB68, 0xB6C,
        0xB70, 0xB74, 0xB78, 0xB7C, 0x2F4, 0x2F8, 0x2FC, 0x300, 0x340, 0x350, 0x354, 0x358,
        0x35C, 0x360, 0x368, 0x36C, 0x370, 0x374, 0x378, 0x37C, 0x380, 0x3A0, 0x3A8, 0x3AC,
        0x3B0, 0x3B4, 0x3B8, 0x3BC, 0x3C0, 0x3D0, 0x3D4, 0x3D8, 0x3DC, 0x3E0, 0x3E8, 0x3EC,
        0x3F0, 0x3F4, 0x3F8, 0x3FC, 0x400, 0x500, 0x540, 0x550, 0x554, 0x558, 0x55C, 0x560,
        0x568, 0x56C, 0x570, 0x574, 0x578, 0x57C, 0x580, 0x5A0, 0x5A8, 0x5AC, 0x5B0, 0x5B4,
        0x5B8, 0x5BC, 0x5C0, 0x5D0, 0x5D4, 0x5D8, 0x5DC, 0x5E0, 0x5E8, 0x5EC, 0x5F0, 0x5F4,
        0x5F8, 0x5FC, 0x600, 0x680, 0x6A0, 0x6A8, 0x6AC, 0x6B0, 0x6B4, 0x6B8, 0x6BC, 0x6C0,
        0x6D0, 0x6D4, 0x6D8, 0x6DC, 0x6E0, 0x6E8, 0x6EC, 0x6F0, 0x6F4, 0x6F8, 0x6FC, 0x700,
        0x740, 0x750, 0x754, 0x758,
    ]
    return {c: i for i, c in enumerate(codes)}


def read_wav(path):
    w = wave.open(path)
    n = w.getnframes()
    a = np.frombuffer(w.readframes(n), dtype="<i2").astype(np.float32) / 32768.0
    return a, w.getframerate()


def baseband(x, fs, fc, cutoff):
    """Mix to baseband and lowpass.

    The cutoff is 0.9 x the baud rate and the filter is 257 taps: a short
    filter with its corner at the band edge ate the outer part of the main lobe
    and read nothing even on samples the Rust receiver decodes.
    """
    t = np.arange(len(x)) / fs
    z = x * np.exp(-2j * np.pi * fc * t)
    n = 257
    m = np.arange(n) - (n - 1) / 2
    cut = 0.9 * cutoff
    h = 2 * cut / fs * np.sinc(2 * cut / fs * m) * np.hamming(n)
    h /= h.sum()
    return np.convolve(z, h, mode="same")


def symbols(z, sps):
    # pick the sampling phase that makes the differential product most real
    best = None
    for phase in range(sps):
        s = z[phase::sps]
        d = s[1:] * np.conj(s[:-1])
        score = np.mean(np.abs(d.imag) / (np.abs(d) + 1e-20))
        if best is None or score < best[0]:
            best = (score, phase)
    return z[best[1]::sps]


class Interleave:
    def __init__(self, size, depth, forward):
        self.size, self.depth, self.forward = size, depth, forward
        self.t = np.full(size * size * depth, 0 if forward else 128, dtype=np.uint8)

    def symbols(self, p):
        size, depth = self.size, self.depth
        for k in range(depth):
            base = size * size * k
            for i in range(size):
                row = base + i * size
                self.t[row:row + size - 1] = self.t[row + 1:row + size]
                self.t[row + size - 1] = p[i]
            for i in range(size):
                row = base + i * size
                p[i] = self.t[row + (size - i - 1 if self.forward else i)]


def viterbi(soft_a, soft_b, poly1=0x6D, poly2=0x4F, k=7):
    n = 1 << (k - 1)
    out = [[((bin(i & poly1).count("1") & 1), (bin(i & poly2).count("1") & 1))
            for i in range(1 << k)]][0]
    metric = np.full(n, -1e9)
    metric[0] = 0.0
    back = np.zeros((len(soft_a), n), dtype=np.uint8)
    for t in range(len(soft_a)):
        r0, r1 = soft_a[t], soft_b[t]
        nxt = np.full(n, -1e9)
        for ns in range(n):
            b = ns & 1
            for from_hi in (0, 1):
                p = (ns >> 1) | (from_hi * (n // 2))
                o1, o2 = out[(p << 1) | b]
                m = metric[p] + (r0 if o1 else -r0) + (r1 if o2 else -r1)
                if m > nxt[ns]:
                    nxt[ns] = m
                    back[t, ns] = from_hi
        best = nxt.max()
        if np.isfinite(best):
            nxt -= best
        metric = nxt
    s = int(np.argmax(metric))
    bits = np.zeros(len(soft_a), dtype=np.uint8)
    for t in range(len(soft_a) - 1, -1, -1):
        bits[t] = s & 1
        s = (s >> 1) | (int(back[t, s]) * (n // 2))
    return bits


def decode(path, baud, depth, fc, interleaved=True):
    x, fs = read_wav(path)
    z = baseband(x, fs, fc, baud)
    sps = int(round(fs / baud))
    sym = symbols(z, sps)
    d = sym[1:] * np.conj(sym[:-1])
    # fldigi's soft symbol is an unsigned byte with 128 as the erasure point,
    # and the interleaver stores exactly those bytes.
    soft = np.clip(np.round(128.0 + 127.0 * d.real / (np.abs(d) + 1e-20)), 0, 255).astype(np.uint8)

    table = mfsk_table()
    best = None
    for parity in (0, 1):
        inlv = Interleave(2, depth, False)
        sa, sb = [], []
        prev = None
        for n, v in enumerate(soft):
            if prev is not None and n % 2 == parity:
                pair = [np.uint8(v), np.uint8(prev)]
                if interleaved:
                    inlv.symbols(pair)
                sa.append((float(pair[1]) - 128.0) / 127.0)
                sb.append((float(pair[0]) - 128.0) / 127.0)
            prev = v
        bits = viterbi(sa, sb)
        # MFSK varicode: a word ends on ...,0,0 and the next 1
        shreg = 0
        text = ""
        for b in bits:
            shreg = ((shreg << 1) | int(b)) & 0x1FFFF
            if (shreg & 7) == 1:
                code = shreg >> 1
                c = table.get(code)
                text += chr(c) if c else "?"
                shreg = 1
            elif shreg > 0x10000:
                shreg = 1
        clean = "".join(ch if ch.isprintable() else "." for ch in text)
        hits = sum(w in text for w in ("EAGLE", "LANDED", "SDRBROWN"))
        if best is None or hits > best[0]:
            best = (hits, parity, clean)
    return best


if __name__ == "__main__":
    path = sys.argv[1]
    baud = float(sys.argv[2])
    depth = int(sys.argv[3])
    fc = float(sys.argv[4])
    interleaved = sys.argv[5] != "none" if len(sys.argv) > 5 else True
    hits, parity, text = decode(path, baud, depth, fc, interleaved)
    print(f"{hits}/3 parity {parity}: {text[:300]!r}")
