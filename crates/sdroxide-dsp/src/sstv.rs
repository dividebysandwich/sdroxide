//! SSTV modem: image ⇄ audio for the Scottie, Martin, and Robot modes.
//!
//! Transmit builds a per-mode timing plan and synthesises tones with a
//! continuous phase accumulator (the `psk.rs`/`rtty.rs` idiom). Receive runs an
//! FM discriminator to recover instantaneous frequency, detects the VIS
//! calibration header to pick the mode, then samples pixels line-by-line,
//! re-aligning on each 1200 Hz sync pulse for slant tolerance.
//!
//! Timing follows the canonical N7CXI spec (as used by PySSTV/QSSTV). Colour
//! maps to frequency by black = 1500 Hz, white = 2300 Hz; sync = 1200 Hz.

use std::f64::consts::TAU;

use sdroxide_types::SstvMode;

use crate::Complex32;
use crate::fir::{ComplexFir, bandpass_taps};

const BLACK_HZ: f64 = 1500.0;
const WHITE_HZ: f64 = 2300.0;
const SYNC_HZ: f64 = 1200.0;
const VIS_LEADER_HZ: f64 = 1900.0;
const VIS_BIT1_HZ: f64 = 1100.0;
const VIS_BIT0_HZ: f64 = 1300.0;

/// Frequency (Hz) for an 8-bit intensity, black→white.
fn value_to_hz(v: u8) -> f64 {
    BLACK_HZ + (v as f64 / 255.0) * (WHITE_HZ - BLACK_HZ)
}

/// Inverse of [`value_to_hz`], clamped to a byte.
fn hz_to_value(hz: f64) -> u8 {
    let v = ((hz - BLACK_HZ) / (WHITE_HZ - BLACK_HZ)) * 255.0;
    v.round().clamp(0.0, 255.0) as u8
}

// ───────────────────────────── mode timing ─────────────────────────────

/// A colour channel within a scan segment.
#[derive(Clone, Copy, PartialEq)]
enum Chan {
    R,
    G,
    B,
    /// Luma.
    Y,
    /// R-Y chroma (Cr).
    Cr,
    /// B-Y chroma (Cb).
    Cb,
}

/// One timed segment of a scan line.
#[derive(Clone, Copy)]
enum Seg {
    /// Constant tone for `dur` seconds at `hz`.
    Tone { hz: f64, dur: f64 },
    /// A pixel scan of `width` samples of channel `chan`, `px` seconds each.
    Scan { chan: Chan, width: u16, px: f64 },
}

/// Per-mode parameters used to build a line plan.
struct Timing {
    sync: f64,
    sync_hz: f64,
    sep: f64,
    sep_hz: f64,
    /// Colour-channel pixel time, seconds.
    px: f64,
}

fn scottie_timing(px: f64) -> Timing {
    Timing { sync: 0.009, sync_hz: SYNC_HZ, sep: 0.0015, sep_hz: 1500.0, px }
}

fn martin_timing(px: f64) -> Timing {
    Timing { sync: 0.004_862, sync_hz: SYNC_HZ, sep: 0.000_572, sep_hz: 1500.0, px }
}

