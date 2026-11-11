//! An [`IqSource`] for a LimeSDR driven through LimeSuite by `sdroxide-lime`,
//! and the LimeRFE in front of it.
//!
//! # No thread
//!
//! Unlike the native USB backends this one calls the library straight from the
//! engine's thread. That is not an omission: LimeSuite already runs its own USB
//! workers behind its own FIFO, and `LMS_RecvStream` takes a timeout, so the
//! shape here is the SoapySDR source's — one `read_within` called with 200 ms
//! from [`IqSource::read`] and zero from [`IqSource::read_available`]. Stacking
//! a second ring on top would add latency and buy nothing.
//!
//! # Drive stays digital
//!
//! [`IqSource::set_tx_drive`] is left at its trait default and
//! `commands_tx_power` stays false, for the reason spelled out in
//! `hackrf_source.rs`: the engine already applies drive to the samples, so a
//! source that also mapped it onto the transmit gain would attenuate twice.
//! The gain is published as an ordinary transmit element instead.
//!
//! # Where the LimeRFE band-follow hooks in
//!
//! Two signals, and the receive one is not obvious. [`IqSource::set_tx_freq_hz`]
//! is the transmit side and exists for exactly this — its doc names
//! band-switching amplifiers as the case it was written for. The receive side
//! is **not** `set_center_hz` alone: this is a zero-IF radio, so the engine
//! parks the LO a quarter-span above the dial, and near a band edge that is the
//! difference between the right filter and the wrong one. The operator's actual
//! receive frequency is `centre + if_offset`, and `set_if_offset` is what
//! carries the second term.

use std::time::{Duration, Instant};

use sdroxide_dsp::{Diversity, IqCorrect, PureSignal};
use sdroxide_lime::LimeHandle;
use sdroxide_lime::handle::RX_TIMEOUT_MS;
use sdroxide_limerfe::LimeRfeHandle;
use sdroxide_radio::{Complex32, DC_BLOCK_HZ, IqSource, RadioError, Result, lo_offset_for};
use sdroxide_types::{DiversityMode, LimeAuxRole, LimeConfig, RfeLink, RfeModeControl};

/// How often the diversity filter's achieved null depth reaches the log.
///
/// It is the number that says whether any of this is working, and there is
/// nowhere else to put it: a backend has one chance to talk to the operator
/// (`open_status`, at open) and no channel for a live reading. So it goes to
/// the log, slowly enough not to fill it.
const DEPTH_LOG_INTERVAL: Duration = Duration::from_secs(10);

pub struct LimeSource {
    handle: LimeHandle,
    cfg: LimeConfig,
    center: f64,
    lo_offset: f64,
    /// The operator's receive dial relative to the centre, as the engine last
    /// reported it. Zero until the first `set_if_offset`, which is within a
    /// quarter span and corrected on the first tuning update.
    if_offset: f64,
    iq_correct: Option<IqCorrect>,
    /// The second chain's samples for the block being read, and its own
    /// correction: it is a separate zero-IF front end with a DC offset and an
    /// imbalance of its own, and handing those to the canceller would have it
    /// spend taps subtracting one radio's artefacts from the other's.
    aux_buf: Vec<Complex32>,
    aux_correct: Option<IqCorrect>,
    /// The adaptive combiner, present exactly when the second chain is doing
    /// diversity (issue #98).
    diversity: Option<Diversity>,
    /// The predistortion loop, present exactly when the second chain is on a
    /// transmit coupler and the transmitter is armed (issue #98).
    puresignal: Option<PureSignal>,
    /// The transmit block, predistorted. `IqSource::tx_write` hands over a
    /// shared slice, so bending it needs a copy.
    tx_scratch: Vec<Complex32>,
    /// Said once per over rather than per block: the feedback lands at the
    /// difference between the two synthesisers, and past the edge of the span
    /// there is nothing to hear.
    warned_offset: bool,
    last_depth_log: Instant,
    rfe: Option<LimeRfeHandle>,
    /// Set when the LimeRFE could not be opened. Reported through
    /// `open_status` rather than refusing the radio: a front end that failed to
    /// answer should not cost the operator their receiver.
    rfe_note: Option<String>,
    /// Where the chip's own DC/IQ calibration was last made, and when the next
    /// one falls due. See [`Self::recalibrate_if_due`].
    cal_center: f64,
    cal_due: Option<Instant>,
    label: String,
}

/// How far the synthesiser may move before the chip's DC-offset and
/// image-rejection calibration is no longer the one for where it is.
///
/// Proportional, with a floor: the LO leakage a zero-IF mixer demodulates
/// against itself is a property of the synthesiser's own settings, so it drifts
/// on a scale that goes with the frequency rather than a fixed number of
/// megahertz. One per cent puts the boundary at 1.5 MHz on 2 m and 4 MHz on
/// 70 cm, and the floor keeps HF — where one per cent would be a few tens of
/// kilohertz — from recalibrating on every band change within a band.
const RECAL_FRACTION: f64 = 0.01;
const RECAL_FLOOR_HZ: f64 = 1.0e6;

