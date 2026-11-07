//! The three serial relay protocols, against fake boards on a pty.
//!
//! This test exists for the reason the LimeRFE's next door does. That backend's
//! first release could not talk to a real board at all and every unit test
//! passed, because the tests checked the bytes against *my reading* of the
//! protocol rather than against something that behaves like the firmware.
//!
//! So the fakes below are strict: each one accepts only its own family's frame
//! lengths and rejects anything else, and each records the sequence of
//! `(channel, state)` changes it was actually asked for. A driver that sends
//! four bytes where three are expected, or that numbers a Numato channel from
//! one, fails here rather than in somebody's shack.
//!
//! Unix only: it needs a pty.

#![cfg(unix)]

use std::io::{Read, Write};
use std::os::fd::{FromRawFd, OwnedFd};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sdroxide_relay::{RelayTransport, SerialTransport};
use sdroxide_types::{LineState, RelayFamily, SenseLine, SerialConfig};

/// Open a pty pair, returning the master and the slave's path.
fn open_pty() -> (std::fs::File, String) {
    unsafe {
        let mut master: libc::c_int = 0;
        let mut slave: libc::c_int = 0;
        let rc = libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        assert_eq!(rc, 0, "openpty failed");
        let mut name = [0i8; 256];
        assert_eq!(libc::ttyname_r(slave, name.as_mut_ptr(), name.len()), 0);
        let path = std::ffi::CStr::from_ptr(name.as_ptr()).to_string_lossy().into_owned();
        // The slave stays open for the pty's lifetime; the driver opens the
        // path itself.
        let keep = OwnedFd::from_raw_fd(slave);
        std::mem::forget(keep);
        let flags = libc::fcntl(master, libc::F_GETFL);
        libc::fcntl(master, libc::F_SETFL, flags | libc::O_NONBLOCK);
        (std::fs::File::from_raw_fd(master), path)
    }
}

/// What a fake board saw: every `(channel, closed)` it was told, in order.
type Log = Arc<Mutex<Vec<(u8, bool)>>>;

struct Board {
    stop: Arc<AtomicBool>,
    log: Log,
    join: Option<std::thread::JoinHandle<()>>,
}

impl Drop for Board {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

impl Board {
    fn saw(&self) -> Vec<(u8, bool)> {
        self.log.lock().unwrap().clone()
    }

