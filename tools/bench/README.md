# The bench harness

One command moves a transmission from fldigi, through the rig and the RSP1, into
the decoder the program ships, and says whether the text came back.

```sh
cd ~/sdroxide/tools/bench
./bench.py --mode OLIVIA-16/500 --tones 16 --bw 500 --reverse 1 \
           --text "TEST TEST THE EAGLE HAS LANDED 12345"
```

It prints the mode fldigi actually selected, the audio centre it found, the
level, the decoded text, and **PASS** or **FAIL**.

## What it exists for

Every wasted round in the 2026-10-20 bench session was one of four things, and
all four are decided here instead of by hand:

| the trap | what the harness does |
|---|---|
| fldigi in the wrong mode | sets it over XML-RPC and **prints what it got** |
| fldigi left in TX | releases `main.rx()` itself, because Olivia's idle tones keep it keyed for ever |
| the wrong audio centre | finds the strongest block of the mode's own width rather than assuming |
| a saturated capture | reports the level, and `--gain` defaults to the quiet end of the scale |

## Requirements

- **fldigi running with XML-RPC enabled.** `CHKUSEXMLRPCIS` in `fldigi_def.xml`
  (set to 1); it listens on `127.0.0.1:7362`. It now persists, because fldigi
  saves its configuration on exit.
- **The rig on VOX**, with its sound card's `PCM` level not at zero —
  `amixer -c 1 sget PCM`. That one has cost an evening by itself.
- **The RSP1 free: sdroxide must be closed**, because it holds the device.
- `tools/rsp1-capture/target/release/rspcap` built, and
  `target/release/examples/decode_file` (`cargo build --release -p
  sdroxide-olivia --example decode_file`).

Nothing here tunes the radio. Set the transmitter's dial by hand and pass `--hz`
to match; the harness only listens.

## What the arguments mean

`--mode` is fldigi's own name (`OLIVIA-16/500`, `OLIVIA-32/1K`, `MFSK-16`, …) —
`modem.get_names()` over XML-RPC lists them. `--tones` and `--bw` describe the
mode to *our* engine, which needs the numbers rather than the name.

`--reverse 1` inverts the tone order. The operator's rig sends **LSB** on 11 m,
so this is usually 1 for Olivia and RTTY; the harness still needs to be told,
because that is exactly the question being asked.

`--gain` runs 20 (loudest) to 59 (quietest) — the scale is inverted from what it
looks like. Five metres from a 4 W rig saturates the RSP1 at every setting, so
the antenna is a **stub**, not a whip.

## Not yet written: the other direction

The same harness can prove our **transmit** side, which is still open: have
sdroxide send, and read `text.get_rx()` back from fldigi. That is the test that
settles whether another program can copy what we send — today it cannot, and the
`our_transmitter_is_not_yet_readable` test records it.

## The sweep: a sample of every mode

```sh
./sweep.py --only "^(BPSK|QPSK|PSK)" --tx-seconds 15
./sweep.py --only "^(OLIVIA|Cont|MT63)" --tx-seconds 45 --resume
./sweep.py --resume                 # everything not yet in the manifest
```

Each mode is set on fldigi, transmitted, recorded, measured and saved to
`~/Downloads/sounds/our-captures/fldigi/<mode>.wav`, with one JSON object per mode
appended to `manifest.jsonl` as it finishes — so an interrupted sweep keeps what
it has, and `--resume` carries on.

The text is the operator's, verbatim, spaces included:

```
 AAA TEST TEST THE EAGLE HAS LANDED 12345 SDRBROWN ZZZ
```

`--tx-seconds` has to come from the mode: Olivia 16/500 is about two characters a
second and needs 45, BPSK31 is five and needs 15. Batches by family keep a slow
mode from starving a fast one, and keep a failure to ten minutes rather than an
hour.

Read the result with:

```sh
./summarise.py            # a table, sorted by mode
./summarise.py --failed   # only the misses
./summarise.py --markdown  # writes manifest.md beside the manifest
```

`summarise.py` is the answer to "does our X work?" — one line per mode, with the
decoder's own output quoted where it passed.

### Only Olivia decodes today

`sweep.py` runs `decode_file` for Olivia and records `decoder: null` for
everything else, because that is the only modem we have written against the
vendored engine. The corpus is the point: a recording per mode, with known text
and measured parameters, so each decoder starts with a fixture instead of a hunt.
