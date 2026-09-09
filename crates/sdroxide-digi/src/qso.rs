//! The FT8/FT4 QSO state machine — pure, deterministic, unit-testable.
//! Given our identity, the operator's message templates, and incoming
//! decodes, it decides the next message to transmit and tracks progress
//! through the standard exchange.

use sdroxide_types::{
    ContestMode, Decode, DigiConfig, DigiStatus, DxpedMode, Mode, QsoRecord, QsoStep, QueuedCall,
    TranscriptLine,
};

use crate::fox::{Fox, slot_spacing_hz};

/// Give up waiting for a picked non-CQ station to call CQ after this long.
const WAIT_CQ_S: i64 = 300;
/// Keep a finished contact live this long to re-send the final message if the
/// DX repeats theirs (they didn't hear our 73 / RR73).
const CONFIRM_S: i64 = 300;
/// How long a call addressed to us is remembered, for deciding where a reply
/// the operator picks by hand opens. Long enough to cover a station calling
/// while we were busy elsewhere, short enough never to speak for a stale one.
const RECENT_CALL_S: i64 = 300;

/// The payload half of a message (`<to> <from> PAYLOAD`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Payload {
    Grid(String),
    Report(i16),
    RReport(i16),
    Rrr,
    Rr73,
    B73,
    /// A contest exchange: a two-digit RS glued to a serial number, and a
    /// six-character locator (issue #223). `rogered` is the leading `R`, which
    /// carries exactly what it carries in the everyday exchange — "I have yours"
    /// — and so is what decides whether the answer is another exchange or an
    /// RR73.
    Exchange {
        rs: u8,
        serial: u32,
        grid6: String,
        rogered: bool,
    },
    Other,
}

/// Split a contest exchange token — `590003` — into `(59, 3)`.
///
/// The RS is bounded because the layout's field is: three bits read back as
/// `52 + n`, so `51` or `60` is not a message anyone can have sent.
fn parse_exchange(tok: &str) -> Option<(u8, u32)> {
    if tok.len() != 6 || !tok.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let rs: u8 = tok[..2].parse().ok()?;
    let serial: u32 = tok[2..].parse().ok()?;
    (52..=59).contains(&rs).then_some((rs, serial))
}

/// A six-character locator in the alphabet the contest layout allows (the
/// sub-square letters run A..X).
fn is_grid6(t: &str) -> bool {
    let b = t.as_bytes();
    b.len() == 6
        && (b'A'..=b'R').contains(&b[0])
        && (b'A'..=b'R').contains(&b[1])
        && b[2].is_ascii_digit()
        && b[3].is_ascii_digit()
        && (b'A'..=b'X').contains(&b[4])
        && (b'A'..=b'X').contains(&b[5])
}

pub(crate) fn classify_payload(text: &str) -> Payload {
    let toks: Vec<&str> = text.split_whitespace().collect();
    let Some(p) = toks.get(2) else { return Payload::Other };
    // A contest exchange is two tokens, and the `R` in front of it is a third —
    // so it is read before the single-token payloads below, which would see
    // only its first half.
    let rogered = *p == "R";
    let rest = &toks[if rogered { 3 } else { 2 }..];
    if let [exch, grid6] = rest
        && let Some((rs, serial)) = parse_exchange(exch)
        && is_grid6(grid6)
    {
        return Payload::Exchange { rs, serial, grid6: (*grid6).to_string(), rogered };
    }
    match *p {
        "RR73" => Payload::Rr73,
        "RRR" => Payload::Rrr,
        "73" => Payload::B73,
        s if s.starts_with("R-") || s.starts_with("R+") => {
            s[1..].parse().map(Payload::RReport).unwrap_or(Payload::Other)
        }
        s if (s.starts_with('-') || s.starts_with('+')) && s[1..].parse::<i16>().is_ok() => {
            Payload::Report(
                s[1..].parse::<i16>().map(|v| if s.starts_with('-') { -v } else { v }).unwrap(),
            )
        }
        s if is_grid(s) => Payload::Grid(s.to_string()),
        _ => Payload::Other,
    }
}

/// Where our reply to `payload` belongs in the exchange — the message that
/// answers what they just sent us. `None` when nothing is owed: a bare 73
/// closes a contact rather than asking for anything.
fn reply_step(payload: &Payload) -> Option<QsoStep> {
    match payload {
        // They called us, with a grid or without one → send them a report.
        Payload::Grid(_) | Payload::Other => Some(QsoStep::TxReport),
        Payload::Report(_) => Some(QsoStep::TxRReport),
        Payload::RReport(_) => Some(QsoStep::TxRr73),
        // A contest exchange answers exactly as a report does: theirs bare asks
        // for ours rogered, theirs rogered says the exchange is complete.
        Payload::Exchange { rogered: false, .. } => Some(QsoStep::TxRReport),
        Payload::Exchange { rogered: true, .. } => Some(QsoStep::TxRr73),
        Payload::Rrr | Payload::Rr73 => Some(QsoStep::Tx73),
        Payload::B73 => None,
    }
}

/// An 11 m (CB) callsign, in the WSJT-CB grammar — `N{1,3}L{1,2}N{1,3}`, a
/// four-digit unit only behind a one-digit prefix, or `N{1,3}L{1,2}/L{2}`.
/// Delegated to mfsk-core so the validator and this sequencer can never drift.
#[inline]
fn is_cb(call: &str) -> bool {
    mfsk_core::msg::wsjt77::is_cb_callsign(call.trim())
}

/// The payload half of a WSJT-CB free-text line: "TOKEN0 [TOKEN1]" where
/// TOKEN0 is a CB callsign and TOKEN1 a report or sign-off. The wire limit is
/// thirteen characters, which leaves room for an address and exactly one such
/// token — a line with anything else, or no CB address, has no CB payload at
/// all (and so must not move an exchange, like any `Payload::Other`).
fn cb_payload(d: &Decode) -> Option<Payload> {
    if !d.free_text {
        return None;
    }
    let mut toks = d.message.split_whitespace();
    let t0 = toks.next()?;
    if !is_cb(t0) {
        return None;
    }
    let p = toks.next()?;
    if toks.next().is_some() {
        return None;
    }
    Some(match p {
        "RR73" => Payload::Rr73,
        "RRR" => Payload::Rrr,
        "73" => Payload::B73,
        s if s.starts_with("R-") || s.starts_with("R+") => {
            s[1..].parse().map(Payload::RReport).unwrap_or(Payload::Other)
        }
        s if (s.starts_with('-') || s.starts_with('+')) && s[1..].parse::<i16>().is_ok() => {
            let v: i16 = s[1..].parse().unwrap();
            Payload::Report(if s.starts_with('-') { -v } else { v })
        }
        _ => Payload::Other,
    })
}

/// The addressing of a WSJT-CB free-text line, which names its addressee
/// first: "26AT715 R-07" is that station speaking to the local one — the
/// sender is omitted outside the opening exchange — and "26AT715 1A1", or any
/// line whose *second* token is our call, is a reply to our CQ. When the
/// sender is not spelled out, the exchange's own peer stands in, exactly as
/// WSJT-CB keeps the selected DX when its parser reads back our own call.
///
/// `None` for free text with no CB addressing, so unaddressed free text keeps
/// its ordinary (decoration-only) treatment.
fn cb_address(
    d: &Decode,
    my_call: &str,
    dx: Option<&str>,
) -> Option<(Option<String>, Option<String>)> {
    if !d.free_text {
        return None;
    }
    let toks: Vec<&str> = d.message.split_whitespace().collect();
    let t0 = *toks.first()?;
    if !is_cb(t0) {
        return None;
    }
    let mine = my_call.trim();
    // Addressed to us by name.
    if t0.eq_ignore_ascii_case(mine) {
        // The second token, when it is a CB call that is not us, is who spoke;
        // otherwise a finished exchange omits the sender and it is the peer we
        // are already working.
        let from = toks
            .get(1)
            .filter(|t| is_cb(t) && !t.eq_ignore_ascii_case(mine))
            .copied()
            .or(dx)
            .map(str::to_string);
        return Some((Some(mine.to_string()), from));
    }
    // A reply to our CQ: "DXCALL MYCALL".
    if toks.get(1).is_some_and(|t| t.eq_ignore_ascii_case(mine)) {
        return Some((Some(mine.to_string()), Some(t0.to_string())));
    }
    None
}

/// The effective payload of a decode, honouring WSJT-CB free text: the CB
/// report/final is the *second* token of such a line, which the generic
/// `to <from> <payload>` classifier would look for in the wrong place.
fn payload_of(d: &Decode) -> Payload {
    if d.free_text {
        cb_payload(d).unwrap_or_else(|| classify_payload(&d.message))
    } else {
        classify_payload(&d.message)
    }
}

/// True when this decode is a station *calling* us: addressed to our station,
/// and not the tail of a contact already finished. A 73 or an RR73 is somebody
/// signing off, not somebody asking to be worked.
fn is_call_to_us(d: &Decode, payload: &Payload, my_call: &str) -> bool {
    d.to.as_deref() == Some(my_call)
        && d.from.as_deref().is_some_and(|f| !f.is_empty())
        && !matches!(payload, Payload::B73 | Payload::Rrr | Payload::Rr73)
}

fn is_grid(t: &str) -> bool {
    let b = t.as_bytes();
    b.len() == 4
        && b[0].is_ascii_uppercase()
        && b[1].is_ascii_uppercase()
        && b[2].is_ascii_digit()
        && b[3].is_ascii_digit()
}

/// True when this mode + config puts us on the Fox side of a pile-up.
///
/// FT8 and FT2 only. Those are the two the DXpedition layout is defined for —
/// the reference FT2 client says so in as many words ("Fox-and-Hound
/// operation is available only in FT8 and FT2 modes") and ships
/// `foxgenft2.f90` to generate the multi-slot waveform. FT4 has no Fox.
fn fox_role(mode: Mode, cfg: &DigiConfig) -> bool {
    matches!(mode, Mode::Ft8 | Mode::Ft2) && cfg.dxped_mode == DxpedMode::Fox
}

#[derive(Debug, Clone)]
struct Dx {
    call: String,
    grid: Option<String>,
    rpt_sent: Option<i16>, // report we sent them (their SNR at us)
    rpt_rcvd: Option<i16>, // report they sent us
    /// The contest exchange they sent us — `(RS, serial, locator)` — kept
    /// whole because all three of them go in the log, and the serial is the
    /// only part of a contest QSO that cannot be worked out again afterwards.
    exch_rcvd: Option<(u8, u32, String)>,
    /// The serial we sent them. Read off `DigiConfig::contest_serial` when the
    /// exchange first goes out rather than at log time, so a contact that
    /// straddles the engine advancing the counter is logged with the number
    /// that actually went on the air.
    serial_sent: Option<u32>,
    started_utc: i64,
    last_utc: i64,
}

pub struct QsoMachine {
    cfg: DigiConfig,
    mode: Mode,
    step: QsoStep,
    dx: Option<Dx>,
    audio_hz: f32,
    tx_even: bool,
    /// The current QSO's message exchange (TX and RX lines).
    transcript: Vec<TranscriptLine>,
    /// QSOs that just completed and should be logged. A queue rather than one
    /// slot because a Fox can finish several contacts in a single transmission.
    completed: std::collections::VecDeque<QsoRecord>,
    /// Deadline while in [`QsoStep::WaitCq`] / [`QsoStep::Confirming`] (Unix s).
    deadline_utc: i64,
    /// The final message (73 / RR73) we sent, re-sent while `Confirming` if the
    /// DX repeats theirs.
    final_msg: Option<String>,
    /// A re-send of `final_msg` is queued for the next transmit slot.
    resend: bool,
    /// This contact has already produced a log entry. The final message can
    /// legitimately go out more than once — the operator picks RR73 again when
    /// they think the DX missed it — and every send of it lands back in
    /// [`log_qso`](Self::log_qso), which would otherwise log the contact twice.
    logged: bool,
    /// A message the operator queued by hand. It takes the next transmit slot
    /// whatever the sequencer had planned, then the exchange carries on from
    /// where it was.
    manual: Option<String>,
    /// Callsigns, and the DXCC entities they were in, worked this session —
    /// what makes one answer to our CQ worth more than another.
    worked_calls: std::collections::HashSet<String>,
    worked_entities: std::collections::HashSet<String>,
    /// When something last counted as progress: a reply, or the operator doing
    /// anything at all. 0 means "not stamped yet" — operator actions have no
    /// clock of their own, so the next tick stamps them.
    progress_utc: i64,
    /// Transmissions since that moment.
    tx_since_progress: u32,
    /// The watchdog stopped the sequencer; cleared by the next operator action.
    watchdog: bool,
    /// Fox (DXpedition) pile-up, present only while running that role. It takes
    /// over receive handling and transmit planning wholesale — a Fox works a
    /// queue, not the single contact the rest of this machine models.
    fox: Option<Fox>,
    /// Hound: the Fox's own tone offset, learned from the message that answered
    /// us. A Hound calls from above 1000 Hz but finishes the contact down on the
    /// Fox's frequency, so this is a move the controller must make for us.
    qsy_hz: Option<f32>,
    /// Stations the operator marked to work, in the order they will be taken.
    queue: std::collections::VecDeque<QueuedCall>,
    /// What each station lately sent *us*, and when. This is what says where a
    /// contact stands when the operator picks one to answer by hand: somebody
    /// who called us with a report wants our R+report back, and opening with
    /// our grid would ask again for the report they already sent.
    called_us: std::collections::HashMap<String, (i64, Payload)>,
}

