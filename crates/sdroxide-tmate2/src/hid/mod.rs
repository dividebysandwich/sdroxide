//! Just enough HID to find a TMate 2, read its reports and write one back.
//!
//! Every call here deals in the report as it is on the wire — 64 bytes, no
//! report id. The platforms disagree about where an id byte goes for a device
//! with unnumbered reports (Windows puts a zero in front in both directions;
//! Linux wants one in front on writes and gives none back on reads), and this
//! module is where that is undone so [`crate::protocol`] never sees it.

use std::io;
use std::time::Duration;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
mod unsupported;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
use linux as backend;
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
use unsupported as backend;
#[cfg(target_os = "windows")]
use windows as backend;

use crate::protocol::REPORT_LEN;

/// A TMate 2, opened.
pub trait Device: Send {
    /// Wait up to `timeout` for one input report. `Ok(None)` is a timeout;
    /// `Ok(Some(n))` is a report of `n` bytes at the front of `buf`.
    fn read(&mut self, buf: &mut [u8; REPORT_LEN], timeout: Duration) -> io::Result<Option<usize>>;

    /// Send one output report.
    fn write(&mut self, report: &[u8; REPORT_LEN]) -> io::Result<()>;
}

/// A TMate 2 seen on the bus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// What [`open`] takes: a `/dev/hidraw*` path or a Windows interface path.
    pub key: String,
    /// The product string, where the platform gives one.
    pub name: String,
}

/// Every attached TMate 2. Non-invasive on Linux; on Windows it has to open
/// each HID interface to ask what it is, which it does with no access rights.
pub fn find() -> Vec<Found> {
    backend::find()
}

pub fn open(key: &str) -> io::Result<Box<dyn Device>> {
    backend::open(key)
}

/// Whether this platform has a backend at all.
pub const SUPPORTED: bool = cfg!(any(target_os = "linux", target_os = "windows"));

/// Take the 64-byte wire report out of whatever a platform read returned:
/// a 65-byte read is the report behind a zero report-id byte.
pub(crate) fn unwrap_read(raw: &[u8], out: &mut [u8; REPORT_LEN]) -> usize {
    let body = if raw.len() == REPORT_LEN + 1 { &raw[1..] } else { raw };
    let n = body.len().min(REPORT_LEN);
    out[..n].copy_from_slice(&body[..n]);
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_leading_zero_id_is_dropped() {
        let mut raw = [0u8; 65];
        raw[1] = 0x01;
        raw[2] = 0x4A;
        let mut out = [0u8; 64];
        assert_eq!(unwrap_read(&raw, &mut out), 64);
        assert_eq!(&out[..2], &[0x01, 0x4A]);
    }

    #[test]
    fn a_bare_report_is_taken_as_is() {
        let mut raw = [0u8; 64];
        raw[0] = 0x01;
        let mut out = [0u8; 64];
        assert_eq!(unwrap_read(&raw, &mut out), 64);
        assert_eq!(out[0], 0x01);
    }
}
