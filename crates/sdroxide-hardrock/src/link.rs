//! The amplifier on a thread of its own: open the port, poll the readings, carry
//! out the panel's buttons, and keep a snapshot the engine can read without
//! waiting on the serial port.
//!
//! Every round trip to the amplifier blocks for tens of milliseconds and the
//! engine's loop cannot spare that, which is the reason `sdroxide-relay` and
//! `sdroxide-rotator` run their hardware on threads too.

use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, bounded};
use sdroxide_types::{
    AmpAction, AmpAlerts, AmpConfig, AmpGauge, AmpLevel, AmpLimits, AmpReadings, AmpStatus,
    AtuNetwork, AtuReadout,
};
use tracing::{debug, info, warn};

use crate::{Amp, AtuParams, Band, Command, KeyMode, Lights, Response, TempScale, TuneResult};

/// How often the readings are asked for. Four a second is what HobbyPCB's own
/// panel manages, and what a meter needs to look alive.
const POLL: Duration = Duration::from_millis(250);
/// How often the tuner's network is read while nothing is happening. During a
/// tune it is read every poll.
const ATU_POLL: Duration = Duration::from_secs(1);
/// How long after a click the tuner is read back: long enough for its relays
/// to settle, so the read confirms the change instead of showing the old state.
const ATU_SETTLE: Duration = Duration::from_millis(300);
/// How often the temperature is read with `HRTP`. Its value is the one the
/// amplifier's own screen shows, and it carries the scale letter; the status
/// frame's temperature lagged it by several degrees after a C/F toggle (AJ3G,
/// 17 Oct 2026: the screen read 75 °F at once while the status frame crept up
/// from 71), so `HRTP` is what the panel shows.
const TEMP_POLL: Duration = Duration::from_secs(1);
/// The same while transmitting, when the amplifier often leaves a query
/// unanswered: each miss holds up the live readings for the query's timeout,
/// and the heat spreader changes far more slowly than they do.
const TEMP_POLL_TX: Duration = Duration::from_secs(10);
/// How old an `HRTP` reading may be before the status frame's own
/// temperature is shown instead. AJ3G, 18 Oct 2026: the panel held 122 °F
/// while the amplifier's screen said 133 °F, because a reading that stops
/// coming was kept for good. A little over the transmitting interval, so a
/// slow read while keyed is still used.
const TEMP_STALE: Duration = Duration::from_secs(15);
/// How long after a C/F toggle the scale is read on every poll.
const SCALE_SETTLE: Duration = Duration::from_secs(3);
/// Missed answers in a row before the link is called lost and reopened.
const MISSES: u32 = 4;
/// Longest wait between tries at reopening the port. Trying is cheap and
/// disturbs nothing, and a USB lead plugged back in should be picked up within
/// seconds (AJ3G, 18 Oct: at 30 s it took minutes).
const RETRY_MAX: Duration = Duration::from_secs(2);
/// Pings tried after opening before giving up: a USB serial adapter just
/// plugged in can drop the first bytes.
const OPEN_PINGS: u32 = 3;
/// Drive above this, with the amplifier in line, counts as a transmission for
/// holding and clearing the status indicators. `HRST` says nothing of TX.
const KEYED_DRIVE_W: f32 = 0.5;

/// Watches for the amplifier going off line in the middle of a transmission,
/// which is what a trip is. `HRST` has no trip flag, so it is read from the
/// power: in line the amplifier has gain, off line its output is no more than
/// the drive passed through. A real HR500 trip (160 m overdrive, Oct 2026)
/// read 0 W out on 5.6 W in, with VDD sagging from 57 to 42 V as the
/// protection cut the drain supply.
#[derive(Default)]
struct TripWatch {
    keyed_polls: u32,
    dead_polls: u32,
    /// The last reading of this transmission with the amplifier in line.
    last_good: Option<AmpReadings>,
}

