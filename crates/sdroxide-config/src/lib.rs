//! Settings and radio-data persistence under the user config directory
//! (`~/.config/sdroxide/` on Linux).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

/// Configuration files that had to be reset to defaults because their content
/// would not parse. A log line is not enough for these: the operator's device
/// pinning and gain setup silently vanish with such a reset, and the first
/// symptom is a radio that behaves like it was never configured. The engine
/// drains this into the on-screen notice at startup and on every source swap.
static LOAD_ALERTS: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Drain the pending configuration-reset alerts (oldest first).
pub fn take_load_alerts() -> Vec<String> {
    std::mem::take(&mut *LOAD_ALERTS.lock().unwrap_or_else(std::sync::PoisonError::into_inner))
}

/// Tell the operator something at startup that is **not** a configuration reset,
/// through the same queue, because the place it surfaces is the same notice.
///
/// Used for the start frequency the hardware could not reach and the fallback
/// that replaced it (#72): the operator must be told their radio opened
/// somewhere else, or the dial reads a frequency they never chose.
pub fn note_startup(msg: String) {
    push_load_alert(msg);
}

fn push_load_alert(msg: String) {
    LOAD_ALERTS.lock().unwrap_or_else(std::sync::PoisonError::into_inner).push(msg);
}

/// Files that are **there** and could not be read: locked by something else,
/// unreadable to this user, a device error. Held by full path, so one radio's
/// scoped `radio.json` is not confused with another's.
///
/// A different failure from a file that read fine and would not parse, and the
/// difference is the whole of issue #269. There, the content is in hand and can
/// be moved aside as `<file>.bak`; here it is still on the disk and is still the
/// operator's only copy of it.
///
/// It used to be treated as the third case instead — an absent file — which is
/// the ordinary first-run one, so it degraded to the defaults in silence. A
/// `memories.json` that a backup or antivirus tool had open for the moment
/// sdroxide read it therefore came up as an empty list, said nothing, and the
/// next memory the operator stored wrote that empty list back over every
/// channel they had. Nothing was quarantined, because nothing had failed to
/// parse.
///
/// So a file in here is not written: [`write_atomic`] refuses until some load
/// has read it successfully. A refused save is an error the operator can see
/// and act on; a completed one is data they do not get back.
static UNREADABLE: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

/// Record that `path` could not be read. Answers whether this is news, so the
/// operator is told once rather than on every load that goes past it.
fn mark_unreadable(path: &Path) -> bool {
    let mut held = UNREADABLE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if held.iter().any(|p| p == path) {
        return false;
    }
    held.push(path.to_path_buf());
    true
}

/// `path` can be read (or is honestly absent), so it may be written again.
fn clear_unreadable(path: &Path) {
    UNREADABLE.lock().unwrap_or_else(std::sync::PoisonError::into_inner).retain(|p| p != path);
}

fn is_unreadable(path: &Path) -> bool {
    UNREADABLE.lock().unwrap_or_else(std::sync::PoisonError::into_inner).iter().any(|p| p == path)
}

/// Where an operator would go to see what a file holds.
///
/// An alert that can only name the file is half an alert, and "check Settings →
/// Radio" is the wrong half for a memory list: nobody finds their channels
/// there. Each of these is the window that shows what has just gone missing.
fn where_to_look(file: &str) -> &'static str {
    match file {
        "memories.json" | "memory_folders.json" => "check the MEM window",
        "qso_log.json" => "check the LOG window",
        "swl_log.json" => "check the SWL LOG window",
        "contacts.json" => "check the FSQ contacts list",
        "scanner.json" => "check the SCAN window",
        "config.toml" => "check Settings",
        _ => "check Settings → Radio",
    }
}

/// What a load found on the disk.
enum FileText {
    /// Not there. A first run, where the defaults are the right answer and
    /// there is nothing to report.
    Missing,
    Text(String),
    /// There, and unreadable — see [`UNREADABLE`]. The defaults are what the
    /// caller has to run on, but they are *not* what the file says, and nothing
    /// may be written over it until it can be read.
    Unreadable,
}

/// Read a config file, telling the two kinds of failure apart.
fn read_config_text(dir: &Path, file: &str) -> FileText {
    let path = dir.join(file);
    match fs::read_to_string(&path) {
        Ok(text) => {
            clear_unreadable(&path);
            FileText::Text(text)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            clear_unreadable(&path);
            FileText::Missing
        }
        Err(e) => {
            warn!("cannot read {file}: {e}; running on defaults and refusing to overwrite it");
            if mark_unreadable(&path) {
                push_load_alert(format!(
                    "{file} is there but could not be read ({e}), so sdroxide has started on \
                     the defaults for it. Nothing will be written to it while that lasts, so \
                     what it holds is safe: close whatever else has the file open (a backup or \
                     antivirus tool, another copy of sdroxide) and start again. {}",
                    where_to_look(file)
                ));
            }
            FileText::Unreadable
        }
    }
}

/// A file was read but did not parse: keep it for inspection as `<file>.bak`
/// (so the evidence of *what* was lost survives the reset), warn, and queue an
/// operator-visible alert. The rename also stops the same complaint firing on
/// every subsequent load.
fn quarantine_unreadable(dir: &Path, file: &str, err: &dyn std::fmt::Display) {
    let kept = fs::rename(dir.join(file), dir.join(format!("{file}.bak"))).is_ok();
    // Whatever was there is now the `.bak` (or is gone), so there is no longer
    // anything under this name for a save to destroy.
    clear_unreadable(&dir.join(file));
    warn!("failed to parse {file}: {err}; resetting to defaults");
    let mut msg = format!("{file} was unreadable and has been reset to defaults");
    if kept {
        msg.push_str(&format!(" — the old file is kept as {file}.bak"));
    }
    msg.push_str(&format!("; {}", where_to_look(file)));
    push_load_alert(msg);
}

/// Replace `dir/file` by writing a sibling temp file and renaming it into
/// place. A crash or power cut mid-write then leaves the old file intact
/// instead of a truncated one — which the next load could only "reset to
/// defaults" from, forgetting the operator's whole setup.
fn write_atomic(dir: &Path, file: &str, text: &str) -> Result<(), ConfigError> {
    // A file this process could not *read* still holds whatever the operator
    // put in it, and what is about to be written is the defaults it was forced
    // to run on. Refuse: an error they can see beats a save that quietly
    // replaces a memory list with an empty one. See [`UNREADABLE`].
    if is_unreadable(&dir.join(file)) {
        return Err(ConfigError::Unreadable(file.to_string()));
    }
    fs::create_dir_all(dir)?;
    let tmp = dir.join(format!("{file}.tmp"));
    {
        use std::io::Write;
        let mut f = fs::File::create(&tmp)?;
        f.write_all(text.as_bytes())?;
        // The data must be on disk before the rename makes it the real file,
        // or a power cut could still promote an empty one.
        f.sync_all()?;
    }
    fs::rename(&tmp, dir.join(file))?;
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("no home/config directory available")]
    NoConfigDir,
    /// The file is there and could not be read, so it was not written either —
    /// see [`UNREADABLE`]. Never a reason to retry: it clears when a load
    /// finally reads the file.
    #[error(
        "{0} could not be read, so it has not been written over — close whatever else has it \
         open and start again"
    )]
    Unreadable(String),
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("serialize: {0}")]
    Serialize(#[from] toml::ser::Error),
}

/// User settings (`config.toml`). Everything has a default so a missing or
/// partial file always loads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// SoapySDR device args, e.g. "driver=hackrf". Empty = first device found.
    pub device_args: String,
    /// Preferred hardware sample rate in Hz.
    pub sample_rate: f64,
    /// dB offset applied to convert dBFS to dBm for the S-meter.
    pub cal_offset_db: f64,
    pub spectrum_fft: u32,
    pub spectrum_fps: u8,
    /// Server mode bind address.
    pub server_bind: String,
    pub server_port: u16,
    /// Refuse to transmit outside amateur bands.
    pub tx_ham_only: bool,
    /// Abort the transmission and latch out further transmit when the rig
    /// reports an SWR at or above [`Self::swr_limit`].
    ///
    /// Only rigs that actually measure SWR and report it over CAT/TCI can arm
    /// this; on anything that leaves `TxTelemetry::swr` as `None` it is inert,
    /// so it costs nothing to leave on.
    pub swr_guard: bool,
    /// The SWR ratio the guard trips at, e.g. `2.5` = 2.5:1.
    pub swr_limit: f32,
    /// Preferred audio output device name; `None` = system default.
    pub audio_output: Option<String>,
    /// Which speaker channel(s) the received audio reaches (#654). `Both` is
    /// the default and the old behaviour.
    pub audio_routing: sdroxide_types::AudioRouting,
    /// Preferred audio input (microphone) device name; `None` = system default.
    pub audio_input: Option<String>,
    /// The ITU / IARU region this station is in, which decides every band edge
    /// and sub-segment sdroxide draws and enforces.
    ///
    /// A property of the *station*, not of the screen: it is where the antenna
    /// is, so it belongs in `config.toml` next to `tx_ham_only` and travels to
    /// remote clients in the [`sdroxide_types::StationConfig`] bundle rather
    /// than being read off each client's own disk.
    ///
    /// Region 1 by default, which is the band plan every sdroxide before this
    /// setting had.
    pub region: sdroxide_types::Region,
    /// Which country's CB channels the station works. A plain value beside
    /// `region`, for the same reason: it belongs to the station and travels to
    /// clients in the [`sdroxide_types::StationConfig`] bundle.
    pub cb_plan: sdroxide_types::CbPlan,
    /// Allow transmit on the 11 m citizens' band.
    ///
    /// [`Self::tx_ham_only`] refuses every non-amateur band, and 11 m is one:
    /// it is a separate radio service with its own rules and its own
    /// type-approved equipment, not a free-for-all. This is the operator's
    /// deliberate opt-in, taken after acknowledging that — the interface makes
    /// them confirm it once. It opens 11 m and nothing else; the broadcast
    /// services stay receive-only. A station property like `cb_plan`, so it
    /// travels to remote clients in the [`sdroxide_types::StationConfig`]
    /// bundle. Off by default.
    pub cb_tx_allowed: bool,
    /// Where audio recordings and raw I/Q captures are written, or `None` for
    /// the default (the user's music folder, `<Music>/sdroxide`, or the config
    /// directory's `recordings` when the platform has no music folder).
    ///
    /// A plain path on **this** machine, so it is deliberately *not* a station
    /// property and does not travel to remote clients — unlike `region` and
    /// `cb_plan`, which describe where the antenna is. Set from the General
    /// tab's **Recordings** row; the engine reads it here when it opens a
    /// recording. Declared before the tables below because a TOML plain value
    /// after a table would be swallowed by it on the next write.
    pub recordings_dir: Option<std::path::PathBuf>,
    /// UI / display preferences (frame rate, waterfall + spectrum speed).
    pub ui: sdroxide_types::UiSettings,
    /// Username and password a remote client must present in server mode.
    /// Empty (the default) leaves the server open, exactly as it was before
    /// this existed.
    ///
    /// Last in the struct because TOML puts tables after values, and serde
    /// emits fields in declaration order: a table declared above a plain value
    /// would swallow that value into itself on the next write.
    pub remote_access: sdroxide_types::RemoteAccess,
    /// Spoken announcements. A client-side preference like `[ui]`: what the
    /// operator at this screen wants to hear, not how the station is set up.
    ///
    /// Also a table, so it goes after every plain value for the reason above.
    pub speech: sdroxide_types::SpeechSettings,
    /// Audible alerts. A client-side preference like `[speech]`: what the
    /// operator at this screen wants heard when a decode matters.
    ///
    /// A table for the same reason.
    pub alerts: sdroxide_types::AlertSettings,
    /// The sdroxide server this screen dials from Settings → Remote — the
    /// counterpart of `remote_access` above, and client-side like `[ui]` and
    /// `[speech]`: it is where *this* machine goes, not who may come here.
    ///
    /// A table again, and last for the same reason.
    pub remote_server: sdroxide_types::RemoteServer,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            device_args: String::new(),
            sample_rate: 1_536_000.0,
            cal_offset_db: 0.0,
            spectrum_fft: 4096,
            spectrum_fps: 30,
            server_bind: "0.0.0.0".into(),
            server_port: 4950,
            tx_ham_only: true,
            // On by default, and defensible: it cannot fire on a rig that does
            // not report SWR, and on one that does, the case it prevents is
            // transmitting into a fault. 2.5:1 is high enough not to trip on a
            // normally-tuned antenna near a band edge and low enough to catch a
            // disconnected feeder or a stuck relay.
            swr_guard: true,
            swr_limit: 2.5,
            audio_output: None,
            audio_routing: sdroxide_types::AudioRouting::default(),
            audio_input: None,
            region: sdroxide_types::Region::default(),
            cb_plan: sdroxide_types::CbPlan::default(),
            cb_tx_allowed: false,
            recordings_dir: None,
            ui: sdroxide_types::UiSettings::default(),
            remote_access: sdroxide_types::RemoteAccess::default(),
            speech: sdroxide_types::SpeechSettings::default(),
            alerts: sdroxide_types::AlertSettings::default(),
            remote_server: sdroxide_types::RemoteServer::default(),
        }
    }
}

/// Load just the UI/display preferences (frame rate, waterfall + spectrum speed).
pub fn load_ui_settings() -> sdroxide_types::UiSettings {
    Settings::load().ui
}

/// Persist the UI/display preferences, preserving every other setting
/// (read-modify-write so a concurrent edit elsewhere isn't clobbered).
pub fn save_ui_settings(ui: &sdroxide_types::UiSettings) -> Result<(), ConfigError> {
    let mut s = Settings::load();
    // `UiSettings` is no longer `Copy` — it carries the Classic UI section
    // order, which is a list.
    s.ui = ui.clone();
    s.save()
}

/// Load just the station's IARU region.
pub fn load_region() -> sdroxide_types::Region {
    Settings::load().region
}

/// Load just the station's CB channel plan.
pub fn load_cb_plan() -> sdroxide_types::CbPlan {
    Settings::load().cb_plan
}

/// Load just the station's 11 m transmit permission.
pub fn load_cb_tx_allowed() -> bool {
    Settings::load().cb_tx_allowed
}

/// Persist the station's 11 m transmit permission, preserving every other
/// setting. Like [`save_cb_plan`], it does not apply it — the caller does, with
/// [`sdroxide_types::set_cb_tx_allowed`].
pub fn save_cb_tx_allowed(allowed: bool) -> Result<(), ConfigError> {
    let mut s = Settings::load();
    s.cb_tx_allowed = allowed;
    s.save()
}

/// Persist the station's CB plan, preserving every other setting. Like
/// [`save_region`], it does not apply it — the caller does, with
/// [`sdroxide_types::set_cb_plan`].
pub fn save_cb_plan(plan: sdroxide_types::CbPlan) -> Result<(), ConfigError> {
    let mut s = Settings::load();
    s.cb_plan = plan;
    s.save()
}

/// Persist the station's IARU region, preserving every other setting
/// (read-modify-write, like [`save_ui_settings`]).
///
/// Does *not* apply it — the caller does that with
/// [`sdroxide_types::set_region`], because the two have different scopes: the
/// file is the station's, the process-wide setting is this process's.
pub fn save_region(region: sdroxide_types::Region) -> Result<(), ConfigError> {
    let mut s = Settings::load();
    s.region = region;
    s.save()
}

/// The band-plan file's name in the config directory.
pub const BAND_PLAN_FILE: &str = "bandplan.json";

/// Where `bandplan.json` lives, for the settings dialog to show the operator.
pub fn band_plan_path() -> Option<PathBuf> {
    config_dir().ok().map(|d| d.join(BAND_PLAN_FILE))
}

/// The station's band plan, seeded from the built-in IARU tables the first
/// time.
///
/// Three outcomes, and the differences matter:
///
/// - **No file** — write the built-in tables and return them. The operator now
///   has a complete, valid document to edit, which is a far better starting
///   point than an empty file and a manual page.
/// - **A file that parses** — return it, with any per-row complaints queued as
///   load alerts so a dropped row is visible rather than silently missing.
/// - **A file that does not parse** — warn, alert, and use the built-in tables
///   *for this run*, leaving the file exactly as it is.
///
/// That last case deliberately breaks with [`quarantine_unreadable`], which
/// renames the offending file aside and writes a fresh one. That is right for
/// the files sdroxide itself writes; this is a document the operator authors by
/// hand, and renaming a half-finished edit out from under them — even to
/// `.bak` — would be taking their work away at exactly the wrong moment. The
/// cost of leaving it is that the same complaint repeats on every start, which
/// is the correct amount of nagging for a file that is currently wrong.
pub fn load_band_plan() -> sdroxide_types::BandPlan {
    let Ok(dir) = config_dir() else { return sdroxide_types::BandPlan::default() };
    let path = dir.join(BAND_PLAN_FILE);
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let seed = sdroxide_types::BandPlan::default();
            match save_band_plan(&seed) {
                Ok(()) => info!("wrote a starting band plan to {}", path.display()),
                // Not fatal: the built-in tables are what it would have
                // contained, so the program runs on exactly the right plan and
                // only the editing convenience is lost.
                Err(e) => warn!("could not write {}: {e}", path.display()),
            }
            return seed;
        }
        Err(e) => {
            warn!("could not read {}: {e}; using the built-in band plan", path.display());
            return sdroxide_types::BandPlan::default();
        }
    };
    match serde_json::from_str::<sdroxide_types::BandPlan>(&text) {
        Ok(plan) => {
            for p in plan.problems() {
                warn!("{BAND_PLAN_FILE}: {p}");
                push_load_alert(format!("{BAND_PLAN_FILE}: {p}"));
            }
            plan
        }
        Err(e) => {
            warn!("failed to parse {BAND_PLAN_FILE}: {e}; using the built-in band plan");
            push_load_alert(format!(
                "{BAND_PLAN_FILE} could not be read ({e}) — running on the built-in IARU band \
                 plan. The file has been left alone; fix it and reload, or delete it to get a \
                 fresh one."
            ));
            sdroxide_types::BandPlan::default()
        }
    }
}

pub const RTL433_FLEX_FILE: &str = "rtl433_flex.conf";

/// Where `rtl433_flex.conf` lives, for the ISM window to show the operator.
pub fn rtl433_flex_path() -> Option<PathBuf> {
    config_dir().ok().map(|d| d.join(RTL433_FLEX_FILE))
}

/// The operator's own rtl_433 decoder specs, as raw text.
///
/// Same three outcomes and the same reasoning as [`load_band_plan`]: seeded with
/// a commented example the first time, never rewritten afterwards, and never
/// quarantined — this is a document somebody authors by hand, and moving a
/// half-finished edit aside would be taking their work away.
///
/// The text is returned unparsed. What a valid spec is belongs to the decoder
/// that has to survive an invalid one, not to the code that reads the file; see
/// `sdroxide_ism::rtl433::flex`.
pub fn load_rtl433_flex() -> String {
    let Ok(dir) = config_dir() else { return String::new() };
    let path = dir.join(RTL433_FLEX_FILE);
    match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            match write_atomic(&dir, RTL433_FLEX_FILE, RTL433_FLEX_SEED) {
                Ok(()) => info!("wrote a starting flex decoder file to {}", path.display()),
                Err(e) => warn!("could not write {}: {e}", path.display()),
            }
            RTL433_FLEX_SEED.to_string()
        }
        Err(e) => {
            warn!("could not read {}: {e}; no user flex decoders loaded", path.display());
            push_load_alert(format!(
                "{RTL433_FLEX_FILE} could not be read ({e}) — the built-in rtl_433 decoders still \
                 run, but none of your own."
            ));
            String::new()
        }
    }
}

/// What a fresh `rtl433_flex.conf` says.
///
/// Entirely commented out: a new file must add no decoders until the operator
/// asks for one. The example is a real, working spec so that uncommenting it is
/// a complete experiment rather than a template to fill in.
const RTL433_FLEX_SEED: &str = "\
# Your own rtl_433 decoders (\"flex\" specs) — one decoder per block.
#
# This is the same syntax as rtl_433's -X option and the \"decoder\" blocks in
# its own conf files, so a spec published by somebody else pastes in unchanged:
#   https://github.com/merbanan/rtl_433/tree/master/conf
#
# sdroxide reads this file when the ISM decoder starts. After editing it, press
# RELOAD DECODERS in the ISM window — the devices already heard stay in the list.
# This file is never rewritten, so anything you put here stays as you wrote it.
#
# Every spec is checked before it is handed to rtl_433, and one that does not
# pass is listed in the ISM window and skipped — the others still load. The
# check is deliberately stricter than rtl_433's own command line, because
# rtl_433 reports a bad spec by stopping the program, which is not something a
# receiver should do halfway through a contact.
#
# Example: a generic OOK doorbell. Remove the leading # from the block below to
# switch it on, then edit the timings to match your own device.
#
#decoder {
#    name=doorbell,
#    modulation=OOK_PWM,
#    short=400,
#    long=800,
#    gap=1000,
#    reset=7000,
#    match={24}0xa9878c,
#    get=@0:{24}:id,
#    unique
#}
";

/// Write the band plan, atomically like every other config file.
///
/// The only file here that does not go through [`save_json`]: the band plan
/// formats itself, because the units and the row-per-line layout are part of
/// what makes the file editable and belong with the type rather than with the
/// code that moves bytes. See [`sdroxide_types::BandPlan::to_json_document`].
///
/// Called in exactly one place — seeding a file that is not there yet — and it
/// should stay that way. An existing `bandplan.json` is the operator's
/// document; nothing in sdroxide rewrites it, so their spacing, their ordering
/// and anything they added to the `readme` all survive untouched.
pub fn save_band_plan(plan: &sdroxide_types::BandPlan) -> Result<(), ConfigError> {
    write_atomic(&config_dir()?, BAND_PLAN_FILE, &plan.to_json_document())
}

/// Load just the remote-access credentials.
///
/// Read fresh rather than cached: the server calls this once per connection, so
/// an edit to `config.toml` — by hand, or from the settings dialog of the GUI
/// running on the same machine — takes effect on the next sign-in instead of
/// waiting for the server to be restarted.
pub fn load_remote_access() -> sdroxide_types::RemoteAccess {
    Settings::load().remote_access
}

/// Persist the remote-access credentials, preserving every other setting
/// (read-modify-write, like [`save_ui_settings`]).
pub fn save_remote_access(access: &sdroxide_types::RemoteAccess) -> Result<(), ConfigError> {
    let mut s = Settings::load();
    s.remote_access = access.clone();
    s.save()
}

/// Load the server this screen last dialled from Settings → General.
pub fn load_remote_server() -> sdroxide_types::RemoteServer {
    Settings::load().remote_server
}

/// Persist the server address, preserving every other setting
/// (read-modify-write, like [`save_ui_settings`]).
pub fn save_remote_server(server: &sdroxide_types::RemoteServer) -> Result<(), ConfigError> {
    let mut s = Settings::load();
    s.remote_server = server.clone();
    s.save()
}

/// Load just the spoken-announcement preferences.
pub fn load_speech_settings() -> sdroxide_types::SpeechSettings {
    Settings::load().speech
}

/// Persist the spoken-announcement preferences, preserving every other setting
/// (read-modify-write, like [`save_ui_settings`]).
pub fn save_speech_settings(speech: &sdroxide_types::SpeechSettings) -> Result<(), ConfigError> {
    let mut s = Settings::load();
    s.speech = speech.clone();
    s.save()
}

/// Load just the audible-alert preferences.
pub fn load_alerts_settings() -> sdroxide_types::AlertSettings {
    Settings::load().alerts
}

/// Persist the audible-alert preferences, preserving every other setting
/// (read-modify-write, like [`save_ui_settings`]).
pub fn save_alerts_settings(alerts: &sdroxide_types::AlertSettings) -> Result<(), ConfigError> {
    let mut s = Settings::load();
    s.alerts = alerts.clone();
    s.save()
}

/// A sign-in the operator asked this client to remember (`remote_login.json`).
///
/// A *client*-side file, like `input.json`: it is what this machine types into
/// somebody else's server, not what this machine demands of anyone. Written
/// only when the sign-in dialog's "remember" box is ticked, and holding the
/// password in the clear — same as every other credential sdroxide stores, and
/// noted as such in the manual and in the dialog itself.
pub fn load_remote_login() -> Option<sdroxide_types::RemoteAccess> {
    let login: sdroxide_types::RemoteAccess = load_json("remote_login.json");
    login.is_enforced().then_some(login)
}

pub fn save_remote_login(login: Option<&sdroxide_types::RemoteAccess>) -> Result<(), ConfigError> {
    match login {
        Some(l) => save_json("remote_login.json", l),
        // Forgetting has to remove the file, not write an empty one: an empty
        // record and a deleted one mean the same thing, and leaving a password
        // field behind that says `""` invites the belief that something was
        // scrubbed when the old file is simply still there.
        None => {
            let path = config_dir()?.join("remote_login.json");
            match fs::remove_file(&path) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(e.into()),
            }
        }
    }
}

mod publicsdr;
pub use publicsdr::public_sdr_directory;

pub mod transfer;

/// The config directory **upstream** uses, and the one this fork used before it
/// had its own. Kept as a named function because the migration needs to find it
/// and several places still reason about "the shared directory".
fn upstream_config_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("org", "sdroxide", "sdroxide")
        .map(|d| d.config_dir().to_path_buf())
}

/// The config directory **this fork** uses: upstream's with a `-brown` suffix,
/// so the two builds never read or write each other's settings.
///
/// This is the fix for the one thing a full rename could not do cheaply — the
/// two installs sharing `config.toml` and `radio.json`, where upstream's
/// read-modify-write silently drops the fork's extra fields. It lives beside
/// upstream's rather than inside it, so a tester's two builds are completely
/// independent with nothing to configure.
fn brown_config_dir() -> Result<PathBuf, ConfigError> {
    let base = upstream_config_dir().ok_or(ConfigError::NoConfigDir)?;
    // `sdroxide` -> `sdroxide-brown`, and `org.sdroxide.sdroxide` ->
    // `org.sdroxide.sdroxide-brown` on macOS, where the last component is the
    // reverse-DNS id. Appending to the whole path rather than replacing a
    // component does the right thing on both.
    let name = base
        .file_name()
        .map(|n| format!("{}-brown", n.to_string_lossy()))
        .ok_or(ConfigError::NoConfigDir)?;
    Ok(base.with_file_name(name))
}

pub fn config_dir() -> Result<PathBuf, ConfigError> {
    // The override exists for the integration tests, which must not write the
    // operator's real configuration, and works as a profile switch for anyone
    // who wants two independent installations.
    if let Some(dir) = std::env::var_os("SDROXIDE_CONFIG_DIR") {
        if !dir.is_empty() {
            return Ok(PathBuf::from(dir));
        }
    }
    brown_config_dir()
}

/// Move an existing installation into the fork's own config directory, once.
///
/// Called at startup **before** anything reads a setting. The fork used to share
/// upstream's directory (`org.sdroxide.sdroxide`); this copies that directory's
/// contents into the `-brown` one so an existing user's stations, logbook,
/// memories and settings come with them, then leaves a marker so it never runs
/// again.
///
/// **Copy, not move.** The shared directory is left exactly as it was, so running
/// upstream afterwards still finds everything, and a half-finished copy cannot
/// destroy a single file. The cost is disk space once; the alternative is a
/// migration that can lose a logbook, which is not a trade worth making.
///
/// It does nothing when a custom `SDROXIDE_CONFIG_DIR` is set (that is already a
/// deliberate, separate profile) or when the fork's own directory already exists
/// (the migration has run, or this is a fresh install that made it).
pub fn migrate_shared_config_once() {
    if std::env::var_os("SDROXIDE_CONFIG_DIR").is_some_and(|v| !v.is_empty()) {
        return;
    }
    let Ok(brown) = brown_config_dir() else { return };
    let Some(shared) = upstream_config_dir() else { return };
    // Already migrated, or nothing to migrate from.
    if brown.exists() || !shared.is_dir() {
        return;
    }
    // Only adopt a directory that is actually sdroxide's: a `config.toml` or a
    // `radio.json` is the evidence, so an unrelated directory that happens to
    // sit at that path is not copied over.
    if !shared.join("config.toml").exists() && !shared.join("radio.json").exists() {
        return;
    }
    if let Err(e) = copy_dir_recursive(&shared, &brown) {
        // If the copy failed there is nothing safe to do but report it and let
        // the fork start on defaults: a partial directory would look migrated
        // and the next start would not retry.
        let _ = fs::remove_dir_all(&brown);
        eprintln!(
            "sdroxide: could not move your settings to {} ({e}); starting with defaults. \
             The original is untouched at {}.",
            brown.display(),
            shared.display()
        );
        return;
    }
    eprintln!(
        "sdroxide: copied your existing settings to {} so this build keeps its own. \
         Your original is untouched; other sdroxide builds are unaffected.",
        brown.display()
    );
}

