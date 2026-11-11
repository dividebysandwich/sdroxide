//! Playing a recording back into the decoders, end to end through the engine:
//! the radio stands down, the dial and mode go to what the file's name records,
//! the transport answers, and STOP puts the radio back exactly where it was.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use sdroxide_radio::{Complex32, EngineConfig, IqSource, Result, start_engine};
use sdroxide_types::{
    Command, DeviceCaps, Mode, PlaybackCommand, RadioEvent, RadioState, RecordingListing, RxId, Vfo,
};

const RATE: f64 = 48_000.0;
const LIVE_HZ: f64 = 7_100_000.0;
const STAMPED: &str = "sdroxide_2026-10-06_10-00-05Z_14.230000MHz_SSTV.wav";

/// Point the configuration — and with it the recordings folder — at a scratch
/// directory, before any engine exists. There is one test in this binary.
fn isolate_config() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("sdroxide-playback-it-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch config directory");
    // SAFETY: the only test in this binary, and no engine exists yet.
    unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) }
    dir
}

/// The live radio: silence, paced.
struct Live;

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

/// A 16-bit mono WAV of a 1 kHz tone.
fn write_wav(path: &std::path::Path, rate: u32, secs: f64) {
    let n = (rate as f64 * secs) as usize;
    let mut data = Vec::with_capacity(n * 2);
    for i in 0..n {
        let v = (std::f64::consts::TAU * 1000.0 * i as f64 / rate as f64).sin() * 0.4;
        data.extend_from_slice(&((v * i16::MAX as f64) as i16).to_le_bytes());
    }
    let mut f = Vec::new();
    f.extend_from_slice(b"RIFF");
    f.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    f.extend_from_slice(b"WAVEfmt ");
    f.extend_from_slice(&16u32.to_le_bytes());
    f.extend_from_slice(&1u16.to_le_bytes());
    f.extend_from_slice(&1u16.to_le_bytes());
    f.extend_from_slice(&rate.to_le_bytes());
    f.extend_from_slice(&(rate * 2).to_le_bytes());
    f.extend_from_slice(&2u16.to_le_bytes());
    f.extend_from_slice(&16u16.to_le_bytes());
    f.extend_from_slice(b"data");
    f.extend_from_slice(&(data.len() as u32).to_le_bytes());
    f.extend_from_slice(&data);
    std::fs::write(path, f).unwrap();
}

struct Watch {
    state: Option<RadioState>,
    driver: String,
    listing: Option<RecordingListing>,
    notices: Vec<String>,
}

impl Watch {
    fn drain(&mut self, rx: &sdroxide_radio::crossbeam_channel::Receiver<RadioEvent>) {
        while let Ok(ev) = rx.try_recv() {
            match ev {
                RadioEvent::State(s) => self.state = Some(s),
                RadioEvent::Capabilities(c) => self.driver = c.driver,
                RadioEvent::Recordings(l) => self.listing = Some(l),
                RadioEvent::Notice(Some(n)) => self.notices.push(n),
                _ => {}
            }
        }
    }

