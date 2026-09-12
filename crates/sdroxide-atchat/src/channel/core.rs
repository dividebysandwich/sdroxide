//! A port of `ChannelServer` — "the physics of the channel". It contains NO
//! protocol logic (master election, ARQ, chat); it only (1) enforces
//! half-duplex access and (2) broadcasts the audio (distorted) to every
//! station.
//!
//! Two output buses:
//!   - **the protocol bus**: one transmission = one `RX_AUDIO` (the station's
//!     demod waits for the whole burst buffer — the same as `client.py`).
//!   - **the monitor tap**: the same distorted samples are streamed in real
//!     time in ~20 ms hops (zero while idle). The scope / waterfall / cpal
//!     consume this; it has no effect on the protocol.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use base64::Engine as _;
use tokio::sync::{broadcast, mpsc};
use tokio::time::{Duration, Instant, MissedTickBehavior};

use crate::netproto::ServerMsg;

use super::config::{ChannelConfig, SAMPLE_RATE, apply_channel};

const BASE64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::STANDARD;

/// Monitor tap hop size: 160 samples @ 8 kHz = 20 ms.
const MONITOR_CHUNK: usize = SAMPLE_RATE as usize / 50;

pub type ClientId = u64;

/// Events published for the GUI's activity log.
#[derive(Debug, Clone)]
pub enum ChannelEvent {
    Joined {
        callsign: String,
        active: usize,
    },
    Left {
        callsign: String,
        active: usize,
    },
    TxGranted {
        src: String,
        n_samples: usize,
        duration: f64,
    },
    TxDenied {
        src: String,
        retry_after: f64,
    },
    Delivered {
        src: String,
        n_samples: usize,
    },
    /// Passive "monitor" decode — the delivered (distorted) burst was
    /// demodulated. The equivalent of `monitor.py`'s text log.
    Decoded {
        duration: f64,
        /// `"TA1ABC -> ALL | CHAT"` if it decoded, `None` otherwise.
        summary: Option<String>,
    },
}

/// A channel snapshot the GUI can read every frame.
#[derive(Debug, Clone)]
pub struct ChannelSnapshot {
    pub busy: bool,
    pub busy_remaining: f64,
    pub current_tx: Option<String>,
    pub active_clients: usize,
    pub cfg: ChannelConfig,
}

struct Client {
    callsign: String,
    out: mpsc::Sender<ServerMsg>,
}

struct Inner {
    clients: HashMap<ClientId, Client>,
    busy_until: Instant,
    busy_src: Option<String>,
    cfg: ChannelConfig,
}

pub struct ChannelCore {
    inner: Mutex<Inner>,
    next_id: AtomicU64,
    /// Delivered-burst counter (for the `corrupt_burst_nums` TEST hook).
    burst_count: AtomicU64,
    events: broadcast::Sender<ChannelEvent>,
    monitor: broadcast::Sender<Vec<i16>>,
    /// The "on-air samples" the monitor pacer drains in real time.
    airwaves: Mutex<VecDeque<i16>>,
    /// The passive monitor demodulator (for `ChannelEvent::Decoded`).
    decoder: crate::modem::Modem,
}

