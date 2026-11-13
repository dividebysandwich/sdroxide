//! Mercury as a [`TncModem`]: audio through the engine's exchange, the host
//! language over Mercury's own (loopback-only) TCP ports.

use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender, unbounded};
use sdroxide_tnc::{TncKind, TncModem};

use crate::engine::Engine;

/// One TCP connection's two halves: a writer thread so a stalled peer cannot
/// stall the audio path, and a reader thread feeding a channel.
struct Conn {
    stream: TcpStream,
    tx: Sender<Vec<u8>>,
    rx: Receiver<Vec<u8>>,
}

impl Conn {
    fn open(port: u16, name: &str) -> std::io::Result<Self> {
        let deadline = Instant::now() + Duration::from_secs(3);
        let stream = loop {
            match TcpStream::connect(("127.0.0.1", port)) {
                Ok(s) => break s,
                Err(e) if Instant::now() >= deadline => return Err(e),
                Err(_) => thread::sleep(Duration::from_millis(50)),
            }
        };
        let _ = stream.set_nodelay(true);
        let (tx, wrx) = unbounded::<Vec<u8>>();
        let mut w = stream.try_clone()?;
        thread::Builder::new().name(format!("{name}-tx")).spawn(move || {
            for b in wrx {
                if w.write_all(&b).is_err() {
                    break;
                }
            }
        })?;
        let (rtx, rx) = unbounded();
        let mut r = stream.try_clone()?;
        thread::Builder::new().name(format!("{name}-rx")).spawn(move || {
            let mut buf = vec![0u8; 16384];
            loop {
                match r.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if rtx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        })?;
        Ok(Self { stream, tx, rx })
    }

    fn send(&self, b: &[u8]) {
        self.tx.send(b.to_vec()).ok();
    }
}

impl Drop for Conn {
    fn drop(&mut self) {
        let _ = self.stream.shutdown(Shutdown::Both);
    }
}

/// Mercury, driven by sdroxide.
pub struct MercuryModem {
    engine: &'static Engine,
    cmd: Option<Conn>,
    data: Option<Conn>,
    /// Bytes of the command stream not yet ended by a CR.
    partial: Vec<u8>,
    lines: Vec<String>,
    received: Vec<u8>,
    rx16: Vec<i16>,
    tx16: Vec<i16>,
    tuning: bool,
}

impl MercuryModem {
    /// Attach to the running engine as its TNC client. Transmit audio Mercury
    /// queued while nobody clocked it is dropped first.
    pub fn new(engine: &'static Engine) -> Result<Self, String> {
        engine.drain_tx();
        let mut m = Self {
            engine,
            cmd: None,
            data: None,
            partial: Vec::new(),
            lines: Vec::new(),
            received: Vec::new(),
            rx16: Vec::new(),
            tx16: Vec::new(),
            tuning: false,
        };
        m.connect()?;
        Ok(m)
    }

    fn connect(&mut self) -> Result<(), String> {
        let port = self.engine.port();
        let cmd =
            Conn::open(port, "mercury-cmd").map_err(|e| format!("Mercury's port {port}: {e}"))?;
        let data = Conn::open(port.wrapping_add(1), "mercury-data")
            .map_err(|e| format!("Mercury's port {}: {e}", port.wrapping_add(1)))?;
        self.cmd = Some(cmd);
        self.data = Some(data);
        self.partial.clear();
        Ok(())
    }

    /// The engine this modem drives.
    pub fn engine(&self) -> &'static Engine {
        self.engine
    }

    fn drain(&mut self) {
        if let Some(c) = &self.cmd {
            while let Ok(b) = c.rx.try_recv() {
                for &ch in &b {
                    match ch {
                        b'\r' | b'\n' => {
                            if !self.partial.is_empty() {
                                self.lines.push(
                                    String::from_utf8_lossy(&self.partial).trim().to_string(),
                                );
                                self.partial.clear();
                            }
                        }
                        _ => self.partial.push(ch),
                    }
                }
            }
        }
        if let Some(d) = &self.data {
            while let Ok(b) = d.rx.try_recv() {
                self.received.extend_from_slice(&b);
            }
        }
    }

    fn send_line(&self, line: &str) {
        if let Some(c) = &self.cmd {
            c.send(format!("{line}\r").as_bytes());
        }
    }
}

impl Drop for MercuryModem {
    fn drop(&mut self) {
        // The engine outlives us: leave it deaf to calls and idle.
        if self.tuning {
            self.send_line("TUNE OFF");
        }
        self.send_line("ABORT");
        self.send_line("LISTEN OFF");
        thread::sleep(Duration::from_millis(50));
    }
}

fn to_i16(v: f32) -> i16 {
    (v.clamp(-1.0, 1.0) * 32767.0) as i16
}

impl TncModem for MercuryModem {
    fn kind(&self) -> TncKind {
        TncKind::Mercury
    }

    fn process(&mut self, rx: &[f32], tx: &mut [f32]) {
        let n = rx.len().min(tx.len());
        self.rx16.clear();
        self.rx16.extend(rx[..n].iter().map(|&v| to_i16(v)));
        self.tx16.resize(n, 0);
        let heard = if self.engine.keyed() { None } else { Some(&self.rx16[..]) };
        self.engine.exchange(heard, &mut self.tx16);
        for (o, &v) in tx[..n].iter_mut().zip(&self.tx16) {
            *o = f32::from(v) / 32768.0;
        }
        self.drain();
    }

    fn keyed(&self) -> bool {
        self.engine.keyed()
    }

    fn command(&mut self, line: &str) {
        self.send_line(line);
    }

    fn data(&mut self, bytes: &[u8]) {
        if let Some(d) = &self.data {
            d.send(bytes);
        }
    }

    fn take_output(&mut self) -> (Vec<String>, Vec<u8>) {
        self.drain();
        (std::mem::take(&mut self.lines), std::mem::take(&mut self.received))
    }

    /// Mercury ends a session whose client leaves; reconnecting is that leave.
    fn client_gone(&mut self) {
        self.drain();
        self.cmd = None;
        self.data = None;
        if let Err(e) = self.connect() {
            tracing::warn!("Mercury: {e}");
        }
    }

    fn abort(&mut self) {
        self.send_line("ABORT");
    }

    fn tune(&mut self, on: bool) -> bool {
        self.tuning = on;
        self.send_line(if on { "TUNE -10" } else { "TUNE OFF" });
        true
    }

    fn set_dial_hz(&mut self, hz: u64) {
        self.engine.set_dial_hz(hz);
    }

    fn tx_peak(&self) -> f32 {
        crate::TX_PEAK
    }

    fn version(&self) -> String {
        format!("Mercury {}", crate::VERSION)
    }
}
