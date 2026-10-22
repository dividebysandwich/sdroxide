//! Measure a coded-PSK recording and decode it, so "it does not decode" can be
//! told apart from "we looked in the wrong place".
//!
//!     cargo run --release -p sdroxide-dsp --example psk_coded_probe -- \
//!         samples/fldigi/PSK500C4.wav 500 4 none
//!     cargo run --release -p sdroxide-dsp --example psk_coded_probe -- \
//!         samples/fldigi/PSK250RC5.wav 250 5 160 1760
//!
//! Arguments: `<wav> <baud> <carriers> <depth|none> [<centre_hz>]`
//!
//! With no centre the band edges are measured and every centre the band admits
//! is tried, coarsely and then finely; with a centre, only that one is decoded
//! and the whole text is printed.
//!
//! **Why this exists.** The corpus generator pushed its wide banks up the band
//! (a four-carrier 500-baud signal is 2.8 kHz wide and does not fit below
//! 4 kHz when centred where the narrow modes are), and the corpus test searched
//! a fixed list of centres that tops out at 2250 Hz. Three modes were recorded
//! as "reads nothing" because of that. The band edges are measured from the
//! audio here rather than read from the manifest, whose `centre_hz` is a block
//! estimate and whose `peak_hz` is a sideband edge.

fn read_wav(path: &str) -> (Vec<f32>, f64) {
    let mut reader = hound::WavReader::open(path).expect("open the wav");
    let spec = reader.spec();
    let audio = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.expect("s")).collect(),
        hound::SampleFormat::Int => {
            reader.samples::<i16>().map(|s| s.expect("s") as f32 / 32_768.0).collect()
        }
    };
    (audio, spec.sample_rate as f64)
}

/// A Welch-averaged power spectrum over the active part of the recording: the
/// first and last tenth are skipped, because the sweep's own window edges are
/// not the transmission.
fn spectrum(audio: &[f32], rate: f64) -> (Vec<f64>, Vec<f64>) {
    use rustfft::FftPlanner;
    use rustfft::num_complex::Complex32;

    let nfft = 8192usize;
    let hop = 4096usize;
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(nfft);
    let window: Vec<f32> = (0..nfft)
        .map(|i| {
            let x = std::f32::consts::TAU * i as f32 / (nfft - 1) as f32;
            0.5 - 0.5 * x.cos()
        })
        .collect();
    let lo = audio.len() / 10;
    let hi = audio.len() - audio.len() / 10;
    let mut acc = vec![0.0f64; nfft / 2 + 1];
    let mut n = 0usize;
    let mut buf = vec![Complex32::new(0.0, 0.0); nfft];
    let mut i = lo;
    while i + nfft < hi {
        for (k, b) in buf.iter_mut().enumerate() {
            *b = Complex32::new(audio[i + k] * window[k], 0.0);
        }
        fft.process(&mut buf);
        for (k, a) in acc.iter_mut().enumerate() {
            *a += buf[k].norm_sqr() as f64;
        }
        n += 1;
        i += hop;
    }
    if n > 0 {
        for a in acc.iter_mut() {
            *a /= n as f64;
        }
    }
    let freqs = (0..=nfft / 2).map(|k| k as f64 * rate / nfft as f64).collect();
    (freqs, acc)
}

/// Where the signal's own band starts and ends, as a fraction of the peak power.
fn band_edges(freqs: &[f64], power: &[f64], frac: f64) -> (f64, f64) {
    let peak = power.iter().cloned().fold(0.0, f64::max);
    let over: Vec<f64> =
        freqs.iter().zip(power).filter(|(_, p)| **p >= peak * frac).map(|(f, _)| *f).collect();
    match (over.first(), over.last()) {
        (Some(a), Some(b)) => (*a, *b),
        _ => (0.0, 0.0),
    }
}

fn decode(
    audio: &[f32],
    rate: f64,
    hz: f64,
    baud: f64,
    carriers: usize,
    depth: Option<usize>,
) -> String {
    let mut out = String::new();
    // A second of silence flushes the tail: the interleaver's last symbols and
    // the Viterbi's path memory are still in the pipeline when the audio ends.
    let pad = vec![0.0f32; rate as usize];
    let samples: Vec<f32> = audio.iter().chain(pad.iter()).copied().collect();
    if let Some(depth) = depth {
        let mut rx = sdroxide_dsp::PskrRx::with_carriers(rate, hz, baud, carriers, Some(depth));
        for chunk in samples.chunks(512) {
            out.push_str(&rx.process(chunk));
        }
        out.push_str(&rx.flush());
    } else {
        let mut rx = sdroxide_dsp::PskMultiRx::new(rate, hz, baud, carriers);
        for chunk in samples.chunks(512) {
            out.push_str(&rx.process(chunk));
        }
    }
    out
}

