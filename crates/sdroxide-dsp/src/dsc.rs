//! DSC's audio front end: 1200-baud FFSK in, messages out.
//!
//! The link layer of Digital Selective Calling is nothing exotic — it is the
//! same non-coherent binary FSK the packet modes use. VHF (channel 70) runs
//! mark 1300 Hz, space 2100 Hz at 1200 baud; MF/HF (2187.5 kHz and the HF
//! channels) runs 1615/1785 Hz at 100 baud. Both detectors run side by side. So the detector is
//! [`crate::afsk::AfskRx`] with the [`AfskProfile::Dsc`] profile, and what this
//! module adds is the one thing packet does not do: it feeds the recovered bits
//! into [`sdroxide_types::DscFramer`] and hands back messages.
//!
//! Unlike AX.25 there is **no NRZI line code**: DSC is direct FSK, so the
//! slicer's bit is the wire bit and nothing has to be differential-decoded. The
//! phasing run at the head of every sequence is a long stretch of the phasing
//! character, which gives the clock plenty of transitions to acquire on — the
//! same job HDLC's stuffing does for packet, done here by the protocol itself.
//!
//! Audio is resampled to a fixed rate the way [`crate::acars::AcarsRx`] does,
//! so the detector's constants are fixed rather than re-derived per rate. The
//! detector wants the tones well inside its band, so the rate is comfortably
//! above twice the higher tone — 9600 Hz is 1200 × 8 and puts the 2100 Hz space
//! tone at a fraction of Nyquist.

use sdroxide_types::{DscFramer, DscMessage};

use crate::afsk::{AfskProfile, AfskRx};
use crate::resample::MonoResampler;

/// The rate the detector runs at: 1200 baud × 8, matching the packet front
/// ends' oversampling.
const DEMOD_RATE: f64 = 9_600.0;

/// A DSC receiver: audio samples in, decoded messages out.
pub struct DscRx {
    /// The FSK detector, at the DSC tone pair.
    fsk: AfskRx,
    /// MF/HF DSC: 100 baud, 1615/1785 Hz, run beside the VHF detector.
    fsk_hf: AfskRx,
    bits_hf: Vec<bool>,
    framer_hf: DscFramer,
    /// Resamples the input to [`DEMOD_RATE`]; `None` when it already is.
    rs: Option<MonoResampler>,
    rs_buf: Vec<f32>,
    /// Reused bit scratch, so the hot path does not allocate.
    bits: Vec<bool>,
    /// The DX/RX framer over the recovered bits.
    framer: DscFramer,
    /// Smoothed audio level, for a panel meter.
    level: f32,
    /// Sequences whose end-of-sequence character was reached.
    sequences: u64,
}

impl DscRx {
    pub fn new(rate: f64) -> Self {
        DscRx {
            fsk: AfskRx::new(DEMOD_RATE, AfskProfile::Dsc),
            fsk_hf: AfskRx::new(DEMOD_RATE, AfskProfile::DscHf),
            bits_hf: Vec::new(),
            framer_hf: DscFramer::new_hf(),
            rs: MonoResampler::new(rate, DEMOD_RATE),
            rs_buf: Vec::new(),
            bits: Vec::new(),
            framer: DscFramer::new(),
            level: 0.0,
            sequences: 0,
        }
    }

    /// The detector's input level, for a panel meter.
    pub fn level(&self) -> f32 {
        self.level
    }

    /// Complete sequences seen, good and marginal.
    pub fn sequences(&self) -> u64 {
        self.sequences
    }

    /// The FSK detector's confidence in its mark/space separation, `0..1`.
    pub fn separation(&self) -> f32 {
        self.fsk.separation().max(self.fsk_hf.separation())
    }

    /// Feed audio and collect every message it completed.
    pub fn process(&mut self, audio: &[f32], out: &mut Vec<DscMessage>) {
        // Level before resampling: it is the input the operator's meter shows.
        for &a in audio {
            self.level += 0.01 * (a.abs() - self.level);
        }

        let block: &[f32] = match &mut self.rs {
            Some(rs) => {
                self.rs_buf.clear();
                rs.push(audio, &mut self.rs_buf);
                &self.rs_buf
            }
            None => audio,
        };

        self.bits.clear();
        self.fsk.process(block, &mut self.bits);
        self.bits_hf.clear();
        self.fsk_hf.process(block, &mut self.bits_hf);
        self.feed_framer(out);
    }

