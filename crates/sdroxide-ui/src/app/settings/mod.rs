//! The Settings dialog.
//!
//! The window closure borrows `&self`, so it cannot reach `&mut self.ctrl`;
//! every edit is written into a [`SettingsIo`] and applied by
//! [`SdroxideApp::settings_window`] after the closure returns. That is also
//! where the blocking operations live — an HPSDR scan, a TCI connection test —
//! so neither runs inside a layout pass.
//!
//! Tabs that configure the *station* rather than this screen — Spots, FreeDV,
//! Uploads, Servers, TLE — are seeded from `RadioEvent::StationConfig` and stay
//! disabled until it arrives. They edit files on the engine host, which may not
//! be this machine, so there is nothing here to read them from and applying an
//! unseeded copy would write defaults over the real thing.
//!
//! [`SdroxideApp::settings_body`] draws the tab strip and dispatches to one
//! submodule per tab.

pub(in crate::app) mod controls;
pub(in crate::app) mod general;
pub(in crate::app) mod net;
pub(in crate::app) mod radio;
pub(in crate::app) mod relay;
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) mod remote;
pub(in crate::app) mod servers;
pub(in crate::app) mod tle;
pub(in crate::app) mod ui_tab;

use eframe::egui::{self, Color32, ComboBox, RichText};
use sdroxide_types::{Command, LoginTarget, LookupProvider, NetworkConfig, UploadTarget};

use self::controls::settings_controls_tab;
use self::general::{device_combo, region_combo, remote_access_settings};
use self::net::{
    broadcast_stations_settings, net_heading, net_row, net_secret, operator_identity_note,
    settings_freedv_tab,
};
use self::radio::{
    settings_airspy_tab, settings_airspyhf_tab, settings_cat_tab, settings_elad_tab,
    settings_fobos_tab, settings_hackrf_tab, settings_hpsdr_tab, settings_hydrasdr_tab,
    settings_icomnet_tab, settings_kiwisdr_tab, settings_lime_tab, settings_pluto_tab,
    settings_rtlsdr_tab, settings_rtltcp_tab, settings_rx888_tab, settings_sdrplay_tab,
    settings_smartsdr_tab, settings_soapy_devices, settings_soapy_tab, settings_spyserver_tab,
    settings_tci_tab,
};
#[cfg(not(target_arch = "wasm32"))]
use self::remote::settings_remote_tab;
use self::servers::{
    settings_rigctld_tab, settings_rotator_tab, settings_tci_server_tab, settings_wsjtx_tab,
};
use self::tle::settings_tle_tab;
use self::ui_tab::settings_ui_tab;
use crate::app::SdroxideApp;
use crate::app::persist::{persist_speech_settings, persist_ui_settings};
use crate::chrome::StyledCombo;
use crate::theme::ThemedScroll as _;

/// Settings dialog tabs: General (station identity + audio devices), the radio
/// interface and its settings, display/UI preferences and spoken
/// announcements, control inputs
/// (keyboard/mouse bindings), the network cockpit (spot feeds + uploads), the
/// built-in TCI server, and — the other direction — the sdroxide server this
/// screen connects *out* to.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum SettingsTab {
    General,
    Radio,
    Ui,
    Controls,
    Spots,
    FreeDv,
    Uploads,
    Winlink,
    Servers,
    /// The external transmit/receive switch: the relay that grounds the SDR's
    /// antenna while the station transmits.
    ///
    /// A tab of its own rather than another section under `Servers`, where the
    /// rotator lives. Two reasons: the panel is large — a channel table, the
    /// sequencer's timings and a test button apiece — and this is the one page
    /// in here whose being hard to find could cost somebody a front end.
    TrSwitch,
    /// Dial another station. Native only: a browser client is already attached
    /// to the server that served it and has nowhere to put a second one.
    #[cfg(not(target_arch = "wasm32"))]
    Remote,
    Tle,
}

/// A "Test connection" button's state: still asking, or an answer.
///
/// The waiting half exists because the answer may come from another machine.
/// Where the radio is local the test blocks for its couple of seconds and the
/// result is simply there afterwards; over a session the press, the connect and
/// the reply are three separate moments, and a button that showed nothing in
/// between would be pressed again.
#[derive(Clone, PartialEq)]
pub(in crate::app) enum TestOutcome {
    Waiting,
    Done(Result<String, String>),
}

/// How long a device question may stay unanswered before the controls that ask
/// are usable again. Only reached when something went wrong — the server
/// dropped the request under load, or the session went away mid-scan — so it is
/// set well past the slowest of them (a five-second connection test).
const PROBE_TIMEOUT_S: f64 = 20.0;

/// Transient state of the TLE tab — what is in the paste box, which rows are
/// unfolded. Not persisted: none of it is a setting, it is where the operator
/// happens to be in the dialog.
#[derive(Default)]
pub(in crate::app) struct SatEditState {
    /// The "paste element sets here" box.
    paste: String,
    /// Catalogue number and name for a new frequency entry.
    new_freq_id: String,
    new_freq_name: String,
    /// Index of the pasted element set whose two lines are shown for editing.
    open_tle: Option<usize>,
    /// Index of the frequency entry whose links are shown for editing.
    open_freq: Option<usize>,
    /// What the last add attempt did, good or bad, so a paste that yielded
    /// nothing says so instead of appearing to have been ignored.
    note: String,
    /// Whether UPDATE NOW is waiting on the engine. The fetch happens over
    /// there — one HTTPS round trip per subscription — so the answer arrives as
    /// an event rather than as a return value, and this is what tells the
    /// arriving status it was asked for.
    fetching: bool,
}

/// Everything the settings dialog can change, collected in one place.
///
/// The window closure borrows `&self`, so `settings_body` can't reach
/// `&mut self.ctrl` — edits are written here and applied by `settings_window`
/// after the closure returns.
pub(in crate::app) struct SettingsIo<'a> {
    iface_opts: &'a [sdroxide_types::Backend],
    radio_edit: &'a mut Option<sdroxide_types::RadioConfig>,
    /// Whether the controls that ask a question about a *machine* — which
    /// dongles are on the USB bus, which serial ports exist, what a Discover
    /// broadcast finds, whether an address answers — can be used.
    ///
    /// They are asked of the machine the radio is attached to rather than
    /// answered locally ([`sdroxide_types::DeviceProbe`]), so they work from a
    /// remote or browser client too and describe the right computer. False only
    /// where that machine has said it does not answer them, or while one is
    /// still in flight — a press that crossed the network is visibly in
    /// progress rather than looking like it did nothing.
    can_probe: bool,
    /// Whether the engine is in this process, so this dialog is editing files
    /// on the machine in front of the operator. Wording only: what is
    /// *changeable* is decided by `can_probe` above and by whether the config
    /// has arrived, but a note that says "saved on the machine the radio is
    /// attached to" is only worth making when that is somewhere else.
    local_engine: bool,
    /// Converter offset in Hz, buffered until Apply — see
    /// `SdroxideApp::converter_edit_hz`.
    converter_hz: &'a mut Option<f64>,
    /// The RX and TX tuning ranges as typed, buffered until Apply — see
    /// `SdroxideApp::range_edit`.
    ranges: &'a mut Option<(String, String)>,
    /// Where the antenna is, buffered until Apply — see
    /// `SdroxideApp::rx_site_edit`.
    rx_site: &'a mut Option<sdroxide_types::RxSite>,
    audio_pick: &'a mut Option<(bool, Option<String>)>,
    hpsdr_discover: &'a mut bool,
    /// Re-enumerate the USB bus for RTL-SDR dongles. Cheap and non-invasive —
    /// no device is opened — so it cannot disturb a running stream.
    rtlsdr_rescan: &'a mut bool,
    rx888_rescan: &'a mut bool,
    /// Re-enumerate the USB bus for Airspy HF+ receivers. Opens nothing.
    airspyhf_rescan: &'a mut bool,
    /// Copy the last Airspy HF+ session's trace to the clipboard.
    airspyhf_copy_report: &'a mut bool,
    hackrf_rescan: &'a mut bool,
    hackrf_copy_report: &'a mut bool,
    airspy_rescan: &'a mut bool,
    airspy_copy_report: &'a mut bool,
    /// Re-enumerate the USB bus for HydraSDR RFOne receivers. Opens nothing.
    hydrasdr_rescan: &'a mut bool,
    /// Copy the last HydraSDR session's trace to the clipboard.
    hydrasdr_copy_report: &'a mut bool,
    /// Re-enumerate the USB bus for ELAD devices. Opens nothing.
    elad_rescan: &'a mut bool,
    /// Copy the last ELAD session's trace to the clipboard.
    elad_copy_report: &'a mut bool,
    /// Ask LimeSuite for its device list. Not free, unlike the `nusb` scans:
    /// it opens each candidate to read its identity, so it can disturb a board
    /// another program is holding.
    lime_rescan: &'a mut bool,
    /// Copy the last LimeSDR session's trace to the clipboard.
    lime_copy_report: &'a mut bool,
    /// Ask the SDRplay API service for its device list. Brief and
    /// non-invasive, so it cannot disturb a running stream.
    sdrplay_rescan: &'a mut bool,
    /// Re-enumerate the USB bus for Fobos SDRs. Opens nothing.
    fobos_rescan: &'a mut bool,
    /// Re-run the SoapySDR enumeration. Opens nothing, but loads every
    /// installed module and asks each to scan, so it is not instant.
    soapy_rescan: &'a mut bool,
    tci_test: &'a mut bool,
    /// Connect to the Icom, report what it is, and disconnect (blocking).
    icomnet_test: &'a mut bool,
    spyserver_test: &'a mut bool,
    kiwi_test: &'a mut bool,
    /// Copy the last Icom LAN session's trace to the clipboard.
    icomnet_copy_report: &'a mut bool,
    /// Listen for FlexRadio discovery broadcasts (a couple of seconds, blocking).
    smartsdr_discover: &'a mut bool,
    smartsdr_test: &'a mut bool,
    /// Copy the last SmartSDR session's protocol trace to the clipboard.
    smartsdr_copy_report: &'a mut bool,
    /// Ask mDNS for IIO devices, and try the USB gadget's address (~1.5 s,
    /// blocking).
    pluto_discover: &'a mut bool,
    pluto_test: &'a mut bool,
    /// Copy the last PlutoSDR session's protocol trace to the clipboard.
    pluto_copy_report: &'a mut bool,
    apply_iface: &'a mut bool,
    ui_edit: &'a mut sdroxide_types::UiSettings,
    /// Who may connect to this machine's server, or `None` where this client
    /// is in no position to say — a remote one, and every browser one. Those
    /// credentials are `config.toml` on the machine the radio is attached to,
    /// and this is not it.
    access_edit: Option<&'a mut sdroxide_types::RemoteAccess>,
    /// The other direction — which station this screen dials from the Remote
    /// tab, and whether CONNECT was pressed. Editable everywhere `access_edit`
    /// is not: it is this machine's own setting, so a remote client is exactly
    /// as entitled to it as the shack machine.
    #[cfg(not(target_arch = "wasm32"))]
    remote_edit: &'a mut sdroxide_types::RemoteServer,
    #[cfg(not(target_arch = "wasm32"))]
    remote_connect: &'a mut bool,
    digi_edit: &'a mut sdroxide_types::DigiConfig,
    digi_seeded: bool,
    net_edit: &'a mut NetworkConfig,
    /// Whether the engine has said what the station's network config actually
    /// is. Until it has, the tabs that edit it are disabled: applying an
    /// unseeded copy would write defaults over the operator's real settings,
    /// and showing empty boxes would claim the station is unconfigured when it
    /// is not.
    net_seeded: bool,
    net_cmds: &'a mut String,
    rbn_cmds: &'a mut String,
    net_apply: &'a mut bool,
    net_sync: &'a mut bool,
    /// The built-in TCI *server* — this app acting as a rig for third-party
    /// clients, as opposed to the TCI client configured on the Radio tab.
    tci_srv_edit: &'a mut sdroxide_types::TciServerConfig,
    tci_srv_apply: &'a mut bool,
    /// The built-in Hamlib rigctld server — the control-only surface every
    /// "NET rigctl" client speaks.
    rigctld_edit: &'a mut sdroxide_types::RigctldConfig,
    rigctld_apply: &'a mut bool,
    /// The WSJT-X UDP broadcast — decodes, status and logged QSOs for
    /// GridTracker, JTAlert, N1MM+ and Log4OM.
    wsjtx_edit: &'a mut sdroxide_types::WsjtxConfig,
    wsjtx_apply: &'a mut bool,
    /// The rotctld *client* — the satellite lock steering a motorized antenna.
    rot_edit: &'a mut sdroxide_types::RotatorConfig,
    rot_apply: &'a mut bool,
    relay_edit: &'a mut sdroxide_types::RelayConfig,
    relay_apply: &'a mut bool,
    /// Which contact the operator asked to hear, if any.
    relay_test: &'a mut Option<u8>,
    /// Control-input bindings, plus the row (if any) waiting to capture a
    /// keypress. Persisted on close, since a rebind has no APPLY step.
    input_edit: &'a mut sdroxide_types::InputSettings,
    key_capture: &'a mut Option<usize>,
    midi_learn: &'a mut Option<crate::input::MidiLearn>,
    midi_rescan: &'a mut bool,
    /// The operator's satellite additions, and the transient state of the
    /// dialog that edits them. Sent to the engine on change, like the input
    /// bindings are written on change: there is no APPLY step to hang it off.
    sat_edit: &'a mut sdroxide_types::SatConfig,
    /// Whether the engine has said what the station tracks. Same contract as
    /// `net_seeded`, and with more at stake: this tab writes on every keystroke.
    sat_seeded: bool,
    sat_ui: &'a mut SatEditState,
    sat_subs: &'a [sdroxide_types::TleSubStatus],
    /// Re-fetch every subscription now. The engine does it, so this is a
    /// command rather than a blocking call.
    sat_sub_refresh: &'a mut bool,
    /// How the 3D view draws its cloud deck: `Some(true)` marches the volume,
    /// `Some(false)` stacks shells through it. `None` where there is no 3D view
    /// to set it for — the browser client, whose solar view is a separate tab
    /// with its own settings — because a switch that provably does nothing is
    /// worse than no switch.
    solar_cloud_march: Option<&'a mut bool>,
    /// Re-read the operator's broadcast station file, and re-download this
    /// season's schedule. Both act on files rather than on an edit buffer, so
    /// they are done after the window closure like the HPSDR scan.
    bc_reload: &'a mut bool,
    bc_refetch: &'a mut bool,
    /// Whether a schedule download is in flight, and what the last one did.
    bc_fetching: bool,
    bc_status: Option<&'a Result<String, String>>,
    /// Radio-management actions from the roster strip at the top of the Radio
    /// page — switch/add/close/mute/rename — collected here like every other
    /// edit and handed to the multi-radio shell after the frame.
    radio_tabs: &'a mut Vec<crate::app::RadioTabRequest>,
    /// The rename field's buffer: (radio id, text as typed). UI-owned until
    /// the edit commits, so the roster's per-frame republish cannot fight the
    /// keyboard — see `SdroxideApp::radio_name_edit`.
    radio_name_edit: &'a mut Option<(u32, String)>,
    /// Spoken announcements, edited in place and written back after the
    /// window closure like every other buffer here.
    speech_edit: &'a mut sdroxide_types::SpeechSettings,
    /// Voices found on disk, listed once when the dialog opened.
    speech_voices: &'a [String],
    speech_status: &'a crate::app::speech::SpeechStatus,
    /// The TEST button was pressed; answered after the closure, where the
    /// announcer is reachable.
    speech_test: &'a mut bool,
    /// The station's IARU region. Applied and sent the moment it changes —
    /// there is no APPLY step on the General tab, and the whole point of it is
    /// that the band plan follows immediately.
    region_edit: &'a mut sdroxide_types::Region,
    /// The transmit parametric EQ (voice modes only). Same contract as
    /// `region_edit`: no APPLY step, sent the moment a band changes. This is
    /// live `RadioState`, not a per-backend `RadioConfig`, so it applies
    /// regardless of which radio interface is selected.
    tx_eq_edit: &'a mut sdroxide_types::TxEqState,
    tab: &'a mut SettingsTab,
    /// Which logging service the Uploads tab's own strip is showing. Its own
    /// field rather than state inside the tab's body, because the body is drawn
    /// from scratch every frame — see `SdroxideApp::settings_upload_tab`.
    upload_tab: &'a mut sdroxide_types::UploadTarget,
}

/// Guard for the three tabs that edit the network config. Returns whether the
/// station's copy has arrived; if it has not, it says so and the caller draws
/// nothing.
///
/// The alternative — showing the boxes empty — reads as "this station has
/// nothing configured", which is a lie whenever the engine is on another
/// machine, and one the operator could act on by typing over it.
fn net_seeded_note(ui: &mut egui::Ui, seeded: bool) -> bool {
    if !seeded {
        ui.label(RichText::new("Waiting for the station's network configuration…").weak());
    }
    seeded
}

/// One "which frequencies does this radio cover" box, in the megahertz an
/// operator would say them in.
///
/// Whatever is typed is checked as it is typed and the fault named underneath,
/// because the alternative — finding out on Apply, by way of a band that has
/// quietly stopped working — is a poor way to learn that a dash was a slash.
fn freq_range_edit(ui: &mut egui::Ui, id: &str, text: &mut String, hover: &str) {
    ui.vertical(|ui| {
        crate::chrome::field(
            ui,
            egui::TextEdit::singleline(text)
                .id_salt(id)
                .desired_width(220.0)
                .hint_text("as the device reports"),
        )
        .on_hover_text(hover);
        if let Err(e) = sdroxide_types::parse_freq_ranges(text) {
            ui.label(RichText::new(e).color(Color32::from_rgb(230, 90, 80)));
        }
    });
}

/// A frequency field shown and typed in megahertz, held in hertz.
///
/// The converter Offset above these is in hertz because that is how every
/// converter's documentation states it. A transverter's *band* is not: nobody
/// writes 144000000, and the RX/TX range boxes on this same page are already in
/// megahertz. So this one reads the way a band plan does.
fn mhz_drag(ui: &mut egui::Ui, hz: &mut f64, hover: &str) -> egui::Response {
    ui.add(
        egui::DragValue::new(hz)
            .speed(1000.0)
            .range(0.0..=sdroxide_types::FREQ_RANGE_MAX_HZ)
            .custom_formatter(|n, _| format!("{:.4}", n / 1e6))
            .custom_parser(|s| s.trim().parse::<f64>().ok().map(|m| m * 1e6))
            .suffix(" MHz"),
    )
    .on_hover_text(hover)
}