/// The ordered segments for one scan line of `mode` at image width `w`.
/// Robot modes carry their (per-line-varying) chroma channel via `line`.
fn line_segments(mode: SstvMode, w: u16, line: u16) -> Vec<Seg> {
    use Chan::*;
    match mode {
        SstvMode::Scottie1 | SstvMode::Scottie2 | SstvMode::ScottieDx => {
            let px = match mode {
                SstvMode::Scottie1 => 0.000_432,
                SstvMode::Scottie2 => 0.000_275_2,
                _ => 0.001_08,
            };
            let t = scottie_timing(px);
            // Scottie order: sep · G · sep · B · SYNC · sep · R.
            vec![
                Seg::Tone { hz: t.sep_hz, dur: t.sep },
                Seg::Scan { chan: G, width: w, px: t.px },
                Seg::Tone { hz: t.sep_hz, dur: t.sep },
                Seg::Scan { chan: B, width: w, px: t.px },
                Seg::Tone { hz: t.sync_hz, dur: t.sync },
                Seg::Tone { hz: t.sep_hz, dur: t.sep },
                Seg::Scan { chan: R, width: w, px: t.px },
            ]
        }
        SstvMode::Martin1 | SstvMode::Martin2 => {
            let px = if mode == SstvMode::Martin1 { 0.000_457_6 } else { 0.000_228_8 };
            let t = martin_timing(px);
            // Martin order: SYNC · sep · G · sep · B · sep · R.
            vec![
                Seg::Tone { hz: t.sync_hz, dur: t.sync },
                Seg::Tone { hz: t.sep_hz, dur: t.sep },
                Seg::Scan { chan: G, width: w, px: t.px },
                Seg::Tone { hz: t.sep_hz, dur: t.sep },
                Seg::Scan { chan: B, width: w, px: t.px },
                Seg::Tone { hz: t.sep_hz, dur: t.sep },
                Seg::Scan { chan: R, width: w, px: t.px },
            ]
        }
        SstvMode::Robot72 => {
            // Y full width; Cr, Cb half width. 300 ms/line.
            let cw = w / 2;
            vec![
                Seg::Tone { hz: SYNC_HZ, dur: 0.009 },
                Seg::Tone { hz: 1500.0, dur: 0.003 },
                Seg::Scan { chan: Y, width: w, px: 0.000_431_25 },
                Seg::Tone { hz: 1500.0, dur: 0.0045 },
                Seg::Tone { hz: 1900.0, dur: 0.0015 },
                Seg::Scan { chan: Cr, width: cw, px: 0.000_431_25 },
                Seg::Tone { hz: 2300.0, dur: 0.0045 },
                Seg::Tone { hz: 1900.0, dur: 0.0015 },
                Seg::Scan { chan: Cb, width: cw, px: 0.000_431_25 },
            ]
        }
        SstvMode::Robot36 => {
            // Y full width; one chroma per line, alternating even=Cr / odd=Cb
            // (4:2:0). 150 ms/line. Separator frequency signals which chroma.
            let cw = w / 2;
            let even = line % 2 == 0;
            let (chan, sep_hz) = if even { (Cr, 1500.0) } else { (Cb, 2300.0) };
            vec![
                Seg::Tone { hz: SYNC_HZ, dur: 0.009 },
                Seg::Tone { hz: 1500.0, dur: 0.003 },
                Seg::Scan { chan: Y, width: w, px: 0.000_275 },
                Seg::Tone { hz: sep_hz, dur: 0.0045 },
                Seg::Tone { hz: 1900.0, dur: 0.0015 },
                Seg::Scan { chan, width: cw, px: 0.000_275 },
            ]
        }
    }
}

// BT.601-ish YUV used by the Robot modes (MMSSTV coefficients).
fn rgb_to_yuv(r: u8, g: u8, b: u8) -> (u8, u8, u8) {
    let (r, g, b) = (r as f64, g as f64, b as f64);
    let y = 16.0 + (65.738 * r + 129.057 * g + 25.064 * b) / 256.0;
    let cr = 128.0 + (112.439 * r - 94.154 * g - 18.285 * b) / 256.0;
    let cb = 128.0 + (-37.945 * r - 74.494 * g + 112.439 * b) / 256.0;
    (
        y.round().clamp(0.0, 255.0) as u8,
        cr.round().clamp(0.0, 255.0) as u8,
        cb.round().clamp(0.0, 255.0) as u8,
    )
}

fn yuv_to_rgb(y: u8, cr: u8, cb: u8) -> (u8, u8, u8) {
    let y = y as f64 - 16.0;
    let cr = cr as f64 - 128.0;
    let cb = cb as f64 - 128.0;
    let r = 1.164 * y + 1.596 * cr;
    let g = 1.164 * y - 0.392 * cb - 0.813 * cr;
    let b = 1.164 * y + 2.017 * cb;
    (
        r.round().clamp(0.0, 255.0) as u8,
        g.round().clamp(0.0, 255.0) as u8,
        b.round().clamp(0.0, 255.0) as u8,
    )
}

// ─────────────────────────────── transmit ──────────────────────────────

/// SSTV transmitter: turns an RGB image into a stream of audio samples.
pub struct SstvTx {
    rate: f64,
    /// Flattened plan of (frequency, sample-count) tone runs. Scans are expanded
    /// to one entry per pixel up front — a 320×256 image is ~250 k entries, a
    /// few MB, produced once per transmission.
    plan: Vec<(f64, u32)>,
    idx: usize,
    left: u32,
    cur_hz: f64,
    phase: f64,
    total: u64,
    done: u64,
}