/// How long the dial has to sit still before the calibration is actually run.
///
/// `LMS_Calibrate` stalls whatever thread calls it for the better part of a
/// second, and the thread that calls it here is the one reading samples. So it
/// is never run *while* the operator is moving: dragging a panadapter across a
/// band pushes this deadline ahead of itself and nothing happens until they
/// stop.
const RECAL_SETTLE: Duration = Duration::from_millis(1500);

impl LimeSource {
    pub fn open(cfg: &LimeConfig, center_hz: f64) -> anyhow::Result<Self> {
        let handle = LimeHandle::open(cfg, center_hz)?;
        let rate = handle.sample_rate();
        let label = format!("{} @ {:.3} Msps", handle.label(), rate / 1e6);
        // Zero-IF part: park the LO off the VFO where the analog filter allows
        // it — see `sdroxide_radio::lo_offset_for`.
        let lo_offset = lo_offset_for(rate, handle.analog_bw());
        let iq_correct = cfg.iq_correction.then(|| IqCorrect::new(DC_BLOCK_HZ, rate));
        // Only where the chain actually came up: `LimeHandle::open` degrades a
        // second chain that would not start to a note, and a combiner with
        // nothing to combine would sit there reporting no null.
        let diversity =
            (cfg.aux.role == LimeAuxRole::Diversity && handle.aux_active()).then(|| {
                Diversity::new(div_mode(cfg.aux.mode), usize::from(cfg.aux.taps), cfg.aux.rate)
            });
        let aux_correct =
            (diversity.is_some() && cfg.iq_correction).then(|| IqCorrect::new(DC_BLOCK_HZ, rate));
        // Predistortion needs the coupler chain *and* a transmitter: a
        // correction loop on a radio that cannot key has nothing to correct.
        let puresignal =
            (cfg.aux.role == LimeAuxRole::PureSignal && handle.aux_active() && handle.can_tx())
                .then(|| PureSignal::new(usize::from(cfg.aux.ps_bins), cfg.aux.ps_rate, rate));

        // The LimeRFE. Its own USB cable is opened here; the path through the
        // board's GPIO needs the device handle and so is built in the crate
        // that owns it.
        let (rfe, rfe_note) = Self::open_rfe(cfg, &handle);
        if let Some(r) = &rfe {
            // Where we are, before its thread decides anything. It holds off
            // until a dial has been reported rather than configuring itself
            // from a zero that resolves to HF, and the engine's first tuning
            // update is tens of milliseconds away — long enough to be heard as
            // a front end that does nothing at all. The IF offset is not known
            // yet, so this is the centre; it is within a quarter of a span and
            // the first `set_if_offset` corrects it.
            r.set_rx_hz(center_hz);
            r.set_tx_hz(center_hz);
        }

        tracing::info!(
            "LimeSDR source ready: {label}, centre {center_hz:.0} Hz, LO offset \
             {lo_offset:.0} Hz (0 = LO on the VFO), receiving on {}{}",
            // Which socket, always — a front end feeding one the radio is not
            // listening on is silent rather than faulty, and the log is where
            // that gets diagnosed. Named by the connector rather than the
            // chip's port, because `LNAL` is the same word on both chains.
            handle.rx_socket_label(),
            match &rfe {
                Some(r) => format!(", {}", r.describe()),
                None => String::new(),
            }
        );
        if let Some(socket) = handle.aux_socket_label().filter(|_| puresignal.is_some()) {
            tracing::info!(
                "PureSignal is on: transmit feedback on {socket}, {} table steps — the \
                 correction stays at unity until the feedback aligns with what was sent",
                cfg.aux.ps_bins
            );
        }
        if let Some(socket) = handle.aux_socket_label().filter(|_| diversity.is_some()) {
            tracing::info!(
                "diversity is on: second aerial on {socket}, {} filter, {} taps{}",
                match cfg.aux.mode {
                    DiversityMode::Cancel => "cancelling",
                    DiversityMode::Combine => "combining",
                },
                cfg.aux.taps,
                if handle.aux_timestamped() {
                    ""
                } else {
                    " (this LimeSuite does not stamp receive blocks, so the two chains are \
                     paired by arrival order)"
                }
            );
        }

        Ok(LimeSource {
            handle,
            cfg: cfg.clone(),
            center: center_hz,
            lo_offset,
            if_offset: 0.0,
            iq_correct,
            aux_buf: Vec::new(),
            aux_correct,
            diversity,
            puresignal,
            tx_scratch: Vec::new(),
            warned_offset: false,
            last_depth_log: Instant::now(),
            rfe,
            rfe_note,
            cal_center: center_hz,
            cal_due: None,
            label,
        })
    }

