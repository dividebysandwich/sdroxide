//! Elecraft CAT (ASCII, `;`-terminated) — K3, K3S, KX3, KX2, and the K4 in so
//! far as it answers the K3 command set. Frequency `FA<11 digits>;`, mode
//! `MD<x>;`, PTT `TX;`/`RX;`, CW streamed to the rig's own keyer (`KY`).
//!
//! Descended from the same Kenwood dialect as [`crate::kenwood`] and near
//! enough to it that the Kenwood profile *almost* drives an Elecraft — which
//! is the dangerous kind of near. The differences that matter:
//!
//! * **DATA is a mode, not a flag.** `MD6` is DATA and `MD9` is DATA-REV; there
//!   is no `DA` command at all, and no `MD8`. A Kenwood-shaped `MD2;DA1;` puts
//!   an Elecraft in plain USB and then earns a `?;` for the `DA`, so every
//!   digital-mode over would go out on the wrong side of the filter with no
//!   sign that anything was refused.
//! * **`MD6` alone is not enough.** DATA carries a *sub-mode* — `DT0` DATA A,
//!   `DT1` AFSK A, `DT2` FSK D, `DT3` PSK D — and `MD6` restores whichever was
//!   last used on that band. Only DATA A is the sound-card path sdroxide
//!   drives; landing in FSK D or PSK D hands the over to the rig's own modem,
//!   which has nothing to send.
//! * **`TX1;` is not a command.** Elecraft keys with a bare `TX;`. (`TX0;` is
//!   not one either, so the Kenwood `RX;` unkey is the one thing that carries
//!   over unchanged.)
//! * **The power scale is not 100 W.** `PC` is watts, as on the other ASCII
//!   families, but a KX2 tops out at 12 and a KX3 at 15 while a K3 with its PA
//!   reaches 110. Rather than assume, the rig is asked what it is: see
//!   [`Elecraft::learn_options`].
//! * **`?;` is not a syntax error.** On this family it means the rig was busy
//!   or in a limited-access state (transmitting, BSET, VFO REV) — the same
//!   command may well be accepted a moment later. It is logged and nothing
//!   more; see the note above [`Protocol::parse`] for why it must not be read
//!   as a refusal the way CI-V's "NG" is.
//!
//! Two meta-commands are asserted when the port opens, because a rig left in
//! the other setting by whatever ran before us answers in a format this file
//! would misread:
//!
//! * `K20;` — K2 command modes 1 and 3 report DATA and DATA-REV *as LSB and
//!   USB*, so a rig in one of them looks like it never leaves SSB. Mode 2 adds
//!   a fourth digit to `PC`, which would read back as a power ten times too
//!   large.
//! * `K31;` — the extended S-meter scale (0..21 rather than 0..15). Both are
//!   handled, and a `K3;` readback settles which is in force.
//!
//! Written from Elecraft's public *K3S/K3/KX3/KX2 Programmer's Reference*
//! (Rev. G5, Feb. 20 2019). Not yet verified against a rig.

use crate::{CatUpdate, Protocol};
use sdroxide_types::Mode;
use tracing::{debug, info};

/// Digits in the `FA`/`FB` frequency field. Fixed across the family — "where
/// xxxxxxxxxxx is the frequency in Hz" — unlike Yaesu, which has two widths.
const FREQ_DIGITS: usize = 11;

/// Characters the `KY` buffer holds in one go.
const CW_MAX: usize = 24;

/// Non-alphanumeric characters handed to the keyer: a space, the four
/// punctuation marks an exchange actually needs, and the six symbols the rig
/// keys as prosigns.
///
/// An allow-list rather than a deny-list, and deliberately so. Three ASCII
/// characters are *commands* inside a `KY` payload: `<` puts the rig into TX
/// TEST — transmitting into its own load, no RF out — until a `>` returns it,
/// and `@` truncates the message. A `<SK>` typed in the CW panel that reached
/// the wire intact would leave an Elecraft silently off the air for the rest of
/// the session, with the panel still showing text going out.
const KEYER_PUNCTUATION: &str = " ./?,=+%*(!";

/// Watts at the top of the Drive slider before the rig has said what it has.
///
/// The QRP figure, because erring low is the right way to be wrong about a
/// transmitter — and because it is also the *correct* figure for a bare K3
/// (`PC` is 000-012 with no KPA3) and within a hair of the KX3's 15 W. The
/// other direction would ask a KX2 for eight times what it can do.
const QRP_FULL_POWER_W: f32 = 12.0;

/// Watts at the top of the Drive slider once a PA has been found — a KPA3 in a
/// K3/K3S, or a KXPA100 on a KX3/KX2. Both put `PC` on the 000-110 scale.
const PA_FULL_POWER_W: f32 = 110.0;

/// Position of the PA letter in the `OM` option string. The two layouts —
/// `APXSDFfLVR--` for the K3/K3S and `APF---TBXI0n` for the KX3/KX2 — disagree
/// about almost everything else, but both put the ATU first and the PA second.
const OM_PA_INDEX: usize = 1;

