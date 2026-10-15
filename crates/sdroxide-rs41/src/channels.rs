//! Multiple `*mod` helpers across one IQ lane.
//!
//! A single `rs41mod --IQ 0` only demodulates the dial centre. Two sondes a
//! few kilohertz apart in the same 48 kHz (or 76.8 kHz) window need two
//! helpers, each with `--IQ <frac>` and `--lpIQ` so its IF low-pass sits on
//! that carrier. This bank scans the lane spectrum, keeps one channel per
//! peak (plus DC), and fans the same IQ into every live helper.

use std::time::{Duration, Instant};

use sdroxide_dsp::{Complex32, SpectrumAnalyzer};
use sdroxide_types::SondeKind;
use tracing::{debug, info, warn};

use crate::modproc::ModProc;
use crate::package::Package;

/// How often to re-scan the lane for carriers.
const SCAN_EVERY: Duration = Duration::from_secs(2);
/// FFT size for the peak search (≈11.7 Hz bins at 48 kHz).
const SCAN_FFT: usize = 4096;
/// Minimum spacing between channels — wider than an RS41 occupied bandwidth.
const MIN_SEP_HZ: f64 = 5_000.0;
/// Peak must clear the median spectrum by this much.
const PEAK_MARGIN_DB: f32 = 8.0;
/// Concurrent `*mod` helpers (DC + strongest peaks).
const MAX_CHANNELS: usize = 6;
/// Keep a channel this long after its peak vanishes if it is still decoding.
const HOLD_AFTER_HEARD: Duration = Duration::from_secs(45);

pub struct ChannelBank {
    kind: SondeKind,
    rate_hz: f64,
    channels: Vec<Channel>,
    spectrum: SpectrumAnalyzer,
    scan_buf: Vec<Complex32>,
    next_scan: Instant,
    db: Vec<f32>,
}

struct Channel {
    /// Offset from the lane centre, Hz (positive = above dial).
    offset_hz: f64,
    proc: ModProc,
    last_pkg: Instant,
}

impl ChannelBank {
    pub fn open(kind: SondeKind, rate_hz: f64) -> Result<ChannelBank, String> {
        let rate = if rate_hz > 1.0 { rate_hz } else { kind.lane_rate_hz() };
        let mut bank = ChannelBank {
            kind,
            rate_hz: rate,
            channels: Vec::new(),
            spectrum: SpectrumAnalyzer::new(SCAN_FFT, rate, 0.3),
            scan_buf: Vec::with_capacity(SCAN_FFT * 2),
            next_scan: Instant::now(),
            db: Vec::new(),
        };
        bank.spectrum.set_dc_suppress(false);
        // Always start with the dial centre so a sonde on-frequency works
        // before the first scan lands.
        bank.ensure_channel(0.0)?;
        Ok(bank)
    }

    pub fn len(&self) -> usize {
        self.channels.len()
    }

    /// Feed one IQ block: scan when due, then fan-out to every live helper.
    pub fn on_iq(&mut self, iq: &[Complex32]) -> Vec<(f64, Package)> {
        if iq.is_empty() {
            return Vec::new();
        }
        self.scan_buf.extend_from_slice(iq);
        if self.scan_buf.len() > SCAN_FFT * 4 {
            let drop = self.scan_buf.len() - SCAN_FFT * 2;
            self.scan_buf.drain(..drop);
        }
        if Instant::now() >= self.next_scan && self.scan_buf.len() >= SCAN_FFT {
            self.next_scan = Instant::now() + SCAN_EVERY;
            self.rescan();
        }

        let mut out = Vec::new();
        let mut dead = Vec::new();
        for (i, ch) in self.channels.iter_mut().enumerate() {
            if !ch.proc.alive() {
                warn!(
                    offset_hz = ch.offset_hz,
                    bin = self.kind.binary(),
                    "sonde channel *mod exited"
                );
                dead.push(i);
                continue;
            }
            ch.proc.write_iq(iq);
            let pkgs = ch.proc.drain_packages();
            if !pkgs.is_empty() {
                ch.last_pkg = Instant::now();
            }
            for pkg in pkgs {
                out.push((ch.offset_hz, pkg));
            }
        }
        for i in dead.into_iter().rev() {
            self.channels.remove(i);
        }
        // Centre channel is required — recreate if it died.
        if !self.channels.iter().any(|c| c.offset_hz.abs() < 500.0)
            && let Err(e) = self.ensure_channel(0.0)
        {
            warn!(%e, "sonde centre channel restart failed");
        }
        out
    }

