//! `TncTransport` against a second Data2G station on a simulated channel: the
//! call comes up, bytes go both ways through `Read`/`Write`, and dropping the
//! transport hangs up cleanly at the far end.

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use sdroxide_data2g::Data2g;
use sdroxide_tnc::{HostMux, Input, TncModem};
use sdroxide_winlink::transport::{TncTransport, Transport};

const BLOCK: usize = 160;

/// One station: a modem behind its host mux, as the controller pairs them.
struct Station {
    mux: HostMux,
    modem: Data2g,
    out: Vec<f32>,
}

impl Station {
    fn new(call: &str, seed: u64) -> Self {
        Self {
            mux: HostMux::new("Data2G"),
            modem: Data2g::new(call, Some(seed), 0.05).unwrap(),
            out: vec![0.0; BLOCK],
        }
    }

    fn step(&mut self, rx: &[f32]) {
        self.modem.process(rx, &mut self.out);
        while let Some(i) = self.mux.try_recv() {
            match i {
                Input::Command(l) => self.modem.command(&l),
                Input::Data(d) => self.modem.data(&d),
                Input::Closed => self.modem.client_gone(),
                Input::Connected => {}
            }
        }
        let (lines, data) = self.modem.take_output();
        for l in lines {
            self.mux.line(&l);
        }
        if !data.is_empty() {
            self.mux.data(&data);
        }
    }
}

#[test]
fn a_tnc_transport_carries_bytes_and_hangs_up() {
    let mut a = Station::new("OE1AAA", 1);
    let mut b = Station::new("OE1GW", 2);
    let handle = a.mux.handle();
    let gw = b.mux.handle().claim("gateway").unwrap();
    gw.command("MYCALL OE1GW");
    gw.command("LISTEN ON");

    // The air, on its own thread, a little faster than real time.
    let stop = Arc::new(AtomicBool::new(false));
    let pump = {
        let stop = stop.clone();
        std::thread::spawn(move || {
            let (mut ab, mut ba) = (VecDeque::<f32>::new(), VecDeque::<f32>::new());
            while !stop.load(Ordering::SeqCst) {
                ab.extend(a.out.iter().copied());
                ba.extend(b.out.iter().copied());
                let a_in: Vec<f32> =
                    (0..BLOCK).map(|_| ba.pop_front().unwrap_or(0.0) * 0.5).collect();
                let b_in: Vec<f32> =
                    (0..BLOCK).map(|_| ab.pop_front().unwrap_or(0.0) * 0.5).collect();
                a.step(&a_in);
                b.step(&b_in);
                std::thread::sleep(Duration::from_millis(2));
            }
        })
    };

    let cancel = Arc::new(AtomicBool::new(false));
    let mut t = TncTransport::connect(
        &handle,
        "Data2G",
        "OE1AAA",
        "OE1GW",
        "BW2300",
        Duration::from_secs(60),
        cancel,
    )
    .expect("the gateway should answer");
    assert_eq!(t.target_call(), "OE1GW");
    assert_eq!(t.describe(), "Data2G to OE1GW");
    assert!(handle.claim("panel").is_err(), "a running session must hold the modem");

    t.write_all(b"[sdroxide-1.0-B2FHM$]\r").unwrap();
    let mut got = Vec::new();
    let until = Instant::now() + Duration::from_secs(60);
    while !got.ends_with(b"\r") && Instant::now() < until {
        if let Ok(d) = gw.recv_data(Duration::from_millis(100)) {
            got.extend(d);
        }
    }
    assert_eq!(got, b"[sdroxide-1.0-B2FHM$]\r");

    gw.send_data(b"FF\r");
    let mut buf = [0u8; 16];
    let n = t.read(&mut buf).unwrap();
    assert_eq!(&buf[..n], b"FF\r");

    drop(t);
    let mut lines = Vec::new();
    let until = Instant::now() + Duration::from_secs(60);
    while !lines.iter().any(|l: &String| l.starts_with("DISCONNECTED")) && Instant::now() < until {
        if let Ok(l) = gw.recv_line(Duration::from_millis(100)) {
            lines.push(l);
        }
    }
    assert!(
        lines.iter().any(|l| l.starts_with("DISCONNECTED")),
        "the gateway never saw a hang-up: {lines:?}"
    );
    assert!(handle.claim("panel").is_ok(), "the hold must end with the session");

    stop.store(true, Ordering::SeqCst);
    pump.join().unwrap();
}

#[test]
fn a_refused_callsign_is_named() {
    let mut a = Station::new("OE1AAA", 3);
    let handle = a.mux.handle();
    let stop = Arc::new(AtomicBool::new(false));
    let pump = {
        let stop = stop.clone();
        std::thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                a.step(&[0.0; BLOCK]);
                std::thread::sleep(Duration::from_millis(2));
            }
        })
    };
    let cancel = Arc::new(AtomicBool::new(false));
    let err = TncTransport::connect(
        &handle,
        "Data2G",
        "OE1AAA",
        "OE1GW",
        "BW999",
        Duration::from_secs(10),
        cancel,
    )
    .expect_err("a width the modem does not have is refused");
    assert!(err.to_string().contains("Data2G refused the bandwidth"), "{err}");
    stop.store(true, Ordering::SeqCst);
    pump.join().unwrap();
}
