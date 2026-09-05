//! Bringing a programmed receiver up: sample clock, front-end GPIO, and the two
//! analogue gain stages.
//!
//! Everything here runs on the thread that owns the [`UsbDev`] — see the
//! invariant in [`crate::usb`].

use std::time::Duration;

use nusb::transfer::TransferError;

use sdroxide_r82xx::{Chip, GainSetting, R82xx};

use crate::band::{self, Band, BandState};
use crate::error::{Error, Result};
use crate::protocol::{Cmd, Identity, STATS_LEN, Stats, Version, arg, gpio};
use crate::si5351;
use crate::usb::{self, UsbDev};

/// Default ADC clock. 64.8 Msps covers 0–32.4 MHz, needs 129.6 MB/s, and leaves
/// enough SuperSpeed headroom to be reliable on a shared bus. 129.6 Msps is
/// offered but is not the default, for the same reason the UI labels the
/// RTL-SDR's 3.2 Msps as the rate that drops samples.
pub const DEFAULT_ADC_HZ: f64 = 64_800_000.0;

/// Sample rates offered in the UI, one list shared with the wasm-safe settings
/// panel. The Si5351 synthesises nearly anything in range — the firmware takes
/// the clock as a plain integer of hertz, and the UI has a free-entry field
/// for exactly that — so this is the set worth a click, not a limit.
pub const ADC_RATES: &[f64] = &sdroxide_types::Rx888Config::ADC_RATES;

/// The LTC2208's specified ceiling.
const MAX_ADC_HZ: f64 = 130_000_000.0;
/// Below this the Si5351 and the ADC both misbehave, and the coverage is not
/// worth having anyway.
const MIN_ADC_HZ: f64 = 4_000_000.0;

/// Gain element names, shared with the settings UI.
pub const ATT_ELEMENT: &str = "ATT";
pub const VGA_ELEMENT: &str = "VGA";

/// The AD8370 code the VHF path pins the VGA to: high range, code 3, about
/// +1.3 dB. Upstream's figure, written as a register value rather than a gain
/// because the dB-based setter would snap it to the nearest of two ranges.
const VHF_VGA_REG: u8 = 0x80 | 3;

/// PE4312 step attenuator: 0..=63 in 0.5 dB steps.
const ATT_MAX_CODE: u8 = 63;
pub const ATT_MIN_DB: f64 = -31.5;
pub const ATT_MAX_DB: f64 = 0.0;
pub const ATT_STEP_DB: f64 = 0.5;

/// How the receiver should be set up.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub serial: Option<String>,
    pub adc_rate_hz: f64,
    /// LTC2208 dither: costs a little noise floor, buys spurious-free dynamic
    /// range. Off by default, matching the upstream host software.
    pub dither: bool,
    /// LTC2208 output randomizer. On by default — it keeps the digital bus from
    /// radiating into the front end, and undoing it costs one XOR per sample.
    pub randomize: bool,
    /// Power the HF antenna port. Off by default: switching phantom power onto
    /// someone's antenna without being asked is not a good default.
    pub bias_tee_hf: bool,
    /// Select the ADC's wider 2.25 Vp-p input range. Despite the GPIO bit's
    /// name this is a range select, not a preamp — see [`gpio::PGA_EN`]. On by
    /// default for the headroom.
    pub pga: bool,
    /// Attenuation as a gain, i.e. -31.5..=0 dB.
    pub attenuator_db: f64,
    /// AD8370 VGA gain in dB.
    pub vga_db: f64,
    /// Reference trim, parts per million.
    pub ppm: f64,
    /// Power the VHF antenna port.
    pub bias_tee_vhf: bool,
    /// R828D RF gain in dB, used above the automatic crossover.
    pub tuner_gain_db: f64,
    /// Let the tuner run its own LNA and mixer loops instead of the fixed
    /// ladder.
    pub tuner_agc: bool,
    /// Bins the downconverter keeps of its 8192-bin analysis, which sets the
    /// complex output rate — and so the panadapter width — to
    /// `adc_rate · bins / 8192`. Snapped by [`crate::stream::sanitize_ddc_bins`].
    pub ddc_bins: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            serial: None,
            adc_rate_hz: DEFAULT_ADC_HZ,
            dither: false,
            randomize: true,
            bias_tee_hf: false,
            pga: true,
            attenuator_db: 0.0,
            vga_db: 12.0,
            ppm: 0.0,
            bias_tee_vhf: false,
            tuner_gain_db: 30.0,
            tuner_agc: false,
            ddc_bins: crate::stream::DEFAULT_DDC_BINS,
        }
    }
}

