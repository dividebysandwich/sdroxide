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
// Bytes 0..32 are the LCD's segments, one bit each and scattered across the
// bytes in no useful order; then a status byte and the settings ELAD's DLL
// sends, as OpenTMate2Lib recorded them from hardware. The segment positions
// below are OpenTMate2Lib's map, captured by setting every segment in turn
// through ELAD's DLL and recording what went over the wire.

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

/// One LCD segment: byte and bit.
#[derive(Debug, Clone, Copy)]
struct Seg(usize, u8);

mod seg {
    use super::Seg;
    pub const SMETER_LINE: Seg = Seg(2, 0x01);
    pub const DB_MINUS: Seg = Seg(28, 0x10);
    pub const DOT_MHZ: Seg = Seg(9, 0x10);
    pub const DOT_KHZ: Seg = Seg(15, 0x10);
    pub const HZ: Seg = Seg(23, 0x01);
    pub const S: Seg = Seg(0, 0x10);
    pub const NR: Seg = Seg(0, 0x40);
    pub const NB: Seg = Seg(1, 0x40);
    pub const A: Seg = Seg(1, 0x20);
    pub const B: Seg = Seg(2, 0x20);
    pub const AN: Seg = Seg(2, 0x40);
    pub const DSB: Seg = Seg(21, 0x10);
    pub const FM: Seg = Seg(21, 0x20);
    pub const USB: Seg = Seg(21, 0x40);
    pub const SAM: Seg = Seg(21, 0x80);
    pub const DRM: Seg = Seg(22, 0x01);
    pub const DIG: Seg = Seg(22, 0x02);
    pub const DBM: Seg = Seg(22, 0x10);
    pub const CW: Seg = Seg(22, 0x20);
    pub const LSB: Seg = Seg(22, 0x40);
    pub const AM: Seg = Seg(22, 0x80);
    pub const RX: Seg = Seg(0, 0x04);
    pub const TX: Seg = Seg(0, 0x08);
    pub const W: Seg = Seg(20, 0x20);
    pub const RIT: Seg = Seg(13, 0x10);
    pub const XIT: Seg = Seg(14, 0x10);
    /// The printed S-meter scale: 1 3 5 7 9 +20 +40 +60.
    pub const SCALE: [Seg; 8] = [
        Seg(1, 0x10),
        Seg(2, 0x10),
        Seg(2, 0x08),
        Seg(2, 0x04),
        Seg(2, 0x02),
        Seg(9, 0x20),
        Seg(15, 0x20),
        Seg(18, 0x20),
    ];
    /// The bargraph, left to right.
    pub const BARS: [Seg; 15] = [
        Seg(1, 0x08),
        Seg(1, 0x04),
        Seg(1, 0x02),
        Seg(31, 0x80),
        Seg(31, 0x40),
        Seg(31, 0x20),
        Seg(31, 0x10),
        Seg(30, 0x10),
        Seg(30, 0x20),
        Seg(30, 0x40),
        Seg(30, 0x80),
        Seg(29, 0x80),
        Seg(29, 0x40),
        Seg(29, 0x20),
        Seg(29, 0x10),
    ];
}

/// Seven-segment patterns for 0–9 on the main display: (A B C in bits 0–2
/// of the high byte, F G E D in bits 0–3 of the low byte).
const MAIN_DIGIT: [(u8, u8); 10] = [
    (0x07, 0x0D),
    (0x06, 0x00),
    (0x03, 0x0E),
    (0x07, 0x0A),
    (0x06, 0x03),
    (0x05, 0x0B),
    (0x05, 0x0F),
    (0x07, 0x00),
    (0x07, 0x0F),
    (0x07, 0x0B),
];

/// The same on the small display, whose bits run the other way: A B C D in
/// bits 4–7 of the high byte, E F G in bits 5–7 of the low byte.
const SMALL_DIGIT: [(u8, u8); 10] = [
    (0xF0, 0xA0),
    (0x60, 0x00),
    (0xD0, 0x60),
    (0xF0, 0x40),
    (0x60, 0xC0),
    (0xB0, 0xC0),
    (0xB0, 0xE0),
    (0xE0, 0x00),
    (0xF0, 0xE0),
    (0xF0, 0xC0),
];

/// The mode legends the LCD has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeIcon {
    Lsb,
    Usb,
    Cw,
    Am,
    Sam,
    Fm,
    Dsb,
    Drm,
    Dig,
}

