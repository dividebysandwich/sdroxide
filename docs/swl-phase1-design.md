# Phase 1 design — the listener's identity

Scope: **[ROADMAP.md](../ROADMAP.md) Phase 1** — a listener profile, an SWL
listening log with SINPO, and a reception-report generator. Receive-only; no
transmit work.

Written before code, so the data model and the integration points are settled
before they are expensive to change.

## 1. The SWL log

A log of what was **heard**, not worked. It is not the QSO log and must not be
folded into it: an SWL entry has no callsign, no RST and no grid, and forcing
those into `QsoRecord` would poison both.

### Data model (`sdroxide-types`)

```rust
/// One reception: what was heard, when, and how well.
pub struct SwlEntry {
    /// Unix seconds, UTC. The log is sorted and grouped by this.
    pub heard_at_unix: u64,
    /// Station name, as free text or as it appears in the schedule.
    pub station: String,
    pub freq_hz: f64,
    pub mode: Mode,
    /// Programme language, free text ("English", "Dutch").
    pub language: String,
    /// SINPO: the five figures, each 1–5. `None` where not judged.
    pub sinpo: Option<Sinpo>,
    /// S-meter reading in dBm at the time, from the engine's own meter.
    pub smeter_dbm: Option<f32>,
    /// Programme notes — what was on.
    pub notes: String,
    /// Transmitter site / target, when the schedule supplied them.
    pub site: String,
}

/// S (strength) I (interference) N (noise) P (propagation) O (overall), each 1–5.
pub struct Sinpo { pub s: u8, pub i: u8, pub n: u8, pub p: u8, pub o: u8 }
```

- `Sinpo` is the reporting convention; `SIO` (three figures) is just the same
  type with `n`/`p` left at a sentinel, or a small `enum ReportCode { Sinpo, Sio }`
  on the entry. Decide in code; the storage is the same five bytes either way.
- Serde-stable and `#[serde(default)]` throughout, so a log written by an older
  build always loads.

### Storage (`sdroxide-config`)

- `swl_log.json` under the config directory, next to `qso_log.json`, with the
  same load/save shape (atomic write, one file).
- **Owned by the UI, like the QSO log** (`persist.rs` → `sdroxide_config`,
  and eframe storage in the browser) — *not* the engine-owned profiles pattern
  first written here. The QSO log is the sibling window and stores this way,
  and one log of the two being engine-owned and the other not would be a
  difference with nothing behind it. The engine keeps only its `LogQso`
  side-effects; it does not hold the logbook, and it will not hold this.

### UI (`sdroxide-ui`)

- A **LISTEN** window in the same family as the LOG window, reachable from the
  top strip in SWL mode. A grouped list (by day), click a row to edit, plus a
  **+ NEW**. Reuse the logbook window's layout and widgets rather than inventing
  a second visual language.
- The entry form: station, frequency (pre-filled from the dial), mode (from the
  dial), language, **SINPO** as five small steppers, notes, and a
  **HEARD NOW** button that stamps UTC and reads the S-meter and the dial.
- In **listener mode** the LOGBOOK button is hidden and LISTEN takes its place.

### Localisation of the workflow

- **Pre-fill from the dial**: opening "+ NEW" seeds frequency, mode and the
  S-meter from the radio, so logging a station found by tuning is two fields
  (station, SINPO) and a click.
- **From the schedule** (Phase 2): the station identity, language and site come
  in filled, which is the payoff for building the log first.

## 2. The reception report

A **REPORT** button on a selected entry produces plain text a listener can
paste into an email or a station's web form:

```
Reception report

Station:    Radio Taiwan International
Frequency:  6185 kHz, AM
Heard:      2026-09-16 19:42–20:10 UTC
Location:   (station's grid, from Settings → General)
Receiver:   RTL-SDR + sdroxide
SINPO:      4 3 3 4 4
Notes:      ...
```

- Pure formatting from the entry plus the station identity and grid — a testable
  function, no UI dependency, so the tests pin the exact wording.
- Copy-to-clipboard and (native) save-to-file. No email client integration.

## 3. The listener profile

> **As built:** the separate `listener_mode` switch was folded into **SWL
> mode** instead — one switch, not two. SWL mode now hides the transmit
> controls *and* swaps the spot feeds and awards for SCHEDULE and LISTEN. The
> design below is kept as written, with that correction.

One switch, `listener_mode`, that carries the identity:

- Sets **SWL mode** and **Simple UI** on, and keeps them on (they stay
  independently toggleable afterwards).
- Hides the ham *receive* chrome a listener never uses — **AWARDS**, **SPOTS**
  (DX cluster / POTA / SOTA), **QSL uploads** — from the top strip and menus.
- Replaces the LOGBOOK entry point with **LISTEN** (the SWL log).
- Lives in `UiSettings` (per screen, like SWL mode and Simple UI, because it is
  a display preference and a remote client may want its own). The *data* it
  shows — the SWL log — is the station's.

The switch is a plain `#[serde(default)] bool` so every existing config loads
with listener mode off.

## 4. What is deliberately out of scope

- The **schedule browser** — Phase 2. The log is built so the schedule can fill
  it in later, but Phase 1 does not add the picker.
- **ECSS, recordings, instant replay** — Phase 3.
- **Removing the CB band** — a later decision (see ROADMAP "Not goals"); Phase 1
  only hides the ham chrome a listener does not use.

## 5. Tests

- `Sinpo`/`SwlEntry` serde round-trip, and "an entry written by an older build
  still loads" (a fixture with a missing field).
- Log storage: add / update / delete / reload, and that the file survives a
  corrupt entry by dropping it rather than failing to load.
- The report formatter: a golden string for a known entry, and the
  missing-pieces case (no SINPO, no S-meter) printing "not judged" rather than
  blank or a panic.
- Listener mode: turning it on sets SWL + Simple UI, and hides the ham chips;
  turning it off leaves SWL/Simple UI as they were.

## 6. Open questions

- **SIO vs SINPO**: offer both, or SINPO only with the unused digits left at a
  neutral 3? Both is more faithful to how listeners report; SINPO-only is less
  UI.
- **One log or two?** The SWL log and the QSO log are separate files, and the
  UI shows one or the other. Confirm a listener never wants both at once.
- **Remote clients**: the SWL log is a station fact and travels like the QSO
  log; the report's receiver/antenna line is per screen. Keep them apart.
