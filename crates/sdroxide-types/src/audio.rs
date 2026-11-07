//! Output-audio routing — which speaker channel the received audio goes to.
//!
//! A listener hearing on one side only (one earpiece, a single monitor speaker)
//! wants all of it on that side; the setting folds the stereo pair to mono on
//! the chosen channel at the output stage, so nothing else in the chain changes.
//! `Both` is the default and exactly the old behaviour.

use serde::{Deserialize, Serialize};

/// Which output channels the speaker audio reaches (#654).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioRouting {
    /// Left and right, as produced — the default.
    #[default]
    Both,
    /// Everything, mixed to mono, on the left channel; the right is silent.
    Left,
    /// Everything, mixed to mono, on the right channel; the left is silent.
    Right,
}

impl AudioRouting {
    /// Every choice, in the order a picker lists them.
    pub const ALL: [AudioRouting; 3] =
        [AudioRouting::Both, AudioRouting::Left, AudioRouting::Right];

    pub fn label(self) -> &'static str {
        match self {
            AudioRouting::Both => "Left + right",
            AudioRouting::Left => "Left only",
            AudioRouting::Right => "Right only",
        }
    }

    /// The interleaved `(left, right)` pair that actually reaches the card.
    /// One side gets the mono mix, so a sub receiver (panned right), WFM
    /// stereo or binaural audio is not lost to the silent ear.
    pub fn apply(self, left: f32, right: f32) -> (f32, f32) {
        let mono = 0.5 * (left + right);
        match self {
            AudioRouting::Both => (left, right),
            AudioRouting::Left => (mono, 0.0),
            AudioRouting::Right => (0.0, mono),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routing_folds_to_one_side() {
        assert_eq!(AudioRouting::Both.apply(0.5, -0.5), (0.5, -0.5));
        assert_eq!(AudioRouting::Left.apply(0.6, 0.2), (0.4, 0.0));
        assert_eq!(AudioRouting::Right.apply(0.6, 0.2), (0.0, 0.4));
        // Mono audio (left == right) passes at full level.
        assert_eq!(AudioRouting::Left.apply(0.5, 0.5), (0.5, 0.0));
        assert_eq!(AudioRouting::default(), AudioRouting::Both);
    }
}
