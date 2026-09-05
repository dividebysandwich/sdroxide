//! FT8/FT4 digital-mode domain types, shared by the native engine, the
//! wire protocol, and the UI (native + WASM). Pure data + serde + pure
//! formatters — no mfsk-core here (that GPL dependency lives only in the
//! native `sdroxide-digi` crate).

use serde::{Deserialize, Serialize};

use crate::entity::{resolve_callsign, resolve_prefix};
use crate::geo::grid_distance_km;
use crate::{Mode, RifpEncoding, RifpProfile, RifpSize};

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
    /// The modifier on a directed CQ, uppercased: the token between `CQ` and
    /// the caller's callsign. `DX`, a continent (`EU`, `NA`, `AS`…), a country
    /// prefix (`JA`, `DL`), or an activity (`POTA`, `TEST`). `None` for a plain
    /// CQ, which is open to everyone. See [`cq_is_for_us`].
    #[serde(default)]
    pub cq_to: Option<String>,
    /// True for a 13-character free-text message. It carries no addressing at
    /// all — `to` and `from` are always `None`, however much the text may look
    /// like an exchange.
    #[serde(default)]
    pub free_text: bool,
    /// DXpedition (Fox) layout only: the station whose contact the Fox is
    /// closing with `RR73` ("K1ABC RR73; W9XYZ <DX1FOX> +03" → `K1ABC`).
    ///
    /// It rides beside `to`/`from`, which name the *other* half of that message
    /// (the station being worked now). A Hound learns its QSO is complete from
    /// this field and nowhere else — the RR73 addressed to it is never in `to`.
    #[serde(default)]
    pub rr73_to: Option<String>,
}

/// How the FT8/FT4 decode list orders the stations — within each turn, or
/// across the whole list when the single-list view is on.
///
/// A view preference and nothing more, like [`crate::MemorySort`]: the decodes
/// arrive in the order they were decoded and every screen draws them the way
/// its own operator asked for. Persisted in `[ui]` with the rest of this
/// screen's preferences.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DecodeSort {
    /// Strongest signal first.
    Signal,
    /// Farthest (DX) first.
    Distance,
    /// Grouped by DXCC entity, alphabetically — what a band opening looks like
    /// when you are counting countries rather than reading callsigns.
    Country,
    /// As received (no reordering).
    ///
    /// Declared last and `#[serde(other)]` for the reason [`crate::MemorySort`]
    /// is: a typo in a hand-edited `config.toml` costs the operator this one
    /// preference rather than throwing the whole `[ui]` table — theme, fonts,
    /// layout and all — away.
    #[default]
    #[serde(other)]
    None,
}

impl DecodeSort {
    /// Every order, as the chips offer them.
    pub const ALL: [DecodeSort; 4] =
        [DecodeSort::None, DecodeSort::Signal, DecodeSort::Distance, DecodeSort::Country];

    pub fn label(self) -> &'static str {
        match self {
            DecodeSort::None => "None",
            DecodeSort::Signal => "SNR",
            DecodeSort::Distance => "Dist",
            DecodeSort::Country => "Country",
        }
    }
}

/// How far away a station has to be to count as DX when the DXCC entity can't
/// be resolved from either callsign — a rough stand-in for "another country".
const DX_FALLBACK_KM: f64 = 3000.0;

/// The continent codes cty.dat uses, which are also what a `CQ EU` names.
const CONTINENTS: [&str; 7] = ["NA", "SA", "EU", "AF", "AS", "OC", "AN"];

/// CQ modifiers that name an *activity* rather than a place: anyone may answer
/// one, wherever they are.
///
/// The list exists because several of these collide with real callsign
/// prefixes — `FD` (Field Day) begins like France, `WW` (CQ WW) like the United
/// States, `RU` (ARRL RTTY Roundup) like Russia — and reading them
/// geographically would hide contest CQs from everybody outside one country.
const ACTIVITY_CQ: [&str; 11] =
    ["POTA", "SOTA", "WWFF", "IOTA", "TEST", "QRP", "FD", "WW", "RU", "SKCC", "DIG"];

/// True when a decoded CQ is one *we* may answer.
///
/// A plain CQ is open to everyone. A directed one names who it wants, and the
/// UI neither colours nor lists under "CQ only" a call we would only be
/// answering out of turn:
///
/// * `CQ DX` wants stations outside the caller's own DXCC entity.
/// * `CQ EU`, `CQ NA`, … want a continent.
/// * `CQ JA`, `CQ DL`, … want a country, named by its prefix.
/// * `CQ POTA`, `CQ TEST`, … want a kind of contact, not a place, so they are
///   open to anyone.
///
/// Every test fails *open*: when the entity can't be resolved on both sides we
/// fall back to the great-circle distance between the grids, and where even
/// that is unavailable the call is treated as open. Showing a CQ we can't judge
/// beats hiding one we could have worked.
pub fn cq_is_for_us(d: &Decode, my_call: &str, my_grid: &str) -> bool {
    if !d.is_cq {
        return false;
    }
    let Some(dir) = d.cq_to.as_deref() else { return true };
    let mine = resolve_callsign(my_call);
    match dir {
        "DX" => {
            let Some(from) = d.from.as_deref() else { return true };
            match (resolve_callsign(from), mine) {
                // The DXCC entity is what "DX" means on HF: same entity → local.
                (Some(theirs), Some(mine)) => theirs.name != mine.name,
                _ => match d.grid.as_deref().and_then(|g| grid_distance_km(my_grid, g)) {
                    Some(km) => km >= DX_FALLBACK_KM,
                    None => true,
                },
            }
        }
        c if CONTINENTS.contains(&c) => mine.is_none_or(|m| m.continent == c),
        a if ACTIVITY_CQ.contains(&a) => true,
        // A country prefix. It is ours when the entity it names is the entity
        // our own callsign belongs to — "CQ JA" from anywhere is for every
        // Japanese station, however that station's own prefix is written.
        pfx => match (resolve_prefix(pfx), mine) {
            (Some(wanted), Some(mine)) => wanted.name == mine.name,
            _ => true,
        },
    }
}

/// Which side of an FT8 DXpedition-mode pile-up this station is operating.
///
/// DXpedition mode is FT8's answer to a rare-entity pile-up: one *Fox* works up
/// to five *Hounds* at a time, transmitting several signals at once in the low
/// part of the passband, always in the same (even) period. Hounds call from
/// above 1000 Hz and, once the Fox comes back to them, move down onto the Fox's
/// own frequency to finish. Both roles are FT8-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DxpedMode {
    /// Ordinary FT8/FT4 operation; neither side of a DXpedition pile-up.
    #[default]
    Normal,
    /// Calling a DXpedition station running Fox mode.
    Hound,
    /// Running the pile-up: several simultaneous signals, a queue of callers.
    Fox,
}

/// DXpedition mode splits the passband in two: the Fox transmits its signals
/// below this audio offset, and Hounds call above it, so the pile-up never
/// lands on top of the one station everybody is trying to work. A Hound crosses
/// into the Fox's half only after the Fox has answered it, to finish the
/// contact on the Fox's own frequency.
pub const FOX_ZONE_MAX_HZ: f32 = 1000.0;
/// Top of the Hound calling zone — the practical upper edge of an FT8 passband.
/// Display only; nothing refuses to transmit above it.
pub const HOUND_ZONE_MAX_HZ: f32 = 3000.0;
/// Most signals a Fox may transmit at once (WSJT-X's limit).
pub const FOX_MAX_SLOTS: u8 = 5;

/// Centre of the standard amateur RTTY tone pair, in Hz above the dial.
///
/// Everybody's tones are 2125 and 2295 Hz, and at the 170 Hz amateur shift this
/// centre is exactly that pair. High in the audio passband on purpose: it is
/// where every other RTTY program puts them, so a station tuned by ear against
/// one of those decodes here too, and both tones clear a transmit filter that
/// rolls off below 300 Hz. A wider shift keeps the same centre and simply
/// spreads about it — 850 Hz gives 1785/2635, still inside the passband.
///
/// The pair is a standard rather than a slot, which is why it does not move
/// when a signal is clicked and why the per-band transmit offsets do not touch
/// it — see [`Mode::holds_standard_tones`](crate::Mode::holds_standard_tones).
pub const RTTY_CENTER_HZ: f32 = 2210.0;

/// Where a NAVTEX signal's tone pair sits above the dial, in Hz.
///
/// The service's channel frequencies — 518, 490 and 4209.5 kHz — are the
/// *assigned* frequency, and for an F1B emission that is the centre of the two
/// tones rather than either of them. So a receiver in upper sideband tunes
/// 1700 Hz below the channel and the tones land at 1615 and 1785 Hz, which is
/// where the decoder looks for them. Fixed by the standard: unlike a keyboard
/// mode's audio offset, there is nothing here for an operator to choose.
pub const NAVTEX_TONE_HZ: f32 = 1700.0;

/// A "special operating activity": a contest whose exchange is not the
/// everyday grid-and-report, so the slotted modes have to send and read
/// something else (issue #223).
///
/// One entry so far. WSJT-X offers six, and each is its own 77-bit message
/// layout with its own sequence and its own log fields — ARRL Field Day
/// (`i3.n3 = 0.3`/`0.4`), the ARRL RTTY Roundup (`i3 = 3`), NA VHF and WW Digi
/// (standard messages carrying a grid where the report goes). Naming the enum
/// rather than a bare `eu_vhf` flag is what leaves room for them; adding one is
/// a variant here, its messages in `sdroxide_digi::qso` and its packing in
/// `sdroxide_digi::modem`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ContestMode {
    /// No contest: the everyday exchange.
    #[default]
    None,
    /// European VHF contests: a signal report, a serial number and a
    /// **6-character** locator, exchanged in the `i3 = 5` layout.
    EuVhf,
}

impl ContestMode {
    pub const ALL: [ContestMode; 2] = [ContestMode::None, ContestMode::EuVhf];

    pub fn label(self) -> &'static str {
        match self {
            ContestMode::None => "None",
            ContestMode::EuVhf => "EU VHF Contest",
        }
    }

    /// What a CQ in this activity says after "CQ" — the word that tells the
    /// band which contest is being called, and which every other program
    /// recognises. Empty outside a contest.
    pub fn cq_word(self) -> &'static str {
        match self {
            ContestMode::None => "",
            ContestMode::EuVhf => "TEST",
        }
    }
}

/// The signal report an EU VHF contest exchange carries, as the two digits of
/// an RS: `5{n}` for `n` in 2..=9, from the measured signal-to-noise ratio.
///
/// WSJT-X's own arithmetic (`mainwindow.cpp`: `nn = (snr + 36) / 6`, clamped to
/// 2..9, giving `5{nn}9` of which the exchange keeps the first two digits), and
/// it has to be exactly that: the layout carries three bits for this field and
/// reads them back as `52 + n`, so a value outside the range is not a rounding
/// difference but a message the far end cannot unpack.
/// The largest serial number an EU VHF contest exchange can carry: the layout
/// gives the field eleven bits. Past it the count wraps back to 1 rather than
/// sticking, because sending 2047 for the rest of a contest is worse than
/// starting again — a duplicate serial is at least visibly one.
pub const CONTEST_SERIAL_MAX: u32 = 2047;

/// The serial number that follows `n`, wrapping at [`CONTEST_SERIAL_MAX`].
pub fn next_contest_serial(n: u32) -> u32 {
    if n >= CONTEST_SERIAL_MAX { 1 } else { n + 1 }
}

pub fn eu_vhf_rs(snr_db: i16) -> u8 {
    let nn = (snr_db + 36).div_euclid(6).clamp(2, 9);
    50 + nn as u8
}

impl DxpedMode {
    pub const ALL: [DxpedMode; 3] = [DxpedMode::Normal, DxpedMode::Hound, DxpedMode::Fox];

    pub fn label(self) -> &'static str {
        match self {
            DxpedMode::Normal => "Normal",
            DxpedMode::Hound => "Hound",
            DxpedMode::Fox => "Fox",
        }
    }
}

/// A station the operator has marked to be called, holding everything the
/// sequencer needs to open the contact without hearing them again.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueuedCall {
    pub call: String,
    pub grid: Option<String>,
    /// Their signal when we queued them — the report we open with.
    pub snr_db: i16,
    /// Their tone offset, so we answer where they were transmitting.
    pub audio_hz: f32,
    /// Hold silently until they call CQ (or call us) rather than opening on
    /// them. Decided when they were queued, exactly as the reply button decides
    /// it: a station mid-exchange with someone else is not free yet.
    pub wait_for_cq: bool,
}

/// One station in a Fox's pile-up, for the operator's queue display.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoxCaller {
    pub call: String,
    pub grid: Option<String>,
    /// Their signal at us — the report we send them.
    pub snr_db: i16,
    /// True once we have sent them a report and are running their contact;
    /// false while they are only waiting in the queue.
    pub working: bool,
}

/// How workable a station's clock offset is.
///
/// FT8 and FT4 depend on both ends agreeing where a slot begins. Being a little
/// out costs nothing; being a lot out is the commonest reason a station calls
/// all evening and nobody ever comes back, because its transmissions land
/// outside the window everyone else's decoder searches. The thresholds follow
/// WSJT-X's practical guidance rather than any hard decoder limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockHealth {
    /// Well inside tolerance.
    Good,
    /// Still decodable, but worth fixing — FT4's slot is half as long, so this
    /// hurts there first.
    Marginal,
    /// Far enough out that stations will fail to decode us.
    Bad,
}

pub fn clock_health(offset_s: f32) -> ClockHealth {
    match offset_s.abs() {
        d if d < 0.5 => ClockHealth::Good,
        d if d < 1.5 => ClockHealth::Marginal,
        _ => ClockHealth::Bad,
    }
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
        }
    }
}

/// One line of the current QSO's message exchange.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptLine {
    /// True = we transmitted it, false = we received it.
    pub tx: bool,
    pub text: String,
    /// True for a note about the station we called working *someone else* —
    /// overheard traffic, not part of our own exchange. The UI colours these
    /// differently so it's obvious they aren't talking to us.
    #[serde(default)]
    pub overheard: bool,
    /// True for the line that marks the contact complete and logged. The
    /// transcript stays on screen after a QSO ends — that is what makes it
    /// readable — so without a line saying so, a finished exchange and one
    /// still waiting for the other station look exactly alike.
    #[serde(default)]
    pub done: bool,
}

