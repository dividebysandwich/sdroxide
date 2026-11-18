//! A real Beast stream, from `readsb` on a Raspberry Pi, through the parser and
//! the aircraft table. The recording is not in the tree; point
//! `SDROXIDE_BEAST_REC` at one (`nc <pi> 30005 > rec.bin`) to run this.

use sdroxide_adsb::beast::{Parser, signal_to_dbfs};
use sdroxide_adsb::{Accepted, Rejected, Tracker, accept};
use sdroxide_types::AdsbSettings;

#[test]
fn a_recorded_readsb_stream_parses_and_fills_the_table() {
    let Ok(path) = std::env::var("SDROXIDE_BEAST_REC") else {
        eprintln!("SDROXIDE_BEAST_REC not set; skipped");
        return;
    };
    let data = std::fs::read(&path).expect("recording");
    let mut p = Parser::new();
    // In the pieces a socket would hand over, to exercise the joins.
    let frames: Vec<_> = data.chunks(1460).flat_map(|c| p.push(c)).collect();

    let mut by_len = [0usize; 15];
    let mut with_id = 0;
    for f in &frames {
        by_len[f.bytes.len()] += 1;
        with_id += usize::from(f.receiver.is_some());
    }
    let mut t = Tracker::new(AdsbSettings::default());
    let (mut ok, mut matched, mut repaired, mut rejected) = (0, 0, 0, 0);
    let mut why = std::collections::BTreeMap::<(u8, String), usize>::new();
    let now = 1_763_000_000;
    let ts0 = frames.first().map_or(0, |f| f.timestamp);
    for f in frames.iter().filter(|f| f.bytes.len() != 2) {
        match accept(&f.bytes, |i| t.knows(i, now)) {
            Ok(a) => {
                match a {
                    Accepted::Verified(_) => ok += 1,
                    Accepted::Matched(_) => matched += 1,
                    Accepted::Repaired { .. } => repaired += 1,
                }
                let mono = f.timestamp.wrapping_sub(ts0) as f64 / 12e6;
                t.absorb(&a, &f.bytes, signal_to_dbfs(f.signal).unwrap_or(-48.1), now, mono);
            }
            Err(r) => {
                rejected += 1;
                *why.entry((f.bytes[0] >> 3, format!("{r:?}"))).or_default() += 1;
                let _: Rejected = r;
            }
        }
    }
    let ac = t.snapshot();
    let placed = ac.iter().filter(|a| a.lat.is_some()).count();
    eprintln!(
        "{} bytes · {} frames (A/C {}, short {}, long {}) · skipped {} bytes · with receiver id {} \
         · verified {ok} matched {matched} repaired {repaired} rejected {rejected} \
         · {} aircraft, {placed} with a position · span {:.0} s by the 12 MHz clock",
        data.len(),
        frames.len(),
        by_len[2],
        by_len[7],
        by_len[14],
        p.skipped,
        with_id,
        ac.len(),
        frames.last().map_or(0, |f| f.timestamp.wrapping_sub(ts0)) as f64 / 12e6,
    );
    eprintln!("rejected by downlink format and reason: {why:?}");
    assert!(frames.len() > 1000);
    assert!(p.skipped < 64, "only a partial frame at the very start may be skipped");
    // Every extended squitter `readsb` forwards passes our own check; what we
    // turn down is all-calls answering a radar with a non-zero interrogator
    // code, and replies for aircraft not yet heard — this decoder's policy for
    // its own receiver too, applied unchanged to a Beast feed.
    for ((df, r), n) in &why {
        assert!(
            (*df == 11 && r == "BadCrc") || r == "Unmatched",
            "{n} × DF{df} {r}: readsb forwards checked frames"
        );
    }
    let _ = rejected;
    assert!(placed > 0);
}
