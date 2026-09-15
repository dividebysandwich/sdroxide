//! A rolling window of the last couple of minutes of audio, for instant replay.
//!
//! A shortwave listener misses things — a station identification, a frequency
//! the announcer reads out, the start of a programme while tuning past it — and
//! wants to hear *that* again. This keeps the recent past so they can.
//!
//! The model is a delay line rather than a recorder: the read pointer trails
//! the write pointer by the whole window, so while replay is on the listener
//! hears the last hours-worth going forward, two minutes behind live. Stop, and
//! it snaps back to now. That is exactly a DVR, and it is the least surprising
//! thing to build out of one buffer.
//!
//! Mono on purpose: the ring is fed the speaker's summed channel. A stereo
//! replay would need both channels kept in step, and the left sum is what a
//! listener replays a faded broadcast for anyway.

/// A fixed-capacity ring of audio samples with a write head and an optional
/// read head trailing it.
pub struct ReplayBuffer {
    buf: Vec<f32>,
    write: usize,
    /// Samples held, up to the capacity. Below capacity the window is the whole
    /// buffer, from index 0.
    filled: usize,
    read: Option<usize>,
    on: bool,
}

impl ReplayBuffer {
    /// `cap` samples — `audio_rate * seconds`. At 48 kHz, two minutes is
    /// 5.76 M samples, about 23 MB.
    pub fn new(cap: usize) -> Self {
        ReplayBuffer {
            buf: vec![0.0; cap.max(1)],
            write: 0,
            filled: 0,
            read: None,
            on: false,
        }
    }

    pub fn on(&self) -> bool {
        self.on
    }

    /// Turn replay on or off. Off drops the read head, so the next replay
    /// starts again from the oldest sample in the window rather than wherever
    /// the last one stopped.
    pub fn set_on(&mut self, on: bool) {
        self.on = on;
        if !on {
            self.read = None;
        }
    }

    /// The window length in samples.
    pub fn capacity(&self) -> usize {
        self.buf.len()
    }

    /// Push the live block, overwriting the oldest samples when full.
    pub fn push(&mut self, samples: &[f32]) {
        let cap = self.buf.len();
        // A block longer than the window leaves only its tail, which is what
        // the ring would hold anyway — do it in one copy rather than spinning.
        let tail = if samples.len() > cap { &samples[samples.len() - cap..] } else { samples };
        for &s in tail {
            self.buf[self.write] = s;
            self.write = (self.write + 1) % cap;
        }
        self.filled = (self.filled + tail.len()).min(cap);
    }

    /// Append `len` replayed samples to `out` (which is cleared first). One
    /// sample comes out per sample that has gone in since the last call, so the
    /// read head trails the write head by the window.
    pub fn read_into(&mut self, out: &mut Vec<f32>, len: usize) {
        out.clear();
        if self.filled == 0 || len == 0 {
            return;
        }
        let cap = self.buf.len();
        let mut r = self.read.unwrap_or_else(|| (self.write + cap - self.filled) % cap);
        out.reserve(len);
        for _ in 0..len {
            out.push(self.buf[r]);
            r = (r + 1) % cap;
        }
        self.read = Some(r);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_short_buffer_replays_what_went_in() {
        let mut r = ReplayBuffer::new(64);
        r.push(&(0..10).map(|i| i as f32).collect::<Vec<_>>());
        let mut out = Vec::new();
        r.read_into(&mut out, 10);
        assert_eq!(out, (0..10).map(|i| i as f32).collect::<Vec<_>>());
    }

    #[test]
    fn the_window_keeps_the_most_recent_samples() {
        let mut r = ReplayBuffer::new(5);
        r.push(&(1..=8).map(|i| i as f32).collect::<Vec<_>>());
        let mut out = Vec::new();
        r.read_into(&mut out, 5);
        assert_eq!(out, vec![4.0, 5.0, 6.0, 7.0, 8.0], "the last five of eight");
    }

    #[test]
    fn the_read_head_trails_the_write_by_the_window() {
        let mut r = ReplayBuffer::new(4);
        r.push(&[1.0, 2.0, 3.0, 4.0]);
        // The first replay block reads the whole window.
        let mut a = Vec::new();
        r.read_into(&mut a, 4);
        assert_eq!(a, vec![1.0, 2.0, 3.0, 4.0]);
        // Push the next live block; the read head is one block behind, so it
        // now returns that block, not the oldest samples again.
        r.push(&[5.0, 6.0, 7.0, 8.0]);
        let mut b = Vec::new();
        r.read_into(&mut b, 4);
        assert_eq!(b, vec![5.0, 6.0, 7.0, 8.0]);
    }

    #[test]
    fn switching_off_returns_the_next_replay_to_the_oldest() {
        let mut r = ReplayBuffer::new(4);
        r.push(&[1.0, 2.0, 3.0, 4.0]);
        let mut a = Vec::new();
        r.read_into(&mut a, 2);
        r.set_on(true);
        r.set_on(false);
        let mut b = Vec::new();
        r.read_into(&mut b, 2);
        assert_eq!(b, vec![1.0, 2.0], "replay starts over at the oldest sample");
    }

    #[test]
    fn an_empty_buffer_replays_nothing() {
        let mut r = ReplayBuffer::new(8);
        let mut out = vec![9.0];
        r.read_into(&mut out, 4);
        assert!(out.is_empty());
    }
}
