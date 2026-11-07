//! Drive a running sdroxide server's picture stores over the WebSocket and
//! report what comes back.
//!
//! An end-to-end check of the path a browser tab takes, which no unit test can
//! reach: connect, receive the presets replayed on connect, upload a picture and
//! see it come back scaled and thumbnailed, fetch its source, list both received
//! stores, and try to fetch a name from outside one.
//!
//! Safe to point at a station in use. It borrows an *empty* transmit slot and
//! empties it again at the end, and skips the upload checks entirely when the
//! operator has filled all five — a probe has no business overwriting the
//! picture somebody was about to send.
//!
//! Usage: cargo run -p sdroxide-proto --example picture_store_probe -- 127.0.0.1:4950

use std::io::ErrorKind;
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use tungstenite::Message;

use sdroxide_proto::{AudioCaps, ClientMsg, PROTO_VERSION, ServerMsg, decode, encode};
use sdroxide_types::{Command, IMAGE_SLOTS, ImageKind};

fn main() {
    let addr = std::env::args().nth(1).unwrap_or_else(|| "127.0.0.1:4950".to_string());
    println!("connecting to {addr} ...");
    let sockaddr = addr.to_socket_addrs().unwrap().next().unwrap();
    let stream = TcpStream::connect_timeout(&sockaddr, Duration::from_secs(3)).unwrap();
    stream.set_read_timeout(Some(Duration::from_millis(250))).unwrap();
    let (mut ws, _) = tungstenite::client(format!("ws://{addr}/ws").as_str(), stream).unwrap();

    let send = |ws: &mut tungstenite::WebSocket<TcpStream>, msg: &ClientMsg| {
        ws.send(Message::Binary(encode(msg).unwrap().into())).unwrap();
    };
    send(
        &mut ws,
        &ClientMsg::Hello {
            proto: PROTO_VERSION,
            audio: AudioCaps { opus_decode: false, opus_encode: false },
        },
    );

    // Bigger than the store's bound in both directions, so what comes back
    // proves the engine scaled it rather than took it as given.
    let big = {
        let img = image::RgbImage::from_fn(2000, 1000, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, 128])
        });
        let mut buf = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut buf, image::ImageFormat::Png)
            .expect("encode");
        buf.into_inner()
    };

    let mut checks: Vec<(bool, String)> = Vec::new();
    // Which slot the upload checks borrowed, once one has been found free.
    let mut borrowed: Option<u8> = None;
    let mut greeted = false;
    // Handshake, the connect-time presets, the upload echo, the source, two
    // listings, the refused fetch, and the presets echo of putting the slot back.
    let mut want = 8;

    let deadline = Instant::now() + Duration::from_secs(30);
    while want > 0 && Instant::now() < deadline {
        let msg = match ws.read() {
            Ok(Message::Binary(b)) => match decode::<ServerMsg>(&b) {
                Ok(m) => m,
                Err(e) => {
                    checks.push((false, format!("undecodable message: {e}")));
                    break;
                }
            },
            Ok(_) => continue,
            Err(tungstenite::Error::Io(e))
                if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut =>
            {
                continue;
            }
            Err(e) => {
                checks.push((false, format!("connection: {e}")));
                break;
            }
        };
        match msg {
            ServerMsg::HelloAck { proto, .. } => {
                checks.push((proto == PROTO_VERSION, format!("handshake at v{proto}")));
                want -= 1;
            }
            // Replayed on connect. This is the one that was missing: a browser
            // tab never saw the operator's slots at all.
            ServerMsg::ImagePresets(p) if !greeted => {
                greeted = true;
                checks.push((
                    p.slots.len() == IMAGE_SLOTS,
                    format!("presets replayed on connect: {} slots", p.slots.len()),
                ));
                want -= 1;

                // The listing and traversal checks stand on their own; only the
                // upload ones need somewhere to put a picture.
                for kind in ImageKind::ALL {
                    send(
                        &mut ws,
                        &ClientMsg::Command(Command::ImageList { kind, offset: 0, count: 8 }),
                    );
                }
                send(
                    &mut ws,
                    &ClientMsg::Command(Command::ImageGet {
                        kind: ImageKind::Sstv,
                        name: "../../../etc/passwd".into(),
                    }),
                );

                match (0..IMAGE_SLOTS).find(|i| !p.slot(*i).has_picture()) {
                    Some(free) => {
                        borrowed = Some(free as u8);
                        println!("borrowing empty slot {} for the upload checks", free + 1);
                        send(
                            &mut ws,
                            &ClientMsg::Command(Command::ImageSetSlot {
                                slot: free as u8,
                                bytes: big.clone(),
                            }),
                        );
                    }
                    None => {
                        println!(
                            "all slots are in use — skipping the upload checks rather than \
                             overwriting one"
                        );
                        want -= 3; // the upload echo, the source, and the put-back
                        checks.push((true, "upload checks skipped (no empty slot)".into()));
                    }
                }
            }
            // The upload, come back scaled and thumbnailed; then the same slot
            // emptied again, which is the probe putting it back.
            ServerMsg::ImagePresets(p) => {
                let Some(slot) = borrowed else { continue };
                let s = p.slot(usize::from(slot));
                if !s.has_picture() {
                    checks.push((true, format!("slot {} put back empty", slot + 1)));
                    want -= 1;
                    borrowed = None;
                    continue;
                }
                checks.push((
                    (s.width, s.height) == (1024, 512),
                    format!("upload stored at {}×{} (bounded from 2000×1000)", s.width, s.height),
                ));
                checks.push((
                    !s.thumb.is_empty() && s.version != 0,
                    format!("slot thumbnail {} B, version {:#010x}", s.thumb.len(), s.version),
                ));
                want -= 1;
                send(&mut ws, &ClientMsg::Command(Command::ImageGetSlot(slot)));
            }
            ServerMsg::ImageSlotSource { slot, version, png } => {
                let size = image::load_from_memory(&png).map(|i| (i.width(), i.height())).ok();
                checks.push((
                    Some(slot) == borrowed && version != 0 && size == Some((1024, 512)),
                    format!("source fetched: slot {}, {} B, {size:?}", slot + 1, png.len()),
                ));
                want -= 1;
                // Everything the upload path had to show has been shown; give
                // the slot back before anything else can go wrong.
                send(&mut ws, &ClientMsg::Command(Command::ImageClearSlot(slot)));
            }
            ServerMsg::ImageListing(l) => {
                let thumbed = l.entries.iter().filter(|e| !e.thumb.is_empty()).count();
                let ordered = l.entries.windows(2).all(|w| w[0].unix >= w[1].unix);
                checks.push((
                    !l.dir.is_empty() && thumbed == l.entries.len() && ordered,
                    format!(
                        "{:?}: {} of {} listed newest-first, all thumbnailed, dir {}",
                        l.kind,
                        l.entries.len(),
                        l.total,
                        l.dir,
                    ),
                ));
                want -= 1;
            }
            ServerMsg::ImageFile { name, png, .. } => {
                checks.push((
                    png.is_empty(),
                    format!("traversal refused and answered empty: {name:?}"),
                ));
                want -= 1;
            }
            ServerMsg::Busy => {
                println!("FAIL another client already holds the control session");
                std::process::exit(1);
            }
            _ => {}
        }
    }

    let mut failed = want > 0;
    for (ok, what) in &checks {
        failed |= !ok;
        println!("{} {what}", if *ok { "ok  " } else { "FAIL" });
    }
    if want > 0 {
        println!("FAIL timed out with {want} answer(s) outstanding");
    }
    std::process::exit(i32::from(failed));
}
