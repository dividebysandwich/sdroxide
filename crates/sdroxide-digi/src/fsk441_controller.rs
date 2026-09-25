//! `Fsk441Controller` — FSK441, the original meteor-scatter mode, receive and
//! transmit.
//!
//! Slotted, and shaped like the other receive-only decoders here, with two
//! differences that matter:
//!
//! * **The rate.** FSK441's constants — 25 samples a dit at 441 baud — are the
//!   mode, and they only line up at 11 025 Hz. The tap is 48 kHz, so this
//!   resamples to [`sdroxide_dsp::FSK441_RATE`] rather than the 12 kHz the
//!   mfsk-core modes use, and the slot buffer is f32 at that rate.
//! * **The period** is an operator setting ([`Fsk441Period`]), not a property
//!   of the mode, so the slot length follows it.
//!
//! # Transmitting
//!
//! FSK441 has no frame at a fixed offset: an operator keys and **sends the
//! message over and over** through the period, and a meteor's brief trail
//! catches whatever part of it happens to be passing. So transmit is not a
//! one-shot burst — it is the encoded message looping for as long as the
//! operator holds transmit, exactly as the mode is worked on the air. The
//! keyboard seam drives it: [`DigiEngine::set_tx_text`] takes the message,
//! [`DigiEngine::set_tx_active`] keys and unkeys, and [`DigiEngine::fill_tx_block`]
//! loops the audio out.
//!
//! # Why a worker thread
//!
//! A whole-slot ping search is a sliding matched filter over thirty seconds of
//! audio, four tones deep, plus a decode per ping — far from free, and the
//! engine polls the controller on the audio thread. The decode runs on its own
//! thread and the result is drained from [`Self::poll`], one slot in flight at a
//! time, dropping rather than queueing when the machine cannot keep up.

use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::time::SystemTime;

use sdroxide_dsp::{FSK441_RATE, MonoResampler, fsk441_encode_tones, fsk441_generate_audio};
use sdroxide_types::{Decode, DigiConfig, DigiStatus, Fsk441Period, Mode, QsoStep};

use crate::DigiEngine;
use crate::controller::DigiAction;
use crate::modem::decode_fsk441_slot;
use crate::scheduler::SlotScheduler;

/// One slot of audio handed to the decode worker.
struct DecodeJob {
    audio: Vec<f32>,
    slot_utc: i64,
}

/// Samples taken from the transmit pass at a time before it is resampled up.
/// Small enough that a queued block never exceeds what one `fill_tx_block`
/// consumes, large enough to amortise the resampler call.
const FSK441_BLOCK: usize = 256;

/// What the worker sends back.
struct DecodeResult {
    decodes: Vec<Decode>,
}

pub struct Fsk441Controller {
    cfg: DigiConfig,
    /// The period in force, mirroring `cfg.fsk441_period`; re-read whenever the
    /// config changes, because it decides both the slot length and the decode.
    period: Fsk441Period,
    scheduler: SlotScheduler,
    resampler: Option<MonoResampler>,
    /// 11 025 Hz audio accumulated for the slot in progress.
    slot_buf: Vec<f32>,
    /// Whether `slot_buf` began at a slot boundary. It does not after a start,
    /// a reset or a period change part-way through a slot, and a buffer that
    /// began mid-slot is not the slot the decoder is told it is — so such a
    /// slot is dropped rather than decoded.
    buf_aligned: bool,
    tap_scratch: Vec<f32>,
    last_slot_idx: i64,
    audio_hz: f32,

    job_tx: Sender<DecodeJob>,
    res_rx: Receiver<DecodeResult>,
    _worker: std::thread::JoinHandle<()>,
    pending: bool,
    last_count: u32,
    status_dirty: bool,

