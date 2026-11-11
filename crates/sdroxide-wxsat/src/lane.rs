//! The weather-satellite lane the engine runs: a downconverter on the channel,
//! a worker thread, and a status with the picture lines new since the last one.
//!
//! The realtime engine thread hands I/Q blocks to the worker over a bounded
//! channel (a dropped block costs a fraction of a line) and drains statuses
//! non-blocking, like `sdroxide_hfdl::HfdlController`.

use std::collections::VecDeque;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender, bounded, select, unbounded};
use sdroxide_dsp::{Complex32, ComplexResampler, Ddc};
use sdroxide_types::{
    WXSAT_MAX_NEW_LINES, WXSAT_MAX_NEW_ROWS, WxsatKind, WxsatRow, WxsatSettings, WxsatStatus,
};

use crate::apt::{AptDecoder, AptDemod, AptLine};
use crate::lrpt::jpeg::PICTURE_WIDTH;
use crate::lrpt::picture::{FinishedRow, LrptPicture, ROW_LINES};
use crate::lrpt::rx::LrptRx;
use crate::{WXSAT_LANE_RATE_HZ, lane_rate_hz};

/// Bounded I/Q queue depth: a couple of seconds of lane at a typical device
/// read.
const IQ_QUEUE_DEPTH: usize = 256;

/// A status goes out at least once every N blocks even when no line was
/// decoded, so the level meter keeps moving.
const IDLE_TICK: u64 = 250;

/// Seconds without a segment before the last rows of a pass are let go.
const QUIET_S: f64 = 3.0;

enum Ctl {
    Enabled(bool),
    Flush,
    Stop,
}

/// What a worker runs. One per lane, built once and never moved, so the size
/// of the APT arm costs nothing.
#[allow(clippy::large_enum_variant)]
enum Work {
    Apt {
        demod: AptDemod,
        dec: AptDecoder,
        px: Vec<f32>,
        lines: Vec<AptLine>,
        quality: f32,
        total: u32,
    },
    Lrpt {
        rx: Box<LrptRx>,
        pic: LrptPicture,
        pending: VecDeque<FinishedRow>,
        last_segment: Option<Instant>,
        power: f32,
    },
}

impl Work {
    fn new(kind: WxsatKind) -> Self {
        match kind {
            WxsatKind::Apt => Work::Apt {
                demod: AptDemod::new(WXSAT_LANE_RATE_HZ),
                dec: AptDecoder::new(),
                px: Vec::new(),
                lines: Vec::new(),
                quality: 0.0,
                total: 0,
            },
            WxsatKind::Lrpt => Work::Lrpt {
                rx: Box::new(LrptRx::new()),
                pic: LrptPicture::new(),
                pending: VecDeque::new(),
                last_segment: None,
                power: 0.0,
            },
        }
    }

    /// Take a block of lane I/Q; says whether there is something to report.
    fn feed(&mut self, iq: &[Complex32], enabled: bool) -> bool {
        match self {
            Work::Apt { demod, dec, px, lines, .. } => {
                px.clear();
                lines.clear();
                if enabled {
                    demod.process(iq, px);
                    dec.process(px, lines);
                }
                !lines.is_empty()
            }
            Work::Lrpt { rx, pic, pending, last_segment, power } => {
                for x in iq {
                    *power += (x.norm_sqr() - *power) * 1e-4;
                }
                if !enabled {
                    return false;
                }
                let segs = rx.process(iq);
                if !segs.is_empty() {
                    *last_segment = Some(Instant::now());
                }
                segs.iter().for_each(|s| pic.push(s));
                let quiet =
                    last_segment.is_some_and(|t| t.elapsed() > Duration::from_secs_f64(QUIET_S));
                pending.extend(pic.take_finished(quiet));
                !pending.is_empty()
            }
        }
    }

    /// Let go every row held back (the decoder is being switched off).
    fn flush(&mut self) {
        if let Work::Lrpt { pic, pending, .. } = self {
            pending.extend(pic.take_finished(true));
        }
    }

