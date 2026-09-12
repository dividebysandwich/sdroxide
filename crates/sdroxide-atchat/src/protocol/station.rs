//! A port of the `Client` class in `client.py` — the NET station protocol logic.
//!
//! The model matches `client.py` exactly: a SINGLE `receive_loop` reads;
//! `send_frame` writes behind a `tx` lock ("PTT"); EVERY send triggered by an
//! incoming frame is started in the background with `tokio::spawn` — it is
//! NEVER `await`ed on the `receive_loop` call chain (CLAUDE.md bug #1:
//! deadlock).
//!
//! Other fixes preserved:
//!   - The control window: a `control_window_pause` pause every
//!     `control_window_every` blocks in `_send_blocks` (CLAUDE.md bug #3).
//!   - Rejoin: never declare yourself master if an active beacon is heard
//!     (`reconnect` only sends a `JOIN_REQUEST`).
//!   - Master conflict: the alphabetically smaller callsign wins.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, Mutex};

use rand::Rng;
use tokio::sync::{Notify, broadcast, oneshot};
use tokio::task::JoinHandle;
use tokio::time::{Duration, Instant};

use crate::channel::{Connector, LinkRx, LinkTx};
use crate::modem::Modem;
use crate::netproto::{BLOCK_SIZE, ClientMsg, Frame, Mode, ServerMsg};

use super::types::*;

fn jitter() -> f64 {
    rand::thread_rng().gen_range(0.05f64..0.3)
}

fn new_transfer_id() -> String {
    format!("{:08x}", rand::random::<u32>())
}

struct StationState {
    role: Role,
    master: Option<String>,
    backup: Option<String>,
    last_beacon_time: Instant,
    roster: BTreeMap<String, RosterEntry>,
    transfers_in: BTreeMap<String, TransferIn>,
    transfers_out: BTreeMap<String, TransferOut>,
}

pub struct StationShared<C: Connector> {
    callsign: String,
    default_mode: Mode,
    cfg: StationConfig,
    connector: C,
    modem: Modem,

    /// "PTT": only one frame is written at a time. `None` -> the link is down.
    tx: tokio::sync::Mutex<Option<C::Tx>>,
    connected: AtomicBool,
    /// The TX_GRANTED / CHANNEL_BUSY reply `send_frame` is waiting for arrives here.
    tx_reply: Mutex<Option<oneshot::Sender<ServerMsg>>>,
    /// The BULK_STATUS an active ARQ loop is waiting for.
    status_waiters: Mutex<HashMap<String, oneshot::Sender<Vec<usize>>>>,

    state: Mutex<StationState>,
    events: broadcast::Sender<StationEvent>,

    /// `receive_loop` takes its next `rx` from here (drop/reconnect).
    rx_slot: tokio::sync::Mutex<Option<C::Rx>>,
    reconnect_notify: Notify,
    /// Bumped at the start of every [`reconnect`](Self::reconnect). `receive_loop`
    /// reads it when it picks up an `rx` and again if that `rx` closes: a close
    /// with the count unchanged is a real drop and marks the link down; a close
    /// after the count moved is just this reconnect swapping the channel, and
    /// must not touch `connected` — the reconnect has already set it true.
    reconnect_epoch: AtomicU64,
    /// Held for the duration of a [`reconnect`](Self::reconnect) so a double
    /// click on REJOIN cannot run two of them into each other.
    reconnecting: AtomicBool,
}

impl<C: Connector> StationShared<C> {
    fn log(&self, msg: impl Into<String>) {
        let m = msg.into();
        tracing::debug!(callsign = %self.callsign, "{m}");
        let _ = self.events.send(StationEvent::Log(format!("[{}] {m}", self.callsign)));
    }

    fn role(&self) -> Role {
        self.state.lock().unwrap().role
    }

