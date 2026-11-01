//! What to answer, and — mostly — what not to.
//!
//! This is the only place in the JS8 implementation that decides to transmit
//! without an operator asking, so it is deliberately conservative. Every rule
//! here exists to stop the station doing something on the air that its operator
//! would not have chosen:
//!
//! * Only queries **addressed to us** are answered. A query to `@ALLCALL` is
//!   addressed to us; one to another callsign is not, however clearly we heard
//!   it.
//! * Only the commands upstream marks auto-repliable (`autoreply_cmds`) are
//!   answered at all. Everything else is displayed and left to the operator.
//! * A heartbeat is answered with a report only when the operator switched
//!   that on, and then only as often as the caller allows. A CQ is never
//!   answered: it asks for a contact, not a report, and starting one is the
//!   operator's decision.
//! * We never answer ourselves. One misconfiguration — an operator's own
//!   callsign arriving via a relay, or a loopback — would otherwise become a
//!   two-station beacon with no off switch.
//! * Answering is off unless the operator turned it on.
//!
//! Relay (`>`), the message store (` MSG TO:`, ` QUERY MSGS`) and the APRS
//! gateway are decoded and displayed but never acted on. They are out of scope
//! by choice, not oversight: each one makes this station responsible for
//! traffic it did not originate.

use sdroxide_types::Js8Msg;

use super::frame::{AUTOREPLY_CMDS, Directed, cmd_code};

/// What a received message is asking of us.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Js8Intent {
    /// "What is my signal report?"
    SnrQuery,
    /// "What is your grid?"
    GridQuery,
    /// "Who are you hearing?"
    HearingQuery,
    /// "What is your status message?"
    StatusQuery,
    /// "What is your station information?"
    InfoQuery,
    /// "Say again."
    RepeatQuery,
    /// A station announcing itself.
    Heartbeat,
    /// A call for contacts.
    Cq,
    /// Something we understand but do not answer.
    Other,
}

/// Station facts an auto-reply may quote.
#[derive(Debug, Clone, Default)]
pub struct StationInfo {
    pub call: String,
    pub grid: String,
    /// Free-text status the operator set, sent in reply to ` STATUS?`.
    pub status: String,
    /// Free-text station information, sent in reply to ` INFO?`.
    pub info: String,
    /// Callsigns heard recently, most recent first, for ` HEARING?`.
    pub hearing: Vec<String>,
}

/// A reply, ready to be framed.
///
/// Two parts, because a directed frame has room for a command and a six-bit
/// number and nothing else. ` SNR` fits its answer in that number; a grid, a
/// status line or a list of stations heard does not, and has to follow as free
/// text in data frames — which is exactly what JS8Call does
/// (`varicode.cpp`, `buildMessageFrames`: the directed pack consumes
/// `CALLSIGN CMD` and the rest of the line goes on to the data path).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Js8Reply {
    pub directed: Directed,
    /// Text the command cannot carry itself, or `None` when it can.
    pub body: Option<String>,
}

/// Policy switches for one received message.
#[derive(Debug, Clone, Copy)]
pub struct ReplyPolicy {
    /// Master switch. Off means this module never produces a transmission.
    pub auto_reply: bool,
    /// Whether *this* heartbeat may be acknowledged.
    ///
    /// Not simply the operator's setting: acknowledging is rate-limited per
    /// station, suspended while a multi-frame message is still arriving, and
    /// unavailable at Turbo — all of which need a clock and the engine's state,
    /// so the caller decides and passes the answer in. Keeping it out of here
    /// is what lets this module stay a pure function of the message.
    pub hb_ack: bool,
}

impl Default for ReplyPolicy {
    fn default() -> Self {
        // Answering questions is the useful default and is what makes a JS8
        // station worth leaving on. Answering announcements is not: see
        // [`DigiConfig::js8_hb_ack`](sdroxide_types::DigiConfig::js8_hb_ack).
        ReplyPolicy { auto_reply: true, hb_ack: false }
    }
}

