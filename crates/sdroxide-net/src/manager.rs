//! The spot manager: owns the feed threads (DX cluster, POTA, SOTA, PSK
//! Reporter) and the on-demand worker threads (lookup, upload, confirmations),
//! merges spots across feeds, prunes by age, and hands the engine a stream of
//! [`NetEvent`]s to forward as `RadioEvent`s.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crossbeam_channel::Receiver;
use sdroxide_types::{
    NetworkConfig, Spot, SpotKind, UploadResult, UploadTarget, WsprSpot, grid_to_latlon,
};

use crate::cluster::ClusterHandle;
use crate::event::{EventTx, FeedBatch, FeedTx, NetEvent};
use crate::freedvreporter::ReporterHandle;
use crate::poll::{self, PollHandle};
use crate::rbn::RbnHandle;
use crate::wsprnet::{self, WsprPollHandle, WsprUploadHandle};
use crate::{pota, pskreporter, sota};

/// UTC seconds now (native-only wall clock).
fn now_utc() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

pub struct SpotManager {
    cfg: NetworkConfig,
    feed_tx: FeedTx,
    feed_rx: Receiver<FeedBatch>,
    event_tx: EventTx,
    event_rx: Receiver<NetEvent>,
    /// Latest spots per feed kind (each feed replaces its own set).
    by_kind: HashMap<SpotKind, Vec<Spot>>,
    last_snapshot: Vec<Spot>,
    /// Current dial frequency (Hz) as bits, shared with the PSK feed.
    dial_bits: Arc<AtomicU64>,

    cluster: Option<ClusterHandle>,
    /// The Reverse Beacon Network reader. Emits paths, not spots, so unlike
    /// every other feed here it has no entry in `by_kind`.
    rbn: Option<RbnHandle>,
    pota: Option<PollHandle>,
    sota: Option<PollHandle>,
    psk: Option<PollHandle>,
    /// The operator's callsign and grid, pushed in from the digi config by the
    /// engine. Not part of [`NetworkConfig`]: there is one operator identity in
    /// the app and it is set on the General tab.
    op_call: String,
    op_grid: String,

    /// PSK Reporter upload worker (what *we* hear), when reporting is on and
    /// the operator identity is known.
    psk_upload: Option<crate::pskupload::PskUploadHandle>,

    /// WSPRnet upload worker (what we decoded), when reporting is on and the
    /// operator identity is known.
    wspr_upload: Option<WsprUploadHandle>,
    /// WSPRnet download poller: who heard us.
    wspr_heard_us: Option<WsprPollHandle>,

    freedv: Option<ReporterHandle>,
    /// What we last told the reporter. Replayed into a freshly rebuilt session
    /// so a config change never leaves the site showing a stale frequency.
    rep_freq: u64,
    rep_tx: bool,
    rep_visible: bool,

    /// Whether this manager may hold the station's long-lived feeds.
    ///
    /// A station has one DX cluster login, one RBN reader and one FreeDV
    /// Reporter session, not one per radio, so only the primary engine's
    /// manager brings them up. The configuration is still applied everywhere —
    /// callsign lookups and logbook uploads are per-request work any radio can
    /// do, and they need the credentials — which is exactly why this cannot be
    /// a matter of simply not calling [`SpotManager::set_config`]: settings
    /// applied from a second radio's window have to reach it too.
    station: bool,
}

impl SpotManager {
    /// Create an idle manager (no feeds until [`SpotManager::set_config`]).
    pub fn new() -> Self {
        let (feed_tx, feed_rx) = crossbeam_channel::unbounded();
        let (event_tx, event_rx) = crossbeam_channel::unbounded();
        SpotManager {
            cfg: NetworkConfig::default(),
            feed_tx,
            feed_rx,
            event_tx,
            event_rx,
            by_kind: HashMap::new(),
            last_snapshot: Vec::new(),
            dial_bits: Arc::new(AtomicU64::new(14_074_000f64.to_bits())),
            cluster: None,
            rbn: None,
            pota: None,
            sota: None,
            psk: None,
            psk_upload: None,
            wspr_upload: None,
            wspr_heard_us: None,
            op_call: String::new(),
            op_grid: String::new(),
            freedv: None,
            rep_freq: 0,
            rep_tx: false,
            rep_visible: false,
            station: true,
        }
    }

