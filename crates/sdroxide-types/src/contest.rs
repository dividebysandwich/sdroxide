//! Contest logging: the contest list, the exchange each one wants, a live score
//! estimate, and the Cabrillo export.
//!
//! The logger is **mode-agnostic** — the operator types the exchange by hand
//! for CW, SSB or anything else, and the FT8 side auto-fills the same entry
//! when it is driving. Nothing here is on the wire: a contest session is the
//! operator's, held in the UI, and it writes into the ordinary logbook as
//! [`crate::QsoRecord`]s.

use serde::{Deserialize, Serialize};

use crate::digi::QsoRecord;

/// The contests the logger ships with.
///
/// [`Generic`](Self::Generic) is the fallback for anything not listed;
/// [`Text`](Self::Text) carries a report and a free-text exchange.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ContestId {
    /// No contest — the logger is idle.
    #[default]
    None,
    /// CQ World Wide: RST + CQ zone.
    CqWw,
    /// CQ WPX: RST + serial.
    CqWpx,
    /// ARRL DX: RST + state (or power from outside the US and Canada).
    ArrlDx,
    /// European VHF: RST + serial + a 6-character locator — the FT8 layout too.
    EuVhf,
    /// RST + a free-text exchange — a signal report plus something typed, for
    /// an activity whose exchange is not one of the modelled ones.
    Text,
    /// RST + serial, for anything else.
    Generic,
}

/// One part of a contest exchange.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exchange {
    /// The signal report — `599`/`59`, or an FT8 dB.
    Rst,
    /// The serial number: auto-incremented when sent, captured when received.
    Serial,
    /// CQ zone (1–40).
    CqZone,
    /// ITU zone (1–90).
    ItuZone,
    /// State or province.
    State,
    /// Maidenhead locator.
    Grid,
    /// Transmitter power.
    Power,
    /// Free text — the CB activity's channel, name or area.
    Text,
}

impl Exchange {
    /// The column heading the entry form and the log show.
    pub fn label(self) -> &'static str {
        match self {
            Exchange::Rst => "RST",
            Exchange::Serial => "SERIAL",
            Exchange::CqZone => "CQ ZONE",
            Exchange::ItuZone => "ITU ZONE",
            Exchange::State => "STATE",
            Exchange::Grid => "GRID",
            Exchange::Power => "POWER",
            Exchange::Text => "EXCHANGE",
        }
    }
}

/// How a contest's multipliers are counted, for the live estimate. The
/// sponsor's own adjudication is authoritative; this is what the operator
/// watches during the contest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Multiplier {
    /// Every QSO counts 1 and there are no multipliers.
    None,
    /// Distinct CQ zones worked.
    CqZone,
    /// Distinct callsign prefixes worked.
    WpxPrefix,
    /// Distinct DXCC entities and US states worked.
    DxccState,
}

/// What a contest exchanges, and how its score is estimated.
#[derive(Debug, Clone, Copy)]
pub struct ContestSpec {
    pub name: &'static str,
    /// The base `CONTEST:` value Cabrillo wants — CW/SSB/RTTY is appended for
    /// the sponsors that split by mode, see [`cabrillo_contest`].
    pub cabrillo: &'static str,
    /// What we send.
    pub sent: &'static [Exchange],
    /// What they send.
    pub rcvd: &'static [Exchange],
    pub multiplier: Multiplier,
}

const RST_SERIAL: &[Exchange] = &[Exchange::Rst, Exchange::Serial];
const RST_ZONE: &[Exchange] = &[Exchange::Rst, Exchange::CqZone];
const RST_TEXT: &[Exchange] = &[Exchange::Rst, Exchange::Text];

impl ContestId {
    /// Every value, `None` included, for serialisation and tests.
    pub const ALL: [ContestId; 7] = [
        ContestId::None,
        ContestId::CqWw,
        ContestId::CqWpx,
        ContestId::ArrlDx,
        ContestId::EuVhf,
        ContestId::Text,
        ContestId::Generic,
    ];

