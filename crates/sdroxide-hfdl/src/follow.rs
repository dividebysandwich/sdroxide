//! Auto-follow: find a band where ground stations can be heard, and stay there.
//!
//! HFDL is ordinary shortwave: which band is open depends on the time of day
//! and the path, and a channel that decodes nothing is usually a closed band,
//! not a broken decoder. Every ground station sends a squitter on each of its
//! channels about every 32 seconds whether or not any aircraft is talking, so
//! a band that is open says so within a minute. That makes a simple, honest
//! search possible:
//!
//! * **Parked** — a ground station has been heard on one of the channels being
//!   decoded within the last [`PARK_S`]. Stay.
//! * **Listening** — nothing heard yet on this band; give it [`DWELL_S`]: about
//!   three squitter periods, so one missed to fading does not move us on.
//! * Then **hop** to the next band in the order the time of day suggests,
//!   taking every channel of that band that fits one lane (see
//!   [`sdroxide_types::hfdl_lane_plan`]), so a band is tried all at once rather
//!   than one channel per minute.
//!
//! The order: in local daytime (by the sun at the operator's longitude) the
//! higher bands first, at night the lower ones, around dawn and dusk the middle
//! of the spectrum; and before any of them, a band where some station's latest
//! squitter says it is working a channel — that is direct evidence the band is
//! in use, even if this receiver cannot hear it yet.
//!
//! Pure: the engine feeds it the status and the time and applies the hop.

use sdroxide_types::{
    HFDL_MAX_CHANNELS, HFDL_MAX_SPAN_HZ, HfdlFollow, HfdlFollowState, HfdlStation, hfdl_working_khz,
};

/// A station heard this recently on a channel being decoded means the band is
/// open: stay.
pub const PARK_S: i64 = 120;

/// How long a band is listened to before moving on, with nothing heard.
pub const DWELL_S: i64 = 100;

/// One band's channels, as a lane: the dial channel first.
#[derive(Debug, Clone, PartialEq)]
pub struct BandLane {
    pub band_mhz: u32,
    pub primary_khz: u32,
    pub also_khz: Vec<u32>,
}

#[derive(Debug, Default)]
pub struct Follow {
    /// When the band being decoded was chosen (or last had a station heard).
    since: i64,
    /// The band the last decision was about, so a hop is noticed as one.
    band: Option<u32>,
}

/// The band a channel is in, by its whole megahertz: 13 276 kHz is the 13 MHz
/// band. HFDL's allocations sit well inside each.
pub fn band_of(khz: u32) -> u32 {
    khz / 1000
}

/// Band order for the local solar hour `h` (0..24) at the operator's station.
pub fn band_order(h: f64) -> &'static [u32] {
    const DAY: &[u32] = &[21, 17, 15, 13, 11, 10, 8, 6, 5, 4, 3, 2];
    const NIGHT: &[u32] = &[5, 6, 8, 4, 3, 10, 11, 2, 13, 15, 17, 21];
    const TWILIGHT: &[u32] = &[8, 10, 11, 13, 6, 5, 17, 15, 4, 3, 21, 2];
    match h {
        h if (7.0..17.0).contains(&h) => DAY,
        h if (5.0..7.0).contains(&h) || (17.0..20.0).contains(&h) => TWILIGHT,
        _ => NIGHT,
    }
}

/// The local solar hour at longitude `lon` (degrees east) at `unix`, or plain
/// UTC when the station's position is unknown.
pub fn solar_hour(unix: i64, lon: Option<f64>) -> f64 {
    let utc_h = (unix.rem_euclid(86_400)) as f64 / 3600.0;
    (utc_h + lon.unwrap_or(0.0) / 15.0).rem_euclid(24.0)
}

/// The lane for `band`: the most channels of it, across every station's table,
/// that fit within [`HFDL_MAX_SPAN_HZ`] — up to [`HFDL_MAX_CHANNELS`] — with
/// the channel the most stations share as the dial. `None` when no station has
/// a channel in that band.
pub fn band_lane(band: u32, stations: &[HfdlStation]) -> Option<BandLane> {
    let mut all: Vec<u32> = stations
        .iter()
        .flat_map(|s| s.freqs.iter().map(|f| f.khz()))
        .filter(|&k| band_of(k) == band)
        .collect();
    all.sort_unstable();
    all.dedup();
    if all.is_empty() {
        return None;
    }
    // The widest run of channels inside the span limit.
    let span_khz = (HFDL_MAX_SPAN_HZ / 1000.0) as u32;
    let (mut best_lo, mut best_n) = (0, 0);
    for lo in 0..all.len() {
        let n = all[lo..].iter().take_while(|&&k| k - all[lo] <= span_khz).count();
        if n > best_n {
            (best_lo, best_n) = (lo, n);
        }
    }
    let mut run: Vec<u32> = all[best_lo..best_lo + best_n].to_vec();
    // Too many for one lane: keep those the most stations use.
    let users = |k: u32| stations.iter().filter(|s| s.carries(k)).count();
    run.sort_by_key(|&k| (std::cmp::Reverse(users(k)), k));
    run.truncate(HFDL_MAX_CHANNELS);
    let primary = run[0];
    let mut also: Vec<u32> = run[1..].to_vec();
    also.sort_unstable();
    Some(BandLane { band_mhz: band, primary_khz: primary, also_khz: also })
}

