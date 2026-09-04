//! The AD9361/AD9363 front end, addressed through IIO attributes.
//!
//! Three IIO devices make up a Pluto. `ad9361-phy` is the transceiver itself —
//! local oscillators, sample rate, analog filter, gains, AGC mode, port
//! selection. `cf-ad9361-lpc` and `cf-ad9361-dds-core-lpc` are the receive and
//! transmit DMA buffers, which carry samples and nothing else.
//!
//! # Read the limits, do not assume them
//!
//! Every limit this backend reports to the UI is read off the device through an
//! `_available` attribute. That is not fastidiousness: the same firmware runs on
//! a stock AD9363 board (325 MHz–3.8 GHz) and on one unlocked to AD9364
//! (70 MHz–6 GHz), and the receive gain range moves with frequency. Hardcoding
//! either would mean half the Plutos in circulation refusing frequencies they
//! can reach, or accepting ones they cannot.
//!
//! Where a firmware publishes no `_available`, a fallback is used **and
//! recorded** in [`PlutoLimits::assumed`], which the source adapter surfaces
//! through `IqSource::open_status` — a limit that was guessed should say so
//! rather than look like a measurement.
//!
//! # The two channels called `voltage0`
//!
//! In IIO the AD9361's receive chain is `INPUT voltage0` and its transmit chain
//! `OUTPUT voltage0`. They share a name and several attribute names
//! (`hardwaregain`, `rf_bandwidth`, `sampling_frequency`) and are entirely
//! different things — receive `hardwaregain` is gain in dB, transmit
//! `hardwaregain` is *attenuation* expressed as negative dB.

use sdroxide_types::{PlutoDuplex, PlutoPtt};

use crate::context::{Context, Device, ScanFormat};
use crate::error::{Error, Result};
use crate::iiod::Connection;

/// The transceiver's control device.
pub const PHY: &str = "ad9361-phy";
/// The receive DMA buffer.
pub const RX_BUFFER: &str = "cf-ad9361-lpc";
/// The transmit DMA buffer.
pub const TX_BUFFER: &str = "cf-ad9361-dds-core-lpc";

/// First receive chain: `INPUT voltage0` on the phy.
const RX_CHAN: (bool, &str) = (false, "voltage0");
/// Second receive chain, where the firmware has one: `INPUT voltage1`.
const RX2_CHAN: (bool, &str) = (false, "voltage1");
/// Transmit chain: `OUTPUT voltage0` on the phy.
const TX_CHAN: (bool, &str) = (true, "voltage0");

/// The control channel for receive chain `chain` (0 or 1).
/// `-EOPNOTSUPP`, which is what the AD9361 driver answers to a write of a
/// register the part currently owns — the receive gain of a chain that is not
/// in manual gain control.
const EOPNOTSUPP: i64 = -95;

fn rx_chan(chain: u8) -> (bool, &'static str) {
    if chain == 0 { RX_CHAN } else { RX2_CHAN }
}
/// Receive local oscillator.
const RX_LO: (bool, &str) = (true, "altvoltage0");
/// Transmit local oscillator.
const TX_LO: (bool, &str) = (true, "altvoltage1");

/// The device-tree property that picks the duplex: `1` for FDD, `0` for TDD. A
/// debug attribute, and one that does nothing until [`INITIALIZE`] is written.
const FDD_ENABLE: &str = "adi,frequency-division-duplex-mode-enable";
/// Writing `1` here re-runs the driver's whole setup from its (just modified)
/// copy of the device tree. Everything configured before it — sample rate,
/// filter, gains, local oscillators — goes back to the board's own defaults,
/// which is why the transmit-receive switching is set up before any of that
/// rather than after it.
const INITIALIZE: &str = "initialize";
/// The properties that give GPO0 and GPO1 to an external LNA instead. Cleared
/// when the PTT pair *is* GPO0/GPO1, because one pin cannot do both jobs —
/// which is the reason [`PlutoPtt::Gpo23`] exists.
const ELNA_GPO: [&str; 2] =
    ["adi,elna-rx1-gpo0-control-enable", "adi,elna-rx2-gpo1-control-enable"];
/// How many general-purpose output pins an AD9361 has.
const GPO_PINS: u8 = 4;
/// The four slave bits this backend ever sets — one per pin, in the direction
/// [`PlutoPtt`] gives it. Finding one of them set on a board that is not in FDD
/// is how a session's own leftovers are told apart from a board somebody put in
/// TDD for reasons of their own.
const PTT_SLAVE_ATTRS: [&str; 4] = [
    "adi,gpo0-slave-rx-enable",
    "adi,gpo1-slave-tx-enable",
    "adi,gpo2-slave-rx-enable",
    "adi,gpo3-slave-tx-enable",
];

/// AD9363 tuning range, the conservative fallback when the device publishes no
/// `frequency_available`.
const AD9363_LO_HZ: (f64, f64) = (325_000_000.0, 3_800_000_000.0);
/// AD9364 tuning range — what an unlocked Pluto reaches.
const AD9364_LO_HZ: (f64, f64) = (70_000_000.0, 6_000_000_000.0);
/// AD9361 sample-rate range. The floor is the internal FIR decimator's output;
/// the ceiling is the part's, not the link's — see `PlutoConfig::SAMPLE_RATES`
/// for what a USB Ethernet gadget can actually carry.
const RATE_HZ: (f64, f64) = (521_000.0, 61_440_000.0);
/// AD9361 analog filter range.
const BANDWIDTH_HZ: (f64, f64) = (200_000.0, 56_000_000.0);
/// Receive gain, dB: (min, max, step).
const RX_GAIN_DB: (f64, f64, f64) = (0.0, 71.0, 1.0);
/// Transmit attenuation expressed as gain, dB: (min, max, step).
const TX_GAIN_DB: (f64, f64, f64) = (-89.75, 0.0, 0.25);

