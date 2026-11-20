//! The aircraft book: who the decoder has heard from, kept past the log's
//! 64-event window and past the map's single square per aircraft.
//!
//! HFDL gives an aircraft no stable name on the air. It logs on to a ground
//! station quoting its **ICAO address** and is handed a short **alias** that
//! means something only on that station; every later frame is addressed by the
//! alias until it logs off. So an aircraft's identity here is whatever has been
//! learned so far:
//!
//! * a downlink names the alias and the station (`who`), and carries the ICAO
//!   too when xng's per-channel cache could resolve it;
//! * a *logon request/resume* carries the ICAO; a *logon confirm* (uplink) binds
//!   it to the alias just assigned — which is how an alias row turns into an
//!   ICAO row, and the book merges the two when that happens;
//! * a *logoff* or *logon-denied* ends the binding.
//!
//! What counts as an aircraft event is a downlink, or one of those logon
//! uplinks. A squitter, a system table, or an uplink addressed to an alias with
//! no logon heard says nothing reliable about *which* aircraft.

use std::collections::{BTreeMap, HashMap};

use sdroxide_types::{
    HFDL_AIRCRAFT_KEEP_S, HFDL_AIRCRAFT_MAX, HFDL_LAST_TEXT_MAX, HfdlAircraft, HfdlMotion,
    hfdl_motion, hfdl_utc_dt,
};
use xng_mode_hfdl::pdu::{HfdlEvent, gs_name};

#[derive(Default)]
pub struct AircraftBook {
    rows: BTreeMap<String, HfdlAircraft>,
    /// (ground station, alias) → ICAO, learned from logon confirms and from
    /// downlinks xng resolved.
    alias: HashMap<(u8, u8), String>,
}

/// What an event says about which aircraft it concerns.
struct Who {
    gs: Option<u8>,
    alias: Option<u8>,
    icao: Option<String>,
}

impl AircraftBook {
    pub fn new() -> Self {
        Self::default()
    }

