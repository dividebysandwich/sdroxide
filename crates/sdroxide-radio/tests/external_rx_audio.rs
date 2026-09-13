//! Receive audio that arrives from a *different* radio than the I/Q.
//!
//! The arrangement is a transceiver with another radio attached as its
//! panadapter, set to be listened to rather than demodulated here: the picture
//! comes from the attached receiver's I/Q and the sound from the transceiver's
//! own sound card. What that has to be true of, and what these hold:
//!
//! - the speaker plays the *transceiver's* audio, not what the engine
//!   demodulated from the receiver;
//! - the panadapter still spans the receiver's whole I/Q, so the operator gets
//!   the wideband display the pairing exists for;
//! - the S-meter reads the audio being heard rather than the chain measuring
//!   the other antenna.

use std::time::Duration;

use sdroxide_radio::{AudioParams, Complex32, EngineConfig, IqSource, Result, rtrb, start_engine};
use sdroxide_types::{DeviceCaps, RadioEvent};

/// The attached receiver's I/Q rate — wideband, and deliberately not the
/// transceiver's audio rate so a resampler has to be involved.
const IQ_RATE: f64 = 240_000.0;
/// The transceiver's sound card.
const AUDIO_RATE: f64 = 48_000.0;
const CENTER: f64 = 14_100_000.0;

/// What the transceiver sends, and what the receiver's I/Q carries: two
/// distinguishable constants, so which one reached the speaker is not a
/// question of interpretation.
const RIG_AUDIO: f32 = 0.25;
const RX_IQ: f32 = 0.9;

/// A stand-in for the pairing: wideband I/Q from one radio, audio from another.
struct PairedSource {
    center: f64,
    /// False makes it behave like an ordinary wideband radio, for the contrast
    /// case — same samples, no external audio.
    external_audio: bool,
}

impl IqSource for PairedSource {
    fn sample_rate(&self) -> f64 {
        IQ_RATE
    }
    fn center_hz(&self) -> f64 {
        self.center
    }
    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center = hz;
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        // Paced like a real front end so the engine loop isn't a spin, and at
        // a block size that keeps the audio ring fed.
        std::thread::sleep(Duration::from_millis(10));
        let n = buf.len().min(2400);
        buf[..n].fill(Complex32::new(RX_IQ, 0.0));
        Ok(n)
    }
    fn rx_audio(&mut self, out: &mut Vec<f32>) -> Option<f64> {
        if !self.external_audio {
            return None;
        }
        // One 10 ms block of the transceiver's audio per I/Q block, which is
        // what the pairing's elastic queue hands over in the steady state.
        out.resize(out.len() + (AUDIO_RATE / 100.0) as usize, RIG_AUDIO);
        Some(AUDIO_RATE)
    }
    fn describe(&self) -> String {
        "mock transceiver + attached receiver".into()
    }
}

fn caps(external_audio: bool) -> DeviceCaps {
    DeviceCaps {
        driver: "rtlsdr".into(),
        label: "mock pairing".into(),
        rx_channels: 1,
        tx_channels: 1,
        tx_audio: true,
        freq_ranges_rx: vec![(10_000.0, 148_000_000.0)],
        freq_ranges_tx: vec![(1_800_000.0, 54_000_000.0)],
        rx_audio_external: external_audio,
        ..DeviceCaps::default()
    }
}

struct Ran {
    /// Peak absolute sample that reached the speaker ring.
    peak: f32,
    /// The span of the last panadapter frame published, in Hz.
    span_hz: f64,
    s_dbm: Option<f32>,
}

fn run(external_audio: bool) -> Ran {
    run_calibrated(external_audio, 0.0)
}