/// An opened, configured receiver.
pub struct Device {
    usb: UsbDev,
    /// The authoritative GPIO word. The firmware keeps no shadow of its own and
    /// applies whatever it is sent, so every bit must be re-stated on every
    /// write — which means exactly one place may own this value.
    gpio_word: u32,
    adc_rate_hz: f64,
    /// Bins the downconverter keeps, already sanitised. The device itself never
    /// touches them — they live here because the band plan does: how far the
    /// downconverter may slide, and whether VHF is possible at all, both depend
    /// on the output width.
    ddc_bins: usize,
    randomize: bool,
    identity: Option<Identity>,
    /// Set when the link is too slow for the requested rate, or when the
    /// firmware never initialised the front end, for `IqSource::open_status`.
    warning: Option<String>,
    streaming: bool,

    // ---- the VHF front end ----------------------------------------------
    /// Built on first entry into VHF and kept afterwards: `standby` already
    /// returns the part to a known state, so there is nothing to rebuild.
    tuner: Option<R82xx>,
    band: Band,
    /// The RF the tuner is parked on. Meaningless in HF.
    lo_dial_hz: f64,
    /// Whether the firmware found an R828D at boot *and* the ADC clock leaves
    /// room for its IF.
    vhf_capable: bool,
    /// Latched after a failed entry, so a dial spin does not retry a broken
    /// tuner on every step.
    vhf_failed: Option<String>,
    /// What the operator asked for on the HF gain controls.
    ///
    /// In VHF the hardware is forced elsewhere — the attenuator to maximum and
    /// the VGA to a fixed code — so these are what gets restored on the way
    /// back. Without them the slider would be able to undo the front end's own
    /// settings and quietly put the whole broadcast band back on top of the IF.
    requested_att_db: f64,
    requested_vga_db: f64,
    tuner_gain_db: f64,
    tuner_agc: bool,
    ppm: f64,
}

/// What a retune means downstream.
///
/// The band machine runs on the thread that owns the USB device; the converter
/// is a different thread and cannot ask, so it is told.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tune {
    pub band: Band,
    pub lo_dial_hz: f64,
    /// Where the wideband downconverter must be centred, in ADC baseband Hz.
    pub ddc_center_hz: f64,
    /// True in VHF: the tuner's high-side LO mirrors the spectrum, and the
    /// converter undoes it by negating Q. Measured on the bench — see
    /// [`crate::band`].
    pub conjugate: bool,
}

impl Device {
    /// Open, upload firmware if needed, and apply `settings`.
    pub fn open(settings: &Settings, firmware_override: Option<&[u8]>) -> Result<Device> {
        let mut usb = usb::open(settings.serial.as_deref(), firmware_override)?;
        let mut identity = identify(&usb);

        // A receiver the firmware failed to *identify* streams perfectly and
        // ignores every front-end switch — the bias tee included, which is the
        // way this usually shows up. The detection runs once, at boot, so the
        // one thing worth trying is booting it again.
        //
        // Only a failed detection, mind. A board the firmware named and this
        // driver does not steer — an RX-888 Mk1, which has neither the Mk2's
        // attenuator nor its VGA — is not a fault and reloading the firmware
        // would find the same answer a second time, having told its owner to go
        // and unplug a receiver that is working (issue #342).
        if identity.is_some_and(|i| !i.recognised()) {
            tracing::warn!(
                "RX-888: the firmware did not recognise this receiver (hardware id 0x{:02x}) — \
                 its front-end GPIO was never initialised. Reloading the firmware to run the \
                 detection again.",
                identity.map(|i| i.hardware).unwrap_or(0),
            );
            usb = usb::reload_firmware(usb, firmware_override)?;
            identity = identify(&usb);
        }

        let (adc_rate_hz, link_warning) = clamp_rate_to_link(settings.adc_rate_hz, &usb);
        let warning = join_warnings(link_warning, front_end_warning(identity));
        let ddc_bins = crate::stream::sanitize_ddc_bins(settings.ddc_bins);

        // The firmware sets hardware id 0x04 only when its boot-time I2C probe
        // found an R828D, so it already answers "is there a tuner" — but a
        // clock too slow for the tuner's IF rules VHF out just as firmly. The
        // output width does not: an output too wide to centre on the IF still
        // contains it, and the source reports the off-centre truth.
        let vhf_capable = band::vhf_available(
            adc_rate_hz,
            identity.is_some_and(|i| i.front_end_ready()) && tuner_present(&usb),
        );

        let mut dev = Device {
            usb,
            gpio_word: 0,
            adc_rate_hz,
            ddc_bins,
            randomize: settings.randomize,
            identity,
            warning,
            streaming: false,
            tuner: None,
            band: Band::Hf,
            lo_dial_hz: 0.0,
            vhf_capable,
            vhf_failed: None,
            requested_att_db: settings.attenuator_db,
            requested_vga_db: settings.vga_db,
            tuner_gain_db: settings.tuner_gain_db,
            tuner_agc: settings.tuner_agc,
            ppm: settings.ppm,
        };

        dev.apply_gpio(settings)?;
        dev.set_attenuator_db(settings.attenuator_db)?;
        dev.set_vga_db(settings.vga_db)?;
        dev.set_adc_rate(adc_rate_hz, settings.ppm)?;
        Ok(dev)
    }