/// Copy a directory tree, files and all. Plain and non-destructive: it never
/// removes or overwrites the source, and a failure is returned rather than
/// half-ignored.
fn copy_dir_recursive(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let src = entry.path();
        let dst = to.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_dir_recursive(&src, &dst)?;
        } else if kind.is_file() {
            fs::copy(&src, &dst)?;
        }
        // Symlinks and anything else are skipped on purpose: a config directory
        // has no legitimate symlink in it, and following one could copy
        // something outside the tree.
    }
    Ok(())
}

/// A configuration scope: the station's root config directory, or one radio's
/// subdirectory under it.
///
/// Multi-radio keeps **one file per radio scope** rather than one list file,
/// because the engine and the UI each re-read and re-write `radio.json`
/// independently — with a shared list every radio's "Apply" would be a
/// read-modify-write race against every other radio's. Radio 0 maps to the
/// legacy root paths, so an existing single-radio installation needs no
/// migration and stays downgrade-safe.
///
/// Only the files that describe *a radio* are scoped: `radio.json`,
/// `session.json`, `scanner.json`, `modeprofiles.json`, `tciserver.json`,
/// `rigctld.json`, `wsjtx.json`, `wspr.json`. Everything the operator shares
/// across radios — memories, band stacks, the logbook, `config.toml` — stays
/// on the root free functions.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Store {
    /// `None` = the legacy root (the station scope, and radio 0);
    /// `Some(n)` = `<config>/radio-<n>/`.
    scope: Option<u32>,
}

impl Store {
    /// The station scope: the legacy root directory.
    pub fn station() -> Store {
        Store { scope: None }
    }

    /// The scope for radio `id`. Id 0 is the legacy root — the radio every
    /// existing installation already has — so its files stay exactly where a
    /// single-radio sdroxide reads and writes them.
    pub fn radio(id: u32) -> Store {
        Store { scope: (id != 0).then_some(id) }
    }

    /// Which radio this scope belongs to, as the roster numbers it. The station
    /// scope answers 0, which is the radio it holds the files of.
    pub fn radio_id(&self) -> u32 {
        self.scope.unwrap_or(0)
    }

    /// This scope's directory. Not created here; writers create it on demand.
    pub fn dir(&self) -> Result<PathBuf, ConfigError> {
        let root = config_dir()?;
        Ok(match self.scope {
            None => root,
            Some(id) => root.join(format!("radio-{id}")),
        })
    }

    fn load<T: serde::de::DeserializeOwned + Default>(&self, file: &str) -> T {
        self.load_checked(file).0
    }

    /// [`Store::load`], plus whether the file had to be **quarantined**.
    ///
    /// That is a different thing from the file being absent, and for some files
    /// the difference decides what a safe fallback is. Absent means a first run
    /// and the defaults are the right answer; quarantined means the operator
    /// had a configuration and it has just been taken away from them, which is
    /// not a state to guess confidently in.
    fn load_checked<T: serde::de::DeserializeOwned + Default>(&self, file: &str) -> (T, bool) {
        let Ok(dir) = self.dir() else { return (T::default(), false) };
        let FileText::Text(text) = read_config_text(&dir, file) else {
            return (T::default(), false);
        };
        match serde_json::from_str(&text) {
            Ok(v) => (v, false),
            Err(e) => {
                quarantine_unreadable(&dir, file, &e);
                (T::default(), true)
            }
        }
    }

    fn save<T: serde::Serialize>(&self, file: &str, value: &T) -> Result<(), ConfigError> {
        let dir = self.dir()?;
        let text = serde_json::to_string_pretty(value).expect("serialize");
        write_atomic(&dir, file, &text)
    }

    /// Radio backend config, this scope's `radio.json`.
    ///
    /// A file that had to be quarantined comes back as the defaults with the
    /// interface set to [`Backend::None`] — open *nothing* — rather than the
    /// default backend.
    ///
    /// The difference is not cosmetic, and the reason is a field report. Serde
    /// fails a `RadioConfig` whole: one unreadable value anywhere in the file,
    /// from a setting for a rig that is not even connected, discards the
    /// operator's interface selection along with it. The default backend then
    /// goes looking for hardware and opens whatever it finds first — on that
    /// machine, a SoapySDR module that claims the receiver's bare USB
    /// controller id, which flooded the console and never started. The operator
    /// had selected a native driver and had no way to connect any of it to a
    /// config file that had been silently reset.
    ///
    /// `Backend::None` is exactly the right state to land in: it already means
    /// "opened nothing, waiting for the operator to choose", the settings panel
    /// already renders that as a page about choosing one, and the alert
    /// [`quarantine_unreadable`] queues already points there. Nothing is
    /// grabbed on the way.
    ///
    /// A *missing* file is untouched by this — that is a first run, where the
    /// defaults are the right answer.
    ///
    /// The fallback is **written back**, and that is the half that matters.
    /// Quarantine *renames* the unreadable file away, so without a re-seed the
    /// very next start finds no `radio.json` at all — indistinguishable from a
    /// first run — and goes straight back to the default backend. The operator
    /// in the report above hit that on every subsequent launch, not just the
    /// one where their config broke. Persisting the choice makes "nothing is
    /// selected" a state that survives a restart, which is the only way it is
    /// any use.
    pub fn load_radio_config(&self) -> sdroxide_types::RadioConfig {
        let (mut cfg, quarantined) = self.load_checked::<sdroxide_types::RadioConfig>("radio.json");
        if quarantined {
            warn!(
                "radio.json was reset, so no interface is selected — pick one in \
                 Settings → Radio rather than letting the defaults open something"
            );
            cfg.backend = sdroxide_types::Backend::None;
            // Best effort. If it cannot be written the program still runs on
            // the safe value for this session; only the memory of it is lost.
            if let Err(e) = self.save_radio_config(&cfg) {
                warn!("could not record the reset radio.json: {e}");
            }
        }
        cfg
    }

    pub fn save_radio_config(&self, cfg: &sdroxide_types::RadioConfig) -> Result<(), ConfigError> {
        self.save("radio.json", cfg)
    }

    /// The remembered dial and mode, or the defaults on a first run.
    pub fn load_session(&self) -> Session {
        self.load_session_if_present().unwrap_or_default()
    }

    /// The remembered session, or `None` when there is not one to restore —
    /// no `session.json`, or one that failed [`Session::is_usable`].
    ///
    /// The engine has to tell "a session was restored" from "this is a first
    /// run" apart. A mode's default profile is applied at startup only when
    /// nothing was restored: laying it over a restored session would reset the
    /// operator's saved AGC, squelch, noise reduction, binaural and RX gain to
    /// the mode's defaults, which is exactly what a station upgrading to a
    /// build with per-mode settings would hit, its `modeprofiles.json` still
    /// empty.
    ///
    /// The file has to be *there*: [`Session::default`] is itself usable, so
    /// `load` alone cannot tell a first run from a session that was saved.
    /// `load_session` keeps returning a default either way, so an engine still
    /// starts remembering from its first change.
    pub fn load_session_if_present(&self) -> Option<Session> {
        let dir = self.dir().ok()?;
        if !matches!(read_config_text(&dir, "session.json"), FileText::Text(_)) {
            return None;
        }
        let s: Session = self.load("session.json");
        if s.is_usable() { Some(s.sanitized()) } else { None }
    }

    pub fn save_session(&self, session: &Session) -> Result<(), ConfigError> {
        self.save("session.json", session)
    }

    /// This station's per-mode settings overrides, or none on a first run.
    pub fn load_mode_profiles(&self) -> sdroxide_types::ModeProfiles {
        self.load("modeprofiles.json")
    }

    pub fn save_mode_profiles(
        &self,
        profiles: &sdroxide_types::ModeProfiles,
    ) -> Result<(), ConfigError> {
        self.save("modeprofiles.json", profiles)
    }

    /// The operator's digital-mode config as this radio runs it: the shared
    /// `digi.json` with this radio's own WSPR beacon settings laid over it
    /// (issue #615).
    ///
    /// A radio that has never saved its own gets the shared file's — radio 0,
    /// whose settings those are on an installation that predates the split —
    /// except that any other radio starts with its beacon off. Transmitting
    /// because a radio was added is not something the operator asked for.
    pub fn load_digi_config(&self) -> sdroxide_types::DigiConfig {
        let mut cfg = load_digi_config();
        match self.load_wspr_config() {
            Some(w) => w.apply_to(&mut cfg),
            None if self.radio_id() != 0 => cfg.wspr_tx_percent = 0,
            None => {}
        }
        cfg
    }

    /// Save `cfg` as this radio's: its WSPR beacon settings to its own
    /// `wspr.json`, the rest to the shared `digi.json`.
    ///
    /// Radio 0 also leaves its WSPR settings in `digi.json`, where an older
    /// build reads them. Any other radio writes the shared file with the WSPR
    /// settings it already holds, so radio 0 — which may still be reading them
    /// from there — is not handed another radio's beacon.
    pub fn save_digi_config(&self, cfg: &sdroxide_types::DigiConfig) -> Result<(), ConfigError> {
        let own = sdroxide_types::WsprRadio::of(cfg);
        if self.radio_id() == 0 {
            save_digi_config(cfg)?;
        } else {
            let mut shared = cfg.clone();
            sdroxide_types::WsprRadio::of(&load_digi_config()).apply_to(&mut shared);
            save_digi_config(&shared)?;
        }
        self.save("wspr.json", &own)
    }

    /// This radio's WSPR beacon settings, or `None` where it has never saved
    /// any. An unreadable file is quarantined and reads as the defaults —
    /// beacon off — rather than as absent.
    pub fn load_wspr_config(&self) -> Option<sdroxide_types::WsprRadio> {
        let dir = self.dir().ok()?;
        match read_config_text(&dir, "wspr.json") {
            FileText::Missing => None,
            _ => Some(self.load("wspr.json")),
        }
    }

    /// This radio's DAB settings: its channels, stations and presets.
    ///
    /// Each radio keeps its own, because which blocks it hears is a fact about
    /// its antenna and its front end, and an × on one radio's channel must not
    /// take it off another's. They used to share the station's `dab.json`, so a
    /// radio other than the first that has never saved one starts from that —
    /// the list it was showing — and is its own from its first save.
    pub fn load_dab_config(&self) -> sdroxide_types::DabSettings {
        if self.scope.is_some()
            && let Ok(dir) = self.dir()
            && matches!(read_config_text(&dir, "dab.json"), FileText::Missing)
        {
            return Store::station().load("dab.json");
        }
        self.load("dab.json")
    }

    pub fn save_dab_config(&self, cfg: &sdroxide_types::DabSettings) -> Result<(), ConfigError> {
        self.save("dab.json", cfg)
    }

    pub fn load_scanner_config(&self) -> sdroxide_types::ScannerConfig {
        self.load("scanner.json")
    }

    pub fn save_scanner_config(
        &self,
        cfg: &sdroxide_types::ScannerConfig,
    ) -> Result<(), ConfigError> {
        self.save("scanner.json", cfg)
    }

    pub fn load_tci_server_config(&self) -> sdroxide_types::TciServerConfig {
        self.load("tciserver.json")
    }

    pub fn save_tci_server_config(
        &self,
        cfg: &sdroxide_types::TciServerConfig,
    ) -> Result<(), ConfigError> {
        self.save("tciserver.json", cfg)
    }

    pub fn load_rigctld_config(&self) -> sdroxide_types::RigctldConfig {
        self.load("rigctld.json")
    }

    pub fn save_rigctld_config(
        &self,
        cfg: &sdroxide_types::RigctldConfig,
    ) -> Result<(), ConfigError> {
        self.save("rigctld.json", cfg)
    }

    pub fn load_wsjtx_config(&self) -> sdroxide_types::WsjtxConfig {
        self.load("wsjtx.json")
    }

    pub fn save_wsjtx_config(&self, cfg: &sdroxide_types::WsjtxConfig) -> Result<(), ConfigError> {
        self.save("wsjtx.json", cfg)
    }
}

/// One radio in the station (`radios.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RadioSlot {
    pub id: u32,
    /// The operator's own name for this radio. Empty — the default — means
    /// "name it after whatever interface it runs", so an unrenamed tab follows
    /// the radio it is configured as instead of going stale on a label chosen
    /// before the radio was even picked. The UI resolves the display name;
    /// this file only records what the operator typed.
    pub name: String,
    /// Whether this radio is switched on: false and nothing opens its
    /// interface — no device claimed, no CAT port held, no network rig dialled
    /// — while everything it is configured as stays exactly where it is. The
    /// rig that is boxed for the summer, or the dongle somebody else has
    /// borrowed, stops being a tab that reconnects for ever without having to
    /// be deleted and set up again.
    ///
    /// Defaults to true, which is what every roster written before this
    /// existed means.
    #[serde(default = "yes")]
    pub enabled: bool,
}

/// `serde(default)` for [`RadioSlot::enabled`]: a radio nobody said anything
/// about is on.
fn yes() -> bool {
    true
}

/// The station's radio roster (`radios.json`). A missing file means what every
/// installation before multi-radio meant: exactly the one radio at the legacy
/// root paths.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RadiosFile {
    pub radios: Vec<RadioSlot>,
    pub next_id: u32,
    /// The operator's own left-to-right order for the tab strip, by radio id
    /// (issue #224). Empty — the default — means "the order they were created
    /// in", which is what [`RadiosFile::radios`] is already in.
    ///
    /// Deliberately *not* the order of `radios` itself. That list is the
    /// station's identity: its first entry is the radio that holds the shared
    /// network services, takes the command line's overrides and the legacy
    /// configuration paths, and cannot be closed — here and, over the wire, at
    /// every client. Rearranging the strip is a display choice and must not
    /// hand any of that to a different radio, so the two are kept apart.
    ///
    /// Read through [`RadiosFile::display_order`], never raw: a hand-edited or
    /// stale list may name a radio that has been closed or leave out one that
    /// has been added.
    #[serde(default)]
    pub order: Vec<u32>,
}

impl Default for RadiosFile {
    fn default() -> Self {
        RadiosFile {
            radios: vec![RadioSlot { id: 0, name: String::new(), enabled: true }],
            next_id: 1,
            order: Vec::new(),
        }
    }
}

pub fn load_radios() -> RadiosFile {
    let mut file: RadiosFile = load_json("radios.json");
    if file.radios.is_empty() {
        // An empty roster cannot be operated: seed the legacy radio, exactly
        // as a missing file would.
        file.radios = RadiosFile::default().radios;
    }
    // A hand-edited file must not be able to hand out an id twice.
    let max = file.radios.iter().map(|r| r.id).max().unwrap_or(0);
    file.next_id = file.next_id.max(max + 1);
    file
}

pub fn save_radios(file: &RadiosFile) -> Result<(), ConfigError> {
    save_json("radios.json", file)
}

impl RadiosFile {
    /// Whether radio `id` is switched on. A radio the roster has never heard
    /// of counts as on: the callers that ask are the ones that open interfaces,
    /// and refusing to open a radio because it is missing from a file would
    /// turn a lost roster into a station with no radios.
    pub fn is_enabled(&self, id: u32) -> bool {
        self.radios.iter().find(|r| r.id == id).is_none_or(|r| r.enabled)
    }

    /// Every radio in the roster, in the order the tab strip should show them:
    /// [`RadiosFile::order`] first, then anything it does not name, in the
    /// order the roster has it.
    ///
    /// Always a permutation of the roster, whatever is in the file. A radio
    /// named in `order` that is no longer in the roster is dropped, and one the
    /// roster has gained since — added from another client, or by hand — lands
    /// at the end rather than disappearing off the strip.
    pub fn display_order(&self) -> Vec<u32> {
        let mut out: Vec<u32> = self
            .order
            .iter()
            .copied()
            .filter(|id| self.radios.iter().any(|r| r.id == *id))
            .collect();
        out.dedup();
        for r in &self.radios {
            if !out.contains(&r.id) {
                out.push(r.id);
            }
        }
        out
    }
}

/// Create the on-disk scope for a new radio and add it to the roster.
///
/// The scope is seeded with a default `radio.json` and — deliberately — a
/// `tciserver.json` with the server disabled: [`TciServerConfig`]'s default is
/// *enabled* on a fixed port, which is right for the station's first radio and
/// a guaranteed bind collision for every additional one.
pub fn create_radio(name: &str) -> Result<RadioSlot, ConfigError> {
    let mut roster = load_radios();
    let id = roster.next_id;
    // An empty name is the default, not a placeholder to fill: it means the
    // tab names itself after whatever interface the radio ends up configured
    // as (see [`RadioSlot::name`]).
    let slot = RadioSlot { id, name: name.to_string(), enabled: true };
    let store = Store::radio(id);
    // Seed with *no* interface: the defaults would open the first device
    // found, which is whatever the station's first radio already holds.
    let cfg = sdroxide_types::RadioConfig {
        backend: sdroxide_types::Backend::None,
        ..Default::default()
    };
    store.save_radio_config(&cfg)?;
    let tci = sdroxide_types::TciServerConfig { enabled: false, ..Default::default() };
    store.save_tci_server_config(&tci)?;
    roster.radios.push(slot.clone());
    roster.next_id = id + 1;
    save_radios(&roster)?;
    Ok(slot)
}

/// Remove a radio from the roster. Its scope directory is kept on disk — a
/// closed tab is not a request to destroy the configuration behind it.
pub fn remove_radio(id: u32) -> Result<(), ConfigError> {
    let mut roster = load_radios();
    roster.radios.retain(|r| r.id != id);
    save_radios(&roster)
}

/// Switch a radio on or off (see [`RadioSlot::enabled`]).
///
/// The roster is the authority the interface factories read, so this is the
/// whole of the change: the engine is then asked to rebuild its front end and
/// finds out from here whether it is opening a radio or standing down.
pub fn set_radio_enabled(id: u32, enabled: bool) -> Result<(), ConfigError> {
    let mut roster = load_radios();
    if let Some(slot) = roster.radios.iter_mut().find(|r| r.id == id) {
        slot.enabled = enabled;
    }
    save_radios(&roster)
}

/// Record the operator's left-to-right order for the tab strip (issue #224).
///
/// Only the order is written — the roster itself is untouched, so the station's
/// first radio stays the station's first radio however the strip is arranged.
/// Ids that are not in the roster are dropped rather than stored, so a stale
/// list cannot grow in the file.
pub fn reorder_radios(ids: &[u32]) -> Result<(), ConfigError> {
    let mut roster = load_radios();
    roster.order =
        ids.iter().copied().filter(|id| roster.radios.iter().any(|r| r.id == *id)).collect();
    save_radios(&roster)
}

/// Record the operator's name for a radio. Empty puts it back on the default
/// — named after its interface (see [`RadioSlot::name`]).
pub fn rename_radio(id: u32, name: &str) -> Result<(), ConfigError> {
    let mut roster = load_radios();
    if let Some(slot) = roster.radios.iter_mut().find(|r| r.id == id) {
        slot.name = name.to_string();
    }
    save_radios(&roster)
}