impl TranscriptLine {
    /// A message we transmitted.
    pub fn sent(text: impl Into<String>) -> Self {
        TranscriptLine { tx: true, text: text.into(), overheard: false, done: false }
    }

    /// A message we received as part of our exchange.
    pub fn rcvd(text: impl Into<String>) -> Self {
        TranscriptLine { tx: false, text: text.into(), overheard: false, done: false }
    }

    /// A note about the DX working another station.
    pub fn note(text: impl Into<String>) -> Self {
        TranscriptLine { tx: false, text: text.into(), overheard: true, done: false }
    }

    /// The contact is complete and in the log.
    pub fn complete(text: impl Into<String>) -> Self {
        TranscriptLine { tx: false, text: text.into(), overheard: false, done: true }
    }
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
    /// The transmit watchdog stopped the sequencer. Cleared by any operator
    /// action (calling CQ, replying, picking a message).
    #[serde(default)]
    pub tx_watchdog: bool,
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
    /// FSQ directed layer: stations recently heard, most-recent first.
    #[serde(default)]
    pub fsq_heard: Vec<FsqHeard>,
    /// FSQ directed layer: parsed directed/allcall messages (rolling, capped).
    #[serde(default)]
    pub fsq_messages: Vec<FsqMsg>,
    /// RADE digital voice: modem state, when that mode is active.
    #[serde(default)]
    pub rade: Option<RadeStatus>,
    /// AX.25 packet: channel and link state, when that mode is active.
    #[serde(default)]
    pub packet: Option<PacketStatus>,
    /// NAVTEX: the messages received and the live text, when that mode is
    /// active. `None` in every other mode, so the panel that draws it is its
    /// own "are we in NAVTEX?" test — the rule the modes above follow.
    #[serde(default)]
    pub navtex: Option<NavtexStatus>,
    /// APRS: the stations on the map, the messages, and the channel. `None`
    /// in every other mode, so the panel that draws it is its own "are we in
    /// APRS?" test — the same rule [`DigiStatus::js8`] follows.
    #[serde(default)]
    pub aprs: Option<Box<crate::AprsStatus>>,
    /// JS8: heard list, reassembled conversation and transmit-queue progress.
    /// `None` in every other mode, so the panel that renders it is its own
    /// "are we in JS8?" test.
    #[serde(default)]
    pub js8: Option<crate::Js8Status>,
    /// Fox mode: the pile-up, callers being worked first. Empty in every other
    /// role, so the panel showing it is its own "are we the Fox?" test.
    #[serde(default)]
    pub fox_queue: Vec<FoxCaller>,
    /// Stations the operator has marked to work, in the order they will be
    /// taken. The sequencer starts the next one as soon as it is free.
    #[serde(default)]
    pub call_queue: Vec<QueuedCall>,
    /// How far our slot timing sits from the stations we are hearing, in
    /// seconds. Positive means our clock runs ahead of theirs — we transmit
    /// early and everyone else appears late to us. `None` until enough decodes
    /// have arrived to say. See [`clock_health`].
    ///
    /// It measures the whole receive path, not the system clock alone: a slow
    /// audio or network chain adds to it the same way a fast clock does. Either
    /// way it is the offset stations on the air actually see.
    #[serde(default)]
    pub clock_offset_s: Option<f32>,
    /// CW: what the decoder is making of the signal under the cursor. `None` in
    /// every other mode, so the panel that renders it is its own mode test.
    #[serde(default)]
    pub cw: Option<CwStatus>,
    /// WSPR: where the beacon is in its two-minute cycle, and where hopping
    /// will take it next. `None` in every other mode, as `cw` and `js8` are.
    #[serde(default)]
    pub wspr: Option<crate::WsprStatus>,
    /// The contact in progress, beyond the callsign and grid above. `None`
    /// whenever no station is being worked. See [`QsoLive`].
    #[serde(default)]
    pub qso: Option<QsoLive>,
}

/// The running detail of the contact in progress: when it started and what has
/// been exchanged so far.
///
/// [`DigiStatus`] already names the station being worked (`dx_call`,
/// `dx_grid`); this is the rest of what only the sequencer knows. A client can
/// *almost* recover it by re-parsing `transcript`, which is exactly why it is
/// sent explicitly — two parsers for one exchange is two chances to disagree
/// about what was sent, and the transcript is written for a human to read.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct QsoLive {
    /// Unix seconds when the contact began — what becomes
    /// [`QsoRecord::start_utc`] once it is logged.
    pub started_utc: i64,
    /// The report we sent them, which is their signal at us.
    pub rpt_sent: Option<i16>,
    /// The report they sent us.
    pub rpt_rcvd: Option<i16>,
}

/// Live state of the CW decoder.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct CwStatus {
    /// The decoder is copying: its timing fit is good and holding steady.
    pub locked: bool,
    /// Sending speed it is reading, in words per minute.
    pub wpm: f32,
    /// Signal-to-noise referred to the 500 Hz bandwidth CW reports are quoted
    /// in, so it is comparable with what another operator would tell you.
    pub snr_db: f32,
    /// The tone actually being copied, in Hz above the dial — the operator's
    /// pitch plus whatever the decoder's AFC has pulled to stay on the signal.
    pub tone_hz: f32,
}

/// Live state of the RADE V1 modem.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct RadeStatus {
    /// The receiver is locked to a signal.
    pub sync: bool,
    /// SNR estimate in a 3 kHz noise bandwidth (meaningful while `sync`).
    pub snr_db: f32,
    /// Frequency offset of the received signal (meaningful while `sync`).
    pub freq_offset_hz: f32,
    /// Peak level of the decoded speech, 0..1 — drives the RX meter.
    pub rx_level: f32,
    /// End-of-over frames seen this session; a change means the far end
    /// finished an over.
    pub eoo_count: u64,
    /// Samples dropped between the engine and the decode thread. Should stay
    /// at zero; anything else means the machine can't keep up.
    pub dropped: u64,
}

/// The speed a packet station is running.
///
/// One `Mode` covers both VHF speeds because they differ only in the modem;
/// which of the two, and whether HF's 300 baud applies at all, is this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PacketBaud {
    /// 300 baud AFSK, 200 Hz shift — HF, on single sideband. Only meaningful
    /// with [`crate::Mode::PacketHf`].
    Hf300,
    /// 1200 baud Bell 202 — the VHF workhorse, and what most RMS Packet
    /// gateways answer on.
    #[default]
    Vhf1200,
    /// 9600 baud G3RUH. Needs a radio with a real data port: the mic and
    /// speaker path destroys it at both ends.
    Vhf9600,
}

impl PacketBaud {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            PacketBaud::Hf300 => "300",
            PacketBaud::Vhf1200 => "1200",
            PacketBaud::Vhf9600 => "9600",
        }
    }

    #[must_use]
    pub fn baud(self) -> f64 {
        match self {
            PacketBaud::Hf300 => 300.0,
            PacketBaud::Vhf1200 => 1200.0,
            PacketBaud::Vhf9600 => 9600.0,
        }
    }
}

/// One frame heard on the channel, for the monitor pane.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PacketHeard {
    /// Seconds since the Unix epoch.
    pub at: i64,
    pub from: String,
    pub to: String,
    /// Digipeater path, in order, empty when direct.
    pub via: Vec<String>,
    /// The frame type as a monitor would print it: `UI`, `SABM`, `I`, `RR`…
    pub kind: String,
    /// Printable payload, if the frame carried one.
    pub text: String,
    /// True when we sent it, so the pane can show both sides of a QSO.
    pub sent: bool,
}

/// Where one line of the terminal came from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PacketTermKind {
    /// The far end said it.
    #[default]
    Rx,
    /// We sent it.
    Tx,
    /// sdroxide said it: the link came up, the call was refused, the far end
    /// gave up. Not traffic, and coloured differently so it cannot be mistaken
    /// for something a BBS printed.
    Note,
}

/// One line of a connected-mode session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PacketTermLine {
    /// Seconds since the Unix epoch.
    pub at: i64,
    pub kind: PacketTermKind,
    pub text: String,
}

/// Who is driving the connected-mode link.
///
/// There is one link and one radio, so this is also the answer to "why was I
/// refused" — a question an operator otherwise has to guess at.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PacketLinkOwner {
    /// Nobody. The link is free for either.
    #[default]
    Idle,
    /// The operator, from the packet panel.
    Terminal,
    /// A Winlink forwarding session, from the MAIL window.
    Session,
}

/// The connected-mode link, as the panel needs to see it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PacketLink {
    /// The state machine's own name for where it is: `Disconnected`,
    /// `AwaitingConnection`, `Connected`, `TimerRecovery`, `AwaitingRelease`.
    /// Its word rather than a translation, so a transcript and a debug log
    /// describe the same thing.
    pub state: String,
    /// Who is at the far end, once there is a far end.
    pub peer: Option<String>,
    /// The digipeater path in use, in order. Empty for a direct link.
    pub via: Vec<String>,
    /// Extended (mod-128) sequence numbers.
    pub ext: bool,
    /// I frames sent and not yet acknowledged.
    pub unacked: u16,
    /// Bytes handed to the link that have not been made into frames yet.
    pub pending: u32,
    /// Retries used against N2. A count climbing while `unacked` stays put is
    /// what a fading path looks like from this side, and the only warning
    /// before the link gives up.
    pub retries: u8,
    pub owner: PacketLinkOwner,
}

/// What a packet station is doing.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PacketStatus {
    pub baud: PacketBaud,
    /// The channel is busy — a modem-level carrier detect, not a squelch.
    /// CSMA will not key while this is set.
    pub dcd: bool,
    /// Smoothed receive level, 0..1, for the meter.
    pub level: f32,
    /// Frames heard, newest last, capped.
    pub heard: Vec<PacketHeard>,
    /// Frames that arrived but failed their check sequence. A rising count
    /// against a steady `heard` is what a marginal path looks like.
    pub bad_frames: u32,
    /// The connected-mode link, when there is one.
    pub link: Option<PacketLink>,
    /// The terminal session, oldest first, capped.
    #[serde(default)]
    pub term: Vec<PacketTermLine>,
    /// The tail of a line that has arrived without its terminator.
    ///
    /// Carried apart from `term` because it is the most important thing on the
    /// screen: a BBS prompt has no CR after it, so a terminal that printed only
    /// whole lines would sit there showing nothing while the far end waited for
    /// an answer to a question the operator never saw.
    #[serde(default)]
    pub term_partial: String,
}

/// One NAVTEX broadcast, as its header names it.
///
/// The header is four characters after `ZCZC`: the transmitter, what kind of
/// message it is, and a serial number. Together they are what a receiver
/// *filters* on — a station that has already printed B1=`S`, B2=`A`, serial 42
/// does not print it again when it is repeated four hours later — so they are
/// parsed out rather than left in the text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NavtexMessage {
    /// B1: which transmitter sent it, `A`..`Z` — a letter allocated by the
    /// NAVAREA co-ordinator, and the only identification a NAVTEX broadcast
    /// carries.
    pub station: char,
    /// B2: the subject. `A` navigational warning, `B` meteorological warning,
    /// `C` ice report, `D` search and rescue, `E` meteorological forecast,
    /// `L` a further navigational warning, `Z` no messages on hand — among
    /// others. `A`, `B` and `D` may not be turned off by a ship's receiver,
    /// which is why they are the ones this program never hides either.
    pub kind: char,
    /// B3B4: the serial number, 01–99, `00` for a message that must always be
    /// printed.
    pub serial: u8,
    /// The body, as received, `*` where a character was lost.
    pub text: String,
    /// Unix time the header arrived.
    pub at: i64,
    /// Whether the closing `NNNN` was seen. A message that ends because the
    /// next one started, or because the signal went, is still worth showing —
    /// and worth marking, because half a gale warning is not a gale warning.
    pub complete: bool,
    /// Characters the FEC could not recover.
    pub lost: u32,
}

impl NavtexMessage {
    /// What the subject letter means, for the panel.
    #[must_use]
    pub fn kind_label(&self) -> &'static str {
        match self.kind {
            'A' => "Navigational warning",
            'B' => "Meteorological warning",
            'C' => "Ice report",
            'D' => "Search and rescue",
            'E' => "Meteorological forecast",
            'F' => "Pilot service",
            'G' => "AIS / DECCA",
            'H' => "LORAN",
            'I' => "Alpha",
            'J' => "SATNAV",
            'K' => "Other electronic navaid",
            'L' => "Navigational warning (additional)",
            'T' => "Test transmission",
            'V' | 'W' | 'X' | 'Y' => "Special service",
            'Z' => "No messages on hand",
            _ => "Unknown",
        }
    }

    /// Whether a ship's receiver is forbidden to reject this class. The three
    /// that cannot be switched off are the ones somebody's life may depend on,
    /// and sdroxide does not offer to hide them either.
    #[must_use]
    pub fn is_mandatory(&self) -> bool {
        matches!(self.kind, 'A' | 'B' | 'D')
    }
}

/// Messages kept. A station transmits on a ten-minute slot every four hours
/// and its neighbours fill the rest, so this is a day or two of a busy area.
pub const NAVTEX_MESSAGE_MAX: usize = 200;

/// What the NAVTEX receiver is doing.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct NavtexStatus {
    /// Whether the character phase is locked — the mode's own carrier detect.
    pub in_sync: bool,
    /// Smoothed level at the two tones, for a meter.
    pub level: f32,
    /// Messages received, newest last.
    pub messages: Vec<NavtexMessage>,
    /// The message being received now, if one is open.
    pub live: Option<NavtexMessage>,
    /// Everything decoded, whether or not it was inside a message — a coast
    /// station's phasing and its idle chatter included. The honest view when a
    /// header is missed.
    pub text: String,
    /// Characters taken straight, repaired from the repeat, and lost. The only
    /// quality figure a mode with no checksum has.
    pub direct: u64,
    pub repaired: u64,
    pub lost: u64,
    /// Whether the tones are being read the other way up.
    pub reverse: bool,
}

