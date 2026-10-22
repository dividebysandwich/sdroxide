//! A bench signal generator: an Annex E coded STANAG 4285 transmission, as a WAV.
//!
//!     cargo run --release -p sdroxide-dsp --example s4285_test_signal -- \
//!         /tmp/opencode/s4285-tx.wav 600 "RYRY TEST RYRY"
//!
//! **This is not the mode's transmitter.** `Mode::Stanag4285` is receive-only and
//! stays that way: what this generates is a known-good signal to prove the
//! *receiver* against, over the air, when no coded station is on the band. Play it
//! into a rig on a sideband and the receiver should read its text back.
//!
//! What it sends is what Annex E says a transmitter sends: zeros to fill the
//! interleaver, then the SOM, the message bits, the EOM, and the flush — encoded,
//! pair-repeated, interleaved and modulated as 1200 baud BPSK on an 1800 Hz
//! sub-carrier.
//!
//! Two things are deliberately explicit. The **lead-in** exists because the
//! receiver starts mid-transmission when it acquires, and a convolutional
//! interleaver holds ten seconds of the stream: without it the receiver's
//! deinterleaver would still be filling when the message went past. And the
//! **flush** is the annex's own count for the rate, because a message is only
//! complete once the interleaver has pushed out everything behind it.

use sdroxide_dsp::stanag4285_fec as fec;