impl QsoMachine {
    pub fn new(mode: Mode, cfg: DigiConfig) -> Self {
        let tx_even = cfg.tx_even;
        let fox = fox_role(mode, &cfg).then(Fox::new);
        QsoMachine {
            cfg,
            mode,
            step: QsoStep::Idle,
            dx: None,
            audio_hz: 1500.0,
            tx_even,
            transcript: Vec::new(),
            completed: std::collections::VecDeque::new(),
            deadline_utc: 0,
            final_msg: None,
            resend: false,
            logged: false,
            manual: None,
            worked_calls: std::collections::HashSet::new(),
            worked_entities: std::collections::HashSet::new(),
            progress_utc: 0,
            tx_since_progress: 0,
            watchdog: false,
            fox,
            qsy_hz: None,
            queue: std::collections::VecDeque::new(),
            called_us: std::collections::HashMap::new(),
        }
    }

    pub fn set_config(&mut self, cfg: DigiConfig) {
        self.tx_even = cfg.tx_even;
        // Entering Fox mode starts an empty pile-up; leaving it discards one.
        // Staying in it must not, or every config edit would drop the queue.
        match (fox_role(self.mode, &cfg), self.fox.is_some()) {
            (true, false) => self.fox = Some(Fox::new()),
            (false, true) => self.fox = None,
            _ => {}
        }
        self.cfg = cfg;
    }

    /// Which side of a DXpedition pile-up we are operating. Always `Normal`
    /// outside FT8 and FT2 — no other mode has a DXpedition layout to speak.
    pub fn dxped(&self) -> DxpedMode {
        if matches!(self.mode, Mode::Ft8 | Mode::Ft2) {
            self.cfg.dxped_mode
        } else {
            DxpedMode::Normal
        }
    }

    /// Hound: the Fox's tone offset to move onto, taken once.
    pub fn take_qsy(&mut self) -> Option<f32> {
        self.qsy_hz.take()
    }

    /// Mark a station to work. Queueing one already in the queue moves it to
    /// the end — a second press is a re-ordering, not a duplicate.
    pub fn queue_add(&mut self, entry: QueuedCall) {
        self.queue.retain(|q| !q.call.eq_ignore_ascii_case(&entry.call));
        self.queue.push_back(entry);
    }

    /// Drop a station from the queue; an empty callsign clears it.
    pub fn queue_remove(&mut self, call: &str) {
        if call.trim().is_empty() {
            self.queue.clear();
        } else {
            self.queue.retain(|q| !q.call.eq_ignore_ascii_case(call.trim()));
        }
    }

    /// The next queued station to call, once the sequencer has finished with
    /// the last one. Taking it removes it from the queue, so the caller must
    /// actually start the contact.
    ///
    /// Returns `None` while anything is in progress — and while the transmit
    /// watchdog is up, because the watchdog exists to stop an unattended
    /// station transmitting and marching down a queue would defeat it.
    pub fn take_next_queued(&mut self) -> Option<QueuedCall> {
        self.is_free().then(|| self.queue.pop_front()).flatten()
    }

    /// True when nothing is in progress and the queue may take the sequencer.
    fn is_free(&self) -> bool {
        if self.manual.is_some() || self.watchdog || self.fox.is_some() {
            return false;
        }
        match self.step {
            QsoStep::Idle => true,
            // Calling into an empty band: a station we marked is a better use
            // of the slot. Once someone answers, `dx` is set and we are busy.
            QsoStep::CallingCq => self.dx.is_none(),
            // Logged already; only the re-send window holds the contact open,
            // and an unclaimed re-send is not worth delaying the next contact.
            QsoStep::Confirming => !self.resend,
            _ => false,
        }
    }

    pub fn set_audio_hz(&mut self, hz: f32) {
        self.audio_hz = hz;
    }

    pub fn step(&self) -> QsoStep {
        self.step
    }

    /// The callsign of the station we're currently working, if any.
    pub fn dx_call(&self) -> Option<&str> {
        self.dx.as_ref().map(|d| d.call.as_str())
    }

    /// Our own station callsign.
    pub fn my_call(&self) -> &str {
        &self.cfg.my_call
    }

    /// Note that the sequencer got somewhere: restart the watchdog's clock and
    /// the unanswered-call count.
    ///
    /// This is progress *on the air* — a reply arrived — and it deliberately
    /// leaves [`watchdog`](Self::tx_watchdog) alone. Once the watchdog has
    /// stopped the sequencer, only the operator starts it again: the whole
    /// point of it is that an unattended station stops transmitting, and a
    /// station that resumed the moment somebody called it would be unattended
    /// and transmitting. Clearing the flag here also cleared the readout that
    /// said why the sequencer had stopped, and released the call queue to march
    /// on to the next station.
    ///
    /// The timestamp is left for the next [`tick`](Self::tick) to fill in,
    /// because operator actions arrive without a clock.
    fn progress(&mut self) {
        self.progress_utc = 0;
        self.tx_since_progress = 0;
    }

    /// The operator did something — called CQ, picked a message, queued text,
    /// answered a station. That is the one thing that takes the watchdog down,
    /// on top of everything [`progress`](Self::progress) does.
    fn operator_acted(&mut self) {
        self.progress();
        self.watchdog = false;
    }

    /// True when the transmit watchdog has stopped the sequencer.
    pub fn tx_watchdog(&self) -> bool {
        self.watchdog
    }

    /// Start calling CQ.
    pub fn call_cq(&mut self) {
        self.dx = None;
        self.transcript.clear();
        self.final_msg = None;
        self.resend = false;
        self.logged = false;
        self.manual = None;
        self.operator_acted();
        self.step = QsoStep::CallingCq;
    }

    /// Jump the exchange to `step`, the way WSJT-X's Tx1–Tx6 buttons do: the
    /// operator picks which message goes next and the sequencer carries on from
    /// there. Steps that address a station need one; `Idle` stands down.
    pub fn set_step(&mut self, step: QsoStep) -> bool {
        let needs_dx = !matches!(step, QsoStep::Idle | QsoStep::CallingCq);
        if needs_dx && self.dx.is_none() {
            return false;
        }
        if step == QsoStep::CallingCq {
            self.call_cq();
            return true;
        }
        self.manual = None;
        self.operator_acted();
        self.step = step;
        true
    }

    /// Send `text` verbatim in the next transmit slot, then carry on with the
    /// exchange. Empty text cancels a message queued but not yet sent.
    pub fn queue_text(&mut self, text: String) {
        let text = text.trim().to_ascii_uppercase();
        self.manual = (!text.is_empty()).then_some(text);
        self.operator_acted();
    }

    /// Record a message we transmitted (called by the controller when it
    /// actually keys the burst).
    pub fn record_tx(&mut self, text: &str) {
        self.transcript.push(TranscriptLine::sent(text));
    }

    /// Engage a decoded station. `snr` is their signal at us — the report we'll
    /// send. When `wait_for_cq` we hold in [`QsoStep::WaitCq`] (the operator
    /// picked a station that isn't calling CQ and isn't calling us) and only
    /// start transmitting once they call CQ or address us; otherwise we start
    /// replying right away.
    ///
    /// A station that has recently called *us* is answered from where their
    /// exchange stands rather than from the top — somebody repeating
    /// `<us> <them> -19` at us is owed our R+report, not our grid.
    pub fn start_qso(
        &mut self,
        from: String,
        grid: Option<String>,
        snr: i16,
        wait_for_cq: bool,
        now_utc: i64,
    ) {
        self.transcript.clear();
        self.final_msg = None;
        self.resend = false;
        self.logged = false;
        self.manual = None;
        self.operator_acted();
        // Working them now settles whatever the queue had them down for.
        self.queue.retain(|q| !q.call.eq_ignore_ascii_case(&from));
        // What they last sent us, if they have been calling recently: it decides
        // both the opening message and the report we already hold from them.
        // Anything older is a contact of its own and speaks for nothing here.
        let heard = self
            .called_us
            .get(&from.to_ascii_uppercase())
            .filter(|(t, _)| now_utc - *t <= RECENT_CALL_S)
            .map(|(_, p)| p.clone());
        let rpt_rcvd = match heard {
            Some(Payload::Report(r) | Payload::RReport(r)) => Some(r),
            _ => None,
        };
        let exch_rcvd = match &heard {
            Some(Payload::Exchange { rs, serial, grid6, .. }) => {
                Some((*rs, *serial, grid6.clone()))
            }
            _ => None,
        };
        self.dx = Some(Dx {
            call: from,
            grid,
            rpt_sent: Some(snr),
            rpt_rcvd,
            exch_rcvd,
            serial_sent: None,
            started_utc: now_utc,
            last_utc: now_utc,
        });
        // A Hound never waits for the Fox to be free: it never is. Calling into
        // the pile-up is exactly the operation, so start straight away.
        if wait_for_cq && self.dxped() != DxpedMode::Hound {
            self.step = QsoStep::WaitCq;
            self.deadline_utc = now_utc + WAIT_CQ_S;
        } else {
            self.step = heard.as_ref().and_then(reply_step).unwrap_or(QsoStep::TxGrid);
        }
    }

    /// Graceful stop: no new bursts planned, revert to idle.
    pub fn stop(&mut self) {
        self.step = QsoStep::Idle;
        self.manual = None;
    }

    /// Hard reset.
    pub fn abort(&mut self) {
        self.step = QsoStep::Idle;
        self.dx = None;
        self.manual = None;
        self.logged = false;
        self.watchdog = false;
    }

    /// True while we intend to transmit this cycle. `WaitCq` holds silently
    /// until the DX calls CQ; `Confirming` transmits only when a re-send of our
    /// final message is queued.
    pub fn wants_tx(&self) -> bool {
        if self.manual.is_some() && !self.cfg.my_call.trim().is_empty() {
            return true; // the operator asked for this one explicitly
        }
        // A running Fox always has something to send — a report, an RR73, or a
        // CQ to draw the next callers. Idle means the operator stood it down.
        if self.fox.is_some() {
            return self.step == QsoStep::CallingCq && !self.cfg.my_call.trim().is_empty();
        }
        match self.step {
            QsoStep::Idle | QsoStep::WaitCq => false,
            QsoStep::Confirming => self.resend,
            _ => true,
        }
    }

    /// Advance timeouts (called each engine tick). Returns true if the state
    /// changed: gives up a stale `WaitCq`, and retires a `Confirming` contact
    /// once its re-send window elapses.
    pub fn tick(&mut self, now_utc: i64) -> bool {
        if self.progress_utc == 0 {
            self.progress_utc = now_utc;
        }
        // Transmit watchdog: nothing has come back and nobody has touched the
        // controls for the configured span, so stop calling. The contact stays
        // on screen — picking a message or calling CQ resumes.
        let limit = self.cfg.tx_watchdog_min as i64 * 60;
        if limit > 0 && !self.watchdog && self.wants_tx() && now_utc - self.progress_utc >= limit {
            self.watchdog = true;
            self.manual = None;
            self.step = QsoStep::Idle;
            self.transcript.push(TranscriptLine::note(format!(
                "transmit watchdog: {} minutes with no progress",
                self.cfg.tx_watchdog_min
            )));
            return true;
        }
        match self.step {
            // A re-send the DX asked for holds the contact open past its
            // deadline. It is one transmission, already earned by them
            // repeating their final message, and retiring on top of it would
            // drop the answer rather than send it — `note_tx_sent` clears the
            // flag as it leaves, and the next tick retires as usual.
            QsoStep::Confirming if self.resend => false,
            QsoStep::WaitCq | QsoStep::Confirming if now_utc >= self.deadline_utc => {
                self.step = QsoStep::Idle;
                self.dx = None;
                self.resend = false;
                self.final_msg = None;
                self.logged = false;
                true
            }
            _ => false,
        }
    }

