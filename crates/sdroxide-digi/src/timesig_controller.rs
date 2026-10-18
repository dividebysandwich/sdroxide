//! `TimeSigController` — a standard time-signal station, and how far the
//! computer's clock is from it.
//!
//! The decoding is `sdroxide-timesig`'s, and names input samples rather than
//! instants. What happens here is the other half: tying those samples to the
//! computer's clock as it read when each block arrived, and turning every
//! second boundary of a believed minute into a measurement of the clock.
//!
//! # A measurement
//!
//! A second boundary at input sample `p` is UTC second `u` (from the tracker)
//! and arrived when the clock read `w` (from the sample clock). The signal left
//! the transmitter at `u`, reached the antenna a flight time `f` later, and
//! reached the decoder a receiver latency `l` after that; so the clock *should*
//! have read `u + f + l` and read `w`. The offset is the difference, positive
//! when the clock is slow. A minute of them, and their median, is the answer —
//! a desktop computer's scheduling is noisy and only ever late, and the median
//! of sixty is not moved by the late ones.
//!
//! # Receive only
//!
//! Every transmit hook is a no-op. These are national time services.

use std::collections::VecDeque;
use std::time::{Duration, SystemTime};

use sdroxide_timesig::{Decoder, Event, SampleClock, Tracker};
use sdroxide_types::{
    DigiConfig, DigiStatus, Mode, QsoStep, TIMESIG_TONE_HZ, TimeFrame, TimeSigStatus,
    TranscriptLine,
};

use crate::DigiEngine;
use crate::controller::DigiAction;

/// The window the offset is the median of, in seconds of the computer's clock.
const MEASURE_WINDOW_NS: i128 = 60 * 1_000_000_000;
/// Measurements needed before an offset is reported.
const MIN_MEASUREMENTS: usize = 10;
/// The speed of light, km per nanosecond. Long-wave ground wave is a fraction
/// of a percent slower over land; at a thousand kilometres that is ten
/// microseconds, well inside everything else.
const C_KM_PER_NS: f64 = 299_792.458e-9;

pub struct TimeSigController {
    cfg: DigiConfig,
    rate: f64,
    dec: Decoder,
    tracker: Tracker,
    clock: SampleClock,
    /// Input samples so far.
    samples: u64,
    events: Vec<Event>,
    /// The current minute's symbols, for the panel.
    symbols: Vec<u8>,
    second_of_minute: Option<u8>,
    last_frame: Option<TimeFrame>,
    frames_good: u32,
    frames_bad: u32,
    /// (clock reading the measurement was made at, UTC − arrival in ns, before
    /// the flight time and latency are added).
    meas: VecDeque<(i128, i64)>,
    distance_km: Option<f64>,
    status_dirty: bool,
    last_status: Option<SystemTime>,
    /// The newest measurement already reported to the engine.
    reported: Option<i128>,
}

impl TimeSigController {
    pub fn new(cfg: DigiConfig, tap_rate: f64) -> Self {
        let station = cfg.timesig.station;
        let need = cfg.timesig.min_confirmations();
        let mut c = TimeSigController {
            rate: tap_rate,
            dec: Decoder::new(station, tap_rate),
            tracker: Tracker::new(tap_rate, need),
            clock: SampleClock::new(tap_rate),
            samples: 0,
            events: Vec::new(),
            symbols: Vec::new(),
            second_of_minute: None,
            last_frame: None,
            frames_good: 0,
            frames_bad: 0,
            meas: VecDeque::new(),
            distance_km: None,
            status_dirty: true,
            last_status: None,
            reported: None,
            cfg,
        };
        c.distance_km = c.distance();
        c
    }

    /// From the operator's locator to the transmitter, if there is a locator.
    fn distance(&self) -> Option<f64> {
        let here = sdroxide_types::grid_to_latlon(self.cfg.my_grid.trim())?;
        Some(sdroxide_types::distance_km(here, self.cfg.timesig.station.lat_lon()))
    }

    /// The signal's flight time plus the receiver's latency, ns.
    fn delay_ns(&self) -> i64 {
        let flight = self.distance_km.map_or(0.0, |d| d / C_KM_PER_NS);
        (flight + self.cfg.timesig.latency_ms * 1e6) as i64
    }