/// Directory for a mode's received pictures (`~/.config/sdroxide/<kind>_rx`),
/// created on demand.
///
/// One store per mode rather than one for everything: an SSTV picture and a
/// weather chart are browsed for entirely different reasons, and a
/// fifteen-minute chart arriving every half hour would bury a session's SSTV.
pub fn image_rx_dir(kind: &str) -> Result<PathBuf, ConfigError> {
    // The caller's `kind` is a literal today, but it ends up in a path.
    let safe: String = kind.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
    let dir = config_dir()?.join(format!("{}_rx", if safe.is_empty() { "image" } else { &safe }));
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Directory for received weather-fax charts, created on demand: the user's
/// pictures directory (`<Pictures>/sdroxide/wefax`), or the config directory
/// (`~/.config/sdroxide/wefax_rx`) when the platform exposes no pictures folder.
///
/// Charts go where pictures go, unlike every other store here, because that is
/// what they are for. A weather chart is printed, mailed, dropped into a
/// passage plan or opened next to a routing program — all of which happen
/// outside this program, in a file manager, and none of which anyone will do
/// from a hidden directory under `~/.config`.
pub fn wefax_rx_dir() -> Result<PathBuf, ConfigError> {
    let dir = match directories::UserDirs::new()
        .and_then(|u| u.picture_dir().map(std::path::Path::to_path_buf))
    {
        Some(pictures) => pictures.join("sdroxide").join("wefax"),
        None => config_dir()?.join("wefax_rx"),
    };
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Where charts were kept before they moved to the pictures directory.
///
/// Read-only and never created: the gallery lists it alongside the current
/// store so an existing collection does not appear to have been lost. `None`
/// when it is the current store anyway, or when there is no config directory.
pub fn wefax_legacy_rx_dir() -> Option<PathBuf> {
    let old = config_dir().ok()?.join("wefax_rx");
    let current = wefax_rx_dir().ok()?;
    (old != current && old.is_dir()).then_some(old)
}

/// Directory for the operator's transmit-image slots
/// (`~/.config/sdroxide/sstv_tx`), created on demand.
pub fn sstv_tx_dir() -> Result<PathBuf, ConfigError> {
    let dir = config_dir()?.join("sstv_tx");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Directory for the voice keyer's recorded messages
/// (`~/.config/sdroxide/voice`), created on demand. One 48 kHz mono WAV per
/// slot, so a message can be edited or replaced with any audio editor.
pub fn voice_dir() -> Result<PathBuf, ConfigError> {
    let dir = config_dir()?.join("voice");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Directory for operator-supplied speech voices
/// (`~/.config/sdroxide/speech_voices`), created on demand. One Piper voice per
/// `.onnx` + `.onnx.json` pair.
///
/// Deliberately not [`voice_dir`], which is the voice *keyer*'s recordings —
/// two unrelated meanings of the word that would be a nasty surprise to share
/// a directory.
pub fn speech_voice_dir() -> Result<PathBuf, ConfigError> {
    let dir = config_dir()?.join("speech_voices");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Directory for cached solar imagery and space-weather JSON
/// (`~/.config/sdroxide/solar`), created on demand.
///
/// The 3D solar view loads this before its first network request, so the window
/// opens with the last-known data and stays useful with no connection at all.
pub fn solar_cache_dir() -> Result<PathBuf, ConfigError> {
    let dir = config_dir()?.join("solar");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// The recordings directory, resolved but **not created**: what the General
/// tab shows and what [`recordings_dir`] then makes.
///
/// `Settings::recordings_dir` wins when it is set; otherwise the user's
/// music/audio directory (`<Music>/sdroxide`), or the config directory
/// (`~/.config/sdroxide/recordings`) when the platform exposes no music folder.
pub fn recordings_dir_path() -> Result<PathBuf, ConfigError> {
    // An isolated configuration is isolated: `SDROXIDE_CONFIG_DIR` is how a
    // test, a second station or a throwaway session says "keep everything
    // here", and a recording written into the operator's real music folder
    // from one of those is the one file that escapes. It also wins over the
    // setting, so an isolated session cannot be pointed out of its sandbox.
    // Checked here rather than inside `config_dir` because the two answer
    // different questions — that one is where settings live, and it has an
    // isolated form already.
    if std::env::var_os("SDROXIDE_CONFIG_DIR").is_some() {
        return Ok(config_dir()?.join("recordings"));
    }
    let music =
        directories::UserDirs::new().and_then(|u| u.audio_dir().map(std::path::Path::to_path_buf));
    pick_recordings_dir(Settings::load().recordings_dir.as_deref(), music)
}

/// The choice itself, split out so it can be tested without a config directory
/// or the environment: the setting if one is set and non-empty, else the music
/// folder's `sdroxide`, else the config directory's `recordings`.
fn pick_recordings_dir(
    custom: Option<&std::path::Path>,
    music: Option<PathBuf>,
) -> Result<PathBuf, ConfigError> {
    if let Some(c) = custom.filter(|p| !p.as_os_str().is_empty()) {
        return Ok(c.to_path_buf());
    }
    match music {
        Some(music) => Ok(music.join("sdroxide")),
        None => Ok(config_dir()?.join("recordings")),
    }
}

/// Directory for audio recordings, created on demand (see
/// [`recordings_dir_path`] for where).
pub fn recordings_dir() -> Result<PathBuf, ConfigError> {
    let dir = recordings_dir_path()?;
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Persist the recordings directory (`None` restores the default), preserving
/// every other setting — read-modify-write, like [`save_ui_settings`].
pub fn save_recordings_dir(dir: Option<&std::path::Path>) -> Result<(), ConfigError> {
    let mut s = Settings::load();
    s.recordings_dir = dir.map(std::path::Path::to_path_buf);
    s.save()
}

impl Settings {
    /// Load settings; missing file or unreadable content falls back to
    /// defaults (with a warning), so startup never fails on config.
    pub fn load() -> Settings {
        let path = match config_dir() {
            Ok(d) => d.join("config.toml"),
            Err(e) => {
                warn!("no config dir: {e}; using default settings");
                return Settings::default();
            }
        };
        let Some(dir) = path.parent() else { return Settings::default() };
        let FileText::Text(text) = read_config_text(dir, "config.toml") else {
            return Settings::default();
        };
        match toml::from_str(&text) {
            Ok(s) => s,
            Err(e) => {
                quarantine_unreadable(dir, "config.toml", &e);
                Settings::default()
            }
        }
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        let dir = config_dir()?;
        let text = toml::to_string_pretty(self)?;
        write_atomic(&dir, "config.toml", &text)
    }
}

/// Where the operator left the radio (`session.json`): the dial, the mode, the
/// selected antennas, the audio/RF levels, and the receive settings that are
/// set by ear (squelch, noise reduction, the front end's own gain stages).
/// Restored on the next start, so the program comes back up where it was
/// instead of on a fixed default frequency and whichever port and level the
/// driver happens to power up on.
///
/// Deliberately not part of `config.toml`. That file holds preferences the
/// operator sets once; this is written by the engine as the radio is used, and
/// the command line still wins over it (`--freq`, `--mode`, `--antenna`,
/// `--tx-antenna`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Session {
    /// Dial frequency of VFO A, in Hz.
    pub freq_hz: f64,
    /// Dial frequency of VFO B, in Hz. `None` in a session written before the
    /// second VFO was remembered, and on a first run — where B comes up on A's
    /// frequency, exactly as a radio that has never had its B set does.
    pub vfo_b_hz: Option<f64>,
    /// Which of the two the operator was working on. Restored along with both
    /// dials, so a station left listening on B comes back listening on B rather
    /// than silently on A's frequency.
    pub active_vfo: sdroxide_types::Vfo,
    /// Mode of the main receiver — which is the mode of whichever VFO was
    /// active, and the one `--mode` overrides.
    pub mode: sdroxide_types::Mode,
    /// The mode each VFO was left in, `[A, B]`.
    ///
    /// A VFO is a whole listening position and not just a dial: CW on A while B
    /// sits on an SSB net is what the pair is for (issue #286), and a station
    /// that came back with both of them in yesterday's active mode had lost
    /// half of its setup. `None` in a session written before this was
    /// remembered, and on a first run — both then come up in
    /// [`Self::mode`], which is where they used to be anyway.
    pub vfo_modes: Option<[sdroxide_types::Mode; 2]>,
    /// RX antenna port, as the device names it ("LNAH", "TX/RX"). `None` on a
    /// front end that has no antenna to choose, and on every session written
    /// before this was remembered.
    pub antenna_rx: Option<String>,
    /// TX antenna port, likewise ("BAND1", "BAND2").
    pub antenna_tx: Option<String>,
    /// Main receiver's AF volume, 0.0..=1.0.
    pub volume: f32,
    /// Whether the main receiver was left muted — the MUTE button and the
    /// radio strip's mute chip, which are one state. Remembered so a radio
    /// muted on purpose comes back quiet instead of opening at full volume.
    pub muted: bool,
    /// Main receiver's manual (AGC-off) gain, in dB.
    pub rx_gain_db: f32,
    /// Main receiver's AGC mode.
    pub agc: sdroxide_types::AgcMode,
    /// TX drive, 0.0..=1.0 fraction of maximum.
    pub drive: f32,
    /// Drive used while tuning, 0.0..=1.0 fraction of maximum.
    pub tune_drive: f32,
    /// Mic gain, 0.0..=1.0.
    pub mic_gain: f32,
    /// Controlled-envelope SSB compression, in decibels; 0 is off.
    ///
    /// Here with the mic gain and the EQ rather than in `config.toml`, because
    /// it is the same kind of setting: something set by ear at the radio, per
    /// radio, and expected to still be there next time.
    #[serde(default)]
    pub cessb_db: f32,
    /// Transmit parametric EQ (voice modes only). Absent in a session written
    /// before this existed, in which case `#[serde(default)]` above gives it
    /// [`sdroxide_types::TxEqState::default`], disabled and flat.
    pub tx_eq: sdroxide_types::TxEqState,
    /// Main receiver's squelch threshold, in dBFS.
    /// [`sdroxide_types::SQUELCH_OPEN_DB`] = always open.
    pub squelch_db: f32,
    /// Main receiver's noise reduction (engine + strength, or off).
    pub noise_reduction: sdroxide_types::NrLevel,
    /// Whether binaural (pseudo-stereo) audio was left switched on. Set by ear
    /// like the noise reduction above it, and remembered for the same reason:
    /// an operator who listens this way listens to *everything* this way, and
    /// should not have to switch it back on every start.
    pub binaural: bool,
    /// How far the raw IQ was being decimated (a power of two; 1 is off).
    ///
    /// Kept per radio like everything else in this file, because it is a
    /// property of what the operator wants to *see* on a given front end: a
    /// dongle streaming 2.4 Msps to watch 20 m SSB on is left decimated, and
    /// the same station's 192 kHz IQ rig is not. The engine re-clamps it to
    /// what the device it opens can carry, so a file written against a wideband
    /// front end cannot leave a narrow one with no span at all.
    pub decimation: u32,
    /// Front-end RX gain stages the operator has set, as `(element, dB)` —
    /// the sliders on the Radio tab's device panel.
    ///
    /// Only the elements that have actually been moved are listed, and an
    /// empty list means "no preference": a device is opened on its driver's
    /// own gains, and a front end nobody has touched must keep coming up on
    /// those rather than on a figure this file invented for it. Same reasoning
    /// as [`Self::antenna_rx`], and the engine checks the names against the
    /// device the same way before applying any of them.
    pub gains: Vec<(String, f64)>,
    /// Front-end TX gain stages, likewise.
    pub tx_gains: Vec<(String, f64)>,
    /// Working a repeater: the transmit shift, the sub-audible tone under the
    /// voice and the 1750 Hz burst.
    ///
    /// Here rather than in `config.toml` for the same reason the dial and the
    /// squelch are: it is set at the radio as it is used, and a station left
    /// on the local repeater should come back on it rather than transmitting
    /// simplex into the input. Absent in a session written before this
    /// existed, in which case `#[serde(default)]` on the struct gives the
    /// plain simplex, no-tone setting every start used to have.
    pub repeater: sdroxide_types::RepeaterState,

    /// The last centre frequency this radio actually **tuned**, in Hz; `0.0` for
    /// none, which is what every `session.json` written before this loads as.
    ///
    /// **A start frequency the hardware cannot reach means the radio never
    /// opens** (#72): the reported case is an I.F. offset saved as a frequency,
    /// so an RTL-SDR was asked for 6 GHz, refused, and the operator had to
    /// correct the number by hand *every* time before anything worked.
    ///
    /// It lives here rather than in `radio.json` because this is where the dial
    /// itself is remembered — the start frequency *is* `freq_hz` above, so the
    /// value it falls back to belongs beside it. Not on the wire, so no
    /// `PROTO_VERSION` change.
    #[serde(default)]
    pub last_good_vfo_hz: f64,
    /// Whether a recording mixes RX/TX down to one channel instead of putting
    /// RX left and TX right. A preference the operator sets once and expects to
    /// still hold next time, not something the engine moves on its own — but it
    /// rides here rather than in `config.toml` because the UI is the only thing
    /// that sets it, and the engine is what owns writing it back.
    pub recording_mono: bool,
    /// The format a recording of the receiver audio is written in, and the
    /// operator's standing choice of it — see
    /// [`sdroxide_types::RecordingFormat`]. Rides here beside
    /// [`Self::recording_mono`] for the same reason: the UI sets it, the engine
    /// owns writing it back, and it is a preference rather than a property of
    /// the radio. Absent in a session written before this existed, in which case
    /// `#[serde(default)]` on the struct gives it the MP3 the station used to
    /// write.
    pub recording_format: sdroxide_types::RecordingFormat,
    /// The antenna sockets the operator last chose **on each band**, as
    /// `(RX, TX)`.
    ///
    /// A station with more than one antenna does not have one preference, it
    /// has one per band: the beam on 2 m, the vertical on 40, the Hi-Z port on
    /// 160. Radios with an antenna selector remember it that way themselves —
    /// an Icom's band stacking register holds the socket beside the frequency
    /// and the mode — and an operator who has to reach for the switch on every
    /// band change has an antenna selector that is not doing its job (issues
    /// #235 and #238).
    ///
    /// Only bands the operator has actually chosen a socket on appear here, so
    /// a band never worked opens on whatever the front end is already set to.
    /// Absent in a session written before this existed.
    #[serde(default)]
    pub band_antenna: BandAntennas,
    /// The antenna sockets each VFO was left on, `[A, B]`, as `(RX, TX)`.
    ///
    /// The companion to [`Self::vfo_modes`], and there for the same reason: a
    /// VFO is a whole listening position, not a dial. [`Self::band_antenna`]
    /// cannot stand in for this — it holds one socket per band, so two VFOs at
    /// opposite ends of the same band write the same entry and the second
    /// choice wins for both (issue #404).
    ///
    /// Either side may be `None`, for a VFO that has never had a socket chosen
    /// on it or a direction with no port to choose; the whole field is absent
    /// in a session written before this was remembered. No preference means no
    /// assertion, the same rule the two memories above follow.
    #[serde(default)]
    pub vfo_antennas: Option<[(Option<String>, Option<String>); 2]>,
    /// Remember a separate front-end gain per band, and recall it on a band
    /// change. **Off by default** — see [`Self::band_gains`].
    ///
    /// Opt-in because moving the gain on every band change is a surprise an
    /// operator who never asked for it would meet as a radio that suddenly
    /// hears differently; the switch is the operator choosing that behaviour,
    /// not this code choosing it for them.
    #[serde(default)]
    pub gain_by_band: bool,
    /// The gain stages remembered for each band — `(element, dB)` pairs with
    /// the same shape [`Self::gains`] holds, but keyed by band.
    ///
    /// A receiver's right gain is a property of where the dial is: an RTL-SDR
    /// that needs 10 dB on 40 m to stay out of its own noise will overload on
    /// 10 dB at 70 cm and wants 40 there. One number for the whole front end
    /// cannot serve both, and the operator was setting it by hand on every QSY.
    /// The 11 m band is a band like any other, so a CB operator's 11 m set keeps
    /// its own entry beside the ham bands'.
    ///
    /// Empty, and unread, unless [`Self::gain_by_band`] is on. Absent in a
    /// session written before this existed.
    #[serde(default)]
    pub band_gains: std::collections::HashMap<sdroxide_types::Band, Vec<(String, f64)>>,
}

/// Which antenna socket was last chosen on each band — see
/// [`Session::band_antenna`]. `(RX, TX)`; either may be `None` where that
/// direction has no port to choose.
pub type BandAntennas =
    std::collections::HashMap<sdroxide_types::Band, (Option<String>, Option<String>)>;

impl Default for Session {
    fn default() -> Self {
        // The 20 m band-stack default — exactly where the program started every
        // time before it remembered anything.
        let (freq_hz, mode) = sdroxide_types::Band::M20.default_entry();
        // No antenna preference: whatever the driver selects on open stands,
        // which is what every start did before this was remembered.
        // Levels match `RadioState::default()` so an operator who has never
        // touched them comes up at the same drive and mic gain a fresh start
        // always used, rather than a dead mic.
        let radio = sdroxide_types::RadioState::default();
        Session {
            freq_hz,
            // Nothing tuned yet on a first run; the fallback only ever fires
            // once this front end has opened somewhere.
            last_good_vfo_hz: 0.0,
            // No second dial of its own until one has been used: B mirrors A.
            vfo_b_hz: None,
            active_vfo: sdroxide_types::Vfo::A,
            mode,
            // No mode of its own for either VFO until one has been used, for
            // the same reason B has no dial of its own above.
            vfo_modes: None,
            antenna_rx: None,
            antenna_tx: None,
            volume: radio.rx[0].volume,
            muted: radio.rx[0].muted,
            rx_gain_db: radio.rx[0].manual_gain_db,
            agc: radio.rx[0].agc,
            drive: radio.tx.drive,
            tune_drive: radio.tx.tune_drive,
            mic_gain: radio.tx.mic_gain,
            cessb_db: radio.tx.cessb_db,
            tx_eq: radio.tx.eq,
            squelch_db: radio.rx[0].squelch_db,
            noise_reduction: radio.rx[0].noise_reduction,
            binaural: radio.rx[0].binaural,
            decimation: radio.decimation,
            repeater: radio.repeater,
            gains: Vec::new(),
            tx_gains: Vec::new(),
            recording_mono: radio.recording_mono,
            recording_format: radio.recording_format,
            band_antenna: BandAntennas::new(),
            // And no socket of its own for either VFO until one has been
            // chosen on it — nothing moves a relay before the operator has
            // said what belongs where.
            vfo_antennas: None,
            // No per-band gain memory until the operator turns it on.
            gain_by_band: false,
            band_gains: std::collections::HashMap::new(),
        }
    }
}

impl Session {
    /// Whether this record can be restored.
    ///
    /// The frequency is handed straight to a front end as its centre, so a
    /// hand-edited or truncated file must not be able to open the receiver on
    /// 0 Hz or NaN — a far worse failure than a forgotten session.
    fn is_usable(&self) -> bool {
        Self::usable_dial(self.freq_hz)
    }

    fn usable_dial(hz: f64) -> bool {
        hz.is_finite() && hz > 0.0
    }

    /// Drop a VFO B that a hand edit left unusable, rather than the whole
    /// record: A is what the receiver opens on and is checked by
    /// [`Self::is_usable`], so a nonsense B costs only the second dial.
    fn sanitized(mut self) -> Session {
        self.vfo_b_hz = self.vfo_b_hz.filter(|hz| Self::usable_dial(*hz));
        self
    }

    /// The dial the radio was actually being worked on — what the front end
    /// should open on, so the program comes back up hearing what it was
    /// hearing whichever VFO that was.
    pub fn active_dial_hz(&self) -> f64 {
        match self.active_vfo {
            sdroxide_types::Vfo::A => self.freq_hz,
            sdroxide_types::Vfo::B => self.vfo_b_hz.unwrap_or(self.freq_hz),
        }
    }
}

/// The remembered dial and mode, or the defaults on a first run.
/// Radio 0's session; other radios go through [`Store::load_session`].
pub fn load_session() -> Session {
    Store::station().load_session()
}

pub fn save_session(session: &Session) -> Result<(), ConfigError> {
    Store::station().save_session(session)
}

/// Band-stack registers: up to 3 remembered (freq, mode, filter) per band.
pub type BandStacks =
    std::collections::HashMap<sdroxide_types::Band, Vec<sdroxide_types::BandStackEntry>>;

/// One named snapshot of how the station is being worked — the whole
/// rememberable radio state (dials, VFOs, mode, filters, gains, drive,
/// antennas) plus the digital identity and message templates it works with
/// and the band stacks it was put together in (issue #197).
///
/// A profile is a scoping of the operator's working setup, not a config
/// backup: it deliberately does **not** carry the hardware. The backend, the
/// audio devices and the converters are personal to a radio — profiles
/// follow the operator across the rig they always sit at, and a "contest"
/// profile should not drag yesterday's sound card behind it.
///
/// "Save" writes whatever the radio is doing right now; "apply" puts the
/// radio back onto a saved setup without touching hardware it is not part of.
///
/// `#[serde(default)]`, like the [`Session`] and the digital settings inside
/// it: a profile is kept for as long as the operator keeps it, so it will be
/// read by builds that have grown fields since it was written. A field it
/// lacks takes its default instead of dropping the whole profile out of the
/// list.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    /// The operator's name for it — "contest", "DX", "CB", the band plan.
    pub name: String,
    /// The remembered radio state: dials, VFOs, mode, filters, gains, drive,
    /// antennas.
    pub session: Session,
    /// The digital identity and the message templates.
    pub digi: sdroxide_types::DigiConfig,
    /// The band stacks — which bands this way of working the station has its
    /// setup stored in.
    pub stacks: BandStacks,
}

/// The named profiles defined on this station. Station scope, like the band
/// stacks: a profile is a way of working the station, not a thing a single
/// radio owns.
/// Saved window arrangements — SDRuno's workspaces. A list of independent
/// records, so a workspace this build cannot read costs only itself and not the
/// rest (see [`load_json_list`]).
pub fn load_workspaces() -> Vec<sdroxide_types::Workspace> {
    load_json_list("workspaces.json")
}

pub fn save_workspaces(workspaces: &Vec<sdroxide_types::Workspace>) -> Result<(), ConfigError> {
    save_json("workspaces.json", workspaces)
}

pub fn load_profiles() -> Vec<Profile> {
    load_json_list("profiles.json")
}

pub fn save_profiles(profiles: &Vec<Profile>) -> Result<(), ConfigError> {
    save_json("profiles.json", profiles)
}

fn load_json<T: serde::de::DeserializeOwned + Default>(file: &str) -> T {
    let Ok(dir) = config_dir() else { return T::default() };
    let FileText::Text(text) = read_config_text(&dir, file) else { return T::default() };
    match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            quarantine_unreadable(&dir, file, &e);
            T::default()
        }
    }
}

/// Load a JSON list, keeping the entries that parse when one of them does not.
///
/// `serde_json` fails a `Vec<T>` whole: one entry carrying a value this build
/// cannot read takes every other entry in the file with it. For a settings
/// struct that is the right answer — it is one document, and half of it is not
/// a setup — but a memory list, a logbook or an address book is not a document.
/// It is hundreds of rows entered by hand over years, each one independent of
/// the rest, and "all of them, because one was odd" is a trade nobody would
/// make. The one channel that will not load is the one the operator can retype.
///
/// So the outer list is parsed first and each row after it. What loads is
/// returned, what does not is named in the log and counted in an alert, and the
/// file is **copied** to `<file>.bak` before the caller can write the survivors
/// back over it — copied, not renamed, because the survivors are already live
/// and the next save has to land somewhere.
fn load_json_list<T: serde::de::DeserializeOwned>(file: &str) -> Vec<T> {
    let Ok(dir) = config_dir() else { return Vec::new() };
    let FileText::Text(text) = read_config_text(&dir, file) else { return Vec::new() };
    // The shape of the file itself. Something that is not a list at all is
    // broken rather than partly readable, and is quarantined like any other.
    let rows: Vec<serde_json::Value> = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            quarantine_unreadable(&dir, file, &e);
            return Vec::new();
        }
    };
    let total = rows.len();
    let mut out = Vec::with_capacity(total);
    for (i, row) in rows.into_iter().enumerate() {
        match serde_json::from_value(row) {
            Ok(v) => out.push(v),
            // Logged one by one on purpose: the count goes on screen, but which
            // entry and why is what turns "some of my memories are missing"
            // into a bug that can be fixed.
            Err(e) => warn!("{file}: entry {i} will not load ({e}); keeping the other entries"),
        }
    }
    let dropped = total - out.len();
    if dropped > 0 {
        let kept = fs::copy(dir.join(file), dir.join(format!("{file}.bak"))).is_ok();
        let mut msg = format!(
            "{file}: {dropped} of {total} entries could not be read and have been left out; \
             the other {} are here",
            out.len()
        );
        if kept {
            msg.push_str(&format!(" — the file as it was is kept as {file}.bak"));
        }
        msg.push_str(&format!("; {}", where_to_look(file)));
        push_load_alert(msg);
    }
    out
}

/// [`load_json`] with a chance to rewrite the parsed JSON first, for a file
/// whose shape has changed since it was written. The migration sees the object
/// as it is on disk and leaves anything it does not recognise alone; a file
/// that is already current goes through untouched.
///
/// Nothing is written back here: a start that only reads the file leaves it
/// exactly as it was. The new shape reaches the disk the next time the operator
/// changes the setting, and the save drops whatever the migration replaced — so
/// a downgrade after that point comes up on the defaults for those keys rather
/// than on the operator's old values.
fn load_json_migrated<T: serde::de::DeserializeOwned + Default>(
    file: &str,
    migrate: fn(&mut serde_json::Value),
) -> T {
    let Ok(dir) = config_dir() else { return T::default() };
    let FileText::Text(text) = read_config_text(&dir, file) else { return T::default() };
    let parsed = serde_json::from_str::<serde_json::Value>(&text).and_then(|mut v| {
        migrate(&mut v);
        serde_json::from_value(v)
    });
    match parsed {
        Ok(v) => v,
        Err(e) => {
            quarantine_unreadable(&dir, file, &e);
            T::default()
        }
    }
}

fn save_json<T: serde::Serialize>(file: &str, value: &T) -> Result<(), ConfigError> {
    let dir = config_dir()?;
    let text = serde_json::to_string_pretty(value).expect("serialize");
    write_atomic(&dir, file, &text)
}

pub fn load_bandstacks() -> BandStacks {
    load_json("bandstacks.json")
}

pub fn save_bandstacks(stacks: &BandStacks) -> Result<(), ConfigError> {
    save_json("bandstacks.json", stacks)
}

/// A remote client's screen settings kept on the server — the "save on the
/// server" half of [`sdroxide_types::UiSettings::client_save_scope`].
///
/// Two levels, because both a shared station and a single owner want this: each
/// named **profile** has its own set, and there is one **station default** for a
/// client whose profile has none. A client signing in as `madmedicnl` gets that
/// profile's screen; one with no profile of its own gets the default.
///
/// Only [`sdroxide_types::UiSettings`] is stored, and only its **presentation**
/// half (see [`sdroxide_types::ClientScreen`]) — plus, in the `bindings` maps,
/// the control bindings of a client that opted in to carrying them. The screen
/// is stored for every client; the bindings only when one sends them, which a
/// client does only behind its operator's acknowledgement, because a shared
/// station's keyboard belongs to the machine.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ClientSettingsStore {
    /// A station-wide default applied to a client whose profile has none.
    pub default: Option<sdroxide_types::ClientScreen>,
    /// Per-profile sets, keyed by the profile name the client signed in as.
    pub profiles: std::collections::BTreeMap<String, sdroxide_types::ClientScreen>,
    /// Control bindings for a client that opted in, station-wide.
    pub bindings_default: Option<sdroxide_types::InputSettings>,
    /// Control bindings per profile, for clients that opted in.
    pub bindings: std::collections::BTreeMap<String, sdroxide_types::InputSettings>,
    /// The "do not ask me again" answers, per **login and browser**, keyed
    /// `login@Browser/System` (`@Chrome/Android` with no login). The operator's
    /// rule: once answered, a login is not asked again on that browser; on a
    /// new browser it answers once more. No fallback between keys.
    pub acks: std::collections::BTreeMap<String, sdroxide_types::ClientAcks>,
}

impl ClientSettingsStore {
    /// The settings for `profile`: its own set, else the station default. The
    /// name comes back so the client can say which one it is using.
    pub fn for_profile(
        &self,
        profile: Option<&str>,
    ) -> Option<(Option<String>, sdroxide_types::ClientScreen)> {
        if let Some(name) = profile
            && let Some(s) = self.profiles.get(name)
        {
            return Some((Some(name.to_string()), *s));
        }
        self.default.map(|s| (None, s))
    }

    /// Store `settings` against `profile`, or as the default when it is `None`.
    /// Only the presentation half is kept.
    pub fn set(&mut self, profile: Option<&str>, settings: sdroxide_types::ClientScreen) {
        match profile {
            Some(name) => {
                self.profiles.insert(name.to_string(), settings);
            }
            None => self.default = Some(settings),
        }
    }

    /// The control bindings for `profile`, its own set else the default, with
    /// the name they came from. `None` when the store holds none for it.
    pub fn bindings_for_profile(
        &self,
        profile: Option<&str>,
    ) -> Option<(Option<String>, sdroxide_types::InputSettings)> {
        if let Some(name) = profile
            && let Some(b) = self.bindings.get(name)
        {
            return Some((Some(name.to_string()), b.clone()));
        }
        self.bindings_default.clone().map(|b| (None, b))
    }

    /// The answers stored for `login` on `browser`, empty when none have been
    /// given there.
    pub fn acks_for(&self, login: &str, browser: &str) -> sdroxide_types::ClientAcks {
        self.acks.get(&acks_key(login, browser)).cloned().unwrap_or_default()
    }

    /// Merge `incoming` into what `login` holds on `browser`; returns the result.
    pub fn merge_acks(
        &mut self,
        login: &str,
        browser: &str,
        incoming: &sdroxide_types::ClientAcks,
    ) -> sdroxide_types::ClientAcks {
        let slot = self.acks.entry(acks_key(login, browser)).or_default();
        slot.merge(incoming);
        slot.clone()
    }

    /// Store `bindings` against `profile`, or as the default when it is `None`.
    pub fn set_bindings(&mut self, profile: Option<&str>, bindings: sdroxide_types::InputSettings) {
        match profile {
            Some(name) => {
                self.bindings.insert(name.to_string(), bindings);
            }
            None => self.bindings_default = Some(bindings),
        }
    }
}

fn acks_key(login: &str, browser: &str) -> String {
    format!("{login}@{browser}")
}

/// Name a browser by its family and system from a User-Agent — `Chrome/Android`,
/// `Firefox/Windows` — **without the version**, so an update does not make it a
/// new browser. Nothing is stored in the browser for this: it is read from the
/// request every time, which is the point (cookies and storage cannot be
/// depended on). The cost, accepted: two machines with the same browser and
/// system share one set of answers, and Chrome and its installed app (PWA) on
/// one phone are one browser. A client sending no User-Agent (the desktop app)
/// is `App`.
pub fn browser_label(ua: Option<&str>) -> String {
    let Some(ua) = ua.filter(|u| !u.trim().is_empty()) else { return "App".into() };
    // Order matters: Edge, Opera and Samsung all claim Chrome, and Chrome
    // claims Safari.
    let family = if ua.contains("Edg") {
        "Edge"
    } else if ua.contains("OPR/") || ua.contains("Opera") {
        "Opera"
    } else if ua.contains("SamsungBrowser") {
        "Samsung"
    } else if ua.contains("Firefox/") || ua.contains("FxiOS") {
        "Firefox"
    } else if ua.contains("Chrome/") || ua.contains("CriOS") || ua.contains("Chromium") {
        "Chrome"
    } else if ua.contains("Safari/") {
        "Safari"
    } else {
        "Other"
    };
    let system = if ua.contains("Android") {
        "Android"
    } else if ua.contains("iPhone") || ua.contains("iPad") || ua.contains("iPod") {
        "iOS"
    } else if ua.contains("Windows") {
        "Windows"
    } else if ua.contains("CrOS") {
        "ChromeOS"
    } else if ua.contains("Mac OS") || ua.contains("Macintosh") {
        "macOS"
    } else if ua.contains("Linux") {
        "Linux"
    } else {
        "Other"
    };
    format!("{family}/{system}")
}

pub fn load_client_settings() -> ClientSettingsStore {
    load_json("clientsettings.json")
}

pub fn save_client_settings(store: &ClientSettingsStore) -> Result<(), ConfigError> {
    save_json("clientsettings.json", store)
}

pub fn load_memories() -> Vec<sdroxide_types::MemoryChannel> {
    load_json_list("memories.json")
}

pub fn save_memories(memories: &[sdroxide_types::MemoryChannel]) -> Result<(), ConfigError> {
    save_json("memories.json", &memories)
}

/// The frequencies the operator has added to the digital modes' own tables.
///
/// Its own file for the same reason the memory list has one: it is a list the
/// operator builds as they use the radio, and it must survive an sdroxide that
/// knows nothing about it — a version without the feature leaves the file
/// alone rather than writing an empty one over it.
pub fn load_digi_presets() -> Vec<sdroxide_types::DigiPreset> {
    load_json_list("digi_presets.json")
}

pub fn save_digi_presets(presets: &[sdroxide_types::DigiPreset]) -> Result<(), ConfigError> {
    save_json("digi_presets.json", &presets)
}

/// The memory folders. Their own file rather than a new shape for
/// `memories.json`, so a list written before folders existed still loads.
pub fn load_memory_folders() -> Vec<sdroxide_types::MemoryFolder> {
    load_json_list("memory_folders.json")
}

pub fn save_memory_folders(folders: &[sdroxide_types::MemoryFolder]) -> Result<(), ConfigError> {
    save_json("memory_folders.json", &folders)
}

/// The Morse trainer's progress: which Koch characters are unlocked and the
/// running score. Its own file, like the memory list, so a version without the
/// trainer leaves it alone rather than overwriting it.
pub fn load_morse_progress() -> sdroxide_types::MorseProgress {
    load_json("morse.json")
}

pub fn save_morse_progress(progress: &sdroxide_types::MorseProgress) -> Result<(), ConfigError> {
    save_json("morse.json", progress)
}

/// Radio backend config (SoapySDR vs CAT rig; serial + sound-card settings).
/// Radio 0's config; other radios go through [`Store::load_radio_config`].
pub fn load_radio_config() -> sdroxide_types::RadioConfig {
    Store::station().load_radio_config()
}

pub fn save_radio_config(cfg: &sdroxide_types::RadioConfig) -> Result<(), ConfigError> {
    Store::station().save_radio_config(cfg)
}

/// FT8/FT4 operator config (own call, grid, message templates).
pub fn load_digi_config() -> sdroxide_types::DigiConfig {
    load_json_migrated("digi.json", migrate_digi)
}

/// One `tx_audio_level` became two — `_fm` (deviation) and `_ssb` (drive into
/// the modulator) — because the single number was doing both jobs at once and
/// an FM setting was silently taking the level off sideband data as well.
///
/// A file written before the split carries its value into **both**, so no
/// station's signal changes level on an update: an operator who had turned it
/// down for packet keeps that level everywhere until they deliberately raise
/// the sideband one, which they can now see. A file written since keeps what
/// it says — `or_insert` only fills a key that is not there.
fn migrate_digi(v: &mut serde_json::Value) {
    let Some(obj) = v.as_object_mut() else { return };
    let Some(old) = obj.get("tx_audio_level").and_then(serde_json::Value::as_f64) else { return };
    let old = serde_json::json!(old);
    for key in ["tx_audio_level_fm", "tx_audio_level_ssb"] {
        obj.entry(key).or_insert_with(|| old.clone());
    }
}

pub fn save_digi_config(cfg: &sdroxide_types::DigiConfig) -> Result<(), ConfigError> {
    save_json("digi.json", cfg)
}

/// Skimmer preferences (per-kind enable + squelch). The operator's choice, kept
/// separate from the live `RadioState.skimmer`: a narrowband (audio-mode)
/// source forces the skimmers off, and that must not overwrite what the
/// operator picked for a wideband one.
pub fn load_skimmer_config() -> sdroxide_types::SkimmerSettings {
    load_json("skimmer.json")
}

pub fn save_skimmer_config(cfg: &sdroxide_types::SkimmerSettings) -> Result<(), ConfigError> {
    save_json("skimmer.json", cfg)
}

