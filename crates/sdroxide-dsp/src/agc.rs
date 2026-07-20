//! Hang AGC with lookahead delay: fast attack, configurable hang time,
//! slow recovery, capped maximum gain.

use std::collections::VecDeque;

use sdroxide_types::AgcMode;

const TARGET: f32 = 0.35;
const LOOKAHEAD_S: f32 = 0.020;
const ATTACK_TC_S: f32 = 0.002;
const RECOVERY_TC_S: f32 = 0.250;
const ENV_DECAY_TC_S: f32 = 0.050;

pub struct Agc {
    rate: f32,
    enabled: bool,
    max_gain: f32,
    gain: f32,
    env: f32,
    hang_samples: u32,
    hang_left: u32,
    attack_alpha: f32,
    recovery_alpha: f32,
    env_decay: f32,
    delay: VecDeque<f32>,
    delay_len: usize,
}

impl Agc {
    pub fn new(sample_rate: f64) -> Self {
        let rate = sample_rate as f32;
        let mut agc = Agc {
            rate,
            enabled: true,
            max_gain: 10f32.powf(90.0 / 20.0),
            gain: 1.0,
            env: 0.0,
            hang_samples: 0,
            hang_left: 0,
            attack_alpha: 1.0 - (-1.0 / (rate * ATTACK_TC_S)).exp(),
            recovery_alpha: 1.0 - (-1.0 / (rate * RECOVERY_TC_S)).exp(),
            env_decay: (-1.0 / (rate * ENV_DECAY_TC_S)).exp(),
            delay: VecDeque::new(),
            delay_len: (rate * LOOKAHEAD_S) as usize,
        };
        agc.set_mode(AgcMode::Med);
        agc
    }

    pub fn set_mode(&mut self, mode: AgcMode) {
        match mode.hang_ms() {
            Some(ms) => {
                self.enabled = true;
                self.hang_samples = (self.rate * ms / 1000.0) as u32;
            }
            None => {
                self.enabled = false;
                self.gain = 1.0;
            }
        }
    }

    pub fn set_max_gain_db(&mut self, db: f32) {
        self.max_gain = 10f32.powf(db.clamp(0.0, 120.0) / 20.0);
    }

    /// In-place. Output is the delayed input scaled by the tracked gain.
    pub fn process(&mut self, samples: &mut [f32]) {
        if !self.enabled {
            return;
        }
        for s in samples.iter_mut() {
            let x = *s;
            self.delay.push_back(x);
            let delayed = if self.delay.len() > self.delay_len {
                self.delay.pop_front().unwrap()
            } else {
                0.0
            };

            // Envelope: instant attack, exponential decay — sees the sample
            // `LOOKAHEAD_S` before it leaves the delay line.
            let a = x.abs();
            self.env = if a > self.env { a } else { self.env * self.env_decay };

            let desired = (TARGET / self.env.max(1e-9)).min(self.max_gain);
            if desired < self.gain {
                self.gain += (desired - self.gain) * self.attack_alpha;
                self.hang_left = self.hang_samples;
            } else if self.hang_left > 0 {
                self.hang_left -= 1;
            } else {
                self.gain += (desired - self.gain) * self.recovery_alpha;
            }

            *s = delayed * self.gain;
        }
    }
}
