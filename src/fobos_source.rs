//! An [`IqSource`] for a RigExpert Fobos SDR driven through `libfobos` by the
//! native driver in `sdroxide-fobos` — no SoapySDR; the vendor library itself
//! is found with dlopen at runtime, matching the SDRplay and LimeSDR
//! backends.
//!
//! Receive only: the trait's transmit methods already default to errors,
//! which is the correct answer for this hardware.
//!
//! RF port, single-channel HF (direct sampling), and dual-channel HF
//! (`FobosPort::HfDual`, both real ADC channels combined by a
//! [`sdroxide_dsp::Diversity`] filter) all work. The combiner lives here
//! rather than inside `sdroxide-fobos` itself — same layer
//! `sdroxide-sdrplay`'s own dual-tuner combining lives at — running the
//! plain adaptive NLMS filter: Cancel or Combine mode, configurable taps
//! and adaptation rate, with a freeze/hold control.

use std::time::{Duration, Instant};

use sdroxide_dsp::{Complex32, Diversity};
use sdroxide_fobos::{FobosHandle, OpenParams, Port};
use sdroxide_radio::{ControlUpdate, IqSource, Result};
use sdroxide_types::{DiversityMode, FobosConfig, FobosPort};

/// How long the receiver may deliver nothing before the connection counts as
/// dead. Same three seconds as the other native USB-adjacent backends: this
/// is a local device, so there is no network to be briefly slow.
const SILENCE_BEFORE_REOPEN: Duration = Duration::from_secs(3);

/// How often the diversity filter's achieved null depth reaches the log —
/// same interval `sdroxide-sdrplay`'s own dual-tuner combining uses.
const DEPTH_LOG_INTERVAL: Duration = Duration::from_secs(10);

fn to_driver_port(p: FobosPort) -> Port {
    match p {
        FobosPort::Rf => Port::Rf,
        FobosPort::Hf1 => Port::Hf1,
        FobosPort::Hf2 => Port::Hf2,
        FobosPort::HfDual => Port::HfDual,
    }
}

/// Where a dial the operator asked for actually lands. `None` on `Rf` — the
/// tuner's mixer reaches anywhere in its own range, nothing to correct — but
/// on the HF ports the downconverter can refuse a centre closer to DC (or to
/// Nyquist) than half its own output rate, silently: `WbDdc::set_center_hz`
/// clamps rather than erroring, since a live retune has nowhere to report a
/// `Result` to (see `sdroxide_dsp::clamp_center_hz`'s own doc comment). This
/// mirrors that clamp using only the two rates `FobosHandle` already
/// reports, rather than asking the DDC instances themselves — which live on
/// the stream thread's own converter, not here.
///
/// Skipping this and trusting the requested dial is exactly the bug this
/// fixes: the operator's own pan/zoom kept scrolling the frequency axis past
/// the reachable floor while the spectrum itself stayed pinned at the true,
/// clamped centre — the axis and the spectrum visibly disagreeing about
/// where the receiver actually was.
fn achieved_center_hz(
    port: FobosPort,
    requested_hz: f64,
    adc_rate_hz: f64,
    out_rate_hz: f64,
) -> Option<f64> {
    match port {
        FobosPort::Rf => None,
        FobosPort::Hf1 | FobosPort::Hf2 | FobosPort::HfDual => {
            Some(sdroxide_dsp::clamp_center_hz(requested_hz, adc_rate_hz, out_rate_hz))
        }
    }
}

/// The configuration's mode, as the DSP crate spells it — two different
/// enums with the same two variants, same reasoning as every other
/// diversity-capable backend here: `sdroxide_types::DiversityMode` is the
/// wasm-safe config mirror, `sdroxide_dsp::DiversityMode` is what the filter
/// actually runs on.
fn dsp_div_mode(mode: DiversityMode) -> sdroxide_dsp::DiversityMode {
    match mode {
        DiversityMode::Cancel => sdroxide_dsp::DiversityMode::Cancel,
        DiversityMode::Combine => sdroxide_dsp::DiversityMode::Combine,
    }
}

