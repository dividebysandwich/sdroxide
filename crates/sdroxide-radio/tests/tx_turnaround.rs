//! How long an over costs, at both ends of it.
//!
//! AX.25 connected mode is half duplex: every over is key up, send, key down,
//! then listen for the answer. What the link can actually achieve is set by two
//! numbers this codebase has never measured — how long the engine takes to get
//! from "transmit" to the first sample on the air, and how coarse its receive
//! loop is when it is *not* transmitting. The first sets TXDELAY. The second
//! decides whether a CSMA slot timer can live in `DigiEngine::poll` at all, or
//! whether it has to be counted on the audio sample clock instead.
//!
//! `Engine::sync_tx_state` re-runs the whole interlock chain on every key-up:
//! the transmit gate, the capability and band rails, then `set_control_mode`,
//! `set_tx_drive`, `set_tune_drive` and `tx_begin` — four calls that are four
//! serial round trips on a CAT rig, and free on an IQ device. `TxChain::new`
//! then designs a 331-tap filter and builds a DUC. On key-down it all comes
//! apart again, and the receive loop does not resume until the source hands
//! back its next block.
//!
//! So the source here is a stopwatch: it timestamps every call the engine makes
//! to it, and can be told to charge a configurable delay for the three control
//! calls to stand in for a serial bus. Nothing in the engine changes.
//!
//! These are **measurements, not assertions**. The test prints a table and
//! passes as long as the engine transmitted at all; the numbers are for the
//! operator and for choosing defaults. Once a packet modem exists and the
//! numbers are known, pin a regression floor here — the posture
//! `sdroxide-digi/tests/wspr_corpus.rs` takes with its recall threshold.
//!
//! Run with output:
//!
//! ```text
//! cargo test -p sdroxide-radio --test tx_turnaround -- --nocapture
//! ```

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sdroxide_radio::{Complex32, EngineConfig, IqSource, Result, start_engine};
use sdroxide_types::{Command, DeviceCaps, Mode, RxId};

/// One thing the engine asked the source to do, and when.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ev {
    /// A receive block was handed back (non-empty).
    Read,
    SetControlMode,
    SetTxDrive,
    SetTuneDrive,
    TxBegin,
    TxWrite,
    TxEnd,
    DiscardRx,
}

type Log = Arc<Mutex<Vec<(Instant, Ev)>>>;

/// A transmit-capable stand-in that records when it was called.
///
/// `cat_delay` is charged inside the three control calls and `tx_begin` — the
/// ones a CAT rig answers over a serial link. At 0 ms this measures the engine's
/// own overhead; at 15 ms it stands in for a CI-V bus at 19200 baud, which is
/// roughly what one short command costs there once the rig has thought about it.
struct TimedSource {
    center: f64,
    rate: f64,
    log: Log,
    cat_delay: Duration,
    /// Samples per receive block. Real hardware takes `rx_block / rate` seconds
    /// to fill one and the read blocks until it has, which is what sets the
    /// receive-side tick granularity — so that is what this sleeps, rather than
    /// a figure of its own choosing.
    rx_block: usize,
}

impl TimedSource {
    fn mark(&self, ev: Ev) {
        self.log.lock().unwrap().push((Instant::now(), ev));
    }
    /// Stand in for a serial round trip.
    fn cat(&self) {
        if !self.cat_delay.is_zero() {
            std::thread::sleep(self.cat_delay);
        }
    }
}

