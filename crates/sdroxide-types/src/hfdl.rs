//! HFDL (ARINC 635) ground-network decode: what the operator asks the engine
//! for, and what it reports back. Pure data + serde, shared by the native
//! engine and the UI (native + WASM) — the demodulation itself lives in the
//! native `sdroxide-hfdl` crate, same split as [`crate::qo100`] and
//! [`crate::ism`].
//!
//! As with the other not-a-mode lanes (ADS-B, VDL2, AIS, QO-100), HFDL rides
//! its own downconversion of the raw I/Q rather than the audio chain: the
//! engine hands the lane the complex baseband around the tuned channel, and
//! the worker's per-channel decoder does the rest.

use serde::{Deserialize, Serialize};

/// The default channel to listen on: Riverhead's primary 21 931 kHz — the
/// frequency the reference off-air capture that validated the whole chain was
/// recorded on, and the strongest single HFDL ground signal most of the north
/// Atlantic/North America can reach.
pub const HFDL_DEFAULT_HZ: f64 = 21_931_000.0;

/// The engine's HFDL lane decimates the raw I/Q to this rate before handing
/// it to [`crate::HfdlSettings`]-configured worker: the same 24 kS/s the
/// off-air validation used (a 2.8 kHz USB channel inside a ±12 kHz lane, with
/// the +1440 Hz subcarrier shift handled inside the decoder core). Keeping
/// the lane rate fixed at the validated one is deliberate — HFDL's demod
/// constants live at 12 kS/s ≈ 6.67 samples per 1800-Bd symbol, and this is
/// the rate the whole chain was proven against.
pub const HFDL_LANE_RATE_HZ: f64 = 24_000.0;

/// What the operator asks the HFDL decoder to do.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HfdlSettings {
    /// Whether the decoder runs at all. Off by default: it is a
    /// sometimes-reached utility lane, not something every station pays a
    /// downconverter and a worker thread for by default, the same reason
    /// [`crate::Qo100Settings::enabled`] is off.
    pub enabled: bool,
    /// The USB channel frequency (the HFDL "assigned frequency") the lane
    /// centres on, in Hz. The decoder's subcarrier sits at +1440 Hz, and the
    /// lane is a fixed 24 kS/s channel centred on this — so it works wherever
    /// the radio itself is tuned, exactly like the QO-100 beacon lane.
    pub frequency_hz: f64,
}

impl Default for HfdlSettings {
    fn default() -> Self {
        Self { enabled: false, frequency_hz: HFDL_DEFAULT_HZ }
    }
}

/// One aircraft position an HFDL downlink carried.
///
/// The normalized object xng lifts out of a performance-data (0xD1) or
/// frequency-data (0xD5) HFNPDU: a 20-bit lat/lon pair and whatever identity
/// the payload held — the GS-local downlink alias, and the ICAO address when
/// the logon-confirm cache resolved the alias to one. The all-zero
/// not-yet-acquired fix is dropped upstream, so a `Some` here is a real
/// position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HfdlFix {
    pub lat: f64,
    pub lon: f64,
    /// The GS-local downlink alias (8-bit). *Not* the ICAO address.
    pub aircraft_id: Option<u32>,
    /// The ICAO address, when the logon-confirm cache resolved the alias.
    pub icao: Option<String>,
    /// The flight identifier the payload carried, when it carried one.
    pub flight: Option<String>,
}

