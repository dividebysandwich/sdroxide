//! The ISM decoder the operator switched on comes back with the radio.
//!
//! Playing a recording back swaps the front end for an audio-mode one, which
//! has no wideband IQ and so forces the ISM decoder off. Stopping swaps the
//! live radio back in — and the decoder has to come back with it, as the
//! skimmers and the other lanes do, rather than staying off until the operator
//! notices and switches it on again.

use std::time::{Duration, Instant};

use sdroxide_radio::{Complex32, EngineConfig, IqSource, Result, start_engine};
use sdroxide_types::{
    Command, DeviceCaps, IsmFamily, IsmSettings, PlaybackCommand, RadioEvent, RadioState,
};

const RATE: f64 = 48_000.0;
const STAMPED: &str = "sdroxide_2026-10-06_10-00-05Z_14.230000MHz_USB.wav";

/// Point the configuration — and with it the recordings folder — at a scratch
/// directory, before any engine exists. There is one test in this binary.
fn isolate_config() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("sdroxide-ism-swap-it-{}", std::process::id()));
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

/// A 16-bit mono WAV of silence.
fn write_wav(path: &std::path::Path, rate: u32, secs: f64) {
    let data = vec![0u8; (rate as f64 * secs) as usize * 2];
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
}

impl Watch {
    fn until(
        &mut self,
        rx: &sdroxide_radio::crossbeam_channel::Receiver<RadioEvent>,
        what: &str,
        pred: impl Fn(&Watch) -> bool,
    ) {
        let deadline = Instant::now() + Duration::from_secs(8);
        while Instant::now() < deadline {
            while let Ok(ev) = rx.try_recv() {
                match ev {
                    RadioEvent::State(s) => self.state = Some(s),
                    RadioEvent::Capabilities(c) => self.driver = c.driver,
                    _ => {}
                }
            }
            if pred(self) {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!(
            "{what} — never happened (driver {:?}, ism {:?})",
            self.driver,
            self.state.as_ref().map(|s| s.ism)
        );
    }
}

#[test]
fn the_ism_decoder_comes_back_after_a_playback() {
    let cfg_dir = isolate_config();
    let rec = cfg_dir.join("recordings");
    std::fs::create_dir_all(&rec).unwrap();
    write_wav(&rec.join(STAMPED), 8000, 3.0);

    let reopen: sdroxide_radio::ReopenFn =
        Box::new(|_center: f64| Ok((Box::new(Live) as Box<dyn IqSource>, caps())));
    let cfg = EngineConfig { reopen: Some(reopen), ..Default::default() };
    let mut h = start_engine(Box::new(Live), caps(), cfg);
    let thread = h.thread.take();
    let mut w = Watch { state: None, driver: String::new() };

    // The operator switches the ISM decoder on.
    let mut on = IsmSettings { enabled: true, ..IsmSettings::default() };
    on.set_family(IsmFamily::Weather, true);
    h.cmd_tx.send(Command::SetIsmConfig(on)).unwrap();
    w.until(&h.event_rx, "the ISM decoder running", |w| {
        w.driver == "live" && w.state.as_ref().is_some_and(|s| s.ism == on)
    });

    // A recording: audio only, so the decoder has nothing to work from.
    h.cmd_tx.send(Command::Playback(PlaybackCommand::Open { name: STAMPED.into() })).unwrap();
    w.until(&h.event_rx, "the ISM decoder off for the playback", |w| {
        w.driver == "playback" && w.state.as_ref().is_some_and(|s| !s.ism.any_enabled())
    });

    // Back to the live radio: the decoder is on again, as it was left.
    h.cmd_tx.send(Command::Playback(PlaybackCommand::Stop)).unwrap();
    w.until(&h.event_rx, "the ISM decoder back on with the live radio", |w| {
        w.driver == "live"
            && w.state.as_ref().is_some_and(|s| s.playback.is_none() && s.ism == on)
    });

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
    let _ = std::fs::remove_dir_all(&cfg_dir);
}
