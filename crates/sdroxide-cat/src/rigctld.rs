//! Hamlib `rigctld` over TCP — the catch-all, for a radio no native profile
//! covers.
//!
//! Every other profile in this crate speaks a manufacturer's own dialect and
//! knows things about it that a translation layer cannot: which Kenwood this is
//! and therefore what its meters read on, that an Elecraft's DATA mode needs a
//! sub-mode pinned behind it, how much text a rig's keyer buffer takes at a
//! time. Between them they still cover five makes. Hamlib covers a couple of
//! hundred, and where it is already installed and working there is no reason to
//! be locked out of it.
//!
//! What is given up is worth stating plainly. This profile reaches the
//! frequency, the mode, PTT, the transmit power, the S-meter and the SWR, and
//! nothing else — no keying from the rig's own text buffer, no filter tables,
//! no per-model anything. Where a native profile fits the radio, it is the
//! better answer.
//!
//! # The extended protocol
//!
//! Commands go out with a leading `+`, which is what makes the answers
//! *self-describing*. In the default protocol a `f` is answered with a bare
//! `14074000` and a `t` with a bare `1`, and nothing on the wire says which
//! reply is which — fine for a client that sends one command and blocks for its
//! answer, useless for one that pipelines a poll and reads whatever comes back.
//! With `+` the daemon answers in blocks:
//!
//! ```text
//! get_freq:
//! Frequency: 14074000
//! RPRT 0
//! ```
//!
//! The first line names the command, the middle lines carry labelled values and
//! `RPRT` closes the block with Hamlib's error code. That echo header is what
//! this file keys on rather than the value labels, because the labels are the
//! one part that has moved between Hamlib versions — and because `get_level`
//! answers three different meters under the same label, with only the header
//! saying which was asked for.
//!
//! Newlines terminate commands here, not semicolons: a `\n` is the frame
//! boundary, and [`crate::write_frame`]'s inter-frame gap is as harmless over a
//! socket as it is over a wire.

use crate::{CatUpdate, Protocol};
use sdroxide_types::Mode;
use tracing::debug;

/// Hamlib's own name for a mode.
///
/// Deliberately not the same table as `sdroxide-rigctld`'s: that one answers
/// for sdroxide *as* a radio, this one drives someone else's. The digital modes
/// all go out as `PKTUSB`, which is what tells a rig to take its audio from the
/// data input rather than the microphone.
fn mode_name(m: Mode) -> &'static str {
    match m {
        Mode::Lsb => "LSB",
        Mode::Usb | Mode::Spec | Mode::Sstv | Mode::Wefax | Mode::Navtex | Mode::RfPaint => "USB",
        Mode::Cw => "CW",
        Mode::Am | Mode::Sam | Mode::Drm => "AM",
        Mode::Dsb => "DSB",
        // Hamlib has no independent-sideband mode; AM is what a rig asked to
        // pass both sidebands would be put into anyway.
        Mode::Isb => "AM",
        Mode::Nfm => "FM",
        // No rig has an ADS-B mode and none ever will: the dial is at
        // 1090 MHz. Grouped with FM so nothing downstream has to special-case
        // a mode a radio can neither be put into nor report back.
        Mode::Wfm | Mode::Adsb | Mode::Vdl2 | Mode::Ais => "WFM",
        // Data over FM rather than over a sideband: the carrier is the signal's
        // centre, not one edge of it.
        Mode::Rifp | Mode::Packet | Mode::Aprs | Mode::SstvFm | Mode::RttyFm => "PKTFM",
        Mode::Digl => "PKTLSB",
        Mode::Digu
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
        | Mode::Rade => "PKTUSB",
    }
}

