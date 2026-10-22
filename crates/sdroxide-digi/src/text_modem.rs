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
    MonoResampler, OliviaTx, PskMultiRx, PskRx, PskTx, PskrRx, RttyRx, RttyTx, ardop,
    dtmf::DtmfRx,
    ifk::{IfkRx, IfkTx, IfkVariant},
    mfsk_rx::{MfskReceiver, MfskTx, MfskVariant},
    mt63::{Mt63Receiver, Mt63Tx},
    stanag4285,
};
use sdroxide_olivia::OliviaReceiver;
use sdroxide_types::{DigiConfig, DigiStatus, Mode, PskCoded, QsoStep, TranscriptLine};

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

/// STANAG 4285's front end. The modem takes audio at the controller's own modem
/// rate and finds its 1800 Hz carrier in it, so it needs no resampler of its own —
/// but its report does need carrying, because the panel's job is to show what the
/// receiver measured rather than a lamp that lit.
struct StanagRx {
    rx: stanag4285::S4285Receiver,
    level: f32,
}

impl StanagRx {
    fn new(tap_rate: f64) -> Self {
        StanagRx {
            rx: stanag4285::S4285Receiver::new(
                tap_rate,
                sdroxide_types::Mode::Stanag4285.audio_centre_hz() as f64,
            ),
            level: 0.0,
        }
    }

    fn process(&mut self, audio: &[f32]) -> String {
        let mut out = String::new();
        for ev in self.rx.process(audio) {
            if let stanag4285::S4285Event::Text(t) = ev {
                out.push_str(&t);
            }
        }
        let rms = (audio.iter().map(|v| v * v).sum::<f32>() / audio.len().max(1) as f32).sqrt();
        self.level = (0.98 * self.level + 0.02 * (rms * 8.0).min(1.0)).min(1.0);
        out
    }

    /// A crude input level: the modem's own report says whether it is locked, and
    /// the squelch wants a number that follows the audio.
    fn magnitude(&self) -> f32 {
        self.level
    }

    fn reset(&mut self) {
        self.rx.reset();
    }

    fn report(&self) -> &stanag4285::S4285Report {
        self.rx.report()
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
    Psk(PskAny),
    Rtty(RttyRx),
    Olivia(OliviaReceiver),
    Thor(ThorRx),
    Domino(ThorRx),
    Dtmf(DtmfRx),
    Mfsk(MfskReceiver),
    Mt63(Mt63Receiver),
    Ardop(ArdopRx),
    Stanag(StanagRx),
}

/// PSK's three receivers behind one arm.
///
/// PSK31's family is one differential carrier with no code. fldigi's coded
/// families are three different receivers — `PskrRx` for the code (with one
/// carrier for BPSK63F and the `R` set, several for `RC`), `PskMultiRx` for the
/// `C` banks, which have carriers and **no** code — and the operator picks
/// between them with `DigiConfig::psk_coded`.
enum PskAny {
    Plain(PskRx),
    Coded(PskrRx),
    Multi(PskMultiRx),
}

impl PskAny {
    fn process(&mut self, audio: &[f32]) -> String {
        match self {
            PskAny::Plain(r) => r.process(audio),
            PskAny::Coded(r) => r.process(audio),
            PskAny::Multi(r) => r.process(audio),
        }
    }

