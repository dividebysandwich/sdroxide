//! The ISM decoder's window across a band change (issue #142).
//!
//! The window is not just a place, it is a width: the 868 MHz plan wants nearly
//! two megahertz where 433.92 wants a quarter of one. Both come out of the same
//! front end, so changing band changes the decimation the engine has to take —
//! and a chain built for the previous band cannot be retuned into the new one,
//! it has to be rebuilt.
//!
//! The reported symptom was the whole decoder going silent after a band change,
//! with the panel saying nothing was inside the window, and both lanes coming
//! back the moment the operator switched decoding off and on again. That is the
//! signature of a stale window rather than a wrong one, which is what this test
//! pins: the engine is walked from one band to the other and the lane has to be
//! running on the far side, without the decoder being restarted.
//!
//! rtl_433 is the lane that can be pointed at more than one band: the native
//! decoders are fixed on the 868 MHz channel plan, so their window slides with
//! the dial but is never asked to change width.
//!
//! The second test here is issue #310, which is the opposite failure: a front
//! end with room for *both* lanes at once being handed a window sized to one of
//! them.

#![cfg(feature = "rtl433")]

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sdroxide_radio::{AudioParams, Complex32, EngineConfig, IqSource, Result, rtrb, start_engine};
use sdroxide_types::{Command, DeviceCaps, IsmSettings, IsmStatus, RadioEvent, Vfo};

/// The bits of `Rtl433Settings::bands`, from `RTL433_BAND_LABELS`.
const BAND_433: u32 = 1 << 0;
const BAND_868: u32 = 1 << 1;

/// `Rtl433Settings::bandwidth_hz` meaning "whatever the band asks for".
const AUTO: u32 = sdroxide_types::RTL433_BANDWIDTH_AUTO;

const DIAL_433: f64 = 433_920_000.0;
const DIAL_868: f64 = 868_650_000.0;

/// Wide enough for the 868 MHz band to fit undecimated (1.024 MHz inside three
/// quarters of 1.4), and narrow enough that 433.92 MHz is decimated by four —
/// so the two bands genuinely ask for different chains, which is the whole
/// point of the test.
const RATE: f64 = 1_400_000.0;

/// The PlutoSDR of issue #310, and where its LO sat: 3.840 Msps, which has room
/// for the whole 868 MHz channel plan *and* rtl_433's 868 MHz band together.
const PLUTO_RATE: f64 = 3_840_000.0;
const PLUTO_LO: f64 = 868_957_858.0;

/// A front end that tunes anywhere and hears nothing but its own noise floor.
/// This test is about which window the engine builds, not about what decodes
/// inside it.
struct Quiet {
    center: Arc<Mutex<f64>>,
    rate: f64,
    /// How far above the dial this front end parks its LO — zero for most,
    /// a quarter of the span for a zero-IF radio like the PlutoSDR.
    lo_offset: f64,
    seed: u64,
}

impl IqSource for Quiet {
    fn sample_rate(&self) -> f64 {
        self.rate
    }
    fn center_hz(&self) -> f64 {
        *self.center.lock().unwrap()
    }
    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        *self.center.lock().unwrap() = hz;
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        // Not real time: the lane's status is emitted on a wall clock, so a
        // trickle of blocks is enough to keep it talking without spending a core
        // on 1.4 Msps of silence.
        std::thread::sleep(Duration::from_millis(5));
        let n = buf.len().min(4096);
        for z in buf[..n].iter_mut() {
            let mut r = || {
                self.seed ^= self.seed << 13;
                self.seed ^= self.seed >> 7;
                self.seed ^= self.seed << 17;
                ((self.seed >> 11) as f64 / (1u64 << 53) as f64) as f32 - 0.5
            };
            *z = Complex32::new(r() * 0.01, r() * 0.01);
        }
        Ok(n)
    }
    fn lo_offset_hz(&self) -> f64 {
        self.lo_offset
    }
    fn describe(&self) -> String {
        "quiet stand-in".into()
    }
}

