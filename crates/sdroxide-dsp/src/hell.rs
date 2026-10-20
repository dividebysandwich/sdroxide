//! Hellschreiber — the facsimile "dot" mode, and the oldest one still in
//! regular amateur use.
//!
//! There is no sync, no framing and no error correction. The transmitter scans a
//! 7×14 character cell column by column, top to bottom, switching the carrier on
//! and off (the Feld Hell family) or shifting it (the FSK family). The receiver
//! free-runs at the same dot rate and paints what it measures. Reading it is the
//! operator's eye's job, not a decoder's — which is why the vertical phase
//! floats and why the display stacks two copies of every column.
//!
//! That absence of sync shapes the whole design here:
//!
//! * Both ends keep a *cumulative-exact* dot clock (the [`crate::sstv`] idiom),
//!   because nothing downstream will ever re-align a drifting one.
//! * The receiver's per-dot detector takes the **peak** rather than the mean:
//!   its window sits at an arbitrary phase against the transmitted dot, and
//!   averaging would smear a lone dot across two rows.
//! * There is no "decode" entry point. [`HellRx::process`] emits grays.

use std::collections::VecDeque;
use std::f64::consts::{PI, TAU};

use sdroxide_types::HellVariant;

use crate::Complex32;
use crate::fir::{ComplexFir, bandpass_taps};
use crate::hell_font::{COLS, ROWS, glyph};

/// Dot rows in one scanned column.
pub const HELL_ROWS: usize = ROWS;
/// Columns per character cell, the last being the inter-character gap.
pub const HELL_CELL_COLS: usize = COLS;

/// Peak transmit amplitude, matching `RttyTx`/`SstvTx` so switching modes does
/// not change the drive the operator has set.
const TX_PEAK: f32 = 0.5;

/// Rise/fall of the whole burst, in seconds. Independent of the dot ramp: the
/// FSK variants hold a constant-amplitude carrier, so without this they would
/// click hard at key-up and key-down.
const GATE_SECS: f64 = 0.005;

/// Sub-dot slices the receiver decimates to. Eight is enough to find the peak of
/// a raised-cosine dot at any phase without spending much on the FIR.
const RX_SLICES: usize = 8;

/// Receive filter length at the decimated rate. Affordable only *because* of the
/// decimation — the same selectivity at 48 kHz would need several hundred taps.
const RX_TAPS: usize = 65;

/// Dot columns the receive AGC's peak decays over.
const AGC_COLUMNS: f32 = 8.0;

/// Print sync: how the receiver finds where the sender's character cell starts.
///
/// Hell carries no frame sync, so a receiver that counts dots from whenever it
/// was started paints the cell at an arbitrary phase and every glyph is split
/// across the join between two print copies. The signal carries the answer
/// anyway: every on-air font leaves a blank pixel above and below the glyph, and
/// because the bottom padding is sent first and the top padding last, those two
/// blanks are *adjacent* in the cyclic element order. They form a run of four
/// quiet element positions, and the cell boundary sits a fixed distance from it.
///
/// The numbers below are the same ones `lu1aat/hellschreiber2026` arrived at
/// (MIT), measured against off-air recordings: band noise scores 0.03–0.06, a
/// clean machine clip 0.87, a real 80 m QSO through QSB 0.84.
const PHASE_COLS: f32 = 80.0;
/// Below this the "gap" is noise shaped by the AGC, and following it would make
/// the picture wander while the band is empty.
const PHASE_MIN_CONF: f32 = 0.2;
/// How much quieter a candidate must be before the picture is moved. Without it
/// the estimate flips between near-equal candidates and the text twitches.
const PHASE_MARGIN: f32 = 0.15;
/// Element positions in the blank run for a fourteen-row cell: the padding
/// above and below, doubled.
const GAP_LANES: usize = 4;
/// Signal-bearing columns needed before the picture may be moved at all. One
/// column cannot locate a gap: a single character's blank lanes look like the
/// inter-line gap wherever they happen to fall, and acting on that walks the
/// picture across the screen during the first moments of every over.
const PHASE_MIN_COLS: u32 = 4;
/// Signal-bearing columns to wait after a move. A sender whose clock differs
/// from ours slides the boundary by a fraction of an element per column, so
/// following it is what keeps the glyphs upright; moving no more often than
/// this keeps the picture still.
const PHASE_DWELL_COLS: u32 = 3;
/// Mean ink a column must carry before it is allowed to teach the tracker. An
/// empty band is not neutral evidence: the shade function returns exact zeros
/// between dots, so silence scores a perfect gap on the quietest four lanes and
/// the picture would chase it. Only a column with signal in it says anything
/// about where the sender's cell starts.
const PHASE_MIN_INK: f32 = 0.02;

/// The dot columns `text` will be sent as, bit 0 = the top row.
///
/// Exposed for the panel's transmit preview and for the loopback tests: it is
/// the same expansion [`HellTx::push_text`] queues, so a test can assert what
/// came back is what went in.
pub fn render_columns(text: &str) -> Vec<u16> {
    let mut out = Vec::with_capacity(text.chars().count() * COLS);
    for ch in text.chars() {
        out.extend_from_slice(&glyph(ch));
    }
    out
}

/// One queued character column plus, on a cell's last column, the index of the
/// character it completes — that is what advances `sent_chars` for the UI.
#[derive(Clone, Copy)]
struct TxCol {
    bits: u16,
    char_done: Option<usize>,
}

/// Hellschreiber modulator. Feed it text, pull audio blocks.
pub struct HellTx {
    rate: f64,
    variant: HellVariant,
    carrier_hz: f64,

    /// Samples per dot, fractional — 48000/245 is 195.918…, and no variant
    /// divides evenly.
    spp: f64,
    /// Dot-ramp length in samples. One dot period, which is what fldigi's
    /// 33-sample `OnShape` works out to at its own rate; a *fixed* millisecond
    /// ramp would smear nine dots at X9.
    ramp: f64,
    gate_step: f64,