impl Follow {
    pub fn new() -> Self {
        Self::default()
    }

    /// Decide, at `now`, given the channels being decoded and the stations as
    /// the decoder knows them. Returns what to report, and the lane to move to
    /// when it is time to hop.
    pub fn step(
        &mut self,
        now: i64,
        decoding: &[u32],
        stations: &[HfdlStation],
        lon: Option<f64>,
    ) -> (HfdlFollow, Option<BandLane>) {
        let band = decoding.first().map(|&k| band_of(k)).unwrap_or(0);
        if self.band != Some(band) {
            // A new band (ours or the operator's doing): give it its dwell.
            self.band = Some(band);
            self.since = now;
        }
        // Heard here, lately?
        let heard = stations
            .iter()
            .filter(|s| {
                s.heard_at > 0 && now - s.heard_at <= PARK_S && decoding.contains(&s.heard_khz)
            })
            .max_by_key(|s| s.heard_at);
        if let Some(s) = heard {
            self.since = now;
            return (
                HfdlFollow {
                    state: HfdlFollowState::Parked,
                    band_mhz: band,
                    station: Some(s.name.clone()),
                    heard_at: s.heard_at,
                    next_hop_at: 0,
                },
                None,
            );
        }
        let next_hop_at = self.since + DWELL_S;
        if now < next_hop_at {
            return (
                HfdlFollow {
                    state: HfdlFollowState::Listening,
                    band_mhz: band,
                    station: None,
                    heard_at: 0,
                    next_hop_at,
                },
                None,
            );
        }
        // Time to move on.
        let next = self.next_band(band, now, stations, lon);
        let report = HfdlFollow {
            state: HfdlFollowState::Listening,
            band_mhz: next.as_ref().map_or(band, |l| l.band_mhz),
            station: None,
            heard_at: 0,
            next_hop_at: now + DWELL_S,
        };
        if let Some(l) = &next {
            self.band = Some(l.band_mhz);
            self.since = now;
        } else {
            // Nowhere to go (no table at all): try again after another dwell.
            self.since = now;
        }
        (report, next)
    }

    /// The band after `band` in today's order, bands with a station known to be
    /// working a channel first; one that has channels in some station's table.
    fn next_band(
        &self,
        band: u32,
        now: i64,
        stations: &[HfdlStation],
        lon: Option<f64>,
    ) -> Option<BandLane> {
        let order = candidate_order(now, stations, lon);
        let start = order.iter().position(|&b| b == band).map_or(0, |i| i + 1);
        (0..order.len())
            .map(|i| order[(start + i) % order.len()])
            .filter(|&b| b != band)
            .find_map(|b| band_lane(b, stations))
    }
}

