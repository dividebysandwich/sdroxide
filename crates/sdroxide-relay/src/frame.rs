//! The wire, with no port in it.
//!
//! Every byte any of these boards is ever sent is built here, by functions that
//! take a channel number and a state and return bytes. Nothing in this module
//! opens anything, blocks on anything, or knows what time it is — which is what
//! makes the fiddly half checkable against a datasheet with nothing plugged in.
//! The same split every native driver in this workspace makes.
//!
//! # The channel numbering trap
//!
//! Everything above this module numbers contacts from 1, the way the boards'
//! own silkscreens do. Two of the five families disagree on the wire: Numato
//! counts from zero, and dcttech's HID relays count from one. The conversion
//! lives here, once, with a test per board width — it is the single most likely
//! off-by-one in the whole subsystem and it belongs in the half that can be
//! tested.

use sdroxide_types::RelayFamily;

/// A set of contacts, one bit per channel, bit `n` being channel `n + 1`.
///
/// One word covers `sdroxide_types::MAX_CHANNEL`, which is far more contacts
/// than an amateur station sequences.
pub type ChannelMask = u32;

/// The bit for a 1-based channel number. Channel 0 and anything past the end of
/// the word are not representable and answer with no bits, which is what the
/// configuration's own range check exists to prevent reaching here.
pub fn bit(channel: u8) -> ChannelMask {
    if channel == 0 || u32::from(channel) > ChannelMask::BITS { 0 } else { 1u32 << (channel - 1) }
}

/// LCUS-1/2/4/8 and the boards that copy them.
///
/// Four bytes at 9600 8N1: a fixed `0xA0`, the 1-based channel, the state, and
/// a checksum that is simply the sum of the first three. The famous example
/// from every listing — `A0 01 01 A2` to close channel 1 — is this and nothing
/// else. The board does not answer.
pub mod lcus {
    /// The command that puts channel `ch` (1-based) in state `on`.
    pub fn set(ch: u8, on: bool) -> [u8; 4] {
        let state = u8::from(on);
        let sum = 0xA0u8.wrapping_add(ch).wrapping_add(state);
        [0xA0, ch, state, sum]
    }
}

/// KMtronic's USB relay controllers.
///
/// Three bytes at 9600 8N1: `FF`, the 1-based channel, the state. Unlike the
/// LCUS it will say what it is set to, which is worth having: a board that
/// disagrees with what it was told is a board with a flat supply or a stuck
/// relay, and that is a different fault from a dead cable.
pub mod kmtronic {
    pub fn set(ch: u8, on: bool) -> [u8; 3] {
        [0xFF, ch, u8::from(on)]
    }

    /// Ask channel `ch` what it is set to. The board answers with three bytes
    /// in the same shape.
    pub fn read(ch: u8) -> [u8; 3] {
        [0xFF, ch, 0x03]
    }

    /// The reply to [`read`]. `None` when it is not an answer to that question.
    pub fn decode_read(ch: u8, reply: &[u8]) -> Option<bool> {
        match reply {
            [0xFF, c, s] if *c == ch => Some(*s != 0),
            _ => None,
        }
    }

    pub const REPLY_LEN: usize = 3;
}

/// Numato Lab's USB relay modules.
///
/// A CDC ACM port speaking ASCII, so the baud rate is a formality. The channel
/// is **zero-based**, and how it is spelled depends on how wide the board is:
/// the 1/2/4/8/16-channel modules take a single hexadecimal digit (`0`–`9`,
/// `A`–`F`), and the 32- and 64-channel ones take two decimal digits. Both are
/// straight out of Numato's own manuals, and getting it wrong on a 16-channel
/// board means silently operating the wrong relay.
pub mod numato {
    /// How the board spells channel `ch` (1-based here, zero-based on the
    /// wire). `None` for a channel the format cannot express.
    pub fn channel_token(ch: u8) -> Option<String> {
        let n = ch.checked_sub(1)?;
        match n {
            0..=9 => Some(String::from((b'0' + n) as char)),
            10..=15 => Some(String::from((b'A' + (n - 10)) as char)),
            // Past a single hex digit the boards switch to two decimal digits.
            16..=63 => Some(format!("{n:02}")),
            _ => None,
        }
    }

    pub fn set(ch: u8, on: bool) -> Option<Vec<u8>> {
        let tok = channel_token(ch)?;
        Some(format!("relay {} {}\r", if on { "on" } else { "off" }, tok).into_bytes())
    }

