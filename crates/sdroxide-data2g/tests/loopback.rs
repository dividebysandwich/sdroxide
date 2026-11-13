//! Two Data2G stations back to back through a noisy channel: one calls, both
//! send a payload, and every byte arrives intact.

use sdroxide_data2g::Data2g;
use sdroxide_tnc::{Reply, TncModem};

/// A small deterministic noise source, so a failure reproduces.
struct Noise(u64);

impl Noise {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        ((self.0 >> 40) as f32 / (1u64 << 24) as f32) - 0.5
    }
}

fn payload(seed: u8, n: usize) -> Vec<u8> {
    (0..n).map(|i| (i as u8).wrapping_mul(31).wrapping_add(seed)).collect()
}

#[test]
fn two_stations_connect_and_exchange_payload() {
    let mut a = Data2g::new("OE1AAA", Some(1), 0.05).unwrap();
    let mut b = Data2g::new("OE3BBB", Some(2), 0.05).unwrap();
    for (m, call) in [(&mut a, "OE1AAA"), (&mut b, "OE3BBB")] {
        m.command(&format!("MYCALL {call}"));
        m.command("BW2300");
    }
    b.command("LISTEN ON");
    a.command("LISTEN OFF");
    let (lines, _) = a.take_output();
    assert!(lines.iter().any(|l| l == "OK"), "{lines:?}");
    b.take_output();

    a.command("CONNECT OE1AAA OE3BBB");
    let a_sends = payload(1, 4096);
    let b_sends = payload(7, 4096);

    let block = 800; // 100 ms, as upstream's station loop
    let (mut a_tx, mut b_tx) = (vec![0f32; block], vec![0f32; block]);
    let (mut a_rx, mut b_rx) = (vec![0f32; block], vec![0f32; block]);
    let mut noise = Noise(0x9e37_79b9_7f4a_7c15);
    let (mut a_got, mut b_got) = (Vec::new(), Vec::new());
    let (mut a_up, mut b_up, mut sent) = (false, false, false);
    let mut a_log = Vec::new();

    // At most ten minutes of simulated air.
    let mut blocks = 0;
    for _ in 0..6000 {
        blocks += 1;
        // Each hears the other's last block, plus noise ~25 dB down.
        for i in 0..block {
            a_rx[i] = 0.3 * b_tx[i] + 0.01 * noise.next();
            b_rx[i] = 0.3 * a_tx[i] + 0.01 * noise.next();
        }
        a.process(&a_rx, &mut a_tx);
        b.process(&b_rx, &mut b_tx);
        let (al, ad) = a.take_output();
        let (bl, bd) = b.take_output();
        a_got.extend(ad);
        b_got.extend(bd);
        for l in &al {
            if matches!(Reply::parse(l), Reply::Connected { .. }) {
                a_up = true;
            }
        }
        a_log.extend(al);
        for l in &bl {
            if matches!(Reply::parse(l), Reply::Connected { .. }) {
                b_up = true;
            }
        }
        if a_up && b_up && !sent {
            a.data(&a_sends);
            b.data(&b_sends);
            sent = true;
        }
        if a_got.len() >= b_sends.len() && b_got.len() >= a_sends.len() {
            break;
        }
    }
    eprintln!("{} s of air; caller saw {a_log:?}", blocks as f64 * block as f64 / 8000.0);
    assert!(a_up && b_up, "never connected; caller saw {a_log:?}, state {}", a.state());
    assert_eq!(b_got, a_sends, "callee received");
    assert_eq!(a_got, b_sends, "caller received");
}

#[test]
fn version_and_protocol_are_the_pinned_release() {
    assert_eq!(sdroxide_data2g::version(), "0.4.3");
    assert_eq!(sdroxide_data2g::protocol(), 15);
    let mut m = Data2g::new("N0CALL", Some(3), 0.0).unwrap();
    m.command("VERSION");
    let (lines, _) = m.take_output();
    assert!(lines.iter().any(|l| l.starts_with("VERSION Data2G 0.4.3")), "{lines:?}");
}
