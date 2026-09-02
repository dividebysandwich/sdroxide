//! An [`IqSource`] for an SDRplay RSP driven through the vendor's API service
//! by the native driver in `sdroxide-sdrplay` — no SoapySDR in the path.
//!
//! Receive only: the trait's transmit methods already default to errors, which
//! is the correct answer for this hardware.
//!
//! On an RSPduo with both tuners running, this is also where the two aerials
//! meet: the driver hands back a sample-aligned pair and the adaptive filter
//! in `sdroxide_dsp` turns it into one stream — a noise source nulled, or two
//! fading paths combined (issue #153).
//!
//! Or they do not meet at all. The same two tuners can instead be two radios
//! on two frequencies — HF in one tab, VHF in another — and then each tab has
//! a source of its own on one tuner of one shared session (issue #165). The
//! [`crate::device_registry`] is what pairs the second source with the first
//! one's device, exactly as it does for a TCI rig's second receiver.

use std::time::{Duration, Instant};

use sdroxide_dsp::Diversity;
use sdroxide_radio::{Complex32, IqSource, Result, lo_offset_for};
use sdroxide_sdrplay::{DuoMode, SdrPlayDevice, SdrPlayHandle};
use sdroxide_types::{
    Direction, DiversityMode, GainElement, SdrPlayAgc, SdrPlayConfig, SdrPlayDuo, SdrPlayDuoTuner,
    SdrPlayHdrBw, SdrPlayModel,
};

use crate::device_registry::{DeviceKey, SharedDevice, registry};

impl SharedDevice for SdrPlayDevice {
    fn is_alive(&self) -> bool {
        SdrPlayDevice::is_alive(self)
    }
}

/// How long the receiver may deliver nothing before the connection counts as
/// dead. The service delivers continuously while healthy, so three seconds of
/// silence means the session is gone, same as the other local backends.
const SILENCE_BEFORE_REOPEN: Duration = Duration::from_secs(3);

/// How often the diversity filter's achieved null depth reaches the log.
const DEPTH_LOG_INTERVAL: Duration = Duration::from_secs(10);

pub struct SdrPlaySource {
    handle: SdrPlayHandle,
    center: f64,
    rx_scratch: Vec<f32>,
    label: String,
    lo_offset: f64,
    /// Last values the operator asked for, reported back until the hardware
    /// says otherwise (the GainChange event keeps `IF` honest under AGC).
    if_gr_db: i32,
    agc: SdrPlayAgc,
    bias_tee: bool,
    /// The RSPdx's high-dynamic-range path, tracked because it selects a
    /// different LNA table below 2 MHz — see [`Self::rx_gain_elements`].
    hdr: bool,
    /// The HDR path's analog filter, tracked so the open-status note can say
    /// which centres it is actually implemented at.
    hdr_bw: SdrPlayHdrBw,
    /// Whether the IF gain reduction may go below the API's 20 dB floor.
    extended_if_gr: bool,
    antenna: String,
    /// The ports this model (and, on the RSPduo, this tuner) offers — what
    /// the capabilities advertise. Empty hides the selector.
    antennas: Vec<String>,
    /// The second tuner's settings as configured, so the panel's live controls
    /// have somewhere to land.
    div_cfg: SdrPlayDuo,
    /// `Some` when both tuners are running: the filter that combines them.
    diversity: Option<Diversity>,
    /// The second tuner's samples, as the filter wants them.
    aux_scratch: Vec<f32>,
    aux_buf: Vec<Complex32>,
    last_depth_log: Instant,
}

