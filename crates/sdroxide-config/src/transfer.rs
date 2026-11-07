//! Moving a station's settings between installations, as one file.
//!
//! Everything sdroxide remembers lives as a handful of small files under
//! [`crate::config_dir`], which is fine until an operator has two of them: a
//! second machine, a club callsign beside a personal one, a rebuilt laptop.
//! Copying that by hand means knowing which files there are and where the
//! directory is on three different platforms, and doing it from screenshots
//! means not copying it at all (issue #356).
//!
//! # What travels
//!
//! Every settings file in the config directory and in each radio's
//! `radio-<n>/` subdirectory, carried **verbatim** — the text as it is on the
//! disk, not a re-serialisation of a parsed value. A bundle written by a newer
//! sdroxide therefore keeps settings this one has never heard of, and a field
//! added between two versions is not quietly dropped in transit.
//!
//! Two things deliberately do not travel, and both for the same reason: they
//! are not settings.
//!
//! * **The logbook.** [`SKIP`] leaves `qso_log.json` where it is. An operator
//!   moving settings between two callsigns wants their setup on both and their
//!   contacts kept apart, which is exactly what issue #356 asks for; a log that
//!   really is to be moved has ADIF, which every other program reads too.
//!   `wavelog_sync.json` stays with it: it says how much of a Wavelog has been
//!   downloaded *into this log*, and is wrong for any other.
//! * **A saved sign-in.** `remote_login.json` holds a password for a server
//!   *this* machine connects to, and a settings bundle is a file people email
//!   each other.
//!
//! # What an import may write
//!
//! A bundle is data from somewhere else, so [`import`] treats every path in it
//! as untrusted: a name has to be a plain file name with a known extension,
//! optionally under a single `radio-<n>/` directory, and anything else is
//! skipped and reported rather than written. Nothing can therefore be made to
//! land outside the config directory, whatever a hand-edited bundle says.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{ConfigError, config_dir, write_atomic};

/// The marker every bundle carries, so a file that is not one is refused with
/// an explanation rather than half-applied.
const MAGIC: &str = "sdroxide-settings";

/// Bundle format version. Bumped only for a change an older reader could not
/// cope with; the file list itself is free to grow.
const FORMAT: u32 = 1;

/// Extensions a settings file may have. Everything else in the config
/// directory — the SSTV pictures, the cached broadcast schedules, the speech
/// voices, the quarantined `.bak` files — is data or cache, not settings.
const EXTENSIONS: &[&str] = &["json", "toml", "conf"];

/// Files that are in the config directory, look like settings, and are not.
/// See the module header for why each is here.
const SKIP: &[&str] = &["qso_log.json", "wavelog_sync.json", "remote_login.json"];

/// A station's settings, as one file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bundle {
    /// Always [`MAGIC`]. The first thing [`import`] checks.
    pub kind: String,
    pub format: u32,
    /// The sdroxide that wrote it, for a human reading the file.
    pub version: String,
    /// When it was written, unix seconds. `0` if the clock could not be read.
    pub exported_unix: i64,
    /// Relative path (`config.toml`, `radio-1/radio.json`) to the file's text,
    /// exactly as it was on the disk.
    pub files: std::collections::BTreeMap<String, String>,
}

impl Bundle {
    /// A one-line summary for the operator, before and after.
    pub fn summary(&self) -> String {
        let radios = self.files.keys().filter_map(|k| radio_dir_of(k)).max().map_or(1, |n| n + 1);
        format!(
            "{} settings file(s) for {radios} radio(s), from sdroxide {}",
            self.files.len(),
            self.version,
        )
    }
}

/// Which radio subdirectory a bundle path is under, if any.
fn radio_dir_of(path: &str) -> Option<u32> {
    path.split_once('/')
        .and_then(|(dir, _)| dir.strip_prefix("radio-"))
        .and_then(|n| n.parse().ok())
}

/// Whether `name` is a settings file this bundle carries.
fn is_settings_file(name: &str) -> bool {
    !SKIP.contains(&name)
        && Path::new(name)
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| EXTENSIONS.contains(&e))
}

/// Every settings file in `dir`, as `(name, text)`, sorted.
///
/// A file that cannot be read is left out rather than failing the export: one
/// unreadable file should not cost the operator the other twenty.
fn read_dir_settings(dir: &Path) -> Vec<(String, String)> {
    let Ok(entries) = fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<(String, String)> = entries
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            is_settings_file(&name).then(|| Some((name, fs::read_to_string(e.path()).ok()?)))?
        })
        .collect();
    out.sort();
    out
}