    /// Declare that this manager does *not* run the station's feeds — the
    /// engine that owns it is one of the extra radios, not the primary.
    ///
    /// Configuration still applies; only the long-lived logins and sockets stay
    /// away. Call before the first [`SpotManager::set_config`]: anything already
    /// running keeps running.
    pub fn stand_down(&mut self) {
        self.station = false;
    }

    /// Apply a new configuration, (re)starting only the feeds whose settings
    /// changed. Disabled feeds have their threads dropped and spots cleared.
    pub fn set_config(&mut self, cfg: NetworkConfig) {
        let old = std::mem::replace(&mut self.cfg, cfg);
        if old.cluster != self.cfg.cluster {
            self.rebuild_cluster();
        }
        if old.rbn != self.cfg.rbn {
            self.rebuild_rbn();
        }
        if old.pota != self.cfg.pota {
            self.rebuild_pota();
        }
        if old.sota != self.cfg.sota {
            self.rebuild_sota();
        }
        if old.psk != self.cfg.psk {
            self.rebuild_psk();
            self.rebuild_psk_upload();
        }
        if old.wspr != self.cfg.wspr {
            self.rebuild_wspr();
        }
        // The reporter sends its settings at connect, so a change to them has
        // to restart the session. The status message is the one field that can
        // be pushed down a live session, and editing a line of text should not
        // cost a reconnect.
        let rep_changed = {
            let mut without_message = old.freedv_reporter.clone();
            without_message.message = self.cfg.freedv_reporter.message.clone();
            without_message != self.cfg.freedv_reporter
        };
        if rep_changed {
            self.rebuild_freedv();
        } else if old.freedv_reporter.message != self.cfg.freedv_reporter.message
            && let Some(h) = &self.freedv
        {
            h.set_message(self.cfg.freedv_reporter.message.clone());
        }
    }

    /// Set the operator's callsign and grid, which the engine takes from the
    /// digi config so the whole app reports one identity.
    ///
    /// Both are sent when a session opens rather than during it, so a change
    /// restarts the feeds that carry them.
    pub fn set_operator(&mut self, call: &str, grid: &str) {
        let (call, grid) = (call.trim(), grid.trim());
        if call == self.op_call && grid == self.op_grid {
            return;
        }
        let call_changed = call != self.op_call;
        self.op_call = call.to_string();
        self.op_grid = grid.to_string();
        // The cluster logs in with the callsign; the reporter sends both.
        if call_changed && self.cfg.cluster.login.trim().is_empty() {
            self.rebuild_cluster();
        }
        // RBN logs in the same way, and wants a real callsign.
        if call_changed && self.cfg.rbn.login.trim().is_empty() {
            self.rebuild_rbn();
        }
        self.rebuild_freedv();
        // Our callsign and grid *are* the PSK Reporter receiver record.
        self.rebuild_psk_upload();
        // And they are the whole of WSPRnet's identity: the callsign in the
        // query is the account.
        self.rebuild_wspr();
    }

    // The engine pushes these on every tick of its ~100 Hz loop, so each one
    // sends only on a change. The cached value is what `rebuild_freedv` replays
    // into a new session, so nothing is lost by not re-sending.

    /// Tell the FreeDV Reporter where we transmit.
    pub fn set_reporter_freq(&mut self, hz: u64) {
        if hz == self.rep_freq {
            return;
        }
        self.rep_freq = hz;
        if let Some(h) = &self.freedv {
            h.set_freq(hz);
        }
    }

    /// Tell the FreeDV Reporter whether we are transmitting.
    pub fn set_reporter_tx(&mut self, on: bool) {
        if on == self.rep_tx {
            return;
        }
        self.rep_tx = on;
        if let Some(h) = &self.freedv {
            h.set_tx(on);
        }
    }

    /// Show or hide this station on FreeDV Reporter. The engine pushes whether
    /// the radio is currently in RADE, so we only appear when we can actually
    /// work FreeDV.
    pub fn set_reporter_visible(&mut self, visible: bool) {
        if visible == self.rep_visible {
            return;
        }
        self.rep_visible = visible;
        if let Some(h) = &self.freedv {
            h.set_visible(visible);
        }
    }

