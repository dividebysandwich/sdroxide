//! The time-signal mode, from a mock front end to a decoded minute and a
//! measured clock — on long wave, and on short wave.
//!
//! The decoder crate tests the decoding on audio it makes itself. What only an
//! engine can show is the path: the dial put a kilohertz below the carrier,
//! the USB demodulator and its passband, the digital-mode tap ahead of the
//! AGC, the controller picked for the mode, and the arrival times the engine
//! hands it. A carrier that came out of that chain on the wrong side of the
//! dial, or a tap behind the AGC, would decode nothing here and everything in
//! the crate's own tests. On short wave there is one thing more: the passband
//! has to open up when the dial moves there, or the ticks a kilohertz above
//! the carrier never reach the decoder. And ALS162, whose time is all in its
//! carrier's phase, needs the chain to keep the phase the right way up — a
//! mirrored spectrum would turn every swing upside down.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use sdroxide_radio::{AudioParams, Complex32, EngineConfig, IqSource, Result, rtrb, start_engine};
use sdroxide_timesig::Format;
use sdroxide_timesig::format::als162;
use sdroxide_types::{
    Command, DeviceCaps, DigiConfig, Mode, RadioEvent, RxId, TIMESIG_HF_TOP_HZ, TimeSigConfig,
    TimeSigStatus, TimeStation, Vfo,
};

const RATE: f64 = 192_000.0;
/// DCF77's reduced carrier, against its full one.
const LOW: f32 = 0.15;

/// A long-wave front end hearing one station — DCF77 or ALS162 — and nothing
/// else.
///
/// The station's time runs from `utc0` at the first sample, or — when
/// `realtime` — from the computer's clock plus `ahead_s`, paced to it, so a
/// receiver measuring that clock should find it `ahead_s` slow.
struct Longwave {
    station: TimeStation,
    center: Arc<Mutex<f64>>,
    pos: u64,
    utc0: f64,
    realtime: Option<(Instant, f64)>,
    phase: f64,
    noise: u64,
}

impl Longwave {
    fn amplitude(t: f64) -> f32 {
        let fmt = Format::for_station(TimeStation::Dcf77);
        let sec = t.floor() as i64;
        let minute = sec.div_euclid(60) * 60;
        let sym = (fmt.encode)(minute)[(sec - minute) as usize];
        let slot = (((t - sec as f64) * 10.0) as usize).min(9);
        if fmt.template(sym).is_some_and(|tpl| tpl.low[slot]) { LOW } else { 1.0 }
    }

    /// ALS162's phase: one swing for a 0, two for a 1, centred on the second.
    fn phase(t: f64) -> f64 {
        let fmt = Format::for_station(TimeStation::Als162);
        // Each swing starts 50 ms before its second.
        let sec = (t + 0.05).floor() as i64;
        let minute = sec.div_euclid(60) * 60;
        let sym = (fmt.encode)(minute)[(sec - minute) as usize];
        let into = t - sec as f64;
        (0..als162::elements(sym))
            .map(|k| als162::element(into - k as f64 * als162::ELEMENT_S))
            .sum()
    }

    fn rand(&mut self) -> f32 {
        self.noise ^= self.noise << 13;
        self.noise ^= self.noise >> 7;
        self.noise ^= self.noise << 17;
        (self.noise >> 40) as f32 / (1u64 << 24) as f32 - 0.5
    }
}

impl IqSource for Longwave {
    fn sample_rate(&self) -> f64 {
        RATE
    }
    fn center_hz(&self) -> f64 {
        *self.center.lock().unwrap()
    }
    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        *self.center.lock().unwrap() = hz;
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        let n = buf.len().min(4096);
        if let Some((start, _)) = self.realtime {
            // Hand samples over no sooner than a real receiver would have them.
            let due = start + Duration::from_secs_f64((self.pos + n as u64) as f64 / RATE);
            // An engine that cannot keep up takes the samples later and
            // later, which is a clock running fast as far as anything
            // measuring it can tell. Said every ten seconds, so a failure
            // here can be told apart from a slow build.
            if self.pos / (RATE as u64 * 10) != (self.pos + n as u64) / (RATE as u64 * 10) {
                let late = Instant::now().saturating_duration_since(due).as_secs_f64();
                eprintln!("{late:.3} s behind real time at {:.0} s", self.pos as f64 / RATE);
            }
            if let Some(wait) = due.checked_duration_since(Instant::now()) {
                std::thread::sleep(wait);
            }
        }
        let step = std::f64::consts::TAU * (self.station.carrier_hz() - self.center_hz()) / RATE;
        for z in buf[..n].iter_mut() {
            let t = self.utc0 + self.pos as f64 / RATE;
            let (a, phase) = match self.station {
                TimeStation::Als162 => (0.05, self.phase + Self::phase(t)),
                _ => (0.05 * Self::amplitude(t), self.phase),
            };
            let (s, c) = phase.sin_cos();
            *z = Complex32::new(
                a * c as f32 + 0.002 * self.rand(),
                a * s as f32 + 0.002 * self.rand(),
            );
            self.phase = (self.phase + step) % std::f64::consts::TAU;
            self.pos += 1;
        }
        Ok(n)
    }
    fn describe(&self) -> String {
        format!("{} on a long-wave antenna", self.station.label())
    }
}