/// Everything the panel shows. Plain values, so a caller can compare two and
/// only send when something changed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Display {
    /// The frequency on the 9-digit display, in Hz. Above 999.999999 MHz it
    /// is shown in kHz instead, with the dots moved to match.
    pub freq_hz: Option<u64>,
    pub mode: Option<ModeIcon>,
    /// Which VFO is in use: `false` lights A, `true` lights B.
    pub vfo_b: bool,
    pub tx: bool,
    pub nb: bool,
    pub nr: bool,
    pub an: bool,
    pub rit: bool,
    pub xit: bool,
    /// Received signal in dBm: the bargraph and, on the small display, the
    /// figure itself. Ignored while transmitting.
    pub signal_dbm: Option<i32>,
    /// Forward power while transmitting, on the small display.
    pub tx_watts: Option<u32>,
    pub lock_led: bool,
}

fn set(r: &mut [u8; REPORT_LEN], s: Seg, on: bool) {
    if on {
        r[s.0] |= s.1;
    } else {
        r[s.0] &= !s.1;
    }
}

/// Write `value` right-aligned on the 9-digit display, blanking leading zeros.
fn main_digits(r: &mut [u8; REPORT_LEN], mut value: u64) {
    value = value.min(999_999_999);
    for d in 1..=9usize {
        let (hi, lo) = (22 - 2 * d, 21 - 2 * d);
        r[hi] &= 0xF8;
        r[lo] &= 0xF0;
        if value == 0 && d > 1 {
            continue;
        }
        let (h, l) = MAIN_DIGIT[(value % 10) as usize];
        r[hi] |= h;
        r[lo] |= l;
        value /= 10;
    }
}

/// Write `value` right-aligned on the 3-digit display, blanking leading zeros.
fn small_digits(r: &mut [u8; REPORT_LEN], mut value: u32) {
    value = value.min(999);
    for d in 1..=3usize {
        let (hi, lo) = (21 + 2 * d, 22 + 2 * d);
        r[hi] &= 0x0F;
        r[lo] &= 0x1F;
        if value == 0 && d > 1 {
            continue;
        }
        let (h, l) = SMALL_DIGIT[(value % 10) as usize];
        r[hi] |= h;
        r[lo] |= l;
        value /= 10;
    }
}

/// Bars lit for a signal: S1–S9 at 6 dB each (S9 = −73 dBm, the HF
/// convention), then one bar per 10 dB over S9 to +60.
pub fn smeter_bars(dbm: i32) -> usize {
    if dbm <= -73 {
        let s = (dbm + 127).div_euclid(6); // S1 at −121 dBm
        s.clamp(0, 9) as usize
    } else {
        let over = dbm + 73;
        (9 + (over + 9) / 10).min(15) as usize
    }
}

/// The settings part of every report: backlight, contrast, the dial
/// acceleration ELAD ships (1, 5 and 10 counts per detent) and the LEDs.
fn settings(r: &mut [u8; REPORT_LEN], lock: bool) {
    r[STATUS] = LED_USB | if lock { LED_LOCK } else { 0 };
    r[BACKLIGHT_R] = 0x00;
    r[BACKLIGHT_G] = 0x32;
    r[BACKLIGHT_B] = 0xFF;
    r[CONTRAST] = 0x28;
    r[REFRESH] = 0x28;
    r[SPEED..SPEED + 3].copy_from_slice(&[1, 5, 10]);
    r[THRESHOLD..THRESHOLD + 2].copy_from_slice(&[15, 25]);
    r[EVAL_TIME] = 10;
}

/// The full output report for `d`.
pub fn output(d: &Display) -> [u8; REPORT_LEN] {
    let mut r = [0u8; REPORT_LEN];
    settings(&mut r, d.lock_led);

    if let Some(hz) = d.freq_hz {
        if hz <= 999_999_999 {
            main_digits(&mut r, hz);
            set(&mut r, seg::DOT_MHZ, hz >= 1_000_000);
            set(&mut r, seg::DOT_KHZ, hz >= 1_000);
            set(&mut r, seg::HZ, true);
        } else {
            // GHz: shown in kHz, so the dots fall at GHz and MHz.
            let khz = hz / 1_000;
            main_digits(&mut r, khz);
            set(&mut r, seg::DOT_MHZ, true);
            set(&mut r, seg::DOT_KHZ, true);
        }
    }

    if let Some(m) = d.mode {
        let s = match m {
            ModeIcon::Lsb => seg::LSB,
            ModeIcon::Usb => seg::USB,
            ModeIcon::Cw => seg::CW,
            ModeIcon::Am => seg::AM,
            ModeIcon::Sam => seg::SAM,
            ModeIcon::Fm => seg::FM,
            ModeIcon::Dsb => seg::DSB,
            ModeIcon::Drm => seg::DRM,
            ModeIcon::Dig => seg::DIG,
        };
        set(&mut r, s, true);
    }

    set(&mut r, if d.vfo_b { seg::B } else { seg::A }, d.freq_hz.is_some());
    set(&mut r, seg::TX, d.tx);
    set(&mut r, seg::RX, !d.tx && d.freq_hz.is_some());
    set(&mut r, seg::NB, d.nb);
    set(&mut r, seg::NR, d.nr);
    set(&mut r, seg::AN, d.an);
    set(&mut r, seg::RIT, d.rit);
    set(&mut r, seg::XIT, d.xit);

    if d.tx {
        if let Some(w) = d.tx_watts {
            small_digits(&mut r, w);
            set(&mut r, seg::W, true);
        }
    } else if let Some(dbm) = d.signal_dbm {
        set(&mut r, seg::SMETER_LINE, true);
        set(&mut r, seg::S, true);
        for s in seg::SCALE {
            set(&mut r, s, true);
        }
        for s in seg::BARS.iter().take(smeter_bars(dbm)) {
            set(&mut r, *s, true);
        }
        small_digits(&mut r, dbm.unsigned_abs());
        set(&mut r, seg::DB_MINUS, dbm < 0);
        set(&mut r, seg::DBM, true);
    }
    r
}

