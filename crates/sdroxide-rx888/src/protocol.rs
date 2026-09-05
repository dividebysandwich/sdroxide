//! The RX-888 Mk2 wire protocol: USB ids, vendor commands, GPIO bits and the
//! `SETARGFX3` parameter space.
//!
//! Transcribed from the MIT-licensed rx888-firmware USB API reference
//! (<https://ringof.github.io/rx888-firmware/api/>), which documents the
//! firmware image vendored in `firmware/`. Values are stated there rather than
//! derived, so this module is a table and nothing else — the moment it starts
//! containing logic, that logic belongs in `device.rs`.

/// Cypress's vendor id, used by both the bootloader and the programmed device.
pub const VID_CYPRESS: u16 = 0x04b4;

/// The bare FX3 boot ROM, before any firmware has been uploaded.
///
/// The RX-888 has no populated boot EEPROM, so it comes up like this on *every*
/// plug-in — this is the normal state of a healthy receiver, not a fault.
pub const PID_FX3_BOOTLOADER: u16 = 0x00f3;

/// The receiver once our firmware is running.
pub const PID_RX888: u16 = 0x00f1;

/// The bulk IN endpoint carrying the sample stream.
pub const BULK_EP: u8 = 0x81;

/// The endpoint's SuperSpeed packet size. Every transfer length must be a
/// multiple of it.
pub const BULK_PACKET_SS: usize = 1024;

/// The endpoint's High-Speed packet size, used when the link only trained to
/// USB 2.0.
pub const BULK_PACKET_HS: usize = 512;

/// EP0 cannot carry more than this in one vendor request; longer ones STALL.
pub const EP0_MAX: usize = 64;

/// Vendor requests understood by the programmed device.
///
/// The bootloader understands none of these — it speaks only `0xA0`
/// (see [`crate::firmware`]).
#[allow(dead_code)] // several are diagnostics used only by examples/probe.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Cmd {
    /// Start the GPIF engine and the sample stream.
    StartFx3 = 0xaa,
    /// Halt the GPIF engine and drain DMA.
    StopFx3 = 0xab,
    /// IN, 4 bytes: hardware config and firmware version.
    TestFx3 = 0xac,
    /// OUT, 4 bytes: the whole GPIO word (see [`gpio`]).
    GpioFx3 = 0xad,
    /// OUT: I2C write passthrough.
    I2cWrite = 0xae,
    /// IN: I2C read passthrough.
    I2cRead = 0xaf,
    /// Hard reset back to the boot ROM.
    ResetFx3 = 0xb1,
    /// OUT, 4 bytes: program the Si5351 ADC clock, in Hz, u32 little-endian.
    StartAdc = 0xb2,
    /// IN: the diagnostic block described by [`Stats`].
    GetStats = 0xb3,
    /// OUT: set one parameter, selected by `wIndex` (see [`arg`]).
    SetArgFx3 = 0xb6,
}

impl Cmd {
    pub fn code(self) -> u8 {
        self as u8
    }
}

/// Bits of the `GPIOFX3` control word.
///
/// The firmware applies the word verbatim and keeps no shadow copy of its own,
/// so the host owns every bit: a caller that wants to change one must send all
/// of them. `device.rs` therefore holds the authoritative shadow register, and
/// nothing else in this crate may write GPIO.
pub mod gpio {
    /// Front-end shutdown (active high).
    pub const SHDWN: u32 = 1 << 5;
    /// LTC2208 dither enable — trades a little noise floor for far better
    /// spurious-free dynamic range.
    pub const DITH: u32 = 1 << 6;
    /// LTC2208 output randomizer. On, the ADC scrambles its own output to keep
    /// the digital bus from radiating into the analogue front end; the host has
    /// to undo it (see `crate::convert::derandomize`).
    pub const RANDO: u32 = 1 << 7;
    /// HF antenna bias-tee power.
    pub const BIAS_HF: u32 = 1 << 8;
    /// VHF antenna bias-tee power.
    pub const BIAS_VHF: u32 = 1 << 9;
    /// Blue front-panel LED.
    pub const LED_BLUE: u32 = 1 << 11;
    /// Attenuator bank select, bit 0.
    pub const ATT_SEL0: u32 = 1 << 13;
    /// Attenuator bank select, bit 1.
    pub const ATT_SEL1: u32 = 1 << 14;
    /// VHF front end enable. Unused while this backend is HF-only, but kept so
    /// the VHF path can be added without revisiting the protocol layer.
    pub const VHF_EN: u32 = 1 << 15;
    /// LTC2208 input-range select, not a preamplifier despite the name.
    ///
    /// The bit drives the ADC's `PGA` pin, which chooses between a 2.25 Vp-p and
    /// a 1.5 Vp-p full-scale input. The firmware inverts it (`GPIO_PGA` low when
    /// the bit is set), so **setting** this bit selects the *wider* 2.25 Vp-p
    /// range — more headroom, fewer counts for a given input. Measured on the
    /// bench at a fixed VGA setting: set → 575 counts rms, clear → 2385.
    ///
    /// So it behaves as the opposite of an "enable", and the default is to set
    /// it, because headroom is what protects a wideband HF front end from the
    /// strongest broadcast carrier in the band.
    pub const PGA_EN: u32 = 1 << 16;
}

