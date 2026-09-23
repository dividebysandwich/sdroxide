//! `UvPacketController` — the UVPacket packet receiver, receive only.
//!
//! UVPacket is the odd one out among the modes here: it is not a WSJT-X
//! message mode but a packet protocol carrying an application byte pipe, and
//! its frames are **not slotted** — one can begin anywhere. So instead of a
//! slot scheduler this holds a rolling window of 12 kHz audio and re-scans it
//! as it advances, exactly as [`crate::AcarsController`] and
//! [`crate::DscController`] hold a channel open.
//!
//! # Why a worker thread and a rolling window
//!
//! The longest frame (UltraRobust, 32 payload blocks) is nearly seven seconds
//! of audio, so the window has to be long enough to hold one; the decoder scans
//! the whole window for preambles, which is tens of milliseconds of work. That
//! runs on its own thread, and a window that overlaps its predecessor means the
//! same frame is seen more than once — [`UvPacketController::seen_recently`]
//! keeps a short-lived set of frame hashes so each is filed once.
//!
//! The sub-mode is carried by the preamble and detected, not chosen, so there
//! is no operator setting and no `set_config` geometry to rebuild.
//!
//! Receive only in this build: transmit needs an application layer.

use std::collections::VecDeque;
use std::hash::{Hash, Hasher};
use std::sync::mpsc::{Receiver, Sender};
use std::time::SystemTime;

use sdroxide_dsp::MonoResampler;
use sdroxide_types::{
    DigiConfig, DigiStatus, Mode, QsoStep, UVPACKET_FRAME_MAX, UvPacketFrame, UvPacketStatus,
};

use crate::DigiEngine;
use crate::controller::DigiAction;
use crate::modem::decode_uvpacket;
use crate::params::DECODE_RATE;

/// Audio window scanned, in seconds. Must exceed the longest frame: an
/// UltraRobust 32-block burst is ~6.8 s before the RRC tail.
const WINDOW_S: f64 = 9.0;
/// How much new audio has to accumulate before the window is scanned again.
const SCAN_STEP_S: f64 = 0.5;
/// How long a frame hash suppresses a repeat of the same frame, in seconds.
/// Longer than the window, so overlapping scans do not file one frame twice.
const DEDUP_TTL_S: u64 = 12;

/// One window handed to the decode worker: the audio and the wall-clock time
/// to stamp its frames with.
struct DecodeJob {
    audio: Vec<f32>,
    at: i64,
}

pub struct UvPacketController {
    cfg: DigiConfig,
    resampler: Option<MonoResampler>,
    /// Rolling 12 kHz audio, newest last, capped at [`WINDOW_S`].
    buf: Vec<f32>,
    tap_scratch: Vec<f32>,
    /// Smoothed audio level, for the panel's meter.
    level: f32,
    frames: Vec<UvPacketFrame>,
    total: u64,
    /// Frame hashes filed recently, with when, so an overlapping scan does not
    /// file the same frame again.
    seen: VecDeque<(u64, SystemTime)>,
    /// Samples appended since the last scan was dispatched.
    since_scan: usize,

    job_tx: Sender<DecodeJob>,
    res_rx: Receiver<Vec<UvPacketFrame>>,
    _worker: std::thread::JoinHandle<()>,
    pending: bool,
    status_dirty: bool,
}

impl UvPacketController {
    pub fn new(cfg: DigiConfig, tap_rate: f64) -> Self {
        let (job_tx, job_rx) = std::sync::mpsc::channel::<DecodeJob>();
        let (res_tx, res_rx) = std::sync::mpsc::channel::<Vec<UvPacketFrame>>();
        let worker = std::thread::Builder::new()
            .name("sdroxide-uvpacket-decode".into())
            .spawn(move || {
                while let Ok(job) = job_rx.recv() {
                    let frames = decode_uvpacket(&job.audio, job.at);
                    if res_tx.send(frames).is_err() {
                        break;
                    }
                }
            })
            .expect("spawn uvpacket decode worker");

        UvPacketController {
            cfg,
            resampler: MonoResampler::new(tap_rate, DECODE_RATE),
            buf: Vec::new(),
            tap_scratch: Vec::new(),
            level: 0.0,
            frames: Vec::new(),
            total: 0,
            seen: VecDeque::new(),
            since_scan: 0,
            job_tx,
            res_rx,
            _worker: worker,
            pending: false,
            status_dirty: true,
        }
    }

    fn window_samples() -> usize {
        (WINDOW_S * DECODE_RATE) as usize
    }

    fn step_samples() -> usize {
        (SCAN_STEP_S * DECODE_RATE) as usize
    }

    fn seen_recently(&self, hash: u64, now: SystemTime) -> bool {
        self.seen.iter().any(|&(h, t)| {
            h == hash && now.duration_since(t).map(|d| d.as_secs() < DEDUP_TTL_S).unwrap_or(false)
        })
    }

