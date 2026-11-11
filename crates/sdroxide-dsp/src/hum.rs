//! Hum filter for the listener audio, switched on from the NR menu: a
//! 4th-order Butterworth high-pass at [`HUM_CORNER_HZ`]. It takes mains hum at
//! 50 or 60 Hz and its first harmonics without having to know which mains it
//! is (−56 dB at 50 Hz, −38 dB at 100 Hz, −24 dB at 150 Hz), and leaves a
//! voice channel, which starts around 300 Hz, alone.
//
// Why: HDZ 2026-11-10: "im Ton sollte so was wie Klickfilter und 50/60-Hz-Filter
//      denkbar sein … bau es als Minipatch ins Submenü NR ein". Measured on
//      Voice of Turkey, 11.815 MHz AM, switched on and off live: the 100 Hz
//      line went from 5–18 dB to 40–44 dB below the speech. The click filter
//      tried beside it is out again ("lass mal IMP weg, wenn er nicht wirkt").

/// Corner of the hum high-pass.
pub const HUM_CORNER_HZ: f64 = 300.0;

#[derive(Clone, Copy, Default)]
struct Biquad {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl Biquad {
    fn highpass(corner_hz: f64, q: f64, rate: f64) -> Self {
        let w0 = std::f64::consts::TAU * corner_hz / rate;
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * q);
        let a0 = 1.0 + alpha;
        Biquad {
            b0: (1.0 + cos) / 2.0 / a0,
            b1: -(1.0 + cos) / a0,
            b2: (1.0 + cos) / 2.0 / a0,
            a1: -2.0 * cos / a0,
            a2: (1.0 - alpha) / a0,
            ..Default::default()
        }
    }

    fn step(&mut self, x: f64) -> f64 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;
        (self.x2, self.x1, self.y2, self.y1) = (self.x1, x, self.y1, y);
        y
    }
}

/// 4th-order Butterworth high-pass at [`HUM_CORNER_HZ`].
pub struct HumFilter {
    rate: f32,
    stages: [Biquad; 2],
}

impl HumFilter {
    pub fn new(rate: f32) -> Self {
        // The two pole pairs of a 4th-order Butterworth.
        let q = [0.541_196_100_146_197, 1.306_562_964_876_376_6];
        let r = f64::from(rate);
        HumFilter {
            rate,
            stages: [
                Biquad::highpass(HUM_CORNER_HZ, q[0], r),
                Biquad::highpass(HUM_CORNER_HZ, q[1], r),
            ],
        }
    }

    pub fn rate(&self) -> f32 {
        self.rate
    }

    pub fn reset(&mut self) {
        *self = HumFilter::new(self.rate);
    }

    pub fn process(&mut self, audio: &mut [f32]) {
        for s in audio {
            let mut v = f64::from(*s);
            for st in &mut self.stages {
                v = st.step(v);
            }
            *s = v as f32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f32 = 48_000.0;

    fn tone(hz: f32, n: usize) -> Vec<f32> {
        (0..n).map(|i| (std::f32::consts::TAU * hz * i as f32 / RATE).sin() * 0.1).collect()
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt()
    }

    /// Proves: mains hum at 50 and 60 Hz goes, a voice tone stays.
    #[test]
    fn the_hum_filter_takes_mains_and_leaves_a_voice() {
        for (hz, lo, hi) in [(50.0, 0.0, -45.0), (60.0, 0.0, -40.0), (1000.0, -0.5, 0.5)] {
            let mut x = tone(hz, 96_000);
            HumFilter::new(RATE).process(&mut x);
            let db = 20.0 * (rms(&x[48_000..]) / rms(&tone(hz, 48_000))).log10();
            if hz < 100.0 {
                assert!(db < hi, "{hz} Hz only down {db:.1} dB");
            } else {
                assert!((lo..hi).contains(&db), "{hz} Hz moved {db:.2} dB");
            }
        }
    }
}
