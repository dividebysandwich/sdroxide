//! What the panadapter resolves has to follow the window on screen, not the
//! width the front end happens to be streaming.
//!
//! The device-wide analyser is a fixed number of bins across the whole stream,
//! so the further in the operator zooms the fewer of them land on screen. On a
//! narrow front end that never bites; on a wide one it bites almost at once. An
//! RX-888 asked for 8.1 MHz gives 247 Hz a bin through the largest FFT the
//! display will ask for, so a 68 kHz window on screen was drawn out of 275
//! numbers and stair-stepped visibly. Front-end decimation was the only cure,
//! and it buys the resolution by throwing the rest of the band away.
//!
//! So a viewport the device-wide analyser can no longer fill is served from a
//! zoom lane instead: the window mixed down and decimated to its own width,
//! analysed there. These tests pin the two halves of that — the resolution it
//! is for, and the frequency accuracy that makes the resolution worth having.

use std::f64::consts::TAU;
use std::time::{Duration, Instant};

use sdroxide_radio::{Complex32, EngineConfig, EngineHandles, IqSource, Result, start_engine};
use sdroxide_types::{Command, DeviceCaps, SpectrumConfig, SpectrumFrame};

/// A front end as wide as an RX-888 asked for a third of its half-spectrum.
const RATE: f64 = 8_100_000.0;
const CENTER: f64 = 14_200_000.0;

/// Where the pair sits. Well off the front end's own centre, so the DC-spike
/// suppression cannot be what makes them hard to see — and so the lane's NCO
/// has a real offset to get the sign of.
const TONE_MID_HZ: f64 = CENTER + 100_000.0;
/// Two tones this far apart. A fifth of one bin of the device-wide analyser
/// (8.1 MHz over 4096 points is 1978 Hz), and 52 bins apart through the lane.
const TONE_GAP_HZ: f64 = 400.0;
const TONE_A_HZ: f64 = TONE_MID_HZ - TONE_GAP_HZ / 2.0;
const TONE_B_HZ: f64 = TONE_MID_HZ + TONE_GAP_HZ / 2.0;

/// The window the client asks to see: 10 kHz, which is 5 bins of the
/// device-wide analyser and 1300 of the lane's.
const VIEW_HZ: f64 = 10_000.0;

/// How far below the weaker peak the trough between two tones has to sit before
/// they count as resolved, in the u8 units a frame carries — about 11 dB over
/// the 140 dB window these tests map.
const TROUGH_UNITS: i32 = 20;

/// A front end streaming two closely spaced tones at the device rate.
struct TwoTones {
    center_hz: f64,
    phase_a: f64,
    phase_b: f64,
}

impl IqSource for TwoTones {
    fn sample_rate(&self) -> f64 {
        RATE
    }
    fn center_hz(&self) -> f64 {
        self.center_hz
    }
    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center_hz = hz;
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        // Paced so the engine's loop runs at a realistic rate without the test
        // waiting real time for 8.1 Msps.
        std::thread::sleep(Duration::from_millis(2));
        let (sa, sb) =
            (TAU * (TONE_A_HZ - self.center_hz) / RATE, TAU * (TONE_B_HZ - self.center_hz) / RATE);
        for s in buf.iter_mut() {
            *s = Complex32::new(
                (self.phase_a.cos() + self.phase_b.cos()) as f32 * 0.4,
                (self.phase_a.sin() + self.phase_b.sin()) as f32 * 0.4,
            );
            self.phase_a = (self.phase_a + sa) % TAU;
            self.phase_b = (self.phase_b + sb) % TAU;
        }
        Ok(buf.len())
    }
    fn describe(&self) -> String {
        "two-tone generator".into()
    }
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "test".into(),
        label: "test".into(),
        rx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(1_000_000.0, 60_000_000.0)],
        ..DeviceCaps::default()
    }
}

fn engine() -> EngineHandles {
    start_engine(
        Box::new(TwoTones { center_hz: CENTER, phase_a: 0.0, phase_b: 0.0 }),
        caps(),
        EngineConfig { remember_session: false, ..Default::default() },
    )
}

