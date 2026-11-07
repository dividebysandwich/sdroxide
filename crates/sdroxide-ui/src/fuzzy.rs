//! A small fuzzy matcher for the search-as-you-type fields.
//!
//! Subsequence matching in the style of `fzf`: the query characters have to
//! appear in order but not adjacently, so `bbcws` finds "BBC World Service" and
//! `okee` finds "Okeechobee, FL". Scored rather than boolean so a list can be
//! ranked by how well each row matched instead of staying in its natural order.
//!
//! Hand-rolled because the tree carries no fuzzy-matching crate and this is
//! forty lines; it also has to compile for wasm32, like the rest of the UI.

/// A match at the start of a word is worth the most.
const BOUNDARY_BONUS: i32 = 16;
/// A match immediately after the previous one is worth nearly as much: it means
/// the query is a contiguous run rather than scattered letters.
const RUN_BONUS: i32 = 10;
/// An isolated match still counts for something.
const MATCH_BONUS: i32 = 2;
/// Each haystack character stepped over between matches costs a little, so a
/// tight match beats a loose one.
const SKIP_PENALTY: i32 = 1;

fn is_boundary(c: u8) -> bool {
    matches!(c, b' ' | b'-' | b'_' | b'/' | b'.' | b',' | b'(' | b')' | b':' | b'\t')
}

/// Score one term against `haystack`, or `None` if the term is not a
/// subsequence of it. Higher is better. An empty term scores 0 and matches.
///
/// ASCII-case-insensitive, matching the rest of the codebase's callsign and
/// station handling; non-ASCII bytes still compare exactly, so an accented
/// station name is findable by typing it.
pub fn score(haystack: &str, term: &str) -> Option<i32> {
    if term.is_empty() {
        return Some(0);
    }
    let h = haystack.as_bytes();
    let n = term.as_bytes();
    let mut total = 0;
    let mut ni = 0;
    let mut prev_match: Option<usize> = None;
    for (hi, &hc) in h.iter().enumerate() {
        if ni == n.len() {
            break;
        }
        if !hc.eq_ignore_ascii_case(&n[ni]) {
            continue;
        }
        let prev_idx = hi.checked_sub(1);
        let at_boundary = prev_idx.is_none_or(|i| is_boundary(h[i]));
        let continues_run = prev_idx.is_some() && prev_match == prev_idx;
        total += if at_boundary {
            BOUNDARY_BONUS
        } else if continues_run {
            RUN_BONUS
        } else {
            MATCH_BONUS
        };
        if let Some(p) = prev_match {
            total -= (hi - p - 1) as i32 * SKIP_PENALTY;
        }
        prev_match = Some(hi);
        ni += 1;
    }
    if ni < n.len() {
        return None;
    }
    // Prefer the shorter of two otherwise equal matches, so a query that is
    // nearly the whole name outranks the same letters buried in a long one.
    Some(total - (h.len() as i32) / 8)
}

/// Whether every whitespace-separated term of `query` appears in `haystack` as
/// a plain, contiguous substring — a literal match rather than a scattered
/// subsequence one.
///
/// Subsequence matching is generous by design, and on a long haystack it is
/// generous to a fault: a callsign typed into a list of eleven hundred
/// receiver names is a subsequence of dozens of them by accident, which is not
/// what somebody who typed a callsign wanted. Callers use this to keep the
/// literal matches when there are any and fall back to the fuzzy ones only
/// when there are none. An all-whitespace query has nothing to fail, so it is
/// literally matched by everything.
pub fn contains_terms(haystack: &str, query: &str) -> bool {
    query.split_whitespace().all(|term| contains(haystack, term))
}

/// One term, ASCII-case-insensitively, as a contiguous run.
fn contains(haystack: &str, term: &str) -> bool {
    let (h, n) = (haystack.as_bytes(), term.as_bytes());
    n.len() <= h.len() && h.windows(n.len()).any(|w| w.eq_ignore_ascii_case(n))
}

/// Whether every whitespace-separated term of `query` appears in `haystack` as
/// a *tight* subsequence: all its characters, in order, inside a window at
/// most a few characters longer than the term itself.
///
/// This is the fallback test for a long haystack when nothing matched
/// literally. A loose subsequence is no evidence there: the six letters of
/// `twente` fall in that order somewhere in hundreds of receiver descriptions
/// by accident, and offering those as near misses filled the public-SDR list
/// with rows that have nothing to do with the query. A tight window keeps
/// what a near miss actually looks like — a dropped or swapped-out letter
/// (`hambrg` → "Hamburg", `nethrlands` → "Netherlands") — and nothing else.
/// The slack grows with the term (a third of its length, at least two), so a
/// long word may carry more than one typo.
pub fn tight_terms(haystack: &str, query: &str) -> bool {
    query.split_whitespace().all(|term| tight(haystack, term))
}

/// One term, ASCII-case-insensitively, as a subsequence inside a window of at
/// most `term.len() + slack` haystack characters.
fn tight(haystack: &str, term: &str) -> bool {
    let (h, n) = (haystack.as_bytes(), term.as_bytes());
    if n.is_empty() {
        return true;
    }
    let max_span = n.len() + (n.len() / 3).max(2);
    (0..h.len()).filter(|&start| h[start].eq_ignore_ascii_case(&n[0])).any(|start| {
        // Greedy from each start: the earliest completion from a given
        // start is also the shortest window beginning there.
        let end = (start + max_span).min(h.len());
        let mut ni = 1;
        for &hc in &h[start + 1..end] {
            if ni == n.len() {
                break;
            }
            if hc.eq_ignore_ascii_case(&n[ni]) {
                ni += 1;
            }
        }
        ni == n.len()
    })
}