/// Collect this station's settings into a bundle.
pub fn export() -> Result<Bundle, ConfigError> {
    let root = config_dir()?;
    let mut files = std::collections::BTreeMap::new();
    for (name, text) in read_dir_settings(&root) {
        files.insert(name, text);
    }
    // Each radio's own subdirectory, by the name `Store::dir` gives it, so a
    // station with four radios exports all four.
    let radios = fs::read_dir(&root)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            radio_dir_of(&format!("{name}/x")).map(|_| name)
        });
    for radio in radios {
        for (name, text) in read_dir_settings(&root.join(&radio)) {
            files.insert(format!("{radio}/{name}"), text);
        }
    }
    Ok(Bundle {
        kind: MAGIC.to_string(),
        format: FORMAT,
        version: env!("CARGO_PKG_VERSION").to_string(),
        exported_unix: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0),
        files,
    })
}

/// A bundle as the file that gets written: pretty-printed, so it can be read
/// and edited like every other file sdroxide keeps.
pub fn export_json() -> Result<String, ConfigError> {
    let bundle = export()?;
    Ok(serde_json::to_string_pretty(&bundle).expect("a bundle of strings always serializes"))
}

/// What an import did.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ImportReport {
    /// Files written, by their relative path.
    pub written: Vec<String>,
    /// Entries the bundle carried that were not written, and why. A skipped
    /// entry is never a silent one: the operator is told, because a setting
    /// they expected to arrive and did not is the whole point of importing.
    pub skipped: Vec<(String, String)>,
}

impl ImportReport {
    pub fn summary(&self) -> String {
        let mut s = format!("{} settings file(s) restored", self.written.len());
        if !self.skipped.is_empty() {
            s.push_str(&format!("; {} skipped", self.skipped.len()));
        }
        s
    }
}

/// Where a bundle entry may be written, or why it may not be.
///
/// The whole of the trust boundary. A bundle arrives from another machine, and
/// a path in it is a string somebody else wrote: `..` in it, an absolute path,
/// a nested directory, an extension that is not a settings file's. Each is
/// refused by name here rather than being caught downstream by the filesystem,
/// so the refusal can say which entry it was and why.
fn destination(path: &str) -> Result<(Option<String>, String), String> {
    let (dir, name) = match path.split_once('/') {
        None => (None, path),
        Some((dir, name)) => {
            let Some(n) = dir.strip_prefix("radio-").and_then(|n| n.parse::<u32>().ok()) else {
                return Err("not a settings file or a radio-<n> directory".into());
            };
            // `radio-1/a/b` splits to ("radio-1", "a/b"), which the name check
            // below refuses — but say the real reason rather than that one.
            if name.contains('/') {
                return Err("nested directories are not part of a settings bundle".into());
            }
            (Some(format!("radio-{n}")), name)
        }
    };
    if name.is_empty()
        || name.starts_with('.')
        || !name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    {
        return Err("not a plain settings file name".into());
    }
    if !is_settings_file(name) {
        return Err("not a settings file this version carries".into());
    }
    Ok((dir, name.to_string()))
}

/// Restore a bundle over this station's settings.
///
/// Every file it carries replaces the one here; anything it does not mention is
/// left alone, so importing a bundle from a station with one radio does not
/// delete a second radio's configuration on this one.
///
/// The caller has to restart sdroxide afterwards. Nothing here reaches into a
/// running engine — the settings it is holding in memory were read at startup
/// and would be written straight back over these on the next save.
pub fn import(json: &str) -> Result<ImportReport, ConfigError> {
    let bundle: Bundle = serde_json::from_str(json).map_err(|e| {
        ConfigError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("this is not an sdroxide settings file: {e}"),
        ))
    })?;
    if bundle.kind != MAGIC {
        return Err(ConfigError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("this is a {:?} file, not an sdroxide settings export", bundle.kind),
        )));
    }
    if bundle.format > FORMAT {
        return Err(ConfigError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "this settings file is version {} and this sdroxide reads version {FORMAT} — \
                 update sdroxide to import it",
                bundle.format
            ),
        )));
    }
    let root = config_dir()?;
    let mut report = ImportReport::default();
    for (path, text) in &bundle.files {
        match destination(path) {
            Err(why) => report.skipped.push((path.clone(), why)),
            Ok((dir, name)) => {
                let target = match &dir {
                    Some(d) => root.join(d),
                    None => root.clone(),
                };
                match write_atomic(&target, &name, text) {
                    Ok(()) => report.written.push(path.clone()),
                    Err(e) => report.skipped.push((path.clone(), e.to_string())),
                }
            }
        }
    }
    Ok(report)
}
