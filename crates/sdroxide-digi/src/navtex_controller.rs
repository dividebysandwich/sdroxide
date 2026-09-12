//! `NavtexController` — the maritime safety broadcast, receive only.
//!
//! The decoder is [`sdroxide_dsp::NavtexRx`]; what lives here is everything
//! above the characters: the message framing, the duplicate suppression that
//! makes a station's four-hourly repeat cycle readable, and the status the
//! panel draws.
//!
//! # Framing
//!
//! A NAVTEX message is `ZCZC B1B2B3B4`, a body, and `NNNN`. The four header
//! characters are the transmitter (`B1`), the subject (`B2`) and a serial
//! number (`B3B4`) — and they are the whole of the addressing the service has,
//! so they are parsed out rather than left in the body. A station repeats each
//! message on its next slot, four hours later; a receiver that printed both
//! would fill the screen with yesterday, so a header already held replaces the
//! copy that is there rather than adding to it — **unless** the new copy is
//! better, which is the point of listening to a repeat at all.
//!
//! # Why there is no transmitter
//!
//! Not a limitation. NAVTEX is a coast station's service on a frequency
//! adjacent to distress traffic, and an amateur station putting characters on
//! it would be transmitting safety information nobody may act on. See
//! [`Mode::is_rx_only`].

use std::time::SystemTime;

use sdroxide_dsp::NavtexRx;
use sdroxide_types::{
    DigiConfig, DigiStatus, Mode, NAVTEX_MESSAGE_MAX, NavtexMessage, NavtexStatus, QsoStep,
    TranscriptLine,
};

use crate::DigiEngine;
use crate::controller::DigiAction;

/// Characters of loose text kept — what was decoded outside any message.
const TEXT_CAP: usize = 8_000;
/// The longest a message may run before it is closed without its `NNNN`.
///
/// A NAVTEX slot is ten minutes at a hundred baud, so a message that has
/// reached this has not ended: the closing sequence was lost, or the signal
/// was. Either way the text so far is worth keeping and the next `ZCZC` must
/// not append to it.
const BODY_CAP: usize = 20_000;

pub struct NavtexController {
    cfg: DigiConfig,
    rx: NavtexRx,
    /// Text decoded but not yet consumed by the framer.
    pending: String,
    /// The message being received, if a `ZCZC` has been seen.
    live: Option<NavtexMessage>,
    messages: Vec<NavtexMessage>,
    text: String,
    status_dirty: bool,
    last_status: Option<SystemTime>,
}

impl NavtexController {
    pub fn new(cfg: DigiConfig, tap_rate: f64) -> Self {
        let mut rx = NavtexRx::new(tap_rate);
        rx.set_reverse(cfg.navtex_reverse);
        NavtexController {
            cfg,
            rx,
            pending: String::new(),
            live: None,
            messages: Vec::new(),
            text: String::new(),
            status_dirty: true,
            last_status: None,
        }
    }

