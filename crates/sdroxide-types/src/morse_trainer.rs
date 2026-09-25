//! The Morse trainer's curriculum and progress.
//!
//! Learning Morse by ear works best the way Ludwig Koch taught it: start with
//! a few characters at full character speed but wide spacing, and add the next
//! character only once the ones already known are copied reliably. This module
//! is the pure half of that — the character order, what an answer counts as,
//! and the progress remembered between sessions. The audio, the clock and the
//! random choice live in the UI.

use serde::{Deserialize, Serialize};

/// The Koch character order, as used by the common trainers.
///
/// Easiest-to-distinguish first (K and M are hard to confuse, E and T are not
/// adjacent), with the digits and punctuation folded in where they are usually
/// introduced. The unforgiving opposites — `E`/`I`/`S`/`H` on one hand and
/// `T`/`M`/`O` on the other — are deliberately far apart.
pub const KOCH_ORDER: [char; 40] = [
    'K', 'M', 'R', 'S', 'U', 'A', 'P', 'T', 'L', 'O', 'W', 'I', '.', 'N', 'J', 'E', 'F', '0', 'Y',
    'V', ',', 'G', '5', '/', 'Q', '9', 'Z', 'H', '3', '8', 'B', '?', '4', '2', '7', 'C', '1', 'D',
    '6', 'X',
];

/// How many characters the set starts with: enough for two different sounds,
/// few enough that a first session is not discouraging.
pub const START_UNLOCKED: u8 = 2;

/// A run of correct answers against the *newest* character that promotes it.
///
/// The classic drill: when the character just added has been copied this many
/// times in a row, add the next. A wrong answer anywhere resets the run.
pub const ADVANCE_RUN: u32 = 10;

/// What one answer did to the progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    /// The character was copied correctly and nothing new was unlocked.
    Correct,
    /// The character was copied correctly and the run promoted a new one.
    Promoted(char),
    /// The answer was wrong (or unparseable). The run resets.
    Wrong,
}

/// The trainer's remembered state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MorseProgress {
    /// How many of [`KOCH_ORDER`] are unlocked. Never below [`START_UNLOCKED`].
    pub unlocked: u8,
    /// Correct answers, ever.
    pub correct: u32,
    /// Answers given, ever.
    pub total: u32,
    /// Correct answers in a row; reset by a wrong one.
    pub streak: u32,
}

impl Default for MorseProgress {
    fn default() -> Self {
        MorseProgress { unlocked: START_UNLOCKED, correct: 0, total: 0, streak: 0 }
    }
}

impl MorseProgress {
    /// The characters currently in play.
    pub fn set(&self) -> &'static [char] {
        let n = (self.unlocked as usize).clamp(START_UNLOCKED as usize, KOCH_ORDER.len());
        &KOCH_ORDER[..n]
    }

    /// The character the run is against — the newest one unlocked, which is
    /// the one whose [`ADVANCE_RUN`] promotes the next.
    pub fn newest(&self) -> char {
        *self.set().last().unwrap_or(&'K')
    }

    /// Whether every character is unlocked.
    pub fn complete(&self) -> bool {
        self.unlocked as usize >= KOCH_ORDER.len()
    }

    /// Record one answer. `got` is what the operator typed; `want` is the
    /// character that was played. Only the first character of `got` is read,
    /// and case is ignored, so a whole word typed into the box still answers
    /// its first letter rather than counting once per character.
    pub fn answer(&mut self, want: char, got: &str) -> Answer {
        let got = got.trim().chars().next().map(|c| c.to_ascii_uppercase());
        let want = want.to_ascii_uppercase();
        if got == Some(want) {
            self.correct += 1;
            self.total += 1;
            self.streak += 1;
            // Only the newest character's run advances the set; answers that
            // were right but for an older character keep the streak going and
            // promote nothing. A hard set that has no next character is done.
            if self.streak >= ADVANCE_RUN && !self.complete() {
                self.unlocked += 1;
                self.streak = 0;
                return Answer::Promoted(self.newest());
            }
            Answer::Correct
        } else {
            self.total += 1;
            self.streak = 0;
            Answer::Wrong
        }
    }

    /// Start again: back to [`START_UNLOCKED`], scores cleared.
    pub fn reset(&mut self) {
        *self = MorseProgress::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_order_starts_with_two_and_has_no_repeats() {
        let set: std::collections::HashSet<char> = KOCH_ORDER.iter().copied().collect();
        assert_eq!(set.len(), KOCH_ORDER.len(), "a repeated character would skew the drill");
        assert_eq!(MorseProgress::default().set(), &['K', 'M']);
    }

    #[test]
    fn a_wrong_answer_resets_the_run_but_not_the_set() {
        let mut p = MorseProgress::default();
        for _ in 0..ADVANCE_RUN - 1 {
            assert_eq!(p.answer('K', "k"), Answer::Correct);
        }
        assert_eq!(p.unlocked, START_UNLOCKED);
        assert_eq!(p.answer('K', "m"), Answer::Wrong, "a wrong answer is wrong");
        assert_eq!(p.streak, 0);
        assert_eq!(p.unlocked, START_UNLOCKED);
        assert_eq!(p.total, ADVANCE_RUN);
        assert_eq!(p.correct, ADVANCE_RUN - 1);
    }

    #[test]
    fn a_run_of_ten_promotes_the_next_character() {
        let mut p = MorseProgress::default();
        for _ in 0..ADVANCE_RUN - 1 {
            p.answer('M', "m");
        }
        assert_eq!(p.answer('M', "m"), Answer::Promoted('R'));
        assert_eq!(p.unlocked, START_UNLOCKED + 1);
        assert_eq!(p.set(), &['K', 'M', 'R']);
        assert_eq!(p.streak, 0, "the run starts over against the new character");
    }

    #[test]
    fn an_empty_or_unknown_answer_is_wrong() {
        let mut p = MorseProgress::default();
        assert_eq!(p.answer('K', ""), Answer::Wrong);
        assert_eq!(p.answer('K', "   "), Answer::Wrong);
        assert_eq!(p.answer('K', "?!"), Answer::Wrong);
    }

    #[test]
    fn a_full_set_never_promotes_again() {
        let mut p = MorseProgress { unlocked: KOCH_ORDER.len() as u8, ..Default::default() };
        for _ in 0..ADVANCE_RUN {
            p.answer('X', "x");
        }
        assert!(p.complete());
        assert_eq!(p.unlocked as usize, KOCH_ORDER.len(), "nothing past the end");
    }

    #[test]
    fn reset_clears_everything() {
        let mut p = MorseProgress { unlocked: 12, correct: 40, total: 55, streak: 3 };
        p.reset();
        assert_eq!(p, MorseProgress::default());
    }
}