    fn status(&mut self, enabled: bool, pass: u32, first_apt: &mut u32) -> WxsatStatus {
        match self {
            Work::Apt { demod, dec, lines, quality, total, .. } => {
                // A client that fell far behind loses the oldest.
                let skip = lines.len().saturating_sub(WXSAT_MAX_NEW_LINES);
                let first = *total + skip as u32;
                if let Some(l) = lines.last() {
                    *quality = l.quality;
                }
                let new_lines: Vec<Vec<u8>> =
                    lines.drain(..).skip(skip).map(|l| l.pixels).collect();
                *total = first + new_lines.len() as u32;
                *first_apt = first;
                WxsatStatus {
                    running: enabled,
                    level_dbfs: demod.level_dbfs(),
                    locked: dec.locked(),
                    quality: *quality,
                    pass,
                    total_lines: *total,
                    first_line: first,
                    new_lines,
                    kind: WxsatKind::Apt,
                    ..Default::default()
                }
            }
            Work::Lrpt { rx, pic, pending, power, .. } => {
                let n = pending.len().min(WXSAT_MAX_NEW_ROWS);
                let rows: Vec<WxsatRow> = pending
                    .drain(..n)
                    .map(|f| WxsatRow { apid: f.apid, row: f.row, lines: f.lines })
                    .collect();
                let (carrier, evm) = rx.demod_figures();
                debug_assert_eq!(
                    PICTURE_WIDTH * ROW_LINES,
                    sdroxide_types::WXSAT_LRPT_WIDTH * sdroxide_types::WXSAT_LRPT_ROW_LINES
                );
                WxsatStatus {
                    running: enabled,
                    level_dbfs: 10.0 * power.max(1e-12).log10(),
                    locked: rx.locked(),
                    quality: (1.0 - evm).clamp(0.0, 1.0),
                    pass,
                    total_lines: (pic.rows() * ROW_LINES) as u32,
                    kind: WxsatKind::Lrpt,
                    frames_ok: rx.stats().frames_ok as u32,
                    carrier_hz: carrier as f32,
                    rows,
                    ..Default::default()
                }
            }
        }
    }
}

/// The decoder worker, fed lane-rate I/Q.
pub struct WxsatController {
    iq_tx: Sender<Vec<Complex32>>,
    ctl_tx: Sender<Ctl>,
    res_rx: Receiver<WxsatStatus>,
    worker: Option<JoinHandle<()>>,
}

impl WxsatController {
    /// `pass` is the number the statuses carry, so the client can tell a new
    /// picture from the continuation of the old one.
    pub fn new(pass: u32, kind: WxsatKind) -> Self {
        let (iq_tx, iq_rx) = bounded::<Vec<Complex32>>(IQ_QUEUE_DEPTH);
        let (ctl_tx, ctl_rx) = unbounded::<Ctl>();
        let (res_tx, res_rx) = unbounded::<WxsatStatus>();
        let worker = std::thread::Builder::new()
            .name("sdroxide-wxsat".into())
            .spawn(move || {
                let mut work = Work::new(kind);
                let mut enabled = true;
                let mut tick = 0u64;
                let mut first_apt = 0u32;
                loop {
                    select! {
                        recv(ctl_rx) -> msg => match msg {
                            Ok(Ctl::Enabled(on)) => {
                                if enabled && !on {
                                    work.flush();
                                    let st = work.status(false, pass, &mut first_apt);
                                    let _ = res_tx.send(st);
                                }
                                enabled = on;
                            }
                            Ok(Ctl::Flush) => {
                                work.flush();
                                let st = work.status(enabled, pass, &mut first_apt);
                                let _ = res_tx.send(st);
                            }
                            Ok(Ctl::Stop) | Err(_) => break,
                        },
                        recv(iq_rx) -> msg => match msg {
                            Ok(iq) => {
                                tick += 1;
                                let news = work.feed(&iq, enabled);
                                if news || tick.is_multiple_of(IDLE_TICK) || tick == 1 {
                                    let st = work.status(enabled, pass, &mut first_apt);
                                    let _ = res_tx.send(st);
                                }
                            }
                            Err(_) => break,
                        },
                    }
                }
            })
            .expect("spawn wxsat worker");
        WxsatController { iq_tx, ctl_tx, res_rx, worker: Some(worker) }
    }

    /// Realtime path: hand a block of lane-rate I/Q to the worker. Non-blocking.
    pub fn on_rx_iq(&self, iq: &[Complex32]) {
        let _ = self.iq_tx.try_send(iq.to_vec());
    }

    pub fn set_enabled(&self, on: bool) {
        let _ = self.ctl_tx.send(Ctl::Enabled(on));
    }

    /// Let go every row of picture still held back.
    pub fn flush(&self) {
        let _ = self.ctl_tx.send(Ctl::Flush);
    }

