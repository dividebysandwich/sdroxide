//! The HPSDR Protocol 2 network engine: one blocking UDP thread that streams DDC
//! (RX) I/Q into a ring, packetizes DUC (TX) I/Q from a ring, and keeps the
//! hardware watchdog fed with periodic high-priority packets.

use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender};
use rtrb::{Consumer, Producer, RingBuffer};

use crate::discovery;
use crate::protocol2::{self, port};

/// Host→radio TX I/Q rate. The engine's modulator is native 48 kHz, so we ask
/// for 48 kHz DUC and pass the samples straight through.
pub const TX_RATE_HZ: u32 = 48_000;
/// Fixed FPGA drive level while keyed. The engine already scales the I/Q by the
/// operator's drive fraction in software, so the FPGA runs at full scale and the
/// I/Q amplitude sets the power (the TX safety rails still apply upstream).
const TX_DRIVE: u8 = 255;
/// Resend the high-priority packet at least this often to satisfy the radio's
/// watchdog (which otherwise stops the radio).
const WATCHDOG: Duration = Duration::from_millis(50);

#[derive(Debug, thiserror::Error)]
pub enum HpsdrError {
    #[error("network I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("device at {0} speaks Protocol 1, which is not yet supported")]
    Protocol1(String),
    #[error("{0}")]
    Msg(String),
}

/// Control messages to the network thread.
enum Ctrl {
    RxFreq(f64),
    TxOn(f64),
    TxOff,
    Shutdown,
}

/// A live connection to an HPSDR radio. Dropping it stops streaming.
pub struct HpsdrHandle {
    ctrl: Sender<Ctrl>,
    rx: Consumer<f32>,
    tx: Producer<f32>,
    join: Option<JoinHandle<()>>,
    /// Board name reported by discovery (or "HPSDR" if it did not answer).
    pub board: String,
    /// Actual DDC (RX) sample rate in Hz.
    pub sample_rate_hz: f64,
    /// Actual DUC (TX) sample rate in Hz.
    pub tx_rate_hz: f64,
}

impl HpsdrHandle {
    /// Open a Protocol 2 connection to `ip`, configuring DDC0 at
    /// `sample_rate_hz` and starting the stream.
    pub fn open(ip: Ipv4Addr, sample_rate_hz: f64) -> Result<HpsdrHandle, HpsdrError> {
        // Confirm the board and reject Protocol-1-only hardware early. A manual
        // IP that does not answer discovery is still tried (some setups firewall
        // the broadcast but pass unicast streaming).
        let board = match discovery::probe(ip, Duration::from_millis(800)) {
            Some(dev) if dev.protocol == 1 => return Err(HpsdrError::Protocol1(ip.to_string())),
            Some(dev) => dev.board,
            None => "HPSDR".to_string(),
        };

        let rate = clamp_rate(sample_rate_hz);
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
        socket.set_read_timeout(Some(Duration::from_millis(2)))?;

        // RX ring: ~0.5 s of interleaved I/Q at the DDC rate. TX ring: ~0.5 s at
        // the DUC rate.
        let rx_cap = ((rate * 2.0 * 0.5) as usize).next_power_of_two().max(1 << 16);
        let (rx_prod, rx_cons) = RingBuffer::<f32>::new(rx_cap);
        let tx_cap = (TX_RATE_HZ as usize * 2 / 2).next_power_of_two();
        let (tx_prod, tx_cons) = RingBuffer::<f32>::new(tx_cap);

        let (ctrl_tx, ctrl_rx) = crossbeam_channel::unbounded();

        let net = NetThread {
            socket,
            radio: IpAddr::V4(ip),
            rate_khz: (rate / 1000.0) as u16,
            rx: rx_prod,
            tx: tx_cons,
            ctrl: ctrl_rx,
            seq: SeqCounters::default(),
            rx_freq: 7_100_000.0,
            tx_freq: 7_100_000.0,
            ptt: false,
        };
        let join = std::thread::Builder::new()
            .name("sdroxide-hpsdr".into())
            .spawn(move || net.run())
            .map_err(|e| HpsdrError::Msg(format!("spawn network thread: {e}")))?;

        Ok(HpsdrHandle {
            ctrl: ctrl_tx,
            rx: rx_cons,
            tx: tx_prod,
            join: Some(join),
            board,
            sample_rate_hz: rate,
            tx_rate_hz: TX_RATE_HZ as f64,
        })
    }

    /// Retune the RX DDC NCO.
    pub fn set_rx_freq(&self, hz: f64) {
        let _ = self.ctrl.send(Ctrl::RxFreq(hz));
    }

    /// Begin transmitting at `tx_freq_hz`; returns the TX I/Q rate to feed
    /// [`Self::tx_write`].
    pub fn tx_begin(&self, tx_freq_hz: f64) -> f64 {
        let _ = self.ctrl.send(Ctrl::TxOn(tx_freq_hz));
        self.tx_rate_hz
    }

    /// Stop transmitting.
    pub fn tx_end(&self) {
        let _ = self.ctrl.send(Ctrl::TxOff);
    }

    /// Push interleaved I,Q TX samples (at [`Self::tx_rate_hz`]). Blocks briefly
    /// when the ring is full (pacing the caller); drops if the thread stalls.
    pub fn tx_write(&mut self, iq: &[f32]) {
        for &v in iq {
            let mut val = v;
            let mut tries = 0u32;
            loop {
                match self.tx.push(val) {
                    Ok(()) => break,
                    Err(rtrb::PushError::Full(x)) => {
                        if tries > 2000 {
                            return; // network thread stalled — drop rather than hang
                        }
                        tries += 1;
                        val = x;
                        std::thread::sleep(Duration::from_micros(100));
                    }
                }
            }
        }
    }

