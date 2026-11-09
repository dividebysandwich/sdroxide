//! Windows: the clock, the privilege to set it, and UAC.
//!
//! Setting the clock needs `SeSystemtimePrivilege`. An administrator's
//! elevated token holds it, as do the service accounts; the filtered token UAC
//! gives an administrator's ordinary programs does not, and neither does a
//! standard user's. A token that holds it still has to *enable* it before
//! `SetSystemTime` will work, which is what [`step`] does first.
//!
//! Without it, [`elevated`] asks UAC to start this same binary elevated with
//! the helper flag — `ShellExecuteExW` with the `runas` verb — and waits for
//! it. UAC's own dialog asks for consent, or for an administrator's password
//! on a standard account; it cannot be shown from session 0, where services
//! run, which is how [`prompt_available`] tells a desktop from a service.
//!
//! The clock is moved as everywhere else: read, offset added, written back.
//! `SetSystemTime` takes whole milliseconds, so the target is rounded to the
//! nearest, which leaves the clock within half a millisecond of the aim — well
//! inside what the measurement itself is good to.

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_CANCELLED, ERROR_NOT_ALL_ASSIGNED, FILETIME, GetLastError, HANDLE, LUID,
    SYSTEMTIME, WAIT_OBJECT_0,
};
use windows_sys::Win32::Security::{
    AdjustTokenPrivileges, GetTokenInformation, LUID_AND_ATTRIBUTES, LookupPrivilegeValueW,
    SE_PRIVILEGE_ENABLED, SE_SYSTEMTIME_NAME, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES,
    TOKEN_QUERY, TokenPrivileges,
};
use windows_sys::Win32::System::RemoteDesktop::ProcessIdToSessionId;
use windows_sys::Win32::System::Services::{
    CloseServiceHandle, OpenSCManagerW, OpenServiceW, QueryServiceStatus, SC_MANAGER_CONNECT,
    SERVICE_QUERY_STATUS, SERVICE_RUNNING, SERVICE_STATUS,
};
use windows_sys::Win32::System::SystemInformation::{
    GetSystemTimePreciseAsFileTime, SetSystemTime,
};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentProcessId, GetExitCodeProcess, INFINITE, OpenProcessToken,
    WaitForSingleObject,
};
use windows_sys::Win32::System::Time::FileTimeToSystemTime;
use windows_sys::Win32::UI::Shell::{
    SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
    ShellExecuteExW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;

use crate::os::HELPER_FLAG;

/// A NUL-terminated UTF-16 copy of `s`, for the wide-string APIs.
fn wide(s: impl AsRef<OsStr>) -> Vec<u16> {
    s.as_ref().encode_wide().chain(std::iter::once(0)).collect()
}

/// This process's token, closed when dropped.
struct Token(HANDLE);

impl Token {
    fn open(access: u32) -> Option<Token> {
        let mut h: HANDLE = std::ptr::null_mut();
        // SAFETY: GetCurrentProcess returns a pseudo-handle that needs no
        // closing; `h` is a valid place for the token handle.
        let ok = unsafe { OpenProcessToken(GetCurrentProcess(), access, &mut h) };
        (ok != 0).then_some(Token(h))
    }
}

impl Drop for Token {
    fn drop(&mut self) {
        // SAFETY: the handle came from OpenProcessToken and is closed once.
        unsafe { CloseHandle(self.0) };
    }
}

/// The local identifier of `SeSystemtimePrivilege`.
fn systemtime_luid() -> Option<LUID> {
    let mut luid = LUID { LowPart: 0, HighPart: 0 };
    // SAFETY: a null system name means this machine; the name is a static
    // NUL-terminated wide string and `luid` a valid place for the answer.
    let ok = unsafe { LookupPrivilegeValueW(std::ptr::null(), SE_SYSTEMTIME_NAME, &mut luid) };
    (ok != 0).then_some(luid)
}

/// Whether this process's token holds `SeSystemtimePrivilege` — enabled or
/// not, since [`step`] enables it.
pub fn may_set_directly() -> bool {
    let (Some(token), Some(want)) = (Token::open(TOKEN_QUERY), systemtime_luid()) else {
        return false;
    };
    // The privileges are a count and that many entries; ask how big first.
    let mut len = 0u32;
    // SAFETY: a null buffer of length 0 asks only for the size needed.
    unsafe { GetTokenInformation(token.0, TokenPrivileges, std::ptr::null_mut(), 0, &mut len) };
    if len == 0 {
        return false;
    }
    // u64s, so the buffer is aligned for the LUIDs inside it.
    let mut buf = vec![0u64; (len as usize).div_ceil(8)];
    // SAFETY: `buf` is at least `len` bytes and aligned for TOKEN_PRIVILEGES.
    let ok = unsafe {
        GetTokenInformation(token.0, TokenPrivileges, buf.as_mut_ptr().cast(), len, &mut len)
    };
    if ok == 0 {
        return false;
    }
    let tp = buf.as_ptr().cast::<TOKEN_PRIVILEGES>();
    // SAFETY: the call filled `buf` with a TOKEN_PRIVILEGES whose array has
    // `PrivilegeCount` entries, all inside the `len` bytes it reported.
    let privileges = unsafe {
        let n = (*tp).PrivilegeCount as usize;
        std::slice::from_raw_parts(
            std::ptr::addr_of!((*tp).Privileges).cast::<LUID_AND_ATTRIBUTES>(),
            n,
        )
    };
    privileges.iter().any(|p| p.Luid.LowPart == want.LowPart && p.Luid.HighPart == want.HighPart)
}

/// Whether UAC can be asked: a process in an interactive session. Services
/// run in session 0, which has no desktop to put the dialog on.
pub fn prompt_available() -> bool {
    let mut session = 0u32;
    // SAFETY: `session` is a valid place for the answer.
    let ok = unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut session) };
    ok != 0 && session != 0
}