/// ISM decoder preferences (which device families, and the burst threshold).
///
/// Kept apart from the live `RadioState.ism` for the same reason as the skimmer's:
/// a front end that hands over demodulated audio rather than IQ forces the
/// decoder off, and that must not overwrite what the operator chose for one that
/// does.
pub fn load_ism_config() -> sdroxide_types::IsmSettings {
    load_json("ism.json")
}

pub fn save_ism_config(cfg: &sdroxide_types::IsmSettings) -> Result<(), ConfigError> {
    save_json("ism.json", cfg)
}

/// ADS-B decoder preferences (issue #160).
///
/// Kept apart from the live `RadioState.adsb` for the same reason the ISM
/// decoder's are: a front end that hands over demodulated audio, or one that
/// cannot deliver two megasamples a second, forces the decoder off, and that
/// must not overwrite what the operator chose for a receiver that can.
pub fn load_adsb_config() -> sdroxide_types::AdsbSettings {
    load_json::<sdroxide_types::AdsbSettings>("adsb.json").sane()
}

pub fn save_adsb_config(cfg: &sdroxide_types::AdsbSettings) -> Result<(), ConfigError> {
    save_json("adsb.json", cfg)
}

/// VDL Mode 2 decoder preferences.
///
/// Kept apart from the live `RadioState.vdl2` for the same reason the ADS-B and
/// ISM decoders' are: a front end that hands over demodulated audio, or one too
/// narrow to reach any of the channel plan, forces the decoder off, and that
/// must not overwrite what the operator chose for a receiver that can run it.
/// Kept apart from the live `RadioState.ais` for the same reason the ADS-B and
/// VDL2 configs are: the engine forces the live one off on a front end that
/// cannot feed it, and that must not overwrite what the operator chose.
pub fn load_ais_config() -> sdroxide_types::AisSettings {
    load_json::<sdroxide_types::AisSettings>("ais.json").sane()
}

pub fn save_ais_config(cfg: &sdroxide_types::AisSettings) -> Result<(), ConfigError> {
    save_json("ais.json", cfg)
}

/// The DAB panel's memory: the channels a scan found, the stations on each,
/// the starred presets and the station chosen. A fact about where the receiver
/// is, which costs a two-minute sweep to learn again, so it is kept on the
/// machine running the radio and read back at every start. Kept apart from the
/// live `RadioState.dab` for the same reason AIS's is: a front end that cannot
/// feed the lane forces the live one off, and that must not overwrite this.
/// The first radio's DAB settings; each radio has its own — see
/// [`Store::load_dab_config`].
pub fn load_dab_config() -> sdroxide_types::DabSettings {
    Store::station().load_dab_config()
}

pub fn save_dab_config(cfg: &sdroxide_types::DabSettings) -> Result<(), ConfigError> {
    Store::station().save_dab_config(cfg)
}

pub fn load_vdl2_config() -> sdroxide_types::Vdl2Settings {
    load_json::<sdroxide_types::Vdl2Settings>("vdl2.json").sane()
}

pub fn save_vdl2_config(cfg: &sdroxide_types::Vdl2Settings) -> Result<(), ConfigError> {
    save_json("vdl2.json", cfg)
}

/// QO-100 beacon-tracker preferences.
///
/// Kept apart from the live `RadioState.qo100` for the same reason the ADS-B
/// and AIS configs are: a front end with no downconverter, or one too narrow
/// to hold the parking window, forces the live lane off, and that must not
/// overwrite what the operator chose for a receiver that can run it.
pub fn load_qo100_config() -> sdroxide_types::Qo100Settings {
    load_json::<sdroxide_types::Qo100Settings>("qo100.json").sane()
}

pub fn save_qo100_config(cfg: &sdroxide_types::Qo100Settings) -> Result<(), ConfigError> {
    save_json("qo100.json", cfg)
}

/// HFDL decoder preferences — the lane's on/off and the assigned frequency it
/// sits on. The operator's own list of channels is separate and lives beside
/// it, in `hfdl_channels.json` (see [`load_hfdl_channels`]).
pub fn load_hfdl_config() -> sdroxide_types::HfdlSettings {
    load_json::<sdroxide_types::HfdlSettings>("hfdl.json").sane()
}

pub fn save_hfdl_config(cfg: &sdroxide_types::HfdlSettings) -> Result<(), ConfigError> {
    save_json("hfdl.json", cfg)
}

/// Scanner settings: what to scan, how hard a signal has to be to stop it, and
/// which memories to pass over. Restored at startup so a scan set up once is
/// one keypress away afterwards.
pub fn load_scanner_config() -> sdroxide_types::ScannerConfig {
    Store::station().load_scanner_config()
}

pub fn save_scanner_config(cfg: &sdroxide_types::ScannerConfig) -> Result<(), ConfigError> {
    Store::station().save_scanner_config(cfg)
}

/// FSQ contacts (address book for directed FSQCALL messaging).
pub fn load_contacts() -> Vec<sdroxide_types::FsqContact> {
    load_json_list("contacts.json")
}

pub fn save_contacts(contacts: &[sdroxide_types::FsqContact]) -> Result<(), ConfigError> {
    save_json("contacts.json", &contacts)
}

/// Persistent logbook (digital + manual QSO entries).
pub fn load_qso_log() -> Vec<sdroxide_types::QsoRecord> {
    load_json_list("qso_log.json")
}

pub fn save_qso_log(log: &[sdroxide_types::QsoRecord]) -> Result<(), ConfigError> {
    save_json("qso_log.json", &log)
}

/// Favourite broadcast stations (`broadcast_favourites.json`), by name — the
/// stations a listener comes back to.
pub fn load_broadcast_favourites() -> Vec<String> {
    load_json_list("broadcast_favourites.json")
}

pub fn save_broadcast_favourites(names: &[String]) -> Result<(), ConfigError> {
    save_json("broadcast_favourites.json", &names)
}

/// The operator's own HFDL channels (`hfdl_channels.json`), in kHz — the busy
/// assigned frequencies that are not in the published plan, saved as one-click
/// chips beside it. Per machine, like the broadcast favourites: a channel a
/// listener found busy is theirs, not the station's.
pub fn load_hfdl_channels() -> Vec<u32> {
    load_json_list("hfdl_channels.json")
}

pub fn save_hfdl_channels(khz: &[u32]) -> Result<(), ConfigError> {
    save_json("hfdl_channels.json", &khz)
}

/// Scheduled recordings (`recording_jobs.json`).
pub fn load_recording_jobs() -> Vec<sdroxide_types::RecordingJob> {
    load_json_list("recording_jobs.json")
}

pub fn save_recording_jobs(jobs: &[sdroxide_types::RecordingJob]) -> Result<(), ConfigError> {
    save_json("recording_jobs.json", &jobs)
}

/// The shortwave listener's reception log (`swl_log.json`) — what was *heard*.
///
/// A separate file from the QSO log on purpose: a reception has no callsign and
/// no exchange, and the two are shown by different windows.
pub fn load_swl_log() -> Vec<sdroxide_types::SwlEntry> {
    load_json_list("swl_log.json")
}

pub fn save_swl_log(log: &[sdroxide_types::SwlEntry]) -> Result<(), ConfigError> {
    save_json("swl_log.json", &log)
}

/// The listener's reception-report preferences (`swl_report.json`) — the picture
/// a report carries and their standing note.
///
/// Its own file beside the log rather than a field in `Settings` or
/// `NetworkConfig`, for one reason: `picture` is a **path on this machine**, and
/// neither of those travels to a browser tab or a second station. A file path is
/// not an identity, and `swl_id` is the part of a report that is.
pub fn load_swl_report() -> sdroxide_types::SwlReportPrefs {
    load_json::<sdroxide_types::SwlReportPrefs>("swl_report.json")
}

/// Persist the report preferences. See [`load_swl_report`] for the file's own
/// name and why it is separate.
pub fn save_swl_report(prefs: &sdroxide_types::SwlReportPrefs) -> Result<(), ConfigError> {
    save_json("swl_report.json", prefs)
}

/// Network cockpit config (spot feeds, callsign lookup, uploads; credentials).
pub fn load_network_config() -> sdroxide_types::NetworkConfig {
    load_json("net.json")
}

pub fn save_network_config(cfg: &sdroxide_types::NetworkConfig) -> Result<(), ConfigError> {
    save_json("net.json", cfg)
}

/// Built-in TCI server config (the listener third-party TCI clients connect
/// to). Owned by the engine, like the network-cockpit config above.
pub fn load_tci_server_config() -> sdroxide_types::TciServerConfig {
    Store::station().load_tci_server_config()
}

pub fn save_tci_server_config(cfg: &sdroxide_types::TciServerConfig) -> Result<(), ConfigError> {
    Store::station().save_tci_server_config(cfg)
}

/// Built-in Hamlib rigctld server config (the listener "NET rigctl" clients
/// connect to). Owned by the engine, like the TCI server config above.
pub fn load_rigctld_config() -> sdroxide_types::RigctldConfig {
    Store::station().load_rigctld_config()
}

pub fn save_rigctld_config(cfg: &sdroxide_types::RigctldConfig) -> Result<(), ConfigError> {
    Store::station().save_rigctld_config(cfg)
}

/// WSJT-X UDP broadcast config (where decode/QSO datagrams are sent). Owned by
/// the engine, like the server configs above.
pub fn load_wsjtx_config() -> sdroxide_types::WsjtxConfig {
    Store::station().load_wsjtx_config()
}

pub fn save_wsjtx_config(cfg: &sdroxide_types::WsjtxConfig) -> Result<(), ConfigError> {
    Store::station().save_wsjtx_config(cfg)
}

/// Control-input bindings: keyboard chords, panadapter mouse behaviour and the
/// MIDI mapping. Unlike the configs above this one belongs to the *client*, not
/// the engine — it describes the hardware on the operator's desk, so a knob
/// keeps working when the UI drives a remote engine over `--connect`.
pub fn load_input_settings() -> sdroxide_types::InputSettings {
    let Ok(dir) = config_dir() else { return Default::default() };
    let FileText::Text(text) = read_config_text(&dir, "input.json") else {
        return Default::default();
    };
    parse_input_settings(&text)
}

/// Read one `input.json`, keeping the bindings it can and dropping the ones it
/// cannot — and naming what it dropped.
///
/// `serde_json` fails a whole struct on one bad value, and `input.json` is a
/// list of chords the operator set up one at a time, each independent of the
/// rest. It is not a document where half of it being unreadable means none of it
/// is usable, which is the argument `load_json_list` makes for a memory list or
/// a logbook. So one binding naming an action this build no longer has — a
/// removed feature, a hand-edited file — would otherwise take the other two
/// dozen with it, and the operator would come back to a keyboard that had
/// forgotten everything. [`load_json`] cannot help: it is one document, and it
/// quarantines a file it cannot read at all, which is the right answer there.
///
/// Split out from [`load_input_settings`] so it can be exercised on a string.
/// Setting `SDROXIDE_CONFIG_DIR` to test it would be two tests in one binary
/// racing on a process-global, and the one already there says in its own SAFETY
/// note that nothing else reads the variable.
fn parse_input_settings(text: &str) -> sdroxide_types::InputSettings {
    let Ok(mut v) = serde_json::from_str::<serde_json::Value>(text) else {
        // Not a shape we can pick apart — hand the whole file to the strict
        // reader, which quarantines it and says why, as it always has.
        return load_json("input.json");
    };
    let Some(keys) = v.get_mut("keys").and_then(serde_json::Value::as_array_mut) else {
        return load_json("input.json");
    };
    let before = keys.len();
    let mut dropped = Vec::new();
    keys.retain(|k| match serde_json::from_value::<sdroxide_types::KeyBinding>(k.clone()) {
        Ok(_) => true,
        Err(_) => {
            dropped.push(
                k.get("action").and_then(serde_json::Value::as_str).unwrap_or("?").to_string(),
            );
            false
        }
    });
    if dropped.is_empty() {
        // Everything read: the strict path, so nothing about the ordinary file
        // changes for the sake of this tolerance.
        return serde_json::from_value(v).unwrap_or_default();
    }
    tracing::warn!(
        "input.json: {} of {before} bindings name an action this build does not have ({}) \
         and were dropped; every other binding was kept. The rest of the file loaded.",
        dropped.len(),
        dropped.join(", "),
    );
    serde_json::from_value(v).unwrap_or_default()
}

pub fn save_input_settings(cfg: &sdroxide_types::InputSettings) -> Result<(), ConfigError> {
    save_json("input.json", cfg)
}

/// SSTV per-slot transmit overlay messages (one entry per image slot). The
/// image pixels live as PNGs under [`sstv_tx_dir`]; this stores just the text
/// that is composited over each slot's picture.
pub fn load_sstv_messages() -> Vec<String> {
    load_json("sstv_messages.json")
}

pub fn save_sstv_messages(messages: &[String]) -> Result<(), ConfigError> {
    save_json("sstv_messages.json", &messages)
}

/// The operator's satellite additions: element sets pasted in by hand, and
/// frequency entries that override or extend the built-in table.
///
/// An engine-side file like `net.json`, despite describing something only the
/// UI draws. The subscribed listings are fetched over HTTPS and cached on disk,
/// and in server mode that machine's tracker is also what feeds the browser's
/// 3D view — so a browser client, which has neither, has to be able to
/// configure the one that does.
pub fn load_sat_config() -> sdroxide_types::SatConfig {
    let mut cfg: sdroxide_types::SatConfig = load_json("satellites.json");
    // The amateur satellites and the ISS used to be fetched unconditionally.
    // They are subscriptions now, so a config that predates them — or a fresh
    // install with no file at all — has to be given them once, or the sky comes
    // up empty. Written back immediately so the seeding happens exactly once
    // and unsubscribing sticks.
    if cfg.seed_defaults() {
        if let Err(e) = save_sat_config(&cfg) {
            warn!("could not write the seeded satellite subscriptions: {e}");
        }
    }
    cfg
}

pub fn save_sat_config(cfg: &sdroxide_types::SatConfig) -> Result<(), ConfigError> {
    save_json("satellites.json", cfg)
}

/// The rotctld client a satellite lock steers the antenna through. Owned by
/// the engine, like the server configs above.
pub fn load_rotator_config() -> sdroxide_types::RotatorConfig {
    load_json("rotator.json")
}

pub fn save_rotator_config(cfg: &sdroxide_types::RotatorConfig) -> Result<(), ConfigError> {
    save_json("rotator.json", cfg)
}

/// The external transmit/receive switch — the relay board or contact closure
/// that grounds the SDR's antenna while the station transmits. Owned by the
/// engine, like the rotator above, and for the same reason: it is a fact about
/// the machine the antenna is attached to, not about the screen in front of the
/// operator.
pub fn load_relay_config() -> sdroxide_types::RelayConfig {
    load_json("relay.json")
}

pub fn save_relay_config(cfg: &sdroxide_types::RelayConfig) -> Result<(), ConfigError> {
    save_json("relay.json", cfg)
}

// ── Broadcast station schedules ──────────────────────────────────────────────
//
// Three layers, in the order they win:
//
//   1. the schedule EiBi publishes for the current season, downloaded and cached
//      under `broadcast/`, or the copy compiled into the binary until one
//      arrives;
//   2. the hand-kept longwave and standard-time entries, merged in by
//      `sdroxide_types::broadcast::merge` because EiBi covers neither;
//   3. `broadcast_stations.json`, the operator's own additions and corrections,
//      which is never written by sdroxide.
//
// This is the arrangement `sdroxide-solar`'s satellite frequencies already use —
// a built-in table plus user overrides — rather than seeding a copy of everything
// into the config directory, which cannot survive a schedule that is reissued
// twice a year.

/// The operator's own broadcast stations (`broadcast_stations.json`).
pub const BROADCAST_STATIONS_FILE: &str = "broadcast_stations.json";
/// Where the downloaded season schedules are cached.
const BROADCAST_CACHE_DIR: &str = "broadcast";
/// EiBi's schedule files, published free for exactly this use.
///
/// Plain HTTP because the site's certificate is expired; nothing is trusted on
/// the strength of the transport, the payload is parsed into typed rows and
/// rejected unless it looks like a schedule.
const EIBI_SKED_URL: &str = "http://www.eibispace.de/dx/sked-{season}.csv";

/// Aoki's schedule, as published: one **zip** per season holding a single `.txt`.
///
/// There is no plain-text URL — `.../ncb24.txt` answers **404** — so the zip is
/// the only way to get it, and unzipping is a step EiBi never needed. The file
/// inside is **Shift-JIS**, not Latin-1.
/// Aoki's **index page**, from which the shortwave schedule's own filename is
/// read rather than guessed.
///
/// This is the whole reason Aoki never downloaded. The name follows the
/// broadcasting season, so a template built from ours asks for `ncb26.zip` — and
/// **Aoki publishes shortwave once a year, so the newest there is `ncb24.zip`**
/// while our season is A26. Every guessed name 404s, which is why the first build
/// silently had no Aoki at all. Reading the site's own index is the only way to
/// ask it what it has.
const AOKI_INDEX_URL: &str = "https://www1.s2.starcat.ne.jp/ndxc/nnk.htm";

/// Turn a schedule's **bytes** into text, which depends entirely on the source.
///
/// EiBi is Latin-1 and Aoki is Shift-JIS, and getting this wrong is silent: the
/// wrong decode still yields the right *number* of stations and the right
/// positions, because the columns are aligned by the file's own header, and only
/// the station **names** come out as mojibake. Aoki's file has four non-ASCII
/// bytes in 800 kB — almost all of it is ASCII — so a Latin-1 reading is nearly
/// indistinguishable from a right one until you read a name.
fn decode_schedule(source: sdroxide_types::ScheduleSource, bytes: &[u8]) -> String {
    match source {
        sdroxide_types::ScheduleSource::Aoki => {
            let (cow, _, _) = encoding_rs::SHIFT_JIS.decode(bytes);
            cow.into_owned()
        }
        _ => sdroxide_types::broadcast::decode_latin1(bytes),
    }
}

/// The cached bytes of `source` as schedule text.
///
/// **One place, because the cache round-trip is where this goes wrong.** The
/// cache holds the bytes **as downloaded** — EiBi's CSV, Aoki's **zip** — and the
/// reader here has to undo whatever the fetcher did. Aoki is the case that
/// matters: reading its cached zip with the generic Shift-JIS decode returns
/// *something*, not an error, so the store comes back empty and looks exactly
/// like a source with nothing in it.
fn cached_text_from_bytes(
    source: sdroxide_types::ScheduleSource,
    season: &str,
    bytes: &[u8],
) -> Result<String, String> {
    if source == sdroxide_types::ScheduleSource::Aoki {
        aoki_text_from_zip(bytes, season)
    } else {
        Ok(decode_schedule(source, bytes))
    }
}

/// Read the schedule out of Aoki's zip — or out of a file that **is** the `.txt`,
/// as an operator who saved one by hand has.
///
/// The two arrive at the same place because they are the same schedule: the
/// downloader caches the zip it fetched, and [`import_schedule_file`] caches the
/// zip or the text the operator gave it, under the same name. The zip's own
/// magic number decides which, so a reader cannot be handed the wrong one by a
/// file extension.
fn aoki_text_from_zip(bytes: &[u8], season: &str) -> Result<String, String> {
    let text = if bytes.starts_with(b"PK\x03\x04") {
        aoki_schedule_entry(bytes, season)?
    } else {
        bytes.to_vec()
    };
    // Shift-JIS (CP932). `encoding_rs` is already in the tree via
    // symphonia-metadata, so this costs nothing new.
    let (cow, _, had_errors) = encoding_rs::SHIFT_JIS.decode(&text);
    if had_errors {
        // A station name with a character CP932 cannot hold comes back as U+FFFD
        // and keeps its **byte length**, so the columns after it still line up —
        // but say so, because a schedule quietly full of replacement characters
        // is worse than one that says which names are affected.
        warn!(
            "Aoki's file has bytes CP932 cannot decode; some station names will read as \u{fffd}"
        );
    }
    Ok(cow.into_owned())
}

/// Every `.txt` entry in a zip, decompressed, as `(name, bytes)`.
///
/// **A schedule zip is not a one-file zip.** Aoki's `ncb24.zip` holds three
/// entries: `ncb24.xlsx`, `userlist1.txt` and `ncb24.txt` — and
/// `userlist1.txt` is *a listener's own saved list*, 473 kB, in a **different
/// Aoki format** (kHz, Time, ITU, Station, Lang, Location, Days — no ADM column,
/// no position column at all).
///
/// So "the first `.txt`" is wrong, and wrong in the worst way: it is a real
/// schedule-shaped file from the right publisher, so nothing about the download
/// looks amiss. The first version did exactly this and reported **zero**
/// stations, which is the only reason it was caught — with a user's list in the
/// older format it would have parsed into plausible, wrong stations.
///
/// Enumerating rather than picking keeps the decision explicit and lets the
/// caller say what it chose.
fn zip_txt_entries(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    use std::io::Read;
    const EOCD_SIG: u32 = 0x0605_4b50;
    const CEN_SIG: u32 = 0x0201_4b50;

    // An EOCD is 22 bytes plus an optional comment, so it **starts** at most
    // `len - 22`. The first cut searched `0..len - 65`, which excluded the
    // record from any archive under about 87 bytes and so refused every small
    // one — including the ones a test builds. Found by the test, not by reading.
    let eocd = (0..bytes.len().saturating_sub(21))
        .rev()
        .find(|&i| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap()) == EOCD_SIG)
        .ok_or("not a zip: no end-of-central-directory record")?;
    let count = u16::from_le_bytes(bytes[eocd + 10..eocd + 12].try_into().unwrap()) as usize;
    let mut off = u32::from_le_bytes(bytes[eocd + 16..eocd + 20].try_into().unwrap()) as usize;
    let mut out = Vec::new();

    for _ in 0..count {
        if off + 46 > bytes.len()
            || u32::from_le_bytes(bytes[off..off + 4].try_into().unwrap()) != CEN_SIG
        {
            return Err("not a zip: central directory is malformed".into());
        }
        let method = u16::from_le_bytes(bytes[off + 10..off + 12].try_into().unwrap());
        let csize = u32::from_le_bytes(bytes[off + 20..off + 24].try_into().unwrap()) as usize;
        let name_len = u16::from_le_bytes(bytes[off + 28..off + 30].try_into().unwrap()) as usize;
        let extra_len = u16::from_le_bytes(bytes[off + 30..off + 32].try_into().unwrap()) as usize;
        let comment_len =
            u16::from_le_bytes(bytes[off + 32..off + 34].try_into().unwrap()) as usize;
        let lho = u32::from_le_bytes(bytes[off + 42..off + 46].try_into().unwrap()) as usize;
        let name = String::from_utf8_lossy(&bytes[off + 46..off + 46 + name_len]).into_owned();

        if name.to_ascii_lowercase().ends_with(".txt") {
            if lho + 30 > bytes.len() {
                return Err("not a zip: local header is out of range".into());
            }
            let l_name = u16::from_le_bytes(bytes[lho + 26..lho + 28].try_into().unwrap()) as usize;
            let l_extra =
                u16::from_le_bytes(bytes[lho + 28..lho + 30].try_into().unwrap()) as usize;
            let start = lho + 30 + l_name + l_extra;
            let end = start + csize;
            if end > bytes.len() {
                return Err("not a zip: entry data is out of range".into());
            }
            let raw = &bytes[start..end];
            let data = match method {
                0 => raw.to_vec(),
                8 => {
                    let mut buf = Vec::with_capacity(csize * 2);
                    flate2::read::DeflateDecoder::new(raw)
                        .read_to_end(&mut buf)
                        .map_err(|e| format!("zip: {e}"))?;
                    buf
                }
                // Skipped rather than failed: a `.txt` we cannot decompress is
                // not a reason to reject a schedule sitting beside it.
                other => {
                    warn!("zip entry {name} uses unsupported method {other}; skipping it");
                    off += 46 + name_len + extra_len + comment_len;
                    continue;
                }
            };
            out.push((name, data));
        }
        off += 46 + name_len + extra_len + comment_len;
    }
    if out.is_empty() {
        return Err("not a schedule zip: no .txt entry".into());
    }
    Ok(out)
}

/// Aoki's **schedule** out of its zip: the one `.txt` that is actually the
/// schedule, chosen in a stated order rather than by position.
///
/// 1. An entry named for the season — `ncb26.txt` for season `b26`. Aoki names
///    its files by two-digit year, so the season letter is dropped.
/// 2. Otherwise the `.txt` whose head says what it is: Aoki's schedule files
///    begin `Bnn Shortwave Frequency List …`, and no user list does.
/// 3. Otherwise the largest `.txt`, with the names reported — a guess, but a
///    named one.
///
/// **Identifying by content is the point.** A user's saved list is a real file
/// from the right publisher in a real Aoki format, so nothing about picking the
/// wrong one looks like a mistake until the stations come out wrong.
fn aoki_schedule_entry(bytes: &[u8], season: &str) -> Result<Vec<u8>, String> {
    let entries = zip_txt_entries(bytes)?;
    let names: Vec<&str> = entries.iter().map(|(n, _)| n.as_str()).collect();

    let yy: String = season.chars().filter(|c| c.is_ascii_digit()).take(2).collect();
    if yy.len() == 2 {
        let want = format!("ncb{yy}.txt");
        if let Some((_, d)) = entries.iter().find(|(n, _)| n.eq_ignore_ascii_case(&want)) {
            info!("Aoki's zip: {want} is the schedule");
            return Ok(d.clone());
        }
    }
    if let Some((n, d)) = entries.iter().find(|(_, d)| {
        let head: String = String::from_utf8_lossy(&d[..d.len().min(256)]).into_owned();
        head.contains("Frequency List")
    }) {
        info!("Aoki's zip: {n} names itself a frequency list, so it is the schedule");
        return Ok(d.clone());
    }
    let (name, data) =
        entries.iter().max_by_key(|(_, d)| d.len()).expect("zip_txt_entries guarantees one entry");
    warn!(
        "Aoki's zip has no entry that looks like the schedule; taking the largest .txt ({name}) of {names:?}"
    );
    Ok(data.clone())
}
/// Where the operator's own station list lives, for showing in the settings panel.
pub fn broadcast_stations_path() -> Result<PathBuf, ConfigError> {
    Ok(config_dir()?.join(BROADCAST_STATIONS_FILE))
}

/// Where a season's downloaded schedule is cached.
pub fn broadcast_cache_path(season: &str) -> Result<PathBuf, ConfigError> {
    // The season is used in a filename, so it must not be able to escape the
    // directory even though it is computed rather than typed in.
    let season: String = season.chars().filter(|c| c.is_ascii_alphanumeric()).take(8).collect();
    Ok(config_dir()?.join(BROADCAST_CACHE_DIR).join(format!("sked-{season}.csv")))
}

/// Where a season's downloaded schedule is cached, **for a given source**.
///
/// EiBi keeps the name it has always had (`sked-b26.csv`), so an existing
/// download is still found after this change and no operator re-downloads 6 MB
/// of schedule because we added a second source. Every other source gets its own
/// file, which is what lets both be **kept** while only one is **active** — the
/// choice bebepip was asked about, answered as "gone from the waterfall, kept on
/// disk so switching back is one click".
pub fn broadcast_cache_path_for(
    season: &str,
    source: sdroxide_types::ScheduleSource,
) -> Result<PathBuf, ConfigError> {
    use sdroxide_types::ScheduleSource as S;

    // The season is used in a filename, so it must not be able to escape the
    // directory even though it is computed rather than typed in.
    let season: String = season.chars().filter(|c| c.is_ascii_alphanumeric()).take(8).collect();
    let name = match source {
        S::Eibi => format!("sked-{season}.csv"),
        S::Aoki => format!("aoki-{season}.txt"),
        S::Hfcc => format!("hfcc{season}.sked"),
        S::User => "user-stations.txt".to_string(),
    };
    Ok(config_dir()?.join(BROADCAST_CACHE_DIR).join(name))
}

/// The operator's own broadcast entries. Absent by default — this file holds
/// additions and corrections, not a copy of the schedule.
pub fn load_broadcast_overrides() -> Vec<sdroxide_types::BroadcastStation> {
    let Ok(path) = broadcast_stations_path() else { return Vec::new() };
    retire_seeded_broadcast_list(&path);
    let Ok(text) = fs::read_to_string(&path) else { return Vec::new() };
    match serde_json::from_str::<sdroxide_types::BroadcastStations>(&text) {
        Ok(f) => f.stations,
        Err(e) => {
            warn!("failed to parse {BROADCAST_STATIONS_FILE}: {e}; ignoring it");
            Vec::new()
        }
    }
}

