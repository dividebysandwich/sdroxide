//! FSQ (Fast Simple QSO) modem: streaming decode + encode.
//!
//! FSQ is a 33-tone **incremental frequency keying** chat mode: a symbol is sent
//! as `tone = (previous + symbol + 1) mod 33`, so it carries five bits of
//! *difference* and never repeats a tone. There is no forward error correction —
//! robustness comes from the tone spacing and, in the directed FSQCALL layer,
//! from message checksums.
//!
//! Three things here are read out of fldigi's own `src/fsq/fsq.cxx` and
//! `fsq_varicode.cxx` (GPL-3.0), and every one of them was wrong before:
//!
//! * **The alphabet is nibbles, not bits.** fldigi carries
//!   `fsq_varicode[256][2]`: one or two *nibbles* per character, the second sent
//!   only when it is above 28. The text does **not** ride PSK31's bit-level
//!   varicode, which is what this file used, and that is why no sample ever
//!   decoded.
//! * **The tone spacing is three bins of a 4096-sample frame at 12 kHz** — 8.79
//!   Hz, the same at every speed (`spacing = 3`, `FSQ_SYMLEN = 4096`) — not the
//!   orthogonal grid this used to build, where the spacing was the baud rate.
//! * **The speeds are labels.** The symbol length is 8192, 6144, 4096, 3072 or
//!   2048 samples at 12 kHz for "FSQ-1.5 / 2 / 3 / 4.5 / 6", so the real rates
//!   are 1.46, 1.95, 2.93, 3.91 and 5.86 baud.
//!
//! The FSQCALL message layer is not ported: this is the raw text mode, with its
//! idle and its BOT/EOT markers decoded as text and nothing else.

use std::collections::VecDeque;

use crate::mfsk::{MfskClock, ToneGen};

include!("fsq_tables.rs");

const OUT_AMP: f32 = 0.5;
/// FSQ tone count.
const NUMTONES: usize = 33;
/// fldigi's FFT frame and the rate it is defined at: `FSQ_SYMLEN` samples at
/// 12 kHz, so a bin is 2.93 Hz and the carriers are three bins apart.
const FSQ_FRAME: f64 = 4096.0;
const FSQ_RATE: f64 = 12_000.0;
/// Carriers per frame.
const CARR_SEP: f64 = 3.0;
/// `idle` is symbols 28 then 30, which the receiver reads as the LF at nibble
/// 28 and drops.
const IDLE: [u8; 2] = [28, 30];

struct Geom {
    spacing: f64,
    sps: usize,
    base_hz: f64,
}

impl Geom {
    fn new(rate: f64, audio_hz: f64, speed: f64) -> Self {
        // The symbol length in samples at 12 kHz, as `adjust_for_speed` sets it.
        let symlen = match speed as i32 {
            1 => 8192.0,
            2 => 6144.0,
            4 => 3072.0, // 4.5 baud: the `else` branch in fldigi's own switch
            6 => 2048.0,
            _ => 4096.0, // 3 baud, the default
        };
        let sps = (rate * symlen / FSQ_RATE).round().max(16.0) as usize;
        let spacing = CARR_SEP * FSQ_RATE / FSQ_FRAME;
        let bw = NUMTONES as f64 * spacing;
        let base_hz = audio_hz - bw / 2.0;
        Geom { spacing, sps, base_hz }
    }

    fn tone_hz(&self, k: usize) -> f64 {
        self.base_hz + k as f64 * self.spacing
    }
}

// ─────────────────────────────── transmit ───────────────────────────────

pub struct FsqTx {
    rate: f64,
    g: Geom,
    tonegen: ToneGen,
    prev_tone: usize,
    /// One entry per symbol: the nibble, and the character index it completes.
    symq: VecDeque<(u8, Option<usize>)>,
    total_chars: usize,
    sent_chars: usize,
    cur: Vec<f32>,
    cur_pos: usize,
    cur_done: Option<usize>,
}

impl FsqTx {
    pub fn new(rate: f64, audio_hz: f64, baud: f64) -> Self {
        FsqTx {
            rate,
            g: Geom::new(rate, audio_hz, baud),
            tonegen: ToneGen::new(rate),
            prev_tone: 0,
            symq: VecDeque::new(),
            total_chars: 0,
            sent_chars: 0,
            cur: Vec::new(),
            cur_pos: 0,
            cur_done: None,
        }
    }

