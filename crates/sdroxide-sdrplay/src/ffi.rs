//! Hand-written bindings for the closed `sdrplay_api` v3.x.
//!
//! The vendor library is loaded with dlopen at runtime — nothing is linked at
//! build time, so this crate builds and ships everywhere and merely finds the
//! API missing where it is not installed.
//!
//! # Layout
//!
//! Every struct below mirrors the 3.15 headers field for field, and the
//! `const` asserts at the bottom pin the sizes and offsets to values measured
//! with a C `offsetof` program compiled against `/usr/include/sdrplay_api.h`
//! on x86_64 Linux. A silent layout drift would not crash — it would quietly
//! write a gain into a padding byte — so any mismatch has to be a compile
//! error instead.
//!
//! # C enums
//!
//! C enums are deliberately *not* Rust enums here. The event callback hands us
//! enum values produced by foreign code, and transmuting an unexpected value
//! into a Rust enum is undefined behaviour; plain integers with named
//! constants cannot be invalid. One of them (`ReasonForUpdate`) contains
//! `0x80000000`, which does not fit `int`, so under GCC that enum's underlying
//! type is **unsigned** — represented as `u32` bitflags. `If_kHzT` contains
//! `-1`, so it is `i32`. All are four bytes, which is what the ABI cares
//! about.

#![allow(dead_code)]

use std::ffi::{c_char, c_void};

/// `HANDLE` — an opaque device handle owned by the service.
pub type Handle = *mut c_void;

/// `sdrplay_api_ErrT`.
pub type ErrT = i32;
pub const ERR_SUCCESS: ErrT = 0;
pub const ERR_FAIL: ErrT = 1;
pub const ERR_INVALID_PARAM: ErrT = 2;
pub const ERR_OUT_OF_RANGE: ErrT = 3;
pub const ERR_HW_ERROR: ErrT = 7;
pub const ERR_ALREADY_INITIALISED: ErrT = 9;
pub const ERR_NOT_INITIALISED: ErrT = 10;
pub const ERR_HW_VER_ERROR: ErrT = 12;
pub const ERR_SERVICE_NOT_RESPONDING: ErrT = 14;
pub const ERR_INVALID_SERVICE_VERSION: ErrT = 24;

/// `sdrplay_api_TunerSelectT`.
pub type TunerSelect = i32;
pub const TUNER_NEITHER: TunerSelect = 0;
pub const TUNER_A: TunerSelect = 1;
pub const TUNER_B: TunerSelect = 2;
pub const TUNER_BOTH: TunerSelect = 3;

/// `sdrplay_api_RspDuoModeT` (a bitmask in the API's own use).
pub type RspDuoMode = i32;
pub const RSPDUO_MODE_UNKNOWN: RspDuoMode = 0;
pub const RSPDUO_MODE_SINGLE_TUNER: RspDuoMode = 1;
pub const RSPDUO_MODE_DUAL_TUNER: RspDuoMode = 2;
pub const RSPDUO_MODE_MASTER: RspDuoMode = 4;
pub const RSPDUO_MODE_SLAVE: RspDuoMode = 8;

/// `sdrplay_api_ReasonForUpdateT` — unsigned, see the module doc.
pub type Reason = u32;
pub const UPDATE_NONE: Reason = 0;
pub const UPDATE_DEV_FS: Reason = 0x0000_0001;
pub const UPDATE_DEV_PPM: Reason = 0x0000_0002;
pub const UPDATE_RSP1A_BIAS_T: Reason = 0x0000_0010;
pub const UPDATE_RSP1A_RF_NOTCH: Reason = 0x0000_0020;
pub const UPDATE_RSP1A_DAB_NOTCH: Reason = 0x0000_0040;
pub const UPDATE_RSP2_BIAS_T: Reason = 0x0000_0080;
pub const UPDATE_RSP2_AM_PORT: Reason = 0x0000_0100;
pub const UPDATE_RSP2_ANTENNA: Reason = 0x0000_0200;
pub const UPDATE_RSP2_RF_NOTCH: Reason = 0x0000_0400;
pub const UPDATE_TUNER_GR: Reason = 0x0000_8000;
/// `sdrplay_api_Update_Tuner_GrLimits` — the *limits* moved, not the value.
/// What `gain.minGr` is sent with.
pub const UPDATE_TUNER_GR_LIMITS: Reason = 0x0001_0000;
pub const UPDATE_TUNER_FRF: Reason = 0x0002_0000;
pub const UPDATE_TUNER_BW_TYPE: Reason = 0x0004_0000;
pub const UPDATE_TUNER_IF_TYPE: Reason = 0x0008_0000;
pub const UPDATE_CTRL_DECIMATION: Reason = 0x0080_0000;
pub const UPDATE_CTRL_AGC: Reason = 0x0100_0000;
pub const UPDATE_CTRL_OVERLOAD_ACK: Reason = 0x0400_0000;
pub const UPDATE_RSPDUO_BIAS_T: Reason = 0x0800_0000;
pub const UPDATE_RSPDUO_AM_PORT: Reason = 0x1000_0000;
pub const UPDATE_RSPDUO_T1_AM_NOTCH: Reason = 0x2000_0000;
pub const UPDATE_RSPDUO_RF_NOTCH: Reason = 0x4000_0000;
pub const UPDATE_RSPDUO_DAB_NOTCH: Reason = 0x8000_0000;