    /// Fold in decodes from a slot; advance the exchange when the DX replied
    /// to us. While calling CQ, the first station to answer us is adopted as
    /// the DX. Returns true if the state changed.
    pub fn on_rx(&mut self, decodes: &[Decode], now_utc: i64) -> bool {
        let my_call = self.cfg.my_call.clone();
        if my_call.is_empty() {
            return false;
        }
        // A Fox has no single exchange to advance: everything addressed to it is
        // pile-up traffic, handled by the queue.
        if let Some(fox) = self.fox.as_mut() {
            let changed = fox.on_rx(decodes, &my_call, now_utc);
            if changed {
                self.progress();
                self.progress_utc = now_utc;
            }
            return changed;
        }
        // WSJT-CB free-text traffic does not carry its addressing in the
        // layout: "26AT715 R-07" is addressed *by the first token*, to the
        // station whose call it is. The rest of this method only understands
        // `to`/`from` fields, so every decode passes through the CB rewrite
        // first — the addressing stays as decoded (free text, no addressee)
        // unless it actually names this station, and the payload is read the
        // CB way (second token) wherever it might be.
        let effective: Vec<(Decode, Payload)> = decodes
            .iter()
            .map(|d| {
                let (to, from) = cb_address(d, &my_call, self.dx.as_ref().map(|x| x.call.as_str()))
                    .unwrap_or((None, None));
                let mut e = d.clone();
                if to.is_some() || from.is_some() {
                    e.to = to;
                    e.from = from;
                }
                (e, payload_of(d))
            })
            .collect();
        // Remember who has been calling us and what they sent, whether or not
        // it touches the contact in hand. A station calling while we are busy
        // elsewhere — or stood down — is one the operator may pick later, and
        // this is the only record of where their exchange had got to.
        for (d, payload) in &effective {
            if let Some(from) = d.from.as_deref().filter(|_| is_call_to_us(d, payload, &my_call)) {
                self.called_us.insert(from.to_ascii_uppercase(), (now_utc, payload.clone()));
            }
        }
        // Housekeeping only — `start_qso` checks the age itself, since a silent
        // band means no decodes and so no chance to run this.
        self.called_us.retain(|_, (t, _)| now_utc - *t <= RECENT_CALL_S);
        // A pileup can decode both their answer to us *and* other traffic from
        // them in the same slot; an answer to us always wins over "they're
        // working someone else".
        let mut changed = false;
        let dx_call = self.dx.as_ref().map(|d| d.call.clone());
        let answered_us = dx_call.as_deref().is_some_and(|dx| {
            effective
                .iter()
                .any(|(d, _)| d.from.as_deref() == Some(dx) && d.to.as_deref() == Some(my_call.as_str()))
        });
        // A CQ can be answered by several stations in one slot. Choose between
        // them once, up front, rather than taking whichever the decoder
        // happened to report first.
        let answerer = (self.step == QsoStep::CallingCq && self.dx.is_none())
            .then(|| self.pick_answerer(&effective, &my_call))
            .flatten();
        if let Some(pick) = &answerer {
            let others: Vec<&str> = effective
                .iter()
                .filter(|(d, p)| is_call_to_us(d, p, &my_call))
                .filter_map(|(d, _)| d.from.as_deref())
                .filter(|c| c != pick)
                .collect();
            if !others.is_empty() {
                // The ones we're not taking are worth knowing about: they are
                // still calling, and the decode list will have scrolled on.
                self.transcript
                    .push(TranscriptLine::note(format!("also calling: {}", others.join(", "))));
                changed = true;
            }
        }
        let hound = self.dxped() == DxpedMode::Hound;
        for (d, payload) in &effective {
            let Some(from) = d.from.as_deref().filter(|f| !f.is_empty()) else { continue };
            let to_me = d.to.as_deref() == Some(my_call.as_str());

            // Hound: the Fox closes our contact inside a message addressed to
            // the *next* Hound ("<us> RR73; W9XYZ <fox> +03"), so `to` never
            // names us. This is the only place a Hound's QSO completes.
            if hound
                && d.rr73_to.as_deref() == Some(my_call.as_str())
                && self.dx.as_ref().map(|x| x.call.as_str()) == Some(from)
                && !matches!(self.step, QsoStep::Idle | QsoStep::Confirming)
            {
                self.transcript.push(TranscriptLine::rcvd(d.message.clone()));
                self.log_qso(now_utc);
                changed = true;
                continue;
            }
            // Our station answering someone else: they're in another exchange.
            let other = (!to_me && !answered_us && !d.is_cq && dx_call.as_deref() == Some(from))
                .then(|| d.to.as_deref().filter(|t| !t.is_empty() && *t != my_call))
                .flatten();

            // Waiting for our picked station: start replying once they call CQ
            // (they're free) or address us directly (no need to keep waiting).
            if self.step == QsoStep::WaitCq {
                if self.dx.as_ref().map(|x| x.call.as_str()) == Some(from) && (d.is_cq || to_me) {
                    // Where to pick the exchange up. Their CQ says nothing
                    // about us, so we open with our grid; anything addressed to
                    // us is answered from where *they* are, because resuming at
                    // Tx1 would send our grid to a station that already has it
                    // and is waiting on a report. A bare 73 asks for nothing at
                    // all, so it is not the thing we were waiting for.
                    let Some(resume) =
                        (if d.is_cq { Some(QsoStep::TxGrid) } else { reply_step(payload) })
                    else {
                        continue;
                    };
                    if let Some(dx) = self.dx.as_mut() {
                        dx.last_utc = now_utc;
                        if let Payload::Grid(g) = payload
                            && dx.grid.is_none()
                        {
                            dx.grid = Some(g.clone());
                        }
                    }
                    if let Payload::Report(r) | Payload::RReport(r) = payload {
                        self.set_rcvd(*r);
                    }
                    self.set_exch_rcvd(payload);
                    // They are on the air and free: that is the thing we were
                    // waiting for, so the calls we made before they went quiet
                    // must not count against the ones we are about to make.
                    self.progress();
                    self.progress_utc = now_utc;
                    self.step = resume;
                    changed = true;
                } else if let Some(other) = other {
                    // Still busy — keep the log showing who with.
                    changed |= self.note_working(from, other);
                }
                continue;
            }

            // They took someone else's call: stop calling and hold until they
            // are free again rather than doubling into their exchange. Only
            // while our own exchange is still unfinished — once we owe them a
            // 73 / RR73 that message goes out regardless, so the contact gets
            // completed and logged.
            //
            // `TxReport` counts as much as the other two. It is the step a CQ
            // caller sits in, and a station that answers a CQ and then goes off
            // with somebody else leaves us reporting into their QSO for as many
            // slots as the unanswered-call count allows.
            //
            // A Hound is exempt: a Fox is *always* working somebody, and a Hound
            // that stood down each time would never be answered at all.
            if let Some(other) = other.filter(|_| !hound)
                && matches!(self.step, QsoStep::TxGrid | QsoStep::TxReport | QsoStep::TxRReport)
            {
                self.note_working(from, other);
                self.step = QsoStep::WaitCq;
                self.deadline_utc = now_utc + WAIT_CQ_S;
                changed = true;
                continue;
            }

            if !to_me {
                continue; // everything else must be addressed to us
            }

            // Contact just logged: if the DX repeats their message they didn't
            // hear our final one — queue a single re-send. A bare 73 means they
            // got it and are closing, so nothing to do.
            if self.step == QsoStep::Confirming {
                if self.dx.as_ref().map(|x| x.call.as_str()) == Some(from) {
                    self.transcript.push(TranscriptLine::rcvd(d.message.clone()));
                    if !matches!(payload, Payload::B73) {
                        self.resend = true;
                    }
                    changed = true;
                }
                continue;
            }

            // Calling CQ and someone answers → adopt the best of them.
            if self.step == QsoStep::CallingCq
                && self.dx.is_none()
                && answerer.as_deref() == Some(from)
            {
                let grid = match &payload {
                    Payload::Grid(g) => Some(g.clone()),
                    _ => d.grid.clone(),
                };
                self.dx = Some(Dx {
                    call: from.to_string(),
                    grid,
                    rpt_sent: Some(d.snr_db),
                    rpt_rcvd: None,
                    exch_rcvd: None,
                    serial_sent: None,
                    started_utc: now_utc,
                    last_utc: now_utc,
                });
                self.logged = false; // a new contact, whatever the last one did
                self.transcript.push(TranscriptLine::rcvd(d.message.clone()));
                // Somebody came back to us: the CQ run is over and a fresh
                // contact starts here. Without this the CQs we sent waiting for
                // them stay on the unanswered-call count, and a run long enough
                // to reach `max_tx_repeats` gives up on the very station that
                // answered it — before ever sending them a report.
                self.progress();
                self.progress_utc = now_utc;
changed |= self.advance(payload, now_utc);
                // Whatever they sent, we are working them now. An answer we
                // could not parse is still an answer, and staying in `CallingCq`
                // would leave them adopted but never replied to — and, because
                // the pick above only runs while `dx` is none, would wedge the
                // CQ run so nobody else could be taken either.
                if self.step == QsoStep::CallingCq {
                    self.step = QsoStep::TxReport;
                    changed = true;
                }
                continue;
            }

            // Otherwise only the station we're working advances us.
            if self.dx.as_ref().map(|d| d.call.as_str()) != Some(from) {
                continue;
            }
            if let Some(dx) = self.dx.as_mut() {
                dx.last_utc = now_utc;
                if let Payload::Grid(g) = payload
                    && dx.grid.is_none()
                {
                    dx.grid = Some(g.clone());
                }
                if dx.rpt_sent.is_none() {
                    dx.rpt_sent = Some(d.snr_db);
                }
            }
            // Hound: the Fox came back to us, so finish the contact on its
            // frequency rather than up in the calling zone where we started.
            if hound && matches!(payload, Payload::Report(_) | Payload::RReport(_)) {
                self.qsy_hz = Some(d.audio_hz);
            }
            self.transcript.push(TranscriptLine::rcvd(d.message.clone()));
            self.progress();
            self.progress_utc = now_utc;
            changed |= self.advance(payload, now_utc);
        }
        changed
    }

    /// Which of the stations answering our CQ to work first.
    ///
    /// Signal strength decides between comparable stations, but not across the
    /// board: a station we have already worked this session goes last whatever
    /// its signal, and among signals of similar strength (6 dB bands — the
    /// difference between "solid" and "marginal" matters, a couple of dB does
    /// not) a new DXCC entity is worth more than a repeat one. Beyond that the
    /// strongest wins, because it is the one most likely to complete.
    fn pick_answerer(&self, decodes: &[(Decode, Payload)], my_call: &str) -> Option<String> {
        decodes
            .iter()
            .filter(|(d, p)| is_call_to_us(d, p, my_call))
            .filter_map(|(d, _)| {
                let call = d.from.as_deref().filter(|f| !f.is_empty())?;
                Some((call, d.snr_db))
            })
            .max_by_key(|(call, snr)| {
                let fresh = !self.worked_calls.contains(&call.to_ascii_uppercase());
                let new_entity = sdroxide_types::entity_name(call)
                    .is_some_and(|e| !self.worked_entities.contains(e));
                (fresh, snr.div_euclid(6), new_entity, *snr)
            })
            .map(|(call, _)| call.to_string())
    }

    /// Note in the transcript that the station we called is working someone
    /// else. Deduplicated against the line before it, so holding through a whole
    /// exchange adds one line per partner rather than one per slot. Returns true
    /// if a line was added.
    fn note_working(&mut self, dx: &str, other: &str) -> bool {
        let text = format!("{dx} is working {other}");
        if self.transcript.last().is_some_and(|l| l.overheard && l.text == text) {
            return false;
        }
        self.transcript.push(TranscriptLine::note(text));
        true
    }

    fn advance(&mut self, payload: &Payload, now_utc: i64) -> bool {
        // Hound: the Fox's RR73 ends the contact there and then. Sending a 73
        // back would only take a slot from the pile-up — the Fox has already
        // logged us and moved on.
        if self.dxped() == DxpedMode::Hound
            && self.step == QsoStep::TxRReport
            && matches!(payload, Payload::Rrr | Payload::Rr73 | Payload::B73)
        {
            self.log_qso(now_utc);
            return true;
        }
        // Stood down — by the operator, the watchdog, or the unanswered-call
        // count. The DX turning up again is worth showing, but not worth
        // starting to transmit over: a stand-down that the far end can undo is
        // no stand-down at all. The operator resumes it, from the message they
        // pick or the REPLY that re-opens the contact.
        if self.step == QsoStep::Idle {
            return false;
        }
        // Their report is their report wherever the exchange had got to.
        if let Payload::Report(r) | Payload::RReport(r) = payload {
            self.set_rcvd(*r);
        }
        self.set_exch_rcvd(payload);
        let prev = self.step;
        match payload {
            // Free text, a bare call, anything the packer mangled: it says
            // nothing about where they are in the exchange, so it must not move
            // us — least of all backwards. "TNX QSO 73 GL" arriving while we
            // are sending 73 is not a request to start again from a report.
            // (Answering our CQ is the one case where an unreadable message is
            // still information — `on_rx` handles that, since there what makes
            // it an answer is that it was addressed to us at all.)
            Payload::Other => {}
            // A bare 73 closes the contact rather than asking for anything. If
            // we still owe a final message it goes out and logs the QSO on its
            // own; the one step that would otherwise wait for ever is
            // TxRReport, where their 73 does the job of the RR73 we expected.
            Payload::B73 => {
                if self.step == QsoStep::TxRReport {
                    self.step = QsoStep::Tx73;
                }
            }
            // Everything else: send the message that answers what they actually
            // sent. That is the whole rule, and it is the same one
            // [`reply_step`] states for a contact the operator opens by hand.
            //
            // It is deliberately blind to where *we* thought we were. Matching
            // on both sides — only accepting the reply the sequence predicted —
            // is what used to wedge these exchanges: two stations both sending
            // bare reports, or both sending R+report, each waiting for a message
            // the other had already moved past, until the unanswered-call count
            // gave up. When it moves us backwards, that is the point: they never
            // heard the message we have moved past, so we send it again.
            _ => {
                if let Some(step) = reply_step(payload) {
                    self.step = step;
                }
            }
        }
        self.step != prev
    }

    fn set_rcvd(&mut self, r: i16) {
        if let Some(dx) = self.dx.as_mut() {
            dx.rpt_rcvd = Some(r);
        }
    }

    /// Keep the contest exchange they sent, and the locator with it.
    ///
    /// The locator lands in `grid` as well as in the exchange: the map, the
    /// distance and the awards all read that field, and a six-character one is
    /// simply a better answer to the same question than the four-character grid
    /// the opening messages carried.
    fn set_exch_rcvd(&mut self, payload: &Payload) {
        let Payload::Exchange { rs, serial, grid6, .. } = payload else { return };
        if let Some(dx) = self.dx.as_mut() {
            dx.exch_rcvd = Some((*rs, *serial, grid6.clone()));
            dx.grid = Some(grid6.clone());
        }
    }

    /// Log the QSO and enter [`QsoStep::Confirming`], keeping the DX so we can
    /// re-send our final message for a few minutes if they didn't hear it.
    ///
    /// Only the first call for a given contact writes a log entry. Sending the
    /// final message again lands here again — which is what put a second
    /// identical record in the logbook, and from there in every ADIF export and
    /// upload, whenever the operator re-sent an RR73 they thought was missed.
    fn log_qso(&mut self, now_utc: i64) {
        if let Some(dx) = self.dx.as_ref().filter(|_| !self.logged) {
            self.worked_calls.insert(dx.call.to_ascii_uppercase());
            if let Some(e) = sdroxide_types::entity_name(&dx.call) {
                self.worked_entities.insert(e.to_string());
            }
            // A contest contact logs what was actually exchanged (issue #223).
            // The reports become the two digits of the RS each side sent — the
            // way every contest log reads, and the way WSJT-X writes them —
            // rather than the signal-to-noise ratio, which is not what went
            // over. `SRX`/`STX` carry the serial numbers and the `_STRING`
            // fields the exchange whole, which is what a checker wants to see.
            let (mut rst_sent, mut rst_rcvd) = (dx.rpt_sent, dx.rpt_rcvd);
            let (mut srx, mut stx) = (None, None);
            let (mut srx_string, mut stx_string) = (String::new(), String::new());
            if self.contest() == ContestMode::EuVhf {
                let rs_sent = sdroxide_types::eu_vhf_rs(dx.rpt_sent.unwrap_or(0));
                let serial = self.serial_for_this_qso();
                rst_sent = Some(i16::from(rs_sent));
                stx = Some(serial);
                stx_string =
                    format!("{rs_sent:02}{serial:04} {}", self.cfg.my_grid.to_ascii_uppercase());
                if let Some((rs, ser, grid6)) = &dx.exch_rcvd {
                    rst_rcvd = Some(i16::from(*rs));
                    srx = Some(*ser);
                    srx_string = format!("{rs:02}{ser:04} {grid6}");
                }
            }
            self.completed.push_back(QsoRecord {
                call: dx.call.clone(),
                grid: dx.grid.clone(),
                rst_sent,
                rst_rcvd,
                freq_hz: 0.0, // filled by the controller (needs dial freq)
                mode: self.mode.label().to_string(),
                band: String::new(), // filled by the controller
                start_utc: dx.started_utc,
                end_utc: now_utc,
                my_call: self.cfg.my_call.clone(),
                my_grid: self.cfg.my_grid.clone(),
                srx,
                stx,
                srx_string,
                stx_string,
                ..Default::default() // id assigned by the logbook, no comment
            });
            // Say so in the transcript. The exchange stays on screen for the
            // operator to read, and the step alone ("Confirming") is not the
            // word anyone uses for a contact that is finished and in the log.
            self.transcript
                .push(TranscriptLine::complete(format!("QSO with {} complete — logged", dx.call)));
        }
        self.logged = true;
        self.step = QsoStep::Confirming;
        self.deadline_utc = now_utc + CONFIRM_S;
        self.resend = false;
    }

