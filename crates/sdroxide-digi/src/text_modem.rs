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
    MonoResampler, OliviaTx, PskRx, PskTx, RttyRx, RttyTx, ThorRx, ThorTx, ardop,
    mfsk_rx::MfskReceiver, mt63::Mt63Receiver,
};
use sdroxide_olivia::OliviaReceiver;
use sdroxide_types::{DigiConfig, DigiStatus, Mode, QsoStep, TranscriptLine};

/// ARDOP's front end: the modem runs at 12 kHz and takes the tap directly, so a
/// receiver of its own rather than a fourth `process(&[f32])` arm. Its
/// resampler is built from the *tap* rate — the controller keeps the
/// tap→modem-rate stage out of its path, because ARDOP's grid is fixed at its
/// own rate and a round trip through 8 kHz would cost half the band for nothing.
struct ArdopRx {
    rs: Option<MonoResampler>,
    rx: ardop::ArdopReceiver,
    scratch: Vec<f32>,
    buf: Vec<f32>,
}

impl ArdopRx {
    fn new(tap_rate: f64) -> Self {
        ArdopRx {
            rs: MonoResampler::new(tap_rate, ardop::SR),
            rx: ardop::ArdopReceiver::new(),
            scratch: Vec::new(),
            buf: Vec::new(),
        }
    }

    /// One line per frame: the mode it arrived in and what it carried. ARDOP is a
    /// frame protocol, so a "decode" is a frame rather than a character and the
    /// log says which kind it was.
    fn process(&mut self, audio: &[f32]) -> String {
        self.buf.clear();
        match &mut self.rs {
            Some(r) => {
                self.scratch.clear();
                r.push(audio, &mut self.scratch);
                self.buf.extend_from_slice(&self.scratch);
            }
            None => self.buf.extend_from_slice(audio),
        }
        let mut out = String::new();
        for f in self.rx.process(&self.buf) {
            out.push('[');
            out.push_str(&ardop::frame_name(f.ftype));
            out.push(']');
            match &f.data {
                Some(data) => {
                    let text: String = String::from_utf8_lossy(data)
                        .chars()
                        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
                        .filter(|c| (' '..='~').contains(c))
                        .collect();
                    out.push(' ');
                    out.push_str(text.trim_end());
                }
                None => out.push_str(" (body failed its check)"),
            }
            out.push('\n');
        }
        out
    }

    fn magnitude(&self) -> f32 {
        self.rx.magnitude()
    }

    /// A retune abandons any part-decoded frame: it is not ours any more.
    fn reset(&mut self) {
        self.rx.reset();
    }
}

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
    Olivia(OliviaReceiver),
    Thor(ThorRx),
    Mfsk(MfskReceiver),
    Mt63(Mt63Receiver),
    Ardop(ArdopRx),
}

impl RxModem {
    fn process(&mut self, audio: &[f32]) -> String {
        match self {
            RxModem::Psk(r) => r.process(audio),
            RxModem::Rtty(r) => r.process(audio),
            RxModem::Olivia(r) => r.process(audio),
            RxModem::Thor(r) => r.process(audio),
            RxModem::Mfsk(r) => r.process(audio),
            RxModem::Mt63(r) => r.process(audio),
            RxModem::Ardop(r) => r.process(audio),
        }
    }
    fn magnitude(&self) -> f32 {
        match self {
            RxModem::Psk(r) => r.magnitude(),
            RxModem::Rtty(r) => r.magnitude(),
            RxModem::Olivia(r) => r.magnitude(),
            RxModem::Thor(r) => r.magnitude(),
            RxModem::Mfsk(r) => r.magnitude(),
            RxModem::Mt63(r) => r.magnitude(),
            RxModem::Ardop(r) => r.magnitude(),
        }
    }
}

enum TxModem {
    Psk(PskTx),
    Rtty(RttyTx),
    Olivia(OliviaTx),
    Thor(ThorTx),
    /// A mode whose modulator is not written yet. It holds the controller's
    /// shape without pretending: `set_tx_active` refuses a key-up in a
    /// receive-only mode, so nothing here ever reaches the radio, and this arm
    /// is what keeps the alternative from being a second controller.
    Receive,
}