/// The bands in the order to try them at `now`: those with fresh evidence of a
/// station working one of their channels first (in the time-of-day order among
/// themselves), then the rest by time of day.
pub fn candidate_order(now: i64, stations: &[HfdlStation], lon: Option<f64>) -> Vec<u32> {
    let by_time = band_order(solar_hour(now, lon));
    let working: Vec<u32> = hfdl_working_khz(stations, now).into_iter().map(band_of).collect();
    let mut out: Vec<u32> = by_time.iter().copied().filter(|b| working.contains(b)).collect();
    out.extend(by_time.iter().copied().filter(|b| !working.contains(b)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_types::hfdl_builtin_stations;

    #[test]
    fn the_band_order_follows_the_sun() {
        assert_eq!(band_order(12.0)[0], 21, "midday: the highest band first");
        assert_eq!(band_order(1.0)[0], 5, "night: the low bands");
        assert_eq!(band_order(6.0)[0], 8, "dawn: the middle");
        assert_eq!(band_order(18.5)[0], 8, "dusk: the middle");
        // Every order names the same bands.
        let mut a = band_order(12.0).to_vec();
        let mut b = band_order(1.0).to_vec();
        a.sort();
        b.sort();
        assert_eq!(a, b);
    }

    #[test]
    fn solar_hour_is_utc_shifted_by_longitude() {
        let noon_utc = 12 * 3600;
        assert_eq!(solar_hour(noon_utc, None), 12.0);
        assert_eq!(
            solar_hour(noon_utc, Some(-75.0)),
            7.0,
            "New York: 5 h behind the sun at Greenwich"
        );
        assert_eq!(solar_hour(noon_utc, Some(150.0)), 22.0);
    }

    #[test]
    fn a_band_lane_takes_its_channels_in_one_lane() {
        let st = hfdl_builtin_stations();
        let l = band_lane(13, &st).expect("13 MHz has channels");
        let all: Vec<u32> =
            std::iter::once(l.primary_khz).chain(l.also_khz.iter().copied()).collect();
        assert!(all.iter().all(|&k| band_of(k) == 13));
        let (lo, hi) = (all.iter().min().unwrap(), all.iter().max().unwrap());
        assert!(f64::from(hi - lo) * 1000.0 <= HFDL_MAX_SPAN_HZ);
        assert!(all.len() <= HFDL_MAX_CHANNELS);
        // 13 276 kHz is shared by three stations, more than any other channel
        // of the band: it is the dial.
        assert_eq!(l.primary_khz, 13_276);
        assert!(band_lane(7, &st).is_none(), "no HFDL channels at 7 MHz");
    }

    fn heard(mut st: Vec<HfdlStation>, id: u8, khz: u32, at: i64) -> Vec<HfdlStation> {
        let s = st.iter_mut().find(|s| s.gs_id == id).unwrap();
        s.heard_at = at;
        s.heard_khz = khz;
        st
    }

    #[test]
    fn a_band_with_a_station_heard_is_kept() {
        let mut f = Follow::new();
        let st = heard(hfdl_builtin_stations(), 4, 13_276, 1_000);
        let (r, hop) = f.step(1_010, &[13_276, 13_312], &st, None);
        assert_eq!(hop, None);
        assert_eq!(r.state, HfdlFollowState::Parked);
        assert_eq!(r.station.as_deref(), Some("Riverhead, New York"));
        // Still parked long after the dwell, as long as it keeps being heard.
        let st = heard(st, 4, 13_276, 1_500);
        let (r, hop) = f.step(1_510, &[13_276, 13_312], &st, None);
        assert_eq!((r.state, hop), (HfdlFollowState::Parked, None));
    }

    #[test]
    fn a_silent_band_is_given_its_dwell_then_left() {
        let mut f = Follow::new();
        let st = hfdl_builtin_stations();
        // Noon UTC at Greenwich: daytime order, 21 MHz first, so from 21 the
        // next is 17.
        let t0 = 12 * 3600;
        let (r, hop) = f.step(t0, &[21_931], &st, Some(0.0));
        assert_eq!((r.state, hop.clone()), (HfdlFollowState::Listening, None));
        assert_eq!(r.next_hop_at, t0 + DWELL_S);
        let (_, hop) = f.step(t0 + DWELL_S - 1, &[21_931], &st, Some(0.0));
        assert_eq!(hop, None, "not before the dwell is up");
        let (_, hop) = f.step(t0 + DWELL_S, &[21_931], &st, Some(0.0));
        let l = hop.expect("time to move on");
        assert_eq!(l.band_mhz, 17);
        // The engine applies it; next step sees the new band and gives it a
        // full dwell of its own.
        let decoding: Vec<u32> = std::iter::once(l.primary_khz).chain(l.also_khz).collect();
        let (r, hop) = f.step(t0 + DWELL_S + 1, &decoding, &st, Some(0.0));
        assert_eq!((r.state, hop), (HfdlFollowState::Listening, None));
        assert_eq!(r.band_mhz, 17);
    }

    #[test]
    fn a_station_heard_long_ago_or_on_another_channel_does_not_hold_the_band() {
        let mut f = Follow::new();
        let st = heard(hfdl_builtin_stations(), 4, 8_912, 1_000);
        // Heard, but on 8 MHz while we decode 13: no reason to stay on 13.
        let (r, _) = f.step(1_010, &[13_276], &st, None);
        assert_eq!(r.state, HfdlFollowState::Listening);
        // Heard here, but too long ago.
        let mut g = Follow::new();
        let (r, _) = g.step(1_000 + PARK_S + 1, &[8_912], &st, None);
        assert_eq!(r.state, HfdlFollowState::Listening);
    }

    #[test]
    fn a_band_a_station_says_it_is_working_is_tried_first() {
        // Midnight at Greenwich would start with 5 MHz — but a fresh squitter
        // from Shannon says it is working 11 384 kHz. (A day in: a time of 0
        // reads as "never heard".)
        let now = 86_400;
        let mut st = hfdl_builtin_stations();
        let shannon = st.iter_mut().find(|s| s.gs_id == 7).unwrap();
        shannon.heard_at = now - 10;
        shannon.heard_khz = 8_942;
        shannon.active_at = now - 10;
        shannon.active_bits = 0b1; // its first channel: 11 384 kHz
        shannon.apply_active_bits();
        let order = candidate_order(now, &st, Some(0.0));
        assert_eq!(order[0], 8, "where it was heard");
        assert!(order[..2].contains(&11), "where it says it is working: {order:?}");
        assert_eq!(order.len(), band_order(0.0).len());
        assert_eq!(
            band_order(solar_hour(now, Some(0.0)))[0],
            5,
            "without the evidence, 5 MHz would lead"
        );
    }

    #[test]
    fn with_no_table_there_is_nowhere_to_hop_to() {
        let mut f = Follow::new();
        let (_, hop) = f.step(0, &[21_931], &[], None);
        assert_eq!(hop, None);
        let (r, hop) = f.step(DWELL_S + 1, &[21_931], &[], None);
        assert_eq!(hop, None);
        assert_eq!(r.next_hop_at, 2 * DWELL_S + 1, "it tries again after another dwell");
    }
}
