// Portions of this file are ported from FSK441-PLUS
// (https://github.com/Nythbran23/FSK441-PLUS), which carries this notice:
//
// MIT License — Copyright (c) 2026 Roger Banks GW4WND
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! FSK441 — the original high-speed meteor-scatter mode, receive side.
//!
//! Four-tone FSK at 441 baud on 882 / 1323 / 1764 / 2205 Hz, carrying the
//! 43-character PUA-43 alphabet three dits to a character. A meteor leaves a
//! short ionised trail, so the receiver hears pings of ten to a few hundred
//! milliseconds at random points through a 30-second (or 15-second) T/R period;
//! the operator transmits the message over and over and the decoder's job is to
//! catch whatever fragment a trail reflects and read the text out of it.
//!
//! # Why this is not mfsk-core
//!
//! Every other weak-signal mode in `sdroxide-digi` is a feature flag over
//! mfsk-core's WSJT-X port. FSK441 is not in mfsk-core, so this is the fork's
//! own decoder — the 4-FSK front end, the ping search and the alphabet. It
//! follows the published definition (K1JT, *Definition and specification of the
//! FSK441 encoding scheme used in WSJT*, 2001) and the MIT-licensed Rust
//! reference [`Nythbran23/FSK441-PLUS`](https://github.com/Nythbran23/FSK441-PLUS)
//! (© 2026 Roger Banks GW4WND), whose constants and search shape this port
//! keeps: 25 samples a dit at 11 025 Hz, the sliding matched-filter detector,
//! the sample-level sync search, and the "tone 3 never starts a character"
//! framing that recovers the character boundary.
//!
//! # The rate
//!
//! 441 baud is exactly 25 samples at 11 025 Hz — the rate WSJT and MSHV both
//! capture at. The engine's clean tap is 48 kHz, so the controller resamples to
//! [`FSK441_RATE`] before this sees a sample, exactly as the other modes
//! resample to 12 kHz. Do not re-derive the constants at another rate: the
//! NSPD window and the tone spacing are the mode.

use std::f32::consts::TAU;

use num_complex::Complex32;
use rustfft::FftPlanner;

/// The decoder's sample rate: 441 baud × 25 samples a dit, exactly.
pub const FSK441_RATE: f64 = 11_025.0;
/// The symbol rate.
pub const FSK441_BAUD: f32 = 441.0;
/// Samples per dit — one tone interval.
pub const FSK441_NSPD: usize = 25;
/// The four tone centre frequencies, Hz.
pub const FSK441_TONES: [f32; 4] = [882.0, 1323.0, 1764.0, 2205.0];
/// The 48-symbol alphabet, `nc = 16·d0 + 4·d1 + d2`. 43 are used; the four
/// single-tone encodings (`000`/`111`/`222`/`333`) are the shorthand messages,
/// and a few entries are reserved. Duplicate spaces are the reserved slots.
pub const FSK441_CHARSET: &[u8; 48] = b" 123456789.,?/# $ABCD FGHIJKLMNOPQRSTUVWXY 0EZ*!";

/// The single-tone shorthand each all-one-tone ping means, indexed by tone.
pub const FSK441_SHORTHAND: [&str; 4] = ["R26", "R27", "RRR", "73"];

/// Longest dit count one ping may carry; bounds the work and the text.
const MAX_NDITS: usize = 441;
/// Longest message one ping may decode to, in characters.
const MAX_MSG_CHARS: usize = 46;
/// FFT size for the coarse frequency refinement.
const SPEC_NFFT: usize = 256;
/// How far off the nominal tones a signal may be and still be read, Hz.
pub const DEFAULT_DFTOL: f32 = 200.0;
/// Mean dit confidence a ping must reach. Noise dits average about 0.37, so
/// this is a gate on a short burst that has already stood 6 dB out of the
/// floor.
const PING_MIN_CONFIDENCE: f32 = 0.35;
/// The longest run the detector reads as a meteor ping, in seconds. Underdense
/// trails last well under a second and overdense ones a few; a run longer than
/// this is a signal heard steadily — or a carrier, or noise that filled the
/// buffer as evenly as a signal would — and is judged by the `STEADY_` gates.
const LONGEST_PING_S: f32 = 4.0;
/// Mean dit confidence a steady run must reach. Standing out of the floor is
/// no evidence for it, so the tones themselves have to separate cleanly, as a
/// signal heard for seconds on a direct path does.
const STEADY_MIN_CONFIDENCE: f32 = 0.6;
/// The largest share of a steady run's dits one tone may carry. FSK441 text
/// spreads its dits over all four tones; a carrier puts them all on one.
const STEADY_MAX_TONE_SHARE: f32 = 0.6;

