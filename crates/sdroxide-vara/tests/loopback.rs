//! Two modems on one simulated channel: connect, move a payload, disconnect.
//!
//! Each modem is driven by an in-process host, exactly as the Winlink window
//! drives it, so this exercises the host mux, the TNC port and the station
//! glue together. The channel is half-duplex air: whatever one station puts out
//! reaches the other after a fixed delay, with a little noise on top.

use std::collections::VecDeque;
use std::time::Duration;

use sdroxide_vara::{Bandwidth, LocalClient, Modem, Vara, new_modem};

const BLOCK: usize = 960; // 20 ms
const DELAY: usize = 2_400; // 50 ms of propagation and radio turnaround

struct Air {
    q: VecDeque<f64>,
    seed: u64,
}

impl Air {
    fn new(seed: u64) -> Self {
        Self { q: std::iter::repeat_n(0.0, DELAY).collect(), seed }
    }

    fn noise(&mut self) -> f64 {
        // xorshift; ~-40 dBFS of white noise so the receiver is never handed
        // pure digital silence.
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        ((self.seed >> 11) as f64 / (1u64 << 53) as f64 - 0.5) * 0.02
    }

    fn pass(&mut self, tx: &[f64], rx: &mut [f64]) {
        self.q.extend(tx.iter().copied());
        for r in rx.iter_mut() {
            let v = self.q.pop_front().unwrap_or(0.0);
            *r = v + self.noise();
        }
    }
}

struct Pair {
    a: Vara,
    b: Vara,
    ab: Air,
    ba: Air,
    a_out: Vec<f64>,
    b_out: Vec<f64>,
}

impl Pair {
    fn new(bw: Bandwidth, registered: bool) -> Self {
        Self {
            a: new_modem(bw, "OE1AAA", registered, 1),
            b: new_modem(bw, "OE1BBB", registered, 2),
            ab: Air::new(0x1234_5678),
            ba: Air::new(0x8765_4321),
            a_out: vec![0.0; BLOCK],
            b_out: vec![0.0; BLOCK],
        }
    }

    /// One 20 ms tick of both stations.
    fn step(&mut self) {
        let mut a_in = vec![0.0; BLOCK];
        let mut b_in = vec![0.0; BLOCK];
        self.ba.pass(&self.b_out, &mut a_in);
        self.ab.pass(&self.a_out, &mut b_in);
        self.a.process(&a_in, &mut self.a_out);
        self.b.process(&b_in, &mut self.b_out);
    }

    fn run_until(&mut self, secs: f64, mut done: impl FnMut(&mut Self) -> bool) -> bool {
        let steps = (secs * 50.0) as usize;
        for _ in 0..steps {
            self.step();
            if done(self) {
                return true;
            }
        }
        false
    }
}

fn lines(c: &LocalClient) -> Vec<String> {
    let mut v = Vec::new();
    while let Some(l) = c.try_recv_line() {
        v.push(l);
    }
    v
}

fn drain_data(c: &LocalClient, into: &mut Vec<u8>) {
    while let Ok(d) = c.try_recv_data() {
        into.extend(d);
    }
}

fn session(bw: Bandwidth, registered: bool, payload: usize) -> (Vec<String>, Vec<String>) {
    let mut p = Pair::new(bw, registered);
    let ha = p.a.host().handle().claim("test A").unwrap();
    let hb = p.b.host().handle().claim("test B").unwrap();
    ha.command("MYCALL OE1AAA");
    hb.command("MYCALL OE1BBB");
    hb.command("LISTEN ON");
    p.run_until(0.2, |_| false);
    ha.command("CONNECT OE1AAA OE1BBB");

    let (mut la, mut lb) = (Vec::new(), Vec::new());
    let ok = p.run_until(60.0, |_| {
        la.extend(lines(&ha));
        lb.extend(lines(&hb));
        la.iter().any(|l| l.starts_with("CONNECTED"))
            && lb.iter().any(|l| l.starts_with("CONNECTED"))
    });
    assert!(ok, "no link: A {la:?} B {lb:?}");

    let msg: Vec<u8> =
        (0..payload).map(|i| b"The quick brown fox jumps over the lazy dog. "[i % 45]).collect();
    ha.send_data(&msg);
    let mut got = Vec::new();
    let ok = p.run_until(600.0, |_| {
        la.extend(lines(&ha));
        lb.extend(lines(&hb));
        drain_data(&hb, &mut got);
        got.len() >= msg.len()
    });
    assert!(ok, "A→B moved {} of {} bytes", got.len(), msg.len());
    assert_eq!(got, msg);
    eprintln!(
        "{bw:?} reg={registered}: {} bytes by t={:.1}s, max level {}",
        msg.len(),
        p.a.station().now() as f64 / 48000.0,
        max_level(&la)
    );

    let back = b"73 de OE1BBB\r".to_vec();
    hb.send_data(&back);
    let mut got = Vec::new();
    let ok = p.run_until(120.0, |_| {
        la.extend(lines(&ha));
        lb.extend(lines(&hb));
        drain_data(&ha, &mut got);
        got.len() >= back.len()
    });
    assert!(ok, "B→A moved {} of {} bytes; A {la:?}", got.len(), back.len());
    assert_eq!(got, back);

    ha.command("DISCONNECT");
    let ok = p.run_until(60.0, |_| {
        la.extend(lines(&ha));
        lb.extend(lines(&hb));
        la.iter().any(|l| l == "DISCONNECTED") && lb.iter().any(|l| l == "DISCONNECTED")
    });
    assert!(ok, "no clean disconnect: A {la:?} B {lb:?}");
    (la, lb)
}