    pub fn label(&self) -> &str {
        self.usb.label()
    }

    pub fn serial(&self) -> Option<&str> {
        self.usb.serial()
    }

    pub fn version(&self) -> Option<Version> {
        self.identity.map(|i| i.version)
    }

    /// What the firmware says it is running on, or `None` if it did not answer.
    pub fn identity(&self) -> Option<Identity> {
        self.identity
    }

    /// The ADC clock, which is also the real-sample rate.
    pub fn adc_rate_hz(&self) -> f64 {
        self.adc_rate_hz
    }

    /// Bins the downconverter keeps, already sanitised.
    pub fn ddc_bins(&self) -> usize {
        self.ddc_bins
    }

    /// The downconverter's complex output rate for this clock and bin count.
    pub fn out_rate_hz(&self) -> f64 {
        self.adc_rate_hz * self.ddc_bins as f64 / crate::stream::DDC_BLOCK as f64
    }

    /// Whether the ADC is scrambling its output, so the converter knows whether
    /// to undo it.
    pub fn randomized(&self) -> bool {
        self.randomize
    }

    pub fn warning(&self) -> Option<&str> {
        self.warning.as_deref()
    }

    pub fn usb(&self) -> &UsbDev {
        &self.usb
    }

    /// Program the Si5351. `ppm` trims the reference.
    pub fn set_adc_rate(&mut self, hz: f64, ppm: f64) -> Result<()> {
        let hz = hz.clamp(MIN_ADC_HZ, MAX_ADC_HZ);
        let trimmed = hz * (1.0 + ppm * 1e-6);
        let word = trimmed.round() as u32;
        tracing::info!("RX-888: ADC clock {:.6} MHz", f64::from(word) / 1e6);
        self.usb.vendor_out(Cmd::StartAdc, 0, 0, &word.to_le_bytes())?;
        self.adc_rate_hz = trimmed;
        Ok(())
    }

    /// Rebuild and send the whole GPIO word from `settings`.
    pub fn apply_gpio(&mut self, settings: &Settings) -> Result<()> {
        let mut w = 0u32;
        // SHDWN stays clear: setting it powers the front end down.
        if settings.dither {
            w |= gpio::DITH;
        }
        if settings.randomize {
            w |= gpio::RANDO;
        }
        if settings.bias_tee_hf {
            w |= gpio::BIAS_HF;
        }
        if settings.pga {
            w |= gpio::PGA_EN;
        }
        if settings.bias_tee_vhf {
            w |= gpio::BIAS_VHF;
        }
        // The RF switch follows the band, never the settings: the band machine
        // owns that bit and re-states it here like every other one, because the
        // firmware keeps no shadow of its own.
        if self.band == Band::Vhf {
            w |= gpio::VHF_EN;
        }
        self.randomize = settings.randomize;
        self.write_gpio(w)
    }

    /// Turn the front-panel LED on or off, leaving every other bit alone.
    pub fn set_led(&mut self, on: bool) -> Result<()> {
        let w = if on { self.gpio_word | gpio::LED_BLUE } else { self.gpio_word & !gpio::LED_BLUE };
        self.write_gpio(w)
    }

    pub fn set_bias_tee(&mut self, on: bool) -> Result<()> {
        let w = if on { self.gpio_word | gpio::BIAS_HF } else { self.gpio_word & !gpio::BIAS_HF };
        self.write_gpio(w)
    }

    pub fn set_dither(&mut self, on: bool) -> Result<()> {
        let w = if on { self.gpio_word | gpio::DITH } else { self.gpio_word & !gpio::DITH };
        self.write_gpio(w)
    }

    /// Changing randomization mid-stream would desynchronise the converter from
    /// the ADC, so this reports the new state for the caller to act on.
    pub fn set_randomize(&mut self, on: bool) -> Result<()> {
        let w = if on { self.gpio_word | gpio::RANDO } else { self.gpio_word & !gpio::RANDO };
        self.write_gpio(w)?;
        self.randomize = on;
        Ok(())
    }

