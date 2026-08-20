//! The main window's application state and its behaviour.
//!
//! [`SdroxideApp`] is one large struct — everything the window draws is a
//! function of it, and egui redraws all of it every frame — so splitting the
//! state up would only scatter it. What *is* split is the behaviour: this
//! module holds the state and its constructor, and each submodule holds the
//! `impl SdroxideApp` methods for one part of the window:
//!
//! - [`frame`] — the per-frame loop: drain engine events, lay the window out.
//! - [`top_bar`] — the control strip along the top (VFO, filter, TX, display).
//! - [`spectrum`] — panadapter/waterfall config and the overlays drawn on it.
//! - [`panels`] — the bottom panel, one submodule per operating mode.
//! - [`settings`] — the Settings dialog, one submodule per tab.
//! - [`spots`], [`logbook`], [`awards`], [`net`] — the network cockpit.
//! - [`windows`], [`solar`] — the remaining overlays and the 3D view's feed.
//! - [`persist`], [`util`] — on-disk state and small shared helpers.
//!
//! Submodules are descendants of this one, so they reach the private fields
//! below directly; what they expose *to each other* is marked
//! `pub(in crate::app)`, which is as wide as anything here ever gets.

pub(in crate::app) mod awards;
pub(in crate::app) mod bands;
pub(in crate::app) mod frame;
pub(in crate::app) mod ism;
pub(in crate::app) mod logbook;
pub(in crate::app) mod net;
pub(in crate::app) mod panels;
pub(crate) mod persist;
pub(in crate::app) mod rds;
pub(in crate::app) mod sat;
pub(in crate::app) mod scanner;
pub(in crate::app) mod settings;
pub(in crate::app) mod solar;
pub(in crate::app) mod spectrum;
pub(in crate::app) mod speech;
pub(in crate::app) mod spots;
pub(in crate::app) mod top_bar;
pub(in crate::app) mod util;
pub(in crate::app) mod windows;
pub(in crate::app) mod winlink;

use std::sync::{Arc, Mutex};

use eframe::egui;
use sdroxide_types::{
    AudioDevices, CallsignInfo, Decode, DeviceCaps, DigiStatus, MemoryChannel, MemoryFolder,
    Meters, Mode, NetworkConfig, QsoRecord, RadioController, RadioState, SkimmerSpot,
    SpectrumConfig, SpectrumFrame, Spot, UploadTarget,
};

use crate::view::ViewState;
use crate::waterfall_gpu;
use crate::widgets::spectrum_view;

use self::logbook::LogEditForm;
use self::panels::decodes::DecodeSort;
use self::panels::fsq::fsq_load_contacts;
use self::panels::rf_paint::RfPaintUi;
use self::panels::sstv::SstvUi;
use self::persist::{
    load_broadcast_stations, load_qso_log, load_speech_settings, load_ui_settings,
};
use self::settings::servers::TciServerStatus;
use self::settings::{SatEditState, SettingsTab, TestOutcome};

/// What a greyed-out transmit control says on hover.
///
/// Every operating panel has one, and a disabled button with no explanation
/// reads as a broken one — the reason it is dead is a property of the radio,
/// not of the panel, so it is worded once here rather than per mode.
const RX_ONLY_HINT: &str = "This radio can only receive — it has no transmitter to key.";

/// Attach [`RX_ONLY_HINT`] to a control greyed *because* this radio cannot
/// transmit.
///
/// Only then: several of these controls are also greyed for reasons of their
/// own — no station selected, nothing composed yet, an over already running —
/// and naming the receiver there would be the wrong answer to "why can't I
/// press this". `tx_ok` is [`SdroxideApp::tx_capable`] at the call site.
pub(in crate::app) fn rx_only_hint(resp: egui::Response, tx_ok: bool) -> egui::Response {
    if tx_ok { resp } else { resp.on_disabled_hover_text(RX_ONLY_HINT) }
}

/// Draw a control that puts this station on the air: as `add` draws it on a
/// transceiver, greyed out and explaining itself on a receiver.
///
/// For the controls whose only condition is that there *is* a transmitter. One
/// that is also greyed for a reason of its own keeps its own
/// `add_enabled_ui` and passes the response through [`rx_only_hint`], so the
/// two reasons do not end up wearing each other's explanation.
pub(in crate::app) fn tx_gated(
    ui: &mut egui::Ui,
    tx_ok: bool,
    add: impl FnOnce(&mut egui::Ui) -> egui::Response,
) -> egui::Response {
    rx_only_hint(ui.add_enabled_ui(tx_ok, add).inner, tx_ok)
}

