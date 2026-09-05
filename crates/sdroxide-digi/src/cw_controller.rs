//! `CwController` — the receive decoder and keyer behind the CW panel.
//!
//! CW is not a digital mode and this is not a digital-mode controller in the
//! sense the others are: there is no modem, no framing and nothing that has to
//! be decoded before the operator can use the signal. The rig is in CW, the
//! demodulated tone is what an operator listens to, and everything here runs
//! *alongside* that rather than in place of it — the panel reads a copy of what
//! the ear is already hearing, and keys what the operator types.
//!
//! It implements [`DigiEngine`] all the same, because the seam is exactly the
//! right shape: an audio tap in, actions and transmit audio out. What it does
//! not take from the other modes is ownership of the audio path. The
//! demodulated CW stays audible throughout.
//!
//! Two things are one thing here, and that is deliberate. The decoder listens
//! at `cw_pitch_hz` above the dial and the keyer transmits there, because in CW
//! those are the same frequency: you answer a station on the frequency you
//! heard it on, and the waterfall cursor that picks one picks the other.
//!
//! The text comes from DeepCW, which reads the spectrogram with a neural net
//! and copies several dB further down than a timing fit reaches. It answers only
//! that one question, though — it has no notion of speed, signal-to-noise, or
//! where the tone actually is. So the classic [`CwRx`] front end stays, running
//! alongside for exactly those readouts, and its AFC is also what tells the
//! tuner where to find the signal.
//!
//! Sending has two routes, and which one is right is a property of the radio
//! rather than of the operator. On an SDR the keyer's sidetone *is* the signal:
//! it goes through the transmit chain like any other audio and comes out as
//! on-off keying. On a transceiver told to be in CW it is nothing at all — the
//! rig keys its own transmitter and never modulates what arrives at its sound
//! card — so there the text is handed to the rig's keyer instead and the rig
//! does the sending. [`CatKeyer`] is that second route.

use std::collections::VecDeque;
use std::time::SystemTime;

use sdroxide_deepcw::{Tuner, Worker};
use sdroxide_dsp::{CwRx, CwTx, MonoResampler};
use sdroxide_types::{CwStatus, DigiConfig, DigiStatus, Mode, QsoStep, TranscriptLine};

use crate::DigiEngine;
use crate::controller::DigiAction;

/// Internal rate for the decoder and the keyer. High enough that the sidetone
/// is clean and the envelope detector has samples to work with, low enough that
/// the per-sample mixing costs nothing.
const CW_RATE: f64 = 8000.0;
const OUT_RATE: f64 = 48_000.0;
/// Cap on the rolling receive text.
const RX_TEXT_CAP: usize = 8000;
/// Sidetone samples generated per fill iteration.
const TX_CHUNK: usize = 400;
/// Drop out of transmit after this long with nothing left to send.
///
/// Holding the key down between characters is what makes typing feel like
/// sending, but "between characters" has to end somewhere: a transmitter left
/// keyed on an empty buffer holds the frequency, and the operator who wandered
/// off is exactly the one not watching for it.
const TX_IDLE_S: f32 = 5.0;
/// How long a part-typed word waits for the rest of itself before it is handed
/// to the rig anyway.
///
/// A rig keyed from text sends one message at a time, so every hand-off is a
/// seam in the sending. Waiting out a typist's pause puts the seams between
/// words, where the gap is a word space and nobody hears it, instead of inside
/// the callsign they were half way through.
const GATHER_S: f32 = 0.4;
/// Breathing room after a message before the next one goes down the wire — the
/// rig has to finish keying and take the new text in.
const CHUNK_GAP_S: f32 = 0.15;

/// Sending by handing text to the radio's own keyer.
///
/// Nothing comes back from the rig to say where it has got to, so this keeps
/// the clock itself: a message handed over at a known speed takes a known time,
/// which is what says when the next one may go, when the operator's text has
/// actually been sent, and when our own sending has stopped reaching the
/// decoder.
struct CatKeyer {
    /// Longest run of text the rig takes at once.
    chunk: usize,
    /// Typed, but not yet handed over.
    queue: String,
    /// When the message now on the air was handed over, and how long it takes.
    sending_since: Option<SystemTime>,
    sending_s: f32,
    /// Characters of the operator's buffer already sent, and how many are in
    /// the message on the air — enough to colour the panel's sent prefix as it
    /// goes out rather than a message at a time.
    sent: usize,
    in_flight: usize,
    /// When text last arrived, so a burst of typing leaves as one message.
    last_input: Option<SystemTime>,
    /// An abort the operator asked for, waiting to be passed on.
    abort: bool,
}

impl CatKeyer {
    fn new(chunk: usize) -> Self {
        CatKeyer {
            chunk: chunk.max(1),
            queue: String::new(),
            sending_since: None,
            sending_s: 0.0,
            sent: 0,
            in_flight: 0,
            last_input: None,
            abort: false,
        }
    }

    /// How far into the message on the air we are, 0..1. `None` when the rig is
    /// not sending — including once the message it was given has run out.
    fn progress(&self, now: SystemTime) -> Option<f32> {
        let since = self.sending_since?;
        // A wall clock that stepped backwards mid-message (NTP does) is not a
        // message that finished. Reading it as "no progress yet" holds the next
        // one back; reading it as "done" would land it on top of what the rig
        // is still keying.
        let elapsed = now.duration_since(since).unwrap_or_default().as_secs_f32();
        (elapsed < self.sending_s).then(|| (elapsed / self.sending_s.max(1e-3)).clamp(0.0, 1.0))
    }

    /// True while the rig is keying text we gave it.
    fn sending(&self, now: SystemTime) -> bool {
        self.progress(now).is_some()
    }

    /// Retire a finished message: its characters are sent, and the panel's
    /// colouring stops interpolating across them.
    fn settle(&mut self, now: SystemTime) {
        if self.sending_since.is_some() && !self.sending(now) {
            self.sent += self.in_flight;
            self.in_flight = 0;
            self.sending_since = None;
        }
    }