/// What the device says it can do.
#[derive(Debug, Clone)]
pub struct PlutoLimits {
    pub rx_lo_hz: (f64, f64),
    pub tx_lo_hz: (f64, f64),
    pub sample_rate_hz: (f64, f64),
    pub rf_bandwidth_hz: (f64, f64),
    /// (min, max, step)
    pub rx_gain_db: (f64, f64, f64),
    /// (min, max, step), as gain — so the numbers are negative.
    pub tx_gain_db: (f64, f64, f64),
    pub rx_ports: Vec<String>,
    pub tx_ports: Vec<String>,
    pub agc_modes: Vec<String>,
    /// Limits that came from the fallback table rather than the device, named
    /// so the operator can be told which figures are guesses.
    pub assumed: Vec<String>,
}

impl PlutoLimits {
    /// The fallback set, before anything has been read.
    fn fallback(ad9364: bool) -> PlutoLimits {
        let lo = if ad9364 { AD9364_LO_HZ } else { AD9363_LO_HZ };
        PlutoLimits {
            rx_lo_hz: lo,
            tx_lo_hz: lo,
            sample_rate_hz: RATE_HZ,
            rf_bandwidth_hz: BANDWIDTH_HZ,
            rx_gain_db: RX_GAIN_DB,
            tx_gain_db: TX_GAIN_DB,
            rx_ports: Vec::new(),
            tx_ports: Vec::new(),
            agc_modes: vec![
                "manual".into(),
                "slow_attack".into(),
                "fast_attack".into(),
                "hybrid".into(),
            ],
            assumed: Vec::new(),
        }
    }

    /// A sentence for `IqSource::open_status`, or `None` when everything came
    /// from the device.
    pub fn assumption_notice(&self) -> Option<String> {
        if self.assumed.is_empty() {
            return None;
        }
        Some(format!(
            "PlutoSDR: this firmware does not publish {} — sdroxide is assuming the \
             standard AD936x figures, which may not match your board",
            self.assumed.join(", ")
        ))
    }
}

/// Everything resolved once at open time: the wire ids of the devices and
/// channels, the sample layouts, and the device's own limits.
#[derive(Debug, Clone)]
pub struct Phy {
    pub phy_id: String,
    pub rx_buffer_id: String,
    pub tx_buffer_id: String,
    /// Wire layouts of the receive buffer's usable scan elements, in scan
    /// order — I then Q per chain, so a stock Pluto has two entries and a
    /// 2R2T firmware four. How many are actually enabled is decided per
    /// session ([`Phy::rx_pairs_available`] is the ceiling); the receive
    /// thread opens the buffer with the first `pairs × 2` of these.
    pub rx_scan: Vec<ScanFormat>,
    /// … and the transmit buffer's first I/Q pair. Transmit stays on the
    /// first chain whatever the receive side does.
    pub tx_scan: Vec<ScanFormat>,
    /// Whether the phy exposes the second receive chain's control channel
    /// (`in_voltage1` with a gain register). Streaming the second pair works
    /// without it; per-chain gain/port control does not.
    pub rx2_control: bool,
    /// DDS tone channels on the transmit buffer, which have to be silenced
    /// before DMA transmit will be heard.
    pub dds_channels: Vec<String>,
    /// The phy's debug attributes — the driver's copy of the device tree
    /// (`adi,…`) and the `initialize` trigger. Kept so the transmit-receive
    /// switching in [`Phy::setup_tr_switching`] can look before it writes: a
    /// firmware that publishes none is a board this cannot be done on, which
    /// is worth saying plainly rather than as four rejected writes.
    pub debug_attrs: Vec<String>,
    pub limits: PlutoLimits,
    pub model: String,
    pub firmware: String,
    pub serial: String,
}

/// What [`Phy::setup_tr_switching`] left the board in.
pub struct TrSwitching {
    /// The part is in TDD: it does one direction at a time, and `ensm_mode`
    /// is now this driver's job at every key-up and key-down. False when
    /// nothing was asked for, and false when the board would not take it —
    /// either way nobody should be driving a state machine that is still in
    /// FDD.
    pub tdd: bool,
    /// What the operator should be told, if anything.
    pub warnings: Vec<String>,
}

