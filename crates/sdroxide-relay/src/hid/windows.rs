//! HID on Windows: SetupAPI to find the devices, `hid.dll` to talk to them.
//!
//! ⚠️ **Never run against hardware.** This machine is Linux; the code below is
//! written from the Win32 documentation and compiles on the release runner, so
//! a signature error is caught and a wrong buffer offset is not. The
//! `relay` example prints enough of a transcript to settle it from one
//! operator's report — that is what it is there for.
//!
//! `hid.dll` is not among the libraries `windows-sys` links, so the four
//! functions used here are declared and linked explicitly.
//!
//! # Buffer conventions, which differ from every other platform
//!
//! `HidD_SetFeature` and `HidD_GetFeature` both take a buffer whose byte 0 is
//! the report id **in both directions** — unlike Linux, which strips a zero id
//! on the way out and does not put one back on the way in. `HidD_GetFeature`
//! also insists the buffer be exactly the length the device's capabilities
//! declare for a feature report, which for these devices is the report plus the
//! id byte. So the sizes here are `body.len() + 1` throughout, and the answer
//! is read from byte 1.
//!
//! # Input reports
//!
//! `ReadFile` on a HID handle blocks until the device sends something, which a
//! knob left alone never does. So the read goes through a *second* handle,
//! opened overlapped, with the read left pending across a timeout rather than
//! cancelled — cancelling and reissuing would drop a report that lands in the
//! gap. The first handle stays synchronous so the feature and output paths
//! above are untouched.

use std::os::windows::ffi::OsStrExt;

use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
    DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, SP_DEVICE_INTERFACE_DATA, SetupDiDestroyDeviceInfoList,
    SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW, SetupDiGetDeviceInterfaceDetailW,
};
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_IO_PENDING, GENERIC_READ, GENERIC_WRITE, GetLastError, HANDLE,
    WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_OVERLAPPED, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_READ,
    FILE_SHARE_WRITE, OPEN_EXISTING, ReadFile, WriteFile,
};
use windows_sys::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};
use windows_sys::Win32::System::Threading::{CreateEventW, ResetEvent, WaitForSingleObject};
use windows_sys::core::GUID;

use crate::error::{Error, Result};

use super::{HidDev, HidEntry};

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct HiddAttributes {
    size: u32,
    vendor_id: u16,
    product_id: u16,
    version_number: u16,
}

#[link(name = "hid")]
unsafe extern "system" {
    fn HidD_GetHidGuid(guid: *mut GUID);
    fn HidD_GetAttributes(device: HANDLE, attributes: *mut HiddAttributes) -> i32;
    fn HidD_GetProductString(device: HANDLE, buffer: *mut u16, len: u32) -> i32;
    fn HidD_GetSerialNumberString(device: HANDLE, buffer: *mut u16, len: u32) -> i32;
    fn HidD_SetFeature(device: HANDLE, buffer: *const u8, len: u32) -> i32;
    fn HidD_GetFeature(device: HANDLE, buffer: *mut u8, len: u32) -> i32;
    fn HidD_GetPreparsedData(device: HANDLE, preparsed: *mut isize) -> i32;
    fn HidD_FreePreparsedData(preparsed: isize) -> i32;
    fn HidP_GetCaps(preparsed: isize, caps: *mut HidpCaps) -> i32;
}

/// `HIDP_CAPS`. Only the report lengths are read; the rest is the usage and
/// the counts of every kind of control, none of which matter here.
#[repr(C)]
#[derive(Clone, Copy)]
struct HidpCaps {
    usage: u16,
    usage_page: u16,
    input_report_byte_length: u16,
    output_report_byte_length: u16,
    feature_report_byte_length: u16,
    rest: [u16; 27],
}

/// `HIDP_STATUS_SUCCESS`.
const HIDP_STATUS_SUCCESS: i32 = 0x0011_0000;

/// An owned device handle. A `Drop` rather than a bare `HANDLE` because every
/// path out of `enumerate` opens one and most of them throw it away.
struct Handle(HANDLE);

impl Drop for Handle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CloseHandle(self.0) };
        }
    }
}

// The handle is only ever used from the driver's own thread.
unsafe impl Send for Handle {}

pub struct WinHid {
    handle: Handle,
    path: String,
    /// The overlapped handle input reports are read through, opened on the
    /// first read — a relay is never read, and should not hold a second one.
    reader: Option<Reader>,
}

/// An overlapped read that may still be in flight.
///
/// Boxed so the `OVERLAPPED` and the buffer the kernel is writing into keep
/// their addresses for as long as the read is pending, whatever happens to
/// the `WinHid` around them.
struct Reader {
    handle: Handle,
    event: Handle,
    ov: Box<OVERLAPPED>,
    buf: Box<[u8]>,
    pending: bool,
}

// `OVERLAPPED` holds raw pointers; the reader is only ever touched from the
// thread that owns the device, as the `Handle` above is.
unsafe impl Send for Reader {}

