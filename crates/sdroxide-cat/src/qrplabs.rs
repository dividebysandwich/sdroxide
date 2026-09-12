//! QRP Labs CAT (ASCII, `;`-terminated) — QMX, QMX+ and the QDX-series
//! ancestors that share the command set. Frequency `FA<11 digits>;`, mode
//! `MD<x>;`, PTT `TX;`/`RX;`, CW streamed to the rig's own keyer (`KY`).
//!
//! A third Kenwood dialect, and the one that says so out loud: "QMX implements
//! a subset of the Kenwood TS-480/TS-440 CAT command set". The subset is small
//! and the additions are large, which is why it is a family rather than a
//! Kenwood with a note attached:
//!
//! * **A carriage return is not whitespace — it is a mode switch.** "QMX
//!   interprets an incoming carriage return character as a trigger to switch
//!   the serial port to terminal mode." One stray CR and the port stops being a
//!   CAT port for the rest of the session. Nothing here ever writes one, and
//!   nothing here ever should.
//! * **`PC` reads, it does not set.** On every other ASCII family `PC` is the
//!   transmit power control, in watts. Here it is the *power meter*, in tenths
//!   of a watt, and there is no CAT command that sets the power at all — so
//!   this family reports what the radio measured (`fwd_w`, real watts) and
//!   commands nothing. A Kenwood-shaped `PC005;` would be a meter read with
//!   junk on the end.
//! * **`MD8` is not a mode.** It is SWR Tune: the radio holds a carrier into
//!   its own bridge. It is never commanded from here, and a rig reporting it is
//!   reported as no mode rather than as some sideband it is not on.
//! * **The sound card is two different things.** `Q9` switches the USB codec
//!   between demodulated audio and the raw I/Q the ADC sees, so which one the
//!   operator picked in Settings has to be asserted *at the radio* — a rig left
//!   in I/Q mode by a previous session feeds quadrature into the audio path,
//!   which is noise, and one left out of it feeds audio into the panadapter,
//!   which is a spectrum of the demodulator.
//! * **`SS` decides whose voice goes out.** SSB transmit audio comes from the
//!   host's sound card, an internal two-tone generator, or the microphone. This
//!   interface only works on the first, so it asserts it — the ELAD `TI` trap
//!   in another dialect.
//!
//! Deliberately not used: `TA`, which transmits a *tone frequency* commanded
//! over CAT rather than audio through the sound card. It is the neatest digital
//! transmit path any radio here offers and it is the wrong shape for this
//! program, whose modems synthesise audio; the sound card is the interface both
//! ends already agree on.
//!
//! Written from QRP Labs' *QMX CAT programming manual* (firmware 1.04_004,
//! 23-Jul-2026) and the *QMX operating manual* (1_00_022) for the receiver's
//! architecture and its meter scales. **Not verified against a radio.**

use crate::{CatUpdate, Protocol};
use sdroxide_types::Mode;
use tracing::{debug, info, warn};

/// Digits in the `FA`/`FB` frequency field on a read — "returns the VFO A
/// contents as an 11-digit number". A set takes fewer ("FA7030000; sets VFO A
/// to 7.030MHz"), but is written at the full width anyway: it is what the
/// family's other members document, and a fixed field cannot be misread.
const FREQ_DIGITS: usize = 11;

/// Characters the `KY` buffer takes in one go.
///
/// The radio's own buffer is 80 characters and circular, but only in QRP Labs'
/// native `KY` format; in the TS-480 compatibility format the message is a
/// fixed 24. Which of the two a given radio is in is a menu setting this end
/// cannot read, so the chunk is sized to the smaller — a message that fits both
/// is one that goes out whichever way the radio is set.
const CW_MAX: usize = 24;

/// The bracketed prosigns the CW panel writes, and the single ASCII characters
/// this radio keys them as.
///
/// From the manual's own table, which is missing one entry: the line reads
/// "`>` is SK, is = KN" — the character for KN was lost somewhere between the
/// firmware and the PDF, in both the 1.02 and 1.04 editions. Guessing at it
/// would key *something* on the air, so `<KN>` is not here; it goes out as the
/// letters K and N instead, which is what a keyer without the prosign sends.
const PROSIGNS: [(&str, char); 7] =
    [("BT", '['), ("AR", '_'), ("AS", '<'), ("HH", '#'), ("SK", '>'), ("BK", '\\'), ("SN", '%')];

/// Ordinary punctuation the keyer is handed, on top of the prosign characters
/// above. An allow-list, because three of the prosign characters are ones an
/// operator might type meaning something else, and because a semicolon in the
/// payload would end the frame early and key the rest as a new command.
const KEYER_PUNCTUATION: &str = " ./?,=+-:";

