//! Converter headroom: how close the front end is running to its rails.
//!
//! Everything downstream of the discriminator is indifferent to signal level —
//! an FM demodulator is `arg()`, and the RDS receiver's carrier and timing
//! errors are both magnitude-normalised — so no amount of signal can overload
//! the software. The converter is another matter, and when it clips, what the
//! operator loses first is the thing they are least likely to notice.
//!
//! Measured on a broadcast station driven progressively harder into a ±1.0
//! converter (1.92 Msps, the station 480 kHz off the LO):
//!
//! | drive | at full scale | audio noise | RDS |
//! |------:|--------------:|------------:|:----|
//! | ≤1.0× | 0 % | −104 dB | clean |
//! | 1.4× | 99 % | −51 dB | clean |
//! | 1.6× | 100 % | −47 dB | 7 % of blocks bad |
//! | 1.8× | 100 % | −45 dB | sync lost |
//! | 2.0× | 100 % | −43 dB | no station name |
//!
//! Two things make this worth a meter of its own. The cliff is **about one dB
//! wide** — 1.6× to 1.8× — so a fade or a passing car walks a station across it
//! and RDS flickers on and off. And the audio never tells: the pilot stayed
//! locked and the stereo blend full at every drive in that table, with in-band
//! noise still 45 dB down where the data was already dead. FM's noise is
//! triangular, rising 6 dB per octave, so at 57 kHz it is some 15 dB worse than
//! where the programme lives, and the subcarrier sitting 28–34 dB below peak
//! deviation in 2.4 kHz of bandwidth is always the first casualty. The station
//! sounds perfect while its data is gone, which is exactly the report that
//! reaches the issue tracker as "RDS fails on strong signals".
//!
//! So the reading is published whether or not anything is decoding: it is the
//! one number that says the answer is *turn the gain down*.

use crate::Complex32;

/// Amplitude counted as "at full scale", −0.18 dBFS.
///
/// Deliberately a shade under 1.0, because the backends do not agree on what
/// full scale converts to: the RTL-SDR's byte table tops out at 0.99688, a
/// packed 8-bit front end at 0.9922, a 16-bit one at 0.99997. A threshold that
/// demanded 1.0 would report nothing at all on most of them.
///
/// The cost of the margin is that a signal which merely *reaches* the last
/// fraction of a dB without clipping is counted too. That is why the field this
/// feeds is a headroom indicator rather than a clipping detector, and why it is
/// published beside the peak instead of on its own — see [`AdcMeter::read`].
const FULL_SCALE: f32 = 0.98;

/// Peak and full-scale count over the samples since the last read.
///
/// Both arms deliberately measure the *raw* device samples, before the noise
/// blanker and before decimation: a blanker replaces the impulse that clipping
/// looks like, and a decimating filter averages a rail down into something with
/// headroom to spare. By the time either has run, the evidence is gone.
#[derive(Debug, Clone, Copy, Default)]
pub struct AdcMeter {
    /// What a converter sample at full scale arrives as, after whatever scale
    /// the backend puts on its samples. 1.0 unless told otherwise.
    full_scale: f32,
    peak: f32,
    at_scale: u64,
    seen: u64,
    /// The last reading with samples behind it, repeated while none arrive.
    last: (f32, f32),
}

impl AdcMeter {
    pub fn new() -> Self {
        AdcMeter {
            full_scale: 1.0,
            peak: 0.0,
            at_scale: 0,
            seen: 0,
            last: (f32::NEG_INFINITY, 0.0),
        }
    }

    /// Tell the meter what a full-scale converter sample arrives as.
    ///
    /// A backend that calibrates its samples to dBm (an ELAD multiplies every
    /// sample by its EEPROM scale, +12 to +23 dB on an FDM-DUO) delivers a
    /// full-scale sample well above 1.0, and one well below the rails already
    /// at 1.0. Measured against 1.0 that lit OVL with headroom to spare.
    /// Non-finite or non-positive values are ignored.
    pub fn set_full_scale(&mut self, full_scale: f32) {
        if full_scale.is_finite() && full_scale > 0.0 {
            self.full_scale = full_scale;
        }
    }

    /// Take in one block of converter samples.
    pub fn observe(&mut self, iq: &[Complex32]) {
        for z in iq {
            // Per component, not on the envelope: the converter clips I and Q
            // separately, and a complex sample whose magnitude is inside the
            // circle can still have had one of its two axes flattened.
            let m = z.re.abs().max(z.im.abs());
            if m > self.peak {
                self.peak = m;
            }
            if m >= FULL_SCALE * self.full_scale {
                self.at_scale += 1;
            }
        }
        self.seen += iq.len() as u64;
    }

