//! An [`IqSource`] for a HackRF driven over USB by the native driver in
//! `sdroxide-hackrf` — no SoapySDR, no libusb, no libhackrf.
//!
//! The first native USB backend here that transmits, and it is half duplex:
//! receive stops for the length of an over. Two consequences show up in this
//! file rather than in the driver.
//!
//! **Drive stays digital.** [`IqSource::set_tx_drive`] and `set_tune_drive` are
//! left at their trait defaults, which looks like an omission and is not. The
//! engine applies drive to the samples unconditionally (`engine.rs:7205-7215`)
//! and only consults `commands_tx_power()` on the *audio* path
//! (`engine.rs:7338`), so a source that claimed power control and mapped drive
//! onto the transmit VGA would be attenuated twice — exactly what the trait
//! warns about at `source.rs:194-200`. The VGA is published as an ordinary
//! transmit gain element instead, which the generic slider drives and the
//! engine remembers across a reopen.
//!
//! **The transmitter is off unless it is armed.** `HackRfConfig::tx_enabled`
//! decides whether `hackrf_caps` publishes a transmit channel at all; with it
//! off, the engine's own capability check refuses to key. See `main.rs`.

use std::time::{Duration, Instant};

use sdroxide_dsp::IqCorrect;
use sdroxide_hackrf::HackRfHandle;
use sdroxide_hackrf::convert::{from_iq, sample_lut, to_iq};
use sdroxide_hackrf::protocol::GainSetter;
use sdroxide_radio::{Complex32, DC_BLOCK_HZ, IqSource, Result, lo_offset_for};
use sdroxide_types::HackRfConfig;

/// How long the radio may deliver nothing before the connection counts as
/// dead. Same three seconds as the RTL-SDR and the Airspy HF+: this is a local
/// USB device, so there is no network to be briefly slow.
///
/// The handle reports zero silence while the transmitter is keyed, so this
/// cannot fire during an over — see `HackRfHandle::silent_for`.
const SILENCE_BEFORE_REOPEN: Duration = Duration::from_secs(3);

/// How long a single `tx_write` will wait for room in the transmit ring.
///
/// The engine paces transmit blocks against the wall clock and expects this
/// call to block as backpressure. The bound exists so a stalled USB link ends
/// the over with an error rather than wedging the engine's transmit thread.
const TX_WRITE_TIMEOUT: Duration = Duration::from_secs(2);

/// A sample rate for an operator-facing message.
///
/// A HackRF Pro's floor is 200 ksps, and `{:.1} Msps` renders that as "0.2
/// Msps" — a figure with one significant digit pretending to be a limit. Below
/// a megasample the unit is kilosamples.
fn rate_text(rate_hz: f64) -> String {
    if rate_hz < 1.0e6 {
        format!("{:.0} ksps", rate_hz / 1e3)
    } else {
        format!("{:.1} Msps", rate_hz / 1e6)
    }
}

pub struct HackRfSource {
    handle: HackRfHandle,
    center: f64,
    label: String,
    lo_offset: f64,
    /// Wire bytes on their way from the ring to `read`'s complex buffer.
    rx_scratch: Vec<u8>,
    tx_scratch: Vec<u8>,
    /// Held across calls so a ring read that ends between an I and its Q does
    /// not swap the two for the rest of the session.
    carry: Option<u8>,
    lut: [f32; 256],
    /// Mirrors of what the panel drives, so `current_gains` can answer without
    /// a round trip to the stream thread.
    amp: bool,
    tx_amp: bool,
    bias_tee: bool,
    tx_enabled: bool,
    /// Whether the operator has pinned the baseband filter rather than leaving
    /// it automatic. Only needed to tell them when their radio ignores it.
    filter_override: bool,
    /// Front-end DC and image correction, applied to every block as it arrives
    /// so the panadapter, the demodulators and the skimmers all see the same
    /// clean stream. `None` while the operator has it switched off. See
    /// [`IqCorrect`].
    iq_correct: Option<IqCorrect>,
}