/// `SETARGFX3` parameter indices, passed in `wIndex`.
///
/// Unrecognised indices STALL, so these must match the firmware exactly.
pub mod arg {
    /// PE4312 step attenuator, 0..=63 in 0.5 dB steps (0 dB .. 31.5 dB).
    pub const DAT31_ATT: u16 = 10;
    /// AD8370 VGA, as the raw 8-bit register value.
    pub const AD8370_VGA: u16 = 11;
    /// Watchdog: how many stream recoveries before escalating. 0 = unlimited.
    pub const WDG_MAX_RECOV: u16 = 14;
    /// Watchdog: whether exhausting recoveries resets the device.
    pub const WDG_RESET_ESCALATE: u16 = 15;
}

/// The `GETSTATS` reply: 30 little-endian bytes of firmware health.
///
/// Worth reading during bring-up and when the stream misbehaves — `gpif_state`
/// and the error counters distinguish "the host is too slow" from "the device
/// stopped producing", which look identical from the sample side.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    /// DMA completions since `STARTFX3`.
    pub dma_count: u32,
    /// GPIF state machine: 1 = idle, 255 = error.
    pub gpif_state: u8,
    /// PIB error count.
    pub pib_errors: u32,
    /// Argument of the most recent PIB error.
    pub last_pib_arg: u16,
    /// I2C failure count.
    pub i2c_errors: u32,
    /// Streaming faults / recoveries.
    pub stream_faults: u32,
    /// Si5351 register 0 — PLL lock status.
    pub si5351_status: u8,
    /// Firmware init counter; increments on every (re)boot of the FX3.
    pub boot_count: u32,
    /// Si5351 `CLK0_CONTROL` (register 16). Bit 7 is power-down.
    pub si5351_clk0_control: u8,
    /// Whether CLK0 is enabled: 1 = on, 0 = off or error.
    pub clk0_enabled: u8,
    /// Live GPIO pin state, same bit mapping as [`gpio`].
    ///
    /// `None` on firmware that predates the field — see [`STATS_MIN_LEN`].
    pub gpio: Option<u32>,
}

/// How many bytes to ask `GETSTATS` for: the longest layout we understand.
pub const STATS_LEN: usize = 30;

/// The shortest reply we can still decode.
///
/// Measured, not documented: the published API reference describes a 30-byte
/// block ending in the live GPIO word, but firmware 2.3 — the version in the
/// image vendored here — returns 26 bytes and stops after `clk0_result`. Every
/// preceding field is at the same offset in both, so the GPIO word is simply
/// reported as absent rather than refusing to decode a block that is otherwise
/// perfectly good.
pub const STATS_MIN_LEN: usize = 26;