/// The S-meter's zero, in dBm. "The entire AGC system operates on dB values,
/// using a reference base value equivalent to S0 (S-meter) which means -127
/// dBm", and the meter itself runs "S0 (-127dBm) to S9+36dB (-37dBm)" — which
/// is S9 at −73 dBm and 6 dB per S-unit, the ordinary calibration, expressed
/// from the bottom rather than from S9.
///
/// `SM;` answers "the S-meter value in dB", and this is the reference those dB
/// are above. ⚠️ The manual does not say so in as many words; it is read off
/// the AGC section, where the same reference is named outright.
const S0_DBM: f32 = -127.0;

/// The top of that scale, in dB above [`S0_DBM`] — S9+36. A reading past it is
/// not a signal 100 dB over nine, it is a reply this parser has misread, so it
/// is clamped rather than believed.
const SMETER_MAX_DB: f32 = 90.0;

/// The firmware that first had `SR` — the SWR-protection latch — per the
/// manual's own revision history ("1_04_004 23-Jul-2026 ... Added commands GP
/// and SR"). Older radios answer `?;`, and there is no sense asking one of them
/// twice a second for the rest of the session.
const SR_SINCE: (u32, u32, u32) = (1, 4, 4);

pub struct QrpLabs {
    buf: String,
    /// Whether this session wants the radio's USB codec carrying raw I/Q rather
    /// than demodulated audio — the sound format the operator chose, asserted
    /// at the radio with `Q9`. See the module comment.
    iq: bool,
    /// Mode digit from the rig's last `MD;` reply.
    mode_digit: Option<char>,
    /// Firmware version from `VN;`, as (major, minor, patch). `None` on a radio
    /// that has not answered — which is itself evidence, `VN` being newer than
    /// the 1.02 command set.
    firmware: Option<(u32, u32, u32)>,
    /// Whether the radio's SWR protection is latched, as last reported. `None`
    /// until it has said. Held so the log carries the *change* rather than a
    /// line every poll.
    swr_locked: Option<bool>,
    /// The radio answered `?;` since this was last read (see
    /// [`Protocol::refused`]).
    nak: bool,
    /// True once the radio has named itself, so a reconnect's second `OM;`
    /// does not re-announce the same radio.
    identified: bool,
}

impl QrpLabs {
    pub fn new(iq: bool) -> Self {
        QrpLabs {
            buf: String::new(),
            iq,
            mode_digit: None,
            firmware: None,
            swr_locked: None,
            nak: false,
            identified: false,
        }
    }

    /// Whether the radio's firmware is at least `want`, as far as `VN;` has
    /// said. False on a radio that has not answered it — which is the safe
    /// direction: `VN` is itself newer than the 1.02 command set, so silence
    /// means old, and an old radio is not asked for commands it lacks.
    fn firmware_at_least(&self, want: (u32, u32, u32)) -> bool {
        self.firmware.is_some_and(|v| v >= want)
    }

    /// The app's mode for the rig's mode digit, or `None` for a position that
    /// is not one.
    ///
    /// Deliberately not the inverse of [`mode_digit`] over its whole range.
    /// `MD8` is SWR Tune — a carrier into the radio's own bridge, not a mode —
    /// and `MD4` (FM on a TS-480) is a position no QMX has. Reporting either as
    /// the nearest sideband would put the app and the radio in a loop of
    /// correcting each other.
    fn app_mode(&self) -> Option<Mode> {
        Some(match self.mode_digit? {
            '1' => Mode::Lsb,
            '2' => Mode::Usb,
            // CW and CW-R are both CW to the app; the reverse is which side of
            // the carrier the radio listens on, not a different mode.
            '3' | '7' => Mode::Cw,
            '5' => Mode::Am,
            // "6 (FSK)" and "9 (FSR/FSK Reverse)" in the CAT manual are the
            // radio's own DIGI-U and DIGI-L: the sound-card data modes, the
            // ones this interface exists for.
            '6' => Mode::Digu,
            '9' => Mode::Digl,
            _ => return None,
        })
    }
}

/// The radio's mode digit for an app mode. `MD`: 1 LSB, 2 USB, 3 CW, 5 AM,
/// 6 FSK (DIGI-U), 7 CW-R, 8 SWR Tune, 9 FSR (DIGI-L). There is no 4 — the
/// TS-480's FM — and 8 is never sent from here.
fn mode_digit(m: Mode) -> char {
    match m {
        Mode::Lsb => '1',
        Mode::Cw => '3',
        Mode::Am | Mode::Sam | Mode::Dsb | Mode::Isb | Mode::Drm => '5',
        Mode::Digl => '9',
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
        | Mode::Rade => '6',
        // No QMX has an FM position at all, so there is nothing closer to ask
        // for than a sideband — which at least leaves the sound card in the
        // transmit path. What actually goes out on one is then whatever the
        // engine modulated, through an SSB transmitter.
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
        | Mode::Ais => '2',
        Mode::Usb | Mode::Spec | Mode::Sstv | Mode::Wefax | Mode::Navtex | Mode::RfPaint => '2',
    }
}