    /// The next message to hand over, or `None` if it is not time for one.
    ///
    /// Held back while the rig is still sending (a rig part way through a
    /// message has nowhere to put another), and while the operator looks like
    /// they are still typing the word — unless they have typed enough to fill a
    /// message, which is as long as anything may wait.
    fn next_message(&mut self, now: SystemTime, wpm: f32) -> Option<String> {
        if self.queue.is_empty() || self.sending_since.is_some() {
            return None;
        }
        let queued = self.queue.chars().count();
        let paused = self
            .last_input
            .and_then(|t| now.duration_since(t).ok())
            .is_some_and(|d| d.as_secs_f32() >= GATHER_S);
        // A trailing space is the operator finishing a word — the one moment a
        // seam costs nothing. A newline is the stronger form of the same thing:
        // the line was committed, so there is no more typing to wait out and
        // holding it for the gather timer would only delay it.
        if !(queued >= self.chunk || paused || self.queue.ends_with([' ', '\n'])) {
            return None;
        }
        let take = self.split_at(queued);
        let msg: String = self.queue.chars().take(take).collect();
        self.queue = self.queue.chars().skip(take).collect();
        self.in_flight = take;
        self.sending_since = Some(now);
        // The rig's keyer sends at one speed: Farnsworth spacing is the app's
        // sidetone keyer stretching the gaps, and there is no way to ask a rig
        // for it, so the estimate must not assume it either.
        self.sending_s = sdroxide_dsp::text_duration_s(&msg, wpm, 0.0) + CHUNK_GAP_S;
        Some(msg)
    }

    /// How many characters of the queue go in the next message: all of it when
    /// it fits, otherwise up to the last word break inside the limit so the
    /// seam lands between words.
    fn split_at(&self, queued: usize) -> usize {
        if queued <= self.chunk {
            return queued;
        }
        let head: Vec<char> = self.queue.chars().take(self.chunk).collect();
        head.iter()
            .rposition(|&c| c == ' ' || c == '\n')
            // Only when the break leaves a message worth sending; a long
            // unbroken run (a string of digits) has to be cut somewhere.
            .filter(|&i| i + 1 > self.chunk / 2)
            .map(|i| i + 1)
            .unwrap_or(self.chunk)
    }

    fn clear(&mut self) {
        self.queue.clear();
        self.sending_since = None;
        self.sending_s = 0.0;
        self.sent = 0;
        self.in_flight = 0;
        self.last_input = None;
    }
}

pub struct CwController {
    cfg: DigiConfig,

    // RX
    rx: CwRx,
    rx_rs: Option<MonoResampler>,
    rx_text: String,
    /// The tail DeepCW has decoded but not settled on. Shown after `rx_text`
    /// and replaced wholesale each time, so the last word or two visibly firms
    /// up instead of appearing late.
    rx_pending: String,
    /// `None` only if the model would not load, in which case the classic
    /// decoder's text is used instead of leaving the panel blank.
    deep: Option<Worker>,
    tuner: Tuner,
    deep_scratch: Vec<f32>,

    // TX
    tx: CwTx,
    tx_rs: Option<MonoResampler>,
    tx48: VecDeque<f32>,
    tx_buffer: String,
    tx_pushed: usize,
    tx_active: bool,
    keyed: bool,
    last_sent: usize,
    /// Something has actually been sent since transmit was switched on.
    ///
    /// What makes "the over is finished" different from "the over never
    /// started": under `send_on_enter` an empty buffer draining is the end of a
    /// message and releases transmit, but an empty buffer that was *always*
    /// empty is the operator holding the switch down, and taking it off them
    /// the instant they pressed it would read as a broken button.
    over_had_text: bool,
    /// Sidetone samples produced with an empty keyer queue.
    idle_samples: usize,
    /// Set when the radio keys itself from text — see [`CatKeyer`]. `None` when
    /// sending is the sidetone above, which is every SDR.
    cat: Option<CatKeyer>,

    scratch: Vec<f32>,
    scratch48: Vec<f32>,
    status_dirty: bool,
    /// Last reported decoder state, so status is emitted when it changes rather
    /// than on every poll — the speed readout moves by a tenth of a WPM
    /// constantly and the panel does not need to hear about it.
    last_cw: CwStatus,
}

impl CwController {
    /// `cat_keying` is how much text the radio's own keyer takes at a time, or
    /// `None` when sending is the keyer's sidetone through the transmit chain.
    pub fn new(cfg: DigiConfig, tap_rate: f64, cat_keying: Option<usize>) -> Self {
        let pitch = cfg.cw_pitch_hz;
        let mut rx = CwRx::new(CW_RATE, pitch);
        rx.set_speed_lock(cfg.cw_speed_lock.then_some(cfg.cw_wpm));
        let deep = match Worker::new() {
            Ok(w) => Some(w),
            Err(e) => {
                tracing::error!("DeepCW unavailable, falling back to the timing decoder: {e}");
                None
            }
        };
        CwController {
            rx,
            rx_rs: MonoResampler::new(tap_rate, CW_RATE),
            rx_text: String::new(),
            rx_pending: String::new(),
            deep,
            tuner: Tuner::new(tap_rate, pitch as f64),
            deep_scratch: Vec::new(),
            tx: CwTx::new(CW_RATE, pitch as f64, cfg.cw_wpm),
            tx_rs: MonoResampler::new(CW_RATE, OUT_RATE),
            tx48: VecDeque::new(),
            tx_buffer: String::new(),
            tx_pushed: 0,
            tx_active: false,
            keyed: false,
            last_sent: 0,
            over_had_text: false,
            idle_samples: 0,
            cat: cat_keying.map(CatKeyer::new),
            scratch: Vec::new(),
            scratch48: Vec::new(),
            status_dirty: true,
            last_cw: CwStatus::default(),
            cfg,
        }
    }

    /// Append settled text to the receive window, keeping it word-separated and
    /// bounded.
    fn append_rx(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        // DeepCW commits whole words with the edges trimmed, so the separator
        // between one commit and the next has to be put back.
        if !self.rx_text.is_empty()
            && !self.rx_text.ends_with(' ')
            && !text.starts_with(' ')
            && self.deep.is_some()
        {
            self.rx_text.push(' ');
        }
        self.rx_text.push_str(text);
        if self.rx_text.len() > RX_TEXT_CAP {
            let cut = self.rx_text.len() - RX_TEXT_CAP;
            let cut = (cut..self.rx_text.len())
                .find(|&i| self.rx_text.is_char_boundary(i))
                .unwrap_or(self.rx_text.len());
            self.rx_text.drain(..cut);
        }
        self.status_dirty = true;
    }