/// `sdrplay_api_ReasonForUpdateExtension1T`.
pub type ReasonExt1 = u32;
pub const UPDATE_EXT1_NONE: ReasonExt1 = 0;
pub const UPDATE_RSPDX_HDR_ENABLE: ReasonExt1 = 0x0000_0001;
pub const UPDATE_RSPDX_BIAS_T: ReasonExt1 = 0x0000_0002;
pub const UPDATE_RSPDX_ANTENNA: ReasonExt1 = 0x0000_0004;
pub const UPDATE_RSPDX_RF_NOTCH: ReasonExt1 = 0x0000_0008;
pub const UPDATE_RSPDX_DAB_NOTCH: ReasonExt1 = 0x0000_0010;
pub const UPDATE_RSPDX_HDR_BW: ReasonExt1 = 0x0000_0020;

/// `sdrplay_api_Bw_MHzT` — the value *is* the bandwidth in kHz.
pub type BwType = i32;
pub const BW_UNDEFINED: BwType = 0;
/// `sdrplay_api_If_kHzT` — the value is the IF in kHz; `-1` is "undefined".
pub type IfType = i32;
pub const IF_ZERO: IfType = 0;
/// The 450 kHz low IF. What the tuner uses where a zero IF is impractical,
/// and — per SDRuno's HDR documentation — what the RSPdx's HDR path expects.
/// Nothing here selects it: the driver pins a zero IF, and running the HDR
/// path at 450 kHz by hand did not make it deliver anything either.
pub const IF_0_450: IfType = 450;
/// The low IF the API's own downconverter works from. Mandatory with both of
/// an RSPduo's tuners running, where the ADC is fixed at 6 MHz.
pub const IF_1_620: IfType = 1620;
/// The same for an 8 MHz ADC clock.
pub const IF_2_048: IfType = 2048;
/// `sdrplay_api_LoModeT`.
pub type LoMode = i32;
pub const LO_AUTO: LoMode = 1;
/// The fixed LO settings. The API picks one under `LO_AUTO`; the HDR path's
/// short list of usable centres looks like the product of a particular choice,
/// so they are nameable here.
pub const LO_120MHZ: LoMode = 2;
pub const LO_144MHZ: LoMode = 3;
pub const LO_168MHZ: LoMode = 4;
/// `sdrplay_api_MinGainReductionT`.
pub type MinGr = i32;
pub const NORMAL_MIN_GR: MinGr = 20;
/// The extended floor, which lets `gRdB` go to 0 instead of stopping at 20 —
/// the last 20 dB of IF gain the hardware has. The API's own default is
/// `NORMAL_MIN_GR`; this is opt-in because it also moves the AGC set-point
/// ceiling (see the set-point clamp in `device`).
pub const EXTENDED_MIN_GR: MinGr = 0;
/// `sdrplay_api_AgcControlT`.
pub type AgcControl = i32;
pub const AGC_DISABLE: AgcControl = 0;
pub const AGC_100HZ: AgcControl = 1;
pub const AGC_50HZ: AgcControl = 2;
pub const AGC_5HZ: AgcControl = 3;
/// `sdrplay_api_AdsbModeT`.
pub type AdsbMode = i32;

