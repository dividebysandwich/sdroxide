//! The network cockpit's outbound half: callsign lookup and QSO upload.
//!
//! Lookups and uploads are queued here and drained into [`Command`]s by the
//! frame loop, so nothing in the UI ever blocks on the network. What comes
//! back is applied to the in-progress QSO and appended to the rolling log the
//! SPOTS window shows.

use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

use sdroxide_types::{
    CallsignInfo, Command, LookupProvider, NetworkConfig, QsoRecord, UploadResult, UploadTarget,
};

use crate::app::SdroxideApp;
use crate::app::persist::persist_qso_log;

/// Least time between two locator lookups driven by a heard list.
const GRID_LOOKUP_INTERVAL_S: f64 = 1.5;

/// Fill `dst` from `src` only when `dst` is blank and `src` is non-empty;
/// returns whether it changed anything.
fn fill_blank(dst: &mut String, src: Option<&str>) -> bool {
    if dst.trim().is_empty() {
        if let Some(v) = src.map(str::trim) {
            if !v.is_empty() {
                *dst = v.to_string();
                return true;
            }
        }
    }
    false
}

/// Which upload targets are enabled for auto-upload in `cfg`.
fn auto_upload_targets(cfg: &NetworkConfig) -> Vec<UploadTarget> {
    let mut t = Vec::new();
    if cfg.auto_upload_eqsl {
        t.push(UploadTarget::Eqsl);
    }
    if cfg.auto_upload_qrz {
        t.push(UploadTarget::QrzLogbook);
    }
    if cfg.auto_upload_hamqth {
        t.push(UploadTarget::HamQth);
    }
    if cfg.auto_upload_clublog {
        t.push(UploadTarget::ClubLog);
    }
    if cfg.auto_upload_wrl {
        t.push(UploadTarget::Wrl);
    }
    if cfg.wavelog.auto_upload && cfg.wavelog.is_configured() {
        t.push(UploadTarget::Wavelog);
    }
    t
}

/// Upload targets that have credentials configured (for the manual per-QSO
/// upload button).
pub(in crate::app) fn configured_upload_targets(cfg: &NetworkConfig) -> Vec<UploadTarget> {
    let mut t = Vec::new();
    if !cfg.eqsl.user.trim().is_empty() {
        t.push(UploadTarget::Eqsl);
    }
    if !cfg.qrz_logbook_key.trim().is_empty() {
        t.push(UploadTarget::QrzLogbook);
    }
    // The same account as the callsign lookup — a HamQTH user who filled in the
    // lookup half has already configured the upload half.
    if !cfg.hamqth.user.trim().is_empty() && !cfg.hamqth.password.trim().is_empty() {
        t.push(UploadTarget::HamQth);
    }
    if !cfg.clublog.user.trim().is_empty() && !cfg.clublog_api_key.trim().is_empty() {
        t.push(UploadTarget::ClubLog);
    }
    if !cfg.wrl_api_key.trim().is_empty() {
        t.push(UploadTarget::Wrl);
    }
    if cfg.wavelog.is_configured() {
        t.push(UploadTarget::Wavelog);
    }
    t
}

/// Single-record ADIF + targets for auto-upload of a freshly logged QSO, or
/// `None` when auto-upload is off or no target is enabled.
pub(in crate::app) fn auto_upload_adif(
    cfg: &NetworkConfig,
    rec: &QsoRecord,
) -> Option<(u64, String, Vec<UploadTarget>)> {
    if !cfg.auto_upload {
        return None;
    }
    let targets = auto_upload_targets(cfg);
    if targets.is_empty() {
        return None;
    }
    Some((rec.id, sdroxide_types::qso_log_to_adif(std::slice::from_ref(rec)), targets))
}

impl SdroxideApp {
    /// Queue an auto-lookup if a provider + auto-lookup are configured.
    ///
    /// Returns whether one was actually queued, so a caller that rations
    /// lookups can tell "asked" from "lookup is switched off" and not burn its
    /// one-per-interval budget on a request that never left.
    pub(in crate::app) fn queue_lookup(&mut self, call: String) -> bool {
        let call = call.trim().to_string();
        if call.is_empty()
            || !self.net_cfg_edit.auto_lookup
            || self.net_cfg_edit.lookup_provider == LookupProvider::None
        {
            return false;
        }
        self.pending_lookups.push(call);
        true
    }