impl Phy {
    /// Resolve the devices and read the limits. `addr` only decorates errors.
    pub fn probe(conn: &mut Connection, ctx: &Context, addr: &str) -> Result<Phy> {
        let phy = ctx.require(PHY, addr)?;
        let rx = ctx.require(RX_BUFFER, addr)?;
        let tx = ctx.require(TX_BUFFER, addr)?;

        // Receive keeps up to two chains' worth of scan elements; transmit
        // stays on the first pair whatever the firmware carries.
        let (rx_scan, rx_note) = iq_scan(RX_BUFFER, rx, 2)?;
        let (tx_scan, tx_note) = iq_scan(TX_BUFFER, tx, 1)?;
        for note in [&rx_note, &tx_note].into_iter().flatten() {
            tracing::info!("PlutoSDR: {note}");
            conn.note(note);
        }
        let rx2_control =
            phy.channel(RX2_CHAN.1, RX2_CHAN.0).is_some_and(|c| c.has_attr("hardwaregain"));
        let dds_channels: Vec<String> = tx
            .channels
            .iter()
            .filter(|c| c.output && c.scan.is_none() && c.has_attr("raw"))
            .map(|c| c.id.clone())
            .collect();

        let mut limits = PlutoLimits::fallback(ctx.is_ad9364());
        let phy_id = phy.id.clone();

        // Each of these only issues a READ when the context says the attribute
        // is there — an -ENOENT per missing attribute would fill the trace with
        // noise on exactly the firmwares where the trace matters most.
        let mut missing = Vec::new();
        if let Some(r) = read_range(conn, &phy_id, phy, RX_LO, "frequency_available")? {
            limits.rx_lo_hz = (r.0, r.2);
            limits.tx_lo_hz = (r.0, r.2);
        } else {
            missing.push("a tuning range");
        }
        if let Some(r) = read_range(conn, &phy_id, phy, TX_LO, "frequency_available")? {
            limits.tx_lo_hz = (r.0, r.2);
        }
        // Not counted as an assumption worth telling the operator about: no
        // AD9361 driver publishes this, and the range is a property of the
        // silicon rather than of the board, so the fallback is simply the right
        // answer. Flagging it would put a warning on every Pluto ever made.
        if let Some(r) = read_range(conn, &phy_id, phy, RX_CHAN, "sampling_frequency_available")? {
            limits.sample_rate_hz = (r.0, r.2);
        }
        if let Some(r) = read_range(conn, &phy_id, phy, RX_CHAN, "rf_bandwidth_available")? {
            limits.rf_bandwidth_hz = (r.0, r.2);
        }
        if let Some(r) = read_range(conn, &phy_id, phy, RX_CHAN, "hardwaregain_available")? {
            limits.rx_gain_db = (r.0, r.2, r.1);
        } else {
            missing.push("a receive gain range");
        }
        if let Some(r) = read_range(conn, &phy_id, phy, TX_CHAN, "hardwaregain_available")? {
            limits.tx_gain_db = (r.0, r.2, r.1);
        } else {
            missing.push("a transmit attenuation range");
        }
        limits.rx_ports = read_list(conn, &phy_id, phy, RX_CHAN, "rf_port_select_available")?;
        limits.tx_ports = read_list(conn, &phy_id, phy, TX_CHAN, "rf_port_select_available")?;
        let modes = read_list(conn, &phy_id, phy, RX_CHAN, "gain_control_mode_available")?;
        if !modes.is_empty() {
            limits.agc_modes = modes;
        }
        limits.assumed = missing.into_iter().map(str::to_string).collect();

        tracing::info!(
            "PlutoSDR: {} firmware {} — RX LO {:.3}–{:.3} MHz, rate {:.0}–{:.0} ksps, \
             RX gain {:.1}–{:.1} dB, TX gain {:.2}–{:.2} dB",
            ctx.hw_model(),
            ctx.fw_version(),
            limits.rx_lo_hz.0 / 1e6,
            limits.rx_lo_hz.1 / 1e6,
            limits.sample_rate_hz.0 / 1e3,
            limits.sample_rate_hz.1 / 1e3,
            limits.rx_gain_db.0,
            limits.rx_gain_db.1,
            limits.tx_gain_db.0,
            limits.tx_gain_db.1,
        );
        if let Some(notice) = limits.assumption_notice() {
            tracing::warn!("{notice}");
        }

        Ok(Phy {
            phy_id,
            rx_buffer_id: rx.id.clone(),
            tx_buffer_id: tx.id.clone(),
            rx_scan,
            tx_scan,
            rx2_control,
            dds_channels,
            debug_attrs: phy.debug_attrs.clone(),
            limits,
            model: ctx.hw_model().to_string(),
            firmware: ctx.fw_version().to_string(),
            serial: ctx.hw_serial().to_string(),
        })
    }

    /// How many receive chains this firmware can stream: the I/Q pairs its
    /// buffer carries (a stock Pluto 1, a 2R2T build 2).
    pub fn rx_pairs_available(&self) -> usize {
        self.rx_scan.len() / 2
    }

    /// Bytes one sample *set* occupies on the receive buffer when `pairs`
    /// chains are enabled.
    pub fn rx_sample_bytes(&self, pairs: usize) -> usize {
        self.rx_scan[..pairs * 2].iter().map(|f| f.bytes()).sum()
    }

    /// … and on the transmit buffer.
    pub fn tx_sample_bytes(&self) -> usize {
        self.tx_scan.iter().map(|f| f.bytes()).sum()
    }

    // ---- control --------------------------------------------------------

    /// Set the receive sample rate, returning what the hardware settled on.
    ///
    /// On the AD9361 the receive and transmit paths hang off the same clock
    /// tree, so this moves both; the transmit rate is read back rather than
    /// assumed.
    pub fn set_sample_rate(&self, conn: &mut Connection, hz: f64) -> Result<f64> {
        let (min, max) = self.limits.sample_rate_hz;
        let want = hz.clamp(min, max).round() as i64;
        self.write_on_a_floor(conn, RX_CHAN, "sampling_frequency", want, min)?;
        self.sample_rate(conn)
    }

    /// Write an integer attribute that may be sitting exactly on the floor the
    /// device published, retrying one step up when the device then rejects its
    /// own stated minimum as out of range.
    ///
    /// IIO prints an `_available` range through integer division, so a floor
    /// that is really a fraction arrives a hair low — and the driver that
    /// printed it still enforces the true value. An AD9361 with its FIR
    /// bypassed publishes `[2083333 1 61440000]` for a real minimum of
    /// 25 MHz / 12 = 2083333.33: its clock-chain solver accepts a rate only if
    /// `rate × 12` clears the ADC's 25 MHz floor, and 2083333 × 12 is four
    /// hertz short. So a stock Pluto refuses the rate it just asked for, and
    /// every sdroxide sample rate at or below 2.083 Msps — which was all three
    /// of the lowest offered, including the default — died here with EINVAL
    /// before a single buffer was opened.
    ///
    /// One retry is enough and no more is justified: truncation cannot lose as
    /// much as a whole unit, so the next integer up is necessarily inside the
    /// real range. It is only ever attempted when the value *is* the published
    /// floor and the device called it invalid.
    fn write_on_a_floor(
        &self,
        conn: &mut Connection,
        chan: (bool, &str),
        attr: &str,
        value: i64,
        floor: f64,
    ) -> Result<()> {
        /// The errno a driver answers when a value is outside what it accepts.
        const EINVAL: i64 = -22;
        let attempt = conn.write_attr(&self.phy_id, Some(chan), attr, &value.to_string());
        // Only the floor itself is a candidate: anywhere else in the range an
        // EINVAL means what it says.
        let on_the_floor = (value as f64 - floor).abs() < 1.0;
        match attempt {
            Err(Error::Remote { code: EINVAL, .. }) if on_the_floor => {
                tracing::info!(
                    "PlutoSDR: {attr} {value} was refused as out of range even though the \
                     device published it as the minimum — its true floor is a fraction above \
                     that, so trying {}",
                    value + 1
                );
                conn.write_attr(&self.phy_id, Some(chan), attr, &(value + 1).to_string())
            }
            other => other,
        }
    }

    pub fn sample_rate(&self, conn: &mut Connection) -> Result<f64> {
        read_f64(conn, &self.phy_id, RX_CHAN, "sampling_frequency")
    }

