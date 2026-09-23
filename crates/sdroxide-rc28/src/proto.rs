//! The RC-28's reports, and nothing that touches a device.
//!
//! Read off a unit on the bench (firmware `102 3210`) rather than out of a
//! datasheet, since Icom publishes none. Both directions are 32-byte reports
//! with no report id.
//!
//! In, byte 0 says what the report is:
//!
//! ```text
//! 01 cc 00 dd 00 kk 00 …   state: cc steps of the knob in direction dd
//!                          (1 clockwise, 2 anticlockwise, 0 still), and kk
//!                          the buttons, active low — bit 0 TRANSMIT,
//!                          bit 1 F-1, bit 2 F-2. 0x07 is nothing pressed.
//! 02 "102 3210" 00 …       the firmware version, in answer to 02 below.
//! ```
//!
//! A state report arrives every 10 ms or so while anything is moving and not
//! at all otherwise. Byte 4 was seen at 1 during a fast spin and carries no
//! count that the totals could account for, so it is ignored.
//!
//! Out:
//!
//! ```text
//! 01 ll 00 …               LEDs, active low — bit 0 TRANSMIT, bit 1 F-1,
//!                          bit 2 F-2, bit 3 LINK.
//! 02 00 …                  ask for the firmware version.
//! ```

/// USB vendor and product id: Icom's own, on a Prolific part.
pub const VID: u16 = 0x0c26;
pub const PID: u16 = 0x001e;

/// Length of every report in both directions, report id excluded.
pub const REPORT_LEN: usize = 32;

/// One of the three buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Transmit,
    F1,
    F2,
}

impl Key {
    pub const ALL: [Key; 3] = [Key::Transmit, Key::F1, Key::F2];

    /// The key's bit, in both the button byte and the LED byte.
    pub const fn bit(self) -> u8 {
        match self {
            Key::Transmit => 0x01,
            Key::F1 => 0x02,
            Key::F2 => 0x04,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Key::Transmit => "TRANSMIT",
            Key::F1 => "F-1",
            Key::F2 => "F-2",
        }
    }
}

/// The LINK LED's bit. The other three share their key's bit.
pub const LED_LINK: u8 = 0x08;

/// A report from the device, parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Report {
    /// The knob moved `steps` (positive clockwise), and/or the buttons are now
    /// `keys` — a bitmask of [`Key::bit`], set for *pressed*.
    State {
        steps: i32,
        keys: u8,
    },
    Firmware(String),
}

impl Report {
    /// `None` for anything that is not one of the two reports above.
    pub fn parse(bytes: &[u8]) -> Option<Report> {
        match *bytes.first()? {
            0x01 if bytes.len() >= 6 => {
                let n = i32::from(bytes[1]);
                let steps = match bytes[3] {
                    1 => n,
                    2 => -n,
                    _ => 0,
                };
                Some(Report::State { steps, keys: !bytes[5] & 0x07 })
            }
            0x02 => {
                let text = &bytes[1..];
                let end = text.iter().position(|b| *b == 0).unwrap_or(text.len());
                Some(Report::Firmware(String::from_utf8_lossy(&text[..end]).trim().to_string()))
            }
            _ => None,
        }
    }
}

/// Something the operator did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    /// Knob steps, positive clockwise.
    Turn(i32),
    Key {
        key: Key,
        down: bool,
    },
}

/// Turns the level-triggered button byte into presses and releases.
#[derive(Debug, Clone, Copy, Default)]
pub struct Decoder {
    keys: u8,
}

impl Decoder {
    /// Everything a report says happened, turn first, then keys in
    /// [`Key::ALL`] order.
    pub fn feed(&mut self, report: &Report) -> Vec<Input> {
        let Report::State { steps, keys } = *report else { return Vec::new() };
        let mut out = Vec::new();
        if steps != 0 {
            out.push(Input::Turn(steps));
        }
        let changed = keys ^ self.keys;
        for key in Key::ALL {
            if changed & key.bit() != 0 {
                out.push(Input::Key { key, down: keys & key.bit() != 0 });
            }
        }
        self.keys = keys;
        out
    }

    /// The device is gone: whatever it was holding is released, and the next
    /// device starts from nothing held.
    pub fn reset(&mut self) {
        self.keys = 0;
    }
}

/// Knob counts the app treats as one detent.
///
/// The knob has no detents and counts about 670 to the turn, ten times as
/// finely as a detented encoder — fine enough that every step would be a
/// fraction of the binding's step, which actions that land on whole units (an
/// RIT offset in hertz) would round away. So counts are gathered into whole
/// detents, about 67 to the turn, and the step and acceleration mean what they
/// mean for any other knob.
pub const COUNTS_PER_DETENT: i32 = 10;

/// Gathers knob counts into whole detents, carrying the remainder.
#[derive(Debug, Clone, Copy, Default)]
pub struct Detents {
    carry: i32,
}

impl Detents {
    /// Add `counts`, and return how many whole detents that completes. A turn
    /// back the other way spends the carry first, so a wiggle nets out.
    pub fn add(&mut self, counts: i32) -> i32 {
        self.carry += counts;
        let d = self.carry / COUNTS_PER_DETENT;
        self.carry -= d * COUNTS_PER_DETENT;
        d
    }

