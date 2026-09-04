//! An in-process `iiod` that a real [`PlutoHandle`] is driven against.
//!
//! This backend was written without a Pluto to test it on, which makes this the
//! main defence: it exercises the actual client — three connections, the
//! control thread, the receive decoder, the transmit encoder — against a server
//! that answers the way libiio's own daemon documents.
//!
//! Two things it deliberately does *not* do:
//!
//! - It does not use this crate's encoder to build the samples it serves. A
//!   self-consistent encode/decode pair passes its own test while both are
//!   wrong; the bytes below are written out by hand from the AD9361 wire format
//!   (12 significant bits, two's complement, little-endian, in a 16-bit slot).
//! - It does not prove the framing is what a real `iiod` produces. That is a
//!   claim about hardware, and only hardware can settle it — see the trace's
//!   `first READBUF chunk` line.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sdroxide_pluto::PlutoHandle;
use sdroxide_types::{PlutoAgc, PlutoConfig};

/// A trimmed but structurally faithful Pluto context.
const CONTEXT_XML: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<context name="xml" description="Linux (none) 5.10.0 #1 SMP PREEMPT armv7l">
  <context-attribute name="hw_model" value="Analog Devices PlutoSDR Rev.B (Z7010-AD9364)" />
  <context-attribute name="hw_serial" value="1044735411960005" />
  <context-attribute name="fw_version" value="v0.39" />
  <device id="iio:device0" name="ad9361-phy">
    <channel id="voltage0" type="input">
      <attribute name="rf_port_select" filename="in_voltage0_rf_port_select" />
      <attribute name="rf_port_select_available" filename="in_voltage0_rf_port_select_available" />
      <attribute name="hardwaregain" filename="in_voltage0_hardwaregain" />
      <attribute name="hardwaregain_available" filename="in_voltage0_hardwaregain_available" />
      <attribute name="gain_control_mode" filename="in_voltage0_gain_control_mode" />
      <attribute name="gain_control_mode_available" filename="in_voltage0_gain_control_mode_available" />
      <attribute name="rf_bandwidth" filename="in_voltage0_rf_bandwidth" />
      <attribute name="rf_bandwidth_available" filename="in_voltage0_rf_bandwidth_available" />
      <attribute name="sampling_frequency" filename="in_voltage0_sampling_frequency" />
      <attribute name="sampling_frequency_available" filename="in_voltage0_sampling_frequency_available" />
    </channel>
    <channel id="voltage0" type="output">
      <attribute name="rf_port_select" filename="out_voltage0_rf_port_select" />
      <attribute name="rf_port_select_available" filename="out_voltage0_rf_port_select_available" />
      <attribute name="hardwaregain" filename="out_voltage0_hardwaregain" />
      <attribute name="hardwaregain_available" filename="out_voltage0_hardwaregain_available" />
      <attribute name="rf_bandwidth" filename="out_voltage0_rf_bandwidth" />
      <attribute name="sampling_frequency" filename="out_voltage0_sampling_frequency" />
    </channel>
    <channel id="altvoltage0" name="RX_LO" type="output">
      <attribute name="frequency" filename="out_altvoltage0_RX_LO_frequency" />
      <attribute name="frequency_available" filename="out_altvoltage0_RX_LO_frequency_available" />
    </channel>
    <channel id="altvoltage1" name="TX_LO" type="output">
      <attribute name="frequency" filename="out_altvoltage1_TX_LO_frequency" />
      <attribute name="frequency_available" filename="out_altvoltage1_TX_LO_frequency_available" />
    </channel>
    <attribute name="ensm_mode" filename="ensm_mode" />
    <debug-attribute name="adi,frequency-division-duplex-mode-enable" />
    <debug-attribute name="adi,gpo0-slave-rx-enable" />
    <debug-attribute name="adi,gpo0-slave-tx-enable" />
    <debug-attribute name="adi,gpo1-slave-rx-enable" />
    <debug-attribute name="adi,gpo1-slave-tx-enable" />
    <debug-attribute name="adi,gpo2-slave-rx-enable" />
    <debug-attribute name="adi,gpo2-slave-tx-enable" />
    <debug-attribute name="adi,gpo3-slave-rx-enable" />
    <debug-attribute name="adi,gpo3-slave-tx-enable" />
    <debug-attribute name="adi,elna-rx1-gpo0-control-enable" />
    <debug-attribute name="adi,elna-rx2-gpo1-control-enable" />
    <debug-attribute name="initialize" />
  </device>
  <device id="iio:device3" name="cf-ad9361-lpc">
    <channel id="voltage0" type="input">
      <scan-element index="0" format="le:s12/16&gt;&gt;0" />
    </channel>
    <channel id="voltage1" type="input">
      <scan-element index="1" format="le:s12/16&gt;&gt;0" />
    </channel>
  </device>
  <device id="iio:device4" name="cf-ad9361-dds-core-lpc">
    <channel id="voltage0" type="output">
      <scan-element index="0" format="le:s16/16&gt;&gt;0" />
    </channel>
    <channel id="voltage1" type="output">
      <scan-element index="1" format="le:s16/16&gt;&gt;0" />
    </channel>
    <channel id="altvoltage0" name="TX1_I_F1" type="output">
      <attribute name="raw" filename="out_altvoltage0_TX1_I_F1_raw" />
    </channel>
    <channel id="altvoltage1" name="TX1_Q_F1" type="output">
      <attribute name="raw" filename="out_altvoltage1_TX1_Q_F1_raw" />
    </channel>
  </device>
</context>"#;

/// The same context as served by a Pluto+: 2R2T firmware, so both DMA buffers
/// carry four scan elements — two chains' worth of I/Q. This is the shape the
/// HamGeek Pluto+ field report showed, and it used to be refused outright.
fn pluto_plus_xml() -> String {
    let xml = CONTEXT_XML
        // A 2R2T firmware exposes the second chain's *controls* as well as its
        // samples: its own gain register, gain-control mode and port select,
        // hanging off the phy's `voltage1` input channel. Without them there is
        // nothing to reproduce issue #311 against — the client holds every
        // write for want of a channel to send it to.
        .replace(
            r#"    <channel id="voltage0" type="output">
      <attribute name="rf_port_select" filename="out_voltage0_rf_port_select" />"#,
            r#"    <channel id="voltage1" type="input">
      <attribute name="rf_port_select" filename="in_voltage1_rf_port_select" />
      <attribute name="rf_port_select_available" filename="in_voltage1_rf_port_select_available" />
      <attribute name="hardwaregain" filename="in_voltage1_hardwaregain" />
      <attribute name="hardwaregain_available" filename="in_voltage1_hardwaregain_available" />
      <attribute name="gain_control_mode" filename="in_voltage1_gain_control_mode" />
      <attribute name="gain_control_mode_available" filename="in_voltage1_gain_control_mode_available" />
      <attribute name="rf_bandwidth" filename="in_voltage1_rf_bandwidth" />
      <attribute name="sampling_frequency" filename="in_voltage1_sampling_frequency" />
    </channel>
    <channel id="voltage0" type="output">
      <attribute name="rf_port_select" filename="out_voltage0_rf_port_select" />"#,
        )
        .replace(
            r#"<scan-element index="1" format="le:s12/16&gt;&gt;0" />
    </channel>"#,
            r#"<scan-element index="1" format="le:s12/16&gt;&gt;0" />
    </channel>
    <channel id="voltage2" type="input">
      <scan-element index="2" format="le:s12/16&gt;&gt;0" />
    </channel>
    <channel id="voltage3" type="input">
      <scan-element index="3" format="le:s12/16&gt;&gt;0" />
    </channel>"#,
        )
        .replace(
            r#"<scan-element index="1" format="le:s16/16&gt;&gt;0" />
    </channel>"#,
            r#"<scan-element index="1" format="le:s16/16&gt;&gt;0" />
    </channel>
    <channel id="voltage2" type="output">
      <scan-element index="2" format="le:s16/16&gt;&gt;0" />
    </channel>
    <channel id="voltage3" type="output">
      <scan-element index="3" format="le:s16/16&gt;&gt;0" />
    </channel>"#,
        );
    assert_eq!(xml.matches("voltage3").count(), 2, "the 2R2T surgery must have taken");
    assert!(xml.contains("in_voltage1_hardwaregain"), "the second chain's controls must be there");
    xml
}

/// What the fake device remembers, so the test can assert on what the client
/// actually did to it.
#[derive(Default)]
struct DeviceState {
    attrs: Vec<(String, String)>,
    /// Every `WRITEBUF` payload the client sent.
    tx_payloads: Vec<Vec<u8>>,
    rx_buffer_opens: usize,
    tx_buffer_opens: usize,
    rx_buffer_open: bool,
    tx_buffer_open: bool,
    /// The channel mask of every `OPEN` on the receive buffer, verbatim.
    rx_open_masks: Vec<String>,
    /// … and on the transmit buffer.
    tx_open_masks: Vec<String>,
    /// Pause this long in the middle of the next `READBUF` payload, once.
    /// Models the intermittent gap a Pluto on a USB gadget produces every few
    /// minutes. How long it lasts decides which of the client's two answers is
    /// under test — see [`HICCUP`] and [`STALL`].
    stall_next_readbuf: Option<Duration>,
    /// Answer every `READBUF` with `-EAGAIN` until the buffer is reopened.
    /// Models a wedged DMA: the connection is fine, the server is answering,
    /// and no amount of further reading will ever produce a sample.
    wedged_until_reopen: bool,
}

