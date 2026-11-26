//! A radio's own S-meter calibration (#642), set from Settings → Radio and
//! applied while the operator watches the meter.

use std::time::{Duration, Instant};

use sdroxide_radio::{AudioParams, Complex32, EngineConfig, IqSource, Result, rtrb, start_engine};
use sdroxide_types::{Command, DeviceCaps, RadioConfig, RadioEvent};

const RATE: f64 = 48_000.0;

/// A constant carrier at -70 dBFS.
struct Rig;

impl IqSource for Rig {
    fn sample_rate(&self) -> f64 {
        RATE
    }
    fn center_hz(&self) -> f64 {
        14_100_000.0
    }
    fn set_center_hz(&mut self, _hz: f64) -> Result<()> {
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        std::thread::sleep(Duration::from_millis(5));
        let n = buf.len().min(2048);
        buf[..n].fill(Complex32::new(10f32.powf(-70.0 / 20.0), 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "carrier".into()
    }
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "bench".into(),
        label: "bench".into(),
        rx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(0.0, 60_000_000.0)],
        ..DeviceCaps::default()
    }
}

#[test]
fn the_radios_calibration_moves_the_meter_at_once() {
    // The engine saves what it is sent: keep it out of the real config.
    let root = std::env::temp_dir().join(format!("sdroxide-s-meter-cal-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &root) };

    let (producer, mut consumer) = rtrb::RingBuffer::<f32>::new(1 << 16);
    let mut h = start_engine(
        Box::new(Rig),
        caps(),
        EngineConfig {
            audio: Some(AudioParams { producer, out_rate: RATE }),
            ..Default::default()
        },
    );
    let thread = h.thread.take();
    let mut settle = |h: &sdroxide_radio::EngineHandles| {
        let mut last = None;
        let deadline = Instant::now() + Duration::from_millis(1500);
        while Instant::now() < deadline {
            while let Ok(ev) = h.event_rx.try_recv() {
                if let RadioEvent::Meters(m) = ev {
                    last = Some(m.s_dbm);
                }
            }
            while consumer.pop().is_ok() {}
            std::thread::sleep(Duration::from_millis(20));
        }
        last.expect("an S-meter reading")
    };

    let before = settle(&h);
    let cfg = RadioConfig { s_meter_cal_db: -40.0, ..RadioConfig::default() };
    h.cmd_tx.send(Command::SetRadioConfig { cfg: Box::new(cfg), reopen: false }).unwrap();
    let after = settle(&h);
    assert!(
        ((after - before) + 40.0).abs() < 1.0,
        "a -40 dB calibration moved the meter from {before} to {after}"
    );

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
    let _ = std::fs::remove_dir_all(&root);
}
