//! The aircraft table: accepted frames in, one row per ICAO address out.
//!
//! Everything an aircraft says arrives in pieces — the callsign every five
//! seconds, the altitude and position twice a second in alternating even/odd
//! halves, the velocity in its own message, the squawk only in an answer to an
//! interrogation. This module is where those pieces are folded into one picture
//! per address, and where the two ages that govern a target's life on screen
//! are kept.
//!
//! # Positions
//!
//! A position squitter is a position modulo a grid ([`crate::cpr`]). The first
//! fix for an aircraft needs an even and an odd frame close together; every fix
//! after that comes from a single frame decoded locally against the previous
//! one, which is both faster and what keeps a target moving through the gaps
//! where half the frames are lost. Surface positions are local-only and fall
//! back to the operator's own position as the reference, which is the right
//! answer in practice: an aircraft on the ground is at an airport.
//!
//! # The turn rate is derived, not received
//!
//! Nothing in ADS-B broadcasts rate of turn. The leader line on the map is
//! supposed to curve the way the aircraft is curving, so this module
//! differentiates successive track angles over a few seconds and low-passes the
//! result. It is measured against a clock taken from the *sample stream* rather
//! than from the wall, so it is exactly reproducible in a test and unaffected
//! by however the engine happens to be scheduling blocks.

use std::collections::HashMap;

use sdroxide_types::{
    ADSB_POSITION_RATE_HZ, ADSB_RX_WINDOW_S, AdsbAircraft, AdsbSettings, AdsbSource,
};

use crate::cpr;
use crate::frame::Accepted;
use crate::message::{Body, Cpr, Es};

/// How far apart an even and an odd frame may be and still be decoded as a
/// pair, in seconds.
///
/// Ten is the figure the standard's own guidance uses. An aircraft at 500 knots
/// covers about 1.4 NM a second, so a stale partner does not merely add error —
/// past a point it resolves to the wrong zone entirely.
const CPR_PAIR_MAX_S: f64 = 10.0;

/// How old a fix may be and still serve as the reference for a local decode.
const LOCAL_REF_MAX_S: f64 = 60.0;

/// The fastest a new fix may imply the aircraft moved since the last one,
/// knots, before it is taken for a decoding error and dropped; plus
/// [`JUMP_SLACK_KM`] for the CPR quantisation and the latency of a feed.
///
/// A CPR pair decoded across a zone boundary, or one frame with an undetected
/// bit error, does not land near the truth: it lands a whole zone away — six
/// degrees of latitude, about 670 km, or a zone of longitude, 10° at 52° N —
/// and the next good frame brings it back. Measured 2026-11-27 on a readsb
/// feed: KLM49L at 40 km, seven seconds later 929 km (+6.09° latitude), twelve
/// seconds after that 43 km again; 42 of 74 339 kept points that day. Two
/// thousand knots is above anything that sends ADS-B, and over the longest gap
/// a fix is checked against (a minute, [`LOCAL_REF_MAX_S`]) it allows about
/// 60 km — a fifth of the smallest jump a zone error makes.
/// Why: HDZ 2026-11-27: „ist das 600km signal echt?“ — „die koordinate ist
///      falsch dekodiert“.
const JUMP_MAX_KT: f64 = 2000.0;
const JUMP_SLACK_KM: f64 = 10.0;

/// Window the turn rate is measured over, seconds. Long enough that the 1-degree
/// quantisation of a broadcast track does not dominate it, short enough to
/// follow a standard rate turn.
const TURN_WINDOW_S: f64 = 4.0;

/// Smoothing on the turn rate: one part new to three parts old, per sample.
const TURN_EASE: f32 = 0.25;