fn max_level(lines: &[String]) -> usize {
    lines
        .iter()
        .filter_map(|l| l.strip_prefix("BITRATE ("))
        .filter_map(|r| r.split(')').next()?.parse().ok())
        .max()
        .unwrap_or(0)
}

#[test]
fn a_2300_link_moves_data_both_ways_and_hangs_up() {
    let (la, _) = session(Bandwidth::Hz2300, false, 5_000);
    assert!(la.iter().any(|l| l.starts_with("CONNECTED OE1AAA OE1BBB 2300")), "{la:?}");
    // Unregistered: the speed ladder stops at level 4, as VARA's own does.
    assert!(max_level(&la) <= 4, "{la:?}");
}

#[test]
fn a_500_link_moves_data() {
    let (la, _) = session(Bandwidth::Hz500, false, 1_000);
    assert!(la.iter().any(|l| l.starts_with("CONNECTED OE1AAA OE1BBB 500")), "{la:?}");
}

#[test]
fn registered_climbs_past_the_unregistered_cap() {
    let (la, _) = session(Bandwidth::Hz2300, true, 20_000);
    assert!(max_level(&la) > 4, "registered never left level 4: {la:?}");
}

#[test]
fn version_and_bad_commands_are_answered() {
    let mut m = new_modem(Bandwidth::Hz2300, "OE1AAA", false, 1);
    let c = m.host().handle().claim("test").unwrap();
    c.command("VERSION");
    c.command("FROBNICATE");
    let mut out = vec![0.0; BLOCK];
    m.process(&[0.0; BLOCK], &mut out);
    let mut got = Vec::new();
    while let Ok(l) = c.recv_line(Duration::from_millis(10)) {
        got.push(l);
    }
    assert!(got.iter().any(|l| l.starts_with("VERSION VARA HF")), "{got:?}");
    assert!(got.iter().any(|l| l == "WRONG"), "{got:?}");
}

#[test]
fn one_host_at_a_time() {
    let m = new_modem(Bandwidth::Hz2300, "OE1AAA", false, 1);
    let h = m.host().handle();
    let first = h.claim("Winlink").unwrap();
    let err = h.claim("panel").err().unwrap();
    assert!(err.contains("Winlink"), "{err}");
    drop(first);
    assert!(h.claim("panel").is_ok());
    drop(m);
    assert!(h.claim("late").is_err(), "a modem that is gone must refuse");
}

/// The figure `VaraController::tx_peak` declares ([`sdroxide_vara::TX_PEAK`]).
/// Measured over a whole session at upstream's default drive (−5 dB) — the
/// call, the acknowledgements and data at every level the link climbed through
/// — rather than assumed. At 0 dB the OFDM peaks reach the clamp and clip,
/// which is why upstream defaults below it.
#[test]
fn transmit_peak_at_default_drive() {
    let mut p = Pair::new(Bandwidth::Hz2300, true);
    let ha = p.a.host().handle().claim("A").unwrap();
    let hb = p.b.host().handle().claim("B").unwrap();
    hb.command("LISTEN ON");
    ha.command("CONNECT OE1AAA OE1BBB");
    ha.send_data(&vec![b'x'; 8000]);
    let (mut peak, mut sum, mut n) = (0f64, 0f64, 0usize);
    p.run_until(60.0, |p| {
        for &v in p.a_out.iter().chain(p.b_out.iter()) {
            if v != 0.0 {
                peak = peak.max(v.abs());
                sum += v * v;
                n += 1;
            }
        }
        false
    });
    let rms = (sum / n.max(1) as f64).sqrt();
    eprintln!("peak {peak:.3} rms {rms:.3} over {n} keyed samples");
    assert!(n > 0);
    assert!(
        peak <= sdroxide_vara::TX_PEAK as f64,
        "peak {peak} above the declared {}",
        sdroxide_vara::TX_PEAK
    );
    assert!(
        peak > sdroxide_vara::TX_PEAK as f64 * 0.9,
        "declared peak is {} but the signal only reaches {peak}",
        sdroxide_vara::TX_PEAK
    );
    let _ = (&ha, &hb);
}
