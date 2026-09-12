//! Yaesu "new CAT" (ASCII, `;`-terminated). Frequency `FA<n digits>;`, mode
//! `MD0<x>;`, PTT `TX1;`/`TX0;`, CW from the rig's own keyer (`KM`/`KY`).
//!
//! The one thing that is not the same across the family is how wide the
//! frequency field is: eight digits on the FT-450/950/2000 and FTDX1200/3000/
//! 5000 generation, nine on the FT-891/991/991A, FTDX10/101 and FT-710. A set
//! written at the wrong width is not rounded or clamped — the rig rejects the
//! whole command with `?;` and stays where it was, so a mismatch presents as a
//! radio that answers every poll and ignores every retune. Rather than ask the
//! operator which generation they own, [`Yaesu`] reads the width off the rig:
//! the answer to `FA;` carries the model's native digit count, so the first
//! poll reply settles it (this is what Hamlib derives from the `IF;` reply
//! length). Until one arrives the newer nine is assumed, and a set written
//! before the rig corrected us is re-issued — see [`Protocol::reframed`].

use crate::{CatUpdate, Protocol};
use sdroxide_types::Mode;
use tracing::{debug, info, warn};

/// Frequency-field width assumed before the rig has answered a poll. The newer
/// generation, which is also the one still being sold.
const DEFAULT_WIDTH: usize = 9;

/// Keyer memory the CW text is written to before it is played. Yaesu has no
/// streaming keying command — the only way to key text is to store it and
/// trigger playback — so sending CW *overwrites whatever the operator had in
/// this memory*. Channel 1 is Hamlib's choice too, so the two agree about which
/// memory is the scratch one.
const CW_MEM: u8 = 1;

/// Longest message the rig's keyer memory holds.
const CW_MAX: usize = 50;

/// `SM0` reading (0..255) → dB relative to S9.
///
/// The family's own scale, and not a linear one: Yaesu spreads S0..S9 over the
/// bottom half of the range and the 60 dB above S9 over the top half. The
/// points are the FT-991 measurements Hamlib carries as `yaesu_default_str_cal`
/// (from W6HN), which it applies to the whole current generation — FTDX1200,
/// FTDX3000, FTDX5000, FT-891, FT-991, FTDX101D/MP and FTDX10 alike.
const STR_CAL: &[(f32, f32)] = &[
    (0.0, -54.0),  // S0
    (26.0, -42.0), // S2
    (51.0, -30.0), // S4
    (81.0, -18.0), // S6
    (105.0, -9.0), // S7.5
    (130.0, 0.0),  // S9
    (157.0, 12.0), // S9+12
    (186.0, 25.0), // S9+25
    (203.0, 35.0), // S9+35
    (237.0, 50.0), // S9+50
    (255.0, 60.0), // S9+60
];

/// `RM6` reading (0..255) → SWR.
///
/// Hamlib's `yaesu_default_swr_cal`, measured on an FT-991 by M7OTP. Note where
/// it starts: a reading of 12 is already 1.0:1, so the bottom of the scale is a
/// dead zone rather than a perfect match — which is why this is clamped at both
/// ends rather than extrapolated.
const SWR_CAL: &[(f32, f32)] = &[(12.0, 1.0), (39.0, 1.35), (65.0, 1.5), (89.0, 2.0), (242.0, 5.0)];

/// `SH` index → filter width in Hz, for the FT-891 and FT-991/991A.
///
/// Yaesu has no command that takes a bandwidth: `SH` selects a *position* in a
/// table the rig holds, and that table is a property of the model. Index 0 is
/// the rig's own default for the mode, so it is never selected from here — a
/// width the operator asked for should not silently become whatever the radio
/// prefers.
const FT991_CW_WIDTHS: &[u32] =
    &[0, 50, 100, 150, 200, 250, 300, 350, 400, 450, 500, 800, 1200, 1400, 1700, 2000, 2400, 3000];
const FT991_SSB_WIDTHS: &[u32] = &[
    0, 200, 400, 600, 850, 1100, 1350, 1500, 1650, 1800, 1950, 2100, 2200, 2300, 2400, 2500, 2600,
    2700, 2800, 2900, 3000, 3200,
];

/// The same for the FTDX10, FTDX101D/MP and FT-710 — a longer table, and a
/// different one: index 11 is 600 Hz here and 800 Hz on the pair above.
const FTDX101_CW_WIDTHS: &[u32] = &[
    0, 50, 100, 150, 200, 250, 300, 350, 400, 450, 500, 600, 800, 1200, 1400, 1700, 2000, 2400,
    3000, 3200, 3500, 4000,
];
const FTDX101_SSB_WIDTHS: &[u32] = &[
    0, 300, 400, 600, 850, 1100, 1200, 1500, 1650, 1800, 1950, 2100, 2200, 2300, 2400, 2500, 2600,
    2700, 2800, 2900, 3000, 3200, 3500, 4000,
];