    q: VecDeque<TxCol>,
    cur: u16,
    cur_done: Option<usize>,
    row: usize,

    /// Dots emitted so far, and the sample index the current one ends at. Kept
    /// as exact integers so rounding cannot accumulate over a long over.
    dot: u64,
    emitted: u64,
    dot_end: u64,

    ph: f64,
    /// Keying envelope phase, 0 = off … 1 = on, shaped through a raised cosine.
    env: f64,
    /// Burst gate, 0 = unkeyed … 1 = keyed.
    gate: f64,

    total_chars: usize,
    sent_chars: usize,

    /// Transmitted rows per column: 14, or 7 for the variants that send each
    /// font pixel once and are doubled for the eye.
    rows: usize,

    /// Dot columns already generated, drained by [`HellTx::take_columns`] so the
    /// panel can paint what is going out alongside what is coming in.
    echo: Vec<u8>,
}

impl HellTx {
    pub fn new(rate: f64, carrier_hz: f64, variant: HellVariant) -> Self {
        let mut tx = HellTx {
            rate,
            variant,
            carrier_hz,
            spp: 1.0,
            ramp: 1.0,
            gate_step: 1.0 / (rate * GATE_SECS).max(1.0),
            q: VecDeque::new(),
            cur: 0,
            cur_done: None,
            // Start one dot short of a wrap so the very first sample pulls a
            // whole column, rather than opening mid-column.
            row: variant.rows() - 1,
            rows: variant.rows(),
            dot: 0,
            emitted: 0,
            dot_end: 0,
            ph: 0.0,
            env: 0.0,
            gate: 0.0,
            total_chars: 0,
            sent_chars: 0,
            echo: Vec::new(),
        };
        tx.retune();
        tx
    }

    /// Change carrier and/or variant. The queued text survives; a variant change
    /// simply sends the rest of it at the new rate.
    pub fn set_params(&mut self, carrier_hz: f64, variant: HellVariant) {
        self.carrier_hz = carrier_hz;
        if variant != self.variant {
            self.variant = variant;
            self.retune();
        }
    }

    fn retune(&mut self) {
        self.spp = (self.rate / self.variant.pixel_rate()).max(1.0);
        self.ramp = self.spp;
        // Restart the dot clock so `dot_end` stays consistent with the new `spp`.
        self.dot = 0;
        self.emitted = 0;
        self.dot_end = 0;
    }

    /// Queue text. Every character becomes seven dot columns; anything the font
    /// cannot render becomes a blank cell rather than being dropped, so the
    /// column stream stays in step with what the operator typed.
    pub fn push_text(&mut self, text: &str) {
        for ch in text.chars() {
            let ci = self.total_chars;
            self.total_chars += 1;
            let cell = glyph(ch);
            for (c, &bits) in cell.iter().enumerate() {
                let last = c == COLS - 1;
                self.q.push_back(TxCol { bits, char_done: last.then_some(ci) });
            }
        }
    }

    pub fn sent_chars(&self) -> usize {
        self.sent_chars
    }

    pub fn total_chars(&self) -> usize {
        self.total_chars
    }

    /// True once everything queued has been generated *and* the burst gate has
    /// closed, so the caller knows the last dot's tail made it out.
    ///
    /// Only reachable with the transmit switch released — while it is held the
    /// gate stays open by design, because that is how Hell holds a channel. Callers
    /// wanting "has the text finished" want `sent_chars() == total_chars()`.
    ///
    /// `cur_done` has to be clear too: a cell's last column is the blank gap, so
    /// `cur == 0` goes true one column *before* the character it completes has
    /// been counted, and without this the final character would never turn green.
    pub fn drained(&self) -> bool {
        self.q.is_empty()
            && self.cur == 0
            && self.cur_done.is_none()
            && self.gate <= 0.0
            && self.env <= 0.0
    }

    pub fn clear(&mut self) {
        self.q.clear();
        self.cur = 0;
        self.cur_done = None;
        self.row = self.rows - 1;
        self.total_chars = 0;
        self.sent_chars = 0;
        self.echo.clear();
    }

    /// Drain the dot columns generated since the last call, 14 bytes per column,
    /// 0 or 255. Feeds the local transmit echo in the raster.
    pub fn take_columns(&mut self, out: &mut Vec<u8>) {
        out.append(&mut self.echo);
    }

    /// Step to the next dot, pulling a new column at each row wrap. Idle pulls
    /// a blank column, which keeps the raster scrolling and the timing honest
    /// while the operator thinks.
    fn advance_dot(&mut self) {
        self.row += 1;
        if self.row < self.rows {
            return;
        }
        self.row = 0;
        if let Some(ci) = self.cur_done.take() {
            self.sent_chars = ci + 1;
        }
        // Echo the column that just finished, so the panel shows what went out.
        if self.echo.len() < 1 << 16 {
            for r in 0..ROWS {
                self.echo.push(if self.cur & (1 << r) != 0 { 255 } else { 0 });
            }
        }
        match self.q.pop_front() {
            Some(c) => {
                self.cur = c.bits;
                self.cur_done = c.char_done;
            }
            None => {
                self.cur = 0;
                self.cur_done = None;
            }
        }
    }

    /// True while audio should keep being generated: text still queued, or a
    /// dot/gate tail still decaying.
    fn producing(&self) -> bool {
        !self.q.is_empty() || self.cur != 0 || self.env > 0.0
    }

