//! The ELAD wire protocol, with no `nusb` in it.
//!
//! Everything here is arithmetic and constants, which is what makes the fiddly
//! half of this driver testable with nothing plugged in — and it is the half
//! that most needs testing, because none of it has been checked against
//! hardware. See the crate header for provenance.

use std::time::Duration;

/// ELAD's USB vendor id. Every device in the family answers to it.
pub const VID: u16 = 0x1721;

/// The one USB interface, and the one bulk endpoint the samples arrive on.
pub const INTERFACE: u8 = 0;
pub const BULK_EP: u8 = 0x86;
pub const CONFIGURATION: u8 = 1;

/// How long a control transfer may take before it is a failure. Generous: the
/// DUO's CAT tunnel is a request into a microcontroller that is also driving a
/// front panel.
pub const CTRL_TIMEOUT: Duration = Duration::from_millis(1000);

/// Bytes per bulk transfer.
///
/// `512 × 24`, which is what `gr-elad` uses and is a whole number of USB 2.0
/// high-speed packets. At the default 192 kHz that is 8 ms of samples; at
/// 6144 kHz, 500 µs.
pub const TRANSFER_BYTES: usize = 512 * 24;

/// Which ELAD this is. The USB product id is the only thing that says so, and
/// it says so before the device has been opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Model {
    /// FDM-DUO / FDM-DUOr transceiver.
    Duo,
    /// FDM-S2 sampler.
    S2,
    /// FDM-S1 sampler.
    S1,
}

impl Model {
    pub fn from_pid(pid: u16) -> Option<Model> {
        match pid {
            0x061a => Some(Model::Duo),
            0x061c => Some(Model::S2),
            0x0610 => Some(Model::S1),
            _ => None,
        }
    }

    pub fn pid(self) -> u16 {
        match self {
            Model::Duo => 0x061a,
            Model::S2 => 0x061c,
            Model::S1 => 0x0610,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Model::Duo => "ELAD FDM-DUO",
            Model::S2 => "ELAD FDM-S2",
            Model::S1 => "ELAD FDM-S1",
        }
    }

    /// The ADC clock the DDC's tuning word is computed against, in Hz.
    ///
    /// The S1 samples at half the rate of the other two, which is also why its
    /// coverage stops at 30 MHz rather than 54.
    pub fn clock_hz(self) -> f64 {
        match self {
            Model::Duo | Model::S2 => 122_880_000.0,
            Model::S1 => 61_440_000.0,
        }
    }

    /// Whether this model can transmit at all. Only the transceiver — and only
    /// through its CAT link and its sound card, never through this USB
    /// interface, which is receive-only on every model.
    pub fn transmits(self) -> bool {
        self == Model::Duo
    }

    /// Whether the host has to load this model's FPGA image before it can
    /// stream.
    ///
    /// The samplers are two halves that come up very differently: the Cypress
    /// bridge's firmware is in an EEPROM, so an untouched device already
    /// enumerates, reports its serial and acknowledges the FIFO start — while
    /// the FPGA behind it is SRAM-configured and empty, so there is no
    /// down-converter in there to start and the bulk endpoint never produces a
    /// byte. The FDM-DUO is a radio with its own controller and boots its own.
    /// See [`crate::fpga`], and issue #178, which is what an unloaded sampler
    /// looks like from the operator's chair.
    ///
    /// Confirmed on the FDM-S2, which is the model ELAD's Linux loader and
    /// every third-party recipe for it name. The FDM-S1 is included by family —
    /// same architecture, same bridge, same open sequence — and if it turns out
    /// not to need one, the cost is a loader run that does nothing.
    pub fn needs_fpga_load(self) -> bool {
        matches!(self, Model::S1 | Model::S2)
    }

    /// What this model can hear, in Hz.
    ///
    /// The published coverage rather than the Nyquist limit: an S2 will
    /// undersample well above 54 MHz and ELAD documents a VHF window for it,
    /// but nothing here can check which of the pre-selection filters is in
    /// circuit at a given frequency, so the range offered is the one the
    /// specification guarantees.
    pub fn rx_range_hz(self) -> (f64, f64) {
        match self {
            Model::Duo | Model::S2 => (10_000.0, 54_000_000.0),
            Model::S1 => (10_000.0, 30_000_000.0),
        }
    }
}

