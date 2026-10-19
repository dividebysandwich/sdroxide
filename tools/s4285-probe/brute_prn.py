import sys, itertools, numpy as np
sys.path.insert(0,'/tmp/opencode')
from readwav import read_wav
SYNC=80; SPS=8; BAUD=2400.0
a,sr,_=read_wav('/tmp/opencode/skysweep/sta4285.wav')
t=np.arange(len(a))/sr
bb=a*np.exp(-2j*np.pi*1800*t)
h=np.hanning(151); h/=h.sum(); bb=np.convolve(bb,h,'same')
FS=BAUD*SPS; n=int(len(bb)*FS/sr); k=np.arange(n)
bb=np.interp(k*sr/FS,np.arange(len(bb)),bb.real)+1j*np.interp(k*sr/FS,np.arange(len(bb)),bb.imag)

def gen(taps, init_bits, right, n=SYNC):
    s=list(init_bits); o=[]
    for _ in range(n):
        o.append(s[0]); fb=0
        for t in taps: fb^=s[t]
        s = s[1:]+[fb] if right else [fb]+s[:-1]
    return np.array(o)

inits = {
 'msb-first 11010': [1,1,0,1,0],
 'lsb-first 11010': [0,1,0,1,1],
 'msb-first 10110': [1,0,1,1,0],
 'ones':            [1,1,1,1,1],
}
res=[]
for taps in itertools.combinations(range(5),2):
    for iname,ib in inits.items():
        for right in (True,False):
            prn=np.where(gen(taps,ib,right)>0,1.0,-1.0)
            if prn.sum() in (0,SYNC): continue
            best=0
            for sub in range(SPS):
                sym=bb[sub::SPS]
                mag=np.convolve(np.abs(sym),np.ones(SYNC),'valid')+1e-12
                c=np.abs(np.convolve(sym,prn,'valid'))/mag
                best=max(best,float(c.max()))
            res.append((best,taps,iname,right))
res.sort(reverse=True)
for b,t,i,r in res[:6]:
    print(f"{b:.3f}  taps={t} init={i} right={r}")
print(f"... worst {res[-1][0]:.3f} of {len(res)} combinations")