    /// Whether another heard-list locator lookup may go out yet.
    ///
    /// Rationed across every mode that places stations by callsign rather than
    /// per panel: they share one cache and one network, and a busy band would
    /// otherwise put fifty requests on fifty threads the moment a panel opens.
    pub(in crate::app) fn grid_lookup_due(&self, now_t: f64) -> bool {
        now_t - self.grid_lookup_at >= GRID_LOOKUP_INTERVAL_S
    }

    /// Ask where a heard station is, and remember having asked.
    pub(in crate::app) fn queue_grid_lookup(&mut self, call: Option<String>, now_t: f64) {
        let Some(call) = call else { return };
        // Only spend the interval on a request that actually left: with no
        // provider configured this must stay ready for the moment one is.
        if self.queue_lookup(call.clone()) {
            self.grid_looked_up.insert(call);
            self.grid_lookup_at = now_t;
        }
    }

    /// Merge a callsign-lookup result into the open log entry and, if none
    /// matches, into the most recent logged QSO for that call — filling only
    /// blank fields.
    pub(in crate::app) fn apply_callsign(&mut self, info: CallsignInfo) {
        let mut summary = info.call.clone();
        if let Some(n) = &info.name {
            summary.push_str(&format!(" — {n}"));
        }
        if let Some(q) = &info.qth {
            summary.push_str(&format!(", {q}"));
        }
        if let Some(c) = &info.country {
            summary.push_str(&format!(" ({c})"));
        }
        self.push_net_log(summary);
        // Keep the whole record: the JS8 map places stations by the grid in it,
        // and that station may never have a log entry to merge into.
        self.callsign_cache.insert(info.call.to_ascii_uppercase(), info.clone());

        // 1) The open entry form, if it's for this call.
        if let Some(f) = self.log_edit.as_mut() {
            if f.call.trim().eq_ignore_ascii_case(&info.call) {
                fill_blank(&mut f.name, info.name.as_deref());
                fill_blank(&mut f.qth, info.qth.as_deref());
                fill_blank(&mut f.grid, info.grid.as_deref());
                fill_blank(&mut f.state, info.state.as_deref());
                fill_blank(&mut f.country, info.country.as_deref());
                return;
            }
        }
        // 2) Otherwise, enrich the most recent logged QSO with that call.
        if let Some(rec) = self
            .qso_log
            .iter_mut()
            .filter(|q| q.call.eq_ignore_ascii_case(&info.call))
            .max_by_key(|q| q.start_utc)
        {
            let mut changed = false;
            changed |= fill_blank(&mut rec.name, info.name.as_deref());
            changed |= fill_blank(&mut rec.qth, info.qth.as_deref());
            changed |= fill_blank(&mut rec.state, info.state.as_deref());
            changed |= fill_blank(&mut rec.country, info.country.as_deref());
            if rec.grid.as_deref().unwrap_or("").is_empty() {
                if let Some(g) = &info.grid {
                    rec.grid = Some(g.clone());
                    changed = true;
                }
            }
            if rec.dxcc.is_none() && info.dxcc.is_some() {
                rec.dxcc = info.dxcc;
                changed = true;
            }
            if rec.cq_zone.is_none() && info.cq_zone.is_some() {
                rec.cq_zone = info.cq_zone;
                changed = true;
            }
            if rec.itu_zone.is_none() && info.itu_zone.is_some() {
                rec.itu_zone = info.itu_zone;
                changed = true;
            }
            if changed {
                persist_qso_log(&self.qso_log);
                self.log_content_changed();
            }
        }
    }