    /// Open the front end, if one is configured.
    ///
    /// A LimeRFE that will not answer costs a note, not the radio: an operator
    /// who came to listen should not lose their receiver because an accessory
    /// is unplugged.
    fn open_rfe(cfg: &LimeConfig, handle: &LimeHandle) -> (Option<LimeRfeHandle>, Option<String>) {
        if cfg.rfe.link == RfeLink::Off {
            return (None, None);
        }
        match sdroxide_lime::open_rfe(&cfg.rfe, handle) {
            Ok(h) => (h, None),
            Err(e) => {
                tracing::warn!("LimeRFE not opened: {e}");
                (None, Some(format!("the LimeRFE was not opened: {e}")))
            }
        }
    }

    /// Whether the transmitter is armed and usable.
    pub fn tx_enabled(&self) -> bool {
        self.handle.can_tx()
    }

    /// The synthesiser's reach in one direction, as the board reports it.
    ///
    /// `None` where LimeSuite would not say, which the capability layer treats
    /// as "unknown" rather than "nowhere" — but on this hardware a missing
    /// range is what lets the engine make the call that wedges the device, so
    /// it is logged rather than passed over quietly.
    pub fn freq_range(&self, tx: bool) -> Option<(f64, f64)> {
        match self.handle.lo_range(tx) {
            Ok(r) => Some((r.min, r.max)),
            Err(e) => {
                tracing::warn!(
                    "LimeSDR did not report its {} tuning range ({e}) — the engine's retune \
                     guard has nothing to work from, so a frequency below the chip's range may \
                     stop the stream",
                    if tx { "transmit" } else { "receive" }
                );
                None
            }
        }
    }

    /// The sample-rate range the board accepts.
    pub fn rate_range(&self) -> Option<(f64, f64)> {
        self.handle.rate_range(false).ok().map(|r| (r.min, r.max))
    }

    /// The port names this board offers in one direction.
    pub fn antennas(&self, tx: bool) -> Vec<String> {
        if tx { self.handle.antennas_tx().to_vec() } else { self.handle.antennas_rx().to_vec() }
    }

    /// The operator's receive frequency, which is not the hardware centre on a
    /// zero-IF radio. See the module doc.
    fn rx_dial_hz(&self) -> f64 {
        self.center + self.if_offset
    }

    fn push_rfe_rx(&self) {
        if let Some(rfe) = &self.rfe {
            rfe.set_rx_hz(self.rx_dial_hz());
        }
    }

    /// Note that the front end has moved far enough for its calibration to be
    /// stale, and when to do something about it.
    ///
    /// A zero-IF receiver's DC offset and image rejection are corrected by
    /// numbers `LMS_Calibrate` measured *at one LO frequency*. Carry those to
    /// another band and the correction is not merely absent but wrong — which
    /// is what the LO leakage sitting in the middle of the span as a carrier
    /// the operator can see actually is (issue #94). SDRangel recalibrates on
    /// every retune; here it waits for the dial to settle first, because a
    /// panadapter drag is a stream of retunes and each calibration costs the
    /// receiver most of a second.
    ///
    /// Also called when the receive port changes: a different LNA input is a
    /// different path to the mixer, so the offsets it measured are somebody
    /// else's.
    fn calibration_may_be_stale(&mut self, now_center_hz: f64) {
        if !self.cfg.calibrate {
            return;
        }
        let moved = (now_center_hz - self.cal_center).abs();
        let far = (self.cal_center.abs() * RECAL_FRACTION).max(RECAL_FLOOR_HZ);
        if moved > far {
            self.cal_due = Some(Instant::now() + RECAL_SETTLE);
        } else {
            // Back where the calibration was made: whatever was pending is no
            // longer owed.
            self.cal_due = None;
        }
    }

    /// Run it, if it has fallen due and the radio is free to be stopped.
    ///
    /// Called from the read path, which is the one place on this backend that
    /// runs often and is allowed to block — and never during an over, where the
    /// stall would come out of the transmitter's budget and the calibration
    /// would be measuring our own carrier.
    fn recalibrate_if_due(&mut self) {
        let Some(due) = self.cal_due else { return };
        if Instant::now() < due || self.handle.tx_active() {
            return;
        }
        self.cal_due = None;
        let was = self.cal_center;
        let center = self.handle.center_hz();
        // Recorded either way: a calibration that will not converge here will
        // not converge on the next block either, and retrying it every read
        // would cost the receiver far more than the stale numbers do.
        self.cal_center = center;
        match self.handle.calibrate() {
            Ok(()) => tracing::info!(
                "LimeSDR recalibrated for {:.3} MHz — the DC offset and image rejection it \
                 was carrying were measured {:.3} MHz away",
                center / 1e6,
                (center - was).abs() / 1e6
            ),
            // Not fatal: the host DC blocker and the image estimator are still
            // running, so an uncalibrated radio is a poorer one, not a dead one.
            Err(e) => {
                tracing::warn!("LimeSDR would not recalibrate for the new band, continuing: {e}")
            }
        }
    }

