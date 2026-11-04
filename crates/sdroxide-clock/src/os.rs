//! The operating system's clock: reading it, finding out whether we may set
//! it, and setting it.
//!
//! # Moving, not setting
//!
//! Everything here takes an *offset* — "move the clock this far" — and never a
//! time to set it to. The clock is read and written back with the offset
//! added in two consecutive calls, so whatever happened before the call does
//! not matter. That is the point: when the system asks the operator for a
//! password first, the dialog may sit there for a minute, and a time computed
//! before it would be a minute out by the time it was set. An offset is as
//! right after the dialog as before it.
//!
//! # Privilege
//!
//! Setting the clock needs `CAP_SYS_TIME` on Linux, root on macOS, and
//! `SeSystemtimePrivilege` on Windows (an elevated administrator, or a
//! service account). A process that has it sets the clock itself. One that
//! does not asks the system's own administrator prompt to run this same
//! binary with that privilege, with [`HELPER_FLAG`] and the offset —
//! `pkexec` on Linux, an AppleScript `do shell script … with administrator
//! privileges` on macOS, UAC on Windows. The helper does that one thing and
//! exits before anything else in the program starts (see [`helper_main`]),
//! so nothing else ever runs with the privilege.

use std::time::{SystemTime, UNIX_EPOCH};

use sdroxide_types::ClockAuthority;

/// The command-line flag that makes the binary the root helper.
pub const HELPER_FLAG: &str = "--set-clock-offset-ns";

/// The largest step the helper will make. Four hundred days: further than any
/// real clock is out, and a bound on what a mangled argument can do.
pub const MAX_STEP_NS: i64 = 400 * 86_400 * 1_000_000_000;

/// The computer's clock, nanoseconds since the Unix epoch.
pub fn now_ns() -> i128 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_nanos() as i128,
        Err(e) => -(e.duration().as_nanos() as i128),
    }
}

/// [`now_ns`] for a time already in hand.
pub fn system_time_ns(t: SystemTime) -> i128 {
    match t.duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_nanos() as i128,
        Err(e) => -(e.duration().as_nanos() as i128),
    }
}

/// How this process may set the clock.
pub fn authority() -> ClockAuthority {
    if may_set_directly() {
        ClockAuthority::Direct
    } else if prompt_available() {
        ClockAuthority::Prompt
    } else {
        ClockAuthority::Unavailable
    }
}

#[cfg(unix)]
fn may_set_directly() -> bool {
    // SAFETY: geteuid has no preconditions and cannot fail.
    let euid = unsafe { libc::geteuid() };
    euid == 0 || has_cap_sys_time()
}

#[cfg(windows)]
fn may_set_directly() -> bool {
    crate::windows::may_set_directly()
}

#[cfg(not(any(unix, windows)))]
fn may_set_directly() -> bool {
    false
}

/// Linux: whether the effective capability set holds `CAP_SYS_TIME` (25).
#[cfg(target_os = "linux")]
fn has_cap_sys_time() -> bool {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find_map(|l| l.strip_prefix("CapEff:"))
                .and_then(|v| u64::from_str_radix(v.trim(), 16).ok())
        })
        .is_some_and(|caps| caps & (1 << 25) != 0)
}

#[cfg(all(unix, not(target_os = "linux")))]
fn has_cap_sys_time() -> bool {
    false
}

/// Whether there is an administrator prompt to ask through: a desktop
/// session with the system's tool for it. A headless server has neither, and
/// a prompt nobody can see would only hang.
#[cfg(target_os = "linux")]
fn prompt_available() -> bool {
    let desktop = ["DISPLAY", "WAYLAND_DISPLAY"].iter().any(|v| std::env::var_os(v).is_some());
    desktop && find_in_path("pkexec").is_some()
}

#[cfg(target_os = "macos")]
fn prompt_available() -> bool {
    // An SSH login has no window server to put the dialog on.
    std::env::var_os("SSH_CONNECTION").is_none()
        && std::path::Path::new("/usr/bin/osascript").exists()
}

