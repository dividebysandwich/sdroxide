//! ELAD CAT (ASCII, `;`-terminated) — the FDM-DUO and FDM-DUOr. Frequency
//! `FA<11 digits>;`, mode `MD<x>;`, PTT `TX;`/`RX;`.
//!
//! ELAD's own description is "proprietary commands and also a subset of the
//! Kenwood TS-480 command set", and the subset is smaller than it sounds:
//! everything past the dial and the mode is ELAD's own. Pointing the Kenwood
//! profile at an FDM-DUO gets the frequency right and then misses every meter,
//! the filter, the power and the split. The differences that matter:
//!
//! * **The S-meter is not a Kenwood S-meter.** `SM` answers on ELAD's own
//!   scale, where the counts are not evenly spaced — S5 is 6 and S6 is 8, with
//!   nothing at 7 — so no linear reading of it is right anywhere except by
//!   accident. The published points are in [`SMETER_CAL`].
//! * **SWR is `WR`, not `RM`.** It answers a real ratio in text (`01.50`)
//!   rather than a count of lit bars, and it carries its own "this reading is
//!   not to be trusted" flag, which is set in receive, below 500 mW, and after
//!   a high-SWR trip.
//! * **Power is nine fixed steps, not watts.** `TP` selects 0.3, 0.5, 1.0, 1.2,
//!   1.5, 2.0, 3.0, 4.0 or 5.0 W — a Kenwood-shaped `PC050;` is a syntax error,
//!   and even read as an index it would be five times the rig's whole output.
//! * **The receive filter is `RF`, indexed per mode.** Three different tables
//!   behind one command, and the mode digit selects which — see [`filter_index`].
//! * **Split is `SP`, and `FR0;` does not cancel it.** `SP0;` is the only thing
//!   that does.
//! * **The antenna is a *count*, not a port index.** `AN1;` is one antenna, on
//!   the RTX socket; `AN2;` is two — receive on the RX-only socket, transmit
//!   still out of RTX. It is read once at open (`AN;`) and otherwise left
//!   alone, because it is the operator's own front-panel setting (menu 31
//!   `ANTENNAS`) and it survives a power cycle.
//!
//! There is no DATA mode and no `DA` flag: `MD` has six positions and none of
//! them is a data position. Digital modes therefore go out as plain USB or LSB,
//! and which input the rig transmits from is the separate `TI` setting rather
//! than anything the mode says — which is why [`CatConfig::elad_tx_input`]
//! exists and is asserted when the port opens.
//!
//! **No text keyer.** `SW` triggers one of the ten CW messages stored *in the
//! radio*; there is no command that hands the rig arbitrary text, so
//! [`Protocol::cw_chunk_len`] stays 0 and CW is keyed by the operator's own key
//! or paddle (or by DTR, with menu 37 `CW IN` set to `Key+DTR`).
//!
//! Written from the *ELAD FDM-DUO User Manual* v2.6 (12/2017), §6 "CAT Remote
//! Control". **Not yet verified against a rig** — no ELAD hardware was
//! available. Everything below is transcribed from the command tables.

use crate::{CatUpdate, Protocol};
use sdroxide_types::{EladAntenna, EladTxInput, Mode};
use tracing::{debug, info};

/// Digits in the `FA`/`FB` frequency field — "Frequency in Hz (11 digit)",
/// the same width as Kenwood and Elecraft and unlike Yaesu, which has two.
const FREQ_DIGITS: usize = 11;

/// What the transmitter puts out at each of `TP`'s nine settable steps, in
/// watts. Index into this array *is* the command's parameter.
///
/// The tenth setting, `TP09`, is "MAX": more than 5 W by an amount ELAD does
/// not publish and which is not the same on every band. It is accepted from the
/// radio — an operator who set MAX at the front panel should see a full slider
/// rather than a needle that fell off the bottom — but never commanded, because
/// there is no fraction of anything it can honestly be the answer to.
const TX_POWER_STEPS_W: [f32; 9] = [0.3, 0.5, 1.0, 1.2, 1.5, 2.0, 3.0, 4.0, 5.0];

/// The `TP` parameter for "MAX", which is read but never written.
const TX_POWER_MAX_CODE: u32 = 9;

/// Watts at the top of the Drive slider. The highest step that means a number.
const FULL_POWER_W: f32 = 5.0;