/// The sample-rate floor a stock Pluto publishes — and refuses.
///
/// The AD9361's clock-chain solver takes a rate only if `rate × 12` reaches the
/// ADC's 25 MHz minimum, so the true floor is 25 MHz / 12 = 2083333.33 Hz. The
/// driver prints that range through integer division, which truncates it to
/// 2083333 — four hertz below what it will actually accept. Writing back the
/// number the device just published therefore earns `-EINVAL`.
///
/// This fixture used to advertise no rate range at all, which left the client's
/// own fallback floor (521 kHz, only reachable with the FIR decimator loaded)
/// in charge and hid the collision completely: every test passed while every
/// real Pluto failed to open.
const AD9361_RATE_FLOOR: i64 = 2_083_333;

impl DeviceState {
    fn get(&self, key: &str) -> Option<&str> {
        self.attrs.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    fn writes_of(&self, key: &str) -> Vec<&str> {
        self.attrs.iter().filter(|(k, _)| k == key).map(|(_, v)| v.as_str()).collect()
    }
}

struct Fake {
    addr: std::net::SocketAddr,
    state: Arc<Mutex<DeviceState>>,
    stop: Arc<AtomicBool>,
    connections: Arc<AtomicUsize>,
}

impl Fake {
    fn start() -> Fake {
        Fake::start_with(DeviceState::default(), CONTEXT_XML.to_string())
    }

    /// A device that goes quiet in the middle of one buffer for longer than the
    /// client will wait, and then carries on.
    fn start_that_stalls_mid_buffer() -> Fake {
        Fake::start_with(
            DeviceState { stall_next_readbuf: Some(STALL), ..DeviceState::default() },
            CONTEXT_XML.to_string(),
        )
    }

    /// The same fault, briefly enough that the client should ride it out on the
    /// socket it already has — see [`HICCUP`].
    fn start_that_hiccups_mid_buffer() -> Fake {
        Fake::start_with(
            DeviceState { stall_next_readbuf: Some(HICCUP), ..DeviceState::default() },
            CONTEXT_XML.to_string(),
        )
    }

    /// A device whose first buffer produces nothing at all and whose second
    /// one — after a reopen — works.
    fn start_wedged_until_reopened() -> Fake {
        Fake::start_with(
            DeviceState { wedged_until_reopen: true, ..DeviceState::default() },
            CONTEXT_XML.to_string(),
        )
    }

    /// A fake Pluto+: the same device behaviour behind a 2R2T context.
    fn start_pluto_plus() -> Fake {
        Fake::start_with(DeviceState::default(), pluto_plus_xml())
    }

    fn start_with(initial: DeviceState, xml: String) -> Fake {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        let state = Arc::new(Mutex::new(initial));
        let stop = Arc::new(AtomicBool::new(false));
        let connections = Arc::new(AtomicUsize::new(0));
        {
            let state = Arc::clone(&state);
            let stop = Arc::clone(&stop);
            let connections = Arc::clone(&connections);
            let xml = Arc::new(xml);
            std::thread::spawn(move || {
                for sock in listener.incoming() {
                    if stop.load(Ordering::Relaxed) {
                        break;
                    }
                    let Ok(sock) = sock else { break };
                    connections.fetch_add(1, Ordering::Relaxed);
                    let state = Arc::clone(&state);
                    let stop = Arc::clone(&stop);
                    let xml = Arc::clone(&xml);
                    std::thread::spawn(move || serve(sock, state, stop, xml));
                }
            });
        }
        Fake { addr, state, stop, connections }
    }

    fn address(&self) -> String {
        self.addr.to_string()
    }
}

impl Drop for Fake {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        // Unblock the accept loop.
        let _ = TcpStream::connect(self.addr);
    }
}

/// The default value of every attribute the client reads before it writes one.
fn default_attr(key: &str) -> &'static str {
    match key {
        "ad9361-phy/OUTPUT/altvoltage0/frequency_available" => "[70000000 1 6000000000]",
        "ad9361-phy/OUTPUT/altvoltage1/frequency_available" => "[70000000 1 6000000000]",
        "ad9361-phy/INPUT/voltage0/hardwaregain_available" => "[0 1 71]",
        "ad9361-phy/OUTPUT/voltage0/hardwaregain_available" => "[-89.750000 0.250000 0.000000]",
        "ad9361-phy/INPUT/voltage0/gain_control_mode_available" => {
            "manual fast_attack slow_attack hybrid"
        }
        // The second chain, on a 2R2T firmware. Its gain-control mode boots
        // where the driver's device tree left it and *not* where chain 0's was
        // just put — which is the whole of issue #311.
        "ad9361-phy/INPUT/voltage1/hardwaregain_available" => "[0 1 71]",
        "ad9361-phy/INPUT/voltage1/gain_control_mode_available" => {
            "manual fast_attack slow_attack hybrid"
        }
        "ad9361-phy/INPUT/voltage1/gain_control_mode" => "slow_attack",
        "ad9361-phy/INPUT/voltage1/rf_port_select_available" => "A_BALANCED B_BALANCED",
        "ad9361-phy/INPUT/voltage1/rf_port_select" => "A_BALANCED",
        "ad9361-phy/INPUT/voltage0/rf_port_select_available" => "A_BALANCED B_BALANCED",
        "ad9361-phy/OUTPUT/voltage0/rf_port_select_available" => "A B",
        "ad9361-phy/INPUT/voltage0/rf_port_select" => "A_BALANCED",
        "ad9361-phy/OUTPUT/voltage0/rf_port_select" => "A",
        // The ranges a real Pluto publishes, verbatim — including the floor
        // that is a truncated quotient (see AD9361_RATE_FLOOR).
        "ad9361-phy/INPUT/voltage0/sampling_frequency_available" => "[2083333 1 61440000]",
        "ad9361-phy/INPUT/voltage0/rf_bandwidth_available" => "[200000 1 56000000]",
        "ad9361-phy/INPUT/voltage0/sampling_frequency" => "2500000",
        "ad9361-phy/OUTPUT/voltage0/sampling_frequency" => "2500000",
        "ad9361-phy/INPUT/voltage0/rf_bandwidth" => "1800000",
        // How a Pluto boots: transmit and receive enabled together, which is
        // what makes full duplex a question about the link rather than about
        // the part.
        "ad9361-phy/ensm_mode" => "fdd",
        _ => "0",
    }
}

/// One I/Q sample set as a real device would put it on the wire: two 12-bit
/// two's-complement values, each little-endian in a 16-bit slot.
///
/// Written out by hand rather than by calling this crate's encoder, so an
/// encoder that has the byte order or the sign extension wrong cannot make this
/// test agree with it.
/// +2047, positive full scale — `0x07FF` on the wire.
const SAMPLE_I: i16 = 2047;
/// -2048, negative full scale. On the wire that is `0x0800`: in *12-bit* two's
/// complement the top bit of the 12 is the sign, so this reads as +2048 if the
/// decoder sign-extends from the 16-bit container instead.
const SAMPLE_Q: i16 = -2048;
const SAMPLE_BYTES: [u8; 4] = [0xFF, 0x07, 0x00, 0x08];

/// The second chain's distinct pattern (±half scale), emitted only when the
/// receive mask enables both pairs — what proves the deinterleaver routes
/// each chain to its own ring rather than duplicating the first.
const SAMPLE2_I: i16 = 1024;
const SAMPLE2_Q: i16 = -1024;
/// +1024 = `0x400`; -1024 in 12-bit two's complement = `0xC00`.
const SAMPLE2_BYTES: [u8; 4] = [0x00, 0x04, 0x00, 0x0C];

