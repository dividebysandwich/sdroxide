//! Print what an RC-28 does, and light the LED over whichever button is held.
//!
//! `cargo run -p sdroxide-rc28 --example rc28 [seconds]`
//!
//! For settling a platform backend from one operator's report: if the knob
//! counts here and the buttons light up, the app will work too.

use std::time::{Duration, Instant};

use sdroxide_rc28::Rc28Event;
use sdroxide_rc28::proto::Input;

fn main() {
    let secs: u64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(30);
    let mut h = sdroxide_rc28::spawn(true, || {});
    // A worker leaves the LEDs alone until told what to show; LINK comes on
    // with this.
    h.set_leds(0);
    let end = Instant::now() + Duration::from_secs(secs);
    let (mut total, mut held) = (0i64, 0u8);
    let mut last_status = None;
    while Instant::now() < end {
        for e in h.poll() {
            match e {
                Rc28Event::Input(Input::Turn(n)) => {
                    total += i64::from(n);
                    println!("turn {n:+4}   total {total:+}");
                }
                Rc28Event::Input(Input::Key { key, down }) => {
                    println!("{} {}", key.label(), if down { "down" } else { "up" });
                    held = if down { held | key.bit() } else { held & !key.bit() };
                    h.set_leds(held);
                }
                other => println!("{other:?}"),
            }
        }
        let s = h.status();
        if last_status.as_ref() != Some(&s) {
            println!("status: {s:?}");
            last_status = Some(s);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    println!("total steps {total:+}");
}