/// The transverter table: one row per box in front of the radio, each with the
/// band it works and the offset it works it at (issue #278).
///
/// Kept apart from the single **Offset** field above rather than replacing it.
/// The two answer different questions: an upconverter or an LNB is in front of
/// *everything* and has one offset for the whole dial, while a transverter is
/// in front of one band and the radio is on its own below. A station can have
/// both, and a dial no row covers falls through to the offset above it.
fn transverter_table(ui: &mut egui::Ui, cfg: &mut sdroxide_types::RadioConfig) {
    use sdroxide_types::{ConverterTx as Tx, MAX_TRANSVERTERS, Transverter};

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(4.0);
    ui.label(RichText::new("Transverters").size(14.0).strong().color(crate::theme::CYAN()));
    ui.add_space(2.0);
    ui.label(
        RichText::new(
            "One row per box. While the dial is inside a row's band the radio is tuned to \
             dial + offset and that row's transmit rule and drive limit apply; outside every \
             row the radio is on its own bands. Rows are tried top to bottom.",
        )
        .weak(),
    );
    ui.add_space(6.0);

    let mut remove: Option<usize> = None;
    egui::Grid::new("xvtr-grid").num_columns(8).spacing([10.0, 6.0]).striped(true).show(ui, |ui| {
        for h in ["", "Name", "Band low", "Band high", "Offset", "Transmit", "Max drive", ""] {
            ui.label(RichText::new(h).weak().size(10.0));
        }
        ui.end_row();

        for (i, x) in cfg.transverters.iter_mut().enumerate() {
            crate::chrome::checkbox(ui, &mut x.enabled, "").on_hover_text(
                "Off takes this transverter out of the line without losing the row — the \
                     band goes back to whatever the radio reaches on its own.",
            );
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut x.name)
                    .id_salt(("xvtr-name", i))
                    .desired_width(110.0)
                    .hint_text("2 m"),
            )
            .on_hover_text("What you call it. Shown in the log when the dial selects it.");
            mhz_drag(
                ui,
                &mut x.rf_lo_hz,
                "The bottom of the band this transverter works, \
                     on the dial. 144 for 2 m.",
            );
            mhz_drag(
                ui,
                &mut x.rf_hi_hz,
                "The top of the band, on the dial. 148 for 2 m \
                     in Regions 2 and 3, 146 in Region 1.",
            );
            ui.add(
                egui::DragValue::new(&mut x.offset_hz)
                    .speed(1000.0)
                    .range(
                        -sdroxide_types::CONVERTER_OFFSET_MAX_HZ
                            ..=sdroxide_types::CONVERTER_OFFSET_MAX_HZ,
                    )
                    .custom_formatter(|n, _| format!("{:.4}", n / 1e6))
                    .custom_parser(|s| s.trim().parse::<f64>().ok().map(|m| m * 1e6))
                    .suffix(" MHz"),
            )
            .on_hover_text(
                "The radio ends up on dial + offset, so a transverter that brings a band \
                     down to an I.F. is negative: 2 m into a 28 MHz I.F. is -116, because \
                     144 - 116 = 28. Drag to trim it a hertz at a time for an oscillator that \
                     is slightly off.",
            );
            egui::ComboBox::from_id_salt(("xvtr-tx", i))
                .selected_text(x.tx.label())
                .width(120.0)
                .show_styled(ui, |ui| {
                    for opt in [Tx::Off, Tx::Transverter] {
                        let on = std::mem::discriminant(&opt) == std::mem::discriminant(&x.tx);
                        if ui.selectable_label(on, opt.label()).clicked() && !on {
                            x.tx = opt;
                        }
                    }
                })
                .response
                .on_hover_text(
                    "Off while converting: receive only — the row is a converter, not a \
                         transverter, and nothing is keyed on this band.\n\nThrough the same \
                         converter: it works both ways, and transmit takes the same offset.",
                );
            ui.add(
                egui::DragValue::new(&mut x.tx_drive)
                    .speed(0.005)
                    .range(0.0..=1.0)
                    .custom_formatter(|n, _| format!("{:.0}", n * 100.0))
                    .custom_parser(|s| s.trim().parse::<f64>().ok().map(|p| p / 100.0))
                    .suffix(" %"),
            )
            .on_hover_text(
                "A ceiling on transmit drive while this transverter is selected, as a \
                     percentage of full. A transverter's I.F. input takes milliwatts and the \
                     drive that is right for the radio's own bands will destroy it, so this is \
                     the row's most important field. 100% is no limit.\n\nYour Drive setting \
                     is held under this rather than moved, so the number you use on HF comes \
                     back when the dial leaves the band.",
            );
            if crate::chrome::chip(ui, false, "REMOVE").clicked() {
                remove = Some(i);
            }
            ui.end_row();
        }
    });
    if let Some(i) = remove {
        cfg.transverters.remove(i);
    }
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let full = cfg.transverters.len() >= MAX_TRANSVERTERS;
        if ui
            .add_enabled(!full, egui::Button::new("ADD TRANSVERTER"))
            .on_disabled_hover_text(format!("{MAX_TRANSVERTERS} is the most this table holds"))
            .clicked()
        {
            cfg.transverters.push(Transverter { tx: Tx::Transverter, ..Transverter::default() });
        }
        ui.label(
            RichText::new("Takes effect on Apply / reconnect, like the converter offset above.")
                .weak(),
        );
    });
}

/// The per-band transmit drive calibration: one trim per band, so that one
/// Drive setting means one output power everywhere (issue #295).
///
/// Rows are only written for bands the operator has actually moved. A table
/// full of zeros and one full of nothing say the same thing, and the second is
/// what a station that has never opened this keeps in its `radio.json`.
fn drive_trim_table(ui: &mut egui::Ui, cfg: &mut sdroxide_types::RadioConfig) {
    use sdroxide_types::{Band, BandDriveTrim};

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(4.0);
    ui.label(
        RichText::new("Transmit drive by band").size(14.0).strong().color(crate::theme::CYAN()),
    );
    ui.add_space(2.0);
    ui.label(
        RichText::new(
            "Every amplifier has a different gain on every band, so one Drive setting makes a \
             different power on each. Measure the output on each band, then trim the bands \
             that come out high until they all match: set Drive for the band that needs the \
             most (usually the highest), and take the others down to it. Decibels of output \
             power, applied to voice, digital and TUNE alike. Zero everywhere — the default — \
             changes nothing.",
        )
        .weak(),
    );
    ui.add_space(6.0);

    // Bands the station's own region actually has: a Region 1 operator has no
    // 1.25 m to calibrate, and a row for one would be a row that can never be
    // reached. GEN closes the table for a dial outside every ham band.
    let bands: Vec<Band> =
        Band::ALL.iter().copied().filter(|b| *b == Band::Gen || b.edges().is_some()).collect();
    let mut set: Option<(Band, f32)> = None;
    egui::Grid::new("drive-trim-grid").num_columns(8).spacing([12.0, 6.0]).striped(true).show(
        ui,
        |ui| {
            for (i, band) in bands.iter().enumerate() {
                let mut db =
                    cfg.tx_drive_trim.iter().find(|t| t.band == *band).map(|t| t.db).unwrap_or(0.0);
                ui.label(if *band == Band::Gen { "Other" } else { band.label() });
                if ui
                    .add(
                        egui::DragValue::new(&mut db)
                            .speed(0.1)
                            .range(BandDriveTrim::DB_RANGE)
                            .fixed_decimals(1)
                            .suffix(" dB"),
                    )
                    .changed()
                {
                    set = Some((*band, db));
                }
                if i % 4 == 3 {
                    ui.end_row();
                }
            }
            ui.end_row();
        },
    );
    if let Some((band, db)) = set {
        cfg.tx_drive_trim.retain(|t| t.band != band);
        if db != 0.0 {
            cfg.tx_drive_trim.push(BandDriveTrim { band, db });
        }
    }
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui
            .add_enabled(!cfg.tx_drive_trim.is_empty(), egui::Button::new("CLEAR ALL"))
            .on_hover_text("Back to no calibration: the Drive setting reaches every band whole.")
            .clicked()
        {
            cfg.tx_drive_trim.clear();
        }
        ui.label(
            RichText::new(
                "Applies immediately, and to the band you would transmit on — behind a \
                 transverter that is the band on the dial, and the transverter's own drive \
                 limit still wins over anything set here.",
            )
            .weak(),
        );
    });
}

/// One row of the transmit-EQ grid: a band's frequency, gain and Q/slope.
/// `q_range` doubles as the shelf-slope range for the two shelf bands, see
/// [`sdroxide_types::TxEqBand::q`].
fn tx_eq_band_row(
    ui: &mut egui::Ui,
    label: &str,
    band: &mut sdroxide_types::TxEqBand,
    freq_range: std::ops::RangeInclusive<f32>,
    q_range: std::ops::RangeInclusive<f32>,
    hover: &str,
) {
    ui.label(label).on_hover_text(hover);
    ui.add(egui::DragValue::new(&mut band.freq_hz).range(freq_range).suffix(" Hz").speed(5.0));
    ui.add(
        egui::DragValue::new(&mut band.gain_db)
            .range(sdroxide_types::TxEqState::GAIN_DB_RANGE)
            .suffix(" dB")
            .speed(0.2),
    );
    ui.add(egui::DragValue::new(&mut band.q).range(q_range).speed(0.02));
    ui.end_row();
}

pub(in crate::app) fn enum_combo<T: PartialEq + Copy>(
    ui: &mut egui::Ui,
    id: &str,
    cur: &mut T,
    all: &[T],
    label: impl Fn(T) -> &'static str,
) {
    ComboBox::from_id_salt(id).selected_text(label(*cur)).show_styled(ui, |ui| {
        for &opt in all {
            if ui.selectable_label(*cur == opt, label(opt)).clicked() {
                *cur = opt;
            }
        }
    });
}

/// The concrete interface types the user chooses between in the picker.
/// SoapySDR only appears when compiled in; there is no auto-detect (an
/// unavailable interface falls back to a null source so the user can
/// reconfigure).
///
/// Built in whatever order the reasoning below groups them and sorted by
/// label at the end: the list is long enough now that an operator looks
/// for their radio by name rather than reading it through, and sorting
/// means a backend added here needs no thought about where to put it. A
/// free function (not inlined into the settings UI) so
/// [`every_backend_but_auto_and_none_is_offered_in_the_picker`] can check it
/// stays complete without going through the whole `egui` render path.
fn iface_opts(soapy_supported: bool) -> Vec<sdroxide_types::Backend> {
    let mut opts: Vec<sdroxide_types::Backend> = Vec::new();
    if soapy_supported {
        opts.push(sdroxide_types::Backend::Soapy);
    }
    opts.push(sdroxide_types::Backend::Hpsdr);
    opts.push(sdroxide_types::Backend::Cat);
    opts.push(sdroxide_types::Backend::Tci);
    // Pure-Rust UDP, no system library — in every build variant, as TCI is.
    opts.push(sdroxide_types::Backend::IcomNet);
    opts.push(sdroxide_types::Backend::SmartSdr);
    // Pure-Rust IIOD over TCP — no libiio, no libusb — so like the two USB
    // backends below it is in every build variant.
    opts.push(sdroxide_types::Backend::Pluto);
    // Ungated, unlike SoapySDR: the RTL-SDR driver is pure Rust and needs
    // no system library, so it is compiled into every build variant.
    opts.push(sdroxide_types::Backend::RtlSdr);
    // The same driver over a socket instead of the USB bus — pure Rust and
    // std::net, so it is in every build variant too.
    opts.push(sdroxide_types::Backend::RtlTcp);
    // Pure Rust over std::net as well: any receiver somebody has published
    // with spyserver, in either of the two shapes it can send.
    opts.push(sdroxide_types::Backend::SpyServer);
    opts.push(sdroxide_types::Backend::SpyServerVfo);
    // Pure Rust over `ws://`, no TLS and no system library: the ~900
    // public KiwiSDRs and Web-888s, and any private one on the same
    // firmware.
    opts.push(sdroxide_types::Backend::KiwiSdr);
    // Same reasoning as the RTL-SDR: pure Rust over `nusb`, no system
    // library, so it is in every build variant.
    opts.push(sdroxide_types::Backend::Rx888);
    // Same reasoning again: pure Rust over `nusb`, no libairspyhf and no
    // system library, so it is in every build variant.
    opts.push(sdroxide_types::Backend::AirspyHf);
    // Same again, and a different radio from the HF+ above despite the
    // name: an R2/Mini is other silicon behind another protocol.
    opts.push(sdroxide_types::Backend::Airspy);
    // A fork of the Airspy R2 rather than a relative of it, and its own
    // interface for the same reason the two crates are separate: neither
    // driver tunes the other's hardware correctly.
    opts.push(sdroxide_types::Backend::HydraSdr);
    // And again — pure Rust over `nusb`, no libhackrf. The only one of
    // these USB backends that transmits, which is why it is the only one
    // whose settings tab has a switch to arm before it will.
    opts.push(sdroxide_types::Backend::HackRf);
    // Also in every build variant, but for a different reason: nothing is
    // linked at build time — the vendor's sdrplay_api library is found
    // with dlopen at runtime, and opening explains what to install when
    // it is absent.
    opts.push(sdroxide_types::Backend::SdrPlay);
    // Pure Rust over `nusb` again, so it is in every build variant. On an
    // FDM-DUO this one interface covers the whole radio — the USB receiver,
    // the CAT serial link and the transmit sound card — which is why it is
    // here rather than under CAT / Audio.
    opts.push(sdroxide_types::Backend::Elad);
    // The one interface here that needs a library installed rather than
    // shipping its own driver — LimeSuite, found by dlopen at runtime, so
    // this still builds and runs everywhere and merely finds nothing where
    // the library is absent. Offered unconditionally for that reason: a
    // greyed-out entry would not say what to install.
    opts.push(sdroxide_types::Backend::Lime);
    // Also in every build variant, and for the same reason as SdrPlay
    // above: libfobos is LGPL and open source, but still found with
    // dlopen at runtime rather than linked, so nothing here needs it
    // installed to build or to see this entry — only to open it.
    opts.push(sdroxide_types::Backend::Fobos);
    // Case-folded so HackRF lands under H beside HPSDR rather than after
    // it, which a byte-order sort would do.
    opts.sort_by_key(|b| b.label().to_ascii_lowercase());
    opts
}

/// The device list an interface draws itself from, where asking for it is
/// free: a USB enumeration that opens nothing, a sound-card list, or the
/// SDRplay service's own device table.
///
/// `None` for the interfaces with no list (an address is typed, not picked)
/// and for the three whose enumeration is *not* free — SoapySDR loads every
/// installed module, LimeSuite opens each board to read its identity, and the
/// HPSDR / FlexRadio / Pluto scans take seconds on the network. Those stay on
/// the buttons the operator presses deliberately.
fn free_device_probe(backend: sdroxide_types::Backend) -> Option<sdroxide_types::DeviceProbe> {
    use sdroxide_types::{Backend as B, DeviceProbe as P};
    Some(match backend {
        B::Cat => P::RadioAudio,
        B::RtlSdr => P::RtlSdr,
        B::Rx888 => P::Rx888,
        B::AirspyHf => P::AirspyHf,
        B::Airspy => P::Airspy,
        B::HydraSdr => P::HydraSdr,
        B::HackRf => P::HackRf,
        B::SdrPlay => P::SdrPlay,
        B::Elad => P::Elad,
        B::Fobos => P::Fobos,
        _ => return None,
    })
}

impl SdroxideApp {
    /// Ask the machine the radio is attached to a question about its devices.
    ///
    /// Answered on the spot where that machine is this one, and otherwise sent
    /// over the session to come back through
    /// [`SdroxideApp::apply_probe_answer`]. Callers do not care which: they ask
    /// and then draw whatever list they have.
    pub(in crate::app) fn ask_device(
        &mut self,
        ctx: &egui::Context,
        req: sdroxide_types::DeviceProbe,
    ) {
        // Before the ask, not after: a local answer arrives inside this call
        // and would be overwritten by its own "waiting".
        if let sdroxide_types::DeviceProbe::Test(t) = &req {
            self.set_test_outcome(t.kind(), TestOutcome::Waiting);
        }
        match self.ctrl.probe(req) {
            Some(answer) => self.apply_probe_answer(ctx, answer),
            None => {
                self.probe_waiting += 1;
                self.probe_wait_until = crate::time::now_unix_f64() + PROBE_TIMEOUT_S;
            }
        }
    }

    /// File one answer where the dialog will draw it.
    pub(in crate::app) fn apply_probe_answer(
        &mut self,
        ctx: &egui::Context,
        answer: sdroxide_types::ProbeAnswer,
    ) {
        use sdroxide_types::ProbeAnswer as A;
        self.probe_waiting = self.probe_waiting.saturating_sub(1);
        match answer {
            A::Host { soapy } => self.soapy_supported = soapy,
            A::RadioAudio { inputs, outputs } => self.radio_audio_devices = Some((inputs, outputs)),
            A::SerialPorts(p) => self.serial_ports = p,
            A::RtlSdr(d) => self.rtlsdr_devices = d,
            A::Rx888(d) => self.rx888_devices = d,
            A::AirspyHf(d) => self.airspyhf_devices = d,
            A::Airspy(d) => self.airspy_devices = d,
            A::HydraSdr(d) => self.hydrasdr_devices = d,
            A::HackRf(d) => self.hackrf_devices = d,
            A::SdrPlay(d) => self.sdrplay_devices = d,
            A::Fobos(d) => self.fobos_devices = d,
            A::Elad(d) => self.elad_devices = d,
            A::Lime(d) => self.lime_devices = d,
            // `Some` even when empty: "enumerated and found nothing" is a
            // different thing from "not enumerated yet", and only this one
            // means the operator should go looking for a driver.
            A::Soapy(d) => self.soapy_devices = Some(d),
            A::Hpsdr(d) => self.hpsdr_devices = d,
            A::SmartSdr(d) => self.smartsdr_devices = d,
            A::Pluto(d) => self.pluto_devices = d,
            A::Relays(d) => self.relay_devices = d,
            // Not a device on this machine but a list from the internet,
            // fetched by it — see `DeviceProbe::PublicSdrs`. Kept whole rather
            // than merged so a refresh that came back short replaces the list
            // instead of leaving stale receivers in it.
            A::PublicSdrs(d) => self.public_sdrs = Some(*d),
            A::Test(kind, result) => self.set_test_outcome(kind, TestOutcome::Done(result)),
            // Straight to the clipboard, which is what the button that asked
            // for it promised — from here it goes into a bug report.
            A::Report(_, text) => ctx.copy_text(text),
            // The far end does not answer device questions. Said once, remembered
            // for the session: the controls that ask grey out with a reason
            // instead of each one having to discover it for itself.
            A::Unsupported => {
                self.probes_answered = false;
                self.probe_waiting = 0;
            }
        }
    }

    fn set_test_outcome(&mut self, kind: sdroxide_types::TestKind, outcome: TestOutcome) {
        use sdroxide_types::TestKind as K;
        let slot = match kind {
            K::Tci => &mut self.tci_test_result,
            K::SpyServer => &mut self.spyserver_test_result,
            K::IcomNet => &mut self.icomnet_test_result,
            K::SmartSdr => &mut self.smartsdr_test_result,
            K::Pluto => &mut self.pluto_test_result,
            K::Kiwi => &mut self.kiwi_test_result,
        };
        *slot = Some(outcome);
    }