fn caps_at(rate: f64) -> DeviceCaps {
    DeviceCaps {
        driver: "mock".into(),
        label: "mock".into(),
        rx_channels: 1,
        sample_rates: vec![rate],
        freq_ranges_rx: vec![(1_000_000.0, 2_000_000_000.0)],
        ..DeviceCaps::default()
    }
}

fn ism_on(bands: u32, bandwidth_hz: u32) -> IsmSettings {
    IsmSettings {
        enabled: true,
        // The native decoders switched off: they listen on 868 MHz whatever the
        // dial says, and this is about the lane that follows it.
        families: 0,
        rtl433: sdroxide_types::Rtl433Settings { bands, bandwidth_hz },
        ..IsmSettings::default()
    }
}

/// Wait for an ISM status that satisfies `f`, or say what the last one was.
fn ism_status(
    h: &sdroxide_radio::EngineHandles,
    what: &str,
    mut f: impl FnMut(&IsmStatus) -> bool,
) -> IsmStatus {
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut last: Option<IsmStatus> = None;
    while Instant::now() < deadline {
        while let Ok(ev) = h.event_rx.try_recv() {
            if let RadioEvent::IsmStatus(s) = ev {
                if f(&s) {
                    return s;
                }
                last = Some(s);
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("the ISM decoder never reported {what}; last status: {last:#?}");
}

fn lane_running_on(band: &'static str) -> impl Fn(&IsmStatus) -> bool {
    move |s: &IsmStatus| s.rtl433.as_ref().is_some_and(|r| r.running && r.band == band)
}

#[test]
fn a_band_change_re_sizes_the_window_without_a_restart() {
    let root = std::env::temp_dir().join(format!("sdroxide-ism-window-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &root) };

    // The lanes below the speaker — the skimmers, the ISM window, the IQ
    // recorder — are fed from the same pass as the main receiver, so an engine
    // with nowhere to play audio never reaches them. Hence a ring nothing reads.
    let (producer, _consumer) = rtrb::RingBuffer::<f32>::new(48_000);
    let center = Arc::new(Mutex::new(DIAL_433));
    let mut h = start_engine(
        Box::new(Quiet { center, rate: RATE, lo_offset: 0.0, seed: 0x1234_5678_9ABC_DEF0 }),
        caps_at(RATE),
        EngineConfig {
            remember_session: false,
            audio: Some(AudioParams { producer, out_rate: 48_000.0 }),
            ..Default::default()
        },
    );
    let thread = h.thread.take();
    let send = |c: Command| h.cmd_tx.send(c).unwrap();

    // ---- Listening on 433.92 MHz, which is decimated by four ----
    send(Command::SetVfo { vfo: Vfo::A, hz: DIAL_433 });
    send(Command::SetIsmConfig(ism_on(BAND_433, AUTO)));
    let narrow = ism_status(&h, "the 433.92 MHz lane starting", lane_running_on("433.92 MHz"));
    assert!(
        narrow.window_rate_hz < 500_000.0,
        "433.92 MHz should be reached through a decimated window, not {:.0} Hz",
        narrow.window_rate_hz
    );

    // ---- The operator picks 868 MHz and tunes there ----
    // Nothing else happens: the decoder is never stopped, which is precisely the
    // workaround this test exists to make unnecessary.
    send(Command::SetIsmConfig(ism_on(BAND_868, AUTO)));
    send(Command::SetVfo { vfo: Vfo::A, hz: DIAL_868 });
    let wide = ism_status(&h, "the 868 MHz lane starting", lane_running_on("868 MHz EU"));
    assert!(
        wide.window_rate_hz > 1_024_000.0,
        "868 MHz EU needs 1.024 MHz of window and got {:.0} Hz",
        wide.window_rate_hz
    );
    assert!(
        (wide.window_center_hz - DIAL_868).abs() < 100_000.0,
        "the window sat on {:.4} MHz with the dial on {:.4}",
        wide.window_center_hz / 1e6,
        DIAL_868 / 1e6
    );
    assert_eq!(wide.unavailable, None, "the decoder called itself unavailable while running");

    // ---- The operator narrows the window by hand (issue #141) ----
    // Same band, same dial: only the chosen bandwidth moves, and it has to move
    // the window with it exactly as a band change does.
    send(Command::SetIsmConfig(ism_on(BAND_868, 250_000)));
    let hand = ism_status(&h, "the 868 MHz lane on a 250 kHz window", |s| {
        s.rtl433.as_ref().is_some_and(|r| r.running && r.band == "868 MHz EU" && r.rate_hz > 0.0)
            && s.window_rate_hz < 500_000.0
    });
    let lane_rate = hand.rtl433.as_ref().unwrap().rate_hz;
    assert!(
        (250_000.0..=400_000.0).contains(&lane_rate),
        "a 250 kHz request settled on {lane_rate:.0} Hz"
    );

    // ---- And back to AUTO, so the stretch is covered as well as the shrink ----
    send(Command::SetIsmConfig(ism_on(BAND_868, AUTO)));
    let back = ism_status(&h, "the 868 MHz lane back on its own width", |s| {
        s.rtl433.as_ref().is_some_and(|r| r.running && r.band == "868 MHz EU")
            && s.window_rate_hz > 1_024_000.0
    });
    assert!(
        back.rtl433.as_ref().unwrap().rate_hz >= 1_024_000.0,
        "AUTO should give 868 MHz its own megahertz back"
    );

    // ---- And back to the other band, which is the shrink the ticket was ----
    send(Command::SetIsmConfig(ism_on(BAND_433, AUTO)));
    send(Command::SetVfo { vfo: Vfo::A, hz: DIAL_433 });
    ism_status(&h, "the 433.92 MHz lane starting again", lane_running_on("433.92 MHz"));

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
    unsafe { std::env::remove_var("SDROXIDE_CONFIG_DIR") };
}

/// Issue #310: a front end wide enough for both lanes is given both.
///
/// A 3.840 Msps PlutoSDR on the 868 MHz band has room for the whole native
/// channel plan and rtl_433's 868 MHz band at the same time, and was being
/// handed a window sized to whichever of the two won a tie. On AUTO that was
/// rtl_433's, sized 1.365 MHz — which a `Ddc` then built at 1.28 MHz, leaving a
/// window that covered neither the band it was chosen for nor two of the four
/// native channels.
///
/// So this pins both halves: the lane runs, and the channel plan is whole
/// beside it.
#[test]
fn one_window_serves_both_lanes_when_the_front_end_has_room() {
    let root = std::env::temp_dir().join(format!("sdroxide-ism-310-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &root) };

    let (producer, _consumer) = rtrb::RingBuffer::<f32>::new(48_000);
    let center = Arc::new(Mutex::new(PLUTO_LO));
    let mut h = start_engine(
        Box::new(Quiet { center, rate: PLUTO_RATE, lo_offset: 0.0, seed: 0x0BAD_F00D_DEAD_BEEF }),
        caps_at(PLUTO_RATE),
        EngineConfig {
            remember_session: false,
            audio: Some(AudioParams { producer, out_rate: 48_000.0 }),
            ..Default::default()
        },
    );
    let thread = h.thread.take();
    let send = |c: Command| h.cmd_tx.send(c).unwrap();

    // Every native family on beside the rtl_433 band, which is the operator's
    // configuration in the ticket. Both widths are checked: the reported one
    // and the one the ticket was filed on.
    for bandwidth_hz in [AUTO, 2_048_000] {
        send(Command::SetVfo { vfo: Vfo::A, hz: PLUTO_LO });
        send(Command::SetIsmConfig(IsmSettings {
            enabled: true,
            families: u32::MAX,
            rtl433: sdroxide_types::Rtl433Settings { bands: BAND_868, bandwidth_hz },
            ..IsmSettings::default()
        }));

        let s = ism_status(&h, "the 868 MHz lane starting", lane_running_on("868 MHz EU"));
        let lane = s.rtl433.as_ref().unwrap();
        assert!(
            lane.rate_hz >= 1_024_000.0,
            "bw {bandwidth_hz}: the lane was fed {:.0} Hz for a band that asked for at least \
             1.024 MHz",
            lane.rate_hz
        );

        // The two channels that have decoders must be live, not "outside the
        // receiver's window" — this receiver had room for them all along.
        for want in [868_300_000.0, 868_420_000.0] {
            let row = s
                .channels
                .iter()
                .find(|c| c.freq_hz == want)
                .unwrap_or_else(|| panic!("no row for {:.3} MHz", want / 1e6));
            assert!(
                row.live,
                "bw {bandwidth_hz}: {:.3} MHz is idle ({:?}) in a {:.3} MHz window on {:.4} MHz",
                want / 1e6,
                row.reason,
                s.window_rate_hz / 1e6,
                s.window_center_hz / 1e6
            );
        }
    }

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
    unsafe { std::env::remove_var("SDROXIDE_CONFIG_DIR") };
}

/// Issue #310 again, after the first fix: the band button has to tune for the
/// *window*, not for the dial.
///
/// The two are the same place only on a front end whose LO sits on the dial. A
/// PlutoSDR parks its LO a quarter-span above, and the ISM window is placed from
/// the LO — so pressing **868 MHz EU** put the dial on 868.650 MHz, the LO on
/// 869.275, and the window 625 kHz above the plan it had just been sent to. On a
/// 2.5 Msps stream there is no slack to slide it back, because the window *is*
/// the stream, so it stayed there and 868.300 MHz read `outside the receiver's
/// window` — which is exactly what was reported.
#[test]
fn the_band_button_tunes_for_the_window_not_for_the_dial() {
    let root = std::env::temp_dir().join(format!("sdroxide-ism-310b-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &root) };

    // A 2.5 Msps zero-IF front end: the plan needs 1.88 MHz, so the window is
    // the undecimated stream and cannot be slid off the LO at all.
    const RATE_2M5: f64 = 2_500_000.0;
    const LO_OFFSET: f64 = 625_000.0;

    let (producer, _consumer) = rtrb::RingBuffer::<f32>::new(48_000);
    let center = Arc::new(Mutex::new(433_920_000.0));
    let seen = Arc::clone(&center);
    let mut h = start_engine(
        Box::new(Quiet {
            center,
            rate: RATE_2M5,
            lo_offset: LO_OFFSET,
            seed: 0x5EED_1234_5678_9ABC,
        }),
        caps_at(RATE_2M5),
        EngineConfig {
            remember_session: false,
            audio: Some(AudioParams { producer, out_rate: 48_000.0 }),
            ..Default::default()
        },
    );
    let thread = h.thread.take();
    let send = |c: Command| h.cmd_tx.send(c).unwrap();

    // The native decoders only: this is about where the button points the
    // receiver, and the channel plan is the thing with a fixed answer.
    send(Command::SetIsmConfig(IsmSettings {
        enabled: true,
        families: u32::MAX,
        rtl433: sdroxide_types::Rtl433Settings::OFF,
        ..IsmSettings::default()
    }));
    // What the band button sends. `Command::SetVfo` here — which is what it used
    // to send — leaves 868.300 MHz outside the window and this test hanging.
    send(Command::TuneWidebandTo(DIAL_868));

    let s = ism_status(&h, "the 868 MHz channel plan in view", |s| {
        s.channels.iter().any(|c| c.freq_hz == 868_300_000.0 && c.live)
    });

    // Every channel, not just the one that was reported: the window is wide
    // enough for the whole plan once it is in the right place.
    for c in &s.channels {
        assert!(
            c.reason.as_deref() != Some("outside the receiver's window"),
            "{:.3} MHz is outside a {:.3} MHz window on {:.4} MHz",
            c.freq_hz / 1e6,
            s.window_rate_hz / 1e6,
            s.window_center_hz / 1e6
        );
    }

    // And the reason it works: the *hardware centre* went to the band, with the
    // dial left an LO offset below it rather than sitting on top of it.
    let hw = *seen.lock().unwrap();
    assert!(
        (hw - DIAL_868).abs() < 1.0,
        "the front end sat on {:.4} MHz, not the band centre",
        hw / 1e6
    );

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
    unsafe { std::env::remove_var("SDROXIDE_CONFIG_DIR") };
}