pub struct SdroxideApp {
    ctrl: Box<dyn RadioController>,
    caps: Option<DeviceCaps>,
    state: RadioState,
    /// Latest spectrum frame, shared with the GPU waterfall callback — the Arc
    /// makes the per-repaint handoff a refcount bump instead of a bins clone.
    frame: Option<std::sync::Arc<SpectrumFrame>>,
    /// Latest full-band frame, from a direct-sampling front end that can see far
    /// more than the IQ it delivers. `None` on every other backend, which is
    /// also how the UI decides whether to offer the strip at all.
    wide_frame: Option<std::sync::Arc<SpectrumFrame>>,
    /// Scrolling history behind the full-band strip.
    wide_wf: crate::widgets::wide_spectrum::WideWaterfall,
    meters: Option<Meters>,
    memories: Vec<MemoryChannel>,
    mem_folders: Vec<MemoryFolder>,
    /// The scanner's settings, mirrored from the engine and edited in place —
    /// the whole struct goes back on any change, the way the skimmer's does.
    scanner: sdroxide_types::ScannerConfig,
    view: ViewState,
    peaks: spectrum_view::PeakHold,
    /// UI-side smoothing for the spectrum *line* (waterfall stays un-averaged).
    spec_smooth: spectrum_view::SpectrumSmooth,
    error: Option<String>,
    /// Persistent, non-fatal operator notice (e.g. radio audio input
    /// unavailable / mono card selected for IQ). Shown as a warning banner.
    radio_notice: Option<String>,
    sent_cfg: Option<SpectrumConfig>,
    desired_cfg: Option<SpectrumConfig>,
    desired_at: f64,
    /// The visible span last handed to the skimmers, the one waiting to be, and
    /// when it settled. Separate from `sent_cfg` because the spectrum viewport
    /// carries deliberate slack and this must not.
    sent_skim_view: Option<(f64, f64)>,
    desired_skim_view: Option<(f64, f64)>,
    desired_skim_view_at: f64,
    /// Auto-fit's timers and the view it last measured — what keeps the
    /// floor/ceiling fitted while the FIT chip is lit.
    fit: spectrum::AutoFit,
    /// The (mode, dial) the digital sub-band view was last fitted for. In the
    /// free-roaming digital modes (everything but FT8/FT4/JS8/WSPR) the fit is
    /// applied only when this changes, so zoom/pan survive between frames.
    digi_view_fit: Option<(Mode, f64)>,
    /// egui time of the last received spectrum frame, for stall detection.
    last_spectrum_at: f64,
    /// Waterfall time-scroll state: wall-clock (UTC secs) of the last tick and
    /// the carried fractional row, so the scroll rate is exact and independent
    /// of the frame rate (keeps the waterfall and time gridlines in lockstep).
    wf_last_now: f64,
    wf_row_accum: f32,
    /// Cached spectrum polylines (recomputed only when frame/view/rect change).
    trace_cache: spectrum_view::TraceCache,
    /// Switchable sound devices, queried once each time the settings dialog
    /// opens (cpal enumeration is too slow for per-frame).
    audio_devices: Option<AudioDevices>,
    audio_devices_queried: bool,
    /// Whether the build *the radio is attached to* can drive SoapySDR — that
    /// interface is offered only when it can. Answered by
    /// [`sdroxide_types::DeviceProbe::Host`], so on a remote client this is the
    /// server's answer and not this screen's.
    soapy_supported: bool,
    /// The sound cards on the radio's machine, for the CAT / Audio interface:
    /// (inputs, outputs). Distinct from `audio_devices`, which is *this*
    /// screen's speaker and microphone — the two are the same list only when
    /// the radio is on this machine. `None` until asked for.
    radio_audio_devices: Option<(Vec<String>, Vec<String>)>,
    /// Whether the far end answers device questions at all. False only after it
    /// has said so ([`sdroxide_types::ProbeAnswer::Unsupported`]), which greys
    /// out the controls that ask rather than leaving them to look broken.
    probes_answered: bool,
    /// Device questions asked of another machine and not yet answered, and when
    /// to stop waiting. The discovery controls are disabled meanwhile, so a
    /// press that has crossed the network is visibly in flight instead of
    /// looking like it did nothing.
    probe_waiting: u32,
    probe_wait_until: f64,
    /// Settings dialog: current tab, plus the radio-backend config + serial
    /// ports loaded once on open (edited live, persisted on change).
    ///
    /// The tab is deliberately session-only — reopening the dialog returns to
    /// wherever you last were, but a restart starts again at General, so it is
    /// never written to storage in [`eframe::App::save`].
    settings_tab: SettingsTab,
    /// Display preferences (frame rate, waterfall + spectrum speed), loaded from
    /// config at startup, edited in the UI tab, persisted on change.
    ui_settings: sdroxide_types::UiSettings,
    /// The theme and chrome styles last written into the egui context, so the
    /// top-of-frame check in `frame.rs` can re-apply the visuals the moment
    /// the settings dialog changes any of them — no restart.
    applied_look:
        (sdroxide_types::UiTheme, sdroxide_types::ChromeStyle, sdroxide_types::ChromeStyle),
    /// The interface font size last written into the context as its zoom
    /// factor, for the same reason — and so the zoom is left alone in between,
    /// where it belongs to egui's own ctrl+plus / ctrl+minus.
    applied_ui_font: sdroxide_types::FontSize,
    /// Spoken announcements: the state machine that decides what to say, and
    /// the synthesizer that says it. Always present — switched off, it holds a
    /// null sink and costs a comparison per event.
    speech: speech::SpeechRuntime,
    /// Voices found on disk, listed when the settings dialog opens.
    speech_voices: Vec<String>,
    radio_cfg: Option<sdroxide_types::RadioConfig>,
    /// The converter offset being typed on the Radio tab, in Hz. Held apart
    /// from `radio_cfg` because every other field on that tab is written to
    /// `radio.json` as it is typed, and the engine rereads that file whenever it
    /// reopens the radio — so a half-typed `125000000` would arrive as an offset
    /// of 12 Hz. This one lands only on Apply. `None` = not being edited.
    converter_edit_hz: Option<f64>,
    /// The RX and TX tuning ranges being typed on the Radio tab, as the
    /// operator's megahertz text. Buffered until Apply for the same reason as
    /// `converter_edit_hz`, and kept as text so a half-typed range is only ever
    /// a half-typed range and never a limit the radio is briefly held to.
    /// `None` = not being edited.
    range_edit: Option<(String, String)>,
    serial_ports: Vec<String>,
    /// HPSDR devices found by the last "Discover" scan in the settings dialog.
    hpsdr_devices: Vec<sdroxide_types::HpsdrDevice>,
    rtlsdr_devices: Vec<sdroxide_types::RtlSdrDevice>,
    rx888_devices: Vec<sdroxide_types::Rx888Device>,
    /// Airspy HF+ receivers found on the last Rescan.
    airspyhf_devices: Vec<sdroxide_types::AirspyHfDevice>,
    elad_devices: Vec<sdroxide_types::EladDevice>,
    lime_devices: Vec<sdroxide_types::LimeDevice>,
    airspy_devices: Vec<sdroxide_types::AirspyDevice>,
    hackrf_devices: Vec<sdroxide_types::HackRfDevice>,
    /// RSPs the SDRplay API service reported on the last Rescan.
    sdrplay_devices: Vec<sdroxide_types::SdrPlayDevice>,
    /// SoapySDR devices from the last enumeration (dialog-open on the SoapySDR
    /// interface, or Rescan). `None` = not enumerated yet, which is a different
    /// thing from "enumerated and found nothing".
    soapy_devices: Option<Vec<sdroxide_types::SoapyDeviceInfo>>,
    /// Result of the last TCI "Test connection" (Ok summary / Err message), or
    /// [`TestOutcome::Waiting`] while the machine with the radio on it is still
    /// answering.
    tci_test_result: Option<TestOutcome>,
    icomnet_test_result: Option<TestOutcome>,
    spyserver_test_result: Option<TestOutcome>,
    /// FlexRadios found by the last SmartSDR "Discover" listen.
    smartsdr_devices: Vec<sdroxide_types::SmartSdrDevice>,
    /// Result of the last SmartSDR "Test connection".
    smartsdr_test_result: Option<TestOutcome>,
    /// PlutoSDRs found by the last "Discover" (mDNS plus the gadget address).
    pluto_devices: Vec<sdroxide_types::PlutoDevice>,
    /// Result of the last PlutoSDR "Test connection".
    pluto_test_result: Option<TestOutcome>,
    seen_first_state: bool,
    show_memories: bool,
    /// The RDS window. Opened from the RDS chip in the receive menu, which only
    /// appears on WFM.
    show_rds: bool,
    /// Which half of the RDS window is showing.
    rds_tab: rds::RdsTab,
    /// The station picture from the engine's RDS decoder, minus the group log —
    /// that arrives as a delta and is accumulated into `rds_log`.
    rds: Option<sdroxide_types::RdsData>,
    /// The diagnostics log, oldest first, capped at [`rds::RDS_LOG_ROWS`].
    rds_log: std::collections::VecDeque<sdroxide_types::RdsGroupLog>,
    /// Absolute index the next group in `rds_log` should have, so a batch that
    /// was dropped between here and the engine shows as a gap rather than
    /// silently closing up.
    rds_next_seq: u64,
    show_scanner: bool,
    /// The ISM decoder window, opened from the ISM chip in the system row.
    show_ism: bool,
    /// Every 868 MHz device the engine has heard, newest first. A whole-table
    /// snapshot from the engine, so this is replaced rather than merged.
    ism_reports: Vec<sdroxide_types::IsmReport>,
    /// Where the decoder is listening and what its gates are seeing.
    ism_status: Option<sdroxide_types::IsmStatus>,
    /// How the device list is ordered, and which way round.
    ism_sort: ism::IsmSort,
    ism_sort_desc: bool,
    show_settings: bool,
    /// Voice keyer: the engine's slot list and what it is doing, the window's
    /// open state, and the one slot label being typed into (only the focused
    /// row is UI-owned, so the status echo can't fight the keyboard).
    voice: sdroxide_types::VoiceStatus,
    show_voice: bool,
    voice_name_edit: Option<(usize, String)>,
    /// When the band/mode, FFT and skimmer popups opened (egui time), for their
    /// auto-fade.
    mode_popup_since: Option<f64>,
    fft_popup_since: Option<f64>,
    skimmer_popup_since: Option<f64>,
    /// Fade clock for the SPEC popup's layer chips, like `skimmer_popup_since`.
    layers_popup_since: Option<f64>,
    /// Fade clock for the sub-audible tone popup, like `skimmer_popup_since`.
    tone_popup_since: Option<f64>,
    /// Fade clock for the noise-reduction picker, like `tone_popup_since`.
    nr_popup_since: Option<f64>,
    /// The layout in force last frame, so a change can re-apply the style
    /// metrics (chip padding, text sizes) exactly once instead of every frame.
    tier: crate::layout::Tier,
    /// Whether the compact layout's press-and-hold PTT is being held, so it
    /// sends one command per edge rather than one per frame.
    ptt_held: bool,
    mem_name: String,
    /// Name being typed into the memories window's "new folder" field.
    mem_folder_name: String,
    /// Folder rename in progress: (folder id, text as typed). UI-owned until
    /// the edit commits, exactly like `voice_name_edit`.
    mem_folder_edit: Option<(u32, String)>,
    /// One-shot: focus the rename field on the frame after REN was clicked.
    mem_folder_focus: bool,
    // Skimmer (CW etc.) spots, newest merge-by-id.
    skimmer_spots: Vec<SkimmerSpot>,
    /// Per-spot last-active timestamp (egui seconds), so a box fades out over
    /// `SKIMMER_FADE_SECS` once its signal stops keying instead of vanishing.
    skimmer_active_at: std::collections::HashMap<u64, f64>,
    // FT8/FT4 digital-mode state.
    digi_decodes: Vec<Decode>,
    digi_status: Option<DigiStatus>,
    /// PSK/RTTY outgoing text buffer (UI-owned; streamed to the engine, which
    /// reports back how many characters have been sent so we colour them green).
    text_tx: String,
    qso_log: Vec<QsoRecord>,
    /// Cached newest-first ordering and day grouping of [`Self::qso_log`], so
    /// the logbook list does not re-sort and re-group the whole log on every
    /// frame it is open. See `logbook::LogView`.
    log_view: crate::app::logbook::LogView,
    /// QSOs worked since this run started, which is what the FT8 panel's
    /// "Session" readout counts. Deliberately not derived from the logbook: the
    /// log is persisted and grows for ever, so counting it would report every
    /// contact ever made as if it had just been worked.
    session_qsos: usize,
    show_digi_settings: bool,
    /// UI-owned editable copy of the operator config, so typing isn't fought
    /// by the round-tripped status echo. Seeded once from the first status.
    digi_cfg_edit: sdroxide_types::DigiConfig,
    digi_cfg_seeded: bool,
    /// SSTV image-mode panel state (gallery, TX slots, message, textures).
    sstv: SstvUi,
    /// RF Paint (Spectrum Painting) panel state (text/image + previews).
    rf_paint: RfPaintUi,
    /// Hellschreiber receive raster (scrollback ring + texture).
    hell: crate::hell::HellUi,
    /// FSQ directed-message target callsign ("" = broadcast/ALLCALL).
    fsq_target: String,
    /// JS8: the `To:` callsign the composer addresses. Also what the globe
    /// draws the QSO arc to, JS8 having no QSO sequencer to ask instead.
    js8_target: String,
    /// Callsigns a locator has already been requested for this session,
    /// successful or not. Every lookup is an HTTP round trip on its own thread,
    /// and a busy band puts fifty stations in a heard list.
    ///
    /// Shared by every mode that places stations this way — JS8 and FSQ both
    /// feed the one `callsign_cache`, so asking twice for the same station
    /// would buy nothing.
    grid_looked_up: std::collections::HashSet<String>,
    /// Frame time of the last locator lookup, so they go out one at a time
    /// rather than fifty at once the moment a panel opens.
    grid_lookup_at: f64,
    /// JS8: the last message we transmitted. What `AGN?` — "say again" — is
    /// asking for, and the one reply the operator cannot retype from memory.
    js8_last_sent: String,
    /// FSQ contacts (address book), native-persisted in `contacts.json`.
    fsq_contacts: Vec<sdroxide_types::FsqContact>,
    /// FSQ "add contact" input field.
    fsq_new_contact: String,
    /// Whether the FSQ contacts editor window is open.
    fsq_show_contacts: bool,
    /// FSQ received-image gallery (decoded textures, newest first).
    fsq_rx_images: Vec<egui::TextureHandle>,
    /// Picked-image inbox for FSQ image transmit (raw file bytes).
    fsq_img_inbox: std::sync::Arc<std::sync::Mutex<Option<Vec<u8>>>>,
    /// The last decode the user clicked (not REPLY): its call and map
    /// location, shown as a faint preview marker distinct from the active DX.
    digi_preview: Option<(String, (f64, f64))>,
    /// Animated centre/zoom of the FT8 world map (eased toward the fit target).
    map_view: crate::widgets::worldmap::MapView,
    /// Which decoded stations are currently up, and how brightly. Shared with
    /// the 3D globe so the flat map and the globe never disagree.
    digi_stations: crate::digi_map::DigiStations,
    /// Every source's view of what is getting through, per band. Fed by the
    /// decode list, the WSPR receptions, the logbook and — when it is switched
    /// on — the Reverse Beacon Network; drawn by the flat map and the globe
    /// from this one copy so the two cannot disagree.
    prop: crate::prop_map::PropStore,
    /// N0NBH's published band conditions: a forecast, not a measurement.
    ///
    /// Fetched hourly in the background from the first frame, because the band
    /// menu shows them and the band menu is always there. `None` only until the
    /// first fetch lands, or when there is no cached copy and no network.
    band_conditions: Option<sdroxide_solar::BandConditions>,
    /// The in-flight fetch, if there is one.
    band_conditions_fetch:
        Option<std::sync::mpsc::Receiver<Option<sdroxide_solar::BandConditions>>>,
    /// Frame clock reading after which another fetch is due. The worker decides
    /// whether that turns into a request or a cache read; this only decides
    /// when to ask it.
    band_conditions_due: f64,
    /// Whether the Sun is up at the operator's own locator, recomputed a few
    /// times a minute. Which half of the published table to read.
    daylight: bool,
    /// When `daylight` was last worked out, so the ephemeris is not run on
    /// every frame for an answer that changes twice a day.
    daylight_at: f64,
    /// The BANDS window.
    show_bands: bool,
    /// The propagation field rendered to pixels, rebuilt only when it moves.
    prop_heat: crate::prop_map::PropHeat,
    /// WSPR receptions, newest first: what this station decoded, and — when the
    /// WSPRnet download is on — who decoded this station. Capped at
    /// [`crate::app::panels::wspr::WSPR_SPOT_ROWS`]; the propagation store keeps
    /// the long view, this is only what the panel lists.
    wspr_spots: Vec<sdroxide_types::WsprSpot>,
    /// Location of the decode row hovered this frame, shown on the map as a
    /// bright yellow dot. Frame-scoped (set by the decode list, read by the map).
    digi_hover_ll: Option<(f64, f64)>,
    /// Decode-list ordering within each turn, and whether to show CQ only.
    digi_sort: DecodeSort,
    /// Sort direction: `true` = descending (strongest / farthest first).
    digi_sort_desc: bool,
    digi_cq_only: bool,
    /// Decode-list filter: only stations that would put something new in the
    /// log (new entity, new band-slot, new grid, or a callsign never worked).
    digi_new_only: bool,
    /// The FT8 free-text entry, sent verbatim in the next transmit slot.
    digi_free_text: String,
    /// Country-flag textures, uploaded on first use and kept for the session.
    /// Lives on the app rather than in the decode list because the list is
    /// rebuilt from scratch every frame, and the JS8 heard list draws the same
    /// flags from the same entities.
    flags: crate::flags::Flags,
    /// Logbook overlay open state, and the in-progress new/edit entry (if any).
    show_logbook: bool,
    /// The Winlink mail window. Holds its own view state; the mailbox itself
    /// lives engine-side and is read a page at a time.
    pub(in crate::app) mail: winlink::MailUi,
    log_edit: Option<LogEditForm>,
    // ── Network cockpit (spots / lookup / uploads) ──
    /// Latest merged network spots (DX cluster / POTA / SOTA / PSK Reporter).
    spots: Vec<Spot>,
    /// Latest feed/connection status line (cluster state, feed errors).
    net_status: Option<String>,
    /// Spots window open state.
    show_spots: bool,
    /// Show only spots that fall inside the current panadapter view span.
    spot_in_view_only: bool,
    /// Fuzzy search query for the spot list. Narrows the list in the SPOTS
    /// window only — the waterfall labels are positioned by frequency, so
    /// reordering them by match quality would mean nothing.
    spot_search: String,
    /// The broadcast schedule in use: the cached season file plus the operator's
    /// own entries, or the compiled-in copy until a download lands.
    broadcast: Vec<sdroxide_types::BroadcastStation>,
    /// The subset of `broadcast` on air right now, as spots. Rebuilt when the
    /// UTC minute rolls over — the finest granularity a schedule changes at —
    /// rather than every frame.
    broadcast_spots: Vec<Spot>,
    /// The UTC minute `broadcast_spots` was built for.
    broadcast_minute: i64,
    /// An in-flight schedule download, if one is running.
    broadcast_fetch: Option<std::sync::mpsc::Receiver<persist::ScheduleFetch>>,
    /// What the last download did, for the settings panel. `None` while one is
    /// running or before any has been attempted.
    broadcast_fetch_status: Option<Result<String, String>>,
    /// The UTC day the season was last checked on. A season change is a calendar
    /// event, so checking once a day catches it without a timer.
    broadcast_checked_day: i64,
    /// UI-owned editable copy of the network config (edited in the Settings
    /// dialog's Spots / FreeDV / Uploads tabs). Carries no operator identity —
    /// that comes from the digi config, edited on the General tab.
    ///
    /// Seeded from `RadioEvent::StationConfig` — the engine's copy, wherever
    /// the engine is running — and left alone afterwards so typing sticks. The
    /// tabs that edit it stay disabled until it has arrived: applying an
    /// unseeded copy would write defaults over the station's real config.
    net_cfg_edit: NetworkConfig,
    net_cfg_seeded: bool,
    // ── Built-in TCI server ──
    /// UI-owned editable copy of the TCI server config, seeded from the engine
    /// like the network config above.
    tci_srv_edit: sdroxide_types::TciServerConfig,
    tci_srv_seeded: bool,
    /// Live server status (bound address, connected clients, bind error) from
    /// `RadioEvent::TciServerStatus`.
    tci_srv_status: Option<TciServerStatus>,
    // ── Built-in rigctld server ──
    /// UI-owned editable copy of the rigctld config, seeded from the engine.
    rigctld_edit: sdroxide_types::RigctldConfig,
    rigctld_seeded: bool,
    // ── WSJT-X UDP broadcast (decodes / status / QSOs for the loggers) ──
    /// UI-owned editable copy, seeded from the engine like the configs above.
    wsjtx_edit: sdroxide_types::WsjtxConfig,
    wsjtx_seeded: bool,
    /// Live status from `RadioEvent::RigctldStatus`. Same shape as the TCI
    /// server's, so the two share one status type.
    rigctld_status: Option<TciServerStatus>,
    /// Editable "extra cluster commands" (one per line), split into
    /// `net_cfg_edit.cluster.commands` on apply.
    net_cluster_cmds: String,
    /// The same for the RBN reader's post-login commands, which is where an
    /// operator narrows the skimmer firehose to their own continent.
    net_rbn_cmds: String,
    /// Rolling upload/lookup result log for the spots window (newest first).
    net_log: Vec<String>,
    /// Latest credential-check result per logging service, and which checks are
    /// still in flight. Not persisted: an answer from an hour ago says nothing
    /// about the password typed since, and a stale green tick is worse than no
    /// tick at all.
    login_tests:
        std::collections::HashMap<sdroxide_types::LoginTarget, sdroxide_types::LoginTestResult>,
    login_tests_pending: std::collections::HashSet<sdroxide_types::LoginTarget>,
    /// Inbox for an ADIF file chosen via the native "Import" dialog (a picker
    /// thread writes; the UI drains it each frame).
    adif_import_inbox: Arc<Mutex<Option<String>>>,
    /// Callsigns queued for lookup, drained into commands each frame.
    pending_lookups: Vec<String>,
    /// Everything callsign lookup has resolved this session, by callsign. Kept
    /// because a JS8 station's locator usually never arrives on the air —
    /// only heartbeats and CQs carry one — so the map has nothing else to
    /// place the rest of the conversation by.
    callsign_cache: std::collections::HashMap<String, CallsignInfo>,
    /// QSO uploads queued (id, single-record ADIF, targets), drained to commands.
    pending_uploads: Vec<(u64, String, Vec<UploadTarget>)>,
    /// Awards dashboard open state + band filter ("" = all bands).
    show_awards: bool,
    awards_band: String,
    /// Cached award tally, keyed by (log length, band filter).
    awards_cache: Option<(usize, String, sdroxide_types::Awards)>,
    /// The same tally placed on the globe for the 3D view's award layer, keyed
    /// the same way. Shared rather than copied: it is three hundred entities
    /// and the window republishes it every frame.
    awards_heat: Option<(usize, String, Arc<Vec<sdroxide_types::EntitySlot>>)>,
    /// Cached set of worked DXCC entity names, keyed by log length (for the
    /// "new entity" spot badge).
    worked_entities_cache: Option<(usize, std::collections::HashSet<String>)>,
    /// Cached membership sets over the log, keyed by log length — the decode
    /// list asks these which stations would be a new one, every row, every slot.
    log_index_cache: Option<(usize, sdroxide_types::LogIndex)>,
    /// F1 help: the embedded user manual with a navigation outline.
    help: crate::help::Help,
    /// Control inputs: keyboard/mouse bindings, MIDI, and what is held right now.
    input: crate::input::InputRuntime,
    /// MIDI ports as `(id, name)`, enumerated when the settings dialog opens
    /// (touching the host MIDI stack is too slow for per-frame).
    midi_in_ports: Vec<(String, String)>,
    midi_out_ports: Vec<(String, String)>,
    /// Solar-system 3D view, shown in its own OS window (native-only).
    #[cfg(not(target_arch = "wasm32"))]
    solar: crate::solar3d::Solar3d,
    /// The operator's satellite additions: element sets they pasted in or
    /// subscribed to, and their frequency corrections. Shared by `Arc` because
    /// the solar window's render closure takes a handle it outlives any borrow
    /// of; replaced wholesale on every edit rather than mutated in place.
    ///
    /// Seeded from the engine like the network config: the subscribed listings
    /// are fetched — and cached — on the machine the engine runs on, which is
    /// the only one a browser client can reach.
    sat_cfg: std::sync::Arc<sdroxide_types::SatConfig>,
    /// The settings dialog's working copy, its transient state, and what each
    /// subscription's last fetch did.
    sat_cfg_edit: sdroxide_types::SatConfig,
    sat_cfg_seeded: bool,
    sat_ui: SatEditState,
    sat_sub_status: Vec<sdroxide_types::TleSubStatus>,
    /// The engine's satellite lock, mirrored from [`RadioEvent::SatTrack`].
    /// `Some` while a lock is active — the SAT window's live pane, the top-bar
    /// accent and the 3D view's highlight all read this one field.
    sat_track: Option<sdroxide_types::SatTrackStatus>,
    /// The SAT window and everything it remembers between frames.
    show_sat: bool,
    sat_win: sat::SatWinState,
    /// The rotctld client's health, mirrored from
    /// [`RadioEvent::RotatorStatus`].
    rotator_status: Option<(bool, f64, f64, Option<String>)>,
    /// The rotator settings dialog's working copy, seeded once like the
    /// server configs.
    rot_cfg_edit: sdroxide_types::RotatorConfig,
    rot_cfg_seeded: bool,
    /// The station's IARU region, as the General tab's dropdown last showed it.
    ///
    /// Unlike the config buffers around it this is not seeded-once-then-owned
    /// by the dialog: it is overwritten by every `StationConfig` announcement,
    /// because the band plan the whole client draws with has to be the
    /// station's and not a stale local guess. The dropdown writes through it to
    /// [`Command::SetRegion`].
    region_edit: sdroxide_types::Region,
    /// Weather fax: the chart being painted and the gallery of saved ones.
    wefax: crate::wefax::WefaxUi,
    /// Whether the operator has dismissed the out-of-band transmit warning
    /// this session. Never persisted: `--oob-tx` has to be passed again on the
    /// next launch, so the warning has to be acknowledged again too.
    oob_tx_ack: bool,
    /// The sign-in screen a server that asks for a password puts up, in place
    /// of everything above.
    login: crate::login::LoginForm,
    /// Who may connect to *this* machine's server, edited on the General tab.
    ///
    /// Only shown — and only meaningful — when the engine is in this process:
    /// these are `config.toml` on the machine the radio is attached to, so a
    /// remote client has nothing here to read them from and no business
    /// writing them. Left at the default and never persisted in the browser.
    remote_access: sdroxide_types::RemoteAccess,
    /// Which station the **Remote** tab dials — the address as last entered,
    /// read from this machine's `config.toml` and written back as it is typed.
    /// Belongs to the screen, not to the station, so unlike `remote_access` it
    /// is editable from every native client.
    #[cfg(not(target_arch = "wasm32"))]
    remote_server: sdroxide_types::RemoteServer,
    /// What the last CONNECT from the Remote tab did — "connecting…" while the
    /// shell has it, then whatever came back. Shown on that tab, because a
    /// connection that failed to open leaves no tab of its own to say so.
    #[cfg(not(target_arch = "wasm32"))]
    remote_status: Option<Result<String, String>>,
    // ── Multi-radio (see `crate::multi::MultiApp`) ──
    /// Whether this instance is the focused tab. Always true in a single-radio
    /// session. Gates the announcer and the window title: a background radio
    /// keeps decoding, but it stays quiet and leaves the title alone.
    focused: bool,
    /// Whether this instance writes the station-wide storage keys (UI
    /// settings, logbook backup, input bindings) in `save`. Exactly one tab
    /// does, or the last tab to save would overwrite the others'.
    station_writer: bool,
    /// This instance's per-radio storage key for the view state: `"view"` for
    /// radio 0 — so a single-radio install keeps its saved layout — and
    /// `"view:<id>"` for every other radio.
    view_key: String,
    /// Multi-radio: another tab may append to the logbook file between this
    /// copy's reads, so re-read it before appending. False when this is the
    /// only instance, where the copy in memory is always current.
    shared_log: bool,
    /// This radio's id — keys this tab's GPU waterfall history apart from the
    /// other tabs'.
    radio_id: u32,
    /// The station's radio roster as the shell last published it, drawn at the
    /// top of Settings → Radio. Empty outside a multi-radio session (remote
    /// clients, the browser), which is also what hides the area there.
    radio_roster: Vec<RadioChip>,
    /// Whether the shell can build a radio of this machine's own — false in a
    /// client that only ever drives somebody else's station (`--connect`),
    /// where the roster's "+" would open nothing. Connecting to a *further*
    /// server is a different matter and stays available there.
    can_add_radio: bool,
    /// Radio-management actions the settings dialog asked for this frame. The
    /// dialog lives inside one tab and cannot reach the shell that owns the
    /// tab set, so requests are returned as values and drained by
    /// [`crate::multi::MultiApp`] after the frame — the same arrangement the
    /// solar window's LOCK button uses.
    radio_tab_requests: Vec<RadioTabRequest>,
    /// The radio rename in progress on the settings page: (radio id, text as
    /// typed). UI-owned until the edit commits — the roster republishes every
    /// frame, and a field bound straight to it would fight the keyboard —
    /// exactly like `voice_name_edit` and `mem_folder_edit`.
    radio_name_edit: Option<(u32, String)>,
}

