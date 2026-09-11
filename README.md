# SDR Oxide BUT tuned for CB use!

A PowerSDR/Thetis-style software-defined-radio transceiver client in Rust, with
pluggable radio backends (**SoapySDR**, **OpenHPSDR**, **TCI**, **SmartSDR**, **Icom LAN**, **ELAD**, and **CAT**), an
[egui](https://github.com/emilk/egui) GUI, and a cyberpunk theme. It runs as a **native desktop application** and, from the same
binary, as a **server that streams the same UI to a web browser** over
WebSocket. It includes an integrated, persistent **logbook**, many digital modes like **FT8/FT4/FT2**
built-in, and **TCI and Hamlib rigctld servers** so third-party programs like WSJT-X can use it as their radio.

<hr/>

<img width="1496" height="933" alt="image" src="https://github.com/user-attachments/assets/9d88118c-0efe-45c5-9918-8ee2bb91b700" />

<hr/>

<img width="1682" height="1212" alt="image" src="https://github.com/user-attachments/assets/aa08f5d3-ec62-4d91-9dd0-13bde1b0ae43" />

<hr/>

<img width="1496" height="933" alt="image" src="https://github.com/user-attachments/assets/902a73ff-c8bf-43cd-9fc3-884d40ce4b04" />

<hr/>

> ## [User Manual](docs/USER_MANUAL.md)

One binary, three ways to run it:

- **Native** — a local desktop transceiver against your SDR hardware.
- **Server** — `sdroxide --server`; the DSP runs on the machine with the radio
  and the full UI (plus audio and the waterfall) is served to a browser as
  WebAssembly. Every radio the station has is served, one client each: `/ws`
  and `/ws/<id>`, listed at `/radios`. Its roster is editable from a client
  too — a signed-in operator can add a radio to the station and close one
  again, without touching the machine or restarting it.
- **Native remote** — `sdroxide --connect host:4950`; the desktop UI driving a
  remote server instead of local hardware. A station with several radios comes
  up with all of them, one tab each.

## Core features

- **Radios** - CAT/Audio, CAT/Stereo IQ, TCI (SunSDR), OpenHPSDR P1 and P2
  (Hermes Lite 2, Apache Labs), SoapySDR (HackRF, etc.), RTL-SDR (native support,
  over USB or over the network via rtl_tcp),
  RX-888 (native support), SDRplay RSP (native, via the vendor API service),
  SmartSDR (FlexRadio - experimental!), PlutoSDR (native support, experimental!),
  Icom LAN / RS-BA1 protocol, HackRF (native support, RX
  verified / TX unmeasured), Airspy R2/Mini (native support, experimental!),
  HydraSDR RFOne (native support, experimental!),
  ELAD FDM-DUO / FDM-S1 / FDM-S2 (native support, experimental!),
  LimeSDR family + LimeRFE front end (via LimeSuite, experimental!),
  RigExpert Fobos SDR (native, via libfobos)
- **Panadapter** — GPU (wgpu) waterfall + spectrum line, wheel-zoom around the
  cursor, drag-to-pan, per-digit frequency readout, selectable colormaps,
  peak-hold, and **auto-contrast** ("FIT", on by default) that keeps the display
  floor/ceiling fitted to the signals currently on screen — refitting on a band
  change, after a pan or zoom, and when the levels drift, eased in rather than
  switched.
- **Modes** — SSB (USB/LSB), CW, AM, SAM, NFM (with **CTCSS/DCS** decoding and
  tone squelch), WFM (with broadcast
  pilot-tone **stereo** and **RDS/RBDS**), DSB, **ISB** (independent sideband —
  the two sidebands are two different transmissions, one to each ear), DIGU/DIGL, a
  spectrum-only mode, **FT8/FT4/FT2**, **JS8** (all four speeds, with directed
  messaging, heartbeats and multi-frame free text), the keyboard modes
  **PSK31**, **RTTY**,
  **Olivia**, **THOR** and **FSQ** (with directed messaging + images),
  **WSPR** (transmit and receive, with WSPRnet reporting and optional band
  hopping),
  **Hellschreiber** (all seven Feld Hell / FSK Hell variants, on a scrolling
  raster), image **SSTV** (Scottie, Martin, Robot), image **RIFP**
  (draft-dulaunoy-rifp-00 — packetised, checksummed pictures over a 4800-baud
  CPFSK modem), receive-only **weather fax** (WEFAX/radiofax charts with a
  station picker, phasing and slant correction), transmit-only **RF Paint**
  (spectrum painting of text and images onto the waterfall), and receive-only
  **ADS-B** — the aircraft overhead on 1090 MHz, on a radar display with history
  dots, speed vectors and data blocks — and receive-only **VDL2**, the VHF
  datalink those same aircraft exchange ACARS over, on all seven channels around
  136.8 MHz at once, with the messages and the stations sending them.
- **Receiver** — hang AGC, draggable passband filter edges (on the spectrum and
  the waterfall), noise blanker, auto-notch, **four noise-reduction engines**
  (RNNoise, DeepFilterNet3, a libspecbleach port and the built-in spectral NR,
  three strengths each), squelch, a second sub-receiver, RIT/XIT, VFO A/B with split,
  per-band band stacks, and memory channels.
- **Winlink radio email** — a built-in client for the amateur store-and-forward
  email network, speaking B2F/FBB, LZHUF and the secure login natively (no Pat,
  no external modem). Mailbox with inbox / outbox / sent / archive, compose and
  reply, attachments, and the session transcript when something goes wrong.
  Over the internet CMS today; AX.25 packet and ARDOP are the planned radio
  transports.
- **Bandplan overlay** — a colour-coded strip along the bottom of the waterfall
  that labels allocations (ham bands, broadcast, CB, AM); it shows coarse bands
  when zoomed out and CW/digital/SSB sub-segments when zoomed into a ham band.
- **Scanner** — work through the memory channels or a frequency range and stop
  where somebody is transmitting, with a configurable threshold (or the
  receiver's own squelch), dwell, skip list, and carrier / timed / manual
  resume. A range scan reads a whole span out of the panadapter's FFT rather
  than visiting channels one at a time, so sweeping 2 m takes well under a
  second instead of minutes.
- **Transmit** — PTT and tune carrier, drive/ALC metering, device-aware
  half-duplex sequencing (HackRF) or full-duplex (LimeSDR), and a ham-band /
  TX-range lockout so you can't key outside your allocation.
- **Resizable layout** — drag the frequency-scale strip to resize the spectrum
  vs. waterfall split; in FT8/FT4/FT2, drag the divider to resize the operating
  panel.
- **Live spotting, awards & QSL** — DX cluster / POTA / SOTA / PSK Reporter spots
  as clickable panadapter markers (click to tune + pre-fill a log entry),
  QRZ/HamQTH callsign lookup, one-click upload to LoTW / eQSL / Club Log /
  QRZ / HamQTH, and live **DXCC / WAS / WAZ / grid** award tracking (worked vs
  confirmed).
- **Control inputs** — every shortcut is rebindable, and any class-compliant
  **MIDI controller** can drive the radio: a jog wheel as the VFO knob, pads as
  PTT and band buttons, faders as gain controls, with LED/motor feedback. Mouse
  buttons take bindings too (a side button held for PTT works as a footswitch),
  and the panadapter wheel can zoom or tune.
- **Spoken announcements** — the radio reads itself out, for operating it
  without seeing it. A neural voice ships with the program and runs locally, so
  nothing is sent anywhere and no speech service has to be installed. It reads
  the frequency once the dial stops moving, folds a band change into one phrase,
  warns when you leave an amateur band, reads the SWR out while you tune up
  (with a warning above 3:1), and announces FT8/JS8 messages addressed to you.
  Announcements play on their own sound device, so they are never recorded and
  never sent to a remote listener. The window is also exposed to NVDA, Orca and
  VoiceOver.
- **T/R switch** — drives an external relay that disconnects and grounds the
  SDR's antenna input while the station transmits, and sequences an amplifier or
  an outboard T/R relay with it (per-contact lead and hold, so the antenna
  always throws before the amplifier is keyed and unkeys before it comes back).
  Works with the cheap USB relay boards (LCUS/CH340, KMtronic, Numato), USB HID
  relay boards, CM108/CM119 sound-card GPIO, a serial RTS/DTR line into any
  interface that wants a PTT closure, a Raspberry Pi GPIO line, or an external
  command. An optional transmit-sense input on the same port sees a rig keyed at
  its own microphone in milliseconds instead of the few hundred a CAT poll
  takes. See "T/R switch" in the user manual for what this cannot do.
- **Persistence** — device, rates, gains, memories, band stacks, the FT8/FT4/FT2
  operator profile, network/QSL credentials, control bindings, and the logbook
  are all stored under `~/.config/sdroxide/`.

## FT8 / FT4 / FT2

Selecting FT8, FT4 or FT2 switches the panadapter to a zoomed sub-band waterfall
with a decode list and an auto-sequencing QSO panel. The three are one protocol
at three speeds — 15 s slots for FT8, 7.5 s for FT4, 3.75 s for FT2 — sharing a
message format, a panel and a logbook:

- Click a decoded line to move your TX audio frequency onto that signal (a faint
  marker appears on the world map); press **REPLY** to start an auto-sequenced
  QSO, or **Call CQ** to call.
- A dot-matrix **world map** shows your grid, the station you're working, and an
  animated pulse travelling the great-circle path while you transmit.
- Own callsign, grid, and message templates are set in the FT8/FT4/FT2 setup dialog
  and persisted.
- **FT2** trades sensitivity and spectrum for speed: 4-GFSK at 41.667 baud,
  167 Hz wide, a 2.52 s burst, and a complete contact in about six seconds. It
  wants an accurate clock — its timing search is only about half a second wide.
- All decoding and encoding run server-side in the native engine, so native and
  browser clients behave identically.

## WSPR

Selecting **WSPR** opens a reception list beside the world map, and a beacon
status pane. WSPR is not a QSO mode — a transmission carries a callsign, a grid
and a power level and nothing else — so what the panel shows is measurements of
paths rather than a conversation.

- **Receive** decodes every two-minute slot. Each row is a beacon heard (`←`) or,
  once "who heard me" is on, a station that heard *this* one (`→`), with its
  locator, signal report, declared power and distance. Reports are coloured on
  WSPR's own scale, where −25 dB is still a good path.
- **Transmit** is off until you ask for it. The panel carries the duty cycle
  (10–50% of slots) and the power you actually radiate, in watts — only the
  nineteen levels WSPR's message can name are offered, because that figure goes
  out on the air and everyone who hears you judges the path by it. The beacon
  picks its slots from your callsign, so two stations running sdroxide do not
  transmit on top of each other, and it moves within the 200 Hz window each
  time. Callsign and grid come from Settings → General, like everywhere else.
- **Band hopping** moves the dial between slots so one receiver samples the whole
  spectrum. Turning the VFO yourself pauses it and says so.
- **WSPRnet** — spots are uploaded as they are decoded (this is on by default; it
  puts nothing on the air and it is what makes a receiver part of the network),
  and **WHO HEARD ME** polls wsprnet.org for reports of your own callsign, which
  is the only feedback a beacon ever gets.
- Transmitting needs a plain callsign and a 4-character locator — the 50-bit
  message has room for nothing else. A compound call or a 6-character grid is
  said plainly rather than mangled; receiving is unaffected.

## Propagation heat map

Everything the station hears becomes evidence about the ionosphere, and the
**PROP** layer on the 3D globe draws it: WSPR both ways, FT8/FT4/FT2 and JS8
decodes, and the logbook.

- Each reception is placed at the **midpoint of its path** — the patch of
  ionosphere that bent it — rather than at the far station, so the picture is a
  map of the sky rather than of where radio amateurs live. Long paths get a
  control point per hop.
- **ALL BANDS** gives every band its own hue, **ONE BAND** runs a
  single band through a blue → green → yellow → red ramp. The same picture can be switched on under the flat map in the operating panel.

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

## Olivia, THOR and FSQ

Three more keyboard modes share the same ragchew panel and setup dialog as
PSK/RTTY; the submode is chosen on each mode's setup page:

- **Olivia** — a very robust MFSK chat mode with Walsh/Hadamard coding. Pick the
  tone count (2–64) and bandwidth (125–2000 Hz); 32/1000 and 16/500 are common.
- **THOR** — DominoEX-family 18-tone incremental-FSK with convolutional FEC.
  Pick a submode (THOR4 … THOR32; THOR16 is the usual default).
- **FSQ** — 33-tone incremental-FSK (speeds FSQ-2/3/4.5/6) with a dedicated panel
  for the **directed (FSQCALL)** layer: a **heard list**, a persistent **contacts**
  book, directed `CALL:message` sends, ALLCALL broadcast, an automatic reply to
  the `?` heard-list query, and **image** transmit/receive (pick a picture to send;
  received pictures land in the gallery).

These modems are native-Rust and self-contained (no external decoder); on-air
interoperability with fldigi is being validated and refined.

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

## RIFP

Selecting **RIFP** opens the same image panel over the **Radio Image Framing
Protocol** ([draft-dulaunoy-rifp-00](https://datatracker.ietf.org/doc/draft-dulaunoy-rifp/)):
a picture is encoded, split into numbered chunks, and sent as CRC-protected
frames behind a JSON manifest, with the complete object verified by CRC-32 and
SHA-256 before it is shown. Interoperates both ways with the
[reference implementation](https://github.com/adulau/rifp) across every encoding
either side can produce.

- **Radio profile** `rifp-cpfsk-4800`: continuous-phase binary FSK, 4800 baud,
  ±4 kHz, sent on the carrier rather than in a sideband — **the dial is the
  centre of the signal**. ⚠ Its ~25 kHz channel does not fit a narrow-band
  segment; the panel warns wherever it does not, and band buttons land in the
  segments where it does — 10 m FM, the 6 m and 2 m all-modes parts, and 70 cm,
  where a **433.920** chip jumps to the calling frequency the draft names.
- **Encodings:** CCITT Group 4 facsimile, PNG, JPEG, and the packed grayscale
  raster raw / RLE8 / ZLIB — or Auto, which sends whichever comes out smallest.
  1, 2, 4 or 8 bits per pixel, with optional dithering.
- **Receive** shows every transfer being reassembled with a per-chunk map of
  what has arrived, paints the raw raster row by row as it lands, and adds a
  picture to the gallery only once its digest checks out.

## RF Paint

Selecting **RFPAINT** opens a transmit-only **spectrum-painting** panel that draws
text and pictures **directly onto a receiver's waterfall** — there is no decoder,
the picture *is* the signal, so anyone watching their panadapter on your frequency
simply sees what you paint. It transmits on USB inside a 3 kHz audio band, so it
fits a normal SSB channel:

- **Text paint** — type a line and it is rendered as upright letters that scroll
  up the far station's waterfall (constant font size — a longer message just makes
  a wider banner / longer transmission).
- **Image paint** — load a PNG/JPEG, reduced to a contrast-stretched grayscale
  bitmap and painted onto the waterfall.
- Each area has a **live preview waterfall** showing exactly how it will look on
  the receiving end, plus a **TRANSMIT** button, a transmit-progress bar, and
  **Abort**.
- A **scan-speed** control (≈6%–100%, default 25%) trades transmission time for
  legibility — slower gives the receiver's waterfall more scan lines to render
  the picture. Transmit goes through the normal path, so the ham-band lockout and
  transmit safety still apply.

## RADE digital voice

Selecting **RADE** switches the receiver to **FreeDV RADE V1** (Radio
Autoencoder) — a neural speech codec carried on an OFDM waveform, which stays
intelligible at signal-to-noise ratios where SSB is just noise. It fits inside a
normal USB channel, occupying roughly 1060–1880 Hz of audio.

- **Receive** replaces the demodulated audio with the decoded speech as soon as
  the modem locks. Out of sync you still hear the raw signal, so you can tune by
  ear; the panel shows a sync lamp, the SNR estimate and the frequency offset,
  and the waterfall is marked with the band the waveform occupies.
- **Transmit** with the panel's **TALK** button or the ordinary PTT. The modem 
  needs ~120 ms of speech before the first frame goes out and appends an 
  end-of-over frame when you stop, so transmit runs on slightly past the button.
- Band buttons tune to the FreeDV calling frequencies (e.g. 20 m = 14.236 MHz).
- Decoding is neural-network inference and runs on its own thread; it is far
  faster than real time on a modern CPU, but the panel warns if the machine
  falls behind.

`rade-harness` (in `crates/sdroxide-rade`) drives the same codec over files, for
bench testing without a radio:

```sh
cargo run -p sdroxide-rade --bin rade-harness -- \
    tx --input vendor/rade_c/wav/david_vk5dgr.wav --output modem8k.wav
cargo run -p sdroxide-rade --bin rade-harness -- \
    rx --input modem8k.wav --speech decoded16k.wav --stats rx.csv
```

## Logbook

Open the **LOG** button (available in any mode) for a persistent logbook that
holds both FT8/FT4/FT2 and manually entered QSOs:

- Entries are grouped into daily **sessions** with a time span and QSO count.
- **+ New Entry** adds a manual QSO. Alongside the basics (call, frequency, mode,
  RST, grid, UTC date/time) the entry form now carries **name, QTH, state,
  country**, transmit **power**, and **contest** fields (contest id + sent/received
  serials); a **worked-before** badge warns when you've already worked that call
  on the band. **EDIT** and **DEL** amend or remove any past entry.
- FT8/FT4/FT2 QSOs are logged automatically as they complete.
- **IMPORT** loads QSOs from an ADIF (`.adi`) file (de-duplicated against the
  existing log); export the whole book to **ADIF** or plain **TXT**. A
  QSL/confirmation status column shows what's been uploaded and confirmed.
- Records also hold DXCC entity, CQ/ITU zones, IOTA and POTA/SOTA references, and
  per-service QSL status — the data behind lookup, upload and award tracking.
- The log is stored at `~/.config/sdroxide/qso_log.json` (native) or in browser
  storage (remote).

## Spotting, awards & QSL upload

Turn the logbook into a live station cockpit. Everything here is configured on
the **Spots** and **Uploads** tabs of the Settings dialog, and surfaced by the
**SPOTS** and **AWARDS** buttons in the System module.

![Live spots as clickable markers on the panadapter, and the SPOTS window](docs/images/14-spots-panel.png)

- **Spot feeds** — connect a **DX cluster** (telnet) and poll **POTA**, **SOTA**
  and **PSK Reporter**. Spots appear as clickable, colour-coded markers along the
  bottom of the waterfall (and as dots on the FT8 world map); the **SPOTS** window
  lists them with per-source filters and a **fuzzy search** over calls, station
  names, sites and frequencies. **Click a spot** to tune the VFO, set the mode,
  and pre-fill a new log entry — one click from "heard" to "working".
- **Broadcast stations** — ~4,600 **longwave and shortwave broadcasts** label the
  AM carriers on the waterfall. Each carries its **UTC transmit window** and the transmitter
  site it actually radiates from, so only the stations on air right now are
  shown, and tuning one draws a great-circle arc from your grid to the
  transmitter on the 3D globe. The schedule is downloaded from
  [EiBi](https://www.eibispace.de/) on first run and again at each season change,
  falling back to a built-in copy when offline. Users can define their own stations and corrections
  in `~/.config/sdroxide/broadcast_stations.json`.
- **Callsign lookup** — auto-fill name, QTH, grid and state from **QRZ.com** or
  **HamQTH** on a spot click, at QSO start, or when you type a call (or press
  **LOOKUP** in the entry form).
- **One-click upload** — push QSOs to **eQSL**, **QRZ Logbook**, **HamQTH** and
  **Club Log** (a per-QSO **UP** button, or automatically as each QSO is logged).
  HamQTH uploads with the same login as the callsign lookup — one account does
  both. **LoTW** is handled by exporting ADIF for TQSL signing; LoTW/eQSL
  **confirmations are downloaded** to mark worked-vs-confirmed.
- **Award tracking** — the **AWARDS** window tallies **DXCC**, **WAS**, **WAZ**
  and **grid squares**, worked vs confirmed, with a per-band filter. DXCC entity
  and CQ/ITU zones are resolved from the callsign (bundled `cty.dat`), so spots
  for a **new entity** are flagged in the SPOTS list. The same resolution puts a
  **country flag and name** on every FT8/FT4/FT2 and JS8 decode, and the decode
  list sorts by country. The flags are compiled into the program (public domain,
  from [region-flags](https://github.com/fonttools/region-flags)) — nothing is
  fetched at runtime to draw them.

Credentials are stored in plaintext under `~/.config/sdroxide/net.json` (as with
other ham software). See the [User Manual](docs/USER_MANUAL.md) for setup steps.

## Radio backends

sdroxide can drive seventeen kinds of radio, selected on the **Radio** tab of the
Settings window. Backend, serial, and radio-audio changes apply live when you
press **Apply / reconnect**. A radio that isn't there yet at startup — or that
drops mid-session — is retried in the background and attaches by itself, so
starting sdroxide before the rig is fine:

- **RTL-SDR (USB)** — an RTL2832U dongle, driven directly over USB by a native
  pure-Rust driver. **No SoapySDR and no libusb needed**, so it works in every
  build including the standard `.msi` and `.dmg`. Covers the R820T, R820T2 and
  R828D tuners, which is effectively every dongle still sold. HF works through
  an RTL-SDR Blog V4's built-in upconverter, or on other sticks by direct
  sampling the ADC's Q branch (the V3's HF port). Bias tee and ppm correction
  are on the Radio tab; see "RTL-SDR permissions" under Building.
  
- **RTL-SDR over rtl_tcp (network)** — the same dongle on another machine — a
  Raspberry Pi at the antenna, say — published with `rtl_tcp -a 0.0.0.0`. The
  same controls as the USB interface, since it is the same radio; the far end
  performs them. Rates are shown with what they cost on the link (the samples
  are uncompressed: 1.024 Msps is 16 Mbit/s, 2.4 Msps is 38), and a dropped
  connection reconnects by itself. `rtl_tcp` has no authentication, so keep it
  on a trusted network or reach it through an SSH tunnel.

- **`rsp_tcp` servers** (an SDRplay RSP published the same way) work here too,
  with the RSP's own controls — antenna input, LNA state, IF gain reduction,
  AGC and set point, notches, reference out. Start the server with **`-E`**: it
  greets exactly like a dongle, and the extended block it sends only in that
  mode is what lets sdroxide name the radio and, more importantly, know when it
  is streaming **16-bit** samples. Without `-E` a `-b 16` server reads as noise,
  because the protocol carries nothing that would say otherwise.

- **SpyServer (network)** — any receiver somebody has published with Airspy's
  `spyserver`, or one of the servers that speak the same protocol: an Airspy
  R2/Mini, an Airspy HF+, or an RTL-SDR behind it. Receive only.
  A server whose receiver another client already owns still works: tuning is
  limited to the slice that client is receiving.

- **SpyServer VFO+FFT, low bandwidth (network)** — the same servers, in the
  mode that makes a remote receiver usable over WiFi or a cellular modem. The
  server sends a *narrow* I/Q stream that follows the dial — 96 kHz, about
  1.5 Mbit/s at 8-bit — plus a low-rate FFT of the whole band, a couple of
  kilobytes a frame. Roughly a hundredth of the link a wideband Airspy stream
  needs.

- **KiwiSDR / Web-888 (network)** — one of the ~900 receivers published on
  `rx.kiwisdr.com`, or a private one on the same firmware. Receive only: these
  are other people's antennas. A KiwiSDR has no wideband I/Q to give — what it
  has is user channels about 12 kHz wide, one of which it will send as complex
  baseband instead of as audio — so the panadapter is that window and the strip
  above it is the receiver's own 0–30 MHz waterfall. Tuning across the strip
  retunes the receiver. About 64 kB/s while connected.

  Browse the public ones with **PUBLIC SDR** in the System box; picking one there
  fills the address in and opens it, either in this radio or in a tab of its
  own.

- **RX-888 (USB)** — an RX-888 or RX-888 Mk2 direct-sampling HF receiver
  (LTC2208 16-bit ADC, Cypress FX3), driven directly over USB by a native
  pure-Rust driver. **No SoapySDR, no libusb, and no vendor driver package.**
  Covers 0–32 MHz by sampling HF directly at up to 129.6 Msps.

  The FX3 on this board has no boot EEPROM, so the receiver appears as a bare
  Cypress bootloader on every plug-in with no radio function at all. sdroxide
  uploads the (MIT-licensed, bundled) firmware itself, so there is nothing to
  install and nothing to run first — plug it in and pick it in Settings. See
  "RX-888 permissions" under Building for the Linux udev rule.

  There is no hardware downconverter in this receiver: the full ADC stream is
  converted to complex baseband on the host, which is why retuning anywhere in
  HF is instantaneous, and why it wants a modern CPU and a real USB 3 port.

  Above the ADC's Nyquist limit the receiver switches automatically to its
  R828D tuner and the VHF SMA, reaching to 1.75 GHz, so both antennas want to
  be connected. The bundled firmware carries no tuner driver — it was removed
  upstream over a licence conflict — so sdroxide drives the tuner itself over
  the firmware's I2C passthrough, including the synthesiser output that clocks
  it. VHF needs an ADC clock of 32.4 Msps or more for the tuner's 8 MHz IF to
  fit under Nyquist. Receive only.

- **Airspy HF+ (USB)** — an Airspy HF+ Dual, Discovery or Ranger, driven
  directly over USB by a native pure-Rust driver. **No SoapySDR, no libusb and
  no libairspyhf needed**, so it works in every build including the standard
  `.msi` and `.dmg`. Up to 912 kSPS of complex baseband over 0.5 kHz–31 MHz and
  60–260 MHz, with the receiver's own AGC and threshold, attenuator, preamp and
  bias tee on the Radio tab. See "Airspy HF+ permissions" under Building for the
  Linux udev rule.

  The host runs the same DSP the vendor library does: an adaptive **IQ image
  balancer**, DC cancellation, and a fine-tuning oscillator. That oscillator is
  not a nicety — the synthesiser is programmed in whole kilohertz and parked
  5 kHz off on the zero-IF rates so its own leakage stays clear of your signal,
  and below its 180 kHz floor the oscillator does *all* the tuning, which is how
  this receiver reaches VLF at all. It can be switched off from the Radio tab to
  see raw hardware output, which is also the one-click way to tell a driver
  problem from a DSP one.

  Which sample rates exist depends on the model *and* the firmware, so the list
  is read off the receiver rather than assumed; a rate the hardware does not
  have is snapped to the nearest one it does, and it says so instead of refusing
  to open. Receive only. **Not yet verified against real hardware** — see the
  user manual, §5.2.9; the Radio tab has a **Copy diagnostic report** button,
  and that report is what makes a fix possible.

- **Airspy R2 / Mini (USB)** — an Airspy R2 or Airspy Mini, driven directly
  over USB by a native pure-Rust driver. **No SoapySDR, no libusb and no
  libairspy needed**, so it works in every build including the standard `.msi`
  and `.dmg`. 24–1800 MHz; up to 10 Msps on an R2, 6 on a Mini. Receive only.
  See "Airspy R2 / Mini permissions" under Building for the Linux udev rule.

  A **different receiver from the Airspy HF+** above — different silicon, a
  different USB id and a different protocol — so it has its own interface and
  its own driver. The two are not variants of each other.

  The rate you pick is the rate you get, but the receiver runs at twice it: its
  ADC is real, not complex, and sdroxide does the downconversion on the host —
  a quarter-rate translate and a half-band decimator, which is also why the
  receiver's own DC offset lands at the *edge* of the span rather than its
  centre. An R2 and a Mini cannot be told apart on the USB bus, so the rate list
  is read off whichever one is connected rather than assumed.

  Gain is a **step along a curve** rather than three sliders: the R820T2's LNA,
  mixer and VGA move together along one of two curated curves — linearity for
  strong-signal handling, sensitivity for weak signals — which is how every
  Airspy program drives this tuner and what the numbers were tuned for. Bias
  tee, the tuner's own AGC loops and 12-bit USB packing are on the same tab.

  **Not yet verified against real hardware** — the Radio tab has a **Copy
  diagnostic report** button, and that report is what makes a fix possible.

- **HydraSDR RFOne (USB)** — a HydraSDR RFOne, driven directly over USB by a
  native pure-Rust driver. **No SoapySDR, no libusb and no libhydrasdr needed**,
  so it works in every build including the standard `.msi` and `.dmg`.
  24–1800 MHz, up to 12 Msps. Receive only. See "HydraSDR RFOne permissions"
  under Building for the Linux udev rule.

  A **fork of the Airspy R2** rather than a relative of it — libhydrasdr still
  carries libairspy's copyright header, vendor requests 0–26 line up number for
  number, and the gain curves are byte-for-byte the same — but its own interface
  all the same, because the two cannot drive each other's hardware: the RFOne
  takes an eight-byte tuning command where the Airspy takes four. Everything
  said above about the rate you pick being half the rate the ADC runs at applies
  here too, for the same reason.

  Three things this radio has that the Airspy does not. **Three RF sockets** —
  `ANT`, `CABLE1` and `CABLE2` — selectable on the Radio tab, with the bias tee
  on the antenna port alone (the panel greys the switch out on the others rather
  than letting it claim DC that is not there). **Seven sample rates** — 12, 10,
  8, 6, 5, 4.096 and 2.5 Msps — of which the receiver only reports three: the
  other four live in the firmware's alternate table, are marked as such in the
  menu, and fall back to a listed rate with a note if a particular firmware
  turns out not to carry them. And **two USB ids**: production boards are
  `38af:0001`, while prototypes came up on `1d50:60a1`, which is the Airspy R2
  and Mini's own. sdroxide separates the two by the USB descriptors before
  opening and by the firmware version string after, so picking the wrong
  interface for either radio is answered with the name of the right one.

  **Not yet verified against real hardware** — the Radio tab has a **Copy
  diagnostic report** button, and that report is what makes a fix possible.

- **HackRF One / Pro (USB)** — a HackRF One or HackRF Pro (or a Jawbreaker or
  rad1o), driven directly over USB by a native pure-Rust driver. **No SoapySDR,
  no libusb and no libhackrf needed**, so it works in every build including the
  standard `.msi` and `.dmg`. DC–7.25 GHz, 2–20 Msps, wideband IQ receive *and*
  transmit. See "HackRF permissions" under Building for the Linux udev rule.

  **Half duplex**: receive stops for the length of an over, the way the hardware
  is built. The transmitter is also **off until you arm it** — a HackRF radiates
  harmonics strong enough to need an external low-pass filter for whatever band
  you are on, so "Enable transmit" is a deliberate switch on the Radio tab
  rather than something a receive session leaves one PTT away.

  The gain model is the radio's own: an **LNA** in 8 dB steps, a baseband
  **VGA** in 2 dB steps, a **TX VGA** in 1 dB steps, and the 14 dB RF amplifier
  — which is one switch shared between the two directions, offered here as
  separate receive and transmit settings so you can run the preamp bypassed on
  receive and in circuit on transmit. sdroxide reprograms the whole front end on
  every change of direction, which is why that works here and does not through
  SoapySDR. Bias tee, baseband filter and ppm correction are on the same tab.

  The tuning range is the **firmware's**, DC–7.25 GHz, the same on every board
  and the same span libhackrf and SoapySDR publish. That is wider than any of
  these radios is specified for — a HackRF One is a 1 MHz–6 GHz receiver — and
  outside that it is heavily attenuated and makes little transmit power. It
  does still tune there, though, which is what people listening to shortwave
  below 1 MHz or watching 5 GHz Wi-Fi are relying on, so the dial goes where
  the radio goes.

  A **HackRF Pro** is the same driver and the same protocol — it shares the
  HackRF One's USB id and every vendor request this driver sends — with three
  differences. It is specified down to **100 kHz** instead of 1 MHz, which is
  most of the reason to have one for the low bands. It accepts sample rates
  down to **250 ksps**, because it decimates in its FPGA rather than running
  the converter slowly. And it **chooses its own baseband filter** (three
  quarters of the sample rate) and ignores anything the host asks for, so that
  control is greyed out. The last two sdroxide reads off the board rather than
  assuming.

  sdroxide decodes the Pro's standard 8-bit stream; its half-precision and
  extended-precision gateware modes are not driven, and a Pro left in one of
  them by `hackrf_debug -P` will need unplugging.

  Receive on a HackRF One is verified against hardware; **transmit has not yet
  been measured, and HackRF Pro is unverified** — the Pro path is
  transcribed from Great Scott Gadgets' firmware and libhackrf sources. In case of problems, use the **Copy diagnostic report** button
  which  records every command exchanged with the radio.

- **SDRplay RSP (USB)** — any SDRplay RSP (RSP1, RSP1A, RSP1B, RSP2, RSPduo,
  RSPdx, RSPdx R2), driven natively through the vendor's **SDRplay API
  service** — no SoapySDR in the path. The RSPs after the original RSP1 have
  no open USB protocol, so this is the one backend that needs a vendor
  package: install the [SDRplay API](https://www.sdrplay.com/api/) (v3.x) and
  make sure its service is running (Linux: `sudo systemctl enable --now
  sdrplay`). Nothing is linked at build time — sdroxide finds the library at
  runtime, so every build variant has the backend and simply reports what to
  install when it is missing (`sdroxide --probe` says which piece is absent).

  Receive only, 1 kHz–2 GHz, up to 10 Msps. The RSP gain model is exposed the
  way the hardware means it: an **IF gain reduction** slider, an **LNA state**
  step control (clamped per band, honestly reported back), and the RSP's own
  hardware **AGC** with an adjustable set point. FM-broadcast and DAB notch
  filters, bias tee, RSP2/RSPdx antenna selection, RSPduo tuner selection and
  RSPdx HDR mode are available on the Radio tab, and only the rows the selected
  model actually supports are shown.

  **An RSPduo can run both of its tuners at once**, and there are two things to
  do with the second one. Combined, it is two aerials on one clock — the same
  arrangement the LimeSDR's second receive chain gives, and the same adaptive
  filter: *cancel* to null a local noise source, or *combine* for diversity
  reception that fills in HF fades, with the controls you actually use while
  listening on the main strip. Left apart, it is simply a second radio: the
  tuners tune separately, so one board can be an HF radio in one tab and a VHF
  radio in another. The API fixes the ADC clock either way, so the widest span
  with both tuners running is 2 Msps.

- **ELAD FDM-DUO / FDM-S (USB)** — an ELAD FDM-DUO, FDM-DUOr, FDM-S2 or FDM-S1,
  driven directly over USB by a pure-Rust driver — no libusb, no gr-elad, no
  SoapySDR module. All three are direct-sampling receivers (a 122.88 MHz ADC,
  61.44 on the S1) with an FPGA down-converter delivering one wideband I/Q
  channel; the S2 and S1 are receive-only, 10 kHz–54 MHz and 10 kHz–30 MHz.

  An **FDM-DUO is three USB devices** and this one interface drives all of them:
  the vendor interface for I/Q, the CAT serial port for rig control, and the
  radio's USB Audio port for transmit audio. Set the CAT port on the same tab
  and the dial, the mode, PTT, the S-meter, the SWR and the transmit power all
  work; leave it empty and the DUO is still tuned and keyed through its *receive*
  cable alone, using the CAT gateway on that interface — everything that needs an
  answer from the radio is what you give up. CW is keyed by the radio's own key
  or paddle: the FDM-DUO has no command that accepts text.

  **An FDM-S1 or FDM-S2 needs its FPGA loaded before it will send anything.**
  These are bus-powered front ends: the USB bridge runs from an EEPROM, so an
  untouched one enumerates, reports its serial and acknowledges the start of the
  stream — while the FPGA behind it comes up empty and no sample ever arrives.
  Install ELAD's own `elad-firmware` loader (their Linux download area) as
  `/usr/local/bin/elad-firmware` and sdroxide runs it for you at every open,
  choosing the image for the Sample rate you picked; the six rates *are* six
  images, which is why nothing in the vendor protocol selects between them. Take
  the "intel" build — the newer "ubuntu-32" one is 32-bit and on a 64-bit machine
  exits silently having done nothing.
  Without the loader sdroxide says so on screen rather than sitting on "waiting
  for spectrum" for ever.

  **On an FDM-DUO the sample rate cannot be commanded.** The radio boots its own
  FPGA, has no menu for the rate, and arrives in whatever mode it was left in
  (192 kHz on a fresh one); there the Sample rate setting says how the stream is
  *read*, and sdroxide measures the real throughput a couple of seconds in and
  tells you on screen if the two disagree. See "ELAD permissions" under Building
  for the Linux udev rule.

  **Not verified against hardware.** The whole backend is written from ELAD's own
  [gr-elad](https://github.com/ELADIT/gr-elad), the FDM-DUO manual's CAT chapter
  and — for the FPGA load, which `gr-elad` does not do at all —
  [SoapyELAD](https://github.com/DisagioDigitale/SoapyELAD). **Copy diagnostic report** on the Radio tab dumps every command
  exchanged with the device, and `cargo run -p sdroxide-elad --example probe`
  does the same from a terminal.

- **LimeSDR + LimeRFE (LimeSuite)** — the LimeSDR family (LimeSDR-USB, Mini v1
  and v2, LimeNET-Micro, PCIe), driven through **LimeSuite** rather than through
  SoapySDR. Wideband I/Q both ways and genuinely full duplex: the receiver keeps
  running through your own transmission.

  A LimeSDR has always been reachable here through the SoapySDR interface, and
  SoapyLMS7 is itself a thin wrapper over this same library — so the I/Q path is
  not what this adds. **The LimeRFE is.** SoapySDR exposes none of it, so the
  band filters, the LNA, the power amplifier and the transmit/receive relay are
  unreachable from that side. Here the front end follows the dial: change band
  and the right filter is in circuit before any RF appears, while tuning *within*
  a band puts nothing at all on the control link.

  **The board's second receiver is the other thing SoapySDR cannot reach.** On
  a LimeSDR-USB or PCIe the two receive chains share one synthesiser and one
  sample clock, so they hear the same span at the same instant — which is what
  lets a second aerial be combined with the first. Two things worth doing with
  that: *cancel*, the DSP form of a noise-cancelling phaser, which finds the
  gain, phase and delay that make a local noise source line up on both aerials
  and subtracts it; and *combine*, diversity reception, which adds the two in
  the phase that reinforces and weights each by how well it hears, filling in
  HF fades. The filter is adaptive and multi-tap, so the null holds across the
  whole span rather than at one frequency, and it can be held once it has
  converged. Which chain you listen on is a setting too, so a board with the HF
  matching modification on one of its low-band inputs can name which socket the
  aerial is in.

  That chain can instead take a **directional coupler on an amplifier's
  output**, and linearise it — the technique openHPSDR calls **PureSignal**.
  The transmitter compares what came back with what it meant to send and emits
  the inverse of the difference, for around twenty decibels less
  intermodulation without backing the amplifier off. The correction stays at
  unity until the feedback lines up with the transmission, and is clamped so it
  can never ask the converter for more than full scale — so an unconnected
  coupler costs nothing and a feedback path reading nonsense cannot over-drive
  anything.

  The LimeRFE is reached either way the hardware allows. Over **its own
  micro-USB port** it is a serial device, driven by pure Rust that needs no
  LimeSuite at all — so that link works whatever is driving the radio. Through
  the **LimeSDR's GPIO header** it is bit-banged I²C on the radio's own pins,
  one cable fewer but far slower: a band change there is the better part of a
  second against a few tens of milliseconds over the serial cable. Pick whichever
  suits the shack; if you change band often, pick the cable.

  **The board receives between overs, and is switched to transmit at key-down.**
  On either cabling: its amateur channels have one filter with a
  transmit/receive switch either side of it, so a board asked for both
  directions at once puts that switch on the transmitter and stops passing
  anything to the receiver. The **Relays** setting is left on *Automatic*, which
  does that and waits for the relay before letting drive out; pin it to *Always
  receive* and transmit is refused outright rather than driven into a closed
  relay. Which connector still decides what is reachable — J5 is one jack for
  both directions and the only path to the HF and 6 m amplifiers.

  **LimeSuite is found at runtime, not linked**, so this interface is in every
  build variant and simply reports what to install where the library is absent
  (Debian/Ubuntu and Arch: `limesuite`; macOS: `brew install limesuite`; Windows:
  the PothosSDR bundle). It needs no SoapySDR module. See "LimeRFE permissions"
  under Building for the Linux udev rule — the LimeSDR itself needs nothing from
  this project, because LimeSuite ships its own rules.

  **Not verified against hardware.** No LimeSDR has been attached to this code.
  **Copy diagnostic report** on the Radio tab dumps the session's library calls,
  and `cargo run -p sdroxide-lime --example probe` does the same from a terminal.
  For the LimeRFE specifically, `cargo run -p sdroxide-limerfe --example rfe --
  /dev/ttyUSB0` talks to the board on its own and prints what happened at each
  step.

- **RigExpert Fobos SDR (USB)** — a Fobos SDR, driven through RigExpert's own
  **libfobos**. That library is LGPL-2.1 and open source
  ([github.com/rigexpert/libfobos](https://github.com/rigexpert/libfobos)), but
  it is still **found at runtime rather than linked** — same as SDRplay and
  LimeSuite above, and for the same reason: this interface is in every build
  variant, nobody needs the library installed to compile sdroxide, and a
  machine without it enumerates nothing and says what to install. Receive only.

  **Three inputs, and they are genuinely different radios.** The **RF port**
  goes through the tuner — 25 MHz–5.4 GHz, the receiver's own reported sample
  rates up to 80 Msps, with the LNA (0–3) and VGA (0–31) gain stages on the
  Radio tab. **HF1** and **HF2** bypass the tuner for direct sampling, which
  means no local oscillator at all: the receiver hands back two independent
  *real* ADC channels, and sdroxide turns the selected one into complex
  baseband itself with the same wideband channelizer the RX-888 uses for the
  identical problem. Tuning on those two is therefore pure software — nothing
  is commanded at the radio — and the gain sliders do nothing, because the
  front end they belong to is powered down while direct sampling runs. The
  panel says so rather than leaving them looking broken.

  **The rate you pick on HF1/HF2 sets how low you can tune.** The
  downconverter selects a band out of a real spectrum, so it can never centre
  closer to DC than half its own output rate: at 2.5 Msps the floor is
  1.25 MHz, which puts the whole AM broadcast band out of reach. The default is
  **625 kHz** for exactly that reason — a floor of 312.5 kHz, under every
  mediumwave channel — and a dial below whatever the current floor is says so
  in the log and reports the frequency it really landed on, rather than
  labelling something else with the frequency you asked for.

  **HF1 + HF2 runs both real channels at once**, combined by the same adaptive
  filter the RSPduo's second tuner and the LimeSDR's second chain use: *cancel*
  to null a local noise source, or *combine* for diversity reception, with mode,
  adaptation rate and hold on the main strip and the filter length on the Radio
  tab. Both channels come off one ADC on one clock, so unlike two independent
  tuners the phase between them is reproducible across a restart. An external
  clock reference is a switch on the same tab.

  **Verified against real hardware** by the contributor who wrote it, on all
  three inputs — including 38.6 dB of measured cancellation on real aerials.
  Two ADC rates turned out to be unusable for streaming on that unit (40 Msps
  never streams; 50 Msps streams but distorts audibly), and the rate selection
  steers around both, at the cost of a practical ceiling around 10 MHz for the
  widest HF views.

- **PlutoSDR (network)** — an ADALM-Pluto, driven directly over the **IIOD**
  protocol its on-board daemon serves. **No SoapySDR and no libiio**, so it
  works in every build including the standard `.msi` and `.dmg`. Wideband IQ
  receive *and* transmit.

  A Pluto is a network device even on a USB cable — the cable presents an
  Ethernet gadget — so it is reached at an address (`192.168.2.1` out of the
  box) rather than by a serial number, and one on the LAN works the same way.
  Press **Discover** to ask the network, or type the address; **Test
  connection** reports what the board says about itself.

  The AD9361's four AGC modes, receive gain, transmit attenuation and both RF
  ports are on the Radio tab. Tuning limits are read off the device, so a stock
  AD9363 board (325 MHz–3.8 GHz), one unlocked to AD9364 (70 MHz–6 GHz) and one
  running F5OEO's [tezuka](https://github.com/F5OEO/tezuka_fw) firmware
  (47.5 MHz–6 GHz, which puts 6 m and 4 m in reach) are all reported correctly
  without a setting. Half duplex by default: receive
  stops for the length of an over, because a USB 2.0 gadget will not carry a
  megasample-per-second stream both ways at once. On a board with real Ethernet
  behind it — a LibreSDR, or a Pluto on a gigabit adapter — tick **Full duplex**
  and the receiver keeps running through your own transmission, which is how a
  QO-100 station listens to its own downlink. Not yet hardware-verified — see
  the user manual, §5.2.7.

- **SoapySDR** — any [SoapySDR](https://github.com/pothosware/SoapySDR) device
  (wideband IQ) — LimeSDR, bladeRF, USRP and friends. See below. A HackRF or an
  Airspy reaches sdroxide better through its own interface above, and the device
  list says so when it finds one.

- **OpenHPSDR** — Hermes/Metis-family Ethernet SDRs on the LAN (Protocol 1 and
  2). Press **Discover** to scan for devices, or enter the IP manually; pick a
  DDC sample rate (48 kHz–1536 kHz). Not yet hardware-verified — testers can run
  `RUST_LOG=sdroxide_hpsdr=debug sdroxide` for connection/RX diagnostics (see the
  user manual, §5.4).

- **CAT / Audio** — a CAT-controlled rig (Icom/CI-V, Kenwood, Yaesu, Elecraft,
  Xiegu, ELAD, QRP Labs QMX/QMX+/QDX, or anything Hamlib's `rigctld` or **flrig**
  drives) with audio over a USB sound card, as either demodulated mono audio or
  stereo IQ. On a QMX, picking stereo IQ also switches the radio's own I/Q mode
  on and sets the 12 kHz I.F. offset its superhet receiver needs.

- **TCI** — a TCI (Transceiver Control Interface) server such as ExpertSDR3 
  over WebSocket (default `127.0.0.1:50001`): wideband IQ receive plus 
  audio transmit.

- **Icom LAN** — an Icom on its own Ethernet/WiFi port (IC-7300MK2, IC-705,
  IC-7610, IC-7760, IC-7851, IC-9700, IC-905, IC-R8600), speaking the same
  IP-remote protocol as RS-BA1 — no RS-BA1 licence and no PC at the radio.
  Control, audio and the radio's own spectrum scope — 475 points, or 689 on an
  IC-7610 or IC-7760 — over one connection. There is no I/Q on
  any Icom; the audio stream carries either demodulated AF or the 12 kHz DRM IF,
  and sdroxide can demodulate the latter itself over about ±12 kHz. Not yet
  hardware-verified.

- **SmartSDR / FlexRadio** — a FLEX-6000 or FLEX-8000 on the LAN. Press
  **Discover** to listen for radios (they announce themselves), or enter an
  address for one reached over a router or VPN. Receive is a **DAX IQ** stream,
  so sdroxide does its own demodulation and the radio's slice follows the dial;
  transmit is DAX audio the radio modulates. DAX IQ tops out at **192 kHz**,
  which is this backend's widest span.

The wideband-IQ backends (RTL-SDR over USB or rtl_tcp, RX-888, Airspy HF+,
Airspy R2/Mini, HydraSDR RFOne, SDRplay, HackRF, ELAD, SoapySDR, HPSDR, TCI,
SmartSDR, PlutoSDR)
drive the full panadapter, the CW/PSK/RTTY skimmers, and internal demodulation;
a CAT rig feeding demodulated audio shows only a narrow audio-band slice.
RTL-SDR, RX-888, Airspy HF+, Airspy R2/Mini, HydraSDR RFOne, SDRplay and the
ELAD FDM-S are receive-only; the others can transmit — the HackRF half duplex, the PlutoSDR either way (half
duplex by default, full duplex on a board with real Ethernet behind it), the
rest while still receiving.

Whichever backend you pick, a **converter offset** on the same tab handles an
external frequency converter: an HF upconverter (Ham It Up, SpyVerter), a
transverter, or a satellite LNB. Pick one from the list or type an offset in Hz
— the same number and sign the converter's documentation and other SDR programs
use — and tune in real frequencies: with a Ham It Up you work 10.1008 MHz while
the receiver is quietly sent to 135.1008 MHz. The dial, band buttons, memories,
the logbook and every spot and upload stay on the real on-air frequency. A
**Transmit** row beside it says what is in the transmit line, since a converter
is a receive accessory: nothing (the default, so a receive-only accessory cannot
key up 125 MHz away from the dial), the same conversion for a bidirectional
transverter, or an offset of its own — including none at all, which is the
QO-100 station that hears 10 GHz through an LNB and puts 2.4 GHz straight out of
the radio. Not yet verified against physical hardware.

A station with more than one box has a **transverter table** below that: up to
ten rows, each with the band it works (on the dial, in MHz), its own offset, its
own transmit rule, and a **max drive** the operator's Drive setting is held
under — a transverter's I.F. input takes milliwatts, and the drive that is right
for the radio's own bands will destroy it. The dial picks the row; a frequency
no row covers falls through to the single offset above and then to the bare
radio, so HF still works on a station whose only converter is a 2 m transverter.
On an HPSDR board with a filter board on J16 the open-collector band code
follows the dial too, so an accessory board switches its filters, relays and
transverters for the band on the air rather than for the intermediate frequency.

Beside it, **RX range** and **TX range** state which frequencies the radio
covers, in MHz (`144-146, 430-440`). Leave them empty and sdroxide uses what the
device says about itself. Fill them in when the device says nothing — publishing
a tuning range is optional in SoapySDR and plenty of drivers skip it — or when
what it says is the tuner chip's range rather than the radio's. A driver that
publishes no transmit range is taken at its word rather than silenced, so a
transceiver like the SXceiver keys up out of the box; the amateur-band gate
still applies either way.

### SmartSDR Simulator

A **wire-level radio simulator** allows for the backend to be exercised end to end with no radio present.

## Built-in TCI server

sdroxide is also a **TCI server**, so TCI-capable programs — WSJT-X's SunSDR
(TCI) rig type, JTDX, MSHV, skimmers — can use it as their radio: frequency and
mode control, a wideband IQ stream, receive audio to decode, and transmit audio
to put on the air. Several clients can connect at once.

It is on by default at `127.0.0.1:50001` and configured on the **Servers** tab
of the Settings dialog, which also shows the live client count. TCI has no
authentication, so it listens on localhost only unless you change that; the
transmitter has a single owner, and keying up locally always takes it back.
Verified against WSJT-X (rig *TCI Client RX1*, PTT via CAT, TCI audio). See the
user manual, §5.6.

## Built-in Hamlib rigctld server

Most amateur software reaches a radio through **Hamlib**, over the network
protocol its `rigctld` daemon speaks. sdroxide serves that protocol directly, so
**WSJT-X, fldigi, JS8Call, N1MM, Log4OM, GPredict and CQRLOG** can drive it with
no extra daemon, no serial cable and no virtual COM port pair — frequency, mode
and passband, PTT, VFO A/B and split, RIT/XIT, power and volume levels, the
NB/NR/ANF/MUTE functions, and the VFO operations.

It is **off by default** — port 4532 is often already held by a real `rigctld`,
and the protocol has no authentication — and lives on the **Servers** tab next
to the TCI server. In WSJT-X or fldigi choose the rig **Hamlib NET rigctl**
(model 2) and point it at `127.0.0.1:4532`. Unlike TCI it carries control only,
no audio or IQ; both servers can run at once. See the user manual, §5.10.

## Control inputs

Every keyboard shortcut is a rebindable **action**, and the same action list is
reachable from mouse buttons and from a MIDI controller — the cheapest real VFO
knob there is. Configured on the **Controls** tab; see the user manual, §5.9.

Push-to-talk ships **unbound** on purpose. One click binds hold-to-talk to
Space, and a held PTT is released on key-up, on window focus loss, on a text
field taking the keyboard, when the controller is unplugged, and after a
configurable timeout.

Bindings are stored with the *user interface*, not the engine, so a knob plugged
into your laptop works against a remote radio over `--connect` too.

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

## Installing

Every release carries, for Linux:

- an **AppImage** — one file, no install: download
  `sdroxide-<version>-linux-x86_64-compat.AppImage` (or the `aarch64` one for a
  Raspberry Pi or other ARM board), `chmod +x` it, and run it. Built against
  glibc 2.35, so it runs on old and new distributions alike, and it needs no
  SoapySDR: every native driver is compiled in. USB receivers still need the
  udev rule (see below); the rules travel inside the AppImage under
  `usr/share/doc/sdroxide`.
- a **`.deb`**, which installs the udev rules and the menu entry for you, and
  declares its dependencies.
- a **portable tarball**, for anything else.

Windows gets an `.msi`, macOS a `.dmg`. Or build it yourself:

## Building

### Toolchain

Install Rust with [rustup](https://rustup.rs/) rather than your distribution's
`rust`/`cargo` package. The workspace is edition 2024, so it needs Rust 1.85 or
newer, and the browser client needs a second compilation target that only
rustup can add:

```sh
rustup target add wasm32-unknown-unknown
```

A distro-packaged cargo cannot add targets itself — some distros ship the wasm
standard library as a separate package, but the usual symptom is that the native
build works fine and the web client build fails on the missing target. Migrating
to rustup is the shortest way out.

The RADE digital-voice codec and the rtl_433 ISM decoders are vendored as git
submodules, so clone with:

```sh
git clone --recurse-submodules https://github.com/dividebysandwich/sdroxide
# or, in an existing checkout:
git submodule update --init --recursive
```

Both are built from source by `build.rs`, so a default build needs a C compiler
and CMake — already in the package lists below. If you would rather not build
rtl_433, `--no-default-features` leaves it out and SDRoxide falls back to its own
ISM decoders; see [`crates/sdroxide-ism/PROVENANCE.md`](crates/sdroxide-ism/PROVENANCE.md)
for what it is and what licence it carries.

### What depends on what

The native binary and the browser client are two separate builds. Only one
combination couples them, and it couples them at *compile* time:

| You want | Build | Web client needed? |
| --- | --- | --- |
| Native desktop UI | `cargo build --release` | no |
| Native remote client (`--connect`) | `cargo build --release` | no |
| Server, client served from a directory | `cargo build --release`, run with `--web-root` | yes, at run time |
| Server, client baked into the binary | `cargo build --release --features embed-web` | **yes, before you compile** |

`embed-web` embeds `crates/sdroxide-web/dist` with `rust-embed`, so that
directory has to exist *while cargo compiles the server*. It is `.gitignore`d
and therefore absent from a fresh clone, so reaching for `--features embed-web`
first thing fails with:

```
#[derive(RustEmbed)] folder '.../crates/sdroxide-web/dist' does not exist
```

Build the web client first (below) and it compiles. Nothing else in the
workspace depends on the wasm crate — plain `cargo build --release` never
touches it.

You do not need `embed-web` to run a server. Without it `--server` still
serves the WebSocket backend for native `--connect` clients; pass `--web-root`
to serve a Trunk-built directory, or browse to the HTTP port and get a one-line
placeholder saying the client wasn't built.

### System dependencies

A native build needs a C toolchain and a handful of libraries on top of Rust:

```sh
# Debian / Ubuntu
sudo apt install build-essential pkg-config cmake autoconf automake libtool \
                 libclang-dev libasound2-dev libopus-dev
# Arch
sudo pacman -S base-devel pkgconf cmake autoconf automake libtool clang alsa-lib opus
# macOS
brew install pkg-config cmake autoconf automake libtool opus
```

- **ALSA** (`libasound2-dev` / `alsa-lib`) is not optional on Linux: the audio
  device layer and the MIDI control input both link it. macOS and Windows use
  their own system audio APIs.
- **CMake**, a **C compiler**, **libclang** (for `bindgen`) and **autoconf /
  automake / libtool** are for RADE, whose build fetches and compiles a
  FARGAN-enabled Opus from source. That fetch means the *first* build needs
  network access; later builds reuse it. It is also the slow part of a clean
  build: RADE's model weights are ~110 MB of generated C.
- **libopus** is strictly optional, but installing it avoids a CMake 4 problem —
  see below.
- **libfdk-aac** is optional and is a *runtime* dependency, not a build one:
  nothing needs it to compile. It is what decodes **xHE-AAC**, the codec most
  surviving DRM shortwave broadcasters now use, and the DRM window says so when
  it is missing. It cannot be built in — its licence carries restrictions the
  GPL does not permit, which is why Debian ships it in `non-free` — so sdroxide
  looks for it at startup and uses it if it is there. `libfdk-aac2` on
  Debian/Ubuntu, `libfdk-aac` on Arch, `brew install fdk-aac` on macOS, or
  `libfdk-aac-2.dll` beside the executable on Windows. See
  `vendor/fdk-aac/PROVENANCE.md`.

For the **SoapySDR** backend you need its development libraries and the driver
module(s) for your radio (e.g. `soapysdr`, `soapysdr-module-hackrf`,
`soapysdr-module-lms7` on Arch/Debian-style distros). Everything else — including
the RTL-SDR backend — needs no SDR system library at all, so
`cargo build --release --no-default-features` gives a working binary with no
SoapySDR installed.

#### "Compatibility with CMake < 3.5 has been removed" on CMake 4

Two unrelated Opus builds happen during a full build, which makes this error
easy to misattribute:

- **RADE's Opus** — the patched, FARGAN-enabled one that `vendor/rade_c` fetches
  and builds with **autotools**. Every CMake file involved — the vendored ones
  and the wrapper project `crates/sdroxide-rade/build.rs` generates — requires
  3.16 and configures cleanly under CMake 4.
- **The server's Opus** — audio compression for browser and native remote
  clients, via the `opus` → `audiopus_sys` crates. On Unix `audiopus_sys` probes
  `pkg-config` for a system Opus and, if it finds none, compiles its own
  vendored copy **with CMake**. That copy starts with
  `cmake_minimum_required(VERSION 3.1)`.

CMake 4.0 removed support for pre-3.5 minimums, so on a machine with CMake ≥ 4
and no system Opus the build stops with:

```
CMake Error at CMakeLists.txt:1 (cmake_minimum_required):
  Compatibility with CMake < 3.5 has been removed from CMake.
```

The bare `CMakeLists.txt` there is
`~/.cargo/registry/src/*/audiopus_sys-0.2.2/opus/CMakeLists.txt`, not anything
under `vendor/rade_c` — editing the RADE sources or the generated wrapper has no
effect on it, and 0.2.2 is `audiopus_sys`'s newest release, so there is no
version bump to pick up either. Fix it from either end:

```sh
# Install a system Opus and no CMake build happens at all (see the lists above).
sudo apt install libopus-dev pkg-config

# Or configure the vendored copy anyway — this is what the release workflow
# does. CMake 3.x ignores the variable, so it is harmless to leave set.
export CMAKE_POLICY_VERSION_MINIMUM=3.5
```

The two are not quite equivalent: on glibc Linux `audiopus_sys` links a
*system* Opus dynamically, so a binary built with `libopus-dev` present needs
libopus installed wherever it runs, while the vendored route builds it in. Set
`OPUS_STATIC=1` to link the system one statically instead, or `OPUS_LIB_DIR` to
point at a libopus that `pkg-config` cannot see.

### Native binary

```sh
cargo build --release
./target/release/sdroxide --probe        # verify your device is seen
```

### Browser client

The browser client is a separate WebAssembly crate built with
[Trunk](https://github.com/trunk-rs/trunk) 0.21 or newer (CI pins 0.21.14).
Install it with `cargo install --locked trunk`, or drop a prebuilt binary from
its releases page on your `PATH`:

```sh
cd crates/sdroxide-web && trunk build --release
```

Output lands in `crates/sdroxide-web/dist`. Trunk downloads `wasm-bindgen-cli`
and `wasm-opt` itself the first time, so that run needs network access too.

While working on the UI, skip the embed step entirely and point the server at
the directory — a plain `trunk build` (debug) is much faster, and a browser
reload picks up a rebuild:

```sh
cd crates/sdroxide-web && trunk build && cd ../..
./target/release/sdroxide --server --web-root crates/sdroxide-web/dist
```

### Server with the client baked in

Build in this order, then the binary is self-contained and `--server` needs no
`--web-root`:

```sh
(cd crates/sdroxide-web && trunk build --release)   # 1. produces dist/
cargo build --release --features embed-web          # 2. embeds dist/
```

One wrinkle worth knowing: only a **release** build actually bakes the files in.
A debug build with `embed-web` reads them off disk at run time from the path
recorded at compile time, which is why a debug server picks up a rebuilt web
client without recompiling — and why a release binary does not.

### RTL-SDR permissions

The RTL-SDR backend talks to the dongle directly over USB, so the invoking user
needs access to it.

**Linux.** Install the packaged udev rule and replug the dongle:

```sh
sudo cp packaging/linux/60-sdroxide-rtlsdr.rules /usr/lib/udev/rules.d/
sudo udevadm control --reload
```

The `.deb` installs this for you. If your distribution's `rtl-sdr` package is
already installed, its rules cover the same ids and you need not do anything.
The `dvb_usb_rtl28xxu` DVB driver does **not** need blacklisting — sdroxide
detaches it automatically and the kernel rebinds it when the dongle is
unplugged.

**No desktop session?** Every packaged rule grants access two ways: an ACL for
whoever is logged in at the seat (`TAG+="uaccess"`), and `GROUP="plugdev"` for
everyone else. The first grants nothing where there is no seat — **WSL2**, a
headless machine over ssh, a container, or sdroxide running as a systemd
service — and there the second is what makes the device reachable:

```sh
sudo groupadd -f plugdev
sudo usermod -aG plugdev $USER
sudo udevadm control --reload
```

then log out and back in. This applies to every radio below, not just the
RTL-SDR.

The `udevadm control --reload` is not redundant even if you already ran it when
installing the rules: udev resolves `GROUP=` when it *parses* a rule, not when
a device appears, so a `plugdev` created afterwards is invisible to rules
already loaded. Creating the group first and reloading second works too — the
order that does not work is creating it and never reloading.

If the group does not exist you lose nothing but that fallback; the `uaccess`
ACL is on a separate line in every packaged rule precisely so that an unknown
group cannot take it down with it. `plugdev` ships on Debian and Ubuntu but not
on Arch or Fedora, where the ACL alone is normally all you need.

**Windows.** The dongle must be bound to **WinUSB**, which you do once with
[Zadig](https://zadig.akeo.ie/). This is the same step SDR#, gqrx and every
libusb-based program require, so if the dongle already works with any of them
there is nothing to do. Note that Zadig replaces the DVB driver, so the stick
stops working as a TV tuner.

**macOS.** Nothing to do.

If a dongle is present but sdroxide cannot open it, `--probe` says so in words
rather than errnos.

### RX-888 permissions

Same situation as the RTL-SDR — direct USB access — with one wrinkle worth
knowing about.

**Linux.** Install the packaged udev rule and replug the receiver:

```sh
sudo cp packaging/linux/60-sdroxide-rx888.rules /usr/lib/udev/rules.d/
sudo udevadm control --reload
```

The `.deb` installs this for you. The rule covers **two** USB ids, and both are
required: `04b4:00f3` is the bare Cypress FX3 bootloader, which is how the
receiver appears on every plug-in, and `04b4:00f1` is the same device once
sdroxide has uploaded firmware into it. A rule covering only the second looks
right and never works, because the upload happens through the first.

**Windows.** Bind the device to **WinUSB** with [Zadig](https://zadig.akeo.ie/),
once for each of the two ids above.

**macOS.** Nothing to do.

**Getting the full sample rate.** The FX3 bootloader always enumerates at USB
2.0, *even on a perfectly good USB 3 cable and port* — only the firmware
sdroxide uploads re-enumerates at SuperSpeed. So a receiver reported as "USB
2.0" before it is programmed is not a problem. If it is still USB 2.0
afterwards, that is a real cable or port problem, and sdroxide clamps the sample
rate and says so on screen rather than silently dropping samples. `--probe`
reports the link speed.

### Airspy HF+ permissions

Same situation as the RTL-SDR — direct USB access, no vendor package.

**Linux.** Install the packaged udev rule and replug the receiver:

```sh
sudo cp packaging/linux/60-sdroxide-airspyhf.rules /usr/lib/udev/rules.d/
sudo udevadm control --reload
```

The `.deb` installs this for you. If Airspy's own `52-airspyhf.rules` is already
installed it covers the same id and there is nothing to do. The rule also covers
`03eb:6124`, the SAM-BA bootloader — sdroxide cannot flash firmware and never
enters it, but a receiver left there by an interrupted vendor update is
otherwise invisible to a non-root user, and "the device disappeared" is a worse
thing to debug than a stray rule line.

**Windows.** Bind the device to **WinUSB** with [Zadig](https://zadig.akeo.ie/),
or install Airspy's own package, which does the same thing. If the receiver
already works with SDR# or SDR++ there is nothing to do.

**macOS.** Nothing to do.

All three models — Dual, Discovery and Ranger — share the id `03eb:800c`, so a
device list cannot say which one is plugged in. sdroxide asks the receiver once
it is open, and `--probe` lists what is on the bus.

### Airspy R2 / Mini permissions

Same situation as the Airspy HF+ — direct USB access, no vendor package — but a
**separate rule file**, because it is a separate receiver with its own USB id.
Both may be installed together and neither covers the other.

```sh
sudo cp packaging/linux/60-sdroxide-airspy.rules /usr/lib/udev/rules.d/
sudo udevadm control --reload
```

The `.deb` installs this for you. If Airspy's own `52-airspy.rules` is already
installed it covers the same id and there is nothing to do.

**Windows.** Bind the device to WinUSB with [Zadig](https://zadig.akeo.ie/), or
install Airspy's own package, which does the same thing.

**macOS.** Nothing to do.

Both models share the id `1d50:60a1` *and* the same USB product string, so a
device list cannot say whether an R2 or a Mini is plugged in — the only thing
that separates them is the set of sample rates they offer, which sdroxide reads
once the receiver is open.

### HydraSDR RFOne permissions

Same situation again — direct USB access, no vendor package.

**Linux.** Install the packaged udev rule and replug the receiver:

```sh
sudo cp packaging/linux/60-sdroxide-hydrasdr.rules /usr/lib/udev/rules.d/
sudo udevadm control --reload
```

The `.deb` installs this for you. If HydraSDR's own `51-hydrasdr.rules` is
already installed it covers the same ids and there is nothing to do.

The rule covers **both** ids an RFOne can appear on: `38af:0001` for production
boards and `1d50:60a1` for the prototypes, which share the Airspy R2 and Mini's
pair. The second line duplicates `60-sdroxide-airspy.rules`, deliberately — two
udev rules granting the same access to the same device is a no-op, and it means
installing either file alone is enough.

**Windows.** Bind the device to WinUSB with [Zadig](https://zadig.akeo.ie/), or
install HydraSDR's own package, which does the same thing.

**macOS.** Nothing to do.

### HackRF permissions

Same situation again — direct USB access, no vendor package.

**Linux.** Install the packaged udev rule and replug the radio:

```sh
sudo cp packaging/linux/60-sdroxide-hackrf.rules /usr/lib/udev/rules.d/
sudo udevadm control --reload
```

The `.deb` installs this for you. If Great Scott Gadgets' own `53-hackrf.rules`
is already installed it covers the same ids and there is nothing to do. The rule
covers the HackRF One *and Pro* (`1d50:6089` — the two share an id), the
Jawbreaker (`604b`) and the rad1o (`cc15`), plus `1fc9:000c` — the DFU
bootloader. sdroxide cannot flash firmware and never enters DFU, but a radio
left there by an interrupted `hackrf_spiflash` is otherwise invisible to a
non-root user.

**Windows.** A HackRF carries the Microsoft OS descriptors that ask Windows to
bind WinUSB by itself, so this normally needs nothing. A radio that has been
through [Zadig](https://zadig.akeo.ie/) for something else may need Zadig again
to put it back on WinUSB.

**macOS.** Nothing to do.

### ELAD permissions

Same situation again — direct USB access, no vendor package.

**Linux.** Install the packaged udev rule and replug the device:

```sh
sudo cp packaging/linux/60-sdroxide-elad.rules /usr/lib/udev/rules.d/
sudo udevadm control --reload
```

The `.deb` installs this for you. The rule covers the FDM-DUO (`1721:061a`), the
FDM-S2 (`061c`) and the FDM-S1 (`0610`) — the *receive* interface only. An
FDM-DUO's other two USB ports need nothing installed: its CAT port is an FTDI
bridge the in-tree `ftdi_sio` driver already handles (it appears as
`/dev/ttyUSB*`, and access to that is the `dialout` group, not this file), and
its audio port is an ordinary USB audio device.

**Windows.** ELAD's own driver package binds the receive interface to their
Cypress driver, which only their software can use. Bind it to WinUSB with
[Zadig](https://zadig.akeo.ie/) instead — note that this stops FDM-SW2 seeing
the device until the driver is put back.

**macOS.** Nothing to do.

The board *revision* is not in the USB descriptors — a rev-9 HackRF One and an
older one both report `1d50:6089` — so the device list names the family and
sdroxide asks the radio once it is open. The product id does separate a HackRF
One from a Jawbreaker or a rad1o, which matters because the three do not tune
the same range. It does **not** separate a HackRF One from a HackRF Pro: Great
Scott Gadgets ship one device descriptor for both, and only the USB *product
string* differs. sdroxide reads that string during enumeration so the settings
dialog can offer the Pro's extra low sample rates without opening anything, and
confirms the board from its board-id register once the radio is open.

### LimeSDR and LimeRFE permissions

Not quite the same situation as the rest, because sdroxide does not open the
LimeSDR itself — LimeSuite does, and it ships its own rules.

**Linux, the LimeSDR.** Nothing from this project. Installing LimeSuite installs
`64-limesuite.rules`, which covers the LimeSDR-USB (`1d50:6108`), the Mini
(`0403:601f`) and the Cypress FX3 bootloader states. If `LimeUtil --find` sees
your board, so will sdroxide.

**Linux, the LimeRFE.** This one sdroxide *does* open — its own micro-USB port
is a serial device — so install the packaged rule and replug it:

```sh
sudo cp packaging/linux/60-sdroxide-limerfe.rules /usr/lib/udev/rules.d/
sudo udevadm control --reload
```

The `.deb` installs this for you. Worth knowing what it does and does not do:
the LimeRFE's USB-serial bridge presents a *generic FTDI* id shared with a great
many unrelated adapters, so the rule cannot identify a LimeRFE and does not try.
It grants access to those ports, you pick the right one in Settings → Radio, and
the board's own handshake is what confirms a LimeRFE is on the other end. The
side effect is that any other FTDI serial adapter on the machine gets the same
loosened permissions — the same trade every distribution's own FTDI rules make.
If you would rather not, add yourself to the `dialout` group instead and skip
this file.

**Windows.** The PothosSDR bundle installs LimeSuite and its drivers. The
LimeRFE appears as an ordinary COM port through FTDI's driver.

**macOS.** `brew install limesuite`. Nothing else to do.

**If sdroxide finds no board but `LimeUtil --find` does**, the likely cause is
that the library found at runtime is not the one you think: the interface logs
its version at startup (`LimeSuite loaded, version …`). If sdroxide lists a board
you do not recognise, note that LimeSuite claims the bare Cypress FX3 id that an
*unprogrammed RX-888* also presents — sdroxide filters those out by board name
and `--probe` names what it skipped.

### T/R switch permissions

Only two of the six kinds of switching hardware need anything here.

**USB HID relay boards and CM108 sound-card GPIO.** sdroxide writes HID reports
to these, and no distribution grants that by default — a card that plainly works
for audio still cannot switch anything until the rule is in:

```sh
sudo cp packaging/linux/60-sdroxide-relay.rules /usr/lib/udev/rules.d/
sudo udevadm control --reload
```

The `.deb` installs it for you. As with the LimeRFE rule, it is worth knowing
what it costs: the dcttech relay boards carry `16c0:05df`, a *V-USB hobby id*
shared with other people's home-made keyboards and LED controllers, so the rule
loosens permissions on any such device on the machine. sdroxide's own device list
additionally filters on the product string (`USBRelay…`), so it does not offer
you somebody's keyboard as an antenna relay — but udev cannot make that
distinction.

**Serial relay boards** (LCUS, KMtronic, Numato) and **RTS/DTR lines** need
nothing from this file: they are serial ports, so add yourself to `dialout` as
you would for a CAT cable.

To check what a machine can see, and to throw one contact with the transmitter
cold:

```sh
cargo run -p sdroxide-relay --example relay -- --list --all
cargo run -p sdroxide-relay --example relay -- --serial /dev/ttyUSB0 --board lcus --set 1 on
```

**Windows and macOS.** No driver and no permissions to set — but note that the
HID paths on both have never been run against hardware, only compiled. If a
board is not found or does not switch on either, the `--list --all` transcript
above is what to attach to a bug report.

### SDRplay RSP prerequisites

SDR Oxide does not interface with the USB device itself. It talks to the [SDRplay API](https://www.sdrplay.com/api/)
(v3.x) — a userland library plus a background service that owns the hardware,
and whose installer sets up its own USB permissions. Install it, make sure the
service is running (Linux: `sudo systemctl enable --now sdrplay`; the Windows
and macOS installers start it themselves), and the RSP appears under Rescan in
**Settings → Radio → SDRplay RSP (USB)**. If it doesn't, `sdroxide --probe`
says which piece is missing — the library, the service, or the device.

## Running

```sh
# Native desktop, tuned to 20 m, FT8:
sdroxide --freq 14074000 --mode ft8

# Server: DSP + hardware here, UI in a browser at http://<host>:4950
# (needs a web client: either an embed-web build, or --web-root as below)
sdroxide --server

# Server serving a Trunk-built client from disk instead of an embedded one:
sdroxide --server --web-root crates/sdroxide-web/dist

# Desktop UI driven by a remote server (no web client involved):
sdroxide --connect 192.168.1.10:4950

# ...just one of that server's radios, instead of all of them in tabs
# (ids: curl http://192.168.1.10:4950/radios):
sdroxide --connect 192.168.1.10:4950/ws/1
```

**Raspberry Pi 4/5 (and 400/500).** Mesa's Vulkan driver for the Pi's V3D GPU
(V3DV) makes the display flicker, even in other
wgpu and Vulkan applications. sdroxide detects that adapter at startup and
renders through OpenGL ES instead, which is steady. That costs roughly one core
of the four (237% CPU against 140% on an RTL-SDR at 2.4 Msps), so if your
desktop does not flicker under Vulkan, `WGPU_BACKEND=vulkan sdroxide` takes it
back. `WGPU_BACKEND` is honoured on every machine, and pins the renderer either
way.

## Startup parameters

| Flag | Description |
| --- | --- |
| `--device <ARGS>` | SoapySDR device args (e.g. `driver=hackrf`). Default: config, then first device found. |
| `--probe` | List devices and their probed capabilities, then exit. |
| `--console` | Terminal (ASCII) waterfall mode, no GUI. |
| `--siggen` | Use the built-in signal generator instead of hardware. |
| `--file <FILE>` | Play a raw interleaved CF32 IQ file instead of hardware. |
| `--freq <HZ>` | Center frequency in Hz (default: where the last session was left; `14200000` on a first run). |
| `--rate <HZ>` | Sample rate in Hz (default: from config). |
| `--gain <DB>` | Overall RX gain in dB (default: hardware AGC / moderate). |
| `--mode <MODE>` | Initial mode: `USB LSB CW AM SAM NFM WFM DIGU DIGL DSB ISB SPEC FT8 FT4 FT2 PSK RTTY OLIVIA THOR FSQ HELL SSTV RIFP WEFAX RFPAINT RADE ADS-B VDL2`. Default: the mode the last session was left in. |
| `--antenna <NAME>` | RX antenna port, as the device names it (`LNAH`, `TX/RX`; see `--probe`). Default: the port the last session was left on. |
| `--tx-antenna <NAME>` | TX antenna port, likewise (`BAND1`, `BAND2`). |
| `--server` | Run as a server: HTTP web client + WebSocket streaming backend. |
| `--connect <HOST[:PORT]>` | Connect as a native remote client to a running server. |
| `--port <PORT>` | Server port (default: from config, `4950`). |
| `--web-root <DIR>` | Directory with the Trunk-built web client, e.g. `crates/sdroxide-web/dist` (default: embedded assets with `--features embed-web`). |
| `--fft <N>` | Spectrum FFT size (default `4096`). |
| `--tx-tune <SECS>` | Headless TX smoke test: key a tune carrier at minimal drive, then exit. |
| `--ft8-cq <SECS>` | Headless FT8 smoke test: call CQ at minimal power, then exit. |
| `--rade-rx <SECS>` | Headless RADE smoke test: receive for SECS seconds and report whether the modem synced. Pair with `--file`. |
| `--oob-tx` | Lift the amateur-band transmit lockout for this run, for licensed out-of-band use. Shows a warning that must be dismissed by hand; never persisted, so it has to be passed again every launch. |
| console extras | `--fps <N>` lines/sec, `--width <CHARS>`, `--db-floor <dBFS>`, `--db-ceil <dBFS>`. |

## Keyboard shortcuts

Active whenever a text field isn't focused. These are the **defaults** — all of
them, plus PTT, band, mode, filter and much else, are rebindable on the
**Controls** tab.

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
| Left-click | Tune the active VFO to that frequency. In FT8/FT4/FT2, sets the TX audio offset instead. |
| **Shift** + left-click | Place the second receiver: the sub-receiver when SUB is on, VFO B otherwise. Works over a spot box and in FT8/FT4/FT2 too. |
| Drag inside the sub-receiver's passband | Tune the sub-receiver (violet, when SUB is on) instead of panning. |
| Left-drag | Grab and slide the spectrum — pans the view and tunes along with it. |
| Right-drag | Pan the view only (no tuning). |
| Scroll wheel | Zoom in/out around the cursor. |
| Drag a passband edge | Move that filter edge (works on the spectrum and the waterfall). |
| Drag the frequency-scale strip | Resize the spectrum vs. waterfall split. |
| Drag the waterfall / FT8 panel divider | Resize the FT8/FT4/FT2 operating panel. |

**Frequency readout** — scroll the wheel over a digit to step that digit; click
its upper half to increment, lower half to decrement.

**FT8/FT4/FT2 decode list** — click a row to move your TX audio onto that signal
(and preview it on the map); press **REPLY** to start an auto-sequenced QSO.


## Contributing, LLM Usage, Licensing

Both local and hosted LLMs (usually advertised as "Generative AI") were used in 
the development of this software. Contributions written using LLMs are ok 
provided the following rules are observed:

* **Read and review** generated code. You should be able to answer questions 
about your contribution.
* **Document and comment** non-trivial parts of the code.
* **Test** your contribution using real radio equipment. If this is not possible,
consider if this is a useful contribution and disclose the need for testing help
before you start.
* Don't use LLMs for trivial things like changing a constant. This is slow,  wasteful
and runs the risk of unneccessary modifications elsewhere.
* Use modern, sufficiently sized models with sufficient context size. Running 
small or outdated models or limiting them to small contexts results in low 
quality code and damage to existing functionality.
* Usage of locally-hosted LLMs is encouraged, but not required.
* Please keep commits vendor-neutral and don't commit specific files for 
one specific cloud hosted LLM.
* Observe the project license. This is a GPLv3 project. Changing the license 
would violate the terms of several of the used libraries.

One part goes further than GPLv3, and it is worth knowing about before you
deploy rather than after. CW decoding uses the
[DeepCW](https://github.com/e04/deepcw-engine) model, which is **AGPL-3.0-only**
and is linked into the binary rather than read as a data file — so its terms
cover the built program as a whole. The practical difference is AGPL section 13:
**running `sdroxide --server` and letting other people use that instance over a
network counts as conveying it to them, so they have to be offered the
Corresponding Source.** Using sdroxide on your own machine changes nothing. The
model is confined to the `sdroxide-deepcw` crate, and the wasm web client links
none of it.

