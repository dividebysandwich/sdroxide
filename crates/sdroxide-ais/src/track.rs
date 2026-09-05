//! The vessel table: decoded messages in, one row per MMSI out.
//!
//! Everything a ship says arrives in pieces, and the pieces are minutes apart.
//! The position comes every two to ten seconds; the name, call sign, IMO
//! number, dimensions, draught and destination come once every six minutes in a
//! different message; a Class B unit splits even the name and the call sign
//! into two halves sent separately. This module is where those are folded into
//! one picture per station, and where the two ages that govern a target's life
//! on screen are kept.
//!
//! # The trail is trimmed by time, on both clocks
//!
//! A trail point is dropped when it is older than
//! [`sdroxide_types::AisSettings::trail_minutes`], which happens on *both* the
//! arrival path and the expiry sweep. Only trimming on arrival would leave a
//! ship that stopped reporting frozen with a ten-minute tail behind it forever;
//! only trimming on the sweep would let a fast vessel outrun the cap between
//! sweeps. The point cap is a second, independent bound: a vessel in a traffic
//! separation scheme reports every two seconds, and ten minutes of that is
//! three hundred points times five hundred ships on a message re-sent twice a
//! second.
//!
//! # What the position is *not* compared against
//!
//! Nothing. Unlike ADS-B there is no ambiguous encoding to resolve, no even/odd
//! pair, no reference position: an AIS position report carries an absolute
//! latitude and longitude, and the only judgement made here is whether the
//! numbers are on the earth — which [`crate::message`] has already made.

use std::collections::HashMap;
use std::collections::VecDeque;

use sdroxide_types::{AIS_TRACK_MAX, AisKind, AisSettings, AisVessel};

use crate::channel::Decoded;
use crate::message::Message;
use crate::sixbit;

/// One station, plus the working state that never leaves this crate.
struct Entry {
    v: AisVessel,
    /// Where it has been, with the time each point was recorded — which is what
    /// lets the trail be a window in time rather than a count.
    trail: VecDeque<(f32, f32, i64)>,
}

/// How far a vessel has to have moved before a point is worth remembering.
///
/// About a metre of latitude. A ship alongside reports every three minutes from
/// a GNSS receiver that wanders by a few metres, and without this its trail
/// would be a smudge on top of the symbol rather than a trail.
const MIN_MOVE_DEG: f64 = 1e-5;

/// The table.
pub struct Tracker {
    by_mmsi: HashMap<u32, Entry>,
    cfg: AisSettings,
    /// Rotating group identifier for the `!AIVDM` sentences of a multi-sentence
    /// message. Means nothing on its own; it is there so two interleaved groups
    /// can be told apart, which is what the format asks for.
    seq: u8,
}

impl Tracker {
    pub fn new(cfg: AisSettings) -> Tracker {
        Tracker { by_mmsi: HashMap::new(), cfg: cfg.sane(), seq: 0 }
    }

    pub fn set_config(&mut self, cfg: AisSettings) {
        self.cfg = cfg.sane();
    }

    pub fn len(&self) -> usize {
        self.by_mmsi.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_mmsi.is_empty()
    }

    /// Fold one decoded message into the table.
    pub fn absorb(&mut self, d: &Decoded, now: i64) {
        let m = &d.message;
        // MMSI 0 is what a transmitter sends before it has been programmed, and
        // it is not a station: several of them would share one row and drag it
        // across the sea.
        if m.mmsi == 0 {
            return;
        }
        // A message of a type this crate does not interpret carries an MMSI and,
        // as far as anything here can tell, nothing else — no position, no
        // name, no speed. It may say that a station already on the list is
        // still there, but it must not put a new one on it: a row like that
        // cannot be drawn, cannot be identified, and is indistinguishable from
        // the ships the operator is actually looking for. A list filling up
        // with them beside an empty chart is what issue #330 reported.
        if !m.known && !self.by_mmsi.contains_key(&m.mmsi) {
            return;
        }
        let seq = self.seq;
        self.seq = self.seq.wrapping_add(1);
        let cfg = self.cfg;

        let e = self
            .by_mmsi
            .entry(m.mmsi)
            .or_insert_with(|| Entry { v: AisVessel::new(m.mmsi, now), trail: VecDeque::new() });

        e.v.last_at = now;
        e.v.messages = e.v.messages.saturating_add(1);
        e.v.rssi_dbfs = d.rssi_dbfs;
        e.v.snr_db = d.snr_db;
        e.v.channel = d.channel;
        e.v.last_type = m.kind;
        e.v.nmea = sixbit::nmea(&d.bits, d.channel, seq).join("\n");
        // What the MMSI's own format says outranks what the message implies; a
        // message that implies nothing leaves the row as it was.
        if let Some(k) = AisKind::from_mmsi(m.mmsi).or(m.class) {
            e.v.kind = k;
        }
        absorb_fields(e, m, now);
        trim_trail(e, now, cfg);
    }

