//! Wideband PSK31 / RTTY skimmer.
//!
//! Reuses the same streaming STFT as the CW skimmer, but treats the FFT as a
//! *filterbank*: each bin is a down-converted, decimated complex stream at the
//! frame rate (~187 Hz over a 192 kHz window), so a per-track BPSK or FSK
//! decoder consumes bins directly — no per-track decimation. Detection finds
//! persistent carriers; decoding is gated to the digimode band segments.
//!
//! Decode quality is a first cut: 47 Hz bins and ~6 samples/symbol are coarse,
//! so a per-track Costas loop (in [`BpskCore`]) pulls the residual offset. It is
//! good enough to place spots and read call-outs; the operator clicks a spot to
//! decode it cleanly in the PSK/RTTY panel.

use std::sync::Arc;

use rustfft::{Fft, FftPlanner};
use sdroxide_dsp::{BaudotRx, BpskCore, Complex32 as C32};
use sdroxide_types::{SkimmerKind, SkimmerSpot, is_psk_segment, is_rtty_segment};

use crate::callsign::find_callsign;

const FFT_SIZE: usize = 4096;
/// Small hop → a high frame rate (~750 Hz), giving ~24 samples/symbol for PSK
/// and ~16/bit for RTTY — enough for clean symbol timing on the bin filterbank.
const HOP: usize = 256;
const WARMUP: u32 = 80;
/// Detection threshold on the *smoothed* spectrum, above the broadband floor
/// (~7 dB of sustained energy). Smoothing already rejects noise spikes, so a
/// carrier need only stand steadily above the floor.
const SMOOTH_ON: f32 = 5.0;
/// Envelope key-off ratio, for a track's activity/SNR metering.
const OFF_RATIO: f32 = 4.0;
/// Fraction of a carrier's own power a tone `shift` away must reach to count as
/// an RTTY mark/space pair (~-8 dB). Real RTTY tones are comparable; a strong
/// signal's window sidelobes are ~-30 dB, well below this.
const PAIR_FRAC: f32 = 0.15;
const PEAK_SPACING: usize = 6;
const TRACK_TOL: i64 = 2;
const DC_GUARD: i64 = 3;
/// Consecutive frames a peak must persist before it becomes a track — the key
/// noise rejector for continuous carriers (a noise spike lasts 1–2 frames).
const CONFIRM_FRAMES: u32 = 60;
/// Frames a track must sustain the carrier before it may be reported.
const MIN_HITS: u32 = 120;
/// Decoded characters a track needs before it is reported as a spot.
const MIN_CHARS: usize = 4;
const PRUNE_MS: f64 = 5000.0;
const ACTIVE_MS: f64 = 2500.0;
const MAX_TRACKS: usize = 64;
const TEXT_CAP: usize = 64;

const PSK_BAUD: f64 = 31.25;
const RTTY_BAUD: f64 = 45.45;
const RTTY_SHIFT: f64 = 170.0;

/// Per-track RTTY (FSK) demod: slices mark vs space bin power and frames Baudot.
struct RttyDemod {
    baudot: BaudotRx,
    bit_len: f32, // frames per bit
    in_data: bool,
    clk: f32,
    nbits: u8,
    value: u8,
    last_mark: bool,
}

impl RttyDemod {
    fn new(bit_len: f32) -> Self {
        RttyDemod {
            baudot: BaudotRx::new(),
            bit_len,
            in_data: false,
            clk: 0.0,
            nbits: 0,
            value: 0,
            last_mark: true,
        }
    }

    fn push(&mut self, mark_pow: f32, space_pow: f32, out: &mut String) {
        let mark = mark_pow >= space_pow;
        if !self.in_data {
            if self.last_mark && !mark {
                self.in_data = true;
                self.clk = self.bit_len * 1.5;
                self.nbits = 0;
                self.value = 0;
            }
        } else {
            self.clk -= 1.0;
            if self.clk <= 0.0 {
                if mark {
                    self.value |= 1 << self.nbits;
                }
                self.nbits += 1;
                if self.nbits >= 5 {
                    if let Some(c) = self.baudot.decode(self.value) {
                        out.push(c);
                    }
                    self.in_data = false;
                } else {
                    self.clk += self.bit_len;
                }
            }
        }
        self.last_mark = mark;
    }
}

enum Decoder {
    Psk(BpskCore),
    Rtty(RttyDemod),
}

struct Track {
    id: u64,
    bin: i64,
    dec: Decoder,
    text: String,
    last_on_ms: f64,
    snr_db: i16,
    env: f32,
    /// Frames the carrier has been present (confirms a real, sustained signal).
    hits: u32,
}

