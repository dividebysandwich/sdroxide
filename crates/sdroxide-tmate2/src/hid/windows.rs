//! HID on Windows: SetupAPI to find the device, `hid.dll` to size its
//! reports, and overlapped `ReadFile` / `WriteFile` to move them.
//!
//! Overlapped because a plain `ReadFile` on a HID handle blocks until the
//! device speaks, and a TMate 2 nobody touches never does — the worker could
//! then neither notice a settings change nor shut down. Each read waits on an
//! event with a timeout and cancels itself when that runs out.
//!
//! Report sizes come from `HidP_GetCaps` rather than being assumed: for a
//! device with unnumbered reports Windows reports 65 bytes each way, a zero id
//! byte then the 64 on the wire, and that is what the buffers are sized to.

use std::io;
use std::os::windows::ffi::OsStrExt;
use std::time::Duration;

use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
    DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, SP_DEVICE_INTERFACE_DATA, SetupDiDestroyDeviceInfoList,
    SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW, SetupDiGetDeviceInterfaceDetailW,
};
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_IO_PENDING, GENERIC_READ, GENERIC_WRITE, GetLastError, HANDLE,
    INVALID_HANDLE_VALUE, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_OVERLAPPED, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING, ReadFile,
    WriteFile,
};
use windows_sys::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};
use windows_sys::Win32::System::Threading::{CreateEventW, ResetEvent, WaitForSingleObject};
use windows_sys::core::GUID;

use super::{Device, Found, unwrap_read};
use crate::protocol::{PRODUCT_ID, REPORT_LEN, VENDOR_ID};

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct HiddAttributes {
    size: u32,
    vendor_id: u16,
    product_id: u16,
    version_number: u16,
}

/// `HIDP_CAPS`: five `USHORT`s, seventeen reserved, then ten counts.
#[repr(C)]
#[derive(Clone, Copy)]
struct HidpCaps {
    usage: u16,
    usage_page: u16,
    input_report_byte_length: u16,
    output_report_byte_length: u16,
    feature_report_byte_length: u16,
    reserved: [u16; 17],
    counts: [u16; 10],
}

const HIDP_STATUS_SUCCESS: i32 = 0x0011_0000;

// `BOOLEAN` is one byte; reading it as an `i32` would pick up whatever sits in
// the rest of the register.
#[link(name = "hid")]
unsafe extern "system" {
    fn HidD_GetHidGuid(guid: *mut GUID);
    fn HidD_GetAttributes(device: HANDLE, attributes: *mut HiddAttributes) -> u8;
    fn HidD_GetProductString(device: HANDLE, buffer: *mut u16, len: u32) -> u8;
    fn HidD_GetPreparsedData(device: HANDLE, data: *mut isize) -> u8;
    fn HidD_FreePreparsedData(data: isize) -> u8;
    fn HidP_GetCaps(data: isize, caps: *mut HidpCaps) -> i32;
}

/// An owned handle, closed on drop.
struct Handle(HANDLE);

impl Drop for Handle {
    fn drop(&mut self) {
        if !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE {
            unsafe { CloseHandle(self.0) };
        }
    }
}

// Only ever used from the worker thread that opened it.
unsafe impl Send for Handle {}

