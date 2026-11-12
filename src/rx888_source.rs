//! An [`IqSource`] for an RX-888 Mk2 driven over USB by the native driver in
//! `sdroxide-rx888` — no SoapySDR, no libusb, and no vendor driver package.
//!
//! Receive only: the trait's transmit methods already default to errors, which
//! is the correct answer for a receiver.
//!
//! # What this source is doing that the others are not
//!
//! The hardware has no downconverter. It sends 16-bit *real* samples at the full
//! ADC rate — 64.8 Msps covering 0–32.4 MHz at the default clock — and an FFT
//! downconverter in the backend turns a selectable slice of that into complex
//! baseband: 1/32 of the clock by default, up to the whole half-spectrum in the
//! panadapter at once. Three consequences leak out here:
//!
//! * [`IqSource::sample_rate`] is the *downconverter's* output rate, not the
//!   ADC clock. The engine's own DDC chain then works from that as usual.
//! * Retuning is free. There is no LO to move and no I2C to wait for — a retune
//!   is a change of FFT bin — so dragging the panadapter across the whole of HF
//!   costs nothing.
//! * The downconverter's centre cannot go everywhere the dial can: it clamps to
//!   what keeps the output inside the half-spectrum, and the wider the output
//!   the larger the strip of dial it cannot centre on. Where that happens the
//!   achieved centre is reported through [`IqSource::poll_control`] — see
//!   [`achieved_center_hz`].

use sdroxide_rx888::device::{ad8370_code_in, ad8370_db_for, att_code_for, att_db_for};
use std::time::Duration;

use sdroxide_radio::{Complex32, ControlUpdate, IqSource, Result};
use sdroxide_rx888::band::{self, Band, BandState};
use sdroxide_rx888::{Rx888Handle, Settings};
use sdroxide_types::Rx888Config;

/// How long the receiver may deliver nothing before the link counts as dead.
/// Matches the RTL-SDR backend: this is a local USB device, so there is no
/// network to be briefly slow.
const SILENCE_BEFORE_REOPEN: Duration = Duration::from_secs(3);

/// Translate the persisted config into what the driver wants.
fn settings_from(cfg: &Rx888Config) -> Settings {
    Settings {
        serial: Some(cfg.serial.trim().to_string()).filter(|s| !s.is_empty()),
        adc_rate_hz: cfg.adc_rate_hz,
        dither: cfg.dither,
        randomize: cfg.randomize,
        bias_tee_hf: cfg.bias_tee_hf,
        pga: cfg.pga,
        attenuator_db: cfg.attenuator_db,
        vga_db: cfg.vga_db,
        ppm: cfg.ppm,
        bias_tee_vhf: cfg.bias_tee_vhf,
        tuner_gain_db: cfg.tuner_gain_db,
        tuner_agc: cfg.tuner_agc,
        ddc_bins: cfg.ddc_bins as usize,
    }
}

/// Where the stream's centre actually lands for this dial, tracked with the
/// same pure band plan the device runs.
///
/// The downconverter's centre clamps to what fits in the half-spectrum (see
/// [`band::achieved_dial_center_hz`]); the widest output pins it at a quarter
/// of the ADC clock however the dial moves, and on VHF an output too wide to
/// park on the tuner's IF carrier leaves the dial a fixed offset inside the
/// span. The engine has to be told, or it demodulates against a centre the
/// converter never took — so [`Rx888Source::set_center_hz`] computes the truth
/// here and reports it through [`IqSource::poll_control`], the same adoption
/// path a shared-LO device uses when its centre moves.
///
/// The band state is mirrored rather than asked of the device because the
/// device's copy lives on the USB thread; the plan is pure, both start from
/// the same state, and they see the same dials in the same order. The one
/// divergence is a VHF entry that fails on real hardware and falls back to HF
/// aliasing — reception is already wrong there in a way no centre report
/// could mend.
fn achieved_center_hz(
    dial_hz: f64,
    adc_rate_hz: f64,
    out_rate_hz: f64,
    vhf: bool,
    st: &mut BandState,
) -> f64 {
    let p = band::plan(dial_hz, adc_rate_hz, out_rate_hz, vhf, *st);
    *st = BandState { band: p.band, lo_dial_hz: p.lo_dial_hz };
    // The alias sliver between Nyquist and the crossover-plus-hysteresis
    // stays uncorrected on purpose: it is how a scrolled dial climbs into
    // the VHF range. A correction there would be adopted, the adoption
    // clamps the VFO back under Nyquist, and the crossover could never be
    // reached by scrolling again. Reception in the sliver is aliased
    // regardless, so there is no truth to protect.
    if p.band == Band::Hf && dial_hz > adc_rate_hz / 2.0 {
        return dial_hz;
    }
    band::achieved_dial_center_hz(&p, adc_rate_hz, out_rate_hz)
}