    /// Fill `out` with modulated audio. `keyed` is the operator's transmit
    /// switch: while it is set the gate stays open and idle columns go out as
    /// blank paper, which is how Hell holds a channel between overs.
    pub fn next_block(&mut self, out: &mut [f32], keyed: bool) {
        let ramp_step = 1.0 / self.ramp;
        for s in out.iter_mut() {
            if self.emitted >= self.dot_end {
                self.advance_dot();
                self.dot += 1;
                // Cumulative, not incremental: the dot boundary is derived from
                // the dot index every time, so per-dot rounding cannot pile up.
                self.dot_end = (self.dot as f64 * self.spp).round() as u64;
            }
            self.emitted += 1;

            // A seven-row variant sends one dot per font pixel; the fourteen-row
            // ones send the doubling the font already carries.
            let bit = if self.rows == ROWS { self.row } else { 2 * self.row };
            let lit = self.cur & (1 << bit) != 0;

            // Raised-cosine keying. Stepping a phase toward the target and
            // mapping it through 0.5*(1-cos(pi*x)) reproduces fldigi's
            // OnShape/OffShape, needs no run-length special cases, and turns a
            // lone dot into one smooth hump instead of a click.
            self.env = (self.env + if lit { ramp_step } else { -ramp_step }).clamp(0.0, 1.0);
            let key = 0.5 * (1.0 - (PI * self.env).cos());

            let open = keyed || self.producing();
            self.gate =
                (self.gate + if open { self.gate_step } else { -self.gate_step }).clamp(0.0, 1.0);
            let gate = 0.5 * (1.0 - (PI * self.gate).cos());

            let f = if self.variant.is_fsk() {
                // Ink is the lower tone, matching fldigi.
                let half = self.variant.shift_hz() * 0.5;
                self.carrier_hz + if lit { -half } else { half }
            } else {
                self.carrier_hz
            };
            self.ph += TAU * f / self.rate;
            if self.ph > TAU {
                self.ph -= TAU;
            }

            // FSK holds a constant-amplitude carrier and puts the data in the
            // frequency; OOK puts it in the envelope.
            let amp = if self.variant.is_fsk() { gate } else { key * gate };
            *s = (self.ph.sin() * amp) as f32 * TX_PEAK;
        }
    }
}

/// Hellschreiber demodulator. Feed it audio, collect dot columns.
pub struct HellRx {
    variant: HellVariant,
    carrier_hz: f64,
    rate: f64,
    agc: f32,

    ph: f64,
    /// Boxcar decimation: both the anti-alias filter and the rate reduction, at
    /// one complex add per input sample. Accumulated in f64 so a session running
    /// for hours cannot drift.
    dec: usize,
    dec_acc: (f64, f64),
    dec_n: usize,
    dec_rate: f64,

    fir: ComplexFir,
    fir_in: Vec<Complex32>,
    fir_out: Vec<Complex32>,

    /// Slices per dot, and the cumulative-exact slice clock.
    slice: u64,
    slice_end: u64,
    slices_seen: u64,
    spd: f64,

    /// Per-dot accumulators: peak magnitude for OOK, mean deviation for FSK.
    peak_acc: f32,
    fsk_acc: f32,
    fsk_n: u32,
    prev: Complex32,

    col: Vec<u8>,

    /// Per-element-position ink, for print sync. One bin is updated once per
    /// column, so the time constant is in columns rather than samples.
    phase_bins: [f32; ROWS],
    phase_alpha: f32,
    /// Dots seen since the last retune; indexes `phase_bins` and `col`, so both
    /// are keyed by the receiver's own arbitrary phase and the picture is
    /// rotated into place when it is emitted.
    dots: u64,
    /// Signal-bearing columns folded into `phase_bins` since the last retune.
    phase_cols: u32,
    /// Sample-clock trim in parts per million, applied to the dot clock. A
    /// sender whose clock differs from ours slides the character boundary by a
    /// fraction of an element every column, which draws the glyphs leaning;
    /// WEFAX calls the same correction a slant and every fax operator ends up
    /// touching it.
    slant_ppm: f64,
    /// How many times the picture has been re-framed, for diagnostics and for
    /// the test that a trim really does stop the boundary walking.
    sync_moves: u32,
    /// Signal-bearing columns since the picture last moved. Following the
    /// boundary is also what corrects a rate error between the sender and us:
    /// the drift shows up as the boundary sliding, and each move takes it out,
    /// so the shear never exceeds half an element. The dwell stops that turning
    /// into a twitch on every column.
    cols_since_move: u32,
    /// The cell start the raster is currently drawn at, 0..lanes.
    sync: usize,
    /// Transmitted rows per column: 14, or 7 for the variants that send each
    /// font pixel once. The raster handed to the caller is always `ROWS` tall;
    /// a seven-lane cell is doubled into it.
    lanes: usize,

    agc_peak: f32,
    floor: f32,
    level: f32,
}

impl HellRx {
    /// `agc` is the receive AGC speed: 0 pins the scale (meaningful, because the
    /// digi tap is already post-AGC), 1 is fastest.
    pub fn new(rate: f64, carrier_hz: f64, variant: HellVariant, agc: f32) -> Self {
        let mut rx = HellRx {
            variant,
            carrier_hz,
            rate,
            agc,
            ph: 0.0,
            dec: 1,
            dec_acc: (0.0, 0.0),
            dec_n: 0,
            dec_rate: rate,
            fir: ComplexFir::new(Vec::new()),
            fir_in: Vec::new(),
            fir_out: Vec::new(),
            slice: 0,
            slice_end: 0,
            slices_seen: 0,
            spd: 1.0,
            peak_acc: 0.0,
            fsk_acc: 0.0,
            fsk_n: 0,
            prev: Complex32::default(),
            col: vec![0; ROWS],
            phase_bins: [0.0; ROWS],
            phase_alpha: 1.0 - (-1.0 / PHASE_COLS).exp(),
            slant_ppm: 0.0,
            sync_moves: 0,
            dots: 0,
            phase_cols: 0,
            cols_since_move: PHASE_MIN_COLS,
            sync: 0,
            lanes: variant.rows(),
            agc_peak: 1e-3,
            floor: 0.0,
            level: 0.0,
        };
        rx.retune();
        rx
    }

    /// Change carrier, variant and/or AGC speed. A variant change resets the
    /// dot clock and the partial column: there is no sync to recover, so the
    /// only honest thing is to start the raster afresh.
    pub fn set_params(&mut self, carrier_hz: f64, variant: HellVariant, agc: f32) {
        let retune = variant != self.variant;
        self.carrier_hz = carrier_hz;
        self.variant = variant;
        self.agc = agc;
        if retune {
            self.retune();
        }
    }

