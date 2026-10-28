//! Messages, acknowledgements, rejections and bulletins (chapter 14), and the
//! status report (chapter 16).
//!
//! The one part of APRS that is a conversation rather than a broadcast, and the
//! only part with a retry: a message carrying an identifier is retransmitted
//! until the addressee acknowledges it or the sender gives up.

use crate::{AprsError, Result, printable};

/// What a `:` frame turned out to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    /// Traffic for one station.
    Text,
    /// An acknowledgement of a message we (or somebody) sent.
    Ack,
    /// A refusal: the addressee will not take it. Distinct from silence — a
    /// rejected message must not be retried.
    Rej,
    /// A bulletin or an announcement, addressed to `BLNn`. Broadcast to the
    /// channel; never acknowledged.
    Bulletin,
    /// A telemetry definition — the parameter names, units, coefficients or
    /// bit labels a telemetry-sending station publishes about itself. Sent as
    /// a message so that it can be addressed and repeated.
    TelemetryDef,
}

impl MessageKind {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            MessageKind::Text => "message",
            MessageKind::Ack => "ack",
            MessageKind::Rej => "reject",
            MessageKind::Bulletin => "bulletin",
            MessageKind::TelemetryDef => "telemetry def",
        }
    }
}

/// One message frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// Who it is for, trimmed of the padding that makes the field nine wide.
    pub addressee: String,
    pub text: String,
    /// The message number, empty when there is none. A message sent without
    /// one is an announcement: nobody acknowledges it and nothing retries it.
    pub id: String,
    /// The identifier this frame simultaneously acknowledges — the "reply-ack"
    /// of addendum 1.2, which saves a whole transmission by folding the
    /// acknowledgement into the reply.
    pub reply_ack: String,
    pub kind: MessageKind,
}

/// Parse the body of a `:` frame.
pub(crate) fn parse(rest: &[u8]) -> Result<Message> {
    // The addressee is a (space-padded, nine-wide) field followed by a colon.
    // Real traffic is not always padded to the full nine characters, so the
    // colon that ends the field is found rather than assumed at a fixed offset.
    // The addressee itself carries no colon, so the first colon in the field is
    // unambiguously the delimiter — even when the message text contains one
    // (a time, say) or an identifier.
    let colon = rest.iter().position(|&b| b == b':').ok_or(AprsError::Malformed("message addressee"))?;
    if colon == 0 || colon > 9 {
        return Err(AprsError::Malformed("message addressee"));
    }
    let addressee = printable(&rest[..colon]).trim_end().to_string();
    let body = printable(&rest[colon + 1..]);

    // `ack` and `rej` are recognised before anything else: their identifier is
    // the rest of the line rather than something after a brace.
    for (tag, kind) in [("ack", MessageKind::Ack), ("rej", MessageKind::Rej)] {
        if let Some(id) = body.strip_prefix(tag) {
            // Not every `ack…` is an acknowledgement — "acknowledged, thanks"
            // is a message. The identifier is 1 to 5 characters and nothing
            // else follows it.
            let id = id.trim_end();
            if !id.is_empty() && id.len() <= 5 && id.chars().all(|c| c.is_ascii_alphanumeric()) {
                return Ok(Message {
                    addressee,
                    text: String::new(),
                    id: id.to_string(),
                    reply_ack: String::new(),
                    kind,
                });
            }
        }
    }

    // The identifier, when there is one, is everything after the last `{`.
    let (text, id, reply_ack) = match body.rfind('{') {
        Some(i) if body.len() - i <= 12 => {
            let tail = &body[i + 1..];
            // Reply-ack: `{aa}bb` is "here is my message aa, and it also
            // acknowledges your bb".
            match tail.split_once('}') {
                Some((mine, theirs)) => {
                    (body[..i].to_string(), mine.to_string(), theirs.to_string())
                }
                None => (body[..i].to_string(), tail.to_string(), String::new()),
            }
        }
        _ => (body.clone(), String::new(), String::new()),
    };

    let kind = if addressee.starts_with("BLN") {
        MessageKind::Bulletin
    } else if ["PARM.", "UNIT.", "EQNS.", "BITS."].iter().any(|p| text.starts_with(p)) {
        MessageKind::TelemetryDef
    } else {
        MessageKind::Text
    };
    Ok(Message { addressee, text: text.trim_end().to_string(), id, reply_ack, kind })
}

/// The nine-character addressee field.
fn pad9(to: &str) -> String {
    let mut s: String =
        to.chars().filter(char::is_ascii_graphic).map(|c| c.to_ascii_uppercase()).take(9).collect();
    while s.chars().count() < 9 {
        s.push(' ');
    }
    s
}

/// Text a message may carry: printable, no `{` (which would look like the
/// start of an identifier and truncate the message), no `|` or `~` (which are
/// TNC stream-switch characters), and no more than the 67 the format allows.
fn clean(text: &str) -> String {
    text.chars()
        .filter(|&c| (c.is_ascii_graphic() || c == ' ') && !matches!(c, '{' | '|' | '~'))
        .take(67)
        .collect()
}