    pub fn set_pga(&mut self, on: bool) -> Result<()> {
        let w = if on { self.gpio_word | gpio::PGA_EN } else { self.gpio_word & !gpio::PGA_EN };
        self.write_gpio(w)
    }

    /// Switch the ADC's input between the HF chain and the VHF tuner's IF.
    ///
    /// The two front ends share one ADC, so this is an either/or: with the
    /// tuner in circuit the HF chain must be attenuated out of the way, or the
    /// whole of the broadcast band lands on top of the IF.
    pub fn set_vhf_en(&mut self, on: bool) -> Result<()> {
        let w = if on { self.gpio_word | gpio::VHF_EN } else { self.gpio_word & !gpio::VHF_EN };
        self.write_gpio(w)
    }

    /// Power the VHF antenna port.
    pub fn set_bias_tee_vhf(&mut self, on: bool) -> Result<()> {
        let w = if on { self.gpio_word | gpio::BIAS_VHF } else { self.gpio_word & !gpio::BIAS_VHF };
        self.write_gpio(w)
    }

    /// Set the AD8370 by raw register value.
    ///
    /// The dB-based [`Device::set_vga_db`] snaps to whichever of the part's two
    /// ranges lands nearer, which is right for an operator's slider and wrong
    /// for reproducing a specific value: the VHF path wants exactly the code
    /// upstream uses, not the nearest gain to it.
    pub fn set_vga_reg(&mut self, reg: u8) -> Result<()> {
        self.set_arg(arg::AD8370_VGA, reg)?;
        tracing::debug!("RX-888: VGA register 0x{reg:02x} ({:+.1} dB)", ad8370_db_for(reg));
        Ok(())
    }

    fn write_gpio(&mut self, word: u32) -> Result<()> {
        tracing::debug!("RX-888: GPIO <- 0x{word:08x}");
        self.usb.vendor_out(Cmd::GpioFx3, 0, 0, &word.to_le_bytes())?;
        self.gpio_word = word;
        Ok(())
    }

    /// The live GPIO word as this driver believes it to be.
    pub fn gpio_word(&self) -> u32 {
        self.gpio_word
    }

    /// Set the step attenuator. `db` is a gain, so -31.5..=0.
    ///
    /// Returns the attenuation actually programmed, which is `db` rounded to the
    /// hardware's 0.5 dB grid.
    /// Set the step attenuator. `db` is a gain, so -31.5..=0.
    ///
    /// On VHF the request is remembered and applied on the way back to HF
    /// rather than programmed now: the HF chain is deliberately held at maximum
    /// attenuation there, because both front ends share one ADC and letting it
    /// back in puts the whole broadcast band on top of the tuner's IF.
    pub fn set_attenuator_db(&mut self, db: f64) -> Result<f64> {
        self.requested_att_db = db;
        if self.band == Band::Vhf {
            return Ok(att_db_for(att_code_for(db)));
        }
        self.program_att(db)
    }

    /// Program the attenuator, whatever the band. Used by the band machine.
    fn program_att(&mut self, db: f64) -> Result<f64> {
        let code = att_code_for(db);
        self.set_arg(arg::DAT31_ATT, code)?;
        let achieved = att_db_for(code);
        tracing::debug!("RX-888: attenuator {achieved:+.1} dB (code {code})");
        Ok(achieved)
    }

    /// Set the AD8370 VGA. Returns the gain actually programmed.
    ///
    /// Held on VHF for the same reason as the attenuator — the VGA is in the
    /// tuner's path too, and this stage is pinned near unity there.
    pub fn set_vga_db(&mut self, db: f64) -> Result<f64> {
        self.requested_vga_db = db;
        let code = ad8370_code_for(db);
        let achieved = ad8370_db_for(code);
        if self.band == Band::Vhf {
            return Ok(achieved);
        }
        self.set_arg(arg::AD8370_VGA, code)?;
        tracing::debug!("RX-888: VGA {achieved:+.1} dB (code 0x{code:02x})");
        Ok(achieved)
    }

    /// Send one `SETARGFX3` parameter.
    ///
    /// # The value travels in `wValue`, not in the data phase
    ///
    /// The published API reference describes this command as "OUT, n bytes",
    /// which reads as though the parameter is the payload. It is not: the
    /// firmware's handler is `rx888r2_SetGain(wValue)`, and the payload it reads
    /// with `CyU3PUsbGetEP0Data` is discarded. Putting the value in the payload
    /// therefore sets the parameter to **zero** — and since code 0 mutes the
    /// AD8370 outright, the symptom is a receiver that streams perfectly and
    /// hears nothing, at every gain setting, with no error anywhere.
    ///
    /// A one-byte payload is still sent so the data phase the firmware expects
    /// is well formed.
    fn set_arg(&self, index: u16, value: u8) -> Result<()> {
        self.usb.vendor_out(Cmd::SetArgFx3, u16::from(value), index, &[value])
    }