    /// Collect whatever the model finished since the last poll.
    fn drain_deep(&mut self) {
        let updates = self.deep.as_ref().map(Worker::poll).unwrap_or_default();
        for update in updates {
            match update {
                Ok(update) => {
                    self.append_rx(&update.committed);
                    if self.rx_pending != update.pending {
                        self.rx_pending = update.pending;
                        self.status_dirty = true;
                    }
                }
                // One failed inference is not fatal; the next window is a fresh
                // attempt at the same audio.
                Err(e) => tracing::warn!("DeepCW: {e}"),
            }
        }
    }

    /// The receive window as the panel should show it: settled text, then the
    /// tail that is still moving.
    fn rx_display(&self) -> String {
        if self.rx_pending.is_empty() {
            return self.rx_text.clone();
        }
        let mut out = String::with_capacity(self.rx_text.len() + self.rx_pending.len() + 1);
        out.push_str(&self.rx_text);
        if !out.is_empty() && !out.ends_with(' ') {
            out.push(' ');
        }
        out.push_str(&self.rx_pending);
        out
    }

    /// Keep the radio's own keyer fed: retire a finished message, hand over the
    /// next one when it is due, and pass on an abort.
    ///
    /// Only what the operator has asked to transmit goes out. Text typed with
    /// the panel out of transmit waits in the queue — the same as the sidetone
    /// keyer, which holds it in `CwTx` until the over starts.
    fn poll_cat(&mut self, now: SystemTime, actions: &mut Vec<DigiAction>) {
        let wpm = self.cfg.cw_wpm;
        let Some(cat) = self.cat.as_mut() else { return };
        if std::mem::take(&mut cat.abort) {
            actions.push(DigiAction::AbortCw);
            self.status_dirty = true;
        }
        let was_sending = cat.sending_since.is_some();
        cat.settle(now);
        if cat.sending_since.is_none() && was_sending {
            self.status_dirty = true; // the message finished: TX lamp, colouring
            // Our sending is over, so what the decoder holds is our own — the
            // same reset an over's end does on the sidetone path.
            self.rx.reset();
            if let Some(deep) = self.deep.as_ref() {
                deep.reset();
            }
            self.rx_pending.clear();
        }
        if self.tx_active
            && let Some(msg) = cat.next_message(now, wpm)
        {
            let seconds = cat.sending_s;
            actions.push(DigiAction::SendCw { text: msg, seconds });
            self.status_dirty = true;
        }
        // The sent prefix walks across the message as the rig keys it, and the
        // panel only hears about it when a status goes out — so it is a change
        // in the count, not the sending itself, that repaints. A character at
        // any sane speed is a good fraction of a second, so this is a handful
        // of updates a second and not one per tick.
        let n = self.sent_chars(now);
        if n != self.last_sent {
            self.last_sent = n;
            self.status_dirty = true;
        }
        // Same rule as the sidetone keyer: transmit is not left switched on
        // around an empty buffer.
        let empty =
            self.cat.as_ref().is_some_and(|c| c.queue.is_empty() && c.sending_since.is_none());
        if self.tx_active && !empty {
            self.over_had_text = true;
        }
        // And the same shortcut: a committed line ends its own over as soon as
        // the rig has finished keying it, rather than waiting out a typist who
        // was never going to type.
        let committed_over_done = self.cfg.send_on_enter && self.over_had_text;
        let waited = self.cat.as_ref().is_some_and(|c| {
            c.last_input
                .and_then(|t| now.duration_since(t).ok())
                .is_some_and(|d| d.as_secs_f32() >= TX_IDLE_S)
        });
        if self.tx_active && empty && (committed_over_done || waited) {
            self.over_had_text = false;
            self.tx_active = false;
            self.status_dirty = true;
        }
    }

    fn cw_status(&self) -> CwStatus {
        CwStatus {
            locked: self.rx.locked(),
            wpm: self.rx.wpm(),
            snr_db: self.rx.snr_db(),
            tone_hz: self.rx.tone_hz(),
        }
    }

    /// True while we should keep generating sidetone: keyed, or still draining
    /// characters the operator typed before releasing transmit.
    fn producing(&self) -> bool {
        self.tx_active || !self.tx.drained()
    }

    /// True while anything of ours is on the air — our own keying, by either
    /// route. What it guards is the decoder: the tap carries our sending, and
    /// reading it would only echo us back onto the panel.
    fn on_air(&self, now: SystemTime) -> bool {
        self.keyed || self.cat.as_ref().is_some_and(|c| c.sending(now))
    }

    /// Characters of the operator's buffer that have gone out — what colours
    /// the sent prefix. With the rig keying itself there is nothing to count
    /// but the clock, so the message on the air is interpolated across its own
    /// duration rather than jumping a message at a time.
    fn sent_chars(&self, now: SystemTime) -> usize {
        let Some(cat) = self.cat.as_ref() else {
            return self.tx.sent_chars();
        };
        cat.sent + (cat.in_flight as f32 * cat.progress(now).unwrap_or(1.0)).round() as usize
    }

    /// `now` is the caller's clock rather than this function's, so that what a
    /// status reports about the message on the air — how far into it the rig
    /// is — is the same instant the decision to send one was made against.
    fn build_status(&self, now: SystemTime) -> DigiStatus {
        DigiStatus {
            mode: Mode::Cw,
            step: QsoStep::Idle,
            dx_call: None,
            dx_grid: None,
            tx_next: self.tx_active,
            tx_pending_msg: (!self.tx_buffer.is_empty()).then(|| self.tx_buffer.clone()),
            audio_hz: self.cfg.cw_pitch_hz,
            tx_even: false,
            transmitting: self.on_air(now),
            tx_watchdog: false,
            transcript: Vec::<TranscriptLine>::new(),
            config: self.cfg.clone(),
            text_rx: self.rx_display(),
            tx_sent: self.sent_chars(now),
            fsq_heard: Vec::new(),
            fsq_messages: Vec::new(),
            rade: None,
            packet: None,
            navtex: None,
            aprs: None,
            js8: None,
            fox_queue: Vec::new(),
            call_queue: Vec::new(),
            clock_offset_s: None,
            cw: Some(self.cw_status()),
            wspr: None,
            qso: None,
        }
    }
}

