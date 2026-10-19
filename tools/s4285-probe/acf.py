import sys, numpy as np
sys.path.insert(0,'/tmp/opencode')
from readwav import read_wav
a,sr,_=read_wav('/tmp/opencode/skysweep/sta4285.wav')
t=np.arange(len(a))/sr
bb=a*np.exp(-2j*np.pi*1800*t)
h=np.hanning(151); h/=h.sum(); bb=np.convolve(bb,h,'same')
# complex autocorrelation over a range of lags around the frame period
win=int(sr*0.25)
lags=np.arange(int(sr*0.02), int(sr*0.25))
def acf(lag):
    x=bb[:len(bb)-lag]; y=bb[lag:]
    return abs(np.sum(x*np.conj(y)))/(np.sqrt(np.sum(np.abs(x)**2)*np.sum(np.abs(y)**2))+1e-12)
vals=np.array([acf(l) for l in lags])
fr=int(round(sr*256/2400))
print(f"frame period = 256/2400 s = {256/2400*1000:.2f} ms = {fr} samples at {sr} Hz")
print(f"|ACF| at the frame period: {vals[np.argmin(np.abs(lags-fr))]:.3f}")
top=np.argsort(vals)[-5:][::-1]
print("top lags:", [(int(lags[i]), round(vals[i],3), f"{lags[i]/sr*1000:.1f}ms") for i in top])
print(f"median |ACF|: {np.median(vals):.3f}")
# and a narrower search right at the period
near=lags[np.abs(lags-fr)<40]; nv=np.array([acf(int(l)) for l in near])
print(f"best within 40 samples of {fr}: {nv.max():.3f} at {near[np.argmax(nv)]}")