#[cfg(windows)]
fn prompt_available() -> bool {
    crate::windows::prompt_available()
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn prompt_available() -> bool {
    false
}

#[cfg(target_os = "linux")]
fn find_in_path(name: &str) -> Option<std::path::PathBuf> {
    std::env::var_os("PATH")?
        .to_str()?
        .split(':')
        .map(|d| std::path::Path::new(d).join(name))
        .find(|p| p.exists())
}

/// Moves the clock by `offset_ns`, from this process. Needs
/// [`ClockAuthority::Direct`].
#[cfg(unix)]
pub fn step_direct(offset_ns: i64) -> std::io::Result<()> {
    if offset_ns.unsigned_abs() > MAX_STEP_NS as u64 {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "step too large"));
    }
    let mut tv = libc::timeval { tv_sec: 0, tv_usec: 0 };
    // SAFETY: both calls are given a valid timeval and a null timezone, as
    // POSIX requires; nothing else is read or written.
    unsafe {
        if libc::gettimeofday(&mut tv, std::ptr::null_mut()) != 0 {
            return Err(std::io::Error::last_os_error());
        }
        let us = i128::from(tv.tv_sec) * 1_000_000
            + i128::from(tv.tv_usec)
            + i128::from(offset_ns).div_euclid(1000);
        tv.tv_sec = us.div_euclid(1_000_000) as libc::time_t;
        tv.tv_usec = us.rem_euclid(1_000_000) as libc::suseconds_t;
        if libc::settimeofday(&tv, std::ptr::null()) != 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(())
}

/// Moves the clock by `offset_ns`, from this process. Needs
/// [`ClockAuthority::Direct`].
#[cfg(windows)]
pub fn step_direct(offset_ns: i64) -> std::io::Result<()> {
    if offset_ns.unsigned_abs() > MAX_STEP_NS as u64 {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "step too large"));
    }
    crate::windows::step(offset_ns)
}

#[cfg(not(any(unix, windows)))]
pub fn step_direct(_offset_ns: i64) -> std::io::Result<()> {
    Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "not supported on this system"))
}

/// Moves the clock by `offset_ns` through the administrator prompt. Blocks
/// until the operator has answered it, so it belongs on a thread of its own.
pub fn step_elevated(offset_ns: i64) -> Result<(), String> {
    let exe = helper_exe(std::env::var_os("APPIMAGE"), std::env::current_exe())?;
    elevated(&exe, offset_ns)
}

/// The program the administrator prompt is asked to run as the helper.
///
/// Normally this one. Inside an AppImage, though, this one is
/// `/tmp/.mount_XXXX/usr/bin/sdroxide` on a FUSE mount that only the user
/// who started it may enter — not even root, without `allow_other` — so the
/// helper would never start. The AppImage file itself is an ordinary file
/// root can run, and its runtime sets `APPIMAGE` to its path; its AppRun hands
/// the arguments through, so the helper flag arrives the same.
fn helper_exe(
    appimage: Option<std::ffi::OsString>,
    current: std::io::Result<std::path::PathBuf>,
) -> Result<String, String> {
    let exe = match appimage.map(std::path::PathBuf::from) {
        Some(p) if p.is_absolute() && p.is_file() => p,
        _ => current.map_err(|e| format!("cannot find this program: {e}"))?,
    };
    exe.to_str().map(str::to_owned).ok_or_else(|| "this program's path is not valid text".into())
}

#[cfg(target_os = "linux")]
fn elevated(exe: &str, offset_ns: i64) -> Result<(), String> {
    let out = std::process::Command::new("pkexec")
        .arg(exe)
        .arg(HELPER_FLAG)
        .arg(offset_ns.to_string())
        .output()
        .map_err(|e| format!("could not run pkexec: {e}"))?;
    match out.status.code() {
        Some(0) => Ok(()),
        // pkexec(1): 126 when the dialog was dismissed, 127 when not
        // authorised.
        Some(126) => Err("the administrator password was not given".into()),
        Some(127) => Err("not authorised to set the clock".into()),
        _ => Err(helper_error(&out.stderr)),
    }
}

