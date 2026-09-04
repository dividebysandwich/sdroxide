//! Two radios behaving as one: a transceiver that is controlled and keyed, and
//! a second receiver that supplies the spectrum.
//!
//! The arrangement this exists for is a station whose transceiver has no
//! wideband output — CAT on a serial port, demodulated audio on a sound card —
//! beside an SDR fed from the same antenna or from the rig's I.F. socket. On its
//! own the rig can only show a slice of its audio band mapped to RF; with the
//! SDR's I/Q underneath it, the same tab gets a real panadapter, a waterfall, a
//! sub receiver, digital modes and a skimmer, while every control that matters —
//! the dial, the mode, the filter, PTT, the keyer — stays with the transceiver.
//!
//! # Why a source rather than a link between two engines
//!
//! [`IqSource`] already separates *where the samples come from* from *what
//! drives the radio*: `poll_control`, `set_control_mode`, `set_control_filter`,
//! `rx_signal_dbm`, the CW text keyer and the whole transmit half are there for
//! front ends that are transceivers. Nothing says both halves have to be the
//! same device. Implementing the trait across two of them means the engine, and
//! therefore the panadapter's pan and zoom bounds, the sub-receiver clamp,
//! `keep_vfo_in_span`, every remote client and the browser, all see an ordinary
//! wideband radio — with no code anywhere else that knows about the pairing.
//!
//! The consequence is that the attached receiver's device belongs to this
//! source. It cannot also be running as a radio of its own, which is why
//! `boot_radios` skips it.
//!
//! # The frequency algebra
//!
//! One number: `off`, the offset the receiver sits above the dial, which
//! [`PanadapterConfig::offset_for`] may make depend on the mode. Everything the
//! engine is told is on the *dial's* scale and everything the receiver is told
//! is on its own:
//!
//! - `center_hz()` reports `rx.center_hz() - off` — where the receiver's centre
//!   lands on the dial. Reported from what the receiver actually achieved, so a
//!   front end that can only tune in 1 kHz steps needs no correction here: the
//!   engine's DDC absorbs the remainder as it does for any other radio.
//! - `set_center_hz(hz)` tunes the receiver to `hz + off`.
//! - `set_if_offset(o)` is how the transceiver's dial moves. The engine pushes
//!   `vfo - center` through it on every retune, which is the one place the VFO
//!   itself can be reconstructed — `set_center_hz` only ever says where the span
//!   should sit.
//!
//! With an I.F. tap the receiver is not really being tuned at all: it watches a
//! fixed intermediate frequency while the rig's own first oscillator moves the
//! band underneath it. That falls out of the same arithmetic — `off` changes
//! with the dial by construction — so it needs no separate mode here, only an
//! offset the size of an I.F. and, on rigs whose carrier moves with the mode, a
//! per-mode override.

use std::collections::VecDeque;

use sdroxide_radio::{Complex32, ControlUpdate, IqSource, Result};
use sdroxide_types::{DeviceCaps, Mode, PanadapterAudio, PanadapterConfig};
use tracing::{info, warn};

/// How much transceiver audio may queue up before the oldest is dropped.
///
/// The two radios run off different clocks — a sound card and an SDR — and
/// nothing disciplines one against the other, so the backlog between them
/// wanders. A tenth of a second is long enough to ride out the jitter of one
/// engine block against one capture period, and short enough that the audio
/// stays in step with the picture beside it.
const AUDIO_QUEUE_MS: f64 = 100.0;

/// Longest receive read taken while transmitting, in milliseconds of samples.
///
/// Only reached where the operator asked for the display to keep running
/// through an over. During transmit the engine owes the transmitter a block
/// every few milliseconds and whatever a receive call spends comes out of that
/// budget, so the request is bounded rather than trusted to return promptly:
/// [`IqSource::read_available`] is documented as non-blocking but only the
/// SoapySDR backend actually implements it, and everything else falls through
/// to a `read` that fills what it is given.
const TX_READ_MS: f64 = 2.0;

/// A transceiver and a second receiver, presented to the engine as one radio.
pub struct PanadapterSource {
    /// The attached receiver: I/Q, the spectrum, and the receive front end's
    /// own gains and antenna.
    rx: Box<dyn IqSource>,
    /// The transceiver: the dial, the mode, the filter, the transmitter, the
    /// keyer, its S-meter — and its audio, when that is what is being heard.
    ctrl: Box<dyn IqSource>,
    cfg: PanadapterConfig,
    /// The mode the transceiver is in, which picks the offset. `None` until
    /// somebody says — the engine asserts the operator's mode as soon as the
    /// source is established ([`IqSource::tracks_rx_mode`]), and a rig reports
    /// its own on its first poll — because guessing here would open an I.F.
    /// tap on the wrong intermediate frequency.
    mode: Option<Mode>,
    /// The offset in force, so a mode change can be recognised as one and the
    /// receiver moved to match.
    off: f64,
    /// The dial the transceiver was last commanded to, so an unchanged retune
    /// is not written to the serial port again.
    dial_hz: f64,
    /// Whether `dial_hz` has been established at all yet.
    dial_known: bool,
    /// Scratch for draining the transceiver's own stream.
    ctrl_buf: Vec<Complex32>,
    /// Transceiver audio waiting to be played, when it is what is being heard.
    audio_q: VecDeque<f32>,
    /// True while the transmitter is keyed, so the receive side knows to stop
    /// handing over what it hears of our own signal.
    keyed: bool,
    label: String,
}