    /// Report a station we decoded to PSK Reporter. Batched and uploaded by
    /// the worker; a no-op when reporting is off or we have no identity yet.
    pub fn psk_report(&self, report: crate::pskupload::Report) {
        if let Some(h) = &self.psk_upload {
            h.report(report);
        }
    }

    /// Report what a WSPR slot decoded to WSPRnet.
    ///
    /// An empty slice is not nothing to say: `function=wsprstat` tells the
    /// network this station was listening on `dial_hz` and heard silence, which
    /// is how a shut band is told apart from a receiver that was switched off.
    /// `tx_percent` rides along because the same request carries it.
    pub fn wspr_report(&self, spots: &[WsprSpot], dial_hz: f64, tx_percent: u8) {
        let Some(h) = &self.wspr_upload else { return };
        if spots.is_empty() {
            h.report(wsprnet::Item::Quiet { dial_hz, tx_percent });
            return;
        }
        for s in spots {
            // Only our own decodes get uploaded. A report that came back *from*
            // WSPRnet must never be posted to it again — that would credit us
            // with hearing something we did not.
            if s.reporter.is_some() {
                continue;
            }
            // Nor a callsign we could not resolve. A Type-3 message names its
            // sender by a hash this station cannot invert, and posting the
            // placeholder would put a station that does not exist into a
            // database everybody else reads.
            if s.call.starts_with("<#") {
                continue;
            }
            h.report(wsprnet::Item::Spot(s.clone()));
        }
    }

    /// Report a station we decoded (from a RADE End-of-Over callsign).
    pub fn reporter_rx_report(&self, call: String, snr: i32) {
        if !self.cfg.freedv_reporter.report_rx {
            return;
        }
        if let Some(h) = &self.freedv {
            h.rx_report(call, snr);
        }
    }

    /// Report that we are receiving *something* we have not identified yet —
    /// a RADE signal in sync with no End-of-Over callsign decoded. This is what
    /// makes a transmitting station see that it is being heard before either
    /// end knows the other's callsign.
    pub fn reporter_rx_presence(&self, snr: i32) {
        if !self.cfg.freedv_reporter.report_rx {
            return;
        }
        if let Some(h) = &self.freedv {
            h.rx_presence(snr);
        }
    }

    /// Update the operator's dial frequency, so band-scoped feeds query the
    /// right slice.
    pub fn set_dial(&self, hz: f64) {
        self.dial_bits.store(hz.to_bits(), Ordering::Relaxed);
    }

    /// Kick off a callsign lookup; the result arrives via [`SpotManager::poll`].
    pub fn lookup(&self, call: String) {
        let provider = self.cfg.lookup_provider;
        let qrz = self.cfg.qrz.clone();
        let hamqth = self.cfg.hamqth.clone();
        let tx = self.event_tx.clone();
        std::thread::Builder::new()
            .name("sdroxide-lookup".into())
            .spawn(move || match crate::lookup::lookup(provider, &qrz, &hamqth, &call) {
                Ok(info) => {
                    let _ = tx.send(NetEvent::Callsign(info));
                }
                Err(e) => {
                    let _ = tx.send(NetEvent::Status(Some(format!("Lookup {call}: {e}"))));
                }
            })
            .ok();
    }

    /// Upload one QSO's ADIF to the given targets; results arrive via `poll`.
    pub fn upload(&self, qso_id: u64, adif: String, targets: Vec<UploadTarget>) {
        let cfg = self.cfg.clone();
        let my_call = self.op_call.clone();
        let tx = self.event_tx.clone();
        std::thread::Builder::new()
            .name("sdroxide-upload".into())
            .spawn(move || {
                for target in targets {
                    let (ok, message) = match crate::upload::upload(&cfg, &my_call, target, &adif) {
                        Ok(m) => (true, m),
                        Err(e) => (false, e),
                    };
                    let _ = tx.send(NetEvent::Upload(UploadResult { qso_id, target, ok, message }));
                }
            })
            .ok();
    }

