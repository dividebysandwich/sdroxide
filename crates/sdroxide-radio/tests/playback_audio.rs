//! What a playback sounds like at the speaker, end to end through the engine.
//!
//! The first build of playback was reported "chopped, half of it missing": the
//! source handed the engine a third of a second at a time, and then nothing for
//! a third of a second. A sound card hands over ten milliseconds at a time, and
//! the paths after the engine are sized for that — the server's audio lane to a
//! browser holds eight 20 ms frames, so a 340 ms lump lost more than half of
//! itself there. It also played one ear in both, so a WFM stereo recording came
//! back mono. This listens to the speaker ring itself and checks all three: the
//! audio arrives steadily, keeps up with the clock, and the right ear is the
//! recording's right ear.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use sdroxide_radio::{AudioParams, Complex32, EngineConfig, IqSource, Result, start_engine};
use sdroxide_types::{Command, DeviceCaps, PlaybackCommand};

const RATE: f64 = 48_000.0;
const NAME: &str = "stereo.wav";

/// The live radio: silence, paced.
struct Live;

impl IqSource for Live {
    fn sample_rate(&self) -> f64 {
        RATE
    }
    fn center_hz(&self) -> f64 {
        7_100_000.0
    }
    fn set_center_hz(&mut self, _hz: f64) -> Result<()> {
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        std::thread::sleep(Duration::from_millis(5));
        let n = buf.len().min(256);
        buf[..n].fill(Complex32::new(0.0, 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "live rig".into()
    }
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "live".into(),
        label: "live".into(),
        rx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(0.0, 60_000_000.0)],
        ..DeviceCaps::default()
    }
}

/// A 16-bit stereo WAV: 1 kHz in the left ear, 400 Hz in the right.
fn write_stereo_wav(path: &std::path::Path, secs: f64) {
    let rate = RATE as u32;
    let n = (RATE * secs) as usize;
    let mut data = Vec::with_capacity(n * 4);
    for i in 0..n {
        let t = i as f64 / RATE;
        for hz in [1000.0, 400.0] {
            let v = (std::f64::consts::TAU * hz * t).sin() * 0.4;
            data.extend_from_slice(&((v * i16::MAX as f64) as i16).to_le_bytes());
        }
    }
    let mut f = Vec::new();
    f.extend_from_slice(b"RIFF");
    f.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    f.extend_from_slice(b"WAVEfmt ");
    f.extend_from_slice(&16u32.to_le_bytes());
    f.extend_from_slice(&1u16.to_le_bytes());
    f.extend_from_slice(&2u16.to_le_bytes());
    f.extend_from_slice(&rate.to_le_bytes());
    f.extend_from_slice(&(rate * 4).to_le_bytes());
    f.extend_from_slice(&4u16.to_le_bytes());
    f.extend_from_slice(&16u16.to_le_bytes());
    f.extend_from_slice(b"data");
    f.extend_from_slice(&(data.len() as u32).to_le_bytes());
    f.extend_from_slice(&data);
    std::fs::write(path, f).unwrap();
}

/// Power of `hz` in `s` (Goertzel).
fn power_at(s: &[f32], hz: f64) -> f64 {
    let k = 2.0 * (std::f64::consts::TAU * hz / RATE).cos();
    let (mut a, mut b) = (0.0f64, 0.0f64);
    for &x in s {
        let c = x as f64 + k * a - b;
        b = a;
        a = c;
    }
    a * a + b * b - k * a * b
}

#[test]
fn a_stereo_recording_reaches_the_speaker_whole_and_in_stereo() {
    let dir = std::env::temp_dir().join(format!("sdroxide-playback-audio-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("recordings")).unwrap();
    // SAFETY: the only test in this binary, and no engine exists yet.
    unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) }
    write_stereo_wav(&dir.join("recordings").join(NAME), 6.0);

    // The speaker: a ring drained as fast as it fills, as a sound card would,
    // keeping what came out.
    let (producer, mut consumer) = rtrb::RingBuffer::<f32>::new(96_000);
    let heard = Arc::new(std::sync::Mutex::new(Vec::<f32>::new()));
    // The longest wait between two arrivals at the speaker while listening.
    let worst_gap = Arc::new(std::sync::Mutex::new(Duration::ZERO));
    let listening = Arc::new(AtomicBool::new(false));
    let done = Arc::new(AtomicBool::new(false));
    let speaker = {
        let (heard, listening, done, worst_gap) =
            (heard.clone(), listening.clone(), done.clone(), worst_gap.clone());
        std::thread::spawn(move || {
            let mut last = Instant::now();
            while !done.load(Ordering::SeqCst) {
                let mut got = false;
                while let Ok(s) = consumer.pop() {
                    got = true;
                    if listening.load(Ordering::SeqCst) {
                        heard.lock().unwrap().push(s);
                    }
                }
                let now = Instant::now();
                if got {
                    if listening.load(Ordering::SeqCst) {
                        let mut w = worst_gap.lock().unwrap();
                        *w = (*w).max(now - last);
                    }
                    last = now;
                }
                std::thread::sleep(Duration::from_millis(2));
            }
        })
    };

    let reopen: sdroxide_radio::ReopenFn =
        Box::new(|_c: f64| Ok((Box::new(Live) as Box<dyn IqSource>, caps())));
    let cfg = EngineConfig {
        reopen: Some(reopen),
        audio: Some(AudioParams { producer, out_rate: RATE }),
        ..Default::default()
    };
    let mut h = start_engine(Box::new(Live), caps(), cfg);
    let thread = h.thread.take();
    std::thread::sleep(Duration::from_millis(200));
    h.cmd_tx.send(Command::Playback(PlaybackCommand::Open { name: NAME.into() })).unwrap();

    // Let it settle, then listen for two seconds of wall clock.
    std::thread::sleep(Duration::from_millis(800));
    listening.store(true, Ordering::SeqCst);
    let t0 = Instant::now();
    std::thread::sleep(Duration::from_secs(2));
    listening.store(false, Ordering::SeqCst);
    let secs = t0.elapsed().as_secs_f64();
    done.store(true, Ordering::SeqCst);
    let _ = speaker.join();

    let heard = std::mem::take(&mut *heard.lock().unwrap());
    let frames = heard.len() / 2;
    let left: Vec<f32> = heard.iter().step_by(2).copied().collect();
    let right: Vec<f32> = heard.iter().skip(1).step_by(2).copied().collect();

    // Steady: audio arrives every few milliseconds, as from a sound card. The
    // chopped build went a third of a second between lumps.
    let gap = *worst_gap.lock().unwrap();
    assert!(gap < Duration::from_millis(100), "the speaker waited {gap:?} between blocks");

    // Whole: the speaker got (close to) a second of audio per second.
    let got = frames as f64 / (RATE * secs);
    assert!(got > 0.9, "the speaker got {:.0} % of real time", got * 100.0);

    // In stereo: each ear carries its own tone.
    let (l1k, l400) = (power_at(&left, 1000.0), power_at(&left, 400.0));
    let (r1k, r400) = (power_at(&right, 1000.0), power_at(&right, 400.0));
    assert!(l1k > l400 * 100.0, "left: 1 kHz {l1k:.3e} against 400 Hz {l400:.3e}");
    assert!(r400 > r1k * 100.0, "right: 400 Hz {r400:.3e} against 1 kHz {r1k:.3e}");

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
    let _ = std::fs::remove_dir_all(&dir);
}