    fn now_unix() -> i64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }

    /// Take whatever complete framing is in [`Self::pending`].
    ///
    /// Character at a time rather than by searching the buffer, because a
    /// message arrives over ten minutes and the `ZCZC` that opens it may be
    /// split across any two calls. Once opened, everything goes into the body
    /// until `NNNN` closes it — including the line breaks, which is how a
    /// warning keeps its shape.
    fn frame(&mut self) {
        let taken = std::mem::take(&mut self.pending);
        for ch in taken.chars() {
            match self.live.as_mut() {
                None => {
                    self.text.push(ch);
                    if self.text.len() > TEXT_CAP {
                        let cut = self.text.len() - TEXT_CAP;
                        let cut = (cut..self.text.len())
                            .find(|&i| self.text.is_char_boundary(i))
                            .unwrap_or(self.text.len());
                        self.text.drain(..cut);
                    }
                    if self.text.ends_with("ZCZC") {
                        self.live = Some(NavtexMessage {
                            station: '?',
                            kind: '?',
                            serial: 0,
                            text: String::new(),
                            at: Self::now_unix(),
                            complete: false,
                            lost: 0,
                        });
                        self.status_dirty = true;
                    }
                }
                Some(msg) => {
                    if ch == '*' {
                        msg.lost += 1;
                    }
                    msg.text.push(ch);
                    // The four header characters follow `ZCZC` and a space,
                    // and are parsed as soon as they have all arrived. A
                    // station that sends no space still lands them here: the
                    // parse skips leading whitespace of its own.
                    if msg.station == '?' {
                        let head: Vec<char> =
                            msg.text.chars().filter(|c| !c.is_whitespace()).collect();
                        if head.len() >= 4 {
                            msg.station = head[0];
                            msg.kind = head[1];
                            msg.serial =
                                head[2..4].iter().collect::<String>().parse::<u8>().unwrap_or(0);
                            // The header is addressing, not text: drop it out
                            // of the body so the panel's first line is the
                            // warning rather than four letters of routing.
                            msg.text.clear();
                            self.status_dirty = true;
                        }
                    }
                    if msg.text.ends_with("NNNN") {
                        msg.text.truncate(msg.text.len() - 4);
                        msg.complete = true;
                        self.close();
                    } else if msg.text.len() > BODY_CAP {
                        self.close();
                    }
                }
            }
        }
    }

    /// File the message being received.
    ///
    /// A repeat of one already held replaces it when it is *better* — fewer
    /// characters lost, or the first copy that reached its `NNNN`. That is the
    /// whole reason a station repeats: the second pass through a fade fills in
    /// what the first missed, and a receiver that kept only the first, or kept
    /// both, throws that away.
    fn close(&mut self) {
        let Some(msg) = self.live.take() else { return };
        self.status_dirty = true;
        if msg.station == '?' {
            // A `ZCZC` whose header never arrived is not a message; the text is
            // in the loose stream already.
            return;
        }
        let key = (msg.station, msg.kind, msg.serial);
        if let Some(held) = self.messages.iter_mut().find(|m| (m.station, m.kind, m.serial) == key)
        {
            let better = (msg.complete && !held.complete)
                || (msg.complete == held.complete && msg.lost < held.lost);
            if better {
                let at = held.at;
                *held = msg;
                // The time it was *first* heard, which is what a reader is
                // looking for when a warning is timestamped.
                held.at = at;
            }
            return;
        }
        self.messages.push(msg);
        while self.messages.len() > NAVTEX_MESSAGE_MAX {
            self.messages.remove(0);
        }
    }

    fn navtex_status(&self) -> NavtexStatus {
        let (direct, repaired, lost) = self.rx.counts();
        NavtexStatus {
            in_sync: self.rx.in_sync(),
            level: self.rx.magnitude(),
            messages: self.messages.clone(),
            live: self.live.clone(),
            text: self.text.clone(),
            direct,
            repaired,
            lost,
            reverse: self.rx.reverse(),
        }
    }

    fn digi_status(&self) -> DigiStatus {
        DigiStatus {
            mode: Mode::Navtex,
            step: QsoStep::Idle,
            dx_call: None,
            dx_grid: None,
            tx_next: false,
            tx_pending_msg: None,
            audio_hz: sdroxide_types::NAVTEX_TONE_HZ,
            tx_even: false,
            transmitting: false,
            tx_watchdog: false,
            transcript: Vec::<TranscriptLine>::new(),
            config: self.cfg.clone(),
            text_rx: String::new(),
            tx_sent: 0,
            fsq_heard: Vec::new(),
            fsq_messages: Vec::new(),
            rade: None,
            packet: None,
            navtex: Some(self.navtex_status()),
            aprs: None,
            js8: None,
            atchat: None,
            fox_queue: Vec::new(),
            call_queue: Vec::new(),
            clock_offset_s: None,
            cw: None,
            wspr: None,
            qso: None,
        }
    }
}

impl DigiEngine for NavtexController {
    fn mode(&self) -> Mode {
        Mode::Navtex
    }

    fn on_rx_audio(&mut self, tap: &[f32]) {
        let text = self.rx.process(tap);
        if !text.is_empty() {
            self.pending.push_str(&text);
            self.frame();
            self.status_dirty = true;
        }
    }

    fn poll(&mut self, _now: SystemTime, _dial_hz: f64) -> Vec<DigiAction> {
        let now = SystemTime::now();
        // On change, and a few times a second regardless: the sync flag and
        // the level are what an operator watches while tuning, and neither
        // changes the text.
        let due = self
            .last_status
            .map(|t| now.duration_since(t).map(|d| d.as_secs_f32() > 0.25).unwrap_or(true))
            .unwrap_or(true);
        if !self.status_dirty && !due {
            return Vec::new();
        }
        self.status_dirty = false;
        self.last_status = Some(now);
        vec![DigiAction::Status(self.digi_status())]
    }

    /// Receive only — every transmit hook is a no-op rather than a refusal,
    /// so nothing upstream has to ask first.
    fn tx_burst_active(&self) -> bool {
        false
    }

    fn fill_tx_block(&mut self, _out: &mut [f32]) -> bool {
        false
    }

    fn on_burst_done(&mut self) {}