    /// Drop what has gone quiet, hold the table to its ceiling, and let the
    /// trails of everything still listed decay.
    ///
    /// Called on the emit tick rather than per message: it walks the whole
    /// table, and the table is walked to build the snapshot anyway.
    pub fn expire(&mut self, now: i64) {
        let ttl = i64::from(self.cfg.drop_list_s);
        self.by_mmsi.retain(|_, e| now - e.v.last_at <= ttl);
        let cfg = self.cfg;
        for e in self.by_mmsi.values_mut() {
            trim_trail(e, now, cfg);
        }
        let max = self.cfg.max_vessels as usize;
        if self.by_mmsi.len() > max {
            // Oldest first: a ceiling reached in a busy estuary should cost the
            // stations nobody has heard from lately rather than the new
            // arrivals.
            let mut ages: Vec<(i64, u32)> =
                self.by_mmsi.iter().map(|(k, e)| (e.v.last_at, *k)).collect();
            ages.sort_unstable();
            for (_, k) in ages.into_iter().take(self.by_mmsi.len() - max) {
                self.by_mmsi.remove(&k);
            }
        }
    }

    /// The table as the panel sees it.
    pub fn snapshot(&self) -> Vec<AisVessel> {
        self.by_mmsi
            .values()
            .map(|e| {
                let mut v = e.v.clone();
                v.track = e.trail.iter().map(|&(la, lo, _)| (la, lo)).collect();
                v
            })
            .collect()
    }
}

/// Fold whatever pieces of the picture one message carried.
fn absorb_fields(e: &mut Entry, m: &Message, now: i64) {
    if let Some(f) = &m.fix {
        // Every field is taken only where the message actually stated one:
        // "not available" is not a value, and writing it over a heading that
        // arrived thirty seconds ago would make a vessel lose its bow every
        // other report.
        if f.sog_kt.is_some() {
            e.v.sog_kt = f.sog_kt;
        }
        if f.cog_deg.is_some() {
            e.v.cog_deg = f.cog_deg;
        }
        if f.heading_deg.is_some() {
            e.v.heading_deg = f.heading_deg;
        }
        if f.turn_deg_min.is_some() {
            e.v.turn_rate_deg_min = f.turn_deg_min;
        }
        if f.nav_status.is_some() {
            e.v.nav_status = f.nav_status;
        }
        if f.altitude_m.is_some() {
            e.v.altitude_m = f.altitude_m;
        }
        e.v.accuracy = f.accuracy;
        e.v.raim = f.raim;
        if let (Some(lat), Some(lon)) = (f.lat, f.lon) {
            place(e, lat, lon, now);
        }
    }
    if let Some(s) = &m.statics {
        if let Some(n) = &s.name {
            e.v.name = n.clone();
        }
        if let Some(c) = &s.call_sign {
            e.v.call_sign = c.clone();
        }
        if s.imo.is_some() {
            e.v.imo = s.imo;
        }
        if s.ship_type.is_some() {
            e.v.ship_type = s.ship_type;
        }
        if let Some((bow, stern, port, stbd)) = s.dim {
            e.v.dim_bow_m = bow;
            e.v.dim_stern_m = stern;
            e.v.dim_port_m = port;
            e.v.dim_starboard_m = stbd;
        }
    }
    if let Some(v) = &m.voyage {
        if let Some(d) = &v.destination {
            e.v.destination = d.clone();
        }
        if v.draught_m.is_some() {
            e.v.draught_m = v.draught_m;
        }
        if let Some(t) = &v.eta {
            e.v.eta = t.clone();
        }
    }
    if let Some(a) = &m.aid {
        e.v.aid_type = Some(a.aid_type);
        e.v.virtual_aid = a.virtual_aid;
        if !a.name.is_empty() {
            e.v.name = a.name.clone();
        }
        if let Some((bow, stern, port, stbd)) = a.dim {
            e.v.dim_bow_m = bow;
            e.v.dim_stern_m = stern;
            e.v.dim_port_m = port;
            e.v.dim_starboard_m = stbd;
        }
        // A buoy that says it has drifted off station is the one thing about an
        // aid anybody has to be told, and there is no other field for it.
        if a.off_position {
            e.v.nav_status = Some(6); // aground, which is what it amounts to
        }
    }
    if let Some(t) = &m.utc {
        e.v.utc = t.clone();
    }
}

