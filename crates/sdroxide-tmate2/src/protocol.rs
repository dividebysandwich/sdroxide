//! The TMate 2's reports, byte for byte, with no I/O.
//!
//! Taken from two independent public descriptions of the device, which agree:
//! OpenTMate2Lib (MIT, Jan Svabenik, from USB captures of ELAD's own DLL) and
//! microenh/Tmate2_C (a hidapi driver). Both read the same 64-byte interrupt
//! report:
//!
//! | Offset | Size | Meaning                                             |
//! |--------|------|-----------------------------------------------------|
//! | 0      | 1    | 0x01 on every report seen                           |
//! | 1      | 2    | main dial counter, little-endian, wraps at 16 bits  |
//! | 3      | 2    | E2 dial counter                                     |
//! | 5      | 2    | E1 dial counter                                     |
//! | 7      | 2    | keys, little-endian, **active low**, idle `0x01FF`  |
//!
//! The dials report a running count rather than a step, so a step is the
//! difference between two reports, folded back into ±32767 across the wrap.
//! The device also accelerates in hardware: the per-detent increment and the
//! speeds at which it switches are part of the output report below.

use sdroxide_types::Tmate2Control;

pub const VENDOR_ID: u16 = 0x1721;
pub const PRODUCT_ID: u16 = 0x0614;
/// Both directions, without any report-id byte a platform puts in front.
pub const REPORT_LEN: usize = 64;

/// Key masks in the 16-bit key word. A *cleared* bit is a pressed key.
pub const KEYS: [(u16, Tmate2Control); 9] = [
    (0x0001, Tmate2Control::F1),
    (0x0002, Tmate2Control::F2),
    (0x0004, Tmate2Control::F3),
    (0x0008, Tmate2Control::F4),
    (0x0010, Tmate2Control::F5),
    (0x0020, Tmate2Control::F6),
    (0x0040, Tmate2Control::MainPush),
    (0x0080, Tmate2Control::E2Push),
    (0x0100, Tmate2Control::E1Push),
];

/// The key word when nothing is pressed.
pub const KEYS_IDLE: u16 = 0x01FF;

/// One decoded input report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputReport {
    pub main: u16,
    pub e1: u16,
    pub e2: u16,
    pub keys: u16,
}

impl InputReport {
    /// Decode a report as it came off the wire (no platform report-id byte in
    /// front). `None` for anything too short to hold the dials and keys.
    pub fn parse(r: &[u8]) -> Option<InputReport> {
        if r.len() < 9 {
            return None;
        }
        let w = |i: usize| u16::from_le_bytes([r[i], r[i + 1]]);
        Some(InputReport { main: w(1), e2: w(3), e1: w(5), keys: w(7) })
    }

    pub fn pressed(&self, mask: u16) -> bool {
        self.keys & mask == 0
    }
}

/// Signed step between two readings of a 16-bit counter that wraps.
pub fn counter_delta(now: u16, before: u16) -> i32 {
    now.wrapping_sub(before) as i16 as i32
}

/// Something the operator did, found by comparing two reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    Dial { control: Tmate2Control, delta: i32 },
    Key { control: Tmate2Control, down: bool },
}

/// Everything that differs between `before` and `now`, dials first.
pub fn changes(before: &InputReport, now: &InputReport) -> Vec<Change> {
    let mut out = Vec::new();
    for (control, a, b) in [
        (Tmate2Control::MainDial, before.main, now.main),
        (Tmate2Control::E1Dial, before.e1, now.e1),
        (Tmate2Control::E2Dial, before.e2, now.e2),
    ] {
        let delta = counter_delta(b, a);
        if delta != 0 {
            out.push(Change::Dial { control, delta });
        }
    }
    for (mask, control) in KEYS {
        let (was, is) = (before.pressed(mask), now.pressed(mask));
        if was != is {
            out.push(Change::Key { control, down: is });
        }
    }
    out
}

// ── Output report ────────────────────────────────────────────────────────────
//
// Bytes 0..32 are LCD segments (left blank here; the display is a later step),
// then a status byte and the settings ELAD's DLL sends, as OpenTMate2Lib
// recorded them from hardware.

const STATUS: usize = 32;
const BACKLIGHT_R: usize = 33;
const BACKLIGHT_G: usize = 34;
const BACKLIGHT_B: usize = 35;
const CONTRAST: usize = 36;
const REFRESH: usize = 37;
const SPEED: usize = 38; // 38, 39, 40: increment per detent at each speed
const THRESHOLD: usize = 41; // 41, 42: when the speed steps up
const EVAL_TIME: usize = 43;