    /// Check one service's stored credentials; the result arrives via `poll`.
    ///
    /// On its own thread like every other network job, because these talk to
    /// servers that can take seconds to answer and the UI thread is drawing a
    /// waterfall. Read-only throughout: see [`crate::upload::test_login`].
    ///
    /// Tests the APPLIED config. The settings dialog therefore sends its edits
    /// with [`sdroxide_types::Command::SetNetworkConfig`] immediately before
    /// asking for a test, so what is checked is what is on screen: testing the
    /// applied config alone would answer about the old password whenever
    /// someone pasted a new one and pressed Test before Apply, which is exactly
    /// when they would press it.
    pub fn test_login(&self, target: sdroxide_types::LoginTarget) {
        let cfg = self.cfg.clone();
        let my_call = self.op_call.clone();
        let tx = self.event_tx.clone();
        std::thread::Builder::new()
            .name("sdroxide-login".into())
            .spawn(move || {
                let (ok, message) = match crate::upload::test_login(&cfg, &my_call, target) {
                    Ok(m) => (true, m),
                    Err(e) => (false, e),
                };
                let _ = tx.send(NetEvent::LoginTest(sdroxide_types::LoginTestResult {
                    target,
                    ok,
                    message,
                }));
            })
            .ok();
    }

    /// Download QSL confirmations; results arrive via `poll`.
    pub fn sync_confirmations(&self) {
        let cfg = self.cfg.clone();
        let tx = self.event_tx.clone();
        std::thread::Builder::new()
            .name("sdroxide-qsl".into())
            .spawn(move || {
                let _ = tx.send(NetEvent::Status(Some("Syncing confirmations…".into())));
                let (recs, errs) = crate::upload::sync_confirmations(&cfg);
                for e in errs {
                    let _ = tx.send(NetEvent::Status(Some(e)));
                }
                let n = recs.len();
                let _ = tx.send(NetEvent::Confirmations(recs));
                let _ = tx.send(NetEvent::Status(Some(format!("Confirmation sync: {n} records"))));
            })
            .ok();
    }

    /// Drain everything pending: feed updates (merged into a fresh spot
    /// snapshot when the set changed) plus worker results.
    pub fn poll(&mut self) -> Vec<NetEvent> {
        let mut got_feed = false;
        while let Ok((kind, spots)) = self.feed_rx.try_recv() {
            self.by_kind.insert(kind, spots);
            got_feed = true;
        }
        let mut out: Vec<NetEvent> = self.event_rx.try_iter().collect();
        // Recompute the snapshot when feeds changed (also catches age-outs on
        // the periodic polls, since feeds re-send their full set on each cycle).
        if got_feed || out.iter().any(|e| matches!(e, NetEvent::Status(_))) {
            let snap = self.snapshot();
            if snap != self.last_snapshot {
                self.last_snapshot = snap.clone();
                out.push(NetEvent::Spots(snap));
            }
        }
        out
    }

    /// Force a fresh snapshot emit on the next poll (e.g. after age-out).
    fn snapshot(&self) -> Vec<Spot> {
        let now = now_utc();
        let max_age = self.cfg.spot_max_age_secs.max(60) as i64;
        let mut v: Vec<Spot> = Vec::new();
        for spots in self.by_kind.values() {
            for s in spots {
                if now - s.when_utc > max_age {
                    continue;
                }
                let mut s = s.clone();
                if s.loc.is_none() {
                    if let Some(g) = &s.grid {
                        s.loc = grid_to_latlon(g);
                    }
                }
                v.push(s);
            }
        }
        v.sort_by(|a, b| a.freq_hz.total_cmp(&b.freq_hz));
        v
    }

    fn rebuild_cluster(&mut self) {
        if !self.station {
            return;
        }
        self.cluster = None; // drop stops the thread
        self.by_kind.remove(&SpotKind::DxCluster);
        if self.cfg.cluster.enabled && !self.cfg.cluster.host.trim().is_empty() {
            // The node's `login:` prompt takes the override if set, else the
            // operator callsign.
            let login = if self.cfg.cluster.login.trim().is_empty() {
                self.op_call.clone()
            } else {
                self.cfg.cluster.login.trim().to_string()
            };
            self.cluster = Some(ClusterHandle::connect(
                self.cfg.cluster.clone(),
                login,
                now_utc,
                self.feed_tx.clone(),
                self.event_tx.clone(),
            ));
        }
    }

