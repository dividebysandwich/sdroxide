//! HID through `/dev/hidraw*`.
//!
//! What direwolf, `usbrelay` and every other program that drives one of these
//! boards on Linux does, and for the same reason: the kernel binds `usbhid` to
//! the device and hands out a character device that speaks whole reports, so
//! nothing has to claim an interface or fight a driver for it.
//!
//! Enumeration reads `/sys/class/hidraw/hidrawN/device/uevent` rather than
//! opening anything — `HID_ID=0003:000016C0:000005DF` is the bus, vendor and
//! product, and `HID_NAME` is the product string. That matters here beyond
//! tidiness: `16c0:05df` is a *shared* V-USB hobby id, so the product string is
//! the only thing separating a relay board from somebody's home-made keyboard,
//! and a probe that opened every candidate to find out would be opening
//! arbitrary devices on the operator's bus.

use std::io::{Read, Write};
use std::os::fd::AsRawFd;

use crate::error::{Error, Result};

use super::{HidDev, HidEntry};

/// `_IOC(_IOC_WRITE|_IOC_READ, 'H', 0x06, len)` — `HIDIOCSFEATURE`.
fn hidiocsfeature(len: usize) -> u64 {
    ioc(3, 0x06, len)
}

/// `HIDIOCGFEATURE`.
fn hidiocgfeature(len: usize) -> u64 {
    ioc(3, 0x07, len)
}

/// The `asm-generic` ioctl encoding. Correct on x86_64, aarch64, arm and
/// riscv — every target this program is built for.
fn ioc(dir: u64, nr: u64, size: usize) -> u64 {
    (dir << 30) | ((size as u64) << 16) | (u64::from(b'H') << 8) | nr
}

pub struct HidRaw {
    file: std::fs::File,
    path: String,
}

impl HidDev for HidRaw {
    fn set_feature(&mut self, report_id: u8, body: &[u8]) -> Result<()> {
        // The id goes in byte 0. For id 0 the kernel strips it again before it
        // reaches the wire, which is what makes this the same buffer every
        // other program on this platform sends.
        let mut buf = Vec::with_capacity(body.len() + 1);
        buf.push(report_id);
        buf.extend_from_slice(body);
        let rc = unsafe {
            libc::ioctl(self.file.as_raw_fd(), hidiocsfeature(buf.len()) as _, buf.as_mut_ptr())
        };
        if rc < 0 {
            return Err(Error::opening(&self.path, std::io::Error::last_os_error()));
        }
        Ok(())
    }

    fn get_feature(&mut self, report_id: u8, body: &mut [u8]) -> Result<()> {
        // One byte longer than the report, with the id in byte 0 on the way in.
        //
        // On the way *out* `hidraw_get_report` does not put the id back — it
        // fills the buffer with the report itself from byte 0 — so the answer
        // is `buf[..body.len()]` and not `buf[1..]`. This asymmetry with the
        // set path above is the kernel's, not ours; it is also the one thing
        // in this file that no test here can catch, so the caller
        // ([`crate::dcttech`]) checks that what came back looks like an answer
        // before believing it.
        let mut buf = vec![0u8; body.len() + 1];
        buf[0] = report_id;
        let rc = unsafe {
            libc::ioctl(self.file.as_raw_fd(), hidiocgfeature(buf.len()) as _, buf.as_mut_ptr())
        };
        if rc < 0 {
            return Err(Error::opening(&self.path, std::io::Error::last_os_error()));
        }
        body.copy_from_slice(&buf[..body.len()]);
        Ok(())
    }

    fn write_output(&mut self, report_id: u8, body: &[u8]) -> Result<()> {
        let mut buf = Vec::with_capacity(body.len() + 1);
        buf.push(report_id);
        buf.extend_from_slice(body);
        self.file.write_all(&buf)?;
        Ok(())
    }

    fn read_input(
        &mut self,
        body: &mut [u8],
        timeout: std::time::Duration,
    ) -> Result<Option<usize>> {
        // `poll` rather than a non-blocking descriptor, so the write path above
        // keeps its ordinary blocking semantics.
        let mut pfd = libc::pollfd { fd: self.file.as_raw_fd(), events: libc::POLLIN, revents: 0 };
        let ms = timeout.as_millis().min(i32::MAX as u128) as i32;
        let rc = unsafe { libc::poll(&mut pfd, 1, ms) };
        if rc < 0 {
            let e = std::io::Error::last_os_error();
            if e.kind() == std::io::ErrorKind::Interrupted {
                return Ok(None);
            }
            return Err(e.into());
        }
        if rc == 0 {
            return Ok(None);
        }
        // An unplug shows up as POLLHUP/POLLERR, and the read below then fails
        // with ENODEV — which is the error the caller treats as "gone".
        // hidraw hands over exactly one report per read, without an id byte
        // for a device that has no numbered reports.
        let n = self.file.read(body)?;
        if n == 0 {
            return Err(Error::Io(std::io::Error::from(std::io::ErrorKind::UnexpectedEof)));
        }
        Ok(Some(n))
    }
}

