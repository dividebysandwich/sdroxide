//! Which decoded stations are currently "up", and how brightly — plus the last
//! hour of them, which the globe replays as a time-lapse.
//!
//! Two views draw this: the flat FT8 panel map and the 3D globe. They have to
//! agree — a station visible on one and absent from the other reads as a bug —
//! so the rule lives here once rather than in each of them. (The globe draws
//! *more*: the whole hour of history behind the live set, which is the one
//! thing a flat panel-sized map has no room for.)
//!
//! Ages use egui's frame time, which is monotonic and works on both targets;
//! `slot_utc` only decides whether a decode is *newer* than the one already
//! recorded for that station. The history is the exception: a replay of the
//! last hour is a wall-clock question, so it is stamped and pruned in UTC.
//!
//! Each station carries the callsign it was heard under, which is what the
//! globe writes beside its dot. The flat map has no room for it and ignores it,
//! but it is kept here rather than looked up again per view: whichever feed
//! placed the dot is the only thing that knows who was standing there — WSPR's
//! own report names one of two different stations depending on which way round
//! it was heard.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use sdroxide_types::{Decode, WsprSpot};

use crate::solar3d::DigiTraffic;

/// A decoded station's dot fades over this many seconds since it was last
/// heard, then expires.
pub const STATION_FADE_S: f64 = 120.0;

/// How far back the activity history reaches — the span the globe's time-lapse
/// can replay.
pub const HISTORY_S: i64 = 3600;

/// The oldest slot still inside the replay window.
fn cutoff(now_utc: i64) -> i64 {
    now_utc - HISTORY_S
}

/// One located decode, stamped with the slot it came from. The unit the globe's
/// time-lapse replays.
#[derive(Clone, Debug, PartialEq)]
pub struct DigiHit {
    pub lat: f64,
    pub lon: f64,
    /// Unix seconds at the start of the slot it was decoded in.
    pub slot_utc: i64,
    /// Who was heard there, when the mode carries a callsign. Shared rather
    /// than copied: the same call recurs in slot after slot, and the whole
    /// history is republished into the globe every frame.
    pub call: Option<Arc<str>>,
}

/// One station currently on the map: where it is, how brightly, and who it is.
#[derive(Clone, Debug, PartialEq)]
pub struct DigiStation {
    pub lat: f64,
    pub lon: f64,
    /// 1.0 → 0.0 as the decode ages out.
    pub fade: f32,
    /// The callsign heard there. `None` for a mode that placed a station
    /// without naming one.
    pub call: Option<Arc<str>>,
    /// The band the reception was on, where the feed carries a frequency.
    ///
    /// Only WSPR fills this in, and only because it is the one mode whose map
    /// routinely mixes bands: a hopping beacon puts 40 m, 20 m and 10 m dots on
    /// one picture, and undifferentiated dots cannot say which path opened
    /// (issue #316). A mode that stays on one band all evening gains nothing
    /// from a hue, so it leaves this `None` and keeps the neutral dot.
    pub band: Option<sdroxide_types::Band>,
}

/// What is known about one station between decodes.
struct Seen {
    /// Newest slot it has been heard in.
    slot: i64,
    /// Frame time it was last heard at — what the fade counts from.
    at: f64,
    lat: f64,
    lon: f64,
    call: Option<Arc<str>>,
    band: Option<sdroxide_types::Band>,
}

/// Station → what was last heard from it, plus the last hour of located
/// decodes.
pub struct DigiStations {
    /// Keyed by callsign, falling back to the grid square for a decode that
    /// carries no call.
    ///
    /// A station is its callsign, not its square: two operators in the same
    /// square are two stations rather than one dot that cannot decide what it
    /// is called, and a station that corrects its locator moves instead of
    /// leaving a ghost behind at the old one.
    seen: HashMap<String, Seen>,
    /// Oldest first. Behind an `Arc` because every frame republishes it into
    /// the globe and an hour of a busy band is a few thousand entries.
    history: Arc<Vec<DigiHit>>,
    /// The (station, slot) pairs already in `history`, so a decode that stays
    /// in the caller's rolling list for many frames is recorded exactly once.
    recorded: HashSet<(String, i64)>,
    /// How long a station stays lit after it was last heard.
    fade_s: f64,
}