    /// (Re)start the RBN reader.
    ///
    /// No `by_kind` entry to clear: RBN produces paths, and a path already
    /// folded into the propagation field is not something a feed owns and can
    /// take back. Switching RBN off stops new evidence arriving and lets what
    /// is there decay, which is what switching off a source should do.
    fn rebuild_rbn(&mut self) {
        if !self.station {
            return;
        }
        self.rbn = None; // drop stops the thread
        if !self.cfg.rbn.enabled || self.cfg.rbn.host.trim().is_empty() {
            return;
        }
        let login = if self.cfg.rbn.login.trim().is_empty() {
            self.op_call.trim().to_string()
        } else {
            self.cfg.rbn.login.trim().to_string()
        };
        // RBN wants a real callsign and will not send spots without one. Since
        // this feed is on by default, "no operator identity yet" is a state a
        // fresh install genuinely passes through — connecting anyway would open
        // a socket that could only ever sit at the prompt. `set_operator`
        // rebuilds when the callsign arrives.
        if login.is_empty() {
            return;
        }
        self.rbn =
            Some(RbnHandle::connect(self.cfg.rbn.clone(), login, now_utc, self.event_tx.clone()));
    }

    fn rebuild_pota(&mut self) {
        if !self.station {
            return;
        }
        self.pota = None;
        self.by_kind.remove(&SpotKind::Pota);
        if self.cfg.pota.enabled {
            let interval = Duration::from_secs(self.cfg.pota.interval_secs.max(15) as u64);
            self.pota = Some(poll::spawn(
                "sdroxide-pota",
                SpotKind::Pota,
                interval,
                self.feed_tx.clone(),
                self.event_tx.clone(),
                move || pota::fetch(now_utc()),
            ));
        }
    }

    fn rebuild_sota(&mut self) {
        if !self.station {
            return;
        }
        self.sota = None;
        self.by_kind.remove(&SpotKind::Sota);
        if self.cfg.sota.enabled {
            let interval = Duration::from_secs(self.cfg.sota.interval_secs.max(15) as u64);
            self.sota = Some(poll::spawn(
                "sdroxide-sota",
                SpotKind::Sota,
                interval,
                self.feed_tx.clone(),
                self.event_tx.clone(),
                move || sota::fetch(now_utc()),
            ));
        }
    }

    fn rebuild_psk(&mut self) {
        if !self.station {
            return;
        }
        self.psk = None;
        self.by_kind.remove(&SpotKind::PskReporter);
        if self.cfg.psk.enabled {
            let interval = Duration::from_secs(self.cfg.psk.interval_secs.max(60) as u64);
            let dial = Arc::clone(&self.dial_bits);
            self.psk = Some(poll::spawn(
                "sdroxide-pskreporter",
                SpotKind::PskReporter,
                interval,
                self.feed_tx.clone(),
                self.event_tx.clone(),
                move || pskreporter::fetch(f64::from_bits(dial.load(Ordering::Relaxed)), now_utc()),
            ));
        }
    }

    /// (Re)start the PSK Reporter upload worker. Reporting needs both halves of
    /// the operator identity: without a callsign there is no receiver to
    /// report, and without a grid the reports can't be placed on the map.
    fn rebuild_psk_upload(&mut self) {
        if !self.station {
            return;
        }
        self.psk_upload = None; // drop flushes what's pending and stops the thread
        if !self.cfg.psk.report || self.op_call.is_empty() || self.op_grid.is_empty() {
            return;
        }
        let station = crate::pskupload::Station {
            call: self.op_call.clone(),
            grid: self.op_grid.clone(),
            software: format!("sdroxide {}", env!("CARGO_PKG_VERSION")),
            antenna: self.cfg.psk.antenna.trim().to_string(),
        };
        self.psk_upload =
            Some(crate::pskupload::spawn(&self.cfg.psk, station, self.event_tx.clone(), now_utc));
    }

    /// (Re)start both halves of the WSPRnet conversation.
    ///
    /// Both need the whole operator identity: the callsign is the account and
    /// the grid is where the report is placed. Without either there is nothing
    /// to say and nobody to say it as, so neither half starts — silently, since
    /// a station that has not filled in its callsign yet is not an error.
    fn rebuild_wspr(&mut self) {
        if !self.station {
            return;
        }
        // Dropping the uploader flushes what is pending first.
        self.wspr_upload = None;
        self.wspr_heard_us = None;
        if self.op_call.is_empty() || self.op_grid.is_empty() {
            return;
        }
        if self.cfg.wspr.upload {
            let rx = wsprnet::Reporter { call: self.op_call.clone(), grid: self.op_grid.clone() };
            self.wspr_upload = Some(wsprnet::spawn_upload(rx, self.event_tx.clone()));
        }
        if self.cfg.wspr.download_heard_us {
            self.wspr_heard_us = Some(wsprnet::spawn_download(
                self.op_call.clone(),
                wsprnet::Query::HeardUs,
                Duration::from_secs(self.cfg.wspr.download_interval_secs.max(60) as u64),
                self.cfg.wspr.download_window_min.max(2),
                self.event_tx.clone(),
            ));
        }
    }

