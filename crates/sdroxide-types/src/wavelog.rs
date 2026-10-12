//! Wavelog (<https://www.wavelog.org/>), the self-hosted web logbook: where it
//! is, how to reach it, and what it told us about itself.
//!
//! A Wavelog is somebody's own server, not a public service, which is the
//! reason this is a config of its own rather than a key beside the others in
//! [`crate::NetworkConfig`]. It is reached at whatever address the operator
//! gave it — a hostname with a proper certificate, a LAN address over plain
//! HTTP, or HTTPS with a certificate it signed itself — and every QSO goes to
//! one of the *station locations* defined in it, which the API names by
//! number.
//!
//! The location is chosen per operating identity: [`crate::DigiConfig`]
//! carries `wavelog_station_id`, so a profile that works as a portable call
//! logs to the portable location. Wavelog insists on it — a QSO whose
//! `STATION_CALLSIGN` is not the location's callsign is refused — so the
//! location belongs with the callsign rather than with the server.

use serde::{Deserialize, Serialize};

/// How to reach the operator's Wavelog, and what to do with it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WavelogConfig {
    /// The address the Wavelog front page is served from, as the operator
    /// would type it into a browser: `https://log.example.org`,
    /// `http://192.168.1.10/wavelog`. A trailing `/index.php` is accepted
    /// and so is a missing one; see `sdroxide_net::wavelog::api_url`.
    pub url: String,
    /// An API key from Wavelog's own API page. Uploading needs a read/write
    /// key; a read-only one can only download.
    pub api_key: String,
    /// SHA-256 of the one certificate to accept from this server, as 64 hex
    /// digits, for a Wavelog behind a certificate it signed itself. Empty
    /// means ordinary verification against the public roots.
    ///
    /// A pin rather than "accept any certificate": the operator trusts *their*
    /// server's certificate, which is a different and much smaller thing than
    /// trusting whatever answers at that address. A changed certificate is then
    /// refused and reported, not silently used.
    pub cert_sha256: String,
    /// The station location QSOs go to when the operating profile does not
    /// name one of its own (`DigiConfig::wavelog_station_id` is 0). 0 means
    /// none chosen yet, and an upload with no location fails saying so.
    pub station_id: u32,
    /// Send each newly logged QSO to Wavelog. Subject to the master
    /// `NetworkConfig::auto_upload` switch, like every other service.
    pub auto_upload: bool,
    /// When `auto_upload` was switched on, as a Unix time.
    ///
    /// The retry queue is defined by it: a QSO logged since then that Wavelog
    /// has not acknowledged is still owed, and is sent again until it is —
    /// across restarts, because the owed state is read off the logbook rather
    /// than kept in memory. Contacts from before are the backlog, which is
    /// sent only when the operator asks.
    pub auto_upload_since: i64,
    /// Bring the QSOs of `download_station_ids` into the logbook: at start,
    /// and then every `sync_interval_min`. What makes the worked-before and
    /// new-entity marks on a decode know about contacts made from another
    /// program, another rig or the web form.
    pub download: bool,
    /// The station locations whose QSOs are downloaded. A list the operator
    /// ticks rather than "all of them": an old QTH, a club call or a special
    /// event station may not be one they want counted as worked from here.
    pub download_station_ids: Vec<u32>,
    /// Minutes between downloads after the first. Each is incremental — only
    /// what Wavelog has added since the last — so it is cheap.
    pub sync_interval_min: u32,
    /// Tell Wavelog the radio's frequency and mode as they change, for its
    /// live-logging form and bandmap — the other half of what WaveLogGate
    /// does. Not the power: sdroxide knows the drive as a fraction and the
    /// forward power only while transmitting, and Wavelog's power field is
    /// watts set on the rig — a number made up from either would be wrong.
    pub radio_status: bool,
    /// The name the radio is listed under in Wavelog's radio selector.
    pub radio_name: String,
}