    pub(in crate::app) fn settings_window(&mut self, ctx: &egui::Context, cmds: &mut Vec<Command>) {
        // Query slow lists (cpal devices, serial ports, radio config) once per
        // dialog-open; a pick invalidates so the selection refreshes.
        if !self.show_settings {
            self.audio_devices = None;
            self.audio_devices_queried = false;
            // Drop an uncommitted converter offset rather than carry it to the
            // next open: closing the dialog without pressing Apply means the
            // radio is still on the old one, and the box should say so.
            self.converter_edit_hz = None;
            self.range_edit = None;
            self.rx_site_edit = None;
            // The next open starts at the tab bar, not wherever the window was
            // last scrolled to — the offset is one shared egui memory for all
            // the tabs, and it outlives the process.
            self.settings_scroll_top = true;
            // ...and asks again for the device list of whatever interface it
            // opens on: a dongle can be unplugged between one open and the
            // next.
            self.iface_probed = None;
            return;
        } else if !self.audio_devices_queried {
            self.audio_devices = self.ctrl.audio_devices();
            self.radio_cfg = self.ctrl.radio_config();
            (self.midi_in_ports, self.midi_out_ports) = self.input.midi_ports();
            // What the radio's machine can do at all, which decides whether
            // SoapySDR is offered as an interface — so it is asked before the
            // list below is built rather than when somebody reaches for it.
            self.ask_device(ctx, sdroxide_types::DeviceProbe::Host);
            self.ask_device(ctx, sdroxide_types::DeviceProbe::SerialPorts);
            // The switching hardware, for the T/R switch's picker. Non-invasive
            // — nothing is opened — and asked here rather than when the tab is
            // opened so the list is already there when it is.
            self.ask_device(ctx, sdroxide_types::DeviceProbe::Relays);
            // Everything the *station* is set to — the network cockpit, the two
            // built-in servers, the WSJT-X broadcast and the satellites — was
            // seeded from `RadioEvent::StationConfig` and needs no query here:
            // those files live wherever the engine does, which may not be this
            // machine at all.
            //
            // Subscription status is the exception worth refreshing: a native
            // client with the solar window open has a fetcher of its own, and
            // what it last did is fresher than what the engine announced.
            self.refresh_sat_sub_status();
            // Reading a directory listing is cheap, but not per-frame cheap.
            self.speech_voices = crate::app::speech::SpeechRuntime::voices();
            // Only when SoapySDR is the interface being configured: enumeration
            // loads every installed module and asks each to scan its bus, which
            // is not something to do to an operator who is here to change their
            // waterfall colours. Rescan re-runs it for anyone switching to it.
            if self.radio_cfg.as_ref().is_some_and(|c| c.backend == sdroxide_types::Backend::Soapy)
            {
                self.ask_device(ctx, sdroxide_types::DeviceProbe::Soapy);
            }
            if self.radio_cfg.as_ref().is_some_and(|c| c.backend == sdroxide_types::Backend::Lime) {
                self.ask_device(ctx, sdroxide_types::DeviceProbe::Lime);
            }
            // Whichever interface is configured, its device list — the sound
            // cards a CAT rig is wired to, the dongles on the bus, the RSPs the
            // service reports. Every one of these is cheap and opens nothing,
            // and each tab draws itself from its own list: which antenna ports
            // exist, whether there is an HDR path, how far the LNA goes. Asked
            // here so the tab is furnished when it is first looked at, and
            // remembered so switching *to* another interface asks for that
            // one's (see below) — a list that never arrived is a tab that
            // silently falls back to a different model's feature set, which is
            // issue #165.
            let applied = self.radio_cfg.as_ref().map(|c| c.backend);
            self.iface_probed = applied;
            if let Some(p) = applied.and_then(free_device_probe) {
                self.ask_device(ctx, p);
            }
            if self.radio_cfg.as_ref().is_some_and(|c| c.backend == sdroxide_types::Backend::Elad) {
                // An FDM-DUO's control link is a serial port like any other
                // rig's, and its transmit audio is a sound card, so this tab
                // needs both lists the CAT tab needs. The ports are asked for
                // unconditionally above; the cards are not.
                self.ask_device(ctx, sdroxide_types::DeviceProbe::RadioAudio);
            }
            self.audio_devices_queried = true;
        } else if self.radio_cfg.is_none() {
            // Still waiting for the interface configuration. On a remote client
            // it arrives with the connect-time replay, so this is normally
            // answered before anyone opens the dialog — but a client that
            // opened it during a reconnect would otherwise sit on "only
            // available in the native app" until the dialog was closed and
            // reopened. One `Option` clone a frame, and only while it is empty.
            self.radio_cfg = self.ctrl.radio_config();
        }
        // A question that was never answered — the far end dropped it under
        // load, or the session went away mid-scan — must not leave the controls
        // that ask dark for the rest of the session.
        if self.probe_waiting > 0 && crate::time::now_unix_f64() > self.probe_wait_until {
            self.probe_waiting = 0;
        }
        // Edits collected here and applied after the window closure, which
        // borrows `&self` and so can't touch `&mut self.ctrl`.
        let mut audio_pick: Option<(bool, Option<String>)> = None;
        let mut speech_edit = self.speech.settings().clone();
        let speech_status = self.speech.status();
        let mut speech_test = false;
        let mut hpsdr_discover = false;
        let mut rtlsdr_rescan = false;
        let mut rx888_rescan = false;
        let mut airspyhf_rescan = false;
        let mut airspyhf_copy_report = false;
        let mut elad_rescan = false;
        let mut elad_copy_report = false;
        let mut lime_rescan = false;
        let mut lime_copy_report = false;
        let mut hackrf_rescan = false;
        let mut hackrf_copy_report = false;
        let mut airspy_rescan = false;
        let mut airspy_copy_report = false;
        let mut hydrasdr_rescan = false;
        let mut hydrasdr_copy_report = false;
        let mut sdrplay_rescan = false;
        let mut fobos_rescan = false;
        let mut soapy_rescan = false;
        let mut tci_test = false;
        let mut icomnet_test = false;
        let mut spyserver_test = false;
        let mut kiwi_test = false;
        let mut icomnet_copy_report = false;
        let mut smartsdr_discover = false;
        let mut smartsdr_test = false;
        let mut smartsdr_copy_report = false;
        let mut pluto_discover = false;
        let mut pluto_test = false;
        let mut pluto_copy_report = false;
        let mut apply_iface = false;
        let mut radio_edit = self.radio_cfg.clone();
        let mut converter_hz = self.converter_edit_hz;
        let mut ranges = self.range_edit.clone();
        let mut rx_site = self.rx_site_edit.clone();
        let mut ui_edit = self.ui_settings;
        // Only where the engine is in this process: see `SettingsIo`.
        let owns_server = !self.ctrl.engine_is_remote();
        let mut access_edit = self.remote_access.clone();
        #[cfg(not(target_arch = "wasm32"))]
        let mut remote_edit = self.remote_server.clone();
        #[cfg(not(target_arch = "wasm32"))]
        let mut remote_connect = false;
        let mut digi_edit = self.digi_cfg_edit.clone();
        let digi_seeded = self.digi_cfg_seeded;
        let mut region_edit = self.region_edit;
        let mut tx_eq_edit = self.state.tx.eq;
        let mut net_edit = self.net_cfg_edit.clone();
        let mut net_cmds = self.net_cluster_cmds.clone();
        let mut rbn_cmds = self.net_rbn_cmds.clone();
        let mut net_apply = false;
        let mut net_sync = false;
        let mut tci_srv_edit = self.tci_srv_edit.clone();
        let mut tci_srv_apply = false;
        let mut rigctld_edit = self.rigctld_edit.clone();
        let mut rigctld_apply = false;
        let mut wsjtx_edit = self.wsjtx_edit.clone();
        let mut wsjtx_apply = false;
        let mut rot_edit = self.rot_cfg_edit.clone();
        let mut rot_apply = false;
        let mut relay_edit = self.relay_edit.clone();
        let mut relay_apply = false;
        let mut relay_test: Option<u8> = None;
        let mut input_edit = self.input.cfg.clone();
        let mut key_capture = self.input.key_capture;
        let mut midi_learn = self.input.midi_learn;
        let mut midi_rescan = false;
        let mut sat_edit = self.sat_cfg_edit.clone();
        let mut sat_ui = std::mem::take(&mut self.sat_ui);
        let mut sat_sub_refresh = false;
        let sat_subs = self.sat_sub_status.clone();
        let mut bc_reload = false;
        let mut bc_refetch = false;

        // The concrete interface types the user chooses between.
        let iface_opts = iface_opts(self.soapy_supported);

        let mut tab = self.settings_tab;
        let mut upload_tab = self.settings_upload_tab;
        let mut open = self.show_settings;
        // The 3D window owns the live copy of its own settings — `view.solar3d`
        // is only the snapshot persisted from it — so this is read out of the
        // window here and handed back to it below, the way `ui_edit` is.
        #[cfg(not(target_arch = "wasm32"))]
        let mut solar_cloud_march = self.solar.cloud_march();
        let mut radio_tab_reqs: Vec<crate::app::RadioTabRequest> = Vec::new();
        let mut radio_name_edit = self.radio_name_edit.take();
        // The window does its own scrolling, so its bar can only be themed
        // through the context style — lend the palette for the length of the
        // call and hand the body back the normal one.
        let bars = crate::theme::ScrollPalette::push(ctx);
        // Sized like the other overlays instead of by its contents. An
        // auto-sized egui window lays the body out inside `Resize`'s remembered
        // size, which grows to the widest tab ever shown and never shrinks
        // again — so the width drifted with wherever the operator had been
        // while the height stayed at egui's 420 pt default no matter how tall
        // the display was.
        let want =
            egui::vec2(crate::layout::window_w(ctx, 900.0), crate::layout::window_h(ctx, 760.0));
        let resp = egui::Window::new("Settings")
            // Pinned id, versioned: egui persists the remembered size and
            // position under it, and the suffix drops the stale (often very
            // wide, always 420 pt tall) geometry left by the builds before this
            // one. Position is centred on first use because it goes with it.
            .id(crate::layout::salted_id(ctx, "settings-window-v2"))
            .open(&mut open)
            .frame(crate::chrome::window_frame())
            .resizable(true)
            .default_pos(ctx.content_rect().center() - want * 0.5)
            .default_size(want)
            .min_width(crate::layout::window_w(ctx, 380.0))
            .min_height(crate::layout::window_h(ctx, 300.0))
            .vscroll(true)
            .show(ctx, |ui| {
                crate::chrome::window_body_bg(ui);
                bars.restore(ui);
                // On the frame the dialog opens, jump back to the origin so
                // the tab bar is on screen. Instant, not animated: a leftover
                // offset gliding away would look like the window scrolling by
                // itself. Two calls, because a scroll area only acts on
                // targets set inside its own content and discards the axis it
                // doesn't scroll — this one reaches the window's vertical bar,
                // the one below reaches the horizontal region.
                let jump_to_origin = std::mem::take(&mut self.settings_scroll_top);
                if jump_to_origin {
                    ui.scroll_to_cursor_animation(
                        Some(egui::Align::TOP),
                        egui::style::ScrollAnimation::none(),
                    );
                }
                // The tabs that are wider than the window get a scrollbar
                // instead of widening it: the Controls tables and the TLE
                // element editors are laid out at fixed widths, and egui grows a
                // window to whatever its content asked for and never gives the
                // width back. Text still wraps at the window's width — pinning
                // the body's max width is what keeps `available_width` finite
                // inside a horizontally scrollable region.
                let body_w = ui.available_width();
                egui::ScrollArea::horizontal().show_themed(ui, |ui| {
                    if jump_to_origin {
                        ui.scroll_to_cursor_animation(
                            Some(egui::Align::TOP),
                            egui::style::ScrollAnimation::none(),
                        );
                    }
                    ui.set_max_width(body_w);
                    self.settings_body(
                        ui,
                        cmds,
                        &mut SettingsIo {
                            iface_opts: &iface_opts,
                            radio_edit: &mut radio_edit,
                            // Both of these questions are asked of the machine
                            // the radio is on, which may not be this one: it
                            // answers the device lists over the session, and
                            // stays quiet only if it was built without a way to.
                            can_probe: self.probes_answered && self.probe_waiting == 0,
                            local_engine: owns_server,
                            converter_hz: &mut converter_hz,
                            ranges: &mut ranges,
                            rx_site: &mut rx_site,
                            audio_pick: &mut audio_pick,
                            hpsdr_discover: &mut hpsdr_discover,
                            rtlsdr_rescan: &mut rtlsdr_rescan,
                            rx888_rescan: &mut rx888_rescan,
                            airspyhf_rescan: &mut airspyhf_rescan,
                            airspyhf_copy_report: &mut airspyhf_copy_report,
                            elad_rescan: &mut elad_rescan,
                            elad_copy_report: &mut elad_copy_report,
                            lime_rescan: &mut lime_rescan,
                            lime_copy_report: &mut lime_copy_report,
                            hackrf_rescan: &mut hackrf_rescan,
                            hackrf_copy_report: &mut hackrf_copy_report,
                            airspy_rescan: &mut airspy_rescan,
                            airspy_copy_report: &mut airspy_copy_report,
                            hydrasdr_rescan: &mut hydrasdr_rescan,
                            hydrasdr_copy_report: &mut hydrasdr_copy_report,
                            sdrplay_rescan: &mut sdrplay_rescan,
                            fobos_rescan: &mut fobos_rescan,
                            soapy_rescan: &mut soapy_rescan,
                            tci_test: &mut tci_test,
                            smartsdr_discover: &mut smartsdr_discover,
                            smartsdr_test: &mut smartsdr_test,
                            icomnet_test: &mut icomnet_test,
                            spyserver_test: &mut spyserver_test,
                            kiwi_test: &mut kiwi_test,
                            icomnet_copy_report: &mut icomnet_copy_report,
                            smartsdr_copy_report: &mut smartsdr_copy_report,
                            pluto_discover: &mut pluto_discover,
                            pluto_test: &mut pluto_test,
                            pluto_copy_report: &mut pluto_copy_report,
                            apply_iface: &mut apply_iface,
                            ui_edit: &mut ui_edit,
                            access_edit: owns_server.then_some(&mut access_edit),
                            #[cfg(not(target_arch = "wasm32"))]
                            remote_edit: &mut remote_edit,
                            #[cfg(not(target_arch = "wasm32"))]
                            remote_connect: &mut remote_connect,
                            digi_edit: &mut digi_edit,
                            digi_seeded,
                            net_edit: &mut net_edit,
                            net_seeded: self.net_cfg_seeded,
                            net_cmds: &mut net_cmds,
                            rbn_cmds: &mut rbn_cmds,
                            net_apply: &mut net_apply,
                            bc_reload: &mut bc_reload,
                            bc_refetch: &mut bc_refetch,
                            bc_fetching: self.broadcast_fetch.is_some(),
                            bc_status: self.broadcast_fetch_status.as_ref(),
                            speech_edit: &mut speech_edit,
                            speech_voices: &self.speech_voices,
                            speech_status: &speech_status,
                            speech_test: &mut speech_test,
                            net_sync: &mut net_sync,
                            tci_srv_edit: &mut tci_srv_edit,
                            tci_srv_apply: &mut tci_srv_apply,
                            rigctld_edit: &mut rigctld_edit,
                            rigctld_apply: &mut rigctld_apply,
                            wsjtx_edit: &mut wsjtx_edit,
                            wsjtx_apply: &mut wsjtx_apply,
                            rot_edit: &mut rot_edit,
                            rot_apply: &mut rot_apply,
                            relay_edit: &mut relay_edit,
                            relay_apply: &mut relay_apply,
                            relay_test: &mut relay_test,
                            input_edit: &mut input_edit,
                            key_capture: &mut key_capture,
                            midi_learn: &mut midi_learn,
                            midi_rescan: &mut midi_rescan,
                            sat_edit: &mut sat_edit,
                            sat_seeded: self.sat_cfg_seeded,
                            sat_ui: &mut sat_ui,
                            sat_subs: &sat_subs,
                            sat_sub_refresh: &mut sat_sub_refresh,
                            #[cfg(not(target_arch = "wasm32"))]
                            solar_cloud_march: Some(&mut solar_cloud_march),
                            #[cfg(target_arch = "wasm32")]
                            solar_cloud_march: None,
                            radio_tabs: &mut radio_tab_reqs,
                            radio_name_edit: &mut radio_name_edit,
                            region_edit: &mut region_edit,
                            tx_eq_edit: &mut tx_eq_edit,
                            tab: &mut tab,
                            upload_tab: &mut upload_tab,
                        },
                    );
                });
            });
        bars.pop(ctx);
        if let Some(r) = &resp {
            crate::chrome::paint_window_border(ctx, &r.response);
        }
        self.show_settings = open;
        self.settings_tab = tab;
        self.settings_upload_tab = upload_tab;
        // The multi-radio shell drains these after the frame.
        self.radio_tab_requests.append(&mut radio_tab_reqs);
        {
            self.radio_name_edit = radio_name_edit;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if solar_cloud_march != self.solar.cloud_march() {
            self.solar.set_cloud_march(solar_cloud_march);
        }
        // Persist net-config edits (kept across frames) and apply on demand.
        if self.net_cfg_seeded {
            self.net_cfg_edit = net_edit;
            self.net_cluster_cmds = net_cmds;
            self.net_rbn_cmds = rbn_cmds;
        }
        if net_apply && self.net_cfg_seeded {
            let split = |s: &str| -> Vec<String> {
                s.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect()
            };
            self.net_cfg_edit.cluster.commands = split(&self.net_cluster_cmds);
            self.net_cfg_edit.rbn.commands = split(&self.net_rbn_cmds);
            // The engine persists net.json when it applies this.
            cmds.push(Command::SetNetworkConfig(self.net_cfg_edit.clone()));
        }
        if net_sync {
            cmds.push(Command::SyncConfirmations);
        }
        self.input.key_capture = key_capture;
        self.input.midi_learn = midi_learn;
        if midi_rescan {
            (self.midi_in_ports, self.midi_out_ports) = self.input.midi_ports();
        }
        if input_edit != self.input.cfg {
            // Bindings take effect on the next frame and are written straight
            // out — a rebind the operator can't see saved is a rebind they
            // will make again after the next restart.
            self.input.cfg = input_edit;
            self.input.persist();
        }
        self.tci_srv_edit = tci_srv_edit;
        if tci_srv_apply {
            // The engine persists tciserver.json when it binds (or fails to).
            cmds.push(Command::SetTciServerConfig(self.tci_srv_edit.clone()));
        }
        self.rigctld_edit = rigctld_edit;
        if rigctld_apply {
            // The engine persists rigctld.json when it binds (or fails to).
            cmds.push(Command::SetRigctldConfig(self.rigctld_edit.clone()));
        }
        self.wsjtx_edit = wsjtx_edit;
        if wsjtx_apply {
            // The engine persists wsjtx.json when it opens the socket.
            cmds.push(Command::SetWsjtxConfig(self.wsjtx_edit.clone()));
        }
        self.rot_cfg_edit = rot_edit;
        if rot_apply {
            // The engine persists rotator.json when it (re)starts the client.
            cmds.push(Command::SetRotatorConfig(self.rot_cfg_edit.clone()));
        }
        self.relay_edit = relay_edit;
        if relay_apply {
            // The engine persists relay.json when it (re)opens the switch — and
            // refuses to reopen it mid-over, which is why this is an APPLY and
            // not a write-through.
            cmds.push(Command::SetRelayConfig(Box::new(self.relay_edit.clone())));
        }
        if let Some(channel) = relay_test {
            cmds.push(Command::TestRelay { channel });
        }
        self.sat_ui = sat_ui;
        if self.sat_cfg_seeded && sat_edit != self.sat_cfg_edit {
            // Written straight out, like the input bindings: there is no APPLY
            // step here, and a satellite the operator cannot see saved is one
            // they will add again after the next restart. The engine persists
            // it — the subscribed listings are fetched on its machine — and the
            // solar window picks the new `Arc` up on its next frame.
            //
            // Gated on having been seeded, so a client that has not yet been
            // told what the station tracks cannot write an empty list over it.
            self.sat_cfg_edit = sat_edit;
            self.sat_cfg_edit.prune();
            self.sat_cfg = std::sync::Arc::new(self.sat_cfg_edit.clone());
            cmds.push(Command::SetSatConfig(self.sat_cfg_edit.clone()));
        }
        if sat_sub_refresh {
            self.refresh_sat_subs_now(cmds);
        }
        if bc_refetch {
            self.refetch_broadcast_schedule();
        }
        if bc_reload {
            self.reload_broadcast_stations();
        }
        if let Some((output, name)) = audio_pick {
            self.ctrl.set_audio_device(output, name);
            self.audio_devices_queried = false;
        }
        // Every device question below is asked after the window closure, which
        // is what lets it take `&mut self`. Where the radio is on this machine
        // the answer is back before the next frame; where it is elsewhere the
        // question crosses the session and the answer arrives later, into the
        // same lists — see `SdroxideApp::ask_device`.
        use sdroxide_types::{DeviceProbe as P, ProbeTest as T, ReportKind as R};
        if hpsdr_discover {
            // A LAN scan (~1.5 s), on the radio's network — which is the only
            // one an HPSDR announcement would arrive on.
            self.ask_device(ctx, P::Hpsdr);
        }
        if rtlsdr_rescan {
            // USB enumeration only — no device is opened, so this is safe to
            // press at any time, including while a dongle is streaming.
            self.ask_device(ctx, P::RtlSdr);
        }
        if sdrplay_rescan {
            self.ask_device(ctx, P::SdrPlay);
        }
        if fobos_rescan {
            self.ask_device(ctx, P::Fobos);
        }
        if soapy_rescan {
            // Loads every installed SoapySDR module and asks each to scan, so
            // it can take a moment; on demand only, never per frame.
            self.ask_device(ctx, P::Soapy);
        }
        if rx888_rescan {
            self.ask_device(ctx, P::Rx888);
        }
        if airspyhf_rescan {
            self.ask_device(ctx, P::AirspyHf);
        }
        if airspyhf_copy_report {
            // This backend has not been verified against hardware, so the trace
            // is how a fault gets reported without asking anybody to reproduce
            // it under a log filter. It describes the session the *engine* ran,
            // so it is fetched from there and lands on this clipboard.
            self.ask_device(ctx, P::Report(R::AirspyHf));
        }
        if airspy_rescan {
            self.ask_device(ctx, P::Airspy);
        }
        if airspy_copy_report {
            self.ask_device(ctx, P::Report(R::Airspy));
        }
        if hydrasdr_rescan {
            self.ask_device(ctx, P::HydraSdr);
        }
        if hydrasdr_copy_report {
            self.ask_device(ctx, P::Report(R::HydraSdr));
        }
        if elad_rescan {
            self.ask_device(ctx, P::Elad);
        }
        if elad_copy_report {
            self.ask_device(ctx, P::Report(R::Elad));
        }
        if lime_rescan {
            self.ask_device(ctx, P::Lime);
        }
        if lime_copy_report {
            self.ask_device(ctx, P::Report(R::Lime));
        }
        if hackrf_rescan {
            self.ask_device(ctx, P::HackRf);
        }
        if hackrf_copy_report {
            // Worth more on this backend than on the receive-only ones: a
            // transmit fault is about the *order* control transfers went out
            // in around a key-down, which nobody can reconstruct from a
            // spectrum.
            self.ask_device(ctx, P::Report(R::HackRf));
        }
        if tci_test {
            if let Some(cfg) = &radio_edit {
                let address = cfg.tci.address.clone();
                self.ask_device(ctx, P::Test(T::Tci(address)));
            }
        }
        // Tests the address *typed in the dialog* rather than the applied one,
        // and against the block belonging to whichever of the two SpyServer
        // interfaces is selected — they are usually different servers, and
        // answering about the other one would be a lie that looks like a
        // success.
        if let (true, Some(cfg)) = (spyserver_test, &radio_edit) {
            let block = if cfg.backend == sdroxide_types::Backend::SpyServerVfo {
                &cfg.spyserver_vfo
            } else {
                &cfg.spyserver
            };
            if block.address.trim().is_empty() {
                // Answered here rather than sent: nothing on the far end can
                // say anything useful about an empty address.
                self.set_test_outcome(
                    sdroxide_types::TestKind::SpyServer,
                    TestOutcome::Done(Err("enter the server's address first".to_string())),
                );
            } else {
                let endpoint = block.endpoint();
                self.ask_device(ctx, P::Test(T::SpyServer(endpoint)));
            }
        }
        // The address typed in the dialog, like every other test here. A
        // receiver behind the project's proxy answers on port 80 rather than
        // 8073, so the endpoint is built from the config's own rule rather than
        // by appending a default here.
        if let (true, Some(cfg)) = (kiwi_test, &radio_edit) {
            if cfg.kiwi.address.trim().is_empty() {
                self.set_test_outcome(
                    sdroxide_types::TestKind::Kiwi,
                    TestOutcome::Done(Err(
                        "enter the receiver's address first, or pick one from Public SDRs"
                            .to_string(),
                    )),
                );
            } else {
                let endpoint = cfg.kiwi.endpoint();
                self.ask_device(ctx, P::Test(T::Kiwi(endpoint)));
            }
        }
        if smartsdr_discover {
            // A passive listen (~2.5 s) — radios broadcast unprompted, so
            // nothing is sent and nothing on the network is disturbed.
            self.ask_device(ctx, P::SmartSdr);
        }
        if smartsdr_test {
            if let Some(cfg) = &radio_edit {
                match cfg.smartsdr.target().map(str::to_string) {
                    Some(addr) => self.ask_device(ctx, P::Test(T::SmartSdr(addr))),
                    None => self.set_test_outcome(
                        sdroxide_types::TestKind::SmartSdr,
                        TestOutcome::Done(Err(
                            "no radio selected — press Discover, or enter an address".to_string(),
                        )),
                    ),
                }
            }
        }
        // What is *typed in the dialog*, not what was last applied — a test
        // that ignored an edited address would be answering a question nobody
        // asked.
        if let (true, Some(cfg)) = (icomnet_test, &radio_edit) {
            if cfg.icomnet.address.trim().is_empty() {
                self.set_test_outcome(
                    sdroxide_types::TestKind::IcomNet,
                    TestOutcome::Done(Err("enter the radio's address first".to_string())),
                );
            } else {
                let icomnet = Box::new(cfg.icomnet.clone());
                self.ask_device(ctx, P::Test(T::IcomNet(icomnet)));
            }
        }
        if icomnet_copy_report {
            self.ask_device(ctx, P::Report(R::IcomNet));
        }
        if smartsdr_copy_report {
            // The whole point of the trace is that a user can hand it over
            // without reproducing the fault under a log filter.
            self.ask_device(ctx, P::Report(R::SmartSdr));
        }
        if pluto_discover {
            // An mDNS query plus a direct try of the USB gadget's address
            // (~1.5 s), both from where the Pluto is plugged in.
            self.ask_device(ctx, P::Pluto);
        }
        if pluto_test {
            if let Some(cfg) = &radio_edit {
                let target = cfg.pluto.target();
                self.ask_device(ctx, P::Test(T::Pluto(target)));
            }
        }
        if pluto_copy_report {
            self.ask_device(ctx, P::Report(R::Pluto));
        }
        // Switching to another interface inside the open dialog: its device
        // list was not among the questions asked when the dialog opened, and
        // that list is what its tab draws itself from. Once per switch, not
        // once per frame — and only the free enumerations, never the ones that
        // open devices to identify them (SoapySDR, LimeSuite) or scan a
        // network, which stay on their Rescan buttons.
        let editing = radio_edit.as_ref().map(|c| c.backend);
        if editing.is_some()
            && editing != self.iface_probed
            && self.probes_answered
            && self.probe_waiting == 0
        {
            self.iface_probed = editing;
            if let Some(p) = editing.and_then(free_device_probe) {
                self.ask_device(ctx, p);
            }
            // The FDM-DUO's second list, for the same reason as on the
            // dialog-open path above: its transmit audio is a sound card.
            if editing == Some(sdroxide_types::Backend::Elad) {
                self.ask_device(ctx, P::RadioAudio);
            }
        }
        if apply_iface {
            // The fields that wait for this moment, so the radio is never
            // reopened on a partly-typed offset or a half-written range.
            if let (Some(cfg), Some(hz)) = (radio_edit.as_mut(), converter_hz) {
                cfg.converter_offset_hz = hz;
            }
            if let (Some(cfg), Some(site)) = (radio_edit.as_mut(), rx_site.as_ref()) {
                // Trimmed on the way in, so a stray space is not the difference
                // between a locator and "somewhere, nobody knows where".
                cfg.rx_site = match site {
                    sdroxide_types::RxSite::Elsewhere(g) => {
                        sdroxide_types::RxSite::Elsewhere(g.trim().to_string())
                    }
                    other => other.clone(),
                };
            }
            if let (Some(cfg), Some((rx, tx))) = (radio_edit.as_mut(), ranges.as_ref()) {
                // Anything that doesn't parse leaves that direction as it was:
                // the box is showing the operator why in red, and applying half
                // of what they meant would be worse than applying none of it.
                if let Ok(r) = sdroxide_types::parse_freq_ranges(rx) {
                    cfg.freq_ranges_rx = r;
                }
                if let Ok(r) = sdroxide_types::parse_freq_ranges(tx) {
                    cfg.freq_ranges_tx = r;
                }
            }
            // Persist the latest edits, then rebuild the live source (no restart).
            if let Some(cfg) = &radio_edit {
                self.ctrl.set_radio_config(cfg.clone());
            }
            self.ctrl.reopen_source();
            // An Apply here can move a *device* between two engines, and the
            // other engine has to be told or nothing happens. Three cases, and
            // reopening all of them covers each:
            //
            // - the receiver this radio is now claiming, which is still holding
            //   its own device open: a reopen makes it let go (the factory
            //   refuses it the device from here on), and this radio's own retry
            //   picks it up a moment later;
            // - a receiver this radio has just *stopped* borrowing, which
            //   should go back to being a radio without waiting for its retry;
            // - this radio itself, when it is the one that has been lent out —
            //   it has no source of its own to rebuild, so what was saved here
            //   reaches the hardware only through the borrower.
            //
            // A reopen that turns out to be none of them is harmless: the
            // factory refuses while a pairing stands, and a source that cannot
            // be replaced is left exactly as it was.
            let station = self.radio_roster.iter().find(|c| c.id == self.radio_id);
            let claiming = radio_edit
                .as_ref()
                .and_then(|c| c.panadapter.source_radio)
                .and_then(|rx| {
                    self.radio_roster.iter().find(|c| {
                        c.station_id == rx && station.is_none_or(|s| c.station == s.station)
                    })
                })
                .map(|c| c.id);
            let stir: Vec<u32> = self
                .radio_roster
                .iter()
                .filter(|c| c.attached_to.is_some())
                .map(|c| c.id)
                .chain(claiming)
                .chain(self.lent_to())
                .collect();
            for id in stir {
                self.radio_tab_requests.push(crate::app::RadioTabRequest::Reopen(id));
            }
        }
        self.converter_edit_hz = converter_hz;
        self.range_edit = ranges;
        self.rx_site_edit = rx_site;
        if radio_edit != self.radio_cfg {
            if let Some(cfg) = &radio_edit {
                self.ctrl.set_radio_config(cfg.clone());
            }
            self.radio_cfg = radio_edit;
        }
        if ui_edit != self.ui_settings {
            // Live: fps + averaging flow to the engine via the spectrum-config
            // diff next frame; waterfall speed is read each frame. Persist too.
            self.ui_settings = ui_edit;
            persist_ui_settings(&self.ui_settings);
        }
        if &speech_edit != self.speech.settings() {
            // Live too: rate and volume reach the running worker, and only a
            // change of voice or output device restarts it.
            self.speech.set_settings(speech_edit.clone());
            persist_speech_settings(&speech_edit);
        }
        if speech_test {
            self.speech.announcer.say_sample(ctx.input(|i| i.time));
        }
        // Written as it is typed, like the control bindings: the server rereads
        // the file for every sign-in, so there is no APPLY step to hang this
        // off. Gated on owning the server, so a remote client cannot write its
        // own machine's config.toml from a tab it was never shown.
        if owns_server && access_edit != self.remote_access {
            self.remote_access = access_edit;
            crate::app::persist::persist_remote_access(&self.remote_access);
        }
        // The address to dial, written as it is typed for the same reason: a
        // pressed CONNECT is a poor moment to find out the address was never
        // saved, and there is no APPLY step here to hang it off. Not gated on
        // `owns_server` — this is the operator's own machine's setting, so a
        // remote client is as entitled to it as the shack machine is.
        #[cfg(not(target_arch = "wasm32"))]
        {
            if remote_edit != self.remote_server {
                self.remote_server = remote_edit;
                crate::app::persist::persist_remote_server(&self.remote_server);
            }
            if remote_connect && !self.remote_server.host.trim().is_empty() {
                // Dialled by the shell after the frame: it owns the tab set,
                // and this connection needs a tab to live in.
                self.remote_status = Some(Ok(format!("Dialling {}…", self.remote_server.url())));
                self.radio_tab_requests.push(crate::app::RadioTabRequest::Connect {
                    url: self.remote_server.url(),
                    name: self.remote_server.label(),
                });
            }
        }
        // Callsign/grid from the General tab — same store as the FT8/SSTV setup
        // dialog. Only apply once seeded so we can't overwrite the engine's saved
        // config with defaults.
        if digi_seeded && digi_edit != self.digi_cfg_edit {
            self.digi_cfg_edit = digi_edit;
            cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
        }
        // The station's region. Applied here as well as sent, so the band
        // buttons and the waterfall's band strip redraw on this frame instead
        // of on whichever one the engine's echo lands in — and so the change is
        // visible even before a remote station has confirmed it. The engine
        // persists it and announces the authoritative value back.
        if region_edit != self.region_edit {
            self.region_edit = region_edit;
            sdroxide_types::set_region(region_edit);
            cmds.push(Command::SetRegion(region_edit));
        }
        // The transmit EQ: live `RadioState`, applied the moment a band
        // changes, same as the region above. There is no per-backend config
        // to wait on, so no APPLY step either.
        if tx_eq_edit != self.state.tx.eq {
            self.state.tx.eq = tx_eq_edit;
            cmds.push(Command::SetTxEq(tx_eq_edit));
        }
    }

    /// The Settings body: a General tab (station identity + the sound devices)
    /// and a Radio tab whose single interface selector drives the
    /// interface-specific section below it.
    ///
    /// Everything the dialog changes goes out through `io`, because the window
    /// closure borrows `&self` — see [`SettingsIo`].
    pub(in crate::app) fn settings_body(
        &self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        io: &mut SettingsIo,
    ) {
        use sdroxide_types::Backend;

        // Built rather than written out, because one of the tabs only exists on
        // native (see [`SettingsTab::Remote`]).
        let mut tabs = vec![
            (SettingsTab::General, "General"),
            (SettingsTab::Radio, "Radio"),
            (SettingsTab::Ui, "UI"),
            (SettingsTab::Controls, "Controls"),
            (SettingsTab::Spots, "Spots"),
            (SettingsTab::FreeDv, "FreeDV"),
            (SettingsTab::Uploads, "Uploads"),
            (SettingsTab::Winlink, "Winlink"),
            (SettingsTab::Servers, "Servers"),
            (SettingsTab::TrSwitch, "T/R switch"),
        ];
        // Next to Servers: the two are the same subject from opposite ends —
        // what this station offers others, and where this screen goes.
        #[cfg(not(target_arch = "wasm32"))]
        tabs.push((SettingsTab::Remote, "Remote"));
        tabs.push((SettingsTab::Tle, "TLE"));
        // Wrapped: the tab strip no longer fits the window's width on one line.
        // Real tabs rather than chips — a chip strip standing in for a tab strip
        // reads as a row of buttons that happen to stay pressed, with nothing to
        // say the page below belongs to the one that is lit.
        crate::chrome::tab_bar(ui, |ui, bar| {
            for (t, label) in tabs {
                if bar.tab(ui, *io.tab == t, label).clicked() {
                    *io.tab = t;
                }
            }
        });
        // No separator: the strip's own baseline is the line between the tabs
        // and the page they open.
        ui.add_space(8.0);

        let backend = io.radio_edit.as_ref().map(|c| c.backend);

        match io.tab {
            SettingsTab::General => {
                // Which build this is, taken from the crate metadata at compile
                // time — so a bug report can name the version without the
                // operator having to find the binary.
                ui.label(
                    RichText::new(format!("SDRoxide {}", env!("CARGO_PKG_VERSION")))
                        .size(15.0)
                        .strong(),
                );
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);

                ui.label(RichText::new("Station").size(14.0).strong().color(crate::theme::CYAN()));
                ui.add_space(6.0);
                if !io.digi_seeded {
                    ui.label(
                        RichText::new(
                            "Enter a digital mode (FT8 / SSTV / …) once to load the saved values.",
                        )
                        .weak(),
                    );
                }
                ui.add_enabled_ui(io.digi_seeded, |ui| {
                    egui::Grid::new("general-grid").num_columns(2).spacing([12.0, 8.0]).show(
                        ui,
                        |ui| {
                            ui.label("Callsign");
                            if crate::chrome::field(
                                ui,
                                egui::TextEdit::singleline(&mut io.digi_edit.my_call),
                            )
                            .changed()
                            {
                                io.digi_edit.my_call = io.digi_edit.my_call.to_uppercase();
                            }
                            ui.end_row();
                            ui.label("Grid square");
                            crate::chrome::field(
                                ui,
                                egui::TextEdit::singleline(&mut io.digi_edit.my_grid),
                            );
                            ui.end_row();
                        },
                    );
                });
                ui.add_space(6.0);
                ui.label(
                    RichText::new(
                        "Your callsign and grid, shared across FT8/FT4/FT2, SSTV image headers, and \
                         the logbook. Also editable from the FT8 / SSTV setup dialog.",
                    )
                    .weak(),
                );

                // Its own grid, outside the enabled-ui above: the region comes
                // from `config.toml` by way of the station announcement, not
                // from the digi config, so it is neither waiting on the same
                // seeding nor sent with the same command.
                ui.add_space(8.0);
                egui::Grid::new("general-region-grid").num_columns(2).spacing([12.0, 8.0]).show(
                    ui,
                    |ui| {
                        ui.label("IARU region");
                        region_combo(ui, io.region_edit);
                        ui.end_row();
                    },
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!(
                        "Where the station is. Sets every band plan: the band edges, the CW / \
                         data / phone sub-segments on the waterfall strip, where the skimmers \
                         listen, the calling frequencies offered, and what counts as out of band \
                         for transmit. In Region {} that makes 70 cm {}. Takes effect at once, \
                         and applies to every radio at this station.",
                        io.region_edit.number(),
                        match io.region_edit {
                            sdroxide_types::Region::R1 =>
                                "430–440 MHz — so 446 is out of band, and 40 m stops at 7.200",
                            sdroxide_types::Region::R2 => "420–450 MHz, and 40 m runs to 7.300",
                            sdroxide_types::Region::R3 => "430–450 MHz, and 80 m stops at 3.900",
                        }
                    ))
                    .weak(),
                );
                ui.add_space(6.0);
                self.settings_band_plan_file(ui, cmds);

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);
                self.settings_swr_guard(ui, cmds);

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);
                self.settings_user_audio(ui, io.audio_pick);
                if let Some(cfg) = io.radio_edit.as_mut() {
                    crate::app::settings::general::settings_rx_audio_gain(ui, cfg);
                }
                // The radio's own sound card is only used by the CAT / Audio
                // interface; every other backend carries its audio in-band.
                //
                // These are the cards on the machine the *rig* is plugged into,
                // asked for by name rather than taken from `audio_devices` —
                // that list is this screen's own speaker and microphone, and
                // offering a laptop's built-in mic as the shack transceiver's
                // transmit path would be worse than offering nothing at all.
                if backend == Some(Backend::Cat) && self.radio_audio_devices.is_none() {
                    ui.add_space(8.0);
                    ui.label(RichText::new("Radio audio (sound card)").strong());
                    ui.label(
                        RichText::new(
                            "Waiting for the sound cards on the machine the radio is plugged \
                             into.",
                        )
                        .weak(),
                    );
                }
                if backend == Some(Backend::Cat)
                    && let (Some((inputs, outputs)), Some(cfg)) =
                        (self.radio_audio_devices.as_ref(), io.radio_edit.as_mut())
                {
                    ui.add_space(8.0);
                    ui.label(RichText::new("Radio audio (sound card)").strong());
                    egui::Grid::new("radio-audio").num_columns(2).spacing([12.0, 6.0]).show(
                        ui,
                        |ui| {
                            let (ci, co) =
                                (cfg.radio_audio_in.clone(), cfg.radio_audio_out.clone());
                            ui.label("From radio (RX)");
                            device_combo(ui, "r-in", inputs, &ci, |n| cfg.radio_audio_in = n);
                            ui.end_row();
                            ui.label("To radio (TX)");
                            device_combo(ui, "r-out", outputs, &co, |n| cfg.radio_audio_out = n);
                            ui.end_row();
                        },
                    );
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if ui
                            .button("Apply / reconnect")
                            .on_hover_text("Reopen the CAT rig with these sound cards — no restart")
                            .clicked()
                        {
                            *io.apply_iface = true;
                        }
                        ui.add(
                            egui::Label::new(
                                RichText::new("Reconnects the radio without restarting.").weak(),
                            )
                            .wrap(),
                        );
                    });
                }

                if let Some(access) = io.access_edit.as_deref_mut() {
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(6.0);
                    remote_access_settings(ui, access);
                }
            }
            SettingsTab::Radio => {
                // The station's radios, managed from here: this page is where
                // radios are configured, and — with the main window's tab area
                // hidden until there is more than one — where the second radio
                // is added in the first place.
                self.settings_radio_roster(ui, io.radio_tabs, io.radio_name_edit);
                let Some(cfg) = io.radio_edit.as_mut() else {
                    // A remote client cannot choose the server's interface or
                    // edit its config — that file lives on the other machine.
                    // The hardware controls are a different matter: they ride
                    // ordinary commands to the running device, and both the
                    // capabilities and the current gains/antennas are already
                    // replicated here, so an operator away from the shack can
                    // still swap to the beam or wind the LNA back.
                    self.settings_device_tab(ui, cmds);
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(
                            "Which radio interface the server uses is set on the machine it \
                             runs on.",
                        )
                        .weak(),
                    );
                    return;
                };
                // The single "which radio interface" selector, and the converter
                // that may sit in front of whichever one is chosen.
                let backend = cfg.backend;
                let converter = io.converter_hz.get_or_insert(cfg.converter_offset_hz);
                let ranges = io.ranges.get_or_insert_with(|| {
                    (
                        sdroxide_types::format_freq_ranges(&cfg.freq_ranges_rx),
                        sdroxide_types::format_freq_ranges(&cfg.freq_ranges_tx),
                    )
                });
                let site = io.rx_site.get_or_insert_with(|| cfg.rx_site.clone());
                egui::Grid::new("iface-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                    ui.label(RichText::new("Radio interface").strong());
                    // Switching the far end's interface is allowed: the device
                    // lists below come from that machine, so this is a choice
                    // between radios that are actually there, and a reopen that
                    // fails leaves the old one running. What it needs is those
                    // lists — with nothing to choose from there is nothing to
                    // choose, so the row is a label until they can be had.
                    if io.can_probe {
                        // Chosen into a copy and applied through `set_backend`,
                        // never straight into the field: the stated tuning
                        // ranges belong to the interface they were typed for,
                        // and moving a tab to another device has to leave them
                        // behind with it. Issue #254 is what the field
                        // assignment cost — a public SpyServer's 200-350 MHz
                        // still clamping the dial after the tab was switched
                        // back to an IC-9700 over the LAN.
                        let mut chosen = cfg.backend;
                        enum_combo(ui, "iface", &mut chosen, io.iface_opts, Backend::label);
                        if chosen != cfg.backend {
                            cfg.set_backend(chosen);
                            // The two boxes below are showing what was typed
                            // for the interface that has just been left. Reseed
                            // them from the one now selected, or Apply would
                            // write one radio's ranges onto another.
                            *ranges = (
                                sdroxide_types::format_freq_ranges(&cfg.freq_ranges_rx),
                                sdroxide_types::format_freq_ranges(&cfg.freq_ranges_tx),
                            );
                            // Same reseed for the same reason: `set_backend`
                            // has just put the antenna back at the station,
                            // because where it was belonged to the receiver
                            // being left. The box has to show that, or Apply
                            // would write the old receiver's square back.
                            *site = cfg.rx_site.clone();
                        }
                    } else {
                        ui.label(backend.label()).on_hover_text(
                            "Waiting for the machine the radio is attached to. Its interface \
                             settings are below and can be changed from here meanwhile.",
                        );
                    }
                    ui.end_row();

                    ui.label(RichText::new("Converter").strong());
                    let named = sdroxide_types::converter_preset_name(*converter);
                    egui::ComboBox::from_id_salt("converter-preset")
                        .selected_text(named)
                        .show_styled(ui, |ui| {
                            for (name, hz) in sdroxide_types::CONVERTER_PRESETS {
                                if ui.selectable_label(named == name, name).clicked() {
                                    *converter = hz;
                                }
                            }
                            // Whatever the operator typed. There is no mode to
                            // unlock — the box beside this one is always
                            // editable — so the entry says what it is and
                            // cannot be picked: clicking it and watching
                            // nothing happen is what it used to do.
                            ui.add_enabled(
                                false,
                                egui::Button::selectable(
                                    named == "Manual",
                                    "Manual — type an offset below",
                                ),
                            )
                            .on_disabled_hover_text(
                                "What this box reads when the offset is not one of the presets. \
                                 Set it by typing in the Offset field below; there is nothing to \
                                 select here.",
                            );
                        })
                        .response
                        .on_hover_text(
                            "A frequency converter in front of the receiver. Pick one, or type \
                             an offset beside it.",
                        );
                    ui.end_row();

                    ui.label(RichText::new("Offset").strong());
                    ui.add(
                        egui::DragValue::new(converter)
                            .speed(1.0)
                            .range(
                                -sdroxide_types::CONVERTER_OFFSET_MAX_HZ
                                    ..=sdroxide_types::CONVERTER_OFFSET_MAX_HZ,
                            )
                            .max_decimals(0)
                            .suffix(" Hz"),
                    )
                    .on_hover_text(
                        "How far a converter moves the signal on its way to the receiver, in Hz \
                         — the same number and sign every converter's documentation and every \
                         other SDR program states. The radio ends up on dial + offset: positive \
                         for an upconverter (a Ham It Up is 125000000), negative for anything \
                         that brings a band down to an I.F. — a satellite LNB, or a 2 m \
                         transverter on a 28 MHz I.F., which is -116000000 (28 − 144). \
                         0 = no converter.\n\nDrag to trim it a hertz at a time, which is what \
                         a converter whose oscillator is slightly off wants.\n\nThis is the \
                         receive path. What is in the transmit line is the row below.\n\nTakes \
                         effect on Apply.",
                    );
                    ui.end_row();

                    // A converter is a receive accessory, so what the transmit
                    // line does is a separate fact about the station — and one
                    // only its operator knows. Withdrawing transmit is the safe
                    // default, but it is not the answer for the station that
                    // hears through a converter and transmits around it: a
                    // QO-100 operator receives 10 GHz through an LNB and puts
                    // 2.4 GHz straight out of the radio.
                    ui.label(RichText::new("Transmit").strong());
                    // Greyed out with no converter set, because that is exactly
                    // when it decides nothing: the transmit path was never
                    // touched to begin with.
                    ui.add_enabled_ui(*converter != 0.0, |ui| {
                        ui.horizontal(|ui| {
                            use sdroxide_types::ConverterTx as Tx;
                            let tx = &mut cfg.converter_tx;
                            egui::ComboBox::from_id_salt("converter-tx")
                                .selected_text(tx.label())
                                .show_styled(ui, |ui| {
                                    for opt in [Tx::Off, Tx::Transverter, Tx::Own(0.0)] {
                                        // Matched on the *kind*, so choosing "its
                                        // own offset" again does not wipe the
                                        // number already typed beside it.
                                        let on = std::mem::discriminant(&opt)
                                            == std::mem::discriminant(tx);
                                        if ui.selectable_label(on, opt.label()).clicked() && !on {
                                            *tx = opt;
                                        }
                                    }
                                })
                                .response
                                .on_hover_text(
                                    "What is in the transmit line while a converter is set.\n\n\
                                     Off while converting: nothing is transmitted — the safe \
                                     default, and right for a receive-only accessory.\n\n\
                                     Through the same converter: one box converts both ways, a \
                                     transverter. Transmit takes the offset above and follows it \
                                     when it is trimmed.\n\nIts own offset: the transmit line is \
                                     different from the receive one. Nought is the common case — \
                                     a converter (an LNB, a Ham It Up) on receive with the \
                                     transmitter on its own antenna, which is the QO-100 station.\
                                     \n\nThe amateur-band check still applies, on the frequency \
                                     you are transmitting on.\n\nTakes effect on Apply.",
                                );
                            if let Tx::Own(hz) = tx {
                                ui.add(
                                    egui::DragValue::new(hz)
                                        .speed(1.0)
                                        .range(
                                            -sdroxide_types::CONVERTER_OFFSET_MAX_HZ
                                                ..=sdroxide_types::CONVERTER_OFFSET_MAX_HZ,
                                        )
                                        .max_decimals(0)
                                        .suffix(" Hz"),
                                )
                                .on_hover_text(
                                    "How far the transmit line moves the signal, in Hz, on the \
                                     same sign rule as the receive offset: the radio transmits at \
                                     dial + offset.\n\n0 = the transmitter is wired to its own \
                                     antenna and works on the frequency the dial says — a QO-100 \
                                     station's 2.4 GHz uplink.\n\nA transmit converter that \
                                     takes an I.F. *up* is a negative number: the radio works \
                                     below the dial.",
                                );
                            }
                        });
                    });
                    ui.end_row();

                    // The ranges the operator says this radio has, for a driver
                    // that publishes none (SoapySX and friends implement no
                    // frequency-range call at all, which leaves nothing to
                    // check a frequency against) or publishes the tuner chip's
                    // rather than the radio's.
                    ui.label(RichText::new("RX range").strong());
                    freq_range_edit(
                        ui,
                        "rx-range",
                        &mut ranges.0,
                        "Which frequencies this radio receives, in MHz: 144-146, 430-440. Leave \
                         empty to use whatever the device reports about itself.\n\nBand buttons \
                         outside the range are greyed out and the dial will not go there.\n\n\
                         These are dial frequencies. With a converter set, state the band you \
                         tune — 144-148 for a 2 m transverter — not the I.F. the radio is really \
                         on.\n\nTakes effect on Apply.",
                    );
                    ui.end_row();

                    ui.label(RichText::new("TX range").strong());
                    freq_range_edit(
                        ui,
                        "tx-range",
                        &mut ranges.1,
                        "Which frequencies this radio transmits on, in MHz: 144-146, 430-440. \
                         Leave empty to use whatever the device reports — and if it reports \
                         nothing, the driver is taken at its word and any frequency is \
                         allowed.\n\nDial frequencies, like the receive range above.\n\nThis \
                         is a limit you set, not a licence: transmitting outside \
                         the amateur bands is refused regardless unless you have turned that off \
                         in config.toml. Nor does it give a receive-only device a \
                         transmitter.\n\nTakes effect on Apply.",
                    );
                    ui.end_row();

                    // Where the antenna is. The station's locator answers that
                    // for a radio in the shack and answers it wrongly for an
                    // online receiver — issue #284, where a KiwiSDR in
                    // Australia taken in a European tab posted every 2 m decode
                    // as an intercontinental opening, because the reports went
                    // out from the operator's square.
                    ui.label(RichText::new("Antenna is").strong());
                    ui.horizontal(|ui| {
                        use sdroxide_types::RxSite;
                        egui::ComboBox::from_id_salt("rx-site")
                            .selected_text(site.label())
                            .show_styled(ui, |ui| {
                                for opt in [RxSite::Station, RxSite::Elsewhere(String::new())] {
                                    // On the *kind*, so picking "somewhere
                                    // else" again does not wipe the locator
                                    // already typed beside it.
                                    let on = std::mem::discriminant(&opt)
                                        == std::mem::discriminant(&*site);
                                    if ui.selectable_label(on, opt.label()).clicked() && !on {
                                        *site = opt;
                                    }
                                }
                            })
                            .response
                            .on_hover_text(
                                "Where this radio listens from.\n\nAt the station: the antenna \
                                 is yours, so your own locator says where it is. The default, \
                                 and right for everything in the shack.\n\nSomewhere else: an \
                                 online receiver, or your own set up on a hilltop. What it hears \
                                 is reported to PSK Reporter, WSPRnet and FreeDV Reporter from \
                                 the locator beside this, never from yours — and ADS-B places \
                                 aircraft against it too.\n\nPicking a receiver under \
                                 \"Public SDRs\" fills this in for you.\n\nTakes effect on \
                                 Apply.",
                            );
                        if let RxSite::Elsewhere(g) = site {
                            ui.add(
                                egui::TextEdit::singleline(g)
                                    .desired_width(90.0)
                                    .hint_text("locator"),
                            )
                            .on_hover_text(
                                "The receiver's Maidenhead locator — JN88ec, DO30db. Four or \
                                 six characters.\n\nLeave it empty if you do not know where \
                                 the receiver is: nothing is then reported at all, which is the \
                                 only honest answer. Reporting from your own square would put \
                                 somebody else's reception on the wrong continent.",
                            );
                        }
                    });
                    ui.end_row();
                });
                if let sdroxide_types::RxSite::Elsewhere(g) = &*site {
                    let g = g.trim();
                    ui.label(
                        RichText::new(if g.is_empty() {
                            "The antenna is somewhere else and no locator is set, so nothing \
                             this radio hears is reported to PSK Reporter, WSPRnet or FreeDV \
                             Reporter. Type the receiver's locator above to report from it."
                                .to_string()
                        } else if sdroxide_types::grid_to_latlon(g).is_some() {
                            format!(
                                "Receptions through this radio are reported from {g}, not from \
                                 your own locator."
                            )
                        } else {
                            format!(
                                "{g} is not a Maidenhead locator, so nothing this radio hears is \
                                 reported. Four or six characters: two letters, two digits, and \
                                 optionally two more letters."
                            )
                        })
                        .weak(),
                    );
                }
                // The two range boxes are the only megahertz on a tab whose
                // other frequency field is hertz, so the example says the same
                // range both ways rather than leaving anyone to count zeros.
                ui.label(
                    RichText::new(
                        "Ranges are in MHz, low-high, separated by commas: 144-146, 430-440 — \
                         that is 144000000-146000000 Hz and 430000000-440000000 Hz. They are \
                         dial frequencies: a converter offset does not move them. The \
                         converter offset above is the field in hertz. Leave a range empty to \
                         use whatever the device reports about itself; a device that reports \
                         nothing is taken at its word.",
                    )
                    .weak(),
                );
                if *converter != 0.0 {
                    // What the two rows above add up to, in one sentence: the
                    // transmit answer is the one an operator gets wrong, and a
                    // radio that will not key is the symptom either way.
                    let tx = match cfg.converter_tx {
                        sdroxide_types::ConverterTx::Off => {
                            "Transmit is off while a converter is set — say what is in the \
                             transmit line above to turn it back on."
                        }
                        sdroxide_types::ConverterTx::Transverter => {
                            "Transmit goes through the same converter, so the radio works the \
                             same offset away from the dial in both directions."
                        }
                        sdroxide_types::ConverterTx::Own(0.0) => {
                            "Transmit is not converted: the radio transmits on the frequency the \
                             dial shows, while receive comes through the converter."
                        }
                        sdroxide_types::ConverterTx::Own(_) => {
                            "Transmit takes its own offset, separate from the receive one."
                        }
                    };
                    ui.label(RichText::new(tx).weak());
                    if backend == Backend::RtlSdr && *converter < 0.0 {
                        ui.label(
                            RichText::new(
                                "Careful on an RTL-SDR: the Blog V4 upconverts on its own below \
                                 28.8 MHz, so a negative offset that lands the hardware there \
                                 shifts twice.",
                            )
                            .weak(),
                        );
                    }
                }

                transverter_table(ui, cfg);

                self.settings_panadapter(ui, cfg);

                // Transmit EQ: mic/modulator-path DSP, not a device feature.
                // It applies the same way regardless of which interface is
                // selected above, so it sits here rather than in one of the
                // per-backend sections below.
                if self.tx_capable() {
                    drive_trim_table(ui, cfg);
                    ui.separator();
                    ui.label(
                        RichText::new("Transmit EQ")
                            .size(14.0)
                            .strong()
                            .color(crate::theme::CYAN()),
                    );
                    ui.add_space(4.0);
                    crate::chrome::checkbox(ui, &mut io.tx_eq_edit.enabled, "Enabled").on_hover_text(
                        "A 3-band parametric EQ on the microphone audio, ahead of the modulator. \
                         Voice modes only (SSB/AM/FM); digital modes and CW carry synthesized or \
                         keyed audio that never reaches it. Off by default and flat when turned \
                         on, so enabling it changes nothing on the air until a band below is \
                         actually moved. Applies immediately.",
                    );
                    ui.add_space(4.0);
                    egui::Grid::new("tx-eq-grid").num_columns(4).spacing([10.0, 6.0]).show(
                        ui,
                        |ui| {
                            ui.label("");
                            ui.label(RichText::new("Freq").weak());
                            ui.label(RichText::new("Gain").weak());
                            ui.label(RichText::new("Q / Slope").weak());
                            ui.end_row();

                            tx_eq_band_row(
                                ui,
                                "Low shelf",
                                &mut io.tx_eq_edit.low,
                                sdroxide_types::TxEqState::LOW_FREQ_HZ_RANGE,
                                sdroxide_types::TxEqState::SHELF_SLOPE_RANGE,
                                "Cuts/boosts rumble and handling noise.",
                            );
                            tx_eq_band_row(
                                ui,
                                "Mid peak",
                                &mut io.tx_eq_edit.mid,
                                sdroxide_types::TxEqState::MID_FREQ_HZ_RANGE,
                                sdroxide_types::TxEqState::MID_Q_RANGE,
                                "Presence: a narrow bump or dip somewhere in the voice band.",
                            );
                            tx_eq_band_row(
                                ui,
                                "High shelf",
                                &mut io.tx_eq_edit.high,
                                sdroxide_types::TxEqState::HIGH_FREQ_HZ_RANGE,
                                sdroxide_types::TxEqState::SHELF_SLOPE_RANGE,
                                "Brightness/de-ess.",
                            );
                        },
                    );
                }
                ui.separator();

                // Read out before the match: the arms below hand `io.radio_edit`
                // to a tab function, which cannot happen while `cfg` — a borrow
                // of that same field — is still alive.
                let backend = cfg.backend;
                match backend {
                    Backend::Soapy => {
                        self.settings_device_tab(ui, cmds);
                        ui.add_space(4.0);
                        settings_soapy_tab(
                            ui,
                            self.caps.as_ref(),
                            io.radio_edit,
                            io.apply_iface,
                            cmds,
                        );
                        ui.add_space(4.0);
                        settings_soapy_devices(
                            ui,
                            self.soapy_devices.as_deref(),
                            io.soapy_rescan,
                            io.can_probe,
                        );
                    }
                    Backend::Hpsdr => settings_hpsdr_tab(
                        ui,
                        &self.hpsdr_devices,
                        io.radio_edit,
                        io.hpsdr_discover,
                        io.can_probe,
                        cmds,
                    ),
                    Backend::Cat => settings_cat_tab(
                        ui,
                        &self.serial_ports,
                        io.radio_edit,
                        self.caps.as_ref(),
                        &self.state.antenna_rx,
                        io.can_probe,
                        cmds,
                    ),
                    Backend::Tci => settings_tci_tab(
                        ui,
                        io.radio_edit,
                        io.tci_test,
                        &self.tci_test_result,
                        io.can_probe,
                    ),
                    Backend::IcomNet => settings_icomnet_tab(
                        ui,
                        io.radio_edit,
                        self.caps.as_ref(),
                        &self.state.antenna_rx,
                        io.icomnet_test,
                        io.icomnet_copy_report,
                        &self.icomnet_test_result,
                        io.can_probe,
                        cmds,
                    ),
                    Backend::SmartSdr => settings_smartsdr_tab(
                        ui,
                        &self.smartsdr_devices,
                        io.radio_edit,
                        io.smartsdr_discover,
                        io.smartsdr_test,
                        io.smartsdr_copy_report,
                        &self.smartsdr_test_result,
                        io.can_probe,
                    ),
                    Backend::Pluto => settings_pluto_tab(
                        ui,
                        &self.pluto_devices,
                        io.radio_edit,
                        io.pluto_discover,
                        io.pluto_test,
                        io.pluto_copy_report,
                        &self.pluto_test_result,
                        io.can_probe,
                        cmds,
                    ),
                    Backend::RtlSdr => settings_rtlsdr_tab(
                        ui,
                        &self.rtlsdr_devices,
                        io.radio_edit,
                        io.rtlsdr_rescan,
                        io.can_probe,
                        cmds,
                    ),
                    // No device list: the dongle is the server's, and the
                    // protocol has no enumeration — there is nothing to rescan
                    // and nothing to pick from.
                    Backend::RtlTcp => settings_rtltcp_tab(ui, io.radio_edit, cmds),
                    // Likewise no device list: the receiver is the server's,
                    // and the protocol has no enumeration. What it does have
                    // is an answer — the Test button reads it back.
                    Backend::SpyServer | Backend::SpyServerVfo => settings_spyserver_tab(
                        ui,
                        io.radio_edit,
                        backend == Backend::SpyServerVfo,
                        io.spyserver_test,
                        &self.spyserver_test_result,
                        io.can_probe,
                        cmds,
                    ),
                    Backend::KiwiSdr => settings_kiwisdr_tab(
                        ui,
                        io.radio_edit,
                        io.kiwi_test,
                        &self.kiwi_test_result,
                        io.can_probe,
                        cmds,
                    ),
                    Backend::Rx888 => settings_rx888_tab(
                        ui,
                        &self.rx888_devices,
                        io.radio_edit,
                        io.rx888_rescan,
                        io.apply_iface,
                        io.can_probe,
                        self.state.rx_freq_hz(),
                        cmds,
                    ),
                    Backend::Elad => settings_elad_tab(
                        ui,
                        &self.elad_devices,
                        &self.serial_ports,
                        &self.state.antenna_rx,
                        self.radio_audio_devices.as_ref().map(|(_, o)| o.as_slice()),
                        io.radio_edit,
                        io.elad_rescan,
                        io.elad_copy_report,
                        io.can_probe,
                        cmds,
                    ),
                    Backend::Lime => settings_lime_tab(
                        ui,
                        &self.lime_devices,
                        &self.serial_ports,
                        io.radio_edit,
                        io.lime_rescan,
                        io.lime_copy_report,
                        io.apply_iface,
                        io.can_probe,
                        cmds,
                    ),
                    Backend::AirspyHf => settings_airspyhf_tab(
                        ui,
                        &self.airspyhf_devices,
                        self.caps.as_ref(),
                        io.radio_edit,
                        io.airspyhf_rescan,
                        io.airspyhf_copy_report,
                        io.apply_iface,
                        io.can_probe,
                        cmds,
                    ),
                    Backend::Airspy => settings_airspy_tab(
                        ui,
                        &self.airspy_devices,
                        self.caps.as_ref(),
                        io.radio_edit,
                        io.airspy_rescan,
                        io.airspy_copy_report,
                        io.apply_iface,
                        io.can_probe,
                        cmds,
                    ),
                    Backend::HydraSdr => settings_hydrasdr_tab(
                        ui,
                        &self.hydrasdr_devices,
                        self.caps.as_ref(),
                        io.radio_edit,
                        io.hydrasdr_rescan,
                        io.hydrasdr_copy_report,
                        io.apply_iface,
                        io.can_probe,
                        cmds,
                    ),
                    Backend::HackRf => settings_hackrf_tab(
                        ui,
                        &self.hackrf_devices,
                        io.radio_edit,
                        io.hackrf_rescan,
                        io.hackrf_copy_report,
                        io.apply_iface,
                        io.can_probe,
                        cmds,
                    ),
                    Backend::SdrPlay => settings_sdrplay_tab(
                        ui,
                        &self.sdrplay_devices,
                        self.caps.as_ref(),
                        io.radio_edit,
                        io.sdrplay_rescan,
                        io.apply_iface,
                        io.can_probe,
                        cmds,
                    ),
                    Backend::Fobos => settings_fobos_tab(
                        ui,
                        &self.fobos_devices,
                        self.caps.as_ref(),
                        io.radio_edit,
                        io.fobos_rescan,
                        io.apply_iface,
                        io.can_probe,
                        cmds,
                    ),
                    // Legacy configs may still carry the removed auto-detect
                    // backend; prompt the user to pick a concrete interface.
                    // Which nobody can do from a remote client — the row above
                    // is a label there — so say who has to.
                    Backend::Auto => {
                        ui.label(
                            RichText::new(if io.can_probe {
                                "Pick a radio interface above (this configuration used the \
                                 removed auto-detect mode)."
                            } else {
                                "This configuration used the removed auto-detect mode. Pick an \
                                 interface above once the machine the radio is attached to has \
                                 answered."
                            })
                            .weak(),
                        );
                    }
                    // A freshly added radio tab: nothing opens until an
                    // interface is chosen, so choosing one is the whole page.
                    Backend::None => {
                        ui.label(
                            RichText::new(if io.can_probe {
                                "This radio has no interface yet — pick one above and press \
                                 Apply / reconnect."
                            } else {
                                "This radio has no interface yet. One can be picked above once \
                                 the machine the radio is attached to has answered."
                            })
                            .weak(),
                        );
                    }
                }
                ui.separator();
                ui.horizontal(|ui| {
                    // The same button either way: it reopens whichever radio
                    // the engine has, on the settings just edited — including a
                    // different interface, which from a remote client is how a
                    // headless station is moved from one device to another.
                    if ui
                        .button("Apply / reconnect")
                        .on_hover_text(if io.local_engine {
                            "Switch to this interface now — no restart needed"
                        } else {
                            "Switch the server's radio to this interface now — no restart \
                             needed. The old one keeps running if the new one cannot be opened."
                        })
                        .clicked()
                    {
                        *io.apply_iface = true;
                    }
                    // Labels in a horizontal row default to Extend; wrap so a
                    // narrow window doesn't push this under the scrollbar.
                    ui.add(
                        egui::Label::new(
                            RichText::new(if io.local_engine {
                                "Switches the live radio without restarting."
                            } else {
                                "Everything above is the server's own configuration, and \
                                 changes are saved there. Most settings apply as you change \
                                 them; the ones fixed when the device is opened — the sample \
                                 rate, an address, the interface itself — need this button."
                            })
                            .weak(),
                        )
                        .wrap(),
                    );
                });
            }
            SettingsTab::Ui => {
                settings_ui_tab(ui, io.ui_edit, io.solar_cloud_march.as_deref_mut());
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);
                ui_tab::speech_settings(
                    ui,
                    io.speech_edit,
                    io.speech_voices,
                    self.audio_devices.as_ref().map(|d| d.outputs.as_slice()).unwrap_or(&[]),
                    io.speech_status,
                    io.speech_test,
                );
            }
            SettingsTab::Spots => {
                if !net_seeded_note(ui, io.net_seeded) {
                    return;
                }
                operator_identity_note(ui, io.digi_edit, io.digi_seeded);

                net_heading(ui, "DX cluster (telnet)");
                crate::chrome::checkbox(ui, &mut io.net_edit.cluster.enabled, "Enabled");
                net_row(ui, "Host", &mut io.net_edit.cluster.host, 220.0);
                ui.horizontal(|ui| {
                    ui.add_sized([96.0, 22.0], egui::Label::new("Port"));
                    ui.add(egui::DragValue::new(&mut io.net_edit.cluster.port).range(1..=65535));
                });
                net_row(ui, "Login call", &mut io.net_edit.cluster.login, 140.0);
                ui.horizontal(|ui| {
                    ui.add_sized([96.0, 22.0], egui::Label::new("Commands"));
                    crate::chrome::field(
                        ui,
                        egui::TextEdit::multiline(io.net_cmds)
                            .desired_rows(2)
                            .hint_text("one per line, e.g. SET/FT8")
                            .desired_width(220.0),
                    );
                });

                net_heading(ui, "Reverse Beacon Network");
                crate::chrome::checkbox(ui, &mut io.net_edit.rbn.enabled, "Enabled").on_hover_text(
                    "Read the world's CW/RTTY skimmers and feed the propagation map with \
                     them. This is what makes the map show bands this radio is not \
                     listening to. On by default: it puts nothing on the air, needs no \
                     account, and uses the callsign from the General tab. RBN spots do not \
                     appear in the spot list — they are measurements, not invitations.",
                );
                net_row(ui, "Host", &mut io.net_edit.rbn.host, 220.0);
                ui.horizontal(|ui| {
                    ui.add_sized([96.0, 22.0], egui::Label::new("Port"));
                    ui.add(egui::DragValue::new(&mut io.net_edit.rbn.port).range(1..=65535))
                        .on_hover_text("7000 is the CW/RTTY feed, 7001 the FT8/FT4 one");
                });
                net_row(ui, "Login call", &mut io.net_edit.rbn.login, 140.0);
                ui.horizontal(|ui| {
                    ui.add_sized([96.0, 22.0], egui::Label::new("Commands"));
                    crate::chrome::field(
                        ui,
                        egui::TextEdit::multiline(io.rbn_cmds)
                            .desired_rows(2)
                            .hint_text("one per line, e.g. set/filter cont=eu")
                            .desired_width(220.0),
                    )
                    .on_hover_text(
                        "Sent after login. The place to narrow the feed — without a filter \
                         this is every skimmer on Earth.",
                    );
                });
                ui.label(
                    egui::RichText::new(
                        "RBN paths are placed from country centres, not locators — accurate \
                         for a small country, out by a long way for a large one. They are a \
                         separate layer on the propagation map and can be switched off there.",
                    )
                    .size(9.5)
                    .italics()
                    .color(crate::theme::gray(110)),
                );

                net_heading(ui, "POTA / SOTA / PSK Reporter");
                crate::chrome::checkbox(ui, &mut io.net_edit.pota.enabled, "POTA activator spots");
                crate::chrome::checkbox(ui, &mut io.net_edit.sota.enabled, "SOTA spots");
                crate::chrome::checkbox(
                    ui,
                    &mut io.net_edit.psk.enabled,
                    "PSK Reporter (current band)",
                );
                crate::chrome::checkbox(
                    ui,
                    &mut io.net_edit.psk.report,
                    "Upload my FT8/FT4/FT2 decodes",
                )
                .on_hover_text(
                    "Report what this station hears to pskreporter.info, so it appears \
                         there as a receiver. Uses the callsign and grid from the General tab.",
                );
                if io.net_edit.psk.report {
                    net_row(ui, "Antenna", &mut io.net_edit.psk.antenna, 200.0);
                    ui.horizontal(|ui| {
                        ui.add_sized([96.0, 22.0], egui::Label::new("Collector"));
                        crate::chrome::field(
                            ui,
                            egui::TextEdit::singleline(&mut io.net_edit.psk.host)
                                .desired_width(140.0),
                        );
                        ui.add(egui::DragValue::new(&mut io.net_edit.psk.port).range(1..=65535))
                            .on_hover_text("4739 is the live collector, 14739 the test one");
                    });
                }
                ui.horizontal(|ui| {
                    ui.add_sized([96.0, 22.0], egui::Label::new("Max age (s)"));
                    let age = &mut io.net_edit.spot_max_age_secs;
                    ui.add(egui::DragValue::new(age).range(60..=7200));
                });

                net_heading(ui, "WSPRnet");
                crate::chrome::checkbox(ui, &mut io.net_edit.wspr.upload, "Upload my WSPR decodes")
                    .on_hover_text(
                        "Send every WSPR reception to wsprnet.org. On by default: it puts \
                         nothing on the air, and reporting what you hear is what makes a WSPR \
                         receiver part of the network rather than a private curiosity. A slot \
                         that decoded nothing is reported too, which is how the network tells a \
                         shut band from a receiver that was switched off.",
                    );
                crate::chrome::checkbox(
                    ui,
                    &mut io.net_edit.wspr.download_heard_us,
                    "Download who heard me",
                )
                .on_hover_text(
                    "Ask wsprnet.org which stations decoded this one. WSPR has no \
                         acknowledgement of any kind, so this is the only way a transmitting \
                         beacon learns anything about its own reach.",
                );
                if io.net_edit.wspr.download_heard_us {
                    ui.horizontal(|ui| {
                        ui.add_sized([96.0, 22.0], egui::Label::new("Ask every"));
                        ui.add(
                            egui::DragValue::new(&mut io.net_edit.wspr.download_interval_secs)
                                .range(60..=3600)
                                .suffix(" s"),
                        );
                        ui.add_sized([56.0, 22.0], egui::Label::new("looking back"));
                        ui.add(
                            egui::DragValue::new(&mut io.net_edit.wspr.download_window_min)
                                .range(2..=180)
                                .suffix(" min"),
                        );
                    });
                }

                ui.add_space(8.0);
                if crate::chrome::chip_accent(
                    ui,
                    false,
                    RichText::new(" APPLY ").strong(),
                    crate::theme::GREEN(),
                    crate::theme::INK_ON_CYAN(),
                )
                .on_hover_text("Persist and (re)connect the feeds")
                .clicked()
                {
                    *io.net_apply = true;
                }

                net_heading(ui, "Broadcast stations");
                broadcast_stations_settings(
                    ui,
                    io.bc_reload,
                    io.bc_refetch,
                    io.bc_fetching,
                    io.bc_status,
                );
            }
            SettingsTab::Winlink => {
                if !net_seeded_note(ui, io.net_seeded) {
                    return;
                }
                let wl = &mut io.net_edit.winlink;
                net_heading(ui, "Winlink account");
                net_row(ui, "Callsign", &mut wl.callsign, 140.0);
                net_secret(ui, "Password", &mut wl.password, 140.0);
                ui.label(
                    RichText::new(
                        "The Winlink account password, not the gateway password. It is \
                         case-sensitive — enter it exactly as it was issued.",
                    )
                    .weak(),
                );
                net_row(ui, "Locator", &mut wl.locator, 100.0);

                ui.add_space(6.0);
                net_heading(ui, "How to connect");
                ui.horizontal(|ui| {
                    ui.add_sized([96.0, 22.0], egui::Label::new("Route"));
                    ui.selectable_value(
                        &mut wl.lane,
                        sdroxide_types::WinlinkLane::Telnet,
                        "Internet",
                    )
                    .on_hover_text("Forward with the CMS over the internet");
                    ui.selectable_value(
                        &mut wl.lane,
                        sdroxide_types::WinlinkLane::Packet,
                        "Radio (packet)",
                    )
                    .on_hover_text(
                        "Call an RMS gateway on the air. The radio must be in PACKET or \
                         PACKET-HF.",
                    );
                });

                if wl.lane == sdroxide_types::WinlinkLane::Packet {
                    ui.add_space(4.0);
                    net_row(ui, "Gateway", &mut wl.gateway, 140.0);
                    let mut via = wl.gateway_via.join(" ");
                    ui.horizontal(|ui| {
                        ui.add_sized([96.0, 22.0], egui::Label::new("Via"));
                        if crate::chrome::field_sized(
                            ui,
                            [200.0, 22.0],
                            egui::TextEdit::singleline(&mut via),
                        )
                        .changed()
                        {
                            wl.gateway_via =
                                via.split_whitespace().map(|s| s.to_uppercase()).collect();
                        }
                    });
                    ui.label(
                        RichText::new(
                            "Digipeaters, in order, separated by spaces. Usually empty — a \
                             gateway you can hear directly is a gateway you should call \
                             directly.",
                        )
                        .weak(),
                    );

                    // The gateway's speed, not the operator's: it belongs
                    // beside the callsign it describes rather than with the
                    // modem settings, because nothing about the band predicts
                    // it and only the gateway's owner knows.
                    ui.horizontal(|ui| {
                        ui.add_sized([96.0, 22.0], egui::Label::new("Speed"));
                        for b in [
                            sdroxide_types::PacketBaud::Vhf1200,
                            sdroxide_types::PacketBaud::Vhf9600,
                            sdroxide_types::PacketBaud::Hf300,
                        ] {
                            ui.selectable_value(&mut wl.gateway_baud, b, b.label());
                        }
                    });
                    ui.label(
                        RichText::new(
                            "Most RMS gateways answer at 1200. Calling a 1200 gateway at 9600 \
                             sounds exactly like a gateway that is off the air, so set this \
                             from what the gateway's owner publishes rather than guessing. \
                             9600 also needs the radio's data port at both ends. 300 is HF.",
                        )
                        .weak(),
                    );

                    let mut mhz = wl.gateway_freq_hz / 1e6;
                    ui.horizontal(|ui| {
                        ui.add_sized([96.0, 22.0], egui::Label::new("Frequency"));
                        if ui
                            .add(
                                egui::DragValue::new(&mut mhz)
                                    .speed(0.001)
                                    .range(0.0..=1300.0)
                                    .max_decimals(4)
                                    .suffix(" MHz"),
                            )
                            .changed()
                        {
                            wl.gateway_freq_hz = mhz * 1e6;
                        }
                        if wl.gateway_freq_hz > 0.0 && ui.button("CLEAR").clicked() {
                            wl.gateway_freq_hz = 0.0;
                        }
                    });
                    ui.label(
                        RichText::new(
                            "Zero leaves the dial alone, which is what you want when you park \
                             on one channel. Anything else tunes the radio when the session \
                             starts.",
                        )
                        .weak(),
                    );

                    ui.add_space(6.0);
                    net_heading(ui, "My gateways");
                    ui.label(
                        RichText::new(
                            "Winlink's published gateway list needs an API key sdroxide does \
                             not have, so keep your own. The two or three gateways reachable \
                             from one location are learned by trying, and they rarely change.",
                        )
                        .weak(),
                    );

                    let mut remove = None;
                    let mut pick = None;
                    for (i, g) in wl.gateways.iter().enumerate() {
                        ui.horizontal(|ui| {
                            if ui
                                .button("USE")
                                .on_hover_text("Call this one on the next connect")
                                .clicked()
                            {
                                pick = Some(i);
                            }
                            let freq = if g.freq_hz > 0.0 {
                                format!("{:.4} MHz", g.freq_hz / 1e6)
                            } else {
                                "current dial".to_string()
                            };
                            let via = if g.via.is_empty() {
                                String::new()
                            } else {
                                format!(" via {}", g.via.join(" "))
                            };
                            ui.label(format!(
                                "{}{via} — {freq}, {} baud{}{}",
                                g.callsign,
                                g.baud.label(),
                                if g.label.is_empty() { "" } else { " — " },
                                g.label,
                            ));
                            // Text, not a glyph: this font has no ✕ and draws a
                            // tofu box, which reads as a broken button.
                            if ui.button("FORGET").on_hover_text("Forget this gateway").clicked() {
                                remove = Some(i);
                            }
                        });
                    }
                    if let Some(i) = pick {
                        wl.select_gateway(i);
                    }
                    if let Some(i) = remove {
                        wl.gateways.remove(i);
                    }

                    ui.add_space(4.0);
                    if ui
                        .button("+ ADD GATEWAY")
                        .on_hover_text("Save the gateway above to the list")
                        .clicked()
                        && !wl.gateway.trim().is_empty()
                    {
                        wl.gateways.push(sdroxide_types::WinlinkGateway {
                            callsign: wl.gateway.trim().to_uppercase(),
                            via: wl.gateway_via.clone(),
                            freq_hz: wl.gateway_freq_hz,
                            baud: wl.gateway_baud,
                            label: String::new(),
                        });
                    }
                }

                ui.add_space(6.0);
                net_heading(ui, "Internet gateway");
                net_row(ui, "CMS address", &mut wl.cms_address, 220.0);
                net_row(ui, "Client name", &mut wl.app_name, 140.0);
                ui.label(
                    RichText::new(
                        "Winlink's production servers only accept client names they know, and \
                         answer an unknown one with \"Unknown client types are not allowed on \
                         production servers\". Until sdroxide is registered with the Winlink \
                         Development Team, connecting needs a name they recognise.",
                    )
                    .weak(),
                );

                ui.add_space(6.0);
                net_heading(ui, "Automatic connection");
                crate::chrome::checkbox(ui, &mut wl.auto_connect, "Connect on a timer");
                ui.add_enabled_ui(wl.auto_connect, |ui| {
                    ui.horizontal(|ui| {
                        ui.add_sized([96.0, 22.0], egui::Label::new("Every"));
                        ui.add(
                            egui::DragValue::new(&mut wl.auto_connect_minutes)
                                .range(5..=1440)
                                .suffix(" min"),
                        );
                    });
                });

                // Every network tab commits through its own APPLY: the edits
                // above live in a scratch copy until one is pressed. Without
                // this button the account is typed in, appears to stick, and
                // the engine never hears about it — so connecting reports that
                // no callsign has been set.
                ui.add_space(8.0);
                if crate::chrome::chip_accent(
                    ui,
                    false,
                    RichText::new(" APPLY ").strong(),
                    crate::theme::GREEN(),
                    crate::theme::INK_ON_CYAN(),
                )
                .on_hover_text("Persist the Winlink account")
                .clicked()
                {
                    *io.net_apply = true;
                }
            }
            SettingsTab::Uploads => {
                if !net_seeded_note(ui, io.net_seeded) {
                    return;
                }
                net_heading(ui, "Callsign lookup");
                ui.horizontal(|ui| {
                    ui.add_sized([96.0, 22.0], egui::Label::new("Provider"));
                    egui::ComboBox::from_id_salt("lookup_provider")
                        .selected_text(io.net_edit.lookup_provider.label())
                        .show_styled(ui, |ui| {
                            for p in LookupProvider::ALL {
                                let cur = &mut io.net_edit.lookup_provider;
                                ui.selectable_value(cur, p, p.label());
                            }
                        });
                });
                crate::chrome::checkbox(
                    ui,
                    &mut io.net_edit.auto_lookup,
                    "Auto-fill name/QTH/grid on spot click & QSO",
                );
                // Only the provider in use. Lookups go to exactly one service,
                // so a second pair of boxes below the chosen one is a login
                // nothing will read — and, worse, an invitation to fill it in
                // and wonder why lookups still say the account is not set.
                match io.net_edit.lookup_provider {
                    LookupProvider::None => {
                        ui.label(
                            RichText::new("Choose a provider to enter its login.")
                                .size(10.5)
                                .color(crate::theme::gray(140)),
                        );
                    }
                    LookupProvider::Qrz => {
                        net_row(ui, "QRZ user", &mut io.net_edit.qrz.user, 140.0);
                        net_secret(ui, "QRZ pass", &mut io.net_edit.qrz.password, 140.0);
                    }
                    LookupProvider::HamQth => {
                        net_row(ui, "HamQTH user", &mut io.net_edit.hamqth.user, 140.0);
                        net_secret(ui, "HamQTH pass", &mut io.net_edit.hamqth.password, 140.0);
                        ui.label(
                            RichText::new(
                                "One HamQTH account does both — this login is the one the \
                                 HamQTH upload tab uses.",
                            )
                            .size(10.5)
                            .color(crate::theme::gray(140)),
                        );
                    }
                }

                net_heading(ui, "Upload");
                crate::chrome::checkbox(
                    ui,
                    &mut io.net_edit.auto_upload,
                    "Auto-upload each new QSO",
                )
                .on_hover_text(
                    "The master switch. Which services a QSO goes to is set on each tab below.",
                );
                ui.add_space(4.0);
                // A tab per service, rather than four stacked blocks of
                // credentials: an operator configures the one they have an
                // account with, and the other three are noise they have to read
                // past to find it. Each tab now holds everything about that
                // service — whether it is an auto-upload target, its login, and
                // the button that checks it.
                crate::chrome::tab_bar(ui, |ui, bar| {
                    for t in UploadTarget::ALL {
                        if bar.tab(ui, *io.upload_tab == t, t.label()).clicked() {
                            *io.upload_tab = t;
                        }
                    }
                });
                ui.add_space(6.0);
                let target = *io.upload_tab;
                let enable = match target {
                    UploadTarget::Eqsl => &mut io.net_edit.auto_upload_eqsl,
                    UploadTarget::QrzLogbook => &mut io.net_edit.auto_upload_qrz,
                    UploadTarget::HamQth => &mut io.net_edit.auto_upload_hamqth,
                    UploadTarget::ClubLog => &mut io.net_edit.auto_upload_clublog,
                };
                crate::chrome::checkbox(
                    ui,
                    enable,
                    format!("Auto-upload each new QSO to {}", target.label()),
                );
                if !io.net_edit.auto_upload {
                    ui.label(
                        RichText::new(
                            "Auto-upload is off above, so nothing is pushed automatically \
                             yet — the per-QSO UP button in the logbook still works.",
                        )
                        .size(10.5)
                        .color(crate::theme::gray(140)),
                    );
                }
                ui.add_space(4.0);
                match target {
                    UploadTarget::QrzLogbook => {
                        net_secret(ui, "QRZ log key", &mut io.net_edit.qrz_logbook_key, 200.0);
                        ui.label(
                            RichText::new(
                                "The QRZ logbook API key from your QRZ logbook settings — \
                                 not the XML-lookup login.",
                            )
                            .size(10.5)
                            .color(crate::theme::gray(140)),
                        );
                    }
                    UploadTarget::Eqsl => {
                        net_row(ui, "eQSL user", &mut io.net_edit.eqsl.user, 140.0);
                        net_secret(ui, "eQSL pass", &mut io.net_edit.eqsl.password, 140.0);
                    }
                    UploadTarget::HamQth => {
                        // The *same* two fields as the lookup section above, not
                        // a second pair: HamQTH has one account per operator and
                        // its logbook endpoint authenticates with it, so
                        // whichever box the operator finds first is the right one
                        // to type into. Both pairs are on screen at once when the
                        // lookup provider is HamQTH too, and that is fine — two
                        // `TextEdit`s over one `String` each keep their own
                        // cursor, only the focused one takes input, and the other
                        // redraws with what was typed.
                        net_row(ui, "HamQTH user", &mut io.net_edit.hamqth.user, 140.0);
                        net_secret(ui, "HamQTH pass", &mut io.net_edit.hamqth.password, 140.0);
                        ui.label(
                            RichText::new(
                                "The same login as the HamQTH callsign lookup — filling in \
                                 either one fills in both.",
                            )
                            .size(10.5)
                            .color(crate::theme::gray(140)),
                        );
                    }
                    UploadTarget::ClubLog => {
                        net_row(ui, "Club Log email", &mut io.net_edit.clublog.user, 200.0);
                        net_secret(ui, "Club Log pass", &mut io.net_edit.clublog.password, 140.0);
                        net_secret(ui, "Club Log key", &mut io.net_edit.clublog_api_key, 200.0);
                    }
                }

                // Only where the engine is in this process: the result is not
                // sent to remote clients, so a button there would spin for ever.
                if !self.ctrl.engine_is_remote() {
                    self.login_test_row(ui, cmds, io.net_edit, target.login_target());
                }

                net_heading(ui, "Confirmations (download)");
                net_row(ui, "LoTW user", &mut io.net_edit.lotw.user, 140.0);
                net_secret(ui, "LoTW pass", &mut io.net_edit.lotw.password, 140.0);
                if !self.ctrl.engine_is_remote() {
                    self.login_test_row(ui, cmds, io.net_edit, LoginTarget::Lotw);
                }
                ui.label(
                    RichText::new(
                        "LoTW upload uses TQSL — export ADIF from the logbook and sign it. \
                         LoTW/eQSL confirmations are downloaded here to mark worked-vs-confirmed.",
                    )
                    .size(10.5)
                    .color(crate::theme::gray(140)),
                );

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if crate::chrome::chip_accent(
                        ui,
                        false,
                        RichText::new(" APPLY ").strong(),
                        crate::theme::GREEN(),
                        crate::theme::INK_ON_CYAN(),
                    )
                    .clicked()
                    {
                        *io.net_apply = true;
                    }
                    if crate::chrome::chip(ui, false, "SYNC CONFIRMATIONS").clicked() {
                        *io.net_sync = true;
                    }
                });
            }
            SettingsTab::FreeDv => {
                if !net_seeded_note(ui, io.net_seeded) {
                    return;
                }
                // The reported identity is the operator's, from the General tab.
                let call = io.digi_edit.my_call.trim().to_string();
                let grid = io.digi_edit.my_grid.trim().to_string();
                settings_freedv_tab(
                    ui,
                    io.net_edit,
                    &call,
                    &grid,
                    io.digi_seeded,
                    &self.net_status,
                    io.net_apply,
                )
            }
            SettingsTab::Controls => settings_controls_tab(
                ui,
                io,
                &self.memories,
                &self.midi_in_ports,
                &self.midi_out_ports,
                &self.input.midi_status(),
                self.input.last_midi,
            ),
            SettingsTab::Servers => {
                settings_rigctld_tab(
                    ui,
                    io.rigctld_edit,
                    self.rigctld_seeded,
                    &self.rigctld_status,
                    io.rigctld_apply,
                );
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                settings_tci_server_tab(
                    ui,
                    io.tci_srv_edit,
                    self.tci_srv_seeded,
                    &self.tci_srv_status,
                    io.tci_srv_apply,
                );
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                settings_wsjtx_tab(ui, io.wsjtx_edit, self.wsjtx_seeded, io.wsjtx_apply);
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                settings_rotator_tab(
                    ui,
                    io.rot_edit,
                    self.rot_cfg_seeded,
                    &self.rotator_status,
                    io.rot_apply,
                );
            }
            SettingsTab::TrSwitch => {
                relay::settings_relay_tab(
                    ui,
                    io.relay_edit,
                    self.relay_seeded,
                    &self.relay_status,
                    &self.serial_ports,
                    &self.relay_devices,
                    io.relay_apply,
                    io.relay_test,
                );
            }
            #[cfg(not(target_arch = "wasm32"))]
            SettingsTab::Remote => settings_remote_tab(
                ui,
                io.remote_edit,
                io.remote_connect,
                // A session with no shell around it has nowhere to put the
                // connection. Every native build has one; this is what keeps
                // the button honest if that ever stops being true.
                !self.radio_roster.is_empty(),
                self.remote_status.as_ref(),
            ),
            SettingsTab::Tle => settings_tle_tab(ui, io),
        }
    }

    /// Subscription status for the settings dialog.
    ///
    /// The engine announces this with the config it annotates, which is the
    /// only source a browser client has. A native client's solar window runs a
    /// fetcher of its own, though, and what *it* last did is both fresher and
    /// what that window is actually drawing — so it wins while it has anything
    /// to say.
    #[cfg(not(target_arch = "wasm32"))]
    fn refresh_sat_sub_status(&mut self) {
        let live = self.solar.tle_sub_status();
        if !live.is_empty() {
            self.sat_sub_status = live;
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn refresh_sat_sub_status(&mut self) {}

    /// Fetch every enabled subscription now, from the settings dialog's UPDATE
    /// NOW button.
    ///
    /// The engine does the fetching: its config directory holds the listings,
    /// and on a server it is also what feeds the browser's solar view. A native
    /// client's own window shares the same disk cache when the engine is local,
    /// so it is told to re-read rather than being left on what it loaded at
    /// open time.
    fn refresh_sat_subs_now(&mut self, cmds: &mut Vec<Command>) {
        cmds.push(Command::RefreshTleSubs);
        self.sat_ui.note = "Fetching subscriptions…".to_string();
        self.sat_ui.fetching = true;
    }

    /// Summarise a refresh the operator asked for, once its status lands.
    pub(in crate::app) fn on_tle_sub_status(&mut self, status: Vec<sdroxide_types::TleSubStatus>) {
        let asked = std::mem::take(&mut self.sat_ui.fetching);
        self.sat_sub_status = status;
        if !asked {
            return;
        }
        let done = &self.sat_sub_status;
        let failed = done.iter().filter(|s| s.error.is_some()).count();
        let total: usize = done.iter().map(|s| s.count).sum();
        self.sat_ui.note = match (done.len(), failed) {
            (0, _) => "No enabled subscriptions to update.".to_string(),
            (n, 0) => format!("Updated {n} subscription(s): {total} satellites."),
            (n, f) => format!("Updated {} of {n}; {f} failed — see the rows above.", n - f),
        };
        // The window's feed shares the disk cache with a local engine, so it is
        // told to re-read rather than being left on what it loaded at open time.
        #[cfg(not(target_arch = "wasm32"))]
        self.solar.reload_tle_subs();
    }
}

/// Settings → Radio → **Panadapter**: borrowing another radio in the roster as
/// this one's receiver.
impl SdroxideApp {
    fn settings_panadapter(&self, ui: &mut egui::Ui, cfg: &mut sdroxide_types::RadioConfig) {
        use sdroxide_types::{IfModeClass, PanadapterAudio, PanadapterTap};

        // A radio that has itself been lent out is not a radio anything can be
        // attached to; its page says so instead.
        if let Some(owner) = self.lent_to() {
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);
            ui.label(
                RichText::new(format!(
                    "This radio is {}'s panadapter",
                    crate::app::radio_name(&self.radio_roster, owner)
                ))
                .size(14.0)
                .strong()
                .color(crate::theme::CYAN()),
            );
            ui.add_space(4.0);
            ui.label(
                RichText::new(
                    "Its spectrum, its waterfall and everything read off them are on that \
                     radio's tab, which is why it has none of its own. The interface settings \
                     above are still this radio's and still apply — press Apply / reconnect \
                     after changing them and both radios come back up together.\n\nTo use it on \
                     its own again, clear the Panadapter receiver box on that radio's page.",
                )
                .weak(),
            );
            return;
        }

        // Every other radio on the station, minus any that is already somebody
        // else's receiver: one receiver, one borrower.
        //
        // Named the way the *station* numbers them, not the way this screen
        // does: what is chosen here is written into a `radio.json` on that
        // station, which knows nothing of this screen's tab ids. And restricted
        // to radios on the same station — a receiver is lent inside one
        // station, and a screen may be holding two.
        let here = self.radio_roster.iter().find(|c| c.id == self.radio_id);
        let (station, me) = match here {
            Some(c) => (c.station.clone(), c.station_id),
            None => (String::new(), self.radio_id),
        };
        let candidates: Vec<(u32, String)> = self
            .radio_roster
            .iter()
            .filter(|c| c.station == station && c.station_id != me)
            .filter(|c| c.attached_to.is_none_or(|owner| owner == self.radio_id))
            .map(|c| (c.station_id, c.display_name().to_string()))
            .collect();
        if candidates.is_empty() && !cfg.panadapter.is_attached() {
            return;
        }

        let pan = &mut cfg.panadapter;
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(6.0);
        ui.label(RichText::new("Panadapter").size(14.0).strong().color(crate::theme::CYAN()));
        ui.add_space(4.0);
        ui.label(
            RichText::new(
                "Give this radio a wideband panadapter by borrowing another radio's receiver — \
                 an SDR on the same antenna, or one fed from this radio's I.F. output. The \
                 spectrum, the waterfall, the sub receiver, the digital modes and the skimmer \
                 all come from that receiver; the dial, the mode, the filter and the \
                 transmitter stay here.",
            )
            .weak(),
        );
        ui.add_space(6.0);

        egui::Grid::new("panadapter-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label(RichText::new("Receiver").strong());
            let selected = pan
                .source_radio
                .map(|id| {
                    candidates
                        .iter()
                        .find(|(c, _)| *c == id)
                        .map_or_else(|| format!("radio {}", id + 1), |(_, n)| n.clone())
                })
                .unwrap_or_else(|| "None".to_string());
            ComboBox::from_id_salt("pan-source")
                .selected_text(selected)
                .show_styled(ui, |ui| {
                    if ui.selectable_label(pan.source_radio.is_none(), "None").clicked() {
                        pan.source_radio = None;
                    }
                    for (id, name) in &candidates {
                        if ui.selectable_label(pan.source_radio == Some(*id), name).clicked() {
                            pan.source_radio = Some(*id);
                        }
                    }
                })
                .response
                .on_hover_text(
                    "Which radio receives for this one. It leaves the tab strip while it is \
                 lent — its front end belongs to this radio's engine — and comes straight \
                 back when this is set to None.\n\nTakes effect on Apply.",
                );
            ui.end_row();

            if !pan.is_attached() {
                return;
            }

            ui.label(RichText::new("Connected to").strong());
            enum_combo(ui, "pan-tap", &mut pan.tap, &PanadapterTap::ALL, PanadapterTap::label);
            ui.end_row();

            ui.label(RichText::new("Offset").strong());
            ui.add(
                egui::DragValue::new(&mut pan.offset_hz)
                    .speed(1.0)
                    .range(
                        -sdroxide_types::PANADAPTER_OFFSET_MAX_HZ
                            ..=sdroxide_types::PANADAPTER_OFFSET_MAX_HZ,
                    )
                    .max_decimals(0)
                    .suffix(" Hz"),
            )
            .on_hover_text(
                "How far above this radio's dial the receiver sits, in hertz. 0 for a receiver \
                 on the same antenna; the intermediate frequency for an I.F. tap — 9000000 for \
                 a 9 MHz I.F., 70455000 for a 70.455 MHz one.\n\nThis is not the Converter field \
                 above, which retunes the radio, and not the CAT tab's I/Q centre offset, which \
                 is about this radio's own sound card. Nothing typed here is ever sent to \
                 either radio.\n\nTakes effect on Apply.",
            );
            ui.end_row();

            ui.label(RichText::new("Audio from").strong());
            enum_combo(
                ui,
                "pan-audio",
                &mut pan.audio,
                &PanadapterAudio::ALL,
                PanadapterAudio::label,
            );
            ui.end_row();

            ui.label(RichText::new("Follow the dial").strong());
            crate::chrome::checkbox(ui, &mut pan.track, "").on_hover_text(
                "Keep the receiver on this radio's dial, and move the dial when you tune on \
                 the panadapter. Off parks the receiver where it is — a fixed watch on one \
                 segment while the radio goes elsewhere.",
            );
            ui.end_row();

            ui.label(RichText::new("Invert spectrum").strong());
            crate::chrome::checkbox(ui, &mut pan.invert, "").on_hover_text(
                "Mirror the receiver's span about its centre, for a tap whose oscillator sits \
                 above the signal and hands the band over the wrong way round. Leave it off \
                 unless the waterfall fills with signals that are all on the wrong side of the \
                 dial.",
            );
            ui.end_row();

            ui.label(RichText::new("Mute on transmit").strong());
            crate::chrome::checkbox(ui, &mut pan.mute_on_tx, "").on_hover_text(
                "Silence receive audio while this radio is transmitting. On by default: with \
                 the receiver on the same antenna, or on this radio's I.F., what it hears \
                 during an over is your own transmitter. Overs keyed at the radio itself — \
                 the microphone's PTT, a foot switch, MOX — count too, as soon as the CAT \
                 link says the rig has gone to transmit.",
            );
            ui.end_row();

            ui.label(RichText::new("Blank on transmit").strong());
            crate::chrome::checkbox(ui, &mut pan.blank_on_tx, "").on_hover_text(
                "Stop the panadapter and waterfall while this radio is transmitting. On by \
                 default: a transmitter painted across the whole span erases the band behind \
                 it. Turn it off to watch the band — or your own signal — through an over, \
                 which is worth having with the receiver on a separate antenna. Applies to \
                 overs sdroxide keys: one keyed at the radio is received right through, so \
                 the picture keeps running whatever this says.",
            );
            ui.end_row();
        });

        if pan.is_attached() && pan.tap == PanadapterTap::IfOutput {
            ui.add_space(6.0);
            ui.label(
                RichText::new(
                    "Per-mode offsets, for a radio whose I.F. moves with the mode. A mode left \
                     empty uses the offset above.",
                )
                .weak(),
            );
            ui.add_space(4.0);
            egui::Grid::new("pan-mode-offsets").num_columns(3).spacing([10.0, 4.0]).show(
                ui,
                |ui| {
                    for class in IfModeClass::ALL {
                        ui.label(RichText::new(class.label()).strong());
                        let mut on = pan.mode_offset(class).is_some();
                        if crate::chrome::checkbox(ui, &mut on, "").changed() {
                            pan.set_mode_offset(class, on.then_some(pan.offset_hz));
                        }
                        match pan.mode_offset(class) {
                            Some(mut hz) => {
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut hz)
                                            .speed(1.0)
                                            .range(
                                                -sdroxide_types::PANADAPTER_OFFSET_MAX_HZ
                                                    ..=sdroxide_types::PANADAPTER_OFFSET_MAX_HZ,
                                            )
                                            .max_decimals(0)
                                            .suffix(" Hz"),
                                    )
                                    .changed()
                                {
                                    pan.set_mode_offset(class, Some(hz));
                                }
                            }
                            None => {
                                ui.label(RichText::new(format!("{} Hz", pan.offset_hz)).weak());
                            }
                        }
                        ui.end_row();
                    }
                },
            );
        }
    }
}

