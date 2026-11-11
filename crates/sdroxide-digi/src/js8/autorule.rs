//! Group auto-responses: a canned reply to a message sent to a group.
//!
//! [`super::directed`] answers questions addressed to this station. This
//! module answers *announcements to a group*, which nobody asked this station
//! about personally. That makes it the more dangerous of the two, so the rules
//! here are deliberately narrow:
//!
//! * A rule fires only on a **complete** message sent **to its group**.
//! * The station **never answers itself**, including its own message coming
//!   back through a relay or a loopback.
//! * The phrase match ignores case and spacing, so "qrv?" matches "QRV?".
//!   Nothing fuzzier than that: an automatic transmitter should be
//!   predictable from what the operator wrote.
//!
//! Scheduling (repeats, cooldowns, the hourly cap) needs a clock and lives in
//! the controller. Everything here is a pure function of the rule and the
//! message, so it can be tested without one.
//!
//! Rendering the reply lives on [`Js8AutoRule`] in `sdroxide-types`. It is
//! this project's own logic, not JS8Call's, so the settings dialog can preview
//! a reply without linking this crate.

use sdroxide_types::{Js8AutoRule, Js8Msg, js8_group_normalize, js8_normalize_text};

/// The callsign without a portable or other suffix, for the self test.
fn base_call(call: &str) -> &str {
    call.split('/').max_by_key(|p| p.len()).unwrap_or(call)
}

/// Whether `msg` triggers `rule` at a station whose callsign is `my_call`.
///
/// Group membership is not required: a rule on @EMCOMM is itself the
/// operator's statement that they want to act on @EMCOMM traffic.
pub fn matches(rule: &Js8AutoRule, msg: &Js8Msg, my_call: &str) -> bool {
    if !rule.enabled || !msg.complete {
        return false;
    }
    let from = msg.from.trim();
    if from.is_empty() || my_call.trim().is_empty() {
        return false;
    }
    if from.eq_ignore_ascii_case(my_call)
        || base_call(from).eq_ignore_ascii_case(base_call(my_call))
    {
        return false;
    }
    // A heartbeat is addressed to @ALLCALL so that HB AUTO can acknowledge it,
    // but it is a beacon, not a message: a rule on @ALLCALL would otherwise
    // answer every station beaconing on the band.
    if msg.cmd.as_deref().is_some_and(|c| c.trim().eq_ignore_ascii_case("HB")) {
        return false;
    }
    let group = js8_group_normalize(&rule.group);
    if group.is_empty() || !msg.to.trim().eq_ignore_ascii_case(&group) {
        return false;
    }
    // The command and the text together are what the sender typed: "@EMCOMM
    // QRV?" arrives as free text, while "@EMCOMM SNR?" arrives as a command.
    let said = js8_normalize_text(&format!("{} {}", msg.cmd.as_deref().unwrap_or(""), msg.text));
    let want = js8_normalize_text(&rule.contains);
    want.is_empty() || said.contains(&want)
}

/// Facts about this station that a reply may quote.
#[derive(Debug, Clone, Default)]
pub struct Me {
    pub call: String,
    pub grid: String,
}