/// One decoded ping.
#[derive(Debug, Clone, PartialEq)]
pub struct Fsk441Ping {
    /// The decoded text. The single-tone shorthand comes out as `R26`, `R27`,
    /// `RRR` or `73`.
    pub text: String,
    /// Frequency offset of the tone set from nominal, Hz.
    pub df_hz: f32,
    /// Where the signal's centre sits in the passband, Hz — the middle of the
    /// four tones plus the offset, which is what the decode list plots.
    pub audio_hz: f32,
    /// Time into the slot the ping starts at, seconds.
    pub start_s: f32,
    /// Ping duration, seconds.
    pub duration_s: f32,
    /// An approximate SNR in dB, the ping's tone energy over the slot's own
    /// noise floor. Not calibrated against a 2500 Hz reference; it orders the
    /// rows rather than reporting a measurement.
    pub snr_db: f32,
    /// Mean dit confidence, 0..1 — how cleanly the four tones separated.
    pub confidence: f32,
}

/// The dit indices for a character, or `None` if it is not in the alphabet.
///
/// Space is `033`. The alphabet has a space in four slots, but three of them
/// — `000`, `111` and `222` — are the single-tone shorthand's (`R26`, `R27`,
/// `RRR`), filled with a space only so a decoder reads them as one; K1JT's
/// definition sends a space as `033`, and so does this.
pub fn fsk441_char_to_dits(c: char) -> Option<(u8, u8, u8)> {
    if c == ' ' {
        return Some((0, 3, 3));
    }
    let uc = c.to_ascii_uppercase();
    let pos = FSK441_CHARSET.iter().position(|&b| b as char == uc)?;
    Some(((pos / 16) as u8, ((pos / 4) % 4) as u8, (pos % 4) as u8))
}

/// The character for three dits. Anything outside the used alphabet reads as a
/// space, which keeps a marginal ping legible instead of filling it with `?`.
pub fn fsk441_dits_to_char(d0: u8, d1: u8, d2: u8) -> char {
    let nc = 16 * d0 as usize + 4 * d1 as usize + d2 as usize;
    FSK441_CHARSET.get(nc).map(|&b| b as char).unwrap_or(' ')
}

/// The single-tone dit index a shorthand word is sent as, or `None`.
///
/// `R26`, `R27`, `RRR` and `73` are not spelled out as text: each is a *pure
/// carrier* on one tone for the whole duration, which is why a meteor catches
/// them far more easily than text. Their text spellings are listed in
/// [`FSK441_SHORTHAND`], indexed by tone — [`fsk441_dits_to_char`] reads them
/// back the same way.
pub fn fsk441_shorthand_tone(word: &str) -> Option<u8> {
    FSK441_SHORTHAND.iter().position(|&s| s.eq_ignore_ascii_case(word)).map(|i| i as u8)
}

/// The tone sequence for a message.
///
/// A whole word that is one of the shorthand words the mode is worked with —
/// `R26`, `R27`, `RRR`, `73` — is sent as its single tone rather than as four
/// characters, which is what the operator means by typing it. Anything else is
/// the ordinary three-dits-a-character, with a character outside the alphabet
/// becoming a space, exactly as the reference's encoder does.
pub fn fsk441_encode_tones(msg: &str) -> Vec<u8> {
    // Time in seconds a single-tone shorthand is held, so a word on its own is
    // a carrier long enough for a trail to reflect rather than a lone dit.
    const SHORTHAND_TONES: usize = 30;
    let trimmed = msg.trim();
    if let Some(tone) = fsk441_shorthand_tone(trimmed) {
        return vec![tone; SHORTHAND_TONES];
    }
    let mut tones = Vec::with_capacity(msg.len() * 3);
    for c in msg.chars() {
        let (d0, d1, d2) = fsk441_char_to_dits(c).unwrap_or((0, 3, 3));
        tones.push(d0);
        tones.push(d1);
        tones.push(d2);
    }
    tones
}

/// Continuous-phase FSK audio for a tone sequence, at [`FSK441_RATE`],
/// normalised to ±1. A signal *generator*, for tests and bench checks — the
/// fork transmits nothing.
pub fn fsk441_generate_audio(tones: &[u8]) -> Vec<f32> {
    let mut samples = Vec::with_capacity(tones.len() * FSK441_NSPD);
    let dt = 1.0_f32 / FSK441_RATE as f32;
    let mut phase = 0.0f32;
    for &t in tones {
        let dpha = TAU * FSK441_TONES[t.min(3) as usize] * dt;
        for _ in 0..FSK441_NSPD {
            samples.push(phase.sin());
            phase += dpha;
            if phase >= TAU {
                phase -= TAU;
            }
        }
    }
    samples
}

/// The sliding matched-filter energy of one tone over `data`: the squared
/// magnitude of the last `NSPD` samples of the tone, sample by sample.
fn detect(data: &[f32], freq: f32) -> Vec<f32> {
    let npts = data.len();
    if npts < FSK441_NSPD {
        return Vec::new();
    }
    let dpha = TAU * freq / FSK441_RATE as f32;
    let c: Vec<Complex32> = data
        .iter()
        .enumerate()
        .map(|(i, &s)| {
            let a = dpha * i as f32;
            Complex32::new(s * a.cos(), -s * a.sin())
        })
        .collect();

    let mut csum: Complex32 = c[..FSK441_NSPD].iter().sum();
    let mut y = Vec::with_capacity(npts - FSK441_NSPD + 1);
    y.push(csum.norm_sqr());
    for i in 1..npts.saturating_sub(FSK441_NSPD - 1) {
        csum = csum - c[i - 1] + c[i + FSK441_NSPD - 1];
        y.push(csum.norm_sqr());
    }
    y
}

