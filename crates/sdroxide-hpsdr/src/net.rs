//! Shared HPSDR connection handle and the protocol-dispatch open path. Each
//! protocol (1 = Metis, 2 = new) runs its own blocking UDP thread; both stream
//! RX I/Q into a ring, packetize TX I/Q from a ring, and keep the radio alive.

use std::net::{IpAddr, Ipv4Addr, UdpSocket};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender};
use rtrb::{Consumer, Producer, RingBuffer};

use crate::discovery;
use crate::{protocol1, protocol2};

/// Host→radio TX I/Q rate for Protocol 2 (the engine's modulator is native
/// 48 kHz, fed straight to the DUC). Protocol 1 transmits at the DDC rate.
pub const TX_RATE_HZ: u32 = 48_000;
/// Resend keep-alive/high-priority state at least this often so the radio's
/// watchdog does not stop the stream.
pub(crate) const WATCHDOG: Duration = Duration::from_millis(50);
/// How often a protocol thread emits an RX throughput line (`RUST_LOG=…=debug`).
pub(crate) const STATS_INTERVAL: Duration = Duration::from_secs(2);

/// Format the first `n` bytes of a datagram as spaced uppercase hex, for the
/// diagnostic logs that let a remote tester compare on-wire bytes against the
/// OpenHPSDR spec (the wire offsets in this crate are not hardware-verified).
pub(crate) fn hex_head(bytes: &[u8], n: usize) -> String {
    bytes
        .iter()
        .take(n)
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Periodic RX throughput/health accounting for a protocol thread. Counts
/// decoded I/Q datagrams and unrecognized datagrams, emitting one `debug` line
/// per [`STATS_INTERVAL`] so a tester can see whether I/Q is actually flowing —
/// and at what rate — without a per-packet log flood. A wrong sample rate or a
/// bad decode offset shows up immediately as an implausible ksps figure.
pub(crate) struct RxStats {
    proto: u8,
    since: Instant,
    win_datagrams: u64,
    win_samples: u64,
    win_other: u64,
    total_datagrams: u64,
    total_samples: u64,
}

impl RxStats {
    pub(crate) fn new(proto: u8) -> Self {
        RxStats {
            proto,
            since: Instant::now(),
            win_datagrams: 0,
            win_samples: 0,
            win_other: 0,
            total_datagrams: 0,
            total_samples: 0,
        }
    }

    /// Record a decoded I/Q datagram carrying `pairs` complex samples.
    pub(crate) fn on_iq(&mut self, pairs: usize) {
        self.win_datagrams += 1;
        self.total_datagrams += 1;
        self.win_samples += pairs as u64;
        self.total_samples += pairs as u64;
    }

    /// Record a datagram that was not a recognized I/Q frame.
    pub(crate) fn on_other(&mut self) {
        self.win_other += 1;
    }

    /// Emit a throughput line if the reporting interval has elapsed.
    pub(crate) fn tick(&mut self) {
        let dt = self.since.elapsed();
        if dt < STATS_INTERVAL {
            return;
        }
        let ksps = self.win_samples as f64 / dt.as_secs_f64() / 1000.0;
        tracing::debug!(
            "HPSDR P{} RX: {} datagrams, {} samples ({:.1} ksps) over {:.2}s; \
             {} unrecognized; totals {} datagrams / {} samples",
            self.proto,
            self.win_datagrams,
            self.win_samples,
            ksps,
            dt.as_secs_f64(),
            self.win_other,
            self.total_datagrams,
            self.total_samples,
        );
        self.since = Instant::now();
        self.win_datagrams = 0;
        self.win_samples = 0;
        self.win_other = 0;
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HpsdrError {
    #[error("network I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Msg(String),
}

/// Control messages from the [`HpsdrHandle`] to its network thread.
pub(crate) enum Ctrl {
    RxFreq(f64),
    TxOn(f64),
    TxOff,
    Shutdown,
}

/// Everything a protocol thread needs: the socket, the radio address, the rates,
/// the RX/TX rings, and the control channel.
pub(crate) struct ThreadCtx {
    pub socket: UdpSocket,
    pub radio: IpAddr,
    pub rate_hz: f64,
    pub rx: Producer<f32>,
    pub tx: Consumer<f32>,
    pub ctrl: Receiver<Ctrl>,
}

/// A live connection to an HPSDR radio. Dropping it stops streaming.
pub struct HpsdrHandle {
    ctrl: Sender<Ctrl>,
    rx: Consumer<f32>,
    tx: Producer<f32>,
    join: Option<JoinHandle<()>>,
    /// Board name reported by discovery (or "HPSDR" if it did not answer).
    pub board: String,
    /// OpenHPSDR protocol in use (1 or 2).
    pub protocol: u8,
    /// Actual RX sample rate in Hz.
    pub sample_rate_hz: f64,
    /// Actual TX I/Q rate in Hz.
    pub tx_rate_hz: f64,
}

impl HpsdrHandle {
    /// Open a connection to `ip`, auto-detecting the protocol from a discovery
    /// probe (both P1 and P2 requests are sent), configuring the RX at
    /// `sample_rate_hz`, and starting the stream. A manual IP that does not
    /// answer the probe is still tried as Protocol 2.
    pub fn open(ip: Ipv4Addr, sample_rate_hz: f64) -> Result<HpsdrHandle, HpsdrError> {
        tracing::info!("HPSDR: opening {ip}, requested RX rate {sample_rate_hz:.0} Hz");
        let (board, protocol) = match discovery::probe(ip, Duration::from_millis(800)) {
            Some(dev) => {
                tracing::info!(
                    "HPSDR: {ip} answered probe: board \"{}\", Protocol {}, MAC {}, {}",
                    dev.board,
                    dev.protocol,
                    dev.mac,
                    if dev.in_use { "IN USE" } else { "idle" }
                );
                (dev.board, dev.protocol)
            }
            None => {
                tracing::warn!(
                    "HPSDR: {ip} did not answer the discovery probe; assuming Protocol 2. \
                     If this board is a Hermes-Lite 2 or other Protocol 1 device, RX will not \
                     start — check the IP and that no other program holds the radio."
                );
                ("HPSDR".to_string(), 2)
            }
        };

        let rate = clamp_rate(sample_rate_hz, protocol);
        if (rate - sample_rate_hz).abs() > 1.0 {
            tracing::info!(
                "HPSDR: requested {sample_rate_hz:.0} Hz rounded to nearest Protocol {protocol} \
                 rate {rate:.0} Hz"
            );
        }
        // Protocol 1 sends TX I/Q inside the RX frame stream at the DDC rate;
        // Protocol 2 has a dedicated 48 kHz DUC.
        let tx_rate = if protocol == 1 { rate } else { TX_RATE_HZ as f64 };

        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
        socket.set_read_timeout(Some(Duration::from_millis(2)))?;
        tracing::debug!(
            "HPSDR: bound local UDP socket {}",
            socket.local_addr().map(|a| a.to_string()).unwrap_or_else(|_| "?".into())
        );

        // RX ring ~0.5 s at the RX rate; TX ring ~0.5 s at the TX rate.
        let rx_cap = ((rate * 2.0 * 0.5) as usize).next_power_of_two().max(1 << 16);
        let (rx_prod, rx_cons) = RingBuffer::<f32>::new(rx_cap);
        let tx_cap = ((tx_rate * 2.0 * 0.5) as usize).next_power_of_two().max(1 << 15);
        let (tx_prod, tx_cons) = RingBuffer::<f32>::new(tx_cap);
        tracing::debug!(
            "HPSDR: RX ring {rx_cap} floats (~0.5 s @ {rate:.0} Hz), TX ring {tx_cap} floats \
             (~0.5 s @ {tx_rate:.0} Hz)"
        );

        let (ctrl_tx, ctrl_rx) = crossbeam_channel::unbounded();
        let ctx = ThreadCtx {
            socket,
            radio: IpAddr::V4(ip),
            rate_hz: rate,
            rx: rx_prod,
            tx: tx_cons,
            ctrl: ctrl_rx,
        };
        tracing::info!(
            "HPSDR: starting Protocol {protocol} network thread to {ip} \
             (board \"{board}\", RX {rate:.0} Hz, TX {tx_rate:.0} Hz)"
        );
        let join = std::thread::Builder::new()
            .name("sdroxide-hpsdr".into())
            .spawn(move || match protocol {
                1 => protocol1::run(ctx),
                _ => protocol2::run(ctx),
            })
            .map_err(|e| HpsdrError::Msg(format!("spawn network thread: {e}")))?;

        Ok(HpsdrHandle {
            ctrl: ctrl_tx,
            rx: rx_cons,
            tx: tx_prod,
            join: Some(join),
            board,
            protocol,
            sample_rate_hz: rate,
            tx_rate_hz: tx_rate,
        })
    }

    /// Retune the RX NCO.
    pub fn set_rx_freq(&self, hz: f64) {
        tracing::debug!("HPSDR: set RX freq {hz:.0} Hz");
        let _ = self.ctrl.send(Ctrl::RxFreq(hz));
    }

    /// Begin transmitting at `tx_freq_hz`; returns the TX I/Q rate to feed
    /// [`Self::tx_write`].
    pub fn tx_begin(&self, tx_freq_hz: f64) -> f64 {
        tracing::info!("HPSDR: TX begin at {tx_freq_hz:.0} Hz ({:.0} Hz I/Q)", self.tx_rate_hz);
        let _ = self.ctrl.send(Ctrl::TxOn(tx_freq_hz));
        self.tx_rate_hz
    }

    /// Stop transmitting.
    pub fn tx_end(&self) {
        tracing::info!("HPSDR: TX end");
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

/// Round a requested rate to the nearest rate valid for the protocol.
fn clamp_rate(hz: f64, protocol: u8) -> f64 {
    sdroxide_types::HpsdrConfig::rates_for(protocol)
        .iter()
        .copied()
        .min_by(|a, b| (a - hz).abs().partial_cmp(&(b - hz).abs()).unwrap())
        .unwrap_or(1_536_000.0)
}
