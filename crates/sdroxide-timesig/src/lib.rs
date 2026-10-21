//! Standard time signals, from receive audio to "this sample is 14:37:00 UTC".
//!
//! Four pieces, each usable on its own:
//!
//! * [`Decoder`] — audio in; out come the seconds it read and the minutes it
//!   decoded, each tied to the input sample it began on. Long-wave stations
//!   are timed by their carrier's keying (ALS162 by its phase), short-wave
//!   ones by their ticks.
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
mod hf;
mod integrate;
mod search;
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
                Event::Frame { pos, frame, .. } => Some((*pos, frame.utc)),
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
            // Short wave's time code is 21 dB below the carrier the SNR is
            // quoted against, where a long-wave code is the carrier itself.
            // ALS162's bit is one phase element more or less, which carries
            // a few dB less than another 100 ms of reduced carrier does.
            s.snr_db = Some(match station {
                _ if station.is_hf() => 12.0,
                TimeStation::Als162 => -7.0,
                _ => -10.0,
            });
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

    /// WWV and WWVH on one frequency, WWVH 12.7 ms later and weaker — a
    /// listener somewhere in between. Each decoder is timed by its own
    /// station's ticks and says the other is there.
    #[test]
    fn the_short_wave_pair_are_told_apart() {
        let delay = 0.0127;
        for (station, late) in [(TimeStation::Wwv, 0.0), (TimeStation::Wwvh, delay)] {
            let mut s = synth::Synth::new(TimeStation::Wwv, 12_000.0);
            s.sibling = Some(synth::Sibling { delay_s: delay, level: 0.7 });
            s.snr_db = Some(20.0);
            let audio = s.render(start(), 200.0);
            let mut dec = Decoder::new(station, s.rate);
            let mut ev = Vec::new();
            for block in audio.chunks(240) {
                dec.process(block, &mut ev);
            }
            let f = frames(&ev);
            assert!(f.len() >= 2, "{station:?}: {} frames", f.len());
            for (pos, utc) in f {
                let err_ms = (start() + pos / s.rate - utc as f64 - late) * 1000.0;
                assert!(err_ms.abs() < 1.0, "{station:?}: timed {err_ms:.3} ms from its own ticks");
            }
            let snap = dec.snapshot();
            assert!(snap.sibling_heard, "{station:?} should hear the other: {:?}", snap.sibling_z);
        }
        // And alone, nothing is said to be there — however clean the signal,
        // since its own gating is what a careless detector would hear.
        for snr in [20.0, 40.0] {
            let s =
                synth::Synth { snr_db: Some(snr), ..synth::Synth::new(TimeStation::Wwv, 12_000.0) };
            let mut dec = Decoder::new(TimeStation::Wwv, s.rate);
            let mut ev = Vec::new();
            dec.process(&s.render(start(), 200.0), &mut ev);
            let snap = dec.snapshot();
            assert!(!snap.sibling_heard, "WWVH heard where there is none: {:?}", snap.sibling_z);
        }
    }

    /// A short-wave signal fading 20 dB and back every few seconds. The
    /// levels are read against the carrier, so the fade itself costs
    /// nothing; what it costs is the SNR at the bottom of each trough, which
    /// here stays above what the code needs.
    #[test]
    fn short_wave_decodes_through_a_fade() {
        let mut s = synth::Synth::new(TimeStation::Wwv, 12_000.0);
        s.snr_db = Some(35.0);
        s.fade_hz = Some(0.3);
        let f = frames(&run(&s, start(), 200.0));
        assert!(f.len() >= 2, "{} frames through the fade", f.len());
    }

    /// WWVB's phase bits, read second by second, are the ones it sent —
    /// right way up, and each in its own second.
    #[test]
    fn the_phase_bits_are_the_ones_sent() {
        let mut s = synth::Synth::new(TimeStation::Wwvb, 12_000.0);
        s.snr_db = Some(0.0);
        s.tone_hz += 0.7;
        let ev = run(&s, start(), 200.0);
        let codes: Vec<_> = ev
            .iter()
            .filter_map(|e| match e {
                Event::Frame { code, .. } => Some(*code),
                _ => None,
            })
            .collect();
        assert!(codes.contains(&sdroxide_types::TimeCode::Both), "codes {codes:?}");
        let mut checked = 0;
        for e in &ev {
            if let Event::Second(sec) = e
                && let Some(bit) = sec.phase_bit
            {
                let utc = (start() + sec.pos / s.rate).round() as i64;
                assert_eq!(bit, format::wwvb_phase::bit_at(utc), "second {utc}");
                checked += 1;
            }
        }
        assert!(checked > 60, "only {checked} phase bits read");
    }

    /// From XX:10 and XX:40 WWVB's phase carries six minutes of its extended
    /// symbol instead of time frames: the amplitude code carries on, and the
    /// phase code says nothing until the frames come back.
    #[test]
    fn the_extended_symbols_leave_the_phase_code_quiet() {
        let begin = ymd_hms_to_unix(2026, 10, 18, 12, 8, 30) as f64;
        let s = synth::Synth::new(TimeStation::Wwvb, 12_000.0);
        let audio = s.render(begin, 600.0);
        let mut dec = Decoder::new(TimeStation::Wwvb, s.rate);
        let mut ev = Vec::new();
        for block in audio.chunks(240) {
            dec.process(block, &mut ev);
        }
        let ten = ymd_hms_to_unix(2026, 10, 18, 12, 10, 0);
        let mut seen = Vec::new();
        for e in &ev {
            if let Event::Frame { frame, code, .. } = e {
                let extended = (ten..ten + 360).contains(&frame.utc);
                let by_phase = *code != sdroxide_types::TimeCode::Amplitude;
                assert_eq!(by_phase, !extended, "{} by {code:?}", frame.utc);
                seen.push(frame.utc);
            }
        }
        assert!(seen.contains(&(ten + 120)), "the amplitude code carried on: {seen:?}");
        assert!(seen.contains(&(ten + 360)), "the phase code came back: {seen:?}");
    }

    /// DCF77 with its carrier gone for the whole of two seconds — each in a
    /// minute's field, so its parity can stand in. The first is in the very
    /// first minute, with nothing before it to check a filled minute against,
    /// and that minute is lost; the second is filled.
    #[test]
    fn an_unread_second_is_filled_only_with_a_frame_to_check_against() {
        let s = synth::Synth::new(TimeStation::Dcf77, 12_000.0);
        let mut audio = s.render(start(), 330.0);
        let at = |h, m, sec| ymd_hms_to_unix(2026, 10, 18, h, m, sec);
        for gone in [at(12, 37, 25), at(12, 39, 25)] {
            let from = ((gone as f64 - start()) * s.rate) as usize;
            audio[from..from + s.rate as usize].iter_mut().for_each(|x| *x = 0.0);
        }
        let mut dec = Decoder::new(TimeStation::Dcf77, s.rate);
        let mut ev = Vec::new();
        for block in audio.chunks(240) {
            dec.process(block, &mut ev);
        }
        let named: Vec<i64> = frames(&ev).into_iter().map(|(_, utc)| utc).collect();
        let unread =
            ev.iter().filter(|e| matches!(e, Event::Second(sec) if sec.sym.is_none())).count();
        assert_eq!(unread, 2, "the two silent seconds, and only they, were unread");
        assert!(!named.contains(&at(12, 38, 0)), "filled with nothing to check it: {named:?}");
        assert!(named.contains(&at(12, 39, 0)), "{named:?}");
        assert!(named.contains(&at(12, 40, 0)), "the filled minute: {named:?}");
        assert!(named.contains(&at(12, 41, 0)), "{named:?}");
        assert!(!ev.iter().any(|e| matches!(e, Event::BadFrame { .. })), "a filled minute failed");
    }

    /// Too weak for any one minute to read — DCF77 at −16 dB reads none on
    /// its own — and still named, minute after minute, from several.
    #[test]
    fn minutes_too_weak_to_read_are_named_from_several() {
        for (station, snr) in [(TimeStation::Dcf77, -16.0), (TimeStation::Als162, -14.0)] {
            let mut s = synth::Synth::new(station, 8_000.0);
            s.snr_db = Some(snr);
            s.seed = 7;
            let ev = run(&s, start(), 1200.0);
            let f = frames(&ev);
            assert!(f.len() >= 8, "{station:?}: {} frames in twenty minutes", f.len());
            for (pos, utc) in f {
                let err_ms = (start() + pos / s.rate - utc as f64) * 1000.0;
                assert!(err_ms.abs() < 10.0, "{station:?}: frame {err_ms:.3} ms out");
            }
        }
    }

    /// Another station's seconds lock as well as this one's would — MSF's
    /// carrier drops at every second too — but its minute is laid out
    /// differently, and nothing in it must add up to a minute of this one's.
    #[test]
    fn another_stations_code_is_never_named() {
        for (sent, heard) in [
            (TimeStation::Msf, TimeStation::Dcf77),
            (TimeStation::Wwvb, TimeStation::Dcf77),
            (TimeStation::Dcf77, TimeStation::Als162),
        ] {
            let mut s = synth::Synth::new(sent, 8_000.0);
            s.snr_db = Some(0.0);
            let audio = s.render(start(), 1200.0);
            let mut dec = Decoder::new(heard, s.rate);
            let mut ev = Vec::new();
            for block in audio.chunks(160) {
                dec.process(block, &mut ev);
            }
            assert!(frames(&ev).is_empty(), "{heard:?} named a minute from {sent:?}");
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

    /// A carrier on the air with nothing on it, in noise: the transmitter is
    /// there, the time code is not. Nothing must be read from it, and on
    /// short wave — where the carrier is what everything is measured against
    /// — not even a second.
    #[test]
    fn dead_air_never_decodes() {
        for station in TimeStation::ALL {
            let mut s = synth::Synth::new(station, 8_000.0);
            s.dead_air = true;
            s.snr_db = Some(if station.is_hf() { 10.0 } else { -10.0 });
            s.seed = 5;
            let ev = run(&s, start(), 600.0);
            assert!(frames(&ev).is_empty(), "{station:?} decoded a frame from a bare carrier");
            let locks = ev.iter().filter(|e| matches!(e, Event::Locked)).count();
            assert_eq!(locks, 0, "{station:?} found seconds on a bare carrier");
        }
    }

    /// `cargo test -p sdroxide-timesig --release -- --ignored --nocapture`
    ///
    /// Each station's six hours are noise alone and then a bare carrier in
    /// noise, three hours of each.
    #[test]
    #[ignore]
    fn noise_never_decodes_long() {
        for station in TimeStation::ALL {
            let mut s = synth::Synth::new(station, 8_000.0);
            let mut total = 0;
            let mut total_locks = 0;
            for seed in 0..12 {
                s.seed = 100 + seed;
                s.dead_air = seed % 2 == 1;
                s.snr_db = Some(match (s.dead_air, station.is_hf()) {
                    (false, _) => -200.0,
                    (true, true) => 10.0,
                    (true, false) => -10.0,
                });
                let ev = run(&s, start(), 1800.0);
                let n = frames(&ev).len();
                let bad = ev.iter().filter(|e| matches!(e, Event::BadFrame { .. })).count();
                let locks = ev.iter().filter(|e| matches!(e, Event::Locked)).count();
                let what = if s.dead_air { "carrier" } else { "noise" };
                println!("{station:?} seed {seed} {what}: {n} frames, {bad} bad, {locks} locks");
                total += n;
                total_locks += locks;
            }
            assert_eq!(total, 0, "{station:?}: frames from six hours of noise");
            assert_eq!(total_locks, 0, "{station:?}: seconds found in six hours of noise");
        }
    }

    /// The integrator's reach, and that it never names a wrong minute: forty
    /// minutes at a time, from where single minutes read down to where the
    /// seconds can no longer be held.
    ///
    /// `cargo test -p sdroxide-timesig --release -- --ignored --nocapture integration_sweep`
    #[test]
    #[ignore]
    fn integration_sweep() {
        let mut wrong_total = 0;
        for station in [TimeStation::Dcf77, TimeStation::Als162] {
            for snr in [-12.0, -14.0, -16.0, -18.0, -20.0, -22.0, -24.0] {
                let (mut got, mut wrong, mut first) = (0usize, 0usize, Vec::new());
                let seeds = 6;
                for seed in 0..seeds {
                    let mut s = synth::Synth::new(station, 8_000.0);
                    s.snr_db = Some(snr);
                    s.seed = 40 + seed;
                    let ev = run(&s, start(), 2400.0);
                    let f = frames(&ev);
                    if let Some(&(pos, _)) = f.first() {
                        first.push(pos / s.rate / 60.0);
                    }
                    for (pos, utc) in f {
                        let err = (start() + pos / s.rate - utc as f64) * 1000.0;
                        if err.abs() < 500.0 {
                            got += 1;
                        } else {
                            wrong += 1;
                        }
                    }
                }
                first.sort_by(f64::total_cmp);
                let median = first.get(first.len() / 2).map_or(f64::NAN, |m| *m);
                println!(
                    "{:<7} {snr:>6} dB: {got:>3}/{} minutes, {wrong} wrong, first after {median:.0} min",
                    station.label(),
                    40 * seeds
                );
                wrong_total += wrong;
            }
        }
        assert_eq!(wrong_total, 0, "minutes named wrong");
    }

    /// How decoding holds up as the signal fades: the share of minutes that
    /// decode, and the timing error, at each SNR.
    ///
    /// `cargo test -p sdroxide-timesig --release -- --ignored --nocapture sweep`
    ///
    /// Short wave is swept from much higher: its time code is 21 dB below the
    /// carrier the SNR is quoted against. WWVB's minutes are also counted by
    /// which code read them, which is what the phase code is worth.
    #[test]
    #[ignore]
    fn sensitivity_sweep() {
        for station in TimeStation::ALL {
            let snrs: &[f64] = if station.is_hf() {
                &[20.0, 10.0, 5.0, 0.0, -5.0, -10.0]
            } else if station == TimeStation::Wwvb {
                &[-10.0, -15.0, -20.0, -25.0, -28.0, -31.0, -34.0, -37.0]
            } else {
                &[-10.0, -15.0, -20.0, -25.0, -28.0, -31.0]
            };
            for &snr in snrs {
                let mut got = 0usize;
                let mut wrong = 0usize;
                let mut by_phase = 0usize;
                let mut worst_ms = 0.0f64;
                let seeds = 4;
                for seed in 0..seeds {
                    let mut s = synth::Synth::new(station, 12_000.0);
                    s.snr_db = Some(snr);
                    s.seed = 1 + seed;
                    let ev = run(&s, start(), 600.0);
                    for e in &ev {
                        let Event::Frame { pos, frame, code } = e else { continue };
                        let err = (start() + pos / s.rate - frame.utc as f64) * 1000.0;
                        if err.abs() < 500.0 {
                            worst_ms = worst_ms.max(err.abs());
                            got += 1;
                            by_phase += usize::from(*code != sdroxide_types::TimeCode::Amplitude);
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
                    "{:<7} {snr:>6} dB: {got:>2}/{possible} minutes ({by_phase} by phase), \
                     {wrong} wrong, worst {worst_ms:.2} ms",
                    station.label()
                );
            }
        }
    }
}