pub struct Elecraft {
    buf: String,
    /// Watts the Drive slider's top means on this particular rig, learned from
    /// `OM;` (see [`Self::learn_options`]).
    full_power_w: f32,
    /// True once `OM;` has been answered, so a later reply cannot keep
    /// re-announcing the same radio.
    options_known: bool,
    /// Mode digit from the rig's last `MD;` reply.
    mode_digit: Option<char>,
    /// Data sub-mode from the last `DT;` reply, and only while the rig is in
    /// DATA — "use DT only when the transceiver is in DATA mode; otherwise the
    /// returned value may not be valid", so it is dropped the moment the rig
    /// leaves it rather than kept as an answer about some earlier band.
    data_sub: Option<char>,
    /// Whether K3 extended mode is in force, which is what the S-meter's scale
    /// hangs off. Assumed, because `K31;` goes out when the port opens; a
    /// `K3n;` reply is what actually settles it.
    extended: bool,
}

impl Elecraft {
    pub fn new() -> Self {
        Elecraft {
            buf: String::new(),
            full_power_w: QRP_FULL_POWER_W,
            options_known: false,
            mode_digit: None,
            data_sub: None,
            extended: true,
        }
    }

    /// Adopt what the `OM` reply says this radio is.
    ///
    /// Only one thing is taken from it — whether there is a power amplifier —
    /// but that one thing is the difference between a Drive slider that spans
    /// 12 W and one that spans 110. The product identifier is read too, purely
    /// so the log names the radio the operator is holding.
    fn learn_options(&mut self, opts: &str) {
        let pa = opts.chars().nth(OM_PA_INDEX) == Some('P');
        // The last two characters are the product number on a KX3/KX2 and a
        // pair of reserved dashes on a K3/K3S, so this tells the two layouts
        // apart as well as naming the rig.
        let model = if opts.ends_with("01") {
            "KX2"
        } else if opts.ends_with("02") {
            "KX3"
        } else if opts.chars().nth(9) == Some('R') {
            // The K3S RF board — "the preferred way to identify a K3S".
            "K3S"
        } else {
            "K3"
        };
        let full = if pa { PA_FULL_POWER_W } else { QRP_FULL_POWER_W };
        if !self.options_known || full != self.full_power_w {
            info!(model, pa, full_power_w = full, "Elecraft CAT: rig identified");
        }
        self.full_power_w = full;
        self.options_known = true;
    }

    /// The rig's mode digit and DATA sub-mode combined into the app's mode.
    ///
    /// Deliberately not the inverse of [`Self::mode_frames`] over its whole
    /// range. A rig position that would be commanded back as something else
    /// yields `None` — the two would otherwise take turns correcting each
    /// other. That covers the three DATA sub-modes sdroxide does not drive:
    /// AFSK A tunes the rig's own RTTY path, and FSK D and PSK D key the rig's
    /// internal modems, none of which is the sound card.
    fn effective_mode(&self) -> Option<Mode> {
        Some(match self.mode_digit? {
            '1' => Mode::Lsb,
            '2' => Mode::Usb,
            // CW and CW-REV are both CW to the app.
            '3' | '7' => Mode::Cw,
            '4' => Mode::Nfm,
            '5' => Mode::Am,
            // DATA normal is the upper side, DATA-REV the lower. Neither is a
            // mode on its own until the sub-mode has been heard: `MD6` restores
            // whichever sub-mode that band was last left in, so the digit alone
            // does not say whether the sound card is in the path.
            '6' if self.data_sub == Some('0') => Mode::Digu,
            '9' if self.data_sub == Some('0') => Mode::Digl,
            _ => return None,
        })
    }

    /// `MD` for `m`, followed by the `DT` that pins the DATA sub-mode.
    ///
    /// `MD` goes first: `DT` is only valid once the rig is in DATA. And `DT`
    /// has to go at all — `MD6;` on its own lands in whichever of the four
    /// sub-modes that band was last left in, which on a rig used for RTTY is
    /// the rig's own FSK keyer and not the sound card at all.
    fn mode_frames(&self, m: Mode) -> Vec<u8> {
        let digit = mode_digit(m);
        let mut out = format!("MD{digit};").into_bytes();
        if matches!(digit, '6' | '9') {
            out.extend_from_slice(b"DT0;");
        }
        out
    }
}

impl Default for Elecraft {
    fn default() -> Self {
        Elecraft::new()
    }
}

/// The rig's mode digit for an app mode. Elecraft `MD`: 1=LSB 2=USB 3=CW 4=FM
/// 5=AM 6=DATA 7=CW-REV 9=DATA-REV. There is no 8, and no separate FSK
/// position — the rig's RTTY and PSK live in DATA's sub-modes instead.
fn mode_digit(m: Mode) -> char {
    match m {
        Mode::Lsb => '1',
        Mode::Cw => '3',
        // RIFP keys the carrier itself and VHF packet frequency-modulates it:
        // data over FM. This family has no DATA-FM position — the sub-modes all
        // hang off SSB — so plain FM is as close as the rig gets, and which
        // input it transmits is then the rig's MIC/LINE menu rather than
        // anything CAT can say.
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
        Mode::Usb | Mode::Spec | Mode::Sstv | Mode::Wefax | Mode::Navtex | Mode::RfPaint => '2',
    }
}