    /// The median offset over the window and its scatter, once there are
    /// enough measurements.
    fn estimate(&self) -> Option<(i64, i64)> {
        if self.meas.len() < MIN_MEASUREMENTS {
            return None;
        }
        let delay = self.delay_ns();
        let mut v: Vec<i64> = self.meas.iter().map(|&(_, raw)| raw + delay).collect();
        v.sort_unstable();
        let median = v[v.len() / 2];
        let mut dev: Vec<i64> = v.iter().map(|x| (x - median).abs()).collect();
        dev.sort_unstable();
        // Median absolute deviation, scaled to a standard deviation.
        let jitter = (dev[dev.len() / 2] as f64 * 1.4826) as i64;
        Some((median, jitter))
    }

    fn restart(&mut self) {
        let station = self.cfg.timesig.station;
        self.dec = Decoder::new(station, self.rate);
        // The new decoder counts its samples from zero, and the sample clock
        // has to count from the same place or every arrival time is read for
        // the wrong sample.
        self.samples = 0;
        self.clock.reset();
        self.reported = None;
        self.tracker.reset();
        self.symbols.clear();
        self.second_of_minute = None;
        self.last_frame = None;
        self.frames_good = 0;
        self.frames_bad = 0;
        self.meas.clear();
        self.status_dirty = true;
    }

    fn handle(&mut self, e: Event) {
        match e {
            Event::Locked => {}
            Event::LockLost => {
                self.symbols.clear();
                self.second_of_minute = None;
            }
            Event::Second(sec) => {
                let code = sec.sym.map_or(TimeSigStatus::UNKNOWN, |s| s.code());
                match self.tracker.utc_at(sec.pos) {
                    Some(utc) => {
                        let som = utc.rem_euclid(60) as u8;
                        if som == 0 || self.second_of_minute.is_none() {
                            // A new minute, or the first second placed in one:
                            // the ribbon starts at the minute's own second 0.
                            self.symbols = vec![TimeSigStatus::UNKNOWN; usize::from(som)];
                        }
                        self.second_of_minute = Some(som);
                        if let Some(wall) = self.clock.wall_ns_at(sec.pos) {
                            let raw = i128::from(utc) * 1_000_000_000 - wall;
                            self.meas.push_back((wall, raw as i64));
                            while self
                                .meas
                                .front()
                                .is_some_and(|&(w, _)| w < wall - MEASURE_WINDOW_NS)
                            {
                                self.meas.pop_front();
                            }
                        }
                    }
                    None => {
                        self.second_of_minute = None;
                        if self.symbols.len() >= 60 {
                            self.symbols.remove(0);
                        }
                    }
                }
                self.symbols.push(code);
            }
            Event::Frame { pos, frame } => {
                self.tracker.frame(pos, frame.utc);
                self.frames_good += 1;
                self.last_frame = Some(frame);
            }
            Event::BadFrame { .. } => self.frames_bad += 1,
        }
        self.status_dirty = true;
    }

    fn timesig_status(&self) -> TimeSigStatus {
        let snap = self.dec.snapshot();
        let est = self.estimate();
        TimeSigStatus {
            station: self.cfg.timesig.station,
            carrier_hz: snap.carrier_hz,
            level_db: snap.level_db,
            snr_db: snap.snr_db,
            second_lock: snap.locked,
            second_of_minute: self.second_of_minute,
            symbols: self.symbols.clone(),
            scope: snap.scope,
            last_frame: self.last_frame.clone(),
            frames_good: self.frames_good,
            frames_bad: self.frames_bad,
            confirmed: if self.tracker.anchor().is_some() { self.tracker.run() } else { 0 },
            offset_ms: est.map(|(o, _)| o as f64 / 1e6),
            jitter_ms: est.map(|(_, j)| j as f64 / 1e6),
            propagation_ms: self.distance_km.map(|d| d / C_KM_PER_NS / 1e6),
            distance_km: self.distance_km,
            // The engine fills this in: the clock and the NTP server are the
            // host's, not this radio's.
            clock: Default::default(),
        }
    }

    fn digi_status(&self) -> DigiStatus {
        DigiStatus {
            mode: Mode::TimeSignal,
            step: QsoStep::Idle,
            dx_call: None,
            dx_grid: None,
            tx_next: false,
            tx_pending_msg: None,
            audio_hz: TIMESIG_TONE_HZ,
            tx_even: false,
            transmitting: false,
            tx_watchdog: false,
            tx_refused: None,
            timesig: Some(Box::new(self.timesig_status())),
            transcript: Vec::<TranscriptLine>::new(),
            config: self.cfg.clone(),
            text_rx: String::new(),
            tx_sent: 0,
            fsq_heard: Vec::new(),
            fsq_messages: Vec::new(),
            rade: None,
            packet: None,
            navtex: None,
            acars: None,
            aprs: None,
            js8: None,
            atchat: None,
            fox_queue: Vec::new(),
            call_queue: Vec::new(),
            clock_offset_s: None,
            cw: None,
            wspr: None,
            pi4: None,
            qso: None,
        }
    }
}