    // ------------------------------------------------------------------ //
    // Low-level send (LBT + backoff)
    // ------------------------------------------------------------------ //
    async fn send_frame(&self, frame: Frame, mode: Mode) -> bool {
        let kind = frame.kind();
        if !self.connected.load(Relaxed) {
            self.log(format!(">> {kind}: not sent — link is down"));
            return false;
        }
        let payload = frame.to_json_bytes();
        let samples = self.modem.modulate(&payload, mode);
        let audio_b64 = crate::channel::samples_to_b64(&samples);
        self.log(format!(
            ">> {kind}: {} payload bytes, {} modem samples ({mode:?}) — asking to transmit",
            payload.len(),
            samples.len()
        ));

        let mut backoff = 0.2_f64;
        for attempt in 0..40 {
            if !self.connected.load(Relaxed) {
                self.log(format!(">> {kind}: giving up — link went down mid-send"));
                return false;
            }
            let reply = {
                let mut tx_guard = self.tx.lock().await;
                let Some(tx) = tx_guard.as_mut() else {
                    self.log(format!(">> {kind}: no transmit link — not sent"));
                    return false;
                };
                let (rtx, rrx) = oneshot::channel();
                *self.tx_reply.lock().unwrap() = Some(rtx);
                if tx.send(ClientMsg::TransmitAudio { audio_b64: audio_b64.clone() }).await.is_err()
                {
                    self.connected.store(false, Relaxed);
                    self.log(format!("!! {kind}: send failed, the link is treated as down"));
                    return false;
                }
                match tokio::time::timeout(Duration::from_secs(8), rrx).await {
                    Ok(Ok(m)) => m,
                    _ => {
                        self.connected.store(false, Relaxed);
                        self.log(format!(
                            "!! {kind}: no transmit-grant reply in 8 s, the link is treated as down"
                        ));
                        return false;
                    }
                }
            }; // the tx lock is released here

            match reply {
                ServerMsg::TxGranted { duration } => {
                    self.log(format!(
                        ">> {kind}: on the air ({:.0} ms), attempt {}",
                        duration * 1000.0,
                        attempt + 1
                    ));
                    tokio::time::sleep(Duration::from_secs_f64(duration)).await;
                    return true;
                }
                ServerMsg::ChannelBusy { retry_after } => {
                    self.log(format!(
                        ">> {kind}: channel busy (attempt {}/40), waiting {retry_after:.2}s",
                        attempt + 1
                    ));
                    tokio::time::sleep(Duration::from_secs_f64(retry_after + jitter())).await;
                    backoff = (backoff * 1.7).min(3.0);
                    let _ = backoff;
                }
                ServerMsg::RxAudio { .. } => {}
            }
        }
        self.log(format!("!! {kind}: channel stayed busy for 40 tries — the send was given up"));
        false
    }

