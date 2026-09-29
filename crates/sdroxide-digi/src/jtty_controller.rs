//! `JttyController` — the JTTY asynchronous text receiver, receive only.
//!
//! JTTY is not a slotted mode: a transmission can start at any instant, so
//! there is no T/R period and no frame boundary to lock to. Like
//! [`crate::UvPacketController`] and the ACARS/DSC controllers, this holds a
//! rolling window of audio and re-scans it as it advances, then keeps a
//! short-lived set of decoded-message hashes so an overlapping window does not
//! file the same message twice.
//!
//! # Messages, not frames
//!
//! A JTTY **message** is one or more ~1.888 s **frames**; the last carries the
//! end-of-message flag. The receiver searches the window for a frame, decodes
//! it, and appends it to a message until EOM or until the run breaks. What the
//! panel shows is that accumulated text.
//!
//! Receive only in this build. Transmit needs the source-grammar packer and the
//! asynchronous keying path, which is the next stage — as FSK441 shipped
//! receive before transmit.

use std::collections::VecDeque;
use std::hash::{Hash, Hasher};
use std::sync::mpsc::{Receiver, Sender};
use std::time::SystemTime;

use sdroxide_dsp::MonoResampler;
use sdroxide_dsp::jtty::{self, JTTY_RATE};
use sdroxide_types::{
    DigiConfig, DigiStatus, JTTY_MESSAGE_MAX, JttyMessage, JttyStatus, Mode, QsoStep,
};

use crate::DigiEngine;
use crate::controller::DigiAction;

/// Audio window scanned, in seconds. Long enough for a several-frame message.
const WINDOW_S: f64 = 12.0;
/// How much new audio accumulates before the window is scanned again.
const SCAN_STEP_S: f64 = 0.5;
/// How long a decoded-message hash suppresses a repeat, in seconds.
const DEDUP_TTL_S: u64 = 15;

/// One window handed to the decode worker.
struct DecodeJob {
    audio: Vec<f32>,
    at: i64,
}

pub struct JttyController {
    cfg: DigiConfig,
    resampler: Option<MonoResampler>,
    buf: Vec<f32>,
    tap_scratch: Vec<f32>,
    level: f32,
    messages: Vec<JttyMessage>,
    total: u64,
    seen: VecDeque<(u64, SystemTime)>,
    since_scan: usize,

    job_tx: Sender<DecodeJob>,
    res_rx: Receiver<Vec<JttyMessage>>,
    _worker: std::thread::JoinHandle<()>,
    pending: bool,
    status_dirty: bool,

    // --- Transmit ---
    /// The message box, as typed.
    tx_text: String,
    /// The whole message as 48 kHz audio, ready to play once.
    tx_audio: Vec<f32>,
    /// Resamples the synthesized 6 kHz burst up to 48 kHz on key.
    tx_rs: Option<MonoResampler>,
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
        let (res_tx, res_rx) = std::sync::mpsc::channel::<Vec<JttyMessage>>();
        let worker = std::thread::Builder::new()
            .name("sdroxide-jtty-decode".into())
            .spawn(move || {
                while let Ok(job) = job_rx.recv() {
                    let msgs = decode_window(&job.audio, job.at);
                    if res_tx.send(msgs).is_err() {
                        break;
                    }
                }
            })
            .expect("spawn jtty decode worker");

        JttyController {
            cfg,
            resampler: MonoResampler::new(tap_rate, JTTY_RATE),
            tx_rs: MonoResampler::new(JTTY_RATE, 48_000.0),
            buf: Vec::new(),
            tap_scratch: Vec::new(),
            level: 0.0,
            messages: Vec::new(),
            total: 0,
            seen: VecDeque::new(),
            since_scan: 0,
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
        // The modem synthesizes at the tone-spacing base, like the receiver
        // expects.
        let f0 = sdroxide_dsp::jtty::JTTY_TONE_SPACING_HZ;
        let mut at6 = Vec::new();
        for p in &frames {
            let coded = sdroxide_dsp::jtty::encode_tones(p);
            let mut tones = Vec::with_capacity(
                sdroxide_dsp::jtty::JTTY_SYNC_SYMBOLS + sdroxide_dsp::jtty::INFORMATION_BITS,
            );
            tones.extend_from_slice(&sdroxide_dsp::jtty::JTTY_SYNC);
            tones.extend_from_slice(&coded);
            at6.extend_from_slice(&sdroxide_dsp::jtty::synthesize_frame(&tones, f0));
        }
        // Resample the 6 kHz burst to the 48 kHz the transmit seam expects.
        let mut out48 = Vec::new();
        match &mut self.tx_rs {
            Some(r) => r.push(&at6, &mut out48),
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
        s.mode = Mode::Jtty;
        s.step = QsoStep::Idle;
        s.jtty = Some(JttyStatus {
            level: self.level,
            messages: self.messages.clone(),
            total: self.total,
        });
        s
    }
}

/// Hash over the identifying content of a message, for the recent-seen set.
fn message_hash(m: &JttyMessage) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    m.text.hash(&mut h);
    h.finish()
}