    /// Wait for the board to have been told `n` things, so the test does not
    /// race the pty.
    fn wait_for(&self, n: usize) {
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while self.log.lock().unwrap().len() < n && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}

fn spawn_board(family: RelayFamily, mut master: std::fs::File) -> Board {
    let stop = Arc::new(AtomicBool::new(false));
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let stop_t = Arc::clone(&stop);
    let log_t = Arc::clone(&log);
    let join = std::thread::spawn(move || {
        let mut buf: Vec<u8> = Vec::new();
        let mut chunk = [0u8; 256];
        // KMtronic and Numato remember their state so a read-back can answer.
        let mut state = [false; 33];
        while !stop_t.load(Ordering::Relaxed) {
            match master.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => buf.extend_from_slice(&chunk[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(2));
                    continue;
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
            loop {
                match family {
                    RelayFamily::Lcus => {
                        if buf.len() < 4 {
                            break;
                        }
                        let f: Vec<u8> = buf.drain(..4).collect();
                        assert_eq!(f[0], 0xA0, "an LCUS frame starts with A0: {f:02x?}");
                        let sum = f[0].wrapping_add(f[1]).wrapping_add(f[2]);
                        assert_eq!(f[3], sum, "bad checksum in {f:02x?}");
                        state[f[1] as usize] = f[2] != 0;
                        log_t.lock().unwrap().push((f[1], f[2] != 0));
                        // The real board says nothing at all.
                    }
                    RelayFamily::KMtronic => {
                        if buf.len() < 3 {
                            break;
                        }
                        let f: Vec<u8> = buf.drain(..3).collect();
                        assert_eq!(f[0], 0xFF, "a KMtronic frame starts with FF: {f:02x?}");
                        let ch = f[1];
                        if f[2] == 0x03 {
                            let _ = master.write_all(&[0xFF, ch, u8::from(state[ch as usize])]);
                        } else {
                            state[ch as usize] = f[2] != 0;
                            log_t.lock().unwrap().push((ch, f[2] != 0));
                        }
                    }
                    RelayFamily::Numato => {
                        let Some(end) = buf.iter().position(|b| *b == b'\r') else {
                            break;
                        };
                        let line: Vec<u8> = buf.drain(..=end).collect();
                        let line = String::from_utf8_lossy(&line[..end]).to_string();
                        let words: Vec<&str> = line.split_whitespace().collect();
                        assert_eq!(words[0], "relay", "unexpected command {line:?}");
                        // Zero-based on the wire, hex-ish for narrow boards.
                        let n = u8::from_str_radix(words[2], 16).expect("channel token");
                        // The module echoes everything it is given.
                        let _ = master.write_all(format!("{line}\n\r").as_bytes());
                        match words[1] {
                            "read" => {
                                let s = if state[n as usize + 1] { "on" } else { "off" };
                                let _ = master.write_all(format!("{s}\n\r>").as_bytes());
                            }
                            w => {
                                let on = w == "on";
                                state[n as usize + 1] = on;
                                log_t.lock().unwrap().push((n + 1, on));
                                let _ = master.write_all(b">");
                            }
                        }
                    }
                }
            }
        }
    });
    Board { stop, log, join: Some(join) }
}

fn cfg(path: &str) -> SerialConfig {
    SerialConfig {
        path: path.to_string(),
        force_rts: LineState::None,
        force_dtr: LineState::None,
        ..SerialConfig::default()
    }
}

/// Channels 1 and 2, which is the antenna-plus-amplifier arrangement the whole
/// subsystem is built around.
const MANAGED: u32 = 0b11;

#[test]
fn an_lcus_board_is_told_both_channels_and_then_only_what_changed() {
    let (master, path) = open_pty();
    let board = spawn_board(RelayFamily::Lcus, master);
    let mut t =
        SerialTransport::open(&cfg(&path), RelayFamily::Lcus, MANAGED, SenseLine::Off).unwrap();

    // The first apply writes everything: this end and the board have no
    // agreement to diff against yet.
    t.apply(0).unwrap();
    board.wait_for(2);
    assert_eq!(board.saw(), vec![(1, false), (2, false)], "both channels are put in a known state");

    // Then only the channel that moved.
    t.apply(0b01).unwrap();
    board.wait_for(3);
    assert_eq!(board.saw()[2], (1, true));

    t.apply(0b11).unwrap();
    board.wait_for(4);
    assert_eq!(board.saw()[3], (2, true));

    // And nothing at all when nothing moved: the deduplication that keeps a
    // 9600-baud board out of the key-down path.
    t.apply(0b11).unwrap();
    std::thread::sleep(Duration::from_millis(30));
    assert_eq!(board.saw().len(), 4, "an unchanged state is not written");
}

/// Issue #627: an LCUS drops a frame that follows the last too closely, so a
/// word moving several contacts is paced, and off air the state is restated
/// because the board cannot be asked whether it agrees.
#[test]
fn an_lcus_word_is_paced_and_restated_off_air() {
    let (master, path) = open_pty();
    let board = spawn_board(RelayFamily::Lcus, master);
    let mut t =
        SerialTransport::open(&cfg(&path), RelayFamily::Lcus, MANAGED, SenseLine::Off).unwrap();
    t.apply(0).unwrap();
    board.wait_for(2);

    let started = std::time::Instant::now();
    t.apply(0b11).unwrap();
    assert!(started.elapsed() >= Duration::from_millis(25), "two frames went out back to back");
    board.wait_for(4);

    assert_eq!(t.read_back().unwrap(), None, "an LCUS has nothing to say");
    board.wait_for(6);
    assert_eq!(&board.saw()[4..], &[(1, true), (2, true)], "the whole word is restated");
}

#[test]
fn a_kmtronic_board_answers_a_read_back_with_what_it_holds() {
    let (master, path) = open_pty();
    let board = spawn_board(RelayFamily::KMtronic, master);
    let mut t =
        SerialTransport::open(&cfg(&path), RelayFamily::KMtronic, MANAGED, SenseLine::Off).unwrap();

    t.apply(0b10).unwrap();
    board.wait_for(2);
    assert_eq!(board.saw(), vec![(1, false), (2, true)]);
    assert_eq!(t.read_back().unwrap(), Some(0b10), "the board agrees with what it was told");
}

#[test]
fn a_numato_channel_one_is_relay_zero_on_the_wire() {
    let (master, path) = open_pty();
    let board = spawn_board(RelayFamily::Numato, master);
    let mut t =
        SerialTransport::open(&cfg(&path), RelayFamily::Numato, MANAGED, SenseLine::Off).unwrap();

    t.apply(0b01).unwrap();
    board.wait_for(2);
    // The fake converts back from the wire's zero-based number, so this
    // asserting channel 1 is the whole point: an off-by-one here would operate
    // the wrong relay in somebody's antenna line.
    assert_eq!(board.saw(), vec![(1, true), (2, false)]);
    assert_eq!(t.read_back().unwrap(), Some(0b01));
}

#[test]
fn a_channel_outside_the_managed_set_is_never_written() {
    let (master, path) = open_pty();
    let board = spawn_board(RelayFamily::Lcus, master);
    // Only channel 2 is ours; channel 1 belongs to whatever else the operator
    // wired to this board.
    let mut t =
        SerialTransport::open(&cfg(&path), RelayFamily::Lcus, 0b10, SenseLine::Off).unwrap();
    t.apply(0b11).unwrap();
    board.wait_for(1);
    std::thread::sleep(Duration::from_millis(30));
    assert_eq!(board.saw(), vec![(2, true)], "channel 1 is left exactly as it was");
}

#[test]
fn a_port_that_is_not_there_says_so_rather_than_being_opened() {
    let opened = SerialTransport::open(
        &cfg("/dev/definitely-not-a-serial-port"),
        RelayFamily::Lcus,
        MANAGED,
        SenseLine::Off,
    );
    let msg = match opened {
        Ok(_) => panic!("opening nothing must fail"),
        Err(e) => e.to_string(),
    };
    assert!(msg.contains("definitely-not-a-serial-port"), "{msg}");
}
