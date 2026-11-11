//! Data2G for sdroxide: the native C++ core of arodland/Data2G, one station per
//! [`Data2g`], driven through a small C shim (`shim/data2g_shim.cpp`).
//!
//! The engine runs in Data2G's sync mode: each [`TncModem::process`] call is
//! one `Engine::step`, so the modem's clock is exactly the samples handed to
//! it — the same contract upstream's own channel simulator relies on — and two
//! stations can be run back to back in a test faster than real time.

use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::sync::Once;

use sdroxide_tnc::{TncKind, TncModem};

mod ffi {
    use std::ffi::{c_char, c_int, c_void};

    #[repr(C)]
    pub struct Station {
        _private: [u8; 0],
    }

    pub type LogFn = extern "C" fn(*const c_char, c_int, *const c_char);
    pub type LineFn = extern "C" fn(*mut c_void, *const c_char, usize);
    pub type DataFn = extern "C" fn(*mut c_void, *const u8, usize);

    unsafe extern "C" {
        pub fn sdrx_d2g_set_log(f: LogFn, min_level: c_int);
        pub fn sdrx_d2g_new(
            mycall: *const c_char,
            seed: i64,
            ptt_delay_s: f64,
            err: *mut c_char,
            errlen: usize,
        ) -> *mut Station;
        pub fn sdrx_d2g_free(d: *mut Station);
        pub fn sdrx_d2g_step(
            d: *mut Station,
            input: *const f64,
            out: *mut f64,
            n: usize,
            ptt: *mut c_int,
        ) -> c_int;
        pub fn sdrx_d2g_command(d: *mut Station, line: *const c_char) -> c_int;
        pub fn sdrx_d2g_data_in(d: *mut Station, data: *const u8, len: usize) -> c_int;
        pub fn sdrx_d2g_client_gone(d: *mut Station);
        pub fn sdrx_d2g_abort(d: *mut Station) -> c_int;
        pub fn sdrx_d2g_take_output(d: *mut Station, line: LineFn, data: DataFn, user: *mut c_void);
        pub fn sdrx_d2g_busy(d: *const Station) -> c_int;
        pub fn sdrx_d2g_state(d: *const Station) -> *const c_char;
        pub fn sdrx_d2g_version() -> *const c_char;
        pub fn sdrx_d2g_protocol() -> c_int;
    }
}

/// The modem's transmit audio peaks at 1.0 (`Engine::Out::audio`); its own
/// clip-and-filter holds the crest factor.
pub const TX_PEAK: f32 = 1.0;

/// The upstream release linked in, e.g. `0.4.3`.
pub fn version() -> String {
    // SAFETY: a static NUL-terminated string.
    unsafe { CStr::from_ptr(ffi::sdrx_d2g_version()) }.to_string_lossy().into_owned()
}

/// The on-air protocol version: two stations interoperate only on the same one.
pub fn protocol() -> i32 {
    // SAFETY: no arguments, no state.
    unsafe { ffi::sdrx_d2g_protocol() }
}

extern "C" fn log_line(logger: *const c_char, level: c_int, msg: *const c_char) {
    // SAFETY: the shim passes NUL-terminated strings valid for the call.
    let (logger, msg) = unsafe {
        (CStr::from_ptr(logger).to_string_lossy(), CStr::from_ptr(msg).to_string_lossy())
    };
    match level {
        l if l >= 40 => tracing::error!(target: "data2g", "{logger}: {msg}"),
        l if l >= 30 => tracing::warn!(target: "data2g", "{logger}: {msg}"),
        _ => tracing::debug!(target: "data2g", "{logger}: {msg}"),
    }
}

fn init_log() {
    static ONCE: Once = Once::new();
    // SAFETY: set once, before any station exists.
    ONCE.call_once(|| unsafe { ffi::sdrx_d2g_set_log(log_line, 20) });
}

#[derive(Default)]
struct Output {
    lines: Vec<String>,
    data: Vec<u8>,
}

extern "C" fn take_line(user: *mut c_void, line: *const c_char, len: usize) {
    // SAFETY: `user` is the `Output` take_output lent; `line` holds `len` bytes.
    let (out, bytes) = unsafe {
        (&mut *(user as *mut Output), std::slice::from_raw_parts(line as *const u8, len))
    };
    out.lines.push(String::from_utf8_lossy(bytes).into_owned());
}

extern "C" fn take_data(user: *mut c_void, data: *const u8, len: usize) {
    // SAFETY: as take_line.
    let (out, bytes) =
        unsafe { (&mut *(user as *mut Output), std::slice::from_raw_parts(data, len)) };
    out.data.extend_from_slice(bytes);
}