/// The text the rule sends in answer to `msg`, addressee included, or `None`
/// when there is nothing to send. See [`Js8AutoRule::render_for`].
pub fn render(rule: &Js8AutoRule, msg: &Js8Msg, me: &Me) -> Option<String> {
    rule.render_for(&msg.from, &me.call, &me.grid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_types::Js8AutoReplyTo;

    fn rule() -> Js8AutoRule {
        Js8AutoRule {
            name: "net check-in".into(),
            group: "@EMCOMM".into(),
            contains: "qrv?".into(),
            reply: "QRV @MYGRID".into(),
            ..Default::default()
        }
    }

    fn to_group(from: &str, to: &str, cmd: Option<&str>, text: &str) -> Js8Msg {
        Js8Msg {
            from: from.into(),
            to: to.into(),
            text: text.into(),
            cmd: cmd.map(str::to_string),
            snr_db: -10,
            audio_hz: 1500.0,
            first_slot_utc: 1000,
            last_slot_utc: 1015,
            frames: 2,
            complete: true,
            to_me: false,
            speed: sdroxide_types::Js8Speed::Normal,
        }
    }

    fn me() -> Me {
        Me { call: "N0JDS".into(), grid: "FN42AB".into() }
    }

    #[test]
    fn a_message_to_the_group_with_the_phrase_triggers_the_rule() {
        let m = to_group("KN4CRD", "@EMCOMM", None, "ANY STATIONS  QRV? PSE");
        assert!(matches(&rule(), &m, "N0JDS"));
        // Case and the group's spelling do not matter.
        let lower = Js8AutoRule { group: "emcomm".into(), ..rule() };
        assert!(matches(&lower, &m, "n0jds"));
    }

    #[test]
    fn an_empty_phrase_matches_anything_sent_to_the_group() {
        let any = Js8AutoRule { contains: String::new(), ..rule() };
        assert!(matches(&any, &to_group("KN4CRD", "@EMCOMM", None, "HELLO"), "N0JDS"));
    }

    #[test]
    fn a_command_counts_as_part_of_what_was_said() {
        let snr = Js8AutoRule { contains: "SNR?".into(), ..rule() };
        assert!(matches(&snr, &to_group("KN4CRD", "@EMCOMM", Some("SNR?"), ""), "N0JDS"));
    }

    #[test]
    fn anything_else_leaves_the_rule_alone() {
        let r = rule();
        // Another group, or no group at all.
        assert!(!matches(&r, &to_group("KN4CRD", "@ARES", None, "QRV?"), "N0JDS"));
        assert!(!matches(&r, &to_group("KN4CRD", "N0JDS", None, "QRV?"), "N0JDS"));
        assert!(!matches(&r, &to_group("KN4CRD", "", None, "QRV?"), "N0JDS"));
        // The phrase is missing.
        assert!(!matches(&r, &to_group("KN4CRD", "@EMCOMM", None, "HELLO"), "N0JDS"));
        // Half a message.
        let mut part = to_group("KN4CRD", "@EMCOMM", None, "QRV?");
        part.complete = false;
        assert!(!matches(&r, &part, "N0JDS"));
        // Switched off.
        let off = Js8AutoRule { enabled: false, ..rule() };
        assert!(!matches(&off, &to_group("KN4CRD", "@EMCOMM", None, "QRV?"), "N0JDS"));
        // Nobody named as the sender.
        assert!(!matches(&r, &to_group("", "@EMCOMM", None, "QRV?"), "N0JDS"));
    }

    #[test]
    fn a_heartbeat_is_not_a_message_to_the_group() {
        let allcall = Js8AutoRule { group: "@ALLCALL".into(), contains: String::new(), ..rule() };
        let hb = to_group("KN4CRD", "@ALLCALL", Some("HB"), "EM73");
        assert!(!matches(&allcall, &hb, "N0JDS"));
        // Text sent to @ALLCALL still counts.
        assert!(matches(&allcall, &to_group("KN4CRD", "@ALLCALL", None, "QRV?"), "N0JDS"));
    }

    #[test]
    fn we_never_answer_ourselves_however_our_call_is_written() {
        let r = rule();
        for me in ["N0JDS", "N0JDS/P", "N0JDS/QRP", "VE/N0JDS"] {
            let m = to_group(me, "@EMCOMM", None, "QRV?");
            assert!(!matches(&r, &m, "N0JDS"), "{me}");
        }
        assert!(!matches(&r, &to_group("KN4CRD", "@EMCOMM", None, "QRV?"), ""), "no callsign");
    }

    #[test]
    fn the_reply_is_addressed_as_the_rule_says() {
        let m = to_group("KN4CRD", "@EMCOMM", None, "QRV?");
        assert_eq!(render(&rule(), &m, &me()).as_deref(), Some("KN4CRD QRV FN42"));
        let g = Js8AutoRule { reply_to: Js8AutoReplyTo::Group, ..rule() };
        assert_eq!(render(&g, &m, &me()).as_deref(), Some("@EMCOMM QRV FN42"));
        let w = Js8AutoRule {
            reply_to: Js8AutoReplyTo::AsWritten,
            reply: "@SENDER DE @MYCALL ON @GROUP".into(),
            ..rule()
        };
        assert_eq!(render(&w, &m, &me()).as_deref(), Some("KN4CRD DE N0JDS ON @EMCOMM"));
    }

    #[test]
    fn an_addressee_already_written_is_not_repeated() {
        let r = Js8AutoRule { reply: "@SENDER QRV".into(), ..rule() };
        let m = to_group("KN4CRD", "@EMCOMM", None, "QRV?");
        assert_eq!(render(&r, &m, &me()).as_deref(), Some("KN4CRD QRV"));
    }

    #[test]
    fn an_empty_reply_sends_nothing() {
        let r = Js8AutoRule { reply: "   ".into(), ..rule() };
        assert_eq!(render(&r, &to_group("KN4CRD", "@EMCOMM", None, "QRV?"), &me()), None);
    }
}
