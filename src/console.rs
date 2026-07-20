//! Terminal waterfall: one printed line per spectrum frame.

use std::time::{Duration, Instant};

use sdroxide_dsp::{Complex32, SpectrumAnalyzer};
use sdroxide_radio::IqSource;

const GRADIENT: &[u8] = b" .:-=+*#%@";
/// Print a frequency ruler every this many waterfall lines.
const RULER_EVERY: u32 = 24;

pub struct Options {
    pub fft_size: usize,
    pub fps: u32,
    pub db_floor: f32,
    pub db_ceil: f32,
    pub width: usize,
}

pub fn run(mut source: Box<dyn IqSource>, opts: Options) -> anyhow::Result<()> {
    let rate = source.sample_rate();
    let center = source.center_hz();
    println!("source : {}", source.describe());
    println!(
        "span   : {:.3} MHz … {:.3} MHz  (center {:.3} MHz, {:.3} Msps)",
        (center - rate / 2.0) / 1e6,
        (center + rate / 2.0) / 1e6,
        center / 1e6,
        rate / 1e6
    );
    println!(
        "display: {} bins, FFT {}, {} lines/s, {} … {} dBFS   (Ctrl-C to quit)",
        opts.width, opts.fft_size, opts.fps, opts.db_floor, opts.db_ceil
    );

    let mut analyzer = SpectrumAnalyzer::new(opts.fft_size, rate, 0.3);
    let mut buf = vec![Complex32::default(); 16_384];
    let period = Duration::from_secs_f64(1.0 / opts.fps as f64);
    let mut next_line = Instant::now() + period;
    let mut lines: u32 = 0;

    loop {
        let n = source.read(&mut buf)?;
        analyzer.process(&buf[..n]);

        if Instant::now() >= next_line {
            next_line += period;
            if lines % RULER_EVERY == 0 {
                println!("{}", ruler(center, rate, opts.width));
            }
            lines += 1;

            let frame =
                analyzer.make_frame(center, rate, opts.db_floor, opts.db_ceil, opts.width, None);
            let mut line = String::with_capacity(opts.width + 16);
            for &b in &frame.bins {
                let idx = (b as usize * GRADIENT.len()) / 256;
                line.push(GRADIENT[idx.min(GRADIENT.len() - 1)] as char);
            }
            println!("{line}|{:6.1} dBFS", analyzer.take_peak_dbfs());
        }
    }
}

/// A frequency ruler like `|14.100        14.200        14.300|` fitted to `width`.
fn ruler(center: f64, span: f64, width: usize) -> String {
    let mut out = vec![b'-'; width];
    let labels = 5.min(width / 16).max(2);
    let mut annotations = Vec::new();
    for i in 0..labels {
        let frac = i as f64 / (labels - 1) as f64;
        let hz = center - span / 2.0 + frac * span;
        let text = format!("{:.4}", hz / 1e6);
        let pos = ((width - 1) as f64 * frac) as usize;
        annotations.push((pos, text));
    }
    for (pos, text) in annotations {
        let start = pos.min(width.saturating_sub(text.len()));
        for (i, ch) in text.bytes().enumerate() {
            if start + i < width {
                out[start + i] = ch;
            }
        }
    }
    String::from_utf8(out).unwrap()
}