    /// Start the GPIF engine. Samples begin arriving on the bulk endpoint.
    ///
    /// # Why the status stage is allowed to go missing
    ///
    /// `STARTFX3` does not poke a register — it rebuilds the FX3's DMA channels
    /// and restarts the GPIF state machine, and firmware 2.3 does not answer the
    /// control transfer while that is happening. On the hardware this was
    /// developed against the request never completes at all: nusb gives up after
    /// its timeout and reports `Cancelled` (usbfs maps `ETIMEDOUT` onto it), and
    /// yet the receiver is streaming perfectly by the time the call returns.
    ///
    /// Treating that as a failure means refusing to stream from a device that is
    /// already streaming, so a timeout here is logged and accepted. Whether the
    /// command worked is decided by the only thing that actually settles it —
    /// samples arriving on the bulk endpoint.
    pub fn start(&mut self) -> Result<()> {
        // Deliberately short. Since the status stage is never coming, every
        // millisecond here is dead air before the first transfer is submitted —
        // and it is charged to the operator as a slow start, or, if they happen
        // to be measuring throughput over a short window, as lost samples. Two
        // seconds of it made a perfectly healthy receiver look like it was
        // running at 67 % of its sample rate.
        const START_TIMEOUT: Duration = Duration::from_millis(150);
        match self.usb.vendor_out_timeout(Cmd::StartFx3, 0, 0, &[], START_TIMEOUT) {
            Ok(()) => {}
            Err(Error::Transfer { source: TransferError::Cancelled, .. }) => {
                tracing::debug!(
                    "RX-888: STARTFX3 returned no status stage (expected on firmware 2.3)"
                );
            }
            Err(e) => return Err(e),
        }
        self.streaming = true;
        let _ = self.set_led(true);
        Ok(())
    }

    /// Halt the GPIF engine.
    pub fn stop(&mut self) -> Result<()> {
        let _ = self.set_led(false);
        self.usb.vendor_out(Cmd::StopFx3, 0, 0, &[])?;
        self.streaming = false;
        Ok(())
    }

    /// Read the firmware's health counters.
    pub fn stats(&self) -> Result<Stats> {
        let b = self.usb.vendor_in(Cmd::GetStats, 0, 0, STATS_LEN)?;
        Stats::parse(&b).ok_or_else(|| {
            Error::Unsupported(format!("GETSTATS returned {} bytes, too short to decode", b.len()))
        })
    }

    /// Best-effort quiesce, for the drop path.
    pub fn shutdown(&mut self) {
        if self.streaming {
            let _ = self.stop();
        }
        // Leave the front end where an HF user expects to find it, and the
        // tuner asleep rather than injecting its LO into a shared ADC.
        if self.band == Band::Vhf {
            let _ = self.leave_vhf();
        }
        let _ = self.set_led(false);
    }

    // ---- the VHF front end ------------------------------------------------

    /// Whether this receiver can use its VHF front end at all.
    pub fn vhf_capable(&self) -> bool {
        self.vhf_capable && self.vhf_failed.is_none()
    }

    pub fn band(&self) -> Band {
        self.band
    }

    /// Why VHF is unavailable, if it was tried and failed.
    pub fn vhf_failure(&self) -> Option<&str> {
        self.vhf_failed.as_deref()
    }

    /// Tune, switching the front end if the dial has crossed the crossover.
    ///
    /// Returns what the converter thread has to do about it. Everything here
    /// runs on the thread that owns the [`UsbDev`], which is why the converter
    /// is told rather than asked.
    pub fn set_center_hz(&mut self, dial_hz: f64) -> Result<Tune> {
        let cur = BandState { band: self.band, lo_dial_hz: self.lo_dial_hz };
        let p = band::plan(dial_hz, self.adc_rate_hz, self.out_rate_hz(), self.vhf_capable(), cur);

        match (self.band, p.band) {
            (Band::Hf, Band::Vhf) => {
                if let Err(e) = self.enter_vhf(p.lo_dial_hz) {
                    tracing::warn!("RX-888: cannot use the VHF front end: {e}");
                    let _ = self.leave_vhf();
                    self.vhf_failed = Some(e.to_string());
                    // Fall back to HF rather than sit deaf. The dial is above
                    // Nyquist so what comes back is an alias — wrong, but
                    // honest, and it keeps the receiver answering.
                    return Ok(Tune {
                        band: Band::Hf,
                        lo_dial_hz: 0.0,
                        ddc_center_hz: dial_hz,
                        conjugate: false,
                    });
                }
            }
            (Band::Vhf, Band::Hf) => self.leave_vhf()?,
            (Band::Vhf, Band::Vhf) if p.move_lo => {
                if let Some(t) = self.tuner.as_mut() {
                    t.set_freq(&self.usb, p.lo_dial_hz)?;
                }
                self.lo_dial_hz = p.lo_dial_hz;
            }
            _ => {}
        }

        Ok(Tune {
            band: p.band,
            lo_dial_hz: p.lo_dial_hz,
            ddc_center_hz: p.ddc_center_hz,
            conjugate: p.conjugate,
        })
    }