/// Most frames kept for the monitor pane. A busy VHF channel produces a few a
/// second, and the pane is a rolling view rather than a log.
pub const PACKET_HEARD_MAX: usize = 200;

/// Most lines kept in the terminal.
///
/// The whole status is cloned into every `DigiStatus` five times a second and
/// crosses the wire to remote clients, so the transcript is a rolling view like
/// the monitor above it rather than a session log. Lines are cut to
/// [`PACKET_TERM_LINE_MAX`] for the same reason.
pub const PACKET_TERM_MAX: usize = 200;

/// Longest terminal line kept. A BBS wraps at 80; a station sending more than
/// this without a terminator is not sending text a person is reading.
pub const PACKET_TERM_LINE_MAX: usize = 256;

/// One station on the FSQ heard list.
///
/// Stamped with when it was last heard, because FSQ's own heard list is only
/// ordered: it never drops a station, so without a time there is no way to tell
/// somebody who transmitted a minute ago from somebody who transmitted when the
/// receiver was first switched on. The map needs that difference to fade a
/// station out.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FsqHeard {
    /// Sender callsign, uppercased.
    pub call: String,
    /// Unix seconds when this station was last heard.
    pub last_utc: i64,
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
            tx_watchdog: false,
            transcript: Vec::new(),
            config,
            text_rx: String::new(),
            tx_sent: 0,
            fsq_heard: Vec::new(),
            fsq_messages: Vec::new(),
            rade: None,
            packet: None,
            navtex: None,
            aprs: None,
            js8: None,
            fox_queue: Vec::new(),
            call_queue: Vec::new(),
            clock_offset_s: None,
            cw: None,
            wspr: None,
            qso: None,
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

    // Extended fields: contesting, awards, QSL status. All
    // `#[serde(default)]` via the struct attribute, so older logs still load.
    /// Worked operator's name.
    pub name: String,
    /// Worked station's town / QTH.
    pub qth: String,
    /// Worked station's primary subdivision (US state, etc.).
    pub state: String,
    /// County (US), if known.
    pub county: String,
    /// DXCC entity / country name.
    pub country: String,
    /// DXCC entity number.
    pub dxcc: Option<u16>,
    /// CQ zone (1..40).
    pub cq_zone: Option<u8>,
    /// ITU zone (1..90).
    pub itu_zone: Option<u8>,
    /// Continent (NA/SA/EU/AF/AS/OC/AN).
    pub continent: String,
    /// IOTA reference (e.g. "EU-005").
    pub iota: String,
    /// Special activity group (POTA / SOTA / WWFF) and its reference — mapped to
    /// ADIF SIG / SIG_INFO.
    pub sig: String,
    pub sig_info: String,
    /// Transmit power in watts.
    pub tx_pwr: Option<f32>,
    /// Logging operator (when different from the station call).
    pub operator: String,
    /// Contest identifier (ADIF CONTEST_ID).
    pub contest_id: String,
    /// Received serial (contest).
    pub srx: Option<u32>,
    /// Sent serial (contest).
    pub stx: Option<u32>,
    /// Received exchange string (contest).
    pub srx_string: String,
    /// Sent exchange string (contest).
    pub stx_string: String,
    /// My station's subdivision / country / zones (contest + awards).
    pub my_state: String,
    pub my_country: String,
    pub my_dxcc: Option<u16>,
    pub my_cq_zone: Option<u8>,
    pub my_itu_zone: Option<u8>,

    // ── QSL / confirmation status ──
    pub lotw_sent: bool,
    pub lotw_rcvd: bool,
    pub eqsl_sent: bool,
    pub eqsl_rcvd: bool,
    /// Uploaded to the QRZ logbook.
    pub qrz_sent: bool,
    /// Uploaded to Club Log.
    pub clublog_sent: bool,
    /// Paper QSL card sent / received.
    pub qsl_sent: bool,
    pub qsl_rcvd: bool,
    /// QSL routing / manager.
    pub qsl_via: String,
    /// Uploaded to the HamQTH logbook. Appended after `qsl_via` rather than
    /// beside its `*_sent` siblings because this record rides the postcard
    /// wire, which reads it positionally.
    pub hamqth_sent: bool,
    /// Uploaded to the World Radio League logbook (issue #337). Appended for
    /// the reason [`Self::hamqth_sent`] gives.
    #[serde(default)]
    pub wrl_sent: bool,
}

impl QsoRecord {
    /// True when any confirmation (LoTW, eQSL, or paper card) has been received.
    pub fn is_confirmed(&self) -> bool {
        self.lotw_rcvd || self.eqsl_rcvd || self.qsl_rcvd
    }
}

/// True if `log` already contains a QSO with the same callsign on `band`
/// (optionally the same `mode`) — a "worked before" / contest-dupe check.
/// `mode` empty matches any mode. `exclude_id` skips the record being edited.
pub fn worked_before(
    log: &[QsoRecord],
    call: &str,
    band: &str,
    mode: &str,
    exclude_id: u64,
) -> bool {
    let call = call.trim();
    if call.is_empty() {
        return false;
    }
    log.iter().any(|q| {
        q.id != exclude_id
            && q.call.eq_ignore_ascii_case(call)
            && q.band.eq_ignore_ascii_case(band)
            && (mode.is_empty() || q.mode.eq_ignore_ascii_case(mode))
    })
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

/// Hellschreiber variant. Feld Hell is the classic on/off-keyed facsimile mode;
/// X5 and X9 speed it up, Slow Hell crawls for weak signals, and the FSK
/// variants keep the carrier up and shift it instead of keying it.
///
/// Rates follow fldigi's `feldcolumnrate` (character columns per second); every
/// variant scans 14 dot rows per column and 7 columns per character cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum HellVariant {
    #[default]
    Feld,
    Slow,
    X5,
    X9,
    Fsk245,
    Fsk105,
    Hell80,
}

impl HellVariant {
    /// Dot rows scanned per character column.
    pub const ROWS: usize = 14;
    /// Columns per character cell; the last is the inter-character gap.
    pub const CELL_COLS: usize = 7;

    pub const ALL: [HellVariant; 7] = [
        HellVariant::Feld,
        HellVariant::Slow,
        HellVariant::X5,
        HellVariant::X9,
        HellVariant::Fsk245,
        HellVariant::Fsk105,
        HellVariant::Hell80,
    ];

