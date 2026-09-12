//! `RadioConnector` — the [`Station`](crate::protocol::Station)'s channel when
//! it runs on the real radio instead of the virtual TCP one.
//!
//! There is no half-duplex arbiter server: a transmit is granted straight away
//! *unless* the receiver currently hears a carrier, which turns the Station's
//! LBT loop into real listen-before-transmit. A granted burst is queued as
//! 8 kHz PCM for the audio thread to resample and play; incoming demodulated
//! audio arrives from the audio thread and is energy-segmented back into
//! bursts. The [`RadioBridge`] is the sync handle the DigiEngine controller
//! holds; the [`RadioConnector`] is what `Station::start` is given.

#![allow(clippy::manual_async_fn)]

use std::collections::VecDeque;
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use rand::Rng;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::link::{Connector, LinkRx, LinkTx, b64_to_samples, samples_to_b64};
use crate::netproto::{ClientMsg, SAMPLE_RATE, ServerMsg};

const HOP: usize = SAMPLE_RATE as usize / 50; // 20 ms
/// Quiet run that ends a burst (~60 ms).
const SILENCE_HANG_HOPS: usize = 3;
/// Absolute noise floor (i16 peak) below which a hop is silence no matter what
/// the adaptive floor has learned — keeps a genuinely dead input (a loopback,
/// an unplugged rig) from ever reading as a carrier.
const GATE: i32 = 250;
/// A hop counts as a carrier when its peak is this many times the learned
/// ambient floor. ~3× ≈ 9.5 dB over the noise.
const CARRIER_OVER_FLOOR: f64 = 3.0;
/// How fast the ambient-floor estimate tracks the input, per 20 ms hop. ~0.05
/// settles in about half a second, so the first transmit after joining waits
/// out a brief measure rather than being blocked forever by a floor that
/// started at zero.
const FLOOR_ALPHA: f64 = 0.05;
/// Where the ambient-floor estimate starts, before any audio has been measured
/// — permissive on purpose: a station that has just joined should be able to
/// call rather than sit deaf behind an unlearned floor.
const FLOOR_INIT: f64 = 4000.0;
/// A burst shorter than this is noise, not a frame (~2 OFDM symbols).
const MIN_BURST: usize = 640;
const RX_RING_CAP: usize = SAMPLE_RATE as usize * 6;
const TX_RING_CAP: usize = SAMPLE_RATE as usize * 30;

fn jitter() -> f64 {
    rand::thread_rng().gen_range(0.05f64..0.35)
}

/// How many diagnostic lines the bridge keeps before dropping the oldest.
const DIAG_CAP: usize = 300;

/// The audio-thread-facing handle. Cheap to clone; all state is shared.
#[derive(Clone)]
pub struct RadioBridge {
    rx_pcm: Arc<Mutex<VecDeque<i16>>>,
    tx_pcm: Arc<Mutex<VecDeque<i16>>>,
    carrier: Arc<AtomicBool>,
    seg: Arc<Mutex<Option<JoinHandle<()>>>>,
    /// Lines the RF channel wants shown in the station log — the segmenter and
    /// the transmit-grant path have no `Station` handle, so they leave notes
    /// here and `session_main` drains them into the snapshot each tick.
    diag: Arc<Mutex<Vec<String>>>,
}