    // ------------------------------------------------------------------ //
    // Receive loop (the SINGLE reader)
    // ------------------------------------------------------------------ //
    async fn receive_loop(self: Arc<Self>) {
        'outer: loop {
            // The reconnect count as of the moment we pick up this rx — see the
            // `None` arm below.
            let epoch = self.reconnect_epoch.load(Relaxed);
            // IMPORTANT: do NOT hold the rx_slot lock across `.await` —
            // otherwise `reconnect` cannot put the new rx in and `notified()`
            // never completes (deadlock). The lock is held only for the
            // duration of `take()`.
            let maybe_rx = self.rx_slot.lock().await.take();
            let mut rx = match maybe_rx {
                Some(rx) => rx,
                None => {
                    self.reconnect_notify.notified().await;
                    continue 'outer;
                }
            };
            loop {
                match rx.recv().await {
                    None => {
                        // A close with the reconnect count unchanged is a real
                        // drop and marks the link down. A close after a
                        // reconnect bumped the count is just that reconnect
                        // swapping the channel under us — `connected` has
                        // already been set true and must be left alone, or the
                        // rejoined station's first JOIN sees a dead link.
                        if self.reconnect_epoch.load(Relaxed) == epoch {
                            self.connected.store(false, Relaxed);
                        }
                        continue 'outer;
                    }
                    Some(msg) => match msg {
                        ServerMsg::TxGranted { .. } | ServerMsg::ChannelBusy { .. } => {
                            if let Some(s) = self.tx_reply.lock().unwrap().take() {
                                let _ = s.send(msg);
                            }
                        }
                        ServerMsg::RxAudio { audio_b64 } => {
                            let Ok(samples) = crate::channel::b64_to_samples(&audio_b64) else {
                                continue;
                            };
                            let Some(payload) = self.modem.demodulate(&samples) else {
                                continue; // could not decode -> treated as "not heard"
                            };
                            let Ok(frame) = serde_json::from_slice::<Frame>(&payload) else {
                                continue;
                            };
                            self.handle_frame(frame).await;
                        }
                    },
                }
            }
        }
    }

    async fn handle_frame(self: &Arc<Self>, frame: Frame) {
        let src = frame.src().map(|s| s.to_string());
        let dst = frame.dst().to_string();
        let is_self = src.as_deref() == Some(self.callsign.as_str());
        let for_me = dst == "ALL" || dst == self.callsign;

        if !matches!(frame, Frame::BulkBlock { .. }) {
            self.log(format!(
                "<< {} from {} -> {dst}{}",
                frame.kind(),
                src.as_deref().unwrap_or("?"),
                if is_self { " (our own transmit, heard back)" } else { "" }
            ));
        }

        if !is_self && let Some(s) = &src {
            self.touch_roster(s);
        }

        match &frame {
            Frame::Beacon { .. } => self.on_beacon(&frame).await,

            Frame::JoinRequest { .. } if !is_self => {
                let src = src.clone().unwrap_or_default();
                if self.role() == Role::Master {
                    self.log(format!("{src} joined the net"));
                }
                self.request_missing_from(&src);
            }

            Frame::Chat { text, .. } if !is_self && for_me => {
                let scope = if dst == "ALL" { ChatScope::Broadcast } else { ChatScope::Private };
                let _ = self.events.send(StationEvent::Chat {
                    from: src.clone().unwrap_or_default(),
                    scope,
                    text: text.clone(),
                });
            }

            Frame::BulkMeta { .. } if !is_self && for_me => self.on_bulk_meta(&frame),
            Frame::BulkBlock { .. } if for_me => self.on_bulk_block(&frame),

            Frame::BulkEnd { .. } if for_me => {
                let this = Arc::clone(self);
                let f = frame.clone();
                tokio::spawn(async move { this.on_bulk_end(f).await });
            }
            Frame::BulkStatus { .. } if dst == self.callsign => {
                let this = Arc::clone(self);
                let f = frame.clone();
                tokio::spawn(async move { this.on_bulk_status(f).await });
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------------ //
    // Roster
    // ------------------------------------------------------------------ //
    fn touch_roster(self: &Arc<Self>, callsign: &str) {
        let was_lost = {
            let mut st = self.state.lock().unwrap();
            let was_lost =
                st.roster.get(callsign).map(|e| e.status == RosterStatus::Lost).unwrap_or(false);
            st.roster.insert(
                callsign.to_string(),
                RosterEntry { last_seen: Instant::now(), status: RosterStatus::Active },
            );
            was_lost
        };
        let _ = self.events.send(StationEvent::StateChanged);
        if was_lost {
            self.log(format!("{callsign} became visible again"));
            self.request_missing_from(callsign);
        }
    }

    fn age_roster(&self) {
        let now = Instant::now();
        let mut newly_lost = Vec::new();
        let mut changed = false;
        {
            let mut st = self.state.lock().unwrap();
            let remove_to = self.cfg.remove_timeout;
            let lost_to = self.cfg.lost_timeout;
            let before = st.roster.len();
            st.roster.retain(|_, info| now.duration_since(info.last_seen) <= remove_to);
            if st.roster.len() != before {
                changed = true;
            }
            for (c, info) in st.roster.iter_mut() {
                if now.duration_since(info.last_seen) > lost_to && info.status != RosterStatus::Lost
                {
                    info.status = RosterStatus::Lost;
                    newly_lost.push(c.clone());
                    changed = true;
                }
            }
        }
        for c in newly_lost {
            self.log(format!("{c} marked as lost"));
        }
        if changed {
            let _ = self.events.send(StationEvent::StateChanged);
        }
    }

    /// When a station comes back, request the missing blocks for the
    /// half-finished transfers received from it (SPAWN — CLAUDE.md #1).
    fn request_missing_from(self: &Arc<Self>, src: &str) {
        let reqs: Vec<(String, Vec<usize>)> = {
            let st = self.state.lock().unwrap();
            st.transfers_in
                .values()
                .filter(|t| t.src == src && !t.complete)
                .filter_map(|t| {
                    let m = t.missing_blocks();
                    (!m.is_empty()).then(|| (t.transfer_id.clone(), m))
                })
                .collect()
        };
        for (tid, missing) in reqs {
            self.log(format!(
                "[{tid}] {src} came back, requesting {} missing blocks",
                missing.len()
            ));
            let this = Arc::clone(self);
            let src = src.to_string();
            let mode = self.default_mode;
            tokio::spawn(async move {
                this.send_frame(
                    Frame::BulkStatus {
                        src: this.callsign.clone(),
                        dst: src,
                        transfer_id: tid,
                        missing,
                    },
                    mode,
                )
                .await;
            });
        }
    }

    // ------------------------------------------------------------------ //
    // Master election / beacon / failover
    // ------------------------------------------------------------------ //
    async fn on_beacon(&self, frame: &Frame) {
        let Frame::Beacon { src, backup, roster, .. } = frame else {
            return;
        };
        let now = Instant::now();
        let mut conflict = false;
        let mut role_evt: Option<Role> = None;
        {
            let mut st = self.state.lock().unwrap();

            if st.role == Role::Master && *src != self.callsign {
                if src.as_str() < self.callsign.as_str() {
                    conflict = true;
                    st.role = Role::Listener;
                    role_evt = Some(Role::Listener);
                } else {
                    return;
                }
            }

            st.last_beacon_time = now;
            st.master = Some(src.clone());
            st.backup = backup.clone();
            for c in roster {
                if c != &self.callsign {
                    let e = st
                        .roster
                        .entry(c.clone())
                        .or_insert(RosterEntry { last_seen: now, status: RosterStatus::Active });
                    e.last_seen = now;
                    e.status = RosterStatus::Active;
                }
            }
            if st.role != Role::Master {
                let r = if st.backup.as_deref() == Some(self.callsign.as_str()) {
                    Role::Backup
                } else {
                    Role::Listener
                };
                if r != st.role {
                    role_evt = Some(r);
                }
                st.role = r;
            }
        }

        if conflict {
            self.log(format!("master conflict: {src} continues, I am backing off"));
        }
        if let Some(r) = role_evt {
            let _ = self.events.send(StationEvent::RoleChanged(r));
        }
        let _ = self.events.send(StationEvent::StateChanged);
    }

    async fn master_watchdog(self: Arc<Self>) {
        let mut tick = tokio::time::interval(Duration::from_secs(1));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tick.tick().await;
            if !self.connected.load(Relaxed) {
                continue;
            }
            self.age_roster();
            let now = Instant::now();
            let take_over = {
                let st = self.state.lock().unwrap();
                st.role != Role::Master
                    && now.duration_since(st.last_beacon_time) > self.cfg.beacon_timeout
                    && (st.role == Role::Backup || st.master.is_none())
            };
            if take_over {
                {
                    let mut st = self.state.lock().unwrap();
                    st.role = Role::Master;
                    st.last_beacon_time = now;
                }
                self.log("beacon timed out -> taking the master role");
                let _ = self.events.send(StationEvent::RoleChanged(Role::Master));
                self.send_beacon().await;
            }
        }
    }

    async fn beacon_loop(self: Arc<Self>) {
        loop {
            tokio::time::sleep(self.cfg.beacon_interval).await;
            if self.role() == Role::Master && self.connected.load(Relaxed) {
                self.send_beacon().await;
            }
        }
    }

    fn pick_backup(&self) -> Option<String> {
        let st = self.state.lock().unwrap();
        st.roster
            .iter()
            .filter(|(c, e)| e.status == RosterStatus::Active && c.as_str() != self.callsign)
            .map(|(c, _)| c.clone())
            .min()
    }

    async fn send_beacon(&self) {
        let backup = self.pick_backup();
        let roster_list: Vec<String> = {
            let mut st = self.state.lock().unwrap();
            st.backup = backup.clone();
            let mut v = vec![self.callsign.clone()];
            v.extend(st.roster.keys().cloned());
            v
        };
        self.send_frame(
            Frame::Beacon {
                src: self.callsign.clone(),
                dst: "ALL".into(),
                backup,
                roster: roster_list,
            },
            Mode::Bpsk,
        )
        .await;
    }

    // ------------------------------------------------------------------ //
    // Chat
    // ------------------------------------------------------------------ //
    async fn chat(&self, text: &str, dst: &str) -> bool {
        self.send_frame(
            Frame::Chat {
                src: self.callsign.clone(),
                dst: dst.to_string(),
                text: text.to_string(),
            },
            Mode::Bpsk,
        )
        .await
    }

    // ------------------------------------------------------------------ //
    // Bulk transfer - send
    // ------------------------------------------------------------------ //
    async fn send_bulk(self: Arc<Self>, path: PathBuf, dst: String) {
        let data = match std::fs::read(&path) {
            Ok(d) => d,
            Err(_) => {
                self.log(format!("file not found: {}", path.display()));
                return;
            }
        };
        let n_blocks = data.len().div_ceil(BLOCK_SIZE).max(1);
        let mut blocks = BTreeMap::new();
        for i in 0..n_blocks {
            let s = i * BLOCK_SIZE;
            let e = (s + BLOCK_SIZE).min(data.len());
            blocks.insert(i, data[s..e].to_vec());
        }
        let transfer_id = new_transfer_id();
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("file").to_string();
        let mode = self.default_mode;
        {
            let mut st = self.state.lock().unwrap();
            st.transfers_out.insert(
                transfer_id.clone(),
                TransferOut {
                    transfer_id: transfer_id.clone(),
                    filename: filename.clone(),
                    dst: dst.clone(),
                    blocks,
                    mode,
                    arq_round: 0,
                    sent: 0,
                    done: false,
                },
            );
        }
        self.log(format!(
            "[{transfer_id}] {filename} -> {dst} starting ({} B, {n_blocks} blocks, {})",
            data.len(),
            mode.as_str()
        ));
        let _ = self.events.send(StationEvent::StateChanged);

        if !self
            .send_frame(
                Frame::BulkMeta {
                    src: self.callsign.clone(),
                    dst: dst.clone(),
                    transfer_id: transfer_id.clone(),
                    filename: filename.clone(),
                    total_blocks: n_blocks,
                    total_size: data.len(),
                },
                mode,
            )
            .await
        {
            self.log(format!("[{transfer_id}] could not start (no link)"));
            return;
        }

        let all: Vec<usize> = (0..n_blocks).collect();
        if !self.send_blocks(&transfer_id, &all).await {
            self.log(format!("[{transfer_id}] the link dropped, the transfer is suspended"));
            return;
        }
        if !self
            .send_frame(
                Frame::BulkEnd {
                    src: self.callsign.clone(),
                    dst: dst.clone(),
                    transfer_id: transfer_id.clone(),
                },
                mode,
            )
            .await
        {
            return;
        }

        for round_no in 0..6 {
            let missing = self.wait_for_status(&transfer_id, Duration::from_secs(6)).await;
            if !self.connected.load(Relaxed) {
                self.log(format!("[{transfer_id}] the link dropped, the transfer is suspended"));
                return;
            }
            let Some(missing) = missing else {
                self.log(format!("[{transfer_id}] no status reply (round {})", round_no + 1));
                continue;
            };
            if missing.is_empty() {
                self.log(format!("[{transfer_id}] complete (round {})", round_no + 1));
                self.mark_out_done(&transfer_id);
                return;
            }
            self.log(format!(
                "[{transfer_id}] resending {} blocks (round {})",
                missing.len(),
                round_no + 1
            ));
            if let Some(t) = self.state.lock().unwrap().transfers_out.get_mut(&transfer_id) {
                t.arq_round = round_no + 1;
            }
            if !self.send_blocks(&transfer_id, &missing).await {
                return;
            }
            self.send_frame(
                Frame::BulkEnd {
                    src: self.callsign.clone(),
                    dst: dst.clone(),
                    transfer_id: transfer_id.clone(),
                },
                mode,
            )
            .await;
        }
        self.log(format!(
            "[{transfer_id}] reached the round limit; it will resume automatically if the receiver comes back"
        ));
    }

    async fn send_blocks(&self, transfer_id: &str, seqs: &[usize]) -> bool {
        let (dst, mode) = {
            let st = self.state.lock().unwrap();
            let Some(t) = st.transfers_out.get(transfer_id) else {
                return false;
            };
            (t.dst.clone(), t.mode)
        };
        for (i, &seq) in seqs.iter().enumerate() {
            if !self.connected.load(Relaxed) {
                return false;
            }
            let data = {
                let st = self.state.lock().unwrap();
                match st.transfers_out.get(transfer_id).and_then(|t| t.blocks.get(&seq)) {
                    Some(d) => d.clone(),
                    None => return false,
                }
            };
            let crc = crate::netproto::crc32(&data);
            let ok = self
                .send_frame(
                    Frame::BulkBlock {
                        src: self.callsign.clone(),
                        dst: dst.clone(),
                        transfer_id: transfer_id.to_string(),
                        seq,
                        data: crate::netproto::b64e(&data),
                        crc,
                    },
                    mode,
                )
                .await;
            if !ok {
                return false;
            }
            if let Some(t) = self.state.lock().unwrap().transfers_out.get_mut(transfer_id) {
                t.sent += 1;
            }
            let _ = self.events.send(StationEvent::StateChanged);

            // The control window (CLAUDE.md bug #3): a control_window_pause
            // pause every control_window_every blocks — so chat AND BEACONs
            // can get through. The rationale for the values is in CLAUDE.md.
            if (i + 1) % self.cfg.control_window_every == 0 {
                tokio::time::sleep(self.cfg.control_window_pause).await;
            }
        }
        true
    }

    async fn wait_for_status(&self, transfer_id: &str, timeout: Duration) -> Option<Vec<usize>> {
        let (tx, rx) = oneshot::channel();
        self.status_waiters.lock().unwrap().insert(transfer_id.to_string(), tx);
        let res = tokio::time::timeout(timeout, rx).await;
        self.status_waiters.lock().unwrap().remove(transfer_id);
        match res {
            Ok(Ok(missing)) => Some(missing),
            _ => None,
        }
    }

    async fn on_bulk_status(self: &Arc<Self>, frame: Frame) {
        let Frame::BulkStatus { transfer_id, missing, .. } = frame else {
            return;
        };

        if let Some(tx) = self.status_waiters.lock().unwrap().remove(&transfer_id) {
            let _ = tx.send(missing);
            return;
        }

        // No active ARQ loop but we still hold the blocks -> a delayed "carry on".
        let dst = {
            let st = self.state.lock().unwrap();
            st.transfers_out.get(&transfer_id).map(|t| t.dst.clone())
        };
        if let Some(dst) = dst
            && !missing.is_empty()
        {
            self.log(format!(
                "[{transfer_id}] delayed resume request: resending {} blocks",
                missing.len()
            ));
            let this = Arc::clone(self);
            let mode = self.default_mode;
            tokio::spawn(async move {
                if this.send_blocks(&transfer_id, &missing).await {
                    this.send_frame(
                        Frame::BulkEnd {
                            src: this.callsign.clone(),
                            dst,
                            transfer_id: transfer_id.clone(),
                        },
                        mode,
                    )
                    .await;
                }
            });
        }
    }

    // ------------------------------------------------------------------ //
    // Bulk transfer - receive
    // ------------------------------------------------------------------ //
    fn on_bulk_meta(&self, frame: &Frame) {
        let Frame::BulkMeta { src, dst, transfer_id, filename, total_blocks, .. } = frame else {
            return;
        };
        {
            let mut st = self.state.lock().unwrap();
            st.transfers_in.insert(
                transfer_id.clone(),
                TransferIn {
                    transfer_id: transfer_id.clone(),
                    filename: filename.clone(),
                    total_blocks: *total_blocks,
                    src: src.clone(),
                    dst: dst.clone(),
                    received: BTreeMap::new(),
                    complete: false,
                    saved_path: None,
                },
            );
        }
        self.log(format!(
            "[{transfer_id}] {src} started a transfer: {filename} ({total_blocks} blocks)"
        ));
        let _ = self.events.send(StationEvent::StateChanged);
    }

    fn on_bulk_block(&self, frame: &Frame) {
        let Frame::BulkBlock { transfer_id, seq, data, crc, .. } = frame else {
            return;
        };
        let Ok(bytes) = crate::netproto::b64d(data) else {
            return;
        };
        if crate::netproto::crc32(&bytes) != *crc {
            return; // CRC mismatch -> ignore it, ARQ will re-request it
        }
        {
            let mut st = self.state.lock().unwrap();
            if let Some(t) = st.transfers_in.get_mut(transfer_id) {
                if t.complete {
                    return;
                }
                t.received.insert(*seq, bytes);
            }
        }
        let _ = self.events.send(StationEvent::StateChanged);
    }

    async fn on_bulk_end(self: &Arc<Self>, frame: Frame) {
        let Frame::BulkEnd { src, transfer_id, .. } = frame else {
            return;
        };
        let (missing, total, complete_already) = {
            let st = self.state.lock().unwrap();
            let Some(t) = st.transfers_in.get(&transfer_id) else {
                return;
            };
            (t.missing_blocks(), t.total_blocks, t.complete)
        };
        if complete_already {
            return;
        }
        if missing.is_empty() {
            self.save_transfer(&transfer_id);
        } else {
            self.log(format!(
                "[{transfer_id}] {}/{total} blocks missing, requesting them",
                missing.len()
            ));
        }
        // Send BULK_STATUS in EVERY case (SPAWN — CLAUDE.md #1).
        let this = Arc::clone(self);
        tokio::spawn(async move {
            this.send_frame(
                Frame::BulkStatus { src: this.callsign.clone(), dst: src, transfer_id, missing },
                Mode::Bpsk,
            )
            .await;
        });
    }

    fn save_transfer(&self, transfer_id: &str) {
        let (data, filename, src) = {
            let mut st = self.state.lock().unwrap();
            let Some(t) = st.transfers_in.get_mut(transfer_id) else {
                return;
            };
            t.complete = true;
            let mut buf = Vec::new();
            for seq in 0..t.total_blocks {
                if let Some(b) = t.received.get(&seq) {
                    buf.extend_from_slice(b);
                }
            }
            (buf, t.filename.clone(), t.src.clone())
        };
        let _ = std::fs::create_dir_all(&self.cfg.received_dir);
        let out_path = self.cfg.received_dir.join(format!("{transfer_id}_{filename}"));
        if let Err(e) = std::fs::write(&out_path, &data) {
            self.log(format!("[{transfer_id}] could not be saved: {e}"));
            return;
        }
        let p = out_path.display().to_string();
        if let Some(t) = self.state.lock().unwrap().transfers_in.get_mut(transfer_id) {
            t.saved_path = Some(p.clone());
        }
        self.log(format!("[{transfer_id}] complete -> {p}"));
        let _ = self.events.send(StationEvent::Transfer {
            id: transfer_id.to_string(),
            dir: TransferDir::In,
            filename,
            peer: src,
            have: 0,
            total: 0,
            done: true,
            saved_path: Some(p),
        });
        let _ = self.events.send(StationEvent::StateChanged);
    }

    fn mark_out_done(&self, transfer_id: &str) {
        if let Some(t) = self.state.lock().unwrap().transfers_out.get_mut(transfer_id) {
            t.done = true;
        }
        let _ = self.events.send(StationEvent::StateChanged);
    }

    // ------------------------------------------------------------------ //
    // Connection management
    // ------------------------------------------------------------------ //
    async fn drop_link(&self) {
        self.connected.store(false, Relaxed);
        if let Some(mut tx) = self.tx.lock().await.take() {
            tx.close();
        }
        self.log(
            "!! link dropped (simulated) - state is preserved, you can come back with /reconnect",
        );
        let _ = self.events.send(StationEvent::StateChanged);
    }

    async fn reconnect(self: &Arc<Self>) {
        if self.connected.load(Relaxed) {
            self.log("already connected");
            return;
        }
        if self.reconnecting.swap(true, Relaxed) {
            self.log("reconnect already in progress — ignoring the repeat");
            return;
        }
        // Every early return from here on must clear `reconnecting`.
        self.reconnect_epoch.fetch_add(1, Relaxed);
        self.log("reconnecting — dialing the channel");
        let (tx, rx) = match self.connector.connect().await {
            Ok(p) => p,
            Err(e) => {
                self.log(format!("!! could not reconnect: {e}"));
                self.reconnecting.store(false, Relaxed);
                return;
            }
        };
        *self.tx.lock().await = Some(tx);
        *self.rx_slot.lock().await = Some(rx);
        self.connected.store(true, Relaxed);
        {
            let mut st = self.state.lock().unwrap();
            st.role = Role::Listener; // rejoin: never declare master if an active beacon is heard
            st.last_beacon_time = Instant::now();
        }
        self.reconnect_notify.notify_one();
        self.reconnecting.store(false, Relaxed);
        self.log("reconnected to the channel");
        let _ = self.events.send(StationEvent::RoleChanged(Role::Listener));

        let this = Arc::clone(self);
        let mode = self.default_mode;
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            this.send_frame(
                Frame::JoinRequest { src: this.callsign.clone(), dst: "ALL".into() },
                mode,
            )
            .await;
            this.resume_pending_receives();
        });
    }

    fn resume_pending_receives(self: &Arc<Self>) {
        let reqs: Vec<(String, String, Vec<usize>)> = {
            let st = self.state.lock().unwrap();
            st.transfers_in
                .values()
                .filter(|t| !t.complete)
                .filter_map(|t| {
                    let m = t.missing_blocks();
                    (!m.is_empty()).then(|| (t.transfer_id.clone(), t.src.clone(), m))
                })
                .collect()
        };
        for (tid, src, missing) in reqs {
            self.log(format!(
                "[{tid}] we are back, requesting {} missing blocks from {src}",
                missing.len()
            ));
            let this = Arc::clone(self);
            let mode = self.default_mode;
            tokio::spawn(async move {
                this.send_frame(
                    Frame::BulkStatus {
                        src: this.callsign.clone(),
                        dst: src,
                        transfer_id: tid,
                        missing,
                    },
                    mode,
                )
                .await;
            });
        }
    }

    fn snapshot(&self) -> StationSnapshot {
        let st = self.state.lock().unwrap();
        let now = Instant::now();
        StationSnapshot {
            callsign: self.callsign.clone(),
            role: st.role,
            master: st.master.clone(),
            backup: st.backup.clone(),
            connected: self.connected.load(Relaxed),
            roster: st
                .roster
                .iter()
                .map(|(c, e)| (c.clone(), e.status, now.duration_since(e.last_seen).as_secs_f64()))
                .collect(),
            transfers_in: st
                .transfers_in
                .values()
                .map(|t| TransferSnapshot {
                    id: t.transfer_id.clone(),
                    filename: t.filename.clone(),
                    peer: t.src.clone(),
                    have: t.received.len(),
                    total: t.total_blocks,
                    complete: t.complete,
                    arq_round: 0,
                })
                .collect(),
            transfers_out: st
                .transfers_out
                .values()
                .map(|t| TransferSnapshot {
                    id: t.transfer_id.clone(),
                    filename: t.filename.clone(),
                    peer: t.dst.clone(),
                    have: t.sent.min(t.blocks.len()),
                    total: t.blocks.len(),
                    complete: t.done,
                    arq_round: t.arq_round,
                })
                .collect(),
        }
    }
}