/// Score a whole query: every whitespace-separated term has to match somewhere,
/// and the scores add up. That makes `bbc asc` find the BBC transmission from
/// Ascension without caring which order the two words appear in.
///
/// An all-whitespace query matches everything with score 0, which is what the
/// callers want for an empty search box.
pub fn score_terms(haystack: &str, query: &str) -> Option<i32> {
    let mut total = 0;
    for term in query.split_whitespace() {
        total += score(haystack, term)?;
    }
    Some(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_query_matches_everything() {
        assert_eq!(score_terms("BBC World Service", ""), Some(0));
        assert_eq!(score_terms("BBC World Service", "   "), Some(0));
    }

    #[test]
    fn a_substring_matches_case_insensitively() {
        assert!(score("BBC World Service", "world").is_some());
        assert!(score("BBC World Service", "WORLD").is_some());
        assert!(score("BBC World Service", "bbc").is_some());
    }

    #[test]
    fn a_scattered_subsequence_matches() {
        assert!(score("BBC World Service", "bbcws").is_some());
        assert!(score("Okeechobee, FL", "okfl").is_some());
    }

    #[test]
    fn a_non_subsequence_does_not_match() {
        assert_eq!(score("BBC World Service", "zzz"), None);
        // Right letters, wrong order.
        assert_eq!(score("BBC World Service", "dlrow"), None);
    }

    #[test]
    fn every_term_must_match() {
        assert!(score_terms("BBC World Service Ascension", "bbc asc").is_some());
        assert!(score_terms("BBC World Service Ascension", "bbc nowhere").is_none());
        // Order between terms does not matter.
        assert!(score_terms("BBC World Service Ascension", "asc bbc").is_some());
    }

    #[test]
    fn a_word_start_outranks_a_letter_in_the_middle() {
        let boundary = score("BBC World Service", "w").unwrap();
        let middle = score("Newton", "w").unwrap();
        assert!(boundary > middle, "{boundary} should beat {middle}");
    }

    #[test]
    fn the_intended_station_outranks_an_accidental_subsequence() {
        // "bbc" is a real prefix of the first and only a scattered accident in
        // the second, so the ranking must put the BBC first.
        let intended = score("BBC World Service", "bbc").unwrap();
        let accident = score("Bangladesh Betar Cabirpur", "bbc").unwrap();
        assert!(intended > accident, "{intended} should beat {accident}");
    }

    #[test]
    fn a_tight_run_outranks_a_loose_one() {
        let tight = score("Radio Nikkei", "nikkei").unwrap();
        let loose = score("Radio Nacional Inconfidencia Kilo Kilo Echo India", "nikkei").unwrap();
        assert!(tight > loose, "{tight} should beat {loose}");
    }

    #[test]
    fn a_literal_match_is_told_apart_from_a_scattered_one() {
        // Both of these are subsequence matches for the callsign...
        assert!(score_terms("KiwiSDR DL1ABC Hamburg", "dl1abc").is_some());
        assert!(score_terms("Dresden L-antenna 1 Ansbach Bad Camberg", "dl1abc").is_some());
        // ...and only the first is what somebody typing a callsign meant.
        assert!(contains_terms("KiwiSDR DL1ABC Hamburg", "dl1abc"));
        assert!(!contains_terms("Dresden L-antenna 1 Ansbach Bad Camberg", "dl1abc"));
    }

    #[test]
    fn every_term_has_to_be_there_literally() {
        assert!(contains_terms("BBC World Service Ascension", "bbc asc"));
        assert!(contains_terms("BBC World Service Ascension", "asc bbc"));
        assert!(!contains_terms("BBC World Service Ascension", "bbc nowhere"));
        // An empty box has nothing to fail on.
        assert!(contains_terms("BBC World Service", ""));
        assert!(contains_terms("BBC World Service", "   "));
        // A term longer than the haystack cannot be in it.
        assert!(!contains_terms("BBC", "BBC World Service"));
    }

    #[test]
    fn a_typo_is_a_tight_match_and_scattered_letters_are_not() {
        assert!(tight_terms("KiwiSDR DF3LZ | Hamburg, Germany", "hambrg"));
        assert!(tight_terms("De Kiel, Netherlands", "nethrlands"));
        assert!(tight_terms("R7KJG SDR Rostov-on-Don", "rstv"));
        // A subsequence, but spread across two words: a near miss of nothing.
        let hay = "KiwiSDR 0-30 MHz | South Wenatchee, WA";
        assert!(score_terms(hay, "twente").is_some());
        assert!(!tight_terms(hay, "twente"));
        // Every term has to be tight, and an empty query is.
        assert!(!tight_terms("Hamburg, Germany", "hambrg nowhere"));
        assert!(tight_terms("Hamburg, Germany", "  "));
    }

    #[test]
    fn a_frequency_is_findable_as_typed() {
        // The callers fold both renderings of the frequency into the haystack.
        let hay = "voice of greece avlis greece 9420 9.420";
        assert!(score_terms(hay, "9420").is_some());
        assert!(score_terms(hay, "9.420").is_some());
        assert!(score_terms(hay, "avlis").is_some());
    }
}