fn serve(sock: TcpStream, state: Arc<Mutex<DeviceState>>, stop: Arc<AtomicBool>, xml: Arc<String>) {
    let _ = sock.set_read_timeout(Some(Duration::from_millis(100)));
    let mut writer = sock.try_clone().expect("clone");
    let mut reader = BufReader::new(sock);
    let mut line = String::new();
    while !stop.load(Ordering::Relaxed) {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => continue,
            Err(_) => break,
        }
        let cmd = line.trim_end_matches(['\r', '\n']).to_string();
        if cmd.is_empty() {
            continue;
        }
        let words: Vec<&str> = cmd.split_whitespace().collect();
        let ok = match words.first().copied() {
            Some("VERSION") => writer.write_all(b"0.25 (git tag:v0.25)\n").is_ok(),
            Some("TIMEOUT") => writer.write_all(b"0\n").is_ok(),
            Some("PRINT") => writer.write_all(format!("{}\n{xml}\n", xml.len()).as_bytes()).is_ok(),
            Some("READ") => {
                let key = attr_key(&words[1..]);
                let value = {
                    let g = state.lock().expect("lock");
                    g.get(&key).map(str::to_string)
                }
                .unwrap_or_else(|| default_attr(&key).to_string());
                let mut payload = value.into_bytes();
                payload.push(0);
                writer
                    .write_all(format!("{}\n", payload.len()).as_bytes())
                    .and_then(|()| writer.write_all(&payload))
                    .and_then(|()| writer.write_all(b"\n"))
                    .is_ok()
            }
            Some("WRITE") => {
                // The trailing count is the payload length; everything before
                // it names the attribute.
                let len: usize = words.last().and_then(|s| s.parse().ok()).unwrap_or(0);
                let key = attr_key(&words[1..words.len() - 1]);
                let mut payload = vec![0u8; len];
                if reader.read_exact(&mut payload).is_err() {
                    break;
                }
                let value =
                    String::from_utf8_lossy(&payload).trim_end_matches('\0').trim().to_string();
                // The receive gain register belongs to the AD9361 unless the
                // gain-control mode is manual; writing it in an attack mode is
                // `-EOPNOTSUPP`, not a silently ignored write.
                //
                // Per *chain*, as the part is: a 2R2T board has a register set
                // and a gain-control mode for each, and putting the first one
                // in manual says nothing about the second (issue #311).
                if key.starts_with("ad9361-phy/INPUT/voltage")
                    && key.ends_with("/hardwaregain")
                    && state
                        .lock()
                        .expect("lock")
                        .get(&key.replace("hardwaregain", "gain_control_mode"))
                        != Some("manual")
                {
                    let _ = writer.write_all(b"-95\n");
                    if writer.flush().is_err() {
                        break;
                    }
                    continue;
                }
                let rate = key.ends_with("/sampling_frequency");
                // The AD9361 clock-chain rule: a rate at or below the printed
                // floor cannot be realised, however confidently the device
                // published that floor. Refused, and not recorded — the
                // register does not move.
                if rate && value.parse::<i64>().is_ok_and(|v| v <= AD9361_RATE_FLOOR) {
                    writer.write_all(b"-22\n").is_ok()
                } else {
                    let mut g = state.lock().expect("lock");
                    // Receive and transmit hang off one clock tree, so setting
                    // either rate moves both. Modelled here because the
                    // transmit interpolator is sized from the read-back value,
                    // and a fake that let the two drift would bless a client
                    // that transmits on the wrong frequency.
                    if rate {
                        g.attrs.push((
                            "ad9361-phy/OUTPUT/voltage0/sampling_frequency".to_string(),
                            value.clone(),
                        ));
                    }
                    g.attrs.push((key, value));
                    drop(g);
                    writer.write_all(format!("{len}\n").as_bytes()).is_ok()
                }
            }
            Some("OPEN") => {
                // `OPEN <dev> <samples> <mask> [CYCLIC]` — keep the mask, so a
                // test can assert which scan elements the client enabled.
                let mask = words.get(3).unwrap_or(&"").to_string();
                let mut g = state.lock().expect("lock");
                if words[1] == "iio:device3" {
                    g.rx_buffer_opens += 1;
                    g.rx_buffer_open = true;
                    // What a reopen is *for*: the DMA that was producing
                    // nothing starts again. The buffer this radio opened on the
                    // way up is the wedged one, so it takes a *second* open.
                    if g.rx_buffer_opens > 1 {
                        g.wedged_until_reopen = false;
                    }
                    g.rx_open_masks.push(mask);
                } else {
                    g.tx_buffer_opens += 1;
                    g.tx_buffer_open = true;
                    g.tx_open_masks.push(mask);
                }
                drop(g);
                writer.write_all(b"0\n").is_ok()
            }
            Some("CLOSE") => {
                let mut g = state.lock().expect("lock");
                if words[1] == "iio:device3" {
                    g.rx_buffer_open = false;
                } else {
                    g.tx_buffer_open = false;
                }
                drop(g);
                writer.write_all(b"0\n").is_ok()
            }
            Some("READBUF") => {
                // A dual-pair mask gets four-element sample sets, the second
                // pair carrying its own pattern; anything else gets the stock
                // two-element sets.
                let mask = {
                    let g = state.lock().expect("lock");
                    if !g.rx_buffer_open {
                        drop(g);
                        let _ = writer.write_all(b"-9\n");
                        continue;
                    }
                    if g.wedged_until_reopen {
                        drop(g);
                        // -EAGAIN: the server is alive and the device has
                        // nothing. Paced, because a real `iiod` holds this
                        // answer back for its whole device timeout.
                        std::thread::sleep(Duration::from_millis(20));
                        let _ = writer.write_all(b"-11\n");
                        let _ = writer.flush();
                        continue;
                    }
                    g.rx_open_masks.last().cloned().unwrap_or_else(|| "00000003".into())
                };
                let dual = mask == "0000000f";
                let set: Vec<u8> = if dual {
                    [SAMPLE_BYTES, SAMPLE2_BYTES].concat()
                } else {
                    SAMPLE_BYTES.to_vec()
                };
                let want: usize = words[2].parse().unwrap_or(0);
                let sets = want / set.len();
                let mut data = Vec::with_capacity(sets * set.len());
                for _ in 0..sets {
                    data.extend_from_slice(&set);
                }
                // Deliberately answered in two chunks, each with its own mask,
                // because that is the shape the protocol allows and the client
                // has to survive it.
                let split = (data.len() / 2) & !3;
                let stall = {
                    let mut g = state.lock().expect("lock");
                    std::mem::take(&mut g.stall_next_readbuf)
                };
                let ok = if let Some(how_long) = stall {
                    // The same bytes in the same order — only the timing
                    // differs. The pause falls *inside* the first chunk's
                    // payload, which is the position that matters: a read that
                    // gives up there has consumed an unknown number of bytes
                    // and cannot simply be retried from the top.
                    stalled_chunk(&mut writer, &data[..split], &mask, how_long)
                        && write_chunk(&mut writer, &data[split..], &mask)
                } else {
                    write_chunk(&mut writer, &data[..split], &mask)
                        && write_chunk(&mut writer, &data[split..], &mask)
                };
                // A terminating zero only when the request was NOT fully
                // satisfied — a real iiod stops silently once it has sent the
                // requested bytes, and the client stops reading there too.
                // Always appending one (as this fake once did) left the
                // client one status line behind, which surfaced the moment a
                // close/reopen transition changed the framing underneath it.
                if ok && data.len() < want {
                    let _ = writer.write_all(b"0\n");
                }
                ok
            }
            Some("WRITEBUF") => {
                let len: usize = words[2].parse().unwrap_or(0);
                if writer.write_all(b"0\n").is_err() {
                    break;
                }
                let mut payload = vec![0u8; len];
                if reader.read_exact(&mut payload).is_err() {
                    break;
                }
                state.lock().expect("lock").tx_payloads.push(payload);
                writer.write_all(format!("{len}\n").as_bytes()).is_ok()
            }
            Some("EXIT") => break,
            _ => writer.write_all(b"-22\n").is_ok(),
        };
        if !ok || writer.flush().is_err() {
            break;
        }
    }
    let _ = writer.shutdown(Shutdown::Both);
}

fn write_chunk(writer: &mut TcpStream, data: &[u8], mask: &str) -> bool {
    writer
        .write_all(format!("{}\n{mask}\n", data.len()).as_bytes())
        .and_then(|()| writer.write_all(data))
        .is_ok()
}

/// The same chunk, with the device going quiet part-way through its payload for
/// `how_long` — longer than one socket poll interval either way.
fn stalled_chunk(writer: &mut TcpStream, data: &[u8], mask: &str, how_long: Duration) -> bool {
    let half = (data.len() / 2) & !3;
    let sent = writer
        .write_all(format!("{}\n{mask}\n", data.len()).as_bytes())
        .and_then(|()| writer.write_all(&data[..half]))
        .and_then(|()| writer.flush())
        .is_ok();
    if !sent {
        return false;
    }
    // Longer than one socket poll either way, so the read really does come back
    // empty-handed rather than being absorbed by timing luck.
    std::thread::sleep(how_long);
    writer.write_all(&data[half..]).is_ok()
}

/// `["iio:device0", "INPUT", "voltage0", "hardwaregain"]` →
/// `"ad9361-phy/INPUT/voltage0/hardwaregain"`, so the fake keys on names a
/// reader recognises rather than on `iio:deviceN`.
fn attr_key(words: &[&str]) -> String {
    let mut parts: Vec<String> = words.iter().map(|s| s.to_string()).collect();
    if let Some(first) = parts.first_mut() {
        *first = match first.as_str() {
            "iio:device0" => "ad9361-phy".into(),
            "iio:device3" => "cf-ad9361-lpc".into(),
            "iio:device4" => "cf-ad9361-dds-core-lpc".into(),
            other => other.to_string(),
        };
    }
    parts.join("/")
}

/// A mid-buffer gap short enough that the client should wait it out on the
/// socket it already has.
///
/// Over the one-second socket poll, so the read genuinely comes back empty and
/// the retry loop is what carries it; under the payload deadline, so the answer
/// under test is patience rather than the reconnect below.
const HICCUP: Duration = Duration::from_millis(1_200);

/// A mid-buffer gap long enough that the client should stop waiting and replace
/// the receive socket.
///
/// Six seconds is chosen against the field report: a LibreSDR over a saturated
/// link went quiet mid-payload while `iiod` on the same board answered a fresh
/// connection in six milliseconds. Waiting it out was eight seconds of dead
/// audio for a fault a reconnect clears in fifty milliseconds.
const STALL: Duration = Duration::from_secs(6);

fn wait_for(what: &str, cond: impl FnMut() -> bool) {
    wait_up_to(Duration::from_secs(5), what, cond);
}