impl DigiEngine for TimeSigController {
    fn mode(&self) -> Mode {
        Mode::TimeSignal
    }

    /// Without a time of arrival there is nothing to measure the clock
    /// against but the moment of the call, which is what this falls back to.
    fn on_rx_audio(&mut self, tap: &[f32]) {
        self.on_rx_audio_at(tap, SystemTime::now());
    }

    fn on_rx_audio_at(&mut self, tap: &[f32], arrived: SystemTime) {
        self.samples += tap.len() as u64;
        let wall = match arrived.duration_since(SystemTime::UNIX_EPOCH) {
            Ok(d) => d.as_nanos() as i128,
            Err(e) => -(e.duration().as_nanos() as i128),
        };
        self.clock.observe(self.samples, wall);
        let mut events = std::mem::take(&mut self.events);
        events.clear();
        self.dec.process(tap, &mut events);
        for e in events.drain(..) {
            self.handle(e);
        }
        self.events = events;
    }

    fn poll(&mut self, now: SystemTime, _dial_hz: f64) -> Vec<DigiAction> {
        let mut out = Vec::new();
        let newest = self.meas.back().map(|&(w, _)| w);
        if let (Some((offset_ns, jitter_ns)), Some(w)) = (self.estimate(), newest)
            && self.tracker.anchor().is_some()
            && self.reported != Some(w)
        {
            self.reported = Some(w);
            out.push(DigiAction::TimeFix {
                offset_ns,
                jitter_ns,
                at: SystemTime::UNIX_EPOCH + Duration::from_nanos(w.max(0) as u64),
                refid: self.cfg.timesig.station.ntp_refid(),
            });
        }
        // On change, at most four times a second; and twice a second
        // regardless, for the level and the scope.
        let since = self.last_status.and_then(|t| now.duration_since(t).ok());
        let due = since.is_none_or(|d| d >= Duration::from_millis(500));
        let allowed = since.is_none_or(|d| d >= Duration::from_millis(250));
        if due || (self.status_dirty && allowed) {
            self.status_dirty = false;
            self.last_status = Some(now);
            out.push(DigiAction::Status(self.digi_status()));
        }
        out
    }

    fn tx_burst_active(&self) -> bool {
        false
    }

    fn fill_tx_block(&mut self, _out: &mut [f32]) -> bool {
        false
    }

    fn on_burst_done(&mut self) {}

    /// Starts the receiver over: the minute being read, the counts and the
    /// measurements — what CLEAR RX means here.
    fn abort(&mut self) {
        self.restart();
    }

    fn abort_tx(&mut self) {}

    fn set_audio_hz(&mut self, _hz: f32) {}

    /// Fixed: the carrier is put at the tone by tuning, and the decoder finds
    /// it within a few hundred hertz of there by itself.
    fn audio_hz(&self) -> f32 {
        TIMESIG_TONE_HZ
    }

    fn status(&self) -> DigiStatus {
        self.digi_status()
    }

    fn set_config(&mut self, cfg: DigiConfig) {
        let station_changed = cfg.timesig.station != self.cfg.timesig.station;
        self.tracker.set_need(cfg.timesig.min_confirmations());
        self.cfg = cfg;
        self.distance_km = self.distance();
        if station_changed {
            self.restart();
        }
        self.status_dirty = true;
    }