/// `SM` reading → dBm, from the manual's own table.
///
/// S9 is −73 dBm and an S-unit is 6 dB, which fixes the dB column; the reading
/// column is copied as published. It is worth reading twice, because it is the
/// reason this cannot be a multiply: the counts step by one from S0 to S5, skip
/// 7 entirely between S5 and S6, step by one again to S9, and then step by two
/// for each 10 dB above it. Scaling the reading linearly — the shape every
/// other family in this crate uses — would be six decibels out through the
/// middle of the scale and thirty at the top.
const SMETER_CAL: &[(f32, f32)] = &[
    (0.0, -127.0), // S0
    (2.0, -121.0), // S1
    (3.0, -115.0), // S2
    (4.0, -109.0), // S3
    (5.0, -103.0), // S4
    (6.0, -97.0),  // S5
    (8.0, -91.0),  // S6
    (9.0, -85.0),  // S7
    (10.0, -79.0), // S8
    (11.0, -73.0), // S9
    (12.0, -63.0), // S9+10
    (14.0, -53.0), // S9+20
    (16.0, -43.0), // S9+30
    (18.0, -33.0), // S9+40
    (20.0, -23.0), // S9+50
    (22.0, -13.0), // S9+60
];

/// `RF` filter widths for SSB, in Hz, indexed by the command's parameter.
///
/// Indices 19 to 21 are the manual's `DATA 300`, `DATA 600` and `DATA 1000`
/// entries and are deliberately left off the end. They are a separate filter
/// path whose behaviour on the air this side cannot check, and including them
/// would let an ordinary narrow-SSB request land in one of them.
const RF_SSB_HZ: &[u32] = &[
    1600, 1700, 1800, 1900, 2000, 2100, 2200, 2300, 2400, 2500, 2600, 2700, 2800, 2900, 3000, 3100,
    4000, 5000, 6000,
];

/// `RF` filter widths for CW, in Hz, and the parameter each one takes.
///
/// Not a bare array like the others: the CW column starts at index 07, and its
/// first four entries are all "100 Hz" with the rig's DR (dynamic range) level
/// varying behind them. Only the plain 100 Hz at index 11 is a filter width, so
/// the four DR positions are skipped rather than offered as duplicates.
const RF_CW: &[(u32, u32)] = &[(100, 11), (300, 12), (500, 13), (1000, 14), (1500, 15), (2600, 16)];

/// `RF` filter widths for AM, in Hz, indexed by the command's parameter.
const RF_AM_HZ: &[u32] = &[2500, 3000, 3500, 4000, 4500, 5000, 5500, 6000];

/// The `FA` frame that puts the dial on `hz`.
///
/// Public — with [`mode_frame`] and [`ptt_frame`] — for the same reason
/// [`crate::civ`] is: the native ELAD backend can tunnel these same commands
/// through the FDM-DUO's *USB* interface, which is what lets a DUO with only
/// its receive cable plugged in still be tuned and keyed. Two copies of the
/// framing would be one copy too many.
pub fn freq_frame(hz: f64) -> String {
    let hz = hz.round().clamp(0.0, 99_999_999_999.0) as u64;
    format!("FA{hz:0FREQ_DIGITS$};")
}

/// The `MD` frame for an app mode.
pub fn mode_frame(m: Mode) -> String {
    format!("MD{};", mode_digit(m))
}

/// The frame that keys or unkeys the transmitter.
pub fn ptt_frame(on: bool) -> String {
    // `TX0;` and `TX1;` are both "normal transmission" here and `TX2;` is
    // TUNE — a keyed carrier, not an over. The bare `TX;` the manual's own
    // examples use is what goes out; `RX;` unkeys.
    if on { "TX;".to_string() } else { "RX;".to_string() }
}

/// The `AN` frame that puts the receiver on `ant`.
///
/// Public for the same reason the three above are: an FDM-DUO on nothing but
/// its receive cable is driven through the USB gateway, and the antenna is one
/// of the things that still works there.
pub fn antenna_frame(ant: EladAntenna) -> String {
    format!("AN{};", ant.digit())
}

/// The frame that asks which socket the rig is receiving on.
///
/// Sent once when the port opens and never polled. The setting lives in the
/// radio and survives a power cycle, so what it is at the moment sdroxide
/// arrives is a question with an answer worth having — but it is also a
/// front-panel menu item nobody changes mid-over, and this family's answer
/// costs the same bus time as a dial poll.
pub fn read_antenna_frame() -> String {
    "AN;".to_string()
}

pub struct Elad {
    buf: String,
    /// Which input the rig should transmit from, asserted once when the port
    /// opens. See [`Protocol::clear_offsets`].
    tx_input: EladTxInput,
    /// Mode digit from the rig's last `MD;` reply.
    mode_digit: Option<char>,
    /// True once the radio has named itself, so a later `DT`/`VS` reply cannot
    /// re-announce the same rig on every poll.
    identified: bool,
}

impl Elad {
    pub fn new(tx_input: EladTxInput) -> Self {
        Elad { buf: String::new(), tx_input, mode_digit: None, identified: false }
    }

