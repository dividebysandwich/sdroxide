# SDR Oxide Brown

> **Windows download** — [**installer (`.msi`)**](https://github.com/madmedicnl/sdroxide-brown/releases/latest/download/sdroxide-windows-x86_64.msi)
> · [**portable `.zip`** (contains `sdroxide.exe`)](https://github.com/madmedicnl/sdroxide-brown/releases/latest/download/sdroxide-windows-x86_64.zip)
> · [every platform and build](https://github.com/madmedicnl/sdroxide-brown/releases/latest)
>
> Linux (AppImage · `.deb` · tarball) and macOS (`.dmg`) are on the same
> [Releases page](https://github.com/madmedicnl/sdroxide-brown/releases/latest).

> **A fork of [sdroxide](https://github.com/dividebysandwich/sdroxide), and a
> complete copy of it.** Every ham feature is here — the whole transceiver, all
> the digital modes, the logbook, awards, rig control. The fork changes the
> **focus**: it is tuned for the **11 m citizens' band (CB)** and the
> **shortwave listener (SWL)**.

## The whole station in one program

Most SDR software is a receiver, a radio panel, or a decoder. This is the **whole
station** — receiver, transmitter, every digital mode, the logbook, and the ADS-B,
AIS, APRS, HFDL and DAB decoders you would otherwise install five programs for.

**And two things nobody else does.** The **11 m citizens' band is a real two-way
band**, voice and digital modes with WSJT-CB interoperability, so you can answer a
CB station in FT8 like any other. And it is **built for listening** — three
broadcast schedules in one store, a **LISTEN** window that draws the path to a
station with the share of it **in darkness**, and a reception log that tracks QSL.

Same binary, two jobs: a desktop app, or a server you reach from any browser,
phone, or a headless Raspberry Pi at a remote site.

![The main window: radio, receiver, display and system controls across the top, waterfall with its level slider](docs/images/main-ft8-cb.jpg)

*Voice and the digital modes on 11 m, from one window.*

## Read the manual first

This README is a quick orientation, not the documentation. **The
[User Manual](docs/USER_MANUAL.md) is the real guide** — every window, every
option, every backend and permission detail. It is the fastest way to find out
what this program can actually do; dip into the parts you need and ignore the
rest.

- **[User Manual](docs/USER_MANUAL.md)** — the complete guide.
- **[CB quick-start](docs/cb-quickstart.en.md)** ([Nederlands](docs/cb-quickstart.nl.md) · [Français](docs/cb-quickstart.fr.md) · [Italiano](docs/cb-quickstart.it.md)).
- **[FT8 on 11 m quick-start](docs/ft8-11m-quickstart.en.md)** ([Nederlands](docs/ft8-11m-quickstart.nl.md) · [Français](docs/ft8-11m-quickstart.fr.md) · [Italiano](docs/ft8-11m-quickstart.it.md)).
- **[SSTV on 11 m quick-start](docs/sstv-11m-quickstart.en.md)** ([Nederlands](docs/sstv-11m-quickstart.nl.md) · [Français](docs/sstv-11m-quickstart.fr.md) · [Italiano](docs/sstv-11m-quickstart.it.md)).
- **[Listening quick-start](docs/listening-quickstart.en.md)** ([Nederlands](docs/listening-quickstart.nl.md) · [Français](docs/listening-quickstart.fr.md) · [Italiano](docs/listening-quickstart.it.md)).
- **[Pi Zero 2 W headless SWL station](docs/pi-zero-2w-swl.en.md)** ([Nederlands](docs/pi-zero-2w-swl.nl.md) · [Français](docs/pi-zero-2w-swl.fr.md) · [Italiano](docs/pi-zero-2w-swl.it.md)).
- **[QO-100 Quick-start Guide](docs/qo100-quickstart.en.md)** ([Türkçe](docs/qo100-quickstart.tr.md)).
- **[ROADMAP.md](ROADMAP.md)** — where the listener work goes next.

## Quick start

1. **Get it.** Download for your platform from the
   [Releases page](https://github.com/madmedicnl/sdroxide-brown/releases/latest)
   (Linux AppImage / `.deb` / tarball, Windows `.msi` / `.zip`, macOS `.dmg`),
   or [build it yourself](#building).
2. **Point it at a radio.** Plug in an SDR, or use a network radio
   (SpyServer, KiwiSDR, an OpenHPSDR, a CAT rig, …). `sdroxide --probe` lists
   what it can see and says in words what is missing. Linux USB receivers need
   the packaged udev rules (the `.deb` installs them); Windows USB receivers
   usually need [Zadig](https://zadig.akeo.ie/) to bind WinUSB. See
   [Radio-specific notes](docs/USER_MANUAL.md#15-radio-specific-notes).
3. **Run it.**
   ```sh
   sdroxide                       # native desktop
   sdroxide --server              # UI in a browser at http://<host>:4950
   sdroxide --connect host:4950   # desktop UI driving a remote server
   ```
4. **Tune.** The band/mode menu leads with a **Primary modes** row
   (**AM · FM · USB · LSB**) and an **HF / VHF / UHF / ALL** band row, and lists
   the broadcast bands (LW / MW / SW / FM) and the CB band by name. The
   **LISTEN** tab offers every decoder on every band; **OPERATE** enforces the
   band/mode rule. A band comes up on its own memory of mode, filter and
   frequency, so switching back and forth is one click.
5. **On CB.** Pick the **11 m** band: the channelised dial reads `CH nn`, the
   per-country channel plans are on the General tab, and the WSJT-CB digital
   exchange (with country flags) works like FT8 does. CB is a two-way band:
   switch on **Allow transmit on 11 m (CB)** once and it transmits and receives
   like any other band. To send the digital modes you need a transceiver, keyed
   by **VOX** or **CAT** — a receive-only dongle can hear but not send.
6. **Listening (SWL).** Turn on **SWL mode** for that radio (Settings → Radio →
   Transmit controls, or start every radio with `--swl`) and its transmit
   controls disappear. Then: browse the **SCHEDULE**
   window of ~4,600 broadcasts and tune or log a station; watch broadcast
   carriers labelled on the waterfall; keep the separate **SWL log** with
   **SINPO/SIO** and send a **reception report**; replay the last two minutes
   with **REPLAY**; record a band on a timer; or scan 49 m and stop on carriers.
7. **Everything else** — FT8/FT4/FT2, JT65/JT9, FST4, MSK144, FSK441, Q65,
   UVPACKET, JTTY (experimental), WSPR, PSK/RTTY, Olivia, SSTV, RIFP, weather
   fax, DRM, HD Radio, ADS-B/VDL2/ACARS/HFDL, the logbook, awards, QSL upload,
   MIDI control — is in the **[User Manual](docs/USER_MANUAL.md)**.

<details>
<summary><h2>How this fork differs from upstream</h2></summary>

Upstream is an amateur transceiver. This build runs **the CB band used to the
full — voice *and* the digital modes — plus the listener's tools**.

| | Upstream (`dividebysandwich/sdroxide`) | This fork |
| --- | --- | --- |
| **Focus** | amateur (ham) transceiver | **CB used in full (voice and digital), SWL and decoding**; 11 m transmit is a one-time opt-in only because the ham lockout is generic |
| **Amateur bands** | 160 m … 3 cm, by IARU region, with band-plan lockout | identical, untouched |
| **11 m / citizens' band** | the band itself (26.965–27.860 MHz) and its digimode conventions | **per-country channel plans** (`WORLD EU DE UK US AU`), **CB country flags**, a one-time transmit opt-in, opt-in **WSJT-CB spot-server spotting**, and an **experimental** wider callsign grammar for the community's newer identifiers, off by default |
| **LOG11DX logbook** | — | uploads each logged QSO straight to the 11 m [LOG11DX](https://log11dx.com/) logbook — no separate bridge program, which its own WSJT-X integration otherwise needs |
| **Broadcast & utility bands** | general coverage only | **LW / MW / SW / FM**, the VHF civil **AIR**band (108–137 MHz, AM) and the **MIL**itary UHF airband (225–400 MHz, AM) on the selector and in the band plan; on shortwave the **metre band** is named ("SW 49m · AM") and offered as a shortcut |
| **Broadcast schedule** | EiBi transmitters labelled on the waterfall | plus a **SCHEDULE** window that filters them by time, band, language and target and tunes or logs a station; utilities (time signals, VOLMET) labelled and stations starred |
| **Listening log** | QSO logbook | a separate **SWL log** — station, frequency, UTC, **SINPO/SIO**, S-meter, notes — with a **reception report** signed with the listener's own **SWL number** (a **Report as (SWL)** box on the Spots tab, separate from the transmitting callsign: it also signs the PSK Reporter/WSPR uploads, but is never keyed, logged or spotted) |
| **C-QUAM AM stereo** | — | decoded on MW, with a stereo lamp and a mono blend (not yet verified against a real signal) |
| **ACARS** | — | the VHF airband airline datalink (131.550, 131.725 MHz and friends), with a message panel; the demodulator has carrier and bit-clock recovery and is checked against an off-air recording |
| **HFDL** | — | the shortwave aircraft datalink (ARINC 635): ground stations across 2.8–22 MHz talking to aircraft over the ocean, with a decode log and an **aircraft map** drawn from the positions the aircraft downlink |
| **DSC** | — | the marine Digital Selective Calling system on VHF channel 70 and the MF/HF distress channels (2187.5, 4207.5, 8414.5 kHz …): 1200-baud FFSK carrying distress alerts (MMSI, nature, position, time) and routine calls, with a message panel and a raw-symbol view; receive only |
| **MW/SW DX tools** | SAM and the ham audio chain | **ECSS-U / ECSS-L** presets on SAM and a receive **tone** control |
| **Listening tools** | — | two-minute time-shift **replay**, **scheduled recordings**, band scanning that names what it stops on |
| **Signal ID guide** | — | a **SIG ID** window in the SWL LOG window names what is on the dial — ranked against the mode, frequency, band and passband from a built-in catalogue of ~60 amateur, broadcast, marine, aviation, utility and satellite signals, with a free-text search and a **sigidwiki** link for the sample |
| **Morse trainer** | — | a **TRAINER** window on the CW panel: text to Morse and back, paced playback with **Farnsworth** spacing, a **Koch drill** that unlocks a character at a time and remembers the score, and a **SEND** pane that reads a real USB paddle (the iambic timing is made in software) |
| **CW key** | the keyboard straight key | **Settings → CW**: the key is the keyboard's straight-key binding or a **USB paddle**, typed **Straight** / **Iambic A** / **Iambic B**, with a **REVERSE** switch for the paddle. With **KEY THE TRANSMITTER** it keys the radio through the ordinary manual-key path — the same band lockout, watchdog and read-back — iambic and straight; off, it drives only the trainer. Linux desktop for the USB reader |
| **SWL mode** | — | hides every transmit control and swaps the ham chips (awards) for the listener's — **per radio**, so a listening set and a transceiver can sit side by side; the SPOTS window keeps the receive-only networks and drops only the ham feeds. A receive-only radio (a public SDR, an RTL-SDR) is offered **Listening controls** in its warning banner. Start in SWL mode from Settings → UI, or **`--swl`** |
| **Per-radio identity** | one station callsign | a **callsign per radio** (Settings → Radio), falling back to the station callsign on the General tab — a CB callsign on the 11 m set and an amateur callsign on the HF rig at the same time |
| **Simple interface** | — | hides the advanced chips |
| **Band/mode menu** | one long list, no band/mode rule | **LISTEN / OPERATE** tabs, a **Primary modes** row above the full list, and an **HF / VHF / UHF / ALL** band row. OPERATE greys out (and the engine refuses) a mode that does not apply on the band — AM on the FM broadcast band, WFM on 11 m; LISTEN offers **every mode on every band**. On desktop it can be **docked** beside the waterfall (undock/hide from the panel, toggle from the band chip) |
| **Propagation columns** | propagation heat map | measured **WSPR** and **PSK Reporter** activity in the **BANDS** window |
| **Weak-signal decoding** | FT8 decode on a single pass | FT8 runs WSJT-X's **checkpointed signal subtraction**, so weak signals buried inside stronger neighbours decode — on the WSJT-X busy-slot sample **12 → 22 decodes**, none lost, at no cost to the single-signal sensitivity floor. The whole FT8 path, so it helps on 11 m where a busy channel puts stations on top of one another |
| **Waterfall levels** | a popup behind a chip | a vertical level slider beside the waterfall, plus the popup |

Upstream has merged most of this fork's general-purpose work since it was
offered, so several rows that used to be differences are not any more and have
been dropped from the table: **HD Radio (NRSC-5)**, **station profiles**, the
**CW straight key**, **audible alerts**, the ten editor **themes**, the **USB
sound-card** backend, the 11 m band and its digimode conventions, **EiBi**
broadcast labelling, **AIS**, decode **CSV/ADIF export**, browser **ADIF/CHIRP
import** and the step-row **snap** all live in upstream now; so, since the
2026-09-25 merge, do the **grey-line night shading**, the **meteor-shower
calendar**, the **IBP beacons**, the **Kp history trend**, the weak-signal
modes **MSK144 / JT65 / JT9 / FST4 / Q65 / FSK441**, the wide **VDL2 window
rate**, and the **FT4/FT2/JS8 successive-interference cancellation**. What the
table lists is what this fork still adds on top — plus, of those, **FSK441
transmit**, DSC and UVPacket, which upstream has not taken. The CW work is
offered upstream as [#568](https://github.com/dividebysandwich/sdroxide/pull/568)
(the Morse trainer), [#569](https://github.com/dividebysandwich/sdroxide/pull/569)
(the iambic/straight keyer) and
[#572](https://github.com/dividebysandwich/sdroxide/pull/572)
(the no-control-link keying fix), with the whole key as
[#573](https://github.com/dividebysandwich/sdroxide/pull/573).

</details>

## A look inside

The band and mode menu has two sides. **OPERATE** is the ham view, where a mode
the band does not carry is greyed out; **LISTEN** offers every mode on every
band, for exploring the dial.

| OPERATE | LISTEN |
| --- | --- |
| ![The band and mode menu, OPERATE tab](docs/images/band-menu-operate.jpg) | ![The band and mode menu, LISTEN tab](docs/images/band-menu-listen.jpg) |

On 11 m the WSJT-CB exchange runs like FT8 does, with country flags on the
decode, and the propagation globe behind it:

![11 m CB with the 3-D propagation globe](docs/images/cb-3d-globe.jpg)

For the listener, **SWL mode** hides every transmit control and swaps the ham
chips for the listener's. The **SCHEDULE** window tunes or logs a broadcast
station, the reception log keeps the listening record, and the globe works the
same on a receive-only set:

![SWL mode with the broadcast schedule](docs/images/swl-schedule.jpg)

![The reception log](docs/images/swl-log.jpg)

![SWL with the 3-D propagation globe](docs/images/swl-3d-globe.jpg)

<details>
<summary><h2>More screenshots</h2></summary>

![The reception-log entry form](docs/images/swl-log-entry.jpg)

![Settings → Radio: per-radio identity and the SWL switch](docs/images/settings-radio.jpg)

![The CW panel](docs/images/cw-panel.jpg)

![Station profiles](docs/images/station-profiles.jpg)

![Settings → UI](docs/images/ui-menu.jpg)

![Settings → Uploads](docs/images/settings-uploads.jpg)

</details>

More still — the panadapter, logbook, awards, ADS-B, APRS and the browser
client — are in [`docs/images/`](docs/images).

<details>
<summary><h2>What it does — the full feature list</h2></summary>

- **Radios** — CAT/audio, CAT/stereo I/Q, TCI (SunSDR), OpenHPSDR P1/P2
  (Hermes Lite 2, Apache Labs), SoapySDR, and native drivers for RTL-SDR,
  RX-888, SDRplay RSP, Airspy HF+/R2/Mini, HydraSDR RFOne, HackRF, PlutoSDR,
  LimeSDR + LimeRFE, ELAD FDM, RigExpert Fobos, plus Icom LAN, FlexRadio
  (SmartSDR) and a **USB sound-card** backend. Setup and permissions per model:
  [Radio-specific notes](docs/USER_MANUAL.md#15-radio-specific-notes).
- **Panadapter** — GPU waterfall + spectrum, wheel-zoom on the cursor,
  drag-to-pan, per-digit readout, colormaps, peak-hold and auto-contrast.
- **Modes** — SSB, CW, AM, SAM, **C-QUAM** AM stereo, NFM (CTCSS/DCS),
  WFM (stereo + **RDS/RBDS**), DSB, **ISB**, DIGU/DIGL, SPEC, **DRM**,
  **HD Radio** (FM, stereo), and the receive-only utility decoders **ADS-B**,
  **VDL2**, **ACARS**, **HFDL**, **NAVTEX**, **DSC**, **weather fax**.
- **Digital modes** — **FT8/FT4/FT2**, **JT65/JT9**, **FST4**, **MSK144**, **FSK441**, **Q65**, **UVPACKET**, **JTTY** (experimental), **JS8**, **WSPR**, **PSK31/RTTY**,
  **Olivia/THOR/DominoEX/FSQ**, **Hellschreiber**, **SSTV**, **RIFP**, **RF Paint**,
  **RADE** digital voice, **packet/APRS**, **AtCHAT NET**, **Winlink** email.
  Details and setup are in the [manual's digital-modes chapter](docs/USER_MANUAL.md#3-digital-modes).
- **Receiver** — hang AGC, draggable filter edges, noise blanker, auto-notch,
  five noise-reduction engines, squelch, a sub-receiver, RIT/XIT, VFO A/B with
  split, band stacks and memories.
- **Spots, awards, QSL** — DX cluster / POTA / SOTA / PSK Reporter spots as
  clickable panadapter markers, callsign lookup, one-click upload to
  LoTW/eQSL/Club Log/QRZ/HamQTH, and DXCC/WAS/WAZ/grid tracking. The fork adds
  a **SCHEDULE** window over the broadcast-station labels — filter by time,
  band, language and target, then tune or log a station.
- **Control** — every shortcut rebindable, any class-compliant **MIDI** controller
  (jog wheel, pads, faders, LEDs), mouse-button bindings, and optional **spoken
  announcements** through a bundled local neural voice (plus NVDA/Orca/VoiceOver).
- **Contest logger** — a mode-agnostic single-operator logger with the common
  contests built in, a dupe warning, a live score and rate, and **Cabrillo**
  export. On FT8 an EU VHF or CQ WPX run logs itself.
- **T/R switch** — drives an external relay that grounds the antenna while
  transmitting and sequences an amplifier with it; several USB/serial/GPIO
  relay kinds supported. A contact can take a **band-decoder** role instead,
  switching an outboard filter or transverter by the dial's band from a
  per-band RX/TX table. See "T/R switch" in the manual for the limits.
- **Antenna rotator** — points a motorized beam. The satellite lock steers it
  through a Hamlib `rotctld` daemon (or an EasyComm II / GS-232 controller on a
  serial port directly), and the **ROTATOR** window is the manual side: a
  compass showing the hardware's own bearing against the one you asked for,
  drag or click to point, a DX-country lookup, and STOP / PARK / AUTO. Point the
  beam from a **right-click** on the map or a decode row's **BEAM** chip.
- **Classic UI** — the SDR++-style arrangement, if you want it: every control in
  one column down the left, each section folding away, **drag the headings to
  order them** and the picture takes everything else. Off by default; Settings →
  UI ([manual §6.3](docs/USER_MANUAL.md#63-ui-display-preferences-and-voice-announcements)).
- **Persistence** — device, rates, gains, memories, band stacks, network/QSL
  credentials, control bindings and the logbook under `~/.config/sdroxide-brown/`,
  plus named **station profiles**.

</details>
## Installing

Every release carries, for Linux:

- an **AppImage** — one file, no install: download
  `sdroxide-<version>-linux-x86_64.AppImage`, `chmod +x` it and run it. Built
  with every native driver compiled in.
- a **.deb**, which installs the udev rules and the menu entry for you.
- a **portable tarball**, for anything else.

**Raspberry Pi, ARM boards and older distributions are not published as
prebuilt binaries** (from 1.9.20 — see the release notes). Building from source
below works on all of them and is the supported route there.

Windows gets an `.msi` and a portable `.zip`, macOS a `.dmg`. Or build it
yourself.

**Installing a club's worth of machines?** [`packaging/headless/`](packaging/headless/)
has a script — bash and Python, Linux/macOS/Windows — that installs the package
and a **station's configuration** with no questions asked: set one machine up
properly, copy its config directory in beside the script, and every other machine
is one command.

**On an Apple Silicon Mac, one thing first.** The tree pins its toolchain in
`rust-toolchain.toml`, and you should let it: `rustup toolchain install 1.98.1`.
A newer stable currently **hangs** on this tree rather than failing — rustc sits
in codegen on `mfsk-core` at 100 % of one core and never finishes. It is an LLVM
bug on aarch64 that we cannot fix from here, it does not affect Intel machines,
and 1.98.1 builds the same tree in seconds.

<details>
<summary><h2>Building from source</h2></summary>

**Toolchain.** Install Rust with [rustup](https://rustup.rs/), not your
distribution's `rust`/`cargo`. The workspace is edition 2024 (Rust 1.85+), and
the browser client needs a second target:

```sh
rustup target add wasm32-unknown-unknown
```

The RADE codec, the rtl_433 ISM decoders and the nrsc5/faad2 DRM/HD-Radio
libraries are vendored as git submodules, so clone with:

```sh
git clone --recurse-submodules https://github.com/madmedicnl/sdroxide-brown
# in an existing checkout:
git submodule update --init --recursive
```

**System dependencies.** A native build needs a C toolchain and a few libraries:

```sh
# Debian / Ubuntu
sudo apt install build-essential pkg-config cmake autoconf automake libtool \
                 libclang-dev libasound2-dev libopus-dev
# Arch
sudo pacman -S base-devel pkgconf cmake autoconf automake libtool clang alsa-lib opus
# macOS
brew install pkg-config cmake autoconf automake libtool opus
```

- **ALSA** is not optional on Linux (audio + MIDI). **CMake**, **libclang** and
  **autoconf/automake/libtool** are for RADE, which builds a FARGAN-enabled Opus
  from source — the first build needs network access.
- **libopus** is optional but avoids a CMake 4 problem: if you have CMake ≥ 4 and
  no system Opus, the build stops on `Compatibility with CMake < 3.5 has been
  removed`. Fix it with `sudo apt install libopus-dev pkg-config`, or
  `export CMAKE_POLICY_VERSION_MINIMUM=3.5`.
- **libfdk-aac** is optional and a *runtime* dependency only: it decodes the
  **xHE-AAC** most DRM broadcasters use, and cannot be built in for licence
  reasons. The DRM window says when it is missing.

For the **SoapySDR** backend, install its dev libraries and your radio's driver
module; nothing else (including RTL-SDR) needs an SDR system library, so
`cargo build --release --no-default-features` works with no SoapySDR installed.

**Native binary:**

```sh
cargo build --release
./target/release/sdroxide --probe        # verify your device is seen
```

**Browser client** (separate WebAssembly crate, built with
[Trunk](https://github.com/trunk-rs/trunk) 0.21+):

```sh
cargo install --locked trunk
cd crates/sdroxide-web && trunk build --release      # output in ./dist
```

To bake the client into the binary, build it first, then:

```sh
(cd crates/sdroxide-web && trunk build --release) && cargo build --release --features embed-web
```

Without `embed-web`, `--server` still serves native `--connect` clients; pass
`--web-root crates/sdroxide-web/dist` to serve a Trunk build from disk.

</details>

### Uninstalling completely

Asked for, and answered once and for all here (fork discussion #17). The
`.deb`/`AUR` package owns exactly this, and every line of it goes with the
package:

| Path | What |
|---|---|
| `/usr/bin/sdroxide` | the binary — client and server are the same program |
| `/usr/share/applications/sdroxide.desktop` | the desktop menu entry |
| `/usr/lib/udev/rules.d/60-sdroxide-*.rules` | 9 dongle rules, so no root is needed |
| `/usr/share/icons/hicolor/*/apps/sdroxide-brown.{png,svg}` | the icon, nine sizes |
| `/usr/share/sdroxide/voices/` | the speech voice, for spoken announcements |
| `/usr/share/doc/sdroxide/` | README, `copyright` and the four licence notices |

```sh
sudo apt purge sdroxide-soapysdr      # or sdroxide — removes every path above
rm -rf ~/.config/sdroxide-brown      # every setting, logbook and recording it wrote
```

Three things worth stating plainly, because "is anything left?" is the question:

- **No systemd service is installed**, so a purge cannot leave one running or
  enabled. If you created a unit yourself, it is yours to disable and delete.
- **Nothing is written to `/etc` or `/var`**, and no configuration is created by
  the installer — the program makes `~/.config/sdroxide-brown` on first run, and
  that directory is the other half of a complete removal.
- **The tarball** adds nothing outside the folder you unpacked it into, except
  `~/.local/share/applications/sdroxide.desktop` and its icon if you ran
  `install-desktop-entry.sh` — `./install-desktop-entry.sh --uninstall` removes
  both.

Not ours to remove: your reverse proxy, your own scripts and backups, and any
other program using `libsoapysdr0.8`.

## Running

```sh
sdroxide --freq 14074000 --mode ft8               # native desktop, 20 m FT8
sdroxide --server                                 # server + UI at http://<host>:4950
sdroxide --server --web-root crates/sdroxide-web/dist
sdroxide --connect 192.168.1.10:4950              # native UI driving a remote server
```

**It works with the network unplugged.** Receiving, transmitting, every
decoder, the maps, the panadapter, the logbook and the reception log are all
local — the maps are drawn from data that ships inside the program, and there is
no service to sign into, no key to paste in and nothing to download before it
will run. The handful of features that do use the internet are extras: spots
from PSK Reporter and the DX cluster, the satellite elements, the space-weather
feed, the broadcast schedule, and uploading a log to LoTW, eQSL or QRZ. Each one
is off until asked for, and with no connection the program is quiet about it and
everything else carries on. The manual's **Offline by default** table (2.1)
names each one and what happens without a route out.

**Raspberry Pi 4/5.** Mesa's Vulkan driver (V3DV) makes the display flicker;
sdroxide detects it and renders through OpenGL ES instead, at the cost of about
one core. `WGPU_BACKEND=vulkan sdroxide` takes Vulkan back where it is steady.

<details>
<summary><h2>Startup parameters — every command-line flag</h2></summary>

| Flag | Description |
| --- | --- |
| `--device <ARGS>` | SoapySDR device args (e.g. `driver=hackrf`). |
| `--probe` | List devices and their probed capabilities, then exit. |
| `--console` | Terminal (ASCII) waterfall mode, no GUI. |
| `--siggen` | Use the built-in signal generator instead of hardware. |
| `--file <FILE>` | Play a raw interleaved CF32 IQ file instead of hardware. |
| `--freq <HZ>` | Center frequency in Hz (default: where the last session was left; `14200000` on a first run). |
| `--rate <HZ>` | Sample rate in Hz (default: from config). |
| `--gain <DB>` | Overall RX gain in dB (default: hardware AGC / moderate). |
| `--mode <MODE>` | Initial mode, case-insensitive: `LSB USB CW AM SAM NFM WFM DRM ADS-B VDL2 AIS DIGU DIGL DSB ISB SPEC FT8 FT4 FT2 JS8 WSPR PI4 JT65 JT9 FST4 MSK144 FSK441 Q65 UVPACKET JTTY PSK RTTY RTTY-FM PACKET PACKET-HF APRS SSTV SSTV-FM RIFP WEFAX NAVTEX DSC ACARS OLIVIA CONTESTIA MT63 ARDOP THOR MFSK FSQ ATCHAT HELL RFPAINT RADE HFDL`, and `"HD RADIO"` (the one name with a space in it, so it needs the quotes). Default: the mode the last session was left in. |
| `--antenna <NAME>` | RX antenna port, as the device names it (`LNAH`, `TX/RX`; see `--probe`). Default: the port the last session was left on. |
| `--tx-antenna <NAME>` | TX antenna port, likewise (`BAND1`, `BAND2`). |
| `--server` | Run as a server: HTTP web client + WebSocket streaming backend. |
| `--connect <HOST[:PORT]>` | Connect as a native remote client to a running server. |
| `--port <PORT>` | Server port (default: from config, `4950`). |
| `--web-root <DIR>` | Directory with the Trunk-built web client, e.g. `crates/sdroxide-web/dist` (default: embedded assets with `--features embed-web`). |
| `--fft <N>` | Spectrum FFT size (default `4096`). |
| `--swl` | Start in SWL mode. |
| `--oob-tx` | Lift the amateur-band transmit lockout for this run (licensed out-of-band use; not persisted). |
| smoke tests | `--tx-tune <SECS>`, `--ft8-cq <SECS>`, `--rade-rx <SECS>` |

</details>

<details>
<summary><h2>Keyboard and mouse</h2></summary>

Defaults — all of them, plus PTT, band, mode, filter and more, are rebindable on
the **Controls** tab. The full reference is in the
[manual](docs/USER_MANUAL.md).

| Key | Action |
| --- | --- |
| `←` / `→` | Tune ∓/± 100 Hz (hold **Shift** for 10 Hz) |
| `↑` / `↓` | Tune ± 1 kHz |
| `PageUp` / `PageDown` | Tune ± 10 kHz |
| `M` / `N` / `F` | Mute / noise blanker / fit the panadapter |

On the panadapter: left-click tunes the active VFO, **Shift**+click places the
second receiver, left-drag pans and tunes, right-drag pans only, the wheel zooms
around the cursor, and dragging a passband edge or the frequency-scale strip
resizes the filter or the split.

</details>

## Contributing, LLM usage, licensing

Both local and hosted LLMs were used in the development of this software.
Contributions written with LLMs are welcome under the upstream project's rules:
**read and review** what you submit and be able to explain it; **comment** the
non-trivial parts; **test** on real radio hardware where possible, and disclose
when you could not; don't use an LLM for trivial edits; use a modern model with
enough context; keep commits vendor-neutral. This is a **GPLv3** project, and
changing the licence would violate the terms of several bundled libraries.

One part goes further than GPLv3: CW decoding uses the
[DeepCW](https://github.com/e04/deepcw-engine) model, which is **AGPL-3.0-only**
and is linked into the binary rather than read as data, so its terms cover the
built program as a whole. The practical difference is AGPL section 13: **running
`sdroxide --server` and letting other people use it over a network counts as
conveying, so they must be offered the Corresponding Source.** Running it for
yourself changes nothing.

## What we took from WSJT-CB

The 11 m side of this fork interoperates with,
**[WSJT-CB](https://github.com/vash909/WSJT-CB)** — thanks to its developers.
Their README is a careful description of every convention this fork follows, and
it is worth reading in full; this is the short list of what was borrowed, and of
what was not.

**Borrowed — the on-the-air conventions, so a CB station and this station
interoperate:**

- **The callsign grammar and its acceptance table.** An 11 m identifier is
  `N{1,3}L{1,2}N{1,3}` — a country prefix, one or two letters, a unit number —
  plus the compound `N{1,3}L{1,2}/L{2}` form, the four-digit unit behind a
  one-digit prefix, and the **portable-style modifier suffixes** (`/P`, `/MM`,
  `/QRP`, an event marker) that their `is_callsign` *widens* the standard rules
  to accept. Mirrored case for case.
- **The CB country numbering and names** — the `cb_NNN_to_country` list, and
  the historical country names it shows ("East Germany", "Czechoslovakia",
  "Alaska"). The DXCC flag/continent mapping bolted onto each entry is this
  fork's, so the decode row reuses the same flag machinery as the amateur
  bands.
- **The message layouts and etiquette** — the `<HISCALL> MYCALL` identity
  opener, the one-call free-text answer, the `R±NN` / `RR73` / `73` handling,
  and answering a CQ on the frequency it was heard on.
- **The both-hashed, grid-less CB exchange** — when both stations are CB, one
  call is abbreviated as a 28-bit hash. Their README documents this (third
  parties may see `<...> 26AT016`) as a protocol behaviour, not a bug; this
  fork arrived at the same layout independently and implements the same
  sequence.
- **The band entry and its default dial** — 11 m is a full two-way band, so
  the same modes are offered on it as anywhere else, and **FT8 sits on CB
  channel 26 (27.265 MHz)** as the WSJT-CB community settled, beside the other
  CB channels the per-country plans list.
- **The 11-character ceiling, and its consequence.** A CB call travels in clear
  as a 58-bit base-38 number, and `38^11` is just under `2^58` — so **an 11 m
  callsign may not exceed 11 characters, and any `/zzz` suffix is counted as
  part of that**, not added to it. `19DC3733/P` (10) goes out; `19DC373/QRP`
  (11) is exactly at the limit; `19TST1001/QRP` (13) is not sendable by anyone.
  **If you are running an activation on 11 m, budget the suffix into the call
  before you announce it** — the slash counts, so `/P` costs 2, `/QRP` costs 4.
  This build flags a too-long 11 m call as you type it, in **Settings → General
  → Callsign**. Amateur callsigns are not affected.

### How a CQ run ends on 11 m, and why that is different

An unattended station must stop transmitting eventually, and two separate limits
do that job. They are not interchangeable, and on 11 m they are deliberately not
the same:

- **A stalled exchange is bounded by the transmit watchdog**, in minutes. This
  is the one that matters most on 11 m: propagation can drop mid-QSO and leave
  the station reporting into a dead channel indefinitely. On the CB band that
  is worse than it sounds, because 11 m transmissions are free text with no
  addressing — nothing but another station's 73 on that frequency can end it.
  WSJT-CB cuts transmit for the same reason.
- **A CQ run on 11 m is bounded by "Give up after" — the unanswered-call
  count** — and the watchdog does not cut one. The reason is that a clock cannot
  measure the thing that matters here. A station answering a CQ is expected
  inside about 30 seconds, so "no reply yet" is the *expected* state of a CQ run,
  not evidence of a dead one; and a wall clock started when the operator pressed
  CQ fires on schedule whether the run is young or stale. It was measured cutting
  a run in which a station then answered four times over ninety seconds, from
  +2 dB to +12 dB, its callsign resolved in the clear. The call count is the
  right unit: it counts real attempts, and the settings hover says what the
  number comes to at the current mode's slot length.

**This is 11 m only.** On every other band, and in every other state, the
watchdog behaves exactly as it always has — including the station queue, where
**REPLY** on a busy station holds this one for that station until they call CQ.
If a station answers after a run has ended, the QSO transcript says so by name
and **REPLY** takes them; nothing is transmitted without the operator.

**Not borrowed — this fork's own work:** the decoder is the shared `mfsk-core`
crate, the DSP is this fork's, and the decoder, packer, QSO sequencer and
interoperability glue are written here. WSJT-CB was never a dependency — it is a
protocol this build speaks fluently, not code it runs.

## Acknowledgements

The amateur-side FT8/FT4/FT2 that 11 m builds on comes from the WSJT-X project.
The original program is [sdroxide](https://github.com/dividebysandwich/sdroxide)
by dividebysandwich; this fork is upstream's work plus the CB and listener
additions. All of it stands on your work.

**A big thank you to [OnAir](https://github.com/adswill/OnAir) and its author
[adswill](https://github.com/adswill).** OnAir is a GPL-3.0 receive-first SDR
program of unusual scope — 142 modes, from DVB-T2 to pagers — and it is the
reference this fork reads when it adds a keyboard mode. Its MFSK, MT63, Hell,
SITOR, ARDOP and FSQ implementations are written from the published
specifications and documented per mode, including an honest list of what has been
tested on real signals and what has only been tested on generated ones. That
list, and its signal sample library, are the model this fork's own mode
roadmap is trying to live up to. Nothing of OnAir's is copied without saying so
where it lands, and every mode it teaches us is written in Rust here — the port is
ours, the understanding is theirs. Thank you for publishing it under a licence
that lets a small fork learn from you.

A special shout-out to the **[Dutch CB Group](https://www.dutchcbgroup.nl/)** and
**[LOG11DX.net](https://log11dx.com/)** — two of the best CB communities there
are, and the reason the 11 m side of this fork exists at all. Thanks for the
channels, the logs and the company.

**Thank you to [Kevin](https://github.com/kevin2008-01) (F4NRM)** for the
testing that made this fork measurably better, and for being blunt about it.
Three stations of his own — a PlutoSDR, a HackRF and an RTL-SDR on one Debian
box behind Caddy, reached from a phone as an installed PWA and from a desktop
browser — went through a full checklist in one pass, and he came back with a
report that separated what was fixed from what was still broken instead of
rounding it up. That report is the reason the sign-in cookie exists, the reason
the phone's tab strip scrolls, and the reason the image share button checks
whether it can share before it claims it did. He also handed over a real Olivia
signal — with the text another decoder had already read — which is what settled
that mode's polarity, and he answered a page of questions about his own setup
when answering them was the only way to move a bug forward.

Testing a radio program on three radios at once is not a thing anyone does by
accident, and doing it as a favour rather than as a job is worth more than the
fixes it produced. If you have a station, a list, and the patience: the reports
are what this fork is built on.

Several of the satellite tools were **inspired by
[SkyRoof](https://github.com/VE3NEA/SkyRoof)** by Alex Shovkoplyas, VE3NEA — the
pass scheduler and the transponder bands on the panadapter's frequency scale came
from reading what it does. **SkyRoof is AGPL-3.0 and this fork is GPL-3.0, so none
of its code is used**: the ideas are theirs and the implementation here is ours,
written against our own satellite and Doppler code. Thank you for showing what the
shape should be.

Several of the listener's propagation tools were adapted from
**[OpenHamClock](https://github.com/accius/openhamclock)** (MIT): the
band-opening detector, the grey-line shading on the flat maps, the
meteor-shower calendar, the IBP beacon schedule and the schedule's local solar
time. The algorithms are theirs; the integration is this fork's. Thanks for the
groundwork.