/// One aircraft, plus the working state that never leaves this crate.
struct Entry {
    ac: AdsbAircraft,
    /// The most recent even and odd airborne position frames, with the stream
    /// time they arrived at.
    even: Option<(Cpr, f64)>,
    odd: Option<(Cpr, f64)>,
    /// Stream time of the last fix, for judging whether it can still serve as a
    /// local reference.
    fix_at: f64,
    /// A verified squitter has been heard from this address, which is what
    /// makes it eligible to authenticate surveillance replies.
    verified: bool,
    /// Track angle and the stream time it was sampled, for the turn rate.
    turn_ref: Option<(f32, f64)>,
    /// The last frames' carrier measurements, absolute Hz, newest last.
    carriers: std::collections::VecDeque<f64>,
    /// Stream times of the airborne position messages heard within the last
    /// [`ADSB_RX_WINDOW_S`], oldest first, for [`AdsbAircraft::rx_share`].
    positions_at: std::collections::VecDeque<f64>,
    /// Stream time of the first airborne position message ever heard from it:
    /// an aircraft that came into range a minute ago has had a minute to send,
    /// not two.
    positions_since: Option<f64>,
}

/// The shortest stretch [`AdsbAircraft::rx_share`] is given for: twenty
/// expected messages, so one more or less moves it by five points.
const RX_MIN_SPAN_S: f64 = 10.0;

/// Note an airborne position message heard at stream time `mono_s`.
fn note_position(e: &mut Entry, mono_s: f64) {
    e.positions_since.get_or_insert(mono_s);
    e.positions_at.push_back(mono_s);
    while e.positions_at.front().is_some_and(|&t| t < mono_s - ADSB_RX_WINDOW_S) {
        e.positions_at.pop_front();
    }
}

/// The share of the expected airborne position messages heard over the last
/// [`ADSB_RX_WINDOW_S`] up to stream time `mono_s`, or since the first one if
/// that is more recent. Counted at the time asked rather than the time of the
/// last message, so an aircraft gone behind a building falls towards zero
/// instead of keeping the share it had when it was last heard.
fn rx_share(e: &Entry, mono_s: f64) -> Option<f32> {
    if e.ac.on_ground {
        return None;
    }
    let start = (mono_s - ADSB_RX_WINDOW_S).max(e.positions_since?);
    let span = mono_s - start;
    if span < RX_MIN_SPAN_S {
        return None;
    }
    let heard = e.positions_at.iter().filter(|&&t| t >= start).count() as f64;
    Some((heard / (ADSB_POSITION_RATE_HZ * span)).min(1.0) as f32)
}

/// How many carrier measurements the median is taken over. A median rather
/// than a mean: a weak frame's spectrum can peak on noise far from the line,
/// and one such frame must not drag the aircraft off its trace.
const CARRIER_FRAMES: usize = 9;

/// The median of the measurements held, for [`AdsbAircraft::carrier_hz`].
fn carrier_median(c: &std::collections::VecDeque<f64>) -> Option<f64> {
    let mut v: Vec<f64> = c.iter().copied().collect();
    v.sort_by(f64::total_cmp);
    v.get(v.len() / 2).copied()
}

/// The table.
pub struct Tracker {
    by_icao: HashMap<u32, Entry>,
    cfg: AdsbSettings,
    /// Where the receiver is, when the operator has said. Reference of last
    /// resort for a surface position.
    home: Option<(f64, f64)>,
    /// The latest stream time seen, which [`AdsbAircraft::rx_share`] is
    /// counted up to.
    mono_s: f64,
}

impl Tracker {
    pub fn new(cfg: AdsbSettings) -> Tracker {
        Tracker { by_icao: HashMap::new(), cfg: cfg.sane(), home: None, mono_s: 0.0 }
    }

    /// Move the stream clock on without a message, so the shares of the
    /// aircraft that have gone quiet fall while nothing is heard.
    pub fn clock(&mut self, mono_s: f64) {
        self.mono_s = self.mono_s.max(mono_s);
    }

    pub fn set_config(&mut self, cfg: AdsbSettings) {
        self.cfg = cfg.sane();
    }

    pub fn set_home(&mut self, home: Option<(f64, f64)>) {
        self.home = home;
    }

