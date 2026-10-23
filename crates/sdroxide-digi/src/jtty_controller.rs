//! `JttyController` — the JTTY asynchronous text mode, receive and transmit.
//!
//! JTTY is not a slotted mode: a transmission can start at any instant, so
//! there is no T/R period and no frame boundary to lock to. The receiver holds
//! a rolling buffer of audio and, every half second, searches the starts it
//! has not searched yet — each start once, with a frame's worth of audio after
//! it — across the audio band the reference searches (1200–1800 Hz for the
//! lowest tone) and the operator's own frequency either side.
//!
//! # Messages, not frames
//!
//! A JTTY **message** is one or more ~1.888 s **frames** sent back to back; the
//! last carries the end-of-message flag. Each decoded frame is filed against
//! the run it continues — the one at the same frequency whose next frame was
//! due exactly where this one starts — so two stations never merge, a message
//! longer than the buffer still completes, and the log shows one line per
//! message that grows as its frames arrive rather than one line per glimpse.
//!
//! Transmit packs the typed line, synthesizes the burst at the operator's
//! audio frequency and sends it once.

use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, Sender};
use std::time::SystemTime;

use sdroxide_dsp::MonoResampler;
use sdroxide_dsp::jtty::{self, JTTY_RATE};
use sdroxide_types::{
    DigiConfig, DigiStatus, JTTY_MESSAGE_MAX, JttyMessage, JttyStatus, Mode, QsoStep,
};

use crate::DigiEngine;
use crate::controller::DigiAction;

/// Most audio buffered, in seconds: the frame being searched and room for the
/// worker to fall behind.
const WINDOW_S: f64 = 12.0;
/// How much new audio accumulates before it is searched.
const SCAN_STEP_S: f64 = 0.5;
/// Audio kept either side of what is searched, so the FFT-based Hilbert
/// transform's wrap-around at the buffer ends lands outside it. Two symbols,
/// a whole number of search steps.
const EDGE: usize = 2 * jtty::JTTY_NSPS;
/// One frame, in samples.
const FRAME_LEN: usize = jtty::JTTY_FRAME_SYMBOLS * jtty::JTTY_NSPS;
/// Default audio frequency: the lowest tone where the reference puts it.
const DEFAULT_AUDIO_HZ: f32 = jtty::JTTY_BASE_HZ;
/// The band the reference always searches for the lowest tone, Hz.
const WIDE_LO_HZ: f32 = 1200.0;
const WIDE_HI_HZ: f32 = 1800.0;
/// How far either side of the operator's own frequency is searched, Hz.
const FTOL_HZ: f32 = 50.0;
/// Two decodes this close in frequency are the same station.
const SAME_STATION_HZ: f32 = 2.0 * jtty::JTTY_TONE_SPACING_HZ;
/// Silence (6 kHz samples) pushed after a transmit burst to flush the
/// resampler: one of its 1024-sample input chunks, so nothing real is left
/// behind in it.
const TX_RESAMPLE_PAD: usize = 1024;

/// One stretch of audio handed to the decode worker.
struct DecodeJob {
    /// Which receive this belongs to; a result from before an abort is dropped.
    generation: u64,
    audio: Vec<f32>,
    /// Absolute sample index of `audio[0]`.
    base: u64,
    /// Frame starts to search, as indices into `audio`.
    from: usize,
    to: usize,
    f_lo: f32,
    f_hi: f32,
}

/// What the worker found in a job.
struct DecodeResult {
    generation: u64,
    base: u64,
    /// Absolute sample index the search reached: every start before it has
    /// been looked at.
    searched_to: u64,
    frames: Vec<jtty::JttyDecoded>,
}

/// A message being received: the frames so far, and where the next is due.
struct Run {
    id: u64,
    frames: Vec<jtty::JttyFrame>,
    /// Absolute sample index the next frame would start at.
    next: u64,
    hz: f32,
}

pub struct JttyController {
    cfg: DigiConfig,
    resampler: Option<MonoResampler>,
    /// Received audio at 6 kHz; `buf[0]` is absolute sample `buf_at`.
    buf: Vec<f32>,
    buf_at: u64,
    /// The first frame start not yet searched, absolute.
    next_search: u64,
    tap_scratch: Vec<f32>,
    level: f32,
    /// The log, newest last, each line keyed by its run.
    log: Vec<(u64, JttyMessage)>,
    next_id: u64,
    total: u64,
    /// Runs still waiting for their next frame.
    runs: Vec<Run>,
    /// Frames already filed (absolute start, Hz): the searches overlap at
    /// their edges, so a frame can be found twice.
    recent: VecDeque<(u64, f32)>,
    /// The operator's audio frequency: the lowest tone, transmit and receive.
    audio_hz: f32,
    generation: u64,

