//! Which instant on the computer's clock an input sample arrived at.
//!
//! The engine hands audio over in blocks, and notes the computer's clock the
//! moment each block arrived from the receiver. The last sample of a block
//! arrived then *at the earliest*: anything the operating system or the engine
//! did between the receiver delivering it and the clock being read — another
//! thread scheduled first, a USB transfer completed late — only makes the
//! reading later, never earlier.
//!
//! So each block gives an upper bound on the clock reading of sample zero
//! (`epoch` here), and the smallest of the recent ones is the estimate. A
//! lower envelope is the right statistic for a delay that is never negative,
//! and on a desktop machine it is the difference between a measurement that
//! wanders by tens of milliseconds and one that does not.
//!
//! What it cannot see is a delay that is *always* there — the receiver's own
//! buffering and the receive filters. That is the latency the operator
//! calibrates once, in [`sdroxide_types::TimeSigConfig::latency_ms`].

use std::collections::VecDeque;

/// How far back the lower envelope reaches, seconds of input. Long enough to
/// contain some block that arrived promptly; short enough that a sample clock
/// fifty parts per million out drifts a quarter of a millisecond within it.
const WINDOW_S: f64 = 5.0;

pub struct SampleClock {
    rate: f64,
    /// (seconds of input at the block's end, clock reading of sample zero in
    /// nanoseconds that block implies).
    obs: VecDeque<(f64, i128)>,
}

impl SampleClock {
    pub fn new(rate: f64) -> Self {
        SampleClock { rate, obs: VecDeque::new() }
    }

    /// A block ending at input sample `end` (exclusive — the count of samples
    /// so far) arrived when the clock read `wall_ns`, nanoseconds since the
    /// Unix epoch.
    pub fn observe(&mut self, end: u64, wall_ns: i128) {
        let t = end as f64 / self.rate;
        let epoch = wall_ns - (t * 1e9) as i128;
        self.obs.push_back((t, epoch));
        while self.obs.front().is_some_and(|&(ot, _)| ot < t - WINDOW_S) {
            self.obs.pop_front();
        }
    }

    /// The clock reading, nanoseconds since the Unix epoch, at which input
    /// sample `pos` arrived — or `None` before any block has.
    pub fn wall_ns_at(&self, pos: f64) -> Option<i128> {
        let epoch = self.obs.iter().map(|&(_, e)| e).min()?;
        Some(epoch + (pos / self.rate * 1e9) as i128)
    }

    /// Forgets every observation: the computer's clock was just stepped, and
    /// the envelope from before it describes a clock that no longer exists.
    pub fn reset(&mut self) {
        self.obs.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn late_blocks_do_not_move_the_estimate() {
        let rate = 12_000.0;
        let mut c = SampleClock::new(rate);
        let base: i128 = 1_800_000_000_000_000_000;
        let mut x: u64 = 7;
        // 10 ms blocks, each noticed up to 30 ms late, and one in four prompt.
        for k in 1..=500u64 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            let late_ns = if k % 4 == 0 { 0 } else { (x % 30_000_000) as i128 };
            let end = k * 120;
            c.observe(end, base + (end as i128 * 1_000_000_000 / 12_000) + late_ns);
        }
        let at = c.wall_ns_at(55_000.0).unwrap();
        let truth = base + 55_000 * 1_000_000_000 / 12_000;
        assert!((at - truth).abs() < 10_000, "off by {} ns", at - truth);
    }

    #[test]
    fn nothing_is_claimed_before_the_first_block() {
        assert_eq!(SampleClock::new(48_000.0).wall_ns_at(0.0), None);
    }
}