    /// Whether tuning to `dial_hz` would reprogram the tuner's PLL.
    ///
    /// Pure, so the stream thread can rate-limit the expensive case without
    /// paying for it first — a PLL write sleeps for lock on the same thread
    /// that services the bulk endpoint.
    pub fn hardware_retune_needed(&self, dial_hz: f64) -> bool {
        let cur = BandState { band: self.band, lo_dial_hz: self.lo_dial_hz };
        let p = band::plan(dial_hz, self.adc_rate_hz, self.out_rate_hz(), self.vhf_capable(), cur);
        p.move_lo || p.band != self.band
    }

    /// Bring the tuner up and put the ADC on its IF.
    ///
    /// The order is upstream's and none of it is arbitrary: the HF chain has to
    /// be attenuated away before the switch throws, because both front ends
    /// share one ADC; and CLK2 has to be running before the tuner is spoken to,
    /// because initialisation calibrates the IF filter against a PLL that has
    /// nothing to lock to without it.
    fn enter_vhf(&mut self, lo_dial_hz: f64) -> Result<()> {
        self.program_att(ATT_MIN_DB)?;
        self.set_vhf_en(true)?;
        // Upstream pins the VGA near unity here and lets the R828D's own ladder
        // provide the gain.
        self.set_vga_reg(VHF_VGA_REG)?;
        si5351::set_clk2_hz(&self.usb, si5351::TUNER_REF_HZ)?;

        let mut t = R82xx::new(Chip::R828D, false, self.ppm.round() as i32);
        t.init(&self.usb)?;
        let int_freq = t.set_bandwidth(&self.usb, band::IF_BW_HZ)?;
        debug_assert_eq!(int_freq, band::IF_CENTER_HZ);
        t.set_freq(&self.usb, lo_dial_hz)?;
        self.tuner = Some(t);

        self.band = Band::Vhf;
        self.lo_dial_hz = lo_dial_hz;
        self.apply_tuner_gain()?;
        tracing::info!("RX-888: VHF front end on, tuner parked at {:.4} MHz", lo_dial_hz / 1e6);
        Ok(())
    }

    /// Put the tuner away and give the ADC back to the HF chain.
    ///
    /// Best-effort on the tuner and its clock: a receiver on its way back to HF
    /// must not be left with `VHF_EN` set because the tuner stopped answering.
    fn leave_vhf(&mut self) -> Result<()> {
        if let Some(t) = self.tuner.as_mut() {
            let _ = t.standby(&self.usb);
        }
        let _ = si5351::clk2_off(&self.usb);
        self.set_vhf_en(false)?;
        self.band = Band::Hf;
        self.lo_dial_hz = 0.0;
        // Restore what the operator actually asked for.
        let vga = self.requested_vga_db;
        let att = self.requested_att_db;
        self.set_vga_reg(ad8370_code_for(vga))?;
        self.program_att(att)?;
        tracing::info!("RX-888: HF front end restored");
        Ok(())
    }

    /// R828D RF gain, in dB. Ignored while the receiver is on HF.
    pub fn set_tuner_gain_db(&mut self, db: f64) -> Result<()> {
        self.tuner_gain_db = db;
        self.apply_tuner_gain()
    }

    /// Let the tuner run its own gain loops.
    pub fn set_tuner_agc(&mut self, on: bool) -> Result<()> {
        self.tuner_agc = on;
        self.apply_tuner_gain()
    }

    fn apply_tuner_gain(&mut self) -> Result<()> {
        let setting = if self.tuner_agc {
            GainSetting::Auto
        } else {
            GainSetting::Manual { total_db: self.tuner_gain_db }
        };
        if let Some(t) = self.tuner.as_mut() {
            t.set_gain(&self.usb, setting)?;
        }
        Ok(())
    }
}

/// Whether an R828D answers on the FX3's I2C bus.
fn tuner_present(usb: &UsbDev) -> bool {
    crate::tuner::present(usb, sdroxide_r82xx::R828D_I2C_ADDR)
}

