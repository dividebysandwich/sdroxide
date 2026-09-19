//! (tr)uSDX CAT — DL2MAN/PE1NNZ's pocket QRP transceiver and the open uSDX
//! firmware it grew from. A fourth Kenwood dialect.
//!
//! The radio emulates a Kenwood TS-480 and answers `ID;` with `020`, so on
//! paper a Kenwood profile would drive it. On the wire the subset is thin
//! enough that it would also spend most of its frames being refused: the
//! firmware answers `?;` to `SM`, `RM`, `SL`, `SH`, `PC`, `AG` (the bare read),
//! `FB`, `FR`, `FT`, `RA`, `SQ` and the rest, because it has no S-meter, no
//! SWR, no power control, no VFO B, no split and no keyer. What it *does*
//! answer is:
//!
//! | Command | Meaning |
//! |---------|---------|
//! | `FA;` / `FAnnnnnnnnnnn;` | read / set the dial |
//! | `MD;` / `MDn;` | read / set the mode, 1..5 = LSB USB CW FM AM |
//! | `IF;` | status: the dial, and the transmit flag |
//! | `ID;` | `ID020;` |
//! | `PS;` | power on/off state |
//! | `TX;` / `RX;` | key / unkey |
//! | `AG0;` `FL0;` `RS;` `AI;` `VX;` `RC;` `RT1;` `XT1;` | the handful of |
//! |   | settings the firmware kept |
//!
//! Every one of those was confirmed against a bench radio rather than read off
//! the manual. The manual's own list is a *superset*: it names commands the
//! firmware's parser does not reach.
//!
//! # The audio is not on this link
//!
//! A (tr)uSDX has no sound card of its own, but its audio can be carried in
//! two ways and this profile is the ordinary one: the radio's 3.5 mm
//! speaker/mic jack is wired to a USB sound card, and sdroxide treats it as any
//! other CAT rig — control over the serial port, audio over the card. The other
//! way, an 8-bit stream *inside* the CAT link switched on with the firmware's
//! `UA` command, needs different framing and is not this profile.
//!
//! The stream is nevertheless switched **off** at open (`UA0;`), because a
//! previous session — or one of the community streaming drivers — may have left
//! it running, and a port carrying audio bytes is a port whose `;`-framed
//! replies cannot be read.
//!
//! # One thing about the hardware the driver has to work around
//!
//! Opening the port may reset the radio. On the common CH340 board the serial
//! adapter's DTR is wired to the ATmega's reset, and toggling DTR resets it
//! again. So DTR is held high for the whole session and never offered as a
//! keying line — a PTT on DTR would reboot the radio on every over.
//!
//! Written against a bench radio running DL2MAN firmware 2.x.

use crate::{CatUpdate, Protocol};
use sdroxide_types::Mode;
use tracing::{debug, info};

/// Digits in the `FA` frequency field, fixed across the family.
const FREQ_DIGITS: usize = 11;

/// The transmit flag's offset in an `IF;` reply body — the same positional
/// layout the TS-480 emulation copies.
const IF_TX_FLAG: usize = 26;
/// Length of that body, `IF` and `;` excluded.
const IF_BODY_LEN: usize = 35;

pub struct TrUsdx {
    /// Bytes arrived and not yet split into whole frames.
    buf: String,
    /// Mode digit from the radio's last `MD;` reply.
    mode_digit: Option<char>,
    /// True once `ID;` has answered `020`, so the model is logged once.
    identified: bool,
    /// The radio answered `?;` since this was last read.
    nak: bool,
}

impl TrUsdx {
    pub fn new() -> Self {
        TrUsdx { buf: String::new(), mode_digit: None, identified: false, nak: false }
    }

    /// The app's mode for the radio's mode digit.
    ///
    /// `MD1..5` are LSB, USB, CW, FM, AM — the TS-480's own order, and the one
    /// the firmware's `MD` handler emits (`mode + 1`). There is no DATA flag
    /// and no CW-R, so this is the inverse of [`mode_digit`] over the whole
    /// range.
    fn app_mode(&self) -> Option<Mode> {
        Some(match self.mode_digit? {
            '1' => Mode::Lsb,
            '2' => Mode::Usb,
            '3' => Mode::Cw,
            '4' => Mode::Nfm,
            '5' => Mode::Am,
            _ => return None,
        })
    }
}

impl Default for TrUsdx {
    fn default() -> Self {
        Self::new()
    }
}