    /// Record an upload result and mark the QSO's sent flag on success.
    pub(in crate::app) fn on_upload_result(&mut self, r: UploadResult) {
        let status = if r.ok { "OK" } else { "FAIL" };
        let retry = if r.retryable { " (will retry)" } else { "" };
        self.push_net_log(format!("{} → {}: {}{retry}", r.target.label(), status, r.message));
        if r.target == UploadTarget::Wavelog && !r.ok && !r.retryable {
            self.wavelog_refused.insert(r.qso_id);
        }
        if r.ok {
            if let Some(rec) = self.qso_log.iter_mut().find(|q| q.id == r.qso_id) {
                match r.target {
                    UploadTarget::Eqsl => rec.eqsl_sent = true,
                    UploadTarget::QrzLogbook => rec.qrz_sent = true,
                    UploadTarget::ClubLog => rec.clublog_sent = true,
                    UploadTarget::HamQth => rec.hamqth_sent = true,
                    UploadTarget::Wrl => rec.wrl_sent = true,
                    UploadTarget::Wavelog => rec.wavelog_sent = true,
                }
                persist_qso_log(&self.qso_log);
            }
        }
    }

    /// The QSOs owed to Wavelog: logged since auto-upload to it was switched
    /// on, and not yet acknowledged.
    ///
    /// Read off the logbook rather than kept as a queue, so a QSO logged while
    /// the server was down is still owed after a restart — the logbook is
    /// already saved, and a second record of what is pending could only
    /// disagree with it.
    pub(in crate::app) fn wavelog_owed(&self) -> impl Iterator<Item = &QsoRecord> {
        let cfg = &self.net_cfg_edit;
        let on = cfg.auto_upload && cfg.wavelog.auto_upload && cfg.wavelog.is_configured();
        let since = cfg.wavelog.auto_upload_since;
        self.qso_log.iter().filter(move |q| {
            on && !q.wavelog_sent && q.start_utc >= since && !self.wavelog_refused.contains(&q.id)
        })
    }

    /// Every QSO not in Wavelog, for the operator's "send them all" button —
    /// the backlog from before auto-upload, and anything refused this session
    /// that they may since have fixed.
    pub(in crate::app) fn wavelog_unsent(&self) -> impl Iterator<Item = &QsoRecord> {
        self.qso_log.iter().filter(|q| !q.wavelog_sent && !q.call.trim().is_empty())
    }

    /// Send whatever is owed to Wavelog again, every few minutes.
    ///
    /// The first look is a short while after start, once the network settings
    /// have arrived, which is what carries a QSO logged with the server down
    /// across a restart. A QSO already on its way is not sent twice: the
    /// engine's queue drops a repeat of one it is still holding.
    ///
    /// One tab only. Every radio tab holds a copy of the shared logbook, and
    /// two of them retrying the same QSOs would double the traffic for
    /// nothing; the station writer is the tab there is exactly one of.
    pub(in crate::app) fn wavelog_retry_tick(&mut self, now: f64) {
        const FIRST_S: f64 = 15.0;
        const EVERY_S: f64 = 300.0;
        // A batch at a time: the queue sends them in order anyway, and the
        // rest are still owed at the next look.
        const BATCH: usize = 200;
        if !self.net_cfg_seeded || !self.station_writer {
            return;
        }
        if self.wavelog_retry_at == 0.0 {
            self.wavelog_retry_at = now + FIRST_S;
            return;
        }
        if now < self.wavelog_retry_at {
            return;
        }
        self.wavelog_retry_at = now + EVERY_S;
        let owed: Vec<(u64, String, Vec<UploadTarget>)> = self
            .wavelog_owed()
            .take(BATCH)
            .map(|q| {
                let adif = sdroxide_types::qso_log_to_adif(std::slice::from_ref(q));
                (q.id, adif, vec![UploadTarget::Wavelog])
            })
            .collect();
        self.pending_uploads.extend(owed);
    }