impl HfdlFix {
    /// The identity the map keys an aircraft by: the ICAO when it is known,
    /// else the GS-local alias, else the flight — most stable first, so an
    /// aircraft that resolves its ICAO mid-session keeps one plot rather than
    /// splitting into two.
    pub fn key(&self) -> String {
        if let Some(icao) = self.icao.as_deref().filter(|s| !s.is_empty()) {
            return format!("icao:{icao}");
        }
        if let Some(id) = self.aircraft_id {
            return format!("ac:{id}");
        }
        if let Some(flt) = self.flight.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            return format!("flt:{flt}");
        }
        // No identity at all (an un-resolved downlink): key by the rounded
        // position so the fix still plots, even if it cannot be followed.
        format!("pos:{:.2},{:.2}", self.lat, self.lon)
    }

    /// What the map labels the aircraft: the flight, then the ICAO, then the
    /// alias, and a bare `#?` when the payload named nothing.
    pub fn label(&self) -> String {
        if let Some(flt) = self.flight.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            return flt.to_string();
        }
        if let Some(icao) = self.icao.as_deref().filter(|s| !s.is_empty()) {
            return icao.to_string();
        }
        match self.aircraft_id {
            Some(id) => format!("#{id}"),
            None => "#?".to_string(),
        }
    }
}

/// One decoded HFDL event, as it appears in the panel's log.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HfdlDecode {
    /// Unix second it was decoded.
    pub unix: i64,
    /// The event kind, in xng's terms: "squitter", "logon", "logoff",
    /// "position", "performance-data", "frequency-data", "acars", ... — the
    /// string the SPDU/MPDU parser produced.
    pub kind: String,
    /// The ground station the event names, when the payload carried one
    /// (squitters and frequency data always do) — `name (GS n)`.
    pub gs: Option<String>,
    /// The channel frequency, in kHz.
    pub freq_khz: u32,
    /// EVM-derived burst SNR in dB, where the demod measured one.
    pub snr_db: Option<f32>,
    /// Measured carrier-frequency offset in Hz.
    pub freq_skew_hz: Option<f32>,
    /// Coded symbols the Viterbi decoder corrected for the burst.
    pub fec_corrected: Option<u32>,
    /// The payload details as compact JSON — every field xng's parser
    /// surfaced for the kind (aircraft id, position, UTC, ground station id,
    /// frequencies in use, ...).
    pub details: String,
    /// The aircraft position this event carried, where it carried one. Both
    /// the performance-data and frequency-data records do; squitters, logons
    /// and the rest do not. Lifted out of [`Self::details`] so the panel and
    /// the map need not parse JSON. Appended last: the wire is positional.
    pub position: Option<HfdlFix>,
}

/// What the engine tells the window about the decoder's own state. Re-sent
/// whenever it changes, the same convention [`crate::Qo100Status`] follows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HfdlStatus {
    /// Whether the downconverter and worker are actually running — mirrors
    /// [`HfdlSettings::enabled`], but from the engine's side, so a client
    /// that opens mid-session sees the true state rather than assuming it.
    pub running: bool,
    /// Burst average level of the channel, in dBFS, as the demod sees it —
    /// a low figure while tuned to dead air says plainly that the lane is up
    /// but no station is in it, rather than looking merely idle.
    pub level_dbfs: f32,
    /// Bursts captured since the decoder was switched on — a burst is the
    /// demod-level unit; most carry no recognized payload, so a high burst
    /// count with few [`Self::decodes`] is a station up but idle.
    pub bursts: u64,
    /// Bursts that produced at least one decoded event.
    pub decodes: u64,
    /// The most recent decoded events, newest first, bounded to
    /// [`HFDL_LOG_DEPTH`] — the panel's log. A ground station's squitter
    /// repeats every ~32 s, so a fresh entry about that often is the norm,
    /// not evidence of noise; aircraft traffic between squitters is the
    /// interesting part. Carried whole on every status, the same snapshot
    /// convention [`crate::AdsbStatus`] uses: a dropped status costs nothing
    /// because the next one carries the same information.
    pub log: Vec<HfdlDecode>,
    /// The ground stations the decoder knows of, by id. Unlike [`Self::log`]
    /// this does not roll: a station stays listed from its first squitter, and
    /// from the saved system table before that. Appended last: the wire is
    /// positional.
    pub stations: Vec<HfdlStation>,
    /// The version of the system table received over the air since the decoder
    /// started, or `None` while none has been — [`Self::stations`] may still
    /// carry channels from the copy saved last time.
    pub table_version: Option<u16>,
}

