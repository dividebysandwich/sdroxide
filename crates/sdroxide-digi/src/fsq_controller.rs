//! FSQ (Fast Simple QSO) controller: the continuous keyboard modem plus the
//! FSQCALL **directed** layer — a heard list, parsed directed/ALLCALL messages,
//! and an automatic reply to the `?` heard-list query. The plain text path mirrors
//! [`TextModemController`](crate::TextModemController); the directed layer is text
//! parsing/formatting on top of it. Image transmit/receive is handled here too.
//!
//! FSQCALL framing here is a pragmatic subset: a transmission is `SENDER:` then
//! `target<trigger>payload` (` ` = message, `?` = heard-list query); ALLCALL and
//! broadcast are recognised. It is not yet bit-matched to fldigi's CRC framing
//! (tracked for live validation).

use std::collections::VecDeque;
use std::time::SystemTime;

use sdroxide_dsp::{FsqImageRx, FsqImageTx, FsqRx, FsqTx, MonoResampler};
use sdroxide_types::{DigiConfig, DigiStatus, FsqMsg, Mode, QsoStep};

use crate::DigiEngine;
use crate::controller::DigiAction;

const MODEM_RATE: f64 = 8000.0;
const OUT_RATE: f64 = 48_000.0;
const RX_TEXT_CAP: usize = 8000;
const HEARD_CAP: usize = 30;
const MSG_CAP: usize = 100;

pub struct FsqController {
    cfg: DigiConfig,
    audio_hz: f32,
    dial_hz: f64,

    rx: FsqRx,
    rx_rs: Option<MonoResampler>,
    rx_text: String,
    /// Bytes of `rx_text` already scanned for directed-message parsing.
    rx_parsed: usize,
    sq: crate::squelch::Squelch,
    /// FSQ image receiver (runs alongside the text decoder on the same tap).
    img_rx: FsqImageRx,
    /// Completed image actions awaiting the next `poll`.
    pending: Vec<DigiAction>,

    tx: FsqTx,
    /// Active image transmission, if any (takes priority over text TX).
    img_tx: Option<FsqImageTx>,
    img_done: bool,
    tx_rs: Option<MonoResampler>,
    tx48: VecDeque<f32>,
    tx_buffer: String,
    tx_pushed: usize,
    tx_active: bool,
    keyed: bool,
    last_sent: usize,

    heard: Vec<String>,
    messages: Vec<FsqMsg>,

    scratch8: Vec<f32>,
    scratch48: Vec<f32>,
    status_dirty: bool,
}

impl FsqController {
    pub fn new(cfg: DigiConfig, tap_rate: f64) -> Self {
        let audio_hz = 1500.0f32;
        let baud = cfg.fsq_baud as f64;
        FsqController {
            audio_hz,
            dial_hz: 0.0,
            rx: FsqRx::new(MODEM_RATE, audio_hz as f64, baud),
            rx_rs: MonoResampler::new(tap_rate, MODEM_RATE),
            rx_text: String::new(),
            rx_parsed: 0,
            sq: crate::squelch::Squelch::new(),
            img_rx: FsqImageRx::new(MODEM_RATE, audio_hz as f64),
            pending: Vec::new(),
            tx: FsqTx::new(MODEM_RATE, audio_hz as f64, baud),
            img_tx: None,
            img_done: false,
            tx_rs: MonoResampler::new(MODEM_RATE, OUT_RATE),
            tx48: VecDeque::new(),
            tx_buffer: String::new(),
            tx_pushed: 0,
            tx_active: false,
            keyed: false,
            last_sent: 0,
            heard: Vec::new(),
            messages: Vec::new(),
            scratch8: Vec::new(),
            scratch48: Vec::new(),
            status_dirty: true,
            cfg,
        }
    }

    fn my_call(&self) -> String {
        if !self.cfg.fsq_call.is_empty() {
            self.cfg.fsq_call.to_uppercase()
        } else {
            self.cfg.my_call.to_uppercase()
        }
    }

    fn retune(&mut self) {
        let baud = self.cfg.fsq_baud as f64;
        self.rx.set_params(self.audio_hz as f64, baud);
        self.tx.set_params(self.audio_hz as f64, baud);
        self.img_rx.set_center(self.audio_hz as f64);
    }

    /// FSQ transmits in discrete bursts (not a continuous carrier), so we keep
    /// generating audio only while queued text is still draining — not while
    /// `tx_active` idles. `tx_active` just arms the next burst.
    fn producing(&self) -> bool {
        self.tx.sent_chars() < self.tx.total_chars()
    }

    /// Scan newly-arrived RX text for complete `SENDER:...` transmissions and
    /// update the heard list + parsed messages. Returns an auto-reply to queue,
    /// if a `?` heard-query addressed us.
    fn parse_rx(&mut self) -> Option<String> {
        let mut auto_reply = None;
        // Work on whole lines only.
        while let Some(nl) = self.rx_text[self.rx_parsed..].find('\n') {
            let end = self.rx_parsed + nl;
            let line = self.rx_text[self.rx_parsed..end].trim().to_string();
            self.rx_parsed = end + 1;
            if let Some(reply) = self.parse_line(&line) {
                auto_reply = Some(reply);
            }
        }
        auto_reply
    }

