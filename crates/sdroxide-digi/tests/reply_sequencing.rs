//! A reply goes out in the slot *opposite* the station being answered.
//!
//! Driven through the real controller with real audio: a DX calls CQ into the
//! receiver, the operator presses REPLY once the decode is in, and the test
//! watches which slots the burst actually keys in. Nothing here reads the
//! controller's internals — the question the operator asks is "did I transmit
//! on top of them", and that is what this measures.
//!
//! Issue #191.

use std::time::{Duration, Instant, UNIX_EPOCH};

use sdroxide_digi::{DigiAction, DigiController, Ft8Modem};
use sdroxide_types::{DigiConfig, Mode};

const TICK_S: f64 = 0.05;
const RX_RATE: f64 = 12_000.0;
const TX_RATE: f64 = 48_000.0;
const DIAL_HZ: f64 = 14_074_000.0;
/// 2021-01-01 00:00:00 UTC — a whole minute, so it starts every mode's slot 0.
const BASE_UNIX: f64 = 1_609_459_200.0;

/// One run of the simulator.
struct Scene {
    mode: Mode,
    /// Which period the station being answered keeps.
    dx_even: bool,
    /// How many overs the DX sends (one per period of theirs, from slot 0).
    dx_overs: i64,
    /// A third station transmitting in *every* slot. It is what makes the
    /// controller prune what it remembers, so a scene testing how long a
    /// station's period is remembered needs one.
    filler: bool,
    /// Press REPLY this many slots in, `click_into` seconds past the boundary.
    click_slot: i64,
    click_into: f64,
    /// How long to run, in slots.
    slots: i64,
}

fn cfg() -> DigiConfig {
    DigiConfig {
        my_call: "AB1CD".into(),
        my_grid: "FN42".into(),
        tx_even: true,
        ..Default::default()
    }
}

/// Play `scene` past a real controller and report the slot indices it keyed in.
fn keyed_slots(scene: &Scene) -> Vec<i64> {
    let t = scene.mode.slot_timing().expect("a slotted mode");
    let (slot_s, tx_off) = (t.slot_s, t.tx_offset_s);
    let modem = Ft8Modem::new(scene.mode);
    let burst = |msg: &str, hz: f32| modem.encode_burst_12k(msg, hz, 0.5).expect("packs").0;
    let dx_burst = burst("CQ W9XYZ EM48", 1200.0);
    let filler_burst = burst("CQ K1FIL FN31", 2000.0);

    let mut ctl = DigiController::new(scene.mode, cfg(), RX_RATE);
    // Start on a slot boundary of the DX's own parity, so their first over is in
    // the period they keep.
    let base = (BASE_UNIX / slot_s) as i64;
    let first = base + i64::from((base % 2 == 0) != scene.dx_even);
    let start = first as f64 * slot_s;
    let click_at = start + scene.click_slot as f64 * slot_s + scene.click_into;

    let mut rx = vec![0.0f32; (RX_RATE * TICK_S) as usize];
    let mut tx = vec![0.0f32; (TX_RATE * TICK_S) as usize];
    let mut clicked = false;
    let mut heard_dx = false;
    let mut keyed = Vec::new();
    // The slot in progress, for spotting a boundary.
    let mut slot = first - 1;
    // Which slots the DX transmits in. The decode wait below hangs off this:
    // those are the slots whose decode the sequencer has to have acted on
    // before REPLY is pressed, and waiting on the others costs a full decode
    // of a quiet slot for nothing.
    let dx_in =
        |i: i64| (i % 2 == 0) == scene.dx_even && i - first < scene.dx_overs * 2;

    let mut t = start;
    let end = start + scene.slots as f64 * slot_s;
    while t < end {
        let idx = (t / slot_s).floor() as i64;
        let now = UNIX_EPOCH + Duration::from_secs_f64(t);
        let dx_on = dx_in(idx);

        if ctl.tx_burst_active() {
            // Our own over: the receiver hears nothing while we transmit.
            if ctl.fill_tx_block(&mut tx) {
                ctl.on_burst_done();
            }
        } else {
            // Each station's over, positioned in the slot exactly as they key it.
            rx.fill(0.0);
            let from = ((t - (idx as f64 * slot_s + tx_off)) * RX_RATE).round() as isize;
            let mut mix = |src: &[f32]| {
                for (i, s) in rx.iter_mut().enumerate() {
                    let j = from + i as isize;
                    if j >= 0 && (j as usize) < src.len() {
                        *s += src[j as usize];
                    }
                }
            };
            if dx_on {
                mix(&dx_burst);
            }
            if scene.filler {
                mix(&filler_burst);
            }
            ctl.on_rx_audio(&rx);
        }

        let mut acted = ctl.poll(now, DIAL_HZ);
        // On the first tick of a slot, that poll handed the slot that just
        // ended to the decode worker. Where that slot was one the DX
        // transmitted in, wait for the worker to answer for it, so the
        // sequencer acts on the decode inside the following slot — which is
        // where a real one lands, and is the only point at which the operator
        // can press REPLY at all.
        //
        // Asked of the controller rather than inferred from what it returns.
        // A slot that decodes to nothing produces no `Decodes` action at all,
        // so "has a decode arrived?" cannot answer "has the decoder finished?":
        // waiting on the actions stalls for ever on every quiet slot, and
        // giving up on a timer instead means the wait for a slot that *is*
        // about to decode expires under load — which is what made this test
        // fail perhaps half the time in a full `cargo test` and never once run
        // on its own. `DigiController::decoding` is the question that was
        // actually being asked.
        //
        // The deadline is a guard against hanging and nothing else: by this
        // point the worker has a slot in hand and is going to answer for it.
        if idx != slot {
            if dx_in(slot) {
                let give_up = Instant::now() + Duration::from_secs(60);
                while ctl.decoding() > 0 && Instant::now() < give_up {
                    std::thread::sleep(Duration::from_millis(5));
                    acted.extend(ctl.poll(now, DIAL_HZ));
                }
            }
            slot = idx;
        }
        keyed.extend(acted.iter().filter(|a| matches!(a, DigiAction::KeyTx)).map(|_| idx));
        heard_dx |= acted.iter().any(|a| match a {
            DigiAction::Decodes(d) => d.iter().any(|x| x.from.as_deref() == Some("W9XYZ")),
            _ => false,
        });

        // Answering only makes sense once the decode is on screen, which is
        // what the wait above has just guaranteed.
        if !clicked && t >= click_at {
            assert!(heard_dx, "the DX never decoded, so REPLY could not have been pressed");
            clicked = true;
            ctl.start_qso("W9XYZ".into(), Some("EM48".into()), -10, 1200.0, false);
        }
        t += TICK_S;
    }
    assert!(clicked, "the scene never got as far as pressing REPLY");
    keyed
}