fn wide(s: &str) -> Vec<u16> {
    std::ffi::OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

fn from_wide(buf: &[u16]) -> String {
    let end = buf.iter().position(|c| *c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

fn create(path: &str, access: u32, flags: u32) -> io::Result<Handle> {
    let w = wide(path);
    let h = unsafe {
        CreateFileW(
            w.as_ptr(),
            access,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            flags,
            std::ptr::null_mut(),
        )
    };
    if h.is_null() || h == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    Ok(Handle(h))
}

fn caps(h: &Handle) -> Option<HidpCaps> {
    let mut pp: isize = 0;
    if unsafe { HidD_GetPreparsedData(h.0, &mut pp) } == 0 {
        return None;
    }
    let mut caps: HidpCaps = unsafe { std::mem::zeroed() };
    let rc = unsafe { HidP_GetCaps(pp, &mut caps) };
    unsafe { HidD_FreePreparsedData(pp) };
    (rc == HIDP_STATUS_SUCCESS).then_some(caps)
}

struct WinHid {
    handle: Handle,
    /// Signalled when an overlapped operation completes. One is enough: reads
    /// and writes are never in flight at the same time.
    event: Handle,
    in_len: usize,
    out_len: usize,
}

impl WinHid {
    /// Run one overlapped transfer to completion or `timeout`, and return the
    /// bytes moved. `start` issues the `ReadFile` / `WriteFile`.
    fn transfer(
        &mut self,
        timeout: Duration,
        start: impl FnOnce(HANDLE, *mut OVERLAPPED) -> i32,
    ) -> io::Result<Option<usize>> {
        let mut ov: OVERLAPPED = unsafe { std::mem::zeroed() };
        ov.hEvent = self.event.0;
        unsafe { ResetEvent(self.event.0) };
        let ok = start(self.handle.0, &mut ov);
        if ok == 0 {
            let err = unsafe { GetLastError() };
            if err != ERROR_IO_PENDING {
                return Err(io::Error::from_raw_os_error(err as i32));
            }
            let ms = timeout.as_millis().min(u32::MAX as u128 - 1) as u32;
            let w = unsafe { WaitForSingleObject(self.event.0, ms) };
            if w == WAIT_TIMEOUT {
                // Cancel, then wait for the cancellation to land: the
                // OVERLAPPED and the buffer must outlive the operation.
                unsafe { CancelIoEx(self.handle.0, &ov) };
                let mut n = 0u32;
                let done = unsafe { GetOverlappedResult(self.handle.0, &ov, &mut n, 1) };
                // It may have completed in the gap; take the data if so.
                return if done != 0 && n > 0 { Ok(Some(n as usize)) } else { Ok(None) };
            }
            if w != WAIT_OBJECT_0 {
                return Err(io::Error::last_os_error());
            }
        }
        let mut n = 0u32;
        if unsafe { GetOverlappedResult(self.handle.0, &ov, &mut n, 1) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Some(n as usize))
    }
}

impl Device for WinHid {
    fn read(&mut self, buf: &mut [u8; REPORT_LEN], timeout: Duration) -> io::Result<Option<usize>> {
        let mut raw = vec![0u8; self.in_len];
        let len = raw.len() as u32;
        let ptr = raw.as_mut_ptr();
        let got = self.transfer(timeout, |h, ov| unsafe {
            ReadFile(h, ptr, len, std::ptr::null_mut(), ov)
        })?;
        Ok(got.map(|n| unwrap_read(&raw[..n.min(raw.len())], buf)))
    }

    fn write(&mut self, report: &[u8; REPORT_LEN]) -> io::Result<()> {
        // Windows wants exactly the declared length: a zero id byte in front
        // when that is 65, the report alone when it is 64.
        let mut out = vec![0u8; self.out_len];
        let body = if self.out_len > REPORT_LEN { &mut out[1..] } else { &mut out[..] };
        let n = body.len().min(REPORT_LEN);
        body[..n].copy_from_slice(&report[..n]);
        let len = out.len() as u32;
        let ptr = out.as_ptr();
        match self.transfer(Duration::from_secs(1), |h, ov| unsafe {
            WriteFile(h, ptr, len, std::ptr::null_mut(), ov)
        })? {
            Some(_) => Ok(()),
            None => Err(io::Error::new(io::ErrorKind::TimedOut, "TMate 2 write timed out")),
        }
    }
}

pub fn open(key: &str) -> io::Result<Box<dyn Device>> {
    let handle = create(key, GENERIC_READ | GENERIC_WRITE, FILE_FLAG_OVERLAPPED)?;
    let c = caps(&handle).ok_or_else(|| io::Error::other("HidP_GetCaps failed"))?;
    let event = unsafe { CreateEventW(std::ptr::null(), 1, 0, std::ptr::null()) };
    if event.is_null() {
        return Err(io::Error::last_os_error());
    }
    let in_len = (c.input_report_byte_length as usize).max(REPORT_LEN);
    let out_len = (c.output_report_byte_length as usize).max(REPORT_LEN);
    tracing::info!(key, in_len, out_len, "TMate 2 report sizes");
    Ok(Box::new(WinHid { handle, event: Handle(event), in_len, out_len }))
}

pub fn find() -> Vec<Found> {
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
    if set == 0 || set == -1 {
        return out;
    }

    let mut index = 0u32;
    loop {
        let mut iface: SP_DEVICE_INTERFACE_DATA = unsafe { std::mem::zeroed() };
        iface.cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32;
        if unsafe { SetupDiEnumDeviceInterfaces(set, std::ptr::null(), &guid, index, &mut iface) }
            == 0
        {
            break;
        }
        index += 1;

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
        // SP_DEVICE_INTERFACE_DETAIL_DATA_W: a `u32` cbSize (8 on 64-bit,
        // padded to the alignment of the WCHAR array) followed by the path.
        let mut detail = vec![0u8; needed as usize];
        let header = detail.as_mut_ptr().cast::<u32>();
        unsafe { header.write(if cfg!(target_pointer_width = "64") { 8 } else { 6 }) };
        if unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                set,
                &iface,
                detail.as_mut_ptr().cast(),
                needed,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        } == 0
        {
            continue;
        }
        let path_u16: Vec<u16> = detail[4..]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .take_while(|c| *c != 0)
            .collect();
        let path = String::from_utf16_lossy(&path_u16);
        if path.is_empty() {
            continue;
        }

        // No access rights: enough to read the attributes, and it never
        // fails on a keyboard or mouse the system holds exclusively.
        let Ok(h) = create(&path, 0, 0) else { continue };
        let mut attrs = HiddAttributes {
            size: std::mem::size_of::<HiddAttributes>() as u32,
            ..HiddAttributes::default()
        };
        if unsafe { HidD_GetAttributes(h.0, &mut attrs) } == 0 {
            continue;
        }
        if (attrs.vendor_id, attrs.product_id) != (VENDOR_ID, PRODUCT_ID) {
            continue;
        }
        let mut name = [0u16; 128];
        let name = if unsafe { HidD_GetProductString(h.0, name.as_mut_ptr(), 256) } != 0 {
            from_wide(&name)
        } else {
            "TMate 2".to_string()
        };
        out.push(Found { key: path, name });
    }
    unsafe { SetupDiDestroyDeviceInfoList(set) };
    out
}