impl TxModem {
    fn push_text(&mut self, t: &str) {
        match self {
            TxModem::Psk(m) => m.push_text(t),
            TxModem::Rtty(m) => m.push_text(t),
            TxModem::Olivia(m) => m.push_text(t),
            TxModem::Thor(m) => m.push_text(t),
            TxModem::Receive => {
                let _ = t;
            }
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
            TxModem::Receive => out.fill(0.0),
        }
    }
    /// Olivia closes a transmission with tones of its own; the other modes end
    /// when their characters do. Only Olivia has a tail.
    fn tail_pending(&self) -> bool {
        match self {
            TxModem::Olivia(m) => m.tail_pending(),
            _ => false,
        }
    }

    fn sent_chars(&self) -> usize {
        match self {
            TxModem::Psk(m) => m.sent_chars(),
            TxModem::Rtty(m) => m.sent_chars(),
            TxModem::Olivia(m) => m.sent_chars(),
            TxModem::Thor(m) => m.sent_chars(),
            TxModem::Receive => 0,
        }
    }
    fn total_chars(&self) -> usize {
        match self {
            TxModem::Psk(m) => m.total_chars(),
            TxModem::Rtty(m) => m.total_chars(),
            TxModem::Olivia(m) => m.total_chars(),
            TxModem::Thor(m) => m.total_chars(),
            TxModem::Receive => 0,
        }
    }
    fn clear(&mut self) {
        match self {
            TxModem::Psk(m) => m.clear(),
            TxModem::Rtty(m) => m.clear(),
            TxModem::Olivia(m) => m.clear(),
            TxModem::Thor(m) => m.clear(),
            TxModem::Receive => {}
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
        // The fixed-centre modes read their centre from the mode, which is also
        // what the panadapter anchors on: one number, so a click that puts the
        // signal where the demodulator listens cannot drift from where the
        // window says it is.
        let audio_hz = if mode.holds_audio_centre() {
            mode.audio_centre_hz()
        } else {
            match mode {
                Mode::Rtty | Mode::RttyFm => sdroxide_types::RTTY_CENTER_HZ,
                _ => 1000.0f32,
            }
        };
        let (baud, shift) = (cfg.rtty_baud as f64, cfg.rtty_shift_hz as f64);
        let (o_tones, o_bw) = (cfg.olivia_tones as usize, cfg.olivia_bw_hz as f64);
        let t_baud = cfg.thor_mode.baud() as f64;
        let p_baud = cfg.psk_speed.baud();
        let p_qpsk = cfg.psk_modulation == sdroxide_types::PskModulation::Qpsk;
        let rx = match mode {
            Mode::Rtty | Mode::RttyFm => {
                let mut r = RttyRx::new(MODEM_RATE, audio_hz as f64, baud, shift);
                r.set_reverse(cfg.rtty_reverse);
                r.set_afc(cfg.rtty_afc);
                RxModem::Rtty(r)
            }
            Mode::Olivia => RxModem::Olivia(OliviaReceiver::new(
                MODEM_RATE,
                audio_hz as f64,
                o_tones,
                o_bw as u32,
                cfg.olivia_reverse,
            )),
            Mode::Contestia => RxModem::Olivia(OliviaReceiver::contestia(
                MODEM_RATE,
                audio_hz as f64,
                // Contestia's narrowest is four carriers, and this field is
                // shared with Olivia — a 2 left over from Olivia would ask for
                // a mode Contestia does not have.
                o_tones.max(4),
                o_bw as u32,
                cfg.olivia_reverse,
            )),
            Mode::Thor => RxModem::Thor(ThorRx::new(MODEM_RATE, audio_hz as f64, t_baud)),
            Mode::Mfsk => RxModem::Mfsk(MfskReceiver::new(
                MODEM_RATE,
                audio_hz as f64,
                cfg.mfsk_mode,
                cfg.mfsk_reverse,
            )),
            Mode::Mt63 => {
                RxModem::Mt63(Mt63Receiver::new(MODEM_RATE, audio_hz as f64, cfg.mt63_mode.into()))
            }
            Mode::Ardop => RxModem::Ardop(ArdopRx::new(tap_rate)),
            _ => {
                let mut r = PskRx::new_baud(MODEM_RATE, audio_hz as f64, p_baud);
                r.set_qpsk(p_qpsk);
                RxModem::Psk(r)
            }
        };
        let tx = match mode {
            Mode::Rtty | Mode::RttyFm => {
                TxModem::Rtty(RttyTx::new(MODEM_RATE, audio_hz as f64, baud, shift))
            }
            Mode::Olivia => {
                TxModem::Olivia(OliviaTx::new(MODEM_RATE, audio_hz as f64, o_tones, o_bw))
            }
            Mode::Thor => TxModem::Thor(ThorTx::new(MODEM_RATE, audio_hz as f64, t_baud)),
            // MFSK's decoder is ported and reads the corpus; its modulator is
            // not written yet, so there is nothing to hand text to and the
            // controller refuses the key-up with a reason. Contestia is the
            // same shape for a different reason: its transmitter still speaks
            // Olivia's scrambler.
            Mode::Mfsk | Mode::Contestia | Mode::Mt63 | Mode::Ardop => TxModem::Receive,
            _ => {
                let mut t = PskTx::new_baud(MODEM_RATE, audio_hz as f64, p_baud);
                t.set_qpsk(p_qpsk);
                TxModem::Psk(t)
            }
        };
        TextModemController {
            mode,
            cfg,
            audio_hz,
            dial_hz: 0.0,
            rx,
            // ARDOP resamples the tap itself, straight to the modem's 12 kHz.
            rx_rs: (mode != Mode::Ardop)
                .then(|| MonoResampler::new(tap_rate, MODEM_RATE))
                .flatten(),
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
        let p_baud = self.cfg.psk_speed.baud();
        let p_qpsk = self.cfg.psk_modulation == sdroxide_types::PskModulation::Qpsk;
        match &mut self.rx {
            RxModem::Psk(r) => {
                r.set_carrier(hz);
                r.set_baud(p_baud);
                r.set_qpsk(p_qpsk);
            }
            RxModem::Rtty(r) => {
                r.set_tuning(hz, baud, shift);
                r.set_reverse(self.cfg.rtty_reverse);
                r.set_afc(self.cfg.rtty_afc);
            }
            RxModem::Olivia(r) => {
                r.set_reverse(self.cfg.olivia_reverse);
                r.set_params(hz, o_tones, o_bw as u32);
            }
            RxModem::Thor(r) => r.set_params(hz, t_baud),
            RxModem::Mfsk(r) => {
                r.set_mirror(self.cfg.mfsk_reverse);
                r.set_params(hz, self.cfg.mfsk_mode);
            }
            RxModem::Mt63(r) => r.set_params(hz, self.cfg.mt63_mode.into()),
            RxModem::Ardop(r) => r.reset(),
        }
        match &mut self.tx {
            TxModem::Psk(m) => {
                m.set_carrier(hz, MODEM_RATE);
                m.set_baud(p_baud, MODEM_RATE);
                m.set_qpsk(p_qpsk);
            }
            TxModem::Rtty(m) => m.set_tuning(hz, shift),
            TxModem::Olivia(m) => {
                m.set_reverse(self.cfg.olivia_reverse);
                m.set_params(hz, o_tones, o_bw);
            }
            TxModem::Thor(m) => m.set_params(hz, t_baud),
            TxModem::Receive => {}
        }
    }

    /// True while we should keep generating TX audio: actively transmitting,
    /// still flushing already-queued characters after TX was released, or —
    /// Olivia only — still playing the bracket that closes the transmission.
    ///
    /// That last clause is the whole reason this is a contract and not a
    /// detail. Olivia's closing tones are generated *after* the last character
    /// is out, and the caller stopped asking for audio the moment
    /// `sent_chars` reached `total_chars`, so without it the bracket was built
    /// and never played: an interop feature that could never be heard.
    fn producing(&self) -> bool {
        self.tx_active || self.tx.sent_chars() < self.tx.total_chars() || self.tx.tail_pending()
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
            tx_refused: self.mode.is_rx_only().then(|| match self.mode {
                Mode::Contestia => {
                    "Contestia is receive only in this build — the transmitter still sends \
                     Olivia's scrambling code"
                        .to_string()
                }
                Mode::Mt63 => {
                    "MT63 is receive only in this build — the modulator is not written yet"
                        .to_string()
                }
                Mode::Ardop => {
                    "ARDOP is receive only in this build — the modem has no transmitter and the \
                     ARQ state machine is not ported"
                        .to_string()
                }
                _ => "MFSK is receive only in this build — the modulator is not written yet"
                    .to_string(),
            }),
            transcript: Vec::<TranscriptLine>::new(),
            config: self.cfg.clone(),
            text_rx: self.rx_text.clone(),
            tx_sent: self.tx.sent_chars(),
            fsq_heard: Vec::new(),
            fsq_messages: Vec::new(),
            rade: None,
            packet: None,
            navtex: None,
            acars: None,
            dsc: None,
            uvpacket: None,
            jtty: None,
            ale: None,
            aprs: None,
            js8: None,
            atchat: None,
            fox_queue: Vec::new(),
            call_queue: Vec::new(),
            clock_offset_s: None,
            cw: None,
            wspr: None,
            pi4: None,
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
            || cfg.olivia_reverse != self.cfg.olivia_reverse
            || cfg.thor_mode != self.cfg.thor_mode
            || cfg.psk_speed != self.cfg.psk_speed
            || cfg.psk_modulation != self.cfg.psk_modulation
            || cfg.mfsk_mode != self.cfg.mfsk_mode
            || cfg.mfsk_reverse != self.cfg.mfsk_reverse
            || cfg.mt63_mode != self.cfg.mt63_mode;
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
        // A receive-only mode refuses the key-up and says why, rather than
        // arming a latch that would key the radio with nothing to send. The
        // panel greys the control too; this is the half a remote client sees.
        if on && self.mode.is_rx_only() {
            self.tx_active = false;
            self.status_dirty = true;
            return;
        }
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

    /// MFSK's decoder is ported and reads the corpus; its modulator is not
    /// written, so the controller must refuse the key-up **and say why** — a
    /// mode that greys nothing and keys silence is the fault the house rules
    /// name. The other text modes must not pick up the refusal, which is what
    /// stops this being a blanket.
    #[test]
    fn a_receive_only_mode_refuses_the_key_and_says_why() {
        let mut mfsk = TextModemController::new(Mode::Mfsk, DigiConfig::default(), 48_000.0);
        mfsk.set_tx_text("TEST THE EAGLE".into());
        mfsk.set_tx_active(true);
        let actions = mfsk.poll(SystemTime::now(), 0.0);
        let s = mfsk.status();
        assert!(!s.tx_next, "a receive-only mode armed the transmitter");
        assert!(!mfsk.tx_burst_active(), "it asked the engine to key");
        assert!(
            !actions.iter().any(|a| matches!(a, DigiAction::KeyTx)),
            "it sent KeyTx to the engine"
        );
        assert!(s.tx_refused.is_some(), "it refused in silence");
        assert!(
            s.tx_refused.as_deref().unwrap_or("").contains("MFSK"),
            "the refusal does not name the mode: {:?}",
            s.tx_refused
        );

        let mut psk = TextModemController::new(Mode::Psk, DigiConfig::default(), 48_000.0);
        psk.set_tx_active(true);
        psk.poll(SystemTime::now(), 0.0);
        assert!(psk.status().tx_refused.is_none(), "PSK picked up the refusal");
        assert!(psk.tx_burst_active(), "PSK stopped transmitting");
    }

    /// The Contestia corpus sample through the controller — the same object the
    /// console builds. Contestia is Olivia's engine with a different scrambling
    /// code, so what this catches is the flag reaching the constructor and the
    /// shared tones/bandwidth being placed where fldigi places them.
    #[test]
    #[ignore]
    fn the_contestia_corpus_sample_decodes_through_the_controller() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../samples/fldigi/CONTESTIA.wav");
        let mut reader = hound::WavReader::open(path).expect("the Contestia sample");
        let spec = reader.spec();
        let audio: Vec<f32> =
            reader.samples::<i16>().map(|s| s.expect("sample") as f32 / 32_768.0).collect();

        let mut cfg = DigiConfig::default();
        cfg.olivia_tones = 4;
        cfg.olivia_bw_hz = 125.0;
        cfg.olivia_reverse = true;
        let mut ctl = TextModemController::new(Mode::Contestia, cfg, spec.sample_rate as f64);
        ctl.set_audio_hz(1718.0);
        for chunk in audio.chunks(400) {
            ctl.on_rx_audio(chunk);
        }
        ctl.on_rx_audio(&vec![0.0f32; 16_000]);
        let text = ctl.status().text_rx;
        eprintln!("Contestia through the controller: {text:?}");
        for w in ["EAGLE", "SDRBROWN"] {
            assert!(text.contains(w), "the controller lost {w}: {text:?}");
        }
    }

    /// MT63's two 2000 Hz variants through the controller, which is the whole
    /// claim: a live receiver that has to find the symbol clock and the polarity
    /// by itself, fed the way the audio thread feeds it.
    #[test]
    #[ignore]
    fn the_mt63_corpus_samples_decode_through_the_controller() {
        for (mode, words) in [
            (sdroxide_types::Mt63Mode::Mt632KS, &["EAGLE", "LANDED", "SDRBROWN"][..]),
            (sdroxide_types::Mt63Mode::Mt632KL, &["SDRBROWN", "TEST"][..]),
        ] {
            let path = format!(
                "{}/../../samples/fldigi/MT63-{}.wav",
                env!("CARGO_MANIFEST_DIR"),
                mode.label()
            );
            let mut reader = hound::WavReader::open(&path).expect("the MT63 sample");
            let spec = reader.spec();
            let audio: Vec<f32> =
                reader.samples::<i16>().map(|s| s.expect("s") as f32 / 32_768.0).collect();
            let mut cfg = DigiConfig::default();
            cfg.mt63_mode = mode;
            let mut ctl = TextModemController::new(Mode::Mt63, cfg, spec.sample_rate as f64);
            ctl.set_audio_hz(1740.0);
            for chunk in audio.chunks(400) {
                ctl.on_rx_audio(chunk);
            }
            let text = ctl.status().text_rx;
            eprintln!("MT63-{} through the controller: {text:?}", mode.label());
            for w in words {
                assert!(text.contains(w), "MT63-{} lost {w}: {text:?}", mode.label());
            }
        }
    }

    /// ARDOP through the controller, which is the claim the mode is offered on:
    /// the tap is resampled to the modem's 12 kHz, the frames are found by the
    /// streaming scanner, and each arrives as a line naming the mode it came in.
    #[test]
    #[ignore]
    fn the_ardop_recordings_decode_through_the_controller() {
        for (file, want_type) in [
            ("4psk-1car_known-text.wav", "4PSK"),
            ("16qam-8car_known-text.wav", "16QAM"),
            ("4fsk-100bd_known-text.wav", "4FSK"),
        ] {
            let path = format!("{}/../../samples/ardop/{file}", env!("CARGO_MANIFEST_DIR"));
            let mut reader = hound::WavReader::open(&path).expect("an ARDOP recording");
            let spec = reader.spec();
            let audio: Vec<f32> =
                reader.samples::<i16>().map(|s| s.expect("s") as f32 / 32_768.0).collect();
            let mut ctl = TextModemController::new(
                Mode::Ardop,
                DigiConfig::default(),
                spec.sample_rate as f64,
            );
            for chunk in audio.chunks(600) {
                ctl.on_rx_audio(chunk);
            }
            let text = ctl.status().text_rx;
            eprintln!("ARDOP {file}: {text:?}");
            assert!(
                text.contains("EAGLE HAS LANDED 12345 SDRBROWN ZZZ"),
                "{file} lost its text through the controller: {text:?}"
            );
            // The line names the mode the frame arrived in — that is what tells
            // an operator whether the path was carrying 4PSK or 16QAM.
            assert!(text.contains(want_type), "{file} did not name its mode: {text:?}");
        }
    }

    /// The corpus sample through the controller, which is the claim the mode is
    /// offered on: the same object the console builds, fed the recording it was
    /// verified against, with only the centre, variant and polarity the journal
    /// measured.
    #[test]
    #[ignore]
    fn the_mfsk_corpus_sample_decodes_through_the_controller() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../samples/fldigi/MFSK16.wav");
        let mut reader = hound::WavReader::open(path).expect("the MFSK16 sample");
        let spec = reader.spec();
        let audio: Vec<f32> =
            reader.samples::<i16>().map(|s| s.expect("sample") as f32 / 32_768.0).collect();
        // Past the previous transmission's tail into this one's preamble.
        let skip = (13.0 * 512.0 / 8000.0 + 0.1) * spec.sample_rate as f64;
        let audio = &audio[(skip as usize).min(audio.len())..];

        let mut cfg = DigiConfig::default();
        cfg.mfsk_mode = sdroxide_types::MfskMode::Mfsk16;
        cfg.mfsk_reverse = true;
        let mut ctl = TextModemController::new(Mode::Mfsk, cfg, spec.sample_rate as f64);
        ctl.set_audio_hz(1752.0);
        for chunk in audio.chunks(400) {
            ctl.on_rx_audio(chunk);
        }
        ctl.on_rx_audio(&vec![0.0f32; 16_000]);
        let text = ctl.status().text_rx;
        eprintln!("MFSK16 through the controller: {text:?}");
        for w in ["TEST", "EAGLE", "12345", "SDRBROWN"] {
            assert!(text.contains(w), "the controller lost {w}: {text:?}");
        }
    }
}