/// ITA2 letters, in the order the five-bit code numbers them.
const ITA2: [char; 32] = [
    ' ', 'E', '\n', 'A', ' ', 'S', 'I', 'U', '\r', 'D', 'R', 'J', 'N', 'F', 'C', 'K', 'T', 'Z',
    'L', 'W', 'H', 'Y', 'P', 'Q', 'O', 'B', 'G', ' ', 'M', 'X', 'V', ' ',
];

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a.is_empty() {
        eprintln!("usage: s4285_test_signal <out.wav> [rate_bps] [text]");
        std::process::exit(2);
    }
    let path = &a[0];
    let bps: f64 = a.get(1).and_then(|v| v.parse().ok()).unwrap_or(600.0);
    let text =
        a.get(2).cloned().unwrap_or_else(|| "RYRY TEST RYRY THE EAGLE HAS LANDED".to_string());
    let rate = match bps as u32 {
        600 => fec::CodedRate::Bps600,
        300 => fec::CodedRate::Bps300,
        150 => fec::CodedRate::Bps150,
        75 => fec::CodedRate::Bps75,
        other => {
            eprintln!("rate {other} is not a BPSK coded rate (600, 300, 150, 75)");
            std::process::exit(2);
        }
    };

    // The message: SOM, ITA2 5N1.5, EOM.
    let mut bits: Vec<u8> = Vec::new();
    for k in (0..32).rev() {
        bits.push(((fec::SOM >> k) & 1) as u8);
    }
    for c in text.to_uppercase().chars() {
        let v = match ITA2.iter().position(|x| *x == c) {
            Some(v) => v as u8,
            None => {
                eprintln!("'{c}' is not in the ITA2 letters table");
                std::process::exit(2);
            }
        };
        bits.push(0); // start
        for i in 0..5 {
            bits.push((v >> i) & 1);
        }
        bits.push(1); // one and a half stop bits, sampled as two
        bits.push(1);
    }
    for k in (0..32).rev() {
        bits.push(((fec::EOM >> k) & 1) as u8);
    }

    // Annex E's flush, by rate (the 10.24 s interleave column).
    let flush = match rate {
        fec::CodedRate::Bps600 => 6246,
        fec::CodedRate::Bps300 => 3174,
        fec::CodedRate::Bps150 => 1638,
        _ => 870,
    };
    let lead = (rate.bps() * 11.0) as usize; // eleven seconds of zeros
    let mut data = vec![0u8; lead];
    let message_at = data.len();
    data.extend_from_slice(&bits);
    data.extend(std::iter::repeat_n(0u8, flush));

    let coded = fec::encode(&data);
    let repeated = fec::repeat_pairs(&coded, rate.repeat());
    let mut il = fec::ConvInterleaver::interleaver(rate.j());
    let stream: Vec<u8> = repeated
        .iter()
        .map(|b| {
            let v = il.push(if *b == 1 { -1.0 } else { 1.0 });
            if v < 0.0 { 1 } else { 0 }
        })
        .collect();

    if let Ok(path) = std::env::var("S4285_TX_STREAM") {
        let t: String = stream.iter().map(|b| if *b == 1 { '1' } else { '0' }).collect();
        std::fs::write(path, t).expect("write the stream");
    }

    // 4285 framing: 80 sync symbols, then 32 data + 16 reference three times and a
    // final 32 data — all scrambled except the sync.
    let sync = sdroxide_dsp::stanag4285::sync_symbols();
    let scr = sdroxide_dsp::stanag4285::scrambling();
    let frames = stream.len().div_ceil(128) + 2;
    let mut symbols: Vec<f32> = Vec::with_capacity(frames * 256);
    for f in 0..frames {
        for s in sync.iter() {
            symbols.push(if *s > 0.0 { 0.0 } else { 4.0 });
        }
        let mut d = 0;
        for k in 0..176 {
            let reference = matches!(k, 32..=47 | 80..=95 | 128..=143);
            let sym = if reference {
                0.0
            } else {
                let b = stream.get(f * 128 + d).copied().unwrap_or(0);
                d += 1;
                if b == 1 { 4.0 } else { 0.0 }
            };
            let n = (sym + scr[k].arg() / std::f32::consts::FRAC_PI_4).rem_euclid(8.0);
            symbols.push(n);
        }
    }

    // 1200 baud BPSK on 1800 Hz, raised-cosine pulses, 48 kHz out.
    let out_rate = 48_000.0;
    let sps = (out_rate / 2400.0) as usize; // 20 samples per symbol
    let mut base = vec![sdroxide_dsp::stanag4285::C::default(); symbols.len() * sps];
    for (i, s) in symbols.iter().enumerate() {
        base[i * sps] = sdroxide_dsp::stanag4285::C::from_phase(s * std::f32::consts::FRAC_PI_4);
    }
    // 12 % excess bandwidth, not 30 %. A CB rig's transmit passband is about
    // 300–3000 Hz, so a 3 kHz-wide signal at 1800 Hz is cut at both edges: the
    // reference blocks still collapse (16 identical symbols have no ISI to
    // suffer) while the data does not decode. Narrow the pulse and centre it
    // lower, and the rig passes it whole.
    let alpha: f32 =
        std::env::var("S4285_TX_ALPHA").ok().and_then(|v| v.parse().ok()).unwrap_or(0.12);
    let taps: Vec<f32> = (-40i32..=40)
        .map(|i| {
            let x = i as f32 / sps as f32;
            let sinc = if x.abs() < 1e-6 {
                1.0
            } else {
                (std::f32::consts::PI * x).sin() / (std::f32::consts::PI * x)
            };
            let d = 1.0 - (2.0 * alpha * x).powi(2);
            let rc = if d.abs() < 1e-6 {
                std::f32::consts::FRAC_PI_4 * (std::f32::consts::PI * x / 2.0).sin()
            } else {
                (std::f32::consts::PI * alpha * x).cos() / d
            };
            sinc * rc
        })
        .collect();
    let mut shaped = vec![sdroxide_dsp::stanag4285::C::default(); base.len()];
    for (i, s) in base.iter().enumerate() {
        if s.norm() < 1e-9 {
            continue;
        }
        for (j, t) in taps.iter().enumerate() {
            let k = i as isize + j as isize - 40;
            if k >= 0 && (k as usize) < shaped.len() {
                shaped[k as usize] = shaped[k as usize].add(s.scale(*t));
            }
        }
    }
    // A modest level: the rig keys by VOX and 2 W is already more than this path
    // needs. -22 dBFS with the card at 96 % is where the last bench session had it.
    let audio: Vec<f32> = shaped
        .iter()
        .enumerate()
        .map(|(i, z)| {
            let carrier: f64 = std::env::var("S4285_TX_CARRIER")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1500.0);
            let ph = (std::f32::consts::TAU as f64 * carrier * i as f64 / out_rate)
                .rem_euclid(std::f32::consts::TAU as f64) as f32;
            let level: f32 =
                std::env::var("S4285_TX_LEVEL").ok().and_then(|v| v.parse().ok()).unwrap_or(0.05);
            level * z.mul(sdroxide_dsp::stanag4285::C::from_phase(ph)).re
        })
        .collect();

    // A transmit path whose audio comes out at the wrong pitch — a cheap rig's
    // USB codec measured here at ×1.2124 by sending a 1 kHz tone and reading
    // 1212 Hz — is pre-compensated by running the whole waveform that much
    // faster, so what leaves the antenna is 2400 baud at its own carrier. The
    // receive path is not involved: a real station measured 2400.0 baud on it.
    let scaled: Vec<f32> =
        match std::env::var("S4285_TX_SCALE").ok().and_then(|v| v.parse::<f64>().ok()) {
            Some(k) if k > 0.0 && (k - 1.0).abs() > 1e-6 => {
                // Stretch, not squeeze: the path multiplies frequency by k, so the
                // file has to be k times *longer* for the antenna to carry the
                // waveform at its own rate.
                let n = (audio.len() as f64 * k) as usize;
                (0..n)
                    .map(|i| {
                        let x = i as f64 / k;
                        let j = x.floor() as usize;
                        let f = (x - j as f64) as f32;
                        let a = audio.get(j).copied().unwrap_or(0.0);
                        let b = audio.get(j + 1).copied().unwrap_or(a);
                        a + (b - a) * f
                    })
                    .collect()
            }
            _ => audio.clone(),
        };
    let audio = scaled;

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: out_rate as u32,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(path, spec).expect("create the wav");
    for v in &audio {
        w.write_sample((v.clamp(-1.0, 1.0) * 32767.0) as i16).expect("write");
    }
    w.finalize().expect("finalise");

    let hex: String = bits
        .chunks(8)
        .map(|b| {
            let mut v = 0u8;
            for (i, x) in b.iter().enumerate() {
                v |= x << (7 - i);
            }
            format!("{v:02x}")
        })
        .collect();
    println!("message bits ({}): {hex}", bits.len());
    let secs = audio.len() as f64 / out_rate;
    println!(
        "{path}: {secs:.1} s, {} bps coded, message at {:.1} s ({message_at} data bits in), \
         text {:?}",
        rate.label(),
        message_at as f64 / rate.bps(),
        text
    );
}