/// The radio's mode digit for an app mode. There is no DATA position: a digital
/// mode rides a plain sideband and the audio does the rest, exactly as it does
/// on a rig with no DATA switch.
fn mode_digit(m: Mode) -> char {
    match m {
        Mode::Lsb => '1',
        Mode::Cw => '3',
        Mode::Am | Mode::Sam | Mode::Dsb | Mode::Isb | Mode::Drm | Mode::Acars => '5',
        // No (tr)uSDX is a general-coverage FM set, but it has the position and
        // the firmware answers it, so the modes sdroxide maps to FM go there.
        Mode::Nfm
        | Mode::Wfm
        | Mode::Rifp
        | Mode::Packet
        | Mode::Aprs
        | Mode::SstvFm
        | Mode::RttyFm
        | Mode::Adsb
        | Mode::Vdl2
        | Mode::Ais
        | Mode::HdRadio => '4',
        Mode::Usb
        | Mode::Spec
        | Mode::Sstv
        | Mode::Wefax
        | Mode::Navtex
        | Mode::RfPaint
        | Mode::Digl
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
        | Mode::Rade => '2',
    }
}

/// Whether the radio is transmitting, from the body of an `IF;` reply.
///
/// Positional, exactly as [`crate::kenwood`] reads it and for the same reason:
/// the layout is the TS-480's. The length is checked rather than indexed into
/// blindly, so a reply of another shape reports nothing at all.
fn if_transmitting(body: &str) -> Option<bool> {
    if body.len() != IF_BODY_LEN {
        return None;
    }
    match body.as_bytes()[IF_TX_FLAG] {
        b'0' => Some(false),
        b'1' => Some(true),
        _ => None,
    }
}

impl Protocol for TrUsdx {
    fn set_freq(&mut self, hz: f64) -> Vec<u8> {
        let hz = hz.round().clamp(0.0, 99_999_999_999.0) as u64;
        format!("FA{hz:0FREQ_DIGITS$};").into_bytes()
    }

    fn set_mode(&mut self, m: Mode) -> Vec<u8> {
        format!("MD{};", mode_digit(m)).into_bytes()
    }

    fn ptt(&self, on: bool) -> Vec<u8> {
        // `TX;` keys and `RX;` unkeys. Not `TX0;` — the manual lists it as a
        // *transmit* form, and the firmware's own `TX` handler treats the bare
        // command as the key.
        if on { b"TX;".to_vec() } else { b"RX;".to_vec() }
    }

    fn poll_requests(&self) -> Vec<Vec<u8>> {
        vec![b"FA;".to_vec(), b"MD;".to_vec()]
    }

    fn dial_requests(&self) -> Vec<Vec<u8>> {
        vec![b"FA;".to_vec()]
    }

    /// `IF;` for the transmit flag alone — see [`if_transmitting`].
    fn tx_state_requests(&self) -> Vec<Vec<u8>> {
        vec![b"IF;".to_vec()]
    }

    /// Stop any auto-information the radio may have been left in, and switch off
    /// the in-band audio stream a previous session may have left running — see
    /// the module comment. Order matters only in that both precede the first
    /// poll.
    fn open_requests(&self) -> Vec<Vec<u8>> {
        vec![b"AI0;".to_vec(), b"UA0;".to_vec()]
    }

    /// Clear the clarifier and switch RIT/XIT off. sdroxide carries RIT on the
    /// dial — the radio's dial is the only frequency control a CAT rig gives us
    /// — so an offset the radio is still holding would add to ours unseen.
    /// `RC;` clears the offset; the `RT0;`/`XT0;` that would switch it off are
    /// not in this firmware, so only the clear is sent.
    fn clear_offsets(&self) -> Vec<Vec<u8>> {
        vec![b"RC;".to_vec()]
    }

    /// DTR is the reset line on the common board — see the module comment.
    fn holds_dtr_high(&self) -> bool {
        true
    }

    fn refused(&mut self) -> bool {
        std::mem::take(&mut self.nak)
    }

    fn parse(&mut self, buf: &mut Vec<u8>) -> Vec<CatUpdate> {
        self.buf.push_str(&String::from_utf8_lossy(buf));
        buf.clear();
        let mut out = Vec::new();
        while let Some(idx) = self.buf.find(';') {
            let msg: String = self.buf.drain(..=idx).collect();
            out.extend(self.parse_frame(msg.trim_end_matches(';').trim()));
        }
        out
    }
}

