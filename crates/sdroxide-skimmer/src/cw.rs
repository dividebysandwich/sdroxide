//! The CW skimmer core: a streaming STFT over a wide complex-baseband window,
//! per-bin on/off-keying envelope detection with an adaptive noise floor, light
//! signal tracking, and a per-track [`MorseDecoder`]. Produces one
//! [`SkimmerSpot`] per tracked CW signal.

use std::sync::Arc;

use rustfft::{Fft, FftPlanner};
use sdroxide_dsp::Complex32 as C32;
use sdroxide_types::{SkimmerKind, SkimmerSpot};

use crate::callsign::find_callsign;
use crate::morse::MorseDecoder;

// A 4096-pt window over the ~200 kHz skim rate is ~20 ms / ~49 Hz per bin —
// close to a CW signal's own bandwidth, so the carrier lands in one bin with
// good SNR, while the window is still shorter than a dit at moderate speeds.
const FFT_SIZE: usize = 4096;
/// Hop between analysis frames (75% overlap). Frame time = HOP / skim_rate
/// (~5 ms at 200 kHz) — good keying resolution for CW.
const HOP: usize = 1024;
/// Frames of noise-floor priming before detection starts.
const WARMUP: u32 = 40;

/// Key-on threshold above the per-bin noise floor (power ratio; ~10 dB). High
/// enough that random noise rarely crosses it across thousands of bins.
const ON_RATIO: f32 = 10.0;
/// Key-off threshold (per-track hysteresis; ~6 dB).
const OFF_RATIO: f32 = 4.0;
/// A peak must be the strongest bin within ±this window to count — enforces a
/// minimum signal spacing and rejects a strong signal's own leakage sidelobes.
const PEAK_SPACING: usize = 8; // ~±390 Hz at 49 Hz/bin
/// Bins within this of a track's center are "the same signal" (spawn tolerance).
const TRACK_TOL: i64 = 3;
/// Guard band around DC (the window center) to ignore, in bins.
const DC_GUARD: i64 = 3;
/// Frames a track must be detected before it's reported (rejects noise blips;
/// one dit at 20 WPM is ~12 frames at this hop).
const MIN_HITS: u32 = 8;
/// Envelope low-pass factor (fraction of the previous envelope retained). A
/// little smoothing bridges frame-to-frame flicker on marginal signals; too
/// much makes a strong signal's envelope decay slowly and merge marks.
const ENV_A: f32 = 0.0;

/// Track pruning (ms): empties (noise blips) go fast; decoded tracks linger.
const PRUNE_EMPTY_MS: f64 = 1200.0;
const PRUNE_DECODED_MS: f64 = 8000.0;
/// A track counts as "active" (currently keying) within this of its last mark.
const ACTIVE_MS: f64 = 1500.0;
/// Force-decode the pending character after this much silence.
const FLUSH_SILENCE_MS: f32 = 800.0;
/// Bound on simultaneous tracks.
const MAX_TRACKS: usize = 256;

struct Track {
    id: u64,
    bin: i64, // signed offset bin from DC (negative = below center)
    dec: MorseDecoder,
    key_on: bool,
    dur_ms: f32,
    silence_ms: f32,
    flushed: bool,
    last_on_ms: f64,
    snr_db: i16,
    /// Frames this track has been keyed (for confirmation).
    hits: u32,
    /// Smoothed power envelope at the track's bin.
    env: f32,
    /// Time-smoothed power at [bin-1, bin, bin+1], accumulated over keyed-on
    /// frames. Quadratic interpolation over these three resolves the carrier to
    /// a fraction of a bin, so the spot marker lands on the signal instead of
    /// snapping to the 49 Hz grid. Smoothing is essential: a single frame of a
    /// keying CW signal is spiky and would bias the estimate.
    pk: [f32; 3],
}