    /// Fold one decoded event in. `freq_khz` is the lane's channel, `unix`
    /// the decode time.
    pub fn observe(&mut self, e: &HfdlEvent, freq_khz: u32, unix: i64) {
        let Some(who) = who_of(e) else { return };
        let kind = e.kind.as_str();

        // A confirm binds the alias it assigns to the ICAO; a logoff or denial
        // unbinds. Done before the row is chosen so the row the confirm itself
        // belongs to is already the ICAO one.
        if kind == "logon-confirm" {
            if let (Some(gs), Some(icao), Some(a)) =
                (who.gs, who.icao.as_deref(), e.details.get("assigned_id").and_then(|v| v.as_u64()))
            {
                self.alias.insert((gs, a as u8), icao.to_owned());
                if let Some(row) = self.rows.get(&alias_key(gs, a as u8)).cloned() {
                    // Rows keyed by this alias before its logon was heard are
                    // this aircraft.
                    self.rows.remove(&row.key);
                    self.row(&icao_key(icao), unix).absorb(row);
                }
            }
        } else if let (Some(gs), Some(a), Some(icao)) = (who.gs, who.alias, who.icao.as_deref()) {
            // A downlink xng resolved: remember the pair for the ones it cannot.
            self.alias.insert((gs, a), icao.to_owned());
        }

        let icao = who
            .icao
            .clone()
            .or_else(|| who.gs.zip(who.alias).and_then(|k| self.alias.get(&k).cloned()));
        let key = match (&icao, who.gs, who.alias) {
            (Some(i), _, _) => icao_key(i),
            (None, Some(gs), Some(a)) => alias_key(gs, a),
            _ => return,
        };
        // An alias row that predates this aircraft's ICAO becoming known.
        if let (Some(i), Some(gs), Some(a)) = (&icao, who.gs, who.alias) {
            if let Some(old) = self.rows.remove(&alias_key(gs, a)) {
                self.row(&icao_key(i), unix).absorb(old);
            }
        }

        let row = self.row(&key, unix);
        row.last_heard = row.last_heard.max(unix);
        row.msgs = row.msgs.saturating_add(1);
        row.freq_khz = freq_khz;
        row.snr_db = e.snr_db.or(row.snr_db);
        if let Some(i) = &icao {
            row.icao = Some(i.clone());
        }
        if let Some(a) = who.alias {
            row.aircraft_id = Some(u32::from(a));
        }
        // The station the aircraft is on: its own payload's, else the
        // envelope's.
        let gs = e
            .details
            .get("gs_id")
            .and_then(|v| v.as_u64())
            .filter(|&v| v < 128)
            .map(|v| v as u8)
            .or(who.gs);
        if let Some(g) = gs {
            row.gs_id = Some(g);
            row.gs = Some(gs_name(g).map_or_else(|| format!("GS {g}"), str::to_owned));
        }
        row.logged_off = matches!(kind, "logoff-request" | "logon-denied")
            || (row.logged_off
                && !matches!(kind, "logon-request" | "logon-resume" | "logon-confirm"));

        if let Some(p) = e.details.get("position") {
            if let (Some(lat), Some(lon)) =
                (p.get("lat").and_then(|v| v.as_f64()), p.get("lon").and_then(|v| v.as_f64()))
            {
                // Course and speed from the fix before this one: timed by the
                // aircraft's own report clock when both reports carry it (the
                // decode time lags it by however long the frame waited), by
                // the decode times otherwise.
                let utc_s = p.get("utc_s").and_then(|v| v.as_u64()).map(|v| v as u32);
                if let (Some(plat), Some(plon)) = (row.lat, row.lon)
                    && row.fix_at > 0
                {
                    let dt = match (row.fix_utc_s, utc_s) {
                        (Some(a), Some(b)) => hfdl_utc_dt(a, b),
                        _ => unix - row.fix_at,
                    };
                    match hfdl_motion((plat, plon), (lat, lon), dt) {
                        HfdlMotion::Moving(trk, kt) => {
                            row.calc_track_deg = Some(trk);
                            row.calc_speed_kt = Some(kt);
                        }
                        HfdlMotion::Keep => {}
                        HfdlMotion::Clear => {
                            row.calc_track_deg = None;
                            row.calc_speed_kt = None;
                        }
                    }
                }
                row.lat = Some(lat);
                row.lon = Some(lon);
                row.fix_at = unix;
                row.fix_utc_s = utc_s;
                row.map_key = Some(map_key(p));
            }
            if let Some(f) = p.get("flight").and_then(|v| v.as_str()) {
                set_if_text(&mut row.flight, f);
            }
        }
        row.last_kind = kind.to_owned();
        if let Some(a) = crate::controller::acars_of(e) {
            // What it said, for the list's last-message column: the text, or
            // — for a block with none, a link test say — what the decoder made
            // of it. Nothing to say leaves the previous message standing.
            let body = match (a.one_line(), &a.decoded) {
                (t, _) if !t.is_empty() => Some(t),
                (_, Some(d)) => Some(d.clone()),
                _ => None,
            };
            if let Some(body) = body {
                let line = format!("[{}] {body}", a.label);
                row.last_text = Some(if line.chars().count() > HFDL_LAST_TEXT_MAX {
                    let mut cut: String = line.chars().take(HFDL_LAST_TEXT_MAX - 1).collect();
                    cut.push('…');
                    cut
                } else {
                    line
                });
            }
        }
        if let Some(b) = &e.acars {
            if let Some(t) = &b.core.tail {
                set_if_text(&mut row.registration, t);
            }
            if let Some(f) = &b.core.flight {
                set_if_text(&mut row.flight, f);
            }
        }

        if kind == "logoff-request" || kind == "logon-denied" {
            if let Some(i) = &icao {
                self.alias.retain(|_, v| v != i);
            }
        }
    }

    /// Drop what has been silent too long, trim to the cap, and return the
    /// rows most recently heard first.
    pub fn snapshot(&mut self, now: i64) -> Vec<HfdlAircraft> {
        self.rows.retain(|_, r| now - r.last_heard <= HFDL_AIRCRAFT_KEEP_S);
        if self.rows.len() > HFDL_AIRCRAFT_MAX {
            let mut by_age: Vec<(i64, String)> =
                self.rows.iter().map(|(k, r)| (r.last_heard, k.clone())).collect();
            by_age.sort();
            for (_, k) in by_age.into_iter().take(self.rows.len() - HFDL_AIRCRAFT_MAX) {
                self.rows.remove(&k);
            }
        }
        let mut v: Vec<HfdlAircraft> = self.rows.values().cloned().collect();
        v.sort_by_key(|r| std::cmp::Reverse(r.last_heard));
        v
    }