/// Classify a received message.
pub fn intent(msg: &Js8Msg) -> Js8Intent {
    match msg.cmd.as_deref() {
        Some("SNR?") => Js8Intent::SnrQuery,
        Some("GRID?") => Js8Intent::GridQuery,
        Some("HEARING?") => Js8Intent::HearingQuery,
        Some("STATUS?") => Js8Intent::StatusQuery,
        Some("INFO?") => Js8Intent::InfoQuery,
        Some("AGN?") => Js8Intent::RepeatQuery,
        Some("HB") => Js8Intent::Heartbeat,
        Some("CQ") => Js8Intent::Cq,
        _ => Js8Intent::Other,
    }
}

/// The reply this message deserves, if any.
///
/// Returns the directed frame to transmit and, when the command cannot carry
/// its own answer, the text that has to follow it. `None` means "say nothing",
/// which is the answer for the large majority of received traffic.
pub fn auto_reply(msg: &Js8Msg, me: &StationInfo, policy: ReplyPolicy) -> Option<Js8Reply> {
    if !policy.auto_reply || me.call.is_empty() {
        return None;
    }
    // Not for us, not our problem. `to_me` already covers @ALLCALL and groups.
    if !msg.to_me || msg.from.is_empty() {
        return None;
    }
    // The loop guard. Worth keeping even though the sender should never be us:
    // relays and loopbacks make it possible, and the failure mode is a station
    // that transmits forever.
    if msg.from.eq_ignore_ascii_case(&me.call) {
        return None;
    }
    // Only complete messages — replying to half a query invites replying twice.
    if !msg.complete {
        return None;
    }

    // A heartbeat is an announcement, not a question — but the mode's whole
    // point is that a beacon gets a report back, so the sender learns who is
    // copying them. It has a switch of its own because answering every one of
    // them would flood the band; `hb_ack` is the caller's verdict for this
    // message, rate limit and all.
    if intent(msg) == Js8Intent::Heartbeat {
        if !policy.hb_ack {
            return None;
        }
        return Some(Js8Reply {
            directed: Directed {
                from: me.call.clone(),
                to: msg.from.clone(),
                cmd: cmd_code(" HEARTBEAT SNR")?,
                num: Some(i32::from(msg.snr_db)),
                portable_from: false,
                portable_to: false,
            },
            body: None,
        });
    }

    // `body` is what the frame cannot say by itself. A signal report rides in
    // the directed frame's number field, so it has none; everything else is
    // text that must follow.
    let (cmd_text, body) = match intent(msg) {
        Js8Intent::SnrQuery => (" SNR", None),
        Js8Intent::GridQuery => (" GRID", Some(me.grid.clone())),
        Js8Intent::StatusQuery => (" STATUS", Some(me.status.clone())),
        Js8Intent::InfoQuery => (" INFO", Some(me.info.clone())),
        Js8Intent::HearingQuery => (" HEARING", Some(me.hearing.join(" "))),
        // A CQ is an announcement too, but unlike a heartbeat it is asking for
        // a contact rather than a report — answering it automatically would
        // start a QSO the operator did not agree to have.
        _ => return None,
    };

    let cmd = cmd_code(cmd_text)?;
    // Belt and braces: refuse anything upstream does not mark auto-repliable,
    // so widening the match above cannot quietly widen what we transmit.
    let query_cmd = msg.cmd.as_deref().and_then(|c| cmd_code(&format!(" {c}")))?;
    if !AUTOREPLY_CMDS.contains(&query_cmd) {
        return None;
    }

    // An empty answer is worse than none — it says "I am here" while answering
    // nothing, and costs a full transmission to do it.
    let body = body.map(|b| b.trim().to_string());
    if body.as_deref().is_some_and(str::is_empty) {
        return None;
    }

    Some(Js8Reply {
        directed: Directed {
            from: me.call.clone(),
            to: msg.from.clone(),
            cmd,
            num: if matches!(intent(msg), Js8Intent::SnrQuery) {
                Some(i32::from(msg.snr_db))
            } else {
                None
            },
            portable_from: false,
            portable_to: false,
        },
        body,
    })
}

/// A message this station sent, kept so ` AGN?` can say it again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastTx {
    /// Who it was addressed to: a callsign, a group, or empty for undirected
    /// text.
    pub to: String,
    pub text: String,
    pub unix: i64,
}