    /// Drain interleaved I,Q floats from the RX ring into `out`. Always returns
    /// an even count (never splits an I/Q pair, so the stream stays aligned).
    /// Returns 0 when no data is available yet.
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
}

impl Drop for HpsdrHandle {
    fn drop(&mut self) {
        let _ = self.ctrl.send(Ctrl::Shutdown);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

/// Round a requested rate to the nearest supported DDC rate.
fn clamp_rate(hz: f64) -> f64 {
    sdroxide_types::HpsdrConfig::SAMPLE_RATES
        .iter()
        .copied()
        .min_by(|a, b| (a - hz).abs().partial_cmp(&(b - hz).abs()).unwrap())
        .unwrap_or(1_536_000.0)
}

#[derive(Default)]
struct SeqCounters {
    general: u32,
    high_priority: u32,
    ddc: u32,
    duc: u32,
    tx_iq: u32,
}

impl SeqCounters {
    fn next(v: &mut u32) -> u32 {
        let n = *v;
        *v = v.wrapping_add(1);
        n
    }
}

struct NetThread {
    socket: UdpSocket,
    radio: IpAddr,
    rate_khz: u16,
    rx: Producer<f32>,
    tx: Consumer<f32>,
    ctrl: Receiver<Ctrl>,
    seq: SeqCounters,
    rx_freq: f64,
    tx_freq: f64,
    ptt: bool,
}

impl NetThread {
    fn dest(&self, port: u16) -> SocketAddr {
        SocketAddr::new(self.radio, port)
    }

    fn send_high_priority(&mut self) {
        let seq = SeqCounters::next(&mut self.seq.high_priority);
        let rx = protocol2::phase_word(self.rx_freq, protocol2::CLOCK_HZ);
        let tx = protocol2::phase_word(self.tx_freq, protocol2::CLOCK_HZ);
        let drive = if self.ptt { TX_DRIVE } else { 0 };
        let pkt = protocol2::high_priority_packet(seq, rx, tx, self.ptt, drive);
        let _ = self.socket.send_to(&pkt, self.dest(port::HIGH_PRIORITY));
    }

    fn send_ddc_command(&mut self) {
        let seq = SeqCounters::next(&mut self.seq.ddc);
        let pkt = protocol2::ddc_command_packet(seq, self.rate_khz);
        let _ = self.socket.send_to(&pkt, self.dest(port::DDC_COMMAND));
    }

    fn send_duc_command(&mut self) {
        let seq = SeqCounters::next(&mut self.seq.duc);
        let pkt = protocol2::duc_command_packet(seq);
        let _ = self.socket.send_to(&pkt, self.dest(port::DUC_COMMAND));
    }

    fn send_general(&mut self, run: bool) {
        let seq = SeqCounters::next(&mut self.seq.general);
        let pkt = protocol2::general_packet(seq, run);
        let _ = self.socket.send_to(&pkt, self.dest(port::GENERAL));
    }

    fn run(mut self) {
        // Start-up handshake: configure DDC/DUC, set the initial NCO, then run.
        self.send_ddc_command();
        self.send_duc_command();
        self.send_high_priority();
        self.send_general(true);

        let mut last_watchdog = Instant::now();
        let mut rx_scratch: Vec<f32> = Vec::with_capacity(512);
        let mut tx_scratch: Vec<f32> = Vec::with_capacity(protocol2::DUC_SAMPLES_PER_PKT * 2);
        let mut buf = [0u8; 2048];

        loop {
            // 1) Control messages.
            let mut freq_changed = false;
            while let Ok(msg) = self.ctrl.try_recv() {
                match msg {
                    Ctrl::RxFreq(hz) => {
                        self.rx_freq = hz;
                        freq_changed = true;
                    }
                    Ctrl::TxOn(hz) => {
                        self.tx_freq = hz;
                        self.ptt = true;
                        self.send_duc_command();
                        freq_changed = true;
                    }
                    Ctrl::TxOff => {
                        self.ptt = false;
                        freq_changed = true;
                    }
                    Ctrl::Shutdown => {
                        self.send_general(false);
                        return;
                    }
                }
            }
            if freq_changed {
                self.send_high_priority();
            }

            // 2) One inbound datagram (RX I/Q or status).
            match self.socket.recv_from(&mut buf) {
                Ok((n, src)) => {
                    let p = src.port();
                    if (port::DDC_IQ_BASE..port::DDC_IQ_BASE + 8).contains(&p) {
                        rx_scratch.clear();
                        if protocol2::decode_ddc_iq(&buf[..n], &mut rx_scratch).is_some() {
                            for &s in &rx_scratch {
                                // Drop on overflow: the consumer (DSP) sets the pace.
                                let _ = self.rx.push(s);
                            }
                        }
                    }
                }
                Err(ref e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(e) => {
                    tracing::warn!("HPSDR recv error: {e}; stopping network thread");
                    self.send_general(false);
                    return;
                }
            }

            // 3) Packetize any pending TX I/Q while keyed.
            if self.ptt {
                while let Ok(v) = self.tx.pop() {
                    tx_scratch.push(v);
                    if tx_scratch.len() >= protocol2::DUC_SAMPLES_PER_PKT * 2 {
                        let seq = SeqCounters::next(&mut self.seq.tx_iq);
                        let pkt = protocol2::duc_iq_packet(seq, &tx_scratch);
                        let _ = self.socket.send_to(&pkt, self.dest(port::DUC_IQ));
                        tx_scratch.clear();
                    }
                }
            } else if !tx_scratch.is_empty() {
                tx_scratch.clear();
            }

            // 4) Watchdog: keep the radio alive.
            if last_watchdog.elapsed() >= WATCHDOG {
                self.send_high_priority();
                self.send_general(true);
                last_watchdog = Instant::now();
            }
        }
    }
}