/// The vendor requests, by `bRequest`.
///
/// Naming is ELAD's in spirit only — the codes come from `gr-elad`, which uses
/// them without naming them. What each one does is inferred from how it is
/// used, and is recorded here so a capture can be read against it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Request {
    /// `0xA2` — read the configuration EEPROM. `wValue` is the address,
    /// `wIndex` is always [`EEPROM_INDEX`].
    Eeprom = 0xA2,
    /// `0xE1` — the FDM-DUO's gateway to its FPGA and its microcontroller. The
    /// *real* command is the high byte of `wIndex`; see [`DuoSub`].
    DuoGateway = 0xE1,
    /// `0xE9` — start the sample FIFO on an S1/S2. Answers with its own code.
    SamplerStart = 0xE9,
    /// `0xF2` — write the DDC tuning word on an S1/S2.
    SamplerTune = 0xF2,
    /// `0xF7` — the S1/S2 front end. `wIndex` 2 is the filter, 3 the
    /// attenuator. Answers with its own code.
    SamplerFrontEnd = 0xF7,
    /// `0xFF` — the firmware's own version, two bytes.
    Version = 0xFF,
}

impl Request {
    pub fn code(self) -> u8 {
        self as u8
    }
}

/// The FDM-DUO's sub-command, carried in the **high byte** of `wIndex` behind
/// [`Request::DuoGateway`].
///
/// This is the part of the protocol most worth stating plainly, because it is
/// the part a reader of the original C will miss: `0xE1` on its own means
/// nothing, and every DUO operation is `0xE1` with one of these shifted eight
/// bits left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DuoSub {
    /// `0xE8` — initialise the FIFO, putting the Cypress bridge's EP6 into
    /// slave FIFO mode.
    FifoInit = 0xE8,
    /// `0xE9` — the FIFO run/stop switch: `wValue` 0 stops it, 1 starts it. A
    /// start answers with the byte `0xE9`.
    FifoRun = 0xE9,
    /// `0xF1` — write a 16-byte ASCII CAT command into the radio's own command
    /// buffer, the same commands the CAT serial port takes.
    CatWrite = 0xF1,
    /// `0xF0` — read the radio's answer to the last CAT query: eighteen bytes,
    /// `F0`, then the answer in ASCII up to its `;`. Read once the status's
    /// [`STATUS_CAT_REPLY`] bit is set.
    CatRead = 0xF0,
    /// `0xF2` — write the DDC tuning word. The low sixteen bits go in `wValue`
    /// and the next eight in the *low* byte of `wIndex`, beside this code.
    Tune = 0xF2,
    /// `0xF5` — read back what the radio is tuned to: eleven bytes carrying
    /// the receive frequency and the DDC word the radio put in its own FPGA.
    /// See [`decode_tune_read`].
    TuneRead = 0xF5,
    /// `0xFC` — read three status bytes. Bit 2 of the third says the CAT
    /// command buffer is still busy with the last command.
    Status = 0xFC,
    /// `0xF6`, `0xF9` — read one and two bytes before a remote image is
    /// loaded. They answer `F6` and `F9 00`; what they report is not known,
    /// they are sent because SDR Console sends them. See [`crate::remote`].
    PreLoadA = 0xF6,
    PreLoadB = 0xF9,
    /// `0xE3` — begin loading a remote FPGA image (`wValue` 1). Answers `E3`.
    LoadBegin = 0xE3,
    /// `0xE4` — one piece of the image, `wValue` its length.
    LoadData = 0xE4,
    /// `0xE5` — the load is finished. Answers `E5 10`.
    LoadEnd = 0xE5,
    /// `0xFD` — read three bytes after a load; answers `FD 32 31`, which reads
    /// as "21" and may be the image's version.
    PostLoad = 0xFD,
}

impl DuoSub {
    /// This sub-command as a `wIndex` with no low byte of its own.
    pub fn index(self) -> u16 {
        (self as u16) << 8
    }
}

/// Bit 2 of the third status byte: the CAT buffer has not finished the previous
/// command. Writing another one before this clears loses it.
pub const STATUS_CAT_BUSY: u8 = 0x04;

