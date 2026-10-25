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
//!
//! # The second chain as this radio's own (issue #525)
//!
//! On a 2R2T board the RX1 radio can keep RX2 for itself instead of leaving it
//! to a second tab — as a second aerial, combined with the first by the same
//! adaptive filter a LimeSDR's second chain uses, or as the transmit coupler
//! PureSignal learns the amplifier from. Both chains arrive in one device
//! buffer, so this is the best-aligned pair any backend here has: the
//! hardware interleaves the two, and `PlutoRx::rx_read_paired` keeps them that
//! way on the host. PureSignal also needs full duplex, since the coupler is
//! only heard while receive keeps running through the over.

use std::time::{Duration, Instant};

use sdroxide_dsp::{ComplexDcBlock, Diversity, PureSignal};
use sdroxide_pluto::{PlutoRig, PlutoRx};
use sdroxide_radio::{Complex32, ControlUpdate, DC_BLOCK_HZ, IqSource, Result, lo_offset_for};
use sdroxide_types::{DiversityMode, LimeAuxRole, PlutoAgc, PlutoAuxConfig, PlutoConfig};

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

/// How often the diversity filter's null depth, or the predistortion loop's
/// state, reaches the log — the same cadence the LimeSDR backend keeps.
const AUX_LOG_INTERVAL: Duration = Duration::from_secs(10);

/// How far back PureSignal may look for the transmission in RX2's samples.
///
/// The LimeSDR's tenth of a second is far too short here, and that is what a
/// first test on a PlutoSky R2 showed as `PS --` on every over: between the
/// predistorter and the coupler's samples coming back lie the host's transmit
/// ring, the board's DMA buffers in both directions, the network twice, and
/// the receive ring — easily a few hundred milliseconds. One second covers it
/// with room to spare; the search is throttled so the longer history costs
/// nothing while unlocked.
const PS_HISTORY_S: f64 = 1.0;