    pub fn frame_counters(&self) -> (u64, u64) {
        let mut frames = 0u64;
        let mut bad = 0u64;
        for ch in &self.channels {
            frames = frames.saturating_add(ch.proc.frames());
            bad = bad.saturating_add(ch.proc.bad());
        }
        (frames, bad)
    }

    fn rescan(&mut self) {
        let chunk = &self.scan_buf[self.scan_buf.len().saturating_sub(SCAN_FFT * 2)..];
        self.spectrum.process(chunk);
        self.spectrum.spectrum_db(&mut self.db);
        if self.db.len() < 16 {
            return;
        }
        let peaks = find_peaks(&self.db, self.rate_hz, MAX_CHANNELS);
        debug!(?peaks, channels = self.channels.len(), "sonde lane peak scan");
        self.reconcile(&peaks);
    }

    fn reconcile(&mut self, peaks: &[f64]) {
        let now = Instant::now();
        // Drop channels whose peak is gone and that have gone quiet.
        self.channels.retain(|ch| {
            let near_peak = peaks.iter().any(|&p| (p - ch.offset_hz).abs() < MIN_SEP_HZ * 0.6);
            near_peak || now.duration_since(ch.last_pkg) < HOLD_AFTER_HEARD
        });
        for &hz in peaks {
            if self.channels.iter().any(|c| (c.offset_hz - hz).abs() < MIN_SEP_HZ * 0.6) {
                continue;
            }
            if self.channels.len() >= MAX_CHANNELS {
                break;
            }
            if let Err(e) = self.ensure_channel(hz) {
                warn!(offset_hz = hz, %e, "sonde channel open failed");
            }
        }
    }

    fn ensure_channel(&mut self, offset_hz: f64) -> Result<(), String> {
        if self.channels.iter().any(|c| (c.offset_hz - offset_hz).abs() < 500.0) {
            return Ok(());
        }
        let proc = ModProc::start(self.kind, self.rate_hz, offset_hz)?;
        info!(
            offset_hz,
            rate = self.rate_hz,
            kind = ?self.kind,
            "sonde *mod channel started"
        );
        self.channels.push(Channel { offset_hz, proc, last_pkg: Instant::now() });
        Ok(())
    }
}

/// DC-centred `spectrum_db` bins → offset frequencies of the strongest peaks.
fn find_peaks(db: &[f32], rate_hz: f64, max_n: usize) -> Vec<f64> {
    let n = db.len();
    let mut sorted = db.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let noise = sorted[n / 2];
    let thresh = noise + PEAK_MARGIN_DB;

    let mut candidates: Vec<(usize, f32)> = Vec::new();
    for i in 2..n.saturating_sub(2) {
        let v = db[i];
        if v >= thresh
            && v >= db[i - 1]
            && v >= db[i + 1]
            && v >= db[i - 2]
            && v >= db[i + 2]
        {
            candidates.push((i, v));
        }
    }
    candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut offsets = Vec::new();
    // Dial centre first — the operator pointed here.
    offsets.push(0.0);
    for (bin, _) in candidates {
        let hz = (bin as f64 - n as f64 / 2.0) * rate_hz / n as f64;
        if hz.abs() > rate_hz * 0.42 {
            continue; // near Nyquist / DDC roll-off
        }
        if offsets.iter().any(|&o| (o - hz).abs() < MIN_SEP_HZ) {
            continue;
        }
        offsets.push(hz);
        if offsets.len() >= max_n {
            break;
        }
    }
    offsets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_two_tones_across_the_lane() {
        // Synthetic DC-centred spectrum: noise floor −80 dB, two carriers.
        let n = 512;
        let mut db = vec![-80.0f32; n];
        // bins → Hz: (i - n/2) * rate / n ; rate=48000 → bin 256 = 0 Hz
        // +8 kHz → bin 256 + 8000*512/48000 ≈ 256+85
        // −12 kHz → bin 256 - 128
        db[256 + 85] = -40.0;
        db[256 + 84] = -45.0;
        db[256 + 86] = -45.0;
        db[256 - 128] = -38.0;
        db[256 - 127] = -44.0;
        db[256 - 129] = -44.0;
        let peaks = find_peaks(&db, 48_000.0, 6);
        assert!(peaks.iter().any(|p| p.abs() < 1.0), "DC present: {peaks:?}");
        assert!(
            peaks.iter().any(|p| (p - 8_000.0).abs() < 500.0),
            "+8 kHz missing: {peaks:?}"
        );
        assert!(
            peaks.iter().any(|p| (p + 12_000.0).abs() < 500.0),
            "-12 kHz missing: {peaks:?}"
        );
    }
}