/// `sdrplay_api_EventT`.
pub type EventId = i32;
pub const EVENT_GAIN_CHANGE: EventId = 0;
pub const EVENT_POWER_OVERLOAD: EventId = 1;
pub const EVENT_DEVICE_REMOVED: EventId = 2;
pub const EVENT_RSPDUO_MODE_CHANGE: EventId = 3;
pub const EVENT_DEVICE_FAILURE: EventId = 4;

/// `sdrplay_api_PowerOverloadCbEventIdT`.
pub type OverloadEvent = i32;
pub const OVERLOAD_DETECTED: OverloadEvent = 0;
pub const OVERLOAD_CORRECTED: OverloadEvent = 1;

/// Antenna selector values (`sdrplay_api_Rsp2_AntennaSelectT`,
/// `sdrplay_api_RspDx_AntennaSelectT`, `sdrplay_api_Rsp2_AmPortSelectT`,
/// `sdrplay_api_RspDuo_AmPortSelectT`).
pub const RSP2_ANTENNA_A: i32 = 5;
pub const RSP2_ANTENNA_B: i32 = 6;
pub const RSP2_AMPORT_1_HIZ: i32 = 1;
pub const RSP2_AMPORT_2: i32 = 0;
pub const RSPDX_ANTENNA_A: i32 = 0;
pub const RSPDX_ANTENNA_B: i32 = 1;
pub const RSPDX_ANTENNA_C: i32 = 2;
pub const RSPDUO_AMPORT_1_HIZ: i32 = 1;
pub const RSPDUO_AMPORT_2: i32 = 0;

pub const MAX_DEVICES: usize = 16;
pub const MAX_SER_NO_LEN: usize = 64;

/// `sdrplay_api_DeviceT`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DeviceT {
    pub ser_no: [c_char; MAX_SER_NO_LEN],
    pub hw_ver: u8,
    pub tuner: TunerSelect,
    pub rsp_duo_mode: RspDuoMode,
    pub valid: u8,
    pub rsp_duo_sample_freq: f64,
    pub dev: Handle,
}

impl DeviceT {
    pub fn zeroed() -> DeviceT {
        // Every field is valid at all-zeroes: empty serial, hwVer 0, Neither,
        // Unknown mode, invalid, 0.0, null handle.
        unsafe { std::mem::zeroed() }
    }

    /// The serial as Rust text (the API guarantees NUL termination).
    pub fn serial(&self) -> String {
        let bytes: &[u8] =
            unsafe { std::slice::from_raw_parts(self.ser_no.as_ptr().cast(), MAX_SER_NO_LEN) };
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(MAX_SER_NO_LEN);
        String::from_utf8_lossy(&bytes[..end]).trim().to_string()
    }
}

/// `sdrplay_api_DeviceParamsT` — pointers into service-owned storage.
#[repr(C)]
pub struct DeviceParamsT {
    pub dev_params: *mut DevParamsT,
    pub rx_channel_a: *mut RxChannelParamsT,
    pub rx_channel_b: *mut RxChannelParamsT,
}

/// `sdrplay_api_FsFreqT`.
#[repr(C)]
pub struct FsFreqT {
    pub fs_hz: f64,
    pub sync_update: u8,
    pub re_cal: u8,
}

/// `sdrplay_api_SyncUpdateT`.
#[repr(C)]
pub struct SyncUpdateT {
    pub sample_num: u32,
    pub period: u32,
}

/// `sdrplay_api_ResetFlagsT`.
#[repr(C)]
pub struct ResetFlagsT {
    pub reset_gain_update: u8,
    pub reset_rf_update: u8,
    pub reset_fs_update: u8,
}

/// `sdrplay_api_Rsp1aParamsT` — also what an RSP1B carries.
#[repr(C)]
pub struct Rsp1aParamsT {
    pub rf_notch_enable: u8,
    pub rf_dab_notch_enable: u8,
}

/// `sdrplay_api_Rsp2ParamsT`.
#[repr(C)]
pub struct Rsp2ParamsT {
    pub ext_ref_output_en: u8,
}

/// `sdrplay_api_RspDuoParamsT`.
#[repr(C)]
pub struct RspDuoParamsT {
    pub ext_ref_output_en: i32,
}

/// `sdrplay_api_RspDxParamsT`.
#[repr(C)]
pub struct RspDxParamsT {
    pub hdr_enable: u8,
    pub bias_t_enable: u8,
    pub antenna_sel: i32,
    pub rf_notch_enable: u8,
    pub rf_dab_notch_enable: u8,
}