impl Default for DigiStations {
    fn default() -> Self {
        DigiStations {
            seen: HashMap::new(),
            history: Arc::new(Vec::new()),
            recorded: HashSet::new(),
            fade_s: STATION_FADE_S,
        }
    }
}

impl DigiStations {
    /// Set how long a station stays lit after it was last heard.
    ///
    /// FT8 fills every slot, so [`STATION_FADE_S`] of silence means gone. JS8
    /// is deliberately sparse — the mode's own convention is a heartbeat every
    /// ten or fifteen minutes — and a map that emptied between them would be
    /// blank almost all the time, which reads as the feature not working rather
    /// than as the band being quiet.
    pub fn set_fade_s(&mut self, seconds: f64) {
        self.fade_s = seconds.max(1.0);
    }

    /// Fold in this frame's decode list and drop anything that has expired.
    /// `now_utc` is the wall clock, which only the history uses.
    ///
    /// Idempotent within a slot: re-observing the same decodes does not refresh
    /// a dot, because only a *newer* `slot_utc` counts as hearing the station
    /// again. That is what lets both the panel map and the globe call this with
    /// whatever list they happen to be holding.
    pub fn observe(&mut self, decodes: &[Decode], now_t: f64, now_utc: i64) {
        let mut fresh: Vec<DigiHit> = Vec::new();
        for d in decodes {
            let Some(grid) = d.grid.as_deref() else { continue };
            self.note(d.from.as_deref(), grid, None, d.slot_utc, now_t, now_utc, &mut fresh);
        }
        self.retire(fresh, now_t, cutoff(now_utc));
    }

    /// Fold in one located station. Shared by both `observe` paths, so the two
    /// cannot disagree about what counts as hearing a station again.
    fn note(
        &mut self,
        call: Option<&str>,
        grid: &str,
        band: Option<sdroxide_types::Band>,
        slot_utc: i64,
        now_t: f64,
        now_utc: i64,
        fresh: &mut Vec<DigiHit>,
    ) {
        let Some((lat, lon)) = sdroxide_types::grid_to_latlon(grid) else { return };
        let call = call.filter(|c| !c.is_empty());
        let key = call.unwrap_or(grid);

        let e = self.seen.entry(key.to_string()).or_insert(Seen {
            slot: i64::MIN,
            at: now_t,
            lat,
            lon,
            call: None,
            band: None,
        });
        if slot_utc > e.slot {
            // Heard again → the dot returns to full brightness, and moves if the
            // station has since given a better locator.
            e.slot = slot_utc;
            e.at = now_t;
            e.lat = lat;
            e.lon = lon;
            // Only re-allocate when the name actually changed: on a busy band
            // this runs a few hundred times a slot for calls that never move.
            if e.call.as_deref() != call {
                e.call = call.map(Arc::from);
            }
            // The band of the *newest* reception, so a beacon followed across a
            // hop moves to the hue of the band it is on now rather than the one
            // it was first heard on.
            e.band = band;
        }
        let hit_call = e.call.clone();

        // A slot outside the window is either older than the replay reaches or
        // stamped by a clock we should not trust; either way it has no place on
        // a time axis.
        if slot_utc <= cutoff(now_utc) || slot_utc > now_utc + 300 {
            return;
        }
        let recorded = (key.to_string(), slot_utc);
        if !self.recorded.contains(&recorded) {
            self.recorded.insert(recorded);
            fresh.push(DigiHit { lat, lon, slot_utc, call: hit_call });
        }
    }

    /// Fold in WSPR spots the same way [`DigiStations::observe`] folds in
    /// decodes.
    ///
    /// Which end goes on the map is the whole of the difference. A spot we
    /// decoded places the *transmitter*; a spot that came back from WSPRnet
    /// saying somebody heard *us* places the **reporter**, because the
    /// transmitter in that report is this station and a dot on our own QTH says
    /// nothing. Either way the far end of the path is what is drawn, which is
    /// what makes the two sets one picture rather than two.
    pub fn observe_wspr(&mut self, spots: &[WsprSpot], now_t: f64, now_utc: i64) {
        let mut fresh: Vec<DigiHit> = Vec::new();
        for s in spots {
            // Both ends of the report follow the same rule: the callsign named
            // is the one standing at the grid that gets the dot.
            let (grid, call) = if s.is_heard_by_other() {
                (s.reporter_grid.as_deref(), s.reporter.as_deref())
            } else {
                (s.grid.as_deref(), Some(s.call.as_str()))
            };
            let Some(grid) = grid.filter(|g| !g.is_empty()) else { continue };
            // `Gen` means the frequency fell outside every band in the
            // station's region, which is not a hue — a WSPRnet report from
            // another region can land there.
            let band = Some(sdroxide_types::Band::containing(s.freq_hz))
                .filter(|b| *b != sdroxide_types::Band::Gen);
            self.note(call, grid, band, s.slot_utc, now_t, now_utc, &mut fresh);
        }
        self.retire(fresh, now_t, cutoff(now_utc));
    }

