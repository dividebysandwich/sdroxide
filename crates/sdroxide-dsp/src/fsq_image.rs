//! FSQ image sub-mode: a small grayscale picture sent as an analog tone scan
//! (pixel intensity → audio frequency), the way FSQCALL / MFSK picture transfer
//! works. Fixed size; the UI crops/scales the source. Receive uses an FM
//! discriminator (as SSTV does) for continuous grayscale, with a leader tone
//! marking the start.
//!
//! Interop note: the geometry/timing are self-consistent and loopback-tested but
//! not yet bit-matched to fldigi's FSQ image (tracked for live validation).

use crate::Complex32;
use crate::fir::{ComplexFir, bandpass_taps};
use crate::mfsk::ToneGen;

/// Fixed transmit geometry (the UI scales any source to this).
pub const IMG_W: usize = 160;
pub const IMG_H: usize = 120;

/// Image frequency span (Hz) around the audio centre.
const SPAN: f64 = 1000.0;
/// Samples per pixel at the internal rate.
const PIX: usize = 8;
/// Leader duration (seconds) of the below-band sync tone.
const LEADER_S: f64 = 0.3;

fn fmin(center: f64) -> f64 {
    center - SPAN / 2.0
}
fn freq_for(center: f64, gray: u8) -> f64 {
    fmin(center) + (gray as f64 / 255.0) * SPAN
}
fn sync_hz(center: f64) -> f64 {
    fmin(center) - 200.0
}

// ─────────────────────────────── transmit ───────────────────────────────

/// Precomputes the whole transmission; [`FsqImageTx::next_block`] drains it.
pub struct FsqImageTx {
    audio: Vec<f32>,
    pos: usize,
}

impl FsqImageTx {
    /// `gray` is `w*h` bytes; it is resampled (nearest) to `IMG_W×IMG_H`.
    pub fn new(rate: f64, center: f64, gray: &[u8], w: usize, h: usize) -> Self {
        let mut tg = ToneGen::new(rate);
        let mut audio = Vec::new();
        // Leader: a steady below-band sync tone.
        tg.emit(sync_hz(center), (LEADER_S * rate) as usize, 0.5, &mut audio);
        for y in 0..IMG_H {
            for x in 0..IMG_W {
                let sx = x * w.max(1) / IMG_W;
                let sy = y * h.max(1) / IMG_H;
                let g = gray.get(sy * w + sx).copied().unwrap_or(0);
                tg.emit(freq_for(center, g), PIX, 0.5, &mut audio);
            }
        }
        // Trailer sync so the last row is fully captured.
        tg.emit(sync_hz(center), (0.1 * rate) as usize, 0.5, &mut audio);
        FsqImageTx { audio, pos: 0 }
    }

    /// Fill `out`; returns true when the whole image has been emitted.
    pub fn next_block(&mut self, out: &mut [f32]) -> bool {
        for s in out.iter_mut() {
            *s = self.audio.get(self.pos).copied().unwrap_or(0.0);
            self.pos += 1;
        }
        self.pos >= self.audio.len()
    }

    pub fn done(&self) -> bool {
        self.pos >= self.audio.len()
    }
}

// ─────────────────────────────── receive ───────────────────────────────

#[derive(PartialEq)]
enum State {
    SeekLeader,
    Collect,
}

pub struct FsqImageRx {
    rate: f64,
    center: f64,
    ph: f32,
    ph_inc: f32,
    lpf: ComplexFir,
    prev: Complex32,
    state: State,
    leader_run: usize,
    saw_leader: bool,
    pix_acc: f64,
    pix_n: usize,
    img: Vec<u8>,
}

impl FsqImageRx {
    pub fn new(rate: f64, center: f64) -> Self {
        let taps = bandpass_taps(63, -700.0, 700.0, rate);
        FsqImageRx {
            rate,
            center,
            ph: 0.0,
            ph_inc: (std::f64::consts::TAU * center / rate) as f32,
            lpf: ComplexFir::new(taps),
            prev: Complex32::new(0.0, 0.0),
            state: State::SeekLeader,
            leader_run: 0,
            saw_leader: false,
            pix_acc: 0.0,
            pix_n: 0,
            img: Vec::with_capacity(IMG_W * IMG_H),
        }
    }

    /// True while a picture is actively being received (suppress text decode).
    pub fn is_collecting(&self) -> bool {
        self.state == State::Collect
    }