    job_tx: Sender<DecodeJob>,
    res_rx: Receiver<DecodeResult>,
    _worker: std::thread::JoinHandle<()>,
    pending: bool,
    status_dirty: bool,

    // --- Transmit ---
    /// The message box, as typed.
    tx_text: String,
    /// The whole message as 48 kHz audio, ready to play once.
    tx_audio: Vec<f32>,
    /// Where the one-shot transmit has reached in [`Self::tx_audio`].
    tx_pos: usize,
    /// The operator is holding transmit (or a one-shot is being sent).
    tx_active: bool,
    /// Currently keyed.
    keyed: bool,
}

impl JttyController {
    pub fn new(cfg: DigiConfig, tap_rate: f64) -> Self {
        let (job_tx, job_rx) = std::sync::mpsc::channel::<DecodeJob>();
        let (res_tx, res_rx) = std::sync::mpsc::channel::<DecodeResult>();
        let worker = std::thread::Builder::new()
            .name("sdroxide-jtty-decode".into())
            .spawn(move || {
                while let Ok(job) = job_rx.recv() {
                    let frames =
                        jtty::decode_frames(&job.audio, job.from, job.to, job.f_lo, job.f_hi);
                    let result = DecodeResult {
                        generation: job.generation,
                        base: job.base,
                        searched_to: job.base + job.to as u64,
                        frames,
                    };
                    if res_tx.send(result).is_err() {
                        break;
                    }
                }
            })
            .expect("spawn jtty decode worker");

        JttyController {
            cfg,
            resampler: MonoResampler::new(tap_rate, JTTY_RATE),
            buf: Vec::new(),
            buf_at: 0,
            next_search: 0,
            tap_scratch: Vec::new(),
            level: 0.0,
            log: Vec::new(),
            next_id: 0,
            total: 0,
            runs: Vec::new(),
            recent: VecDeque::new(),
            audio_hz: DEFAULT_AUDIO_HZ,
            generation: 0,
            job_tx,
            res_rx,
            _worker: worker,
            pending: false,
            status_dirty: true,
            tx_text: String::new(),
            tx_audio: Vec::new(),
            tx_pos: 0,
            tx_active: false,
            keyed: false,
        }
    }

    /// Whether there is a message to send.
    fn pass_ready(&self) -> bool {
        !self.tx_audio.is_empty()
    }

    /// Rebuild the one-shot transmit audio from the current text: pack the
    /// message to frames, synthesize each frame at the modem's rate, and
    /// resample the whole burst to the 48 kHz the transmit seam hands the
    /// engine. JTTY is asynchronous, so there is no repeat and no slot; a press
    /// sends the message once.
    fn rebuild_tx_audio(&mut self) {
        self.tx_audio.clear();
        self.tx_pos = 0;
        let Some(frames) = sdroxide_dsp::jtty::pack_message(&self.tx_text) else {
            return;
        };
        let f0 = self.audio_hz;
        // The whole message is one waveform, as the reference sends it: one
        // continuous phase across the frames and a ramp only at either end.
        let mut tones = Vec::with_capacity(frames.len() * jtty::JTTY_FRAME_SYMBOLS);
        for p in &frames {
            tones.extend_from_slice(&jtty::JTTY_SYNC);
            tones.extend_from_slice(&jtty::encode_tones(p));
        }
        let mut at6 = jtty::synthesize_frame(&tones, f0);
        // Resample the 6 kHz burst to the 48 kHz the transmit seam expects,
        // through a resampler of its own each time. The resampler works in
        // whole chunks and holds the remainder back, so a shared one kept the
        // last part of the final frame for the *next* rebuild — the burst lost
        // its tail and the next message began with the previous one's. A
        // chunk of silence pushes the whole burst through; what the padding
        // adds past the burst's own length is trimmed off again.
        let want = at6.len() * 8;
        let mut out48 = Vec::with_capacity(want + 8 * TX_RESAMPLE_PAD);
        match MonoResampler::new(JTTY_RATE, 48_000.0) {
            Some(mut r) => {
                at6.extend(std::iter::repeat_n(0.0, TX_RESAMPLE_PAD));
                r.push(&at6, &mut out48);
                out48.truncate(want);
            }
            None => out48 = at6,
        }
        self.tx_audio = out48;
    }

