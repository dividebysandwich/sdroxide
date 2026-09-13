//! An [`IqSource`] for one receive chain of an ADALM-Pluto driven over IIOD by
//! the native driver in `sdroxide-pluto` — no libiio, no libSoapySDR.
//!
//! The AD9361 delivers wideband complex I/Q, so this drives the engine's normal
//! DDC/demod path exactly like a SoapySDR device (`audio_mode = false`), and
//! transmit is modulated I/Q rather than audio the rig modulates.
//!
//! The connection is shared: a 2R2T firmware (a Pluto+) streams two receive
//! chains, so two radio tabs on the same address each take one over a single
//! connection — the [`crate::device_registry`] pairs the second source with
//! the first one's rig. What the chains do **not** have is their own LOs: the
//! AD9361's receive chains share one synthesiser, so either radio retuning
//! moves both, and the sibling learns of it through
//! [`sdroxide_radio::ControlUpdate::Center`] — its span simply is somewhere
//! else now. The transmitter belongs to the chain-0 radio.
//!
//! # Zero IF
//!
//! Unlike every other native backend here, the Pluto's front end is zero-IF:
//! LO leakage, DC offset and flicker noise all pile up exactly where the
//! operator's VFO would otherwise sit. So this source does the two things the
//! SoapySDR path does for the same reason — it asks the engine to park the LO a
//! quarter-span away ([`IqSource::lo_offset_hz`]) and DC-blocks the stream
//! before anything downstream sees it.

use std::time::Duration;

use sdroxide_dsp::ComplexDcBlock;
use sdroxide_pluto::{PlutoRig, PlutoRx};
use sdroxide_radio::{Complex32, ControlUpdate, DC_BLOCK_HZ, IqSource, Result, lo_offset_for};
use sdroxide_types::{PlutoAgc, PlutoConfig};

use crate::device_registry::{DeviceKey, SharedDevice, registry};

impl SharedDevice for PlutoRig {
    fn is_alive(&self) -> bool {
        PlutoRig::is_alive(self)
    }
}

/// How long the device may deliver nothing before the connection counts as
/// dead and the engine starts reconnecting. This is a network rig, and a Pluto
/// that has just been re-plugged takes a while to bring its interface back up.
///
/// The backstop, not the primary detector. A socket that stops delivering is
/// now handled where it happens: the IIOD layer waits out a short gap, and
/// failing that replaces the receive socket and reopens the buffer on its own
/// (`stream::redial_rx`), which costs tens of milliseconds against the second
/// or more a reopen from here costs. What still reaches this point is the case
/// that cannot be fixed one socket at a time — a board that has gone away, or
/// one whose receive keeps stalling however often it is redialled.
///
/// It was five seconds, which is shorter than a stall the link recovers from
/// on its own — so a hiccup the read layer had already absorbed still cost a
/// teardown here.
const SILENCE_BEFORE_REOPEN: Duration = Duration::from_secs(10);

pub struct PlutoSource {
    /// The shared connection and this source's stream on it. `None` after
    /// [`IqSource::release`]: the stream is given back so a rebuilt source
    /// can claim the chain again, while a sibling's stream on the same
    /// connection runs on undisturbed.
    rig: Option<std::sync::Arc<PlutoRig>>,
    rx: Option<PlutoRx>,
    center: f64,
    rx_scratch: Vec<f32>,
    tx_scratch: Vec<f32>,
    dc: ComplexDcBlock,
    lo_offset: f64,
    /// The connection's sample rate, kept here so a released source still
    /// answers `IqSource::sample_rate` — see that method.
    rate: f64,
    label: String,
    /// The receive AGC mode this chain was last put in. Published beside the
    /// gain so a panel can tell that the gain register is the AD9361's while
    /// an attack mode runs, and a gain slider moves nothing (issue #417).
    agc: PlutoAgc,
}

