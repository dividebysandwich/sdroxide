//! The TCI WebSocket client: one blocking thread that streams RX IQ into a ring,
//! sends TX audio from a ring (paced by the engine's fill rate / `TxChrono`), and
//! carries frequency/mode/PTT control. Mirrors the `sdroxide-hpsdr` net thread.

use std::net::{TcpStream, ToSocketAddrs};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender};
use rtrb::{Consumer, Producer, RingBuffer};
use sdroxide_types::Mode;
use tungstenite::{Message, WebSocket};

use crate::protocol as p;
use crate::split_addr;

/// TX audio rate (TCI audio streams are 48 kHz).
pub const TX_RATE_HZ: u32 = 48_000;
/// Receiver/channel we drive.
const RX: u32 = 0;
const CH: u32 = 0;

#[derive(Debug, thiserror::Error)]
pub enum TciError {
    #[error("{0}")]
    Msg(String),
}

/// A frequency or mode change reported by the rig (for two-way sync).
#[derive(Debug, Clone)]
pub enum TciUpdate {
    Freq(f64),
    Mode(Mode),
}

/// Control messages to the WebSocket thread.
enum Ctrl {
    SetCenter(f64),
    /// Offset (Hz) of the operator's VFO from the IQ centre — keeps the rig's own
    /// VFO on our dial.
    SetIf(f64),
    SetMode(Mode),
    TxOn(f64),
    TxOff,
    /// TX drive / tune-drive as a 0..1 fraction.
    SetDrive(f64),
    SetTuneDrive(f64),
    Shutdown,
}

/// A live TCI connection. Dropping it stops streaming.
pub struct TciHandle {
    ctrl: Sender<Ctrl>,
    rx: Consumer<f32>,
    tx: Producer<f32>,
    updates: Receiver<TciUpdate>,
    join: Option<JoinHandle<()>>,
    pub device: String,
    pub sample_rate_hz: f64,
}

type Ws = WebSocket<TcpStream>;

fn send_text(ws: &mut Ws, s: String) {
    let _ = ws.write(Message::Text(s.into()));
}

impl TciHandle {
    /// Connect to `address` (`host:port`), start the IQ stream at `iq_rate_hz`,
    /// and spawn the streaming thread.
    pub fn connect(address: &str, iq_rate_hz: f64) -> Result<TciHandle, TciError> {
        let (host, port) = split_addr(address).map_err(TciError::Msg)?;
        let sockaddr = (host.as_str(), port)
            .to_socket_addrs()
            .map_err(|e| TciError::Msg(format!("resolve {host}:{port}: {e}")))?
            .next()
            .ok_or_else(|| TciError::Msg(format!("no address for {host}:{port}")))?;
        let stream = TcpStream::connect_timeout(&sockaddr, Duration::from_secs(3))
            .map_err(|e| TciError::Msg(format!("connect {host}:{port}: {e}")))?;
        stream
            .set_read_timeout(Some(Duration::from_millis(20)))
            .map_err(|e| TciError::Msg(e.to_string()))?;
        let url = format!("ws://{host}:{port}/");
        let (mut ws, _) = tungstenite::client(url.as_str(), stream)
            .map_err(|e| TciError::Msg(format!("WebSocket handshake failed: {e}")))?;

        // Read the status burst until `ready;` (or time out).
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut device = String::new();
        loop {
            if Instant::now() > deadline {
                return Err(TciError::Msg("timed out waiting for 'ready;'".into()));
            }
            match ws.read() {
                Ok(Message::Text(t)) => {
                    let mut ready = false;
                    for (cmd, args) in p::parse_status(t.as_str()) {
                        match cmd.as_str() {
                            "device" => device = args,
                            "ready" => ready = true,
                            _ => {}
                        }
                    }
                    if ready {
                        break;
                    }
                }
                Ok(Message::Close(_)) => {
                    return Err(TciError::Msg("server closed during handshake".into()));
                }
                Ok(_) => {}
                Err(tungstenite::Error::Io(e))
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(e) => return Err(TciError::Msg(format!("handshake read: {e}"))),
            }
        }

        // Start the wideband IQ stream at the requested rate.
        let iq_rate = iq_rate_hz.round() as u32;
        send_text(&mut ws, p::iq_samplerate(iq_rate));
        send_text(&mut ws, p::rx_enable(RX, true));
        send_text(&mut ws, p::iq_start(RX));
        // The TX-audio path rides the (separate) audio stream; set its sample
        // rate up front. The stream itself is started only while transmitting.
        send_text(&mut ws, p::audio_samplerate(TX_RATE_HZ));
        let _ = ws.flush();

        let rx_cap = ((iq_rate_hz * 2.0 * 0.5) as usize).next_power_of_two().max(1 << 16);
        let (rx_prod, rx_cons) = RingBuffer::<f32>::new(rx_cap);
        let (tx_prod, tx_cons) = RingBuffer::<f32>::new(1 << 15);
        let (ctrl_tx, ctrl_rx) = crossbeam_channel::unbounded();
        let (upd_tx, upd_rx) = crossbeam_channel::unbounded();

        let thread = NetThread {
            ws,
            rx: rx_prod,
            tx: tx_cons,
            ctrl: ctrl_rx,
            updates: upd_tx,
            center: 0.0,
            if_hz: 0.0,
            rx_if: 0.0,
            mode: String::new(),
            ptt: false,
            tx_pkts: 0,
        };
        let join = std::thread::Builder::new()
            .name("sdroxide-tci".into())
            .spawn(move || thread.run())
            .map_err(|e| TciError::Msg(format!("spawn thread: {e}")))?;

        Ok(TciHandle {
            ctrl: ctrl_tx,
            rx: rx_cons,
            tx: tx_prod,
            updates: upd_rx,
            join: Some(join),
            device,
            sample_rate_hz: iq_rate_hz,
        })
    }