impl PanadapterSource {
    /// `rx_caps` and `ctrl_caps` are what the two devices reported when they
    /// opened; [`Self::merge_caps`] is what the engine is told.
    pub fn new(
        rx: Box<dyn IqSource>,
        ctrl: Box<dyn IqSource>,
        cfg: PanadapterConfig,
        mode: Option<Mode>,
    ) -> Self {
        let off = mode.map_or(cfg.offset_hz, |m| cfg.offset_for(m));
        let label = format!("{} + {} panadapter", ctrl.describe(), rx.describe());
        info!(
            offset_hz = off,
            audio = ?cfg.audio,
            tap = ?cfg.tap,
            "panadapter pairing: {label}"
        );
        PanadapterSource {
            rx,
            ctrl,
            cfg,
            mode,
            off,
            dial_hz: 0.0,
            dial_known: false,
            ctrl_buf: vec![Complex32::new(0.0, 0.0); 4096],
            audio_q: VecDeque::new(),
            keyed: false,
            label,
        }
    }

    /// What the engine is told the pairing can do: the receive half from the
    /// receiver, the transmit half from the transceiver.
    ///
    /// Nothing is invented. A receive-only transceiver stays receive-only, and
    /// a receiver that publishes no tuning range still publishes none — the
    /// distinction between "says nothing" and "reaches nothing" is
    /// [`DeviceCaps::may_rx_hz`]'s to keep.
    pub fn merge_caps(rx: &DeviceCaps, ctrl: &DeviceCaps, cfg: &PanadapterConfig) -> DeviceCaps {
        DeviceCaps {
            driver: rx.driver.clone(),
            label: format!("{} + {}", ctrl.label, rx.label),
            rx_channels: rx.rx_channels,
            tx_channels: ctrl.tx_channels,
            // Not the receiver's own answer: what it means here is whether the
            // engine may keep reading while the *transceiver* is keyed, which
            // is the operator's call and nothing to do with either device's
            // own duplex. A separate receiver on a separate antenna genuinely
            // can, which is the whole reason the setting exists.
            full_duplex: !cfg.blank_on_tx,
            // There is real I/Q here, so this is emphatically not a
            // demod-audio radio, however the audio is arriving.
            audio_mode: false,
            // The transceiver modulates audio we hand its sound card, exactly
            // as a TCI rig does with wideband I/Q receive. An SDR used as the
            // control radio keeps modulated-I/Q transmit instead.
            tx_audio: ctrl.tx_audio || ctrl.audio_mode,
            // Translated into the station's dial, which is the domain the
            // engine judges every tune against — and on an I.F. tap the
            // receiver's own numbers are `offset_hz` away from it. Same rule
            // and same clamp as a converter's `shift_caps`: an edge below DC
            // is not a frequency, and a range that collapses is dropped. The
            // plain offset rather than a mode's, because those differ by a few
            // kHz and a tuning range is not measured that finely.
            freq_ranges_rx: rx
                .freq_ranges_rx
                .iter()
                .map(|&(lo, hi)| ((lo - cfg.offset_hz).max(0.0), hi - cfg.offset_hz))
                .filter(|&(lo, hi)| hi > lo)
                .collect(),
            freq_ranges_tx: ctrl.freq_ranges_tx.clone(),
            sample_rates: rx.sample_rates.clone(),
            rate_ranges: rx.rate_ranges.clone(),
            gains: rx
                .gains
                .iter()
                .filter(|g| g.direction == sdroxide_types::Direction::Rx)
                .chain(ctrl.gains.iter().filter(|g| g.direction == sdroxide_types::Direction::Tx))
                .cloned()
                .collect(),
            antennas_rx: rx.antennas_rx.clone(),
            antennas_tx: ctrl.antennas_tx.clone(),
            // The meters that matter here are the transmitter's.
            sensors: ctrl.sensors.clone(),
            has_swr_sensor: ctrl.has_swr_sensor,
            has_fwd_power_sensor: ctrl.has_fwd_power_sensor,
            shared_lo_rx: rx.shared_lo_rx,
            rx_audio_external: cfg.audio == PanadapterAudio::Transceiver,
            // This is the radio doing the borrowing, not the one lent out.
            lent_to: None,
            // The receiver's, because these describe the stream: the filter it
            // is running and the driver knobs behind it belong to the radio
            // painting the picture, not to the transceiver being listened to.
            bandwidths: rx.bandwidths.clone(),
            bandwidth_ranges: rx.bandwidth_ranges.clone(),
            settings: rx.settings.clone(),
            // The engine overwrites this from `Self::center_is_dial`, which is
            // false here however the transceiver answers: the window belongs to
            // the receiver and the dial moves inside it.
            center_is_dial: false,
            // The transceiver's, like the rest of the transmit half — and the
            // engine overwrites it from `Self::cw_audio_keyed`, which says the
            // same thing.
            cw_audio_keyed: ctrl.cw_audio_keyed,
            // The transceiver's too, but only while it is the one being
            // listened to: its squelch decides what comes out of its sound
            // card, and that is the audio here only under
            // `PanadapterAudio::Transceiver`. Listening to the receiver's own
            // I/Q instead, the honest gate is the engine's — the transceiver's
            // squelch would then be muting nothing but its own speaker. The
            // engine overwrites this from `Self::commands_squelch`, which says
            // the same thing.
            commands_squelch: ctrl.commands_squelch && cfg.audio == PanadapterAudio::Transceiver,
            // The receiver's, like the filter: if the radio painting the
            // picture is combining two aerials, this radio is listening to
            // what came out of that.
            diversity: rx.diversity,
            // The receiver's too — it is the one drawing the band — and the
            // engine overwrites it from `Self::wide_span_hz` in any case.
            wide_span_hz: rx.wide_span_hz,
            // The transceiver's: the power switch on offer here is the switch
            // on the radio being *listened to*, and switching the receiver
            // painting the picture off would leave nothing to draw with.
            commands_rig_power: ctrl.commands_rig_power,
            // The transceiver's as well, and for the same reason: the receiving
            // antenna on offer is the one on the radio being listened to. A
            // borrowed receiver has its own aerial and no say in this.
            has_rx_antenna: ctrl.has_rx_antenna,
        }
    }