impl TripWatch {
    /// Polls of a key-up let pass before judging, while relays settle.
    const SETTLE: u32 = 2;
    /// Polls in a row with no gain before calling it a trip.
    const CONFIRM: u32 = 2;

    fn reset(&mut self) {
        *self = TripWatch::default();
    }

    /// One reading taken while keyed in PTT. `true` once the amplifier has
    /// been seen off line.
    fn keyed(&mut self, l: &AmpLimits, r: &AmpReadings) -> bool {
        self.keyed_polls += 1;
        let no_gain = r.drive_w > 1.0 && r.fwd_w < r.drive_w * 3.0;
        if !no_gain {
            self.last_good = Some(*r);
        }
        // The drain supply collapsing under drive is the protection itself
        // acting, so it needs no confirming.
        let sagged = r.vdd_v > 0.0 && r.vdd_v < l.vdd_low_v;
        if no_gain && sagged {
            return true;
        }
        self.dead_polls =
            if no_gain && self.keyed_polls > Self::SETTLE { self.dead_polls + 1 } else { 0 };
        self.dead_polls >= Self::CONFIRM
    }

    /// Mark the transmission tripped, blaming the likeliest reading when none
    /// was caught past its trip level.
    fn trip(&self, alerts: &mut AmpAlerts, l: &AmpLimits, now: &AmpReadings) {
        alerts.tripped = true;
        if let (true, Some(g)) = (alerts.trips().is_empty(), self.cause(l, now)) {
            alerts.blame(g);
        }
    }

    /// The likeliest cause of a trip when no reading was caught past its
    /// trip level; the amplifier tends not to answer in the moment before
    /// it trips. Drive that jumped since the last good reading is the drive;
    /// otherwise whichever reading stood nearest its trip level.
    fn cause(&self, l: &AmpLimits, now: &AmpReadings) -> Option<AmpGauge> {
        let g = self.last_good?;
        if now.drive_w > g.drive_w * 1.5 {
            return Some(AmpGauge::Drive);
        }
        [
            (AmpGauge::Fwd, g.fwd_w / l.fwd_trip_w),
            (AmpGauge::Rfl, g.rfl_w / l.rfl_trip_w),
            (AmpGauge::Drive, g.drive_w / l.drive_trip_w),
            (AmpGauge::Idd, g.idd_a / l.idd_trip_a),
        ]
        .into_iter()
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(g, _)| g)
    }
}

/// The amplifier's own lights as the panel's alerts, given whether a
/// transmission has been seen since the link opened.
///
/// Until this link has seen a transmission, yellows show green. A red is a
/// trip and is shown whenever it happened.
///
/// Why: the HR500 holds a yellow until the next key-up, and can start up with
/// DRV yellow with nothing wrong (a firmware quirk). From remote the operator
/// can't see the amplifier, so a yellow there reads as a fault when there is
/// none, and a warning that is often false teaches people to ignore it
/// (AJ3G, 18 Oct 2026). Decision 1 in docs/hardrock/design-decisions.md.
fn lights_seen(l: Lights, seen_tx: bool) -> AmpAlerts {
    let a = lights_alerts(l);
    if seen_tx {
        return a;
    }
    let calm = |v: AmpLevel| if v == AmpLevel::Yellow { AmpLevel::Green } else { v };
    AmpAlerts {
        fwd: calm(a.fwd),
        rfl: calm(a.rfl),
        drive: calm(a.drive),
        vdd: calm(a.vdd),
        idd: calm(a.idd),
        ..a
    }
}

/// The amplifier's own lights as the panel's alerts. Red on any of them is
/// a trip.
fn lights_alerts(l: Lights) -> AmpAlerts {
    let level = |d: u8| match d {
        3 => AmpLevel::Red,
        2 => AmpLevel::Yellow,
        _ => AmpLevel::Green,
    };
    let a = AmpAlerts {
        fwd: level(l.fwd),
        rfl: level(l.rfl),
        drive: level(l.drv),
        vdd: level(l.vdd),
        idd: level(l.idd),
        tripped: false,
        from_amp: true,
    };
    AmpAlerts { tripped: [a.fwd, a.rfl, a.drive, a.vdd, a.idd].contains(&AmpLevel::Red), ..a }
}