    /// The rig's mode digit as the app's mode.
    ///
    /// Deliberately not the inverse of [`mode_digit`] over its whole range: a
    /// rig position that would be commanded back as something *else* yields
    /// `None`, so the app and the radio cannot spend the session correcting each
    /// other. Here that only leaves the digits the rig has no meaning for —
    /// `MD` has no 0, 6 or 8 — because every position it does have maps onto a
    /// mode that maps back to it.
    fn effective_mode(&self) -> Option<Mode> {
        Some(match self.mode_digit? {
            '1' => Mode::Lsb,
            '2' => Mode::Usb,
            // CW and CW-REV are both CW to the app.
            '3' | '7' => Mode::Cw,
            '4' => Mode::Nfm,
            '5' => Mode::Am,
            _ => return None,
        })
    }
}

/// The rig's mode digit for an app mode. ELAD `MD`: 1=LSB 2=USB 3=CW 4=FM 5=AM
/// 7=CWR. There is no 6 and no 8 — this family has no FSK position and no data
/// position, so every keyboard mode rides on a plain sideband.
fn mode_digit(m: Mode) -> char {
    match m {
        Mode::Lsb | Mode::Digl => '1',
        Mode::Cw => '3',
        // RIFP keys the carrier itself and VHF packet frequency-modulates it,
        // so the dial only means what they mean by it with the rig in FM.
        // No rig has an ADS-B mode and none ever will: the dial is at
        // 1090 MHz. Grouped with FM so nothing downstream has to special-case
        // a mode a radio can neither be put into nor report back.
        Mode::Nfm
        | Mode::Wfm
        | Mode::Rifp
        | Mode::Packet
        | Mode::Aprs
        | Mode::SstvFm
        | Mode::RttyFm
        | Mode::Adsb
        | Mode::Vdl2
        | Mode::Ais => '4',
        Mode::Am | Mode::Sam | Mode::Dsb | Mode::Isb | Mode::Drm => '5',
        // Everything else is upper sideband: the digital and keyboard modes,
        // and the receive-only modes that have no transmit side at all.
        Mode::Usb
        | Mode::Digu
        | Mode::Ft8
        | Mode::Js8
        | Mode::Wspr
        | Mode::Ft4
        | Mode::Ft2
        | Mode::Psk
        | Mode::Rtty
        | Mode::Olivia
        | Mode::Thor
        | Mode::Fsq
        | Mode::Hell
        | Mode::PacketHf
        | Mode::AtChat
        | Mode::Rade
        | Mode::Spec
        | Mode::Sstv
        | Mode::Wefax
        | Mode::Navtex
        | Mode::RfPaint => '2',
    }
}

/// The `TI` parameter for a configured transmit input, or `None` to leave the
/// rig's own setting alone.
fn tx_input_digit(input: EladTxInput) -> Option<char> {
    match input {
        EladTxInput::Mic => Some('0'),
        EladTxInput::UsbAudio => Some('1'),
        EladTxInput::Auto => Some('2'),
        EladTxInput::Leave => None,
    }
}

/// Interpolate `reading` through [`SMETER_CAL`] to dBm.
///
/// This is the rig's own meter, in the units ELAD calibrated it in — not a level
/// derived from the sound card — so it needs no dBFS→dBm offset on top.
fn dbm_from_smeter(reading: u32) -> f32 {
    let r = reading as f32;
    let first = SMETER_CAL[0];
    let last = SMETER_CAL[SMETER_CAL.len() - 1];
    if r <= first.0 {
        return first.1;
    }
    if r >= last.0 {
        return last.1;
    }
    for w in SMETER_CAL.windows(2) {
        let ((x0, y0), (x1, y1)) = (w[0], w[1]);
        if r <= x1 {
            return y0 + (y1 - y0) * (r - x0) / (x1 - x0);
        }
    }
    last.1
}

/// The `RF` parameter for a passband `width_hz` wide in `mode`, or `None` where
/// this family cannot express it.
///
/// Errs wide throughout: the first entry that *reaches* the requested width, and
/// the widest entry when nothing does. A filter quietly narrower than the one on
/// screen presents as a signal that is simply not there, which is a far worse
/// failure than a little extra noise either side.
///
/// FM returns `None` on purpose. Its three positions are "Voice Narrow", "Voice
/// Wide" and "Data" — the manual publishes no bandwidth for any of them — so
/// there is no honest mapping from a width, and the rig's own selection is left
/// where the operator put it.
fn filter_index(mode: Mode, width_hz: f32) -> Option<u32> {
    let hz = width_hz.abs().round().max(0.0) as u32;
    let pick = |table: &[u32]| -> u32 {
        table.iter().position(|&v| v >= hz).unwrap_or(table.len() - 1) as u32
    };
    Some(match mode_digit(mode) {
        '1' | '2' => pick(RF_SSB_HZ),
        '3' | '7' => {
            let (_, code) = RF_CW.iter().find(|(w, _)| *w >= hz).unwrap_or(&RF_CW[RF_CW.len() - 1]);
            *code
        }
        '5' => pick(RF_AM_HZ),
        // FM, and anything else that ever appears here.
        _ => return None,
    })
}