pub struct FobosSource {
    handle: FobosHandle,
    center: f64,
    rx_scratch: Vec<f32>,
    label: String,
    /// Mirrors of the settings the panel drives, so `current_gains` and
    /// `open_status` can answer without a round trip to the stream thread.
    port: FobosPort,
    lna_gain: u8,
    vga_gain: u8,

    /// A centre the HF downconverter could not take verbatim, waiting to be
    /// reported through [`IqSource::poll_control`] — see
    /// [`achieved_center_hz`]. Overwritten by every tune, so only the latest
    /// correction is ever delivered; always `None` on `Rf`.
    pending_center: Option<f64>,

    /// `Some` only on `FobosPort::HfDual` — the combiner for the two real
    /// channels `sdroxide-fobos::FobosHandle::rx_read_pair` hands back.
    diversity: Option<Diversity>,
    div_mode: DiversityMode,
    aux_scratch: Vec<f32>,
    aux_buf: Vec<Complex32>,
    last_depth_log: Instant,
}

impl FobosSource {
    pub fn open(cfg: &FobosConfig, center_hz: f64) -> anyhow::Result<Self> {
        let params = OpenParams {
            serial: cfg.serial.clone(),
            port: to_driver_port(cfg.port),
            center_hz,
            sample_rate_hz: cfg.sample_rate_hz,
            lna_gain: cfg.lna_gain,
            vga_gain: cfg.vga_gain,
            clk_external: cfg.clk_external,
        };
        let handle = FobosHandle::open(params).map_err(|e| anyhow::anyhow!("{e}"))?;
        let label = format!("{} @ {:.4} Msps", handle.label, handle.sample_rate_hz / 1e6);
        let diversity = (cfg.port == FobosPort::HfDual).then(|| {
            Diversity::new(dsp_div_mode(cfg.div_mode), usize::from(cfg.div_taps), cfg.div_rate)
        });
        tracing::info!(
            "Fobos source ready: {label}, center {center_hz:.0} Hz{}",
            if diversity.is_some() {
                format!(
                    ", diversity {} filter, {} taps",
                    match cfg.div_mode {
                        DiversityMode::Cancel => "cancelling",
                        DiversityMode::Combine => "combining",
                    },
                    cfg.div_taps,
                )
            } else {
                String::new()
            }
        );
        let achieved =
            achieved_center_hz(cfg.port, center_hz, handle.adc_rate_hz, handle.sample_rate_hz);
        let pending_center = achieved.filter(|a| (a - center_hz).abs() >= 0.5);
        if let Some(a) = pending_center {
            tracing::warn!(
                "Fobos: {center_hz:.0} Hz requested is below {}'s reachable floor at the \
                 current bandwidth — opened at {a:.0} Hz instead. Lower the sample rate in \
                 Settings → Radio to reach lower frequencies, or retune above the floor.",
                cfg.port.name()
            );
        }
        Ok(FobosSource {
            center: center_hz,
            rx_scratch: Vec::new(),
            label,
            port: cfg.port,
            lna_gain: cfg.lna_gain,
            vga_gain: cfg.vga_gain,
            pending_center,
            diversity,
            div_mode: cfg.div_mode,
            aux_scratch: Vec::new(),
            aux_buf: Vec::new(),
            last_depth_log: Instant::now(),
            handle,
        })
    }

    pub fn model_rates(&self) -> &[f64] {
        &self.handle.rates_hz
    }

    /// The ADC's own real rate for this session — on `Rf` the same as
    /// [`IqSource::sample_rate`], on the HF ports the raw rate the
    /// downconverter decimates from. For the settings UI to report, and for
    /// callers outside this module (`fobos_caps`) to compute the HF ports'
    /// reachable tuning range the same way [`achieved_center_hz`] does.
    pub fn adc_rate_hz(&self) -> f64 {
        self.handle.adc_rate_hz
    }

