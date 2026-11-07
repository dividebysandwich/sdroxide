//! Protocol 2 transmit I/Q goes out on a clock (#659).
//!
//! The DUC stream used to be sent as fast as the transmit ring filled, so the
//! engine's jitter cushion went to the radio as a burst at key-down and bursts
//! after every block — living in the radio's TX FIFO, which on a Brick2 is
//! shallower than that, and the over came out choppy. One 240-sample datagram
//! per 1250 µs is the stream's own rate at 192 kHz, and what piHPSDR sends.
//!
//! Same real ports as `multi_ddc.rs` on 127.0.0.1, skipped if any is taken.

use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use sdroxide_hpsdr::HpsdrBoard;

fn bind(port: u16) -> Option<UdpSocket> {
    match UdpSocket::bind(("127.0.0.1", port)) {
        Ok(s) => {
            s.set_read_timeout(Some(Duration::from_millis(2))).ok();
            Some(s)
        }
        Err(e) => {
            eprintln!("skip: cannot bind 127.0.0.1:{port} ({e})");
            None
        }
    }
}

#[test]
fn duc_datagrams_are_paced_at_the_stream_rate() {
    let Some(general) = bind(1024) else { return };
    let Some(ddc_cmd) = bind(1025) else { return };
    let Some(duc_cmd) = bind(1026) else { return };
    let Some(high_prio) = bind(1027) else { return };
    let Some(duc_iq) = bind(1029) else { return };

    let stop = Arc::new(AtomicBool::new(false));
    let stop_radio = Arc::clone(&stop);
    // Discovery as a Saturn; every other command drained.
    let radio = std::thread::spawn(move || {
        let mut buf = [0u8; 2048];
        while !stop_radio.load(Ordering::Relaxed) {
            if let Ok((n, src)) = general.recv_from(&mut buf)
                && n >= 60
                && buf[0] != 0xEF
                && buf[4] == 0x02
            {
                let mut reply = [0u8; 60];
                reply[4] = 0x02;
                reply[5..11].copy_from_slice(&[0x00, 0x1C, 0xC0, 0x00, 0x00, 0x02]);
                reply[11] = 10;
                let _ = general.send_to(&reply, src);
            }
            for s in [&ddc_cmd, &duc_cmd, &high_prio] {
                s.set_nonblocking(true).ok();
                while s.recv_from(&mut buf).is_ok() {}
            }
        }
    });

    let board = HpsdrBoard::open(
        "127.0.0.1".parse().unwrap(),
        192_000.0,
        0.0,
        sdroxide_types::HpsdrOcPlan::none(),
        false,
        true,
        sdroxide_types::HpsdrIoRxInput::Radio,
        sdroxide_hpsdr::AutoGain::new(false, 1.0, 100, 10_000, -12.0, 48.0),
    )
    .expect("open");
    let mut rx = board.rx(0).expect("attach DDC 0");

    let rate = rx.tx_begin(14_200_000.0);
    assert_eq!(rate, 192_000.0);
    // 100 ms of transmit I/Q in one go: what a pacer's cushion plus a block
    // looks like from the network thread.
    let pairs = (rate * 0.1) as usize;
    let iq = vec![0.1f32; pairs * 2];
    let t0 = Instant::now();
    rx.tx_write(&iq);

    let want = pairs / 240;
    let mut arrivals = Vec::new();
    let mut buf = [0u8; 2048];
    while arrivals.len() < want && t0.elapsed() < Duration::from_secs(2) {
        if let Ok((n, _)) = duc_iq.recv_from(&mut buf)
            && n >= 4 + 240 * 6
        {
            arrivals.push(t0.elapsed());
        }
    }
    rx.tx_end();
    drop(rx);
    drop(board);
    stop.store(true, Ordering::Relaxed);
    radio.join().unwrap();

    assert_eq!(arrivals.len(), want, "every datagram should arrive");
    let early = arrivals.iter().filter(|t| **t < Duration::from_millis(10)).count();
    assert!(early <= 10, "{early} datagrams in the first 10 ms: sent as a burst, not paced");
    let spread = *arrivals.last().unwrap() - arrivals[0];
    assert!(
        spread >= Duration::from_millis(85),
        "100 ms of I/Q went out in {spread:?}: not paced at the stream rate"
    );
}