pub fn open(key: &str) -> Result<Box<dyn HidDev>> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(key)
        .map_err(|e| Error::opening(key, e))?;
    Ok(Box::new(HidRaw { file, path: key.to_string() }))
}

pub fn enumerate(ids: &[(u16, u16)]) -> Vec<HidEntry> {
    let mut out = Vec::new();
    let Ok(dir) = std::fs::read_dir("/sys/class/hidraw") else { return out };
    let mut names: Vec<String> = dir
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.starts_with("hidraw"))
        .collect();
    // Numerically, so hidraw2 comes before hidraw10 and the list an operator
    // sees does not reorder itself the moment they own eleven of these.
    names.sort_by_key(|n| n[6..].parse::<u32>().unwrap_or(u32::MAX));

    for name in names {
        let uevent = format!("/sys/class/hidraw/{name}/device/uevent");
        let Ok(text) = std::fs::read_to_string(&uevent) else { continue };
        let Some((vendor, product)) = hid_id(&text) else { continue };
        if !ids.is_empty() && !ids.contains(&(vendor, product)) {
            continue;
        }
        out.push(HidEntry {
            key: format!("/dev/{name}"),
            vendor,
            product,
            name: field(&text, "HID_NAME").unwrap_or_default(),
            serial: field(&text, "HID_UNIQ").unwrap_or_default(),
        });
    }
    out
}

/// `HID_ID=0003:000016C0:000005DF` → `(0x16c0, 0x05df)`. The leading field is
/// the bus and is not what anything here selects on.
fn hid_id(uevent: &str) -> Option<(u16, u16)> {
    let v = field(uevent, "HID_ID")?;
    let mut parts = v.split(':');
    let _bus = parts.next()?;
    let vendor = u32::from_str_radix(parts.next()?.trim(), 16).ok()?;
    let product = u32::from_str_radix(parts.next()?.trim(), 16).ok()?;
    Some((vendor as u16, product as u16))
}

fn field(uevent: &str, key: &str) -> Option<String> {
    uevent
        .lines()
        .find_map(|l| l.strip_prefix(key).and_then(|r| r.strip_prefix('=')))
        .map(|v| v.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real `uevent` from a dcttech relay board, verbatim.
    const UEVENT: &str = "DRIVER=hid-generic\n\
         HID_ID=0003:000016C0:000005DF\n\
         HID_NAME=www.dcttech.com USBRelay2\n\
         HID_PHYS=usb-0000:00:14.0-2/input0\n\
         HID_UNIQ=\n\
         MODALIAS=hid:b0003g0001v000016C0p000005DF\n";

    #[test]
    fn a_uevent_yields_the_usb_ids_and_the_product_string() {
        assert_eq!(hid_id(UEVENT), Some((0x16c0, 0x05df)));
        assert_eq!(field(UEVENT, "HID_NAME").as_deref(), Some("www.dcttech.com USBRelay2"));
        // Present but empty — which is why the board's serial has to be read
        // out of a feature report instead.
        assert_eq!(field(UEVENT, "HID_UNIQ").as_deref(), Some(""));
        assert_eq!(field(UEVENT, "HID_NOPE"), None);
    }

    #[test]
    fn a_uevent_without_an_id_is_skipped_rather_than_guessed_at() {
        assert_eq!(hid_id("DRIVER=hid-generic\n"), None);
        assert_eq!(hid_id("HID_ID=0003:zzzz:0005\n"), None);
    }

    /// The numbers a reviewer can check against `linux/hidraw.h`.
    #[test]
    fn the_ioctl_numbers_are_the_kernels() {
        // HIDIOCSFEATURE(9) and HIDIOCGFEATURE(9).
        assert_eq!(hidiocsfeature(9), 0xC009_4806);
        assert_eq!(hidiocgfeature(9), 0xC009_4807);
    }
}
