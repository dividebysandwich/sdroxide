//! Serial CAT control for non-SoapySDR rigs (Icom CI-V / Yaesu / Xiegu).
//!
//! NATIVE ONLY — links `serialport`; must never be a dependency of any
//! wasm-targeted crate. The rest of the app talks to it only through the
//! opaque [`CatHandle`] (a background serial thread), so no serial types leak
//! into the engine or UI.

mod civ;
mod yaesu;

use std::io::Write;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender, TryRecvError};
use sdroxide_types::{
    CatConfig, CatFamily, LineState, ModePolicy, Mode, Parity, PttMethod, SerialConfig, StopBits,
};
use tracing::{info, warn};

/// A change the rig reported (external dial/mode movement) or that we read back.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CatUpdate {
    Freq(f64),
    Mode(Mode),
}

/// Enumerate serial ports for the settings UI.
pub fn available_ports() -> Vec<String> {
    serialport::available_ports()
        .map(|ports| ports.into_iter().map(|p| p.port_name).collect())
        .unwrap_or_default()
}

/// Per-family framing. `parse` consumes complete frames from a rolling buffer.
trait Protocol: Send {
    fn set_freq(&self, hz: f64) -> Vec<u8>;
    fn set_mode(&self, m: Mode) -> Vec<u8>;
    /// CAT-command PTT (only used when `PttMethod::Cat`).
    fn ptt(&self, on: bool) -> Vec<u8>;
    /// Frames that request the rig's current freq + mode.
    fn poll_requests(&self) -> Vec<Vec<u8>>;
    fn parse(&mut self, buf: &mut Vec<u8>) -> Vec<CatUpdate>;
}

/// CI-V protocol (Icom + Xiegu). `radio` is the CI-V transceiver address.
struct Civ {
    radio: u8,
}

impl Protocol for Civ {
    fn set_freq(&self, hz: f64) -> Vec<u8> {
        civ::set_freq_frame(self.radio, hz)
    }
    fn set_mode(&self, m: Mode) -> Vec<u8> {
        civ::set_mode_frame(self.radio, m)
    }
    fn ptt(&self, on: bool) -> Vec<u8> {
        civ::ptt_frame(self.radio, on)
    }
    fn poll_requests(&self) -> Vec<Vec<u8>> {
        vec![civ::read_freq_frame(self.radio), civ::read_mode_frame(self.radio)]
    }
    fn parse(&mut self, buf: &mut Vec<u8>) -> Vec<CatUpdate> {
        let mut out = Vec::new();
        for reply in civ::parse_frames(buf) {
            // Ignore our own echoes (controller-sourced frames).
            if reply.from == civ::CONTROLLER_ADDR {
                continue;
            }
            match reply.cmd {
                0x03 => {
                    if let Some(hz) = civ::decode_freq(&reply.data) {
                        out.push(CatUpdate::Freq(hz));
                    }
                }
                0x04 => {
                    if let Some(&b) = reply.data.first() {
                        if let Some(m) = civ::civ_to_mode(b) {
                            out.push(CatUpdate::Mode(m));
                        }
                    }
                }
                _ => {}
            }
        }
        out
    }
}

fn make_protocol(cfg: &CatConfig) -> Box<dyn Protocol> {
    match cfg.family {
        CatFamily::Xiegu | CatFamily::Icom => Box::new(Civ { radio: cfg.icom_radio_id }),
        CatFamily::Yaesu => Box::new(yaesu::Yaesu::new()),
    }
}

enum CatCmd {
    Freq(f64),
    Mode(Mode),
    Ptt(bool),
    Stop,
}

/// Opaque handle to the running serial thread.
pub struct CatHandle {
    cmd_tx: Sender<CatCmd>,
    event_rx: Receiver<CatUpdate>,
}

impl CatHandle {
    pub fn set_freq(&self, hz: f64) {
        let _ = self.cmd_tx.send(CatCmd::Freq(hz));
    }
    pub fn set_mode(&self, m: Mode) {
        let _ = self.cmd_tx.send(CatCmd::Mode(m));
    }
    pub fn set_ptt(&self, on: bool) {
        let _ = self.cmd_tx.send(CatCmd::Ptt(on));
    }
    /// Non-blocking drain of rig-reported freq/mode changes.
    pub fn poll(&self) -> Vec<CatUpdate> {
        self.event_rx.try_iter().collect()
    }
}