    /// One block from the radio, corrected and — where a second aerial is
    /// running — combined with it.
    ///
    /// The order matters. Each chain's own DC offset and image are removed
    /// *before* the two meet, because they are artefacts of two separate
    /// zero-IF front ends and have nothing in common; leaving them in would
    /// have the adaptive filter spend its taps trying to explain one radio's
    /// defects with the other's.
    ///
    /// A block the second chain could not be aligned to comes through
    /// uncombined rather than combined against the wrong samples — see
    /// `LimeHandle::read_pair`.
    fn read_within(&mut self, buf: &mut [Complex32], timeout_ms: u32) -> Result<usize> {
        // Before the read rather than after it: the calibration stops the
        // converters, so the samples either side of it are not ones to hand on.
        self.recalibrate_if_due();
        // The coupler chain is not part of what is being listened to: it is
        // read separately, and only its own loop ever sees it.
        if self.puresignal.is_some() {
            self.pump_feedback(buf.len());
        }
        let want_aux = self.diversity.is_some() && self.handle.aux_active();
        if want_aux && self.aux_buf.len() < buf.len() {
            self.aux_buf.resize(buf.len(), Complex32::new(0.0, 0.0));
        }
        // Disjoint fields, so the second chain's landing buffer can be handed
        // to the handle while the handle borrows itself.
        let aux: &mut [Complex32] = if want_aux { &mut self.aux_buf[..buf.len()] } else { &mut [] };
        let (n, got) = self
            .handle
            .read_pair(buf, aux, timeout_ms)
            .map_err(|e| RadioError::Msg(e.to_string()))?;
        if n == 0 {
            return Ok(0);
        }
        if let Some(iq) = self.iq_correct.as_mut() {
            iq.process(&mut buf[..n]);
        }
        if got == n {
            if let Some(iq) = self.aux_correct.as_mut() {
                iq.process(&mut self.aux_buf[..got]);
            }
            if let Some(d) = self.diversity.as_mut() {
                d.process(&mut buf[..n], &self.aux_buf[..got]);
            }
        }
        if want_aux {
            self.log_depth();
        }
        if self.puresignal.is_some() && self.handle.tx_active() {
            self.log_puresignal();
        }
        Ok(n)
    }

    /// Take what the transmit coupler heard and give it to the predistortion
    /// loop.
    ///
    /// Called every time the engine asks for samples, transmitting or not, and
    /// the "or not" half matters: a stream nobody reads overruns, and its
    /// FIFO would be full of a previous over by the time the next one started.
    /// Off the air the block is simply thrown away.
    ///
    /// The feedback does not arrive at the middle of the receiver's span. The
    /// two synthesisers are commanded separately — the transmit one sits on
    /// the carrier, the receive one a quarter of a span off the dial — so the
    /// coupled signal lands at the difference, which is known exactly and
    /// spun back out inside the loop.
    fn pump_feedback(&mut self, block: usize) {
        let want = block.max(4096);
        if self.aux_buf.len() < want {
            self.aux_buf.resize(want, Complex32::new(0.0, 0.0));
        }
        let n = self.handle.read_aux_raw(&mut self.aux_buf[..want]);
        if n == 0 {
            return;
        }
        if !self.handle.tx_active() {
            return;
        }
        let rate = self.handle.sample_rate();
        let offset = self.handle.tx_center_hz() - self.center;
        if offset.abs() > rate * 0.45 {
            if !self.warned_offset {
                self.warned_offset = true;
                tracing::warn!(
                    "the transmit frequency is {:.3} MHz from the receiver's centre, which is \
                     outside the captured span — the coupler's signal cannot be seen, so \
                     PureSignal will not correct this over",
                    offset / 1e6
                );
            }
            return;
        }
        let Some(ps) = self.puresignal.as_mut() else { return };
        ps.feed_back(&self.aux_buf[..n], offset, rate);
    }

    /// Say how the filter is doing, occasionally.
    ///
    /// The null depth is the one number that separates "the second aerial
    /// hears the noise" from "the second aerial hears nothing the first one
    /// does", and no amount of adjusting the filter fixes the second case.
    fn log_depth(&mut self) {
        if self.last_depth_log.elapsed() < DEPTH_LOG_INTERVAL {
            return;
        }
        self.last_depth_log = Instant::now();
        let Some(d) = self.diversity.as_ref() else { return };
        let slips = self.handle.aux_slips();
        match d.depth_db() {
            Some(db) => tracing::info!(
                "diversity: {db:.1} dB of the main aerial's signal is being cancelled{}{}",
                if d.frozen() { ", filter held" } else { "" },
                if slips > 0 { format!(", {slips} pairing restart(s))") } else { String::new() }
            ),
            None if slips > 0 => {
                tracing::debug!("diversity: combining, {slips} pairing restart(s)");
            }
            None => {}
        }
        if self.handle.aux_stalled() {
            tracing::warn!(
                "the second receive chain is not keeping up, so blocks are going through \
                 uncombined — try a lower sample rate"
            );
        }
    }