impl DigiEngine for CwController {
    fn mode(&self) -> Mode {
        Mode::Cw
    }

    fn on_rx_audio(&mut self, tap: &[f32]) {
        // Our own sidetone is not a signal to copy. Full break-in would let the
        // decoder read the other station between our own elements, but the tap
        // carries what we are sending, not what they are, so reading it would
        // only echo us back onto the panel. A rig keying itself from text we
        // handed it is doing exactly that too, and its sidetone comes back down
        // the same audio path.
        if self.on_air(SystemTime::now()) {
            return;
        }
        self.scratch.clear();
        match &mut self.rx_rs {
            Some(r) => r.push(tap, &mut self.scratch),
            None => self.scratch.extend_from_slice(tap),
        }
        // The classic front end runs either way: it is where speed, SNR, lock
        // and the tone offset come from, and it is cheap next to the model.
        let classic = self.rx.process(&self.scratch);

        let Some(deep) = self.deep.as_ref() else {
            self.append_rx(&classic);
            return;
        };

        // Follow the tone the AFC settled on rather than the pitch the operator
        // dialled, so a drifting or mistuned signal still lands mid-band.
        self.tuner.set_tone(self.rx.tone_hz() as f64);
        self.deep_scratch.clear();
        self.tuner.push(tap, &mut self.deep_scratch);
        deep.push(&self.deep_scratch);
    }

    fn poll(&mut self, now: SystemTime, _dial_hz: f64) -> Vec<DigiAction> {
        let mut actions = Vec::new();
        self.drain_deep();
        if self.cat.is_some() {
            // The rig is doing the sending: there is no over for the engine to
            // key, only text to hand over as the rig gets through it.
            self.poll_cat(now, &mut actions);
        } else if self.tx_active && !self.keyed {
            self.keyed = true;
            self.status_dirty = true;
            // Nothing more will arrive for this over, so settle the tail now
            // rather than leaving the last word unconfirmed until they come
            // back to us.
            if let Some(deep) = self.deep.as_ref() {
                deep.flush();
            }
            actions.push(DigiAction::KeyTx);
        }
        // The decoder's readouts change continuously; only a change worth
        // showing repaints the panel.
        let cw = self.cw_status();
        let moved = cw.locked != self.last_cw.locked
            || (cw.wpm - self.last_cw.wpm).abs() > 0.5
            || (cw.snr_db - self.last_cw.snr_db).abs() > 1.0
            || (cw.tone_hz - self.last_cw.tone_hz).abs() > 2.0;
        if moved {
            self.last_cw = cw;
            self.status_dirty = true;
        }
        if self.status_dirty {
            self.status_dirty = false;
            actions.push(DigiAction::Status(self.build_status(now)));
        }
        actions
    }

    fn tx_burst_active(&self) -> bool {
        self.keyed
    }

    fn fill_tx_block(&mut self, out: &mut [f32]) -> bool {
        if self.cat.is_some() {
            // Nothing keys an over on this route, so nothing should be asking
            // for one. If something does, it gets silence and an immediate end
            // rather than a sidetone the rig would not transmit anyway.
            out.fill(0.0);
            return true;
        }
        if self.tx.drained() {
            // A committed line is a whole over: it ends when the text has gone
            // out, not five seconds later. The hang below exists to bridge the
            // gaps between typed characters, and under `send_on_enter` there
            // are no gaps to bridge — only a carrier nobody is using.
            if self.cfg.send_on_enter && self.over_had_text {
                self.over_had_text = false;
                self.tx_active = false;
                self.status_dirty = true;
            }
            self.idle_samples += out.len();
            if self.idle_samples as f32 > TX_IDLE_S * OUT_RATE as f32 {
                self.tx_active = false;
                self.status_dirty = true;
            }
        } else {
            self.idle_samples = 0;
            self.over_had_text = true;
        }
        while self.tx48.len() < out.len() && self.producing() {
            self.scratch.clear();
            self.scratch.resize(TX_CHUNK, 0.0);
            self.tx.next_block(&mut self.scratch);
            self.scratch48.clear();
            match &mut self.tx_rs {
                Some(r) => r.push(&self.scratch, &mut self.scratch48),
                None => self.scratch48.extend_from_slice(&self.scratch),
            }
            self.tx48.extend(self.scratch48.iter().copied());
        }
        for s in out.iter_mut() {
            *s = self.tx48.pop_front().unwrap_or(0.0);
        }
        if self.tx.sent_chars() != self.last_sent {
            self.last_sent = self.tx.sent_chars();
            self.status_dirty = true;
        }
        !self.producing() && self.tx48.is_empty()
    }

    /// The keyer's sidetone is already full scale: its raised-cosine envelope
    /// runs the whole way from silence to 1.0, so there is no headroom here for
    /// the transmit chain to divide out.
    fn tx_peak(&self) -> f32 {
        1.0
    }

    fn on_burst_done(&mut self) {
        self.keyed = false;
        self.idle_samples = 0;
        // The decoder heard nothing but our own sending for the length of that
        // over; its window is stale and its speed fit is ours, not theirs.
        self.rx.reset();
        if let Some(deep) = self.deep.as_ref() {
            deep.reset();
        }
        self.rx_pending.clear();
        self.status_dirty = true;
    }

    fn abort(&mut self) {
        self.abort_tx();
    }

    fn abort_tx(&mut self) {
        // First, and not merely for tidiness: `keyed` is what `poll` checks to
        // decide there is an over to start, and it used to be cleared only by
        // `on_burst_done` — which never runs for an over that was *refused*
        // rather than finished. The engine calls this when a rail denies a
        // key-up, so one refusal left the latch set and this controller could
        // never key again; the only cure was changing mode and back, which
        // rebuilds it. Every controller in this crate keys through the same
        // latch, so every one of them clears it here.
        self.keyed = false;
        self.tx.clear();
        self.tx48.clear();
        self.tx_buffer.clear();
        self.tx_pushed = 0;
        self.tx_active = false;
        self.last_sent = 0;
        self.idle_samples = 0;
        self.over_had_text = false;
        if let Some(cat) = self.cat.as_mut() {
            let sending = cat.sending_since.is_some();
            cat.clear();
            // Only worth telling the rig to stop if it is part way through
            // something; the queue it never saw is dropped here.
            cat.abort = sending;
        }
        self.status_dirty = true;
    }