impl PlutoSource {
    /// Attach receive chain `cfg.rx` of the Pluto at `address`, connecting
    /// only if no radio in this process already holds that connection, and
    /// start receiving at `center_hz`. The rate, bandwidth, reference trim,
    /// duplex and GPO transmit-receive pins are connection-level: whoever
    /// connects first sets them, and a later attach runs with the established
    /// ones whatever its own config says. (Which is why the capabilities read duplex back off the rig rather
    /// than out of `cfg` — the engine must be told what the link is actually
    /// doing, not what this radio asked for.)
    pub fn open(address: &str, cfg: &PlutoConfig, center_hz: f64) -> anyhow::Result<Self> {
        let rig = registry()
            .get_or_open(DeviceKey::Pluto(address.to_string()), || {
                PlutoRig::open(address, cfg, center_hz)
                    .map(std::sync::Arc::new)
                    .map_err(|e| e.to_string())
            })
            .map_err(anyhow::Error::msg)?;
        let rx = rig.rx(cfg.rx).map_err(|e| anyhow::Error::msg(e.to_string()))?;
        rx.set_rx_freq(center_hz);
        let rate = rig.sample_rate_hz();
        // Decided against the analog filter we set ourselves — see
        // `sdroxide_radio::lo_offset_for` for why that filter is opened up
        // rather than left at the AD9361's default.
        let lo_offset = lo_offset_for(rate, rig.rf_bandwidth_hz());
        let label =
            if cfg.rx == 0 { rig.label() } else { format!("RX{} {}", cfg.rx + 1, rig.label()) };
        // Worth a word wherever the label shows (window title, settings): on
        // a 2R2T build a sibling radio's retune moves this radio too.
        let label = if rig.rx_chains() > 1 { format!("{label} — shared LO") } else { label };
        tracing::info!(
            "PlutoSDR source ready: {label}, centre {center_hz:.0} Hz, \
             LO offset {lo_offset:.0} Hz (0 = LO on the VFO)"
        );
        Ok(PlutoSource {
            center: center_hz,
            rx_scratch: Vec::new(),
            tx_scratch: Vec::new(),
            dc: ComplexDcBlock::new(DC_BLOCK_HZ, rate),
            lo_offset,
            rate,
            label,
            rx: Some(rx),
            rig: Some(rig),
            agc: cfg.agc,
        })
    }

    /// What the device says it can do — the source of every figure in
    /// `pluto_caps`.
    pub fn limits(&self) -> Option<&sdroxide_pluto::PlutoLimits> {
        self.rig.as_deref().map(PlutoRig::limits)
    }

    /// Whether receive runs through an over on this connection.
    pub fn full_duplex(&self) -> bool {
        self.rig.as_deref().is_some_and(PlutoRig::full_duplex)
    }

    /// Drain what the receive thread has queued. `wait` naps briefly on an
    /// empty ring, which is what keeps the engine's receive loop off a hot
    /// spin; a full-duplex over passes `false` and takes the empty answer.
    fn take(&mut self, buf: &mut [Complex32], wait: bool) -> Result<usize> {
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
            if wait {
                // Nothing yet — brief nap so the DSP loop doesn't spin hot.
                std::thread::sleep(Duration::from_millis(2));
            }
            return Ok(0);
        }
        for p in 0..pairs {
            buf[p] = Complex32::new(self.rx_scratch[2 * p], self.rx_scratch[2 * p + 1]);
        }
        // Deliberately not reset across an over: the offset is a property of
        // the hardware, not of the stream, so carrying the estimate avoids a
        // re-convergence transient every time receive resumes.
        self.dc.process(&mut buf[..pairs]);
        Ok(pairs)
    }

    /// How many receive chains this firmware streams.
    pub fn rx_chains(&self) -> u8 {
        self.rig.as_deref().map_or(1, PlutoRig::rx_chains)
    }
}

impl IqSource for PlutoSource {
    /// The connection's rate, remembered rather than asked for: it is fixed
    /// when the Pluto is opened, and [`IqSource::release`] lets the connection
    /// go while the engine is still running on this source.
    fn sample_rate(&self) -> f64 {
        self.rate
    }