    /// Everything to transmit this slot, each message with the tone offset it
    /// goes out on. The everyday sequencer sends exactly one signal at the
    /// operator's frequency; a Fox sends up to five, spaced 60 Hz apart.
    pub fn plan_tx_all(&self) -> Vec<(String, f32)> {
        if self.cfg.my_call.trim().is_empty() {
            return Vec::new();
        }
        // A hand-queued message takes the slot on its own, Fox or not: the
        // operator typed it to be heard, not to be one of five.
        if let Some(m) = &self.manual {
            return vec![(m.clone(), self.audio_hz)];
        }
        if let Some(plan) = self.fox_plan() {
            return plan
                .messages
                .into_iter()
                .enumerate()
                .map(|(i, m)| (m, self.audio_hz + i as f32 * slot_spacing_hz(self.mode)))
                .collect();
        }
        self.plan_tx().map(|m| vec![(m, self.audio_hz)]).unwrap_or_default()
    }

    /// This slot's Fox transmission, or `None` when we aren't a running Fox.
    fn fox_plan(&self) -> Option<crate::fox::FoxPlan> {
        self.fox
            .as_ref()
            .filter(|_| self.step == QsoStep::CallingCq && !self.cfg.my_call.trim().is_empty())
            .map(|f| f.plan(&self.cfg))
    }

    /// The message to transmit this slot, or None if we shouldn't key. In Fox
    /// mode this is the first of the [`plan_tx_all`](Self::plan_tx_all) signals
    /// — what the operator's "next transmission" readout shows.
    pub fn plan_tx(&self) -> Option<String> {
        if self.fox.is_some() && self.manual.is_none() {
            return self.fox_plan().and_then(|p| p.messages.into_iter().next());
        }
        // Nothing goes out without a station callsign: every message is built
        // around ours, and an unconfigured station must never key. (The message
        // packer degrades unpackable text to free text, so this is the guard
        // that keeps a bare "CQ" off the air.)
        if self.cfg.my_call.trim().is_empty() {
            return None;
        }
        if let Some(m) = &self.manual {
            return Some(m.clone());
        }
        let dx = self.dx.as_ref();
        let dx_call = dx.map(|d| d.call.as_str()).unwrap_or("");
        let mc = &self.cfg.my_call;
        // FT8/FT4 use the 4-character Maidenhead locator; a 6-char grid like
        // "JN78ve" is truncated to "JN78" for the transmitted message.
        let mg: String = self.cfg.my_grid.chars().take(4).collect();
        let rpt_sent = dx.and_then(|d| d.rpt_sent);
        if self.contest() == ContestMode::EuVhf {
            return self.plan_eu_vhf(dx_call, &mg, rpt_sent);
        }
        let fill = |tmpl: &str, rpt: Option<i16>| DigiConfig::fill(tmpl, mc, &mg, dx_call, rpt);
        // A WSJT-CB exchange between two 11 m stations: both callsigns are
        // non-standard, so nothing has a layout to carry it, and free text has
        // room for an address and one narrow token — which is why WSJT-CB
        // itself says a CB pair "cannot reliably carry signal reports" and
        // falls back to a short deterministic exchange. Plan that vocabulary:
        // CQ, the address pair, report, rogered report, RR73 and 73, each line
        // inside the 13-character limit.
        if is_cb(mc) && self.dx.as_ref().is_none_or(|d| is_cb(&d.call)) {
            let rpt = rpt_sent.map(sdroxide_types::fmt_report);
            return match self.step {
                QsoStep::CallingCq => Some(format!("CQ {}", mc.trim())),
                QsoStep::TxGrid => Some(format!("{dx_call} {}", mc.trim())),
                QsoStep::TxReport => Some(format!(
                    "{dx_call} {}",
                    rpt.as_deref().unwrap_or(mc.trim())
                )),
                QsoStep::TxRReport => Some(format!(
                    "{dx_call} R{}",
                    rpt.as_deref().unwrap_or(mc.trim())
                )),
                QsoStep::TxRr73 => Some(format!("{dx_call} RR73")),
                QsoStep::Tx73 => Some(format!("{dx_call} 73")),
                QsoStep::Confirming => self.resend.then(|| self.final_msg.clone()).flatten(),
                QsoStep::Idle | QsoStep::WaitCq => None,
            };
        }
        match self.step {
            QsoStep::CallingCq => Some(fill(&self.cfg.msg_cq, None)),
            QsoStep::TxGrid => Some(fill(&self.cfg.msg_grid, None)),
            QsoStep::TxReport => Some(fill(&self.cfg.msg_report, rpt_sent)),
            QsoStep::TxRReport => Some(fill(&self.cfg.msg_rreport, rpt_sent)),
            QsoStep::TxRr73 => Some(fill(&self.cfg.msg_rr73, None)),
            QsoStep::Tx73 => Some(fill(&self.cfg.msg_73, None)),
            // Re-send our final message only when the DX prompted it.
            QsoStep::Confirming => self.resend.then(|| self.final_msg.clone()).flatten(),
            QsoStep::Idle | QsoStep::WaitCq => None,
        }
    }

    /// The special operating activity in force, or `None` where it cannot
    /// apply.
    ///
    /// Two things put it out of reach, and both of them silently: a mode with
    /// no 77-bit layout to carry the exchange, and a station whose locator is
    /// only four characters — the whole point of the EU VHF layout is the
    /// six-character one, and there is no honest message to send without it.
    /// The setup panel says so where the activity is chosen; here the ordinary
    /// exchange simply goes out, which is the one behaviour that is never
    /// wrong on the air.
    /// Whether the operator has *chosen* the EU VHF contest, whatever the
    /// station is equipped to send.
    ///
    /// [`Self::contest`] answers the narrower question — whether this station
    /// can honestly transmit the exchange, which needs a six-character locator
    /// — and that is the right test for the transmitter. It is the wrong one
    /// for the receiver: a station with only a four-character grid is still
    /// sitting in the middle of a contest and still wants to read what is going
    /// on around it.
    pub(crate) fn contest_selected(&self) -> bool {
        self.cfg.contest == ContestMode::EuVhf
    }

    pub(crate) fn contest(&self) -> ContestMode {
        match self.cfg.contest {
            ContestMode::EuVhf
                if matches!(self.mode, Mode::Ft8 | Mode::Ft4 | Mode::Ft2)
                    && is_grid6(&self.cfg.my_grid.to_ascii_uppercase()) =>
            {
                ContestMode::EuVhf
            }
            _ => ContestMode::None,
        }
    }

    /// This slot's message in an EU VHF contest (issue #223).
    ///
    /// The first two and the last two of the six are ordinary messages — the
    /// exchange has to be preceded by messages that spell both callsigns out,
    /// because the exchange itself carries only their hashes. The two in the
    /// middle are the `i3 = 5` layout: a signal report as the two digits of an
    /// RS, a serial number, and the six-character locator.
    fn plan_eu_vhf(&self, dx_call: &str, grid4: &str, rpt_sent: Option<i16>) -> Option<String> {
        let mc = self.cfg.my_call.trim();
        let grid6 = self.cfg.my_grid.to_ascii_uppercase();
        let serial = self.serial_for_this_qso();
        // The report is theirs at us, mapped onto the RS the field can carry.
        // With nobody worked yet there is nothing measured, so the exchange
        // opens at 59 rather than inventing a signal.
        let rs = sdroxide_types::eu_vhf_rs(rpt_sent.unwrap_or(0));
        let exch = format!("{rs:02}{serial:04}");
        match self.step {
            QsoStep::CallingCq => Some(format!("CQ TEST {mc} {grid4}")),
            QsoStep::TxGrid => Some(format!("{dx_call} {mc} {grid4}")),
            QsoStep::TxReport => Some(format!("<{dx_call}> <{mc}> {exch} {grid6}")),
            QsoStep::TxRReport => Some(format!("<{dx_call}> <{mc}> R {exch} {grid6}")),
            QsoStep::TxRr73 => Some(format!("{dx_call} {mc} RR73")),
            QsoStep::Tx73 => Some(format!("{dx_call} {mc} 73")),
            QsoStep::Confirming => self.resend.then(|| self.final_msg.clone()).flatten(),
            QsoStep::Idle | QsoStep::WaitCq => None,
        }
    }

    /// The serial this contact is sending: the one already committed to it, or
    /// the station's next unused number.
    ///
    /// Committed on the first exchange that goes out and held for the rest of
    /// the contact, so a contact still in progress when the engine advances the
    /// counter — which it does the instant the *previous* one is logged — does
    /// not change its serial halfway through. That is the fault this exists to
    /// prevent: the far end logs the number in the message it heard, and two
    /// different numbers for one contact is a scoring error on both sides.
    fn serial_for_this_qso(&self) -> u32 {
        self.dx
            .as_ref()
            .and_then(|d| d.serial_sent)
            .unwrap_or(self.cfg.contest_serial)
            .clamp(1, sdroxide_types::CONTEST_SERIAL_MAX)
    }

    /// The controller calls this after each burst finishes. When the final
    /// message (73 as answerer, RR73 as CQ caller) has gone out, log the QSO and
    /// move to `Confirming`; while confirming, a queued re-send has now left.
    pub fn note_tx_sent(&mut self, now_utc: i64) {
        // A hand-queued message took this slot; the exchange is where it was.
        if self.manual.take().is_some() {
            return;
        }
        // A Fox commits the plan that just went out: contacts closed by an RR73
        // are logged, and the callers it reported to are now being worked.
        if let Some(plan) = self.fox_plan() {
            let (cfg, mode) = (self.cfg.clone(), self.mode);
            if let Some(fox) = self.fox.as_mut() {
                fox.apply(&plan, &cfg, mode, now_utc);
                while let Some(rec) = fox.take_completed() {
                    self.completed.push_back(rec);
                }
            }
            self.tx_since_progress += 1;
            return;
        }
        self.tx_since_progress += 1;
        // The serial belongs to this contact from the first exchange that goes
        // out on the air. See [`Self::serial_for_this_qso`].
        if self.contest() == ContestMode::EuVhf
            && matches!(self.step, QsoStep::TxReport | QsoStep::TxRReport)
        {
            let serial = self.serial_for_this_qso();
            if let Some(dx) = self.dx.as_mut() {
                dx.serial_sent.get_or_insert(serial);
            }
        }
        // Calling one station that never comes back: give up rather than call
        // into the void all afternoon. (Repeating a CQ is exempt — that *is*
        // the operation; the watchdog above bounds it instead.)
        let repeats = self.cfg.max_tx_repeats;
        if repeats > 0
            && self.tx_since_progress >= repeats
            && matches!(self.step, QsoStep::TxGrid | QsoStep::TxReport | QsoStep::TxRReport)
        {
            let call = self.dx.as_ref().map(|d| d.call.clone()).unwrap_or_default();
            self.transcript
                .push(TranscriptLine::note(format!("no reply from {call} after {repeats} calls")));
            self.step = QsoStep::Idle;
            return;
        }
        match self.step {
            QsoStep::Tx73 | QsoStep::TxRr73 => {
                self.final_msg = self.plan_tx();
                self.log_qso(now_utc);
            }
            QsoStep::Confirming => self.resend = false,
            _ => {}
        }
    }

    /// Take a completed QSO record for logging (fields freq_hz/band still 0).
    /// Call until it returns `None` — a Fox can finish several at once.
    pub fn take_completed(&mut self) -> Option<QsoRecord> {
        self.completed.pop_front()
    }