    fn set_config(&mut self, cfg: DigiConfig) {
        if (cfg.cw_pitch_hz - self.cfg.cw_pitch_hz).abs() > 0.5 {
            self.rx.set_pitch(cfg.cw_pitch_hz);
            self.tuner.set_tone(cfg.cw_pitch_hz as f64);
        }
        self.rx.set_speed_lock(cfg.cw_speed_lock.then_some(cfg.cw_wpm));
        self.tx.set_params(cfg.cw_pitch_hz as f64, cfg.cw_wpm, cfg.cw_farnsworth_wpm);
        self.cfg = cfg;
        self.status_dirty = true;
    }

    /// The waterfall cursor. In CW the frequency being copied and the frequency
    /// being transmitted on are one and the same, so this moves both.
    fn set_audio_hz(&mut self, hz: f32) {
        let hz = hz.clamp(200.0, 3000.0);
        self.cfg.cw_pitch_hz = hz;
        self.rx.set_pitch(hz);
        self.tuner.set_tone(hz as f64);
        // The old frequency's audio is still buffered and is not this signal.
        if let Some(deep) = self.deep.as_ref() {
            deep.reset();
        }
        self.rx_pending.clear();
        self.tx.set_params(hz as f64, self.cfg.cw_wpm, self.cfg.cw_farnsworth_wpm);
        self.status_dirty = true;
    }

    fn audio_hz(&self) -> f32 {
        self.cfg.cw_pitch_hz
    }

    fn status(&self) -> DigiStatus {
        self.build_status(SystemTime::now())
    }

    fn call_cq(&mut self) {
        let call = if self.cfg.my_call.is_empty() { "NOCALL" } else { &self.cfg.my_call };
        let cq = format!("CQ CQ CQ DE {call} {call} {call} K ");
        self.set_tx_text(cq);
        self.set_tx_active(true);
    }

    /// Take the operator's outgoing buffer, keying whatever is new in it.
    ///
    /// Only the tail past what has already been queued goes to the keyer, so a
    /// character typed mid-transmission is sent on the end of what is already
    /// going out rather than restarting the message — which is the whole point
    /// of a keyboard on a CW panel.
    fn set_tx_text(&mut self, text: String) {
        let n = text.chars().count();
        if n > self.tx_pushed {
            let tail: String = text.chars().skip(self.tx_pushed).collect();
            match self.cat.as_mut() {
                // The rig keys from text, so the tail queues for it rather than
                // for the sidetone keyer — which would otherwise send an over
                // nothing on the air could hear.
                Some(cat) => {
                    cat.queue.push_str(&tail);
                    cat.last_input = Some(SystemTime::now());
                }
                None => self.tx.push_text(&tail),
            }
            self.tx_pushed = n;
        }
        self.tx_buffer = text;
        self.status_dirty = true;
    }

    fn set_tx_active(&mut self, on: bool) {
        self.tx_active = on;
        self.idle_samples = 0;
        // A fresh over: whatever the last one sent has no bearing on whether
        // this one has sent anything yet.
        self.over_had_text = false;
        // Coming out of transmit deliberately does not stop a message the rig
        // has already been given: it is on the air, and the operator watching
        // the sent prefix catch up is watching it go. `abort_tx` is what stops
        // it — that is what the panel's CLEAR is for.
        if let Some(cat) = self.cat.as_mut() {
            cat.last_input = Some(SystemTime::now());
        }
        self.status_dirty = true;
    }

