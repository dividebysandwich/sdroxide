//! An [`IqSource`] for one DDC of an OpenHPSDR ethernet SDR, Protocol 1
//! (Metis / Hermes-Lite 2) or Protocol 2 — the protocol is detected at open
//! time. The DDC delivers wideband complex I/Q, so this drives the engine's
//! normal DDC/demod path exactly like a SoapySDR device (`audio_mode = false`);
//! transmit I/Q goes to the board's DUC (P2) or the EP2 frame stream (P1).
//!
//! The connection is shared: a Protocol 2 board carries several independently
//! tunable DDCs, so two radio tabs on the same address each take one over a
//! single connection. The [`crate::device_registry`] is what pairs a second
//! source with the first one's connection; the transmitter belongs to the
//! DDC-0 source alone, and Protocol 1 boards have only DDC 0.

use std::net::Ipv4Addr;
use std::time::{Duration, Instant};

use sdroxide_dsp::{Decimator, PureSignal};
use sdroxide_hpsdr::{HpsdrBoard, HpsdrRx, LNA_GAIN_ELEMENT, TX_RATE_HZ};
use sdroxide_radio::{Complex32, ControlUpdate, IqSource, Result};

use crate::device_registry::{DeviceKey, SharedDevice, registry};

impl SharedDevice for HpsdrBoard {
    fn is_alive(&self) -> bool {
        HpsdrBoard::is_alive(self)
    }
}

/// How long the board may deliver no I/Q at all before this stream counts as
/// dead and the engine starts reconnecting. Comfortably longer than the few
/// milliseconds a healthy board takes to begin streaming after the run
/// command, and longer than the network thread's own starved-DDC nudges.
const SILENCE_BEFORE_REOPEN: Duration = Duration::from_secs(5);

/// How often the predistortion loop says whether it has found the feedback.
///
/// It is one line and it only appears during an over, but it is the only thing
/// that separates "the coupler is doing its job" from "the correction table has
/// been sitting at unity all evening", so it goes out often enough to be seen
/// on a short transmission.
const PS_LOG_INTERVAL: Duration = Duration::from_secs(5);

pub struct HpsdrSource {
    /// The shared connection and this source's stream on it. `None` after
    /// [`IqSource::release`]: the stream is given back so a rebuilt source
    /// can claim the DDC again, while other radios' streams on the same
    /// connection run on undisturbed.
    board: Option<std::sync::Arc<HpsdrBoard>>,
    rx: Option<HpsdrRx>,
    /// The connection's sample rate, kept here so a released source still
    /// answers `IqSource::sample_rate` — see that method.
    rate: f64,
    center: f64,
    /// See [`sdroxide_types::HpsdrConfig::ppm`].
    ppm: f64,
    rx_scratch: Vec<f32>,
    tx_scratch: Vec<f32>,
    /// The radio's PTT line as [`IqSource::poll_control`] last reported it, so
    /// only changes are handed on. The board reports a level and the engine
    /// wants an edge.
    ptt: bool,
    label: String,
    /// See [`sdroxide_types::HpsdrConfig::tx_latency_ms`].
    tx_latency_ms: f64,

    // ── PureSignal ──
    /// The predistortion loop, present exactly when the operator has asked for
    /// it on the radio that owns the transmitter.
    puresignal: Option<PureSignal>,
    /// Brings the board's receive rate down to the 48 kHz the transmit stream
    /// runs at, so the two can be compared sample for sample. `None` when the
    /// board is already at 48 kHz.
    ps_decim: Option<Decimator>,
    /// Where the transmitter is this over, for the offset the feedback has to
    /// be spun back by. Zero while receiving.
    ps_tx_hz: f64,
    ps_fb_raw: Vec<f32>,
    ps_fb_iq: Vec<Complex32>,
    ps_fb_dec: Vec<Complex32>,
    /// The transmit block, predistorted — `IqSource::tx_write` is handed a
    /// shared slice and the predistorter has to own what it bends.
    ps_tx_iq: Vec<Complex32>,
    ps_log_at: Instant,
    /// Whether the "the transmitter is too far outside the receiver's window"
    /// complaint has already been made this session.
    ps_warned_offset: bool,
}

