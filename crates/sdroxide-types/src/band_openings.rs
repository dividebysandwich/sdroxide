// Portions of this file are adapted from OpenHamClock's
// `server/utils/bandOpenings.js` (<https://github.com/accius/openhamclock>),
// which carries this notice:
//
// MIT License
//
// Copyright (c) 2024-2026 OpenHamClock Contributors
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! Band-opening detection over recent spot activity.
//!
//! The idea: a listener watches for openings, not for spots. Most of the time
//! the feeds produce a trickle of spots on a path; when the path *opens* the
//! trickle becomes a burst. So each (band × DX-continent → spotter-continent)
//! key compares a short trailing window against a long baseline, and a path
//! whose short-window rate has surged past its baseline by a factor, with
//! enough distinct calls, is named an *opening* — then tracked through a
//! hysteresis state machine (opening → active → closing) so the UI does not
//! flap at the threshold.
//!
//! # What the rates count
//!
//! The caller decides what "a record" is by the stable `id` it gives each
//! [`BandPath`]; this module only sees distinct ids. A station heard in two
//! different short windows is two records, so a path that stays busy keeps
//! contributing rather than counting its callers once ever — deduping on the id
//! alone made a station still active three hours later count as new again as
//! its entry aged out. The rates are therefore distinct calls per short-window
//! bucket.
//!
//! # The warm-up
//!
//! Everything is deterministic on injected timestamps: the tracker holds no
//! timers and does no I/O, so it is directly unit-testable and a restart is
//! harmless — the baseline simply warms back up. The feeds hand over only about
//! one short window of spots (PSK Reporter's query is 15 minutes), so at launch
//! there is nothing older to rest against; the baseline period is scaled,
//! **per key**, to the history actually observed rather than the nominal 2h45m,
//! and a silent baseline is trusted as an opening only once at least a short
//! window of it has been seen for that key. Per key and not once for the whole
//! tracker: a feed is only polled for the band the dial is on, so a path on a
//! band just switched to has no history of its own even when the tracker has
//! been up for hours.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::Band;

/// The knobs of the analysis, mirroring OpenHamClock's defaults.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OpenOptions {
    /// Trailing "right now" window.
    pub short_window_s: i64,
    /// Trailing baseline window.
    pub baseline_window_s: i64,
    /// The short-window rate suspects an opening when it exceeds the baseline
    /// rate by this factor…
    pub open_factor: f64,
    /// …and the short window holds at least this many distinct calls, so a
    /// quiet band (baseline ≈ 0) cannot false-positive on a couple of spots.
    pub min_distinct_calls: usize,
    /// How long a `Closing` entry stays visible once a path is back to normal.
    pub closing_linger_s: i64,
    /// Hard memory cap per (band × path) key.
    pub max_spots_per_key: usize,
}

impl Default for OpenOptions {
    fn default() -> Self {
        OpenOptions {
            short_window_s: 15 * 60,
            baseline_window_s: 3 * 60 * 60,
            open_factor: 3.0,
            min_distinct_calls: 5,
            closing_linger_s: 10 * 60,
            max_spots_per_key: 20_000,
        }
    }
}

/// Where a path is in its surge lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpeningState {
    /// The criteria were just met; the path has no track record yet.
    Opening,
    /// Still above the (lower) close criteria, so it is an established event.
    Active,
    /// Dropped below even the close criteria; visible briefly, then gone.
    Closing,
}

impl OpeningState {
    pub fn label(self) -> &'static str {
        match self {
            OpeningState::Opening => "opening",
            OpeningState::Active => "active",
            OpeningState::Closing => "closing",
        }
    }
}

/// One spot told to the tracker.
#[derive(Debug, Clone)]
pub struct BandPath {
    /// The DX (spotted) callsign.
    pub call: String,
    pub band: Band,
    /// The DX side's continent (`NA`, `EU`, …).
    pub from_continent: &'static str,
    /// The spotters' (hearing) side's continent.
    pub to_continent: &'static str,
    /// Spot time, unix seconds.
    pub timestamp: i64,
    /// Stable id (any string is fine) so repeated ingests of the same cache
    /// snapshot dedupe; when `None`, one is derived from the other fields.
    pub id: Option<String>,
}