/// Widths above which the rig's NARROW switch has to be off, and below which it
/// has to be on, for the FT-991 generation. The FTDX101 generation has no such
/// preselect.
const FT991_CW_NARROW_MAX: u32 = 500;
const FT991_SSB_NARROW_MAX: u32 = 1800;

/// What a Yaesu's filter tables look like, which is a question about the model.
#[derive(Clone, Copy)]
struct ModelCaps {
    /// The model, for the log.
    name: &'static str,
    /// `SH` tables for CW-like and SSB-like modes.
    cw_widths: &'static [u32],
    ssb_widths: &'static [u32],
    /// Widths at or below which `NA` has to select the narrow position first,
    /// or `None` on a generation that has no such switch.
    narrow_max: Option<(u32, u32)>,
    /// The `KY` parameter that plays back the *stored-text* memory [`CW_MEM`]
    /// was written to. See [`caps_for`] for why this is not a constant.
    cw_play: u8,
}

/// The model behind an `ID;` reply, or `None` for one this file has no tables
/// for — where the rig's filter is then left exactly as the operator set it.
fn caps_for(id: u32) -> Option<ModelCaps> {
    // A Yaesu keyer holds two kinds of memory on the same five channels: the
    // ones recorded from the paddle, played by `KY1`..`KY5`, and the ones
    // *written as text* by `KM`, played by `KY6`..`KYA`. The two are not
    // interchangeable, and playing the wrong one is silent: the rig transmits
    // whatever the operator once recorded on that channel, or nothing at all if
    // they never recorded anything, while sdroxide's panel colours the text as
    // sent.
    //
    // The FT-710 is the single model of the family that inverts this and plays
    // its stored text from `KY1`..`KY5`. Nothing on the wire distinguishes it,
    // which is why the model is asked for.
    let (ft991, ftdx101) = (
        ModelCaps {
            name: "FT-991",
            cw_widths: FT991_CW_WIDTHS,
            ssb_widths: FT991_SSB_WIDTHS,
            narrow_max: Some((FT991_CW_NARROW_MAX, FT991_SSB_NARROW_MAX)),
            cw_play: CW_MEM + 5,
        },
        ModelCaps {
            name: "FTDX101",
            cw_widths: FTDX101_CW_WIDTHS,
            ssb_widths: FTDX101_SSB_WIDTHS,
            narrow_max: None,
            cw_play: CW_MEM + 5,
        },
    );
    Some(match id {
        135 => ModelCaps { name: "FT-891", ..ft991 },
        570 => ModelCaps { name: "FT-991", ..ft991 },
        670 => ModelCaps { name: "FT-991A", ..ft991 },
        761 => ModelCaps { name: "FTDX10", ..ftdx101 },
        681 => ModelCaps { name: "FTDX101D", ..ftdx101 },
        682 => ModelCaps { name: "FTDX101MP", ..ftdx101 },
        800 => ModelCaps { name: "FT-710", cw_play: CW_MEM, ..ftdx101 },
        _ => return None,
    })
}

/// What is assumed before `ID;` has been answered: no filter tables at all, and
/// the keyer playback range every model of the family but one uses.
const UNKNOWN_CAPS: ModelCaps = ModelCaps {
    name: "unidentified",
    cw_widths: &[],
    ssb_widths: &[],
    narrow_max: None,
    cw_play: CW_MEM + 5,
};

pub struct Yaesu {
    buf: String,
    /// What this rig's filter tables and keyer look like, learned from `ID;`.
    caps: ModelCaps,
    /// True once `ID;` has been answered, so the model is only logged once.
    model_known: bool,
    /// Digits in the `FA`/`FB` frequency field on this particular rig.
    width: usize,
    /// True once the rig has told us its width, so a later reply that disagrees
    /// (there should be none) doesn't keep re-triggering a retune.
    width_known: bool,
    /// Set when [`Self::width`] changed under a frame we had already written.
    reframed: bool,
}

impl Yaesu {
    pub fn new() -> Self {
        Yaesu {
            buf: String::new(),
            caps: UNKNOWN_CAPS,
            model_known: false,
            width: DEFAULT_WIDTH,
            width_known: false,
            reframed: false,
        }
    }

    /// Adopt the tables of the model behind an `ID;` reply.
    ///
    /// An unrecognised number is worth saying out loud: it leaves the rig's own
    /// filter under the operator's hand for the rest of the session, and the
    /// number in the log is the one thing that would let the model be added.
    fn learn_model(&mut self, id: u32) {
        if self.model_known {
            return;
        }
        self.model_known = true;
        match caps_for(id) {
            Some(c) => {
                info!(model = c.name, id, "Yaesu CAT: rig identified");
                self.caps = c;
            }
            None => warn!(
                "Yaesu CAT: rig reports model ID {id:04}, which sdroxide has no filter table for \
                 — the width control will not reach the radio"
            ),
        }
    }