    /// Throw away the message in progress and the loose text — the page, not
    /// the file of messages already received.
    fn abort(&mut self) {
        self.live = None;
        self.pending.clear();
        self.text.clear();
        self.status_dirty = true;
    }

    fn abort_tx(&mut self) {}

    fn set_audio_hz(&mut self, _hz: f32) {}

    /// Fixed by the standard: the tones are 1615 and 1785 Hz, and there is
    /// nothing here for an operator to tune.
    fn audio_hz(&self) -> f32 {
        sdroxide_types::NAVTEX_TONE_HZ
    }

    fn status(&self) -> DigiStatus {
        self.digi_status()
    }

    fn set_config(&mut self, cfg: DigiConfig) {
        if cfg.navtex_reverse != self.cfg.navtex_reverse {
            self.rx.set_reverse(cfg.navtex_reverse);
            self.status_dirty = true;
        }
        self.cfg = cfg;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_dsp::navtex_test::{encode_bits, synth};

    /// One transmission, with the silence either side of it that a real slot
    /// has. The gap matters: two broadcasts butted together share no character
    /// phase, and a receiver that stayed latched on the first would read the
    /// front of the second at the wrong offset.
    fn feed(c: &mut NavtexController, text: &str) {
        let mut audio = vec![0.0f32; 8000];
        audio.extend(synth(&encode_bits(text), 8000.0, 1700.0, 0.4));
        audio.extend(std::iter::repeat_n(0.0f32, 8000));
        for chunk in audio.chunks(512) {
            c.on_rx_audio(chunk);
        }
    }

    fn ctrl() -> NavtexController {
        NavtexController::new(DigiConfig::default(), 8000.0)
    }

    #[test]
    fn it_is_a_receive_only_engine() {
        let c = ctrl();
        assert_eq!(c.mode(), Mode::Navtex);
        assert!(Mode::Navtex.is_rx_only());
        assert!(!c.tx_burst_active());
    }

    /// A whole broadcast comes out as one message, with the header taken apart
    /// and the body left as it was sent.
    #[test]
    fn a_broadcast_is_framed_into_a_message() {
        let mut c = ctrl();
        feed(&mut c, "ZCZC FA12 GALE WARNING NORTH SEA NNNN");
        let s = c.navtex_status();
        assert_eq!(
            s.messages.len(),
            1,
            "framed {:?} text={:?} live={:?}",
            s.messages,
            s.text,
            s.live
        );
        let m = &s.messages[0];
        assert_eq!(m.station, 'F');
        assert_eq!(m.kind, 'A');
        assert_eq!(m.serial, 12);
        assert!(m.complete, "the closing NNNN was not seen");
        assert!(m.text.contains("GALE WARNING NORTH SEA"), "body {:?}", m.text);
        assert_eq!(m.kind_label(), "Navigational warning");
        assert!(m.is_mandatory(), "a navigational warning may not be filtered out");
        assert!(s.live.is_none(), "the message was left open");
    }

    /// The four-hourly repeat replaces the copy already held when it is better,
    /// and is dropped when it is not — the whole reason a station repeats.
    #[test]
    fn a_repeat_replaces_a_worse_copy_and_never_doubles_the_list() {
        let mut c = ctrl();
        // First pass: the closing sequence is missed, so the message is filed
        // incomplete when the next one starts.
        feed(&mut c, "ZCZC OB07 FIRST PASS");
        feed(&mut c, "ZCZC OB07 SECOND PASS NNNN");
        let s = c.navtex_status();
        assert_eq!(s.messages.len(), 1, "the repeat was filed as a second message");
        let m = &s.messages[0];
        assert!(m.complete, "the complete copy did not replace the truncated one");
        assert!(m.text.contains("SECOND PASS"), "body {:?}", m.text);

        // …and a third, worse copy does not undo that.
        feed(&mut c, "ZCZC OB07 THIRD");
        let s = c.navtex_status();
        assert_eq!(s.messages.len(), 1);
        assert!(s.messages[0].text.contains("SECOND PASS"), "a worse copy replaced a better one");
    }

    /// Two different messages from the same transmitter are two messages: the
    /// serial number is part of what identifies one.
    #[test]
    fn different_serials_are_different_messages() {
        let mut c = ctrl();
        feed(&mut c, "ZCZC PA01 ONE NNNN");
        feed(&mut c, "ZCZC PA02 TWO NNNN");
        let s = c.navtex_status();
        assert_eq!(s.messages.len(), 2, "text={:?} live={:?}", s.text, s.live);
        assert_eq!(s.messages[0].serial, 1, "{:?}", s.messages);
        assert_eq!(s.messages[1].serial, 2, "{:?}", s.messages);
    }
}
