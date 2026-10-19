import sys, numpy as np
sys.path.insert(0,'/tmp/opencode')
from readwav import read_wav

BAUD=2400.0; CENTRE=1800.0; FRAME=256; SYNC=80; SPS=8   # samples per symbol after resampling
FS = BAUD*SPS

def lfsr(taps, n, init):
    s=[(init>>i)&1 for i in range(max(taps)+1)]; out=[]
    for _ in range(n):
        out.append(s[0]); fb=0
        for t in taps: fb^=s[t]
        s=s[1:]+[fb]
    return np.array(out)

prn = lfsr([5,2], SYNC, 0b11010)
bits = lfsr([9,4], 511*3, 0b1).reshape(-1,3)
n_sc = bits[:,0] + 2*bits[:,1] + 4*bits[:,2]
scr = np.exp(1j*np.pi/4*n_sc)

a, sr, meta = read_wav('/tmp/opencode/skysweep/sta4285.wav')
t = np.arange(len(a))/sr
bb = a*np.exp(-2j*np.pi*CENTRE*t)
h=np.hanning(201); h/=h.sum()                       # ~±1.5 kHz at 11025
bb=np.convolve(bb,h,'same')
# resample to exactly SPS per symbol
n=int(len(bb)*FS/sr)
bb=np.interp(np.arange(n)*sr/FS, np.arange(len(bb)), bb.real) + 1j*np.interp(np.arange(n)*sr/FS, np.arange(len(bb)), bb.imag)
print(f"{len(a)/sr:.1f}s, {sr} Hz {meta[2]}bit -> baseband {FS:.0f} Hz, {SPS} samples/symbol")

sym = bb[::SPS]                                     # one sample per symbol, best sub-phase to be found
ns = len(sym)
print(f"{ns} symbols = {ns/BAUD:.1f}s = {ns/FRAME:.1f} frames")

def corr(start, sub=0):
    idx = np.arange(SYNC)*SPS + sub
    if start*SPS + idx[-1] >= len(bb): return 0.0
    z = bb[start*SPS+idx]
    c = np.sum(z*(1-2*prn))
    return abs(c)/(np.sum(np.abs(z))+1e-12)

best=(0,None)
for sub in range(SPS):
    for s in range(0, ns-SYNC):
        v=corr(s,sub)
        if v>best[0]: best=(v,(s,sub))
print(f"best normalised sync |corr| = {best[0]:.3f} at symbol {best[1][0]}, sub-phase {best[1][1]}  (1.0 = perfect)")