impl SdrPlaySource {
    pub fn open(cfg: &SdrPlayConfig, center_hz: f64) -> anyhow::Result<Self> {
        let dev = registry()
            .get_or_open(DeviceKey::SdrPlay(cfg.serial.trim().to_string()), || {
                sdroxide_sdrplay::open(cfg, center_hz).map_err(|e| e.to_string())
            })
            .map_err(anyhow::Error::msg)?;
        // Which tuner this radio listens on — except on a session running
        // only one, where the setting means nothing: every RSP but the duo has
        // a single tuner, and an RSPduo the operator did not ask to run both
        // opened on one of them. Attaching to the one that exists beats
        // refusing to open over a setting that does not apply.
        let tuner = match dev.duo_mode() {
            DuoMode::Single => dev.main_tuner(),
            _ => match cfg.duo_tuner {
                SdrPlayDuoTuner::Tuner1 => 0,
                SdrPlayDuoTuner::Tuner2 => 1,
            },
        };
        let handle = dev.attach(tuner, center_hz)?;
        let rate = handle.out_rate_hz();
        let dual = handle.dual_tuner();
        let label = handle.label();
        // Zero-IF part: park the LO off the VFO where the analog filter
        // allows it — see `sdroxide_radio::lo_offset_for`. A low IF has no DC
        // spike in the middle of the span to dodge, so it wants none: the
        // offset would only push the wanted signal towards the skirt of an
        // analog filter that is already the narrowest part of this mode.
        let lo_offset =
            if handle.low_if_khz() != 0 { 0.0 } else { lo_offset_for(rate, handle.analog_bw_hz()) };
        let antennas: Vec<String> =
            handle.model().antennas(cfg.duo_tuner).iter().map(|s| s.to_string()).collect();
        let antenna = if cfg.antenna.is_empty() {
            antennas.first().cloned().unwrap_or_default()
        } else {
            cfg.antenna.clone()
        };
        let diversity = dual.then(|| {
            Diversity::new(div_mode(cfg.duo.mode), usize::from(cfg.duo.taps), cfg.duo.rate)
        });
        // The *effective* offset, not the computed one: HDR withdraws it (see
        // `lo_offset_hz`), and a log line reporting the offset that would have
        // applied is a log line that disagrees with the hardware.
        let effective_offset = if cfg.hdr && handle.model().has_hdr() { 0.0 } else { lo_offset };
        tracing::info!(
            "SDRplay source ready: {label}, center {center_hz:.0} Hz, \
             LO offset {effective_offset:.0} Hz (0 = LO on the VFO){}",
            if effective_offset == 0.0 && lo_offset != 0.0 { " — withdrawn for HDR" } else { "" }
        );
        if diversity.is_some() {
            tracing::info!(
                "diversity is on: {} filter, {} taps{}",
                match cfg.duo.mode {
                    DiversityMode::Cancel => "cancelling",
                    DiversityMode::Combine => "combining",
                },
                cfg.duo.taps,
                if handle.pair_stamped() {
                    ""
                } else {
                    " (the service is not numbering its blocks, so the two tuners are being \
                     paired by arrival order)"
                }
            );
        }
        let src = SdrPlaySource {
            center: center_hz,
            rx_scratch: Vec::new(),
            label,
            lo_offset,
            if_gr_db: cfg.if_gr_db,
            agc: cfg.agc,
            bias_tee: cfg.bias_tee,
            hdr: cfg.hdr,
            hdr_bw: cfg.hdr_bw,
            extended_if_gr: cfg.extended_if_gr,
            antenna,
            antennas,
            div_cfg: cfg.duo.clone(),
            diversity,
            aux_scratch: Vec::new(),
            aux_buf: Vec::new(),
            last_depth_log: Instant::now(),
            handle,
        };
        // A radio that did not open the board still has to have its own way
        // with its own tuner.
        src.push_settings(cfg);
        Ok(src)
    }

    pub fn model(&self) -> SdrPlayModel {
        self.handle.model()
    }

    pub fn antennas(&self) -> &[String] {
        &self.antennas
    }

    /// Whether both tuners are running and being combined into this stream.
    pub fn dual_tuner(&self) -> bool {
        self.handle.dual_tuner()
    }

    /// Whether the board is running both tuners as separate radios, this
    /// source being one of them.
    pub fn split_tuner(&self) -> bool {
        self.handle.split_tuner()
    }

    /// The two real gain stages, as they stand on the band this source is
    /// tuned to.
    ///
    /// Both are carried *negated* — `IF` is −(gain reduction dB), `LNA` is
    /// −(state) — so that on a slider more is louder, like every other
    /// backend. The LNA is listed first because the first element is the main
    /// window's Gain slider, and the LNA is the only gain the operator always
    /// owns: with the hardware AGC on (the default) the service holds the IF
    /// gain, and a slider the AGC snaps back is worse than none. It is also
    /// the control that actually clears an overloaded front end, which the IF
    /// gain never can.
    ///
    /// The LNA's range follows the *band*, not the model: an RSPdx has 28
    /// states at 250–420 MHz and 19 below 12 MHz, and offering the longer
    /// ladder on HF gives the operator a third of a slider that moves nothing
    /// and snaps back as the driver clamps. It is also a `Step` element rather
    /// than a dB one, because a state index is not a measurement: state 7 is
    /// 24 dB of reduction here and 25 dB at 420 MHz, and a control that
    /// appended "dB" to the 7 would be wrong by both the number and the unit.
    pub fn rx_gain_elements(&self) -> Vec<GainElement> {
        let hiz = self.model().antenna_is_hiz(&self.antenna);
        let max_lna = self.model().max_lna_state_on(self.center, hiz, self.hdr);
        vec![
            GainElement::steps(
                SdrPlayConfig::LNA_ELEMENT,
                Direction::Rx,
                -(f64::from(max_lna)),
                0.0,
                1.0,
            ),
            GainElement::db(
                SdrPlayConfig::IF_GAIN_ELEMENT,
                Direction::Rx,
                -f64::from(SdrPlayConfig::IF_GR_MAX),
                -f64::from(SdrPlayConfig::IF_GR_MIN),
                1.0,
            ),
        ]
    }