impl IqSource for TimedSource {
    fn sample_rate(&self) -> f64 {
        self.rate
    }
    fn center_hz(&self) -> f64 {
        self.center
    }
    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center = hz;
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        let n = buf.len().min(self.rx_block);
        std::thread::sleep(Duration::from_secs_f64(n as f64 / self.rate));
        for b in buf.iter_mut().take(n) {
            *b = Complex32::new(0.0, 0.0);
        }
        self.mark(Ev::Read);
        Ok(n)
    }
    fn describe(&self) -> String {
        "timed tx source".into()
    }

    fn set_control_mode(&mut self, _mode: Mode) -> Result<()> {
        self.cat();
        self.mark(Ev::SetControlMode);
        Ok(())
    }
    fn set_tx_drive(&mut self, _frac: f64) {
        self.cat();
        self.mark(Ev::SetTxDrive);
    }
    fn set_tune_drive(&mut self, _frac: f64) {
        self.cat();
        self.mark(Ev::SetTuneDrive);
    }
    fn tx_begin(&mut self, _center_hz: f64, rate: f64) -> Result<f64> {
        self.cat();
        self.mark(Ev::TxBegin);
        Ok(rate)
    }
    fn tx_write(&mut self, _samples: &[Complex32]) -> Result<()> {
        self.mark(Ev::TxWrite);
        Ok(())
    }
    fn tx_write_audio(&mut self, _audio: &[f32]) -> Result<()> {
        self.mark(Ev::TxWrite);
        Ok(())
    }
    fn tx_end(&mut self) -> Result<()> {
        self.mark(Ev::TxEnd);
        Ok(())
    }
    fn discard_pending_rx(&mut self) {
        self.mark(Ev::DiscardRx);
    }
}

fn caps(rate: f64, audio_mode: bool) -> DeviceCaps {
    DeviceCaps {
        driver: "timed".into(),
        label: "timed".into(),
        rx_channels: 1,
        tx_channels: 1,
        audio_mode,
        sample_rates: vec![rate],
        freq_ranges_rx: vec![(0.0, 1_000_000_000.0)],
        freq_ranges_tx: vec![(0.0, 1_000_000_000.0)],
        ..DeviceCaps::default()
    }
}

/// What one over cost, in milliseconds.
#[derive(Debug, Default)]
struct Turnaround {
    /// PTT accepted → first sample on the air. The TXDELAY floor.
    key_to_air: Option<f64>,
    /// Last sample on the air → the transmitter released.
    air_to_unkey: Option<f64>,
    /// Transmitter released → the first receive block after it. How long the
    /// station is deaf after its own over, which is exactly when a CSMA modem
    /// needs to hear whether the channel came back.
    unkey_to_rx: Option<f64>,
    /// Receive-block spacing while idle: the finest CSMA slot `poll` could
    /// ever resolve.
    rx_tick_ms: Vec<f64>,
    /// Transmit-block spacing: should be the engine's steady 10 ms pacing.
    tx_tick_ms: Vec<f64>,
}

fn ms(a: Instant, b: Instant) -> f64 {
    b.duration_since(a).as_secs_f64() * 1e3
}

fn median(v: &mut [f64]) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

/// Reduce a call log to the numbers that matter.
///
/// `key_at` is when the transmit command was sent, which is the only thing the
/// operator's modem can actually observe — everything the engine does before
/// the first sample reaches the source is latency the modem pays.
fn analyse(log: &[(Instant, Ev)], key_at: Instant) -> Turnaround {
    let mut t = Turnaround::default();

    let first_write = log.iter().find(|(_, e)| *e == Ev::TxWrite).map(|(i, _)| *i);
    let last_write = log.iter().rev().find(|(_, e)| *e == Ev::TxWrite).map(|(i, _)| *i);
    let tx_end = log.iter().find(|(_, e)| *e == Ev::TxEnd).map(|(i, _)| *i);

    t.key_to_air = first_write.map(|w| ms(key_at, w));
    if let (Some(w), Some(e)) = (last_write, tx_end) {
        t.air_to_unkey = Some(ms(w, e));
    }
    if let Some(e) = tx_end {
        t.unkey_to_rx =
            log.iter().find(|(i, ev)| *ev == Ev::Read && *i > e).map(|(i, _)| ms(e, *i));
    }

    // Receive spacing before the over only: after it, the loop is still
    // settling and the numbers say more about this harness than the engine.
    let mut prev: Option<Instant> = None;
    for (i, ev) in log.iter().filter(|(i, ev)| *ev == Ev::Read && *i < key_at) {
        if let Some(p) = prev {
            t.rx_tick_ms.push(ms(p, *i));
        }
        prev = Some(*i);
        let _ = ev;
    }
    let mut prev: Option<Instant> = None;
    for (i, _) in log.iter().filter(|(_, ev)| *ev == Ev::TxWrite) {
        if let Some(p) = prev {
            t.tx_tick_ms.push(ms(p, *i));
        }
        prev = Some(*i);
    }
    t
}

