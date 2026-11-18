//! The tuner itself: bring-up, tuning, filters and gain.
//!
//! See the crate documentation for provenance and for the two properties of the
//! part — write-only registers and bit-reversed reads — that shape all of it.

use std::time::Duration;

use crate::bus::TunerBus;
use crate::error::{Error, Result};
use crate::tables::{
    self, FILT_HP_BW1, FILT_HP_BW2, GAINS_TENTH_DB, IF_LOW_PASS_BW, INIT_ARRAY, LNA_GAIN_STEPS,
    MIXER_GAIN_STEPS, NUM_REGS, REG_SHADOW_START, VER_NUM,
};

/// The IF `set_tv_standard` leaves the part on, and what [`R82xx::set_bandwidth`]
/// reports for anything under 6 MHz.
pub const DEFAULT_IF_HZ: f64 = 3_570_000.0;

/// The tuner reference on an RTL-SDR Blog V4.
///
/// The same number as [`V4_HF_CROSSOVER_HZ`] and for the same reason: it is the
/// demodulator's crystal, which is also what the board's upconverter mixes
/// against. The RTL-SDR backend cross-checks it against its own `RTL_XTAL_HZ`
/// so the two cannot drift apart.
pub const BLOG_V4_REF_HZ: f64 = 28_800_000.0;

/// Apply a parts-per-million correction to a nominal frequency.
pub fn apply_ppm(nominal_hz: f64, ppm: i32) -> f64 {
    nominal_hz * (1.0 + ppm as f64 / 1e6)
}

/// I2C address of an R820T/R820T2.
pub const R820T_I2C_ADDR: u8 = 0x34;
/// I2C address of an R828D.
pub const R828D_I2C_ADDR: u8 = 0x74;
/// Register 0 of any R82xx reads back as this once bit-reversed.
pub const R82XX_CHECK_VAL: u8 = 0x69;

/// Reference crystal on an R820T/R820T2 board.
pub const R820T_XTAL_HZ: f64 = 28_800_000.0;
/// The R828D uses a different part. Feeding the R820T value to an R828D PLL
/// puts every frequency out by a factor of 1.8.
pub const R828D_XTAL_HZ: f64 = 16_000_000.0;

/// Frequency below which a Blog V4 upconverts, and the top of its HF input.
pub const V4_HF_CROSSOVER_HZ: f64 = 28_800_000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chip {
    /// R820T and R820T2 are indistinguishable over I2C, so they share a
    /// variant; the label reports both names.
    R820T,
    R828D,
}

impl Chip {
    pub fn i2c_addr(self) -> u8 {
        match self {
            Chip::R820T => R820T_I2C_ADDR,
            Chip::R828D => R828D_I2C_ADDR,
        }
    }

    pub fn nominal_xtal_hz(self) -> f64 {
        match self {
            Chip::R820T => R820T_XTAL_HZ,
            Chip::R828D => R828D_XTAL_HZ,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Chip::R820T => "R820T/R820T2",
            Chip::R828D => "R828D",
        }
    }
}

/// Which input a Blog V4 is switched to. Tracked so the four register writes
/// that change it happen only on a band change — a dial spin must not repeat
/// them hundreds of times a second.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Band {
    Hf,
    Vhf,
    Uhf,
}

/// Requested gain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GainSetting {
    /// The tuner runs its own LNA and mixer loops.
    Auto,
    /// Fixed gain. The value is snapped to [`GAINS_TENTH_DB`].
    Manual { total_db: f64 },
}

/// One of the three gain stages, set on its own rather than through the
/// combined LNA/mixer ladder of [`GainSetting::Manual`].
// Why: HDZ 2026-11-09: "beim RTL-SDR gibt es in SDR++ typischerweise zwei
//      unterschiedliche Verstärkungsregler: LNA Gain … IF Gain"; the mixer
//      as a third.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// R5 low nibble, [`LNA_GAIN_STEPS`].
    Lna,
    /// R7 low nibble, [`MIXER_GAIN_STEPS`].
    Mixer,
    /// The IF VGA, R12 low nibble. No measured table: librtlsdr names two of
    /// its codes, 0x08 ≈ 16.3 dB and 0x0b ≈ 26.5 dB, so it is offered in
    /// steps rather than in invented decibels.
    Vga,
}

/// Highest index of each stage's 16-step ladder.
pub const STAGE_MAX_INDEX: u8 = 15;

/// What `index` steps of `stage` add up to, in dB: the measured increments
/// summed for LNA and mixer, the index itself for the VGA.
pub fn stage_db(stage: Stage, index: u8) -> f64 {
    let i = usize::from(index.min(STAGE_MAX_INDEX));
    match stage {
        Stage::Lna => LNA_GAIN_STEPS[..=i].iter().sum::<i32>() as f64 / 10.0,
        Stage::Mixer => MIXER_GAIN_STEPS[..=i].iter().sum::<i32>() as f64 / 10.0,
        Stage::Vga => i as f64,
    }
}