/// Bit 3 of the third status byte: an answer to a CAT query is waiting for
/// [`DuoSub::CatRead`]. SDR Console waits for it (`FC 25 09`) before reading.
pub const STATUS_CAT_REPLY: u8 = 0x08;

/// Bytes in a [`DuoSub::CatRead`] reply.
pub const CAT_REPLY_LEN: u16 = 18;

/// The ASCII answer in a [`DuoSub::CatRead`] reply, up to and including its
/// `;`. Empty when the reply is not one.
pub fn decode_cat_reply(reply: &[u8]) -> String {
    if reply.first() != Some(&(DuoSub::CatRead as u8)) {
        return String::new();
    }
    let body = &reply[1..];
    let end = body.iter().position(|&b| b == b';').map_or(0, |i| i + 1);
    String::from_utf8_lossy(&body[..end]).into_owned()
}

/// Whether a `GI` answer says the radio is transmitting (P4: 0 receive, 1
/// transmit, 2 tune; CAT manual v2.13 p50). `None` for anything that is not a
/// `GI` answer.
pub fn gi_transmitting(answer: &str) -> Option<bool> {
    let b = answer.as_bytes();
    if !answer.starts_with("GI") || b.len() < 8 || !answer.ends_with(';') {
        return None;
    }
    match b[7] {
        b'0' => Some(false),
        b'1' | b'2' => Some(true),
        _ => None,
    }
}

/// Longest a CAT command written through the USB gateway may be. Fixed-length:
/// the request always carries sixteen bytes whatever the command is.
pub const CAT_FRAME_LEN: usize = 16;

/// Bytes in a [`DuoSub::TuneRead`] reply.
pub const TUNE_READ_LEN: u16 = 11;

/// What an FDM-DUO says it is tuned to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DuoTuning {
    /// The receive frequency, in Hz.
    pub hz: f64,
    /// The DDC tuning word the radio has in its own FPGA — the same number
    /// [`tuning_word`] computes for [`Self::hz`] against the calibrated clock.
    pub word: u32,
    /// The last byte of the reply, kept raw. On an FDM-DUO with hardware 2.9
    /// and firmware 4.9 it reads `0x10` at rest and `0x11` while the dial is
    /// turning, but two values seen on one radio are not a specification, so
    /// nothing here acts on it.
    pub status: u8,
}

/// Decode a [`DuoSub::TuneRead`] reply, or `None` if it is not one.
///
/// The frame, from an FDM-DUO sitting on 940 kHz:
///
/// ```text
/// F5  00 0E 57 E0  01 F5 58 21  00 10
/// ^   ^            ^            ^
/// |   |            |            status
/// |   |            the DDC tuning word, big-endian
/// |   the frequency in Hz, big-endian: 940000
/// the sub-command, echoed
/// ```
///
/// Both halves are worth having. The frequency is what the operator's knob
/// moves and the only way to follow it without the CAT serial port; the word is
/// the radio's own arithmetic, which on the bench matched [`tuning_word`] to
/// the bit at 940 kHz and at 991.3 kHz — but only with the EEPROM clock
/// correction applied, which is what makes it a check on that too.
pub fn decode_tune_read(reply: &[u8]) -> Option<DuoTuning> {
    if reply.len() < TUNE_READ_LEN as usize || reply[0] != DuoSub::TuneRead as u8 {
        return None;
    }
    let be32 = |b: &[u8]| u32::from_be_bytes([b[0], b[1], b[2], b[3]]);
    Some(DuoTuning {
        hz: f64::from(be32(&reply[1..5])),
        word: be32(&reply[5..9]),
        status: reply[10],
    })
}

/// `wIndex` for every EEPROM read. Not a meaningful index — it is the constant
/// the device wants beside the address.
pub const EEPROM_INDEX: u16 = 0x0151;

/// EEPROM addresses, as `(address, length)`.
pub mod eeprom {
    /// The device's serial number, as ASCII.
    pub const SERIAL: (u16, u16) = (0x4000, 32);
    /// Hardware version, as `(major, minor)`.
    pub const HW_VERSION: (u16, u16) = (0x404C, 2);
    /// A signed correction in Hz added to the nominal ADC clock. Small — parts
    /// per million of 122.88 MHz — but it is what makes the tuning word land on
    /// the frequency asked for rather than a few tens of hertz away.
    pub const RATE_CORRECTION: (u16, u16) = (0x4024, 4);
    /// Per-unit gain offsets in dB, as little-endian `f32`: the receiver's
    /// overall calibration, and the corrections for having the low-pass filter
    /// and the attenuator in circuit.
    pub const GLOBAL_OFFSET_DB: (u16, u16) = (0x4028, 4);
    pub const LP_OFFSET_DB: (u16, u16) = (0x402C, 4);
    pub const ATT_OFFSET_DB: (u16, u16) = (0x4030, 4);
}

