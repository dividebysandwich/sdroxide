//! `VaraTransport` against a second modem on a simulated channel: the call
//! comes up, bytes go both ways through `Read`/`Write`, and dropping the
//! transport hangs up cleanly at the far end.

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use sdroxide_vara::{Bandwidth, new_modem};
use sdroxide_winlink::transport::{Transport, VaraTransport};

const BLOCK: usize = 960;

#[test]
fn a_vara_transport_carries_bytes_and_hangs_up() {
    let mut a = new_modem(Bandwidth::Hz2300, "OE1AAA", false, 1);
    let mut b = new_modem(Bandwidth::Hz2300, "OE1GW", false, 2);
    let handle = a.host().handle();
    let gw = b.host().handle().claim("gateway").unwrap();
    gw.command("MYCALL OE1GW");
    gw.command("LISTEN ON");

    // The air, on its own thread, as fast as it will go.
    let stop = Arc::new(AtomicBool::new(false));
    let pump = {
        let stop = stop.clone();
        std::thread::spawn(move || {
            let (mut ab, mut ba) = (VecDeque::<f64>::new(), VecDeque::<f64>::new());
            let (mut a_out, mut b_out) = (vec![0.0; BLOCK], vec![0.0; BLOCK]);
            while !stop.load(Ordering::SeqCst) {
                ab.extend(a_out.iter().copied());
                ba.extend(b_out.iter().copied());
                let a_in: Vec<f64> = (0..BLOCK).map(|_| ba.pop_front().unwrap_or(0.0)).collect();
                let b_in: Vec<f64> = (0..BLOCK).map(|_| ab.pop_front().unwrap_or(0.0)).collect();
                a.process(&a_in, &mut a_out);
                b.process(&b_in, &mut b_out);
                std::thread::sleep(Duration::from_micros(200));
            }
        })
    };

    let cancel = Arc::new(AtomicBool::new(false));
    let mut t = VaraTransport::connect(&handle, "OE1AAA", "OE1GW", Duration::from_secs(60), cancel)
        .expect("the gateway should answer");
    assert_eq!(t.target_call(), "OE1GW");
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
    while !lines.iter().any(|l: &String| l == "DISCONNECTED") && Instant::now() < until {
        if let Ok(l) = gw.recv_line(Duration::from_millis(100)) {
            lines.push(l);
        }
    }
    assert!(
        lines.iter().any(|l| l == "DISCONNECTED"),
        "the gateway never saw a hang-up: {lines:?}"
    );
    assert!(handle.claim("panel").is_ok(), "the hold must end with the session");

    stop.store(true, Ordering::SeqCst);
    pump.join().unwrap();
}