    /// The transmit chain's rate. Normally identical to the receive one — both
    /// hang off the same clock tree — but read rather than assumed, because the
    /// engine sizes its transmit interpolator from this number and a wrong one
    /// puts the signal on the wrong frequency.
    pub fn tx_sample_rate(&self, conn: &mut Connection) -> Result<f64> {
        read_f64(conn, &self.phy_id, TX_CHAN, "sampling_frequency")
    }

    /// Set the analog filter bandwidth on both chains, returning the receive
    /// side's actual value.
    ///
    /// This is load-bearing for more than selectivity: the engine parks the LO
    /// a quarter of a span away from the VFO to keep the signal off a zero-IF
    /// part's DC spike, and that offset is abandoned if the analog filter is
    /// too narrow to pass it. Leaving the AD9361's default here is what makes
    /// the offset silently do nothing.
    pub fn set_bandwidth(&self, conn: &mut Connection, hz: f64) -> Result<f64> {
        let hz = hz.clamp(self.limits.rf_bandwidth_hz.0, self.limits.rf_bandwidth_hz.1);
        let value = format!("{}", hz.round() as i64);
        conn.write_attr(&self.phy_id, Some(RX_CHAN), "rf_bandwidth", &value)?;
        // The transmit filter is a separate register; a failure there must not
        // stop receive from coming up.
        if let Err(e) = conn.write_attr(&self.phy_id, Some(TX_CHAN), "rf_bandwidth", &value) {
            tracing::warn!("PlutoSDR: could not set the transmit filter bandwidth: {e}");
        }
        read_f64(conn, &self.phy_id, RX_CHAN, "rf_bandwidth")
    }

    pub fn set_rx_lo(&self, conn: &mut Connection, hz: f64) -> Result<()> {
        let hz = hz.clamp(self.limits.rx_lo_hz.0, self.limits.rx_lo_hz.1);
        conn.write_attr(&self.phy_id, Some(RX_LO), "frequency", &format!("{}", hz.round() as i64))
    }

    pub fn set_tx_lo(&self, conn: &mut Connection, hz: f64) -> Result<()> {
        let hz = hz.clamp(self.limits.tx_lo_hz.0, self.limits.tx_lo_hz.1);
        conn.write_attr(&self.phy_id, Some(TX_LO), "frequency", &format!("{}", hz.round() as i64))
    }

    /// Where the transmit synthesiser actually is.
    ///
    /// A read, not a guess, and it costs one round trip: `key_up` uses it to
    /// decide whether this over needs the synthesiser moved at all, and moving
    /// it is the expensive half of keying an AD9361 (see that function). What
    /// the part reports is the frequency it *achieved*, which is quantised to
    /// the RF PLL's own step and so is not always the number that was written.
    pub fn tx_lo(&self, conn: &mut Connection) -> Result<f64> {
        let text = conn.read_attr(&self.phy_id, Some(TX_LO), "frequency")?;
        text.trim()
            .parse::<f64>()
            .map_err(|_| Error::Msg(format!("the transmit oscillator answered {text:?}")))
    }

    /// The one gain-control mode in which the operator, rather than the
    /// AD9361, owns the receive gain register.
    pub const MANUAL_AGC: &'static str = "manual";

    /// `manual`, `slow_attack`, `fast_attack` or `hybrid` for receive chain
    /// `chain`. An unknown mode is refused here rather than on the wire so the
    /// message names the modes the device actually offers; a chain the
    /// firmware has no control channel for is skipped with a note rather than
    /// refused — its stream still works, at whatever the AD9361 does.
    pub fn set_agc_mode(&self, conn: &mut Connection, chain: u8, mode: &str) -> Result<()> {
        if !self.limits.agc_modes.iter().any(|m| m == mode) {
            return Err(Error::Unsupported(format!(
                "this Pluto has no AGC mode {mode:?} — it offers {}",
                self.limits.agc_modes.join(", ")
            )));
        }
        if chain > 0 && !self.rx2_control {
            tracing::debug!("PlutoSDR: no control channel for receive chain {chain}; AGC held");
            return Ok(());
        }
        conn.write_attr(&self.phy_id, Some(rx_chan(chain)), "gain_control_mode", mode)
    }

    /// Receive gain in dB — **only writable while the gain-control mode is
    /// `manual`**, which is why `mode` is a parameter rather than something the
    /// caller is trusted to have checked.
    ///
    /// In any attack mode the AD9361 owns this register and the driver answers
    /// `-EOPNOTSUPP` (-95) rather than ignoring the write. Since the default
    /// AGC here is slow attack, sending it unconditionally aborted the open on
    /// every radio that had not been switched to manual — and the workaround,
    /// pinning the AGC to manual, left the receiver at a fixed 40 dB of an
    /// available 71 and looking deaf.
    ///
    /// Skipping is not the same as losing the value: the caller keeps it and
    /// replays it on the way back into manual (see `control_thread`).
    ///
    /// `mode` is what the *caller* believes the chain is in, and belief can be
    /// wrong: the AD9361 keeps a mode per chain, another program can move one,
    /// and a 2R2T firmware boots the second chain wherever its device tree left
    /// it rather than where the first one was put. So a write the driver
    /// refuses with `-EOPNOTSUPP` is answered by asserting the mode and trying
    /// once more, instead of leaving the operator with a slider that logs an
    /// errno and does nothing (issue #311).
    pub fn set_rx_gain(&self, conn: &mut Connection, chain: u8, mode: &str, db: f64) -> Result<()> {
        if mode != Phy::MANUAL_AGC {
            tracing::debug!(
                "PlutoSDR: holding the {db:.1} dB receive gain — the AGC is in {mode}, where \
                 the AD9361 owns that register"
            );
            return Ok(());
        }
        if chain > 0 && !self.rx2_control {
            tracing::debug!("PlutoSDR: no control channel for receive chain {chain}; gain held");
            return Ok(());
        }
        let db = db.clamp(self.limits.rx_gain_db.0, self.limits.rx_gain_db.1);
        match self.write_rx_gain(conn, chain, db) {
            Err(e) if e.remote_code() == Some(EOPNOTSUPP) => {
                tracing::debug!(
                    "PlutoSDR: receive chain {chain} would not take a gain, so it is not in \
                     {mode} after all — putting it there and trying again"
                );
                self.set_agc_mode(conn, chain, mode)?;
                self.write_rx_gain(conn, chain, db)
            }
            other => other,
        }
    }