    /// Retune: set the DDC center (the IQ stream centers here).
    pub fn set_center(&self, hz: f64) {
        let _ = self.ctrl.send(Ctrl::SetCenter(hz));
    }
    /// Keep the rig's VFO `hz` above the IQ centre (our software DDC offset).
    pub fn set_if_offset(&self, hz: f64) {
        let _ = self.ctrl.send(Ctrl::SetIf(hz));
    }
    pub fn set_mode(&self, mode: Mode) {
        let _ = self.ctrl.send(Ctrl::SetMode(mode));
    }
    /// Begin transmitting at `tx_freq_hz`; returns the TX audio rate.
    pub fn tx_begin(&self, tx_freq_hz: f64) -> f64 {
        let _ = self.ctrl.send(Ctrl::TxOn(tx_freq_hz));
        TX_RATE_HZ as f64
    }
    pub fn tx_end(&self) {
        let _ = self.ctrl.send(Ctrl::TxOff);
    }

    /// Set TX drive (`0..1`) — mapped to the rig's `drive:` percentage.
    pub fn set_drive(&self, frac: f64) {
        let _ = self.ctrl.send(Ctrl::SetDrive(frac));
    }
    /// Set TUNE drive (`0..1`) — mapped to the rig's `tune_drive:` percentage.
    pub fn set_tune_drive(&self, frac: f64) {
        let _ = self.ctrl.send(Ctrl::SetTuneDrive(frac));
    }

    /// Push mono 48 kHz TX audio, with bounded back-pressure.
    pub fn tx_write(&mut self, audio: &[f32]) {
        for &v in audio {
            let mut val = v;
            let mut tries = 0u32;
            loop {
                match self.tx.push(val) {
                    Ok(()) => break,
                    Err(rtrb::PushError::Full(x)) => {
                        if tries > 2000 {
                            return;
                        }
                        tries += 1;
                        val = x;
                        std::thread::sleep(Duration::from_micros(100));
                    }
                }
            }
        }
    }

    /// Drain interleaved I,Q floats from the RX ring (always an even count).
    pub fn rx_read(&mut self, out: &mut [f32]) -> usize {
        let take = self.rx.slots().min(out.len()) & !1;
        let mut n = 0;
        while n < take {
            match self.rx.pop() {
                Ok(v) => {
                    out[n] = v;
                    n += 1;
                }
                Err(_) => break,
            }
        }
        n
    }