impl Drop for CatHandle {
    fn drop(&mut self) {
        let _ = self.cmd_tx.send(CatCmd::Stop);
    }
}

/// Blocking one-shot query of the rig's current frequency + mode, used at
/// startup so the app adopts the radio's state instead of overwriting it.
/// Returns `None` if the port can't be opened or the rig doesn't answer.
pub fn query_once(cfg: &CatConfig) -> Option<(Option<f64>, Option<Mode>)> {
    let mut port = open_port(&cfg.serial).ok()?;
    let protocol = make_protocol(cfg);
    for req in protocol.poll_requests() {
        let _ = port.write_all(&req);
    }
    let _ = port.flush();
    let mut protocol = protocol;
    let mut rx = Vec::new();
    let mut buf = [0u8; 128];
    let (mut freq, mut mode) = (None, None);
    let deadline = Instant::now() + Duration::from_millis(600);
    while Instant::now() < deadline && (freq.is_none() || mode.is_none()) {
        if let Ok(n) = port.read(&mut buf) {
            if n > 0 {
                rx.extend_from_slice(&buf[..n]);
                for u in protocol.parse(&mut rx) {
                    match u {
                        CatUpdate::Freq(hz) => freq = Some(hz),
                        CatUpdate::Mode(m) => mode = Some(m),
                    }
                }
            }
        }
    }
    (freq.is_some() || mode.is_some()).then_some((freq, mode))
}

/// Spawn the serial CAT thread from a persisted [`CatConfig`].
pub fn spawn(cfg: CatConfig) -> CatHandle {
    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
    let (event_tx, event_rx) = crossbeam_channel::unbounded();
    std::thread::Builder::new()
        .name("sdroxide-cat".into())
        .spawn(move || serial_thread(cfg, cmd_rx, event_tx))
        .expect("spawn cat thread");
    CatHandle { cmd_tx, event_rx }
}

fn map_parity(p: Parity) -> serialport::Parity {
    match p {
        Parity::None => serialport::Parity::None,
        Parity::Even => serialport::Parity::Even,
        Parity::Odd => serialport::Parity::Odd,
    }
}
fn map_stop(s: StopBits) -> serialport::StopBits {
    match s {
        StopBits::One => serialport::StopBits::One,
        StopBits::Two => serialport::StopBits::Two,
    }
}
fn map_data_bits(n: u8) -> serialport::DataBits {
    match n {
        5 => serialport::DataBits::Five,
        6 => serialport::DataBits::Six,
        7 => serialport::DataBits::Seven,
        _ => serialport::DataBits::Eight,
    }
}

fn open_port(s: &SerialConfig) -> serialport::Result<Box<dyn serialport::SerialPort>> {
    let port = serialport::new(&s.path, s.baud)
        .data_bits(map_data_bits(s.data_bits))
        .parity(map_parity(s.parity))
        .stop_bits(map_stop(s.stop_bits))
        .timeout(Duration::from_millis(50))
        .open()?;
    Ok(port)
}

/// Apply a forced control-line level (ignored when `LineState::None`). If a
/// line is used for PTT, PTT owns it instead (handled in the loop).
fn apply_line(port: &mut dyn serialport::SerialPort, forced: LineState, rts: bool) {
    let level = match forced {
        LineState::None => return,
        LineState::High => true,
        LineState::Low => false,
    };
    let _ = if rts { port.write_request_to_send(level) } else { port.write_data_terminal_ready(level) };
}