    fn write_rx_gain(&self, conn: &mut Connection, chain: u8, db: f64) -> Result<()> {
        conn.write_attr(&self.phy_id, Some(rx_chan(chain)), "hardwaregain", &format!("{db:.6}"))
    }

    /// Transmit gain in dB — negative, because on the AD9361 this is
    /// attenuation. `0` is full output and `-89.75` is as close to off as the
    /// part gets.
    pub fn set_tx_gain(&self, conn: &mut Connection, db: f64) -> Result<()> {
        let db = db.clamp(self.limits.tx_gain_db.0, self.limits.tx_gain_db.1);
        conn.write_attr(&self.phy_id, Some(TX_CHAN), "hardwaregain", &format!("{db:.6}"))
    }

    /// Put the transmitter at its quietest setting.
    ///
    /// Called before the source is handed to the engine, so that whatever the
    /// configuration says, a first key-up cannot emit unexpected power. The
    /// SoapySDR backend does the same thing for the same reason.
    pub fn silence_transmitter(&self, conn: &mut Connection) -> Result<()> {
        self.set_tx_gain(conn, self.limits.tx_gain_db.0)
    }

    pub fn set_rx_port(&self, conn: &mut Connection, chain: u8, port: &str) -> Result<()> {
        if chain > 0 && !self.rx2_control {
            tracing::debug!("PlutoSDR: no control channel for receive chain {chain}; port held");
            return Ok(());
        }
        conn.write_attr(&self.phy_id, Some(rx_chan(chain)), "rf_port_select", port)
    }

    pub fn set_tx_port(&self, conn: &mut Connection, port: &str) -> Result<()> {
        conn.write_attr(&self.phy_id, Some(TX_CHAN), "rf_port_select", port)
    }

    pub fn rx_port(&self, conn: &mut Connection) -> Result<String> {
        conn.read_attr(&self.phy_id, Some(RX_CHAN), "rf_port_select")
    }

    /// The AD9361's enable-state-machine mode — `fdd` or `tdd`, a device-level
    /// attribute rather than a channel one.
    ///
    /// In FDD the part receives and transmits at once, each direction with its
    /// own synthesiser; in TDD it does one at a time and the receiver is dead
    /// for the length of an over whatever the link can carry. Two callers want
    /// to know: full duplex, which collides with TDD and says so rather than
    /// reconfiguring a board out from under its owner, and
    /// [`Phy::setup_tr_switching`], which uses it to recognise a board an
    /// earlier session left in TDD.
    pub fn ensm_mode(&self, conn: &mut Connection) -> Result<String> {
        conn.read_attr(&self.phy_id, None, "ensm_mode")
    }

    /// Put the enable state machine into `mode` — `rx` or `tx` on a board this
    /// backend has put in TDD.
    ///
    /// In FDD nobody writes this: the part is in both states at once and the
    /// mode is a fact about the board, not a control. In TDD it is the control
    /// — the one that decides which direction is live, and with it which of the
    /// slaved GPO pins is high, which is to say whether the external amplifier
    /// is keyed.
    pub fn set_ensm_mode(&self, conn: &mut Connection, mode: &str) -> Result<()> {
        conn.write_attr(&self.phy_id, None, "ensm_mode", mode)
    }

    pub fn has_debug_attr(&self, attr: &str) -> bool {
        self.debug_attrs.iter().any(|a| a == attr)
    }