/// One radio in the roster the shell publishes to the focused tab — what a
/// tab chip shows, wherever it is drawn.
#[derive(Clone)]
pub(crate) struct RadioChip {
    /// The *tab's* id, which is what a [`RadioTabRequest`] names. Not what the
    /// station this radio belongs to calls it — see [`Self::station_id`].
    pub id: u32,
    /// Which station this radio is on: empty for this machine's own, and the
    /// address for one reached over the network
    /// ([`SdroxideApp::station_key`]).
    pub station: String,
    /// The id that station knows this radio by
    /// ([`SdroxideApp::station_radio_id`]). What goes into a configuration
    /// stored on that station — a panadapter pairing names the receiver this
    /// way, because the file it is written to lives there.
    pub station_id: u32,
    /// The operator's own name, exactly as typed — empty until they rename
    /// the radio. What the rename field edits.
    pub name: String,
    /// The name this radio carries while `name` is empty: its interface type
    /// ("TCI", "RTL-SDR", …), or a neutral "Radio N" while it has none. What
    /// the chips display by default, and the rename field's hint.
    pub default_name: String,
    pub tx_on: bool,
    pub error: bool,
    pub muted: bool,
    pub focused: bool,
    /// The radio that has borrowed this one as its panadapter receiver, when
    /// one has, by *tab* id — this is for naming it on screen and for asking
    /// the shell to reopen it, both of which are tab-side. Such a radio is off
    /// the main window's strip (its front end belongs to the borrower) but
    /// stays in this roster, because this is where its interface is configured
    /// and where the pairing is undone.
    pub attached_to: Option<u32>,
    /// Whether this radio is switched on. Off means its interface is not open
    /// — no device claimed, no rig dialled — while its tab, its engine and its
    /// whole configuration stay exactly where they are.
    pub enabled: bool,
    /// Whether *this* station is where that switch lives. False for a radio at
    /// the far end of a connection: it is switched on and off where it is, and
    /// this machine's roster has no entry for it to record.
    pub switchable: bool,
}