/// Move aside a `broadcast_stations.json` that is a copy of a shipped schedule.
///
/// Earlier versions seeded the whole table into this file. Now that the schedule
/// is downloaded and this file holds only the operator's own entries, such a copy
/// would lay a stale season back over a fresh one — hundreds of duplicated,
/// out-of-date transmissions.
///
/// Generated copies are recognised by the `source` or `updated` keys, which only
/// sdroxide's own table generators ever set, and nothing hand-written would
/// carry them. So this never touches a file an operator actually wrote, and it
/// is kept as `.bak` either way.
fn retire_seeded_broadcast_list(path: &std::path::Path) {
    let Ok(text) = fs::read_to_string(path) else { return };
    let Ok(file) = serde_json::from_str::<sdroxide_types::BroadcastStations>(&text) else {
        return;
    };
    if file.source.is_empty() && file.updated.is_empty() {
        return;
    }
    let backup = path.with_extension("json.bak");
    match fs::rename(path, &backup) {
        Ok(()) => info!(
            "{BROADCAST_STATIONS_FILE} was a copy of a bundled schedule; kept as \
             {} and replaced by the downloaded one",
            backup.display()
        ),
        Err(e) => warn!("could not retire the seeded {BROADCAST_STATIONS_FILE}: {e}"),
    }
}

// There is deliberately no writer for `broadcast_stations.json`. It is the one
// file here that belongs entirely to the operator, and "sdroxide never writes
// it" is a contract the manual states — shipping a save function would be an
// invitation to break it, and would give `retire_seeded_broadcast_list` a case
// it cannot distinguish from a stale seeded copy.

/// The full station list: the cached (or compiled-in) schedule, plus the
/// hand-kept longwave entries, with the operator's own entries laid over the top.
pub fn load_broadcast_stations() -> Vec<sdroxide_types::BroadcastStation> {
    let schedule = match cached_schedule() {
        Some(stations) => stations,
        None => sdroxide_types::broadcast::builtin().to_vec(),
    };
    apply_broadcast_overrides(schedule, load_broadcast_overrides())
}

/// **The master store**: every downloaded source in one list, tagged.
///
/// bebepip's answer on fork #69 — *"I would merge all these databases into one
/// master repository"* — rather than the handover's older "one active source at a
/// time". The repetition a merge creates in this list is his accepted cost: it is
/// the Schedule window, where the detail belongs, and the panadapter is what
/// collapses it (one name per station, `broadcast_label_dedupe`).
///
/// A source with nothing cached simply does not appear, so this degrades to the
/// old behaviour rather than to an empty store — **the failure that matters here
/// is silence**, since a store that quietly holds nothing looks exactly like a
/// working one.
pub fn load_broadcast_master() -> Vec<sdroxide_types::BroadcastStation> {
    use sdroxide_types::ScheduleSource as S;
    let mut sources: Vec<(S, Vec<sdroxide_types::BroadcastStation>)> = Vec::new();
    let mut found = 0usize;
    for source in [S::Eibi, S::Aoki, S::Hfcc] {
        if let Some(rows) = cached_schedule_for(source) {
            found += rows.len();
            sources.push((source, rows));
        }
    }
    let overrides = load_broadcast_overrides();
    if !overrides.is_empty() {
        sources.push((S::User, overrides.clone()));
    }
    if sources.is_empty() {
        // Nothing downloaded: the compiled-in EiBi still applies, and is tagged as
        // what it is rather than as the operator's own.
        return apply_broadcast_overrides(sdroxide_types::broadcast::builtin().to_vec(), overrides);
    }
    info!("broadcast master store: {found} rows from {} sources", sources.len());
    apply_broadcast_overrides(
        sdroxide_types::broadcast::master_store(&sources),
        // The user's own rows are already in the store; laying them over again
        // would be harmless but would re-sort, so they are left where they are.
        Vec::new(),
    )
}

/// Download **every** source that can be downloaded, keeping the caches that
/// fail.
///
/// One source failing is not a reason to lose the other: HFCC in particular needs
/// its companion station-name file and is not buildable yet, which used to be a
/// reason to refuse the whole request.
pub fn fetch_broadcast_master() -> Result<Vec<sdroxide_types::BroadcastStation>, String> {
    use sdroxide_types::ScheduleSource as S;
    let mut errors: Vec<String> = Vec::new();
    let mut got = 0usize;
    for source in [S::Eibi, S::Aoki, S::Hfcc] {
        match fetch_broadcast_schedule_from(source) {
            Ok(rows) => {
                got += rows.len();
                info!("fetched the {source:?} schedule: {} transmissions", rows.len());
            }
            Err(e) => {
                warn!("the {source:?} schedule did not download: {e}");
                errors.push(format!("{source:?}: {e}"));
            }
        }
    }
    if got == 0 {
        return Err(if errors.is_empty() {
            "no broadcast source could be downloaded".into()
        } else {
            errors.join("; ")
        });
    }
    Ok(load_broadcast_master())
}

/// Aoki's own shortwave schedule URL, **read from their index**.
///
/// Picks the highest-numbered season in `pc/nc/`, because that is the newest file
/// they have published and not necessarily the season we are in.
fn aoki_latest(agent: &ureq::Agent) -> Result<(String, String), String> {
    let season = sdroxide_types::broadcast::season_file(now_unix());
    // **The season's own package first.** `xt{season}.zip` is what the season's
    // schedule is actually published as, and it is the one a working third-party
    // downloader uses. The fallback below exists because the shortwave folder
    // lags: it still held **B24** while we were in A26, which is a file eighteen
    // months stale and a schedule of last season's transmitters.
    let url = format!("https://www1.m2.mediacat.ne.jp/binews/ut/xt/xt{season}.zip");
    let resp = agent.get(&url).call().map_err(|e| e.to_string())?;
    if resp.status().is_success() {
        return Ok((url, format!("xt{season}")));
    }
    // Fall back to Aoki's own index: the newest **shortwave** file it lists, which
    // is not necessarily the season we are in. Better than nothing, and it names
    // itself so the cache says which file it holds.
    let mut index = agent.get(AOKI_INDEX_URL).call().map_err(|e| {
        format!(
            "Aoki's seasonal package 404s ({}) and its index would not load: {e}",
            resp.status()
        )
    })?;
    if !index.status().is_success() {
        return Err(format!(
            "Aoki's seasonal package is HTTP {} and its index is HTTP {}",
            resp.status(),
            index.status()
        ));
    }
    let bytes = index
        .body_mut()
        .with_config()
        .limit(4 * 1024 * 1024)
        .read_to_vec()
        .map_err(|e| e.to_string())?;
    let (page, _, _) = encoding_rs::SHIFT_JIS.decode(&bytes);
    let mut best: Option<(String, String)> = None;
    for link in page.split("href=\"") {
        let Some(href) = link.split('"').next() else {
            continue;
        };
        let Some(name) = href.rsplit('/').next() else {
            continue;
        };
        let stem = name.strip_suffix(".zip").unwrap_or(name);
        if !href.contains("/pc/nc/") || !stem.starts_with("nc") || stem.len() < 2 {
            continue;
        }
        let year = &stem[stem.len() - 2..];
        if !year.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        if best.as_ref().is_none_or(|(_, y)| year > y.as_str()) {
            best = Some((href.to_string(), stem.to_string()));
        }
    }
    let (href, stem) =
        best.ok_or("Aoki publishes neither a seasonal package nor a shortwave schedule")?;
    warn!(
        "Aoki's seasonal package is unavailable; falling back to its newest shortwave file, {stem}"
    );
    Ok((href, stem))
}

/// Which sources are cached, and how many rows each holds — the Settings page's
/// per-source line, so *"which of my schedules is actually here"* has an answer
/// instead of being inferred from a list that happens to be short.
pub fn cached_broadcast_sources() -> Vec<(sdroxide_types::ScheduleSource, Option<usize>)> {
    let season = sdroxide_types::broadcast::season_file(now_unix());
    [
        sdroxide_types::ScheduleSource::Eibi,
        sdroxide_types::ScheduleSource::Aoki,
        sdroxide_types::ScheduleSource::Hfcc,
    ]
    .into_iter()
    .map(|source| {
        let here = broadcast_cache_path_for(&season, source).map(|p| p.exists()).unwrap_or(false);
        (source, here.then(|| cached_schedule_for(source).map(|v| v.len()).unwrap_or(0)))
    })
    .collect()
}

/// Import a listener's own station list into `broadcast_stations.json`.
///
/// **This is the answer to "where do I upload mine", and it is not the reception
/// log.** The log records what you *heard*; this is a *schedule* — what is expected
/// to be on air, which is what the waterfall labels against and what LISTEN draws
/// its span from. Mixing them would put a station in the picture that was never
/// transmitted.
///
/// Three shapes are accepted, because a listener's list comes from wherever they
/// kept it and asking them to convert it first is the thing that stops them: the
/// sdroxide JSON, a CSV of `frequency,name…`, and a plain text list. **Frequency
/// may carry any unit** — kHz or MHz — and where none is written the **number**
/// decides, by the same rule the schedule files are read by.
pub fn import_broadcast_stations(path: &std::path::Path) -> Result<usize, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut rows: Vec<sdroxide_types::BroadcastStation> =
        serde_json::from_str::<sdroxide_types::BroadcastStations>(&text)
            .map(|f| f.stations)
            .unwrap_or_else(|_| parse_own_list(&text));
    rows.retain(|r| r.freq_khz > 0.0 && !r.name.trim().is_empty());
    if rows.is_empty() {
        return Err("nothing in that file looked like a station".into());
    }
    for r in &mut rows {
        r.source = sdroxide_types::ScheduleSource::User;
    }
    rows.sort_by(|a, b| a.freq_khz.total_cmp(&b.freq_khz));
    let mut all = load_broadcast_overrides();
    for row in rows {
        let dup = all
            .iter()
            .position(|s| s.name == row.name && (s.freq_khz - row.freq_khz).abs() < 0.001);
        match dup {
            Some(i) => all[i] = row,
            None => all.push(row),
        }
    }
    let n = all.len();
    // `BroadcastStations` carries provenance for tables *sdroxide* generated; a
    // listener's own list is not one, so those fields stay empty rather than
    // stamped with a source `retire_seeded_broadcast_list` would then have to
    // reason about.
    save_json(
        BROADCAST_STATIONS_FILE,
        &sdroxide_types::BroadcastStations {
            version: 1,
            updated: String::new(),
            source: String::new(),
            note: String::new(),
            stations: all,
        },
    )
    .map_err(|e| e.to_string())?;
    Ok(n)
}

/// What [`import_schedule_file`] found in the file it was handed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedSchedule {
    /// Which publisher's list it is — the one whose cache it was filed under.
    pub source: sdroxide_types::ScheduleSource,
    /// The season the cache file is named for, which is what the reader looks
    /// for: Aoki's own for Aoki (`xta26`), ours for EiBi.
    pub season: String,
    pub stations: usize,
}

/// Take a published schedule from a file the operator saved themselves.
///
/// **Aoki is the reason this exists.** Its short-wave list lives on a personal
/// site whose path changes with the season, so the automatic download is the one
/// part that can break — and when it does, the program is the one that cannot do
/// what it says. So the same file is accepted from disk: either the `.zip` Aoki
/// publishes (the one the downloader would have fetched) or the `.txt` from
/// inside it. EiBi's `.csv` is read too, because it is one file and the operation
/// is the same one; HFCC is not offered, since its names live in a package of
/// three files and half of it is worse than none.
///
/// The bytes go into **the same cache the downloader writes, under the same
/// name**, so everything downstream — the Shift-JIS decode, the parser, the merge
/// with the other sources, the operator's own corrections — is the code that
/// already runs. There is no second schedule path here, and a file imported by
/// hand and one downloaded are the same file as far as the rest of the program
/// is concerned.
pub fn import_schedule_file(path: &std::path::Path) -> Result<ImportedSchedule, String> {
    use sdroxide_types::ScheduleSource as S;

    let bytes = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if bytes.is_empty() {
        return Err(format!("{} is empty", path.display()));
    }
    let ours = sdroxide_types::broadcast::season_file(now_unix());
    // **Told apart by what is in the file, not by what it is called.** A browser
    // saves Aoki's `xta26.zip` and `xta26.txt` with the same stem, and an
    // operator who renamed either should still get the right reader.
    let (source, season) = if bytes.starts_with(b"PK\x03\x04") {
        (S::Aoki, aoki_season_from_path(path).unwrap_or_else(|| format!("xt{ours}")))
    } else {
        let head = sdroxide_types::broadcast::decode_latin1(&bytes[..bytes.len().min(512)]);
        // Aoki's title line says "… Frequecy List" (their spelling), and the
        // column header under it says `FRE STATION … UTC`, which is what the
        // parser itself looks for — so a file saved without its first line is
        // still recognised as Aoki's.
        let aoki_head = head.contains("Frequecy List")
            || head.contains("Frequency List")
            || (head.contains("STATION") && head.contains("UTC"));
        if aoki_head {
            (S::Aoki, aoki_season_from_path(path).unwrap_or_else(|| format!("xt{ours}")))
        } else if head.contains("kHz:") && head.contains(';') {
            (S::Eibi, ours)
        } else {
            return Err(format!(
                "{} is not a schedule this program knows: an Aoki file is the \
                 .zip Aoki publishes or the .txt from inside it, and an EiBi file is \
                 its sked .csv.",
                path.display()
            ));
        }
    };

    // Read it exactly the way the cache is read, so a file that would not work
    // from the cache does not look fine here and fail after the restart.
    let text = cached_text_from_bytes(source, &season, &bytes)?;
    let stations =
        sdroxide_types::broadcast::parse_from(source, &text).map_err(|e| e.to_string())?;
    if stations.len() < MIN_SCHEDULE_ROWS {
        return Err(format!(
            "{} parses to {} transmissions, which is not a schedule — a truncated \
             download or the wrong file",
            path.display(),
            stations.len()
        ));
    }

    let cache = broadcast_cache_path_for(&season, source).map_err(|e| e.to_string())?;
    if let Some(dir) = cache.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    fs::write(&cache, &bytes).map_err(|e| e.to_string())?;
    // **One Aoki file, so the reader cannot pick the stale one.** The cache is
    // read by asking what Aoki files are on disk and taking the newest by name;
    // a season named with a letter that sorts below last year's would lose to it
    // and the import would look like it did nothing.
    if source == S::Aoki
        && let Some(dir) = cache.parent()
        && let Ok(entries) = fs::read_dir(dir)
    {
        for entry in entries.flatten() {
            let p = entry.path();
            if p != cache
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("aoki-") && n.ends_with(".txt"))
            {
                let _ = fs::remove_file(p);
            }
        }
    }
    info!(
        "imported the {season} {source:?} broadcast schedule from {}: {} transmissions",
        path.display(),
        stations.len()
    );
    Ok(ImportedSchedule { source, season, stations: stations.len() })
}

/// The season an Aoki file names itself with: `xta26` for `xta26.zip`, `b26` for
/// `ncb26.txt`, and nothing when the name says nothing (`schedule.zip`), which
/// leaves the caller to fall back to the season we are in.
fn aoki_season_from_path(path: &std::path::Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    let clean: String = stem
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(8)
        .collect::<String>()
        .to_ascii_lowercase();
    let named = clean.starts_with("xt") || clean.starts_with("nc") || clean.starts_with("b2");
    (named && clean.chars().any(|c| c.is_ascii_digit())).then_some(clean)
}

/// HFCC's cached package, unpacked and parsed.
///
/// **The tables travel with the schedule inside one zip**, so this is the same
/// three-file read as the download and cannot pair a fresh schedule with a stale
/// table. Every HFCC-specific rule lives in `parse_hfcc`; this only opens it.
fn cached_hfcc() -> Option<Vec<sdroxide_types::BroadcastStation>> {
    let season = sdroxide_types::broadcast::season_file(now_unix());
    let path = broadcast_cache_path_for(&season, sdroxide_types::ScheduleSource::Hfcc).ok()?;
    let bytes = fs::read(&path).ok()?;
    parse_hfcc_rows(&bytes).ok()
}

/// The Aoki seasons already cached, newest first.
///
/// **Aoki publishes shortwave once a year and files it under its own season**, so
/// the season this program is in is frequently not the season on disk. Reading the
/// cache therefore asks *"what Aoki files are here?"* rather than assuming.
fn aoki_cached_seasons() -> Vec<String> {
    let Ok(dir) = config_dir() else { return Vec::new() };
    let Ok(entries) = fs::read_dir(dir.join(BROADCAST_CACHE_DIR)) else {
        return Vec::new();
    };
    let mut out: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            n.strip_prefix("aoki-")
                .and_then(|r| r.strip_suffix(".txt"))
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        })
        .collect();
    out.sort_by(|a, b| b.cmp(a));
    out
}

/// The season a cache filename carries, if it is one of ours.
fn path_stem(path: &std::path::Path) -> Option<String> {
    let n = path.file_name()?.to_str()?.to_string();
    for (pre, suf) in [("aoki-", ".txt"), ("sked-", ".csv"), ("hfcc", ".sked")] {
        if let Some(mid) = n.strip_prefix(pre).and_then(|r| r.strip_suffix(suf)) {
            return Some(mid.to_string());
        }
    }
    None
}

/// HFCC's own season, which is ours except that they publish per season.
fn cache_season_of(season: &str) -> String {
    season.to_string()
}

/// HFCC's published package for a season.
///
/// **`a26allx2.zip`, under the season's own folder** — `data/a26/a26allx2.zip`,
/// not `data/a26allx2.zip`, which is a 404. That is the same shape of mistake as
/// Aoki's guessed filename, and the same reason: the path is read from their data
/// index rather than composed here.
fn hfcc_schedule_url() -> String {
    let season = sdroxide_types::broadcast::season_file(now_unix());
    format!("https://new.hfcc.org/data/{season}/{season}allx2.zip")
}

/// HFCC's package: unpack the schedule and both tables, and parse them together.
fn fetch_hfcc(
    bytes: &[u8],
    season: String,
) -> Result<Vec<sdroxide_types::BroadcastStation>, String> {
    let stations = parse_hfcc_rows(bytes)?;
    // **Refuse rather than ship a schedule of codes.** Measured against A26:
    // 3367 rows, and the station name resolves for a small minority — the
    // schedule's own `LOC` and `BRC` columns live in a different code space from
    // the `SITE.TXT` and `BROADCAS.TXT` tables that hold the names. So the
    // frequencies, times, days, countries and positions are all real and the
    // *names* are not, which is precisely the "list of WCR" this feature was
    // built to refuse. The names live in HFCC's separate by-broadcaster
    // schedules, which is a different artefact and not built yet.
    let named = stations.iter().filter(|r| !r.name.is_empty() && r.name.len() > 3).count();
    if stations.len() > 0 && named * 10 < stations.len() * 8 {
        return Err(format!(
            "HFCC's schedule has no broadcaster names ({named} of {} rows resolve), \
             so it would be a list of codes rather than stations. The names are in \
             HFCC's separate by-broadcaster schedules, which is not built yet.",
            stations.len()
        ));
    }
    let path = broadcast_cache_path_for(&season, sdroxide_types::ScheduleSource::Hfcc)
        .map_err(|e| e.to_string())?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    fs::write(&path, bytes).map_err(|e| e.to_string())?;
    prune_broadcast_cache(&season);
    info!("downloaded the {season} HFCC schedule: {} transmissions", stations.len());
    Ok(stations)
}

/// The three tables out of an HFCC package, parsed together.
///
/// Every HFCC-specific rule lives in `sdroxide_types::broadcast::parse_hfcc`; this
/// only decides which entry in the package is which, and **refuses rather than
/// guessing** when a table is absent — a schedule without its tables is a list of
/// `WCR` and `BRC` codes, which is worse than no schedule.
fn parse_hfcc_rows(bytes: &[u8]) -> Result<Vec<sdroxide_types::BroadcastStation>, String> {
    let mut schedule = String::new();
    let mut broadcasters = String::new();
    let mut sites = String::new();
    for (name, data) in zip_entries(bytes)? {
        let lower = name.to_ascii_lowercase();
        let text = String::from_utf8_lossy(&data).into_owned();
        if schedule.is_empty() && text.contains(";FREQ") {
            schedule = text;
        } else if lower.contains("broadcas") {
            broadcasters = text;
        } else if lower.contains("site") {
            sites = text;
        }
    }
    if schedule.is_empty() {
        return Err("HFCC's package has no schedule file in it".into());
    }
    if broadcasters.is_empty() || sites.is_empty() {
        return Err(format!(
            "HFCC's package is missing its reference tables ({}), so it would be a \
             list of codes rather than stations",
            if broadcasters.is_empty() { "broadcaster names" } else { "sites" }
        ));
    }
    sdroxide_types::broadcast::parse_hfcc(&schedule, &broadcasters, &sites)
        .map_err(|e| format!("HFCC: {e}"))
}

/// Every entry in a zip, by name — not just `.txt`, because HFCC's tables are
/// lower-case and its schedule is upper-case and both matter.
fn zip_entries(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    zip_txt_entries(bytes)
}

/// A listener's own list, as CSV or as plain lines.
fn parse_own_list(text: &str) -> Vec<sdroxide_types::BroadcastStation> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let cells: Vec<&str> = if line.contains(',') {
            line.split(',').map(str::trim).collect()
        } else {
            line.split_whitespace().collect()
        };
        let Some(pos) = cells.iter().position(|c| own_frequency_khz(c).is_some()) else {
            continue;
        };
        let freq = own_frequency_khz(cells[pos]).expect("just found it");
        let mut name: Vec<&str> = cells.clone();
        name.remove(pos);
        // A trailing unit belongs to the frequency, not the name: "BBC 15400 kHz"
        // is a station called BBC, not one called "BBC kHz".
        if name
            .last()
            .is_some_and(|t| t.eq_ignore_ascii_case("khz") || t.eq_ignore_ascii_case("mhz"))
        {
            name.pop();
        }
        let name = name.join(" ");
        if name.trim().is_empty() {
            continue;
        }
        out.push(sdroxide_types::BroadcastStation {
            name: name.trim().to_string(),
            freq_khz: freq,
            source: sdroxide_types::ScheduleSource::User,
            ..Default::default()
        });
    }
    out
}

/// A frequency in a listener's own notes, in kHz.
///
/// **Decided by magnitude where no unit is written.** Shortwave here is 3 kHz to
/// 130 MHz, so a bare number at or above 1000 can only be kHz and anything under
/// it can only be MHz. That is the rule the schedule files use, and it means
/// `15400`, `15400 kHz` and `15.4` all land on the same row without asking.
fn own_frequency_khz(cell: &str) -> Option<f64> {
    let c = cell.trim();
    let (digits, unit) = match c.split_once(char::is_whitespace) {
        Some((d, u)) => (d.trim(), Some(u.trim())),
        None => (c, None),
    };
    let digits = digits.trim_end_matches("kHz").trim_end_matches("MHz").trim();
    let v: f64 = digits.parse().ok()?;
    let khz = match unit {
        Some(u) if u.eq_ignore_ascii_case("khz") => v,
        Some(u) if u.eq_ignore_ascii_case("mhz") => v * 1000.0,
        _ if v >= 1000.0 => v,
        _ => v * 1000.0,
    };
    (3.0..=130_000.0).contains(&khz).then_some(khz)
}

/// Lay the operator's entries over a schedule: one with the same name and
/// frequency replaces the scheduled row, anything else is added.
fn apply_broadcast_overrides(
    mut schedule: Vec<sdroxide_types::BroadcastStation>,
    overrides: Vec<sdroxide_types::BroadcastStation>,
) -> Vec<sdroxide_types::BroadcastStation> {
    for own in overrides {
        let same = |s: &sdroxide_types::BroadcastStation| {
            s.name == own.name && (s.freq_khz - own.freq_khz).abs() < 0.001
        };
        schedule.retain(|s| !same(s));
        schedule.push(own);
    }
    schedule.sort_by(|a, b| {
        a.freq_khz
            .total_cmp(&b.freq_khz)
            .then_with(|| a.start_utc.cmp(&b.start_utc))
            .then_with(|| a.name.cmp(&b.name))
    });
    schedule
}

/// The cached schedule for the season we are in, if it has been downloaded.
fn cached_schedule() -> Option<Vec<sdroxide_types::BroadcastStation>> {
    cached_schedule_for(sdroxide_types::ScheduleSource::Eibi)
}

/// The cached schedule of **one chosen source**, or `None`.
///
/// One source, not all of them: EiBi, Aoki and HFCC overlap heavily, so reading
/// all three would paint one station on the waterfall two or three times, and the
/// label is there so a station can be **identified** — three copies of it is
/// worse than none. `SWL-HANDOVER.md`, fork #69.
///
/// A source that is **not** currently downloaded is simply absent: the fallback
/// to the compiled-in EiBi list happens above this, so an operator who selects
/// Aoki before it has downloaded sees EiBi and says so, rather than a blend.
pub fn cached_schedule_for(
    source: sdroxide_types::ScheduleSource,
) -> Option<Vec<sdroxide_types::BroadcastStation>> {
    if source == sdroxide_types::ScheduleSource::Hfcc {
        return cached_hfcc();
    }
    let season = sdroxide_types::broadcast::season_file(now_unix());
    // Aoki's cache is filed under **Aoki's** season, so look for that first and
    // fall back to ours; a reader that assumed ours would simply never find it.
    let aoki_season = aoki_cached_seasons();
    let path = aoki_season
        .iter()
        .map(|s| broadcast_cache_path_for(s, source))
        .chain(std::iter::once(broadcast_cache_path_for(&season, source)))
        .find_map(|p| p.ok().filter(|p| p.exists()))?;
    let bytes = fs::read(&path).ok()?;
    let text = match cached_text_from_bytes(
        source,
        path_stem(&path).as_deref().unwrap_or(&season),
        &bytes,
    ) {
        Ok(t) => t,
        Err(e) => {
            warn!("cached {source:?} schedule would not unpack: {e}");
            return None;
        }
    };
    let stations = sdroxide_types::broadcast::parse_from(source, &text).unwrap_or_else(|e| {
        warn!("cached {source:?} schedule would not parse: {e}");
        Vec::new()
    });
    if stations.len() < MIN_SCHEDULE_ROWS {
        warn!("cached schedule {season} has only {} entries; ignoring it", stations.len());
        return None;
    }
    Some(sdroxide_types::broadcast::merge(stations))
}

/// A schedule with fewer transmissions than this is not a schedule — a captive
/// portal's login page, a truncated download, a season file that has not been
/// published yet. Whatever it is, the compiled-in copy is better.
const MIN_SCHEDULE_ROWS: usize = 500;

/// Whether the current season's schedule still needs downloading.
///
/// True on a first run, and again after each changeover, because the cache is
/// keyed by season: October's file simply is not March's.
pub fn broadcast_schedule_due() -> bool {
    broadcast_schedule_due_for(sdroxide_types::ScheduleSource::Eibi)
}

/// Whether the **chosen** source still needs downloading.
pub fn broadcast_schedule_due_for(source: sdroxide_types::ScheduleSource) -> bool {
    let season = sdroxide_types::broadcast::season_file(now_unix());
    match broadcast_cache_path_for(&season, source) {
        Ok(p) => !p.exists(),
        Err(_) => false,
    }
}

/// The season sdroxide is currently using, and whether it came from the network.
pub fn broadcast_schedule_status() -> (String, bool) {
    let season = sdroxide_types::broadcast::season_file(now_unix());
    let cached = broadcast_cache_path(&season).map(|p| p.exists()).unwrap_or(false);
    (season, cached)
}

/// The fork's **latest full release**, from the GitHub Releases API.
///
/// `/releases/latest` is deliberately the endpoint rather than `/releases`:
/// GitHub answers it with the newest release that is **not** a draft and **not**
/// a pre-release, so the nightly tag (a pre-release) can never be offered as an
/// update. Blocking, so the caller puts it on a worker thread.
pub const RELEASES_LATEST_URL: &str =
    "https://api.github.com/repos/madmedicnl/sdroxide-brown/releases/latest";

