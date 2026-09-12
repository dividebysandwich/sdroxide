//! Kenwood CAT (ASCII, `;`-terminated). Frequency `FA<11 digits>;`, mode
//! `MD<x>;`, PTT `TX…;`/`RX;`, CW streamed to the rig's own keyer (`KY`).
//!
//! Close enough to Yaesu's "new CAT" to look like the same protocol and far
//! enough away that pointing the Yaesu family at a Kenwood fails in every
//! direction at once — which is how this file came to exist. The differences
//! that matter:
//!
//! * **Frequency is eleven digits**, not the family-dependent eight or nine.
//!   A Yaesu-width `FA` is a syntax error the rig answers with `?;`, so the
//!   dial never moves.
//! * **`MD` has no VFO parameter.** Yaesu's read (`MD0;`) is a Kenwood *set*
//!   to mode 0, which is the documented "setting failure" value.
//! * **`TX0;` transmits.** Yaesu unkeys with it. A Kenwood told `TX0;` keys up
//!   and stays there, because the command that unkeys it is `RX;`.
//! * **`FT2;`** — Yaesu's "split off" — selects the memory channel as transmit
//!   VFO on a Kenwood, which the reference guide explicitly forbids.
//!
//! DATA mode is a flag beside the mode (`DA`) rather than a mode of its own, so
//! USB and USB-DATA are both `MD2;` and only `DA;` tells them apart.
//!
//! Written from Kenwood's public *PC Control Command Reference Guide* for the
//! TS-590S/TS-590SG (B5A-0316-20/01) and the TS-2000 command tables. Not yet
//! verified against a rig.

use crate::{CatUpdate, Protocol};
use sdroxide_types::{KenwoodSend, Mode};
use tracing::{debug, info, warn};

/// Digits in the `FA`/`FB` frequency field. Fixed across the family, unlike
/// Yaesu — "enter 00014195000 for 14.195 MHz. Blank digits must be entered
/// as 0."
const FREQ_DIGITS: usize = 11;

/// Characters the `KY` buffer holds in one go.
const CW_MAX: usize = 24;

/// Non-alphanumeric characters the keyer accepts: the reference guide's own
/// table, then the six symbols that stand for prosigns. A semicolon is not
/// among them, which is what keeps operator text from ending the frame early.
const KEYER_PUNCTUATION: &str = " '\"()*+,-./:=?@[_<>]\\";