impl Default for WavelogConfig {
    fn default() -> Self {
        WavelogConfig {
            url: String::new(),
            api_key: String::new(),
            cert_sha256: String::new(),
            station_id: 0,
            auto_upload: false,
            auto_upload_since: 0,
            download: false,
            download_station_ids: Vec::new(),
            sync_interval_min: 15,
            radio_status: false,
            radio_name: "sdroxide".into(),
        }
    }
}

impl WavelogConfig {
    /// Whether enough is filled in to talk to the server at all.
    pub fn is_configured(&self) -> bool {
        !self.url.trim().is_empty() && !self.api_key.trim().is_empty()
    }

    /// The station location a QSO logged under `profile_station` goes to:
    /// the profile's own when it names one, this config's default when not.
    pub fn station_for(&self, profile_station: u32) -> u32 {
        if profile_station != 0 { profile_station } else { self.station_id }
    }
}

/// One station location, as Wavelog's `station_info` lists it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WavelogStation {
    pub id: u32,
    /// The operator's own name for it — "Home", "Portable".
    pub name: String,
    pub callsign: String,
    pub grid: String,
    /// The location Wavelog's own web logging is currently set to.
    pub active: bool,
}

impl WavelogStation {
    /// "Home — EA5ABC / IM99" for a dropdown.
    pub fn label(&self) -> String {
        let grid = if self.grid.is_empty() { String::new() } else { format!(" / {}", self.grid) };
        format!("{} — {}{grid}", self.name, self.callsign)
    }
}

/// What a connection check found out about the server.
///
/// More than the pass/fail of a [`crate::LoginTestResult`], which it is sent
/// alongside: the settings need the station locations to offer as choices,
/// and, when the server presented a certificate nothing vouches for, the
/// fingerprint the operator is being asked to trust.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WavelogProbe {
    pub ok: bool,
    /// What happened, in a sentence for the settings tab.
    pub message: String,
    /// Whether the key may write. `None` when the check did not get that far.
    pub read_write: Option<bool>,
    /// The Wavelog version the server reports, empty when unknown.
    pub version: String,
    pub stations: Vec<WavelogStation>,
    /// The SHA-256 (64 hex digits) of a certificate the server presented that
    /// failed verification, for the operator to look at and pin if it is
    /// theirs. `None` when TLS was not the problem.
    pub untrusted_cert: Option<String>,
}

/// One page of QSOs downloaded from one station location.
///
/// A sync arrives as a run of these rather than as one list: a first download
/// can be a whole log, and taking it a page at a time means the logbook — and
/// the cursor saying how far it got — moves forward as it goes, so an
/// interrupted sync resumes rather than restarts.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WavelogBatch {
    pub station_id: u32,
    /// The cursor this page was asked from.
    pub from_id: u64,
    /// The highest Wavelog record id in this page — where the next ask
    /// starts. Equal to `from_id` when there was nothing new.
    pub last_id: u64,
    pub records: Vec<crate::QsoRecord>,
    /// A full resync's download, collected for the operator to approve rather
    /// than merged as it arrives.
    pub full: bool,
    /// The last batch of the whole sync, every location included.
    pub done: bool,
    /// Why the sync stopped early. Comes with `done`.
    pub error: Option<String>,
}

/// How far each station location's QSOs have been downloaded, kept with the
/// logbook rather than with the settings: it describes what the log holds.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WavelogSyncState {
    /// The server the cursors are for. Record ids mean nothing on another
    /// Wavelog, so a changed address starts every location from the top.
    pub url: String,
    /// Station location → the highest Wavelog record id already merged.
    pub cursors: std::collections::BTreeMap<u32, u64>,
    /// When the last sync finished, as a Unix time; 0 for never.
    pub last_sync_utc: i64,
}

impl WavelogSyncState {
    /// Where to start each of `stations` from on the server at `url`.
    pub fn starts(&mut self, url: &str, stations: &[u32]) -> Vec<(u32, u64)> {
        let url = url.trim().trim_end_matches('/').to_string();
        if self.url != url {
            self.url = url;
            self.cursors.clear();
        }
        stations.iter().map(|&s| (s, self.cursors.get(&s).copied().unwrap_or(0))).collect()
    }