/// Status-byte bit: the "USB" LED, lit while a program is driving the device.
pub const LED_USB: u8 = 0x01;
/// Status-byte bit: the lock LED.
pub const LED_LOCK: u8 = 0x02;

/// The report sent on connect and periodically after: blank LCD, the USB LED
/// on, ELAD's backlight and contrast, and the hardware dial acceleration ELAD
/// ships (1, 5 and 10 counts per detent).
pub fn idle_output() -> [u8; REPORT_LEN] {
    let mut r = [0u8; REPORT_LEN];
    r[STATUS] = LED_USB;
    r[BACKLIGHT_R] = 0x00;
    r[BACKLIGHT_G] = 0x32;
    r[BACKLIGHT_B] = 0xFF;
    r[CONTRAST] = 0x28;
    r[REFRESH] = 0x28;
    r[SPEED..SPEED + 3].copy_from_slice(&[1, 5, 10]);
    r[THRESHOLD..THRESHOLD + 2].copy_from_slice(&[15, 25]);
    r[EVAL_TIME] = 10;
    r
}

/// The report sent on the way out: as [`idle_output`] with the USB LED off,
/// so the panel shows nothing is driving it any more.
pub fn closing_output() -> [u8; REPORT_LEN] {
    let mut r = idle_output();
    r[STATUS] = 0;
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The idle report OpenTMate2Lib captured: `01 4A 00 0B 00 FE FF FF 01`.
    #[test]
    fn decodes_the_captured_idle_report() {
        let mut r = [0u8; 64];
        r[..9].copy_from_slice(&[0x01, 0x4A, 0x00, 0x0B, 0x00, 0xFE, 0xFF, 0xFF, 0x01]);
        let p = InputReport::parse(&r).unwrap();
        assert_eq!(p.main, 0x004A);
        assert_eq!(p.e2, 0x000B);
        assert_eq!(p.e1, 0xFFFE);
        assert_eq!(p.keys, KEYS_IDLE);
        assert!(KEYS.iter().all(|(m, _)| !p.pressed(*m)));
    }

    #[test]
    fn short_reports_are_refused() {
        assert_eq!(InputReport::parse(&[1, 2, 3]), None);
    }

    #[test]
    fn counter_delta_folds_across_the_wrap() {
        assert_eq!(counter_delta(97, 79), 18);
        assert_eq!(counter_delta(81, 97), -16);
        assert_eq!(counter_delta(0, 9), -9);
        assert_eq!(counter_delta(65535, 0), -1);
        assert_eq!(counter_delta(65520, 65535), -15);
        assert_eq!(counter_delta(11, 65520), 27);
        assert_eq!(counter_delta(2, 65534), 4);
    }

    /// Each confirmed key transition from the captures names one key.
    #[test]
    fn active_low_keys_map_to_the_panel() {
        let idle = InputReport { main: 0, e1: 0, e2: 0, keys: KEYS_IDLE };
        for (keys, control) in [
            (0x00FF, Tmate2Control::E1Push),
            (0x017F, Tmate2Control::E2Push),
            (0x01BF, Tmate2Control::MainPush),
            (0x01FE, Tmate2Control::F1),
            (0x01FD, Tmate2Control::F2),
            (0x01FB, Tmate2Control::F3),
            (0x01F7, Tmate2Control::F4),
            (0x01EF, Tmate2Control::F5),
            (0x01DF, Tmate2Control::F6),
        ] {
            let now = InputReport { keys, ..idle };
            assert_eq!(changes(&idle, &now), vec![Change::Key { control, down: true }]);
            assert_eq!(changes(&now, &idle), vec![Change::Key { control, down: false }]);
        }
    }

    #[test]
    fn dials_and_keys_in_one_report() {
        let a = InputReport { main: 100, e1: 0, e2: 5, keys: KEYS_IDLE };
        let b = InputReport { main: 103, e1: 65535, e2: 5, keys: 0x01FE };
        assert_eq!(
            changes(&a, &b),
            vec![
                Change::Dial { control: Tmate2Control::MainDial, delta: 3 },
                Change::Dial { control: Tmate2Control::E1Dial, delta: -1 },
                Change::Key { control: Tmate2Control::F1, down: true },
            ]
        );
    }

    #[test]
    fn output_report_matches_the_captured_settings() {
        let r = idle_output();
        assert_eq!(&r[33..44], &[0x00, 0x32, 0xFF, 0x28, 0x28, 1, 5, 10, 15, 25, 10]);
        assert_eq!(r[STATUS], LED_USB);
        assert!(r[..32].iter().all(|b| *b == 0));
        assert!(r[44..].iter().all(|b| *b == 0));
        assert_eq!(closing_output()[STATUS], 0);
    }
}
