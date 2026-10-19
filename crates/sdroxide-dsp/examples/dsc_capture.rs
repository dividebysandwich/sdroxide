//! Decode a raw CF32 I/Q capture of a DSC channel with sdroxide's own chain.
//!
//! Usage: `dsc_capture capture.cs16 capture_rate_hz [audio_offset_hz]`
//!
//! The point is the real-burst check the synthetic round-trip tests cannot
//! make: does the detector recover a sequence off the air. The chain is the
//! engine's own — a DDC to a 48 kHz channel, an SSB demodulator over the audio
//! passband, the same `DscRx` the mode builds.
//!
//! `audio_offset_hz` (default 1700) is where the capture's centre frequency
//! should appear in the audio, because a DSC channel is quoted as its
//! *assigned* frequency — the centre of the J2B tone pair — and the MF/HF
//! tones are 1615 and 1785 Hz either side of it (VHF: 1300/2100 Hz). Captured with the assigned channel
//! at the centre, that offset is 1700.

use std::io::Read;

use sdroxide_dsp::{Complex32, Ddc, DscRx, make_demod};

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: dsc_capture <cf32> <rate_hz> [offset_hz]");
    let in_rate: f64 = args.next().expect("rate_hz").parse().expect("rate");
    let offset_hz: f64 = args.next().unwrap_or_else(|| "1700".into()).parse().expect("offset");

    let mut raw = Vec::new();
    std::fs::File::open(&path).expect("open capture").read_to_end(&mut raw).expect("read");
    let iq: Vec<Complex32> = raw
        .chunks_exact(8)
        .map(|b| {
            Complex32::new(
                f32::from_le_bytes([b[0], b[1], b[2], b[3]]),
                f32::from_le_bytes([b[4], b[5], b[6], b[7]]),
            )
        })
        .collect();
    eprintln!("{} samples at {in_rate:.0} Hz ({:.1}s)", iq.len(), iq.len() as f64 / in_rate);

    let mut ddc = Ddc::new(in_rate, 48_000.0);
    // `set_offset_hz` mixes a signal `offset` above the hardware centre down to
    // DC, so to *place* the centre at `+offset_hz` audio the mix is the other
    // way: a negative offset.
    ddc.set_offset_hz(-offset_hz);
    let rate = ddc.out_rate();
    eprintln!("channel rate {rate:.1} Hz, centre at {offset_hz:.0} Hz audio");

    let mut demod =
        make_demod(sdroxide_types::Mode::Dsc, rate).expect("the DSC mode has a demodulator");
    let mut rx = DscRx::new(rate);
    let mut base = Vec::new();
    let mut audio = Vec::new();
    let mut out = Vec::new();
    let mut seqs = 0u64;
    let mut peak = 0.0f32;
    let mut sum = 0.0f64;
    let mut nsamp = 0u64;
    for block in iq.chunks(16_384) {
        base.clear();
        ddc.process(block, &mut base);
        if base.is_empty() {
            continue;
        }
        audio.clear();
        demod.process(&base, &mut audio);
        for &a in &audio {
            peak = peak.max(a.abs());
            sum += f64::from(a) * f64::from(a);
            nsamp += 1;
        }
        rx.process(&audio, &mut out);
        for m in out.drain(..) {
            seqs += 1;
            println!("{}", m.summary());
        }
    }
    rx.flush(&mut out);
    for m in out.drain(..) {
        seqs += 1;
        println!("{}", m.summary());
    }
    eprintln!(
        "audio peak {peak:.4} rms {:.4}, separation {:.3}",
        (sum / nsamp.max(1) as f64).sqrt(),
        rx.separation()
    );
    eprintln!("{seqs} sequences");
}