    /// Flush the resampler and detector at the end of a burst, so the sequence
    /// tail reaches the framer.
    ///
    /// A live stream never needs this — the audio that follows a burst pushes
    /// the tail out — but a file or batch decode ends with the audio, and the
    /// withheld tail holds the end-of-sequence character the framer needs to
    /// close a sequence.
    pub fn flush(&mut self, out: &mut Vec<DscMessage>) {
        self.bits.clear();
        self.bits_hf.clear();
        if let Some(rs) = &mut self.rs {
            let mut tail = Vec::new();
            rs.flush(&mut tail);
            self.fsk.process(&tail, &mut self.bits);
            self.fsk_hf.process(&tail, &mut self.bits_hf);
        }
        self.fsk.flush(&mut self.bits);
        self.fsk_hf.flush(&mut self.bits_hf);
        self.feed_framer(out);
    }

    fn feed_framer(&mut self, out: &mut Vec<DscMessage>) {
        for &bit in &self.bits {
            let before = self.framer.sequences();
            self.framer.push(u8::from(bit), out);
            if self.framer.sequences() > before {
                self.sequences += 1;
            }
        }
        for &bit in &self.bits_hf {
            let before = self.framer_hf.sequences();
            self.framer_hf.push(u8::from(bit), out);
            if self.framer_hf.sequences() > before {
                self.sequences += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::afsk::{AfskProfile, AfskRx};
    use sdroxide_types::{DSC_PHASING_SYMBOL, DscFormat, DscNature, dsc_bch};

    fn char_bits(symbol: u8) -> Vec<bool> {
        let cw = dsc_bch::encode(symbol);
        (0..10).rev().map(|i| (cw >> i) & 1 != 0).collect()
    }

    fn modulate(bits: &[bool], rate: f64) -> Vec<f32> {
        let spb = (rate / 1200.0).round() as usize;
        let mut out = Vec::with_capacity(bits.len() * spb);
        let mut phase = 0.0f64;
        for &b in bits {
            let freq = if b { 1300.0 } else { 2100.0 };
            let inc = std::f64::consts::TAU * freq / rate;
            for _ in 0..spb {
                out.push(phase.sin() as f32);
                phase += inc;
                if phase > std::f64::consts::TAU {
                    phase -= std::f64::consts::TAU;
                }
            }
        }
        out
    }

    fn sequence_bits(body: &[u8]) -> Vec<bool> {
        let mut bits = Vec::new();
        for i in 0..8u8 {
            bits.extend(char_bits(DSC_PHASING_SYMBOL));
            bits.extend(char_bits(111 - i));
        }
        for &s in body {
            bits.extend(char_bits(s));
            bits.extend(char_bits(s));
        }
        bits.extend(char_bits(127));
        bits
    }

    #[test]
    #[ignore = "diagnostic: prints the detector's dropped-tail result"]
    fn diag_drift_and_framer() {
        let body = [112u8, 36, 61, 23, 45, 60, 105, 0, 51, 30, 0, 7, 12, 34];
        let bits = sequence_bits(&body);
        // No trailing silence: `flush` releases the FIR group delay's tail,
        // which otherwise holds back the last ~16 bits (the end-of-sequence
        // character).
        let audio = modulate(&bits, DEMOD_RATE);
        let mut fsk = AfskRx::new(DEMOD_RATE, AfskProfile::Dsc);
        let mut got = Vec::new();
        fsk.process(&audio, &mut got);
        fsk.flush(&mut got);
        let mut best = (0usize, usize::MAX);
        for o in 0..30 {
            let w: usize = (0..got.len().min(bits.len().saturating_sub(o)))
                .filter(|&i| got[i] != bits[o + i])
                .count();
            if w < best.1 {
                best = (o, w);
            }
        }
        let off = best.0;
        let n = got.len().min(bits.len().saturating_sub(off));
        let wrong = (0..n).filter(|&i| got[i] != bits[off + i]).count();
        eprintln!(
            "sent {} got {} sep {:.3} offset {} wrong {}/{}",
            bits.len(),
            got.len(),
            fsk.separation(),
            off,
            wrong,
            n
        );
        let mut f1 = sdroxide_types::DscFramer::new();
        let mut o1 = Vec::new();
        for &b in &got {
            f1.push(u8::from(b), &mut o1);
        }
        eprintln!("framer on recovered: {} messages", o1.len());
        let mut f2 = sdroxide_types::DscFramer::new();
        let mut o2 = Vec::new();
        for &b in &bits[off..] {
            f2.push(u8::from(b), &mut o2);
        }
        eprintln!("framer on sent[{off}..]: {} messages", o2.len());
        // Pad the recovered stream with the sent tail it is missing.
        let mut padded = got.clone();
        let start = off + got.len();
        if start < bits.len() {
            padded.extend_from_slice(&bits[start..]);
        }
        let mut f3 = sdroxide_types::DscFramer::new();
        let mut o3 = Vec::new();
        for &b in &padded {
            f3.push(u8::from(b), &mut o3);
        }
        eprintln!("framer on recovered+tail: {} messages", o3.len());
    }

    #[test]
    fn a_distress_alert_round_trips_through_audio() {
        let mut body = vec![112u8];
        body.extend([36, 61, 23, 45, 60]);
        body.push(105);
        body.extend([0, 51, 30, 0, 7]);
        body.extend([12, 34]);
        let bits = sequence_bits(&body);
        let audio = modulate(&bits, 48_000.0);
        let mut rx = DscRx::new(48_000.0);
        let mut out = Vec::new();
        for chunk in audio.chunks(2048) {
            rx.process(chunk, &mut out);
        }
        rx.flush(&mut out);
        assert!(!out.is_empty(), "no sequence decoded");
        let m = out.iter().find(|m| m.format == DscFormat::Distress).expect("a distress alert");
        assert_eq!(m.self_mmsi, 366_123_456);
        assert_eq!(m.nature, DscNature::Sinking);
    }

    #[test]
    fn a_routine_call_round_trips_at_the_demod_rate() {
        let body = [120u8, 24, 41, 23, 45, 60, 100, 36, 61, 23, 45, 60];
        let bits = sequence_bits(&body);
        let audio = modulate(&bits, DEMOD_RATE);
        let mut rx = DscRx::new(DEMOD_RATE);
        let mut out = Vec::new();
        rx.process(&audio, &mut out);
        rx.flush(&mut out);
        assert_eq!(out.len(), 1, "expected one sequence");
        assert_eq!(out[0].format, DscFormat::Individual);
        assert_eq!(out[0].target_mmsi, 244_123_456);
    }

    /// MF/HF DSC as it is heard on 2187.5 kHz: 100 baud, 1615/1785 Hz (here
    /// with the sense inverted, as a USB capture delivers it), DX/RX interleaved
    /// with the phasing character in the RX slots, and the format specifier sent
    /// twice. Layout taken from a recording of a real coast-station call.
    #[test]
    fn an_mf_hf_call_at_100_baud_decodes() {
        let dx_phasing = [111u8, 110, 109, 108, 107, 106, 105, 104];
        let body = [120u8, 120, 0, 23, 20, 0, 10, 100, 50, 30, 0, 10, 0, 0, 50, 10, 30, 117];
        let mut dx: Vec<u8> = dx_phasing.to_vec();
        dx.extend(body);
        let mut bits: Vec<bool> = (0..200).map(|i| i % 2 == 0).collect();
        for (i, &d) in dx.iter().enumerate() {
            bits.extend(char_bits(d));
            let rx = if i < 5 { DSC_PHASING_SYMBOL } else { dx[i - 5] };
            bits.extend(char_bits(rx));
        }
        // 100 baud, tone pair 1615/1785; a set bit is the lower tone.
        let rate = 48_000.0;
        let spb = (rate / 100.0) as usize;
        let mut audio = Vec::new();
        let mut phase = 0.0f64;
        for &b in &bits {
            let f = if b { 1615.0 } else { 1785.0 };
            for _ in 0..spb {
                audio.push(phase.sin() as f32);
                phase += std::f64::consts::TAU * f / rate;
            }
        }
        let mut rx = DscRx::new(rate);
        let mut out = Vec::new();
        for chunk in audio.chunks(2048) {
            rx.process(chunk, &mut out);
        }
        rx.flush(&mut out);
        let m = out.first().expect("no MF/HF sequence decoded");
        assert_eq!(m.format, DscFormat::Individual);
        assert_eq!(m.target_mmsi, 2_320_001);
    }
}
