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

pub(in crate::app) mod alerts;
pub(in crate::app) mod auto_mode;
pub(in crate::app) mod awards;
pub(in crate::app) mod bands;
/// **Classic UI** — the left-column arrangement. Native only: it takes a column
/// off the desktop and a browser has one window with nowhere to put it.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) mod classic;
pub(in crate::app) mod contest;
#[cfg(all(not(target_arch = "wasm32"), target_os = "linux"))]
pub(in crate::app) mod cw_key;
pub(in crate::app) mod dial_hold;
pub(in crate::app) mod drm;
pub(in crate::app) mod enigma;
pub(in crate::app) mod frame;
pub(in crate::app) mod grid_tracker;
pub(in crate::app) mod hd;
pub(in crate::app) mod hfdl;
pub(in crate::app) mod ignore;
pub(in crate::app) mod ism;
pub(in crate::app) mod known_calls;
pub(in crate::app) mod konami;
pub(in crate::app) mod listen;
pub(in crate::app) mod logbook;
pub(in crate::app) mod morse;
pub(in crate::app) mod net;
pub(in crate::app) mod panels;
pub(crate) mod persist;
pub(in crate::app) mod playback;
pub(in crate::app) mod publicsdr;
pub(in crate::app) mod qo100;
pub(in crate::app) mod rds;
pub(in crate::app) mod recording_jobs;
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) mod rotor;
pub(in crate::app) mod sat;
pub(in crate::app) mod save_text;
pub(in crate::app) mod scanner;
pub(in crate::app) mod schedule;
pub(in crate::app) mod settings;
pub(in crate::app) mod signal_id;
pub(in crate::app) mod solar;
pub(in crate::app) mod spectrum;
pub(in crate::app) mod speech;
pub(in crate::app) mod spots;
mod swl_log;
pub(in crate::app) mod top_bar;
/// The update check — native only (a browser has no updater to point at).
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) mod update;
pub(crate) mod util;
mod weather;
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
    load_alerts_settings, load_broadcast_favourites, load_broadcast_stations, load_hfdl_channels,
    load_morse_progress, load_qso_log, load_recording_jobs, load_speech_settings, load_swl_log,
    load_swl_report, load_ui_settings, recordings_dir_for_display,
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
/// transceiver, greyed out and explaining itself on a receiver, or hidden
/// entirely in SWL mode.
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
    if SWL_ACTIVE.load(std::sync::atomic::Ordering::Relaxed) {
        // SWL mode: hide the control entirely rather than greying it out.
        return ui.label("");
    }
    rx_only_hint(ui.add_enabled_ui(tx_ok, add).inner, tx_ok)
}