/// Reduce `text` to what the `KY` buffer will accept.
///
/// The bracketed prosigns the CW panel writes become the single characters this
/// radio keys them as, and a token with no character of its own goes as its
/// letters. Everything else is filtered to letters, digits and
/// [`KEYER_PUNCTUATION`] — which is what keeps a semicolon out of the payload,
/// and keeps a stray `<` or `>` from being keyed as AS or SK.
fn keyer_text(text: &str) -> String {
    let up = text.trim().to_ascii_uppercase();
    // The prosign pass has to be a scan rather than a series of replacements:
    // three of the characters it *produces* (`<`, `>`, `#`) are characters an
    // operator can also type, so the two have to be told apart while the
    // brackets are still there to tell them apart by.
    let mut expanded = String::with_capacity(up.len());
    let mut chars = up.chars();
    while let Some(c) = chars.next() {
        match c {
            '<' => {
                let mut token = String::new();
                for n in chars.by_ref() {
                    if n == '>' {
                        break;
                    }
                    token.push(n);
                }
                match PROSIGNS.iter().find(|(name, _)| *name == token) {
                    Some((_, symbol)) => expanded.push(*symbol),
                    // A prosign with no character on this radio — `<KN>`, and
                    // anything the operator invented. Its letters run together,
                    // which is a prosign keyed the long way rather than nothing.
                    None => expanded.push_str(&token),
                }
            }
            // A closer with no opener. Dropped rather than passed through:
            // on this radio a bare `>` keys SK, which is not what somebody who
            // mistyped a bracket meant to put on the air.
            '>' => {}
            _ => expanded.push(c),
        }
    }
    let prosign_chars: String = PROSIGNS.iter().map(|(_, c)| *c).collect();
    expanded
        .chars()
        // A line break is a word break, not nothing: dropping it would run the
        // end of one line into the start of the next and send them as one word.
        .map(|c| if c.is_ascii_whitespace() { ' ' } else { c })
        .filter(|c| {
            c.is_ascii_alphanumeric()
                || KEYER_PUNCTUATION.contains(*c)
                || prosign_chars.contains(*c)
        })
        // Collapse the runs of spaces a trimmed chunk boundary can leave behind.
        .scan(false, |prev_space, c| {
            let space = c == ' ';
            let keep = !(space && *prev_space);
            *prev_space = space;
            Some(keep.then_some(c))
        })
        .flatten()
        .take(CW_MAX)
        .collect::<String>()
        .trim()
        .to_string()
}

impl Protocol for QrpLabs {
    fn set_freq(&mut self, hz: f64) -> Vec<u8> {
        let hz = hz.round().clamp(0.0, 99_999_999_999.0) as u64;
        format!("FA{hz:0FREQ_DIGITS$};").into_bytes()
    }

    fn set_mode(&mut self, m: Mode) -> Vec<u8> {
        format!("MD{};", mode_digit(m)).into_bytes()
    }

    fn ptt(&self, on: bool) -> Vec<u8> {
        // The documented pair, and no parameter on either: `TX;` "immediately
        // puts the radio into transmit mode", `RX;` immediately takes it out.
        // Not the Kenwood `TX0;`/`TX1;` — this family spells the parameterised
        // form `TQ0;`/`TQ1;`, and a `TX0;` is a `TX;` with a stray digit behind
        // it.
        if on { b"TX;".to_vec() } else { b"RX;".to_vec() }
    }

    fn poll_requests(&self) -> Vec<Vec<u8>> {
        vec![b"FA;".to_vec(), b"MD;".to_vec()]
    }

    fn dial_requests(&self) -> Vec<Vec<u8>> {
        vec![b"FA;".to_vec()]
    }

    fn open_requests(&self) -> Vec<Vec<u8>> {
        // Which radio and which firmware, in that order, before anything else.
        // The firmware is not decoration: half the command set here postdates
        // the QDX-compatible core, and `SR` below is asked for only on a
        // version that has it.
        vec![b"OM;".to_vec(), b"VN;".to_vec()]
    }

    fn tx_state_requests(&self) -> Vec<Vec<u8>> {
        let mut reqs = vec![b"TQ;".to_vec()];
        // Whether the radio has latched its SWR protection — which is not a
        // meter reading but a refusal: while it is set the transmitter will not
        // key at all, and nothing else on the link says why. Only on firmware
        // that has the command; older radios would answer `?;` to it forever.
        if self.firmware_at_least(SR_SINCE) {
            reqs.push(b"SR;".to_vec());
        }
        reqs
    }

    fn tx_telemetry_requests(&self) -> Vec<Vec<u8>> {
        // The SWR, and the power meter — which on this family is a real
        // measurement in watts rather than a needle position, so it is worth
        // both frames.
        vec![b"SW;".to_vec(), b"PC;".to_vec()]
    }

    fn rx_telemetry_requests(&self) -> Vec<Vec<u8>> {
        vec![b"SM;".to_vec()]
    }