impl RadioChip {
    /// What a tab chip shows: the operator's name, else the derived default.
    pub fn display_name(&self) -> &str {
        if self.name.is_empty() { &self.default_name } else { &self.name }
    }
}

/// What to call radio `id` given the roster, falling back to its number for a
/// radio that is not in it (a roster still filling in, a hand-edited file).
pub(crate) fn radio_name(roster: &[RadioChip], id: u32) -> String {
    roster
        .iter()
        .find(|c| c.id == id)
        .map_or_else(|| format!("radio {}", id + 1), |c| c.display_name().to_string())
}

/// A radio-management action requested from inside a tab (the settings
/// dialog's copy of the strip), acted on by the shell.
pub(crate) enum RadioTabRequest {
    /// Switch to this radio, carrying the settings dialog along: the origin's
    /// closes, the target's opens on its Radio page.
    Focus(u32),
    Add,
    /// Open somebody else's station as a tab of its own: dial this WebSocket
    /// URL and, if it answers, show it under `name`. Queued by the **Remote**
    /// tab's CONNECT button — the connection is made by the shell, because
    /// only it can add a tab to hold it.
    ///
    /// Native only, like the Remote tab that queues it: the address it dials
    /// is remembered in this machine's `config.toml`, which a browser has no
    /// equivalent of. A browser client reaches the radios of the station that
    /// served it, which arrive by themselves.
    #[cfg(not(target_arch = "wasm32"))]
    Connect {
        url: String,
        name: String,
    },
    Close(u32),
    Mute {
        id: u32,
        muted: bool,
    },
    /// Record the operator's name for this radio; empty goes back to the
    /// interface-derived default.
    Rename {
        id: u32,
        name: String,
    },
    /// Switch this radio on or off — open its interface, or let it go while
    /// keeping everything it is configured as.
    Power {
        id: u32,
        on: bool,
    },
    /// Rebuild another radio's front end. Queued when a radio that has been
    /// lent out as a panadapter receiver is reconfigured: the device is open on
    /// the *borrower's* engine, so Apply on the lender's own page has to reach
    /// the borrower or nothing it changed takes effect.
    Reopen(u32),
}