    /// The message to be sent, and its one-pass audio at [`FSK441_RATE`]. Built
    /// when the text is set rather than per block: the encode is cheap but it is
    /// the same audio every pass, and the loop below only ever cycles a slice.
    tx_text: String,
    tx_pass: Vec<f32>,
    /// Where the repeat has reached in [`Self::tx_pass`].
    tx_pass_pos: usize,
    /// Transmit is on. Held by the operator, as the mode is worked — the message
    /// loops for as long as this stands.
    tx_active: bool,
    /// The operator has asked to transmit and the box had nothing to send, so
    /// the panel can say so rather than looking armed and doing nothing.
    tx_latched: bool,
    /// The radio has been keyed for the over in progress.
    keyed: bool,
    /// A whole over has had text in it, so the end of the pass can unkey.
    over_had_text: bool,
    /// Transmit audio at 48 kHz waiting to go out, and the 11 025 Hz audio not
    /// yet through the resampler.
    tx48: VecDeque<f32>,
    tx_scratch11: Vec<f32>,
    tx_scratch48: Vec<f32>,
    tx_rs: Option<MonoResampler>,
}

impl Fsk441Controller {
    pub fn new(cfg: DigiConfig, tap_rate: f64) -> Self {
        let period = cfg.fsk441_period;
        let (job_tx, job_rx) = std::sync::mpsc::channel::<DecodeJob>();
        let (res_tx, res_rx) = std::sync::mpsc::channel::<DecodeResult>();
        let worker = std::thread::Builder::new()
            .name("sdroxide-fsk441-decode".into())
            .spawn(move || {
                while let Ok(job) = job_rx.recv() {
                    let decodes = decode_fsk441_slot(&job.audio, job.slot_utc);
                    if res_tx.send(DecodeResult { decodes }).is_err() {
                        break;
                    }
                }
            })
            .expect("spawn fsk441 decode worker");

        Fsk441Controller {
            cfg,
            period,
            scheduler: SlotScheduler::new(period.slot_s(), period.start_delay_s()),
            resampler: MonoResampler::new(tap_rate, FSK441_RATE),
            slot_buf: Vec::new(),
            buf_aligned: false,
            tap_scratch: Vec::new(),
            last_slot_idx: i64::MIN,
            // FSK441's tones sit 882–2205 Hz; the centre is where the cursor
            // belongs and what the decode list plots.
            audio_hz: 1543.5,
            job_tx,
            res_rx,
            _worker: worker,
            pending: false,
            last_count: 0,
            status_dirty: true,
            tx_text: String::new(),
            tx_pass: Vec::new(),
            tx_pass_pos: 0,
            tx_active: false,
            tx_latched: false,
            keyed: false,
            over_had_text: false,
            tx48: VecDeque::new(),
            tx_scratch11: Vec::new(),
            tx_scratch48: Vec::new(),
            // The transmit chain always hands the engine 48 kHz, whatever the
            // tap runs at, so the resampler is built on that fixed figure.
            tx_rs: MonoResampler::new(FSK441_RATE, 48_000.0),
        }
    }

    fn slot_samples(&self) -> usize {
        (self.period.slot_s() * FSK441_RATE) as usize
    }

    /// Rebuild the one-pass transmit audio for the current text.
    fn rebuild_tx_pass(&mut self) {
        let text = self.tx_text.trim();
        if text.is_empty() {
            self.tx_pass.clear();
            self.tx_pass_pos = 0;
            return;
        }
        self.tx_pass = fsk441_generate_audio(&fsk441_encode_tones(text));
        self.tx_pass_pos = 0;
    }

    /// Whether there is a message to send. Transmit is refused otherwise, so a
    /// key with an empty box cannot key the radio and sit on an empty carrier.
    fn pass_ready(&self) -> bool {
        !self.tx_pass.is_empty()
    }

    fn digi_status(&self) -> DigiStatus {
        let mut s = DigiStatus::idle(self.cfg.clone());
        s.mode = Mode::Fsk441;
        s.step = QsoStep::Idle;
        s.audio_hz = self.audio_hz;
        s.transmitting = self.keyed;
        s.tx_pending_msg = (!self.tx_text.is_empty()).then(|| self.tx_text.clone());
        // Armed with nothing to send: the panel shows the empty-box reason rather
        // than a key that appears to be doing nothing.
        s.tx_refused =
            (self.tx_active && self.tx_latched).then(|| "type a message to transmit".to_string());
        s
    }
}