    fn retune(&mut self) {
        let pixel_rate = self.variant.pixel_rate() * (1.0 + self.slant_ppm * 1e-6);
        // Land near RX_SLICES samples per dot after decimation; never below 1.
        self.dec = ((self.rate / (RX_SLICES as f64 * pixel_rate)).floor() as usize).max(1);
        self.dec_rate = self.rate / self.dec as f64;
        self.dec_acc = (0.0, 0.0);
        self.dec_n = 0;

        let bw = self.variant.bandwidth_hz();
        self.fir = ComplexFir::new(bandpass_taps(RX_TAPS, -bw / 2.0, bw / 2.0, self.dec_rate));

        // Slices per dot at the decimated rate; the clock counts slices so the
        // dot boundaries land on exact cumulative positions.
        self.spd = (self.dec_rate / pixel_rate).max(1.0);
        self.slice = 0;
        self.slice_end = self.spd.round().max(1.0) as u64;
        self.slices_seen = 0;
        self.peak_acc = 0.0;
        self.fsk_acc = 0.0;
        self.fsk_n = 0;
        self.col.iter_mut().for_each(|v| *v = 0);
        self.phase_bins = [0.0; ROWS];
        self.dots = 0;
        self.phase_cols = 0;
        self.cols_since_move = PHASE_MIN_COLS;
        self.lanes = self.variant.rows();
        self.sync = 0;
    }

    /// Smoothed detector magnitude, for the decode squelch and the tuning
    /// indicator.
    pub fn magnitude(&self) -> f32 {
        self.level
    }

    /// Push audio at the construction rate; append one gray byte per received
    /// dot (0 = black … 255 = white). Fourteen consecutive bytes are one column,
    /// emitted from the character-cell boundary the print-sync tracker has
    /// found, so the caller never has to search for it.
    pub fn process(&mut self, audio: &[f32], out: &mut Vec<u8>) {
        let dph = -TAU * self.carrier_hz / self.rate;
        self.fir_in.clear();
        for &s in audio {
            // Mix to baseband.
            self.ph += dph;
            if self.ph < -TAU {
                self.ph += TAU;
            }
            let (sn, cs) = self.ph.sin_cos();
            self.dec_acc.0 += s as f64 * cs;
            self.dec_acc.1 += s as f64 * sn;
            self.dec_n += 1;
            if self.dec_n >= self.dec {
                let k = 1.0 / self.dec as f64;
                self.fir_in
                    .push(Complex32::new((self.dec_acc.0 * k) as f32, (self.dec_acc.1 * k) as f32));
                self.dec_acc = (0.0, 0.0);
                self.dec_n = 0;
            }
        }
        if self.fir_in.is_empty() {
            return;
        }
        self.fir_out.clear();
        let mut fir_in = std::mem::take(&mut self.fir_in);
        let mut fir_out = std::mem::take(&mut self.fir_out);
        self.fir.process(&fir_in, &mut fir_out);

        for &z in fir_out.iter() {
            let mag = z.norm();
            self.level += 0.001 * (mag - self.level);

            if self.variant.is_fsk() {
                // Discriminator. Its unambiguous range is ±dec_rate/2, which is
                // ≥ 4× the dot rate for every variant — far outside the shift.
                let d = z * self.prev.conj();
                let hz = d.arg() as f64 * self.dec_rate / TAU;
                let norm = (hz / (self.variant.shift_hz() * 0.5)).clamp(-1.0, 1.0);
                self.fsk_acc += norm as f32;
                self.fsk_n += 1;
            }
            self.prev = z;
            // Peak, not mean: the dot window sits at an arbitrary phase against
            // the transmitted dot, and averaging smears a lone dot over two rows.
            self.peak_acc = self.peak_acc.max(mag);

            self.slices_seen += 1;
            if self.slices_seen < self.slice_end {
                continue;
            }
            self.slice += 1;
            self.slice_end = ((self.slice + 1) as f64 * self.spd).round().max(1.0) as u64;

            let v = if self.variant.is_fsk() {
                let mean = if self.fsk_n > 0 { self.fsk_acc / self.fsk_n as f32 } else { 0.0 };
                self.fsk_acc = 0.0;
                self.fsk_n = 0;
                // Ink is the lower tone, so a negative deviation is a lit dot.
                (-mean * 0.5 + 0.5).clamp(0.0, 1.0)
            } else {
                self.shade(self.peak_acc)
            };
            self.peak_acc = 0.0;

            // Two indexes, and they are not the same thing. `lane` is the
            // receiver's own count, which is what the sync estimate is measured
            // in; `cell` is the position within the sender's character, which
            // is `lane` shifted by the boundary the tracker has found. Collect
            // by `cell`, so the fourteen dots in hand are always one whole
            // character and never straddle two — rotating a straddling window
            // would preserve the mixture, not undo it.
            let lanes = self.lanes;
            let lane = (self.dots % lanes as u64) as usize;
            let cell = (lane + lanes - self.sync) % lanes;
            self.col[cell] = (v * 255.0) as u8;
            self.dots += 1;
            if cell + 1 == lanes {
                // One lane is visited once per column, so folding a whole column
                // is the same as updating each lane as it arrives. The bins are
                // keyed by `lane`, so the estimate keeps its own frame of
                // reference however the picture is currently drawn.
                let mut ink = 0.0;
                for (i, &px) in self.col[..lanes].iter().enumerate() {
                    ink += px as f32;
                    let lane = (i + self.sync) % lanes;
                    self.phase_bins[lane] +=
                        (px as f32 / 255.0 - self.phase_bins[lane]) * self.phase_alpha;
                }
                if ink / (lanes as f32 * 255.0) > PHASE_MIN_INK {
                    self.phase_cols += 1;
                    self.cols_since_move += 1;
                }
                // Collected from the cell boundary, so it goes out as it stands
                // — doubled into the display height when the sender only sends
                // half of it.
                if lanes == ROWS {
                    out.extend_from_slice(&self.col[..lanes]);
                } else {
                    for i in 0..lanes {
                        out.push(self.col[i]);
                        out.push(self.col[i]);
                    }
                }
                self.track_sync();
            }
        }

        fir_in.clear();
        fir_out.clear();
        self.fir_in = fir_in;
        self.fir_out = fir_out;
    }

