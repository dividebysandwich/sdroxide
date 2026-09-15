# Phase 2 & 3 design — the schedule, and listening tools

Scope: **[ROADMAP.md](../ROADMAP.md) Phases 2 and 3.**
- Phase 2: a browsable broadcast schedule you can tune from, feeding the log.
- Phase 3: time-shift/instant replay, scheduled recordings, ECSS, and a listener
  audio chain.

Written before code. What already exists shapes most of this: the EiBi schedule
is *already in the program* (`broadcast.rs`: name, frequency, site, country,
lat/lon, power, language, target, mode, UTC window, days, `on_air_at`,
`schedule_label`), and the SPOTS window already lists the on-air broadcasters.
Phase 2 is therefore mostly a **window and a query**, not new data.

## Phase 2 — the broadcast schedule

### What is missing today

- The on-air broadcasters appear in the **SPOTS** list, which is DX-shaped:
  cluster spots, POTA/SOTA, callsign logic. A listener wants to browse the
  *schedule* by language, country, target and band, and at a **chosen UTC time**,
  not only "now".
- Clicking a spot tunes, but there is no path from a schedule row to a **log
  entry** with the station filled in — which is the whole point of having the
  schedule in a listener's program.

### The SCHEDULE window

A new window, in the same family as SPOTS and LISTEN, and the centrepiece of the
listener's workflow.

- **Time**: a control for *now* or a typed UTC time ("19:00"), so a listener can
  plan around a broadcast, not only catch what is on. The list answers
  "what runs at that time".
- **Filters**, combinable:
  - free-text over name / site / country (the search a listener reaches for:
    `BBC`, `Ascension`, `Romania`);
  - **language** (`lang`);
  - **target** area (`target`) and **country** of the transmitter;
  - **band** — derived from the frequency against the broad band edges, so
    "49 m" is a filter rather than a guess.
- **Rows**: frequency, station, language, target, site, the UTC window, power.
  Sorted by frequency, the way a schedule is read.
- **Click to tune**: set the dial to the carrier and the mode from the
  station's `mode` (default AM). Reuse the same tuning path the spots use, so a
  schedule row and a spot behave identically.
- **LOG**: opens the LISTEN entry form pre-filled with station, frequency,
  language and site, so a reception is one judgement away — the payoff for
  building the log first.
- **Counts**: "N on air at 19:00" and "N in this filter", so an empty result is
  legible as a filter that matched nothing rather than a broken list.

### Data and queries

- Reuse `sdroxide_types::broadcast` and the app's `broadcast` list (the loaded
  EiBi table, user edits merged). No new download path.
- Add small pure helpers in `broadcast.rs`, testable without a UI:
  - `on_air_at(stations, unix) -> impl Iterator` already exists per station;
    a slice-level filter is a `filter` over `on_air_at`.
  - `band_of_khz(f64) -> Option<&str>` — the metre-band name for a broadcast
    frequency (LW/MW/49m/41m/…), so the band filter and the row label agree.
  - `matches_query(&self, q: &str) -> bool` — name/site/country.
- All filtering is pure and unit-tested; the window is a thin view over it.

### Out of scope in Phase 2

- Editing the schedule (the Settings → Spots tab already lets an operator add
  stations and corrections).
- A calendar view; a time-of-day filter is enough.

## Phase 3 — listening tools

Four features, each its own slice. They are independent, so they land one at a
time rather than behind one big switch.

### 3a. Time-shift buffer / instant replay

Keep the last N seconds (two minutes is the target) of demodulated audio in a
ring buffer per receiver, and a **REPLAY** control that plays it back.

- Lives in the receive chain beside the recorder tap (`RxChain`), which already
  holds a copy of the demodulated audio — the buffer is a bounded ring of that
  same stream, so nothing new is tapped.
- Playback is a UI-side read of the ring through the existing audio path, with
  a position readout. It is **listen-only**: no transmit, no engine state.
- Memory cost: two minutes of 48 kHz mono f32 is ~23 MB per receiver — state the
  figure and make the length a setting with a sane cap.
- Edge cases to pin in tests: wrap-around, a mode change (the buffer's content
  is still valid — it is audio), and a sample-rate change on reconnect (drop the
  buffer rather than play mixed rates).

### 3b. Scheduled recordings

"Record 49 m at 18:00 UTC for 30 minutes", audio and optionally IQ, with the
filename naming the station.

- A small persisted list of **jobs** (time UTC, frequency, mode, duration,
  audio/IQ, optional station name). A UI list in the LISTEN window or its own.
- The engine already records audio and IQ on demand (`SetRecording`,
  `SetIqRecording`); a scheduler is a clock and a state machine over those, plus
  retuning to the frequency at start. It must **not** fight the operator: a job
  that fires while somebody is listening is reported, not silently obeyed.
- Storage `recordings.json`; the files land beside the existing recordings.

### 3c. ECSS

Selectable-sideband synchronous AM — SAM with one sideband, the standard MW DX
tool for ducking an adjacent channel.

- The SAM demod already has the carrier PLL; ECSS is choosing one sideband of
  the *demodulated* audio rather than tuning off the carrier. The cleanest
  framing is a **receive option on the AM/SAM modes** (a "sideband" selector:
  both / upper / lower), not a new mode — it is the same signal with a different
  filter, and a mode would multiply the picker.
- Pairs naturally with a narrowing filter, so it is also where the **listener
  audio chain** (bandwidth, tone, NR aimed at broadcast, not speech) belongs.

### 3d. Listener audio chain

- A broadcast-appropriate audio path: adjustable bandwidth, a tone control, and
  noise reduction that does not assume speech. May be as small as exposing what
  exists with listener-appropriate defaults, or a small shelf/peak EQ plus a
  gentle AGC. Decide after 3c, since the two share a control area.

## Order

1. **Phase 2** first — the schedule is the reason to open the program.
2. **3a** time-shift next: the highest value for the least code, and it touches
   only the receive chain and a control.
3. **3b** scheduled recordings, then **3c/3d** together (same control area).

## Testing

- Phase 2: the pure filters (band, query, time) as unit tests; the window as a
  thin view.
- 3a: the ring's wrap, the drop-on-rate-change, and a golden "replay returns
  what was just played".
- 3b: a job firing (with a mocked clock) records for the right duration, and a
  job that fires while the operator is listening reports rather than retunes.
- 3c: ECSS sidebands recover the expected side of a synthetic AM signal, in the
  C-QUAM test style.