    pub fn read(ch: u8) -> Option<Vec<u8>> {
        Some(format!("relay read {}\r", channel_token(ch)?).into_bytes())
    }

    /// Read one of the board's digital inputs — what makes a Numato worth the
    /// extra money for this job, because it is where a transceiver's SEND line
    /// can be wired.
    pub fn gpio_read(line: u8) -> Option<Vec<u8>> {
        Some(format!("gpio read {}\r", channel_token(line.saturating_add(1))?).into_bytes())
    }

    /// Pull a relay's on/off out of whatever the board sent back.
    ///
    /// Deliberately a search rather than a parse: the module echoes the command
    /// it was given, then a newline, then the answer, then its `>` prompt, and
    /// exactly how much of that arrives in one read is not worth predicting.
    ///
    /// **Only the words**, never a bare digit. The echo of `relay read 0` ends
    /// in the channel number, so accepting `0` as "open" would read every
    /// channel-1 query as an answer whether or not the board had replied yet —
    /// which is a stuck relay reported as a working one.
    pub fn decode_state(reply: &str) -> Option<bool> {
        reply.split(|c: char| !c.is_ascii_alphanumeric()).rev().find_map(|w| match w {
            "on" => Some(true),
            "off" => Some(false),
            _ => None,
        })
    }

    /// Pull a digital input's level out of a `gpio read` reply.
    ///
    /// This one *is* a bare digit, so the echo has to be got out of the way
    /// first: everything up to the end of the first line is the command coming
    /// back, and the channel number is in it.
    pub fn decode_gpio(reply: &str) -> Option<bool> {
        let after_echo = match reply.find(['\r', '\n']) {
            Some(i) => &reply[i..],
            // No line ending yet means the echo is still arriving and the
            // answer certainly has not.
            None => return None,
        };
        after_echo.split(|c: char| !c.is_ascii_alphanumeric()).rev().find_map(|w| match w {
            "1" => Some(true),
            "0" => Some(false),
            _ => None,
        })
    }
}

/// The dcttech HID relay family — the "free-driver USB control switch" sold
/// under MagiDeal, LCUS-lookalike and a dozen other names, 1 to 8 channels,
/// all sharing the V-USB vendor id `16c0:05df`.
///
/// Everything happens through an 8-byte HID **feature report**: byte 0 is the
/// command, byte 1 the 1-based channel. Reading the same report back gives the
/// board's five-character serial in bytes 0..5 and its relay states as a
/// bitmask in byte 7.
pub mod dcttech {
    /// The vendor and product id every board in the family carries. Shared with
    /// a great many other V-USB hobby devices, which is why the product string
    /// has to be checked too.
    pub const USB_ID: (u16, u16) = (0x16c0, 0x05df);

    /// Noyito USB HID relay modules.
    pub const NOYITO_USB_ID: (u16, u16) = (0x5131, 0x2007);

    pub const USB_IDS: &[(u16, u16)] = &[USB_ID, NOYITO_USB_ID];

    /// What the product string starts with. The only thing distinguishing these
    /// boards from everything else on the shared id.
    pub const PRODUCT_PREFIX: &str = "USBRelay";
    pub const NOYITO_PRODUCT_PREFIX: &str = "HIDRelay";

    pub const REPORT_LEN: usize = 8;

    const CMD_ON: u8 = 0xFF;
    const CMD_OFF: u8 = 0xFD;
    const CMD_ALL_ON: u8 = 0xFE;
    const CMD_ALL_OFF: u8 = 0xFC;

    pub fn set(ch: u8, on: bool) -> [u8; REPORT_LEN] {
        let mut r = [0u8; REPORT_LEN];
        r[0] = if on { CMD_ON } else { CMD_OFF };
        r[1] = ch;
        r
    }

    /// Every contact at once. Used at shutdown, where "put the board back in
    /// receive" must not depend on knowing how many channels it has.
    pub fn set_all(on: bool) -> [u8; REPORT_LEN] {
        let mut r = [0u8; REPORT_LEN];
        r[0] = if on { CMD_ALL_ON } else { CMD_ALL_OFF };
        r
    }

    /// The board's serial and the state of its contacts, from a feature report
    /// read back.
    pub fn decode(report: &[u8]) -> Option<(String, u8)> {
        if report.len() < REPORT_LEN {
            return None;
        }
        let serial: String =
            report[..5].iter().take_while(|b| **b != 0).map(|b| *b as char).collect();
        Some((serial, report[7]))
    }
}

