//! `TextModemController` — the continuous keyboard-mode (PSK31 / RTTY)
//! counterpart to the slotted [`DigiController`](crate::DigiController).
//!
//! Unlike FT8/FT4 there are no timed slots: RX decodes incrementally as audio
//! arrives, and TX streams an open-ended carrier while the operator's outgoing
//! text drains (idle reversals/mark when there is nothing to send). It reports
//! how many outgoing characters have gone out so the UI can colour the sent
//! prefix green.

use std::collections::VecDeque;
use std::time::SystemTime;

use sdroxide_dsp::{MonoResampler, PskRx, PskTx, RttyRx, RttyTx};
use sdroxide_types::{DigiConfig, DigiStatus, Mode, QsoStep, TranscriptLine};

use crate::DigiEngine;
use crate::controller::DigiAction;

/// Internal modem sample rate (integer samples/symbol for both modems).
const MODEM_RATE: f64 = 8000.0;
const OUT_RATE: f64 = 48_000.0;
/// Cap the rolling RX text so it can't grow without bound.
const RX_TEXT_CAP: usize = 8000;

enum RxModem {
    Psk(PskRx),
    Rtty(RttyRx),
}

impl RxModem {
    fn process(&mut self, audio: &[f32]) -> String {
        match self {
            RxModem::Psk(r) => r.process(audio),
            RxModem::Rtty(r) => r.process(audio),
        }
    }
}

enum TxModem {
    Psk(PskTx),
    Rtty(RttyTx),
}

impl TxModem {
    fn push_text(&mut self, t: &str) {
        match self {
            TxModem::Psk(m) => m.push_text(t),
            TxModem::Rtty(m) => m.push_text(t),
        }
    }
    fn next_block(&mut self, out: &mut [f32]) {
        match self {
            TxModem::Psk(m) => {
                m.next_block(out);
            }
            TxModem::Rtty(m) => {
                m.next_block(out);
            }
        }
    }
    fn sent_chars(&self) -> usize {
        match self {
            TxModem::Psk(m) => m.sent_chars(),
            TxModem::Rtty(m) => m.sent_chars(),
        }
    }
    fn total_chars(&self) -> usize {
        match self {
            TxModem::Psk(m) => m.total_chars(),
            TxModem::Rtty(m) => m.total_chars(),
        }
    }
    fn clear(&mut self) {
        match self {
            TxModem::Psk(m) => m.clear(),
            TxModem::Rtty(m) => m.clear(),
        }
    }
}

pub struct TextModemController {
    mode: Mode,
    cfg: DigiConfig,
    audio_hz: f32,
    dial_hz: f64,

    // RX
    rx: RxModem,
    rx_rs: Option<MonoResampler>,
    rx_text: String,

    // TX
    tx: TxModem,
    tx_rs: Option<MonoResampler>,
    tx48: VecDeque<f32>,
    tx_buffer: String,
    tx_pushed: usize,
    tx_active: bool,
    keyed: bool,
    last_sent: usize,

    scratch8: Vec<f32>,
    scratch48: Vec<f32>,
    status_dirty: bool,
}

impl TextModemController {
    pub fn new(mode: Mode, cfg: DigiConfig, tap_rate: f64) -> Self {
        let audio_hz = 1000.0f32;
        let (baud, shift) = (cfg.rtty_baud as f64, cfg.rtty_shift_hz as f64);
        let rx = match mode {
            Mode::Rtty => RxModem::Rtty(RttyRx::new(MODEM_RATE, audio_hz as f64, baud, shift)),
            _ => RxModem::Psk(PskRx::new(MODEM_RATE, audio_hz as f64)),
        };
        let tx = match mode {
            Mode::Rtty => TxModem::Rtty(RttyTx::new(MODEM_RATE, audio_hz as f64, baud, shift)),
            _ => TxModem::Psk(PskTx::new(MODEM_RATE, audio_hz as f64)),
        };
        TextModemController {
            mode,
            cfg,
            audio_hz,
            dial_hz: 0.0,
            rx,
            rx_rs: MonoResampler::new(tap_rate, MODEM_RATE),
            rx_text: String::new(),
            tx,
            tx_rs: MonoResampler::new(MODEM_RATE, OUT_RATE),
            tx48: VecDeque::new(),
            tx_buffer: String::new(),
            tx_pushed: 0,
            tx_active: false,
            keyed: false,
            last_sent: 0,
            scratch8: Vec::new(),
            scratch48: Vec::new(),
            status_dirty: true,
        }
    }