    /// Drain any rig-reported frequency/mode changes.
    pub fn poll_updates(&self) -> Vec<TciUpdate> {
        self.updates.try_iter().collect()
    }
}

impl Drop for TciHandle {
    fn drop(&mut self) {
        let _ = self.ctrl.send(Ctrl::Shutdown);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

struct NetThread {
    ws: Ws,
    rx: Producer<f32>,
    tx: Consumer<f32>,
    ctrl: Receiver<Ctrl>,
    updates: Sender<TciUpdate>,
    center: f64,
    /// Current rig IF offset (rig VFO = center + if_hz).
    if_hz: f64,
    /// The receive IF offset, restored when leaving TX.
    rx_if: f64,
    mode: String,
    ptt: bool,
    /// TX-audio packets sent this key-down (for diagnostics).
    tx_pkts: u64,
}

impl NetThread {
    fn run(mut self) {
        let mut iq_scratch: Vec<f32> = Vec::with_capacity(1024);
        loop {
            let mut dirty = false;

            // 1) Control messages.
            while let Ok(msg) = self.ctrl.try_recv() {
                match msg {
                    Ctrl::SetCenter(hz) => {
                        // Move the IQ centre. The IF offset (VFO within the band)
                        // is re-asserted separately by `SetIf`.
                        self.center = hz;
                        send_text(&mut self.ws, p::dds(RX, hz));
                        send_text(&mut self.ws, p::if_offset(RX, CH, self.rx_if));
                        self.if_hz = self.rx_if;
                        dirty = true;
                    }
                    Ctrl::SetIf(hz) => {
                        // Our software DDC tunes the VFO within the IQ band; mirror
                        // it onto the rig's own VFO so its display tracks the dial
                        // and returning from TX doesn't snap back to the centre.
                        self.rx_if = hz;
                        if !self.ptt && (hz - self.if_hz).abs() > 0.5 {
                            self.if_hz = hz;
                            send_text(&mut self.ws, p::if_offset(RX, CH, hz));
                            dirty = true;
                        }
                    }
                    Ctrl::SetMode(m) => {
                        self.mode = p::mode_to_tci(m).to_string();
                        send_text(&mut self.ws, p::modulation(RX, &self.mode));
                        dirty = true;
                    }
                    Ctrl::TxOn(tx_freq) => {
                        // Place the rig VFO at the TX frequency via the IF offset
                        // (keeps dds/IQ centered), then key with the TCI source.
                        let off = tx_freq - self.center;
                        if (off - self.if_hz).abs() > 0.5 {
                            send_text(&mut self.ws, p::if_offset(RX, CH, off));
                        }
                        self.if_hz = off;
                        if !self.mode.is_empty() {
                            send_text(&mut self.ws, p::modulation(RX, &self.mode));
                        }
                        // Start the audio stream so the rig accepts our TxAudio,
                        // then key with the TCI audio source.
                        send_text(&mut self.ws, p::audio_start(RX));
                        send_text(&mut self.ws, p::trx(RX, true, true));
                        self.ptt = true;
                        dirty = true;
                        tracing::debug!(tx_freq, off, mode = %self.mode, "TCI TX on");
                    }
                    Ctrl::TxOff => {
                        send_text(&mut self.ws, p::trx(RX, false, false));
                        send_text(&mut self.ws, p::audio_stop(RX));
                        // Restore the receive IF offset (the VFO within the IQ) —
                        // NOT zero — so the rig VFO stays on the operator's dial.
                        if (self.rx_if - self.if_hz).abs() > 0.5 {
                            send_text(&mut self.ws, p::if_offset(RX, CH, self.rx_if));
                        }
                        self.if_hz = self.rx_if;
                        self.ptt = false;
                        dirty = true;
                        tracing::debug!("TCI TX off");
                    }
                    Ctrl::SetDrive(frac) => {
                        let pct = (frac.clamp(0.0, 1.0) * 100.0).round() as u32;
                        send_text(&mut self.ws, p::drive(RX, pct));
                        dirty = true;
                        tracing::debug!(pct, "TCI drive");
                    }
                    Ctrl::SetTuneDrive(frac) => {
                        let pct = (frac.clamp(0.0, 1.0) * 100.0).round() as u32;
                        send_text(&mut self.ws, p::tune_drive(RX, pct));
                        dirty = true;
                    }
                    Ctrl::Shutdown => {
                        send_text(&mut self.ws, p::trx(RX, false, false));
                        send_text(&mut self.ws, p::iq_stop(RX));
                        let _ = self.ws.flush();
                        let _ = self.ws.close(None);
                        return;
                    }
                }
            }

            // 2) One inbound WebSocket message.
            match self.ws.read() {
                Ok(Message::Binary(b)) => {
                    if let Some(h) = p::parse_header(&b) {
                        match h.dtype {
                            p::DataType::Iq if h.receiver == RX => {
                                iq_scratch.clear();
                                p::decode_f32_payload(&b, &h, &mut iq_scratch);
                                for &s in &iq_scratch {
                                    let _ = self.rx.push(s);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Ok(Message::Text(t)) => self.on_text(t.as_str()),
                Ok(Message::Close(_)) => {
                    tracing::warn!("TCI server closed the connection");
                    return;
                }
                Ok(_) => {}
                Err(tungstenite::Error::Io(e))
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(e) => {
                    tracing::warn!("TCI read error: {e}; stopping");
                    return;
                }
            }

            // 3) While keyed, ship any pending TX audio (self-paced by the
            //    engine's ~48 kHz fill; TxChrono also just means "send more").
            if self.ptt {
                let avail = self.tx.slots();
                if avail >= 480 {
                    let take = avail.min(2400);
                    let mut mono = Vec::with_capacity(take);
                    for _ in 0..take {
                        match self.tx.pop() {
                            Ok(v) => mono.push(v),
                            Err(_) => break,
                        }
                    }
                    let peak = mono.iter().fold(0.0f32, |a, &v| a.max(v.abs()));
                    let pkt = p::build_tx_audio(TX_RATE_HZ, RX, &mono);
                    match self.ws.write(Message::Binary(pkt.into())) {
                        Ok(()) => {
                            self.tx_pkts += 1;
                            if self.tx_pkts == 1 || self.tx_pkts % 50 == 0 {
                                tracing::debug!(
                                    pkts = self.tx_pkts,
                                    samples = take,
                                    peak,
                                    "TCI TX audio sent"
                                );
                            }
                        }
                        Err(e) => tracing::warn!("TCI TX audio write failed: {e}"),
                    }
                    dirty = true;
                }
            } else {
                self.tx_pkts = 0;
            }

            if dirty {
                let _ = self.ws.flush();
            }
        }
    }

    /// Parse rig status echoes; emit freq/mode changes the operator made on the
    /// rig (filtered against what we last set, so our own echoes don't loop).
    fn on_text(&mut self, text: &str) {
        for (cmd, args) in p::parse_status(text) {
            match cmd.as_str() {
                "vfo" => {
                    // vfo:<rx>,<ch>,<hz>. The rig VFO we expect (from our own dds +
                    // if commands) is `center + if_hz`; echoes matching it are our
                    // own and must be ignored, or a TX IF move would loop back as a
                    // dial change. Only a genuine operator dial move differs.
                    let f: Vec<&str> = args.split(',').collect();
                    if f.len() == 3 && f[0] == "0" && f[1] == "0" {
                        if let Ok(hz) = f[2].trim().parse::<f64>() {
                            let expected = self.center + self.if_hz;
                            if (hz - expected).abs() > 1.0 {
                                // Operator turned the rig dial: adopt it as our
                                // centre (VFO == centre, IF back to zero).
                                self.center = hz;
                                self.if_hz = 0.0;
                                self.rx_if = 0.0;
                                let _ = self.updates.send(TciUpdate::Freq(hz));
                            }
                        }
                    }
                }
                "modulation" => {
                    // modulation:<rx>,<mode>
                    if let Some((rx, m)) = args.split_once(',') {
                        if rx == "0" {
                            let ml = m.trim().to_lowercase();
                            if ml != self.mode {
                                if let Some(mode) = p::tci_to_mode(&ml) {
                                    self.mode = ml;
                                    let _ = self.updates.send(TciUpdate::Mode(mode));
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