/// `sdrplay_api_DevParamsT`.
#[repr(C)]
pub struct DevParamsT {
    pub ppm: f64,
    pub fs_freq: FsFreqT,
    pub sync_update: SyncUpdateT,
    pub reset_flags: ResetFlagsT,
    pub mode: i32,
    pub samples_per_pkt: u32,
    pub rsp1a_params: Rsp1aParamsT,
    pub rsp2_params: Rsp2ParamsT,
    pub rsp_duo_params: RspDuoParamsT,
    pub rsp_dx_params: RspDxParamsT,
}

/// `sdrplay_api_GainValuesT` (output parameter).
#[repr(C)]
pub struct GainValuesT {
    pub curr: f32,
    pub max: f32,
    pub min: f32,
}

/// `sdrplay_api_GainT`.
#[repr(C)]
pub struct GainT {
    pub g_rdb: i32,
    pub lna_state: u8,
    pub sync_update: u8,
    pub min_gr: MinGr,
    pub gain_vals: GainValuesT,
}

/// `sdrplay_api_RfFreqT`.
#[repr(C)]
pub struct RfFreqT {
    pub rf_hz: f64,
    pub sync_update: u8,
}

/// `sdrplay_api_DcOffsetTunerT`.
#[repr(C)]
pub struct DcOffsetTunerT {
    pub dc_cal: u8,
    pub speed_up: u8,
    pub track_time: i32,
    pub refresh_rate_time: i32,
}

/// `sdrplay_api_TunerParamsT`.
#[repr(C)]
pub struct TunerParamsT {
    pub bw_type: BwType,
    pub if_type: IfType,
    pub lo_mode: LoMode,
    pub gain: GainT,
    pub rf_freq: RfFreqT,
    pub dc_offset_tuner: DcOffsetTunerT,
}

/// `sdrplay_api_DcOffsetT`.
#[repr(C)]
pub struct DcOffsetT {
    pub dc_enable: u8,
    pub iq_enable: u8,
}

/// `sdrplay_api_DecimationT`.
#[repr(C)]
pub struct DecimationT {
    pub enable: u8,
    pub decimation_factor: u8,
    pub wide_band_signal: u8,
}

/// `sdrplay_api_AgcT`.
#[repr(C)]
pub struct AgcT {
    pub enable: AgcControl,
    pub set_point_dbfs: i32,
    pub attack_ms: u16,
    pub decay_ms: u16,
    pub decay_delay_ms: u16,
    pub decay_threshold_db: u16,
    pub sync_update: i32,
}

/// `sdrplay_api_ControlParamsT`.
#[repr(C)]
pub struct ControlParamsT {
    pub dc_offset: DcOffsetT,
    pub decimation: DecimationT,
    pub agc: AgcT,
    pub adsb_mode: AdsbMode,
}

/// `sdrplay_api_Rsp1aTunerParamsT` — also what an RSP1B carries.
#[repr(C)]
pub struct Rsp1aTunerParamsT {
    pub bias_t_enable: u8,
}

/// `sdrplay_api_Rsp2TunerParamsT`.
#[repr(C)]
pub struct Rsp2TunerParamsT {
    pub bias_t_enable: u8,
    pub am_port_sel: i32,
    pub antenna_sel: i32,
    pub rf_notch_enable: u8,
}

/// `sdrplay_api_RspDuo_ResetSlaveFlagsT`.
#[repr(C)]
pub struct RspDuoResetSlaveFlagsT {
    pub reset_gain_update: u8,
    pub reset_rf_update: u8,
}

/// `sdrplay_api_RspDuoTunerParamsT`.
#[repr(C)]
pub struct RspDuoTunerParamsT {
    pub bias_t_enable: u8,
    pub tuner1_am_port_sel: i32,
    pub tuner1_am_notch_enable: u8,
    pub rf_notch_enable: u8,
    pub rf_dab_notch_enable: u8,
    pub reset_slave_flags: RspDuoResetSlaveFlagsT,
}

/// `sdrplay_api_RspDxTunerParamsT`.
#[repr(C)]
pub struct RspDxTunerParamsT {
    pub hdr_bw: HdrBw,
}

