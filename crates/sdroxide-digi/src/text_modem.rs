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

use sdroxide_dsp::{
    MonoResampler, OliviaRx, OliviaTx, PskRx, PskTx, RttyRx, RttyTx, ThorRx, ThorTx,
};
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
    Olivia(OliviaRx),
    Thor(ThorRx),
}

impl RxModem {
    fn process(&mut self, audio: &[f32]) -> String {
        match self {
            RxModem::Psk(r) => r.process(audio),
            RxModem::Rtty(r) => r.process(audio),
            RxModem::Olivia(r) => r.process(audio),
            RxModem::Thor(r) => r.process(audio),
        }
    }
    fn magnitude(&self) -> f32 {
        match self {
            RxModem::Psk(r) => r.magnitude(),
            RxModem::Rtty(r) => r.magnitude(),
            RxModem::Olivia(r) => r.magnitude(),
            RxModem::Thor(r) => r.magnitude(),
        }
    }
}

enum TxModem {
    Psk(PskTx),
    Rtty(RttyTx),
    Olivia(OliviaTx),
    Thor(ThorTx),
}

impl TxModem {
    fn push_text(&mut self, t: &str) {
        match self {
            TxModem::Psk(m) => m.push_text(t),
            TxModem::Rtty(m) => m.push_text(t),
            TxModem::Olivia(m) => m.push_text(t),
            TxModem::Thor(m) => m.push_text(t),
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
            TxModem::Olivia(m) => {
                m.next_block(out);
            }
            TxModem::Thor(m) => {
                m.next_block(out);
            }
        }
    }
    fn sent_chars(&self) -> usize {
        match self {
            TxModem::Psk(m) => m.sent_chars(),
            TxModem::Rtty(m) => m.sent_chars(),
            TxModem::Olivia(m) => m.sent_chars(),
            TxModem::Thor(m) => m.sent_chars(),
        }
    }
    fn total_chars(&self) -> usize {
        match self {
            TxModem::Psk(m) => m.total_chars(),
            TxModem::Rtty(m) => m.total_chars(),
            TxModem::Olivia(m) => m.total_chars(),
            TxModem::Thor(m) => m.total_chars(),
        }
    }
    fn clear(&mut self) {
        match self {
            TxModem::Psk(m) => m.clear(),
            TxModem::Rtty(m) => m.clear(),
            TxModem::Olivia(m) => m.clear(),
            TxModem::Thor(m) => m.clear(),
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
    sq: crate::squelch::Squelch,

    // TX
    tx: TxModem,
    tx_rs: Option<MonoResampler>,
    tx48: VecDeque<f32>,
    tx_buffer: String,
    tx_pushed: usize,
    tx_active: bool,
    keyed: bool,
    last_sent: usize,
    /// Something has actually been sent since transmit was switched on — see
    /// the release under `send_on_enter` in [`Self::fill_tx_block`]. Without
    /// it, holding the switch down over an empty buffer would end the over the
    /// instant it began.
    over_had_text: bool,

    scratch8: Vec<f32>,
    scratch48: Vec<f32>,
    status_dirty: bool,
}

impl TextModemController {
    pub fn new(mode: Mode, cfg: DigiConfig, tap_rate: f64) -> Self {
        // MFSK modes want a higher default centre so their wider tone banks fit
        // inside the audio passband; the narrow PSK carrier sits at 1000 Hz.
        // RTTY starts on the standard tone pair instead of anywhere convenient:
        // 2125/2295 Hz is what every other RTTY program sends and expects, and
        // it is the offset the log has to take off again to name the contact.
        let audio_hz = match mode {
            Mode::Olivia | Mode::Thor | Mode::Fsq => 1500.0f32,
            Mode::Rtty | Mode::RttyFm => sdroxide_types::RTTY_CENTER_HZ,
            _ => 1000.0f32,
        };
        let (baud, shift) = (cfg.rtty_baud as f64, cfg.rtty_shift_hz as f64);
        let (o_tones, o_bw) = (cfg.olivia_tones as usize, cfg.olivia_bw_hz as f64);
        let t_baud = cfg.thor_mode.baud() as f64;
        let rx = match mode {
            Mode::Rtty | Mode::RttyFm => {
                let mut r = RttyRx::new(MODEM_RATE, audio_hz as f64, baud, shift);
                r.set_reverse(cfg.rtty_reverse);
                r.set_afc(cfg.rtty_afc);
                RxModem::Rtty(r)
            }
            Mode::Olivia => {
                RxModem::Olivia(OliviaRx::new(MODEM_RATE, audio_hz as f64, o_tones, o_bw))
            }
            Mode::Thor => RxModem::Thor(ThorRx::new(MODEM_RATE, audio_hz as f64, t_baud)),
            _ => RxModem::Psk(PskRx::new(MODEM_RATE, audio_hz as f64)),
        };
        let tx = match mode {
            Mode::Rtty | Mode::RttyFm => {
                TxModem::Rtty(RttyTx::new(MODEM_RATE, audio_hz as f64, baud, shift))
            }
            Mode::Olivia => {
                TxModem::Olivia(OliviaTx::new(MODEM_RATE, audio_hz as f64, o_tones, o_bw))
            }
            Mode::Thor => TxModem::Thor(ThorTx::new(MODEM_RATE, audio_hz as f64, t_baud)),
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
            sq: crate::squelch::Squelch::new(),
            tx,
            tx_rs: MonoResampler::new(MODEM_RATE, OUT_RATE),
            tx48: VecDeque::new(),
            tx_buffer: String::new(),
            tx_pushed: 0,
            tx_active: false,
            keyed: false,
            last_sent: 0,
            over_had_text: false,
            scratch8: Vec::new(),
            scratch48: Vec::new(),
            status_dirty: true,
        }
    }

    fn retune_modems(&mut self) {
        let hz = self.audio_hz as f64;
        let (baud, shift) = (self.cfg.rtty_baud as f64, self.cfg.rtty_shift_hz as f64);
        let (o_tones, o_bw) = (self.cfg.olivia_tones as usize, self.cfg.olivia_bw_hz as f64);
        let t_baud = self.cfg.thor_mode.baud() as f64;
        match &mut self.rx {
            RxModem::Psk(r) => r.set_carrier(hz),
            RxModem::Rtty(r) => {
                r.set_tuning(hz, baud, shift);
                r.set_reverse(self.cfg.rtty_reverse);
                r.set_afc(self.cfg.rtty_afc);
            }
            RxModem::Olivia(r) => r.set_params(hz, o_tones, o_bw),
            RxModem::Thor(r) => r.set_params(hz, t_baud),
        }
        match &mut self.tx {
            TxModem::Psk(m) => m.set_carrier(hz, MODEM_RATE),
            TxModem::Rtty(m) => m.set_tuning(hz, shift),
            TxModem::Olivia(m) => m.set_params(hz, o_tones, o_bw),
            TxModem::Thor(m) => m.set_params(hz, t_baud),
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
            tx_watchdog: false,
            transcript: Vec::<TranscriptLine>::new(),
            config: self.cfg.clone(),
            text_rx: self.rx_text.clone(),
            tx_sent: self.tx.sent_chars(),
            fsq_heard: Vec::new(),
            fsq_messages: Vec::new(),
            rade: None,
            packet: None,
            navtex: None,
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
        // Squelch: drop decoded text while the signal sits at the noise floor so
        // pure noise doesn't fill the window. `digi_squelch` is the sensitivity.
        //
        // RTTY and PSK are exempt: both now refuse to emit anything unless the
        // decoder itself is convinced (RTTY needs several consecutively
        // well-framed characters, PSK needs a carrier-coherent constellation),
        // which is a strictly better signal-presence test than a level
        // comparison. It is also the only one that works here — the magnitude
        // squelch seeds its noise floor from the first block it sees, so tuning
        // to a station that is already transmitting seeds the "floor" at the
        // signal's own level and the gate never opens at all. Olivia and THOR
        // have no such internal gate and still need it.
        let self_gating = matches!(self.mode, Mode::Rtty | Mode::RttyFm | Mode::Psk);
        let open = self.sq.open(self.rx.magnitude(), self.cfg.digi_squelch) || self_gating;
        if !decoded.is_empty() && open {
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
        // A committed line is a whole over, and it ends when the line does.
        // Holding the carrier past that is what type-ahead wants — the other
        // station hears that you are still there — but under `send_on_enter`
        // the operator is composing off the air, so the hold would be idle
        // reversals for as long as they take to type.
        if self.tx.sent_chars() < self.tx.total_chars() {
            self.over_had_text = true;
        } else if self.cfg.send_on_enter && self.over_had_text {
            self.over_had_text = false;
            self.tx_active = false;
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
        // See `CwController::abort_tx`: a refused key-up must not leave the
        // latch set, or nothing can ever key again.
        self.keyed = false;
        self.tx.clear();
        self.tx48.clear();
        self.tx_buffer.clear();
        self.tx_pushed = 0;
        self.tx_active = false;
        self.last_sent = 0;
        self.over_had_text = false;
        self.status_dirty = true;
    }

    fn set_config(&mut self, cfg: DigiConfig) {
        let retune = cfg.rtty_baud != self.cfg.rtty_baud
            || cfg.rtty_shift_hz != self.cfg.rtty_shift_hz
            || cfg.rtty_reverse != self.cfg.rtty_reverse
            || cfg.rtty_afc != self.cfg.rtty_afc
            || cfg.olivia_tones != self.cfg.olivia_tones
            || cfg.olivia_bw_hz != self.cfg.olivia_bw_hz
            || cfg.thor_mode != self.cfg.thor_mode;
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
        // A fresh over: what the last one sent says nothing about this one.
        self.over_had_text = false;
        self.status_dirty = true;
    }

    fn clear_rx(&mut self) {
        self.rx_text.clear();
        self.status_dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RTTY's tone pair is a standard, not a convenience: 2125 and 2295 Hz at
    /// the 170 Hz amateur shift, which is what every other RTTY program sends
    /// and expects. It used to start at 915/1085 — decodable here, but low in
    /// the passband, off what the rest of the band is on, and two kilohertz of
    /// unexplained offset between the dial and the contact (issue #143).
    ///
    /// PSK keeps its own carrier: one tone, no convention to keep.
    #[test]
    fn rtty_starts_on_the_standard_tone_pair() {
        let cfg = DigiConfig::default();
        let rtty = TextModemController::new(Mode::Rtty, cfg.clone(), 48_000.0);
        assert_eq!(rtty.audio_hz(), sdroxide_types::RTTY_CENTER_HZ);
        let shift = cfg.rtty_shift_hz;
        assert_eq!(rtty.audio_hz() - shift / 2.0, 2125.0, "space tone");
        assert_eq!(rtty.audio_hz() + shift / 2.0, 2295.0, "mark tone");

        let psk = TextModemController::new(Mode::Psk, cfg, 48_000.0);
        assert_eq!(psk.audio_hz(), 1000.0);
    }
}
