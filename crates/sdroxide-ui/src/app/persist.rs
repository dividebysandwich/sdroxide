//! Loading and saving the app's own on-disk state.
//!
//! Everything here exists twice, once per target. Natively it goes through
//! [`sdroxide_config`], which writes real files next to the rest of the
//! configuration; in the browser there is no filesystem, so the same state
//! lives in eframe's storage (or, where it is bundled data, nowhere at all).

use sdroxide_types::QsoRecord;

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

// ── Update check ─────────────────────────────────────────────────────────────
//
// One request to sdroxide.com per start, asking what the released version is.
// Native only: the check is about *this binary* being out of date, and the
// browser client has no binary — it is served fresh by its station every time.

/// Ask sdroxide.com for the released version on a worker thread.
///
/// Off the UI thread because it is a network round trip; the app picks the
/// result up from the receiver on a later frame. A version only arrives on
/// the channel when it is strictly newer than the running build *and* differs
/// from the one whose banner the operator already dismissed — an unreachable
/// site, a current or ahead-of-release build and a dismissed release all just
/// let the channel close quietly.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn spawn_update_check() -> Option<std::sync::mpsc::Receiver<String>> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("update-check".into())
        .spawn(move || {
            let Some(published) = sdroxide_config::fetch_published_version() else { return };
            if sdroxide_config::version_is_newer(&published, env!("CARGO_PKG_VERSION"))
                && published != sdroxide_config::load_dismissed_update()
            {
                let _ = tx.send(published);
            }
        })
        .ok()?;
    Some(rx)
}

/// No binary to be out of date in the browser, so nothing to check.
#[cfg(target_arch = "wasm32")]
pub(in crate::app) fn spawn_update_check() -> Option<std::sync::mpsc::Receiver<String>> {
    None
}

/// Remember that the operator dismissed the banner for `version`, so it only
/// returns when a different release is published.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn persist_dismissed_update(version: &str) {
    if let Err(e) = sdroxide_config::save_dismissed_update(version) {
        eprintln!("failed to save the dismissed update version: {e}");
    }
}

#[cfg(target_arch = "wasm32")]
pub(in crate::app) fn persist_dismissed_update(_version: &str) {}

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
