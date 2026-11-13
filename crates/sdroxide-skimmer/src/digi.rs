//! Wideband PSK31 / RTTY skimmer.
//!
//! Reuses the same streaming STFT as the CW skimmer, but treats the FFT as a
//! *filterbank*: each bin is a down-converted, decimated complex stream at the
//! frame rate (750 Hz over a 192 kHz window), so a per-track BPSK or FSK
//! decoder consumes bins directly — no per-track decimation. Detection finds
//! persistent carriers; decoding is gated to the digimode band segments.
//!
//! A bin is a coarse tap (47 Hz wide, ~24 samples per PSK31 symbol) whose
//! phase also steps with the hop, so a PSK track first takes out the hop's
//! phase advance and the carrier's place within the bin before its Costas
//! loop (in [`BpskCore`]) sees it — see [`PskDemod`]. Good enough to place
//! spots and read call-outs; the operator clicks a spot to decode it cleanly
//! in the PSK/RTTY panel.

use std::sync::Arc;

use rustfft::{Fft, FftPlanner};
use sdroxide_dsp::{BaudotRx, BpskCore, Complex32 as C32};
use sdroxide_types::{SkimmerKind, SkimmerSpot, is_psk_segment, is_rtty_segment};

use crate::callsign::find_callsign;

const FFT_SIZE: usize = 4096;
/// Small hop → a high frame rate (~750 Hz), giving ~24 samples/symbol for PSK
/// and ~16/bit for RTTY — enough for clean symbol timing on the bin filterbank.
const HOP: usize = 256;
/// Hops per transform window: a bin-centred tone's phase advances `k` turns
/// over this many frames, which is what [`PskDemod`]'s hop correction undoes.
const HOPS_PER_FFT: usize = FFT_SIZE / HOP;
const _: () =
    assert!(FFT_SIZE.is_multiple_of(HOP), "hop must divide the window for the phase correction");
const WARMUP: u32 = 80;
/// Detection threshold on the *smoothed* spectrum, above the (per-region) mean
/// noise floor (~7 dB of sustained energy). Smoothing already rejects noise
/// spikes, so a carrier need only stand steadily above the floor.
const SMOOTH_ON: f32 = 5.0;
/// Bins per region for the median noise-floor estimate (~3 kHz at 47 Hz/bin).
/// A per-region floor tracks the *local* noise level, so a locally noisy patch
/// of the band no longer lets noise or faint non-digimode signals cross a floor
/// set by the quieter rest of the window; the median is a noise bin (signals are
/// sparse and narrow) so a carrier can't inflate its own floor either.
const REGION_BINS: usize = 64;
/// Frames between noise-floor updates.
///
/// One full window of fresh samples: at 75%+ overlap, consecutive frames share
/// most of their input, so re-running the region medians every frame re-reads
/// the same noise over and over for an estimate that barely moves. Updating
/// once per `FFT_SIZE / HOP` frames costs a sixteenth as much *and* draws each
/// estimate from independent samples; [`sdroxide_dsp::noisefloor::stride_alpha`] keeps the
/// floor's time constant in seconds unchanged.
const FLOOR_EVERY: u32 = HOPS_PER_FFT as u32;
/// Envelope key-off ratio, for a track's activity/SNR metering.
const OFF_RATIO: f32 = 4.0;
/// Fraction of a carrier's own power a tone `shift` away must reach to count as
/// an RTTY mark/space pair (~-8 dB). Real RTTY tones are comparable; a strong
/// signal's window sidelobes are ~-30 dB, well below this.
const PAIR_FRAC: f32 = 0.15;
const PEAK_SPACING: usize = 6;
const TRACK_TOL: i64 = 2;
/// Guard band around the front end's DC spike, in bins — see
/// [`DigiSkimmer::set_dc_offset_hz`].
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

/// Smoothing of the squared-signal lag product behind [`PskDemod`]'s coarse
/// AFC, per frame (~40 ms at a 750 Hz frame rate — a symbol and a bit).
const AFC_SMOOTH: f32 = 1.0 / 32.0;