/// The tag of the latest full release, e.g. `v2.0.3_brown`, or an error.
pub fn latest_release_tag() -> Result<String, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(std::time::Duration::from_secs(10)))
        .timeout_global(Some(std::time::Duration::from_secs(20)))
        .user_agent(concat!("sdroxide/", env!("CARGO_PKG_VERSION")))
        .build()
        .into();
    let mut resp = agent
        .get(RELEASES_LATEST_URL)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let bytes =
        resp.body_mut().with_config().limit(64 * 1024).read_to_vec().map_err(|e| e.to_string())?;
    let json: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    json.get("tag_name")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| "the release has no tag_name".to_string())
}

/// Download the current season's schedule and cache it.
///
/// Blocking, so callers put it on a worker thread. Returns the merged station
/// list on success. The download is written to the cache only after it parses
/// into a plausible schedule, so a failure leaves the previous file in place
/// rather than replacing it with a captive portal's login page.
pub fn fetch_broadcast_schedule() -> Result<Vec<sdroxide_types::BroadcastStation>, String> {
    fetch_broadcast_schedule_from(sdroxide_types::ScheduleSource::Eibi)
}

/// Download one chosen source's schedule and cache it.
///
/// **Aoki** arrives as a zip holding a Shift-JIS `.txt`, which is two steps EiBi
/// never needed and both of them quiet if wrong: unzipping the wrong way yields
/// bytes that still parse as *a* schedule, and decoding Shift-JIS as Latin-1
/// still yields the right stations and the right columns with only the
/// non-ASCII names mangled. So the decode goes through [`decode_schedule`] and
/// the same helper the cache read uses, and the plausibility check below is the
/// only thing standing between the two files and a plausible wrong answer.
pub fn fetch_broadcast_schedule_from(
    source: sdroxide_types::ScheduleSource,
) -> Result<Vec<sdroxide_types::BroadcastStation>, String> {
    use sdroxide_types::ScheduleSource as S;
    let season = sdroxide_types::broadcast::season_file(now_unix());
    if source == S::User {
        return Err("the listener's own stations are a local file, not a download".into());
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(std::time::Duration::from_secs(15)))
        .timeout_global(Some(std::time::Duration::from_secs(120)))
        .user_agent(concat!("sdroxide/", env!("CARGO_PKG_VERSION")))
        .build()
        .into();
    let url = match source {
        S::Eibi => EIBI_SKED_URL.replace("{season}", &season),
        S::Aoki => aoki_latest(&agent)?.0,
        // **One package, not two downloads.** `a26allx2.zip` holds the schedule
        // *and* its reference tables, so "the companion file" cannot go stale
        // against the schedule it belongs to — which is what made the two-file
        // version of this awkward to begin with.
        S::Hfcc => hfcc_schedule_url(),
        S::User => unreachable!("handled above"),
    };

    let mut resp = agent.get(&url).call().map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let bytes = resp
        .body_mut()
        .with_config()
        .limit(16 * 1024 * 1024)
        .read_to_vec()
        .map_err(|e| e.to_string())?;

    // Aoki's is a zip; EiBi's is the file itself. Either way the schedule text
    // ends up in one variable and everything below is source-agnostic.
    // **Aoki's own season, not ours.** They publish shortwave once a year, so
    // the file in the zip is `ncb24.txt` while we are in A26 — and asking for
    // `ncb26.txt` misses, falls through to "the largest .txt", and lands on
    // Aoki's 474 kB *listener list*, which is a real file in a real Aoki format
    // and parses to nothing. The discovered season is threaded in precisely so
    // that cannot happen.
    let aoki_season = if source == S::Aoki { Some(aoki_latest(&agent)?.1) } else { None };
    if source == S::Hfcc {
        return fetch_hfcc(&bytes, cache_season_of(&season));
    }
    let text = cached_text_from_bytes(source, aoki_season.as_deref().unwrap_or(&season), &bytes)?;
    let stations =
        sdroxide_types::broadcast::parse_from(source, &text).map_err(|e| e.to_string())?;
    if stations.len() < MIN_SCHEDULE_ROWS {
        return Err(format!(
            "{url} yielded {} transmissions, which is not a schedule",
            stations.len()
        ));
    }

    // The cache holds the **bytes as downloaded** — EiBi's CSV or Aoki's zip —
    // and is re-decoded on read, so the two paths cannot drift.
    // Cached under **Aoki's** season, so the cache file names the file it holds.
    let cache_season = aoki_season.as_deref().unwrap_or(&season);
    let path = broadcast_cache_path_for(cache_season, source).map_err(|e| e.to_string())?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    // Last season's file is only dead weight once this one has landed.
    prune_broadcast_cache(cache_season);
    info!(
        "downloaded the {cache_season} {source:?} broadcast schedule: {} transmissions",
        stations.len()
    );

    Ok(apply_broadcast_overrides(
        sdroxide_types::broadcast::merge(stations),
        load_broadcast_overrides(),
    ))
}