    /// Adopt a mode change: re-pick the offset and slide the receiver to match,
    /// so a rig whose I.F. moves between SSB and DATA keeps the band where it
    /// was on screen.
    fn set_mode(&mut self, mode: Mode) {
        if self.mode == Some(mode) {
            return;
        }
        self.mode = Some(mode);
        let off = self.cfg.offset_for(mode);
        if (off - self.off).abs() < 0.5 {
            return;
        }
        // Where the span sits on the dial has not changed — the mode did — so
        // hold that and move the hardware under it.
        let dial_center = self.rx.center_hz() - self.off;
        self.off = off;
        if let Err(e) = self.rx.set_center_hz(dial_center + off) {
            warn!("panadapter receiver refused the {} offset: {e}", mode.label());
        }
    }

    /// Command the transceiver's dial, skipping a write that would change
    /// nothing. The CAT layer debounces as well, but a rig reached over a slow
    /// shared port should not be handed the same frequency on every engine
    /// block to begin with.
    fn set_dial(&mut self, hz: f64) {
        if self.dial_known && (hz - self.dial_hz).abs() < 0.5 {
            return;
        }
        self.dial_known = true;
        self.dial_hz = hz;
        if let Err(e) = self.ctrl.set_center_hz(hz) {
            warn!("transceiver refused {:.6} MHz: {e}", hz / 1e6);
        }
    }

    /// Drain whatever the transceiver's stream has waiting.
    ///
    /// Always, whichever radio is being listened to: the rig's capture ring
    /// fills whether or not anybody wants it, and a ring nobody empties is a
    /// ring that overruns. What is done with the samples is
    /// [`Self::queue_audio`]'s business.
    fn pump_ctrl(&mut self) {
        // A demod-audio rig packs its audio in the real component; anything
        // else here is an I/Q radio whose receive stream this arrangement does
        // not use, and draining it is all that is wanted.
        let want_audio = self.cfg.audio == PanadapterAudio::Transceiver && !self.keyed;
        loop {
            let n = match self.ctrl.read_available(&mut self.ctrl_buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) => {
                    warn!("transceiver audio: {e}");
                    break;
                }
            };
            if want_audio {
                self.audio_q.extend(self.ctrl_buf[..n].iter().map(|c| c.re));
            }
            if n < self.ctrl_buf.len() {
                break;
            }
        }
        // Keep the backlog bounded: the two clocks are independent, and the
        // slow drift between them is otherwise pure latency that only grows.
        let cap = (self.ctrl.sample_rate() * AUDIO_QUEUE_MS / 1000.0) as usize;
        if cap > 0 && self.audio_q.len() > cap {
            let cut = self.audio_q.len() - cap;
            self.audio_q.drain(..cut);
        }
    }

    /// Mirror the span about its centre, for a tap whose oscillator sits above
    /// the signal and hands the band over the wrong way round.
    fn maybe_invert(&self, buf: &mut [Complex32]) {
        if self.cfg.invert {
            for c in buf.iter_mut() {
                c.im = -c.im;
            }
        }
    }
}

impl IqSource for PanadapterSource {
    // ── Receive: the attached receiver ──────────────────────────────────────

    fn sample_rate(&self) -> f64 {
        self.rx.sample_rate()
    }