fn wait_up_to(how_long: Duration, what: &str, mut cond: impl FnMut() -> bool) {
    let deadline = Instant::now() + how_long;
    while Instant::now() < deadline {
        if cond() {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("timed out waiting for {what}");
}

/// A rate a real AD9361 can actually produce — above the 2.083 Msps floor it
/// has whenever the FIR decimator is bypassed, which is how a stock Pluto
/// comes.
const RATE: f64 = 2_500_000.0;

fn config() -> PlutoConfig {
    PlutoConfig {
        sample_rate_hz: RATE,
        rx_gain_db: 40.0,
        agc: PlutoAgc::Manual,
        tx_gain_db: -30.0,
        buffer_samples: 4096,
        ..PlutoConfig::default()
    }
}

#[test]
fn a_pluto_comes_up_and_streams() {
    let fake = Fake::start();
    let mut handle =
        PlutoHandle::open(&fake.address(), &config(), 435_000_000.0).expect("open the fake Pluto");

    assert_eq!(handle.sample_rate_hz, RATE);
    assert!(handle.model.contains("AD9364"));
    assert_eq!(handle.firmware, "v0.39");
    // The limits are the ones the device published, not the fallback table.
    assert_eq!(handle.limits.rx_lo_hz, (70e6, 6e9));
    assert_eq!(handle.limits.rx_gain_db, (0.0, 71.0, 1.0));
    assert_eq!(handle.limits.tx_gain_db, (-89.75, 0.0, 0.25));
    assert_eq!(handle.limits.rx_ports, vec!["A_BALANCED", "B_BALANCED"]);
    assert!(handle.limits.assumed.is_empty(), "{:?}", handle.limits.assumed);
    // Everything came from the device, so there is nothing to warn about.
    assert_eq!(handle.open_status(), None);

    // Three connections: control, receive, transmit.
    assert_eq!(fake.connections.load(Ordering::Relaxed), 3);

    let mut buf = vec![0f32; 4096];
    let mut got = 0;
    wait_for("samples", || {
        got = handle.rx_read(&mut buf);
        got > 0
    });
    assert_eq!(got % 2, 0, "an odd count would swap I and Q for good");
    // The hand-written wire bytes decode to positive and negative full scale.
    assert!((buf[0] - (SAMPLE_I as f32 / 2048.0)).abs() < 1e-6, "I was {}", buf[0]);
    assert!((buf[1] - (SAMPLE_Q as f32 / 2048.0)).abs() < 1e-6, "Q was {}", buf[1]);
    assert!((buf[1] - -1.0).abs() < 1e-6);

    handle.release();
}

/// The Pluto+ field failure: 2R2T firmware puts four scan elements on each DMA
/// buffer — two chains' worth of I/Q — and this driver used to refuse the whole
/// radio over it ("has 4 scan elements, not the 2 (I and Q) this driver
/// decodes"). The first pair is a perfectly good radio, and `OPEN`'s channel
/// mask exists to enable a subset: the buffer must come up with bits 0 and 1
/// set and nothing else, receive the first chain, and transmit on it too.
#[test]
fn a_2r2t_pluto_plus_streams_its_first_pair_and_leaves_the_second_disabled() {
    let fake = Fake::start_pluto_plus();
    let mut handle = PlutoHandle::open(&fake.address(), &config(), 435_000_000.0)
        .expect("a 2R2T Pluto+ must open");

    let mut buf = vec![0f32; 4096];
    let mut got = 0;
    wait_for("samples", || {
        got = handle.rx_read(&mut buf);
        got > 0
    });
    // Two-element sample sets, decoded exactly as on a stock Pluto — only the
    // enabled pair travels the wire.
    assert!((buf[0] - (SAMPLE_I as f32 / 2048.0)).abs() < 1e-6, "I was {}", buf[0]);
    assert!((buf[1] - (SAMPLE_Q as f32 / 2048.0)).abs() < 1e-6, "Q was {}", buf[1]);

    handle.tx_begin(435_000_000.0);
    let iq: Vec<f32> = (0..8192).map(|i| if i % 2 == 0 { 1.0 } else { 0.0 }).collect();
    handle.tx_write(&iq);
    wait_for("a transmit buffer", || !fake.state.lock().unwrap().tx_payloads.is_empty());

    {
        let g = fake.state.lock().expect("lock");
        assert!(!g.rx_open_masks.is_empty());
        for mask in g.rx_open_masks.iter().chain(&g.tx_open_masks) {
            assert_eq!(mask, "00000003", "the second pair must stay disabled");
        }
        // The encoder writes two-element sets as well: full-scale I, silent Q.
        assert_eq!(
            &g.tx_payloads[0][..4],
            &[0xFF, 0x7F, 0x00, 0x00],
            "first sample was {:?}",
            &g.tx_payloads[0][..4]
        );
    }

    // A report from such a device has to say what was set aside (the second
    // transmit pair; both receive pairs are kept available for streams).
    let trace = handle.trace().dump();
    assert!(trace.contains("dual-channel (2R2T)"), "{trace}");
    assert!(trace.contains("voltage2, voltage3 aside"), "{trace}");

    handle.tx_end();
    handle.release();
}

/// The field failure this guards against: the operator picks a rate at or below
/// the device's floor, sdroxide clamps it up to the 2083333 the device
/// published, and the device answers `-EINVAL` to the very number it just
/// printed. Nothing after that write ever ran — no buffer was opened, no sample
/// arrived, and the screen stayed on "waiting for spectrum" while the radio sat
/// there perfectly reachable.
#[test]
fn a_rate_floor_the_device_prints_but_will_not_accept_is_still_reached() {
    let fake = Fake::start();
    let cfg = PlutoConfig { sample_rate_hz: 1_000_000.0, ..config() };
    let handle = PlutoHandle::open(&fake.address(), &cfg, 435_000_000.0)
        .expect("a device that refuses its own published floor must still come up");

    // One hertz above the published floor: the smallest rate actually inside
    // the real range, and what the hardware settles on.
    assert_eq!(handle.sample_rate_hz, (AD9361_RATE_FLOOR + 1) as f64);
    let g = fake.state.lock().expect("lock");
    assert_eq!(g.get("ad9361-phy/INPUT/voltage0/sampling_frequency"), Some("2083334"));
    drop(g);
    // The rate asked for is not the rate delivered — twice over — so the open
    // has to say so rather than let it pass as the operator's own setting.
    let status = handle.open_status().expect("a raised rate is worth a notice");
    assert!(status.contains("1.000 Msps requested"), "{status}");
    assert!(status.contains("2.083 Msps"), "{status}");
}

/// The AD9361 owns the receive gain register in every mode but manual, and
/// answers `-EOPNOTSUPP` to a write rather than ignoring it. Since slow attack
/// is the default AGC, sending the gain unconditionally aborted the open on any
/// radio the operator had not pinned to manual — and pinning it to manual is
/// what left the receiver at a fixed 40 dB of an available 71, looking deaf.
#[test]
fn an_attack_mode_does_not_take_the_gain_write_that_would_abort_the_open() {
    let fake = Fake::start();
    let cfg = PlutoConfig { agc: PlutoAgc::SlowAttack, rx_gain_db: 40.0, ..config() };
    let handle = PlutoHandle::open(&fake.address(), &cfg, 435_000_000.0)
        .expect("the default AGC must not abort the open");

    let g = fake.state.lock().expect("lock");
    assert_eq!(g.get("ad9361-phy/INPUT/voltage0/gain_control_mode"), Some("slow_attack"));
    assert!(
        g.writes_of("ad9361-phy/INPUT/voltage0/hardwaregain").is_empty(),
        "the gain must not be written while an attack mode owns it"
    );
    drop(g);

    // Switching to manual replays the held gain, so the operator's setting is
    // deferred rather than discarded.
    handle.set_agc_mode("manual");
    wait_for("the held gain to be replayed", || {
        fake.state.lock().unwrap().get("ad9361-phy/INPUT/voltage0/hardwaregain").is_some()
    });
    let g = fake.state.lock().expect("lock");
    assert_eq!(g.get("ad9361-phy/INPUT/voltage0/gain_control_mode"), Some("manual"));
    assert!(
        g.get("ad9361-phy/INPUT/voltage0/hardwaregain").unwrap().starts_with("40"),
        "{:?}",
        g.get("ad9361-phy/INPUT/voltage0/hardwaregain")
    );
}

/// A gap in the data is not a broken connection. The socket's receive timeout
/// fires on an ordinary stall, and letting it through ended the stream and
/// forced a full reopen — every one to five minutes, on a link with no errors
/// and no dropped packets.
#[test]
fn a_gap_in_the_middle_of_a_buffer_does_not_end_the_stream() {
    let fake = Fake::start_that_hiccups_mid_buffer();
    let mut handle =
        PlutoHandle::open(&fake.address(), &config(), 435_000_000.0).expect("open the fake Pluto");

    let mut buf = vec![0f32; 4096];
    let mut got = 0;
    wait_up_to(HICCUP + Duration::from_secs(5), "samples across the gap", || {
        got = handle.rx_read(&mut buf);
        got > 0
    });
    assert!(handle.is_alive(), "a gap must not take the connection down");
    // On the same socket: three connections is what the open made, and waiting
    // out a gap this short must not have cost a fourth. This is the half of the
    // behaviour the reconnect below could otherwise quietly swallow — a client
    // that redialled on every hiccup would still deliver samples and still pass
    // every other assertion here.
    assert_eq!(
        fake.connections.load(Ordering::Relaxed),
        3,
        "a gap shorter than the payload deadline must be waited out, not redialled around"
    );
    // The samples either side of the gap are still the ones the device sent,
    // in the right order — a retry that lost its place would interleave I and Q.
    assert!((buf[0] - (SAMPLE_I as f32 / 2048.0)).abs() < 1e-6, "I was {}", buf[0]);
    assert!((buf[1] - (SAMPLE_Q as f32 / 2048.0)).abs() < 1e-6, "Q was {}", buf[1]);
    handle.release();
}

/// A gap too long to wait out costs one socket, not the whole radio.
///
/// The fault this is drawn from wedged a single TCP connection mid-payload for
/// eight seconds while `iiod` on the same board answered a fresh connection in
/// six milliseconds — so the board was never gone, and taking the rig down to
/// redial it from scratch (context XML, whole front end, source swap) added a
/// second of dead audio to a stall that was already the complaint. Replacing
/// the one socket that failed is tens of milliseconds, and it leaves the dial,
/// the gains and any transmission in progress alone.
#[test]
fn a_gap_too_long_to_wait_out_replaces_the_socket_and_keeps_the_radio() {
    let fake = Fake::start_that_stalls_mid_buffer();
    let mut handle =
        PlutoHandle::open(&fake.address(), &config(), 435_000_000.0).expect("open the fake Pluto");
    wait_for("the receive buffer", || fake.state.lock().unwrap().rx_buffer_open);

    let mut buf = vec![0f32; 4096];
    let mut got = 0;
    wait_up_to(STALL + Duration::from_secs(5), "samples after the socket was replaced", || {
        got = handle.rx_read(&mut buf);
        got > 0
    });
    assert!(handle.is_alive(), "the board was never gone, and the connection must survive");
    let connections = fake.connections.load(Ordering::Relaxed);
    assert!(
        connections >= 4,
        "the receive socket should have been replaced; the fake saw {connections} connection(s)"
    );
    let opens = fake.state.lock().expect("lock").rx_buffer_opens;
    assert!(opens >= 2, "the buffer should have been reopened, opened {opens} time(s)");
    assert!((buf[0] - (SAMPLE_I as f32 / 2048.0)).abs() < 1e-6, "I was {}", buf[0]);
    assert!((buf[1] - (SAMPLE_Q as f32 / 2048.0)).abs() < 1e-6, "Q was {}", buf[1]);

    // And the control connection was never in the blast radius: a retune still
    // reaches the oscillator afterwards, on the socket it was always on.
    handle.set_rx_freq(144_200_000.0);
    wait_for("a retune after the receive socket was replaced", || {
        fake.state.lock().unwrap().get("ad9361-phy/OUTPUT/altvoltage0/frequency")
            == Some("144200000")
    });
    handle.release();
}

/// A device that answers but produces nothing gets its buffer reopened, and the
/// stream comes back on the connection we already have.
///
/// This is the other half of the stall above, and the opposite treatment is
/// right for it. There the link hiccups and reading again is all that is
/// needed; here the server keeps answering `-EAGAIN` from a buffer that will
/// never produce another sample, and reading again is the one thing that cannot
/// help. Without the reopen the only way out is the engine's silence watchdog
/// tearing the radio down and dialling it again — seconds of dead air, and a
/// "reconnecting" line in the log that names the symptom rather than the fault.
#[test]
fn a_buffer_that_stops_producing_is_reopened_rather_than_read_again() {
    let fake = Fake::start_wedged_until_reopened();
    let mut handle =
        PlutoHandle::open(&fake.address(), &config(), 435_000_000.0).expect("open the fake Pluto");

    let mut buf = vec![0f32; 4096];
    let mut got = 0;
    // The client waits out its patience window (a second, at this buffer and
    // rate) before deciding the buffer is wedged rather than merely quiet.
    wait_up_to(Duration::from_secs(6), "samples after the buffer was reopened", || {
        got = handle.rx_read(&mut buf);
        got > 0
    });
    assert!(handle.is_alive(), "the connection was never the problem, and must survive");
    let opens = fake.state.lock().expect("lock").rx_buffer_opens;
    assert!(opens >= 2, "the buffer should have been reopened, opened {opens} time(s)");
    // And what came back is a real sample set, not a resynchronised guess.
    assert!((buf[0] - (SAMPLE_I as f32 / 2048.0)).abs() < 1e-6, "I was {}", buf[0]);
    assert!((buf[1] - (SAMPLE_Q as f32 / 2048.0)).abs() < 1e-6, "Q was {}", buf[1]);
    handle.release();
}

/// Closing a radio must not wait for a read that is in the middle of a stall.
///
/// `release` runs on the engine thread, ahead of opening the replacement, and
/// it joins the three threads that live inside reads. Without shutting their
/// sockets down first, a stalled link makes that join take as long as the read
/// deadline — seconds of frozen audio and an unresponsive window at exactly the
/// moment the operator is trying to recover. It never showed over USB, where
/// reads return every few milliseconds.
#[test]
fn releasing_does_not_wait_out_a_stalled_read() {
    let fake = Fake::start_that_stalls_mid_buffer();
    let mut handle =
        PlutoHandle::open(&fake.address(), &config(), 435_000_000.0).expect("open the fake Pluto");
    // Long enough to be inside the stalled read, far short of its end.
    std::thread::sleep(Duration::from_millis(300));

    let started = Instant::now();
    handle.release();
    let took = started.elapsed();
    assert!(
        took < STALL / 2,
        "release waited {took:?} for a read stalled {STALL:?} — it must break the read, \
         not outlive it"
    );
}

/// `rf_port_select` is refused while the receive buffer is running, and the
/// engine re-asserts the antenna on every retune. A radio with one wired port
/// therefore logged a rejected write each time the dial moved, for a change
/// that was never a change.
#[test]
fn selecting_the_port_already_selected_writes_nothing() {
    let fake = Fake::start();
    let mut handle =
        PlutoHandle::open(&fake.address(), &config(), 435_000_000.0).expect("open the fake Pluto");
    let already = handle.rx_port().to_string();
    assert_eq!(already, "A_BALANCED");

    let tx_already = handle.tx_port().to_string();
    handle.set_rx_port(&already);
    handle.set_tx_port(&tx_already);
    std::thread::sleep(Duration::from_millis(50));
    let g = fake.state.lock().expect("lock");
    assert!(g.writes_of("ad9361-phy/INPUT/voltage0/rf_port_select").is_empty(), "no-op RX port");
    assert!(g.writes_of("ad9361-phy/OUTPUT/voltage0/rf_port_select").is_empty(), "no-op TX port");
    drop(g);

    // A real change still goes through.
    handle.set_rx_port("B_BALANCED");
    wait_for("the port change", || {
        !fake.state.lock().unwrap().writes_of("ad9361-phy/INPUT/voltage0/rf_port_select").is_empty()
    });
}

#[test]
fn the_front_end_is_configured_in_an_order_that_works() {
    let fake = Fake::start();
    let handle =
        PlutoHandle::open(&fake.address(), &config(), 435_000_000.0).expect("open the fake Pluto");
    wait_for("the receive buffer", || fake.state.lock().unwrap().rx_buffer_open);
    drop(handle);

    let g = fake.state.lock().expect("lock");
    // The analog filter is opened up to 0.9 × the rate, so the engine's
    // quarter-span LO offset still clears it. Leaving the AD9361 default here
    // is what makes that offset silently do nothing.
    assert_eq!(g.get("ad9361-phy/INPUT/voltage0/rf_bandwidth"), Some("2250000"));
    assert_eq!(g.get("ad9361-phy/OUTPUT/voltage0/rf_bandwidth"), Some("2250000"));
    assert_eq!(g.get("ad9361-phy/INPUT/voltage0/sampling_frequency"), Some("2500000"));
    assert_eq!(g.get("ad9361-phy/INPUT/voltage0/gain_control_mode"), Some("manual"));
    assert_eq!(g.get("ad9361-phy/OUTPUT/altvoltage0/frequency"), Some("435000000"));

    // The transmitter is silenced before the operator's level is applied, so
    // whatever the previous program left in the attenuator is never live.
    let tx_gains = g.writes_of("ad9361-phy/OUTPUT/voltage0/hardwaregain");
    assert_eq!(tx_gains.len(), 2, "{tx_gains:?}");
    assert!(tx_gains[0].starts_with("-89.75"), "{tx_gains:?}");
    assert!(tx_gains[1].starts_with("-30"), "{tx_gains:?}");
}

#[test]
fn transmitting_takes_the_link_and_gives_it_back() {
    let fake = Fake::start();
    let mut handle =
        PlutoHandle::open(&fake.address(), &config(), 145_500_000.0).expect("open the fake Pluto");
    wait_for("the receive buffer", || fake.state.lock().unwrap().rx_buffer_open);

    let rate = handle.tx_begin(145_500_000.0);
    assert_eq!(rate, RATE);

    // Full-scale I, silence in Q — a tone this crate's encoder has to put on
    // the wire as +32767 in the I slot.
    let iq: Vec<f32> = (0..8192).map(|i| if i % 2 == 0 { 1.0 } else { 0.0 }).collect();
    handle.tx_write(&iq);

    wait_for("a transmit buffer", || !fake.state.lock().unwrap().tx_payloads.is_empty());

    {
        let g = fake.state.lock().expect("lock");
        // Receive let go of the link before transmit took it.
        assert!(!g.rx_buffer_open, "receive was still running during the over");
        assert!(g.tx_buffer_open);
        // The DDS tone generators are silenced, or the Pluto transmits a
        // carrier pair instead of the modulation — at full power, on frequency.
        assert_eq!(g.get("cf-ad9361-dds-core-lpc/OUTPUT/altvoltage0/raw"), Some("0"));
        assert_eq!(g.get("cf-ad9361-dds-core-lpc/OUTPUT/altvoltage1/raw"), Some("0"));
        assert_eq!(g.get("ad9361-phy/OUTPUT/altvoltage1/frequency"), Some("145500000"));
        // 16-bit little-endian, full scale in I and nothing in Q.
        let payload = &g.tx_payloads[0];
        assert_eq!(
            &payload[..4],
            &[0xFF, 0x7F, 0x00, 0x00],
            "first sample was {:?}",
            &payload[..4]
        );
    }

    handle.tx_end();
    wait_for("receive to resume", || fake.state.lock().unwrap().rx_buffer_open);
    {
        let g = fake.state.lock().expect("lock");
        assert!(!g.tx_buffer_open, "transmit was still running after the over");
        // The receive oscillator is put back where the dial is.
        assert_eq!(g.get("ad9361-phy/OUTPUT/altvoltage0/frequency"), Some("145500000"));
        assert!(g.rx_buffer_opens >= 2, "receive should have reopened");
    }

    handle.release();
}

/// Full duplex is the same over with the arbitration taken out: both buffers
/// hold the link at once, receive is never closed, and the receive oscillator
/// is left exactly where it was.
///
/// That last one is the part worth a test. Half duplex rewrites the receive LO
/// at every unkey, on the chance that the two directions share a synthesiser;
/// doing that to a receiver that is running would put a gap in it every time
/// the operator let go of PTT.
#[test]
fn full_duplex_receives_through_an_over() {
    let fake = Fake::start();
    let cfg = PlutoConfig { full_duplex: true, ..config() };
    let mut handle =
        PlutoHandle::open(&fake.address(), &cfg, 145_500_000.0).expect("open the fake Pluto");
    wait_for("the receive buffer", || fake.state.lock().unwrap().rx_buffer_open);
    let opens_before = fake.state.lock().expect("lock").rx_buffer_opens;
    // Where the receive oscillator was left by the tune-up, so a later write
    // shows as a change rather than as an equal value.
    handle.set_rx_freq(145_400_000.0);
    wait_for("the retune", || {
        fake.state.lock().unwrap().get("ad9361-phy/OUTPUT/altvoltage0/frequency")
            == Some("145400000")
    });
    let lo_writes =
        fake.state.lock().expect("lock").writes_of("ad9361-phy/OUTPUT/altvoltage0/frequency").len();

    handle.tx_begin(2_400_050_000.0);
    let iq: Vec<f32> = (0..8192).map(|i| if i % 2 == 0 { 1.0 } else { 0.0 }).collect();
    handle.tx_write(&iq);
    wait_for("a transmit buffer", || !fake.state.lock().unwrap().tx_payloads.is_empty());

    // Samples keep arriving while the transmitter is running — the whole point.
    let mut buf = vec![0f32; 4096];
    let mut got = 0;
    wait_for("receive samples during the over", || {
        got = handle.rx_read(&mut buf);
        got > 0
    });
    {
        let g = fake.state.lock().expect("lock");
        assert!(g.rx_buffer_open, "receive must hold the link through the over");
        assert!(g.tx_buffer_open, "and transmit must have it as well");
        assert_eq!(g.rx_buffer_opens, opens_before, "the receive buffer was never closed");
        // Transmit went to its own oscillator, receive stayed on the dial.
        assert_eq!(g.get("ad9361-phy/OUTPUT/altvoltage1/frequency"), Some("2400050000"));
        assert_eq!(g.get("ad9361-phy/OUTPUT/altvoltage0/frequency"), Some("145400000"));
    }

    handle.tx_end();
    wait_for("the transmit buffer to close", || !fake.state.lock().unwrap().tx_buffer_open);
    {
        let g = fake.state.lock().expect("lock");
        assert!(g.rx_buffer_open, "receive was never interrupted, so it is still open");
        assert_eq!(g.rx_buffer_opens, opens_before);
        assert_eq!(
            g.writes_of("ad9361-phy/OUTPUT/altvoltage0/frequency").len(),
            lo_writes,
            "unkeying must not rewrite the receive oscillator of a running receiver"
        );
    }

    handle.release();
}

/// A board whose enable state machine is in TDD cannot do both directions at
/// once whatever its link can carry, so asking for full duplex there earns a
/// sentence on screen rather than a silent half-duplex over.
#[test]
fn full_duplex_on_a_tdd_board_says_so() {
    let fake = Fake::start_with(
        DeviceState {
            attrs: vec![("ad9361-phy/ensm_mode".to_string(), "tdd".to_string())],
            ..DeviceState::default()
        },
        CONTEXT_XML.to_string(),
    );
    let cfg = PlutoConfig { full_duplex: true, ..config() };
    let handle = PlutoHandle::open(&fake.address(), &cfg, 145_500_000.0).expect("it still opens");
    let status = handle.open_status().unwrap_or_default();
    assert!(status.contains("tdd"), "the mode should be named, got {status:?}");
    assert!(status.contains("full duplex"), "and what it collides with, got {status:?}");
}

/// The GPO transmit-receive switching, end to end: the device-tree properties
/// are written and committed *before* the front end is configured, and the
/// enable state machine then follows the operator's key.
///
/// The ordering assertion is the one worth having. `initialize` re-runs the
/// AD9361's whole setup from those properties, so a driver that set the sample
/// rate first would have it thrown away — and the symptom is not an error but a
/// radio quietly running at the board's default rate.
#[test]
fn the_gpo_pins_are_slaved_to_the_radio_and_keyed_by_an_over() {
    let fake = Fake::start();
    let cfg = PlutoConfig {
        duplex: sdroxide_types::PlutoDuplex::Tdd,
        ptt_gpo: sdroxide_types::PlutoPtt::Gpo23,
        ..config()
    };
    let mut handle =
        PlutoHandle::open(&fake.address(), &cfg, 145_500_000.0).expect("open the fake Pluto");
    wait_for("the receive buffer", || fake.state.lock().unwrap().rx_buffer_open);

    {
        let g = fake.state.lock().expect("lock");
        let debug = |attr: &str| g.get(&format!("ad9361-phy/DEBUG/{attr}")).map(str::to_string);
        assert_eq!(debug("adi,frequency-division-duplex-mode-enable").as_deref(), Some("0"));
        assert_eq!(debug("adi,gpo2-slave-rx-enable").as_deref(), Some("1"));
        assert_eq!(debug("adi,gpo3-slave-tx-enable").as_deref(), Some("1"));
        assert_eq!(debug("initialize").as_deref(), Some("1"));
        // Every other slave bit is cleared, not merely left: a pair chosen in
        // an earlier session is still slaved, and two pairs following the
        // radio would key two amplifiers.
        for attr in [
            "adi,gpo0-slave-rx-enable",
            "adi,gpo0-slave-tx-enable",
            "adi,gpo1-slave-rx-enable",
            "adi,gpo1-slave-tx-enable",
            "adi,gpo2-slave-tx-enable",
            "adi,gpo3-slave-rx-enable",
        ] {
            assert_eq!(debug(attr).as_deref(), Some("0"), "{attr}");
        }
        // An external LNA on GPO0/GPO1 is none of this pair's business.
        assert_eq!(debug("adi,elna-rx1-gpo0-control-enable"), None);

        let at = |key: &str| {
            g.attrs
                .iter()
                .position(|(k, _)| k == key)
                .unwrap_or_else(|| panic!("{key} was never written"))
        };
        assert!(
            at("ad9361-phy/DEBUG/initialize") < at("ad9361-phy/INPUT/voltage0/sampling_frequency"),
            "the commit has to come before the front end is set up, or it undoes it"
        );
        assert!(
            at("ad9361-phy/DEBUG/adi,gpo2-slave-rx-enable") < at("ad9361-phy/DEBUG/initialize"),
            "the pins have to be written before the commit that makes them take"
        );
        // Nothing receives in TDD until the state machine is told to.
        assert_eq!(g.get("ad9361-phy/ensm_mode"), Some("rx"));
    }

    handle.tx_begin(145_500_000.0);
    let iq: Vec<f32> = vec![0.0; 4096];
    handle.tx_write(&iq);
    wait_for("the transmit state", || {
        fake.state.lock().unwrap().get("ad9361-phy/ensm_mode") == Some("tx")
    });
    {
        let g = fake.state.lock().expect("lock");
        let states = g.writes_of("ad9361-phy/ensm_mode");
        // Keyed before the buffer opens: the transmit path is off until the
        // state machine moves, and the amplifier should be switched in ahead
        // of the signal rather than behind it.
        assert_eq!(states, vec!["rx", "tx"], "{states:?}");
    }

    handle.tx_end();
    wait_for("the receive state", || {
        fake.state.lock().unwrap().get("ad9361-phy/ensm_mode") == Some("rx")
    });
    handle.release();
}

/// A GPO pair keys from the enable state machine, so it can only work in TDD —
/// and TDD is one direction at a time however good the link is. Both of those
/// silently disagree with settings the operator may have left behind, so both
/// are overridden and both are said out loud.
#[test]
fn gpo_ptt_wins_over_fdd_and_full_duplex_and_says_so() {
    let fake = Fake::start();
    let cfg = PlutoConfig {
        duplex: sdroxide_types::PlutoDuplex::Fdd,
        ptt_gpo: sdroxide_types::PlutoPtt::Gpo01,
        full_duplex: true,
        ..config()
    };
    let mut handle =
        PlutoHandle::open(&fake.address(), &cfg, 145_500_000.0).expect("open the fake Pluto");
    let status = handle.open_status().unwrap_or_default();
    assert!(status.contains("TDD"), "the override should be named, got {status:?}");
    assert!(status.contains("full duplex"), "and so should the other, got {status:?}");
    wait_for("the receive buffer", || fake.state.lock().unwrap().rx_buffer_open);

    {
        let g = fake.state.lock().expect("lock");
        assert_eq!(g.get("ad9361-phy/DEBUG/adi,frequency-division-duplex-mode-enable"), Some("0"));
        assert_eq!(g.get("ad9361-phy/DEBUG/adi,gpo0-slave-rx-enable"), Some("1"));
        assert_eq!(g.get("ad9361-phy/DEBUG/adi,gpo1-slave-tx-enable"), Some("1"));
        // This pair is Analog Devices' own external-LNA pair, and a pin cannot
        // do both jobs — so the eLNA control is stood down with it.
        assert_eq!(g.get("ad9361-phy/DEBUG/adi,elna-rx1-gpo0-control-enable"), Some("0"));
        assert_eq!(g.get("ad9361-phy/DEBUG/adi,elna-rx2-gpo1-control-enable"), Some("0"));
    }

    // And the over is half duplex whatever the checkbox said.
    handle.tx_begin(145_500_000.0);
    handle.tx_write(&vec![0.0; 4096]);
    wait_for("the over", || !fake.state.lock().unwrap().rx_buffer_open);
    handle.tx_end();
    handle.release();
}

/// Turning the PTT pins back off has to undo them, and the board is the only
/// place that remembers: these are device-tree properties that live in the
/// radio until it is rebooted. A session that walked away from them would leave
/// an operator with a Pluto stuck in TDD, its transmitter disabled because
/// nothing is driving the state machine any more, and nothing on screen saying
/// so.
#[test]
fn a_board_this_left_in_tdd_is_put_back_when_the_pins_are_turned_off() {
    let fake = Fake::start_with(
        DeviceState {
            attrs: vec![
                ("ad9361-phy/ensm_mode".to_string(), "tdd".to_string()),
                ("ad9361-phy/DEBUG/adi,gpo2-slave-rx-enable".to_string(), "1".to_string()),
                ("ad9361-phy/DEBUG/adi,gpo3-slave-tx-enable".to_string(), "1".to_string()),
            ],
            ..DeviceState::default()
        },
        CONTEXT_XML.to_string(),
    );
    let handle =
        PlutoHandle::open(&fake.address(), &config(), 145_500_000.0).expect("open the fake Pluto");
    let status = handle.open_status().unwrap_or_default();
    assert!(status.contains("FDD"), "the undo should be said out loud, got {status:?}");
    wait_for("the receive buffer", || fake.state.lock().unwrap().rx_buffer_open);

    let g = fake.state.lock().expect("lock");
    assert_eq!(g.get("ad9361-phy/DEBUG/adi,frequency-division-duplex-mode-enable"), Some("1"));
    assert_eq!(g.get("ad9361-phy/DEBUG/adi,gpo2-slave-rx-enable"), Some("0"));
    assert_eq!(g.get("ad9361-phy/DEBUG/adi,gpo3-slave-tx-enable"), Some("0"));
    assert_eq!(g.get("ad9361-phy/DEBUG/initialize"), Some("1"));
    // And the state machine is put where FDD belongs, rather than left in
    // whatever `initialize` happened to leave behind.
    assert_eq!(g.get("ad9361-phy/ensm_mode"), Some("fdd"));
}

/// The default is to touch none of this. A Pluto that has never had these
/// settings opened must come up exactly as it booted — somebody else's board
/// may be in TDD for a reason, and its GPO pins may already be driving
/// something this radio knows nothing about.
#[test]
fn a_radio_that_asked_for_neither_is_left_as_it_booted() {
    let fake = Fake::start();
    let handle =
        PlutoHandle::open(&fake.address(), &config(), 145_500_000.0).expect("open the fake Pluto");
    wait_for("the receive buffer", || fake.state.lock().unwrap().rx_buffer_open);
    let g = fake.state.lock().expect("lock");
    let touched: Vec<&str> =
        g.attrs.iter().map(|(k, _)| k.as_str()).filter(|k| k.contains("/DEBUG/")).collect();
    assert!(touched.is_empty(), "the device tree was rewritten unasked: {touched:?}");
    // Nor is the state machine driven on a board that was left in FDD.
    assert!(g.get("ad9361-phy/ensm_mode").is_none());
    drop(g);
    drop(handle);
}

#[test]
fn a_retune_reaches_the_oscillator_while_samples_are_flowing() {
    let fake = Fake::start();
    let handle =
        PlutoHandle::open(&fake.address(), &config(), 100_000_000.0).expect("open the fake Pluto");
    wait_for("the receive buffer", || fake.state.lock().unwrap().rx_buffer_open);

    handle.set_rx_freq(144_200_000.0);
    wait_for("the retune", || {
        fake.state.lock().unwrap().get("ad9361-phy/OUTPUT/altvoltage0/frequency")
            == Some("144200000")
    });

    // ppm is applied in software to the requested frequency, not written to the
    // device's own persistent trim.
    handle.set_ppm(10.0);
    wait_for("the ppm trim", || {
        fake.state.lock().unwrap().get("ad9361-phy/OUTPUT/altvoltage0/frequency")
            == Some("144201442")
    });
    assert!(
        fake.state.lock().unwrap().get("ad9361-phy/xo_correction").is_none(),
        "the device's persistent trim must not be touched"
    );

    drop(handle);
}

/// An IIO device that is not a Pluto has to be named, not driven into nonsense.
#[test]
fn a_non_pluto_iio_device_is_refused_by_name() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    std::thread::spawn(move || {
        let (sock, _) = listener.accept().expect("accept");
        let mut writer = sock.try_clone().expect("clone");
        let mut reader = BufReader::new(sock);
        let xml = r#"<context name="xml" description="x">
            <device id="iio:device0" name="adc081c" />
        </context>"#;
        let mut line = String::new();
        while reader.read_line(&mut line).unwrap_or(0) > 0 {
            let cmd = line.trim().to_string();
            line.clear();
            let _ = match cmd.split_whitespace().next() {
                Some("TIMEOUT") => writer.write_all(b"0\n"),
                Some("VERSION") => writer.write_all(b"0.25 (git tag:v0.25)\n"),
                Some("PRINT") => writer.write_all(format!("{}\n{xml}\n", xml.len()).as_bytes()),
                _ => writer.write_all(b"-22\n"),
            };
            let _ = writer.flush();
        }
    });

    let text = match PlutoHandle::open(&addr.to_string(), &config(), 435_000_000.0) {
        Ok(_) => panic!("an ADC is not a Pluto and must not open as one"),
        Err(e) => e.to_string(),
    };
    assert!(text.contains("not a PlutoSDR"), "{text}");
    assert!(text.contains("adc081c"), "{text}");
}

/// A firmware that publishes no `_available` attributes still has to work — and
/// has to say which figures are guesses rather than quoting them as fact.
#[test]
fn a_silent_firmware_falls_back_and_says_so() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    // The same context with every `_available` attribute removed, and an
    // AD9363 model string.
    let xml = CONTEXT_XML
        .replace("(Z7010-AD9364)", "(Z7010-AD9363)")
        .lines()
        .filter(|l| !l.contains("_available"))
        .collect::<Vec<_>>()
        .join("\n");
    let state = Arc::new(Mutex::new(DeviceState::default()));
    let stop = Arc::new(AtomicBool::new(false));
    {
        let state = Arc::clone(&state);
        let stop = Arc::clone(&stop);
        std::thread::spawn(move || {
            for sock in listener.incoming().take(3) {
                let Ok(sock) = sock else { break };
                let state = Arc::clone(&state);
                let stop = Arc::clone(&stop);
                let xml = xml.clone();
                std::thread::spawn(move || serve_with_xml(sock, state, stop, xml));
            }
        });
    }

    let handle = PlutoHandle::open(&addr.to_string(), &config(), 435_000_000.0)
        .expect("a silent firmware still opens");
    // The AD9363 range, because that is what the model string names.
    assert_eq!(handle.limits.rx_lo_hz, (325_000_000.0, 3_800_000_000.0));
    let status = handle.open_status().expect("a guessed limit has to announce itself");
    assert!(status.contains("assuming"), "{status}");
    assert!(status.contains("tuning range"), "{status}");
    drop(handle);
    stop.store(true, Ordering::Relaxed);
}

