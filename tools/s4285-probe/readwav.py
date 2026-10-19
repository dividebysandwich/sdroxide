import struct, numpy as np
def read_wav(path):
    d=open(path,'rb').read()
    assert d[:4]==b'RIFF' and d[8:12]==b'WAVE', 'not a wav'
    i=12; fmt=None; data=None
    while i+8<=len(d):
        cid=d[i:i+4]; sz=struct.unpack('<I',d[i+4:i+8])[0]; body=d[i+8:i+8+sz]
        if cid==b'fmt ':
            tag,ch,sr,_,_,bits=struct.unpack('<HHIIHH',body[:16]); fmt=(tag,ch,sr,bits)
        elif cid==b'data':
            data=body
        i+=8+sz+(sz&1)
    tag,ch,sr,bits=fmt
    if bits==8:   a=(np.frombuffer(data,dtype=np.uint8).astype(np.float32)-128.0)/128.0
    elif bits==16:a=np.frombuffer(data,dtype='<i2').astype(np.float32)/32768.0
    else: raise ValueError(bits)
    if ch>1: a=a.reshape(-1,ch)[:,0]
    return a, sr, (tag,ch,bits)