    /// Note that `station` has been merged up to `last_id`.
    pub fn advance(&mut self, station: u32, last_id: u64) {
        let c = self.cursors.entry(station).or_insert(0);
        *c = (*c).max(last_id);
    }
}

/// Whether two records are the same contact: the rule Wavelog itself uses for
/// a duplicate — call, band, and time — with the time allowed two minutes'
/// slack and the station callsign compared only where both have one.
///
/// Mode is left out on purpose. The same FT4 contact is `FT4` in one log and
/// `MFSK`/`FT4` in another, and an SSB one `SSB` or `USB`; the same callsign
/// on the same band within two minutes in two different modes is not a thing
/// that happens. The same window as sdroxide's own ADIF import.
pub fn same_contact(a: &crate::QsoRecord, b: &crate::QsoRecord) -> bool {
    a.call.eq_ignore_ascii_case(&b.call)
        && a.band.eq_ignore_ascii_case(&b.band)
        && (a.start_utc - b.start_utc).abs() < 120
        && (a.my_call.trim().is_empty()
            || b.my_call.trim().is_empty()
            || a.my_call.trim().eq_ignore_ascii_case(b.my_call.trim()))
}

/// What a merge did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WavelogMerge {
    /// Contacts new to the log, appended.
    pub added: usize,
    /// Contacts the log already had, now marked as in Wavelog.
    pub matched: usize,
    /// Whether anything in the log changed and it needs saving.
    pub changed: bool,
}

/// Fold downloaded contacts into the log.
///
/// A contact the log already has is marked as in Wavelog and given the
/// confirmations Wavelog knows about, and nothing else of it is touched: the
/// local record is the operator's, edited or not. One it does not have is
/// appended with a fresh id. `next_id` is the first free logbook id.
pub fn merge_from_wavelog(
    log: &mut Vec<crate::QsoRecord>,
    incoming: Vec<crate::QsoRecord>,
    mut next_id: u64,
) -> WavelogMerge {
    let mut out = WavelogMerge::default();
    // Indexed by call and band, so each incoming contact is compared with the
    // few that could match rather than with a whole log.
    let mut by_key: std::collections::HashMap<(String, String), Vec<usize>> =
        std::collections::HashMap::new();
    for (i, q) in log.iter().enumerate() {
        by_key.entry(key(q)).or_default().push(i);
    }
    for mut r in incoming {
        if r.call.trim().is_empty() {
            continue;
        }
        let k = key(&r);
        let found =
            by_key.get(&k).and_then(|ix| ix.iter().copied().find(|&i| same_contact(&log[i], &r)));
        match found {
            Some(i) => {
                let local = &mut log[i];
                let before = (local.wavelog_sent, local.lotw_rcvd, local.eqsl_rcvd, local.qsl_rcvd);
                local.wavelog_sent = true;
                local.lotw_rcvd |= r.lotw_rcvd;
                local.eqsl_rcvd |= r.eqsl_rcvd;
                local.qsl_rcvd |= r.qsl_rcvd;
                if before != (local.wavelog_sent, local.lotw_rcvd, local.eqsl_rcvd, local.qsl_rcvd)
                {
                    out.changed = true;
                }
                out.matched += 1;
            }
            None => {
                r.id = next_id;
                next_id += 1;
                r.wavelog_sent = true;
                by_key.entry(k).or_default().push(log.len());
                log.push(r);
                out.added += 1;
                out.changed = true;
            }
        }
    }
    out
}

