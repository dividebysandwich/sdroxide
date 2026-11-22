//! The labels TCI clients put on this radio's panorama with `SPOT:`.
//!
//! One book per server, shared by all its clients, because the specification
//! has one panorama: `SPOT_DELETE:<call>` names a callsign, not a client, and
//! `SPOT_CLEAR` "deletes all spots from panorama". The book is the hub's; the
//! engine is handed a fresh copy whenever it changes, and passes it to the
//! waterfall as [`SpotKind::Tci`] spots. Pure data with the clock passed in,
//! so every rule here is unit-testable.
//!
//! **Lifetime.** A spot lives [`TCI_SPOT_LIFETIME_SECS`] (ten minutes, as the
//! spec says) from the `SPOT:` that last placed it. A client that disconnects
//! leaves its spots up to run out that time: nothing in the spec ties a spot
//! to its connection, and a logger or cluster bridge that reconnects after a
//! network blip would otherwise blank the waterfall and leave it blank until
//! it next re-sends — which for a needed-station marker can be many minutes.
//! The spots go when the server itself stops, because then nobody can delete
//! them any more.

use std::collections::HashMap;

use sdroxide_types::{Spot, SpotKind, TCI_SPOT_LIFETIME_SECS};

use super::text::TciSpot;

/// Most spots the book holds. A skimmer bridge pushing a contest weekend's
/// worth of spots would otherwise grow the book without bound for the ten
/// minutes each one lives, and every change ships the whole book to the
/// waterfall. Past this the oldest spot gives way to the new one.
pub(crate) const MAX_SPOTS: usize = 1000;

#[derive(Default)]
pub(crate) struct SpotBook {
    /// Keyed by upper-case callsign: a callsign has one spot, and a new `SPOT:`
    /// for it replaces the old one wherever it was.
    by_call: HashMap<String, Spot>,
}

impl SpotBook {
    /// Place or replace `s`'s callsign's spot, stamped `now_utc`.
    pub fn put(&mut self, s: TciSpot, now_utc: i64) {
        if !self.by_call.contains_key(&s.call)
            && self.by_call.len() >= MAX_SPOTS
            && let Some(oldest) =
                self.by_call.values().min_by_key(|x| x.when_utc).map(|x| x.call.clone())
        {
            self.by_call.remove(&oldest);
        }
        let spot = Spot {
            id: Spot::make_id(SpotKind::Tci, &s.call, s.freq_hz),
            kind: SpotKind::Tci,
            freq_hz: s.freq_hz,
            call: s.call.clone(),
            spotter: String::new(),
            mode: s.mode,
            comment: s.text,
            reference: None,
            grid: None,
            loc: None,
            when_utc: now_utc,
            snr_db: None,
            color: s.color,
        };
        self.by_call.insert(s.call, spot);
    }

    /// Take `call`'s spot down. False when there was none.
    pub fn delete(&mut self, call: &str) -> bool {
        self.by_call.remove(call).is_some()
    }

    /// Take every spot down. False when there were none.
    pub fn clear(&mut self) -> bool {
        let had = !self.by_call.is_empty();
        self.by_call.clear();
        had
    }

    /// Drop the spots whose lifetime has run out by `now_utc`. True when any did.
    pub fn expire(&mut self, now_utc: i64) -> bool {
        let before = self.by_call.len();
        self.by_call.retain(|_, s| now_utc - s.when_utc < TCI_SPOT_LIFETIME_SECS);
        self.by_call.len() != before
    }

    /// Every live spot, in frequency order as the feed manager sends its own.
    pub fn snapshot(&self) -> Vec<Spot> {
        let mut v: Vec<Spot> = self.by_call.values().cloned().collect();
        v.sort_by(|a, b| a.freq_hz.total_cmp(&b.freq_hz).then_with(|| a.call.cmp(&b.call)));
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tci(call: &str, hz: f64) -> TciSpot {
        TciSpot {
            call: call.into(),
            mode: "CW".into(),
            freq_hz: hz,
            color: Some([255, 0, 0]),
            text: "needed".into(),
        }
    }

    #[test]
    fn a_spot_becomes_a_tci_label() {
        let mut b = SpotBook::default();
        b.put(tci("RN6LHF", 7_100_000.0), 1_000);
        let v = b.snapshot();
        assert_eq!(v.len(), 1);
        let s = &v[0];
        assert_eq!(s.kind, SpotKind::Tci);
        assert_eq!(
            (s.call.as_str(), s.mode.as_str(), s.comment.as_str()),
            ("RN6LHF", "CW", "needed")
        );
        assert_eq!(s.freq_hz, 7_100_000.0);
        assert_eq!(s.color, Some([255, 0, 0]));
        assert_eq!(s.when_utc, 1_000);
    }

    #[test]
    fn the_same_call_replaces_its_spot() {
        let mut b = SpotBook::default();
        b.put(tci("RN6LHF", 7_100_000.0), 1_000);
        b.put(tci("RN6LHF", 14_025_000.0), 1_100);
        let v = b.snapshot();
        assert_eq!(v.len(), 1, "one callsign, one label");
        assert_eq!(v[0].freq_hz, 14_025_000.0);
        assert_eq!(v[0].when_utc, 1_100, "a re-sent spot starts its lifetime again");
    }

    #[test]
    fn delete_and_clear() {
        let mut b = SpotBook::default();
        b.put(tci("RN6LHF", 7_100_000.0), 1_000);
        b.put(tci("DL1ABC", 7_010_000.0), 1_000);
        assert!(b.delete("RN6LHF"));
        assert!(!b.delete("RN6LHF"), "already gone");
        assert_eq!(b.snapshot().len(), 1);
        assert!(b.clear());
        assert!(!b.clear(), "nothing left to clear");
        assert!(b.snapshot().is_empty());
    }

    #[test]
    fn spots_live_ten_minutes() {
        let mut b = SpotBook::default();
        b.put(tci("RN6LHF", 7_100_000.0), 1_000);
        b.put(tci("DL1ABC", 7_010_000.0), 1_300);
        assert!(!b.expire(1_000 + TCI_SPOT_LIFETIME_SECS - 1));
        assert!(b.expire(1_000 + TCI_SPOT_LIFETIME_SECS), "RN6LHF's ten minutes are up");
        let v = b.snapshot();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].call, "DL1ABC");
    }

    #[test]
    fn a_full_book_gives_up_its_oldest_spot() {
        let mut b = SpotBook::default();
        for i in 0..MAX_SPOTS {
            b.put(tci(&format!("C{i}"), 7_000_000.0 + i as f64), 1_000 + i as i64);
        }
        b.put(tci("NEW", 7_500_000.0), 5_000);
        let v = b.snapshot();
        assert_eq!(v.len(), MAX_SPOTS);
        assert!(!v.iter().any(|s| s.call == "C0"), "the oldest went");
        assert!(v.iter().any(|s| s.call == "NEW"));
        // Replacing a call already in a full book evicts nobody.
        b.put(tci("C1", 7_600_000.0), 5_001);
        assert_eq!(b.snapshot().len(), MAX_SPOTS);
        assert!(b.snapshot().iter().any(|s| s.call == "C2"));
    }

    #[test]
    fn the_snapshot_is_in_frequency_order() {
        let mut b = SpotBook::default();
        b.put(tci("HIGH", 14_000_000.0), 1);
        b.put(tci("LOW", 7_000_000.0), 2);
        let calls: Vec<String> = b.snapshot().into_iter().map(|s| s.call).collect();
        assert_eq!(calls, ["LOW", "HIGH"]);
    }
}