    /// Put the part in the duplex `duplex` asks for and slave `ptt`'s pair of
    /// GPO pins to the receive and transmit enable lines, so an external power
    /// amplifier, LNA or transmit-receive switch follows the radio with no host
    /// software in the loop.
    ///
    /// # Why this is not just two attribute writes
    ///
    /// The `adi,…` properties are the driver's copy of the device tree, and
    /// writing one changes nothing until [`INITIALIZE`] re-runs the AD9361's
    /// whole setup from it — which also puts the sample rate, the filter, the
    /// gains and both oscillators back to the board's defaults. So this runs
    /// *first*, before the front end is configured, and everything after it
    /// lands on a part that has already been reinitialised.
    ///
    /// The GPO pins follow the ENSM's enable lines, and in FDD both of those
    /// are asserted for the whole session: a pin slaved to transmit would go
    /// high at open and stay there. That is why choosing a pair implies TDD
    /// rather than being refused alongside FDD — the operator asked for a
    /// keyed amplifier, and TDD is what keys it.
    ///
    /// A device that refuses one of these writes is not a failed open. The
    /// radio still works; its PTT pins do not, and the operator is told which.
    ///
    /// # Putting it back
    ///
    /// These properties live in the radio until it is rebooted, so turning the
    /// pins back off has to undo them — otherwise the board stays in TDD with
    /// nobody driving its state machine, which is a transmitter that does
    /// nothing and no way out but a power cycle. What is undone is exactly what
    /// this writes: a board found in TDD with one of [`PTT_SLAVE_ATTRS`] set is
    /// put back into FDD, and one in TDD with no pin slaved is somebody else's
    /// arrangement and is left alone.
    pub fn setup_tr_switching(
        &self,
        conn: &mut Connection,
        duplex: PlutoDuplex,
        ptt: PlutoPtt,
    ) -> Result<TrSwitching> {
        let mut out = TrSwitching { tdd: false, warnings: Vec::new() };
        let pins = ptt.pins();
        // The pins are what decides the duplex. They follow the enable lines,
        // and in FDD both of those are asserted for the whole session, so a
        // pair chosen alongside FDD would simply never move.
        let want_tdd = duplex == PlutoDuplex::Tdd || pins.is_some();
        if want_tdd && duplex == PlutoDuplex::Fdd {
            let msg = format!(
                "PlutoSDR: {} keys from the enable-state machine, which needs TDD — so \
                 the duplex setting of FDD is being overridden. Set the PTT pins to \
                 Off to run in FDD",
                ptt.label()
            );
            tracing::warn!("{msg}");
            out.warnings.push(msg);
        }
        if !want_tdd {
            // Nothing asked for — usually. Leave a board that is already in
            // FDD exactly as it booted: that is every Pluto that has never had
            // these settings opened, and reinitialising one unasked is not
            // this driver's business.
            //
            // The exception is a board this very feature left in TDD, which
            // an operator turning the PTT pins back off would otherwise be
            // stuck with: nothing here would drive its state machine, so its
            // transmitter would stay disabled and its receiver would run only
            // because the last session happened to leave it in `rx` — a radio
            // that appears broken with nothing on screen to explain it, and
            // no way out but power-cycling the Pluto. So the *specific*
            // configuration this writes is recognised and undone, and a board
            // in TDD for somebody else's reasons is still left alone.
            match self.ensm_mode(conn) {
                Ok(mode) if !mode.to_ascii_lowercase().contains("fdd") => {
                    let mut slaved = Vec::new();
                    for attr in PTT_SLAVE_ATTRS {
                        if self.has_debug_attr(attr)
                            && conn
                                .read_debug_attr(&self.phy_id, attr)
                                .is_ok_and(|v| v.trim() == "1")
                        {
                            slaved.push(attr);
                        }
                    }
                    if slaved.is_empty() {
                        tracing::info!(
                            "PlutoSDR: this board's enable state machine is in {mode} and \
                             no GPO pin is slaved to it — somebody put it in TDD for their \
                             own reasons, so it is left that way"
                        );
                        return Ok(out);
                    }
                    let msg = format!(
                        "PlutoSDR: this board is in {mode} with {} slaved to the radio — \
                         the transmit-receive switching from an earlier session. Putting \
                         it back into FDD and un-slaving the pins, because with the PTT \
                         pins off nothing here drives its state machine. Choose TDD in \
                         the PlutoSDR settings if you meant to keep it",
                        slaved.join(", ")
                    );
                    tracing::warn!("{msg}");
                    out.warnings.push(msg);
                }
                // In FDD, or a firmware that does not publish the mode: either
                // way there is nothing to undo and nothing to say.
                _ => return Ok(out),
            }
        }
        for attr in [FDD_ENABLE, INITIALIZE] {
            if !self.has_debug_attr(attr) {
                let msg = format!(
                    "PlutoSDR: this firmware publishes no {attr}, so the transmit-receive \
                     switching cannot be set up — the GPO pins are left alone. It is \
                     the AD9361 driver's own device-tree property, so a board without \
                     it is running something other than a stock Pluto image"
                );
                tracing::warn!("{msg}");
                out.warnings.push(msg);
                return Ok(out);
            }
        }

        // The duplex first, then the pins, then the commit — the order the
        // AD9361 driver's own note gives, and the only one where `initialize`
        // sees all of it.
        let fdd_enable = if want_tdd { "0" } else { "1" };
        if !self.write_setup_attr(conn, FDD_ENABLE, fdd_enable, &mut out.warnings)? {
            return Ok(out);
        }
        for pin in 0..GPO_PINS {
            // Every pin is written, not just the chosen pair: a pair selected
            // in an earlier session is still slaved, and leaving it that way
            // would key two amplifiers.
            for (attr, want) in [
                (format!("adi,gpo{pin}-slave-rx-enable"), pins.is_some_and(|(rx, _)| rx == pin)),
                (format!("adi,gpo{pin}-slave-tx-enable"), pins.is_some_and(|(_, tx)| tx == pin)),
            ] {
                if !self.has_debug_attr(&attr) {
                    if want {
                        let msg = format!(
                            "PlutoSDR: this firmware publishes no {attr}, so GPO{pin} cannot \
                             be slaved to the radio — pick the other pair, or a \
                             firmware that offers it"
                        );
                        tracing::warn!("{msg}");
                        out.warnings.push(msg);
                    }
                    continue;
                }
                let value = if want { "1" } else { "0" };
                self.write_setup_attr(conn, &attr, value, &mut out.warnings)?;
            }
        }
        if pins == Some((0, 1)) {
            // Analog Devices' own note puts an external LNA on these two, so a
            // board wired that way has them claimed. PTT wins here because the
            // operator just asked for it; GPO2/GPO3 is the pair that leaves an
            // eLNA alone.
            for attr in ELNA_GPO {
                if self.has_debug_attr(attr) {
                    tracing::info!(
                        "PlutoSDR: clearing {attr} — GPO0/GPO1 are the PTT pair now, and a \
                         pin cannot drive an external LNA and a T/R switch at once"
                    );
                    self.write_setup_attr(conn, attr, "0", &mut out.warnings)?;
                }
            }
        }
        if !self.write_setup_attr(conn, INITIALIZE, "1", &mut out.warnings)? {
            return Ok(out);
        }
        // Read the duplex back rather than trusting the write. An accepted
        // write only means the driver stored the property; whether the part
        // came up in the duplex asked for is the question, and getting it
        // wrong the optimistic way is the worst of the outcomes — a radio
        // driving `ensm_mode` on a board still in FDD, where receive would
        // stop at the first key-up and never come back.
        match conn.read_debug_attr(&self.phy_id, FDD_ENABLE) {
            Ok(v) if v.trim() == fdd_enable => out.tdd = want_tdd,
            Ok(v) => {
                let msg = format!(
                    "PlutoSDR: the radio took {FDD_ENABLE} = {fdd_enable} and then read it \
                     back as {v:?} — so it is in the other duplex, the GPO pins will not \
                     follow the radio, and its state machine is being left alone"
                );
                tracing::warn!("{msg}");
                out.warnings.push(msg);
                return Ok(out);
            }
            // A firmware that will not read back what it just accepted is odd
            // but not a reason to refuse the setting the operator asked for.
            Err(e) => {
                tracing::debug!("PlutoSDR: could not read {FDD_ENABLE} back: {e}");
                out.tdd = want_tdd;
            }
        }
        if !want_tdd {
            // Restoring: `initialize` leaves the state machine wherever the
            // driver's setup puts it, and this backend does not drive it in
            // FDD — so the one write that says "and receive, please" is worth
            // making rather than assuming. Best-effort: a board that refuses
            // it will say so in the full-duplex check on the way past.
            if let Err(e) = self.set_ensm_mode(conn, "fdd") {
                tracing::debug!("PlutoSDR: could not put the state machine back to fdd: {e}");
            }
            return Ok(out);
        }
        match pins {
            Some((rx, tx)) => tracing::info!(
                "PlutoSDR: TDD, with GPO{rx} high on receive and GPO{tx} high on \
                 transmit — those pins are about 1.3 V at a few milliamps, enough \
                 for a transistor or an opto-isolator and not for a relay coil"
            ),
            None => tracing::info!("PlutoSDR: TDD, with no GPO pin slaved to the radio"),
        }
        Ok(out)
    }