    fn parse_line(&mut self, line: &str) -> Option<String> {
        let (from, after) = line.split_once(':')?;
        let from = from.trim().to_uppercase();
        if from.is_empty() {
            return None;
        }
        self.note_heard(&from);
        let after = after.trim_start();
        // Split the (optional) target token and its trigger from the payload.
        let target_len = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '/'))
            .unwrap_or(after.len());
        let target = after[..target_len].to_uppercase();
        let trigger = after[target_len..].chars().next().unwrap_or(' ');
        let payload = after[target_len..]
            .strip_prefix(trigger)
            .unwrap_or(&after[target_len..])
            .trim();
        let my = self.my_call();
        let is_query = trigger == '?';
        let (to, to_me) = if target.is_empty() {
            (String::new(), true) // broadcast
        } else {
            let mine = !my.is_empty() && (target == my || target == "ALLCALL");
            (target.clone(), mine)
        };
        self.push_message(FsqMsg {
            from: from.clone(),
            to: to.clone(),
            text: if payload.is_empty() { after.to_string() } else { payload.to_string() },
            to_me,
        });
        // Auto-answer a heard-list query addressed directly to us.
        if is_query && !my.is_empty() && to == my && !self.producing() {
            let list: Vec<String> = self.heard.iter().take(10).cloned().collect();
            return Some(format!("{my}:{from} HEARD {}\n", list.join(" ")));
        }
        None
    }

    fn note_heard(&mut self, call: &str) {
        self.heard.retain(|c| c != call);
        self.heard.insert(0, call.to_string());
        self.heard.truncate(HEARD_CAP);
        self.status_dirty = true;
    }

    fn push_message(&mut self, m: FsqMsg) {
        self.messages.push(m);
        if self.messages.len() > MSG_CAP {
            let drop = self.messages.len() - MSG_CAP;
            self.messages.drain(0..drop);
        }
        self.status_dirty = true;
    }

    fn build_status(&self) -> DigiStatus {
        DigiStatus {
            mode: Mode::Fsq,
            step: QsoStep::Idle,
            dx_call: None,
            dx_grid: None,
            tx_next: self.tx_active,
            tx_pending_msg: (!self.tx_buffer.is_empty()).then(|| self.tx_buffer.clone()),
            audio_hz: self.audio_hz,
            tx_even: false,
            transmitting: self.keyed,
            transcript: Vec::new(),
            config: self.cfg.clone(),
            text_rx: self.rx_text.clone(),
            tx_sent: self.tx.sent_chars(),
            fsq_heard: self.heard.clone(),
            fsq_messages: self.messages.clone(),
        }
    }

    fn queue_tx(&mut self, text: String) {
        self.set_tx_text(text);
        self.set_tx_active(true);
    }
}

impl DigiEngine for FsqController {
    fn mode(&self) -> Mode {
        Mode::Fsq
    }

    fn on_rx_audio(&mut self, tap: &[f32]) {
        self.scratch8.clear();
        match &mut self.rx_rs {
            Some(r) => r.push(tap, &mut self.scratch8),
            None => self.scratch8.extend_from_slice(tap),
        }
        // Image receiver runs on the same tap; a completed picture is emitted as
        // grayscale-as-RGB. Suppress text decode while a picture is arriving.
        let scratch = std::mem::take(&mut self.scratch8);
        if let Some((gray, w, h)) = self.img_rx.process(&scratch) {
            let rgb: Vec<u8> = gray.iter().flat_map(|&g| [g, g, g]).collect();
            self.pending.push(DigiAction::DigiImage { w, h, rgb });
            self.status_dirty = true;
        }
        let collecting = self.img_rx.is_collecting();
        self.scratch8 = scratch;
        if collecting {
            return;
        }
        let decoded = self.rx.process(&self.scratch8);
        // Squelch pure noise so the stream + heard list aren't polluted.
        let open = self.sq.open(self.rx.magnitude(), self.cfg.digi_squelch);
        if !decoded.is_empty() && open {
            self.rx_text.push_str(&decoded);
            if self.rx_text.len() > RX_TEXT_CAP {
                let cut = self.rx_text.len() - RX_TEXT_CAP;
                let cut = (cut..self.rx_text.len())
                    .find(|&i| self.rx_text.is_char_boundary(i))
                    .unwrap_or(self.rx_text.len());
                self.rx_text.drain(..cut);
                self.rx_parsed = self.rx_parsed.saturating_sub(cut);
            }
            self.status_dirty = true;
            if let Some(reply) = self.parse_rx() {
                self.queue_tx(reply);
            }
        }
    }

    fn poll(&mut self, _now: SystemTime, dial_hz: f64) -> Vec<DigiAction> {
        self.dial_hz = dial_hz;
        let mut actions = std::mem::take(&mut self.pending);
        if (self.tx_active || self.img_tx.is_some()) && !self.keyed {
            self.keyed = true;
            self.status_dirty = true;
            actions.push(DigiAction::KeyTx);
        }
        if self.status_dirty {
            self.status_dirty = false;
            actions.push(DigiAction::Status(self.build_status()));
        }
        actions
    }