/// One live or dying opening, as analysed for the caller.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BandOpening {
    pub band: Band,
    /// The DX side's continent.
    pub from_continent: String,
    /// The spotters' side's continent.
    pub to_continent: String,
    pub state: OpeningState,
    /// When the path entered its current state, unix seconds.
    pub since_utc: i64,
    /// Distinct DX calls seen in the short window.
    pub short_calls: usize,
    /// Baseline spot rate (spots per minute) for the path.
    pub baseline_per_min: f64,
    /// Short/baseline rate ratio; `None` serializes the JavaScript's
    /// `Infinity`, meaning a silent path that has suddenly come alive.
    pub factor: Option<f64>,
    /// Up to three of the most recent calls behind the surge.
    pub sample_calls: Vec<String>,
}

type Key = (Band, &'static str, &'static str);

#[derive(Debug, Clone)]
struct SpotRec {
    call: String,
    ts: i64,
}

#[derive(Debug, Clone, Copy)]
struct StateRec {
    state: OpeningState,
    since: i64,
    closing_since: Option<i64>,
}

/// Tracks spot activity per (band × path) and names the openings.
#[derive(Debug)]
pub struct BandOpeningTracker {
    opts: OpenOptions,
    spots: HashMap<Key, Vec<SpotRec>>,
    /// `(spot id, short-window bucket)` → the bucket's latest timestamp, for
    /// dedupe across repeated ingests of the same cache snapshot. The bucket is
    /// what lets a station still active in a later window count again instead
    /// of being suppressed forever by its first appearance.
    seen: HashMap<(String, i64), i64>,
    states: HashMap<Key, StateRec>,
}

impl Default for BandOpeningTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl BandOpeningTracker {
    pub fn new() -> Self {
        Self::with_options(OpenOptions::default())
    }

    pub fn with_options(opts: OpenOptions) -> Self {
        BandOpeningTracker { opts, spots: HashMap::new(), seen: HashMap::new(), states: HashMap::new() }
    }

    pub fn options(&self) -> OpenOptions {
        self.opts
    }

    /// Feed spots; returns how many were actually accepted (new, valid, and
    /// inside the retention window).
    pub fn ingest(&mut self, spots: &[BandPath], now: i64) -> usize {
        let cutoff = now - self.opts.baseline_window_s;
        let mut accepted = 0;
        for s in spots {
            if s.call.is_empty() || s.from_continent.is_empty() || s.to_continent.is_empty() {
                continue;
            }
            if s.timestamp <= cutoff || s.timestamp > now + 60 {
                continue;
            }
            let id = s.id.clone().unwrap_or_else(|| {
                format!("{}|{:?}|{}|{}", s.call, s.band, s.from_continent, s.to_continent)
            });
            // One record per (id, short-window bucket): a station heard again in
            // a later window is fresh activity, not a repeat of its first spot.
            let bucket = s.timestamp.div_euclid(self.opts.short_window_s);
            if self.seen.contains_key(&(id.clone(), bucket)) {
                continue;
            }
            self.seen.insert((id, bucket), s.timestamp);
            let key = (s.band, s.from_continent, s.to_continent);
            let list = self.spots.entry(key).or_default();
            list.push(SpotRec { call: s.call.clone(), ts: s.timestamp });
            if list.len() > self.opts.max_spots_per_key {
                let drop = list.len() - self.opts.max_spots_per_key;
                list.drain(..drop);
            }
            accepted += 1;
        }
        self.prune(now);
        accepted
    }