impl Stats {
    /// Decode a `GETSTATS` reply. Returns `None` only if the block is too short
    /// to contain even the fixed fields.
    pub fn parse(b: &[u8]) -> Option<Stats> {
        if b.len() < STATS_MIN_LEN {
            return None;
        }
        let u32_at = |i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
        Some(Stats {
            dma_count: u32_at(0),
            gpif_state: b[4],
            pib_errors: u32_at(5),
            last_pib_arg: u16::from_le_bytes([b[9], b[10]]),
            i2c_errors: u32_at(11),
            stream_faults: u32_at(15),
            si5351_status: b[19],
            boot_count: u32_at(20),
            si5351_clk0_control: b[24],
            clk0_enabled: b[25],
            gpio: (b.len() >= STATS_LEN).then(|| u32_at(26)),
        })
    }

    /// Whether the Si5351 is actually driving the ADC clock. A receiver whose
    /// CLK0 is off delivers a stream of nothing in particular, which is easy to
    /// mistake for a dead antenna.
    pub fn clock_running(&self) -> bool {
        self.clk0_enabled == 1 && self.si5351_clk0_control & 0x80 == 0
    }
}

/// Which receiver the firmware decided it is running on.
///
/// The firmware probes for an R828D tuner over I2C and senses GPIO36 at boot,
/// and **only initialises the front-end GPIO block when both say RX-888 Mk2**.
/// On a receiver it does not recognise it still enumerates, still clocks the
/// ADC and still streams — but every GPIO the host writes lands on a pin that
/// was never configured as an output, so the bias tee, the dither and
/// randomizer switches, the ADC range select and both gain stages silently do
/// nothing. That is why this byte matters enough to have a name.
pub mod hardware {
    /// The firmware's detection failed: no front-end control at all.
    pub const NO_RADIO: u8 = 0x00;
    /// The rest are the SDDC firmware's own board numbers, in its order — the
    /// same `radio_type_t` the bundled image was built from (see
    /// `firmware/PROVENANCE.md`). They are worth naming even though this driver
    /// steers none of their front ends: a board the firmware recognised
    /// perfectly well is a different thing from one it failed to identify, and
    /// telling an RX-888 Mk1 owner that detection failed sends them to unplug a
    /// receiver that is working exactly as it should (issue #342).
    pub const BBRF103: u8 = 0x01;
    pub const HF103: u8 = 0x02;
    /// RX-888 Mk1: the same LTC2208 and FX3 as the Mk2 behind a different front
    /// end — no PE4312 attenuator, no AD8370 VGA, and no VHF tuner. It streams
    /// here; nothing this driver offers in front of the ADC reaches it.
    pub const RX888: u8 = 0x03;
    /// RX-888 Mk2, the receiver this driver steers.
    pub const RX888R2: u8 = 0x04;
    pub const RX999: u8 = 0x05;
    pub const RXLUCY: u8 = 0x06;

    /// The board's name, or `None` for a number this table does not know.
    pub fn name(id: u8) -> Option<&'static str> {
        Some(match id {
            BBRF103 => "BBRF103",
            HF103 => "HF103",
            RX888 => "RX-888 Mk1",
            RX888R2 => "RX-888 Mk2",
            RX999 => "RX-999",
            RXLUCY => "RX-Lucy",
            _ => return None,
        })
    }
}

/// Firmware version as reported by `TESTFX3`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
}

/// What `TESTFX3` says about the device: which board the firmware thinks it is
/// driving, and which firmware that is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Identity {
    /// Hardware configuration byte — see [`hardware`].
    pub hardware: u8,
    pub version: Version,
}

impl Identity {
    /// Decode the 4-byte `TESTFX3` reply.
    ///
    /// The layout is `[hardware, major, minor, vendor-request count]`: the
    /// firmware fills `glEp0Buffer[0]` with `glHWconfig` and the next two bytes
    /// with `glFWconfig` (`major << 8 | minor`) *high byte first*, so the
    /// version is not a little-endian 16-bit field and must not be read as one.
    /// The receiver on the bench replies `[04, 02, 03, n]` — an RX-888 Mk2
    /// running firmware 2.3, which is the version the bundled image announces
    /// (see `firmware/PROVENANCE.md`).
    pub fn parse(b: &[u8]) -> Option<Identity> {
        Some(Identity {
            hardware: *b.first()?,
            version: Version { major: *b.get(1)?, minor: *b.get(2)? },
        })
    }