/// Map an Elecraft `SM` reading to dBm.
///
/// Two scales, chosen by the K3 command mode: 0-15 normally, 0-21 under `K31`.
/// Both are calibrated the same way — one count per S-unit up to S9, then a
/// straight run to S9+60 — so the two published anchors (S9, and the top of the
/// scale) define the whole curve. S9 is −73 dBm and an S-unit is 6 dB.
///
/// This is the rig's *own* meter, in the units Elecraft calibrated it in — not
/// a level derived from the sound card — so it needs no dBFS→dBm offset on top.
fn dbm_from_smeter(reading: u32, extended: bool) -> f32 {
    // (S9 count, top-of-scale count). Extended: S9=9, S9+60=21. Basic: S9=6,
    // S9+60=15.
    let (s9, top) = if extended { (9.0, 21.0) } else { (6.0, 15.0) };
    let r = (reading as f32).clamp(0.0, top);
    if r <= s9 { -73.0 - (s9 - r) * 6.0 } else { -73.0 + (r - s9) * (60.0 / (top - s9)) }
}

/// Reduce `text` to what the `KY` buffer will accept: the letters, digits and
/// punctuation of [`KEYER_PUNCTUATION`], and nothing longer than the buffer
/// holds.
///
/// The bracketed prosigns the CW panel writes become the single symbols the rig
/// keys them as — `<SK>` is `*` to an Elecraft, not the two letters S and K,
/// and emphatically not the `<` that would put the transmitter into TX TEST.
fn keyer_text(text: &str) -> String {
    let mut s = text.trim().to_ascii_uppercase();
    // The reference's own prosign table: the symbol *is* the prosign.
    for (token, symbol) in
        [("<KN>", "("), ("<AR>", "+"), ("<BT>", "="), ("<AS>", "%"), ("<SK>", "*"), ("<VE>", "!")]
    {
        if s.contains(token) {
            s = s.replace(token, symbol);
        }
    }
    s.chars()
        // A line break is a word break, not nothing: dropping it would run the
        // end of one line into the start of the next and send them as one word.
        .map(|c| if c.is_ascii_whitespace() { ' ' } else { c })
        .filter(|c| c.is_ascii_alphanumeric() || KEYER_PUNCTUATION.contains(*c))
        // Collapse the runs of spaces a trimmed chunk boundary can leave behind.
        .scan(false, |prev_space, c| {
            let space = c == ' ';
            let keep = !(space && *prev_space);
            *prev_space = space;
            Some(keep.then_some(c))
        })
        .flatten()
        .take(CW_MAX)
        .collect()
}

impl Protocol for Elecraft {
    fn set_freq(&mut self, hz: f64) -> Vec<u8> {
        let hz = hz.round().clamp(0.0, 99_999_999_999.0) as u64;
        format!("FA{hz:0FREQ_DIGITS$};").into_bytes()
    }

    fn set_mode(&mut self, m: Mode) -> Vec<u8> {
        self.mode_frames(m)
    }

    fn ptt(&self, on: bool) -> Vec<u8> {
        // No parameter on either: `TX1;` is a Kenwood/Yaesu form this family
        // does not have, and `TX0;` — the Yaesu unkey — is not one either.
        if on { b"TX;".to_vec() } else { b"RX;".to_vec() }
    }

    /// `TQ;` — transmit query, which this family has as a command of its own.
    fn tx_state_requests(&self) -> Vec<Vec<u8>> {
        vec![b"TQ;".to_vec()]
    }

    fn poll_requests(&self) -> Vec<Vec<u8>> {
        let mut reqs = vec![b"FA;".to_vec(), b"MD;".to_vec()];
        // Only in DATA, where the sub-mode is both valid and load-bearing.
        // Everywhere else the reply would be a stale answer about some other
        // band, and the reference says as much.
        if matches!(self.mode_digit, Some('6') | Some('9')) {
            reqs.push(b"DT;".to_vec());
        }
        reqs
    }
    // `DT` is the DATA sub-mode, which is part of the mode and moves with it.
    fn dial_requests(&self) -> Vec<Vec<u8>> {
        vec![b"FA;".to_vec()]
    }

    fn tx_telemetry_requests(&self) -> Vec<Vec<u8>> {
        // `SW` is the one meter on this family that reads a *number* rather
        // than a bargraph, and it is documented to work during transmit, TUNE
        // and ATU tuning — which is every moment we ask.
        vec![b"SW;".to_vec()]
    }

    fn rx_telemetry_requests(&self) -> Vec<Vec<u8>> {
        vec![b"SM;".to_vec()]
    }

