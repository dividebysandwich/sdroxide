//! A front end whose samples arrive later than the command that moved them.
//!
//! A retune is a command, not an event. The engine learns the new centre the
//! instant `set_center_hz` returns, but the pipeline behind it is still full of
//! samples taken at the old one — 131 ms of them, measured on a SunSDR2DX
//! through ExpertSDR3's TCI. Label those with the new centre and every signal
//! in them is drawn at the wrong frequency, displaced by however far the centre
//! moved in the meantime.
//!
//! Standing still nobody would ever see it: the centre stops moving and the
//! picture catches up within one delay. It is a **drag** that makes it visible,
//! because with the view fully zoomed out a pan sends `SetCenter` once per
//! displayed frame (issue #133) — so the label runs continuously ahead of the
//! data, and the whole spectrum sits displaced in the direction of the drag
//! until the operator lets go and it snaps back.
//!
//! The source below is the honest version of that rig: it answers with the tone
//! as it stood at the centre in force `DELAY` ago.

use std::collections::VecDeque;
use std::f32::consts::TAU;
use std::time::{Duration, Instant};

use sdroxide_radio::{Complex32, EngineConfig, EngineHandles, IqSource, Result, start_engine};
use sdroxide_types::{Command, DeviceCaps, SpectrumConfig, SpectrumFrame};

const RATE: f64 = 2_000_000.0;
const CENTER: f64 = 14_200_000.0;
/// A steady carrier, a fifth of the span above the starting centre.
const TONE_HZ: f64 = CENTER + 400_000.0;
/// What the front end's pipeline costs, in the same order as a real TCI rig.
const DELAY: Duration = Duration::from_millis(200);
/// The same, as the thing a pipeline is actually made of: samples in a buffer.
///
/// Counted rather than timed on purpose. A wall clock here makes the test a
/// race — under a loaded machine the sleeps below stretch, the stream runs
/// slower than its nominal rate, and a delay measured in seconds stops being
/// the same delay measured in samples, which is what the engine counts. A real
/// front end has no such ambiguity: its buffer holds a number of samples and
/// the clock that empties it is the sample clock.
const DELAY_SAMPLES: u64 = (DELAY.as_millis() as u64) * (RATE as u64) / 1000;

/// A rig at the end of a pipe: retunes are obeyed `DELAY_SAMPLES` after they
/// are given.
struct Delayed {
    /// Every retune still working its way down the pipe, stamped with the
    /// position on the stream it was given at.
    trail: VecDeque<(u64, f64)>,
    /// The centre the samples now leaving the pipe were taken at.
    streaming: f64,
    /// Samples handed over so far.
    produced: u64,
    commanded: f64,
    phase: f32,
    /// Whether to own up to the delay. The engine can only compensate for a
    /// delay it is told about, so the same rig with this off is the bug.
    declare: bool,
}

impl Delayed {
    fn new(declare: bool) -> Delayed {
        Delayed {
            trail: VecDeque::new(),
            streaming: CENTER,
            produced: 0,
            commanded: CENTER,
            phase: 0.0,
            declare,
        }
    }

    /// The centre the samples handed over right now were taken at.
    fn streaming_center(&mut self) -> f64 {
        let Some(cutoff) = self.produced.checked_sub(DELAY_SAMPLES) else {
            return self.streaming;
        };
        while let Some(&(at, hz)) = self.trail.front() {
            if at > cutoff {
                break;
            }
            self.streaming = hz;
            self.trail.pop_front();
        }
        self.streaming
    }
}

impl IqSource for Delayed {
    fn sample_rate(&self) -> f64 {
        RATE
    }
    fn center_hz(&self) -> f64 {
        self.commanded
    }
    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.commanded = hz;
        self.trail.push_back((self.produced, hz));
        Ok(())
    }
    fn stream_delay_s(&self) -> f64 {
        if self.declare { DELAY.as_secs_f64() } else { 0.0 }
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        std::thread::sleep(Duration::from_millis(4));
        let center = self.streaming_center();
        // The carrier as it stands in samples taken at that centre.
        let step = (TAU as f64 * (TONE_HZ - center) / RATE) as f32;
        let n = buf.len().min(8192);
        for s in buf[..n].iter_mut() {
            *s = Complex32::new(self.phase.cos() * 0.7, self.phase.sin() * 0.7);
            self.phase = (self.phase + step) % TAU;
        }
        self.produced += n as u64;
        Ok(n)
    }
    fn describe(&self) -> String {
        "delayed front end".into()
    }
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "test".into(),
        label: "test".into(),
        rx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(1_000_000.0, 60_000_000.0)],
        ..DeviceCaps::default()
    }
}