pub struct Rx888Source {
    handle: Rx888Handle,
    center: f64,
    label: String,
    vga_db: f64,
    /// The VGA's range as last asked for, reported back as a pseudo-gain so
    /// the top bar's chip reads what is in force.
    vga_range: sdroxide_rx888::device::VgaRange,
    att_db: f64,
    bias_tee: bool,
    bias_tee_vhf: bool,
    tuner_gain_db: f64,
    /// Mirror of the device's band machine — see [`achieved_center_hz`].
    band_state: BandState,
    /// A centre the converter could not take verbatim, waiting to be reported
    /// through [`IqSource::poll_control`]. Overwritten by every tune, so only
    /// the latest correction is ever delivered — a stale one would have the
    /// engine adopt a centre the stream has already left.
    pending_center: Option<f64>,
}

impl Rx888Source {
    pub fn open(cfg: &Rx888Config, center_hz: f64) -> anyhow::Result<Self> {
        let firmware = cfg.firmware_path.trim();
        if !firmware.is_empty() {
            tracing::info!("RX-888: using firmware image {firmware}");
        }
        let handle = sdroxide_rx888::spawn(&settings_from(cfg), center_hz)?;
        let label = format!(
            "{} @ {:.1} Msps ADC → {:.3} Msps",
            handle.label(),
            handle.adc_rate_hz() / 1e6,
            handle.out_rate_hz() / 1e6,
        );
        tracing::info!("RX-888 source ready: {label}, center {center_hz:.0} Hz");
        let mut band_state = BandState::default();
        let achieved = achieved_center_hz(
            center_hz,
            handle.adc_rate_hz(),
            handle.out_rate_hz(),
            handle.vhf_capable(),
            &mut band_state,
        );
        Ok(Rx888Source {
            // The requested dial, not the achieved centre: the engine seeds its
            // VFOs from `center_hz()` when it opens, and the operator's saved
            // dial must survive that. The correction below moves only the
            // centre, through the adoption path that leaves the VFOs alone.
            center: center_hz,
            label,
            vga_db: cfg.vga_db,
            vga_range: Default::default(),
            att_db: cfg.attenuator_db,
            bias_tee: cfg.bias_tee_hf,
            bias_tee_vhf: cfg.bias_tee_vhf,
            tuner_gain_db: cfg.tuner_gain_db,
            band_state,
            pending_center: ((achieved - center_hz).abs() >= 0.5).then_some(achieved),
            handle,
        })
    }

    /// The complex baseband rate the downconverter produces.
    pub fn sample_rate_hz(&self) -> f64 {
        self.handle.out_rate_hz()
    }

    /// The ADC clock, for the settings UI to report.
    pub fn adc_rate_hz(&self) -> f64 {
        self.handle.adc_rate_hz()
    }

    /// Whether this receiver has a usable VHF front end.
    pub fn vhf_capable(&self) -> bool {
        self.handle.vhf_capable()
    }

    /// The ranges this receiver can reach, HF and — where it has a tuner — VHF.
    pub fn freq_ranges(&self) -> Vec<(f64, f64)> {
        sdroxide_rx888::band::freq_ranges(self.adc_rate_hz(), self.vhf_capable())
    }
}