    /// Push this radio's own settings at its tuner.
    ///
    /// The session was configured from whichever radio opened the board, so a
    /// second radio arriving on the other tuner would otherwise inherit the
    /// first one's gains and filters. Sent once, on open; everything after
    /// that arrives as ordinary control.
    fn push_settings(&self, cfg: &SdrPlayConfig) {
        self.handle.set_agc(cfg.agc.code() as i32);
        self.handle.set_agc_setpoint(cfg.agc_setpoint_dbfs);
        self.handle.set_if_gr_db(cfg.if_gr_db);
        self.handle.set_lna_state(cfg.lna_state);
        self.handle.set_bias_tee(cfg.bias_tee);
        self.handle.set_rf_notch(cfg.rf_notch);
        self.handle.set_dab_notch(cfg.dab_notch);
        if !self.antenna.is_empty() {
            self.handle.set_antenna(&self.antenna);
        }
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
        let slips = self.handle.pair_slips();
        match d.depth_db() {
            Some(db) => tracing::info!(
                "diversity: {db:.1} dB of the main aerial's signal is being cancelled{}{}",
                if d.frozen() { ", filter held" } else { "" },
                if slips > 0 { format!(", {slips} pairing restart(s)") } else { String::new() }
            ),
            None if slips > 0 => {
                tracing::debug!("diversity: combining, {slips} pairing restart(s)");
            }
            None => {}
        }
        if self.handle.aux_stalled() {
            tracing::warn!(
                "the RSPduo's second tuner is not delivering, so blocks are going through \
                 uncombined — the receiver is still working, but diversity is not"
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

impl IqSource for SdrPlaySource {
    /// The engine is transmitting and has stopped reading, or has started
    /// again. Passed straight through to the stream thread, which keeps
    /// receiving either way — this only decides whether a full ring is
    /// reported as an overrun or as the ordinary cost of an over. See
    /// [`IqSource::set_rx_paused`].
    fn set_rx_paused(&mut self, paused: bool) {
        self.handle.set_rx_paused(paused);
    }

    fn sample_rate(&self) -> f64 {
        self.handle.out_rate_hz()
    }

    fn center_hz(&self) -> f64 {
        self.center
    }

    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center = hz;
        self.handle.set_center_hz(hz);
        Ok(())
    }

    /// Zero, on the HDR path.
    ///
    /// The offset exists to park the LO clear of the zero-IF DC spike, and the
    /// engine tunes the hardware to `VFO + this`. HDR cannot pay that: its
    /// analog filter is built at a fixed handful of centres, so an offset puts
    /// the hardware beside every one of them — and for the highest, 1.9 MHz,
    /// beside *and above* the 2 MHz ceiling the path has at all. Left in, HDR
    /// is a switch that can never do anything.
    ///
    /// The same withdrawal the low-IF path gets a few lines up, for the
    /// neighbouring reason: where the tuned frequency is not ours to choose,
    /// the offset is not ours to add.
    fn lo_offset_hz(&self) -> f64 {
        if self.hdr && self.model().has_hdr() { 0.0 } else { self.lo_offset }
    }

    /// One block from the receiver — and, with both tuners running, the second
    /// aerial combined with the first.
    ///
    /// The pair the driver hands back is sample-aligned by construction, so
    /// there is nothing to check here; what there is to respect is the second
    /// tuner having stopped, in which case the block goes through uncombined
    /// rather than combined against silence.
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        let need = buf.len() * 2;
        if self.rx_scratch.len() < need {
            self.rx_scratch.resize(need, 0.0);
        }
        let dual = self.handle.dual_tuner();
        if dual && self.aux_scratch.len() < need {
            self.aux_scratch.resize(need, 0.0);
        }
        // Disjoint fields, so both landing buffers can be handed to the handle
        // while the handle borrows itself.
        let (main, aux) = (&mut self.rx_scratch[..need], &mut self.aux_scratch[..]);
        let pairs = self.handle.read_pair(main, if dual { aux } else { &mut [] });
        if pairs == 0 {
            // Nothing yet — brief nap so the DSP loop doesn't spin hot.
            std::thread::sleep(Duration::from_millis(2));
            return Ok(0);
        }
        for p in 0..pairs {
            buf[p] = Complex32::new(self.rx_scratch[2 * p], self.rx_scratch[2 * p + 1]);
        }
        if dual && !self.handle.aux_stalled() {
            if self.aux_buf.len() < pairs {
                self.aux_buf.resize(pairs, Complex32::new(0.0, 0.0));
            }
            for p in 0..pairs {
                self.aux_buf[p] =
                    Complex32::new(self.aux_scratch[2 * p], self.aux_scratch[2 * p + 1]);
            }
            if let Some(d) = self.diversity.as_mut() {
                d.process(&mut buf[..pairs], &self.aux_buf[..pairs]);
            }
        }
        if dual {
            self.log_depth();
        }
        Ok(pairs)
    }

    fn describe(&self) -> String {
        self.label.clone()
    }

    /// The two real gain elements, plus pseudo-elements for everything else.
    ///
    /// Both real elements are carried negated — `IF` is −(gain reduction),
    /// `LNA` is −(state) — so that on the sliders more is louder, like every
    /// other backend. The switches ride `SetGain` for the usual reason: no
    /// new `Command` variant for settings only this backend has.
    fn set_gain_element(&mut self, name: &str, db: f64) -> Result<()> {
        match name {
            SdrPlayConfig::IF_GAIN_ELEMENT => {
                let gr = (-db).round() as i32;
                let min = if self.extended_if_gr {
                    SdrPlayConfig::IF_GR_MIN_EXTENDED
                } else {
                    SdrPlayConfig::IF_GR_MIN
                };
                self.if_gr_db = gr.clamp(min, SdrPlayConfig::IF_GR_MAX);
                self.handle.set_if_gr_db(self.if_gr_db);
            }
            SdrPlayConfig::LNA_ELEMENT => {
                let state = (-db).round().clamp(0.0, 255.0) as u8;
                self.handle.set_lna_state(state);
            }
            SdrPlayConfig::AGC_ELEMENT => {
                self.agc = SdrPlayAgc::from_code(db);
                self.handle.set_agc(self.agc.code() as i32);
            }
            SdrPlayConfig::AGC_SETPOINT_ELEMENT => {
                self.handle.set_agc_setpoint(db.round() as i32);
            }
            SdrPlayConfig::PPM_ELEMENT => {
                self.handle.set_ppm(db);
            }
            SdrPlayConfig::BIAS_TEE_ELEMENT => {
                self.bias_tee = db >= 0.5;
                self.handle.set_bias_tee(self.bias_tee);
            }
            SdrPlayConfig::RF_NOTCH_ELEMENT => {
                self.handle.set_rf_notch(db >= 0.5);
            }
            SdrPlayConfig::DAB_NOTCH_ELEMENT => {
                self.handle.set_dab_notch(db >= 0.5);
            }
            SdrPlayConfig::HDR_ELEMENT => {
                self.hdr = db >= 0.5;
                self.handle.set_hdr(self.hdr);
                // Switching HDR moves the LO offset by half a megahertz (see
                // `lo_offset_hz`). Nothing has to be done about that here: the
                // engine compares its centre against `vfo + lo_offset_hz()` on
                // every pass and retunes when they part company, so the dial
                // stays where the operator left it and the hardware follows.
            }
            SdrPlayConfig::HDR_BW_ELEMENT => {
                self.hdr_bw = SdrPlayHdrBw::from_code(db);
                self.handle.set_hdr_bw(self.hdr_bw.code());
            }
            SdrPlayConfig::EXTENDED_IF_GR_ELEMENT => {
                self.extended_if_gr = db >= 0.5;
                self.handle.set_extended_if_gr(self.extended_if_gr);
                // The floor just moved under a reduction that may be below it.
                let min = if self.extended_if_gr {
                    SdrPlayConfig::IF_GR_MIN_EXTENDED
                } else {
                    SdrPlayConfig::IF_GR_MIN
                };
                if self.if_gr_db < min {
                    self.if_gr_db = min;
                    self.handle.set_if_gr_db(min);
                }
            }
            // The second tuner and its filter. Everything here is live: a null
            // is found by adjusting and listening, so a control that needed a
            // reconnect would be no use at all.
            // The second aerial's gains, and only while there *is* a second
            // aerial: with the tuners running as two radios the other one
            // belongs to the other radio, and reaching across to set its gain
            // from here would be one operator's slider moving another's
            // receiver.
            SdrPlayConfig::AUX_LNA_ELEMENT => {
                let state = (-db).round().clamp(0.0, 255.0) as u8;
                self.div_cfg.lna_state = state;
                if self.handle.dual_tuner() {
                    self.handle.set_aux_lna_state(state);
                }
            }
            SdrPlayConfig::AUX_IF_GAIN_ELEMENT => {
                let gr = (-db).round() as i32;
                self.div_cfg.if_gr_db =
                    gr.clamp(SdrPlayConfig::IF_GR_MIN, SdrPlayConfig::IF_GR_MAX);
                if self.handle.dual_tuner() {
                    self.handle.set_aux_if_gr_db(self.div_cfg.if_gr_db);
                }
            }
            SdrPlayConfig::DIV_MODE_ELEMENT => {
                self.div_cfg.mode =
                    if db >= 0.5 { DiversityMode::Combine } else { DiversityMode::Cancel };
                if let Some(d) = self.diversity.as_mut() {
                    // The filter is kept across the switch: it is the same
                    // estimate either way, and throwing away a converged one
                    // would cost the operator the convergence they watched.
                    d.set_mode(div_mode(self.div_cfg.mode));
                }
            }
            SdrPlayConfig::DIV_RATE_ELEMENT => {
                self.div_cfg.rate = db as f32;
                if let Some(d) = self.diversity.as_mut() {
                    d.set_rate(self.div_cfg.rate);
                }
            }
            SdrPlayConfig::DIV_TAPS_ELEMENT => {
                let taps =
                    db.round().clamp(1.0, f64::from(sdroxide_types::DIVERSITY_MAX_TAPS)) as u8;
                self.div_cfg.taps = taps;
                if let Some(d) = self.diversity.as_mut() {
                    d.set_taps(usize::from(taps));
                }
            }
            SdrPlayConfig::DIV_FREEZE_ELEMENT => {
                self.div_cfg.frozen = db >= 0.5;
                if let Some(d) = self.diversity.as_mut() {
                    d.set_frozen(self.div_cfg.frozen);
                }
            }
            SdrPlayConfig::DIV_RESET_ELEMENT => {
                if let Some(d) = self.diversity.as_mut()
                    && db >= 0.5
                {
                    d.reset();
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// What the hardware is actually doing. `IF` prefers the GainChange
    /// event's answer, which follows the AGC; `LNA` is the programmed state
    /// after any per-band clamp.
    fn current_gains(&self) -> Vec<(String, f64)> {
        let gr = self.handle.effective_if_gr_db().unwrap_or(self.if_gr_db);
        vec![
            (SdrPlayConfig::LNA_ELEMENT.to_string(), -(self.handle.lna_state() as f64)),
            (SdrPlayConfig::IF_GAIN_ELEMENT.to_string(), -(gr as f64)),
        ]
    }

    /// What the API says is between the antenna socket and the converter, so
    /// the engine's dBFS measurement can be referred back to the aerial.
    ///
    /// Worth having on this backend more than on most: the RSP's own AGC is on
    /// by default and owns the IF gain, so the gain here moves whether the
    /// operator touches anything or not. Without this the S-meter would report
    /// the loop working — a signal appearing would raise the reading, the AGC
    /// would take the gain back down, and the reading would sink to where it
    /// started while the signal was still there.
    ///
    /// `None` until the first block arrives carrying a settled gain, which is
    /// the first block of the session: the engine then falls back to raw dBFS
    /// for the fraction of a second before the receiver has delivered
    /// anything, which is the same thing it shows for every other front end.
    ///
    /// The pair's second aerial is deliberately not in this: the two branches
    /// are combined by an adaptive filter with a gain of its own, so no single
    /// front-end figure describes what comes out. The main tuner's is the
    /// honest half, and it is the one the operator's own controls move.
    fn rx_gain_db(&self) -> Option<f32> {
        self.handle.system_gain_db().map(|db| db as f32)
    }

    /// The LNA ladder as this band has it — see [`Self::rx_gain_elements`].
    /// The engine asks after a retune, a port change and the HDR switch, which
    /// are the three things that can change the answer.
    fn learned_rx_gains(&self) -> Option<Vec<GainElement>> {
        Some(self.rx_gain_elements())
    }

    fn set_antenna(&mut self, name: &str) -> Result<()> {
        self.antenna = name.to_string();
        self.handle.set_antenna(name);
        Ok(())
    }

    fn current_antenna(&self) -> String {
        self.antenna.clone()
    }

    /// An RSP whose service session died — unplug, service restart — reports
    /// as needing a reopen so the engine reconnects on its own.
    fn needs_reopen(&self) -> bool {
        !self.handle.is_alive() || self.handle.silent_for() >= SILENCE_BEFORE_REOPEN
    }

    /// Hand the device back to the service before the engine opens its
    /// replacement; without this, Apply fails with "in use" — by us.
    fn release(&mut self) {
        self.handle.release();
    }

    /// Standing conditions an operator needs to see: a degraded enumeration,
    /// DC on the antenna port, and an ADC being overloaded right now.
    fn open_status(&self) -> Option<String> {
        let mut notes = Vec::new();
        // A device the service lists without an identity streams but often
        // hears nothing — and nothing else about the session looks wrong, so
        // this note is the only thing standing between the operator and a
        // deaf receiver with no explanation.
        if let Some(w) =
            sdroxide_types::SdrPlayDevice::degraded_warning(self.handle.serial(), self.model())
        {
            notes.push(w);
        }
        if self.bias_tee {
            notes.push(format!("{}: bias tee is ON — ~4.7 V DC on the antenna port", self.label));
        }
        if self.handle.overloaded() {
            notes.push(
                "RF overload — raise the LNA slider (more attenuation), lower IF gain, or \
                 enable AGC"
                    .to_string(),
            );
        }
        // HDR is not a mode that follows the dial: the path has a fixed analog
        // filter, built only at a short list of centres. Tuned anywhere else
        // the switch is accepted and does nothing, which reads as HDR being
        // broken rather than as the dial being in the wrong place — so say
        // where it does work rather than only that it does not.
        if self.hdr && self.model().has_hdr() && !self.hdr_bw.works_at(self.center) {
            notes.push(format!(
                "HDR is on but does nothing at {:.3} MHz: the {} path is only built at {}.",
                self.center / 1e6,
                self.hdr_bw.label(),
                self.hdr_bw.centres_label(),
            ));
        }
        // A setting that quietly did nothing is worse than one that is
        // refused: only an RSPduo has a second tuner, and only the API's
        // dual-tuner mode runs it.
        let running = if self.handle.dual_tuner() {
            DuoMode::Paired
        } else if self.handle.split_tuner() {
            DuoMode::Split
        } else {
            DuoMode::Single
        };
        match (self.div_cfg.enabled, running) {
            (true, DuoMode::Single) => notes.push(if self.model() == SdrPlayModel::RspDuo {
                "Both tuners were asked for but the device opened with one — another \
                 application may be holding the RSPduo, or holding it in single-tuner mode."
                    .to_string()
            } else {
                format!(
                    "Two tuners were asked for and an {} has one, so the second is off.",
                    self.model().label()
                )
            }),
            (_, DuoMode::Paired) => {
                notes.push(format!(
                    "Diversity is running on the RSPduo's second tuner — {}. Watch the log for \
                     the depth it is reaching.",
                    match self.div_cfg.mode {
                        DiversityMode::Cancel => "cancelling a noise source",
                        DiversityMode::Combine => "combining two aerials",
                    }
                ));
                // The second tuner's ladder is shorter on some bands than
                // others, exactly like the first one's — and unlike the first
                // one's, no gain readout on screen would ever show it.
                let programmed = self.handle.aux_lna_state();
                if programmed != self.div_cfg.lna_state {
                    notes.push(format!(
                        "The second tuner's LNA state is clamped to {programmed} in this band \
                         (you asked for {}); your choice comes back when you tune somewhere it \
                         fits.",
                        self.div_cfg.lna_state
                    ));
                }
            }
            (_, DuoMode::Split) => notes.push(format!(
                "Running on the RSPduo's tuner {}, with its other tuner free for a second \
                 radio on its own frequency. Both share one ADC clock, so both run at \
                 {:.3} Msps.",
                self.handle.tuner() + 1,
                self.handle.out_rate_hz() / 1e6,
            )),
            (false, DuoMode::Single) => {}
        }
        if notes.is_empty() { None } else { Some(notes.join("\n")) }
    }
}