/// The sample offset of the dit grid within the first `NSPD` window.
///
/// Every dit position has one tone dominant and the rest quiet; the sample
/// offset where that contrast is strongest, accumulated over the whole ping,
/// is the boundary. A single-bin DFT over the `NSPD` phase bins interpolates
/// it, which is the reference's `find_sync_phase`.
fn find_sync_phase(y: &[[f32; 4]]) -> usize {
    let mut zf = [0.0f32; FSK441_NSPD];
    for (i, e) in y.iter().enumerate() {
        let best = e.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let second = e
            .iter()
            .copied()
            .filter(|&v| (v - best).abs() > 1e-30)
            .fold(f32::NEG_INFINITY, f32::max)
            .max(0.0);
        zf[i % FSK441_NSPD] += 1e-6 * (best - second);
    }
    let csum: Complex32 = zf
        .iter()
        .enumerate()
        .map(|(j, &z)| {
            let a = TAU * j as f32 / FSK441_NSPD as f32;
            z * Complex32::new(a.cos(), -a.sin())
        })
        .sum();
    let phase = -csum.im.atan2(csum.re);
    ((FSK441_NSPD as f32 * phase / TAU).round() as i32).rem_euclid(FSK441_NSPD as i32) as usize
}

/// The frequency offset that best aligns the four nominal tones with the
/// signal, searched over ±`dftol` Hz. A meteor ping can be mistuned by a few
/// tens of Hz and every tone would otherwise lose energy.
fn refine_frequency(data: &[f32], dftol: f32) -> f32 {
    let nfft = SPEC_NFFT;
    let nh = nfft / 2;
    let df_bin = FSK441_RATE as f32 / nfft as f32;

    let n_windows = data.len() / nfft;
    if n_windows == 0 {
        return 0.0;
    }
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(nfft);
    let mut s = vec![0.0f32; nh];
    for w in 0..n_windows {
        let start = w * nfft;
        let mut buf: Vec<Complex32> =
            data[start..start + nfft].iter().map(|&x| Complex32::new(x, 0.0)).collect();
        fft.process(&mut buf);
        for i in 0..nh {
            s[i] += buf[i].norm_sqr();
        }
    }
    let fac = 1.0 / (100.0 * nfft as f32 * n_windows as f32);
    s.iter_mut().for_each(|x| *x *= fac);

    // Each tone is read at its exact frequency, interpolated between bins,
    // with the offset stepped finer than a bin — so a signal on its nominal
    // tones refines to zero rather than to whichever bin its tones round to.
    // The search is the reference's `±dftol` about nominal, no wider: a
    // window reaching a whole tone spacing down lets three of the four tones
    // line up one slot low.
    let wgt = [1.0f32, 4.0, 6.0, 4.0, 1.0];
    let bin = |pos: f32| -> f32 {
        let i0 = pos.floor();
        let frac = pos - i0;
        let at = |i: f32| s[(i as i32).clamp(0, nh as i32 - 1) as usize];
        at(i0) * (1.0 - frac) + at(i0 + 1.0) * frac
    };
    const STEP_HZ: f32 = 5.0;
    let mut smax = 0.0f32;
    let mut best_df = 0.0f32;
    let steps = (dftol / STEP_HZ).round() as i32;
    for k in -steps..=steps {
        let off = k as f32 * STEP_HZ;
        let sum: f32 = FSK441_TONES
            .iter()
            .map(|&f| {
                let centre = (f + off) / df_bin;
                wgt.iter().enumerate().map(|(j, &w)| w * bin(centre + j as f32 - 2.0)).sum::<f32>()
            })
            .sum();
        if sum > smax {
            smax = sum;
            best_df = off;
        }
    }
    best_df
}

/// The hard tone of one dit and how cleanly it separated from the runner-up.
fn soft_dit(e: &[f32; 4]) -> (u8, f32) {
    let mut best_idx = 0usize;
    let mut best = f32::NEG_INFINITY;
    for (i, &v) in e.iter().enumerate() {
        if v > best {
            best = v;
            best_idx = i;
        }
    }
    let second = e
        .iter()
        .enumerate()
        .filter(|&(i, _)| i != best_idx)
        .map(|(_, &v)| v)
        .fold(f32::NEG_INFINITY, f32::max)
        .max(0.0);
    let confidence = if best > 1e-30 { ((best - second) / best).clamp(0.0, 1.0) } else { 0.0 };
    (best_idx as u8, confidence)
}

/// The energy matrix for the four tones at a given frequency offset.
fn energy_matrix(data: &[f32], df: f32) -> Vec<[f32; 4]> {
    let raw: [Vec<f32>; 4] = [
        detect(data, FSK441_TONES[0] + df),
        detect(data, FSK441_TONES[1] + df),
        detect(data, FSK441_TONES[2] + df),
        detect(data, FSK441_TONES[3] + df),
    ];
    let n = raw.iter().map(|e| e.len()).min().unwrap_or(0);
    (0..n).map(|i| [raw[0][i], raw[1][i], raw[2][i], raw[3][i]]).collect()
}

