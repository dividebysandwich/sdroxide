//! Core domain vocabulary shared by every sdroxide component, native and WASM.
//!
//! This crate must stay free of I/O, threads, and native-only dependencies:
//! it compiles for `wasm32-unknown-unknown`.

mod band;
mod band_segments;
mod caps;
mod command;
mod contacts;
mod controller;
mod digi;
mod geo;
mod worldmask;
mod memory;
mod meters;
mod mode;
mod radio;
mod skimmer;
mod spectrum;
mod sstv;
mod state;
mod ui;

pub use band::Band;
pub use band_segments::{
    FT4_DIALS, FT8_DIALS, JS8_DIALS, PSK_RANGES, RTTY_RANGES, SSTV_CALLING, Segment, SegmentKind,
    WSPR_DIALS, is_auto_digi, is_cw_segment, is_digi_segment, is_psk_segment, is_rtty_segment,
    segment_kind_at,
};
pub use caps::{DeviceCaps, Direction, GainElement};
pub use contacts::FsqContact;
pub use command::Command;
pub use controller::{AudioDevices, RadioController, RadioEvent};
pub use digi::{
    Decode, DigiConfig, DigiStatus, FsqMsg, QsoRecord, QsoStep, ThorMode, TranscriptLine,
    adif_band, fmt_report, qso_log_to_adif, qso_log_to_text, utc_ymd_hms, ymd_hms_to_unix,
};
pub use geo::{
    distance_km, grid_bearing, grid_distance_km, grid_to_latlon, great_circle_points, is_land,
    land_cell, land_mask_dims,
};
pub use memory::{BandStackEntry, MemoryChannel};
pub use meters::{Meters, TxMeters, TxTelemetry};
pub use mode::{AgcMode, Mode, NrLevel};
pub use radio::{
    Backend, CatConfig, CatFamily, DigiMode, HpsdrConfig, HpsdrDevice, LineState, ModeControl,
    Parity, PttMethod, RadioConfig, SerialConfig, SoundFormat, StopBits, TciConfig,
};
pub use skimmer::{SkimmerKind, SkimmerSpot};
pub use spectrum::{SpectrumConfig, SpectrumFrame};
pub use sstv::{SstvMode, SstvStatus};
pub use state::{OffsetState, RadioState, RxId, RxState, SQUELCH_OPEN_DB, TxState, Vfo};
pub use ui::{Speed, UiSettings};