/// A short-wave front end hearing WWV on 10 MHz: an AM broadcast of its
/// carrier, its 100 Hz time code and its ticks.
struct Shortwave {
    center: Arc<Mutex<f64>>,
    pos: u64,
    utc0: f64,
    phase: f64,
    noise: u64,
}

impl Shortwave {
    /// WWV's modulation at `t`, as a fraction of the carrier.
    fn modulation(t: f64) -> f64 {
        let fmt = Format::for_station(TimeStation::Wwv);
        let sec = t.floor() as i64;
        let minute = sec.div_euclid(60) * 60;
        let s_of_m = (sec - minute) as usize;
        let sym = (fmt.encode)(minute)[s_of_m];
        let into = t - sec as f64;
        let tau = std::f64::consts::TAU;
        let mut m = 0.0;
        if into >= 0.03 && !sym.hole {
            let slot = ((into * 10.0) as usize).min(9);
            let high = fmt.template(sym).is_some_and(|tpl| !tpl.low[slot]);
            m += if high { 0.178 } else { 0.0316 } * (tau * 100.0 * into).sin();
        }
        let tick = if s_of_m == 0 { 0.8 } else { 0.005 };
        if into < tick && s_of_m != 29 && s_of_m != 59 {
            m += (tau * 1000.0 * into).sin();
        }
        m
    }
}

impl IqSource for Shortwave {
    fn sample_rate(&self) -> f64 {
        RATE
    }
    fn center_hz(&self) -> f64 {
        *self.center.lock().unwrap()
    }
    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        *self.center.lock().unwrap() = hz;
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        let n = buf.len().min(4096);
        let step =
            std::f64::consts::TAU * (TimeStation::Wwv.carrier_hz() - self.center_hz()) / RATE;
        let mut r = Noise(self.noise);
        for z in buf[..n].iter_mut() {
            let t = self.utc0 + self.pos as f64 / RATE;
            // Double sideband with its carrier: what any AM station is.
            let a = (0.03 * (1.0 + Self::modulation(t))) as f32;
            let (s, c) = self.phase.sin_cos();
            *z = Complex32::new(a * c as f32 + 0.001 * r.next(), a * s as f32 + 0.001 * r.next());
            self.phase = (self.phase + step) % std::f64::consts::TAU;
            self.pos += 1;
        }
        self.noise = r.0;
        Ok(n)
    }
    fn describe(&self) -> String {
        "WWV on 10 MHz".into()
    }
}

/// xorshift, for a little noise that is the same every run.
struct Noise(u64);

impl Noise {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32 - 0.5
    }
}

fn isolate_config(name: &str) {
    let root = std::env::temp_dir().join(format!("sdroxide-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    // An engine test that saves anything writes the operator's real config
    // directory unless this is set.
    unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &root) };
}

fn engine(src: impl IqSource + 'static) -> sdroxide_radio::EngineHandles {
    let (producer, _consumer) = rtrb::RingBuffer::<f32>::new(48_000);
    start_engine(
        Box::new(src),
        DeviceCaps {
            driver: "mock".into(),
            label: "mock".into(),
            rx_channels: 1,
            sample_rates: vec![RATE],
            freq_ranges_rx: vec![(0.0, 30_000_000.0)],
            ..DeviceCaps::default()
        },
        EngineConfig {
            remember_session: false,
            audio: Some(AudioParams { producer, out_rate: 48_000.0 }),
            ..Default::default()
        },
    )
}