    fn window_samples() -> usize {
        (WINDOW_S * JTTY_RATE) as usize
    }

    fn step_samples() -> usize {
        (SCAN_STEP_S * JTTY_RATE) as usize
    }

    /// The lowest-tone frequencies searched: the reference's band and the
    /// operator's own frequency either side, as one range.
    fn search_range(&self) -> (f32, f32) {
        let lo = WIDE_LO_HZ.min(self.audio_hz - FTOL_HZ).max(100.0);
        let hi = WIDE_HI_HZ.max(self.audio_hz + FTOL_HZ).min(2800.0);
        (lo, hi)
    }

    /// Hand the worker every start not yet searched that has a whole frame of
    /// audio after it, once half a second of them has built up.
    fn schedule_search(&mut self) {
        let step = jtty::JTTY_SEARCH_STEP as u64;
        let written = self.buf_at + self.buf.len() as u64;
        // Fell behind the buffer (a stalled worker): pick up where audio is.
        let from = self.next_search.max(self.buf_at + EDGE as u64).div_ceil(step) * step;
        let Some(last) = written.checked_sub((FRAME_LEN + EDGE) as u64) else {
            return;
        };
        let to = last / step * step;
        if to < from + Self::step_samples() as u64 {
            return;
        }
        let start = from - EDGE as u64;
        let audio = self.buf[(start - self.buf_at) as usize..].to_vec();
        let (f_lo, f_hi) = self.search_range();
        let job = DecodeJob {
            generation: self.generation,
            audio,
            base: start,
            from: EDGE,
            to: (to - start) as usize,
            f_lo,
            f_hi,
        };
        if self.job_tx.send(job).is_ok() {
            self.pending = true;
            self.next_search = to;
        }
    }

    /// File one decoded frame: drop it if it is one already filed, add it to
    /// the run it continues, or start a new message with it.
    fn file_frame(&mut self, at: u64, found: jtty::JttyDecoded, now: SystemTime) {
        let nsps = jtty::JTTY_NSPS as u64;
        let hz = found.f0;
        if self
            .recent
            .iter()
            .any(|&(a, h)| a.abs_diff(at) < nsps && (h - hz).abs() < SAME_STATION_HZ)
        {
            return;
        }
        self.recent.push_back((at, hz));
        while self.recent.len() > 256 {
            self.recent.pop_front();
        }

        // A frame continues a run when it starts where that run's next frame
        // was due, at the same frequency.
        if let Some(i) = self
            .runs
            .iter()
            .position(|r| r.next.abs_diff(at) <= nsps / 2 && (r.hz - hz).abs() < SAME_STATION_HZ)
        {
            let mut frames = self.runs[i].frames.clone();
            frames.push(found.frame.clone());
            if let Some(msg) = jtty::decode_source(&frames) {
                let run = &mut self.runs[i];
                run.frames = frames;
                run.next = at + FRAME_LEN as u64;
                let id = run.id;
                if msg.complete {
                    self.runs.remove(i);
                }
                if let Some((_, line)) = self.log.iter_mut().rev().find(|(k, _)| *k == id) {
                    line.text = msg.text;
                    line.complete = msg.complete;
                    line.at_unix = now_unix(now);
                }
                self.status_dirty = true;
                return;
            }
            // It does not read on the end of that run: the run is over, and
            // this frame may be the start of something else.
            self.runs.remove(i);
        }

        let Some(msg) = jtty::decode_source(std::slice::from_ref(&found.frame)) else {
            return;
        };
        let id = self.next_id;
        self.next_id += 1;
        self.log.push((
            id,
            JttyMessage {
                at_unix: now_unix(now),
                text: msg.text,
                audio_hz: hz,
                snr_db: found.snr_db.round() as i16,
                complete: msg.complete,
            },
        ));
        if self.log.len() > JTTY_MESSAGE_MAX {
            let excess = self.log.len() - JTTY_MESSAGE_MAX;
            self.log.drain(..excess);
        }
        self.total += 1;
        if !msg.complete {
            self.runs.push(Run { id, frames: vec![found.frame], next: at + FRAME_LEN as u64, hz });
        }
        self.status_dirty = true;
    }

