//! HID on macOS: `IOHIDManager` from IOKit.
//!
//! ⚠️ **Never run against hardware.** This machine is Linux; the code below is
//! written from Apple's documentation and compiles on the release runner. The
//! `relay` example prints enough of a transcript to settle it from one
//! operator's report.
//!
//! Hand-written `extern "C"` declarations rather than a bindings crate, for the
//! reason `sdroxide-sdrplay` reaches its vendor library itself: this is a dozen
//! symbols and two frameworks that are present on every Mac, and a dependency
//! for that would cost more to keep current than the code does.
//!
//! # Buffer conventions
//!
//! `IOHIDDeviceGetReport` and `IOHIDDeviceSetReport` take the report **body**
//! and the id separately, which is the shape this module's own trait uses — so
//! this is the one platform where nothing has to be shifted.
//!
//! # Input reports
//!
//! IOKit delivers these by callback, on a run loop. The device is scheduled on
//! the run loop of whichever thread first reads it, and each read runs that
//! loop for up to the timeout — so the callback fires on the reading thread,
//! into a queue that thread then drains, and nothing here needs a lock.
//!
//! # The key
//!
//! A device is named by its IOKit registry entry id, printed as decimal. Stable
//! for as long as the device stays plugged in, and re-enumerated whenever the
//! settings dialog is opened, which is what an operator who replugged their
//! board needs.

use std::ffi::c_void;

use crate::error::{Error, Result};

use super::{HidDev, HidEntry};

type CFIndex = isize;
type CFTypeRef = *const c_void;
type CFStringRef = *const c_void;
type CFDictionaryRef = *const c_void;
type CFSetRef = *const c_void;
type CFAllocatorRef = *const c_void;
type IOHIDManagerRef = *const c_void;
type IOHIDDeviceRef = *const c_void;
type IOReturn = i32;
type CFRunLoopRef = *const c_void;
type IOHIDReportCallback = unsafe extern "C" fn(
    context: *mut c_void,
    result: IOReturn,
    sender: *mut c_void,
    report_type: u32,
    report_id: u32,
    report: *mut u8,
    report_length: CFIndex,
);
type IOHIDCallback =
    unsafe extern "C" fn(context: *mut c_void, result: IOReturn, sender: *mut c_void);

const KERN_SUCCESS: IOReturn = 0;
const K_IOHID_REPORT_TYPE_OUTPUT: u32 = 1;
const K_IOHID_REPORT_TYPE_FEATURE: u32 = 2;
const K_IOHID_OPTIONS_TYPE_NONE: u32 = 0;
const K_CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;
const K_CF_NUMBER_SINT32_TYPE: CFIndex = 3;

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    static kCFAllocatorDefault: CFAllocatorRef;
    fn CFRelease(cf: CFTypeRef);
    fn CFStringCreateWithCString(
        alloc: CFAllocatorRef,
        cstr: *const i8,
        encoding: u32,
    ) -> CFStringRef;
    fn CFStringGetCString(s: CFStringRef, buf: *mut i8, len: CFIndex, encoding: u32) -> u8;
    fn CFGetTypeID(cf: CFTypeRef) -> usize;
    fn CFStringGetTypeID() -> usize;
    fn CFNumberGetTypeID() -> usize;
    fn CFNumberGetValue(n: CFTypeRef, the_type: CFIndex, value_ptr: *mut c_void) -> u8;
    fn CFSetGetCount(s: CFSetRef) -> CFIndex;
    fn CFSetGetValues(s: CFSetRef, values: *mut CFTypeRef);
    static kCFRunLoopDefaultMode: CFStringRef;
    fn CFRunLoopGetCurrent() -> CFRunLoopRef;
    fn CFRunLoopRunInMode(mode: CFStringRef, seconds: f64, return_after_source: u8) -> i32;
}

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOHIDManagerCreate(alloc: CFAllocatorRef, options: u32) -> IOHIDManagerRef;
    fn IOHIDManagerSetDeviceMatching(manager: IOHIDManagerRef, matching: CFDictionaryRef);
    fn IOHIDManagerCopyDevices(manager: IOHIDManagerRef) -> CFSetRef;
    fn IOHIDManagerOpen(manager: IOHIDManagerRef, options: u32) -> IOReturn;
    fn IOHIDDeviceGetProperty(device: IOHIDDeviceRef, key: CFStringRef) -> CFTypeRef;
    fn IOHIDDeviceOpen(device: IOHIDDeviceRef, options: u32) -> IOReturn;
    fn IOHIDDeviceClose(device: IOHIDDeviceRef, options: u32) -> IOReturn;
    fn IOHIDDeviceSetReport(
        device: IOHIDDeviceRef,
        report_type: u32,
        report_id: CFIndex,
        report: *const u8,
        report_length: CFIndex,
    ) -> IOReturn;
    fn IOHIDDeviceGetReport(
        device: IOHIDDeviceRef,
        report_type: u32,
        report_id: CFIndex,
        report: *mut u8,
        report_length: *mut CFIndex,
    ) -> IOReturn;
    fn CFRetain(cf: CFTypeRef) -> CFTypeRef;
    fn IOHIDDeviceScheduleWithRunLoop(
        device: IOHIDDeviceRef,
        run_loop: CFRunLoopRef,
        mode: CFStringRef,
    );
    fn IOHIDDeviceUnscheduleFromRunLoop(
        device: IOHIDDeviceRef,
        run_loop: CFRunLoopRef,
        mode: CFStringRef,
    );
    fn IOHIDDeviceRegisterInputReportCallback(
        device: IOHIDDeviceRef,
        report: *mut u8,
        report_length: CFIndex,
        callback: Option<IOHIDReportCallback>,
        context: *mut c_void,
    );
    fn IOHIDDeviceRegisterRemovalCallback(
        device: IOHIDDeviceRef,
        callback: Option<IOHIDCallback>,
        context: *mut c_void,
    );
}