impl Drop for Reader {
    fn drop(&mut self) {
        if self.pending {
            // The kernel still owns `ov` and `buf`: cancel, and wait for it to
            // let go of them before they are freed.
            let mut n = 0u32;
            unsafe {
                CancelIoEx(self.handle.0, &*self.ov);
                GetOverlappedResult(self.handle.0, &*self.ov, &mut n, 1);
            }
        }
    }
}

/// The length Windows insists an input-report read buffer has: the device's
/// longest input report plus the id byte.
fn input_report_len(h: HANDLE) -> Option<usize> {
    let mut pp: isize = 0;
    if unsafe { HidD_GetPreparsedData(h, &mut pp) } == 0 {
        return None;
    }
    let mut caps: HidpCaps = unsafe { std::mem::zeroed() };
    let rc = unsafe { HidP_GetCaps(pp, &mut caps) };
    unsafe { HidD_FreePreparsedData(pp) };
    (rc == HIDP_STATUS_SUCCESS && caps.input_report_byte_length > 0)
        .then_some(usize::from(caps.input_report_byte_length))
}

fn wide(s: &str) -> Vec<u16> {
    std::ffi::OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

fn from_wide(buf: &[u16]) -> String {
    let end = buf.iter().position(|c| *c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

fn open_path(path: &str) -> Result<Handle> {
    open_path_with(path, 0)
}

fn open_path_with(path: &str, flags: FILE_FLAGS_AND_ATTRIBUTES) -> Result<Handle> {
    let w = wide(path);
    // Shared: a CM108 is a sound card the rig is also using, and opening it
    // exclusively would take the audio away from whatever is playing it.
    let h = unsafe {
        CreateFileW(
            w.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            flags,
            std::ptr::null_mut(),
        )
    };
    if h.is_null() || h as isize == -1 {
        return Err(Error::opening(path, std::io::Error::last_os_error()));
    }
    Ok(Handle(h))
}

impl HidDev for WinHid {
    fn set_feature(&mut self, report_id: u8, body: &[u8]) -> Result<()> {
        let mut buf = Vec::with_capacity(body.len() + 1);
        buf.push(report_id);
        buf.extend_from_slice(body);
        let ok = unsafe { HidD_SetFeature(self.handle.0, buf.as_ptr(), buf.len() as u32) };
        if ok == 0 {
            return Err(Error::opening(&self.path, std::io::Error::last_os_error()));
        }
        Ok(())
    }

    fn get_feature(&mut self, report_id: u8, body: &mut [u8]) -> Result<()> {
        let mut buf = vec![0u8; body.len() + 1];
        buf[0] = report_id;
        let ok = unsafe { HidD_GetFeature(self.handle.0, buf.as_mut_ptr(), buf.len() as u32) };
        if ok == 0 {
            return Err(Error::opening(&self.path, std::io::Error::last_os_error()));
        }
        // Byte 0 comes back as the report id — see the module docs.
        body.copy_from_slice(&buf[1..]);
        Ok(())
    }

    fn write_output(&mut self, report_id: u8, body: &[u8]) -> Result<()> {
        let mut buf = Vec::with_capacity(body.len() + 1);
        buf.push(report_id);
        buf.extend_from_slice(body);
        let mut written = 0u32;
        let ok = unsafe {
            WriteFile(
                self.handle.0,
                buf.as_ptr(),
                buf.len() as u32,
                &mut written,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(Error::opening(&self.path, std::io::Error::last_os_error()));
        }
        Ok(())
    }

    fn read_input(
        &mut self,
        body: &mut [u8],
        timeout: std::time::Duration,
    ) -> Result<Option<usize>> {
        if self.reader.is_none() {
            let handle = open_path_with(&self.path, FILE_FLAG_OVERLAPPED)?;
            let len = input_report_len(handle.0).unwrap_or(body.len() + 1);
            // Manual-reset, so the wait below and `GetOverlappedResult` agree
            // about whether the read has finished.
            let event = Handle(unsafe { CreateEventW(std::ptr::null(), 1, 0, std::ptr::null()) });
            if event.0.is_null() {
                return Err(std::io::Error::last_os_error().into());
            }
            self.reader = Some(Reader {
                handle,
                event,
                ov: Box::new(unsafe { std::mem::zeroed() }),
                buf: vec![0u8; len].into_boxed_slice(),
                pending: false,
            });
        }
        let Some(r) = self.reader.as_mut() else { return Ok(None) };

        if !r.pending {
            *r.ov = unsafe { std::mem::zeroed() };
            r.ov.hEvent = r.event.0;
            unsafe { ResetEvent(r.event.0) };
            let ok = unsafe {
                ReadFile(
                    r.handle.0,
                    r.buf.as_mut_ptr(),
                    r.buf.len() as u32,
                    std::ptr::null_mut(),
                    &mut *r.ov,
                )
            };
            if ok == 0 {
                let err = unsafe { GetLastError() };
                if err != ERROR_IO_PENDING {
                    self.reader = None;
                    return Err(std::io::Error::from_raw_os_error(err as i32).into());
                }
            }
            r.pending = true;
        }

        let ms = timeout.as_millis().min(u128::from(u32::MAX - 1)) as u32;
        match unsafe { WaitForSingleObject(r.event.0, ms) } {
            WAIT_TIMEOUT => return Ok(None),
            WAIT_OBJECT_0 => {}
            _ => {
                let e = std::io::Error::last_os_error();
                self.reader = None;
                return Err(e.into());
            }
        }
        let mut n = 0u32;
        let ok = unsafe { GetOverlappedResult(r.handle.0, &*r.ov, &mut n, 0) };
        r.pending = false;
        if ok == 0 {
            // Unplugged: ERROR_DEVICE_NOT_CONNECTED, or the handle is dead.
            let e = std::io::Error::last_os_error();
            self.reader = None;
            return Err(e.into());
        }
        // Byte 0 is the report id, 0 for a device with no numbered reports.
        let n = n as usize;
        if n == 0 {
            return Ok(None);
        }
        let got = &r.buf[1..n];
        let k = got.len().min(body.len());
        body[..k].copy_from_slice(&got[..k]);
        Ok(Some(k))
    }
}

pub fn open(key: &str) -> Result<Box<dyn HidDev>> {
    let handle = open_path(key)?;
    Ok(Box::new(WinHid { handle, path: key.to_string(), reader: None }))
}

pub fn enumerate(ids: &[(u16, u16)]) -> Vec<HidEntry> {
    let mut out = Vec::new();
    let mut guid: GUID = unsafe { std::mem::zeroed() };
    unsafe { HidD_GetHidGuid(&mut guid) };

    let set = unsafe {
        SetupDiGetClassDevsW(
            &guid,
            std::ptr::null(),
            std::ptr::null_mut(),
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        )
    };
    // `HDEVINFO` is an `isize` here, not a pointer: 0 and -1 are both failures.
    if set == 0 || set == -1 {
        return out;
    }

    let mut index = 0u32;
    loop {
        let mut iface: SP_DEVICE_INTERFACE_DATA = unsafe { std::mem::zeroed() };
        iface.cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32;
        let ok =
            unsafe { SetupDiEnumDeviceInterfaces(set, std::ptr::null(), &guid, index, &mut iface) };
        if ok == 0 {
            break;
        }
        index += 1;

        // Two calls, as the API demands: one for the size, one for the detail.
        let mut needed = 0u32;
        unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                set,
                &iface,
                std::ptr::null_mut(),
                0,
                &mut needed,
                std::ptr::null_mut(),
            )
        };
        if needed == 0 {
            continue;
        }
        // SP_DEVICE_INTERFACE_DETAIL_DATA_W is a `u32` followed by the path,
        // and `cbSize` is the size of the *fixed* part — 8 on 64-bit, where the
        // struct is padded to the alignment of its `WCHAR[ANYSIZE_ARRAY]`.
        let mut detail = vec![0u8; needed as usize];
        let header = detail.as_mut_ptr().cast::<u32>();
        unsafe { header.write(if cfg!(target_pointer_width = "64") { 8 } else { 6 }) };
        let ok = unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                set,
                &iface,
                detail.as_mut_ptr().cast(),
                needed,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            continue;
        }
        // The path is UTF-16 starting after the `cbSize` field.
        let path_bytes = &detail[4..];
        let path_u16: Vec<u16> = path_bytes
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .take_while(|c| *c != 0)
            .collect();
        let path = String::from_utf16_lossy(&path_u16);
        if path.is_empty() {
            continue;
        }

        // Opening is the only way to ask what a HID device is on Windows. Done
        // for every HID device on the machine, which is why it is shared and
        // immediately closed — and why this is not called in a loop.
        let Ok(h) = open_path(&path) else { continue };
        let mut attrs = HiddAttributes {
            size: std::mem::size_of::<HiddAttributes>() as u32,
            ..HiddAttributes::default()
        };
        if unsafe { HidD_GetAttributes(h.0, &mut attrs) } == 0 {
            continue;
        }
        if !ids.is_empty() && !ids.contains(&(attrs.vendor_id, attrs.product_id)) {
            continue;
        }
        let mut name = [0u16; 128];
        let name = if unsafe { HidD_GetProductString(h.0, name.as_mut_ptr(), 256) } != 0 {
            from_wide(&name)
        } else {
            String::new()
        };
        let mut serial = [0u16; 128];
        let serial = if unsafe { HidD_GetSerialNumberString(h.0, serial.as_mut_ptr(), 256) } != 0 {
            from_wide(&serial)
        } else {
            String::new()
        };
        out.push(HidEntry {
            key: path,
            vendor: attrs.vendor_id,
            product: attrs.product_id,
            name,
            serial,
        });
    }
    unsafe { SetupDiDestroyDeviceInfoList(set) };
    out
}