/// The same server as [`serve`], with the context document substituted.
fn serve_with_xml(
    sock: TcpStream,
    state: Arc<Mutex<DeviceState>>,
    stop: Arc<AtomicBool>,
    xml: String,
) {
    let _ = sock.set_read_timeout(Some(Duration::from_millis(100)));
    let mut writer = sock.try_clone().expect("clone");
    let mut reader = BufReader::new(sock);
    let mut line = String::new();
    while !stop.load(Ordering::Relaxed) {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                continue;
            }
            Err(_) => break,
        }
        let cmd = line.trim_end_matches(['\r', '\n']).to_string();
        if cmd.is_empty() {
            continue;
        }
        let words: Vec<&str> = cmd.split_whitespace().collect();
        let ok = match words.first().copied() {
            Some("PRINT") => writer.write_all(format!("{}\n{xml}\n", xml.len()).as_bytes()).is_ok(),
            Some("VERSION") => writer.write_all(b"0.25 (git tag:v0.25)\n").is_ok(),
            Some("TIMEOUT") => writer.write_all(b"0\n").is_ok(),
            Some("READ") => {
                let key = attr_key(&words[1..]);
                let value = {
                    let g = state.lock().expect("lock");
                    g.get(&key).map(str::to_string)
                }
                .unwrap_or_else(|| default_attr(&key).to_string());
                let mut payload = value.into_bytes();
                payload.push(0);
                writer
                    .write_all(format!("{}\n", payload.len()).as_bytes())
                    .and_then(|()| writer.write_all(&payload))
                    .and_then(|()| writer.write_all(b"\n"))
                    .is_ok()
            }
            Some("WRITE") => {
                let len: usize = words.last().and_then(|s| s.parse().ok()).unwrap_or(0);
                let key = attr_key(&words[1..words.len() - 1]);
                let mut payload = vec![0u8; len];
                if reader.read_exact(&mut payload).is_err() {
                    break;
                }
                let value =
                    String::from_utf8_lossy(&payload).trim_end_matches('\0').trim().to_string();
                state.lock().expect("lock").attrs.push((key, value));
                writer.write_all(format!("{len}\n").as_bytes()).is_ok()
            }
            Some("OPEN") | Some("CLOSE") => writer.write_all(b"0\n").is_ok(),
            Some("READBUF") | Some("WRITEBUF") => writer.write_all(b"-11\n").is_ok(),
            Some("EXIT") => break,
            _ => writer.write_all(b"-22\n").is_ok(),
        };
        if !ok || writer.flush().is_err() {
            break;
        }
    }
}