    fn clock_stepped(&mut self, _by_ns: i64) {
        // Every arrival time held was read off the clock as it was.
        self.clock.reset();
        self.meas.clear();
        self.reported = None;
        self.status_dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_timesig::synth::Synth;
    use sdroxide_types::{TimeStation, ymd_hms_to_unix};

    /// Feeds `secs` of a synthetic station in 20 ms blocks, each handed over
    /// with the arrival time a computer whose clock is `clock_error_s` *behind*
    /// true time would read — late by up to 15 ms of scheduling, except every
    /// fifth block, which arrives promptly.
    fn feed(c: &mut TimeSigController, station: TimeStation, secs: f64, clock_error_s: f64) {
        let rate = 12_000.0;
        let start = ymd_hms_to_unix(2026, 10, 18, 12, 36, 23) as f64 + 0.4;
        let mut s = Synth::new(station, rate);
        s.snr_db = Some(0.0);
        let audio = s.render(start, secs);
        let block = (rate / 50.0) as usize;
        let mut x: u64 = 99;
        for (i, chunk) in audio.chunks(block).enumerate() {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            let late = if i % 5 == 0 { 0.0 } else { (x % 15_000) as f64 * 1e-6 };
            let end_utc = start + ((i + 1) * block) as f64 / rate;
            let host = end_utc - clock_error_s + late;
            let at = SystemTime::UNIX_EPOCH + Duration::from_secs_f64(host);
            c.on_rx_audio_at(chunk, at);
        }
    }

    #[test]
    fn measures_how_far_the_clock_is_out() {
        for station in [TimeStation::Dcf77, TimeStation::Msf, TimeStation::Jjy60] {
            let cfg = DigiConfig {
                timesig: sdroxide_types::TimeSigConfig { station, ..Default::default() },
                ..DigiConfig::default()
            };
            let mut c = TimeSigController::new(cfg, 12_000.0);
            feed(&mut c, station, 200.0, 2.5);
            let st = c.timesig_status();
            assert!(st.confirmed >= 2, "{station:?}: {} frames confirmed", st.confirmed);
            let off = st.offset_ms.expect("an offset");
            assert!((off - 2500.0).abs() < 1.0, "{station:?}: measured {off} ms for 2500");
            assert!(st.jitter_ms.unwrap() < 2.0, "{station:?}: jitter {:?}", st.jitter_ms);
            let actions = c.poll(SystemTime::now(), 0.0);
            assert!(
                actions.iter().any(|a| matches!(a, DigiAction::TimeFix { offset_ns, .. }
                    if (*offset_ns - 2_500_000_000).abs() < 1_000_000)),
                "{station:?}: no fix reported"
            );
        }
    }

    #[test]
    fn the_flight_time_and_latency_are_added() {
        // JN61 is around Rome, some 950 km from Mainflingen: about 3 ms.
        let cfg = DigiConfig {
            my_grid: "JN61".into(),
            timesig: sdroxide_types::TimeSigConfig { latency_ms: 4.0, ..Default::default() },
            ..DigiConfig::default()
        };
        let mut c = TimeSigController::new(cfg, 12_000.0);
        feed(&mut c, TimeStation::Dcf77, 200.0, 0.0);
        let st = c.timesig_status();
        let flight = st.propagation_ms.expect("a flight time");
        assert!(flight > 2.0 && flight < 4.0, "flight {flight} ms");
        let off = st.offset_ms.expect("an offset");
        assert!((off - (flight + 4.0)).abs() < 1.0, "offset {off} for {flight} + 4");
    }

    #[test]
    fn nothing_is_reported_before_the_time_is_believed() {
        let mut c = TimeSigController::new(DigiConfig::default(), 12_000.0);
        // Under a minute: the second is found, but no frame has been read.
        feed(&mut c, TimeStation::Dcf77, 50.0, 1.0);
        let st = c.timesig_status();
        assert_eq!(st.offset_ms, None);
        assert!(
            !c.poll(SystemTime::now(), 0.0).iter().any(|a| matches!(a, DigiAction::TimeFix { .. }))
        );
    }

    #[test]
    fn a_clock_step_forgets_the_old_measurements() {
        let mut c = TimeSigController::new(DigiConfig::default(), 12_000.0);
        feed(&mut c, TimeStation::Dcf77, 200.0, 2.5);
        assert!(c.timesig_status().offset_ms.is_some());
        c.clock_stepped(2_500_000_000);
        assert_eq!(c.timesig_status().offset_ms, None);
    }

    #[test]
    fn clearing_the_receiver_keeps_samples_and_arrivals_in_step() {
        // CLEAR RX builds a new decoder, which counts from zero; the arrival
        // times have to start counting there too, or every measurement after
        // it is out by however long the receiver had been running.
        let mut c = TimeSigController::new(DigiConfig::default(), 12_000.0);
        feed(&mut c, TimeStation::Dcf77, 200.0, 2.5);
        c.abort();
        feed(&mut c, TimeStation::Dcf77, 200.0, 2.5);
        let off = c.timesig_status().offset_ms.expect("an offset after the restart");
        assert!((off - 2500.0).abs() < 1.0, "measured {off} ms for 2500 after CLEAR RX");
    }

    #[test]
    fn it_is_a_receive_only_engine() {
        let c = TimeSigController::new(DigiConfig::default(), 12_000.0);
        assert_eq!(c.mode(), Mode::TimeSignal);
        assert!(Mode::TimeSignal.is_rx_only());
        assert!(!c.tx_burst_active());
    }
}