    fn set_carrier(&mut self, hz: f64) {
        match self {
            PskAny::Plain(r) => r.set_carrier(hz),
            PskAny::Coded(r) => r.set_carrier(hz),
            PskAny::Multi(r) => r.set_carrier(hz),
        }
    }
}

impl RxModem {
    fn process(&mut self, audio: &[f32]) -> String {
        match self {
            RxModem::Psk(r) => r.process(audio),
            RxModem::Rtty(r) => r.process(audio),
            RxModem::Olivia(r) => r.process(audio),
            RxModem::Thor(r) => r.process(audio),
            RxModem::Domino(r) => r.process(audio),
            RxModem::Dtmf(r) => r.process(audio),
            RxModem::Mfsk(r) => r.process(audio),
            RxModem::Mt63(r) => r.process(audio),
            RxModem::Ardop(r) => r.process(audio),
            RxModem::Stanag(r) => r.process(audio),
        }
    }
    fn magnitude(&self) -> f32 {
        match self {
            // The coded receivers do not publish a level; the controller's
            // squelch reads this, and PSK is self-gating anyway.
            RxModem::Psk(_) => 1.0,
            RxModem::Rtty(r) => r.magnitude(),
            RxModem::Olivia(r) => r.magnitude(),
            RxModem::Thor(r) => r.magnitude(),
            RxModem::Domino(r) => r.magnitude(),
            RxModem::Dtmf(r) => r.magnitude(),
            RxModem::Mfsk(r) => r.magnitude(),
            RxModem::Mt63(r) => r.magnitude(),
            RxModem::Ardop(r) => r.magnitude(),
            RxModem::Stanag(r) => r.magnitude(),
        }
    }
}

enum TxModem {
    Psk(PskTx),
    Rtty(RttyTx),
    Olivia(OliviaTx),
    Thor(IfkTx),
    Mt63(Mt63Tx),
    Mfsk(MfskTx),
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
            TxModem::Mfsk(m) => m.push_text(t),
            TxModem::Mt63(m) => m.push_text(t),
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
            TxModem::Mfsk(m) => {
                m.next_block(out);
            }
            TxModem::Mt63(m) => {
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
            TxModem::Mfsk(m) => m.sent_chars(),
            TxModem::Mt63(m) => m.sent_chars(),
            TxModem::Receive => 0,
        }
    }
    fn total_chars(&self) -> usize {
        match self {
            TxModem::Psk(m) => m.total_chars(),
            TxModem::Rtty(m) => m.total_chars(),
            TxModem::Olivia(m) => m.total_chars(),
            TxModem::Thor(m) => m.total_chars(),
            TxModem::Mfsk(m) => m.total_chars(),
            TxModem::Mt63(m) => m.total_chars(),
            TxModem::Receive => 0,
        }
    }
    fn clear(&mut self) {
        match self {
            TxModem::Psk(m) => m.clear(),
            TxModem::Rtty(m) => m.clear(),
            TxModem::Olivia(m) => m.clear(),
            TxModem::Thor(m) => m.clear(),
            TxModem::Mfsk(m) => m.clear(),
            TxModem::Mt63(m) => m.clear(),
            TxModem::Receive => {}
        }
    }
}

/// THOR's and DominoEX's receiver: the IFK port in `sdroxide-dsp::ifk`, which is
/// the one that reads the corpus.
///
/// It replaces a second, earlier IFK receiver that lived in `sdroxide-dsp::thor`
/// and read nothing at all — every THOR corpus sample, in every variant, both
/// polarities, five centres. Two implementations of one waveform is how the
/// working one stayed a test; this is the one the journal's measurements are on.
///
/// One adapter serves both modes because they are one front end: they differ only
/// in what the steps mean, which is `IfkRx`'s own business.
struct ThorRx {
    rx: IfkRx,
    variant: IfkVariant,
    centre: f64,
    rate: f64,
    emitted: usize,
}

impl ThorRx {
    fn new(rate: f64, audio_hz: f64, variant: IfkVariant) -> Self {
        ThorRx {
            rx: IfkRx::new(rate, audio_hz, variant),
            variant,
            centre: audio_hz,
            rate,
            emitted: 0,
        }
    }

    /// Retune, or change variant. The bank is built once around a centre, so a
    /// move **rebuilds** it — leaving the old bank in place and only moving a
    /// number is how a receiver listens to 1000 Hz while the operator looks at
    /// 1600, and the corruption reads exactly like a wrong convention.
    fn set_params(&mut self, audio_hz: f64, variant: IfkVariant) {
        if variant.name != self.variant.name || (audio_hz - self.centre).abs() > 0.5 {
            self.variant = variant;
            self.rx = IfkRx::new(self.rate, audio_hz, variant);
        }
        self.centre = audio_hz;
        self.emitted = 0;
    }

    fn set_reverse(&mut self, on: bool) {
        self.rx.reverse = on;
    }

    fn magnitude(&self) -> f32 {
        self.rx.snr_db
    }