    /// Start a Wavelog download when one is due: shortly after start, every
    /// `sync_interval_min` after that, or when the settings tab asks. Also
    /// carries out the operator's answer to a full resync.
    ///
    /// Only where the engine runs in this process — the pages are not sent to
    /// remote clients — and only in the station-writer tab, for the reason
    /// the upload retry gives.
    pub(in crate::app) fn wavelog_sync_tick(&mut self, now: f64, cmds: &mut Vec<Command>) {
        const FIRST_S: f64 = 5.0;
        // A download whose end never came (the engine restarted under it) is
        // given up on after this long, so it cannot block every later one.
        const STUCK_S: f64 = 1800.0;
        if let Some(apply) = self.wavelog_resync_answer.take() {
            self.answer_wavelog_resync(apply);
        }
        if !self.net_cfg_seeded || !self.station_writer || self.ctrl.engine_is_remote() {
            return;
        }
        let asked = self.wavelog_sync_request.take();
        if self.wavelog_sync_running.is_some_and(|t| now - t < STUCK_S) {
            return;
        }
        let cfg = &self.net_cfg_edit.wavelog;
        let ready = cfg.is_configured() && !cfg.download_station_ids.is_empty();
        if self.wavelog_sync_at == 0.0 {
            self.wavelog_sync_at = now + FIRST_S;
        }
        let due = cfg.download && now >= self.wavelog_sync_at;
        let full = match asked {
            Some(full) => full,
            None if due => false,
            None => return,
        };
        self.wavelog_sync_at = now + f64::from(cfg.sync_interval_min.max(5)) * 60.0;
        if !ready {
            return;
        }
        let stations = self.wavelog_sync.starts(&cfg.url, &cfg.download_station_ids);
        self.wavelog_sync_running = Some(now);
        self.wavelog_sync_tally = Default::default();
        if full {
            self.wavelog_resync_rx = Some(Default::default());
            self.wavelog_resync = None;
            self.wavelog_sync_status = Some("Full resync: downloading…".into());
        } else {
            self.wavelog_sync_status = Some("Downloading…".into());
        }
        cmds.push(Command::WavelogSync { stations, full });
    }

    /// One page of a Wavelog download.
    pub(in crate::app) fn on_wavelog_batch(&mut self, b: sdroxide_types::WavelogBatch) {
        if b.full {
            self.on_wavelog_resync_batch(b);
            return;
        }
        if !b.records.is_empty() {
            // Another tab may have logged since this copy was read.
            if self.shared_log {
                self.qso_log = crate::app::persist::load_qso_log(None);
            }
            let next = self.next_log_id();
            let m = sdroxide_types::merge_from_wavelog(&mut self.qso_log, b.records, next);
            self.wavelog_sync_tally.added += m.added;
            self.wavelog_sync_tally.matched += m.matched;
            if m.changed {
                persist_qso_log(&self.qso_log);
                self.log_content_changed();
            }
        }
        if !b.done {
            // Saved per page, after the log it describes: an interrupted
            // download resumes from the last page that reached the log.
            self.wavelog_sync.advance(b.station_id, b.last_id);
            crate::app::persist::persist_wavelog_sync(&self.wavelog_sync);
            return;
        }
        self.wavelog_sync_running = None;
        let when = crate::app::util::time_str(crate::time::now_unix());
        let t = self.wavelog_sync_tally;
        let failed = b.error.is_some();
        let line = match b.error {
            Some(e) => format!("{when}Z: {}", e.trim_start_matches("Wavelog: ")),
            None => {
                self.wavelog_sync.last_sync_utc = crate::time::now_unix();
                crate::app::persist::persist_wavelog_sync(&self.wavelog_sync);
                format!("Synced {when}Z: {} new, {} already in the log", t.added, t.matched)
            }
        };
        // The net log hears about a download that brought something or
        // failed; one that found nothing new every quarter of an hour is not
        // news.
        if t.added > 0 || failed {
            self.push_net_log(format!("Wavelog: {line}"));
        }
        self.wavelog_sync_status = Some(line);
    }