    fn clear_offsets(&self) -> Vec<Vec<u8>> {
        vec![
            // Stop unsolicited status the radio would otherwise push at us: in
            // auto-information mode it sends an `IF;` reply on every change
            // *and* one every 1.5 seconds regardless. We poll, and a previous
            // program may well have left it on.
            b"AI0;".to_vec(),
            // Clear the clarifier, then switch RIT off. sdroxide carries RIT on
            // the dial — the radio's dial is the only frequency control a CAT
            // rig gives us — so an offset the radio is still holding would add
            // to ours unseen.
            b"RC;".to_vec(),
            b"RT0;".to_vec(),
            // Simplex, on VFO A. `SP0;` is the documented split switch and
            // `FR0;` sets the VFO mode, which on this family is one setting
            // shared by `FR` and `FT` ("the VFO mode use does not correspond
            // exactly to TS-480"). Both, because the second is also the fix for
            // a radio left on VFO B, where every `FA` we send would land on a
            // VFO nothing is listening to.
            b"SP0;".to_vec(),
            b"FR0;".to_vec(),
            // Transmit audio from the host's sound card — not the internal
            // two-tone generator, and not the microphone. This is what makes
            // the interface work at all: a radio left on "external microphone"
            // sends the room instead, and nothing on screen says so.
            b"SS0;".to_vec(),
            // And the receive side of the same question: the USB codec carries
            // either demodulated audio or the raw I/Q the ADC sees, and which
            // one it must carry is the sound format the operator chose here.
            // Asserted in both directions, because either one left over from a
            // previous session is silently wrong — quadrature fed to the
            // demodulators is noise, and audio fed to the panadapter is a
            // picture of the demodulator rather than of the band.
            if self.iq { b"Q91;".to_vec() } else { b"Q90;".to_vec() },
        ]
    }

    fn cw_chunk_len(&self) -> usize {
        CW_MAX
    }

    fn send_cw(&mut self, text: &str) -> Vec<Vec<u8>> {
        let msg = keyer_text(text);
        if msg.is_empty() {
            return Vec::new();
        }
        // One space between `KY` and the text — the documented parameter in
        // both of this family's `KY` formats, and the difference between keying
        // and a syntax error.
        //
        // Unpadded, which is the native format's own example (`KY HELLO;`). The
        // TS-480 compatibility format documents a fixed 24 characters padded
        // with spaces, but padding is the more dangerous way to be wrong of the
        // two: on a radio in the native format the padding is keyed as up to 23
        // word gaps after every chunk, where a short message on a radio in the
        // compatibility format is at worst one the parser fills in itself.
        //
        // Nothing turns break-in on first, unlike Kenwood: `KY` keys the
        // transmitter itself here, and this family has no `VX`.
        vec![format!("KY {msg};").into_bytes()]
    }

    fn abort_cw(&mut self) -> Vec<Vec<u8>> {
        // The whole of the stop this family has. The TS-480 compatibility
        // format also stops on a message of 24 spaces, but that is not sent
        // here: on a radio in the native format the same frame appends 24 word
        // gaps to an 80-character circular buffer, and the *next* over would
        // open with them.
        vec![b"RX;".to_vec()]
    }

    fn set_cw_wpm(&mut self, wpm: f32) -> Vec<Vec<u8>> {
        // ⚠️ The floor is not there to be tidy. On this radio "setting speed to
        // 0 enables Straight Key mode regardless of the keyer mode setting" —
        // so a `KS000;` is not a very slow keyer, it is a radio that has
        // stopped being a keyer, and the paddle stops working with it.
        let wpm = wpm.round().clamp(5.0, 60.0) as u32;
        vec![format!("KS{wpm:03};").into_bytes()]
    }

    // No `set_power`, and `commands_power` stays false: `PC` on this family
    // reads the power meter, and no plain CAT command sets the output power at
    // all. What reaches the transmitter is the level of the audio going into
    // its sound card, which is the Drive slider's other job.

    // No `set_filter` either. `FW;` reads the bandwidth the radio's own mode
    // implies — 3200 in Digi, 300 in CW — and there is nothing to write it
    // with.

    fn refused(&mut self) -> bool {
        std::mem::take(&mut self.nak)
    }