/// Per-track PSK31 demod over one filterbank bin.
///
/// Two offsets stand between a bin's output and a carrier [`BpskCore`] can
/// lock:
///
/// * **The hop.** Frame `m` transforms samples from `m·HOP` on, so a tone at
///   bin `k`'s centre comes out advanced by `2π·k·HOP/FFT_SIZE` per frame —
///   `k/16` of a turn here. Left in, that is `(k mod 16)` bin widths of
///   apparent offset (up to ±375 Hz); only carriers on every 16th bin
///   decoded. The skimmer takes it out before the sample gets here — see
///   [`DigiSkimmer::on_frame`].
/// * **Where in the bin the carrier sits.** Up to half a bin (±23 Hz) off the
///   centre, the rest is still several times what the Costas loop pulls in:
///   it derotates *before* its one-symbol matched filter, so a 20 Hz offset
///   turns most of a cycle across each symbol and the filter nulls the very
///   signal the loop needs to see. Squaring BPSK strips the ±1 modulation, so
///   the lag-1 product of the squared stream turns at twice the residual
///   regardless of the data; an NCO removes that, and the loop is left only
///   the last fraction of a hertz.
struct PskDemod {
    core: BpskCore,
    /// Previous frame's squared sample.
    sq_prev: C32,
    /// Smoothed `sq[m]·conj(sq[m-1])`; half its angle is the residual in
    /// radians per frame.
    acf: C32,
    /// Phase of the NCO that removes the residual.
    nco: f32,
}

impl PskDemod {
    fn new(sps: f32) -> Self {
        PskDemod {
            core: BpskCore::new(sps),
            sq_prev: C32::default(),
            acf: C32::default(),
            nco: 0.0,
        }
    }

    /// Residual carrier offset from the bin centre, in radians per frame.
    fn residual(&self) -> f32 {
        if self.acf.norm_sqr() > 0.0 { self.acf.arg() * 0.5 } else { 0.0 }
    }

    /// Feed one hop-corrected bin sample.
    fn push(&mut self, z: C32, out: &mut String) {
        let sq = z * z;
        self.acf += (sq * self.sq_prev.conj() - self.acf) * AFC_SMOOTH;
        self.sq_prev = sq;
        self.nco -= self.residual();
        if self.nco < -std::f32::consts::PI {
            self.nco += std::f32::consts::TAU;
        } else if self.nco > std::f32::consts::PI {
            self.nco -= std::f32::consts::TAU;
        }
        self.core.push(z * C32::from_polar(1.0, self.nco), out);
    }
}