/// Drop cached schedules for seasons other than `keep`.
fn prune_broadcast_cache(keep: &str) {
    let Ok(path) = broadcast_cache_path(keep) else { return };
    let Some(dir) = path.parent() else { return };
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        if entry.path() != path && entry.path().extension().is_some_and(|e| e == "csv") {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// Forget the cached schedule so the next check downloads it again.
pub fn clear_broadcast_cache() -> Result<(), ConfigError> {
    let season = sdroxide_types::broadcast::season_file(now_unix());
    let path = broadcast_cache_path(&season)?;
    if path.exists() {
        fs::remove_file(&path)?;
    }
    Ok(())
}

/// Seconds since the Unix epoch.
fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Voice-keyer slot labels (one entry per slot). The recordings themselves are
/// WAV files under [`voice_dir`]; this stores only what each slot is called.
pub fn load_voice_names() -> Vec<String> {
    load_json("voice_names.json")
}

pub fn save_voice_names(names: &[String]) -> Result<(), ConfigError> {
    save_json("voice_names.json", &names)
}

/// The whole-file check, against a zip freshly downloaded from Aoki.
///
/// **Ignored by default** because it needs the publisher's file, fetched from
/// `https://www1.s2.starcat.ne.jp/ndxc/pc/nc/ncb24.zip` (the season changes, so
/// the name moves with it). It is here because the **fixture** tests cannot
/// catch the fault this caught: Aoki's zip carries three entries and the
/// schedule is not the first one, so a reader that picked by position took a
/// listener's own 474 kB saved list — a real file, in a real Aoki format, from
/// the right publisher — and reported zero stations. Every fixture in this
/// module is a one-entry zip, so every one of them passed.
///
/// ```sh
/// curl -o /tmp/ncb24.zip https://www1.s2.starcat.ne.jp/ndxc/pc/nc/ncb24.zip
/// SDROXIDE_AOKI_ZIP=/tmp/ncb24.zip \
///   cargo test -p sdroxide-config --release -- --ignored --nocapture a_published
/// ```
#[test]
#[ignore = "needs Aoki's published zip; see the doc comment for the URL"]
fn a_published_aoki_zip_yields_its_schedule() {
    let Ok(p) = std::env::var("SDROXIDE_AOKI_ZIP") else {
        panic!("set SDROXIDE_AOKI_ZIP to a downloaded ncbXX.zip");
    };
    let bytes = std::fs::read(&p).expect("the zip");

    // The file must hold more than the schedule, or the test cannot catch
    // picking the wrong one.
    let entries = zip_txt_entries(&bytes).expect("a zip");
    println!("entries: {:?}", entries.iter().map(|(n, d)| (n, d.len())).collect::<Vec<_>>());
    assert!(
        entries.len() > 1,
        "Aoki's zip is expected to hold a user list alongside the schedule; with \
         one entry this test cannot catch picking the wrong one"
    );

    let text = aoki_text_from_zip(&bytes, "b24").expect("the schedule");
    let stations =
        sdroxide_types::broadcast::parse_from(sdroxide_types::ScheduleSource::Aoki, &text)
            .expect("a schedule");
    println!("{} stations from {} chars", stations.len(), text.chars().count());
    for s in stations.iter().take(5) {
        println!("  {:8.0} kHz  {:<30} {}", s.freq_khz, s.name, s.country);
    }

    // A whole published file is thousands of rows, not a handful.
    assert!(
        stations.len() > 2_000,
        "only {} stations from the published file — the wrong entry was probably taken",
        stations.len()
    );
    // And nearly all of them belong on the map, or the position column is being
    // mis-read. 95% is a floor, not the measured figure.
    let placed = stations.iter().filter(|s| s.lat.is_some()).count();
    let pct = 100.0 * placed as f64 / stations.len() as f64;
    println!("placed on the map: {placed}/{} ({pct:.1}%)", stations.len());
    assert!(pct > 95.0, "only {pct:.1}% of stations got a position");
    // The three-letter ADM resolving to a real country is the shared vocabulary
    // doing its job, and what a wrong entry would lack entirely.
    let named = stations.iter().filter(|s| !s.country.is_empty()).count();
    assert!(named > 2_000, "only {named} stations resolved a country");
}

#[cfg(test)]
mod aoki_source_tests {
    use super::{aoki_text_from_zip, broadcast_cache_path_for, decode_schedule, zip_txt_entries};
    use sdroxide_types::ScheduleSource as S;

    const AOKI_HEADER: &str = concat!(
        "B24 Shortwave Frequency List  March 23  2025, 1300 UTC   Day 1 = Sunday\n",
        "FRE   STATION                         UTC       Su-W-Sa Language             Pow Azi Location                ADM L/L             Remarks\n"
    );

    /// A real zip, built here with the same layout Aoki publishes: deflated
    /// entries, a central directory, an end-of-central-directory record.
    ///
    /// Takes **several** entries because the fault this module guards against is
    /// only visible with more than one, and because concatenating two archives
    /// is not an archive — the second one's recorded offsets are relative to
    /// itself, so a naive concatenation yields a central directory that does not
    /// point where it says.
    fn make_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        use flate2::write::DeflateEncoder;
        use std::io::Write;
        let mut out = Vec::new();
        let mut dir = Vec::new();

        for (name, body) in entries {
            let mut enc = DeflateEncoder::new(Vec::new(), flate2::Compression::default());
            enc.write_all(body).unwrap();
            let comp = enc.finish().unwrap();
            let lho = out.len() as u32;

            // local file header
            out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
            out.extend_from_slice(&20u16.to_le_bytes()); // version needed
            out.extend_from_slice(&0u16.to_le_bytes()); // flags
            out.extend_from_slice(&8u16.to_le_bytes()); // method: deflate
            out.extend_from_slice(&[0, 0, 0, 0]); // time, date
            out.extend_from_slice(&0u32.to_le_bytes()); // crc (not checked by us)
            out.extend_from_slice(&(comp.len() as u32).to_le_bytes());
            out.extend_from_slice(&(body.len() as u32).to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes()); // extra length
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(&comp);

            // central directory record
            dir.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
            dir.extend_from_slice(&20u16.to_le_bytes()); // version made by
            dir.extend_from_slice(&20u16.to_le_bytes()); // version needed
            dir.extend_from_slice(&0u16.to_le_bytes()); // flags
            dir.extend_from_slice(&8u16.to_le_bytes()); // method
            dir.extend_from_slice(&[0, 0, 0, 0]); // time, date
            dir.extend_from_slice(&0u32.to_le_bytes()); // crc
            dir.extend_from_slice(&(comp.len() as u32).to_le_bytes());
            dir.extend_from_slice(&(body.len() as u32).to_le_bytes());
            dir.extend_from_slice(&(name.len() as u16).to_le_bytes());
            // extra, comment, disk-start, internal-attrs — four u16, then the
            // external attributes (4) and the local header offset (4). The first
            // cut wrote only three and put the offset 2 bytes early, which the
            // reader then reported as "no .txt entry" — a fault in the fixture,
            // not in the reader, and only visible because the name did not
            // arrive.
            dir.extend_from_slice(&[0u8; 8]);
            dir.extend_from_slice(&0u32.to_le_bytes()); // external attributes
            dir.extend_from_slice(&lho.to_le_bytes());
            dir.extend_from_slice(name.as_bytes());
        }

        let cen = out.len() as u32;
        let cen_len = dir.len() as u32;
        out.extend_from_slice(&dir);
        out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        out.extend_from_slice(&[0, 0, 0, 0]); // disk numbers
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&cen_len.to_le_bytes());
        out.extend_from_slice(&cen.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // comment length
        out
    }

    /// The reader has to actually inflate, and hand back the original bytes.
    #[test]
    fn a_deflated_zip_entry_comes_back_whole() {
        let body = "station,1000,1200\nsecond,2000,2400\n".repeat(200);
        let zip = make_zip(&[("ncb24.txt", body.as_bytes())]);
        assert_eq!(zip_txt_entries(&zip).unwrap().len(), 1);
        assert_eq!(zip_txt_entries(&zip).unwrap()[0].1, body.as_bytes());
    }

    /// A stored (uncompressed) entry is method 0 and takes the other arm.
    #[test]
    fn a_stored_zip_entry_comes_back_whole() {
        let mut zip = make_zip(&[("ncb24.txt", b"plain stored bytes")]);
        // rewrite the method fields to 0 (stored) and leave sizes as they are:
        // for a stored entry comp size == body size, so make them agree.
        let body = b"plain stored bytes".to_vec();
        let mut zip2 = make_zip(&[("ncb24.txt", &body)]);
        // find the two method fields and zero them
        for sig in [0x0403_4b50u32, 0x0201_4b50u32] {
            let at = zip2
                .windows(4)
                .position(|w| u32::from_le_bytes(w.try_into().unwrap()) == sig)
                .unwrap();
            zip2[at + 8..at + 10].copy_from_slice(&0u16.to_le_bytes());
        }
        zip.shrink_to_fit();
        assert_eq!(zip_txt_entries(&zip2).unwrap()[0].1, &body[..]);
    }

    /// **The regression guard for picking the wrong file out of Aoki's zip.**
    ///
    /// A published `ncbXX.zip` holds three entries — an `.xlsx`, a listener's own
    /// `userlist1.txt`, and the schedule's `ncb24.txt` — and the schedule is
    /// **not** the first `.txt`. Taking the first one yielded a real file, from
    /// the right publisher, in a real Aoki format, holding **zero** stations;
    /// with a user's list in the older layout it would have yielded plausible,
    /// wrong ones instead.
    ///
    /// The zip is built here with the **user list first**, the order that broke
    /// it, so this fails on any reader that picks by position.
    #[test]
    fn the_schedule_is_chosen_by_name_not_by_position() {
        // The listener's own list: Aoki's *other* format — frequency first, no
        // ADM, no position column — so a reader that takes it gets nothing.
        let user_list =
            b"kHz           Time(UTC)       ITU Station                Lang. Location    Days \n";
        let schedule = "B24 Shortwave Frequency List  March 23  2025, 1300 UTC   Day 1 = Sunday\n";
        let zip = make_zip(&[("userlist1.txt", user_list), ("ncb24.txt", schedule.as_bytes())]);
        let entries = zip_txt_entries(&zip).unwrap();
        assert_eq!(entries.len(), 2, "both text entries must be found");
        assert_eq!(entries[0].0, "userlist1.txt", "the wrong one does come first");

        // Season b24 names the file ncb24.txt, and that is what must be chosen.
        let chosen = crate::aoki_schedule_entry(&zip, "b24").unwrap();
        assert_eq!(
            String::from_utf8(chosen).unwrap(),
            schedule,
            "the entry chosen was the user's list, not the schedule"
        );
    }

    /// And when the season does not name the file, the one that **says what it
    /// is** wins — so a season numbered differently still works.
    #[test]
    fn a_season_that_does_not_match_the_name_still_finds_the_schedule() {
        let user_list =
            b"kHz           Time(UTC)       ITU Station                Lang. Location    Days \n";
        let schedule = "B26 Shortwave Frequency List  June 1  2026, 1300 UTC   Day 1 = Sunday\n";
        let zip = make_zip(&[("userlist1.txt", user_list), ("ncb26.txt", schedule.as_bytes())]);
        // Ask for a season whose file is not in the zip at all.
        let chosen = crate::aoki_schedule_entry(&zip, "b99").unwrap();
        assert_eq!(
            String::from_utf8(chosen).unwrap(),
            schedule,
            "it must fall back to the entry that names itself a frequency list"
        );
    }

    /// The three failure modes must be **errors**, never a wrong answer: not a
    /// zip at all, a zip with no text file, and a truncated archive.
    #[test]
    fn a_zip_that_is_not_one_is_refused() {
        assert!(zip_txt_entries(b"this is not a zip at all").is_err());
        assert!(
            zip_txt_entries(&make_zip(&[("readme.md", b"# hello")])).is_err(),
            "a zip with no .txt must be refused, not silently yield nothing"
        );
        let mut trunc = make_zip(&[("ncb24.txt", b"some content here")]);
        trunc.truncate(30);
        assert!(zip_txt_entries(&trunc).is_err(), "a truncated zip must be refused");
    }

    /// **The decode is the quiet half.** Shift-JIS read as Latin-1 still yields
    /// a schedule with the right stations and the right columns, because the
    /// columns come from the file's own header — only the non-ASCII names are
    /// wrong. So this test is about the one byte pattern that tells the two
    /// apart.
    #[test]
    fn shift_jis_is_not_latin_one() {
        // U+65E5  日 in Shift-JIS is two bytes: 0x93 0xFA. Latin-1 would read
        // those as " control characters instead of a character.
        let sjis = [0x93u8, 0xFA];
        let shift = decode_schedule(S::Aoki, &sjis);
        assert_eq!(shift, "\u{65e5}", "Aoki is Shift-JIS and must decode as it");
        let latin = decode_schedule(S::Eibi, &sjis);
        assert_ne!(latin, "\u{65e5}", "EiBi is Latin-1 and must not");
        assert_eq!(latin.chars().count(), 2, "Latin-1 turns one CJK char into two mojibake");
    }

    /// Aoki's Shift-JIS file decodes through the whole zip path, and the
    /// station names survive.
    #[test]
    fn aoki_text_survives_the_zip_and_the_shift_jis() {
        // Traditional forms throughout, and every one of them **in CP932**: the
        // first cut used \u{5f55} (录, simplified) which CP932 does not hold,
        // and came back as a numeric character reference. See the test below for
        // why that is worth knowing rather than just avoiding.
        let name = "\u{65e5}\u{672c}\u{8a18}\u{9332}\u{53f0}"; // \u65e5\u672c\u8a18\u9332\u53f0
        let row = format!(
            " 2350 {:<29} 2000-1800 1234567 Japanese            100  NE Tokyo                 JPN 4014N12549E     test\n",
            name
        );
        // Encode the whole file as Shift-JIS, which is what Aoki publishes.
        let whole = format!("{AOKI_HEADER}{row}");
        let (file, _, _) = encoding_rs::SHIFT_JIS.encode(&whole);
        let zip = make_zip(&[("ncb24.txt", file.as_ref())]);
        let text = aoki_text_from_zip(&zip, "b24").unwrap();
        assert!(
            text.contains(name),
            "the name must come back whole through zip and Shift-JIS: {text:?}"
        );
        // And it must parse into a station carrying that name.
        let stations = sdroxide_types::broadcast::parse_from(S::Aoki, &text).unwrap();
        assert_eq!(stations.len(), 1, "one row, one station: {stations:?}");
        assert!(
            stations[0].name.contains(name),
            "name {:?} should contain {name:?}",
            stations[0].name
        );
    }

    /// **Two sources must not share a cache file.** If they did, selecting Aoki
    /// would show EiBi's stations under Aoki's name, or the other way round,
    /// and neither would look wrong.
    #[test]
    fn each_source_has_its_own_cache_file() {
        let a = broadcast_cache_path_for("b26", S::Eibi).unwrap();
        let b = broadcast_cache_path_for("b26", S::Aoki).unwrap();
        assert_ne!(a, b, "EiBi and Aoki cannot share a cache file");
        assert_ne!(a, broadcast_cache_path_for("b26", S::Hfcc).unwrap(), "nor can HFCC");
        // EiBi's name is unchanged, so an existing download is still found.
        assert!(a.to_string_lossy().ends_with("sked-b26.csv"), "{}", a.display());
        assert!(b.to_string_lossy().ends_with("aoki-b26.txt"), "{}", b.display());
        // Same rules for the season sanitising as the original: a season is
        // computed, not typed, but it must not be able to escape the directory.
        let nasty = broadcast_cache_path_for("../../etc/passwd", S::Aoki).unwrap();
        assert!(nasty.to_string_lossy().contains("aoki"), "{}", nasty.display());
        assert!(
            !nasty.to_string_lossy().contains(".."),
            "a season must not escape the cache directory"
        );
    }

    /// **A `&#nnnn;` in a station name is not a decoding bug.** Found while
    /// writing the test above: `encoding_rs`'s Shift-JIS **encoder** follows the
    /// WHATWG rule and, for a character the encoding cannot hold, writes an
    /// HTML numeric reference instead of refusing — so `录` (U+5F55, simplified;
    /// CP932 has the traditional 録, U+9332, not this) encodes to the seven
    /// ASCII bytes `&#24405;` and decodes back to exactly that text. It **also
    /// sets `had_errors`**, so a caller that checks hears about it; the trap is
    /// only for a caller that does not, and ours does not decode at all.
    ///
    /// Nothing in the **read** path does this: decoding Aoki's own bytes never
    /// produces one, and `had_errors` is what tells us a name had a byte CP932
    /// could not read. This is pinned so that a future `&#…;` in a name is
    /// recognised as the encoder's doing rather than chased into the reader.
    #[test]
    fn an_unrepresentable_character_becomes_a_numeric_reference_not_a_mojibake() {
        let (bytes, _, had_errors) = encoding_rs::SHIFT_JIS.encode("\u{5f55}");
        assert!(
            had_errors,
            "and it does flag it — the substitution is loud to a caller that checks"
        );
        assert_eq!(
            String::from_utf8(bytes.to_vec()).unwrap(),
            "&#24405;",
            "it writes the reference as literal ASCII, which survives a round trip"
        );
        // And our decode path reads that back as the text it is, not as U+5F55.
        assert_eq!(decode_schedule(S::Aoki, b"&#24405;"), "&#24405;");
    }

    /// The source's own reader must be what the cache uses, or selecting Aoki
    /// and then reading it back would need two code paths.
    #[test]
    fn a_cached_aoki_file_is_read_back_as_aoki() {
        let text = format!(
            "{AOKI_HEADER}\n 2350 {:<29} 2000-1800 1234567 Japanese            100  NE Tokyo                 JPN 4014N12549E     test\n",
            "NHK Radio"
        );
        let stations = sdroxide_types::broadcast::parse_from(S::Aoki, &text).unwrap();
        assert_eq!(stations.len(), 1);
        // And EiBi must not read it: two formats, two readers, never each other.
        assert!(
            sdroxide_types::broadcast::parse_from(S::Eibi, &text).is_err()
                || sdroxide_types::broadcast::parse_from(S::Eibi, &text).unwrap().is_empty(),
            "EiBi's reader must not make sense of Aoki's file"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The client-settings store answers for a profile with its own set, and
    /// otherwise falls back to the station default — the two levels the shared
    /// station and the single owner each need.
    #[test]
    fn the_client_settings_store_falls_back_to_the_default() {
        let mut store = ClientSettingsStore::default();
        let mut def = sdroxide_types::ClientScreen::default();
        def.simple_ui = true;
        store.set(None, def);
        let mut mine = sdroxide_types::ClientScreen::default();
        mine.theme = sdroxide_types::UiTheme::AmberPhosphor;
        store.set(Some("Contest"), mine);

        // A profile with its own set gets it.
        let (from, s) = store.for_profile(Some("Contest")).expect("a stored set");
        assert_eq!(from.as_deref(), Some("Contest"));
        assert_eq!(s.theme, sdroxide_types::UiTheme::AmberPhosphor);
        assert!(!s.simple_ui);
        // An unknown profile falls back to the default.
        let (from, s) = store.for_profile(Some("DX")).expect("the default");
        assert!(from.is_none());
        assert!(s.simple_ui);
        assert_ne!(s.theme, sdroxide_types::UiTheme::AmberPhosphor);
    }

    /// The opt-in bindings sit beside the screen, with the same profile/default
    /// fallback, and are absent until a client actually sends some — an older
    /// `clientsettings.json` has neither key and still loads.
    #[test]
    fn the_client_settings_store_holds_bindings_only_when_given() {
        let mut store = ClientSettingsStore::default();
        assert!(store.bindings_for_profile(Some("Contest")).is_none());
        assert!(store.bindings_for_profile(None).is_none());

        store.set_bindings(Some("Contest"), sdroxide_types::InputSettings::default());
        let (from, _) = store.bindings_for_profile(Some("Contest")).expect("its own");
        assert_eq!(from.as_deref(), Some("Contest"));
        // A profile with none of its own falls back only when there is a
        // default; here there is not.
        assert!(store.bindings_for_profile(Some("DX")).is_none());

        store.set_bindings(None, sdroxide_types::InputSettings::default());
        assert!(store.bindings_for_profile(Some("DX")).is_some(), "the default now");

        // The old shape (screen only) deserialises with the new keys defaulted.
        let old = r#"{"default":null,"profiles":{}}"#;
        let loaded: ClientSettingsStore = serde_json::from_str(old).unwrap();
        assert!(loaded.bindings_for_profile(None).is_none());
    }

    #[test]
    fn digi_config_roundtrip_via_json() {
        let cfg = sdroxide_types::DigiConfig {
            my_call: "AB1CD".into(),
            my_grid: "FN42".into(),
            ..Default::default()
        };
        let text = serde_json::to_string_pretty(&cfg).unwrap();
        let back: sdroxide_types::DigiConfig = serde_json::from_str(&text).unwrap();
        assert_eq!(back.my_call, "AB1CD");
        assert_eq!(back.my_grid, "FN42");
        assert_eq!(back, cfg);
    }

    #[test]
    fn skimmer_config_roundtrip_via_json() {
        use sdroxide_types::{SkimmerKind, SkimmerSettings};
        let mut cfg = SkimmerSettings::default();
        cfg.set_enabled(SkimmerKind::Psk, false);
        cfg.set_squelch_db(SkimmerKind::Cw, 12);
        let back: SkimmerSettings =
            serde_json::from_str(&serde_json::to_string(&cfg).unwrap()).unwrap();
        assert_eq!(back, cfg);
        assert!(!back.enabled(SkimmerKind::Psk));
        assert_eq!(back.squelch_db(SkimmerKind::Cw), 12);
    }

    #[test]
    fn skimmer_config_fills_missing_fields() {
        // A file written before one of the fields existed still loads.
        let cfg: sdroxide_types::SkimmerSettings =
            serde_json::from_str(r#"{"enabled":[false,false,true]}"#).unwrap();
        let default = sdroxide_types::SkimmerSettings::default();
        assert_eq!(cfg.enabled, [false, false, true]);
        assert_eq!(cfg.squelch_db, default.squelch_db);
        // A `skimmer.json` from before the CW decoder was the operator's choice
        // still opens, on the decoder it used to have.
        assert_eq!(cfg.cw_decoder, sdroxide_types::CwEngine::Neural);
        assert_eq!(cfg.cw_slots, default.cw_slots);
    }

    #[test]
    fn bandstacks_roundtrip_via_json() {
        use sdroxide_types::{Band, BandStackEntry, Mode};
        let mut stacks = BandStacks::default();
        stacks.insert(
            Band::M40,
            vec![BandStackEntry {
                freq_hz: 7_100_000.0,
                mode: Mode::Lsb,
                filter_lo: -2850.0,
                filter_hi: -150.0,
            }],
        );
        let text = serde_json::to_string(&stacks).unwrap();
        let back: BandStacks = serde_json::from_str(&text).unwrap();
        assert_eq!(back, stacks);
    }

    #[test]
    fn session_roundtrips_via_json() {
        let s = Session {
            freq_hz: 7_074_000.0,
            last_good_vfo_hz: 7_074_000.0,
            vfo_b_hz: Some(7_090_000.0),
            active_vfo: sdroxide_types::Vfo::B,
            mode: sdroxide_types::Mode::Ft8,
            // A VFO left in CW while the one in use is on FT8 — the point of
            // remembering a mode per VFO at all.
            vfo_modes: Some([sdroxide_types::Mode::Cw, sdroxide_types::Mode::Ft8]),
            antenna_rx: Some("LNAW".into()),
            antenna_tx: Some("BAND2".into()),
            volume: 0.8,
            muted: true,
            rx_gain_db: 35.0,
            agc: sdroxide_types::AgcMode::Fast,
            drive: 0.4,
            tune_drive: 0.2,
            mic_gain: 0.6,
            cessb_db: 6.0,
            tx_eq: sdroxide_types::TxEqState {
                enabled: true,
                low: sdroxide_types::TxEqBand { freq_hz: 250.0, gain_db: -3.0, q: 0.8 },
                mid: sdroxide_types::TxEqBand { freq_hz: 1800.0, gain_db: 4.0, q: 1.2 },
                high: sdroxide_types::TxEqBand { freq_hz: 3000.0, gain_db: 2.0, q: 0.6 },
            },
            squelch_db: -70.0,
            noise_reduction: sdroxide_types::NrLevel::RnnMed,
            binaural: true,
            decimation: 4,
            repeater: sdroxide_types::RepeaterState {
                shift: sdroxide_types::Shift::Minus,
                offset_hz: 7_600_000,
                auto: true,
                tone: sdroxide_types::ToneMode::Ctcss,
                ctcss_tenths: 1230,
                dcs_code: 131,
                dcs_invert: true,
                burst_auto: true,
                burst_ms: 750,
            },
            gains: vec![("LNA".into(), 24.0), ("VGA".into(), 16.0)],
            tx_gains: vec![("PAD".into(), -6.0)],
            recording_mono: true,
            recording_format: sdroxide_types::RecordingFormat::Wav,
            band_antenna: BandAntennas::from([
                (sdroxide_types::Band::M40, (Some("ANT1".into()), None)),
                (sdroxide_types::Band::M2, (Some("ANT2".into()), Some("ANT2".into()))),
            ]),
            // Both VFOs on 40 m, one on each socket — what the band map above
            // cannot express on its own.
            vfo_antennas: Some([
                (Some("ANT1".into()), None),
                (Some("ANT2".into()), Some("ANT2".into())),
            ]),
            gain_by_band: true,
            band_gains: std::collections::HashMap::from([(
                sdroxide_types::Band::M11,
                vec![("LNA".to_string(), 40.0)],
            )]),
        };
        let back: Session = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back, s);
    }

    /// Every `session.json` written before the levels were remembered has to
    /// keep restoring its dial and mode, and fall back to the levels a fresh
    /// start has always come up at.
    #[test]
    fn a_session_written_before_levels_still_loads() {
        let old: Session =
            serde_json::from_str(r#"{"freq_hz":7074000.0,"mode":"Ft8"}"#).expect("parses");
        let radio = sdroxide_types::RadioState::default();
        assert_eq!(old.volume, radio.rx[0].volume);
        assert_eq!(old.muted, radio.rx[0].muted);
        assert_eq!(old.rx_gain_db, radio.rx[0].manual_gain_db);
        assert_eq!(old.agc, radio.rx[0].agc);
        assert_eq!(old.drive, radio.tx.drive);
        assert_eq!(old.tune_drive, radio.tx.tune_drive);
        assert_eq!(old.mic_gain, radio.tx.mic_gain);
        assert_eq!(old.tx_eq, radio.tx.eq);
        assert_eq!(old.recording_mono, radio.recording_mono);
        // The recording format an older session has no field for: MP3, the
        // only thing that station ever wrote.
        assert_eq!(old.recording_format, radio.recording_format);
        // Likewise the receive settings added after those: an operator upgrading
        // into this comes up squelch-open with NR off, exactly as they always
        // did, and on the front end's own gains rather than on invented ones.
        assert_eq!(old.squelch_db, radio.rx[0].squelch_db);
        assert_eq!(old.noise_reduction, radio.rx[0].noise_reduction);
        assert_eq!(old.binaural, radio.rx[0].binaural);
        assert!(old.gains.is_empty(), "no gain preference until one is expressed");
        assert!(old.tx_gains.is_empty());
        // And it names one dial, which is the one it was left on: B mirrors it,
        // the way a radio that has never had its B set comes up.
        assert_eq!(old.vfo_b_hz, None);
        assert_eq!(old.active_vfo, sdroxide_types::Vfo::A);
        assert_eq!(old.active_dial_hz(), 7_074_000.0);
    }

    /// A hand edit that ruins VFO B costs VFO B, not the whole session: A is
    /// what the receiver opens on, and it is checked separately.
    #[test]
    fn an_unusable_vfo_b_is_dropped_rather_than_restored() {
        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let s = Session { vfo_b_hz: Some(bad), ..Session::default() }.sanitized();
            assert_eq!(s.vfo_b_hz, None, "{bad} should not be accepted as VFO B");
            assert!(s.is_usable(), "and it must not take the rest of the session with it");
        }
        let good = Session { vfo_b_hz: Some(7_100_000.0), ..Session::default() }.sanitized();
        assert_eq!(good.vfo_b_hz, Some(7_100_000.0));
    }

    /// The dial handed to the front end at the next start is the one the
    /// operator was actually working on.
    #[test]
    fn the_restored_dial_is_the_active_vfos() {
        let s = Session {
            freq_hz: 14_200_000.0,
            vfo_b_hz: Some(7_100_000.0),
            active_vfo: sdroxide_types::Vfo::B,
            ..Session::default()
        };
        assert_eq!(s.active_dial_hz(), 7_100_000.0);
        assert_eq!(
            Session { active_vfo: sdroxide_types::Vfo::A, ..s }.active_dial_hz(),
            14_200_000.0
        );
    }

    /// The first run, and every run before this file existed, has to land where
    /// the program has always started.
    #[test]
    fn the_session_default_is_where_the_program_always_started() {
        let s = Session::default();
        assert_eq!(s.freq_hz, 14_200_000.0);
        assert_eq!(s.mode, sdroxide_types::Mode::Usb);
        assert_eq!(s.antenna_rx, None, "no port preference until one is expressed");
        assert_eq!(s.antenna_tx, None);
        // A file missing a key still loads; only what it names is used.
        let partial: Session = serde_json::from_str(r#"{"freq_hz":3573000.0}"#).unwrap();
        assert_eq!(partial.freq_hz, 3_573_000.0);
        assert_eq!(partial.mode, s.mode);
    }

    /// Every `session.json` written before the antennas were remembered has to
    /// keep restoring its dial and mode, and simply express no preference.
    #[test]
    fn a_session_written_before_antennas_still_loads() {
        let old: Session =
            serde_json::from_str(r#"{"freq_hz":7074000.0,"mode":"Ft8"}"#).expect("parses");
        assert_eq!(old.freq_hz, 7_074_000.0);
        assert_eq!(old.mode, sdroxide_types::Mode::Ft8);
        assert_eq!(old.antenna_rx, None);
        assert_eq!(old.antenna_tx, None);
    }

    /// This frequency is handed straight to a front end as its centre, so a
    /// nonsense one has to be dropped rather than passed on: a receiver opening
    /// at 0 Hz or NaN is a much worse failure than a forgotten session.
    #[test]
    fn a_session_frequency_that_is_not_one_is_refused() {
        for bad in [0.0, -14_200_000.0, f64::NAN, f64::INFINITY] {
            let s = Session { freq_hz: bad, ..Session::default() };
            assert!(!s.is_usable(), "{bad} should not be accepted as a dial frequency");
        }
        assert!(Session { freq_hz: 1_840_000.0, ..Session::default() }.is_usable());
        assert!(Session::default().is_usable(), "the fallback must itself be restorable");
    }

    #[test]
    fn default_settings_roundtrip_via_toml() {
        let s = Settings::default();
        let text = toml::to_string_pretty(&s).unwrap();
        let back: Settings = toml::from_str(&text).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn partial_file_fills_defaults() {
        let s: Settings = toml::from_str("sample_rate = 2400000.0").unwrap();
        assert_eq!(s.sample_rate, 2_400_000.0);
        assert_eq!(s.server_port, Settings::default().server_port);
    }

    /// The region has to survive a write, and a `config.toml` written before it
    /// existed has to keep loading — as Region 1, which is the band plan those
    /// installations already have. Getting this wrong would move an operator's
    /// band edges without them touching anything.
    #[test]
    fn the_region_round_trips_and_defaults_to_region_1() {
        for region in sdroxide_types::Region::ALL {
            let s = Settings { region, server_port: 4951, ..Settings::default() };
            let text = toml::to_string_pretty(&s).unwrap();
            let back: Settings = toml::from_str(&text).unwrap();
            assert_eq!(back.region, region, "{text}");
            assert_eq!(back.server_port, 4951, "a table swallowed a value below it");
        }
        let old: Settings = toml::from_str("tx_ham_only = false").unwrap();
        assert_eq!(old.region, sdroxide_types::Region::R1);
    }

    /// The credentials survive a write and a read, and — the part that is easy
    /// to get wrong — every plain value above them is still a plain value
    /// afterwards. TOML puts tables last, so a table declared before
    /// `tx_ham_only` would quietly adopt it into itself and the next start
    /// would come up with the band-edge lockout in a different place.
    #[test]
    fn remote_access_survives_a_write_without_swallowing_the_settings_above_it() {
        let s = Settings {
            remote_access: sdroxide_types::RemoteAccess {
                username: "oe1test".into(),
                password: "hunter2".into(),
            },
            tx_ham_only: false,
            server_port: 4951,
            ..Settings::default()
        };
        let text = toml::to_string_pretty(&s).unwrap();
        let back: Settings = toml::from_str(&text).unwrap();
        assert_eq!(back, s);
        assert_eq!(back.remote_access.password, "hunter2");
        assert!(!back.tx_ham_only, "a value below the table must not become part of it");
        assert_eq!(back.server_port, 4951);
    }

    /// The same hazard, with three tables rather than two: `[speech]` carries
    /// sub-tables of its own, and a scalar of *its* declared after them would
    /// be swallowed just as surely.
    #[test]
    fn speech_settings_survive_a_write_without_swallowing_anything() {
        let mut speech = sdroxide_types::SpeechSettings {
            enabled: true,
            voice: "en_US-hfc_female-medium".into(),
            rate: 1.4,
            verbosity: sdroxide_types::Verbosity::Full,
            ..Default::default()
        };
        speech.cat.filters = true;
        speech.text.cw = true;
        speech.tune.period_s = 3.0;

        let s = Settings { speech: speech.clone(), tx_ham_only: false, ..Settings::default() };
        let text = toml::to_string_pretty(&s).unwrap();
        let back: Settings = toml::from_str(&text).unwrap();
        assert_eq!(back, s);
        assert_eq!(back.speech, speech);
        assert!(back.speech.enabled);
        assert!(back.speech.cat.filters);
        assert!(!back.tx_ham_only, "a value below a table must not become part of it");
        assert_eq!(back.ui, s.ui, "the table above must survive too");
    }

    /// The fourth table, and the one most recently appended: the address the
    /// General tab dials must survive a write, and must not take the scalars
    /// above it with it.
    #[test]
    fn the_remote_server_address_survives_a_write() {
        let s = Settings {
            remote_server: sdroxide_types::RemoteServer {
                host: "shack.local".into(),
                port: 4951,
                tls: true,
            },
            tx_ham_only: false,
            server_port: 4952,
            ..Settings::default()
        };
        let text = toml::to_string_pretty(&s).unwrap();
        let back: Settings = toml::from_str(&text).unwrap();
        assert_eq!(back, s);
        assert_eq!(back.remote_server.host, "shack.local");
        assert_eq!(back.remote_server.port, 4951);
        assert!(back.remote_server.tls, "the secure switch is part of the address (issue #360)");
        assert!(!back.tx_ham_only, "a value below a table must not become part of it");
        assert_eq!(back.server_port, 4952, "the port we listen on is not the one we dial");
        assert_eq!(back.speech, s.speech, "the table above must survive too");
    }

    /// A profile written before a field existed still loads, with that field
    /// at its default — here one with nothing but a name and a dial.
    #[test]
    fn a_profile_missing_fields_still_loads() {
        let p: Profile =
            serde_json::from_str(r#"{"name":"DX","session":{"freq_hz":14025000.0}}"#).unwrap();
        assert_eq!(p.name, "DX");
        assert_eq!(p.session.freq_hz, 14_025_000.0);
        assert!(p.stacks.is_empty());
        assert_eq!(p.digi, sdroxide_types::DigiConfig::default());
    }

    /// The alerts table carries sub-tables of its own, so it gets the same
    /// swallowing test: a write that scatters `[alerts.events.new-dxcc]` must
    /// leave every scalar above it standing.
    #[test]
    fn alerts_settings_survive_a_write_without_swallowing_anything() {
        let mut alerts = sdroxide_types::AlertSettings {
            enabled: true,
            volume: 0.4,
            device: Some("Speakers".into()),
            ..Default::default()
        };
        alerts.events.called.sound = sdroxide_types::AlertSound::Warble;
        alerts.events.new_dxcc.enabled = true;

        let s = Settings { alerts: alerts.clone(), tx_ham_only: false, ..Settings::default() };
        let text = toml::to_string_pretty(&s).unwrap();
        let back: Settings = toml::from_str(&text).unwrap();
        assert_eq!(back, s);
        assert_eq!(back.alerts, alerts);
        assert!(back.alerts.enabled);
        assert_eq!(back.alerts.events.called.sound, sdroxide_types::AlertSound::Warble);
        assert_eq!(back.alerts.volume, 0.4);
        assert!(!back.tx_ham_only, "a value below a table must not become part of it");
        assert_eq!(back.speech, s.speech, "the table above must survive too");
        assert_eq!(back.remote_server, s.remote_server, "the table below must survive too");
    }

    /// A `config.toml` written before this feature existed has no alerts table,
    /// and must come up quiet rather than beeping at its owner.
    #[test]
    fn a_config_without_an_alerts_table_stays_quiet() {
        let s: Settings = toml::from_str("server_port = 4950").unwrap();
        assert!(!s.alerts.enabled);
    }

    /// A `config.toml` written before this feature existed has no address to
    /// dial, and the port box starts on the one every server binds.
    #[test]
    fn a_config_without_a_remote_server_table_has_nowhere_to_go() {
        let s: Settings = toml::from_str("server_port = 4950").unwrap();
        assert!(s.remote_server.host.is_empty());
        assert_eq!(s.remote_server.port, 4950);
        assert!(!s.remote_server.tls, "a link that was plain before must stay plain");
    }

    /// A `config.toml` written before this feature existed comes up silent,
    /// which is what that operator has always had.
    #[test]
    fn a_config_without_a_speech_table_stays_quiet() {
        let s: Settings = toml::from_str("server_port = 4950").unwrap();
        assert!(!s.speech.enabled);
    }

    /// A `config.toml` written before this feature existed leaves the server
    /// open, which is what that operator has always had.
    #[test]
    fn a_config_without_credentials_leaves_the_server_open() {
        let s: Settings = toml::from_str("server_port = 4950").unwrap();
        assert!(!s.remote_access.is_enforced());
    }

    /// A `config.toml` written before the spot label colours were settable
    /// comes up on the stock tints, with the rest of its `[ui]` table intact.
    #[test]
    fn a_config_without_spot_colours_wears_the_stock_tints() {
        use sdroxide_types::SpotKind;
        let s: Settings = toml::from_str("[ui]\nframe_rate_fps = 30\n").unwrap();
        assert_eq!(s.ui.frame_rate_fps, 30, "the rest of the table still applies");
        for kind in SpotKind::ALL {
            let (r, g, b) = kind.default_color();
            assert_eq!(s.ui.spot_colors[kind.index()], [r, g, b]);
        }
    }

    /// A colour list of the wrong width — one written by a build with fewer or
    /// more spot kinds than this one — keeps the entries it does have and
    /// leaves the rest stock. The whole `[ui]` table used to ride on that
    /// length, and with it the operator's theme, fonts and layout.
    #[test]
    fn a_spot_colour_list_of_the_wrong_width_still_loads_the_ui_table() {
        use sdroxide_types::SpotKind;
        let short: Settings =
            toml::from_str("[ui]\nframe_rate_fps = 30\nspot_colors = [[1, 2, 3], [4, 5, 6]]\n")
                .unwrap();
        assert_eq!(short.ui.frame_rate_fps, 30, "the rest of the table still applies");
        assert_eq!(short.ui.spot_colors[0], [1, 2, 3]);
        assert_eq!(short.ui.spot_colors[1], [4, 5, 6]);
        let (r, g, b) = SpotKind::Broadcast.default_color();
        assert_eq!(
            short.ui.spot_colors[SpotKind::Broadcast.index()],
            [r, g, b],
            "a kind the list never reached stays on its stock tint"
        );

        let entries = ["[9, 9, 9]"; SpotKind::COUNT + 2].join(", ");
        let long: Settings =
            toml::from_str(&format!("[ui]\nframe_rate_fps = 30\nspot_colors = [{entries}]\n"))
                .unwrap();
        assert_eq!(long.ui.frame_rate_fps, 30);
        assert_eq!(long.ui.spot_colors, [[9, 9, 9]; SpotKind::COUNT]);
    }

    /// The colours have to survive the round trip through `config.toml` — they
    /// are the first nested array the file carries.
    #[test]
    fn spot_colours_round_trip_through_the_config_file() {
        use sdroxide_types::SpotKind;
        let mut s = Settings::default();
        s.ui.spot_colors[SpotKind::Broadcast.index()] = [10, 20, 30];
        let back: Settings = toml::from_str(&toml::to_string_pretty(&s).unwrap()).unwrap();
        assert_eq!(back.ui.spot_colors, s.ui.spot_colors);
    }

    /// The band-plan strip's shades load and round-trip on the same terms as
    /// the spot tints above — including a list of the wrong width, which is
    /// what an operator gets by upgrading past a build that added a class.
    #[test]
    fn bandplan_colours_load_and_round_trip() {
        use sdroxide_types::BandplanKind;
        let stock: Settings = toml::from_str("[ui]\nframe_rate_fps = 30\n").unwrap();
        assert_eq!(stock.ui.frame_rate_fps, 30, "the rest of the table still applies");
        for kind in BandplanKind::ALL {
            let (r, g, b) = kind.default_color();
            assert_eq!(stock.ui.bandplan_colors[kind.index()], [r, g, b]);
        }

        let short: Settings =
            toml::from_str("[ui]\nframe_rate_fps = 30\nbandplan_colors = [[1, 2, 3]]\n").unwrap();
        assert_eq!(short.ui.frame_rate_fps, 30);
        assert_eq!(short.ui.bandplan_colors[0], [1, 2, 3]);
        let (r, g, b) = BandplanKind::Broadcast.default_color();
        assert_eq!(
            short.ui.bandplan_colors[BandplanKind::Broadcast.index()],
            [r, g, b],
            "a class the list never reached stays on its stock shade"
        );

        let mut s = Settings::default();
        s.ui.bandplan_colors[BandplanKind::Am.index()] = [10, 20, 30];
        let back: Settings = toml::from_str(&toml::to_string_pretty(&s).unwrap()).unwrap();
        assert_eq!(back.ui.bandplan_colors, s.ui.bandplan_colors);
    }

    #[test]
    fn network_config_loads_without_the_freedv_section() {
        // A net.json written before FreeDV Reporter existed.
        let c: sdroxide_types::NetworkConfig =
            serde_json::from_str(r#"{"spot_max_age_secs":600}"#).unwrap();
        assert_eq!(c.spot_max_age_secs, 600);
        assert_eq!(c.freedv_reporter, sdroxide_types::FreeDvReporterConfig::default());
    }

    #[test]
    fn network_config_loads_without_the_swl_identity() {
        // A net.json written before the reception-report identity existed.
        let c: sdroxide_types::NetworkConfig =
            serde_json::from_str(r#"{"spot_max_age_secs":600}"#).unwrap();
        assert_eq!(c.swl_id, "", "a station that never set one reports as its callsign");
    }

    #[test]
    fn network_config_carries_the_swl_identity() {
        let c: sdroxide_types::NetworkConfig =
            serde_json::from_str(r#"{"swl_id":"19SWL001"}"#).unwrap();
        assert_eq!(c.swl_id, "19SWL001");
    }

    #[test]
    fn network_config_ignores_the_retired_operator_identity_keys() {
        // net.json used to hold its own copy of the operator callsign and grid,
        // and the reporter section briefly held a third. All of that now comes
        // from the digi config, so a file still carrying them must load and
        // ignore them rather than fail.
        let c: sdroxide_types::NetworkConfig = serde_json::from_str(
            r#"{"my_call":"AB1CD","my_grid":"FN42","spot_max_age_secs":600,
                "cluster":{"enabled":true,"host":"cluster.example","port":7373},
                "freedv_reporter":{"enabled":true,"callsign":"OLD","grid":"AA00"}}"#,
        )
        .unwrap();
        assert_eq!(c.spot_max_age_secs, 600, "the rest of the file still applies");
        assert!(c.cluster.enabled);
        assert!(c.freedv_reporter.enabled);
    }

    /// One binding naming an action this build no longer has must not take the
    /// rest of the file with it. Removing the Retro Radio faceplate is what made
    /// this reachable: `input.json` is self-describing JSON, so a stored
    /// `"ToggleRetroRadio"` fails the whole struct, and `load_json` on a parse
    /// failure quarantines the file and returns defaults — the operator gets a
    /// keyboard that has forgotten every chord they set. `parse_input_settings`
    /// is the behaviour; this drives it on a string, so nothing here depends on
    /// a process-global config directory.
    #[test]
    fn one_unreadable_binding_does_not_take_the_file_with_it() {
        let mut good = sdroxide_types::InputSettings::default();
        good.keys.truncate(3);
        let text = serde_json::to_string(&good).expect("serialise");
        // Hand-added, as a file written by an older build would carry.
        let stale = r#"{"chord":{"key":"R","ctrl":true,"shift":false,"alt":true},"action":"ToggleRetroRadio","value":1.0,"tuning":{"step":1.0,"accel":0.0,"invert":false},"button":"Toggle","enabled":true}"#;
        let patched = text.replacen("[", &format!("[{stale},"), 1);
        assert_ne!(patched, text, "the stale binding has to be spliced in");

        let back = parse_input_settings(&patched);
        assert_eq!(
            back.keys.len(),
            3,
            "the three readable bindings survive, got {}",
            back.keys.len()
        );
        // The three that survived are *these* three, in order — not merely three
        // that happened to parse.
        for (kept, original) in back.keys.iter().zip(good.keys.iter()) {
            assert_eq!(kept.chord.key, original.chord.key);
            assert_eq!(kept.action.label(), original.action.label());
        }
        // And a file where everything reads takes the strict path unchanged,
        // so this tolerance costs the ordinary case nothing.
        let clean = parse_input_settings(&text);
        assert_eq!(clean.keys.len(), 3);
    }

    /// A scratch directory of our own, so the station-list tests never touch the
    /// operator's real config. No `tempfile` dependency for a handful of tests.
    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("sdroxide-bc-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("scratch dir");
        dir.join(BROADCAST_STATIONS_FILE)
    }

    fn station(name: &str, khz: f64) -> sdroxide_types::BroadcastStation {
        sdroxide_types::BroadcastStation {
            name: name.into(),
            freq_khz: khz,
            source: sdroxide_types::broadcast::ScheduleSource::User,
            site: String::new(),
            country: String::new(),
            lat: None,
            lon: None,
            power_kw: None,
            lang: String::new(),
            target: String::new(),
            mode: None,
            start_utc: None,
            end_utc: None,
            days: String::new(),
            season: None,
            email: String::new(),
            address: String::new(),
        }
    }

    #[test]
    fn an_override_replaces_the_scheduled_row_it_names() {
        let schedule = vec![station("BBC", 15400.0), station("Voice of Greece", 9420.0)];
        let mine = vec![
            // Same name and frequency: a correction, so it wins.
            sdroxide_types::BroadcastStation {
                site: "Woofferton".into(),
                ..station("BBC", 15400.0)
            },
            // Not in the schedule: an addition.
            station("My Local Pirate", 6295.0),
        ];
        let merged = apply_broadcast_overrides(schedule, mine);
        assert_eq!(merged.len(), 3, "the correction replaced rather than duplicated");
        let bbc: Vec<_> = merged.iter().filter(|s| s.name == "BBC").collect();
        assert_eq!(bbc.len(), 1);
        assert_eq!(bbc[0].site, "Woofferton");
        assert!(merged.iter().any(|s| s.name == "My Local Pirate"));
        // Frequency order is what the spot list expects.
        assert!(merged.windows(2).all(|w| w[0].freq_khz <= w[1].freq_khz));
    }

    #[test]
    fn an_override_on_a_different_frequency_is_an_addition() {
        // Same station, another channel — not a correction of the first.
        let merged =
            apply_broadcast_overrides(vec![station("BBC", 15400.0)], vec![station("BBC", 12095.0)]);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn a_seeded_copy_of_a_shipped_schedule_is_retired_not_used() {
        let path = scratch("retire");
        // What an older sdroxide wrote: a generated table, marked with `source`.
        fs::write(
            &path,
            r#"{"version":2,"source":"EiBi A26","stations":[
                 {"name":"Stale Station","freq_khz":6070}]}"#,
        )
        .unwrap();
        retire_seeded_broadcast_list(&path);
        assert!(!path.exists(), "the seeded copy should have been moved aside");
        let backup = path.with_extension("json.bak");
        assert!(backup.exists(), "and kept as .bak, not deleted");
        assert!(fs::read_to_string(&backup).unwrap().contains("Stale Station"));
    }

    #[test]
    fn an_older_seeded_copy_is_recognised_by_its_datestamp() {
        // The first version of this feature seeded a table with `updated` but no
        // `source`. It is still a copy of a shipped list and must not be laid
        // back over a downloaded schedule.
        let path = scratch("retire-dated");
        fs::write(
            &path,
            r#"{"version":1,"updated":"2026-07-30","note":"bundled",
                "stations":[{"name":"Stale","freq_khz":6070}]}"#,
        )
        .unwrap();
        retire_seeded_broadcast_list(&path);
        assert!(!path.exists());
        assert!(path.with_extension("json.bak").exists());
    }

    #[test]
    fn what_the_overrides_writer_produces_is_never_retired() {
        // A file in the shape the manual documents has to survive the next
        // start, or an operator's list would vanish exactly once.
        let path = scratch("roundtrip");
        let file = sdroxide_types::BroadcastStations {
            version: 1,
            updated: String::new(),
            source: String::new(),
            note: "mine".into(),
            stations: vec![station("My Local Pirate", 6295.0)],
        };
        fs::write(&path, serde_json::to_string_pretty(&file).unwrap()).unwrap();
        retire_seeded_broadcast_list(&path);
        assert!(path.exists(), "the operator's own file must survive");
    }

    #[test]
    fn a_hand_written_list_is_left_alone() {
        let path = scratch("keep");
        // No `source` key, so it is the operator's own and must survive.
        let mine = r#"{"version":1,"stations":[{"name":"My Local Pirate","freq_khz":6295}]}"#;
        fs::write(&path, mine).unwrap();
        retire_seeded_broadcast_list(&path);
        assert!(path.exists());
        assert_eq!(fs::read_to_string(&path).unwrap(), mine);
    }

    #[test]
    fn the_cache_path_is_named_for_the_season_and_cannot_escape_it() {
        let a = broadcast_cache_path("a26").unwrap();
        let b = broadcast_cache_path("b26").unwrap();
        assert_ne!(a, b, "a season change has to miss the previous cache");
        assert!(a.to_string_lossy().ends_with("sked-a26.csv"));
        // The season is interpolated into a filename, so nothing in it may walk
        // out of the cache directory even though it is computed, not typed.
        let nasty = broadcast_cache_path("../../etc/passwd").unwrap();
        assert_eq!(nasty.parent(), a.parent());
        assert!(!nasty.to_string_lossy().contains(".."));
    }

    /// A typo in a hand-edited theme name must cost only the theme, never the
    /// whole config: `Settings::load` falls back to full defaults on any parse
    /// error, so the theme/style enums carry a serde catch-all that swallows
    /// unknown values instead of erroring.
    #[test]
    fn an_unknown_theme_degrades_to_the_default_without_losing_the_config() {
        let s: Settings = toml::from_str(
            "sample_rate = 123456.0\n\
             [ui]\n\
             theme = \"HotDogStand\"\n\
             button_style = \"Bouncy\"\n\
             window_style = \"Rounded\"\n",
        )
        .expect("an unknown theme name must still parse");
        assert_eq!(s.sample_rate, 123456.0, "the rest of the config must survive");
        assert_eq!(s.ui.theme, sdroxide_types::UiTheme::Default);
        assert_eq!(s.ui.button_style, sdroxide_types::ChromeStyle::Angled);
        assert_eq!(s.ui.window_style, sdroxide_types::ChromeStyle::Rounded);
    }

    /// And the round trip: what `save_ui_settings` writes, `load` reads back.
    #[test]
    fn theme_and_styles_survive_a_toml_round_trip() {
        let mut s = Settings::default();
        s.ui.theme = sdroxide_types::UiTheme::AmberPhosphor;
        s.ui.button_style = sdroxide_types::ChromeStyle::Bevel;
        s.ui.window_style = sdroxide_types::ChromeStyle::Gradient;
        let text = toml::to_string_pretty(&s).unwrap();
        let back: Settings = toml::from_str(&text).unwrap();
        assert_eq!(back.ui.theme, sdroxide_types::UiTheme::AmberPhosphor);
        assert_eq!(back.ui.button_style, sdroxide_types::ChromeStyle::Bevel);
        assert_eq!(back.ui.window_style, sdroxide_types::ChromeStyle::Gradient);
    }

    /// The S-meter's face rides in `[ui]` for the same reason (issue #185): it
    /// is written the moment the meter is clicked, so it has to come back on
    /// the next start — and a hand-typed face that is not a face must cost the
    /// operator the face, not the whole table.
    #[test]
    fn the_smeter_face_survives_a_toml_round_trip() {
        let mut s = Settings::default();
        assert_eq!(s.ui.smeter_style, sdroxide_types::SmeterStyle::Needle, "stock face");
        s.ui.smeter_style = sdroxide_types::SmeterStyle::Trace;
        let text = toml::to_string_pretty(&s).unwrap();
        let back: Settings = toml::from_str(&text).unwrap();
        assert_eq!(back.ui.smeter_style, sdroxide_types::SmeterStyle::Trace);

        let typo: Settings = toml::from_str(
            "[ui]\n\
             theme = \"AmberPhosphor\"\n\
             smeter_style = \"Dial\"\n",
        )
        .expect("an unknown face must still parse");
        assert_eq!(typo.ui.theme, sdroxide_types::UiTheme::AmberPhosphor, "the table survived");
        assert_eq!(typo.ui.smeter_style, sdroxide_types::SmeterStyle::Needle);
    }

    /// The FT8/FT4 decode list's own view preferences — the sort, its
    /// direction, the grouping and the two filters — ride in `[ui]` beside the
    /// theme, so they have to survive the same trip: a chip clicked in the
    /// panel is written the moment it is pressed and has to come back on the
    /// next start.
    #[test]
    fn the_decode_list_view_survives_a_toml_round_trip() {
        let mut s = Settings::default();
        s.ui.decode_sort = sdroxide_types::DecodeSort::Country;
        s.ui.decode_sort_desc = false;
        s.ui.decode_single_list = true;
        s.ui.decode_cq_only = true;
        s.ui.decode_new_only = true;
        let text = toml::to_string_pretty(&s).unwrap();
        let back: Settings = toml::from_str(&text).unwrap();
        assert_eq!(back.ui.decode_sort, sdroxide_types::DecodeSort::Country);
        assert!(!back.ui.decode_sort_desc);
        assert!(back.ui.decode_single_list);
        assert!(back.ui.decode_cq_only);
        assert!(back.ui.decode_new_only);
    }

    /// A config written before these existed — or one with a hand-typed sort
    /// name that is not a sort — loads with the list unsorted and every other
    /// preference intact, rather than throwing the whole `[ui]` table away.
    #[test]
    fn an_unknown_decode_sort_degrades_to_no_sorting() {
        let s: Settings = toml::from_str(
            "[ui]\n\
             theme = \"AmberPhosphor\"\n\
             decode_sort = \"Loudest\"\n",
        )
        .expect("an unknown sort name must still parse");
        assert_eq!(s.ui.decode_sort, sdroxide_types::DecodeSort::None);
        assert_eq!(s.ui.theme, sdroxide_types::UiTheme::AmberPhosphor);
        let old: Settings = toml::from_str("[ui]\ntheme = \"AmberPhosphor\"\n").unwrap();
        assert_eq!(old.ui.decode_sort, sdroxide_types::DecodeSort::None);
        assert!(!old.ui.decode_cq_only, "a config from before these must not filter the list");
    }

    /// The fork's directory is upstream's with `-brown` appended, on both the
    /// Linux (`…/sdroxide`) and macOS (`…/org.sdroxide.sdroxide`) layouts — the
    /// name is appended to the whole path, not a component, so both work.
    #[test]
    fn the_brown_dir_appends_to_the_whole_name() {
        let base = std::path::Path::new("/home/u/.config/sdroxide");
        let name = base.file_name().map(|n| format!("{}-brown", n.to_string_lossy())).unwrap();
        assert_eq!(
            base.with_file_name(name),
            std::path::Path::new("/home/u/.config/sdroxide-brown")
        );

        let mac =
            std::path::Path::new("/Users/u/Library/Application Support/org.sdroxide.sdroxide");
        let name = mac.file_name().map(|n| format!("{}-brown", n.to_string_lossy())).unwrap();
        assert_eq!(
            mac.with_file_name(name),
            std::path::Path::new(
                "/Users/u/Library/Application Support/org.sdroxide.sdroxide-brown"
            )
        );
    }

    /// The migration's copier takes the whole tree and leaves the source alone —
    /// the property the "copy, not move" rule rests on.
    #[test]
    fn copying_a_config_tree_leaves_the_original_in_place() {
        let base = std::env::temp_dir().join(format!("sdroxide-copy-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let from = base.join("from");
        let to = base.join("to");
        fs::create_dir_all(from.join("radio-1")).unwrap();
        fs::write(from.join("config.toml"), "sample_rate = 1000.0\n").unwrap();
        fs::write(from.join("radio-1/radio.json"), "{}\n").unwrap();

        copy_dir_recursive(&from, &to).unwrap();
        assert_eq!(fs::read_to_string(to.join("config.toml")).unwrap(), "sample_rate = 1000.0\n");
        assert!(to.join("radio-1/radio.json").exists(), "subdirectories come too");
        assert!(from.join("config.toml").exists(), "the source must be left alone");
        assert!(from.join("radio-1/radio.json").exists());
        let _ = fs::remove_dir_all(&base);
    }

    /// One test rather than several, because it redirects the config directory
    /// through the environment — process-global state that must not race the
    /// other tests in this binary (none of which touch `config_dir`).
    #[test]
    fn radio_scopes_map_under_the_config_dir_and_radio_zero_is_the_root() {
        let root = std::env::temp_dir().join(format!("sdroxide-store-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("scratch dir");
        // SAFETY: single-threaded within this test; no other test in this
        // binary reads the variable.
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &root) };

        // Radio 0 is the legacy installation: its files stay at the root.
        assert_eq!(Store::radio(0).dir().unwrap(), root);
        assert_eq!(Store::station().dir().unwrap(), root);
        assert_eq!(Store::radio(2).dir().unwrap(), root.join("radio-2"));

        // A scoped write lands in the scope, resolves on read, and leaves the
        // root's file alone.
        let mut cfg = sdroxide_types::RadioConfig::default();
        cfg.converter_offset_hz = 120_000_000.0;
        Store::radio(2).save_radio_config(&cfg).unwrap();
        assert!(root.join("radio-2/radio.json").exists());
        assert!(!root.join("radio.json").exists());
        assert_eq!(Store::radio(2).load_radio_config(), cfg);
        assert_eq!(
            Store::radio(0).load_radio_config(),
            sdroxide_types::RadioConfig::default(),
            "radio 0 must not see radio 2's file"
        );

        // The roster: a missing file is the one legacy radio; creating a radio
        // allocates a fresh id, seeds its scope, and disables the TCI server
        // there (whose default is enabled — a guaranteed port collision on any
        // radio but the first).
        let roster = load_radios();
        assert_eq!(roster.radios, vec![RadioSlot { id: 0, name: String::new(), enabled: true }]);
        assert_eq!(roster.next_id, 1);
        let slot = create_radio("").unwrap();
        assert_eq!(slot.id, 1);
        assert_eq!(slot.name, "", "no name until the operator types one — the UI derives it");
        let roster = load_radios();
        assert_eq!(roster.radios.len(), 2);
        assert_eq!(roster.next_id, 2);

        // Renaming records what was typed; renaming to nothing goes back to
        // the derived default rather than pinning an empty label.
        rename_radio(slot.id, "Bench 2 m").unwrap();
        assert_eq!(load_radios().radios[1].name, "Bench 2 m");
        rename_radio(slot.id, "").unwrap();
        assert_eq!(load_radios().radios[1].name, "");
        assert!(
            !Store::radio(slot.id).load_tci_server_config().enabled,
            "a new radio's TCI server must come up disabled"
        );
        assert!(sdroxide_types::TciServerConfig::default().enabled, "or this seeding is moot");

        // The on/off switch. A radio is on until somebody says otherwise, the
        // answer survives a reload, and switching it off is nothing but that
        // one field — the scope, and everything configured in it, is untouched.
        assert!(load_radios().is_enabled(slot.id));
        set_radio_enabled(slot.id, false).unwrap();
        assert!(!load_radios().radios[1].enabled);
        assert!(!load_radios().is_enabled(slot.id));
        assert!(load_radios().is_enabled(0), "one radio's switch is not another's");
        assert!(
            Store::radio(slot.id).dir().unwrap().join("radio.json").exists(),
            "switching a radio off must not touch what it is configured as"
        );
        set_radio_enabled(slot.id, true).unwrap();
        assert!(load_radios().is_enabled(slot.id));

        // A roster written before the switch existed — every radio in it is on.
        fs::write(root.join("radios.json"), r#"{"radios":[{"id":0,"name":"Shack"}],"next_id":1}"#)
            .unwrap();
        let legacy = load_radios();
        assert_eq!(legacy.radios, vec![RadioSlot { id: 0, name: "Shack".into(), enabled: true }]);
        assert!(legacy.is_enabled(7), "a radio the roster never heard of is not switched off");
        save_radios(&roster).unwrap();

        // Closing the tab keeps the scope directory: a closed radio is not a
        // destroyed configuration.
        remove_radio(slot.id).unwrap();
        assert_eq!(load_radios().radios.len(), 1);
        assert!(root.join("radio-1/radio.json").exists());

        // An id must never be handed out twice, even after a removal.
        let again = create_radio("Bench RX").unwrap();
        assert_eq!(again.id, 2, "id 1 was used once; it stays used");
        assert_eq!(again.name, "Bench RX");

        // The strip's order (issue #224). Nothing said yet is the roster's own
        // order; what the operator arranges survives a reload and leaves the
        // roster — and so the station's first radio — exactly where it was.
        let third = create_radio("Bench RX 2").unwrap();
        assert_eq!(load_radios().display_order(), vec![0, again.id, third.id]);
        reorder_radios(&[third.id, 0, again.id]).unwrap();
        let after = load_radios();
        assert_eq!(after.display_order(), vec![third.id, 0, again.id]);
        assert_eq!(
            after.radios.iter().map(|r| r.id).collect::<Vec<_>>(),
            vec![0, again.id, third.id],
            "the roster itself must not be rearranged: its first radio is the station",
        );

        // A stale order — one that names a radio that has since been closed,
        // and misses one that has since been added — is still a permutation of
        // the roster, with the newcomer at the end.
        remove_radio(third.id).unwrap();
        let fourth = create_radio("Bench RX 3").unwrap();
        assert_eq!(load_radios().display_order(), vec![0, again.id, fourth.id]);
        // And the file never grows an id the roster does not have.
        reorder_radios(&[fourth.id, third.id, 0, again.id]).unwrap();
        assert_eq!(load_radios().order, vec![fourth.id, 0, again.id]);
        remove_radio(again.id).unwrap();
        remove_radio(fourth.id).unwrap();

        // A radio.json that does not parse (truncated by a crash mid-write,
        // disk full, hand edit gone wrong) silently stripping the operator's
        // whole setup is how a field RSP1B "forgot" its pinned serial. The
        // load still falls back — startup must never fail on config — but it
        // falls back to *no interface at all* rather than to the default one,
        // the broken file is kept as evidence, and an alert is queued. See
        // `Store::load_radio_config` and `tests/quarantine.rs` for why the
        // default backend is the wrong place to land.
        let _ = take_load_alerts();
        fs::write(root.join("radio.json"), "{ \"backend\": \"Sdr").unwrap();
        let reset = sdroxide_types::RadioConfig {
            backend: sdroxide_types::Backend::None,
            ..Default::default()
        };
        assert_eq!(Store::station().load_radio_config(), reset);
        assert!(root.join("radio.json.bak").exists(), "the broken file must be kept");
        let alerts = take_load_alerts();
        assert_eq!(alerts.len(), 1);
        assert!(alerts[0].contains("radio.json.bak"), "{}", alerts[0]);
        // The fallback is written back, so "nothing is selected" survives a
        // restart. Without this the next start finds no file, reads that as a
        // first run, and goes straight back to the default backend — which is
        // the state the operator was actually stuck in.
        assert!(root.join("radio.json").exists(), "the safe fallback was not recorded");
        assert_eq!(Store::station().load_radio_config(), reset);
        // Once: the rename leaves nothing for the next load to complain about.
        assert!(take_load_alerts().is_empty());

        // Saves go through a sibling temp file + rename, so a crash mid-write
        // leaves the previous file rather than a truncated one — and the temp
        // name must not linger.
        Store::station().save_radio_config(&cfg).unwrap();
        assert_eq!(Store::station().load_radio_config(), cfg);
        assert!(!root.join("radio.json.tmp").exists(), "the temp file is renamed away");

        // ── The band plan ───────────────────────────────────────────────────
        // Folded into this test rather than given its own, for the reason in
        // the doc comment above: only one test in this binary may redirect the
        // config directory.
        let plan_path = root.join(BAND_PLAN_FILE);
        assert!(!plan_path.exists(), "nothing has written it yet");

        // First load seeds the file from the built-in tables, and what lands on
        // disk has to be the plan that was returned — otherwise the operator's
        // starting point is not what the program is running.
        let seeded = load_band_plan();
        assert!(seeded.is_default());
        assert!(plan_path.exists(), "a first load must write a starting file");
        assert!(take_load_alerts().is_empty(), "seeding is not a problem to report");
        assert_eq!(load_band_plan(), seeded, "the second load reads the file it wrote");
        assert_eq!(band_plan_path().unwrap(), plan_path);

        // The seeded file is editable: MHz, and one row per line rather than
        // `to_string_pretty`'s five, which is the difference between a file an
        // operator can read down and nine hundred lines of scrolling.
        let text = fs::read_to_string(&plan_path).unwrap();
        assert!(text.contains("\"lo_mhz\""), "the file has to be in megahertz");
        assert!(text.contains("\"readme\""), "and has to explain itself");
        assert!(
            text.contains(r#"{"band": "M160", "lo_mhz": 1.81, "hi_mhz": 2.0}"#),
            "a band should be one readable line:\n{text}"
        );
        assert!(text.lines().count() < 300, "{} lines is too many to edit", text.lines().count());
        assert!(text.ends_with('\n'), "a text file ends with a newline");
        fs::write(
            &plan_path,
            r#"{"region1":{"bands":[{"band":"M2","lo_mhz":144.0,"hi_mhz":144.4}]},
                "region2":{"bands":[{"band":"M2","lo_mhz":144.0,"hi_mhz":148.0}]},
                "region3":{"bands":[{"band":"M2","lo_mhz":144.0,"hi_mhz":148.0}]}}"#,
        )
        .unwrap();
        let mine = load_band_plan();
        assert!(!mine.is_default());
        assert_eq!(
            mine.region(sdroxide_types::Region::R1).edges(sdroxide_types::Band::M2),
            Some((144_000_000.0, 144_400_000.0))
        );

        // A row that says nothing is dropped, named, and does not take the file
        // down with it.
        fs::write(
            &plan_path,
            r#"{"region1":{"bands":[
                    {"band":"M2","lo_mhz":148.0,"hi_mhz":144.0},
                    {"band":"M20","lo_mhz":14.0,"hi_mhz":14.35}]},
                "region2":{"bands":[{"band":"M20","lo_mhz":14.0,"hi_mhz":14.35}]},
                "region3":{"bands":[{"band":"M20","lo_mhz":14.0,"hi_mhz":14.35}]}}"#,
        )
        .unwrap();
        let patched = load_band_plan();
        assert_eq!(
            patched.region(sdroxide_types::Region::R1).edges(sdroxide_types::Band::M2),
            None
        );
        let alerts = take_load_alerts();
        assert_eq!(alerts.len(), 1, "{alerts:?}");
        assert!(alerts[0].contains("2M"), "{}", alerts[0]);

        // A file that will not parse is *left alone* — unlike `radio.json`,
        // which is quarantined. This one is the operator's own document, and a
        // half-finished edit must survive the start that failed to read it.
        fs::write(&plan_path, "{ \"region1\": ").unwrap();
        assert!(load_band_plan().is_default(), "a broken file falls back to the built-ins");
        assert!(plan_path.exists(), "the operator's file must not be renamed away");
        assert!(!root.join(format!("{BAND_PLAN_FILE}.bak")).exists());
        let alerts = take_load_alerts();
        assert_eq!(alerts.len(), 1, "{alerts:?}");
        assert!(alerts[0].contains("built-in"), "{}", alerts[0]);
        // And it says so again next time, because the file is still wrong.
        assert!(load_band_plan().is_default());
        assert_eq!(take_load_alerts().len(), 1);

        // Recordings follow the isolation too, whatever the setting says: a
        // recording is the one file that would otherwise escape to the
        // operator's real music folder.
        save_recordings_dir(Some(std::path::Path::new("/tmp/elsewhere"))).unwrap();
        assert_eq!(
            Settings::load().recordings_dir.as_deref(),
            Some(std::path::Path::new("/tmp/elsewhere")),
            "the setting round-trips"
        );
        assert_eq!(
            recordings_dir().unwrap(),
            root.join("recordings"),
            "an isolated session ignores it and stays in the sandbox"
        );

        // The operator's own HFDL channels: a save writes the file and a load
        // reads it back, and a missing one is simply empty (not an error).
        assert!(load_hfdl_channels().is_empty(), "no file, no saved channels");
        save_hfdl_channels(&[5_451, 8_843, 11_184]).unwrap();
        assert!(root.join("hfdl_channels.json").exists());
        assert_eq!(load_hfdl_channels(), vec![5_451, 8_843, 11_184]);

        unsafe { std::env::remove_var("SDROXIDE_CONFIG_DIR") };
    }

    /// The recordings-directory choice: the setting wins, an empty one is "no
    /// setting", and without either the music folder's `sdroxide` is the home.
    #[test]
    fn a_configured_recordings_dir_wins_over_the_music_folder() {
        let music = || Some(PathBuf::from("/home/u/Music"));
        assert_eq!(
            pick_recordings_dir(Some(std::path::Path::new("/data/air")), music()).unwrap(),
            PathBuf::from("/data/air")
        );
        assert_eq!(
            pick_recordings_dir(Some(std::path::Path::new("")), music()).unwrap(),
            PathBuf::from("/home/u/Music/sdroxide"),
            "an empty path is 'no setting', not the filesystem root"
        );
        assert_eq!(
            pick_recordings_dir(None, music()).unwrap(),
            PathBuf::from("/home/u/Music/sdroxide")
        );
    }

    #[test]
    fn a_short_download_is_not_a_schedule() {
        // The guard that stops a captive portal's login page replacing the real
        // list. One row parses fine; it just is not a season's worth.
        let csv = "kHz;Time;Days;ITU;Station;Lng;Target;Remarks;P;Start;Stop;\n\
                   9420;0000-2400;;GRC;Voice of Greece;G;Eu;a;1;;\n";
        let parsed = sdroxide_types::broadcast::parse_schedule(csv);
        assert_eq!(parsed.len(), 1);
        assert!(parsed.len() < MIN_SCHEDULE_ROWS);
        // Whereas the compiled-in fallback comfortably clears the bar.
        assert!(sdroxide_types::broadcast::builtin().len() > MIN_SCHEDULE_ROWS);
    }
}

#[cfg(test)]
mod aoki_cache_roundtrip {
    use super::{cached_text_from_bytes, decode_schedule};
    use sdroxide_types::ScheduleSource;

    /// Aoki's **published** schedule, verbatim from the fixture the parser's own
    /// tests use — not a header typed in here, which is how a test ends up
    /// pinning an invention.
    const AOKI_SCHEDULE: &str = include_str!("../../sdroxide-types/src/aoki-fixture.txt");

    /// **The cache holds what was downloaded, so the reader has to undo it.**
    /// Aoki's is a zip and EiBi's is the CSV itself; a reader that only decodes
    /// bytes gets Aoki's *archive* rather than its schedule. This is pinned on
    /// the behaviour rather than on the bytes, because the bytes are the thing
    /// that is easy to get subtly wrong and hard to read: what a listener gets is
    /// either a full schedule or an empty one.
    #[test]
    fn a_cached_aoki_zip_is_unpacked_rather_than_decoded() {
        let zip =
            zip_of(&[("userlist1.txt", "listener's own list\r\n"), ("ncb24.txt", AOKI_SCHEDULE)]);

        let good = sdroxide_types::broadcast::parse_from(
            ScheduleSource::Aoki,
            &cached_text_from_bytes(ScheduleSource::Aoki, "24", &zip).expect("unpacks"),
        )
        .expect("parses");
        assert!(
            good.len() > 20,
            "the packed path yields the published schedule, not nothing (got {})",
            good.len()
        );

        // The way it was read before: decoded, not unpacked.
        let naive = decode_schedule(ScheduleSource::Aoki, &zip);
        let parsed =
            sdroxide_types::broadcast::parse_from(ScheduleSource::Aoki, &naive).unwrap_or_default();
        assert!(
            parsed.is_empty(),
            "decoding the archive as text is the fault, and it yields {} rows",
            parsed.len()
        );
    }

    /// EiBi is plain Latin-1 text, so one helper for both sources is only safe
    /// if that is genuinely the identity there.
    #[test]
    fn a_cached_eibi_csv_is_still_read_as_itself() {
        let csv = b"Freq;Station;Language\r\n15400;BBC;English\r\n";
        assert_eq!(
            cached_text_from_bytes(ScheduleSource::Eibi, "24", csv).unwrap(),
            String::from_utf8_lossy(csv)
        );
    }

    /// A minimal **deflated** zip, like Aoki's real archive.
    ///
    /// Stored entries would defeat the test: their bytes sit verbatim in the file,
    /// so a reader that merely *decoded* the archive would find the schedule text
    /// by accident and the fault would be invisible — the test would pass for the
    /// wrong reason. Compressed is what the reader actually has to unpack.
    fn zip_of(entries: &[(&str, &str)]) -> Vec<u8> {
        use std::io::Write;
        let mut out = Vec::new();
        let mut dir = Vec::new();
        for (name, data) in entries {
            let off = out.len() as u32;
            let b = data.as_bytes();
            // flate2's `write::DeflateEncoder` already emits **raw** deflate,
            // which is what a zip entry is — the `Zlib*` types are the wrapped
            // ones. Symmetric with the reader's `read::DeflateDecoder`.
            let mut enc =
                flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::new(6));
            enc.write_all(b).unwrap();
            let comp = enc.finish().unwrap();
            out.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04]);
            out.extend_from_slice(&[20, 0, 0, 0, 8, 0, 0, 0, 0, 0]);
            out.extend_from_slice(&crc32(b).to_le_bytes());
            out.extend_from_slice(&(comp.len() as u32).to_le_bytes());
            out.extend_from_slice(&(b.len() as u32).to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(&comp);

            dir.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02]);
            dir.extend_from_slice(&[20, 0, 20, 0, 0, 0, 8, 0, 0, 0, 0, 0]);
            dir.extend_from_slice(&crc32(b).to_le_bytes());
            dir.extend_from_slice(&(comp.len() as u32).to_le_bytes());
            dir.extend_from_slice(&(b.len() as u32).to_le_bytes());
            dir.extend_from_slice(&(name.len() as u16).to_le_bytes());
            dir.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            dir.extend_from_slice(&off.to_le_bytes());
            dir.extend_from_slice(name.as_bytes());
        }
        let (start, len) = (out.len() as u32, dir.len() as u32);
        out.extend_from_slice(&dir);
        out.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06]);
        out.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&start.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out
    }

    fn crc32(data: &[u8]) -> u32 {
        let mut c = 0xffff_ffffu32;
        for b in data {
            c ^= *b as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 { (c >> 1) ^ 0xedb8_8320 } else { c >> 1 };
            }
        }
        !c
    }
}