/// The spectrum config a client sends for a `span_hz` window on the centre —
/// `None` for the fully zoomed-out view, which is what the device-wide
/// analyser already is. `bins` is the panadapter width the client is drawing,
/// which is what the lane's threshold and its own FFT are both measured
/// against.
fn cfg_at(span_hz: Option<f64>, bins: u32) -> Command {
    cfg_full(span_hz, bins, 28)
}

/// The same, with the waterfall's row clock named too.
fn cfg_full(span_hz: Option<f64>, bins: u32, rows_per_sec: u16) -> Command {
    Command::SetSpectrumCfg(SpectrumConfig {
        fft_size: 4096,
        display_bins: bins,
        rows_per_sec,
        db_floor: -140.0,
        db_ceil: 0.0,
        viewport: span_hz.map(|s| (TONE_MID_HZ - s / 2.0, TONE_MID_HZ + s / 2.0)),
        smooth: false,
        reassign: false,
        reassign_floor_db: 6.0,
        reassign_dim_db: 0.0,
        fps: 30,
        avg_tc: 0.0,
    })
}

/// The same, on the historic 2048-column panadapter.
fn cfg(span_hz: Option<f64>) -> Command {
    cfg_at(span_hz, 2048)
}

/// A `span_hz` window centred wherever the operator has dragged it to, rather
/// than on the tones.
fn cfg_window(center_hz: f64, span_hz: f64) -> Command {
    Command::SetSpectrumCfg(SpectrumConfig {
        fft_size: 4096,
        display_bins: 2048,
        rows_per_sec: 28,
        db_floor: -140.0,
        db_ceil: 0.0,
        viewport: Some((center_hz - span_hz / 2.0, center_hz + span_hz / 2.0)),
        smooth: false,
        reassign: false,
        reassign_floor_db: 6.0,
        reassign_dim_db: 0.0,
        fps: 30,
        avg_tc: 0.0,
    })
}

/// Collect frames for `secs` and return the last one whose span matches what
/// was asked for, so a frame still in flight from the previous config cannot be
/// mistaken for the answer.
fn frame_of_span(h: &mut EngineHandles, want_span: f64, secs: f64) -> SpectrumFrame {
    let mut got: Option<SpectrumFrame> = None;
    let deadline = Instant::now() + Duration::from_secs_f64(secs);
    while Instant::now() < deadline {
        if h.spectrum_out.update() {
            let f = h.spectrum_out.output_buffer();
            if (f.span_hz - want_span).abs() < want_span * 0.05 && !f.bins.is_empty() {
                got = Some(f.clone());
            }
        }
        while h.event_rx.try_recv().is_ok() {}
        std::thread::sleep(Duration::from_millis(5));
    }
    got.unwrap_or_else(|| panic!("no {want_span} Hz frame arrived"))
}

/// Whether this frame shows the pair as *two* signals: a peak at each tone and
/// a trough between them well below both.
///
/// A frame whose columns are wider than the gap cannot, by construction — there
/// is nothing between the two to be a trough — which is exactly the state the
/// zoom lane exists to get out of, so it reports false rather than asserting.
fn resolves_the_pair(f: &SpectrumFrame) -> bool {
    let base = f.center_hz - f.span_hz / 2.0;
    let n = f.bins.len();
    let col = |hz: f64| ((hz - base) / f.span_hz * n as f64).floor() as isize;
    let (a, b) = (col(TONE_A_HZ), col(TONE_B_HZ));
    if b - a < 2 || a < 0 || b >= n as isize {
        return false;
    }
    // The tone need not land dead centre of its column, so look either side.
    let peak = |c: isize| {
        (c - 3..=c + 3)
            .filter_map(|i| usize::try_from(i).ok().and_then(|i| f.bins.get(i)))
            .copied()
            .max()
            .unwrap_or(0)
    };
    let trough = (a + 4..b - 3)
        .filter_map(|i| usize::try_from(i).ok().and_then(|i| f.bins.get(i)))
        .copied()
        .min()
        .unwrap_or(255);
    i32::from(peak(a).min(peak(b))) - i32::from(trough) >= TROUGH_UNITS
}