    fn parse(&mut self, buf: &mut Vec<u8>) -> Vec<CatUpdate> {
        self.buf.push_str(&String::from_utf8_lossy(buf));
        buf.clear();
        let mut out = Vec::new();
        while let Some(idx) = self.buf.find(';') {
            let msg: String = self.buf.drain(..=idx).collect();
            let msg = msg.trim_end_matches(';').trim();
            if let Some(rest) = msg.strip_prefix("FA") {
                // At the documented width and nothing else: a reply of some
                // other length is not a frequency at a different scale, it is a
                // reply this parser does not understand.
                if rest.len() == FREQ_DIGITS
                    && let Ok(hz) = rest.parse::<u64>()
                {
                    out.push(CatUpdate::Freq(hz as f64));
                }
            } else if let Some(rest) = msg.strip_prefix("MD") {
                if let Some(d) = rest.chars().next() {
                    self.mode_digit = Some(d);
                    match self.app_mode() {
                        Some(m) => out.push(CatUpdate::Mode(m)),
                        // `MD8` is the one worth a word: the radio is holding a
                        // carrier into its own SWR bridge, which is not a mode
                        // and is also not a moment to be commanding one.
                        None if d == '8' => debug!("QRP Labs CAT: radio is in SWR Tune"),
                        None => {}
                    }
                }
            } else if let Some(rest) = msg.strip_prefix("TQ") {
                // `TQ0;` receive, `TQ1;` transmit — the transmit query, which
                // this family has as a command of its own.
                if let Some(d) = rest.chars().next().filter(char::is_ascii_digit) {
                    out.push(CatUpdate::Ptt(d != '0'));
                }
            } else if let Some(rest) = msg.strip_prefix("SW") {
                // Hundredths of a unit: "SW121; ... indicates that the SWR is
                // 1.21:1". In receive the radio answers a bare `SW;`, which is
                // not a reading of zero and must not be reported as one.
                if let Ok(hundredths) = rest.parse::<u32>() {
                    out.push(CatUpdate::Swr((hundredths as f32 / 100.0).max(1.0)));
                }
            } else if let Some(rest) = msg.strip_prefix("PC") {
                // Tenths of a watt, and *measured* — "if command PC; returned
                // PC45; then this would mean the output power is currently
                // measured as 4.5 W". Watts a radio has actually measured are
                // `fwd_w`, not a needle position: see [`CatUpdate::FwdW`].
                if let Ok(tenths) = rest.parse::<u32>() {
                    out.push(CatUpdate::FwdW(tenths as f32 / 10.0));
                }
            } else if let Some(rest) = msg.strip_prefix("SM") {
                // dB above S0, which this family puts at −127 dBm. Parsed
                // width-agnostically because the manual gives none: a TS-480
                // pads the field and a QMX may not, and both read the same as
                // a number.
                if let Ok(db) = rest.parse::<u32>() {
                    let db = (db as f32).min(SMETER_MAX_DB);
                    out.push(CatUpdate::Signal(S0_DBM + db));
                }
            } else if let Some(rest) = msg.strip_prefix("SR") {
                // Not a meter: the SWR protection latch. While it is set the
                // radio refuses to transmit, and an operator watching a
                // transmitter that will not key has nothing else to go on.
                // Deliberately not reset from here — clearing a protection trip
                // is a decision about an antenna, and it belongs to whoever can
                // see the antenna.
                if let Some(d) = rest.chars().next().filter(char::is_ascii_digit) {
                    let locked = d != '0';
                    if self.swr_locked != Some(locked) {
                        if locked {
                            warn!(
                                "the radio's SWR protection has tripped and is latched — it \
                                 will not transmit until it is reset at the radio; check the \
                                 antenna first"
                            );
                        } else if self.swr_locked.is_some() {
                            info!("QRP Labs CAT: the radio's SWR protection has been reset");
                        }
                        self.swr_locked = Some(locked);
                    }
                }
            } else if let Some(rest) = msg.strip_prefix("OM") {
                // "For QMX this is QC so the result is simply OMQC;" — two
                // letters that name the model, and the only thing on the link
                // that does.
                if !self.identified {
                    self.identified = true;
                    info!(model = rest.trim(), "QRP Labs CAT: radio identified");
                }
            } else if let Some(rest) = msg.strip_prefix("VN") {
                // "VN1_00_021QMX;" — the firmware file's own name, without the
                // dot. Underscore-separated, with the product glued to the end.
                if let Some(v) = parse_firmware(rest) {
                    if self.firmware != Some(v) {
                        info!(
                            version = %format!("{}.{:02}.{:03}", v.0, v.1, v.2),
                            "QRP Labs CAT: firmware"
                        );
                    }
                    self.firmware = Some(v);
                }
            } else if msg == "?" {
                // A real error return on this family — "if any parameters are
                // invalid ... the command returns an error ?;" — but also what
                // an older radio answers to any of the commands it predates,
                // several of which go out when the port opens. The caller only
                // consults it in the one place where it can be diagnosed: on
                // the heels of a key-down.
                self.nak = true;
                debug!("QRP Labs CAT: radio rejected a command (?)");
            }
        }
        out
    }
}

