//! Writing received audio into a playback ring without letting it fall behind.
//!
//! Every radio's receive audio ends in a ring the sound card drains in real
//! time. A producer that runs ahead fills that ring, and the ring plays every
//! sample it holds, so the operator hears the audio as far behind the picture
//! as the ring is deep. This module is the one place that decides how much may
//! be queued, for every path that writes to a speaker.

/// The most playout a ring may hold before new audio is held back, in
/// interleaved stereo samples: 250 ms at 48 kHz.
///
/// The output ring is a second deep. Filling it to the top plays a second of
/// stale audio. Capping the backlog bounds that delay at a quarter of a second.
/// What is held back is the newest audio, because the older audio is the part
/// already queued ahead of it.
pub const PLAYOUT_BACKLOG: usize = 24_000;

/// Write up to `frames` stereo frames into `producer`, taking each frame from
/// `frame(i)` and stopping once the ring already holds `max_backlog` samples.
///
/// Whole frames only. A frame is written as a left and right pair, and is taken
/// only when both halves fit, so a full ring never leaves a lone channel behind
/// — which would swap left and right from that point on.
///
/// Returns how many frames were taken. The rest were not played.
pub fn queue_frames(
    producer: &mut rtrb::Producer<f32>,
    frames: usize,
    max_backlog: usize,
    mut frame: impl FnMut(usize) -> (f32, f32),
) -> usize {
    let capacity = producer.buffer().capacity();
    let mut taken = 0;
    while taken < frames {
        let backlog = capacity - producer.slots();
        if backlog + 2 > max_backlog || producer.slots() < 2 {
            break;
        }
        let (l, r) = frame(taken);
        let _ = producer.push(l);
        let _ = producer.push(r);
        taken += 1;
    }
    taken
}

#[cfg(test)]
mod tests {
    use super::{PLAYOUT_BACKLOG, queue_frames};

    fn ring(slots: usize) -> (rtrb::Producer<f32>, rtrb::Consumer<f32>) {
        rtrb::RingBuffer::<f32>::new(slots)
    }

    /// Left and right come from the callback independently, so a frame that is
    /// not mono reaches the two channels as given.
    #[test]
    fn each_channel_is_written_from_its_own_source() {
        let (mut p, mut c) = ring(96_000);
        let left = [0.1f32, 0.2];
        let right = [0.9f32, 0.8];
        let taken = queue_frames(&mut p, 2, PLAYOUT_BACKLOG, |i| (left[i], right[i]));
        assert_eq!(taken, 2);
        let mut out = Vec::new();
        while let Ok(s) = c.pop() {
            out.push(s);
        }
        assert_eq!(out, vec![0.1, 0.9, 0.2, 0.8]);
    }

    /// Five slots hold two whole frames. The third frame has room for one
    /// channel only and must not be started.
    #[test]
    fn a_frame_that_does_not_fit_is_not_started() {
        let (mut p, _c) = ring(5);
        assert_eq!(queue_frames(&mut p, 3, usize::MAX, |_| (1.0, 2.0)), 2);
        assert_eq!(p.slots(), 1);
    }

    /// The backlog stays at the cap however many frames are offered.
    #[test]
    fn the_backlog_is_held_at_the_cap() {
        let (mut p, _c) = ring(96_000);
        let taken = queue_frames(&mut p, 48_000, PLAYOUT_BACKLOG, |_| (0.1, 0.1));
        let backlog = 96_000 - p.slots();
        assert!(backlog <= PLAYOUT_BACKLOG);
        assert!(backlog + 2 > PLAYOUT_BACKLOG, "the cap was not used");
        assert_eq!(taken * 2, backlog);
    }
}
