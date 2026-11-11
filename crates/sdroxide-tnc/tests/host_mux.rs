//! The host ports with a real socket on the other end, and an echo "modem"
//! behind them.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread::sleep;
use std::time::{Duration, Instant};

use sdroxide_tnc::{Holder, HostMux, Input};

/// Answer every command `OK <line>` and echo data, as a modem's loop would.
fn pump(m: &mut HostMux, for_ms: u64) -> Vec<String> {
    let mut seen = Vec::new();
    let until = Instant::now() + Duration::from_millis(for_ms);
    while Instant::now() < until {
        while let Some(i) = m.try_recv() {
            match i {
                Input::Command(l) => {
                    m.line(&format!("OK {l}"));
                    seen.push(l);
                }
                Input::Data(d) => m.data(&d),
                Input::Connected => seen.push("<connected>".into()),
                Input::Closed => seen.push("<closed>".into()),
            }
        }
        sleep(Duration::from_millis(5));
    }
    seen
}

fn read_all(s: &mut TcpStream) -> String {
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
    let mut m = HostMux::new("Test");
    m.listen_tcp("127.0.0.1", 0).unwrap();
    let [cmd, data] = m.tcp_addrs().unwrap();
    let mut c = TcpStream::connect(cmd).unwrap();
    let mut d = TcpStream::connect(data).unwrap();
    pump(&mut m, 200);
    assert!(matches!(m.holder(), Some(Holder::Tcp(_))));
    let refused = m.handle().claim("Winlink").err().expect("a TCP host excludes an in-process one");
    assert!(refused.contains("Test modem is in use"), "{refused}");

    c.write_all(b"MYCALL OE1AAA\rVERSION\r").unwrap();
    d.write_all(b"hello").unwrap();
    let seen = pump(&mut m, 300);
    assert!(seen.contains(&"MYCALL OE1AAA".to_string()), "{seen:?}");
    let got = read_all(&mut c);
    assert!(got.contains("OK MYCALL OE1AAA\r"), "{got:?}");
    assert!(got.contains("OK VERSION\r"), "{got:?}");
    assert_eq!(read_all(&mut d), "hello");

    // A second command client is turned away while the first holds the port.
    let mut second = TcpStream::connect(cmd).unwrap();
    pump(&mut m, 200);
    second.set_read_timeout(Some(Duration::from_millis(300))).unwrap();
    let mut b = [0u8; 8];
    assert!(matches!(second.read(&mut b), Ok(0) | Err(_)), "second client should be closed");

    drop(c);
    let seen = pump(&mut m, 500);
    assert!(seen.contains(&"<closed>".to_string()), "{seen:?}");
    assert!(m.handle().claim("Winlink").is_ok(), "the hold must end with the socket");
}

#[test]
fn a_local_client_talks_and_releases_on_drop() {
    let mut m = HostMux::new("Test");
    let h = m.handle();
    let c = h.claim("the panel").unwrap();
    assert_eq!(h.holder(), Some(Holder::Local("the panel".into())));
    assert!(h.claim("Winlink").is_err());
    c.command("CONNECT A B");
    c.send_data(b"xyz");
    let seen = pump(&mut m, 50);
    assert_eq!(seen, vec!["<connected>".to_string(), "CONNECT A B".into()]);
    assert_eq!(c.recv_line(Duration::from_millis(100)).unwrap(), "OK CONNECT A B");
    assert_eq!(c.recv_data(Duration::from_millis(100)).unwrap(), b"xyz");
    drop(c);
    let seen = pump(&mut m, 50);
    assert_eq!(seen, vec!["<closed>".to_string()]);
    assert_eq!(h.holder(), None);

    drop(m);
    let gone = h.claim("late").err().unwrap();
    assert!(gone.contains("no longer in Test"), "{gone}");
}
