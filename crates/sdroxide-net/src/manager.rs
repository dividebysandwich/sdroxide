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
    Band, BandOpening, BandOpeningTracker, BandPath, NetworkConfig, RbnConfig, Spot, SpotKind,
    UploadResult, UploadTarget, WsprSpot, grid_to_latlon, resolve_callsign,
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
    /// Rolling RBN spot set, synthesized from the PropPaths RBN emits, so
    /// the skimmer spots can be drawn on the waterfall. Age-pruned.
    rbn_spots: Vec<Spot>,
    /// Band-opening analysis over the feeds' raw spots (the 3-hour baseline is
    /// longer than the display age, so this ingests the un-pruned sets), plus
    /// any evidence pushed in by [`SpotManager::feed_openings`].
    band_openings: BandOpeningTracker,
    last_band_openings: Vec<BandOpening>,
    /// The band the opening tracker is currently warmed for. `None` until the
    /// first poll, so the first one never restarts a tracker that is empty.
    openings_band: Option<Band>,
    /// Set when evidence arrived outside a feed cycle, so the next poll runs
    /// the analysis even when no feed spoke.
    feed_dirty: bool,
    /// Callsign → continent memo, so the per-poll continent lookup over a few
    /// thousand spots stays cheap.
    continent_memo: HashMap<String, Option<&'static str>>,
    /// Current dial frequency (Hz) as bits, shared with the PSK feed.
    dial_bits: Arc<AtomicU64>,

    cluster: Option<ClusterHandle>,
    /// The Reverse Beacon Network reader. Emits paths, not spots, so unlike
    /// every other feed here it has no entry in `by_kind`.
    rbn: Option<RbnHandle>,
    /// The second skimmer connection, beside `rbn`.
    rbn2: Option<RbnHandle>,
    pota: Option<PollHandle>,
    sota: Option<PollHandle>,
    psk: Option<PollHandle>,
    /// The "who heard me" poll: reports where *we* are the sender, drawn as
    /// reporter rings. Separate from `psk` above, which is the band-activity
    /// feed, and polled hourly rather than every few minutes.
    psk_heard_me: Option<PollHandle>,
    /// The operator's callsign and grid, pushed in from the digi config by the
    /// engine. Not part of [`NetworkConfig`]: there is one operator identity in
    /// the app and it is set on the General tab. The callsign is the *station*
    /// identity; a listener's reception-report identity is the separate
    /// [`NetworkConfig::swl_id`], which overrides it only for reporting.
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
    /// WSJT-CB spot server uploader (decoded 11 m spots), when its opt-in is on
    /// and the operator identity is known.
    wsjtcb: Option<crate::wsjtcb::WsjtCbHandle>,

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
            rbn_spots: Vec::new(),
            band_openings: BandOpeningTracker::new(),
            last_band_openings: Vec::new(),
            openings_band: None,
            feed_dirty: false,
            continent_memo: HashMap::new(),
            dial_bits: Arc::new(AtomicU64::new(14_074_000f64.to_bits())),
            cluster: None,
            rbn: None,
            rbn2: None,
            pota: None,
            sota: None,
            psk: None,
            psk_heard_me: None,
            psk_upload: None,
            wspr_upload: None,
            wspr_heard_us: None,
            wsjtcb: None,
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
        if old.rbn2 != self.cfg.rbn2 {
            self.rebuild_rbn2();
        }
        if old.pota != self.cfg.pota {
            self.rebuild_pota();
        }
        if old.sota != self.cfg.sota {
            self.rebuild_sota();
        }
        if old.psk != self.cfg.psk {
            self.rebuild_psk();
            self.rebuild_psk_heard_me();
        }
        // A changed SWL identity restarts the uploads but not the fetch feed:
        // the reporter is who the decodes are credited to, the fetch feed is
        // not identity-bearing.
        if old.psk != self.cfg.psk || old.swl_id != self.cfg.swl_id {
            self.rebuild_psk_upload();
        }
        if old.wspr != self.cfg.wspr || old.swl_id != self.cfg.swl_id {
            self.rebuild_wspr();
        }
        if old.wsjtcb != self.cfg.wsjtcb {
            self.rebuild_wsjtcb();
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
        if call_changed && self.cfg.rbn2.login.trim().is_empty() {
            self.rebuild_rbn2();
        }
        self.rebuild_freedv();
        // The callsign and grid (or the SWL identity, if one is set) *are* the
        // PSK Reporter receiver record.
        self.rebuild_psk_upload();
        // The callsign is also the key the "who heard me" query asks about.
        self.rebuild_psk_heard_me();
        // And they are the whole of WSPRnet's report identity: the callsign in
        // the query is the account.
        self.rebuild_wspr();
        // The WSJT-CB server names the spotter by callsign and grid too.
        self.rebuild_wsjtcb();
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

    /// Report a decoded 11 m spot to the WSJT-CB spot server.
    ///
    /// Queued and posted by the worker; a no-op when the opt-in is off or we
    /// have no callsign to spot as.
    pub fn wsjtcb_report(&self, spot: crate::CbSpot) {
        if let Some(h) = &self.wsjtcb {
            h.report(spot);
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
        // **The band the dial is on, and whether it changed.** A feed is only
        // polled for the current band, so every path on the band we have just
        // left has had no observation for as long as we were elsewhere. The
        // band-opening tracker is told, so its warm-up clock is not trusted
        // across a gap it never saw — which is what produced an **infinite**
        // opening on the first spots back (upstream #537).
        let dial_hz = f64::from_bits(self.dial_bits.load(Ordering::Relaxed));
        let band_now = Band::containing(dial_hz);
        if let Some(previous) = self.openings_band.replace(band_now)
            && previous != band_now
        {
            self.band_openings.restart();
        }
        let mut got_feed = false;
        while let Ok((kind, spots)) = self.feed_rx.try_recv() {
            self.by_kind.insert(kind, spots);
            got_feed = true;
        }
    let mut out: Vec<NetEvent> = self.event_rx.try_iter().collect();
    // Synthesize waterfall spots from RBN propagation paths, so RBN 1/2
    // labels can be drawn on the panadapter (not only on the propagation map).
    let mut rbn_changed = false;
    for ev in &out {
        if let NetEvent::PropPaths(paths) = ev {
            for p in paths {
                let call = p.key.split('>').nth(1).unwrap_or(&p.key).trim().to_string();
                if call.is_empty() {
                    continue;
                }
                let spot = Spot {
                    id: Spot::make_id(SpotKind::Rbn, &call, p.freq_hz),
                    kind: SpotKind::Rbn,
                    freq_hz: p.freq_hz,
                    call,
                    spotter: p.key.split('>').next().unwrap_or("").trim().to_string(),
                    mode: String::new(),
                    comment: p.snr_db.map(|s| format!("{s:.0} dB")).unwrap_or_default(),
                    reference: None,
                    grid: None,
                    loc: None,
                    when_utc: p.at_utc,
                    snr_db: p.snr_db.map(|s| s.round() as i16),
                };
                if let Some(existing) = self.rbn_spots.iter_mut().find(|s| s.id == spot.id) {
                    *existing = spot;
                } else {
                    self.rbn_spots.push(spot);
                }
                rbn_changed = true;
            }
        }
    }
    let now = now_utc();
    let before = self.rbn_spots.len();
    self.rbn_spots.retain(|s| now - s.when_utc < 600);
    if self.rbn_spots.len() != before {
        rbn_changed = true;
    }
    if self.rbn_spots.len() > 200 {
        let drop = self.rbn_spots.len() - 200;
        self.rbn_spots.drain(..drop);
        rbn_changed = true;
    }
    if rbn_changed {
        if self.rbn_spots.is_empty() {
            self.by_kind.remove(&SpotKind::Rbn);
        } else {
            self.by_kind.insert(SpotKind::Rbn, self.rbn_spots.clone());
        }
        self.feed_dirty = true;
    }
    if got_feed || self.feed_dirty || out.iter().any(|e| matches!(e, NetEvent::Status(_))) {

            self.feed_dirty = false;
            let now = now_utc();
            // The band-opening analysis ingests the feeds' own sets — not the
            // age-pruned snapshot — because its 3-hour baseline outlives the
            // display age. Ids are stable, so re-sending the full set each
            // cycle is a no-op for everything already seen.
            let mut paths: Vec<BandPath> = Vec::new();
            let kinds: Vec<Vec<Spot>> = self.by_kind.values().cloned().collect();
            for spots in &kinds {
                for s in spots {
                    let band = Band::containing(s.freq_hz);
                    if !band.is_amateur() || s.call.is_empty() || s.spotter.is_empty() {
                        continue;
                    }
                    let from = self.continent(&s.call);
                    let to = self.continent(&s.spotter);
                    let (Some(from), Some(to)) = (from, to) else { continue };
                    paths.push(BandPath {
                        call: s.call.clone(),
                        band,
                        from_continent: from,
                        to_continent: to,
                        timestamp: s.when_utc,
                        id: Some(s.id.to_string()),
                    });
                }
            }
            self.band_openings.ingest(&paths, now);
            let openings = self.band_openings.analyze(now);
            if openings != self.last_band_openings {
                self.last_band_openings = openings.clone();
                out.push(NetEvent::BandOpenings(openings));
            }
            // Recompute the snapshot when feeds changed (also catches age-outs on
            // the periodic polls, since feeds re-send their full set on each cycle).
            let snap = self.snapshot();
            if snap != self.last_snapshot {
                self.last_snapshot = snap.clone();
                out.push(NetEvent::Spots(snap));
            }
        }
        out
    }

    /// Continent code for a callsign, memoised.
    fn continent(&mut self, call: &str) -> Option<&'static str> {
        if let Some(c) = self.continent_memo.get(call) {
            return *c;
        }
        let c = resolve_callsign(call).map(|i| i.continent);
        self.continent_memo.insert(call.to_string(), c);
        c
    }

    /// Push path evidence that did not come from a spot feed — today the 11 m
    /// WSJT-CB decodes, which reach the engine directly rather than through
    /// [`SpotManager::set_config`]. The evidence is ingested and the analysis
    /// runs on the next [`SpotManager::poll`].
    pub fn feed_openings(&mut self, paths: Vec<BandPath>, now: i64) {
        self.band_openings.ingest(&paths, now);
        self.feed_dirty = true;
    }

    /// Mark a band's path as watched since `since`, for a feed that is up but
    /// has not produced spots. The warm-up clock is watched time, not "oldest
    /// spot", so a caller that knows a band's feed has been running says so.
    pub fn watch_openings(
        &mut self,
        band: Band,
        from_continent: &'static str,
        to_continent: &'static str,
        since: i64,
    ) {
        self.band_openings.watch(band, from_continent, to_continent, since);
    }

    /// Force a fresh snapshot emit on the next poll (e.g. after age-out).
    fn snapshot(&self) -> Vec<Spot> {
        let now = now_utc();
        let max_age = self.cfg.spot_max_age_secs.max(60) as i64;
        let mut v: Vec<Spot> = Vec::new();
        for spots in self.by_kind.values() {
            for s in spots {
                // HeardMe is replaced wholesale on its own hourly poll and
                // describes the last hour, so the spot max-age (minutes) does
                // not apply: aging it out would blank the overlay for the last
                // three quarters of every hour.
                if s.kind != SpotKind::HeardMe && now - s.when_utc > max_age {
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
        self.rbn = None; // drop stops the thread, before a new login opens
        self.rbn = self.connect_rbn(&self.cfg.rbn);
    }

    /// (Re)start the second RBN reader. Same rules as [`Self::rebuild_rbn`].
    fn rebuild_rbn2(&mut self) {
        if !self.station {
            return;
        }
        self.rbn2 = None;
        self.rbn2 = self.connect_rbn(&self.cfg.rbn2);
    }

    /// Open one RBN connection for `cfg`, or none when it is off, has no host,
    /// or there is no callsign to log in as.
    fn connect_rbn(&self, cfg: &RbnConfig) -> Option<RbnHandle> {
        if !cfg.enabled || cfg.host.trim().is_empty() {
            return None;
        }
        let login = if cfg.login.trim().is_empty() {
            self.op_call.trim().to_string()
        } else {
            cfg.login.trim().to_string()
        };
        // RBN wants a real callsign and will not send spots without one. Since
        // the first feed is on by default, "no operator identity yet" is a
        // state a fresh install genuinely passes through — connecting anyway
        // would open a socket that could only ever sit at the prompt.
        // `set_operator` rebuilds when the callsign arrives.
        if login.is_empty() {
            return None;
        }
        Some(RbnHandle::connect(cfg.clone(), login, now_utc, self.event_tx.clone()))
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

    /// (Re)start the "who heard me" poll: reception reports where *this*
    /// station is the sender, on the current band, over the last hour. It runs
    /// whenever the PSK feed is on and a callsign is known, and on its own
    /// hour-long interval — it is a picture of the last hour, not of the last
    /// slot, and the client decides whether to draw it.
    ///
    /// Needs a callsign to ask about: a listener with only an SWL identity has
    /// nothing to look up and gets no poll.
    fn rebuild_psk_heard_me(&mut self) {
        if !self.station {
            return;
        }
        self.psk_heard_me = None;
        self.by_kind.remove(&SpotKind::HeardMe);
        let call = self.op_call.trim();
        if self.cfg.psk.enabled && !call.is_empty() {
            let dial = Arc::clone(&self.dial_bits);
            let call = call.to_string();
            self.psk_heard_me = Some(poll::spawn(
                "sdroxide-pskreporter-heardme",
                SpotKind::HeardMe,
                Duration::from_secs(3600),
                self.feed_tx.clone(),
                self.event_tx.clone(),
                move || {
                    pskreporter::fetch_heard_me(
                        f64::from_bits(dial.load(Ordering::Relaxed)),
                        &call,
                        now_utc(),
                    )
                },
            ));
        }
    }

    /// The identity to report *receptions* under: the listener's SWL number
    /// when one is set, else the operator's callsign.
    ///
    /// Only the reporting paths use this. Every path that asserts a station on
    /// the air or in a logbook — keying, QSO uploads, the DX cluster and RBN
    /// logins, WSJT-CB — keeps `op_call`, so an SWL number is never
    /// transmitted and never names a spotter.
    fn report_call(&self) -> &str {
        let swl = self.cfg.swl_id.trim();
        if swl.is_empty() { self.op_call.trim() } else { swl }
    }

    /// (Re)start the PSK Reporter upload worker. Reporting needs a reporter
    /// identity and a grid: without the former there is no receiver to report,
    /// and without the latter the reports can't be placed on the map. The
    /// reporter may be the SWL number alone — a listener with no callsign is
    /// exactly who reception reporting is for.
    fn rebuild_psk_upload(&mut self) {
        if !self.station {
            return;
        }
        self.psk_upload = None; // drop flushes what's pending and stops the thread
        let call = self.report_call().to_string();
        if !self.cfg.psk.report || call.is_empty() || self.op_grid.is_empty() {
            return;
        }
        let station = crate::pskupload::Station {
            call,
            grid: self.op_grid.clone(),
            software: format!("sdroxide {}", env!("CARGO_PKG_VERSION")),
            antenna: self.cfg.psk.antenna.trim().to_string(),
        };
        self.psk_upload =
            Some(crate::pskupload::spawn(&self.cfg.psk, station, self.event_tx.clone(), now_utc));
    }

    /// (Re)start both halves of the WSPRnet conversation.
    ///
    /// Uploading needs a reporter identity and a grid: the reporter is the
    /// account and the grid is where the report is placed. The reporter may be
    /// the SWL number alone, so a receive-only listener can report. Without
    /// either there is nothing to say and nobody to say it as, so nothing
    /// starts — silently, since a station that has not filled in its identity
    /// yet is not an error.
    ///
    /// "Who heard us" is different: it asks the network about a *transmitted*
    /// callsign, and an SWL number is never on the air to be heard, so that
    /// half still needs the operator's callsign.
    fn rebuild_wspr(&mut self) {
        if !self.station {
            return;
        }
        // Dropping the uploader flushes what is pending first.
        self.wspr_upload = None;
        self.wspr_heard_us = None;
        let call = self.report_call().to_string();
        if call.is_empty() || self.op_grid.is_empty() {
            return;
        }
        if self.cfg.wspr.upload {
            let rx = wsprnet::Reporter { call, grid: self.op_grid.clone() };
            self.wspr_upload = Some(wsprnet::spawn_upload(rx, self.event_tx.clone()));
        }
        if self.cfg.wspr.download_heard_us && !self.op_call.is_empty() {
            self.wspr_heard_us = Some(wsprnet::spawn_download(
                self.op_call.clone(),
                wsprnet::Query::HeardUs,
                Duration::from_secs(self.cfg.wspr.download_interval_secs.max(60) as u64),
                self.cfg.wspr.download_window_min.max(2),
                self.event_tx.clone(),
            ));
        }
    }

    /// (Re)start the WSJT-CB spot reporter.
    ///
    /// Needs the operator's callsign: it is the `spotter_call` the server
    /// shows, and without one there is nobody to spot as — silently, since a
    /// station that has not filled in its callsign yet is not an error. The
    /// grid is optional and sent when there is one.
    fn rebuild_wsjtcb(&mut self) {
        if !self.station {
            return;
        }
        self.wsjtcb = None;
        if !self.cfg.wsjtcb.report || self.op_call.is_empty() {
            return;
        }
        let station =
            crate::wsjtcb::Station { call: self.op_call.clone(), grid: self.op_grid.clone() };
        self.wsjtcb =
            Some(crate::wsjtcb::spawn(self.cfg.wsjtcb.url.clone(), station, self.event_tx.clone()));
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

    /// The second RBN connection is off out of the box and is its own reader:
    /// switching it on opens it without touching the first, and off drops it.
    #[test]
    fn the_second_rbn_connection_is_independent_and_off_by_default() {
        let mut m = SpotManager::new();
        m.set_operator("OE1TEST", "JN88");
        m.set_config(NetworkConfig::default());
        assert!(m.rbn.is_some());
        assert!(m.rbn2.is_none(), "the second RBN connection is meant to be off by default");
        assert_eq!(m.cfg.rbn2.port, 7001);

        let mut on = NetworkConfig::default();
        on.rbn2.enabled = true;
        m.set_config(on);
        assert!(m.rbn.is_some() && m.rbn2.is_some());

        let mut first_off = NetworkConfig::default();
        first_off.rbn.enabled = false;
        first_off.rbn2.enabled = true;
        m.set_config(first_off);
        assert!(m.rbn.is_none() && m.rbn2.is_some(), "the two must not drag each other");
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
        assert!(m.rbn2.is_none(), "nor its own second RBN reader");
        assert!(m.cluster.is_none(), "nor a second DX cluster login");
        assert!(m.freedv.is_none(), "nor a second FreeDV Reporter session");
        assert_eq!(m.cfg.qrz.user, "OE1TEST", "but the credentials must still have landed");
        assert!(m.cfg.cluster.enabled, "and the config is what a settings window reads back");
    }

    /// The SWL number takes over as the *reporter* identity, and only that:
    /// the callsign is still what the cluster, RBN, the logbook and WSJT-CB
    /// use, because those paths never consult `report_call`.
    #[test]
    fn the_swl_number_is_the_report_identity_when_set() {
        let mut m = SpotManager::new();
        m.set_operator("19DCG373", "JO22");
        assert_eq!(m.report_call(), "19DCG373", "no number: report as the callsign");

        let mut cfg = NetworkConfig::default();
        cfg.swl_id = "  19SWL001  ".into();
        m.set_config(cfg);
        assert_eq!(m.report_call(), "19SWL001", "the number wins, trimmed");
        assert_eq!(m.op_call, "19DCG373", "and the station identity is untouched");

        m.set_config(NetworkConfig::default());
        assert_eq!(m.report_call(), "19DCG373", "cleared again: back to the callsign");
    }

    /// A receive-only listener has no callsign at all; the number alone is
    /// enough to be a receiver in the reporting networks.
    #[test]
    fn a_listener_with_no_callsign_still_has_a_report_identity() {
        let mut m = SpotManager::new();
        m.set_operator("", "JO22");
        assert_eq!(m.report_call(), "", "no callsign and no number: nobody to report as");

        let mut cfg = NetworkConfig::default();
        cfg.swl_id = "19SWL001".into();
        m.set_config(cfg);
        assert_eq!(m.report_call(), "19SWL001");
    }

    /// 11 m WSJT-CB decodes reach the detector directly — there is no cluster
    /// or PSK-Reporter feed on the citizens' band — and a burst of them must
    /// surface on the very next poll as an `M11` opening. The warm-up is per
    /// path, so the test gives the path a quiet baseline first: a burst with no
    /// history is the band being open, not a *change* on it.
    #[test]
    fn fed_eleven_metre_paths_surge_the_detector_on_the_next_poll() {
        let mut m = SpotManager::new();
        let now = now_utc();
        let baseline: Vec<BandPath> = (0..6)
            .map(|i| BandPath {
                call: format!("G{}ABCD", i % 10),
                band: Band::M11,
                from_continent: "AS",
                to_continent: "EU",
                timestamp: now - 1000 - i * 500,
                id: Some(format!("11m-base-{i}")),
            })
            .collect();
        // The baseline is ingested at the time it was heard, so the path has
        // actually been watched for that span — the warm-up clock is watched
        // time, not "oldest spot in one batch", and a single ingest is not a
        // watched span.
        // The app has been watching 11 m; establish that, then ingest at now.
        m.watch_openings(Band::M11, "AS", "EU", now - 3600);
        m.feed_openings(baseline, now);
        let burst: Vec<BandPath> = (0..8)
            .map(|i| BandPath {
                call: format!("JA{}ABCD", i % 10),
                band: Band::M11,
                from_continent: "AS",
                to_continent: "EU",
                timestamp: now - 60 - (i % 10) * 60,
                id: Some(format!("11m|JA{}ABCD|{i}", i % 10)),
            })
            .collect();
        m.feed_openings(burst, now);
        let openings = m.poll().into_iter().find_map(|e| match e {
            NetEvent::BandOpenings(o) => Some(o),
            _ => None,
        });
        let openings = openings.expect("fed evidence must be analysed on the next poll");
        assert_eq!(openings.len(), 1);
        assert_eq!(openings[0].band, Band::M11);
        assert_eq!(openings[0].to_continent, "EU");
        assert_eq!(openings[0].state, sdroxide_types::OpeningState::Opening);
    }
}