impl Protocol for Elad {
    fn set_freq(&mut self, hz: f64) -> Vec<u8> {
        freq_frame(hz).into_bytes()
    }

    fn set_mode(&mut self, m: Mode) -> Vec<u8> {
        mode_frame(m).into_bytes()
    }

    fn ptt(&self, on: bool) -> Vec<u8> {
        ptt_frame(on).into_bytes()
    }

    fn poll_requests(&self) -> Vec<Vec<u8>> {
        vec![b"FA;".to_vec(), b"MD;".to_vec()]
    }
    fn dial_requests(&self) -> Vec<Vec<u8>> {
        vec![b"FA;".to_vec()]
    }

    fn tx_telemetry_requests(&self) -> Vec<Vec<u8>> {
        // `WR` only. `FP` reads forward power, but the manual gives its six
        // digits no unit at all, and a power meter calibrated in guesses is
        // worse than none — so it is not asked for and not reported.
        vec![b"WR;".to_vec()]
    }

    fn rx_telemetry_requests(&self) -> Vec<Vec<u8>> {
        // P1 is documented as "Always 0", so the read carries it.
        vec![b"SM0;".to_vec()]
    }

    fn clear_offsets(&self) -> Vec<Vec<u8>> {
        let mut frames = vec![
            // RIT off, then its offset zeroed — sdroxide carries RIT on the
            // dial, so anything the rig is still holding would add to ours
            // unseen. `RC` is a SET-only command and takes no parameter.
            b"RT0;".to_vec(),
            b"RC;".to_vec(),
            // Split off. `SP0;` is the only thing that cancels it on this
            // family: `FR0;` selects VFO-A and leaves the split state alone,
            // and Yaesu's `FT2;` is not a command here at all.
            b"SP0;".to_vec(),
            b"FR0;".to_vec(),
            // Name the radio in the log — which model, and which firmware.
            b"DT;".to_vec(),
            b"VS;".to_vec(),
        ];
        // Where transmit audio comes from. Asserted rather than assumed because
        // the rig remembers it across power cycles: a DUO left on `TI0` sends
        // the microphone no matter what sdroxide puts into its sound card, and
        // an operator who wants the microphone would find it silently taken
        // away. Both are visible mistakes only if this is a setting, which is
        // why it is one.
        if let Some(d) = tx_input_digit(self.tx_input) {
            frames.push(format!("TI{d};").into_bytes());
        }
        frames
    }

    // Deliberately no `cw_chunk_len`/`send_cw`/`set_cw_wpm`. `SW` plays one of
    // the ten CW messages stored in the radio and there is no command that hands
    // it text, so there is nothing for the CW panel to stream to. Keying is the
    // operator's key or paddle, or DTR with menu 37 `CW IN` set to `Key+DTR`.
    // The keyer's speed lives in menu 45 and CAT cannot reach it either.

    fn set_filter(&mut self, mode: Mode, lo_hz: f32, hi_hz: f32) -> Vec<Vec<u8>> {
        let Some(idx) = filter_index(mode, hi_hz - lo_hz) else {
            return Vec::new();
        };
        // P1 is the same digit `MD` uses, and it selects which of the three
        // tables P2 indexes — the identical parameter means three different
        // bandwidths depending on it.
        vec![format!("RF{}{idx:02};", mode_digit(mode)).into_bytes()]
    }

    fn commands_filter(&self) -> bool {
        true
    }

    fn set_power(&mut self, frac: f32) -> Vec<Vec<u8>> {
        let want = frac.clamp(0.0, 1.0) * FULL_POWER_W;
        // Nearest step rather than the next one down: the steps are coarse and
        // unevenly spaced at the bottom (0.3, 0.5, 1.0), so rounding down would
        // put most of the lower half of the slider on 0.3 W.
        let idx = TX_POWER_STEPS_W
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                (**a - want)
                    .abs()
                    .partial_cmp(&(**b - want).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
            .unwrap_or(0);
        vec![format!("TP{idx:02};").into_bytes()]
    }

    fn read_power(&self) -> Vec<Vec<u8>> {
        vec![b"TP;".to_vec()]
    }

    fn commands_power(&self) -> bool {
        true
    }

    fn antennas(&self) -> &'static [&'static str] {
        &EladAntenna::LABELS
    }