impl IqSource for Rx888Source {
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
        let achieved = achieved_center_hz(
            hz,
            self.handle.adc_rate_hz(),
            self.handle.out_rate_hz(),
            self.handle.vhf_capable(),
            &mut self.band_state,
        );
        self.pending_center = ((achieved - hz).abs() >= 0.5).then_some(achieved);
        self.handle.set_center_hz(hz);
        Ok(())
    }

    /// The one update this receiver volunteers: where the stream's centre
    /// really is, after a tune the downconverter could not take verbatim — a
    /// dial closer to DC or Nyquist than half the output width, or any dial at
    /// all on the full-Nyquist width, whose centre never leaves a quarter of
    /// the ADC clock. The engine adopts it exactly as it adopts a shared-LO
    /// sibling's retune: the demodulator offset is corrected, the VFO stays
    /// where the operator put it (clamped into the span it can still reach),
    /// and nothing is commanded back at the hardware.
    fn poll_control(&mut self) -> Vec<ControlUpdate> {
        self.pending_center.take().map(ControlUpdate::Center).into_iter().collect()
    }

    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        // Straight into the caller's block: the ring's interleaved floats and a
        // complex buffer are the same bytes, so there is nothing to unpack —
        // see `sdroxide_dsp::as_interleaved_mut`.
        let n = self.handle.read(sdroxide_dsp::as_interleaved_mut(buf));
        let pairs = n / 2;
        if pairs == 0 {
            // Nothing yet — brief nap so the DSP loop doesn't spin hot.
            std::thread::sleep(Duration::from_millis(2));
            return Ok(0);
        }
        Ok(pairs)
    }

    /// A centre of its own — the downconverter moves for nothing; the driver clamps
    /// the centre to what its half-spectrum allows, so at the band's ends the
    /// window comes as far as it can — so the engine may centre the I/Q on a
    /// resting dial (`Engine::recentre_after_dwell`).
    fn recentre_on_dwell(&self) -> bool {
        true
    }

    fn describe(&self) -> String {
        self.label.clone()
    }

    /// The two analogue gain stages, plus pseudo-elements for the switches.
    ///
    /// Routing the switches through `SetGain` rather than adding `Command`
    /// variants keeps `Command`, `DeviceCaps` and the engine untouched for
    /// settings only this backend has — the same trick the RTL-SDR backend uses
    /// for its AGC and bias tee.
    fn set_gain_element(&mut self, name: &str, db: f64) -> Result<()> {
        match name {
            Rx888Config::VGA_ELEMENT => {
                self.vga_db = db;
                self.handle.set_vga_db(db);
            }
            Rx888Config::VGA_RANGE_ELEMENT => {
                self.vga_range = sdroxide_rx888::device::VgaRange::from_code(db);
                self.handle.set_vga_range(self.vga_range);
            }
            Rx888Config::ATT_ELEMENT => {
                self.att_db = db;
                self.handle.set_att_db(db);
            }
            Rx888Config::DITHER_ELEMENT => self.handle.set_dither(db >= 0.5),
            Rx888Config::BIAS_TEE_ELEMENT => {
                self.bias_tee = db >= 0.5;
                self.handle.set_bias_tee(self.bias_tee);
            }
            Rx888Config::PGA_ELEMENT => self.handle.set_pga(db >= 0.5),
            Rx888Config::TUNER_GAIN_ELEMENT => {
                self.tuner_gain_db = db;
                self.handle.set_tuner_gain_db(db);
            }
            Rx888Config::TUNER_AGC_ELEMENT => self.handle.set_tuner_agc(db >= 0.5),
            Rx888Config::BIAS_TEE_VHF_ELEMENT => {
                self.bias_tee_vhf = db >= 0.5;
                self.handle.set_bias_tee_vhf(self.bias_tee_vhf);
            }
            _ => {}
        }
        Ok(())
    }

    fn current_gains(&self) -> Vec<(String, f64)> {
        vec![
            // What the hardware is programmed with, since both stages
            // quantise: the VGA onto a linear voltage vernier — and, with a
            // range forced, onto that range's span — and the attenuator onto
            // a 0.5 dB grid. Worked out here from the request rather than
            // read from the stream thread's report: the engine asks right
            // after the request, before the thread has applied it, and the
            // thread never filled that report anyway, so the bar echoed the
            // request (a VGA forced LOW read 30 dB while running 16.9).
            // Why: HDZ 2026-11-09: "da stimmt was nicht am Treiber und
            //      Visual".
            (
                Rx888Config::VGA_ELEMENT.to_string(),
                ad8370_db_for(ad8370_code_in(self.vga_db, self.vga_range)),
            ),
            (Rx888Config::ATT_ELEMENT.to_string(), att_db_for(att_code_for(self.att_db))),
            (Rx888Config::VGA_RANGE_ELEMENT.to_string(), self.vga_range.code()),
        ]
    }

    /// The whole of HF at once, at about twenty frames a second.
    ///
    /// The backend analyses roughly 2 % of the samples going past to build this,
    /// so it costs about 1 % of what the downconverter costs — see
    /// `sdroxide_dsp::WideSpectrum`.
    fn wide_spectrum_db(&mut self, out: &mut Vec<f32>) -> Option<(f64, f64)> {
        // The axis comes with the frame rather than being asked for: above the
        // VHF crossover the strip is a slice of the tuner's IF mapped back to
        // RF, and it moves with the tuner, so a frame captured before a retune
        // must not be drawn on the axis that came after it.
        let frame = self.handle.take_wide_spectrum()?;
        out.clear();
        out.extend_from_slice(&frame.bins);
        Some((frame.center_hz, frame.span_hz))
    }

    /// A receiver that has been unplugged, or whose threads have died, is
    /// reported as needing a reopen so the engine reconnects on its own.
    fn needs_reopen(&self) -> bool {
        !self.handle.is_alive() || self.handle.silent_for() >= SILENCE_BEFORE_REOPEN
    }

    /// Hand the device back before the engine opens its replacement — without
    /// this, pressing Apply in Settings → Radio fails with "held by another
    /// program", the other program being us.
    fn release(&mut self) {
        self.handle.release();
    }

    /// Surface what an operator needs to know but cannot see: a link too slow
    /// for the requested sample rate, or DC sitting on the antenna feedline.
    fn open_status(&self) -> Option<String> {
        let mut notes: Vec<String> = Vec::new();
        if let Some(w) = self.handle.warning() {
            notes.push(w.to_string());
        }
        if self.bias_tee {
            notes.push(format!("{}: HF bias tee is ON — DC on the antenna coax", self.label));
        }
        if self.bias_tee_vhf {
            notes.push(format!("{}: VHF bias tee is ON — DC on the antenna coax", self.label));
        }
        (!notes.is_empty()).then(|| notes.join("  •  "))
    }
}