    /// Whether an address has been proved by a verified squitter recently
    /// enough to authenticate a surveillance reply. This is what
    /// [`crate::frame::accept`] asks.
    pub fn knows(&self, icao: u32, now: i64) -> bool {
        self.by_icao
            .get(&icao)
            .is_some_and(|e| e.verified && now - e.ac.last_at <= i64::from(self.cfg.drop_list_s))
    }

    pub fn len(&self) -> usize {
        self.by_icao.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_icao.is_empty()
    }

    /// Fold one accepted message into the table.
    ///
    /// `now` is unix seconds, for the ages the panel shows; `mono_s` is the
    /// stream clock, for the arithmetic that has to be monotonic and finer than
    /// a second.
    pub fn absorb(
        &mut self,
        acc: &Accepted,
        bytes: &[u8],
        rssi_dbfs: f32,
        carrier_hz: Option<f64>,
        now: i64,
        mono_s: f64,
    ) {
        let icao = acc.icao();
        let verified = acc.is_verified();
        let home = self.home;
        let history = self.cfg.history_points as usize;

        let e = self.by_icao.entry(icao).or_insert_with(|| Entry {
            ac: AdsbAircraft::new(icao, now),
            even: None,
            odd: None,
            fix_at: f64::NEG_INFINITY,
            verified: false,
            turn_ref: None,
            carriers: std::collections::VecDeque::new(),
            positions_at: std::collections::VecDeque::new(),
            positions_since: None,
        });
        self.mono_s = self.mono_s.max(mono_s);
        if let Some(hz) = carrier_hz {
            if e.carriers.len() == CARRIER_FRAMES {
                e.carriers.pop_front();
            }
            e.carriers.push_back(hz);
            e.ac.carrier_hz = carrier_median(&e.carriers);
        }
        e.verified |= verified;
        e.ac.last_at = now;
        e.ac.frames = e.ac.frames.saturating_add(1);
        e.ac.rssi_dbfs = rssi_dbfs;
        e.ac.source = if acc.is_squitter() { AdsbSource::Squitter } else { AdsbSource::Reply };
        e.ac.raw_hex = bytes.iter().map(|b| format!("{b:02X}")).collect();

        match &acc.message().body {
            Body::Squitter(es) => absorb_es(e, es, home, history, now, mono_s),
            // The answers to an interrogation. Worth taking: near a busy
            // airport a radar sweeps every few seconds, so these arrive far
            // more often than the aircraft's own squitters.
            Body::Altitude { altitude_ft, on_ground } => {
                if let Some(ft) = altitude_ft {
                    e.ac.altitude_ft = Some(*ft);
                }
                e.ac.on_ground = *on_ground;
            }
            Body::Identity { squawk, on_ground } => {
                e.ac.squawk = Some(*squawk);
                e.ac.on_ground = *on_ground;
            }
            // An all-call reply says only that the aircraft is there, which is
            // still worth a row: it is how a target whose ADS-B transmitter is
            // off or broken shows up at all.
            Body::AllCall => {}
        }
    }

    /// Drop what has gone quiet, and hold the table to its ceiling.
    ///
    /// Called on the emit tick rather than per frame: it walks the whole table,
    /// and the table is walked to build the snapshot anyway.
    pub fn expire(&mut self, now: i64) {
        let ttl = i64::from(self.cfg.drop_list_s);
        self.by_icao.retain(|_, e| now - e.ac.last_at <= ttl);
        let max = self.cfg.max_aircraft as usize;
        if self.by_icao.len() > max {
            // Oldest first: a ceiling reached on a busy sector should cost the
            // targets nobody has heard from lately rather than the new arrivals.
            let mut ages: Vec<(i64, u32)> =
                self.by_icao.iter().map(|(k, e)| (e.ac.last_at, *k)).collect();
            ages.sort_unstable();
            for (_, k) in ages.into_iter().take(self.by_icao.len() - max) {
                self.by_icao.remove(&k);
            }
        }
    }