    /// A page of a full resync: collected, and worked out when the last one
    /// arrives, for the operator to approve.
    fn on_wavelog_resync_batch(&mut self, b: sdroxide_types::WavelogBatch) {
        let Some((records, last)) = self.wavelog_resync_rx.as_mut() else {
            // Nothing is collecting — the app restarted mid-resync, say — but
            // the end still means nothing is running any more.
            if b.done {
                self.wavelog_sync_running = None;
            }
            return;
        };
        if !b.done {
            records.extend(b.records);
            let l = last.entry(b.station_id).or_insert(0);
            *l = (*l).max(b.last_id);
            return;
        }
        self.wavelog_sync_running = None;
        let (records, last) = self.wavelog_resync_rx.take().unwrap_or_default();
        if let Some(e) = b.error {
            self.wavelog_sync_status = Some(format!("Full resync stopped, nothing changed: {e}"));
            return;
        }
        if self.shared_log {
            self.qso_log = crate::app::persist::load_qso_log(None);
        }
        let plan = sdroxide_types::plan_resync(&self.qso_log, records);
        if plan.is_empty() {
            for (station, id) in &last {
                self.wavelog_sync.advance(*station, *id);
            }
            crate::app::persist::persist_wavelog_sync(&self.wavelog_sync);
            self.wavelog_sync_status = Some("Full resync: the log already matches Wavelog".into());
        } else {
            self.wavelog_sync_status = Some("Full resync: review the changes below".into());
            self.wavelog_resync = Some((plan, last));
        }
    }

    /// Carry out — or drop — the resync the operator was shown.
    fn answer_wavelog_resync(&mut self, apply: bool) {
        let Some((plan, last)) = self.wavelog_resync.take() else { return };
        if !apply {
            self.wavelog_sync_status = Some("Full resync discarded; nothing changed".into());
            return;
        }
        // Another tab may have appended to the shared log while the plan sat
        // waiting for an answer, and writing this copy back would lose its
        // QSOs. Re-reading is safe: tabs only append, and a plan names
        // existing records by id and adds new ones under fresh ids.
        if self.shared_log {
            self.qso_log = crate::app::persist::load_qso_log(None);
        }
        let (add, update, remove, mark) =
            (plan.add.len(), plan.update.len(), plan.remove.len(), plan.mark.len());
        let next = self.next_log_id();
        sdroxide_types::apply_resync(&mut self.qso_log, plan, next);
        persist_qso_log(&self.qso_log);
        self.log_content_changed();
        for (station, id) in &last {
            self.wavelog_sync.advance(*station, *id);
        }
        crate::app::persist::persist_wavelog_sync(&self.wavelog_sync);
        let line = format!(
            "Full resync applied: {add} added, {update} updated, {remove} removed, \
             {mark} of yours marked as in Wavelog"
        );
        self.push_net_log(format!("Wavelog: {line}"));
        self.wavelog_sync_status = Some(line);
    }

    pub(in crate::app) fn push_net_log(&mut self, line: String) {
        self.net_log.insert(0, line);
        self.net_log.truncate(50);
    }