    pub fn set_center(&mut self, center: f64) {
        self.center = center;
        self.ph_inc = (std::f64::consts::TAU * center / self.rate) as f32;
        self.reset();
    }

    fn reset(&mut self) {
        self.state = State::SeekLeader;
        self.leader_run = 0;
        self.saw_leader = false;
        self.pix_acc = 0.0;
        self.pix_n = 0;
        self.img.clear();
        self.lpf.reset();
    }

    /// Feed audio; returns `(gray, w, h)` once a whole image has been received.
    pub fn process(&mut self, audio: &[f32]) -> Option<(Vec<u8>, u16, u16)> {
        let mut mixed = Vec::with_capacity(audio.len());
        for &a in audio {
            let z = Complex32::new(a * self.ph.cos(), -a * self.ph.sin());
            self.ph += self.ph_inc;
            if self.ph > std::f32::consts::TAU {
                self.ph -= std::f32::consts::TAU;
            }
            mixed.push(z);
        }
        let mut bb = Vec::with_capacity(audio.len());
        self.lpf.process(&mixed, &mut bb);

        let sync_off = (sync_hz(self.center) - self.center) as f32; // negative
        let fmin_off = (fmin(self.center) - self.center) as f32;
        let leader_min = (0.15 * self.rate) as usize;
        let mut result = None;
        for z in bb {
            // FM discriminator: instantaneous frequency offset (Hz) from centre.
            let d = z * self.prev.conj();
            self.prev = z;
            let off_hz = d.im.atan2(d.re) * self.rate as f32 / std::f32::consts::TAU;

            match self.state {
                State::SeekLeader => {
                    // Below-band sync tone marks the leader; the image begins when
                    // the tone rises back into the pixel band.
                    if off_hz < fmin_off - 50.0 && off_hz > sync_off - 200.0 {
                        self.leader_run += 1;
                        if self.leader_run >= leader_min {
                            self.saw_leader = true;
                        }
                    } else if self.saw_leader && off_hz >= fmin_off - 20.0 {
                        self.state = State::Collect;
                        self.img.clear();
                        self.pix_acc = off_hz as f64;
                        self.pix_n = 1;
                    } else {
                        self.leader_run = 0;
                    }
                }
                State::Collect => {
                    self.pix_acc += off_hz as f64;
                    self.pix_n += 1;
                    if self.pix_n >= PIX {
                        let avg = self.pix_acc / self.pix_n as f64;
                        // Map the frequency offset back to 0..255.
                        let g = ((avg - fmin_off as f64) / SPAN * 255.0).round().clamp(0.0, 255.0);
                        self.img.push(g as u8);
                        self.pix_acc = 0.0;
                        self.pix_n = 0;
                        if self.img.len() >= IMG_W * IMG_H {
                            result = Some((
                                std::mem::take(&mut self.img),
                                IMG_W as u16,
                                IMG_H as u16,
                            ));
                            self.reset();
                        }
                    }
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_loopback_gradient() {
        let rate = 8000.0;
        let center = 1500.0;
        // A horizontal gradient.
        let mut src = vec![0u8; IMG_W * IMG_H];
        for y in 0..IMG_H {
            for x in 0..IMG_W {
                src[y * IMG_W + x] = (x * 255 / IMG_W) as u8;
            }
        }
        let mut tx = FsqImageTx::new(rate, center, &src, IMG_W, IMG_H);
        let mut sig = vec![0.0f32; (0.2 * rate) as usize]; // quiet lead-in
        loop {
            let mut b = [0.0f32; 4096];
            let done = tx.next_block(&mut b);
            sig.extend_from_slice(&b);
            if done {
                break;
            }
        }
        let mut rx = FsqImageRx::new(rate, center);
        let mut got = None;
        for chunk in sig.chunks(1024) {
            if let Some(img) = rx.process(chunk) {
                got = Some(img);
                break;
            }
        }
        let (img, w, h) = got.expect("image should decode");
        assert_eq!((w, h), (IMG_W as u16, IMG_H as u16));
        // Mean absolute error should be small (discriminator is approximate).
        let mae: f64 = img
            .iter()
            .zip(&src)
            .map(|(&a, &b)| (a as f64 - b as f64).abs())
            .sum::<f64>()
            / img.len() as f64;
        assert!(mae < 30.0, "mean abs error too high: {mae}");
    }
}