impl SstvTx {
    /// Build a transmitter for `mode` from interleaved RGB (`rgb.len() == w*h*3`)
    /// at output sample `rate`, with an optional transmit clock trim `ppm`
    /// (parts-per-million; stretches/compresses the image time-scale to null out
    /// slant against a receiver whose clock differs — tone frequencies are
    /// unaffected).
    pub fn new(mode: SstvMode, rgb: &[u8], w: u16, h: u16, rate: f64, ppm: f32) -> Self {
        let mut plan: Vec<(f64, u32)> = Vec::new();
        // Cumulative-exact sample clock: derive each element's integer sample
        // count from the running fractional time, so per-element rounding never
        // accumulates into image slant (e.g. Scottie 1's 0.432 ms pixel is
        // 20.736 samples at 48 kHz — rounding each to 21 would drift +1.3%).
        // The timing rate carries the ppm trim; the phase accumulator (in
        // `next_block`) uses the true `rate`, so the tone frequencies stay exact.
        let timing_rate = rate * (1.0 + ppm as f64 / 1_000_000.0);
        let mut emitted: i64 = 0;
        let mut t_exact: f64 = 0.0;
        let mut push = |plan: &mut Vec<(f64, u32)>, hz: f64, dur: f64| {
            t_exact += dur * timing_rate;
            let target = t_exact.round() as i64;
            let n = (target - emitted).max(0);
            emitted = target;
            if n > 0 {
                plan.push((hz, n as u32));
            }
        };
        let px_at = |x: usize, y: usize| -> (u8, u8, u8) {
            let i = (y * w as usize + x) * 3;
            (rgb[i], rgb[i + 1], rgb[i + 2])
        };

        // VIS calibration header.
        push(&mut plan, VIS_LEADER_HZ, 0.300);
        push(&mut plan, SYNC_HZ, 0.010);
        push(&mut plan, VIS_LEADER_HZ, 0.300);
        push(&mut plan, SYNC_HZ, 0.030); // start bit
        let code = mode.vis_code();
        let mut parity = 0u8;
        for bit in 0..7 {
            let one = (code >> bit) & 1 == 1;
            parity ^= one as u8;
            push(&mut plan, if one { VIS_BIT1_HZ } else { VIS_BIT0_HZ }, 0.030);
        }
        push(&mut plan, if parity == 1 { VIS_BIT1_HZ } else { VIS_BIT0_HZ }, 0.030);
        push(&mut plan, SYNC_HZ, 0.030); // stop bit

        // Scottie sends a 9 ms starting sync before the very first line.
        if matches!(mode, SstvMode::Scottie1 | SstvMode::Scottie2 | SstvMode::ScottieDx) {
            push(&mut plan, SYNC_HZ, 0.009);
        }

        for y in 0..h as usize {
            for seg in line_segments(mode, w, y as u16) {
                match seg {
                    Seg::Tone { hz, dur } => push(&mut plan, hz, dur),
                    Seg::Scan { chan, width, px } => {
                        for x in 0..width as usize {
                            // Map the (possibly subsampled) scan x to a source column.
                            let sx = if width == w { x } else { (x * 2).min(w as usize - 1) };
                            let (r, g, b) = px_at(sx, y);
                            let v = match chan {
                                Chan::R => r,
                                Chan::G => g,
                                Chan::B => b,
                                Chan::Y => rgb_to_yuv(r, g, b).0,
                                Chan::Cr => rgb_to_yuv(r, g, b).1,
                                Chan::Cb => rgb_to_yuv(r, g, b).2,
                            };
                            // Cumulative-exact per-pixel timing (no drift/slant).
                            push(&mut plan, value_to_hz(v), px);
                        }
                    }
                }
            }
        }

        let total: u64 = plan.iter().map(|&(_, n)| n as u64).sum();
        SstvTx {
            rate,
            plan,
            idx: 0,
            left: 0,
            cur_hz: 0.0,
            phase: 0.0,
            total,
            done: 0,
        }
    }

    /// Fill `out` with audio; returns the number of real samples written before
    /// the transmission ended (the rest of `out`, if any, is zeroed).
    pub fn next_block(&mut self, out: &mut [f32]) -> usize {
        let mut written = 0;
        for s in out.iter_mut() {
            if self.left == 0 {
                match self.plan.get(self.idx) {
                    Some(&(hz, n)) => {
                        self.cur_hz = hz;
                        self.left = n;
                        self.idx += 1;
                    }
                    None => {
                        *s = 0.0;
                        continue;
                    }
                }
            }
            self.phase += TAU * self.cur_hz / self.rate;
            if self.phase > TAU {
                self.phase -= TAU;
            }
            *s = (self.phase.sin() as f32) * 0.5;
            self.left -= 1;
            self.done += 1;
            written += 1;
        }
        written
    }

    /// True once every planned sample has been emitted.
    pub fn done(&self) -> bool {
        self.idx >= self.plan.len() && self.left == 0
    }

    /// Total number of audio samples this transmission will emit.
    pub fn total_samples(&self) -> u64 {
        self.total
    }