pub struct CwSkimmer {
    skim_rate: f64,
    skim_center_hz: f64,
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    inbuf: Vec<C32>,
    /// Read cursor into `inbuf`; consumed samples are compacted out rarely so
    /// the STFT never memmoves the whole buffer every frame.
    read_pos: usize,
    scratch: Vec<C32>,
    power: Vec<f32>,
    noise: Vec<f32>,
    /// Reused per-frame scratch (avoids re-allocating every frame).
    cands: Vec<(f32, i64)>,
    centers: Vec<i64>,
    frame_ms: f32,
    frames: u32,
    now_ms: f64,
    tracks: Vec<Track>,
    next_id: u64,
    /// Centers seen last frame, so a track spawns only on a peak that persists
    /// (a single-frame noise blip never becomes a track).
    prev_centers: Vec<i64>,
}

impl CwSkimmer {
    pub fn new(skim_rate: f64, skim_center_hz: f64) -> Self {
        let fft = FftPlanner::<f32>::new().plan_fft_forward(FFT_SIZE);
        // Hann window (reduces spectral leakage between adjacent CW signals).
        let window: Vec<f32> = (0..FFT_SIZE)
            .map(|i| {
                let x = std::f32::consts::PI * i as f32 / (FFT_SIZE as f32 - 1.0);
                x.sin().powi(2)
            })
            .collect();
        CwSkimmer {
            skim_rate,
            skim_center_hz,
            fft,
            window,
            inbuf: Vec::with_capacity(FFT_SIZE * 4),
            read_pos: 0,
            scratch: vec![C32::default(); FFT_SIZE],
            power: vec![0.0; FFT_SIZE],
            noise: vec![0.0; FFT_SIZE],
            cands: Vec::with_capacity(512),
            centers: Vec::with_capacity(256),
            frame_ms: (HOP as f64 / skim_rate * 1000.0) as f32,
            frames: 0,
            now_ms: 0.0,
            tracks: Vec::new(),
            next_id: 1,
            prev_centers: Vec::new(),
        }
    }

    pub fn set_center(&mut self, center_hz: f64) {
        if (center_hz - self.skim_center_hz).abs() > 1.0 {
            self.skim_center_hz = center_hz;
            self.tracks.clear();
            self.inbuf.clear();
            self.read_pos = 0;
            self.frames = 0; // re-prime the noise floor
        }
    }

    /// Feed a block of complex baseband IQ (skim-rate, centered on skim_center).
    pub fn process(&mut self, iq: &[C32]) {
        self.inbuf.extend_from_slice(iq);
        while self.read_pos + FFT_SIZE <= self.inbuf.len() {
            let base = self.read_pos;
            for i in 0..FFT_SIZE {
                self.scratch[i] = self.inbuf[base + i] * self.window[i];
            }
            self.fft.process(&mut self.scratch);
            self.on_frame();
            self.read_pos += HOP;
            self.frames = self.frames.saturating_add(1);
            self.now_ms += self.frame_ms as f64;
        }
        // Compact only once the consumed prefix is large, so the memmove is
        // amortized O(1)/sample instead of shifting the buffer every frame.
        if self.read_pos >= FFT_SIZE {
            self.inbuf.drain(..self.read_pos);
            self.read_pos = 0;
        }
    }