/// Key up, hold, key down — and time all of it.
///
/// Every wait here is measured in *receive blocks*, not in fixed milliseconds,
/// because the block is what the engine's main loop turns on: `source.read()`
/// blocks for one, and the transmit pump is the same loop a few lines further
/// down. A wait shorter than a block is a wait the engine spends entirely
/// inside `read`, and the thing being measured never gets a turn.
///
/// That is not hypothetical — it is what this test used to do. A fixed 300 ms
/// over against the slowest case here, a 16384-sample buffer at 48 kHz, is 300
/// ms against a 341 ms block: the engine keyed, sat in one `read`, and unkeyed
/// again without a single transmit block in between, so `key→air` was a dash
/// and the assertion below failed on an engine that was behaving correctly.
fn one_over(audio_mode: bool, cat_delay: Duration, rx_block: usize) -> Turnaround {
    let rate = if audio_mode { 48_000.0 } else { 2_400_000.0 };
    // How long the source takes to hand over one receive block, which is this
    // engine's whole loop granularity.
    let block = Duration::from_secs_f64(rx_block as f64 / rate);
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let src =
        TimedSource { center: 144_800_000.0, rate, log: Arc::clone(&log), cat_delay, rx_block };
    let cfg = EngineConfig { tx_ham_only: false, ..Default::default() };
    let mut h = start_engine(Box::new(src), caps(rate, audio_mode), cfg);
    let thread = h.thread.take();

    // Let the engine settle and take several receive blocks, so the idle tick
    // has something to take a median of. Four of them, or the measurement is a
    // hole — and never less than the 1.4 s the engine wants to come up.
    h.cmd_tx.send(Command::SetMode { rx: RxId::Main, mode: Mode::Nfm }).unwrap();
    std::thread::sleep((block * 4).max(Duration::from_millis(1400)));

    // The over: at least three blocks, so there is room for the loop to come
    // round, notice the key-up and pump audio before the key-down.
    let key_at = Instant::now();
    h.cmd_tx.send(Command::SetPtt(true)).unwrap();
    std::thread::sleep((block * 3).max(Duration::from_millis(300)));
    h.cmd_tx.send(Command::SetPtt(false)).unwrap();
    // …and long enough after it for at least one receive block to arrive, which
    // is what `unkey_to_rx` is measuring.
    std::thread::sleep((block * 2).max(Duration::from_millis(400)));

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }

    let log = log.lock().unwrap().clone();
    analyse(&log, key_at)
}

fn report(name: &str, t: &mut Turnaround) {
    let f = |v: Option<f64>| match v {
        Some(v) => format!("{v:8.1}"),
        None => "       -".to_string(),
    };
    println!(
        "{name:<34} key→air {}  air→unkey {}  unkey→rx {}  rx tick {:6.1}  tx tick {:5.1}",
        f(t.key_to_air),
        f(t.air_to_unkey),
        f(t.unkey_to_rx),
        median(&mut t.rx_tick_ms),
        median(&mut t.tx_tick_ms),
    );
}