/// The point of the lane: two tones the device-wide analyser cannot tell apart
/// are two signals on screen once the operator has zoomed in on them.
#[test]
fn a_zoomed_viewport_resolves_what_the_device_wide_fft_cannot() {
    let mut h = engine();
    let thread = h.thread.take();

    // Zoomed out first, to show the pair is genuinely unresolvable there: one
    // bin of the device-wide analyser is five times the gap between them.
    h.cmd_tx.send(cfg(None)).unwrap();
    let wide = frame_of_span(&mut h, RATE, 1.5);
    assert!(
        !resolves_the_pair(&wide),
        "the device-wide analyser should see one blur, not two tones"
    );

    // Now the window the operator would open on them.
    h.cmd_tx.send(cfg(Some(VIEW_HZ))).unwrap();
    let zoomed = frame_of_span(&mut h, VIEW_HZ, 3.0);
    assert!(resolves_the_pair(&zoomed), "the zoomed window should resolve both tones");

    // And the strongest thing on screen is one of them, at the frequency it
    // really is: a mis-signed NCO offset would mirror the pair about the centre
    // and still "resolve" two peaks.
    let (top, _) =
        zoomed.bins.iter().enumerate().max_by_key(|(_, v)| **v).expect("a frame has bins");
    let hz = zoomed.freq_at_bin(top);
    let tol = TONE_GAP_HZ;
    assert!(
        (hz - TONE_MID_HZ).abs() < tol,
        "the pair read at {hz}, expected {TONE_MID_HZ} ± {tol:.0}"
    );

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
}

/// The lane is an addition, not a replacement: with no viewport the frame is
/// still the whole of what the front end is streaming, and going back to it
/// from a zoomed window has to restore it.
#[test]
fn the_zoomed_out_view_is_still_the_whole_window() {
    let mut h = engine();
    let thread = h.thread.take();

    h.cmd_tx.send(cfg(Some(VIEW_HZ))).unwrap();
    let zoomed = frame_of_span(&mut h, VIEW_HZ, 2.0);
    assert!((zoomed.center_hz - TONE_MID_HZ).abs() < VIEW_HZ * 0.05);

    h.cmd_tx.send(cfg(None)).unwrap();
    let wide = frame_of_span(&mut h, RATE, 2.0);
    assert!(
        (wide.span_hz - RATE).abs() < 1.0,
        "the zoomed-out frame should span the whole stream, got {}",
        wide.span_hz
    );

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
}

/// A viewport the device-wide analyser can still fill is left to it. The lane
/// costs a decimation of every sample and restarts its averaging whenever the
/// window moves, so it may only exist where it buys something: three quarters
/// of the span still gets three quarters of these 4096 bins, more than the 2048
/// columns [`cfg`] asks the frame for.
///
/// In the running application the client grows its own FFT with the zoom, so
/// this covers everything down to about a thirty-second of the span before the
/// lane is reached at all.
#[test]
fn a_shallow_zoom_is_left_to_the_device_wide_analyser() {
    let mut h = engine();
    let thread = h.thread.take();

    let span = RATE * 0.75;
    h.cmd_tx.send(cfg(Some(span))).unwrap();
    let f = frame_of_span(&mut h, span, 2.0);
    // Served from the device-wide analyser the frame is a slice of its bins, so
    // the tones are still one blur. Through a lane they would be two.
    assert!(
        !resolves_the_pair(&f),
        "a three-quarter-span viewport should not have cost a zoom lane"
    );

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
}

/// The panadapter's width is the client's to choose, and the lane follows it.
///
/// Two things at once, because they are the same thing: the frame really is as
/// wide as was asked for, and the zoom lane — whose FFT is sized from that
/// width ([`sdroxide_radio::engine`]'s `zoom_lane_fft`) — still resolves the
/// pair at twice the columns. A lane left at the width it had for a
/// 2048-column display would stair-step here, which is the complaint of issue
/// #172 one zoom level further in.
#[test]
fn the_client_chooses_the_panadapter_width_and_the_lane_follows() {
    let mut h = engine();
    let thread = h.thread.take();

    h.cmd_tx.send(cfg_at(Some(VIEW_HZ), 4096)).unwrap();
    let f = frame_of_span(&mut h, VIEW_HZ, 3.0);
    assert_eq!(f.bins.len(), 4096, "the frame should be as wide as the client asked for");
    assert!(resolves_the_pair(&f), "a 4096-column zoomed window should resolve both tones");

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
}