/// Global flag mirrored from [`SdroxideApp::swl_mode`] every frame so
/// `tx_gated` can read it without threading the setting through every panel.
static SWL_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Update the global SWL flag. Called once per frame from the app.
pub(in crate::app) fn set_swl_active(on: bool) {
    SWL_ACTIVE.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// Whether a tool window is undocked into its own OS window, and where that
/// window last was. See [`SdroxideApp::tool_windows`].
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ToolWindowState {
    /// Drawn in its own OS window instead of the in-viewport egui window.
    pub undocked: bool,
    /// Where its window last was; `None` until it has been open once.
    pub window: Option<sdroxide_types::DetachedWindow>,
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
    /// The **AUX SP** window's own view — its zoom, its pan, its layers. The
    /// panadapter draw reads `view`, so the AUX window is drawn with this
    /// **swapped in** around it (see `show_detached_module`): that keeps two
    /// spectra from mirroring each other without threading a view parameter
    /// through the whole draw. Session-only for now — the main view's zoom is
    /// persisted, this one is not yet.
    #[cfg(not(target_arch = "wasm32"))]
    aux_view: ViewState,
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
    /// Another sdroxide found running on this machine at start-up — see
    /// [`SdroxideApp::warn_other_instance`]. Kept apart from `radio_notice`
    /// because the engine clears that one with its own notices, and this is
    /// not the engine's to clear: it stays until the operator dismisses it.
    other_instance: Option<String>,
    /// The mode for which the "your undocked panel has nowhere to go" notice has
    /// already been raised. Set once per mode so the banner is not re-raised
    /// after the operator dismisses it; cleared when the mode has a panel again.
    #[cfg(not(target_arch = "wasm32"))]
    panel_undock_notice_mode: Option<sdroxide_types::Mode>,
    /// Undocked **tool windows** — the scanner, the schedule, the logbook and the
    /// rest — by their stable id. Session-only, like the in-viewport position
    /// egui keeps for the same windows: a tool is transient, and where it sat
    /// last run is not worth a config field per tool.
    #[cfg(not(target_arch = "wasm32"))]
    tool_windows: std::collections::BTreeMap<&'static str, ToolWindowState>,
    /// Dismissed the receive-only nudge this session. The nudge offers a
    /// receive-only radio the per-radio listening screen; an operator who has
    /// already said no must not be asked on every retune.
    rx_only_nudge_dismissed: bool,
    /// Whether the Retro Radio faceplate's decode window is open. Session-only:
    /// it is a view choice, not a station setting, so it is not persisted.
    /// The contest logger: whether its window is open, the running session
    /// (`None` until START), the contest picked in setup, our own exchange and
    /// the entry being typed. Session-only; the QSOs it logs go to `qso_log`.
    show_contest: bool,
    contest: Option<sdroxide_types::ContestSession>,
    contest_pick: sdroxide_types::ContestId,
    contest_my_exchange: String,
    contest_entry: crate::app::contest::ContestEntry,
    /// The FT8 contest layout the digi engine held **before** this session
    /// applied its own, put back on STOP.
    ///
    /// A session that sets a layout must not leave it set: otherwise the CQ
    /// keeps the contest's calling message after the operator has stopped, and
    /// looking at the digi panel shows a layout they did not choose and cannot
    /// remember choosing. `Some` only while a session has changed it, so a
    /// session with no FT8 layout leaves the engine exactly as it found it.
    contest_prev_digi: Option<sdroxide_types::ContestMode>,
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
    /// The name being typed in the Profiles tab's save box. Owned on the app
    /// because the dialog lives across taps of the tab bar, and a half-typed
    /// name is not a setting.
    profile_name_edit: String,
    /// The station's saved profile names (issue #197), announced by the engine
    /// at start and after every save/apply/delete. Only the names — the
    /// profiles themselves stay with the radio's other remembered files.
    ///
    /// Empty until the announcement arrives — at connect, for a remote or
    /// browser client, which the server replays it to.
    profiles: Vec<String>,
    /// A profile was put on from this screen and the engine has not answered
    /// yet. Its answer — the profile list, announced once the apply is done —
    /// clears [`Self::digi_cfg_seeded`], so the editable digital settings
    /// re-seed from the next status, which the engine can only send after the
    /// apply, and the next edit cannot put the old callsign back.
    profile_apply_pending: bool,
    /// Which logging service the Uploads tab's own strip is showing. Session-only
    /// for the same reason as `settings_tab`: it is where the operator happens to
    /// be in the dialog, not a setting.
    settings_upload_tab: sdroxide_types::UploadTarget,
    /// Display preferences (frame rate, waterfall + spectrum speed), loaded from
    /// config at startup, edited in the UI tab, persisted on change.
    ui_settings: sdroxide_types::UiSettings,
    /// Whether this session started in SWL mode because the stored
    /// `start_swl` preference or `--swl` asked for it. A seed, not a setting:
    /// the per-radio switch in Settings → Radio clears it, so the operator's
    /// explicit per-radio choice is what governs once they have made one.
    swl_start: bool,
    /// The theme and chrome styles last written into the egui context, so the
    /// top-of-frame check in `frame.rs` can re-apply the visuals the moment
    /// the settings dialog changes any of them — no restart.
    applied_look:
        (sdroxide_types::UiTheme, sdroxide_types::ChromeStyle, sdroxide_types::ChromeStyle),
    /// **Classic UI**: the column is folded away. Session-only, and separate
    /// from the setting, because folding a column to see the picture is a thing
    /// done for a minute rather than a preference — SDR++'s ☰ is the same
    /// gesture. The setting decides whether the arrangement is in use at all.
    pub(in crate::app) classic_collapsed: bool,
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
    /// Audible alerts: played over a separate output, so they are heard even
    /// when the radio audio is somewhere else. Always present — switched off,
    /// the sink is closed and the cost is a boolean per decode batch. One for
    /// the whole station: every radio tab holds the same one.
    alerts: alerts::AlertRuntime,
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
    public_sdr_nets_shown: [bool; sdroxide_types::PublicSdrNetwork::ALL.len()],
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
    /// The dial and the centre this screen has just sent, held against
    /// engine snapshots that have not caught up with them. See [`dial_hold`].
    dial_hold: dial_hold::DialHold,
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
    /// The HD Radio window's open state, and the latest snapshot from the
    /// decoder. A whole snapshot each time, like DRM.
    show_hd: bool,
    hd: Option<sdroxide_types::HdRadioStatus>,
    /// Lower and upper sideband MER over the last minute, as (lower, upper)
    /// dB pairs, accumulated here from the status stream rather than sent.
    hd_history: std::collections::VecDeque<(f32, f32)>,
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
    /// The HFDL ground-network decoder's live status — level, decode count and
    /// the rolling decode log — as last reported by the engine. `None` until
    /// the decoder has been enabled at least once this session.
    hfdl_status: Option<sdroxide_types::HfdlStatus>,
    /// HFDL: the aircraft plot table and the map's pan/zoom and selection. Its
    /// own table rather than the decode log's, so an aircraft stays on the map
    /// after its earliest decodes have scrolled out of the log.
    hfdl_map: crate::hfdl_map::HfdlMapState,
    /// HFDL: filter for the decode log; matches a kind, a ground station or the
    /// payload details.
    hfdl_filter: String,
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
    /// Saved window arrangements — SDRuno's **workspaces**. A list of its own
    /// rather than a `UiSettings` field, because `UiSettings` is `Copy`; see
    /// [`sdroxide_types::Workspace`]. Loaded from the shared `workspaces.json`.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    workspaces: Vec<sdroxide_types::Workspace>,
    /// What is being typed into the next **SAVE CURRENT**.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    workspace_name: String,
    /// The startup update check, and whether its banner has been dismissed this
    /// session. Native only; `None` while the check is off or still in flight.
    #[cfg(not(target_arch = "wasm32"))]
    update: update::UpdateCheck,
    #[cfg(not(target_arch = "wasm32"))]
    update_dismissed: bool,
    /// Which half of the band/mode menu was last shown (listen or operate).
    band_menu_tab: top_bar::BandMenuTab,
    /// Which slice of the spectrum the band menu's chips are narrowed to (HF,
    /// VHF, UHF or all). Session UI state: it does not move the dial and is not
    /// worth remembering across restarts.
    band_filter: top_bar::BandFilter,
    fft_popup_since: Option<f64>,
    skimmer_popup_since: Option<f64>,
    /// Fade clock for the SPEC popup's layer chips, like `skimmer_popup_since`.
    layers_popup_since: Option<f64>,
    /// Fade clock for the sub-audible tone popup, like `skimmer_popup_since`.
    tone_popup_since: Option<f64>,
    /// Which NAVTEX message the reading pane is showing, or `None` for the one
    /// arriving.
    navtex_open: Option<usize>,
    /// Which DSC sequence the reading pane is showing, or `None` for the
    /// newest.
    dsc_open: Option<usize>,
    /// Which UVPacket frame the reading pane is showing, or `None` for the
    /// newest.
    uvpacket_open: Option<usize>,
    /// Fade clock for the noise-reduction picker, like `tone_popup_since`.
    nr_popup_since: Option<f64>,
    rec_popup_since: Option<f64>,
    /// Fade clock for the receive-tone (EQ) popup, like `rec_popup_since`.
    eq_popup_since: Option<f64>,
    /// When the running MP3 recording should stop, Unix UTC seconds, and the
    /// preset in **seconds** that asked for it — armed by the REC popup's
    /// "quick clip" and "stop after" chips. UI-owned: the engine's
    /// [`sdroxide_types::Command::SetRecording`] carries no deadline, so this
    /// is a per-tab timer ticked once a frame rather than a setting, and it
    /// does not survive a restart — which is fine for a "this over"
    /// convenience (issue #520). Cleared when the recording stops for any
    /// reason.
    ///
    /// The preset is carried rather than worked back out of the deadline
    /// because the remaining time does not identify it: a quarter of an hour
    /// left is a quarter of an hour left whether the operator asked for 15
    /// minutes or is most of the way through 90, and the chip that reads as
    /// armed has to be the one they pressed. Seconds rather than minutes
    /// because a quick clip is 30 s long.
    recording_stop_at: Option<(i64, u16)>,
    /// A quick clip asked for while no recording was running, `(asked_at,
    /// secs)`, waiting for the recorder to actually start.
    ///
    /// A clip's deadline cannot be armed at the press: the timer tick drops
    /// any deadline whose recording is not running, so an early deadline
    /// would be cleared on the next frame before the start took. The request
    /// rides here instead and is armed the first frame the recording is seen
    /// running — measured from that frame, so the clip is a full span — or
    /// dropped if the start never takes. See
    /// [`Self::poll_recording_timer`].
    rec_clip: Option<(i64, u16)>,
    /// The silence auto-split hold, in seconds, or `None` when it is off.
    ///
    /// While armed, the MP3 recording follows the receiver's squelch: a file
    /// starts when the squelch opens and the one running is closed after this
    /// many seconds of silence, so a session of many transmissions becomes one
    /// stamped file each rather than one file that grows all afternoon (issue
    /// #546). UI-owned and session-only, exactly like
    /// [`Self::recording_stop_at`], and ticked once a frame by
    /// [`Self::poll_recording_gate`].
    rec_gate_s: Option<u16>,
    /// Everything the gate carries between frames — the run of silence, a start
    /// still pending, and the two holds that stop a manual stop or a refused
    /// start being undone every frame. See [`Self::poll_recording_gate`].
    rec_gate: top_bar::RecGate,
    /// Fade clock for the receive-filter popup behind the BW chip, like
    /// `nr_popup_since`.
    bw_popup_since: Option<f64>,
    /// Fade clocks for the repeater popups in the VFO box — the DUPLEX shift
    /// and the TONE encoder — like `tone_popup_since`.
    duplex_popup_since: Option<f64>,
    rpt_tone_popup_since: Option<f64>,
    /// Whether the band/mode selector is docked beside the panadapter rather
    /// than opened from the top-bar chip, and whether the docked column is
    /// currently shown. Session UI state: hiding the column leaves it docked,
    /// so the band chip brings it straight back. A window too narrow for the
    /// column (see `band_dock_room`) leaves both alone and simply does not draw
    /// it, so widening the window again brings it back.
    band_docked: bool,
    band_dock_visible: bool,
    /// The console's band keypad: which of the two it is in, and the frequency
    /// typed so far. Session UI state — see [`top_bar::BandKeypad`] — and
    /// console-only, because the keypad is the console's RX-control surface.
    #[cfg(not(target_arch = "wasm32"))]
    band_keypad: top_bar::BandKeypad,
    /// How wide the docked column may be this frame, or `None` where it cannot
    /// dock — see [`top_bar::band_dock_room`]. Settled at the top of the frame
    /// from this app's own column, before the top bar draws the chip that
    /// shows and hides it.
    band_dock_room: Option<f32>,
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
    /// The receive dial each entry of `digi_decodes` arrived on, index for
    /// index. A decode carries only its audio offset, and the list is kept
    /// across a QSY inside the band, so the dial at export time is not the one
    /// an older decode was heard on.
    digi_decode_dials: Vec<f64>,
    digi_status: Option<DigiStatus>,
    /// Callsigns muted for this session, from a decode row's **−** button.
    ///
    /// Deliberately a `HashSet` in the app and nothing more: no file, no wire
    /// type, no schema, and no `PROTO_VERSION`. A memory list is cheaper to
    /// run and to build than a persisted one, and it cannot leave a station
    /// permanently hidden because of a press made by accident. Filtered at
    /// ingress — see [`ignore`] — so a muted station is out of the list, the
    /// alerts, the read-out and auto mode at once.
    session_ignored: ignore::Ignored,
    /// Coded weather reports (SYNOP / SHIP) seen on the text decode path,
    /// newest first. Session-only — they are of no use tomorrow, and a stored
    /// report read on another day is a lie about when it was observed.
    pub weather_reports: Vec<(i64, sdroxide_types::synop::Report)>,
    /// The WEATHER window's own open state.
    pub weather: crate::app::weather::WeatherWindow,
    /// PSK/RTTY outgoing text buffer (UI-owned; streamed to the engine, which
    /// reports back how many characters have been sent so we colour them green).
    text_tx: String,
    qso_log: Vec<QsoRecord>,
    /// The listener's reception log (`swl_log.json`) — separate from the QSO
    /// log on purpose. See [`crate::app::swl_log`].
    pub(in crate::app) swl_log: Vec<sdroxide_types::SwlEntry>,
    /// What a reception report carries besides the reception: the picture the
    /// listener wants a station to see, and their standing note. Its own file
    /// beside the log — see [`sdroxide_config::load_swl_report`].
    pub(in crate::app) swl_report: sdroxide_types::SwlReportPrefs,
    /// What the last **MAIL REPORT…** press did, said plainly. None until one is
    /// pressed; some until the next entry opens.
    pub(in crate::app) swl_report_note: Option<String>,
    /// Where the Settings row's file picker leaves the path it chose.
    pub(in crate::app) swl_report_pick: std::sync::Arc<std::sync::Mutex<Option<String>>>,
    /// The reception log window's own state: open, the entry being edited, and
    /// the row the REPORT button acts on.
    pub(in crate::app) show_swl: bool,
    /// The LISTEN window (`listen.rs`): the schedule on a map with a comment box.
    pub(in crate::app) show_listen: bool,
    /// Show logged stations on the LISTEN map as well as what is on now.
    ///
    /// **Off by default**, on purpose: an experienced listening station does not
    /// want its own history cluttering the band it is listening to. Turning it on
    /// is the screenshot, and the toggle is right there.
    pub(in crate::app) listen_history: bool,
    /// The station picked on the LISTEN map or from its list. Held whole rather
    /// than as an index, so a reload of the schedule cannot leave a stale
    /// selection behind.
    pub(in crate::app) listen_sel: Option<sdroxide_types::BroadcastStation>,
    /// What the operator is typing into the LISTEN comment box.
    pub(in crate::app) listen_comment: String,
    /// The LISTEN map's own view.
    ///
    /// **Not** the shared `map_view`: `worldmap::show` eases toward a target
    /// fitted to the stations it was *given*, so two maps sharing one view would
    /// each pull the other toward its own fit and fight over the zoom.
    pub(in crate::app) listen_map_view: crate::widgets::worldmap::MapView,
    pub(in crate::app) swl_edit: Option<crate::app::swl_log::SwlEditForm>,
    pub(in crate::app) swl_selected: Option<u64>,
    /// The antenna the listener is using, in their own words, for the
    /// reception report's **Antenna:** line. Session-only and free text, set
    /// once when a listening session starts: an aerial is swapped far more
    /// often than a setting is opened, so it is not worth a trip to Settings —
    /// and it is not a property of the radio, so it does not ride the wire.
    pub(in crate::app) swl_antenna: String,
    /// How the listener is looking through the reception log right now — the
    /// find box, band, day and the pirate-only switch. Session-only, like the
    /// antenna box: a way of reading the log for the moment, not a property of
    /// it.
    pub(in crate::app) swl_filter: crate::app::swl_log::SwlFilter,
    /// The broadcast schedule window and its filters.
    pub(in crate::app) schedule: crate::app::schedule::ScheduleUi,
    /// Favourite broadcast stations, by name (`broadcast_favourites.json`).
    pub(in crate::app) broadcast_favs: Vec<String>,
    /// The operator's own HFDL channels, in kHz (`hfdl_channels.json`) — the
    /// busy assigned frequencies that are not in the published plan, drawn as
    /// one-click chips beside it in the HFDL panel.
    pub(in crate::app) hfdl_channels: Vec<u32>,
    /// Scheduled recordings: the jobs, and the clock that runs them.
    pub(in crate::app) recording_jobs: Vec<sdroxide_types::RecordingJob>,
    pub(in crate::app) jobs: crate::app::recording_jobs::JobsUi,
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
    /// Whether the CW panel's message-button editor is open (issue #374).
    /// Screen state, not the operator's: the buttons themselves live in
    /// `DigiConfig`.
    cw_macro_edit: bool,
    /// The keyboard-mode message editor (`DigiConfig::text_macros`), the same
    /// window on its own list.
    text_macro_edit: bool,
    /// Whether the CW panel's keyboard-as-straight-key mode is engaged (issue
    /// #322). Screen state: the keyer's engagement lives in the controller,
    /// and this is the toggle's face and where the key is read.
    cw_straight: bool,
    /// Whether the straight key was down the last frame, so a press or release
    /// is told once — never per frame. Straight keying only: iambic has no
    /// "down" to report, it has contacts.
    cw_key_down: bool,
    /// The two paddle contacts as last sent to the engine (issue #569), so a
    /// poll that changed nothing sends nothing. A paddle's state is what goes
    /// up the wire; the elements are the engine's to make.
    cw_contacts_sent: (bool, bool),
    /// The USB paddle, when one is the CW key and the panel has armed it.
    #[cfg(all(not(target_arch = "wasm32"), target_os = "linux"))]
    cw_key: Option<cw_key::CwKeySource>,
    /// A failed paddle start, said in the panel rather than swallowed.
    #[cfg(all(not(target_arch = "wasm32"), target_os = "linux"))]
    cw_key_error: Option<String>,
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
    /// Held unaddressed; the composer adds the callsign when it is sent.
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
    /// AtCHAT: what is typed on the chat line but not yet sent. The active
    /// chat tab decides where it goes — the common channel, or one callsign.
    atchat_draft: String,
    /// AtCHAT: the open direct-message tabs, in the order they appeared. The
    /// common "CHAT" tab is always present and is not in this list.
    atchat_dm_tabs: Vec<String>,
    /// AtCHAT: the selected chat tab. `None` is the common "CHAT" tab (lines go
    /// to ALL); `Some(call)` is that station's direct-message tab.
    atchat_chat_tab: Option<String>,
    /// AtCHAT: the permanent LOG tab is up — the transcript area shows the
    /// station's on-air activity log instead of a conversation. Takes
    /// precedence over [`Self::atchat_chat_tab`] while set; any other tab
    /// click clears it.
    atchat_show_log: bool,
    /// AtCHAT: newest incoming direct-message timestamp already turned into a
    /// tab, per peer — so closing a tab does not make an old message reopen it.
    atchat_dm_seen: std::collections::HashMap<String, u64>,
    /// AtCHAT: newest incoming direct-message timestamp the operator has viewed
    /// for that peer — drives the unread dot on an inactive DM tab.
    atchat_dm_read: std::collections::HashMap<String, u64>,
    /// AtCHAT: which received image the viewer is showing — an index into the
    /// image-only subset of the status file list, oldest first.
    atchat_img_at: usize,
    /// AtCHAT: received images decoded from disk once and kept as textures,
    /// keyed by file path. `None` marks a path that would not decode, so it is
    /// not retried every frame.
    atchat_img_cache: std::collections::HashMap<String, Option<egui::TextureHandle>>,
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
    dab_status: Option<Box<sdroxide_types::DabStatus>>,
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
    /// VDL2: the log frozen where the operator stopped it, and the frame count
    /// at that moment so the panel can say how much has arrived since.
    ///
    /// The *log* rather than the whole status: a busy channel plate is unreadable
    /// — a message an operator is trying to read is pushed up the screen by the
    /// next one before they finish it — but the counters and the channel strip
    /// beside it are what say the receiver is still working, and freezing those
    /// too would look exactly like a decoder that had stopped.
    vdl2_hold: Option<(Vec<sdroxide_types::Vdl2Message>, u64)>,
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
    // ── Auto mode: the unattended 11 m sequencer (see `app::auto_mode`) ──
    //
    // The policy itself is `sdroxide_types::auto`; these are the session state
    // and the clocks it is run against. Deliberately session-only, never
    // persisted: a restart must not bring up a transmitting radio nobody asked
    // for.
    /// Whether auto mode is armed.
    auto_mode: bool,
    /// A DAB channel scan in progress: which Band III block it is sitting on,
    /// and when it arrived there. Session-only — a scan is an act, not a
    /// setting; what it *finds* goes in `state.dab.found` and is remembered.
    dab_scan: Option<crate::app::frame::DabScan>,
    /// DAB: the VU meter's motion, and the frame clock it last stepped on.
    dab_vu: crate::widgets::vu_meter::VuBallistics,
    dab_vu_at: Option<f64>,
    /// DAB: stations the operator has cleared with ✕ this session, as
    /// `(channel, service id)`. Cleared from the memory too, and kept out of it
    /// until the next SCAN — which is when a cleared station comes back.
    dab_hidden: Vec<(String, String)>,
    /// The last slideshow picture the station sent, ready to draw.
    dab_slide: Option<egui::TextureHandle>,
    /// The station's logo, as the station broadcasts it.
    dab_logo: Option<egui::TextureHandle>,
    /// The station list shows each station's programme type.
    dab_show_pty: bool,
    /// Frame time of the last input event, for the inactivity stop.
    auto_last_activity: f64,
    /// Frame time before which the tick will not queue another command — the
    /// gap between the command leaving and the engine's status coming back.
    auto_cooldown_until: f64,
    /// When the transmit watchdog tripped, for the pause before auto resumes.
    auto_resume_at: Option<f64>,
    /// Stations auto has answered this session, so a dead CQ is not picked
    /// again.
    auto_tried: std::collections::HashSet<String>,
    /// What auto mode last did, for the toggle's tooltip and the panel note.
    auto_note: String,
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
    /// Global WSPR activity per band, from wspr.live — the measured column
    /// beside the forecast. Fetched every ten minutes, native only.
    band_activity: Option<sdroxide_solar::BandActivityTable>,
    /// The in-flight activity fetch, if there is one.
    band_activity_fetch:
        Option<std::sync::mpsc::Receiver<Option<sdroxide_solar::BandActivityTable>>>,
    /// Frame clock reading after which another activity fetch is due.
    band_activity_due: f64,
    /// Global PSK Reporter activity per band — the activity modes' counterpart
    /// of `band_activity`, from the same fetch pattern.
    psk_activity: Option<sdroxide_solar::BandActivityTable>,
    /// The in-flight PSK activity fetch, if there is one.
    psk_activity_fetch:
        Option<std::sync::mpsc::Receiver<Option<sdroxide_solar::BandActivityTable>>>,
    /// Frame clock reading after which another PSK activity fetch is due.
    psk_activity_due: f64,
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
    /// The grey-line (night/twilight) overlay rendered to pixels, rebuilt at
    /// most once a minute.
    night_shade: crate::prop_map::NightShade,
    /// WSPR receptions, newest first: what this station decoded, and — when the
    /// WSPRnet download is on — who decoded this station. Capped at
    /// [`crate::app::panels::wspr::WSPR_SPOT_ROWS`]; the propagation store keeps
    /// the long view, this is only what the panel lists.
    wspr_spots: Vec<sdroxide_types::WsprSpot>,
    /// PI4 receptions, newest first — what this station decoded. Capped at
    /// [`crate::app::panels::pi4::PI4_SPOT_ROWS`]. No propagation-store
    /// counterpart, unlike `wspr_spots`: a PI4 message carries no grid
    /// square, so there is no path to fold into the map.
    pi4_spots: Vec<sdroxide_types::Pi4Spot>,
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
    /// Which stored set the last `RadioEvent::ClientSettings` came from —
    /// a profile name, or `None` for the station default. `None` also before
    /// any has arrived, which the Settings label reads as "not using any".
    client_settings_from: Option<Option<String>>,
    /// When the screen was last pushed to the profile by the **automatic**
    /// save, so a theme being dragged through its options is one write and not
    /// sixty. See `SdroxideApp::auto_save_screen`.
    client_save_last: f64,
    /// The set the server last sent, kept so the settings row can offer "back
    /// to the profile's look" without another round trip.
    client_settings_stored: Option<sdroxide_types::ClientScreen>,
    /// Control bindings the signed-in profile carries, held here because this
    /// client has not adopted them yet — see
    /// [`crate::app::frame::BindingsOffer`].
    bindings_pending: Option<crate::app::frame::BindingsOffer>,
    /// Set once a profile's bindings have been offered this session, so a
    /// client that says no is not asked again on the next reconnect. Session
    /// only, deliberately: the answer that matters is persisted (see
    /// `client_share_bindings`), not this.
    bindings_offer_asked: bool,
    /// The "do not ask me again" answers as the server last reported them for
    /// this profile, or `None` before it has (a local engine never does). Only
    /// once it is known are changes sent back — see `sync_client_acks`.
    acks_server: Option<sdroxide_types::ClientAcks>,
    /// What the last explicit save did, said where the operator pressed it. A
    /// control that saves silently is a control that cannot be trusted.
    client_settings_status: Option<String>,
    /// Set by the settings row, drained by the frame loop. The modal is drawn
    /// from `&self`, so the action cannot be performed where it is pressed;
    /// `Cell` rather than plumbing an out-parameter up two calls.
    client_settings_pending: std::cell::Cell<Option<crate::app::settings::ui_tab::ProfileAction>>,
    /// The KNOWN window's state — see [`known_calls::KnownCallsState`].
    pub(in crate::app) known_calls: known_calls::KnownCallsState,
    /// The PLAYBACK window's state — see [`playback::PlaybackUi`].
    pub(in crate::app) playback: playback::PlaybackUi,
    /// The Morse trainer window and its persisted progress.
    morse: morse::MorseState,
    /// The signal-identification guide window.
    pub(in crate::app) signal_id: signal_id::SignalIdState,
    /// The Enigma machine and solver window (a gimmick; holds no station state).
    pub(in crate::app) enigma: enigma::EnigmaState,
    /// The Winlink mail window. Holds its own view state; the mailbox itself
    /// lives engine-side and is read a page at a time.
    pub(in crate::app) mail: winlink::MailUi,
    log_edit: Option<LogEditForm>,
    // ── Network cockpit (spots / lookup / uploads) ──
    /// Latest merged network spots (DX cluster / POTA / SOTA / PSK Reporter).
    spots: Vec<Spot>,
    /// Live band-opening detections (band × continent path surges).
    band_openings: Vec<sdroxide_types::BandOpening>,
    /// Latest feed/connection status line (cluster state, feed errors).
    net_status: Option<String>,
    /// Bumped whenever `spots` or `net_status` changes, so the multi-radio
    /// shell can hand the station radio's feeds to the other tabs without
    /// comparing or cloning the list every frame.
    spots_gen: u64,
    /// The station radio's `spots_gen` this tab last took its spots from, in a
    /// multi-radio window — see [`SdroxideApp::adopt_spot_feed`].
    adopted_spots_gen: Option<u64>,
    /// Spots window open state.
    show_spots: bool,
    /// Show only spots that fall inside the current panadapter view span.
    spot_in_view_only: bool,
    /// What the SPOTS window's KML chip last did, shown beside it: the spots
    /// that were dropped because they carry no position, or that there were none
    /// to save. Session-only, and cleared by the next save.
    kml_note: Option<String>,
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
    net_rbn2_cmds: String,
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
    /// Inbox for a settings bundle chosen on the General tab (issue #356).
    /// Native only — the browser has no filesystem to pick one from, and the
    /// settings it would replace are on the engine's machine anyway.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    settings_import_inbox: crate::download::LoadInbox,
    /// What the last settings export or import did, shown beside the buttons
    /// until the dialog is closed. Not persisted: it describes an action, not a
    /// setting, and one from a previous session would be a lie about this one.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    settings_transfer_note: Option<String>,
    /// Callsigns queued for lookup, drained into commands each frame.
    pending_lookups: Vec<String>,
    /// Everything callsign lookup has resolved this session, by callsign. Kept
    /// because a JS8 station's locator usually never arrives on the air —
    /// only heartbeats and CQs carry one — so the map has nothing else to
    /// place the rest of the conversation by.
    callsign_cache: std::collections::HashMap<String, CallsignInfo>,
    /// QSO uploads queued (id, single-record ADIF, targets), drained to commands.
    pending_uploads: Vec<(u64, String, Vec<UploadTarget>)>,
    /// The last logbook-upload outcome per QSO id and target, so the 11 m panel
    /// can confirm that a logged contact reached LOG11DX (or any other online
    /// logbook) rather than only the local log. Session-only and *not* the
    /// `*_sent` flags on the record: those say a QSO is in a logbook, this says
    /// how the attempt just went, which is what an operator watching a QSO
    /// finish wants to see. Trimmed to the recent few.
    qso_upload_status: std::collections::HashMap<u64, Vec<(UploadTarget, bool)>>,
    /// The id of the logbook record the digi engine just wrote, if one landed
    /// this session. `DigiStatus` carries no QSO id, and giving it one would be
    /// a wire change across every controller, so the panel finds the badge for
    /// a completed QSO by the record the log *just* gained instead.
    last_logged_qso_id: Option<u64>,
    /// Awards dashboard open state + band filter ("" = all bands).
    show_awards: bool,
    /// Grid tracker window open state, and its pan/zoom + heard-layer toggle.
    show_grid: bool,
    grid_tracker: crate::app::grid_tracker::GridTracker,
    awards_band: String,
    /// Cached award tally, keyed by (log length, band filter).
    awards_cache: Option<(usize, String, sdroxide_types::Awards)>,
    /// Worked countries placed for the grid tracker's country mode, keyed by
    /// log length: `(lat, lon, confirmed, name)`. Resolved from the log's calls
    /// so CB countries place correctly.
    grid_countries: Option<(usize, Vec<(f64, f64, bool, &'static str)>)>,
    /// The worked squares on the tracker map, keyed by log length like
    /// `grid_countries` and deliberately **not** read from `awards_cache`: that
    /// one follows the AWARDS window's band filter, and the map has no band
    /// picker of its own, so it would hide squares with nothing on screen to say
    /// why (upstream's fix, in 3348b389).
    grid_squares: Option<(usize, Vec<(String, bool)>)>,
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
    /// The antenna rotator window's own state: the last manual target the
    /// operator asked for (drawn on the compass so the commanded bearing is
    /// visible against the hardware's reported one) and the manual entry fields
    /// a compass click or a country pick carries (azimuth, elevation).
    ///
    /// Session-only, and deliberately not the engine's — the engine owns what
    /// it is actually driving, this only remembers what the operator last chose.
    ///
    /// Native only, with the window itself: the browser has no second-window
    /// mechanism, so the rotator's manual side is a desktop tool.
    #[cfg(not(target_arch = "wasm32"))]
    rotor_target: Option<(f64, f64)>,
    #[cfg(not(target_arch = "wasm32"))]
    rotor_entry: (f64, f64),
    /// The country last chosen from the rotator window's DX lookup, kept only
    /// so the drop-down can show it and the caption can name the bearing.
    #[cfg(not(target_arch = "wasm32"))]
    rotor_country: Option<String>,
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
    /// The station's CB channel plan, as the General tab's dropdown last showed
    /// it. Overwritten by every `StationConfig` announcement like the region
    /// beside it; the dropdown writes through it to [`Command::SetCbPlan`].
    cb_plan_edit: sdroxide_types::CbPlan,
    /// The station's 11 m transmit permission, as the General tab's switch last
    /// showed it. Overwritten by every `StationConfig` announcement like the CB
    /// plan beside it; the switch writes through it to
    /// [`Command::SetCbTxAllowed`].
    cb_tx_edit: bool,
    /// Set when the operator has switched 11 m transmit on without yet having
    /// confirmed the one-time warning. The confirmation window is drawn from
    /// this; the permission itself is not granted until they confirm.
    cb_tx_confirm_open: bool,
    /// Where recordings are written, resolved from `config.toml` for the
    /// General tab to show. Native only; empty in the browser, which has no
    /// engine to record with.
    recordings_dir: std::path::PathBuf,
    /// Weather fax: the chart being painted and the gallery of saved ones.
    wefax: crate::wefax::WefaxUi,
    /// Whether the out-of-band transmit warning has been acknowledged for this
    /// session. Seeded from the screen's remembered choice, so ticking the
    /// warning's "don't show this again" box keeps it away on the next launch;
    /// otherwise a dismissal lasts until the window closes, and `--oob-tx` on
    /// the next launch warns afresh.
    oob_tx_ack: bool,
    /// The ATS Mini's memory slots, from its last `memories-dump`; the
    /// settings tab's Memories section draws them. `None` until asked for.
    atsmini_memories: Option<Vec<sdroxide_types::atsmini::AtsMiniMemory>>,
    /// The ATS Mini is still stepping its band cycle toward a requested tune,
    /// so the dial on screen is ahead of the radio. Set by
    /// [`sdroxide_types::RadioEvent::AtsMiniTuning`] and shown as a small note
    /// under the frequency readout.
    atsmini_tuning: bool,
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
    /// Which station the **General** tab dials — the address as last entered,
    /// read from this machine's `config.toml` and written back as it is typed.
    /// Belongs to the screen, not to the station, so unlike `remote_access` it
    /// is editable from every native client.
    #[cfg(not(target_arch = "wasm32"))]
    remote_server: sdroxide_types::RemoteServer,
    /// What the last CONNECT from the General tab did — "connecting…" while the
    /// shell has it, then whatever came back. Shown in the dialog, because a
    /// connection that failed to open leaves no tab of its own to say so.
    #[cfg(not(target_arch = "wasm32"))]
    remote_status: Option<Result<String, String>>,
    // ── Multi-radio (see `crate::multi::MultiApp`) ──
    /// Whether this instance is the focused tab. Always true in a single-radio
    /// session. Gates the announcer and the window title: a background radio
    /// keeps decoding, but it stays quiet and leaves the title alone.
    focused: bool,
    /// Nothing to see here.
    konami: konami::Konami,
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
    /// URL and, if it answers, show it under `name`. Queued by the **General**
    /// tab's CONNECT button — the connection is made by the shell, because
    /// only it can add a tab to hold it.
    ///
    /// Native only, like the General tab section that queues it: the address it dials
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
        let mut ui_settings = load_ui_settings(storage);
        // Fold a pre-map config's panadapter undock into the `detached` map, so
        // an operator who had it open does not find it docked after the update.
        ui_settings.migrate_detached();
        // Start in SWL mode when asked: either the stored preference or this
        // run's `--swl`. The per-radio switch can still turn it off for a
        // session (which clears this seed), but the next start honours the
        // preference again. See `SdroxideApp::swl_start`.
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
        crate::theme::set_map_cities(ui_settings.map_cities);
        crate::theme::set_ui_zoom(ui_settings.ui_zoom);
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
        // This radio's interface configuration, read now rather than lazily.
        // The per-radio SWL mode lives in it (`RadioConfig::hide_tx`), and the
        // first frame already has to know whether this radio is a listener's
        // screen. Read lazily it was `None` on frame one, so a SWL radio showed
        // its transmit controls until something happened to ask for the config
        // — which is exactly the flicker a click used to fix.
        let radio_cfg = ctrl.radio_config();
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
            #[cfg(not(target_arch = "wasm32"))]
            aux_view: ViewState::default(),
            peaks: spectrum_view::PeakHold::default(),
            spec_smooth: spectrum_view::SpectrumSmooth::default(),
            error: None,
            retry_at: None,
            retry_backoff: RETRY_MIN_S,
            radio_notice: None,
            other_instance: None,
            #[cfg(not(target_arch = "wasm32"))]
            panel_undock_notice_mode: None,
            #[cfg(not(target_arch = "wasm32"))]
            tool_windows: std::collections::BTreeMap::new(),
            rx_only_nudge_dismissed: false,
            show_contest: false,
            contest: None,
            contest_pick: sdroxide_types::ContestId::CqWpx,
            contest_my_exchange: String::new(),
            contest_entry: Default::default(),
            contest_prev_digi: None,
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
            profile_name_edit: String::new(),
            profiles: Vec::new(),
            profile_apply_pending: false,
            settings_upload_tab: sdroxide_types::UploadTarget::QrzLogbook,
            ui_settings: ui_settings.clone(),
            // `swl` is the retired global switch: carried so a listener who had
            // it on is not dropped back into a transceiver's screen by the
            // move to per-radio SWL mode. The Radio tab's switch is the one
            // that persists now.
            swl_start: ui_settings.start_swl || ui_settings.swl || sdroxide_types::force_swl(),
            applied_look: (ui_settings.theme, ui_settings.button_style, ui_settings.window_style),
            classic_collapsed: false,
            applied_ui_font: ui_settings.menu_font_size,
            speech: speech::SpeechRuntime::new(load_speech_settings(storage)),
            speech_voices: Vec::new(),
            alerts: alerts::AlertRuntime::station(load_alerts_settings(storage)),
            radio_cfg,
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
            public_sdr_nets_shown: [true; sdroxide_types::PublicSdrNetwork::ALL.len()],
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
            dial_hold: dial_hold::DialHold::default(),
            show_memories: false,
            show_rds: false,
            show_drm: false,
            drm: None,
            drm_channel: sdroxide_types::DrmChannel::Msc,
            drm_const_req: None,
            drm_history: std::collections::VecDeque::new(),
            show_hd: false,
            hd: None,
            hd_history: std::collections::VecDeque::new(),
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
            hfdl_status: None,
            hfdl_map: crate::hfdl_map::HfdlMapState::default(),
            hfdl_filter: String::new(),
            show_settings: false,
            settings_scroll_top: true,
            voice: sdroxide_types::VoiceStatus::default(),
            show_voice: false,
            voice_name_edit: None,
            mode_popup_since: None,
            workspaces: crate::app::persist::load_workspaces(),
            workspace_name: String::new(),
            // Started here only where the operator has asked for it. Every
            // radio tab asks on its own startup; a few requests an hour is
            // nothing against the API's limit.
            #[cfg(not(target_arch = "wasm32"))]
            update: if ui_settings.check_for_updates {
                update::UpdateCheck::start()
            } else {
                update::UpdateCheck::default()
            },
            #[cfg(not(target_arch = "wasm32"))]
            update_dismissed: false,
            band_menu_tab: top_bar::BandMenuTab::Operate,
            band_filter: top_bar::BandFilter::default(),
            fft_popup_since: None,
            skimmer_popup_since: None,
            layers_popup_since: None,
            tone_popup_since: None,
            navtex_open: None,
            dsc_open: None,
            uvpacket_open: None,
            nr_popup_since: None,
            rec_popup_since: None,
            eq_popup_since: None,
            recording_stop_at: None,
            rec_clip: None,
            rec_gate_s: None,
            rec_gate: Default::default(),
            bw_popup_since: None,
            duplex_popup_since: None,
            rpt_tone_popup_since: None,
            band_docked: false,
            band_dock_visible: false,
            #[cfg(not(target_arch = "wasm32"))]
            band_keypad: Default::default(),
            band_dock_room: None,
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
            digi_decode_dials: Vec::new(),
            digi_status: None,
            session_ignored: ignore::Ignored::new(),
            weather_reports: Vec::new(),
            weather: Default::default(),
            text_tx: String::new(),
            qso_log: load_qso_log(storage),
            swl_log: load_swl_log(storage),
            swl_report: load_swl_report(),
            swl_report_note: None,
            swl_report_pick: std::sync::Arc::new(std::sync::Mutex::new(None)),
            show_swl: false,
            show_listen: false,
            listen_history: false,
            listen_sel: None,
            listen_comment: String::new(),
            listen_map_view: Default::default(),
            swl_edit: None,
            swl_selected: None,
            swl_antenna: String::new(),
            swl_filter: Default::default(),
            schedule: Default::default(),
            broadcast_favs: load_broadcast_favourites(storage),
            hfdl_channels: load_hfdl_channels(storage),
            recording_jobs: load_recording_jobs(storage),
            jobs: Default::default(),
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
            dab_status: None,
            ais_map: crate::ais_map::AisMapState::default(),
            ais_filter: String::new(),
            show_ais_setup: false,
            ais_sort: panels::ais::AisSort::default(),
            ais_sort_desc: true,
            vdl2_status: None,
            vdl2_filter: String::new(),
            vdl2_selected: None,
            vdl2_hold: None,
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
            atchat_draft: String::new(),
            atchat_dm_tabs: Vec::new(),
            atchat_chat_tab: None,
            atchat_show_log: false,
            atchat_dm_seen: std::collections::HashMap::new(),
            atchat_dm_read: std::collections::HashMap::new(),
            atchat_img_at: 0,
            atchat_img_cache: std::collections::HashMap::new(),
            aprs_show_traffic: false,
            aprs_filter: String::new(),
            aprs_lat_buf: String::new(),
            aprs_lon_buf: String::new(),
            digi_cfg_seeded: false,
            cw_macro_edit: false,
            text_macro_edit: false,
            cw_straight: false,
            cw_key_down: false,
            cw_contacts_sent: (false, false),
            #[cfg(all(not(target_arch = "wasm32"), target_os = "linux"))]
            cw_key: None,
            #[cfg(all(not(target_arch = "wasm32"), target_os = "linux"))]
            cw_key_error: None,
            digi_tx_hz_edit: String::new(),
            digi_preview: None,
            auto_mode: false,
            dab_scan: None,
            dab_vu: Default::default(),
            dab_vu_at: None,
            dab_hidden: Vec::new(),
            dab_slide: None,
            dab_logo: None,
            dab_show_pty: false,
            auto_last_activity: 0.0,
            auto_cooldown_until: 0.0,
            auto_resume_at: None,
            auto_tried: std::collections::HashSet::new(),
            auto_note: String::new(),
            map_view: Default::default(),
            band_conditions: None,
            band_conditions_fetch: None,
            band_conditions_due: 0.0,
            band_activity: None,
            band_activity_fetch: None,
            band_activity_due: 0.0,
            psk_activity: None,
            psk_activity_fetch: None,
            psk_activity_due: 0.0,
            daylight: true,
            daylight_at: f64::NEG_INFINITY,
            show_bands: false,
            digi_stations: Default::default(),
            prop: Default::default(),
            prop_heat: Default::default(),
            night_shade: Default::default(),
            wspr_spots: Vec::new(),
            pi4_spots: Vec::new(),
            digi_hover_ll: None,
            digi_free_text: String::new(),
            flags: Default::default(),
            show_logbook: false,
            client_settings_from: None,
            client_save_last: 0.0,
            client_settings_stored: None,
            bindings_pending: None,
            bindings_offer_asked: false,
            acks_server: None,
            client_settings_status: None,
            client_settings_pending: std::cell::Cell::new(None),
            known_calls: known_calls::KnownCallsState::default(),
            playback: playback::PlaybackUi::default(),
            morse: morse::MorseState::new(load_morse_progress(storage)),
            signal_id: signal_id::SignalIdState::default(),
            enigma: enigma::EnigmaState::default(),
            mail: winlink::MailUi::default(),
            log_edit: None,
            spots: Vec::new(),
            band_openings: Vec::new(),
            net_status: None,
            spots_gen: 0,
            adopted_spots_gen: None,
            show_spots: false,
            spot_in_view_only: false,
            kml_note: None,
            spot_search: String::new(),
            broadcast: sdroxide_types::broadcast::with_utilities(load_broadcast_stations()),
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
            net_rbn2_cmds: String::new(),
            net_log: Vec::new(),
            login_tests: std::collections::HashMap::new(),
            login_tests_pending: std::collections::HashSet::new(),
            adif_import_inbox: Arc::new(Mutex::new(None)),
            settings_import_inbox: Arc::new(Mutex::new(None)),
            settings_transfer_note: None,
            chirp_import_inbox: Arc::new(Mutex::new(None)),
            pending_lookups: Vec::new(),
            callsign_cache: Default::default(),
            pending_uploads: Vec::new(),
            qso_upload_status: std::collections::HashMap::new(),
            last_logged_qso_id: None,
            show_awards: false,
            show_grid: false,
            grid_tracker: crate::app::grid_tracker::GridTracker::default(),
            awards_band: String::new(),
            awards_cache: None,
            grid_countries: None,
            grid_squares: None,
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
            #[cfg(not(target_arch = "wasm32"))]
            rotor_target: None,
            #[cfg(not(target_arch = "wasm32"))]
            rotor_entry: (0.0, 0.0),
            #[cfg(not(target_arch = "wasm32"))]
            rotor_country: None,
            relay_status: Default::default(),
            relay_edit: Default::default(),
            relay_seeded: false,
            relay_devices: Vec::new(),
            rot_cfg_seeded: false,
            // Whatever this process is already on: the binary applies the
            // station's setting before the app is built, and a remote client
            // starts on the default until the station says otherwise.
            region_edit: sdroxide_types::region(),
            cb_plan_edit: sdroxide_types::cb_plan(),
            cb_tx_edit: sdroxide_types::cb_tx_allowed(),
            cb_tx_confirm_open: false,
            recordings_dir: recordings_dir_for_display(),
            sat_ui: Default::default(),
            sat_sub_status: Vec::new(),
            wefax: Default::default(),
            oob_tx_ack: ui_settings.oob_tx_dismissed,
            atsmini_memories: None,
            atsmini_tuning: false,
            login: Default::default(),
            remote_access: persist::load_remote_access(),
            #[cfg(not(target_arch = "wasm32"))]
            remote_server: persist::load_remote_server(),
            #[cfg(not(target_arch = "wasm32"))]
            remote_status: None,
            focused: true,
            konami: Default::default(),
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

    /// This screen's display preferences, read-only — for the radio strip, which
    /// is drawn from `&MultiApp` and so reads the arrangement rather than
    /// borrowing the app to change it. Native-only in use (the browser never
    /// draws the chip); kept on one target so the accessors beside it are too.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn ui_settings_for_windows(&self) -> sdroxide_types::UiSettings {
        self.ui_settings.clone()
    }

    /// Put one module in or out of its own window, and remember it. The one
    /// place the arrangement is written from, so the strip's **WINDOWS** menu and
    /// a window's own **DOCK** chip cannot drift in whether they persist.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn set_module_detached(
        &mut self,
        module: sdroxide_types::DetachableModule,
        on: bool,
    ) {
        self.ui_settings.set_detached(module, on);
        crate::app::persist::persist_ui_settings(&self.ui_settings);
    }

    /// Bring every module window home — the strip's **DOCK ALL WINDOWS** and the
    /// Settings → UI button, one write.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn dock_all_windows(&mut self) {
        self.set_all_detached(false);
    }

    /// Put **every** module window out at once — the counterpart to
    /// [`Self::dock_all_windows`]. The arrangement the undocked mode was built
    /// for: the main window left as the spectrum and the decoders, with SP1, the
    /// operating panel and the RX control each on a display of their own.
    ///
    /// A module that has nothing to show still gets no window — the shell asks
    /// each one whether it wants a window this frame (a voice mode has no
    /// operating panel) — so this sets the *preference*, and the windows that
    /// can exist do.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn undock_all_windows(&mut self) {
        self.set_all_detached(true);
    }

    /// The one place "all of them" is written, so the two buttons cannot drift
    /// in what they persist.
    #[cfg(not(target_arch = "wasm32"))]
    fn set_all_detached(&mut self, on: bool) {
        for m in sdroxide_types::DetachableModule::ALL {
            self.ui_settings.set_detached(m, on);
        }
        crate::app::persist::persist_ui_settings(&self.ui_settings);
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
                ctx.send_viewport_cmd(egui::ViewportCommand::Title(crate::window_title(&c.label)));
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

    /// Keep the operator's ctrl+plus / ctrl+minus zoom for next time
    /// (issue #425).
    ///
    /// egui owns the zoom factor and changes it on those keys without telling
    /// anybody, so it is compared with what the settings would put there: a
    /// difference is the operator zooming, and the part of it that is not the
    /// menu font size is stored. Another radio tab may already have stored it
    /// this frame, which is why a tab behind the shared value only catches up
    /// rather than writing the file again.
    pub(in crate::app) fn remember_ui_zoom(&mut self, ctx: &egui::Context) {
        let shared = crate::theme::ui_zoom();
        let zoom = ctx.zoom_factor() / crate::theme::ui_scale();
        if (zoom - shared).abs() > 1e-3 {
            crate::theme::set_ui_zoom(zoom);
            self.ui_settings.ui_zoom = crate::theme::ui_zoom();
            crate::app::persist::persist_ui_settings(&self.ui_settings);
        } else if (self.ui_settings.ui_zoom - shared).abs() > 1e-3 {
            self.ui_settings.ui_zoom = shared;
        }
    }

    /// Multi-radio: mark the logbook file as shared with other tabs.
    pub(crate) fn set_shared_log(&mut self, shared: bool) {
        self.shared_log = shared;
    }

    /// Multi-radio: the network spots and feed status this tab holds, and the
    /// generation they are at.
    pub(crate) fn spot_feed(&self) -> (u64, &[Spot], Option<&str>, &[sdroxide_types::BandOpening]) {
        (self.spots_gen, &self.spots, self.net_status.as_deref(), &self.band_openings)
    }

    /// Multi-radio: take the station radio's spots, feed status and openings.
    ///
    /// Only the station radio's engine runs the feeds — a DX cluster login, an
    /// RBN socket and the reporters are things a station has one of — so every
    /// other tab's engine sends none, and a spot never reached the waterfall or
    /// the SPOTS list of any radio but the first (issue #410). The openings ride
    /// with the spots for the same reason: they are computed from those feeds,
    /// so without them the OPENINGS section was empty on every tab but the
    /// station radio. `generation` is the station tab's own counter.
    pub(crate) fn adopt_spot_feed(
        &mut self,
        generation: u64,
        spots: &[Spot],
        status: Option<&str>,
        openings: &[sdroxide_types::BandOpening],
    ) {
        self.adopted_spots_gen = Some(generation);
        self.spots = spots.to_vec();
        self.net_status = status.map(str::to_string);
        self.band_openings = openings.to_vec();
    }

    /// Whether this tab is behind the station radio's spot generation `generation`.
    pub(crate) fn wants_spot_feed(&self, generation: u64) -> bool {
        self.adopted_spots_gen != Some(generation)
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

    /// The layout override in force, so the shell can reach the same tier
    /// answer this radio's own frame will — see [`crate::multi`], which has to
    /// decide the split view from the size and this.
    pub(crate) fn layout_mode(&self) -> sdroxide_types::LayoutMode {
        self.ui_settings.layout
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

    /// Whether sdroxide's link to this radio is open — what the top bar's LINK
    /// chip shows. `None` when there is no switch to draw: the roster is empty,
    /// the radio is at the far end of a station that holds no switch a client
    /// may throw, or its front end is lent out as somebody's panadapter
    /// receiver — the same radios the strip offers no switch for.
    ///
    /// Read from the shell's roster either way, so a radio of this machine's
    /// own and one at a station are answered the same: the shell is what knows
    /// which of the two this tab is, and it has already asked the station.
    pub(crate) fn own_link_state(&self) -> Option<bool> {
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

    /// How the connection the General tab asked for went (see
    /// [`SdroxideApp::remote_status`]). Success is reported in the dialog as well
    /// as by the new radio appearing, because the operator may well be looking
    /// at the dialog rather than at the strip behind it.
    ///
    /// Native only, like the General tab section itself.
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

    /// Warn that another sdroxide on this machine may be reading the same
    /// radio — the warning stays on the banner until dismissed.
    pub(crate) fn warn_other_instance(&mut self, text: String) {
        self.other_instance = Some(text);
    }

    /// The banner's Dismiss: one banner shows at a time, so one press takes
    /// the one on screen — the engine's notice first, then this warning.
    pub(in crate::app) fn dismiss_notice(&mut self) {
        if self.radio_notice.take().is_none() {
            self.other_instance = None;
        }
    }

    /// Whether transmit controls are hidden for the radio on screen.
    ///
    /// Three sources, in the order they override: `--swl` for the run, the
    /// session's start-in-SWL seed, and this radio's own SWL switch. The
    /// per-radio switch is the persisted one (Settings → Radio); the other two
    /// only ever turn it *on*. See `SdroxideApp::swl_start`.
    pub(in crate::app) fn swl_mode(&self) -> bool {
        sdroxide_types::force_swl()
            || self.swl_start
            || self.radio_cfg.as_ref().is_some_and(|c| c.hide_tx)
    }

    /// The station's own location, from the digi grid locator — what a beam
    /// bearing is measured from. `None` when no locator is set.
    pub(in crate::app) fn home_latlon(&self) -> Option<(f64, f64)> {
        self.digi_status
            .as_ref()
            .map(|s| s.config.my_grid.clone())
            .filter(|g| !g.is_empty())
            .and_then(|g| sdroxide_types::grid_to_latlon(&g))
    }

    /// Aim the antenna at a location: the bearing from the station's own grid,
    /// if a rotator is configured and a locator is set. Shared by the map's
    /// click-to-point and a decode row's BEAM chip, so the two cannot compute
    /// the bearing differently.
    ///
    /// Nothing happens without a rotator — and deliberately so: this is called
    /// from a map, where there is no control to be misled by, while the rotator
    /// window is the place that says a rotator is not configured.
    pub(in crate::app) fn beam_at(
        &self,
        lat: f64,
        lon: f64,
        cmds: &mut Vec<sdroxide_types::Command>,
    ) {
        if !self.rot_cfg_edit.enabled {
            return;
        }
        let Some(home) = self.home_latlon() else { return };
        let az = sdroxide_types::bearing_deg(home, (lat, lon));
        cmds.push(sdroxide_types::Command::PointRotator { az, el: 0.0 });
    }

    /// Whether this radio's screen is the listener's: SWL mode, or a radio that
    /// cannot transmit at all (an SDR dongle, a public SDR, the ATS Mini). The
    /// receive-tone equalizer is a listener's control and is offered there, and
    /// the LOG chip opens the reception log there rather than the QSO logbook.
    pub(in crate::app) fn listener_screen(&self) -> bool {
        self.swl_mode() || self.caps.as_ref().is_some_and(|c| !c.is_transmit_capable())
    }

    /// Whether the active radio is an ATS Mini, whose dial can lag the one on
    /// screen because it tunes by stepping its own band cycle.
    pub(in crate::app) fn atsmini_active(&self) -> bool {
        self.radio_cfg.as_ref().is_some_and(|c| c.backend == sdroxide_types::Backend::AtsMini)
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
        !self.swl_mode() && self.caps.as_ref().is_some_and(|c| c.is_transmit_capable())
    }

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

    /// The frequency actually being worked, which in CW and the digital modes
    /// is not the dial: the receiver's cursor sits an audio offset away from
    /// it, and both ends of the contact are on *that* frequency.
    ///
    /// The number to log and to quote on the air. A CW dial reads a
    /// sidetone-pitch (700 Hz, usually) below the signal being copied and
    /// keyed; RTTY reads a tone pair — a couple of kilohertz — below its mark.
    /// See [`Mode::on_air_hz`](sdroxide_types::Mode::on_air_hz).
    pub(in crate::app) fn on_air_freq_hz(&self) -> f64 {
        self.state.rx[0].mode.on_air_hz(self.state.rx_freq_hz(), self.digi_audio_hz())
    }

    /// The receiver's audio cursor in Hz — the digital tone offset, or the CW
    /// pitch, as [`DigiStatus::audio_hz`](sdroxide_types::DigiStatus::audio_hz)
    /// reports it. `0.0` before the digi engine has said anything, which is the
    /// honest answer: no cursor is known, and a default invented here would put
    /// an offset on the readout that nothing measured.
    pub(in crate::app) fn digi_audio_hz(&self) -> f32 {
        self.digi_status.as_ref().map_or(0.0, |s| s.audio_hz)
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
mod touch_lock_tests;
#[cfg(test)]
mod tests {
    use super::*;
    use eframe::App;
    use sdroxide_types::{Command, RadioEvent, Spot, SpotKind};

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

    /// A CW spot is tuned the **operator's own** sidetone pitch below its
    /// signal, so it lands on the CW cursor — not the mode's 700 Hz default.
    ///
    /// Thieu (VERON, contest op), 2.0.9.x: *"when I select a CW spot the tuning
    /// is 600 Hz (my pitch) off; in the upper WIDE window it is right"*. The
    /// **The received-audio stream is reachable on an SDR too.**
    ///
    /// Reported by the operator, who went to set it as instructed and found no
    /// such row: **"This radio's audio section"** was gated on
    /// `sound_card_roles`, which answers `None` for every SDR in the list —
    /// SdrPlay, RTL-SDR, Soapy, Pluto and the rest, which is most of them — so
    /// the section never drew and the setting could not be reached at all. The
    /// setting existed, the engine honoured it, and nothing on screen could
    /// turn it on.
    ///
    /// **Why no test caught it, which is the part worth keeping.** The settings
    /// probe draws a **sound-card radio on purpose** — *"so the pickers and the
    /// typed field are in the picture"* — and a probe that only ever looks at
    /// one kind of radio cannot see what the other kind is missing. That is the
    /// same shape as a `EngineConfig::default()` harness reading like "nothing
    /// special" while actually being one of two configurations.
    ///
    /// Verified failing before the fix: with `Backend::SdrPlay` the row was
    /// absent, and with `Backend::Cat` it was there.
    #[test]
    fn the_audio_stream_row_is_reachable_on_every_backend() {
        use sdroxide_types::Backend;
        let _guard = crate::multi::frame_test_lock();
        let dir = std::env::temp_dir().join(format!("sdroxide-udprow-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) };

        for backend in [Backend::SdrPlay, Backend::Cat, Backend::RtlSdr, Backend::AtsMini] {
            let controller: Box<dyn RadioController> = Box::new(RecordingController::default());
            let ctx = egui::Context::default();
            let app = SdroxideApp::new_tab(&ctx, None, None, controller, 0, true);
            let mut radio_edit =
                Some(sdroxide_types::RadioConfig { backend, ..Default::default() });
            let mut audio_pick = None;
            let mut routing_pick = None;
            let mut rescan = false;
            let mut choose = false;
            let mut reset = false;

            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(900.0, 2400.0),
                )),
                ..Default::default()
            };
            let mut texts: Vec<String> = Vec::new();
            for _ in 0..3 {
                let out = ctx.run_ui(input.clone(), |ui| {
                    ui.allocate_ui(egui::vec2(880.0, 2400.0), |ui| {
                        app.settings_audio_tab(
                            ui,
                            &mut audio_pick,
                            &mut routing_pick,
                            &mut radio_edit,
                            false,
                            &mut rescan,
                            &mut choose,
                            &mut reset,
                        );
                    });
                });
                texts = out
                    .shapes
                    .iter()
                    .filter_map(|cs| match &cs.shape {
                        egui::Shape::Text(t) => Some(t.galley.text().to_string()),
                        _ => None,
                    })
                    .collect();
                let mut out = out;
                out.textures_delta.clear();
            }
            assert!(
                texts.iter().any(|t| t.contains("Send to")),
                "no way to set the audio stream on {backend:?} — the row is missing, so the \
                 setting cannot be reached at all"
            );
        }
    }

    /// **Nothing a tool window draws may sit in the foreground layer.**
    ///
    /// The two-window harness this repository did not have, and the regression
    /// test for the ⇱ chip (`3f877e30`).
    ///
    /// **What went wrong, because the shape of the bug is the shape of the
    /// guard.** The chip is an `egui::Area` over a tool window's title bar —
    /// egui offers no hook into its own title bar, so an overlay is the only
    /// way to reach one — and it was ordered `Order::Foreground`. That order
    /// paints above **every** window whatever order they are drawn in, so with
    /// two tool windows overlapping, the lower one's chip showed through the
    /// upper one's body while the rest of its title bar was correctly hidden.
    /// One window, half hidden: which is why reading the draw order and
    /// reading the screenshot disagreed with each other and were both right.
    ///
    /// So the property is not "the chip is under the right window", which needs
    /// a real pointer and a compositor to mean anything. It is the mechanism
    /// underneath it: **a tool window may not paint above the window stack.**
    /// That is checkable from the frame's own layer list, and it fails the
    /// moment anyone reaches for `Foreground` again.
    #[test]
    fn a_tool_window_does_not_paint_above_the_window_stack() {
        let _guard = crate::multi::frame_test_lock();
        let dir = std::env::temp_dir().join(format!("sdroxide-foreground-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) };

        let controller: Box<dyn RadioController> = Box::new(RecordingController::default());
        let ctx = egui::Context::default();
        let mut app = SdroxideApp::new_tab(&ctx, None, None, controller, 0, true);

        // Two tool windows at once, which is the reported situation: the fault
        // only shows when one can be raised over another.
        app.known_calls.show = true;
        app.weather.show = true;

        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 800.0),
            )),
            ..Default::default()
        };
        // Two frames: egui sizes a window on its first pass and places it on the
        // second, so a one-frame run tests a window that has not settled.
        for _ in 0..2 {
            let mut out = ctx.run_ui(input.clone(), |ui| {
                eframe::App::ui(&mut app, ui, &mut eframe::Frame::_new_kittest());
            });
            out.textures_delta.clear();
        }

        let above_the_stack: Vec<egui::LayerId> = ctx
            .memory(|m| m.areas().visible_layer_ids())
            .into_iter()
            .filter(|l| {
                matches!(
                    l.order,
                    egui::Order::Foreground | egui::Order::Tooltip | egui::Order::Debug
                )
            })
            .collect();
        assert!(
            above_the_stack.is_empty(),
            "a tool window is painting above the whole window stack, so it will show through \
             whatever the operator puts on top of it: {above_the_stack:?}"
        );
    }

    /// SPOTS list used the default while the skimmer overlay used his cursor.
    #[test]
    fn a_cw_spot_is_tuned_on_the_operators_own_pitch() {
        let _guard = crate::multi::frame_test_lock();
        let dir = std::env::temp_dir().join(format!("sdroxide-cwspot-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) };

        let controller: Box<dyn RadioController> = Box::new(RecordingController::default());
        let ctx = egui::Context::default();
        let mut app = SdroxideApp::new_tab(&ctx, None, None, controller, 0, true);
        // The operator copies at 600 Hz, not the mode's 700 Hz default, and the
        // engine has not reported a cursor yet.
        app.digi_status = None;
        app.digi_cfg_edit.cw_pitch_hz = 600.0;

        let spot = Spot {
            id: 1,
            kind: SpotKind::DxCluster,
            freq_hz: 14_050_000.0,
            call: "DL1ABC".into(),
            spotter: String::new(),
            mode: "CW".into(),
            comment: String::new(),
            reference: None,
            grid: None,
            loc: None,
            when_utc: 0,
            snr_db: None,
        };
        let mut cmds = Vec::new();
        app.select_spot(&spot, &mut cmds);
        let dial = cmds.iter().find_map(|c| match c {
            Command::SetVfo { hz, .. } => Some(*hz),
            _ => None,
        });
        assert_eq!(
            dial,
            Some(14_050_000.0 - 600.0),
            "a CW spot goes the operator's own pitch (600 Hz) below the signal, \
             not the 700 Hz mode default"
        );
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

    /// A hand-typed contest contact must land in the logbook, not only in the
    /// UDP fan-out.
    ///
    /// `Command::LogQso` is fire-and-forget: the engine uses it to send WSJT-X
    /// and N1MM their datagrams and does not keep a logbook, because the UI
    /// owns that. So a panel that only pushes the command reaches nobody — and
    /// that is exactly what the contest window did. A contact typed by hand was
    /// therefore absent from the local log, the session's list, the score and
    /// the Cabrillo export, while the UI happily showed an empty session. It is
    /// the same pairing the LOGBOOK window's own entry uses for the same reason
    /// (issue #341).
    #[test]
    fn a_manually_logged_contest_contact_reaches_the_logbook() {
        // An empty config directory, or the app loads the operator's real
        // logbook and the assertion below counts their 291 contacts.
        let dir = std::env::temp_dir().join(format!("sdroxide-contest-log-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) };

        let controller: Box<dyn RadioController> = Box::new(RecordingController::default());
        let ctx = egui::Context::default();
        let mut app = SdroxideApp::new_tab(&ctx, None, None, controller, 0, true);
        // A session with a serial, so the contact is also what the next serial
        // is seeded from.
        app.contest = Some(sdroxide_types::ContestSession::new(
            sdroxide_types::ContestId::Generic,
            String::new(),
            0,
        ));
        app.contest_entry.call = "19DC373".into();
        app.contest_entry.fields = vec!["12".into()];

        let mut cmds: Vec<Command> = Vec::new();
        app.log_contest_qso(&mut cmds);

        assert_eq!(app.qso_log.len(), 1, "the contact reached the logbook");
        let q = &app.qso_log[0];
        assert_eq!(q.call, "19DC373");
        assert_eq!(q.contest_id, "GENERIC", "tagged with the sponsor id");
        assert_eq!(q.srx, Some(12), "the typed exchange is read");
        // And it is still announced to the UDP loggers, on the same command.
        assert!(
            cmds.iter().any(|c| matches!(c, Command::LogQso(_))),
            "the WSJT-X/N1MM fan-out still happens"
        );
    }

    /// A controller that records nothing and answers nothing — enough to build
    /// an app over, since the test drives the logbook directly.
    #[derive(Default)]
    struct RecordingController {
        events: std::collections::VecDeque<RadioEvent>,
    }

    impl RadioController for RecordingController {
        fn send(&mut self, _cmd: Command) {}
        fn poll_event(&mut self) -> Option<RadioEvent> {
            self.events.pop_front()
        }
    }

    /// A controller whose event queue the test keeps a handle on, so engine
    /// snapshots can be fed in after the app has been built over it.
    struct SharedController(
        std::rc::Rc<std::cell::RefCell<std::collections::VecDeque<RadioEvent>>>,
    );

    impl RadioController for SharedController {
        fn send(&mut self, _cmd: Command) {}
        fn poll_event(&mut self) -> Option<RadioEvent> {
            self.0.borrow_mut().pop_front()
        }
    }

    /// A controller that queues events for the app and keeps every
    /// "do not ask me again" set the app sends.
    struct AcksController {
        events: std::rc::Rc<std::cell::RefCell<std::collections::VecDeque<RadioEvent>>>,
        sent: std::rc::Rc<std::cell::RefCell<Vec<sdroxide_types::ClientAcks>>>,
    }

    impl RadioController for AcksController {
        fn send(&mut self, _cmd: Command) {}
        fn poll_event(&mut self) -> Option<RadioEvent> {
            self.events.borrow_mut().pop_front()
        }
        fn send_client_acks(&mut self, acks: sdroxide_types::ClientAcks) {
            self.sent.borrow_mut().push(acks);
        }
    }

    /// The report: "Keep mine" and the receive-only banner came back every
    /// session, because the answers lived in the browser. The server now holds
    /// them; a client that is told them asks nothing, and a new answer goes
    /// back to the server at once — but only once the server has said what it
    /// holds, so a fresh tab cannot report "unanswered" over an answer.
    #[test]
    fn the_servers_answers_are_obeyed_and_new_ones_are_sent_back() {
        let _guard = crate::multi::frame_test_lock();
        let dir = std::env::temp_dir().join(format!("sdroxide-acks-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) };

        let events = std::rc::Rc::new(std::cell::RefCell::new(std::collections::VecDeque::new()));
        let sent = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let ctx = egui::Context::default();
        let mut app = SdroxideApp::new_tab(
            &ctx,
            None,
            None,
            Box::new(AcksController { events: events.clone(), sent: sent.clone() }),
            0,
            true,
        );

        // Nothing known from the server yet: a local change is not sent.
        app.rx_only_nudge_dismissed = true;
        app.sync_client_acks();
        assert!(sent.borrow().is_empty(), "nothing goes out before the server has spoken");
        app.rx_only_nudge_dismissed = false;

        // The server: this login said "keep mine" on this browser, and
        // dismissed the receive-only banner on radio 0.
        events.borrow_mut().push_back(RadioEvent::ClientAcks(sdroxide_types::ClientAcks {
            bindings: Some(false),
            rx_only_dismissed: vec![0],
            ..Default::default()
        }));
        // And a profile's bindings arrive anyway (an older server would send
        // them): no question is raised.
        let bindings = sdroxide_types::InputSettings::default();
        assert!(!bindings.keys.is_empty(), "a set that would otherwise be offered");
        events.borrow_mut().push_back(RadioEvent::ClientBindings { profile: None, bindings });
        app.drain_events(&ctx, 1.0);
        assert!(app.ui_settings.client_bindings_declined, "the server's answer is taken");
        assert!(app.rx_only_nudge_dismissed, "and the banner stays down on radio 0");
        assert!(app.bindings_pending.is_none(), "the bindings question is not asked");
        app.sync_client_acks();
        assert!(sent.borrow().is_empty(), "in step with the server: nothing to send");

        // A new answer — an advisory dismissed — goes straight back.
        app.ui_settings.dismissed_advisories |= 1;
        app.sync_client_acks();
        let out = sent.borrow().last().cloned().expect("the new answer is sent");
        assert_eq!(out.dismissed_advisories & 1, 1);
        assert_eq!(out.bindings, Some(false));
        app.sync_client_acks();
        assert_eq!(sent.borrow().len(), 1, "and only once");
    }

    /// Kevin's report: with CTR lit, stepping the dial with the arrow keys made
    /// the waterfall jump, flicker and shake until the keys were let go — and
    /// not at all with CTR off.
    ///
    /// A key writes the new dial into the state at once and sends `SetVfo`;
    /// CTR then recentres and sends `SetCenter`. The engine's next snapshot is
    /// often its answer to an *earlier* command, and adopting it as it stood
    /// put the old dial and the old centre back, so CTR recentred on the old
    /// dial and moved the front end back, then forward again on the next
    /// snapshot. A late snapshot must not pull either value back.
    #[test]
    fn a_late_snapshot_does_not_pull_the_dial_or_the_centre_back() {
        let _guard = crate::multi::frame_test_lock();
        let dir = std::env::temp_dir().join(format!("sdroxide-dial-hold-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) };

        let events = std::rc::Rc::new(std::cell::RefCell::new(std::collections::VecDeque::new()));
        let ctx = egui::Context::default();
        let mut app = SdroxideApp::new_tab(
            &ctx,
            None,
            None,
            Box::new(SharedController(events.clone())),
            0,
            true,
        );
        app.state.active_vfo = sdroxide_types::Vfo::A;
        let (dial, centre) = (7_100_000.0, 7_000_000.0);
        let mut engine = app.state.clone();
        (engine.vfo_a_hz, engine.center_hz) = (dial, centre);

        // One step right: the optimistic echo and the two commands, as the
        // key binding and CTR leave them.
        (app.state.vfo_a_hz, app.state.center_hz) = (dial + 100.0, centre + 100.0);
        app.dispatch_commands(vec![
            Command::SetVfo { vfo: sdroxide_types::Vfo::A, hz: dial + 100.0 },
            Command::SetCenter(centre + 100.0),
        ]);

        // The engine's answer to the command before.
        events.borrow_mut().push_back(RadioEvent::State(engine.clone()));
        app.drain_events(&ctx, 1.0);
        assert_eq!(app.state.vfo_a_hz, dial + 100.0, "a late snapshot pulled the dial back");
        assert_eq!(app.state.center_hz, centre + 100.0, "a late snapshot pulled the centre back");

        // Its answer to this one: adopted, and from here the engine's word stands.
        (engine.vfo_a_hz, engine.center_hz) = (dial + 100.0, centre + 100.0);
        events.borrow_mut().push_back(RadioEvent::State(engine.clone()));
        app.drain_events(&ctx, 1.1);
        assert_eq!((app.state.vfo_a_hz, app.state.center_hz), (dial + 100.0, centre + 100.0));
    }

    /// A DAB channel cleared with × stays cleared when a snapshot the engine
    /// sent *before* it heard the clearing arrives after it. The snapshot is
    /// adopted whole, so it put the channel back on screen, and the next
    /// settings this screen sent — a station named, a channel picked — wrote
    /// it back into the station's list for good (a report: "5A never goes").
    #[test]
    fn a_late_snapshot_does_not_bring_a_cleared_dab_channel_back() {
        let _guard = crate::multi::frame_test_lock();
        let dir = std::env::temp_dir().join(format!("sdroxide-dab-hold-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) };

        let events = std::rc::Rc::new(std::cell::RefCell::new(std::collections::VecDeque::new()));
        let ctx = egui::Context::default();
        let mut app = SdroxideApp::new_tab(
            &ctx,
            None,
            None,
            Box::new(SharedController(events.clone())),
            0,
            true,
        );
        app.state.dab.found = ["8B", "11D", "5A"].map(String::from).to_vec();
        app.state.dab.channel = "11D".into();
        let engine = app.state.clone();

        // The ×, as the channel picker does it.
        app.state.dab.forget_channel("5A");
        app.dispatch_commands(vec![Command::SetDabConfig(app.state.dab.clone())]);

        // A snapshot from before the engine applied it.
        events.borrow_mut().push_back(RadioEvent::State(engine.clone()));
        app.drain_events(&ctx, 1.0);
        assert_eq!(
            app.state.dab.found,
            ["8B", "11D"].map(String::from).to_vec(),
            "a late snapshot put the cleared channel back"
        );

        // The engine's answer to the clearing: from here its word stands.
        let mut after = engine.clone();
        after.dab.forget_channel("5A");
        events.borrow_mut().push_back(RadioEvent::State(after.clone()));
        app.drain_events(&ctx, 1.1);
        assert_eq!(app.state.dab, after.dab);

        // And a change made at the station after that is taken as it comes.
        let mut later = after.clone();
        later.dab.found.push("12C".into());
        events.borrow_mut().push_back(RadioEvent::State(later.clone()));
        app.drain_events(&ctx, 1.2);
        assert_eq!(app.state.dab, later.dab);
    }

    /// JTTY's panel offers the FREQ picker, CLEAR RX and SAVE from the start —
    /// before anything has been heard, since FREQ is how an operator gets onto
    /// a JTTY frequency at all.
    #[test]
    fn the_jtty_panel_has_freq_clear_and_save() {
        let _guard = crate::multi::frame_test_lock();
        let dir = std::env::temp_dir().join(format!("sdroxide-jtty-panel-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) };

        let ctx = egui::Context::default();
        let controller: Box<dyn RadioController> = Box::new(RecordingController::default());
        let mut app = SdroxideApp::new_tab(&ctx, None, None, controller, 0, true);
        app.state.rx[0].mode = sdroxide_types::Mode::Jtty;
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 800.0),
            )),
            ..Default::default()
        };
        let out = ctx.run_ui(input, |ui| {
            let mut cmds = Vec::new();
            app.jtty_panel(ui, &mut cmds, 300.0);
        });
        let mut texts = Vec::new();
        fn walk(s: &egui::Shape, out: &mut Vec<String>) {
            match s {
                egui::Shape::Text(t) => out.push(t.galley.text().to_string()),
                egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
                _ => {}
            }
        }
        for c in &out.shapes {
            walk(&c.shape, &mut texts);
        }
        out.drop_without_applying_deltas();
        for want in ["⇵", "CLEAR RX", "SAVE"] {
            assert!(
                texts.iter().any(|t| t.contains(want)),
                "no {want:?} in the JTTY panel: {texts:?}"
            );
        }
    }

    /// Drive one whole app frame off-screen at a phone size, to catch any
    /// panic in rendering or layout at that width.
    ///
    /// The size is in **logical points**, which is what egui lays out in and
    /// what `layout::tier_for` reads — one point is one CSS pixel in the
    /// browser, and a physical pixel count is not a layout size. Asserting the
    /// tier here is the point of the helper: a phone regression test that
    /// quietly resolves to the desktop tier guards nothing phone-shaped, and
    /// that is not a failure anyone would see.
    fn phone_frame(tag: &str, size: egui::Vec2) {
        // A whole frame writes process-global theme state — the map-cities
        // setting is published off the app's own settings on every frame — so
        // these are serialised against each other and against the shell's frame
        // tests. See [`crate::multi::FRAME_TEST_LOCK`], and the worldmap test
        // that was failing in roughly one run in four without it.
        let _guard = crate::multi::frame_test_lock();
        assert_eq!(
            crate::layout::tier_for(size, sdroxide_types::LayoutMode::Auto),
            crate::layout::Tier::Phone,
            "{tag}: {size:?} is not a phone size, so this test would not guard one"
        );
        let dir = std::env::temp_dir().join(format!("sdroxide-phone-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) };

        let controller: Box<dyn RadioController> = Box::new(RecordingController::default());
        let ctx = egui::Context::default();
        let mut app = SdroxideApp::new_tab(&ctx, None, None, controller, 0, true);

        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
            ..Default::default()
        };
        // The frame hands back a texture delta — a glyph atlas this test has no
        // backend to upload. epaint debug-asserts on a delta dropped unapplied
        // and names the remedy, which is what this is.
        ctx.run_ui(input, |ui| {
            app.ui(ui, &mut eframe::Frame::_new_kittest());
        })
        .drop_without_applying_deltas();
    }

    /// The console's band area never paints past its own window, at any width.
    ///
    /// The operator's report: *"the buttons from the band menu run off screen"*.
    /// The menu sits beside the keypad in a `horizontal_top`, and a child of a
    /// horizontal layout is offered the **whole row's** width — so the menu's
    /// wrapped rows never wrapped (`horizontal_wrapped` wraps at
    /// `available_width`, which was the full row) and the chips ran off the
    /// edge. Both the keypad and the menu are now given an allocated rect, which
    /// is a width the rows can see. Before that, this sweep read the menu out to
    /// x≈1538 in a 951 pt window.
    ///
    /// Ink, not geometry: a transparent rect is not a chip an operator can see
    /// run off the screen, and counting them reports faults that are not there.
    #[test]
    fn the_console_band_area_stays_inside_its_window() {
        use crate::multi::frame_test_lock;
        let _guard = frame_test_lock();
        let dir =
            std::env::temp_dir().join(format!("sdroxide-console-band-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) };
        fn walk(s: &egui::Shape, out: &mut Vec<(egui::Rect, bool)>) {
            match s {
                egui::Shape::Rect(r) => {
                    out.push((r.rect, r.fill.is_opaque() || r.stroke.width > 0.0));
                }
                egui::Shape::Path(q) => out.push((egui::Rect::from_points(&q.points), true)),
                egui::Shape::Vec(v) => {
                    for s in v {
                        walk(s, out);
                    }
                }
                _ => {}
            }
        }
        for w in [360.0_f32, 480.0, 600.0, 720.0, 951.0, 1280.0, 1920.0] {
            let controller: Box<dyn RadioController> = Box::new(RecordingController::default());
            let ctx = egui::Context::default();
            let mut app = SdroxideApp::new_tab(&ctx, None, None, controller, 0, true);
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(w, 900.0),
                )),
                ..Default::default()
            };
            let mut cmds: Vec<Command> = Vec::new();
            let full = ctx.run_ui(input, |ui| app.console_band_area(ui, &mut cmds));
            let mut rects = Vec::new();
            for cs in &full.shapes {
                walk(&cs.shape, &mut rects);
            }
            let worst =
                rects.iter().filter(|(_, ink)| *ink).fold(0.0_f32, |m, (r, _)| m.max(r.max.x));
            assert!(
                worst <= w + 1.0,
                "the console's band area paints to x={worst:.0} in a {w:.0} pt window"
            );
            full.drop_without_applying_deltas();
        }
    }

    /// Drive one desktop frame with the band dock requested, and hand back its
    /// shape rectangles — (rect, drawn) where drawn means an opaque fill or a
    /// stroke, i.e. ink an operator can see — plus the dock column
    /// the frame resolved to and the room the dock asked for. Shared by the
    /// #643 probe and the regression test beside it.
    fn panel_edge_frame(
        mode: sdroxide_types::Mode,
        w: f32,
        detached: bool,
    ) -> (Vec<(egui::Rect, bool)>, Option<egui::Rect>, Option<f32>) {
        let dir = std::env::temp_dir().join(format!("sdroxide-panel-edge-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) };

        let controller: Box<dyn RadioController> = Box::new(RecordingController::default());
        let ctx = egui::Context::default();
        let mut app = SdroxideApp::new_tab(&ctx, None, None, controller, 0, true);
        app.band_docked = true;
        app.band_dock_visible = true;
        app.state.rx[0].mode = mode;
        // The detached-window draw is a no-op under an embedded-viewport
        // context (the headless harness), so this drives the *main window*
        // layout: the panadapter out of the column, the panel taking it.
        app.ui_settings.set_detached(sdroxide_types::DetachableModule::Panadapter, detached);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, 1080.0))),
            ..Default::default()
        };
        let full = ctx.run_ui(input, |ui| {
            app.ui(ui, &mut eframe::Frame::_new_kittest());
        });
        let rows: Vec<(egui::Rect, bool)> = full
            .shapes
            .iter()
            .filter_map(|cs| match &cs.shape {
                egui::Shape::Rect(r) if r.rect.width() > 20.0 && r.rect.height() > 20.0 => {
                    Some((r.rect, r.fill.is_opaque() || r.stroke.width > 0.0))
                }
                _ => None,
            })
            .collect();
        let dock_room = app.band_dock_room;
        // The dock is the ink column hugging the window's right edge; the top
        // bar spans the full width but starts at 0, so the half-width floor
        // tells the two apart.
        let dock = rows
            .iter()
            .filter(|(r, _)| r.max.x >= w - 0.5 && r.min.x > w * 0.4 && r.height() > 40.0)
            .map(|(r, _)| *r)
            .min_by(|a, b| a.min.x.partial_cmp(&b.min.x).unwrap_or(std::cmp::Ordering::Equal));
        full.drop_without_applying_deltas();
        (rows, dock, dock_room)
    }

    /// Render the DAB panel to a PNG to look at: the four columns grouped on
    /// the left and the VU meter, in the theme in force, on a station that is
    /// playing. Writes `target/dab_preview.png` (or `$DAB_PREVIEW`); width from
    /// `DAB_W` (default 1600), theme from `DAB_THEME` by its `Debug` name.
    ///
    /// ```sh
    /// cargo test -p sdroxide-ui --release --lib dab_panel_preview -- --ignored --nocapture
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    #[ignore = "writes a picture to look at; not an assertion"]
    fn dab_panel_preview() {
        use crate::multi::frame_test_lock;
        use sdroxide_types::{DabKnownStation, DabService, DabStatus, Mode};
        let _guard = frame_test_lock();
        let dir = std::env::temp_dir().join(format!("sdroxide-dab-preview-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) };

        let w: f32 = std::env::var("DAB_W").ok().and_then(|v| v.parse().ok()).unwrap_or(1600.0);
        let h: f32 = std::env::var("DAB_H").ok().and_then(|v| v.parse().ok()).unwrap_or(900.0);
        let theme = std::env::var("DAB_THEME").ok().and_then(|name| {
            sdroxide_types::UiTheme::ALL.iter().find(|t| format!("{t:?}") == name).copied()
        });

        let controller: Box<dyn RadioController> = Box::new(RecordingController::default());
        let ctx = egui::Context::default();
        let mut app = SdroxideApp::new_tab(&ctx, None, None, controller, 0, true);
        if let Some(t) = theme {
            app.ui_settings.theme = t;
        }
        app.state.rx[0].mode = Mode::Dab;
        let svc = |id: &str, label: &str, br: u16| DabService {
            service_id: id.into(),
            label: label.into(),
            subchannel: Some(3),
            bitrate: Some(br),
            protection: Some("EEP 3-A".into()),
            pty: Some(
                match id {
                    "F201" => "News",
                    "F203" => "Jazz",
                    "F204" => "Pop Music",
                    _ => "Varied",
                }
                .into(),
            ),
        };
        app.state.dab.channel = "8B".into();
        app.state.dab.presets = vec![
            DabKnownStation {
                channel: "8B".into(),
                service_id: "F201".into(),
                label: "BFM BUSINESS".into(),
            },
            DabKnownStation {
                channel: "8B".into(),
                service_id: "F203".into(),
                label: "FIP".into(),
            },
        ];
        // A stand-in for a slideshow picture: a station-logo-like card.
        let load = |var: &str| {
            std::env::var(var).ok().map(|path| {
                let img = image::open(&path).expect("read the preview picture").to_rgba8();
                let (iw, ih) = img.dimensions();
                let img =
                    egui::ColorImage::from_rgba_unmultiplied([iw as usize, ih as usize], &img);
                ctx.load_texture(var, img, Default::default())
            })
        };
        app.dab_logo = load("DAB_LOGO");
        if let Some(t) = load("DAB_SLIDE") {
            app.dab_slide = Some(t);
        } else if std::env::var("DAB_NO_SLIDE").is_err() {
            let (sw, sh) = (320usize, 240usize);
            let mut px = Vec::with_capacity(sw * sh);
            for y in 0..sh {
                for x in 0..sw {
                    let (fx, fy) = (x as f32 / sw as f32, y as f32 / sh as f32);
                    let d = ((fx - 0.5).powi(2) * 1.8 + (fy - 0.45).powi(2)).sqrt();
                    let ring = d < 0.28;
                    let bar = fy > 0.78 && fy < 0.86 && fx > 0.2 && fx < 0.8;
                    let c = if ring && d > 0.2 {
                        egui::Color32::from_rgb(255, 255, 255)
                    } else if ring {
                        egui::Color32::from_rgb(230, 40, 60)
                    } else if bar {
                        egui::Color32::from_rgb(250, 200, 40)
                    } else {
                        egui::Color32::from_rgb(
                            (20.0 + 40.0 * fx) as u8,
                            (30.0 + 30.0 * fy) as u8,
                            (90.0 + 100.0 * fy) as u8,
                        )
                    };
                    px.push(c);
                }
            }
            let img = egui::ColorImage {
                size: [sw, sh],
                source_size: egui::vec2(sw as f32, sh as f32),
                pixels: px,
            };
            app.dab_slide = Some(ctx.load_texture("dab_slide_preview", img, Default::default()));
        }
        app.state.dab.service_id = "F201".into();
        app.state.dab.found = ["5C", "6A", "8B", "9D", "11C", "12B"].map(String::from).to_vec();
        app.state.dab.stations = [
            ("8B", "F201", "BFM BUSINESS"),
            ("8B", "F202", "RADIO NOVA"),
            ("8B", "F203", "FIP"),
            ("8B", "F204", "SKYROCK"),
            ("8B", "F207", "OUI FM"),
        ]
        .map(|(c, id, l)| DabKnownStation {
            channel: c.into(),
            service_id: id.into(),
            label: l.into(),
        })
        .to_vec();
        let center = sdroxide_types::DAB_BAND_III.iter().find(|(n, _)| *n == "8B").unwrap().1;
        app.dab_status = Some(Box::new(DabStatus {
            ensemble: Some("Nancy Metropole".into()),
            services: vec![
                svc("F201", "BFM BUSINESS", 96),
                svc("F202", "RADIO NOVA", 88),
                svc("F203", "FIP", 96),
                svc("F204", "SKYROCK", 80),
            ],
            window_center_hz: center,
            window_rate_hz: 2_500_000.0,
            frames: 8412,
            fibs: 33648,
            playing: Some("F201".into()),
            audio_rms_db: Some([-14.0, -17.0]),
            dls: Some(
                "BFM Business — Le journal de l'économie à 18h00 · Tél. 01 23 45 67 89".into(),
            ),
            ..Default::default()
        }));
        // A loud DAB+ station heard at the volume the preview asks for
        // (DAB_VOL, default 0.6): a recent peak held above the column.
        let vol: f32 = std::env::var("DAB_VOL").ok().and_then(|v| v.parse().ok()).unwrap_or(0.6);
        app.state.rx[0].volume = vol;
        for _ in 0..40 {
            app.dab_vu.step(Some([-9.0, -11.0]), 1.0 / 30.0);
        }
        for _ in 0..6 {
            app.dab_vu.step(Some([-13.0, -15.0]), 1.0 / 30.0);
        }

        let mut tex = crate::test_raster::Textures::default();
        let mut last = None;
        for i in 0..3 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, h))),
                time: Some(10.0 + i as f64 / 30.0),
                ..Default::default()
            };
            let mut full = ctx.run_ui(input, |ui| {
                app.ui(ui, &mut eframe::Frame::_new_kittest());
            });
            tex.apply(&full.textures_delta);
            full.textures_delta.clear();
            last = Some(full);
        }
        let img = crate::test_raster::rasterize(
            &ctx,
            last.unwrap(),
            &tex,
            egui::vec2(w, h),
            crate::theme::BG_DEEP(),
        );
        let out = std::env::var("DAB_PREVIEW").unwrap_or_else(|_| {
            format!("{}/../../target/dab_preview.png", env!("CARGO_MANIFEST_DIR"))
        });
        img.save(&out).expect("write the preview");
        println!("wrote {out}");
    }
    /// The DAB meter shows the programme, like a hi-fi's: turned down to 5 %
    /// it moves exactly as at full volume (a report: at low volume it hardly
    /// moved at all).
    #[test]
    fn the_dab_meter_moves_the_same_at_any_volume() {
        let _guard = crate::multi::frame_test_lock();
        let level_at = |volume: f32| {
            let controller: Box<dyn RadioController> = Box::new(RecordingController::default());
            let ctx = egui::Context::default();
            let mut app = SdroxideApp::new_tab(&ctx, None, None, controller, 0, true);
            app.state.rx[0].volume = volume;
            let st = sdroxide_types::DabStatus {
                playing: Some("F204".into()),
                audio_rms_db: Some([-12.0, -14.0]),
                ..Default::default()
            };
            for i in 0..20 {
                let input = egui::RawInput { time: Some(i as f64 / 30.0), ..Default::default() };
                ctx.run_ui(input, |ui| app.dab_vu_step(ui, &st)).drop_without_applying_deltas();
            }
            app.dab_vu.level
        };
        let (quiet, loud) = (level_at(0.05), level_at(1.0));
        assert_eq!(quiet, loud, "the meter at 5 % reads {quiet:?}, at full volume {loud:?}");
        assert!(quiet[0] > -10.0, "a station playing lights the column: {quiet:?}");
    }

    /// The station's logo (SPI) is not decoded, so the logo tile shows the
    /// slideshow's picture in small while the slideshow shows it large — the
    /// one picture twice, rather than "no logo yet" beside it.
    #[test]
    fn the_logo_tile_shows_the_slideshow_picture() {
        use sdroxide_types::{DabService, DabStatus, Mode};
        fn walk(s: &egui::Shape, tex: egui::TextureId, n: &mut usize, words: &mut Vec<String>) {
            match s {
                egui::Shape::Rect(r) if r.fill_texture_id() == tex => *n += 1,
                egui::Shape::Mesh(m) if m.texture_id == tex => *n += 1,
                egui::Shape::Text(t) => words.push(t.galley.text().to_string()),
                egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, tex, n, words)),
                _ => {}
            }
        }
        let _guard = crate::multi::frame_test_lock();
        let dir = std::env::temp_dir().join(format!("sdroxide-dab-logo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) };

        let controller: Box<dyn RadioController> = Box::new(RecordingController::default());
        let ctx = egui::Context::default();
        let mut app = SdroxideApp::new_tab(&ctx, None, None, controller, 0, true);
        app.state.rx[0].mode = Mode::Dab;
        app.state.dab.channel = "8D".into();
        app.state.dab.service_id = "F0E1".into();
        let center = sdroxide_types::DAB_BAND_III.iter().find(|(n, _)| *n == "8D").unwrap().1;
        app.dab_status = Some(Box::new(DabStatus {
            ensemble: Some("NANCY 8D6A".into()),
            services: vec![DabService {
                service_id: "F0E1".into(),
                label: "ICI SUD LORRAINE".into(),
                subchannel: None,
                bitrate: None,
                protection: None,
                pty: None,
            }],
            window_center_hz: center,
            window_rate_hz: 2_500_000.0,
            playing: Some("F0E1".into()),
            ..Default::default()
        }));
        let img = egui::ColorImage::new([320, 240], vec![egui::Color32::RED; 320 * 240]);
        let slide = ctx.load_texture("slide", img, Default::default());
        let tex = slide.id();
        app.dab_slide = Some(slide);

        let mut last = None;
        for i in 0..3 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(2000.0, 900.0),
                )),
                time: Some(10.0 + i as f64 / 30.0),
                ..Default::default()
            };
            last = Some(ctx.run_ui(input, |ui| {
                app.ui(ui, &mut eframe::Frame::_new_kittest());
            }));
        }
        let (mut n, mut words) = (0, Vec::new());
        for cs in &last.unwrap().shapes {
            walk(&cs.shape, tex, &mut n, &mut words);
        }
        assert_eq!(n, 2, "the picture is drawn {n} times: once in the tile, once in the slideshow");
        assert!(!words.iter().any(|w| w == "no logo yet"), "the tile still says there is no logo");
    }

    /// The operator's report (2026-10-12): a phone's WFALL tab drew the
    /// waterfall alone, with the whole screen below the readout to spare. It
    /// draws the spectrum above it now, as every other window does — read here
    /// from the spectrum's dB scale, which only the spectrum strip carries.
    #[test]
    fn a_phone_wfall_tab_draws_the_spectrum_too() {
        use crate::multi::frame_test_lock;
        use sdroxide_types::{Mode, SpectrumFrame};
        fn walk(s: &egui::Shape, out: &mut Vec<String>) {
            match s {
                egui::Shape::Text(t) => out.push(t.galley.text().to_string()),
                egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
                _ => {}
            }
        }
        let _guard = frame_test_lock();
        let controller: Box<dyn RadioController> = Box::new(RecordingController::default());
        let ctx = egui::Context::default();
        let mut app = SdroxideApp::new_tab(&ctx, None, None, controller, 0, true);
        app.state.rx[0].mode = Mode::Dab;
        app.view.digi_pane = usize::MAX; // the WFALL tab
        let mut texts = Vec::new();
        for t in 0..3 {
            let now = 10.0 + t as f64 / 30.0;
            app.frame = Some(std::sync::Arc::new(SpectrumFrame {
                seq: t,
                center_hz: app.view.view_lo_hz / 2.0 + app.view.view_hi_hz / 2.0,
                span_hz: (app.view.view_hi_hz - app.view.view_lo_hz).max(1.0),
                db_floor: -120.0,
                db_ceil: -20.0,
                bins: vec![60; 512],
                rows: Vec::new(),
                rows_clocked: false,
            }));
            app.last_spectrum_at = now;
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(390.0, 844.0),
                )),
                time: Some(now),
                ..Default::default()
            };
            let mut full = ctx.run_ui(input, |ui| {
                app.ui(ui, &mut eframe::Frame::_new_kittest());
            });
            full.textures_delta.clear();
            texts.clear();
            full.shapes.iter().for_each(|c| walk(&c.shape, &mut texts));
        }
        assert_eq!(crate::layout::tier(&ctx), crate::layout::Tier::Phone, "not a phone layout");
        assert!(
            texts.iter().any(|t| t == "-60" || t == "-80" || t == "-100"),
            "a phone's WFALL tab drew no spectrum: {texts:?}"
        );
    }

    /// A phone's WFALL tab, rendered for a person to look at: the spectrum
    /// above the waterfall, as every other window has it. Env `WF_W`/`WF_H`
    /// (default 390x844), `WF_MODE` by `Debug` name (default `Dab`), and
    /// `WF_PREVIEW` for the PNG.
    #[test]
    #[ignore]
    fn phone_wfall_preview() {
        use crate::multi::frame_test_lock;
        use sdroxide_types::{Mode, SpectrumFrame};
        let _guard = frame_test_lock();
        let dir = std::env::temp_dir().join(format!("sdroxide-wf-preview-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) };
        let w: f32 = std::env::var("WF_W").ok().and_then(|v| v.parse().ok()).unwrap_or(390.0);
        let h: f32 = std::env::var("WF_H").ok().and_then(|v| v.parse().ok()).unwrap_or(844.0);
        let mode = std::env::var("WF_MODE")
            .ok()
            .and_then(|m| Mode::ALL.iter().find(|x| format!("{x:?}") == m).copied())
            .unwrap_or(Mode::Dab);

        let controller: Box<dyn RadioController> = Box::new(RecordingController::default());
        let ctx = egui::Context::default();
        let mut app = SdroxideApp::new_tab(&ctx, None, None, controller, 0, true);
        app.state.rx[0].mode = mode;
        app.view.digi_pane = usize::MAX; // the WFALL tab
        // A DAB block: a flat plateau 1.5 MHz wide over the noise.
        let n = 1024usize;
        let row = |t: usize| -> Vec<u8> {
            (0..n)
                .map(|i| {
                    let x = i as f32 / n as f32;
                    let noise = ((i * 7919 + t * 104_729) % 23) as f32;
                    let block = if (0.2..0.8).contains(&x) { 90.0 } else { 40.0 };
                    (block + noise).min(255.0) as u8
                })
                .collect()
        };
        let mut tex = crate::test_raster::Textures::default();
        let mut last = None;
        for t in 0..40 {
            let now = 10.0 + t as f64 / 30.0;
            app.frame = Some(std::sync::Arc::new(SpectrumFrame {
                seq: t as u32,
                center_hz: app.view.view_lo_hz / 2.0 + app.view.view_hi_hz / 2.0,
                span_hz: (app.view.view_hi_hz - app.view.view_lo_hz).max(1.0),
                db_floor: -120.0,
                db_ceil: -20.0,
                bins: row(t),
                rows: (0..4).flat_map(|k| row(t * 4 + k)).collect(),
                rows_clocked: true,
            }));
            app.last_spectrum_at = now;
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, h))),
                time: Some(now),
                ..Default::default()
            };
            let mut full = ctx.run_ui(input, |ui| {
                app.ui(ui, &mut eframe::Frame::_new_kittest());
            });
            tex.apply(&full.textures_delta);
            full.textures_delta.clear();
            last = Some(full);
        }
        let img = crate::test_raster::rasterize(
            &ctx,
            last.unwrap(),
            &tex,
            egui::vec2(w, h),
            crate::theme::BG_DEEP(),
        );
        let out = std::env::var("WF_PREVIEW").unwrap_or_else(|_| {
            format!("{}/../../target/wf_preview.png", env!("CARGO_MANIFEST_DIR"))
        });
        img.save(&out).expect("write the preview");
        println!("wrote {out}");
    }

    /// **Unchecking Classic UI gives the window back.** The operator reported
    /// that it does not, and this is the frame that has to hold for it to: the
    /// strip's own boxes leave with the setting and come back with it, the fold
    /// chip is drawn only while the column is in use, and — the part that would
    /// show as "the normal window state does not return" — the picture goes back
    /// to the left edge it had.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn unchecking_classic_ui_gives_the_strip_back() {
        use crate::multi::frame_test_lock;
        let _guard = frame_test_lock();
        fn texts(shapes: &[egui::epaint::ClippedShape]) -> String {
            fn walk(s: &egui::Shape, out: &mut String) {
                match s {
                    egui::Shape::Text(t) => {
                        out.push_str(t.galley.text());
                        out.push('\n');
                    }
                    egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
                    _ => {}
                }
            }
            let mut out = String::new();
            shapes.iter().for_each(|c| walk(&c.shape, &mut out));
            out
        }
        let dir = std::env::temp_dir().join(format!("sdroxide-classicui-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) };
        let controller: Box<dyn RadioController> = Box::new(RecordingController::default());
        let ctx = egui::Context::default();
        let mut app = SdroxideApp::new_tab(&ctx, None, None, controller, 0, true);
        let frame = |app: &mut SdroxideApp| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 900.0),
                )),
                ..Default::default()
            };
            let full = ctx.run_ui(input, |ui| app.ui(ui, &mut eframe::Frame::_new_kittest()));
            // The picture's left edge: the **widest** big rectangle that is not the
            // window's own background (which starts at zero, spans the window, and
            // would answer the same in both arrangements). Printed on failure so a
            // reading that does not measure what it claims can be seen rather than
            // believed — the first version of this measured the window and said 0
            // in both states.
            let big: Vec<egui::Rect> = full
                .shapes
                .iter()
                .filter_map(|cs| match &cs.shape {
                    egui::Shape::Rect(r) => Some(r.rect),
                    _ => None,
                })
                .filter(|r| r.width() > 150.0 && r.height() > 100.0 && r.left() > 2.0)
                .collect();
            let left = big
                .iter()
                .max_by(|a, b| a.width().total_cmp(&b.width()))
                .map(|r| r.left())
                .unwrap_or(-1.0);
            (texts(&full.shapes), left, big)
        };
        // `PUBLIC SDR` is the System box's own chip. With the arrangement off it is
        // the strip's; with it on it is the column's — and the count is the point
        // either way, because drawing it in both places at once is the
        // duplication this arrangement exists to remove.
        for _ in 0..12 {
            frame(&mut app);
        }
        let (before, left_before, big_before) = frame(&mut app);
        assert!(
            left_before < 120.0,
            "the picture starts at the left edge to begin with (it starts at {left_before}; \
             the big rects were {big_before:?})"
        );
        // **`SCHEDULE` is drawn exactly once, either way.** With the arrangement
        // off it is the strip's own chip; with it on it is the column's — and if
        // both drew it, the operator would have the controls in two places at
        // once, which is the duplication this arrangement exists to remove.
        assert_eq!(
            before.matches("PUBLIC SDR").count(),
            1,
            "the strip draws the tools once to begin with; it drew: {before}"
        );
        assert!(!before.contains("CONTROLS"), "no column while the setting is off");
        app.ui_settings.classic_ui = true;
        frame(&mut app);
        let (during, left_during, big_during) = frame(&mut app);
        assert_eq!(
            during.matches("PUBLIC SDR").count(),
            1,
            "the tools moved to the column rather than being drawn in both places: {during}"
        );
        assert!(during.contains("CONTROLS"), "the column is drawn; it drew: {during}");
        assert!(
            during.contains('◀'),
            "the column carries the fold, on its own right edge; it drew: {during}"
        );
        assert!(
            !during.contains('▶'),
            "the unfold chip belongs to the folded rail, not to the open column; it drew: \
             {during}"
        );
        assert!(
            left_during > left_before + 120.0,
            "the column took the picture's left edge ({left_during} against {left_before}; \
             the big rects were {big_during:?})"
        );
        // **Folded it is a rail, not nothing** — the operator's idea: leave it a
        // few points wide so the chip that opens it stays on screen, which is
        // what lets the fold live on the column at all.
        app.classic_collapsed = true;
        frame(&mut app);
        let (folded, left_folded, big_folded) = frame(&mut app);
        assert!(
            folded.contains('▶'),
            "the folded column leaves the chip that opens it again; it drew: {folded}"
        );
        assert!(
            left_folded < 60.0,
            "the rail takes a few points, not the column's width ({left_folded}; the big \
             rects were {big_folded:?})"
        );
        assert_eq!(
            folded.matches("PUBLIC SDR").count(),
            0,
            "the folded column draws no controls at all — the rail is one chip, and the \
             section is a click away: {folded}"
        );
        app.classic_collapsed = false;
        // A fold animates open over a few frames, and a header drawn on its
        // first frame shows no body at all — so the sections are read after the
        // animation has settled, not on the frame the click produced.
        for _ in 0..8 {
            frame(&mut app);
        }
        let (_, left_unfolded, big_unfolded) = frame(&mut app);
        assert!(
            left_unfolded > left_before + 120.0,
            "the column came back at its own width, not at the rail's ({left_unfolded}; the \
             big rects were {big_unfolded:?})"
        );
        // **Every section has to be *drawn*.** A section that silently does not
        // appear is the failure this arrangement cannot afford, and in a
        // 900-point window the later ones are below the fold — so this reading
        // is taken on a window tall enough to hold all of them.
        let tall = |app: &mut SdroxideApp| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 2600.0),
                )),
                ..Default::default()
            };
            texts(&ctx.run_ui(input, |ui| app.ui(ui, &mut eframe::Frame::_new_kittest())).shapes)
        };
        for _ in 0..8 {
            tall(&mut app);
        }
        let all = tall(&mut app);
        for section in [
            "Source",
            "Band & mode",
            "Receiver",
            "Tuning",
            "Transmit",
            "Recorder",
            "Display",
            "Windows & tools",
        ] {
            // Transmit is for a radio that can transmit: a listening source
            // gets the section not at all, which is what the strip's own
            // transmit box does too.
            if section == "Transmit" && !app.tx_capable() {
                continue;
            }
            assert!(all.contains(section), "the {section} section is not drawn: {all}");
        }
        // **The operator's order, not the shipped one.** He wants the recorder
        // at the top of his own column, so the column reads the order out of the
        // settings — and the frame shows it in that order, which is what this
        // checks: the label that comes first in the drawn column is the first
        // section he placed.
        app.ui_settings.classic_sections = "recorder,source".into();
        for _ in 0..8 {
            tall(&mut app);
        }
        let ordered = tall(&mut app);
        let at =
            |label: &str| ordered.find(label).unwrap_or_else(|| panic!("{label} drawn: {ordered}"));
        assert!(
            at("Recorder") < at("Source"),
            "the column is drawn in the order he put it in: {ordered}"
        );
        assert!(
            at("Source") < at(sdroxide_types::ClassicSection::Bands.label()),
            "and a section he did not place follows the ones he did: {ordered}"
        );
        app.ui_settings.classic_sections = String::new();
        app.ui_settings.classic_ui = false;
        frame(&mut app);
        let (after, left_after, big_after) = frame(&mut app);
        assert_eq!(
            after.matches("PUBLIC SDR").count(),
            1,
            "the tools are back on the strip, once, when the setting is cleared: {after}"
        );
        assert!(
            (left_after - left_before).abs() < 1.0,
            "the picture did not go back to the left edge: {left_after} against {left_before}; \
             the big rects were {big_after:?}"
        );
        assert!(
            !after.contains('◀') && !after.contains('▶'),
            "no fold chip of any kind is drawn when Classic UI is off; it drew: {after}"
        );
        // **And the layout override must not put the arrangement out of reach.**
        // The override folds the *strip* into fewer rows; the classic strip has
        // no rows to fold, so it is drawn whatever the override says. Left to
        // the override, a `Small` setting would draw the compact strip's own
        // chips beside a column carrying the same controls.
        app.ui_settings.layout = sdroxide_types::LayoutMode::Small;
        app.ui_settings.classic_ui = true;
        if std::env::var("CLASSIC_SWL").is_ok() {
            app.ui_settings.swl = true;
        }
        if std::env::var("CLASSIC_SIMPLE").is_ok() {
            app.ui_settings.simple_ui = true;
        }
        frame(&mut app);
        let (overridden, _, _) = frame(&mut app);
        assert!(
            overridden.contains('◀'),
            "the column is drawn even with the strip folded to Small"
        );
        assert_eq!(
            overridden.matches("PUBLIC SDR").count(),
            1,
            "the compact strip's boxes are not drawn beside the column: {overridden}"
        );
    }

    /// **The column fits the width it is given.** Draws the column itself at
    /// each width it may be dragged to and asks what ink crosses the edge it was
    /// handed — the #643 shape, asked of the column rather than of the dock.
    ///
    /// It exists because the first version of the column did *not* fit and
    /// looked fine in a screenshot check: a horizontal scroll area makes
    /// `available_width` infinite, so every chip row stretched to infinity and
    /// was clipped at the viewport edge — all of them ending at exactly the
    /// same last pixel, which reads as "the chips need wrapping" rather than as
    /// "nothing was given a width". A probe that measures ink against the width
    /// is what tells those two apart.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn the_column_fits_the_width_it_is_given() {
        use crate::multi::frame_test_lock;
        let _guard = frame_test_lock();
        fn ink(shape: &egui::Shape, out: &mut Vec<egui::Rect>) {
            match shape {
                egui::Shape::Vec(v) => v.iter().for_each(|s| ink(s, out)),
                _ => {
                    let r = shape.visual_bounding_rect();
                    if r.is_finite() && r.width() > 0.0 {
                        out.push(r);
                    }
                }
            }
        }
        let dir = std::env::temp_dir().join(format!("sdroxide-classicfit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) };
        let controller: Box<dyn RadioController> = Box::new(RecordingController::default());
        let ctx = egui::Context::default();
        let mut app = SdroxideApp::new_tab(&ctx, None, None, controller, 0, true);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 900.0),
            )),
            ..Default::default()
        };
        // **A sweep, not a handful of spot widths.** The defect this test was
        // written for lived only in a band about thirty points wide (465 to 495
        // — the receive row's last chip), so three or four spot readings across
        // the whole range step straight over it. `CLASSIC_WIDTHS` overrides the
        // list for a one-off reading.
        let widths: Vec<f32> = std::env::var("CLASSIC_WIDTHS")
            .ok()
            .map(|s| s.split(',').filter_map(|v| v.trim().parse().ok()).collect())
            .unwrap_or_else(|| (0..=17).map(|i| 232.0 + i as f32 * 24.0).collect());
        for w in widths {
            let mut cmds: Vec<Command> = Vec::new();
            let mut worst: Option<(f32, egui::Rect)> = None;
            for _ in 0..3 {
                let full = ctx.run_ui(input.clone(), |ui| {
                    egui::Panel::left(egui::Id::new(("classic-fit", w as i32)))
                        .resizable(false)
                        .exact_size(w)
                        .show(ui, |ui| {
                            app.classic_column(ui, &mut cmds);
                        });
                });
                let mut all = Vec::new();
                for cs in &full.shapes {
                    ink(&cs.shape, &mut all);
                }
                // **And the content is *left*-aligned.** A `Frame`'s content
                // layout defaults to `top_down(Center)`, so the first version of
                // this column centred everything: every section's name sat in the
                // middle of its own bar and a row a few points too wide hung out
                // of both edges at once. This is the assertion that catches that
                // — it fails with "a heading is centred" the moment the layout is
                // not pinned to the left.
                let mut words: Vec<(String, egui::Rect)> = Vec::new();
                {
                    fn texts(shape: &egui::Shape, out: &mut Vec<(String, egui::Rect)>) {
                        match shape {
                            egui::Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
                            egui::Shape::Text(t) => {
                                out.push((t.galley.text().to_string(), t.visual_bounding_rect()))
                            }
                            _ => {}
                        }
                    }
                    for cs in &full.shapes {
                        texts(&cs.shape, &mut words);
                    }
                }
                if w >= 400.0 {
                    // Every control in the column is at the *left* gutter of its
                    // row: the leading heading in its bar, and a control under it.
                    // The heading is read from the shipped order rather than
                    // written out, so changing which section leads cannot leave
                    // this probe asking for one that is below the fold.
                    let first = sdroxide_types::ClassicSection::ALL[0].label();
                    for (text, what) in [(first, "a section heading"), ("LOG", "a control")] {
                        let left = words
                            .iter()
                            .find(|(s, _)| s == text)
                            .map(|(_, r)| r.left())
                            .unwrap_or(-1.0);
                        assert!(
                            left >= 0.0 && left < 70.0,
                            "the column at {w} pt draws {what} ({text}) at x {left:.0} — \
                             it belongs at the left of its row, not centred in it"
                        );
                    }
                }
                // The panel's *own* background is the one thing that legitimately
                // overhangs: egui fills a panel's frame a couple of points past
                // its edge, the same 2 pt that cost a session on the top bar
                // (discussion #16, item 1b). Ink that starts inside the column
                // and crosses it is what this is for.
                //
                // **And nothing may hang out of the *left* side either.** A row
                // wider than the column, in a centred layout, overflows both
                // edges at once and is cut at the left — which reads as missing
                // padding and is not: `Frame`'s content layout defaults to
                // `top_down(Center)`, so this column's own content was centred
                // until it said otherwise.
                let leftmost = all
                    .iter()
                    .filter(|r| r.height() < 800.0)
                    .map(|r| r.left())
                    .fold(f32::MAX, f32::min);
                assert!(
                    leftmost >= 0.0,
                    "the column at {w} pt draws {:.0} pt outside its left edge",
                    -leftmost
                );
                let cross: Vec<egui::Rect> = all
                    .iter()
                    .filter(|r| r.left() > 1.0 && r.left() < w && r.right() > w + 2.0)
                    .copied()
                    .collect();
                worst =
                    cross.iter().map(|r| (r.right() - w, *r)).max_by(|a, b| a.0.total_cmp(&b.0));
            }
            if let Some((over, r)) = worst {
                panic!(
                    "the column at {w} pt draws {over:.0} pt past its edge \
                     (x {:.0}..{:.0}, y {:.0}..{:.0})",
                    r.left(),
                    r.right(),
                    r.top(),
                    r.bottom()
                );
            }
        }
    }

    /// #643: drive one frame at a chosen window width (env `DOCK_W`, default
    /// 960) and mode (env `MODE`, by its `Debug` name) and print the largest
    /// shapes, flagging any ink past the window's right edge or across the
    /// dock's left border.
    #[test]
    fn probe_narrow_window_overflow() {
        use crate::multi::frame_test_lock;
        let _guard = frame_test_lock();

        let w: f32 = std::env::var("DOCK_W").ok().and_then(|v| v.parse().ok()).unwrap_or(960.0);
        // Any mode, by its `Debug` name, so a report naming a mode can be put
        // into the probe without editing it first.
        let want = std::env::var("MODE").ok();
        let mode = want
            .as_deref()
            .and_then(|w| sdroxide_types::Mode::ALL.iter().find(|m| format!("{m:?}") == w).copied())
            .unwrap_or(sdroxide_types::Mode::Ft8);
        let (mut rows, dock, dock_room) = panel_edge_frame(mode, w, false);
        rows.sort_by(|a, b| b.0.max.x.partial_cmp(&a.0.max.x).unwrap_or(std::cmp::Ordering::Equal));
        println!("--- #643 probe: {mode:?} at {w:.0} pt, dock_room={dock_room:?}");
        if let Some(d) = dock {
            println!(
                "  dock x {:7.1}..{:7.1} y {:7.1}..{:7.1}",
                d.min.x, d.max.x, d.min.y, d.max.y
            );
        }
        for (r, ink) in rows.iter().take(10) {
            let past_window = r.max.x > w + 1.0 || r.min.x < -1.0;
            let crosses_dock = dock.is_some_and(|d| {
                r.min.y < d.max.y
                    && r.max.y > d.min.y
                    && r.max.x > d.min.x + 0.5
                    && r.min.x < d.min.x - 0.5
            });
            let mark = if past_window {
                "   <-- PAST THE WINDOW"
            } else if crosses_dock {
                "   <-- PAST THE DOCK"
            } else {
                ""
            };
            println!(
                "  {} x {:7.1}..{:7.1} (w {:6.1})  y {:7.1}..{:7.1}{}",
                if *ink { "ink" } else { "clear" },
                r.min.x,
                r.max.x,
                r.width(),
                r.min.y,
                r.max.y,
                mark
            );
        }
    }

    /// #643: the operating panel must stop at **both** edges of its column —
    /// the window's right border and the docked Bands & Modes panel's left
    /// border. The dock is drawn first and the panel after it, so ink past the
    /// dock's edge is painted over it; that is the fault the operator's
    /// screenshots show. Ink past the window edge is Olivia's (810 pt over at
    /// 960). Only x is asserted: the y-overflow past the window's bottom edge
    /// predates the report, is clipped by the toolkit, and is not this bug.
    #[test]
    fn the_operating_panel_stays_inside_its_column() {
        use crate::multi::frame_test_lock;
        let _guard = frame_test_lock();

        for w in [960.0f32, 1280.0, 1920.0] {
            for mode in [
                sdroxide_types::Mode::Sstv,
                sdroxide_types::Mode::SstvFm,
                sdroxide_types::Mode::Rifp,
                sdroxide_types::Mode::Olivia,
                sdroxide_types::Mode::Ft8,
            ] {
                // Both with the panadapter in-window and with it detached: the
                // detached case is the undocked mode's oracle — the panel takes
                // the whole column in the main window and must still stop at
                // both edges. (The detached window itself is a no-op under the
                // embedded-viewport harness; only the main-window layout runs.)
                for detached in [false, true] {
                    let (rows, dock, _) = panel_edge_frame(mode, w, detached);
                    let ink: Vec<egui::Rect> =
                        rows.iter().filter(|(_, drawn)| *drawn).map(|(r, _)| *r).collect();
                    for r in &ink {
                        assert!(
                            r.max.x <= w + 0.5 && r.min.x >= -0.5,
                            "{mode:?} at {w:.0} pt (detached={detached}): ink x {}..{} runs past the window",
                            r.min.x,
                            r.max.x
                        );
                    }
                    if let Some(d) = dock {
                        for r in &ink {
                            let y_overlaps = r.min.y < d.max.y && r.max.y > d.min.y;
                            if y_overlaps && r.max.x > d.min.x + 0.5 && r.min.x < d.min.x - 0.5 {
                                panic!(
                                    "{mode:?} at {w:.0} pt (detached={detached}): ink x {}..{} y {}..{} crosses the dock edge at {}",
                                    r.min.x, r.max.x, r.min.y, r.max.y, d.min.x
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    /// Discussion #10: while a recording plays back, the strip above the
    /// waterfall says so — "not live" and the STOP that ends it — on a phone as
    /// on a desktop, and none of its controls is drawn past the window's edge.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn the_playback_strip_shows_and_fits() {
        use crate::multi::frame_test_lock;
        let _guard = frame_test_lock();
        let dir = std::env::temp_dir().join(format!("sdroxide-pb-strip-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) };

        let name = "sdroxide_radio2_2026-10-06_10-00-05Z_14.230000MHz_SSTV.mp3";
        for (w, h, tier) in [
            (360.0f32, 800.0f32, crate::layout::Tier::Phone),
            (1280.0, 800.0, crate::layout::Tier::Desktop),
        ] {
            let controller: Box<dyn RadioController> = Box::new(RecordingController::default());
            let ctx = egui::Context::default();
            crate::layout::set_tier(&ctx, tier);
            let mut app = SdroxideApp::new_tab(&ctx, None, None, controller, 0, true);
            app.state.rx[0].mode = sdroxide_types::Mode::Sstv;
            app.state.playback = Some(sdroxide_types::PlaybackStatus {
                name: name.into(),
                position_s: 42.0,
                duration_s: Some(117.0),
                paused: false,
                looping: true,
                ended: false,
                rate_hz: 48_000,
                channels: 2,
                stamp: sdroxide_types::playback::parse_recording_name(name),
                fault: None,
            });
            let mut texts: Vec<(String, egui::Rect)> = Vec::new();
            for _ in 0..2 {
                let input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(w, h),
                    )),
                    ..Default::default()
                };
                let out = ctx.run_ui(input, |ui| {
                    app.ui(ui, &mut eframe::Frame::_new_kittest());
                });
                texts.clear();
                fn walk(s: &egui::Shape, out: &mut Vec<(String, egui::Rect)>) {
                    match s {
                        egui::Shape::Text(t) => out.push((
                            t.galley.text().to_string(),
                            t.galley.rect.translate(t.pos.to_vec2()),
                        )),
                        egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
                        _ => {}
                    }
                }
                for c in &out.shapes {
                    walk(&c.shape, &mut texts);
                }
                out.drop_without_applying_deltas();
            }
            for want in ["▶ PLAYBACK", "not live", "■ STOP", "LOOP", "↺ AGAIN", "0:42 / 1:57"]
            {
                let hit = texts.iter().find(|(t, _)| t.contains(want));
                let Some((_, r)) = hit else {
                    panic!("{want:?} missing from the {w:.0} pt frame");
                };
                assert!(
                    r.max.x <= w + 0.5 && r.min.x >= -0.5,
                    "{want:?} drawn at x {}..{} in a {w:.0} pt window",
                    r.min.x,
                    r.max.x
                );
            }
        }
    }

    /// The split arithmetic the undocked mode rests on: a part that is undocked
    /// (or the panadapter with its layers off) reserves no height here, and the
    /// other takes the whole column. A pure counterpart to the render checks, so
    /// the rule is pinned even where a viewport cannot be drawn.
    #[test]
    fn undocking_either_part_gives_its_column_away() {
        let total = 900.0;
        let split = crate::app::frame::column_split;
        // Panadapter undocked: the panel takes it all.
        assert_eq!(split(true, true, false, total, 9.0, 0.5, 24.0), (0.0, total));
        // Panel undocked: the panadapter takes it all, and no divider is owed.
        assert_eq!(split(false, true, true, total, 9.0, 0.5, 24.0), (total, 0.0));
        // Both undocked: neither is drawn in this window.
        assert_eq!(split(true, true, true, total, 9.0, 0.5, 24.0), (0.0, 0.0));
        // Layers off is the same as the panadapter being undocked.
        assert_eq!(split(false, false, false, total, 9.0, 0.5, 24.0), (0.0, total));
        // Both present: the ordinary split, still summing to the usable height.
        let (wf, panel) = split(false, true, false, total, 9.0, 0.5, 24.0);
        assert!(wf > 0.0 && panel > 0.0 && (wf + panel - (total - 9.0)).abs() < 0.01);
    }

    /// The operator's screenshots: with the panadapter and the panel both
    /// undocked, nothing is left in the main window's centre, so the band/mode
    /// selector fills it (`band_menu_fill`) rather than a black hole. With
    /// either one in-window, the centre is not empty.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn the_centre_is_empty_only_when_both_modules_are_undocked() {
        use sdroxide_types::{DetachableModule as M, Mode};
        let dir = std::env::temp_dir().join(format!("sdroxide-centre-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) };

        let controller: Box<dyn RadioController> = Box::new(RecordingController::default());
        let ctx = egui::Context::default();
        crate::layout::set_tier(&ctx, crate::layout::Tier::Desktop);
        let mut app = SdroxideApp::new_tab(&ctx, None, None, controller, 0, true);

        // A digital mode: a panel and a panadapter, both in-window by default.
        assert!(app.center_has_content(Mode::Ft8));
        app.ui_settings.set_detached(M::Panadapter, true);
        assert!(app.center_has_content(Mode::Ft8), "the panel still fills it");
        app.ui_settings.set_detached(M::Panel, true);
        assert!(!app.center_has_content(Mode::Ft8), "both undocked leaves it empty");

        // A voice mode has no panel: the panadapter alone fills the centre,
        // until it too is undocked.
        app.ui_settings.set_detached(M::Panadapter, false);
        assert!(app.center_has_content(Mode::Am));
        app.ui_settings.set_detached(M::Panadapter, true);
        assert!(!app.center_has_content(Mode::Am));
    }

    /// The tool-window mechanism: a closed tool returns false and draws nothing;
    /// an open tool stays open and docked by default; a tool marked undocked
    /// stays undocked (the embedded-viewport harness draws no OS window, but the
    /// state must round-trip).
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn a_tool_window_round_trips_its_docked_state() {
        let dir = std::env::temp_dir().join(format!("sdroxide-toolwin-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) };
        let controller: Box<dyn RadioController> = Box::new(RecordingController::default());
        let ctx = egui::Context::default();
        crate::layout::set_tier(&ctx, crate::layout::Tier::Desktop);
        let mut app = SdroxideApp::new_tab(&ctx, None, None, controller, 0, true);

        let run = |app: &mut SdroxideApp, open: bool| -> bool {
            let mut out = false;
            let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
                out = app.tool_window(ui.ctx(), "t", "T", [300.0, 200.0], open, |_, _| {});
            });
            out
        };

        // Closed: draws nothing, returns false.
        assert!(!run(&mut app, false));
        // Open: stays open, and docked (undocked is off by default).
        assert!(run(&mut app, true));
        assert!(!app.tool_windows.get("t").is_some_and(|s| s.undocked));
        // Undocked: the state survives the frame.
        app.tool_windows.insert("t", ToolWindowState { undocked: true, window: None });
        assert!(run(&mut app, true));
        assert!(app.tool_windows.get("t").is_some_and(|s| s.undocked));
    }

    /// A whole frame with a tool open, docked and then undocked, runs without
    /// panicking and keeps the tool open — the real call site (`fn ui`) rather
    /// than `tool_window` on its own.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn an_undocked_tool_window_survives_a_whole_frame() {
        let dir = std::env::temp_dir().join(format!("sdroxide-toolframe-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) };
        let controller: Box<dyn RadioController> = Box::new(RecordingController::default());
        let ctx = egui::Context::default();
        let mut app = SdroxideApp::new_tab(&ctx, None, None, controller, 0, true);
        app.show_scanner = true;
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 800.0),
            )),
            ..Default::default()
        };
        let _ = ctx.run_ui(input, |ui| app.ui(ui, &mut eframe::Frame::_new_kittest()));
        assert!(app.show_scanner, "a docked tool stays open");
        app.tool_windows.insert("scanner", ToolWindowState { undocked: true, window: None });
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 800.0),
            )),
            ..Default::default()
        };
        let _ = ctx.run_ui(input, |ui| app.ui(ui, &mut eframe::Frame::_new_kittest()));
        assert!(app.show_scanner, "an undocked tool stays open");
        assert!(app.tool_windows.get("scanner").is_some_and(|s| s.undocked));
    }

    /// Another sdroxide on the same machine is the cause of a broken-up stream
    /// that nothing on screen used to name. The warning is drawn on the banner,
    /// survives the engine clearing its own notice (which it does with
    /// `Notice(None)`), and goes when the operator dismisses it.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn another_instance_warning_stays_on_the_banner_until_dismissed() {
        fn texts(shapes: &[egui::epaint::ClippedShape]) -> String {
            fn walk(s: &egui::Shape, out: &mut String) {
                match s {
                    egui::Shape::Text(t) => {
                        out.push_str(t.galley.text());
                        out.push('\n');
                    }
                    egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
                    _ => {}
                }
            }
            let mut out = String::new();
            shapes.iter().for_each(|c| walk(&c.shape, &mut out));
            out
        }
        let dir = std::env::temp_dir().join(format!("sdroxide-otherinst-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) };
        let controller: Box<dyn RadioController> = Box::new(RecordingController::default());
        let ctx = egui::Context::default();
        let mut app = SdroxideApp::new_tab(&ctx, None, None, controller, 0, true);
        let frame = |app: &mut SdroxideApp| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 800.0),
                )),
                ..Default::default()
            };
            texts(&ctx.run_ui(input, |ui| app.ui(ui, &mut eframe::Frame::_new_kittest())).shapes)
        };
        app.warn_other_instance("an sdroxide server is already running — PID 2488".into());
        assert!(frame(&mut app).contains("PID 2488"), "the warning is drawn");
        // What `RadioEvent::Notice(None)` does: the engine's own notice gone.
        app.radio_notice = None;
        assert!(frame(&mut app).contains("PID 2488"), "the engine does not clear it");
        // An engine notice shows first; dismissing it brings the warning back.
        app.show_notice("engine says something".into());
        assert!(frame(&mut app).contains("engine says something"));
        app.dismiss_notice();
        assert!(frame(&mut app).contains("PID 2488"), "one press takes the engine's notice");
        app.dismiss_notice();
        assert!(!frame(&mut app).contains("PID 2488"), "dismissed, it stays gone");
    }

    /// Reproduce Kevin's phone crash report (discussion #9) at the geometry
    /// Chrome Android gave: a 360 pt viewport, which crashed.
    #[test]
    fn phone_crash_regression_360x800() {
        phone_frame("360x800", egui::vec2(360.0, 800.0));
    }

    /// The same report from the real phone, whose 1440×3200 panel is 411×914
    /// logical points at DPR 3.5. It used to be driven at the raw pixel count,
    /// which is 1440 points wide and therefore the **desktop** tier — a phone
    /// test that tested nothing phone-shaped, and passed in half the time it
    /// was named for.
    #[test]
    fn phone_crash_regression_411x914() {
        phone_frame("411x914", egui::vec2(411.0, 914.0));
    }

    /// Kevin's DAB report (discussion #16): *"Scanner doesn't stop on its own —
    /// has to be stopped manually once it's done finding stations."*
    ///
    /// The sweep in `tick_dab_scan` is exhaustive by design — every Band III
    /// block, a dwell each — so the question worth answering first is whether
    /// it terminates **at all**, or whether the operator is watching a loop
    /// that never ends. This drives the tick with a clock that steps past the
    /// dwell and asserts the sweep hands back.
    #[test]
    fn the_dab_sweep_ends_by_itself() {
        let dir = std::env::temp_dir().join(format!("sdroxide-dab-scan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) };

        let controller: Box<dyn RadioController> = Box::new(RecordingController::default());
        let ctx = egui::Context::default();
        let mut app = SdroxideApp::new_tab(&ctx, None, None, controller, 0, true);
        app.dab_scan = Some(crate::app::frame::DabScan { at: 0, since: None, found: Vec::new() });

        let mut cmds = Vec::new();
        // Generous: 38 blocks at the 3 s dwell is ~114 s, and the clock here
        // steps a little over the dwell so the arithmetic cannot hide an
        // off-by-one at the end.
        let mut t = 0.0;
        let mut ticks = 0;
        while app.dab_scan.is_some() && ticks < 200 {
            t += 3.1;
            app.tick_dab_scan(&ctx, t, &mut cmds);
            ticks += 1;
        }
        assert!(
            app.dab_scan.is_none(),
            "the scan still holds a block after {ticks} ticks ({t:.0} s) — it never ends"
        );
    }

    /// The settings box opens in the middle of the screen, not tucked into the
    /// corner where the top bar covers it.
    ///
    /// It used to be a pinned `egui::Window` that put itself in the middle, and
    /// that `default_pos` was lost when it became a tool window — the very next
    /// build drew it at `x 15.4..916.6 y 15.4..776.6` in a 1920x1080 window, so
    /// its whole tab bar sat under the 187 pt top bar. The test says where the
    /// body lands rather than only that it does not panic, because a box drawn
    /// in the wrong place still draws.
    #[test]
    fn the_settings_box_opens_below_the_top_bar() {
        let _guard = crate::multi::frame_test_lock();
        let dir = std::env::temp_dir().join(format!("sdroxide-setbox-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) };
        let controller: Box<dyn RadioController> = Box::new(RecordingController::default());
        let ctx = egui::Context::default();
        let mut app = SdroxideApp::new_tab(&ctx, None, None, controller, 0, true);
        app.show_settings = true;
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1920.0, 1080.0),
            )),
            ..Default::default()
        };
        let out = ctx.run_ui(input, |ui| {
            app.ui(ui, &mut eframe::Frame::_new_kittest());
        });
        // The window's own frame is the largest inked rect near its 900x760
        // default, so take the widest that fits rather than guessing at a shape
        // type — a rounded frame is a Path, not a Rect.
        let win = out
            .shapes
            .iter()
            // Intersected with the clip rect: a shape clipped against the
            // viewport still reports its full *unclipped* bounds, and one
            // hanging off the top of the screen then reads as a 1041 pt tall
            // rect that happens to be near the window's width. This is the same
            // trap the page-overflow probe in `multi.rs` records — measure the
            // ink, not the geometry.
            .map(|cs| cs.shape.visual_bounding_rect().intersect(cs.clip_rect))
            .filter(|b| {
                b.is_finite()
                    && !b.is_negative()
                    && b.width() > 700.0
                    && b.width() < 1100.0
                    && b.height() > 500.0
            })
            .fold(None::<egui::Rect>, |acc: Option<egui::Rect>, b| {
                Some(match acc {
                    Some(a) if a.width() * a.height() >= b.width() * b.height() => a,
                    _ => b,
                })
            })
            .expect("the settings box should have painted something its own size");
        // Handed back before the assertions, so a failure here reports its own
        // message instead of aborting inside epaint's dropped-texture check.
        out.drop_without_applying_deltas();
        assert!(
            win.min.y > 150.0,
            "the settings box opened at y {:.0}, under the top bar: {:?}",
            win.min.y,
            win,
        );
        assert!(
            win.max.x <= 1920.0 && win.max.y <= 1080.0,
            "the settings box does not fit the screen: {:?}",
            win,
        );
    }

    /// The S-Meter calib on Settings → Radio: the reading the operator is
    /// shown moves by the calibration, and the squelch's own measure does not.
    /// Fails without the correction with `left: -80.0, right: -92.0`.
    #[test]
    fn the_smeter_calibration_moves_the_reading_and_not_the_squelch() {
        let _guard = crate::multi::frame_test_lock();
        let events = std::rc::Rc::new(std::cell::RefCell::new(std::collections::VecDeque::new()));
        let ctx = egui::Context::default();
        let mut app = SdroxideApp::new_tab(
            &ctx,
            None,
            None,
            Box::new(SharedController(events.clone())),
            0,
            true,
        );
        let raw = sdroxide_types::Meters {
            s_dbm: -80.0,
            pa_temp_c: None,
            adc_overload: None,
            adc_peak_dbfs: -30.0,
            adc_clip: 0.0,
            tx: None,
            stereo: false,
            tone: None,
            passband_dbfs: -50.0,
            puresignal: None,
        };
        let shown = |app: &mut SdroxideApp, cal: i8| {
            app.radio_cfg =
                Some(sdroxide_types::RadioConfig { smeter_cal: cal, ..Default::default() });
            events.borrow_mut().push_back(RadioEvent::Meters(raw));
            app.drain_events(&ctx, 1.0);
            app.meters.expect("the meters arrived")
        };

        // 0 is the meter exactly as measured: the reference.
        let m = shown(&mut app, 0);
        assert_eq!(m.s_dbm, -80.0, "no calibration, no change");

        // Two S-units down for a meter that reads high.
        let m = shown(&mut app, -2);
        assert_eq!(m.s_dbm, -92.0, "the reading is lowered by two S-units");
        assert_eq!(m.passband_dbfs, -50.0, "the squelch's measure is untouched");

        // Five up for one that reads low.
        let m = shown(&mut app, 5);
        assert_eq!(m.s_dbm, -50.0, "the reading is raised by five S-units");
        assert_eq!(m.passband_dbfs, -50.0, "the squelch's measure is untouched");
    }

    /// The S-Meter calib row is drawn on Settings → Radio, directly under
    /// "Antenna is" and in the same column — where the operator asked for it.
    #[test]
    fn the_smeter_calibration_row_sits_under_antenna_is() {
        let _guard = crate::multi::frame_test_lock();
        let dir = std::env::temp_dir().join(format!("sdroxide-smcal-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) };
        /// A controller holding a radio configuration, as a local engine
        /// does, so Settings → Radio has a radio to edit.
        struct WithConfig;
        impl RadioController for WithConfig {
            fn send(&mut self, _cmd: Command) {}
            fn poll_event(&mut self) -> Option<RadioEvent> {
                None
            }
            fn radio_config(&self) -> Option<sdroxide_types::RadioConfig> {
                Some(sdroxide_types::RadioConfig::default())
            }
        }
        let ctx = egui::Context::default();
        let mut app = SdroxideApp::new_tab(&ctx, None, None, Box::new(WithConfig), 0, true);
        app.show_settings = true;
        app.settings_tab = SettingsTab::Radio;
        let input = || egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1920.0, 1080.0),
            )),
            ..Default::default()
        };
        let mut found = (None, None, None);
        // Two frames: a window lays out on its first and paints settled on its
        // second.
        for _ in 0..2 {
            let out = ctx.run_ui(input(), |ui| {
                app.ui(ui, &mut eframe::Frame::_new_kittest());
            });
            found = (None, None, None);
            for cs in &out.shapes {
                if let egui::Shape::Text(t) = &cs.shape {
                    let r = t.visual_bounding_rect();
                    match t.galley.text() {
                        "Antenna is" => found.0 = Some(r),
                        "S-Meter calib" => found.1 = Some(r),
                        "0 (as measured)" => found.2 = Some(r),
                        _ => {}
                    }
                }
            }
            out.drop_without_applying_deltas();
        }
        let (Some(antenna), Some(calib), Some(value)) = found else {
            panic!("not drawn on Settings → Radio: {found:?}");
        };
        assert!(calib.min.y > antenna.max.y, "under Antenna is: {antenna:?} {calib:?}");
        assert!(calib.min.y - antenna.max.y < 40.0, "the next row, not further down");
        assert!((calib.min.x - antenna.min.x).abs() < 1.0, "in the same column");
        assert!(value.min.x > calib.max.x, "the picker beside it shows 0 by default");
    }

    /// Draws Settings → UI's own body at `width` points, over a few frames so
    /// the grid settles, and hands back every text with where it was painted
    /// plus the frame's shapes, for the tests and the preview below.
    fn ui_tab_at(width: f32) -> (Vec<(String, egui::Rect)>, egui::Context, egui::FullOutput) {
        let (t, c, o, _) = ui_tab_drawn(width);
        (t, c, o)
    }

    /// [`ui_tab_at`], plus every texture the frames uploaded — what
    /// [`render_png`] needs to draw the text.
    #[allow(clippy::type_complexity)]
    fn ui_tab_drawn(
        width: f32,
    ) -> (
        Vec<(String, egui::Rect)>,
        egui::Context,
        egui::FullOutput,
        Vec<(egui::TextureId, egui::epaint::ImageDelta)>,
    ) {
        let dir = std::env::temp_dir().join(format!("sdroxide-uitab-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) };
        let controller: Box<dyn RadioController> = Box::new(RecordingController::default());
        let ctx = egui::Context::default();
        // Built for its side effect: the app installs the program's own fonts
        // and style on the context, which is what the tab is drawn in.
        let app = SdroxideApp::new_tab(&ctx, None, None, controller, 0, true);
        let mut cfg = app.ui_settings;
        let mut report = sdroxide_types::SwlReportPrefs::default();
        let pick = std::sync::Arc::new(std::sync::Mutex::new(None));
        let action = std::cell::Cell::new(None);
        let input = || egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width + 40.0, 2400.0),
            )),
            ..Default::default()
        };
        let mut last = None;
        let mut textures = Vec::new();
        for _ in 0..4 {
            let out = ctx.run_ui(input(), |ui| {
                ui.allocate_ui(egui::vec2(width, 2400.0), |ui| {
                    ui.set_width(width);
                    crate::app::settings::ui_tab::settings_ui_tab(
                        ui,
                        &mut cfg,
                        None,
                        None,
                        &action,
                        None,
                        None,
                        &mut report,
                        &pick,
                    );
                });
            });
            for (id, deltas) in out.textures_delta.set.iter() {
                textures.extend(deltas.iter().map(|d| (*id, d.clone())));
            }
            if let Some(o) = last.replace(out) {
                o.drop_without_applying_deltas();
            }
        }
        let out = last.unwrap();
        let texts = out
            .shapes
            .iter()
            .filter_map(|cs| match &cs.shape {
                egui::Shape::Text(t) => {
                    Some((t.galley.text().to_string(), t.visual_bounding_rect()))
                }
                _ => None,
            })
            .collect();
        (texts, ctx, out, textures)
    }

    /// The drawn box around a text: the smallest painted shape, other than
    /// text, that holds the text's centre — a button's or a picker's frame.
    fn frame_around(out: &egui::FullOutput, text: egui::Rect) -> egui::Rect {
        out.shapes
            .iter()
            .filter(|cs| !matches!(cs.shape, egui::Shape::Text(_)))
            .map(|cs| cs.shape.visual_bounding_rect())
            .filter(|r| r.is_finite() && r.contains(text.center()) && r.area() > text.area())
            .min_by(|a, b| a.area().total_cmp(&b.area()))
            .unwrap_or(text)
    }

    /// Draws a frame's shapes into a PNG, in software: egui's own tessellation,
    /// then each triangle filled with its vertex colours and texture — the same
    /// triangles the GPU would get. For looking at a layout, not for its
    /// colours to the last bit. `crop` is in points.
    fn render_png(
        ctx: &egui::Context,
        out: &egui::FullOutput,
        textures: &[(egui::TextureId, egui::epaint::ImageDelta)],
        crop: egui::Rect,
        ppp: f32,
        path: &std::path::Path,
    ) {
        use std::collections::HashMap;
        let mut tex: HashMap<egui::TextureId, (usize, usize, Vec<egui::Color32>)> = HashMap::new();
        for (id, delta) in textures {
            let egui::ImageData::Color(img) = &delta.image;
            match delta.pos {
                None => {
                    tex.insert(*id, (img.size[0], img.size[1], img.pixels.clone()));
                }
                Some([x0, y0]) => {
                    if let Some((w, _, px)) = tex.get_mut(id) {
                        for y in 0..img.size[1] {
                            for x in 0..img.size[0] {
                                px[(y0 + y) * *w + x0 + x] = img.pixels[y * img.size[0] + x];
                            }
                        }
                    }
                }
            }
        }
        let w = (crop.width() * ppp).round() as usize;
        let h = (crop.height() * ppp).round() as usize;
        let bg = ctx.global_style().visuals.window_fill;
        let mut img = vec![[bg.r() as f32, bg.g() as f32, bg.b() as f32]; w * h];
        let prims = ctx.tessellate(out.shapes.clone(), ppp);
        for prim in prims {
            let egui::epaint::Primitive::Mesh(mesh) = prim.primitive else { continue };
            let Some((tw, th, tpx)) = tex.get(&mesh.texture_id) else { continue };
            let clip = prim.clip_rect.intersect(crop);
            let to_px =
                |p: egui::Pos2| egui::pos2((p.x - crop.min.x) * ppp, (p.y - crop.min.y) * ppp);
            let (cx0, cy0) = (to_px(clip.min).x.max(0.0), to_px(clip.min).y.max(0.0));
            let (cx1, cy1) = (to_px(clip.max).x.min(w as f32), to_px(clip.max).y.min(h as f32));
            for tri in mesh.indices.chunks_exact(3) {
                let v = [
                    mesh.vertices[tri[0] as usize],
                    mesh.vertices[tri[1] as usize],
                    mesh.vertices[tri[2] as usize],
                ];
                let p = [to_px(v[0].pos), to_px(v[1].pos), to_px(v[2].pos)];
                let area =
                    (p[1].x - p[0].x) * (p[2].y - p[0].y) - (p[2].x - p[0].x) * (p[1].y - p[0].y);
                if area.abs() < 1e-6 {
                    continue;
                }
                let x0 = p.iter().map(|q| q.x).fold(f32::MAX, f32::min).max(cx0).floor() as i32;
                let x1 = p.iter().map(|q| q.x).fold(f32::MIN, f32::max).min(cx1).ceil() as i32;
                let y0 = p.iter().map(|q| q.y).fold(f32::MAX, f32::min).max(cy0).floor() as i32;
                let y1 = p.iter().map(|q| q.y).fold(f32::MIN, f32::max).min(cy1).ceil() as i32;
                for y in y0.max(0)..y1.min(h as i32) {
                    for x in x0.max(0)..x1.min(w as i32) {
                        let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                        let w0 =
                            ((p[1].x - px) * (p[2].y - py) - (p[2].x - px) * (p[1].y - py)) / area;
                        let w1 =
                            ((p[2].x - px) * (p[0].y - py) - (p[0].x - px) * (p[2].y - py)) / area;
                        let w2 = 1.0 - w0 - w1;
                        if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                            continue;
                        }
                        let u = w0 * v[0].uv.x + w1 * v[1].uv.x + w2 * v[2].uv.x;
                        let vv = w0 * v[0].uv.y + w1 * v[1].uv.y + w2 * v[2].uv.y;
                        let tx = ((u * *tw as f32) as usize).min(tw - 1);
                        let ty = ((vv * *th as f32) as usize).min(th - 1);
                        let t = tpx[ty * tw + tx];
                        let mut c = [0.0f32; 4];
                        for (k, cv) in c.iter_mut().enumerate() {
                            let vc = w0 * v[0].color[k] as f32
                                + w1 * v[1].color[k] as f32
                                + w2 * v[2].color[k] as f32;
                            *cv = vc * t[k] as f32 / 255.0;
                        }
                        let a = c[3] / 255.0;
                        let d = &mut img[y as usize * w + x as usize];
                        for k in 0..3 {
                            d[k] = c[k] + d[k] * (1.0 - a);
                        }
                    }
                }
            }
        }
        let bytes: Vec<u8> =
            img.iter().flat_map(|p| p.iter().map(|c| c.clamp(0.0, 255.0) as u8)).collect();
        image::RgbImage::from_raw(w as u32, h as u32, bytes)
            .expect("an image of the size asked for")
            .save(path)
            .expect("the preview is written");
    }

    /// Not a check — a picture. Draws Settings → UI's colour and font rows at
    /// `SDROXIDE_PREVIEW_WIDTH` points (default 700) into the PNG at
    /// `SDROXIDE_PREVIEW_PNG`, so a layout change can be looked at before it
    /// ships.
    #[test]
    #[ignore]
    fn preview_settings_ui_colour_rows() {
        let _guard = crate::multi::frame_test_lock();
        let Ok(path) = std::env::var("SDROXIDE_PREVIEW_PNG") else { return };
        let width: f32 = std::env::var("SDROXIDE_PREVIEW_WIDTH")
            .ok()
            .and_then(|w| w.parse().ok())
            .unwrap_or(700.0);
        let (texts, ctx, out, textures) = ui_tab_drawn(width);
        let find = |name: &str| texts.iter().find(|(t, _)| t == name).map(|(_, r)| *r);
        let top = find("Spectrum background").expect("the row above the colours").min.y - 10.0;
        let bottom =
            find("Waterfall / spectrum font size").expect("the row under them").max.y + 20.0;
        let crop = egui::Rect::from_min_max(egui::pos2(0.0, top), egui::pos2(width, bottom));
        render_png(&ctx, &out, &textures, crop, 2.0, std::path::Path::new(&path));
        out.drop_without_applying_deltas();
    }

    /// Settings → UI: a colour row that wraps onto a second line — the band
    /// plan's CB swatch and its Reset — must end above the row under it, not
    /// under the Skimmer font size picker. At several widths, because where
    /// the row wraps depends on the width of the settings box.
    #[test]
    fn the_colour_rows_do_not_run_into_the_font_size_rows() {
        let _guard = crate::multi::frame_test_lock();
        for width in [700.0, 870.0, 1000.0, 1200.0] {
            let (texts, _ctx, out) = ui_tab_at(width);
            let find = |name: &str| texts.iter().find(|(t, _)| t == name).map(|(_, r)| *r);
            let skim = find("Skimmer font size").expect("the skimmer font row");
            // The band plan's swatches: CB is its last, and its Reset is the
            // last one above the font rows (the spot row has one too).
            let cb = find("CB").expect("the band plan's CB swatch label");
            let reset = texts
                .iter()
                .filter(|(t, r)| t == "Reset" && r.min.y < skim.min.y)
                .map(|(_, r)| *r)
                .max_by(|a, b| a.min.y.total_cmp(&b.min.y))
                .expect("its Reset button");
            let picker = texts
                .iter()
                .filter(|(t, r)| t == "Medium" && r.min.y >= skim.min.y - 6.0)
                .map(|(_, r)| *r)
                .next()
                .expect("the skimmer font picker");
            // Each label reads level with the first line of its swatches.
            for (label, first) in [("Spot label colours", "DX"), ("Band plan colours", "Ham")] {
                let label = find(label).expect("the colour row's label");
                let first = texts
                    .iter()
                    .filter(|(t, r)| t == first && r.min.y >= label.min.y - 20.0)
                    .map(|(_, r)| *r)
                    .min_by(|a, b| a.min.y.total_cmp(&b.min.y))
                    .expect("the row's first swatch label");
                assert!(
                    (label.center().y - first.center().y).abs() <= 1.5,
                    "at {width} pt the label (centre {:.1}) is not level with its first line \
                     of colours (centre {:.1})",
                    label.center().y,
                    first.center().y,
                );
            }
            let reset_box = frame_around(&out, reset);
            let picker_box = frame_around(&out, picker);
            let row_end = reset_box.max.y.max(cb.max.y);
            eprintln!("w {width}: reset box {reset_box:?} picker box {picker_box:?}");
            out.drop_without_applying_deltas();
            assert!(
                picker_box.min.y >= row_end + 4.0,
                "at {width} pt the Skimmer font size picker (top {:.1}) runs into the band-plan \
                 row's CB / Reset (bottom {:.1})",
                picker_box.min.y,
                row_end,
            );
        }
    }
}
