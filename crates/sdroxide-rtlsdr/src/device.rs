//! The whole dongle: demodulator plus tuner, and the ordering between them.
//!
//! The two halves are separately transcribed but they are not independent, and
//! this is where that shows. Changing the sample rate moves the tuner's IF
//! filter, which moves the intermediate frequency, which the demodulator's DDC
//! must be reprogrammed to, after which the centre must be re-applied.
//! `librtlsdr` spreads that chain across `rtlsdr_set_sample_rate` and its
//! `r820t_set_bw` callback; here it is one function so the order cannot drift.

use sdroxide_types::{RtlSdrAgc, RtlSdrConfig, RtlSdrHfMode};

use crate::error::{Error, Result};
use crate::regs;
use crate::rtl2832::{DirectSampling, Rtl2832};
use crate::tuner;
use crate::usb::UsbDev;
// The tuner driver itself lives in `sdroxide-r82xx`, shared with the RX-888
// backend — the same chip family, reached over a different bus.
use sdroxide_r82xx::{Chip, GainSetting, R82xx};

/// The bottom of the R82xx's own tuning range, and so where the automatic
/// crossover to direct sampling belongs: below this the tuner cannot lock at
/// all, and above it it is the better path — no alias, and a gain control.
///
/// Not the 28.8 MHz the Blog V4's upconverter is referenced to, which is what
/// this used to be. The two numbers are equal on a V4 and unrelated on
/// everything else, and only a dongle *without* an upconverter ever reaches the
/// automatic branch below — so using the V4's figure sent 12 m and the bottom of
/// 10 m through direct sampling, aliased, on hardware whose tuner covers them
/// properly.
pub const TUNER_MIN_HZ: f64 = 24_000_000.0;

/// Top of the R82xx's range.
pub const TUNER_MAX_HZ: f64 = 1_766_000_000.0;

/// The highest a direct-sampling dongle reaches: the ADC's own clock.
///
/// Half of that — 14.4 MHz — is the Nyquist limit, and it is tempting to stop
/// there. The band between it and the clock is receivable all the same, in the
/// ADC's second Nyquist zone: a signal at `f` above 14.4 MHz aliases to
/// `f - 28.8` (negative), and the DDC's frequency word is a 22-bit two's
/// complement fraction of the clock, so asking for `f` wraps to exactly that
/// negative alias. The wanted signal lands at DC the right way up — an offset
/// above the dial stays above it — so nothing downstream has to know.
///
/// What the operator has to know is that there is no filter in front of the
/// ADC: whatever is at `28.8 - f` is folded on top. 17 m lands on 10.7 MHz and
/// 15 m on 7.726 MHz, both quiet; 12 m would land on 3.885 MHz, which is not,
/// and is one more reason the crossover above hands 12 m to the tuner.
pub const DIRECT_SAMPLING_TOP_HZ: f64 = regs::RTL_XTAL_HZ;

/// Hysteresis above the crossover, so a dial parked next to it cannot
/// oscillate between direct sampling and the tuner on every nudge. Each switch
/// costs a tuner re-init and a gap in the stream, so this is worth having.
///
/// Entirely *above* [`TUNER_MIN_HZ`], never straddling it: a dial below the
/// tuner's floor has nowhere else to go, so the band the hysteresis buys can
/// only be spent on the far side.
const HF_HYSTERESIS_HZ: f64 = 500_000.0;

pub struct Device {
    rtl: Rtl2832,
    tuner: R82xx,
    /// Requested centre, as the operator set it.
    center: f64,
    hf_mode: RtlSdrHfMode,
    agc: RtlSdrAgc,
    gain_db: f64,
    /// The gain actually in effect after snapping, so the UI can settle on a
    /// value the hardware can produce.
    effective_gain_db: Option<f64>,
    label: String,
}