/// What a Kenwood's meters read on and how its receive filter is addressed —
/// both of which are questions about the *model*, not about the family.
///
/// Nothing else in this file needs to know which Kenwood it is talking to. The
/// meters do, and unavoidably: `SM` and `RM` both answer with a count of lit
/// bars on the rig's own display, and how many bars that display has changed
/// twice across the family. A TS-2000's SWR runs to 20, a TS-590's to 30 and a
/// TS-890's to 70, so the same reading of 12 is a 3:1 fault, a 2:1 match, or
/// very nearly flat, depending on a rig this side cannot see.
///
/// Which is why the model is *asked for* (`ID;`) rather than assumed, and why a
/// model with no published curve reports no SWR at all. A meter that reads
/// plausibly and means nothing is worse than an empty one.
#[derive(Clone, Copy)]
struct ModelCaps {
    /// The model, for the log.
    name: &'static str,
    /// `SM` reading → dB relative to S9, or `None` to use [`generic_strength`].
    str_cal: Option<&'static [(f32, f32)]>,
    /// Which `RM` digit carries the SWR on this model, and the curve it reads
    /// on. `None` where no published curve exists.
    swr: Option<(char, &'static [(f32, f32)])>,
    /// Whether the SWR meter has to be switched on before `RM;` will report it.
    /// The TS-480/590/2000 generation answers `RM;` with every meter at once;
    /// the TS-890 and TS-990 report only the ones enabled.
    swr_needs_enable: bool,
    /// Whether the S-meter read carries a receiver digit (`SM0;`). The rigs
    /// that predate a sub receiver have only the bare `SM;`.
    smeter_has_vfo: bool,
    /// The rig's SSB-side slope filter as `(low-cut, high-cut)` index tables,
    /// in Hz. `None` where sdroxide has no table for this model, in which case
    /// its filter is left alone rather than moved to a guessed index.
    slopes: Option<(&'static [u32], &'static [u32])>,
}

// `SL`/`SH` index → Hz, from Hamlib's per-model slope tables. The TS-480 and
// TS-590 agree; the TS-2000's high-cut table starts two steps further up, so
// the same index is a different filter on it — which is exactly the sort of
// thing that has to come from the model rather than the family.
const TS480_SL: &[u32] = &[0, 50, 100, 200, 300, 400, 500, 600, 700, 800, 900, 1000];
const TS480_SH: &[u32] =
    &[1000, 1200, 1400, 1600, 1800, 2000, 2200, 2400, 2600, 2800, 3000, 3400, 4000, 5000];
const TS2000_SL: &[u32] = TS480_SL;
const TS2000_SH: &[u32] = &[1400, 1600, 1800, 2000, 2200, 2400, 2600, 2800, 3000, 3400, 4000, 5000];

/// The index into `table` for a cut at `hz`, erring in the direction that keeps
/// the passband *wider* than asked for.
///
/// `at_least` picks the first entry that reaches `hz` (a high cut: never let
/// less through than the operator asked for); clearing it picks the last entry
/// that does not exceed it (a low cut: never cut more away than they asked).
/// Between them the rig ends up with a filter no narrower than the one on
/// screen, which matters because a filter that is quietly too narrow presents
/// as a signal that is simply not there.
fn slope_index(table: &[u32], hz: f32, at_least: bool) -> u32 {
    let hz = hz.max(0.0).round() as u32;
    let idx = if at_least {
        table.iter().position(|&v| v >= hz).unwrap_or(table.len() - 1)
    } else {
        table.iter().rposition(|&v| v <= hz).unwrap_or(0)
    };
    idx as u32
}

// Bars → dB relative to S9. All from Hamlib's per-model `str_cal` tables.
const TS2000_STR: &[(f32, f32)] = &[
    (0.0, -54.0),
    (3.0, -48.0),
    (6.0, -36.0),
    (9.0, -24.0),
    (12.0, -12.0),
    (15.0, 0.0),
    (20.0, 20.0),
    (25.0, 40.0),
    (30.0, 60.0),
];
const TS480_STR: &[(f32, f32)] = &[
    (0.0, -60.0),
    (3.0, -48.0),
    (5.0, -36.0),
    (7.0, -24.0),
    (9.0, -12.0),
    (11.0, 0.0),
    (14.0, 20.0),
    (17.0, 40.0),
    (20.0, 60.0),
];
const TS590_STR: &[(f32, f32)] = &[
    (0.0, -60.0),
    (3.0, -48.0),
    (6.0, -36.0),
    (9.0, -24.0),
    (12.0, -12.0),
    (15.0, 0.0),
    (20.0, 20.0),
    (25.0, 40.0),
    (30.0, 60.0),
];
const TS990_STR: &[(f32, f32)] = &[
    (0.0, -54.0),
    (4.0, -48.0),
    (11.0, -36.0),
    (19.0, -24.0),
    (27.0, -12.0),
    (35.0, 0.0),
    (70.0, 60.0),
];

// Bars → SWR. Also Hamlib's, and note how differently the three generations
// scale: full deflection is 20 bars on a TS-2000, 30 on a TS-590, 70 on a
// TS-890.
const TS2000_SWR: &[(f32, f32)] = &[(0.0, 1.0), (4.0, 1.5), (8.0, 2.0), (12.0, 3.0), (20.0, 10.0)];
const TS590_SWR: &[(f32, f32)] = &[(0.0, 1.0), (6.0, 1.5), (12.0, 2.0), (18.0, 3.0), (30.0, 10.0)];
const TS890_SWR: &[(f32, f32)] = &[(0.0, 1.0), (11.0, 1.5), (23.0, 2.0), (35.0, 3.0), (70.0, 15.0)];
const TS990_SWR: &[(f32, f32)] = &[(0.0, 1.0), (7.0, 1.5), (36.0, 3.0), (43.0, 6.0), (70.0, 10.0)];

/// The meters of the model behind an `ID;` reply, or `None` for one this file
/// has never been told about.
///
/// Note 017: an Elecraft K2, K3 or KX3 answers `ID;` with it too. That is not a
/// problem to solve here — an Elecraft belongs on [`crate::elecraft`] — but it
/// is why the TS-570's entry claims no SWR curve.
fn caps_for(id: u32) -> Option<ModelCaps> {
    Some(match id {
        // TS-870S, TS-570D/S. No published meter curves, and no sub receiver,
        // so the S-meter read is the bare one.
        15 | 17 | 18 => ModelCaps {
            name: if id == 15 { "TS-870S" } else { "TS-570" },
            str_cal: None,
            swr: None,
            swr_needs_enable: false,
            smeter_has_vfo: false,
            slopes: None,
        },
        19 => ModelCaps {
            name: "TS-2000",
            str_cal: Some(TS2000_STR),
            swr: Some(('1', TS2000_SWR)),
            swr_needs_enable: false,
            smeter_has_vfo: true,
            slopes: Some((TS2000_SL, TS2000_SH)),
        },
        20 => ModelCaps {
            name: "TS-480",
            str_cal: Some(TS480_STR),
            // The TS-480's own bar count matches the TS-2000's.
            swr: Some(('1', TS2000_SWR)),
            swr_needs_enable: false,
            smeter_has_vfo: true,
            slopes: Some((TS480_SL, TS480_SH)),
        },
        21 | 23 => ModelCaps {
            name: if id == 21 { "TS-590S" } else { "TS-590SG" },
            str_cal: Some(TS590_STR),
            swr: Some(('1', TS590_SWR)),
            swr_needs_enable: false,
            smeter_has_vfo: true,
            // The TS-590's slope tables are the TS-480's.
            slopes: Some((TS480_SL, TS480_SH)),
        },
        22 => ModelCaps {
            name: "TS-990S",
            str_cal: Some(TS990_STR),
            // The later pair moved the SWR to meter 2 — meter 1 is the ALC
            // there, and reading it as an SWR would show a clean antenna as a
            // fault every time the compressor worked.
            swr: Some(('2', TS990_SWR)),
            swr_needs_enable: true,
            smeter_has_vfo: true,
            // The later pair address their filters differently again, and
            // sdroxide has no table for them.
            slopes: None,
        },
        24 => ModelCaps {
            name: "TS-890S",
            // No published S-meter curve for this one; the generic line stands.
            str_cal: None,
            swr: Some(('2', TS890_SWR)),
            swr_needs_enable: true,
            smeter_has_vfo: true,
            slopes: None,
        },
        _ => return None,
    })
}

/// `SM` bars → dB relative to S9 on a rig with no published curve.
///
/// Hamlib's fallback, and the only defensible one: a straight line through the
/// two points every Kenwood bar graph agrees on — no bars is S0 (54 dB below
/// S9) and each bar is 4 dB.
fn generic_strength(bars: f32) -> f32 {
    bars * 4.0 - 54.0
}

/// The meters assumed before `ID;` has been answered — none. The S-meter still
/// reads, on the generic line; the SWR waits for the model, because there is no
/// generic SWR curve worth showing.
const UNKNOWN_CAPS: ModelCaps = ModelCaps {
    name: "unidentified",
    str_cal: None,
    swr: None,
    swr_needs_enable: false,
    smeter_has_vfo: true,
    slopes: None,
};

/// How many mode replies to wait through before concluding the rig has no `DA`
/// command. A TS-2000-generation rig has no DATA mode and answers `?;`, which
/// is indistinguishable from any other rejection — but a rig that *does* have
/// it answers the very first poll, so silence across a few rounds settles it.
const DATA_PROBE_POLLS: u32 = 3;

pub struct Kenwood {
    buf: String,
    /// Which generation's `TX` form keys this rig — see [`KenwoodSend`].
    send: KenwoodSend,
    /// Mode digit from the rig's last `MD;` reply.
    mode_digit: Option<char>,
    /// Whether the rig last reported DATA mode on.
    data_on: bool,
    /// True once the rig has answered a `DA;`, i.e. it has a DATA mode at all.
    da_seen: bool,
    /// `MD;` replies seen, counted only until [`DATA_PROBE_POLLS`].
    md_replies: u32,
    /// What this rig's meters and filters do, learned from its `ID;` reply.
    caps: ModelCaps,
    /// True once `ID;` has been answered, so the model is only logged once.
    model_known: bool,
}

impl Kenwood {
    pub fn new(send: KenwoodSend) -> Self {
        Kenwood {
            buf: String::new(),
            send,
            mode_digit: None,
            data_on: false,
            da_seen: false,
            md_replies: 0,
            caps: UNKNOWN_CAPS,
            model_known: false,
        }
    }