    pub fn label(self) -> &'static str {
        match self {
            HellVariant::Feld => "FELD",
            HellVariant::Slow => "SLOW",
            HellVariant::X5 => "X5",
            HellVariant::X9 => "X9",
            HellVariant::Fsk245 => "FSK245",
            HellVariant::Fsk105 => "FSK105",
            HellVariant::Hell80 => "HELL80",
        }
    }

    /// Character columns per second (fldigi's `feldcolumnrate`).
    pub fn column_rate(self) -> f64 {
        match self {
            HellVariant::Feld => 17.5,
            HellVariant::Slow => 2.1875,
            HellVariant::X5 => 87.5,
            HellVariant::X9 => 157.5,
            HellVariant::Fsk245 => 17.5,
            HellVariant::Fsk105 => 17.5,
            HellVariant::Hell80 => 35.0,
        }
    }

    /// Dots (transmitted pixels) per second — 14 rows in every column.
    pub fn pixel_rate(self) -> f64 {
        self.column_rate() * Self::ROWS as f64
    }

    /// Characters per second — 2.5 for classic Feld Hell.
    pub fn chars_per_sec(self) -> f64 {
        self.column_rate() / Self::CELL_COLS as f64
    }

    /// **Peak-to-peak** FSK shift in Hz; 0 for the on/off-keyed variants.
    ///
    /// This is fldigi's `hell_bandwidth`, which it applies as `tone ± value/2` —
    /// so it is the full shift, not the deviation.
    pub fn shift_hz(self) -> f64 {
        match self {
            HellVariant::Fsk245 => 122.5,
            HellVariant::Fsk105 => 55.0,
            HellVariant::Hell80 => 300.0,
            _ => 0.0,
        }
    }

    /// True for the frequency-shifted variants (continuous carrier); false for
    /// the on/off-keyed ones.
    pub fn is_fsk(self) -> bool {
        self.shift_hz() > 0.0
    }

    /// Nominal occupied bandwidth in Hz — fldigi's receive-filter width. Drives
    /// the receive filter, the waterfall markers, and the audio-centre clamp.
    pub fn bandwidth_hz(self) -> f64 {
        let raw = if self.is_fsk() { 4.0 * self.shift_hz() } else { 1.2 * self.pixel_rate() };
        5.0 * (raw / 5.0).round()
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
    /// RTTY frequency shift in Hz. 170 is the amateur norm; commercial and
    /// weather broadcasts commonly use 425/450 (Deutscher Wetterdienst) or 850.
    pub rtty_shift_hz: f32,
    /// Swap the RTTY mark and space tones. A signal received on the opposite
    /// sideband from the one it was sent on arrives inverted and decodes as
    /// nonsense until this is set — the "Reverse"/RV control other RTTY
    /// programs offer.
    pub rtty_reverse: bool,
    /// The same swap for NAVTEX, and needed for the same reason: the tones are
    /// fixed by the standard but which of them a receiver hears as a mark
    /// depends on the sideband it is listening on. Off by default, which is
    /// upper sideband on the channel frequency — the way every published
    /// tuning instruction for the service reads.
    #[serde(default)]
    pub navtex_reverse: bool,
    /// Let the RTTY decoder track tuning error rather than staying pinned to
    /// the cursor. On by default; the matched-filter detector is much less
    /// forgiving of mistuning than a wideband discriminator would be.
    pub rtty_afc: bool,
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
    /// Send the station's callsign in tones after every picture — the FSK ID
    /// that every SSTV program and unattended repeater reads (issue #287).
    ///
    /// A banner printed into the picture identifies the station to a *person*;
    /// this identifies it to a *machine*, which is what a repeater needs before
    /// it can log or announce who sent the frame. It costs about two and a half
    /// seconds after a transmission that has already taken a minute or two.
    ///
    /// Nothing is sent when there is no callsign in
    /// [`my_call`](Self::my_call), so a station that has not set one transmits
    /// exactly what it always did.
    #[serde(default = "yes")]
    pub sstv_fsk_id: bool,

    // ── The banner across the top of every transmitted picture ──
    //
    // On the station and not on the screen, unlike the waterfall's colours:
    // this is drawn *into* the picture that goes on the air, so it is the
    // station identifying itself, and every client composing a preview of the
    // same slot has to draw the same thing.
    /// Draw the banner strip at all. Off sends the picture and the slot's
    /// message with nothing over the top of them.
    #[serde(default = "yes")]
    pub sstv_banner: bool,
    /// What to print at the left end of the banner.
    ///
    /// Placeholders, substituted when the picture is composed:
    /// `{call}` — the operator callsign, uppercased; `{grid}` — the locator;
    /// `{version}` — the running sdroxide version, without a leading `v`.
    /// An unknown `{…}` is left alone rather than swallowed, so a typo shows
    /// up in the preview instead of silently printing nothing.
    #[serde(default = "sstv_default_banner_left")]
    pub sstv_banner_left: String,
    /// What to print at the right end of the banner, right-aligned. Same
    /// placeholders as [`sstv_banner_left`](Self::sstv_banner_left).
    #[serde(default = "sstv_default_banner_right")]
    pub sstv_banner_right: String,
    /// The banner's colour at its top edge (sRGB), fading to black at the
    /// bottom of the strip.
    #[serde(default = "sstv_default_banner_fill")]
    pub sstv_banner_fill: [u8; 3],
    /// The colour both texts are printed in (sRGB).
    #[serde(default = "sstv_default_banner_ink")]
    pub sstv_banner_ink: [u8; 3],
    /// Height of the strip in pixels of the transmitted picture. The text is
    /// sized from it, so this is the one control over how large the banner
    /// reads on the far end — worth turning up, since an SSTV frame is 320
    /// pixels wide and lands on the other operator's screen as a small window.
    #[serde(default = "sstv_default_banner_height")]
    pub sstv_banner_height: u16,
    /// RF Paint scan speed as a fraction of the base rate (1.0 = base/fastest,
    /// 0.25 = default = quarter speed / 4× slower). Lower scans the text/image
    /// more slowly, giving the receiver's waterfall more lines to render it.
    pub rf_paint_speed: f32,
    /// FT8/FT4: stop transmitting after this many minutes with no progress —
    /// no reply, no operator action. The guard against a station left calling
    /// into an empty band for hours. 0 disables it.
    pub tx_watchdog_min: u32,
    /// FT8/FT4: give up on a station after this many unanswered transmissions.
    /// Calling CQ is exempt (repeating a CQ is the point); this counts calls to
    /// one station that never comes back. 0 disables it.
    pub max_tx_repeats: u32,
    /// FT8/FT4: choose our transmit tone offset ourselves, rather than moving
    /// onto whichever station we are answering.
    ///
    /// Answering on the DX's own frequency is the obvious thing and the wrong
    /// one: they transmit in the opposite period to us, so their frequency
    /// tells us nothing about who is transmitting there when *we* do — and the
    /// station that is will not hear a word. With this on, the engine picks the
    /// quietest spot in the period we are about to transmit in, from the
    /// stations it has actually decoded there. Ignored in DXpedition mode, where
    /// both roles have their frequencies decided for them.
    ///
    /// Turning it OFF does not hold the frequency: it selects the other mover,
    /// answering on the frequency of the station being called. To hold, see
    /// [`hold_tx_freq`](Self::hold_tx_freq).
    pub auto_tx_freq: bool,
    /// FT8/FT4: never move the transmit tone by itself, whatever else asks.
    ///
    /// The third state the pair above cannot express. `auto_tx_freq` chooses
    /// *which* automatic mover runs, not whether one does: on, the engine hunts
    /// the quietest slot between 400 and 2600 Hz; off, it jumps onto whichever
    /// station is being answered. Both are wrong where the licence, and not the
    /// band plan, sets the ceiling.
    ///
    /// The case that earned it is UK 60 m. On a 5357 kHz dial the allocation
    /// ends at 5358.0, so the transmit tone must stay under 1 kHz, and either
    /// mover will walk out of the band unprompted between one over and the next.
    ///
    /// With this on nothing moves the tone: not answering a station, not the
    /// call queue walking on, not calling CQ, not a click on a decode or on the
    /// waterfall. Turn it off to move, then on again.
    ///
    /// Two exceptions, both of them moves the operator has effectively asked
    /// for. A Hound follows the Fox that answered it, which is the
    /// DXpedition's frequency to give and not ours to hold. And a change of
    /// band restores that band's own entry in
    /// [`tx_audio_hz`](Self::tx_audio_hz), because holding through a band
    /// change is what carries a licence-edge figure onto a band that does not
    /// want it.
    #[serde(default)]
    pub hold_tx_freq: bool,
    /// FT8/FT4: the transmit tone offset last chosen on each band, in Hz.
    ///
    /// Per band and not one figure for the station, because the constraint that
    /// makes an offset worth remembering belongs to the band rather than to the
    /// operator. UK 60 m holds the tone under 1 kHz; carrying that figure onto
    /// 20 m would sit us at the bottom of the passband for no reason, and
    /// carrying 20 m's usual 1500 back onto 60 m is out of band. WSJT-X
    /// remembers this by MODE instead, which does not help here: the edge is a
    /// property of where the dial is, not of what is being sent.
    ///
    /// Only the operator's own moves are recorded. An automatic hop (the
    /// quietest-slot hunt, or answering a station where it transmits) is the
    /// engine's choice for one over and not a preference to restore next time.
    ///
    /// A band with no entry starts at the mode's usual 1500 Hz.
    ///
    /// `HashMap` rather than `BTreeMap` because [`Band`](crate::Band) has no
    /// `Ord` and should not gain one: its declaration order is a postcard wire
    /// index with later bands appended out of place, so a derived ordering
    /// would read as frequency order without being it.
    #[serde(default)]
    pub tx_audio_hz: std::collections::HashMap<crate::Band, f32>,
    /// FT8: which side of a DXpedition pile-up to operate (see [`DxpedMode`]).
    /// Ignored in every other mode.
    pub dxped_mode: DxpedMode,
    /// The special operating activity the slotted modes are working, if any
    /// (see [`ContestMode`]). Ignored in every other mode.
    #[serde(default)]
    pub contest: ContestMode,
    /// The serial number the next contest exchange will carry.
    ///
    /// On the station rather than on the screen, and remembered: a contest is
    /// worked over a weekend and across restarts, and every client composing
    /// the same transmission has to compose the same number. Advanced by the
    /// engine as each contact is logged, and settable by hand — an operator who
    /// has been logging on paper starts where the paper got to.
    ///
    /// The `i3 = 5` layout carries eleven bits, so 1..=2047. Past that it wraps
    /// rather than being clipped to 2047 and sending the same number for the
    /// rest of the contest.
    #[serde(default = "one_u32")]
    pub contest_serial: u32,
    /// Fox mode: how many signals to transmit at once (1..=5, WSJT-X's limit).
    /// They are spaced 60 Hz apart starting at the transmit tone offset, and
    /// share the transmitter's power between them.
    pub fox_slots: u8,
    /// RADE: silence the demodulated (analog) audio, so only decoded speech is
    /// audible. Off by default — hearing the raw signal is how the operator
    /// tunes onto an over before the modem syncs.
    pub rade_mute_analog: bool,

    /// JS8: transmission speed. Normal is the band convention; Fast and Turbo
    /// trade sensitivity for latency, Slow the reverse. Changing it restarts
    /// the decoder, since every speed is a different waveform.
    #[serde(default)]
    pub js8_speed: crate::Js8Speed,
    /// JS8: answer SNR? / GRID? / HEARING? / STATUS? addressed to us or to
    /// @ALLCALL. What makes a station worth leaving switched on.
    #[serde(default = "yes")]
    pub js8_auto_reply: bool,
    /// JS8: send an automatic heartbeat every N minutes; 0 disables it.
    ///
    /// Off by default, deliberately. An automatic beacon that switches itself
    /// on when the operator picks a mode is an on-air behaviour nobody
    /// consented to.
    #[serde(default)]
    pub js8_heartbeat_min: u32,
    /// JS8: answer a heard heartbeat with a signal report, so the station
    /// beaconing learns who is copying them.
    ///
    /// Off by default, as upstream has it. This is the one auto-reply that
    /// answers something nobody asked: a busy band carries a heartbeat every
    /// slot, and a station that answered all of them would flood exactly the
    /// band heartbeats exist to keep quiet. The engine rate-limits it besides.
    #[serde(default)]
    pub js8_hb_ack: bool,
    /// JS8: beacon on the working frequency instead of moving to a free slot in
    /// the 500–1000 Hz heartbeat sub-band.
    ///
    /// Off by default, so heartbeats go where the band convention says they
    /// go — a beacon on top of somebody's QSO is exactly what the sub-band
    /// exists to prevent. Upstream calls the same switch "heartbeat anywhere".
    #[serde(default)]
    pub js8_hb_anywhere: bool,
    /// JS8: forget an incomplete multi-frame message after this many seconds.
    #[serde(default = "js8_default_timeout")]
    pub js8_assembly_timeout_s: u32,
    /// JS8: station callsign, falling back to `my_call` when empty. Mirrors
    /// `fsq_call`.
    #[serde(default)]
    pub js8_call: String,
    /// JS8: free-text status sent in reply to ` STATUS?`.
    #[serde(default)]
    pub js8_status: String,
    /// JS8: groups this station belongs to, so directed traffic to them counts
    /// as addressed to us.
    #[serde(default)]
    pub js8_groups: Vec<String>,
    /// Hellschreiber variant (Feld Hell / Slow / X5 / X9 / the FSK variants).
    pub hell_variant: HellVariant,
    /// Hellschreiber receive AGC speed: 0 = off (an absolute scale, meaningful
    /// because the digi tap is already post-AGC) … 1 = fast. Normalises the
    /// raster so a weak signal still paints legibly.
    ///
    /// Contrast, brightness and reverse video are deliberately *not* here: the
    /// panel keeps its own copy of the raw grays, so shading them client-side
    /// lets those controls repaint the whole scrollback rather than only the
    /// columns that arrive after the change.
    pub hell_rx_agc: f32,

    // ── RIFP (draft-dulaunoy-rifp-00) ──
    /// RIFP radio profile — the modulation the frames ride on.
    pub rifp_profile: RifpProfile,
    /// How the transmitted picture is encoded into the object bytes.
    pub rifp_encoding: RifpEncoding,
    /// Transmitted picture size.
    pub rifp_size: RifpSize,
    /// Weather fax: scan rate, index of cooperation, and whether the start and
    /// stop tones drive the receiver on their own.
    pub wefax_lpm: crate::WefaxLpm,
    pub wefax_ioc: crate::WefaxIoc,
    pub wefax_auto_start: bool,
    pub wefax_auto_stop: bool,
    /// Sample-clock trim in parts per million. A sound card a hundred ppm off
    /// walks a quarter-hour chart most of a line sideways, which is the one
    /// setting a fax operator always ends up touching.
    pub wefax_slant_ppm: f32,
    /// Grayscale depth the picture is quantised to before encoding (1/2/4/8).
    /// The bilevel facsimile encodings are always 1 regardless.
    pub rifp_bits_per_pixel: u8,
    /// Payload octets per DATA frame. The CPFSK profile recommends 192: small
    /// chunks cost header overhead, large ones lose more to a single hit.
    pub rifp_chunk_size: u16,
    /// Send every DATA frame this many times. Two is the reference
    /// implementation's default — RIFP has no repair requests, so repetition is
    /// the only recovery there is.
    pub rifp_data_repeats: u8,
    /// Send the MANIFEST this many times before the data starts.
    pub rifp_manifest_repeats: u8,
    /// Repeat the MANIFEST every N data chunks so a receiver that tuned in late
    /// can still reassemble. 0 disables the periodic repeat.
    pub rifp_manifest_every: u16,
    /// Carry the operator's callsign as the Sender ID header TLV. The draft's
    /// privacy considerations note that a stable sender identifier is trackable;
    /// on the amateur bands identifying is the point, so this defaults on.
    pub rifp_send_sender_id: bool,
    /// Short UTF-8 description carried as the Content Hint header TLV, shown by
    /// a receiver before the picture finishes arriving. Empty sends none.
    pub rifp_content_hint: String,
    /// Dither the picture when quantising to fewer than 8 bits per pixel.
    pub rifp_dither: bool,
    /// CW: the tone the decoder listens on and the transmitter keys, in Hz
    /// above the dial. This is where the waterfall cursor sits, so it is both
    /// "which signal am I copying" and "where does mine go out" — in CW those
    /// are the same question, because a station answers on the frequency it
    /// heard the call on.
    #[serde(default = "cw_default_pitch")]
    pub cw_pitch_hz: f32,
    /// CW: transmit speed in words per minute.
    #[serde(default = "cw_default_wpm")]
    pub cw_wpm: f32,
    /// CW: transmit character speed for Farnsworth sending — the elements go
    /// out at `cw_wpm` and only the spacing is stretched to this overall speed.
    /// 0, or anything at or above `cw_wpm`, means ordinary timing.
    #[serde(default)]
    pub cw_farnsworth_wpm: f32,
    /// CW: pin the decoder's speed search to `cw_wpm` instead of reading the
    /// speed off the signal. Worth having for a signal too weak for the search
    /// to settle when you already know how fast the other station sends.
    #[serde(default)]
    pub cw_speed_lock: bool,
    /// Keyboard modes and CW: hold what is typed until Return, then send the
    /// line in one piece, instead of putting each character on the air as it is
    /// typed.
    ///
    /// Off is how these modes are worked — the first letter of a callsign goes
    /// out while the rest is still being typed, and the sent prefix catches up
    /// as it goes. On buys the chance to read a line back before committing it,
    /// and on a rig that keys itself from text it is the difference between one
    /// transmit-receive cycle for the line and one per word.
    #[serde(default)]
    pub send_on_enter: bool,
    /// Give up on an incomplete incoming session after this many seconds.
    pub rifp_session_timeout_s: u32,

    // ── AX.25 packet ──
    /// Which speed the packet modem runs at.
    #[serde(default)]
    pub packet_baud: PacketBaud,
    /// The callsign this station answers to on the air, with an optional SSID
    /// (`OE3JJS-10`). Separate from the logbook callsign: a packet station
    /// conventionally uses an SSID to distinguish the mailbox from the operator.
    #[serde(default)]
    pub packet_mycall: String,
    /// Longest information field sent in one frame. 128 is the AX.25 default
    /// and what a gateway will assume; 256 is faster on a clean VHF channel and
    /// worse on a marginal one, because a single bit error costs the whole
    /// frame.
    #[serde(default = "default_packet_paclen")]
    pub packet_paclen: u16,
    /// Frames that may be outstanding before an acknowledgement is required.
    #[serde(default = "default_packet_maxframe")]
    pub packet_maxframe: u8,
    /// Flags sent ahead of a frame, in milliseconds, to give the far end's
    /// receiver time to hear us and lock its clock.
    ///
    /// Generous by default. The engine alone spends 165–240 ms getting from
    /// "transmit" to the first sample on a CAT rig — measured, see
    /// `crates/sdroxide-radio/tests/tx_turnaround.rs` — and the rig's own
    /// transmit-ready time is on top of that. On an IQ SDR the engine costs
    /// 7 ms and this is purely the far end's business.
    #[serde(default = "default_packet_txdelay_ms")]
    pub packet_txdelay_ms: u16,
    /// Flags sent after a frame before dropping the transmitter.
    #[serde(default = "default_packet_txtail_ms")]
    pub packet_txtail_ms: u16,
    /// CSMA persistence, 0–255: the chance of transmitting in any one slot once
    /// the channel is clear. The classic value is 63.
    #[serde(default = "default_packet_persist")]
    pub packet_persist: u8,
    /// CSMA slot time in milliseconds.
    ///
    /// Ten is the classic figure and is not achievable here: on a sound-card
    /// rig the receive loop only comes round every 341 ms, so slots are counted
    /// on the audio sample clock instead, and this is what that clock counts.
    #[serde(default = "default_packet_slottime_ms")]
    pub packet_slottime_ms: u16,
    /// Offer the modem as a KISS TNC on a TCP port, so Pat, an APRS client or
    /// the Linux AX.25 stack can use the radio.
    #[serde(default)]
    pub packet_kiss_server: bool,
    /// Port for the KISS server. 8001 is what most software expects.
    #[serde(default = "default_packet_kiss_port")]
    pub packet_kiss_port: u16,
    /// Answer incoming connection requests instead of refusing them.
    ///
    /// Off by default, which is the right posture for a Winlink client: it
    /// dials out to a gateway and has no reason to accept calls. Turning it on
    /// makes the station reachable — a mailbox, or a peer for another station
    /// to connect to.
    #[serde(default)]
    pub packet_accept_incoming: bool,
    /// Text sent as a periodic UNPROTO beacon. Empty disables it.
    #[serde(default)]
    pub packet_beacon_text: String,
    /// Sent to a station that connects to us, once the link is up.
    ///
    /// The TNC world calls this CTEXT. Empty sends nothing, which is right for
    /// a station that only ever dials out — and a station that answers calls
    /// but says nothing looks broken to whoever called it.
    #[serde(default = "default_packet_connect_text")]
    pub packet_connect_text: String,
    /// The digipeater path a terminal connect uses when the operator leaves the
    /// via box empty — `OE3XLR-1,OE3XMS-1`, the way `c CALL v A,B` writes it.
    ///
    /// Worth having as a setting because the path to a local node is the same
    /// every time, and retyping it is how a hop gets left off.
    #[serde(default)]
    pub packet_connect_via: String,
    /// Ask for extended (mod-128) sequence numbers when dialling out.
    ///
    /// Off. A window of 128 frames is only useful on a fast, clean path, and
    /// many nodes answer a SABME with a DM — which an operator reads as "that
    /// station refused me" with no way to tell it was the frame they sent.
    #[serde(default)]
    pub packet_ext_seq: bool,
    /// Minutes between beacons; zero disables.
    #[serde(default)]
    pub packet_beacon_minutes: u32,

    // ── APRS ──
    //
    // Separate from the packet settings above rather than shared with them.
    // The two modes run the same modem, but an operator's packet station and
    // their APRS station are conventionally different SSIDs of the same call,
    // beacon different things at different intervals, and are configured on
    // different days. One set of fields would mean changing the mailbox SSID
    // every time the tracker's did.
    /// The callsign this station beacons under, with its SSID.
    ///
    /// Empty falls back to the station callsign ([`DigiConfig::my_call`]) —
    /// see [`DigiConfig::aprs_call`]. It is a field of its own because an
    /// APRS station conventionally carries an SSID that distinguishes it from
    /// the operator (`-9` for a car, `-10` for an I-gate), not because an
    /// operator should have to type their callsign twice.
    #[serde(default)]
    pub aprs_mycall: String,
    /// The digipeater path, as an operator writes it — `WIDE1-1,WIDE2-1`.
    ///
    /// The single most consequential setting on the whole channel: it decides
    /// how many times the network as a whole repeats each of your frames. See
    /// `sdroxide_aprs::path_advice`, which is what the setup dialog shows.
    #[serde(default = "default_aprs_path")]
    pub aprs_path: String,
    /// What this station is: the two characters that pick its map icon.
    #[serde(default)]
    pub aprs_symbol: crate::AprsSymbol,
    /// The comment sent with each beacon — free text, and the only place a
    /// position report says anything an operator wrote.
    #[serde(default)]
    pub aprs_comment: String,
    /// Minutes between beacons; zero — the default — disables them.
    ///
    /// Off rather than on, for the reason every unattended transmitter here is
    /// off by default: selecting a mode must not put a station on the air.
    /// Thirty minutes is the convention for a fixed station once it is on.
    #[serde(default)]
    pub aprs_beacon_minutes: u32,
    /// Take the beacon's position from the station locator
    /// ([`DigiConfig::my_grid`]) rather than from the coordinates below.
    ///
    /// On by default because the locator is already filled in, and a six
    /// character one is good to a couple of kilometres — honest for a fixed
    /// station, and reported with the ambiguity that says so.
    #[serde(default = "yes")]
    pub aprs_use_grid: bool,
    /// Latitude to beacon, degrees north, when not using the locator.
    #[serde(default)]
    pub aprs_lat: f64,
    /// Longitude to beacon, degrees east, when not using the locator.
    #[serde(default)]
    pub aprs_lon: f64,
    /// Send the compressed position format: a third the air time of the
    /// uncompressed one and more precise, which is why it is the default.
    #[serde(default = "yes")]
    pub aprs_compressed: bool,
    /// Acknowledge messages addressed to us.
    ///
    /// On by default and worth its own switch: an acknowledgement is a
    /// transmission this station makes without the operator asking, and a
    /// receive-only setup — a screen with no antenna on transmit — must be
    /// able to turn it off.
    #[serde(default = "yes")]
    pub aprs_ack_messages: bool,
    /// Drop a station from the map this many minutes after it was last heard.
    #[serde(default = "default_aprs_ttl")]
    pub aprs_station_ttl_min: u32,

    /// How loud a digital mode's transmit audio is handed to a radio that
    /// modulates it itself — a CAT rig on a sound card, a FLEX, an Icom on its
    /// network port — for a mode with no entry of its own in
    /// [`tx_audio_levels`](Self::tx_audio_levels). [`TX_AUDIO_LEVEL_MIN`] to
    /// 1.0, and 1.0 is what a radio we modulate ourselves always gets, where
    /// the modulator and Drive own the level instead.
    ///
    /// **Two of them, because the number does two unrelated jobs.** They were
    /// one until an operator set 40 % for FM packet and quietly took 8 dB off
    /// their FT8 as well — the same complaint as issue #131, arriving by
    /// another road.
    ///
    /// `_fm` **is the deviation**: an FM transmitter turns audio level into
    /// frequency swing and has no ALC to catch it, and 1200 baud packet wants
    /// about 3 kHz where voice wants 5. A full-scale burst into a data input
    /// set for voice over-deviates — which sounds completely normal to a
    /// listener and decodes for nobody.
    ///
    /// `_ssb` is **drive into the modulator**, and what keeps a data signal
    /// clean: the usual adjustment is to bring it down until the rig's ALC is
    /// barely moving, then set the power at the radio. On these backends it is
    /// the only level sdroxide has — `TxState::drive` reaches the rig's *power*
    /// register there, not its audio — so this is the knob that stands between
    /// a constant-envelope mode and a splattery signal.
    ///
    /// Which one applies follows the carrier the mode goes out on, not which
    /// panel set it: FM for VHF packet, APRS and RIFP, sideband for everything
    /// else, HF packet included.
    ///
    /// Both full scale by default, which is what every digital mode did before
    /// either existed. The radio's own input level is the other half of it and
    /// only the operator can set that.
    #[serde(default = "one")]
    pub tx_audio_level_fm: f32,
    /// Drive into a sideband rig's modulator; see [`DigiConfig::tx_audio_level_fm`],
    /// which this is the other half of.
    #[serde(default = "one")]
    pub tx_audio_level_ssb: f32,
    /// The transmit-audio level the operator set for a particular mode, which
    /// overrides the carrier default above (issue #186).
    ///
    /// Per mode because the carrier pair is still one number for every mode
    /// riding that carrier, and an operator reported the obvious consequence:
    /// FT8, RTTY, MCW and PSK each want a different figure into the same rig.
    /// What is being set is where the waveform sits against the radio's ALC,
    /// and that is a property of the waveform — a constant-envelope FT8 tone
    /// and RTTY's two-tone shift do not load a modulator the same way.
    ///
    /// **An override, not a replacement.** A mode with no entry takes its
    /// carrier's figure, which is what keeps this honest across an upgrade in
    /// both directions: nobody's signal changes level when the map arrives,
    /// because an empty map is exactly the old behaviour, and a mode appended
    /// in a later release (as most of [`Mode`](crate::Mode)'s have been)
    /// inherits the level the operator actually runs instead of springing back
    /// to full scale the first time they select it. Materialising an entry per
    /// mode would have got that second one wrong, which is issue #131's
    /// symptom by a third road.
    ///
    /// The same override-over-default shape as
    /// [`tx_audio_hz`](Self::tx_audio_hz), which falls back to the mode's usual
    /// 1500 Hz, and [`js8_call`](Self::js8_call), which falls back to
    /// [`my_call`](Self::my_call).
    ///
    /// `HashMap` rather than `BTreeMap` for [`tx_audio_hz`]'s reason, which
    /// applies to [`Mode`](crate::Mode) exactly as it does to
    /// [`Band`](crate::Band): declaration order is a postcard wire index with
    /// later variants appended out of place, so a derived ordering would read
    /// as something meaningful without being it.
    ///
    /// [`tx_audio_hz`]: Self::tx_audio_hz
    #[serde(default)]
    pub tx_audio_levels: std::collections::HashMap<crate::Mode, f32>,

    // ── WSPR ──
    /// WSPR: percentage of two-minute slots to transmit in, 0–100.
    ///
    /// Zero — receive only — is the default, for the reason `js8_heartbeat_min`
    /// gives: a beacon that starts transmitting because the operator selected a
    /// mode is an on-air behaviour nobody consented to. Twenty is the
    /// convention once it *is* switched on: enough to be heard, sparse enough
    /// that a hundred beacons share the window.
    #[serde(default)]
    pub wspr_tx_percent: u8,
    /// WSPR: the power actually radiated, in dBm, as the beacon will announce
    /// it. Rounded to something the message can express by
    /// [`crate::round_power_dbm`] before it goes out — the announcement is part
    /// of everybody else's measurement, so it has to be true.
    #[serde(default = "wspr_default_power")]
    pub wspr_power_dbm: i16,
    /// WSPR: move the dial from band to band between slots, so one receiver
    /// samples the whole spectrum instead of one slice of it.
    #[serde(default)]
    pub wspr_hop: bool,
    /// WSPR: which bands the hop cycle visits, one bit per index into
    /// [`crate::Band::ALL`]. Ignored unless `wspr_hop`.
    #[serde(default = "wspr_default_hop_bands")]
    pub wspr_hop_bands: u16,
    /// WSPR: upload what we decode to wsprnet.org.
    ///
    /// On by default, unlike transmitting: reporting what you hear is the
    /// entire point of the network, it puts nothing on the air, and a receiver
    /// that quietly kept its spots to itself would be missing the mode.
    #[serde(default = "yes")]
    pub wspr_upload: bool,
}