fn cfstr(s: &str) -> CFStringRef {
    let c = std::ffi::CString::new(s).unwrap_or_default();
    unsafe { CFStringCreateWithCString(kCFAllocatorDefault, c.as_ptr(), K_CF_STRING_ENCODING_UTF8) }
}

/// One of a device's properties, as a number.
fn number_property(device: IOHIDDeviceRef, key: &str) -> Option<i64> {
    let k = cfstr(key);
    if k.is_null() {
        return None;
    }
    let v = unsafe { IOHIDDeviceGetProperty(device, k) };
    unsafe { CFRelease(k) };
    if v.is_null() || unsafe { CFGetTypeID(v) != CFNumberGetTypeID() } {
        return None;
    }
    let mut n: i32 = 0;
    let ok = unsafe { CFNumberGetValue(v, K_CF_NUMBER_SINT32_TYPE, (&raw mut n).cast::<c_void>()) };
    (ok != 0).then_some(i64::from(n))
}

/// One of a device's properties, as a string.
fn string_property(device: IOHIDDeviceRef, key: &str) -> Option<String> {
    let k = cfstr(key);
    if k.is_null() {
        return None;
    }
    let v = unsafe { IOHIDDeviceGetProperty(device, k) };
    unsafe { CFRelease(k) };
    if v.is_null() || unsafe { CFGetTypeID(v) != CFStringGetTypeID() } {
        return None;
    }
    let mut buf = [0i8; 256];
    let ok = unsafe {
        CFStringGetCString(v, buf.as_mut_ptr(), buf.len() as CFIndex, K_CF_STRING_ENCODING_UTF8)
    };
    (ok != 0).then(|| unsafe { std::ffi::CStr::from_ptr(buf.as_ptr()) }.to_string_lossy().into())
}

/// Every HID device this process can see, as raw references. The caller must
/// not outlive the manager, which is why this is private and both users take
/// what they need inside it.
fn with_devices<T>(f: impl FnOnce(&[IOHIDDeviceRef]) -> T) -> Option<T> {
    let manager = unsafe { IOHIDManagerCreate(kCFAllocatorDefault, K_IOHID_OPTIONS_TYPE_NONE) };
    if manager.is_null() {
        return None;
    }
    // Null matching dictionary: every HID device. Filtering happens here rather
    // than in IOKit because the ids are already a list this crate holds.
    unsafe { IOHIDManagerSetDeviceMatching(manager, std::ptr::null()) };
    unsafe { IOHIDManagerOpen(manager, K_IOHID_OPTIONS_TYPE_NONE) };
    let set = unsafe { IOHIDManagerCopyDevices(manager) };
    if set.is_null() {
        unsafe { CFRelease(manager) };
        return None;
    }
    let count = unsafe { CFSetGetCount(set) }.max(0) as usize;
    let mut devices: Vec<CFTypeRef> = vec![std::ptr::null(); count];
    unsafe { CFSetGetValues(set, devices.as_mut_ptr()) };
    let out = f(&devices);
    unsafe { CFRelease(set) };
    unsafe { CFRelease(manager) };
    Some(out)
}

