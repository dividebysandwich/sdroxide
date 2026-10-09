//! In-process loopback test: a minimal fake Hermes-Lite 2 (Protocol 1) that
//! answers discovery and streams EP6 I/Q, exercising the real P1 network thread
//! end-to-end (open → protocol detection → RX ring) without hardware.
//!
//! Uses UDP port 1024 (non-privileged) on localhost; skips gracefully if the
//! port is unavailable.

use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// The C&C bytes of the Hermes-Lite's I2C tunnel, spelled out here rather than
/// imported so the test pins the wire format independently of the crate.
///
/// C0 for a request on I2C bus 2 (address `0x3D`) with the RQST bit set, and the
/// answer the gateware sends back with its ACK bit set. Requests carry the MOX
/// bit too, so compare with that masked off.
const I2C2_RQST: u8 = 0x80 | (0x3D << 1);
const ACK_I2C2: u8 = 0x80 | (0x3D << 1);
/// The address the gateware answers with when nothing on the bus responded.
const ACK_ERROR: u8 = 0x80 | (0x3F << 1);
const I2C_WRITE: u8 = 0x06;
const I2C_READ: u8 = 0x07;
/// The HL2IOBoard: its Pico, its hardware-version register, and what that
/// register reads back.
const IOB_ADDR: u8 = 0x1D;
const IOB_VERSION_ADDR: u8 = 0x41;
const IOB_VERSION: u8 = 0xF1;

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
    // The radio's own PTT input (a Hermes-Lite's CN4 ring), as the test holds
    // it: the fake board reports it in C0 bit 0 of every EP6 frame.
    let ptt = Arc::new(AtomicBool::new(false));
    let ptt_radio = Arc::clone(&ptt);
    // Every C&C block the host sends us on EP2, so the test can check what the
    // board is actually told rather than only what comes back.
    let cc_seen: Arc<Mutex<Vec<[u8; 5]>>> = Arc::new(Mutex::new(Vec::new()));
    let cc_radio = Arc::clone(&cc_seen);
    // Every `(register, byte)` the host wrote to the IO board's Pico over the
    // I2C tunnel, in order.
    let i2c_seen: Arc<Mutex<Vec<(u8, u8)>>> = Arc::new(Mutex::new(Vec::new()));
    let i2c_radio = Arc::clone(&i2c_seen);
    let io_present = Arc::new(AtomicBool::new(false));
    let io_radio = Arc::clone(&io_present);

    // Fake radio: reply to discovery as an HL2 (Protocol 1), and once it has seen
    // the host's streaming socket, stream EP6 frames carrying I=0.5, Q=-0.25.
    let radio_thread = thread::spawn(move || {
        let mut buf = [0u8; 2048];
        let mut host: Option<SocketAddr> = None;
        let mut seq: u32 = 0;
        let mut pending_ack: Option<[u8; 5]> = None;
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
                    // EP2: pull the C&C block out of both OZY frames.
                    if n == 1032 && d[3] == 0x02 {
                        let mut seen = cc_radio.lock().unwrap();
                        for f in 0..2 {
                            let fr = 8 + f * 512;
                            if d[fr] != 0x7F || d[fr + 1] != 0x7F || d[fr + 2] != 0x7F {
                                continue;
                            }
                            let cc: [u8; 5] = d[fr + 3..fr + 8].try_into().unwrap();
                            seen.push(cc);
                            // An I2C request on bus 2, with the MOX bit masked
                            // off: serve it as the gateware and the accessory
                            // bus would, and answer it.
                            if cc[0] & !0x01 == I2C2_RQST {
                                pending_ack = Some(match (cc[1], cc[2]) {
                                    // The IO board's hardware-version register.
                                    (I2C_READ, IOB_VERSION_ADDR)
                                        if io_radio.load(Ordering::Relaxed) =>
                                    {
                                        [ACK_I2C2, IOB_VERSION, 0, 0, 0]
                                    }
                                    // A one-byte write to the Pico; the
                                    // gateware echoes the request back.
                                    (I2C_WRITE, IOB_ADDR) => {
                                        i2c_radio.lock().unwrap().push((cc[3], cc[4]));
                                        [ACK_I2C2, cc[1], cc[2], cc[3], cc[4]]
                                    }
                                    // Nothing on the bus answered.
                                    _ => [ACK_ERROR, cc[1], cc[2], cc[3], cc[4]],
                                });
                            }
                        }
                    }
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
                let ptt_bit = if ptt_radio.load(Ordering::Relaxed) { 0x01 } else { 0x00 };
                for f in 0..2 {
                    let fr = 8 + f * 512;
                    d[fr] = 0x7F;
                    d[fr + 1] = 0x7F;
                    d[fr + 2] = 0x7F;
                    d[fr + 3] = ptt_bit; // C0: status set 0, PTT in bit 0
                    // An outstanding I2C request rides the first frame of the
                    // next datagram, which is what the gateware does with it.
                    if f == 0
                        && let Some(ack) = pending_ack.take()
                    {
                        d[fr + 3..fr + 8].copy_from_slice(&ack);
                    }
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

    // Open the connection: discovery must detect Protocol 1, then RX must
    // flow on the one stream the framing carries.
    let board = sdroxide_hpsdr::HpsdrBoard::open(
        Ipv4Addr::LOCALHOST,
        48_000.0,
        sdroxide_hpsdr::LNA_GAIN_DEFAULT_DB,
        sdroxide_types::HpsdrOcPlan::none(),
        false,
        true,
        sdroxide_types::HpsdrIoRxInput::Radio,
        // Automatic overload protection off: these tests drive the wire, not
        // the loop that rides on it.
        sdroxide_hpsdr::AutoGain::new(false, 1.0, 100, 10_000, -12.0, 48.0),
    )
    .expect("open loopback connection");
    assert_eq!(board.protocol(), 1, "detected as Protocol 1");
    assert_eq!(board.board(), "Hermes-Lite 2");
    // An HL2 is a board whose front-end gain we can command.
    assert!(board.has_lna_gain());
    assert!(!board.io_board_present(), "model eligibility is not accessory detection");
    // Protocol 1 carries exactly one receiver here; a second is refused with
    // a message that says so.
    assert_eq!(board.ddc_count(), 1);
    let err = board.rx(1).expect_err("a P1 board has no DDC 2").to_string();
    assert!(err.contains("1 receiver"), "{err}");
    let mut handle = board.rx(0).expect("attach the receiver");

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

    // End to end, the thing that decides whether a keyed Hermes-Lite makes any
    // power: register 0x09 must reach the board with the PA bit set. It went
    // out as C2 = 0 — "PA off" — for as long as this backend had a transmitter,
    // which keys the radio and puts nothing on the antenna.
    let drive: Vec<[u8; 5]> = cc_seen
        .lock()
        .unwrap()
        .iter()
        .copied()
        .filter(|cc| cc[0] & 0xFE == 0x12) // register 0x09, MOX bit masked off
        .collect();
    assert!(!drive.is_empty(), "the drive register is in the rotation");
    for cc in &drive {
        assert_eq!(cc[2] & 0x08, 0x08, "PA enabled in {cc:02X?}");
        assert_eq!(cc[2] & 0x04, 0, "T/R relay left free to switch in {cc:02X?}");
        assert_eq!(cc[1], 255, "full drive level in {cc:02X?}");
    }

    for (enabled, bits) in [(false, 0x04), (true, 0x08)] {
        cc_seen.lock().unwrap().clear();
        handle.set_onboard_pa(enabled);
        let arrived = (0..300).any(|_| {
            thread::sleep(Duration::from_millis(10));
            cc_seen
                .lock()
                .unwrap()
                .iter()
                .rev()
                .find(|cc| cc[0] & 0xFE == 0x12)
                .is_some_and(|cc| cc[2] & 0x0C == bits)
        });
        assert!(arrived, "PA {enabled} must update register 0x09 and T/R routing");
    }

    cc_seen.lock().unwrap().clear();
    handle.set_onboard_pa(false);
    handle.tx_begin(28_174_000.0);
    let keyed = (0..300).any(|_| {
        thread::sleep(Duration::from_millis(10));
        cc_seen.lock().unwrap().iter().any(|cc| cc[0] & 1 != 0)
    });
    assert!(keyed, "key-down must reach the simulated board");
    {
        let seen = cc_seen.lock().unwrap();
        let first_mox = seen.iter().position(|cc| cc[0] & 1 != 0).unwrap();
        assert!(
            seen[..first_mox].iter().any(|cc| cc[0] & 0xFE == 0x12 && cc[2] & 0x0C == 0x04),
            "PA off and receive-only T/R routing must arrive before MOX"
        );
    }
    handle.tx_end();
    handle.set_onboard_pa(true);
    assert!(!board.io_board_present(), "PA switching does not require an IO board");
    io_present.store(true, Ordering::Relaxed);
    let detected = (0..300).any(|_| {
        thread::sleep(Duration::from_millis(10));
        board.io_board_present()
    });
    assert!(detected, "the accessory probe must publish IO board presence");

    // The radio's own PTT line reaches the handle, which is what carries a foot
    // switch or mic button up to the engine. Reported as a level, so a poll
    // that lands between the press and the release still sees it.
    assert!(!handle.radio_ptt(), "the line starts open");
    ptt.store(true, Ordering::Relaxed);
    let closed = (0..300).any(|_| {
        thread::sleep(Duration::from_millis(10));
        handle.radio_ptt()
    });
    assert!(closed, "a closed PTT line should reach the handle");
    ptt.store(false, Ordering::Relaxed);
    let opened = (0..300).any(|_| {
        thread::sleep(Duration::from_millis(10));
        !handle.radio_ptt()
    });
    assert!(opened, "releasing it should reach the handle too");

    // An HL2IOBoard on the accessory bus is found by asking, and is then told
    // the transmit frequency — the one thing its documentation requires of SDR
    // software, and what makes an external amplifier follow a band change.
    // Nothing here waits for a key-down: the amplifier has to be on the right
    // band before any RF appears.
    handle.set_tx_freq(14_074_000.0);
    // The last five writes, as the register numbers they went to and the
    // frequency they spell, once there are at least that many.
    let latest = || -> Option<(Vec<u8>, u64)> {
        let w = i2c_seen.lock().unwrap();
        let tail = w.get(w.len().checked_sub(5)?..)?;
        Some((
            tail.iter().map(|x| x.0).collect(),
            tail.iter().fold(0u64, |acc, x| acc << 8 | x.1 as u64),
        ))
    };
    // Allow for the half-second rate limit the board's documentation asks for:
    // the frequency it was given at startup goes out first, and this one waits
    // its turn rather than flooding the I2C bus behind it.
    let told = (0..300).any(|_| {
        thread::sleep(Duration::from_millis(10));
        latest().is_some_and(|(_, hz)| hz == 14_074_000)
    });
    let writes = i2c_seen.lock().unwrap().clone();
    assert!(told, "the IO board should be told the transmit frequency, got {writes:02X?}");
    // First contact clears the board: its registers are static and may hold
    // whatever the last program to drive it left behind.
    assert_eq!(writes[0], (5, 1), "REG_CONTROL = 1 resets the board first");
    // The frequency goes as a five-byte big-endian integer in registers 0..=4,
    // with BYTE0 — the write the board acts on — last.
    assert_eq!(latest().expect("five writes").0, vec![0, 1, 2, 3, 4], "in order, BYTE0 last");

    stop.store(true, Ordering::Relaxed);
    drop(handle);
    drop(board);
    let _ = radio_thread.join();
}