/// The five numbers, across the two source shapes that matter for packet: an IQ
/// SDR (the engine modulates) and a CAT rig on a sound card (the rig does), each
/// with and without a serial bus in the way.
///
/// The one to watch is **rx tick**. A KISS-style CSMA slot time is classically
/// 10 ms; if the idle receive loop only comes round every few hundred
/// milliseconds, a slot timer driven from `DigiEngine::poll` cannot resolve it,
/// and the packet controller must count slots on the audio sample clock inside
/// `on_rx_audio` instead. That is a structural decision, and this is the
/// measurement it rests on.
#[test]
fn over_turnaround_and_loop_granularity() {
    println!();
    println!("--- TX turnaround, milliseconds (median over the run) ---");

    // IQ path: 2.4 Msps in 16384-sample blocks — 6.8 ms of signal each, so the
    // receive loop comes round often whatever else it is doing.
    let mut t = one_over(false, Duration::ZERO, 16384);
    report("IQ SDR 2.4 Msps, no CAT bus", &mut t);
    assert!(t.key_to_air.is_some(), "the engine never transmitted");

    let mut t = one_over(false, Duration::from_millis(15), 16384);
    report("IQ SDR 2.4 Msps, 15 ms CAT bus", &mut t);

    // Audio path: a CAT rig's sound card at 48 kHz. The same 16384-sample
    // buffer is 341 ms of audio here — the case that decides whether `poll`
    // can carry a CSMA slot clock.
    let mut t = one_over(true, Duration::ZERO, 16384);
    report("CAT rig audio 48 k/16384, no bus", &mut t);

    let mut t = one_over(true, Duration::from_millis(15), 16384);
    report("CAT rig audio 48 k/16384, 15 ms bus", &mut t);
    assert!(t.key_to_air.is_some(), "the engine never transmitted in audio mode");

    // The same rig with a small ALSA period, for contrast: block size is the
    // only thing that moves the receive tick.
    let mut t = one_over(true, Duration::from_millis(15), 1024);
    report("CAT rig audio 48 k/1024, 15 ms bus", &mut t);

    println!();
}

/// Does a CW skimmer running beside a packet session steal the timing?
///
/// The question is real: the skimmer links DeepCW, rten grabs every core per
/// inference, and a packet station's CSMA slot clock and transmit pacing share
/// the engine thread. But the skimmer is an operator setting independent of
/// mode, so it can be on during a Winlink session — and if it does cost
/// timing, that is a rule worth adding (the precedent is `audio_mode` forcing
/// `SkimmerSettings::OFF` and writing it back so the UI shows it).
///
/// Measured rather than assumed. Compares the receive-loop tick and the over
/// turnaround with the machine idle against the same run under an artificial
/// all-core load standing in for rten's fan-out.
#[test]
fn a_busy_machine_does_not_wreck_the_receive_tick() {
    println!();
    println!("--- receive tick under CPU contention (ms) ---");

    let mut quiet = one_over(true, Duration::from_millis(15), 1024);
    report("idle machine", &mut quiet);

    // Stand in for rten's fan-out: one spinning thread per core, which is what
    // a DeepCW inference does to this machine.
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let workers: Vec<_> = (0..std::thread::available_parallelism().map_or(8, |n| n.get()))
        .map(|_| {
            let stop = Arc::clone(&stop);
            std::thread::spawn(move || {
                let mut x = 0u64;
                while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                    x = x.wrapping_mul(6364136223846793005).wrapping_add(1);
                    std::hint::black_box(x);
                }
            })
        })
        .collect();

    let mut loaded = one_over(true, Duration::from_millis(15), 1024);
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    for w in workers {
        let _ = w.join();
    }
    report("every core spinning", &mut loaded);

    // Not an assertion on the absolute figures — this runs on whatever machine
    // CI gives it. What matters is that the receive loop still turns over: a
    // tick that collapsed to seconds would mean CSMA cannot see the channel,
    // and that is the finding worth failing on.
    let tick = median(&mut loaded.rx_tick_ms);
    assert!(
        tick.is_finite() && tick < 1000.0,
        "receive tick collapsed to {tick:.0} ms under load — CSMA would be blind"
    );
    println!();
}