/// `sdrplay_api_RspDx_HdrModeBwT` — the analog filter in front of the HDR
/// path, which is a *different* control from the tuner's own `bwType`.
pub type HdrBw = i32;
pub const HDR_BW_0_200: HdrBw = 0;
pub const HDR_BW_0_500: HdrBw = 1;
pub const HDR_BW_1_200: HdrBw = 2;
/// The API's default, and what an RSPdx runs with when nobody sets one.
pub const HDR_BW_1_700: HdrBw = 3;

/// `sdrplay_api_RxChannelParamsT`.
#[repr(C)]
pub struct RxChannelParamsT {
    pub tuner_params: TunerParamsT,
    pub ctrl_params: ControlParamsT,
    pub rsp1a_tuner_params: Rsp1aTunerParamsT,
    pub rsp2_tuner_params: Rsp2TunerParamsT,
    pub rsp_duo_tuner_params: RspDuoTunerParamsT,
    pub rsp_dx_tuner_params: RspDxTunerParamsT,
}

/// `sdrplay_api_StreamCbParamsT`.
#[repr(C)]
pub struct StreamCbParamsT {
    pub first_sample_num: u32,
    pub gr_changed: i32,
    pub rf_changed: i32,
    pub fs_changed: i32,
    pub num_samples: u32,
}

/// `sdrplay_api_GainCbParamT`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GainCbParamT {
    pub g_rdb: u32,
    pub lna_g_rdb: u32,
    pub curr_gain: f64,
}

/// `sdrplay_api_PowerOverloadCbParamT`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PowerOverloadCbParamT {
    pub power_overload_change_type: OverloadEvent,
}

/// `sdrplay_api_RspDuoModeCbParamT`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RspDuoModeCbParamT {
    pub mode_change_type: i32,
}

/// `sdrplay_api_EventParamsT` — a C union; which member is live is named by
/// the `EventId` alongside it.
#[repr(C)]
pub union EventParamsT {
    pub gain: GainCbParamT,
    pub power_overload: PowerOverloadCbParamT,
    pub rsp_duo_mode: RspDuoModeCbParamT,
}

pub type StreamCallback = unsafe extern "C" fn(
    xi: *mut i16,
    xq: *mut i16,
    params: *mut StreamCbParamsT,
    num_samples: u32,
    reset: u32,
    cb_context: *mut c_void,
);

pub type EventCallback = unsafe extern "C" fn(
    event_id: EventId,
    tuner: TunerSelect,
    params: *mut EventParamsT,
    cb_context: *mut c_void,
);

/// `sdrplay_api_CallbackFnsT`. The API treats all three as required; a no-op
/// StreamB is installed even though single-tuner mode never fires it.
#[repr(C)]
pub struct CallbackFnsT {
    pub stream_a: Option<StreamCallback>,
    pub stream_b: Option<StreamCallback>,
    pub event: Option<EventCallback>,
}

/// The vendor library, loaded once and kept for the life of the process.
///
/// The function pointers are `Copy` values resolved at load time, so there are
/// no `Symbol` lifetimes to carry around — the `Library` field is what keeps
/// them valid, and it is dropped never (see [`crate::api`]).
pub struct Api {
    _lib: libloading::Library,
    pub open: unsafe extern "C" fn() -> ErrT,
    pub close: unsafe extern "C" fn() -> ErrT,
    pub api_version: unsafe extern "C" fn(*mut f32) -> ErrT,
    pub lock_device_api: unsafe extern "C" fn() -> ErrT,
    pub unlock_device_api: unsafe extern "C" fn() -> ErrT,
    pub get_devices: unsafe extern "C" fn(*mut DeviceT, *mut u32, u32) -> ErrT,
    pub select_device: unsafe extern "C" fn(*mut DeviceT) -> ErrT,
    pub release_device: unsafe extern "C" fn(*mut DeviceT) -> ErrT,
    pub get_error_string: unsafe extern "C" fn(ErrT) -> *const c_char,
    pub get_device_params: unsafe extern "C" fn(Handle, *mut *mut DeviceParamsT) -> ErrT,
    pub init: unsafe extern "C" fn(Handle, *mut CallbackFnsT, *mut c_void) -> ErrT,
    pub uninit: unsafe extern "C" fn(Handle) -> ErrT,
    pub update: unsafe extern "C" fn(Handle, TunerSelect, Reason, ReasonExt1) -> ErrT,
}