fn wspr_default_power() -> i16 {
    // 5 W — the level the message can express exactly, and what a barefoot
    // beacon on a shared transceiver usually ends up running.
    37
}

fn wspr_default_hop_bands() -> u16 {
    // 80/40/30/20/17/15/12/10 — the bands with a WSPR dial and enough traffic
    // to be worth a slot. 160 m is left out of the default cycle because it is
    // dead by day, and adding it costs a whole slot every time round.
    use crate::Band;
    [Band::M80, Band::M40, Band::M30, Band::M20, Band::M17, Band::M15, Band::M12, Band::M10]
        .iter()
        .filter_map(|b| Band::ALL.iter().position(|x| x == b))
        .fold(0u16, |m, i| m | (1 << i))
}

/// Default for [`DigiConfig::sstv_banner_left`] — the operator's own callsign,
/// which is what the banner was hard-wired to print before it could be edited.
fn sstv_default_banner_left() -> String {
    "{call}".into()
}

/// Default for [`DigiConfig::sstv_banner_right`] — the program and its version,
/// as the banner has always printed them.
fn sstv_default_banner_right() -> String {
    "SDRoxide v{version}".into()
}

/// Default for [`DigiConfig::sstv_banner_fill`] — the historic red.
fn sstv_default_banner_fill() -> [u8; 3] {
    [170, 0, 0]
}

/// Default for [`DigiConfig::sstv_banner_ink`] — white.
fn sstv_default_banner_ink() -> [u8; 3] {
    [255, 255, 255]
}

/// Default for [`DigiConfig::sstv_banner_height`], in picture pixels.
fn sstv_default_banner_height() -> u16 {
    16
}

fn cw_default_pitch() -> f32 {
    700.0
}

fn cw_default_wpm() -> f32 {
    20.0
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
            rtty_reverse: false,
            navtex_reverse: false,
            rtty_afc: true,
            olivia_tones: 32,
            olivia_bw_hz: 1000.0,
            thor_mode: ThorMode::Thor16,
            fsq_baud: 4.5,
            fsq_call: String::new(),
            digi_squelch: 0.35,
            cw_pitch_hz: cw_default_pitch(),
            cw_wpm: cw_default_wpm(),
            cw_farnsworth_wpm: 0.0,
            cw_speed_lock: false,
            send_on_enter: false,
            tx_watchdog_min: 6,
            max_tx_repeats: 10,
            sstv_tx_ppm: 0.0,
            sstv_fsk_id: true,
            sstv_banner: true,
            sstv_banner_left: sstv_default_banner_left(),
            sstv_banner_right: sstv_default_banner_right(),
            sstv_banner_fill: sstv_default_banner_fill(),
            sstv_banner_ink: sstv_default_banner_ink(),
            sstv_banner_height: sstv_default_banner_height(),
            rf_paint_speed: 0.25,
            auto_tx_freq: true,
            hold_tx_freq: false,
            tx_audio_hz: std::collections::HashMap::new(),
            dxped_mode: DxpedMode::Normal,
            contest: ContestMode::None,
            contest_serial: 1,
            fox_slots: 3,
            rade_mute_analog: false,
            js8_speed: crate::Js8Speed::Normal,
            js8_auto_reply: true,
            js8_heartbeat_min: 0,
            js8_hb_ack: false,
            js8_hb_anywhere: false,
            js8_assembly_timeout_s: 300,
            js8_call: String::new(),
            js8_status: String::new(),
            js8_groups: Vec::new(),
            hell_variant: HellVariant::Feld,
            hell_rx_agc: 0.35,
            rifp_profile: RifpProfile::default(),
            rifp_encoding: RifpEncoding::default(),
            rifp_size: RifpSize::default(),
            wefax_lpm: crate::WefaxLpm::default(),
            wefax_ioc: crate::WefaxIoc::default(),
            wefax_auto_start: true,
            wefax_auto_stop: true,
            wefax_slant_ppm: 0.0,
            rifp_bits_per_pixel: 4,
            rifp_chunk_size: 192,
            rifp_data_repeats: 2,
            rifp_manifest_repeats: 3,
            rifp_manifest_every: 8,
            rifp_send_sender_id: true,
            rifp_content_hint: String::new(),
            rifp_dither: true,
            packet_baud: PacketBaud::default(),
            packet_mycall: String::new(),
            packet_paclen: default_packet_paclen(),
            packet_maxframe: default_packet_maxframe(),
            packet_txdelay_ms: default_packet_txdelay_ms(),
            packet_txtail_ms: default_packet_txtail_ms(),
            packet_persist: default_packet_persist(),
            packet_slottime_ms: default_packet_slottime_ms(),
            packet_kiss_server: false,
            packet_kiss_port: default_packet_kiss_port(),
            packet_accept_incoming: false,
            packet_beacon_text: String::new(),
            packet_connect_text: default_packet_connect_text(),
            packet_connect_via: String::new(),
            packet_ext_seq: false,
            packet_beacon_minutes: 0,
            aprs_mycall: String::new(),
            aprs_path: default_aprs_path(),
            aprs_symbol: crate::AprsSymbol::default(),
            aprs_comment: String::new(),
            aprs_beacon_minutes: 0,
            aprs_use_grid: true,
            aprs_lat: 0.0,
            aprs_lon: 0.0,
            aprs_compressed: true,
            aprs_ack_messages: true,
            aprs_station_ttl_min: default_aprs_ttl(),
            tx_audio_level_fm: 1.0,
            tx_audio_level_ssb: 1.0,
            tx_audio_levels: std::collections::HashMap::new(),
            rifp_session_timeout_s: 300,
            wspr_tx_percent: 0,
            wspr_power_dbm: wspr_default_power(),
            wspr_hop: false,
            wspr_hop_bands: wspr_default_hop_bands(),
            wspr_upload: true,
        }
    }
}