    fn take_result(&mut self, res: DecodeResult, now: SystemTime) {
        self.pending = false;
        if res.generation != self.generation {
            return;
        }
        for found in res.frames {
            let at = res.base + found.start as u64;
            self.file_frame(at, found, now);
        }
        // A run whose next frame was due inside what has now been searched,
        // and did not come, is over: the line stays, marked incomplete.
        let nsps = jtty::JTTY_NSPS as u64;
        self.runs.retain(|r| r.next + nsps > res.searched_to);
    }

    fn digi_status(&self) -> DigiStatus {
        let mut s = DigiStatus::idle(self.cfg.clone());
        s.mode = Mode::Jtty;
        s.step = QsoStep::Idle;
        s.audio_hz = self.audio_hz;
        // What the panel's SENDING chip and everything else watching the
        // status read: without it a burst on the air reported idle.
        s.transmitting = self.keyed;
        s.jtty = Some(JttyStatus {
            level: self.level,
            messages: self.log.iter().map(|(_, m)| m.clone()).collect(),
            total: self.total,
        });
        s
    }
}

fn now_unix(now: SystemTime) -> i64 {
    now.duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

impl DigiEngine for JttyController {
    fn mode(&self) -> Mode {
        Mode::Jtty
    }

    fn on_rx_audio(&mut self, tap: &[f32]) {
        self.tap_scratch.clear();
        match &mut self.resampler {
            Some(r) => r.push(tap, &mut self.tap_scratch),
            None => self.tap_scratch.extend_from_slice(tap),
        }
        self.buf.extend_from_slice(&self.tap_scratch);
        // Trimmed back to the window only once it has doubled, so the move
        // happens every twelve seconds rather than on every audio block.
        let cap = Self::window_samples();
        if self.buf.len() > 2 * cap {
            let excess = self.buf.len() - cap;
            self.buf.drain(..excess);
            self.buf_at += excess as u64;
        }

        if !tap.is_empty() {
            let energy = tap.iter().map(|s| s * s).sum::<f32>() / tap.len() as f32;
            self.level = 0.9 * self.level + 0.1 * energy.sqrt();
        }
    }

    fn poll(&mut self, now: SystemTime, _dial_hz: f64) -> Vec<DigiAction> {
        let mut actions = Vec::new();

        while let Ok(res) = self.res_rx.try_recv() {
            self.take_result(res, now);
        }
        if !self.pending {
            self.schedule_search();
        }

        // Key on the operator's transmit. JTTY is asynchronous: a press sends
        // the message once, in one burst, and then unkeys — there is no slot to
        // key on and nothing repeats.
        //
        // A key with nothing in the box is refused *and dropped*: leaving
        // `tx_active` set would arm the mode, and the next keystroke would key
        // the radio unexpectedly. Sending takes a fresh press.
        if self.tx_active && !self.keyed {
            if self.pass_ready() {
                self.keyed = true;
                self.status_dirty = true;
                actions.push(DigiAction::KeyTx);
            } else {
                self.tx_active = false;
                self.status_dirty = true;
            }
        }

        if self.status_dirty {
            self.status_dirty = false;
            actions.push(DigiAction::Status(self.digi_status()));
        }
        actions
    }

    fn tx_burst_active(&self) -> bool {
        self.keyed
    }

    /// Play the one-shot transmit burst. The audio is governed by the
    /// synthesized message, not by how long transmit is held: JTTY sends the
    /// message once, so this returns true (the burst is done) as soon as the
    /// queue is drained.
    fn fill_tx_block(&mut self, out: &mut [f32]) -> bool {
        for s in out.iter_mut() {
            if self.tx_pos < self.tx_audio.len() {
                *s = self.tx_audio[self.tx_pos];
                self.tx_pos += 1;
            } else {
                *s = 0.0;
            }
        }
        self.tx_pos >= self.tx_audio.len()
    }

    /// The synthesized audio is full-scale, so say so: the engine scales by
    /// `1/peak`, and the default 0.5 would double it into the limiter.
    fn tx_peak(&self) -> f32 {
        1.0
    }

    fn on_burst_done(&mut self) {
        self.keyed = false;
        self.tx_active = false;
        self.tx_pos = 0;
        self.status_dirty = true;
    }

    fn abort(&mut self) {
        self.buf_at += self.buf.len() as u64;
        self.buf.clear();
        self.next_search = self.buf_at;
        self.runs.clear();
        self.generation += 1;
        self.status_dirty = true;
    }

    fn abort_tx(&mut self) {
        self.keyed = false;
        self.tx_active = false;
        self.tx_pos = 0;
        self.status_dirty = true;
    }

    fn set_tx_text(&mut self, text: String) {
        self.tx_text = text;
        // Rebuild the burst only while nothing is keyed: an over on the air
        // keeps the message it started with.
        if !self.keyed {
            self.rebuild_tx_audio();
        }
        self.status_dirty = true;
    }

    fn set_tx_active(&mut self, on: bool) {
        self.tx_active = on;
        self.status_dirty = true;
    }

    fn set_config(&mut self, cfg: DigiConfig) {
        self.cfg = cfg;
        self.status_dirty = true;
    }

    fn clear_rx(&mut self) {
        self.log.clear();
        self.runs.clear();
        self.recent.clear();
        self.total = 0;
        self.status_dirty = true;
    }

    /// The lowest tone, as the reference's own frequency is: the transmit
    /// sits there and the receiver searches either side of it as well as its
    /// fixed band. Clamped so the four tones stay inside an SSB passband.
    fn set_audio_hz(&mut self, hz: f32) {
        let hz = hz.clamp(200.0, 2800.0);
        if hz != self.audio_hz {
            self.audio_hz = hz;
            if !self.keyed {
                self.rebuild_tx_audio();
            }
            self.status_dirty = true;
        }
    }

    fn audio_hz(&self) -> f32 {
        self.audio_hz
    }

    fn status(&self) -> DigiStatus {
        self.digi_status()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic white noise at `amp`.
    fn noise(len: usize, amp: f32, seed: u32) -> Vec<f32> {
        let mut state = seed.wrapping_mul(0x9e37_79b9) | 1;
        (0..len)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                ((state >> 8) as f32 / 8_388_608.0 - 1.0) * amp
            })
            .collect()
    }

    /// The 48 kHz burst a controller hands the engine for `text` sent at
    /// `hz`, keyed the way the engine keys it.
    fn tx_burst(text: &str, hz: f32) -> Vec<f32> {
        let mut c = JttyController::new(DigiConfig::default(), 48_000.0);
        c.set_audio_hz(hz);
        c.set_tx_text(text.into());
        c.set_tx_active(true);
        let keyed =
            c.poll(SystemTime::now(), 0.0).into_iter().any(|a| matches!(a, DigiAction::KeyTx));
        assert!(keyed, "{text:?} should key the radio");
        let mut out = Vec::new();
        let mut block = vec![0.0f32; 4800];
        loop {
            let done = c.fill_tx_block(&mut block);
            out.extend_from_slice(&block);
            if done {
                break;
            }
        }
        out
    }

    /// Play 48 kHz audio into a receiver in 0.1 s blocks, polling as the engine
    /// does and letting each search finish, so the result is deterministic.
    fn receive(c: &mut JttyController, audio: &[f32]) {
        for block in audio.chunks(4800) {
            c.on_rx_audio(block);
            c.poll(SystemTime::now(), 0.0);
            while c.pending {
                std::thread::sleep(std::time::Duration::from_millis(1));
                c.poll(SystemTime::now(), 0.0);
            }
        }
    }

    /// Air: `secs` of noise with each burst mixed in at its start time.
    fn air(secs: f32, bursts: &[(f32, &[f32])]) -> Vec<f32> {
        let mut out = noise((secs * 48_000.0) as usize, 0.5, 11);
        for &(at, b) in bursts {
            let i0 = (at * 48_000.0) as usize;
            for (d, s) in out[i0..].iter_mut().zip(b) {
                *d += s;
            }
        }
        out
    }

    fn lines(c: &JttyController) -> Vec<JttyMessage> {
        c.digi_status().jtty.unwrap().messages
    }

    /// A message of many frames is one line, filed once and completed — not
    /// one line per frame seen and per re-scan, and not cut off where the
    /// buffer ends: this one is longer than the whole buffer.
    #[test]
    fn a_long_message_is_one_complete_line() {
        let text = "CQ W1ABC W1ABC FN42 THIS IS A LONGER MESSAGE";
        let burst = tx_burst(text, 1500.0);
        assert!(burst.len() as f64 > WINDOW_S * 48_000.0, "{} s", burst.len() / 48_000);
        let mut c = JttyController::new(DigiConfig::default(), 48_000.0);
        receive(&mut c, &air(burst.len() as f32 / 48_000.0 + 6.0, &[(1.3, &burst)]));
        let got = lines(&c);
        assert_eq!(got.len(), 1, "{got:?}");
        assert!(got[0].complete);
        assert_eq!(got[0].text, text);
        assert!((got[0].audio_hz - 1500.0).abs() < 8.0, "{}", got[0].audio_hz);
    }

    /// Two stations sending at once, a few hundred hertz apart, are two lines:
    /// a frame is filed against the run at its own frequency, never joined
    /// onto whichever came before it in time.
    #[test]
    fn two_stations_at_once_are_two_lines() {
        let a = tx_burst("CQ K1ABC CQ", 1320.0);
        let b = tx_burst("W9XYZ 599 123", 1660.0);
        let mut c = JttyController::new(DigiConfig::default(), 48_000.0);
        receive(&mut c, &air(10.0, &[(0.7, &a), (1.6, &b)]));
        let mut got: Vec<_> = lines(&c).into_iter().map(|m| (m.text, m.complete)).collect();
        got.sort();
        assert_eq!(
            got,
            vec![("CQ K1ABC CQ".to_string(), true), ("W9XYZ 599 123".to_string(), true)]
        );
    }

    /// A station off the reference band is heard once the operator's own
    /// frequency is set near it, and transmit goes out there.
    #[test]
    fn the_operator_frequency_is_searched_and_sent_on() {
        let burst = tx_burst("CQ K1ABC CQ", 2210.0);
        let power_at = |sig: &[f32], hz: f32| -> f64 {
            let w = std::f64::consts::TAU * hz as f64 / 48_000.0;
            let (mut re, mut im) = (0.0f64, 0.0f64);
            for (k, &s) in sig.iter().enumerate() {
                let ph = w * k as f64;
                re += s as f64 * ph.cos();
                im += s as f64 * ph.sin();
            }
            (re * re + im * im) / sig.len() as f64
        };
        // Probed at literal frequencies, not at the constant under test.
        assert!(power_at(&burst, 2210.0) > 10.0 * power_at(&burst, 1500.0));

        let on_air = air(6.0, &[(0.5, &burst)]);
        let mut wide = JttyController::new(DigiConfig::default(), 48_000.0);
        receive(&mut wide, &on_air);
        assert!(lines(&wide).is_empty(), "2210 Hz is outside 1500 ± 50 and 1200–1800");

        let mut tuned = JttyController::new(DigiConfig::default(), 48_000.0);
        tuned.set_audio_hz(2190.0);
        receive(&mut tuned, &on_air);
        let got = lines(&tuned);
        assert_eq!(got.len(), 1, "{got:?}");
        assert_eq!(got[0].text, "CQ K1ABC CQ");
    }

    /// The transmit chain end to end: the burst the controller hands the engine,
    /// resampled down the receive path, reads back as the message typed.
    #[test]
    fn a_transmitted_message_decodes_back() {
        let burst = tx_burst("W1ABC W9XYZ FN42", 1500.0);
        let mut c = JttyController::new(DigiConfig::default(), 48_000.0);
        receive(&mut c, &air(8.0, &[(0.2, &burst)]));
        let got = lines(&c);
        assert_eq!(got.len(), 1, "{got:?}");
        assert_eq!(got[0].text, "W1ABC W9XYZ FN42");
        assert!(got[0].complete);
    }

    /// Noise alone files nothing.
    #[test]
    fn noise_alone_files_nothing() {
        let mut c = JttyController::new(DigiConfig::default(), 48_000.0);
        receive(&mut c, &noise(30 * 48_000, 1.0, 5));
        assert!(lines(&c).is_empty(), "{:?}", lines(&c));
    }

    /// An empty box is refused and dropped: nothing keys, and a later keystroke
    /// does not key either without a fresh press.
    #[test]
    fn an_empty_message_does_not_key() {
        let mut c = JttyController::new(DigiConfig::default(), 48_000.0);
        c.set_tx_active(true);
        let keyed =
            c.poll(SystemTime::now(), 0.0).into_iter().any(|a| matches!(a, DigiAction::KeyTx));
        assert!(!keyed, "an empty box must not key");
        assert!(!c.tx_burst_active());
        // A keystroke after a refused press still does not key.
        c.set_tx_text("W1ABC".into());
        let keyed =
            c.poll(SystemTime::now(), 0.0).into_iter().any(|a| matches!(a, DigiAction::KeyTx));
        assert!(!keyed, "typing into a refused box must not key");
        // A fresh press keys with the text present.
        c.set_tx_active(true);
        let keyed =
            c.poll(SystemTime::now(), 0.0).into_iter().any(|a| matches!(a, DigiAction::KeyTx));
        assert!(keyed, "a fresh press must key");
    }
}