/// A mode Hamlib reported → the app's mode.
///
/// Deliberately not the inverse of [`mode_name`] over its whole range, on the
/// same principle as every other profile here: a rig position that would be
/// commanded back as something *else* yields `None`, or the two sides spend the
/// session correcting each other. Two cases matter. `RTTY`, because sdroxide's
/// RTTY is its own modem in a data sideband and would go back out as `PKTUSB`;
/// and `AMS`, because synchronous AM is this side's own detector riding on the
/// rig's plain AM — the rig is commanded to `AM` either way, so following its
/// sync setting back would flip the app into a mode it then un-asked for.
fn mode_from_name(s: &str) -> Option<Mode> {
    Some(match s.trim().to_ascii_uppercase().as_str() {
        "USB" => Mode::Usb,
        "LSB" => Mode::Lsb,
        // CW and its reverse are both CW to the app.
        "CW" | "CWR" => Mode::Cw,
        "AM" => Mode::Am,
        "DSB" => Mode::Dsb,
        "FM" | "FMN" => Mode::Nfm,
        "WFM" => Mode::Wfm,
        "PKTUSB" => Mode::Digu,
        "PKTLSB" => Mode::Digl,
        _ => return None,
    })
}

/// Which command's block the values now arriving belong to.
///
/// Only the ones this file asks for. Anything else — a daemon that volunteers
/// something, a reply to a command that has since been given up on — falls to
/// `Other` and is read past rather than guessed at.
#[derive(Clone, Copy, PartialEq)]
enum Block {
    Freq,
    Mode,
    Strength,
    Swr,
    Power,
    Ptt,
    Other,
}

pub struct Rigctld {
    buf: String,
    /// The block being read, from the echo header that opened it.
    block: Block,
    /// The daemon answered `RPRT` with a failure since this was last read.
    failed: bool,
}

impl Rigctld {
    pub fn new() -> Self {
        Rigctld { buf: String::new(), block: Block::Other, failed: false }
    }

    /// Interpret an echo header — the line that opens an extended-protocol
    /// block and names the command it answers.
    ///
    /// `get_level` carries the level's name as its argument, which is the only
    /// thing that separates a signal strength from an SWR from a power setting:
    /// all three come back as the same labelled value on the next line.
    fn open_block(&mut self, verb: &str, arg: &str) {
        self.block = match (verb, arg.trim()) {
            ("get_freq", _) => Block::Freq,
            ("get_mode", _) => Block::Mode,
            ("get_level", "STRENGTH") => Block::Strength,
            ("get_level", "SWR") => Block::Swr,
            ("get_level", "RFPOWER") => Block::Power,
            ("get_ptt", _) => Block::Ptt,
            _ => Block::Other,
        };
    }

    /// Interpret one labelled value inside the block now open.
    fn value(&mut self, value: &str, out: &mut Vec<CatUpdate>) {
        let v = value.trim();
        match self.block {
            // Hamlib carries a frequency as an integer number of hertz, but a
            // daemon that has been talking to a rig with fractional steps can
            // print it as a float — so it is parsed as one and rounded.
            Block::Freq => {
                if let Ok(hz) = v.parse::<f64>() {
                    out.push(CatUpdate::Freq(hz.round()));
                }
            }
            // A mode block carries the mode and then its passband; the passband
            // is not a mode, and `mode_from_name` refusing it is what keeps it
            // out.
            Block::Mode => {
                if let Some(m) = mode_from_name(v) {
                    out.push(CatUpdate::Mode(m));
                }
            }
            // `STRENGTH` is decibels relative to S9, which is the one meter
            // Hamlib defines in real units rather than a rig's own scale.
            Block::Strength => {
                if let Ok(db) = v.parse::<f32>() {
                    out.push(CatUpdate::Signal(crate::S9_DBM + db));
                }
            }
            // An SWR below 1:1 is not a reading, it is a daemon reporting a
            // meter it has nothing behind.
            Block::Swr => {
                if let Ok(swr) = v.parse::<f32>()
                    && swr >= 1.0
                {
                    out.push(CatUpdate::Swr(swr));
                }
            }
            Block::Power => {
                if let Ok(frac) = v.parse::<f32>() {
                    out.push(CatUpdate::Power(frac.clamp(0.0, 1.0)));
                }
            }
            // Hamlib's PTT is an enum, not a flag: 0 is receive and the values
            // above it name *which* input keyed the rig. Only the zero matters
            // here — this is asked for solely while sdroxide is not keying, so
            // anything else is the operator's own hand.
            Block::Ptt => {
                if let Ok(n) = v.parse::<i32>() {
                    out.push(CatUpdate::Ptt(n != 0));
                }
            }
            Block::Other => {}
        }
    }
}