impl HackRfSource {
    pub fn open(cfg: &HackRfConfig, center_hz: f64) -> anyhow::Result<Self> {
        let handle = HackRfHandle::open(cfg, center_hz)?;
        let rate = handle.sample_rate_hz;
        let label = format!("{} @ {:.1} Msps", handle.label, rate / 1e6);
        // Computed once at open, from the filter the radio actually settled on
        // rather than the one that was asked for. `sdroxide-hackrf`'s
        // `auto_filter_bw` has a test pinning the two together, because a
        // filter chosen too narrow silently withdraws this offset.
        let lo_offset = lo_offset_for(rate, handle.filter_bw_hz as f64);
        tracing::info!(
            "HackRF source ready: {label}, center {center_hz:.0} Hz, \
             filter {:.2} MHz, LO offset {:.0} Hz, transmit {}",
            handle.filter_bw_hz as f64 / 1e6,
            lo_offset,
            if cfg.tx_enabled { "armed" } else { "off" },
        );
        Ok(HackRfSource {
            center: center_hz,
            label,
            lo_offset,
            rx_scratch: Vec::new(),
            tx_scratch: Vec::new(),
            carry: None,
            lut: sample_lut(),
            amp: cfg.amp,
            tx_amp: cfg.tx_amp,
            bias_tee: cfg.bias_tee,
            tx_enabled: cfg.tx_enabled,
            filter_override: cfg.filter_bw_hz > 0.0,
            iq_correct: cfg.iq_correction.then(|| IqCorrect::new(DC_BLOCK_HZ, rate)),
            handle,
        })
    }

    /// What this radio will tune: DC – 7.25 GHz, the firmware's own clamp and
    /// the same span libhackrf and SoapyHackRF publish. Not a per-board figure
    /// — see `sdroxide_hackrf::protocol::TUNING_RANGE_HZ` for why a board's
    /// *specified* coverage is the wrong thing to put in front of the dial.
    pub fn freq_range(&self) -> (f64, f64) {
        self.handle.freq_range
    }

    pub fn tx_enabled(&self) -> bool {
        self.tx_enabled
    }
}

impl IqSource for HackRfSource {
    /// The engine is transmitting and has stopped reading, or has started
    /// again. Passed straight through to the stream thread, which keeps
    /// receiving either way — this only decides whether a full ring is
    /// reported as an overrun or as the ordinary cost of an over. See
    /// [`IqSource::set_rx_paused`].
    fn set_rx_paused(&mut self, paused: bool) {
        self.handle.set_rx_paused(paused);
    }

    fn sample_rate(&self) -> f64 {
        self.handle.sample_rate_hz
    }