    /// The rig's own RIT, XIT and split, switched off. sdroxide carries all
    /// three on the dial, so anything the radio is still holding would add to
    /// ours unseen.
    fn offsets(&self) -> Vec<Vec<u8>> {
        // `FT2` rather than the `ST0` other families use: this generation has
        // no `ST` command, and a split the rig was still holding would transmit
        // us a VFO away.
        vec![b"RC;".to_vec(), b"RT0;".to_vec(), b"XT0;".to_vec(), b"FT2;".to_vec()]
    }

    /// The `SH` table this mode selects from, or an empty one where the model
    /// is unknown or the mode has no table.
    fn widths(&self, mode: Mode) -> (&'static [u32], Option<u32>) {
        let narrow = self.caps.narrow_max;
        match mode {
            Mode::Cw => (self.caps.cw_widths, narrow.map(|(cw, _)| cw)),
            // FM and AM select a roofing filter rather than a DSP width on this
            // family, which is a different command and a different table.
            Mode::Nfm
            | Mode::Wfm
            | Mode::Am
            | Mode::Sam
            | Mode::Dsb
            | Mode::Rifp
            | Mode::Packet
            | Mode::Aprs
            | Mode::SstvFm => (&[], None),
            _ => (self.caps.ssb_widths, narrow.map(|(_, ssb)| ssb)),
        }
    }

    /// Adopt the frequency-field width the rig just demonstrated.
    fn learn_width(&mut self, digits: usize) {
        // Only the two widths the family actually uses; anything else is a
        // malformed reply and must not redefine how we address the rig.
        if !(8..=9).contains(&digits) || (self.width_known && digits == self.width) {
            return;
        }
        if digits != self.width {
            info!(
                from = self.width,
                to = digits,
                "Yaesu CAT: rig uses a {digits}-digit frequency field"
            );
            self.width = digits;
            // Whatever we set before this was written at the wrong width, so the
            // rig rejected it and is still on its old frequency.
            self.reframed = true;
        }
        self.width_known = true;
    }
}

impl Default for Yaesu {
    fn default() -> Self {
        Yaesu::new()
    }
}

fn mode_digit(m: Mode) -> char {
    // Yaesu MD map (FT-891/991 family): 1=LSB 2=USB 3=CW 4=FM 5=AM
    // 6=RTTY-L 7=CW-R 8=DATA-L 9=RTTY-U A=DATA-FM B=FM-N C=DATA-U
    match m {
        Mode::Lsb => '1',
        Mode::Cw => '3',
        // No rig has an ADS-B mode and none ever will: the dial is at
        // 1090 MHz. Grouped with FM so nothing downstream has to special-case
        // a mode a radio can neither be put into nor report back.
        Mode::Nfm | Mode::Wfm | Mode::Adsb | Mode::Vdl2 | Mode::Ais => '4',
        // RIFP keys the carrier itself and VHF packet frequency-modulates it:
        // data over FM (DATA-FM), not over a sideband.
        Mode::Rifp | Mode::Packet | Mode::Aprs | Mode::SstvFm | Mode::RttyFm => 'A',
        Mode::Am | Mode::Sam | Mode::Dsb | Mode::Isb | Mode::Drm => '5',
        Mode::Digl => '8',
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
        | Mode::Rade => 'C',
        Mode::Usb | Mode::Spec | Mode::Sstv | Mode::Wefax | Mode::Navtex | Mode::RfPaint => '2',
    }
}

/// A mode digit the rig reported → the app's mode.
///
/// Deliberately not the inverse of [`mode_digit`] over its whole range. Only
/// the digits that mean one thing on both sides are followed; the rig's
/// RTTY-L/RTTY-U/DATA-FM positions map onto app modes that would be commanded
/// back as a *different* digit, and the two would then take turns correcting
/// each other. `None` leaves the app's mode alone, which is the right answer
/// for a rig position sdroxide has no equivalent of.
fn mode_from_digit(d: char) -> Option<Mode> {
    Some(match d.to_ascii_uppercase() {
        '1' => Mode::Lsb,
        '2' => Mode::Usb,
        '3' | '7' => Mode::Cw, // CW and CW-R are both CW to the app
        '4' | 'B' => Mode::Nfm,
        '5' => Mode::Am,
        '8' => Mode::Digl,
        'C' => Mode::Digu,
        _ => return None,
    })
}