    /// Elements in the blank run. A fourteen-row cell pads by a pixel at each
    /// end, doubled; a seven-row one pads by a single element at each end.
    fn gap_lanes(&self) -> usize {
        if self.lanes == 7 { 2 } else { GAP_LANES }
    }

    /// Where that run starts within a cell: the bottom padding is sent first,
    /// so the run opens half its length before the cell boundary.
    fn gap_offset(&self) -> usize {
        self.lanes - self.gap_lanes() / 2
    }

    /// Ink in the blank-run element positions starting at `start`, cyclically.
    fn gap_ink(&self, start: usize) -> f32 {
        (0..self.gap_lanes()).map(|k| self.phase_bins[(start + k) % self.lanes]).sum()
    }

    /// The cell start implied by the ink seen so far, and how pronounced the
    /// inter-line gap is. A confidence of 0 means every element position is
    /// equally inked — no signal, or a sender whose font has no padding.
    fn estimate_sync(&self) -> (usize, f32) {
        let lanes = self.lanes;
        let total: f32 = self.phase_bins[..lanes].iter().sum();
        let mut quietest = 0usize;
        let mut least = f32::INFINITY;
        for start in 0..lanes {
            let ink = self.gap_ink(start);
            if ink < least {
                least = ink;
                quietest = start;
            }
        }
        // Compared against a flat distribution: a picture with no gap scores 0.
        let gap = self.gap_lanes();
        let flat = total * gap as f32 / lanes as f32;
        let confidence = if flat > 0.0 { (1.0 - least / flat).max(0.0) } else { 0.0 };
        let offset = self.gap_offset();
        ((quietest + lanes - offset) % lanes, confidence)
    }

    /// Move the raster only when another position is convincingly quieter than
    /// the one in use. This is a phasing control automated: it decides *where*
    /// the picture is drawn and never what a dot's intensity is, so a weak
    /// signal is not made to look like a strong one.
    fn track_sync(&mut self) -> usize {
        if self.phase_cols < PHASE_MIN_COLS || self.cols_since_move < PHASE_DWELL_COLS {
            return self.sync;
        }
        let (sync, confidence) = self.estimate_sync();
        if confidence < PHASE_MIN_CONF || sync == self.sync {
            return self.sync;
        }
        let offset = self.gap_offset();
        let candidate = self.gap_ink((sync + offset) % self.lanes);
        let in_use = self.gap_ink((self.sync + offset) % self.lanes);
        if candidate < in_use * (1.0 - PHASE_MARGIN) {
            self.sync = sync;
            self.cols_since_move = 0;
            self.sync_moves += 1;
        }
        self.sync
    }

    /// The cell boundary the raster is currently drawn from, for diagnostics.
    pub fn sync(&self) -> usize {
        self.sync
    }

    /// How many times the picture has been re-framed. A trim that matches the
    /// sender's clock should leave this small.
    pub fn sync_moves(&self) -> u32 {
        self.sync_moves
    }

    /// Sample-clock trim, positive to speed the dot clock up. See `slant_ppm`.
    pub fn set_slant_ppm(&mut self, ppm: f32) {
        let ppm = ppm.clamp(-10_000.0, 10_000.0) as f64;
        if ppm == self.slant_ppm {
            return;
        }
        self.slant_ppm = ppm;
        self.retune();
    }

    pub fn slant_ppm(&self) -> f32 {
        self.slant_ppm as f32
    }