/// Library names/paths to try, most specific first.
#[cfg(target_os = "linux")]
fn lib_candidates() -> Vec<std::ffi::OsString> {
    ["libsdrplay_api.so.3", "libsdrplay_api.so"].iter().map(Into::into).collect()
}
#[cfg(target_os = "macos")]
fn lib_candidates() -> Vec<std::ffi::OsString> {
    [
        "libsdrplay_api.dylib",
        "/usr/local/lib/libsdrplay_api.dylib",
        "/usr/local/lib/libsdrplay_api.so.3",
    ]
    .iter()
    .map(Into::into)
    .collect()
}
/// The vendor installer puts the DLL in `<Install_Dir>\x64` under Program
/// Files — a directory that is on nobody's DLL search path — and records
/// `Install_Dir` in the registry. Loading by bare name only works when some
/// other program's installer copied the DLL somewhere public, so the registry
/// pointer is the load-bearing entry here; the Program Files guesses cover a
/// service that was installed but did not (or could not) write the key.
#[cfg(target_os = "windows")]
fn lib_candidates() -> Vec<std::ffi::OsString> {
    use std::path::PathBuf;
    let mut out: Vec<std::ffi::OsString> = vec!["sdrplay_api.dll".into()];
    let arch = if cfg!(target_pointer_width = "64") { "x64" } else { "x86" };
    let mut push_install_dir = |dir: PathBuf| {
        out.push(dir.join(arch).join("sdrplay_api.dll").into_os_string());
        out.push(dir.join("sdrplay_api.dll").into_os_string());
    };
    let hklm = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
    // Both hives: which one the installer wrote depends on its bitness.
    for key in ["SOFTWARE\\SDRplay\\Service\\API", "SOFTWARE\\WOW6432Node\\SDRplay\\Service\\API"] {
        if let Ok(dir) = hklm.open_subkey(key).and_then(|k| k.get_value::<String, _>("Install_Dir"))
        {
            push_install_dir(PathBuf::from(dir));
        }
    }
    for var in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(pf) = std::env::var_os(var) {
            push_install_dir(PathBuf::from(pf).join("SDRplay").join("API"));
        }
    }
    out
}
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn lib_candidates() -> Vec<std::ffi::OsString> {
    ["libsdrplay_api.so.3", "libsdrplay_api.so"].iter().map(Into::into).collect()
}

impl Api {
    /// Load the vendor library, or say why not (used verbatim in the UI).
    pub fn load() -> Result<Api, String> {
        let mut last = String::new();
        for name in lib_candidates() {
            match unsafe { libloading::Library::new(&name) } {
                Ok(lib) => return unsafe { Api::from_lib(lib) },
                Err(e) => last = e.to_string(),
            }
        }
        Err(format!(
            "the SDRplay API library was not found ({last}) — install the SDRplay API from \
             sdrplay.com/api, then rescan"
        ))
    }

    unsafe fn from_lib(lib: libloading::Library) -> Result<Api, String> {
        // Resolve every symbol up front: a library that is missing one is the
        // wrong library, and finding out now beats finding out mid-stream.
        macro_rules! sym {
            ($name:literal) => {
                *unsafe { lib.get(concat!($name, "\0").as_bytes()) }
                    .map_err(|e| format!("{} missing from the SDRplay library: {e}", $name))?
            };
        }
        Ok(Api {
            open: sym!("sdrplay_api_Open"),
            close: sym!("sdrplay_api_Close"),
            api_version: sym!("sdrplay_api_ApiVersion"),
            lock_device_api: sym!("sdrplay_api_LockDeviceApi"),
            unlock_device_api: sym!("sdrplay_api_UnlockDeviceApi"),
            get_devices: sym!("sdrplay_api_GetDevices"),
            select_device: sym!("sdrplay_api_SelectDevice"),
            release_device: sym!("sdrplay_api_ReleaseDevice"),
            get_error_string: sym!("sdrplay_api_GetErrorString"),
            get_device_params: sym!("sdrplay_api_GetDeviceParams"),
            init: sym!("sdrplay_api_Init"),
            uninit: sym!("sdrplay_api_Uninit"),
            update: sym!("sdrplay_api_Update"),
            _lib: lib,
        })
    }