    fn row(&mut self, key: &str, unix: i64) -> &mut HfdlAircraft {
        self.rows.entry(key.to_owned()).or_insert_with(|| HfdlAircraft::new(key.to_owned(), unix))
    }
}

fn icao_key(icao: &str) -> String {
    format!("icao:{icao}")
}

fn alias_key(gs: u8, alias: u8) -> String {
    format!("gs{gs}:ac{alias}")
}

/// The key the UI's map files a fix under — [`sdroxide_types::HfdlFix::key`]
/// for the same position object, so a row can select its plot. Kept in step by
/// building the same type rather than re-spelling its rules.
fn map_key(p: &serde_json::Value) -> String {
    sdroxide_types::HfdlFix {
        lat: p.get("lat").and_then(|v| v.as_f64()).unwrap_or(0.0),
        lon: p.get("lon").and_then(|v| v.as_f64()).unwrap_or(0.0),
        aircraft_id: p.get("aircraft_id").and_then(|v| v.as_u64()).map(|v| v as u32),
        icao: p.get("icao").and_then(|v| v.as_str()).map(str::to_owned),
        flight: p
            .get("flight")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned),
    }
    .key()
}

fn set_if_text(slot: &mut Option<String>, v: &str) {
    let t = v.trim();
    if !t.is_empty() {
        *slot = Some(t.to_owned());
    }
}

