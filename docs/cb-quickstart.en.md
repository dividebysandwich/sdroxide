# CB quick-start (sdroxide — 11 m)

A short, task-oriented walkthrough for getting **sdroxide** going on the
**11 m (27 MHz)** citizens band: setting up your SDR or CAT rig, using the
per-country channel plans, and following the WSJT-CB digital traffic. For the
reference detail behind each control, see [`USER_MANUAL.md`](USER_MANUAL.md);
for the why of this fork, see the [README](../README.md).

> This is the CB/SWL fork of sdroxide. The 11 m band and the broadcast bands
> are added on top; the amateur bands and everything else are upstream and
> unchanged.

*Nederlands: [cb-quickstart.nl.md](cb-quickstart.nl.md).*

Bold names such as **SETTINGS** and **Callsign** are buttons and fields exactly
as they appear on screen. `Settings > Radio` is a menu path.

---

## What you need

- **An SDR or a radio sdroxide supports.** For 11 m the usual choices are:
  - **RTL-SDR** (dongle) — native, no SoapySDR needed.
  - **RX-888 / RX-888 Mk2** — native; the firmware is uploaded to the receiver
    automatically.
  - **Airspy HF+** (Dual / Discovery / Ranger) — native, 0.5 kHz–31 MHz.
  - Also possible: HackRF, Airspy R2/Mini, SDRplay RSP, ELAD, PlutoSDR, or a
    **CAT rig** (Icom/Yaesu/Xiegu) over a serial port + sound card, TCI,
    OpenHPSDR, or SoapySDR.
- **A 27 MHz antenna** suited to your receiver.
- **sdroxide installed** (below).

## Installing

- **Windows** — the installer (`.msi`) or the portable `.zip` (which contains
  `sdroxide.exe`): see the [Releases page](https://github.com/madmedicnl/sdroxide/releases/latest).
- **Linux** — the **AppImage** (one file, `chmod +x` and run), the `.deb`, or
  the portable tarball.
- **macOS** — the `.dmg`.

You can also run sdroxide as a **server** and open it in a browser:
`sdroxide --server`, then `http://localhost:4950`. Handy when the antenna is
somewhere else.

---

## Part A — One-time setup

Everything here is saved under `~/.config/sdroxide/`, so you do it once.

### 1. Start sdroxide

The main window has the control bar across the top and the panadapter and
waterfall below it.

### 2. Pick your radio

Open **SETTINGS** and go to the **Radio** tab. Pick your interface (for example
**RTL-SDR**, **RX-888** or **Airspy HF+**), then the **sample rate** and the
**gain**. Changes apply right after **Apply / reconnect**.

- **Linux — USB receivers:** install the bundled udev rules, or sdroxide will
  see the device but fail to open it:
  `sudo cp 60-sdroxide-*.rules /usr/lib/udev/rules.d/ && sudo udevadm control --reload`, then replug.
- **Windows — RX-888:** bind the device to **WinUSB** once with
  [Zadig](https://zadig.akeo.ie/) — for **both** USB ids (`04B4:00F3` and
  `04B4:00F1`).

### 3. Fill in your details

Go to the **General** tab:

- **Callsign** — for CB/WSJT-CB, enter your CB identifier here.
- **Locator / grid** — your Maidenhead grid; the map and the decoding use it.
- **IARU region** — the region that sets the band plan.
- **CB plan** — pick your country's channel plan: **World / freeband**,
  **CEPT/EU** (the Netherlands), **Germany 80 channels**, **UK 27/81**,
  **USA**, **Australia**. This is what decides the channels and the channel
  number shown on the waterfall.

---

## Part B — Listening on 11 m

1. Pick the **11 m** band on the band bar. It runs from **26.965 to
   27.860 MHz**.
2. With the **CEPT/EU** plan, **channel 1 = 26.965 MHz** and **channel 40 =
   27.405 MHz** (10 kHz spacing). The channel number appears on the
   panadapter.
3. Pick the **mode**: **AM** or **FM** for voice, **USB/LSB** for SSB on the
   freeband.
4. **Listening only?** Turn on **SWL mode**: every transmit control (PTT,
   TUNE, CALL CQ, …) disappears and you are left with a clean receiver. With
   **Simple UI** you see only the controls you need, with **AM · FM · USB ·
   LSB** up front.

---

## Part C — Digital on 27 MHz (WSJT-CB / FT8 family)

This fork speaks the same hashed WSJT exchange that
[WSJT-CB](https://github.com/vash909/WSJT-CB) uses on 27 MHz.

1. Pick **FT8** (or FT4/FT2).
2. Pick the digital channel from the channel plan / channel list of your CB
   plan.
3. In the **decode list**: click a row to move your audio onto that signal and
   press **REPLY** or **Call CQ**.
4. Every decoded station shows its **country flag** — sdroxide follows
   WSJT-CB's callsign conventions and country numbering.
5. In **SWL mode** you can read it all without transmitting.

---

## Part D — Saving your setup (profiles)

Under **Settings > Profiles** you save a whole working setup under a name and
put it back later with one click: dials and VFOs, mode and filters, gain, drive
and antennas, the digital identity and the band stacks. Handy for switching
between "11 m at home" and "listening to shortwave".

---

## Part E — Extras

- **CW on your keyboard:** hold the **Space** bar — your keyboard works as a
  straight key.
- **Export the decode list** to **CSV** or a *received-report* **ADIF**, for
  keeping track of what was on the air.
- The **browser client** can **import** ADIF and **CHIRP** files.
- **Audible alerts** for calls and new DXCC/grids.
- **Themes:** ten extra colour themes.

---

## A note on transmitting

- Outside the amateur bands the **transmit lockout** is on by default. **11 m /
  CB is not an amateur band**, so transmit is blocked unless you lift the
  lockout with `--oob-tx` for that session.
- Whether you may transmit on 11 m, and at what power/mode, differs by country
  (in the Netherlands, the licence-free CEPT channels at limited power).
  **Check the current rules — the responsibility is yours.**
- This fork is first of all built to **receive and decode**.

---

## Help and links

- [README](../README.md) — the full overview of the fork.
- [USER_MANUAL.md](USER_MANUAL.md) — the manual, control by control.
- [WSJT-CB](https://github.com/vash909/WSJT-CB) — the digital 11 m project
  this builds on.
- [Releases](https://github.com/madmedicnl/sdroxide/releases/latest) — the
  latest build for every platform.

See you on 11 metres! 73
