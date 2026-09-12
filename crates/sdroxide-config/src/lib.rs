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
    /// Preferred audio input (microphone) device name; `None` = system default.
    pub audio_input: Option<String>,
    /// The published version the operator dismissed the update banner for.
    /// The banner stays away until sdroxide.com names a newer version than
    /// the running build that is also not this one. Empty = nothing dismissed.
    pub dismissed_update: String,
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
            audio_input: None,
            dismissed_update: String::new(),
            region: sdroxide_types::Region::default(),
            ui: sdroxide_types::UiSettings::default(),
            remote_access: sdroxide_types::RemoteAccess::default(),
            speech: sdroxide_types::SpeechSettings::default(),
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
    s.ui = *ui;
    s.save()
}

/// Load just the station's IARU region.
pub fn load_region() -> sdroxide_types::Region {
    Settings::load().region
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

/// Load the server this screen last dialled from Settings → Remote.
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

pub fn config_dir() -> Result<PathBuf, ConfigError> {
    // The override exists for the integration tests, which must not write the
    // operator's real configuration, and works as a profile switch for anyone
    // who wants two independent installations.
    if let Some(dir) = std::env::var_os("SDROXIDE_CONFIG_DIR") {
        if !dir.is_empty() {
            return Ok(PathBuf::from(dir));
        }
    }
    directories::ProjectDirs::from("org", "sdroxide", "sdroxide")
        .map(|d| d.config_dir().to_path_buf())
        .ok_or(ConfigError::NoConfigDir)
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
/// `session.json`, `scanner.json`, `tciserver.json`, `rigctld.json`,
/// `wsjtx.json`. Everything the operator shares across radios — memories,
/// band stacks, the logbook, `config.toml` — stays on the root free functions.
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
        let s: Session = self.load("session.json");
        if s.is_usable() { s.sanitized() } else { Session::default() }
    }

    pub fn save_session(&self, session: &Session) -> Result<(), ConfigError> {
        self.save("session.json", session)
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

/// Directory for audio recordings, created on demand: the user's music/audio
/// directory (`<Music>/sdroxide`), or the config directory
/// (`~/.config/sdroxide/recordings`) when the platform exposes no music folder.
pub fn recordings_dir() -> Result<PathBuf, ConfigError> {
    // An isolated configuration is isolated: `SDROXIDE_CONFIG_DIR` is how a
    // test, a second station or a throwaway session says "keep everything
    // here", and a recording written into the operator's real music folder
    // from one of those is the one file that escapes. Checked here rather than
    // inside `config_dir` because the two answer different questions — that one
    // is where settings live, and it has an isolated form already.
    if std::env::var_os("SDROXIDE_CONFIG_DIR").is_some() {
        let dir = config_dir()?.join("recordings");
        fs::create_dir_all(&dir)?;
        return Ok(dir);
    }
    let dir = match directories::UserDirs::new()
        .and_then(|u| u.audio_dir().map(std::path::Path::to_path_buf))
    {
        Some(music) => music.join("sdroxide"),
        None => config_dir()?.join("recordings"),
    };
    fs::create_dir_all(&dir)?;
    Ok(dir)
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
    /// Whether a recording mixes RX/TX down to one channel instead of putting
    /// RX left and TX right. A preference the operator sets once and expects to
    /// still hold next time, not something the engine moves on its own — but it
    /// rides here rather than in `config.toml` because the UI is the only thing
    /// that sets it, and the engine is what owns writing it back.
    pub recording_mono: bool,
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
            band_antenna: BandAntennas::new(),
            // And no socket of its own for either VFO until one has been
            // chosen on it — nothing moves a relay before the operator has
            // said what belongs where.
            vfo_antennas: None,
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

pub fn load_vdl2_config() -> sdroxide_types::Vdl2Settings {
    load_json::<sdroxide_types::Vdl2Settings>("vdl2.json").sane()
}

pub fn save_vdl2_config(cfg: &sdroxide_types::Vdl2Settings) -> Result<(), ConfigError> {
    save_json("vdl2.json", cfg)
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
    load_json("input.json")
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
    let season = sdroxide_types::broadcast::season_file(now_unix());
    let path = broadcast_cache_path(&season).ok()?;
    let bytes = fs::read(&path).ok()?;
    let text = sdroxide_types::broadcast::decode_latin1(&bytes);
    let stations = sdroxide_types::broadcast::parse_schedule(&text);
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
    let season = sdroxide_types::broadcast::season_file(now_unix());
    match broadcast_cache_path(&season) {
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

/// Download the current season's schedule and cache it.
///
/// Blocking, so callers put it on a worker thread. Returns the merged station
/// list on success. The download is written to the cache only after it parses
/// into a plausible schedule, so a failure leaves the previous file in place
/// rather than replacing it with a captive portal's login page.
pub fn fetch_broadcast_schedule() -> Result<Vec<sdroxide_types::BroadcastStation>, String> {
    let season = sdroxide_types::broadcast::season_file(now_unix());
    let url = EIBI_SKED_URL.replace("{season}", &season);
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(std::time::Duration::from_secs(15)))
        .timeout_global(Some(std::time::Duration::from_secs(120)))
        .user_agent(concat!("sdroxide/", env!("CARGO_PKG_VERSION")))
        .build()
        .into();

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

    let text = sdroxide_types::broadcast::decode_latin1(&bytes);
    let stations = sdroxide_types::broadcast::parse_schedule(&text);
    if stations.len() < MIN_SCHEDULE_ROWS {
        return Err(format!(
            "{url} yielded {} transmissions, which is not a schedule",
            stations.len()
        ));
    }

    let path = broadcast_cache_path(&season).map_err(|e| e.to_string())?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    // Last season's file is only dead weight once this one has landed.
    prune_broadcast_cache(&season);
    info!("downloaded the {season} broadcast schedule: {} transmissions", stations.len());

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

/// Where the released version is read from: the script that drives the
/// download page at sdroxide.com, which holds it in a `RELEASE` constant at
/// the top. Reading the page's own source of truth means the check can never
/// disagree with what the download buttons would actually hand out.
const UPDATE_CHECK_URL: &str = "https://sdroxide.com/script.js";

/// The released version according to sdroxide.com, or `None` when the site
/// is unreachable or the script no longer parses. Blocking, so callers put
/// it on a worker thread. A query parameter that changes every start defeats
/// any cache between here and the server, so a new release is seen the day
/// it ships rather than when a proxy's copy expires.
pub fn fetch_published_version() -> Option<String> {
    let url = format!("{UPDATE_CHECK_URL}?cb={}", now_unix());
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(std::time::Duration::from_secs(15)))
        .timeout_global(Some(std::time::Duration::from_secs(30)))
        .user_agent(concat!("sdroxide/", env!("CARGO_PKG_VERSION")))
        .build()
        .into();
    let mut resp = agent.get(&url).call().ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let text = resp.body_mut().with_config().limit(1024 * 1024).read_to_string().ok()?;
    parse_published_version(&text)
}

/// Pull the version out of the script's `RELEASE` constant, which reads
/// `version: "1.3.2",`. The first `version:` key with a quoted value
/// immediately after it wins; the value must look like a version number, so
/// an error page or a rewritten script yields `None` rather than a banner
/// announcing an "update" to whatever was scraped.
fn parse_published_version(js: &str) -> Option<String> {
    for (at, _) in js.match_indices("version:") {
        let rest = js[at + "version:".len()..].trim_start();
        let Some(rest) = rest.strip_prefix('"') else { continue };
        let Some(close) = rest.find('"') else { continue };
        let v = &rest[..close];
        if !v.is_empty()
            && v.chars().all(|c| c.is_ascii_digit() || c == '.')
            && v.chars().next().is_some_and(|c| c.is_ascii_digit())
        {
            return Some(v.to_string());
        }
    }
    None
}

/// Whether `candidate` is a strictly newer version than `current`, comparing
/// dotted components numerically — so `1.10.0` beats `1.9.9`, and a build
/// ahead of the published release (this machine running `1.4.0` while the
/// site still says `1.3.2`) is *not* offered its own past as an update.
/// Missing components count as zero, so `1.4` and `1.4.0` are the same.
pub fn version_is_newer(candidate: &str, current: &str) -> bool {
    let parts = |v: &str| -> Vec<u64> {
        v.split('.').map(|p| p.trim().parse::<u64>().unwrap_or(0)).collect()
    };
    let (a, b) = (parts(candidate), parts(current));
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(i).copied().unwrap_or(0), b.get(i).copied().unwrap_or(0));
        if x != y {
            return x > y;
        }
    }
    false
}

/// The published version the operator last dismissed the update banner for.
pub fn load_dismissed_update() -> String {
    Settings::load().dismissed_update
}

/// Remember a dismissed update, preserving every other setting
/// (read-modify-write, like [`save_ui_settings`]).
pub fn save_dismissed_update(version: &str) -> Result<(), ConfigError> {
    let mut s = Settings::load();
    s.dismissed_update = version.to_string();
    s.save()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn published_version_parses_from_release_constant() {
        // The shape script.js actually has: prose mentioning "version" above
        // the constant, then the RELEASE object.
        let js = r#"
            /* Everything on the page (version pills, download links,
               filenames) is derived from this object. */
            const RELEASE = {
              repo: "dividebysandwich/sdroxide",
              version: "1.3.2",   // <-- change me for a new release
              tag: "v1.3.2",
            };
        "#;
        assert_eq!(parse_published_version(js), Some("1.3.2".to_string()));
        // An error page, a script without the constant, and a quoted value
        // that is not a version number all yield nothing.
        assert_eq!(parse_published_version("<html>404</html>"), None);
        assert_eq!(parse_published_version(r#"version: "v1.3.2""#), None);
        assert_eq!(parse_published_version("version: 1.3.2"), None);
    }

    #[test]
    fn version_is_newer_compares_numerically() {
        assert!(version_is_newer("1.4.0", "1.3.2"));
        assert!(version_is_newer("1.10.0", "1.9.9"));
        assert!(version_is_newer("2.0", "1.9.9"));
        // Equal — including a missing trailing component — is not newer.
        assert!(!version_is_newer("1.4.0", "1.4.0"));
        assert!(!version_is_newer("1.4", "1.4.0"));
        // A build ahead of the published release is not offered its own past.
        assert!(!version_is_newer("1.3.2", "1.4.0"));
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
    /// Remote tab dials must survive a write, and must not take the scalars
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

        unsafe { std::env::remove_var("SDROXIDE_CONFIG_DIR") };
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