/// Enables or disables `SeSystemtimePrivilege` on this process's token.
fn set_privilege(token: &Token, luid: LUID, enable: bool) -> std::io::Result<()> {
    let tp = TOKEN_PRIVILEGES {
        PrivilegeCount: 1,
        Privileges: [LUID_AND_ATTRIBUTES {
            Luid: luid,
            Attributes: if enable { SE_PRIVILEGE_ENABLED } else { 0 },
        }],
    };
    // SAFETY: `tp` is a valid one-entry TOKEN_PRIVILEGES; the previous state
    // is not wanted.
    let ok = unsafe {
        AdjustTokenPrivileges(token.0, 0, &tp, 0, std::ptr::null_mut(), std::ptr::null_mut())
    };
    // It succeeds without having enabled anything when the token does not
    // hold the privilege at all, and says so only through the last error.
    // SAFETY: GetLastError has no preconditions.
    if ok == 0 || unsafe { GetLastError() } == ERROR_NOT_ALL_ASSIGNED {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "this account may not set the clock (SeSystemtimePrivilege)",
        ));
    }
    Ok(())
}

/// 100 ns intervals between 1601-01-01, where FILETIME counts from, and the
/// Unix epoch.
const FILETIME_UNIX_EPOCH: i128 = 116_444_736_000_000_000;

/// FILETIME ticks (100 ns) for an offset `offset_ns` from now, rounded to the
/// whole millisecond `SetSystemTime` will keep.
fn target_ticks(now_ticks: u64, offset_ns: i64) -> u64 {
    let ns = i128::from(now_ticks) * 100 + i128::from(offset_ns);
    let ms = (ns + 500_000).div_euclid(1_000_000);
    (ms * 10_000).clamp(0, i128::from(u64::MAX)) as u64
}

/// Moves the clock by `offset_ns` from this process. Needs
/// `SeSystemtimePrivilege` in its token.
pub fn step(offset_ns: i64) -> std::io::Result<()> {
    let token = Token::open(TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY)
        .ok_or_else(std::io::Error::last_os_error)?;
    let luid = systemtime_luid().ok_or_else(std::io::Error::last_os_error)?;
    set_privilege(&token, luid, true)?;
    let result = (|| {
        let mut ft = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
        // SAFETY: `ft` is a valid place for the time.
        unsafe { GetSystemTimePreciseAsFileTime(&mut ft) };
        let now = (u64::from(ft.dwHighDateTime) << 32) | u64::from(ft.dwLowDateTime);
        let target = target_ticks(now, offset_ns);
        let ft = FILETIME { dwLowDateTime: target as u32, dwHighDateTime: (target >> 32) as u32 };
        // SAFETY: zeroed SYSTEMTIME is a valid value to be overwritten.
        let mut st: SYSTEMTIME = unsafe { std::mem::zeroed() };
        // SAFETY: both point at valid, initialised structures.
        if unsafe { FileTimeToSystemTime(&ft, &mut st) } == 0 {
            return Err(std::io::Error::last_os_error());
        }
        // SAFETY: `st` is the valid UTC time just converted.
        if unsafe { SetSystemTime(&st) } == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    })();
    // Leave the token as it was found; the step's own outcome is what counts.
    let _ = set_privilege(&token, luid, false);
    result
}