    fn retune_modems(&mut self) {
        let hz = self.audio_hz as f64;
        let (baud, shift) = (self.cfg.rtty_baud as f64, self.cfg.rtty_shift_hz as f64);
        match &mut self.rx {
            RxModem::Psk(r) => r.set_carrier(hz),
            RxModem::Rtty(r) => r.set_tuning(hz, baud, shift),
        }
        match &mut self.tx {
            TxModem::Psk(m) => m.set_carrier(hz, MODEM_RATE),
            TxModem::Rtty(m) => m.set_tuning(hz, shift),
        }
    }

    /// True while we should keep generating TX audio: actively transmitting, or
    /// still flushing already-queued characters after TX was released.
    fn producing(&self) -> bool {
        self.tx_active || self.tx.sent_chars() < self.tx.total_chars()
    }

    fn build_status(&self) -> DigiStatus {
        DigiStatus {
            mode: self.mode,
            step: QsoStep::Idle,
            dx_call: None,
            dx_grid: None,
            tx_next: self.tx_active,
            tx_pending_msg: (!self.tx_buffer.is_empty()).then(|| self.tx_buffer.clone()),
            audio_hz: self.audio_hz,
            tx_even: false,
            transmitting: self.keyed,
            transcript: Vec::<TranscriptLine>::new(),
            config: self.cfg.clone(),
            text_rx: self.rx_text.clone(),
            tx_sent: self.tx.sent_chars(),
        }
    }
}

impl DigiEngine for TextModemController {
    fn mode(&self) -> Mode {
        self.mode
    }

    fn on_rx_audio(&mut self, tap: &[f32]) {
        self.scratch8.clear();
        match &mut self.rx_rs {
            Some(r) => r.push(tap, &mut self.scratch8),
            None => self.scratch8.extend_from_slice(tap),
        }
        let decoded = self.rx.process(&self.scratch8);
        if !decoded.is_empty() {
            self.rx_text.push_str(&decoded);
            if self.rx_text.len() > RX_TEXT_CAP {
                let cut = self.rx_text.len() - RX_TEXT_CAP;
                // Trim on a char boundary.
                let cut = (cut..self.rx_text.len())
                    .find(|&i| self.rx_text.is_char_boundary(i))
                    .unwrap_or(self.rx_text.len());
                self.rx_text.drain(..cut);
            }
            self.status_dirty = true;
        }
    }

    fn poll(&mut self, _now: SystemTime, dial_hz: f64) -> Vec<DigiAction> {
        self.dial_hz = dial_hz;
        let mut actions = Vec::new();
        if self.tx_active && !self.keyed {
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
        // Generate 8 kHz modem audio and resample to 48 kHz until we have enough.
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
        self.keyed = false;
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
        self.last_sent = 0;
        self.status_dirty = true;
    }

    fn set_config(&mut self, cfg: DigiConfig) {
        let retune = cfg.rtty_baud != self.cfg.rtty_baud || cfg.rtty_shift_hz != self.cfg.rtty_shift_hz;
        self.cfg = cfg;
        if retune {
            self.retune_modems();
        }
        self.status_dirty = true;
    }

    fn set_audio_hz(&mut self, hz: f32) {
        self.audio_hz = hz.clamp(200.0, 3500.0);
        self.retune_modems();
        self.status_dirty = true;
    }

    fn audio_hz(&self) -> f32 {
        self.audio_hz
    }

    fn status(&self) -> DigiStatus {
        self.build_status()
    }

    fn call_cq(&mut self) {
        let call = if self.cfg.my_call.is_empty() { "NOCALL" } else { &self.cfg.my_call };
        let cq = format!("CQ CQ CQ DE {call} {call} {call} PSE K\n");
        self.set_tx_text(cq);
        self.set_tx_active(true);
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
}