    /// Drain what the worker has said, oldest first. Every status is kept: each
    /// carries lines nobody else has, so they cannot be collapsed to the newest.
    pub fn poll(&self) -> Vec<WxsatStatus> {
        let mut out = Vec::new();
        while let Ok(s) = self.res_rx.try_recv() {
            out.push(s);
        }
        out
    }
}

impl Drop for WxsatController {
    fn drop(&mut self) {
        let _ = self.ctl_tx.send(Ctl::Stop);
        if let Some(h) = self.worker.take() {
            let _ = h.join();
        }
    }
}

/// The channel mixed out of the wide window, decoded.
pub struct WxsatLane {
    in_rate: f64,
    kind: WxsatKind,
    ddc: Ddc,
    rs: Option<ComplexResampler>,
    buf: Vec<Complex32>,
    res_buf: Vec<Complex32>,
    freq_hz: f64,
    ctl: WxsatController,
    pass: u32,
}

impl WxsatLane {
    pub fn new(in_rate: f64, center_hz: f64, cfg: WxsatSettings, pass: u32) -> Self {
        let rate = lane_rate_hz(cfg.kind);
        let mut ddc = Ddc::new(in_rate, rate);
        ddc.set_offset_hz(cfg.frequency_hz - center_hz);
        let rs = ComplexResampler::new(ddc.out_rate(), rate);
        Self {
            in_rate,
            kind: cfg.kind,
            ddc,
            rs,
            buf: Vec::new(),
            res_buf: Vec::new(),
            freq_hz: cfg.frequency_hz,
            ctl: WxsatController::new(pass, cfg.kind),
            pass,
        }
    }

    /// The sample rate this lane was built for; a different one needs a new lane.
    pub fn in_rate(&self) -> f64 {
        self.in_rate
    }

    /// The signal this lane decodes; another one needs a new lane.
    pub fn kind(&self) -> WxsatKind {
        self.kind
    }

    pub fn pass(&self) -> u32 {
        self.pass
    }

    pub fn frequency_hz(&self) -> f64 {
        self.freq_hz
    }

    /// Re-seat the mixer after a retune of the window or of the channel.
    pub fn set_tuning(&mut self, center_hz: f64, freq_hz: f64) {
        self.freq_hz = freq_hz;
        self.ddc.set_offset_hz(freq_hz - center_hz);
    }

    /// Hand the engine's wide I/Q to the lane.
    pub fn on_wide_iq(&mut self, iq: &[Complex32]) {
        self.buf.clear();
        self.ddc.process(iq, &mut self.buf);
        let lane = match self.rs.as_mut() {
            Some(rs) => {
                self.res_buf.clear();
                rs.push(&self.buf, &mut self.res_buf);
                &self.res_buf
            }
            None => &self.buf,
        };
        self.ctl.on_rx_iq(lane);
    }

    /// Let go every row of picture still held back.
    pub fn flush(&self) {
        self.ctl.flush();
    }