enum Msg {
    Action(AmpAction),
    Frequency(u64),
}

/// A running link to the amplifier. Dropping it closes the port.
pub struct AmpLink {
    tx: Option<Sender<Msg>>,
    status: Arc<Mutex<AmpStatus>>,
    thread: Option<JoinHandle<()>>,
}

impl AmpLink {
    pub fn start(cfg: AmpConfig) -> AmpLink {
        let (tx, rx) = bounded(16);
        let status = Arc::new(Mutex::new(AmpStatus {
            configured: true,
            model: cfg.model,
            ..AmpStatus::default()
        }));
        let st = status.clone();
        let thread =
            thread::Builder::new().name("hardrock".into()).spawn(move || worker(cfg, rx, st)).ok();
        AmpLink { tx: Some(tx), status, thread }
    }

    /// The latest snapshot.
    pub fn status(&self) -> AmpStatus {
        self.status.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn action(&self, a: AmpAction) {
        if let Some(tx) = &self.tx {
            let _ = tx.try_send(Msg::Action(a));
        }
    }

    /// Tell the amplifier the radio's frequency; it picks its band from it.
    pub fn set_frequency(&self, hz: u64) {
        if let Some(tx) = &self.tx {
            let _ = tx.try_send(Msg::Frequency(hz));
        }
    }
}

impl Drop for AmpLink {
    fn drop(&mut self) {
        // Closing the channel is the worker's signal to stop.
        self.tx = None;
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn set(status: &Mutex<AmpStatus>, f: impl FnOnce(&mut AmpStatus)) {
    f(&mut status.lock().unwrap_or_else(|e| e.into_inner()));
}

fn open(cfg: &AmpConfig) -> crate::Result<Amp<Box<dyn serialport::SerialPort>>> {
    let port = serialport::new(&cfg.serial.path, cfg.serial.baud)
        .data_bits(serialport::DataBits::Eight)
        .parity(serialport::Parity::None)
        .stop_bits(serialport::StopBits::One)
        .flow_control(serialport::FlowControl::None)
        .timeout(Duration::from_millis(50))
        .open()?;
    let _ = port.clear(serialport::ClearBuffer::All);
    let mut amp = Amp::new(port).with_timeout(Duration::from_millis(400));
    let mut last = None;
    for _ in 0..OPEN_PINGS {
        match amp.query(&Command::Ping) {
            Ok(_) => return Ok(amp),
            Err(e) => last = Some(e),
        }
    }
    Err(last.expect("OPEN_PINGS > 0"))
}

/// What the operator is told when the port won't open or the amp won't answer.
fn open_error(path: &str, e: &crate::Error) -> String {
    match e {
        crate::Error::Serial(se) if se.kind() == serialport::ErrorKind::NoDevice => {
            format!("{path} not found. Check the amplifier's USB lead.")
        }
        crate::Error::Serial(se)
            if matches!(se.kind(), serialport::ErrorKind::Io(std::io::ErrorKind::NotFound)) =>
        {
            format!("{path} not found. Check the amplifier's USB lead.")
        }
        crate::Error::Timeout(_) => {
            format!(
                "{path} is open but the amplifier isn't answering. Is it on, at the set baud rate?"
            )
        }
        _ => format!("{path}: {e}"),
    }
}

fn worker(cfg: AmpConfig, rx: Receiver<Msg>, status: Arc<Mutex<AmpStatus>>) {
    if cfg.serial.path.trim().is_empty() {
        set(&status, |s| s.error = Some("No serial port chosen".into()));
        // Nothing to do but wait to be dropped.
        while rx.recv().is_ok() {}
        return;
    }
    let mut backoff = Duration::from_secs(1);
    loop {
        match open(&cfg) {
            Ok(amp) => {
                info!(port = %cfg.serial.path, baud = cfg.serial.baud, "hardrock: connected");
                backoff = Duration::from_secs(1);
                set(&status, |s| {
                    s.connected = true;
                    s.error = None;
                });
                if !run(amp, &rx, &status, cfg.model.limits()) {
                    return;
                }
                set(&status, |s| {
                    s.connected = false;
                    s.readings = None;
                    s.atu = None;
                    s.error = Some("The amplifier stopped answering".into());
                });
            }
            Err(e) => {
                debug!("hardrock: {e}");
                set(&status, |s| {
                    s.connected = false;
                    s.error = Some(open_error(&cfg.serial.path, &e));
                });
            }
        }
        // Wait out the backoff, still draining the channel so a drop is seen.
        let until = Instant::now() + backoff;
        while let Some(left) = until.checked_duration_since(Instant::now()) {
            match rx.recv_timeout(left) {
                Err(RecvTimeoutError::Disconnected) => return,
                Err(RecvTimeoutError::Timeout) => break,
                Ok(_) => {}
            }
        }
        backoff = (backoff * 2).min(RETRY_MAX);
    }
}

/// Talk to an open amplifier until it stops answering (`true`) or the link is
/// dropped (`false`).
fn run<P: std::io::Read + std::io::Write>(
    mut amp: Amp<P>,
    rx: &Receiver<Msg>,
    status: &Mutex<AmpStatus>,
    limits: AmpLimits,
) -> bool {
    let has_atu = matches!(amp.query(&Command::AtuPresent), Ok(Response::AtuPresent(true)));
    set(status, |s| s.atu = has_atu.then(AtuReadout::default));
    let mut celsius: Option<bool> = None;
    let mut misses = 0;
    let mut next_atu = Instant::now();
    let mut next_temp = Instant::now();
    // A C/F toggle not yet seen on the amplifier, and how long to keep looking.
    let mut scale_wanted: Option<(bool, Instant)> = None;
    // A band change waiting for the amplifier to be off the air: it switches
    // the filter relays, which must not move under power (as in `act`).
    let mut freq_wanted: Option<u64> = None;
    // The temperature as `HRTP` last gave it, in the scale it gave, and when.
    let mut screen_temp: Option<(f32, Instant)> = None;
    let mut was_tuning = false;
    let mut alerts = AmpAlerts::default();
    let mut was_keyed = false;
    // A transmission seen since this link opened (see `lights_seen`).
    let mut seen_tx = false;
    let mut watch = TripWatch::default();
    // When the operator last switched PTT/OFF, so the amplifier dropping to
    // OFF by itself mid-transmission can be told from a click.
    let mut ptt_set_at: Option<Instant> = None;
    loop {
        match rx.recv_timeout(POLL) {
            Err(RecvTimeoutError::Disconnected) => return false,
            Ok(Msg::Action(a)) => {
                if matches!(a, AmpAction::SetPtt(_)) {
                    ptt_set_at = Some(Instant::now());
                }
                match act(&mut amp, a, status, &limits) {
                    Err(e) => warn!("hardrock: {a:?}: {e}"),
                    // The drawing follows the click at once rather than
                    // waiting for the relays: the amplifier answers a read
                    // straight after with the old state.
                    Ok(true) => {
                        if let AmpAction::SetAtu(on) = a {
                            set(status, |st| {
                                let atu = st.atu.get_or_insert_with(AtuReadout::default);
                                atu.active = on;
                                if let Some(n) = atu.network.as_mut() {
                                    n.bypassed = !on;
                                }
                            });
                        }
                    }
                    Ok(false) => {}
                }
                // Read back once the relays have settled, to confirm the click.
                next_atu = Instant::now() + ATU_SETTLE;
                // After a C/F toggle the scale is read on every poll until
                // the amplifier has switched: the status reading arrives in
                // the new unit, and a letter left over from before put the
                // wrong one beside it until the next ten-second read (AJ3G).
                if let AmpAction::SetCelsius(c) = a {
                    scale_wanted = Some((c, Instant::now() + SCALE_SETTLE));
                }
            }
            Ok(Msg::Frequency(hz)) => freq_wanted = Some(hz),
            Err(RecvTimeoutError::Timeout) => {}
        }
        if let Some(hz) = freq_wanted
            && let Ok(s) = amp.status()
            && !(s.lights().is_some_and(|l| l.tx) || s.forward_w >= 1.0 || s.drive_w >= 0.1)
        {
            let _ = amp.set_frequency(hz);
            freq_wanted = None;
        }

        let settling = scale_wanted.filter(|&(_, until)| Instant::now() < until);
        if settling.is_some() || Instant::now() >= next_temp {
            next_temp = Instant::now() + if was_keyed { TEMP_POLL_TX } else { TEMP_POLL };
            match amp.query(&Command::GetTemperature) {
                Ok(Response::Temperature(v, Some(scale))) => {
                    celsius = Some(scale == TempScale::C);
                    screen_temp = Some((v, Instant::now()));
                }
                // No scale letter: the scale last seen still holds.
                Ok(Response::Temperature(v, None)) if celsius.is_some() => {
                    screen_temp = Some((v, Instant::now()));
                }
                other => debug!("HRTP not read: {other:?}"),
            }
        }
        scale_wanted = settling.filter(|&(want, _)| celsius != Some(want));

        match amp.status() {
            Ok(s) => {
                misses = 0;
                let r = AmpReadings {
                    fwd_w: s.forward_w,
                    rfl_w: s.reflected_w,
                    drive_w: s.drive_w,
                    swr: s.swr,
                    vdd_v: s.vdd_v,
                    idd_a: s.idd_a,
                    temperature: shown_temp(
                        fresh_temp(screen_temp, Instant::now()),
                        s.temperature,
                        s.mode == KeyMode::Ptt || was_keyed,
                    ),
                    celsius,
                    ptt: s.mode == KeyMode::Ptt,
                    band: s.band.map(Band::code),
                    antenna: s.antenna,
                };
                if let Some(l) = s.lights() {
                    // The amplifier's own lights: shown as sent.
                    seen_tx |= l.tx;
                    alerts = lights_seen(l, seen_tx);
                    was_keyed = l.tx;
                } else {
                    // Older firmware sends no lights, so they are worked out
                    // as on the amplifier's screen: a key-up clears the
                    // indicators, and the worst of the transmission stays lit.
                    // A reading past a trip level is only shown as a trip once
                    // the amplifier is seen to go off line.
                    let keyed = r.ptt && r.drive_w > KEYED_DRIVE_W;
                    if keyed && !was_keyed {
                        alerts = AmpAlerts::default();
                        watch.reset();
                    }
                    if keyed {
                        alerts.hold(&limits, &r);
                        if watch.keyed(&limits, &r) && !alerts.tripped {
                            watch.trip(&mut alerts, &limits, &r);
                        }
                    }
                    let clicked_off =
                        ptt_set_at.is_some_and(|t| t.elapsed() < Duration::from_secs(2));
                    if was_keyed
                        && !r.ptt
                        && r.drive_w > KEYED_DRIVE_W
                        && !clicked_off
                        && !alerts.tripped
                    {
                        // Still driven, but the amplifier switched itself to OFF.
                        watch.trip(&mut alerts, &limits, &r);
                    }
                    was_keyed = keyed;
                }
                set(status, |st| {
                    st.readings = Some(r);
                    st.alerts = alerts;
                });
            }
            Err(e) => {
                misses += 1;
                debug!("hardrock: status: {e}");
                if misses >= MISSES {
                    return true;
                }
            }
        }

        if has_atu && (was_tuning || Instant::now() >= next_atu) {
            next_atu = Instant::now() + ATU_POLL;
            let tuning = matches!(amp.query(&Command::IsTuning), Ok(Response::Tuning(true)));
            let active = match amp.query(&Command::GetAtuActive) {
                Ok(Response::AtuActive(a)) => Some(a),
                _ => None,
            };
            let network = match amp.query(&Command::AtuParams) {
                Ok(Response::AtuParams(p)) => Some(network_of(p)),
                _ => None,
            };
            let result = if was_tuning && !tuning {
                match amp.query(&Command::LastTuneResult) {
                    Ok(Response::TuneResult(r)) => Some(result_text(r)),
                    _ => None,
                }
            } else {
                None
            };
            was_tuning = tuning;
            set(status, |st| {
                let atu = st.atu.get_or_insert_with(AtuReadout::default);
                atu.tuning = tuning;
                if let Some(a) = active {
                    atu.active = a;
                }
                if network.is_some() {
                    atu.network = network;
                }
                if result.is_some() {
                    atu.last_result = result;
                }
            });
        }
    }
}

fn network_of(p: AtuParams) -> AtuNetwork {
    AtuNetwork {
        l_step: p.l_step,
        c_step: p.c_step,
        bypassed: p.bypassed,
        cap_output: p.cap_output,
        swr: p.swr(),
    }
}

fn result_text(r: TuneResult) -> String {
    match r {
        TuneResult::Tuned => "Tuned".into(),
        TuneResult::Aborted => "Aborted".into(),
        TuneResult::SwrTooHigh => "SWR above 6:1".into(),
        TuneResult::NotBelow2to1 => "Failed (above 2:1)".into(),
        TuneResult::PowerTooHigh => "Power too high".into(),
        TuneResult::PowerTooLow => "Lo power".into(),
        TuneResult::Unknown(c) => format!("Result {c}"),
    }
}

/// Carries out one button. `Ok(false)` when it was refused (a relay switch
/// while transmitting).
fn act<P: std::io::Read + std::io::Write>(
    amp: &mut Amp<P>,
    a: AmpAction,
    status: &Mutex<AmpStatus>,
    limits: &AmpLimits,
) -> crate::Result<bool> {
    let band_now =
        || status.lock().unwrap_or_else(|e| e.into_inner()).readings.and_then(|r| r.band);
    // SAFETY INTERLOCK (docs/hardrock/safety-interlocks.md, #1): never switch
    // a relay in the RF path (antenna, band filter, tuner) with RF on it.
    // Hot-switching opens the load under power: the reflected power spike
    // makes the amplifier raise its RFL flag (seen on AJ3G's HR500, 17 Oct
    // 2026), stresses the VRF2944 output devices, and arcs and pits the relay
    // contacts. The panel greys those buttons while transmitting (#2); this
    // is the last check, made against the amplifier itself just before the
    // command goes out, so it also covers a remote client or a click that
    // raced the key-up. Things are left as they are if the amplifier is
    // keyed, has drive or output, or does not answer (it often skips its
    // status reply mid-transmission), because "don't know" is not "safe".
    let switches_rf = matches!(
        a,
        AmpAction::SetAntenna(_)
            | AmpAction::SetBand(_)
            | AmpAction::BandUp
            | AmpAction::BandDown
            | AmpAction::SetAtu(_)
    );
    if switches_rf {
        let s = amp.status()?;
        if s.lights().is_some_and(|l| l.tx) || s.forward_w >= 1.0 || s.drive_w >= 0.1 {
            warn!("hardrock: {a:?} refused: transmitting");
            return Ok(false);
        }
    }
    // SAFETY INTERLOCK (docs/hardrock/safety-interlocks.md, #4): never put the
    // amplifier in line (OFF to PTT) while the radio is driving it harder
    // than its drive limit. In OFF the radio's power goes straight through,
    // so a 100 W radio running barefoot is fine there; switched in line, it
    // would drive the amplifier far past its trip level and can damage the
    // output devices. Checked against the amplifier itself just before the
    // command goes out; no answer refuses it, as in #1. The panel greys the
    // button too (#4, AMP window).
    if a == AmpAction::SetPtt(true) {
        let s = amp.status()?;
        if s.drive_w > limits.drive_warn_w {
            warn!(
                "hardrock: {a:?} refused: drive {:.1} W is over the amplifier's limit",
                s.drive_w
            );
            return Ok(false);
        }
    }
    match a {
        AmpAction::SetPtt(on) => amp.set_mode(if on { KeyMode::Ptt } else { KeyMode::Off }),
        AmpAction::SetAntenna(n) => amp.set_antenna(n.clamp(1, 2)),
        AmpAction::SetBand(code) => match Band::from_code(code) {
            Some(b) => amp.set_band(b),
            None => Ok(()),
        },
        // Codes run 1 = 10 m to 10 = 160 m, so "up" in frequency is down in code.
        AmpAction::BandUp => match band_now() {
            Some(c) if c > 1 => amp.set_band(Band::from_code(c - 1).unwrap_or(Band::M10)),
            Some(_) => Ok(()),
            None => amp.set_band(Band::M20),
        },
        AmpAction::BandDown => match band_now() {
            Some(c) if c < 10 => amp.set_band(Band::from_code(c + 1).unwrap_or(Band::M160)),
            Some(_) => Ok(()),
            None => amp.set_band(Band::M20),
        },
        AmpAction::Tune => amp.send(&Command::Tune),
        AmpAction::SetAtu(on) => amp.send(&Command::SetAtuActive(on)),
        AmpAction::SetCelsius(c) => {
            amp.send(&Command::SetTempScale(if c { TempScale::C } else { TempScale::F }))
        }
    }
    .map(|()| true)
}

/// The temperature the panel shows. At rest, `HRTP`'s (the amplifier's own
/// screen), falling back to the status frame's when it stops coming. Keyed,
/// `HRTP` is read only every [`TEMP_POLL_TX`], so the status frame's, which
/// comes with every poll, is shown whenever it is the higher: the heat
/// spreader only climbs while transmitting, and a reading 10 s old was 11 °F
/// behind the amplifier's screen (AJ3G, 18 Oct 2026: 122 °F against 133 °F).
fn shown_temp(screen: Option<f32>, frame: f32, keyed: bool) -> f32 {
    match screen {
        Some(v) if keyed => v.max(frame),
        Some(v) => v,
        None => frame,
    }
}

/// The `HRTP` temperature while it is recent enough to show (see
/// [`TEMP_STALE`]).
fn fresh_temp(screen: Option<(f32, Instant)>, now: Instant) -> Option<f32> {
    screen.filter(|&(_, at)| now.saturating_duration_since(at) <= TEMP_STALE).map(|(v, _)| v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_peak_past_a_trip_level_is_not_a_trip() {
        let l = sdroxide_types::AmpModel::Hr500.limits();
        let mut w = TripWatch::default();
        // 450 W out of 8 W in, IDD spiking past 20 A: still in line.
        let busy =
            AmpReadings { drive_w: 8.0, fwd_w: 450.0, idd_a: 21.0, ..AmpReadings::default() };
        assert!((0..10).all(|_| !w.keyed(&l, &busy)));
        // The first polls of a key-up see no output yet, and are let pass.
        let mut w = TripWatch::default();
        let starting = AmpReadings { drive_w: 5.0, fwd_w: 4.0, ..AmpReadings::default() };
        assert!(!w.keyed(&l, &starting) && !w.keyed(&l, &starting));
        assert!(!w.keyed(&l, &busy));
        // Off line mid-over: the drive comes straight out.
        assert!(!w.keyed(&l, &starting));
        assert!(w.keyed(&l, &starting));
    }

    #[test]
    fn a_real_overdrive_trip_is_seen_and_blamed_on_the_drive() {
        // AJ3G's HR500 on 160 m, from hrctl's log: in line at 2.4 W in, then
        // tripped with 5.6 W in, nothing out and the drain supply cut.
        let l = sdroxide_types::AmpModel::Hr500.limits();
        let good = AmpReadings {
            fwd_w: 305.0,
            rfl_w: 1.0,
            drive_w: 2.4,
            vdd_v: 57.0,
            idd_a: 8.0,
            ptt: true,
            ..AmpReadings::default()
        };
        let tripped = AmpReadings {
            fwd_w: 0.0,
            rfl_w: 0.0,
            drive_w: 5.6,
            vdd_v: 42.0,
            idd_a: 0.0,
            ptt: true,
            ..AmpReadings::default()
        };
        let mut w = TripWatch::default();
        assert!((0..4).all(|_| !w.keyed(&l, &good)));
        // The supply sagging makes it a trip at the first such reading.
        assert!(w.keyed(&l, &tripped));
        assert_eq!(w.cause(&l, &tripped), Some(AmpGauge::Drive));

        let mut a = AmpAlerts::default();
        a.hold(&l, &good);
        a.hold(&l, &tripped);
        a.tripped = true;
        assert!(a.trips().is_empty());
        a.blame(AmpGauge::Drive);
        assert_eq!(a.trips(), vec!["DRV"]);
        assert_eq!(a.get(AmpGauge::Drive), sdroxide_types::AmpLevel::Red);
    }

    #[test]
    fn a_yellow_held_from_before_the_link_opened_shows_green() {
        // Idle, DRV yellow and FWD red still latched from an earlier over.
        let l = Lights { tx: false, fwd: 3, rfl: 1, drv: 2, vdd: 1, idd: 1 };
        let a = lights_seen(l, false);
        assert_eq!(a.drive, AmpLevel::Green);
        // A trip is still shown.
        assert_eq!(a.fwd, AmpLevel::Red);
        assert!(a.tripped);
        // Once a transmission has been seen, the lights are shown as sent.
        assert_eq!(lights_seen(l, true).drive, AmpLevel::Yellow);
    }

    #[test]
    fn tuner_results_read_as_the_panel_words_them() {
        assert_eq!(result_text(TuneResult::PowerTooLow), "Lo power");
        assert_eq!(result_text(TuneResult::Tuned), "Tuned");
    }

    #[test]
    fn network_carries_the_swr() {
        let p = AtuParams::parse("HRTM-L012-C034-H1-B5-S000-F145-").unwrap();
        let n = network_of(p);
        assert_eq!((n.l_step, n.c_step, n.cap_output), (12, 34, true));
        assert!(n.swr.is_some());
    }

    #[test]
    fn a_temperature_that_stops_coming_gives_way() {
        let t0 = Instant::now();
        assert_eq!(fresh_temp(Some((122.0, t0)), t0 + Duration::from_secs(10)), Some(122.0));
        assert_eq!(fresh_temp(Some((122.0, t0)), t0 + Duration::from_secs(16)), None);
        assert_eq!(fresh_temp(None, t0), None);
    }

    #[test]
    fn keyed_the_live_frame_keeps_the_temperature_current() {
        // At rest the amplifier's screen reading is shown.
        assert_eq!(shown_temp(Some(75.0), 73.0, false), 75.0);
        // Keyed, the frame's reading is shown once it has climbed past the
        // last screen reading.
        assert_eq!(shown_temp(Some(122.0), 133.0, true), 133.0);
        assert_eq!(shown_temp(Some(122.0), 120.0, true), 122.0);
        // No screen reading: the frame's.
        assert_eq!(shown_temp(None, 90.0, false), 90.0);
    }
}