#[cfg(test)]
mod own_list_import {
    use super::{own_frequency_khz, parse_own_list};

    /// A listener's notes are not a data format, so the frequency may be written
    /// either way round and in either unit — and **where no unit is written the
    /// number decides**, because asking somebody to convert a spreadsheet before
    /// they can use it is the thing that stops them.
    #[test]
    fn a_frequency_is_read_in_whichever_unit_the_number_implies() {
        assert_eq!(own_frequency_khz("15400"), Some(15_400.0));
        assert_eq!(own_frequency_khz("15400 kHz"), Some(15_400.0));
        assert_eq!(own_frequency_khz("15.4"), Some(15_400.0), "a small number is MHz");
        assert_eq!(own_frequency_khz("0.153"), Some(153.0), "an LF frequency");
        assert_eq!(own_frequency_khz("999"), None, "999 MHz is not shortwave");
        assert_eq!(own_frequency_khz("BBC"), None, "not a number at all");
    }

    /// Both shapes, because both are what people actually have.
    #[test]
    fn a_plain_list_and_a_csv_both_import() {
        let plain = parse_own_list("BBC World Service 15400 kHz\nRadio Prague 5010\n");
        assert_eq!(plain.len(), 2);
        assert_eq!(plain[0].name, "BBC World Service", "not \"BBC World Service kHz\"");
        assert_eq!(plain[0].freq_khz, 15_400.0);
        assert_eq!(plain[1].name, "Radio Prague");
        assert_eq!(plain[1].freq_khz, 5_010.0);

        let csv = parse_own_list("15400,BBC World Service,English\n5010,Radio Prague\n");
        assert_eq!(csv.len(), 2);
        assert_eq!(csv[0].freq_khz, 15_400.0);
        // The remaining cells are the name, space-joined: a CSV row is
        // `frequency, name…, anything else`, and the trailing columns are kept
        // rather than dropped, because a listener's notes may put the language or
        // a remark there.
        assert_eq!(csv[0].name, "BBC World Service English");
    }

    /// Comments and blanks are not stations, and a line with no frequency in it is
    /// **skipped rather than half-read** — a list that quietly gains a row called
    /// "my notes" is worse than one that misses three.
    ///
    /// Deliberately *not* a line mentioning a bare number: "check the 6 MHz band"
    /// holds a token that reads as 6000 kHz, and the parser is right to take it.
    /// The test says what it means rather than pretending otherwise.
    #[test]
    fn lines_without_a_frequency_are_skipped() {
        let rows = parse_own_list("# my stations\n\nBBC 15400\nremember to check this later\n");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "BBC");
    }

    /// Everything imported is **the listener's own**, whatever the file said — the
    /// tag is what the Schedule window's filter reads.
    #[test]
    fn an_imported_row_is_tagged_as_the_listeners_own() {
        let rows = parse_own_list("BBC 15400\n");
        assert_eq!(rows[0].source, sdroxide_types::ScheduleSource::User);
    }
}