/// One place a new radio can go. A screen can hold two rosters at once — this
/// machine's own and that of a station it is connected to — and "add a radio"
/// means nothing until it says which, so the "+" chip offers these.
struct AddTarget {
    /// The station key ([`SdroxideApp::station_key`]) whose roster this is;
    /// empty for this computer's own.
    key: String,
}

impl AddTarget {
    fn label(&self) -> String {
        if self.key.is_empty() {
            "On this computer".to_string()
        } else {
            format!("On {}", station_label(&self.key))
        }
    }

    fn hint(&self) -> String {
        if self.key.is_empty() {
            "Add a radio on this computer".to_string()
        } else {
            format!(
                "Add a radio on {} — the station serves it straight away",
                station_label(&self.key)
            )
        }
    }
}

/// A station key as it reads on a button: the address, without the scheme
/// nobody needs to see. Falls back to the key itself for anything unexpected.
fn station_label(key: &str) -> &str {
    key.split_once("://").map_or(key, |(_, rest)| rest)
}

/// Which gap in `placed` a chip dropped at `p` belongs in: the index of the
/// chip it lands in front of, or `placed.len()` for the end of the strip.
///
/// Measured against the *nearest* chip rather than by scanning left to right,
/// because the strip wraps: on a narrow dialog the chips are on two or three
/// rows, and an x-only test would put a chip dropped on the second row in a gap
/// on the first. Distance to the rectangle handles both axes at once, and the
/// half of that chip the pointer is on decides which side of it the gap is.
fn drop_slot(placed: &[(u32, egui::Rect)], p: egui::Pos2) -> usize {
    let Some((i, r)) = placed
        .iter()
        .enumerate()
        .map(|(i, (_, r))| (i, *r))
        .min_by(|(_, a), (_, b)| a.distance_sq_to_pos(p).total_cmp(&b.distance_sq_to_pos(p)))
    else {
        return 0;
    };
    if p.x < r.center().x { i } else { i + 1 }
}

