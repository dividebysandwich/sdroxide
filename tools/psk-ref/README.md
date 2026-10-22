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

# the C family (multi-carrier, no FEC): baud, "carriers", count, audio centre
python3 tools/psk-ref/psk_ref.py samples/fldigi/PSK500C2.wav 500 carriers 2 1700
```

The `R`/`RC` form takes its interleave depth, and the last argument is `none`
for a mode whose reference does not interleave (BPSK63F alone). The output is
`hits/3 parity <n>: "<text>"`, where `hits` counts `EAGLE`, `LANDED` and
`SDRBROWN`.

**The `C` form is a different front end on purpose.** The `C` family is
`numcarriers` differential BPSK signals `1.4 x baud` apart carrying PSK31's
varicode, and the reference decodes them with **one symbol clock for the whole
bank** (`psk::rx_process` keeps a single `bitclk` and estimates its error from
the *mean* magnitude of every carrier, then reads all of them at that instant).
So does this: a fixed phase shared by all carriers, no timing loop anywhere,
which means it cannot drift between carriers. It settled the three `C`/`RC`
samples that read nothing — see `MODE-PROGRESS.md`.

Two things it is not:

- **not fast** — it is pure Python over a whole recording, seconds per file;
- **not a gate** — the corpus gates are in `sdroxide-dsp`. This is the tie
  breaker, and it is the reason the PSK500R verdict is a measurement rather than
  a suspicion.

It needs `numpy`. The mode table and the varicode are copied from fldigi's
source, as the port's own are; if a convention is wrong in both, this cannot see
it, and a loopback cannot either.