pub struct MacHid {
    device: IOHIDDeviceRef,
    key: String,
    /// Where the input-report callback puts what it receives, once a read has
    /// scheduled the device. Boxed: IOKit holds a pointer to it.
    inbox: Option<Box<Inbox>>,
}

/// Input reports received and not yet read, and whether the device has gone.
struct Inbox {
    run_loop: CFRunLoopRef,
    /// The buffer IOKit writes each report into before calling back.
    buf: Vec<u8>,
    reports: std::collections::VecDeque<Vec<u8>>,
    removed: bool,
}

/// Enough to ride out a slow reader without growing without bound.
const INBOX_MAX: usize = 256;

unsafe extern "C" fn on_report(
    context: *mut c_void,
    _result: IOReturn,
    _sender: *mut c_void,
    _report_type: u32,
    _report_id: u32,
    report: *mut u8,
    report_length: CFIndex,
) {
    let inbox = unsafe { &mut *context.cast::<Inbox>() };
    if report.is_null() || report_length <= 0 {
        return;
    }
    let bytes = unsafe { std::slice::from_raw_parts(report, report_length as usize) };
    if inbox.reports.len() >= INBOX_MAX {
        inbox.reports.pop_front();
    }
    inbox.reports.push_back(bytes.to_vec());
}

unsafe extern "C" fn on_removal(context: *mut c_void, _result: IOReturn, _sender: *mut c_void) {
    let inbox = unsafe { &mut *context.cast::<Inbox>() };
    inbox.removed = true;
}

// The reference is retained for the struct's lifetime and only touched from
// the driver's own thread.
unsafe impl Send for MacHid {}

impl Drop for MacHid {
    fn drop(&mut self) {
        if let Some(inbox) = self.inbox.as_mut() {
            // Detach the callbacks before the inbox they point at is freed.
            let ctx: *mut Inbox = &mut **inbox;
            unsafe {
                IOHIDDeviceRegisterInputReportCallback(
                    self.device,
                    inbox.buf.as_mut_ptr(),
                    inbox.buf.len() as CFIndex,
                    None,
                    ctx.cast(),
                );
                IOHIDDeviceRegisterRemovalCallback(self.device, None, ctx.cast());
                IOHIDDeviceUnscheduleFromRunLoop(
                    self.device,
                    inbox.run_loop,
                    kCFRunLoopDefaultMode,
                );
            }
        }
        unsafe {
            IOHIDDeviceClose(self.device, K_IOHID_OPTIONS_TYPE_NONE);
            CFRelease(self.device);
        }
    }
}

impl MacHid {
    fn fail(&self, rc: IOReturn) -> Error {
        Error::Open {
            path: self.key.clone(),
            source: std::io::Error::other(format!("IOKit returned {rc:#010x}")),
        }
    }
}

impl HidDev for MacHid {
    fn set_feature(&mut self, report_id: u8, body: &[u8]) -> Result<()> {
        let rc = unsafe {
            IOHIDDeviceSetReport(
                self.device,
                K_IOHID_REPORT_TYPE_FEATURE,
                CFIndex::from(report_id),
                body.as_ptr(),
                body.len() as CFIndex,
            )
        };
        if rc != KERN_SUCCESS {
            return Err(self.fail(rc));
        }
        Ok(())
    }

    fn get_feature(&mut self, report_id: u8, body: &mut [u8]) -> Result<()> {
        let mut len = body.len() as CFIndex;
        let rc = unsafe {
            IOHIDDeviceGetReport(
                self.device,
                K_IOHID_REPORT_TYPE_FEATURE,
                CFIndex::from(report_id),
                body.as_mut_ptr(),
                &mut len,
            )
        };
        if rc != KERN_SUCCESS {
            return Err(self.fail(rc));
        }
        Ok(())
    }