#[cfg(target_os = "macos")]
fn elevated(exe: &str, offset_ns: i64) -> Result<(), String> {
    // An AppleScript string literal: backslashes and quotes escaped. `quoted
    // form of` then makes it one shell word whatever the path holds.
    let lit = |s: &str| format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""));
    let script = format!(
        "do shell script (quoted form of {}) & \" {} {}\" with prompt {} with administrator privileges",
        lit(exe),
        HELPER_FLAG,
        offset_ns,
        lit("sdroxide wants to set the clock to the time received by radio."),
    );
    let out = std::process::Command::new("/usr/bin/osascript")
        .arg("-e")
        .arg(script)
        .output()
        .map_err(|e| format!("could not run osascript: {e}"))?;
    if out.status.success() {
        return Ok(());
    }
    let err = String::from_utf8_lossy(&out.stderr);
    if err.contains("-128") {
        Err("the administrator password was not given".into())
    } else {
        Err(helper_error(&out.stderr))
    }
}

#[cfg(windows)]
fn elevated(exe: &str, offset_ns: i64) -> Result<(), String> {
    crate::windows::elevated(exe, offset_ns)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn elevated(_exe: &str, _offset_ns: i64) -> Result<(), String> {
    Err("setting the clock is not supported on this system".into())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn helper_error(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let line = text.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("no reason given");
    format!("the clock was not set: {}", line.trim())
}

/// Something on this machine that keeps the clock itself and would put it
/// back after we set it, named for the panel.
pub fn sync_daemon() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        const DAEMONS: [&str; 4] = ["systemd-timesyn", "chronyd", "ntpd", "openntpd"];
        let found = std::fs::read_dir("/proc").ok()?.flatten().find_map(|e| {
            let comm = std::fs::read_to_string(e.path().join("comm")).ok()?;
            DAEMONS.contains(&comm.trim()).then(|| comm.trim().to_owned())
        })?;
        Some(if found == "systemd-timesyn" { "systemd-timesyncd".into() } else { found })
    }
    #[cfg(target_os = "macos")]
    {
        // timed always runs; whether it sets the clock is the "Set time and
        // date automatically" switch, which only an administrator can read.
        Some("timed, if \"Set time and date automatically\" is on".into())
    }
    #[cfg(windows)]
    {
        crate::windows::sync_daemon()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    {
        None
    }
}

/// The root helper's entry point. Call first thing in `main`, before logging
/// or anything else starts: if this process was started as the helper it
/// returns the exit code to leave with, having stepped the clock or said why
/// not, and otherwise `None` and nothing has happened.
pub fn helper_main() -> Option<i32> {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() != Some(HELPER_FLAG) {
        return None;
    }
    let Some(Ok(offset)) = args.next().map(|a| a.parse::<i64>()) else {
        eprintln!("{HELPER_FLAG} takes one whole number of nanoseconds");
        return Some(2);
    };
    if args.next().is_some() || offset.unsigned_abs() > MAX_STEP_NS as u64 {
        eprintln!("refusing: one offset of at most {} days", MAX_STEP_NS / 86_400_000_000_000);
        return Some(2);
    }
    match step_direct(offset) {
        Ok(()) => Some(0),
        Err(e) => {
            eprintln!("{e}");
            Some(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_appimage_asks_for_itself_not_its_mount() {
        let dir = std::env::temp_dir().join(format!("sdroxide-clock-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let image = dir.join("sdroxide.AppImage");
        std::fs::write(&image, b"").unwrap();
        let mount = Ok(std::path::PathBuf::from("/tmp/.mount_abc/usr/bin/sdroxide"));
        let got = helper_exe(Some(image.clone().into()), mount);
        assert_eq!(got.as_deref(), Ok(image.to_str().unwrap()));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn without_an_appimage_it_is_this_program() {
        let exe = std::path::PathBuf::from("/usr/bin/sdroxide");
        assert_eq!(helper_exe(None, Ok(exe.clone())).as_deref(), Ok("/usr/bin/sdroxide"));
        // A stale or relative APPIMAGE is not believed.
        assert_eq!(
            helper_exe(Some("relative.AppImage".into()), Ok(exe.clone())).as_deref(),
            Ok("/usr/bin/sdroxide")
        );
        assert_eq!(
            helper_exe(Some("/nonexistent/x.AppImage".into()), Ok(exe)).as_deref(),
            Ok("/usr/bin/sdroxide")
        );
    }
}