/// What a full resync would change, worked out before anything is changed so
/// the operator can read it first.
///
/// Only contacts that came from Wavelog (`wavelog_origin` set) are ever
/// updated or removed. The operator's own — logged here or imported from a
/// file — can only be marked as being in Wavelog, never edited or deleted to
/// match it: Wavelog is where these contacts are mirrored *from*, not the
/// owner of every record that happens to look like one of its own.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WavelogResync {
    /// In Wavelog, not in the log: appended.
    pub add: Vec<crate::QsoRecord>,
    /// Downloaded before and changed in Wavelog since: the local record (by
    /// id) is replaced with this, its local upload flags kept.
    pub update: Vec<crate::QsoRecord>,
    /// Downloaded before and no longer in Wavelog — deleted there, or from a
    /// location no longer ticked: these ids go.
    pub remove: Vec<u64>,
    /// The operator's own contacts Wavelog has: these ids get `wavelog_sent`
    /// and Wavelog's confirmations. Carried as the downloaded record, whose
    /// confirmation flags are what is applied.
    pub mark: Vec<(u64, crate::QsoRecord)>,
}

impl WavelogResync {
    pub fn is_empty(&self) -> bool {
        self.add.is_empty()
            && self.update.is_empty()
            && self.remove.is_empty()
            && self.mark.is_empty()
    }
}

/// Compare a full download of the ticked locations with the log.
pub fn plan_resync(log: &[crate::QsoRecord], downloaded: Vec<crate::QsoRecord>) -> WavelogResync {
    let mut plan = WavelogResync::default();
    let mut by_key: std::collections::HashMap<(String, String), Vec<usize>> =
        std::collections::HashMap::new();
    for (i, q) in log.iter().enumerate() {
        by_key.entry(key(q)).or_default().push(i);
    }
    // A local record answers for one downloaded contact at most, so two
    // contacts a minute apart in Wavelog do not both land on one local record.
    let mut claimed = vec![false; log.len()];
    for d in downloaded {
        if d.call.trim().is_empty() {
            continue;
        }
        let found = by_key
            .get(&key(&d))
            .and_then(|ix| ix.iter().copied().find(|&i| !claimed[i] && same_contact(&log[i], &d)));
        let Some(i) = found else {
            plan.add.push(d);
            continue;
        };
        claimed[i] = true;
        let local = &log[i];
        if local.wavelog_origin != 0 {
            let merged = keep_local_flags(local, d);
            if merged != *local {
                plan.update.push(merged);
            }
        } else {
            let unchanged = local.wavelog_sent
                && (local.lotw_rcvd || !d.lotw_rcvd)
                && (local.eqsl_rcvd || !d.eqsl_rcvd)
                && (local.qsl_rcvd || !d.qsl_rcvd);
            if !unchanged {
                plan.mark.push((local.id, d));
            }
        }
    }
    plan.remove = log
        .iter()
        .zip(&claimed)
        .filter(|(q, claimed)| q.wavelog_origin != 0 && !**claimed)
        .map(|(q, _)| q.id)
        .collect();
    plan
}

/// The downloaded record as it will stand in the log: Wavelog's content under
/// the local id, with the uploads this station made to other services kept —
/// Wavelog does not know sdroxide sent the contact to QRZ.
fn keep_local_flags(local: &crate::QsoRecord, mut d: crate::QsoRecord) -> crate::QsoRecord {
    d.id = local.id;
    d.wavelog_sent = true;
    d.lotw_sent |= local.lotw_sent;
    d.eqsl_sent |= local.eqsl_sent;
    d.qrz_sent |= local.qrz_sent;
    d.clublog_sent |= local.clublog_sent;
    d.hamqth_sent |= local.hamqth_sent;
    d.wrl_sent |= local.wrl_sent;
    d.qsl_sent |= local.qsl_sent;
    d
}