/// Record a fix, pushing the previous one onto the trail.
fn place(e: &mut Entry, lat: f64, lon: f64, now: i64) {
    if let (Some(plat), Some(plon)) = (e.v.lat, e.v.lon) {
        let moved = (plat - lat).abs() > MIN_MOVE_DEG || (plon - lon).abs() > MIN_MOVE_DEG;
        if moved {
            e.trail.push_back((plat as f32, plon as f32, e.v.last_pos_at));
        }
    }
    e.v.lat = Some(lat);
    e.v.lon = Some(lon);
    e.v.last_pos_at = now;
}

/// Drop trail points that have aged out, and hold the trail to its ceiling.
fn trim_trail(e: &mut Entry, now: i64, cfg: AisSettings) {
    let window = cfg.trail_secs();
    if window <= 0 {
        e.trail.clear();
        return;
    }
    while e.trail.front().is_some_and(|&(_, _, at)| now - at > window) {
        e.trail.pop_front();
    }
    while e.trail.len() > AIS_TRACK_MAX {
        e.trail.pop_front();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channel::Decoded;
    use crate::message;
    use crate::tx::Payload;
    use sdroxide_types::AIS_CHANNEL_A_HZ;

    fn feed(t: &mut Tracker, bits: Vec<bool>, now: i64) {
        let message = message::parse(&bits).expect("a header");
        t.absorb(
            &Decoded {
                message,
                bits,
                center_hz: AIS_CHANNEL_A_HZ,
                channel: 'A',
                snr_db: 20.0,
                rssi_dbfs: -30.0,
                freq_offset_hz: 0.0,
            },
            now,
        );
    }

    fn position(mmsi: u32, lat: f64, lon: f64) -> Vec<bool> {
        Payload::new(168)
            .put(0, 6, 1)
            .put(8, 30, u64::from(mmsi))
            .put(50, 10, 100)
            .put_signed(61, 28, (lon * 600_000.0) as i64)
            .put_signed(89, 27, (lat * 600_000.0) as i64)
            .put(116, 12, 900)
            .bits()
    }

    fn statics(mmsi: u32, name: &str) -> Vec<bool> {
        Payload::new(424)
            .put(0, 6, 5)
            .put(8, 30, u64::from(mmsi))
            .put(232, 8, 70)
            .put(240, 9, 100)
            .put(249, 9, 20)
            .put(258, 6, 8)
            .put(264, 6, 8)
            .put_text(112, 20, name)
            .put_text(302, 20, "HAMBURG")
            .bits()
    }

    /// Issue #330: a message type this crate cannot interpret carries an MMSI
    /// and nothing that can be drawn or read. Fifty rows of those beside an
    /// empty chart is what the report looked like, so one may confirm a station
    /// already on the list but never put a new one on it.
    #[test]
    fn a_message_that_cannot_be_interpreted_does_not_invent_a_station() {
        let mut t = Tracker::new(AisSettings::default());
        // Type 20, data link management: a real AIS message, and one with
        // nothing in it for a vessel list.
        let admin = Payload::new(160).put(0, 6, 20).put(8, 30, 244_660_000).bits();
        feed(&mut t, admin.clone(), 1_000);
        assert_eq!(t.len(), 0, "an uninterpretable message is not a vessel");

        // Once the station has said something this crate does understand, the
        // same message keeps it alive.
        feed(&mut t, position(244_660_000, 52.0, 4.0), 1_010);
        assert_eq!(t.len(), 1);
        feed(&mut t, admin, 1_020);
        let v = &t.snapshot()[0];
        assert_eq!(v.last_at, 1_020, "it was heard from again");
        assert_eq!(v.lat, Some(52.0), "and it did not lose its position");
    }

    /// The position and the name arrive in different messages minutes apart and
    /// have to land on the same row — the whole reason this module exists.
    #[test]
    fn one_row_gathers_what_arrives_in_separate_messages() {
        let mut t = Tracker::new(AisSettings::default());
        feed(&mut t, position(244_660_000, 52.0, 4.0), 1_000);
        assert_eq!(t.snapshot()[0].label(), "244660000", "no name yet, so the MMSI stands in");
        feed(&mut t, statics(244_660_000, "NORDIC LIGHT"), 1_180);
        let v = &t.snapshot()[0];
        assert_eq!(t.len(), 1);
        assert_eq!(v.name, "NORDIC LIGHT");
        assert_eq!(v.destination, "HAMBURG");
        assert_eq!(v.length_m(), Some(120));
        assert_eq!(v.beam_m(), Some(16));
        assert!(v.has_position());
        assert!(!v.nmea.is_empty(), "every row carries the sentence it came from");
    }

    /// The two clocks: a vessel that stops reporting greys before it is
    /// dropped, and both windows are the long ones AIS needs rather than
    /// ADS-B's.
    #[test]
    fn a_quiet_vessel_greys_long_before_it_is_dropped() {
        let cfg = AisSettings::default();
        let mut t = Tracker::new(cfg);
        feed(&mut t, position(244_660_000, 52.0, 4.0), 1_000);
        // Three minutes on — a vessel at anchor's reporting interval — it is
        // still on the map.
        t.expire(1_180);
        assert!(!t.snapshot()[0].pos_stale(1_180, cfg.drop_map_s));
        // Ten minutes on it has greyed but is still listed, with its name.
        t.expire(1_600);
        assert_eq!(t.len(), 1);
        assert!(t.snapshot()[0].pos_stale(1_600, cfg.drop_map_s));
        // Half an hour on it is gone.
        t.expire(2_900);
        assert!(t.is_empty());
    }

    /// The trail is a window in time. Points older than it go, whether or not
    /// the vessel is still reporting — a ship that stopped must not be left
    /// with a tail behind it forever.
    #[test]
    fn the_trail_is_ten_minutes_and_not_a_number_of_points() {
        let cfg = AisSettings::default();
        assert_eq!(cfg.trail_secs(), 600);
        let mut t = Tracker::new(cfg);
        // A ship reporting every ten seconds for twenty minutes.
        for k in 0..120i64 {
            feed(&mut t, position(244_660_000, 52.0 + k as f64 * 0.001, 4.0), 1_000 + k * 10);
        }
        let v = &t.snapshot()[0];
        // Ten minutes of ten-second reports is sixty points, give or take the
        // one being reported now.
        assert!((55..=62).contains(&v.track.len()), "{} points", v.track.len());
        // The oldest kept point is inside the window.
        let now = 1_000 + 119 * 10;
        let oldest = t.by_mmsi[&244_660_000].trail.front().unwrap().2;
        assert!(now - oldest <= 600, "a point {} s old survived", now - oldest);

        // ...and once it stops reporting, the tail decays rather than freezing.
        t.expire(now + 300);
        assert!(t.snapshot()[0].track.len() < 35);
        t.expire(now + 900);
        assert!(t.snapshot()[0].track.is_empty(), "the whole trail should have aged out");
    }

    /// A vessel alongside reports from a wandering GNSS receiver; its trail
    /// must not become a smudge on top of its own symbol.
    #[test]
    fn a_moored_vessel_does_not_fill_its_trail_with_the_same_point() {
        let mut t = Tracker::new(AisSettings::default());
        for k in 0..20i64 {
            feed(&mut t, position(244_660_000, 52.0, 4.0), 1_000 + k * 10);
        }
        assert!(t.snapshot()[0].track.is_empty(), "a stationary ship left a trail");
    }

    /// The MMSI's own format outranks what a message implies, so a buoy sending
    /// a position report is still a buoy, and an unprogrammed transmitter with
    /// MMSI zero is not a station at all.
    #[test]
    fn the_mmsi_decides_what_a_station_is() {
        let mut t = Tracker::new(AisSettings::default());
        feed(&mut t, position(992_111_840, 51.9, 4.1), 1_000);
        assert_eq!(t.snapshot()[0].kind, AisKind::AidToNavigation);

        feed(&mut t, position(0, 51.9, 4.1), 1_000);
        assert_eq!(t.len(), 1, "MMSI zero was given a row");
    }

    /// A message that says nothing about a field must not erase what an earlier
    /// one said. A Class B unit reports its position without a name every
    /// thirty seconds and its name every six minutes.
    #[test]
    fn a_later_message_does_not_erase_what_an_earlier_one_said() {
        let mut t = Tracker::new(AisSettings::default());
        feed(&mut t, statics(244_660_000, "SEA BREEZE"), 1_000);
        for k in 1..10i64 {
            feed(&mut t, position(244_660_000, 52.0 + k as f64 * 0.01, 4.0), 1_000 + k * 30);
        }
        let v = &t.snapshot()[0];
        assert_eq!(v.name, "SEA BREEZE");
        assert_eq!(v.ship_type, Some(70));
        assert_eq!(v.destination, "HAMBURG");
    }

    /// The ceiling costs the stations nobody has heard from lately, not the
    /// ones that just arrived.
    #[test]
    fn the_ceiling_drops_the_oldest_first() {
        let cfg = AisSettings { max_vessels: 10, ..AisSettings::default() }.sane();
        let mut t = Tracker::new(cfg);
        for k in 0..20u32 {
            feed(&mut t, position(200_000_000 + k, 52.0, 4.0), 1_000 + i64::from(k));
        }
        t.expire(1_020);
        assert_eq!(t.len(), 10);
        let mmsis: Vec<u32> = t.snapshot().iter().map(|v| v.mmsi).collect();
        assert!(mmsis.contains(&200_000_019));
        assert!(!mmsis.contains(&200_000_000));
    }
}