impl Default for HfdlStatus {
    fn default() -> Self {
        Self {
            running: false,
            level_dbfs: 0.0,
            bursts: 0,
            decodes: 0,
            log: Vec::new(),
            stations: Vec::new(),
            table_version: None,
        }
    }
}

/// How many [`HfdlStatus::log`] entries a worker keeps in its rolling log —
/// a session's worth of HFDL is chatty (a squitter every ~32 s plus the
/// aircraft traffic), and a panel has no need of more than a screenful.
pub const HFDL_LOG_DEPTH: usize = 64;

/// How long what a ground station told us stays a claim worth showing: a
/// squitter repeats every ~32 s, but the decoder hears one channel at a time,
/// so a station's picture goes stale the moment the operator tunes away.
/// Quarter of an hour is several squitter cycles on the channel it was heard on
/// and no longer than a band stays open for.
pub const HFDL_ACTIVE_FRESH_S: i64 = 15 * 60;

/// One frequency of a ground station's system-table entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HfdlStationFreq {
    /// The assigned (USB carrier) frequency, in Hz. The table carries it in
    /// 100 Hz BCD, so this is exact.
    pub freq_hz: u32,
    /// The channel's slot in the station's master frame, 0..=15.
    pub slot: u8,
    /// Whether the station's latest squitter listed this channel as in use.
    /// Meaningless without [`HfdlStation::active_at`].
    pub active: bool,
}

impl HfdlStationFreq {
    /// The frequency in kHz, which is how channels are named everywhere else
    /// in the HFDL panel.
    pub fn khz(&self) -> u32 {
        (f64::from(self.freq_hz) / 1000.0).round() as u32
    }
}

/// One HFDL ground station, as the decoder has pieced it together from its
/// squitters and from the system table the network broadcasts.
///
/// Two sources feed it and they arrive independently. A **squitter** names the
/// station, says which of its channels are in use *as a bitmap* and says which
/// table version it is using; the **system table** (a multi-part HFNPDU the
/// stations broadcast now and then, and which sdroxide keeps on disk) is what
/// turns bit *n* of that bitmap into a frequency. So a station heard before any
/// table is known has [`Self::active_bits`] but an empty [`Self::freqs`], and
/// the panel can still say it was heard, where, and how strongly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HfdlStation {
    pub gs_id: u8,
    /// `Riverhead, New York`, or `GS 12` for an id the public roster has no
    /// name for.
    pub name: String,
    /// The station's position from the system table, where it is known.
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    /// The station's channels, in table order — which is the order the squitter
    /// bitmap indexes. Empty until a system table has been received or loaded.
    pub freqs: Vec<HfdlStationFreq>,
    /// The raw 20-bit "frequencies in use" bitmap of the latest squitter.
    pub active_bits: u32,
    /// Unix second [`Self::active_bits`] was last received; 0 = never.
    pub active_at: i64,
    /// Unix second a transmission of the station itself (a squitter or an
    /// uplink) was last decoded; 0 = never.
    pub heard_at: i64,
    /// The channel that transmission was on, in kHz.
    pub heard_khz: u32,
    /// EVM-derived SNR of that transmission, where the demod measured one.
    pub snr_db: Option<f32>,
    /// Squitters decoded from it since the decoder started.
    pub squitters: u32,
    /// Whether the station says it is slaved to UTC (squitter flag).
    pub utc_sync: bool,
}

impl HfdlStation {
    /// A station the decoder has only just learned of.
    pub fn new(gs_id: u8, name: Option<&str>) -> Self {
        Self {
            gs_id,
            name: name.map_or_else(|| format!("GS {gs_id}"), str::to_owned),
            lat: None,
            lon: None,
            freqs: Vec::new(),
            active_bits: 0,
            active_at: 0,
            heard_at: 0,
            heard_khz: 0,
            snr_db: None,
            squitters: 0,
            utc_sync: false,
        }
    }