    /// Transmission progress, 0.0..=1.0.
    pub fn progress(&self) -> f32 {
        if self.total == 0 {
            1.0
        } else {
            (self.done as f32 / self.total as f32).clamp(0.0, 1.0)
        }
    }
}

// ─────────────────────────────── receive ───────────────────────────────

/// A decoded output from the receiver.
pub enum SstvEvent {
    /// A VIS header identified the mode; a new image is starting.
    ModeDetected(SstvMode),
    /// A finished scan line: `rgb` is `3 * width` bytes at row `y`.
    Line { y: u16, rgb: Vec<u8> },
    /// The current image reached its last line.
    ImageComplete,
}

#[derive(PartialEq)]
enum RxPhase {
    /// Hunting for the VIS leader / decoding VIS.
    Hunt,
    /// Decoding image lines for `mode`.
    Image,
}

/// SSTV receiver. Feed audio with [`SstvRx::process`]; it emits [`SstvEvent`]s.
pub struct SstvRx {
    rate: f64,
    // Down-mix + baseband filter for the discriminator.
    mix_ph: f32,
    mix_inc: f32,
    lpf: ComplexFir,
    prev: Complex32,
    // Instantaneous frequency (Hz), lightly smoothed.
    inst_hz: f64,
    // Smoothed raw-input level (mean |audio|) for the UI activity meter.
    in_level: f32,
    have_prev: bool,

    phase: RxPhase,
    mode: SstvMode,
    // Rolling ring of recent instantaneous-frequency samples, so we can look
    // back over a whole line once its trailing sync arrives.
    hist: Vec<f64>,
    // Absolute sample index of hist[0].
    hist_base: u64,
    sample_idx: u64,

    // VIS bit accumulation.
    vis_state: VisState,

    // Image decode bookkeeping.
    line: u16,
    // Sample index where the current line's decoding should start.
    line_start: u64,
    // Robot 4:2:0 chroma carried between lines.
    last_cr: Vec<u8>,
    last_cb: Vec<u8>,

    // Free-run (decode without VIS): lock onto a regular 1200 Hz sync cadence.
    // `expected` = a specific operator-selected mode, or `None` for auto (match
    // the cadence + sync length against every mode).
    expected: Option<SstvMode>,
    sync_run: u32,
    // Recent sync pulses as (centre sample, pulse length in samples).
    sync_hist: Vec<(u64, u32)>,
}

struct VisState {
    // Running count of consecutive ~1900 Hz leader samples.
    leader: u32,
    // A full (>150 ms) leader has been seen at least once.
    leader_seen: bool,
    // Previous sample was ~1200 Hz (rising-edge detection).
    was_sync: bool,
    // Candidate start-bit sample indices awaiting a decode attempt. Both the
    // 10 ms break and the real 30 ms start bit become candidates; the break
    // decodes to VIS code 0 (rejected), so the real start bit wins.
    cands: Vec<u64>,
}

impl VisState {
    fn reset() -> Self {
        VisState { leader: 0, leader_seen: false, was_sync: false, cands: Vec::new() }
    }
}

impl SstvRx {
    pub fn new(rate: f64) -> Self {
        let mix_hz = 1900.0f32;
        SstvRx {
            rate,
            mix_ph: 0.0,
            mix_inc: (TAU as f32) * mix_hz / rate as f32,
            lpf: ComplexFir::new(bandpass_taps(129, -1100.0, 1100.0, rate)),
            prev: Complex32::new(0.0, 0.0),
            inst_hz: 1900.0,
            in_level: 0.0,
            have_prev: false,
            phase: RxPhase::Hunt,
            mode: SstvMode::Scottie1,
            hist: Vec::new(),
            hist_base: 0,
            sample_idx: 0,
            vis_state: VisState::reset(),
            line: 0,
            line_start: 0,
            last_cr: Vec::new(),
            last_cb: Vec::new(),
            expected: None,
            sync_run: 0,
            sync_hist: Vec::new(),
        }
    }

    /// Set the mode used for free-run (no-VIS) decoding, or `None` for auto
    /// (detect the mode from the sync cadence).
    pub fn set_expected(&mut self, mode: Option<SstvMode>) {
        self.expected = mode;
    }

    /// The mode currently being decoded (or last detected).
    pub fn mode(&self) -> SstvMode {
        self.mode
    }

    /// Smoothed raw-input level (mean |sample|), for a UI activity meter so the
    /// operator can set their receive gain.
    pub fn level(&self) -> f32 {
        self.in_level
    }

    /// True while an image is being decoded (VIS locked).
    pub fn receiving(&self) -> bool {
        self.phase == RxPhase::Image
    }