impl Default for RadioBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl RadioBridge {
    pub fn new() -> Self {
        Self {
            rx_pcm: Arc::new(Mutex::new(VecDeque::new())),
            tx_pcm: Arc::new(Mutex::new(VecDeque::new())),
            carrier: Arc::new(AtomicBool::new(false)),
            seg: Arc::new(Mutex::new(None)),
            diag: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Leave a diagnostic line for the station log.
    pub fn push_diag(&self, msg: impl Into<String>) {
        let mut d = self.diag.lock().unwrap();
        d.push(msg.into());
        let over = d.len().saturating_sub(DIAG_CAP);
        if over > 0 {
            d.drain(..over);
        }
    }

    /// Take everything logged since the last drain.
    pub fn drain_diag(&self) -> Vec<String> {
        std::mem::take(&mut *self.diag.lock().unwrap())
    }

    /// Feed demodulated receive audio (8 kHz mono i16).
    pub fn push_rx(&self, pcm: &[i16]) {
        let mut r = self.rx_pcm.lock().unwrap();
        r.extend(pcm.iter().copied());
        let over = r.len().saturating_sub(RX_RING_CAP);
        for _ in 0..over {
            r.pop_front();
        }
    }

    /// Drain up to `max` modulated transmit samples (8 kHz mono i16) into
    /// `out`. Returns `true` while a burst is still queued.
    pub fn drain_tx(&self, out: &mut Vec<i16>, max: usize) -> usize {
        let mut t = self.tx_pcm.lock().unwrap();
        let n = max.min(t.len());
        out.extend(t.drain(..n));
        n
    }

    pub fn tx_pending(&self) -> bool {
        !self.tx_pcm.lock().unwrap().is_empty()
    }

    /// True while the receiver hears a carrier — the Station's LBT test.
    pub fn carrier_sense(&self) -> bool {
        self.carrier.load(Ordering::Relaxed)
    }

    fn clear_tx(&self) {
        self.tx_pcm.lock().unwrap().clear();
    }
}

/// Given to `Station::start`; each `connect()` (re)starts the segmenter.
pub struct RadioConnector {
    bridge: RadioBridge,
}

impl RadioConnector {
    pub fn new(bridge: RadioBridge) -> Self {
        Self { bridge }
    }
}

impl Connector for RadioConnector {
    type Tx = RadioTx;
    type Rx = RadioRx;

    fn connect(&self) -> impl Future<Output = anyhow::Result<(Self::Tx, Self::Rx)>> + Send + '_ {
        async move {
            let (tx, rx) = mpsc::channel::<ServerMsg>(256);
            // Replace any previous segmenter.
            if let Some(h) = self.bridge.seg.lock().unwrap().take() {
                h.abort();
            }
            let handle = tokio::spawn(segmenter(
                Arc::clone(&self.bridge.rx_pcm),
                Arc::clone(&self.bridge.carrier),
                tx.clone(),
                self.bridge.clone(),
            ));
            *self.bridge.seg.lock().unwrap() = Some(handle);
            self.bridge.push_diag("RF: link connected, carrier segmenter (re)started");
            Ok((RadioTx { bridge: self.bridge.clone(), reply: tx }, RadioRx { rx }))
        }
    }
}

pub struct RadioTx {
    bridge: RadioBridge,
    reply: mpsc::Sender<ServerMsg>,
}

impl LinkTx for RadioTx {
    fn send(&mut self, msg: ClientMsg) -> impl Future<Output = anyhow::Result<()>> + Send + '_ {
        async move {
            let ClientMsg::TransmitAudio { audio_b64 } = msg else {
                return Ok(()); // HELLO: nothing to do
            };
            let samples = b64_to_samples(&audio_b64)?;
            if self.bridge.carrier_sense() {
                let retry_after = 0.4 + jitter();
                self.bridge.push_diag(format!(
                    "RF: transmit held — carrier heard; retry in {retry_after:.2}s"
                ));
                let _ = self.reply.send(ServerMsg::ChannelBusy { retry_after }).await;
            } else {
                let n = samples.len();
                let duration = n as f64 / SAMPLE_RATE as f64;
                let ring = {
                    let mut t = self.bridge.tx_pcm.lock().unwrap();
                    t.extend(samples);
                    let over = t.len().saturating_sub(TX_RING_CAP);
                    for _ in 0..over {
                        t.pop_front();
                    }
                    t.len()
                };
                self.bridge.push_diag(format!(
                    "RF: transmit granted — {n} samples ({:.0} ms) queued, tx ring {ring}",
                    duration * 1000.0
                ));
                let _ = self.reply.send(ServerMsg::TxGranted { duration }).await;
            }
            Ok(())
        }
    }