/// A message. `id` empty sends it as an announcement — no acknowledgement is
/// asked for and none will come.
#[must_use]
pub fn encode_message(to: &str, text: &str, id: &str) -> String {
    let mut s = format!(":{}:{}", pad9(to), clean(text));
    if !id.is_empty() {
        s.push('{');
        s.extend(id.chars().filter(char::is_ascii_alphanumeric).take(5));
    }
    s
}

/// An acknowledgement of `id` to `to`.
#[must_use]
pub fn encode_ack(to: &str, id: &str) -> String {
    format!(
        ":{}:ack{}",
        pad9(to),
        id.chars().filter(char::is_ascii_alphanumeric).take(5).collect::<String>()
    )
}

/// A rejection of `id` — "I got it and I am not taking it", which stops the
/// sender retrying where silence would not.
#[must_use]
pub fn encode_rej(to: &str, id: &str) -> String {
    format!(
        ":{}:rej{}",
        pad9(to),
        id.chars().filter(char::is_ascii_alphanumeric).take(5).collect::<String>()
    )
}

/// A status report: one line about the station itself.
#[must_use]
pub fn encode_status(text: &str) -> String {
    format!(">{}", clean(text).chars().take(62).collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AprsData, parse as parse_info};

    #[test]
    fn a_message_with_an_identifier_splits_into_its_parts() {
        let AprsData::Message(m) = parse_info("APRS", b":OE3JJS   :Hello there{42").unwrap() else {
            panic!("not a message")
        };
        assert_eq!(m.addressee, "OE3JJS");
        assert_eq!(m.text, "Hello there");
        assert_eq!(m.id, "42");
        assert_eq!(m.kind, MessageKind::Text);
    }

    /// The addressee is a fixed nine-wide field. Splitting on the first colon
    /// instead breaks every message that contains one — which is any message
    /// mentioning a time.
    #[test]
    fn a_colon_in_the_text_does_not_split_the_addressee() {
        let AprsData::Message(m) = parse_info("APRS", b":OE3JJS   :net at 19:30{7").unwrap() else {
            panic!("not a message")
        };
        assert_eq!(m.addressee, "OE3JJS");
        assert_eq!(m.text, "net at 19:30");
        assert_eq!(m.id, "7");
    }

    #[test]
    fn an_ack_and_a_rej_are_not_messages() {
        let AprsData::Message(a) = parse_info("APRS", b":OE3JJS   :ack42").unwrap() else {
            panic!()
        };
        assert_eq!(a.kind, MessageKind::Ack);
        assert_eq!(a.id, "42");
        let AprsData::Message(r) = parse_info("APRS", b":OE3JJS   :rej42").unwrap() else {
            panic!()
        };
        assert_eq!(r.kind, MessageKind::Rej);
    }

    /// "acknowledged" begins with `ack` and is a message, not an
    /// acknowledgement of a station called "nowledged".
    #[test]
    fn a_message_beginning_with_ack_is_still_a_message() {
        let AprsData::Message(m) = parse_info("APRS", b":OE3JJS   :acknowledged, thanks").unwrap()
        else {
            panic!()
        };
        assert_eq!(m.kind, MessageKind::Text);
        assert_eq!(m.text, "acknowledged, thanks");
    }

    /// Reply-ack folds an acknowledgement into the reply and saves a whole
    /// transmission on a channel where that is the scarce thing.
    #[test]
    fn a_reply_ack_carries_both_identifiers() {
        let AprsData::Message(m) = parse_info("APRS", b":OE3JJS   :on my way{ab}cd").unwrap()
        else {
            panic!()
        };
        assert_eq!(m.text, "on my way");
        assert_eq!(m.id, "ab");
        assert_eq!(m.reply_ack, "cd");
    }

    #[test]
    fn a_bulletin_is_recognised_by_its_addressee() {
        let AprsData::Message(m) = parse_info("APRS", b":BLN1     :Club net Tuesday").unwrap()
        else {
            panic!()
        };
        assert_eq!(m.kind, MessageKind::Bulletin);
    }

    /// The encoder pads to nine and round-trips through the parser, which is
    /// the pair that actually has to agree.
    #[test]
    fn an_encoded_message_reads_back_as_itself() {
        let f = encode_message("oe3jjs", "test message", "01");
        assert_eq!(f, ":OE3JJS   :test message{01}".trim_end_matches('}'));
        let AprsData::Message(m) = parse_info("APRS", f.as_bytes()).unwrap() else { panic!() };
        assert_eq!(m.addressee, "OE3JJS");
        assert_eq!(m.text, "test message");
        assert_eq!(m.id, "01");
    }

    /// A `{` typed into the message box would look like the start of an
    /// identifier and truncate everything after it.
    #[test]
    fn a_brace_in_the_text_is_dropped_rather_than_truncating_the_message() {
        let f = encode_message("N0CALL", "a{b", "1");
        let AprsData::Message(m) = parse_info("APRS", f.as_bytes()).unwrap() else { panic!() };
        assert_eq!(m.text, "ab");
        assert_eq!(m.id, "1");
    }

    /// A nine-character callsign is the longest there is, and it must not be
    /// padded past the field.
    #[test]
    fn a_full_width_addressee_is_not_padded() {
        let f = encode_message("VK2ABC-15", "hi", "");
        assert_eq!(&f[..11], ":VK2ABC-15:");
    }
}