    /// Analyse current activity, advancing the per-key state machine. Returns
    /// the live entries sorted strongest-first (`opening` before `active`,
    /// and both before `closing`; within a rank, highest surge factor first).
    pub fn analyze(&mut self, now: i64) -> Vec<BandOpening> {
        self.prune(now);
        let short_cutoff = now - self.opts.short_window_s;
        // The baseline is only as long as the history actually observed *for
        // this key*. The feeds hand over about one short window of spots — PSK
        // Reporter's query is 15 minutes — and a feed is only polled for the
        // band the dial is on, so a path just switched to has no history even
        // when the tracker has been up for hours. Per key, dividing an empty
        // baseline by the nominal 2h45m would turn every busy path into an
        // infinite surge; "no history older than the short window for this key"
        // is treated as no baseline evidence at all.
        let nominal_baseline_s = self.opts.baseline_window_s - self.opts.short_window_s;
        let close_factor = self.opts.open_factor / 2.0;
        let close_distinct = std::cmp::max(2, self.opts.min_distinct_calls.div_ceil(2));

        let mut results: Vec<BandOpening> = Vec::new();
        let mut live_keys: std::collections::HashSet<Key> = std::collections::HashSet::new();

        for (key, list) in &self.spots {
            let mut short_calls: HashMap<&str, ()> = HashMap::new();
            let mut short_spots = 0u64;
            let mut baseline_spots = 0u64;
            for s in list {
                if s.ts > short_cutoff {
                    short_spots += 1;
                    short_calls.insert(&s.call, ());
                } else {
                    baseline_spots += 1;
                }
            }
            let short_count = short_calls.len();
            let short_rate = short_spots as f64 / (self.opts.short_window_s as f64 / 60.0);
            // This key's own observed baseline span: from its oldest surviving
            // spot to the start of the short window.
            let key_oldest = list.iter().map(|s| s.ts).min().unwrap_or(now);
            let observed_baseline_s =
                (now - key_oldest - self.opts.short_window_s).clamp(0, nominal_baseline_s);
            let baseline_rate = if observed_baseline_s > 0 {
                baseline_spots as f64 / (observed_baseline_s as f64 / 60.0)
            } else {
                0.0
            };
            // A silent baseline is evidence of an opening only once enough of
            // it has been observed to mean anything; before that a busy short
            // window is just the app having started. A short window with no
            // history behind it cannot open anything.
            let factor = if observed_baseline_s <= 0 {
                0.0
            } else if baseline_rate > 0.0 {
                short_rate / baseline_rate
            } else if short_spots > 0 && observed_baseline_s >= self.opts.short_window_s {
                f64::INFINITY
            } else {
                0.0
            };

            let meets_open =
                short_count >= self.opts.min_distinct_calls && factor >= self.opts.open_factor;
            let meets_close = short_count >= close_distinct && factor >= close_factor;

            let prev = self.states.get(key);
            let mut state: Option<(OpeningState, StateRec)> = None;
            match prev {
                None | Some(StateRec { state: OpeningState::Closing, .. }) => {
                    if meets_open {
                        // First detection: store `active` (openhamclock does the
                        // same) but report the entry as `opening`. A path that
                        // closed and comes back is a new event, so its age starts
                        // here rather than inheriting the previous event's.
                        let rec =
                            StateRec { state: OpeningState::Active, since: now, closing_since: None };
                        self.states.insert(*key, rec);
                        state = Some((OpeningState::Opening, rec));
                    } else if let Some(p) = prev {
                        if now - p.closing_since.unwrap_or(now) > self.opts.closing_linger_s {
                            self.states.remove(key);
                        } else {
                            state = Some((OpeningState::Closing, *p));
                        }
                    }
                }
                Some(p) => {
                    if meets_close {
                        let rec = StateRec {
                            state: OpeningState::Active,
                            since: p.since,
                            closing_since: None,
                        };
                        self.states.insert(*key, rec);
                        state = Some((OpeningState::Active, rec));
                    } else {
                        let rec = StateRec {
                            state: OpeningState::Closing,
                            since: p.since,
                            closing_since: Some(now),
                        };
                        self.states.insert(*key, rec);
                        state = Some((OpeningState::Closing, rec));
                    }
                }
            }

            if let Some((st, rec)) = state {
                live_keys.insert(*key);
                let factor_out = if factor.is_infinite() { None } else { Some(round(factor, 2)) };
                // Up to three of the most recent short-window calls, each once.
                // The feeds interleave and a cluster returns newest-first, so
                // the list is not in time order: sort it rather than trusting
                // it, and skip repeats of a call already named.
                let mut recent: Vec<(&i64, &String)> =
                    list.iter().filter(|s| s.ts > short_cutoff).map(|s| (&s.ts, &s.call)).collect();
                recent.sort_by(|a, b| b.0.cmp(a.0));
                let mut named: std::collections::HashSet<&str> = std::collections::HashSet::new();
                let mut sample: Vec<String> = Vec::new();
                for (_, call) in recent {
                    if named.insert(call.as_str()) {
                        sample.push(call.clone());
                        if sample.len() == 3 {
                            break;
                        }
                    }
                }
                results.push(BandOpening {
                    band: key.0,
                    from_continent: key.1.to_string(),
                    to_continent: key.2.to_string(),
                    state: st,
                    since_utc: rec.since,
                    short_calls: short_count,
                    baseline_per_min: round(baseline_rate, 3),
                    factor: factor_out,
                    sample_calls: sample,
                });
            }
        }

        // Keys whose spots aged out entirely can still hold stale state.
        for key in self.states.keys().cloned().collect::<Vec<_>>() {
            if !live_keys.contains(&key) && !self.spots.contains_key(&key) {
                self.states.remove(&key);
            }
        }

        let rank = |s: OpeningState| match s {
            OpeningState::Opening => 0,
            OpeningState::Active => 1,
            OpeningState::Closing => 2,
        };
        results.sort_by(|a, b| {
            rank(a.state).cmp(&rank(b.state)).then_with(|| {
                b.factor.unwrap_or(f64::INFINITY).total_cmp(&a.factor.unwrap_or(f64::INFINITY))
            })
        });
        results
    }

