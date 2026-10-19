import sys, numpy as np
sys.path.insert(0,'/tmp/opencode')
from readwav import read_wav
a,sr,_=read_wav('/tmp/opencode/skysweep/sta4285.wav')
# analytic envelope of the band-passed signal (300-3300 Hz)
A=np.fft.fft(a); f=np.fft.fftfreq(len(a),1/sr)
A[(np.abs(f)<300)|(np.abs(f)>3300)]=0
x=np.fft.ifft(A)
env=np.abs(x)
p=np.percentile(env,[10,50,90,99])
print(f"envelope percentiles 10/50/90/99: {p.round(4)}  ratio 90/10 = {p[2]/max(p[0],1e-9):.1f}")
print("  (8-PSK is constant modulus: a clean one gives ~1.2-1.5, fading or noise much more)")
# symbol-rate spectral line: 8-PSK shows energy at +/- baud offsets
S=np.abs(np.fft.rfft(a*np.hanning(len(a))**0.5)); fr=np.fft.rfftfreq(len(a),1/sr)
def line(f0):
    m=(fr>f0-60)&(fr<f0+60); return S[m].max()
base=np.median(S[(fr>300)&(fr<3300)])
for f0 in (1800-2400, 1800+2400, 2400, 1200):
    if 0 < f0 < sr/2:
        print(f"  level near {f0:5.0f} Hz: {line(f0)/(base+1e-12):5.1f}x median")
# how much of the file is signal?
band=(np.abs(f)>300)&(np.abs(f)<3300)
print(f"in-band power fraction: {(np.abs(A[band])**2).sum()/(np.abs(np.fft.fft(a))**2).sum():.3f}")