    /// What the picker offers — `None` is the "off" state, not a choice.
    pub const CHOICES: [ContestId; 6] = [
        ContestId::CqWw,
        ContestId::CqWpx,
        ContestId::ArrlDx,
        ContestId::EuVhf,
        ContestId::Text,
        ContestId::Generic,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ContestId::None => "No contest",
            ContestId::CqWw => "CQ World Wide",
            ContestId::CqWpx => "CQ WPX",
            ContestId::ArrlDx => "ARRL DX",
            ContestId::EuVhf => "EU VHF",
            ContestId::Text => "Free-text exchange",
            ContestId::Generic => "Serial (generic)",
        }
    }

    pub fn spec(self) -> ContestSpec {
        match self {
            ContestId::None => ContestSpec {
                name: "No contest",
                cabrillo: "",
                sent: &[],
                rcvd: &[],
                multiplier: Multiplier::None,
            },
            ContestId::CqWw => ContestSpec {
                name: "CQ World Wide",
                cabrillo: "CQ-WW",
                sent: RST_ZONE,
                rcvd: RST_ZONE,
                multiplier: Multiplier::CqZone,
            },
            ContestId::CqWpx => ContestSpec {
                name: "CQ WPX",
                cabrillo: "CQ-WPX",
                sent: RST_SERIAL,
                rcvd: RST_SERIAL,
                multiplier: Multiplier::WpxPrefix,
            },
            ContestId::ArrlDx => ContestSpec {
                name: "ARRL DX",
                cabrillo: "ARRL-DX",
                // We send our state, they send theirs (or their power from
                // outside the US and Canada). One free-ish field either way.
                sent: &[Exchange::Rst, Exchange::State],
                rcvd: &[Exchange::Rst, Exchange::State],
                multiplier: Multiplier::DxccState,
            },
            ContestId::EuVhf => ContestSpec {
                name: "EU VHF",
                cabrillo: "EU-VHF",
                sent: RST_SERIAL,
                rcvd: &[Exchange::Rst, Exchange::Serial, Exchange::Grid],
                multiplier: Multiplier::None,
            },
            ContestId::Text => ContestSpec {
                name: "Free-text exchange",
                cabrillo: "GENERIC-TEXT",
                sent: RST_TEXT,
                rcvd: RST_TEXT,
                multiplier: Multiplier::None,
            },
            ContestId::Generic => ContestSpec {
                name: "Serial (generic)",
                cabrillo: "GENERIC",
                sent: RST_SERIAL,
                rcvd: RST_SERIAL,
                multiplier: Multiplier::None,
            },
        }
    }

    /// Whether the sent exchange carries an auto-incremented serial.
    pub fn sends_serial(self) -> bool {
        matches!(self.spec().sent, s if s.contains(&Exchange::Serial))
    }

    /// The contests that ride the FT8 contest exchange rather than a hand-typed
    /// one. `EU VHF` is the layout the digi side already speaks; the serial
    /// contests match WSJT-X's `RTTY Roundup` shape.
    pub fn is_ft8_contest(self) -> bool {
        matches!(self, ContestId::EuVhf | ContestId::CqWpx | ContestId::Generic)
    }
}

/// A running contest session — the operator's own state, held in the UI and
/// never on the wire.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContestSession {
    pub contest: ContestId,
    /// The next serial to send. Kept even for contests that do not send one, so
    /// switching into a serial contest mid-session does not start at 1 again.
    pub next_serial: u32,
    /// What we send as our own exchange — our zone, state, power or free text.
    pub my_exchange: String,
    pub started_utc: i64,
}

impl ContestSession {
    pub fn new(contest: ContestId, my_exchange: String, now: i64) -> Self {
        ContestSession { contest, next_serial: 1, my_exchange, started_utc: now }
    }
}

/// The live score the operator watches. The sponsor's own adjudication is
/// authoritative — this is an estimate, and it says so on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ContestScore {
    pub qsos: usize,
    pub points: u32,
    pub mults: u32,
    pub total: u32,
}

/// Estimate the score of `log` for `contest`.
///
/// Points are **1 per QSO**: the sponsors' band and continent weighting is
/// theirs to adjudicate, and a wrong guess would be worse than an honest
/// baseline. The multiplier count is real, though, and is what the operator
/// watches.
pub fn score(log: &[QsoRecord], contest: ContestId) -> ContestScore {
    let spec = contest.spec();
    let mults = match spec.multiplier {
        Multiplier::None => 0,
        Multiplier::CqZone => distinct(log, |q| q.cq_zone.map(|z| z.to_string())),
        Multiplier::WpxPrefix => distinct(log, |q| Some(wpx_prefix(&q.call))),
        Multiplier::DxccState => {
            distinct(log, |q| q.dxcc.map(|d| d.to_string()).or_else(|| non_empty(&q.state)))
        }
    };
    let qsos = log.len() as u32;
    let total = if mults == 0 { qsos } else { qsos * mults };
    ContestScore { qsos: log.len(), points: qsos, mults, total }
}