    fn rebuild_freedv(&mut self) {
        if !self.station {
            return;
        }
        self.freedv = None; // drop stops the thread and closes the session
        self.by_kind.remove(&SpotKind::FreeDv);
        if !self.cfg.freedv_reporter.enabled {
            return;
        }
        let h = ReporterHandle::connect(
            self.cfg.freedv_reporter.clone(),
            self.op_call.clone(),
            self.op_grid.clone(),
            self.feed_tx.clone(),
            self.event_tx.clone(),
        );
        // Replay what the engine last told us, so the new session starts with
        // the current picture instead of waiting for the next change.
        h.set_freq(self.rep_freq);
        h.set_tx(self.rep_tx);
        h.set_visible(self.rep_visible);
        self.freedv = Some(h);
    }
}

impl Default for SpotManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RBN is the one feed here that is on out of the box, so "the operator has
    /// not said who they are yet" is a state a fresh install really passes
    /// through — and RBN will not send a spot to a session that never gave a
    /// callsign. Connecting anyway would leave a socket sitting at the prompt.
    #[test]
    fn rbn_waits_for_a_callsign_before_connecting() {
        let mut m = SpotManager::new();
        assert!(m.cfg.rbn.enabled, "RBN is meant to be on by default");

        m.set_config(NetworkConfig::default());
        assert!(m.rbn.is_none(), "connected to RBN with no callsign to log in as");

        // The identity arrives — from the digi config, on a later tick.
        m.set_operator("OE1TEST", "JN88");
        assert!(m.rbn.is_some(), "the callsign arrived and RBN did not start");
    }

    /// The override is what an operator uses when their RBN login differs from
    /// the callsign they are operating as.
    #[test]
    fn an_explicit_login_does_not_need_the_operator_identity() {
        let mut m = SpotManager::new();
        let mut cfg = NetworkConfig::default();
        cfg.rbn.login = "  OE1TEST  ".into();
        m.set_config(cfg);
        assert!(m.rbn.is_some(), "an explicit login should have been enough");
    }

    /// Switching it off has to stop the thread, not merely stop reading it.
    #[test]
    fn disabling_rbn_drops_the_connection() {
        let mut m = SpotManager::new();
        m.set_operator("OE1TEST", "JN88");
        m.set_config(NetworkConfig::default());
        assert!(m.rbn.is_some());

        let mut off = NetworkConfig::default();
        off.rbn.enabled = false;
        m.set_config(off);
        assert!(m.rbn.is_none(), "the reader outlived being switched off");
    }

    /// A station has one DX cluster login and one FreeDV Reporter session
    /// however many radios it has — but every radio's manager still needs the
    /// credentials, because a callsign lookup or a logbook upload is per-request
    /// work whichever radio asks for it. Settings applied from the second
    /// radio's window used to open a duplicate of every feed.
    #[test]
    fn a_stood_down_manager_takes_the_config_but_opens_no_sockets() {
        let mut m = SpotManager::new();
        m.stand_down();
        m.set_operator("OE1TEST", "JN88");

        let mut cfg = NetworkConfig::default();
        cfg.cluster.enabled = true;
        cfg.cluster.host = "cluster.example".into();
        cfg.freedv_reporter.enabled = true;
        cfg.qrz.user = "OE1TEST".into();
        m.set_config(cfg);

        assert!(m.rbn.is_none(), "a second radio must not open its own RBN reader");
        assert!(m.cluster.is_none(), "nor a second DX cluster login");
        assert!(m.freedv.is_none(), "nor a second FreeDV Reporter session");
        assert_eq!(m.cfg.qrz.user, "OE1TEST", "but the credentials must still have landed");
        assert!(m.cfg.cluster.enabled, "and the config is what a settings window reads back");
    }
}