    fn on_frame(&mut self) {
        let n = FFT_SIZE;
        for k in 0..n {
            self.power[k] = self.scratch[k].norm_sqr();
        }

        if self.frames < WARMUP {
            // Prime the per-bin floor quickly, no detection yet.
            for k in 0..n {
                self.noise[k] = 0.9 * self.noise[k] + 0.1 * self.power[k];
            }
            return;
        }

        // Slowly update each bin's noise floor from frames where it's quiet
        // (below the key-on threshold), so signals don't inflate their own floor.
        for k in 0..n {
            if self.power[k] <= self.noise[k] * ON_RATIO {
                self.noise[k] = 0.98 * self.noise[k] + 0.02 * self.power[k];
            }
        }

        // Signal centers via non-max suppression: collect above-threshold bins,
        // then take them strongest-first, suppressing anything within
        // ±PEAK_SPACING of an already-taken peak. This yields exactly one center
        // per signal (a plateau of near-equal bins can't spawn duplicates) and
        // rejects a strong signal's own leakage sidelobes.
        let mut cands = std::mem::take(&mut self.cands);
        cands.clear();
        for k in 0..n {
            let off = self.offset_bin(k);
            if off.abs() < DC_GUARD {
                continue;
            }
            let p = self.power[k];
            if p > self.noise[k] * ON_RATIO {
                cands.push((p, off));
            }
        }
        cands.sort_unstable_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let spacing = PEAK_SPACING as i64;
        let mut centers = std::mem::take(&mut self.centers);
        centers.clear();
        for &(_, off) in cands.iter() {
            if centers.iter().all(|&c| (c - off).abs() > spacing) {
                centers.push(off);
            }
        }

        // Spawn a fixed-bin track for each center that has no track nearby and
        // that persisted from the previous frame (a single-frame noise blip
        // never becomes one). Tracks never move — CW carriers are stable to well
        // under a bin, so a fixed bin gives clean per-signal envelopes and can't
        // drift onto a neighbour's noise.
        for &off in &centers {
            let near = self.tracks.iter().any(|t| (t.bin - off).abs() <= spacing);
            let persisted = self.prev_centers.iter().any(|&c| (c - off).abs() <= TRACK_TOL);
            if !near && persisted && self.tracks.len() < MAX_TRACKS {
                let id = self.next_id;
                self.next_id += 1;
                let k = self.bin_index(off);
                let km = self.bin_index(off - 1);
                let kp = self.bin_index(off + 1);
                self.tracks.push(Track {
                    id,
                    bin: off,
                    dec: MorseDecoder::new(),
                    key_on: false,
                    dur_ms: 0.0,
                    silence_ms: 0.0,
                    flushed: false,
                    last_on_ms: self.now_ms,
                    snr_db: 0,
                    hits: 0,
                    env: self.power[k],
                    pk: [self.power[km], self.power[k], self.power[kp]],
                });
            }
        }
        // This frame's centers become next frame's `prev_centers`; the old
        // buffers are kept for reuse (no per-frame allocation).
        std::mem::swap(&mut self.prev_centers, &mut centers);
        self.centers = centers;
        self.cands = cands;

        // Advance every track from a smoothed single-bin envelope: low-pass the
        // power at its (fixed) bin and key on/off with hysteresis. The smoothing
        // bridges frame-to-frame flicker; the 10:4 on:off ratio prevents chatter.
        let dt = self.frame_ms;
        let now = self.now_ms;
        for t in self.tracks.iter_mut() {
            let k = (t.bin.rem_euclid(n as i64)) as usize;
            let floor = self.noise[k].max(1e-12);
            t.env = ENV_A * t.env + (1.0 - ENV_A) * self.power[k];
            let on = if t.key_on {
                t.env > floor * OFF_RATIO
            } else {
                t.env > floor * ON_RATIO
            };
            if on {
                t.hits = t.hits.saturating_add(1);
                t.last_on_ms = now;
                t.snr_db = (10.0 * (t.env / floor).log10()).round().clamp(-30.0, 60.0) as i16;
                // Accumulate the smoothed 3-bin peak shape while keyed.
                let km = ((t.bin - 1).rem_euclid(n as i64)) as usize;
                let kp = ((t.bin + 1).rem_euclid(n as i64)) as usize;
                t.pk[0] = 0.9 * t.pk[0] + 0.1 * self.power[km];
                t.pk[1] = 0.9 * t.pk[1] + 0.1 * self.power[k];
                t.pk[2] = 0.9 * t.pk[2] + 0.1 * self.power[kp];
            }
            if on == t.key_on {
                t.dur_ms += dt;
            } else {
                if t.key_on {
                    t.dec.on_mark(t.dur_ms);
                } else {
                    t.dec.on_gap(t.dur_ms);
                }
                t.key_on = on;
                t.dur_ms = dt;
            }
            if on {
                t.silence_ms = 0.0;
                t.flushed = false;
            } else {
                t.silence_ms += dt;
                if t.silence_ms > FLUSH_SILENCE_MS && !t.flushed {
                    t.dec.flush();
                    t.flushed = true;
                }
            }
        }

        // Prune stale tracks.
        self.tracks.retain(|t| {
            let age = now - t.last_on_ms;
            if t.dec.text().is_empty() {
                age < PRUNE_EMPTY_MS
            } else {
                age < PRUNE_DECODED_MS
            }
        });
    }