impl Device {
    /// Open, initialise and configure a dongle from persisted settings.
    pub fn open(cfg: &RtlSdrConfig, center_hz: f64) -> Result<Device> {
        let usb = UsbDev::open(cfg.serial.as_deref())?;
        let descriptor_v4 = usb.is_blog_v4();
        let usb_label = usb.label().to_string();

        let mut rtl = Rtl2832::new(usb);
        rtl.init_baseband()?;

        // Probe the tuner with the repeater open, then close it: the EEPROM
        // read below is on the demodulator's own bus, not the tuner's.
        rtl.i2c_repeater(true)?;
        let probed = tuner::probe(&rtl);
        let closed = rtl.i2c_repeater(false);
        let chip = probed?;
        closed?;

        // Settle what the descriptors could not. Every V4 is an R828D, so the
        // tuner has a veto: the upconverter offset and the 28.8 MHz reference
        // would wreck an R820T that merely claimed the name. A dongle that
        // already named itself needs no second opinion, which reserves the
        // EEPROM round trip for the case that needs rescuing — a V4 on a host
        // that did not hand us its USB strings.
        //
        // This is upstream's order too: `librtlsdr` probes the tuner, then
        // reads the EEPROM, then initialises, so the pause between the R828D's
        // reset pulse and its init is one a V4 already tolerates.
        let is_blog_v4 = chip == Chip::R828D && (descriptor_v4 || rtl.usb().reads_as_blog_v4());

        // Initialise the tuner, repeater open throughout.
        rtl.i2c_repeater(true)?;
        let tuner = tuner::open(&rtl, chip, is_blog_v4, cfg.ppm);
        rtl.i2c_repeater(false)?;
        let tuner = tuner?;

        // An R828D clocks from a different crystal, so the demodulator's own
        // reference is unaffected but the PLL's is — already handled inside
        // the tuner. What must happen here is the low-IF configuration.
        rtl.configure_for_r82xx()?;

        let label =
            format!("{usb_label} ({}{})", chip.name(), if is_blog_v4 { ", Blog V4" } else { "" });

        let mut dev = Device {
            rtl,
            tuner,
            center: center_hz,
            hf_mode: cfg.hf_mode,
            agc: cfg.agc,
            gain_db: cfg.tuner_gain_db,
            effective_gain_db: None,
            label,
        };

        // ppm first: everything downstream is computed against the corrected
        // crystals.
        dev.set_ppm(cfg.ppm)?;
        dev.set_sample_rate(cfg.sample_rate_hz)?;
        dev.set_center(center_hz)?;
        dev.set_agc(cfg.agc)?;
        if matches!(cfg.agc, RtlSdrAgc::Manual | RtlSdrAgc::Rtl) {
            dev.set_gain_db(cfg.tuner_gain_db)?;
        }
        dev.set_bias_tee(cfg.bias_tee)?;
        dev.rtl.reset_buffer()?;

        Ok(dev)
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn tuner_name(&self) -> &'static str {
        self.tuner.name()
    }

    pub fn is_blog_v4(&self) -> bool {
        self.tuner.is_blog_v4()
    }

    /// Whether this dongle can receive below the tuner's 24 MHz floor.
    ///
    /// A Blog V4 always can, through its upconverter. Anything else depends on
    /// the HF port being wired to the ADC's Q branch, which is a board
    /// property we cannot detect — so this reports what the *configuration*
    /// allows rather than a hardware fact.
    pub fn hf_capable(&self) -> bool {
        self.tuner.is_blog_v4() || self.hf_mode != RtlSdrHfMode::Off
    }

    pub fn sample_rate(&self) -> f64 {
        self.rtl.rate()
    }

    pub fn effective_gain_db(&self) -> Option<f64> {
        self.effective_gain_db
    }

    pub fn usb(&self) -> &UsbDev {
        self.rtl.usb()
    }

    pub fn reset_buffer(&self) -> Result<()> {
        self.rtl.reset_buffer()
    }

    /// Read and parse the configuration EEPROM. Diagnostic only — the USB
    /// descriptor strings carry the same information and cost nothing — but it
    /// is what `rtl_eeprom -d 0` prints, so it makes the two directly
    /// comparable during bring-up.
    pub fn read_eeprom(&self) -> Result<Option<crate::Eeprom>> {
        let raw = self.rtl.usb().read_eeprom(0, 256)?;
        Ok(crate::parse_eeprom(&raw))
    }