    pub fn set_params(&mut self, audio_hz: f64, baud: f64) {
        self.g = Geom::new(self.rate, audio_hz, baud);
    }

    pub fn push_text(&mut self, text: &str) {
        for ch in text.chars() {
            let byte = if ch.is_ascii() { ch as u8 } else { b'?' };
            let (a, b) = FSQ_VARICODE[byte as usize];
            if b > 28 {
                self.symq.push_back((a, None));
                self.symq.push_back((b, Some(self.total_chars)));
            } else {
                self.symq.push_back((a, Some(self.total_chars)));
            }
            self.total_chars += 1;
        }
    }

    pub fn sent_chars(&self) -> usize {
        self.sent_chars
    }
    pub fn total_chars(&self) -> usize {
        self.total_chars
    }
    pub fn drained(&self) -> bool {
        self.symq.is_empty() && self.cur_pos >= self.cur.len()
    }

    pub fn clear(&mut self) {
        self.symq.clear();
        self.cur.clear();
        self.cur_pos = 0;
        self.cur_done = None;
        self.total_chars = 0;
        self.sent_chars = 0;
    }

    fn build_symbol(&mut self) {
        let (sym, tag) = self.symq.pop_front().unwrap_or((IDLE[0], None));
        let tone = (self.prev_tone + sym as usize + 1) % NUMTONES;
        self.prev_tone = tone;
        self.cur.clear();
        self.cur_pos = 0;
        self.cur_done = tag;
        self.tonegen.emit(self.g.tone_hz(tone), self.g.sps, OUT_AMP, &mut self.cur);
    }

    pub fn next_block(&mut self, out: &mut [f32]) -> usize {
        let mut n = 0;
        while n < out.len() {
            if self.cur_pos >= self.cur.len() {
                if let Some(ci) = self.cur_done.take() {
                    self.sent_chars = ci + 1;
                }
                self.build_symbol();
            }
            out[n] = self.cur[self.cur_pos];
            self.cur_pos += 1;
            n += 1;
        }
        n
    }
}

// ─────────────────────────────── receive ───────────────────────────────

pub struct FsqRx {
    rate: f64,
    clock: MfskClock,
    prev_tone: Option<usize>,
    prev_nibble: usize,
}

impl FsqRx {
    pub fn new(rate: f64, audio_hz: f64, baud: f64) -> Self {
        let g = Geom::new(rate, audio_hz, baud);
        FsqRx {
            rate,
            clock: MfskClock::new(rate, g.base_hz, g.spacing, NUMTONES, g.sps),
            prev_tone: None,
            prev_nibble: 0,
        }
    }

    pub fn set_params(&mut self, audio_hz: f64, baud: f64) {
        let g = Geom::new(self.rate, audio_hz, baud);
        self.clock = MfskClock::new(self.rate, g.base_hz, g.spacing, NUMTONES, g.sps);
        self.prev_tone = None;
        self.prev_nibble = 0;
    }

    pub fn magnitude(&self) -> f32 {
        self.clock.peak_mag()
    }