/// Where to draw the insertion caret for a gap: `(x, the row's y range)`. The
/// leading edge of the chip that would follow it, or the trailing edge of the
/// last chip for the gap at the end.
fn caret_at(placed: &[(u32, egui::Rect)], slot: usize) -> Option<(f32, egui::Rangef)> {
    match placed.get(slot) {
        Some((_, r)) => Some((r.left() - 3.0, r.y_range())),
        None => placed.last().map(|(_, r)| (r.right() + 3.0, r.y_range())),
    }
}

/// `ids` with `dragged` taken out and put back in front of `before` — or at the
/// end, where `before` is `None`.
///
/// Both indices are read off the list *before* the removal and the target is
/// then stepped back over the hole, which is what makes dropping a chip on
/// either half of itself a no-op: both halves name a gap that is where it
/// already is, and an index taken after the removal would read one of them as a
/// move by one.
fn reordered(ids: &[u32], dragged: u32, before: Option<u32>) -> Vec<u32> {
    let mut out = ids.to_vec();
    let Some(from) = ids.iter().position(|id| *id == dragged) else { return out };
    let to = before.and_then(|b| ids.iter().position(|id| *id == b)).unwrap_or(ids.len());
    let item = out.remove(from);
    out.insert(if to > from { to - 1 } else { to }.min(out.len()), item);
    out
}