impl DigiEngine for Fsk441Controller {
    fn mode(&self) -> Mode {
        Mode::Fsk441
    }

    fn on_rx_audio(&mut self, tap: &[f32]) {
        self.tap_scratch.clear();
        match &mut self.resampler {
            Some(r) => r.push(tap, &mut self.tap_scratch),
            None => self.tap_scratch.extend_from_slice(tap),
        }
        let cap = self.slot_samples() + self.slot_samples() / 8;
        for &s in &self.tap_scratch {
            if self.slot_buf.len() < cap {
                self.slot_buf.push(s.clamp(-1.0, 1.0));
            }
        }
    }

    fn poll(&mut self, now: SystemTime, _dial_hz: f64) -> Vec<DigiAction> {
        let mut actions = Vec::new();

        // Drain the worker.
        loop {
            match self.res_rx.try_recv() {
                Ok(res) => {
                    self.pending = false;
                    self.last_count = res.decodes.len() as u32;
                    self.status_dirty = true;
                    if !res.decodes.is_empty() {
                        actions.push(DigiAction::Decodes(res.decodes));
                    }
                }
                Err(TryRecvError::Empty) => break,
                // The worker is gone and no answer is coming. Release the slot
                // rather than holding `pending` for the rest of the session,
                // which would stop every later slot being dispatched.
                Err(TryRecvError::Disconnected) => {
                    self.pending = false;
                    break;
                }
            }
        }

        // Key on the operator's transmit, as the keyboard modes do. FSK441 is
        // worked by sending the message continuously, so there is no slot
        // boundary to key on — the over lasts as long as the operator holds it.
        //
        // `tx_latched` is the moment the operator asked to transmit, kept even
        // while the box is empty so the panel can say there is nothing to send:
        // the key is refused, and it is not left armed to spring into life on
        // the first keystroke.
        if self.tx_active && !self.keyed {
            if self.pass_ready() {
                self.keyed = true;
                self.over_had_text = true;
                self.status_dirty = true;
                actions.push(DigiAction::KeyTx);
            } else if !self.tx_latched {
                self.tx_latched = true;
                self.status_dirty = true;
            }
        }

        let idx = self.scheduler.slot_index(now);
        if idx != self.last_slot_idx {
            // Only a slot whose audio began on its own boundary is decoded, and
            // half a slot of it is the floor: less than that is a stream
            // hiccup, not a transmission.
            let min_samples = (self.period.slot_s() * FSK441_RATE * 0.5) as usize;
            if self.buf_aligned && self.slot_buf.len() >= min_samples && !self.pending {
                let audio = std::mem::take(&mut self.slot_buf);
                // The slot that just ended is the one before this boundary.
                let slot_utc = self.scheduler.slot_start_unix(idx - 1) as i64;
                self.pending = self.job_tx.send(DecodeJob { audio, slot_utc }).is_ok();
            }
            self.slot_buf.clear();
            // The very first poll is not a boundary crossing, only the first
            // look at the clock — part-way through a slot.
            self.buf_aligned = self.last_slot_idx != i64::MIN;
            self.last_slot_idx = idx;
        }
        if self.status_dirty {
            self.status_dirty = false;
            actions.push(DigiAction::Status(self.digi_status()));
        }
        actions
    }

    /// FSK441's audio is generated at full scale, so `tx_peak` has to say so:
    /// the engine scales by `1/peak`, and the default 0.5 doubled it into the
    /// limiter, flat-topping every over.
    fn tx_peak(&self) -> f32 {
        1.0
    }

    fn tx_burst_active(&self) -> bool {
        self.keyed
    }