enum Decoder {
    Psk(Box<PskDemod>),
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

/// Whether any part of a `rate`-wide window on `center_hz` is in a sub-segment
/// this skimmer's mode is called on.
///
/// The whole window at once, rather than a mask per bin as the CW skimmer
/// keeps: the per-bin test here is already done inside the candidate loop
/// ([`DigiSkimmer::on_frame`]), and what is missing is only the outer question
/// — is there any point running the transform at all.
fn any_segment(kind: SkimmerKind, rate: f64, center_hz: f64) -> bool {
    let bin_hz = rate / FFT_SIZE as f64;
    (0..FFT_SIZE).any(|k| {
        let off = if k <= FFT_SIZE / 2 { k as i64 } else { k as i64 - FFT_SIZE as i64 };
        let hz = center_hz + off as f64 * bin_hz;
        match kind {
            SkimmerKind::Rtty => is_rtty_segment(hz),
            _ => is_psk_segment(hz),
        }
    })
}

pub struct DigiSkimmer {
    kind: SkimmerKind,
    skim_rate: f64,
    skim_center_hz: f64,
    /// The operator's visible waterfall window in absolute Hz, or `None` for
    /// the whole skim window. See [`DigiSkimmer::set_view`].
    view: Option<(f64, f64)>,
    /// Where the front end's own DC spike sits in this window, as an offset in
    /// Hz from its centre. See [`DigiSkimmer::set_dc_offset_hz`].
    dc_off_hz: f64,
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
    /// Scratch for the per-region median floor estimate (one region's power,
    /// as bit patterns — see [`sdroxide_dsp::noisefloor::update_regions`]).
    pscratch: Vec<u32>,
    /// Per-bin noise floor (each region's median, scaled to a mean), smoothed.
    floor: Vec<f32>,
    cands: Vec<(f32, i64)>,
    centers: Vec<i64>,
    frame_ms: f32,
    frames: u32,
    /// The current frame's index mod [`HOPS_PER_FFT`] — its start sample is
    /// `index·HOP` into the stream, which sets each bin's phase advance.
    hop_index: usize,
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
    /// Whether this window holds any of the calling sub-bands this skimmer
    /// reads. False stands it down entirely — no transform, no detection.
    ///
    /// The candidate loop already refuses everything outside them, so off an
    /// amateur band every frame of the transform was work whose every result
    /// was thrown away: two of these run beside the CW skimmer, and on an
    /// RTL-SDR at 2.4 Msps they were the whole of the skimmer thread.
    any_seg: bool,
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
            view: None,
            dc_off_hz: 0.0,
            fft,
            window,
            inbuf: Vec::with_capacity(FFT_SIZE * 4),
            read_pos: 0,
            scratch: vec![C32::default(); FFT_SIZE],
            power: vec![0.0; FFT_SIZE],
            smooth_power: vec![0.0; FFT_SIZE],
            pscratch: Vec::with_capacity(REGION_BINS),
            floor: vec![0.0; FFT_SIZE],
            cands: Vec::with_capacity(512),
            centers: Vec::with_capacity(128),
            frame_ms: (HOP as f64 / skim_rate * 1000.0) as f32,
            frames: 0,
            hop_index: 0,
            now_ms: 0.0,
            tracks: Vec::new(),
            next_id: 1,
            confirm: Vec::new(),
            psk_sps: (frame_rate / PSK_BAUD) as f32,
            rtty_bit_len: (frame_rate / RTTY_BAUD) as f32,
            rtty_shift_bins: (RTTY_SHIFT / bin_hz).round() as i64,
            any_seg: any_segment(kind, skim_rate, skim_center_hz),
        }
    }

    pub fn set_center(&mut self, center_hz: f64) {
        if (center_hz - self.skim_center_hz).abs() > 1.0 {
            self.skim_center_hz = center_hz;
            self.any_seg = any_segment(self.kind, self.skim_rate, center_hz);
            self.reset();
        }
    }

    /// Limit the skimmer to the operator's visible waterfall window; see
    /// [`crate::cw::CwSkimmer::set_view`], which this mirrors. Carriers outside
    /// it are not confirmed, not tracked and not demodulated, and any track
    /// already outside is dropped along with its decoder.
    pub fn set_view(&mut self, view: Option<(f64, f64)>) {
        if self.view == view {
            return;
        }
        self.view = view;
        let Some((lo, hi)) = view else { return };
        let bin_hz = self.skim_rate / FFT_SIZE as f64;
        let center = self.skim_center_hz;
        let visible = |bin: i64| {
            let hz = center + bin as f64 * bin_hz;
            hz >= lo && hz <= hi
        };
        self.tracks.retain(|t| visible(t.bin));
        self.confirm.retain(|&(bin, _)| visible(bin));
    }

    /// Where the front end's own DC spike falls in this window, as an offset in
    /// Hz from the window centre; see [`crate::cw::CwSkimmer::set_dc_offset_hz`],
    /// which this mirrors.
    pub fn set_dc_offset_hz(&mut self, off_hz: f64) {
        self.dc_off_hz = off_hz;
    }

    /// Whether a bin offset from the skim center is on that spike. One outside
    /// this window never matches.
    fn at_dc(&self, off: i64) -> bool {
        let bin = (self.dc_off_hz / (self.skim_rate / FFT_SIZE as f64)).round() as i64;
        (off - bin).abs() < DC_GUARD
    }

    /// Whether an absolute frequency is on screen.
    fn in_view(&self, abs_hz: f64) -> bool {
        match self.view {
            Some((lo, hi)) => abs_hz >= lo && abs_hz <= hi,
            None => true,
        }
    }

    /// Forget every track and re-prime the noise floor (band moved, or the
    /// skimmer was switched off and its state is now stale).
    pub fn reset(&mut self) {
        self.tracks.clear();
        self.confirm.clear();
        self.inbuf.clear();
        self.read_pos = 0;
        self.frames = 0;
        self.hop_index = 0;
        for p in self.floor.iter_mut() {
            *p = 0.0;
        }
        for p in self.smooth_power.iter_mut() {
            *p = 0.0;
        }
    }

    pub fn process(&mut self, iq: &[C32]) {
        // Nowhere in this window is a sub-band this skimmer reads, so there is
        // nothing for the transform to find — see [`DigiSkimmer::any_seg`].
        // Ahead of the buffering, so a stand-down does not hand the band back
        // as a backlog when the operator tunes onto one.
        if !self.any_seg {
            return;
        }
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
            self.hop_index = (self.hop_index + 1) % HOPS_PER_FFT;
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
            _ => Decoder::Psk(Box::new(PskDemod::new(self.psk_sps))),
        }
    }

    fn on_frame(&mut self) {
        let n = FFT_SIZE;
        for k in 0..n {
            self.power[k] = self.scratch[k].norm_sqr();
        }
        // Robust *per-region* noise floor: the median of each ~64-bin region
        // (scaled to a mean), smoothed over time. A continuous carrier occupies a
        // tiny fraction of bins so it can't move its region's median and hide
        // itself; and because the floor is local, a noisy patch of the band no
        // longer inherits a too-low floor from the quieter rest of the window
        // (which let noise / faint non-digimode signals decode).
        if self.frames % FLOOR_EVERY == 0 {
            let smooth = if self.frames < WARMUP { 0.3 } else { 0.1 };
            sdroxide_dsp::noisefloor::update_regions(
                &mut self.floor,
                &self.power,
                &mut self.pscratch,
                REGION_BINS,
                sdroxide_dsp::noisefloor::stride_alpha(smooth, FLOOR_EVERY),
                self.frames == 0,
            );
        }
        if self.frames < WARMUP {
            return;
        }
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
            if self.at_dc(off) {
                continue;
            }
            if self.smooth_power[k] > self.floor[k] * SMOOTH_ON {
                let abs_hz = self.skim_center_hz + off as f64 * bin_hz;
                // Restrict each skimmer to its mode's well-known calling sub-bands
                // (PSK31 / RTTY areas per band) — not the whole digi segment —
                // and to what the operator can actually see.
                let in_band = match self.kind {
                    SkimmerKind::Rtty => is_rtty_segment(abs_hz),
                    _ => is_psk_segment(abs_hz),
                };
                if in_band && self.in_view(abs_hz) {
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
            let floor = self.floor[k].max(1e-12);
            t.env = 0.9 * t.env + 0.1 * self.power[k];
            if t.env > floor * OFF_RATIO {
                t.last_on_ms = now;
                t.hits = t.hits.saturating_add(1);
                t.snr_db = (10.0 * (t.env / floor).log10()).round().clamp(-30.0, 60.0) as i16;
            }
            match &mut t.dec {
                Decoder::Psk(demod) => {
                    // Undo the hop's phase advance, 2π·k·m·HOP/FFT_SIZE — see
                    // [`PskDemod`]. `k·m` taken mod the hops per window, so it
                    // stays exact however long the skimmer runs.
                    let turn = (k * self.hop_index) % HOPS_PER_FFT;
                    let ang = -std::f32::consts::TAU * turn as f32 / HOPS_PER_FFT as f32;
                    let mut s = String::new();
                    demod.push(self.scratch[k] * C32::from_polar(1.0, ang), &mut s);
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
                // The track sits on the carrier the detector locked, which for
                // RTTY is the MARK tone — half a shift above the middle of the
                // signal. Reported at that middle instead: it is where the box
                // belongs over a two-tone signal, and it is the frequency a
                // click has to put the receiver's own tone pair on. PSK has one
                // carrier and the two are the same thing.
                let pair_offset = match self.kind {
                    SkimmerKind::Rtty => RTTY_SHIFT / 2.0,
                    _ => 0.0,
                };
                // PSK knows where in its bin the carrier sits (the AFC's
                // residual), so the spot lands on the carrier, not the bin.
                let fine_hz = match &t.dec {
                    Decoder::Psk(d) => {
                        d.residual() as f64 / std::f64::consts::TAU * (self.skim_rate / HOP as f64)
                    }
                    Decoder::Rtty(_) => 0.0,
                };
                Some(SkimmerSpot {
                    id: t.id,
                    kind: self.kind,
                    freq_hz: self.skim_center_hz + t.bin as f64 * bin_hz + fine_hz - pair_offset,
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
        let floor_avg = self.floor.iter().sum::<f32>() / self.floor.len().max(1) as f32;
        eprintln!(
            "floor(avg)={:.2e} maxsmooth/floor={:.1} tracks={} confirm={:?}",
            floor_avg,
            mx / floor_avg.max(1e-12),
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
    /// The PSK/RTTY skimmer is gated to the visible window too — and clearing
    /// the view puts the whole skim window back.
    #[test]
    fn only_signals_inside_the_view_are_tracked() {
        let rate = 192_000.0;
        let center = 14_070_000.0;
        let off = 2_000.0;
        let iq = synth_psk("CQ CQ DE AB1CD K ", off, rate);

        let mut hidden = DigiSkimmer::new(SkimmerKind::Psk, rate, center);
        hidden.set_view(Some((center + 20_000.0, center + 60_000.0)));
        for chunk in iq.chunks(8192) {
            hidden.process(chunk);
        }
        assert!(hidden.spots().is_empty(), "spotted a carrier outside the view");

        let mut shown = DigiSkimmer::new(SkimmerKind::Psk, rate, center);
        shown.set_view(Some((center + 20_000.0, center + 60_000.0)));
        shown.set_view(None);
        for chunk in iq.chunks(8192) {
            shown.process(chunk);
        }
        assert!(!shown.spots().is_empty(), "no spots with the view cleared");
    }

    /// A window with none of the mode's calling sub-bands in it does no work at
    /// all — the candidate loop would refuse everything the transform found, so
    /// the transform itself is what has to stop.
    #[test]
    fn a_window_with_no_calling_sub_band_skims_nothing() {
        let rate = 192_000.0;
        let off = 2_000.0;
        let iq = synth_psk("CQ CQ DE AB1CD K ", off, rate);

        // The control: the same signal in the 20 m PSK area.
        let mut ham = DigiSkimmer::new(SkimmerKind::Psk, rate, 14_070_000.0);
        for chunk in iq.chunks(8192) {
            ham.process(chunk);
        }
        assert!(!ham.spots().is_empty(), "the control case decoded nothing");

        let mut fm = DigiSkimmer::new(SkimmerKind::Psk, rate, 96_300_000.0);
        for chunk in iq.chunks(8192) {
            fm.process(chunk);
        }
        assert!(fm.spots().is_empty(), "spotted a carrier on the broadcast FM band");

        // And the stand-down travels with the window.
        fm.set_center(14_070_000.0);
        for chunk in iq.chunks(8192) {
            fm.process(chunk);
        }
        assert!(!fm.spots().is_empty(), "nothing decoded after moving onto 20 m");
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
        // Real copy, not just *some* text: varicode prints letters off noise, so
        // a non-empty check passed while this decoded garbage (#822).
        assert!(s.text.contains("AB1CD"), "garbled copy: {:?}", s.text);
    }

    /// A carrier anywhere decodes: on bins whose index is and isn't a multiple
    /// of the hops per window (the hop's phase advance, #822), and anywhere
    /// within a bin (the residual the AFC takes out) — not just on every 16th
    /// bin centre. The spot also lands on the carrier, not its bin centre.
    #[test]
    fn decodes_psk_at_any_offset() {
        let rate = 192_000.0;
        let center = 14_070_000.0;
        let bin_hz = rate / FFT_SIZE as f64;
        let mut bad = Vec::new();
        // 1.37 bins apart: walks through every k mod 16 and spread of in-bin
        // fractions, from a bin centre to half a bin off.
        for j in 0..13 {
            let off = 1500.0 + j as f64 * 1.37 * bin_hz;
            let iq = synth_psk("CQ CQ DE AB1CD AB1CD K ", off, rate);
            let mut sk = DigiSkimmer::new(SkimmerKind::Psk, rate, center);
            for chunk in iq.chunks(8192) {
                sk.process(chunk);
            }
            let hit = sk.spots().into_iter().find(|s| s.text.contains("AB1CD"));
            // Within a few Hz rather than exact: idling PSK31 is two tones
            // ±15.6 Hz, and a carrier near a bin's edge has them weighted
            // unequally, pulling the estimate a little toward the bin centre.
            // Well inside what the panel's own AFC takes up on a click.
            match hit {
                Some(s) if (s.freq_hz - (center + off)).abs() < 6.0 => {}
                other => bad.push((off, other.map(|s| (s.freq_hz - center, s.text)))),
            }
        }
        assert!(bad.is_empty(), "offsets without clean copy at the carrier: {bad:?}");
    }

    #[test]
    fn floor_tracks_the_local_noise_level() {
        // Regression for "decodes noise / faint signals": the per-region floor
        // must follow the *local* noise level, so a noisy patch of the band gets
        // its own high floor instead of inheriting a too-low one from the quiet
        // rest of the window. Build band-limited noise that's loud only in
        // +30..+50 kHz and quiet elsewhere, and confirm the floor reflects it.
        let rate = 192_000.0;
        let center = 14_070_000.0;
        let bin_hz = rate / FFT_SIZE as f64;

        let mut seed = 0x1357_9bdfu32;
        let mut rng = || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / (1u32 << 24) as f32 - 0.5
        };
        // Construct one block's spectrum (low white noise everywhere, elevated in
        // the sub-band), IFFT to a band-limited-noise time block, then tile it.
        let mut spec = vec![C32::default(); FFT_SIZE];
        for (k, s) in spec.iter_mut().enumerate() {
            let off = if k <= FFT_SIZE / 2 { k as i64 } else { k as i64 - FFT_SIZE as i64 };
            let hz = off as f64 * bin_hz;
            let amp = if (30_000.0..50_000.0).contains(&hz) { 0.30 } else { 0.02 };
            *s = C32::new(amp * rng(), amp * rng());
        }
        rustfft::FftPlanner::<f32>::new().plan_fft_inverse(FFT_SIZE).process(&mut spec);
        let block = spec;
        let mut sk = DigiSkimmer::new(SkimmerKind::Psk, rate, center);
        for _ in 0..80 {
            sk.process(&block);
        }
        let loud = (40_000.0 / bin_hz) as usize; // +40 kHz (loud sub-band)
        let quiet = FFT_SIZE - (40_000.0 / bin_hz) as usize; // -40 kHz (quiet)
        assert!(
            sk.floor[loud] > 5.0 * sk.floor[quiet],
            "floor not local: loud {:.2e} vs quiet {:.2e}",
            sk.floor[loud],
            sk.floor[quiet]
        );
    }

    fn bin_tol(rate: f64) -> f64 {
        rate / FFT_SIZE as f64 // ±1 bin
    }
}