    /// Whether this station's table entry is known, so that its bitmap can be
    /// read as frequencies.
    pub fn has_table(&self) -> bool {
        !self.freqs.is_empty()
    }

    /// The channels the latest squitter listed as in use, in kHz. Empty when
    /// the table is unknown — the bitmap is [`Self::active_bits`] then.
    pub fn active_khz(&self) -> Vec<u32> {
        self.freqs.iter().filter(|f| f.active).map(HfdlStationFreq::khz).collect()
    }

    /// Whether the table gives this station a channel at `khz`.
    pub fn carries(&self, khz: u32) -> bool {
        self.freqs.iter().any(|f| f.khz() == khz)
    }

    /// Re-derive each channel's `active` flag from [`Self::active_bits`]. The
    /// bitmap's bit *n* is the station's *n*-th table frequency (the table's
    /// frequency count is bounded by the bitmap's 20 bits for exactly this
    /// reason).
    pub fn apply_active_bits(&mut self) {
        for (i, f) in self.freqs.iter_mut().enumerate() {
            f.active = i < 20 && self.active_bits & (1 << i) != 0;
        }
    }

    /// The part of the station that does not change from squitter to squitter —
    /// what is worth keeping on disk. Everything heard is dropped: it would be
    /// wrong the next time the program runs.
    pub fn table_entry(&self) -> HfdlStation {
        let mut e = HfdlStation::new(self.gs_id, Some(&self.name));
        e.lat = self.lat;
        e.lon = self.lon;
        e.freqs =
            self.freqs.iter().map(|f| HfdlStationFreq { active: false, ..f.clone() }).collect();
        e
    }

    /// Whether what was heard from the station is recent enough to show as
    /// current, at `now` (Unix seconds).
    pub fn is_fresh(&self, now: i64) -> bool {
        self.heard_at > 0 && now - self.heard_at <= HFDL_ACTIVE_FRESH_S
    }

    /// The channels this station is known to be working right now, in kHz: the
    /// one it was just heard on, and — when its squitter is fresh and the table
    /// turns the bitmap into frequencies — the others it lists as in use.
    pub fn working_khz(&self, now: i64) -> Vec<u32> {
        let mut out = Vec::new();
        if self.is_fresh(now) && self.heard_khz > 0 {
            out.push(self.heard_khz);
        }
        if self.active_at > 0 && now - self.active_at <= HFDL_ACTIVE_FRESH_S {
            out.extend(self.active_khz());
        }
        out.sort_unstable();
        out.dedup();
        out
    }
}