impl SdroxideApp {
    pub fn new(cc: &eframe::CreationContext<'_>, ctrl: Box<dyn RadioController>) -> Self {
        Self::new_tab(&cc.egui_ctx, cc.storage, cc.wgpu_render_state.clone(), ctrl, 0, true)
    }

    /// One radio tab of a multi-radio session ([`crate::multi::MultiApp`]).
    /// Takes the creation context's pieces rather than the context itself,
    /// because a tab added from the "+" chip is built long after startup, when
    /// no [`eframe::CreationContext`] exists any more.
    pub fn new_tab(
        egui_ctx: &egui::Context,
        storage: Option<&dyn eframe::Storage>,
        wgpu_render_state: Option<eframe::egui_wgpu::RenderState>,
        ctrl: Box<dyn RadioController>,
        radio_id: u32,
        station_writer: bool,
    ) -> Self {
        // The look and the font sizes must be selected before `theme::apply`
        // reads them, or the first frame flashes the default theme at the
        // default scale.
        let ui_settings = load_ui_settings(storage);
        crate::theme::set_look(
            ui_settings.theme,
            ui_settings.button_style,
            ui_settings.window_style,
        );
        crate::theme::set_font_sizes(
            ui_settings.skimmer_font_size,
            ui_settings.waterfall_font_size,
            ui_settings.menu_font_size,
        );
        crate::theme::apply(egui_ctx);
        if let Some(rs) = &wgpu_render_state {
            waterfall_gpu::init(rs);
        }
        // Radio 0 keeps the historical key, so an existing installation keeps
        // its saved layout.
        let view_key = if radio_id == 0 { "view".to_string() } else { format!("view:{radio_id}") };
        let mut view: ViewState =
            storage.and_then(|s| eframe::get_value(s, &view_key)).unwrap_or_default();
        view.migrate();
        // Copied out before `view` is moved into the struct below.
        #[cfg(not(target_arch = "wasm32"))]
        let solar3d_view = view.solar3d;
        #[cfg(target_arch = "wasm32")]
        let _ = &wgpu_render_state;
        SdroxideApp {
            ctrl,
            caps: None,
            state: RadioState::default(),
            frame: None,
            wide_frame: None,
            wide_wf: Default::default(),
            meters: None,
            memories: Vec::new(),
            mem_folders: Vec::new(),
            scanner: sdroxide_types::ScannerConfig::default(),
            view,
            peaks: spectrum_view::PeakHold::default(),
            spec_smooth: spectrum_view::SpectrumSmooth::default(),
            error: None,
            radio_notice: None,
            sent_cfg: None,
            desired_cfg: None,
            desired_at: 0.0,
            sent_skim_view: None,
            desired_skim_view: None,
            desired_skim_view_at: 0.0,
            fit: Default::default(),
            digi_view_fit: None,
            last_spectrum_at: 0.0,
            wf_last_now: 0.0,
            wf_row_accum: 0.0,
            trace_cache: spectrum_view::TraceCache::default(),
            audio_devices: None,
            audio_devices_queried: false,
            // Asked for when the settings dialog opens, along with everything
            // else about the machine the radio is on — see
            // `SdroxideApp::ask_device`. Until then the interface list simply
            // does not offer SoapySDR, which is the safe way round: the list is
            // only ever read inside that dialog.
            soapy_supported: false,
            radio_audio_devices: None,
            // Believed until the far end says otherwise: the answer costs a
            // round trip, and greying the controls out until one arrives would
            // make every session start by looking like the old, local-only one.
            probes_answered: true,
            probe_waiting: 0,
            probe_wait_until: 0.0,
            settings_tab: SettingsTab::General,
            ui_settings,
            applied_look: (ui_settings.theme, ui_settings.button_style, ui_settings.window_style),
            applied_ui_font: ui_settings.menu_font_size,
            speech: speech::SpeechRuntime::new(load_speech_settings(storage)),
            speech_voices: Vec::new(),
            radio_cfg: None,
            converter_edit_hz: None,
            range_edit: None,
            serial_ports: Vec::new(),
            hpsdr_devices: Vec::new(),
            rtlsdr_devices: Vec::new(),
            rx888_devices: Vec::new(),
            airspyhf_devices: Vec::new(),
            elad_devices: Vec::new(),
            lime_devices: Vec::new(),
            airspy_devices: Vec::new(),
            hackrf_devices: Vec::new(),
            sdrplay_devices: Vec::new(),
            soapy_devices: None,
            tci_test_result: None,
            icomnet_test_result: None,
            spyserver_test_result: None,
            smartsdr_devices: Vec::new(),
            smartsdr_test_result: None,
            pluto_devices: Vec::new(),
            pluto_test_result: None,
            seen_first_state: false,
            show_memories: false,
            show_rds: false,
            rds_tab: rds::RdsTab::default(),
            rds: None,
            rds_log: std::collections::VecDeque::new(),
            rds_next_seq: 0,
            show_scanner: false,
            show_ism: false,
            ism_reports: Vec::new(),
            ism_status: None,
            ism_sort: ism::IsmSort::default(),
            ism_sort_desc: true,
            show_settings: false,
            voice: sdroxide_types::VoiceStatus::default(),
            show_voice: false,
            voice_name_edit: None,
            mode_popup_since: None,
            fft_popup_since: None,
            skimmer_popup_since: None,
            layers_popup_since: None,
            tone_popup_since: None,
            nr_popup_since: None,
            // Corrected on the first frame, once the viewport size is known.
            tier: crate::layout::Tier::Desktop,
            ptt_held: false,
            mem_name: String::new(),
            mem_folder_name: String::new(),
            mem_folder_edit: None,
            mem_folder_focus: false,
            skimmer_spots: Vec::new(),
            skimmer_active_at: std::collections::HashMap::new(),
            digi_decodes: Vec::new(),
            digi_status: None,
            text_tx: String::new(),
            qso_log: load_qso_log(storage),
            log_view: Default::default(),
            session_qsos: 0,
            show_digi_settings: false,
            digi_cfg_edit: sdroxide_types::DigiConfig::default(),
            sstv: SstvUi::default(),
            rf_paint: RfPaintUi::default(),
            hell: Default::default(),
            fsq_target: String::new(),
            js8_target: String::new(),
            grid_looked_up: Default::default(),
            grid_lookup_at: 0.0,
            js8_last_sent: String::new(),
            fsq_contacts: fsq_load_contacts(),
            fsq_new_contact: String::new(),
            fsq_show_contacts: false,
            fsq_rx_images: Vec::new(),
            fsq_img_inbox: std::sync::Arc::new(std::sync::Mutex::new(None)),
            digi_cfg_seeded: false,
            digi_preview: None,
            map_view: Default::default(),
            band_conditions: None,
            band_conditions_fetch: None,
            band_conditions_due: 0.0,
            daylight: true,
            daylight_at: f64::NEG_INFINITY,
            show_bands: false,
            digi_stations: Default::default(),
            prop: Default::default(),
            prop_heat: Default::default(),
            wspr_spots: Vec::new(),
            digi_hover_ll: None,
            digi_sort: DecodeSort::None,
            digi_sort_desc: true,
            digi_cq_only: false,
            digi_new_only: false,
            digi_free_text: String::new(),
            flags: Default::default(),
            show_logbook: false,
            mail: winlink::MailUi::default(),
            log_edit: None,
            spots: Vec::new(),
            net_status: None,
            show_spots: false,
            spot_in_view_only: false,
            spot_search: String::new(),
            broadcast: load_broadcast_stations(),
            broadcast_spots: Vec::new(),
            broadcast_minute: -1,
            // Kicked off at startup: the first run has no cached schedule, and
            // after that this only fires again when the season turns over.
            // One tab downloads for the whole station; the others read the
            // cache it fills.
            broadcast_fetch: if station_writer {
                persist::spawn_schedule_fetch(false)
            } else {
                None
            },
            broadcast_fetch_status: None,
            broadcast_checked_day: -1,
            net_cfg_edit: NetworkConfig::default(),
            net_cfg_seeded: false,
            rigctld_edit: sdroxide_types::RigctldConfig::default(),
            rigctld_seeded: false,
            wsjtx_edit: sdroxide_types::WsjtxConfig::default(),
            wsjtx_seeded: false,
            rigctld_status: None,
            tci_srv_edit: sdroxide_types::TciServerConfig::default(),
            tci_srv_seeded: false,
            tci_srv_status: None,
            net_cluster_cmds: String::new(),
            net_rbn_cmds: String::new(),
            net_log: Vec::new(),
            login_tests: std::collections::HashMap::new(),
            login_tests_pending: std::collections::HashSet::new(),
            adif_import_inbox: Arc::new(Mutex::new(None)),
            pending_lookups: Vec::new(),
            callsign_cache: Default::default(),
            pending_uploads: Vec::new(),
            show_awards: false,
            awards_band: String::new(),
            awards_cache: None,
            awards_heat: None,
            worked_entities_cache: None,
            log_index_cache: None,
            help: crate::help::Help::default(),
            input: crate::input::InputRuntime::new(storage, egui_ctx),
            midi_in_ports: Vec::new(),
            midi_out_ports: Vec::new(),
            // The GPU resources are built on first open, not here: most
            // sessions never open this window.
            #[cfg(not(target_arch = "wasm32"))]
            solar: crate::solar3d::Solar3d::new(wgpu_render_state, solar3d_view),
            sat_cfg: Default::default(),
            sat_cfg_edit: Default::default(),
            sat_cfg_seeded: false,
            sat_track: None,
            show_sat: false,
            sat_win: Default::default(),
            rotator_status: None,
            rot_cfg_edit: Default::default(),
            rot_cfg_seeded: false,
            // Whatever this process is already on: the binary applies the
            // station's setting before the app is built, and a remote client
            // starts on the default until the station says otherwise.
            region_edit: sdroxide_types::region(),
            sat_ui: Default::default(),
            sat_sub_status: Vec::new(),
            wefax: Default::default(),
            oob_tx_ack: false,
            login: Default::default(),
            remote_access: persist::load_remote_access(),
            #[cfg(not(target_arch = "wasm32"))]
            remote_server: persist::load_remote_server(),
            #[cfg(not(target_arch = "wasm32"))]
            remote_status: None,
            focused: true,
            station_writer,
            view_key,
            shared_log: false,
            radio_id,
            radio_roster: Vec::new(),
            // Set by the shell, which is the only thing that knows whether it
            // was given a factory.
            can_add_radio: false,
            radio_tab_requests: Vec::new(),
            radio_name_edit: None,
        }
    }