impl Default for Rigctld {
    fn default() -> Self {
        Rigctld::new()
    }
}

/// One command, in the extended protocol and newline-terminated.
fn cmd(text: &str) -> Vec<u8> {
    format!("+{text}\n").into_bytes()
}

impl Protocol for Rigctld {
    fn set_freq(&mut self, hz: f64) -> Vec<u8> {
        cmd(&format!("F {}", hz.round().max(0.0) as u64))
    }

    fn set_mode(&mut self, m: Mode) -> Vec<u8> {
        // Passband 0 is Hamlib's "leave the filter alone" — the rig's own
        // default for the mode. sdroxide's filter width reaches the radio
        // through `set_filter` on the native profiles and not at all here, so
        // there is nothing to say about it and every reason not to guess.
        cmd(&format!("M {} 0", mode_name(m)))
    }

    fn ptt(&self, on: bool) -> Vec<u8> {
        cmd(if on { "T 1" } else { "T 0" })
    }

    fn poll_requests(&self) -> Vec<Vec<u8>> {
        vec![cmd("f"), cmd("m")]
    }
    fn dial_requests(&self) -> Vec<Vec<u8>> {
        vec![cmd("f")]
    }

    fn tx_telemetry_requests(&self) -> Vec<Vec<u8>> {
        vec![cmd("l SWR")]
    }

    /// `t` — Hamlib's own PTT read, which every backend it supports implements
    /// one way or another. Nothing here has to know how the rig behind the
    /// daemon spells it.
    fn tx_state_requests(&self) -> Vec<Vec<u8>> {
        vec![cmd("t")]
    }

    fn rx_telemetry_requests(&self) -> Vec<Vec<u8>> {
        vec![cmd("l STRENGTH")]
    }

    fn clear_offsets(&self) -> Vec<Vec<u8>> {
        // sdroxide carries RIT, XIT and split on the dial itself, so anything
        // the radio is still holding would add to ours unseen. Split off names
        // the receive VFO as the transmit one, which is what "off" means here.
        vec![cmd("J 0"), cmd("Z 0"), cmd("S 0 VFOA")]
    }

    fn set_power(&mut self, frac: f32) -> Vec<Vec<u8>> {
        // `RFPOWER` is already a 0..1 fraction of what the radio can do, which
        // is the one place this profile has an easier job than the native ones:
        // no watts, no per-model full scale, no assumption to get wrong.
        vec![cmd(&format!("L RFPOWER {:.6}", frac.clamp(0.0, 1.0)))]
    }

    fn read_power(&self) -> Vec<Vec<u8>> {
        vec![cmd("l RFPOWER")]
    }

    fn commands_power(&self) -> bool {
        true
    }

    fn refused(&mut self) -> bool {
        std::mem::take(&mut self.failed)
    }

    fn parse(&mut self, buf: &mut Vec<u8>) -> Vec<CatUpdate> {
        self.buf.push_str(&String::from_utf8_lossy(buf));
        buf.clear();
        let mut out = Vec::new();
        while let Some(idx) = self.buf.find('\n') {
            let line: String = self.buf.drain(..=idx).collect();
            let line = line.trim();
            if let Some(code) = line.strip_prefix("RPRT ") {
                // The block is over either way; a non-zero code is Hamlib's
                // errno, negated. Which command it belonged to is known — it is
                // the block we were reading — but the driver only asks about
                // this on the heels of a key-down, so it is enough to note that
                // *something* failed.
                if code.trim() != "0" {
                    debug!("rigctld: {line} closing a {}", label(self.block));
                    self.failed = true;
                }
                self.block = Block::Other;
                continue;
            }
            let Some((head, rest)) = line.split_once(':') else {
                continue;
            };
            // An echo header opens a block; anything else with a colon in it is
            // a value inside the one already open. The two are told apart by
            // the header being a command name, which is the only reliable
            // discriminator — value labels have moved between Hamlib releases.
            if head.starts_with("get_") || head.starts_with("set_") {
                self.open_block(head, rest);
            } else {
                self.value(rest, &mut out);
            }
        }
        out
    }
}