/// Every channel any station is known to be working at `now`, sorted and
/// unique — what lights the channel chips.
pub fn hfdl_working_khz(stations: &[HfdlStation], now: i64) -> Vec<u32> {
    let mut out: Vec<u32> = stations.iter().flat_map(|s| s.working_khz(now)).collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// The channels to offer as chips: the union of every known station's table
/// frequencies, ascending — and `fallback` (the published plan) when no table
/// is known yet. Once the live table is in, it replaces the plan outright: it
/// is what the network is actually using, and a stale hard-coded allocation
/// would offer channels that were retired.
pub fn hfdl_plan_khz(stations: &[HfdlStation], fallback: &[u32]) -> Vec<u32> {
    let mut out: Vec<u32> = stations.iter().flat_map(|s| s.freqs.iter().map(|f| f.khz())).collect();
    if out.is_empty() {
        return fallback.to_vec();
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// The stations whose table entry carries `khz`, for a chip's tooltip.
pub fn hfdl_stations_on(stations: &[HfdlStation], khz: u32) -> Vec<&HfdlStation> {
    stations.iter().filter(|s| s.carries(khz)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn station(id: u8, khz: &[u32]) -> HfdlStation {
        let mut s = HfdlStation::new(id, Some("Test"));
        s.freqs = khz
            .iter()
            .enumerate()
            .map(|(i, k)| HfdlStationFreq { freq_hz: k * 1000, slot: i as u8, active: false })
            .collect();
        s
    }

    #[test]
    fn the_bitmap_lights_the_matching_table_channels() {
        let mut s = station(4, &[5_451, 8_912, 21_931]);
        s.active_bits = 0b101;
        s.apply_active_bits();
        assert_eq!(s.active_khz(), vec![5_451, 21_931]);
        // A later squitter replaces the set, it does not add to it.
        s.active_bits = 0b010;
        s.apply_active_bits();
        assert_eq!(s.active_khz(), vec![8_912]);
    }

    #[test]
    fn working_channels_are_the_one_heard_plus_the_fresh_active_ones() {
        let mut s = station(4, &[5_451, 8_912, 21_931]);
        s.heard_at = 1_000;
        s.heard_khz = 21_931;
        s.active_at = 1_000;
        s.active_bits = 0b011;
        s.apply_active_bits();
        assert_eq!(s.working_khz(1_100), vec![5_451, 8_912, 21_931]);
        // Past the freshness window neither claim stands.
        assert!(s.working_khz(1_000 + HFDL_ACTIVE_FRESH_S + 1).is_empty());
    }

    #[test]
    fn a_station_never_heard_is_working_nowhere() {
        let mut s = station(4, &[5_451]);
        s.active_bits = 1;
        s.apply_active_bits();
        // A table entry alone (loaded from disk) is not evidence of anything.
        assert!(s.working_khz(10).is_empty());
    }

    #[test]
    fn the_plan_is_the_live_table_once_there_is_one() {
        let fallback = [21_931, 2_941];
        assert_eq!(hfdl_plan_khz(&[], &fallback), fallback.to_vec());
        // A station heard with no table keeps the fallback too.
        assert_eq!(hfdl_plan_khz(&[HfdlStation::new(4, None)], &fallback), fallback.to_vec());
        let live = [station(4, &[8_912, 5_451]), station(7, &[5_451, 3_455])];
        assert_eq!(hfdl_plan_khz(&live, &fallback), vec![3_455, 5_451, 8_912]);
    }

    #[test]
    fn stations_on_a_channel_come_from_the_table() {
        let live = [station(4, &[8_912, 5_451]), station(7, &[5_451, 3_455])];
        let on: Vec<u8> = hfdl_stations_on(&live, 5_451).iter().map(|s| s.gs_id).collect();
        assert_eq!(on, vec![4, 7]);
        assert!(hfdl_stations_on(&live, 21_931).is_empty());
    }

    #[test]
    fn working_channels_across_stations_are_sorted_and_unique() {
        let mut a = station(4, &[5_451]);
        a.heard_at = 10;
        a.heard_khz = 5_451;
        let mut b = station(7, &[5_451, 3_455]);
        b.heard_at = 10;
        b.heard_khz = 3_455;
        assert_eq!(hfdl_working_khz(&[a, b], 20), vec![3_455, 5_451]);
    }

    #[test]
    fn the_saved_entry_drops_everything_heard() {
        let mut s = station(4, &[5_451]);
        s.lat = Some(40.95);
        s.heard_at = 5;
        s.squitters = 3;
        s.active_bits = 1;
        s.apply_active_bits();
        let e = s.table_entry();
        assert_eq!((e.heard_at, e.squitters, e.active_bits), (0, 0, 0));
        assert_eq!((e.lat, e.freqs.len(), e.freqs[0].active), (Some(40.95), 1, false));
    }

    #[test]
    fn a_table_entry_round_trips_through_json() {
        // It goes to disk as JSON between runs.
        let s = station(4, &[5_451, 8_912]).table_entry();
        let back: HfdlStation = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back, s);
    }
}