/// The radio-management strip at the top of Settings → Radio: the same chips
/// the main window's tab area shows, drawn where radios are configured. The
/// main window only shows its copy once there is more than one radio, so this
/// is where the second one is added. Actions cannot be taken here — the tab
/// set lives in the multi-radio shell, not in this tab — so they are queued as
/// [`crate::app::RadioTabRequest`]s and the shell acts on them after the frame.
impl SdroxideApp {
    /// Where a new radio could go, in the order the chip offers them: this
    /// computer first where it has hardware of its own to open, then every
    /// station on screen that keeps a roster it will let a client touch, once
    /// each however many of its radios are in tabs.
    fn add_radio_targets(&self) -> Vec<AddTarget> {
        let mut out = Vec::new();
        if self.can_add_radio {
            out.push(AddTarget { key: String::new() });
        }
        for chip in &self.radio_roster {
            if chip.station.is_empty() || !chip.roster_editable {
                continue;
            }
            if out.iter().any(|t: &AddTarget| t.key == chip.station) {
                continue;
            }
            out.push(AddTarget { key: chip.station.clone() });
        }
        out
    }

    fn settings_radio_roster(
        &self,
        ui: &mut egui::Ui,
        requests: &mut Vec<crate::app::RadioTabRequest>,
        name_edit: &mut Option<(u32, String)>,
    ) {
        // Nothing to manage: one radio, with nowhere to add a second — no
        // hardware of this machine's own to open, and no station at the far end
        // that keeps a roster it would let us touch.
        if self.radio_roster.is_empty() {
            return;
        }
        let targets = self.add_radio_targets();
        if self.radio_roster.len() < 2 && targets.is_empty() {
            return;
        }
        // Which chip is being dragged along the strip, and where every chip
        // ended up this frame — the two halves of the reorder (issue #224).
        let drag_id = ui.id().with("roster-drag");
        let mut dragging: Option<u32> = ui.data(|d| d.get_temp(drag_id)).unwrap_or(None);
        let mut placed: Vec<(u32, egui::Rect)> = Vec::new();
        let mut dropped = false;
        ui.horizontal_wrapped(|ui| {
            for chip in &self.radio_roster {
                let mut label = RichText::new(chip.display_name()).size(12.5);
                if !chip.enabled {
                    label = label.weak();
                }
                if chip.focused {
                    // The ink a *selected* chip carries, not the panel's:
                    // this sits on the accent fill, and the strong body
                    // colour is the one shade guaranteed to be closest to it.
                    label = label.strong().color(crate::theme::INK_ON_CYAN());
                }
                // Draggable, so the operator can arrange the strip. A drag that
                // never travels far enough to be one still arrives as a click,
                // so switching radios costs exactly what it did before.
                let resp = crate::chrome::chip_draggable(ui, chip.focused, label);
                placed.push((chip.id, resp.rect));
                if resp.drag_started() {
                    dragging = Some(chip.id);
                }
                if resp.drag_stopped() {
                    dropped = true;
                }
                if resp
                    .on_hover_text(if chip.focused {
                        "This radio's settings are below. Drag to move it along the strip."
                    } else {
                        "Switch to this radio (the dialog follows). Drag to move it along the \
                         strip."
                    })
                    .clicked()
                    && !chip.focused
                {
                    requests.push(crate::app::RadioTabRequest::Focus(chip.id));
                }
                if let Some(owner) = chip.attached_to {
                    // Off the main window's strip, so this is the one place it
                    // shows at all — and the marker has to say why. An emoji
                    // rather than an arrow: the arrows are not in the text font
                    // and come out as an empty box, while the mute markers
                    // beside it prove the emoji font is there.
                    ui.label(RichText::new("🔗").size(11.0).color(crate::theme::CYAN()))
                        .on_hover_text(format!(
                            "Panadapter receiver for {}",
                            crate::app::radio_name(&self.radio_roster, owner)
                        ));
                } else if chip.tx_on {
                    ui.label(RichText::new("● TX").size(11.0).color(crate::theme::ALERT()));
                } else if chip.error && chip.enabled {
                    ui.label(RichText::new("⚠").size(11.0).color(crate::theme::ALERT()));
                }
                if chip.enabled {
                    let mute = crate::chrome::chip(
                        ui,
                        chip.muted,
                        RichText::new(if chip.muted { "🔇" } else { "🔊" }).size(11.0),
                    );
                    if mute.on_hover_text("Mute this radio's audio").clicked() {
                        requests.push(crate::app::RadioTabRequest::Mute {
                            id: chip.id,
                            muted: !chip.muted,
                        });
                    }
                }
                // The switch. A radio that is off keeps everything on this page
                // — it is simply not opened — so this is where a rig that has
                // been put away for the season is left, rather than being
                // deleted and set up again in the spring. One at the far end of
                // a connection is switched off where it lives, and this asks
                // its station to do that. Not offered for a radio lent out as
                // somebody's panadapter, whose front end is the borrower's to
                // hold — nor by a station that does not take the request.
                //
                // Lit while the radio is on, as the strip's copy of the switch
                // is: the chip says which state the radio is in, and a chip
                // wears the accent when what it says is in force (issue #253).
                if chip.switchable
                    && chip.attached_to.is_none()
                    && self.radio_roster.len() > 1
                    && crate::chrome::chip(
                        ui,
                        chip.enabled,
                        RichText::new(if chip.enabled { "ON" } else { "OFF" }).size(11.0),
                    )
                    .on_hover_text(if chip.enabled {
                        crate::chrome::POWER_OFF_TIP
                    } else {
                        crate::chrome::POWER_ON_TIP
                    })
                    .clicked()
                {
                    requests.push(crate::app::RadioTabRequest::Power {
                        id: chip.id,
                        on: !chip.enabled,
                    });
                }
                // The first radio is the station: it runs the shared network
                // services and the legacy configuration, and it stays.
                //
                // On one of this machine's own radios "×" closes it, as it
                // always has. On a *connection* it opens a menu instead,
                // because there are two things it could mean and they are not
                // the same thing at all: hanging up leaves the radio where it
                // is, and closing it on the station takes it out of somebody's
                // roster. Both live here, where the operator reaches for the
                // close box — a second button somewhere else is a button that
                // does not get found (issue #188) — and the second asks again
                // before it acts, so taking a radio out of a station is still
                // never one stray click.
                // Exactly the one tab the shell refuses to close: this
                // machine's station radio, which holds the shared services and
                // the legacy configuration. Not "the first chip on the strip" —
                // that is the operator's to arrange (issue #224) — and never a
                // connection, however far left it has been dragged: hanging up
                // has to stay possible.
                if !(chip.station.is_empty() && chip.first_of_station) {
                    let closing = if chip.station.is_empty() {
                        "Close this radio (its configuration is kept)"
                    } else {
                        "Close this tab, or close the radio on its station"
                    };
                    let close = crate::chrome::chip(ui, false, RichText::new("×").size(11.0))
                        .on_hover_text(closing);
                    if chip.station.is_empty() {
                        if close.clicked() {
                            requests.push(crate::app::RadioTabRequest::Close(chip.id));
                        }
                    } else {
                        Self::close_station_radio_menu(ui, &close, chip, requests);
                    }
                }
                ui.add_space(6.0);
            }
            // Absent where there is nothing to add: a client that only drives
            // somebody else's station, and that station does not take roster
            // edits. Connecting to a further server is still offered, on the
            // Remote tab.
            //
            // Where a radio can go in more than one place — this computer and a
            // station at the far end of a connection — the chip asks which
            // first. "Add a radio" is ambiguous the moment a screen holds two
            // rosters, and the two answers are as different as plugging a
            // dongle in here and plugging one in at the remote site.
            match targets.as_slice() {
                [] => {}
                // One destination, and it is this computer: press and it is
                // made. Nobody has to be told which machine their own radio
                // goes on.
                [only] if only.key.is_empty() => {
                    if crate::chrome::chip(ui, false, RichText::new("+").size(13.0))
                        .on_hover_text(only.hint())
                        .clicked()
                    {
                        requests.push(crate::app::RadioTabRequest::Add {
                            station: only.key.clone(),
                            preset: None,
                        });
                    }
                }
                // One destination, and it is somebody else's station — which is
                // every browser client, since a browser has no hardware of its
                // own and so no second roster to disambiguate against. It still
                // names the station and asks: creating a radio on a machine at
                // the other end of a network is not what "+" looks like it
                // does, and a station that grew radios 2, 3 and 4 nobody meant
                // to ask for is what that cost (issue #188).
                many => {
                    // Plain "+", like the single-destination case: the arrow
                    // that would mark it as a menu is not in the text font and
                    // comes out as an empty box. What it opens is a menu, and
                    // the hover text says which rosters are in it.
                    let tip = match many {
                        [only] => only.hint(),
                        _ => "Add a radio — here, or on a station you are connected to".to_string(),
                    };
                    let btn = crate::chrome::chip(ui, false, RichText::new("+").size(13.0))
                        .on_hover_text(tip);
                    crate::chrome::menu_popup(ui, &btn, |ui| {
                        for t in many {
                            if ui.button(t.label()).on_hover_text(t.hint()).clicked() {
                                requests.push(crate::app::RadioTabRequest::Add {
                                    station: t.key.clone(),
                                    preset: None,
                                });
                                ui.close();
                            }
                        }
                    });
                }
            }
        });
        // The drop. While a chip is being dragged a caret shows where it would
        // land; on release the whole strip's order goes to the shell, which
        // sorts its tabs to match and records it.
        // A drag whose release this strip never saw — the dialog was closed
        // under it, or the pointer left the window — must not leave a caret
        // following the mouse the next time the page is opened.
        if dragging.is_some() && !ui.input(|i| i.pointer.any_down()) {
            dragging = None;
        }
        if let Some(id) = dragging {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            let at = ui.ctx().pointer_interact_pos();
            let slot = at.map(|p| drop_slot(&placed, p));
            if let Some((x, y)) = slot.and_then(|i| caret_at(&placed, i)) {
                ui.painter().vline(x, y, egui::Stroke::new(2.0, crate::theme::CYAN()));
            }
            if dropped {
                let before = slot.and_then(|i| placed.get(i)).map(|(id, _)| *id);
                let now: Vec<u32> = placed.iter().map(|(id, _)| *id).collect();
                let next = reordered(&now, id, before);
                if next != now {
                    requests.push(crate::app::RadioTabRequest::Reorder(next));
                }
                dragging = None;
            }
        }
        ui.data_mut(|d| d.insert_temp(drag_id, dragging));
        // The focused radio's name. By default a radio is named after its
        // interface — the box is empty and the hint shows what that resolves
        // to — and typing here gives it a name of the operator's own.
        // Committed when the field loses focus (Enter included); cleared, the
        // derived default takes over again.
        if let Some(chip) = self.radio_roster.iter().find(|c| c.focused) {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Name").strong());
                let stale = !matches!(name_edit, Some((id, _)) if *id == chip.id);
                if stale {
                    *name_edit = Some((chip.id, chip.name.clone()));
                }
                let buf = &mut name_edit.as_mut().expect("seeded above").1;
                let resp = crate::chrome::field(
                    ui,
                    egui::TextEdit::singleline(buf)
                        .hint_text(chip.default_name.as_str())
                        .desired_width(220.0),
                );
                if resp.lost_focus() {
                    let name = buf.trim().to_string();
                    if name != chip.name {
                        requests.push(crate::app::RadioTabRequest::Rename { id: chip.id, name });
                    }
                }
            });
        }
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(6.0);
    }

    /// What the "×" on a *connection's* chip opens: hang up, or close the radio
    /// on the station it lives at.
    ///
    /// Two answers rather than one, because "close" means two different things
    /// to a radio on somebody else's machine and neither is obviously the one
    /// meant. Hanging up is the first entry and acts at once — it is what "×"
    /// has always done, and it changes nothing anywhere else. Taking the radio
    /// out of the station's roster is the second, and asks again: it reaches
    /// across the network, every other client on that station sees it, and its
    /// device is released.
    ///
    /// The station's own first radio is never offered the second: it holds the
    /// shared network services and the address every client that knows nothing
    /// of a roster arrives at, and the station refuses to close it.
    fn close_station_radio_menu(
        ui: &mut egui::Ui,
        button: &egui::Response,
        chip: &crate::app::RadioChip,
        requests: &mut Vec<crate::app::RadioTabRequest>,
    ) {
        let where_ = station_label(&chip.station).to_string();
        let removable = chip.roster_editable && !chip.first_of_station;
        crate::chrome::menu_popup(ui, button, |ui| {
            if ui
                .button("Close this tab")
                .on_hover_text(format!("Hang up. The radio stays on {where_}."))
                .clicked()
            {
                requests.push(crate::app::RadioTabRequest::Close(chip.id));
                ui.close();
            }
            if !removable {
                return;
            }
            ui.separator();
            ui.label(
                RichText::new(format!(
                    "Close {} on {where_}? Its configuration stays on that machine; the tab here \
                     closes with it, and so does everyone else's.",
                    chip.display_name(),
                ))
                .size(11.5),
            );
            if ui.button(RichText::new(format!("Close it on {where_}")).strong()).clicked() {
                requests.push(crate::app::RadioTabRequest::RemoveFromStation(chip.id));
                ui.close();
            }
        });
    }
}

