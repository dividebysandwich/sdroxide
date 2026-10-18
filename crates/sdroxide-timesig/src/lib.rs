//! Standard time signals, from receive audio to "this sample is 14:37:00 UTC".
//!
//! Four pieces, each usable on its own:
//!
//! * [`Decoder`] — audio in; out come the seconds it read and the minutes it
//!   decoded, each tied to the input sample it began on.
//! * [`Tracker`] — decides when consecutive minutes agree well enough to be
//!   believed, and from then on names the UTC second every boundary began.
//! * [`SampleClock`] — which reading of the computer's clock an input sample
//!   arrived at, from the readings taken as each block came in.
//! * [`synth::Synth`] — a station in a box, for the tests.
//!
//! Put together (as the digi controller does), a second boundary at sample
//! `p` is UTC second `tracker.utc_at(p)`, which arrived at clock reading
//! `clock.wall_ns_at(p)`; the difference, less the signal's flight time and
//! the receiver's latency, is how far the computer's clock is out.

#![forbid(unsafe_code)]

mod civil;
mod clock;
mod decoder;
pub mod format;
mod frontend;
pub mod synth;
mod tracker;

pub use clock::SampleClock;
pub use decoder::{Decoder, Event, Second, Snapshot};
pub use format::{Context, Format, FrameError, Sym, Template};
pub use tracker::{Anchor, Tracker};

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_types::{TimeStation, ymd_hms_to_unix};

    /// Runs a synthetic station through the decoder in engine-sized blocks
    /// and returns every event, with the UTC instant the first sample was.
    fn run(synth: &synth::Synth, start: f64, secs: f64) -> Vec<Event> {
        let audio = synth.render(start, secs);
        let mut dec = Decoder::new(synth.station, synth.rate);
        let mut events = Vec::new();
        for block in audio.chunks((synth.rate / 50.0) as usize) {
            dec.process(block, &mut events);
        }
        events
    }

    fn frames(events: &[Event]) -> Vec<(f64, i64)> {
        events
            .iter()
            .filter_map(|e| match e {
                Event::Frame { pos, frame } => Some((*pos, frame.utc)),
                _ => None,
            })
            .collect()
    }

    /// Start 23.4 s into a minute so the lock and the first frame have to be
    /// found, not handed over on a boundary.
    fn start() -> f64 {
        ymd_hms_to_unix(2026, 10, 18, 12, 36, 23) as f64 + 0.4
    }

    #[test]
    fn every_station_decodes_cleanly_and_on_time() {
        for station in TimeStation::ALL {
            for rate in [12_000.0, 48_000.0, 44_100.0] {
                let s = synth::Synth::new(station, rate);
                let ev = run(&s, start(), 200.0);
                let f = frames(&ev);
                assert!(f.len() >= 2, "{station:?} @ {rate}: {} frames", f.len());
                for (pos, utc) in f {
                    let err_ms = (start() + pos / rate - utc as f64) * 1000.0;
                    assert!(err_ms.abs() < 1.0, "{station:?} @ {rate}: frame {err_ms:.3} ms out");
                }
                assert!(
                    !ev.iter().any(|e| matches!(e, Event::BadFrame { .. })),
                    "{station:?} @ {rate}: a clean signal produced a bad frame"
                );
            }
        }
    }

    #[test]
    fn decodes_through_noise_off_frequency_and_off_rate() {
        for station in TimeStation::ALL {
            let mut s = synth::Synth::new(station, 12_000.0);
            s.snr_db = Some(-10.0);
            s.tone_hz = 1000.0 + 187.0;
            s.rate_error_ppm = 40.0;
            s.seed = 3;
            let ev = run(&s, start(), 260.0);
            let f = frames(&ev);
            assert!(f.len() >= 2, "{station:?}: {} frames", f.len());
            for (pos, utc) in f {
                // The receiver's samples are 40 ppm short of true seconds.
                // One frame's timing at this SNR is good to a few
                // milliseconds; the controller's median over every second
                // does better than any one of them.
                let t = start() + pos / (s.rate * (1.0 + 40e-6));
                let err_ms = (t - utc as f64) * 1000.0;
                assert!(err_ms.abs() < 4.0, "{station:?}: frame {err_ms:.3} ms out");
            }
        }
    }

    #[test]
    fn the_tracker_believes_two_agreeing_minutes() {
        let s = synth::Synth::new(TimeStation::Dcf77, 12_000.0);
        let ev = run(&s, start(), 200.0);
        let mut tr = Tracker::new(s.rate, 2);
        for (pos, utc) in frames(&ev) {
            tr.frame(pos, utc);
        }
        let a = tr.anchor().expect("believed");
        // Every later second boundary is a whole UTC second.
        for e in &ev {
            if let Event::Second(sec) = e
                && sec.pos > a.pos
            {
                let utc = tr.utc_at(sec.pos).expect("named");
                let err_ms = (start() + sec.pos / s.rate - utc as f64) * 1000.0;
                assert!(err_ms.abs() < 1.0, "second {utc} is {err_ms:.3} ms out");
            }
        }
    }

    /// Noise alone must never produce a frame. Ten minutes here, for every
    /// station; `noise_never_decodes_long` runs for hours.
    #[test]
    fn noise_alone_never_decodes() {
        for station in TimeStation::ALL {
            let mut s = synth::Synth::new(station, 8_000.0);
            s.snr_db = Some(-200.0);
            s.seed = 11;
            let ev = run(&s, start(), 600.0);
            assert!(frames(&ev).is_empty(), "{station:?} decoded a frame from noise");
        }
    }

    /// `cargo test -p sdroxide-timesig --release -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn noise_never_decodes_long() {
        for station in TimeStation::ALL {
            let mut s = synth::Synth::new(station, 8_000.0);
            s.snr_db = Some(-200.0);
            let mut total = 0;
            for seed in 0..12 {
                s.seed = 100 + seed;
                let ev = run(&s, start(), 1800.0);
                let n = frames(&ev).len();
                let bad = ev.iter().filter(|e| matches!(e, Event::BadFrame { .. })).count();
                let locks = ev.iter().filter(|e| matches!(e, Event::Locked)).count();
                println!("{station:?} seed {seed}: {n} frames, {bad} bad, {locks} locks");
                total += n;
            }
            assert_eq!(total, 0, "{station:?}: frames from six hours of noise");
        }
    }

    /// How decoding holds up as the signal fades: the share of minutes that
    /// decode, and the timing error, at each SNR.
    ///
    /// `cargo test -p sdroxide-timesig --release -- --ignored --nocapture sweep`
    #[test]
    #[ignore]
    fn sensitivity_sweep() {
        for station in TimeStation::ALL {
            for snr in [-10.0, -15.0, -20.0, -25.0, -28.0, -31.0] {
                let mut got = 0usize;
                let mut wrong = 0usize;
                let mut worst_ms = 0.0f64;
                let seeds = 4;
                for seed in 0..seeds {
                    let mut s = synth::Synth::new(station, 12_000.0);
                    s.snr_db = Some(snr);
                    s.seed = 1 + seed;
                    let ev = run(&s, start(), 600.0);
                    for (pos, utc) in frames(&ev) {
                        let err = (start() + pos / s.rate - utc as f64) * 1000.0;
                        if err.abs() < 500.0 {
                            worst_ms = worst_ms.max(err.abs());
                            got += 1;
                        } else {
                            // Passed every check and names the wrong minute:
                            // what the tracker's agreement rule is for.
                            wrong += 1;
                        }
                    }
                }
                // Ten minutes, of which the first is spent finding the second.
                let possible = 9 * seeds as usize;
                println!(
                    "{:<7} {snr:>6} dB: {got:>2}/{possible} minutes, {wrong} wrong, worst {worst_ms:.2} ms",
                    station.label()
                );
            }
        }
    }
}