/// How long a message stays worth repeating. ` AGN?` means "say that again",
/// and after ten minutes "that" is no longer the same conversation.
pub const AGN_WINDOW_S: i64 = 10 * 60;

/// The text to send again in answer to an ` AGN?`, if any.
///
/// We repeat only what we said to the station that asks. That is either a
/// message addressed to them, or one sent to the group they ask through.
/// Anything else would put our last words to one station on the air for
/// another, which is not what they asked for. A ` AGN?` is a reply to
/// something heard moments ago, so the message must also be recent.
pub fn repeat_reply(
    msg: &Js8Msg,
    me: &StationInfo,
    auto_reply: bool,
    last: Option<&LastTx>,
    now_unix: i64,
) -> Option<String> {
    if !auto_reply || me.call.is_empty() || intent(msg) != Js8Intent::RepeatQuery {
        return None;
    }
    if !msg.to_me || !msg.complete || msg.from.is_empty() || msg.from.eq_ignore_ascii_case(&me.call)
    {
        return None;
    }
    let last = last?;
    if now_unix - last.unix > AGN_WINDOW_S || last.text.trim().is_empty() {
        return None;
    }
    let theirs = last.to.eq_ignore_ascii_case(&msg.from)
        || (last.to.starts_with('@') && last.to.eq_ignore_ascii_case(&msg.to));
    theirs.then(|| last.text.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn me() -> StationInfo {
        StationInfo {
            call: "N0JDS".into(),
            grid: "FN42".into(),
            status: "IDLE".into(),
            info: "IC-7300 50W DIPOLE".into(),
            hearing: vec!["KN4CRD".into(), "VK3ABC".into()],
        }
    }

    fn query(from: &str, to: &str, cmd: &str, to_me: bool) -> Js8Msg {
        Js8Msg {
            from: from.into(),
            to: to.into(),
            text: String::new(),
            cmd: Some(cmd.into()),
            snr_db: -7,
            audio_hz: 1500.0,
            first_slot_utc: 1000,
            last_slot_utc: 1000,
            frames: 1,
            complete: true,
            to_me,
            speed: sdroxide_types::Js8Speed::Normal,
        }
    }

    #[test]
    fn a_query_addressed_to_us_is_answered() {
        for (cmd, expect) in
            [("SNR?", " SNR"), ("GRID?", " GRID"), ("STATUS?", " STATUS"), ("HEARING?", " HEARING")]
        {
            let m = query("KN4CRD", "N0JDS", cmd, true);
            let r = auto_reply(&m, &me(), ReplyPolicy::default())
                .unwrap_or_else(|| panic!("{cmd} went unanswered"));
            assert_eq!(r.directed.to, "KN4CRD");
            assert_eq!(r.directed.from, "N0JDS");
            assert_eq!(super::super::frame::cmd_text(r.directed.cmd), Some(expect));
        }
    }

    /// The answer has to actually leave the station. A directed frame holds a
    /// command and a six-bit number and nothing else, so every reply whose
    /// content is *text* has to carry that text alongside — otherwise the
    /// station transmits a bare " GRID" and says nothing at all.
    #[test]
    fn an_answer_made_of_text_carries_it_alongside_the_command() {
        for (cmd, expect) in [("GRID?", "FN42"), ("STATUS?", "IDLE"), ("HEARING?", "KN4CRD VK3ABC")]
        {
            let m = query("KN4CRD", "N0JDS", cmd, true);
            let r = auto_reply(&m, &me(), ReplyPolicy::default()).expect("answered");
            assert_eq!(r.body.as_deref(), Some(expect), "{cmd}");
        }
    }

    #[test]
    fn a_report_needs_no_text_because_the_frame_carries_it() {
        // ` SNR` puts its value in the directed frame's number field. Sending
        // it as text as well would double the transmission to say it twice.
        let m = query("KN4CRD", "N0JDS", "SNR?", true);
        let r = auto_reply(&m, &me(), ReplyPolicy::default()).expect("answered");
        assert_eq!(r.body, None);
        assert_eq!(r.directed.num, Some(-7));
    }

    #[test]
    fn a_query_for_someone_else_is_ignored() {
        let m = query("KN4CRD", "VK3ABC", "SNR?", false);
        assert!(auto_reply(&m, &me(), ReplyPolicy::default()).is_none());
    }

    #[test]
    fn an_allcall_query_is_answered() {
        // @ALLCALL reaches us, and the assembler sets `to_me` accordingly.
        let m = query("KN4CRD", "@ALLCALL", "SNR?", true);
        assert!(auto_reply(&m, &me(), ReplyPolicy::default()).is_some());
    }

    #[test]
    fn we_never_answer_ourselves() {
        // The loop that turns one misconfiguration into an unattended beacon.
        let m = query("N0JDS", "N0JDS", "SNR?", true);
        assert!(auto_reply(&m, &me(), ReplyPolicy::default()).is_none());
        let m = query("n0jds", "N0JDS", "GRID?", true);
        assert!(auto_reply(&m, &me(), ReplyPolicy::default()).is_none(), "case must not matter");
    }

    #[test]
    fn auto_reply_is_off_when_the_operator_turned_it_off() {
        let m = query("KN4CRD", "N0JDS", "SNR?", true);
        assert!(auto_reply(&m, &me(), ReplyPolicy { auto_reply: false, hb_ack: false }).is_none());
    }

    #[test]
    fn a_station_with_no_callsign_stays_silent() {
        // Transmitting without a callsign is not legal anywhere.
        let m = query("KN4CRD", "N0JDS", "SNR?", true);
        let anon = StationInfo { call: String::new(), ..me() };
        assert!(auto_reply(&m, &anon, ReplyPolicy::default()).is_none());
    }

    #[test]
    fn announcements_are_not_answered_by_default() {
        // Replying to every heartbeat would flood exactly the band they exist
        // to keep quiet, so acknowledging is off until the operator says so.
        for cmd in ["HB", "CQ"] {
            let m = query("KN4CRD", "@ALLCALL", cmd, true);
            assert!(auto_reply(&m, &me(), ReplyPolicy::default()).is_none(), "{cmd}");
        }
    }

    #[test]
    fn a_heartbeat_is_acknowledged_with_a_report_when_that_is_switched_on() {
        // What the acknowledgement is *for*: the station beaconing learns who
        // copied them, and how well.
        let m = query("KN4CRD", "@ALLCALL", "HB", true);
        let policy = ReplyPolicy { hb_ack: true, ..ReplyPolicy::default() };
        let r = auto_reply(&m, &me(), policy).expect("acknowledged");
        assert_eq!(super::super::frame::cmd_text(r.directed.cmd), Some(" HEARTBEAT SNR"));
        assert_eq!(r.directed.to, "KN4CRD");
        assert_eq!(r.directed.num, Some(-7), "the report is the point of the reply");
        assert_eq!(r.body, None, "the frame carries the report itself");
    }

    #[test]
    fn a_cq_is_never_acknowledged_however_the_switches_are_set() {
        // A CQ asks for a contact. Answering one automatically starts a QSO
        // the operator did not agree to have.
        let m = query("KN4CRD", "@ALLCALL", "CQ", true);
        let policy = ReplyPolicy { auto_reply: true, hb_ack: true };
        assert!(auto_reply(&m, &me(), policy).is_none());
    }

    #[test]
    fn acknowledging_still_obeys_every_other_rule() {
        let policy = ReplyPolicy { auto_reply: true, hb_ack: true };
        // Our own heartbeat back through a relay or a loopback: the failure
        // mode is two stations beaconing at each other forever.
        let mine = query("N0JDS", "@ALLCALL", "HB", true);
        assert!(auto_reply(&mine, &me(), policy).is_none());
        // Auto-reply off is off, whatever the heartbeat switch says.
        let m = query("KN4CRD", "@ALLCALL", "HB", true);
        assert!(auto_reply(&m, &me(), ReplyPolicy { auto_reply: false, hb_ack: true }).is_none());
        // And a station with no callsign may not transmit at all.
        let anon = StationInfo { call: String::new(), ..me() };
        assert!(auto_reply(&m, &anon, policy).is_none());
    }

    #[test]
    fn commands_upstream_does_not_auto_reply_to_are_left_alone() {
        for cmd in ["QSL", "73", "SK", "RR", "FB"] {
            let m = query("KN4CRD", "N0JDS", cmd, true);
            assert!(auto_reply(&m, &me(), ReplyPolicy::default()).is_none(), "{cmd}");
        }
    }

    #[test]
    fn relay_and_message_store_requests_are_shown_but_not_acted_on() {
        // Explicitly out of scope: each makes this station responsible for
        // traffic it did not originate.
        for cmd in [">", "MSG TO:", "QUERY MSGS"] {
            let m = query("KN4CRD", "N0JDS", cmd, true);
            assert!(auto_reply(&m, &me(), ReplyPolicy::default()).is_none(), "{cmd}");
        }
    }

    #[test]
    fn a_half_received_query_is_not_answered_yet() {
        let mut m = query("KN4CRD", "N0JDS", "SNR?", true);
        m.complete = false;
        assert!(auto_reply(&m, &me(), ReplyPolicy::default()).is_none());
    }

    #[test]
    fn we_say_nothing_rather_than_answer_emptily() {
        // A station with no grid set should not transmit " GRID" and nothing.
        let m = query("KN4CRD", "N0JDS", "GRID?", true);
        let blank = StationInfo { grid: String::new(), ..me() };
        assert!(auto_reply(&m, &blank, ReplyPolicy::default()).is_none());

        let m = query("KN4CRD", "N0JDS", "HEARING?", true);
        let deaf = StationInfo { hearing: Vec::new(), ..me() };
        assert!(auto_reply(&m, &deaf, ReplyPolicy::default()).is_none());
    }

    #[test]
    fn station_information_is_answered_when_there_is_some() {
        let m = query("KN4CRD", "N0JDS", "INFO?", true);
        let r = auto_reply(&m, &me(), ReplyPolicy::default()).expect("answered");
        assert_eq!(super::super::frame::cmd_text(r.directed.cmd), Some(" INFO"));
        assert_eq!(r.body.as_deref(), Some("IC-7300 50W DIPOLE"));
        let blank = StationInfo { info: String::new(), ..me() };
        assert!(auto_reply(&m, &blank, ReplyPolicy::default()).is_none());
    }

    #[test]
    fn agn_repeats_only_what_we_said_to_them_and_only_recently() {
        let t = 10_000;
        let last =
            |to: &str| LastTx { to: to.into(), text: format!("{to} QTH IS BOSTON"), unix: t };
        let m = query("KN4CRD", "N0JDS", "AGN?", true);
        assert_eq!(
            repeat_reply(&m, &me(), true, Some(&last("KN4CRD")), t + 60).as_deref(),
            Some("KN4CRD QTH IS BOSTON")
        );
        // Said to somebody else.
        assert_eq!(repeat_reply(&m, &me(), true, Some(&last("VK3ABC")), t + 60), None);
        // Too long ago.
        assert_eq!(
            repeat_reply(&m, &me(), true, Some(&last("KN4CRD")), t + AGN_WINDOW_S + 1),
            None
        );
        // Auto-reply off, or nothing sent yet.
        assert_eq!(repeat_reply(&m, &me(), false, Some(&last("KN4CRD")), t + 60), None);
        assert_eq!(repeat_reply(&m, &me(), true, None, t + 60), None);
        // Said to a group, and asked again through that group.
        let g = query("KN4CRD", "@EMCOMM", "AGN?", true);
        assert!(repeat_reply(&g, &me(), true, Some(&last("@EMCOMM")), t + 60).is_some());
        // Ourselves, through a relay or loopback.
        let mine = query("N0JDS", "N0JDS", "AGN?", true);
        assert_eq!(repeat_reply(&mine, &me(), true, Some(&last("N0JDS")), t + 60), None);
    }

    #[test]
    fn intent_classifies_the_commands_we_act_on() {
        assert_eq!(intent(&query("A", "B", "SNR?", true)), Js8Intent::SnrQuery);
        assert_eq!(intent(&query("A", "B", "GRID?", true)), Js8Intent::GridQuery);
        assert_eq!(intent(&query("A", "B", "QSL", true)), Js8Intent::Other);
    }
}