    fn prune_seen(&mut self, now: SystemTime) {
        while let Some(&(_, t)) = self.seen.front() {
            let expired = now.duration_since(t).map(|d| d.as_secs() >= DEDUP_TTL_S).unwrap_or(true);
            if expired {
                self.seen.pop_front();
            } else {
                break;
            }
        }
    }

    fn digi_status(&self) -> DigiStatus {
        let mut s = DigiStatus::idle(self.cfg.clone());
        s.mode = Mode::UvPacket;
        s.step = QsoStep::Idle;
        s.audio_hz = sdroxide_types::UVPACKET_AUDIO_CENTRE_HZ;
        s.uvpacket = Some(UvPacketStatus {
            level: self.level,
            frames: self.frames.clone(),
            frames_total: self.total,
        });
        s
    }
}

/// A hash over the fields that identify a frame, for the recent-seen set. The
/// timestamp is deliberately not included: the same bytes in an overlapping
/// window are the same frame, and the point is to file it once.
fn frame_hash(f: &UvPacketFrame) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    f.mode.hash(&mut h);
    f.app_type.hash(&mut h);
    f.sequence.hash(&mut h);
    f.block_count.hash(&mut h);
    f.payload.hash(&mut h);
    h.finish()
}

fn now_unix(now: SystemTime) -> i64 {
    now.duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

impl DigiEngine for UvPacketController {
    fn mode(&self) -> Mode {
        Mode::UvPacket
    }

    fn on_rx_audio(&mut self, tap: &[f32]) {
        self.tap_scratch.clear();
        match &mut self.resampler {
            Some(r) => r.push(tap, &mut self.tap_scratch),
            None => self.tap_scratch.extend_from_slice(tap),
        }
        self.buf.extend_from_slice(&self.tap_scratch);
        let cap = Self::window_samples();
        if self.buf.len() > cap {
            let excess = self.buf.len() - cap;
            self.buf.drain(..excess);
        }
        self.since_scan += self.tap_scratch.len();

        if !tap.is_empty() {
            let energy = tap.iter().map(|s| s * s).sum::<f32>() / tap.len() as f32;
            self.level = 0.9 * self.level + 0.1 * energy.sqrt();
        }
    }

    fn poll(&mut self, now: SystemTime, _dial_hz: f64) -> Vec<DigiAction> {
        let mut actions = Vec::new();

        while let Ok(frames) = self.res_rx.try_recv() {
            self.pending = false;
            for f in frames {
                let hash = frame_hash(&f);
                if !self.seen_recently(hash, now) {
                    self.seen.push_back((hash, now));
                    self.frames.push(f);
                    self.total += 1;
                    self.status_dirty = true;
                }
            }
            if self.frames.len() > UVPACKET_FRAME_MAX {
                let excess = self.frames.len() - UVPACKET_FRAME_MAX;
                self.frames.drain(..excess);
            }
            self.prune_seen(now);
        }

        // Dispatch a scan once enough new audio has arrived and the previous
        // one has come back. A window that is still short is decoded anyway
        // (the decoder returns nothing if it is), so a burst right after
        // entering the mode is not lost waiting for nine seconds to accumulate.
        if !self.pending && self.since_scan >= Self::step_samples() {
            self.since_scan = 0;
            let audio = self.buf.clone();
            self.pending = true;
            let _ = self.job_tx.send(DecodeJob { audio, at: now_unix(now) });
        }

        if self.status_dirty {
            self.status_dirty = false;
            actions.push(DigiAction::Status(self.digi_status()));
        }
        actions
    }

    fn tx_burst_active(&self) -> bool {
        false
    }

    fn fill_tx_block(&mut self, _out: &mut [f32]) -> bool {
        false
    }

    fn on_burst_done(&mut self) {}

    fn abort(&mut self) {
        self.buf.clear();
        self.since_scan = 0;
        self.status_dirty = true;
    }

    fn abort_tx(&mut self) {}

    fn set_config(&mut self, cfg: DigiConfig) {
        self.cfg = cfg;
        self.status_dirty = true;
    }

    fn clear_rx(&mut self) {
        self.frames.clear();
        self.total = 0;
        self.seen.clear();
        self.status_dirty = true;
    }

    fn set_audio_hz(&mut self, _hz: f32) {}

    /// Fixed by the modem: the four tones span 800–2600 Hz and the decoder
    /// searches that window, so there is nothing here for an operator to tune.
    fn audio_hz(&self) -> f32 {
        sdroxide_types::UVPACKET_AUDIO_CENTRE_HZ
    }

    fn status(&self) -> DigiStatus {
        self.digi_status()
    }
}