fn serial_thread(cfg: CatConfig, cmd_rx: Receiver<CatCmd>, event_tx: Sender<CatUpdate>) {
    let mut protocol = make_protocol(&cfg);
    let poll_period = Duration::from_secs_f32((1.0 / cfg.poll_hz.max(0.2)).min(5.0));
    // What mode to actually command the rig into for a given app mode.
    let mode_cmd = |_app_mode: Mode| -> Option<Mode> {
        match cfg.mode_policy {
            ModePolicy::SetByRadio => None,
            ModePolicy::Usb => Some(Mode::Usb),
            ModePolicy::DataPkt => Some(Mode::Digu),
        }
    };

    loop {
        // (Re)open the port, retrying on failure.
        let mut port = match open_port(&cfg.serial) {
            Ok(p) => {
                info!(path = %cfg.serial.path, baud = cfg.serial.baud, "CAT port open");
                p
            }
            Err(e) => {
                warn!(path = %cfg.serial.path, "CAT open failed: {e}");
                // Wait, but still honor a Stop.
                match cmd_rx.recv_timeout(Duration::from_secs(2)) {
                    Ok(CatCmd::Stop) | Err(crossbeam_channel::RecvTimeoutError::Disconnected) => return,
                    _ => continue,
                }
            }
        };
        // Forced control lines (unless the line is the PTT method).
        if cfg.ptt != PttMethod::Rts {
            apply_line(&mut *port, cfg.serial.force_rts, true);
        }
        if cfg.ptt != PttMethod::Dtr {
            apply_line(&mut *port, cfg.serial.force_dtr, false);
        }
        // Deassert PTT line at start.
        match cfg.ptt {
            PttMethod::Rts => {
                let _ = port.write_request_to_send(false);
            }
            PttMethod::Dtr => {
                let _ = port.write_data_terminal_ready(false);
            }
            _ => {}
        }
        // Apply the mode policy once on connect.
        if let Some(m) = mode_cmd(Mode::Usb) {
            let _ = port.write_all(&protocol.set_mode(m));
        }

        let mut rx = Vec::with_capacity(256);
        let mut read_buf = [0u8; 256];
        let mut next_poll = Instant::now();
        let mut pending_freq: Option<f64> = None;
        let mut last_sent_freq: Option<f64> = None;
        let mut freq_deadline = Instant::now();
        // Only forward genuine changes so the engine isn't re-notified every poll.
        let mut emit_freq: Option<f64> = None;
        let mut emit_mode: Option<Mode> = None;

        let broke = 'io: loop {
            // Drain commands.
            loop {
                match cmd_rx.try_recv() {
                    Ok(CatCmd::Freq(hz)) => pending_freq = Some(hz), // coalesce
                    Ok(CatCmd::Mode(m)) => {
                        if let Some(mm) = mode_cmd(m) {
                            if port.write_all(&protocol.set_mode(mm)).is_err() {
                                break 'io true;
                            }
                        }
                    }
                    Ok(CatCmd::Ptt(on)) => {
                        let failed = match cfg.ptt {
                            PttMethod::Vox => false,
                            PttMethod::Rts => port.write_request_to_send(on).is_err(),
                            PttMethod::Dtr => port.write_data_terminal_ready(on).is_err(),
                            PttMethod::Cat => port.write_all(&protocol.ptt(on)).is_err(),
                        };
                        if failed {
                            break 'io true;
                        }
                    }
                    Ok(CatCmd::Stop) => return,
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => return,
                }
            }

            // Debounced frequency write (rate-limit to ~50 ms, only on change).
            if let Some(hz) = pending_freq {
                let now = Instant::now();
                if last_sent_freq != Some(hz) && now >= freq_deadline {
                    if port.write_all(&protocol.set_freq(hz)).is_err() {
                        break 'io true;
                    }
                    last_sent_freq = Some(hz);
                    emit_freq = Some(hz); // suppress the poll echo of our own set
                    pending_freq = None;
                    freq_deadline = now + Duration::from_millis(50);
                }
            }

            // Poll the rig for external changes.
            if Instant::now() >= next_poll {
                next_poll = Instant::now() + poll_period;
                for req in protocol.poll_requests() {
                    if port.write_all(&req).is_err() {
                        break 'io true;
                    }
                }
            }

            // Read whatever arrived; parse and emit updates.
            match port.read(&mut read_buf) {
                Ok(0) => {}
                Ok(n) => {
                    rx.extend_from_slice(&read_buf[..n]);
                    for u in protocol.parse(&mut rx) {
                        // Forward only genuine changes (poll repeats otherwise).
                        let changed = match u {
                            CatUpdate::Freq(hz) => {
                                let c = emit_freq.map(|f| (f - hz).abs() >= 1.0).unwrap_or(true);
                                if c {
                                    emit_freq = Some(hz);
                                }
                                c
                            }
                            CatUpdate::Mode(m) => {
                                let c = emit_mode != Some(m);
                                if c {
                                    emit_mode = Some(m);
                                }
                                c
                            }
                        };
                        if changed {
                            let _ = event_tx.send(u);
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(e) => {
                    warn!("CAT read error: {e}");
                    break 'io true;
                }
            }

            std::thread::sleep(Duration::from_millis(5));
        };

        if broke {
            warn!("CAT link error; reconnecting");
            std::thread::sleep(Duration::from_secs(1));
        } else {
            return;
        }
    }
}