    fn set_antenna(&mut self, name: &str) -> Vec<Vec<u8>> {
        // A name this family does not have is dropped rather than guessed at.
        // It arrives when a session file remembers the port of whatever radio
        // was on this interface last, and a guess would move an antenna relay
        // on the strength of a name from another rig.
        match EladAntenna::from_label(name) {
            Some(a) => vec![antenna_frame(a).into_bytes()],
            None => Vec::new(),
        }
    }

    fn read_antenna(&self) -> Vec<Vec<u8>> {
        vec![read_antenna_frame().into_bytes()]
    }

    fn parse(&mut self, buf: &mut Vec<u8>) -> Vec<CatUpdate> {
        self.buf.push_str(&String::from_utf8_lossy(buf));
        buf.clear();
        let mut out = Vec::new();
        while let Some(idx) = self.buf.find(';') {
            let msg: String = self.buf.drain(..=idx).collect();
            let msg = msg.trim_end_matches(';');
            if let Some(rest) = msg.strip_prefix("FA") {
                if rest.len() == FREQ_DIGITS
                    && let Ok(hz) = rest.parse::<u64>()
                {
                    out.push(CatUpdate::Freq(hz as f64));
                }
            } else if let Some(rest) = msg.strip_prefix("MD") {
                if let Some(d) = rest.chars().next() {
                    self.mode_digit = Some(d);
                    if let Some(m) = self.effective_mode() {
                        out.push(CatUpdate::Mode(m));
                    }
                }
            } else if let Some(rest) = msg.strip_prefix("SM") {
                // `SM0nnnn` — the P1 the read carried, then four digits.
                if let Some(reading) = rest.strip_prefix('0')
                    && reading.len() == 4
                    && let Ok(r) = reading.parse::<u32>()
                {
                    out.push(CatUpdate::Signal(dbm_from_smeter(r)));
                }
            } else if let Some(rest) = msg.strip_prefix("WR") {
                if let Some(swr) = parse_swr(rest) {
                    out.push(CatUpdate::Swr(swr));
                }
            } else if let Some(rest) = msg.strip_prefix("TP") {
                if let Ok(code) = rest.trim().parse::<u32>() {
                    let w = if code == TX_POWER_MAX_CODE {
                        FULL_POWER_W
                    } else {
                        *TX_POWER_STEPS_W.get(code as usize).unwrap_or(&0.0)
                    };
                    out.push(CatUpdate::Power((w / FULL_POWER_W).clamp(0.0, 1.0)));
                }
            } else if let Some(rest) = msg.strip_prefix("AN") {
                // "How many antennas" as the rig puts it, which is the same
                // question as "which socket is the receiver on".
                if let Some(a) = rest.chars().next().and_then(EladAntenna::from_digit) {
                    out.push(CatUpdate::Antenna(a.label()));
                }
            } else if let Some(rest) = msg.strip_prefix("RI") {
                // Not reported, only logged. The manual gives P2 as the "RSSI
                // absolute value" in four digits and never says in what — dBm,
                // tenths of a dBm and dBµV all fit — so it is kept where a
                // future calibration against a signal generator can find it,
                // and `SM` (which has a published table) drives the meter.
                debug!(reading = rest, "ELAD CAT: RSSI reported (units unverified)");
            } else if let Some(rest) = msg.strip_prefix("DT") {
                if !self.identified {
                    self.identified = true;
                    info!(duo_type = rest, "ELAD CAT: rig identified");
                }
            } else if let Some(rest) = msg.strip_prefix("VS") {
                debug!(firmware = rest, "ELAD CAT: firmware version");
            }
        }
        out
    }
}