    /// Map an on/off-keyed dot magnitude onto 0..1.
    ///
    /// Two trackers, because either alone misbehaves: the decaying peak (fldigi's
    /// `agc *= 1 - k/rows`) keeps a weak signal legible, and the noise floor stops
    /// it from stretching an idle band into a snowstorm.
    fn shade(&mut self, v: f32) -> f32 {
        if self.agc > 0.0 {
            let k = 0.01 + 0.19 * self.agc.clamp(0.0, 1.0);
            self.agc_peak *= 1.0 - k / (ROWS as f32 * AGC_COLUMNS);
        }
        self.agc_peak = self.agc_peak.max(v);
        // Fast down, slow up: the floor should chase a quietening band promptly
        // but must not creep up through the gaps inside a word.
        let a = if v < self.floor { 0.05 } else { 0.0005 };
        self.floor += a * (v - self.floor);
        let span = (self.agc_peak - self.floor).max(1e-6);
        ((v - self.floor) / span).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f64 = 48_000.0;

    /// A carrier that keeps the whole occupied bandwidth inside a USB passband.
    fn carrier(v: HellVariant) -> f64 {
        1500.0f64.max(v.bandwidth_hz() * 0.5 + 200.0)
    }

    fn run(tx: &mut HellTx, rx: &mut HellRx, px: &mut Vec<u8>, samples: usize, keyed: bool) {
        let mut block = [0.0f32; 480];
        let mut left = samples;
        while left > 0 {
            let n = left.min(block.len());
            tx.next_block(&mut block[..n], keyed);
            rx.process(&block[..n], px);
            left -= n;
        }
    }

    /// Generate until the transmitter drains, with a hard bound.
    ///
    /// `drained()` is only reachable with the switch released — holding it open
    /// is what keeps the channel, so an unbounded `while !drained()` loop driven
    /// with `keyed: true` never terminates. The cap turns any future regression
    /// of that kind into a failing test rather than a hung one.
    fn drain(tx: &mut HellTx, rx: &mut HellRx, px: &mut Vec<u8>, budget: usize) {
        let mut block = [0.0f32; 480];
        let mut spent = 0;
        while !tx.drained() {
            assert!(spent < budget, "transmitter did not drain within {budget} samples");
            tx.next_block(&mut block, false);
            rx.process(&block, px);
            spent += block.len();
        }
    }

    /// Best normalised cross-correlation of `want`'s dot pattern against the
    /// received grays, and the offset it occurred at.
    ///
    /// Hell has no sync, so the recovered stream starts at an unknown dot. Any
    /// honest test of it has to be phase-agnostic.
    fn best_correlation(px: &[u8], want: &[u16]) -> (f32, usize) {
        let pat: Vec<f32> = want
            .iter()
            .flat_map(|&c| (0..ROWS).map(move |r| if c & (1 << r) != 0 { 1.0 } else { 0.0 }))
            .collect();
        let n = pat.len();
        assert!(px.len() > n, "not enough received dots to correlate");
        let pat_mean = pat.iter().sum::<f32>() / n as f32;
        let pat_dev: Vec<f32> = pat.iter().map(|v| v - pat_mean).collect();
        let pat_norm = pat_dev.iter().map(|v| v * v).sum::<f32>().sqrt();

        let mut best = (f32::MIN, 0usize);
        for off in 0..px.len() - n {
            let win = &px[off..off + n];
            let mean = win.iter().map(|&v| v as f32).sum::<f32>() / (n as f32 * 255.0);
            let mut num = 0.0f32;
            let mut den = 0.0f32;
            for (i, &v) in win.iter().enumerate() {
                let d = v as f32 / 255.0 - mean;
                num += d * pat_dev[i];
                den += d * d;
            }
            let r = num / (den.sqrt() * pat_norm).max(1e-9);
            if r > best.0 {
                best = (r, off);
            }
        }
        best
    }

    /// Print the received raster for one variant. Not an assertion — legibility
    /// is a human judgement, and this is how it gets made.
    #[test]
    #[ignore = "diagnostic: cargo test -p sdroxide-dsp -- --ignored --nocapture"]
    fn show_the_received_raster() {
        for variant in HellVariant::ALL {
            let hz = carrier(variant);
            let mut tx = HellTx::new(RATE, hz, variant);
            let mut rx = HellRx::new(RATE, hz, variant, 0.5);
            let mut px = Vec::new();
            let dot = (RATE / variant.pixel_rate()) as usize;
            run(&mut tx, &mut rx, &mut px, dot * ROWS * 8, true);
            tx.push_text("HELL");
            drain(&mut tx, &mut rx, &mut px, dot * ROWS * 300);
            run(&mut tx, &mut rx, &mut px, dot * ROWS * 4, false);

            let (score, off) = best_correlation(&px, &render_columns("HELL"));
            println!("\n{variant:?}  ({} dots, correlation {score:.3} at {off})", px.len());
            let cols = px.len() / ROWS;
            for r in 0..ROWS {
                let line: String = (0..cols)
                    .map(|c| match px[c * ROWS + r] {
                        0..=63 => ' ',
                        64..=127 => '.',
                        128..=191 => '+',
                        _ => '#',
                    })
                    .collect();
                println!("|{line}|");
            }
        }
    }

    #[test]
    fn text_survives_a_loopback_round_trip() {
        // The SSB modulator/demodulator pair the engine puts between these two
        // is unity for a real audio signal, so wiring them directly is a fair
        // test of the modem itself.
        //
        // Threshold set from measurement, not aspiration: the on/off-keyed
        // variants land at 0.85–0.91 and the FSK ones at 0.96–0.98. OOK cannot
        // reach the latter and should not be expected to — a raised-cosine dot's
        // peak depends on its run length, so an isolated dot reads greyer than
        // one inside a run, and correlating against an ideal 0/1 pattern caps
        // out. That greyness is what real Feld Hell looks like. A broken modem
        // scores near zero, so 0.80 separates the two cleanly.
        let mut scores = Vec::new();
        for variant in HellVariant::ALL {
            let hz = carrier(variant);
            let mut tx = HellTx::new(RATE, hz, variant);
            let mut rx = HellRx::new(RATE, hz, variant, 0.5);
            let mut px = Vec::new();

            let dot = (RATE / variant.pixel_rate()) as usize;
            // Lead-in settles the AGC, the FIR and the floor tracker.
            run(&mut tx, &mut rx, &mut px, dot * ROWS * 8, true);
            let lead = px.len();

            // Repeated, so the correlation has whole cells left to match after
            // the settling columns are skipped.
            tx.push_text("HELLHELLHELLHELLHELLHELL");
            drain(&mut tx, &mut rx, &mut px, dot * ROWS * 300);
            // Tail clears the filter's group delay.
            run(&mut tx, &mut rx, &mut px, dot * ROWS * 4, false);

            // The first few columns go out before there is enough signal for
            // the phasing to settle, exactly as they do on the air; an operator
            // reads past them, and so does this.
            let settled = (lead + 8 * ROWS).min(px.len() - 1);
            let want = render_columns("HELL");
            let (score, _off) = best_correlation(&px[settled..], &want);
            scores.push((variant, score));
        }
        // Report the whole set, so a failure shows whether one variant regressed
        // or something common to all of them did.
        let worst = scores.iter().fold(f32::MAX, |a, &(_, s)| a.min(s));
        assert!(
            worst > 0.80,
            "loopback correlations: {}",
            scores.iter().map(|(v, s)| format!("{v:?} {s:.3}")).collect::<Vec<_>>().join(", ")
        );
    }

    /// Print sync: the receiver has to find where the sender's character cell
    /// begins, because Hell carries no frame sync at all.
    ///
    /// The measure is the inter-line gap. Every on-air font leaves the top and
    /// bottom rows of the cell blank, so a correctly framed picture has four
    /// quiet rows where two characters meet. Get the boundary wrong and each
    /// glyph is cut and wrapped onto itself, which fills those rows with ink —
    /// a reader sees it as a wall of pixels with no line to read along.
    ///
    /// A correlation against the expected text cannot see this: a constant
    /// rotation of a dot stream is indistinguishable from a shift of it, so the
    /// old loopback scored well on a picture nobody could read. This measures
    /// the picture instead.
    #[test]
    fn the_cell_boundary_is_found_without_being_told() {
        for variant in HellVariant::ALL {
            let hz = carrier(variant);
            let mut tx = HellTx::new(RATE, hz, variant);
            let mut rx = HellRx::new(RATE, hz, variant, 0.5);
            let mut px = Vec::new();
            let dot = (RATE / variant.pixel_rate()) as usize;

            // A loopback shares one clock, so put the receiver out of step on
            // purpose: that is the state every real signal arrives in.
            rx.process(&vec![0.0f32; dot / 3 + 2], &mut px);
            run(&mut tx, &mut rx, &mut px, dot * ROWS * 8, true);
            let lead = px.len();

            tx.push_text("HELLHELLHELLHELLHELLHELL");
            drain(&mut tx, &mut rx, &mut px, dot * ROWS * 400);
            run(&mut tx, &mut rx, &mut px, dot * ROWS * 4, false);

            // Past the columns drawn before the phasing had anything to work on.
            let tail = &px[lead + 8 * ROWS..];
            let cols = tail.len() / ROWS;
            let row_ink = |r: usize| {
                (0..cols).map(|c| tail[c * ROWS + r] as f32).sum::<f32>() / cols.max(1) as f32
            };
            let pad: f32 =
                [0, 1, ROWS - 2, ROWS - 1].iter().map(|&r| row_ink(r)).sum::<f32>() / 4.0;
            let body: f32 = (2..ROWS - 2).map(row_ink).sum::<f32>() / (ROWS - 4) as f32;
            let ratio = pad / body.max(1.0);
            assert!(
                ratio < 0.6,
                "{variant:?}: the inter-line gap is missing — the padding rows carry {ratio:.2} of the body's ink, so the cell boundary is wrong"
            );
        }
    }

    /// A sender whose clock differs from ours slides the character boundary by
    /// a fraction of an element every column, and the picture leans. The trim
    /// is the fix, and this is what says so: with it the boundary stops
    /// walking, without it the raster is re-framed over and over.
    #[test]
    fn a_clock_trim_stops_the_boundary_walking() {
        // Dots 0.4 % long, which is an ordinary sound card capture — and what
        // SkySweep's off-air Hell recording measures. Generated at a higher
        // sample rate and played back at this one, a dot comes out longer.
        let slow = RATE * 1.004;
        let variant = HellVariant::Feld;
        let mut counts = Vec::new();
        for ppm in [0.0f32, -4000.0] {
            let hz = carrier(variant);
            let mut tx = HellTx::new(slow, hz, variant);
            let mut rx = HellRx::new(RATE, hz, variant, 0.5);
            rx.set_slant_ppm(ppm);
            let mut px = Vec::new();
            let dot = (RATE / variant.pixel_rate()) as usize;
            run(&mut tx, &mut rx, &mut px, dot * ROWS * 8, true);
            tx.push_text("HELLHELLHELLHELLHELLHELLHELLHELL");
            drain(&mut tx, &mut rx, &mut px, dot * ROWS * 400);
            counts.push((ppm, rx.sync_moves()));
        }
        let untrimmed = counts[0].1;
        let trimmed = counts[1].1;
        assert!(
            trimmed * 3 < untrimmed.max(1),
            "the trim did not stop the boundary walking: {untrimmed} moves untrimmed, {trimmed} with {} ppm",
            counts[1].0
        );
    }

    #[test]
    fn the_dot_clock_is_cumulative_exact() {
        for variant in HellVariant::ALL {
            let mut tx = HellTx::new(RATE, 1500.0, variant);
            let mut out = vec![0.0f32; 4096];
            // Long enough to expose per-dot rounding if it accumulated.
            let secs = 20.0;
            let mut left = (RATE * secs) as usize;
            while left > 0 {
                let n = left.min(out.len());
                tx.next_block(&mut out[..n], true);
                left -= n;
            }
            let want = RATE * secs / (RATE / variant.pixel_rate());
            let got = tx.dot as f64;
            assert!(
                (got - want).abs() <= 1.0,
                "{variant:?}: {got} dots after {secs}s, want {want:.1}"
            );
        }
    }

    #[test]
    fn characters_are_seven_columns_wide() {
        assert_eq!(render_columns("A").len(), HELL_CELL_COLS);
        assert_eq!(render_columns("HELL").len(), 4 * HELL_CELL_COLS);
        // 2.5 characters/second is the number Feld Hell is specified by.
        assert!((HellVariant::Feld.chars_per_sec() - 2.5).abs() < 1e-9);
    }

    #[test]
    fn transmit_stays_within_headroom() {
        for variant in HellVariant::ALL {
            let mut tx = HellTx::new(RATE, carrier(variant), variant);
            tx.push_text("HELL DE TEST");
            let mut out = vec![0.0f32; 4096];
            let mut peak = 0.0f32;
            let mut guard = 0;
            while !tx.drained() {
                guard += 1;
                assert!(guard < 100_000, "{variant:?}: never drained");
                tx.next_block(&mut out, false);
                peak = peak.max(out.iter().fold(0.0f32, |a, &s| a.max(s.abs())));
            }
            assert!(peak <= TX_PEAK + 1e-4, "{variant:?}: peak {peak}");
            assert!(peak > 0.4, "{variant:?}: peak {peak} — is anything being sent?");
        }
    }

    #[test]
    fn on_off_keying_goes_quiet_when_idle() {
        let mut tx = HellTx::new(RATE, 1500.0, HellVariant::Feld);
        let mut out = vec![0.0f32; 48_000];
        tx.next_block(&mut out, true);
        // Skip the gate's rise; the rest is idle paper and must be silent.
        let tail = &out[4800..];
        let peak = tail.iter().fold(0.0f32, |a, &s| a.max(s.abs()));
        assert!(peak < 1e-3, "idle Feld Hell should be silent, peak {peak}");
    }

    #[test]
    fn fsk_keeps_the_carrier_up_when_idle() {
        let mut tx = HellTx::new(RATE, 1500.0, HellVariant::Fsk245);
        let mut out = vec![0.0f32; 48_000];
        tx.next_block(&mut out, true);
        let tail = &out[4800..];
        let peak = tail.iter().fold(0.0f32, |a, &s| a.max(s.abs()));
        assert!(peak > 0.4, "idle FSK Hell should hold its carrier, peak {peak}");
    }

    #[test]
    fn sent_chars_reaches_total_exactly_when_drained() {
        let mut tx = HellTx::new(RATE, 1500.0, HellVariant::Feld);
        tx.push_text("CQ DE TEST");
        assert_eq!(tx.total_chars(), 10);
        let mut out = vec![0.0f32; 4096];
        let mut guard = 0;
        while !tx.drained() {
            guard += 1;
            assert!(guard < 100_000, "never drained");
            assert!(tx.sent_chars() <= tx.total_chars());
            tx.next_block(&mut out, false);
        }
        assert_eq!(tx.sent_chars(), tx.total_chars());
    }

    #[test]
    fn the_transmit_echo_matches_the_font() {
        let mut tx = HellTx::new(RATE, 1500.0, HellVariant::Feld);
        tx.push_text("HI");
        let mut out = vec![0.0f32; 4096];
        let mut echo = Vec::new();
        let mut guard = 0;
        while !tx.drained() {
            guard += 1;
            assert!(guard < 100_000, "never drained");
            tx.next_block(&mut out, false);
            tx.take_columns(&mut echo);
        }
        let want = render_columns("HI");
        // The echo leads with the blank column that was in flight at start-up.
        let start = ROWS;
        assert!(echo.len() >= start + want.len() * ROWS);
        for (c, &bits) in want.iter().enumerate() {
            for r in 0..ROWS {
                let got = echo[start + c * ROWS + r];
                let want_lit = bits & (1 << r) != 0;
                assert_eq!(got != 0, want_lit, "column {c} row {r}");
            }
        }
    }

    #[test]
    fn shaped_keying_keeps_the_spectrum_narrow() {
        // The whole point of the raised-cosine dot: hard keying would splatter
        // far outside the mode's nominal bandwidth.
        let variant = HellVariant::Feld;
        let hz = 1500.0;
        let mut tx = HellTx::new(RATE, hz, variant);
        tx.push_text("HELL");
        let mut audio = Vec::new();
        let mut out = vec![0.0f32; 4096];
        let mut guard = 0;
        while !tx.drained() {
            guard += 1;
            assert!(guard < 100_000, "never drained");
            tx.next_block(&mut out, false);
            audio.extend_from_slice(&out);
        }

        // Energy inside ±400 Hz of the carrier, against the total, by direct
        // single-bin sums — no FFT needed for a two-way split. Each bin walks
        // the samples with a complex rotator rather than a sin/cos per sample.
        let mut inside = 0.0f64;
        let mut total = 0.0f64;
        for f in (100..3500).step_by(25) {
            let (s, c) = (TAU * f as f64 / RATE).sin_cos();
            let (mut cs, mut sn) = (1.0f64, 0.0f64);
            let (mut re, mut im) = (0.0f64, 0.0f64);
            for &v in audio.iter() {
                re += v as f64 * cs;
                im += v as f64 * sn;
                (cs, sn) = (cs * c - sn * s, sn * c + cs * s);
            }
            let p = re * re + im * im;
            total += p;
            if (f as f64 - hz).abs() <= 400.0 {
                inside += p;
            }
        }
        let frac = inside / total;
        assert!(frac > 0.99, "only {:.3}% of the energy is in-band", frac * 100.0);
    }
    /// What the decoder makes of a real transmission, drawn as dots.
    ///
    /// Hell delivers pictures rather than text, so a capture can only be judged
    /// by looking at it. `SDROXIDE_HELL_SAMPLE` is a mono 8 kHz WAV,
    /// `SDROXIDE_HELL_CARRIER` the audio centre (default 1700 Hz).
    ///
    /// Run it with
    /// `SDROXIDE_HELL_SAMPLE=... cargo test -p sdroxide-dsp --release --lib \
    ///   -- --ignored --nocapture an_off_air_capture_renders`.
    #[test]
    #[ignore]
    fn an_off_air_capture_renders() {
        let Ok(path) = std::env::var("SDROXIDE_HELL_SAMPLE") else {
            eprintln!("SDROXIDE_HELL_SAMPLE unset; skipping");
            return;
        };
        let mut reader = hound::WavReader::open(&path).expect("open the sample WAV");
        let spec = reader.spec();
        assert_eq!(spec.sample_rate, 8000, "the sample must be at 8 kHz");
        assert_eq!(spec.channels, 1, "the sample must be mono");
        let audio: Vec<f32> =
            reader.samples::<i16>().map(|s| s.expect("sample") as f32 / 32_768.0).collect();
        let carrier: f64 = std::env::var("SDROXIDE_HELL_CARRIER")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1700.0);
        let mut rx = HellRx::new(8000.0, carrier, HellVariant::Feld, 0.05);
        let mut px = Vec::new();
        for chunk in audio.chunks(512) {
            rx.process(chunk, &mut px);
        }
        let cols = px.len() / HELL_ROWS;
        let lit = px.iter().filter(|&&v| v > 96).count();
        eprintln!(
            "{cols} columns of {HELL_ROWS} dots, {lit} lit, max {}",
            px.iter().copied().max().unwrap_or(0)
        );
        const STRIP: usize = 104;
        for start in (0..cols).step_by(STRIP) {
            let end = (start + STRIP).min(cols);
            for row in 0..HELL_ROWS {
                // Hell staggers alternate rows by half a column, which is what
                // turns a lattice of dots into letter shapes.
                let lead = if row % 2 == 1 { " " } else { "" };
                let line: String = (start..end)
                    .map(|c| match px[c * HELL_ROWS + row] {
                        v if v > 128 => '#',
                        v if v > 48 => '+',
                        _ => ' ',
                    })
                    .collect();
                eprintln!("{lead}{line}");
            }
            eprintln!("---");
        }
    }
}