impl HpsdrSource {
    /// Attach DDC `cfg.ddc` of the board at `ip`, connecting only if no radio
    /// in this process already holds that connection, and start streaming at
    /// `center_hz`. The rate, initial front-end gain and open-collector plan
    /// are connection-level: whoever connects first sets them, and a later
    /// attach runs with the established ones whatever its own config says.
    pub fn open(
        ip: Ipv4Addr,
        cfg: &sdroxide_types::HpsdrConfig,
        center_hz: f64,
    ) -> anyhow::Result<Self> {
        let board = registry()
            .get_or_open(DeviceKey::Hpsdr(ip), || {
                HpsdrBoard::open(
                    ip,
                    cfg.sample_rate_hz,
                    cfg.lna_gain_db,
                    cfg.oc_plan(),
                    cfg.invert_spectrum,
                    cfg.pa_enable,
                    cfg.io_rx_input,
                )
                .map(std::sync::Arc::new)
                .map_err(|e| e.to_string())
            })
            .map_err(anyhow::Error::msg)?;
        let rx = board.rx(cfg.ddc).map_err(|e| anyhow::Error::msg(e.to_string()))?;
        rx.set_rx_freq(sdroxide_types::HpsdrConfig::apply_ppm(center_hz, cfg.ppm));
        let label = if cfg.ddc == 0 {
            format!("HPSDR {} @ {ip} ({:.3} Msps)", board.board(), board.sample_rate_hz() / 1e6)
        } else {
            format!(
                "HPSDR DDC{} {} @ {ip} ({:.3} Msps)",
                cfg.ddc + 1,
                board.board(),
                board.sample_rate_hz() / 1e6
            )
        };
        tracing::info!(
            "HPSDR source ready: {label}, Protocol {}, center {center_hz:.0} Hz",
            board.protocol()
        );
        // The predistortion loop, on the radio that owns the transmitter and
        // nowhere else: a second DDC has no transmitter to linearise.
        let rate = board.sample_rate_hz();
        let puresignal = (cfg.puresignal && cfg.ddc == 0)
            .then(|| PureSignal::new(usize::from(cfg.ps_bins), cfg.ps_rate, TX_RATE_HZ as f64));
        // The receive stream is faster than the transmit one on every rate but
        // the lowest, and the loop compares the two sample for sample, so the
        // feedback is brought down to the transmitter's rate before it is
        // handed over. Every Protocol 1 and Protocol 2 rate is a power of two
        // times 48 kHz, which is exactly what this decimator does.
        let ps_decim = puresignal.as_ref().and_then(|_| {
            let factor = (rate / f64::from(TX_RATE_HZ)).round() as u32;
            (factor > 1).then(|| Decimator::new(factor))
        });
        if puresignal.is_some() {
            tracing::info!(
                "HPSDR: PureSignal is on — the receiver is the feedback path, so a coupled \
                 sample of the transmitter has to reach it during the over (on a Hermes-Lite 2 \
                 that is the IO board's PureSignal input). {} table steps, feedback decimated \
                 by {}. Until the loop locks the transmitter is left exactly as it would have \
                 been.",
                cfg.ps_bins,
                ps_decim.as_ref().map_or(1, |d| d.factor())
            );
            if cfg.io_rx_input != sdroxide_types::HpsdrIoRxInput::IoBoardPureSignal {
                tracing::warn!(
                    "HPSDR: PureSignal is on but the IO board's receive input is set to \"{}\" \
                     — unless you have wired the transmit sample in some other way, the T/R \
                     switch takes the coupler away for the length of every over and the loop \
                     will never lock",
                    cfg.io_rx_input.label()
                );
            }
        }
        Ok(HpsdrSource {
            rate,
            center: center_hz,
            ppm: cfg.ppm,
            rx_scratch: Vec::new(),
            tx_scratch: Vec::new(),
            ptt: false,
            label,
            rx: Some(rx),
            board: Some(board),
            // Held to the offered range here rather than trusting the file: a
            // cushion deeper than the host TX ring cannot be delivered, and
            // asking for one only saturates the ring (see
            // `HpsdrConfig::TX_LATENCY_MS_RANGE`). A `radio.json` written by
            // hand, or by the one build whose ceiling was 1000 ms, still opens
            // on something the ring can hold.
            tx_latency_ms: cfg.tx_latency_ms.clamp(
                *sdroxide_types::HpsdrConfig::TX_LATENCY_MS_RANGE.start(),
                *sdroxide_types::HpsdrConfig::TX_LATENCY_MS_RANGE.end(),
            ),
            puresignal: puresignal.map(|mut ps| {
                ps.set_frozen(cfg.ps_frozen);
                ps
            }),
            ps_decim,
            ps_tx_hz: 0.0,
            ps_fb_raw: Vec::new(),
            ps_fb_iq: Vec::new(),
            ps_fb_dec: Vec::new(),
            ps_tx_iq: Vec::new(),
            ps_log_at: Instant::now(),
            ps_warned_offset: false,
        })
    }