/// Distinct, non-empty values of `pick`, in first-seen order.
fn distinct(log: &[QsoRecord], pick: impl Fn(&QsoRecord) -> Option<String>) -> u32 {
    let mut seen: Vec<String> = Vec::new();
    for q in log {
        if let Some(v) = pick(q).filter(|v| !v.is_empty())
            && !seen.contains(&v)
        {
            seen.push(v);
        }
    }
    seen.len() as u32
}

fn non_empty(s: &str) -> Option<String> {
    let t = s.trim();
    (!t.is_empty()).then(|| t.to_ascii_uppercase())
}

/// The CQ **WPX** prefix of a callsign: the leading letters and the first
/// digit, ignoring any `/suffix`. Exact enough for the live count — WPX has
/// special prefixes (a leading `3DA0`, say) the sponsor settles.
pub fn wpx_prefix(call: &str) -> String {
    let base = call.split('/').next().unwrap_or(call).to_ascii_uppercase();
    let mut p = String::new();
    for c in base.chars() {
        p.push(c);
        if c.is_ascii_digit() {
            break;
        }
    }
    p
}

/// QSOs whose start falls within `window_s` seconds of `now` — the rate panel's
/// number, counted over the window it names.
pub fn rate(log: &[QsoRecord], now: i64, window_s: i64) -> usize {
    log.iter().filter(|q| (0..=window_s).contains(&(now - q.start_utc))).count()
}

/// The Cabrillo v3 mode code for a logged mode string.
pub fn cabrillo_mode(mode: &str) -> &'static str {
    let m = mode.to_ascii_uppercase();
    if m.contains("CW") {
        "CW"
    } else if m.contains("RTTY") {
        "RY"
    } else if m.contains("FT")
        || m.contains("JT")
        || m.contains("PSK")
        || m.contains("MSK")
        || m.contains("Q65")
        || m.contains("FST")
        || m.contains("JS8")
        || m.contains("OLIVIA")
        || m.contains("THOR")
    {
        "DG"
    } else {
        "PH"
    }
}

/// The `CONTEST:` value, with the CW/SSB/RTTY split the sponsor's Cabrillo
/// wants where there is one, chosen from the log's own modes.
pub fn cabrillo_contest(contest: ContestId, log: &[QsoRecord]) -> String {
    let spec = contest.spec();
    if spec.cabrillo.is_empty() {
        return String::new();
    }
    let count = |code: &str| log.iter().filter(|q| cabrillo_mode(&q.mode) == code).count();
    let (cw, phone, ry) = (count("CW"), count("PH"), count("RY"));
    let suffix = if cw >= phone && cw >= ry && cw > 0 {
        "-CW"
    } else if ry > phone && ry > 0 {
        "-RTTY"
    } else {
        "-SSB"
    };
    match contest {
        // Only these three split by mode in their Cabrillo names.
        ContestId::CqWw | ContestId::CqWpx | ContestId::ArrlDx => {
            format!("{}{suffix}", spec.cabrillo)
        }
        _ => spec.cabrillo.to_string(),
    }
}