    /// Run `f` with the I2C repeater open, closing it even if `f` fails.
    ///
    /// Leaving the repeater open is not harmless: it keeps the tuner on the
    /// demodulator's bus, where the next non-tuner register access corrupts
    /// it.
    fn with_repeater<T>(&mut self, f: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        self.rtl.i2c_repeater(true)?;
        let out = f(self);
        let closed = self.rtl.i2c_repeater(false);
        let out = out?;
        closed?;
        Ok(out)
    }

    // ---- rate, frequency, gain -------------------------------------------

    /// Set the sample rate and return what the hardware actually produces.
    ///
    /// The full chain, in the order it has to happen: choose the resampler
    /// ratio, let the tuner pick an IF filter for that bandwidth (which moves
    /// its intermediate frequency), reprogram the DDC to the new IF, write the
    /// ratio, then re-apply the centre because the LO depends on the IF.
    pub fn set_sample_rate(&mut self, rate_hz: f64) -> Result<f64> {
        let requested = rate_hz.round() as u32;
        if !regs::rate_supported(requested) {
            return Err(Error::Unsupported(format!(
                "{:.0} Hz is outside the RTL2832U's resampler range — it can do \
                 225–300 kHz and 900 kHz–3.2 MHz, nothing between",
                rate_hz
            )));
        }

        // The tuner filter is chosen for the rate the hardware will really
        // run at, not the one that was asked for.
        let (_, achieved) = regs::resamp_ratio(regs::RTL_XTAL_HZ, requested);
        let int_freq = self.with_repeater(|d| Ok(d.tuner.set_bandwidth(&d.rtl, achieved)?))?;
        self.rtl.set_if_freq(int_freq)?;
        let achieved = self.rtl.set_sample_rate(rate_hz)?;

        // The IF moved, so the LO must move with it.
        let center = self.center;
        self.set_center(center)?;
        Ok(achieved)
    }

    /// Tune. Handles the switch between the tuner and direct sampling.
    pub fn set_center(&mut self, hz: f64) -> Result<()> {
        check_tunable_hz(hz)?;
        self.center = hz;
        self.apply_hf_path(hz)?;

        if self.rtl.direct_sampling() != DirectSampling::Off {
            // The ADC is the receiver; the DDC does the tuning.
            self.rtl.set_if_freq(hz)?;
        } else {
            self.with_repeater(|d| Ok(d.tuner.set_freq(&d.rtl, hz)?))?;
        }
        self.rtl.set_center(hz);
        Ok(())
    }

    /// Refuse a centre the tuner cannot reach, and say why.
    ///
    /// A frequency well above [`TUNER_MAX_HZ`] is not a mistyped number the
    /// operator can be blamed for: it is what happens when an **I.F. offset is
    /// stored as if it were a tune frequency**. QO-100's beacon-stabilisation
    /// plugin writes offsets near −9750 MHz into Settings → Radio, and a saved
    /// session carrying one was restored as the start frequency, so the dongle
    /// was asked to tune to 6 GHz and simply refused to open — with no message
    /// naming the frequency or the limit (issue #72).
    ///
    /// The PLL cannot reach it, so `set_freq` would fail with `PllUnlocked`,
    /// which reads as a fault in the hardware rather than a number that is
    /// wrong on its face. Saying it plainly costs one comparison and turns an
    /// unopenable tuner into a sentence the operator can act on.
    ///
    /// Only the tuner path is checked. In direct sampling the DDC does the
    /// tuning and the ADC covers 0–66 MHz, which is above [`TUNER_MIN_HZ`] and
    /// well below this ceiling, so the check can only ever be wrong in the
    /// tuner's favour.
    /// Decide whether this frequency needs the direct-sampling path, and
    /// switch if so.
    ///
    /// A Blog V4 never does — it upconverts in hardware, inside
    /// [`R82xx::set_freq`]. For everything else, below the crossover the tuner
    /// simply cannot reach, so the choice is direct sampling or nothing.
    fn apply_hf_path(&mut self, hz: f64) -> Result<()> {
        if self.tuner.is_blog_v4() {
            return Ok(());
        }
        self.set_direct_sampling(direct_sampling_for(hz, self.hf_mode, self.rtl.direct_sampling()))
    }