fn engine(declare: bool) -> EngineHandles {
    let h = start_engine(
        Box::new(Delayed::new(declare)),
        caps(),
        EngineConfig { remember_session: false, ..Default::default() },
    );
    h.cmd_tx
        .send(Command::SetSpectrumCfg(SpectrumConfig {
            fft_size: 4096,
            display_bins: 2048,
            rows_per_sec: 28,
            db_floor: -140.0,
            db_ceil: 0.0,
            viewport: None,
            smooth: false,
            reassign: false,
            reassign_floor_db: 6.0,
            reassign_dim_db: 0.0,
            fps: 60,
            avg_tc: 0.0,
        }))
        .unwrap();
    h
}

/// Where the strongest column of the newest frame sits, in Hz.
///
/// `None` while no frame has arrived, so a caller can poll. The answer is read
/// off the frame's own axis, which is exactly the thing under test: a frame
/// says where its own left edge is, and everything drawn from it believes that.
fn peak_hz(h: &mut EngineHandles) -> Option<f64> {
    let mut got: Option<SpectrumFrame> = None;
    if h.spectrum_out.update() {
        let f = h.spectrum_out.output_buffer();
        if !f.bins.is_empty() {
            got = Some(f.clone());
        }
    }
    while h.event_rx.try_recv().is_ok() {}
    let f = got?;
    let (i, _) = f.bins.iter().enumerate().max_by_key(|&(_, &b)| b)?;
    Some(f.center_hz - f.span_hz / 2.0 + (i as f64 + 0.5) * f.span_hz / f.bins.len() as f64)
}

/// Poll until a frame arrives, then hand back where it puts the carrier.
fn settle_peak(h: &mut EngineHandles, secs: f64) -> f64 {
    let deadline = Instant::now() + Duration::from_secs_f64(secs);
    let mut last = None;
    while Instant::now() < deadline {
        if let Some(hz) = peak_hz(h) {
            last = Some(hz);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    last.expect("no spectrum frame arrived")
}

/// Pan the way a client does — `SetCenter` once per displayed frame — and
/// report the worst place the carrier was drawn while it was going on.
fn worst_error_during_pan(h: &mut EngineHandles, steps: usize, per_step_hz: f64) -> f64 {
    let mut worst: f64 = 0.0;
    for k in 1..=steps {
        h.cmd_tx.send(Command::SetCenter(CENTER + k as f64 * per_step_hz)).unwrap();
        let until = Instant::now() + Duration::from_millis(16);
        while Instant::now() < until {
            if let Some(hz) = peak_hz(h) {
                worst = worst.max((hz - TONE_HZ).abs());
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    worst
}

/// One column of a 2048-wide panadapter across 2 MHz is 977 Hz; two of them is
/// as close as a peak-pick can be asked to land.
const TOLERANCE_HZ: f64 = 2.0 * RATE / 2048.0;

#[test]
fn a_carrier_stays_put_while_the_window_is_panned_across_it() {
    let mut h = engine(true);
    let settled = settle_peak(&mut h, 1.5);
    assert!(
        (settled - TONE_HZ).abs() < TOLERANCE_HZ,
        "standing still the carrier should be at {TONE_HZ}, drawn at {settled}"
    );

    // 20 kHz a frame for half a second — an ordinary drag, and four times the
    // pipeline's own delay so the error has every chance to show.
    let worst = worst_error_during_pan(&mut h, 30, 20_000.0);
    println!("declared:   worst displacement during the pan {worst:.0} Hz");
    assert!(
        worst < TOLERANCE_HZ * 4.0,
        "the carrier wandered {worst:.0} Hz while the window was panned across it"
    );
}

/// The same rig, saying nothing about its pipeline — which is what every source
/// that has no such pipeline also says, so this is the unchanged path as well
/// as the demonstration that the test can fail.
#[test]
fn a_front_end_that_declares_no_delay_shows_the_error_the_compensation_removes() {
    let mut h = engine(false);
    settle_peak(&mut h, 1.5);
    let worst = worst_error_during_pan(&mut h, 30, 20_000.0);
    println!("undeclared: worst displacement during the pan {worst:.0} Hz");
    assert!(
        worst > TOLERANCE_HZ * 10.0,
        "expected an undeclared pipeline to displace the carrier, worst was only {worst:.0} Hz"
    );
}