#[cfg(test)]
mod roster_order_tests {
    use super::*;
    use eframe::egui::{Rect, pos2};

    /// A row of three chips 60 points wide with 6 points between them.
    fn strip() -> Vec<(u32, Rect)> {
        (0..3u32)
            .map(|i| {
                let x = 10.0 + i as f32 * 66.0;
                (i, Rect::from_min_size(pos2(x, 20.0), egui::vec2(60.0, 22.0)))
            })
            .collect()
    }

    /// The pointer's half of the nearest chip picks the gap, and past the last
    /// chip that is the end of the strip.
    #[test]
    fn the_drop_slot_follows_the_pointer() {
        let s = strip();
        assert_eq!(drop_slot(&s, pos2(15.0, 30.0)), 0, "the left half of the first chip");
        assert_eq!(drop_slot(&s, pos2(65.0, 30.0)), 1, "the right half of the first chip");
        assert_eq!(drop_slot(&s, pos2(90.0, 30.0)), 1, "the left half of the second");
        assert_eq!(drop_slot(&s, pos2(400.0, 30.0)), 3, "past the end of the strip");
    }

    /// The strip wraps on a narrow dialog, and a chip dropped on the row below
    /// must land in a gap on *that* row rather than in the nearest gap by x.
    #[test]
    fn a_wrapped_strip_drops_onto_the_row_under_the_pointer() {
        let mut s = strip();
        // A fourth chip, alone on a second row under the first.
        s.push((3, Rect::from_min_size(pos2(10.0, 50.0), egui::vec2(60.0, 22.0))));
        assert_eq!(drop_slot(&s, pos2(65.0, 60.0)), 4, "the second row's own chip");
        assert_eq!(drop_slot(&s, pos2(15.0, 60.0)), 3, "in front of it");
    }