/// The index of `stage` that comes nearest `db` (lowest on a tie).
pub fn stage_index_for(stage: Stage, db: f64) -> u8 {
    (0..=STAGE_MAX_INDEX)
        .min_by(|a, b| {
            (stage_db(stage, *a) - db).abs().total_cmp(&(stage_db(stage, *b) - db).abs())
        })
        .unwrap_or(0)
}

/// The LNA and mixer indices the combined ladder lands on for `total_db` —
/// the same walk [`R82xx::set_gain`] makes.
pub fn ladder_indices(total_db: f64) -> (u8, u8) {
    let want = tables::snap_gain_tenth_db(total_db);
    let mut total = 0i32;
    let (mut lna_index, mut mix_index) = (0usize, 0usize);
    for _ in 0..15 {
        if total >= want {
            break;
        }
        lna_index += 1;
        total += LNA_GAIN_STEPS[lna_index];
        if total >= want {
            break;
        }
        mix_index += 1;
        total += MIXER_GAIN_STEPS[mix_index];
    }
    (lna_index as u8, mix_index as u8)
}

pub struct R82xx {
    chip: Chip,
    is_blog_v4: bool,
    /// Shadow of R5..R34. The part is write-only above R5, so this is the only
    /// way to do a read-modify-write.
    regs: [u8; NUM_REGS],
    /// Current intermediate frequency, moved by [`R82xx::set_bandwidth`].
    int_freq: f64,
    has_lock: bool,
    /// ppm-corrected reference crystal.
    xtal: f64,
    fil_cal_code: u8,
    /// Last input selected, for the change-only switching above.
    band: Option<Band>,
    /// Last standard-R828D input value, same purpose.
    input: u8,
    init_done: bool,
    /// Last LO actually programmed, so an unchanged retune costs nothing.
    last_lo_hz: Option<f64>,
    /// The IF VGA's code under manual gain: librtlsdr's fixed 0x08 until
    /// [`Stage::Vga`] is set, and kept across a change of the combined gain.
    vga_index: u8,
}

impl R82xx {
    pub fn new(chip: Chip, is_blog_v4: bool, ppm: i32) -> R82xx {
        R82xx {
            chip,
            is_blog_v4,
            regs: [0; NUM_REGS],
            int_freq: DEFAULT_IF_HZ,
            has_lock: false,
            xtal: apply_ppm(Self::reference_xtal_hz(chip, is_blog_v4), ppm),
            fil_cal_code: 0,
            band: None,
            input: 0,
            init_done: false,
            last_lo_hz: None,
            vga_index: 0x08,
        }
    }

