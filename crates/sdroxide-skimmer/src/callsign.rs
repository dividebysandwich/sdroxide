//! Extract a plausible amateur callsign from a rolling stream of decoded text.
//! Hand-rolled (no regex dep): validate each whitespace/`/`-delimited token
//! against the general callsign shape (prefix + call-area digit + 1–4 letters).

/// The most recently seen callsign-shaped token in `text`, if any. Returning
/// the *last* match tracks the current transmitting station (e.g. the answerer
/// in "W1AW DE K3LR", or the caller in "CQ DE W1AW").
pub fn find_callsign(text: &str) -> Option<String> {
    text.split(|c: char| c.is_whitespace() || c == '/')
        .filter(|tok| looks_like_call(tok))
        .last()
        .map(|s| s.to_string())
}

/// True if `tok` matches the general amateur-callsign shape:
/// a 1–2 char prefix containing a letter, a call-area digit, then 1–4 letters.
fn looks_like_call(tok: &str) -> bool {
    if tok.len() < 3 || tok.len() > 6 {
        return false;
    }
    if !tok.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit()) {
        return false;
    }
    // Suffix = trailing letters (1–4).
    let suffix_len = tok.chars().rev().take_while(|c| c.is_ascii_alphabetic()).count();
    if !(1..=4).contains(&suffix_len) {
        return false;
    }
    let head = &tok[..tok.len() - suffix_len]; // prefix + call-area digit
    if !head.ends_with(|c: char| c.is_ascii_digit()) {
        return false;
    }
    let prefix = &head[..head.len() - 1];
    if prefix.is_empty() || prefix.len() > 2 {
        return false;
    }
    // A real prefix has at least one letter (rejects "599", "5NN", RST cut numbers).
    prefix.bytes().any(|b| b.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_common_calls() {
        assert_eq!(find_callsign("CQ CQ DE W1AW W1AW K").as_deref(), Some("W1AW"));
        assert_eq!(find_callsign("W1AW DE K3LR K").as_deref(), Some("K3LR"));
        assert_eq!(find_callsign("TNX QSO OE3JJS/P 73").as_deref(), Some("OE3JJS"));
        assert_eq!(find_callsign("UP2 9A1A TEST").as_deref(), Some("9A1A"));
        assert_eq!(find_callsign("PY2ABC").as_deref(), Some("PY2ABC"));
    }

    #[test]
    fn rejects_non_calls() {
        assert_eq!(find_callsign("599 TU 73 GL"), None);
        assert_eq!(find_callsign("CQ CQ TEST DE"), None);
        assert_eq!(find_callsign("RANDOM NOISE QRZ"), None);
        assert_eq!(find_callsign(""), None);
    }
}