    /// The table as the panel sees it.
    pub fn snapshot(&self) -> Vec<AdsbAircraft> {
        self.by_icao
            .values()
            .map(|e| AdsbAircraft { rx_share: rx_share(e, self.mono_s), ..e.ac.clone() })
            .collect()
    }
}

/// Fold one extended-squitter payload in.
fn absorb_es(
    e: &mut Entry,
    es: &Es,
    home: Option<(f64, f64)>,
    history: usize,
    now: i64,
    mono_s: f64,
) {
    match es {
        Es::Identification { callsign, category } => {
            if !callsign.is_empty() {
                e.ac.callsign = callsign.clone();
            }
            if let Some(c) = category {
                e.ac.category = Some((*c).to_string());
            }
        }
        Es::Airborne { altitude_ft, geometric, cpr } => {
            if *geometric {
                e.ac.gnss_altitude_ft = *altitude_ft;
            } else if altitude_ft.is_some() {
                e.ac.altitude_ft = *altitude_ft;
            }
            e.ac.on_ground = false;
            note_position(e, mono_s);
            airborne_position(e, *cpr, history, now, mono_s);
        }
        Es::Surface { movement_kt, track_deg, cpr } => {
            e.ac.on_ground = true;
            e.ac.altitude_ft = None;
            e.ac.ground_speed_kt = *movement_kt;
            if let Some(deg) = track_deg {
                update_track(e, *deg, mono_s);
            }
            // A surface fix has no global decode, so it needs a reference: the
            // aircraft's own last position if it is fresh, otherwise ours.
            let reference =
                e.ac.lat.zip(e.ac.lon).filter(|_| mono_s - e.fix_at <= LOCAL_REF_MAX_S).or(home);
            if let Some((rlat, rlon)) = reference
                && let Some((lat, lon)) =
                    cpr::local_surface((cpr.lat, cpr.lon), cpr.odd, rlat, rlon)
            {
                place(e, lat, lon, history, now, mono_s);
            }
        }
        Es::Velocity { ground_speed_kt, track_deg, vertical_rate_fpm } => {
            if ground_speed_kt.is_some() {
                e.ac.ground_speed_kt = *ground_speed_kt;
            }
            if vertical_rate_fpm.is_some() {
                e.ac.vertical_rate_fpm = *vertical_rate_fpm;
            }
            if let Some(deg) = track_deg {
                update_track(e, *deg, mono_s);
            }
        }
        Es::Status { squawk, emergency } => {
            if squawk.is_some() {
                e.ac.squawk = *squawk;
            }
            e.ac.emergency = emergency.map(|s| s.to_string());
        }
        Es::Airspeed { heading_deg, airspeed_kt, tas, vertical_rate_fpm } => {
            // Into fields of their own, never the track: see `message::airspeed`.
            let av = &mut e.ac.avionics;
            if heading_deg.is_some() {
                av.heading_deg = *heading_deg;
            }
            if airspeed_kt.is_some() {
                if *tas {
                    av.tas_kt = *airspeed_kt;
                } else {
                    av.ias_kt = *airspeed_kt;
                }
            }
            if vertical_rate_fpm.is_some() {
                e.ac.vertical_rate_fpm = *vertical_rate_fpm;
            }
        }
        Es::TargetState(t) => {
            let av = &mut e.ac.avionics;
            if t.selected_alt_ft.is_some() {
                av.selected_alt_ft = t.selected_alt_ft;
                av.selected_alt_fms = t.selected_alt_fms;
            }
            if t.qnh_hpa.is_some() {
                av.qnh_hpa = t.qnh_hpa;
            }
            if t.selected_heading_deg.is_some() {
                av.selected_heading_deg = t.selected_heading_deg;
            }
            if let Some(m) = t.modes {
                av.autopilot = Some(m.autopilot);
                av.tcas = Some(m.tcas);
                av.modes =
                    [(m.vnav, "VNAV"), (m.alt_hold, "ALT"), (m.approach, "APP"), (m.lnav, "LNAV")]
                        .iter()
                        .filter(|(on, _)| *on)
                        .map(|(_, n)| (*n).to_string())
                        .collect();
            }
            // Version 2 by definition of the subtype: TC 31 may not have
            // been heard yet.
            av.version.get_or_insert(2);
            av.nac_p = Some(t.nac_p);
            av.sil = Some(t.sil);
        }
        Es::OpStatus { version, nac_p, sil } => {
            let av = &mut e.ac.avionics;
            av.version = Some(*version);
            if nac_p.is_some() {
                av.nac_p = *nac_p;
            }
            if sil.is_some() {
                av.sil = *sil;
            }
        }
        Es::Other => {}
    }
}