    /// Drop what has expired and append what is new. Shared by both `observe`
    /// paths so the two can never disagree about when a dot goes out.
    fn retire(&mut self, mut fresh: Vec<DigiHit>, now_t: f64, cutoff: i64) {
        let fade = self.fade_s;
        self.seen.retain(|_, e| now_t - e.at < fade);

        let expired = self.history.first().is_some_and(|h| h.slot_utc <= cutoff);
        if !fresh.is_empty() || expired {
            // The decode list arrives newest-first, so sort what came in before
            // appending: the history is oldest-first, and the globe walks it
            // backwards to take the newest arcs.
            fresh.sort_by_key(|h| h.slot_utc);
            let h = Arc::make_mut(&mut self.history);
            if expired {
                h.retain(|e| e.slot_utc > cutoff);
            }
            h.extend(fresh);
            if expired {
                self.recorded.retain(|(_, slot)| *slot > cutoff);
            }
        }
    }

    /// The last hour of located decodes, oldest first.
    pub fn history(&self) -> Arc<Vec<DigiHit>> {
        Arc::clone(&self.history)
    }

    /// Located stations with their 1.0 → 0.0 fade.
    pub fn stations(&self, now_t: f64) -> Vec<DigiStation> {
        self.seen
            .values()
            .filter_map(|e| {
                let fade = (1.0 - (now_t - e.at) / self.fade_s).clamp(0.0, 1.0) as f32;
                (fade > 0.0).then(|| DigiStation {
                    lat: e.lat,
                    lon: e.lon,
                    fade,
                    call: e.call.clone(),
                    band: e.band,
                })
            })
            .collect()
    }