    /// Write one setup attribute, answering whether it landed. A device that
    /// refuses the value becomes a warning for the operator; a socket that has
    /// gone is still an error, because nothing after it would work either.
    fn write_setup_attr(
        &self,
        conn: &mut Connection,
        attr: &str,
        value: &str,
        warnings: &mut Vec<String>,
    ) -> Result<bool> {
        match conn.write_debug_attr(&self.phy_id, attr, value) {
            Ok(()) => Ok(true),
            Err(e @ (Error::Remote { .. } | Error::Unsupported(_))) => {
                let msg = format!("PlutoSDR: the radio refused {attr} = {value} ({e})");
                tracing::warn!("{msg}");
                warnings.push(msg);
                Ok(false)
            }
            Err(e) => Err(e),
        }
    }

    pub fn tx_port(&self, conn: &mut Connection) -> Result<String> {
        conn.read_attr(&self.phy_id, Some(TX_CHAN), "rf_port_select")
    }

    /// Silence the transmit DDS.
    ///
    /// The transmit path can be fed either by a DMA buffer or by four on-chip
    /// tone generators, and the tone generators win by default. Skip this and
    /// the Pluto transmits a carrier pair instead of the modulated signal —
    /// on frequency, at full power, and completely wrong.
    pub fn silence_dds(&self, conn: &mut Connection) -> Result<()> {
        for chan in &self.dds_channels {
            conn.write_attr(&self.tx_buffer_id, Some((true, chan)), "raw", "0")?;
        }
        if self.dds_channels.is_empty() {
            tracing::warn!(
                "PlutoSDR: {TX_BUFFER} published no DDS tone channels to silence — if \
                 transmit puts out a steady carrier instead of the modulation, this is why"
            );
        }
        Ok(())
    }
}

/// The scan elements a buffer may be opened with: up to `max_pairs` I/Q pairs
/// at contiguous scan indices from 0.
///
/// A stock Pluto's buffers carry exactly one pair. A 2R2T firmware — a
/// Pluto+, or any AD936x build with both chains compiled in — publishes four
/// elements, two chains' worth of I and Q; the receive side keeps both pairs
/// (each can serve a radio of its own) while transmit keeps only the first.
/// `OPEN`'s channel mask enables the first `pairs × 2` of what is returned
/// here, so every kept element must sit at the scan index its position says,
/// or the mask would enable something other than what is decoded. The second
/// value is a sentence for the session trace when elements beyond `max_pairs`
/// were left aside.
///
/// What cannot be accepted is a device with fewer than two elements — that
/// cannot carry complex baseband at all.
fn iq_scan(
    name: &str,
    dev: &Device,
    max_pairs: usize,
) -> Result<(Vec<ScanFormat>, Option<String>)> {
    let scans = dev.scan_channels();
    if scans.len() < 2 {
        return Err(Error::Unsupported(format!(
            "{name} has {} scan element(s), not the I and Q an I/Q stream needs",
            scans.len()
        )));
    }
    // Whole pairs only: a buffer with three elements keeps one pair.
    let keep = (scans.len() / 2 * 2).min(max_pairs * 2);
    for (pos, c) in scans[..keep].iter().enumerate() {
        let idx = c.scan.expect("scan_channels keeps only scan elements").0;
        if idx != pos as u32 {
            return Err(Error::Unsupported(format!(
                "{name}'s scan element {} sits at index {idx}, not {pos} — the channel \
                 mask this driver opens buffers with would not select it",
                c.id
            )));
        }
    }
    let note = (scans.len() > keep).then(|| {
        let kept: Vec<&str> = scans[..keep].iter().map(|c| c.id.as_str()).collect();
        let aside: Vec<&str> = scans[keep..].iter().map(|c| c.id.as_str()).collect();
        format!(
            "{name} has {} scan elements — a dual-channel (2R2T) firmware such as a \
             Pluto+. Keeping {} for this side and leaving {} aside",
            scans.len(),
            kept.join(", "),
            aside.join(", ")
        )
    });
    Ok((scans[..keep].iter().map(|c| c.scan.expect("checked above").1).collect(), note))
}

// ---- attribute helpers --------------------------------------------------

fn read_f64(conn: &mut Connection, dev: &str, chan: (bool, &str), attr: &str) -> Result<f64> {
    let text = conn.read_attr(dev, Some(chan), attr)?;
    // Values arrive as bare numbers, sometimes with a unit ("71.000000 dB").
    let head = text.split_whitespace().next().unwrap_or("");
    head.parse()
        .map_err(|_| Error::Msg(format!("{dev} {} {attr} answered {text:?}, not a number", chan.1)))
}

/// Read an IIO `[min step max]` range, or `None` when the device does not
/// publish that attribute at all.
fn read_range(
    conn: &mut Connection,
    dev_id: &str,
    dev: &crate::context::Device,
    chan: (bool, &str),
    attr: &str,
) -> Result<Option<(f64, f64, f64)>> {
    if !dev.channel(chan.1, chan.0).is_some_and(|c| c.has_attr(attr)) {
        return Ok(None);
    }
    let text = conn.read_attr(dev_id, Some(chan), attr)?;
    Ok(parse_range(&text))
}