pub struct DigiSkimmer {
    kind: SkimmerKind,
    skim_rate: f64,
    skim_center_hz: f64,
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    inbuf: Vec<C32>,
    read_pos: usize,
    scratch: Vec<C32>,
    power: Vec<f32>,
    /// Per-bin power smoothed over ~a character time, so a continuous PSK
    /// carrier and an alternating RTTY mark/space pair both read as steady
    /// energy (the raw RTTY tones each drop out at the baud rate).
    smooth_power: Vec<f32>,
    /// Scratch for the per-frame percentile floor estimate.
    pscratch: Vec<f32>,
    /// Smoothed broadband noise floor (a low percentile of the spectrum).
    floor_level: f32,
    cands: Vec<(f32, i64)>,
    centers: Vec<i64>,
    frame_ms: f32,
    frames: u32,
    now_ms: f64,
    tracks: Vec<Track>,
    next_id: u64,
    /// Per-candidate consecutive-frame counts; a peak must persist this long
    /// before it becomes a track (rejects transient noise spikes).
    confirm: Vec<(i64, u32)>,
    /// Samples/symbol for the bin filterbank (frame_rate / baud).
    psk_sps: f32,
    rtty_bit_len: f32,
    rtty_shift_bins: i64,
}

impl DigiSkimmer {
    pub fn new(kind: SkimmerKind, skim_rate: f64, skim_center_hz: f64) -> Self {
        let fft = FftPlanner::<f32>::new().plan_fft_forward(FFT_SIZE);
        let window: Vec<f32> = (0..FFT_SIZE)
            .map(|i| {
                let x = std::f32::consts::PI * i as f32 / (FFT_SIZE as f32 - 1.0);
                x.sin().powi(2)
            })
            .collect();
        let frame_rate = skim_rate / HOP as f64;
        let bin_hz = skim_rate / FFT_SIZE as f64;
        DigiSkimmer {
            kind,
            skim_rate,
            skim_center_hz,
            fft,
            window,
            inbuf: Vec::with_capacity(FFT_SIZE * 4),
            read_pos: 0,
            scratch: vec![C32::default(); FFT_SIZE],
            power: vec![0.0; FFT_SIZE],
            smooth_power: vec![0.0; FFT_SIZE],
            pscratch: vec![0.0; FFT_SIZE],
            floor_level: 0.0,
            cands: Vec::with_capacity(512),
            centers: Vec::with_capacity(128),
            frame_ms: (HOP as f64 / skim_rate * 1000.0) as f32,
            frames: 0,
            now_ms: 0.0,
            tracks: Vec::new(),
            next_id: 1,
            confirm: Vec::new(),
            psk_sps: (frame_rate / PSK_BAUD) as f32,
            rtty_bit_len: (frame_rate / RTTY_BAUD) as f32,
            rtty_shift_bins: (RTTY_SHIFT / bin_hz).round() as i64,
        }
    }

