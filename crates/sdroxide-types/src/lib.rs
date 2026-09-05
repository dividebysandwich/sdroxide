//! Core domain vocabulary shared by every sdroxide component, native and WASM.
//!
//! This crate must stay free of I/O, threads, and native-only dependencies:
//! it compiles for `wasm32-unknown-unknown`.

mod access;
mod adsb;
mod ais;
mod aprs;
mod awards;
mod band;
mod band_segments;
mod bandplan;
pub mod broadcast;
mod callsign;
mod caps;
mod chirp;
mod command;
mod contacts;
mod controller;
mod digi;
mod drm;
mod entity;
mod entity_flags;
mod geo;
mod input;
mod ism;
mod js8;
mod limerfe;
mod memory;
mod meters;
mod mode;
mod netcfg;
mod pictures;
mod probe;
mod prop_store;
mod propagation;
pub mod publicsdr;
mod qo100;
mod radio;
mod rds;
pub mod region;
mod relay;
mod repeater;
mod rifp;
mod rigctld;
mod rotator;
mod satcfg;
mod satlock;
mod scanner;
mod skimmer;
mod spectrum;
mod speech;
mod spot;
mod sstv;
mod state;
mod station;
mod tciserver;
pub mod text;
mod tone;
mod ui;
mod vdl2;
mod voice;
mod wefax;
mod winlink;
mod wsjtx;
mod wspr;