/// A block's name, for the log.
fn label(b: Block) -> &'static str {
    match b {
        Block::Freq => "frequency read",
        Block::Mode => "mode read",
        Block::Strength => "S-meter read",
        Block::Swr => "SWR read",
        Block::Power => "power read",
        Block::Ptt => "PTT read",
        Block::Other => "command",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(r: &mut Rigctld, s: &str) -> Vec<CatUpdate> {
        let mut buf = s.as_bytes().to_vec();
        r.parse(&mut buf)
    }

    fn text(v: Vec<u8>) -> String {
        String::from_utf8(v).unwrap()
    }

    fn frames(v: Vec<Vec<u8>>) -> Vec<String> {
        v.into_iter().map(text).collect()
    }

    #[test]
    fn every_command_goes_out_in_the_extended_protocol() {
        // The `+` is what makes the answers self-describing, and without it a
        // pipelined poll cannot tell one bare number from another.
        let mut r = Rigctld::new();
        assert_eq!(text(r.set_freq(14_074_000.0)), "+F 14074000\n");
        assert_eq!(text(r.set_mode(Mode::Usb)), "+M USB 0\n");
        assert_eq!(text(r.ptt(true)), "+T 1\n");
        assert_eq!(text(r.ptt(false)), "+T 0\n");
        assert_eq!(frames(r.poll_requests()), vec!["+f\n", "+m\n"]);
        assert_eq!(frames(r.set_power(0.5)), vec!["+L RFPOWER 0.500000\n"]);
        assert_eq!(frames(r.read_power()), vec!["+l RFPOWER\n"]);
        assert_eq!(frames(r.rx_telemetry_requests()), vec!["+l STRENGTH\n"]);
        assert_eq!(frames(r.tx_telemetry_requests()), vec!["+l SWR\n"]);
        assert_eq!(frames(r.clear_offsets()), vec!["+J 0\n", "+Z 0\n", "+S 0 VFOA\n"]);
    }

    #[test]
    fn a_block_is_read_against_the_header_that_opened_it() {
        let mut r = Rigctld::new();
        assert_eq!(
            parse_str(&mut r, "get_freq:\nFrequency: 14074000\nRPRT 0\n"),
            vec![CatUpdate::Freq(14_074_000.0)]
        );
        // A mode block carries the passband after the mode; it is not a mode.
        assert_eq!(
            parse_str(&mut r, "get_mode:\nMode: PKTUSB\nPassband: 2400\nRPRT 0\n"),
            vec![CatUpdate::Mode(Mode::Digu)]
        );
    }

    /// All three meters answer under the same value label. Only the header says
    /// which was asked for — read the wrong one and a flat antenna reads as a
    /// signal, or a strong signal as a fault.
    #[test]
    fn the_three_levels_are_told_apart_by_the_header_and_not_the_label() {
        let mut r = Rigctld::new();
        // `STRENGTH` is decibels relative to S9, so S9 itself is zero.
        assert_eq!(
            parse_str(&mut r, "get_level: STRENGTH\nLevel Value: 0\nRPRT 0\n"),
            vec![CatUpdate::Signal(-73.0)]
        );
        assert_eq!(
            parse_str(&mut r, "get_level: STRENGTH\nLevel Value: 20\nRPRT 0\n"),
            vec![CatUpdate::Signal(-53.0)]
        );
        assert_eq!(
            parse_str(&mut r, "get_level: SWR\nLevel Value: 1.500000\nRPRT 0\n"),
            vec![CatUpdate::Swr(1.5)]
        );
        assert_eq!(
            parse_str(&mut r, "get_level: RFPOWER\nLevel Value: 0.250000\nRPRT 0\n"),
            vec![CatUpdate::Power(0.25)]
        );
        // The label itself is not what is keyed on: Hamlib has moved it between
        // releases, and this has to keep working across them.
        assert_eq!(
            parse_str(&mut r, "get_level: SWR\nLevel: 2.000000\nRPRT 0\n"),
            vec![CatUpdate::Swr(2.0)]
        );
    }

    #[test]
    fn a_level_nobody_asked_for_moves_no_meter() {
        let mut r = Rigctld::new();
        // A daemon answering something else — a leftover from a command given
        // up on, or one sdroxide never sends — must not land on a meter.
        assert!(parse_str(&mut r, "get_level: ALC\nLevel Value: 3\nRPRT 0\n").is_empty());
        assert!(parse_str(&mut r, "get_vfo:\nVFO: VFOA\nRPRT 0\n").is_empty());
        // And a value with no block open at all is read past rather than
        // guessed at.
        assert!(parse_str(&mut r, "Level Value: 42\n").is_empty());
    }

    #[test]
    fn a_failure_code_is_noted_and_reported_once() {
        let mut r = Rigctld::new();
        assert!(!r.refused());
        // -1 is Hamlib's EINVAL; the block carries no value at all.
        assert!(parse_str(&mut r, "set_ptt: 1\nRPRT -1\n").is_empty());
        assert!(r.refused(), "the daemon said the command did not take");
        assert!(!r.refused(), "and it is reported exactly once");
        // A success is not a failure.
        parse_str(&mut r, "set_ptt: 1\nRPRT 0\n");
        assert!(!r.refused());
    }

    #[test]
    fn only_the_positions_that_round_trip_are_followed() {
        let mut r = Rigctld::new();
        // RTTY: sdroxide's is its own modem in a data sideband, and would be
        // commanded back as PKTUSB — so the two would take turns correcting
        // each other.
        assert!(parse_str(&mut r, "get_mode:\nMode: RTTY\nPassband: 300\nRPRT 0\n").is_empty());
        // Synchronous AM is this side's own detector on the rig's plain AM, so
        // the rig goes to `AM` either way and its sync setting is not followed.
        assert_eq!(text(r.set_mode(Mode::Sam)), "+M AM 0\n");
        assert!(parse_str(&mut r, "get_mode:\nMode: AMS\nPassband: 6000\nRPRT 0\n").is_empty());
        // Every mode this profile does command has to read back as itself.
        for m in [
            Mode::Lsb,
            Mode::Usb,
            Mode::Cw,
            Mode::Am,
            Mode::Dsb,
            Mode::Nfm,
            Mode::Wfm,
            Mode::Digl,
            Mode::Digu,
        ] {
            let sent = text(r.set_mode(m));
            let name = mode_name(m);
            assert_eq!(
                parse_str(&mut r, &format!("get_mode:\nMode: {name}\nPassband: 0\nRPRT 0\n")),
                vec![CatUpdate::Mode(m)],
                "{m:?} was set with {sent} and read back as something else"
            );
        }
    }

    #[test]
    fn replies_split_across_reads_are_reassembled() {
        let mut r = Rigctld::new();
        assert!(parse_str(&mut r, "get_freq:\nFrequ").is_empty());
        assert_eq!(
            parse_str(&mut r, "ency: 7030000\nRPRT 0\n"),
            vec![CatUpdate::Freq(7_030_000.0)]
        );
    }

    #[test]
    fn a_meter_with_nothing_behind_it_is_not_a_reading() {
        let mut r = Rigctld::new();
        // A daemon with no SWR to report answers zero, which is not an SWR any
        // antenna has ever had.
        assert!(parse_str(&mut r, "get_level: SWR\nLevel Value: 0.000000\nRPRT 0\n").is_empty());
        assert!(parse_str(&mut r, "get_level: SWR\nLevel Value: nope\nRPRT 0\n").is_empty());
    }
}