fn run_calibrated(external_audio: bool, cal_offset_db: f32) -> Ran {
    let (producer, mut consumer) = rtrb::RingBuffer::<f32>::new(48_000 * 4);
    let src = PairedSource { center: CENTER, external_audio };
    let mut h = start_engine(
        Box::new(src),
        caps(external_audio),
        EngineConfig {
            audio: Some(AudioParams { producer, out_rate: AUDIO_RATE }),
            cal_offset_db,
            ..Default::default()
        },
    );
    let thread = h.thread.take();

    let mut peak = 0.0f32;
    let mut s_dbm = None;
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while std::time::Instant::now() < deadline {
        while let Ok(ev) = h.event_rx.try_recv() {
            if let RadioEvent::Meters(m) = ev {
                s_dbm = Some(m.s_dbm);
            }
        }
        while let Ok(s) = consumer.pop() {
            peak = peak.max(s.abs());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let span_hz = {
        h.spectrum_out.update();
        h.spectrum_out.output_buffer().span_hz
    };

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
    Ran { peak, span_hz, s_dbm }
}

/// The whole point: what comes out of the speakers is the transceiver's audio,
/// while the panadapter still covers the attached receiver's full span.
#[test]
fn the_speaker_gets_the_transceivers_audio_and_the_display_the_receivers_iq() {
    let ran = run(true);
    // The AF knob applies to it like any other receive audio, and starts at
    // half — the same linear scaling a demod-audio rig's audio gets, this
    // being the same stage. What matters is that the *shape* is the rig's
    // constant and not something the demodulator made.
    let want = RIG_AUDIO * sdroxide_types::RadioState::default().rx[0].volume;
    assert!(
        (ran.peak - want).abs() < 0.02,
        "expected the transceiver's {RIG_AUDIO} at the AF setting ({want}) on the speaker, got a \
         peak of {}",
        ran.peak
    );
    assert!(
        (ran.span_hz - IQ_RATE).abs() < 1.0,
        "the panadapter must still span the receiver's {IQ_RATE} Hz, got {}",
        ran.span_hz
    );
}

/// The S-meter follows what is being heard. Measuring the chain instead would
/// report the strength of a signal on the *other* radio's antenna, which is
/// worse than reporting nothing.
#[test]
fn the_meter_reads_the_audio_that_is_heard() {
    let s_dbm = run(true).s_dbm.expect("a pairing must still publish an S-meter");
    // RIG_AUDIO as a mean square, in dBFS, with the default zero calibration.
    let want = 20.0 * RIG_AUDIO.log10();
    assert!(
        (s_dbm - want).abs() < 1.0,
        "expected about {want:.1} dBm from the rig's audio, got {s_dbm:.1}"
    );
}

/// Issue #427: the S-meter calibration belongs to the attached receiver's own
/// front end. The transceiver's audio is a different quantity on a different
/// scale, so a non-zero calibration must not move the meter while it is what
/// is being heard.
#[test]
fn the_receivers_calibration_does_not_land_on_the_transceivers_audio() {
    let s_dbm = run_calibrated(true, 30.0).s_dbm.expect("a pairing must still publish an S-meter");
    let want = 20.0 * RIG_AUDIO.log10();
    assert!(
        (s_dbm - want).abs() < 1.0,
        "a 30 dB receiver calibration moved the transceiver-audio meter: expected about \
         {want:.1}, got {s_dbm:.1}"
    );
}

/// The contrast: the same samples with the flag off are demodulated here as
/// usual, so the speaker carries something quite different. This is what keeps
/// the branch honest — without it the test above would pass on a build that
/// ignored `rx_audio` and happened to be quiet.
#[test]
fn without_the_flag_the_engine_demodulates_as_usual() {
    let ran = run(false);
    let paired = RIG_AUDIO * sdroxide_types::RadioState::default().rx[0].volume;
    assert!(
        (ran.peak - paired).abs() > 0.02,
        "an ordinary wideband radio must not be playing the transceiver's audio (peak {})",
        ran.peak
    );
    assert!((ran.span_hz - IQ_RATE).abs() < 1.0);
}