    /// The API's own text for an error code.
    pub fn err_text(&self, err: ErrT) -> String {
        let p = unsafe { (self.get_error_string)(err) };
        if p.is_null() {
            return format!("error {err}");
        }
        unsafe { std::ffi::CStr::from_ptr(p) }.to_string_lossy().into_owned()
    }
}

// Layout pins — measured on x86_64 Linux against the 3.15 headers. See the
// module doc. `HANDLE` makes a few of these pointer-width dependent, so the
// asserts are limited to 64-bit targets; a 32-bit port must re-measure.
#[cfg(target_pointer_width = "64")]
mod layout_asserts {
    use super::*;
    use std::mem::{offset_of, size_of};

    const _: () = assert!(size_of::<DeviceT>() == 96);
    const _: () = assert!(offset_of!(DeviceT, hw_ver) == 64);
    const _: () = assert!(offset_of!(DeviceT, tuner) == 68);
    const _: () = assert!(offset_of!(DeviceT, rsp_duo_mode) == 72);
    const _: () = assert!(offset_of!(DeviceT, valid) == 76);
    const _: () = assert!(offset_of!(DeviceT, rsp_duo_sample_freq) == 80);
    const _: () = assert!(offset_of!(DeviceT, dev) == 88);

    const _: () = assert!(size_of::<DeviceParamsT>() == 24);

    const _: () = assert!(size_of::<DevParamsT>() == 64);
    const _: () = assert!(offset_of!(DevParamsT, fs_freq) == 8);
    const _: () = assert!(offset_of!(DevParamsT, mode) == 36);
    const _: () = assert!(offset_of!(DevParamsT, rsp1a_params) == 44);
    const _: () = assert!(offset_of!(DevParamsT, rsp_duo_params) == 48);
    const _: () = assert!(offset_of!(DevParamsT, rsp_dx_params) == 52);

    const _: () = assert!(size_of::<RxChannelParamsT>() == 144);
    const _: () = assert!(offset_of!(RxChannelParamsT, ctrl_params) == 72);
    const _: () = assert!(offset_of!(RxChannelParamsT, rsp1a_tuner_params) == 104);
    const _: () = assert!(offset_of!(RxChannelParamsT, rsp2_tuner_params) == 108);
    const _: () = assert!(offset_of!(RxChannelParamsT, rsp_duo_tuner_params) == 124);
    const _: () = assert!(offset_of!(RxChannelParamsT, rsp_dx_tuner_params) == 140);

    const _: () = assert!(size_of::<TunerParamsT>() == 72);
    const _: () = assert!(offset_of!(TunerParamsT, gain) == 12);
    const _: () = assert!(offset_of!(TunerParamsT, rf_freq) == 40);
    const _: () = assert!(offset_of!(TunerParamsT, dc_offset_tuner) == 56);

    const _: () = assert!(size_of::<GainT>() == 24);
    const _: () = assert!(offset_of!(GainT, min_gr) == 8);
    const _: () = assert!(offset_of!(GainT, gain_vals) == 12);

    const _: () = assert!(size_of::<ControlParamsT>() == 32);
    const _: () = assert!(offset_of!(ControlParamsT, decimation) == 2);
    const _: () = assert!(offset_of!(ControlParamsT, agc) == 8);
    const _: () = assert!(offset_of!(ControlParamsT, adsb_mode) == 28);

    const _: () = assert!(size_of::<AgcT>() == 20);
    const _: () = assert!(offset_of!(AgcT, attack_ms) == 8);
    const _: () = assert!(offset_of!(AgcT, sync_update) == 16);

    const _: () = assert!(size_of::<Rsp2TunerParamsT>() == 16);
    const _: () = assert!(size_of::<RspDuoTunerParamsT>() == 16);
    const _: () = assert!(offset_of!(RspDuoTunerParamsT, reset_slave_flags) == 11);
    const _: () = assert!(size_of::<RspDxParamsT>() == 12);
    const _: () = assert!(offset_of!(RspDxParamsT, antenna_sel) == 4);
    const _: () = assert!(size_of::<RspDxTunerParamsT>() == 4);

    const _: () = assert!(size_of::<StreamCbParamsT>() == 20);
    const _: () = assert!(size_of::<EventParamsT>() == 16);
    const _: () = assert!(size_of::<GainCbParamT>() == 16);
    const _: () = assert!(size_of::<CallbackFnsT>() == 24);
}
