//! A recording of weather fax, replayed into the WEFAX decoder (fork discussion
//! #10, and the field report that a continuous carrier on a real recording kept
//! it from ever starting).
//!
//! The file is demodulated audio — twenty seconds of a real off-air chart under
//! a carrier, cut from the recording `crates/sdroxide-dsp/tests/wefax_real_recording.rs`
//! describes — so the only way to check what the decoder makes of it is to play
//! it back through the engine and let the decoder listen, exactly as it would to
//! the radio. Two things are asserted, because they are the two things the
//! operator asked of a replay:
//!
//! * the picture comes out **while the file plays**, at the speed it was sent,
//!   with nothing pressed — no START, no tone;
//! * **the live radio is not listened to** while it does. Playback stands the
//!   radio down, and its source is not read again until the operator stops.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use sdroxide_radio::{Complex32, EngineConfig, IqSource, Result, start_engine};
use sdroxide_types::{Command, DeviceCaps, Mode, PlaybackCommand, RadioEvent, RxId, Vfo};

const RATE: f64 = 48_000.0;
const LIVE_HZ: f64 = 7_100_000.0;
const NAME: &str = "sdroxide_2026-10-20_12-00-00Z_9.300000MHz_WEFAX.wav";

/// The live radio: silence, paced — and a count of how often it is read.
struct Live(Arc<AtomicUsize>);

impl IqSource for Live {
    fn sample_rate(&self) -> f64 {
        RATE
    }
    fn center_hz(&self) -> f64 {
        LIVE_HZ
    }
    fn set_center_hz(&mut self, _hz: f64) -> Result<()> {
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        self.0.fetch_add(1, Ordering::SeqCst);
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
        tx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(0.0, 60_000_000.0)],
        freq_ranges_tx: vec![(0.0, 60_000_000.0)],
        ..DeviceCaps::default()
    }
}

/// The 6 kHz mono fixture, taken to the 48 kHz the recorder writes by linear
/// interpolation, as a 16-bit WAV.
fn recording() -> Vec<u8> {
    let path =
        concat!(env!("CARGO_MANIFEST_DIR"), "/../sdroxide-dsp/tests/testdata/wefax/start.wav");
    let raw = std::fs::read(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    assert_eq!(&raw[..4], b"RIFF", "{path}");
    let rate = u32::from_le_bytes(raw[24..28].try_into().unwrap());
    assert_eq!(rate, 6_000, "the fixture is a 6 kHz recording");
    let x: Vec<f64> =
        raw[44..].chunks_exact(2).map(|b| f64::from(i16::from_le_bytes([b[0], b[1]]))).collect();
    let up = (RATE as u32 / rate) as usize;
    let mut data = Vec::with_capacity(x.len() * up * 2);
    for w in x.windows(2) {
        for k in 0..up {
            let v = w[0] + (w[1] - w[0]) * k as f64 / up as f64;
            data.extend_from_slice(&(v as i16).to_le_bytes());
        }
    }
    let mut f = Vec::new();
    f.extend_from_slice(b"RIFF");
    f.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    f.extend_from_slice(b"WAVEfmt ");
    f.extend_from_slice(&16u32.to_le_bytes());
    f.extend_from_slice(&1u16.to_le_bytes());
    f.extend_from_slice(&1u16.to_le_bytes());
    f.extend_from_slice(&(RATE as u32).to_le_bytes());
    f.extend_from_slice(&(RATE as u32 * 2).to_le_bytes());
    f.extend_from_slice(&2u16.to_le_bytes());
    f.extend_from_slice(&16u16.to_le_bytes());
    f.extend_from_slice(b"data");
    f.extend_from_slice(&(data.len() as u32).to_le_bytes());
    f.extend_from_slice(&data);
    f
}

#[test]
fn a_fax_recording_is_decoded_as_it_plays_and_the_live_radio_is_not_heard() {
    let dir = std::env::temp_dir().join(format!("sdroxide-playback-wefax-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("recordings")).unwrap();
    // SAFETY: the only test in this binary, and no engine exists yet.
    unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) }
    std::fs::write(dir.join("recordings").join(NAME), recording()).unwrap();

    let reads = Arc::new(AtomicUsize::new(0));
    let again = Arc::clone(&reads);
    let reopen: sdroxide_radio::ReopenFn = Box::new(move |_center: f64| {
        Ok((Box::new(Live(Arc::clone(&again))) as Box<dyn IqSource>, caps()))
    });
    let cfg = EngineConfig { reopen: Some(reopen), ..Default::default() };
    let mut h = start_engine(Box::new(Live(Arc::clone(&reads))), caps(), cfg);
    let thread = h.thread.take();

    // The operator is listening to 40 m.
    h.cmd_tx.send(Command::SetModeListen { rx: RxId::Main, mode: Mode::Usb }).unwrap();
    h.cmd_tx.send(Command::SetVfo { vfo: Vfo::A, hz: LIVE_HZ }).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    assert!(reads.load(Ordering::SeqCst) > 0, "the live radio was never read");

    // Play the recording. Its name says WEFAX, so the decoder is the right one.
    h.cmd_tx.send(Command::Playback(PlaybackCommand::Open { name: NAME.into() })).unwrap();

    let mut lines = 0usize;
    let mut first_row_after_s = None;
    let mut reads_when_playing = None;
    let began = Instant::now();
    let deadline = began + Duration::from_secs(45);
    while Instant::now() < deadline && lines < 12 {
        while let Ok(ev) = h.event_rx.try_recv() {
            match ev {
                RadioEvent::State(s) if s.playback.is_some() && reads_when_playing.is_none() => {
                    reads_when_playing = Some(reads.load(Ordering::SeqCst));
                }
                RadioEvent::WefaxLine { .. } => {
                    first_row_after_s.get_or_insert(began.elapsed().as_secs_f64());
                    lines += 1;
                }
                _ => {}
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let heard = reads.load(Ordering::SeqCst);

    h.cmd_tx.send(Command::Playback(PlaybackCommand::Stop)).unwrap();
    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
    let _ = std::fs::remove_dir_all(&dir);

    assert!(lines >= 12, "the recording played and only {lines} lines came out of the decoder");
    // The phasing signal is 1.5 s into the file and takes a few seconds to
    // recognise, and a replay runs at the speed the chart was sent: a picture
    // that arrived at once would mean the file was not being played.
    let first = first_row_after_s.expect("no line");
    assert!(first > 4.0, "the first row came out {first:.1} s in — faster than the file plays");

    // The radio stood down: from the moment playback was reported, its source
    // was read a handful of times at most (a read in flight), not for the half
    // minute it played.
    let at_start = reads_when_playing.expect("playback was never reported");
    assert!(
        heard.saturating_sub(at_start) <= 8,
        "the live radio was read {} times while the recording played",
        heard - at_start
    );
}