impl TrUsdx {
    /// One `;`-delimited frame, without its delimiters.
    fn parse_frame(&mut self, msg: &str) -> Vec<CatUpdate> {
        let mut out = Vec::new();
        if let Some(rest) = msg.strip_prefix("FA") {
            if rest.len() == FREQ_DIGITS
                && let Ok(hz) = rest.parse::<u64>()
            {
                out.push(CatUpdate::Freq(hz as f64));
            }
        } else if let Some(rest) = msg.strip_prefix("MD") {
            if let Some(d) = rest.chars().next() {
                self.mode_digit = Some(d);
                if let Some(m) = self.app_mode() {
                    out.push(CatUpdate::Mode(m));
                }
            }
        } else if let Some(rest) = msg.strip_prefix("IF") {
            if let Some(on) = if_transmitting(rest) {
                out.push(CatUpdate::Ptt(on));
            }
        } else if let Some(rest) = msg.strip_prefix("ID") {
            if !self.identified && rest.trim() == "020" {
                self.identified = true;
                info!("(tr)uSDX CAT: radio identified (TS-480 emulation)");
            }
        } else if msg == "?" {
            self.nak = true;
            debug!("(tr)uSDX CAT: radio rejected a command (?)");
        } else if msg == "E" || msg == "O" {
            debug!("(tr)uSDX CAT: serial error from radio ({msg})");
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(p: &mut TrUsdx, s: &str) -> Vec<CatUpdate> {
        let mut b = s.as_bytes().to_vec();
        p.parse(&mut b)
    }

    #[test]
    fn the_frames_are_the_documented_shape() {
        let mut p = TrUsdx::new();
        assert_eq!(p.set_freq(14_031_000.0), b"FA00014031000;".to_vec());
        assert_eq!(p.set_mode(Mode::Usb), b"MD2;".to_vec());
        assert_eq!(p.set_mode(Mode::Cw), b"MD3;".to_vec());
        assert_eq!(p.ptt(true), b"TX;".to_vec());
        assert_eq!(p.ptt(false), b"RX;".to_vec());
        // Never `TX0;` — that is a transmit form on this family.
        assert_ne!(p.ptt(true), b"TX0;".to_vec());
    }

    #[test]
    fn only_the_commands_the_firmware_answers_are_claimed() {
        let mut p = TrUsdx::new();
        // No meters, no power, no filter, no squelch, no keyer.
        assert!(p.tx_telemetry_requests().is_empty());
        assert!(p.rx_telemetry_requests().is_empty());
        assert!(p.set_power(1.0).is_empty());
        assert!(!p.commands_power());
        assert!(p.set_filter(Mode::Usb, 300.0, 2700.0).is_empty());
        assert!(!p.commands_filter());
        assert!(p.set_squelch(0.5).is_empty());
        assert!(!p.commands_squelch());
        assert_eq!(p.cw_chunk_len(), 0);
        assert!(!p.commands_rig_power());
    }

    /// The port is opened with the in-band stream switched off, so a session
    /// left streaming by another driver does not pour audio into a parser
    /// expecting `;`-framed replies.
    #[test]
    fn the_in_band_stream_is_switched_off_at_open() {
        let p = TrUsdx::new();
        let reqs: Vec<String> =
            p.open_requests().iter().map(|f| String::from_utf8_lossy(f).into_owned()).collect();
        assert!(reqs.contains(&"UA0;".to_string()), "{reqs:?}");
    }

    #[test]
    fn a_frequency_and_mode_reply_are_read() {
        let mut p = TrUsdx::new();
        assert_eq!(feed(&mut p, ";FA00014031000;"), vec![CatUpdate::Freq(14_031_000.0)]);
        assert_eq!(feed(&mut p, ";MD3;"), vec![CatUpdate::Mode(Mode::Cw)]);
    }

    /// The transmit flag, out of the bench radio's own `IF;` reply.
    #[test]
    fn the_transmit_flag_comes_out_of_if_by_position() {
        let mut p = TrUsdx::new();
        assert_eq!(
            feed(&mut p, ";IF0001403100000000+000000000030000000;"),
            vec![CatUpdate::Ptt(false)]
        );
        let mut body = b"0001403100000000+000000000030000000".to_vec();
        body[IF_TX_FLAG] = b'1';
        let mut frame = String::from(";IF");
        frame.push_str(std::str::from_utf8(&body).unwrap());
        frame.push(';');
        assert_eq!(feed(&mut p, &frame), vec![CatUpdate::Ptt(true)]);
    }

    #[test]
    fn a_reply_split_across_two_reads_is_still_one_reply() {
        let mut p = TrUsdx::new();
        assert!(feed(&mut p, "FA0001403").is_empty());
        assert_eq!(feed(&mut p, "1000;"), vec![CatUpdate::Freq(14_031_000.0)]);
    }
}