/// Carry out a [`WavelogResync`]. `next_id` is the first free logbook id.
pub fn apply_resync(log: &mut Vec<crate::QsoRecord>, plan: WavelogResync, mut next_id: u64) {
    let gone: std::collections::HashSet<u64> = plan.remove.into_iter().collect();
    log.retain(|q| !gone.contains(&q.id));
    let updates: std::collections::HashMap<u64, crate::QsoRecord> =
        plan.update.into_iter().map(|q| (q.id, q)).collect();
    let marks: std::collections::HashMap<u64, crate::QsoRecord> = plan.mark.into_iter().collect();
    for q in log.iter_mut() {
        if let Some(u) = updates.get(&q.id) {
            *q = u.clone();
        } else if let Some(d) = marks.get(&q.id) {
            q.wavelog_sent = true;
            q.lotw_rcvd |= d.lotw_rcvd;
            q.eqsl_rcvd |= d.eqsl_rcvd;
            q.qsl_rcvd |= d.qsl_rcvd;
        }
    }
    for mut a in plan.add {
        a.id = next_id;
        next_id += 1;
        a.wavelog_sent = true;
        log.push(a);
    }
}

fn key(q: &crate::QsoRecord) -> (String, String) {
    (q.call.trim().to_ascii_uppercase(), q.band.trim().to_ascii_lowercase())
}