    /// The connection's rate, remembered rather than asked for: it is fixed
    /// when the board is opened, and [`IqSource::release`] lets the connection
    /// go while the engine is still running on this source.
    pub fn sample_rate_hz(&self) -> f64 {
        self.rate
    }

    pub fn board(&self) -> &str {
        self.board.as_ref().map_or("HPSDR", |b| b.board())
    }

    pub fn protocol(&self) -> u8 {
        self.board.as_ref().map_or(2, |b| b.protocol())
    }

    /// Whether the board has a front-end gain the UI should offer.
    pub fn has_lna_gain(&self) -> bool {
        self.board.as_ref().is_some_and(|b| b.has_lna_gain())
    }

    /// Take what the coupler has put into the receiver and give it to the
    /// predistortion loop.
    ///
    /// Called from `tx_write`, which is the only place that runs during an
    /// over: the engine stops reading a half-duplex source for the length of a
    /// transmission, so this is the only thing draining the ring. The board
    /// itself keeps streaming — Protocol 1 is commanded in duplex — so what is
    /// in the ring while keyed is exactly what the receiver heard of our own
    /// transmitter.
    ///
    /// The ring is drained whether or not the loop can use it. Leaving it to
    /// fill would mean the next over's feedback started with the last one's
    /// samples, and an alignment measured against the wrong transmission is
    /// worse than none.
    fn pump_feedback(&mut self) {
        let (Some(rx), Some(ps)) = (self.rx.as_mut(), self.puresignal.as_mut()) else { return };
        // Enough for a comfortable block at the highest rate, so a long over
        // never leaves the ring standing full between transmit blocks.
        const WANT_PAIRS: usize = 32_768;
        if self.ps_fb_raw.len() < WANT_PAIRS * 2 {
            self.ps_fb_raw.resize(WANT_PAIRS * 2, 0.0);
        }
        let n = rx.rx_read(&mut self.ps_fb_raw);
        let pairs = n / 2;
        if pairs == 0 {
            return;
        }
        self.ps_fb_iq.clear();
        self.ps_fb_iq.extend(
            (0..pairs).map(|p| Complex32::new(self.ps_fb_raw[2 * p], self.ps_fb_raw[2 * p + 1])),
        );
        let fb: &[Complex32] = match self.ps_decim.as_mut() {
            Some(d) => {
                self.ps_fb_dec.clear();
                d.process(&self.ps_fb_iq, &mut self.ps_fb_dec);
                &self.ps_fb_dec
            }
            None => &self.ps_fb_iq,
        };
        if fb.is_empty() {
            return;
        }
        // The transmitter and the receiver are two NCOs on one board and they
        // are commanded separately, so the coupled signal lands wherever the
        // difference puts it in the receiver's span. It is known exactly, so
        // this is arithmetic rather than a search.
        let offset = self.ps_tx_hz - self.center;
        if offset.abs() > f64::from(TX_RATE_HZ) * 0.45 {
            if !self.ps_warned_offset {
                self.ps_warned_offset = true;
                tracing::warn!(
                    "HPSDR: transmitting {:.1} kHz from where the receiver is tuned, which is \
                     outside the 48 kHz the feedback is compared over — PureSignal will not \
                     correct this over",
                    offset / 1e3
                );
            }
            return;
        }
        ps.feed_back(fb, offset, f64::from(TX_RATE_HZ));
        if self.ps_log_at.elapsed() >= PS_LOG_INTERVAL {
            self.ps_log_at = Instant::now();
            if ps.locked() {
                tracing::info!(
                    "HPSDR PureSignal is correcting {:.1} dB of compression (feedback matched \
                     at {:.2}){}",
                    ps.correction_db(),
                    ps.score(),
                    if ps.frozen() { ", table held" } else { "" }
                );
            } else {
                tracing::info!(
                    "HPSDR PureSignal has not found the transmission in the receiver's stream \
                     (best match {:.2}) — the transmitter is uncorrected. Check the coupler and \
                     its attenuator, and that the receive input is the one the transmit sample \
                     is wired to",
                    ps.score()
                );
            }
        }
    }
}

impl IqSource for HpsdrSource {
    fn sample_rate(&self) -> f64 {
        self.sample_rate_hz()
    }