/// The numeric part of a meter reply — the digits after the command letters and
/// the selector — as a 0..255 reading, or `None` when it is not one.
///
/// Three digits is the documented width, but an FTDX101 answers `RM` with six:
/// the reading, then three more the reference does not describe. Hamlib
/// truncates to the first three and so does this. Anything shorter, or with a
/// non-digit in it, is not a reading at all — a mangled reply must leave the
/// meter where it was rather than slam it to zero.
fn reading(digits: &str) -> Option<f32> {
    if digits.len() < 3 {
        return None;
    }
    digits[..3].parse::<u32>().ok().map(|v| v as f32)
}

/// Reduce `text` to what a Yaesu keyer memory will accept: upper case, the
/// letters, digits and punctuation the rig can key, and nothing longer than the
/// memory holds. A character the rig would refuse is dropped rather than sent,
/// because `KM` is all-or-nothing — one bad byte and the rig rejects the whole
/// message and sends none of it.
fn keyer_text(text: &str) -> String {
    text.trim()
        .chars()
        // A line break is a word break, not nothing: dropping it would run the
        // end of one line into the start of the next and send them as one word.
        .map(|c| if c.is_ascii_whitespace() { ' ' } else { c.to_ascii_uppercase() })
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '/' | '?' | '.' | ','))
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

impl Protocol for Yaesu {
    fn set_freq(&mut self, hz: f64) -> Vec<u8> {
        let hz = hz.round().max(0.0) as u64;
        format!("FA{hz:0width$};", width = self.width).into_bytes()
    }
    fn set_mode(&mut self, m: Mode) -> Vec<u8> {
        format!("MD0{};", mode_digit(m)).into_bytes()
    }
    fn ptt(&self, on: bool) -> Vec<u8> {
        if on { b"TX1;".to_vec() } else { b"TX0;".to_vec() }
    }
    fn poll_requests(&self) -> Vec<Vec<u8>> {
        vec![b"FA;".to_vec(), b"MD0;".to_vec()]
    }
    fn dial_requests(&self) -> Vec<Vec<u8>> {
        vec![b"FA;".to_vec()]
    }

    /// `TX;` read back. The reply's digit distinguishes *who* keyed the rig —
    /// its own PTT/mic from a CAT key-down — but this is only ever asked while
    /// sdroxide is not keying, so anything non-zero is the operator's hand and
    /// the distinction is not relied on. Which numeral means which is the one
    /// part of this the manuals state inconsistently.
    fn tx_state_requests(&self) -> Vec<Vec<u8>> {
        vec![b"TX;".to_vec()]
    }

    fn tx_telemetry_requests(&self) -> Vec<Vec<u8>> {
        // `RM` reads one of the transmit meters, chosen by the digit: 6 is SWR
        // on every model of this generation. It is a read, not a meter
        // selection — the rig's own display is left showing whatever the
        // operator put it on.
        //
        // The FTDX3000 and FTDX5000 are the exception Hamlib special-cases:
        // with their tuner in line they answer `RM6` only when the front panel
        // is already on SWR. There is nothing to detect that with here, so
        // those two report a meter that reads zero rather than a wrong one.
        vec![b"RM6;".to_vec()]
    }

    fn rx_telemetry_requests(&self) -> Vec<Vec<u8>> {
        // `SM0` — the main receiver's S-meter. `SM1` is the sub, which sdroxide
        // does not receive on.
        vec![b"SM0;".to_vec()]
    }
    fn clear_offsets(&self) -> Vec<Vec<u8>> {
        // Which Yaesu this is, first: the filter tables and the keyer's
        // playback range both hang off it, and neither can do anything until
        // it has been answered.
        let mut frames = vec![b"ID;".to_vec()];
        frames.extend(self.offsets());
        frames
    }

    fn cw_chunk_len(&self) -> usize {
        CW_MAX
    }

    fn send_cw(&mut self, text: &str) -> Vec<Vec<u8>> {
        let msg = keyer_text(text);
        if msg.is_empty() {
            return Vec::new();
        }
        vec![
            // Break-in has to be on or the keyer runs into the sidetone and
            // never keys the transmitter — the exact "CW does nothing" this
            // path exists to fix. Idempotent, so it rides with every chunk.
            b"BI1;".to_vec(),
            format!("KM{CW_MEM}{msg};").into_bytes(),
            // Which parameter plays back what `KM` just wrote is a question
            // about the model, not the family — see `caps_for`.
            format!("KY{};", self.caps.cw_play).into_bytes(),
        ]
    }

    fn set_cw_wpm(&mut self, wpm: f32) -> Vec<Vec<u8>> {
        // The rig keys at its own speed, so the panel's WPM has to be its WPM.
        let wpm = wpm.round().clamp(4.0, 60.0) as u32;
        vec![format!("KS{wpm:03};").into_bytes()]
    }

