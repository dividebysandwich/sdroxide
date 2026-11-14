//! hidraw: the kernel's `usbhid` owns the device and hands out whole reports
//! through `/dev/hidrawN`, so nothing has to be claimed or detached.
//!
//! Enumeration reads `/sys/class/hidraw/*/device/uevent` and opens nothing.
//! Opening needs read/write access to the node — the udev rule in
//! `packaging/linux/60-sdroxide-tmate2.rules` grants it to the seated user.

use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::time::Duration;

use super::{Device, Found, unwrap_read};
use crate::protocol::{PRODUCT_ID, REPORT_LEN, VENDOR_ID};

struct HidRaw {
    file: File,
}

impl Device for HidRaw {
    fn read(&mut self, buf: &mut [u8; REPORT_LEN], timeout: Duration) -> io::Result<Option<usize>> {
        let mut pfd = libc::pollfd { fd: self.file.as_raw_fd(), events: libc::POLLIN, revents: 0 };
        let ms = timeout.as_millis().min(i32::MAX as u128) as i32;
        let rc = unsafe { libc::poll(&mut pfd, 1, ms) };
        if rc < 0 {
            let e = io::Error::last_os_error();
            return if e.kind() == io::ErrorKind::Interrupted { Ok(None) } else { Err(e) };
        }
        if rc == 0 {
            return Ok(None);
        }
        // POLLHUP / POLLERR without POLLIN: the device went away.
        if pfd.revents & libc::POLLIN == 0 {
            return Err(io::Error::new(io::ErrorKind::NotConnected, "TMate 2 unplugged"));
        }
        let mut raw = [0u8; REPORT_LEN + 1];
        let n = self.file.read(&mut raw)?;
        if n == 0 {
            return Err(io::Error::new(io::ErrorKind::NotConnected, "TMate 2 unplugged"));
        }
        Ok(Some(unwrap_read(&raw[..n], buf)))
    }

    fn write(&mut self, report: &[u8; REPORT_LEN]) -> io::Result<()> {
        // Report id 0 in front, which the kernel strips before the wire.
        let mut out = [0u8; REPORT_LEN + 1];
        out[1..].copy_from_slice(report);
        self.file.write_all(&out)
    }
}

pub fn open(key: &str) -> io::Result<Box<dyn Device>> {
    let file = std::fs::OpenOptions::new().read(true).write(true).open(key)?;
    Ok(Box::new(HidRaw { file }))
}

pub fn find() -> Vec<Found> {
    let mut out = Vec::new();
    let Ok(dir) = std::fs::read_dir("/sys/class/hidraw") else { return out };
    let mut names: Vec<String> = dir
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.starts_with("hidraw"))
        .collect();
    names.sort_by_key(|n| n[6..].parse::<u32>().unwrap_or(u32::MAX));
    for name in names {
        let Ok(text) = std::fs::read_to_string(format!("/sys/class/hidraw/{name}/device/uevent"))
        else {
            continue;
        };
        if hid_id(&text) != Some((VENDOR_ID, PRODUCT_ID)) {
            continue;
        }
        let product =
            text.lines().find_map(|l| l.strip_prefix("HID_NAME=")).unwrap_or("TMate 2").to_string();
        out.push(Found { key: format!("/dev/{name}"), name: product });
    }
    out
}

/// `HID_ID=0003:00001721:00000614` → `(0x1721, 0x0614)`.
fn hid_id(uevent: &str) -> Option<(u16, u16)> {
    let v = uevent.lines().find_map(|l| l.strip_prefix("HID_ID="))?;
    let mut parts = v.split(':');
    let _bus = parts.next()?;
    let vendor = u32::from_str_radix(parts.next()?, 16).ok()?;
    let product = u32::from_str_radix(parts.next()?, 16).ok()?;
    Some((vendor as u16, product as u16))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_id_out_of_a_uevent() {
        let t = "DRIVER=hid-generic\nHID_ID=0003:00001721:00000614\nHID_NAME=ELAD TMATE2\n";
        assert_eq!(hid_id(t), Some((0x1721, 0x0614)));
        assert_eq!(hid_id("HID_NAME=x\n"), None);
    }
}