    /// The globe's view of the same set, plus the QSO in progress.
    pub fn traffic(
        &self,
        now_t: f64,
        dx_grid: Option<&str>,
        preview: Option<(f64, f64)>,
        transmitting: bool,
    ) -> DigiTraffic {
        DigiTraffic {
            stations: self.stations(now_t),
            dx: dx_grid.and_then(sdroxide_types::grid_to_latlon),
            // A QSO partner needs no label: its callsign is already on the panel.
            // The caller fills this in when it redirects the arc at a named
            // transmitter instead.
            dx_label: None,
            // Same division as `dx_label`: this store knows who is on the air,
            // not what is being said to them. The caller fills it in from the
            // sequencer's state.
            qso: None,
            preview,
            transmitting,
            history: self.history(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A wall clock for the tests, so slots land inside the history window.
    const NOW: i64 = 1_784_937_600;

    /// One callsign per grid, so each fixture square is a distinct station —
    /// which is what the store keys on. Both feeds derive it the same way, so a
    /// grid heard on WSPR and on FT8 is the same operator, as it would be on
    /// the air.
    fn call_in(grid: &str) -> String {
        format!("TE{}", grid.to_ascii_uppercase())
    }

    /// `slot` is seconds from [`NOW`].
    fn decode(grid: &str, slot: i64) -> Decode {
        Decode {
            slot_utc: NOW + slot,
            snr_db: -10,
            dt: 0.2,
            audio_hz: 1000.0,
            message: format!("CQ TEST {grid}"),
            to: None,
            from: Some(call_in(grid)),
            grid: Some(grid.into()),
            is_cq: true,
            cq_to: None,
            free_text: false,
            rr73_to: None,
        }
    }

    /// `observe` with the wall clock implied by the decodes themselves, which
    /// is what the fade tests care about.
    fn observe(s: &mut DigiStations, decodes: &[Decode], now_t: f64) {
        let now_utc = decodes.iter().map(|d| d.slot_utc).max().unwrap_or(NOW);
        s.observe(decodes, now_t, now_utc);
    }

    #[test]
    fn a_station_fades_out_over_two_minutes_and_then_expires() {
        let mut s = DigiStations::default();
        observe(&mut s, &[decode("FN42", 100)], 0.0);
        assert_eq!(s.stations(0.0)[0].fade, 1.0, "a fresh decode is not at full brightness");

        let half = s.stations(STATION_FADE_S / 2.0);
        assert!((half[0].fade - 0.5).abs() < 1e-6, "half-way fade was {}", half[0].fade);

        // Past the window it is gone entirely, not merely transparent: it must
        // also drop out of the flat map's zoom fit.
        observe(&mut s, &[], STATION_FADE_S + 1.0);
        assert!(s.stations(STATION_FADE_S + 1.0).is_empty());
    }

    #[test]
    fn hearing_a_station_again_restores_full_brightness() {
        let mut s = DigiStations::default();
        observe(&mut s, &[decode("FN42", 100)], 0.0);
        observe(&mut s, &[decode("FN42", 115)], 60.0);
        assert_eq!(s.stations(60.0)[0].fade, 1.0);
    }

    /// The panel map and the globe both call `observe` with whatever decode
    /// list they hold, which is usually the same one twice. Re-observing must
    /// not keep a station alive forever.
    #[test]
    fn re_observing_the_same_slot_does_not_refresh_the_fade() {
        let mut s = DigiStations::default();
        let d = [decode("FN42", 100)];
        observe(&mut s, &d, 0.0);
        observe(&mut s, &d, 60.0);
        let f = s.stations(60.0)[0].fade;
        assert!((f - 0.5).abs() < 1e-6, "re-observing reset the fade to {f}");
    }

    #[test]
    fn a_decode_without_a_grid_places_nothing() {
        let mut s = DigiStations::default();
        let mut d = decode("FN42", 100);
        d.grid = None;
        observe(&mut s, &[d], 0.0);
        assert!(s.stations(0.0).is_empty());

        // Nor does a grid that does not decode to a position.
        observe(&mut s, &[decode("ZZ99zz", 100)], 0.0);
        assert!(s.stations(0.0).is_empty());
        assert!(s.history().is_empty(), "an unplaceable grid made it into the history");
    }

    /// The history is what the globe replays, so it keeps one entry per station
    /// per slot for an hour — well past the two minutes a live dot survives.
    #[test]
    fn the_history_keeps_an_hour_of_decodes_one_per_station_and_slot() {
        let mut s = DigiStations::default();
        let d = [decode("FN42", 0), decode("JN88", 0)];
        s.observe(&d, 0.0, NOW);
        // The same list again, frames later: already recorded, not duplicated.
        s.observe(&d, 5.0, NOW + 5);
        s.observe(&[decode("FN42", 15)], 15.0, NOW + 15);
        let h = s.history();
        assert_eq!(h.len(), 3, "history is {h:?}");
        assert!(h.windows(2).all(|w| w[0].slot_utc <= w[1].slot_utc), "history is not in order");

        // Long past the live fade, the hour is still there…
        s.observe(&[], 600.0, NOW + 600);
        assert_eq!(s.history().len(), 3);
        assert!(s.stations(600.0).is_empty(), "a live dot outlived its fade");

        // …and then it ages out of the window one slot at a time.
        s.observe(&[], 4000.0, NOW + HISTORY_S + 10);
        assert_eq!(s.history().len(), 1, "only the last slot is still inside the hour");
        s.observe(&[], 4100.0, NOW + HISTORY_S + 100);
        assert!(s.history().is_empty());
    }

    /// A slot stamped in the future, or older than the window, is not a point
    /// on the replay's time axis — most likely a clock that cannot be trusted.
    #[test]
    fn history_ignores_slots_outside_the_window() {
        let mut s = DigiStations::default();
        s.observe(&[decode("FN42", -HISTORY_S - 60), decode("JN88", 3600)], 0.0, NOW);
        assert!(s.history().is_empty());
        // The live dots are unaffected: those are frame-time bookkeeping.
        assert_eq!(s.stations(0.0).len(), 2);
    }

    /// JS8 hears a station every ten or fifteen minutes, not every slot, so it
    /// widens the window rather than watching the map empty between beacons.
    #[test]
    fn a_widened_fade_keeps_a_station_lit_past_the_default() {
        let mut s = DigiStations::default();
        s.set_fade_s(900.0);
        observe(&mut s, &[decode("FN42", 100)], 0.0);
        assert!(s.stations(STATION_FADE_S + 1.0).len() == 1, "gone at the default fade");
        let half = s.stations(450.0);
        assert!((half[0].fade - 0.5).abs() < 1e-6, "half-way fade was {}", half[0].fade);
        observe(&mut s, &[], 901.0);
        assert!(s.stations(901.0).is_empty(), "it still has to expire eventually");
    }

    fn wspr(grid: &str, slot: i64) -> WsprSpot {
        WsprSpot {
            slot_utc: NOW + slot,
            call: call_in(grid),
            grid: Some(grid.into()),
            power_dbm: 37,
            freq_hz: 14_097_100.0,
            snr_db: -24,
            dt: 0.3,
            drift_hz: 0.0,
            reporter: None,
            reporter_grid: None,
        }
    }

    /// A beacon we decoded places the transmitter, and it ages out the same way
    /// a decode does — the two views have to agree about WSPR too.
    #[test]
    fn a_wspr_beacon_lights_its_own_grid_and_fades_like_a_decode() {
        let mut s = DigiStations::default();
        s.observe_wspr(&[wspr("FN42", 0)], 0.0, NOW);
        assert_eq!(s.stations(0.0)[0].fade, 1.0);
        assert_eq!(s.history().len(), 1);
        let half = s.stations(STATION_FADE_S / 2.0);
        assert!((half[0].fade - 0.5).abs() < 1e-6, "half-way fade was {}", half[0].fade);
    }

    /// A report of *us* being heard places the station that heard us. Placing
    /// the transmitter would put a dot on our own QTH, which says nothing about
    /// the path and is exactly the end we already know.
    #[test]
    fn a_report_of_us_places_the_reporter_not_our_own_grid() {
        let mut s = DigiStations::default();
        let heard = WsprSpot {
            call: "K1ABC".into(),
            grid: Some("FN42".into()),
            reporter: Some("G0XYZ".into()),
            reporter_grid: Some("IO91".into()),
            ..wspr("FN42", 0)
        };
        s.observe_wspr(&[heard], 0.0, NOW);
        let placed = s.stations(0.0);
        assert_eq!(placed.len(), 1);
        let (lat, lon) = (placed[0].lat, placed[0].lon);
        // IO91 is southern England, not FN42 (New England).
        assert!(lat > 50.0 && lat < 53.0 && lon > -3.0 && lon < 1.0, "placed at {lat},{lon}");
        // …and it is named after the station standing there, not after us.
        assert_eq!(placed[0].call.as_deref(), Some("G0XYZ"));
    }

    /// The two feeds share one history, so a beacon in the same grid as an FT8
    /// decode in the same slot must not be counted twice.
    #[test]
    fn wspr_and_decodes_share_one_history_without_double_counting() {
        let mut s = DigiStations::default();
        s.observe(&[decode("FN42", 0)], 0.0, NOW);
        s.observe_wspr(&[wspr("FN42", 0)], 1.0, NOW);
        assert_eq!(s.history().len(), 1, "the same station and slot was recorded twice");
    }

    /// A hopping beacon puts several bands on one map, so each dot carries the
    /// band it was heard on for the view to colour it by (issue #316) — and it
    /// follows the beacon across a hop rather than sticking on the band it was
    /// first heard on. A decode carries none: FT8 sits on one band all evening,
    /// and a hue that never varies is not information.
    #[test]
    fn a_wspr_dot_carries_the_band_it_was_last_heard_on() {
        use sdroxide_types::Band;
        let mut s = DigiStations::default();
        s.observe_wspr(&[wspr("FN42", 0)], 0.0, NOW);
        assert_eq!(s.stations(0.0)[0].band, Some(Band::M20));

        // The same beacon, a slot later, after the hop.
        let hopped = WsprSpot { freq_hz: 7_040_100.0, ..wspr("FN42", 120) };
        s.observe_wspr(&[hopped], 0.0, NOW + 120);
        assert_eq!(s.stations(0.0)[0].band, Some(Band::M40));

        let mut d = DigiStations::default();
        d.observe(&[decode("FN42", 0)], 0.0, NOW);
        assert_eq!(d.stations(0.0)[0].band, None);
    }

    /// A WSPRnet report can name a frequency in no band this station's region
    /// has. `Gen` is not a colour, so nothing is claimed for it.
    #[test]
    fn a_frequency_in_no_band_gets_no_colour() {
        let mut s = DigiStations::default();
        s.observe_wspr(&[WsprSpot { freq_hz: 9_500_000.0, ..wspr("FN42", 0) }], 0.0, NOW);
        assert_eq!(s.stations(0.0)[0].band, None);
    }

    #[test]
    fn a_report_without_a_reporter_grid_places_nothing() {
        let mut s = DigiStations::default();
        let heard =
            WsprSpot { reporter: Some("G0XYZ".into()), reporter_grid: None, ..wspr("FN42", 0) };
        s.observe_wspr(&[heard], 0.0, NOW);
        assert!(s.stations(0.0).is_empty(), "placed a reporter with no locator");
        assert!(s.history().is_empty());
    }

    #[test]
    fn the_dx_station_comes_from_its_grid_not_from_the_decode_list() {
        let mut s = DigiStations::default();
        observe(&mut s, &[decode("FN42", 100)], 0.0);
        let t = s.traffic(0.0, Some("JN88"), None, true);
        assert_eq!(t.stations.len(), 1);
        assert!(t.transmitting);
        let (lat, lon) = t.dx.expect("JN88 is a valid grid");
        assert!(lat > 40.0 && lat < 50.0 && lon > 10.0 && lon < 20.0, "JN88 at {lat},{lon}");
    }

    /// The whole point of the callsign travelling this far: the globe labels
    /// its dots with it, live and in the replay both.
    #[test]
    fn a_station_carries_the_callsign_that_was_heard() {
        let mut s = DigiStations::default();
        s.observe(&[decode("FN42", 0)], 0.0, NOW);
        assert_eq!(s.stations(0.0)[0].call.as_deref(), Some("TEFN42"));
        assert_eq!(s.history()[0].call.as_deref(), Some("TEFN42"));
    }

    /// Two operators in one square are two stations. Keying on the square would
    /// give one dot that could not decide what it was called, and would drop
    /// whichever of them was decoded second.
    #[test]
    fn two_stations_in_one_grid_are_two_stations() {
        let mut s = DigiStations::default();
        let mut b = decode("FN42", 0);
        b.from = Some("K1ABC".into());
        s.observe(&[decode("FN42", 0), b], 0.0, NOW);

        let mut calls: Vec<String> =
            s.stations(0.0).iter().filter_map(|d| d.call.as_deref().map(str::to_string)).collect();
        calls.sort();
        assert_eq!(calls, ["K1ABC", "TEFN42"]);
        assert_eq!(s.history().len(), 2, "one of them was dropped from the replay");
    }

    /// A mode that places a station without naming one still gets a dot — it
    /// simply goes unlabelled.
    #[test]
    fn a_decode_without_a_callsign_still_places_a_dot() {
        let mut s = DigiStations::default();
        let mut d = decode("FN42", 0);
        d.from = None;
        s.observe(&[d], 0.0, NOW);
        let placed = s.stations(0.0);
        assert_eq!(placed.len(), 1);
        assert!(placed[0].call.is_none());
    }

    /// A station that corrects its locator moves. Keeping the old dot alive
    /// until it faded would show one operator in two places at once.
    #[test]
    fn a_station_that_reports_a_new_grid_moves_rather_than_splitting() {
        let mut s = DigiStations::default();
        s.observe(&[decode("FN42", 0)], 0.0, NOW);
        let mut moved = decode("JN88", 15);
        moved.from = Some(call_in("FN42"));
        s.observe(&[moved], 1.0, NOW + 15);

        let placed = s.stations(1.0);
        assert_eq!(placed.len(), 1, "the station was left behind in two squares");
        // JN88 is central Europe, not New England.
        assert!(placed[0].lon > 10.0, "still at {},{}", placed[0].lat, placed[0].lon);
    }
}
