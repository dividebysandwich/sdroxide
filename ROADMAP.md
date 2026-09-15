# ROADMAP — SDR Oxide, the SWL edition

What this fork is for: turning sdroxide's receiver into the best **shortwave
listening** program it can be — no licence, no callsign, no transmitting unless
asked for. This file is the plan; it changes as the fork teaches us what
matters.

Ordered by value to a listener, not by effort. Each phase is meant to stand on
its own.

## Phase 1 — the listener's identity

**Done.** A **listener mode** switch (it sets SWL mode and Simple UI and hides
the DX-cluster/POTA/SOTA spots and the awards), a **LISTEN window** with the
SWL reception log — station, frequency, UTC, mode, language, **SINPO or SIO**,
S-meter, notes — and a **REPORT** button that writes the entry as a reception
report to send to the broadcaster. The log lives in `swl_log.json`, its own
file, and records what was *heard* rather than worked.

The goal it was built for: opening the program should feel like a listener's
radio, not a transceiver with the transmit parts hidden.

- **Listener profile.** One setting that sets SWL mode and Simple UI, and hides
  the ham *receive* chrome that means nothing to a listener — awards
  (DXCC/WAS/WAZ), the DX-cluster/POTA/SOTA spots, QSL uploads — replacing them
  with station, schedule and propagation.
- **SWL listening log.** A log that records what was *heard*, not what was
  worked: station (from the schedule where known), frequency, UTC, mode,
  **SINPO/SIO**, language, programme notes, and an S-meter reading.
- **Reception report.** Generate a ready-to-send report for a station
  (station, date/time UTC, frequency, SINPO, receiver/antenna), the way SWLs
  report to broadcasters.

## Phase 2 — the broadcast schedule (the centrepiece)

- **Browsable schedule.** The EiBi data is already in the program, labelling the
  waterfall with each transmitter's UTC window, language and site. Surface it as
  a list you can search and filter: *"what is on now?"*, *"19:00 UTC, Dutch, to
  Europe"*, *"everything on 49 m"*.
- **Tune from the schedule.** Clicking a station tunes the dial, sets AM (or the
  right mode), and opens a log entry with the station identity filled in.
- **Favourites.** "My stations", tied to the schedule rather than to bare
  frequencies.

## Phase 3 — listening tools

- **Time-shift buffer / instant replay.** Keep a rolling few minutes of audio so
  the thing you just missed can be played again. For a listener this is worth
  more than another decoder.
- **Scheduled recordings.** "Record 49 m at 18:00 UTC for 30 minutes", audio and
  optionally IQ, with a filename that names the station.
- **ECSS.** Selectable-sideband synchronous AM (SAM with one sideband), the
  standard medium-wave DX tool for ducking an adjacent channel.
- **Listener audio chain.** Bandwidth, tone and noise reduction aimed at
  broadcast audio, rather than the speech-trained tools the ham side uses.

## Phase 4 — polish

- **UTC first**: a prominent UTC clock, and UTC wherever a time is shown.
- **Utility labels**: VOLMET, NAVTEX, time signals, numbers stations — the same
  idea as the broadcast labels, for the listener who follows utilities.
- **DRM for the listener**: show the programme text and MOT slideshow, not just
  the audio.
- **Band scanning for listeners**: walk 49 m and stop on carriers, with the
  station name.

## Not goals

- **Transmitting.** SWL mode is the default; there is no push to make transmit
  first-class here. It stays available for anyone who wants it, behind the
  upstream lockouts.
- **Ham operating aids.** Awards, contesting and QSL chasing are upstream's and
  stay read-only-or-hidden.
- **The CB band.** This fork grew out of the CB/SWL fork and still carries its
  11 m work; it is not the focus here and may be retired or hidden as the
  listener features take over.

## Relationship to upstream

Seeded from the CB/SWL fork (`madmedicnl/sdroxide`), which is a fork of
`dividebysandwich/sdroxide`. Upstream changes are merged in periodically; a
feature that is useful to *anyone* (not just listeners) is a candidate to offer
upstream as a pull request rather than keep here.