    pub fn poll(&self) -> Vec<WxsatStatus> {
        self.ctl.poll()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apt::{IMAGE_A_START, IMAGE_WIDTH, synth};

    fn wait_lines(c: &WxsatController, want: u32) -> Vec<WxsatStatus> {
        let mut all = Vec::new();
        for _ in 0..500 {
            all.extend(c.poll());
            if all.last().is_some_and(|s| s.total_lines >= want) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        all
    }

    #[test]
    fn the_worker_reports_the_lines_once_each_with_their_numbers() {
        let a: Vec<u8> = (0..IMAGE_WIDTH).map(|i| (i * 255 / (IMAGE_WIDTH - 1)) as u8).collect();
        let lines: Vec<Vec<u8>> = (0..12).map(|_| synth::line(&a, &a)).collect();
        let iq = synth::modulate(&lines, WXSAT_LANE_RATE_HZ, 1.0);
        let c = WxsatController::new(5, WxsatKind::Apt);
        for chunk in iq.chunks(2048) {
            c.on_rx_iq(chunk);
        }
        let all = wait_lines(&c, 9);
        let total = all.last().expect("a status").total_lines;
        assert!(total >= 9, "{total} lines");
        // Every line once, numbered in order.
        let mut next = 0u32;
        for s in &all {
            assert_eq!(s.pass, 5);
            assert_eq!(s.first_line, next);
            next += s.new_lines.len() as u32;
        }
        assert_eq!(next, total);
        let first = all.iter().find_map(|s| s.new_lines.first()).unwrap();
        assert_eq!(first.len(), sdroxide_types::WXSAT_LINE_PIXELS);
        let mid = first[IMAGE_A_START + IMAGE_WIDTH / 2];
        assert!((i32::from(mid) - 127).abs() < 30, "mid grey {mid}");
    }

    #[test]
    fn a_lane_on_a_wide_window_finds_a_channel_off_centre() {
        let a: Vec<u8> = vec![128; IMAGE_WIDTH];
        let lines: Vec<Vec<u8>> = (0..14).map(|_| synth::line(&a, &a)).collect();
        let narrow = synth::modulate(&lines, WXSAT_LANE_RATE_HZ, 1.0);
        // Put the signal 100 kHz off the centre of a 480 kS/s window: upsample
        // by 10 (hold) and mix up.
        let wide_rate = 480_000.0;
        let mut wide = Vec::with_capacity(narrow.len() * 10);
        let mut ph = 0.0f64;
        let w = std::f64::consts::TAU * 100_000.0 / wide_rate;
        for x in &narrow {
            for _ in 0..10 {
                wide.push(*x * Complex32::new(ph.cos() as f32, ph.sin() as f32));
                ph += w;
            }
        }
        let cfg =
            WxsatSettings { enabled: true, frequency_hz: 137_100_000.0, ..Default::default() };
        let mut lane = WxsatLane::new(wide_rate, 137_000_000.0, cfg, 1);
        for chunk in wide.chunks(16_384) {
            lane.on_wide_iq(chunk);
        }
        let mut total = 0;
        for _ in 0..500 {
            if let Some(s) = lane.poll().last() {
                total = total.max(s.total_lines);
            }
            if total >= 6 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(total >= 6, "{total} lines through the downconverter");
    }

    /// The real Meteor pass of the test data, as a wide window centred on the
    /// channel: the lane has to mix, resample, decode and send finished rows.
    #[test]
    fn an_lrpt_lane_turns_a_real_pass_into_rows_of_picture() {
        let raw = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/testdata/lrpt/meteor_lrpt_137900_3s.wav"
        ))
        .unwrap();
        let rate = f64::from(u32::from_le_bytes(raw[24..28].try_into().unwrap()));
        let iq: Vec<Complex32> = raw[44..]
            .chunks(4)
            .filter(|c| c.len() == 4)
            .map(|c| {
                Complex32::new(
                    f32::from(i16::from_le_bytes([c[0], c[1]])) / 32768.0,
                    f32::from(i16::from_le_bytes([c[2], c[3]])) / 32768.0,
                )
            })
            .collect();
        let cfg =
            WxsatSettings { enabled: true, frequency_hz: 137_900_000.0, kind: WxsatKind::Lrpt };
        let mut lane = WxsatLane::new(rate, 137_900_000.0, cfg, 3);
        assert_eq!(lane.kind(), WxsatKind::Lrpt);
        let mut got = Vec::new();
        for chunk in iq.chunks(8192) {
            lane.on_wide_iq(chunk);
            // The queue is bounded and drops on backpressure: let the worker keep up.
            std::thread::sleep(std::time::Duration::from_millis(5));
            got.extend(lane.poll());
        }
        // The worker is slower than real time in a debug build: wait until it
        // has caught up (no new frame for a second), then let the last rows go.
        let mut quiet_since = std::time::Instant::now();
        let mut frames = 0;
        while quiet_since.elapsed() < std::time::Duration::from_secs(1) {
            std::thread::sleep(std::time::Duration::from_millis(50));
            for st in lane.poll() {
                if st.frames_ok != frames {
                    frames = st.frames_ok;
                    quiet_since = std::time::Instant::now();
                }
                got.push(st);
            }
        }
        lane.flush();
        std::thread::sleep(std::time::Duration::from_millis(300));
        got.extend(lane.poll());
        let last = got.last().expect("a status");
        assert_eq!((last.kind, last.pass), (WxsatKind::Lrpt, 3));
        assert!(last.frames_ok >= 15, "{} frames", last.frames_ok);
        let rows: Vec<&WxsatRow> = got.iter().flat_map(|s| s.rows.iter()).collect();
        assert!(rows.len() >= 3, "{} rows", rows.len());
        assert!(rows.iter().all(|r| r.lines.len() == 1568 * 8 && (64..=69).contains(&r.apid)));
        // The picture client builds an image from them.
        let mut pic = sdroxide_types::WxsatPicture::default();
        got.iter().for_each(|s| pic.apply(s));
        assert!(!pic.is_empty());
    }
}