    /// Drain a pending ADIF import: parse, lightly de-dup against the current
    /// log (same call+band within 2 minutes), append with fresh ids, persist.
    ///
    /// The parse runs under [`catch_unwind`] because this is the frame loop
    /// handing an operator-supplied file to a parser: a file it cannot make
    /// sense of has to cost the import, not the application. Nothing touches
    /// the log until the parse has returned, so a failed one leaves it as it
    /// was.
    ///
    /// That backstop is native only. The browser build aborts on a panic, so
    /// there is nothing for `catch_unwind` to catch and a parser panic takes
    /// the page down with it — which is why the parser's own tests feed it
    /// damaged documents (`adif_import_survives_a_truncated_document` and
    /// `adif_import_survives_tags_that_are_not_tags`).
    pub(in crate::app) fn poll_adif_import(&mut self) {
        let loaded = self.adif_import_inbox.lock().ok().and_then(|mut g| g.take());
        let Some(loaded) = loaded else { return };
        // Whatever went wrong is said here rather than on stderr: the operator
        // who clicked Import is looking at the window, and on Windows there is
        // no console behind it to print to.
        let loaded = match loaded {
            Ok(loaded) => loaded,
            Err(e) => {
                self.push_net_log(format!("ADIF import failed: {e}"));
                return;
            }
        };
        let text = loaded.text;
        let parsed =
            catch_unwind(AssertUnwindSafe(|| sdroxide_types::adif_to_qso_log_counting_swl(&text)));
        let Ok((records, swl)) = parsed else {
            self.push_net_log("ADIF import failed: the file could not be parsed".to_string());
            return;
        };
        // One pass to index the existing log by call and band, so the duplicate
        // check only looks at the QSOs that could match rather than at all of
        // them. A full logbook is tens of thousands of records and this runs
        // inside a single frame.
        let mut by_call_band: HashMap<(String, String), Vec<i64>> = HashMap::new();
        for q in &self.qso_log {
            by_call_band
                .entry((q.call.to_ascii_uppercase(), q.band.to_ascii_uppercase()))
                .or_default()
                .push(q.start_utc);
        }
        let mut next_id = self.next_log_id();
        let mut added = 0usize;
        let mut skipped = 0usize;
        for mut r in records {
            if r.call.trim().is_empty() {
                continue;
            }
            let key = (r.call.to_ascii_uppercase(), r.band.to_ascii_uppercase());
            let starts = by_call_band.entry(key).or_default();
            if starts.iter().any(|&t| (t - r.start_utc).abs() < 120) {
                skipped += 1;
                continue;
            }
            starts.push(r.start_utc);
            r.id = next_id;
            next_id += 1;
            self.qso_log.push(r);
            added += 1;
        }
        if added > 0 {
            persist_qso_log(&self.qso_log);
        }
        // Naming the guessed code page is the point of carrying it this far: a
        // name that came out as nonsense is then a code page to report, not a
        // decoder to doubt.
        let assumed = match loaded.assumed {
            Some(enc) => format!(" (not Unicode; read as {enc})"),
            None => String::new(),
        };
        // A file of received reports — the decode list's own ADIF export —
        // adds nothing, and saying why beats reporting an empty import.
        let reports = match swl {
            0 => String::new(),
            n => format!(", {n} received reports (SWL) left out"),
        };
        self.push_net_log(format!(
            "ADIF import: {added} added, {skipped} duplicates skipped{reports}{assumed}"
        ));
    }

    /// Drain a pending channel-list import: parse the CHIRP CSV and hand the
    /// channels to the engine, which owns the memory list and the numbering in
    /// it — including the duplicate check, because only it knows what is
    /// already stored (issue #234).
    ///
    /// Under [`catch_unwind`] for the reason the ADIF import is: this is the
    /// frame loop handing an operator-supplied file to a parser, and a file it
    /// cannot make sense of has to cost the import rather than the application.
    /// Native only, as there: in the browser the parser must simply not panic
    /// (`a_damaged_file_never_panics`).
    pub(in crate::app) fn poll_chirp_import(&mut self, cmds: &mut Vec<Command>) {
        let loaded = self.chirp_import_inbox.lock().ok().and_then(|mut g| g.take());
        let Some(loaded) = loaded else { return };
        let loaded = match loaded {
            Ok(loaded) => loaded,
            Err(e) => {
                self.push_net_log(format!("Channel import failed: {e}"));
                return;
            }
        };
        let text = loaded.text;
        let parsed =
            catch_unwind(AssertUnwindSafe(|| sdroxide_types::chirp_csv_to_memories(&text)));
        let Ok((channels, skipped)) = parsed else {
            self.push_net_log("Channel import failed: the file could not be read".to_string());
            return;
        };
        if channels.is_empty() {
            // The commonest way to get here is a file that is not a CHIRP
            // export at all, so say what was expected rather than "0 imported".
            self.push_net_log(
                "Channel import: nothing to read — a CHIRP CSV file starts with a header                  row naming its columns, one of which must be Frequency"
                    .to_string(),
            );
            return;
        }
        let n = channels.len();
        cmds.push(Command::ImportMemories(channels));
        let skipped = match skipped {
            0 => String::new(),
            n => format!(", {n} line(s) skipped"),
        };
        let assumed = match loaded.assumed {
            Some(enc) => format!(" (not Unicode; read as {enc})"),
            None => String::new(),
        };
        // "read", not "added": the engine drops the channels already stored,
        // and the memory list itself is what shows the result.
        self.push_net_log(format!("Channel import: {n} channel(s) read{skipped}{assumed}"));
    }
}
