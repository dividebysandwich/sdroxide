import sys, numpy as np
sys.path.insert(0,'/tmp/opencode')
from readwav import read_wav
SYNC=80; SPS=8; BAUD=2400.0
def lfsr(taps,n,init):
    s=[(init>>i)&1 for i in range(max(taps)+1)]; o=[]
    for _ in range(n):
        o.append(s[0]); fb=0
        for t in taps: fb^=s[t]
        s=s[1:]+[fb]
    return np.array(o)
prn=np.where(lfsr([1,5],SYNC,0b11010)>0,1.0,-1.0)
a,sr,_=read_wav('/tmp/opencode/skysweep/sta4285.wav')
t=np.arange(len(a))/sr
bb=a*np.exp(-2j*np.pi*1800*t)
h=np.hanning(151); h/=h.sum(); bb=np.convolve(bb,h,'same')
FS=BAUD*SPS; n=int(len(bb)*FS/sr); k=np.arange(n)
bb=np.interp(k*sr/FS,np.arange(len(bb)),bb.real)+1j*np.interp(k*sr/FS,np.arange(len(bb)),bb.imag)

best=(-1,None)
for sub in range(SPS):
    sym=bb[sub::SPS]
    N=len(sym)
    mag=np.convolve(np.abs(sym),np.ones(SYNC),'valid')+1e-12
    nn=np.arange(N)
    for f in np.arange(-120,121,10.0):
        rot=sym*np.exp(-2j*np.pi*f/BAUD*nn)
        c=np.abs(np.convolve(rot,prn,'valid'))/mag
        i=int(np.argmax(c))
        if c[i]>best[0]: best=(c[i],(i,sub,f))
print(f"best normalised sync = {best[0]:.3f} at symbol {best[1][0]}, sub-phase {best[1][1]}, offset {best[1][2]:+.0f} Hz")
s0,sb,f0=best[1]
print(f"  frame start = {s0/BAUD*1000:.1f} ms; frame period 106.67 ms => frames at that phase: {s0 % 256} within the frame")