/// A space-separated list attribute (`gain_control_mode_available`,
/// `rf_port_select_available`), or empty when absent.
fn read_list(
    conn: &mut Connection,
    dev_id: &str,
    dev: &crate::context::Device,
    chan: (bool, &str),
    attr: &str,
) -> Result<Vec<String>> {
    if !dev.channel(chan.1, chan.0).is_some_and(|c| c.has_attr(attr)) {
        return Ok(Vec::new());
    }
    let text = conn.read_attr(dev_id, Some(chan), attr)?;
    Ok(text.split_whitespace().map(str::to_string).collect())
}

/// Parse IIO's `[min step max]` range spelling into `(min, step, max)`.
///
/// The brackets are part of the value, and the fields are in that order — not
/// min/max/step, which is the natural reading and the wrong one.
fn parse_range(text: &str) -> Option<(f64, f64, f64)> {
    let inner = text.trim().trim_start_matches('[').trim_end_matches(']');
    let mut it = inner.split_whitespace();
    let min: f64 = it.next()?.parse().ok()?;
    let step: f64 = it.next()?.parse().ok()?;
    let max: f64 = it.next()?.parse().ok()?;
    if !min.is_finite() || !step.is_finite() || !max.is_finite() || max < min {
        return None;
    }
    Some((min, step, max))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::Channel;

    /// A capture/DMA buffer device whose scan elements sit at `indices`.
    fn buffer(indices: &[u32]) -> Device {
        Device {
            id: "iio:device4".into(),
            name: "cf-ad9361-lpc".into(),
            attrs: Vec::new(),
            debug_attrs: Vec::new(),
            channels: indices
                .iter()
                .map(|&i| Channel {
                    id: format!("voltage{i}"),
                    name: None,
                    output: false,
                    attrs: Vec::new(),
                    scan: Some((i, ScanFormat::parse("le:s12/16>>0").expect("format"))),
                })
                .collect(),
        }
    }

    #[test]
    fn a_stock_pluto_buffer_is_its_own_iq_pair() {
        let (scan, note) = iq_scan("cf-ad9361-lpc", &buffer(&[0, 1]), 2).expect("pair");
        assert_eq!(scan.len(), 2);
        assert!(note.is_none(), "a stock buffer needs no explaining: {note:?}");
    }

    /// A Pluto+ runs 2R2T firmware: four scan elements, two chains' worth of
    /// I/Q. The receive side keeps both pairs — each can serve a radio of its
    /// own — so nothing is left aside and nothing needs explaining.
    #[test]
    fn a_2r2t_receive_buffer_keeps_both_pairs() {
        let (scan, note) = iq_scan("cf-ad9361-lpc", &buffer(&[0, 1, 2, 3]), 2).expect("pairs");
        assert_eq!(scan.len(), 4);
        assert!(note.is_none(), "both pairs kept, nothing to note: {note:?}");
    }

    /// The transmit side stays on the first chain, so a 2R2T transmit buffer
    /// keeps one pair and says what it left aside.
    #[test]
    fn a_2r2t_transmit_buffer_keeps_the_first_pair_and_notes_the_rest() {
        let (scan, note) =
            iq_scan("cf-ad9361-dds-core-lpc", &buffer(&[0, 1, 2, 3]), 1).expect("pair");
        assert_eq!(scan.len(), 2);
        let note = note.expect("a pair left aside is worth a line in the report");
        assert!(note.contains("4 scan elements"), "{note}");
        assert!(note.contains("voltage0, voltage1"), "{note}");
        assert!(note.contains("voltage2, voltage3"), "{note}");
    }

    #[test]
    fn too_few_scan_elements_are_refused() {
        let err = iq_scan("cf-ad9361-lpc", &buffer(&[0]), 2).expect_err("one element");
        assert!(err.to_string().contains("1 scan element"), "{err}");
    }

    /// The mask `open_buffer` sends is literally the low bits; a device whose
    /// pairs sit elsewhere would stream elements we would mis-decode.
    #[test]
    fn a_pair_off_indices_zero_and_one_is_refused() {
        let err = iq_scan("cf-ad9361-lpc", &buffer(&[2, 3]), 2).expect_err("wrong indices");
        assert!(err.to_string().contains("index 2"), "{err}");
    }

    #[test]
    fn ranges_are_min_step_max_in_that_order() {
        // An AD9364-unlocked Pluto's tuning range.
        assert_eq!(parse_range("[70000000 1 6000000000]"), Some((70e6, 1.0, 6e9)));
        // Transmit attenuation, as gain.
        let (min, step, max) = parse_range("[-89.750000 0.250000 0.000000]").expect("tx range");
        assert_eq!((min, step, max), (-89.75, 0.25, 0.0));
        // Whitespace and missing brackets are both survivable.
        assert_eq!(parse_range(" 0 1 71 "), Some((0.0, 1.0, 71.0)));
        assert_eq!(parse_range("[0 1 71]"), Some((0.0, 1.0, 71.0)));
        // Anything else is "the device did not say", not a panic.
        assert_eq!(parse_range(""), None);
        assert_eq!(parse_range("[0 1]"), None);
        assert_eq!(parse_range("slow_attack fast_attack"), None);
        // A backwards range is a firmware bug, not a limit to obey.
        assert_eq!(parse_range("[100 1 10]"), None);
    }

    #[test]
    fn the_fallback_follows_the_part_the_model_string_names() {
        assert_eq!(PlutoLimits::fallback(false).rx_lo_hz, AD9363_LO_HZ);
        assert_eq!(PlutoLimits::fallback(true).rx_lo_hz, AD9364_LO_HZ);
    }

    /// A guessed limit has to announce itself: the operator otherwise reads
    /// "325–3800 MHz" as something the radio said about itself.
    #[test]
    fn assumed_limits_produce_a_notice_and_known_ones_do_not() {
        let mut l = PlutoLimits::fallback(false);
        assert!(l.assumption_notice().is_none());
        l.assumed = vec!["a tuning range".into()];
        let notice = l.assumption_notice().expect("notice");
        assert!(notice.contains("a tuning range"), "{notice}");
        assert!(notice.contains("assuming"), "{notice}");
    }
}