    pub fn reset(&mut self) {
        self.carry = 0;
    }
}

/// The report that lights exactly the LEDs in `lit` (a mask of [`Key::bit`]
/// and [`LED_LINK`]).
pub fn led_report(lit: u8) -> [u8; REPORT_LEN] {
    let mut r = [0u8; REPORT_LEN];
    r[0] = 0x01;
    r[1] = !lit & 0x0f;
    r
}

/// The report that asks for the firmware version.
pub fn firmware_request() -> [u8; REPORT_LEN] {
    let mut r = [0u8; REPORT_LEN];
    r[0] = 0x02;
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(bytes: &[u8]) -> [u8; REPORT_LEN] {
        let mut r = [0u8; REPORT_LEN];
        r[..bytes.len()].copy_from_slice(bytes);
        r
    }

    /// Reports captured from a real RC-28, verbatim apart from the zero tail.
    #[test]
    fn the_knob_reads_as_signed_steps() {
        let cw = report(&[0x01, 0x03, 0x00, 0x01, 0x00, 0x07]);
        let ccw = report(&[0x01, 0x0e, 0x00, 0x02, 0x01, 0x07]);
        assert_eq!(Report::parse(&cw), Some(Report::State { steps: 3, keys: 0 }));
        assert_eq!(Report::parse(&ccw), Some(Report::State { steps: -14, keys: 0 }));
    }

    #[test]
    fn the_buttons_are_active_low() {
        let tx = report(&[0x01, 0x00, 0x00, 0x00, 0x00, 0x06]);
        let f1 = report(&[0x01, 0x00, 0x00, 0x00, 0x00, 0x05]);
        let f2 = report(&[0x01, 0x00, 0x00, 0x00, 0x00, 0x03]);
        let parse = |r: &[u8]| match Report::parse(r) {
            Some(Report::State { keys, .. }) => keys,
            other => panic!("{other:?}"),
        };
        assert_eq!(parse(&tx), Key::Transmit.bit());
        assert_eq!(parse(&f1), Key::F1.bit());
        assert_eq!(parse(&f2), Key::F2.bit());
    }

    #[test]
    fn the_firmware_reply_is_text() {
        let mut r = report(&[0x02]);
        r[1..9].copy_from_slice(b"102 3210");
        assert_eq!(Report::parse(&r), Some(Report::Firmware("102 3210".into())));
    }

    #[test]
    fn anything_else_is_not_a_report() {
        assert_eq!(Report::parse(&[]), None);
        assert_eq!(Report::parse(&report(&[0x05, 0x01])), None);
        // Too short to hold the button byte.
        assert_eq!(Report::parse(&[0x01, 0x01, 0x00]), None);
    }

    #[test]
    fn the_decoder_reports_edges_not_levels() {
        let mut d = Decoder::default();
        let press = Report::State { steps: 0, keys: Key::F1.bit() };
        assert_eq!(d.feed(&press), vec![Input::Key { key: Key::F1, down: true }]);
        // The same level again — the knob moving while F-1 is held — is not a
        // second press.
        let turn = Report::State { steps: 2, keys: Key::F1.bit() };
        assert_eq!(d.feed(&turn), vec![Input::Turn(2)]);
        let release = Report::State { steps: 0, keys: 0 };
        assert_eq!(d.feed(&release), vec![Input::Key { key: Key::F1, down: false }]);
        assert_eq!(d.feed(&Report::Firmware("x".into())), vec![]);
    }

    #[test]
    fn a_reset_forgets_what_was_held() {
        let mut d = Decoder::default();
        d.feed(&Report::State { steps: 0, keys: Key::Transmit.bit() });
        d.reset();
        let again = Report::State { steps: 0, keys: Key::Transmit.bit() };
        assert_eq!(d.feed(&again), vec![Input::Key { key: Key::Transmit, down: true }]);
    }

    #[test]
    fn counts_become_whole_detents_with_the_rest_carried() {
        let mut d = Detents::default();
        assert_eq!(d.add(7), 0);
        assert_eq!(d.add(7), 1);
        assert_eq!(d.add(-4), 0, "the carry of 4 is spent first");
        assert_eq!(d.add(-25), -2);
        assert_eq!(d.add(-5), -1);
        d.reset();
        assert_eq!(d.add(9), 0);
    }

    /// Checked against the unit: each of these lit exactly the LED named.
    #[test]
    fn the_leds_are_active_low() {
        assert_eq!(led_report(0)[..2], [0x01, 0x0f]);
        assert_eq!(led_report(Key::Transmit.bit())[..2], [0x01, 0x0e]);
        assert_eq!(led_report(Key::F1.bit())[..2], [0x01, 0x0d]);
        assert_eq!(led_report(Key::F2.bit())[..2], [0x01, 0x0b]);
        assert_eq!(led_report(LED_LINK)[..2], [0x01, 0x07]);
        assert_eq!(led_report(0x0f)[..2], [0x01, 0x00]);
        assert_eq!(firmware_request()[0], 0x02);
    }
}