/// The per-unit calibration read out of the EEPROM at open.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Calibration {
    /// Hz added to the nominal ADC clock.
    pub rate_correction_hz: i32,
    pub global_db: f32,
    pub lp_db: f32,
    pub att_db: f32,
}

impl Calibration {
    /// The scale from a full-scale wire sample to 1.0, for this device in this
    /// state.
    ///
    /// Three things go into it, all in dB and all added before a single
    /// exponentiation: the per-unit EEPROM offsets, a per-rate term that
    /// compensates the DDC's own decimation gain ([`rate_rescale_db`]), and —
    /// on the FDM-DUO only — a fixed 21.4 dB. That last number is ELAD's, with
    /// no derivation published anywhere; it is carried because leaving it out
    /// puts the DUO's spectrum twenty-one decibels below the samplers' for the
    /// same signal.
    pub fn scale(&self, model: Model, rate_hz: u32, lp: bool, att: bool) -> f32 {
        let duo_db = if model == Model::Duo { 21.4 } else { 0.0 };
        let db = duo_db
            + rate_rescale_db(rate_hz)
            + self.global_db
            + if lp { self.lp_db } else { 0.0 }
            + if att { self.att_db } else { 0.0 };
        10f32.powf(db / 20.0)
    }
}

/// The per-rate gain term, in dB.
///
/// The DDC's decimation chain has a different processing gain at each rate, and
/// these are the corrections that put them all on one scale. Straight from
/// `gr-elad`; there is no formula behind them that ELAD publishes, and the jump
/// from +5.4 to −0.7 at the top is where the samples also change width.
pub fn rate_rescale_db(rate_hz: u32) -> f32 {
    match rate_hz {
        384_000 | 768_000 => 6.0,
        1_536_000 | 3_072_000 => 5.4,
        6_144_000 => -0.7,
        // 192 kHz, and any rate this driver does not know, sit at unity.
        _ => 0.0,
    }
}

/// How wide one component of a sample is on the wire, in bytes, at `rate_hz`.
///
/// Not a scale factor but a decode difference: below 6144 kHz the DDC delivers
/// 32-bit words and at 6144 kHz it delivers 16-bit ones. Reading one as the
/// other does not produce a quiet signal, it produces noise — which is the one
/// mercy in a setting that cannot be verified by asking the device.
pub fn component_bytes(rate_hz: u32) -> usize {
    if rate_hz >= 6_144_000 { 2 } else { 4 }
}

/// Bytes on the wire for one complex sample.
pub fn sample_bytes(rate_hz: u32) -> usize {
    component_bytes(rate_hz) * 2
}

/// The DDC tuning word for `hz` against a clock of `clock_hz`.
///
/// The NCO is a 32-bit phase accumulator, so the word is the fraction of the
/// clock the wanted frequency sits at, scaled by 2³². The modulo is what lets a
/// sampler tune above its own clock — an alias, deliberately, which is how the
/// S2 reaches VHF.
pub fn tuning_word(hz: f64, clock_hz: f64) -> u32 {
    if clock_hz <= 0.0 {
        return 0;
    }
    let hz = hz.max(0.0);
    let frac = hz - (hz / clock_hz).floor() * clock_hz;
    let word = (4_294_967_296.0 * frac) / clock_hz;
    // Wraps rather than saturates: a fraction of exactly 1.0 is phase zero.
    (word as u64 & 0xFFFF_FFFF) as u32
}