    fn process(&mut self, audio: &[f32]) -> String {
        self.rx.feed(audio);
        let text = self.rx.text();
        if self.emitted >= text.len() {
            return String::new();
        }
        let out = text[self.emitted..].to_string();
        self.emitted = text.len();
        out
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
            Mode::Thor => {
                let mut r = ThorRx::new(
                    MODEM_RATE,
                    audio_hz as f64,
                    IfkVariant::named(cfg.thor_mode.label()).expect("the THOR tables agree"),
                );
                r.set_reverse(cfg.thor_reverse);
                RxModem::Thor(r)
            }
            Mode::DominoEx => {
                // The same front end as THOR, and the same polarity control: IFK+
                // carries its data in the step, so a station received on the other
                // sideband has every step negative.
                let mut r = ThorRx::new(
                    MODEM_RATE,
                    audio_hz as f64,
                    IfkVariant::named(cfg.domino_mode.label()).expect("the DominoEX tables agree"),
                );
                r.set_reverse(cfg.thor_reverse);
                RxModem::Domino(r)
            }
            Mode::Mfsk => RxModem::Mfsk(MfskReceiver::new(
                MODEM_RATE,
                audio_hz as f64,
                cfg.mfsk_mode,
                cfg.mfsk_reverse,
            )),
            Mode::Mt63 => {
                RxModem::Mt63(Mt63Receiver::new(MODEM_RATE, audio_hz as f64, cfg.mt63_mode.into()))
            }
            // DTMF's eight tones are absolute in the passband, so there is no
            // centre to tune: whatever the operator's cursor is doing, a digit is
            // two tones where they are.
            Mode::Dtmf => RxModem::Dtmf(DtmfRx::new(MODEM_RATE)),
            Mode::Ardop => RxModem::Ardop(ArdopRx::new(tap_rate)),
            Mode::Stanag4285 => RxModem::Stanag(StanagRx::new(MODEM_RATE)),
            _ => {
                // A coded variant is a different waveform, not a setting on
                // PSK31's modem: it gets its own receiver, built from the
                // table in `PskCoded`.
                let coded = cfg.psk_coded;
                if coded != PskCoded::Off {
                    let any = if coded.fec() {
                        PskAny::Coded(PskrRx::with_carriers(
                            MODEM_RATE,
                            audio_hz as f64,
                            coded.baud(),
                            coded.carriers(),
                            coded.depth(),
                        ))
                    } else {
                        PskAny::Multi(PskMultiRx::new(
                            MODEM_RATE,
                            audio_hz as f64,
                            coded.baud(),
                            coded.carriers(),
                        ))
                    };
                    RxModem::Psk(any)
                } else {
                    let mut r = PskRx::new_baud(MODEM_RATE, audio_hz as f64, p_baud);
                    r.set_qpsk(p_qpsk);
                    RxModem::Psk(PskAny::Plain(r))
                }
            }
        };
        let tx = match mode {
            Mode::Rtty | Mode::RttyFm => {
                TxModem::Rtty(RttyTx::new(MODEM_RATE, audio_hz as f64, baud, shift))
            }
            Mode::Olivia => {
                TxModem::Olivia(OliviaTx::new(MODEM_RATE, audio_hz as f64, o_tones, o_bw))
            }
            // Contestia is Olivia's engine with a different scrambler, a 32-symbol
            // block and a five-bit shift. It emits the opposite Walsh sign, for
            // the reason spelled out on `OliviaTx::contestia`.
            Mode::Contestia => {
                TxModem::Olivia(OliviaTx::contestia(MODEM_RATE, audio_hz as f64, o_tones, o_bw))
            }
            Mode::Thor => TxModem::Thor(IfkTx::new(
                IfkVariant::named(cfg.thor_mode.label()).expect("the THOR tables agree"),
                MODEM_RATE,
                audio_hz as f64,
            )),
            // MFSK is one modem at eleven symbol rates, so the transmitter takes
            // the variant the config names, exactly as the receiver does.
            Mode::Mfsk => TxModem::Mfsk(MfskTx::new(
                MfskVariant::named(cfg.mfsk_mode.label()).expect("the MFSK tables agree"),
                MODEM_RATE,
                audio_hz as f64,
            )),
            Mode::Mt63 => {
                TxModem::Mt63(Mt63Tx::new(cfg.mt63_mode.into(), MODEM_RATE, audio_hz as f64))
            }
            Mode::Ardop | Mode::Stanag4285 => TxModem::Receive,
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
        let p_baud = self.cfg.psk_speed.baud();
        let p_qpsk = self.cfg.psk_modulation == sdroxide_types::PskModulation::Qpsk;
        match &mut self.rx {
            RxModem::Psk(r) => {
                r.set_carrier(hz);
                if let PskAny::Plain(p) = r {
                    p.set_baud(p_baud);
                    p.set_qpsk(p_qpsk);
                }
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
            RxModem::Thor(r) => {
                r.set_params(
                    hz,
                    IfkVariant::named(self.cfg.thor_mode.label()).expect("the THOR tables agree"),
                );
                r.set_reverse(self.cfg.thor_reverse);
            }
            // DTMF tunes itself: its frequencies do not move with the cursor.
            RxModem::Dtmf(_) => {}
            RxModem::Domino(r) => {
                r.set_params(
                    hz,
                    IfkVariant::named(self.cfg.domino_mode.label())
                        .expect("the DominoEX tables agree"),
                );
                r.set_reverse(self.cfg.thor_reverse);
            }
            RxModem::Mfsk(r) => {
                r.set_mirror(self.cfg.mfsk_reverse);
                r.set_params(hz, self.cfg.mfsk_mode);
            }
            RxModem::Mt63(r) => r.set_params(hz, self.cfg.mt63_mode.into()),
            RxModem::Ardop(r) => r.reset(),
            // The carrier is the standard's own 1800 Hz, so a retune of the
            // operator's cursor changes nothing inside the modem; a reset is
            // what a new signal needs.
            RxModem::Stanag(r) => r.reset(),
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
            TxModem::Thor(m) => m.set_params(
                hz,
                IfkVariant::named(self.cfg.thor_mode.label()).expect("the THOR tables agree"),
            ),
            TxModem::Mfsk(m) => m.set_params(
                hz,
                MfskVariant::named(self.cfg.mfsk_mode.label()).expect("the MFSK tables agree"),
            ),
            TxModem::Mt63(m) => m.set_params(hz, self.cfg.mt63_mode.into()),
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
            tx_refused: tx_refusal_text(self.mode, &self.cfg),
            transcript: Vec::<TranscriptLine>::new(),
            config: self.cfg.clone(),
            text_rx: self.rx_text.clone(),
            tx_sent: self.tx.sent_chars(),
            fsq_heard: Vec::new(),
            fsq_messages: Vec::new(),
            stanag: match &self.rx {
                RxModem::Stanag(r) => {
                    let rep = r.report();
                    Some(sdroxide_types::StanagStatus {
                        locked: rep.locked,
                        sync: rep.sync_quality,
                        sync_null: rep.correlation_floor,
                        frame_acf: rep.frame_acf,
                        reference: rep.reference,
                        constellation: rep.constellation,
                        // The coded rate wins when the start-of-message pattern
                        // has named one: the constellation cannot tell the four
                        // BPSK rates apart, and a decode can.
                        rate: rep
                            .coded
                            .map(|c| c.label().to_string())
                            .unwrap_or_else(|| rep.rate.label().to_string()),
                        offset_hz: rep.carrier_offset_hz,
                        frames: rep.frames,
                        messages: rep.messages,
                        best_som: rep.best_som,
                    })
                }
                _ => None,
            },
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

/// Why this configuration cannot key up, if it cannot.
///
/// Two things answer: a mode that is receive-only by capability, and PSK
/// set to one of fldigi's **coded** families, whose transmitters are not
/// written. The second is a *setting* rather than a mode, and refusing it is
/// the difference between a control that does nothing and one that says so —
/// silently sending PSK31 to an operator who selected PSK125R would put the
/// wrong waveform on the air.
fn tx_refusal_text(mode: Mode, cfg: &DigiConfig) -> Option<String> {
    if mode.is_rx_only() {
        return Some(match mode {
            Mode::Ardop => "ARDOP is receive only in this build — the modem has no \
                            transmitter and the ARQ state machine is not ported"
                .to_string(),
            Mode::Stanag4285 => "STANAG 4285 is receive only in this build — there is no \
                                 modulator, and the coded rates need Annex E, which is not \
                                 transcribed"
                .to_string(),
            other => format!("{} is receive only in this build", other.label()),
        });
    }
    if mode == Mode::Psk && cfg.psk_coded != PskCoded::Off {
        return Some(format!(
            "{} is receive only in this build — the coded PSK transmitter is not written",
            cfg.psk_coded.label()
        ));
    }
    None
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
            || cfg.thor_reverse != self.cfg.thor_reverse
            || cfg.domino_mode != self.cfg.domino_mode
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
        if on && tx_refusal_text(self.mode, &self.cfg).is_some() {
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

    /// **Contestia transmits.** The receiver has decoded Contestia for years and
    /// the transmitter did not exist, so the mode was receive-only for a reason
    /// that was not principled: Contestia is Olivia's engine with six bits per
    /// character, a 32-symbol block and the CRC-32 scrambler. This drives the
    /// console's own transmitter into the console's own receiver — the same
    /// objects the band menu builds — and reads the text back.
    ///
    /// **The fault was the Walsh-domain sign, and it is Olivia's opposite here
    /// on purpose.** Contestia is decoded by the vendored Jalocha engine, which
    /// is fldigi's own code: it reads a *negative* peak as the upper half of the
    /// alphabet (`if (Peak < 0) Char += SymbolsPerBlock`), so a transmitter that
    /// follows fldigi's `EncodeBlock` (`FHT_Buffer < 0`) is what it expects.
    /// Olivia's receiver is this tree's own and was measured on an off-air
    /// capture to vote the other way, so the two modes cannot share one sign.
    /// Emitting Olivia's polarity decoded every Contestia character as its
    /// bit-5 twin — idle, in the console — for as long as the transmitter
    /// existed.
    #[test]
    fn a_contestia_transmission_reads_back_through_the_controller() {
        use sdroxide_dsp::OliviaTx;
        let (rate, audio) = (8000.0, 1500.0);
        // The controller builds its receiver from `DigiConfig`, so the
        // transmitter has to speak the geometry the config asks for — a
        // mismatch here is a test that fails for a reason unrelated to the
        // scrambler, which is what the first version of this did.
        let cfg = DigiConfig::default();
        let (tones, bw) = (cfg.olivia_tones as usize, cfg.olivia_bw_hz as f64);
        let mut tx = OliviaTx::contestia(rate, audio, tones, bw);
        assert!(tx.is_contestia(), "the transmitter is not in Contestia mode");
        let mut sig = Vec::new();
        // A runway so the receiver acquires timing and block alignment, the way
        // the Olivia loopback does.
        let mut warm = vec![0.0f32; 20_000];
        tx.next_block(&mut warm);
        sig.extend_from_slice(&warm);
        tx.push_text("THE EAGLE HAS LANDED");
        let mut guard = 0;
        while tx.sent_chars() < tx.total_chars() && guard < 40_000 {
            let mut b = [0.0f32; 2048];
            tx.next_block(&mut b);
            sig.extend_from_slice(&b);
            guard += 1;
        }
        // Four blocks of tail: the receiver integrates `SyncIntegLen` FEC blocks,
        // so a transmission that stops the instant its last character is out is
        // decoded only up to the block before it.
        let mut tail = vec![0.0f32; 48_000];
        tx.next_block(&mut tail);
        sig.extend_from_slice(&tail);

        let mut ctl = TextModemController::new(Mode::Contestia, cfg, rate);
        for chunk in sig.chunks(512) {
            ctl.on_rx_audio(chunk);
        }
        let text = ctl.status().text_rx.to_uppercase();
        assert!(
            text.contains("EAGLE") && text.contains("LANDED"),
            "the Contestia transmission did not read back: {text:?}"
        );
    }

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

    /// THOR's transmitter into its receiver, through the console's own objects.
    ///
    /// Both halves were replaced: the receiver with the IFK port that reads the
    /// corpus, and the transmitter with fldigi's own `sendchar` path. The pair it
    /// replaces agreed only with each other — the old transmitter's module notes
    /// admitted its interleaving was "not yet bit-matched to fldigi", so it could
    /// not have been read by a real station either.
    #[test]
    fn a_thor_transmission_reads_back_through_the_controller() {
        use sdroxide_types::ThorMode;
        let rate = 8000.0;
        let mut cfg = DigiConfig::default();
        cfg.thor_mode = ThorMode::Thor16;
        let mut tx_ctl = TextModemController::new(Mode::Thor, cfg.clone(), rate);
        tx_ctl.set_audio_hz(1500.0);
        tx_ctl.set_tx_text("TEST THE EAGLE HAS LANDED".into());
        tx_ctl.set_tx_active(true);
        tx_ctl.poll(SystemTime::now(), 0.0);
        let mut sig = Vec::new();
        let mut block = vec![0.0f32; 512];
        let mut guard = 0;
        while !tx_ctl.fill_tx_block(&mut block) && guard < 100_000 {
            sig.extend_from_slice(&block);
            guard += 1;
        }
        sig.extend_from_slice(&block);
        let mut rx_ctl = TextModemController::new(Mode::Thor, cfg, OUT_RATE);
        rx_ctl.set_audio_hz(1500.0);
        for chunk in sig.chunks(512) {
            rx_ctl.on_rx_audio(chunk);
        }
        let text = rx_ctl.status().text_rx.to_uppercase();
        assert!(
            text.contains("EAGLE") && text.contains("LANDED"),
            "the THOR transmission did not read back: {text:?}"
        );
    }

    /// The mode tables and the decoder's table must agree, by label.
    ///
    /// `DigiConfig` carries an enum and the decoder carries the protocol
    /// constants; they are tied by the string in between, so a spelling change on
    /// either side silently retunes the receiver to a variant that does not exist
    /// — or worse, to a different one.
    #[test]
    fn the_ifk_mode_labels_are_the_decoders() {
        for m in sdroxide_types::DominoMode::ALL {
            let v = IfkVariant::named(m.label())
                .unwrap_or_else(|| panic!("no decoder variant for {}", m.label()));
            assert_eq!(v.name, m.label(), "the decoder spells {} differently", m.label());
            assert!(!v.thor, "{} resolved to a THOR variant", m.label());
        }
        for m in sdroxide_types::ThorMode::ALL {
            let v = IfkVariant::named(m.label())
                .unwrap_or_else(|| panic!("no decoder variant for {}", m.label()));
            assert_eq!(v.name, m.label(), "the decoder spells {} differently", m.label());
            assert!(v.thor, "{} resolved to a DominoEX variant", m.label());
        }
    }

    /// DTMF through the console's own controller: a key press, decoded.
    ///
    /// The decoder's own tests cover the alphabet and the noise floor; what this
    /// pins is the wiring — that the mode is a text modem at all, that its tones
    /// are absolute in the passband (so a cursor anywhere decodes them), and that
    /// one press is one character rather than one per frame.
    #[test]
    fn a_dtmf_key_press_reads_through_the_controller() {
        let rate = 8000.0f64;
        let mut ctl = TextModemController::new(Mode::Dtmf, DigiConfig::default(), rate);
        // The operator's cursor is nowhere near the tones and it must not matter.
        ctl.set_audio_hz(2100.0);
        let mut audio: Vec<f32> = vec![0.0f32; 4000];
        for (row, col) in [(697.0f64, 1209.0f64), (852.0, 1477.0)] {
            for k in 0..((rate * 0.1) as usize) {
                let t = k as f64 / rate;
                let v = (std::f64::consts::TAU * row * t).sin()
                    + (std::f64::consts::TAU * col * t).sin();
                audio.push((0.4 * v) as f32);
            }
            audio.extend(std::iter::repeat(0.0f32).take((rate * 0.06) as usize));
        }
        for chunk in audio.chunks(256) {
            ctl.on_rx_audio(chunk);
        }
        let text = ctl.status().text_rx.clone();
        assert_eq!(text, "19", "the key presses decoded as {text:?}");
    }

    /// DominoEX through the console's own controller — the mode as its own
    /// choice rather than a THOR variant.
    ///
    /// `DOMX11` reads its whole corpus message; the chip row is nine variants of
    /// one modem, and `the_domino_mode_labels_are_the_decoders` pins the names
    /// against the decoder's own table so a spelling cannot silently retune it.
    #[test]
    #[ignore]
    fn the_dominoex_corpus_sample_decodes_through_the_controller() {
        use sdroxide_types::DominoMode;
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../samples/fldigi/DOMX11.wav");
        let mut reader = hound::WavReader::open(path).expect("the DOMX11 sample");
        let spec = reader.spec();
        let audio: Vec<f32> =
            reader.samples::<i16>().map(|s| s.expect("s") as f32 / 32_768.0).collect();
        let mut cfg = DigiConfig::default();
        cfg.domino_mode = DominoMode::Dom11;
        cfg.thor_reverse = true;
        let mut ctl = TextModemController::new(Mode::DominoEx, cfg, spec.sample_rate as f64);
        ctl.set_audio_hz(1730.0);
        for chunk in audio.chunks(512) {
            ctl.on_rx_audio(chunk);
        }
        let text = ctl.status().text_rx.to_uppercase();
        eprintln!("DOMX11 through the controller: {text:?}");
        assert!(
            text.contains("EAGLE") && text.contains("LANDED") && text.contains("SDRBROWN"),
            "the DOMX11 sample did not read back: {text:?}"
        );
    }

    /// A coded PSK mode through the console's own controller: PSK125R, the
    /// single-carrier robust one, on its corpus sample.
    ///
    /// The receivers were proven against the corpus in `sdroxide-dsp`; what this
    /// pins is the wiring — `DigiConfig::psk_coded` reaching the right receiver
    /// at the right baud, carrier count and interleave depth — and that a coded
    /// selection **refuses the key-up by name** instead of quietly sending
    /// PSK31, which is the failure mode a setting rather than a mode invites.
    #[test]
    #[ignore]
    fn a_coded_psk_sample_decodes_through_the_controller() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../samples/fldigi/PSK125R.wav");
        let mut reader = hound::WavReader::open(path).expect("the PSK125R sample");
        let spec = reader.spec();
        let audio: Vec<f32> =
            reader.samples::<i16>().map(|s| s.expect("s") as f32 / 32_768.0).collect();
        let mut cfg = DigiConfig::default();
        cfg.psk_coded = PskCoded::Psk125R;
        let mut ctl = TextModemController::new(Mode::Psk, cfg, spec.sample_rate as f64);
        ctl.set_audio_hz(1754.0);
        for chunk in audio.chunks(512) {
            ctl.on_rx_audio(chunk);
        }
        let text = ctl.status().text_rx.to_uppercase();
        eprintln!("PSK125R through the controller: {text:?}");
        assert!(
            text.contains("EAGLE") && text.contains("LANDED") && text.contains("SDRBROWN"),
            "the PSK125R sample did not read back: {text:?}"
        );
    }

    /// A coded PSK selection cannot key up, and says which one it is.
    #[test]
    fn a_coded_psk_selection_refuses_the_key_by_name() {
        let mut cfg = DigiConfig::default();
        cfg.psk_coded = PskCoded::Psk63Rc4;
        let mut c = TextModemController::new(Mode::Psk, cfg, 48_000.0);
        c.set_tx_text("TEST".into());
        c.set_tx_active(true);
        c.poll(SystemTime::now(), 0.0);
        let s = c.status();
        assert!(!s.tx_next, "a coded PSK selection armed the transmitter");
        assert!(!c.tx_burst_active(), "it asked the engine to key");
        let reason = s.tx_refused.expect("it refused in silence");
        assert!(reason.contains("PSK63RC4"), "the refusal does not name the variant: {reason:?}");

        // Plain PSK still keys, so this is not a blanket.
        let mut plain = TextModemController::new(Mode::Psk, DigiConfig::default(), 48_000.0);
        plain.set_tx_active(true);
        plain.poll(SystemTime::now(), 0.0);
        assert!(plain.status().tx_refused.is_none(), "plain PSK picked up the refusal");
        assert!(plain.tx_burst_active(), "plain PSK stopped transmitting");
    }

    /// THOR through the console's own controller — the gate that says the program
    /// now reads it.
    ///
    /// The corpus sample is the one at 1600 Hz with the steps reversed; the
    /// decoder is `sdroxide_dsp::ifk::IfkRx`, which reads eleven of the fifteen
    /// variants. Before this, `Mode::Thor` was wired to a second IFK receiver
    /// that read nothing at all — the journal's "0/4, it has never read a real
    /// signal" was measured on that one.
    #[test]
    #[ignore]
    fn the_thor_corpus_sample_decodes_through_the_controller() {
        use sdroxide_types::ThorMode;
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../samples/fldigi/THOR16.wav");
        let mut reader = hound::WavReader::open(path).expect("the THOR16 sample");
        let spec = reader.spec();
        let audio: Vec<f32> =
            reader.samples::<i16>().map(|s| s.expect("s") as f32 / 32_768.0).collect();
        let mut cfg = DigiConfig::default();
        cfg.thor_mode = ThorMode::Thor16;
        cfg.thor_reverse = true;
        let mut ctl = TextModemController::new(Mode::Thor, cfg, spec.sample_rate as f64);
        ctl.set_audio_hz(1600.0);
        for chunk in audio.chunks(512) {
            ctl.on_rx_audio(chunk);
        }
        let text = ctl.status().text_rx.to_uppercase();
        eprintln!("THOR16 through the controller: {text:?}");
        assert!(
            text.contains("EAGLE") && text.contains("LANDED") && text.contains("SDRBROWN"),
            "the THOR16 sample did not read back: {text:?}"
        );
    }

    /// MT63 keys the transmitter and produces the audio it should.
    ///
    /// The modulator is gated at the modem level (`a_synthetic_mt63_transmission_reads_back`,
    /// decoded at the true symbol phase). What this adds is the wiring: the mode
    /// is no longer receive-only, the controller arms it, and audio comes out at
    /// the length a transmission takes — 20 baud at 2 kHz, so a 24-character
    /// message plus the idle either side is a few seconds of it.
    #[test]
    fn mt63_keys_the_transmitter() {
        assert!(!Mode::Mt63.is_rx_only(), "MT63 is still receive only");
        let mut c = TextModemController::new(Mode::Mt63, DigiConfig::default(), 48_000.0);
        c.set_tx_text("TEST THE EAGLE HAS LANDED".into());
        c.set_tx_active(true);
        c.poll(SystemTime::now(), 0.0);
        assert!(c.status().tx_refused.is_none(), "it refused the key-up");
        assert!(c.tx_burst_active(), "it did not arm the transmitter");
        let mut block = vec![0.0f32; 512];
        let mut peak = 0.0f32;
        let mut samples = 0usize;
        let mut guard = 0;
        while !c.fill_tx_block(&mut block) && guard < 100_000 {
            peak = block.iter().fold(peak, |m, s| m.max(s.abs()));
            samples += block.len();
            guard += 1;
        }
        assert!(peak > 0.01, "the transmitter produced silence (peak {peak})");
        assert!(
            samples > 48_000,
            "only {samples} samples came out — too short to be a transmission"
        );
    }

    /// ARDOP is the modem only — no modulator and no ARQ — so the controller must
    /// refuse the key-up **and say why**: a mode that greys nothing and keys
    /// silence is the fault the house rules name. The modes that do transmit must
    /// not pick up the refusal, which is what stops this being a blanket.
    #[test]
    fn a_receive_only_mode_refuses_the_key_and_says_why() {
        let mut ardop = TextModemController::new(Mode::Ardop, DigiConfig::default(), 48_000.0);
        ardop.set_tx_text("TEST THE EAGLE".into());
        ardop.set_tx_active(true);
        let actions = ardop.poll(SystemTime::now(), 0.0);
        let s = ardop.status();
        assert!(!s.tx_next, "a receive-only mode armed the transmitter");
        assert!(!ardop.tx_burst_active(), "it asked the engine to key");
        assert!(
            !actions.iter().any(|a| matches!(a, DigiAction::KeyTx)),
            "it sent KeyTx to the engine"
        );
        assert!(s.tx_refused.is_some(), "it refused in silence");
        assert!(
            s.tx_refused.as_deref().unwrap_or("").contains("ARDOP"),
            "the refusal does not name the mode: {:?}",
            s.tx_refused
        );

        let mut psk = TextModemController::new(Mode::Psk, DigiConfig::default(), 48_000.0);
        psk.set_tx_active(true);
        psk.poll(SystemTime::now(), 0.0);
        assert!(psk.status().tx_refused.is_none(), "PSK picked up the refusal");
        assert!(psk.tx_burst_active(), "PSK stopped transmitting");
    }

    /// MFSK transmits, end to end through the controller: the modem's audio, its
    /// resampler to the output rate, and back in through the controller's own
    /// receiver. The direct `MfskTx` loopback in `sdroxide-dsp` proves the modem;
    /// this proves the wiring — the variant the config names reaching both halves,
    /// and the resample stage in between.
    #[test]
    fn an_mfsk_transmission_reads_back_through_the_controller() {
        use sdroxide_types::MfskMode;
        let mut cfg = DigiConfig::default();
        cfg.mfsk_mode = MfskMode::Mfsk16;
        let (rate, audio) = (8000.0, 1500.0);
        let mut tx_ctl = TextModemController::new(Mode::Mfsk, cfg.clone(), rate);
        tx_ctl.set_audio_hz(audio);
        tx_ctl.set_tx_text("TEST THE EAGLE HAS LANDED".into());
        tx_ctl.set_tx_active(true);
        tx_ctl.poll(SystemTime::now(), 0.0);
        assert!(tx_ctl.tx_burst_active(), "MFSK did not arm the transmitter");

        let mut sig = Vec::new();
        let mut block = vec![0.0f32; 512];
        let mut guard = 0;
        while !tx_ctl.fill_tx_block(&mut block) && guard < 200_000 {
            sig.extend_from_slice(&block);
            guard += 1;
        }
        sig.extend_from_slice(&block);
        // The controller emits at the rig's rate, so the receiving side is built
        // for a 48 kHz tap: that is the real chain, resamplers and all, rather
        // than a rate the modem chose for itself.
        let mut rx_ctl = TextModemController::new(Mode::Mfsk, cfg, OUT_RATE);
        rx_ctl.set_audio_hz(audio);
        for chunk in sig.chunks(512) {
            rx_ctl.on_rx_audio(chunk);
        }
        let text = rx_ctl.status().text_rx.to_uppercase();
        assert!(
            text.contains("EAGLE") && text.contains("LANDED"),
            "the MFSK transmission did not read back: {text:?}"
        );
    }

    /// Contestia keys the transmitter, and it is the only mode that changed
    /// sides: the panel greyed its sending half for as long as the Walsh sign was
    /// Olivia's, so this pins both the capability and the controller.
    #[test]
    fn contestia_keys_the_transmitter() {
        assert!(!Mode::Contestia.is_rx_only(), "Contestia is still receive only");
        let mut c = TextModemController::new(Mode::Contestia, DigiConfig::default(), 48_000.0);
        c.set_tx_text("TEST".into());
        c.set_tx_active(true);
        let actions = c.poll(SystemTime::now(), 0.0);
        assert!(c.status().tx_refused.is_none(), "it refused the key-up");
        assert!(c.tx_burst_active(), "it did not arm the transmitter");
        assert!(
            actions.iter().any(|a| matches!(a, DigiAction::KeyTx)),
            "no KeyTx reached the engine"
        );
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

    /// The 4285 receiver's report reaches the status even with nothing to lock on,
    /// and the mode refuses the key-up with a reason rather than silently.
    ///
    /// A panel that shows a lamp and no numbers cannot be checked against the air,
    /// which is why the report is on the wire at all — and a receive-only mode
    /// that produced no reason would leave the sending half dead with no
    /// explanation, the thing this house does not ship.
    #[test]
    fn the_stanag_report_reaches_the_status_even_with_nothing_to_lock_on() {
        let mut ctl = TextModemController::new(Mode::Stanag4285, DigiConfig::default(), 48_000.0);
        for _ in 0..40 {
            ctl.on_rx_audio(&vec![0.0f32; 960]);
        }
        let st = ctl.status();
        let s = st.stanag.expect("the 4285 report is not on the status");
        assert!(!s.locked, "silence locked");
        assert!(s.frames == 0, "silence produced {} frames", s.frames);
        assert!(
            st.tx_refused.as_deref().unwrap_or_default().contains("Annex E"),
            "the receive-only reason does not name the coded rates: {:?}",
            st.tx_refused
        );
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
