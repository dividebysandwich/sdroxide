//! Connect to a TCI server and dump its init status burst (up to `ready;`).
//! Usage: cargo run -p sdroxide-tci --example probe -- 127.0.0.1:50001

use std::io::ErrorKind;
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use tungstenite::Message;

fn main() {
    let addr = std::env::args().nth(1).unwrap_or_else(|| "127.0.0.1:50001".to_string());
    println!("connecting to {addr} ...");
    let sockaddr = addr.to_socket_addrs().unwrap().next().unwrap();
    let stream = TcpStream::connect_timeout(&sockaddr, Duration::from_secs(3)).unwrap();
    stream.set_read_timeout(Some(Duration::from_millis(250))).unwrap();
    let (mut ws, _) = tungstenite::client(format!("ws://{addr}/").as_str(), stream).unwrap();

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut binaries = 0;
    while Instant::now() < deadline {
        match ws.read() {
            Ok(Message::Text(t)) => {
                for cmd in t.as_str().split(';') {
                    let cmd = cmd.trim();
                    if !cmd.is_empty() {
                        println!("  TEXT  {cmd}");
                    }
                    if cmd == "ready" {
                        println!("--- ready; reached ---");
                    }
                }
            }
            Ok(Message::Binary(b)) => {
                binaries += 1;
                if binaries <= 3 {
                    let ty = u32::from_le_bytes([b[24], b[25], b[26], b[27]]);
                    let sr = u32::from_le_bytes([b[4], b[5], b[6], b[7]]);
                    let len = u32::from_le_bytes([b[20], b[21], b[22], b[23]]);
                    println!("  BIN   type={ty} rate={sr} len={len} bytes={}", b.len());
                }
            }
            Ok(_) => {}
            Err(tungstenite::Error::Io(e))
                if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {}
            Err(e) => {
                println!("read error: {e}");
                break;
            }
        }
    }
    println!("(done; {binaries} binary frames seen)");
}