/// C-Media CM108/CM119 sound-card GPIO — by a wide margin the most common
/// transmit-switching interface in amateur radio, because every cheap USB
/// "rig interface" board is one.
///
/// A 5-byte HID **output** report, not a feature report: two zero bytes, the
/// output data, the direction mask, and a trailing zero. Bit `n − 1` is GPIO
/// `n`. Pin 3 is the near-universal choice because it is at the end of the
/// package and a wire can be tacked to it by hand.
pub mod cm108 {
    pub const REPORT_LEN: usize = 5;

    /// The pin every homebrew plan and every commercial board uses.
    pub const DEFAULT_PIN: u8 = 3;

    /// USB ids known to carry these GPIOs: C-Media's own parts, the SSS chips
    /// that clone them, and the AIOC, which emulates one in a microcontroller.
    pub const USB_IDS: &[(u16, u16)] = &[
        (0x0d8c, 0x0008),
        (0x0d8c, 0x0009),
        (0x0d8c, 0x000a),
        (0x0d8c, 0x000b),
        (0x0d8c, 0x000c),
        (0x0d8c, 0x000d),
        (0x0d8c, 0x000e),
        (0x0d8c, 0x000f),
        (0x0d8c, 0x0012),
        (0x0d8c, 0x0013),
        (0x0d8c, 0x0139),
        (0x0d8c, 0x013a),
        (0x0c76, 0x1605),
        (0x0c76, 0x1607),
        (0x0c76, 0x160b),
        (0x1209, 0x7388),
    ];

    /// Drive the pins in `mask` (bit `n − 1` for GPIO `n`) to the levels in
    /// `data`. Pins outside the mask are left as inputs, which is what keeps a
    /// card's other GPIOs — a squelch input, a COS line — out of this.
    pub fn report(data: u8, mask: u8) -> [u8; REPORT_LEN] {
        [0, 0, data, mask, 0]
    }

    /// The report that puts a single pin at a level, leaving the rest alone.
    pub fn set_pin(pin: u8, on: bool) -> [u8; REPORT_LEN] {
        let m = if pin == 0 || pin > 8 { 0 } else { 1u8 << (pin - 1) };
        report(if on { m } else { 0 }, m)
    }
}