    fn tx_burst_active(&self) -> bool {
        self.keyed
    }

    fn fill_tx_block(&mut self, out: &mut [f32]) -> bool {
        // Image transmission takes priority (a discrete picture burst).
        if self.img_tx.is_some() {
            while self.tx48.len() < out.len() && !self.img_done {
                self.scratch8.clear();
                self.scratch8.resize(400, 0.0);
                if let Some(itx) = &mut self.img_tx {
                    self.img_done = itx.next_block(&mut self.scratch8);
                }
                self.scratch48.clear();
                match &mut self.tx_rs {
                    Some(r) => r.push(&self.scratch8, &mut self.scratch48),
                    None => self.scratch48.extend_from_slice(&self.scratch8),
                }
                self.tx48.extend(self.scratch48.iter().copied());
            }
            for s in out.iter_mut() {
                *s = self.tx48.pop_front().unwrap_or(0.0);
            }
            return self.img_done && self.tx48.is_empty();
        }
        while self.tx48.len() < out.len() && self.producing() {
            self.scratch8.clear();
            self.scratch8.resize(400, 0.0);
            self.tx.next_block(&mut self.scratch8);
            self.scratch48.clear();
            match &mut self.tx_rs {
                Some(r) => r.push(&self.scratch8, &mut self.scratch48),
                None => self.scratch48.extend_from_slice(&self.scratch8),
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

    fn on_burst_done(&mut self) {
        // One-shot: the queued message/image has fully gone out; unkey + disarm.
        self.keyed = false;
        self.tx_active = false;
        self.img_tx = None;
        self.img_done = false;
        self.status_dirty = true;
    }

    fn abort(&mut self) {
        self.abort_tx();
    }

    fn abort_tx(&mut self) {
        self.tx.clear();
        self.tx48.clear();
        self.tx_buffer.clear();
        self.tx_pushed = 0;
        self.tx_active = false;
        self.img_tx = None;
        self.img_done = false;
        self.last_sent = 0;
        self.status_dirty = true;
    }

    fn set_config(&mut self, cfg: DigiConfig) {
        let retune = cfg.fsq_baud != self.cfg.fsq_baud;
        self.cfg = cfg;
        if retune {
            self.retune();
        }
        self.status_dirty = true;
    }

    fn set_audio_hz(&mut self, hz: f32) {
        self.audio_hz = hz.clamp(200.0, 3500.0);
        self.retune();
        self.status_dirty = true;
    }

    fn audio_hz(&self) -> f32 {
        self.audio_hz
    }

    fn status(&self) -> DigiStatus {
        self.build_status()
    }

    fn call_cq(&mut self) {
        let call = self.my_call();
        let call = if call.is_empty() { "NOCALL".to_string() } else { call };
        // FSQCALL CQ is a broadcast from our call.
        let cq = format!("{call}: CQ CQ CQ de {call} K\n");
        self.queue_tx(cq);
    }

    fn set_tx_text(&mut self, text: String) {
        let n = text.chars().count();
        if n > self.tx_pushed {
            let tail: String = text.chars().skip(self.tx_pushed).collect();
            self.tx.push_text(&tail);
            self.tx_pushed = n;
        }
        self.tx_buffer = text;
        self.status_dirty = true;
    }

    fn set_tx_active(&mut self, on: bool) {
        self.tx_active = on;
        self.status_dirty = true;
    }

    fn set_image(&mut self, gray: Vec<u8>, w: u16, h: u16) {
        self.img_tx =
            Some(FsqImageTx::new(MODEM_RATE, self.audio_hz as f64, &gray, w as usize, h as usize));
        self.img_done = false;
        self.status_dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctrl() -> FsqController {
        let mut cfg = DigiConfig::default();
        cfg.fsq_call = "AB1CD".into();
        FsqController::new(cfg, 48_000.0)
    }

    #[test]
    fn parses_directed_message_to_me() {
        let mut c = ctrl();
        assert!(c.parse_line("K9XYZ:AB1CD hello there").is_none());
        assert_eq!(c.heard, vec!["K9XYZ".to_string()]);
        assert_eq!(c.messages.len(), 1);
        let m = &c.messages[0];
        assert_eq!(m.from, "K9XYZ");
        assert_eq!(m.to, "AB1CD");
        assert!(m.to_me);
        assert_eq!(m.text, "hello there");
    }

    #[test]
    fn directed_to_other_is_not_to_me() {
        let mut c = ctrl();
        c.parse_line("K9XYZ:W5AAA hi");
        assert!(!c.messages[0].to_me);
    }

    #[test]
    fn heard_query_autoreplies() {
        let mut c = ctrl();
        c.parse_line("W5AAA:AB1CD hi"); // seed heard
        let reply = c.parse_line("K9XYZ:AB1CD?");
        let reply = reply.expect("? query should auto-reply");
        assert!(reply.starts_with("AB1CD:K9XYZ HEARD"));
        assert!(reply.contains("K9XYZ"));
        assert!(reply.contains("W5AAA"));
    }
}
