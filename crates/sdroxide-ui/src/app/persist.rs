//! Loading and saving the app's own on-disk state.
//!
//! Everything here exists twice, once per target. Natively it goes through
//! [`sdroxide_config`], which writes real files next to the rest of the
//! configuration; in the browser there is no filesystem, so the same state
//! lives in eframe's storage (or, where it is bundled data, nowhere at all).

use sdroxide_types::{QsoRecord, RecordingJob, SwlEntry};

// ── Logbook persistence (native: config-dir JSON; wasm: eframe storage) ──────
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn load_qso_log(_storage: Option<&dyn eframe::Storage>) -> Vec<QsoRecord> {
    sdroxide_config::load_qso_log()
}

#[cfg(target_arch = "wasm32")]
pub(in crate::app) fn load_qso_log(storage: Option<&dyn eframe::Storage>) -> Vec<QsoRecord> {
    storage.and_then(|s| eframe::get_value(s, "qso_log")).unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn persist_qso_log(log: &[QsoRecord]) {
    if let Err(e) = sdroxide_config::save_qso_log(log) {
        eprintln!("failed to save logbook: {e}");
    }
}

#[cfg(target_arch = "wasm32")]
pub(in crate::app) fn persist_qso_log(_log: &[QsoRecord]) {
    // Written by eframe's periodic `save()` into localStorage.
}

// ── Reception log persistence (native: config-dir JSON; wasm: eframe storage) ─
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn load_swl_log(_storage: Option<&dyn eframe::Storage>) -> Vec<SwlEntry> {
    sdroxide_config::load_swl_log()
}

#[cfg(target_arch = "wasm32")]
pub(in crate::app) fn load_swl_log(storage: Option<&dyn eframe::Storage>) -> Vec<SwlEntry> {
    storage.and_then(|s| eframe::get_value(s, "swl_log")).unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn persist_swl_log(log: &[SwlEntry]) {
    if let Err(e) = sdroxide_config::save_swl_log(log) {
        eprintln!("failed to save reception log: {e}");
    }
}

#[cfg(target_arch = "wasm32")]
pub(in crate::app) fn persist_swl_log(_log: &[SwlEntry]) {
    // Written by eframe's periodic `save()` into localStorage.
}

// ── Recording jobs (native: config-dir JSON; wasm: eframe storage) ───────────
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn load_recording_jobs(_storage: Option<&dyn eframe::Storage>) -> Vec<RecordingJob> {
    sdroxide_config::load_recording_jobs()
}

#[cfg(target_arch = "wasm32")]
pub(in crate::app) fn load_recording_jobs(storage: Option<&dyn eframe::Storage>) -> Vec<RecordingJob> {
    storage.and_then(|s| eframe::get_value(s, "recording_jobs")).unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn persist_recording_jobs(jobs: &[RecordingJob]) {
    if let Err(e) = sdroxide_config::save_recording_jobs(jobs) {
        eprintln!("failed to save recording jobs: {e}");
    }
}

#[cfg(target_arch = "wasm32")]
pub(in crate::app) fn persist_recording_jobs(_jobs: &[RecordingJob]) {
    // Written by eframe's periodic `save()` into localStorage.
}

// ── UI/display preferences (native: config.toml [ui]; wasm: eframe storage) ──
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn load_ui_settings(
    _storage: Option<&dyn eframe::Storage>,
) -> sdroxide_types::UiSettings {
    sdroxide_config::load_ui_settings()
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn load_ui_settings(
    storage: Option<&dyn eframe::Storage>,
) -> sdroxide_types::UiSettings {
    storage.and_then(|s| eframe::get_value(s, "ui_settings")).unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn persist_ui_settings(ui: &sdroxide_types::UiSettings) {
    if let Err(e) = sdroxide_config::save_ui_settings(ui) {
        eprintln!("failed to save UI settings: {e}");
    }
}

#[cfg(target_arch = "wasm32")]
pub(in crate::app) fn persist_ui_settings(_ui: &sdroxide_types::UiSettings) {
    // Written by eframe's periodic `save()` into localStorage.
}

// ── Spoken announcements (native: config.toml [speech]) ─────────────────────
//
// A client-side preference like `[ui]`, so it has a browser half too — the
// wasm build carries the announcer with a null sink, and remembering the
// settings there means a later browser backend inherits them rather than
// starting from defaults.

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn load_speech_settings(
    _storage: Option<&dyn eframe::Storage>,
) -> sdroxide_types::SpeechSettings {
    sdroxide_config::load_speech_settings()
}

#[cfg(target_arch = "wasm32")]
pub(in crate::app) fn load_speech_settings(
    storage: Option<&dyn eframe::Storage>,
) -> sdroxide_types::SpeechSettings {
    storage.and_then(|s| eframe::get_value(s, "speech_settings")).unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn persist_speech_settings(cfg: &sdroxide_types::SpeechSettings) {
    if let Err(e) = sdroxide_config::save_speech_settings(cfg) {
        eprintln!("failed to save speech settings: {e}");
    }
}

#[cfg(target_arch = "wasm32")]
pub(in crate::app) fn persist_speech_settings(_cfg: &sdroxide_types::SpeechSettings) {
    // Written by eframe's periodic `save()` into localStorage.
}

// ── Audible alerts (native: config.toml [alerts]) ────────────────────────────
//
// A client-side preference like `[speech]` and `[ui]`, with the same browser
// half: the wasm build has no alarm sink yet, but remembering the settings
// there means a later browser backend inherits them rather than starting from
// defaults.

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn load_alerts_settings(
    _storage: Option<&dyn eframe::Storage>,
) -> sdroxide_types::AlertSettings {
    sdroxide_config::load_alerts_settings()
}

#[cfg(target_arch = "wasm32")]
pub(in crate::app) fn load_alerts_settings(
    storage: Option<&dyn eframe::Storage>,
) -> sdroxide_types::AlertSettings {
    storage.and_then(|s| eframe::get_value(s, "alerts_settings")).unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn persist_alerts_settings(cfg: &sdroxide_types::AlertSettings) {
    if let Err(e) = sdroxide_config::save_alerts_settings(cfg) {
        eprintln!("failed to save alerts settings: {e}");
    }
}

#[cfg(target_arch = "wasm32")]
pub(in crate::app) fn persist_alerts_settings(_cfg: &sdroxide_types::AlertSettings) {
    // Written by eframe's periodic `save()` into localStorage.
}

// ── Remote-access credentials (native: config.toml [remote_access]) ──────────
//
// Who may connect to *this* machine's server. There is no browser half: these
// are a file on the machine the radio is attached to, and a browser client is
// by definition not it — the General tab hides the editor rather than offering
// one that writes nowhere.

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn load_remote_access() -> sdroxide_types::RemoteAccess {
    sdroxide_config::load_remote_access()
}

#[cfg(target_arch = "wasm32")]
pub(in crate::app) fn load_remote_access() -> sdroxide_types::RemoteAccess {
    sdroxide_types::RemoteAccess::default()
}

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn persist_remote_access(access: &sdroxide_types::RemoteAccess) {
    if let Err(e) = sdroxide_config::save_remote_access(access) {
        eprintln!("failed to save the remote-access credentials: {e}");
    }
}

#[cfg(target_arch = "wasm32")]
pub(in crate::app) fn persist_remote_access(_access: &sdroxide_types::RemoteAccess) {}

// ── Remote server address (native: config.toml [remote_server]) ──────────────
//
// The other direction: which station *this* screen dials, from Settings →
// Remote. No browser half at all, unlike everything above — a browser client is
// already attached to the server that served it and has nowhere to put a second
// connection, so that tab does not exist there and there is nothing to remember.

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn load_remote_server() -> sdroxide_types::RemoteServer {
    sdroxide_config::load_remote_server()
}

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn persist_remote_server(server: &sdroxide_types::RemoteServer) {
    if let Err(e) = sdroxide_config::save_remote_server(server) {
        eprintln!("failed to save the remote server address: {e}");
    }
}

// ── Broadcast stations ───────────────────────────────────────────────────────
//
// Native: the cached season schedule (or the compiled-in one until a download
// lands), plus the operator's own entries. Wasm: the compiled-in schedule, since
// the browser tab has nowhere to cache a download and no config file to overlay.

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn load_broadcast_stations() -> Vec<sdroxide_types::BroadcastStation> {
    sdroxide_config::load_broadcast_stations()
}

#[cfg(target_arch = "wasm32")]
pub(in crate::app) fn load_broadcast_stations() -> Vec<sdroxide_types::BroadcastStation> {
    sdroxide_types::broadcast::builtin().to_vec()
}

// ── Band conditions ──────────────────────────────────────────────────────────
//
// Fetched here rather than by the 3D view, and this is the one product of the
// solar feed that works that way. The verdicts colour the band menu, which is
// on screen for the whole session — waiting for a window most operators never
// open would have left the feature blank for them.
//
// It is still one request an hour: this shares the solar disk cache and its
// validators with the feed's own copy, so with the 3D view open the second of
// the two is a conditional GET that comes back 304. The publisher asks for
// hourly at most and that is what is honoured.

/// Fetch (or reuse) the published band conditions on a worker thread.
///
/// Off the UI thread because it is a network round trip; the app picks the
/// result up from the receiver on a later frame. `None` means the thread could
/// not be spawned at all — an expired cache and an unreachable server both
/// arrive as `Ok(None)` on the channel instead, and leave the last known
/// verdicts on screen with their age.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn spawn_band_conditions_fetch()
-> Option<std::sync::mpsc::Receiver<Option<sdroxide_solar::BandConditions>>> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("band-conditions".into())
        .spawn(move || {
            let _ = tx.send(sdroxide_solar::band_conditions_cached());
        })
        .ok()?;
    Some(rx)
}

/// The browser has no disk cache and no HTTP client of its own; a viewer's
/// verdicts arrive over the solar relay instead.
#[cfg(target_arch = "wasm32")]
pub(in crate::app) fn spawn_band_conditions_fetch()
-> Option<std::sync::mpsc::Receiver<Option<sdroxide_solar::BandConditions>>> {
    None
}

/// Fetch (or reuse) the global WSPR activity on a worker thread.
///
/// The same shape as [`spawn_band_conditions_fetch`], and off the UI thread for
/// the same reason. `None` means the thread could not be spawned; an
/// unreachable database and an expired cache both arrive as `Ok(None)` on the
/// channel instead.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn spawn_band_activity_fetch()
-> Option<std::sync::mpsc::Receiver<Option<sdroxide_solar::BandActivityTable>>> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("band-activity".into())
        .spawn(move || {
            let _ = tx.send(sdroxide_solar::band_activity_cached());
        })
        .ok()?;
    Some(rx)
}

/// The browser gets no disk cache and no HTTP client of its own here; the
/// measured column stays empty there rather than guessed.
#[cfg(target_arch = "wasm32")]
pub(in crate::app) fn spawn_band_activity_fetch()
-> Option<std::sync::mpsc::Receiver<Option<sdroxide_solar::BandActivityTable>>> {
    None
}

/// Fetch (or reuse) the global PSK Reporter activity on a worker thread. The
/// activity-mode sibling of [`spawn_band_activity_fetch`].
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn spawn_psk_activity_fetch()
-> Option<std::sync::mpsc::Receiver<Option<sdroxide_solar::BandActivityTable>>> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("psk-activity".into())
        .spawn(move || {
            let _ = tx.send(sdroxide_solar::psk_activity_cached());
        })
        .ok()?;
    Some(rx)
}

#[cfg(target_arch = "wasm32")]
pub(in crate::app) fn spawn_psk_activity_fetch()
-> Option<std::sync::mpsc::Receiver<Option<sdroxide_solar::BandActivityTable>>> {
    None
}

// ── Schedule download ───────────────────────────────────────────────────────

/// The result of a background schedule download.
pub(in crate::app) type ScheduleFetch = Result<Vec<sdroxide_types::BroadcastStation>, String>;

/// Download the current season's schedule on a worker thread.
///
/// Off the UI thread because it is a megabyte over a link that may not be there;
/// the app picks the result up from the receiver on a later frame. Returns `None`
/// when nothing needs fetching, which after a first run is every start until the
/// season turns over.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn spawn_schedule_fetch(
    force: bool,
) -> Option<std::sync::mpsc::Receiver<ScheduleFetch>> {
    if !force && !sdroxide_config::broadcast_schedule_due() {
        return None;
    }
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("broadcast-schedule".into())
        .spawn(move || {
            let _ = tx.send(sdroxide_config::fetch_broadcast_schedule());
        })
        .ok()?;
    Some(rx)
}

/// The browser client has no cache to fill, so there is nothing to fetch.
#[cfg(target_arch = "wasm32")]
pub(in crate::app) fn spawn_schedule_fetch(
    _force: bool,
) -> Option<std::sync::mpsc::Receiver<ScheduleFetch>> {
    None
}

/// Drop the cached schedule so the next fetch downloads it again.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn clear_broadcast_cache() {
    if let Err(e) = sdroxide_config::clear_broadcast_cache() {
        eprintln!("failed to clear the broadcast schedule cache: {e}");
    }
}

#[cfg(target_arch = "wasm32")]
pub(in crate::app) fn clear_broadcast_cache() {}

// ── The band-plan file's path (shown on the General tab) ─────────────────────

/// Where `bandplan.json` is, so the settings dialog can name the file the
/// operator has to edit. `None` when there is no config directory to point at.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn band_plan_path() -> Option<std::path::PathBuf> {
    sdroxide_config::band_plan_path()
}

/// The browser client has no filesystem, and the file it would want to name is
/// on the engine's machine in any case.
#[cfg(target_arch = "wasm32")]
pub(in crate::app) fn band_plan_path() -> Option<std::path::PathBuf> {
    None
}