    fn center_hz(&self) -> f64 {
        self.center
    }

    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center = hz;
        if let Some(rx) = self.rx.as_ref() {
            rx.set_rx_freq(sdroxide_types::HpsdrConfig::apply_ppm(hz, self.ppm));
        }
        Ok(())
    }

    /// Where the operator's dial is, which on a station with a transverter is
    /// not where the board is tuned. Only the accessory board's band decoder
    /// reads it — see `HpsdrRx::set_band_dial`.
    fn set_dial_hz(&mut self, hz: f64) {
        if let Some(rx) = self.rx.as_ref() {
            // Equal means nothing is in front of the radio, and the decoder is
            // better off on the frequency it already had.
            rx.set_band_dial((hz - self.center).abs().gt(&0.5).then_some(hz));
        }
    }

    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        let Some(rx) = self.rx.as_mut() else {
            // Released: nothing will ever arrive; nap so the engine loop
            // doesn't spin while the reopen it asked for is prepared.
            std::thread::sleep(Duration::from_millis(5));
            return Ok(0);
        };
        let need = buf.len() * 2;
        if self.rx_scratch.len() < need {
            self.rx_scratch.resize(need, 0.0);
        }
        let n = rx.rx_read(&mut self.rx_scratch[..need]);
        let pairs = n / 2;
        if pairs == 0 {
            // No samples yet — brief nap so the DSP loop doesn't spin hot.
            std::thread::sleep(Duration::from_millis(2));
            return Ok(0);
        }
        for p in 0..pairs {
            buf[p] = Complex32::new(self.rx_scratch[2 * p], self.rx_scratch[2 * p + 1]);
        }
        Ok(pairs)
    }

    fn describe(&self) -> String {
        self.label.clone()
    }

    /// The board's own temperature sensor, which on this family means a
    /// Hermes-Lite 2 and nothing else (issue #333). Every other HPSDR board
    /// answers `None` and the meter says nothing.
    fn pa_temp_c(&mut self) -> Option<f32> {
        self.rx.as_ref()?.pa_temp_c()
    }

    /// The board's front-end LNA gain. On a Hermes-Lite 2 this is the only
    /// analogue gain there is, and leaving it at whatever the gateware came up
    /// with is the difference between a deaf receiver and a clipping one.
    fn set_gain_element(&mut self, name: &str, db: f64) -> Result<()> {
        if name == LNA_GAIN_ELEMENT {
            if let Some(rx) = self.rx.as_mut() {
                rx.set_lna_gain_db(db);
            }
        } else if name == sdroxide_types::HpsdrConfig::PPM_ELEMENT {
            self.ppm = db;
            if let Some(rx) = self.rx.as_ref() {
                rx.set_rx_freq(sdroxide_types::HpsdrConfig::apply_ppm(self.center, self.ppm));
            }
        }
        Ok(())
    }

    fn current_gains(&self) -> Vec<(String, f64)> {
        match self.rx.as_ref() {
            Some(rx) if rx.has_lna_gain() => {
                vec![(LNA_GAIN_ELEMENT.to_string(), rx.lna_gain_db())]
            }
            _ => Vec::new(),
        }
    }

    /// A stream that has stopped delivering I/Q — the board unplugged,
    /// rebooted, taken over by another program, or opened with the wrong
    /// protocol because a probe was lost — is reported as needing a reopen so
    /// the engine reconnects on its own. Without this the operator has to
    /// press Apply again by hand. A released source likewise.
    fn needs_reopen(&self) -> bool {
        self.rx.as_ref().is_none_or(|rx| rx.silent_for() >= SILENCE_BEFORE_REOPEN)
    }

    /// Give this DDC's stream back ahead of a rebuild — **and the connection
    /// with it, unless another radio is still streaming its own DDC over it.**
    ///
    /// The connection used to be kept deliberately, so that an Apply with the
    /// address unchanged re-attached over the live link rather than re-opening
    /// the board. That saved a moment and cost the operator every setting they
    /// had just changed: the sample rate, the front-end gain the board starts
    /// at, the J16 filter board, the spectrum inversion, the PA switch and the
    /// IO board's receive input are all connection-level — chosen in
    /// `HpsdrBoard::open` and never revisited — so a live connection outliving
    /// its last radio made an Apply that changed any of them do nothing at
    /// all. That is the half of issue #135 the reporter saw on a Hermes-Lite 2:
    /// a new sample rate only took effect after restarting sdroxide.
    ///
    /// Dropping the last `Arc` runs the connection's teardown before this
    /// returns, so the replacement opens against a board that has already been
    /// let go. A second radio holding its own `Arc` keeps the link up, and the
    /// registry hands the replacement straight back to it — at the settings
    /// that radio established, which is the documented rule for a shared
    /// board.
    fn release(&mut self) {
        self.rx = None;
        self.board = None;
    }

    /// Hand on the radio's own PTT line — a foot switch, a mic button, or
    /// whatever is wired to the board's PTT input — so the engine keys on it
    /// exactly as it does on the on-screen button.
    ///
    /// Only changes are reported. A released stream answers `false`, which
    /// unkeys rather than leaving an over hanging on a line nobody is reading.
    fn poll_control(&mut self) -> Vec<ControlUpdate> {
        let ptt = self.rx.as_ref().is_some_and(|rx| rx.radio_ptt());
        if ptt == self.ptt {
            return Vec::new();
        }
        self.ptt = ptt;
        vec![ControlUpdate::Ptt(ptt)]
    }

    /// Load the transmit frequency while receiving, so an HL2IOBoard on the
    /// accessory bus can switch an amplifier, transverter or loop antenna to
    /// the right band before the operator keys.
    fn set_tx_freq_hz(&mut self, hz: f64) {
        if let Some(rx) = self.rx.as_ref() {
            rx.set_tx_freq(sdroxide_types::HpsdrConfig::apply_ppm(hz, self.ppm));
        }
    }

    fn tx_begin(&mut self, center_hz: f64, _rate: f64) -> Result<f64> {
        // Where the feedback will be, relative to the receiver — see
        // `pump_feedback`. The un-trimmed frequency on both sides: the ppm
        // correction is the same multiplier on each, so the difference the
        // loop needs is the same either way, and mixing the two would put a
        // few hertz of error into it.
        self.ps_tx_hz = center_hz;
        // A new over is a new alignment: the delay through the transmit FIFO,
        // the converters and the amplifier is constant *within* an over and
        // has no reason to be the same across a gap. The table itself is kept,
        // which is the point of having learned it.
        if let Some(ps) = self.puresignal.as_mut() {
            ps.unlock();
        }
        match self.rx.as_ref() {
            Some(rx) => {
                Ok(rx.tx_begin(sdroxide_types::HpsdrConfig::apply_ppm(center_hz, self.ppm)))
            }
            None => Ok(sdroxide_hpsdr::TX_RATE_HZ as f64),
        }
    }

    fn tx_write(&mut self, samples: &[Complex32]) -> Result<()> {
        if self.rx.is_none() {
            return Ok(());
        }
        // What the coupler heard of the *previous* blocks, before this one is
        // bent: the loop's own reference history is written by `predistort`
        // below, and feeding back against a reference that has not been
        // recorded yet would find nothing to align to.
        self.pump_feedback();
        // The predistorter needs to own the samples — it multiplies each by a
        // gain looked up from its table — and this one arrives as a shared
        // slice.
        let tx: &[Complex32] = match self.puresignal.as_mut() {
            Some(ps) => {
                self.ps_tx_iq.clear();
                self.ps_tx_iq.extend_from_slice(samples);
                ps.predistort(&mut self.ps_tx_iq);
                &self.ps_tx_iq
            }
            None => samples,
        };
        self.tx_scratch.clear();
        self.tx_scratch.reserve(tx.len() * 2);
        for s in tx {
            self.tx_scratch.push(s.re);
            self.tx_scratch.push(s.im);
        }
        let Some(rx) = self.rx.as_mut() else { return Ok(()) };
        rx.tx_write(&self.tx_scratch);
        Ok(())
    }

    fn tx_end(&mut self) -> Result<()> {
        if let Some(rx) = self.rx.as_ref() {
            rx.tx_end();
        }
        self.ps_tx_hz = 0.0;
        Ok(())
    }

    fn discard_pending_rx(&mut self) {
        if let Some(rx) = self.rx.as_mut() {
            rx.discard_pending_rx();
        }
    }

    /// The board's own MOX already covers this radio transmitting on its own
    /// DDC — the protocol threads watch it directly. This covers the other
    /// case: an HPSDR receiver lent to a different rig as a panadapter, where
    /// nothing on this connection is keyed and the ring still goes unread for
    /// somebody else's over. See [`IqSource::set_rx_paused`].
    fn set_rx_paused(&mut self, paused: bool) {
        if let Some(rx) = self.rx.as_ref() {
            rx.set_rx_paused(paused);
        }
    }

    /// See [`sdroxide_types::HpsdrConfig::tx_latency_ms`]: this board holds no
    /// transmit buffer of its own to widen, so raising this only widens the
    /// cushion the engine leaves before the board's TX ring on this side of
    /// the network.
    fn tx_pace_cushion_ms(&self) -> f64 {
        self.tx_latency_ms
    }
}