/// Phase 4 of the multi-radio work: a 2R2T firmware's two receive chains as
/// two radios on one connection. The deinterleaver must route each chain's
/// samples to its own ring, the one LO must be shared — a retune from either
/// stream moves it and tells the other — and detaching one chain must narrow
/// the buffer back without disturbing the survivor.
#[test]
fn a_2r2t_pluto_serves_both_chains_and_shares_the_lo() {
    use sdroxide_pluto::PlutoRig;

    let fake = Fake::start_pluto_plus();
    let rig = PlutoRig::open(&fake.address(), &config(), 435_000_000.0).expect("open");
    assert_eq!(rig.rx_chains(), 2);

    // A chain the firmware does not stream is refused with the count.
    let err = rig.rx(2).expect_err("chain 3 of 2 must be refused").to_string();
    assert!(err.contains("2 chain"), "the refusal should carry the count: {err}");

    let mut r0 = rig.rx(0).expect("attach chain 0");
    let mut r1 = rig.rx(1).expect("attach chain 1");
    // A chain cannot be attached twice — two engines draining one ring would
    // each get half the samples.
    let err = rig.rx(1).expect_err("duplicate chain 1 must be refused").to_string();
    assert!(err.contains("already"), "{err}");

    // The receive buffer reopens for both pairs…
    wait_for("a dual-pair open", || {
        fake.state.lock().expect("lock").rx_open_masks.last().map(String::as_str)
            == Some("0000000f")
    });
    // …and each chain's pattern lands in its own ring: full scale on chain 0,
    // half scale on chain 1 — a duplicated first pair would fail both.
    let mut buf = vec![0f32; 4096];
    wait_for("chain 0 samples", || {
        let n = r0.rx_read(&mut buf);
        n > 0 && (buf[0] - (SAMPLE_I as f32 / 2048.0)).abs() < 1e-6
    });
    wait_for("chain 1 samples", || {
        let n = r1.rx_read(&mut buf);
        n > 0
            && (buf[0] - (SAMPLE2_I as f32 / 2048.0)).abs() < 1e-6
            && (buf[1] - (SAMPLE2_Q as f32 / 2048.0)).abs() < 1e-6
    });

    // The LO is shared: chain 1 retunes it, the hardware register moves, and
    // chain 0 is told (in engine-domain hertz) so its radio can adopt the
    // new centre.
    r1.set_rx_freq(146_000_000.0);
    wait_for("the LO write", || {
        fake.state.lock().expect("lock").get("ad9361-phy/OUTPUT/altvoltage0/frequency")
            == Some("146000000")
    });
    wait_for("chain 0's notification", || r0.poll_lo_moves().contains(&146_000_000.0));

    // An echo of where the LO already is moves nothing and tells nobody —
    // the dedup that keeps two engines sharing this LO from chasing each
    // other.
    r0.set_rx_freq(146_000_000.0);
    std::thread::sleep(Duration::from_millis(150));
    assert!(
        r1.poll_lo_moves().is_empty(),
        "an echo of the current LO must not come back as a move"
    );

    // Detaching chain 1 narrows the buffer back to one pair, and chain 0
    // streams on.
    drop(r1);
    wait_for("the single-pair reopen", || {
        fake.state.lock().expect("lock").rx_open_masks.last().map(String::as_str)
            == Some("00000003")
    });
    r0.discard_pending_rx();
    wait_for("chain 0 samples after the detach", || {
        let n = r0.rx_read(&mut buf);
        n > 0 && (buf[0] - (SAMPLE_I as f32 / 2048.0)).abs() < 1e-6
    });
    let r1_again = rig.rx(1).expect("re-attach chain 1 after detach");
    drop(r1_again);
    drop(r0);
    rig.release();
}