    /// The unsettled tail goes with the settled text: it is on the same page,
    /// and leaving it behind would clear the window to a stray half-word.
    fn clear_rx(&mut self) {
        self.rx_text.clear();
        self.rx_pending.clear();
        self.status_dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> DigiConfig {
        DigiConfig { my_call: "W1AW".into(), cw_wpm: 25.0, ..Default::default() }
    }

    /// A refused key-up must not cost every later one.
    ///
    /// The engine calls `abort_tx` when a rail denies the key-up — the station
    /// interlock, an out-of-band dial, a radio that would not key. `keyed` was
    /// cleared only by `on_burst_done`, which never runs for an over that was
    /// refused rather than finished, so one refusal wedged the transmitter
    /// until the operator changed mode and back and rebuilt the controller.
    /// Reported from the field against CW on a HackRF, where a half-duplex
    /// key-up is refused often enough to hit within a couple of overs.
    #[test]
    fn a_refused_key_up_does_not_wedge_the_transmitter() {
        let mut c = CwController::new(cfg(), 48_000.0, None);
        fn keys(c: &mut CwController) -> bool {
            c.poll(SystemTime::now(), 14_030_000.0).iter().any(|a| matches!(a, DigiAction::KeyTx))
        }

        c.set_tx_text("CQ CQ DE W1AW".into());
        c.set_tx_active(true);
        assert!(keys(&mut c), "the first over must key");

        // Exactly what the engine does when a rail refuses.
        c.abort_tx();

        c.set_tx_text("CQ CQ DE W1AW".into());
        c.set_tx_active(true);
        assert!(keys(&mut c), "a refused over must not cost every later one");
    }

    /// Settle the receive window: the model decodes on its own thread, so the
    /// text a test wants has to be waited for rather than read straight back.
    ///
    /// Waited for on the decoder's own account rather than by watching the text
    /// stop changing. "The display has not moved for a second" is not the same
    /// claim as "the decoder has finished", and on a machine running the rest
    /// of this suite beside it they come apart: a decode is tens of
    /// milliseconds of arithmetic but nothing bounds how long that thread waits
    /// to be scheduled, so a pause between two committed words outlasted the
    /// window and this returned half a transmission. `Worker::queued` answers
    /// the question that was actually being asked.
    fn settle(c: &mut CwController) -> String {
        if let Some(deep) = c.deep.as_ref() {
            deep.flush();
            // Everything pushed — the whole transmission, and the flush behind
            // it — is through the model by the time this reaches zero, and
            // whatever it produced is already on the update channel.
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
            while deep.queued() > 0 && std::time::Instant::now() < deadline {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
        // Now drain it into the display. Nothing further is coming, so this
        // converges immediately; the loop is here because `poll` moves one
        // update at a time.
        for _ in 0..100 {
            let before = c.rx_display();
            c.poll(SystemTime::now(), 14_030_000.0);
            if c.rx_display() == before {
                break;
            }
        }
        c.rx_display().trim().to_string()
    }

    /// Transmit what the operator types, receive it back through the decoder.
    /// The controller sits between two halves that have to agree about pitch
    /// and speed, and this is the cheapest way to keep it honest.
    #[test]
    fn keys_what_is_typed_and_reads_it_back() {
        let mut c = CwController::new(cfg(), 48_000.0, None);
        c.set_tx_text("CQ DE W1AW K ".into());
        c.set_tx_active(true);
        c.poll(SystemTime::now(), 14_030_000.0);

        // Pull the transmission as the engine would, in 10 ms blocks.
        let mut audio = Vec::new();
        for _ in 0..2000 {
            let mut blk = [0.0f32; 480];
            let done = c.fill_tx_block(&mut blk);
            audio.extend_from_slice(&blk);
            if done {
                break;
            }
        }
        assert_eq!(c.tx.sent_chars(), 13, "not every character went out");

        // …and back in through the receive side, at the tap rate.
        // A receiver has been listening before the other station starts; the
        // decoder needs a second or so of channel to measure its noise floor
        // against before the first character arrives.
        let mut rx = CwController::new(cfg(), 48_000.0, None);
        let mut lead = vec![0.0f32; 48_000 * 3 / 2];
        lead.extend_from_slice(&audio);
        lead.resize(lead.len() + 48_000 * 3 / 2, 0.0);
        // A receiver always has a noise floor; the decoder measures against one.
        let mut seed = 99u32;
        for a in lead.iter_mut() {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            *a += 0.0004 * ((seed >> 8) as f32 / (1u32 << 24) as f32 - 0.5);
        }
        for blk in lead.chunks(1920) {
            rx.on_rx_audio(blk);
        }
        assert_eq!(settle(&mut rx), "CQ DE W1AW K");
        // The classic front end is still running, and is still where the speed
        // readout comes from.
        assert!((rx.rx.wpm() - 25.0).abs() < 2.0, "read {} WPM", rx.rx.wpm());
    }

    /// The cursor moves the decoder and the transmitter together — in CW they
    /// are the same frequency, and a panel where they could differ would be a
    /// panel that answers a station off its own frequency.
    #[test]
    fn the_cursor_moves_receive_and_transmit_together() {
        let mut c = CwController::new(cfg(), 48_000.0, None);
        c.set_audio_hz(600.0);
        assert_eq!(c.audio_hz(), 600.0);
        assert!((c.rx.tone_hz() - 600.0).abs() < 1.0);
        let st = c.status();
        assert_eq!(st.audio_hz, 600.0);
        assert_eq!(st.config.cw_pitch_hz, 600.0);

        // …and the keyer follows, which is only visible in what it generates.
        c.set_tx_text("E".into());
        c.set_tx_active(true);
        let mut audio = vec![0.0f32; 0];
        for _ in 0..200 {
            let mut blk = [0.0f32; 480];
            let done = c.fill_tx_block(&mut blk);
            audio.extend_from_slice(&blk);
            if done {
                break;
            }
        }
        // Correlate the keyed audio against each candidate pitch: the one the
        // cursor asked for has to win, and the default it moved off has to lose.
        let power_at = |hz: f64| -> f64 {
            let (mut re, mut im) = (0.0f64, 0.0f64);
            for (i, &s) in audio.iter().enumerate() {
                let p = std::f64::consts::TAU * hz * i as f64 / OUT_RATE;
                re += s as f64 * p.cos();
                im += s as f64 * p.sin();
            }
            (re * re + im * im).sqrt() / audio.len() as f64
        };
        let want = power_at(600.0);
        for other in [500.0, 700.0, 800.0] {
            assert!(
                want > 4.0 * power_at(other),
                "sidetone is not at 600 Hz: {want:.4} there against {:.4} at {other}",
                power_at(other)
            );
        }
    }

    /// An operator who stops typing stops transmitting. Holding the key down
    /// between characters is the point; holding it down for ever is not.
    #[test]
    fn transmit_releases_itself_when_there_is_nothing_left_to_send() {
        let mut c = CwController::new(cfg(), 48_000.0, None);
        c.set_tx_text("E".into());
        c.set_tx_active(true);
        c.poll(SystemTime::now(), 14_030_000.0);

        let mut blk = [0.0f32; 480];
        let mut blocks = 0;
        loop {
            let done = c.fill_tx_block(&mut blk);
            blocks += 1;
            if done {
                break;
            }
            assert!(blocks < 2000, "transmit never released the key");
        }
        c.on_burst_done();
        assert!(!c.tx_burst_active());
        // Roughly the idle timeout, and certainly not immediately after the dit.
        let held_s = blocks as f32 * 480.0 / OUT_RATE as f32;
        assert!((TX_IDLE_S..TX_IDLE_S + 1.0).contains(&held_s), "held the key for {held_s:.1} s");
    }

    /// Transmit must not decode itself. The tap carries our own sidetone while
    /// we key, and copying it would fill the receive pane with our own callsign.
    #[test]
    fn does_not_copy_its_own_sending() {
        let mut c = CwController::new(cfg(), 48_000.0, None);
        c.set_tx_text("CQ DE W1AW K ".into());
        c.set_tx_active(true);
        c.poll(SystemTime::now(), 14_030_000.0);
        assert!(c.tx_burst_active());
        for _ in 0..600 {
            let mut blk = [0.0f32; 480];
            let done = c.fill_tx_block(&mut blk);
            // The engine loops the transmitted audio back through the tap.
            c.on_rx_audio(&blk);
            if done {
                break;
            }
        }
        assert!(c.rx_text.is_empty(), "copied itself: {:?}", c.rx_text);
    }

    // ── sending from a radio that keys itself ───────────────────────────────

    use std::time::Duration;

    /// The text handed to the radio by a round of polling.
    fn keyed(actions: &[DigiAction]) -> Vec<String> {
        actions
            .iter()
            .filter_map(|a| match a {
                DigiAction::SendCw { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    /// On a rig that keys its own transmitter, what the operator types has to
    /// leave as text. Sidetone would be generated into a sound card the rig
    /// ignores, and the over the engine would key around it is an over that
    /// transmits nothing.
    #[test]
    fn hands_typed_text_to_a_radio_that_keys_itself() {
        let mut c = CwController::new(cfg(), 48_000.0, Some(50));
        let t0 = SystemTime::now();
        c.set_tx_text("CQ DE W1AW K ".into());
        c.set_tx_active(true);
        let actions = c.poll(t0, 14_030_000.0);
        assert_eq!(keyed(&actions), vec!["CQ DE W1AW K "]);
        assert!(
            !actions.iter().any(|a| matches!(a, DigiAction::KeyTx)),
            "keyed an over on a radio that keys itself"
        );
        // The rig is on the air, so the panel says so — and the decoder stays
        // off the tap, which is carrying our own sending.
        assert!(c.status().transmitting);
        assert!(c.on_air(t0));
        // Nothing is asked of the transmit chain, and if something asks anyway
        // it gets silence rather than a sidetone.
        let mut blk = [0.5f32; 480];
        assert!(c.fill_tx_block(&mut blk));
        assert!(blk.iter().all(|&s| s == 0.0));
    }

    /// A rig part way through a message has nowhere to put another, so the next
    /// one waits for the length of the one on the air — which is the only clock
    /// there is, nothing coming back to say where the rig has got to.
    #[test]
    fn sends_one_message_at_a_time() {
        let mut c = CwController::new(cfg(), 48_000.0, Some(50));
        let t0 = SystemTime::now();
        c.set_tx_text("CQ CQ CQ ".into());
        c.set_tx_active(true);
        assert_eq!(keyed(&c.poll(t0, 0.0)), vec!["CQ CQ CQ "]);
        let sending_s = c.cat.as_ref().unwrap().sending_s;
        assert!(sending_s > 1.0, "9 characters at 25 WPM take longer than {sending_s} s");

        c.set_tx_text("CQ CQ CQ DE W1AW ".into());
        let mid = t0 + Duration::from_secs_f32(sending_s * 0.5);
        assert!(keyed(&c.poll(mid, 0.0)).is_empty(), "wrote over a message on the air");
        // The sent prefix walks across the message as it goes out rather than
        // arriving all at once when it ends.
        let part = c.sent_chars(mid);
        assert!((1..9).contains(&part), "colouring did not follow the sending: {part}");

        let after = t0 + Duration::from_secs_f32(sending_s + 0.01);
        assert_eq!(keyed(&c.poll(after, 0.0)), vec!["DE W1AW "]);
        assert_eq!(c.sent_chars(after), 9, "the first message is fully sent");
    }

    /// The colouring only moves on the panel if a status goes out with it.
    #[test]
    fn the_sent_prefix_is_reported_as_it_advances() {
        let mut c = CwController::new(cfg(), 48_000.0, Some(50));
        let t0 = SystemTime::now();
        c.set_tx_text("CQ CQ CQ DE W1AW ".into());
        c.set_tx_active(true);
        c.poll(t0, 0.0);
        let sending_s = c.cat.as_ref().unwrap().sending_s;
        let status_at = |c: &mut CwController, t: SystemTime| -> Option<usize> {
            c.poll(t, 0.0).iter().find_map(|a| match a {
                DigiAction::Status(s) => Some(s.tx_sent),
                _ => None,
            })
        };
        // A tick into the same message, nothing has changed and nothing is said.
        assert_eq!(status_at(&mut c, t0 + Duration::from_millis(5)), None);
        // Part way through, the count has moved and the panel is told.
        let mid = t0 + Duration::from_secs_f32(sending_s * 0.5);
        let reported = status_at(&mut c, mid).expect("no status while the rig was sending");
        assert!((1..17).contains(&reported), "reported {reported} of 17 characters");
    }

    /// Every hand-off is a seam in the sending, so they go where a gap is a
    /// word space instead of the middle of a callsign.
    #[test]
    fn breaks_long_text_between_words() {
        let mut c = CwController::new(cfg(), 48_000.0, Some(10));
        let t0 = SystemTime::now();
        c.set_tx_text("CQ CQ CQ DE W1AW".into());
        c.set_tx_active(true);
        assert_eq!(keyed(&c.poll(t0, 0.0)), vec!["CQ CQ CQ "]);
        // A run with no word break in it has to be cut somewhere, and is.
        let mut c = CwController::new(cfg(), 48_000.0, Some(10));
        c.set_tx_text("599599599599".into());
        c.set_tx_active(true);
        assert_eq!(keyed(&c.poll(t0, 0.0)), vec!["5995995995"]);
    }

    #[test]
    fn a_part_typed_word_waits_for_the_rest_of_itself() {
        let mut c = CwController::new(cfg(), 48_000.0, Some(50));
        let t0 = SystemTime::now();
        c.set_tx_text("W1A".into());
        c.set_tx_active(true);
        assert!(keyed(&c.poll(t0, 0.0)).is_empty(), "a callsign went out half typed");
        // Finishing the word releases it at once — no waiting on a timer for
        // text the operator has plainly finished.
        c.set_tx_text("W1AW ".into());
        assert_eq!(keyed(&c.poll(t0, 0.0)), vec!["W1AW "]);
    }

    #[test]
    fn a_word_never_finished_goes_out_anyway() {
        let mut c = CwController::new(cfg(), 48_000.0, Some(50));
        let t0 = SystemTime::now();
        c.set_tx_text("73".into());
        c.set_tx_active(true);
        assert!(keyed(&c.poll(t0, 0.0)).is_empty());
        let paused = t0 + Duration::from_secs_f32(GATHER_S + 0.05);
        assert_eq!(keyed(&c.poll(paused, 0.0)), vec!["73"]);
    }

    /// A line the operator committed with Return has no more typing coming, so
    /// it is not held for the gather timer — and it leaves as one message,
    /// which on this route is one switch of the rig's relay for the whole line.
    #[test]
    fn a_committed_line_goes_over_at_once() {
        let mut c = CwController::new(cfg(), 48_000.0, Some(50));
        let t0 = SystemTime::now();
        c.set_tx_text("TNX FER CALL OM\n".into());
        c.set_tx_active(true);
        assert_eq!(keyed(&c.poll(t0, 0.0)), vec!["TNX FER CALL OM\n"]);
        // A line longer than the rig takes still breaks between words, and the
        // committing newline is one of them.
        let mut c = CwController::new(cfg(), 48_000.0, Some(10));
        c.set_tx_text("CQ CQ CQ\nDE W1AW\n".into());
        c.set_tx_active(true);
        assert_eq!(keyed(&c.poll(t0, 0.0)), vec!["CQ CQ CQ\n"]);
    }

    /// Send on return makes an over a finite thing: the rig is given the line,
    /// keys it, and transmit is released the moment it is done rather than five
    /// seconds later while the operator types the next one.
    #[test]
    fn a_committed_line_releases_transmit_when_the_rig_has_sent_it() {
        let mut c =
            CwController::new(DigiConfig { send_on_enter: true, ..cfg() }, 48_000.0, Some(50));
        let t0 = SystemTime::now();
        c.set_tx_text("TNX OM\n".into());
        c.set_tx_active(true);
        assert!(!keyed(&c.poll(t0, 0.0)).is_empty());
        let sending_s = c.cat.as_ref().unwrap().sending_s;
        assert!(sending_s + 0.01 < TX_IDLE_S, "the idle timer alone would pass this test");

        // Held while the rig is actually keying it.
        c.poll(t0 + Duration::from_secs_f32(sending_s * 0.5), 0.0);
        assert!(c.status().tx_next, "released while the rig was still sending");

        // Finished: released at once.
        c.poll(t0 + Duration::from_secs_f32(sending_s + 0.01), 0.0);
        assert!(!c.status().tx_next, "transmit was held after the line had gone out");
    }

    /// The switch is still a switch. Pressed over an empty buffer it holds, and
    /// only the idle timer takes it back — an over that never started is not an
    /// over that finished, and grabbing it back on the press would read as a
    /// broken button.
    #[test]
    fn holding_transmit_over_an_empty_buffer_still_holds() {
        let mut c =
            CwController::new(DigiConfig { send_on_enter: true, ..cfg() }, 48_000.0, Some(50));
        let t0 = SystemTime::now();
        c.set_tx_active(true);
        c.poll(t0 + Duration::from_secs_f32(TX_IDLE_S * 0.5), 0.0);
        assert!(c.status().tx_next, "an empty buffer ended the over on its own");
        c.poll(t0 + Duration::from_secs_f32(TX_IDLE_S + 1.0), 0.0);
        assert!(!c.status().tx_next, "the idle timer no longer applies");
    }

    /// The same on the sidetone route, where transmit really is the PTT line:
    /// the key comes up when the line has gone out, not after the hang that
    /// exists to bridge a typist's gaps.
    #[test]
    fn a_committed_line_releases_the_key_when_it_has_gone_out() {
        let mut c = CwController::new(DigiConfig { send_on_enter: true, ..cfg() }, 48_000.0, None);
        c.set_tx_text("E\n".into());
        c.set_tx_active(true);
        c.poll(SystemTime::now(), 14_030_000.0);

        let mut blk = [0.0f32; 480];
        let mut blocks = 0;
        while !c.fill_tx_block(&mut blk) {
            blocks += 1;
            assert!(blocks < 2000, "transmit never released the key");
        }
        let held_s = blocks as f32 * 480.0 / OUT_RATE as f32;
        assert!(held_s < TX_IDLE_S * 0.5, "held the key for {held_s:.1} s after one dit");
        assert!(!c.status().tx_next);
    }

    /// Nothing goes out until the operator says to transmit — the panel's TX
    /// button means the same thing on both routes.
    #[test]
    fn text_typed_out_of_transmit_waits() {
        let mut c = CwController::new(cfg(), 48_000.0, Some(50));
        let t0 = SystemTime::now();
        c.set_tx_text("CQ DE W1AW ".into());
        assert!(keyed(&c.poll(t0 + Duration::from_secs(1), 0.0)).is_empty());
        c.set_tx_active(true);
        assert_eq!(keyed(&c.poll(t0 + Duration::from_secs(1), 0.0)), vec!["CQ DE W1AW "]);
    }

    #[test]
    fn aborting_stops_a_message_the_rig_is_part_way_through() {
        let mut c = CwController::new(cfg(), 48_000.0, Some(50));
        let t0 = SystemTime::now();
        c.set_tx_text("CQ CQ CQ DE W1AW W1AW K ".into());
        c.set_tx_active(true);
        assert!(!keyed(&c.poll(t0, 0.0)).is_empty());

        c.abort_tx();
        let actions = c.poll(t0 + Duration::from_millis(10), 0.0);
        assert!(
            actions.iter().any(|a| matches!(a, DigiAction::AbortCw)),
            "the rig was left sending a message the operator dropped"
        );
        // Whatever had not been handed over is gone, and transmit is released.
        assert!(keyed(&c.poll(t0 + Duration::from_secs(30), 0.0)).is_empty());
        assert!(!c.status().tx_next);
        // Nothing is asked of a rig that was not sending in the first place.
        c.abort_tx();
        assert!(
            !c.poll(t0 + Duration::from_secs(31), 0.0)
                .iter()
                .any(|a| matches!(a, DigiAction::AbortCw))
        );
    }

    /// An operator who stops typing stops transmitting, the same as on the
    /// sidetone route — an idle transmit switch is a transmit switch nobody is
    /// watching.
    #[test]
    fn transmit_releases_itself_when_the_typing_stops() {
        let mut c = CwController::new(cfg(), 48_000.0, Some(50));
        let t0 = SystemTime::now();
        c.set_tx_text("73 ".into());
        c.set_tx_active(true);
        assert!(!keyed(&c.poll(t0, 0.0)).is_empty());
        assert!(c.status().tx_next);
        c.poll(t0 + Duration::from_secs_f32(TX_IDLE_S + 1.0), 0.0);
        assert!(!c.status().tx_next, "transmit was left on around an empty buffer");
    }
}