/// The hard tone of every dit in a ping, aligned by the sync search and then
/// trimmed to the dits that actually carry signal.
///
/// The ping search pads the run by a block either side, and a real ping fades
/// in and out, so the matrix has dits of near-zero energy at both ends.
/// Counting those would drag the mean confidence down and add junk characters;
/// the trim is to where a tone carries at least a fifth of the ping's peak.
fn extract_dits(mat: &[[f32; 4]]) -> Vec<(u8, f32)> {
    if mat.is_empty() {
        return Vec::new();
    }
    let jpk = find_sync_phase(mat);
    let n_dits = (mat.len().saturating_sub(jpk) / FSK441_NSPD).min(MAX_NDITS);
    if n_dits < 6 {
        return Vec::new();
    }
    let dits: Vec<[f32; 4]> = (0..n_dits).map(|i| mat[jpk + i * FSK441_NSPD]).collect();

    let best: Vec<f32> =
        dits.iter().map(|e| e.iter().copied().fold(f32::NEG_INFINITY, f32::max)).collect();
    let peak = best.iter().copied().fold(0.0f32, f32::max);
    if peak <= 1e-30 {
        return Vec::new();
    }
    let lo = best.iter().position(|&v| v > 0.2 * peak).unwrap_or(0);
    let hi = best.iter().rposition(|&v| v > 0.2 * peak).map(|i| i + 1).unwrap_or(best.len());
    dits[lo..hi].iter().map(soft_dit).collect()
}

/// The dominant tone of a ping's dits and how many carry it.
fn dominant_tone(dits: &[(u8, f32)]) -> (usize, usize) {
    let mut counts = [0usize; 4];
    for &(t, _) in dits {
        counts[t as usize] += 1;
    }
    counts.iter().enumerate().max_by_key(|&(_, &c)| c).map(|(i, &c)| (i, c)).unwrap_or((0, 0))
}

/// The offset of a single tone's spectral peak from its nominal frequency,
/// searched over ±300 Hz. Used for the shorthand, whose whole signal is one
/// tone and so has no tone set for [`refine_frequency`] to align.
fn tone_offset(data: &[f32], dom: usize) -> f32 {
    let n = data.len().min(4096);
    let data = &data[..n];
    if data.is_empty() {
        return 0.0;
    }
    let nominal = FSK441_TONES[dom];
    let mut best = 0.0f32;
    let mut best_df = 0.0f32;
    let mut f = nominal - 300.0;
    while f <= nominal + 300.0 {
        let dpha = TAU * f / FSK441_RATE as f32;
        let (mut re, mut im) = (0.0f32, 0.0f32);
        for (i, &s) in data.iter().enumerate() {
            let a = dpha * i as f32;
            re += s * a.cos();
            im -= s * a.sin();
        }
        let m = re * re + im * im;
        if m > best {
            best = m;
            best_df = f - nominal;
        }
        f += 5.0;
    }
    best_df
}

/// Decode one ping's audio into its text, frequency offset and confidence.
/// `None` when the ping is too short or too unclear to be a real signal.
///
/// `steady` is a run longer than any meteor ping — a signal heard for
/// seconds on end, or a carrier, or noise. There a single tone is refused,
/// shorthand included: a steady tone for that long is a birdie or a tune-up as
/// often as it is `RRR`, and nothing in the audio tells them apart. Text must
/// use the tones as text does, no one of them carrying most of the dits.
fn decode_ping(data: &[f32], steady: bool) -> Option<(String, f32, f32)> {
    let npts = data.len().min(FSK441_RATE as usize);
    if npts < FSK441_NSPD * 6 {
        return None;
    }
    let data = &data[..npts];

    // The single-tone shorthand first, and at the nominal tones: a whole ping
    // on one tone is `R26`/`R27`/`RRR`/`73`, not text, and the four-tone
    // frequency refine has nothing to align against when only one tone is
    // present. Noise does not concentrate on one tone like this, so a high
    // dominant share is the signal, not a coincidence.
    let nominal = extract_dits(&energy_matrix(data, 0.0));
    if nominal.len() >= 8 {
        let (dom, dom_count) = dominant_tone(&nominal);
        let share = dom_count as f32 / nominal.len() as f32;
        if steady && share > STEADY_MAX_TONE_SHARE {
            return None;
        }
        if share >= 0.9 {
            let conf = nominal.iter().map(|d| d.1).sum::<f32>() / nominal.len() as f32;
            return Some((FSK441_SHORTHAND[dom].to_string(), tone_offset(data, dom), conf));
        }
    }

    let df = refine_frequency(data, DEFAULT_DFTOL);
    let soft = extract_dits(&energy_matrix(data, df));
    if soft.is_empty() {
        return None;
    }
    let mean_confidence = soft.iter().map(|d| d.1).sum::<f32>() / soft.len() as f32;

    // Tone 3 never starts a character, so the character boundary is the mod-3
    // phase where tone 3 is least often the dominant tone.
    let mut n4 = [0u32; 3];
    for (i, &(t, _)) in soft.iter().enumerate() {
        if t == 3 {
            n4[i % 3] += 1;
        }
    }
    let jsync = n4.iter().enumerate().min_by_key(|&(_, &c)| c).map(|(i, _)| i).unwrap_or(0);

    let n_chars = ((soft.len().saturating_sub(jsync)) / 3).min(MAX_MSG_CHARS);
    let mut text = String::with_capacity(n_chars);
    for i in 0..n_chars {
        let j = jsync + i * 3;
        if j + 2 >= soft.len() {
            break;
        }
        text.push(fsk441_dits_to_char(soft[j].0, soft[j + 1].0, soft[j + 2].0));
    }
    let text = dedup_spaces(text.trim());
    if text.is_empty() {
        return None;
    }
    Some((text, df, mean_confidence))
}