/// Every burst went out in a period the DX does not transmit in.
fn check(scene: &Scene) {
    let keyed = keyed_slots(scene);
    let (mode, dx_even) = (scene.mode, scene.dx_even);
    assert!(!keyed.is_empty(), "{mode:?} dx_even={dx_even}: never transmitted at all");
    for idx in &keyed {
        assert_ne!(
            idx % 2 == 0,
            dx_even,
            "{mode:?} dx_even={dx_even}: keyed in the DX's own slot {idx} \
             (all keyed slots: {keyed:?})"
        );
    }
}

/// The plain case: answer a station calling CQ, from the slot after the one they
/// called in — where an operator watching the decode list presses REPLY.
///
/// FT4 and FT2 are the ones this used to get wrong. Their odd slots begin half a
/// second (FT4) or a quarter (FT2) past a whole second, and the slot a station
/// was heard in was carried around as whole seconds — so an odd slot read back
/// as the even one before it, the parity came out inverted, and the reply went
/// out on top of the station being answered.
#[test]
fn a_reply_never_lands_in_the_dx_slot() {
    for mode in [Mode::Ft8, Mode::Ft4, Mode::Ft2] {
        let slot_s = mode.slot_timing().unwrap().slot_s;
        for dx_even in [true, false] {
            check(&Scene {
                mode,
                dx_even,
                dx_overs: 4,
                filler: false,
                click_slot: 1,
                click_into: slot_s * 0.1,
                slots: 8,
            });
        }
    }
}

/// The same, for a station heard a while back rather than in the slot just gone:
/// the operator scrolls the decode list and answers a row from a few minutes
/// ago, on a band busy enough that the controller has been pruning all along.
///
/// The reply is pressed in a slot of the DX's *own* parity, so a controller that
/// has forgotten them and falls back to "the period we are in now" transmits
/// straight over the top of them.
#[test]
fn a_reply_to_an_older_decode_still_lands_opposite() {
    check(&Scene {
        mode: Mode::Ft8,
        dx_even: true,
        dx_overs: 1,
        filler: true,
        click_slot: 12,
        click_into: 1.0,
        slots: 16,
    });
}