/// The quietest a digital mode's transmit audio may be handed to a radio that
/// modulates it itself: −40 dB, the bottom of
/// [`DigiConfig::tx_audio_levels`]'s range.
///
/// A floor rather than zero because the control is the only level there is on
/// those radios, and an operator who dragged it to the bottom would key a
/// transmitter that radiates nothing — which looks exactly like a broken rig
/// and is diagnosed by everything except the slider that caused it.
///
/// Forty decibels because that is what the adjustment actually needs. The
/// figure it replaced was 0.05 (−26 dB), a percent-scaled floor whose two
/// lowest steps were 1.6 dB apart; nothing could hold a value below it, so
/// widening the range changes no station's level and only gives the ones that
/// were already at the bottom somewhere further to go.
pub const TX_AUDIO_LEVEL_MIN: f32 = 0.01;

/// [`TX_AUDIO_LEVEL_MIN`] as the control shows it: −40 dB.
pub const TX_AUDIO_LEVEL_MIN_DB: f32 = -40.0;

/// A stored transmit-audio level as the control shows it, in dB below full
/// scale.
///
/// The level is stored linear because that is what multiplies the samples, and
/// shown in dB because that is what the adjustment is. A percent scale spends
/// most of a rail's travel in the top of a range whose useful part is the
/// bottom: 5 % and 6 % — two adjacent steps on the control this replaced — are
/// 1.6 dB apart, while 50 % and 51 % are a sixth of that. An operator setting a
/// data input against the rig's ALC is working in decibels, and so is the
/// program they are comparing it against.
#[must_use]
pub fn tx_level_db(level: f32) -> f32 {
    if level <= TX_AUDIO_LEVEL_MIN { TX_AUDIO_LEVEL_MIN_DB } else { 20.0 * level.min(1.0).log10() }
}

/// The inverse of [`tx_level_db`], clamped to the range the control offers.
#[must_use]
pub fn tx_level_from_db(db: f32) -> f32 {
    if db <= TX_AUDIO_LEVEL_MIN_DB {
        TX_AUDIO_LEVEL_MIN
    } else {
        10f32.powf(db.min(0.0) / 20.0).clamp(TX_AUDIO_LEVEL_MIN, 1.0)
    }
}

impl DigiConfig {
    /// The transmit-audio level for `mode` on a radio that modulates what we
    /// send it: the operator's entry for that mode, else the level for the
    /// carrier it goes out on.
    ///
    /// The fallback is the whole design. See
    /// [`tx_audio_levels`](Self::tx_audio_levels) for why an absent entry
    /// inherits rather than resetting, and
    /// [`tx_audio_level_fm`](Self::tx_audio_level_fm) for why the carrier is
    /// what picks between the two defaults.
    #[must_use]
    pub fn tx_level_for(&self, mode: crate::Mode) -> f32 {
        self.tx_audio_levels
            .get(&mode)
            .copied()
            .unwrap_or(if mode.is_fm_carrier() {
                self.tx_audio_level_fm
            } else {
                self.tx_audio_level_ssb
            })
            .clamp(TX_AUDIO_LEVEL_MIN, 1.0)
    }

    /// Record the operator's transmit-audio level for one mode.
    ///
    /// Always an entry, even where the figure equals the carrier default: the
    /// operator has said what this mode should run at, and a value that
    /// silently went on tracking the default would move the next time they set
    /// the default for something else.
    pub fn set_tx_level(&mut self, mode: crate::Mode, level: f32) {
        self.tx_audio_levels.insert(mode, level.clamp(TX_AUDIO_LEVEL_MIN, 1.0));
    }

    /// The callsign APRS transmits under: the APRS-specific one if the
    /// operator set one, else the station callsign.
    ///
    /// One function rather than the same two-line fallback in the controller
    /// and in the panel: they have to agree about what is on the air, and
    /// about whether transmit is possible at all. Empty means it is not —
    /// an APRS frame with no callsign in it is an unidentified transmission,
    /// which is illegal everywhere.
    #[must_use]
    pub fn aprs_call(&self) -> String {
        let own = self.aprs_mycall.trim();
        if own.is_empty() { self.my_call.trim().to_uppercase() } else { own.to_uppercase() }
    }

    /// Fill a template's placeholders. `report` is a signed dB value.
    pub fn fill(
        template: &str,
        my_call: &str,
        my_grid: &str,
        dx: &str,
        report: Option<i16>,
    ) -> String {
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
    if db < 0 { format!("-{:02}", -db) } else { format!("+{:02}", db) }
}

/// Which band a frequency falls in, as an ADIF band string (e.g. "20m").
///
/// ADIF's own names, which are not always the ones the operator reads on screen:
/// the 5650 MHz band is `6cm` in the ADIF enumeration and nothing is called
/// `5cm`, so this says `6cm` wherever the station is while
/// [`crate::Band::label_in`] says what that region's plan calls it.
///
/// Deliberately frequency-only, and deliberately not the band plan: a log entry
/// records where the contact was, and it must come out the same whatever region
/// the station is set to or whichever edges the operator has narrowed their
/// `bandplan.json` to. The thresholds sit above each band's top edge in the
/// widest region that has it, so a frequency inside any region's allocation
/// lands on the right name.
///
/// Above 3 cm the answer is the empty string: sdroxide has no 1.2 cm band or
/// anything beyond, and naming a 24 GHz contact `3cm` would put a false
/// statement in a log file. An empty band field is one the importer derives from
/// the frequency, which is the truthful outcome.
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
        m if m < 54.1 => "6m",
        m if m < 70.6 => "4m",
        m if m < 148.1 => "2m",
        m if m < 225.1 => "1.25m",
        m if m < 450.1 => "70cm",
        m if m < 928.1 => "33cm",
        m if m < 1300.1 => "23cm",
        m if m < 2450.1 => "13cm",
        m if m < 3500.1 => "9cm",
        m if m < 5925.1 => "6cm",
        m if m < 10500.1 => "3cm",
        _ => "",
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
///
/// Lines end in CRLF: ADIF readers are tag-delimited and indifferent, but
/// Windows logging software can refuse a file with bare LF endings.
pub fn qso_log_to_adif(records: &[QsoRecord]) -> String {
    let mut out = String::from(
        "ADIF export from sdroxide\r\n<ADIF_VER:5>3.1.4\r\n<PROGRAMID:8>sdroxide\r\n<EOH>\r\n",
    );
    for r in records {
        out.push_str(&qso_to_adif_record(r));
        out.push_str("\r\n");
    }
    out
}

/// One contact as a bare ADIF record, ending in `<EOR>` and nothing after it.
///
/// The record on its own, with no `<EOH>` header and no free text in front of
/// it, because that is what a *record* is — and what the WSJT-X UDP protocol's
/// ADIF message carries. A whole file export sent down that socket begins with
/// a line of prose before the first tag, and a logger reading the datagram as
/// the single record its protocol promises can make nothing of it (issue #341).
///
/// [`qso_log_to_adif`] is this with a file header in front and one record per
/// contact, which is what a file wants and a datagram does not.
pub fn qso_to_adif_record(r: &QsoRecord) -> String {
    let mut out = String::new();
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
    // Extended fields — written only when populated.
    let opt_str = |out: &mut String, name: &str, v: &str| {
        if !v.trim().is_empty() {
            out.push_str(&adif_field(name, v.trim()));
        }
    };
    opt_str(&mut out, "NAME", &r.name);
    opt_str(&mut out, "QTH", &r.qth);
    opt_str(&mut out, "STATE", &r.state);
    opt_str(&mut out, "CNTY", &r.county);
    opt_str(&mut out, "COUNTRY", &r.country);
    if let Some(v) = r.dxcc {
        out.push_str(&adif_field("DXCC", &v.to_string()));
    }
    if let Some(v) = r.cq_zone {
        out.push_str(&adif_field("CQZ", &v.to_string()));
    }
    if let Some(v) = r.itu_zone {
        out.push_str(&adif_field("ITUZ", &v.to_string()));
    }
    opt_str(&mut out, "CONT", &r.continent);
    opt_str(&mut out, "IOTA", &r.iota);
    opt_str(&mut out, "SIG", &r.sig);
    opt_str(&mut out, "SIG_INFO", &r.sig_info);
    if let Some(v) = r.tx_pwr {
        out.push_str(&adif_field("TX_PWR", &format!("{v}")));
    }
    opt_str(&mut out, "OPERATOR", &r.operator);
    opt_str(&mut out, "CONTEST_ID", &r.contest_id);
    if let Some(v) = r.srx {
        out.push_str(&adif_field("SRX", &v.to_string()));
    }
    if let Some(v) = r.stx {
        out.push_str(&adif_field("STX", &v.to_string()));
    }
    opt_str(&mut out, "SRX_STRING", &r.srx_string);
    opt_str(&mut out, "STX_STRING", &r.stx_string);
    opt_str(&mut out, "MY_STATE", &r.my_state);
    opt_str(&mut out, "MY_COUNTRY", &r.my_country);
    if let Some(v) = r.my_dxcc {
        out.push_str(&adif_field("MY_DXCC", &v.to_string()));
    }
    if let Some(v) = r.my_cq_zone {
        out.push_str(&adif_field("MY_CQ_ZONE", &v.to_string()));
    }
    if let Some(v) = r.my_itu_zone {
        out.push_str(&adif_field("MY_ITU_ZONE", &v.to_string()));
    }
    opt_str(&mut out, "QSL_VIA", &r.qsl_via);
    let yn = |out: &mut String, name: &str, v: bool| {
        if v {
            out.push_str(&adif_field(name, "Y"));
        }
    };
    yn(&mut out, "LOTW_QSL_SENT", r.lotw_sent);
    yn(&mut out, "LOTW_QSL_RCVD", r.lotw_rcvd);
    yn(&mut out, "EQSL_QSL_SENT", r.eqsl_sent);
    yn(&mut out, "EQSL_QSL_RCVD", r.eqsl_rcvd);
    yn(&mut out, "QSL_SENT", r.qsl_sent);
    yn(&mut out, "QSL_RCVD", r.qsl_rcvd);
    out.push_str(&adif_field("STATION_CALLSIGN", &r.my_call));
    out.push_str(&adif_field("MY_GRIDSQUARE", &r.my_grid));
    out.push_str("<EOR>");
    out
}

/// Byte offset just past the `len`th character of `s`, or `None` when it holds
/// fewer characters than that.
fn char_end(s: &str, len: usize) -> Option<usize> {
    let mut chars = s.char_indices();
    for _ in 0..len {
        chars.next()?;
    }
    Some(chars.next().map_or(s.len(), |(off, _)| off))
}

/// Read one field value out of `adif` at `start`, given the length its tag
/// declared, and report where parsing resumes.
///
/// ADIF counts that length in bytes, which is what [`qso_log_to_adif`] emits,
/// but some exporters (QRZ's logbook among them) count characters instead. The
/// two readings agree on plain ASCII and part company on the first accented
/// value, so the declared length is a hint to be checked rather than an offset
/// to slice at — slicing blind lands inside a multi-byte character and panics,
/// or, where it happens to land on a boundary, truncates the value in silence.
///
/// Where the readings agree there is nothing to disambiguate and the spec's
/// reading stands. Where they differ, a candidate end is accepted only if it
/// opens the next tag: values run straight up against the following `<`, which
/// is what makes a miscount detectable at all. Bytes are tried first, then
/// characters; if neither fits, re-sync on the next `<` so a bad count costs
/// its own field instead of every field after it in the record.
fn adif_value(adif: &str, start: usize, len: usize) -> (String, usize) {
    let rest = &adif[start..];
    let as_bytes = rest.is_char_boundary(len).then_some(len);
    let as_chars = char_end(rest, len);
    if as_bytes.is_some() && as_bytes == as_chars {
        return (rest[..len].to_string(), start + len);
    }
    // Exporters break lines between fields, so allow whitespace before the '<'.
    let opens_a_tag = |end: &usize| {
        let after = rest[*end..].trim_start();
        after.is_empty() || after.starts_with('<')
    };
    if let Some(end) = as_bytes.filter(opens_a_tag).or_else(|| as_chars.filter(opens_a_tag)) {
        return (rest[..end].to_string(), start + end);
    }
    // Neither count lands anywhere a value can end. A value may legitimately
    // contain '<' — carrying a length is the whole reason ADIF can afford that
    // — so scanning for one is the last resort and never the first reading.
    let end = rest.find('<').unwrap_or(rest.len());
    (rest[..end].trim_end().to_string(), start + end)
}

/// Split an ADIF (.adi) document into records, each a list of
/// `(UPPERCASE_FIELD, value)` pairs in the order they appear. Header fields
/// (everything before `<EOH>`) are dropped; a trailing record with no `<EOR>`
/// is still returned, because some exporters omit it.
///
/// Public because ADIF is not only something sdroxide reads into its own log:
/// HamQTH's real-time logbook wants a *subset* of the fields, under two names
/// of its own, so `sdroxide-net` re-emits a record rather than parsing one. It
/// goes through [`adif_visit`] so there is one tokenizer — the byte-vs-character
/// length disambiguation in [`adif_value`] is exactly the kind of thing a second
/// copy would get subtly wrong.
///
/// This holds every record's fields at once, which is the wrong shape for a
/// whole-log import — [`adif_to_qso_log`] streams instead. It is the right shape
/// for the handful of records an upload sends.
pub fn adif_records(adif: &str) -> Vec<Vec<(String, String)>> {
    let mut records = Vec::new();
    adif_visit(adif, |fields| records.push(fields.to_vec()));
    records
}

/// Tokenize an ADIF (.adi) document, calling `visit` once per record with its
/// `(UPPERCASE_FIELD, value)` pairs. The one place that reads ADIF's tag syntax.
fn adif_visit(adif: &str, mut visit: impl FnMut(&[(String, String)])) {
    let mut fields: Vec<(String, String)> = Vec::new();
    let bytes = adif.as_bytes();
    let mut i = 0usize;
    let mut in_header = adif.to_ascii_uppercase().contains("<EOH>");
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        let Some(close) = adif[i..].find('>') else { break };
        let tag = &adif[i + 1..i + close];
        i += close + 1;
        let mut parts = tag.splitn(3, ':');
        let name = parts.next().unwrap_or("").trim().to_ascii_uppercase();
        if name == "EOH" {
            in_header = false;
            fields.clear();
            continue;
        }
        if name == "EOR" {
            if !fields.is_empty() {
                visit(&fields);
            }
            fields.clear();
            continue;
        }
        let len: usize = parts.next().and_then(|l| l.trim().parse().ok()).unwrap_or(0);
        // A field with no length (or a header tag) has no value payload.
        let value = if len > 0 {
            let (v, next) = adif_value(adif, i, len);
            i = next;
            v
        } else {
            String::new()
        };
        if !in_header {
            fields.push((name, value));
        }
    }
    // A final record without a trailing <EOR> (some exporters omit it).
    if !fields.is_empty() {
        visit(&fields);
    }
}