/// `ab12…` → `AB:12:…`, the way browsers and `openssl x509 -fingerprint`
/// print a certificate fingerprint, so the operator can compare like with like.
pub fn fingerprint_display(hex: &str) -> String {
    let hex = hex.trim().to_ascii_uppercase();
    let mut out = String::with_capacity(hex.len() * 3 / 2);
    for (i, c) in hex.chars().enumerate() {
        if i > 0 && i % 2 == 0 {
            out.push(':');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_profile_location_wins_over_the_default() {
        let cfg = WavelogConfig { station_id: 3, ..Default::default() };
        assert_eq!(cfg.station_for(7), 7);
        assert_eq!(cfg.station_for(0), 3);
    }

    fn qso(call: &str, band: &str, t: i64) -> crate::QsoRecord {
        crate::QsoRecord {
            call: call.into(),
            band: band.into(),
            start_utc: t,
            ..Default::default()
        }
    }

    #[test]
    fn a_contact_already_in_the_log_is_marked_not_duplicated() {
        let mut log = vec![crate::QsoRecord { id: 1, ..qso("EA1AA", "20m", 1000) }];
        let got = crate::QsoRecord { lotw_rcvd: true, ..qso("ea1aa", "20M", 1060) };
        let m = merge_from_wavelog(&mut log, vec![got], 2);
        assert_eq!((m.added, m.matched, m.changed), (0, 1, true));
        assert_eq!(log.len(), 1);
        assert!(log[0].wavelog_sent && log[0].lotw_rcvd);
    }

    #[test]
    fn a_new_contact_is_appended_with_a_fresh_id_and_marked() {
        let mut log = vec![crate::QsoRecord { id: 7, ..qso("EA1AA", "20m", 1000) }];
        let incoming = vec![qso("EA1AA", "40m", 1000), qso("EA1AA", "20m", 5000)];
        let m = merge_from_wavelog(&mut log, incoming, 8);
        assert_eq!((m.added, m.matched), (2, 0));
        assert_eq!(log.iter().map(|q| q.id).collect::<Vec<_>>(), vec![7, 8, 9]);
        assert!(log[1].wavelog_sent && log[2].wavelog_sent);
    }

    #[test]
    fn the_same_download_twice_adds_nothing_the_second_time() {
        let mut log = Vec::new();
        let page = vec![qso("EA1AA", "20m", 1000), qso("F4XYZ", "15m", 2000)];
        merge_from_wavelog(&mut log, page.clone(), 1);
        let m = merge_from_wavelog(&mut log, page, 3);
        assert_eq!((m.added, m.matched, m.changed), (0, 2, false));
    }

    #[test]
    fn a_different_station_callsign_is_a_different_contact() {
        let mut log =
            vec![crate::QsoRecord { my_call: "EA5ABC".into(), ..qso("EA1AA", "20m", 1000) }];
        let other = crate::QsoRecord { my_call: "EA5ABC/P".into(), ..qso("EA1AA", "20m", 1000) };
        assert_eq!(merge_from_wavelog(&mut log, vec![other], 2).added, 1);
    }

    fn from_wavelog(id: u64, call: &str, t: i64) -> crate::QsoRecord {
        crate::QsoRecord { id, wavelog_origin: 4, wavelog_sent: true, ..qso(call, "20m", t) }
    }

    #[test]
    fn a_resync_adds_updates_and_removes_only_what_came_from_wavelog() {
        let mine = crate::QsoRecord { id: 1, qrz_sent: true, ..qso("EA1AA", "20m", 1000) };
        let stale = from_wavelog(2, "F4XYZ", 2000);
        let edited = crate::QsoRecord { qrz_sent: true, ..from_wavelog(3, "DL1ABC", 3000) };
        let mine_gone = crate::QsoRecord { id: 4, ..qso("G4ABC", "20m", 4000) };
        let log = vec![mine.clone(), stale, edited, mine_gone];

        let mut fixed = from_wavelog(0, "DL1ABC", 3000);
        fixed.name = "Hans".into();
        let downloaded = vec![
            crate::QsoRecord { lotw_rcvd: true, ..from_wavelog(0, "EA1AA", 1030) },
            fixed,
            from_wavelog(0, "JA1XYZ", 9000),
        ];
        let plan = plan_resync(&log, downloaded);

        assert_eq!(plan.add.iter().map(|q| q.call.as_str()).collect::<Vec<_>>(), vec!["JA1XYZ"]);
        assert_eq!(plan.update.len(), 1);
        assert_eq!((plan.update[0].id, plan.update[0].name.as_str()), (3, "Hans"));
        assert!(plan.update[0].qrz_sent, "an upload made from here is kept");
        // F4XYZ was deleted in Wavelog; G4ABC is the operator's own and stays.
        assert_eq!(plan.remove, vec![2]);
        assert_eq!(plan.mark.iter().map(|(id, _)| *id).collect::<Vec<_>>(), vec![1]);

        let mut log = log;
        apply_resync(&mut log, plan, 10);
        let calls: Vec<_> = log.iter().map(|q| (q.id, q.call.as_str())).collect();
        assert_eq!(calls, vec![(1, "EA1AA"), (3, "DL1ABC"), (4, "G4ABC"), (10, "JA1XYZ")]);
        assert!(log[0].wavelog_sent && log[0].lotw_rcvd && log[0].qrz_sent);
        assert_eq!(log[1].name, "Hans");
    }

    #[test]
    fn a_resync_of_a_log_already_in_step_changes_nothing() {
        let log = vec![from_wavelog(1, "EA1AA", 1000), from_wavelog(2, "F4XYZ", 2000)];
        let downloaded = vec![from_wavelog(0, "EA1AA", 1000), from_wavelog(0, "F4XYZ", 2000)];
        assert!(plan_resync(&log, downloaded).is_empty());
    }

    #[test]
    fn two_contacts_a_minute_apart_do_not_share_one_local_record() {
        let log = vec![from_wavelog(1, "EA1AA", 1000)];
        let downloaded = vec![from_wavelog(0, "EA1AA", 1000), from_wavelog(0, "EA1AA", 1060)];
        let plan = plan_resync(&log, downloaded);
        assert_eq!((plan.add.len(), plan.update.len(), plan.remove.len()), (1, 0, 0));
    }

    #[test]
    fn a_new_server_address_starts_every_location_over() {
        let mut st = WavelogSyncState::default();
        assert_eq!(st.starts("https://a.example/", &[1, 2]), vec![(1, 0), (2, 0)]);
        st.advance(1, 500);
        st.advance(1, 300);
        assert_eq!(st.starts("https://a.example", &[1, 2]), vec![(1, 500), (2, 0)]);
        assert_eq!(st.starts("https://b.example", &[1]), vec![(1, 0)]);
    }

    #[test]
    fn a_fingerprint_reads_the_way_openssl_prints_it() {
        assert_eq!(fingerprint_display("ab12cd"), "AB:12:CD");
        assert_eq!(fingerprint_display(""), "");
    }
}