/// Which aircraft, if any, an event is about.
fn who_of(e: &HfdlEvent) -> Option<Who> {
    let who = e.details.get("who");
    let dir = who.and_then(|w| w.get("dir")).and_then(|v| v.as_str());
    let gs = who
        .and_then(|w| w.get("gs_id"))
        .and_then(|v| v.as_u64())
        .filter(|&v| v < 128)
        .map(|v| v as u8);
    let s = |v: Option<&serde_json::Value>| v.and_then(|v| v.as_str()).map(str::to_owned);
    match e.kind.as_str() {
        // The logon exchange names the ICAO directly. The aircraft has no alias
        // on this station yet — whatever the envelope's id field holds there is
        // not one — so none is read from it.
        "logon-request" | "logon-resume" => {
            Some(Who { gs, alias: None, icao: s(e.details.get("icao")) })
        }
        "logon-confirm" => Some(Who {
            gs,
            alias: e.details.get("assigned_id").and_then(|v| v.as_u64()).map(|v| v as u8),
            icao: s(e.details.get("icao")),
        }),
        "logoff-request" | "logon-denied" => Some(Who {
            gs,
            alias: who.and_then(|w| w.get("aircraft_id")).and_then(|v| v.as_u64()).map(|v| v as u8),
            icao: s(e.details.get("icao")),
        }),
        // Everything else counts only as the aircraft's own transmission.
        _ if dir == Some("downlink") => Some(Who {
            gs,
            alias: who.and_then(|w| w.get("aircraft_id")).and_then(|v| v.as_u64()).map(|v| v as u8),
            icao: s(who.and_then(|w| w.get("icao"))),
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ev(kind: &str, details: serde_json::Value, snr: Option<f32>) -> HfdlEvent {
        HfdlEvent {
            kind: kind.into(),
            details,
            acars: None,
            fec_corrected: None,
            freq_skew_hz: None,
            snr_db: snr,
            raw: Vec::new(),
        }
    }

    fn down(gs: u64, alias: u64) -> serde_json::Value {
        json!({ "dir": "downlink", "gs_id": gs, "aircraft_id": alias })
    }

    fn perf(
        gs: u64,
        alias: u64,
        icao: Option<&str>,
        flight: &str,
        lat: f64,
        lon: f64,
    ) -> HfdlEvent {
        let mut who = down(gs, alias);
        if let Some(i) = icao {
            who["icao"] = json!(i);
        }
        let mut pos = json!({ "lat": lat, "lon": lon, "flight": flight, "aircraft_id": alias });
        if let Some(i) = icao {
            pos["icao"] = json!(i);
        }
        ev(
            "performance-data",
            json!({ "gs_id": gs, "flight": flight, "lat": lat, "lon": lon, "who": who, "position": pos }),
            Some(12.0),
        )
    }

    fn confirm(gs: u64, alias: u64, icao: &str) -> HfdlEvent {
        ev(
            "logon-confirm",
            json!({ "icao": icao, "assigned_id": alias, "who": { "dir": "uplink", "gs_id": gs } }),
            Some(9.0),
        )
    }

    #[test]
    fn a_downlink_makes_an_aircraft_with_its_flight_and_position() {
        let mut b = AircraftBook::new();
        b.observe(&perf(4, 12, None, "UAL012", 40.5, -50.2), 8_912, 1_000);
        let v = b.snapshot(1_010);
        assert_eq!(v.len(), 1);
        let a = &v[0];
        assert_eq!(a.key, "gs4:ac12");
        assert_eq!(
            (a.flight.as_deref(), a.aircraft_id, a.gs_id),
            (Some("UAL012"), Some(12), Some(4))
        );
        assert_eq!(a.gs.as_deref(), Some("Riverhead, New York"));
        assert_eq!(
            (a.lat, a.lon, a.fix_at, a.freq_khz, a.msgs),
            (Some(40.5), Some(-50.2), 1_000, 8_912, 1)
        );
        assert_eq!(a.snr_db, Some(12.0));
        // The map files the fix under the alias, as HfdlFix::key does.
        assert_eq!(a.map_key.as_deref(), Some("ac:12"));
    }

    #[test]
    fn the_logon_confirm_turns_an_alias_row_into_an_icao_row() {
        let mut b = AircraftBook::new();
        b.observe(&perf(4, 12, None, "UAL012", 40.0, -50.0), 8_912, 1_000);
        b.observe(&perf(4, 12, None, "UAL012", 40.1, -50.1), 8_912, 1_100);
        assert_eq!(b.snapshot(1_100)[0].msgs, 2);
        // The logon (heard late) resolves the alias.
        b.observe(&confirm(4, 12, "A1B2C3"), 8_912, 1_200);
        let v = b.snapshot(1_200);
        assert_eq!(v.len(), 1, "one aircraft, not an alias row and an ICAO row: {v:?}");
        let a = &v[0];
        assert_eq!(a.key, "icao:A1B2C3");
        assert_eq!(a.icao.as_deref(), Some("A1B2C3"));
        assert_eq!(a.flight.as_deref(), Some("UAL012"));
        assert_eq!((a.first_heard, a.last_heard, a.msgs), (1_000, 1_200, 3));
        assert_eq!(a.lat, Some(40.1), "the newest fix survives the merge");
        // Later downlinks on the alias reach the ICAO row without xng's help.
        b.observe(&perf(4, 12, None, "UAL012", 40.2, -50.2), 8_912, 1_300);
        let v = b.snapshot(1_300);
        assert_eq!((v.len(), v[0].msgs, v[0].lat), (1, 4, Some(40.2)));
    }

    #[test]
    fn the_same_alias_on_another_station_is_another_aircraft() {
        let mut b = AircraftBook::new();
        b.observe(&perf(4, 12, None, "UAL012", 40.0, -50.0), 8_912, 1_000);
        b.observe(&perf(7, 12, None, "BAW099", 51.0, -20.0), 8_825, 1_001);
        assert_eq!(b.snapshot(1_001).len(), 2);
    }

    #[test]
    fn a_resolved_downlink_goes_straight_to_the_icao_row() {
        let mut b = AircraftBook::new();
        b.observe(&perf(4, 12, Some("A1B2C3"), "UAL012", 40.0, -50.0), 8_912, 1_000);
        let v = b.snapshot(1_000);
        assert_eq!((v[0].key.as_str(), v[0].icao.as_deref()), ("icao:A1B2C3", Some("A1B2C3")));
        // …and the map key tracks what the fix itself would be filed under.
        assert_eq!(v[0].map_key.as_deref(), Some("icao:A1B2C3"));
    }

    #[test]
    fn an_acars_message_gives_the_registration() {
        let mut b = AircraftBook::new();
        let octets = xng_acars::block::build(
            '2',
            "N123AB",
            None,
            "H1",
            '4',
            Some("M01A"),
            Some("UAL012"),
            "HI",
            false,
        );
        let mut e = ev("acars", json!({ "who": down(4, 12) }), Some(10.0));
        e.acars = xng_acars::block::parse(&octets);
        assert!(e.acars.is_some());
        b.observe(&e, 8_912, 1_000);
        let a = &b.snapshot(1_000)[0];
        assert_eq!(
            (a.registration.as_deref(), a.flight.as_deref()),
            (Some("N123AB"), Some("UAL012"))
        );
        assert_eq!(a.fix_at, 0, "an ACARS message alone is not a position");
        assert_eq!(a.map_key, None);
    }

    #[test]
    fn logging_off_marks_the_aircraft_and_unbinds_the_alias() {
        let mut b = AircraftBook::new();
        b.observe(&confirm(4, 12, "A1B2C3"), 8_912, 1_000);
        b.observe(&perf(4, 12, None, "UAL012", 40.0, -50.0), 8_912, 1_050);
        assert!(!b.snapshot(1_050)[0].logged_off);
        let off = ev(
            "logoff-request",
            json!({ "icao": "A1B2C3", "reason": 0, "who": down(4, 12) }),
            None,
        );
        b.observe(&off, 8_912, 1_100);
        assert!(b.snapshot(1_100)[0].logged_off);
        // The alias is free again: a downlink on it is no longer that aircraft.
        b.observe(&perf(4, 12, None, "XYZ1", 10.0, 10.0), 8_912, 1_200);
        let v = b.snapshot(1_200);
        assert_eq!(v.len(), 2, "{v:?}");
        // …and the next logon brings the first aircraft back.
        b.observe(&confirm(4, 20, "A1B2C3"), 8_912, 1_300);
        let a = b.snapshot(1_300).into_iter().find(|a| a.key == "icao:A1B2C3").unwrap();
        assert!(!a.logged_off);
    }

    #[test]
    fn squitters_and_unaddressed_uplinks_are_not_aircraft() {
        let mut b = AircraftBook::new();
        b.observe(&ev("squitter", json!({ "gs_id": 4, "freqs_in_use": 1 }), None), 8_912, 1);
        b.observe(
            &ev("acars", json!({ "who": { "dir": "uplink", "gs_id": 4, "aircraft": [{ "aircraft_id": 12, "lpdus": 1 }] } }), None),
            8_912,
            2,
        );
        assert!(b.snapshot(3).is_empty());
    }

    #[test]
    fn the_list_is_newest_first_and_forgets_the_long_silent() {
        let mut b = AircraftBook::new();
        b.observe(&perf(4, 1, None, "OLD1", 0.0, 0.0), 8_912, 100);
        b.observe(&perf(4, 2, None, "NEW2", 0.0, 0.0), 8_912, 500);
        let v = b.snapshot(600);
        assert_eq!(
            v.iter().map(|a| a.flight.clone().unwrap()).collect::<Vec<_>>(),
            ["NEW2", "OLD1"]
        );
        // Three hours after OLD1 it is gone; NEW2 (heard later) stays.
        let v = b.snapshot(100 + HFDL_AIRCRAFT_KEEP_S + 1);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].flight.as_deref(), Some("NEW2"));
    }

    #[test]
    fn the_list_is_capped_and_drops_the_longest_silent_first() {
        let mut b = AircraftBook::new();
        let n = HFDL_AIRCRAFT_MAX as u64 + 20;
        for i in 0..n {
            // Distinct aliases across stations 1..=8 and ids 0..=255.
            b.observe(&perf(1 + i / 200, i % 200, None, "F", 0.0, 0.0), 8_912, 1_000 + i as i64);
        }
        let v = b.snapshot(1_000 + n as i64);
        assert_eq!(v.len(), HFDL_AIRCRAFT_MAX);
        assert_eq!(v[0].last_heard, 1_000 + n as i64 - 1, "the newest is kept");
        assert!(v.iter().all(|a| a.last_heard >= 1_020), "the 20 oldest went");
    }

    #[test]
    fn the_last_message_is_the_last_acars_text_and_survives_a_position_report() {
        let mut b = AircraftBook::new();
        let mk = |label: &str, text: &str| {
            let octets = xng_acars::block::build(
                '2',
                "N123AB",
                None,
                label,
                '4',
                Some("M01A"),
                Some("UAL012"),
                text,
                false,
            );
            let mut e = ev("acars", json!({ "who": down(4, 12) }), Some(10.0));
            e.acars = xng_acars::block::parse(&octets);
            e
        };
        b.observe(&perf(4, 12, None, "UAL012", 40.0, -50.0), 8_912, 1_000);
        assert_eq!(b.snapshot(1_000)[0].last_text, None, "no ACARS yet");
        assert_eq!(b.snapshot(1_000)[0].last_kind, "performance-data");
        b.observe(&mk("H1", "REQUEST\r\nHIGHER"), 8_912, 1_010);
        let a = b.snapshot(1_010).remove(0);
        assert_eq!(a.last_text.as_deref(), Some("[H1] REQUEST ⏎ HIGHER"));
        assert_eq!(a.last_kind, "acars");
        // A later position report changes the kind, not the message.
        b.observe(&perf(4, 12, None, "UAL012", 40.1, -50.1), 8_912, 1_020);
        let a = b.snapshot(1_020).remove(0);
        assert_eq!(a.last_text.as_deref(), Some("[H1] REQUEST ⏎ HIGHER"));
        assert_eq!(a.last_kind, "performance-data");
        // A block with no text falls back to what the decoder made of it …
        b.observe(&mk("Q2", ""), 8_912, 1_030);
        assert_eq!(b.snapshot(1_030)[0].last_text.as_deref(), Some("[Q2] ETA Report"));
        // … and one with neither text nor meaning leaves the last message standing.
        b.observe(&mk("ZZ", ""), 8_912, 1_035);
        assert_eq!(b.snapshot(1_035)[0].last_text.as_deref(), Some("[Q2] ETA Report"));
        // … and a long text is cut.
        b.observe(&mk("H1", &"X".repeat(400)), 8_912, 1_040);
        let t = b.snapshot(1_040)[0].last_text.clone().unwrap();
        assert_eq!(t.chars().count(), HFDL_LAST_TEXT_MAX);
        assert!(t.ends_with('…'));
    }

    fn perf_at(lat: f64, lon: f64, utc_s: u32) -> HfdlEvent {
        let mut e = perf(4, 12, Some("A1B2C3"), "UAL012", lat, lon);
        e.details["position"]["utc_s"] = json!(utc_s);
        e
    }

    #[test]
    fn course_and_speed_come_from_the_last_two_fixes_by_the_aircrafts_clock() {
        let mut b = AircraftBook::new();
        b.observe(&perf_at(40.0, -30.0, 36_000), 8_912, 1_000);
        assert_eq!(b.snapshot(1_000)[0].calc_speed_kt, None, "one fix says nothing about motion");
        // Ten minutes later by the aircraft's clock, one degree north — though
        // the frame was decoded only five minutes after the first.
        b.observe(&perf_at(41.0, -30.0, 36_600), 8_912, 1_300);
        let a = &b.snapshot(1_300)[0];
        let kt = a.calc_speed_kt.expect("a speed");
        assert!((kt - 360.0).abs() < 3.0, "timed by the reports, not the decodes: {kt}");
        let trk = a.calc_track_deg.unwrap();
        assert!(trk < 1.0 || trk > 359.0, "{trk}");
        // A repeat of the same fix (a second record from the same minute)
        // leaves the estimate alone.
        b.observe(&perf_at(41.0, -30.0, 36_620), 8_912, 1_320);
        assert_eq!(b.snapshot(1_320)[0].calc_speed_kt, Some(kt));
    }

    #[test]
    fn without_report_times_the_decode_times_are_used() {
        let mut b = AircraftBook::new();
        b.observe(&perf(4, 12, None, "UAL012", 40.0, -30.0), 8_912, 1_000);
        b.observe(&perf(4, 12, None, "UAL012", 40.0, -31.0), 8_912, 1_600);
        let a = &b.snapshot(1_600)[0];
        assert!((a.calc_track_deg.unwrap() - 270.0).abs() < 1.0);
        // A degree of longitude at 40° N is about 46 NM: 276 kt over ten minutes.
        let kt = a.calc_speed_kt.unwrap();
        assert!((kt - 276.0).abs() < 4.0, "{kt}");
    }
}