    /// Say how the combiner is doing, occasionally.
    ///
    /// The null depth is the one number that separates "the second channel
    /// hears the interference" from "the second channel hears nothing the
    /// first one doesn't", and no amount of adjusting the filter fixes the
    /// second case.
    fn log_depth(&mut self) {
        let Some(d) = self.diversity.as_ref() else { return };
        if self.last_depth_log.elapsed() < DEPTH_LOG_INTERVAL {
            return;
        }
        self.last_depth_log = Instant::now();
        if let Some(db) = d.depth_db() {
            tracing::info!(
                "Fobos diversity: {db:.1} dB of the main channel's signal is being \
                 cancelled{}",
                if d.frozen() { ", filter held" } else { "" },
            );
        }
    }
}

impl IqSource for FobosSource {
    /// The engine is transmitting and has stopped reading, or has started
    /// again. Passed straight through to the stream thread, which keeps
    /// receiving either way — this only decides whether a full ring is
    /// reported as an overrun or as the ordinary cost of an over. See
    /// [`IqSource::set_rx_paused`].
    fn set_rx_paused(&mut self, paused: bool) {
        self.handle.set_rx_paused(paused);
    }

    /// The achieved rate, read live: on `Rf` a rate change moves it under
    /// the engine, and on the HF ports the requested target and the
    /// `WbDdc`-quantised achieved rate can genuinely differ — see
    /// [`FobosConfig::sample_rate_hz`]'s own doc comment.
    fn sample_rate(&self) -> f64 {
        self.handle.current_rate_hz()
    }