    pub fn name(&self) -> &'static str {
        self.chip.name()
    }

    pub fn is_blog_v4(&self) -> bool {
        self.is_blog_v4
    }

    /// The tuner's PLL reference, before ppm correction.
    ///
    /// A stock R828D board clocks the tuner from its own 16 MHz crystal, but a
    /// Blog V4 runs it from the demodulator's 28.8 MHz reference — the same
    /// clock its upconverter mixes against, which is why the crossover and the
    /// reference are the same number. Handing a V4 the 16 MHz figure is the
    /// mirror of the hazard on [`R828D_XTAL_HZ`] and puts every frequency out
    /// by the same factor of 1.8.
    fn reference_xtal_hz(chip: Chip, is_blog_v4: bool) -> f64 {
        if is_blog_v4 { BLOG_V4_REF_HZ } else { chip.nominal_xtal_hz() }
    }

    /// Re-derive the PLL reference from a new ppm figure. The caller must
    /// retune afterwards for it to take effect.
    pub fn set_ppm(&mut self, ppm: i32) {
        self.xtal = apply_ppm(Self::reference_xtal_hz(self.chip, self.is_blog_v4), ppm);
        self.last_lo_hz = None;
    }

    // ---- register access -------------------------------------------------

    fn shadow_index(reg: u8) -> Option<usize> {
        let i = reg.checked_sub(REG_SHADOW_START)? as usize;
        (i < NUM_REGS).then_some(i)
    }

    /// Write consecutive registers starting at `reg`.
    ///
    /// Skips the bus entirely when the shadow already holds these values.
    /// Upstream does the same, and it matters here for a different reason: the
    /// retune path runs on the streaming thread, and every I2C byte skipped is
    /// bulk-transfer time not lost.
    fn write(&mut self, bus: &dyn TunerBus, reg: u8, val: &[u8]) -> Result<()> {
        if let Some(i) = Self::shadow_index(reg) {
            if i + val.len() <= NUM_REGS && self.regs[i..i + val.len()] == *val {
                return Ok(());
            }
        }
        // Store first: a failed transfer leaves the part in an unknown state
        // either way, and the next open re-initialises from scratch.
        if let Some(i) = Self::shadow_index(reg) {
            let n = val.len().min(NUM_REGS - i);
            self.regs[i..i + n].copy_from_slice(&val[..n]);
        }

        // Chunking is the bus's business: the limit that forces it belongs to
        // whatever is carrying the bytes, not to the tuner.
        Ok(bus.write_regs(self.chip.i2c_addr(), reg, val)?)
    }

    fn write_reg(&mut self, bus: &dyn TunerBus, reg: u8, val: u8) -> Result<()> {
        self.write(bus, reg, &[val])
    }

    /// Read-modify-write against the shadow, since the part cannot be read
    /// back register by register.
    fn write_reg_mask(&mut self, bus: &dyn TunerBus, reg: u8, val: u8, mask: u8) -> Result<()> {
        let cur = Self::shadow_index(reg).map(|i| self.regs[i]).ok_or_else(|| {
            Error::Unsupported(format!("R82xx register {reg:#04x} is not shadowed"))
        })?;
        self.write_reg(bus, reg, (cur & !mask) | (val & mask))
    }

    /// Read `len` registers starting at R0, undoing the part's bit reversal.
    ///
    /// Always from R0: the part has no random-access read, it just runs on from
    /// the start. The bit reversal is the tuner's own quirk, so it is undone
    /// here rather than in the bus.
    fn read(&self, bus: &dyn TunerBus, len: u16) -> Result<Vec<u8>> {
        let mut raw = vec![0u8; len as usize];
        bus.read_regs(self.chip.i2c_addr(), 0, &mut raw)?;
        Ok(raw.into_iter().map(tables::bitrev).collect())
    }

    // ---- bring-up --------------------------------------------------------

    pub fn init(&mut self, bus: &dyn TunerBus) -> Result<()> {
        self.regs = [0; NUM_REGS];
        let init = INIT_ARRAY;
        self.write(bus, REG_SHADOW_START, &init)?;
        self.set_tv_standard(bus)?;
        self.sysfreq_sel(bus)?;
        self.init_done = true;
        Ok(())
    }

    /// Put the tuner to sleep. Used when switching to direct sampling, where
    /// a live tuner only injects its LO into the ADC.
    pub fn standby(&mut self, bus: &dyn TunerBus) -> Result<()> {
        if !self.init_done {
            return Ok(());
        }
        // Upstream writes these directly rather than through the shadow, so
        // the shadow is deliberately invalidated afterwards.
        for (reg, val) in [(0x06, 0xb1), (0x05, 0xa0), (0x07, 0x3a), (0x08, 0x40), (0x09, 0xc0)] {
            self.write_reg(bus, reg, val)?;
        }
        self.regs = [0; NUM_REGS];
        self.init_done = false;
        self.last_lo_hz = None;
        self.band = None;
        Ok(())
    }

    /// The DVB-T defaults plus the IF filter calibration.
    fn set_tv_standard(&mut self, bus: &dyn TunerBus) -> Result<()> {
        // "BW < 6 MHz" constants from upstream, kept with their comments.
        let filt_cal_lo = 56_000u32; // kHz
        let filt_gain = 0x10; // +3 dB, 6 MHz on
        let img_r = 0x00; // image negative
        let filt_q = 0x10; // R10[4]: low Q
        let hp_cor = 0x6b; // 1.7 MHz disable, +2 cap, 1.0 MHz
        let ext_enable = 0x60; // R30[6] ext enable, R30[5] ext at lna max-1
        let loop_through = 0x01; // R5[7], LT off
        let lt_att = 0x00; // R31[7], LT att enable
        let flt_ext_widest = 0x00; // R15[7], flt_ext_wide off
        let polyfil_cur = 0x60; // R25[6:5] min

        self.regs = INIT_ARRAY;

        // Init flag and xtal-check result (also initialises VGA gain).
        self.write_reg_mask(bus, 0x0c, 0x00, 0x0f)?;
        self.write_reg_mask(bus, 0x13, VER_NUM, 0x3f)?;
        // For LT gain test.
        self.write_reg_mask(bus, 0x1d, 0x00, 0x38)?;

        self.int_freq = 3_570_000.0;

        // Filter calibration. Upstream always runs it, because rtlsdr calls
        // this exactly once per open.
        for _ in 0..2 {
            self.write_reg_mask(bus, 0x0b, hp_cor, 0x60)?;
            // Calibration clock on.
            self.write_reg_mask(bus, 0x0f, 0x04, 0x04)?;
            // 0 pF crystal cap for the PLL.
            self.write_reg_mask(bus, 0x10, 0x00, 0x03)?;

            self.set_pll(bus, filt_cal_lo as f64 * 1000.0)?;
            if !self.has_lock {
                return Err(Error::PllUnlocked(filt_cal_lo as f64 / 1000.0));
            }

            // Trigger the calibration, then stop it.
            self.write_reg_mask(bus, 0x0b, 0x10, 0x10)?;
            std::thread::sleep(Duration::from_millis(2));
            self.write_reg_mask(bus, 0x0b, 0x00, 0x10)?;
            // Calibration clock off.
            self.write_reg_mask(bus, 0x0f, 0x00, 0x04)?;

            let data = self.read(bus, 5)?;
            self.fil_cal_code = data[4] & 0x0f;
            if self.fil_cal_code != 0 && self.fil_cal_code != 0x0f {
                break;
            }
        }
        // 0x0f means the calibration never converged; upstream falls back to
        // the narrowest setting rather than failing the open.
        if self.fil_cal_code == 0x0f {
            self.fil_cal_code = 0;
        }

        self.write_reg_mask(bus, 0x0a, filt_q | self.fil_cal_code, 0x1f)?;
        self.write_reg_mask(bus, 0x0b, hp_cor, 0xef)?;
        self.write_reg_mask(bus, 0x07, img_r, 0x80)?;
        self.write_reg_mask(bus, 0x06, filt_gain, 0x30)?;
        self.write_reg_mask(bus, 0x1e, ext_enable, 0x60)?;
        self.write_reg_mask(bus, 0x05, loop_through, 0x80)?;
        self.write_reg_mask(bus, 0x1f, lt_att, 0x80)?;
        self.write_reg_mask(bus, 0x0f, flt_ext_widest, 0x80)?;
        self.write_reg_mask(bus, 0x19, polyfil_cur, 0x60)?;
        Ok(())
    }

    /// The DVB-T system settings. Upstream branches on delivery system and
    /// tuner type; rtlsdr only ever uses `SYS_DVBT` with `TUNER_DIGITAL_TV` at
    /// frequency 0, so only that path is transcribed.
    fn sysfreq_sel(&mut self, bus: &dyn TunerBus) -> Result<()> {
        let mixer_top = 0x24; // mixer top 13, top-1, low discharge
        let lna_top = 0xe5; // detect bw 3, lna top 4, predet top 2
        let cp_cur = 0x38; // 111, auto
        let div_buf_cur = 0x30; // 11, 150u
        let lna_vth_l = 0x53; // lna vth 0.84, vtl 0.64
        let mixer_vth_l = 0x75; // mixer vth 1.04, vtl 0.84
        let air_cable1_in = 0x00;
        let cable2_in = 0x00;
        let lna_discharge = 14;
        let filter_cur = 0x40; // 10, low

        self.write_reg_mask(bus, 0x1d, lna_top, 0xc7)?;
        self.write_reg_mask(bus, 0x1c, mixer_top, 0xf8)?;
        self.write_reg(bus, 0x0d, lna_vth_l)?;
        self.write_reg(bus, 0x0e, mixer_vth_l)?;

        self.input = air_cable1_in;
        self.write_reg_mask(bus, 0x05, air_cable1_in, 0x60)?;
        self.write_reg_mask(bus, 0x06, cable2_in, 0x08)?;
        self.write_reg_mask(bus, 0x11, cp_cur, 0x38)?;
        self.write_reg_mask(bus, 0x17, div_buf_cur, 0x30)?;
        self.write_reg_mask(bus, 0x0a, filter_cur, 0x60)?;

        // LNA setup. The odd-looking 0x04 mask on register 0x1c is upstream's;
        // its own comment calls it wrong but keeps it to match the original
        // vendor driver, so it stays here too.
        self.write_reg_mask(bus, 0x1d, 0, 0x38)?;
        self.write_reg_mask(bus, 0x1c, 0, 0x04)?;
        self.write_reg_mask(bus, 0x06, 0, 0x40)?;
        self.write_reg_mask(bus, 0x1a, 0x30, 0x30)?;
        self.write_reg_mask(bus, 0x1d, 0x18, 0x38)?;
        self.write_reg_mask(bus, 0x1c, mixer_top, 0x04)?;
        self.write_reg_mask(bus, 0x1e, lna_discharge, 0x1f)?;
        self.write_reg_mask(bus, 0x1a, 0x20, 0x30)?;
        Ok(())
    }

    // ---- tuning ----------------------------------------------------------

    /// Select the tracking filter and mux for an LO frequency.
    fn set_mux(&mut self, bus: &dyn TunerBus, lo_hz: f64) -> Result<()> {
        let range = tables::freq_range_for(lo_hz);
        self.write_reg_mask(bus, 0x17, range.open_d, 0x08)?;
        self.write_reg_mask(bus, 0x1a, range.rf_mux_ploy, 0xc3)?;
        self.write_reg(bus, 0x1b, range.tf_c)?;
        // rtlsdr always selects XTAL_HIGH_CAP_0P, so the `| 0x08` drive bit
        // that the other cases add is deliberately absent.
        self.write_reg_mask(bus, 0x10, range.xtal_cap0p, 0x0b)?;
        self.write_reg_mask(bus, 0x08, 0x00, 0x3f)?;
        self.write_reg_mask(bus, 0x09, 0x00, 0x3f)?;
        Ok(())
    }

    /// Program the synthesiser to `lo_hz`.
    ///
    /// Sets [`R82xx::has_lock`]; the caller decides whether an unlocked PLL is
    /// fatal. Costs up to 20 ms when the first VCO current setting does not
    /// lock, which is the stall the bulk queue depth is sized to absorb.
    fn set_pll(&mut self, bus: &dyn TunerBus, lo_hz: f64) -> Result<()> {
        const VCO_MIN_KHZ: u64 = 1_770_000;
        const VCO_MAX_KHZ: u64 = VCO_MIN_KHZ * 2;

        let freq = lo_hz.round() as u64;
        let freq_khz = (freq + 500) / 1000;
        let pll_ref = self.xtal.round() as u64;

        // PLL autotune 128 kHz while we move.
        self.write_reg_mask(bus, 0x1a, 0x00, 0x0c)?;

        // Upstream batches R16..R22 into one 7-byte write, so build them up
        // from the shadow rather than writing each individually.
        let base = Self::shadow_index(0x10).expect("0x10 is inside the shadow");
        let mut regs: [u8; 7] = self.regs[base..base + 7].try_into().expect("7 bytes");

        let mask = |r: u8, v: u8, m: u8| (r & !m) | (v & m);

        // refdiv2 off.
        regs[0] = mask(regs[0], 0x00, 0x10);
        // VCO current = 100.
        regs[2] = mask(regs[2], 0x80, 0xe0);

        // Pick the mixer divider that puts the VCO in band.
        let mut mix_div: u64 = 2;
        let mut div_num: i32 = 0;
        while mix_div <= 64 {
            if freq_khz * mix_div >= VCO_MIN_KHZ && freq_khz * mix_div < VCO_MAX_KHZ {
                let mut div_buf = mix_div;
                while div_buf > 2 {
                    div_buf >>= 1;
                    div_num += 1;
                }
                break;
            }
            mix_div <<= 1;
        }

        let data = self.read(bus, 5)?;
        let vco_power_ref: u32 = if self.chip == Chip::R828D { 1 } else { 2 };
        let vco_fine_tune = ((data[4] & 0x30) >> 4) as u32;
        if vco_fine_tune > vco_power_ref {
            div_num -= 1;
        } else if vco_fine_tune < vco_power_ref {
            div_num += 1;
        }
        regs[0] = mask(regs[0], (div_num as u8) << 5, 0xe0);

        let vco_freq = freq * mix_div;
        // Approximate vco_freq / (2 * pll_ref) as nint + sdm/65536, rounded.
        let vco_div = (pll_ref + 65536 * vco_freq) / (2 * pll_ref);
        let nint = (vco_div / 65536) as u32;
        let sdm = (vco_div % 65536) as u32;

        if nint > (128 / vco_power_ref) - 1 {
            return Err(Error::Unsupported(format!(
                "no valid R82xx PLL divider for an LO of {:.6} MHz",
                lo_hz / 1e6
            )));
        }

        let ni = (nint - 13) / 4;
        let si = nint - 4 * ni - 13;
        regs[4] = (ni + (si << 6)) as u8;
        // pw_sdm: power the sigma-delta down when the fraction is zero.
        regs[2] = mask(regs[2], if sdm == 0 { 0x08 } else { 0x00 }, 0x08);
        regs[5] = (sdm & 0xff) as u8;
        regs[6] = (sdm >> 8) as u8;

        self.write(bus, 0x10, &regs)?;

        let mut locked = false;
        for attempt in 0..2 {
            std::thread::sleep(Duration::from_millis(10));
            let data = self.read(bus, 3)?;
            if data[2] & 0x40 != 0 {
                locked = true;
                break;
            }
            if attempt == 0 {
                // Didn't lock — raise the VCO current and try once more.
                self.write_reg_mask(bus, 0x12, 0x60, 0xe0)?;
            }
        }

        self.has_lock = locked;
        if !locked {
            tracing::warn!("R82xx PLL did not lock at {:.6} MHz", lo_hz / 1e6);
            return Ok(());
        }

        // PLL autotune back to 8 kHz now that we have arrived.
        self.write_reg_mask(bus, 0x1a, 0x08, 0x08)
    }

    /// The frequency the tuner itself must reach for a given dial setting.
    ///
    /// A Blog V4 sums HF with a 28.8 MHz reference on board, so below the
    /// crossover the tuner sees the dial shifted up by it. The result is *not*
    /// spectrum-inverted and no offset is applied anywhere else, so the
    /// operator sees one continuous 0–1766 MHz radio.
    fn tuned_freq_hz(&self, dial_hz: f64) -> f64 {
        if self.is_blog_v4 && dial_hz < V4_HF_CROSSOVER_HZ {
            dial_hz + V4_HF_CROSSOVER_HZ
        } else {
            dial_hz
        }
    }

    /// Tune to `hz`, handling the Blog V4's upconverter and input switching.
    pub fn set_freq(&mut self, bus: &dyn TunerBus, hz: f64) -> Result<()> {
        let lo_hz = self.tuned_freq_hz(hz) + self.int_freq;

        if self.last_lo_hz == Some(lo_hz) && self.has_lock {
            return Ok(());
        }

        self.set_mux(bus, lo_hz)?;
        self.set_pll(bus, lo_hz)?;
        if !self.has_lock {
            self.last_lo_hz = None;
            return Err(Error::PllUnlocked(hz / 1e6));
        }
        self.last_lo_hz = Some(lo_hz);

        if self.is_blog_v4 {
            self.v4_switch_inputs(bus, hz)?;
        } else if self.chip == Chip::R828D {
            // Switch between Cable1 and Air-In at 345 MHz, where the noise
            // floor is about equal with identical LNA settings. Only on a
            // change: a dial spin must not rewrite this hundreds of times a
            // second.
            let air_cable1_in = if hz > 345e6 { 0x00 } else { 0x60 };
            if air_cable1_in != self.input {
                self.input = air_cable1_in;
                self.write_reg_mask(bus, 0x05, air_cable1_in, 0x60)?;
            }
        }
        Ok(())
    }

    /// Blog V4 notch filters and three-way input switching.
    fn v4_switch_inputs(&mut self, bus: &dyn TunerBus, hz: f64) -> Result<()> {
        // Notches are OFF inside the band they protect against and ON outside
        // it — so tuning *to* broadcast FM or DAB does not notch out the very
        // thing being received.
        let open_d = if hz <= 2.2e6 || (85e6..=112e6).contains(&hz) || (172e6..=242e6).contains(&hz)
        {
            0x00
        } else {
            0x08
        };
        self.write_reg_mask(bus, 0x17, open_d, 0x08)?;

        let band = if hz <= 28.8e6 {
            Band::Hf
        } else if hz < 250e6 {
            Band::Vhf
        } else {
            Band::Uhf
        };

        // Take the tracking filter out of circuit on HF. What reaches the tuner
        // there is the upconverter's output, already filtered on the way in, so
        // the R828D's own preselector can only add its insertion loss — and a
        // loss ahead of the LNA comes straight off the noise figure, which is
        // how a quiet band ends up reading as a wall of hiss. Bypass is
        // `rf_mux_ploy = 0x40` with no tracking capacitor at all.
        //
        // Deliberately *above* the band-change guard below: `set_mux` rewrites
        // 0x1a and 0x1b from the tracking-filter table on every single tune, so
        // bypassing once at the band change would survive exactly until the next
        // nudge of the dial.
        if band == Band::Hf {
            self.write_reg_mask(bus, 0x1a, 0x40, 0xc3)?;
            self.write_reg(bus, 0x1b, 0x00)?;
        }

        if self.band == Some(band) {
            return Ok(());
        }
        self.band = Some(band);

        // Cable 2 is the HF input.
        let cable_2_in = if band == Band::Hf { 0x08 } else { 0x00 };
        self.write_reg_mask(bus, 0x06, cable_2_in, 0x08)?;
        // Newer V4 batches gate the upconverter with GPIO 5.
        bus.set_upconverter_gate(cable_2_in == 0)?;
        // Cable 1 is VHF, air-in is UHF.
        let cable_1_in = if band == Band::Vhf { 0x40 } else { 0x00 };
        self.write_reg_mask(bus, 0x05, cable_1_in, 0x40)?;
        let air_in = if band == Band::Uhf { 0x00 } else { 0x20 };
        self.write_reg_mask(bus, 0x05, air_in, 0x20)?;
        Ok(())
    }

    /// Choose the IF filter for a signal bandwidth, returning the new
    /// intermediate frequency.
    ///
    /// The returned value is not decoration: the demodulator's DDC must be
    /// reprogrammed to it, because the filter choice *moves the IF*. Callers
    /// that ignore it end up receiving up to a megahertz away from the dial.
    pub fn set_bandwidth(&mut self, bus: &dyn TunerBus, bw_hz: f64) -> Result<f64> {
        let mut bw = bw_hz.round() as i32;
        let mut real_bw = 0i32;
        let reg_0a;
        let mut reg_0b;

        if bw > 7_000_000 {
            reg_0a = 0x10;
            reg_0b = 0x0b;
            self.int_freq = 4_570_000.0;
        } else if bw > 6_000_000 {
            reg_0a = 0x10;
            reg_0b = 0x2a;
            self.int_freq = 4_570_000.0;
        } else if bw > IF_LOW_PASS_BW[0] + FILT_HP_BW1 + FILT_HP_BW2 {
            reg_0a = 0x10;
            reg_0b = 0x6b;
            self.int_freq = 3_570_000.0;
        } else {
            reg_0a = 0x00;
            reg_0b = 0x80;
            let mut int_freq = 2_300_000i32;

            if bw > IF_LOW_PASS_BW[0] + FILT_HP_BW1 {
                bw -= FILT_HP_BW2;
                int_freq += FILT_HP_BW2;
                real_bw += FILT_HP_BW2;
            } else {
                reg_0b |= 0x20;
            }
            if bw > IF_LOW_PASS_BW[0] {
                bw -= FILT_HP_BW1;
                int_freq += FILT_HP_BW1;
                real_bw += FILT_HP_BW1;
            } else {
                reg_0b |= 0x40;
            }

            let mut i = 0;
            while i < IF_LOW_PASS_BW.len() {
                if bw > IF_LOW_PASS_BW[i] {
                    break;
                }
                i += 1;
            }
            // Upstream decrements unconditionally after the loop; at i == 0
            // that underflows in C and lands on the widest entry, so clamp.
            let i = i.saturating_sub(1);
            reg_0b |= (15 - i) as u8;
            real_bw += IF_LOW_PASS_BW[i];
            self.int_freq = (int_freq - real_bw / 2) as f64;
        }

        self.write_reg_mask(bus, 0x0a, reg_0a, 0x10)?;
        self.write_reg_mask(bus, 0x0b, reg_0b, 0xef)?;
        // The IF moved, so any cached LO is stale.
        self.last_lo_hz = None;
        Ok(self.int_freq)
    }

    // ---- gain ------------------------------------------------------------

    /// Apply a gain setting, returning the gain actually in effect in dB
    /// (`None` when the tuner's own loops are driving it).
    pub fn set_gain(&mut self, bus: &dyn TunerBus, gain: GainSetting) -> Result<Option<f64>> {
        match gain {
            GainSetting::Auto => {
                // LNA and mixer loops on, VGA fixed at ~26.5 dB.
                self.write_reg_mask(bus, 0x05, 0, 0x10)?;
                self.write_reg_mask(bus, 0x07, 0x10, 0x10)?;
                self.write_reg_mask(bus, 0x0c, 0x0b, 0x9f)?;
                Ok(None)
            }
            GainSetting::Manual { total_db } => {
                let want = tables::snap_gain_tenth_db(total_db);

                // LNA and mixer loops off, VGA where it was left (~16.3 dB,
                // 0x08, until the IF stage is set on its own).
                self.write_reg_mask(bus, 0x05, 0x10, 0x10)?;
                self.write_reg_mask(bus, 0x07, 0, 0x10)?;
                let _ = self.read(bus, 4)?;
                self.write_reg_mask(bus, 0x0c, self.vga_index, 0x9f)?;

                // Walk the two stages alternately until the target is met.
                let (lna_index, mix_index) = ladder_indices(f64::from(want) / 10.0);
                let total = stage_db(Stage::Lna, lna_index) + stage_db(Stage::Mixer, mix_index);

                self.write_reg_mask(bus, 0x05, lna_index, 0x0f)?;
                self.write_reg_mask(bus, 0x07, mix_index, 0x0f)?;
                Ok(Some(total))
            }
        }
    }

    /// Set one stage to `index` on its own ladder, its automatic loop off.
    /// The other stages stay where they are. Returns the stage's gain (in
    /// steps for the VGA).
    pub fn set_stage(&mut self, bus: &dyn TunerBus, stage: Stage, index: u8) -> Result<f64> {
        let index = index.min(STAGE_MAX_INDEX);
        match stage {
            Stage::Lna => {
                self.write_reg_mask(bus, 0x05, 0x10 | index, 0x1f)?;
            }
            Stage::Mixer => {
                self.write_reg_mask(bus, 0x07, index, 0x1f)?;
            }
            Stage::Vga => {
                self.vga_index = index;
                self.write_reg_mask(bus, 0x0c, index, 0x9f)?;
            }
        }
        Ok(stage_db(stage, index))
    }

    /// The gain values this tuner can produce, in dB.
    pub fn gains_db() -> impl Iterator<Item = f64> {
        GAINS_TENTH_DB.iter().map(|g| *g as f64 / 10.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::MockBus;

    /// The stages one by one: the measured ladders summed, the combined
    /// ladder landing on exactly the gains librtlsdr lists, and a gain asked
    /// of a stage taking its nearest step.
    #[test]
    fn stages_add_up_to_the_ladder() {
        assert_eq!(stage_db(Stage::Lna, 0), 0.0);
        assert!((stage_db(Stage::Lna, 15) - 33.5).abs() < 1e-9);
        assert!((stage_db(Stage::Mixer, 14) - 16.1).abs() < 1e-9);
        assert_eq!(stage_db(Stage::Vga, 9), 9.0);
        for g in tables::GAINS_TENTH_DB {
            let (l, m) = ladder_indices(f64::from(g) / 10.0);
            let sum = stage_db(Stage::Lna, l) + stage_db(Stage::Mixer, m);
            assert!((sum - f64::from(g) / 10.0).abs() < 0.05, "{g}: {l}/{m} gives {sum}");
        }
        assert_eq!(stage_index_for(Stage::Lna, 33.5), 15);
        assert_eq!(stage_index_for(Stage::Lna, 0.4), 0);
        assert_eq!(stage_index_for(Stage::Vga, 12.2), 12);
    }

    /// Every register this driver writes to bring an R828D up on 145 MHz with
    /// the 8 MHz IF filter — which is exactly the RX-888's VHF sequence.
    ///
    /// Captured from this code before the tuner driver was lifted out behind
    /// [`TunerBus`], so it is the contract that extraction had to preserve: a
    /// transcription of someone else's register sequence has no other way to
    /// say "still correct". Lines are `addr reg payload…`, and they record what
    /// the *driver* asked for — chunking belongs to the bus now, and is checked
    /// separately in [`crate::tuner::bus`].
    ///
    /// If this fails, the question is not "what is the new value" but "which
    /// upstream commit changed, and did the radio get better or deafer".
    const R828D_145MHZ_TRACE: &str = "\
74 05 83 32 75 c0 40 d6 6c f5 63 75 68 6c 83 80 00 0f 00 c0 30 48 cc 60 00 54 ae 4a c0 00 00 00
74 0c f0
74 13 31
74 1d 86
74 0f 6c
74 10 8c 83 88 31 ca 00 00
74 1a 68
74 0b 7c
74 0b 6c
74 0f 68
74 0a d5
74 0b 6b
74 06 12
74 1e 6a
74 05 03
74 1f 40
74 19 ec
74 1d c5
74 1c 24
74 0d 53
74 11 bb
74 1c 20
74 1a 78
74 1d dd
74 1c 24
74 1e 6e
74 1a 68
74 0b 0b
74 1a 2a
74 1b 14
74 10 84
74 1a 22
74 10 64 bb 80 31 4f f6 c8
74 1a 2a
74 05 63";

    #[test]
    fn the_r828d_bring_up_writes_exactly_what_it_always_has() {
        let bus = MockBus::healthy();
        let mut t = R82xx::new(Chip::R828D, false, 0);
        t.init(&bus).expect("init");
        let int_freq = t.set_bandwidth(&bus, 8_000_000.0).expect("bandwidth");
        t.set_freq(&bus, 145_000_000.0).expect("freq");

        assert_eq!(bus.trace().join("\n"), R828D_145MHZ_TRACE);
        // The 8 MHz filter is what puts the IF here, and the RX-888's VHF path
        // downconverts to this exact frequency. Upstream calls it
        // `R828D_IF_CARRIER`.
        assert_eq!(int_freq, 4_570_000.0);
    }

    /// A Blog V4 is an R828D that does *not* take the R828D crystal. Getting
    /// this backwards is silent: the PLL locks, the dongle streams, and every
    /// frequency is out by 28.8/16.
    #[test]
    fn a_blog_v4_keeps_the_28_8_mhz_reference() {
        assert_eq!(R82xx::reference_xtal_hz(Chip::R828D, true), 28_800_000.0);
        assert_eq!(R82xx::reference_xtal_hz(Chip::R828D, false), R828D_XTAL_HZ);
        assert_eq!(R82xx::reference_xtal_hz(Chip::R820T, false), R820T_XTAL_HZ);
    }

    /// Only a V4 upconverts, and only below the crossover. The 7.056 MHz here
    /// is the dial from the bug report that started this: a V4 parked on 40 m
    /// must put its tuner at 35.856 MHz, not refuse to reach it.
    #[test]
    fn a_blog_v4_upconverts_below_the_crossover() {
        let v4 = R82xx::new(Chip::R828D, true, 0);
        assert_eq!(v4.tuned_freq_hz(7_056_158.0), 7_056_158.0 + 28_800_000.0);
        assert_eq!(v4.tuned_freq_hz(14.1e6), 14.1e6 + 28_800_000.0);
        // At and above the crossover the tuner reaches directly.
        assert_eq!(v4.tuned_freq_hz(V4_HF_CROSSOVER_HZ), V4_HF_CROSSOVER_HZ);
        assert_eq!(v4.tuned_freq_hz(100e6), 100e6);

        // A stock R828D has no upconverter to offset for; HF there is the
        // demodulator's problem, via direct sampling.
        let plain = R82xx::new(Chip::R828D, false, 0);
        assert_eq!(plain.tuned_freq_hz(7_056_158.0), 7_056_158.0);
    }

    /// The register pair that carries the tracking filter, read out of the
    /// shadow: `(rf_mux_ploy & 0xc3, tf_c)`. Bypass is `0x40` with no capacitor.
    fn tracking_filter(t: &R82xx) -> (u8, u8) {
        let ploy = t.regs[R82xx::shadow_index(0x1a).expect("0x1a is shadowed")] & 0xc3;
        let tf_c = t.regs[R82xx::shadow_index(0x1b).expect("0x1b is shadowed")];
        (ploy, tf_c)
    }

    /// On HF a V4 hears the upconverter's output, not the antenna, so the
    /// R828D's preselector is pure insertion loss ahead of the LNA — and loss
    /// there is noise figure, which is what "S9 of hiss with nothing connected"
    /// is made of. It must be bypassed, and it must *stay* bypassed: `set_mux`
    /// rewrites both registers from the tracking-filter table on every tune, so
    /// a bypass applied only when the band changes lasts one dial nudge.
    #[test]
    fn a_blog_v4_bypasses_the_tracking_filter_on_every_hf_tune() {
        let bus = MockBus::healthy();
        let mut t = R82xx::new(Chip::R828D, true, 0);
        t.init(&bus).expect("init");
        t.set_bandwidth(&bus, 8_000_000.0).expect("bandwidth");

        // First tune into HF — the band change.
        t.set_freq(&bus, 7_056_158.0).expect("freq");
        assert_eq!(tracking_filter(&t), (0x40, 0x00), "bypassed on entering HF");

        // A second HF dial move, with no band change to trigger the switch.
        t.set_freq(&bus, 14_236_000.0).expect("freq");
        assert_eq!(tracking_filter(&t), (0x40, 0x00), "still bypassed after retuning");

        // Above the crossover the tuner really is looking at the antenna
        // through its own preselector, and the tracking filter belongs in.
        t.set_freq(&bus, 145_000_000.0).expect("freq");
        assert_ne!(tracking_filter(&t), (0x40, 0x00), "tracking filter back in circuit on VHF");
    }
}