    fn set_filter(&mut self, mode: Mode, lo_hz: f32, hi_hz: f32) -> Vec<Vec<u8>> {
        let (table, narrow_max) = self.widths(mode);
        // No table, no command. Guessing an index against the wrong
        // generation's table is how a request for 2.4 kHz becomes 500 Hz.
        if table.is_empty() {
            return Vec::new();
        }
        let want = (hi_hz - lo_hz).abs().round().max(0.0) as u32;
        // The narrowest entry that is still at least as wide as asked — index 0
        // skipped, because it is the rig's own default rather than a width.
        let idx = table
            .iter()
            .enumerate()
            .skip(1)
            .find(|&(_, &w)| w >= want)
            .map(|(i, _)| i)
            .unwrap_or(table.len() - 1);
        let mut frames = Vec::new();
        // On the FT-991 generation the NARROW switch has to be in the right
        // position before the width is selected, or the rig quantises the
        // request into the other half of its table.
        if let Some(max) = narrow_max {
            frames.push(format!("NA0{};", u8::from(table[idx] <= max)).into_bytes());
        }
        frames.push(format!("SH0{idx:02};").into_bytes());
        frames
    }

    fn commands_filter(&self) -> bool {
        true
    }

    fn set_power(&mut self, frac: f32) -> Vec<Vec<u8>> {
        vec![crate::pc_set_frame(frac)]
    }
    fn read_power(&self) -> Vec<Vec<u8>> {
        vec![crate::pc_read_frame()]
    }
    fn commands_power(&self) -> bool {
        true
    }

    fn reframed(&mut self) -> bool {
        std::mem::take(&mut self.reframed)
    }