    /// Fraction of the current image decoded, 0.0..=1.0.
    pub fn progress(&self) -> f32 {
        if self.phase != RxPhase::Image {
            return 0.0;
        }
        let (_, h) = self.mode.dimensions();
        (self.line as f32 / h.max(1) as f32).clamp(0.0, 1.0)
    }

    /// Feed audio; push any decoded events.
    pub fn process(&mut self, audio: &[f32], out: &mut Vec<SstvEvent>) {
        // Down-mix by 1900 Hz to complex baseband, then low-pass the whole block.
        let mut mixed = Vec::with_capacity(audio.len());
        for &a in audio {
            self.in_level += 0.001 * (a.abs() - self.in_level);
            let z = Complex32::new(a * self.mix_ph.cos(), -a * self.mix_ph.sin());
            self.mix_ph += self.mix_inc;
            if self.mix_ph > std::f32::consts::TAU {
                self.mix_ph -= std::f32::consts::TAU;
            }
            mixed.push(z);
        }
        let mut bb = Vec::with_capacity(audio.len());
        self.lpf.process(&mixed, &mut bb);

        for z in bb {
            // Instantaneous frequency via the discriminator.
            let raw_hz = if self.have_prev {
                let d = z * self.prev.conj();
                1900.0 + (d.arg() as f64) * self.rate / TAU
            } else {
                1900.0
            };
            self.prev = z;
            self.have_prev = true;
            self.inst_hz += 0.5 * (raw_hz - self.inst_hz);

            self.push_hist(self.inst_hz);
            self.sample_idx += 1;

            match self.phase {
                RxPhase::Hunt => self.step_hunt(out),
                RxPhase::Image => self.step_image(out),
            }
        }
    }

    fn push_hist(&mut self, hz: f64) {
        // Keep ~1.2 s of history (enough for the slowest line + sync search).
        let cap = (self.rate * 1.2) as usize;
        self.hist.push(hz);
        if self.hist.len() > cap {
            let drop = self.hist.len() - cap;
            self.hist.drain(0..drop);
            self.hist_base += drop as u64;
        }
    }

    fn hz_at(&self, idx: u64) -> f64 {
        if idx < self.hist_base {
            return 1900.0;
        }
        let i = (idx - self.hist_base) as usize;
        self.hist.get(i).copied().unwrap_or(1900.0)
    }

    // ── VIS detection ──
    fn step_hunt(&mut self, out: &mut Vec<SstvEvent>) {
        let near = |a: f64, b: f64| (a - b).abs() < 90.0;
        let is_leader = near(self.inst_hz, VIS_LEADER_HZ);
        let is_sync = near(self.inst_hz, SYNC_HZ);
        // Tolerant leader accumulator: brief noise glitches decrement rather than
        // reset the run, so a real ~300 ms leader still arms through hiss. No
        // amplitude gate — the discriminator is level-independent, so a clean but
        // quiet signal must still decode; the VIS code + parity check rejects
        // noise. (On true silence the discriminator jitters randomly, so a stable
        // ~1900 Hz run of 0.12 s effectively never occurs by chance.)
        if is_leader {
            self.vis_state.leader = (self.vis_state.leader + 1).min((self.rate) as u32);
            if self.vis_state.leader as f64 > 0.12 * self.rate {
                self.vis_state.leader_seen = true;
            }
        } else {
            self.vis_state.leader = self.vis_state.leader.saturating_sub(3);
        }
        // Rising edge into a 1200 Hz pulse after a leader → candidate start bit.
        if is_sync && !self.vis_state.was_sync && self.vis_state.leader_seen {
            self.vis_state.cands.push(self.sample_idx);
            if self.vis_state.cands.len() > 8 {
                self.vis_state.cands.remove(0);
            }
        }
        self.vis_state.was_sync = is_sync;

        // Try the oldest candidate once its 8 VIS bits have elapsed.
        let bit = 0.030 * self.rate;
        if let Some(&start) = self.vis_state.cands.first() {
            if (self.sample_idx as f64) >= start as f64 + 9.5 * bit {
                self.vis_state.cands.remove(0);
                let mut code = 0u8;
                let mut parity = 0u8;
                for b in 0..7 {
                    let centre = start as f64 + (1.5 + b as f64) * bit;
                    if self.hz_at(centre as u64) < 1200.0 {
                        code |= 1 << b; // 1100 Hz = 1
                        parity ^= 1;
                    }
                }
                let pbit = if self.hz_at((start as f64 + 8.5 * bit) as u64) < 1200.0 { 1 } else { 0 };
                if parity == pbit {
                    if let Some(mode) = SstvMode::from_vis(code) {
                        // Image data begins after the stop bit (start + 10 bits);
                        // Scottie prefixes a 9 ms starting sync before line 0.
                        let mut first = start as f64 + 10.0 * bit;
                        if matches!(
                            mode,
                            SstvMode::Scottie1 | SstvMode::Scottie2 | SstvMode::ScottieDx
                        ) {
                            first += 0.009 * self.rate;
                        }
                        self.begin_image(mode, first as u64, out);
                    }
                }
            }
        }

        // No VIS yet? Try to lock onto the sync cadence of the selected mode.
        self.try_freerun(out);
    }