    pub fn process(&mut self, audio: &[f32]) -> String {
        let mut out = String::new();
        for mags in self.clock.feed(audio) {
            let tone = mags
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map(|(i, _)| i)
                .unwrap_or(0);
            if let Some(prev) = self.prev_tone {
                // The transmitter sends `symbol + 1`, so the difference is one
                // more than the nibble; two turns of the bank guard a downward
                // step underflowing.
                let nibble = (tone + 2 * NUMTONES - prev - 1) % NUMTONES;
                // `fsq::process_symbol`: a nibble below 29 is a character on its
                // own; 29, 30 and 31 complete the one before it.
                let ch = if self.prev_nibble < 29 && nibble < 29 {
                    FSQ_VARIDECODE[self.prev_nibble]
                } else if self.prev_nibble < 29 && (29..=31).contains(&nibble) {
                    FSQ_VARIDECODE[self.prev_nibble * 32 + nibble]
                } else {
                    0
                };
                if ch != 0 {
                    out.push(ch as char);
                }
                self.prev_nibble = nibble;
            }
            self.prev_tone = Some(tone);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(speed: f64, msg: &str) -> String {
        let rate = 8000.0;
        let audio = 1500.0;
        let mut tx = FsqTx::new(rate, audio, speed);
        let mut sig = Vec::new();
        let mut warm = vec![0.0f32; tx.g.sps * 30];
        tx.next_block(&mut warm);
        sig.extend_from_slice(&warm);

        tx.push_text(msg);
        let mut guard = 0;
        while tx.sent_chars() < tx.total_chars() && guard < 200_000 {
            let mut b = [0.0f32; 2048];
            tx.next_block(&mut b);
            sig.extend_from_slice(&b);
            guard += 1;
        }
        let mut tail = vec![0.0f32; tx.g.sps * 40];
        tx.next_block(&mut tail);
        sig.extend_from_slice(&tail);

        let mut rx = FsqRx::new(rate, audio, speed);
        let mut decoded = String::new();
        for chunk in sig.chunks(512) {
            decoded.push_str(&rx.process(chunk));
        }
        decoded
    }

    #[test]
    fn loopback_fsq45() {
        let msg = "CQ DE AB1CD FSQ";
        let got = run(4.5, msg);
        assert!(got.contains(msg), "decoded {got:?} did not contain {msg:?}");
    }

    /// The three speeds the mode offers are three different symbol lengths, and
    /// each has to survive its own round trip — the grid is no longer the baud
    /// rate, so a speed that works can no longer imply the others do.
    #[test]
    fn every_speed_round_trips() {
        for speed in [2.0, 3.0, 4.5, 6.0] {
            let msg = "TEST THE EAGLE 12345";
            let got = run(speed, msg);
            assert!(got.contains(msg), "speed {speed} decoded {got:?}");
        }
    }

    /// The alphabet is fldigi's nibble table. A single-nibble character is one
    /// symbol, and the second nibble is only sent above 28 — which is what makes
    /// `prev * 32 + curr` the decode.
    #[test]
    fn the_varicode_is_the_reference_nibble_table() {
        assert_eq!(FSQ_VARICODE[b' ' as usize], (0, 0));
        assert_eq!(FSQ_VARICODE[b'a' as usize], (1, 0));
        assert_eq!(FSQ_VARICODE[b'A' as usize], (1, 29));
        assert_eq!(FSQ_VARIDECODE[0], b' ');
        assert_eq!(FSQ_VARIDECODE[1], b'a');
        // 'a' and 'A' share their first nibble and differ in the second, which
        // is the whole reason the decode is a two-dimensional lookup.
        assert_eq!(FSQ_VARIDECODE[1 * 32 + 29], b'A');
        assert_eq!(FSQ_VARIDECODE[0 * 32 + 29], b'@');
    }

    /// The corpus sample, measured rather than claimed.
    ///
    /// It does **not** read, and the sample is the suspect: an FSQ signal is one
    /// tone at a time, and this recording never has one dominant tone at any
    /// centre, any spacing or any speed — the point of the printout below. The
    /// manifest agrees: fldigi never read it back (`decoder: null,
    /// verdict: unverified`), so nothing has ever confirmed what went out.
    ///
    /// When a real FSQ recording arrives, this becomes a gate: flip the last
    /// assertion from 0 to the word count and add it to the journal.
    #[test]
    #[ignore]
    fn the_corpus_sample_is_measured_not_claimed() {
        let path = std::env::var("SDROXIDE_FSQ_SAMPLE").unwrap_or_else(|_| {
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../samples/fldigi/FSQ.wav").to_string()
        });
        let mut reader = hound::WavReader::open(&path).expect("the FSQ sample");
        let spec = reader.spec();
        let audio: Vec<f32> =
            reader.samples::<i16>().map(|s| s.expect("s") as f32 / 32_768.0).collect();
        let want: Vec<&str> = "EAGLE LANDED SDRBROWN".split_whitespace().collect();
        let mut best = (0usize, 0u32, 0.0f64);
        for speed in [1.5f64, 2.0, 3.0, 4.5, 6.0] {
            for hz in (1770..=1815).step_by(3) {
                let mut rx = FsqRx::new(spec.sample_rate as f64, hz as f64, speed);
                let mut out = String::new();
                for chunk in audio.chunks(512) {
                    out.push_str(&rx.process(chunk));
                }
                let hits = want.iter().filter(|w| out.contains(**w)).count();
                if hits > best.0 {
                    best = (hits, hz, speed);
                }
            }
        }
        eprintln!(
            "FSQ corpus: best {}/{} at {} Hz, {} baud (a real signal would read 3)",
            best.0,
            want.len(),
            best.1,
            best.2
        );
        assert_eq!(best.0, 0, "the sample reads now — make this the gate");
    }
}