    /// Wait until `pred` holds, or fail saying `what`.
    fn until(
        &mut self,
        rx: &sdroxide_radio::crossbeam_channel::Receiver<RadioEvent>,
        what: &str,
        pred: impl Fn(&Watch) -> bool,
    ) {
        let deadline = Instant::now() + Duration::from_secs(8);
        while Instant::now() < deadline {
            self.drain(rx);
            if pred(self) {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!(
            "{what} — never happened (driver {:?}, playback {:?}, notices {:?})",
            self.driver,
            self.state.as_ref().map(|s| s.playback.clone()),
            self.notices
        );
    }
}

#[test]
fn a_recording_plays_back_and_the_radio_comes_back() {
    let cfg_dir = isolate_config();
    let rec = cfg_dir.join("recordings");
    std::fs::create_dir_all(&rec).unwrap();
    write_wav(&rec.join(STAMPED), 8000, 3.0);
    // An I/Q capture is listed, marked; something that is not audio is not.
    std::fs::write(rec.join("SDRoxide_20261006_100005Z_14200000Hz_192000sps_IQ.wav"), b"x")
        .unwrap();
    std::fs::write(rec.join("notes.txt"), b"x").unwrap();

    let opened = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&opened);
    let reopen: sdroxide_radio::ReopenFn = Box::new(move |_center: f64| {
        seen.fetch_add(1, Ordering::SeqCst);
        Ok((Box::new(Live) as Box<dyn IqSource>, caps()))
    });
    let cfg = EngineConfig { reopen: Some(reopen), ..Default::default() };
    let mut h = start_engine(Box::new(Live), caps(), cfg);
    let thread = h.thread.take();
    let mut w = Watch { state: None, driver: String::new(), listing: None, notices: Vec::new() };

    // The operator is listening to 40 m USB.
    h.cmd_tx.send(Command::SetMode { rx: RxId::Main, mode: Mode::Usb }).unwrap();
    h.cmd_tx.send(Command::SetVfo { vfo: Vfo::A, hz: LIVE_HZ }).unwrap();
    w.until(&h.event_rx, "the radio on 7.100 USB", |w| {
        w.state
            .as_ref()
            .is_some_and(|s| s.rx[0].mode == Mode::Usb && (s.vfo_a_hz - LIVE_HZ).abs() < 1.0)
    });

    // The list: the recording and the capture, nothing else.
    h.cmd_tx.send(Command::ListRecordings).unwrap();
    w.until(&h.event_rx, "the recordings list", |w| w.listing.is_some());
    let listing = w.listing.take().unwrap();
    let mut names: Vec<(String, bool)> =
        listing.files.iter().map(|f| (f.name.clone(), f.iq)).collect();
    names.sort();
    assert_eq!(
        names,
        vec![
            ("SDRoxide_20261006_100005Z_14200000Hz_192000sps_IQ.wav".to_string(), true),
            (STAMPED.to_string(), false),
        ]
    );

    // A path is refused before anything touches the disk, and so is a capture.
    h.cmd_tx
        .send(Command::Playback(PlaybackCommand::Open { name: "../config.toml".into() }))
        .unwrap();
    h.cmd_tx
        .send(Command::Playback(PlaybackCommand::Open {
            name: "SDRoxide_20261006_100005Z_14200000Hz_192000sps_IQ.wav".into(),
        }))
        .unwrap();
    w.until(&h.event_rx, "two refusals", |w| w.notices.len() >= 2);
    assert!(w.notices.iter().any(|n| n.contains("I/Q capture")), "{:?}", w.notices);
    assert_eq!(w.driver, "live", "a refused file stood the radio down");

    // Play: the radio stands down, and the dial and mode are the file's.
    h.cmd_tx.send(Command::Playback(PlaybackCommand::Open { name: STAMPED.into() })).unwrap();
    w.until(&h.event_rx, "the playback under way on 14.230 SSTV", |w| {
        w.driver == "playback"
            && w.state.as_ref().is_some_and(|s| {
                s.playback.as_ref().is_some_and(|p| p.position_s > 0.5)
                    && s.rx[0].mode == Mode::Sstv
                    && (s.vfo_a_hz - 14_230_000.0).abs() < 1.0
            })
    });
    let p = w.state.as_ref().unwrap().playback.clone().unwrap();
    assert_eq!((p.rate_hz, p.channels), (8000, 1));
    assert!((p.duration_s.unwrap() - 3.0).abs() < 0.01);

    // The file playing back cannot be deleted from under the player.
    h.cmd_tx.send(Command::Playback(PlaybackCommand::Delete { name: STAMPED.into() })).unwrap();
    w.until(&h.event_rx, "the delete of the playing file refused", |w| {
        w.notices.iter().any(|n| n.contains("playing back"))
    });
    assert!(rec.join(STAMPED).exists(), "the file playing back was deleted");

    // Paused, it holds still.
    h.cmd_tx.send(Command::Playback(PlaybackCommand::Pause(true))).unwrap();
    w.until(&h.event_rx, "paused", |w| {
        w.state.as_ref().and_then(|s| s.playback.as_ref()).is_some_and(|p| p.paused)
    });

    // Stop: the live radio, on the dial and in the mode it was left in.
    let before = opened.load(Ordering::SeqCst);
    h.cmd_tx.send(Command::Playback(PlaybackCommand::Stop)).unwrap();
    w.until(&h.event_rx, "the radio back on 7.100 USB", |w| {
        w.driver == "live"
            && w.state.as_ref().is_some_and(|s| {
                s.playback.is_none()
                    && s.rx[0].mode == Mode::Usb
                    && (s.vfo_a_hz - LIVE_HZ).abs() < 1.0
            })
    });
    assert_eq!(opened.load(Ordering::SeqCst), before + 1, "the radio was reopened once");

    // CLEAR: a path is refused, and the files the listing offered go — the
    // capture as well as the recording — with a fresh list after each.
    let outside = cfg_dir.join("config.toml");
    std::fs::write(&outside, b"keep me").unwrap();
    h.cmd_tx
        .send(Command::Playback(PlaybackCommand::Delete { name: "../config.toml".into() }))
        .unwrap();
    w.until(&h.event_rx, "the path refused", |w| {
        w.notices.iter().any(|n| n.contains("not a file in the recordings folder"))
    });
    assert!(outside.exists(), "a delete reached outside the recordings folder");
    for name in [STAMPED, "SDRoxide_20261006_100005Z_14200000Hz_192000sps_IQ.wav"] {
        w.listing = None;
        h.cmd_tx.send(Command::Playback(PlaybackCommand::Delete { name: name.into() })).unwrap();
        w.until(&h.event_rx, "a fresh list after the delete", |w| {
            w.listing.as_ref().is_some_and(|l| l.files.iter().all(|f| f.name != name))
        });
        assert!(!rec.join(name).exists(), "{name} is still on disk");
    }
    assert!(rec.join("notes.txt").exists(), "a file the list never offered was deleted");

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
    let _ = std::fs::remove_dir_all(&cfg_dir);
}