    /// The same for the predistortion loop, whose one number is whether it has
    /// found the feedback at all.
    fn log_puresignal(&mut self) {
        if self.last_depth_log.elapsed() < DEPTH_LOG_INTERVAL {
            return;
        }
        self.last_depth_log = Instant::now();
        let Some(ps) = self.puresignal.as_ref() else { return };
        if ps.locked() {
            tracing::info!(
                "PureSignal is correcting {:.1} dB of compression (feedback matched at {:.2}){}",
                ps.correction_db(),
                ps.score(),
                if ps.frozen() { ", table held" } else { "" }
            );
        } else {
            tracing::info!(
                "PureSignal has not found the transmission in the coupler's chain (best match \
                 {:.2}) — the transmitter is uncorrected. Check the coupler, and that the \
                 second chain's gain is low enough not to be driven into compression by it",
                ps.score()
            );
        }
    }
}

/// The configuration's mode, as the DSP crate spells it.
fn div_mode(mode: DiversityMode) -> sdroxide_dsp::DiversityMode {
    match mode {
        DiversityMode::Cancel => sdroxide_dsp::DiversityMode::Cancel,
        DiversityMode::Combine => sdroxide_dsp::DiversityMode::Combine,
    }
}

impl IqSource for LimeSource {
    fn sample_rate(&self) -> f64 {
        self.handle.sample_rate()
    }

    /// What the predistortion loop on the second chain is doing, for the
    /// operator rather than for the log (issue #441).
    fn puresignal(&mut self) -> Option<sdroxide_types::PsMeter> {
        let ps = self.puresignal.as_ref()?;
        Some(sdroxide_types::PsMeter {
            locked: ps.locked(),
            correction_db: ps.correction_db(),
            score: ps.score(),
            frozen: ps.frozen(),
        })
    }