    fn center_hz(&self) -> f64 {
        self.center
    }

    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center = hz;
        self.handle.set_center_hz(hz);
        Ok(())
    }

    /// A zero-IF front end, so the LO is parked a quarter-span above the dial
    /// and the DC spike lands clear of the signal. This is the radio the policy
    /// was measured on — see `sdroxide_radio::source`.
    fn lo_offset_hz(&self) -> f64 {
        self.lo_offset
    }

    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        // Two wire bytes per complex sample, plus one for a byte that may be
        // carried in from the previous call.
        let need = buf.len() * 2;
        if self.rx_scratch.len() < need {
            self.rx_scratch.resize(need, 0);
        }
        let n = self.handle.rx_read(&mut self.rx_scratch[..need]);
        if n == 0 {
            // Nothing yet — brief nap so the DSP loop doesn't spin hot.
            std::thread::sleep(Duration::from_millis(2));
            return Ok(0);
        }
        // Converting here rather than on the stream thread is what lets the
        // ring carry bytes: a quarter of the memory at 20 Msps, and the work
        // lands on the thread that was going to touch these samples anyway.
        let mut out = Vec::with_capacity(n / 2 + 1);
        to_iq(&self.rx_scratch[..n], &self.lut, &mut self.carry, &mut out);
        let pairs = out.len().min(buf.len());
        buf[..pairs].copy_from_slice(&out[..pairs]);
        if let Some(iq) = self.iq_correct.as_mut() {
            iq.process(&mut buf[..pairs]);
        }
        Ok(pairs)
    }

    /// A centre of its own — the synthesiser's LO — so the engine may centre the
    /// I/Q on a resting dial (`Engine::recentre_after_dwell`).
    fn recentre_on_dwell(&self) -> bool {
        true
    }

    fn describe(&self) -> String {
        self.label.clone()
    }

    /// The two receive gain stages, plus pseudo-elements carrying the switches.
    ///
    /// Routing the switches through `SetGain` rather than adding `Command`
    /// variants keeps `Command`, `DeviceCaps` and the engine untouched for
    /// settings only this backend has. The names live on [`HackRfConfig`] so
    /// the two ends cannot drift apart.
    fn set_gain_element(&mut self, name: &str, db: f64) -> Result<()> {
        match name {
            HackRfConfig::LNA_ELEMENT => self.handle.set_gain(GainSetter::Lna, db),
            HackRfConfig::VGA_ELEMENT => self.handle.set_gain(GainSetter::Vga, db),
            HackRfConfig::AMP_ELEMENT => {
                self.amp = db >= 0.5;
                self.handle.set_amp(false, self.amp);
            }
            HackRfConfig::TXAMP_ELEMENT => {
                self.tx_amp = db >= 0.5;
                self.handle.set_amp(true, self.tx_amp);
            }
            HackRfConfig::BIAS_TEE_ELEMENT => {
                self.bias_tee = db >= 0.5;
                self.handle.set_bias_tee(self.bias_tee);
            }
            HackRfConfig::FILTER_ELEMENT => self.handle.set_filter_bw_hz(db.max(0.0)),
            HackRfConfig::PPM_ELEMENT => self.handle.set_ppm(db.clamp(-1000.0, 1000.0)),
            // Purely a host-side correction, so it switches on and off between
            // one block and the next with nothing to reconfigure on the radio.
            // Switching it on starts from a fresh estimate rather than whatever
            // the loop was left holding, which may be from another band.
            HackRfConfig::IQ_CORRECTION_ELEMENT => {
                if db >= 0.5 {
                    match self.iq_correct.as_mut() {
                        Some(iq) => iq.reset(),
                        None => {
                            self.iq_correct =
                                Some(IqCorrect::new(DC_BLOCK_HZ, self.handle.sample_rate_hz));
                        }
                    }
                } else {
                    self.iq_correct = None;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// The dB the hardware really applied, not what it was asked for.
    ///
    /// The LNA truncates to 8 dB steps and the VGA to 2, so a slider left at
    /// 15 dB is a radio at 8. Reporting the request back would make the UI
    /// disagree with the radio about something the operator can measure.
    fn current_gains(&self) -> Vec<(String, f64)> {
        let (lna, vga, _) = self.handle.gains();
        vec![
            (HackRfConfig::LNA_ELEMENT.to_string(), lna),
            (HackRfConfig::VGA_ELEMENT.to_string(), vga),
            (HackRfConfig::AMP_ELEMENT.to_string(), f64::from(u8::from(self.amp))),
        ]
    }

    // ---- transmit --------------------------------------------------------

    /// Key up. Returns the rate the radio is really running, which is the same
    /// rate it receives at — this hardware has one clock for both directions.
    fn tx_begin(&mut self, center_hz: f64, _rate: f64) -> Result<f64> {
        self.handle.begin_tx(center_hz);
        // The transition happens on the stream thread; wait for it so the
        // engine's first `tx_write` meets a keyed radio rather than a queue
        // that is about to be torn down and rebuilt.
        let deadline = Instant::now() + Duration::from_secs(2);
        while !self.handle.is_transmitting() && Instant::now() < deadline {
            if !self.handle.is_alive() {
                return Err(sdroxide_radio::RadioError::Msg(
                    "the HackRF stopped while keying up".into(),
                ));
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        if !self.handle.is_transmitting() {
            return Err(sdroxide_radio::RadioError::Msg(
                "the HackRF did not key up within two seconds".into(),
            ));
        }
        Ok(self.handle.sample_rate_hz)
    }

    fn tx_write(&mut self, samples: &[Complex32]) -> Result<()> {
        self.tx_scratch.clear();
        let mut saturated = 0u64;
        from_iq(samples, &mut self.tx_scratch, &mut saturated);
        let deadline = Instant::now() + TX_WRITE_TIMEOUT;
        let n = self.handle.tx_write(&self.tx_scratch, deadline);
        if n < self.tx_scratch.len() {
            return Err(sdroxide_radio::RadioError::Msg(format!(
                "the HackRF took only {n} of {} transmit bytes — the USB link is not \
                 keeping up",
                self.tx_scratch.len()
            )));
        }
        Ok(())
    }

    /// Wait for everything queued to reach the air.
    ///
    /// The engine calls this on the digital-burst path, where cutting the tail
    /// of an FT8 transmission is the difference between a decode and nothing.
    /// The driver drains on key-up as well, so an ordinary over is safe without
    /// this — but a burst asks explicitly and gets a longer, blocking wait.
    fn tx_drain(&mut self) {
        let deadline = Instant::now() + TX_WRITE_TIMEOUT;
        while !self.handle.tx_drained() && Instant::now() < deadline {
            if !self.handle.is_alive() {
                return;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    fn tx_end(&mut self) -> Result<()> {
        self.handle.end_tx();
        Ok(())
    }

    /// Throw away what the ring is holding after an over.
    ///
    /// On half-duplex hardware everything buffered when the receiver stopped
    /// is from *before* the transmission, so without this the operator hears
    /// the moment before they keyed up, delayed by the length of the over.
    fn discard_pending_rx(&mut self) {
        self.carry = None;
        self.handle.discard_rx();
    }

    fn set_tx_gain_element(&mut self, name: &str, db: f64) -> Result<()> {
        match name {
            HackRfConfig::TXVGA_ELEMENT => self.handle.set_gain(GainSetter::TxVga, db),
            HackRfConfig::TXAMP_ELEMENT => {
                self.tx_amp = db >= 0.5;
                self.handle.set_amp(true, self.tx_amp);
            }
            _ => {}
        }
        Ok(())
    }

    fn current_tx_gains(&self) -> Vec<(String, f64)> {
        let (_, _, txvga) = self.handle.gains();
        vec![
            (HackRfConfig::TXVGA_ELEMENT.to_string(), txvga),
            (HackRfConfig::TXAMP_ELEMENT.to_string(), f64::from(u8::from(self.tx_amp))),
        ]
    }

    /// A radio that has been unplugged, or whose thread has died, is reported
    /// as needing a reopen so the engine reconnects on its own — which is what
    /// makes replugging one Just Work rather than needing Apply pressed.
    ///
    /// The silence window cannot fire mid-over: the handle reports zero while
    /// the transmitter is keyed, because on half-duplex hardware an over is
    /// exactly what a dead receiver looks like.
    fn needs_reopen(&self) -> bool {
        !self.handle.is_alive() || self.handle.silent_for() >= SILENCE_BEFORE_REOPEN
    }

    /// Hand the radio back before the engine opens its replacement. Without
    /// this, changing anything in Settings → Radio on a running HackRF fails
    /// with "held by another program" — the other program being us.
    ///
    /// The stream thread unkeys on its way out, so a release in the middle of
    /// an over takes the radio off the air rather than leaving it keyed.
    fn release(&mut self) {
        self.handle.release();
    }

    /// Surface what an operator needs to know but cannot see.
    fn open_status(&self) -> Option<String> {
        let mut parts: Vec<String> = Vec::new();
        match (self.bias_tee, self.handle.has_bias_tee) {
            (true, true) => {
                parts.push(format!("{}: bias tee is ON — DC on the antenna coax", self.label))
            }
            // The switch is on and the board has no such circuit. The driver
            // does not send the request at all, so nothing happens — and
            // "nothing happens" is exactly what an operator whose preamp is
            // dark cannot distinguish from a broken feedline.
            (true, false) => parts.push(format!(
                "{}: bias tee is switched on, but this board has no bias-tee \
                 circuit — nothing is being powered",
                self.label
            )),
            _ => {}
        }
        if self.tx_enabled {
            // A standing reminder, because the setting is persisted and
            // survives a restart with nothing else on screen to indicate it.
            parts.push(
                "transmit is ARMED — this radio's harmonics need an external low-pass \
                 filter for the band you are on, and it is half duplex, so receive \
                 stops for the length of every over"
                    .to_string(),
            );
        }
        if let Some(from) = self.handle.snapped_from {
            let (min, max) = self.handle.rate_range;
            parts.push(format!(
                "{} is outside this radio's {}–{}; using {}. \
                 Pick a listed rate in Settings → Radio to make this permanent.",
                rate_text(from),
                rate_text(min),
                rate_text(max),
                rate_text(self.handle.sample_rate_hz),
            ));
        }
        // The switch is set and this board derives its own filter. Same shape
        // as the bias-tee case above: the driver does not send the request, so
        // nothing happens, and "nothing happens" is not something an operator
        // can see.
        if self.filter_override && self.handle.filter_is_automatic {
            parts.push(format!(
                "the baseband filter is pinned in Settings → Radio, but this board \
                 chooses its own — it is running {:.3} MHz for {:.3} Msps",
                self.handle.filter_bw_hz as f64 / 1e6,
                self.handle.sample_rate_hz / 1e6,
            ));
        }
        if let Some(w) = &self.handle.link_warning {
            parts.push(w.clone());
        }
        // The dial and the radio have parted company: the engine was told this
        // tune succeeded when it was posted to the stream thread, and the radio
        // has been refusing it ever since. Without this the panadapter goes on
        // labelling a frequency the radio is not on, and nothing anywhere says
        // why (issue #352).
        if self.handle.ctrl_stuck() {
            parts.push(
                "this radio is refusing settings changes — the frequency and gains on \
                 screen are not what it is running. Unplug it and plug it back in"
                    .to_string(),
            );
        }
        (!parts.is_empty()).then(|| parts.join(" — "))
    }
}