fn now_unix(now: SystemTime) -> i64 {
    now.duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// Scan a window of 6 kHz audio for JTTY frames and assemble messages.
///
/// A JTTY frame is 59 symbols of 4-GFSK with a known 13-symbol sync, so the
/// window is swept a symbol at a time; at each candidate the sync is scored and,
/// when it is strong enough, the following 46 symbols are decoded. Candidate
/// frames are collected with their absolute positions and sorted, then
/// assembled: consecutive frames accumulate into one message until one carries
/// the end-of-message flag. This slide-and-collect form is robust to frames
/// touching each other (a message is several frames back to back) because it
/// never has to guess where the next frame begins.
pub fn decode_window(x: &[f32], at: i64) -> Vec<JttyMessage> {
    let nsps = jtty::JTTY_NSPS;
    let slot = (jtty::JTTY_SYNC_SYMBOLS + jtty::INFORMATION_BITS) * nsps;
    if x.len() < slot {
        return Vec::new();
    }

    // Collect decoded frames with their start positions.
    let mut found: Vec<(usize, jtty::JttyFrame, f32, f32)> = Vec::new();
    let mut pos = 0usize;
    while pos + slot <= x.len() {
        // Find the EARLIEST acceptable sync at or after `pos`, at the mode's
        // fixed audio base. `find_sync_near` scans ascending and returns the
        // first candidate, so a stronger frame later in the buffer cannot make
        // the scanner skip an earlier one.
        let Some(sync) = jtty::find_sync_near(x, jtty::JTTY_TONE_SPACING_HZ, pos, x.len(), 6)
        else {
            break;
        };
        let abs = sync.start;
        let energies = jtty::payload_energies(x, &sync);
        match jtty::decode_tones(&energies, 32, 2) {
            Some(frame) => {
                found.push((abs, frame, sync.f0, sync.snr_db));
                pos = abs + slot;
            }
            None => {
                // Not a frame here; step past the sync start and keep looking.
                pos = abs + nsps;
            }
        }
    }

    // Assemble: consecutive frames run into one message until EOM.
    found.sort_by_key(|&(p, _, _, _)| p);
    let mut out = Vec::new();
    let mut acc: Vec<jtty::JttyFrame> = Vec::new();
    let mut first_hz = 0.0f32;
    let mut first_snr = 0.0f32;
    for (_, frame, f0, snr) in found {
        if acc.is_empty() {
            first_hz = f0;
            first_snr = snr;
        }
        let eom = jtty::is_eom(&frame.payload);
        acc.push(frame);
        if eom {
            if let Some(msg) = jtty::decode_source(&acc) {
                out.push(JttyMessage {
                    at_unix: at,
                    text: msg.text,
                    audio_hz: first_hz,
                    snr_db: first_snr.round() as i16,
                    complete: msg.complete,
                });
            }
            acc.clear();
        }
    }
    if !acc.is_empty()
        && let Some(msg) = jtty::decode_source(&acc)
    {
        out.push(JttyMessage {
            at_unix: at,
            text: msg.text,
            audio_hz: first_hz,
            snr_db: first_snr.round() as i16,
            complete: msg.complete,
        });
    }
    out
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

        while let Ok(msgs) = self.res_rx.try_recv() {
            self.pending = false;
            for m in msgs {
                let hash = message_hash(&m);
                if !self.seen_recently(hash, now) {
                    self.seen.push_back((hash, now));
                    self.messages.push(m);
                    self.total += 1;
                    self.status_dirty = true;
                }
            }
            if self.messages.len() > JTTY_MESSAGE_MAX {
                let excess = self.messages.len() - JTTY_MESSAGE_MAX;
                self.messages.drain(..excess);
            }
            self.prune_seen(now);
        }

        if !self.pending && self.since_scan >= Self::step_samples() {
            self.since_scan = 0;
            let audio = self.buf.clone();
            self.pending = true;
            let _ = self.job_tx.send(DecodeJob { audio, at: now_unix(now) });
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
        self.buf.clear();
        self.since_scan = 0;
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
        self.messages.clear();
        self.total = 0;
        self.seen.clear();
        self.status_dirty = true;
    }

    fn set_audio_hz(&mut self, _hz: f32) {}

    /// Fixed by the mode: the reference demodulates a candidate-centred
    /// baseband, so there is no operator tone to set.
    fn audio_hz(&self) -> f32 {
        1500.0
    }

    fn status(&self) -> DigiStatus {
        self.digi_status()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A synthesized two-frame message decodes through the controller's window
    /// scanner: frame one non-EOM, frame two EOM, accumulated into one message.
    #[test]
    fn a_synthesized_message_is_assembled() {
        use sdroxide_dsp::jtty::{JTTY_SYNC, encode_tones, synthesize_frame};
        // `CQ K1ABC CQ` is one atom; send it twice with EOM only on the second
        // to exercise accumulation. Use the spec's CQ vector.
        let cq = {
            let bits = "0000100110111101111000110101000001";
            let mut p = [0u8; jtty::PAYLOAD_BITS];
            for (i, c) in bits.chars().enumerate() {
                p[i] = c.to_digit(2).unwrap() as u8;
            }
            p
        };
        // Payload bit 34 is the EOM flag: clear it on the first frame, set it
        // on the second so the two accumulate into one message.
        let mut first = cq;
        first[33] = 0;
        let mut last = cq;
        last[33] = 1;

        let mut frame_tones = Vec::new();
        frame_tones.extend_from_slice(&JTTY_SYNC);
        frame_tones.extend_from_slice(&encode_tones(&first));
        frame_tones.extend_from_slice(&JTTY_SYNC);
        frame_tones.extend_from_slice(&encode_tones(&last));

        let audio = synthesize_frame(&frame_tones, jtty::JTTY_TONE_SPACING_HZ);
        let mut buf = vec![0.0f32; jtty::JTTY_NSPS * 4];
        buf.extend_from_slice(&audio);
        buf.extend(std::iter::repeat_n(0.0, jtty::JTTY_NSPS * 8));

        let msgs = decode_window(&buf, 0);
        assert!(!msgs.is_empty(), "no message decoded");
        assert!(msgs.iter().any(|m| m.text.contains("K1ABC")), "{msgs:?}");
    }

    /// The whole transmit chain in one test: set the text, key, play the burst
    /// the controller hands the engine, and decode it back through the
    /// receiver. This is the loop an operator actually makes.
    #[test]
    fn a_transmitted_message_decodes_back() {
        use sdroxide_dsp::jtty::JTTY_RATE;
        let mut c = JttyController::new(DigiConfig::default(), 48_000.0);
        c.set_tx_text("W1ABC W9XYZ FN42".into());
        c.set_tx_active(true);
        // The first poll keys the radio.
        let keyed =
            c.poll(SystemTime::now(), 0.0).into_iter().any(|a| matches!(a, DigiAction::KeyTx));
        assert!(keyed, "a message should key the radio");
        assert!(c.tx_burst_active());

        // Play the whole burst the controller hands the engine, at 48 kHz.
        let mut tx48 = Vec::new();
        let mut block = vec![0.0f32; 48_000];
        loop {
            let done = c.fill_tx_block(&mut block);
            tx48.extend_from_slice(&block);
            if done {
                break;
            }
        }
        // Down to the decoder's rate with the same resampler the receive path
        // uses, then decode the loop the operator actually makes: transmit,
        // resample both ways, and read the message back off the air.
        let mut at6 = Vec::new();
        let mut down = MonoResampler::new(48_000.0, JTTY_RATE).expect("resampler");
        down.push(&tx48, &mut at6);
        let mut buf = vec![0.0f32; sdroxide_dsp::jtty::JTTY_NSPS * 4];
        buf.extend_from_slice(&at6);
        buf.extend(std::iter::repeat_n(0.0, sdroxide_dsp::jtty::JTTY_NSPS * 4));
        let msgs = decode_window(&buf, 0);
        assert!(
            msgs.iter().any(|m| m.text == "W1ABC W9XYZ FN42"),
            "the transmitted message did not decode: {msgs:?}"
        );
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