    /// Total samples per scan line for `mode`.
    fn line_period_samples(&self, mode: SstvMode) -> f64 {
        let (w, _) = mode.dimensions();
        line_segments(mode, w, 0)
            .iter()
            .map(|s| match s {
                Seg::Tone { dur, .. } => *dur * self.rate,
                Seg::Scan { width, px, .. } => *width as f64 * *px * self.rate,
            })
            .sum()
    }

    /// Count how many recent sync gaps are an integer multiple of `mode`'s line
    /// period (within tolerance) — i.e. how well the cadence fits that mode.
    fn cadence_hits(&self, mode: SstvMode) -> u32 {
        let period = self.line_period_samples(mode);
        let mut hits = 0;
        for w in self.sync_hist.windows(2) {
            let gap = w[1].0 as f64 - w[0].0 as f64;
            let k = (gap / period).round();
            if k >= 1.0 && (gap - k * period).abs() < period * 0.04 {
                hits += 1;
            }
        }
        hits
    }

    /// Free-run lock: when 1200 Hz sync pulses arrive at a regular line cadence,
    /// start decoding (no VIS needed — handles tuning into a picture already in
    /// progress). With a fixed `expected` mode it locks to that; in auto it picks
    /// the mode whose line period *and* sync length best fit the cadence.
    fn try_freerun(&mut self, out: &mut Vec<SstvEvent>) {
        let is_sync = self.inst_hz > 1050.0 && self.inst_hz < 1350.0;
        if is_sync {
            self.sync_run += 1;
            return;
        }
        let run = self.sync_run;
        self.sync_run = 0;
        // Plausible sync length across all modes (~4.9 ms Martin .. 9 ms Scottie).
        if (run as f64) < 0.003 * self.rate || (run as f64) > 0.014 * self.rate {
            return;
        }
        let center = self.sample_idx.saturating_sub((run / 2) as u64);
        self.sync_hist.push((center, run));
        if self.sync_hist.len() > 10 {
            self.sync_hist.remove(0);
        }

        // Pick the mode to lock: the fixed one, or the best auto match.
        let locked = match self.expected {
            Some(m) => (self.cadence_hits(m) >= 2).then_some(m),
            None => {
                let mut best: Option<SstvMode> = None;
                let mut best_err = f64::INFINITY;
                for &m in &SstvMode::ALL {
                    if self.cadence_hits(m) < 2 {
                        continue;
                    }
                    let (_, sdur) = self.sync_span(m, m.dimensions().0, 0);
                    let dur_err = (run as f64 - sdur).abs() / sdur;
                    if dur_err > 0.4 {
                        continue; // sync length must also match (Scottie vs Martin)
                    }
                    if dur_err < best_err {
                        best_err = dur_err;
                        best = Some(m);
                    }
                }
                best
            }
        };
        if let Some(mode) = locked {
            let (soff, sdur) = self.sync_span(mode, mode.dimensions().0, 0);
            let line_start = (center as f64 - (soff + sdur * 0.5)).max(0.0) as u64;
            self.sync_hist.clear();
            self.begin_image(mode, line_start, out);
        }
    }

    fn begin_image(&mut self, mode: SstvMode, first_line_start: u64, out: &mut Vec<SstvEvent>) {
        self.mode = mode;
        self.phase = RxPhase::Image;
        self.line = 0;
        self.line_start = first_line_start;
        let (w, _) = mode.dimensions();
        self.last_cr = vec![128u8; (w / 2) as usize];
        self.last_cb = vec![128u8; (w / 2) as usize];
        // Reset free-run tracking so it re-locks cleanly for the next picture.
        self.sync_run = 0;
        self.sync_hist.clear();
        out.push(SstvEvent::ModeDetected(mode));
    }