    /// A chip dropped where it already is changes nothing — on either half of
    /// itself, which is what stops a drag that never really moved from
    /// shuffling the strip by one.
    #[test]
    fn dropping_a_chip_on_itself_is_a_no_op() {
        let ids = [0u32, 1, 2];
        assert_eq!(reordered(&ids, 1, Some(1)), vec![0, 1, 2], "its own left half");
        assert_eq!(reordered(&ids, 1, Some(2)), vec![0, 1, 2], "its own right half");
    }

    /// And a real move takes the chip out and puts it back in the gap named.
    #[test]
    fn a_chip_moves_to_the_gap_it_was_dropped_in() {
        let ids = [0u32, 1, 2, 3];
        assert_eq!(reordered(&ids, 3, Some(0)), vec![3, 0, 1, 2], "to the front");
        assert_eq!(reordered(&ids, 0, None), vec![1, 2, 3, 0], "to the end");
        assert_eq!(reordered(&ids, 0, Some(3)), vec![1, 2, 0, 3], "into the middle");
    }
}

#[cfg(test)]
mod tests {
    use super::iface_opts;

    /// A backend registered everywhere else (`Backend::ALL`, the `open_*`
    /// dispatch in `src/main.rs`, its own settings tab) but never pushed
    /// into this picker's own list is invisible in the UI despite every
    /// other touchpoint being wired up — exactly what happened to Fobos:
    /// `Backend::ALL.contains(&Backend::Fobos)` was true the whole time,
    /// which is a different list, and asserting against it (as the
    /// `sdroxide-types` test for `HydraSdr` does) gives no real coverage of
    /// what the operator actually sees. This checks the list `iface_opts`
    /// itself, the same slice `enum_combo` renders from.
    #[test]
    fn every_backend_but_auto_and_none_is_offered_in_the_picker() {
        let opts = iface_opts(true);
        for b in sdroxide_types::Backend::ALL {
            if matches!(b, sdroxide_types::Backend::Auto | sdroxide_types::Backend::None) {
                continue;
            }
            assert!(opts.contains(&b), "{b:?} is in Backend::ALL but missing from iface_opts");
        }
    }

    /// SoapySDR is the one entry gated on compile-time support — dropped,
    /// not just greyed out, when it isn't there to try.
    #[test]
    fn soapy_only_appears_when_supported() {
        assert!(iface_opts(true).contains(&sdroxide_types::Backend::Soapy));
        assert!(!iface_opts(false).contains(&sdroxide_types::Backend::Soapy));
    }
}