    fn center_hz(&self) -> f64 {
        self.rx.center_hz() - self.off
    }

    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.rx.set_center_hz(hz + self.off)
    }

    fn lo_offset_hz(&self) -> f64 {
        self.rx.lo_offset_hz()
    }

    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        self.pump_ctrl();
        let n = self.rx.read(buf)?;
        self.maybe_invert(&mut buf[..n]);
        Ok(n)
    }

    /// What the engine uses while transmitting, where the operator asked for
    /// the display to keep running. Bounded rather than trusted — see
    /// [`TX_READ_MS`].
    fn read_available(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        self.pump_ctrl();
        let cap = ((self.rx.sample_rate() * TX_READ_MS / 1000.0) as usize).clamp(1, buf.len());
        let n = self.rx.read_available(&mut buf[..cap])?;
        self.maybe_invert(&mut buf[..n]);
        Ok(n)
    }

    fn wide_spectrum_db(&mut self, out: &mut Vec<f32>) -> Option<(f64, f64)> {
        let (center, span) = self.rx.wide_spectrum_db(out)?;
        Some((center - self.off, span))
    }

    /// The lent receiver's lane, whose width the I.F. offset does not change.
    fn wide_span_hz(&self) -> f64 {
        self.rx.wide_span_hz()
    }

    fn set_gain_element(&mut self, name: &str, db: f64) -> Result<()> {
        self.rx.set_gain_element(name, db)
    }

    fn set_antenna(&mut self, name: &str) -> Result<()> {
        self.rx.set_antenna(name)
    }

    fn current_gains(&self) -> Vec<(String, f64)> {
        self.rx.current_gains()
    }

    fn current_antenna(&self) -> String {
        self.rx.current_antenna()
    }

    /// The lent receiver's answer, like every other front-end question here:
    /// a LimeSDR behind a LimeRFE still owns its socket when it is somebody
    /// else's panadapter.
    fn owns_rx_antenna(&self) -> bool {
        self.rx.owns_rx_antenna()
    }

    fn discard_pending_rx(&mut self) {
        self.rx.discard_pending_rx();
        self.ctrl.discard_pending_rx();
        self.audio_q.clear();
    }

    /// This pairing is the reason the hook exists. The lent receiver is a
    /// different radio from the one being keyed and has no idea an over is
    /// happening: it fills its ring at the full sample rate for the length of
    /// one, and with `blank_on_tx` set — the default — nothing drains it. Every
    /// SDR that can serve as a panadapter counts that as an overrun otherwise,
    /// so the plainest setup in the program is the one that logs a fault every
    /// two seconds of transmit and tells the operator to lower a sample rate
    /// that was never the problem.
    ///
    /// The transceiver is told as well: it may be a wideband source in its own
    /// right, and it is the engine's business which half is keyed, not this
    /// one's. A rig that stops its own receiver for the over ignores it.
    fn set_rx_paused(&mut self, paused: bool) {
        self.rx.set_rx_paused(paused);
        self.ctrl.set_rx_paused(paused);
    }

    /// Wideband, so there is no audio-band window to map.
    fn display_bandwidth(&self) -> Option<f64> {
        None
    }

    // ── Audio ───────────────────────────────────────────────────────────────

    fn rx_audio(&mut self, out: &mut Vec<f32>) -> Option<f64> {
        if self.cfg.audio != PanadapterAudio::Transceiver {
            return None;
        }
        // A block's worth, paced by whatever has arrived; silence rather than
        // nothing when the rig is momentarily behind, so the speaker path keeps
        // its cadence instead of stalling and letting the sub receiver's ear
        // run away from it.
        let want = (self.ctrl.sample_rate() * AUDIO_QUEUE_MS / 1000.0 / 4.0).max(1.0) as usize;
        let take = self.audio_q.len().min(want);
        out.extend(self.audio_q.drain(..take));
        out.resize(want, 0.0);
        Some(self.ctrl.sample_rate())
    }

    /// True only where the receiver is still being read through an over: with
    /// the display blanked as well there is nothing to silence, because nothing
    /// is being received.
    fn mutes_rx_audio_on_tx(&self) -> bool {
        self.cfg.mute_on_tx && !self.cfg.blank_on_tx
    }

    /// The operator's own setting, with no `blank_on_tx` in it: the blanking is
    /// the engine declining to read through an over *it* keyed, and it has
    /// nothing to say about one the transceiver keyed itself — that one is
    /// received right through, so the mute is the only thing that can silence
    /// it (issue #244).
    fn mutes_rx_audio_on_rig_tx(&self) -> bool {
        self.cfg.mute_on_tx
    }

    // ── Control: the transceiver ────────────────────────────────────────────

    /// The engine pushes `vfo - center` here on every retune. It is the only
    /// place the dial itself can be reconstructed, so it is where the
    /// transceiver is told where to go.
    fn set_if_offset(&mut self, hz: f64) {
        self.rx.set_if_offset(hz);
        if self.cfg.track {
            let vfo = self.center_hz() + hz;
            self.set_dial(vfo);
        }
    }

    /// False however the transceiver answers: here the window belongs to the
    /// *receiver*, and the dial moves inside it through [`Self::set_if_offset`]
    /// without the span having to follow. That separation is the whole point of
    /// the pairing — it is what a rig with only its own I/Q output does not have.
    fn center_is_dial(&self) -> bool {
        false
    }

    fn poll_control(&mut self) -> Vec<ControlUpdate> {
        let mut out = Vec::new();
        for u in self.ctrl.poll_control() {
            if let ControlUpdate::Freq(hz) = u {
                // The rig's own knob. Recorded as already commanded so the
                // engine's answering `set_if_offset` doesn't write it straight
                // back down the serial port.
                self.dial_known = true;
                self.dial_hz = hz;
            }
            if let ControlUpdate::Mode(m) = u {
                self.set_mode(m);
            }
            out.push(u);
        }
        // The receiver has no dial of its own here — its frequency is ours to
        // set — so the only thing worth hearing from it is a centre that moved
        // out from under us, which the engine adopts rather than corrects.
        for u in self.rx.poll_control() {
            if let ControlUpdate::Center(hz) = u {
                out.push(ControlUpdate::Center(hz - self.off));
            }
        }
        out
    }

    /// A pairing always follows the operator's mode: the transceiver is the
    /// radio being worked, and the offset the receiver is tuned to may depend
    /// on which mode that is.
    fn tracks_rx_mode(&self) -> bool {
        true
    }

    fn set_control_mode(&mut self, mode: Mode) -> Result<()> {
        self.set_mode(mode);
        self.ctrl.set_control_mode(mode)
    }

    fn set_control_filter(&mut self, mode: Mode, lo_hz: f64, hi_hz: f64) {
        self.ctrl.set_control_filter(mode, lo_hz, hi_hz);
    }

    fn set_rit_hz(&mut self, hz: f64) {
        self.ctrl.set_rit_hz(hz);
    }

    fn rx_signal_dbm(&mut self) -> Option<f32> {
        self.ctrl.rx_signal_dbm()
    }

    // ── Transmit: the transceiver ───────────────────────────────────────────

    fn tx_begin(&mut self, center_hz: f64, rate: f64) -> Result<f64> {
        let r = self.ctrl.tx_begin(center_hz, rate)?;
        self.keyed = true;
        // Whatever the rig's capture ring holds from before the over is stale
        // by the time it ends, and what it holds during one is our own
        // transmitter.
        self.audio_q.clear();
        Ok(r)
    }

    fn tx_write(&mut self, samples: &[Complex32]) -> Result<()> {
        self.ctrl.tx_write(samples)
    }

    fn tx_write_audio(&mut self, audio: &[f32]) -> Result<()> {
        self.ctrl.tx_write_audio(audio)
    }

    fn tx_end(&mut self) -> Result<()> {
        self.keyed = false;
        self.audio_q.clear();
        self.ctrl.tx_end()
    }

    fn tx_drain(&mut self) {
        self.ctrl.tx_drain();
    }

    fn tx_pace_cushion_ms(&self) -> f64 {
        self.ctrl.tx_pace_cushion_ms()
    }

    fn set_tx_gain_element(&mut self, name: &str, db: f64) -> Result<()> {
        self.ctrl.set_tx_gain_element(name, db)
    }

    fn current_tx_gains(&self) -> Vec<(String, f64)> {
        self.ctrl.current_tx_gains()
    }

    fn set_tx_antenna(&mut self, name: &str) -> Result<()> {
        self.ctrl.set_tx_antenna(name)
    }

    fn current_tx_antenna(&self) -> String {
        self.ctrl.current_tx_antenna()
    }

    fn set_tx_drive(&mut self, frac: f64) {
        self.ctrl.set_tx_drive(frac);
    }

    fn set_tune_drive(&mut self, frac: f64) {
        self.ctrl.set_tune_drive(frac);
    }

    fn commands_tx_power(&self) -> bool {
        self.ctrl.commands_tx_power()
    }

    fn tx_telemetry(&mut self) -> Option<sdroxide_types::TxTelemetry> {
        self.ctrl.tx_telemetry()
    }

    /// The transmit frequency, for whatever the transceiver switches bands on.
    /// Not the receiver's business: it is not in the transmit path.
    fn set_tx_freq_hz(&mut self, hz: f64) {
        self.ctrl.set_tx_freq_hz(hz);
    }

    fn cw_text_keying(&self) -> Option<usize> {
        self.ctrl.cw_text_keying()
    }

    fn cw_audio_keyed(&self) -> bool {
        self.ctrl.cw_audio_keyed()
    }

    /// The transceiver's squelch, and only while the transceiver is the one
    /// being listened to — see `merge_caps`.
    fn commands_squelch(&self) -> bool {
        self.ctrl.commands_squelch() && self.cfg.audio == PanadapterAudio::Transceiver
    }

    /// The transceiver's power switch, not the receiver's: it is the radio
    /// being listened to, and the one an operator away from the shack means
    /// when they say "switch the radio off" (issue #239). Switching off the
    /// receiver painting the picture would leave the pairing with no picture
    /// and no way back.
    fn set_rig_power(&mut self, on: bool) -> Result<()> {
        self.ctrl.set_rig_power(on)
    }

    fn commands_rig_power(&self) -> bool {
        self.ctrl.commands_rig_power()
    }

    /// The transceiver's receiving antenna, for the reason its power switch is
    /// the transceiver's: it is the radio being listened to.
    fn rx_antenna(&self) -> Option<bool> {
        self.ctrl.rx_antenna()
    }

    fn set_rx_antenna(&mut self, on: bool) -> Result<()> {
        self.ctrl.set_rx_antenna(on)
    }

    fn reread_rx_antenna(&mut self) {
        self.ctrl.reread_rx_antenna();
    }

    /// Gated on the same test, so the two answers cannot disagree: a level that
    /// reached the transceiver while the *receiver* was the one being listened
    /// to would quieten the operator's other radio and nothing here.
    fn set_squelch(&mut self, frac: f32) {
        if self.cfg.audio == PanadapterAudio::Transceiver {
            self.ctrl.set_squelch(frac);
        }
    }

    fn send_cw(&mut self, text: &str) {
        self.ctrl.send_cw(text);
    }

    fn abort_cw(&mut self) {
        self.ctrl.abort_cw();
    }

    fn set_cw_wpm(&mut self, wpm: f32) {
        self.ctrl.set_cw_wpm(wpm);
    }

    // ── Housekeeping ────────────────────────────────────────────────────────

    fn describe(&self) -> String {
        self.label.clone()
    }

    fn open_status(&self) -> Option<String> {
        let notes: Vec<String> =
            [self.ctrl.open_status(), self.rx.open_status()].into_iter().flatten().collect();
        (!notes.is_empty()).then(|| notes.join("\n"))
    }

    /// Either half being unreachable is the pairing being unreachable: half a
    /// radio is not one the operator can use, and the engine's retry rebuilds
    /// both.
    fn needs_reopen(&self) -> bool {
        self.rx.needs_reopen() || self.ctrl.needs_reopen()
    }

    fn release(&mut self) {
        self.rx.release();
        self.ctrl.release();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_types::{IfModeClass, PanadapterTap};
    use std::sync::{Arc, Mutex};

    /// What a fake source was asked to do, and what it should report back —
    /// shared with the test so a rig can be made to move its own dial after the
    /// pairing has taken ownership of it.
    #[derive(Default)]
    struct Log {
        centers: Vec<f64>,
        gains: Vec<String>,
        tx_begins: Vec<f64>,
        cw: Vec<String>,
        /// Queued for the next `poll_control`.
        updates: Vec<ControlUpdate>,
    }

    struct Fake {
        name: &'static str,
        rate: f64,
        center: f64,
        log: Arc<Mutex<Log>>,
        /// Samples handed over by `read`, real part only.
        audio: Vec<f32>,
        signal_dbm: Option<f32>,
    }

    impl Fake {
        fn new(name: &'static str, rate: f64, center: f64) -> (Self, Arc<Mutex<Log>>) {
            let log = Arc::new(Mutex::new(Log::default()));
            (
                Fake { name, rate, center, log: log.clone(), audio: Vec::new(), signal_dbm: None },
                log,
            )
        }
    }

    impl IqSource for Fake {
        fn sample_rate(&self) -> f64 {
            self.rate
        }
        fn center_hz(&self) -> f64 {
            self.center
        }
        fn set_center_hz(&mut self, hz: f64) -> Result<()> {
            self.center = hz;
            self.log.lock().unwrap().centers.push(hz);
            Ok(())
        }
        fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
            let n = self.audio.len().min(buf.len());
            for (i, s) in self.audio.drain(..n).enumerate() {
                buf[i] = Complex32::new(s, 0.0);
            }
            Ok(n)
        }
        fn describe(&self) -> String {
            self.name.into()
        }
        fn set_gain_element(&mut self, name: &str, _db: f64) -> Result<()> {
            self.log.lock().unwrap().gains.push(format!("{}:{name}", self.name));
            Ok(())
        }
        fn poll_control(&mut self) -> Vec<ControlUpdate> {
            std::mem::take(&mut self.log.lock().unwrap().updates)
        }
        fn tx_begin(&mut self, center_hz: f64, _rate: f64) -> Result<f64> {
            self.log.lock().unwrap().tx_begins.push(center_hz);
            Ok(48_000.0)
        }
        fn send_cw(&mut self, text: &str) {
            self.log.lock().unwrap().cw.push(format!("{}:{text}", self.name));
        }
        fn rx_signal_dbm(&mut self) -> Option<f32> {
            self.signal_dbm
        }
    }

    fn cfg() -> PanadapterConfig {
        PanadapterConfig { source_radio: Some(1), ..Default::default() }
    }

    /// Build a pairing and hand back the two logs (receiver, transceiver).
    fn pair(cfg: PanadapterConfig) -> (PanadapterSource, Arc<Mutex<Log>>, Arc<Mutex<Log>>) {
        let (rx, rx_log) = Fake::new("rx", 2_000_000.0, 14_100_000.0 + cfg.offset_for(Mode::Usb));
        let (ctrl, ctrl_log) = Fake::new("rig", 48_000.0, 14_100_000.0);
        let src = PanadapterSource::new(Box::new(rx), Box::new(ctrl), cfg, Some(Mode::Usb));
        (src, rx_log, ctrl_log)
    }

    /// The plain case: a receiver on the same antenna. The engine sees the
    /// receiver's centre on the dial's scale, and moving the span moves the
    /// receiver and nothing else.
    #[test]
    fn a_receiver_on_the_antenna_needs_no_arithmetic() {
        let (mut src, rx_log, ctrl_log) = pair(cfg());
        assert_eq!(src.center_hz(), 14_100_000.0);
        assert_eq!(src.sample_rate(), 2_000_000.0, "the span is the receiver's");

        src.set_center_hz(14_200_000.0).unwrap();
        assert_eq!(rx_log.lock().unwrap().centers, vec![14_200_000.0]);
        assert_eq!(src.center_hz(), 14_200_000.0);
        assert!(ctrl_log.lock().unwrap().centers.is_empty(), "a span move is not a dial move");
    }

    /// An I.F. tap: the receiver sits on the intermediate frequency and the
    /// engine still works entirely in dial frequencies.
    #[test]
    fn an_if_tap_keeps_the_engine_on_the_dials_scale() {
        let c = PanadapterConfig { offset_hz: 70_455_000.0, tap: PanadapterTap::IfOutput, ..cfg() };
        let (mut src, rx_log, _) = pair(c);
        assert_eq!(src.center_hz(), 14_100_000.0);

        src.set_center_hz(7_100_000.0).unwrap();
        assert_eq!(
            rx_log.lock().unwrap().centers,
            vec![7_100_000.0 + 70_455_000.0],
            "the receiver is tuned to the I.F., the engine asked for the dial"
        );
        assert_eq!(src.center_hz(), 7_100_000.0);
    }

    /// The dial only ever moves through `set_if_offset`, which is the one call
    /// carrying the VFO — and an unchanged VFO must not be rewritten to a rig
    /// on a slow serial port.
    #[test]
    fn the_transceivers_dial_follows_the_vfo_and_only_on_a_change() {
        let (mut src, _, ctrl_log) = pair(cfg());
        // Engine: centre 14.100, VFO 14.1002 → offset 200 Hz.
        src.set_if_offset(200.0);
        assert_eq!(ctrl_log.lock().unwrap().centers, vec![14_100_200.0]);
        src.set_if_offset(200.0);
        assert_eq!(ctrl_log.lock().unwrap().centers.len(), 1, "the same dial is not rewritten");
        src.set_if_offset(-1_000.0);
        assert_eq!(ctrl_log.lock().unwrap().centers, vec![14_100_200.0, 14_099_000.0]);
    }

    /// Tracking off parks the receiver: the panadapter is a fixed watch on one
    /// segment and the rig is free to go elsewhere.
    #[test]
    fn tracking_off_leaves_the_transceiver_alone() {
        let (mut src, _, ctrl_log) = pair(PanadapterConfig { track: false, ..cfg() });
        src.set_if_offset(200.0);
        assert!(ctrl_log.lock().unwrap().centers.is_empty());
    }

    /// The rig's own knob reaches the engine unchanged, and does not bounce
    /// straight back out of the serial port when the engine answers.
    #[test]
    fn the_rigs_own_knob_is_reported_and_not_echoed() {
        let (mut src, _, ctrl_log) = pair(cfg());
        ctrl_log.lock().unwrap().updates = vec![ControlUpdate::Freq(14_250_000.0)];
        assert_eq!(src.poll_control(), vec![ControlUpdate::Freq(14_250_000.0)]);
        // The engine's answer: it has moved the span there, so the VFO is the
        // centre and the offset is zero.
        src.set_center_hz(14_250_000.0).unwrap();
        src.set_if_offset(0.0);
        assert!(
            ctrl_log.lock().unwrap().centers.is_empty(),
            "the dial the rig just reported must not be written back to it"
        );
    }

    /// A rig whose I.F. moves between SSB and DATA: changing mode slides the
    /// receiver so the band stays where it was on screen.
    #[test]
    fn a_mode_change_moves_a_per_mode_if_tap() {
        let mut c =
            PanadapterConfig { offset_hz: 9_000_000.0, tap: PanadapterTap::IfOutput, ..cfg() };
        c.set_mode_offset(IfModeClass::Data, Some(9_001_500.0));
        let (mut src, rx_log, ctrl_log) = pair(c);
        assert_eq!(src.center_hz(), 14_100_000.0);

        src.set_control_mode(Mode::Ft8).unwrap();
        assert_eq!(
            rx_log.lock().unwrap().centers,
            vec![14_100_000.0 + 9_001_500.0],
            "DATA's own I.F."
        );
        assert_eq!(src.center_hz(), 14_100_000.0, "the band has not moved on screen");

        // ...and back, whether the change came from us or from the rig.
        ctrl_log.lock().unwrap().updates = vec![ControlUpdate::Mode(Mode::Lsb)];
        let _ = src.poll_control();
        assert_eq!(src.center_hz(), 14_100_000.0);
        assert_eq!(
            rx_log.lock().unwrap().centers.last().copied(),
            Some(14_100_000.0 + 9_000_000.0)
        );
    }

    /// Each half of the radio gets the calls that belong to it.
    #[test]
    fn control_goes_to_the_rig_and_the_front_end_to_the_receiver() {
        let (mut src, rx_log, ctrl_log) = pair(cfg());
        src.set_gain_element("LNA", 20.0).unwrap();
        src.tx_begin(14_100_000.0, 48_000.0).unwrap();
        src.send_cw("CQ");
        assert_eq!(rx_log.lock().unwrap().gains, vec!["rx:LNA"], "gain is the receiver's");
        assert!(ctrl_log.lock().unwrap().gains.is_empty());
        assert_eq!(ctrl_log.lock().unwrap().tx_begins, vec![14_100_000.0]);
        assert_eq!(ctrl_log.lock().unwrap().cw, vec!["rig:CQ"]);
        assert!(rx_log.lock().unwrap().tx_begins.is_empty(), "the receiver never transmits");
    }

    /// The S-meter is the transceiver's: it is the radio on the antenna the
    /// operator is working.
    #[test]
    fn the_s_meter_is_the_transceivers() {
        let (rx, _) = Fake::new("rx", 2e6, 14e6);
        let (mut ctrl, _) = Fake::new("rig", 48_000.0, 14e6);
        ctrl.signal_dbm = Some(-73.0);
        let mut src = PanadapterSource::new(Box::new(rx), Box::new(ctrl), cfg(), Some(Mode::Usb));
        assert_eq!(src.rx_signal_dbm(), Some(-73.0));
    }

    /// An inverted tap is mirrored about its centre on the way in.
    #[test]
    fn an_inverted_tap_is_mirrored() {
        let (mut rx, _) = Fake::new("rx", 48_000.0, 14e6);
        rx.audio = vec![1.0, 2.0];
        let (ctrl, _) = Fake::new("rig", 48_000.0, 14e6);
        let mut src = PanadapterSource::new(
            Box::new(rx),
            Box::new(ctrl),
            PanadapterConfig { invert: true, ..cfg() },
            Some(Mode::Usb),
        );
        let mut buf = vec![Complex32::new(0.0, 0.0); 4];
        // The fake packs into the real part, so give the imaginary side
        // something to flip by reading it back through the source's own path.
        let n = src.read(&mut buf).unwrap();
        assert_eq!(n, 2);
        // Real parts survive; the (zero) imaginary parts negate to zero, which
        // is all a real-packed fake can show. The sign convention itself is
        // asserted by the mirror below.
        assert_eq!(buf[0].re, 1.0);
        let mut span = [Complex32::new(0.0, 1.0), Complex32::new(0.0, -2.0)];
        src.maybe_invert(&mut span);
        assert_eq!(span[0].im, -1.0);
        assert_eq!(span[1].im, 2.0);
    }

    /// The transmit gate the operator chose reaches the engine as the two
    /// things the engine acts on.
    #[test]
    fn the_transmit_gate_maps_onto_duplex_and_the_mute() {
        let rx = DeviceCaps { rx_channels: 1, full_duplex: true, ..Default::default() };
        let ctrl = DeviceCaps { tx_channels: 1, audio_mode: true, ..Default::default() };

        // The default: stop reading for the length of the over. Nothing to
        // mute, because nothing is received.
        let both = PanadapterSource::merge_caps(&rx, &ctrl, &cfg());
        assert!(!both.full_duplex);
        assert!(both.tx_audio, "the rig modulates the audio we send it");
        assert_eq!(both.tx_channels, 1);

        // Keep the picture: the engine must read through the over, and the
        // speaker must be silenced separately.
        let c = PanadapterConfig { blank_on_tx: false, ..cfg() };
        assert!(PanadapterSource::merge_caps(&rx, &ctrl, &c).full_duplex);
        let (src, _, _) = pair(c);
        assert!(src.mutes_rx_audio_on_tx());
        let (src, _, _) = pair(cfg());
        assert!(!src.mutes_rx_audio_on_tx(), "a blanked receiver has nothing to mute");
    }

    /// An over the operator keys at the radio is received right through —
    /// nothing here drives it, so the engine never stops reading for it — and
    /// the mute is then the only thing that can silence a receiver listening to
    /// its own station's transmitter. So it follows the operator's own MUTE ON
    /// TRANSMIT and nothing else: the blanking above says what happens to an
    /// over *sdroxide* keys and has no bearing on this one (issue #244).
    #[test]
    fn a_rig_driven_over_is_muted_by_the_mute_alone() {
        for blank in [true, false] {
            let on = PanadapterConfig { mute_on_tx: true, blank_on_tx: blank, ..cfg() };
            assert!(pair(on).0.mutes_rx_audio_on_rig_tx(), "blank_on_tx {blank}");
            let off = PanadapterConfig { mute_on_tx: false, blank_on_tx: blank, ..cfg() };
            assert!(!pair(off).0.mutes_rx_audio_on_rig_tx(), "blank_on_tx {blank}");
        }
    }

    /// An I.F. tap's receiver is tuned `offset_hz` away from the dial, so the
    /// range it publishes is about frequencies the operator never types. The
    /// engine checks the dial against it before every tune — so what it is told
    /// has to be the dial's range, not the receiver's.
    #[test]
    fn the_receivers_range_arrives_in_the_stations_own_frequencies() {
        // A dongle watching a 70.455 MHz first I.F.
        let rx = DeviceCaps {
            rx_channels: 1,
            freq_ranges_rx: vec![(24_000_000.0, 1_766_000_000.0)],
            ..Default::default()
        };
        let ctrl = DeviceCaps { tx_channels: 1, ..Default::default() };
        let c = PanadapterConfig { offset_hz: 70_455_000.0, tap: PanadapterTap::IfOutput, ..cfg() };
        let both = PanadapterSource::merge_caps(&rx, &ctrl, &c);
        assert!(both.may_rx_hz(14_200_000.0), "20 m is what the operator dials");
        assert!(!both.may_rx_hz(1_700_000_000.0), "and the dongle's own top no longer is");

        // A second receiver on its own antenna is not offset from anything, and
        // its range reaches the engine exactly as it published it.
        let plain = PanadapterSource::merge_caps(&rx, &ctrl, &cfg());
        assert_eq!(plain.freq_ranges_rx, rx.freq_ranges_rx);
    }

    /// Listening to the transceiver: its audio is handed over as audio, and the
    /// engine is told so through the capabilities.
    #[test]
    fn transceiver_audio_is_handed_over_as_audio() {
        let (rx, _) = Fake::new("rx", 2e6, 14e6);
        let (mut ctrl, _) = Fake::new("rig", 48_000.0, 14e6);
        ctrl.audio = vec![0.5; 600];
        let mut src = PanadapterSource::new(
            Box::new(rx),
            Box::new(ctrl),
            PanadapterConfig { audio: PanadapterAudio::Transceiver, ..cfg() },
            Some(Mode::Usb),
        );
        assert!(
            PanadapterSource::merge_caps(
                &DeviceCaps::default(),
                &DeviceCaps::default(),
                &PanadapterConfig { audio: PanadapterAudio::Transceiver, ..cfg() }
            )
            .rx_audio_external
        );

        let mut buf = vec![Complex32::new(0.0, 0.0); 16];
        src.read(&mut buf).unwrap(); // pumps the rig's stream
        let mut out = Vec::new();
        let rate = src.rx_audio(&mut out).expect("the rig's audio");
        assert_eq!(rate, 48_000.0);
        assert!(!out.is_empty());
        assert!(out.iter().take(600).all(|&s| s == 0.5), "the rig's samples, not the receiver's");
        // A block always comes out the same length, padded with silence, so the
        // speaker path keeps its cadence when the rig falls behind.
        let mut later = Vec::new();
        src.rx_audio(&mut later).unwrap();
        assert_eq!(later.len(), out.len());
        assert!(later.iter().all(|&s| s == 0.0));
    }

    /// Listening to the receiver: nothing is offered as external audio, but the
    /// rig's capture ring is still drained so it cannot overrun.
    #[test]
    fn listening_to_the_receiver_still_drains_the_rig() {
        let (rx, _) = Fake::new("rx", 2e6, 14e6);
        let (mut ctrl, _) = Fake::new("rig", 48_000.0, 14e6);
        ctrl.audio = vec![0.5; 600];
        let mut src = PanadapterSource::new(Box::new(rx), Box::new(ctrl), cfg(), Some(Mode::Usb));
        let mut buf = vec![Complex32::new(0.0, 0.0); 16];
        src.read(&mut buf).unwrap();
        assert_eq!(src.rx_audio(&mut Vec::new()), None);
        assert!(src.audio_q.is_empty(), "drained and discarded, not queued");
    }
}