    fn close(&mut self) {
        self.bridge.clear_tx();
    }
}

pub struct RadioRx {
    rx: mpsc::Receiver<ServerMsg>,
}

impl LinkRx for RadioRx {
    fn recv(&mut self) -> impl Future<Output = Option<ServerMsg>> + Send + '_ {
        async move { self.rx.recv().await }
    }
}

/// Energy-gate the receive PCM into whole bursts and emit each as `RX_AUDIO`.
async fn segmenter(
    rx_pcm: Arc<Mutex<VecDeque<i16>>>,
    carrier: Arc<AtomicBool>,
    out: mpsc::Sender<ServerMsg>,
    bridge: RadioBridge,
) {
    let mut tick = tokio::time::interval(tokio::time::Duration::from_millis(15));
    let mut burst: Vec<i16> = Vec::new();
    let mut quiet_hops = 0usize;
    let mut in_burst = false;
    // Ambient level the "is the channel busy?" test measures against. A fixed
    // gate worked on the loopback, where the only thing in the ring is a frame,
    // but real receiver audio sits well above any fixed threshold after the
    // rig's AGC — so the station would read a permanent carrier and never key.
    let mut noise_floor = FLOOR_INIT;
    // Diagnostics: report a carrier edge as it happens, plus a level line every
    // ~4 s so the log shows the floor settling even on a quiet channel.
    let mut carrier_prev = false;
    let mut hops_since_report = 0u32;
    let mut fed_any = false;

    loop {
        tick.tick().await;
        loop {
            let hop: Vec<i16> = {
                let mut r = rx_pcm.lock().unwrap();
                if r.len() < HOP {
                    break;
                }
                r.drain(..HOP).collect()
            };
            if !fed_any {
                fed_any = true;
                bridge.push_diag("RF: first receive audio reached the segmenter");
            }
            let peak = hop.iter().map(|s| (*s as i32).abs()).max().unwrap_or(0);
            let threshold = (noise_floor * CARRIER_OVER_FLOOR).max(GATE as f64);
            let loud = f64::from(peak) > threshold;
            // Learn the ambient level only from hops that are neither a carrier
            // nor part of the burst being captured, so a long over does not pull
            // the floor up to its own level.
            if !loud && !in_burst {
                noise_floor += FLOOR_ALPHA * (f64::from(peak) - noise_floor);
            }
            let carrier_now = loud || (in_burst && quiet_hops < SILENCE_HANG_HOPS);
            carrier.store(carrier_now, Ordering::Relaxed);

            hops_since_report += 1;
            if carrier_now != carrier_prev {
                bridge.push_diag(format!(
                    "RF: carrier {} (peak {peak}, threshold {threshold:.0}, floor {noise_floor:.0})",
                    if carrier_now { "detected" } else { "cleared" }
                ));
                carrier_prev = carrier_now;
                hops_since_report = 0;
            } else if hops_since_report >= 200 {
                bridge.push_diag(format!(
                    "RF: channel {} — peak {peak}, threshold {threshold:.0}, floor {noise_floor:.0}",
                    if carrier_now { "busy" } else { "clear" }
                ));
                hops_since_report = 0;
            }

            if loud {
                if !in_burst {
                    in_burst = true;
                    burst.clear();
                }
                quiet_hops = 0;
                burst.extend_from_slice(&hop);
            } else if in_burst {
                burst.extend_from_slice(&hop);
                quiet_hops += 1;
                if quiet_hops >= SILENCE_HANG_HOPS {
                    in_burst = false;
                    if burst.len() >= MIN_BURST {
                        bridge.push_diag(format!(
                            "RF: captured a {}-sample burst, handing it to the demod",
                            burst.len()
                        ));
                        let b64 = samples_to_b64(&burst);
                        let _ = out.send(ServerMsg::RxAudio { audio_b64: b64 }).await;
                    }
                    burst.clear();
                }
            }
        }
    }
}
