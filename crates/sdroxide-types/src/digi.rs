//! FT8/FT4 digital-mode domain types, shared by the native engine, the
//! wire protocol, and the UI (native + WASM). Pure data + serde + pure
//! formatters — no mfsk-core here (that GPL dependency lives only in the
//! native `sdroxide-digi` crate).

use serde::{Deserialize, Serialize};

use crate::Mode;

/// One decoded FT8/FT4 message from a receive slot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Decode {
    /// Unix seconds at the start of the slot this was decoded from.
    pub slot_utc: i64,
    /// WSJT-X-compatible SNR estimate (dB).
    pub snr_db: i16,
    /// Time offset from the nominal slot start (seconds).
    pub dt: f32,
    /// Audio tone offset within the passband (Hz, ~200..3000).
    pub audio_hz: f32,
    /// Full decoded message text, e.g. "CQ AB1CD FN42".
    pub message: String,
    /// Parsed recipient callsign ("CQ" appears as `is_cq`, not here).
    pub to: Option<String>,
    /// Parsed sender callsign.
    pub from: Option<String>,
    /// Parsed 4-char grid, if the payload was a grid locator.
    pub grid: Option<String>,
    /// True when the message is a CQ call.
    pub is_cq: bool,
}

/// Where a QSO is in the standard FT8/FT4 exchange.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QsoStep {
    Idle,
    /// We picked a non-CQ station and are holding until they call CQ (or call
    /// us) before we start transmitting, so we don't jump into their exchange.
    WaitCq,
    /// We are calling CQ, waiting for an answer.
    CallingCq,
    /// (Answerer) we replied to a CQ with our grid, awaiting their report.
    TxGrid,
    /// We are sending them a signal report.
    TxReport,
    /// We are sending R + their report.
    TxRReport,
    /// We are sending RR73.
    TxRr73,
    /// We are sending 73.
    Tx73,
    /// The exchange is complete and logged, but we keep the contact live for a
    /// few minutes and re-send our final message if the DX repeats theirs (i.e.
    /// they didn't receive our 73 / RR73).
    Confirming,
    /// QSO finished (logged).
    Done,
}

impl QsoStep {
    pub fn label(self) -> &'static str {
        match self {
            QsoStep::Idle => "Idle",
            QsoStep::WaitCq => "Wait CQ",
            QsoStep::CallingCq => "Calling CQ",
            QsoStep::TxGrid => "Tx Grid",
            QsoStep::TxReport => "Tx Report",
            QsoStep::TxRReport => "Tx R+Report",
            QsoStep::TxRr73 => "Tx RR73",
            QsoStep::Tx73 => "Tx 73",
            QsoStep::Confirming => "Confirming",
            QsoStep::Done => "Done",
        }
    }
}

/// One line of the current QSO's message exchange.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptLine {
    /// True = we transmitted it, false = we received it.
    pub tx: bool,
    pub text: String,
}

/// Live status of the digital-mode engine, broadcast to clients.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DigiStatus {
    pub mode: Mode,
    pub step: QsoStep,
    /// The station we're working, if any.
    pub dx_call: Option<String>,
    pub dx_grid: Option<String>,
    /// Whether we'll key the next eligible slot.
    pub tx_next: bool,
    /// The exact message text queued for the next transmission.
    pub tx_pending_msg: Option<String>,
    /// Our transmit tone offset (Hz).
    pub audio_hz: f32,
    /// Which slot period we transmit in (true = even minute-second).
    pub tx_even: bool,
    /// True while a burst is currently on the air.
    pub transmitting: bool,
    /// The current QSO's message exchange (empty when idle).
    pub transcript: Vec<TranscriptLine>,
    /// Current engine config (so a fresh client can populate its editor).
    pub config: DigiConfig,
    /// Continuous keyboard modes (PSK/RTTY): the rolling decoded RX text.
    #[serde(default)]
    pub text_rx: String,
    /// Continuous keyboard modes: how many characters of the operator's
    /// outgoing buffer have been transmitted (drives the green "sent" cursor).
    #[serde(default)]
    pub tx_sent: usize,
    /// FSQ directed layer: callsigns recently heard, most-recent first.
    #[serde(default)]
    pub fsq_heard: Vec<String>,
    /// FSQ directed layer: parsed directed/allcall messages (rolling, capped).
    #[serde(default)]
    pub fsq_messages: Vec<FsqMsg>,
}

/// One parsed FSQ directed (or ALLCALL) message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FsqMsg {
    /// Sender callsign (empty if it couldn't be parsed).
    pub from: String,
    /// Addressee: a callsign, `allcall`, or empty for undirected.
    pub to: String,
    /// The message body (after the `:` trigger).
    pub text: String,
    /// True when the message is addressed to this station (or ALLCALL).
    pub to_me: bool,
}

