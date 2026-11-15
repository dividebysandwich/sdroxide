//! The ground-station book: what the decoder has learned about each HFDL
//! ground station, kept for as long as the worker lives rather than for as long
//! as the log's 64-entry window remembers it.
//!
//! Three kinds of event feed it.
//!
//! * A **squitter** is a station announcing itself. It says which of the
//!   station's channels are in use (a 20-bit bitmap) and carries the burst's
//!   SNR, so it is what makes a station "heard", and where.
//! * An **uplink** MPDU is a station talking to an aircraft; it is a
//!   transmission of the station's too, so it counts as heard (but it carries
//!   no channel bitmap).
//! * A **system table** (`systable-complete`) is the network's own list of its
//!   stations — position and frequencies — and is what turns a bitmap's bit *n*
//!   into the station's *n*-th frequency. The stations broadcast it only now
//!   and then, so the engine saves it and seeds the book with it next time
//!   ([`StationBook::seed`]).
//!
//! Aircraft downlinks name a station too (the one the aircraft is talking to),
//! but that is the aircraft's transmission, heard through the aircraft's
//! reach, not evidence the *station* is on this channel; they do not count.

use std::collections::BTreeMap;

use sdroxide_types::{HfdlStation, HfdlStationFreq};
use xng_mode_hfdl::pdu::{HfdlEvent, gs_name};
use xng_mode_hfdl::systable::{SystemTable, version_is_newer};

/// The squitter's frequency bitmap is 20 bits wide.
const BITMAP_MASK: u32 = 0x000F_FFFF;

#[derive(Default)]
pub struct StationBook {
    stations: BTreeMap<u8, HfdlStation>,
    /// The version of the system table received over the air since the book
    /// was made. A table only seeded from disk leaves this `None`.
    table_version: Option<u16>,
}

impl StationBook {
    pub fn new() -> Self {
        Self::default()
    }

    /// Fill in stations' table entries (position, channels) from a saved copy.
    /// Does nothing once a table has come in over the air, which is newer than
    /// anything on disk, and never touches what has been heard.
    pub fn seed(&mut self, saved: &[HfdlStation]) {
        if self.table_version.is_some() {
            return;
        }
        for s in saved {
            let st = self.entry(s.gs_id, Some(s.name.as_str()));
            st.lat = s.lat;
            st.lon = s.lon;
            st.freqs =
                s.freqs.iter().map(|f| HfdlStationFreq { active: false, ..f.clone() }).collect();
            st.apply_active_bits();
        }
    }

    /// Fold one decoded event in. `freq_khz` is the channel the lane is on and
    /// `unix` the time it was decoded.
    pub fn observe(&mut self, e: &HfdlEvent, freq_khz: u32, unix: i64) {
        match e.kind.as_str() {
            "squitter" => self.squitter(e, freq_khz, unix),
            "systable-complete" => self.table(e),
            _ => self.uplink(e, freq_khz, unix),
        }
    }

    /// The stations by id, and the over-the-air table version.
    pub fn snapshot(&self) -> (Vec<HfdlStation>, Option<u16>) {
        (self.stations.values().cloned().collect(), self.table_version)
    }

    fn entry(&mut self, gs_id: u8, name: Option<&str>) -> &mut HfdlStation {
        self.stations
            .entry(gs_id)
            .or_insert_with(|| HfdlStation::new(gs_id, name.or_else(|| gs_name(gs_id))))
    }

    fn squitter(&mut self, e: &HfdlEvent, freq_khz: u32, unix: i64) {
        let d = &e.details;
        let Some(id) = d.get("gs_id").and_then(|v| v.as_u64()).filter(|&v| v < 128) else { return };
        let st = self.entry(id as u8, d.get("gs_name").and_then(|v| v.as_str()));
        st.heard_at = unix;
        st.heard_khz = freq_khz;
        st.snr_db = e.snr_db;
        st.squitters = st.squitters.saturating_add(1);
        st.utc_sync = d.get("utc_sync").and_then(|v| v.as_bool()).unwrap_or(false);
        if let Some(bits) = d.get("freqs_in_use").and_then(|v| v.as_u64()) {
            st.active_bits = (bits as u32) & BITMAP_MASK;
            st.active_at = unix;
            st.apply_active_bits();
        }
    }