    fn parse(&mut self, buf: &mut Vec<u8>) -> Vec<CatUpdate> {
        // Accumulate ASCII, split on ';'.
        self.buf.push_str(&String::from_utf8_lossy(buf));
        buf.clear();
        let mut out = Vec::new();
        while let Some(idx) = self.buf.find(';') {
            let msg: String = self.buf.drain(..=idx).collect();
            let msg = msg.trim_end_matches(';').trim();
            if let Some(rest) = msg.strip_prefix("FA") {
                if !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()) {
                    // The reply's own width is the rig telling us how wide its
                    // frequency field is; learn it before anything else so the
                    // next set is addressed the way this rig expects.
                    self.learn_width(rest.len());
                    if let Ok(hz) = rest.parse::<u64>() {
                        out.push(CatUpdate::Freq(hz as f64));
                    }
                }
            } else if let Some(rest) = msg.strip_prefix("PC") {
                if let Some(frac) = crate::pc_parse(rest) {
                    out.push(CatUpdate::Power(frac));
                }
            } else if let Some(rest) = msg.strip_prefix("SM") {
                // `SM<vfo><3 digits>`. Only the main receiver's is asked for,
                // and only that one is followed: a sub-receiver reading would
                // move the meter to a signal nobody is listening to.
                if let Some(raw) = rest.strip_prefix('0').and_then(reading) {
                    out.push(CatUpdate::Signal(crate::S9_DBM + crate::interp(STR_CAL, raw)));
                }
            } else if let Some(rest) = msg.strip_prefix("RM") {
                // `RM<meter><3 digits>` — and only meter 6, the SWR, is ours.
                // The other meters answer on the same command, so the digit is
                // the only thing that says which reading this is.
                if let Some(raw) = rest.strip_prefix('6').and_then(reading) {
                    out.push(CatUpdate::Swr(crate::interp(SWR_CAL, raw)));
                }
            } else if let Some(rest) = msg.strip_prefix("ID") {
                if let Ok(id) = rest.trim().parse::<u32>() {
                    self.learn_model(id);
                }
            } else if let Some(rest) = msg.strip_prefix("MD") {
                // `MD<P1><P2>` — P1 selects the VFO (always 0 here), P2 is the
                // mode itself.
                if let Some(m) = rest.chars().nth(1).and_then(mode_from_digit) {
                    out.push(CatUpdate::Mode(m));
                }
            } else if let Some(rest) = msg.strip_prefix("TX") {
                // `TX<0|1|2>`: 0 is receive, and both other values are the
                // transmitter on. See `tx_state_requests` for why the two are
                // not told apart here.
                if let Some(d) = rest.chars().next().filter(char::is_ascii_digit) {
                    out.push(CatUpdate::Ptt(d != '0'));
                }
            } else if msg == "?" {
                // The rig refused the last command. Nothing identifies which,
                // so this can only be a breadcrumb — but it is the difference
                // between "the radio is ignoring me" and silence.
                debug!("Yaesu CAT: rig rejected a command (?)");
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(y: &mut Yaesu, s: &str) -> Vec<CatUpdate> {
        let mut buf = s.as_bytes().to_vec();
        y.parse(&mut buf)
    }

    /// `TX;` read back. Only "is it transmitting" is taken from it: which
    /// non-zero numeral means the mic and which means CAT is stated
    /// inconsistently across the manuals, and this is only ever asked while
    /// sdroxide is not keying, so both mean the operator.
    #[test]
    fn the_transmit_read_treats_every_non_zero_as_on_the_air() {
        let mut y = Yaesu::new();
        assert_eq!(parse_str(&mut y, "TX0;"), vec![CatUpdate::Ptt(false)]);
        assert_eq!(parse_str(&mut y, "TX1;"), vec![CatUpdate::Ptt(true)]);
        assert_eq!(parse_str(&mut y, "TX2;"), vec![CatUpdate::Ptt(true)]);
        // Not a digit: say nothing rather than guess.
        assert!(parse_str(&mut y, "TX;").is_empty());
        assert_eq!(frames(Yaesu::new().tx_state_requests()), vec!["TX;"]);
    }

    #[test]
    fn frequency_width_is_learned_from_the_rigs_own_reply() {
        let mut y = Yaesu::new();
        // Nothing heard yet: the newer nine-digit form.
        assert_eq!(y.set_freq(14_074_000.0), b"FA014074000;".to_vec());
        assert!(!y.reframed());

        // An FTDX1200-generation rig answers `FA;` with eight digits. Every set
        // written so far was rejected, so the caller is told to re-issue.
        let updates = parse_str(&mut y, "FA07055000;");
        assert_eq!(updates, vec![CatUpdate::Freq(7_055_000.0)]);
        assert!(y.reframed(), "a width change invalidates the frames already written");
        assert!(!y.reframed(), "and is reported exactly once");
        assert_eq!(y.set_freq(14_074_000.0), b"FA14074000;".to_vec());

        // Later replies at the width we already learned are not a change.
        parse_str(&mut y, "FA14074000;");
        assert!(!y.reframed());
    }

    #[test]
    fn a_nine_digit_rig_confirms_the_assumed_width_without_a_retune() {
        let mut y = Yaesu::new();
        assert_eq!(parse_str(&mut y, "FA014074000;"), vec![CatUpdate::Freq(14_074_000.0)]);
        assert!(!y.reframed(), "the assumption was right; nothing to re-issue");
        assert_eq!(y.set_freq(7_055_000.0), b"FA007055000;".to_vec());
    }

    #[test]
    fn a_malformed_reply_does_not_redefine_the_frequency_field() {
        let mut y = Yaesu::new();
        // Neither of these is a width the family uses.
        parse_str(&mut y, "FA1407400;"); // 7 digits
        parse_str(&mut y, "FA0140740000;"); // 10
        assert!(!y.reframed());
        assert_eq!(y.set_freq(14_074_000.0), b"FA014074000;".to_vec());
        // Nor is a non-numeric payload a frequency at all.
        assert!(parse_str(&mut y, "FAxxxxxxxxx;").is_empty());
    }

    #[test]
    fn mode_replies_are_followed_only_where_both_sides_agree() {
        let mut y = Yaesu::new();
        assert_eq!(parse_str(&mut y, "MD03;"), vec![CatUpdate::Mode(Mode::Cw)]);
        assert_eq!(parse_str(&mut y, "MD0C;"), vec![CatUpdate::Mode(Mode::Digu)]);
        assert_eq!(parse_str(&mut y, "MD01;"), vec![CatUpdate::Mode(Mode::Lsb)]);
        // RTTY-L/-U and DATA-FM have no round-trip-stable app equivalent.
        assert!(parse_str(&mut y, "MD06;").is_empty());
        assert!(parse_str(&mut y, "MD09;").is_empty());
        // A rejection is not a mode.
        assert!(parse_str(&mut y, "?;").is_empty());
    }

    #[test]
    fn replies_split_across_reads_are_reassembled() {
        let mut y = Yaesu::new();
        assert!(parse_str(&mut y, "FA0140").is_empty());
        assert_eq!(parse_str(&mut y, "74000;"), vec![CatUpdate::Freq(14_074_000.0)]);
    }

    #[test]
    fn cw_text_is_stored_then_played() {
        let mut y = Yaesu::new();
        assert_eq!(frames(y.send_cw("cq de w1aw")), vec!["BI1;", "KM1CQ DE W1AW;", "KY6;"]);
    }

    /// The FT-710 plays its *stored-text* memories from `KY1`..`KY5`, where the
    /// rest of the family plays the paddle-recorded ones. Play the wrong range
    /// and the rig sends whatever the operator once recorded on that channel —
    /// or, far more often, nothing at all — while the panel colours the text as
    /// though it went out.
    #[test]
    fn the_ft_710_plays_its_stored_text_from_the_other_half_of_the_range() {
        let mut y = Yaesu::new();
        parse_str(&mut y, "ID0800;");
        assert_eq!(frames(y.send_cw("test")), vec!["BI1;", "KM1TEST;", "KY1;"]);

        // Every other model of the family, including the FTDX10 it otherwise
        // shares its tables with.
        for id in ["ID0761;", "ID0570;", "ID0135;", "ID0681;"] {
            let mut y = Yaesu::new();
            parse_str(&mut y, id);
            assert_eq!(frames(y.send_cw("test")).last().unwrap(), "KY6;", "{id}");
        }

        // And an unidentified rig uses the range all but one model wants, which
        // is the safer default: it is also what the memory `KM` just wrote is
        // *called* on every rig but the FT-710.
        let mut y = Yaesu::new();
        assert_eq!(frames(y.send_cw("test")).last().unwrap(), "KY6;");
    }

    #[test]
    fn keyer_text_keeps_only_what_the_rig_can_key() {
        // Upper-cased, prosign punctuation the memory has no character for
        // dropped, runs of spaces collapsed, edges trimmed.
        assert_eq!(keyer_text("  r r = tu <sk>  "), "R R TU SK");
        assert_eq!(keyer_text("w1aw/p ur 599 ok?"), "W1AW/P UR 599 OK?");
        // A semicolon would end the frame early; it never reaches the wire.
        assert_eq!(keyer_text("de;w1aw"), "DEW1AW");
        // A line break is a word break — dropping it would send one word.
        assert_eq!(keyer_text("tnx fer call\nur 599"), "TNX FER CALL UR 599");
        // Nothing sendable produces no frames at all rather than an empty
        // memory write.
        assert!(keyer_text("   ").is_empty());
        assert!(Yaesu::new().send_cw("<>").is_empty());
        // Longer than the memory holds is truncated, not refused.
        assert_eq!(keyer_text(&"a".repeat(80)).len(), CW_MAX);
    }

    #[test]
    fn keyer_speed_is_three_digits_within_the_rigs_range() {
        let mut y = Yaesu::new();
        let frame = |y: &mut Yaesu, wpm: f32| String::from_utf8(y.set_cw_wpm(wpm)[0].clone());
        assert_eq!(frame(&mut y, 20.0).unwrap(), "KS020;");
        assert_eq!(frame(&mut y, 4.0).unwrap(), "KS004;");
        // The panel offers speeds the rig's keyer does not go to.
        assert_eq!(frame(&mut y, 80.0).unwrap(), "KS060;");
        assert_eq!(frame(&mut y, 1.0).unwrap(), "KS004;");
    }

    fn frames(v: Vec<Vec<u8>>) -> Vec<String> {
        v.iter().map(|f| String::from_utf8_lossy(f).into_owned()).collect()
    }

    #[test]
    fn the_meters_are_read_from_the_main_receiver_and_the_swr_selector() {
        let y = Yaesu::new();
        assert_eq!(frames(y.rx_telemetry_requests()), vec!["SM0;"]);
        assert_eq!(frames(y.tx_telemetry_requests()), vec!["RM6;"]);
    }

    #[test]
    fn the_s_meter_reads_on_the_familys_own_scale() {
        let mut y = Yaesu::new();
        let dbm = |u: &[CatUpdate]| match u {
            [CatUpdate::Signal(d)] => *d,
            other => panic!("expected one signal reading, got {other:?}"),
        };
        // The published anchors: 130 is S9, and S9 is −73 dBm.
        assert_eq!(dbm(&parse_str(&mut y, "SM0130;")), -73.0);
        assert_eq!(dbm(&parse_str(&mut y, "SM0000;")), -127.0); // S0
        assert_eq!(dbm(&parse_str(&mut y, "SM0255;")), -13.0); // S9+60
        assert_eq!(dbm(&parse_str(&mut y, "SM0051;")), -103.0); // S4
        // Between anchors, interpolated rather than stepped.
        let mid = dbm(&parse_str(&mut y, "SM0143;"));
        assert!((-73.0..-61.0).contains(&mid), "S9+ half way up reads {mid}");

        // The sub receiver's meter is a signal nobody here is listening to.
        assert!(parse_str(&mut y, "SM1130;").is_empty());
        // A mangled reply must leave the meter where it was, not zero it.
        assert!(parse_str(&mut y, "SM0;").is_empty());
        assert!(parse_str(&mut y, "SM0xy;").is_empty());
    }

    #[test]
    fn only_the_swr_meter_moves_the_swr_reading() {
        let mut y = Yaesu::new();
        let swr = |u: &[CatUpdate]| match u {
            [CatUpdate::Swr(v)] => *v,
            other => panic!("expected one SWR reading, got {other:?}"),
        };
        assert_eq!(swr(&parse_str(&mut y, "RM6089;")), 2.0);
        assert_eq!(swr(&parse_str(&mut y, "RM6065;")), 1.5);
        // Below the bottom of the calibration the scale is a dead zone, not a
        // reason to extrapolate under 1:1 — an SWR cannot be less than that.
        assert_eq!(swr(&parse_str(&mut y, "RM6000;")), 1.0);
        assert_eq!(swr(&parse_str(&mut y, "RM6255;")), 5.0);
        // An FTDX101 answers with six digits; the reading is the first three.
        assert_eq!(swr(&parse_str(&mut y, "RM6089000;")), 2.0);

        // The other meters ride the same command and are not the SWR: ALC (4),
        // power out (5), compression (3). Read as one they would show a
        // perfectly matched antenna as a 5:1 fault, or the reverse.
        for other in ['0', '1', '2', '3', '4', '5', '7', '8', '9'] {
            assert!(
                parse_str(&mut y, &format!("RM{other}089;")).is_empty(),
                "meter {other} is not the SWR"
            );
        }
    }

    /// A model this file has no table for keeps its own filter. Guessing an
    /// index against the wrong generation's table is how a request for 2.4 kHz
    /// becomes 500 Hz.
    #[test]
    fn an_unidentified_rigs_filter_is_left_alone() {
        let mut y = Yaesu::new();
        assert!(y.set_filter(Mode::Usb, 200.0, 2600.0).is_empty());
        parse_str(&mut y, "ID0251;"); // FT-2000 — no table here
        assert!(y.set_filter(Mode::Usb, 200.0, 2600.0).is_empty());
        // And the model is asked for before anything that depends on it.
        assert_eq!(frames(y.clear_offsets())[0], "ID;");
    }

    #[test]
    fn the_filter_index_comes_from_this_models_own_table() {
        let mut y = Yaesu::new();
        parse_str(&mut y, "ID0570;"); // FT-991
        let mut d = Yaesu::new();
        parse_str(&mut d, "ID0761;"); // FTDX10

        // 2400 Hz is an entry in both tables, at the same index — and wide
        // enough that the FT-991's NARROW switch has to be off first, while
        // the later generation has no such switch to set.
        assert_eq!(frames(y.set_filter(Mode::Usb, 200.0, 2600.0)), vec!["NA00;", "SH014;"]);
        assert_eq!(frames(d.set_filter(Mode::Usb, 200.0, 2600.0)), vec!["SH014;"]);

        // 1350 Hz is where they part: an entry on the FT-991, absent from the
        // FTDX10, which takes the next one up. The same index would be two
        // different filters, which is why the model has to be asked for.
        assert_eq!(frames(y.set_filter(Mode::Usb, 0.0, 1350.0)), vec!["NA01;", "SH006;"]);
        assert_eq!(frames(d.set_filter(Mode::Usb, 0.0, 1350.0)), vec!["SH007;"]);

        // CW selects from the CW table, and a narrow one turns NARROW on.
        assert_eq!(frames(y.set_filter(Mode::Cw, 600.0, 1000.0)), vec!["NA01;", "SH008;"]);
    }

    #[test]
    fn a_width_is_rounded_up_so_the_passband_is_never_quietly_narrower() {
        let mut y = Yaesu::new();
        parse_str(&mut y, "ID0570;");
        let idx = |y: &mut Yaesu, w: f32| {
            let f = frames(y.set_filter(Mode::Usb, 0.0, w));
            f.last().unwrap().clone()
        };
        // 2400 is in the table exactly.
        assert_eq!(idx(&mut y, 2400.0), "SH014;");
        // 2450 is not, and takes the next one *up* — a filter that is quietly
        // too narrow presents as a signal that simply is not there.
        assert_eq!(idx(&mut y, 2450.0), "SH015;");
        // Wider than the table goes tops out rather than wrapping.
        assert_eq!(idx(&mut y, 9000.0), "SH021;");
        // And index 0 — the rig's own default rather than a width — is never
        // selected, however narrow the request.
        assert_eq!(idx(&mut y, 1.0), "SH001;");
    }

    #[test]
    fn the_modes_with_no_width_table_send_nothing() {
        let mut y = Yaesu::new();
        parse_str(&mut y, "ID0570;");
        // FM and AM select a roofing filter on this family, not a DSP width.
        for m in [Mode::Nfm, Mode::Wfm, Mode::Am, Mode::Packet] {
            assert!(y.set_filter(m, 0.0, 6000.0).is_empty(), "{m:?} has no SH table");
        }
    }

    #[test]
    fn split_is_cleared_with_the_command_this_generation_has() {
        let frames: Vec<String> = Yaesu::new()
            .clear_offsets()
            .iter()
            .map(|f| String::from_utf8_lossy(f).into_owned())
            .collect();
        assert_eq!(frames, vec!["ID;", "RC;", "RT0;", "XT0;", "FT2;"]);
    }
}