    /// Peak in dBFS and the fraction of samples at full scale, over everything
    /// observed since the previous call.
    ///
    /// A call with nothing new behind it repeats the previous answer rather
    /// than reporting silence — the meter ticks a few times a second and a
    /// block need not have landed between two of them, and a reading that
    /// blinked to "no signal at all" every other tick would be worse than
    /// slightly stale.
    ///
    /// The fraction saturates, and it saturates early: a constant-envelope
    /// signal has `re² + im² = 1`, so one of the two axes is always at least
    /// 0.707 and *every* sample clips once the drive passes √2. It answers "is
    /// the front end into its rails", not "by how much" — the peak beside it is
    /// what the operator watches while turning the gain down.
    pub fn read(&mut self) -> (f32, f32) {
        if self.seen > 0 {
            let dbfs = if self.peak > 0.0 {
                20.0 * (self.peak / self.full_scale).log10()
            } else {
                f32::NEG_INFINITY
            };
            self.last = (dbfs, self.at_scale as f32 / self.seen as f32);
        }
        self.peak = 0.0;
        self.at_scale = 0;
        self.seen = 0;
        self.last
    }
}

#[cfg(test)]
mod tests {
    use sdroxide_types::OVERLOAD_FRACTION;

    use super::*;

    #[test]
    fn a_scaled_backend_is_measured_against_its_own_full_scale() {
        let mut m = AdcMeter::new();
        // Samples scaled by 4: a raw half-scale signal arrives at 2.0.
        m.set_full_scale(4.0);
        m.observe(&block(4_096, 2.0));
        let (dbfs, clip) = m.read();
        assert!((dbfs - (-6.02)).abs() < 0.1, "read {dbfs}");
        assert_eq!(clip, 0.0);
        // Driven past its real rails, it is counted.
        m.observe(&block(4_096, 4.0 * 1.6));
        assert!(m.read().1 > OVERLOAD_FRACTION);
        // Nonsense is ignored rather than dividing by it.
        m.set_full_scale(f32::NAN);
        m.set_full_scale(0.0);
        m.observe(&block(4_096, 2.0));
        assert!((m.read().0 - (-6.02)).abs() < 0.1);
    }

    fn block(n: usize, amp: f32) -> Vec<Complex32> {
        (0..n)
            .map(|i| {
                let ph = i as f32 * 0.1;
                Complex32::new(amp * ph.cos(), amp * ph.sin())
            })
            .collect()
    }

    /// A signal with headroom reads its own peak and nothing at the rails.
    #[test]
    fn an_unclipped_signal_reports_headroom_and_no_overload() {
        let mut m = AdcMeter::new();
        m.observe(&block(4_096, 0.5));
        let (dbfs, frac) = m.read();
        assert!((dbfs - (-6.02)).abs() < 0.2, "read {dbfs} dBFS for half scale");
        assert_eq!(frac, 0.0);
        assert!(frac < OVERLOAD_FRACTION);
    }

    /// A constant-envelope signal driven past √2 clips on every sample, which is
    /// the property that makes the fraction a yes/no rather than a severity.
    #[test]
    fn a_constant_envelope_signal_past_root_two_clips_everywhere() {
        let mut m = AdcMeter::new();
        let clipped: Vec<Complex32> = block(4_096, 1.6)
            .iter()
            .map(|z| Complex32::new(z.re.clamp(-1.0, 1.0), z.im.clamp(-1.0, 1.0)))
            .collect();
        m.observe(&clipped);
        let (dbfs, frac) = m.read();
        assert!(dbfs.abs() < 0.2, "a clipped signal peaks at full scale, read {dbfs}");
        assert_eq!(frac, 1.0);
        assert!(frac > OVERLOAD_FRACTION);
    }

    /// One axis flattened while the magnitude stays inside the unit circle —
    /// the case an envelope-based detector misses entirely.
    #[test]
    fn a_flattened_axis_counts_even_when_the_magnitude_does_not() {
        let mut m = AdcMeter::new();
        m.observe(&[Complex32::new(1.0, 0.0); 64]);
        let (_, frac) = m.read();
        assert_eq!(frac, 1.0, "|z| is 1.0, and so is one axis");
    }

    /// A tick with no samples behind it repeats the last real reading rather
    /// than claiming the signal vanished.
    #[test]
    fn a_read_with_nothing_new_repeats_the_last_answer() {
        let mut m = AdcMeter::new();
        m.observe(&block(1_024, 0.25));
        let first = m.read();
        assert_eq!(m.read(), first);
        assert_eq!(m.read(), first);
    }

    /// And a reading is over its own window, not over all time: a burst that
    /// has been read once does not keep the meter pinned.
    #[test]
    fn the_window_resets_on_every_read() {
        let mut m = AdcMeter::new();
        m.observe(&[Complex32::new(1.0, 1.0); 128]);
        assert_eq!(m.read().1, 1.0);
        m.observe(&block(1_024, 0.1));
        assert_eq!(m.read().1, 0.0);
    }
}