    /// Loop the message out while transmit is held.
    ///
    /// A meteor trail catches whatever part of the repeated message is passing
    /// when it appears, so the pass is played again and again with no gap — the
    /// same audio the operator would be sending on the air by hand. Returns true
    /// when there is nothing left to play and transmit can drop, which here is
    /// once the operator has unkeyed and the queued audio has drained.
    fn fill_tx_block(&mut self, out: &mut [f32]) -> bool {
        // Refill the 48 kHz queue a pass-block at a time, but only while
        // transmit is still held: an unkey must stop the repeat, not let it run
        // on until the queue happens to land on a block boundary.
        while self.tx_active && self.tx48.len() < out.len() && !self.tx_pass.is_empty() {
            // The pass repeats, so a block is always taken from it — from the
            // top whenever the last one ran off the end.
            self.tx_scratch11.clear();
            self.tx_scratch11.reserve(FSK441_BLOCK);
            for _ in 0..FSK441_BLOCK {
                if self.tx_pass_pos >= self.tx_pass.len() {
                    self.tx_pass_pos = 0;
                }
                self.tx_scratch11.push(self.tx_pass[self.tx_pass_pos]);
                self.tx_pass_pos += 1;
            }
            self.tx_scratch48.clear();
            match &mut self.tx_rs {
                Some(r) => r.push(&self.tx_scratch11, &mut self.tx_scratch48),
                None => self.tx_scratch48.extend_from_slice(&self.tx_scratch11),
            }
            self.tx48.extend(self.tx_scratch48.iter().copied());
        }
        for s in out.iter_mut() {
            *s = self.tx48.pop_front().unwrap_or(0.0);
        }
        // Done once nothing is left to play and transmit has been released.
        !self.tx_active && self.tx48.is_empty()
    }

    fn on_burst_done(&mut self) {
        self.keyed = false;
        self.status_dirty = true;
    }

    fn abort(&mut self) {
        self.slot_buf.clear();
        self.buf_aligned = false;
        self.status_dirty = true;
    }

    fn abort_tx(&mut self) {
        // A refused key-up must not leave the latch set, or nothing can key
        // again.
        self.keyed = false;
        self.tx_active = false;
        self.tx_latched = false;
        self.tx48.clear();
        self.tx_scratch11.clear();
        self.over_had_text = false;
        self.status_dirty = true;
    }

    fn set_tx_text(&mut self, text: String) {
        self.tx_text = text;
        // A keystroke while armed with an empty box must not restart a running
        // over, so the pass is only rebuilt while nothing is keyed: once the
        // over is on the air it loops the message it started with.
        if !self.keyed {
            self.rebuild_tx_pass();
        }
        if self.pass_ready() {
            self.tx_latched = false;
        }
        self.status_dirty = true;
    }

    fn set_tx_active(&mut self, on: bool) {
        self.tx_active = on;
        if !on {
            self.over_had_text = false;
            self.tx_latched = false;
        }
        self.status_dirty = true;
    }

    fn set_config(&mut self, cfg: DigiConfig) {
        // A period change moves the slot length, so the scheduler, the buffer
        // bound and the decode all have to follow it. Drop the audio in hand:
        // it belongs to the old geometry and decoding it against the new one
        // would look like a signal that is simply not there.
        if cfg.fsk441_period != self.period {
            self.period = cfg.fsk441_period;
            self.scheduler = SlotScheduler::new(self.period.slot_s(), self.period.start_delay_s());
            self.slot_buf.clear();
            self.buf_aligned = false;
            self.last_slot_idx = i64::MIN;
        }
        self.cfg = cfg;
        self.status_dirty = true;
    }

    fn clear_rx(&mut self) {
        self.last_count = 0;
        self.status_dirty = true;
    }