impl DigiStatus {
    /// An idle status carrying just the operator config — emitted at engine
    /// startup so a client can seed its config editor before any digital mode is
    /// entered.
    pub fn idle(config: DigiConfig) -> Self {
        DigiStatus {
            mode: Mode::Usb,
            step: QsoStep::Idle,
            dx_call: None,
            dx_grid: None,
            tx_next: false,
            tx_pending_msg: None,
            audio_hz: 1500.0,
            tx_even: config.tx_even,
            transmitting: false,
            transcript: Vec::new(),
            config,
            text_rx: String::new(),
            tx_sent: 0,
            fsq_heard: Vec::new(),
            fsq_messages: Vec::new(),
        }
    }
}

/// A completed QSO, for the persistent logbook (digital or manual entry).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct QsoRecord {
    /// Stable logbook id (0 = unassigned; the UI assigns on first store).
    pub id: u64,
    pub call: String,
    pub grid: Option<String>,
    /// Signal report we sent them (FT8 dB, or RST like 59/599 for voice).
    pub rst_sent: Option<i16>,
    /// Signal report they sent us.
    pub rst_rcvd: Option<i16>,
    /// RF frequency (dial + audio) at log time.
    pub freq_hz: f64,
    pub mode: String,
    pub band: String,
    pub start_utc: i64,
    pub end_utc: i64,
    pub my_call: String,
    pub my_grid: String,
    /// Free-text note (manual entries, corrections).
    pub comment: String,
}

/// Operator configuration for digital-mode operation. Persisted engine-side,
/// THOR (DominoEX-family) submode: sets the symbol rate. All use 18 tones with
/// incremental frequency keying (IFK+) and convolutional FEC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThorMode {
    Thor4,
    Thor8,
    Thor11,
    #[default]
    Thor16,
    Thor22,
    Thor32,
}

impl ThorMode {
    pub const ALL: [ThorMode; 6] = [
        ThorMode::Thor4,
        ThorMode::Thor8,
        ThorMode::Thor11,
        ThorMode::Thor16,
        ThorMode::Thor22,
        ThorMode::Thor32,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ThorMode::Thor4 => "THOR4",
            ThorMode::Thor8 => "THOR8",
            ThorMode::Thor11 => "THOR11",
            ThorMode::Thor16 => "THOR16",
            ThorMode::Thor22 => "THOR22",
            ThorMode::Thor32 => "THOR32",
        }
    }

    /// Nominal symbol rate (baud). The modem derives the tone spacing from this.
    pub fn baud(self) -> f32 {
        match self {
            ThorMode::Thor4 => 3.90625,
            ThorMode::Thor8 => 7.8125,
            ThorMode::Thor11 => 10.766,
            ThorMode::Thor16 => 15.625,
            ThorMode::Thor22 => 21.53,
            ThorMode::Thor32 => 31.25,
        }
    }
}

/// echoed to clients in [`DigiStatus`]. `#[serde(default)]` so an older
/// `digi.json` without the newer fields still loads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DigiConfig {
    pub my_call: String,
    pub my_grid: String,
    /// Default transmit period for CQ (true = even).
    pub tx_even: bool,
    /// Auto-advance the QSO through its steps (vs. manual button presses).
    pub auto_seq: bool,
    // Message templates. Placeholders: {MYCALL} {MYGRID} {DX} {REPORT}.
    pub msg_cq: String,
    pub msg_grid: String,
    pub msg_report: String,
    pub msg_rreport: String,
    pub msg_rr73: String,
    pub msg_73: String,
    /// RTTY baud rate (45.45 / 50 / 75).
    pub rtty_baud: f32,
    /// RTTY frequency shift in Hz (170 / 425 / 850).
    pub rtty_shift_hz: f32,
    /// Olivia tone count (2 / 4 / 8 / 16 / 32 / 64).
    pub olivia_tones: u8,
    /// Olivia bandwidth in Hz (125 / 250 / 500 / 1000 / 2000).
    pub olivia_bw_hz: f32,
    /// THOR submode (symbol rate).
    pub thor_mode: ThorMode,
    /// FSQ speed / baud (2 / 3 / 4.5 / 6).
    pub fsq_baud: f32,
    /// FSQ station callsign for directed (FSQCALL) messaging. Falls back to
    /// `my_call` when empty.
    pub fsq_call: String,
    /// Keyboard-mode decode squelch (0 = open/decode everything, 1 = only strong
    /// signals). Suppresses decoding of pure noise when no signal is present.
    pub digi_squelch: f32,
    /// SSTV transmit clock trim in parts-per-million. Stretches (+) or compresses
    /// (−) the image time-scale to null out slant against a receiver whose sound-
    /// card clock differs from this station's. 0 = no correction.
    pub sstv_tx_ppm: f32,
    /// RF Paint scan speed as a fraction of the base rate (1.0 = base/fastest,
    /// 0.25 = default = quarter speed / 4× slower). Lower scans the text/image
    /// more slowly, giving the receiver's waterfall more lines to render it.
    pub rf_paint_speed: f32,
}