/// A width nobody sane would send is not a width the engine tries to serve.
///
/// The number arrives over the network, so the clamp in
/// [`sdroxide_types::SpectrumConfig::bins`] is the only thing between a hostile
/// or broken client and an allocation the size of its imagination.
#[test]
fn an_absurd_width_is_held_to_the_ceiling() {
    let mut h = engine();
    let thread = h.thread.take();

    h.cmd_tx.send(cfg_at(None, u32::MAX)).unwrap();
    let f = frame_of_span(&mut h, RATE, 2.0);
    assert_eq!(f.bins.len(), sdroxide_types::MAX_DISPLAY_BINS as usize);

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
}

/// The waterfall's time resolution is the client's to ask for, and it is not
/// the frame rate.
///
/// This is the whole of the second half of issue #172. A frame used to *be* a
/// row, so a waterfall could never advance faster than the screen redrew: at
/// 30 fps and a 56-line-a-second scroll the client wrote the same numbers
/// twice and the operator saw lines two pixels tall, while the analyser behind
/// them was producing hundreds of transforms a second that nobody looked at.
///
/// So: ask for four times the frame rate in rows, and count the rows that come
/// back over a second of wall clock. They have to arrive at the rate asked for,
/// not at the rate frames do.
#[test]
fn the_waterfall_clock_runs_faster_than_the_frame_clock() {
    let mut h = engine();
    let thread = h.thread.take();

    // Deliberately past the rate blocks arrive at. This front end streams
    // 8.1 Msps and the engine reads 16384 samples at a time, so a row clocked
    // once per block could only ever reach about 494 a second — and on the
    // 1.5 Msps sources this suite's cousins use, 94. Asking for 400 is asking
    // for rows *inside* a block, which is the thing being tested.
    const FPS: u16 = 30;
    const ROWS: u16 = 400;
    h.cmd_tx.send(cfg_full(None, 2048, ROWS)).unwrap();

    // Let the lane settle before counting, so the first partial second and the
    // config round trip are not in the sample.
    let _ = frame_of_span(&mut h, RATE, 1.0);

    let (mut rows, mut frames) = (0usize, 0usize);
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(3) {
        if h.spectrum_out.update() {
            let f = h.spectrum_out.output_buffer();
            if !f.bins.is_empty() {
                frames += 1;
                rows += f.row_count();
                // Every row is a whole row of this frame's own width.
                assert_eq!(f.rows.len() % f.bins.len(), 0);
            }
        }
        while h.event_rx.try_recv().is_ok() {}
        std::thread::sleep(Duration::from_millis(2));
    }

    let secs = start.elapsed().as_secs_f64();
    let row_rate = rows as f64 / secs;
    let frame_rate = frames as f64 / secs;

    // Loose bounds on purpose. This is a wall-clock measurement taken while a
    // dozen other test binaries may be running, so the engine thread can be
    // starved to a fraction of its rate; what must survive that is the shape of
    // the answer, not its hundredths.
    //
    // The floor still discriminates against the two ways this can regress. A
    // row clocked per *block* would land near 94 a second on this source, and a
    // row clocked per *frame* near 30 — both far below this even if the engine
    // only gets half the machine.
    assert!(
        row_rate > 150.0,
        "asked for {ROWS} rows/s and got {row_rate:.0} ({rows} rows in {secs:.1} s) — \
         a row clocked per block would look like this"
    );
    assert!(
        row_rate > frame_rate * 3.0,
        "the waterfall should outrun the frame clock several times over: \
         {row_rate:.0} rows/s vs {frame_rate:.0} frames/s"
    );
    // And the frames themselves must not have sped up to match — the whole
    // point is that a repaint stayed as expensive as it was.
    assert!(frame_rate < f64::from(FPS) * 1.5, "{frame_rate:.0} frames/s");

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
}