fn hits(text: &str) -> usize {
    "EAGLE LANDED SDRBROWN".split_whitespace().filter(|w| text.contains(w)).count()
}

fn printable(text: &str) -> String {
    let s: String = text.chars().filter(|c| c.is_ascii_graphic() || *c == ' ').collect();
    // The good text comes *after* the lock, so the head of the string is not
    // where to look: show where each known word lands, and the window around the
    // first one found.
    let at = |w: &str| s.find(w).map(|i| i.to_string()).unwrap_or_else(|| "-".into());
    let marks = format!(
        "[{} chars] AAA@{} EAGLE@{} LANDED@{} SDRBROWN@{}",
        s.chars().count(),
        at("AAA"),
        at("EAGLE"),
        at("LANDED"),
        at("SDRBROWN")
    );
    match s.find("AAA").or_else(|| s.find("EAGLE")) {
        Some(i) => {
            let end = (i + 110).min(s.len());
            format!("{marks} …{}…", &s[i..end])
        }
        None => {
            let head: String = s.chars().take(100).collect();
            format!("{marks} head: {head}")
        }
    }
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a.len() < 4 {
        eprintln!("usage: psk_coded_probe <wav> <baud> <carriers> <depth|none> [<centre_hz>]");
        std::process::exit(2);
    }
    let (audio, rate) = read_wav(&a[0]);
    let baud: f64 = a[1].parse().expect("baud");
    let carriers: usize = a[2].parse().expect("carriers");
    let depth = match a[3].as_str() {
        "none" => None,
        d => Some(d.parse().expect("depth")),
    };
    let (freqs, power) = spectrum(&audio, rate);
    let (lo, hi) = band_edges(&freqs, &power, 0.05);
    let want_w = carriers as f64 * 1.4 * baud;
    println!(
        "{}: {:.1} s at {:.0} Hz, {} carriers x {} baud (bank {:.0} Hz, edge-to-edge {:.0})",
        a[0],
        audio.len() as f64 / rate,
        rate,
        carriers,
        baud,
        want_w,
        (carriers as f64 - 1.0) * 1.4 * baud + baud
    );
    println!(
        "  band -13 dB: {lo:.0} .. {hi:.0} Hz  (centre {:.0}, width {:.0})",
        (lo + hi) / 2.0,
        hi - lo
    );

    if let Some(centre) = a.get(4).and_then(|v| v.parse::<f64>().ok()) {
        let text = decode(&audio, rate, centre, baud, carriers, depth);
        println!("  at {centre:.0} Hz: {}/3 :: {}", hits(&text), printable(&text));
        return;
    }

    // The bank's own extent is (carriers-1) x 1.4 x baud + baud wide, so a
    // centre can only lie between half of that and Nyquist minus it.
    let half = ((carriers as f64 - 1.0) * 1.4 * baud + baud) / 2.0;
    let lowest = (half + 50.0).max(lo - 200.0);
    let highest = (rate / 2.0 - half - 50.0).min(hi + 200.0);
    println!("  centres tried from {lowest:.0} to {highest:.0} Hz");

    // Coarse, then fine around the best the coarse pass found. The longest text
    // is kept alongside the score: a mode that reads no known word still says
    // something about why, and an empty string at every centre is a different
    // fault from a mistuned one.
    let mut best = (0usize, 0.0f64, String::new());
    let mut longest = (0usize, 0.0f64, String::new());
    let mut hz = lowest;
    while hz <= highest {
        let text = decode(&audio, rate, hz, baud, carriers, depth);
        let h = hits(&text);
        if text.chars().count() > longest.0 {
            longest = (text.chars().count(), hz, text.clone());
        }
        if h > best.0 || (h == best.0 && text.chars().count() > best.2.chars().count()) {
            best = (h, hz, text.clone());
            println!("    {hz:7.0} Hz {h}/3 :: {}", printable(&best.2));
        }
        hz += 25.0;
    }
    let coarse = best.1;
    let mut hz = (coarse - 25.0).max(lowest);
    while hz <= (coarse + 25.0).min(highest) {
        let text = decode(&audio, rate, hz, baud, carriers, depth);
        let h = hits(&text);
        if text.chars().count() > longest.0 {
            longest = (text.chars().count(), hz, text.clone());
        }
        if h > best.0 || (h == best.0 && text.chars().count() > best.2.chars().count()) {
            best = (h, hz, text.clone());
            println!("    {hz:7.0} Hz {h}/3 :: {}", printable(&best.2));
        }
        hz += 2.0;
    }
    println!("  BEST {}/3 at {:.0} Hz\n  text: {}", best.0, best.1, printable(&best.2));
    if longest.1 != best.1 {
        println!("  longest at {:.0} Hz: {}", longest.1, printable(&longest.2));
    }
}