    fn clear_offsets(&self) -> Vec<Vec<u8>> {
        vec![
            // Command-mode normalisation first, before anything whose reply we
            // would have to interpret. See the module comment: a rig left in
            // K21/K23 reports DATA as SSB, and one left in K22 reports a power
            // an order of magnitude out.
            b"K20;".to_vec(),
            b"K31;".to_vec(),
            b"K3;".to_vec(),
            // Stop unsolicited status the rig would otherwise push at us — we
            // poll, and a previous program may have left auto-information on.
            b"AI0;".to_vec(),
            // The clarifier, then RIT and XIT off. Unlike Kenwood, `RC` is
            // valid whether or not either is on — "sets RIT/XIT offset to zero,
            // even if RIT and XIT are both turned off" — so the order here is
            // only for tidiness.
            b"RC;".to_vec(),
            b"RT0;".to_vec(),
            b"XT0;".to_vec(),
            // "Any FR SET cancels SPLIT mode", and it is the documented way to
            // do it. Never `FT2;` — Yaesu's split-off is not a value this
            // family's `FT` takes at all.
            b"FR0;".to_vec(),
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
        // One space between `KY` and the text — the documented P1, and the
        // difference between keying and a syntax error.
        //
        // Nothing turns break-in on first, unlike the other two ASCII families.
        // A `KY` message keys the transmitter itself here, the way a recorded
        // message does; `VX` on this family is hit-the-key transmit for the
        // *paddle*, and is read-only on a KX3 or KX2 anyway.
        //
        // Plain `KY`, not the `KYW` "wait" form: that one defers every host
        // command behind the message, which would stall the polls — and the
        // caller already keeps its own clock and does not hand over a second
        // message until the first has had time to go out.
        vec![format!("KY {msg};").into_bytes()]
    }

    fn abort_cw(&mut self) -> Vec<Vec<u8>> {
        // "Terminates transmit in all modes, including message play" — the same
        // command that unkeys, which is the whole of the stop this family has.
        vec![b"RX;".to_vec()]
    }

    fn set_cw_wpm(&mut self, wpm: f32) -> Vec<Vec<u8>> {
        // 008-050. Narrower at both ends than the Kenwood and Yaesu keyers, and
        // narrower than the panel offers.
        let wpm = wpm.round().clamp(8.0, 50.0) as u32;
        vec![format!("KS{wpm:03};").into_bytes()]
    }

    fn set_filter(&mut self, _mode: Mode, lo_hz: f32, hi_hz: f32) -> Vec<Vec<u8>> {
        // `BW` in ten-hertz units, 0000-9999 — the one filter command in any of
        // these families that takes a real bandwidth rather than an index into
        // a per-model table, and the same command in every mode. The rig
        // quantises and range-limits it to what the present mode allows, which
        // is exactly the right place for that to happen.
        //
        // Width only, no `IS`: the passband's centre belongs to the rig. In CW
        // and DATA it follows the operator's PITCH, and moving it from here
        // would drag the sidetone the operator zero-beats against.
        let width = (hi_hz - lo_hz).abs().round().clamp(0.0, 99_990.0);
        vec![format!("BW{:04};", (width / 10.0).round() as u32).into_bytes()]
    }

    fn commands_filter(&self) -> bool {
        true
    }

    fn set_power(&mut self, frac: f32) -> Vec<Vec<u8>> {
        // Watts, against whatever scale `OM` said this radio has. No 5 W floor:
        // Elecraft documents the range as starting at 000, and the rig's own
        // PWR control reaches zero too, so the bottom of the slider means what
        // the bottom of the knob means.
        let w = (frac.clamp(0.0, 1.0) * self.full_power_w).round().clamp(0.0, 999.0) as u32;
        vec![format!("PC{w:03};").into_bytes()]
    }

    fn read_power(&self) -> Vec<Vec<u8>> {
        // `OM` first, and the order is the point: the rig serves one command at
        // a time and answers in turn, so the option string arrives before the
        // power does and the watts in that reply are divided by a scale that is
        // already this radio's.
        vec![b"OM;".to_vec(), b"PC;".to_vec()]
    }

    fn commands_power(&self) -> bool {
        true
    }

    fn mode_moves_dial(&self) -> bool {
        // A K3 whose `CONFIG:CW WGHT` is set to `VFO OFS` shifts its displayed
        // frequency by the CW pitch whenever the mode crosses into or out of
        // CW, so that zero-beat stays zero-beat. Useful at the radio; invisible
        // and wrong over CAT, where an operator who asked for 14.050 would find
        // themselves transmitting six hundred hertz away with nothing on screen
        // to say so.
        //
        // Unconditional, because nothing on the wire says which way that menu
        // entry is set — and because re-asserting a frequency the rig is
        // already on costs one frame and changes nothing. Elecraft's own sample
        // macros send the second `FA` for exactly this reason.
        true
    }

    // Deliberately no `refused()`. On CI-V, "NG" arriving on the heels of a
    // key-down is worth telling the operator about: the rig said no, and a
    // transmitter that did not come up has nothing else to show for it. Here
    // the same reasoning runs backwards. `?;` on this family means *busy*, and
    // the reference's own first example of busy is "such as transmit" — so a
    // `?;` behind a key-down is the rig reporting that it keyed. Reporting it
    // as a refusal would put a warning under every single over.

    fn parse(&mut self, buf: &mut Vec<u8>) -> Vec<CatUpdate> {
        self.buf.push_str(&String::from_utf8_lossy(buf));
        buf.clear();
        let mut out = Vec::new();
        // DATA's mode and its sub-mode are two replies, so a mode is only whole
        // once both are in. Emitting per-reply would report nothing at all for
        // the moment between them, every poll.
        let mut mode_touched = false;
        while let Some(idx) = self.buf.find(';') {
            let msg: String = self.buf.drain(..=idx).collect();
            let msg = msg.trim_end_matches(';').trim();
            if let Some(rest) = msg.strip_prefix("FA") {
                if rest.len() == FREQ_DIGITS
                    && let Ok(hz) = rest.parse::<u64>()
                {
                    out.push(CatUpdate::Freq(hz as f64));
                }
            } else if let Some(rest) = msg.strip_prefix("MD") {
                if let Some(d) = rest.chars().next() {
                    self.mode_digit = Some(d);
                    // A rig that has left DATA has left its sub-mode behind
                    // with it; keeping the last one would answer a question
                    // about a band we are no longer on.
                    if !matches!(d, '6' | '9') {
                        self.data_sub = None;
                    }
                    mode_touched = true;
                }
            } else if let Some(rest) = msg.strip_prefix("DT") {
                if let Some(d) = rest.chars().next() {
                    self.data_sub = Some(d);
                    mode_touched = true;
                }
            } else if let Some(rest) = msg.strip_prefix("TQ") {
                // `TQ0;` receive, `TQ1;` transmit — this family's one-command
                // answer to "are you on the air", and the reason it needs no
                // `IF;` parsing the way Kenwood does.
                if let Some(d) = rest.chars().next().filter(char::is_ascii_digit) {
                    out.push(CatUpdate::Ptt(d != '0'));
                }
            } else if let Some(rest) = msg.strip_prefix("OM") {
                // The leading blank belongs to the response format, not to the
                // option string.
                self.learn_options(rest.trim_start());
            } else if let Some(rest) = msg.strip_prefix("PC") {
                if let Ok(w) = rest.trim().parse::<u32>() {
                    out.push(CatUpdate::Power((w as f32 / self.full_power_w).clamp(0.0, 1.0)));
                }
            } else if let Some(rest) = msg.strip_prefix("SW") {
                // Tenths of a unit, 010-999. Checked for digits because `SWT`
                // and `SWH` share the prefix; neither is ever sent from here,
                // but neither is a reflected one a reason to move the meter.
                if let Ok(tenths) = rest.parse::<u32>() {
                    out.push(CatUpdate::Swr((tenths as f32 / 10.0).max(1.0)));
                }
            } else if let Some(rest) = msg.strip_prefix("SM") {
                // Same care as `SW`: `SMH` — the K3's high-resolution meter,
                // which is not what was asked for — starts with these letters.
                if let Ok(reading) = rest.parse::<u32>() {
                    out.push(CatUpdate::Signal(dbm_from_smeter(reading, self.extended)));
                }
            } else if let Some(rest) = msg.strip_prefix("K3") {
                // The readback of the extended-mode assertion, and the only
                // thing that says which S-meter scale the readings are on.
                if let Some(d) = rest.chars().next() {
                    self.extended = d == '1';
                }
            } else if msg == "?" {
                // Not a syntax error on this family: the rig was busy, or in a
                // limited-access state such as BSET or VFO REV. Nothing
                // identifies which command it could not take, and transmitting
                // is itself one of the busy states — so this can only ever be a
                // breadcrumb in the log, never a diagnosis.
                debug!("Elecraft CAT: rig busy or in a limited-access state (?)");
            }
        }
        if mode_touched && let Some(m) = self.effective_mode() {
            out.push(CatUpdate::Mode(m));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(e: &mut Elecraft, s: &str) -> Vec<CatUpdate> {
        let mut buf = s.as_bytes().to_vec();
        e.parse(&mut buf)
    }

    fn frames(v: Vec<Vec<u8>>) -> Vec<String> {
        v.iter().map(|f| String::from_utf8_lossy(f).into_owned()).collect()
    }

    /// `TQ;` — this family answers "are you on the air" with a command of its
    /// own, so nothing here has to index into a status string.
    #[test]
    fn the_transmit_query_is_a_command_of_its_own() {
        let mut e = Elecraft::new();
        assert_eq!(parse_str(&mut e, "TQ0;"), vec![CatUpdate::Ptt(false)]);
        assert_eq!(parse_str(&mut e, "TQ1;"), vec![CatUpdate::Ptt(true)]);
        assert!(parse_str(&mut e, "TQ;").is_empty());
        assert_eq!(frames(Elecraft::new().tx_state_requests()), vec!["TQ;"]);
    }

    #[test]
    fn frequency_is_eleven_digits() {
        let mut e = Elecraft::new();
        assert_eq!(e.set_freq(14_074_000.0), b"FA00014074000;".to_vec());
        assert_eq!(e.set_freq(7_055_000.0), b"FA00007055000;".to_vec());
        // A reply at that width is a frequency; anything else is not, and must
        // not be read as one at some other scale.
        assert_eq!(parse_str(&mut e, "FA00014060000;"), vec![CatUpdate::Freq(14_060_000.0)]);
        assert!(parse_str(&mut e, "FA014074000;").is_empty()); // Yaesu's nine
        assert!(parse_str(&mut e, "FAxxxxxxxxxxx;").is_empty());
    }

    #[test]
    fn keying_takes_no_parameter_at_either_end() {
        // `TX1;` is the Kenwood/Yaesu key-down and does not exist here; `TX0;`
        // is the Yaesu unkey and does not either.
        let e = Elecraft::new();
        assert_eq!(e.ptt(true), b"TX;".to_vec());
        assert_eq!(e.ptt(false), b"RX;".to_vec());
    }

    #[test]
    fn data_is_a_mode_of_its_own_carrying_a_sub_mode() {
        let mut e = Elecraft::new();
        // `MD` first — `DT` is only valid once the rig is in DATA — and `DT0`
        // every time, because `MD6` alone restores whatever sub-mode that band
        // was last left in.
        assert_eq!(e.set_mode(Mode::Digu), b"MD6;DT0;".to_vec());
        assert_eq!(e.set_mode(Mode::Digl), b"MD9;DT0;".to_vec());
        // Nothing else carries a sub-mode, so nothing else sends one.
        assert_eq!(e.set_mode(Mode::Usb), b"MD2;".to_vec());
        assert_eq!(e.set_mode(Mode::Lsb), b"MD1;".to_vec());
        assert_eq!(e.set_mode(Mode::Cw), b"MD3;".to_vec());
        assert_eq!(e.set_mode(Mode::Am), b"MD5;".to_vec());
        assert_eq!(e.set_mode(Mode::Nfm), b"MD4;".to_vec());
        // No `DA` anywhere: that is the Kenwood flag, and an Elecraft would
        // refuse it after having already been put in the wrong mode.
        for m in [Mode::Digu, Mode::Digl, Mode::Usb, Mode::Cw] {
            let sent = String::from_utf8(e.set_mode(m)).unwrap();
            assert!(!sent.contains("DA"), "{m:?} sent the Kenwood DATA flag: {sent}");
        }
    }

    #[test]
    fn a_mode_is_only_whole_once_the_sub_mode_is_in() {
        let mut e = Elecraft::new();
        // The digit on its own says DATA but not *which* DATA, so nothing is
        // reported yet — the sound card may or may not be in the path.
        assert!(parse_str(&mut e, "MD6;").is_empty());
        assert_eq!(parse_str(&mut e, "DT0;"), vec![CatUpdate::Mode(Mode::Digu)]);
        // Arriving together, as an ordinary poll delivers them, it is reported
        // once and whole.
        assert_eq!(parse_str(&mut e, "MD9;DT0;"), vec![CatUpdate::Mode(Mode::Digl)]);
        assert_eq!(parse_str(&mut e, "MD2;"), vec![CatUpdate::Mode(Mode::Usb)]);
        // CW-REV is CW to the app.
        assert_eq!(parse_str(&mut e, "MD7;"), vec![CatUpdate::Mode(Mode::Cw)]);
    }

    #[test]
    fn only_the_sound_card_data_sub_mode_is_followed() {
        let mut e = Elecraft::new();
        // AFSK A drives the rig's own RTTY path; FSK D and PSK D key its
        // internal modems. All three would be commanded back as `DT0`, and the
        // two sides would spend the session correcting each other.
        for sub in ['1', '2', '3'] {
            let mut e = Elecraft::new();
            assert!(
                parse_str(&mut e, &format!("MD6;DT{sub};")).is_empty(),
                "DT{sub} is not a mode sdroxide drives"
            );
        }
        // And a rig that leaves DATA leaves its sub-mode behind: the stale
        // answer must not make the next `MD6` look resolved.
        assert_eq!(parse_str(&mut e, "MD6;DT0;"), vec![CatUpdate::Mode(Mode::Digu)]);
        assert_eq!(parse_str(&mut e, "MD3;"), vec![CatUpdate::Mode(Mode::Cw)]);
        assert!(parse_str(&mut e, "MD6;").is_empty());
    }

    #[test]
    fn rig_positions_without_a_stable_round_trip_are_left_alone() {
        let mut e = Elecraft::new();
        // 8 is not a mode on this family at all.
        assert!(parse_str(&mut e, "MD8;").is_empty());
        assert!(parse_str(&mut e, "MD0;").is_empty());
    }

    #[test]
    fn every_app_mode_the_rig_can_hold_survives_a_round_trip() {
        // What we command has to be what we read back, or the app and the rig
        // spend the session correcting each other.
        for m in [Mode::Lsb, Mode::Usb, Mode::Cw, Mode::Nfm, Mode::Am, Mode::Digl, Mode::Digu] {
            let mut e = Elecraft::new();
            let sent = String::from_utf8(e.set_mode(m)).unwrap();
            // Echo the set back as the rig's replies would read: the mode, and
            // the sub-mode where there is one.
            let digit = mode_digit(m);
            let reply = if matches!(digit, '6' | '9') {
                format!("MD{digit};DT0;")
            } else {
                format!("MD{digit};")
            };
            assert_eq!(
                parse_str(&mut e, &reply),
                vec![CatUpdate::Mode(m)],
                "{m:?} was set with {sent} and read back as something else"
            );
        }
    }

    #[test]
    fn the_data_sub_mode_is_only_asked_for_in_data_mode() {
        let mut e = Elecraft::new();
        // Nothing known yet: the reply would be about some other band.
        assert_eq!(frames(e.poll_requests()), vec!["FA;", "MD;"]);
        parse_str(&mut e, "MD6;DT0;");
        assert_eq!(frames(e.poll_requests()), vec!["FA;", "MD;", "DT;"]);
        parse_str(&mut e, "MD9;DT0;");
        assert!(frames(e.poll_requests()).contains(&"DT;".to_string()));
        // Out of DATA, and the question stops being asked.
        parse_str(&mut e, "MD2;");
        assert_eq!(frames(e.poll_requests()), vec!["FA;", "MD;"]);
    }

    #[test]
    fn the_command_modes_are_normalised_before_anything_is_read() {
        let f = frames(Elecraft::new().clear_offsets());
        assert_eq!(f, vec!["K20;", "K31;", "K3;", "AI0;", "RC;", "RT0;", "XT0;", "FR0;"]);
        // K2 mode goes out before anything whose reply we interpret: modes 1
        // and 3 report DATA as SSB, and mode 2 adds a digit to `PC`.
        assert_eq!(f[0], "K20;");
        // Split off with the command this family has — never Yaesu's `FT2;`.
        assert!(!f.iter().any(|c| c.starts_with("FT")));
    }

    #[test]
    fn the_power_scale_is_learned_from_the_option_module_query() {
        // `OM` is asked before `PC`, so the watts are always divided by a scale
        // that already belongs to this radio.
        assert_eq!(frames(Elecraft::new().read_power()), vec!["OM;", "PC;"]);

        // A K3 with its KPA3 fitted: 110 W at the top of the slider.
        let mut e = Elecraft::new();
        parse_str(&mut e, "OM AP-S--------;");
        assert_eq!(parse_str(&mut e, "PC110;"), vec![CatUpdate::Power(1.0)]);
        assert_eq!(frames(e.set_power(1.0)), vec!["PC110;"]);
        assert_eq!(frames(e.set_power(0.5)), vec!["PC055;"]);

        // A KX3 with no KXPA100 — the same slider is twelve watts wide, and
        // asking it for a hundred would be asking for eight times what it has.
        let mut e = Elecraft::new();
        parse_str(&mut e, "OM A-F-------02;");
        assert_eq!(frames(e.set_power(1.0)), vec!["PC012;"]);
        assert_eq!(parse_str(&mut e, "PC010;"), vec![CatUpdate::Power(10.0 / 12.0)]);

        // A KX2 with a KXPA100 on the other end of the control cable is back on
        // the 110 W scale.
        let mut e = Elecraft::new();
        parse_str(&mut e, "OM AP-----TBXI01;");
        assert_eq!(frames(e.set_power(1.0)), vec!["PC110;"]);
    }

    #[test]
    fn before_the_rig_answers_the_scale_errs_low() {
        // Nothing heard yet. A slider at the top asks a K3 for twelve watts —
        // wrong, but wrong in the direction that cannot hurt an amplifier or a
        // licence.
        let mut e = Elecraft::new();
        assert_eq!(frames(e.set_power(1.0)), vec!["PC012;"]);
        // And the floor is zero, not the 5 W the Yaesu/Kenwood `PC` documents:
        // Elecraft's own range starts at 000.
        assert_eq!(frames(e.set_power(0.0)), vec!["PC000;"]);
    }

    #[test]
    fn the_s_meter_reads_on_whichever_scale_is_in_force() {
        // Extended (`K31`, which is what we assert): 0-21, S9 at 9.
        let mut e = Elecraft::new();
        let dbm = |u: &[CatUpdate]| match u {
            [CatUpdate::Signal(d)] => *d,
            other => panic!("expected one signal reading, got {other:?}"),
        };
        assert_eq!(dbm(&parse_str(&mut e, "SM0009;")), -73.0); // S9
        assert_eq!(dbm(&parse_str(&mut e, "SM0013;")), -53.0); // S9+20
        assert_eq!(dbm(&parse_str(&mut e, "SM0021;")), -13.0); // S9+60
        assert_eq!(dbm(&parse_str(&mut e, "SM0003;")), -109.0); // S3

        // A rig that would not go extended reads on the 0-15 scale, where the
        // same count means twenty decibels more signal.
        parse_str(&mut e, "K30;");
        assert_eq!(dbm(&parse_str(&mut e, "SM0006;")), -73.0); // S9
        assert_eq!(dbm(&parse_str(&mut e, "SM0009;")), -53.0); // S9+20
        assert_eq!(dbm(&parse_str(&mut e, "SM0015;")), -13.0); // S9+60

        // `SMH` — the K3's high-resolution meter — shares the prefix and is not
        // what was asked for. Read on this scale it would show S9+60 for a bare
        // S1, so it must not be read at all.
        assert!(parse_str(&mut e, "SMH005;").is_empty());
    }

    #[test]
    fn the_filter_is_a_real_bandwidth_in_ten_hertz_units() {
        let mut e = Elecraft::new();
        // The one filter command in any of these families that takes a width
        // rather than an index into a per-model table — and the same command
        // in every mode.
        assert_eq!(frames(e.set_filter(Mode::Usb, 200.0, 2600.0)), vec!["BW0240;"]);
        assert_eq!(frames(e.set_filter(Mode::Cw, 500.0, 1000.0)), vec!["BW0050;"]);
        assert_eq!(frames(e.set_filter(Mode::Am, 0.0, 6000.0)), vec!["BW0600;"]);
        // LSB arrives with its edges negative — sideband lives in their sign
        // here — and is the same width for all that.
        assert_eq!(frames(e.set_filter(Mode::Lsb, -2600.0, -200.0)), vec!["BW0240;"]);
        // Rounded to the unit the command has, not truncated.
        assert_eq!(frames(e.set_filter(Mode::Usb, 0.0, 2445.0)), vec!["BW0245;"]);
    }

    #[test]
    fn swr_arrives_in_tenths_of_a_unit() {
        let mut e = Elecraft::new();
        assert_eq!(parse_str(&mut e, "SW010;"), vec![CatUpdate::Swr(1.0)]);
        assert_eq!(parse_str(&mut e, "SW015;"), vec![CatUpdate::Swr(1.5)]);
        assert_eq!(parse_str(&mut e, "SW032;"), vec![CatUpdate::Swr(3.2)]);
        assert_eq!(parse_str(&mut e, "SW999;"), vec![CatUpdate::Swr(99.9)]);
        // The switch-emulation commands share the prefix and carry no meter.
        assert!(parse_str(&mut e, "SWT16;").is_empty());
        assert!(parse_str(&mut e, "SWH16;").is_empty());
        assert_eq!(frames(e.tx_telemetry_requests()), vec!["SW;"]);
        assert_eq!(frames(e.rx_telemetry_requests()), vec!["SM;"]);
    }

    /// `?;` is *busy*, not *refused* — and transmitting is one of the busy
    /// states, so one arriving behind a key-down is the rig saying it keyed.
    /// It must not reach the driver's refusal path, which would then warn under
    /// every over.
    #[test]
    fn a_busy_answer_is_not_a_refusal() {
        let mut e = Elecraft::new();
        assert!(parse_str(&mut e, "?;").is_empty());
        assert!(!e.refused());
    }

    #[test]
    fn replies_split_across_reads_are_reassembled() {
        let mut e = Elecraft::new();
        assert!(parse_str(&mut e, "FA000140").is_empty());
        assert_eq!(parse_str(&mut e, "74000;"), vec![CatUpdate::Freq(14_074_000.0)]);
    }

    #[test]
    fn cw_is_streamed_to_the_keyer_buffer() {
        let mut e = Elecraft::new();
        // One frame: no break-in to turn on first (a `KY` message keys the
        // transmitter itself here) and no keyer memory to write through.
        assert_eq!(frames(e.send_cw("cq de w1aw")), vec!["KY CQ DE W1AW;"]);
        // The space after `KY` is the documented parameter, not decoration.
        assert!(frames(e.send_cw("test"))[0].starts_with("KY "));
    }

    #[test]
    fn keyer_text_keeps_only_what_the_rig_can_key() {
        // Upper-cased, runs of spaces collapsed, edges trimmed.
        assert_eq!(keyer_text("  r r  tu  "), "R R TU");
        // The bracketed prosigns become the single symbols the rig keys them as.
        assert_eq!(keyer_text("tu <sk>"), "TU *");
        assert_eq!(keyer_text("<bt> <ar> <as> <kn> <ve>"), "= + % ( !");
        // `=` is BT in the rig's own table, so it goes as is.
        assert_eq!(keyer_text("r r = tu"), "R R = TU");
        assert_eq!(keyer_text("w1aw/p ur 599 ok?"), "W1AW/P UR 599 OK?");
        // A line break is a word break — dropping it would send one word.
        assert_eq!(keyer_text("tnx fer call\nur 599"), "TNX FER CALL UR 599");
        // A semicolon would end the frame early; it never reaches the wire.
        assert_eq!(keyer_text("de;w1aw"), "DEW1AW");
        // The three characters that are *commands* inside a KY payload. A bare
        // `<` would leave the rig in TX TEST — transmitting into its own load,
        // nothing on the air — for the rest of the session, and `@` would
        // silently truncate the message.
        assert_eq!(keyer_text("a<b>c@d"), "ABCD");
        // Nothing sendable produces no frames at all rather than an empty `KY`.
        assert!(keyer_text("   ").is_empty());
        assert!(Elecraft::new().send_cw("~~~").is_empty());
        // Longer than the buffer holds is truncated, not refused.
        assert_eq!(keyer_text(&"a".repeat(80)).len(), CW_MAX);
    }

    #[test]
    fn aborting_uses_the_command_that_stops_a_message_in_progress() {
        assert_eq!(frames(Elecraft::new().abort_cw()), vec!["RX;"]);
    }

    #[test]
    fn keyer_speed_is_three_digits_within_this_familys_range() {
        let mut e = Elecraft::new();
        let frame = |e: &mut Elecraft, wpm: f32| String::from_utf8(e.set_cw_wpm(wpm)[0].clone());
        assert_eq!(frame(&mut e, 20.0).unwrap(), "KS020;");
        // 008-050 — narrower at both ends than the panel offers, and narrower
        // than the other two ASCII families take.
        assert_eq!(frame(&mut e, 80.0).unwrap(), "KS050;");
        assert_eq!(frame(&mut e, 1.0).unwrap(), "KS008;");
    }
}
