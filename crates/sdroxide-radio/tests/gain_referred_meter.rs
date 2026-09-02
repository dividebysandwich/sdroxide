//! The S-meter on an I/Q front end that reports its own gain.
//!
//! A receive chain measures its passband in dBFS, which is a level at the
//! converter — so on its own it says as much about the front end's gain as it
//! does about the antenna. Turning an attenuator in used to move the meter by
//! the whole step, and on hardware with its own AGC (every SDRplay RSP, by
//! default) the loop moved it continuously: a signal appearing raised the
//! reading, the AGC took the gain back down, and the reading sank to where it
//! started while the signal was still there.
//!
//! `IqSource::rx_gain_db` is what fixes that, and what these hold is the
//! property that makes it worth having — the reading is a fact about the
//! antenna, so moving the gain must not move it, and a signal that really does
//! change must still come through.
//!
//! Every assertion here is a *difference* between two runs. What the chain's
//! own gain works out to — filters, decimation, the window on the measurement
//! — is not what this is about, and pinning an absolute dBm figure would make
//! the test fail for every honest change to the receive chain.

use std::time::Duration;

use sdroxide_radio::{AudioParams, Complex32, EngineConfig, IqSource, Result, rtrb, start_engine};
use sdroxide_types::{DeviceCaps, RadioEvent};

const RATE: f64 = 48_000.0;
const CENTER: f64 = 14_100_000.0;
/// A deliberately non-zero dBFS→dBm calibration, so a reading that has had it
/// applied is distinguishable from one that has not.
const CAL_OFFSET_DB: f32 = 7.0;
/// The level the stand-in signal would arrive at with the front end at unity,
/// in dBFS. Low enough that the largest gain below still leaves headroom.
const ANTENNA_DBFS: f32 = -70.0;

/// An I/Q front end delivering a constant carrier, with a gain it reports.
///
/// The gain is applied to the samples as well as reported, because that is
/// what makes the exercise mean anything: real hardware hands over a bigger
/// sample for more gain, and the two halves are supposed to cancel. A mock
/// that reported a gain without applying it would "pass" a meter that had the
/// subtraction backwards.
struct Rig {
    /// Level at the antenna, in dBFS-at-unity-gain.
    antenna_dbfs: f32,
    /// What the front end puts in front of the converter, and reports.
    /// `None` for one that cannot say.
    gain_db: Option<f32>,
}

impl Rig {
    fn amplitude(&self) -> f32 {
        let at_adc = self.antenna_dbfs + self.gain_db.unwrap_or(0.0);
        10f32.powf(at_adc / 20.0)
    }
}

impl IqSource for Rig {
    fn sample_rate(&self) -> f64 {
        RATE
    }
    fn center_hz(&self) -> f64 {
        CENTER
    }
    fn set_center_hz(&mut self, _hz: f64) -> Result<()> {
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        std::thread::sleep(Duration::from_millis(5));
        let n = buf.len().min(2048);
        // Constant amplitude: the mean square is exactly this, whatever block
        // length the engine asks for.
        buf[..n].fill(Complex32::new(self.amplitude(), 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "mock I/Q front end".into()
    }
    fn rx_gain_db(&self) -> Option<f32> {
        self.gain_db
    }
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "bench".into(),
        label: "bench front end".into(),
        rx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(0.0, 60_000_000.0)],
        ..DeviceCaps::default()
    }
}

/// The last S-meter reading an engine on this front end published.
fn s_dbm(antenna_dbfs: f32, gain_db: Option<f32>) -> f32 {
    // A speaker for the receive chain to end at: no audio sink, no
    // demodulator, and it is the demodulator that measures the passband.
    let (producer, mut consumer) = rtrb::RingBuffer::<f32>::new(1 << 16);
    let mut h = start_engine(
        Box::new(Rig { antenna_dbfs, gain_db }),
        caps(),
        EngineConfig {
            audio: Some(AudioParams { producer, out_rate: RATE }),
            cal_offset_db: CAL_OFFSET_DB,
            ..Default::default()
        },
    );
    let thread = h.thread.take();

    let mut last = None;
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while std::time::Instant::now() < deadline {
        while let Ok(ev) = h.event_rx.try_recv() {
            if let RadioEvent::Meters(m) = ev {
                last = Some(m.s_dbm);
            }
        }
        // Keep the ring draining, or the chain stalls on a full speaker and
        // the measurement stops with it.
        while consumer.pop().is_ok() {}
        std::thread::sleep(Duration::from_millis(20));
    }

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
    last.expect("an I/Q front end must publish an S-meter")
}

/// The one that matters: the same signal, seen through 30 dB more front-end
/// gain, must read the same. This is the SDRplay AGC case — the loop moves the
/// gain continuously, and every one of those movements used to land on the
/// meter as if the band had done it.
#[test]
fn the_reading_does_not_follow_the_gain() {
    let low = s_dbm(ANTENNA_DBFS, Some(20.0));
    let high = s_dbm(ANTENNA_DBFS, Some(50.0));
    let drift = (high - low).abs();
    assert!(
        drift < 1.0,
        "30 dB more front-end gain moved the S-meter by {drift:.1} dB \
         ({low:.1} → {high:.1}); it is metering the gain, not the antenna"
    );
}

/// And the meter must still be a meter: a signal that really is 10 dB stronger
/// reads 10 dB stronger. Subtracting the gain must not have subtracted the
/// signal with it.
#[test]
fn a_stronger_signal_still_reads_stronger() {
    let weak = s_dbm(ANTENNA_DBFS, Some(30.0));
    let strong = s_dbm(ANTENNA_DBFS + 10.0, Some(30.0));
    let rise = strong - weak;
    assert!(
        (rise - 10.0).abs() < 1.0,
        "a 10 dB stronger signal moved the meter by {rise:.1} dB ({weak:.1} → {strong:.1})"
    );
}

/// A front end that cannot report a gain is left exactly where it was: raw
/// dBFS plus the station's calibration offset, which is what a reported gain
/// of zero also means. Every backend but the SDRplay one is in this case, so a
/// regression here would be a silent change to every other radio's meter.
#[test]
fn a_front_end_that_cannot_say_reads_as_one_at_unity() {
    let silent = s_dbm(ANTENNA_DBFS, None);
    let unity = s_dbm(ANTENNA_DBFS, Some(0.0));
    assert!(
        (silent - unity).abs() < 1.0,
        "a front end that reports no gain read {silent:.1}, one reporting 0 dB read {unity:.1}"
    );
}