/// Wait for a time-signal status that satisfies `f`.
fn status(
    h: &sdroxide_radio::EngineHandles,
    within: Duration,
    what: &str,
    mut f: impl FnMut(&TimeSigStatus) -> bool,
) -> TimeSigStatus {
    let deadline = Instant::now() + within;
    let mut last: Option<TimeSigStatus> = None;
    while Instant::now() < deadline {
        while let Ok(ev) = h.event_rx.try_recv() {
            if let RadioEvent::Ft8Status(s) = ev
                && let Some(t) = s.timesig
            {
                if f(&t) {
                    return *t;
                }
                last = Some(*t);
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let brief = last.map(|s| {
        format!(
            "carrier {:?} lock {} frames {}/{} confirmed {} offset {:?} symbols {:?}",
            s.carrier_hz,
            s.second_lock,
            s.frames_good,
            s.frames_bad,
            s.confirmed,
            s.offset_ms,
            s.symbols
        )
    });
    panic!("the time-signal receiver never reported {what}; last status: {brief:?}");
}

#[test]
fn selecting_the_mode_decodes_the_minute() {
    isolate_config("timesig-window");
    let utc0 = sdroxide_types::ymd_hms_to_unix(2026, 10, 18, 12, 36, 23) as f64 + 0.4;
    let center = Arc::new(Mutex::new(TimeStation::Dcf77.dial_hz()));
    let mut h = engine(Longwave {
        station: TimeStation::Dcf77,
        center,
        pos: 0,
        utc0,
        realtime: None,
        phase: 0.0,
        noise: 7,
    });
    let _thread = h.thread.take();
    let send = |c: Command| h.cmd_tx.send(c).unwrap();
    send(Command::SetVfo { vfo: Vfo::A, hz: TimeStation::Dcf77.dial_hz() });
    send(Command::SetMode { rx: RxId::Main, mode: Mode::TimeSignal });

    let st = status(&h, Duration::from_secs(240), "two decoded minutes", |s| s.frames_good >= 2);
    let carrier = st.carrier_hz.expect("the carrier was found");
    assert!(
        (carrier - 1000.0).abs() < 5.0,
        "a dial 1 kHz below DCF77 puts it at 1 kHz in USB, not {carrier} Hz"
    );
    assert!(st.second_lock);
    let f = st.last_frame.expect("a frame");
    // The frames name the minutes after 12:36:23.4 UTC.
    assert!(
        f.utc >= utc0 as i64 && f.utc <= utc0 as i64 + 240,
        "decoded {} for a signal that started at {utc0}",
        f.utc
    );
    assert_eq!(f.utc % 60, 0);
    assert_eq!(f.zone_min, 120, "CEST in October");
    assert_eq!(st.station, TimeStation::Dcf77);
}

#[test]
fn als162_is_read_from_its_carrier_phase() {
    isolate_config("timesig-als162");
    let utc0 = sdroxide_types::ymd_hms_to_unix(2026, 10, 18, 12, 36, 23) as f64 + 0.4;
    let center = Arc::new(Mutex::new(TimeStation::Als162.dial_hz()));
    let mut h = engine(Longwave {
        station: TimeStation::Als162,
        center,
        pos: 0,
        utc0,
        realtime: None,
        phase: 0.0,
        noise: 5,
    });
    let _thread = h.thread.take();
    let send = |c: Command| h.cmd_tx.send(c).unwrap();
    send(Command::SetVfo { vfo: Vfo::A, hz: TimeStation::Als162.dial_hz() });
    send(Command::SetMode { rx: RxId::Main, mode: Mode::TimeSignal });
    send(Command::SetDigiConfig(DigiConfig {
        timesig: TimeSigConfig { station: TimeStation::Als162, ..Default::default() },
        ..Default::default()
    }));

    let st = status(&h, Duration::from_secs(240), "two decoded minutes", |s| {
        s.station == TimeStation::Als162 && s.frames_good >= 2
    });
    let carrier = st.carrier_hz.expect("the carrier was found");
    assert!((carrier - 1000.0).abs() < 5.0, "the carrier at 1 kHz in USB, not {carrier} Hz");
    assert!(st.second_lock);
    let f = st.last_frame.expect("a frame");
    assert!(f.utc >= utc0 as i64 && f.utc <= utc0 as i64 + 240, "decoded {}", f.utc);
    assert_eq!(f.utc % 60, 0);
    assert_eq!(f.zone_min, 120, "CEST in October");
}

#[test]
fn short_wave_opens_the_passband_and_decodes_the_minute() {
    isolate_config("timesig-shortwave");
    let utc0 = sdroxide_types::ymd_hms_to_unix(2026, 10, 18, 12, 36, 23) as f64 + 0.4;
    let center = Arc::new(Mutex::new(TimeStation::Dcf77.dial_hz()));
    let mut h = engine(Shortwave { center, pos: 0, utc0, phase: 0.0, noise: 3 });
    let _thread = h.thread.take();
    let send = |c: Command| h.cmd_tx.send(c).unwrap();
    // Into the mode on long wave, as an operator coming from DCF77 would be,
    // then to WWV on 10 MHz.
    send(Command::SetVfo { vfo: Vfo::A, hz: TimeStation::Dcf77.dial_hz() });
    send(Command::SetMode { rx: RxId::Main, mode: Mode::TimeSignal });
    send(Command::SetDigiConfig(DigiConfig {
        timesig: TimeSigConfig { station: TimeStation::Wwv, ..Default::default() },
        ..Default::default()
    }));
    send(Command::SetVfo { vfo: Vfo::A, hz: TimeStation::Wwv.dial_hz() });

    let deadline = Instant::now() + Duration::from_secs(30);
    let mut widened = None;
    while widened.is_none() && Instant::now() < deadline {
        while let Ok(ev) = h.event_rx.try_recv() {
            if let RadioEvent::State(st) = ev
                && (st.rx_freq_hz() - TimeStation::Wwv.dial_hz()).abs() < 1.0
                && st.rx[0].mode == Mode::TimeSignal
            {
                widened = Some((st.rx[0].filter_lo, st.rx[0].filter_hi));
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let (lo, hi) = widened.expect("a state on the new dial");
    assert_eq!(hi, TIMESIG_HF_TOP_HZ, "the passband opens on short wave: {lo}..{hi}");

    let st = status(&h, Duration::from_secs(300), "two decoded minutes", |s| {
        s.station == TimeStation::Wwv && s.frames_good >= 2
    });
    let carrier = st.carrier_hz.expect("the carrier was found");
    assert!((carrier - 1000.0).abs() < 5.0, "the carrier at 1 kHz in USB, not {carrier} Hz");
    assert!(st.ticks_heard, "WWV's own ticks timed the seconds");
    assert_eq!(st.sibling_heard, None);
    let f = st.last_frame.expect("a frame");
    assert!(f.utc >= utc0 as i64 && f.utc <= utc0 as i64 + 300, "decoded {}", f.utc);
}

/// The clock measurement through the engine: the station's time runs 2.5 s
/// ahead of this computer's, so the computer's clock must read 2.5 s slow —
/// plus whatever the receive chain's filters delay the signal by, which is
/// what the latency calibration exists for.
///
/// Real time, so three minutes long for each station, and in release: an
/// unoptimised engine at this sample rate runs slower than real time, falls
/// behind the source, and the lag reads as a clock running fast. Both
/// long-wave keyings are measured, because the latency is calibrated once per
/// radio: a station read by its phase has to come out of the chain as late as
/// one read by its amplitude. `cargo test --release -p sdroxide-radio
/// --no-default-features --test timesig_window -- --ignored --nocapture`
#[test]
#[ignore]
fn the_computer_clock_is_measured_against_the_station() {
    isolate_config("timesig-clock");
    let ahead = 2.5;
    for station in [TimeStation::Dcf77, TimeStation::Als162] {
        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs_f64();
        let center = Arc::new(Mutex::new(station.dial_hz()));
        let mut h = engine(Longwave {
            station,
            center,
            pos: 0,
            utc0: now + ahead,
            realtime: Some((Instant::now(), now)),
            phase: 0.0,
            noise: 9,
        });
        let _thread = h.thread.take();
        let send = |c: Command| h.cmd_tx.send(c).unwrap();
        send(Command::SetVfo { vfo: Vfo::A, hz: station.dial_hz() });
        send(Command::SetMode { rx: RxId::Main, mode: Mode::TimeSignal });
        send(Command::SetDigiConfig(DigiConfig {
            timesig: TimeSigConfig { station, ..Default::default() },
            ..Default::default()
        }));

        let st = status(&h, Duration::from_secs(240), "an offset", |s| {
            s.station == station && s.offset_ms.is_some()
        });
        let off = st.offset_ms.unwrap();
        println!(
            "{}: measured {off:.2} ms ± {:.2} for {:.0} ms ahead: the chain's own delay is \
             {:.2} ms",
            station.label(),
            st.jitter_ms.unwrap_or(0.0),
            ahead * 1000.0,
            off - ahead * 1000.0
        );
        assert!(off > 0.0, "a clock behind the station reads slow (positive), not {off}");
        assert!((off - ahead * 1000.0).abs() < 50.0, "measured {off} ms for {} ms", ahead * 1000.0);
    }
}