pub use access::{AUTH_BUSY, AUTH_REFUSED, AuthPhase, RemoteAccess, RemoteServer, is_auth_busy};
pub use adsb::{
    ADSB_DROP_LIST_S, ADSB_DROP_MAP_S, ADSB_FREQ_HZ, ADSB_GOOD_RATE_HZ, ADSB_HISTORY_POINTS,
    ADSB_MAX_AIRCRAFT, ADSB_MAX_RATE_HZ, ADSB_MIN_RATE_HZ, ADSB_TRACK_MAX, ADSB_VECTOR_MINUTES,
    AdsbAircraft, AdsbSettings, AdsbSource, AdsbStatus,
};
pub use ais::{
    AIS_ALL_CHANNELS, AIS_BIT_RATE, AIS_CHANNEL_A_HZ, AIS_CHANNEL_B_HZ, AIS_CHANNEL_SPACING_HZ,
    AIS_DROP_LIST_S, AIS_DROP_MAP_S, AIS_GOOD_SPS, AIS_MAX_VESSELS, AIS_MIN_RATE_HZ,
    AIS_PLAN_CENTER_HZ, AIS_THRESHOLD_DB, AIS_TRACK_MAX, AIS_TRAIL_MINUTES, AIS_VECTOR_MINUTES,
    AisChannelStatus, AisKind, AisSettings, AisStatus, AisVessel, aid_type_label, nav_status_label,
    ship_type_hazard, ship_type_label,
};
pub use aprs::{
    APRS_MESSAGE_MAX, APRS_MSG_RETRIES, APRS_STATION_MAX, APRS_TRACK_MAX, APRS_TRAFFIC_MAX,
    AprsEntryKind, AprsMessage, AprsMsgState, AprsPosition, AprsStation, AprsStatus, AprsSymbol,
    AprsSymbolKind, AprsTraffic, AprsWeather,
};
pub use awards::{
    Awards, Coverage, EntitySlot, Highlight, LogIndex, Novelty, Status as AwardStatus, US_STATES,
    compute_awards, counts, coverage_counts, entity_coverage, entity_name,
};
pub use band::Band;
pub use band_segments::{
    APRS_DIALS, DigiChannel, DigiPreset, FSQ_DIALS, FT2_DIALS, FT4_DIALS, FT8_DIALS,
    FT8_DXPED_DIALS, FT8_VHF_DIALS, JS8_DIALS, PSK_DIALS, PSK_RANGES_R1, PSK_RANGES_R23,
    RIFP_CALLING, RTTY_DIALS, RTTY_RANGES_R1, RTTY_RANGES_R23, SEGMENTS_R1, SEGMENTS_R2,
    SEGMENTS_R3, SSTV_DIALS, Segment, SegmentKind, WSPR_DIALS, aprs_dial, aprs_dial_in,
    digi_channels, digi_channels_for, digi_channels_in, digi_channels_in_region, digi_presets,
    is_aprs_channel, is_auto_digi, is_cw_segment, is_digi_segment, is_psk_segment,
    is_psk_segment_in, is_rtty_segment, is_rtty_segment_in, psk_ranges, psk_ranges_in, rtty_ranges,
    rtty_ranges_in, segment_kind_at, segment_kind_at_in, segments, segments_in, set_digi_presets,
    span_within_segment, span_within_segment_in, sstv_dials_in,
};
pub use bandplan::{BandPlan, BandPlanError, RegionPlan, band_plan, set_band_plan};
pub use broadcast::{BroadcastStation, BroadcastStations};
pub use callsign::{CallsignInfo, LoginTarget, LoginTestResult, UploadResult, UploadTarget};
pub use caps::{DeviceCaps, DeviceSetting, Direction, GainElement, GainUnit, SettingKind};
pub use chirp::{chirp_csv_to_memories, memories_to_chirp_csv};
pub use command::Command;
pub use contacts::FsqContact;
pub use controller::{AudioDevices, PeerRadio, RadioController, RadioEvent};
pub use digi::{
    CONTEST_SERIAL_MAX, ClockHealth, ContestMode, CwStatus, Decode, DecodeSort, DigiConfig,
    DigiStatus, DxpedMode, FOX_MAX_SLOTS, FOX_ZONE_MAX_HZ, FoxCaller, FsqHeard, FsqMsg,
    HOUND_ZONE_MAX_HZ, HellVariant, NAVTEX_MESSAGE_MAX, NAVTEX_TONE_HZ, NavtexMessage,
    NavtexStatus, PACKET_HEARD_MAX, PACKET_TERM_LINE_MAX, PACKET_TERM_MAX, PacketBaud, PacketHeard,
    PacketLink, PacketLinkOwner, PacketStatus, PacketTermKind, PacketTermLine, QsoLive, QsoRecord,
    QsoStep, QueuedCall, RTTY_CENTER_HZ, RadeStatus, TX_AUDIO_LEVEL_MIN, TX_AUDIO_LEVEL_MIN_DB,
    ThorMode, TranscriptLine, adif_band, adif_records, adif_to_qso_log, clock_health, cq_is_for_us,
    eu_vhf_rs, fmt_report, next_contest_serial, qso_log_to_adif, qso_log_to_text,
    qso_to_adif_record, tx_level_db,
    tx_level_from_db, utc_ymd_hms, worked_before, ymd_hms_to_unix,
};
pub use drm::{
    DrmChannel, DrmCodec, DrmConstellation, DrmRobustness, DrmService, DrmStatus, DrmSync, DrmTime,
    spectrum_occupancy_khz,
};
pub use entity::{
    EntityInfo, EntityPlace, all_entities, resolve_callsign, resolve_place, resolve_prefix,
};
pub use geo::{
    bearing_deg, distance_km, great_circle_points, grid_bearing, grid_distance_km, grid_to_latlon,
    latlon_to_grid,
};
pub use input::{
    Action, ActionInput, ActionKind, BindingTuning, ButtonMode, InputSettings, KeyBinding,
    KeyChord, MidiBinding, MidiMsg, MidiMsgKind, MidiSettings, MouseButton, MouseButtonBinding,
    RelativeMode, WheelAction, WheelSettings,
};
pub use ism::{
    ISM_MAX_DEVICES_DEFAULT, ISM_THRESHOLD_DB_DEFAULT, IsmBurstClass, IsmChannelStatus, IsmFamily,
    IsmProtocol, IsmQuantity, IsmReading, IsmReport, IsmSettings, IsmStatus, RTL433_BAND_LABELS,
    RTL433_BANDS_DEFAULT, RTL433_BANDWIDTH_AUTO, RTL433_BANDWIDTH_MIN_HZ, RTL433_BANDWIDTHS,
    Rtl433Settings, Rtl433Status,
};
pub use js8::{
    HB_BAND_HI_HZ, HB_BAND_LO_HZ, HB_SLOT_HZ, Js8FrameInfo, Js8FrameKind, Js8Heard, Js8Msg,
    Js8Speed, Js8Status,
};
pub use limerfe::{
    LimeRfeConfig, RFE_ATTEN_MAX_STEPS, RFE_ATTEN_STEP_DB, RFE_BAUD, RFE_BUFFER_SIZE,
    RFE_I2C_ADDRESS, RfeChannel, RfeLink, RfeMode, RfeModeControl, RfePort, channel_for,
    resolve as rfe_resolve, rx_port_check, tx_port_check,
};
pub use memory::{BandStackEntry, MemoryChannel, MemoryFolder, MemorySort, RttyMemory};
pub use meters::{Meters, OVERLOAD_FRACTION, TxMeters, TxTelemetry};
pub use mode::{AgcMode, Mode, NrEngine, NrLevel, NrStrength, SlotTiming};
pub use netcfg::{
    ClusterConfig, Credentials, FeedConfig, FreeDvReporterConfig, LookupProvider, NetworkConfig,
    PskConfig, RbnConfig, WsprNetConfig,
};
pub use pictures::{
    IMAGE_NAME_MAX, IMAGE_PAGE_MAX, IMAGE_SLOT_THUMB_EDGE, IMAGE_SLOTS, IMAGE_SOURCE_MAX_EDGE,
    IMAGE_THUMB_EDGE, IMAGE_UPLOAD_MAX, ImageEntry, ImageKind, ImageListing, ImagePresets,
    ImageSlotInfo, received_at, safe_name,
};
pub use probe::{DeviceProbe, ProbeAnswer, ProbeTest, ReportKind, TestKind};
pub use prop_store::{PropSources, PropStore};
pub use propagation::{
    BandPlane, DEFAULT_HALFLIFE_S as PROP_DEFAULT_HALFLIFE_S, DEFAULT_HM_KM, GRID_CELLS,
    GRID_H as PROP_GRID_H, GRID_W as PROP_GRID_W, MAX_HOP_KM, MAX_HOPS, MIN_MUF_PATH_KM,
    MIN_MUF_PATHS, PropField, PropMuf, PropObservation, PropPath, PropSource, REF_TX_DBM,
    SPLAT_SIGMA_KM, cell_center, cell_of, fof2_floor_mhz, margin_db, muf3000_floor_mhz,
    obliquity_factor,
};
pub use publicsdr::{PublicSdrDirectory, PublicSdrEntry, PublicSdrNetwork};
pub use qo100::{QO100_BEACON_HZ, Qo100Settings, Qo100Status};
pub use radio::{
    AirspyConfig, AirspyDevice, AirspyGain, AirspyHfConfig, AirspyHfDevice, AirspyHfModel, Backend,
    BandDriveTrim, CAT_IQ_DC_BLOCK_MAX_HZ, CAT_IQ_RATES, CAT_SCOPE_MIN_BAUD,
    CONVERTER_OFFSET_MAX_HZ, CONVERTER_PRESETS, CatConfig, CatFamily, ConverterTx, CwKeying,
    DIV_FREEZE_ELEMENT, DIV_MODE_ELEMENT, DIV_RATE_ELEMENT, DIV_RESET_ELEMENT, DIV_TAPS_ELEMENT,
    DIVERSITY_MAX_TAPS, DigiMode, DiversityMode, ELAD_ATTENUATOR_DB, ELAD_CAT_BAUDS,
    ELAD_DEFAULT_CAT_BAUD, ELAD_DEFAULT_RATE_HZ, ELAD_SAMPLE_RATES, EladAntenna, EladConfig,
    EladDevice, EladTxInput, FREQ_RANGE_MAX_HZ, FobosConfig, FobosDevice, FobosPort, HackRfConfig,
    HackRfDevice, HpsdrConfig, HpsdrDevice, HpsdrFilterBoard, HpsdrIoRxInput, HpsdrOcPlan,
    HpsdrOcRow, HydraSdrConfig, HydraSdrDevice, HydraSdrGain, HydraSdrPort, IcomModel,
    IcomNetConfig, IcomRxSource, IcomScopeSpan, IfModeClass, KenwoodSend, KiwiConfig,
    LimeAuxConfig, LimeAuxRole, LimeConfig, LimeDevice, LineState, MAX_TRANSVERTERS, ModeControl,
    PANADAPTER_OFFSET_MAX_HZ, PanadapterAudio, PanadapterConfig, PanadapterTap, Parity, PlutoAgc,
    PlutoConfig, PlutoDevice, PlutoDuplex, PlutoPtt, PttMethod, QMX_IQ_OFFSET_HZ, QMX_IQ_RATE_HZ,
    RadioConfig, RtlSdrAgc, RtlSdrConfig, RtlSdrDevice, RtlSdrHfMode, RtlTcpConfig, Rx888Config,
    Rx888Device, RxSite, SdrPlayAgc, SdrPlayConfig, SdrPlayDevice, SdrPlayDuo, SdrPlayDuoRole,
    SdrPlayDuoTuner, SdrPlayHdrBw, SdrPlayModel, SerialConfig, SmartSdrConfig, SmartSdrDevice,
    SoapyConfig, SoapyDeviceInfo, SoundFormat, SpyServerConfig, SpyServerFormat, StopBits,
    TciConfig, Transverter, cat_iq_offset_max_hz, converter_preset_name, diversity_cost_note,
    elad_cat_baud, format_freq_ranges, hackrf_serial_matches, hpsdr_alex_oc, hpsdr_n2adr_oc,
    parse_freq_ranges,
};
pub use rds::{
    RdsClock, RdsData, RdsGroupLog, RdsStandard, RdsStats, RtPlus, af_code_hz, pi_callsign,
    pty_name, rt_plus_class,
};
pub use region::{Region, region, set_region};
pub use relay::{
    DEFAULT_HOLD_MS, DEFAULT_LEAD_MS, FailSafe, MAX_CHANNEL, RelayChannel, RelayConfig,
    RelayDevice, RelayFamily, RelayLink, RelayRole, RelayStatus, SenseConfig, SenseLine,
};
pub use repeater::{
    BURST_MS_RANGE, DCS_CODES, MAX_OFFSET_HZ, RepeaterState, Shift, TONE_BURST_HZ, ToneMode,
    TxSubTone, dcs_bits, standard_shift, standard_shift_in,
};
pub use rifp::{
    RIFP_CALLING_HZ, RIFP_MAP_MAX_CHUNKS, RifpEncoding, RifpMeta, RifpProfile, RifpSession,
    RifpSize, RifpStatus,
};
pub use rigctld::RigctldConfig;
pub use rotator::RotatorConfig;
pub use satcfg::{
    CELESTRAK_GROUPS, CelestrakGroup, CustomTle, OrbitRings, Passband, SatConfig, SatFreqs,
    SatLink, TleSubStatus, TleSubscription, fmt_mhz as fmt_sat_mhz, parse_tle_block,
};
pub use satlock::{
    C_KM_S, SatLockConfig, SatPass, SatTrackStatus, SatUplink, doppler_rx_hz, doppler_tx_hz,
};
pub use scanner::{SCAN_STEPS_HZ, ScanKind, ScanResume, ScanState, ScannerConfig};
pub use skimmer::{
    CW_SLOT_CHOICES, CW_SLOTS_DEFAULT, CwSkimmerDecoder, SkimmerKind, SkimmerSettings, SkimmerSpot,
};
pub use spectrum::{
    DEFAULT_DISPLAY_BINS, DEFAULT_ROWS_PER_SEC, MAX_DISPLAY_BINS, MAX_ROWS_PER_SEC, SpectrumConfig,
    SpectrumFrame,
};
pub use speech::{
    CallsignStyle, CategoryFlags, DecodeSpeech, FreqStyle, SpeechSettings, TextSpeech, TuneSpeech,
    Verbosity,
};
pub use spot::{Spot, SpotKind};
pub use sstv::{SstvMode, SstvStatus};
pub use state::{
    CESSB_MAX_DB, MAX_DECIMATION, MAX_MANUAL_GAIN_DB, MIN_DECIMATED_RATE_HZ, OffsetState,
    RadioState, RxId, RxState, SQUELCH_OPEN_DB, SWR_LIMIT_MAX, SWR_LIMIT_MIN, SWR_TUNE_LIMIT_SCALE,
    TxEqBand, TxEqState, TxState, Vfo, ZOOM_LANE_MARGIN, max_decimation, panadapter_fft_ceiling,
    swr_tune_limit, zoom_lane_decimation,
};
pub use station::StationConfig;
pub use tciserver::TciServerConfig;
pub use tone::{CTCSS_TONES, SubTone};
pub use ui::{
    BandplanKind, ChromeStyle, FontSize, LayoutMode, SmeterStyle, SpectrumDetail, Speed,
    UiSettings, UiTheme,
};
pub use vdl2::{
    VDL2_ALL_CHANNELS, VDL2_CHANNEL_LABELS, VDL2_CHANNEL_SPACING_HZ, VDL2_CHANNELS_HZ, VDL2_CSC_HZ,
    VDL2_DROP_LIST_S, VDL2_GOOD_SPS, VDL2_MESSAGE_MAX, VDL2_MESSAGES, VDL2_MIN_RATE_HZ,
    VDL2_PLAN_CENTER_HZ, VDL2_PLAN_RATE_HZ, VDL2_STATION_MAX, VDL2_STATIONS, VDL2_SYMBOL_RATE,
    VDL2_THRESHOLD_DB, Vdl2Acars, Vdl2AddrKind, Vdl2ChannelStatus, Vdl2Frame, Vdl2Message,
    Vdl2Payload, Vdl2Settings, Vdl2Station, Vdl2Status, Vdl2Xid,
};
pub use voice::{VOICE_MAX_LEN_S, VOICE_SLOTS, VoiceSlotInfo, VoiceStatus, slot_label};
pub use wefax::{WEFAX_STATIONS, WefaxChartMeta, WefaxIoc, WefaxLpm, WefaxStation, WefaxStatus};
pub use winlink::{
    DEFAULT_CMS_ADDRESS, MAIL_PAGE_MAX, MailAttachment, MailDraft, MailEntry, MailFolder,
    MailListing, MailMessage, WinlinkConfig, WinlinkGateway, WinlinkLane, WinlinkStatus,
};
pub use wsjtx::{N1mmConfig, WsjtxConfig};
pub use wspr::{
    BURST_S as WSPR_BURST_S, DEFAULT_TX_HZ as WSPR_DEFAULT_TX_HZ, POWERS_DBM as WSPR_POWERS_DBM,
    POWERS_W as WSPR_POWERS_W, SLOT_S as WSPR_SLOT_S, TX_OFFSET_S as WSPR_TX_OFFSET_S,
    WINDOW_HI_HZ as WSPR_WINDOW_HI_HZ, WINDOW_LO_HZ as WSPR_WINDOW_LO_HZ, WsprSpot, WsprStatus,
    dbm_to_mw, grid4 as wspr_grid4, power_dbm_for_watts, power_label, round_power_dbm,
};