/// The `(major, minor, patch)` in a `VN;` reply body — `1_04_004QMX` is
/// `(1, 4, 4)`. `None` for anything that is not three underscore-separated
/// numbers, which includes the silence of a radio too old to have the command.
fn parse_firmware(rest: &str) -> Option<(u32, u32, u32)> {
    let mut parts = rest.trim().splitn(3, '_');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    // The product name is glued to the patch number with no separator, so the
    // digits are taken and the letters left behind.
    let patch = parts.next()?;
    let digits: String = patch.chars().take_while(char::is_ascii_digit).collect();
    Some((major, minor, digits.parse().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(q: &mut QrpLabs, s: &str) -> Vec<CatUpdate> {
        let mut buf = s.as_bytes().to_vec();
        q.parse(&mut buf)
    }

    fn frames(v: Vec<Vec<u8>>) -> Vec<String> {
        v.iter().map(|f| String::from_utf8_lossy(f).into_owned()).collect()
    }

    fn radio() -> QrpLabs {
        QrpLabs::new(false)
    }

    /// ⛔ A carriage return anywhere in what this profile writes takes the
    /// radio's serial port out of CAT mode and into its terminal, for the rest
    /// of the session. Every frame the profile can generate is checked here
    /// rather than trusted, because the failure is silent and unrecoverable
    /// without a reconnect.
    #[test]
    fn nothing_this_profile_writes_contains_a_carriage_return() {
        let mut q = QrpLabs::new(true);
        let mut all: Vec<Vec<u8>> = Vec::new();
        all.push(q.set_freq(14_074_000.0));
        all.push(q.ptt(true));
        all.push(q.ptt(false));
        for m in [Mode::Lsb, Mode::Usb, Mode::Cw, Mode::Am, Mode::Digu, Mode::Digl, Mode::Nfm] {
            all.push(q.set_mode(m));
        }
        all.extend(q.poll_requests());
        all.extend(q.dial_requests());
        all.extend(q.open_requests());
        all.extend(q.clear_offsets());
        all.extend(q.tx_telemetry_requests());
        all.extend(q.rx_telemetry_requests());
        all.extend(q.tx_state_requests());
        all.extend(q.send_cw("cq de w1aw\r\nur 599"));
        all.extend(q.abort_cw());
        all.extend(q.set_cw_wpm(22.0));
        for f in all {
            assert!(
                !f.contains(&b'\r') && !f.contains(&b'\n'),
                "frame would switch the port to terminal mode: {}",
                String::from_utf8_lossy(&f)
            );
        }
    }

    #[test]
    fn frequency_is_eleven_digits() {
        let mut q = radio();
        assert_eq!(q.set_freq(7_030_000.0), b"FA00007030000;".to_vec());
        assert_eq!(parse_str(&mut q, "FA00007030000;"), vec![CatUpdate::Freq(7_030_000.0)]);
        // Anything else is not a frequency at some other scale.
        assert!(parse_str(&mut q, "FA007030000;").is_empty());
        assert!(parse_str(&mut q, "FAxxxxxxxxxxx;").is_empty());
    }

    #[test]
    fn every_app_mode_the_radio_can_hold_survives_a_round_trip() {
        for m in [Mode::Lsb, Mode::Usb, Mode::Cw, Mode::Am, Mode::Digu, Mode::Digl] {
            let mut q = radio();
            let sent = String::from_utf8(q.set_mode(m)).unwrap();
            assert_eq!(
                parse_str(&mut q, &sent),
                vec![CatUpdate::Mode(m)],
                "{m:?} was set with {sent} and read back as something else"
            );
        }
    }

    #[test]
    fn the_digital_modes_are_the_radios_own_digi_sidebands() {
        let mut q = radio();
        // "6 (FSK)" and "9 (FSR)" in the CAT manual are DIGI-U and DIGI-L at
        // the radio — the sound-card modes, not an FSK keyer.
        assert_eq!(q.set_mode(Mode::Ft8), b"MD6;".to_vec());
        assert_eq!(q.set_mode(Mode::Rtty), b"MD6;".to_vec());
        assert_eq!(q.set_mode(Mode::Digl), b"MD9;".to_vec());
        assert_eq!(parse_str(&mut q, "MD6;"), vec![CatUpdate::Mode(Mode::Digu)]);
        assert_eq!(parse_str(&mut q, "MD9;"), vec![CatUpdate::Mode(Mode::Digl)]);
    }

    /// `MD8` is SWR Tune — a carrier into the radio's own bridge. Reported as
    /// no mode at all, never as the nearest sideband, and never commanded.
    #[test]
    fn swr_tune_is_not_a_mode() {
        let mut q = radio();
        assert!(parse_str(&mut q, "MD8;").is_empty());
        // Nor is 4: the TS-480's FM, which no QMX has.
        assert!(parse_str(&mut q, "MD4;").is_empty());
        for m in [Mode::Lsb, Mode::Usb, Mode::Cw, Mode::Am, Mode::Digu, Mode::Digl, Mode::Nfm] {
            assert_ne!(mode_digit(m), '8', "{m:?} would put the radio into SWR Tune");
        }
    }

    #[test]
    fn keying_takes_no_parameter_at_either_end() {
        let q = radio();
        assert_eq!(q.ptt(true), b"TX;".to_vec());
        assert_eq!(q.ptt(false), b"RX;".to_vec());
        assert_eq!(frames(q.tx_state_requests()), vec!["TQ;"]);
    }

    #[test]
    fn the_transmit_query_answers_the_rigs_own_over() {
        let mut q = radio();
        assert_eq!(parse_str(&mut q, "TQ0;"), vec![CatUpdate::Ptt(false)]);
        assert_eq!(parse_str(&mut q, "TQ1;"), vec![CatUpdate::Ptt(true)]);
        // The bare query echoed back is not an answer.
        assert!(parse_str(&mut q, "TQ;").is_empty());
    }

    #[test]
    fn the_power_meter_reads_in_watts_and_sets_nothing() {
        let mut q = radio();
        // "PC45; ... the output power is currently measured as 4.5 W".
        assert_eq!(parse_str(&mut q, "PC45;"), vec![CatUpdate::FwdW(4.5)]);
        assert_eq!(parse_str(&mut q, "PC50;"), vec![CatUpdate::FwdW(5.0)]);
        assert_eq!(parse_str(&mut q, "PC0;"), vec![CatUpdate::FwdW(0.0)]);
        // ⛔ On every other ASCII family `PC` is the power *control*. Here it is
        // not, and nothing must ever write one: a Kenwood-shaped `PC005;` is a
        // meter read with junk behind it.
        assert!(!q.commands_power());
        assert!(q.set_power(1.0).is_empty());
        assert!(q.read_power().is_empty());
        assert_eq!(frames(q.tx_telemetry_requests()), vec!["SW;", "PC;"]);
    }

    #[test]
    fn swr_arrives_in_hundredths_and_receive_carries_none() {
        let mut q = radio();
        assert_eq!(parse_str(&mut q, "SW121;"), vec![CatUpdate::Swr(1.21)]);
        assert_eq!(parse_str(&mut q, "SW300;"), vec![CatUpdate::Swr(3.0)]);
        // "If called while the radio is in Receive mode, the command simply
        // returns SW;" — an absence, not an SWR of zero.
        assert!(parse_str(&mut q, "SW;").is_empty());
    }

    #[test]
    fn the_s_meter_reads_in_db_above_the_familys_own_zero() {
        let mut q = radio();
        let dbm = |u: &[CatUpdate]| match u {
            [CatUpdate::Signal(d)] => *d,
            other => panic!("expected one signal reading, got {other:?}"),
        };
        // S0 is −127 dBm and an S-unit is 6 dB, so S9 (54 dB up) is −73.
        assert_eq!(dbm(&parse_str(&mut q, "SM0;")), -127.0);
        assert_eq!(dbm(&parse_str(&mut q, "SM54;")), -73.0);
        assert_eq!(dbm(&parse_str(&mut q, "SM0054;")), -73.0); // padded reads the same
        // The top of the published scale, S9+36.
        assert_eq!(dbm(&parse_str(&mut q, "SM90;")), -37.0);
        // Past it is a misread reply, not a signal 100 dB over nine.
        assert_eq!(dbm(&parse_str(&mut q, "SM999;")), -37.0);
        assert_eq!(frames(q.rx_telemetry_requests()), vec!["SM;"]);
    }

    /// The sound card is two different things, and which one it is has to be
    /// asserted at the radio — in *both* directions, or a session left over
    /// from the other setting is silently wrong.
    #[test]
    fn the_sound_format_is_asserted_at_the_radio() {
        let iq = frames(QrpLabs::new(true).clear_offsets());
        assert!(iq.contains(&"Q91;".to_string()), "{iq:?}");
        assert!(!iq.contains(&"Q90;".to_string()), "{iq:?}");
        let audio = frames(QrpLabs::new(false).clear_offsets());
        assert!(audio.contains(&"Q90;".to_string()), "{audio:?}");
        assert!(!audio.contains(&"Q91;".to_string()), "{audio:?}");
    }

    #[test]
    fn the_opening_sequence_normalises_what_the_radio_was_left_on() {
        let f = frames(radio().clear_offsets());
        assert_eq!(f, vec!["AI0;", "RC;", "RT0;", "SP0;", "FR0;", "SS0;", "Q90;"]);
        // Auto-information first: everything after it is read against replies
        // we asked for, and a radio still pushing `IF;` every 1.5 seconds is
        // one talking over them.
        assert_eq!(f[0], "AI0;");
        // The clarifier is cleared before RIT is switched off, the way the
        // family's other members need it.
        assert!(f.iter().position(|c| c == "RC;") < f.iter().position(|c| c == "RT0;"));
        // Transmit audio from the host, never the microphone or the internal
        // two-tone generator.
        assert!(f.contains(&"SS0;".to_string()));
    }

    /// `SR` arrived in firmware 1.04.004. A radio that has not said it is that
    /// new is not asked, because an older one answers `?;` and would go on
    /// answering it for the rest of the session.
    #[test]
    fn the_protection_latch_is_only_asked_for_where_it_exists() {
        let mut q = radio();
        assert_eq!(frames(q.tx_state_requests()), vec!["TQ;"]);
        parse_str(&mut q, "VN1_04_003QMX;");
        assert_eq!(frames(q.tx_state_requests()), vec!["TQ;"]);
        parse_str(&mut q, "VN1_04_004QMX;");
        assert_eq!(frames(q.tx_state_requests()), vec!["TQ;", "SR;"]);
        // And the introductions that make that possible go out first.
        assert_eq!(frames(q.open_requests()), vec!["OM;", "VN;"]);
    }

    #[test]
    fn the_firmware_version_is_read_out_of_the_file_name() {
        assert_eq!(parse_firmware("1_04_004QMX"), Some((1, 4, 4)));
        assert_eq!(parse_firmware("1_00_021QMX"), Some((1, 0, 21)));
        assert_eq!(parse_firmware("1_04_004"), Some((1, 4, 4)));
        assert_eq!(parse_firmware(""), None);
        assert_eq!(parse_firmware("QMX"), None);
    }

    /// The latch is a refusal, not a reading: while it is set the transmitter
    /// will not key. Reported once per change rather than once per poll.
    #[test]
    fn the_protection_latch_is_followed_but_never_reset() {
        let mut q = radio();
        assert!(parse_str(&mut q, "SR1;").is_empty());
        assert_eq!(q.swr_locked, Some(true));
        assert!(parse_str(&mut q, "SR0;").is_empty());
        assert_eq!(q.swr_locked, Some(false));
        // ⛔ Nothing here writes `SR0;`. Clearing a protection trip is a
        // decision about an antenna, and it belongs to whoever can see it.
        let mut written = frames(q.clear_offsets());
        written.extend(frames(q.open_requests()));
        written.extend(frames(q.tx_state_requests()));
        assert!(!written.iter().any(|c| c.starts_with("SR") && c != "SR;"), "{written:?}");
    }

    #[test]
    fn a_rejection_is_reported_once() {
        let mut q = radio();
        assert!(parse_str(&mut q, "?;").is_empty());
        assert!(q.refused());
        assert!(!q.refused());
    }

    #[test]
    fn replies_split_across_reads_are_reassembled() {
        let mut q = radio();
        assert!(parse_str(&mut q, "FA000070").is_empty());
        assert_eq!(parse_str(&mut q, "30000;"), vec![CatUpdate::Freq(7_030_000.0)]);
    }

    #[test]
    fn cw_is_streamed_to_the_radios_own_keyer() {
        let mut q = radio();
        assert_eq!(frames(q.send_cw("cq de w1aw")), vec!["KY CQ DE W1AW;"]);
        // The space after `KY` is the documented parameter, not decoration.
        assert!(frames(q.send_cw("test"))[0].starts_with("KY "));
        assert_eq!(frames(q.abort_cw()), vec!["RX;"]);
    }

    #[test]
    fn keyer_text_keeps_only_what_the_radio_can_key() {
        // Upper-cased, runs of spaces collapsed, edges trimmed.
        assert_eq!(keyer_text("  r r  tu  "), "R R TU");
        // The bracketed prosigns become the characters this radio keys them as.
        assert_eq!(keyer_text("tu <sk>"), "TU >");
        assert_eq!(keyer_text("<bt> <ar> <as> <hh> <bk> <sn>"), "[ _ < # \\ %");
        // `<KN>` has no character in the manual's table — the line is missing
        // it — so it goes as its letters rather than as a guess.
        assert_eq!(keyer_text("w1aw <kn>"), "W1AW KN");
        // A semicolon would end the frame early; it never reaches the wire.
        assert_eq!(keyer_text("de;w1aw"), "DEW1AW");
        // A bare closer is dropped rather than keyed as SK.
        assert_eq!(keyer_text("a>b"), "AB");
        // A line break is a word break — dropping it would send one word.
        assert_eq!(keyer_text("tnx fer call\nur 599"), "TNX FER CALL UR 599");
        assert_eq!(keyer_text("w1aw/p ur 599 ok?"), "W1AW/P UR 599 OK?");
        // Nothing sendable produces no frame at all rather than an empty `KY`.
        assert!(keyer_text("   ").is_empty());
        assert!(radio().send_cw("~~~").is_empty());
        // Longer than the shorter of the two buffer formats is truncated.
        assert_eq!(keyer_text(&"a".repeat(80)).len(), CW_MAX);
    }

    /// ⚠️ Zero words per minute is not a slow keyer on this radio, it is
    /// Straight Key mode — which also takes the paddle away from the operator.
    #[test]
    fn the_keyer_speed_can_never_reach_the_straight_key_setting() {
        let mut q = radio();
        let frame = |q: &mut QrpLabs, wpm: f32| String::from_utf8(q.set_cw_wpm(wpm)[0].clone());
        assert_eq!(frame(&mut q, 22.0).unwrap(), "KS022;");
        assert_eq!(frame(&mut q, 0.0).unwrap(), "KS005;");
        assert_eq!(frame(&mut q, -5.0).unwrap(), "KS005;");
        assert_eq!(frame(&mut q, 999.0).unwrap(), "KS060;");
    }

    #[test]
    fn the_receive_filter_is_left_to_the_radio() {
        let mut q = radio();
        // `FW;` reads what the mode implies and there is nothing to write it
        // with, so nothing is written.
        assert!(!q.commands_filter());
        assert!(q.set_filter(Mode::Cw, 400.0, 1000.0).is_empty());
    }
}