    fn set_direct_sampling(&mut self, mode: DirectSampling) -> Result<()> {
        if mode == self.rtl.direct_sampling() {
            return Ok(());
        }
        tracing::info!(
            "switching to {}",
            match mode {
                DirectSampling::Off => "the tuner",
                DirectSampling::I => "direct sampling (I branch)",
                DirectSampling::Q => "direct sampling (Q branch)",
            }
        );

        if mode != DirectSampling::Off {
            // Power the tuner down before bypassing it — a live LO leaks into
            // the ADC and puts a birdie in the middle of the passband.
            self.with_repeater(|d| Ok(d.tuner.standby(&d.rtl)?))?;
        }

        let reinit_tuner = self.rtl.set_direct_sampling(mode)?;
        if reinit_tuner {
            self.with_repeater(|d| Ok(d.tuner.init(&d.rtl)?))?;
            self.rtl.configure_for_r82xx()?;
            // Re-assert gain: a re-initialised tuner is back at its defaults.
            let agc = self.agc;
            self.set_agc(agc)?;
            if matches!(agc, RtlSdrAgc::Manual | RtlSdrAgc::Rtl) {
                let g = self.gain_db;
                self.set_gain_db(g)?;
            }
        }

        // Whatever just happened gapped the stream; drop what is stale.
        self.rtl.reset_buffer()?;
        Ok(())
    }

    /// Set the manual tuner gain, in dB. Returns the snapped value.
    pub fn set_gain_db(&mut self, db: f64) -> Result<Option<f64>> {
        self.gain_db = db;
        let eff = self.with_repeater(|d| {
            Ok(d.tuner.set_gain(&d.rtl, GainSetting::Manual { total_db: db })?)
        })?;
        self.effective_gain_db = eff;
        Ok(eff)
    }

    /// Select which automatic gain loops run.
    pub fn set_agc(&mut self, agc: RtlSdrAgc) -> Result<()> {
        self.agc = agc;
        self.rtl.set_rtl_agc(agc.rtl_auto())?;
        if agc.tuner_auto() {
            self.with_repeater(|d| Ok(d.tuner.set_gain(&d.rtl, GainSetting::Auto)?))?;
            self.effective_gain_db = None;
        } else {
            let g = self.gain_db;
            self.set_gain_db(g)?;
        }
        Ok(())
    }

    pub fn set_hf_mode(&mut self, mode: RtlSdrHfMode) -> Result<()> {
        if mode == self.hf_mode {
            return Ok(());
        }
        if self.tuner.is_blog_v4() && mode == RtlSdrHfMode::DirectQ {
            return Err(Error::Unsupported(
                "a Blog V4 upconverts HF in hardware and has no direct-sampling \
                 path — leave HF reception on Automatic"
                    .into(),
            ));
        }
        self.hf_mode = mode;
        let center = self.center;
        self.set_center(center)
    }

    /// Change the crystal correction. Touches all three places ppm reaches.
    pub fn set_ppm(&mut self, ppm: i32) -> Result<()> {
        let changed = self.rtl.set_ppm(ppm)?;
        if !changed {
            return Ok(());
        }
        self.tuner.set_ppm(ppm);
        // The resampler's ratio is unaffected (it uses the nominal crystal and
        // a separate offset register), but the LO must be recomputed.
        let center = self.center;
        self.set_center(center)
    }

    pub fn set_bias_tee(&mut self, on: bool) -> Result<()> {
        self.rtl.set_bias_tee(on)
    }

    /// Leave the hardware safe to walk away from.
    pub fn shutdown(&mut self) {
        self.rtl.shutdown();
    }

    /// The gain values this dongle can produce, in dB.
    pub fn gains_db(&self) -> Vec<f64> {
        R82xx::gains_db().collect()
    }
}