    fn center_hz(&self) -> f64 {
        self.center
    }

    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center = hz;
        if let Some(rx) = self.rx.as_ref() {
            rx.set_rx_freq(hz);
        }
        Ok(())
    }

    fn lo_offset_hz(&self) -> f64 {
        self.lo_offset
    }

    /// LO moves a sibling stream commanded arrive here as centre changes, for
    /// the engine to adopt — the chains share the one synthesiser, so this
    /// source's span moved whether its operator asked or not.
    fn poll_control(&mut self) -> Vec<ControlUpdate> {
        let Some(rx) = self.rx.as_ref() else { return Vec::new() };
        rx.poll_lo_moves()
            .into_iter()
            .inspect(|hz| self.center = *hz)
            .map(ControlUpdate::Center)
            .collect()
    }

    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        self.take(buf, true)
    }

    /// What a full-duplex over reads with: the same drain, without the nap.
    ///
    /// The engine's thread owes the transmitter a block every 10 ms while it is
    /// keyed, so two milliseconds spent waiting for receive is a fifth of that
    /// budget spent on the wrong direction — and the transmit ring emptying is
    /// heard on the air, where an empty receive block is not heard at all.
    fn read_available(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        self.take(buf, false)
    }

    fn describe(&self) -> String {
        self.label.clone()
    }

    /// The AD9361's receive gain, plus the two pseudo-elements this backend
    /// carries on the same command — the AGC mode and the reference trim. See
    /// [`PlutoConfig::AGC_ELEMENT`] for why they ride `SetGain` rather than
    /// having `Command` variants of their own.
    fn set_gain_element(&mut self, name: &str, db: f64) -> Result<()> {
        let Some(rx) = self.rx.as_mut() else { return Ok(()) };
        match name {
            PlutoConfig::RF_GAIN_ELEMENT => rx.set_rx_gain_db(db),
            PlutoConfig::AGC_ELEMENT => {
                self.agc = PlutoAgc::from_code(db);
                rx.set_agc_mode(self.agc.iio_name());
            }
            PlutoConfig::PPM_ELEMENT => rx.set_ppm(db),
            _ => {}
        }
        Ok(())
    }

    fn current_gains(&self) -> Vec<(String, f64)> {
        match self.rx.as_ref() {
            Some(rx) => vec![
                (PlutoConfig::RF_GAIN_ELEMENT.to_string(), rx.rx_gain_db()),
                (PlutoConfig::AGC_ELEMENT.to_string(), self.agc.code()),
            ],
            None => Vec::new(),
        }
    }

    fn set_tx_gain_element(&mut self, name: &str, db: f64) -> Result<()> {
        if name == PlutoConfig::TX_GAIN_ELEMENT
            && let Some(rx) = self.rx.as_mut()
        {
            rx.set_tx_gain_db(db);
        }
        Ok(())
    }

    fn current_tx_gains(&self) -> Vec<(String, f64)> {
        match self.rx.as_ref() {
            Some(rx) => vec![(PlutoConfig::TX_GAIN_ELEMENT.to_string(), rx.tx_gain_db())],
            None => Vec::new(),
        }
    }

    /// `rf_port_select`. A stock Pluto wires only `A_BALANCED` and `A`, but the
    /// AD9361 has nine receive ports and a board built around one may use
    /// another, so whatever the device published is offered.
    fn set_antenna(&mut self, name: &str) -> Result<()> {
        if let Some(rx) = self.rx.as_mut() {
            rx.set_rx_port(name);
        }
        Ok(())
    }

    fn current_antenna(&self) -> String {
        self.rx.as_ref().map_or_else(String::new, |rx| rx.rx_port().to_string())
    }

    fn set_tx_antenna(&mut self, name: &str) -> Result<()> {
        if let Some(rx) = self.rx.as_mut() {
            rx.set_tx_port(name);
        }
        Ok(())
    }

    fn current_tx_antenna(&self) -> String {
        self.rx.as_ref().map_or_else(String::new, |rx| rx.tx_port().to_string())
    }

    fn tx_begin(&mut self, center_hz: f64, _rate: f64) -> Result<f64> {
        match self.rx.as_ref() {
            Some(rx) => Ok(rx.tx_begin(center_hz)),
            None => Ok(0.0),
        }
    }

    fn tx_write(&mut self, samples: &[Complex32]) -> Result<()> {
        let Some(rx) = self.rx.as_mut() else { return Ok(()) };
        self.tx_scratch.clear();
        self.tx_scratch.reserve(samples.len() * 2);
        for s in samples {
            self.tx_scratch.push(s.re);
            self.tx_scratch.push(s.im);
        }
        rx.tx_write(&self.tx_scratch);
        Ok(())
    }

    /// Let the queued samples reach the device before PTT drops. The engine
    /// hands us a burst faster than real time and the hardware drains it one
    /// buffer at a time, so unkeying immediately would cut the tail — which for
    /// FT8 is the difference between a decode and nothing.
    fn tx_drain(&mut self) {
        let Some(rx) = self.rx.as_ref() else { return };
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while rx.tx_pending() > 0 && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    fn tx_end(&mut self) -> Result<()> {
        if let Some(rx) = self.rx.as_ref() {
            rx.tx_end();
        }
        Ok(())
    }

    /// Receive is torn down for the length of an over, but a partial buffer can
    /// still be sitting in the ring when it resumes.
    ///
    /// Not in full duplex, where receive never stopped: the ring holds the last
    /// few milliseconds of a *live* signal, and throwing it away would put a
    /// gap in the audio at every unkey — the one moment an operator is
    /// listening hardest.
    fn discard_pending_rx(&mut self) {
        if self.full_duplex() {
            return;
        }
        if let Some(rx) = self.rx.as_mut() {
            rx.discard_pending_rx();
        }
    }

    /// Only ever reached with `full_duplex` off — the engine does not call this
    /// otherwise — and then only when this Pluto is somebody else's panadapter:
    /// keying its own transmitter closes the receive buffer for the length of
    /// the over, so there is nothing arriving to account for. See
    /// [`IqSource::set_rx_paused`].
    fn set_rx_paused(&mut self, paused: bool) {
        if let Some(rx) = self.rx.as_ref() {
            rx.set_rx_paused(paused);
        }
    }

    fn open_status(&self) -> Option<String> {
        self.rig.as_deref().and_then(PlutoRig::open_status)
    }

    /// A Pluto that has stopped delivering samples — unplugged, rebooted, its
    /// interface reconfigured, or its buffer taken by another program — is
    /// reported as needing a reopen so the engine reconnects on its own. A
    /// released source likewise.
    fn needs_reopen(&self) -> bool {
        self.rx.as_ref().is_none_or(|rx| !rx.is_alive() || rx.silent_for() >= SILENCE_BEFORE_REOPEN)
    }

    /// Give this chain's stream back ahead of a rebuild — **and the connection
    /// with it, unless a sibling radio is still streaming the other chain.**
    ///
    /// The connection used to be kept deliberately, so that an Apply with the
    /// address unchanged re-attached over the live link (the registry finds it
    /// through this very `Arc`) rather than redialling the board. That saved a
    /// second and cost the operator every setting they had just changed:
    /// everything the session is made of — the sample rate, the RF bandwidth,
    /// the duplex, and which GPO pins key the amplifier — is decided in
    /// `PlutoRig::open` and reaches the AD9361 through an `initialize` that
    /// re-runs its whole setup. A live connection outliving its last radio
    /// makes an Apply that changes any of them do nothing at all, which is
    /// exactly what issue #135 reported: the GPO pair and the sample rate only
    /// took effect after quitting and restarting sdroxide. The RSP backend
    /// already closes on the last stream for the same reason.
    ///
    /// So the `Arc` goes here. Dropping the last one runs the connection's own
    /// teardown — threads joined, sockets shut down — *before* this returns, so
    /// the redial that follows meets a Pluto whose buffer `iiod` has already
    /// let go of rather than the "device busy" a premature close would give.
    /// A sibling holding its own `Arc` keeps the link up, and the registry
    /// hands the replacement straight back to it.
    ///
    /// A connection that has already failed is released explicitly rather than
    /// merely dropped. Its receive thread may still be sitting in a `READBUF`
    /// that has seconds left to run, and until that returns the *device's*
    /// buffer stays open — the reconnect this release is preparing for would
    /// be refused as busy, back off, and try again: the several-second gap
    /// between "the radio froze" and "the radio came back" that has nothing to
    /// do with what broke the link. Shutting the sockets down here ends that
    /// read at once, whether or not a sibling still holds the `Arc`.
    fn release(&mut self) {
        self.rx = None;
        let Some(rig) = self.rig.take() else { return };
        if !rig.is_alive() {
            rig.release();
        }
        // and the drop below closes a healthy one, if this was the last holder
    }
}