/// Resolve an airborne position squitter into a fix, locally or globally.
fn airborne_position(e: &mut Entry, cpr: Cpr, history: usize, now: i64, mono_s: f64) {
    if cpr.odd {
        e.odd = Some((cpr, mono_s));
    } else {
        e.even = Some((cpr, mono_s));
    }

    // Local first: it needs one frame instead of two, so once a target is being
    // tracked this is where every fix comes from.
    if let (Some(lat), Some(lon)) = (e.ac.lat, e.ac.lon)
        && mono_s - e.fix_at <= LOCAL_REF_MAX_S
        && let Some((la, lo)) = cpr::local((cpr.lat, cpr.lon), cpr.odd, lat, lon)
    {
        place(e, la, lo, history, now, mono_s);
        return;
    }

    // Otherwise the even/odd pair, which gives the very first fix and recovers
    // one after a gap long enough to make the last fix untrustworthy.
    if let (Some((ev, te)), Some((od, to))) = (e.even, e.odd)
        && (te - to).abs() <= CPR_PAIR_MAX_S
        && let Some((lat, lon)) = cpr::global((ev.lat, ev.lon), (od.lat, od.lon), te >= to)
    {
        place(e, lat, lon, history, now, mono_s);
    }
}

/// Whether a fix at `lat`/`lon` could follow the aircraft's last one: always
/// when there is none or it is older than [`LOCAL_REF_MAX_S`], else only if
/// getting there took no more than [`JUMP_MAX_KT`].
///
/// The last fix ageing out is what lets a track recover should the wrong one
/// have been the last: a minute later the next decode is taken as it comes.
fn plausible(e: &Entry, lat: f64, lon: f64, mono_s: f64) -> bool {
    let (Some(plat), Some(plon)) = (e.ac.lat, e.ac.lon) else { return true };
    let dt = mono_s - e.fix_at;
    if !(0.0..=LOCAL_REF_MAX_S).contains(&dt) {
        return true;
    }
    let km = sdroxide_types::distance_km((plat, plon), (lat, lon));
    km <= JUMP_MAX_KT * 1.852 / 3600.0 * dt + JUMP_SLACK_KM
}

/// Record a fix, pushing the previous one onto the history.
fn place(e: &mut Entry, lat: f64, lon: f64, history: usize, now: i64, mono_s: f64) {
    if !lat.is_finite() || !lon.is_finite() || lat.abs() > 90.0 || lon.abs() > 180.0 {
        return;
    }
    if !plausible(e, lat, lon, mono_s) {
        return;
    }
    if let (Some(plat), Some(plon)) = (e.ac.lat, e.ac.lon) {
        // Only remember somewhere it has actually been: an aircraft parked on a
        // stand reports twice a second, and its whole trail would otherwise be
        // one point drawn forty times.
        let moved = (plat - lat).abs() > 1e-5 || (plon - lon).abs() > 1e-5;
        if moved && history > 0 {
            e.ac.track.push((plat as f32, plon as f32));
            if e.ac.track.len() > history {
                let excess = e.ac.track.len() - history;
                e.ac.track.drain(..excess);
            }
        }
        // A track angle from the movement itself, for an aircraft that has not
        // sent a velocity message yet — which every one of them is, for the
        // first second or two after it comes into range.
        if moved && e.ac.track_deg.is_none() {
            let dy = lat - plat;
            let dx = (lon - plon) * plat.to_radians().cos();
            let deg = dx.atan2(dy).to_degrees() as f32;
            update_track(e, deg, mono_s);
        }
    }
    e.ac.lat = Some(lat);
    e.ac.lon = Some(lon);
    e.ac.last_pos_at = now;
    e.fix_at = mono_s;
    if history == 0 {
        e.ac.track.clear();
    }
}