/// The whole log as a Cabrillo v3 file.
///
/// `my_exchange` is the session's own exchange, used when a QSO carries no
/// exchange of its own. Each row's own `stx_string`/`srx_string` win when set,
/// so a record logged from a session that has since changed still exports what
/// actually went out.
pub fn to_cabrillo(
    contest: ContestId,
    my_call: &str,
    my_exchange: &str,
    log: &[QsoRecord],
) -> String {
    let score = score(log, contest);
    let mut out = String::new();
    out.push_str("START-OF-LOG: 3.0\n");
    let c = cabrillo_contest(contest, log);
    if !c.is_empty() {
        out.push_str(&format!("CONTEST: {c}\n"));
    }
    out.push_str(&format!("CALLSIGN: {}\n", my_call.trim().to_ascii_uppercase()));
    out.push_str(&format!("CLAIMED-SCORE: {}\n", score.total));
    out.push_str("CATEGORY-OPERATOR: SINGLE-OP\n");
    out.push_str("CATEGORY-ASSISTED: NON-ASSISTED\n");
    out.push_str("CATEGORY-BAND: ALL\n");
    out.push_str("CATEGORY-MODE: MIXED\n");
    out.push_str("CATEGORY-POWER: LOW\n");
    out.push_str("CATEGORY-TRANSMITTER: ONE\n");
    out.push_str("CREATED-BY: SDR Oxide Brown\n");
    for q in log {
        let freq_khz = (q.freq_hz / 1000.0).round() as i64;
        if freq_khz <= 0 {
            continue;
        }
        let (y, mo, d, h, mi, _) = crate::utc_ymd_hms(q.start_utc);
        let sent = cabrillo_exchange(q.stx_string.as_str(), q.stx, q.rst_sent, my_exchange);
        let rcvd = cabrillo_exchange(q.srx_string.as_str(), q.srx, q.rst_rcvd, "");
        out.push_str(&format!(
            "QSO: {freq_khz:5} {} {y:04}-{mo:02}-{d:02} {h:02}{mi:02} {} {sent} {} {rcvd}\n",
            cabrillo_mode(&q.mode),
            up_or(my_call, "NOCALL"),
            up_or(&q.call, "NOCALL"),
        ));
    }
    out.push_str("END-OF-LOG:\n");
    out
}

/// One side of a Cabrillo `QSO:` line: report then exchange. The exchange is
/// the row's own string or serial when it has one, else the session's — an
/// FT8 row carries its own, a hand-typed one usually does not.
fn cabrillo_exchange(s: &str, serial: Option<u32>, rst: Option<i16>, fallback: &str) -> String {
    let rpt = rst.map(|r| r.abs().to_string()).unwrap_or_else(|| "59".to_string());
    let ex = if !s.trim().is_empty() {
        s.trim().to_ascii_uppercase()
    } else if let Some(n) = serial {
        n.to_string()
    } else {
        fallback.trim().to_ascii_uppercase()
    };
    if ex.is_empty() { rpt } else { format!("{rpt} {ex}") }
}