    /// Records held, for the tests' count assertions.
    #[cfg(test)]
    fn spot_count(&self) -> usize {
        self.spots.values().map(|l| l.len()).sum()
    }

    fn prune(&mut self, now: i64) {
        let cutoff = now - self.opts.baseline_window_s;
        self.spots.retain(|_, list| {
            list.retain(|s| s.ts > cutoff);
            !list.is_empty()
        });
        self.seen.retain(|_, ts| *ts > cutoff);
    }
}

fn round(x: f64, places: u32) -> f64 {
    let m = 10f64.powi(places as i32);
    (x * m).round() / m
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: i64 = 60;
    const NOW: i64 = 10 * 60 * 60; // deterministic "now" (10 h epoch offset)

    fn spot(call: &str, ts_offset_min: i64, overrides: &[(&'static str, String)]) -> BandPath {
        let mut n = 0;
        for (k, v) in overrides {
            if *k == "n" {
                n = v.parse().unwrap();
            }
        }
        BandPath {
            call: call.to_string(),
            band: Band::M20,
            from_continent: "EU",
            to_continent: "NA",
            timestamp: NOW + ts_offset_min * MIN,
            id: Some(format!("{call}|{ts_offset_min}|{n}")),
        }
    }

    fn baseline_spots(every_min: i64, calls: &[&str]) -> Vec<BandPath> {
        let mut out = Vec::new();
        let mut n = 0;
        let mut m = -175;
        while m <= -20 {
            out.push(spot(calls[n % calls.len()], m, &[("n", n.to_string())]));
            n += 1;
            m += every_min;
        }
        out
    }

    fn burst_spots(count: usize) -> Vec<BandPath> {
        (0..count)
            .map(|i| spot(&format!("DX{i}AA"), -1 - (i as i64 % 10), &[("n", i.to_string())]))
            .collect()
    }

    /// Distinct baseline spots on one key, older than the short window.
    fn key_baseline(band: Band, from: &'static str, to: &'static str) -> Vec<BandPath> {
        let calls = ["G0AAA", "G0BBB", "G0CCC"];
        (-175..=-20)
            .step_by(10)
            .enumerate()
            .map(|(n, m)| {
                let mut s = spot(calls[n % calls.len()], m, &[]);
                s.band = band;
                s.from_continent = from;
                s.to_continent = to;
                s.id = Some(format!("base|{band:?}|{from}|{to}|{n}"));
                s
            })
            .collect()
    }

    /// A short-window burst on one key.
    fn key_burst(band: Band, from: &'static str, to: &'static str, count: usize) -> Vec<BandPath> {
        (0..count)
            .map(|i| {
                let mut s = spot(&format!("DX{i}AA"), -1 - (i as i64 % 10), &[("n", i.to_string())]);
                s.band = band;
                s.from_continent = from;
                s.to_continent = to;
                s.id = Some(format!("burst|{band:?}|{from}|{to}|{i}"));
                s
            })
            .collect()
    }

    /// History on another band, 40–20 minutes back: outside the short window,
    /// inside the retention. It gives the tracker an observed baseline span
    /// without touching the path under test — which is what the warm-up gate
    /// reads, so a path with a genuinely silent baseline can still be called an
    /// opening.
    fn other_band_history(band: Band) -> Vec<BandPath> {
        (-40..=-20)
            .filter(|m| m % 5 == 0)
            .enumerate()
            .map(|(i, m)| {
                let mut s = spot(&format!("H{i}AA"), m, &[]);
                s.band = band;
                s.id = Some(format!("hist|{band:?}|{m}"));
                s
            })
            .collect()
    }

    #[test]
    fn accepts_valid_spots_and_reports_counts() {
        let mut t = BandOpeningTracker::new();
        assert_eq!(t.ingest(&[spot("G0AAA", -5, &[])], NOW), 1);
        assert_eq!(t.spot_count(), 1);
    }

    #[test]
    fn dedupes_repeated_ingests_of_the_same_spots_by_id() {
        let mut t = BandOpeningTracker::new();
        let spots = vec![spot("G0AAA", -5, &[]), spot("G0BBB", -6, &[])];
        assert_eq!(t.ingest(&spots, NOW), 2);
        assert_eq!(t.ingest(&spots, NOW), 0, "same cache snapshot re-sampled");
        assert_eq!(t.spot_count(), 2);
    }

    #[test]
    fn rejects_incomplete_stale_and_future_spots() {
        let mut t = BandOpeningTracker::new();
        let mut missing_to = spot("G0AAA", 0, &[]);
        missing_to.to_continent = "";
        let accepted = t.ingest(
            &[
                missing_to,
                spot("G0AAA", -200, &[]), // older than 3 h baseline
                spot("G0CCC", 10, &[]),   // future beyond tolerance
            ],
            NOW,
        );
        assert_eq!(accepted, 0, "every rejected spot must be refused");
        assert_eq!(t.spot_count(), 0);
    }

    #[test]
    fn prunes_spots_that_age_out_of_the_baseline_window() {
        let mut t = BandOpeningTracker::new();
        t.ingest(&[spot("G0AAA", -170, &[])], NOW);
        assert_eq!(t.spot_count(), 1);
        t.ingest(&[], NOW + 60 * MIN);
        assert_eq!(t.spot_count(), 0, "1 h later the 170-min-old spot is stale");
    }

    #[test]
    fn stays_quiet_on_steady_baseline_activity() {
        let mut t = BandOpeningTracker::new();
        t.ingest(&baseline_spots(10, &["G0AAA", "G0BBB", "G0CCC"]), NOW);
        assert_eq!(t.analyze(NOW), Vec::new());
    }

    #[test]
    fn flags_an_opening_when_the_short_window_surges_past_baseline() {
        let mut t = BandOpeningTracker::new();
        let mut all = baseline_spots(10, &["G0AAA", "G0BBB", "G0CCC"]);
        all.extend(burst_spots(8));
        t.ingest(&all, NOW);
        let openings = t.analyze(NOW);
        assert_eq!(openings.len(), 1);
        assert_eq!((openings[0].band, openings[0].state), (Band::M20, OpeningState::Opening));
        assert_eq!(openings[0].from_continent, "EU");
        assert_eq!(openings[0].to_continent, "NA");
        assert_eq!(openings[0].short_calls, 8);
        assert!(openings[0].factor.unwrap() >= OpenOptions::default().open_factor);
        // 16 baseline spots over the *observed* 160-minute span, not the nominal
        // 165: the baseline is only as long as the history seen.
        assert_eq!(openings[0].baseline_per_min, round(16.0 / 160.0, 3));
        assert!(openings[0].sample_calls.len() <= 3);
        assert!(openings[0].sample_calls.iter().all(|c| c.starts_with("DX")));
    }

    #[test]
    fn does_not_flag_a_quiet_band_on_a_tiny_burst_below_the_call_floor() {
        let mut t = BandOpeningTracker::new();
        // Zero baseline (factor would be Infinity) but only 3 distinct calls.
        t.ingest(&burst_spots(3), NOW);
        assert_eq!(t.analyze(NOW), Vec::new());
    }

    #[test]
    fn a_fresh_path_does_not_open_on_history_from_another_band() {
        let mut t = BandOpeningTracker::new();
        // History on 40 m gives the *tracker* a span, but 20 m has no history of
        // its own — a feed is only polled for the band the dial is on, so a 20 m
        // burst is not yet a change on 20 m.
        let mut all = other_band_history(Band::M40);
        all.extend(burst_spots(8));
        t.ingest(&all, NOW);
        assert_eq!(t.analyze(NOW), Vec::new(), "no baseline on this path, so no opening");
    }

    /// The maintainer's cold-start case: at launch the feeds hand over about one
    /// short window of spots and nothing older, so a path that looks busy has no
    /// baseline to have surged against. Nothing may open until a baseline has
    /// actually been observed — otherwise every busy band reads as `OPEN ∞×`
    /// the moment the app starts.
    #[test]
    fn a_busy_launch_is_not_an_opening_without_an_observed_baseline() {
        let mut t = BandOpeningTracker::new();
        // 40 distinct calls within the last 15 minutes, nothing older — what PSK
        // Reporter returns before the app has run a while.
        let spots: Vec<BandPath> = (0..40)
            .map(|i| spot(&format!("DX{i}AA"), -1 - (i as i64 % 14), &[("n", i.to_string())]))
            .collect();
        t.ingest(&spots, NOW);
        assert_eq!(t.analyze(NOW), Vec::new(), "no baseline yet, so no opening");
        // History on another band does not become this band's baseline either.
        t.ingest(&other_band_history(Band::M40), NOW);
        assert_eq!(t.analyze(NOW), Vec::new(), "another band's history is not this path's");
        // This path's own baseline, and the same busy window is a real surge.
        t.ingest(&baseline_spots(10, &["G0AAA", "G0BBB", "G0CCC"]), NOW);
        let openings = t.analyze(NOW);
        assert_eq!(openings.len(), 1);
        assert_eq!(openings[0].band, Band::M20, "the burst path, judged once its own baseline exists");
    }

    #[test]
    fn does_not_flag_heavy_but_unremarkable_traffic() {
        let mut t = BandOpeningTracker::new();
        // Busy baseline: 1 spot/min for 3 h; the short window continues at the
        // same rate, so the surge factor stays below the threshold.
        let spots: Vec<BandPath> = (-179..=-1)
            .map(|m: i64| spot(&format!("G{}XX", m.abs() % 10), m, &[("n", m.to_string())]))
            .collect();
        t.ingest(&spots, NOW);
        assert_eq!(t.analyze(NOW), Vec::new());
    }

    #[test]
    fn keeps_distinct_call_counting_distinct() {
        let mut t = BandOpeningTracker::new();
        // 20 spots of the SAME call in the short window, no baseline.
        let spots: Vec<BandPath> =
            (0..20).map(|i| spot("VK2IO", -1 - (i as i64 % 14), &[("n", i.to_string())])).collect();
        t.ingest(&spots, NOW);
        assert_eq!(t.analyze(NOW), Vec::new(), "1 distinct call < 5");
    }

    #[test]
    fn tracks_band_x_continent_pair_keys_independently() {
        let mut t = BandOpeningTracker::new();
        // The 10 m AS→NA path has its own baseline and surges; the 15 m path
        // has only a small burst and no history, so it does not open.
        let mut all = key_baseline(Band::M10, "AS", "NA");
        all.extend(key_burst(Band::M10, "AS", "NA", 6));
        all.extend(key_burst(Band::M15, "EU", "NA", 2));
        t.ingest(&all, NOW);
        let openings = t.analyze(NOW);
        assert_eq!(openings.len(), 1);
        assert_eq!((openings[0].band, openings[0].from_continent.as_str()), (Band::M10, "AS"));
    }

    #[test]
    fn transitions_opening_to_active_while_criteria_hold() {
        let mut t = BandOpeningTracker::new();
        let mut all = baseline_spots(10, &["G0AAA", "G0BBB", "G0CCC"]);
        all.extend(burst_spots(8));
        t.ingest(&all, NOW);
        assert_eq!(t.analyze(NOW)[0].state, OpeningState::Opening);
        assert_eq!(t.analyze(NOW + MIN)[0].state, OpeningState::Active);
    }

    #[test]
    fn transitions_active_to_closing_then_disappears_after_the_linger() {
        let mut t = BandOpeningTracker::new();
        let mut all = baseline_spots(10, &["G0AAA", "G0BBB", "G0CCC"]);
        all.extend(burst_spots(8));
        t.ingest(&all, NOW);
        t.analyze(NOW);
        t.analyze(NOW + MIN);

        // 40 min later the burst has left the short window entirely.
        let later = NOW + 40 * MIN;
        let closing = t.analyze(later);
        assert_eq!(closing.len(), 1);
        assert_eq!(closing[0].state, OpeningState::Closing);

        let gone = t.analyze(later + OpenOptions::default().closing_linger_s + 2 * MIN);
        assert_eq!(gone, Vec::new(), "the closing entry is dropped after its linger");
    }

    #[test]
    fn applies_hysteresis_between_the_close_and_open_thresholds() {
        let mut t = BandOpeningTracker::new();
        let mut all = baseline_spots(10, &["G0AAA", "G0BBB", "G0CCC"]);
        all.extend(burst_spots(8));
        t.ingest(&all, NOW);
        t.analyze(NOW); // opening

        // 10 min later only 4 of the 8 burst calls remain in the short window:
        // below the open criteria (5 calls, 3×) but above the close criteria
        // (3 calls, 1.5×) — so it stays active instead of flapping.
        let mid = NOW + 10 * MIN;
        let res = t.analyze(mid);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].state, OpeningState::Active);
    }

    #[test]
    fn sorts_opening_before_closing() {
        let mut t = BandOpeningTracker::new();
        // Both paths get a baseline; only the 10 m one surges now.
        let mut all = key_baseline(Band::M10, "EU", "NA");
        all.extend(key_baseline(Band::M15, "EU", "NA"));
        all.extend(key_burst(Band::M10, "EU", "NA", 8));
        t.ingest(&all, NOW);
        assert_eq!(t.analyze(NOW).len(), 1, "the 10 m path opens");

        // 40 minutes on, the 10 m burst has aged out and the 15 m one arrives.
        let later = NOW + 40 * MIN;
        let mut fifteen = key_burst(Band::M15, "EU", "NA", 6);
        for s in &mut fifteen {
            s.timestamp += 40 * MIN;
        }
        t.ingest(&fifteen, later);
        let res: Vec<(Band, OpeningState)> =
            t.analyze(later).into_iter().map(|o| (o.band, o.state)).collect();
        assert_eq!(
            res,
            vec![(Band::M15, OpeningState::Opening), (Band::M10, OpeningState::Closing)]
        );
    }

    /// The warm-up is per path, not per tracker: a path with only short-window
    /// spots has no baseline of its own however much history other paths carry.
    #[test]
    fn the_baseline_is_per_path_not_global() {
        let mut t = BandOpeningTracker::new();
        t.ingest(&key_baseline(Band::M20, "EU", "NA"), NOW);
        t.ingest(&key_burst(Band::M40, "EU", "NA", 8), NOW);
        let res = t.analyze(NOW);
        assert!(res.iter().all(|o| o.band != Band::M40), "40 m has no baseline of its own: {res:?}");
    }

    #[test]
    fn honors_custom_thresholds() {
        let mut t = BandOpeningTracker::with_options(OpenOptions {
            open_factor: 10.0,
            ..OpenOptions::default()
        });
        let mut all = baseline_spots(10, &["G0AAA", "G0BBB", "G0CCC"]);
        all.extend(burst_spots(8));
        t.ingest(&all, NOW);
        assert_eq!(t.analyze(NOW), Vec::new(), "8 calls but the surge factor is under 10×");

        let mut t2 = BandOpeningTracker::with_options(OpenOptions {
            open_factor: 2.0,
            min_distinct_calls: 2,
            ..OpenOptions::default()
        });
        let mut all = baseline_spots(10, &["G0AAA", "G0BBB", "G0CCC"]);
        all.extend(burst_spots(3));
        t2.ingest(&all, NOW);
        let res = t2.analyze(NOW);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].state, OpeningState::Opening);
    }
}