    fn center_hz(&self) -> f64 {
        self.center
    }

    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center = hz;
        let achieved = achieved_center_hz(
            self.port,
            hz,
            self.handle.adc_rate_hz,
            self.handle.current_rate_hz(),
        );
        self.pending_center = achieved.filter(|a| (a - hz).abs() >= 0.5);
        self.handle.set_center_hz(hz);
        Ok(())
    }

    /// The one update this receiver volunteers: where the HF downconverter's
    /// centre really is, after a tune it could not take verbatim — see
    /// [`achieved_center_hz`]. The engine adopts it the same way it adopts a
    /// shared-LO sibling's retune: the demodulator offset is corrected, the
    /// operator's own dial stays where they put it, and nothing is commanded
    /// back at the hardware.
    fn poll_control(&mut self) -> Vec<ControlUpdate> {
        self.pending_center.take().map(ControlUpdate::Center).into_iter().collect()
    }

    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        let need = buf.len() * 2;
        if self.rx_scratch.len() < need {
            self.rx_scratch.resize(need, 0.0);
        }

        let pairs = if let Some(diversity) = self.diversity.as_mut() {
            if self.aux_scratch.len() < need {
                self.aux_scratch.resize(need, 0.0);
            }
            let n = self
                .handle
                .rx_read_pair(&mut self.rx_scratch[..need], &mut self.aux_scratch[..need]);
            if n > 0 {
                for p in 0..n {
                    buf[p] = Complex32::new(self.rx_scratch[2 * p], self.rx_scratch[2 * p + 1]);
                }
                if self.aux_buf.len() < n {
                    self.aux_buf.resize(n, Complex32::new(0.0, 0.0));
                }
                for p in 0..n {
                    self.aux_buf[p] =
                        Complex32::new(self.aux_scratch[2 * p], self.aux_scratch[2 * p + 1]);
                }
                diversity.process(&mut buf[..n], &self.aux_buf[..n]);
            }
            n
        } else {
            let n = self.handle.rx_read(&mut self.rx_scratch[..need]);
            let pairs = n / 2;
            for (p, out) in buf.iter_mut().enumerate().take(pairs) {
                *out = Complex32::new(self.rx_scratch[2 * p], self.rx_scratch[2 * p + 1]);
            }
            pairs
        };

        if pairs == 0 {
            // Nothing yet — brief nap so the DSP loop doesn't spin hot.
            std::thread::sleep(Duration::from_millis(2));
            return Ok(0);
        }
        if self.diversity.is_some() {
            self.log_depth();
        }
        Ok(pairs)
    }

    /// A centre of its own — the tuner's LO — so the engine may centre the I/Q on a
    /// resting dial (`Engine::recentre_after_dwell`).
    fn recentre_on_dwell(&self) -> bool {
        true
    }

    fn describe(&self) -> String {
        self.label.clone()
    }

    /// Two real gain elements (`Rf` only — see [`FobosPort`]'s own doc
    /// comment) plus pseudo-elements for the clock-source switch and — on
    /// `FobosPort::HfDual` — the diversity filter, riding the same shared
    /// `DIV_*_ELEMENT` names every diversity-capable backend here uses (see
    /// `sdroxide_types::DIV_MODE_ELEMENT`'s own doc comment: the main
    /// window's strip drives them without needing to know which backend has
    /// a filter). `port` itself is not here: switching it means a different
    /// hardware path and — on the HF ports — rebuilding the downconverter,
    /// so it is a config-only field the settings tab asks for a reconnect
    /// over, the same way `sdroxide-hydrasdr`'s own 12-bit packing switch
    /// works.
    fn set_gain_element(&mut self, name: &str, db: f64) -> Result<()> {
        match name {
            FobosConfig::LNA_GAIN_ELEMENT => {
                self.lna_gain = db.clamp(0.0, f64::from(FobosConfig::LNA_GAIN_MAX)) as u8;
                self.handle.set_lna_gain(self.lna_gain);
            }
            FobosConfig::VGA_GAIN_ELEMENT => {
                self.vga_gain = db.clamp(0.0, f64::from(FobosConfig::VGA_GAIN_MAX)) as u8;
                self.handle.set_vga_gain(self.vga_gain);
            }
            FobosConfig::CLK_EXTERNAL_ELEMENT => {
                self.handle.set_clk_external(db >= 0.5);
            }
            sdroxide_types::DIV_MODE_ELEMENT => {
                self.div_mode =
                    if db >= 0.5 { DiversityMode::Combine } else { DiversityMode::Cancel };
                if let Some(d) = self.diversity.as_mut() {
                    d.set_mode(dsp_div_mode(self.div_mode));
                }
            }
            sdroxide_types::DIV_RATE_ELEMENT => {
                if let Some(d) = self.diversity.as_mut() {
                    d.set_rate(db as f32);
                }
            }
            sdroxide_types::DIV_TAPS_ELEMENT => {
                let taps =
                    db.round().clamp(1.0, f64::from(sdroxide_types::DIVERSITY_MAX_TAPS)) as u8;
                if let Some(d) = self.diversity.as_mut() {
                    d.set_taps(usize::from(taps));
                }
            }
            sdroxide_types::DIV_FREEZE_ELEMENT => {
                if let Some(d) = self.diversity.as_mut() {
                    d.set_frozen(db >= 0.5);
                }
            }
            sdroxide_types::DIV_RESET_ELEMENT => {
                // RESTART's own tooltip promises "zero the filter and find
                // the null again" — the second half needs adaptation to
                // actually resume, which `Diversity::reset()` alone does not
                // do: it clears the taps but leaves `frozen` exactly as it
                // was. With HOLD on (the common case: Restart is naturally
                // reached for right after a held null has gone stale), that
                // left the filter permanently parked at zero — in `Cancel`
                // mode indistinguishable from `Combine`, with nothing on
                // screen to say why — because `process()` gates every
                // update on `!frozen`, and reset alone never clears it. A
                // real capture on this hardware reproduced exactly that: a
                // 33 dB null vanished the instant Restart was pressed with
                // HOLD selected, and did not return until HOLD was
                // separately turned off.
                //
                // Fixed here rather than in `Diversity::reset()` itself (the
                // same gap exists at `LimeConfig::DIV_RESET_ELEMENT` and the
                // SdrPlay equivalent) to keep this branch's diff scoped to
                // Fobos for a clean PR. The better home for this fix is
                // almost certainly `sdroxide_dsp::Diversity::reset()`
                // itself — one change would then cover all three backends
                // instead of three separate copies of the same patch — and
                // that's worth raising in review rather than deciding here.
                //
                // One known gap this local fix does not close: the DIV
                // strip's own HOLD chip is a client-side cache
                // (`top_bar.rs`'s `radio_cfg`) that this call has no way to
                // reach, so it stays lit — and, confirmed on real use,
                // reopening Settings does *not* fix it either, because
                // `radio_config()` (`local_controller.rs`) answers from the
                // *persisted* config file, not the live engine, and nothing
                // here writes the correction back to that file. Clicking
                // HOLD once does: with the chip showing (stale) `true`, the
                // click sends `set_frozen(false)` — a no-op on the already-
                // unfrozen live filter, but it also persists `div_frozen:
                // false` and updates the UI's own cache, so the chip
                // un-lights for good. Cosmetic only — the filter really is
                // unfrozen underneath the whole time — but worth the same
                // review note as the rest of this comment.
                if db >= 0.5
                    && let Some(d) = self.diversity.as_mut()
                {
                    d.reset();
                    d.set_frozen(false);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn current_gains(&self) -> Vec<(String, f64)> {
        vec![
            (FobosConfig::LNA_GAIN_ELEMENT.to_string(), f64::from(self.lna_gain)),
            (FobosConfig::VGA_GAIN_ELEMENT.to_string(), f64::from(self.vga_gain)),
        ]
    }

    /// A receiver that has been unplugged, or whose thread has died, is
    /// reported as needing a reopen so the engine reconnects on its own —
    /// which is what makes replugging one Just Work rather than needing
    /// Apply pressed.
    fn needs_reopen(&self) -> bool {
        !self.handle.is_alive() || self.handle.silent_for() >= SILENCE_BEFORE_REOPEN
    }

    /// Hand the receiver back before the engine opens its replacement.
    /// Without this, changing anything in Settings → Radio on a running
    /// Fobos fails with "may be held by another program" — the other
    /// program being us, and the device's own real reopen-timing quirk
    /// (`sdroxide_fobos::device::open`'s retry) then having to absorb it.
    fn release(&mut self) {
        self.handle.release();
    }

    /// Surface what an operator needs to know but cannot see.
    fn open_status(&self) -> Option<String> {
        let mut parts: Vec<String> = Vec::new();
        if self.port != FobosPort::Rf && self.port != FobosPort::HfDual {
            parts.push(format!(
                "direct sampling on {} — the tuner (and so LNA/VGA gain) is not in the path",
                self.port.name()
            ));
        }
        if self.port == FobosPort::HfDual {
            parts.push(
                "diversity is running on HF1+HF2 — the tuner (and so LNA/VGA gain) is not in \
                 the path. Watch the log for the depth it is reaching."
                    .to_string(),
            );
        }
        if parts.is_empty() { None } else { Some(parts.join(" — ")) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Rf`'s tuner has no near-DC clamp — a dial anywhere in the offered
    /// range lands exactly, so there is nothing for the engine to adopt.
    #[test]
    fn rf_never_reports_an_achieved_center() {
        for hz in [0.0, 100_000.0, 100_000_000.0, 1e9] {
            assert_eq!(achieved_center_hz(FobosPort::Rf, hz, 80_000_000.0, 8_000_000.0), None);
        }
    }

    /// The bug this exists to fix: a dial below the HF downconverter's own
    /// reachable floor (half the output rate) has to come back corrected, or
    /// the engine — and so the UI's own pan/zoom state — keeps believing the
    /// operator's original, unreachable request.
    #[test]
    fn hf_ports_report_the_clamped_center_when_the_dial_is_unreachable() {
        for port in [FobosPort::Hf1, FobosPort::Hf2, FobosPort::HfDual] {
            let achieved = achieved_center_hz(port, 820_000.0, 80_000_000.0, 2_500_000.0);
            assert_eq!(achieved, Some(1_250_000.0), "{port:?}");
        }
    }

    /// A dial already inside the reachable range is reported back unchanged
    /// — not `None` (this is a value, not "no clamp happened", so the
    /// distinction only matters at the call site's own `>= 0.5 Hz`
    /// difference check) but numerically identical to what was asked for.
    #[test]
    fn hf_ports_report_the_dial_unchanged_when_it_is_already_reachable() {
        let achieved = achieved_center_hz(FobosPort::Hf1, 5_000_000.0, 80_000_000.0, 2_500_000.0);
        assert_eq!(achieved, Some(5_000_000.0));
    }
}