    /// Snapshot the current tracked signals worth reporting. Filters out noise
    /// tracks: a spot needs a confirmed track, a plausible speed, and text with
    /// real content (a callsign, or several non-trivial characters — random
    /// noise mostly decodes to strings of E/I/T).
    pub fn spots(&self) -> Vec<SkimmerSpot> {
        let bin_hz = self.skim_rate / FFT_SIZE as f64;
        self.tracks
            .iter()
            .filter_map(|t| {
                if t.hits < MIN_HITS {
                    return None;
                }
                let text = t.dec.text();
                let wpm = t.dec.wpm();
                if !(8..=45).contains(&wpm) {
                    return None;
                }
                // Quadratic peak interpolation over the smoothed 3-bin shape
                // gives a sub-bin carrier offset in [-0.5, 0.5].
                let [a, b, c] = t.pk;
                let denom = a - 2.0 * b + c;
                let delta = if denom < 0.0 {
                    (0.5 * (a - c) / denom).clamp(-0.5, 0.5)
                } else {
                    0.0
                };
                let callsign = find_callsign(text);
                let meaty = text
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric() && !matches!(c, 'E' | 'I' | 'T'))
                    .count();
                if callsign.is_none() && meaty < 3 {
                    return None;
                }
                Some(SkimmerSpot {
                    id: t.id,
                    kind: SkimmerKind::Cw,
                    freq_hz: self.skim_center_hz + (t.bin as f64 + delta as f64) * bin_hz,
                    callsign,
                    text: text.to_string(),
                    snr_db: t.snr_db,
                    wpm,
                    active: (self.now_ms - t.last_on_ms) < ACTIVE_MS,
                })
            })
            .collect()
    }

    #[cfg(test)]
    pub fn debug_dump(&self) {
        for t in &self.tracks {
            if t.hits >= 3 {
                eprintln!(
                    "bin{} hits{} wpm{} text={:?}",
                    t.bin,
                    t.hits,
                    t.dec.wpm(),
                    t.dec.text()
                );
            }
        }
    }

    /// Signed offset (in bins) of FFT bin `k` from DC.
    fn offset_bin(&self, k: usize) -> i64 {
        let n = FFT_SIZE as i64;
        let k = k as i64;
        if k <= n / 2 { k } else { k - n }
    }

    /// FFT bin index for a signed offset bin.
    fn bin_index(&self, off: i64) -> usize {
        let n = FFT_SIZE as i64;
        (off.rem_euclid(n)) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build ~`secs` of skim IQ: a keyed CW tone at `off_hz` plus light noise.
    fn synth(text: &str, off_hz: f64, wpm: f32, rate: f64) -> Vec<C32> {
        let dit = 1200.0 / wpm; // ms
        // Reuse the morse test-encoder shape inline: elements → key envelope.
        let mut key: Vec<bool> = Vec::new(); // per-ms key state
        let push = |v: &mut Vec<bool>, on: bool, ms: f32| {
            for _ in 0..ms.round() as usize {
                v.push(on);
            }
        };
        // 200 ms leading silence to prime the noise floor.
        push(&mut key, false, 250.0);
        let words: Vec<&str> = text.split(' ').filter(|w| !w.is_empty()).collect();
        for (wi, word) in words.iter().enumerate() {
            let chars: Vec<char> = word.chars().collect();
            for (ci, ch) in chars.iter().enumerate() {
                let code = crate::morse::encode_char(*ch).unwrap();
                let m = code.chars().count();
                for (ei, el) in code.chars().enumerate() {
                    push(&mut key, true, if el == '-' { dit * 3.0 } else { dit });
                    if ei + 1 < m {
                        push(&mut key, false, dit);
                    }
                }
                if ci + 1 < chars.len() {
                    push(&mut key, false, dit * 3.0);
                }
            }
            if wi + 1 < words.len() {
                push(&mut key, false, dit * 7.0);
            }
        }
        push(&mut key, false, 1200.0); // trailing silence → flush

        let mut iq = Vec::with_capacity(key.len() * (rate as usize / 1000));
        let spm = rate / 1000.0; // samples per ms
        let mut phase = 0.0f64;
        let dphi = 2.0 * std::f64::consts::PI * off_hz / rate;
        // simple deterministic "noise"
        let mut seed = 0x1234_5678u32;
        let mut rng = || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / (1u32 << 24) as f32 - 0.5
        };
        let mut ms_frac = 0.0f64;
        for &on in &key {
            ms_frac += spm;
            let take = ms_frac as usize;
            ms_frac -= take as f64;
            for _ in 0..take {
                phase += dphi;
                let amp = if on { 1.0 } else { 0.0 };
                let n = 0.02;
                iq.push(C32::new(
                    amp * phase.cos() as f32 + n * rng(),
                    amp * phase.sin() as f32 + n * rng(),
                ));
            }
        }
        iq
    }

    #[test]
    fn decodes_a_single_cw_tone() {
        let rate = 192_000.0;
        let center = 14_020_000.0;
        let off = 5_000.0;
        let iq = synth("CQ DE W1AW", off, 20.0, rate);

        let mut sk = CwSkimmer::new(rate, center);
        for chunk in iq.chunks(8192) {
            sk.process(chunk);
        }
        sk.debug_dump();
        let spots = sk.spots();
        assert!(!spots.is_empty(), "no spots decoded");
        // The spot should sit within a couple bins of the true frequency.
        let s = spots
            .iter()
            .min_by(|a, b| {
                (a.freq_hz - (center + off))
                    .abs()
                    .partial_cmp(&(b.freq_hz - (center + off)).abs())
                    .unwrap()
            })
            .unwrap();
        assert!(
            (s.freq_hz - (center + off)).abs() < 100.0,
            "freq off: {} vs {}",
            s.freq_hz,
            center + off
        );
        assert!(s.text.contains("W1AW"), "text: {:?}", s.text);
        assert_eq!(s.callsign.as_deref(), Some("W1AW"));
    }

    /// End-to-end frequency accuracy through the *real* engine path: device-rate
    /// IQ → skim DDC → CwSkimmer. Catches any bin/decimation mismatch that would
    /// mistune the spot marker against the waterfall.
    #[test]
    fn frequency_is_accurate_through_the_ddc() {
        let dev_rate = 2_000_000.0;
        let center = 14_025_000.0;
        for off in [2_000.0f64, -3_500.0, 7_000.0] {
            let iq_dev = synth("CQ DE W1AW", off, 24.0, dev_rate);
            let mut ddc = sdroxide_dsp::Ddc::new(dev_rate, 192_000.0);
            let skim_rate = ddc.out_rate();
            let mut sk = CwSkimmer::new(skim_rate, center);
            let mut buf = Vec::new();
            for chunk in iq_dev.chunks(16_384) {
                buf.clear();
                ddc.process(chunk, &mut buf);
                sk.process(&buf);
            }
            let spots = sk.spots();
            assert!(!spots.is_empty(), "off {off}: no spots");
            let want = center + off;
            let s = spots
                .iter()
                .min_by(|a, b| {
                    (a.freq_hz - want).abs().partial_cmp(&(b.freq_hz - want).abs()).unwrap()
                })
                .unwrap();
            let err = s.freq_hz - want;
            eprintln!("off {off:+}: got {} want {} err {err:+.1} Hz", s.freq_hz, want);
            assert!(err.abs() < 20.0, "off {off}: freq error {err:+.1} Hz");
        }
    }
}