    fn set_audio_hz(&mut self, hz: f32) {
        self.audio_hz = hz.clamp(200.0, 3500.0);
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

    fn cfg() -> DigiConfig {
        DigiConfig::default()
    }

    /// Setting transmit text builds the pass, and keying loops it: the block is
    /// not silent, and it keeps coming for as long as transmit is held.
    #[test]
    fn keying_loops_the_message_until_unkeyed() {
        let mut c = Fsk441Controller::new(cfg(), 48_000.0);
        c.set_tx_text("W1ABC W9XYZ".into());
        c.set_tx_active(true);
        let keyed =
            c.poll(SystemTime::now(), 0.0).into_iter().any(|a| matches!(a, DigiAction::KeyTx));
        assert!(keyed, "transmit text should key the radio");
        assert!(c.tx_burst_active());

        let mut block = vec![0.0f32; 48_000]; // one second
        let done = c.fill_tx_block(&mut block);
        assert!(!done, "a held over must not end on its own");
        assert!(block.iter().any(|s| s.abs() > 0.01), "the block was silent");
        // A second block still has signal — the pass repeats.
        block.iter_mut().for_each(|s| *s = 0.0);
        c.fill_tx_block(&mut block);
        assert!(block.iter().any(|s| s.abs() > 0.01), "the repeat stopped");

        c.set_tx_active(false);
        // The over ends as soon as the queue drains — releasing transmit stops
        // the repeat, it does not wait for a block boundary.
        let done = c.fill_tx_block(&mut block);
        assert!(done, "releasing transmit must end the over once the queue drains");
    }

    /// Releasing transmit ends the over; the message does not keep looping.
    #[test]
    fn releasing_transmit_stops_the_repeat() {
        let mut c = Fsk441Controller::new(cfg(), 48_000.0);
        c.set_tx_text("W1ABC W9XYZ".into());
        c.set_tx_active(true);
        c.poll(SystemTime::now(), 0.0);
        c.set_tx_active(false);
        let mut block = vec![0.0f32; 48_000 * 8];
        assert!(c.fill_tx_block(&mut block), "an unkeyed over must finish");
    }

    /// A key with an empty box must not key the radio and sit on an empty
    /// carrier.
    #[test]
    fn an_empty_message_does_not_key() {
        let mut c = Fsk441Controller::new(cfg(), 48_000.0);
        c.set_tx_active(true);
        let keyed =
            c.poll(SystemTime::now(), 0.0).into_iter().any(|a| matches!(a, DigiAction::KeyTx));
        assert!(!keyed, "nothing to send should not key");
        assert!(!c.tx_burst_active());
        // …and the panel is told why, rather than looking armed and doing
        // nothing.
        assert!(c.digi_status().tx_refused.is_some(), "an empty armed box must say so");

        // The first keystroke with the box still armed keys the radio — the
        // empty arm is not left waiting to spring into life.
        c.set_tx_text("W1ABC".into());
        let keyed =
            c.poll(SystemTime::now(), 0.0).into_iter().any(|a| matches!(a, DigiAction::KeyTx));
        assert!(keyed, "arming with an empty box then typing must key on the text");
        assert!(c.digi_status().tx_refused.is_none());
    }

    /// A keystroke during an over must not restart the message mid-flight.
    #[test]
    fn typing_mid_over_does_not_restart_it() {
        let mut c = Fsk441Controller::new(cfg(), 48_000.0);
        c.set_tx_text("W1ABC W9XYZ".into());
        c.set_tx_active(true);
        c.poll(SystemTime::now(), 0.0);
        let pass_len = c.tx_pass.len();
        c.set_tx_text("DIFFERENT MESSAGE".into());
        assert_eq!(c.tx_pass.len(), pass_len, "the running over must keep its message");
    }

    /// The transmit audio decodes back through the receiver's own decoder once
    /// it is looped — the round trip the two halves have to agree on.
    #[test]
    fn the_transmitted_pass_decodes_as_fsk441() {
        let mut c = Fsk441Controller::new(cfg(), 48_000.0);
        c.set_tx_text("W1ABC W9XYZ FN42".into());
        c.set_tx_active(true);
        c.poll(SystemTime::now(), 0.0);
        // Enough for the decode: at least the pass' own length, in 48 kHz.
        let mut block = vec![0.0f32; 48_000 * 2];
        c.fill_tx_block(&mut block);
        // Back down to the decoder's rate and search it.
        let n11 = (block.len() as f64 * FSK441_RATE / 48_000.0) as usize;
        let mut at11 = Vec::with_capacity(n11);
        let step = 48_000.0 / FSK441_RATE as f32;
        let mut pos = 0.0f32;
        while (pos as usize) < block.len() && at11.len() < n11 {
            at11.push(block[pos as usize]);
            pos += step;
        }
        let pings = sdroxide_dsp::fsk441_find_pings(&at11);
        assert!(
            pings.iter().any(|p| p.text.contains("W1ABC")),
            "the transmitted pass did not decode: {pings:?}"
        );
    }
}