/// Ask the running firmware what it is and what it thinks it is driving.
fn identify(usb: &UsbDev) -> Option<Identity> {
    let id = usb.vendor_in(Cmd::TestFx3, 0, 0, 4).ok().and_then(|b| Identity::parse(&b))?;
    tracing::info!("{} — firmware {} (hardware id 0x{:02x})", id.board(), id.version, id.hardware);
    Some(id)
}

/// The operator-facing consequence of a receiver whose front end this driver
/// cannot steer: everything on the front panel of the settings page is inert.
///
/// Stated in terms of what stops working rather than in terms of a hardware id,
/// because the symptom — "the bias tee does nothing" — is what brings someone
/// here, and nothing else in the program can tell them why.
///
/// Two different situations end up here and they need different answers
/// (issue #342). A board the firmware *failed to identify* is a fault, and
/// re-plugging it usually clears it. A board it identified and this driver does
/// not drive — an RX-888 Mk1, which has none of the Mk2's front-end parts to
/// drive — is not a fault at all: it will stream for as long as it is plugged
/// in, and no amount of re-plugging will grow it an attenuator. Telling the
/// second owner the first story is what sends them looking for a problem that
/// is not there.
fn front_end_warning(identity: Option<Identity>) -> Option<String> {
    let id = identity?;
    if id.front_end_ready() {
        return None;
    }
    Some(if id.recognised() {
        format!(
            "this is {}, and sdroxide drives the Mk2's front end only: the bias tee, dither, \
             ADC range and both gain stages will do nothing here, and there is no VHF tuner to \
             reach. Reception is unaffected — the ADC, the sample clock and the whole HF \
             spectrum work exactly as they do on a Mk2, and the gain in front of the converter \
             is whatever the board itself sets.",
            id.board()
        )
    } else {
        format!(
            "the RX-888 firmware did not recognise this receiver (hardware id 0x{:02x}), so it \
             never initialised the front-end controls: the bias tee, dither, ADC range and both \
             gain stages will do nothing. Reception is unaffected. Unplugging the receiver and \
             plugging it back in usually clears it.",
            id.hardware
        )
    })
}

fn join_warnings(a: Option<String>, b: Option<String>) -> Option<String> {
    match (a, b) {
        (Some(a), Some(b)) => Some(format!("{a}  •  {b}")),
        (a, b) => a.or(b),
    }
}

/// Clamp the requested ADC rate to what the negotiated link can carry, and say
/// so if it had to.
///
/// The failure this prevents is specific and confusing: the receiver enumerates,
/// the firmware runs, the ADC clocks, and samples simply go missing, which looks
/// like a broken radio rather than a slow cable.
fn clamp_rate_to_link(requested: f64, usb: &UsbDev) -> (f64, Option<String>) {
    let budget = usb::usable_bytes_per_sec(usb.speed());
    let max_rate = budget / 2.0; // 16-bit real samples
    if requested <= max_rate {
        return (requested, None);
    }
    let clamped = ADC_RATES.iter().copied().filter(|r| *r <= max_rate).fold(MIN_ADC_HZ, f64::max);
    let msg = format!(
        "the RX-888 is on a {} link, which can only carry about {:.1} Msps — \
         the sample rate has been reduced to {:.1} Msps ({:.1} MHz of spectrum). \
         Use a USB 3 cable directly into a USB 3 port for the full rate.",
        usb::speed_name(usb.speed()),
        max_rate / 1e6,
        clamped / 1e6,
        clamped / 2e6,
    );
    tracing::warn!("{msg}");
    (clamped, Some(msg))
}

// ---- gain mappings -----------------------------------------------------

/// Attenuator code for a requested gain in dB (negative = attenuation).
fn att_code_for(db: f64) -> u8 {
    let atten = (-db).clamp(0.0, -ATT_MIN_DB);
    (atten / ATT_STEP_DB).round().clamp(0.0, ATT_MAX_CODE as f64) as u8
}

/// The gain a given attenuator code produces.
fn att_db_for(code: u8) -> f64 {
    -(code.min(ATT_MAX_CODE) as f64) * ATT_STEP_DB
}

/// The AD8370's linear voltage vernier, per code step, in the low range.
const AD8370_VERNIER: f64 = 0.055;
/// How much the high range multiplies it by.
const AD8370_PREGAIN: f64 = 7.079;
/// Bit 7 of the register selects the high range.
const AD8370_HIGH: u8 = 0x80;