/// Take a new track angle, and update the derived turn rate with it.
fn update_track(e: &mut Entry, deg: f32, mono_s: f64) {
    let deg = deg.rem_euclid(360.0);
    e.ac.track_deg = Some(deg);
    match e.turn_ref {
        Some((prev, at)) if mono_s - at >= TURN_WINDOW_S => {
            let dt = (mono_s - at) as f32;
            // Shortest way round: an aircraft passing north goes 359 -> 001,
            // which is two degrees right and not three hundred and fifty-eight
            // left.
            let mut d = deg - prev;
            while d > 180.0 {
                d -= 360.0;
            }
            while d < -180.0 {
                d += 360.0;
            }
            let rate = d / dt.max(0.1);
            e.ac.turn_rate_deg_s += (rate - e.ac.turn_rate_deg_s) * TURN_EASE;
            e.turn_ref = Some((deg, mono_s));
        }
        None => e.turn_ref = Some((deg, mono_s)),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    fn entry_at(lat: f64, lon: f64, fix_at: f64) -> Entry {
        let mut ac = AdsbAircraft::new(0x484166, 0);
        ac.lat = Some(lat);
        ac.lon = Some(lon);
        Entry {
            ac,
            even: None,
            odd: None,
            fix_at,
            verified: true,
            turn_ref: None,
            carriers: std::collections::VecDeque::new(),
            positions_at: std::collections::VecDeque::new(),
            positions_since: None,
        }
    }

    /// The jump measured on KLM49L: 40 km out, then a fix one latitude zone
    /// north seven seconds later. It is dropped; the aircraft stays where it
    /// was, and the next ordinary fix is taken.
    #[test]
    fn a_fix_a_zone_away_seconds_later_is_dropped() {
        let mut e = entry_at(51.14, 7.34, 100.0);
        place(&mut e, 57.23, -3.06, 10, 7, 107.0);
        assert_eq!((e.ac.lat, e.ac.lon), (Some(51.14), Some(7.34)), "teleported");
        assert_eq!(e.fix_at, 100.0, "a dropped fix is no reference");
        place(&mut e, 51.11, 7.40, 10, 19, 119.0);
        assert_eq!((e.ac.lat, e.ac.lon), (Some(51.11), Some(7.40)));
    }

    /// Ordinary flight passes: 500 kt for ten seconds is 2.6 km.
    #[test]
    fn a_fix_at_airliner_speed_is_taken() {
        let mut e = entry_at(51.0, 7.0, 0.0);
        place(&mut e, 51.0 + 2.6 / 111.2, 7.0, 10, 10, 10.0);
        assert!((e.ac.lat.unwrap() - 51.0234).abs() < 1e-3);
    }

    /// With the last fix older than a minute there is nothing to judge by, so
    /// a track that was itself wrong recovers.
    #[test]
    fn after_a_minute_the_next_fix_is_taken_wherever_it_is() {
        let mut e = entry_at(57.23, -3.06, 0.0);
        place(&mut e, 51.14, 7.34, 10, 61, 61.0);
        assert_eq!((e.ac.lat, e.ac.lon), (Some(51.14), Some(7.34)));
    }

    use super::*;
    use crate::frame::accept;

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len() / 2).map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap()).collect()
    }

    fn feed(t: &mut Tracker, s: &str, now: i64, mono: f64) {
        let b = hex(s);
        let a = accept(&b, |i| t.knows(i, now)).expect("the test vectors are all valid");
        t.absorb(&a, &b, -20.0, None, now, mono);
    }

    /// The carrier is the median of the last frames: one frame measured far
    /// off — a weak one whose spectrum peaked on noise — does not move the
    /// aircraft off its trace, and a frame with no measurement leaves it.
    #[test]
    fn the_carrier_is_the_median_of_the_last_frames() {
        let mut t = Tracker::new(AdsbSettings::default());
        let b = hex("8D40621D58C382D690C8AC2863A7");
        let a = accept(&b, |_| false).unwrap();
        for hz in
            [1_090_175_200.0, 1_090_174_800.0, 1_090_640_000.0, 1_090_175_000.0, 1_090_175_100.0]
        {
            t.absorb(&a, &b, -20.0, Some(hz), 1_000, 0.0);
        }
        t.absorb(&a, &b, -20.0, None, 1_000, 0.0);
        assert_eq!(t.snapshot()[0].carrier_hz, Some(1_090_175_100.0));
    }

    /// The share of expected position messages heard: all of them on a clear
    /// path, half when every other one is lost, nothing yet for the first
    /// seconds, and falling towards zero once the aircraft has gone quiet.
    #[test]
    fn the_reception_share_counts_the_position_messages_heard() {
        let even = "8D40621D58C382D690C8AC2863A7";
        let mut t = Tracker::new(AdsbSettings::default());
        feed(&mut t, even, 1_000, 0.0);
        t.clock(5.0);
        assert_eq!(t.snapshot()[0].rx_share, None, "too soon to say");

        // 2 Hz for a minute: every message heard.
        let mut t = Tracker::new(AdsbSettings::default());
        for i in 0..120 {
            feed(&mut t, even, 1_000, f64::from(i) * 0.5);
        }
        t.clock(60.0);
        let full = t.snapshot()[0].rx_share.expect("a minute of positions");
        assert!((full - 1.0).abs() < 0.01, "clear path {full}");
        // A minute of silence after it: half the window heard.
        t.clock(120.0);
        let half = t.snapshot()[0].rx_share.unwrap();
        assert!((half - 0.5).abs() < 0.01, "gone quiet for a minute {half}");
        // Two minutes of silence: nothing in the window.
        t.clock(180.0);
        assert_eq!(t.snapshot()[0].rx_share, Some(0.0));

        // Every other message lost.
        let mut t = Tracker::new(AdsbSettings::default());
        for i in 0..60 {
            feed(&mut t, even, 1_000, f64::from(i));
        }
        t.clock(60.0);
        let lossy = t.snapshot()[0].rx_share.unwrap();
        assert!((lossy - 0.5).abs() < 0.01, "every other lost {lossy}");

        // On the ground the rate depends on whether it moves: no share.
        let mut t = Tracker::new(AdsbSettings::default());
        for i in 0..60 {
            feed(&mut t, even, 1_000, f64::from(i));
        }
        t.by_icao.values_mut().for_each(|e| e.ac.on_ground = true);
        t.clock(60.0);
        assert_eq!(t.snapshot()[0].rx_share, None);
    }

    /// The published even/odd pair resolves to the published position, and the
    /// aircraft's altitude comes with it.
    #[test]
    fn an_even_odd_pair_gives_the_first_fix() {
        let mut t = Tracker::new(AdsbSettings::default());
        feed(&mut t, "8D40621D58C382D690C8AC2863A7", 1_000, 0.0);
        feed(&mut t, "8D40621D58C386435CC412692AD6", 1_000, 0.5);
        let ac = &t.snapshot()[0];
        assert_eq!(ac.icao, 0x40_621D);
        assert_eq!(ac.altitude_ft, Some(38_000));
        // The odd frame arrived second, so the fix is reported in its grid:
        // where the aircraft was half a second later, not where it was for the
        // even one. About a kilometre north-east of the published even-grid
        // answer of 52.2572 / 3.9193.
        let (lat, lon) = (ac.lat.expect("positioned"), ac.lon.expect("positioned"));
        assert!((lat - 52.2658).abs() < 0.001, "latitude {lat}");
        assert!((lon - 3.9389).abs() < 0.001, "longitude {lon}");
    }

    /// Once a target has a fix, one frame is enough — which is the whole reason
    /// the local decode is here. Feeding only even frames after the pair must
    /// keep producing positions.
    #[test]
    fn after_the_first_fix_a_single_frame_keeps_it_moving() {
        let mut t = Tracker::new(AdsbSettings::default());
        feed(&mut t, "8D40621D58C382D690C8AC2863A7", 1_000, 0.0);
        feed(&mut t, "8D40621D58C386435CC412692AD6", 1_000, 0.5);
        let first = t.snapshot()[0].last_pos_at;
        assert!(first > 0);
        // The same even frame again, twenty seconds later: too old to pair with
        // the odd one, and only the local decode can place it.
        feed(&mut t, "8D40621D58C382D690C8AC2863A7", 1_020, 20.0);
        let ac = &t.snapshot()[0];
        assert_eq!(ac.last_pos_at, 1_020, "the local decode did not run");
    }

    /// Callsign and velocity arrive in messages of their own and have to land
    /// on the same row as the position.
    #[test]
    fn one_row_gathers_what_arrives_in_separate_messages() {
        let mut t = Tracker::new(AdsbSettings::default());
        feed(&mut t, "8D4840D6202CC371C32CE0576098", 1_000, 0.0);
        let ac = &t.snapshot()[0];
        assert_eq!(ac.callsign, "KLM1023");
        assert_eq!(ac.icao, 0x4840D6);

        let mut t = Tracker::new(AdsbSettings::default());
        feed(&mut t, "8DA3D42599250129780484712C50", 1_000, 0.0);
        let ac = &t.snapshot()[0];
        let gs = ac.ground_speed_kt.expect("a velocity message carries one");
        assert!((gs - 417.66).abs() < 0.5, "ground speed {gs}");
        let trk = ac.track_deg.expect("and a track");
        assert!((trk - 322.2).abs() < 0.5, "track {trk}");
        assert_eq!(ac.vertical_rate_fpm, Some(0));
    }

    /// The two clocks: a target that stops reporting its position greys but
    /// stays listed, and only goes when nothing has been heard at all.
    #[test]
    fn a_quiet_target_greys_before_it_is_dropped() {
        let mut t = Tracker::new(AdsbSettings::default());
        feed(&mut t, "8D40621D58C382D690C8AC2863A7", 1_000, 0.0);
        feed(&mut t, "8D40621D58C386435CC412692AD6", 1_000, 0.5);
        t.expire(1_030);
        let ac = t.snapshot();
        assert_eq!(ac.len(), 1, "still listed thirty seconds on");
        assert!(ac[0].pos_stale(1_030, 10), "but not to be drawn");
        t.expire(1_100);
        assert!(t.is_empty(), "and gone after a minute of silence");
    }

    /// A moving aircraft leaves a trail; a stationary one does not fill it with
    /// the same point over and over.
    #[test]
    fn the_trail_records_movement_and_not_repetition() {
        let mut t = Tracker::new(AdsbSettings::default());
        feed(&mut t, "8D40621D58C382D690C8AC2863A7", 1_000, 0.0);
        feed(&mut t, "8D40621D58C386435CC412692AD6", 1_000, 0.5);
        for k in 1..10 {
            feed(&mut t, "8D40621D58C386435CC412692AD6", 1_000 + k, 0.5 + k as f64);
        }
        assert!(t.snapshot()[0].track.len() <= 1, "the same fix was recorded repeatedly");
    }

    /// A surveillance reply from an address nobody has heard is refused before
    /// it ever reaches the table.
    #[test]
    fn the_table_is_what_authenticates_a_surveillance_reply() {
        let t = Tracker::new(AdsbSettings::default());
        assert!(!t.knows(0x40621D, 1_000));
        let mut t = t;
        feed(&mut t, "8D40621D58C382D690C8AC2863A7", 1_000, 0.0);
        assert!(t.knows(0x40621D, 1_000));
        assert!(!t.knows(0x40621D, 1_100), "and forgets it once it has aged out");
    }
}