/// Parse an ADIF (.adi) document into QSO records. Tolerant of unknown fields
/// (ignored) and of a missing/short header. Used both for importing external
/// logs and for ingesting downloaded QSL confirmations. The inverse of
/// [`qso_log_to_adif`] for the fields sdroxide round-trips.
pub fn adif_to_qso_log(adif: &str) -> Vec<QsoRecord> {
    let mut records = Vec::new();
    // Each record is converted as it is tokenized: importing somebody's
    // fifty-thousand-QSO log should cost one `QsoRecord` per contact, not that
    // plus every field of every contact still held as a pair of `String`s.
    adif_visit(adif, |fields| records.push(record_from_fields(fields)));
    records
}

fn record_from_fields(fields: &[(String, String)]) -> QsoRecord {
    let get = |key: &str| fields.iter().find(|(k, _)| k == key).map(|(_, v)| v.trim().to_string());
    let mut r = QsoRecord::default();
    r.call = get("CALL").unwrap_or_default().to_ascii_uppercase();
    r.grid = get("GRIDSQUARE").filter(|s| !s.is_empty());
    r.rst_sent = get("RST_SENT").and_then(|s| s.parse().ok());
    r.rst_rcvd = get("RST_RCVD").and_then(|s| s.parse().ok());
    if let Some(f) = get("FREQ").and_then(|s| s.parse::<f64>().ok()) {
        r.freq_hz = f * 1e6;
    }
    r.mode = get("MODE").unwrap_or_default().to_ascii_uppercase();
    r.band = get("BAND").map(|b| b.to_ascii_lowercase()).unwrap_or_else(|| {
        if r.freq_hz > 0.0 { adif_band(r.freq_hz).to_string() } else { String::new() }
    });
    let date = get("QSO_DATE").unwrap_or_default();
    let time_on = get("TIME_ON").unwrap_or_default();
    r.start_utc = parse_adif_datetime(&date, &time_on);
    let time_off = get("TIME_OFF").unwrap_or_else(|| time_on.clone());
    r.end_utc =
        if time_off.is_empty() { r.start_utc } else { parse_adif_datetime(&date, &time_off) };
    r.my_call = get("STATION_CALLSIGN")
        .or_else(|| get("OPERATOR"))
        .unwrap_or_default()
        .to_ascii_uppercase();
    r.my_grid = get("MY_GRIDSQUARE").unwrap_or_default();
    // Extended fields.
    r.name = get("NAME").unwrap_or_default();
    r.qth = get("QTH").unwrap_or_default();
    r.state = get("STATE").unwrap_or_default();
    r.county = get("CNTY").unwrap_or_default();
    r.country = get("COUNTRY").unwrap_or_default();
    r.dxcc = get("DXCC").and_then(|s| s.parse().ok());
    r.cq_zone = get("CQZ").and_then(|s| s.parse().ok());
    r.itu_zone = get("ITUZ").and_then(|s| s.parse().ok());
    r.continent = get("CONT").unwrap_or_default();
    r.iota = get("IOTA").unwrap_or_default();
    r.sig = get("SIG").unwrap_or_default();
    r.sig_info = get("SIG_INFO").unwrap_or_default();
    r.tx_pwr = get("TX_PWR").and_then(|s| s.parse().ok());
    r.operator = get("OPERATOR").unwrap_or_default();
    r.contest_id = get("CONTEST_ID").unwrap_or_default();
    r.srx = get("SRX").and_then(|s| s.parse().ok());
    r.stx = get("STX").and_then(|s| s.parse().ok());
    r.srx_string = get("SRX_STRING").unwrap_or_default();
    r.stx_string = get("STX_STRING").unwrap_or_default();
    r.my_state = get("MY_STATE").unwrap_or_default();
    r.my_country = get("MY_COUNTRY").unwrap_or_default();
    r.my_dxcc = get("MY_DXCC").and_then(|s| s.parse().ok());
    r.my_cq_zone = get("MY_CQ_ZONE").and_then(|s| s.parse().ok());
    r.my_itu_zone = get("MY_ITU_ZONE").and_then(|s| s.parse().ok());
    r.qsl_via = get("QSL_VIA").unwrap_or_default();
    let yn = |key: &str| get(key).map(|v| v.eq_ignore_ascii_case("Y")).unwrap_or(false);
    r.lotw_sent = yn("LOTW_QSL_SENT");
    r.lotw_rcvd = yn("LOTW_QSL_RCVD");
    r.eqsl_sent = yn("EQSL_QSL_SENT");
    r.eqsl_rcvd = yn("EQSL_QSL_RCVD");
    r.qsl_sent = yn("QSL_SENT");
    r.qsl_rcvd = yn("QSL_RCVD");
    r
}

/// Parse an ADIF `YYYYMMDD` + `HHMM`/`HHMMSS` pair into a Unix timestamp.
fn parse_adif_datetime(date: &str, time: &str) -> i64 {
    if date.len() < 8 {
        return 0;
    }
    let y = date[0..4].parse().unwrap_or(1970);
    let mo = date[4..6].parse().unwrap_or(1);
    let d = date[6..8].parse().unwrap_or(1);
    let (h, mi, s) = if time.len() >= 6 {
        (
            time[0..2].parse().unwrap_or(0),
            time[2..4].parse().unwrap_or(0),
            time[4..6].parse().unwrap_or(0),
        )
    } else if time.len() >= 4 {
        (time[0..2].parse().unwrap_or(0), time[2..4].parse().unwrap_or(0), 0)
    } else {
        (0, 0, 0)
    };
    ymd_hms_to_unix(y, mo, d, h, mi, s)
}

