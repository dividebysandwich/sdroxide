//! What the panadapter covers in CW.
//!
//! CW borrows the digital modes' audio tap because the panel needs a decoder
//! and a keyer, and for a while it borrowed their display along with it: a
//! high-resolution channel analyzer windowed to `dial-200 .. dial+3500 Hz`.
//! That is the right frame for FT8, where everything worth seeing is inside the
//! sub-band, and the wrong one for CW, where the operator is working the band.
//! It also could not be panned or zoomed out of, because the frame genuinely did
//! not contain the rest of the band to show.
//!
//! These hold CW to the wide device frame while leaving the digital modes their
//! channel view.

use std::time::Duration;

use sdroxide_radio::{AudioParams, Complex32, EngineConfig, IqSource, Result, rtrb, start_engine};
use sdroxide_types::{Command, DeviceCaps, Mode, RxId, SpectrumFrame};

const RATE: f64 = 2_400_000.0;
const CENTER: f64 = 14_030_000.0;
/// The window the channel analyzer puts on the dial: 3.7 kHz.
const CHANNEL_SPAN_HZ: f64 = 3700.0;

struct MockSource {
    center: f64,
}

impl IqSource for MockSource {
    fn sample_rate(&self) -> f64 {
        RATE
    }
    fn center_hz(&self) -> f64 {
        self.center
    }
    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center = hz;
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        std::thread::sleep(Duration::from_millis(5));
        let n = buf.len().min(2048);
        buf[..n].fill(Complex32::new(0.0, 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "mock rx source".into()
    }
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "mock".into(),
        label: "mock".into(),
        rx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(0.0, 1_000_000_000.0)],
        ..DeviceCaps::default()
    }
}

/// Run `cmds`, then return the last spectrum frame the engine published.
///
/// **With an audio chain, which is the whole point.** `EngineConfig::default()`
/// has no audio, and **no audio means no `main` chain at all** (`Engine::new`'s
/// `None => (None, None, …)`), so this harness used to run the one
/// configuration the channel analyzer cannot work in: the analyzer is fed from
/// the main DDC's output, and with no main it is created, never fed, and — since
/// frame selection prefers it — holds the display frozen for ever.
///
/// That is upstream issue #640, and every test in this file was silently
/// exercising it while claiming to test an ordinary station. The ring below
/// nothing reads is the same one `skim_window` uses: the chain needs somewhere
/// to play, not an audience.
fn last_frame(cmds: &[Command]) -> SpectrumFrame {
    last_frame_audio(cmds, true)
}

/// [`last_frame`], told whether the engine has an audio chain — the two
/// configurations behave differently and both are worth pinning.
fn last_frame_audio(cmds: &[Command], with_audio: bool) -> SpectrumFrame {
    let (producer, _consumer) = rtrb::RingBuffer::<f32>::new(48_000);
    let audio = with_audio.then_some(AudioParams { producer, out_rate: 48_000.0 });
    let mut h = start_engine(
        Box::new(MockSource { center: CENTER }),
        caps(),
        EngineConfig { tx_ham_only: false, audio, ..Default::default() },
    );
    let thread = h.thread.take();

    std::thread::sleep(Duration::from_millis(150));
    for c in cmds {
        h.cmd_tx.send(c.clone()).unwrap();
    }

    // Frames arrive through the triple buffer the client draws from, not as
    // events.
    let mut last = None;
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while std::time::Instant::now() < deadline {
        if h.spectrum_out.update() {
            let frame = h.spectrum_out.output_buffer();
            if !frame.bins.is_empty() {
                last = Some(frame.clone());
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
    last.expect("the engine should publish spectrum frames")
}

fn set_mode(mode: Mode) -> Command {
    Command::SetMode { rx: RxId::Main, mode }
}

#[test]
fn cw_keeps_the_whole_band_on_the_panadapter() {
    let frame = last_frame(&[set_mode(Mode::Cw)]);
    assert!(
        frame.span_hz > CHANNEL_SPAN_HZ * 10.0,
        "CW is showing {:.0} Hz — the panadapter is stuck on the channel window",
        frame.span_hz
    );
}

#[test]
fn cw_shows_what_ssb_shows() {
    // Nothing about CW's display should differ from an ordinary analog mode's.
    let cw = last_frame(&[set_mode(Mode::Cw)]);
    let usb = last_frame(&[set_mode(Mode::Usb)]);
    assert_eq!(cw.span_hz, usb.span_hz);
    assert_eq!(cw.center_hz, usb.center_hz);
}

#[test]
fn the_digital_modes_keep_their_channel_view() {
    // The narrow frame is right for FT8 and must survive.
    let frame = last_frame(&[set_mode(Mode::Ft8)]);
    assert!(
        (frame.span_hz - CHANNEL_SPAN_HZ).abs() < 200.0,
        "FT8 should show the sub-band, got {:.0} Hz",
        frame.span_hz
    );
}

/// **#640: with no audio chain there is nothing to feed the analyzer, so it
/// must not be chosen.**
///
/// A station with an attached receiver painting the picture and its audio from
/// the transceiver has no `main` at all. The analyzer used to be created anyway
/// for a digital mode, never fed, and preferred by frame selection — so FT8
/// showed a frozen 3.7 kHz window instead of the live wide view the panadapter
/// was producing. Reported from a TS-440 + RTL-SDR IF setup with a patch, and
/// the same shape here.
#[test]
fn a_digital_mode_without_an_audio_chain_shows_the_live_wide_view() {
    let frame = last_frame_audio(&[set_mode(Mode::Ft8)], false);
    assert!(
        frame.span_hz > CHANNEL_SPAN_HZ * 10.0,
        "FT8 with no audio chain is showing {:.0} Hz — the never-fed channel \
         analyzer was chosen again",
        frame.span_hz
    );
}

#[test]
fn leaving_a_digital_mode_for_cw_gives_the_band_back() {
    // The analyzer is already running when CW arrives from FT8; if nothing takes
    // it down, CW inherits the 3.7 kHz window and the band never comes back.
    let frame = last_frame(&[set_mode(Mode::Ft8), set_mode(Mode::Cw)]);
    assert!(
        frame.span_hz > CHANNEL_SPAN_HZ * 10.0,
        "CW after FT8 is showing {:.0} Hz — the channel analyzer was left up",
        frame.span_hz
    );
}