// ------------------------------------------------------------------ //
// Public handle
// ------------------------------------------------------------------ //

/// A NET station. `start` connects and starts the background tasks; when it is
/// dropped the tasks are aborted.
pub struct Station<C: Connector> {
    shared: Arc<StationShared<C>>,
    tasks: Vec<JoinHandle<()>>,
}

impl<C: Connector> Station<C> {
    pub async fn start(
        connector: C,
        callsign: &str,
        mode: Mode,
        cfg: StationConfig,
    ) -> anyhow::Result<Self> {
        let (tx, rx) = connector.connect().await?;
        let (events, _) = broadcast::channel(512);
        let shared = Arc::new(StationShared {
            callsign: callsign.to_uppercase(),
            default_mode: mode,
            cfg,
            connector,
            modem: Modem::new(),
            tx: tokio::sync::Mutex::new(Some(tx)),
            connected: AtomicBool::new(true),
            tx_reply: Mutex::new(None),
            status_waiters: Mutex::new(HashMap::new()),
            state: Mutex::new(StationState {
                role: Role::Listener,
                master: None,
                backup: None,
                last_beacon_time: Instant::now(),
                roster: BTreeMap::new(),
                transfers_in: BTreeMap::new(),
                transfers_out: BTreeMap::new(),
            }),
            events,
            rx_slot: tokio::sync::Mutex::new(Some(rx)),
            reconnect_notify: Notify::new(),
            reconnect_epoch: AtomicU64::new(0),
            reconnecting: AtomicBool::new(false),
        });
        let _ = std::fs::create_dir_all(&shared.cfg.received_dir);

        shared.log(format!(
            "station started as {} ({:?}); receive/watchdog/beacon tasks up",
            shared.callsign, shared.default_mode
        ));

        let tasks = vec![
            tokio::spawn(StationShared::receive_loop(Arc::clone(&shared))),
            tokio::spawn(StationShared::master_watchdog(Arc::clone(&shared))),
            tokio::spawn(StationShared::beacon_loop(Arc::clone(&shared))),
        ];

        // The first JOIN_REQUEST — a short delay so receive_loop's first turn
        // runs (CLAUDE.md: otherwise the first TX_GRANTED is missed).
        let s = Arc::clone(&shared);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            s.log("joining the net — sending the first JOIN_REQUEST");
            s.send_frame(
                Frame::JoinRequest { src: s.callsign.clone(), dst: "ALL".into() },
                s.default_mode,
            )
            .await;
        });

        Ok(Self { shared, tasks })
    }

    pub fn callsign(&self) -> &str {
        &self.shared.callsign
    }
    pub fn subscribe(&self) -> broadcast::Receiver<StationEvent> {
        self.shared.events.subscribe()
    }
    pub fn snapshot(&self) -> StationSnapshot {
        self.shared.snapshot()
    }
    pub fn role(&self) -> Role {
        self.shared.role()
    }
    pub fn is_connected(&self) -> bool {
        self.shared.connected.load(Relaxed)
    }

    pub async fn chat(&self, text: &str, dst: &str) -> bool {
        self.shared.chat(text, dst).await
    }

    /// Send a file/image (runs in the background).
    pub fn send_file(&self, path: impl Into<PathBuf>, dst: &str) {
        let s = Arc::clone(&self.shared);
        let path = path.into();
        let dst = dst.to_string();
        tokio::spawn(async move { s.send_bulk(path, dst).await });
    }

    pub async fn drop_link(&self) {
        self.shared.drop_link().await
    }
    pub async fn reconnect(&self) {
        let s = Arc::clone(&self.shared);
        s.reconnect().await
    }

    // -- fire-and-forget variants (so the GUI command loop is not blocked) --

    pub fn chat_bg(&self, text: &str, dst: &str) {
        let s = Arc::clone(&self.shared);
        let (t, d) = (text.to_string(), dst.to_string());
        tokio::spawn(async move {
            s.chat(&t, &d).await;
        });
    }

    pub fn drop_bg(&self) {
        let s = Arc::clone(&self.shared);
        tokio::spawn(async move { s.drop_link().await });
    }

    pub fn reconnect_bg(&self) {
        let s = Arc::clone(&self.shared);
        tokio::spawn(async move { s.reconnect().await });
    }
}

impl<C: Connector> Drop for Station<C> {
    fn drop(&mut self) {
        for t in &self.tasks {
            t.abort();
        }
    }
}