impl Default for DigiConfig {
    fn default() -> Self {
        DigiConfig {
            my_call: String::new(),
            my_grid: String::new(),
            tx_even: true,
            auto_seq: true,
            msg_cq: "CQ {MYCALL} {MYGRID}".into(),
            msg_grid: "{DX} {MYCALL} {MYGRID}".into(),
            msg_report: "{DX} {MYCALL} {REPORT}".into(),
            msg_rreport: "{DX} {MYCALL} R{REPORT}".into(),
            msg_rr73: "{DX} {MYCALL} RR73".into(),
            msg_73: "{DX} {MYCALL} 73".into(),
            rtty_baud: 45.45,
            rtty_shift_hz: 170.0,
            olivia_tones: 32,
            olivia_bw_hz: 1000.0,
            thor_mode: ThorMode::Thor16,
            fsq_baud: 4.5,
            fsq_call: String::new(),
            digi_squelch: 0.35,
            sstv_tx_ppm: 0.0,
            rf_paint_speed: 0.25,
        }
    }
}

impl DigiConfig {
    /// Fill a template's placeholders. `report` is a signed dB value.
    pub fn fill(template: &str, my_call: &str, my_grid: &str, dx: &str, report: Option<i16>) -> String {
        let rpt = report.map(fmt_report).unwrap_or_default();
        template
            .replace("{MYCALL}", my_call)
            .replace("{MYGRID}", my_grid)
            .replace("{DX}", dx)
            .replace("{REPORT}", &rpt)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Format an SNR as an FT8 report token: `-13`, `+02`, `+00`.
pub fn fmt_report(db: i16) -> String {
    if db < 0 {
        format!("-{:02}", -db)
    } else {
        format!("+{:02}", db)
    }
}

/// Which band a frequency falls in, as an ADIF band string (e.g. "20m").
pub fn adif_band(freq_hz: f64) -> &'static str {
    let mhz = freq_hz / 1e6;
    match mhz {
        m if m < 2.0 => "160m",
        m if m < 4.0 => "80m",
        m if m < 5.5 => "60m",
        m if m < 7.3 => "40m",
        m if m < 10.5 => "30m",
        m if m < 14.5 => "20m",
        m if m < 18.2 => "17m",
        m if m < 21.5 => "15m",
        m if m < 25.0 => "12m",
        m if m < 29.8 => "10m",
        m if m < 54.0 => "6m",
        _ => "2m",
    }
}

// ── Log formatters (pure, unit-tested; run on any client, native or wasm) ──

/// Split a Unix timestamp into UTC `(year, month, day, hour, min, sec)`.
pub fn utc_ymd_hms(unix: i64) -> (i64, u32, u32, u32, u32, u32) {
    // Civil-from-days (Howard Hinnant's algorithm), no chrono dependency.
    let days = unix.div_euclid(86_400);
    let secs = unix.rem_euclid(86_400);
    let (h, mi, s) = ((secs / 3600) as u32, ((secs % 3600) / 60) as u32, (secs % 60) as u32);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if m <= 2 { y + 1 } else { y };
    (year, m, d, h, mi, s)
}

/// Inverse of [`utc_ymd_hms`]: a UTC civil date/time to a Unix timestamp
/// (days-from-civil, Howard Hinnant's algorithm). Inputs are clamped-ish by
/// the caller; out-of-range months/days still produce a deterministic value.
pub fn ymd_hms_to_unix(y: i64, m: u32, d: u32, h: u32, mi: u32, s: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if m > 2 { m as i64 - 3 } else { m as i64 + 9 };
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    days * 86_400 + h as i64 * 3600 + mi as i64 * 60 + s as i64
}

fn adif_date_time(unix: i64) -> (String, String) {
    let (y, m, d, h, mi, s) = utc_ymd_hms(unix);
    (format!("{y:04}{m:02}{d:02}"), format!("{h:02}{mi:02}{s:02}"))
}

fn adif_field(name: &str, value: &str) -> String {
    format!("<{}:{}>{}", name, value.len(), value)
}

/// Render a session's QSOs as an ADIF (.adi) document importable into
/// standard logging software.
pub fn qso_log_to_adif(records: &[QsoRecord]) -> String {
    let mut out = String::from(
        "ADIF export from sdroxide\n<ADIF_VER:5>3.1.4\n<PROGRAMID:8>sdroxide\n<EOH>\n",
    );
    for r in records {
        let (date, time) = adif_date_time(r.start_utc);
        let (_, time_off) = adif_date_time(r.end_utc);
        out.push_str(&adif_field("CALL", &r.call));
        out.push_str(&adif_field("QSO_DATE", &date));
        out.push_str(&adif_field("TIME_ON", &time));
        out.push_str(&adif_field("TIME_OFF", &time_off));
        out.push_str(&adif_field("BAND", &r.band));
        out.push_str(&adif_field("MODE", &r.mode));
        out.push_str(&adif_field("FREQ", &format!("{:.6}", r.freq_hz / 1e6)));
        if let Some(g) = &r.grid {
            out.push_str(&adif_field("GRIDSQUARE", g));
        }
        if let Some(s) = r.rst_sent {
            out.push_str(&adif_field("RST_SENT", &s.to_string()));
        }
        if let Some(s) = r.rst_rcvd {
            out.push_str(&adif_field("RST_RCVD", &s.to_string()));
        }
        out.push_str(&adif_field("STATION_CALLSIGN", &r.my_call));
        out.push_str(&adif_field("MY_GRIDSQUARE", &r.my_grid));
        out.push_str("<EOR>\n");
    }
    out
}

/// Render a session's QSOs as a human-readable text log.
pub fn qso_log_to_text(records: &[QsoRecord]) -> String {
    let mut out = String::from(
        "sdroxide QSO log\nUTC date/time        call       grid  freq(MHz)  mode  sent rcvd\n",
    );
    for r in records {
        let (date, time) = adif_date_time(r.start_utc);
        let d = format!("{}-{}-{} {}:{}:{}", &date[0..4], &date[4..6], &date[6..8],
            &time[0..2], &time[2..4], &time[4..6]);
        out.push_str(&format!(
            "{:19}  {:10} {:5} {:10.6}  {:4}  {:>4} {:>4}\n",
            d,
            r.call,
            r.grid.as_deref().unwrap_or("-"),
            r.freq_hz / 1e6,
            r.mode,
            r.rst_sent.map(|v| v.to_string()).unwrap_or_else(|| "-".into()),
            r.rst_rcvd.map(|v| v.to_string()).unwrap_or_else(|| "-".into()),
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_formatting() {
        assert_eq!(fmt_report(-13), "-13");
        assert_eq!(fmt_report(2), "+02");
        assert_eq!(fmt_report(0), "+00");
    }

    #[test]
    fn template_fill_collapses_spaces() {
        let s = DigiConfig::fill("{DX} {MYCALL} {REPORT}", "AB1CD", "FN42", "W9XYZ", Some(-9));
        assert_eq!(s, "W9XYZ AB1CD -09");
        let cq = DigiConfig::fill("CQ {MYCALL} {MYGRID}", "AB1CD", "FN42", "", None);
        assert_eq!(cq, "CQ AB1CD FN42");
    }

    #[test]
    fn utc_conversion_matches_known_epoch() {
        // 2021-01-01 00:00:00 UTC = 1609459200
        assert_eq!(utc_ymd_hms(1_609_459_200), (2021, 1, 1, 0, 0, 0));
        // 2023-11-14 22:13:20 UTC = 1700000000
        assert_eq!(utc_ymd_hms(1_700_000_000), (2023, 11, 14, 22, 13, 20));
    }

    #[test]
    fn adif_has_required_fields() {
        let rec = QsoRecord {
            call: "W9XYZ".into(),
            grid: Some("EM48".into()),
            rst_sent: Some(-9),
            rst_rcvd: Some(-12),
            freq_hz: 14_074_000.0,
            mode: "FT8".into(),
            band: "20m".into(),
            start_utc: 1_609_459_200,
            end_utc: 1_609_459_260,
            my_call: "AB1CD".into(),
            my_grid: "FN42".into(),
            ..Default::default()
        };
        let adif = qso_log_to_adif(&[rec]);
        assert!(adif.contains("<CALL:5>W9XYZ"));
        assert!(adif.contains("<QSO_DATE:8>20210101"));
        assert!(adif.contains("<BAND:3>20m"));
        assert!(adif.contains("<MODE:3>FT8"));
        assert!(adif.contains("<GRIDSQUARE:4>EM48"));
        assert!(adif.contains("<EOR>"));
        assert_eq!(adif_band(14_074_000.0), "20m");
    }

    #[test]
    fn time_round_trips() {
        for &t in &[0i64, 1_609_459_260, 1_753_050_960, 2_000_000_000] {
            let (y, mo, d, h, mi, s) = utc_ymd_hms(t);
            assert_eq!(ymd_hms_to_unix(y, mo, d, h, mi, s), t, "round-trip {t}");
        }
        // A known civil date: 2021-01-01 00:01:00 UTC.
        assert_eq!(ymd_hms_to_unix(2021, 1, 1, 0, 1, 0), 1_609_459_260);
    }
}
