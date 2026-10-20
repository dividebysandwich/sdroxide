//! WEFAX against four pieces of one real recording.
//!
//! The recording is a Pinneberg (DDH3/DDK6/DDK3) chart received off air: 48 kHz
//! mono, twenty-six minutes, with a **carrier standing on the channel the whole
//! time** — 2328 Hz, which is 28 Hz *above* white, so it reads as a blank page —
//! that fades under the signal and comes back over it. The transmission is one
//! chart, from a phasing signal at about 40 s to a stop tone at about 1122 s.
//! The start tone is not recoverable (it sits under the carrier), and there is
//! no audio tone at 300, 450 or 675 Hz anywhere in it.
//!
//! What it did to the decoder, each piece below pinning one:
//!
//! * **`intro_carrier`** — the carrier, then a faded intro. Nothing in it is a
//!   transmission, and nothing may be announced.
//! * **`start`** — the phasing signal alone, a quarter of a line wide and
//!   scattered by noise. The decoder has to start a page from it, with no tone.
//! * **`noise_at_the_stop_rate`** — ten seconds of the chart in which the
//!   discriminator's noise crosses mid-grey 450 times a second, exactly the stop
//!   tone's rate. It ended the page there, twenty seconds before the page's
//!   own end.
//! * **`stop`** — the stop tone itself, clean, which must still end the page.
//!
//! The pieces are cut from the recording. Three are stored at 6 kHz (the carrier
//! is at 2.3 kHz) and brought back to the decoder's 12 kHz by linear
//! interpolation; the noisy one is kept at 12 kHz, because the false stop it
//! pins depends on the exact count of mid-grey crossings and a band-limited copy
//! has fewer of them. They come to under a megabyte, and the whole recording is
//! not in the tree.

use sdroxide_dsp::{WefaxEvent, WefaxIoc, WefaxLpm, WefaxRx};

const RATE: f64 = 12_000.0;

fn piece(name: &str) -> Vec<f32> {
    let path = format!("{}/tests/testdata/wefax/{name}.wav", env!("CARGO_MANIFEST_DIR"));
    let mut reader = hound::WavReader::open(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let spec = reader.spec();
    assert_eq!(spec.channels, 1, "{path}");
    let x: Vec<f32> = reader.samples::<i16>().map(|s| f32::from(s.unwrap()) / 32768.0).collect();
    match spec.sample_rate {
        12_000 => x,
        // Twice the rate, by linear interpolation.
        6_000 => {
            let mut out = Vec::with_capacity(x.len() * 2);
            for w in x.windows(2) {
                out.push(w[0]);
                out.push(0.5 * (w[0] + w[1]));
            }
            out
        }
        other => panic!("{path}: {other} Hz"),
    }
}

fn run(rx: &mut WefaxRx, audio: &[f32]) -> Vec<(f64, WefaxEvent)> {
    let mut all = Vec::new();
    let mut fed = 0usize;
    for block in audio.chunks(1024) {
        let mut ev = Vec::new();
        rx.process(block, &mut ev);
        fed += block.len();
        all.extend(ev.into_iter().map(|e| (fed as f64 / RATE, e)));
    }
    all
}

fn receiver() -> WefaxRx {
    let mut rx = WefaxRx::new(RATE);
    rx.set_lpm(WefaxLpm::L120);
    rx.set_ioc(WefaxIoc::I576);
    rx
}

fn lines(ev: &[(f64, WefaxEvent)]) -> usize {
    ev.iter().filter(|(_, e)| matches!(e, WefaxEvent::Line { .. })).count()
}

#[test]
fn a_carrier_and_a_faded_intro_start_nothing() {
    let mut rx = receiver();
    let ev = run(&mut rx, &piece("intro_carrier"));
    assert!(ev.is_empty(), "a transmission was announced that was not one: {ev:?}");
    assert!(!rx.receiving());
}

#[test]
fn the_phasing_signal_alone_starts_the_page_under_a_carrier() {
    let mut rx = receiver();
    let ev = run(&mut rx, &piece("start"));

    let start = ev
        .iter()
        .find_map(|(t, e)| matches!(e, WefaxEvent::Start { auto: true, .. }).then_some(*t))
        .unwrap_or_else(|| panic!("no page was started from the phasing signal: {:?}", ev.len()));
    // The phasing signal begins 1.5 s into the piece. Three seconds of it are
    // needed, so anything much later means it was found by luck, not by shape.
    assert!((1.5..=10.0).contains(&start), "started at {start:.1} s into the piece");

    assert!(ev.iter().any(|(_, e)| matches!(e, WefaxEvent::Phased { .. })));
    // The rest of the phasing signal is not chart: the page's first rows must
    // not be the black bars it is made of. A phasing line averages a tenth of
    // white; the chart's top margin is white.
    let first = ev
        .iter()
        .find_map(|(_, e)| match e {
            WefaxEvent::Line { gray, .. } => Some(gray),
            _ => None,
        })
        .expect("no line came out");
    let mean = first.iter().map(|&g| f64::from(g)).sum::<f64>() / first.len() as f64 / 255.0;
    assert!(mean > 0.3, "the page opens on the phasing signal (mean brightness {mean:.2})");
    // And the chart does come out: twelve seconds of it is some twenty lines.
    assert!(lines(&ev) >= 15, "only {} lines", lines(&ev));
    assert!(
        !ev.iter().any(|(_, e)| matches!(e, WefaxEvent::Complete { .. })),
        "the page ended inside the chart"
    );
}

#[test]
fn noise_at_the_stop_tones_rate_does_not_end_the_page() {
    // The page has to be under way first: started by hand, on the phasing
    // signal that opens the recording's chart, so the noise arrives mid-picture.
    let mut rx = receiver();
    let mut out = Vec::new();
    rx.start_manual(&mut out);
    let opening = run(&mut rx, &piece("start"));
    assert!(opening.iter().any(|(_, e)| matches!(e, WefaxEvent::Phased { .. })));
    let ev = run(&mut rx, &piece("noise_at_the_stop_rate"));
    let complete = ev.iter().find(|(_, e)| matches!(e, WefaxEvent::Complete { .. }));
    assert!(complete.is_none(), "a noisy chart was taken for a stop tone: {complete:?}");
    assert!(lines(&ev) >= 12, "only {} lines", lines(&ev));
}

#[test]
fn the_real_stop_tone_still_ends_the_page() {
    let mut rx = receiver();
    let mut out = Vec::new();
    rx.start_manual(&mut out);
    let ev = run(&mut rx, &piece("stop"));
    let (at, by_tone) = ev
        .iter()
        .find_map(|(t, e)| match e {
            WefaxEvent::Complete { by_tone, .. } => Some((*t, *by_tone)),
            _ => None,
        })
        .expect("the stop tone did not end the page");
    assert!(by_tone);
    // The tone runs from 10.0 s to 14.7 s into the piece; seven half-second
    // windows of it end the page.
    assert!((12.0..=15.5).contains(&at), "ended at {at:.1} s into the piece");
}