/// The gain a given AD8370 register value produces.
///
/// `A_v = code · vernier · (high ? pregain : 1)`, straight from the datasheet's
/// transfer function. Code 0 mutes the part, which has no dB value; it is
/// reported as a large negative number so callers can still order gains.
pub fn ad8370_db_for(reg: u8) -> f64 {
    let code = (reg & !AD8370_HIGH) as f64;
    if code == 0.0 {
        return f64::NEG_INFINITY;
    }
    let av = code * AD8370_VERNIER * if reg & AD8370_HIGH != 0 { AD8370_PREGAIN } else { 1.0 };
    20.0 * av.log10()
}

/// The register value that comes closest to `db`.
///
/// Both ranges overlap across the middle of the span, so the choice is made by
/// which one lands nearer the request — preferring the low range on a tie, since
/// it reaches a given gain with less pre-gain ahead of the vernier.
pub fn ad8370_code_for(db: f64) -> u8 {
    let av = 10f64.powf(db / 20.0);
    let pick = |scale: f64| -> u8 { (av / scale).round().clamp(1.0, 127.0) as u8 };

    let low = pick(AD8370_VERNIER);
    let high = pick(AD8370_VERNIER * AD8370_PREGAIN) | AD8370_HIGH;

    let err = |reg: u8| (ad8370_db_for(reg) - db).abs();
    if err(low) <= err(high) { low } else { high }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attenuator_codes_span_the_documented_range() {
        assert_eq!(att_code_for(0.0), 0);
        assert_eq!(att_code_for(-31.5), 63);
        // Beyond the hardware's reach in both directions.
        assert_eq!(att_code_for(10.0), 0);
        assert_eq!(att_code_for(-99.0), 63);
        // The 0.5 dB grid.
        assert_eq!(att_code_for(-10.0), 20);
        assert_eq!(att_code_for(-10.2), 20);
        assert_eq!(att_db_for(20), -10.0);
    }

    #[test]
    fn attenuator_round_trips_on_its_own_grid() {
        for code in 0..=ATT_MAX_CODE {
            assert_eq!(att_code_for(att_db_for(code)), code, "code {code}");
        }
    }

    #[test]
    fn ad8370_reproduces_the_datasheet_range_endpoints() {
        // Low range tops out near +17 dB, high range near +34 dB — the two
        // figures the datasheet quotes.
        let low_max = ad8370_db_for(127);
        let high_max = ad8370_db_for(127 | AD8370_HIGH);
        assert!((low_max - 16.9).abs() < 0.3, "low range max was {low_max}");
        assert!((high_max - 33.9).abs() < 0.3, "high range max was {high_max}");
        // The high range sits 17 dB above the low one at the same code.
        let step = ad8370_db_for(64 | AD8370_HIGH) - ad8370_db_for(64);
        assert!((step - 17.0).abs() < 0.1, "range step was {step}");
    }

    #[test]
    fn ad8370_gain_is_monotonic_within_each_range() {
        for range in [0u8, AD8370_HIGH] {
            let mut prev = f64::NEG_INFINITY;
            for code in 1..=127u8 {
                let db = ad8370_db_for(code | range);
                assert!(db > prev, "code {code} range {range:#x} went backwards");
                prev = db;
            }
        }
    }

    #[test]
    fn ad8370_codes_land_close_to_what_was_asked_for() {
        // Across the usable span, the chosen code should be within half a step
        // of the request. The vernier is linear in voltage, so the dB step grows
        // as gain falls; 1 dB is a fair bound above -6 dB.
        for tenth in -60..=339 {
            let want = tenth as f64 / 10.0;
            let got = ad8370_db_for(ad8370_code_for(want));
            assert!((got - want).abs() <= 1.0, "asked {want:.1} dB, got {got:.1} dB");
        }
    }

    #[test]
    fn ad8370_uses_the_high_range_only_where_the_low_one_cannot_reach() {
        // Above the low range's ceiling there is no choice.
        assert_eq!(ad8370_code_for(30.0) & AD8370_HIGH, AD8370_HIGH);
        // Well inside the low range, it should not reach for the pre-gain.
        assert_eq!(ad8370_code_for(0.0) & AD8370_HIGH, 0);
        assert_eq!(ad8370_code_for(10.0) & AD8370_HIGH, 0);
    }

    #[test]
    fn a_muted_vga_has_no_finite_gain() {
        assert_eq!(ad8370_db_for(0), f64::NEG_INFINITY);
        // ...and is never chosen for a real request.
        assert!(ad8370_code_for(-40.0) >= 1);
    }

    #[test]
    fn the_default_rate_is_one_of_the_offered_rates() {
        assert!(ADC_RATES.contains(&DEFAULT_ADC_HZ));
        assert!(ADC_RATES.iter().all(|r| *r >= MIN_ADC_HZ && *r <= MAX_ADC_HZ));
    }
}