    /// Whether the firmware set up the front-end GPIO block. False means every
    /// switch this driver offers is a no-op — see [`hardware`].
    pub fn front_end_ready(&self) -> bool {
        self.hardware == hardware::RX888R2
    }

    /// Whether the firmware identified the board at all.
    ///
    /// Not the same question as [`Self::front_end_ready`], and conflating the
    /// two is issue #342: an RX-888 Mk1 answers `0x03`, which is the firmware
    /// saying "I know exactly what this is" about a board whose front end this
    /// driver does not steer. Only `0x00` means the detection failed, and only
    /// that is worth reloading the firmware over.
    pub fn recognised(&self) -> bool {
        hardware::name(self.hardware).is_some()
    }

    /// What the board is called, for anything an operator reads.
    pub fn board(&self) -> String {
        match hardware::name(self.hardware) {
            Some(n) => n.to_string(),
            None => format!("an unknown board (hardware id 0x{:02x})", self.hardware),
        }
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Issue #342: the firmware's own board numbers, and the difference between
    /// "I do not know what this is" and "I know, and it is not the one this
    /// driver steers".
    #[test]
    fn a_recognised_board_is_not_a_failed_detection() {
        let id = |h| Identity { hardware: h, version: Version { major: 2, minor: 3 } };

        // The Mk2: named, recognised, and driven.
        assert!(id(hardware::RX888R2).front_end_ready());
        assert!(id(hardware::RX888R2).recognised());
        assert_eq!(id(hardware::RX888R2).board(), "RX-888 Mk2");

        // The Mk1: named and recognised, but nothing here steers its front end.
        // What it must NOT do is look like a detection failure — that is what
        // sent its owner to re-plug a working receiver.
        assert!(id(hardware::RX888).recognised());
        assert!(!id(hardware::RX888).front_end_ready());
        assert_eq!(id(hardware::RX888).board(), "RX-888 Mk1");

        // The rest of the family is named too, rather than reported as a number.
        for h in [hardware::BBRF103, hardware::HF103, hardware::RX999, hardware::RXLUCY] {
            assert!(id(h).recognised(), "0x{h:02x}");
            assert!(!id(h).board().contains("unknown"), "0x{h:02x}");
        }

        // Detection genuinely failed, and a number nobody has heard of.
        assert!(!id(hardware::NO_RADIO).recognised());
        assert!(id(hardware::NO_RADIO).board().contains("unknown"));
        assert!(!id(0x7f).recognised());
        assert!(id(0x7f).board().contains("0x7f"));
    }

    #[test]
    fn stats_decode_matches_the_documented_offsets() {
        let mut b = [0u8; STATS_LEN];
        b[0..4].copy_from_slice(&0x1234_5678u32.to_le_bytes()); // dma_count
        b[4] = 1; // gpif idle
        b[5..9].copy_from_slice(&7u32.to_le_bytes()); // pib errors
        b[9..11].copy_from_slice(&0xbeefu16.to_le_bytes()); // last pib arg
        b[11..15].copy_from_slice(&3u32.to_le_bytes()); // i2c errors
        b[15..19].copy_from_slice(&2u32.to_le_bytes()); // stream faults
        b[19] = 0x11; // si5351 status
        b[20..24].copy_from_slice(&5u32.to_le_bytes()); // boot count
        b[24] = 0x0f; // clk0 control, PDN clear
        b[25] = 1; // clk0 on
        b[26..30].copy_from_slice(&(gpio::DITH | gpio::RANDO).to_le_bytes());

        let s = Stats::parse(&b).expect("full-length block parses");
        assert_eq!(s.dma_count, 0x1234_5678);
        assert_eq!(s.gpif_state, 1);
        assert_eq!(s.pib_errors, 7);
        assert_eq!(s.last_pib_arg, 0xbeef);
        assert_eq!(s.i2c_errors, 3);
        assert_eq!(s.stream_faults, 2);
        assert_eq!(s.boot_count, 5);
        assert_eq!(s.gpio, Some(gpio::DITH | gpio::RANDO));
        assert!(s.clock_running());
    }

    /// Firmware 2.3 stops after `clk0_result`. Captured from the real device:
    /// a freshly programmed receiver, before `STARTADC`, so the Si5351 is still
    /// powered down and this is exactly what a healthy one reports.
    #[test]
    fn the_26_byte_block_from_firmware_2_4_decodes() {
        let b: [u8; 26] = [
            0x00, 0x00, 0x00, 0x00, // dma_count
            0x00, // gpif_state
            0x00, 0x00, 0x00, 0x00, // pib_errors
            0x00, 0x00, // last_pib_arg
            0x00, 0x00, 0x00, 0x00, // i2c_errors
            0x00, 0x00, 0x00, 0x00, // stream_faults
            0x21, // si5351 status
            0x01, 0x00, 0x00, 0x00, // boot_count = 1, first boot after load
            0x80, // clk0 control: PDN set
            0x00, // clk0 not enabled
        ];
        let s = Stats::parse(&b).expect("a 26-byte block still decodes");
        assert_eq!(s.boot_count, 1);
        assert_eq!(s.si5351_status, 0x21);
        assert_eq!(s.gpio, None, "the GPIO word is absent, not zero");
        // Powered down and not enabled: correct before STARTADC has been sent.
        assert!(!s.clock_running());
    }

    #[test]
    fn a_powered_down_clk0_does_not_count_as_running() {
        let mut b = [0u8; STATS_LEN];
        b[24] = 0x80; // PDN set
        b[25] = 1;
        assert!(!Stats::parse(&b).unwrap().clock_running());
    }

    #[test]
    fn short_stats_blocks_are_rejected_rather_than_misread() {
        assert!(Stats::parse(&[0u8; STATS_MIN_LEN - 1]).is_none());
        assert!(Stats::parse(&[]).is_none());
        // Anything from the 26-byte layout upward decodes; only the trailing
        // GPIO word depends on the length.
        assert!(Stats::parse(&[0u8; STATS_MIN_LEN]).is_some());
        assert_eq!(Stats::parse(&[0u8; STATS_LEN]).unwrap().gpio, Some(0));
    }

    #[test]
    fn the_identity_reply_from_the_bundled_firmware_is_an_mk2_running_2_3() {
        // Captured from the real device after loading firmware/SDDC_FX3.img.
        // The trailing byte is the vendor-request counter and is ignored.
        let id = Identity::parse(&[0x04, 0x02, 0x03, 0x07]).expect("a 4-byte reply decodes");
        assert_eq!(id.hardware, hardware::RX888R2);
        assert_eq!(id.version, Version { major: 2, minor: 3 });
        assert_eq!(id.version.to_string(), "2.3");
        assert!(id.front_end_ready());
        assert_eq!(Identity::parse(&[0x04, 0x02]), None);
    }

    #[test]
    fn a_receiver_the_firmware_did_not_recognise_has_no_front_end_control() {
        // Detection failed: the firmware skipped `rx888r2_GpioInitialize`, so
        // every switch the host writes goes to an unconfigured pin.
        let id = Identity::parse(&[hardware::NO_RADIO, 0x02, 0x03, 0x00]).unwrap();
        assert!(!id.front_end_ready());
        // ...and the firmware version is still readable, which is what makes
        // this distinguishable from a device that answered with rubbish.
        assert_eq!(id.version, Version { major: 2, minor: 3 });
    }

    #[test]
    fn gpio_bits_match_the_published_masks() {
        // The API reference states these as masks rather than shifts; keeping
        // both forms in the test is what catches a typo in the shift.
        assert_eq!(gpio::SHDWN, 0x0000_0020);
        assert_eq!(gpio::DITH, 0x0000_0040);
        assert_eq!(gpio::RANDO, 0x0000_0080);
        assert_eq!(gpio::BIAS_HF, 0x0000_0100);
        assert_eq!(gpio::BIAS_VHF, 0x0000_0200);
        assert_eq!(gpio::LED_BLUE, 0x0000_0800);
        assert_eq!(gpio::ATT_SEL0, 0x0000_2000);
        assert_eq!(gpio::ATT_SEL1, 0x0000_4000);
        assert_eq!(gpio::VHF_EN, 0x0000_8000);
        assert_eq!(gpio::PGA_EN, 0x0001_0000);
    }
}