/// Which ADC path a dial setting calls for, given the operator's HF mode and
/// where the front end is now.
///
/// Pure, and separate from [`Device::apply_hf_path`] that applies it, because
/// the crossover is a decision with edges worth testing and no dongle is needed
/// to make it. Never asked of a Blog V4: that one upconverts inside its own
/// tuning call, and the caller returns before reaching here.
fn direct_sampling_for(hz: f64, mode: RtlSdrHfMode, current: DirectSampling) -> DirectSampling {
    match mode {
        RtlSdrHfMode::Off => DirectSampling::Off,
        RtlSdrHfMode::DirectQ => DirectSampling::Q,
        RtlSdrHfMode::Auto => {
            // Hysteresis, spent above the floor only — see HF_HYSTERESIS_HZ.
            let threshold = if current == DirectSampling::Off {
                TUNER_MIN_HZ
            } else {
                TUNER_MIN_HZ + HF_HYSTERESIS_HZ
            };
            if hz < threshold { DirectSampling::Q } else { DirectSampling::Off }
        }
    }
}

/// Refuse a centre the tuner cannot reach, and say why.
///
/// A frequency well above [`TUNER_MAX_HZ`] is not a mistyped number the operator
/// can be blamed for: it is what happens when an **I.F. offset is stored as if it
/// were a tune frequency**. QO-100's beacon-stabilisation plugin writes offsets
/// near −9750 MHz into Settings → Radio, and a saved session carrying one was
/// restored as the start frequency, so the dongle was asked to tune to 6 GHz and
/// simply refused to open — with no message naming the frequency or the limit
/// (issue #72).
///
/// The PLL cannot reach it, so `set_freq` would fail with `PllUnlocked`, which
/// reads as a fault in the hardware rather than a number that is wrong on its
/// face. Saying it plainly costs one comparison and turns an unopenable tuner
/// into a sentence the operator can act on.
///
/// Only the tuner path is checked. In direct sampling the DDC does the tuning and
/// the ADC covers 0–66 MHz, which is above [`TUNER_MIN_HZ`] and well below this
/// ceiling, so the check can only ever be wrong in the tuner's favour.
///
/// A non-finite centre passes: it is a corrupt session rather than an unreachable
/// frequency, and saying "out of range" about a NaN would be a lie.
pub(crate) fn check_tunable_hz(hz: f64) -> Result<()> {
    if hz.is_finite() && hz > TUNER_MAX_HZ {
        return Err(Error::OutOfRange(format!(
            "{} MHz is above this RTL-SDR's tuner range, which ends at {:.0} MHz. \
             An I.F. offset saved as a start frequency lands here — clear the start \
             frequency and tune the band you want instead.",
            hz / 1e6,
            TUNER_MAX_HZ / 1e6
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reporter's case (issue #72): a QO-100 I.F. offset near −9750 MHz was
    /// saved as the start frequency, so the dongle was asked to tune to 6 GHz.
    /// `check_tunable_hz` refuses it with a sentence naming the limit, rather than
    /// letting the PLL fail with a message that reads as a hardware fault.
    #[test]
    fn a_start_frequency_above_the_tuner_range_is_refused_by_name() {
        // The limit is the only thing the check knows, so this exercises the
        // boundary rather than a magic number.
        let over = TUNER_MAX_HZ * 3.4; // ~6 GHz, as reported
        let err = check_tunable_hz(over).expect_err("6 GHz must be refused");
        let msg = err.to_string();
        assert!(
            msg.contains("above") && msg.contains("I.F. offset"),
            "the sentence must name the limit and the cause: {msg}"
        );
        assert!(
            msg.contains(&format!("{:.0}", TUNER_MAX_HZ / 1e6)),
            "the sentence must name the tuner's own ceiling in MHz: {msg}"
        );
    }

    /// The check must not get in the way of the frequencies this driver exists
    /// to work: everything up to and including the tuner's ceiling passes, and
    /// so does the boundary itself.
    #[test]
    fn frequencies_the_tuner_can_reach_are_untouched() {
        for hz in [0.0, 1.0, TUNER_MIN_HZ, 28.0e6, 100.0e6, 1_090_000_000.0, TUNER_MAX_HZ] {
            assert!(
                check_tunable_hz(hz).is_ok(),
                "{hz} Hz is within the tuner's reach and must pass"
            );
        }
    }

    /// Just over the ceiling is refused, and exactly at it is not — the boundary
    /// is where an off-by-one would hide.
    #[test]
    fn the_ceiling_itself_is_inclusive() {
        assert!(check_tunable_hz(TUNER_MAX_HZ).is_ok());
        assert!(check_tunable_hz(TUNER_MAX_HZ + 1.0).is_err());
    }

    /// `set_center` must consult the check, not merely sit beside it.
    ///
    /// `check_tunable_hz` is a free function and its own tests pass whatever
    /// `set_center` does — which is exactly how a guard can exist, be tested,
    /// and still never run. `set_center` needs real hardware to call, so this
    /// asserts the call is *there* in the source: a test that reads its own
    /// crate is not a substitute for a unit test, but a refactor that unhooks
    /// the guard is invisible otherwise, and the cost of the fault is an
    /// unopenable dongle.
    #[test]
    fn set_center_consults_the_range_check_before_touching_the_hardware() {
        let src = include_str!("device.rs");
        let body = src
            .split("pub fn set_center(&mut self, hz: f64) -> Result<()> {")
            .nth(1)
            .expect("set_center exists");
        let first_stmt = body.lines().find(|l| !l.trim().is_empty()).unwrap_or_default();
        assert!(
            first_stmt.contains("check_tunable_hz"),
            "set_center must check the range before it programs the tuner; it starts with \
             `{first_stmt}`"
        );
    }

    /// A non-finite centre is the session's problem, not the tuner's, and must
    /// not be reported as an unreachable frequency.
    #[test]
    fn a_non_finite_centre_is_not_claimed_to_be_out_of_range() {
        assert!(check_tunable_hz(f64::NAN).is_ok());
        assert!(check_tunable_hz(f64::INFINITY).is_ok());
    }

    /// The bands this exists for. 17 m and 15 m are below the tuner's floor and
    /// above the ADC's Nyquist limit, so direct sampling's second zone is the
    /// only way to them; 12 m and 10 m are above the floor, where the tuner is
    /// the better path.
    #[test]
    fn the_crossover_is_the_tuners_floor_not_the_v4s_reference() {
        let auto = |hz| direct_sampling_for(hz, RtlSdrHfMode::Auto, DirectSampling::Off);
        assert_eq!(auto(18_100_000.0), DirectSampling::Q, "17 m");
        assert_eq!(auto(21_074_000.0), DirectSampling::Q, "15 m");
        assert_eq!(auto(24_915_000.0), DirectSampling::Off, "12 m is the tuner's");
        assert_eq!(auto(28_074_000.0), DirectSampling::Off, "10 m is the tuner's");
    }

    /// The hysteresis holds a dial sitting on the crossover, and holds it on
    /// the upper side: below the tuner's floor there is no choice to make.
    #[test]
    fn the_hysteresis_sits_above_the_floor() {
        let from_tuner = |hz| direct_sampling_for(hz, RtlSdrHfMode::Auto, DirectSampling::Off);
        let from_direct = |hz| direct_sampling_for(hz, RtlSdrHfMode::Auto, DirectSampling::Q);

        // On the tuner, nothing it can still reach sends us to the ADC.
        assert_eq!(from_tuner(TUNER_MIN_HZ), DirectSampling::Off);
        assert_eq!(from_tuner(TUNER_MIN_HZ - 1.0), DirectSampling::Q);
        // Direct sampling, on the way back up: hold through the band, then go.
        assert_eq!(from_direct(TUNER_MIN_HZ + 100_000.0), DirectSampling::Q);
        assert_eq!(from_direct(TUNER_MIN_HZ + HF_HYSTERESIS_HZ), DirectSampling::Off);
    }

    /// Both explicit settings mean what they say, everywhere.
    #[test]
    fn an_explicit_hf_mode_is_never_second_guessed() {
        for hz in [3_573_000.0, 21_074_000.0, 145_500_000.0] {
            assert_eq!(
                direct_sampling_for(hz, RtlSdrHfMode::Off, DirectSampling::Q),
                DirectSampling::Off
            );
            assert_eq!(
                direct_sampling_for(hz, RtlSdrHfMode::DirectQ, DirectSampling::Off),
                DirectSampling::Q
            );
        }
    }
}