    fn center_hz(&self) -> f64 {
        self.center
    }

    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.handle.set_center_hz(hz).map_err(|e| RadioError::Msg(e.to_string()))?;
        self.center = hz;
        self.push_rfe_rx();
        self.calibration_may_be_stale(hz);
        Ok(())
    }

    fn lo_offset_hz(&self) -> f64 {
        self.lo_offset
    }

    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        self.read_within(buf, RX_TIMEOUT_MS)
    }

    /// Take only what is already waiting.
    ///
    /// This radio is full duplex, so the engine calls this every tick during an
    /// over, sharing the thread with the transmit feed — waiting here for
    /// samples that have not arrived would come out of the transmitter's
    /// budget.
    fn read_available(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        self.read_within(buf, 0)
    }

    /// A centre of its own — the RF PLL — so the engine may centre the I/Q on a
    /// resting dial (`Engine::recentre_after_dwell`).
    fn recentre_on_dwell(&self) -> bool {
        true
    }

    fn describe(&self) -> String {
        self.label.clone()
    }

    fn set_gain_element(&mut self, name: &str, db: f64) -> Result<()> {
        match name {
            LimeConfig::RX_GAIN_ELEMENT => {
                self.handle.set_gain_db(false, db).map_err(|e| RadioError::Msg(e.to_string()))?;
            }
            LimeConfig::LPF_RX_ELEMENT => {
                self.handle.set_lpf_bw(false, db).map_err(|e| RadioError::Msg(e.to_string()))?;
                // The filter and the LO offset are coupled: a narrower filter
                // than the offset needs withdraws the offset entirely.
                self.lo_offset = lo_offset_for(self.handle.sample_rate(), self.handle.analog_bw());
            }
            LimeConfig::LPF_TX_ELEMENT => {
                self.handle.set_lpf_bw(true, db).map_err(|e| RadioError::Msg(e.to_string()))?;
            }
            LimeConfig::CALIBRATE_ELEMENT if db >= 0.5 => {
                // Momentary, and slow — the better part of a second. The
                // automatic one waits for the dial to settle (see
                // `recalibrate_if_due`); this is the operator saying "now".
                self.cal_due = None;
                self.cal_center = self.handle.center_hz();
                self.handle.calibrate().map_err(|e| RadioError::Msg(e.to_string()))?;
            }
            LimeConfig::IQ_CORRECTION_ELEMENT => {
                let rate = self.handle.sample_rate();
                if db >= 0.5 {
                    match self.iq_correct.as_mut() {
                        Some(iq) => iq.reset(),
                        None => self.iq_correct = Some(IqCorrect::new(DC_BLOCK_HZ, rate)),
                    }
                    // Both chains or neither: correcting one of a pair would
                    // manufacture exactly the difference the canceller is
                    // trying to remove.
                    if self.diversity.is_some() {
                        match self.aux_correct.as_mut() {
                            Some(iq) => iq.reset(),
                            None => self.aux_correct = Some(IqCorrect::new(DC_BLOCK_HZ, rate)),
                        }
                    }
                } else {
                    self.iq_correct = None;
                    self.aux_correct = None;
                }
            }
            // The second chain and its filter. Pseudo-elements for the same
            // reason the LimeRFE's are: settings only this backend has, and no
            // new `Command` variant for them.
            LimeConfig::AUX_GAIN_ELEMENT => {
                self.handle.set_aux_gain_db(db).map_err(|e| RadioError::Msg(e.to_string()))?;
                self.cfg.aux.gain_db = db;
            }
            LimeConfig::DIV_MODE_ELEMENT => {
                let mode = if db >= 0.5 { DiversityMode::Combine } else { DiversityMode::Cancel };
                self.cfg.aux.mode = mode;
                if let Some(d) = self.diversity.as_mut() {
                    d.set_mode(div_mode(mode));
                }
            }
            LimeConfig::DIV_RATE_ELEMENT => {
                self.cfg.aux.rate = db as f32;
                if let Some(d) = self.diversity.as_mut() {
                    d.set_rate(db as f32);
                }
            }
            LimeConfig::DIV_TAPS_ELEMENT => {
                let taps =
                    db.round().clamp(1.0, f64::from(sdroxide_types::LimeAuxConfig::MAX_TAPS)) as u8;
                self.cfg.aux.taps = taps;
                if let Some(d) = self.diversity.as_mut() {
                    // Necessarily starts the filter again: the taps mean
                    // different delays now.
                    d.set_taps(usize::from(taps));
                }
            }
            LimeConfig::DIV_FREEZE_ELEMENT => {
                self.cfg.aux.frozen = db >= 0.5;
                if let Some(d) = self.diversity.as_mut() {
                    d.set_frozen(db >= 0.5);
                }
            }
            LimeConfig::DIV_RESET_ELEMENT if db >= 0.5 => {
                if let Some(d) = self.diversity.as_mut() {
                    d.reset();
                }
            }
            // The predistortion loop. `PS_BINS_ELEMENT` rebuilds the table, so
            // it necessarily starts the correction again.
            LimeConfig::PS_RATE_ELEMENT => {
                self.cfg.aux.ps_rate = db as f32;
                if let Some(ps) = self.puresignal.as_mut() {
                    ps.set_rate(db as f32);
                }
            }
            LimeConfig::PS_FREEZE_ELEMENT => {
                self.cfg.aux.ps_frozen = db >= 0.5;
                if let Some(ps) = self.puresignal.as_mut() {
                    ps.set_frozen(db >= 0.5);
                }
            }
            LimeConfig::PS_RESET_ELEMENT if db >= 0.5 => {
                if let Some(ps) = self.puresignal.as_mut() {
                    ps.reset();
                }
            }
            LimeConfig::PS_BINS_ELEMENT => {
                let bins = db.round().clamp(
                    f64::from(sdroxide_types::LimeAuxConfig::PS_MIN_BINS),
                    f64::from(sdroxide_types::LimeAuxConfig::PS_MAX_BINS),
                ) as u8;
                self.cfg.aux.ps_bins = bins;
                if self.puresignal.is_some() {
                    self.puresignal = Some(PureSignal::new(
                        usize::from(bins),
                        self.cfg.aux.ps_rate,
                        self.handle.sample_rate(),
                    ));
                    if let Some(ps) = self.puresignal.as_mut() {
                        ps.set_frozen(self.cfg.aux.ps_frozen);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// What the chip really has, not what it was asked for: LimeSuite takes an
    /// integer number of decibels, so a slider left between two of them is a
    /// radio at the lower one.
    fn current_gains(&self) -> Vec<(String, f64)> {
        vec![(LimeConfig::RX_GAIN_ELEMENT.to_string(), self.handle.rx_gain_db())]
    }

    fn set_antenna(&mut self, name: &str) -> Result<()> {
        self.handle.set_antenna(false, name).map_err(|e| RadioError::Msg(e.to_string()))?;
        // A different LNA input is a different path into the mixer, so the
        // offsets the chip is carrying were measured on somebody else's.
        // SDRangel recalibrates on an antenna change for the same reason.
        if self.cfg.calibrate {
            self.cal_due = Some(Instant::now() + RECAL_SETTLE);
        }
        // Kept in step with the handle's own copy, which pins the choice so the
        // automatic one stops overriding it. Here it is what stops
        // [`Self::owns_rx_antenna`] going on claiming a socket the operator has
        // since named for themselves.
        self.cfg.antenna_rx = name.to_string();
        Ok(())
    }

    fn current_antenna(&self) -> String {
        self.handle.antenna_rx().to_string()
    }

    /// The settings that are not numbers: the second chain's socket, and the
    /// LimeRFE's whole configuration.
    ///
    /// The front end comes through here as one value rather than a control at a
    /// time because its fields only mean anything together — see
    /// [`LimeConfig::RFE_SETTING`]. Before this, the connectors, the band and
    /// the relay mode had no door at all: changing them in the settings panel
    /// wrote the file and left the board doing what it had been doing until the
    /// next restart (issue #94).
    fn set_device_setting(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            LimeConfig::AUX_ANTENNA_SETTING if !value.trim().is_empty() => {
                self.handle.set_aux_antenna(value).map_err(|e| RadioError::Msg(e.to_string()))?;
                self.cfg.aux.antenna = value.to_string();
            }
            LimeConfig::RFE_SETTING => {
                let Some(want) = sdroxide_types::LimeRfeConfig::from_setting(value) else {
                    return Err(RadioError::Msg("that is not a LimeRFE configuration".to_string()));
                };
                // Which cable the board is on is settled when it is opened, so
                // those two fields stay as they are however they arrive: the
                // settings panel asks for a rebuild when they change.
                let cfg = sdroxide_types::LimeRfeConfig {
                    link: self.cfg.rfe.link,
                    serial: self.cfg.rfe.serial.clone(),
                    ..want
                };
                if cfg == self.cfg.rfe {
                    return Ok(());
                }
                let fan_moved = cfg.fan != self.cfg.rfe.fan;
                self.cfg.rfe = cfg;
                if let Some(r) = &self.rfe {
                    r.set_config(self.cfg.rfe.clone());
                    // The fan is not part of the board's configured state — it
                    // is its own one-byte command — so it does not ride the
                    // state the follower sends.
                    if fan_moved {
                        r.set_fan(self.cfg.rfe.fan);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// With a LimeRFE in front, the socket is the cabling's answer rather than
    /// the session's — see [`IqSource::owns_rx_antenna`]. Not claimed once the
    /// operator has named one: then the interface's own configuration holds it
    /// and there is nothing for a remembered port to disagree with anyway.
    fn owns_rx_antenna(&self) -> bool {
        self.cfg.rfe.link != RfeLink::Off && self.cfg.antenna_rx.trim().is_empty()
    }

    // ---- transmit --------------------------------------------------------

    fn tx_begin(&mut self, center_hz: f64, _rate: f64) -> Result<f64> {
        // The interlock. A LimeRFE pinned to receive will not open its transmit
        // relay, so the drive would go into the receive path with the amplifier
        // bypassed. Refusing here is the same discipline as an unarmed radio
        // publishing no transmit channel: the point is that no path can key it,
        // not that most paths remember to check.
        if let Some(reason) = self.cfg.rfe.tx_refusal() {
            return Err(RadioError::Msg(reason));
        }
        // Tell the front end before any RF appears. The board thread is already
        // holding the band; this is the relay.
        if let Some(rfe) = &self.rfe {
            rfe.set_tx_hz(center_hz);
            rfe.set_keyed(true);
            // The relay has to have thrown before drive arrives, and how long
            // that takes is the transport's answer rather than a guess: the
            // board's own serial port is tens of milliseconds and the
            // bit-banged I²C path through the LimeSDR is the better part of a
            // second. Not conditional on the cabling any more either — a
            // receiving board is left in receive on both, so both switch at
            // key-down (issue #94).
            if self.cfg.rfe.switches_at_key_down() {
                std::thread::sleep(rfe.relay_settle());
            }
        }
        // A new over: the amplifier's curve has not changed since the last one,
        // but where the coupler's samples sit relative to what was written to
        // the transmit FIFO has — so the table is kept and the alignment is
        // found again.
        if let Some(ps) = self.puresignal.as_mut() {
            ps.unlock();
        }
        self.warned_offset = false;
        let rate = self.handle.tx_begin(center_hz).map_err(|e| {
            // Do not leave the front end keyed if the radio refused.
            if let Some(rfe) = &self.rfe {
                rfe.set_keyed(false);
            }
            RadioError::Msg(e.to_string())
        })?;
        Ok(rate)
    }

    /// One block of modulated baseband, bent on the way out if the amplifier
    /// is being linearised.
    ///
    /// The predistorter needs to *own* the samples — it multiplies each by a
    /// gain read from its table — and the engine hands over a shared slice, so
    /// this copies. It is a copy per block on the transmit path and buys the
    /// twenty-odd decibels of intermodulation that predistortion is for; and
    /// with no correction learned the table is unity, so the copy is the only
    /// cost until the loop has found the feedback.
    fn tx_write(&mut self, samples: &[Complex32]) -> Result<()> {
        if let Some(ps) = self.puresignal.as_mut() {
            self.tx_scratch.clear();
            self.tx_scratch.extend_from_slice(samples);
            ps.predistort(&mut self.tx_scratch);
            return self
                .handle
                .tx_write(&self.tx_scratch)
                .map_err(|e| RadioError::Msg(e.to_string()));
        }
        self.handle.tx_write(samples).map_err(|e| RadioError::Msg(e.to_string()))
    }

    fn tx_drain(&mut self) {
        self.handle.tx_drain();
    }

    fn tx_end(&mut self) -> Result<()> {
        let stopped = self.handle.tx_end();
        // Unkey the front end whatever the radio did — an amplifier left keyed
        // because the stop failed is the worse of the two failures.
        if let Some(rfe) = &self.rfe {
            rfe.set_keyed(false);
        }
        stopped.map_err(|e| RadioError::Msg(e.to_string()))
    }

    fn set_tx_gain_element(&mut self, name: &str, db: f64) -> Result<()> {
        if name == LimeConfig::TX_GAIN_ELEMENT {
            self.handle.set_gain_db(true, db).map_err(|e| RadioError::Msg(e.to_string()))?;
        }
        Ok(())
    }

    fn current_tx_gains(&self) -> Vec<(String, f64)> {
        vec![(LimeConfig::TX_GAIN_ELEMENT.to_string(), self.handle.tx_gain_db())]
    }

    fn set_tx_antenna(&mut self, name: &str) -> Result<()> {
        self.handle.set_antenna(true, name).map_err(|e| RadioError::Msg(e.to_string()))?;
        self.cfg.antenna_tx = name.to_string();
        Ok(())
    }

    fn current_tx_antenna(&self) -> String {
        self.handle.antenna_tx().to_string()
    }

    /// The transmit frequency, told to us *while receiving* so the LimeRFE can
    /// be on the right band before any RF appears. This is what the trait
    /// method exists for.
    fn set_tx_freq_hz(&mut self, hz: f64) {
        if let Some(rfe) = &self.rfe {
            rfe.set_tx_hz(hz);
        }
    }

    /// How far the operator's receive dial sits from the hardware centre.
    ///
    /// Needed here for the LimeRFE rather than for the radio: on a zero-IF part
    /// the centre is parked a quarter-span off the dial, and choosing a band
    /// filter from the centre would pick the wrong one either side of a band
    /// edge.
    fn set_if_offset(&mut self, hz: f64) {
        self.if_offset = hz;
        self.push_rfe_rx();
    }

    fn needs_reopen(&self) -> bool {
        self.handle.needs_reopen()
    }

    /// Hand the hardware back before the engine opens its replacement.
    ///
    /// Two halves, in this order. The LimeRFE first: dropping the handle joins
    /// its thread, and where the link is the board's GPIO that thread bit-bangs
    /// I²C through the very device the second half closes — and where it is the
    /// RFE's own serial port, this is what frees the port for the replacement.
    /// Then the board itself: `LimeHandle::close` stops the streams and closes
    /// the device, because a board still held here fails the replacement's
    /// open on Linux (libusb refuses the second claim, as "in use" — by us)
    /// and *shares* it on Windows, where CyAPI opens the device non-exclusive
    /// and the replacement's `LMS_Init` and stream setup land on top of the
    /// running stream — both sessions dead until a program restart (changing
    /// the sample rate froze the waterfall exactly this way, issue #118).
    fn release(&mut self) {
        self.rfe = None;
        self.handle.close();
    }

    fn open_status(&self) -> Option<String> {
        let mut notes = Vec::new();
        if let Some(n) = self.handle.status_note() {
            notes.push(n);
        }
        if let Some(n) = &self.rfe_note {
            notes.push(n.clone());
        }
        if let Some(rfe) = &self.rfe
            && let Some(n) = rfe.status()
        {
            notes.push(n);
        }
        // The cabling, stated once where it can be acted on: what an over
        // costs on a shared connector, or which connector it leaves by on a
        // split one.
        if self.rfe.is_some() {
            if let Some(n) = self.cfg.rfe.switching_note() {
                notes.push(n);
            }
            if self.handle.can_tx()
                && let Some(n) = self.cfg.rfe.connector_note()
            {
                notes.push(n);
            }
        }
        // What the second chain is doing, when it is meant to be doing
        // something: an operator who turned diversity on and hears no
        // difference needs to know which half of that is true.
        if self.cfg.aux.role == LimeAuxRole::PureSignal {
            match self.handle.aux_socket_label() {
                Some(socket) if self.handle.can_tx() => notes.push(format!(
                    "PureSignal is running on {socket} — the transmitter stays uncorrected \
                     until the coupler's samples line up with what was sent, which the log \
                     reports every few seconds. Set that chain's gain low: the coupled signal \
                     is strong, and a feedback chain in compression teaches the amplifier's \
                     curve wrongly"
                )),
                Some(_) => notes.push(
                    "PureSignal is configured but the transmitter is not armed, so there is \
                     nothing for it to correct"
                        .to_string(),
                ),
                None => notes.push(
                    "the second receive chain is not running, so there is no PureSignal \
                     feedback"
                        .to_string(),
                ),
            }
        } else if self.cfg.aux.role != LimeAuxRole::Off {
            match self.handle.aux_socket_label() {
                Some(socket) => {
                    let mut line = format!(
                        "diversity is running on {socket} — watch the log for the depth it is \
                         achieving, and set the second chain's gain so both aerials show about \
                         the same noise floor"
                    );
                    if !self.handle.aux_timestamped() {
                        line.push_str(
                            ". This LimeSuite does not stamp its receive blocks, so the two \
                             chains are paired by arrival order",
                        );
                    }
                    notes.push(line);
                }
                None => notes.push(
                    "the second receive chain is not running, so there is no diversity".to_string(),
                ),
            }
        }
        if self.cfg.rfe.link != RfeLink::Off && self.cfg.rfe.mode == RfeModeControl::Rx {
            notes.push(
                "the LimeRFE is pinned to receive, so this radio cannot transmit through it"
                    .to_string(),
            );
        }
        if notes.is_empty() { None } else { Some(notes.join("\n")) }
    }
}