    /// Set the focus flag without side effects — used while a multi-radio
    /// session is being assembled, before there is a first frame.
    pub(crate) fn set_focused_flag(&mut self, focused: bool) {
        self.focused = focused;
    }

    /// Focus (or unfocus) this tab at runtime. Gaining focus reseeds the
    /// announcer — the backlog of state changes that happened in the
    /// background is where the radio *is*, not news to recite — and restores
    /// the window title this tab owns while it is up front.
    pub(crate) fn set_focused(&mut self, focused: bool, ctx: &egui::Context) {
        if self.focused == focused {
            return;
        }
        self.focused = focused;
        if focused {
            self.speech.announcer.reseed();
            if let Some(c) = &self.caps {
                ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
                    "sdroxide — {}",
                    c.label
                )));
            }
        } else {
            // Losing the keyboard with a bound control held must release it:
            // this radio stops polling the keyboard now (a visible pane still
            // draws, a hidden tab doesn't even do that), so the key-up would
            // never be seen. A stranded PTT is the failure this guards
            // against.
            let mut cmds = Vec::new();
            self.release_held_controls(&mut cmds);
            for c in cmds {
                self.ctrl.send(c);
            }
        }
    }

    /// Multi-radio: mark the logbook file as shared with other tabs.
    pub(crate) fn set_shared_log(&mut self, shared: bool) {
        self.shared_log = shared;
    }

    /// Whether this radio is on the air — the tab strip's TX badge.
    pub(crate) fn tab_tx_on(&self) -> bool {
        self.state.tx.ptt || self.state.tx.tune
    }

    /// Whether this radio's engine reported a lost connection — the tab
    /// strip's warning badge.
    pub(crate) fn tab_error(&self) -> bool {
        self.error.is_some()
    }

    /// The tab strip's mute toggle — silences this radio's speaker path in the
    /// engine; everything else keeps running.
    pub(crate) fn mute_tab(&mut self, muted: bool) {
        self.ctrl.set_muted(muted);
    }

    /// The radio that has borrowed *this* one's receiver as its panadapter, if
    /// one has — named as the station numbers it, like everything in
    /// [`DeviceCaps`].
    ///
    /// From the engine's capabilities, so it says what is actually open rather
    /// than what has been typed into a dialog and not yet applied. The shell
    /// keeps such a radio off the tab strip: its front end belongs to the
    /// borrower, so it has nothing of its own to show.
    pub(crate) fn lent_to_radio(&self) -> Option<u32> {
        self.caps.as_ref()?.lent_to
    }

    /// Rebuild this radio's front end from its persisted configuration — the
    /// same thing Apply / reconnect does, asked for from outside the tab.
    /// Used when the radio this one borrowed as a panadapter receiver has been
    /// reconfigured on its own page.
    pub(crate) fn reopen_source(&mut self) {
        self.ctrl.reopen_source();
    }

    /// The radio *this* one is configured to borrow as its panadapter receiver,
    /// as its own station numbers it.
    ///
    /// From the configuration rather than the capabilities ([`Self::lent_to`]
    /// and [`Self::lent_to_radio`] read those): the question this answers is
    /// which radio's device this one will claim when it next opens, which is
    /// exactly the radio that has to be told to let go of it.
    pub(crate) fn pan_source_radio(&self) -> Option<u32> {
        self.ctrl.radio_config()?.panadapter.source_radio
    }

    /// The radio that has borrowed *this* one as its panadapter receiver, if
    /// one has. Read from the roster the shell publishes — a radio cannot see
    /// its own borrower any other way, the pairing being recorded on the
    /// borrower's side.
    pub(crate) fn lent_to(&self) -> Option<u32> {
        self.radio_roster.iter().find(|c| c.id == self.radio_id)?.attached_to
    }

    /// The other radios of the station this tab is connected to, and the
    /// address this tab itself is on — what the shell needs to put the rest of
    /// a station's radios in tabs beside it. Both empty for a local radio.
    pub(crate) fn peer_radios(&self) -> Vec<sdroxide_types::PeerRadio> {
        self.ctrl.peer_radios()
    }

    pub(crate) fn peer_url(&self) -> Option<String> {
        self.ctrl.peer_url()
    }

    pub(crate) fn peer_name(&self) -> Option<String> {
        self.ctrl.peer_name()
    }

    /// This connection's station, as the sign-in keys it: every radio of one
    /// station shares a door.
    pub(crate) fn station_key(&self) -> String {
        self.ctrl.peer_url().map(|u| crate::login::station_key(&u)).unwrap_or_default()
    }

    /// Which radio this tab is, *as the station it belongs to numbers it*: the
    /// roster id for a radio on this machine, and the id in the address for one
    /// reached over the network.
    ///
    /// Not the tab's own id, which is this screen's bookkeeping — a connection
    /// is numbered from `REMOTE_TAB_ID_BASE` precisely so it cannot collide
    /// with the local roster. Anything comparing radios *within* a station —
    /// which of them has lent its receiver to which — has to speak the
    /// station's numbering, and pair this with [`Self::station_key`] so two
    /// stations' radio 2 are not taken for one.
    pub(crate) fn station_radio_id(&self) -> u32 {
        let Some(url) = self.ctrl.peer_url() else { return self.radio_id };
        // `…/ws` is the station's first radio; `…/ws/<n>` names one.
        url.trim_end_matches('/')
            .rsplit_once("/ws/")
            .and_then(|(_, id)| id.parse().ok())
            .unwrap_or(0)
    }

    /// Answer a sign-in challenge from what the operator has already given
    /// this station — or from what this device has kept — without drawing
    /// anything.
    ///
    /// For the tabs that are not on screen. They never run a frame, so their
    /// sign-in screen never runs either: a station's second radio would sit at
    /// a challenge nobody can see, staying unconnected until it was clicked,
    /// even though the operator signed in to that very station a moment ago.
    pub(crate) fn poll_auth(&mut self) {
        let phase = self.ctrl.auth_phase();
        self.login.settle(&phase, &self.station_key());
        if !phase.is_pending() {
            return;
        }
        if let Some(a) = self.login.answer_without_asking(&phase) {
            self.ctrl.send_auth(a.username, a.password);
        }
    }

    /// Detach from this radio: disconnect the engine, drop the audio streams,
    /// join the DSP thread. Tab close and app exit.
    pub(crate) fn shutdown_ctrl(&mut self) {
        self.ctrl.shutdown();
    }

    /// Open the Settings dialog on the Radio tab — where a freshly added
    /// radio, which has no interface yet, is configured.
    pub(crate) fn open_radio_settings(&mut self) {
        self.show_settings = true;
        self.settings_tab = SettingsTab::Radio;
    }

    /// Close the Settings dialog — the shell moves it to another tab when the
    /// operator switches radios from inside it.
    pub(crate) fn close_settings(&mut self) {
        self.show_settings = false;
    }

    /// The shell's per-frame publication of the radio roster (see
    /// [`SdroxideApp::radio_roster`]).
    pub(crate) fn set_radio_roster(&mut self, roster: Vec<RadioChip>) {
        self.radio_roster = roster;
    }

    /// Whether the shell can open a radio attached to this machine (see
    /// [`SdroxideApp::can_add_radio`]). Set once, when the session is
    /// assembled.
    pub(crate) fn set_can_add_radio(&mut self, can: bool) {
        self.can_add_radio = can;
    }

    /// How the connection the Remote tab asked for went (see
    /// [`SdroxideApp::remote_status`]). Success is reported on the tab as well
    /// as by the new radio appearing, because the operator may well be looking
    /// at the dialog rather than at the strip behind it.
    ///
    /// Native only, like the Remote tab itself.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn set_remote_status(&mut self, status: Result<String, String>) {
        self.remote_status = Some(status);
    }

    /// Drain the radio-management requests the settings dialog queued this
    /// frame (see [`SdroxideApp::radio_tab_requests`]).
    pub(crate) fn take_radio_tab_requests(&mut self) -> Vec<RadioTabRequest> {
        std::mem::take(&mut self.radio_tab_requests)
    }

    /// Put a message in this tab's dismissable notice banner. The shell uses
    /// it for failures that have no strip to appear in (adding a radio while
    /// the main window's tab area is hidden).
    pub(crate) fn show_notice(&mut self, text: String) {
        self.radio_notice = Some(text);
    }

    /// Whether this radio has a transmitter at all.
    ///
    /// `tx_channels == 0` is how a receive-only interface says so — an RTL
    /// dongle, a SpyServer, a HackRF nobody armed — and it is the same field
    /// the engine's own transmit gate reads, so a control this answers `false`
    /// for could not have keyed anything anyway. Greying it out is what makes
    /// that visible before it is pressed instead of after.
    ///
    /// No capabilities yet reads as "cannot", matching the rest of the window:
    /// the top bar leaves its PTT out until the engine has said what it has.
    pub(in crate::app) fn tx_capable(&self) -> bool {
        self.caps.as_ref().is_some_and(|c| c.is_transmit_capable())
    }

    /// What this radio should be called while the operator hasn't named it:
    /// the interface it runs, read from the capabilities the engine reported —
    /// so the name follows a reconfiguration by itself. `None` while there is
    /// no interface to name it after (a fresh tab on the stand-in source, or
    /// no capabilities yet), where the shell falls back to a neutral
    /// "Radio N" — deliberately not "New radio", because the stand-in is also
    /// what a fully configured radio runs on while it is unreachable.
    pub(crate) fn interface_name(&self) -> Option<String> {
        let caps = self.caps.as_ref()?;
        Some(match caps.driver.as_str() {
            "none" => return None,
            "cat" => "CAT".into(),
            "hpsdr" => "HPSDR".into(),
            "tci" => "TCI".into(),
            "smartsdr" => "SmartSDR".into(),
            "pluto" => "PlutoSDR".into(),
            "rtlsdr" => "RTL-SDR".into(),
            // Named apart from the USB one: on a station with both, which tab
            // is the dongle on the mast is exactly what needs telling apart.
            "rtltcp" => "RTL-SDR (rtl_tcp)".into(),
            // The two SpyServer interfaces are named apart for the same
            // reason: which tab is the low-bandwidth one is the thing an
            // operator running both needs to see at a glance.
            "spyserver" => "SpyServer".into(),
            "spyserver-vfo" => "SpyServer (VFO)".into(),
            "rx888" => "RX-888".into(),
            // Distinct from the `"airspy"` arm below, which is SoapySDR's
            // driver for the R2 and Mini — a different radio entirely.
            "airspyhf" => "Airspy HF+".into(),
            "sdrplay" => "SDRplay".into(),
            // One tab for the whole radio on an FDM-DUO: the USB receiver, the
            // CAT link and the transmit sound card.
            "elad" => "ELAD".into(),
            // A SoapySDR device reports its Soapy driver name — the most
            // specific interface name there is. The common ones get their
            // proper spelling; the list is open-ended, so anything else is
            // capitalised as-is.
            "hackrf" => "HackRF".into(),
            "lime" => "LimeSDR".into(),
            "airspy" => "Airspy".into(),
            "sx" => "SXceiver".into(),
            "uhd" => "USRP".into(),
            other => {
                let mut c = other.chars();
                match c.next() {
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                    None => return None,
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Press a chip drawn inside a `Ui` that is enabled or not, and report
    /// whether the press reached it. Two passes: the first tells egui where the
    /// chip is, the second aims at it.
    fn press_chip(enabled: bool) -> bool {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 200.0));
        let draw = |ui: &mut egui::Ui| {
            ui.add_enabled_ui(enabled, |ui| crate::chrome::chip(ui, false, "REPLY")).inner
        };

        let mut at = egui::Pos2::ZERO;
        let _ =
            ctx.run_ui(egui::RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
                at = draw(ui).rect.center();
            });
        let button = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        let mut clicked = false;
        for events in [vec![egui::Event::PointerMoved(at), button(true)], vec![button(false)]] {
            let input = egui::RawInput { screen_rect: Some(screen), events, ..Default::default() };
            let _ = ctx.run_ui(input, |ui| clicked |= draw(ui).clicked());
        }
        clicked
    }

    /// Every transmit control greyed out for a receive-only radio is greyed the
    /// same way — drawn inside a disabled `Ui` — so this is the one mechanism
    /// all eleven operating panels rest on. A chip that merely *looked* dead
    /// while still reporting clicks would key a QSO from a panel that says it
    /// cannot, so pin it here rather than eleven times over.
    #[test]
    fn a_control_in_a_disabled_ui_cannot_be_clicked() {
        assert!(press_chip(true), "the same press has to work when the radio can transmit");
        assert!(!press_chip(false), "a greyed transmit control took a click");
    }
}