/// A stock Pluto has one receive chain, and asking for a second must be
/// refused with a message that says what would provide one.
#[test]
fn a_stock_pluto_refuses_a_second_chain() {
    use sdroxide_pluto::PlutoRig;

    let fake = Fake::start();
    let rig = PlutoRig::open(&fake.address(), &config(), 435_000_000.0).expect("open");
    assert_eq!(rig.rx_chains(), 1);
    let err = rig.rx(1).expect_err("a stock Pluto has no chain 2").to_string();
    assert!(err.contains("1 chain"), "{err}");
    assert!(err.contains("2R2T"), "the refusal should say what would provide one: {err}");
    rig.release();
}

/// Issue #311: the gain on a 2R2T board's *second* receiver was refused.
///
/// The control thread tracks a gain-control mode per chain and seeds both from
/// the radio's configuration, but only chain 0 was ever put there — so on a
/// board whose firmware boots chain 1 in an attack mode, sdroxide believed it
/// was in manual, sent the gain, and the driver answered `-EOPNOTSUPP` (-95).
/// The slider moved and nothing happened.
///
/// The `voltage1` control channel this needs is what a real 2R2T firmware
/// exposes and what the fixture now carries; a stock Pluto has none, and there
/// the write is held rather than sent.
#[test]
fn the_second_receivers_gain_reaches_the_second_receiver() {
    use sdroxide_pluto::PlutoRig;

    let fake = Fake::start_pluto_plus();
    let cfg = PlutoConfig { agc: PlutoAgc::Manual, rx_gain_db: 40.0, ..config() };
    let rig = PlutoRig::open(&fake.address(), &cfg, 435_000_000.0).expect("open");

    // Both chains are put where the configuration says before either is used:
    // the second one's registers boot wherever the driver left them, which is
    // not where chain 0's config just went.
    wait_for("chain 1's gain-control mode", || {
        fake.state.lock().expect("lock").get("ad9361-phy/INPUT/voltage1/gain_control_mode")
            == Some("manual")
    });

    let mut r1 = rig.rx(1).expect("attach chain 1");
    r1.set_rx_gain_db(9.0);
    wait_for("chain 1's gain", || {
        fake.state
            .lock()
            .expect("lock")
            .get("ad9361-phy/INPUT/voltage1/hardwaregain")
            .is_some_and(|v| v.starts_with('9'))
    });

    // And chain 0 is untouched by it: two receivers, two gain registers.
    let g = fake.state.lock().expect("lock");
    assert!(
        g.get("ad9361-phy/INPUT/voltage0/hardwaregain").unwrap().starts_with("40"),
        "chain 0's gain moved with chain 1's: {:?}",
        g.get("ad9361-phy/INPUT/voltage0/hardwaregain")
    );
}

