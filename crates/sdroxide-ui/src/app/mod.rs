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
pub(in crate::app) mod drm;
pub(in crate::app) mod frame;
pub(in crate::app) mod ism;
pub(in crate::app) mod logbook;
pub(in crate::app) mod net;
pub(in crate::app) mod panels;
pub(crate) mod persist;
pub(in crate::app) mod publicsdr;
pub(in crate::app) mod qo100;
pub(in crate::app) mod rds;
pub(in crate::app) mod sat;
pub(in crate::app) mod scanner;
pub(in crate::app) mod settings;
pub(in crate::app) mod solar;
pub(in crate::app) mod spectrum;
pub(in crate::app) mod speech;
pub(in crate::app) mod spots;
pub(in crate::app) mod top_bar;
pub(crate) mod util;
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

/// How long after a connection drops the first redial goes out, in seconds.
/// Short enough that a station restarting is a blink; long enough that the
/// socket the far end has just closed is properly gone.
pub(in crate::app) const RETRY_MIN_S: f64 = 1.0;

/// The longest the wait between redials grows to. A station that is off for the
/// evening is checked twice a minute, which costs nothing and still has the
/// radio back within half a minute of it coming up.
pub(in crate::app) const RETRY_MAX_S: f64 = 30.0;

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
    /// When to redial the connection that produced [`Self::error`], and how
    /// long the wait before the one after that.
    ///
    /// A link that drops comes back by itself. It used to sit on a button, and
    /// the button is the wrong shape for what actually goes wrong out there: a
    /// station restarting, a laptop's Wi-Fi handing over, a Windows socket
    /// answering a timed-out read with an errno that ends the thread (issue
    /// #188) — all of them are over in seconds, and none of them is a decision
    /// anybody needs to be asked for. Nor is there anybody to ask on the tabs
    /// behind the one on screen, which is most of a station's radios.
    ///
    /// The wait doubles up to [`RETRY_MAX_S`] while the far end stays down, so
    /// a server that is off for the evening is not dialled sixty times a
    /// minute, and goes back to [`RETRY_MIN_S`] the moment a session is
    /// accepted. `None` means nothing is armed — either there is no error, or
    /// this controller has no connection to redial.
    retry_at: Option<f64>,
    retry_backoff: f64,
    /// Persistent, non-fatal operator notice (e.g. radio audio input
    /// unavailable / mono card selected for IQ). Shown as a warning banner.
    radio_notice: Option<String>,
    /// A newer release published on sdroxide.com — the version string
    /// itself. Set at most once, when the startup check lands; drawn in the
    /// notice banner's clothes above the panadapter, with a Dismiss that also
    /// remembers the version so the banner returns only for the next release.
    update_notice: Option<String>,
    /// The startup version check still in flight, if there is one.
    update_fetch: Option<std::sync::mpsc::Receiver<String>>,
    sent_cfg: Option<SpectrumConfig>,
    desired_cfg: Option<SpectrumConfig>,
    desired_at: f64,
    /// What this machine's renderer will carry, gathered once when the window
    /// opened. `None` where there is no wgpu at all (a headless test), which
    /// reads as "the width sdroxide has always drawn".
    display_class: Option<waterfall_gpu::DisplayClass>,
    /// The display's pixels per egui point, as of the last panadapter draw.
    ///
    /// Kept because the row rate asked of the engine has to carry it — the
    /// waterfall stores one line per device pixel — and the config is built
    /// outside the frame that measures it.
    wf_row_scale: f32,
    /// The widest the panadapter has been this session, in *device pixels*.
    ///
    /// Only ever grows. A window dragged narrower keeps the detail it had,
    /// because re-cutting the frame costs the waterfall its scrollback and buys
    /// back nothing but bandwidth — where a window dragged wider is the
    /// operator moving onto the screen they bought this for. It also keeps the
    /// width from thrashing against `cfg_still_good` while a window is resized.
    panadapter_px: u32,
    /// The visible span last handed to the skimmers, the one waiting to be, and
    /// when it settled. Separate from `sent_cfg` because the spectrum viewport
    /// carries deliberate slack and this must not.
    sent_skim_view: Option<(f64, f64)>,
    desired_skim_view: Option<(f64, f64)>,
    desired_skim_view_at: f64,
    /// Auto-fit's timers and the view it last measured — what keeps the
    /// floor/ceiling fitted while the FIT chip is lit.
    fit: spectrum::AutoFit,
    /// The (mode, dial, front-end centre) the digital sub-band view was last
    /// fitted for. In the free-roaming digital modes (everything but
    /// FT8/FT4/JS8/WSPR) the fit is applied only when this changes, so zoom/pan
    /// survive between frames.
    ///
    /// The centre is in here because a fit is only meaningful inside the span
    /// the front end is actually on, and the panadapter clamps it to that span.
    /// The first frame after startup is drawn before the engine's real state
    /// has arrived, so a window fitted then is clamped into the *default*
    /// span and is left stranded when the real one turns up — which is exactly
    /// what APRS does, being the one mode that moves the dial by itself.
    digi_view_fit: Option<(Mode, f64, f64)>,
    /// egui time of the last received spectrum frame, for stall detection.
    last_spectrum_at: f64,
    /// Waterfall time-scroll state: wall-clock (UTC secs) of the last tick and
    /// the carried fractional row, so the scroll rate is exact and independent
    /// of the frame rate (keeps the waterfall and time gridlines in lockstep).
    wf_last_now: f64,
    wf_row_accum: f32,
    /// The wall-clock time the waterfall's newest row stands for. Tracks the
    /// clock while rows are being appended and holds still while they are not
    /// (a switched-off radio, a stalled stream), so the time gridlines stay
    /// pinned to the rows they label instead of sliding over a frozen picture.
    wf_now_pin: f64,
    /// Cached spectrum polylines (recomputed only when frame/view/rect change).
    trace_cache: spectrum_view::TraceCache,
    /// The spectra behind the 3D surface, when the SPEC popup's 3D chip is lit
    /// — the panadapter's only piece of history that is not the waterfall's.
    spec3d: crate::widgets::spectrum3d::Surface,
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
    /// Which logging service the Uploads tab's own strip is showing. Session-only
    /// for the same reason as `settings_tab`: it is where the operator happens to
    /// be in the dialog, not a setting.
    settings_upload_tab: sdroxide_types::UploadTarget,
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
    /// Where this radio's antenna is, being edited on the Radio tab. Buffered
    /// until Apply for the same reason as the two above: it decides the square
    /// every reception report goes out under, and a locator half-typed into
    /// `radio.json` would be a square somewhere else entirely. `None` = not
    /// being edited.
    rx_site_edit: Option<sdroxide_types::RxSite>,
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
    /// HydraSDR RFOne receivers found on the last Rescan.
    hydrasdr_devices: Vec<sdroxide_types::HydraSdrDevice>,
    hackrf_devices: Vec<sdroxide_types::HackRfDevice>,
    /// RSPs the SDRplay API service reported on the last Rescan.
    sdrplay_devices: Vec<sdroxide_types::SdrPlayDevice>,
    /// Fobos SDRs found on the last Rescan.
    fobos_devices: Vec<sdroxide_types::FobosDevice>,
    /// The interface whose device list was last asked for, so switching to
    /// another one inside the open dialog asks for that one's — once, not
    /// every frame. `None` while the dialog is shut.
    iface_probed: Option<sdroxide_types::Backend>,
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
    kiwi_test_result: Option<TestOutcome>,
    /// The public-SDR directories, once the machine with the radio on it has
    /// answered. `None` until then, which is what the browse window shows a
    /// "fetching" line for; `Some` but empty means the fetch really did come
    /// back with nothing.
    public_sdrs: Option<sdroxide_types::PublicSdrDirectory>,
    show_public_sdrs: bool,
    /// Asked once per opening of the window, so reopening it after an hour does
    /// not go on showing an hour-old list.
    public_sdrs_asked: bool,
    public_sdr_search: String,
    /// Indexed by `PublicSdrNetwork::ALL`, positionally.
    public_sdr_nets_shown: [bool; 2],
    public_sdr_free_only: bool,
    public_sdr_in_band: bool,
    /// Take a SpyServer in its VFO+FFT shape rather than wideband. No effect on
    /// a KiwiSDR, which has only one shape.
    public_sdr_low_bw: bool,
    /// The receiver whose **USE** is waiting to be confirmed.
    ///
    /// USE replaces the radio the operator is on, and a radio is a station's
    /// worth of setting up — issue #254 is somebody losing an IC-9700 to a
    /// SpyServer on one click, with the tab still carrying the Icom's name
    /// afterwards. So the row's button arms this instead of acting, and the
    /// window puts the choice, and the way out of it, in front of the list.
    /// `None` is the ordinary state.
    public_sdr_confirm: Option<Box<sdroxide_types::PublicSdrEntry>>,
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
    /// The DRM window's open state, and the latest snapshot from the decoder.
    /// A whole snapshot each time — unlike RDS there is no delta inside it.
    show_drm: bool,
    drm: Option<sdroxide_types::DrmStatus>,
    /// Which logical channel's constellation the window is showing.
    drm_channel: sdroxide_types::DrmChannel,
    /// The constellation request last sent to the engine, so the command only
    /// goes when it actually changes — the decoder only reads the cells back
    /// while somebody is watching them.
    drm_const_req: Option<sdroxide_types::DrmChannel>,
    /// Signal quality over the last minute as (SNR, MER) pairs, accumulated
    /// here from the status stream rather than sent: the engine would be
    /// resending the same history several times a second to say one new thing.
    drm_history: std::collections::VecDeque<(f32, f32)>,
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
    /// The QO-100 beacon decoder's live status — lock state, measured
    /// frequency offset, decoded telemetry — as last reported by the engine.
    /// `None` until the decoder has been enabled at least once this session.
    qo100_status: Option<sdroxide_types::Qo100Status>,
    show_settings: bool,
    /// Scroll the Settings window back to its tab bar on the frame it opens.
    /// The window's scroll offset is egui memory, which outlives both the
    /// dialog and the process — without this, a restart shows wherever the
    /// operator last scrolled to, with the tabs off the top of the window.
    settings_scroll_top: bool,
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
    /// Which NAVTEX message the reading pane is showing, or `None` for the one
    /// arriving.
    navtex_open: Option<usize>,
    nr_popup_since: Option<f64>,
    rec_popup_since: Option<f64>,
    /// Fade clocks for the repeater popups in the VFO box — the DUPLEX shift
    /// and the TONE encoder — like `tone_popup_since`.
    duplex_popup_since: Option<f64>,
    rpt_tone_popup_since: Option<f64>,
    /// The layout in force last frame, so a change can re-apply the style
    /// metrics (chip padding, text sizes) exactly once instead of every frame.
    tier: crate::layout::Tier,
    /// What the compact layout's PTT chip is doing — pressed, latched, or
    /// nothing — so it sends one command per edge rather than one per frame.
    ptt: self::top_bar::PttPress,
    mem_name: String,
    /// Name being typed into the memories window's "new folder" field.
    mem_folder_name: String,
    /// Folder rename in progress: (folder id, text as typed). UI-owned until
    /// the edit commits, exactly like `voice_name_edit`.
    mem_folder_edit: Option<(u32, String)>,
    /// One-shot: focus the rename field on the frame after REN was clicked.
    mem_folder_focus: bool,
    /// The memory being edited in the list, if any — see
    /// [`self::windows::MemoryEdit`].
    mem_edit: Option<self::windows::MemoryEdit>,
    /// One-shot: focus the name field on the frame after EDT was clicked.
    mem_edit_focus: bool,
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
    /// The FT8/FT4 transmit-offset box, as typed. Kept as text rather than a
    /// number so a half-finished figure survives between frames: parsing every
    /// keystroke would rewrite "8" to 200 before the 2 was pressed.
    ///
    /// Refreshed from the engine only while the box is NOT focused, so the
    /// operator's typing is never overwritten by the value they are replacing.
    digi_tx_hz_edit: String,
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
    /// Packet: who the terminal's CONNECT button calls.
    packet_target: String,
    /// Packet: the digipeater path typed beside it.
    packet_via: String,
    /// Packet: what is typed on the terminal's input line but not yet sent.
    packet_draft: String,
    /// Packet: lines already sent, newest last, for the up-arrow.
    ///
    /// A node's command line is retyped constantly — the same `L`, the same
    /// `C CALL`, the same `B` — and at 300 baud retyping is where the typos
    /// come from.
    packet_history: Vec<String>,
    /// Packet: how far back through [`Self::packet_history`] the operator has
    /// walked. `None` means they are typing something new.
    packet_history_at: Option<usize>,
    /// APRS: the station icons, decoded once and kept as textures.
    aprs_icons: crate::aprs_icons::AprsIcons,
    /// APRS: the map's centre, zoom and selected station.
    aprs_map: crate::aprs_map::AprsMapState,
    /// ADS-B: the aircraft table and everything the decoder is seeing, as the
    /// engine last sent it. Boxed because it carries every target's position
    /// history and is far larger than anything else held here.
    adsb_status: Option<Box<sdroxide_types::AdsbStatus>>,
    /// ADS-B: the radar picture's pan/zoom and which target is selected.
    adsb_map: crate::adsb_map::AdsbMapState,
    /// ADS-B: filter for the aircraft list; matches a callsign or an address.
    adsb_filter: String,
    /// ADS-B: the decoder's own settings window is open.
    show_adsb_setup: bool,
    adsb_sort: panels::adsb::AdsbSort,
    adsb_sort_desc: bool,
    /// AIS: the vessel table and everything the decoder is seeing, as the
    /// engine last sent it. Boxed for the reason the ADS-B one is — a busy
    /// estuary's table, each row carrying a trail, is far larger than anything
    /// else held here.
    ais_status: Option<Box<sdroxide_types::AisStatus>>,
    /// AIS: the chart's pan/zoom and which vessel is selected.
    ais_map: crate::ais_map::AisMapState,
    /// AIS: filter for the vessel list; matches a name, a call sign, a
    /// destination or an MMSI.
    ais_filter: String,
    /// AIS: the decoder's own settings window is open.
    show_ais_setup: bool,
    ais_sort: panels::ais::AisSort,
    ais_sort_desc: bool,
    /// VDL2: everything the decoder has, as the engine last sent it. Boxed
    /// because it carries a whole message log and is far larger than anything
    /// else held here.
    vdl2_status: Option<Box<sdroxide_types::Vdl2Status>>,
    /// VDL2: filter for both panes at once — an address typed in one is the
    /// same question asked of the other.
    vdl2_filter: String,
    /// VDL2: index into the log of the message whose card is open.
    vdl2_selected: Option<usize>,
    /// VDL2: the decoder's own settings window is open.
    show_vdl2_setup: bool,
    vdl2_sort: panels::vdl2::Vdl2Sort,
    vdl2_sort_desc: bool,
    /// APRS: who the message box is addressed to.
    aprs_target: String,
    /// APRS: what is typed in the message box but not yet sent.
    aprs_draft: String,
    /// APRS: which pane of the message column is showing — the conversation or
    /// the raw frames on the channel.
    aprs_show_traffic: bool,
    /// APRS: filter for the station list; matches a callsign or a comment.
    aprs_filter: String,
    /// APRS: what is typed in the latitude and longitude boxes.
    ///
    /// Buffers rather than formatting the config value each frame: half a
    /// typed number does not parse, and a field that reverted to the stored
    /// value on every keystroke could not be typed into at all.
    aprs_lat_buf: String,
    aprs_lon_buf: String,
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
    adif_import_inbox: crate::download::LoadInbox,
    /// Inbox for a CHIRP CSV channel list chosen via the memories window's
    /// "Import" button — its own, beside the ADIF one, so a log import and a
    /// channel import cannot land in each other's parser.
    chirp_import_inbox: crate::download::LoadInbox,
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
    /// Which page of the SAT window is showing. QO-100 is one of them — it is
    /// a satellite, and it has no window of its own.
    sat_tab: sat::SatTab,
    /// What the QO-100 page remembers between frames.
    qo100_win: qo100::Qo100WinState,
    /// The rotctld client's health, mirrored from
    /// [`RadioEvent::RotatorStatus`].
    rotator_status: Option<(bool, f64, f64, Option<String>)>,
    /// The rotator settings dialog's working copy, seeded once like the
    /// server configs.
    rot_cfg_edit: sdroxide_types::RotatorConfig,
    rot_cfg_seeded: bool,
    /// The external T/R switch's health, mirrored from
    /// [`RadioEvent::RelayStatus`]. Replayed on connect by the server, so a
    /// remote client is told about a relay that is not answering before it
    /// touches PTT rather than after.
    relay_status: sdroxide_types::RelayStatus,
    /// The T/R switch settings dialog's working copy, seeded once like the
    /// server configs.
    relay_edit: sdroxide_types::RelayConfig,
    relay_seeded: bool,
    /// The switching devices the engine's machine can see, from
    /// [`sdroxide_types::DeviceProbe::Relays`].
    relay_devices: Vec<sdroxide_types::RelayDevice>,
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
    /// Whether the roster this radio is in can be edited from here — radios
    /// added beside it, and this one closed. True for one of this machine's own
    /// (the roster is a file here) and for one on a station that takes roster
    /// edits from a client; false for a browser tab's own machine, which has no
    /// roster, and for a station whose host did not wire that up.
    pub roster_editable: bool,
    /// Whether this is the first radio of the station that holds it — the one
    /// that runs the station-wide network services and answers at its plain
    /// `/ws`. It is not offered for closing anywhere, because no station will
    /// let go of it.
    pub first_of_station: bool,
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
    /// Put another radio in a roster. `station` is empty for this machine's own
    /// — the "+" the shell has always had — and otherwise the station key of a
    /// connection ([`SdroxideApp::station_key`]), which asks the station at the
    /// far end to add one to *its* roster.
    ///
    /// The distinction is the whole point on a native client: a screen holding
    /// its own dongle and a remote station has two rosters in one tab strip,
    /// and "add a radio" means nothing until it says which. A browser has only
    /// the station's.
    ///
    /// `preset` is what "Public SDRs" uses: the new radio comes up already
    /// pointed at the receiver that was picked, rather than arriving blank with
    /// its settings page open for an address to be typed into. `None` is the
    /// plain "+" chip, which does exactly that.
    Add {
        station: String,
        preset: Option<Box<sdroxide_types::RadioConfig>>,
    },
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
    /// Take this tab's radio out of the roster of the station it belongs to —
    /// a station somewhere else, reached over the network. Its configuration
    /// stays there, exactly as a local radio's does when it is closed here.
    ///
    /// Deliberately not what [`RadioTabRequest::Close`] does to such a tab:
    /// closing a connection is hanging up, and hanging up must not be able to
    /// delete somebody's radio. The tab named here is a *tab* id; the shell
    /// translates it into the id its station knows it by.
    RemoveFromStation(u32),
    /// Show the radios in this left-to-right order (issue #224): the whole
    /// strip, by tab id, as the operator dragged it.
    ///
    /// The whole order rather than "move this one there" so that applying it is
    /// idempotent and cannot half-happen: the shell sorts its tabs to match and
    /// ignores an id it does not have. Only this machine's own radios are
    /// recorded — a connection is not in any roster here — so a reordered
    /// connection stays where it was put for the session and comes back at the
    /// end of the strip next time.
    Reorder(Vec<u32>),
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
        crate::theme::set_spot_colors(&ui_settings.spot_colors);
        crate::theme::set_bandplan_colors(&ui_settings.bandplan_colors);
        crate::theme::apply(egui_ctx);
        // What this renderer will carry. Gathered here because it is the one
        // place that holds the render state and the controller at once, and
        // because none of it changes for the life of the window.
        let display_class = wgpu_render_state.as_ref().map(|rs| {
            let info = rs.adapter.get_info();
            waterfall_gpu::DisplayClass {
                max_texture_dim: rs.device.limits().max_texture_dimension_2d,
                device_type: info.device_type,
                backend: info.backend,
                remote: ctrl.engine_is_remote(),
                cores: std::thread::available_parallelism().map_or(1, |n| n.get() as u32),
            }
        });
        if let Some(rs) = &wgpu_render_state {
            waterfall_gpu::init(rs);
        }
        // The world under the flat maps, decoded off the UI thread while the
        // window is still coming up (see `crate::basemap`).
        crate::basemap::prime();
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
            display_class,
            wf_row_scale: 1.0,
            panadapter_px: 0,
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
            retry_at: None,
            retry_backoff: RETRY_MIN_S,
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
            wf_now_pin: 0.0,
            trace_cache: spectrum_view::TraceCache::default(),
            spec3d: Default::default(),
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
            settings_upload_tab: sdroxide_types::UploadTarget::QrzLogbook,
            ui_settings,
            applied_look: (ui_settings.theme, ui_settings.button_style, ui_settings.window_style),
            applied_ui_font: ui_settings.menu_font_size,
            speech: speech::SpeechRuntime::new(load_speech_settings(storage)),
            speech_voices: Vec::new(),
            radio_cfg: None,
            converter_edit_hz: None,
            range_edit: None,
            rx_site_edit: None,
            serial_ports: Vec::new(),
            hpsdr_devices: Vec::new(),
            rtlsdr_devices: Vec::new(),
            rx888_devices: Vec::new(),
            airspyhf_devices: Vec::new(),
            elad_devices: Vec::new(),
            lime_devices: Vec::new(),
            airspy_devices: Vec::new(),
            hydrasdr_devices: Vec::new(),
            hackrf_devices: Vec::new(),
            sdrplay_devices: Vec::new(),
            fobos_devices: Vec::new(),
            iface_probed: None,
            soapy_devices: None,
            tci_test_result: None,
            icomnet_test_result: None,
            spyserver_test_result: None,
            kiwi_test_result: None,
            public_sdrs: None,
            show_public_sdrs: false,
            public_sdrs_asked: false,
            public_sdr_search: String::new(),
            // Both on: the point of the window is to show what is out there.
            public_sdr_nets_shown: [true, true],
            // On, because a full receiver is not a receiver an operator can
            // use, and the reason is still shown for the ones this hides.
            public_sdr_free_only: true,
            public_sdr_in_band: false,
            public_sdr_low_bw: false,
            public_sdr_confirm: None,
            smartsdr_devices: Vec::new(),
            smartsdr_test_result: None,
            pluto_devices: Vec::new(),
            pluto_test_result: None,
            seen_first_state: false,
            show_memories: false,
            show_rds: false,
            show_drm: false,
            drm: None,
            drm_channel: sdroxide_types::DrmChannel::Msc,
            drm_const_req: None,
            drm_history: std::collections::VecDeque::new(),
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
            qo100_status: None,
            show_settings: false,
            settings_scroll_top: true,
            voice: sdroxide_types::VoiceStatus::default(),
            show_voice: false,
            voice_name_edit: None,
            mode_popup_since: None,
            fft_popup_since: None,
            skimmer_popup_since: None,
            layers_popup_since: None,
            tone_popup_since: None,
            navtex_open: None,
            nr_popup_since: None,
            rec_popup_since: None,
            duplex_popup_since: None,
            rpt_tone_popup_since: None,
            // Corrected on the first frame, once the viewport size is known.
            tier: crate::layout::Tier::Desktop,
            ptt: Default::default(),
            mem_name: String::new(),
            mem_folder_name: String::new(),
            mem_folder_edit: None,
            mem_folder_focus: false,
            mem_edit: None,
            mem_edit_focus: false,
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
            aprs_icons: crate::aprs_icons::AprsIcons::default(),
            aprs_map: crate::aprs_map::AprsMapState::default(),
            adsb_status: None,
            adsb_map: crate::adsb_map::AdsbMapState::default(),
            adsb_filter: String::new(),
            show_adsb_setup: false,
            adsb_sort: panels::adsb::AdsbSort::default(),
            adsb_sort_desc: true,
            ais_status: None,
            ais_map: crate::ais_map::AisMapState::default(),
            ais_filter: String::new(),
            show_ais_setup: false,
            ais_sort: panels::ais::AisSort::default(),
            ais_sort_desc: true,
            vdl2_status: None,
            vdl2_filter: String::new(),
            vdl2_selected: None,
            show_vdl2_setup: false,
            vdl2_sort: panels::vdl2::Vdl2Sort::default(),
            vdl2_sort_desc: true,
            aprs_target: String::new(),
            aprs_draft: String::new(),
            packet_target: String::new(),
            packet_via: String::new(),
            packet_draft: String::new(),
            packet_history: Vec::new(),
            packet_history_at: None,
            aprs_show_traffic: false,
            aprs_filter: String::new(),
            aprs_lat_buf: String::new(),
            aprs_lon_buf: String::new(),
            digi_cfg_seeded: false,
            digi_tx_hz_edit: String::new(),
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
            // One version check per start, from one tab for the whole
            // station — every tab is the same binary. Off when the operator
            // has switched the check off in Settings → UI.
            update_notice: None,
            update_fetch: if station_writer && ui_settings.update_check {
                persist::spawn_update_check()
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
            chirp_import_inbox: Arc::new(Mutex::new(None)),
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
            sat_tab: sat::SatTab::default(),
            qo100_win: Default::default(),
            rotator_status: None,
            rot_cfg_edit: Default::default(),
            relay_status: Default::default(),
            relay_edit: Default::default(),
            relay_seeded: false,
            relay_devices: Vec::new(),
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

    /// Whether the main receiver is muted — the one state both the MUTE
    /// button and the tab strip's mute chip show.
    pub(crate) fn tab_muted(&self) -> bool {
        self.state.rx[0].muted
    }

    /// The tab strip's mute toggle: the same per-receiver mute the MUTE
    /// button sends, so muting from either place lights both — and the
    /// engine, which owns the state, remembers it across restarts.
    pub(crate) fn set_tab_muted(&mut self, muted: bool) {
        self.ctrl.send(sdroxide_types::Command::SetMute { rx: sdroxide_types::RxId::Main, muted });
    }

    /// The browser's one-output rule: keep this radio's audio out of the
    /// page's single worklet while another tab is focused. Not the operator's
    /// mute — that is [`Self::set_tab_muted`].
    #[cfg(target_arch = "wasm32")]
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

    /// Whether this radio is switched on — what the top bar's power chip
    /// shows. `None` when there is no switch to draw: the roster is empty, the
    /// radio is at the far end of a station that holds no switch a client may
    /// throw, or its front end is lent out as somebody's panadapter receiver —
    /// the same radios the strip offers no switch for.
    ///
    /// Read from the shell's roster either way, so a radio of this machine's
    /// own and one at a station are answered the same: the shell is what knows
    /// which of the two this tab is, and it has already asked the station.
    pub(crate) fn own_power_state(&self) -> Option<bool> {
        let chip = self.radio_roster.iter().find(|c| c.id == self.radio_id)?;
        (chip.switchable && chip.attached_to.is_none()).then_some(chip.enabled)
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

    /// The position of the switch the station at the far end holds for this
    /// radio, or `None` where it holds none. The shell asks once a frame and
    /// publishes the answer in the roster — see
    /// [`sdroxide_types::RadioController::station_power`].
    pub(crate) fn station_power(&self) -> Option<bool> {
        self.ctrl.station_power()
    }

    /// Ask that station to switch one of its radios on or off, by its own id.
    pub(crate) fn switch_station_radio(&mut self, id: u32, on: bool) {
        self.ctrl.switch_station_radio(id, on);
    }

    pub(crate) fn peer_name(&self) -> Option<String> {
        self.ctrl.peer_name()
    }

    /// The far station's first radio, as it numbers it — the one it will not
    /// let a client close.
    pub(crate) fn peer_first_radio(&self) -> Option<u32> {
        self.ctrl.peer_first_radio()
    }

    /// Whether the station this tab is connected to lets a client add radios to
    /// its roster and take them out again.
    pub(crate) fn station_roster_editable(&self) -> bool {
        self.ctrl.station_roster_editable()
    }

    /// Ask that station for another radio. What answers is its roster, which
    /// it announces to everyone on it — the new radio then arrives as one more
    /// of the station's, and the shell opens it beside this one.
    pub(crate) fn add_station_radio(&mut self, name: &str) {
        self.ctrl.add_station_radio(name);
    }

    /// Ask that station to take one of its radios — named as *it* numbers them
    /// — out of its roster.
    pub(crate) fn remove_station_radio(&mut self, id: u32) {
        self.ctrl.remove_station_radio(id);
    }

    /// Record the operator's name for one of that station's radios, in the
    /// roster where it lives.
    pub(crate) fn rename_station_radio(&mut self, id: u32, name: &str) {
        self.ctrl.rename_station_radio(id, name);
    }

    /// Whether the station has said this tab's radio is no longer one of its
    /// own — see [`sdroxide_types::RadioController::peer_removed`].
    pub(crate) fn peer_removed(&self) -> bool {
        self.ctrl.peer_removed()
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
    ///
    /// Returns whether this tab has an answer ready and is only waiting its
    /// turn at the station — a station judges one sign-in at a time. The shell
    /// keeps asking for frames while that is true, because a hidden tab has no
    /// other reason to be redrawn and its turn comes on somebody else's socket.
    pub(crate) fn poll_auth(&mut self) -> bool {
        let phase = self.ctrl.auth_phase();
        self.login.settle(&phase, &self.station_key());
        if !phase.is_pending() {
            return false;
        }
        if let Some(a) = self.login.answer_without_asking(&phase) {
            self.ctrl.send_auth(a.username, a.password);
            return false;
        }
        self.login.waiting_for_turn()
    }

    /// Line up the next redial after a connection has dropped.
    ///
    /// The wait carries a stagger of up to a second, spread by radio: a station
    /// of four radios loses all four sockets in the same instant, and four
    /// clients dialling back on the same millisecond arrive at a sign-in that
    /// judges one answer at a time and tells the other three to come back.
    fn arm_retry(&mut self, now: f64) {
        // Already lined up. A dropped link is often reported twice — the socket
        // errors and then closes — and neither the wait nor the backoff may be
        // charged for that twice.
        if self.retry_at.is_some() || !self.ctrl.can_reconnect() {
            return;
        }
        let stagger = 0.25 * f64::from(self.radio_id % 4);
        self.retry_at = Some(now + self.retry_backoff + stagger);
        // Charged here rather than when the attempt goes out, so the *next*
        // wait is the longer one and pressing the button really does start
        // again from the bottom.
        self.retry_backoff = (self.retry_backoff * 2.0).min(RETRY_MAX_S);
    }

    /// Dial again now, whatever the clock says. The screen clears optimistically
    /// — the handshake finishes asynchronously, and a socket that fails again
    /// reports itself through the same event that put us here.
    fn reconnect_now(&mut self) {
        let now = crate::time::now_unix_f64();
        self.error = None;
        self.retry_at = None;
        self.frame = None;
        self.wide_frame = None;
        // The new session starts on the engine's own spectrum config, not the
        // one the old session was told. Forgetting what was sent is what makes
        // this frame's debounce push it again.
        self.sent_cfg = None;
        self.desired_cfg = None;
        if let Err(e) = self.ctrl.reconnect() {
            // Threw before the socket was even opened, so nothing is going to
            // report this one asynchronously — line the next attempt up here.
            self.error = Some(e);
            self.arm_retry(now);
        }
    }

    /// Redial if the wait is up. Returns whether one is still pending, so the
    /// shell knows to keep asking for frames.
    ///
    /// Called for every tab, on screen or not: three of a station's four radios
    /// are behind the one being looked at, and a radio nobody is looking at has
    /// to come back on its own — there is no button there to press.
    pub(crate) fn poll_reconnect(&mut self) -> bool {
        let Some(at) = self.retry_at else { return false };
        let now = crate::time::now_unix_f64();
        // `>=` and the step-backwards guard together: this is a wall clock, and
        // an NTP correction must not park the next attempt hours in the future.
        if now >= at || now < at - RETRY_MAX_S {
            self.reconnect_now();
        }
        self.retry_at.is_some()
    }

    /// Detach from this radio: disconnect the engine, drop the audio streams,
    /// join the DSP thread. Tab close and app exit.
    pub(crate) fn shutdown_ctrl(&mut self) {
        self.ctrl.shutdown();
    }

    /// Open the Settings dialog on the Radio tab — where a freshly added
    /// radio, which has no interface yet, is configured.
    /// Point this radio at a configuration and reopen its interface.
    ///
    /// The same two calls Settings → Radio's "Apply / reconnect" makes, and for
    /// the same reason they are two: the configuration is what gets persisted,
    /// and the reopen is what makes it take effect without a restart. Works
    /// unchanged over a connection — the remote controller sends the whole
    /// block and asks for the reopen with it.
    pub(crate) fn apply_radio_config(&mut self, cfg: sdroxide_types::RadioConfig) {
        self.ctrl.set_radio_config(cfg);
        self.ctrl.reopen_source();
    }

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

    /// The frequency actually being worked, which in CW and the digital modes
    /// is not the dial: the receiver's cursor sits an audio offset away from
    /// it, and both ends of the contact are on *that* frequency.
    ///
    /// The number to log and to quote on the air. A CW dial reads a
    /// sidetone-pitch (700 Hz, usually) below the signal being copied and
    /// keyed; RTTY reads a tone pair — a couple of kilohertz — below its mark.
    /// See [`Mode::on_air_hz`](sdroxide_types::Mode::on_air_hz).
    /// The CW tone being copied, in Hz — the cursor the CW panel moves and the
    /// pitch the engine keeps the passband centred on.
    ///
    /// The controller's figure where there is one, because the operator moves
    /// it by clicking the waterfall; the stored default only stands in before
    /// the CW engine has reported. Same rule as the engine's own
    /// `cw_pitch_hz`, and the two have to agree or the readout and the
    /// passband would disagree about where the signal is.
    pub(in crate::app) fn cw_pitch_hz(&self) -> f32 {
        self.digi_status.as_ref().map_or(self.digi_cfg_edit.cw_pitch_hz, |s| s.audio_hz)
    }

    pub(in crate::app) fn on_air_freq_hz(&self) -> f64 {
        let audio_hz = self.digi_status.as_ref().map_or(0.0, |s| s.audio_hz);
        self.state.rx[0].mode.on_air_hz(self.state.rx_freq_hz(), audio_hz)
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
        ctx.run_ui(egui::RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
            at = draw(ui).rect.center();
        })
        .drop_without_applying_deltas();
        let button = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        let mut clicked = false;
        for events in [vec![egui::Event::PointerMoved(at), button(true)], vec![button(false)]] {
            let input = egui::RawInput { screen_rect: Some(screen), events, ..Default::default() };
            ctx.run_ui(input, |ui| clicked |= draw(ui).clicked()).drop_without_applying_deltas();
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
