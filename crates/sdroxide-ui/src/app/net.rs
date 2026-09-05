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
        self.push_net_log(format!("{} → {}: {}", r.target.label(), status, r.message));
        if r.ok {
            if let Some(rec) = self.qso_log.iter_mut().find(|q| q.id == r.qso_id) {
                match r.target {
                    UploadTarget::Eqsl => rec.eqsl_sent = true,
                    UploadTarget::QrzLogbook => rec.qrz_sent = true,
                    UploadTarget::ClubLog => rec.clublog_sent = true,
                    UploadTarget::HamQth => rec.hamqth_sent = true,
                    UploadTarget::Wrl => rec.wrl_sent = true,
                }
                persist_qso_log(&self.qso_log);
            }
        }
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
        let parsed = catch_unwind(AssertUnwindSafe(|| sdroxide_types::adif_to_qso_log(&text)));
        let Ok(records) = parsed else {
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
        self.push_net_log(format!(
            "ADIF import: {added} added, {skipped} duplicates skipped{assumed}"
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