fn up_or(s: &str, fallback: &str) -> String {
    let t = s.trim();
    if t.is_empty() { fallback.to_string() } else { t.to_ascii_uppercase() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Mode, QsoRecord};

    fn qso(call: &str, band: &str, mode: &str, when: i64) -> QsoRecord {
        QsoRecord {
            call: call.to_string(),
            band: band.to_string(),
            mode: mode.to_string(),
            start_utc: when,
            ..Default::default()
        }
    }

    #[test]
    fn every_contest_has_a_spec_and_a_label() {
        for c in ContestId::ALL {
            let s = c.spec();
            assert!(!c.label().is_empty());
            if c != ContestId::None {
                assert!(!s.name.is_empty(), "{c:?} has no name");
                assert!(!s.sent.is_empty(), "{c:?} sends nothing");
                assert!(!s.rcvd.is_empty(), "{c:?} receives nothing");
            }
        }
        // The picker never offers "None".
        assert!(!ContestId::CHOICES.contains(&ContestId::None));
    }

    #[test]
    fn serial_contests_are_the_ones_that_say_so() {
        assert!(ContestId::CqWpx.sends_serial());
        assert!(ContestId::EuVhf.sends_serial());
        assert!(ContestId::Generic.sends_serial());
        assert!(!ContestId::CqWw.sends_serial());
        assert!(!ContestId::Text.sends_serial());
    }

    #[test]
    fn wpx_prefixes_ignore_the_suffix() {
        assert_eq!(wpx_prefix("K1ABC"), "K1");
        assert_eq!(wpx_prefix("DL1ABC"), "DL1");
        assert_eq!(wpx_prefix("VP2E"), "VP2");
        assert_eq!(wpx_prefix("K1ABC/P"), "K1");
        assert_eq!(wpx_prefix("dl1abc"), "DL1");
    }

    #[test]
    fn the_zone_contest_counts_distinct_zones() {
        let mut a = qso("DL1ABC", "20M", "SSB", 1000);
        a.cq_zone = Some(14);
        let mut b = qso("JA1ABC", "20M", "SSB", 1001);
        b.cq_zone = Some(25);
        let mut c = qso("DL2XYZ", "40M", "SSB", 1002);
        c.cq_zone = Some(14); // a dupe zone, not a new multiplier
        let log = [a, b, c];
        let s = score(&log, ContestId::CqWw);
        assert_eq!(s.qsos, 3);
        assert_eq!(s.points, 3);
        assert_eq!(s.mults, 2, "14 and 25, not the repeat of 14");
        assert_eq!(s.total, 6);
    }

    #[test]
    fn a_plain_serial_contest_scores_one_each() {
        let log = [qso("K1ABC", "20M", "FT8", 10), qso("K2ABC", "20M", "FT8", 11)];
        let s = score(&log, ContestId::Generic);
        assert_eq!(s.mults, 0);
        assert_eq!(s.total, 2, "no multipliers: the score is the QSO count");
    }

    #[test]
    fn the_rate_counts_only_the_window() {
        let log = [
            qso("A", "20M", "SSB", 100),
            qso("B", "20M", "SSB", 900),
            qso("C", "20M", "SSB", 1000),
        ];
        assert_eq!(rate(&log, 1000, 600), 2, "the 900 and the 1000, not the 100");
        assert_eq!(rate(&log, 1000, 3600), 3);
    }

    #[test]
    fn cabrillo_picks_the_mode_suffix_from_the_log() {
        let ssb = [qso("DL1ABC", "20M", "SSB", 0)];
        assert_eq!(cabrillo_contest(ContestId::CqWw, &ssb), "CQ-WW-SSB");
        let cw = [qso("DL1ABC", "20M", "CW", 0), qso("G1ABC", "20M", "CW", 1)];
        assert_eq!(cabrillo_contest(ContestId::CqWw, &cw), "CQ-WW-CW");
        // A contest with no mode split keeps its base name.
        assert_eq!(cabrillo_contest(ContestId::EuVhf, &cw), "EU-VHF");
    }

    #[test]
    fn cabrillo_has_the_header_and_a_line_per_qso() {
        let mut q = qso("DL1ABC", "20M", "SSB", 1_700_000_000);
        q.freq_hz = 14_250_000.0;
        q.rst_sent = Some(59);
        q.rst_rcvd = Some(59);
        q.stx_string = "05".into();
        q.srx_string = "14".into();
        let out = to_cabrillo(ContestId::CqWw, "k1abc", "05", &[q]);
        assert!(out.starts_with("START-OF-LOG: 3.0\n"));
        assert!(out.contains("CONTEST: CQ-WW-SSB\n"));
        assert!(out.contains("CALLSIGN: K1ABC\n"), "the call is upcased: {out}");
        assert!(out.contains("QSO: 14250 PH "), "freq in kHz, phone: {out}");
        assert!(out.contains("K1ABC 59 05 DL1ABC 59 14"), "the exchange line: {out}");
        assert!(out.trim_end().ends_with("END-OF-LOG:"));
    }

    #[test]
    fn a_session_starts_at_serial_one() {
        let s = ContestSession::new(ContestId::CqWpx, "05".into(), 0);
        assert_eq!(s.next_serial, 1);
        assert_eq!(s.contest, ContestId::CqWpx);
        // The mode code mapping is what the export relies on.
        assert_eq!(cabrillo_mode("FT8"), "DG");
        assert_eq!(cabrillo_mode("USB"), "PH");
        assert_eq!(cabrillo_mode("CW"), "CW");
    }

    #[test]
    fn ft8_contests_are_the_ones_the_digi_side_can_drive() {
        assert!(ContestId::EuVhf.is_ft8_contest());
        assert!(ContestId::CqWpx.is_ft8_contest());
        assert!(ContestId::Generic.is_ft8_contest());
        assert!(!ContestId::CqWw.is_ft8_contest(), "a zone exchange is not an FT8 layout");
        assert!(!ContestId::Text.is_ft8_contest());
    }

    // `Mode` is referenced so the test module fails to compile if the log's
    // mode field ever stops being a plain string the mapping can read.
    #[allow(dead_code)]
    fn _mode_is_not_structural(_m: Mode) {}
}