/// One serial board's "set channel `ch` to `on`", whichever family it is.
///
/// `None` only for a channel the family cannot express, which the
/// configuration's range check should already have caught.
pub fn serial_set(family: RelayFamily, ch: u8, on: bool) -> Option<Vec<u8>> {
    match family {
        RelayFamily::Lcus => Some(lcus::set(ch, on).to_vec()),
        RelayFamily::KMtronic => Some(kmtronic::set(ch, on).to_vec()),
        RelayFamily::Numato => numato::set(ch, on),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The example printed on every LCUS listing there is. If this ever fails
    /// the board is being sent something nobody has ever tested.
    #[test]
    fn lcus_matches_the_published_example() {
        assert_eq!(lcus::set(1, true), [0xA0, 0x01, 0x01, 0xA2]);
        assert_eq!(lcus::set(1, false), [0xA0, 0x01, 0x00, 0xA1]);
    }

    #[test]
    fn the_lcus_checksum_is_the_sum_of_what_precedes_it() {
        for ch in 1..=8u8 {
            for on in [false, true] {
                let f = lcus::set(ch, on);
                let sum = f[0].wrapping_add(f[1]).wrapping_add(f[2]);
                assert_eq!(f[3], sum, "channel {ch} {on}");
            }
        }
        // Two spot values a reviewer can check by hand.
        assert_eq!(lcus::set(4, true), [0xA0, 0x04, 0x01, 0xA5]);
        assert_eq!(lcus::set(8, false), [0xA0, 0x08, 0x00, 0xA8]);
    }

    #[test]
    fn kmtronic_is_three_bytes_and_answers_in_the_same_shape() {
        assert_eq!(kmtronic::set(1, true), [0xFF, 0x01, 0x01]);
        assert_eq!(kmtronic::set(2, false), [0xFF, 0x02, 0x00]);
        assert_eq!(kmtronic::read(1), [0xFF, 0x01, 0x03]);
        assert_eq!(kmtronic::decode_read(1, &[0xFF, 0x01, 0x01]), Some(true));
        assert_eq!(kmtronic::decode_read(1, &[0xFF, 0x01, 0x00]), Some(false));
        assert_eq!(
            kmtronic::decode_read(1, &[0xFF, 0x02, 0x01]),
            None,
            "an answer about another channel is not an answer about this one"
        );
    }

    /// The conversion this module exists to get right: 1-based here,
    /// zero-based on the wire, and spelled differently on a wide board.
    #[test]
    fn numato_channels_are_zero_based_and_change_spelling_with_board_width() {
        assert_eq!(numato::channel_token(1).as_deref(), Some("0"), "channel 1 is relay 0");
        assert_eq!(numato::channel_token(2).as_deref(), Some("1"));
        assert_eq!(numato::channel_token(10).as_deref(), Some("9"));
        // 16-channel boards: a single hex digit.
        assert_eq!(numato::channel_token(11).as_deref(), Some("A"));
        assert_eq!(numato::channel_token(16).as_deref(), Some("F"));
        // 32- and 64-channel boards: two decimal digits.
        assert_eq!(numato::channel_token(17).as_deref(), Some("16"));
        assert_eq!(numato::channel_token(64).as_deref(), Some("63"));
        assert_eq!(numato::channel_token(0), None, "there is no channel 0");
        assert_eq!(numato::channel_token(65), None);
    }

    #[test]
    fn numato_commands_are_ascii_and_carriage_return_terminated() {
        assert_eq!(numato::set(2, true).unwrap(), b"relay on 1\r".to_vec());
        assert_eq!(numato::set(1, false).unwrap(), b"relay off 0\r".to_vec());
        assert_eq!(numato::set(11, true).unwrap(), b"relay on A\r".to_vec());
        assert_eq!(numato::read(1).unwrap(), b"relay read 0\r".to_vec());
    }

    #[test]
    fn numato_state_is_read_out_of_its_echo_and_prompt() {
        assert_eq!(numato::decode_state("relay read 0\n\ron\n\r>"), Some(true));
        assert_eq!(numato::decode_state("relay read 0\n\roff\n\r>"), Some(false));
        assert_eq!(numato::decode_state("relay read 0\n\r>"), None, "the echo alone says nothing");
        assert_eq!(
            numato::decode_state("relay read 1\n\r"),
            None,
            "and the channel number in the echo is not a state"
        );
    }

    /// The other half of the same trap: a `gpio read` answer *is* a bare digit,
    /// so the echoed channel number has to be stepped over rather than ignored.
    #[test]
    fn numato_inputs_are_read_past_the_echoed_channel_number() {
        assert_eq!(numato::decode_gpio("gpio read 0\n\r1\n\r>"), Some(true));
        assert_eq!(numato::decode_gpio("gpio read 1\n\r0\n\r>"), Some(false));
        assert_eq!(
            numato::decode_gpio("gpio read 1"),
            None,
            "a half-arrived echo is not an answer"
        );
    }

    #[test]
    fn dcttech_is_a_command_and_a_one_based_channel() {
        assert_eq!(dcttech::set(1, true), [0xFF, 1, 0, 0, 0, 0, 0, 0]);
        assert_eq!(dcttech::set(2, false), [0xFD, 2, 0, 0, 0, 0, 0, 0]);
        assert_eq!(dcttech::set_all(false), [0xFC, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(dcttech::set_all(true), [0xFE, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn a_dcttech_read_back_carries_the_serial_and_a_bitmask() {
        let report = [b'A', b'B', b'C', b'D', b'E', 0, 0, 0b0000_0101];
        let (serial, state) = dcttech::decode(&report).expect("well-formed");
        assert_eq!(serial, "ABCDE");
        assert_eq!(state, 0b101, "channels 1 and 3 are closed");
        assert_eq!(dcttech::decode(&[0xFF, 1]), None, "a short report is not a report");
    }

    #[test]
    fn cm108_pin_three_is_bit_two() {
        assert_eq!(cm108::set_pin(3, true), [0, 0, 0x04, 0x04, 0]);
        assert_eq!(cm108::set_pin(3, false), [0, 0, 0x00, 0x04, 0]);
        assert_eq!(cm108::set_pin(1, true), [0, 0, 0x01, 0x01, 0]);
        assert_eq!(
            cm108::set_pin(9, true),
            [0, 0, 0, 0, 0],
            "a pin the chip does not have drives nothing rather than wrapping onto pin 1"
        );
    }

    #[test]
    fn a_bit_is_one_less_than_its_channel_number() {
        assert_eq!(bit(1), 0b1);
        assert_eq!(bit(8), 0b1000_0000);
        assert_eq!(bit(0), 0, "channel 0 does not exist");
        assert_eq!(bit(33), 0, "and neither does one past the end of the word");
    }
}
