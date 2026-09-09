# SDR Oxide — CB / SWL edition

> **This is a fork.** This repository is **`madmedicnl/sdroxide11M`**, a fork of
> [**SDR Oxide**](https://github.com/dividebysandwich/sdroxide) ("upstream") by
> dividebysandwich. It is aimed first at **citizens'-band (11 m / CB) operators**
> and **shortwave listeners (SWL)**, without dropping the wider radio
> machinery upstream ships. The fork tracks the upstream `main` branch, so
> upstream's fixes and features keep flowing in — see
> [Relationship to upstream](#relationship-to-upstream).

What you get is the same program: a PowerSDR/Thetis-style SDR transceiver in
Rust with a native desktop GUI and a browser-served web UI, pluggable radio
backends, a panadapter, an integrated logbook, and built-in digital modes —
plus the CB and SWL work described below.

---

## What this fork adds

Everything else in this document that is not on this list is upstream's
feature, carried here unchanged:

| Piece | What it does |
| --- | --- |
| **11 m band** | The citizens' band, **26.965–27.710 MHz**, present as a real band in the band list and band-plan overlay (`da8e4400`). |
| **WSJT-CB** | CB callsigns on FT8/FT4 (`26AT715`, `1A1`, the split `27SL/AM`, and friends) are decoded — upstream's WPJT-X-style plausible-message filter dropped them — and two CB stations run **auto-sequenced QSOs in WSJT-CB's free-text vocabulary** (`3fef2c44`). |
| **mfsk-core fork** | The WSJT-77 validator that makes the above possible lives in `madmedicnl/mfsk-core` (branch `cb-support`), pinned as a `[patch.crates-io]` dependency like the other vendored crates. |
| **nord palettes** | `nord` and `nord_dark` waterfall/spectrum colormaps (`c1c458da`, `64b0e90b`). |

For CB operators the pair that matters is the 11 m band plus WSJT-CB: point an
HF-capable SDR at 27 MHz and the FT8/FT4 panel works the digital side of the CB
band the way the WSJT-CB client does — decoding `26AT715`, proposing the
short free-text exchange (`CQ`, address pair, report, rogered report, `RR73`,
`73`), and logging the completed contact. A CB station and a ham station still
interoperate: mixed pairs keep the hashed/spelled amateur layouts.

For shortwave listeners the fork adds nothing special — the SWL story below is
the upstream radio, which is an unusually good one for the job.

---

## The shortwave-listener side

Everything a serious SWL rig wants is here, tuned as a receiver first:

- **Shortwave broadcast labels.** ~4,600 longwave/shortwave broadcast stations
  label the waterfall's AM carriers, each carrying its UTC transmit window and
  the transmitter site it actually radiates from. Only stations on air right
  now are shown. The schedule is downloaded from
  [EiBi](https://www.eibispace.de/) on first run and at each season change,
  falls back to a built-in copy offline, and accepts your own stations in
  `~/.config/sdroxide/broadcast_stations.json`.
- **DRM and xHE-AAC.** The DRM digital-shortwave decoder speaks the modern
  broadcasters' codec, xHE-AAC, via a *runtime* `libfdk-aac` — installed or
  not, the DRM window says so clearly. (Free with GPL software cannot be: that
  is why it is looked up at startup rather than built in.)
- **Band-plan overlay.** A colour-coded strip below the waterfall labels
  allocations — ham bands, broadcast, **CB**, **AM** — coarse when zoomed out,
  segmented (CW/digital/SSB) when zoomed into a band.
- **The right receivers.** Native USB drivers for the classic cheap SWL SDRs —
  **RTL-SDR** (no libusb, no vendor package; the V3's HF direct-sampling port
  works), **RX-888**, **Airspy HF+ / R2 / Mini**, **HackRF**, **SDRplay**,
  **ELAD**, **HydraSDR**, **Fobos** — plus the network ones every SWL knows:
  **SpyServer**, **rtl_tcp** / `rsp_tcp`, **KiwiSDR / Web-888** (browse the
  ~900 public receivers from `rx.kiwisdr.com` with **PUBLIC SDR** and open one
  in a tab of its own), and **PlutoSDR**. A radio missing at startup is retried
  in the background and attaches on its own.
- **A capable receiver.** Hang AGC, draggable passband edges, noise blanker,
  auto-notch, four noise-reduction engines (RNNoise, DeepFilterNet3, a
  libspecbleach port and a built-in spectral NR), squelch, a second
  sub-receiver, VFO A/B with split, per-band band stacks, memory channels, and
  a **scanner** that sweeps a range using the panadapter's own FFT — sub-second
  across 2 m, so minutes on HF.
- **WSPR, both ways.** Receive every two-minute slot with each beacon's
  locator, signal report, declared power and distance; transmit a beacon that
  picks its slots from your callsign; **WHO HEARD ME** polls wsprnet.org for
  reports of your own call — the only feedback a beacon ever gets. Spots upload
  to WSPRnet as decoded. (An 11 m WSPR adjacency: WSPR still fits the 50-bit
  stretched message, so a plain call + grid is welcome.)
- **Recording and audio.** Sessions to WAV with the details right; a
  **voice reads the dial out** (locally, never streamed) and can announce
  decodes addressed to you.
- **Propagation picture.** A **PROP** layer on a 3D globe places what you hear
  at the *midpoint of each path*, coloured per band — patches of
  shortwave sky, not dots at the far stations. Toggleable under the flat map in
  the operating panel.

---

## The CB (11 m) side

- The band spans **26.965–27.710 MHz** and is treated as a citizen band, not an
  amateur allocation.
- **FT8 / FT4 / FT2** run the ordinary auto-sequenced QSO panel — click a decode
  to move your TX audio onto that signal, **REPLY** to start a sequenced QSO or
  **Call CQ** — with the WSJT-CB vocabulary on top (see
  [What this fork adds](#what-this-fork-adds)). The three are one protocol at
  three speeds: 15 s / 7.5 s / 3.75 s slots.
- A dot-matrix **world map** shows your grid, the station you are working and
  an animated pulse along the great-circle path while you transmit.
- Own callsign and message templates are set in the FT8/FT4/FT2 setup dialog
  and persisted. A CB callsign is a normal citizen call here: `26AT715`,
  `1A1`, or the split `27SL/AM` form.
- **JS8**, the slow-text keyboard mode, also runs on 11 m (4 speeds, directed
  messaging, heartbeats, free text).
- What the band plan knows is labelled in the **band-plan overlay**, and the
  ordinary **keyboard modes** — PSK31, RTTY, Olivia, THOR, FSQ — plus **SSTV**
  and the **image/spectrum-painting** modes work anywhere an audio channel fits,
  11 m included.

> Radio notes for CB users: sdroxide is a *transceiver*; on 11 m you are
> normally listening (FM/AM/SSB) or doing FT8/FT4. Transmit is gated by the
> general **ham-band / TX-range lockout** — see `--oob-tx` below if you
> operate licensed out of band. Check your licence and allocation before keying.

---

## One binary, three ways to run it

- **Native** — a local desktop transceiver against your SDR hardware.
- **Server** — `sdroxide --server`; the DSP runs on the machine with the radio
  and the full UI (plus audio and the waterfall) is served to a browser as
  WebAssembly. Every radio the station has is served, one client each: `/ws`
  and `/ws/<id>`, listed at `/radios`. The roster is editable from a client
  too — a signed-in operator can add a radio and close one again without
  touching the machine.
- **Native remote** — `sdroxide --connect host:4950`; the desktop UI driving a
  remote server instead of local hardware.

All decoding and encoding runs server-side in the native engine, so native and
browser clients behave identically.

State — device, rates, gains, memories, band stacks, the FT8/FT4/FT2 operator
profile, network/upload credentials, control bindings and the logbook (`qso_log.json`) —
is persisted under `~/.config/sdroxide/`.

## Modes at a glance

| Group | What's included |
| --- | --- |
| Voice / broadcast | SSB (USB/LSB), AM, **SAM**, NFM with CTCSS/DCS, WFM with RDS/RBDS, ISB, DSB, DIGU/DIGL |
| Digital QSO | **FT8 / FT4 / FT2**, **WSJT-CB** (CB callsigns on the same wire), **JS8** (4 speeds) |
| Keyboard | PSK31, RTTY, Olivia, THOR, **FSQ** (directed messaging + images) |
| Image / paint | **SSTV** (Scottie, Martin, Robot), **RIFP** (packetised pictures), **RF Paint** (draw on the waterfall) |
| Weak signal / beacons | **WSPR** (RX + beacon, WSPRnet reporting, band hopping) |
| Fax | Weather fax (WEFAX/radiofax) with a station picker, phasing and slant correction |
| Digital voice | **RADE** (FreeDV RADE V1 neural speech codec) |
| Airborne | ADS-B (receive, radar display) and VDL2 (the aircraft datalink, 7 channels at once) |
| CW | CW with a deep-learning decoder side-by-side with a conventional one, plus your own message buttons |
| Exotic | DRM shortwave, Winlink radio email (over the internet CMS today) |

## Running

```sh
# Native desktop, 11 m:
sdroxide --freq 27000000 --mode ft8

# A server: DSP + hardware here, UI in a browser at http://<host>:4950
sdroxide --server

# The desktop UI driving a remote server (no web client involved):
sdroxide --connect 192.168.1.10:4950
```

Useful flags (full table in the [user manual](docs/USER_MANUAL.md)):

| Flag | Description |
| --- | --- |
| `--device <ARGS>` | SDR device args (e.g. `driver=hackrf`). Default: config, then first device found. |
| `--probe` | List devices and their probed capabilities, then exit. |
| `--freq <HZ>` | Center frequency in Hz (default: where the last session was left; `14200000` on a first run). |
| `--mode <MODE>` | Start in a mode (e.g. `ft8`, `am`, `lsb`, `wspr`). |
| `--console` | Terminal (ASCII) waterfall mode, no GUI. |
| `--siggen` | Use the built-in signal generator instead of hardware. |
| `--file <FILE>` | Play a raw interleaved CF32 IQ file instead of hardware. |
| `--server` / `--connect <HOST:PORT>` | Run as a server / as a remote client. |
| `--web-root <DIR>` | Serve a Trunk-built web client from disk (default: embedded, with `--features embed-web`). |
| `--ft8-cq <SECS>` | Headless FT8 smoke test: call CQ at minimal power, then exit. |
| `--oob-tx` | Lift the amateur-band transmit lockout for this run (never persisted). |

## Building

```sh
git clone --recurse-submodules https://github.com/madmedicnl/sdroxide11M
cd sdroxide11M
```

The workspace is edition 2024 (Rust 1.85+, install with
[rustup](https://rustup.rs/)); the browser client needs `rustup target add
wasm32-unknown-unknown`. RADE's codec and the rtl_433 decoders are submodules
built from source, so a default build also wants a C toolchain and CMake:

```sh
# Debian / Ubuntu
sudo apt install build-essential pkg-config cmake autoconf automake libtool \
                 libclang-dev libasound2-dev libopus-dev
```

```sh
cargo build --release            # native desktop UI
./target/release/sdroxide --probe
```

- **SoapySDR** (optional): install `soapysdr` and the modules for your radio.
  `cargo build --release --no-default-features` gives a working binary with no
  SoapySDR — everything except SoapySDR, including RTL-SDR, needs no SDR system
  library at all.
- **RTL-SDR on Linux**: install `packaging/linux/60-sdroxide-rtlsdr.rules` and
  replug the dongle (the `.deb` does this for you); the DVB driver is detached
  by the program itself, no blacklist needed. Windows: bind the dongle to WinUSB
  with [Zadig](https://zadig.akeo.ie/) once. macOS: nothing to do. Other radios'
  rules (RX-888, Airspy, HackRF, HydraSDR, ELAD, LimeRFE, relay) live in
  `packaging/linux/`; details in the user manual.
- **Browser client**: a separate WebAssembly crate built with
  [Trunk](https://github.com/trunk-rs/trunk) (`cargo install --locked trunk`,
  then `trunk build --release` in `crates/sdroxide-web`; feed the result to
  `--web-root`, or run `cargo build --release --features embed-web` after that
  to bake it in — a release build actually embeds, a debug build reads it off
  disk).

## Relationship to upstream

This fork branches from upstream `main` at an integration point and adds its own
commits on top; upstream and the fork are maintained as a normal fork (merge
upstream into this branch from time to time, reconcile the overlapping 11 m work
by hand).

- **Upstream repo:** [dividebysandwich/sdroxide](https://github.com/dividebysandwich/sdroxide)
- **Fork repo:** [madmedicnl/sdroxide11M](https://github.com/madmedicnl/sdroxide11M)
- **The fork is currently *behind* upstream**: it is based on sdroxide **1.6.5**,
  while upstream `main` has moved on to **1.6.6** (including upstream's own,
  independent 11 m handling). Merging upstream's newer work back is planned; the
  two 11 m implementations overlap and must be reconciled by hand rather than
  auto-merged.
- **Where to report bugs:** for anything not on the
  [fork-only list](#what-this-fork-adds), please report to *upstream* (it will
  reach more people and remains mergeable). CB/11 m and WSJT-CB issues belong
  here.
- The WSJT-77 validator dependency is itself a fork: `madmedicnl/mfsk-core`
  (branch `cb-support`), pinned in the workspace `[patch.crates-io]` section,
  held as a plain git dependency under upstream's GPL-3.0-or-later licence.

## Contributing, LLM usage, licensing

Both local and hosted LLMs ("Generative AI") were used in the development of
this software. Contributions written using LLMs are OK provided these rules are
observed:

- **Read and review** generated code; you should be able to answer questions
  about your contribution.
- **Document and comment** non-trivial parts of the code.
- **Test** your contribution using real radio equipment; if that is impossible,
  consider whether it is a useful contribution and disclose the need for testing
  help.
- Don't use LLMs for trivial things like changing a constant.
- Use modern, sufficiently sized models with enough context.
- Locally-hosted LLMs are encouraged but not required; keep commits
  vendor-neutral.
- Observe the licence — **GPL-3.0-or-later** — and do not change it.

The licence is GPL-3.0-or-later and inherits one constraint stronger than it.
CW decoding links the [DeepCW](https://github.com/e04/deepcw-engine) model,
which is **AGPL-3.0-only**, and it is linked into the binary rather than read as
a data file — so its terms (section 13: offering the program over a network
counts as conveying it, and the Corresponding Source must be offered) cover the
built program as a whole. Running it on your own machine changes nothing; the
model is confined to the `sdroxide-deepcw` crate and the wasm web client links
none of it. The DRM xHE-AAC decoder is optionally loaded from a separately
installed `libfdk-aac` (see `vendor/fdk-aac/PROVENANCE.md`).

---

*SDR Oxide is the upstream project; the CB / SWL edition is this fork. Original
project and feature set: [dividebysandwich/sdroxide](https://github.com/dividebysandwich/sdroxide).*