impl ChannelCore {
    /// Sets up the channel and starts the monitor pacer task. Must be called
    /// from within a tokio runtime.
    pub fn spawn(cfg: ChannelConfig) -> Arc<Self> {
        let (events, _) = broadcast::channel(512);
        let (monitor, _) = broadcast::channel(128);
        let core = Arc::new(Self {
            inner: Mutex::new(Inner {
                clients: HashMap::new(),
                busy_until: Instant::now(),
                busy_src: None,
                cfg,
            }),
            next_id: AtomicU64::new(1),
            burst_count: AtomicU64::new(0),
            events,
            monitor,
            airwaves: Mutex::new(VecDeque::new()),
            decoder: crate::modem::Modem::new(),
        });

        let pacer = Arc::clone(&core);
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_millis(20));
            tick.set_missed_tick_behavior(MissedTickBehavior::Delay);
            loop {
                tick.tick().await;
                let mut chunk = Vec::with_capacity(MONITOR_CHUNK);
                {
                    let mut aw = pacer.airwaves.lock().unwrap();
                    let take = MONITOR_CHUNK.min(aw.len());
                    for _ in 0..take {
                        chunk.push(aw.pop_front().unwrap());
                    }
                }
                chunk.resize(MONITOR_CHUNK, 0);
                let _ = pacer.monitor.send(chunk);
            }
        });

        core
    }

    // -- subscriptions ------------------------------------------------

    pub fn subscribe_events(&self) -> broadcast::Receiver<ChannelEvent> {
        self.events.subscribe()
    }

    /// A continuous 8 kHz i16 stream (20 ms `Vec<i16>` chunks).
    pub fn subscribe_monitor(&self) -> broadcast::Receiver<Vec<i16>> {
        self.monitor.subscribe()
    }

    // -- client registration ----------------------------------------

    pub fn register(&self, callsign: &str) -> (ClientId, mpsc::Receiver<ServerMsg>) {
        let (tx, rx) = mpsc::channel(512);
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let active = {
            let mut inner = self.inner.lock().unwrap();
            inner.clients.insert(id, Client { callsign: callsign.to_string(), out: tx });
            inner.clients.len()
        };
        let _ = self.events.send(ChannelEvent::Joined { callsign: callsign.to_string(), active });
        (id, rx)
    }

    pub fn deregister(&self, id: ClientId) {
        let removed = {
            let mut inner = self.inner.lock().unwrap();
            inner.clients.remove(&id).map(|c| (c.callsign, inner.clients.len()))
        };
        if let Some((callsign, active)) = removed {
            let _ = self.events.send(ChannelEvent::Left { callsign, active });
        }
    }

    // -- configuration ---------------------------------------------

    pub fn set_config(&self, cfg: ChannelConfig) {
        self.inner.lock().unwrap().cfg = cfg;
    }

    pub fn config(&self) -> ChannelConfig {
        self.inner.lock().unwrap().cfg.clone()
    }

    pub fn snapshot(&self) -> ChannelSnapshot {
        let inner = self.inner.lock().unwrap();
        let now = Instant::now();
        let busy = now < inner.busy_until;
        ChannelSnapshot {
            busy,
            busy_remaining: if busy { (inner.busy_until - now).as_secs_f64() } else { 0.0 },
            current_tx: if busy { inner.busy_src.clone() } else { None },
            active_clients: inner.clients.len(),
            cfg: inner.cfg.clone(),
        }
    }

    // -- transmission --------------------------------------------

    /// A port of `handle_transmit` + `deliver_after_delay`. Enforces
    /// half-duplex access; if granted, `duration` later it broadcasts the
    /// distorted audio to EVERY station (the sender included).
    pub fn transmit(self: &Arc<Self>, id: ClientId, samples: Vec<i16>) {
        let n = samples.len();
        let duration = n as f64 / SAMPLE_RATE as f64;
        let now = Instant::now();

        let (src, cfg) = {
            let mut inner = self.inner.lock().unwrap();
            let src = match inner.clients.get(&id) {
                Some(c) => c.callsign.clone(),
                None => return,
            };
            if now < inner.busy_until {
                let retry_after = (inner.busy_until - now).as_secs_f64();
                if let Some(c) = inner.clients.get(&id) {
                    let _ = c.out.try_send(ServerMsg::ChannelBusy { retry_after });
                }
                let _ = self.events.send(ChannelEvent::TxDenied { src, retry_after });
                return;
            }
            inner.busy_until = now + Duration::from_secs_f64(duration);
            inner.busy_src = Some(src.clone());
            if let Some(c) = inner.clients.get(&id) {
                let _ = c.out.try_send(ServerMsg::TxGranted { duration });
            }
            (src, inner.cfg.clone())
        };

        let _ =
            self.events.send(ChannelEvent::TxGranted { src: src.clone(), n_samples: n, duration });

        let burst_num = self.burst_count.fetch_add(1, Ordering::Relaxed) + 1;

        // Compute the distortion ONCE so the monitor and the protocol hear
        // the same audio.
        let distorted = {
            let mut rng = rand::thread_rng();
            let mut d = apply_channel(&samples, &cfg, &mut rng);
            if cfg.corrupt_burst_nums.contains(&burst_num) {
                // TEST hook: zero this burst entirely -> demod is guaranteed to fail.
                d.iter_mut().for_each(|s| *s = 0);
            }
            Arc::new(d)
        };

        // Monitor: straight into airwaves (the pacer drains it in real time).
        {
            let mut aw = self.airwaves.lock().unwrap();
            aw.extend(distorted.iter().copied());
        }

        // Protocol: one RX_AUDIO at the end of the airtime.
        let core = Arc::clone(self);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs_f64(duration)).await;

            let mut bytes = Vec::with_capacity(distorted.len() * 2);
            for s in distorted.iter() {
                bytes.extend_from_slice(&s.to_le_bytes());
            }
            let msg = ServerMsg::RxAudio { audio_b64: BASE64.encode(&bytes) };

            let targets: Vec<mpsc::Sender<ServerMsg>> = {
                let inner = core.inner.lock().unwrap();
                inner.clients.values().map(|c| c.out.clone()).collect()
            };
            for t in targets {
                let _ = t.try_send(msg.clone());
            }

            // The passive monitor decode (the monitor.py equivalent).
            let summary = core.decoder.demodulate(&distorted).and_then(|payload| {
                serde_json::from_slice::<serde_json::Value>(&payload).ok().map(|v| {
                    format!(
                        "{} -> {} | {}",
                        v.get("src").and_then(|x| x.as_str()).unwrap_or("?"),
                        v.get("dst").and_then(|x| x.as_str()).unwrap_or("ALL"),
                        v.get("type").and_then(|x| x.as_str()).unwrap_or("?"),
                    )
                })
            });
            let _ = core.events.send(ChannelEvent::Decoded { duration, summary });

            let _ = core.events.send(ChannelEvent::Delivered { src, n_samples: distorted.len() });
        });
    }
}