/// The second receive chain, borrowed by the RX1 radio (issue #525).
struct Aux {
    rx: PlutoRx,
    /// Interleaved floats as they come off the ring, and the same as complex.
    raw: Vec<f32>,
    iq: Vec<Complex32>,
    /// The second chain's own DC blocker: it is a separate zero-IF front end
    /// with an offset of its own, and handing that to the canceller would have
    /// it spend taps subtracting one chain's artefact from the other's.
    dc: ComplexDcBlock,
    diversity: Option<Diversity>,
    puresignal: Option<PureSignal>,
    cfg: PlutoAuxConfig,
    last_log: Instant,
    /// Said once per over: past the edge of the span the coupler cannot be
    /// heard.
    warned_offset: bool,
}

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
    /// RX2 as this radio's second aerial or PureSignal coupler, when
    /// configured and available.
    aux: Option<Aux>,
    /// Why the second chain is not running although it was asked for — shown
    /// beside the connection's own open status.
    aux_note: Option<String>,
    /// The transmit oscillator of the over in progress, from `tx_begin`: the
    /// coupler's signal lands at its distance from the receive centre.
    tx_hz: f64,
    transmitting: bool,
    /// The transmit block, predistorted, as complex.
    ps_scratch: Vec<Complex32>,
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
        let (aux, aux_note) = Self::open_aux(&rig, cfg, rate);
        Ok(PlutoSource {
            aux,
            aux_note,
            tx_hz: center_hz,
            transmitting: false,
            ps_scratch: Vec::new(),
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

    /// Borrow RX2 for this radio, if its configuration asks for it and the
    /// board and the link can give it. Anything short of that is a note, not
    /// a failed open: the operator still has their receiver.
    fn open_aux(rig: &PlutoRig, cfg: &PlutoConfig, rate: f64) -> (Option<Aux>, Option<String>) {
        let role = cfg.aux.role;
        if role == LimeAuxRole::Off {
            return (None, None);
        }
        let refuse = |why: String| {
            tracing::warn!("PlutoSDR: {why}");
            (None, Some(why))
        };
        if cfg.rx != 0 {
            return refuse(
                "the second receive chain is set up on the RX1 radio, not on RX2's — \
                 this radio is RX2 itself"
                    .into(),
            );
        }
        if rig.rx_chains() < 2 {
            return refuse(
                "this firmware streams one receive chain, so there is no second one for \
                 diversity or PureSignal — a 2R2T firmware is needed"
                    .into(),
            );
        }
        if role == LimeAuxRole::PureSignal && !rig.full_duplex() {
            return refuse(
                "PureSignal is off: it listens to the coupler while you transmit, so it \
                 needs Full duplex on (and FDD, with the PTT pins off)"
                    .into(),
            );
        }
        let rx = match rig.aux_rx() {
            Ok(rx) => rx,
            Err(e) => return refuse(format!("the second receive chain was not opened: {e}")),
        };
        // A fixed gain, whatever the main chain's AGC is doing. An AGC on the
        // coupler would re-scale the feedback the predistorter is measuring,
        // and one on a second aerial would move the balance the combiner has
        // just learned.
        rx.set_agc_mode(PlutoAgc::Manual.iio_name());
        let mut rx = rx;
        rx.set_rx_gain_db(cfg.aux.gain_db);
        let diversity = (role == LimeAuxRole::Diversity).then(|| {
            Diversity::new(div_mode(cfg.aux.mode), usize::from(cfg.aux.taps), cfg.aux.rate)
        });
        let puresignal = (role == LimeAuxRole::PureSignal).then(|| {
            let mut ps = PureSignal::with_history(
                usize::from(cfg.aux.ps_bins),
                cfg.aux.ps_rate,
                rate,
                PS_HISTORY_S,
            );
            ps.set_frozen(cfg.aux.ps_frozen);
            ps
        });
        let mut diversity = diversity;
        if let Some(d) = diversity.as_mut() {
            d.set_frozen(cfg.aux.frozen);
            tracing::info!(
                "PlutoSDR: diversity is on — second aerial on RX2, {} filter, {} taps, \
                 {} dB fixed gain",
                match cfg.aux.mode {
                    DiversityMode::Cancel => "cancelling",
                    DiversityMode::Combine => "combining",
                },
                cfg.aux.taps,
                cfg.aux.gain_db
            );
        }
        if puresignal.is_some() {
            tracing::info!(
                "PlutoSDR: PureSignal is on — transmit feedback on RX2 at {} dB, {} table \
                 steps; the correction stays at unity until the feedback lines up with \
                 what was sent",
                cfg.aux.gain_db,
                cfg.aux.ps_bins
            );
        }
        let aux = Aux {
            rx,
            raw: Vec::new(),
            iq: Vec::new(),
            dc: ComplexDcBlock::new(DC_BLOCK_HZ, rate),
            diversity,
            puresignal,
            cfg: cfg.aux.clone(),
            last_log: Instant::now(),
            warned_offset: false,
        };
        (Some(aux), None)
    }

    /// Whether a diversity filter is running, so the main window's DIV strip
    /// is shown for this radio.
    pub fn diversity_running(&self) -> bool {
        self.aux.as_ref().is_some_and(|a| a.diversity.is_some())
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
        let (n, paired) = match self.aux.as_mut() {
            Some(aux) => {
                if aux.raw.len() < need {
                    aux.raw.resize(need, 0.0);
                }
                rx.rx_read_paired(&mut aux.rx, &mut self.rx_scratch[..need], &mut aux.raw[..need])
            }
            None => (rx.rx_read(&mut self.rx_scratch[..need]), 0),
        };
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
        if paired == n {
            self.use_aux(&mut buf[..pairs]);
        }
        self.log_aux();
        Ok(pairs)
    }

    /// Give the second chain's samples of the block in `main` to whichever
    /// loop is running on them. `aux.raw` holds exactly `main.len()` pairs.
    fn use_aux(&mut self, main: &mut [Complex32]) {
        let Some(aux) = self.aux.as_mut() else { return };
        let pairs = main.len();
        aux.iq.clear();
        aux.iq.extend((0..pairs).map(|p| Complex32::new(aux.raw[2 * p], aux.raw[2 * p + 1])));
        if let Some(d) = aux.diversity.as_mut() {
            // Each chain's own DC offset out first: they are artefacts of two
            // separate front ends and have nothing in common.
            aux.dc.process(&mut aux.iq);
            d.process(main, &aux.iq);
        }
        if let Some(ps) = aux.puresignal.as_mut() {
            // Only while keyed: the loop measures its delay once per over and
            // holds it, so the feedback it sees must be one unbroken stretch
            // of the transmission, not the receiver's idle noise either side.
            if !self.transmitting {
                aux.warned_offset = false;
                return;
            }
            // The two synthesisers are set separately — the transmit one on the
            // carrier, the receive one a quarter span off the dial — so the
            // coupled signal lands at their difference, which is known exactly
            // and spun back out inside the loop.
            let offset = self.tx_hz - self.center;
            if offset.abs() > self.rate * 0.45 {
                if !aux.warned_offset {
                    aux.warned_offset = true;
                    tracing::warn!(
                        "PlutoSDR: the transmit frequency is {:.3} MHz from the receive \
                         centre, outside the captured span — PureSignal cannot hear the \
                         coupler this over",
                        offset / 1e6
                    );
                }
                return;
            }
            ps.feed_back(&aux.iq, offset, self.rate);
        }
    }

    /// Say how the second chain's loop is doing, now and then. The null depth
    /// and the predistortion lock are the numbers that say whether any of it
    /// works, and the log is where the LimeSDR backend puts them too.
    fn log_aux(&mut self) {
        let transmitting = self.transmitting;
        let Some(aux) = self.aux.as_mut() else { return };
        if aux.last_log.elapsed() < AUX_LOG_INTERVAL {
            return;
        }
        if let Some(d) = aux.diversity.as_ref() {
            aux.last_log = Instant::now();
            if let Some(db) = d.depth_db() {
                tracing::info!(
                    "PlutoSDR diversity: {db:.1} dB of the main aerial's signal is being \
                     cancelled{}",
                    if d.frozen() { ", filter held" } else { "" }
                );
            }
        }
        if let Some(ps) = aux.puresignal.as_ref().filter(|_| transmitting) {
            aux.last_log = Instant::now();
            if ps.locked() {
                tracing::info!(
                    "PlutoSDR PureSignal: correcting {:.1} dB of compression (feedback \
                     matched at {:.2}){}",
                    ps.correction_db(),
                    ps.score(),
                    if ps.frozen() { ", table held" } else { "" }
                );
            } else {
                tracing::info!(
                    "PlutoSDR PureSignal: the transmission has not been found on RX2 (best \
                     match {:.2}) — the transmitter is uncorrected. Check the coupler, and \
                     that RX2's gain is low enough not to be driven into compression",
                    ps.score()
                );
            }
        }
    }

    /// The second chain's pseudo-elements. Answers whether `name` was one.
    fn set_aux_element(&mut self, name: &str, v: f64) -> bool {
        let Some(aux) = self.aux.as_mut() else {
            return matches!(
                name,
                PlutoConfig::AUX_GAIN_ELEMENT
                    | PlutoConfig::DIV_MODE_ELEMENT
                    | PlutoConfig::DIV_RATE_ELEMENT
                    | PlutoConfig::DIV_TAPS_ELEMENT
                    | PlutoConfig::DIV_FREEZE_ELEMENT
                    | PlutoConfig::DIV_RESET_ELEMENT
                    | PlutoConfig::PS_BINS_ELEMENT
                    | PlutoConfig::PS_RATE_ELEMENT
                    | PlutoConfig::PS_FREEZE_ELEMENT
                    | PlutoConfig::PS_RESET_ELEMENT
            );
        };
        let on = v >= 0.5;
        match name {
            PlutoConfig::AUX_GAIN_ELEMENT => {
                aux.cfg.gain_db = v;
                aux.rx.set_rx_gain_db(v);
            }
            PlutoConfig::DIV_MODE_ELEMENT => {
                aux.cfg.mode = if on { DiversityMode::Combine } else { DiversityMode::Cancel };
                if let Some(d) = aux.diversity.as_mut() {
                    d.set_mode(div_mode(aux.cfg.mode));
                }
            }
            PlutoConfig::DIV_RATE_ELEMENT => {
                aux.cfg.rate = v as f32;
                if let Some(d) = aux.diversity.as_mut() {
                    d.set_rate(v as f32);
                }
            }
            PlutoConfig::DIV_TAPS_ELEMENT => {
                let taps = v.round().clamp(1.0, f64::from(PlutoAuxConfig::MAX_TAPS)) as u8;
                aux.cfg.taps = taps;
                if let Some(d) = aux.diversity.as_mut() {
                    // Starts the filter again: the taps mean different delays.
                    d.set_taps(usize::from(taps));
                }
            }
            PlutoConfig::DIV_FREEZE_ELEMENT => {
                aux.cfg.frozen = on;
                if let Some(d) = aux.diversity.as_mut() {
                    d.set_frozen(on);
                }
            }
            PlutoConfig::DIV_RESET_ELEMENT => {
                if let Some(d) = aux.diversity.as_mut().filter(|_| on) {
                    d.reset();
                }
            }
            PlutoConfig::PS_RATE_ELEMENT => {
                aux.cfg.ps_rate = v as f32;
                if let Some(ps) = aux.puresignal.as_mut() {
                    ps.set_rate(v as f32);
                }
            }
            PlutoConfig::PS_FREEZE_ELEMENT => {
                aux.cfg.ps_frozen = on;
                if let Some(ps) = aux.puresignal.as_mut() {
                    ps.set_frozen(on);
                }
            }
            PlutoConfig::PS_RESET_ELEMENT => {
                if let Some(ps) = aux.puresignal.as_mut().filter(|_| on) {
                    ps.reset();
                }
            }
            PlutoConfig::PS_BINS_ELEMENT => {
                let bins = v.round().clamp(
                    f64::from(PlutoAuxConfig::PS_MIN_BINS),
                    f64::from(PlutoAuxConfig::PS_MAX_BINS),
                ) as u8;
                aux.cfg.ps_bins = bins;
                if aux.puresignal.is_some() {
                    // A new table means learning it again.
                    let mut ps = PureSignal::with_history(
                        usize::from(bins),
                        aux.cfg.ps_rate,
                        self.rate,
                        PS_HISTORY_S,
                    );
                    ps.set_frozen(aux.cfg.ps_frozen);
                    aux.puresignal = Some(ps);
                }
            }
            _ => return false,
        }
        true
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
        // The borrowed chain is told about LO moves like any stream, but it
        // follows this radio's centre by construction — so its notices are
        // only drained, or they would pile up for the life of the session.
        if let Some(aux) = self.aux.as_ref() {
            let _ = aux.rx.poll_lo_moves();
        }
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
            other => {
                self.set_aux_element(other, db);
            }
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
        self.tx_hz = center_hz;
        self.transmitting = true;
        // A new over: the table (the amplifier's curve) carries over, but the
        // two sample counts the alignment is measured between restart here,
        // so the delay is found afresh rather than inherited from an over
        // whose tail never reached the coupler.
        if let Some(ps) = self.aux.as_mut().and_then(|a| a.puresignal.as_mut()) {
            ps.unlock();
        }
        match self.rx.as_ref() {
            Some(rx) => Ok(rx.tx_begin(center_hz)),
            None => Ok(0.0),
        }
    }

    fn tx_write(&mut self, samples: &[Complex32]) -> Result<()> {
        let Some(rx) = self.rx.as_mut() else { return Ok(()) };
        // Bent by the inverse of the amplifier's curve when PureSignal is
        // running. The loop keeps its own copy of what was *wanted*, which is
        // what the feedback is compared with.
        let samples = match self.aux.as_mut().and_then(|a| a.puresignal.as_mut()) {
            Some(ps) => {
                self.ps_scratch.clear();
                self.ps_scratch.extend_from_slice(samples);
                ps.predistort(&mut self.ps_scratch);
                &self.ps_scratch[..]
            }
            None => samples,
        };
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
        self.transmitting = false;
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
        // With the second chain borrowed the pair is thrown away together, or
        // the two rings would come out of step by whatever was queued.
        match (self.rx.as_mut(), self.aux.as_mut()) {
            (Some(rx), Some(aux)) => rx.discard_pending_paired(&mut aux.rx),
            (Some(rx), None) => rx.discard_pending_rx(),
            _ => {}
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
        let rig = self.rig.as_deref().and_then(PlutoRig::open_status);
        match (rig, self.aux_note.clone()) {
            (Some(a), Some(b)) => Some(format!("{a}; {b}")),
            (a, b) => a.or(b),
        }
    }

    /// What the predistortion loop on RX2 is doing, for the meter.
    fn puresignal(&mut self) -> Option<sdroxide_types::PsMeter> {
        let ps = self.aux.as_ref()?.puresignal.as_ref()?;
        Some(sdroxide_types::PsMeter {
            locked: ps.locked(),
            correction_db: ps.correction_db(),
            score: ps.score(),
            frozen: ps.frozen(),
        })
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
        // The borrowed chain first, so its ring and the lockstep go before the
        // stream they were paired with.
        self.aux = None;
        self.rx = None;
        let Some(rig) = self.rig.take() else { return };
        if !rig.is_alive() {
            rig.release();
        }
        // and the drop below closes a healthy one, if this was the last holder
    }
}

/// The configuration's mode, as the DSP crate spells it.
fn div_mode(mode: DiversityMode) -> sdroxide_dsp::DiversityMode {
    match mode {
        DiversityMode::Cancel => sdroxide_dsp::DiversityMode::Cancel,
        DiversityMode::Combine => sdroxide_dsp::DiversityMode::Combine,
    }
}
