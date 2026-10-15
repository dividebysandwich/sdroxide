//! Spawn and feed an rs1729 `*mod` binary (OpenWebRX ExecModule shape).
//!
//! Argv: `{bin} --IQ <frac> --lpIQ [options…] --json - {rate} 32`, stdin =
//! interleaved little-endian `f32` I/Q, stdout = JSON lines. `<frac>` is the
//! carrier offset as a fraction of the sample rate (`offset_hz / rate`).
//!
//! Binary search order:
//! 1. `SDROXIDE_SONDE_MOD_DIR`
//! 2. Directory of the running `sdroxide` executable (release packages)
//! 3. Build-time stage from feature `sonde` (`SDROXIDE_SONDE_MOD_BUILD`)
//! 4. `PATH`

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use sdroxide_dsp::Complex32;
use sdroxide_types::SondeKind;
use tracing::{debug, info, warn};

use crate::json::parse_mod_json;
use crate::package::Package;

/// Running `*mod` process plus a stdout reader thread that pushes packages.
pub struct ModProc {
    kind: SondeKind,
    child: Child,
    stdin: ChildStdin,
    packages: Arc<Mutex<Vec<Package>>>,
    frames: Arc<std::sync::atomic::AtomicU64>,
    bad: Arc<std::sync::atomic::AtomicU64>,
    stop: Arc<AtomicBool>,
    reader: Option<JoinHandle<()>>,
    stderr_reader: Option<JoinHandle<()>>,
}

impl ModProc {
    /// Resolve and start the helper for `kind` tuned `offset_hz` from the lane
    /// centre. `Err` is a human message for the panel's `unavailable` string.
    pub fn start(kind: SondeKind, rate_hz: f64, offset_hz: f64) -> Result<ModProc, String> {
        let bin_path = resolve_binary(kind).ok_or_else(|| missing_msg(kind))?;
        let bin_label = bin_path.file_name().and_then(|s| s.to_str()).unwrap_or(kind.binary());
        let rate = rate_hz.round().max(1.0) as u32;
        // `--IQ <frac>` is relative to Fs (−0.5…0.5). `--lpIQ` keeps the IF
        // low-pass on that carrier so a neighbour a few kHz away does not
        // steal the demodulator.
        let frac = (offset_hz / rate_hz.max(1.0)).clamp(-0.45, 0.45);
        let mut args: Vec<String> =
            vec!["--IQ".into(), format!("{frac:.6}"), "--lpIQ".into()];
        match kind {
            SondeKind::Rs41 => args.push("--ptu2".into()),
            SondeKind::Dfm9 => args.push("--ptu".into()),
            SondeKind::Dfm17 => {
                args.push("-i".into());
                args.push("--ptu".into());
            }
            SondeKind::M10 | SondeKind::M20 => args.push("--ptu".into()),
            SondeKind::Mts01 => {}
        }
        args.push("--json".into());
        args.push("-".into());
        args.push(rate.to_string());
        args.push("32".into());

        let mut cmd = Command::new(&bin_path);
        cmd.args(&args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
        // Avoid a console window flash when the GUI spawns *mod on Windows.
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("could not start {}: {e}", bin_path.display()))?;

        let stdin = child.stdin.take().ok_or_else(|| format!("{bin_label}: no stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| format!("{bin_label}: no stdout"))?;
        let stderr = child.stderr.take().ok_or_else(|| format!("{bin_label}: no stderr"))?;

        let packages = Arc::new(Mutex::new(Vec::new()));
        let frames = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let bad = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let stop = Arc::new(AtomicBool::new(false));

        let packages_t = Arc::clone(&packages);
        let frames_t = Arc::clone(&frames);
        let bad_t = Arc::clone(&bad);
        let stop_t = Arc::clone(&stop);
        let reader = std::thread::Builder::new()
            .name(format!("sdroxide-{bin_label}-out"))
            .spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    if stop_t.load(Ordering::Relaxed) {
                        break;
                    }
                    let Ok(line) = line else { break };
                    if line.trim().is_empty() {
                        continue;
                    }
                    match parse_mod_json(&line, kind) {
                        Some(pkg) => {
                            frames_t.fetch_add(1, Ordering::Relaxed);
                            if let Ok(mut q) = packages_t.lock() {
                                q.push(pkg);
                            }
                        }
                        None => {
                            bad_t.fetch_add(1, Ordering::Relaxed);
                            debug!(%line, "sonde *mod line skipped");
                        }
                    }
                }
            })
            .map_err(|e| format!("spawn *mod reader: {e}"))?;