    fn write_output(&mut self, report_id: u8, body: &[u8]) -> Result<()> {
        let rc = unsafe {
            IOHIDDeviceSetReport(
                self.device,
                K_IOHID_REPORT_TYPE_OUTPUT,
                CFIndex::from(report_id),
                body.as_ptr(),
                body.len() as CFIndex,
            )
        };
        if rc != KERN_SUCCESS {
            return Err(self.fail(rc));
        }
        Ok(())
    }

    fn read_input(
        &mut self,
        body: &mut [u8],
        timeout: std::time::Duration,
    ) -> Result<Option<usize>> {
        if self.inbox.is_none() {
            let mut inbox = Box::new(Inbox {
                run_loop: unsafe { CFRunLoopGetCurrent() },
                // Far longer than any report a device supported here sends.
                buf: vec![0u8; 1024],
                reports: std::collections::VecDeque::new(),
                removed: false,
            });
            let ctx: *mut Inbox = &mut *inbox;
            unsafe {
                IOHIDDeviceRegisterInputReportCallback(
                    self.device,
                    inbox.buf.as_mut_ptr(),
                    inbox.buf.len() as CFIndex,
                    Some(on_report),
                    ctx.cast(),
                );
                IOHIDDeviceRegisterRemovalCallback(self.device, Some(on_removal), ctx.cast());
                IOHIDDeviceScheduleWithRunLoop(self.device, inbox.run_loop, kCFRunLoopDefaultMode);
            }
            self.inbox = Some(inbox);
        }
        let take = |inbox: &mut Inbox, body: &mut [u8]| {
            inbox.reports.pop_front().map(|r| {
                let k = r.len().min(body.len());
                body[..k].copy_from_slice(&r[..k]);
                k
            })
        };
        let Some(inbox) = self.inbox.as_mut() else { return Ok(None) };
        if let Some(k) = take(inbox, body) {
            return Ok(Some(k));
        }
        if !inbox.removed {
            // Returns after the first source handled, so a report is picked up
            // as soon as it lands rather than at the end of the timeout.
            unsafe { CFRunLoopRunInMode(kCFRunLoopDefaultMode, timeout.as_secs_f64(), 1) };
        }
        let Some(inbox) = self.inbox.as_mut() else { return Ok(None) };
        if let Some(k) = take(inbox, body) {
            return Ok(Some(k));
        }
        if inbox.removed {
            return Err(Error::NotFound { key: self.key.clone() });
        }
        Ok(None)
    }
}

pub fn open(key: &str) -> Result<Box<dyn HidDev>> {
    let want: i64 = key.parse().map_err(|_| Error::NotFound { key: key.to_string() })?;
    let found = with_devices(|devices| {
        devices.iter().copied().find(|d| number_property(*d, "LocationID") == Some(want)).map(|d| {
            // Retained so it outlives the manager this closure runs under.
            unsafe { CFRetain(d) };
            d
        })
    })
    .flatten();
    let Some(device) = found else { return Err(Error::NotFound { key: key.to_string() }) };
    let rc = unsafe { IOHIDDeviceOpen(device, K_IOHID_OPTIONS_TYPE_NONE) };
    if rc != KERN_SUCCESS {
        unsafe { CFRelease(device) };
        return Err(Error::Open {
            path: key.to_string(),
            source: std::io::Error::other(format!("IOKit refused to open it: {rc:#010x}")),
        });
    }
    Ok(Box::new(MacHid { device, key: key.to_string(), inbox: None }))
}

pub fn enumerate(ids: &[(u16, u16)]) -> Vec<HidEntry> {
    with_devices(|devices| {
        let mut out = Vec::new();
        for d in devices.iter().copied() {
            let (Some(v), Some(p)) =
                (number_property(d, "VendorID"), number_property(d, "ProductID"))
            else {
                continue;
            };
            let (v, p) = (v as u16, p as u16);
            if !ids.is_empty() && !ids.contains(&(v, p)) {
                continue;
            }
            let Some(loc) = number_property(d, "LocationID") else { continue };
            out.push(HidEntry {
                key: loc.to_string(),
                vendor: v,
                product: p,
                name: string_property(d, "Product").unwrap_or_default(),
                serial: string_property(d, "SerialNumber").unwrap_or_default(),
            });
        }
        out
    })
    .unwrap_or_default()
}
