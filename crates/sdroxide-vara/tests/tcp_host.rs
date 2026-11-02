//! The VARA TCP ports, with a real socket on the other end.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

use sdroxide_vara::{Bandwidth, Vara, new_modem};

fn pump(m: &mut Vara, for_ms: u64) {
    let until = Instant::now() + Duration::from_millis(for_ms);
    let mut out = vec![0.0; 960];
    while Instant::now() < until {
        m.process(&[0.0; 960], &mut out);
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn read_lines(s: &mut TcpStream) -> String {
    s.set_read_timeout(Some(Duration::from_millis(200))).unwrap();
    let mut buf = [0u8; 4096];
    let mut all = String::new();
    while let Ok(n) = s.read(&mut buf) {
        if n == 0 {
            break;
        }
        all.push_str(&String::from_utf8_lossy(&buf[..n]));
    }
    all
}

#[test]
fn a_tcp_host_is_answered_and_excludes_local_claims() {
    let mut m = new_modem(Bandwidth::Hz2300, "OE1AAA", false, 1);
    m.host_mut().listen_tcp("127.0.0.1", 0).unwrap();
    let [cmd, data] = m.host().tcp_addrs().unwrap();
    let mut c = TcpStream::connect(cmd).unwrap();
    let _d = TcpStream::connect(data).unwrap();
    pump(&mut m, 200);
    assert!(m.host().handle().claim("Winlink").is_err(), "a TCP host must exclude an in-process one");

    c.write_all(b"MYCALL OE1AAA\rVERSION\rBW500\rNONSENSE\r").unwrap();
    pump(&mut m, 300);
    let got = read_lines(&mut c);
    assert!(got.contains("VERSION VARA HF v4.9.0\r"), "{got:?}");
    assert!(got.contains("OK\r"), "{got:?}");
    assert!(got.contains("WRONG\r"), "{got:?}");

    // A second command client is turned away while the first holds the port.
    let mut second = TcpStream::connect(cmd).unwrap();
    pump(&mut m, 200);
    second.set_read_timeout(Some(Duration::from_millis(300))).unwrap();
    let mut b = [0u8; 8];
    assert!(matches!(second.read(&mut b), Ok(0) | Err(_)), "second client should be closed");

    drop(c);
    pump(&mut m, 500);
    assert!(m.host().handle().claim("Winlink").is_ok(), "the hold must end with the socket");
}