        let stop_err = Arc::clone(&stop);
        let stderr_reader = std::thread::Builder::new()
            .name(format!("sdroxide-{bin_label}-err"))
            .spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    if stop_err.load(Ordering::Relaxed) {
                        break;
                    }
                    let Ok(line) = line else { break };
                    if !line.trim().is_empty() {
                        debug!(%line, "sonde *mod stderr");
                    }
                }
            })
            .map_err(|e| format!("spawn *mod stderr reader: {e}"))?;

        info!(
            bin = %bin_path.display(),
            rate,
            offset_hz,
            iq_frac = frac,
            ?kind,
            "sonde *mod started"
        );
        Ok(ModProc {
            kind,
            child,
            stdin,
            packages,
            frames,
            bad,
            stop,
            reader: Some(reader),
            stderr_reader: Some(stderr_reader),
        })
    }

    /// Write interleaved f32 I/Q. Drops the block if the pipe is full / broken.
    pub fn write_iq(&mut self, iq: &[Complex32]) {
        if iq.is_empty() {
            return;
        }
        let mut bytes = Vec::with_capacity(iq.len() * 8);
        for c in iq {
            bytes.extend_from_slice(&c.re.to_le_bytes());
            bytes.extend_from_slice(&c.im.to_le_bytes());
        }
        if let Err(e) = self.stdin.write_all(&bytes) {
            warn!(error = %e, "sonde *mod stdin write failed");
        }
    }

    pub fn drain_packages(&mut self) -> Vec<Package> {
        self.packages.lock().map(|mut q| std::mem::take(&mut *q)).unwrap_or_default()
    }

    pub fn frames(&self) -> u64 {
        self.frames.load(Ordering::Relaxed)
    }

    pub fn bad(&self) -> u64 {
        self.bad.load(Ordering::Relaxed)
    }

    /// True when the child has exited (broken pipe / crash).
    pub fn alive(&mut self) -> bool {
        match self.child.try_wait() {
            Ok(None) => true,
            Ok(Some(status)) => {
                warn!(?status, bin = self.kind.binary(), "sonde *mod exited");
                false
            }
            Err(e) => {
                warn!(error = %e, "sonde *mod try_wait");
                false
            }
        }
    }
}

impl Drop for ModProc {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.stdin.flush();
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(h) = self.reader.take() {
            let _ = h.join();
        }
        if let Some(h) = self.stderr_reader.take() {
            let _ = h.join();
        }
        info!(bin = self.kind.binary(), "sonde *mod stopped");
    }
}

/// Whether a helper for `kind` is resolvable.
pub fn binary_on_path(kind: SondeKind) -> bool {
    resolve_binary(kind).is_some()
}

fn missing_msg(kind: SondeKind) -> String {
    let name = kind.binary();
    if cfg!(feature = "sonde") {
        format!(
            "missing {name} — rebuild with `--features sonde` (default) or put \
             rs1729 helpers on PATH / SDROXIDE_SONDE_MOD_DIR"
        )
    } else {
        format!(
            "this build omitted sonde helpers — rebuild with `--features sonde`, \
             or install {name} on PATH"
        )
    }
}

fn resolve_binary(kind: SondeKind) -> Option<PathBuf> {
    let names = candidate_names(kind);
    for dir in search_dirs() {
        for name in &names {
            let p = dir.join(name);
            if p.is_file() {
                return Some(p);
            }
            #[cfg(windows)]
            {
                let exe = dir.join(format!("{name}.exe"));
                if exe.is_file() {
                    return Some(exe);
                }
            }
        }
    }
    for name in &names {
        if let Some(p) = which(name) {
            return Some(p);
        }
    }
    None
}

fn candidate_names(kind: SondeKind) -> Vec<&'static str> {
    match kind {
        // Release stage also copies m10m20mod → m20mod for OpenWebRX naming.
        SondeKind::M20 => vec!["m20mod", "m10m20mod"],
        other => vec![other.binary()],
    }
}

fn search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(dir) = std::env::var("SDROXIDE_SONDE_MOD_DIR") {
        let p = PathBuf::from(dir.trim());
        if p.is_dir() {
            dirs.push(p);
        }
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        dirs.push(parent.to_path_buf());
        // Optional nested folder in a tarball layout.
        let nested = parent.join("sonde-mod");
        if nested.is_dir() {
            dirs.push(nested);
        }
    }
    let build = env!("SDROXIDE_SONDE_MOD_BUILD");
    if !build.is_empty() {
        let p = PathBuf::from(build);
        if p.is_dir() {
            dirs.push(p);
        }
    }
    dirs
}

fn which(bin: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(bin);
        if candidate.is_file() {
            return Some(candidate);
        }
        #[cfg(windows)]
        {
            let with_exe = dir.join(format!("{bin}.exe"));
            if with_exe.is_file() {
                return Some(with_exe);
            }
        }
    }
    None
}