/// Squeeze runs of spaces to one, so a repeated message reads as words.
fn dedup_spaces(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for c in s.chars() {
        if c == ' ' {
            if !prev_space {
                out.push(' ');
            }
            prev_space = true;
        } else {
            out.push(c);
            prev_space = false;
        }
    }
    out
}

/// The ping search: block energy, a noise floor and a run above it.
///
/// Returns the pings found in one slot, in time order. Each carries its text
/// (empty pings are dropped), where it sat and an approximate SNR. This is the
/// offline form of the reference's streaming detector — the controller hands in
/// a whole slot, so there is no cooldown to keep and every ping in the slot is
/// found in one pass.
pub fn fsk441_find_pings(slot: &[f32]) -> Vec<Fsk441Ping> {
    let npts = slot.len();
    // The detector needs the whole matched filter, and the ping below wants a
    // few dits to be worth decoding.
    if npts < FSK441_NSPD * 8 {
        return Vec::new();
    }

    // A tone-selective envelope: at each sample, the strongest of the four
    // matched filters. A broadband noise burst raises the raw power but not
    // this, which is what keeps the search from chasing noise.
    let mut tone_env = vec![0.0f32; npts - FSK441_NSPD + 1];
    for &tone in &FSK441_TONES {
        let y = detect(slot, tone);
        for (i, &v) in y.iter().enumerate() {
            if v > tone_env[i] {
                tone_env[i] = v;
            }
        }
    }

    // 10 ms blocks, the reference's `ping_bounds` resolution.
    let block = (FSK441_RATE as f32 * 0.010) as usize;
    let n_blocks = tone_env.len() / block;
    if n_blocks < 4 {
        return Vec::new();
    }
    let blocks: Vec<f32> = (0..n_blocks)
        .map(|b| tone_env[b * block..(b + 1) * block].iter().copied().fold(0.0f32, f32::max))
        .collect();

    let peak = blocks.iter().copied().fold(0.0f32, f32::max);
    if peak <= 1e-30 {
        return Vec::new();
    }
    // The noise floor, from the blocks that carry anything at all: digital
    // silence (a muted input, a stream that started late) says nothing about
    // the noise, and a floor taken from it sits at zero and turns everything
    // after it into one long ping. Of the rest, the tenth percentile — under
    // the signal even when most of the buffer is signal, which the median is
    // not, yet not dragged down by one deep fade the way the quietest block
    // is.
    let mut live: Vec<f32> = blocks.iter().copied().filter(|&b| b > peak * 1e-6).collect();
    live.sort_by(f32::total_cmp);
    let floor = live[live.len() / 10];
    // When even that is close to the loudest block the buffer is signal from
    // end to end — an operator transmits the message over and over — and there
    // is no noise in it to threshold against, so the whole buffer is the run.
    // Such a run is long, and a long run is judged harder than a ping (see
    // `LONGEST_PING_S`), since noise and a carrier fill a buffer as evenly.
    let continuous = floor > 0.5 * peak;
    let thresh = if continuous { 0.0 } else { floor * 4.0 };

    let mut pings = Vec::new();
    let mut b = 0usize;
    while b < n_blocks {
        if blocks[b] <= thresh {
            b += 1;
            continue;
        }
        let start_b = b;
        while b < n_blocks && blocks[b] > thresh {
            b += 1;
        }
        let end_b = b;
        // 40 ms is the reference's `wmin`; anything shorter is noise or a
        // fragment too short to carry a character.
        if (end_b - start_b) * block < (FSK441_RATE as f32 * 0.04) as usize {
            continue;
        }
        let start = start_b.saturating_sub(1) * block;
        let end = ((end_b + 2) * block).min(npts);
        if end <= start {
            continue;
        }
        let audio = &slot[start..end];
        let steady = audio.len() as f32 / FSK441_RATE as f32 > LONGEST_PING_S;
        let Some((text, df, confidence)) = decode_ping(audio, steady) else {
            continue;
        };
        // A run that tripped the detector but decoded with poor tone contrast
        // is far more likely to be a noise burst than a signal; the reference
        // leaves this to its QSO layer, but a decode list has nowhere to put
        // the doubt, so drop it here.
        let min_confidence = if steady { STEADY_MIN_CONFIDENCE } else { PING_MIN_CONFIDENCE };
        if confidence < min_confidence {
            continue;
        }
        let peak = blocks[start_b..end_b].iter().copied().fold(0.0f32, f32::max);
        pings.push(Fsk441Ping {
            text,
            df_hz: df,
            audio_hz: (FSK441_TONES[0] + FSK441_TONES[3]) / 2.0 + df,
            start_s: start as f32 / FSK441_RATE as f32,
            duration_s: (end - start) as f32 / FSK441_RATE as f32,
            snr_db: 10.0 * (peak / floor.max(1e-30)).max(1.0).log10(),
            confidence,
        });
    }
    pings
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The alphabet round-trips, including the two reserved-slot duplicates of
    /// space and the shorthand tones.
    #[test]
    fn the_alphabet_round_trips() {
        assert_eq!(fsk441_char_to_dits(' '), Some((0, 3, 3)), "space is 033, not a shorthand tone");
        assert_eq!(fsk441_char_to_dits('A'), Some((1, 0, 1)));
        assert_eq!(fsk441_char_to_dits('G'), Some((1, 1, 3)));
        assert_eq!(fsk441_char_to_dits('W'), Some((2, 1, 3)));
        assert_eq!(fsk441_char_to_dits('0'), Some((2, 2, 3)));
        assert_eq!(fsk441_char_to_dits('z'), Some((2, 3, 1)), "lowercase folds up");
        assert_eq!(fsk441_dits_to_char(0, 0, 0), ' ');
        assert_eq!(fsk441_dits_to_char(1, 0, 1), 'A');
        assert_eq!(fsk441_dits_to_char(3, 3, 3), ' ');

        for c in "ABCXYZ 019.,?/#$*!".chars() {
            let (d0, d1, d2) = fsk441_char_to_dits(c).unwrap();
            assert_eq!(fsk441_dits_to_char(d0, d1, d2), c, "{c}");
        }
    }

    /// The encoder emits three dits a character, in the alphabet's order.
    #[test]
    fn the_encoder_emits_three_dits_a_character() {
        assert_eq!(fsk441_encode_tones("G"), vec![1, 1, 3]);
        assert_eq!(fsk441_encode_tones(" "), vec![0, 3, 3]);
        assert_eq!(fsk441_encode_tones("CQ").len(), 6);
        // A character outside the alphabet becomes a space rather than panicking.
        assert_eq!(fsk441_encode_tones("~"), vec![0, 3, 3]);
    }

    /// The four shorthand words are sent as their single tone, not spelled out:
    /// `RRR` as four characters is not what the operator means by typing it, and
    /// a meteor catches a pure carrier far more easily than text.
    #[test]
    fn a_shorthand_word_is_its_single_tone() {
        for (word, tone) in [("R26", 0u8), ("R27", 1), ("RRR", 2), ("73", 3)] {
            let tones = fsk441_encode_tones(word);
            assert!(tones.iter().all(|&t| t == tone), "{word} must be tone {tone}: {tones:?}");
            assert!(tones.len() >= 3, "{word} must be held, not one dit");
            // Case is the operator's business; the wire is not.
            assert_eq!(fsk441_encode_tones(&word.to_lowercase()), tones);
        }
        // A shorthand word inside a longer message is ordinary text.
        assert_eq!(fsk441_encode_tones("RRR TEST").len(), "RRR TEST".len() * 3);
    }

    /// A clean single pass of a real message decodes back to itself. This is
    /// the whole chain — ping search, frequency refine, matched filter, sync,
    /// framing and alphabet — over the fork's own generator.
    #[test]
    fn a_clean_ping_decodes_to_its_message() {
        let msg = "CQ DE W1ABC";
        let tones = fsk441_encode_tones(msg);
        let mut slot = vec![0.0f32; 2205]; // 0.2 s of silence
        slot.extend_from_slice(&fsk441_generate_audio(&tones));
        slot.extend(std::iter::repeat_n(0.0, 2205));

        let pings = fsk441_find_pings(&slot);
        assert_eq!(pings.len(), 1, "one ping expected, got {pings:?}");
        assert_eq!(pings[0].text, msg);
        assert!(pings[0].confidence > 0.9, "clean signal should be confident");
        // The ping starts where the signal does, within a block.
        assert!((pings[0].start_s - 0.2).abs() < 0.05, "start {}", pings[0].start_s);
        // On its nominal tones, it refines to no offset — not to whichever
        // spectrum bin the tones happen to round to.
        assert!(pings[0].df_hz.abs() <= 10.0, "df {}", pings[0].df_hz);
    }

    /// A mistuned signal is still found and decoded: the frequency refine has
    /// to absorb an offset, which is what a real ping arrives with.
    #[test]
    fn a_mistuned_ping_is_refined_and_decoded() {
        // Shift the generator's tones by +60 Hz by resynthesising here rather
        // than re-deriving the decoder's constants.
        let msg = "CQ TEST";
        let tones = fsk441_encode_tones(msg);
        let dt = 1.0 / FSK441_RATE as f32;
        let mut phase = 0.0f32;
        let mut audio = Vec::new();
        for &t in &tones {
            let dpha = TAU * (FSK441_TONES[t as usize] + 60.0) * dt;
            for _ in 0..FSK441_NSPD {
                audio.push(phase.sin());
                phase += dpha;
                if phase >= TAU {
                    phase -= TAU;
                }
            }
        }
        let mut slot = vec![0.0f32; 2205];
        slot.extend_from_slice(&audio);
        slot.extend(std::iter::repeat_n(0.0, 2205));

        let pings = fsk441_find_pings(&slot);
        assert_eq!(pings.len(), 1, "{pings:?}");
        assert_eq!(pings[0].text, msg);
        assert!((pings[0].df_hz - 60.0).abs() < 30.0, "df {}", pings[0].df_hz);
    }

    /// A pure single tone is the shorthand, not text. `RRR` (tone 2) and `73`
    /// (tone 3) are the two that are not characters in their own right.
    #[test]
    fn a_single_tone_ping_is_the_shorthand() {
        for (tone, want) in [(0usize, "R26"), (1, "R27"), (2, "RRR"), (3, "73")] {
            let tones = vec![tone as u8; 30];
            let mut slot = vec![0.0f32; 2205];
            slot.extend_from_slice(&fsk441_generate_audio(&tones));
            slot.extend(std::iter::repeat_n(0.0, 2205));
            let pings = fsk441_find_pings(&slot);
            assert_eq!(pings.len(), 1, "tone {tone}: {pings:?}");
            assert_eq!(pings[0].text, want, "tone {tone}");
        }
    }

    /// A weak ping behind noise still decodes: the detector and the confidence
    /// gate have to leave room for the sort of level meteor scatter delivers.
    #[test]
    fn a_weak_ping_still_decodes() {
        let msg = "W1ABC W9XYZ";
        let audio = fsk441_generate_audio(&fsk441_encode_tones(msg));
        let npts = FSK441_RATE as usize;
        let start = npts / 4;
        let mut slot = vec![0.0f32; npts];
        let mut state = 0x1234_abcdu32;
        for s in slot.iter_mut() {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            *s = ((state >> 8) as f32 / 8_388_608.0 - 1.0) * 0.45;
        }
        for (i, &s) in audio.iter().enumerate() {
            if start + i < npts {
                slot[start + i] += s * 0.6;
            }
        }
        let pings = fsk441_find_pings(&slot);
        assert!(pings.iter().any(|p| p.text == msg), "weak ping not decoded: {pings:?}");
    }

    /// Noise alone must not produce a decode: the confidence and tone-shape
    /// gates are what keep the list honest.
    #[test]
    fn noise_alone_does_not_decode() {
        let mut state = 0x1234_5678u32;
        let mut slot = Vec::with_capacity(55_125);
        for _ in 0..55_125 {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            slot.push(((state >> 8) as f32 / 8_388_608.0) - 1.0);
        }
        let pings = fsk441_find_pings(&slot);
        assert!(pings.is_empty(), "noise decoded as {pings:?}");
    }

    /// A message transmitted the way the mode is worked — the pass repeated back
    /// to back with no silence between them — decodes. This is the regression
    /// for the ping search's noise floor: with a signal end to end there is no
    /// silent block, and a *median* floor sat at the signal level so nothing was
    /// ever reported. The floor is now the quietest block, which is under the
    /// signal whether or not the buffer has silence in it.
    #[test]
    fn a_continuously_repeated_message_decodes() {
        let msg = "W1ABC W9XYZ FN42";
        let pass = fsk441_generate_audio(&fsk441_encode_tones(msg));
        let mut slot = Vec::new();
        while slot.len() < FSK441_RATE as usize {
            slot.extend_from_slice(&pass);
        }
        let pings = fsk441_find_pings(&slot);
        assert!(
            pings.iter().any(|p| p.text.contains("W1ABC")),
            "a repeated message did not decode: {pings:?}"
        );
    }

    /// Deterministic white noise at `amp`, for the false-decode tests.
    fn noise(len: usize, amp: f32, seed: u32) -> Vec<f32> {
        let mut state = seed.wrapping_mul(0x9e37_79b9) | 1;
        (0..len)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                ((state >> 8) as f32 / 8_388_608.0 - 1.0) * amp
            })
            .collect()
    }

    /// A steady carrier — a birdie, a neighbour's tune-up — held through a
    /// whole slot is not a message. On one of the four tones it is exactly the
    /// shape of the single-tone shorthand, and a false `RRR` or `73` is the
    /// worst decode this mode can make, since those finish a contact.
    #[test]
    fn a_steady_carrier_is_not_shorthand() {
        let n = 30 * FSK441_RATE as usize;
        for hz in [882.0f32, 1000.0, 1323.0, 1764.0, 2205.0, 2300.0] {
            let w = TAU * hz / FSK441_RATE as f32;
            let mut slot = noise(n, 0.1, hz as u32);
            for (i, s) in slot.iter_mut().enumerate() {
                *s += 0.5 * (w * i as f32).sin();
            }
            let pings = fsk441_find_pings(&slot);
            assert!(pings.is_empty(), "a {hz} Hz carrier decoded as {pings:?}");
        }
    }

    /// A carrier for part of the slot — a neighbour tuning up for ten seconds
    /// on the `RRR` tone — is not shorthand either: it stands out of the noise
    /// as a ping does, but for longer than any meteor trail lasts.
    #[test]
    fn a_tune_up_carrier_is_not_shorthand() {
        let rate = FSK441_RATE as usize;
        let w = TAU * FSK441_TONES[2] / FSK441_RATE as f32;
        let mut slot = noise(30 * rate, 0.1, 5);
        for (i, s) in slot.iter_mut().enumerate().skip(5 * rate).take(10 * rate) {
            *s += 0.5 * (w * i as f32).sin();
        }
        let pings = fsk441_find_pings(&slot);
        assert!(pings.is_empty(), "a tune-up decoded as {pings:?}");
    }

    /// A stretch of digital silence (a muted input, a stream that started late)
    /// does not drag the noise floor to zero and turn the noise after it into a
    /// ping.
    #[test]
    fn noise_after_silence_does_not_decode() {
        let rate = FSK441_RATE as usize;
        let mut slot = vec![0.0f32; 10 * rate];
        slot.extend(noise(20 * rate, 0.5, 7));
        let pings = fsk441_find_pings(&slot);
        assert!(pings.is_empty(), "noise after silence decoded as {pings:?}");
    }

    /// Whole slots of plain noise, many of them: a gate that lets a few percent
    /// of slots through is a false decode every few minutes on an empty band.
    #[test]
    fn slots_of_noise_do_not_decode() {
        let n = 30 * FSK441_RATE as usize;
        for seed in 1..=40 {
            let pings = fsk441_find_pings(&noise(n, 0.5, seed));
            assert!(pings.is_empty(), "seed {seed}: noise decoded as {pings:?}");
        }
    }

    /// A message repeated end to end through a whole slot, under noise, still
    /// decodes — the case a noise floor taken from the signal itself misses.
    #[test]
    fn a_continuous_message_under_noise_decodes() {
        let msg = "W1ABC W9XYZ FN42";
        let pass = fsk441_generate_audio(&fsk441_encode_tones(msg));
        let n = 30 * FSK441_RATE as usize;
        let mut slot = noise(n, 0.3, 11);
        for (i, s) in slot.iter_mut().enumerate() {
            *s += pass[i % pass.len()];
        }
        let pings = fsk441_find_pings(&slot);
        assert!(pings.iter().any(|p| p.text.contains("W1ABC")), "not decoded: {pings:?}");
    }

    /// A message that starts part-way through the slot — noise, then the
    /// signal to the end — decodes: most of the buffer is signal, so neither
    /// the median nor the quietest block alone is the floor to judge it by.
    #[test]
    fn a_message_starting_mid_slot_decodes() {
        let msg = "W1ABC W9XYZ FN42";
        let pass = fsk441_generate_audio(&fsk441_encode_tones(msg));
        let rate = FSK441_RATE as usize;
        let mut slot = noise(30 * rate, 0.1, 13);
        for (i, s) in slot.iter_mut().enumerate().skip(8 * rate) {
            *s += pass[(i - 8 * rate) % pass.len()];
        }
        let pings = fsk441_find_pings(&slot);
        assert!(pings.iter().any(|p| p.text.contains("W1ABC")), "not decoded: {pings:?}");
    }

    /// The text out of a very short fragment is a fragment, not a crash: the
    /// decoder must return what it can and never panic on partial dits.
    #[test]
    fn a_fragment_decodes_without_panicking() {
        let tones = fsk441_encode_tones("W1ABC");
        let audio = fsk441_generate_audio(&tones);
        let _ = decode_ping(&audio[..audio.len() / 2], false);
        let _ = fsk441_find_pings(&audio);
    }

    /// The real thing: an off-air FSK441 recording decodes to the callsign it
    /// carries.
    ///
    /// Point `SDROXIDE_FSK441_SAMPLE` at a mono 11 025 Hz WAV — convert a
    /// capture with `ffmpeg -i capture.mp3 -ac 1 -ar 11025 burst.wav`. The
    /// sample is off-air material and cannot live in the tree, so the test is
    /// `#[ignore]`d and skips when the variable is unset. Run it with
    /// `cargo test -p sdroxide-dsp --release -- --ignored --nocapture`.
    ///
    /// The Sigidwiki *FSK441Burst* sample decodes to `YO2NAA` and its `RRR`
    /// rogers, which is the check this test was written for.
    #[test]
    #[ignore]
    fn an_off_air_burst_decodes() {
        let Ok(path) = std::env::var("SDROXIDE_FSK441_SAMPLE") else {
            eprintln!("SDROXIDE_FSK441_SAMPLE unset; skipping");
            return;
        };
        let mut reader = hound::WavReader::open(&path).expect("open the sample WAV");
        let spec = reader.spec();
        assert_eq!(spec.sample_rate, FSK441_RATE as u32, "the sample must be at 11 025 Hz");
        assert_eq!(spec.channels, 1, "the sample must be mono");
        let audio: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Float => {
                reader.samples::<f32>().map(|s| s.expect("sample")).collect()
            }
            hound::SampleFormat::Int => {
                reader.samples::<i16>().map(|s| s.expect("sample") as f32 / 32_768.0).collect()
            }
        };
        let pings = fsk441_find_pings(&audio);
        assert!(
            pings.iter().any(|p| p.text.contains("YO2NAA")),
            "the off-air sample did not decode: {pings:?}"
        );
    }
}