    pub fn status(&self, transmitting: bool) -> DigiStatus {
        DigiStatus {
            mode: self.mode,
            step: self.step,
            dx_call: self.dx.as_ref().map(|d| d.call.clone()),
            dx_grid: self.dx.as_ref().and_then(|d| d.grid.clone()),
            tx_next: self.wants_tx(),
            tx_pending_msg: self.plan_tx(),
            audio_hz: self.audio_hz,
            tx_even: self.tx_even,
            transmitting,
            tx_watchdog: self.watchdog,
            transcript: self.transcript.clone(),
            config: self.cfg.clone(),
            // FT8/FT4 don't use the continuous keyboard-text fields.
            text_rx: String::new(),
            tx_sent: 0,
            fsq_heard: Vec::new(),
            fsq_messages: Vec::new(),
            rade: None,
            packet: None,
            navtex: None,
            aprs: None,
            js8: None,
            fox_queue: self.fox.as_ref().map(Fox::status).unwrap_or_default(),
            call_queue: self.queue.iter().cloned().collect(),
            clock_offset_s: None,
            cw: None,
            wspr: None,
            qso: self.dx.as_ref().map(|d| sdroxide_types::QsoLive {
                started_utc: d.started_utc,
                rpt_sent: d.rpt_sent,
                rpt_rcvd: d.rpt_rcvd,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> DigiConfig {
        DigiConfig { my_call: "AB1CD".into(), my_grid: "FN42".into(), ..Default::default() }
    }

    fn decode(msg: &str) -> Decode {
        // The angle brackets a hashed callsign is written with are the
        // *rendering* of the hash, not part of the call — the real parser
        // strips them (`modem::call_of`), so this stand-in has to as well or a
        // contest exchange never looks addressed to anybody.
        let call = |t: &str| {
            t.strip_prefix('<').and_then(|x| x.strip_suffix('>')).unwrap_or(t).to_string()
        };
        Decode {
            slot_utc: 0,
            snr_db: -10,
            dt: 0.1,
            audio_hz: 1500.0,
            message: msg.to_string(),
            to: msg.split_whitespace().next().filter(|t| *t != "CQ").map(call),
            from: msg.split_whitespace().nth(1).map(call),
            grid: None,
            is_cq: msg.starts_with("CQ"),
            cq_to: msg.split_whitespace().nth(1).filter(|t| *t == "DX").map(str::to_string),
            free_text: false,
            rr73_to: None,
        }
    }

    /// A station set up for a European VHF contest: the six-character locator
    /// the layout requires, and a serial to start from.
    fn eu_cfg() -> DigiConfig {
        DigiConfig {
            my_call: "G4ABC/P".into(),
            my_grid: "IO91NP".into(),
            contest: ContestMode::EuVhf,
            contest_serial: 3,
            ..Default::default()
        }
    }

    /// The exchange from WSJT-X's own `messages.txt`, run through the
    /// sequencer from both sides (issue #223).
    #[test]
    fn a_eu_vhf_contest_runs_the_reference_exchange() {
        let mut q = QsoMachine::new(Mode::Ft8, eu_cfg());
        q.call_cq();
        assert_eq!(q.plan_tx().as_deref(), Some("CQ TEST G4ABC/P IO91"));

        // PA9XYZ answers with a grid; we come back with the exchange. The RS
        // is their signal at us: -10 dB is (−10+36)/6 = 4, so 54.
        assert!(q.on_rx(&[decode("G4ABC/P PA9XYZ JO22")], 115));
        assert_eq!(q.step(), QsoStep::TxReport);
        assert_eq!(q.plan_tx().as_deref(), Some("<PA9XYZ> <G4ABC/P> 540003 IO91NP"));

        // Their exchange comes back rogered → the contact is complete and we
        // send RR73, exactly as a rogered report would have us do.
        q.note_tx_sent(130);
        assert!(q.on_rx(&[decode("<G4ABC/P> <PA9XYZ> R 570007 JO22DB")], 145));
        assert_eq!(q.step(), QsoStep::TxRr73);
        assert_eq!(q.plan_tx().as_deref(), Some("PA9XYZ G4ABC/P RR73"));

        q.note_tx_sent(160);
        let rec = q.take_completed().expect("logged");
        assert_eq!(rec.call, "PA9XYZ");
        // The reports logged are the two RS values that went over the air, not
        // the signal-to-noise ratios behind them.
        assert_eq!(rec.rst_sent, Some(54));
        assert_eq!(rec.rst_rcvd, Some(57));
        assert_eq!(rec.stx, Some(3));
        assert_eq!(rec.srx, Some(7));
        assert_eq!(rec.stx_string, "540003 IO91NP");
        assert_eq!(rec.srx_string, "570007 JO22DB");
        // Their six-character locator is the one the log keeps.
        assert_eq!(rec.grid.as_deref(), Some("JO22DB"));
    }

    /// Answering somebody else's contest CQ: their exchange arrives bare, ours
    /// goes back rogered.
    #[test]
    fn answering_a_contest_cq_rogers_the_exchange() {
        let mut q = QsoMachine::new(Mode::Ft8, eu_cfg());
        q.start_qso("PA9XYZ".into(), Some("JO22".into()), -10, false, 100);
        assert_eq!(q.plan_tx().as_deref(), Some("PA9XYZ G4ABC/P IO91"));
        assert!(q.on_rx(&[decode("<G4ABC/P> <PA9XYZ> 590003 JO22DB")], 115));
        assert_eq!(q.step(), QsoStep::TxRReport);
        assert_eq!(q.plan_tx().as_deref(), Some("<PA9XYZ> <G4ABC/P> R 540003 IO91NP"));
    }

    /// The serial is fixed when the first exchange goes on the air and does
    /// not move under the contact afterwards — the engine advances the
    /// station's counter the instant the *previous* contact is logged, and two
    /// different numbers for one contact is a scoring error at both ends.
    #[test]
    fn a_contact_keeps_the_serial_it_first_sent() {
        let mut q = QsoMachine::new(Mode::Ft8, eu_cfg());
        q.start_qso("PA9XYZ".into(), Some("JO22".into()), -10, false, 100);
        assert!(q.on_rx(&[decode("<G4ABC/P> <PA9XYZ> 590003 JO22DB")], 115));
        assert_eq!(q.plan_tx().as_deref(), Some("<PA9XYZ> <G4ABC/P> R 540003 IO91NP"));
        q.note_tx_sent(130);
        // The station's counter moves on under us.
        q.set_config(DigiConfig { contest_serial: 9, ..eu_cfg() });
        assert_eq!(q.plan_tx().as_deref(), Some("<PA9XYZ> <G4ABC/P> R 540003 IO91NP"));
    }

    /// Without a six-character locator there is no honest contest message to
    /// send, so the ordinary exchange goes out instead — the one behaviour
    /// that is never wrong on the air. The setup panel is where the operator
    /// is told.
    #[test]
    fn a_four_character_grid_falls_back_to_the_ordinary_exchange() {
        let cfg = DigiConfig { my_grid: "IO91".into(), ..eu_cfg() };
        let mut q = QsoMachine::new(Mode::Ft8, cfg);
        q.call_cq();
        assert_eq!(q.plan_tx().as_deref(), Some("CQ G4ABC/P IO91"));
    }

    /// And neither is there in a mode with no 77-bit layout to carry it.
    #[test]
    fn a_mode_without_the_layout_falls_back_too() {
        let mut q = QsoMachine::new(Mode::Psk, eu_cfg());
        q.call_cq();
        assert_eq!(q.plan_tx().as_deref(), Some("CQ G4ABC/P IO91"));
    }

    #[test]
    fn grid_truncated_to_four_for_ft8() {
        // A 6-character locator is cut to the 4-char Maidenhead grid in messages.
        let cfg = DigiConfig { my_call: "AB1CD".into(), my_grid: "JN78ve".into(), ..cfg() };
        let mut q = QsoMachine::new(Mode::Ft8, cfg);
        q.start_qso("W9XYZ".into(), Some("EM48".into()), -10, false, 100);
        assert_eq!(q.plan_tx().as_deref(), Some("W9XYZ AB1CD JN78"));
    }

    #[test]
    fn answerer_full_sequence() {
        // We (AB1CD) answer W9XYZ's CQ and run the QSO to completion.
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.start_qso("W9XYZ".into(), Some("EM48".into()), -10, false, 100);
        assert_eq!(q.step(), QsoStep::TxGrid);
        assert_eq!(q.plan_tx().as_deref(), Some("W9XYZ AB1CD FN42"));

        // They send us a report → we send R+report.
        assert!(q.on_rx(&[decode("AB1CD W9XYZ -13")], 115));
        assert_eq!(q.step(), QsoStep::TxRReport);
        assert_eq!(q.plan_tx().as_deref(), Some("W9XYZ AB1CD R-10"));

        // They roger → we send 73.
        assert!(q.on_rx(&[decode("AB1CD W9XYZ RR73")], 130));
        assert_eq!(q.step(), QsoStep::Tx73);
        assert_eq!(q.plan_tx().as_deref(), Some("W9XYZ AB1CD 73"));

        // Our 73 goes out → logged, and we hold in Confirming (ready to re-send).
        q.note_tx_sent(145);
        assert_eq!(q.step(), QsoStep::Confirming);
        assert!(!q.wants_tx());
        let rec = q.take_completed().expect("logged");
        assert_eq!(rec.call, "W9XYZ");
        assert_eq!(rec.rst_sent, Some(-10));
        assert_eq!(rec.rst_rcvd, Some(-13));
        assert_eq!(rec.my_call, "AB1CD");
    }

    #[test]
    fn cq_caller_sequence() {
        // We call CQ; W9XYZ answers with a grid; we run the exchange.
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.call_cq();
        assert_eq!(q.plan_tx().as_deref(), Some("CQ AB1CD FN42"));

        // W9XYZ answers our CQ → we adopt them and send a report (their SNR).
        assert!(q.on_rx(&[decode("AB1CD W9XYZ EM48")], 100));
        assert_eq!(q.step(), QsoStep::TxReport);
        assert_eq!(q.plan_tx().as_deref(), Some("W9XYZ AB1CD -10"));

        // They send R+report → we send RR73.
        assert!(q.on_rx(&[decode("AB1CD W9XYZ R-12")], 115));
        assert_eq!(q.step(), QsoStep::TxRr73);
        assert_eq!(q.plan_tx().as_deref(), Some("W9XYZ AB1CD RR73"));

        // Our RR73 goes out → logged + Confirming; their 73 just confirms it.
        q.note_tx_sent(130);
        assert_eq!(q.step(), QsoStep::Confirming);
        let rec = q.take_completed().expect("logged");
        assert_eq!(rec.rst_sent, Some(-10));
        assert_eq!(rec.rst_rcvd, Some(-12));
        assert!(q.on_rx(&[decode("AB1CD W9XYZ 73")], 145));
        assert!(!q.wants_tx(), "a bare 73 needs no re-send");
    }

    /// A decode addressed to us from `call` at `snr`.
    fn answer(call: &str, snr: i16) -> Decode {
        let mut d = decode(&format!("AB1CD {call} EM48"));
        d.snr_db = snr;
        d
    }

    #[test]
    fn an_answer_with_a_report_instead_of_a_grid_is_worked() {
        // Plenty of stations answer a CQ with a report rather than their grid.
        // That must not leave us calling CQ over the top of them.
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.call_cq();
        assert!(q.on_rx(&[decode("AB1CD W9XYZ -19")], 100));
        assert_eq!(q.dx_call(), Some("W9XYZ"));
        assert_eq!(q.step(), QsoStep::TxRReport, "still calling CQ at them");
        assert_eq!(q.plan_tx().as_deref(), Some("W9XYZ AB1CD R-10"));

        // From there it is the ordinary answering side: their roger ends it.
        assert!(q.on_rx(&[decode("AB1CD W9XYZ RR73")], 115));
        assert_eq!(q.step(), QsoStep::Tx73);
        q.note_tx_sent(130);
        let rec = q.take_completed().expect("logged");
        assert_eq!(rec.rst_sent, Some(-10));
        assert_eq!(rec.rst_rcvd, Some(-19), "the report they opened with is the one logged");
    }

    #[test]
    fn an_answer_we_cannot_read_still_starts_the_exchange() {
        // Free text, a bare call, a message the packer mangled — whatever it
        // says, somebody addressed us, so send them a report.
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.call_cq();
        assert!(q.on_rx(&[decode("AB1CD W9XYZ HI")], 100));
        assert_eq!(q.step(), QsoStep::TxReport);
        assert_eq!(q.plan_tx().as_deref(), Some("W9XYZ AB1CD -10"));
    }

    #[test]
    fn a_signing_off_station_is_not_a_caller() {
        // A late 73 from the contact we just finished is not somebody asking to
        // be worked: adopting it would wedge the CQ run against everyone else.
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.call_cq();
        assert!(!q.on_rx(&[decode("AB1CD W9XYZ 73")], 100));
        assert_eq!(q.dx_call(), None);
        assert_eq!(q.step(), QsoStep::CallingCq);

        // The real caller in the next slot is taken as usual.
        assert!(q.on_rx(&[decode("AB1CD K1ABC EM48")], 115));
        assert_eq!(q.dx_call(), Some("K1ABC"));
    }

    #[test]
    fn replying_to_a_station_that_called_us_picks_up_where_they_are() {
        // Stood down (STOP QSO), with W9XYZ still calling us. Answering them by
        // hand must send what they are waiting for, not start from our grid.
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.stop();
        assert!(!q.on_rx(&[decode("AB1CD W9XYZ -19")], 100));
        q.start_qso("W9XYZ".into(), None, -10, false, 115);
        assert_eq!(q.step(), QsoStep::TxRReport);
        assert_eq!(q.plan_tx().as_deref(), Some("W9XYZ AB1CD R-10"));
        q.on_rx(&[decode("AB1CD W9XYZ RR73")], 130);
        q.note_tx_sent(145);
        assert_eq!(q.take_completed().and_then(|r| r.rst_rcvd), Some(-19));

        // Someone calling us with their grid is owed a report, likewise.
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.on_rx(&[decode("AB1CD K1ABC EM48")], 100);
        q.start_qso("K1ABC".into(), None, -10, false, 115);
        assert_eq!(q.plan_tx().as_deref(), Some("K1ABC AB1CD -10"));

        // A station we have not heard from is called from the top, as before.
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.start_qso("G4XYZ".into(), None, -10, false, 100);
        assert_eq!(q.plan_tx().as_deref(), Some("G4XYZ AB1CD FN42"));
    }

    #[test]
    fn a_stale_call_does_not_speak_for_a_new_contact() {
        // Answering them ten minutes on is a fresh contact: what they sent back
        // then says nothing about it.
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.on_rx(&[decode("AB1CD W9XYZ -19")], 100);
        q.start_qso("W9XYZ".into(), None, -10, false, 100 + RECENT_CALL_S + 15);
        assert_eq!(q.plan_tx().as_deref(), Some("W9XYZ AB1CD FN42"));
    }

    #[test]
    fn the_strongest_of_several_answers_is_worked_first() {
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.call_cq();
        q.on_rx(&[answer("W9XYZ", -18), answer("K1ABC", -4), answer("G4XYZ", -21)], 100);
        assert_eq!(q.dx_call(), Some("K1ABC"));
        // The ones we passed over are recorded, since they're still calling.
        let note = q.status(false).transcript.iter().find(|l| l.overheard).cloned();
        let note = note.expect("a note listing the other callers");
        assert!(note.text.contains("W9XYZ") && note.text.contains("G4XYZ"), "{}", note.text);
        assert!(!note.text.contains("K1ABC"), "the station we took is not an 'also'");
    }

    #[test]
    fn a_station_worked_this_session_waits_its_turn() {
        // Run a full QSO with W9XYZ, then have them answer the next CQ
        // alongside a weaker station we haven't worked.
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.call_cq();
        q.on_rx(&[answer("W9XYZ", -2)], 100);
        q.on_rx(&[decode("AB1CD W9XYZ R-12")], 115);
        q.note_tx_sent(130); // RR73 out → logged
        assert!(q.take_completed().is_some());

        q.call_cq();
        q.on_rx(&[answer("W9XYZ", -2), answer("K1ABC", -20)], 145);
        assert_eq!(q.dx_call(), Some("K1ABC"), "a dupe loses to a station not yet worked");
    }

    #[test]
    fn a_new_entity_wins_between_comparable_signals() {
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        // Work a US station first, so "United States" is no longer new.
        q.call_cq();
        q.on_rx(&[answer("K1ABC", -2)], 100);
        q.on_rx(&[decode("AB1CD K1ABC R-12")], 115);
        q.note_tx_sent(130);
        assert!(q.take_completed().is_some());

        // Two answers within the same signal band: the new entity is worth more.
        q.call_cq();
        q.on_rx(&[answer("W9XYZ", -3), answer("DL1ABC", -6)], 145);
        assert_eq!(q.dx_call(), Some("DL1ABC"));

        // A much stronger signal still wins — a marginal new one that can't
        // complete is worth less than a contact that will.
        q.abort();
        q.call_cq();
        q.on_rx(&[answer("W9XYZ", 3), answer("DL1ABC", -22)], 160);
        assert_eq!(q.dx_call(), Some("W9XYZ"));
    }

    #[test]
    fn ignores_other_stations() {
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.start_qso("W9XYZ".into(), None, -10, false, 100);
        // Traffic between two other stations must not touch our exchange.
        assert!(!q.on_rx(&[decode("K1ABC G4XYZ -05")], 115));
        assert_eq!(q.step(), QsoStep::TxGrid);
        // Nor must a report someone else sent *our* DX (that's their SNR, not
        // ours) — only what the DX sends us advances the QSO.
        assert!(!q.on_rx(&[decode("W9XYZ K1ABC -05")], 130));
        assert_eq!(q.step(), QsoStep::TxGrid);
    }

    #[test]
    fn reply_to_non_cq_waits_for_cq() {
        // Picking a station that isn't calling CQ holds silently until they do.
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.start_qso("W9XYZ".into(), None, -10, true, 100);
        assert_eq!(q.step(), QsoStep::WaitCq);
        assert!(!q.wants_tx());
        assert_eq!(q.plan_tx(), None);

        // Them working someone else keeps us waiting (and says who with).
        q.on_rx(&[decode("K1ABC W9XYZ RR73")], 115);
        assert_eq!(q.step(), QsoStep::WaitCq);
        assert!(!q.wants_tx());

        // They call CQ → we start replying.
        assert!(q.on_rx(&[decode("CQ W9XYZ EM48")], 130));
        assert_eq!(q.step(), QsoStep::TxGrid);
        assert!(q.wants_tx());
        assert_eq!(q.plan_tx().as_deref(), Some("W9XYZ AB1CD FN42"));
    }

    #[test]
    fn dx_taking_another_caller_holds_us_until_their_next_cq() {
        // We answered W9XYZ's CQ, but they came back to K1ABC instead.
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.start_qso("W9XYZ".into(), Some("EM48".into()), -10, false, 100);
        assert_eq!(q.step(), QsoStep::TxGrid);

        assert!(q.on_rx(&[decode("K1ABC W9XYZ -13")], 115));
        assert_eq!(q.step(), QsoStep::WaitCq, "we must stop calling into their QSO");
        assert!(!q.wants_tx());
        assert_eq!(q.plan_tx(), None);
        let note = q.status(false).transcript.pop().expect("a note about who they're working");
        assert!(note.overheard);
        assert_eq!(note.text, "W9XYZ is working K1ABC");

        // Their exchange continues: one note per partner, not one per slot.
        assert!(!q.on_rx(&[decode("K1ABC W9XYZ RR73")], 130));
        assert_eq!(q.status(false).transcript.iter().filter(|l| l.overheard).count(), 1);
        // A new partner is worth a new line.
        assert!(q.on_rx(&[decode("G4XYZ W9XYZ -07")], 145));
        assert_eq!(
            q.status(false).transcript.last().map(|l| l.text.clone()),
            Some("W9XYZ is working G4XYZ".into())
        );

        // They're free again → we resume calling them.
        assert!(q.on_rx(&[decode("CQ W9XYZ EM48")], 160));
        assert_eq!(q.step(), QsoStep::TxGrid);
        assert!(q.wants_tx());
        assert_eq!(q.plan_tx().as_deref(), Some("W9XYZ AB1CD FN42"));
    }

    #[test]
    fn an_answer_to_us_beats_other_traffic_in_the_same_slot() {
        // Both their reply to us and a stray decode to someone else land in one
        // slot: the reply wins, so we keep the QSO instead of standing down.
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.start_qso("W9XYZ".into(), Some("EM48".into()), -10, false, 100);
        assert!(q.on_rx(&[decode("K1ABC W9XYZ -13"), decode("AB1CD W9XYZ -09")], 115));
        assert_eq!(q.step(), QsoStep::TxRReport);
        assert!(q.status(false).transcript.iter().all(|l| !l.overheard));
    }

    #[test]
    fn a_final_message_still_goes_out_when_they_move_on() {
        // They rogered us and moved to K1ABC before our 73 left: the contact is
        // complete, so we send it and log rather than standing down.
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.start_qso("W9XYZ".into(), Some("EM48".into()), -10, false, 100);
        q.on_rx(&[decode("AB1CD W9XYZ -13")], 115);
        q.on_rx(&[decode("AB1CD W9XYZ RR73")], 130);
        assert_eq!(q.step(), QsoStep::Tx73);
        assert!(!q.on_rx(&[decode("K1ABC W9XYZ -05")], 145));
        assert_eq!(q.step(), QsoStep::Tx73);
        assert!(q.wants_tx());
    }

    #[test]
    fn the_operator_can_pick_which_message_goes_next() {
        // Mid-exchange, jumping to RR73 skips the rest of the sequence and the
        // machine carries on from there — WSJT-X's Tx4 button.
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.start_qso("W9XYZ".into(), Some("EM48".into()), -10, false, 100);
        assert!(q.set_step(QsoStep::TxRr73));
        assert_eq!(q.plan_tx().as_deref(), Some("W9XYZ AB1CD RR73"));
        // Sending it still completes and logs the contact.
        q.note_tx_sent(115);
        assert_eq!(q.step(), QsoStep::Confirming);
        assert!(q.take_completed().is_some());

        // A step that addresses a station is refused when there is none.
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        assert!(!q.set_step(QsoStep::TxReport));
        assert_eq!(q.step(), QsoStep::Idle);
        // Calling CQ needs no DX.
        assert!(q.set_step(QsoStep::CallingCq));
        assert_eq!(q.plan_tx().as_deref(), Some("CQ AB1CD FN42"));
    }

    #[test]
    fn a_queued_message_takes_one_slot_and_leaves_the_exchange_alone() {
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.start_qso("W9XYZ".into(), Some("EM48".into()), -10, false, 100);
        q.queue_text("w9xyz ab1cd tnx".into());
        assert!(q.wants_tx());
        assert_eq!(q.plan_tx().as_deref(), Some("W9XYZ AB1CD TNX"), "sent verbatim, uppercased");

        // After it goes out the sequencer picks up exactly where it was.
        q.note_tx_sent(115);
        assert_eq!(q.step(), QsoStep::TxGrid);
        assert_eq!(q.plan_tx().as_deref(), Some("W9XYZ AB1CD FN42"));

        // Queued at the last step, it must not log the QSO in place of the 73.
        q.on_rx(&[decode("AB1CD W9XYZ -13")], 130);
        q.on_rx(&[decode("AB1CD W9XYZ RR73")], 145);
        assert_eq!(q.step(), QsoStep::Tx73);
        q.queue_text("TNX 73 GL".into());
        q.note_tx_sent(160);
        assert_eq!(q.step(), QsoStep::Tx73, "the 73 still owes us a slot");
        assert!(q.take_completed().is_none(), "nothing to log until the 73 is sent");
        q.note_tx_sent(175);
        assert_eq!(q.step(), QsoStep::Confirming);
        assert!(q.take_completed().is_some());
    }

    #[test]
    fn a_queued_message_still_needs_a_callsign() {
        let mut q = QsoMachine::new(Mode::Ft8, DigiConfig::default());
        q.queue_text("CQ TEST".into());
        assert!(!q.wants_tx());
        assert_eq!(q.plan_tx(), None);
    }

    #[test]
    fn an_unconfigured_station_never_transmits() {
        let mut q = QsoMachine::new(Mode::Ft8, DigiConfig::default());
        q.call_cq();
        assert_eq!(q.plan_tx(), None, "no callsign, no transmission");
    }

    #[test]
    fn the_watchdog_stops_a_station_calling_into_an_empty_band() {
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.call_cq();
        assert!(!q.tick(100), "the clock starts at the first tick");
        assert!(q.wants_tx());
        // Well inside the window nothing happens.
        assert!(!q.tick(100 + 5 * 60));
        // Past it the sequencer stands down and says why.
        assert!(q.tick(100 + 6 * 60));
        assert!(!q.wants_tx());
        assert!(q.tx_watchdog());
        assert_eq!(q.step(), QsoStep::Idle);
        let note = q.status(false).transcript.pop().expect("a note");
        assert!(note.overheard && note.text.contains("watchdog"), "{}", note.text);

        // Calling CQ again clears it and restarts the clock.
        q.call_cq();
        assert!(!q.tx_watchdog());
        assert!(!q.tick(100 + 7 * 60), "the window restarts from the operator's action");
        assert!(q.wants_tx());
    }

    #[test]
    fn a_reply_keeps_the_watchdog_at_bay() {
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.start_qso("W9XYZ".into(), Some("EM48".into()), -10, false, 100);
        q.tick(100);
        // They answer five minutes in: the window starts again from there.
        q.on_rx(&[decode("AB1CD W9XYZ -13")], 100 + 5 * 60);
        assert!(!q.tick(100 + 10 * 60), "progress reset the watchdog");
        assert!(q.wants_tx());
        assert!(q.tick(100 + 11 * 60 + 1), "…but it still fires when they stop replying");
        assert!(q.tx_watchdog());
    }

    #[test]
    fn calling_a_station_that_never_answers_gives_up() {
        let cfg = DigiConfig { max_tx_repeats: 3, ..cfg() };
        let mut q = QsoMachine::new(Mode::Ft8, cfg);
        q.start_qso("W9XYZ".into(), Some("EM48".into()), -10, false, 100);
        for i in 0..2 {
            q.note_tx_sent(100 + i * 15);
            assert_eq!(q.step(), QsoStep::TxGrid, "still calling after {} sends", i + 1);
        }
        q.note_tx_sent(130);
        assert_eq!(q.step(), QsoStep::Idle, "gave up after the third unanswered call");
        assert!(!q.wants_tx());
        let note = q.status(false).transcript.pop().expect("a note");
        assert!(note.text.contains("W9XYZ") && note.text.contains("3 calls"), "{}", note.text);
    }

    #[test]
    fn an_answer_to_our_cq_starts_the_count_again() {
        // A long CQ run must not be charged to the station that finally comes
        // back: giving up on them before the first report goes out is exactly
        // backwards. Only calls to *them* count.
        let cfg = DigiConfig { max_tx_repeats: 4, ..cfg() };
        let mut q = QsoMachine::new(Mode::Ft8, cfg);
        q.call_cq();
        for _ in 0..4 {
            q.note_tx_sent(100);
        }
        assert!(q.on_rx(&[decode("AB1CD W9XYZ EM48")], 115));
        assert_eq!(q.step(), QsoStep::TxReport);

        // Three reports go out with them still silent, and we are still calling.
        for i in 0..3 {
            q.note_tx_sent(130 + i * 15);
            assert_eq!(q.step(), QsoStep::TxReport, "gave up after {} reports", i + 1);
        }
        // The fourth is the one the setting bounds.
        q.note_tx_sent(175);
        assert_eq!(q.step(), QsoStep::Idle);
    }

    #[test]
    fn a_station_that_comes_back_on_the_air_gets_a_fresh_count() {
        // Same rule for a station we held off calling because they were busy:
        // the calls we made before they answered someone else are spent.
        let cfg = DigiConfig { max_tx_repeats: 2, ..cfg() };
        let mut q = QsoMachine::new(Mode::Ft8, cfg);
        q.start_qso("W9XYZ".into(), Some("EM48".into()), -10, false, 100);
        q.note_tx_sent(100);
        q.on_rx(&[decode("K1ABC W9XYZ -13")], 115);
        assert_eq!(q.step(), QsoStep::WaitCq);
        assert!(q.on_rx(&[decode("CQ W9XYZ EM48")], 130));
        assert_eq!(q.step(), QsoStep::TxGrid);
        q.note_tx_sent(145);
        assert_eq!(q.step(), QsoStep::TxGrid, "the earlier call was not this one's");
    }

    #[test]
    fn a_bare_73_finishes_the_exchange() {
        // Plenty of stations close with 73 where the sequence says RR73. It
        // means the same thing — they have our R+report and are signing off —
        // so it must complete the contact, not leave us repeating at them.
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.start_qso("W9XYZ".into(), Some("EM48".into()), -10, false, 100);
        q.on_rx(&[decode("AB1CD W9XYZ -13")], 115);
        assert_eq!(q.step(), QsoStep::TxRReport);
        assert!(q.on_rx(&[decode("AB1CD W9XYZ 73")], 130));
        assert_eq!(q.step(), QsoStep::Tx73);

        q.note_tx_sent(145);
        assert_eq!(q.step(), QsoStep::Confirming);
        let rec = q.take_completed().expect("logged");
        assert_eq!(rec.call, "W9XYZ");
        assert_eq!(rec.rst_rcvd, Some(-13));
    }

    /// Drive a machine to `step` with W9XYZ as the DX, by the route the
    /// sequencer actually takes to get there.
    fn at(step: QsoStep) -> QsoMachine {
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        match step {
            QsoStep::TxGrid => q.start_qso("W9XYZ".into(), Some("EM48".into()), -10, false, 100),
            QsoStep::TxReport => {
                q.call_cq();
                q.on_rx(&[decode("AB1CD W9XYZ EM48")], 100);
            }
            QsoStep::TxRReport => {
                q.start_qso("W9XYZ".into(), Some("EM48".into()), -10, false, 100);
                q.on_rx(&[decode("AB1CD W9XYZ -13")], 115);
            }
            QsoStep::TxRr73 => {
                q.call_cq();
                q.on_rx(&[decode("AB1CD W9XYZ EM48")], 100);
                q.on_rx(&[decode("AB1CD W9XYZ R-13")], 115);
            }
            QsoStep::Tx73 => {
                q.start_qso("W9XYZ".into(), Some("EM48".into()), -10, false, 100);
                q.on_rx(&[decode("AB1CD W9XYZ -13")], 115);
                q.on_rx(&[decode("AB1CD W9XYZ RR73")], 130);
            }
            other => panic!("no route to {other:?}"),
        }
        assert_eq!(q.step(), step, "the route did not reach {step:?}");
        q
    }

    #[test]
    fn every_message_they_send_is_answered_from_wherever_we_are() {
        // The sequencer answers what the DX actually sent, not what it was
        // expecting. Matching on both sides used to wedge these exchanges:
        // each station waiting for a message the other had moved past, both
        // repeating themselves until the unanswered-call count gave up.
        //
        // `want` is what we should send next; the step we were in is the DX's
        // problem, not ours.
        let cases = [
            // (they send, from this step, we answer with)
            ("AB1CD W9XYZ EM48", QsoStep::TxGrid, QsoStep::TxReport),
            ("AB1CD W9XYZ EM48", QsoStep::TxRReport, QsoStep::TxReport),
            ("AB1CD W9XYZ -12", QsoStep::TxReport, QsoStep::TxRReport),
            ("AB1CD W9XYZ -12", QsoStep::TxRr73, QsoStep::TxRReport),
            ("AB1CD W9XYZ R-12", QsoStep::TxGrid, QsoStep::TxRr73),
            ("AB1CD W9XYZ R-12", QsoStep::TxRReport, QsoStep::TxRr73),
            ("AB1CD W9XYZ RR73", QsoStep::TxGrid, QsoStep::Tx73),
            ("AB1CD W9XYZ RR73", QsoStep::TxReport, QsoStep::Tx73),
            ("AB1CD W9XYZ RRR", QsoStep::TxReport, QsoStep::Tx73),
            // Already sending the right answer: their repeat changes nothing.
            ("AB1CD W9XYZ EM48", QsoStep::TxReport, QsoStep::TxReport),
            ("AB1CD W9XYZ -12", QsoStep::TxRReport, QsoStep::TxRReport),
            ("AB1CD W9XYZ R-12", QsoStep::TxRr73, QsoStep::TxRr73),
            ("AB1CD W9XYZ RR73", QsoStep::Tx73, QsoStep::Tx73),
        ];
        for (msg, from, want) in cases {
            let mut q = at(from);
            q.on_rx(&[decode(msg)], 200);
            assert_eq!(q.step(), want, "{from:?} + \"{msg}\" should leave us at {want:?}");
        }
    }

    #[test]
    fn free_text_does_not_move_the_exchange() {
        // Free text carries no place in the sequence, so it must not move us —
        // least of all backwards. "TNX QSO 73 GL" while we are sending 73 is
        // not a request to start again from a signal report.
        for step in [QsoStep::TxGrid, QsoStep::TxReport, QsoStep::TxRReport, QsoStep::Tx73] {
            let mut q = at(step);
            q.on_rx(&[decode("AB1CD W9XYZ TNX QSO")], 200);
            assert_eq!(q.step(), step, "free text moved us out of {step:?}");
        }

        // Answering our CQ is the exception: whatever it says, somebody called
        // us, so they get a report.
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.call_cq();
        q.on_rx(&[decode("AB1CD W9XYZ HI")], 100);
        assert_eq!(q.step(), QsoStep::TxReport);
    }

    #[test]
    fn a_stood_down_sequencer_is_not_restarted_by_the_dx() {
        // The watchdog and the unanswered-call count both stop the sequencer
        // with the contact still on screen. A stand-down the far end can undo
        // is no stand-down at all — only the operator resumes it.
        let cfg = DigiConfig { max_tx_repeats: 1, ..cfg() };
        let mut q = QsoMachine::new(Mode::Ft8, cfg);
        q.start_qso("W9XYZ".into(), Some("EM48".into()), -10, false, 100);
        q.note_tx_sent(115);
        assert_eq!(q.step(), QsoStep::Idle, "gave up");

        q.on_rx(&[decode("AB1CD W9XYZ -13")], 130);
        assert_eq!(q.step(), QsoStep::Idle, "the DX turning up must not key the radio again");
        assert!(!q.wants_tx());
        // …but answering them by hand picks up from where they are.
        q.start_qso("W9XYZ".into(), None, -10, false, 145);
        assert_eq!(q.plan_tx().as_deref(), Some("W9XYZ AB1CD R-10"));
    }

    #[test]
    fn the_final_message_is_logged_once_however_often_it_is_sent() {
        // The operator re-sends RR73 when they think the DX missed it. Every
        // send lands back in `log_qso`, which used to write a second identical
        // record — and from there a duplicate in every ADIF export and upload.
        let mut q = at(QsoStep::TxRr73);
        q.note_tx_sent(130);
        assert_eq!(q.step(), QsoStep::Confirming);
        assert!(q.take_completed().is_some(), "the contact is logged");

        assert!(q.set_step(QsoStep::TxRr73), "operator sends it again");
        q.note_tx_sent(145);
        assert!(q.take_completed().is_none(), "logged twice");
        // And the transcript says "complete" once, not once per send.
        assert_eq!(q.status(false).transcript.iter().filter(|l| l.done).count(), 1);

        // A genuinely new contact logs as usual.
        q.call_cq();
        q.on_rx(&[decode("AB1CD K1ABC EM48")], 160);
        q.on_rx(&[decode("AB1CD K1ABC R-12")], 175);
        q.note_tx_sent(190);
        assert_eq!(q.take_completed().map(|r| r.call), Some("K1ABC".into()));
    }

    #[test]
    fn a_cq_caller_stops_reporting_at_a_station_that_took_someone_else() {
        // They answered our CQ, then went off with K1ABC. Carrying on sends our
        // report straight into their exchange.
        let mut q = at(QsoStep::TxReport);
        assert!(q.on_rx(&[decode("K1ABC W9XYZ R-05")], 130));
        assert_eq!(q.step(), QsoStep::WaitCq);
        assert!(!q.wants_tx());
        let note = q.status(false).transcript.pop().expect("a note");
        assert_eq!(note.text, "W9XYZ is working K1ABC");

        // They come back to us → we carry on where we left off.
        assert!(q.on_rx(&[decode("AB1CD W9XYZ EM48")], 145));
        assert_eq!(q.step(), QsoStep::TxReport);
        assert!(q.wants_tx());
    }

    #[test]
    fn a_requested_resend_outlives_the_confirm_window() {
        // They repeated their final message in the last seconds of the window.
        // Retiring the contact on top of that drops the answer instead of
        // sending it.
        let mut q = at(QsoStep::Tx73);
        q.note_tx_sent(145);
        assert_eq!(q.step(), QsoStep::Confirming);
        q.on_rx(&[decode("AB1CD W9XYZ RR73")], 145 + CONFIRM_S - 1);
        assert!(q.wants_tx(), "a re-send is queued");

        assert!(!q.tick(145 + CONFIRM_S), "the contact is held for the re-send");
        assert_eq!(q.plan_tx().as_deref(), Some("W9XYZ AB1CD 73"));
        q.note_tx_sent(145 + CONFIRM_S);
        // Once it has gone out the contact retires as usual.
        assert!(q.tick(145 + CONFIRM_S + 1));
        assert_eq!(q.step(), QsoStep::Idle);
    }

    #[test]
    fn a_completed_contact_says_so_in_the_transcript() {
        // The exchange stays on screen after it ends, so the transcript has to
        // carry the fact that it *has* ended.
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.call_cq();
        q.on_rx(&[decode("AB1CD W9XYZ EM48")], 100);
        q.on_rx(&[decode("AB1CD W9XYZ R-12")], 115);
        assert!(q.status(false).transcript.iter().all(|l| !l.done), "not finished yet");

        q.note_tx_sent(130); // our RR73 goes out → logged
        let line = q.status(false).transcript.into_iter().find(|l| l.done).expect("a done line");
        assert!(line.text.contains("W9XYZ"), "{}", line.text);
        assert!(!line.overheard && !line.tx, "the completion line is neither ours nor overheard");
    }

    #[test]
    fn repeating_a_cq_is_not_an_unanswered_call() {
        // A CQ run repeats by design; only the watchdog bounds it.
        let cfg = DigiConfig { max_tx_repeats: 3, ..cfg() };
        let mut q = QsoMachine::new(Mode::Ft8, cfg);
        q.call_cq();
        for _ in 0..10 {
            q.note_tx_sent(100);
        }
        assert_eq!(q.step(), QsoStep::CallingCq);
        assert!(q.wants_tx());
    }

    #[test]
    fn waitcq_times_out() {
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.start_qso("W9XYZ".into(), None, -10, true, 100);
        assert!(!q.tick(200)); // within the window
        assert!(q.tick(100 + WAIT_CQ_S)); // deadline reached → give up
        assert_eq!(q.step(), QsoStep::Idle);
    }

    /// A Fox's paired message: `rr73`'s contact closed, `work` being reported to.
    fn fox_msg(rr73: &str, work: &str, fox: &str) -> Decode {
        Decode {
            message: format!("{rr73} RR73; {work} <{fox}> +02"),
            to: Some(work.into()),
            from: Some(fox.into()),
            rr73_to: Some(rr73.into()),
            ..decode("X Y Z")
        }
    }

    #[test]
    fn a_hound_moves_onto_the_fox_and_logs_on_its_rr73() {
        let cfg = DigiConfig { dxped_mode: DxpedMode::Hound, ..cfg() };
        let mut q = QsoMachine::new(Mode::Ft8, cfg);
        q.start_qso("DX1FOX".into(), None, -10, false, 100);
        assert_eq!(q.plan_tx().as_deref(), Some("DX1FOX AB1CD FN42"));
        assert_eq!(q.take_qsy(), None, "nothing to move onto until the fox answers");

        // The Fox comes back to us, from down in its own part of the band.
        let answer = Decode { audio_hz: 750.0, ..decode("AB1CD DX1FOX -13") };
        assert!(q.on_rx(&[answer], 115));
        assert_eq!(q.step(), QsoStep::TxRReport);
        assert_eq!(q.take_qsy(), Some(750.0), "finish the contact on the fox's frequency");
        assert_eq!(q.take_qsy(), None, "the move is made once");
        assert_eq!(q.plan_tx().as_deref(), Some("DX1FOX AB1CD R-10"));

        // The Fox signs us off inside its message to the next hound: our call is
        // nowhere in the addressing, only in the RR73 half.
        assert!(q.on_rx(&[fox_msg("AB1CD", "W9XYZ", "DX1FOX")], 130));
        assert_eq!(q.step(), QsoStep::Confirming);
        assert!(!q.wants_tx(), "a hound owes the fox nothing after RR73");
        let rec = q.take_completed().expect("logged");
        assert_eq!(rec.call, "DX1FOX");
        assert_eq!(rec.rst_sent, Some(-10));
        assert_eq!(rec.rst_rcvd, Some(-13));
    }

    #[test]
    fn a_hound_does_not_send_73_over_the_pile_up() {
        // An ordinary answerer replies to RR73 with 73; a Hound must not — the
        // Fox logged the contact when it sent the RR73 and is working someone
        // else in that slot.
        let hound = DigiConfig { dxped_mode: DxpedMode::Hound, ..cfg() };
        let mut q = QsoMachine::new(Mode::Ft8, hound);
        q.start_qso("DX1FOX".into(), None, -10, false, 100);
        q.on_rx(&[decode("AB1CD DX1FOX -13")], 115);
        assert!(q.on_rx(&[decode("AB1CD DX1FOX RR73")], 130));
        assert_eq!(q.step(), QsoStep::Confirming);
        assert!(q.take_completed().is_some());

        // The same exchange in normal mode does owe a 73.
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.start_qso("DX1FOX".into(), None, -10, false, 100);
        q.on_rx(&[decode("AB1CD DX1FOX -13")], 115);
        q.on_rx(&[decode("AB1CD DX1FOX RR73")], 130);
        assert_eq!(q.step(), QsoStep::Tx73);
    }

    #[test]
    fn a_hound_message_to_someone_else_is_not_ours_to_log() {
        let cfg = DigiConfig { dxped_mode: DxpedMode::Hound, ..cfg() };
        let mut q = QsoMachine::new(Mode::Ft8, cfg);
        q.start_qso("DX1FOX".into(), None, -10, false, 100);
        q.on_rx(&[decode("AB1CD DX1FOX -13")], 115);
        // The Fox signs off a different hound; we keep calling.
        assert!(!q.on_rx(&[fox_msg("K1ABC", "W9XYZ", "DX1FOX")], 130));
        assert_eq!(q.step(), QsoStep::TxRReport);
        assert!(q.take_completed().is_none());
    }

    #[test]
    fn a_fox_transmits_its_pile_up_in_parallel() {
        let cfg = DigiConfig {
            my_call: "DX1FOX".into(),
            my_grid: "AA00".into(),
            dxped_mode: DxpedMode::Fox,
            fox_slots: 2,
            ..Default::default()
        };
        let mut q = QsoMachine::new(Mode::Ft8, cfg);
        assert!(!q.wants_tx(), "a fox stands still until the operator starts it");

        q.call_cq();
        assert!(q.wants_tx());
        assert_eq!(q.plan_tx_all(), [("CQ DX1FOX AA00".to_string(), 1500.0)]);

        // Two hounds call: both get a report, on tones 60 Hz apart.
        assert!(q.on_rx(&[decode("DX1FOX K1ABC FN42"), decode("DX1FOX W9XYZ EM48")], 100));
        let plan = q.plan_tx_all();
        assert_eq!(plan.len(), 2, "both slots in use: {plan:?}");
        assert_eq!(plan[0].1, 1500.0);
        assert_eq!(plan[1].1, 1560.0);
        assert!(plan.iter().any(|(m, _)| m == "K1ABC DX1FOX -10"), "{plan:?}");
        assert!(plan.iter().any(|(m, _)| m == "W9XYZ DX1FOX -10"), "{plan:?}");
        q.note_tx_sent(115);

        // K1ABC rogers → the next transmission closes it and logs the contact.
        // W9XYZ is already being worked, so there is nobody spare to pair the
        // RR73 with and it travels as an ordinary message.
        assert!(q.on_rx(&[decode("DX1FOX K1ABC R-12")], 130));
        assert_eq!(q.plan_tx().as_deref(), Some("K1ABC DX1FOX RR73"));
        q.note_tx_sent(145);
        let rec = q.take_completed().expect("logged when the RR73 went out");
        assert_eq!(rec.call, "K1ABC");
        assert_eq!(rec.rst_rcvd, Some(-12));
        assert_eq!(rec.my_call, "DX1FOX");
        assert!(q.status(false).fox_queue.iter().any(|c| c.call == "W9XYZ"));

        // Standing the fox down stops it transmitting.
        q.stop();
        assert!(!q.wants_tx());
    }

    #[test]
    fn only_ft8_and_ft2_have_a_fox() {
        // FT4 has no DXpedition layout, so the role is inert there.
        let cfg = DigiConfig {
            my_call: "DX1FOX".into(),
            dxped_mode: DxpedMode::Fox,
            ..Default::default()
        };
        let mut q = QsoMachine::new(Mode::Ft4, cfg);
        q.call_cq();
        assert!(q.status(false).fox_queue.is_empty());
        assert_eq!(q.plan_tx().as_deref(), Some("CQ DX1FOX"));
    }

    fn queued(call: &str) -> QueuedCall {
        QueuedCall {
            call: call.into(),
            grid: None,
            snr_db: -10,
            audio_hz: 1500.0,
            wait_for_cq: false,
        }
    }

    #[test]
    fn the_queue_is_taken_in_order_as_the_sequencer_frees_up() {
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.queue_add(queued("W9XYZ"));
        q.queue_add(queued("K1ABC"));
        assert_eq!(q.take_next_queued().map(|e| e.call), Some("W9XYZ".into()));

        // Mid-contact the queue waits its turn.
        q.start_qso("W9XYZ".into(), Some("EM48".into()), -10, false, 100);
        assert!(q.take_next_queued().is_none(), "a contact in progress holds the queue");
        q.on_rx(&[decode("AB1CD W9XYZ -13")], 115);
        assert!(q.take_next_queued().is_none());

        // The contact completes → the next station is taken straight away,
        // without waiting out the re-send window.
        q.on_rx(&[decode("AB1CD W9XYZ RR73")], 130);
        q.note_tx_sent(145);
        assert_eq!(q.step(), QsoStep::Confirming);
        assert_eq!(q.take_next_queued().map(|e| e.call), Some("K1ABC".into()));
        assert!(q.take_next_queued().is_none(), "and the queue is now empty");
    }

    #[test]
    fn queueing_the_same_station_twice_reorders_rather_than_duplicates() {
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.queue_add(queued("W9XYZ"));
        q.queue_add(queued("K1ABC"));
        q.queue_add(queued("W9XYZ"));
        let calls: Vec<String> = q.status(false).call_queue.into_iter().map(|e| e.call).collect();
        assert_eq!(calls, ["K1ABC", "W9XYZ"]);

        q.queue_remove("k1abc");
        assert_eq!(q.status(false).call_queue.len(), 1, "removal ignores case");
        q.queue_remove("");
        assert!(q.status(false).call_queue.is_empty(), "an empty callsign clears the queue");
    }

    #[test]
    fn working_a_station_settles_its_queue_entry() {
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.queue_add(queued("W9XYZ"));
        q.queue_add(queued("K1ABC"));
        // The operator replies to W9XYZ by hand instead of waiting their turn.
        q.start_qso("W9XYZ".into(), None, -10, false, 100);
        let calls: Vec<String> = q.status(false).call_queue.into_iter().map(|e| e.call).collect();
        assert_eq!(calls, ["K1ABC"], "no second go at the station we just started");
    }

    #[test]
    fn a_queued_station_takes_over_from_an_unanswered_cq() {
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.call_cq();
        q.queue_add(queued("W9XYZ"));
        assert_eq!(
            q.take_next_queued().map(|e| e.call),
            Some("W9XYZ".into()),
            "a marked station beats calling into an empty band"
        );

        // But not once somebody has answered the CQ.
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.call_cq();
        q.on_rx(&[decode("AB1CD K1ABC EM48")], 100);
        q.queue_add(queued("W9XYZ"));
        assert!(q.take_next_queued().is_none());
    }

    #[test]
    fn only_the_operator_takes_the_watchdog_down() {
        // The watchdog exists to stop an unattended station transmitting, so
        // nothing arriving over the air may start it again: resuming because
        // somebody transmitted at us is precisely the unattended operation it
        // is there to prevent. It used to take the readout saying why the
        // sequencer had stopped away with it, too.
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.queue_add(queued("K1ABC"));
        q.start_qso("W9XYZ".into(), Some("EM48".into()), -10, false, 100);
        q.tick(100);
        assert!(q.tick(100 + 6 * 60 + 1));
        assert!(q.tx_watchdog());
        assert_eq!(q.step(), QsoStep::Idle);

        // The station we had been calling comes back.
        q.on_rx(&[decode("AB1CD W9XYZ -13")], 100 + 7 * 60);
        assert!(q.tx_watchdog(), "a decode is not an operator");
        assert!(!q.wants_tx());
        assert_eq!(q.step(), QsoStep::Idle);
        assert!(q.take_next_queued().is_none(), "nor may the queue march on");

        // It still restarts the *clock*, so once the operator does resume, the
        // watchdog gives the contact its full window rather than what was left.
        assert!(!q.tick(100 + 7 * 60 + 1));

        // Answering them is what starts it again.
        q.start_qso("W9XYZ".into(), None, -10, false, 100 + 8 * 60);
        assert!(!q.tx_watchdog());
        assert!(q.wants_tx());
        assert_eq!(q.plan_tx().as_deref(), Some("W9XYZ AB1CD R-10"), "from where they are");
    }

    #[test]
    fn the_watchdog_stops_the_queue_too() {
        // The watchdog exists to stop an unattended station transmitting;
        // marching on down a queue would defeat it.
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.call_cq();
        q.tick(100);
        assert!(q.tick(100 + 6 * 60));
        assert!(q.tx_watchdog());
        q.queue_add(queued("W9XYZ"));
        assert!(q.take_next_queued().is_none());

        // Any operator action clears it and the queue runs again.
        q.call_cq();
        assert_eq!(q.take_next_queued().map(|e| e.call), Some("W9XYZ".into()));
    }

    #[test]
    fn confirming_resends_final_when_dx_repeats() {
        // Answerer: after our 73 we re-send it if the DX repeats their RR73.
        let mut q = QsoMachine::new(Mode::Ft8, cfg());
        q.start_qso("W9XYZ".into(), Some("EM48".into()), -10, false, 100);
        q.on_rx(&[decode("AB1CD W9XYZ -13")], 115);
        q.on_rx(&[decode("AB1CD W9XYZ RR73")], 130);
        q.note_tx_sent(145); // 73 sent → Confirming
        assert_eq!(q.step(), QsoStep::Confirming);
        assert!(!q.wants_tx());

        // They repeat RR73 (didn't hear our 73) → one re-send is queued.
        assert!(q.on_rx(&[decode("AB1CD W9XYZ RR73")], 160));
        assert!(q.wants_tx());
        assert_eq!(q.plan_tx().as_deref(), Some("W9XYZ AB1CD 73"));

        // The re-send goes out → back to standby.
        q.note_tx_sent(175);
        assert!(!q.wants_tx());
        assert_eq!(q.step(), QsoStep::Confirming);

        // After the confirm window the contact is retired.
        assert!(q.tick(145 + CONFIRM_S));
        assert_eq!(q.step(), QsoStep::Idle);
    }

    /// A WSJT-CB free-text decode, exactly as the modem's `parse_message`
    /// leaves it: no addressing in the fields (the addressing lives in the
    /// first token), `is_cq` only for the one free-text form that is a
    /// recognised CQ.
    fn cb_free_text(msg: &str) -> Decode {
        let toks: Vec<&str> = msg.split_whitespace().collect();
        let cq = toks.first() == Some(&"CQ") && toks.get(1).is_some_and(|t| is_cb(t));
        Decode {
            slot_utc: 0,
            snr_db: -10,
            dt: 0.1,
            audio_hz: 1500.0,
            message: msg.to_string(),
            to: None,
            from: cq.then(|| toks[1].to_string()),
            grid: None,
            is_cq: cq,
            cq_to: cq.then(|| toks[1].to_string()),
            free_text: true,
            rr73_to: None,
        }
    }

    /// The whole CB vocabulary fits the free-text wire limit (13 characters)
    /// against both slot shapes — CQ caller and answerer.
    fn assert_all_cb_shapes_fit(msg: &str) {
        assert!(
            msg.len() <= 13,
            "{msg}: a WSJT-CB line must fit the free-text limit"
        );
        assert!(
            msg.bytes().all(|b| matches!(b, b'0'..=b'9' | b'A'..=b'Z' | b' ' | b'+' | b'-' | b'.' | b'/' | b'?')),
            "{msg}: outside the free-text alphabet"
        );
    }

    /// We (a CB station) call CQ; a CB peer answers; the two run the whole
    /// WSJT-CB exchange in free text and the contact logs. WSJT-CB's own note
    /// is that such a pair "cannot reliably carry signal reports", so the
    /// shortening is deliberate: every message is an address plus one token.
    #[test]
    fn a_cb_pair_runs_the_wsjt_cb_free_text_exchange() {
        let mut q = QsoMachine::new(
            Mode::Ft8,
            DigiConfig { my_call: "26AT715".into(), my_grid: "JO21".into(), ..Default::default() },
        );
        q.call_cq();
        let cq = q.plan_tx().expect("CQ message");
        assert_eq!(cq, "CQ 26AT715");
        assert_all_cb_shapes_fit(&cq);

        // 1A1 answers by spelling our call first — that is what makes it a
        // reply. There is no grid in a CB exchange, so the adopted answerer
        // is sent their report at us.
        assert!(q.on_rx(&[cb_free_text("26AT715 1A1")], 115));
        assert_eq!(q.step(), QsoStep::TxReport);
        let a = q.plan_tx().expect("report");
        assert_eq!(a, "1A1 -10");
        assert_all_cb_shapes_fit(&a);

        // Their rogered report, sender omitted from the finished exchange.
        q.note_tx_sent(130);
        assert!(q.on_rx(&[cb_free_text("26AT715 R-07")], 145));
        assert_eq!(q.step(), QsoStep::TxRr73);
        let b = q.plan_tx().expect("RR73");
        assert_eq!(b, "1A1 RR73");
        assert_all_cb_shapes_fit(&b);

        // Our final goes out and the contact logs.
        q.note_tx_sent(160);
        assert_eq!(q.take_completed().expect("logged").call, "1A1");
        assert_eq!(q.step(), QsoStep::Confirming);
        assert_eq!(q.plan_tx(), None, "nothing more until they repeat theirs");
    }

    /// We answer a WSJT-CB station's free-text CQ from the picked-station
    /// hold, the same way the ordinary exchange answers a CQ (issue coverage:
    /// the CQ recognition comes from `parse_message`'s free-text arm).
    #[test]
    fn answering_a_cb_cq_uses_the_free_text_vocabulary() {
        let mut q = QsoMachine::new(
            Mode::Ft8,
            DigiConfig { my_call: "1A1".into(), ..Default::default() },
        );
        q.start_qso("26AT715".into(), None, -8, true, 100);
        assert!(q.on_rx(&[cb_free_text("CQ 26AT715")], 115));
        assert_eq!(q.step(), QsoStep::TxGrid);
        let open = q.plan_tx().expect("address pair");
        assert_eq!(open, "26AT715 1A1");
        assert_all_cb_shapes_fit(&open);

        // Their report is addressed to us by name; the sender is implied.
        q.note_tx_sent(130);
        assert!(q.on_rx(&[cb_free_text("1A1 R-07")], 145));
        assert_eq!(q.step(), QsoStep::TxRr73);
        let final_msg = q.plan_tx().expect("RR73");
        assert_eq!(final_msg, "26AT715 RR73");
        assert_all_cb_shapes_fit(&final_msg);
    }
}