    pub fn set_center(&mut self, center_hz: f64) {
        if (center_hz - self.skim_center_hz).abs() > 1.0 {
            self.skim_center_hz = center_hz;
            self.tracks.clear();
            self.confirm.clear();
            self.inbuf.clear();
            self.read_pos = 0;
            self.frames = 0;
            self.floor_level = 0.0;
            for p in self.smooth_power.iter_mut() {
                *p = 0.0;
            }
        }
    }

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
        if self.read_pos >= FFT_SIZE {
            self.inbuf.drain(..self.read_pos);
            self.read_pos = 0;
        }
    }

    fn new_decoder(&self) -> Decoder {
        match self.kind {
            SkimmerKind::Rtty => Decoder::Rtty(RttyDemod::new(self.rtty_bit_len)),
            _ => Decoder::Psk(BpskCore::new(self.psk_sps)),
        }
    }

    fn on_frame(&mut self) {
        let n = FFT_SIZE;
        for k in 0..n {
            self.power[k] = self.scratch[k].norm_sqr();
        }
        // Robust broadband noise floor: a low percentile of this frame's
        // spectrum, smoothed. A continuous carrier occupies a tiny fraction of
        // bins, so — unlike a per-bin temporal average — it can't capture the
        // floor and hide itself (the failure mode for always-on PSK/RTTY).
        self.pscratch.copy_from_slice(&self.power);
        let pidx = n * 4 / 10; // 40th percentile
        self.pscratch
            .select_nth_unstable_by(pidx, |a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let floor_est = self.pscratch[pidx];
        if self.floor_level <= 0.0 {
            self.floor_level = floor_est;
        }
        self.floor_level = 0.9 * self.floor_level + 0.1 * floor_est;
        if self.frames < WARMUP {
            return;
        }
        let floor = self.floor_level.max(1e-12);
        // Smooth per-bin power over ~a character time so RTTY's alternating
        // mark/space and PSK's continuous carrier both read as steady energy.
        for k in 0..n {
            self.smooth_power[k] = 0.992 * self.smooth_power[k] + 0.008 * self.power[k];
        }

        // Candidate carriers from the smoothed spectrum, gated to digimode
        // segments and away from the automatic modes (FT8/FT4/WSPR) that would
        // only decode to garbage.
        let bin_hz = self.skim_rate / FFT_SIZE as f64;
        let shift = self.rtty_shift_bins;
        let mut cands = std::mem::take(&mut self.cands);
        cands.clear();
        for k in 0..n {
            let off = self.offset_bin(k);
            if off.abs() < DC_GUARD {
                continue;
            }
            if self.smooth_power[k] > floor * SMOOTH_ON {
                let abs_hz = self.skim_center_hz + off as f64 * bin_hz;
                // Restrict each skimmer to its mode's well-known calling sub-bands
                // (PSK31 / RTTY areas per band) — not the whole digi segment.
                let in_band = match self.kind {
                    SkimmerKind::Rtty => is_rtty_segment(abs_hz),
                    _ => is_psk_segment(abs_hz),
                };
                if in_band {
                    cands.push((self.smooth_power[k], off));
                }
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
        // Confirmation: count consecutive frames each candidate persists.
        let mut nextc: Vec<(i64, u32)> = Vec::with_capacity(centers.len());
        for &off in &centers {
            let prev = self
                .confirm
                .iter()
                .find(|(b, _)| (b - off).abs() <= TRACK_TOL)
                .map(|&(_, c)| c)
                .unwrap_or(0);
            nextc.push((off, prev + 1));
        }
        self.confirm = nextc;
        for &(off, count) in &self.confirm {
            // Retry every frame once confirmed until the modulation gate passes;
            // the `near` check below stops a second spawn once a track exists.
            if count < CONFIRM_FRAMES {
                continue;
            }
            let near = self.tracks.iter().any(|t| (t.bin - off).abs() <= spacing);
            if near || self.tracks.len() >= MAX_TRACKS {
                continue;
            }
            // Modulation gate, relative to this carrier's own power (so a strong
            // signal's window sidelobes don't read as a second tone): RTTY needs a
            // comparable tone `shift` below (this bin is the mark); PSK needs a
            // lone carrier (both ±shift bins much weaker than the main).
            let main = self.smooth_power[off.rem_euclid(n as i64) as usize];
            let sp_lo = self.smooth_power[(off - shift).rem_euclid(n as i64) as usize];
            let sp_hi = self.smooth_power[(off + shift).rem_euclid(n as i64) as usize];
            let pair = main * PAIR_FRAC;
            let ok = match self.kind {
                SkimmerKind::Rtty => sp_lo > pair,
                _ => sp_lo < pair && sp_hi < pair,
            };
            if !ok {
                continue;
            }
            let id = self.next_id;
            self.next_id += 1;
            let dec = self.new_decoder();
            self.tracks.push(Track {
                id,
                bin: off,
                dec,
                text: String::new(),
                last_on_ms: self.now_ms,
                snr_db: 0,
                env: 0.0,
                hits: 0,
            });
        }
        self.centers = centers;
        self.cands = cands;

        // Advance every track by feeding it this frame's bin(s).
        let now = self.now_ms;
        for t in self.tracks.iter_mut() {
            let k = t.bin.rem_euclid(n as i64) as usize;
            t.env = 0.9 * t.env + 0.1 * self.power[k];
            if t.env > floor * OFF_RATIO {
                t.last_on_ms = now;
                t.hits = t.hits.saturating_add(1);
                t.snr_db = (10.0 * (t.env / floor).log10()).round().clamp(-30.0, 60.0) as i16;
            }
            match &mut t.dec {
                Decoder::Psk(core) => {
                    let mut s = String::new();
                    core.push(self.scratch[k], &mut s);
                    append_capped(&mut t.text, &s);
                }
                Decoder::Rtty(demod) => {
                    let mk = k;
                    let sp = (t.bin - self.rtty_shift_bins).rem_euclid(n as i64) as usize;
                    let mut s = String::new();
                    demod.push(self.power[mk], self.power[sp], &mut s);
                    append_capped(&mut t.text, &s);
                }
            }
        }

        self.tracks.retain(|t| now - t.last_on_ms < PRUNE_MS);
    }

    pub fn spots(&self) -> Vec<SkimmerSpot> {
        let bin_hz = self.skim_rate / FFT_SIZE as f64;
        self.tracks
            .iter()
            .filter_map(|t| {
                if t.hits < MIN_HITS {
                    return None; // not a sustained, confirmed carrier
                }
                let meaty = t.text.chars().filter(|c| c.is_ascii_alphanumeric()).count();
                if meaty < MIN_CHARS {
                    return None;
                }
                let callsign = find_callsign(&t.text);
                Some(SkimmerSpot {
                    id: t.id,
                    kind: self.kind,
                    freq_hz: self.skim_center_hz + t.bin as f64 * bin_hz,
                    callsign,
                    text: t.text.clone(),
                    snr_db: t.snr_db,
                    wpm: 0,
                    active: (self.now_ms - t.last_on_ms) < ACTIVE_MS,
                })
            })
            .collect()
    }

    fn offset_bin(&self, k: usize) -> i64 {
        let n = FFT_SIZE as i64;
        let k = k as i64;
        if k <= n / 2 { k } else { k - n }
    }

    #[cfg(test)]
    fn debug_dump(&self) {
        let mx = self.smooth_power.iter().cloned().fold(0.0f32, f32::max);
        eprintln!(
            "floor={:.2e} maxsmooth/floor={:.1} tracks={} confirm={:?}",
            self.floor_level,
            mx / self.floor_level.max(1e-12),
            self.tracks.len(),
            self.confirm
        );
        for t in &self.tracks {
            eprintln!("  track bin{} hits{} text={:?}", t.bin, t.hits, t.text);
        }
    }
}

fn append_capped(buf: &mut String, s: &str) {
    if s.is_empty() {
        return;
    }
    buf.push_str(s);
    if buf.chars().count() > TEXT_CAP {
        let drop = buf.chars().count() - TEXT_CAP;
        let cut = buf.char_indices().nth(drop).map(|(i, _)| i).unwrap_or(0);
        buf.drain(..cut);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_dsp::PskTx;

    /// Build skim IQ carrying a PSK31 signal at `off_hz` plus light noise.
    fn synth_psk(text: &str, off_hz: f64, rate: f64) -> Vec<C32> {
        // PskTx at carrier 0 yields the real BPSK baseband; shift it to off_hz.
        let mut tx = PskTx::new(rate, 0.0);
        let mut base = Vec::new();
        let mut pre = vec![0.0f32; (rate * 0.6) as usize];
        tx.next_block(&mut pre);
        base.extend_from_slice(&pre);
        tx.push_text(text);
        // Bounded loop (never rely on block/symbol alignment): render until the
        // message is sent, capped so a bug can never exhaust memory.
        let mut guard = 0;
        while tx.sent_chars() < tx.total_chars() && guard < 4000 {
            let mut b = vec![0.0f32; 4096];
            tx.next_block(&mut b);
            base.extend_from_slice(&b);
            guard += 1;
        }
        let mut tail = vec![0.0f32; 8192];
        tx.next_block(&mut tail);
        base.extend_from_slice(&tail);

        let mut seed = 0x2468_1357u32;
        let mut rng = || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / (1u32 << 24) as f32 - 0.5
        };
        let dphi = 2.0 * std::f64::consts::PI * off_hz / rate;
        base.iter()
            .enumerate()
            .map(|(i, &a)| {
                let ph = dphi * i as f64;
                let n = 0.03;
                C32::new(a * ph.cos() as f32 + n * rng(), a * ph.sin() as f32 + n * rng())
            })
            .collect()
    }

    #[test]
    fn decodes_a_psk_signal() {
        let rate = 192_000.0;
        // Signal at 14.072 MHz: in the PSK area, clear of FT8 (14.074)/FT4/WSPR.
        let center = 14_070_000.0;
        let off = 2_000.0;
        let iq = synth_psk("CQ CQ DE AB1CD K ", off, rate);
        let mut sk = DigiSkimmer::new(SkimmerKind::Psk, rate, center);
        for chunk in iq.chunks(8192) {
            sk.process(chunk);
        }
        let spots = sk.spots();
        assert!(!spots.is_empty(), "no PSK spots decoded");
        let s = spots
            .iter()
            .min_by(|a, b| {
                (a.freq_hz - (center + off))
                    .abs()
                    .partial_cmp(&(b.freq_hz - (center + off)).abs())
                    .unwrap()
            })
            .unwrap();
        // The coarse filterbank resolves the carrier to ~a couple of bins; the
        // operator clicks and fine-tunes in the panel.
        assert!(
            (s.freq_hz - (center + off)).abs() < 4.0 * bin_tol(rate),
            "freq {} vs {}",
            s.freq_hz,
            center + off
        );
        // The confirmation gate must reject noise: a lone carrier yields a small
        // number of spots (ideally one), not a spray of false tracks.
        assert!(spots.len() <= 3, "too many spots (noise not rejected): {}", spots.len());
        // Coarse filterbank decode is best-effort text; clean copy happens in the
        // PSK panel after the operator clicks the spot to tune onto it.
        assert!(!s.text.is_empty(), "spot has no text");
    }

    fn bin_tol(rate: f64) -> f64 {
        rate / FFT_SIZE as f64 // ±1 bin
    }
}
