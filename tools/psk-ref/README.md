# An independent PSK-robust decoder, for telling a sample fault from a port fault

`psk_ref.py` reads a BPSK/PSK-robust recording with a decoder written from
fldigi's own `src/psk/psk.cxx`, and **shares no code with `sdroxide-dsp`**. That
is the whole point of it: when our receiver cannot read a corpus sample, the
question is whether the sample or the port is at fault, and a second
implementation with a different front end — a fixed symbol phase instead of a
Gardner loop, a plain FIR instead of the live one — is the only instrument that
can answer it.

It settled `PSK500R`, whose recording carries `AAA TEST TEST THE EAGLE HAS ` and
nothing after it: this decoder reads the same 27 characters and nothing more, at
every centre from 1700 to 1800 Hz.

```sh
# mode, baud, interleave depth, audio centre
python3 tools/psk-ref/psk_ref.py samples/fldigi/PSK125R.wav 125 40 1754
python3 tools/psk-ref/psk_ref.py samples/fldigi/BPSK63F.wav 62.5 2 1738 none
```

The last argument is `none` for a mode whose reference does not interleave
(BPSK63F alone). The output is `hits/3 parity <n>: "<text>"`, where `hits`
counts `EAGLE`, `LANDED` and `SDRBROWN`.

Two things it is not:

- **not fast** — it is pure Python over a whole recording, seconds per file;
- **not a gate** — the corpus gates are in `sdroxide-dsp`. This is the tie
  breaker, and it is the reason the PSK500R verdict is a measurement rather than
  a suspicion.

It needs `numpy`. The mode table and the varicode are copied from fldigi's
source, as the port's own are; if a convention is wrong in both, this cannot see
it, and a loopback cannot either.