/// Render a session's QSOs as a human-readable text log. CRLF line endings,
/// like the ADIF export, so the file opens correctly on Windows.
pub fn qso_log_to_text(records: &[QsoRecord]) -> String {
    let mut out = String::from(
        "sdroxide QSO log\r\nUTC date/time        call       grid  freq(MHz)  mode  sent rcvd\r\n",
    );
    for r in records {
        let (date, time) = adif_date_time(r.start_utc);
        let d = format!(
            "{}-{}-{} {}:{}:{}",
            &date[0..4],
            &date[4..6],
            &date[6..8],
            &time[0..2],
            &time[2..4],
            &time[4..6]
        );
        out.push_str(&format!(
            "{:19}  {:10} {:5} {:10.6}  {:4}  {:>4} {:>4}\r\n",
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
    fn a_band_keyed_offset_survives_the_config_file() {
        // The failure this guards compiles perfectly and only shows at runtime:
        // serde_json requires map keys to be strings, so a key type that
        // serialises as anything else would make `save_digi_config` fail at the
        // moment the operator sets an offset, leaving a log line and no saved
        // figure. `Band` is a unit-variant enum and serialises as its name, and
        // this pins that rather than trusting it.
        let mut cfg = DigiConfig { my_call: "G4MQL".into(), ..DigiConfig::default() };
        cfg.tx_audio_hz.insert(crate::Band::M60, 370.0);
        cfg.tx_audio_hz.insert(crate::Band::M20, 1500.0);
        let text = serde_json::to_string(&cfg).expect("a band-keyed map must serialise");
        assert!(text.contains(r#""M60":370.0"#), "60 m's offset is not in the file: {text}");

        let back: DigiConfig = serde_json::from_str(&text).expect("and must load again");
        assert_eq!(back.tx_audio_hz.get(&crate::Band::M60).copied(), Some(370.0));
        assert_eq!(back.tx_audio_hz.get(&crate::Band::M20).copied(), Some(1500.0));

        // And a `digi.json` written before this field existed still loads, which
        // is what `#[serde(default)]` is there for. Anything else would greet an
        // operator with a config reset to defaults on the first run after an
        // update, callsign included.
        let old = r#"{"my_call":"G4MQL","my_grid":"IO81VS"}"#;
        let loaded: DigiConfig = serde_json::from_str(old).expect("an old config must still load");
        assert_eq!(loaded.my_call, "G4MQL");
        assert!(loaded.tx_audio_hz.is_empty(), "a band with no entry must have none");
    }

    #[test]
    fn a_mode_keyed_tx_level_survives_the_config_file() {
        // The same failure `a_band_keyed_offset_survives_the_config_file`
        // guards, one map along: serde_json map keys must be strings, and a key
        // type that serialised as anything else would make `save_digi_config`
        // fail at the moment the operator moved the rail. `Mode` is a
        // unit-variant enum with no rename attributes, so it serialises as its
        // own name — pinned here rather than trusted.
        let mut cfg = DigiConfig { my_call: "G4MQL".into(), ..DigiConfig::default() };
        cfg.set_tx_level(crate::Mode::Ft8, 0.25);
        cfg.set_tx_level(crate::Mode::Rtty, 0.4);
        let text = serde_json::to_string(&cfg).expect("a mode-keyed map must serialise");
        assert!(text.contains(r#""Ft8":0.25"#), "FT8's level is not in the file: {text}");

        let back: DigiConfig = serde_json::from_str(&text).expect("and must load again");
        assert_eq!(back.tx_level_for(crate::Mode::Ft8), 0.25);
        assert_eq!(back.tx_level_for(crate::Mode::Rtty), 0.4);
    }

    #[test]
    fn a_mode_with_no_level_of_its_own_takes_its_carrier_default() {
        // The property the whole shape rests on. A digi.json written before
        // this field existed carries no map at all, so every mode must come out
        // exactly where the carrier pair left it — that is what makes "nobody's
        // signal changes level on an update" true without a migration.
        let old = r#"{"my_call":"G4MQL","tx_audio_level_fm":0.4,"tx_audio_level_ssb":0.9}"#;
        let cfg: DigiConfig = serde_json::from_str(old).expect("an old config must still load");
        assert!(cfg.tx_audio_levels.is_empty(), "a mode with no entry must have none");
        assert_eq!(cfg.tx_level_for(crate::Mode::Ft8), 0.9, "sideband took the FM level");
        assert_eq!(cfg.tx_level_for(crate::Mode::Rtty), 0.9);
        assert_eq!(cfg.tx_level_for(crate::Mode::Cw), 0.9, "MCW is audio on a sideband");
        assert_eq!(cfg.tx_level_for(crate::Mode::Aprs), 0.4, "APRS took the sideband level");
        assert_eq!(cfg.tx_level_for(crate::Mode::Packet), 0.4);
        assert_eq!(cfg.tx_level_for(crate::Mode::PacketHf), 0.9, "HF packet is not FM");

        // And an entry beats the default for that mode alone.
        let mut cfg = cfg;
        cfg.set_tx_level(crate::Mode::Rtty, 0.2);
        assert_eq!(cfg.tx_level_for(crate::Mode::Rtty), 0.2);
        assert_eq!(cfg.tx_level_for(crate::Mode::Ft8), 0.9, "RTTY's level reached FT8");
    }

    #[test]
    fn the_tx_level_floor_leaves_a_transmitter_that_radiates() {
        // Dragging the rail to the bottom must not key a dead transmitter.
        let mut cfg = DigiConfig::default();
        cfg.set_tx_level(crate::Mode::Ft8, 0.0);
        assert_eq!(cfg.tx_level_for(crate::Mode::Ft8), TX_AUDIO_LEVEL_MIN);
        cfg.set_tx_level(crate::Mode::Ft8, 9.0);
        assert_eq!(cfg.tx_level_for(crate::Mode::Ft8), 1.0);
        // A carrier default from a hand-edited file is clamped on the way out
        // too — `tx_level_for` is the only reader, so this is the one place it
        // can be caught.
        let wild = DigiConfig { tx_audio_level_ssb: 0.0, ..DigiConfig::default() };
        assert_eq!(wild.tx_level_for(crate::Mode::Ft8), TX_AUDIO_LEVEL_MIN);
    }

    #[test]
    fn report_formatting() {
        assert_eq!(fmt_report(-13), "-13");
        assert_eq!(fmt_report(2), "+02");
        assert_eq!(fmt_report(0), "+00");
    }

    fn cq(msg: &str, from: &str, grid: Option<&str>) -> Decode {
        Decode {
            slot_utc: 0,
            snr_db: -10,
            dt: 0.1,
            audio_hz: 1500.0,
            message: msg.to_string(),
            to: None,
            from: Some(from.to_string()),
            grid: grid.map(|g| g.to_string()),
            is_cq: true,
            // Whatever sits between "CQ" and the callsign is the modifier.
            cq_to: msg
                .split_whitespace()
                .nth(1)
                .filter(|t| *t != from)
                .map(|t| t.to_ascii_uppercase()),
            free_text: false,
            rr73_to: None,
        }
    }

    #[test]
    fn cq_dx_is_only_for_stations_outside_the_callers_entity() {
        // A plain CQ is for everyone, near or far.
        let plain = cq("CQ W1AW FN31", "W1AW", Some("FN31"));
        assert!(cq_is_for_us(&plain, "K2XYZ", "FN30"));

        // "CQ DX" from a US station: another US station is not DX for them.
        let dx = cq("CQ DX W1AW FN31", "W1AW", Some("FN31"));
        assert!(!cq_is_for_us(&dx, "K2XYZ", "FN30"));
        // A German station is.
        assert!(cq_is_for_us(&dx, "DL1ABC", "JN48"));
        // So is Hawaii — same country, but a separate DXCC entity.
        assert!(cq_is_for_us(&dx, "KH6ABC", "BL11"));

        // Not a CQ at all → never counted as one.
        let mut qso = plain.clone();
        qso.is_cq = false;
        assert!(!cq_is_for_us(&qso, "DL1ABC", "JN48"));
    }

    #[test]
    fn unresolvable_cq_dx_falls_back_to_distance_then_to_showing_it() {
        // "QQ" is no country cty.dat knows; with grids on both sides the
        // great-circle distance decides instead.
        let mut dx = cq("CQ DX QQ1QQ JN48", "QQ1QQ", Some("JN48"));
        assert!(!cq_is_for_us(&dx, "QQ2QQ", "JN47"), "a neighbour is not DX");
        assert!(cq_is_for_us(&dx, "QQ2QQ", "FN31"), "an ocean away is DX");

        // No grid and no resolvable entity → we can't judge, so show it.
        dx.grid = None;
        assert!(cq_is_for_us(&dx, "QQ2QQ", "JN47"));
        // Nor can we judge with no station callsign or grid of our own.
        let dx = cq("CQ DX W1AW FN31", "W1AW", Some("FN31"));
        assert!(cq_is_for_us(&dx, "", ""));
    }

    #[test]
    fn a_continent_cq_is_for_that_continent() {
        let eu = cq("CQ EU W1AW FN31", "W1AW", Some("FN31"));
        assert!(cq_is_for_us(&eu, "DL1ABC", "JN48"), "Germany is in EU");
        assert!(cq_is_for_us(&eu, "G0ABC", "IO91"));
        assert!(!cq_is_for_us(&eu, "K2XYZ", "FN30"), "a US station is not EU");
        assert!(!cq_is_for_us(&eu, "JA1XYZ", "PM95"));

        let na = cq("CQ NA DL1ABC JN48", "DL1ABC", Some("JN48"));
        assert!(cq_is_for_us(&na, "K2XYZ", "FN30"));
        assert!(!cq_is_for_us(&na, "G0ABC", "IO91"));

        // A station whose own entity we can't resolve gets shown everything.
        assert!(cq_is_for_us(&eu, "QQ2QQ", "JN47"));
    }

    #[test]
    fn a_country_cq_is_for_that_country() {
        let ja = cq("CQ JA W1AW FN31", "W1AW", Some("FN31"));
        assert!(cq_is_for_us(&ja, "JA1XYZ", "PM95"));
        assert!(!cq_is_for_us(&ja, "K2XYZ", "FN30"));
        assert!(!cq_is_for_us(&ja, "DL1ABC", "JN48"));
    }

    #[test]
    fn an_activity_cq_is_open_to_everyone() {
        // POTA, contests and the rest invite a kind of contact, not a place —
        // including the ones whose token collides with a country prefix ("FD"
        // starts like France, "WW" like the United States, "RU" like Russia).
        for modifier in ["POTA", "SOTA", "TEST", "QRP", "FD", "WW", "RU"] {
            let d = cq(&format!("CQ {modifier} W1AW FN31"), "W1AW", Some("FN31"));
            for me in ["K2XYZ", "DL1ABC", "JA1XYZ"] {
                assert!(cq_is_for_us(&d, me, "FN30"), "CQ {modifier} hidden from {me}");
            }
        }
        // A modifier nobody recognises is shown rather than hidden.
        let odd = cq("CQ ZZZZ W1AW FN31", "W1AW", Some("FN31"));
        assert!(cq_is_for_us(&odd, "K2XYZ", "FN30"));
    }

    #[test]
    fn clock_health_is_symmetric_about_zero() {
        assert_eq!(clock_health(0.0), ClockHealth::Good);
        assert_eq!(clock_health(0.49), ClockHealth::Good);
        // Early and late are equally unworkable.
        assert_eq!(clock_health(0.9), ClockHealth::Marginal);
        assert_eq!(clock_health(-0.9), ClockHealth::Marginal);
        assert_eq!(clock_health(2.0), ClockHealth::Bad);
        assert_eq!(clock_health(-2.0), ClockHealth::Bad);
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

    /// Both exports land in front of Windows software — the .adi in loggers
    /// that can refuse a bare-LF file — so every line ending is CRLF, with no
    /// stray LF or CR anywhere.
    #[test]
    fn exports_end_lines_with_crlf() {
        let rec = QsoRecord {
            call: "W9XYZ".into(),
            freq_hz: 14_074_000.0,
            mode: "FT8".into(),
            band: "20m".into(),
            start_utc: 1_609_459_200,
            end_utc: 1_609_459_260,
            my_call: "AB1CD".into(),
            my_grid: "FN42".into(),
            ..Default::default()
        };
        let adif = qso_log_to_adif(std::slice::from_ref(&rec));
        let txt = qso_log_to_text(&[rec]);
        for text in [adif, txt] {
            assert!(text.ends_with("\r\n"), "export does not end in CRLF: {text:?}");
            let stripped = text.replace("\r\n", "");
            assert!(
                !stripped.contains('\n') && !stripped.contains('\r'),
                "a bare LF or CR survives in the export: {text:?}"
            );
        }
    }

    /// Every band sdroxide has gets its own ADIF name, and a contact anywhere
    /// inside it lands on that name. 70 cm through 6 cm used to come out as
    /// "2m", which was the catch-all arm rather than an answer.
    #[test]
    fn every_band_logs_under_its_own_adif_name() {
        for (hz, band) in [
            (1_840_000.0, "160m"),
            (28_074_000.0, "10m"),
            (50_313_000.0, "6m"),
            (70_174_000.0, "4m"),
            (144_174_000.0, "2m"),
            (147_500_000.0, "2m"),
            (223_500_000.0, "1.25m"),
            (432_174_000.0, "70cm"),
            (446_000_000.0, "70cm"),
            (902_100_000.0, "33cm"),
            (1_296_174_000.0, "23cm"),
            (2_304_174_000.0, "13cm"),
            (2_320_174_000.0, "13cm"),
            (3_400_100_000.0, "9cm"),
            (3_456_100_000.0, "9cm"),
            (5_760_100_000.0, "6cm"),
            // ADIF has no "5cm": the band the Americas call 5 cm logs as 6 cm,
            // which is the name the enumeration defines.
            (5_900_000_000.0, "6cm"),
            // 3 cm, which an IC-905 reaches with Icom's own transverter inside
            // it (issue #326).
            (10_368_100_000.0, "3cm"),
        ] {
            assert_eq!(adif_band(hz), band, "{hz} Hz");
        }
        // Above every band sdroxide knows, the honest answer is none at all
        // rather than the nearest name — 1.2 cm and up have no band here.
        assert_eq!(adif_band(24_048_000_000.0), "");
        // And a contact anywhere inside a band gets one name for the whole of
        // it, in every region — the widest allocation included, so a US 40 m
        // contact at 7.290 and a Region 2 5 cm one at 5.9 GHz are not filed
        // under the band above. A kilohertz inside each edge rather than on it:
        // the thresholds are boundaries between bands, and which side an exact
        // edge falls on is not something a log has to have an opinion about.
        for region in crate::Region::ALL {
            for b in crate::Band::ALL {
                let Some((lo, hi)) = b.edges_in(region) else { continue };
                let (inside_lo, inside_hi) = (adif_band(lo + 1000.0), adif_band(hi - 1000.0));
                assert_eq!(inside_lo, inside_hi, "{b:?} in {region:?} straddles two ADIF bands");
                assert!(!inside_lo.is_empty(), "{b:?} in {region:?} has no ADIF name");
            }
        }
    }

    /// Issue #341: the WSJT-X ADIF datagram carries a *record*. A file export
    /// down that socket starts with a line of prose and an `<EOH>`, which is a
    /// perfectly good file and not what the message is defined to hold.
    #[test]
    fn one_contact_becomes_a_bare_adif_record() {
        let rec = QsoRecord {
            call: "W9XYZ".into(),
            band: "20m".into(),
            mode: "SSB".into(),
            freq_hz: 14_250_000.0,
            my_call: "W1AW".into(),
            my_grid: "FN31".into(),
            ..QsoRecord::default()
        };
        let one = qso_to_adif_record(&rec);
        assert!(one.starts_with("<CALL:5>W9XYZ"), "{one}");
        assert!(one.ends_with("<EOR>"), "{one}");
        assert!(!one.contains("<EOH>"), "a record carries no file header: {one}");
        assert!(!one.contains("ADIF export"), "{one}");

        // The file form is that record with a header in front of it, so the two
        // cannot drift apart.
        let file = qso_log_to_adif(std::slice::from_ref(&rec));
        assert!(file.starts_with("ADIF export from sdroxide\r\n"), "{file}");
        assert!(file.contains("<EOH>\r\n"), "{file}");
        assert!(file.contains(&one), "the file must contain the record verbatim");
        assert!(file.ends_with("<EOR>\r\n"), "{file}");
    }

    #[test]
    fn adif_import_round_trips() {
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
        let adif = qso_log_to_adif(std::slice::from_ref(&rec));
        let back = adif_to_qso_log(&adif);
        assert_eq!(back.len(), 1);
        let b = &back[0];
        assert_eq!(b.call, "W9XYZ");
        assert_eq!(b.grid.as_deref(), Some("EM48"));
        assert_eq!(b.mode, "FT8");
        assert_eq!(b.band, "20m");
        assert_eq!(b.rst_sent, Some(-9));
        assert_eq!(b.rst_rcvd, Some(-12));
        assert_eq!(b.start_utc, 1_609_459_200);
        assert_eq!(b.my_call, "AB1CD");
        assert_eq!(b.my_grid, "FN42");
    }

    /// A name outside ASCII survives the export and comes back off it. The
    /// length ADIF declares is a byte count, so a Cyrillic or accented name is
    /// longer than it looks — writing the character count instead would put
    /// every later field in the record out of step.
    #[test]
    fn adif_round_trips_a_name_outside_ascii() {
        let rec = QsoRecord {
            call: "UA1ABC".into(),
            name: "Владимир".into(),
            qth: "Москва".into(),
            country: "Ålesund".into(),
            band: "20m".into(),
            mode: "SSB".into(),
            ..Default::default()
        };
        let adif = qso_log_to_adif(std::slice::from_ref(&rec));
        assert!(adif.contains("<NAME:16>Владимир"), "the count is bytes, not characters");
        let back = adif_to_qso_log(&adif);
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].name, "Владимир");
        assert_eq!(back[0].qth, "Москва");
        assert_eq!(back[0].country, "Ålesund");
    }

    #[test]
    fn adif_import_reads_character_counted_lengths() {
        // QRZ's logbook counts the length in characters rather than the bytes
        // the spec asks for: "Amaro José" is ten characters and eleven bytes.
        let recs = adif_to_qso_log("<call:5>EA1AB <name:10>Amaro José <band:3>20m <eor>");
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].call, "EA1AB");
        assert_eq!(recs[0].name, "Amaro José");
        assert_eq!(recs[0].band, "20m");
    }

    #[test]
    fn adif_import_reads_byte_counted_lengths() {
        // The same value written the way the spec — and qso_log_to_adif — has it.
        let recs = adif_to_qso_log("<call:5>EA1AB <name:11>Amaro José <band:3>20m <eor>");
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].name, "Amaro José");
        assert_eq!(recs[0].band, "20m");
    }

    #[test]
    fn adif_import_does_not_truncate_on_a_short_count() {
        // The half of a miscount that never announces itself: ten bytes into
        // "José Amaro" is a character boundary, so there is nothing to panic
        // on and the value is simply clipped to "José Amar".
        let recs = adif_to_qso_log("<call:5>EA1AB <name:10>José Amaro <band:3>20m <eor>");
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].name, "José Amaro");
        assert_eq!(recs[0].band, "20m");
    }

    #[test]
    fn adif_import_keeps_a_value_holding_a_bracket() {
        // Carrying a length is what lets an ADIF value contain '<', so a
        // correct count must never be second-guessed by scanning for one.
        let recs = adif_to_qso_log("<call:5>W1AW <qth:7>a<b>c<d <band:3>20m <eor>");
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].qth, "a<b>c<d");
        assert_eq!(recs[0].band, "20m");
    }

    #[test]
    fn adif_import_resyncs_past_an_impossible_count() {
        // A count that fits no reading costs its own field and nothing after it.
        let recs = adif_to_qso_log("<call:5>W1AW <qth:99>Anywhere <band:3>20m <eor>");
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].call, "W1AW");
        assert_eq!(recs[0].qth, "Anywhere");
        assert_eq!(recs[0].band, "20m");
    }

    #[test]
    fn adif_import_survives_a_truncated_document() {
        // Every prefix of a miscounted record, so a declared length running
        // off the end or into the middle of a character is hit at each offset.
        let doc = "<call:5>EA1AB <name:10>Amaro José <band:3>20m <eor>";
        for end in 0..=doc.len() {
            if doc.is_char_boundary(end) {
                let _ = adif_to_qso_log(&doc[..end]);
            }
        }
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

/// `#[serde(default)]` helper: a bool that defaults to true.
fn yes() -> bool {
    true
}

/// `#[serde(default)]` helper: the first serial number of a contest.
fn one_u32() -> u32 {
    1
}

/// `#[serde(default)]` helper: full scale.
fn one() -> f32 {
    1.0
}

/// `#[serde(default)]` helper for [`DigiConfig::js8_assembly_timeout_s`].
fn js8_default_timeout() -> u32 {
    300
}

fn default_packet_paclen() -> u16 {
    128
}
fn default_packet_maxframe() -> u8 {
    4
}
fn default_packet_txdelay_ms() -> u16 {
    500
}
fn default_packet_txtail_ms() -> u16 {
    50
}
fn default_packet_persist() -> u8 {
    63
}
fn default_packet_slottime_ms() -> u16 {
    100
}
fn default_packet_kiss_port() -> u16 {
    8001
}
/// What a station that answers a call says first.
///
/// Generic on purpose: it goes out under whatever callsign the operator set,
/// and a greeting naming a station that is not theirs is worse than none.
fn default_packet_connect_text() -> String {
    "Connected to sdroxide. No mailbox here — type away.".to_string()
}
/// One local fill-in hop and one wide one — the path that reaches almost
/// anywhere without asking the whole network to repeat you three times.
fn default_aprs_path() -> String {
    "WIDE1-1,WIDE2-1".to_string()
}
fn default_aprs_ttl() -> u32 {
    60
}