    /// An uplink LPDU's envelope names the station that sent it.
    fn uplink(&mut self, e: &HfdlEvent, freq_khz: u32, unix: i64) {
        let Some(who) = e.details.get("who") else { return };
        if who.get("dir").and_then(|v| v.as_str()) != Some("uplink") {
            return;
        }
        let Some(id) = who.get("gs_id").and_then(|v| v.as_u64()).filter(|&v| v < 128) else {
            return;
        };
        let st = self.entry(id as u8, who.get("gs_name").and_then(|v| v.as_str()));
        st.heard_at = unix;
        st.heard_khz = freq_khz;
        st.snr_db = e.snr_db.or(st.snr_db);
    }

    /// A reassembled system table replaces what is known of every station's
    /// position and channels — the table is the network's own list, so a
    /// station it omits has no channels any more — and keeps what was heard.
    fn table(&mut self, e: &HfdlEvent) {
        let Ok(table) = serde_json::from_value::<SystemTable>(e.details.clone()) else { return };
        // A table older than the one already held is a station repeating a
        // superseded broadcast. (Equal versions are fine: it is the same table.)
        if let Some(cur) = self.table_version {
            if table.version != cur && !version_is_newer(table.version, cur) {
                return;
            }
        }
        for st in self.stations.values_mut() {
            st.freqs.clear();
        }
        for gs in &table.stations {
            let st = self.entry(gs.gs_id, gs.gs_name.as_deref());
            st.lat = Some(gs.lat);
            st.lon = Some(gs.lon);
            st.utc_sync = st.utc_sync || gs.utc_sync;
            st.freqs = gs
                .frequencies
                .iter()
                .map(|f| HfdlStationFreq {
                    freq_hz: f.freq_hz,
                    slot: f.master_frame_slot,
                    active: false,
                })
                .collect();
            st.apply_active_bits();
        }
        self.table_version = Some(table.version);
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

    fn squitter(gs: u64, bits: u64) -> HfdlEvent {
        ev(
            "squitter",
            json!({ "gs_id": gs, "gs_name": gs_name(gs as u8), "utc_sync": true, "freqs_in_use": bits }),
            Some(14.5),
        )
    }

    /// A two-station table: Riverhead on three channels, Shannon on two.
    fn table(version: u16) -> HfdlEvent {
        ev(
            "systable-complete",
            json!({
                "version": version,
                "stations": [
                    { "gs_id": 4, "gs_name": "Riverhead, New York", "utc_sync": true,
                      "lat": 40.95, "lon": -72.63, "spdu_version": 1,
                      "frequencies": [
                        { "freq_hz": 5_451_000, "master_frame_slot": 0 },
                        { "freq_hz": 8_912_000, "master_frame_slot": 1 },
                        { "freq_hz": 21_931_000, "master_frame_slot": 2 } ] },
                    { "gs_id": 7, "gs_name": "Shannon, Ireland", "utc_sync": true,
                      "lat": 52.7, "lon": -8.9, "spdu_version": 1,
                      "frequencies": [
                        { "freq_hz": 3_455_000, "master_frame_slot": 0 },
                        { "freq_hz": 8_825_000, "master_frame_slot": 1 } ] },
                ]
            }),
            None,
        )
    }

    #[test]
    fn a_squitter_makes_a_station_heard_where_it_was_heard() {
        let mut b = StationBook::new();
        b.observe(&squitter(4, 0b101), 21_931, 1_000);
        b.observe(&squitter(4, 0b101), 21_931, 1_032);
        let (s, v) = b.snapshot();
        assert_eq!(v, None);
        assert_eq!(s.len(), 1);
        let st = &s[0];
        assert_eq!((st.gs_id, st.name.as_str()), (4, "Riverhead, New York"));
        assert_eq!((st.heard_at, st.heard_khz, st.squitters), (1_032, 21_931, 2));
        assert_eq!(st.snr_db, Some(14.5));
        assert!(st.utc_sync);
        // No table yet: the bitmap is kept raw and says nothing in kHz.
        assert_eq!(st.active_bits, 0b101);
        assert!(!st.has_table());
        assert!(st.active_khz().is_empty());
    }

    #[test]
    fn an_unnamed_id_is_still_a_station() {
        let mut b = StationBook::new();
        b.observe(&ev("squitter", json!({ "gs_id": 99, "freqs_in_use": 1 }), None), 8_912, 5);
        assert_eq!(b.snapshot().0[0].name, "GS 99");
    }

    #[test]
    fn the_table_turns_the_bitmap_into_frequencies_either_way_round() {
        // Squitter first, table second…
        let mut a = StationBook::new();
        a.observe(&squitter(4, 0b101), 21_931, 1_000);
        a.observe(&table(7), 21_931, 1_010);
        let st = a.snapshot().0.into_iter().find(|s| s.gs_id == 4).unwrap();
        assert_eq!(st.active_khz(), vec![5_451, 21_931]);
        // …and table first, squitter second: the same answer.
        let mut b = StationBook::new();
        b.observe(&table(7), 21_931, 1_000);
        b.observe(&squitter(4, 0b101), 21_931, 1_010);
        let st = b.snapshot().0.into_iter().find(|s| s.gs_id == 4).unwrap();
        assert_eq!(st.active_khz(), vec![5_451, 21_931]);
        assert_eq!((st.lat, st.lon), (Some(40.95), Some(-72.63)));
        assert_eq!(b.snapshot().1, Some(7));
    }

    #[test]
    fn a_bit_past_the_stations_last_channel_names_nothing() {
        let mut b = StationBook::new();
        b.observe(&table(7), 0, 0);
        // Shannon has two channels; bit 5 is beyond them.
        b.observe(&squitter(7, 0b100_010), 3_455, 10);
        let st = b.snapshot().0.into_iter().find(|s| s.gs_id == 7).unwrap();
        assert_eq!(st.active_khz(), vec![8_825]);
    }

    #[test]
    fn a_table_keeps_what_was_heard_and_drops_stations_it_omits() {
        let mut b = StationBook::new();
        b.observe(&squitter(9, 0b1), 11_184, 50);
        b.observe(&table(7), 0, 60);
        let (s, _) = b.snapshot();
        let barrow = s.iter().find(|s| s.gs_id == 9).unwrap();
        // Barrow is not in the table: still listed (it was heard), channel-less.
        assert_eq!((barrow.heard_at, barrow.freqs.len()), (50, 0));
        assert_eq!(s.iter().find(|s| s.gs_id == 4).unwrap().freqs.len(), 3);
    }

    #[test]
    fn an_older_table_does_not_replace_a_newer_one() {
        let mut b = StationBook::new();
        b.observe(&table(10), 0, 0);
        b.observe(&table(9), 0, 1);
        assert_eq!(b.snapshot().1, Some(10));
        // …a newer one does, across the 12-bit wrap too.
        b.observe(&table(11), 0, 2);
        assert_eq!(b.snapshot().1, Some(11));
        let mut w = StationBook::new();
        w.observe(&table(4_095), 0, 0);
        w.observe(&table(1), 0, 1);
        assert_eq!(w.snapshot().1, Some(1));
    }

    #[test]
    fn an_uplink_counts_as_the_station_being_heard_but_a_downlink_does_not() {
        let mut b = StationBook::new();
        let up = ev(
            "acars",
            json!({ "who": { "dir": "uplink", "gs_id": 7, "gs_name": "Shannon, Ireland" } }),
            Some(9.0),
        );
        let down = ev(
            "acars",
            json!({ "who": { "dir": "downlink", "gs_id": 4, "aircraft_id": 12 } }),
            Some(9.0),
        );
        b.observe(&down, 8_912, 100);
        assert!(b.snapshot().0.is_empty(), "an aircraft talking to a station is not the station");
        b.observe(&up, 8_912, 101);
        let (s, _) = b.snapshot();
        assert_eq!((s[0].gs_id, s[0].heard_at, s[0].heard_khz, s[0].squitters), (7, 101, 8_912, 0));
        // An uplink says nothing about channels in use.
        assert_eq!(s[0].active_at, 0);
    }

    #[test]
    fn a_saved_table_seeds_the_book_until_the_air_gives_a_newer_one() {
        let mut first = StationBook::new();
        first.observe(&table(7), 0, 0);
        first.observe(&squitter(4, 0b1), 5_451, 20);
        // What goes to disk carries no heard state…
        let saved: Vec<HfdlStation> =
            first.snapshot().0.iter().map(HfdlStation::table_entry).collect();
        assert!(saved.iter().all(|s| s.heard_at == 0 && s.squitters == 0 && s.active_at == 0));
        // …and seeds the next run's book with the channels and positions.
        let mut next = StationBook::new();
        next.seed(&saved);
        next.observe(&squitter(4, 0b10), 8_912, 30);
        let st = next.snapshot().0.into_iter().find(|s| s.gs_id == 4).unwrap();
        assert_eq!(st.active_khz(), vec![8_912]);
        assert_eq!(next.snapshot().1, None, "a saved table is not an over-the-air version");
        // Once a table arrives, a late seed must not undo it.
        next.observe(&table(8), 0, 40);
        next.seed(&[HfdlStation::new(4, Some("old"))]);
        assert_eq!(next.snapshot().0.iter().find(|s| s.gs_id == 4).unwrap().freqs.len(), 3);
    }
}
