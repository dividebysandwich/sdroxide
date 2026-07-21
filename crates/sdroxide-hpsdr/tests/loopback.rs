//! In-process loopback test: a minimal fake Hermes-Lite 2 (Protocol 1) that
//! answers discovery and streams EP6 I/Q, exercising the real P1 network thread
//! end-to-end (open → protocol detection → RX ring) without hardware.
//!
//! Uses UDP port 1024 (non-privileged) on localhost; skips gracefully if the
//! port is unavailable.

use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

/// 24-bit big-endian encoder matching the crate's decoder (÷ 2^23).
fn be24(x: f32) -> [u8; 3] {
    let v = (x.clamp(-1.0, 1.0) * 8_388_607.0).round() as i32;
    let u = (v as u32) & 0x00FF_FFFF;
    [(u >> 16) as u8, (u >> 8) as u8, u as u8]
}

#[test]
fn p1_loopback_rx() {
    let radio = match UdpSocket::bind((Ipv4Addr::LOCALHOST, 1024)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("skip p1_loopback_rx: cannot bind 127.0.0.1:1024 ({e})");
            return;
        }
    };
    radio.set_read_timeout(Some(Duration::from_millis(20))).unwrap();

    let stop = Arc::new(AtomicBool::new(false));
    let stop_r = stop.clone();

    // Fake radio: reply to discovery as an HL2 (Protocol 1), and once it has seen
    // the host's streaming socket, stream EP6 frames carrying I=0.5, Q=-0.25.
    let radio_thread = thread::spawn(move || {
        let mut buf = [0u8; 2048];
        let mut host: Option<SocketAddr> = None;
        let mut seq: u32 = 0;
        let i = be24(0.5);
        let q = be24(-0.25);
        while !stop_r.load(Ordering::Relaxed) {
            if let Ok((n, src)) = radio.recv_from(&mut buf) {
                let d = &buf[..n];
                let is_discovery = n >= 3 && d[0] == 0xEF && d[1] == 0xFE && d[2] == 0x02;
                if is_discovery {
                    // Protocol 1 discovery response: sync, idle status, MAC, board id 6.
                    let mut r = [0u8; 60];
                    r[0] = 0xEF;
                    r[1] = 0xFE;
                    r[2] = 0x02;
                    r[3..9].copy_from_slice(&[0x00, 0x1C, 0xC0, 0xAA, 0xBB, 0xCC]);
                    r[10] = 6; // Hermes-Lite 2
                    let _ = radio.send_to(&r, src);
                } else {
                    // Any non-discovery packet (start command / EP2) reveals the
                    // host's streaming socket.
                    host = Some(src);
                }
            }
            if let Some(h) = host {
                let mut d = [0u8; 1032];
                d[0] = 0xEF;
                d[1] = 0xFE;
                d[2] = 0x01;
                d[3] = 0x06; // EP6
                d[4..8].copy_from_slice(&seq.to_be_bytes());
                seq = seq.wrapping_add(1);
                for f in 0..2 {
                    let fr = 8 + f * 512;
                    d[fr] = 0x7F;
                    d[fr + 1] = 0x7F;
                    d[fr + 2] = 0x7F;
                    for s in 0..63 {
                        let b = fr + 8 + s * 8;
                        d[b..b + 3].copy_from_slice(&i);
                        d[b + 3..b + 6].copy_from_slice(&q);
                    }
                }
                let _ = radio.send_to(&d, h);
            }
        }
    });

    // Open the handle: discovery must detect Protocol 1, then RX must flow.
    let mut handle = sdroxide_hpsdr::HpsdrHandle::open(Ipv4Addr::LOCALHOST, 48_000.0)
        .expect("open loopback handle");
    assert_eq!(handle.protocol, 1, "detected as Protocol 1");
    assert_eq!(handle.board, "Hermes-Lite 2");

    let mut out = vec![0f32; 4096];
    let mut got = 0usize;
    for _ in 0..300 {
        let n = handle.rx_read(&mut out);
        if n >= 2 {
            got = n;
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    assert!(got >= 2, "received I/Q from the fake radio");
    assert!((out[0] - 0.5).abs() < 0.01, "I ~= 0.5, got {}", out[0]);
    assert!((out[1] + 0.25).abs() < 0.01, "Q ~= -0.25, got {}", out[1]);

    stop.store(true, Ordering::Relaxed);
    drop(handle);
    let _ = radio_thread.join();
}
