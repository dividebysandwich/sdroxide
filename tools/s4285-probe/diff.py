import sys, numpy as np
sys.path.insert(0,'/tmp/opencode')
from readwav import read_wav
SPS=8; BAUD=2400.0
def lfsr(taps,n,init):
    st=[(init>>i)&1 for i in range(max(taps)+1)]; o=[]
    for _ in range(n):
        o.append(st[0]); fb=0
        for tt in taps: fb^=st[tt]
        st=st[1:]+[fb]
    return np.array(o)
a,sr,_=read_wav('/tmp/opencode/skysweep/sta4285.wav')
t=np.arange(len(a))/sr
bb=a*np.exp(-2j*np.pi*1800*t); h=np.hanning(151); h/=h.sum(); bb=np.convolve(bb,h,'same')
FS=BAUD*SPS; n=int(len(bb)*FS/sr); k=np.arange(n)
bb=np.interp(k*sr/FS,np.arange(len(bb)),bb.real)+1j*np.interp(k*sr/FS,np.arange(len(bb)),bb.imag)
best=(0,None)
for taps in [(2,5),(1,5),(1,4),(0,4),(3,4)]:
    for init in (0b11010,0b01011,0b1011,31):
        p=np.where(lfsr(taps,80,init)>0,1.0,-1.0)
        dp=p[1:]*p[:-1]                     # differentially encoded PRN
        for sub in range(SPS):
            sym=bb[sub::SPS]
            d=sym[1:]*np.conj(sym[:-1])     # differential detection
            mag=np.convolve(np.abs(d),np.ones(79),'valid')+1e-12
            c=np.abs(np.convolve(d,dp,'valid'))/mag
            i=int(np.argmax(c))
            if c[i]>best[0]: best=(float(c[i]),(taps,init,i,sub))
print(f"best differential correlation = {best[0]:.3f}  {best[1]}")