/// The `(wValue, wIndex, data)` of a tune request for `model`.
///
/// The two layouts differ in one byte and only in one byte, which is exactly
/// the sort of difference that is invisible in a diff and fatal on the wire: on
/// a sampler the top sixteen bits of the word go in `wIndex`, while on the DUO
/// the high byte of `wIndex` is the [`DuoSub::Tune`] selector and only eight
/// bits of the word fit beside it. The remaining byte travels in the payload on
/// both.
///
/// `remote` is a DUO running a host-loaded image ([`crate::remote`]), where the
/// payload's second byte is `0x02` rather than `0x00` — as SDR Console sends
/// it. What the byte means is not known.
pub fn tune_request(model: Model, word: u32, remote: bool) -> (u16, u16, [u8; 2]) {
    let value = (word & 0xFFFF) as u16;
    let mode = if remote && model == Model::Duo { 0x02 } else { 0x00 };
    let data = [((word >> 24) & 0xFF) as u8, mode];
    let index = match model {
        Model::Duo => DuoSub::Tune.index() | ((word >> 16) & 0xFF) as u16,
        Model::S1 | Model::S2 => ((word >> 16) & 0xFFFF) as u16,
    };
    (value, index, data)
}

/// `wIndex` selectors for [`Request::SamplerFrontEnd`].
pub const FRONT_END_FILTER: u16 = 2;
pub const FRONT_END_ATTENUATOR: u16 = 3;

/// The FDM-S2's pre-selection filter code for a dial at `hz`, with the filters
/// in or out and the attenuator on or off.
///
/// The S2's filter bank is switched by *frequency band* rather than by a
/// bypass flag, so which code is right depends on where the receiver is
/// pointed. Bypass is a code of its own, and the attenuator rides in bit 3 of
/// the same byte.
pub fn s2_front_end_code(hz: f64, filters: bool, att: bool) -> u16 {
    let mut code: u16 = if !filters {
        0x01
    } else if hz < 61_440_000.0 {
        0x03
    } else if hz < 122_880_000.0 {
        0x02
    } else {
        0x04
    };
    if att {
        code |= 0x08;
    }
    code
}

/// Whether a serial the operator pinned matches the one a device reports.
///
/// A suffix match, like the HackRF's: operators copy serials out of other
/// programs, which print them padded, truncated or upper-cased in at least
/// three different ways.
pub fn serial_matches(want: &str, have: Option<&str>) -> bool {
    let want = want.trim();
    if want.is_empty() {
        return true;
    }
    match have {
        Some(h) => {
            let (h, w) = (h.trim().to_ascii_lowercase(), want.to_ascii_lowercase());
            h == w || h.ends_with(&w) || w.ends_with(&h)
        }
        None => false,
    }
}

/// The printable ASCII prefix of an EEPROM string field.
pub fn eeprom_string(bytes: &[u8]) -> Option<String> {
    let s: String = bytes
        .iter()
        .take_while(|&&b| b != 0)
        .filter(|&&b| b.is_ascii_graphic() || b == b' ')
        .map(|&b| b as char)
        .collect();
    let s = s.trim().to_string();
    (!s.is_empty()).then_some(s)
}

/// A little-endian `f32` from an EEPROM read, or `None` when the field is not
/// four bytes or holds something that is not a number.
///
/// The NaN check is not defensive padding: these fields are per-unit
/// calibration written at the factory, and an unprogrammed EEPROM reads as
/// `0xFF` bytes — which is a NaN, and which would propagate through
/// [`Calibration::scale`] and silence the receiver.
pub fn eeprom_f32(bytes: &[u8]) -> Option<f32> {
    let arr: [u8; 4] = bytes.get(..4)?.try_into().ok()?;
    let v = f32::from_le_bytes(arr);
    v.is_finite().then_some(v)
}