/// A row is the loudest thing its slice of time contained, not whatever
/// transform happened to be last.
///
/// That is what makes a fast waterfall worth having: a signal shorter than the
/// interval between two rows — a CW dot, the edge of a burst — used to have a
/// real chance of falling between two frames and never being drawn at all.
/// Here the tones are continuous, so the test is the weaker but sufficient one
/// that every row is a real picture of the band rather than an empty or
/// stale one.
#[test]
fn every_row_is_a_picture_of_the_band() {
    let mut h = engine();
    let thread = h.thread.take();

    h.cmd_tx.send(cfg_full(None, 2048, 120)).unwrap();
    let _ = frame_of_span(&mut h, RATE, 1.0);

    let mut checked = 0usize;
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline && checked < 40 {
        if h.spectrum_out.update() {
            let f = h.spectrum_out.output_buffer().clone();
            let cols = f.bins.len();
            for row in f.rows.chunks_exact(cols) {
                let peak = row.iter().copied().max().unwrap_or(0);
                assert!(peak > 0, "a row of the band should not be blank");
                // The pair sits at TONE_MID_HZ; the strongest column in the row
                // has to be one of them, not noise somewhere else.
                let top = row.iter().enumerate().max_by_key(|(_, v)| **v).map(|(i, _)| i).unwrap();
                let hz = f.freq_at_bin(top);
                assert!(
                    (hz - TONE_MID_HZ).abs() < RATE * 0.01,
                    "row peaked at {hz}, expected the tones at {TONE_MID_HZ}"
                );
                checked += 1;
            }
        }
        while h.event_rx.try_recv().is_ok() {}
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(checked >= 10, "only {checked} rows arrived to check");

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
}

/// A window dragged across the band keeps its lane, and the lane keeps up.
///
/// The window is where the NCO is pointed, and nothing else about the lane
/// depends on it — so a pan re-points it and the filters, the averaging and the
/// waterfall all carry on. Rebuilding instead cost a lane with no spectrum in
/// it at every step of the drag, which is what put a black band across the
/// whole width of the waterfall for as long as the operator held the mouse
/// down.
///
/// The proof that it really moved: the pair is out of view at the start and
/// resolved, at the frequency it really sits on, at the end. A lane left
/// pointed where it was built would show the empty band it started on.
#[test]
fn a_dragged_window_keeps_its_lane_and_still_shows_the_band() {
    let mut h = engine();
    let thread = h.thread.take();

    // Well clear of the pair — four windows away, so nothing of it is in view.
    let start_hz = TONE_MID_HZ - VIEW_HZ * 4.0;
    h.cmd_tx.send(cfg_window(start_hz, VIEW_HZ)).unwrap();
    let away = frame_of_span(&mut h, VIEW_HZ, 3.0);
    assert!(!resolves_the_pair(&away), "the pair should be out of view to start with");

    // The drag: a tenth of the window per displayed frame, as a client sends it.
    for step in 1..=40 {
        let at = start_hz + step as f64 * VIEW_HZ / 10.0;
        h.cmd_tx.send(cfg_window(at.min(TONE_MID_HZ), VIEW_HZ)).unwrap();
        std::thread::sleep(Duration::from_millis(16));
        while h.spectrum_out.update() {}
        while h.event_rx.try_recv().is_ok() {}
    }

    let onto = frame_of_span(&mut h, VIEW_HZ, 2.0);
    assert!(resolves_the_pair(&onto), "the dragged window should resolve the pair it landed on");
    let (top, _) = onto.bins.iter().enumerate().max_by_key(|(_, v)| **v).expect("a frame has bins");
    let hz = onto.freq_at_bin(top);
    assert!(
        (hz - TONE_MID_HZ).abs() < TONE_GAP_HZ,
        "the pair read at {hz}, expected {TONE_MID_HZ} ± {TONE_GAP_HZ:.0}"
    );

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
}