/// One Data2G station.
pub struct Data2g {
    raw: *mut ffi::Station,
    keyed: bool,
    rx: Vec<f64>,
    tx: Vec<f64>,
    /// Command answers that arrived between steps, held so a reply never
    /// overtakes the lines a step produced before it.
    pending: Output,
}

// SAFETY: the station is only reached through `&mut self`; the core keeps no
// thread affinity in sync mode (no worker, no recorder thread).
unsafe impl Send for Data2g {}

impl Data2g {
    /// A station for `mycall`. `seed`: a fixed one for reproducible tests,
    /// `None` for a random one. `ptt_delay_s`: the silence before each burst
    /// while the transmitter settles.
    pub fn new(mycall: &str, seed: Option<u64>, ptt_delay_s: f64) -> Result<Self, String> {
        init_log();
        let call = CString::new(mycall.trim().to_ascii_uppercase()).map_err(|e| e.to_string())?;
        let mut err = [0 as c_char; 256];
        let seed = seed.map_or(-1, |s| (s & (i64::MAX as u64)) as i64);
        // SAFETY: valid strings and an error buffer of the size given.
        let raw = unsafe {
            ffi::sdrx_d2g_new(call.as_ptr(), seed, ptt_delay_s, err.as_mut_ptr(), err.len())
        };
        if raw.is_null() {
            // SAFETY: the shim NUL-terminates what it writes.
            let why = unsafe { CStr::from_ptr(err.as_ptr()) }.to_string_lossy().into_owned();
            return Err(format!("Data2G did not start: {why}"));
        }
        Ok(Self { raw, keyed: false, rx: Vec::new(), tx: Vec::new(), pending: Output::default() })
    }

    /// The session's state as Data2G names it (`IDLE`, `LISTEN`, `CONNECTING`,
    /// `CONNECTED`, `DISCONNECTING`, `CLOSED`).
    pub fn state(&self) -> String {
        // SAFETY: the string lives in the station until the next call.
        unsafe { CStr::from_ptr(ffi::sdrx_d2g_state(self.raw)) }.to_string_lossy().into_owned()
    }

    fn collect(&mut self) {
        let out = &mut self.pending as *mut Output as *mut c_void;
        // SAFETY: `out` outlives the call; the callbacks only append to it.
        unsafe { ffi::sdrx_d2g_take_output(self.raw, take_line, take_data, out) };
    }
}

impl Drop for Data2g {
    fn drop(&mut self) {
        // SAFETY: created by sdrx_d2g_new, freed once.
        unsafe { ffi::sdrx_d2g_free(self.raw) };
    }
}

impl TncModem for Data2g {
    fn kind(&self) -> TncKind {
        TncKind::Data2g
    }

    fn process(&mut self, rx: &[f32], tx: &mut [f32]) {
        let n = rx.len().min(tx.len());
        if n == 0 {
            return;
        }
        self.rx.clear();
        self.rx.extend(rx[..n].iter().map(|&v| f64::from(v)));
        self.tx.resize(n, 0.0);
        let mut ptt: c_int = 0;
        // SAFETY: both buffers hold n samples.
        unsafe {
            ffi::sdrx_d2g_step(self.raw, self.rx.as_ptr(), self.tx.as_mut_ptr(), n, &mut ptt)
        };
        self.keyed = ptt != 0;
        for (o, v) in tx[..n].iter_mut().zip(&self.tx) {
            *o = *v as f32;
        }
        self.collect();
    }

    fn keyed(&self) -> bool {
        self.keyed
    }

    fn command(&mut self, line: &str) {
        let Ok(l) = CString::new(line) else {
            self.pending.lines.push("WRONG".into());
            return;
        };
        // SAFETY: a valid C string.
        unsafe { ffi::sdrx_d2g_command(self.raw, l.as_ptr()) };
        self.collect();
    }

    fn data(&mut self, bytes: &[u8]) {
        // SAFETY: a valid slice.
        unsafe { ffi::sdrx_d2g_data_in(self.raw, bytes.as_ptr(), bytes.len()) };
    }

    fn take_output(&mut self) -> (Vec<String>, Vec<u8>) {
        self.collect();
        let o = std::mem::take(&mut self.pending);
        (o.lines, o.data)
    }

    fn client_gone(&mut self) {
        // SAFETY: a live station.
        unsafe { ffi::sdrx_d2g_client_gone(self.raw) };
        self.collect();
    }

    fn abort(&mut self) {
        // SAFETY: a live station.
        unsafe { ffi::sdrx_d2g_abort(self.raw) };
        self.collect();
    }

    fn busy(&self) -> bool {
        // SAFETY: a live station.
        unsafe { ffi::sdrx_d2g_busy(self.raw) != 0 }
    }

    fn tx_peak(&self) -> f32 {
        TX_PEAK
    }

    fn version(&self) -> String {
        format!("Data2G {} (protocol {})", version(), protocol())
    }
}