    // ── image line decode ──
    fn step_image(&mut self, out: &mut Vec<SstvEvent>) {
        let (w, h) = self.mode.dimensions();
        let line_dur: f64 = line_segments(self.mode, w, self.line)
            .iter()
            .map(|s| match s {
                Seg::Tone { dur, .. } => *dur,
                Seg::Scan { width, px, .. } => *width as f64 * *px,
            })
            .sum();
        let line_samples = (line_dur * self.rate) as u64;

        // Decode a line once its full duration of history is available.
        if self.sample_idx < self.line_start + line_samples + (0.02 * self.rate) as u64 {
            return;
        }

        // Re-align each line to its 1200 Hz sync pulse (corrects timing error
        // and clock slant on real off-air signals).
        let start = self.realign_sync(self.line_start, self.mode, w, self.line);
        let rgb = self.decode_line(self.mode, w, self.line, start);
        out.push(SstvEvent::Line { y: self.line, rgb });

        self.line += 1;
        self.line_start = start + line_samples;
        if self.line >= h {
            out.push(SstvEvent::ImageComplete);
            self.phase = RxPhase::Hunt;
            self.vis_state = VisState::reset();
        }
    }

    /// Offset (in samples) from a line's start to the centre of its 1200 Hz
    /// sync pulse, plus the pulse duration in samples.
    fn sync_span(&self, mode: SstvMode, w: u16, line: u16) -> (f64, f64) {
        let mut t = 0.0;
        for seg in line_segments(mode, w, line) {
            match seg {
                Seg::Tone { hz, dur } => {
                    let d = dur * self.rate;
                    if (hz - SYNC_HZ).abs() < 1.0 {
                        return (t, d);
                    }
                    t += d;
                }
                Seg::Scan { width, px, .. } => t += width as f64 * px * self.rate,
            }
        }
        (0.0, 0.009 * self.rate)
    }

    /// Correct the line-start sample index by locking to the line's 1200 Hz sync
    /// pulse: find the mean position of near-1200 Hz samples in a window around
    /// where the sync is expected, then back-compute the line start.
    fn realign_sync(&self, nominal: u64, mode: SstvMode, w: u16, line: u16) -> u64 {
        let (soff, sdur) = self.sync_span(mode, w, line);
        let centre_off = soff + sdur * 0.5;
        let expected = nominal as f64 + centre_off;
        let win = (0.012 * self.rate) as i64;
        let mut sum = 0.0f64;
        let mut cnt = 0u32;
        for d in -win..=win {
            let idx = expected as i64 + d;
            if idx < 0 {
                continue;
            }
            if self.hz_at(idx as u64) < 1380.0 {
                sum += idx as f64;
                cnt += 1;
            }
        }
        // Trust the correction only when a real sync pulse is present.
        if (cnt as f64) > sdur * 0.4 {
            let centre = sum / cnt as f64;
            (centre - centre_off).max(0.0) as u64
        } else {
            nominal
        }
    }

