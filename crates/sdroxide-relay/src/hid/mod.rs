//! A thin USB-HID layer: three calls, one per platform.
//!
//! # Why this is written here rather than taken from a crate
//!
//! The two devices this subsystem needs are HID, and neither is reachable
//! through `nusb`, the pure-Rust USB library the rest of the workspace uses:
//! on Windows a HID device is bound to `HidUsb` and WinUSB cannot claim it
//! without replacing the driver, and on Linux claiming the interface means
//! detaching `usbhid` and fighting the kernel for a device it is perfectly
//! happy to share. The obvious alternative, the `hidapi` C library, wants
//! libudev at build time on Linux — which this workspace deliberately avoids
//! (`serialport` carries `default-features = false` for exactly that reason)
//! and which would complicate the glibc-2.35 compatibility builds.
//!
//! What is actually needed is four calls and an enumeration. So they are here:
//! `hidraw` ioctls on Linux, SetupAPI plus `hid.dll` on Windows, `IOHIDManager`
//! on macOS. No new system dependency on any target, and every byte-level
//! decision lives in [`crate::frame`] where it is tested.
//!
//! # The report-id convention
//!
//! Every method here takes the report **body**, without a report id, and a
//! separate `report_id`. The platforms disagree about where the id lives in the
//! buffer — Linux strips a zero id on the way out and does not add one back on
//! the way in, Windows keeps it in byte 0 in both directions, macOS passes the
//! body alone — so normalising here is the only way the callers can be written
//! once. Every device supported uses report id 0.
//!
//! # Not only relays
//!
//! The Icom RC-28 tuning knob (`sdroxide-rc28`) is a HID device too, and the
//! one reason [`HidDev::read_input`] exists: a relay is only ever written to,
//! a knob is only ever listened to. It lives here rather than in a HID crate
//! of its own because the platform code is the expensive part, and it is
//! already here.

use crate::error::Result;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
mod unsupported;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
use linux as backend;
#[cfg(target_os = "macos")]
use macos as backend;
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
use unsupported as backend;
#[cfg(target_os = "windows")]
use windows as backend;

/// One HID device, opened.
pub trait HidDev: Send {
    /// SET_REPORT(Feature) — how the dcttech relays are commanded.
    fn set_feature(&mut self, report_id: u8, body: &[u8]) -> Result<()>;

    /// GET_REPORT(Feature) — how they are read back. `body` is filled with the
    /// report, id excluded.
    fn get_feature(&mut self, report_id: u8, body: &mut [u8]) -> Result<()>;

    /// An output report — how a CM108's GPIO pins are driven.
    fn write_output(&mut self, report_id: u8, body: &[u8]) -> Result<()>;

    /// Wait up to `timeout` for the next input report from the interrupt
    /// endpoint, and copy its body — report id excluded, as everywhere here —
    /// into `body`.
    ///
    /// `Ok(Some(n))` is a report of `n` bytes, `Ok(None)` is the timeout with
    /// nothing arriving, and an error means the device has gone. Only devices
    /// with no numbered reports are read, so there is no id to strip.
    ///
    /// The default says so rather than pretending to time out, for the test
    /// doubles of devices that are never read.
    fn read_input(
        &mut self,
        _body: &mut [u8],
        _timeout: std::time::Duration,
    ) -> Result<Option<usize>> {
        Err(crate::error::Error::Unsupported("this device has no input reports to read".into()))
    }
}

/// A HID device seen on the bus, before anything is opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HidEntry {
    /// What to hand [`open`] — a `/dev/hidraw*` path, a Windows interface
    /// path, or a macOS registry entry id. Opaque above this module, and
    /// stored in the operator's configuration as-is.
    pub key: String,
    pub vendor: u16,
    pub product: u16,
    /// The product string, where the platform will give one.
    pub name: String,
    /// The USB serial, where there is one. Almost always empty on these
    /// boards — the dcttech relays keep their five-character serial inside a
    /// feature report instead, not in the USB descriptors.
    pub serial: String,
}

/// Every HID device whose USB ids are in `ids`.
///
/// Non-invasive: nothing is opened, so it is safe to call while a relay is in
/// use — which the settings dialog does every time it is opened.
pub fn enumerate(ids: &[(u16, u16)]) -> Vec<HidEntry> {
    backend::enumerate(ids)
}

/// Open one, by the `key` an [`HidEntry`] gave.
pub fn open(key: &str) -> Result<Box<dyn HidDev>> {
    backend::open(key)
}