    /// Adopt the meter scales of the model behind an `ID;` reply.
    ///
    /// An unrecognised number is worth saying out loud: it leaves the SWR meter
    /// blank for the rest of the session, and the number in the log is the one
    /// thing that would let the model be added.
    fn learn_model(&mut self, id: u32) {
        if self.model_known {
            return;
        }
        self.model_known = true;
        match caps_for(id) {
            Some(m) => {
                info!(model = m.name, id, swr = m.swr.is_some(), "Kenwood CAT: rig identified");
                self.caps = m;
            }
            None => warn!(
                "Kenwood CAT: rig reports model ID {id:03}, which sdroxide has no meter scales \
                 for — the S-meter will read approximately and the SWR not at all"
            ),
        }
    }

    /// Whether to keep spending frames on `DA`. A rig that has answered one is
    /// asked forever; one that has stayed silent through [`DATA_PROBE_POLLS`]
    /// mode replies hasn't got the command, and asking again only earns another
    /// `?;` every poll for as long as the station is on.
    fn data_supported(&self) -> bool {
        self.da_seen || self.md_replies < DATA_PROBE_POLLS
    }

    /// The rig's mode and DATA flag combined into the app's mode.
    ///
    /// Deliberately not the inverse of [`Self::mode_frames`] over its whole
    /// range. A rig position that would be commanded back as something else
    /// yields `None` — the two would otherwise take turns correcting each
    /// other. That covers FSK/FSK-R (sdroxide's RTTY is its own modem in a data
    /// sideband, not the rig's FSK) and the DATA flavours of FM and AM, which
    /// have no round-trip-stable app equivalent.
    fn effective_mode(&self) -> Option<Mode> {
        Some(match (self.mode_digit?, self.data_on) {
            ('1', false) => Mode::Lsb,
            ('1', true) => Mode::Digl,
            ('2', false) => Mode::Usb,
            ('2', true) => Mode::Digu,
            // CW and CW-R are both CW to the app, and neither carries DATA.
            ('3' | '7', _) => Mode::Cw,
            ('4', false) => Mode::Nfm,
            ('5', false) => Mode::Am,
            _ => return None,
        })
    }
}

/// The rig's mode digit for an app mode, and whether DATA rides on top of it.
/// Kenwood `MD`: 1=LSB 2=USB 3=CW 4=FM 5=AM 6=FSK 7=CW-R 9=FSK-R (0 and 8 are
/// documented as setting failures).
fn mode_digit(m: Mode) -> (char, bool) {
    match m {
        Mode::Lsb => ('1', false),
        Mode::Cw => ('3', false),
        // No rig has an ADS-B mode and none ever will: the dial is at
        // 1090 MHz. Grouped with FM so nothing downstream has to special-case
        // a mode a radio can neither be put into nor report back.
        Mode::Nfm | Mode::Wfm | Mode::Adsb | Mode::Vdl2 | Mode::Ais => ('4', false),
        // RIFP keys the carrier itself and VHF packet frequency-modulates it:
        // data over FM, not over a sideband.
        Mode::Rifp | Mode::Packet | Mode::Aprs | Mode::SstvFm | Mode::RttyFm => ('4', true),
        Mode::Am | Mode::Sam | Mode::Dsb | Mode::Isb | Mode::Drm => ('5', false),
        Mode::Digl => ('1', true),
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
        | Mode::Rade => ('2', true),
        Mode::Usb | Mode::Spec | Mode::Sstv | Mode::Wefax | Mode::Navtex | Mode::RfPaint => {
            ('2', false)
        }
    }
}

/// A meter reply's numeric field — a count of lit bars — or `None` when it is
/// not one.
///
/// The width is fixed at four across both meter commands. A short or mangled
/// reply is no reading at all: it has to leave the meter where it was rather
/// than slam it to the bottom of the scale, which on the SWR meter would read
/// as a perfect match at exactly the moment something is wrong.
fn bars(digits: &str) -> Option<f32> {
    if digits.len() != 4 {
        return None;
    }
    digits.parse::<u32>().ok().map(|v| v as f32)
}

/// Reduce `text` to what the `KY` buffer will accept: the letters, digits and
/// punctuation listed in the reference guide, and nothing longer than the
/// buffer holds.
///
/// The bracketed prosigns the CW panel writes become the single symbols the rig
/// keys them as — `<SK>` is `>` to a Kenwood, not the two letters S and K.
fn keyer_text(text: &str) -> String {
    let mut s = text.trim().to_ascii_uppercase();
    // Documented abbreviation table: the symbol *is* the prosign.
    for (token, symbol) in
        [("<BT>", "["), ("<AR>", "_"), ("<AS>", "<"), ("<SK>", ">"), ("<KN>", "]"), ("<BK>", "\\")]
    {
        if s.contains(token) {
            s = s.replace(token, symbol);
        }
    }
    s.chars()
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

impl Kenwood {
    /// `MD` for `m`, followed by the `DA` that selects or clears DATA mode.
    ///
    /// `MD` goes first: `DA` is an error in CW and FSK, so the rig has to be in
    /// a mode that accepts it before it arrives. Modes that carry no DATA flag
    /// send no `DA` at all rather than an explicit `DA0;`, which in CW would be
    /// exactly that error.
    fn mode_frames(&self, m: Mode) -> Vec<u8> {
        let (digit, data) = mode_digit(m);
        let mut out = format!("MD{digit};").into_bytes();
        // CW (3) and CW-R (7) reject `DA` outright; FSK is never commanded here.
        if self.data_supported() && !matches!(digit, '3' | '7') {
            out.extend_from_slice(if data { b"DA1;" } else { b"DA0;" });
        }
        out
    }
}

impl Protocol for Kenwood {
    fn set_freq(&mut self, hz: f64) -> Vec<u8> {
        let hz = hz.round().clamp(0.0, 99_999_999_999.0) as u64;
        format!("FA{hz:0FREQ_DIGITS$};").into_bytes()
    }

    fn set_mode(&mut self, m: Mode) -> Vec<u8> {
        self.mode_frames(m)
    }

    fn ptt(&self, on: bool) -> Vec<u8> {
        if on {
            match self.send {
                KenwoodSend::Ts2000 => b"TX;".to_vec(),
                KenwoodSend::Ts590 => b"TX1;".to_vec(),
            }
        } else {
            // Never `TX0;` — that is a *transmit* command on this family.
            b"RX;".to_vec()
        }
    }

    fn poll_requests(&self) -> Vec<Vec<u8>> {
        let mut reqs = vec![b"FA;".to_vec(), b"MD;".to_vec()];
        if self.data_supported() {
            reqs.push(b"DA;".to_vec());
        }
        reqs
    }
    // `DA` is the DATA switch beside the mode, so it belongs with the mode
    // rather than with the dial.
    fn dial_requests(&self) -> Vec<Vec<u8>> {
        vec![b"FA;".to_vec()]
    }

    /// `IF;` — the whole-status reply, read for one character of it.
    ///
    /// This family has no "are you transmitting" command of its own, so the
    /// transmit flag has to be picked out of the status string by position (see
    /// [`if_transmitting`]). Only that one character is taken: `IF` also
    /// carries the frequency and the mode, and reading those here would give
    /// the engine a second, differently-timed opinion about both when `FA;`
    /// and `MD;` are already answering for them.
    fn tx_state_requests(&self) -> Vec<Vec<u8>> {
        vec![b"IF;".to_vec()]
    }

    fn tx_telemetry_requests(&self) -> Vec<Vec<u8>> {
        let Some((digit, _)) = self.caps.swr else {
            // No curve for this model: asking would only produce a number
            // there is nothing honest to do with.
            return Vec::new();
        };
        let mut reqs = Vec::new();
        if self.caps.swr_needs_enable {
            // The TS-890/990 report only the meters that are switched on, and
            // the operator may have the front panel on ALC. Re-asserted with
            // every read rather than once at connect: this only goes out while
            // transmitting, when SWR is the meter that matters anyway.
            reqs.push(format!("RM{digit}1;").into_bytes());
        }
        // One read, every meter: the rig answers `RM10000;RM20000;RM30000;` and
        // the digit says which is which.
        reqs.push(b"RM;".to_vec());
        reqs
    }

    fn rx_telemetry_requests(&self) -> Vec<Vec<u8>> {
        if self.caps.smeter_has_vfo { vec![b"SM0;".to_vec()] } else { vec![b"SM;".to_vec()] }
    }

    fn clear_offsets(&self) -> Vec<Vec<u8>> {
        vec![
            // Stop unsolicited status the rig would otherwise push at us — we
            // poll, and a previous program may have left auto-information on.
            b"AI0;".to_vec(),
            // Which Kenwood this is. Only the meters and the receive filter
            // care (see `ModelCaps`), but neither can do anything until it has
            // been answered, so it goes out with the rest of the connect
            // traffic rather than waiting for the first key-down.
            b"ID;".to_vec(),
            // Clear the clarifier while it is still on: `RC` is an error once
            // RIT and XIT are both off, so it cannot come after them.
            b"RC;".to_vec(),
            b"RT0;".to_vec(),
            b"XT0;".to_vec(),
            // Receive on VFO A, which also returns the rig to simplex — the
            // documented side effect of `FR`, and the only split-off this
            // family has. It doubles as the fix for a rig left on VFO B or in
            // memory mode, where every `FA` we send would land on a VFO that
            // isn't the one being listened to.
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
        let mut frames = Vec::new();
        // Break-in has to be on or the keyer runs into the sidetone and never
        // keys the transmitter. `VX` is that switch only while the rig is in CW
        // — in any other mode it is the VOX switch, and turning VOX on under a
        // station whose sound card is live would key the rig on its own audio.
        // So it goes out only once the rig has told us it is in CW.
        if matches!(self.mode_digit, Some('3') | Some('7')) {
            frames.push(b"VX1;".to_vec());
        }
        // One space between `KY` and the text — the documented P1, and the
        // difference between keying and a syntax error.
        frames.push(format!("KY {msg};").into_bytes());
        frames
    }

    fn abort_cw(&mut self) -> Vec<Vec<u8>> {
        // `KY0;` — the documented stop for a message in progress.
        vec![b"KY0;".to_vec()]
    }

    fn set_cw_wpm(&mut self, wpm: f32) -> Vec<Vec<u8>> {
        let wpm = wpm.round().clamp(4.0, 60.0) as u32;
        vec![format!("KS{wpm:03};").into_bytes()]
    }

    fn set_filter(&mut self, mode: Mode, lo_hz: f32, hi_hz: f32) -> Vec<Vec<u8>> {
        // Sideband lives in the sign of the edges on this side, so an LSB
        // passband arrives negative. The rig knows nothing of that: its filter
        // is a pair of distances from the carrier either way.
        let (lo, hi) = (lo_hz.abs().min(hi_hz.abs()), lo_hz.abs().max(hi_hz.abs()));
        match mode {
            // CW is the one mode this family takes a real bandwidth for —
            // `FW` in Hz — because the rig's CW filter is centred on its own
            // pitch and only its width is in question.
            Mode::Cw => {
                let w = (hi - lo).round().clamp(0.0, 9999.0) as u32;
                vec![format!("FW{w:04};").into_bytes()]
            }
            // FM and AM have their own slope tables, which sdroxide does not
            // carry: their filters are wide, rarely worth narrowing from here,
            // and getting an index wrong in AM costs more than it buys.
            Mode::Nfm
            | Mode::Wfm
            | Mode::Am
            | Mode::Sam
            | Mode::Dsb
            | Mode::Rifp
            | Mode::Packet
            | Mode::Aprs
            | Mode::SstvFm => Vec::new(),
            // Everything else rides a sideband, where the filter is not a width
            // at all but a pair of cuts — which is exactly what sdroxide's own
            // filter edges are, so the two map onto each other directly.
            _ => {
                let Some((sl, sh)) = self.caps.slopes else {
                    return Vec::new();
                };
                let (l, h) = (slope_index(sl, lo, false), slope_index(sh, hi, true));
                vec![format!("SL{l:02};SH{h:02};").into_bytes()]
            }
        }
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

    fn parse(&mut self, buf: &mut Vec<u8>) -> Vec<CatUpdate> {
        self.buf.push_str(&String::from_utf8_lossy(buf));
        buf.clear();
        let mut out = Vec::new();
        // USB and USB-DATA are the same `MD`, so a mode is only whole once both
        // replies are in. Emitting per-reply would report plain USB for the
        // moment between them every poll.
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
                    self.md_replies = self.md_replies.saturating_add(1).min(DATA_PROBE_POLLS);
                    mode_touched = true;
                }
            } else if let Some(rest) = msg.strip_prefix("PC") {
                if let Some(frac) = crate::pc_parse(rest) {
                    out.push(CatUpdate::Power(frac));
                }
            } else if let Some(rest) = msg.strip_prefix("ID") {
                if let Ok(id) = rest.trim().parse::<u32>() {
                    self.learn_model(id);
                }
            } else if let Some(rest) = msg.strip_prefix("SM") {
                // `SM<vfo><4 digits>` on a rig with a sub receiver, `SM<4
                // digits>` on one without — and only the main receiver's is
                // followed either way.
                let bars = match self.caps.smeter_has_vfo {
                    true => rest.strip_prefix('0').and_then(bars),
                    false => bars(rest),
                };
                if let Some(bars) = bars {
                    let db = match self.caps.str_cal {
                        Some(cal) => crate::interp(cal, bars),
                        None => generic_strength(bars),
                    };
                    out.push(CatUpdate::Signal(crate::S9_DBM + db));
                }
            } else if let Some(rest) = msg.strip_prefix("RM") {
                // Every meter answers on this one command, so the digit is the
                // only thing that says which reading arrived — and which digit
                // is the SWR is itself a question about the model.
                if let Some((digit, cal)) = self.caps.swr
                    && let Some(v) = rest.strip_prefix(digit).and_then(bars)
                {
                    out.push(CatUpdate::Swr(crate::interp(cal, v)));
                }
            } else if let Some(rest) = msg.strip_prefix("IF") {
                if let Some(on) = if_transmitting(rest) {
                    out.push(CatUpdate::Ptt(on));
                }
            } else if let Some(rest) = msg.strip_prefix("DA") {
                if let Some(d) = rest.chars().next() {
                    self.data_on = d == '1';
                    self.da_seen = true;
                    mode_touched = true;
                }
            } else if msg == "?" {
                // The rig refused the last command. Nothing identifies which,
                // so this can only be a breadcrumb — but it is the difference
                // between "the radio is ignoring me" and silence.
                debug!("Kenwood CAT: rig rejected a command (?)");
            } else if msg == "E" || msg == "O" {
                // Framing/overrun (`E`) or a receive-buffer overrun (`O`) — the
                // serial line itself, not the command.
                debug!("Kenwood CAT: serial error from rig ({msg})");
            }
        }
        if mode_touched && let Some(m) = self.effective_mode() {
            out.push(CatUpdate::Mode(m));
        }
        out
    }
}

/// Whether the rig is transmitting, from the body of an `IF;` reply (the
/// 35 characters after `IF`, with the terminating `;` already removed).
///
/// Kenwood's status string is positional, so this is an offset and nothing
/// else: 11 digits of frequency, 4 of step, a signed 5-digit RIT/XIT offset,
/// RIT, XIT, the memory bank and a 2-digit channel — 26 characters — and then
/// the transmit flag. `0` is receive and `1` is transmit.
///
/// ⚠️ Read out of the published command reference, never off a radio. The
/// length is checked exactly rather than indexed into blindly, so a rig whose
/// status string is laid out differently reports nothing at all instead of
/// reporting whatever character happens to sit at offset 26 — a dialect that
/// does not match must look like a rig that cannot answer, not like one that is
/// permanently on the air.
fn if_transmitting(body: &str) -> Option<bool> {
    const TX_FLAG: usize = 26;
    const IF_BODY_LEN: usize = 35;
    if body.len() != IF_BODY_LEN {
        return None;
    }
    match body.as_bytes()[TX_FLAG] {
        b'0' => Some(false),
        b'1' => Some(true),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kenwood() -> Kenwood {
        Kenwood::new(KenwoodSend::Ts2000)
    }

    fn parse_str(k: &mut Kenwood, s: &str) -> Vec<CatUpdate> {
        let mut buf = s.as_bytes().to_vec();
        k.parse(&mut buf)
    }

    fn frames(v: Vec<Vec<u8>>) -> Vec<String> {
        v.iter().map(|f| String::from_utf8_lossy(f).into_owned()).collect()
    }

    /// An `IF;` body built from its documented fields, so the offset this
    /// parser indexes with is spelled out rather than counted by hand.
    fn if_body(tx: bool) -> String {
        let f = [
            "00014074000", // P1  frequency, 11
            "    ",        // P2  step, 4
            "+00000",      // P3  RIT/XIT offset, sign + 5
            "0",           // P4  RIT
            "0",           // P5  XIT
            "0",           // P6  memory bank
            "00",          // P7  memory channel, 2
                           //             // P8  is the transmit flag, appended below
        ]
        .concat();
        let tail = [
            "2",  // P9  mode
            "0",  // P10 VFO/memory
            "0",  // P11 scan
            "0",  // P12 split
            "0",  // P13 tone
            "08", // P14 tone number
            "0",  // P15 always zero
        ]
        .concat();
        format!("{f}{}{tail}", if tx { '1' } else { '0' })
    }

    /// The one character of `IF;` this family reads, and the guard that keeps a
    /// differently-shaped status string from being read as a key-down.
    #[test]
    fn the_transmit_flag_is_taken_from_if_by_position() {
        let rx = if_body(false);
        assert_eq!(rx.len(), 35, "the reply this parser indexes into is 35 + \"IF\" + \";\"");
        assert_eq!(if_transmitting(&rx), Some(false));
        assert_eq!(if_transmitting(&if_body(true)), Some(true));

        // Anything that is not this exact shape reports nothing rather than
        // guessing — a rig left stuck "transmitting" would blank the S-meter
        // and refuse every key-down.
        assert_eq!(if_transmitting(""), None);
        assert_eq!(if_transmitting(&rx[..34]), None);
        assert_eq!(if_transmitting(&format!("{rx}0")), None);
    }

    #[test]
    fn the_status_reply_is_not_a_second_opinion_about_freq_and_mode() {
        // `IF` carries the dial and the mode too. Taking them here would race
        // the `FA;`/`MD;` answers, so only the transmit flag comes out of it.
        let mut k = kenwood();
        let out = parse_str(&mut k, &format!("IF{};", if_body(true)));
        assert_eq!(out, vec![CatUpdate::Ptt(true)]);
    }

    #[test]
    fn frequency_is_eleven_digits() {
        let mut k = kenwood();
        assert_eq!(k.set_freq(14_074_000.0), b"FA00014074000;".to_vec());
        assert_eq!(k.set_freq(7_055_000.0), b"FA00007055000;".to_vec());
        // A reply at that width is a frequency; anything else is not, and must
        // not be read as one at some other scale.
        assert_eq!(parse_str(&mut k, "FA00014195000;"), vec![CatUpdate::Freq(14_195_000.0)]);
        assert!(parse_str(&mut k, "FA014074000;").is_empty()); // Yaesu's nine
        assert!(parse_str(&mut k, "FAxxxxxxxxxxx;").is_empty());
    }

    #[test]
    fn unkeying_uses_rx_because_tx0_would_transmit() {
        // Whichever generation keys the rig, only `RX;` stops it. `TX0;` — the
        // Yaesu unkey — would be a second key-down.
        for send in KenwoodSend::ALL {
            assert_eq!(Kenwood::new(send).ptt(false), b"RX;".to_vec(), "{send:?} unkey");
        }
    }

    #[test]
    fn each_generation_keys_with_its_own_send() {
        // TS-2000 and earlier: the ordinary send, on the main band. `TX1;`
        // there is the *sub*-band, which is a different band entirely.
        assert_eq!(Kenwood::new(KenwoodSend::Ts2000).ptt(true), b"TX;".to_vec());
        // TS-590 and later: DATA SEND, which keys with the ACC2/USB input live
        // instead of the microphone.
        assert_eq!(Kenwood::new(KenwoodSend::Ts590).ptt(true), b"TX1;".to_vec());
        // The default cannot transmit somewhere unintended.
        assert_eq!(KenwoodSend::default(), KenwoodSend::Ts2000);
    }

    #[test]
    fn data_mode_rides_beside_the_mode_not_inside_it() {
        let mut k = kenwood();
        // `MD` first: `DA` is an error in a mode that has no DATA flavour.
        assert_eq!(k.set_mode(Mode::Digu), b"MD2;DA1;".to_vec());
        assert_eq!(k.set_mode(Mode::Usb), b"MD2;DA0;".to_vec());
        assert_eq!(k.set_mode(Mode::Digl), b"MD1;DA1;".to_vec());
        // CW rejects `DA` outright, so none is sent.
        assert_eq!(k.set_mode(Mode::Cw), b"MD3;".to_vec());
    }

    #[test]
    fn usb_and_usb_data_are_told_apart_by_the_data_flag() {
        let mut k = kenwood();
        // Both replies arrive in one poll; the mode is reported once, whole.
        assert_eq!(parse_str(&mut k, "MD2;DA1;"), vec![CatUpdate::Mode(Mode::Digu)]);
        assert_eq!(parse_str(&mut k, "MD2;DA0;"), vec![CatUpdate::Mode(Mode::Usb)]);
        assert_eq!(parse_str(&mut k, "MD1;DA1;"), vec![CatUpdate::Mode(Mode::Digl)]);
        assert_eq!(parse_str(&mut k, "MD3;DA0;"), vec![CatUpdate::Mode(Mode::Cw)]);
        // CW-R is CW to the app.
        assert_eq!(parse_str(&mut k, "MD7;"), vec![CatUpdate::Mode(Mode::Cw)]);
    }

    #[test]
    fn rig_positions_without_a_stable_round_trip_are_left_alone() {
        let mut k = kenwood();
        // FSK / FSK-R: sdroxide's RTTY is its own modem in a data sideband and
        // would be commanded back as `MD2;DA1;`.
        assert!(parse_str(&mut k, "MD6;DA0;").is_empty());
        assert!(parse_str(&mut k, "MD9;DA0;").is_empty());
        // Documented setting-failure values.
        assert!(parse_str(&mut k, "MD0;").is_empty());
        assert!(parse_str(&mut k, "MD8;").is_empty());
        // FM-DATA and AM-DATA have no app equivalent, but plain FM and AM do.
        assert!(parse_str(&mut k, "MD4;DA1;").is_empty());
        assert_eq!(parse_str(&mut k, "MD4;DA0;"), vec![CatUpdate::Mode(Mode::Nfm)]);
        assert!(parse_str(&mut k, "MD5;DA1;").is_empty());
        assert_eq!(parse_str(&mut k, "MD5;DA0;"), vec![CatUpdate::Mode(Mode::Am)]);
    }

    #[test]
    fn every_app_mode_the_rig_can_hold_survives_a_round_trip() {
        // What we command has to be what we read back, or the app and the rig
        // spend the session correcting each other.
        for m in [Mode::Lsb, Mode::Usb, Mode::Cw, Mode::Nfm, Mode::Am, Mode::Digl, Mode::Digu] {
            let mut k = kenwood();
            let sent = String::from_utf8(k.set_mode(m)).unwrap();
            // Echo the set back as the rig's replies would read.
            let (digit, data) = mode_digit(m);
            let reply = format!("MD{digit};DA{};", u8::from(data));
            assert_eq!(
                parse_str(&mut k, &reply),
                vec![CatUpdate::Mode(m)],
                "{m:?} was set with {sent} and read back as something else"
            );
        }
    }

    #[test]
    fn a_rig_without_a_data_command_stops_being_asked() {
        let mut k = kenwood();
        assert!(frames(k.poll_requests()).contains(&"DA;".to_string()));
        // A TS-2000-generation rig answers the mode and rejects `DA`.
        for _ in 0..DATA_PROBE_POLLS {
            assert_eq!(parse_str(&mut k, "MD2;?;"), vec![CatUpdate::Mode(Mode::Usb)]);
        }
        assert_eq!(frames(k.poll_requests()), vec!["FA;", "MD;"]);
        // And no longer carries a `DA` it would only reject again.
        assert_eq!(k.set_mode(Mode::Digu), b"MD2;".to_vec());
    }

    #[test]
    fn a_rig_that_answers_da_keeps_being_asked() {
        let mut k = kenwood();
        for _ in 0..DATA_PROBE_POLLS + 2 {
            parse_str(&mut k, "MD2;DA0;");
        }
        assert!(frames(k.poll_requests()).contains(&"DA;".to_string()));
    }

    #[test]
    fn replies_split_across_reads_are_reassembled() {
        let mut k = kenwood();
        assert!(parse_str(&mut k, "FA000140").is_empty());
        assert_eq!(parse_str(&mut k, "74000;"), vec![CatUpdate::Freq(14_074_000.0)]);
    }

    #[test]
    fn cw_is_streamed_to_the_keyer_with_break_in_only_when_in_cw() {
        let mut k = kenwood();
        // Mode unknown: `VX` would be the VOX switch, so it is not sent.
        assert_eq!(frames(k.send_cw("cq de w1aw")), vec!["KY CQ DE W1AW;"]);
        // In USB it is still the VOX switch.
        parse_str(&mut k, "MD2;DA0;");
        assert_eq!(frames(k.send_cw("test")), vec!["KY TEST;"]);
        // In CW it is break-in, which the keyer needs.
        parse_str(&mut k, "MD3;");
        assert_eq!(frames(k.send_cw("test")), vec!["VX1;", "KY TEST;"]);
    }

    #[test]
    fn keyer_text_keeps_only_what_the_rig_can_key() {
        // Upper-cased, runs of spaces collapsed, edges trimmed.
        assert_eq!(keyer_text("  r r  tu  "), "R R TU");
        // `=` is BT and is in the rig's own character table, so it goes as is.
        assert_eq!(keyer_text("r r = tu"), "R R = TU");
        // Bracketed prosigns become the single symbols the rig keys them as.
        assert_eq!(keyer_text("tu <sk>"), "TU >");
        assert_eq!(keyer_text("<bt> <ar> <as> <kn> <bk>"), "[ _ < ] \\");
        // A semicolon would end the frame early; it never reaches the wire.
        assert_eq!(keyer_text("de;w1aw"), "DEW1AW");
        assert_eq!(keyer_text("w1aw/p ur 599 ok?"), "W1AW/P UR 599 OK?");
        // Nothing sendable produces no frames at all rather than an empty `KY`.
        assert!(keyer_text("   ").is_empty());
        assert!(kenwood().send_cw("~~~").is_empty());
        // Longer than the buffer holds is truncated, not refused.
        assert_eq!(keyer_text(&"a".repeat(80)).len(), CW_MAX);
    }

    #[test]
    fn keyer_speed_is_three_digits_within_the_rigs_range() {
        let mut k = kenwood();
        let frame = |k: &mut Kenwood, wpm: f32| String::from_utf8(k.set_cw_wpm(wpm)[0].clone());
        assert_eq!(frame(&mut k, 20.0).unwrap(), "KS020;");
        // The panel offers speeds the rig's keyer does not go to.
        assert_eq!(frame(&mut k, 80.0).unwrap(), "KS060;");
        assert_eq!(frame(&mut k, 1.0).unwrap(), "KS004;");
    }

    #[test]
    fn the_rigs_own_offsets_are_cleared_in_an_order_it_accepts() {
        // `RC` before `RT0`/`XT0`: it is an error once both are already off.
        // `FR0` last, and never `FT2` — that is Yaesu's split-off and a
        // forbidden memory-channel select here.
        assert_eq!(
            frames(kenwood().clear_offsets()),
            vec!["AI0;", "ID;", "RC;", "RT0;", "XT0;", "FR0;"]
        );
    }

    #[test]
    fn the_meter_scales_come_from_the_model_the_rig_reports() {
        let swr = |k: &mut Kenwood, s: &str| match parse_str(k, s).as_slice() {
            [CatUpdate::Swr(v)] => Some(*v),
            [] => None,
            other => panic!("expected at most one SWR reading, got {other:?}"),
        };

        // A TS-590: thirty bars of SWR, on meter 1, and the whole set of meters
        // arrives on one read.
        let mut k = kenwood();
        parse_str(&mut k, "ID021;");
        assert_eq!(frames(k.tx_telemetry_requests()), vec!["RM;"]);
        assert_eq!(swr(&mut k, "RM10012;"), Some(2.0));

        // The identical reading on a TS-2000, whose scale is twenty bars, is a
        // three-to-one fault rather than a two-to-one match. This is the whole
        // reason the model is asked for.
        let mut k = kenwood();
        parse_str(&mut k, "ID019;");
        assert_eq!(swr(&mut k, "RM10012;"), Some(3.0));

        // A TS-890 keeps its SWR on meter *2* — meter 1 is the ALC there — and
        // has to be told to report it at all.
        let mut k = kenwood();
        parse_str(&mut k, "ID024;");
        assert_eq!(frames(k.tx_telemetry_requests()), vec!["RM21;", "RM;"]);
        assert_eq!(swr(&mut k, "RM20023;"), Some(2.0));
        // Reading meter 1 there would show a clean antenna as a fault whenever
        // the ALC moved.
        assert_eq!(swr(&mut k, "RM10023;"), None);
    }

    #[test]
    fn an_unrecognised_model_reports_no_swr_at_all() {
        let mut k = kenwood();
        // Nothing asked yet, and nothing to ask with.
        assert!(frames(k.tx_telemetry_requests()).is_empty());
        assert!(parse_str(&mut k, "RM10012;").is_empty());
        // A model with no published curve stays that way rather than borrowing
        // another generation's scale, which would read plausibly and mean
        // nothing.
        parse_str(&mut k, "ID099;");
        assert!(frames(k.tx_telemetry_requests()).is_empty());
        assert!(parse_str(&mut k, "RM10012;").is_empty());
    }

    #[test]
    fn the_s_meter_reads_against_this_models_bar_count() {
        let dbm = |k: &mut Kenwood, s: &str| match parse_str(k, s).as_slice() {
            [CatUpdate::Signal(d)] => Some(*d),
            [] => None,
            other => panic!("expected at most one signal reading, got {other:?}"),
        };

        let mut k = kenwood();
        parse_str(&mut k, "ID021;"); // TS-590: fifteen bars is S9
        assert_eq!(frames(k.rx_telemetry_requests()), vec!["SM0;"]);
        assert_eq!(dbm(&mut k, "SM00015;"), Some(-73.0));
        assert_eq!(dbm(&mut k, "SM00030;"), Some(-13.0)); // S9+60
        assert_eq!(dbm(&mut k, "SM00000;"), Some(-133.0)); // S0

        // A TS-990's scale runs to seventy, so the same fifteen bars is a much
        // weaker signal.
        let mut k = kenwood();
        parse_str(&mut k, "ID022;");
        assert_eq!(dbm(&mut k, "SM00035;"), Some(-73.0));
        let fifteen = dbm(&mut k, "SM00015;").unwrap();
        assert!(fifteen < -90.0, "fifteen bars on a TS-990 reads {fifteen} dBm");

        // The sub receiver's meter is a signal nobody here is listening to, and
        // a short or mangled reply must leave the meter where it was rather
        // than drop it to the bottom of the scale.
        assert_eq!(dbm(&mut k, "SM10015;"), None);
        assert_eq!(dbm(&mut k, "SM0015;"), None);
        assert_eq!(dbm(&mut k, "SM0abcd;"), None);
    }

    #[test]
    fn a_sideband_filter_is_a_pair_of_cuts_and_cw_is_a_width() {
        let mut k = kenwood();
        parse_str(&mut k, "ID021;"); // TS-590
        // On a sideband the rig's filter is two cuts, which is exactly what
        // sdroxide's own filter edges are — so they map across directly rather
        // than being collapsed into a width and centred again.
        assert_eq!(frames(k.set_filter(Mode::Usb, 200.0, 2400.0)), vec!["SL03;SH07;"]);
        // LSB arrives negative — sideband lives in the sign of the edges here —
        // and comes out as the same pair of distances from the carrier.
        assert_eq!(frames(k.set_filter(Mode::Lsb, -2400.0, -200.0)), vec!["SL03;SH07;"]);
        // CW is the one mode this family takes a real bandwidth for.
        assert_eq!(frames(k.set_filter(Mode::Cw, 500.0, 1000.0)), vec!["FW0500;"]);
    }

    #[test]
    fn the_slope_indices_err_towards_the_wider_passband() {
        let mut k = kenwood();
        parse_str(&mut k, "ID021;");
        // 250 Hz is not in the low-cut table. Taking the next one *up* would
        // cut away more than was asked for, so the one below is chosen.
        assert_eq!(frames(k.set_filter(Mode::Usb, 250.0, 2400.0)), vec!["SL03;SH07;"]);
        // 2450 is not in the high-cut table either, and there the next one up
        // is the safe direction: a filter quietly too narrow presents as a
        // signal that simply is not there.
        assert_eq!(frames(k.set_filter(Mode::Usb, 200.0, 2450.0)), vec!["SL03;SH08;"]);
        // Past either end of the table, clamped rather than wrapped.
        assert_eq!(frames(k.set_filter(Mode::Usb, 0.0, 99_000.0)), vec!["SL00;SH13;"]);
    }

    #[test]
    fn the_same_index_is_a_different_filter_on_a_different_model() {
        // The TS-2000's high-cut table starts two steps above the TS-590's, so
        // the same passband is a different index on each.
        let mut older = kenwood();
        parse_str(&mut older, "ID019;");
        let mut newer = kenwood();
        parse_str(&mut newer, "ID021;");
        assert_eq!(frames(older.set_filter(Mode::Usb, 200.0, 2400.0)), vec!["SL03;SH05;"]);
        assert_eq!(frames(newer.set_filter(Mode::Usb, 200.0, 2400.0)), vec!["SL03;SH07;"]);
    }

    #[test]
    fn a_model_with_no_slope_table_keeps_its_own_filter() {
        // Nothing known yet, and a TS-890, whose filters are addressed
        // differently again. Neither is a reason to guess at an index.
        let mut k = kenwood();
        assert!(k.set_filter(Mode::Usb, 200.0, 2400.0).is_empty());
        parse_str(&mut k, "ID024;");
        assert!(k.set_filter(Mode::Usb, 200.0, 2400.0).is_empty());
        // AM and FM have slope tables of their own that sdroxide does not
        // carry, on every model.
        let mut k = kenwood();
        parse_str(&mut k, "ID021;");
        for m in [Mode::Am, Mode::Nfm, Mode::Wfm, Mode::Packet] {
            assert!(k.set_filter(m, 0.0, 6000.0).is_empty(), "{m:?} has no slope table");
        }
    }

    #[test]
    fn a_rig_from_before_the_sub_receiver_is_asked_without_one() {
        let mut k = kenwood();
        parse_str(&mut k, "ID017;"); // TS-570
        assert_eq!(frames(k.rx_telemetry_requests()), vec!["SM;"]);
        // No published curve, so the generic line: no bars is S0, each bar 4 dB.
        assert_eq!(parse_str(&mut k, "SM0000;"), vec![CatUpdate::Signal(-127.0)]);
        assert_eq!(parse_str(&mut k, "SM0015;"), vec![CatUpdate::Signal(-127.0 + 60.0)]);
    }
}