/// The SWR out of a `WR` answer body, or `None` when the rig flagged the
/// reading as one not to be trusted.
///
/// Body layout, after the two command letters: P1 the high-SWR trip flag, P2 a
/// space or `!`, then the ratio as `nn.nn`. The `!` is set in receive, at 0 dBm
/// out, and below the 500 mW the rig needs to compute a ratio at all — every one
/// of which is a moment when there is no SWR to show, rather than one where it
/// happens to be 1:1. Reporting a plausible number there would put a flat needle
/// under a transmitter that never came up.
fn parse_swr(body: &str) -> Option<f32> {
    let mut chars = body.chars();
    let _trip = chars.next()?;
    if chars.next()? == '!' {
        return None;
    }
    let ratio: String = chars.collect();
    let v: f32 = ratio.trim().parse().ok()?;
    // Below 1:1 is not a match, it is a misread frame.
    (v >= 1.0).then_some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn elad() -> Elad {
        Elad::new(EladTxInput::UsbAudio)
    }

    fn parse_str(e: &mut Elad, s: &str) -> Vec<CatUpdate> {
        let mut buf = s.as_bytes().to_vec();
        e.parse(&mut buf)
    }

    fn frames(v: Vec<Vec<u8>>) -> Vec<String> {
        v.iter().map(|f| String::from_utf8_lossy(f).into_owned()).collect()
    }

    #[test]
    fn frequency_is_eleven_digits() {
        let mut e = elad();
        assert_eq!(e.set_freq(14_074_000.0), b"FA00014074000;".to_vec());
        assert_eq!(e.set_freq(7_055_000.0), b"FA00007055000;".to_vec());
        assert_eq!(parse_str(&mut e, "FA00014060000;"), vec![CatUpdate::Freq(14_060_000.0)]);
        // Yaesu's nine and Icom's decimal form are not frequencies here.
        assert!(parse_str(&mut e, "FA014074000;").is_empty());
        assert!(parse_str(&mut e, "FAxxxxxxxxxxx;").is_empty());
    }

    /// `AN` is published as a *count* of antennas, not a port index, and the
    /// count is what goes on the wire: one antenna is the shared RTX socket,
    /// two is the receive-only one. Getting this backwards would move the
    /// receiver to a socket with nothing on it and look exactly like a dead
    /// band.
    #[test]
    fn the_antenna_command_carries_a_count_of_antennas() {
        let mut e = elad();
        assert_eq!(frames(e.set_antenna("RTX")), vec!["AN1;"]);
        assert_eq!(frames(e.set_antenna("RX only")), vec!["AN2;"]);
        assert_eq!(frames(e.read_antenna()), vec!["AN;"]);
        // Both sockets are offered, and by the names the rest of sdroxide uses.
        assert_eq!(e.antennas(), &["RTX", "RX only"]);
    }

    /// A name from another radio must not move an antenna relay. `session.json`
    /// remembers the port of whatever front end was last on this radio, and a
    /// LimeSDR's LNAH is what that looks like on the way in.
    #[test]
    fn a_port_this_family_does_not_have_is_not_guessed_at() {
        let mut e = elad();
        assert!(e.set_antenna("LNAH").is_empty());
        assert!(e.set_antenna("").is_empty());
        assert!(e.set_antenna("ANT 2").is_empty());
        // The rig's own names still work whatever case they come back in — an
        // operator's hand-edited session file is a file like any other.
        assert_eq!(frames(e.set_antenna("rx only")), vec!["AN2;"]);
    }

    /// What the rig answers has to be the name it was asked for, or the panel
    /// and the radio spend the session disagreeing about where the aerial is.
    #[test]
    fn every_socket_survives_a_round_trip() {
        for a in EladAntenna::ALL {
            let mut e = elad();
            let sent = String::from_utf8(e.set_antenna(a.label()).concat()).unwrap();
            let reply = sent.clone(); // the ANSWER form and the SET form are the same
            assert_eq!(
                parse_str(&mut e, &reply),
                vec![CatUpdate::Antenna(a.label())],
                "{} was set with {sent} and read back as something else",
                a.label()
            );
        }
        // A count the rig has no meaning for is not a socket.
        let mut e = elad();
        assert!(parse_str(&mut e, "AN0;").is_empty());
        assert!(parse_str(&mut e, "AN3;").is_empty());
        assert!(parse_str(&mut e, "AN;").is_empty());
    }

    #[test]
    fn keying_takes_no_parameter_at_either_end() {
        let e = elad();
        assert_eq!(e.ptt(true), b"TX;".to_vec());
        assert_eq!(e.ptt(false), b"RX;".to_vec());
        // `TX2;` is TUNE — a keyed carrier — and must never be how an over
        // starts.
        assert_ne!(e.ptt(true), b"TX2;".to_vec());
    }

    #[test]
    fn every_app_mode_the_rig_can_hold_survives_a_round_trip() {
        // What we command has to be what we read back, or the app and the rig
        // spend the session correcting each other.
        for m in [Mode::Lsb, Mode::Usb, Mode::Cw, Mode::Nfm, Mode::Am] {
            let mut e = elad();
            let sent = String::from_utf8(e.set_mode(m)).unwrap();
            let reply = format!("MD{};", mode_digit(m));
            assert_eq!(
                parse_str(&mut e, &reply),
                vec![CatUpdate::Mode(m)],
                "{m:?} was set with {sent} and read back as something else"
            );
        }
    }

    #[test]
    fn digital_modes_ride_on_a_plain_sideband() {
        let mut e = elad();
        // No DATA position on this family, and emphatically no Kenwood `DA`
        // flag — a DUO would answer that with nothing at all.
        for m in [Mode::Ft8, Mode::Digu, Mode::Rtty, Mode::Psk] {
            let sent = String::from_utf8(e.set_mode(m)).unwrap();
            assert_eq!(sent, "MD2;", "{m:?}");
            assert!(!sent.contains("DA"));
        }
        assert_eq!(String::from_utf8(e.set_mode(Mode::Digl)).unwrap(), "MD1;");
    }

    #[test]
    fn rig_positions_without_a_stable_round_trip_are_left_alone() {
        let mut e = elad();
        // `MD` has no 0, 6 or 8 on this family.
        assert!(parse_str(&mut e, "MD0;").is_empty());
        assert!(parse_str(&mut e, "MD6;").is_empty());
        assert!(parse_str(&mut e, "MD8;").is_empty());
        // CW-REV is CW to the app.
        assert_eq!(parse_str(&mut e, "MD7;"), vec![CatUpdate::Mode(Mode::Cw)]);
    }

    #[test]
    fn the_s_meter_reads_on_elads_own_uneven_scale() {
        let mut e = elad();
        let dbm = |u: &[CatUpdate]| match u {
            [CatUpdate::Signal(d)] => *d,
            other => panic!("expected one signal reading, got {other:?}"),
        };
        // Every published point in the manual's table.
        for (reading, expect) in [
            (0u32, -127.0f32),
            (2, -121.0),
            (6, -97.0),
            (8, -91.0),
            (11, -73.0), // S9
            (14, -53.0), // S9+20
            (22, -13.0), // S9+60
        ] {
            assert_eq!(
                dbm(&parse_str(&mut e, &format!("SM0{reading:04};"))),
                expect,
                "reading {reading}"
            );
        }
        // The gap at 7 — no such point is published, and it has to fall between
        // S5 and S6 rather than reading as either.
        let seven = dbm(&parse_str(&mut e, "SM00007;"));
        assert!(seven > -97.0 && seven < -91.0, "reading 7 gave {seven}");
        // Reading it as a bar count and scaling — what every other family in
        // this crate does — would put S6 six decibels out.
        assert_ne!(dbm(&parse_str(&mut e, "SM00008;")), -85.0);
        // Past the top of the table is the top of the table, not an
        // extrapolation.
        assert_eq!(dbm(&parse_str(&mut e, "SM00030;")), -13.0);
        assert_eq!(frames(e.rx_telemetry_requests()), vec!["SM0;"]);
    }

    #[test]
    fn swr_is_text_and_carries_its_own_unreliable_flag() {
        let mut e = elad();
        assert_eq!(parse_str(&mut e, "WR0 01.50;"), vec![CatUpdate::Swr(1.5)]);
        assert_eq!(parse_str(&mut e, "WR0 01.00;"), vec![CatUpdate::Swr(1.0)]);
        assert_eq!(parse_str(&mut e, "WR0 12.30;"), vec![CatUpdate::Swr(12.3)]);
        // The trip flag says the rig dropped back to receive on a high SWR; the
        // reading beside it is still a reading.
        assert_eq!(parse_str(&mut e, "WR1 09.90;"), vec![CatUpdate::Swr(9.9)]);
        // `!` is receive, 0 dBm out, or under 500 mW — no SWR to show at all.
        // A flat needle there would sit under a transmitter that never came up.
        assert!(parse_str(&mut e, "WR0!01.00;").is_empty());
        assert!(parse_str(&mut e, "WR0!00.00;").is_empty());
        // A ratio below 1:1 is a misread frame, not a match.
        assert!(parse_str(&mut e, "WR0 00.50;").is_empty());
        assert_eq!(frames(e.tx_telemetry_requests()), vec!["WR;"]);
        // `FP` has no published unit, so it is never asked for.
        assert!(!frames(e.tx_telemetry_requests()).iter().any(|f| f.starts_with("FP")));
    }

    #[test]
    fn power_is_nine_fixed_steps_not_watts() {
        let mut e = elad();
        // The slider's top and bottom are the top and bottom of the step list.
        assert_eq!(frames(e.set_power(1.0)), vec!["TP08;"]); // 5.0 W
        assert_eq!(frames(e.set_power(0.0)), vec!["TP00;"]); // 0.3 W
        // Nearest step, not the next one down — the steps are coarse and
        // uneven at the bottom.
        assert_eq!(frames(e.set_power(0.4)), vec!["TP05;"]); // 2.0 W
        assert_eq!(frames(e.set_power(0.6)), vec!["TP06;"]); // 3.0 W
        // Never `TP09;` — MAX is not a fraction of anything.
        for pct in 0..=100 {
            let f = frames(e.set_power(pct as f32 / 100.0));
            assert_ne!(f[0], "TP09;", "{pct}%");
        }
        // A Kenwood-shaped watts value is not what goes out.
        assert!(!frames(e.set_power(1.0))[0].starts_with("PC"));

        assert_eq!(frames(e.read_power()), vec!["TP;"]);
        assert_eq!(parse_str(&mut e, "TP08;"), vec![CatUpdate::Power(1.0)]);
        assert_eq!(parse_str(&mut e, "TP02;"), vec![CatUpdate::Power(0.2)]); // 1.0 W
        // MAX is accepted from the radio, so an operator who set it at the
        // front panel sees a full slider rather than a fallen needle.
        assert_eq!(parse_str(&mut e, "TP09;"), vec![CatUpdate::Power(1.0)]);
    }

    #[test]
    fn the_filter_index_is_read_out_of_the_table_the_mode_selects() {
        let mut e = elad();
        // SSB: the first entry that reaches the width, so the filter is never
        // quietly narrower than the one on screen.
        assert_eq!(frames(e.set_filter(Mode::Usb, 0.0, 2400.0)), vec!["RF208;"]); // 2400 exactly
        // Anything between two entries takes the wider of them.
        assert_eq!(frames(e.set_filter(Mode::Usb, 200.0, 2650.0)), vec!["RF209;"]); // 2450 -> 2500
        assert_eq!(frames(e.set_filter(Mode::Usb, 0.0, 1000.0)), vec!["RF200;"]); // 1600 floor
        // Wider than the table goes lands on the widest entry, not on nothing.
        assert_eq!(frames(e.set_filter(Mode::Usb, 0.0, 20_000.0)), vec!["RF218;"]); // 6000
        // LSB arrives with negative edges — sideband lives in their sign — and
        // is the same width for all that, on the same table under P1 = 1.
        assert_eq!(frames(e.set_filter(Mode::Lsb, -2600.0, -200.0)), vec!["RF108;"]); // 2400
        // CW indexes a different table under the same command, and its
        // parameters do not start at zero.
        assert_eq!(frames(e.set_filter(Mode::Cw, -250.0, 250.0)), vec!["RF313;"]); // 500
        assert_eq!(frames(e.set_filter(Mode::Cw, -50.0, 50.0)), vec!["RF311;"]); // 100
        assert_eq!(frames(e.set_filter(Mode::Cw, -2000.0, 2000.0)), vec!["RF316;"]); // 2600
        // AM's table again.
        assert_eq!(frames(e.set_filter(Mode::Am, -3000.0, 3000.0)), vec!["RF507;"]); // 6000
        // FM's three positions have no published bandwidths, so the rig's own
        // selection is left where the operator put it.
        assert!(frames(e.set_filter(Mode::Nfm, -5000.0, 5000.0)).is_empty());
        assert!(e.commands_filter());
    }

    #[test]
    fn the_open_sequence_clears_what_the_dial_already_carries() {
        let f = frames(elad().clear_offsets());
        assert_eq!(f, vec!["RT0;", "RC;", "SP0;", "FR0;", "DT;", "VS;", "TI1;"]);
        // Split off with the command this family has. `FR0;` alone does not
        // cancel it here, and Yaesu's `FT2;` is not a command at all.
        assert!(f.contains(&"SP0;".to_string()));
        assert!(!f.iter().any(|c| c.starts_with("FT")));
    }

    #[test]
    fn the_transmit_input_is_asserted_only_when_it_is_configured() {
        let frame_for =
            |i| frames(Elad::new(i).clear_offsets()).into_iter().find(|f| f.starts_with("TI"));
        assert_eq!(frame_for(EladTxInput::UsbAudio), Some("TI1;".to_string()));
        assert_eq!(frame_for(EladTxInput::Mic), Some("TI0;".to_string()));
        assert_eq!(frame_for(EladTxInput::Auto), Some("TI2;".to_string()));
        // "Leave" means leave: a rig set up at the front panel keeps its setting.
        assert_eq!(frame_for(EladTxInput::Leave), None);
    }

    #[test]
    fn replies_split_across_reads_are_reassembled() {
        let mut e = elad();
        assert!(parse_str(&mut e, "FA000140").is_empty());
        assert_eq!(parse_str(&mut e, "74000;"), vec![CatUpdate::Freq(14_074_000.0)]);
    }

    #[test]
    fn the_rsi_reading_is_logged_but_never_shown() {
        // `RI` is the one absolute reading the rig offers and the manual gives
        // it no units, so it must not reach a meter.
        let mut e = elad();
        assert!(parse_str(&mut e, "RI-0730;").is_empty());
        assert!(parse_str(&mut e, "RI+0120;").is_empty());
    }

    #[test]
    fn this_family_has_no_text_keyer() {
        // `SW` plays a message stored in the radio; nothing hands it text. A
        // non-zero chunk length would have the CW panel streaming into a
        // command that cannot carry it.
        let mut e = elad();
        assert_eq!(e.cw_chunk_len(), 0);
        assert!(e.send_cw("cq de w1aw").is_empty());
        assert!(e.set_cw_wpm(20.0).is_empty());
    }
}