/// The report sent on connect, before there is anything to show: a blank
/// LCD with the backlight, the USB LED and the dial settings.
pub fn idle_output() -> [u8; REPORT_LEN] {
    output(&Display::default())
}

/// The report sent on the way out: blank, with the USB LED off, so the panel
/// shows nothing is driving it any more.
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

    /// OpenTMate2Lib's capture of ELAD's DLL writing "0" on both displays:
    /// bytes 19, 20 and 23, 24.
    #[test]
    fn a_zero_lands_where_the_capture_put_it() {
        let mut r = [0u8; REPORT_LEN];
        main_digits(&mut r, 0);
        small_digits(&mut r, 0);
        assert_eq!(&r[19..25], &[0x0D, 0x07, 0x00, 0x00, 0xF0, 0xA0]);
        assert!(r[..19].iter().all(|b| *b == 0));
        assert!(r[25..32].iter().all(|b| *b == 0));
    }

    #[test]
    fn a_frequency_fills_the_main_display_and_keeps_its_neighbours() {
        let d = Display { freq_hz: Some(14_074_000), ..Default::default() };
        let r = output(&d);
        // 14 074 000: digit 8 is "1", digit 7 is "4", digit 9 is blank.
        assert_eq!((r[22 - 16] & 0x07, r[21 - 16] & 0x0F), MAIN_DIGIT[1]);
        assert_eq!((r[22 - 14] & 0x07, r[21 - 14] & 0x0F), MAIN_DIGIT[4]);
        assert_eq!((r[4] & 0x07, r[3] & 0x0F), (0, 0));
        assert_ne!(r[9] & 0x10, 0, "MHz dot");
        assert_ne!(r[15] & 0x10, 0, "kHz dot");
        assert_ne!(r[23] & 0x01, 0, "Hz");
    }

    #[test]
    fn smeter_bars_follow_s_units() {
        assert_eq!(smeter_bars(-140), 0);
        assert_eq!(smeter_bars(-121), 1);
        assert_eq!(smeter_bars(-115), 2);
        assert_eq!(smeter_bars(-73), 9);
        assert_eq!(smeter_bars(-63), 10);
        assert_eq!(smeter_bars(-53), 11);
        assert_eq!(smeter_bars(-13), 15);
        assert_eq!(smeter_bars(20), 15);
    }

    #[test]
    fn receive_shows_the_signal_and_transmit_the_power() {
        let rx = Display {
            freq_hz: Some(7_100_000),
            mode: Some(ModeIcon::Lsb),
            signal_dbm: Some(-73),
            ..Default::default()
        };
        let r = output(&rx);
        assert_ne!(r[22] & 0x40, 0, "LSB");
        assert_ne!(r[0] & 0x04, 0, "RX");
        assert_ne!(r[28] & 0x10, 0, "minus");
        assert_eq!(seg::BARS.iter().filter(|s| r[s.0] & s.1 != 0).count(), 9);
        let (h, l) = SMALL_DIGIT[3];
        assert_eq!((r[23] & 0xF0, r[24] & 0xE0), (h, l), "units digit 3 of 73");

        let tx = Display { tx: true, tx_watts: Some(100), ..rx };
        let r = output(&tx);
        assert_ne!(r[0] & 0x08, 0, "TX");
        assert_eq!(r[0] & 0x04, 0, "not RX");
        assert_eq!(seg::BARS.iter().filter(|s| r[s.0] & s.1 != 0).count(), 0);
        assert_ne!(r[20] & 0x20, 0, "W");
    }
}
