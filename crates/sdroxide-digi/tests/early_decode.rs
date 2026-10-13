//! FT8 decodes a slot early, before it has finished arriving.
//!
//! Driven through the real controller with real audio, the way
//! `reply_sequencing.rs` is: a station transmits into the receiver, and the
//! test watches when its decode comes back and what the sequencer sends next.
//!
//! Issue #667: on a slow machine the decode list stayed empty well past the
//! slot boundary, and the reply keyed half a second after it — so the
//! sequencer, not yet having seen the answer, sent the last message again.

use std::time::{Duration, Instant, UNIX_EPOCH};

use sdroxide_digi::{DigiAction, DigiController, Ft8Modem};
use sdroxide_types::{DigiConfig, Mode};

const TICK_S: f64 = 0.05;
const RX_RATE: f64 = 12_000.0;
const TX_RATE: f64 = 48_000.0;
const DIAL_HZ: f64 = 14_074_000.0;
const SLOT_S: f64 = 15.0;
const TX_OFFSET_S: f64 = 0.5;
/// 2021-01-01 00:00:00 UTC — a whole minute, so an even FT8 slot.
const BASE_UNIX: f64 = 1_609_459_200.0;

fn cfg() -> DigiConfig {
    DigiConfig {
        my_call: "AB1CD".into(),
        my_grid: "FN42".into(),
        tx_even: true,
        ..Default::default()
    }
}

/// Wait for the decode worker to answer for everything it has been handed,
/// folding in what comes back. Simulated time stands still meanwhile, so this
/// is a decoder that takes no time at all.
fn wait_for_decoder(ctl: &mut DigiController, t: f64) -> Vec<DigiAction> {
    let now = UNIX_EPOCH + Duration::from_secs_f64(t);
    let mut acted = Vec::new();
    let give_up = Instant::now() + Duration::from_secs(60);
    while ctl.decoding() > 0 && Instant::now() < give_up {
        std::thread::sleep(Duration::from_millis(5));
        acted.extend(ctl.poll(now, DIAL_HZ));
    }
    acted
}

/// Run `slots` FT8 slots from the even slot at [`BASE_UNIX`]. `dx` is what
/// the other station sends, in which slot (counted from 0). `wait` says, from
/// the seconds into a slot, whether to let the decoder finish there before
/// simulated time moves on. Calls `each` with the seconds since the start and
/// every action of that tick.
fn run(
    ctl: &mut DigiController,
    slots: i64,
    dx: (&str, i64),
    wait: impl Fn(f64) -> bool,
    mut each: impl FnMut(&mut DigiController, f64, &[DigiAction]),
) {
    let modem = Ft8Modem::new(Mode::Ft8);
    let dx_burst = modem.encode_burst_12k(dx.0, 1200.0, 0.5).expect("packs").0;
    let start = BASE_UNIX;
    let mut rx = vec![0.0f32; (RX_RATE * TICK_S) as usize];
    let mut tx = vec![0.0f32; (TX_RATE * TICK_S) as usize];
    // Integer ticks, so the boundaries land exactly where they should.
    let ticks_per_slot = (SLOT_S / TICK_S).round() as i64;
    for tick in 0..slots * ticks_per_slot {
        let since = tick as f64 * TICK_S;
        let t = start + since;
        let slot = tick / ticks_per_slot;
        let into = since - slot as f64 * SLOT_S;
        if ctl.tx_burst_active() {
            // Our own over: the receiver hears nothing while we transmit.
            if ctl.fill_tx_block(&mut tx) {
                ctl.on_burst_done();
            }
        } else {
            rx.fill(0.0);
            if slot == dx.1 {
                let from = ((into - TX_OFFSET_S) * RX_RATE).round() as isize;
                for (i, s) in rx.iter_mut().enumerate() {
                    let j = from + i as isize;
                    if j >= 0 && (j as usize) < dx_burst.len() {
                        *s += dx_burst[j as usize];
                    }
                }
            }
            ctl.on_rx_audio(&rx);
        }
        let mut acted = ctl.poll(UNIX_EPOCH + Duration::from_secs_f64(t), DIAL_HZ);
        if wait(into) {
            acted.extend(wait_for_decoder(ctl, t));
        }
        each(ctl, since, &acted);
    }
}

fn decoded(acted: &[DigiAction], msg: &str) -> usize {
    acted
        .iter()
        .map(|a| match a {
            DigiAction::Decodes(d) => d.iter().filter(|x| x.message == msg).count(),
            _ => 0,
        })
        .sum()
}

/// The decode of a slot is on screen before the slot ends, and the boundary
/// decode of the same slot does not put it there a second time.
#[test]
fn a_slot_decodes_before_it_ends_and_only_once() {
    let mut ctl = DigiController::new(Mode::Ft8, cfg(), RX_RATE);
    let mut seen = Vec::new();
    // Slot 1, odd, so it is not the period this station would call CQ in —
    // not that it calls; nothing here transmits.
    run(
        &mut ctl,
        3,
        ("CQ W9XYZ EM48", 1),
        |_| true,
        |_, since, acted| {
            for _ in 0..decoded(acted, "CQ W9XYZ EM48") {
                seen.push(since);
            }
        },
    );
    assert_eq!(seen.len(), 1, "reported exactly once, got it at {seen:?} s");
    let into = seen[0] - SLOT_S;
    assert!(
        (13.5..SLOT_S).contains(&into),
        "decoded {into:.2} s into its slot — before the boundary, not after"
    );
}

/// The answer to our CQ is acted on in our next turn even when the boundary
/// decode of the slot it was in has not finished by the time that turn keys.
///
/// The test lets the decoder finish only while a slot is still in progress —
/// the early decode — and never at the boundary, so the half second up to our
/// key-up runs in no real time at all and the boundary decode is still in the
/// worker when it comes. Without the early decode the sequencer has not seen
/// the answer and calls CQ again.
#[test]
fn the_reply_is_ready_when_our_turn_comes() {
    let mut ctl = DigiController::new(Mode::Ft8, cfg(), RX_RATE);
    ctl.call_cq();
    let mut sent = Vec::new();
    run(
        &mut ctl,
        3,
        ("AB1CD W9XYZ EM48", 1),
        |into| (13.0..SLOT_S).contains(&into),
        |ctl, since, acted| {
            if acted.iter().any(|a| matches!(a, DigiAction::KeyTx)) {
                let line = ctl.status().transcript.iter().rev().find(|l| l.tx).cloned();
                sent.push((since, line.map(|l| l.text).unwrap_or_default()));
            }
        },
    );
    assert_eq!(sent.len(), 2, "keyed in slots 0 and 2: {sent:?}");
    assert!(sent[0].1.starts_with("CQ AB1CD"), "slot 0 calls CQ: {sent:?}");
    let (at, msg) = &sent[1];
    assert!(
        (2.0 * SLOT_S..2.0 * SLOT_S + 1.0).contains(at),
        "the reply keys on time in slot 2: {sent:?}"
    );
    assert!(msg.starts_with("W9XYZ AB1CD"), "slot 2 answers W9XYZ, not CQ again: {sent:?}");
}