/// Moves the clock by `offset_ns` through UAC: this binary, elevated, as the
/// helper. Blocks until the operator has answered the dialog and the helper
/// has finished.
pub fn elevated(exe: &str, offset_ns: i64) -> Result<(), String> {
    let verb = wide("runas");
    let file = wide(exe);
    let params = wide(format!("{HELPER_FLAG} {offset_ns}"));
    // SAFETY: SHELLEXECUTEINFOW is plain data for which all-zero is the
    // documented "unset" state of every field but the ones set below.
    let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    // The process handle, to wait on; no asynchronous launch, since this
    // thread waits anyway; and no error dialog of the shell's own — the
    // panel says what went wrong.
    info.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI;
    info.lpVerb = verb.as_ptr();
    info.lpFile = file.as_ptr();
    info.lpParameters = params.as_ptr();
    // The helper is a console program; it has nothing to show.
    info.nShow = SW_HIDE;
    // SAFETY: `info` is initialised and every string it points to outlives
    // the call.
    if unsafe { ShellExecuteExW(&mut info) } == 0 {
        // SAFETY: GetLastError has no preconditions.
        return Err(if unsafe { GetLastError() } == ERROR_CANCELLED {
            "the administrator password was not given".into()
        } else {
            format!("could not start the clock helper: {}", std::io::Error::last_os_error())
        });
    }
    if info.hProcess.is_null() {
        return Err("the clock helper started without a handle to wait on".into());
    }
    let mut code = 1u32;
    // SAFETY: hProcess is the helper's process handle, ours to wait on and
    // close; `code` is a valid place for its exit code.
    let waited = unsafe {
        let w = WaitForSingleObject(info.hProcess, INFINITE);
        let got = GetExitCodeProcess(info.hProcess, &mut code);
        CloseHandle(info.hProcess);
        w == WAIT_OBJECT_0 && got != 0
    };
    match (waited, code) {
        (false, _) => Err("lost track of the clock helper".into()),
        (true, 0) => Ok(()),
        // An elevated process's output cannot be read back from here, so
        // the helper's exit code is all there is: see `helper_main`.
        (true, 2) => Err("the clock helper refused the offset".into()),
        (true, c) => Err(format!("the clock was not set (helper exit code {c})")),
    }
}

/// The Windows Time service, which puts the clock back on its own schedule
/// while "Set time automatically" is on.
pub fn sync_daemon() -> Option<String> {
    let name = wide("W32Time");
    // SAFETY: null machine and database names mean this machine's active
    // database; the handles are closed below.
    let running = unsafe {
        let scm = OpenSCManagerW(std::ptr::null(), std::ptr::null(), SC_MANAGER_CONNECT);
        if scm.is_null() {
            return None;
        }
        let svc = OpenServiceW(scm, name.as_ptr(), SERVICE_QUERY_STATUS);
        let running = if svc.is_null() {
            None
        } else {
            let mut st: SERVICE_STATUS = std::mem::zeroed();
            let ok = QueryServiceStatus(svc, &mut st);
            CloseServiceHandle(svc);
            (ok != 0).then_some(st.dwCurrentState == SERVICE_RUNNING)
        };
        CloseServiceHandle(scm);
        running?
    };
    // The service is started by a trigger when it is time to synchronise and
    // stops again in between, so one that is stopped now is not one that
    // will leave the clock alone.
    Some(if running {
        "the Windows Time service (w32time)".into()
    } else {
        "the Windows Time service, if \"Set the time automatically\" is on".into()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_target_is_rounded_to_the_millisecond() {
        // 2026-11-04T00:00:00.0004 UTC in FILETIME ticks.
        let now = (1_793_750_400i128 * 10_000_000 + FILETIME_UNIX_EPOCH + 4_000) as u64;
        // +1.0006 s lands on .001 after rounding .0004 + 1.0006 = 1.0010.
        let t = target_ticks(now, 1_000_600_000);
        assert_eq!(t % 10_000, 0);
        assert_eq!(t - (now - 4_000), 10_010_000);
        // Backwards too.
        let t = target_ticks(now, -2_000_000_000);
        assert_eq!(t, now - 4_000 - 20_000_000);
    }

    #[test]
    fn privileges_and_sessions_can_be_asked_about() {
        // No assertion about the answers — they depend on the account the
        // test runs under — only that asking works and does not crash.
        let _ = may_set_directly();
        let _ = prompt_available();
        let _ = sync_daemon();
    }
}
