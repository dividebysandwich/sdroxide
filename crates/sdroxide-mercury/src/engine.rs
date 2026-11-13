//! The one Mercury engine of the process.

use std::ffi::{CString, c_char, c_int, c_void};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::Config;

mod ffi {
    use std::ffi::{c_char, c_int, c_void};

    #[repr(C)]
    pub struct RadioOps {
        pub key: Option<extern "C" fn(bool, *mut c_void) -> c_int>,
        pub get_frequency: Option<extern "C" fn(*mut u64, *mut c_void) -> bool>,
        pub user: *mut c_void,
    }

    unsafe extern "C" {
        pub fn mercury_host_set_radio(ops: *const RadioOps);
        pub fn mercury_init(
            argc: c_int,
            argv: *mut *mut c_char,
            default_config: *const c_char,
            log_path: *const c_char,
        ) -> c_int;
        pub fn mercury_host_audio_exchange(rx8k: *const i16, tx8k: *mut i16, n: usize) -> usize;
        pub fn hermes_log_init(capacity: usize) -> c_int;
        pub fn hermes_log_set_level(level: c_int);
        pub fn hermes_log_set_max_bytes(bytes: u64);
        pub fn hermes_log_set_file(path: *const c_char, min_level: c_int, jsonl: bool) -> c_int;
    }
}

/// `hermes_log_level_t`.
const LOG_INFO: c_int = 2;
const LOG_WARN: c_int = 3;

/// The running engine.
pub struct Engine {
    port: u16,
    /// Serialises the audio exchange: one block at a time, from whoever is
    /// clocking the modem now.
    audio: Mutex<()>,
}

/// Set by Mercury's TX thread through the `key` callback.
static KEYED: AtomicBool = AtomicBool::new(false);
/// The dial frequency the UI publisher may ask for, Hz (0: none).
static DIAL_HZ: AtomicU64 = AtomicU64::new(0);
static ENGINE: OnceLock<Result<Engine, String>> = OnceLock::new();

extern "C" fn key(tx: bool, _user: *mut c_void) -> c_int {
    KEYED.store(tx, Ordering::SeqCst);
    0
}

extern "C" fn get_frequency(hz: *mut u64, _user: *mut c_void) -> bool {
    let f = DIAL_HZ.load(Ordering::Relaxed);
    if f == 0 || hz.is_null() {
        return false;
    }
    // SAFETY: Mercury hands a valid pointer.
    unsafe { *hz = f };
    true
}

fn ini(cfg: &Config) -> String {
    format!(
        "; Written by sdroxide at every start: edits here are lost.\n\
         [main]\n\
         ui_enabled = false\n\
         waterfall_enabled = false\n\
         arq_tcp_base_port = {port}\n\
         broadcast_tcp_port = {bcast}\n\
         sound_system = null\n\
         verbose = false\n\
         [ptt]\n\
         method = none\n\
         [audio]\n\
         tx_delay_ms = {lead}\n\
         [store]\n\
         enabled = false\n",
        port = cfg.private_port,
        bcast = cfg.private_port.saturating_add(2),
        lead = cfg.tx_delay_ms,
    )
}

/// Start Mercury, or return the engine already running. The first call's
/// configuration holds for the life of the process.
pub fn start(cfg: &Config) -> Result<&'static Engine, String> {
    ENGINE
        .get_or_init(|| {
            std::fs::create_dir_all(&cfg.config_dir)
                .map_err(|e| format!("create {}: {e}", cfg.config_dir.display()))?;
            let path = cfg.config_dir.join("mercury.ini");
            std::fs::write(&path, ini(cfg))
                .map_err(|e| format!("write {}: {e}", path.display()))?;
            // Mercury keeps a logger its host started: only warnings reach
            // stderr, and its full log goes to a capped file beside the ini.
            let log = CString::new(cfg.config_dir.join("mercury.log").to_string_lossy().as_bytes())
                .map_err(|e| e.to_string())?;
            // SAFETY: plain calls into Mercury's logger, before the engine runs.
            unsafe {
                if ffi::hermes_log_init(1024) == 0 {
                    ffi::hermes_log_set_level(LOG_WARN);
                    ffi::hermes_log_set_max_bytes(5 * 1024 * 1024);
                    ffi::hermes_log_set_file(log.as_ptr(), LOG_INFO, false);
                }
            }
            let ops = ffi::RadioOps {
                key: Some(key),
                get_frequency: Some(get_frequency),
                user: std::ptr::null_mut(),
            };
            // SAFETY: Mercury copies the struct.
            unsafe { ffi::mercury_host_set_radio(&ops) };

            let args: Vec<CString> = ["mercury", "-C", &path.to_string_lossy()]
                .iter()
                .map(|a| CString::new(*a).map_err(|e| e.to_string()))
                .collect::<Result<_, _>>()?;
            let mut argv: Vec<*mut c_char> =
                args.iter().map(|a| a.as_ptr() as *mut c_char).collect();
            argv.push(std::ptr::null_mut());
            let def = CString::new(path.to_string_lossy().as_bytes()).map_err(|e| e.to_string())?;
            // SAFETY: argv holds `args.len()` valid strings and a NULL, all of
            // which outlive the call (Mercury copies what it keeps).
            let rc = unsafe {
                ffi::mercury_init(
                    args.len() as c_int,
                    argv.as_mut_ptr(),
                    def.as_ptr(),
                    std::ptr::null(),
                )
            };
            match rc {
                0 => {
                    tracing::info!(port = cfg.private_port, "Mercury {} started", crate::VERSION);
                    Ok(Engine { port: cfg.private_port, audio: Mutex::new(()) })
                }
                -2 => Err(format!("Mercury could not read {}", path.display())),
                _ => Err(format!(
                    "Mercury did not start (is 127.0.0.1:{} or {} in use?)",
                    cfg.private_port,
                    cfg.private_port.saturating_add(1)
                )),
            }
        })
        .as_ref()
        .map_err(Clone::clone)
}

impl Engine {
    /// Mercury's own command port on 127.0.0.1 (data on the next).
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Whether Mercury wants the transmitter keyed.
    pub fn keyed(&self) -> bool {
        KEYED.load(Ordering::SeqCst)
    }

    /// The dial frequency Mercury's status reports, Hz.
    pub fn set_dial_hz(&self, hz: u64) {
        DIAL_HZ.store(hz, Ordering::Relaxed);
    }

    /// One audio block: `rx` heard (None: nothing, e.g. while keyed), `tx`
    /// receives as many samples of Mercury's transmit audio, then silence.
    /// Returns how many were Mercury's.
    pub fn exchange(&self, rx: Option<&[i16]>, tx: &mut [i16]) -> usize {
        let n = rx.map_or(tx.len(), |r| r.len().min(tx.len()));
        let _g = self.audio.lock().unwrap_or_else(|e| e.into_inner());
        // SAFETY: both buffers hold at least n samples; Mercury never waits.
        unsafe {
            ffi::mercury_host_audio_exchange(
                rx.map_or(std::ptr::null(), <[i16]>::as_ptr),
                tx.as_mut_ptr(),
                n,
            )
        }
    }

    /// Throw away transmit audio Mercury queued while nobody was clocking it.
    pub fn drain_tx(&self) {
        let mut buf = [0i16; 800];
        for _ in 0..200 {
            if self.exchange(None, &mut buf) == 0 {
                break;
            }
        }
    }
}