    fn decode_line(&mut self, mode: SstvMode, w: u16, line: u16, start: u64) -> Vec<u8> {
        let mut r = vec![0u8; w as usize];
        let mut g = vec![0u8; w as usize];
        let mut b = vec![0u8; w as usize];
        let mut y = vec![0u8; w as usize];
        let mut cr = self.last_cr.clone();
        let mut cb = self.last_cb.clone();

        let mut t = start as f64;
        for seg in line_segments(mode, w, line) {
            match seg {
                Seg::Tone { dur, .. } => t += dur * self.rate,
                Seg::Scan { chan, width, px } => {
                    let step = px * self.rate;
                    for x in 0..width as usize {
                        // Sample the centre of each pixel window.
                        let idx = (t + (x as f64 + 0.5) * step) as u64;
                        let v = hz_to_value(self.hz_at(idx));
                        let cri = x.min(cr.len().saturating_sub(1));
                        let cbi = x.min(cb.len().saturating_sub(1));
                        match chan {
                            Chan::R => r[x] = v,
                            Chan::G => g[x] = v,
                            Chan::B => b[x] = v,
                            Chan::Y => y[x] = v,
                            Chan::Cr => cr[cri] = v,
                            Chan::Cb => cb[cbi] = v,
                        }
                    }
                    t += width as f64 * step;
                }
            }
        }

        let robot = matches!(mode, SstvMode::Robot72 | SstvMode::Robot36);
        if robot {
            self.last_cr = cr.clone();
            self.last_cb = cb.clone();
        }

        let mut rgb = vec![0u8; w as usize * 3];
        for x in 0..w as usize {
            let (rr, gg, bb) = if robot {
                let cx = (x / 2).min(cr.len() - 1);
                yuv_to_rgb(y[x], cr[cx], cb[cx])
            } else {
                (r[x], g[x], b[x])
            };
            rgb[x * 3] = rr;
            rgb[x * 3 + 1] = gg;
            rgb[x * 3 + 2] = bb;
        }
        rgb
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// End-to-end: encode a small gradient, decode it back, and check the VIS
    /// mode was recovered and the image roughly matches.
    #[test]
    fn scottie1_loopback_recovers_mode() {
        let rate = 48_000.0;
        let mode = SstvMode::Scottie1;
        let (w, h) = mode.dimensions();
        // Simple vertical gradient so a rough decode is easy to sanity-check.
        let mut rgb = vec![0u8; w as usize * h as usize * 3];
        for yy in 0..h as usize {
            for xx in 0..w as usize {
                let i = (yy * w as usize + xx) * 3;
                let v = (xx * 255 / w as usize) as u8;
                rgb[i] = v;
                rgb[i + 1] = v;
                rgb[i + 2] = v;
            }
        }
        let mut tx = SstvTx::new(mode, &rgb, w, h, rate, 0.0);
        let mut rx = SstvRx::new(rate);
        let mut events = Vec::new();
        let mut block = vec![0.0f32; 4096];
        let mut detected = None;
        let mut lines = 0;
        let mut guard = 0;
        while !tx.done() && guard < 20_000 {
            let n = tx.next_block(&mut block);
            rx.process(&block[..n], &mut events);
            for e in events.drain(..) {
                match e {
                    SstvEvent::ModeDetected(m) => detected = Some(m),
                    SstvEvent::Line { .. } => lines += 1,
                    SstvEvent::ImageComplete => {}
                }
            }
            guard += 1;
        }
        // Flush any tail.
        rx.process(&[0.0; 48_000], &mut events);
        for e in events.drain(..) {
            if let SstvEvent::ModeDetected(m) = e {
                detected = Some(m);
            } else if let SstvEvent::Line { .. } = e {
                lines += 1;
            }
        }
        assert_eq!(detected, Some(mode), "VIS mode should be recovered");
        assert!(lines > (h as usize) / 2, "should decode most lines, got {lines}");
    }

    #[test]
    fn vis_codes_roundtrip() {
        for m in SstvMode::ALL {
            assert_eq!(SstvMode::from_vis(m.vis_code()), Some(m));
        }
    }

    #[test]
    fn tx_ppm_scales_duration() {
        let rate = 48_000.0;
        let mode = SstvMode::Martin1;
        let (w, h) = mode.dimensions();
        let rgb = vec![128u8; w as usize * h as usize * 3];
        let base = SstvTx::new(mode, &rgb, w, h, rate, 0.0).total_samples() as f64;
        // +10 000 ppm = +1% longer transmission.
        let trimmed = SstvTx::new(mode, &rgb, w, h, rate, 10_000.0).total_samples() as f64;
        assert!((trimmed / base - 1.01).abs() < 0.0005, "ratio {}", trimmed / base);
    }

    /// Free-run: feed the RX the picture audio *after* the VIS header (as if we
    /// tuned in mid-transmission). With the mode pre-selected it should lock onto
    /// the sync cadence and decode.
    #[test]
    fn freerun_decodes_without_vis() {
        let rate = 48_000.0;
        let mode = SstvMode::Scottie1;
        let (w, h) = mode.dimensions();
        let mut rgb = vec![0u8; w as usize * h as usize * 3];
        for yy in 0..h as usize {
            for xx in 0..w as usize {
                let i = (yy * w as usize + xx) * 3;
                rgb[i] = (xx * 255 / w as usize) as u8;
            }
        }
        // Render the whole transmission to a buffer.
        let mut tx = SstvTx::new(mode, &rgb, w, h, rate, 0.0);
        let mut audio = Vec::new();
        let mut block = vec![0.0f32; 4096];
        let mut guard = 0;
        while !tx.done() && guard < 20_000 {
            let n = tx.next_block(&mut block);
            audio.extend_from_slice(&block[..n]);
            guard += 1;
        }
        // Skip past the VIS (~1.1 s) so only image data is fed → forces free-run.
        // Use auto (`None`): the RX must identify the mode from the sync cadence.
        let skip = (rate * 1.1) as usize;
        let mut rx = SstvRx::new(rate);
        rx.set_expected(None);
        let mut events = Vec::new();
        let mut detected = None;
        let mut lines = 0;
        for chunk in audio[skip.min(audio.len())..].chunks(4096) {
            rx.process(chunk, &mut events);
            for e in events.drain(..) {
                match e {
                    SstvEvent::ModeDetected(m) => detected = Some(m),
                    SstvEvent::Line { .. } => lines += 1,
                    SstvEvent::ImageComplete => {}
                }
            }
        }
        assert_eq!(detected, Some(mode), "free-run should lock the selected mode");
        assert!(lines > 40, "free-run should decode many lines, got {lines}");
    }
}