/// And if the two ever disagree anyway — another program moved the mode, or a
/// firmware boots a chain somewhere sdroxide did not put it — a refused gain
/// write is recovered from rather than merely logged.
#[test]
fn a_chain_found_in_the_wrong_mode_is_put_right_and_the_gain_retried() {
    use sdroxide_pluto::PlutoRig;

    let fake = Fake::start_pluto_plus();
    let cfg = PlutoConfig { agc: PlutoAgc::Manual, rx_gain_db: 40.0, ..config() };
    let rig = PlutoRig::open(&fake.address(), &cfg, 435_000_000.0).expect("open");
    let mut r1 = rig.rx(1).expect("attach chain 1");
    wait_for("chain 1's opening gain", || {
        fake.state.lock().expect("lock").get("ad9361-phy/INPUT/voltage1/hardwaregain").is_some()
    });

    // Somebody else puts the chain into an attack mode behind sdroxide's back.
    fake.state.lock().expect("lock").attrs.push((
        "ad9361-phy/INPUT/voltage1/gain_control_mode".to_string(),
        "slow_attack".to_string(),
    ));

    r1.set_rx_gain_db(11.0);
    wait_for("the gain to land after the mode was put back", || {
        let g = fake.state.lock().expect("lock");
        g.get("ad9361-phy/INPUT/voltage1/gain_control_mode") == Some("manual")
            && g.get("ad9361-phy/INPUT/voltage1/hardwaregain").is_some_and(|v| v.starts_with("11"))
    });
}