/// A little-endian `i32` from an EEPROM read.
pub fn eeprom_i32(bytes: &[u8]) -> Option<i32> {
    let arr: [u8; 4] = bytes.get(..4)?.try_into().ok()?;
    Some(i32::from_le_bytes(arr))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_product_ids_name_the_three_models() {
        assert_eq!(Model::from_pid(0x061a), Some(Model::Duo));
        assert_eq!(Model::from_pid(0x061c), Some(Model::S2));
        assert_eq!(Model::from_pid(0x0610), Some(Model::S1));
        assert_eq!(Model::from_pid(0x0611), None);
        for m in [Model::Duo, Model::S2, Model::S1] {
            assert_eq!(Model::from_pid(m.pid()), Some(m));
        }
        // Only the transceiver transmits, and the S1 hears half as far up as
        // the other two because it samples at half the clock.
        assert!(Model::Duo.transmits());
        assert!(!Model::S2.transmits());
        assert_eq!(Model::S1.clock_hz(), Model::Duo.clock_hz() / 2.0);
        assert_eq!(Model::S1.rx_range_hz().1, 30_000_000.0);
    }

    /// Frames read off an FDM-DUO (hardware 2.9, firmware 4.9) at two dial
    /// settings, which is where every byte of the layout comes from.
    #[test]
    fn a_tune_read_gives_back_the_frequency_the_radio_is_on() {
        let at_940 = [0xF5, 0x00, 0x0E, 0x57, 0xE0, 0x01, 0xF5, 0x58, 0x21, 0x00, 0x10];
        let t = decode_tune_read(&at_940).expect("a well-formed reply");
        assert_eq!(t.hz, 940_000.0);
        assert_eq!(t.word, 0x01F5_5821);

        let at_991_3 = [0xF5, 0x00, 0x0F, 0x20, 0x44, 0x02, 0x10, 0xB4, 0x71, 0x00, 0x11];
        let t = decode_tune_read(&at_991_3).expect("a well-formed reply");
        assert_eq!(t.hz, 991_300.0);
        assert_eq!(t.word, 0x0210_B471);
    }

    /// The radio's own word and ours are the same arithmetic — against the
    /// *calibrated* clock. Nominal misses by several hundred counts, which is
    /// how this doubles as a check that the EEPROM correction is applied.
    #[test]
    fn the_radios_own_tuning_word_matches_the_one_this_crate_computes() {
        let clock = 122_880_000.0 - 2678.0; // this unit's EEPROM correction
        assert_eq!(tuning_word(940_000.0, clock), 0x01F5_5821);
        assert_eq!(tuning_word(991_300.0, clock), 0x0210_B471);
        assert_ne!(tuning_word(940_000.0, 122_880_000.0), 0x01F5_5821);
    }

    #[test]
    fn a_reply_that_is_not_a_tune_read_is_refused() {
        assert_eq!(decode_tune_read(&[]), None);
        // Right length, wrong sub-command echoed.
        assert_eq!(decode_tune_read(&[0xFC; 11]), None);
        // Right sub-command, too short to hold the fields.
        assert_eq!(decode_tune_read(&[0xF5, 0x00, 0x0E, 0x57]), None);
    }

    #[test]
    fn the_tuning_word_is_the_fraction_of_the_clock_scaled_by_two_to_the_32() {
        let clock = 122_880_000.0;
        // Nothing at DC.
        assert_eq!(tuning_word(0.0, clock), 0);
        // A quarter, an eighth and a half of the clock are exact.
        assert_eq!(tuning_word(clock / 4.0, clock), 0x4000_0000);
        assert_eq!(tuning_word(clock / 8.0, clock), 0x2000_0000);
        assert_eq!(tuning_word(clock / 2.0, clock), 0x8000_0000);
        // 14.074 MHz, computed the long way round.
        let want = (4_294_967_296.0f64 * 14_074_000.0 / clock) as u32;
        assert_eq!(tuning_word(14_074_000.0, clock), want);
        // The modulo is what lets a sampler alias above its own clock rather
        // than saturating at the top of the word.
        assert_eq!(tuning_word(clock, clock), 0);
        assert_eq!(tuning_word(clock + 14_074_000.0, clock), want);
        // The S1's clock is half, so the same dial is twice the phase step.
        assert_eq!(
            tuning_word(10_000_000.0, 61_440_000.0),
            tuning_word(20_000_000.0, 122_880_000.0)
        );
    }

    /// One byte apart, and it is the byte that decides whether the request is a
    /// tune at all: on the DUO the high half of `wIndex` is the sub-command
    /// selector.
    #[test]
    fn the_duo_and_the_samplers_pack_the_tuning_word_differently() {
        let word = 0x1234_5678u32;
        let (v, i, d) = tune_request(Model::S2, word, false);
        assert_eq!(v, 0x5678);
        assert_eq!(i, 0x1234);
        assert_eq!(d, [0x12, 0x00]);

        let (v, i, d) = tune_request(Model::Duo, word, false);
        assert_eq!(v, 0x5678);
        // 0xF2 in the high byte — the sub-command — and only bits 23..16 of the
        // word beside it.
        assert_eq!(i, 0xF234);
        assert_eq!(i >> 8, DuoSub::Tune as u16);
        assert_eq!(d, [0x12, 0x00]);

        // The S1 packs like the S2, not like the DUO.
        assert_eq!(tune_request(Model::S1, word, false), tune_request(Model::S2, word, false));
    }

    /// Two tunes from the SDR Console capture of a DUO in remote mode: 10 MHz
    /// and 7.076018 MHz. Both words fit a clock 1 Hz under nominal, not the
    /// 2678 Hz correction above, so SDR Console calibrates its own way.
    #[test]
    fn a_remote_duo_tunes_with_0x02_in_the_payload() {
        let clock = 122_880_000.0 - 1.0;
        let (v, i, d) = tune_request(Model::Duo, tuning_word(10_000_000.0, clock), true);
        assert_eq!((v, i, d), (0x5558, 0xF2D5, [0x14, 0x02]));
        let (v, i, d) = tune_request(Model::Duo, tuning_word(7_076_018.0, clock), true);
        assert_eq!((v, i, d), (0xE055, 0xF2BD, [0x0E, 0x02]));
        // Only the DUO has a remote mode.
        assert_eq!(tune_request(Model::S2, 1, true).2, [0, 0]);
    }

    #[test]
    fn the_duo_sub_commands_live_in_the_high_byte_of_windex() {
        assert_eq!(DuoSub::FifoRun.index(), 0xE900);
        assert_eq!(DuoSub::FifoInit.index(), 0xE800);
        assert_eq!(DuoSub::CatWrite.index(), 0xF100);
        assert_eq!(DuoSub::Status.index(), 0xFC00);
        // Nothing in the low byte, so a caller can OR its own into it.
        for s in [DuoSub::FifoRun, DuoSub::FifoInit, DuoSub::CatWrite, DuoSub::Status] {
            assert_eq!(s.index() & 0x00FF, 0);
        }
    }

    #[test]
    fn the_sample_width_changes_only_at_the_top_rate() {
        for rate in [192_000, 384_000, 768_000, 1_536_000, 3_072_000] {
            assert_eq!(component_bytes(rate), 4, "{rate}");
            assert_eq!(sample_bytes(rate), 8, "{rate}");
        }
        assert_eq!(component_bytes(6_144_000), 2);
        assert_eq!(sample_bytes(6_144_000), 4);
        // A whole number of samples has to fit a transfer at every rate, or a
        // carry would be needed on every block rather than occasionally.
        for rate in sdroxide_types::ELAD_SAMPLE_RATES {
            assert_eq!(TRANSFER_BYTES % sample_bytes(rate), 0, "{rate}");
        }
    }

    #[test]
    fn the_rescale_table_is_the_one_gr_elad_uses() {
        assert_eq!(rate_rescale_db(192_000), 0.0);
        assert_eq!(rate_rescale_db(384_000), 6.0);
        assert_eq!(rate_rescale_db(768_000), 6.0);
        assert_eq!(rate_rescale_db(1_536_000), 5.4);
        assert_eq!(rate_rescale_db(3_072_000), 5.4);
        assert_eq!(rate_rescale_db(6_144_000), -0.7);
        // An unknown rate sits at unity rather than at some interpolation.
        assert_eq!(rate_rescale_db(48_000), 0.0);
    }

    #[test]
    fn the_scale_adds_the_offsets_that_are_in_circuit_and_no_others() {
        let cal = Calibration { rate_correction_hz: 0, global_db: 1.0, lp_db: 2.0, att_db: -12.0 };
        let db = |s: f32| 20.0 * s.log10();
        // A sampler at 192 kHz with nothing switched in is the global offset
        // alone.
        assert!((db(cal.scale(Model::S2, 192_000, false, false)) - 1.0).abs() < 1e-3);
        // Each switch adds its own offset, and only when it is in circuit.
        assert!((db(cal.scale(Model::S2, 192_000, true, false)) - 3.0).abs() < 1e-3);
        assert!((db(cal.scale(Model::S2, 192_000, false, true)) - (-11.0)).abs() < 1e-3);
        assert!((db(cal.scale(Model::S2, 192_000, true, true)) - (-9.0)).abs() < 1e-3);
        // The DUO carries ELAD's fixed 21.4 dB on top; the samplers do not.
        let duo = db(cal.scale(Model::Duo, 192_000, false, false));
        let s2 = db(cal.scale(Model::S2, 192_000, false, false));
        assert!((duo - s2 - 21.4).abs() < 1e-3);
        // And the rate term rides along with it.
        assert!((db(cal.scale(Model::S2, 384_000, false, false)) - 7.0).abs() < 1e-3);
    }

    /// An unprogrammed EEPROM reads as `0xFF` bytes, which is a NaN. Letting one
    /// through would multiply every sample by NaN and silence the receiver with
    /// nothing in the log to say why.
    #[test]
    fn a_calibration_field_that_is_not_a_number_is_refused() {
        assert_eq!(eeprom_f32(&[0xFF, 0xFF, 0xFF, 0xFF]), None);
        assert_eq!(eeprom_f32(&0f32.to_le_bytes()), Some(0.0));
        assert_eq!(eeprom_f32(&(-1.5f32).to_le_bytes()), Some(-1.5));
        assert_eq!(eeprom_f32(&[0x00, 0x00]), None);
        assert_eq!(eeprom_i32(&(-1234i32).to_le_bytes()), Some(-1234));
        assert_eq!(eeprom_i32(&[0x00]), None);
    }

    #[test]
    fn an_eeprom_string_stops_at_the_first_nul() {
        let mut buf = [0u8; 32];
        buf[..6].copy_from_slice(b"123456");
        assert_eq!(eeprom_string(&buf).as_deref(), Some("123456"));
        // A field that was never written has nothing in it to report.
        assert_eq!(eeprom_string(&[0u8; 32]), None);
        assert_eq!(eeprom_string(&[0xFFu8; 32]), None);
    }

    #[test]
    fn the_s2_filter_code_follows_the_band_and_the_attenuator_rides_in_bit_3() {
        // Bypass is a code of its own, whatever the dial is doing.
        assert_eq!(s2_front_end_code(7_000_000.0, false, false), 0x01);
        // HF, then the two aliasing bands above the clock's halves.
        assert_eq!(s2_front_end_code(7_000_000.0, true, false), 0x03);
        assert_eq!(s2_front_end_code(100_000_000.0, true, false), 0x02);
        assert_eq!(s2_front_end_code(140_000_000.0, true, false), 0x04);
        // The attenuator is orthogonal to all of it.
        assert_eq!(s2_front_end_code(7_000_000.0, true, true), 0x0B);
        assert_eq!(s2_front_end_code(7_000_000.0, false, true), 0x09);
    }

    #[test]
    fn a_pinned_serial_matches_however_it_was_copied() {
        assert!(serial_matches("", None));
        assert!(serial_matches("", Some("123456")));
        assert!(serial_matches("123456", Some("123456")));
        assert!(serial_matches("123456", Some("123456")));
        // Suffix either way, because other programs print these truncated.
        assert!(serial_matches("3456", Some("123456")));
        assert!(serial_matches("00123456", Some("123456")));
        assert!(serial_matches("ABCDEF", Some("abcdef")));
        assert!(!serial_matches("999999", Some("123456")));
        // A device that has not been opened has no serial, so it cannot be the
        // one that was pinned.
        assert!(!serial_matches("123456", None));
    }

    #[test]
    fn a_cat_answer_is_read_up_to_its_semicolon() {
        let mut r = vec![0xF0];
        r.extend_from_slice(b"GI0018002110000;");
        r.push(0x09);
        assert_eq!(decode_cat_reply(&r), "GI0018002110000;");
        assert_eq!(decode_cat_reply(&[0xFC, 0x25, 0x09]), "");
        assert_eq!(decode_cat_reply(&[]), "");
    }

    #[test]
    fn gi_says_whether_the_radio_transmits() {
        // SDR Console's capture, receiving at a wide rate.
        assert_eq!(gi_transmitting("GI0018002110000;"), Some(false));
        assert_eq!(gi_transmitting("GI0018012110000;"), Some(true));
        assert_eq!(gi_transmitting("GI0018022110000;"), Some(true));
        assert_eq!(gi_transmitting("RX0;"), None);
        assert_eq!(gi_transmitting("GI00;"), None);
        assert_eq!(gi_transmitting(""), None);
    }
}
