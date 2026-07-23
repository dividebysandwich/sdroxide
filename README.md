# sdroxide

A PowerSDR/Thetis-style software-defined-radio transceiver client in Rust, with
pluggable radio backends (**SoapySDR**, **OpenHPSDR**, **TCI**, and **CAT**), an
[egui](https://github.com/emilk/egui) GUI, and a cyberpunk theme. It runs as a **native desktop application** and, from the same
binary, as a **server that streams the same UI to a web browser** over
WebSocket. It includes an integrated, persistent **logbook** and full
**FT8/FT4** digital-mode operation.

<img width="1683" height="933" alt="image" src="https://github.com/user-attachments/assets/8e708021-573e-42e0-b0a5-9ac9c294f874" />

> ## [User Manual](docs/USER_MANUAL.md)

One binary, three ways to run it:

- **Native** — a local desktop transceiver against your SDR hardware.
- **Server** — `sdroxide --server`; the DSP runs on the machine with the radio
  and the full UI (plus audio and the waterfall) is served to a browser as
  WebAssembly. One remote client at a time.
- **Native remote** — `sdroxide --connect host:4950`; the desktop UI driving a
  remote server instead of local hardware.

## Core features

- **Panadapter** — GPU (wgpu) waterfall + spectrum line, wheel-zoom around the
  cursor, drag-to-pan, per-digit frequency readout, selectable colormaps,
  peak-hold, and a **one-click auto-contrast** ("FIT") that picks the display
  floor/ceiling from the signals currently on screen.
- **Bandplan overlay** — a colour-coded strip along the bottom of the waterfall
  that labels allocations (ham bands, broadcast, CB, AM); it shows coarse bands
  when zoomed out and CW/digital/SSB sub-segments when zoomed into a ham band.
- **Modes** — SSB (USB/LSB), CW, AM, SAM, NFM, WFM, DSB, DIGU/DIGL, a
  spectrum-only mode, **FT8/FT4**, the keyboard modes **PSK31** and **RTTY**, and
  image **SSTV** (Scottie, Martin, Robot).
- **Receiver** — hang AGC, draggable passband filter edges (on the spectrum and
  the waterfall), noise blanker, squelch, a second sub-receiver, RIT/XIT, VFO
  A/B with split, per-band band stacks, and memory channels.
- **Transmit** — PTT and tune carrier, drive/ALC metering, device-aware
  half-duplex sequencing (HackRF) or full-duplex (LimeSDR), and a ham-band /
  TX-range lockout so you can't key outside your allocation.
- **Resizable layout** — drag the frequency-scale strip to resize the spectrum
  vs. waterfall split; in FT8/FT4, drag the divider to resize the operating
  panel.
- **Persistence** — device, rates, gains, memories, band stacks, the FT8/FT4
  operator profile, and the logbook are all stored under
  `~/.config/sdroxide/`.

## FT8 / FT4

<img width="1683" height="933" alt="image" src="https://github.com/user-attachments/assets/02a4b70d-7590-4a71-aacb-56814132b691" />

Selecting FT8 or FT4 switches the panadapter to a zoomed sub-band waterfall with
a decode list and an auto-sequencing QSO panel:

- Click a decoded line to move your TX audio frequency onto that signal (a faint
  marker appears on the world map); press **REPLY** to start an auto-sequenced
  QSO, or **Call CQ** to call.
- A dot-matrix **world map** shows your grid, the station you're working, and an
  animated pulse travelling the great-circle path while you transmit.
- Own callsign, grid, and message templates are set in the FT8/FT4 setup dialog
  and persisted.
- All decoding and encoding run server-side in the native engine, so native and
  browser clients behave identically.

## PSK31 and RTTY

Selecting **PSK** or **RTTY** opens a live keyboard-mode ragchew panel next to a
zoomed sub-band waterfall — tune onto a signal, watch it decode, and type a
reply that transmits as you type:

- **Receive** streams decoded text into a scrolling window. Fine-tune with the
  **−/+** buttons (±10 Hz) onto the carrier; RTTY draws mark/space tuning lines
  on the waterfall.
- **Transmit** as you type: characters already sent turn **green** so you can
  watch the transmission catch up to your typing. **TX** keys/unkeys, **CALL CQ**
  loads and sends a CQ macro, **CLEAR** empties the buffer.
- **PSK** is BPSK31 (differential BPSK, varicode). **RTTY** defaults to 45.45
  baud / 170 Hz shift / Baudot; shift (170/425/850 Hz) and baud (45/50/75) are
  selectable in the PSK/RTTY setup dialog.
- The **PSK and RTTY skimmers** decode signals across each band's PSK/RTTY
  calling sub-bands and label them on the waterfall; click a label to switch to
  that mode, tune onto it, and open the panel.

## SSTV

Selecting **SSTV** opens an image panel with a received-image gallery on the
left and a transmit compositor on the right:

- **Receive** decodes incoming pictures scanline-by-scanline into the gallery;
  the VIS header sets the mode automatically (and pre-selects it for your next
  transmit). Received images are saved under `~/.config/sdroxide/sstv_rx/`.
- **Transmit** from a strip of five image slots — click to select, double-click
  (or click an empty slot) to pick a file, which is auto-cropped/scaled to the
  mode's size. A multi-line message is overlaid on the image, **each line in a
  different font**, bold with a black outline; a live preview shows exactly what
  will be sent. Every transmitted image carries a small red→black header strip
  with "SDRoxide" and the version. **TX** sends; **ABORT TX** stops.
- **Modes:** Scottie 1 / 2 / DX, Martin 1 / 2, Robot 72, Robot 36. Band buttons
  tune to that band's SSTV calling frequency (e.g. 20 m = 14.230 MHz).

## Logbook

Open the **LOG** button (available in any mode) for a persistent logbook that
holds both FT8/FT4 and manually entered QSOs:

- Entries are grouped into daily **sessions** with a time span and QSO count.
- **+ New Entry** adds a manual QSO (call, frequency, mode, RST, grid, comment,
  UTC date/time); **EDIT** and **DEL** amend or remove any past entry.
- FT8/FT4 QSOs are logged automatically as they complete.
- Export the whole book to **ADIF** (`.adi`, importable into standard logging
  software) or plain **TXT**.
- The log is stored at `~/.config/sdroxide/qso_log.json` (native) or in browser
  storage (remote).

## Radio backends

sdroxide can drive four kinds of radio, selected on the **Radio** tab of the
Settings window. Backend, serial, and radio-audio changes apply live when you
press **Apply / reconnect**:

- **SoapySDR** — any [SoapySDR](https://github.com/pothosware/SoapySDR) device
  (wideband IQ). See below.
- **OpenHPSDR** — Hermes/Metis-family Ethernet SDRs on the LAN (Protocol 1 and
  2). Press **Discover** to scan for devices, or enter the IP manually; pick a
  DDC sample rate (48 kHz–1536 kHz). Not yet hardware-verified — testers can run
  `RUST_LOG=sdroxide_hpsdr=debug sdroxide` for connection/RX diagnostics (see the
  user manual, §5.4).
- **CAT / Audio** — a CAT-controlled rig (Icom/CI-V, Yaesu, Xiegu) with audio
  over a USB sound card, as either demodulated mono audio or stereo IQ.
- **TCI** — a TCI (Transceiver Control Interface) server such as ExpertSDR3 or
  Thetis over WebSocket (default `127.0.0.1:50001`): wideband IQ receive plus
  audio transmit.

The wideband-IQ backends (SoapySDR, HPSDR, TCI) drive the full panadapter, the
CW/PSK/RTTY skimmers, and internal demodulation; a CAT rig feeding demodulated
audio shows only a narrow audio-band slice.

## SoapySDR connectivity

sdroxide talks to any [SoapySDR](https://github.com/pothosware/SoapySDR) device.
It has been developed against a **HackRF One** (half-duplex TX) and a
**LimeSDR** (full-duplex TX).

- Select a device with `--device`, using SoapySDR argument syntax, e.g.
  `--device driver=hackrf` or `--device driver=lime,serial=...`. With no
  argument it uses the configured device, else the first one found.
- `sdroxide --probe` lists all detected devices and their probed capabilities
  (frequency and sample-rate ranges, gains, antennas, sensors, duplex) and
  exits.
- Capabilities drive the UI: RX-only devices hide all TX controls, band buttons
  grey out outside the device's tunable range, and SWR/power meters appear only
  when the device exposes those sensors.
- Hardware-free sources for testing: `--siggen` (built-in signal generator) and
  `--file <raw CF32 IQ>`.

## Building

You need the SoapySDR development libraries and the driver module(s) for your
radio installed (e.g. `soapysdr`, `soapysdr-module-hackrf`,
`soapysdr-module-lms7` on Arch/Debian-style distros).

```sh
cargo build --release
./target/release/sdroxide --probe        # verify your device is seen
```

The browser client is a separate WebAssembly crate built with
[Trunk](https://trunkrs.dev/):

```sh
cd crates/sdroxide-web && trunk build --release
```

Build the server with `--features embed-web` to bake the web client into the
binary so `--server` needs no `--web-root`.

## Running

```sh
# Native desktop, tuned to 20 m, FT8:
sdroxide --freq 14074000 --mode ft8

# Server: DSP + hardware here, UI in a browser at http://<host>:4950
sdroxide --server

# Desktop UI driven by a remote server:
sdroxide --connect 192.168.1.10:4950
```

## Startup parameters

| Flag | Description |
| --- | --- |
| `--device <ARGS>` | SoapySDR device args (e.g. `driver=hackrf`). Default: config, then first device found. |
| `--probe` | List devices and their probed capabilities, then exit. |
| `--console` | Terminal (ASCII) waterfall mode, no GUI. |
| `--siggen` | Use the built-in signal generator instead of hardware. |
| `--file <FILE>` | Play a raw interleaved CF32 IQ file instead of hardware. |
| `--freq <HZ>` | Center frequency in Hz (default `14200000`). |
| `--rate <HZ>` | Sample rate in Hz (default: from config). |
| `--gain <DB>` | Overall RX gain in dB (default: hardware AGC / moderate). |
| `--mode <MODE>` | Initial mode: `USB LSB CW AM SAM NFM WFM DIGU DIGL DSB SPEC FT8 FT4 PSK RTTY SSTV`. |
| `--server` | Run as a server: HTTP web client + WebSocket streaming backend. |
| `--connect <HOST[:PORT]>` | Connect as a native remote client to a running server. |
| `--port <PORT>` | Server port (default: from config, `4950`). |
| `--web-root <DIR>` | Directory with the Trunk-built web client (default: embedded assets with `--features embed-web`). |
| `--fft <N>` | Spectrum FFT size (default `4096`). |
| `--tx-tune <SECS>` | Headless TX smoke test: key a tune carrier at minimal drive, then exit. |
| `--ft8-cq <SECS>` | Headless FT8 smoke test: call CQ at minimal power, then exit. |
| console extras | `--fps <N>` lines/sec, `--width <CHARS>`, `--db-floor <dBFS>`, `--db-ceil <dBFS>`. |

## Keyboard shortcuts

Active whenever a text field isn't focused.

| Key | Action |
| --- | --- |
| `←` / `→` | Tune ∓/± 100 Hz (hold **Shift** for 10 Hz fine steps) |
| `↑` / `↓` | Tune ± 1 kHz |
| `PageUp` / `PageDown` | Tune ± 10 kHz |
| `M` | Toggle mute |
| `N` | Toggle the noise blanker |
| `F` | Fit the panadapter to the full device passband |

## Mouse operation

**Panadapter (spectrum + waterfall)**

| Action | Result |
| --- | --- |
| Left-click | Tune the active VFO to that frequency. In FT8/FT4, sets the TX audio offset instead. |
| **Shift** + left-click | Tune VFO B (sub-receiver) to that frequency. |
| Left-drag | Grab and slide the spectrum — pans the view and tunes along with it. |
| Right-drag | Pan the view only (no tuning). |
| Scroll wheel | Zoom in/out around the cursor. |
| Drag a passband edge | Move that filter edge (works on the spectrum and the waterfall). |
| Drag the frequency-scale strip | Resize the spectrum vs. waterfall split. |
| Drag the waterfall / FT8 panel divider | Resize the FT8/FT4 operating panel. |

**Frequency readout** — scroll the wheel over a digit to step that digit; click
its upper half to increment, lower half to decrement.

**FT8/FT4 decode list** — click a row to move your TX audio onto that signal
(and preview it on the map); press **REPLY** to start an auto-sequenced QSO.


