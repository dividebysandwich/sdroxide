use std::time::Duration;

use eframe::egui::{self, Color32, ComboBox, DragValue, RichText, Slider};
use sdroxide_types::{
    AgcMode, AudioDevices, Band, Command, Decode, DeviceCaps, DigiStatus, Direction,
    MemoryChannel, Meters, Mode, QsoRecord, RadioController, RadioEvent, RadioState, RxId,
    SkimmerKind, SkimmerSpot, SpectrumConfig, SpectrumFrame, Vfo,
};

use crate::view::ViewState;
use crate::widgets::{freq_display, smeter, spectrum_view};
use crate::{colormap, waterfall_gpu};

/// Viewport/FFT config updates are sent once the view has been stable this
/// long (seconds of egui time — `std::time::Instant` panics on wasm).
const CFG_DEBOUNCE_S: f64 = 0.25;

/// A skimmer box fades to nothing over this many seconds after its signal
/// stops keying, instead of vanishing.
const SKIMMER_FADE_SECS: f64 = 5.0;

/// Settings dialog tabs: the radio interface + its settings, audio devices, and
/// display/UI preferences.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SettingsTab {
    Radio,
    Audio,
    Ui,
}

/// Repaint-poll cadence when no spectrum stream is flowing (startup, connection
/// lost, stalled stream) — the app truly idles between these wakes.
const IDLE_POLL_MS: u64 = 250;
/// The stream counts as stalled after this long without a new frame (seconds).
const STREAM_STALE_S: f64 = 1.0;

/// Stable per-callsign id for the FT8 overlay boxes (keeps a station's box in
/// place across slots).
fn hash_call(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

pub struct SdroxideApp {
    ctrl: Box<dyn RadioController>,
    caps: Option<DeviceCaps>,
    state: RadioState,
    /// Latest spectrum frame, shared with the GPU waterfall callback — the Arc
    /// makes the per-repaint handoff a refcount bump instead of a bins clone.
    frame: Option<std::sync::Arc<SpectrumFrame>>,
    meters: Option<Meters>,
    memories: Vec<MemoryChannel>,
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
    /// Whether this build can drive SoapySDR (offered as an interface option).
    soapy_supported: bool,
    /// Settings dialog: current tab, plus the radio-backend config + serial
    /// ports loaded once on open (edited live, persisted on change).
    settings_tab: SettingsTab,
    /// Display preferences (frame rate, waterfall + spectrum speed), loaded from
    /// config at startup, edited in the UI tab, persisted on change.
    ui_settings: sdroxide_types::UiSettings,
    radio_cfg: Option<sdroxide_types::RadioConfig>,
    serial_ports: Vec<String>,
    /// HPSDR devices found by the last "Discover" scan in the settings dialog.
    hpsdr_devices: Vec<sdroxide_types::HpsdrDevice>,
    /// Result of the last TCI "Test connection" (Ok summary / Err message).
    tci_test_result: Option<Result<String, String>>,
    seen_first_state: bool,
    show_memories: bool,
    show_settings: bool,
    mem_name: String,
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
    show_digi_settings: bool,
    /// UI-owned editable copy of the operator config, so typing isn't fought
    /// by the round-tripped status echo. Seeded once from the first status.
    digi_cfg_edit: sdroxide_types::DigiConfig,
    digi_cfg_seeded: bool,
    /// The last decode the user clicked (not REPLY): its call and map
    /// location, shown as a faint preview marker distinct from the active DX.
    digi_preview: Option<(String, (f64, f64))>,
    /// Voice-mode view span saved on entering FT8/FT4 (which locks the view to
    /// the narrow sub-band), restored on leaving so the panadapter isn't left
    /// stuck zoomed in.
    pre_digi_view: Option<(f64, f64)>,
    /// Logbook overlay open state, and the in-progress new/edit entry (if any).
    show_logbook: bool,
    log_edit: Option<LogEditForm>,
}

/// Editable text fields for a manual logbook entry (new or edit). Kept as
/// strings so partial input doesn't fight the user; parsed on save.
#[derive(Default)]
struct LogEditForm {
    /// 0 = new entry; otherwise the id of the record being edited.
    id: u64,
    /// Timestamp fallback if the date/time fields don't parse.
    seed_utc: i64,
    call: String,
    grid: String,
    freq_mhz: String,
    mode: String,
    rst_sent: String,
    rst_rcvd: String,
    date: String,
    time: String,
    comment: String,
}

impl LogEditForm {
    /// A blank new entry seeded with the current time, band, and mode.
    fn new_entry(now: i64, freq_hz: f64, mode: &str) -> LogEditForm {
        let (y, mo, d, h, mi, _) = sdroxide_types::utc_ymd_hms(now);
        LogEditForm {
            id: 0,
            seed_utc: now,
            freq_mhz: if freq_hz > 0.0 { format!("{:.4}", freq_hz / 1e6) } else { String::new() },
            mode: mode.to_string(),
            date: format!("{y:04}-{mo:02}-{d:02}"),
            time: format!("{h:02}:{mi:02}"),
            ..Default::default()
        }
    }

    fn from_record(r: &QsoRecord) -> LogEditForm {
        let (y, mo, d, h, mi, _) = sdroxide_types::utc_ymd_hms(r.start_utc);
        LogEditForm {
            id: r.id,
            seed_utc: r.start_utc,
            call: r.call.clone(),
            grid: r.grid.clone().unwrap_or_default(),
            freq_mhz: if r.freq_hz > 0.0 { format!("{:.4}", r.freq_hz / 1e6) } else { String::new() },
            mode: r.mode.clone(),
            rst_sent: r.rst_sent.map(|v| v.to_string()).unwrap_or_default(),
            rst_rcvd: r.rst_rcvd.map(|v| v.to_string()).unwrap_or_default(),
            date: format!("{y:04}-{mo:02}-{d:02}"),
            time: format!("{h:02}:{mi:02}"),
            comment: r.comment.clone(),
        }
    }

    /// Parse into a record, or `None` if the callsign is empty.
    fn to_record(&self, my_call: &str, my_grid: &str) -> Option<QsoRecord> {
        let call = self.call.trim().to_uppercase();
        if call.is_empty() {
            return None;
        }
        let freq_hz = self.freq_mhz.trim().parse::<f64>().ok().map(|m| m * 1e6).unwrap_or(0.0);
        let band =
            if freq_hz > 0.0 { sdroxide_types::adif_band(freq_hz).to_string() } else { String::new() };
        let start = parse_utc(&self.date, &self.time, self.seed_utc);
        let grid = {
            let g = self.grid.trim();
            (!g.is_empty()).then(|| g.to_uppercase())
        };
        let mode = {
            let m = self.mode.trim().to_uppercase();
            if m.is_empty() { "SSB".into() } else { m }
        };
        Some(QsoRecord {
            id: self.id,
            call,
            grid,
            rst_sent: self.rst_sent.trim().parse().ok(),
            rst_rcvd: self.rst_rcvd.trim().parse().ok(),
            freq_hz,
            mode,
            band,
            start_utc: start,
            end_utc: start,
            my_call: my_call.to_string(),
            my_grid: my_grid.to_string(),
            comment: self.comment.trim().to_string(),
        })
    }
}

impl SdroxideApp {
    pub fn new(cc: &eframe::CreationContext<'_>, ctrl: Box<dyn RadioController>) -> Self {
        crate::theme::apply(&cc.egui_ctx);
        if let Some(rs) = &cc.wgpu_render_state {
            waterfall_gpu::init(rs);
        }
        let view: ViewState = cc
            .storage
            .and_then(|s| eframe::get_value(s, "view"))
            .unwrap_or_default();
        let soapy_supported = ctrl.soapy_supported();
        SdroxideApp {
            ctrl,
            caps: None,
            state: RadioState::default(),
            frame: None,
            meters: None,
            memories: Vec::new(),
            view,
            peaks: spectrum_view::PeakHold::default(),
            spec_smooth: spectrum_view::SpectrumSmooth::default(),
            error: None,
            radio_notice: None,
            sent_cfg: None,
            desired_cfg: None,
            desired_at: 0.0,
            last_spectrum_at: 0.0,
            wf_last_now: 0.0,
            wf_row_accum: 0.0,
            trace_cache: spectrum_view::TraceCache::default(),
            audio_devices: None,
            audio_devices_queried: false,
            soapy_supported,
            settings_tab: SettingsTab::Radio,
            ui_settings: load_ui_settings(cc.storage),
            radio_cfg: None,
            serial_ports: Vec::new(),
            hpsdr_devices: Vec::new(),
            tci_test_result: None,
            seen_first_state: false,
            show_memories: false,
            show_settings: false,
            mem_name: String::new(),
            skimmer_spots: Vec::new(),
            skimmer_active_at: std::collections::HashMap::new(),
            digi_decodes: Vec::new(),
            digi_status: None,
            text_tx: String::new(),
            qso_log: load_qso_log(cc.storage),
            show_digi_settings: false,
            digi_cfg_edit: sdroxide_types::DigiConfig::default(),
            digi_cfg_seeded: false,
            digi_preview: None,
            pre_digi_view: None,
            show_logbook: false,
            log_edit: None,
        }
    }

    /// Next free logbook id.
    fn next_log_id(&self) -> u64 {
        self.qso_log.iter().map(|q| q.id).max().unwrap_or(0) + 1
    }

    /// Desired engine-side spectrum config. The requested viewport gets 2×
    /// slack around the visible span so panning inside it needs no
    /// reconfiguration (which would clear the waterfall history); the FFT
    /// grows with zoom for real resolution.
    fn desired_spectrum_cfg(&self) -> SpectrumConfig {
        let full_span = self.state.sample_rate;
        let dev_lo = self.state.center_hz - full_span / 2.0;
        let dev_hi = self.state.center_hz + full_span / 2.0;
        let (viewport, zoom) = if !self.view.is_unset() && full_span > 0.0 {
            let vspan = self.view.span();
            let ratio = (full_span / vspan).max(1.0);
            if ratio > 1.05 {
                let slack = (vspan * 2.0).min(full_span);
                let center = (self.view.view_lo_hz + self.view.view_hi_hz) / 2.0;
                let lo = (center - slack / 2.0).clamp(dev_lo, dev_hi - slack);
                (Some((lo, lo + slack)), ratio)
            } else {
                (None, 1.0)
            }
        } else {
            (None, 1.0)
        };
        let mut fft = self.view.fft_size.max(1024);
        while (fft as f64) < self.view.fft_size as f64 * zoom.min(8.0) && fft < 32_768 {
            fft *= 2;
        }
        SpectrumConfig {
            fft_size: fft,
            db_floor: self.view.db_floor,
            db_ceil: self.view.db_ceil,
            viewport,
            // Frame rate comes from the UI settings and also drives the repaint
            // cadence (see the end of `ui`). Engine averaging is disabled so the
            // waterfall gets full detail; the spectrum *line* is smoothed UI-side
            // per the spectrum-speed setting (decoupled from the waterfall).
            fps: self.ui_settings.fps().min(255) as u8,
            avg_tc: 0.0,
        }
    }

    /// Advance the waterfall time-scroll one frame: convert the wall-clock
    /// elapsed since the last tick into a whole number of rows to append (at the
    /// configured rows/second), carrying the fraction. Returns the tuning the
    /// widget needs; the same rows/second also spaces the time gridlines, so the
    /// line and the waterfall move together. `has_frame` gates scrolling so a
    /// stalled stream doesn't keep duplicating rows.
    fn wf_tick(&mut self, has_frame: bool) -> spectrum_view::WfTuning {
        let now = now_unix_f64();
        let rows_per_sec = self.ui_settings.waterfall_rows_per_sec();
        // Clamp dt so a hitch/tab-away can't dump a huge run of rows at once.
        let dt = if self.wf_last_now > 0.0 { (now - self.wf_last_now).clamp(0.0, 0.3) } else { 0.0 };
        self.wf_last_now = now;
        let rows_to_write = if has_frame {
            self.wf_row_accum += dt as f32 * rows_per_sec;
            let n = self.wf_row_accum.floor();
            self.wf_row_accum -= n;
            (n as u32).min(32)
        } else {
            0
        };
        // Spectrum-line smoothing: convert the time constant to a per-frame EMA
        // coefficient using the frame rate, so the reaction time is the same at
        // any fps (0 tc = no smoothing = raw frames).
        let tc = self.ui_settings.spectrum_avg_tc();
        let fps = self.ui_settings.fps().max(1) as f32;
        let spectrum_alpha = if tc <= 0.0 { 1.0 } else { 1.0 - (-(1.0 / fps) / tc).exp() };
        spectrum_view::WfTuning { rows_to_write, rows_per_sec, now_unix: now, spectrum_alpha }
    }

    /// Hysteresis: is the config the engine already has still fine for the
    /// current view? (Avoids waterfall-clearing resends while panning.)
    fn cfg_still_good(&self) -> bool {
        let Some(sent) = self.sent_cfg else { return false };
        let ideal = self.desired_spectrum_cfg();
        if sent.fft_size != ideal.fft_size
            || sent.db_floor != ideal.db_floor
            || sent.db_ceil != ideal.db_ceil
            || sent.fps != ideal.fps
            || sent.avg_tc != ideal.avg_tc
        {
            return false;
        }
        match (sent.viewport, ideal.viewport) {
            (None, None) => true,
            (Some((slo, shi)), Some(_)) => {
                let full_span = self.state.sample_rate;
                let dev_lo = self.state.center_hz - full_span / 2.0;
                let dev_hi = self.state.center_hz + full_span / 2.0;
                let sspan = shi - slo;
                let margin = sspan * 0.05;
                // Inside with margin, unless the sent window is pinned to a
                // device edge on that side.
                let lo_ok = self.view.view_lo_hz >= slo + margin || slo <= dev_lo + 1.0;
                let hi_ok = self.view.view_hi_hz <= shi - margin || shi >= dev_hi - 1.0;
                let res = sspan / self.view.span().max(1.0);
                lo_ok && hi_ok && (1.15..=3.5).contains(&res)
            }
            _ => false,
        }
    }

    fn top_bar(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
        // All controls are captioned (or bare) modules that reflow when the
        // window is narrow. The frequency box is always first, the S-meter
        // second; the rest follow and wrap to further rows.
        ui.with_layout(
            egui::Layout::left_to_right(egui::Align::Min).with_main_wrap(true),
            |ui| {
                self.freq_module(ui, cmds);
                self.smeter_module(ui);
                self.band_mode_module(ui, cmds);
                self.vfo_module(ui, cmds);
                self.rit_module(ui, cmds);
                self.rx_module(ui, cmds);
                self.filter_module(ui, cmds);
                if self.caps.as_ref().is_some_and(|c| c.is_transmit_capable()) {
                    self.tx_module(ui, cmds);
                }
                self.display_module(ui, cmds);
                self.windows_module(ui);
            },
        );
    }

    /// The VFO frequency controls (A/B select + big readout + the inactive
    /// VFO's frequency) in a label-less box, always the first module.
    fn freq_module(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        crate::chrome::module_bare(ui, 452.0, |ui| {
            let active = self.state.active_vfo;
            for (v, label) in [(Vfo::A, "A"), (Vfo::B, "B")] {
                if crate::chrome::chip(ui, active == v, RichText::new(label).size(15.0)).clicked() {
                    cmds.push(Command::SelectVfo(v));
                }
            }
            if let Some(hz) =
                freq_display::show(ui, egui::Id::new("main-freq"), self.state.active_freq_hz())
            {
                cmds.push(Command::SetVfo { vfo: active, hz });
            }
            let inactive_hz = match active {
                Vfo::A => self.state.vfo_b_hz,
                Vfo::B => self.state.vfo_a_hz,
            };
            ui.label(
                RichText::new(format!("{:.6} MHz", inactive_hz / 1e6))
                    .monospace()
                    .size(12.0)
                    .color(Color32::from_gray(120)),
            );
        });
    }

    /// The S-meter in a label-less box, always pinned top-right. Clicking it
    /// toggles between the bar and analog-needle styles.
    fn smeter_module(&mut self, ui: &mut egui::Ui) {
        crate::chrome::module_bare(ui, 250.0, |ui| {
            let resp = smeter::show(ui, self.meters.as_ref(), self.view.smeter_analog)
                .on_hover_text("Click to switch bar / analog meter");
            if resp.clicked() {
                self.view.smeter_analog = !self.view.smeter_analog;
            }
        });
    }

    /// The CW-skimmer overlay: the current spots plus a parallel per-spot
    /// opacity that fades a box out over `SKIMMER_FADE_SECS` once it stops
    /// keying. Fully-faded spots are dropped so they free their lane.
    fn cw_overlay(&self, now: f64) -> (Vec<SkimmerSpot>, Vec<f32>) {
        let mut spots = Vec::new();
        let mut alpha = Vec::new();
        for s in &self.skimmer_spots {
            let a = if s.active {
                1.0
            } else {
                let last = self.skimmer_active_at.get(&s.id).copied().unwrap_or(now);
                (1.0 - (now - last) / SKIMMER_FADE_SECS).clamp(0.0, 1.0) as f32
            };
            if a <= 0.02 {
                continue;
            }
            spots.push(s.clone());
            alpha.push(a);
        }
        (spots, alpha)
    }

    /// Reuse the skimmer overlay to mark FT8/FT4 stations: one box per decoded
    /// callsign at its audio frequency (`dial + audio_hz`). The newest slot is
    /// solid; the previous slot is dimmed. Clicking a box sets the audio offset.
    fn ft8_overlay(&self) -> (Vec<SkimmerSpot>, Vec<f32>) {
        let mut spots = Vec::new();
        let mut alpha = Vec::new();
        let Some(latest) = self.digi_decodes.first().map(|d| d.slot_utc) else {
            return (spots, alpha);
        };
        let dial = self.state.rx_freq_hz();
        let mut seen = std::collections::HashSet::new();
        for d in &self.digi_decodes {
            // Decodes are newest-first; show only the last couple of slots.
            if latest - d.slot_utc > 30 {
                break;
            }
            let Some(call) = &d.from else { continue };
            if !seen.insert(call.clone()) {
                continue; // keep the most recent decode per callsign
            }
            let newest = d.slot_utc == latest;
            spots.push(SkimmerSpot {
                id: hash_call(call),
                kind: SkimmerKind::Cw,
                freq_hz: dial + d.audio_hz as f64,
                callsign: Some(call.clone()),
                text: d.message.clone(),
                snr_db: d.snr_db,
                wpm: 0,
                active: newest,
            });
            alpha.push(if newest { 1.0 } else { 0.5 });
        }
        (spots, alpha)
    }

    /// One button opening a floating popup with the band + mode + filter
    /// preset button rows.
    fn band_mode_module(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        crate::chrome::module(ui, "Band / Mode", 128.0, |ui| {
            let mode = self.state.rx[0].mode;
            let summary = format!("{} · {}", self.state.band.label(), mode.label());
            let btn = crate::chrome::chip(ui, false, RichText::new(summary).size(14.0));

            egui::Popup::from_toggle_button_response(&btn)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .show(|ui| {
                    ui.set_max_width(430.0);
                    ui.label(RichText::new("BAND").color(crate::theme::CYAN_DIM).size(9.5).strong());
                    let digital = mode.is_digital();
                    ui.horizontal_wrapped(|ui| {
                        for b in Band::ALL {
                            // In a digital mode, a band button tunes to that
                            // band's FT8/FT4 dial frequency (SetVfo keeps the
                            // mode); otherwise it's a normal band change. Bands
                            // with no standard digital frequency are disabled.
                            let digi_hz =
                                if digital { digi_freq_for_band(mode, b) } else { None };
                            let cap_ok = self.caps.as_ref().is_none_or(|c| {
                                b.edges().is_none_or(|(lo, hi)| c.can_rx_hz(lo) || c.can_rx_hz(hi))
                            });
                            let enabled = cap_ok && (!digital || digi_hz.is_some());
                            let active = match digi_hz {
                                Some(hz) => (self.state.active_freq_hz() - hz).abs() < 500.0,
                                None => !digital && self.state.band == b,
                            };
                            let clicked = ui
                                .add_enabled_ui(enabled, |ui| {
                                    crate::chrome::chip(ui, active, b.label())
                                })
                                .inner
                                .clicked();
                            if clicked {
                                match digi_hz {
                                    Some(hz) => cmds.push(Command::SetVfo {
                                        vfo: self.state.active_vfo,
                                        hz,
                                    }),
                                    None => cmds.push(Command::SetBand(b)),
                                }
                            }
                        }
                    });
                    ui.add_space(6.0);
                    ui.label(RichText::new("MODE").color(crate::theme::CYAN_DIM).size(9.5).strong());
                    ui.horizontal_wrapped(|ui| {
                        for m in [Mode::Lsb, Mode::Usb, Mode::Cw, Mode::Am, Mode::Sam,
                                  Mode::Nfm, Mode::Wfm, Mode::Digu, Mode::Digl, Mode::Dsb, Mode::Spec] {
                            if crate::chrome::chip(ui, mode == m, m.label()).clicked() {
                                cmds.push(Command::SetMode { rx: RxId::Main, mode: m });
                            }
                        }
                    });
                    ui.add_space(6.0);
                    ui.label(RichText::new("DIGITAL").color(crate::theme::CYAN_DIM).size(9.5).strong());
                    ui.horizontal_wrapped(|ui| {
                        for m in Mode::DIGITAL {
                            if crate::chrome::chip(ui, mode == m, m.label()).clicked() {
                                cmds.push(Command::SetMode { rx: RxId::Main, mode: m });
                            }
                        }
                    });
                });
        });
    }

    fn vfo_module(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        crate::chrome::module(ui, "VFO", 244.0, |ui| {
            if crate::chrome::chip(ui, false, "A↔B").on_hover_text("Swap VFOs").clicked() {
                cmds.push(Command::SwapVfos);
            }
            if crate::chrome::chip(ui, false, "A→B").on_hover_text("Copy A to B").clicked() {
                cmds.push(Command::CopyAtoB);
            }
            if crate::chrome::chip(ui, self.state.split, "SPLIT").clicked() {
                cmds.push(Command::SetSplit(!self.state.split));
            }
            if crate::chrome::chip(ui, self.state.sub_rx_enabled, "SUB")
                .on_hover_text("Sub receiver on the inactive VFO (right ear)")
                .clicked()
            {
                cmds.push(Command::SetSubRx(!self.state.sub_rx_enabled));
            }
        });
    }

    fn rit_module(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let tx_capable = self.caps.as_ref().is_some_and(|c| c.is_transmit_capable());
        let width = if tx_capable { 340.0 } else { 176.0 };
        crate::chrome::module(ui, "RIT / XIT", width, |ui| {
            let rit = self.state.rit;
            if crate::chrome::chip(ui, rit.enabled, "RIT").clicked() {
                cmds.push(Command::SetRit { enabled: !rit.enabled, hz: rit.hz });
            }
            let mut rit_hz = rit.hz;
            if ui
                .add(DragValue::new(&mut rit_hz).speed(5).range(-9999..=9999).suffix(" Hz"))
                .changed()
            {
                cmds.push(Command::SetRit { enabled: rit.enabled, hz: rit_hz });
            }
            if tx_capable {
                let xit = self.state.xit;
                if crate::chrome::chip(ui, xit.enabled, "XIT").clicked() {
                    cmds.push(Command::SetXit { enabled: !xit.enabled, hz: xit.hz });
                }
                let mut xit_hz = xit.hz;
                if ui
                    .add(DragValue::new(&mut xit_hz).speed(5).range(-9999..=9999).suffix(" Hz"))
                    .changed()
                {
                    cmds.push(Command::SetXit { enabled: xit.enabled, hz: xit_hz });
                }
            }
        });
    }

    fn rx_module(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        crate::chrome::module(ui, "Receiver", 300.0, |ui| {
            let agc = self.state.rx[0].agc;
            ComboBox::from_id_salt("agc")
                .selected_text(format!("AGC {}", agc.label()))
                .width(88.0)
                .show_ui(ui, |ui| {
                    for a in AgcMode::ALL {
                        if ui.selectable_label(agc == a, a.label()).clicked() {
                            cmds.push(Command::SetAgc { rx: RxId::Main, agc: a });
                        }
                    }
                });

            let mut vol = self.state.rx[0].volume;
            ui.label("Vol");
            if crate::chrome::slider(ui, Slider::new(&mut vol, 0.0..=1.0).show_value(false))
                .changed()
            {
                self.state.rx[0].volume = vol; // optimistic echo
                cmds.push(Command::SetVolume { rx: RxId::Main, v: vol });
            }
            let muted = self.state.rx[0].muted;
            if crate::chrome::chip_accent(ui, muted, "MUTE", crate::theme::PINK, Color32::WHITE)
                .clicked()
            {
                cmds.push(Command::SetMute { rx: RxId::Main, muted: !muted });
            }
        });
    }

    fn filter_module(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        crate::chrome::module(ui, "Filter / Noise", 178.0, |ui| {
            let mut sql = self.state.rx[0].squelch_db;
            ui.label("SQL");
            if crate::chrome::slider(
                ui,
                Slider::new(&mut sql, sdroxide_types::SQUELCH_OPEN_DB..=-30.0)
                    .show_value(true)
                    .custom_formatter(|v, _| {
                        if v <= (sdroxide_types::SQUELCH_OPEN_DB + 1.0) as f64 {
                            "off".into()
                        } else {
                            format!("{v:.0}")
                        }
                    }),
            )
            .changed()
            {
                self.state.rx[0].squelch_db = sql; // optimistic echo
                cmds.push(Command::SetSquelch { rx: RxId::Main, db: sql });
            }
            let nb = self.state.noise_blanker;
            if crate::chrome::chip(ui, nb, "NB")
                .on_hover_text("Impulse noise blanker")
                .clicked()
            {
                cmds.push(Command::SetNoiseBlanker(!nb));
            }
        });
    }

    fn tx_module(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        crate::chrome::module(ui, "Transmit", 470.0, |ui| {
            let tx = self.state.tx;
            if crate::chrome::chip_accent(
                ui,
                tx.ptt,
                RichText::new(" PTT ").size(15.0).strong(),
                crate::theme::PINK,
                Color32::WHITE,
            )
            .clicked()
            {
                cmds.push(Command::SetPtt(!tx.ptt));
            }
            if crate::chrome::chip_accent(
                ui,
                tx.tune,
                RichText::new(" TUNE ").size(15.0),
                crate::theme::YELLOW,
                crate::theme::INK_ON_CYAN,
            )
            .clicked()
            {
                cmds.push(Command::SetTune(!tx.tune));
            }
            let mut drive = tx.drive;
            ui.label("Drive");
            if crate::chrome::slider(
                ui,
                Slider::new(&mut drive, 0.0..=1.0)
                    .show_value(true)
                    .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
            )
            .changed()
            {
                cmds.push(Command::SetTxDrive(drive));
            }
            let mut tune_drive = tx.tune_drive;
            ui.label("Tune");
            if crate::chrome::slider(
                ui,
                Slider::new(&mut tune_drive, 0.0..=1.0)
                    .show_value(true)
                    .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
            )
            .changed()
            {
                cmds.push(Command::SetTuneDrive(tune_drive));
            }
            let mut mic = tx.mic_gain;
            ui.label("Mic");
            if crate::chrome::slider(ui, Slider::new(&mut mic, 0.0..=1.0).show_value(false))
                .changed()
            {
                cmds.push(Command::SetMicGain(mic));
            }
        });
    }

    /// Auto-set floor/ceiling from the current frame for best waterfall
    /// contrast (noise dark, signals visible, no over-blow). Only the bins
    /// inside the visible viewport are considered, so signals scrolled or
    /// zoomed off-screen (e.g. a strong broadcaster) don't skew the levels —
    /// the emitted frame carries slack beyond the view.
    fn auto_levels(&mut self) {
        let result = {
            let Some(f) = self.frame.as_ref() else { return };
            let n = f.bins.len();
            if n == 0 || f.span_hz <= 0.0 {
                return;
            }
            let base = f.center_hz - f.span_hz / 2.0;
            let to_idx = |hz: f64| (hz - base) / f.span_hz * n as f64;
            let i_lo = (to_idx(self.view.view_lo_hz).floor().max(0.0) as usize).min(n);
            let i_hi = (to_idx(self.view.view_hi_hz).ceil().max(0.0) as usize).min(n);
            let slice = if i_hi > i_lo { &f.bins[i_lo..i_hi] } else { &f.bins[..] };
            pick_levels(slice, f.db_floor, f.db_ceil)
        };
        if let Some((floor, ceil)) = result {
            self.view.db_floor = floor;
            self.view.db_ceil = ceil;
        }
    }

    fn display_module(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        crate::chrome::module(ui, "Display", 226.0, |ui| {
            if crate::chrome::chip(ui, false, "FIT")
                .on_hover_text("Auto-set floor/ceiling for best waterfall contrast")
                .clicked()
            {
                self.auto_levels();
            }
            if crate::chrome::chip(ui, self.view.peak_hold, "PEAK")
                .on_hover_text("Decaying peak-hold trace")
                .clicked()
            {
                self.view.peak_hold = !self.view.peak_hold;
            }
            // Lit when the spectrum line is visible (not collapsed).
            if crate::chrome::chip(ui, !self.view.spectrum_collapsed, "SPEC")
                .on_hover_text("Show/hide the spectrum line above the waterfall")
                .clicked()
            {
                self.view.spectrum_collapsed = !self.view.spectrum_collapsed;
            }
            let skim = self.state.skimmer_enabled;
            if crate::chrome::chip(ui, skim, "SKIM")
                .on_hover_text("Decode CW signals in the CW band segments and mark them on the waterfall")
                .clicked()
            {
                cmds.push(Command::SetSkimmerEnabled(!skim));
            }
        });
        crate::chrome::module(ui, "FFT", 344.0, |ui| {
            ui.label("floor");
            ui.add(
                DragValue::new(&mut self.view.db_floor).speed(1.0).range(-160.0..=-40.0).suffix(" dB"),
            );
            ui.label("ceil");
            ui.add(
                DragValue::new(&mut self.view.db_ceil).speed(1.0).range(-100.0..=20.0).suffix(" dB"),
            );
            ComboBox::from_id_salt("fft")
                .selected_text(format!("FFT {}", self.view.fft_size))
                .width(88.0)
                .show_ui(ui, |ui| {
                    for n in [2048u32, 4096, 8192, 16384, 32768] {
                        ui.selectable_value(&mut self.view.fft_size, n, format!("{n}"));
                    }
                });
            ComboBox::from_id_salt("colormap")
                .selected_text(colormap::NAMES[self.view.colormap.min(colormap::NAMES.len() - 1)])
                .width(88.0)
                .show_ui(ui, |ui| {
                    for (i, name) in colormap::NAMES.iter().enumerate() {
                        ui.selectable_value(&mut self.view.colormap, i, *name);
                    }
                });
        });
    }

    fn windows_module(&mut self, ui: &mut egui::Ui) {
        crate::chrome::module(ui, "System", 300.0, |ui| {
            if crate::chrome::chip(ui, self.show_logbook, "LOG")
                .on_hover_text("Logbook — all QSOs (digital + manual)")
                .clicked()
            {
                self.show_logbook = !self.show_logbook;
            }
            if crate::chrome::chip(ui, self.show_memories, "MEM")
                .on_hover_text("Memory channels")
                .clicked()
            {
                self.show_memories = !self.show_memories;
            }
            if crate::chrome::chip(ui, self.show_settings, "⚙ SETTINGS")
                .on_hover_text("Settings — device gains, antennas, audio devices")
                .clicked()
            {
                self.show_settings = !self.show_settings;
            }
        });
    }

    /// Center the view on the tuned frequency after big jumps (band change,
    /// memory recall, startup) — i.e. whenever the tuning changed AND left
    /// the visible span. Deliberate pans away from the VFO are never
    /// snapped back, and drag-tuning keeps the VFO in view by itself.
    fn recenter_if_tuned_away(&mut self, prev_vfo: f64) {
        let vfo = self.state.active_freq_hz();
        let first = !self.seen_first_state;
        self.seen_first_state = true;
        if self.view.is_unset() {
            return; // spectrum_view will fit and center on first draw
        }
        let moved = (vfo - prev_vfo).abs() > 0.5;
        let outside = !(self.view.view_lo_hz..=self.view.view_hi_hz).contains(&vfo);
        if (moved || first) && outside {
            let span = self.view.span().min(self.state.sample_rate);
            self.view.view_lo_hz = vfo - span / 2.0;
            self.view.view_hi_hz = vfo + span / 2.0;
        }
    }

    /// Tuning and toggles from the keyboard (ignored while typing in a
    /// text field): ←/→ ±100 Hz (Shift: ±10), ↑/↓ ±1 kHz, PgUp/PgDn
    /// ±10 kHz, M mute, N noise blanker, F fit span.
    fn keyboard_shortcuts(&mut self, ctx: &egui::Context, cmds: &mut Vec<Command>) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        use egui::Key;
        let mut tune_step = 0.0f64;
        ctx.input(|i| {
            let fine = i.modifiers.shift;
            let small = if fine { 10.0 } else { 100.0 };
            if i.key_pressed(Key::ArrowRight) {
                tune_step += small;
            }
            if i.key_pressed(Key::ArrowLeft) {
                tune_step -= small;
            }
            if i.key_pressed(Key::ArrowUp) {
                tune_step += 1_000.0;
            }
            if i.key_pressed(Key::ArrowDown) {
                tune_step -= 1_000.0;
            }
            if i.key_pressed(Key::PageUp) {
                tune_step += 10_000.0;
            }
            if i.key_pressed(Key::PageDown) {
                tune_step -= 10_000.0;
            }
            if i.key_pressed(Key::M) {
                cmds.push(Command::SetMute {
                    rx: RxId::Main,
                    muted: !self.state.rx[0].muted,
                });
            }
            if i.key_pressed(Key::N) {
                cmds.push(Command::SetNoiseBlanker(!self.state.noise_blanker));
            }
            if i.key_pressed(Key::F) {
                self.view.fit(self.state.center_hz, self.state.sample_rate);
            }
        });
        if tune_step != 0.0 {
            cmds.push(Command::SetVfo {
                vfo: self.state.active_vfo,
                hz: (self.state.active_freq_hz() + tune_step).max(0.0),
            });
        }
    }

    /// The FT8/FT4 operating panel: decode list on the left, QSO area on the
    /// right. Sits below the (zoomed) waterfall in digital modes.
    fn digi_panel(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let avail = ui.available_size();
        ui.horizontal_top(|ui| {
            // Decode list takes the left ~52%; the QSO area gets the rest.
            // Force a top-down layout: `allocate_ui` would otherwise inherit
            // the parent `horizontal_top` (left-to-right) and lay the rows out
            // sideways, overflowing and shoving the QSO column off-screen.
            ui.allocate_ui_with_layout(
                egui::vec2(avail.x * 0.52, avail.y),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    self.decode_list(ui, cmds);
                },
            );
            ui.separator();
            ui.vertical(|ui| {
                self.qso_area(ui, cmds);
            });
        });
    }

    /// Touch-friendly decode list with a per-row REPLY button. Clicking a
    /// row moves the TX audio frequency to that signal; REPLY starts a QSO.
    fn decode_list(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("DECODES").size(9.5).strong().color(crate::theme::CYAN_DIM));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{} rx", self.digi_decodes.len()))
                        .size(10.0)
                        .color(Color32::from_gray(120)),
                );
            });
        });
        ui.add_space(2.0);
        // Call of the currently previewed decode (cloned so the scroll closure
        // doesn't hold a borrow of `self` we need to write back afterwards).
        let preview_call = self.digi_preview.as_ref().map(|(c, _)| c.clone());
        // Own grid, for the per-decode great-circle distance column.
        let my_grid = self.digi_status.as_ref().map(|s| s.config.my_grid.clone()).unwrap_or_default();
        // Staged preview change: `None` = no click this frame; `Some(v)` =
        // replace the preview with `v` (`Some(None)` clears it).
        let mut new_preview: Option<Option<(String, (f64, f64))>> = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for (i, d) in self.digi_decodes.iter().enumerate() {
                let cq = d.is_cq;
                let who = d.from.clone().unwrap_or_else(|| "?".into());
                let grid = d.grid.clone().unwrap_or_default();
                // Rough great-circle distance from my grid to the decode's grid.
                let dist_km = (!my_grid.is_empty())
                    .then(|| {
                        d.grid.as_deref().and_then(|g| sdroxide_types::grid_distance_km(&my_grid, g))
                    })
                    .flatten();
                let is_preview =
                    d.from.is_some() && preview_call.as_deref() == d.from.as_deref();
                let mut reply = false;
                // Left edge of the REPLY button, so the row-body click area can
                // exclude it (otherwise the full-row interaction below sits on
                // top of the button and swallows its clicks).
                let mut reply_left: Option<f32> = None;

                let inner = egui::Frame::new()
                    .fill(if cq { crate::theme::CQ_BG } else { crate::theme::ROW_BG })
                    .inner_margin(egui::Margin { left: 11, right: 6, top: 6, bottom: 6 })
                    .show(ui, |ui| {
                        // Fixed-width columns so every field lines up down the
                        // list. Right-aligned numbers, then callsign (wide
                        // proportional font), grid, and the message filling the
                        // rest with a right-pinned REPLY button.
                        let ch = 22.0;
                        ui.horizontal(|ui| {
                            ui.set_min_height(ch);
                            ui.spacing_mut().item_spacing.x = 7.0;
                            let cell = |ui: &mut egui::Ui, w: f32, align_right: bool, lbl: egui::Label| {
                                // Reserve the column width *exactly*: a plain
                                // allocate_ui shrinks to its content, so a short
                                // callsign would collapse the column and shift
                                // the grid + message out of alignment.
                                let (rect, _) =
                                    ui.allocate_exact_size(egui::vec2(w, ch), egui::Sense::hover());
                                let layout = if align_right {
                                    egui::Layout::right_to_left(egui::Align::Center)
                                } else {
                                    egui::Layout::left_to_right(egui::Align::Center)
                                };
                                let mut child =
                                    ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(layout));
                                child.add(lbl);
                            };
                            // SNR.
                            cell(
                                ui,
                                28.0,
                                true,
                                egui::Label::new(
                                    RichText::new(format!("{:+}", d.snr_db))
                                        .monospace()
                                        .size(13.0)
                                        .color(snr_color(d.snr_db)),
                                ),
                            );
                            // Audio frequency.
                            cell(
                                ui,
                                40.0,
                                true,
                                egui::Label::new(
                                    RichText::new(format!("{:.0}", d.audio_hz))
                                        .monospace()
                                        .size(12.0)
                                        .color(Color32::from_gray(120)),
                                ),
                            );
                            // Callsign — wider proportional (button) font.
                            cell(
                                ui,
                                98.0,
                                false,
                                egui::Label::new(
                                    RichText::new(&who).size(15.0).strong().color(if cq {
                                        crate::theme::GREEN
                                    } else {
                                        crate::theme::TEXT_STRONG
                                    }),
                                )
                                .truncate(),
                            );
                            // Grid.
                            cell(
                                ui,
                                44.0,
                                false,
                                egui::Label::new(
                                    RichText::new(&grid)
                                        .monospace()
                                        .size(12.0)
                                        .color(crate::theme::CYAN_DIM),
                                ),
                            );
                            // Distance (km, great-circle from my grid).
                            cell(
                                ui,
                                58.0,
                                true,
                                egui::Label::new(
                                    RichText::new(
                                        dist_km
                                            .map(|km| format!("{km:.0} km"))
                                            .unwrap_or_default(),
                                    )
                                    .monospace()
                                    .size(11.0)
                                    .color(crate::theme::YELLOW),
                                ),
                            );
                            // Message fills the remaining width; REPLY pinned right.
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let resp = crate::chrome::chip_accent(
                                    ui,
                                    false,
                                    RichText::new("REPLY").size(12.0).strong(),
                                    if cq { crate::theme::GREEN } else { crate::theme::CYAN },
                                    crate::theme::INK_ON_CYAN,
                                );
                                reply = resp.clicked();
                                reply_left = Some(resp.rect.left());
                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(&d.message)
                                                    .monospace()
                                                    .size(12.5)
                                                    .color(crate::theme::TEXT),
                                            )
                                            .truncate(),
                                        );
                                    },
                                );
                            });
                        });
                    });

                let r = inner.response.rect;
                // Red (CQ) / cyan left-accent bar.
                ui.painter().rect_filled(
                    egui::Rect::from_min_max(r.left_top(), egui::pos2(r.left() + 2.5, r.bottom())),
                    0.0,
                    if cq { crate::theme::PINK } else { crate::theme::CYAN_DIM },
                );
                // Row-body click (everything left of the REPLY button) tunes
                // the audio freq. Excluding the button's rect keeps this
                // interaction from covering — and stealing clicks from — REPLY.
                let body_right = reply_left.map(|x| x - 2.0).unwrap_or(r.right());
                let body_rect = egui::Rect::from_min_max(r.left_top(), egui::pos2(body_right, r.bottom()));
                let row = ui.interact(body_rect, ui.id().with(("dec", i)), egui::Sense::click());
                if is_preview {
                    // Amber outline ties this row to its faint map marker.
                    ui.painter().rect_stroke(
                        r,
                        0.0,
                        egui::Stroke::new(1.0, crate::theme::YELLOW),
                        egui::StrokeKind::Inside,
                    );
                } else if row.hovered() {
                    ui.painter().rect_stroke(
                        r,
                        0.0,
                        egui::Stroke::new(1.0, crate::theme::CYAN_DIM),
                        egui::StrokeKind::Inside,
                    );
                }
                if reply {
                    if let Some(from) = &d.from {
                        cmds.push(Command::DigiStartQso {
                            from: from.clone(),
                            grid: d.grid.clone(),
                            snr: d.snr_db,
                            audio_hz: d.audio_hz,
                        });
                    }
                    // Starting a QSO promotes the station to the active DX
                    // marker; drop the faint preview so they don't overlap.
                    new_preview = Some(None);
                } else if row.clicked() {
                    cmds.push(Command::SetDigiAudioFreq(d.audio_hz));
                    // Preview this station's location (if it sent a grid).
                    let ll = d.grid.as_deref().and_then(sdroxide_types::grid_to_latlon);
                    new_preview = Some(ll.map(|ll| (who.clone(), ll)));
                }
                ui.add_space(3.0);
            }
        });
        if let Some(sel) = new_preview {
            self.digi_preview = sel;
        }
    }

    /// The QSO operating area to the right of the decode list: header row,
    /// world map, station card, transcript, and action buttons.
    fn qso_area(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let status = self.digi_status.clone();
        let in_qso = status
            .as_ref()
            .map(|s| !matches!(s.step, sdroxide_types::QsoStep::Idle | sdroxide_types::QsoStep::Done))
            .unwrap_or(false);

        // Header: QSO left, session log + downloads centered, SETUP right.
        let logged = self.qso_log.len();
        let row_h = 26.0;
        let (row, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), row_h), egui::Sense::hover());
        let third = row.width() / 3.0;
        let zone = |i: f32| {
            egui::Rect::from_min_size(
                egui::pos2(row.left() + i * third, row.top()),
                egui::vec2(third, row_h),
            )
        };
        ui.scope_builder(
            egui::UiBuilder::new().max_rect(zone(0.0)).layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| {
                ui.label(RichText::new("QSO").size(9.5).strong().color(crate::theme::CYAN_DIM));
            },
        );
        ui.scope_builder(
            egui::UiBuilder::new().max_rect(zone(1.0)).layout(egui::Layout::top_down(egui::Align::Center)),
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("Session: {logged} QSO"))
                            .size(11.0)
                            .color(Color32::from_gray(150)),
                    );
                    if ui.add_enabled(logged > 0, egui::Button::new("ADIF")).clicked() {
                        let adif = sdroxide_types::qso_log_to_adif(&self.qso_log);
                        crate::download::save("sdroxide-log.adi", adif.as_bytes());
                    }
                    if ui.add_enabled(logged > 0, egui::Button::new("TXT")).clicked() {
                        let txt = sdroxide_types::qso_log_to_text(&self.qso_log);
                        crate::download::save("sdroxide-log.txt", txt.as_bytes());
                    }
                });
            },
        );
        ui.scope_builder(
            egui::UiBuilder::new().max_rect(zone(2.0)).layout(egui::Layout::right_to_left(egui::Align::Center)),
            |ui| {
                if crate::chrome::chip(ui, self.show_digi_settings, "⚙ SETUP").clicked() {
                    self.show_digi_settings = !self.show_digi_settings;
                }
            },
        );

        ui.add_space(5.0);
        // World map — drawn only if there is vertical room left for the
        // essentials below it. The action buttons and the QSO conversation must
        // always stay visible and usable, so on short windows the map shrinks
        // (see `worldmap::show`) and then disappears entirely.
        let btn_h = 44.0;
        let gap = 8.0;
        // Space the map must leave below itself: post-map gap + station card +
        // pre-transcript gap + a usable transcript + pre-button gap + buttons.
        const CARD_RESERVE: f32 = 66.0;
        const TRANSCRIPT_MIN: f32 = 56.0;
        let map_budget =
            ui.available_height() - (6.0 + CARD_RESERVE + 5.0 + TRANSCRIPT_MIN + gap + btn_h);
        let my_grid = status.as_ref().map(|s| s.config.my_grid.clone()).unwrap_or_default();
        let home_ll = sdroxide_types::grid_to_latlon(&my_grid);
        let dx_grid = status.as_ref().and_then(|s| s.dx_grid.clone());
        let dx_ll = dx_grid.as_deref().and_then(sdroxide_types::grid_to_latlon);
        // A clicked (but not yet answered) decode shows as a faint preview.
        let preview_ll = self.digi_preview.as_ref().map(|(_, ll)| *ll);
        let tx_active = status.as_ref().map(|s| s.transmitting).unwrap_or(false);
        if map_budget >= crate::widgets::worldmap::MIN_HEIGHT {
            crate::widgets::worldmap::show(ui, home_ll, dx_ll, preview_ll, tx_active, map_budget);
            ui.add_space(6.0);
        }
        // Station card.
        crate::chrome::red_panel(ui, |ui| {
            match status.as_ref() {
                Some(s) => {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(s.step.label()).size(13.0).strong().color(crate::theme::CYAN));
                        if s.transmitting {
                            ui.label(RichText::new("● TX").size(13.0).strong().color(crate::theme::PINK));
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!(
                                    "{:.0} Hz · {} slots",
                                    s.audio_hz,
                                    if s.tx_even { "even" } else { "odd" }
                                ))
                                .size(11.0)
                                .color(Color32::from_gray(140)),
                            );
                        });
                    });
                    match &s.dx_call {
                        Some(dx) => {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(dx)
                                        .size(17.0)
                                        .strong()
                                        .color(crate::theme::TEXT_STRONG),
                                );
                                if let Some(g) = &s.dx_grid {
                                    ui.label(RichText::new(g).size(13.0).color(crate::theme::CYAN_DIM));
                                }
                                if let (Some(hg), Some(dg)) = (
                                    (!my_grid.is_empty()).then_some(my_grid.as_str()),
                                    s.dx_grid.as_deref(),
                                ) {
                                    if let (Some(km), Some(brg)) = (
                                        sdroxide_types::grid_distance_km(hg, dg),
                                        sdroxide_types::grid_bearing(hg, dg),
                                    ) {
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.label(
                                                    RichText::new(format!(
                                                        "{:.0} km · {:.0}°",
                                                        km, brg
                                                    ))
                                                    .size(12.0)
                                                    .color(crate::theme::YELLOW),
                                                );
                                            },
                                        );
                                    }
                                }
                            });
                        }
                        None => {
                            ui.label(
                                RichText::new("no active QSO — pick a decode to reply, or Call CQ")
                                    .size(11.0)
                                    .color(Color32::from_gray(120)),
                            );
                        }
                    }
                }
                None => {
                    ui.label(RichText::new("FT8 engine idle").size(12.0).color(Color32::from_gray(130)));
                }
            }
        });

        // Transcript: a red-bordered scroll box that always fills the space
        // between the station card and the action buttons (reserve the button
        // row height first, give the rest to the transcript).
        ui.add_space(5.0);
        // Reserve the button row (+gap) at the bottom so the action buttons stay
        // visible no matter how short the window is; the transcript takes the
        // rest. Floor at 0 (not a fixed minimum) so a very short window shrinks
        // the conversation rather than pushing the buttons off-screen.
        let trans_h = (ui.available_height() - btn_h - gap).max(0.0);
        ui.allocate_ui(egui::vec2(ui.available_width(), trans_h), |ui| {
            let inner = egui::Frame::new()
                .fill(crate::theme::ROW_BG)
                .stroke(egui::Stroke::new(1.0, crate::theme::RED_DEEP))
                .inner_margin(egui::Margin { left: 9, right: 7, top: 6, bottom: 6 })
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.set_min_height(ui.available_height());
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            let mut any = false;
                            if let Some(s) = status.as_ref() {
                                for line in &s.transcript {
                                    any = true;
                                    let (tag, col) = if line.tx {
                                        ("»", crate::theme::YELLOW)
                                    } else {
                                        ("«", crate::theme::GREEN)
                                    };
                                    ui.label(
                                        RichText::new(format!("{tag} {}", line.text))
                                            .monospace()
                                            .size(12.5)
                                            .color(col),
                                    );
                                }
                                if let Some(msg) = &s.tx_pending_msg {
                                    any = true;
                                    ui.label(
                                        RichText::new(format!("→ {msg}"))
                                            .monospace()
                                            .size(11.5)
                                            .color(Color32::from_gray(150)),
                                    );
                                }
                            }
                            if !any {
                                ui.label(
                                    RichText::new("— no messages —")
                                        .monospace()
                                        .size(11.5)
                                        .color(Color32::from_gray(90)),
                                );
                            }
                        });
                });
            // Red left-accent bar (matching chrome::red_panel).
            let r = inner.response.rect;
            ui.painter().rect_filled(
                egui::Rect::from_min_max(r.left_top(), egui::pos2(r.left() + 2.5, r.bottom())),
                0.0,
                crate::theme::PINK,
            );
        });

        ui.add_space(gap);
        // Action buttons (larger for touch).
        ui.horizontal(|ui| {
            let cq = ui.add_enabled_ui(!in_qso, |ui| {
                crate::chrome::chip_accent(
                    ui,
                    false,
                    RichText::new("  CALL CQ  ").size(15.0).strong(),
                    crate::theme::GREEN,
                    crate::theme::INK_ON_CYAN,
                )
            });
            if cq.inner.clicked() {
                cmds.push(Command::DigiCallCq);
            }
            if crate::chrome::chip(ui, false, RichText::new(" STOP QSO ").size(14.0)).clicked() {
                cmds.push(Command::DigiStopQso);
            }
            if crate::chrome::chip_accent(
                ui,
                false,
                RichText::new(" STOP TX ").size(15.0).strong(),
                crate::theme::PINK,
                Color32::WHITE,
            )
            .clicked()
            {
                cmds.push(Command::DigiAbortTx);
            }
        });
    }

    /// PSK/RTTY keyboard-mode panel: the decoded RX stream on top, then a
    /// streaming TX input (already-sent characters shown green) and controls.
    fn text_modem_panel(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let status = self.digi_status.clone();
        let mode = self.state.rx[0].mode;
        let audio_hz = status.as_ref().map(|s| s.audio_hz).unwrap_or(1500.0);
        let sent = status.as_ref().map(|s| s.tx_sent).unwrap_or(0);
        let tx_on = status.as_ref().map(|s| s.tx_next).unwrap_or(false);
        let transmitting = status.as_ref().map(|s| s.transmitting).unwrap_or(false);
        let rx_text = status.as_ref().map(|s| s.text_rx.clone()).unwrap_or_default();
        let my_call = status.as_ref().map(|s| s.config.my_call.clone()).unwrap_or_default();

        // Header: mode + tuning readout / nudges, SETUP + TX indicator.
        ui.horizontal(|ui| {
            ui.label(RichText::new(mode.label()).size(11.0).strong().color(crate::theme::CYAN));
            ui.label(
                RichText::new(format!("{audio_hz:.0} Hz")).size(11.0).color(Color32::from_gray(150)),
            );
            if crate::chrome::chip(ui, false, "−").on_hover_text("Tune down 10 Hz").clicked() {
                cmds.push(Command::SetDigiAudioFreq((audio_hz - 10.0).clamp(200.0, 3500.0)));
            }
            if crate::chrome::chip(ui, false, "+").on_hover_text("Tune up 10 Hz").clicked() {
                cmds.push(Command::SetDigiAudioFreq((audio_hz + 10.0).clamp(200.0, 3500.0)));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if crate::chrome::chip(ui, self.show_digi_settings, "⚙ SETUP").clicked() {
                    self.show_digi_settings = !self.show_digi_settings;
                }
                if transmitting {
                    ui.label(RichText::new("● TX").size(11.0).strong().color(crate::theme::PINK));
                }
            });
        });
        ui.add_space(4.0);

        // Reserve the input + button rows at the bottom; RX stream gets the rest.
        let btn_h = 34.0;
        let input_h = 64.0;
        let gap = 6.0;
        let rx_h = (ui.available_height() - btn_h - input_h - 2.0 * gap).max(40.0);

        ui.allocate_ui(egui::vec2(ui.available_width(), rx_h), |ui| {
            egui::Frame::new()
                .fill(crate::theme::ROW_BG)
                .stroke(egui::Stroke::new(1.0, crate::theme::RED_DEEP))
                .inner_margin(egui::Margin { left: 8, right: 7, top: 6, bottom: 6 })
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.set_min_height(ui.available_height());
                    egui::ScrollArea::vertical().auto_shrink([false, false]).stick_to_bottom(true).show(
                        ui,
                        |ui| {
                            if rx_text.is_empty() {
                                ui.label(
                                    RichText::new("— listening —")
                                        .monospace()
                                        .size(12.0)
                                        .color(Color32::from_gray(90)),
                                );
                            } else {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&rx_text)
                                            .monospace()
                                            .size(12.5)
                                            .color(crate::theme::GREEN),
                                    )
                                    .wrap(),
                                );
                            }
                        },
                    );
                });
        });
        ui.add_space(gap);

        // TX input: already-sent characters are coloured green via a layouter.
        let prev = self.text_tx.clone();
        let sent = sent.min(prev.chars().count());
        let prefix: String = prev.chars().take(sent).collect();
        let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap: f32| {
            let text = buf.as_str();
            let sent_byte =
                text.char_indices().nth(sent).map(|(i, _)| i).unwrap_or(text.len());
            let mut job = egui::text::LayoutJob::default();
            job.wrap.max_width = wrap;
            let mono = egui::FontId::monospace(13.0);
            if sent_byte > 0 {
                job.append(
                    &text[..sent_byte],
                    0.0,
                    egui::TextFormat { font_id: mono.clone(), color: crate::theme::GREEN, ..Default::default() },
                );
            }
            if sent_byte < text.len() {
                job.append(
                    &text[sent_byte..],
                    0.0,
                    egui::TextFormat { font_id: mono.clone(), color: crate::theme::TEXT_STRONG, ..Default::default() },
                );
            }
            ui.fonts_mut(|f| f.layout_job(job))
        };
        let resp = ui.add_sized(
            egui::vec2(ui.available_width(), input_h),
            egui::TextEdit::multiline(&mut self.text_tx)
                .layouter(&mut layouter)
                .hint_text("Type here to transmit…"),
        );
        if resp.changed() {
            // Protect the already-transmitted prefix from edits.
            if !self.text_tx.starts_with(&prefix) {
                self.text_tx = prev;
            }
            cmds.push(Command::DigiTxText(self.text_tx.clone()));
        }
        ui.add_space(gap);

        // Controls.
        ui.horizontal(|ui| {
            let label = if tx_on { "  TX ON  " } else { "   TX   " };
            if crate::chrome::chip_accent(
                ui,
                tx_on,
                RichText::new(label).size(14.0).strong(),
                crate::theme::PINK,
                Color32::WHITE,
            )
            .clicked()
            {
                cmds.push(Command::DigiTxActive(!tx_on));
            }
            if crate::chrome::chip_accent(
                ui,
                false,
                RichText::new(" CALL CQ ").size(13.0).strong(),
                crate::theme::GREEN,
                crate::theme::INK_ON_CYAN,
            )
            .clicked()
            {
                // Own the CQ text so the green sent-progress shows locally.
                let call = if my_call.is_empty() { "NOCALL".to_string() } else { my_call.clone() };
                let cq = format!("CQ CQ CQ DE {call} {call} {call} PSE K\n");
                cmds.push(Command::DigiAbortTx);
                self.text_tx = cq.clone();
                cmds.push(Command::DigiTxText(cq));
                cmds.push(Command::DigiTxActive(true));
            }
            if crate::chrome::chip(ui, false, " CLEAR ").clicked() {
                self.text_tx.clear();
                cmds.push(Command::DigiAbortTx);
                cmds.push(Command::DigiTxText(String::new()));
            }
        });
    }

    /// Own-call / grid / message-template editor (and RTTY parameters).
    fn digi_settings_window(&mut self, ctx: &egui::Context, cmds: &mut Vec<Command>) {
        let mut open = self.show_digi_settings;
        let is_rtty = self.state.rx[0].mode == Mode::Rtty;
        let title = if self.state.rx[0].mode.is_text_modem() { "PSK / RTTY Setup" } else { "FT8 / FT4 Setup" };
        let resp = egui::Window::new(title)
            .open(&mut open)
            .frame(crate::chrome::window_frame())
            .resizable(false)
            .default_width(420.0)
            .show(ctx, |ui| {
                // Edit the UI-owned copy so keystrokes aren't clobbered by the
                // engine's status echo; persist on any change.
                let cfg = &mut self.digi_cfg_edit;
                let mut changed = false;
                egui::Grid::new("digi-cfg").num_columns(2).show(ui, |ui| {
                    ui.label("My callsign");
                    if ui.text_edit_singleline(&mut cfg.my_call).changed() {
                        cfg.my_call = cfg.my_call.to_uppercase();
                        changed = true;
                    }
                    ui.end_row();
                    ui.label("My grid");
                    if ui.text_edit_singleline(&mut cfg.my_grid).changed() {
                        changed = true;
                    }
                    ui.end_row();
                    ui.label("TX period");
                    ui.horizontal(|ui| {
                        changed |= ui.selectable_value(&mut cfg.tx_even, true, "Even").changed();
                        changed |= ui.selectable_value(&mut cfg.tx_even, false, "Odd").changed();
                    });
                    ui.end_row();
                    ui.label("Auto-sequence");
                    changed |= ui.checkbox(&mut cfg.auto_seq, "").changed();
                    ui.end_row();
                });
                ui.separator();
                ui.label(
                    RichText::new("Message templates  {MYCALL} {MYGRID} {DX} {REPORT}")
                        .size(10.5)
                        .color(Color32::from_gray(150)),
                );
                egui::Grid::new("digi-msgs").num_columns(2).show(ui, |ui| {
                    for (label, field) in [
                        ("CQ", &mut cfg.msg_cq),
                        ("Grid", &mut cfg.msg_grid),
                        ("Report", &mut cfg.msg_report),
                        ("R+Report", &mut cfg.msg_rreport),
                        ("RR73", &mut cfg.msg_rr73),
                        ("73", &mut cfg.msg_73),
                    ] {
                        ui.label(label);
                        changed |= ui.text_edit_singleline(field).changed();
                        ui.end_row();
                    }
                });
                if is_rtty {
                    ui.separator();
                    ui.label(RichText::new("RTTY").size(10.5).strong().color(crate::theme::CYAN_DIM));
                    egui::Grid::new("rtty-cfg").num_columns(2).show(ui, |ui| {
                        ui.label("Shift (Hz)");
                        ui.horizontal(|ui| {
                            for s in [170.0f32, 425.0, 850.0] {
                                let sel = (cfg.rtty_shift_hz - s).abs() < 0.5;
                                if ui.selectable_label(sel, format!("{s:.0}")).clicked() {
                                    cfg.rtty_shift_hz = s;
                                    changed = true;
                                }
                            }
                        });
                        ui.end_row();
                        ui.label("Baud");
                        ui.horizontal(|ui| {
                            for b in [45.45f32, 50.0, 75.0] {
                                let sel = (cfg.rtty_baud - b).abs() < 0.5;
                                let lbl = if (b - 45.45).abs() < 0.5 { "45".to_string() } else { format!("{b:.0}") };
                                if ui.selectable_label(sel, lbl).clicked() {
                                    cfg.rtty_baud = b;
                                    changed = true;
                                }
                            }
                        });
                        ui.end_row();
                    });
                }
                if changed {
                    cmds.push(Command::SetDigiConfig(cfg.clone()));
                }
            });
        if let Some(r) = &resp {
            crate::chrome::paint_window_border(ctx, &r.response);
        }
        self.show_digi_settings = open;
    }

    fn memories_window(&mut self, ctx: &egui::Context, cmds: &mut Vec<Command>) {
        let mut open = self.show_memories;
        let resp = egui::Window::new("Memories")
            .open(&mut open)
            .frame(crate::chrome::window_frame())
            .resizable(true)
            .default_width(340.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.mem_name);
                    let name_ok = !self.mem_name.trim().is_empty();
                    if ui.add_enabled(name_ok, egui::Button::new("Store")).clicked() {
                        cmds.push(Command::StoreMemory { name: self.mem_name.trim().to_string() });
                        self.mem_name.clear();
                    }
                });
                ui.separator();
                if self.memories.is_empty() {
                    ui.label(RichText::new("no memories yet").color(Color32::from_gray(120)));
                }
                for m in &self.memories {
                    ui.horizontal(|ui| {
                        if crate::chrome::chip(ui, false, "RCL").on_hover_text("Recall").clicked() {
                            cmds.push(Command::RecallMemory(m.id));
                        }
                        ui.label(
                            RichText::new(format!(
                                "{:<12} {:>12.6} MHz  {}",
                                m.name,
                                m.freq_hz / 1e6,
                                m.mode.label()
                            ))
                            .monospace(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if crate::chrome::chip_accent(
                                ui,
                                false,
                                RichText::new("DEL").size(11.0),
                                crate::theme::PINK,
                                Color32::WHITE,
                            )
                            .on_hover_text("Delete")
                            .clicked()
                            {
                                cmds.push(Command::DeleteMemory(m.id));
                            }
                        });
                    });
                }
            });
        if let Some(r) = &resp {
            crate::chrome::paint_window_border(ctx, &r.response);
        }
        self.show_memories = open;
    }

    /// The logbook overlay: a session-grouped list of all QSOs (digital and
    /// manual), with add / edit / delete and ADIF/TXT export.
    fn logbook_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_logbook;
        let resp = egui::Window::new("LOGBOOK")
            .open(&mut open)
            .frame(crate::chrome::window_frame())
            .resizable(true)
            .default_width(720.0)
            .default_height(560.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let adding = self.log_edit.as_ref().is_some_and(|f| f.id == 0);
                    if crate::chrome::chip(ui, adding, "+ NEW ENTRY").clicked() {
                        let freq = self.state.rx_freq_hz();
                        let mode = self.state.rx[0].mode.label();
                        self.log_edit = Some(LogEditForm::new_entry(now_unix(), freq, mode));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let have = !self.qso_log.is_empty();
                        ui.add_enabled_ui(have, |ui| {
                            if crate::chrome::chip(ui, false, "TXT").clicked() {
                                let txt = sdroxide_types::qso_log_to_text(&self.qso_log);
                                crate::download::save("sdroxide-log.txt", txt.as_bytes());
                            }
                            if crate::chrome::chip(ui, false, "ADIF").clicked() {
                                let adif = sdroxide_types::qso_log_to_adif(&self.qso_log);
                                crate::download::save("sdroxide-log.adi", adif.as_bytes());
                            }
                        });
                        ui.label(
                            RichText::new(format!("{} QSO", self.qso_log.len()))
                                .size(11.0)
                                .color(Color32::from_gray(150)),
                        );
                    });
                });
                if self.log_edit.is_some() {
                    ui.add_space(4.0);
                    self.log_entry_form(ui);
                }
                ui.separator();
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    self.log_list(ui);
                });
            });
        if let Some(r) = &resp {
            crate::chrome::paint_window_border(ctx, &r.response);
        }
        self.show_logbook = open;
    }

    /// The new/edit entry form (shown inside the logbook when active).
    fn log_entry_form(&mut self, ui: &mut egui::Ui) {
        if self.log_edit.is_none() {
            return;
        }
        let mut action = 0u8; // 1 = save, 2 = cancel
        let mut set_now = false;
        {
            let f = self.log_edit.as_mut().unwrap();
            egui::Frame::new()
                .fill(crate::theme::ROW_BG)
                .stroke(egui::Stroke::new(1.0, crate::theme::RED_DEEP))
                .inner_margin(egui::Margin::same(9))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        RichText::new(if f.id == 0 { "NEW QSO" } else { "EDIT QSO" })
                            .size(11.0)
                            .strong()
                            .color(crate::theme::CYAN),
                    );
                    ui.add_space(4.0);
                    // Horizontal rows (not a Grid) so each field keeps its
                    // explicit width — a Grid redistributes column widths and
                    // squashes the narrow-looking ones.
                    let lbl = |ui: &mut egui::Ui, text: &str| {
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(72.0, 24.0), egui::Sense::hover());
                        ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(
                            egui::Layout::left_to_right(egui::Align::Center),
                        ))
                        .label(text);
                    };
                    let field = |ui: &mut egui::Ui, w: f32, s: &mut String| {
                        ui.add(egui::TextEdit::singleline(s).desired_width(w));
                    };
                    ui.horizontal(|ui| {
                        lbl(ui, "Call");
                        field(ui, 150.0, &mut f.call);
                        lbl(ui, "Grid");
                        field(ui, 120.0, &mut f.grid);
                    });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        lbl(ui, "Freq MHz");
                        field(ui, 150.0, &mut f.freq_mhz);
                        lbl(ui, "Mode");
                        field(ui, 120.0, &mut f.mode);
                    });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        lbl(ui, "RST sent");
                        field(ui, 150.0, &mut f.rst_sent);
                        lbl(ui, "RST rcvd");
                        field(ui, 120.0, &mut f.rst_rcvd);
                    });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        lbl(ui, "Date UTC");
                        field(ui, 150.0, &mut f.date);
                        lbl(ui, "Time");
                        field(ui, 90.0, &mut f.time);
                        if crate::chrome::chip(ui, false, "NOW").clicked() {
                            set_now = true;
                        }
                    });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        lbl(ui, "Comment");
                        field(ui, 500.0, &mut f.comment);
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if crate::chrome::chip_accent(
                            ui,
                            false,
                            RichText::new(" SAVE ").strong(),
                            crate::theme::GREEN,
                            crate::theme::INK_ON_CYAN,
                        )
                        .clicked()
                        {
                            action = 1;
                        }
                        if crate::chrome::chip(ui, false, "CANCEL").clicked() {
                            action = 2;
                        }
                    });
                });
            if set_now {
                let (y, mo, d, h, mi, _) = sdroxide_types::utc_ymd_hms(now_unix());
                f.date = format!("{y:04}-{mo:02}-{d:02}");
                f.time = format!("{h:02}:{mi:02}");
            }
        }
        match action {
            1 => {
                let (mc, mg) =
                    (self.digi_cfg_edit.my_call.clone(), self.digi_cfg_edit.my_grid.clone());
                if let Some(f) = self.log_edit.take() {
                    if let Some(rec) = f.to_record(&mc, &mg) {
                        if rec.id == 0 {
                            let mut rec = rec;
                            rec.id = self.next_log_id();
                            self.qso_log.push(rec);
                        } else if let Some(e) = self.qso_log.iter_mut().find(|q| q.id == rec.id) {
                            *e = rec;
                        }
                        persist_qso_log(&self.qso_log);
                    } else {
                        // Empty callsign — keep the form open for correction.
                        self.log_edit = Some(f);
                    }
                }
            }
            2 => self.log_edit = None,
            _ => {}
        }
    }

    /// The QSO list, grouped into daily sessions (newest first).
    fn log_list(&mut self, ui: &mut egui::Ui) {
        if self.qso_log.is_empty() {
            ui.add_space(8.0);
            ui.label(
                RichText::new("no QSOs yet — run FT8/FT4 or add a manual entry")
                    .color(Color32::from_gray(120)),
            );
            return;
        }
        let mut order: Vec<usize> = (0..self.qso_log.len()).collect();
        order.sort_by(|&a, &b| self.qso_log[b].start_utc.cmp(&self.qso_log[a].start_utc));

        let mut to_edit: Option<u64> = None;
        let mut to_delete: Option<u64> = None;

        let mut i = 0;
        while i < order.len() {
            let day = date_str(self.qso_log[order[i]].start_utc);
            let mut j = i;
            while j < order.len() && date_str(self.qso_log[order[j]].start_utc) == day {
                j += 1;
            }
            let group = &order[i..j];
            let newest = self.qso_log[group[0]].start_utc;
            let oldest = self.qso_log[group[group.len() - 1]].start_utc;
            // Session header.
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(&day).size(12.0).strong().color(crate::theme::CYAN));
                ui.label(
                    RichText::new(format!(
                        "{}–{} UTC · {} QSO",
                        time_str(oldest),
                        time_str(newest),
                        group.len()
                    ))
                    .size(10.5)
                    .color(Color32::from_gray(130)),
                );
            });
            ui.add_space(2.0);
            for &idx in group {
                let r = &self.qso_log[idx];
                let inner = egui::Frame::new()
                    .fill(crate::theme::ROW_BG)
                    .inner_margin(egui::Margin { left: 10, right: 6, top: 5, bottom: 5 })
                    .show(ui, |ui| {
                        ui.set_min_height(22.0);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            let col = |ui: &mut egui::Ui, w: f32, lbl: egui::Label| {
                                let (rect, _) = ui
                                    .allocate_exact_size(egui::vec2(w, 20.0), egui::Sense::hover());
                                let mut c = ui.new_child(
                                    egui::UiBuilder::new().max_rect(rect).layout(
                                        egui::Layout::left_to_right(egui::Align::Center),
                                    ),
                                );
                                c.add(lbl);
                            };
                            let gray = Color32::from_gray(150);
                            col(
                                ui,
                                40.0,
                                egui::Label::new(
                                    RichText::new(time_str(r.start_utc)).monospace().size(12.0).color(gray),
                                ),
                            );
                            col(
                                ui,
                                92.0,
                                egui::Label::new(
                                    RichText::new(&r.call)
                                        .size(14.0)
                                        .strong()
                                        .color(crate::theme::TEXT_STRONG),
                                )
                                .truncate(),
                            );
                            col(
                                ui,
                                42.0,
                                egui::Label::new(
                                    RichText::new(&r.band).monospace().size(11.5).color(gray),
                                ),
                            );
                            col(
                                ui,
                                48.0,
                                egui::Label::new(
                                    RichText::new(&r.mode).monospace().size(11.5).color(gray),
                                ),
                            );
                            let rst = format!(
                                "{}/{}",
                                r.rst_sent.map(|v| v.to_string()).unwrap_or_else(|| "–".into()),
                                r.rst_rcvd.map(|v| v.to_string()).unwrap_or_else(|| "–".into()),
                            );
                            col(
                                ui,
                                72.0,
                                egui::Label::new(RichText::new(rst).monospace().size(11.5).color(gray)),
                            );
                            col(
                                ui,
                                48.0,
                                egui::Label::new(
                                    RichText::new(r.grid.as_deref().unwrap_or(""))
                                        .monospace()
                                        .size(11.5)
                                        .color(crate::theme::CYAN_DIM),
                                ),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if crate::chrome::chip_accent(
                                        ui,
                                        false,
                                        RichText::new("DEL").size(11.0),
                                        crate::theme::PINK,
                                        Color32::WHITE,
                                    )
                                    .on_hover_text("Delete this entry")
                                    .clicked()
                                    {
                                        to_delete = Some(r.id);
                                    }
                                    if crate::chrome::chip(ui, false, RichText::new("EDIT").size(11.0))
                                        .clicked()
                                    {
                                        to_edit = Some(r.id);
                                    }
                                    if !r.comment.is_empty() {
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(&r.comment)
                                                    .size(11.5)
                                                    .color(Color32::from_gray(120)),
                                            )
                                            .truncate(),
                                        );
                                    }
                                },
                            );
                        });
                    });
                let rr = inner.response.rect;
                ui.painter().rect_filled(
                    egui::Rect::from_min_max(rr.left_top(), egui::pos2(rr.left() + 2.0, rr.bottom())),
                    0.0,
                    crate::theme::CYAN_DIM,
                );
                ui.add_space(2.0);
            }
            i = j;
        }

        if let Some(id) = to_delete {
            self.qso_log.retain(|q| q.id != id);
            persist_qso_log(&self.qso_log);
        } else if let Some(id) = to_edit {
            if let Some(r) = self.qso_log.iter().find(|q| q.id == id) {
                self.log_edit = Some(LogEditForm::from_record(r));
            }
        }
    }

    fn settings_window(&mut self, ctx: &egui::Context, cmds: &mut Vec<Command>) {
        // Query slow lists (cpal devices, serial ports, radio config) once per
        // dialog-open; a pick invalidates so the selection refreshes.
        if !self.show_settings {
            self.audio_devices = None;
            self.audio_devices_queried = false;
            return;
        } else if !self.audio_devices_queried {
            self.audio_devices = self.ctrl.audio_devices();
            self.radio_cfg = self.ctrl.radio_config();
            self.serial_ports = self.ctrl.serial_ports();
            self.audio_devices_queried = true;
        }

        // Edits collected here and applied after the window closure, which
        // borrows `&self` and so can't touch `&mut self.ctrl`.
        let mut audio_pick: Option<(bool, Option<String>)> = None;
        let mut hpsdr_discover = false;
        let mut tci_test = false;
        let mut radio_edit = self.radio_cfg.clone();
        let mut ui_edit = self.ui_settings;

        // The concrete interface types the user chooses between. SoapySDR only
        // appears when compiled in; there is no auto-detect (an unavailable
        // interface falls back to a null source so the user can reconfigure).
        let mut iface_opts: Vec<sdroxide_types::Backend> = Vec::new();
        if self.soapy_supported {
            iface_opts.push(sdroxide_types::Backend::Soapy);
        }
        iface_opts.push(sdroxide_types::Backend::Hpsdr);
        iface_opts.push(sdroxide_types::Backend::Cat);
        iface_opts.push(sdroxide_types::Backend::Tci);

        let mut tab = self.settings_tab;
        let mut open = self.show_settings;
        let resp = egui::Window::new("Settings")
            .open(&mut open)
            .frame(crate::chrome::window_frame())
            .resizable(false)
            .vscroll(true)
            .show(ctx, |ui| {
                self.settings_body(
                    ui,
                    cmds,
                    &iface_opts,
                    &mut radio_edit,
                    &mut audio_pick,
                    &mut hpsdr_discover,
                    &mut tci_test,
                    &mut ui_edit,
                    &mut tab,
                );
            });
        if let Some(r) = &resp {
            crate::chrome::paint_window_border(ctx, &r.response);
        }
        self.show_settings = open;
        self.settings_tab = tab;
        if let Some((output, name)) = audio_pick {
            self.ctrl.set_audio_device(output, name);
            self.audio_devices_queried = false;
        }
        if hpsdr_discover {
            // Blocking LAN scan (~1.5 s); done after the window closure so it can
            // take `&self.ctrl`. Results feed the device dropdown next frame.
            self.hpsdr_devices = self.ctrl.discover_hpsdr();
        }
        if tci_test {
            // Blocking connect (~up to 3 s); after the closure so it can take
            // `&self.ctrl`. The result is shown in the TCI section next frame.
            if let Some(cfg) = &radio_edit {
                self.tci_test_result = Some(self.ctrl.test_tci(&cfg.tci.address));
            }
        }
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
    }

    /// The Settings body: a Radio tab (one interface selector drives the
    /// interface-specific section) and an Audio tab (device selection).
    fn settings_body(
        &self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        iface_opts: &[sdroxide_types::Backend],
        radio_edit: &mut Option<sdroxide_types::RadioConfig>,
        audio_pick: &mut Option<(bool, Option<String>)>,
        hpsdr_discover: &mut bool,
        tci_test: &mut bool,
        ui_edit: &mut sdroxide_types::UiSettings,
        tab: &mut SettingsTab,
    ) {
        use sdroxide_types::Backend;

        ui.horizontal(|ui| {
            for (t, label) in [
                (SettingsTab::Radio, "Radio"),
                (SettingsTab::Audio, "Audio"),
                (SettingsTab::Ui, "UI"),
            ] {
                if crate::chrome::chip(ui, *tab == t, label).clicked() {
                    *tab = t;
                }
            }
        });
        ui.separator();

        let backend = radio_edit.as_ref().map(|c| c.backend);

        match tab {
            SettingsTab::Radio => {
                let Some(cfg) = radio_edit.as_mut() else {
                    ui.label("Radio configuration is only available in the native app.");
                    return;
                };
                // The single "which radio interface" selector.
                egui::Grid::new("iface-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                    ui.label(RichText::new("Radio interface").strong());
                    enum_combo(ui, "iface", &mut cfg.backend, iface_opts, Backend::label);
                    ui.end_row();
                });
                ui.separator();

                match cfg.backend {
                    Backend::Soapy => {
                        self.settings_device_tab(ui, cmds);
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(
                                "Choose the SoapySDR device with --device or device_args in \
                                 config.toml.",
                            )
                            .weak(),
                        );
                    }
                    Backend::Hpsdr => {
                        settings_hpsdr_tab(ui, &self.hpsdr_devices, radio_edit, hpsdr_discover)
                    }
                    Backend::Cat => settings_cat_tab(ui, &self.serial_ports, radio_edit),
                    Backend::Tci => {
                        settings_tci_tab(ui, radio_edit, tci_test, &self.tci_test_result)
                    }
                    // Legacy configs may still carry the removed auto-detect
                    // backend; prompt the user to pick a concrete interface.
                    Backend::Auto => {
                        ui.label(
                            RichText::new(
                                "Pick a radio interface above (this configuration used the \
                                 removed auto-detect mode).",
                            )
                            .weak(),
                        );
                    }
                }
            }
            SettingsTab::Audio => {
                self.settings_user_audio(ui, audio_pick);
                // The radio's own sound card is only used by the CAT / Audio interface.
                if backend == Some(Backend::Cat) {
                    if let (Some(devs), Some(cfg)) =
                        (self.audio_devices.as_ref(), radio_edit.as_mut())
                    {
                        ui.separator();
                        ui.label(RichText::new("Radio audio (sound card)").strong());
                        ui.label(RichText::new("Restart to apply.").weak());
                        egui::Grid::new("radio-audio").num_columns(2).spacing([12.0, 6.0]).show(
                            ui,
                            |ui| {
                                let (ci, co) =
                                    (cfg.radio_audio_in.clone(), cfg.radio_audio_out.clone());
                                ui.label("From radio (RX)");
                                device_combo(ui, "r-in", &devs.inputs, &ci, |n| {
                                    cfg.radio_audio_in = n
                                });
                                ui.end_row();
                                ui.label("To radio (TX)");
                                device_combo(ui, "r-out", &devs.outputs, &co, |n| {
                                    cfg.radio_audio_out = n
                                });
                                ui.end_row();
                            },
                        );
                    }
                }
            }
            SettingsTab::Ui => settings_ui_tab(ui, ui_edit),
        }
    }

    /// The user's own speakers / microphone (applied live).
    fn settings_user_audio(
        &self,
        ui: &mut egui::Ui,
        audio_pick: &mut Option<(bool, Option<String>)>,
    ) {
        let Some(devs) = &self.audio_devices else {
            return;
        };
        ui.label(RichText::new("Your audio (speakers / microphone)").strong());
        egui::Grid::new("user-audio").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label("Output");
            device_combo(ui, "u-out", &devs.outputs, &devs.selected_output, |n| {
                *audio_pick = Some((true, n))
            });
            ui.end_row();
            ui.label("Input");
            device_combo(ui, "u-in", &devs.inputs, &devs.selected_input, |n| {
                *audio_pick = Some((false, n))
            });
            ui.end_row();
        });
    }

    /// SoapySDR RX/TX gains + antenna (empty for a CAT rig).
    fn settings_device_tab(&self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let Some(caps) = &self.caps else {
            ui.label("no device");
            return;
        };
        ui.label(RichText::new(&caps.label).size(14.0).strong().color(crate::theme::CYAN));
        ui.add_space(6.0);
        if caps.gains.iter().all(|g| g.direction != Direction::Rx) {
            ui.label(RichText::new("This rig has no software-adjustable gains.").weak());
        }
        ui.label(RichText::new("RX gains").strong());
        egui::Grid::new("gains").num_columns(2).show(ui, |ui| {
            for g in caps.gains.iter().filter(|g| g.direction == Direction::Rx) {
                ui.label(&g.name);
                let mut db = self
                    .state
                    .gains
                    .iter()
                    .find(|(n, _)| *n == g.name)
                    .map(|(_, d)| *d)
                    .unwrap_or(g.min_db);
                let step = if g.step_db > 0.0 { g.step_db } else { 1.0 };
                if crate::chrome::slider(
                    ui,
                    Slider::new(&mut db, g.min_db..=g.max_db).step_by(step).suffix(" dB"),
                )
                .changed()
                {
                    cmds.push(Command::SetGain { dir: Direction::Rx, element: g.name.clone(), db });
                }
                ui.end_row();
            }
        });
        if caps.gains.iter().any(|g| g.direction == Direction::Tx) {
            ui.separator();
            ui.label(RichText::new("TX gains").strong().color(Color32::from_rgb(240, 90, 60)));
            egui::Grid::new("tx-gains").num_columns(2).show(ui, |ui| {
                for g in caps.gains.iter().filter(|g| g.direction == Direction::Tx) {
                    ui.label(&g.name);
                    let mut db = self
                        .state
                        .tx_gains
                        .iter()
                        .find(|(n, _)| *n == g.name)
                        .map(|(_, d)| *d)
                        .unwrap_or(g.min_db);
                    let step = if g.step_db > 0.0 { g.step_db } else { 1.0 };
                    if crate::chrome::slider(
                        ui,
                        Slider::new(&mut db, g.min_db..=g.max_db).step_by(step).suffix(" dB"),
                    )
                    .changed()
                    {
                        cmds.push(Command::SetGain {
                            dir: Direction::Tx,
                            element: g.name.clone(),
                            db,
                        });
                    }
                    ui.end_row();
                }
            });
        }
        if caps.antennas_rx.len() > 1 {
            ui.separator();
            ComboBox::from_id_salt("ant-rx")
                .selected_text(self.state.antenna_rx.clone())
                .show_ui(ui, |ui| {
                    for a in &caps.antennas_rx {
                        if ui.selectable_label(self.state.antenna_rx == *a, a).clicked() {
                            cmds.push(Command::SetAntenna { dir: Direction::Rx, name: a.clone() });
                        }
                    }
                });
        }
    }

}

/// A device dropdown ("System default" + names); calls `pick(Some(name)|None)`.
fn device_combo(
    ui: &mut egui::Ui,
    id: &str,
    names: &[String],
    selected: &Option<String>,
    mut pick: impl FnMut(Option<String>),
) {
    let shown = selected.clone().unwrap_or_else(|| "System default".into());
    ComboBox::from_id_salt(id).width(300.0).selected_text(shown).show_ui(ui, |ui| {
        if ui.selectable_label(selected.is_none(), "System default").clicked() {
            pick(None);
        }
        for n in names {
            if ui.selectable_label(selected.as_deref() == Some(n), n).clicked() {
                pick(Some(n.clone()));
            }
        }
    });
}

/// A dropdown over an enum's `ALL`, using its `label()`.
/// UI / display preferences: frame rate, waterfall scroll speed, spectrum speed.
fn settings_ui_tab(ui: &mut egui::Ui, cfg: &mut sdroxide_types::UiSettings) {
    use sdroxide_types::{Speed, UiSettings};
    ui.label(RichText::new("Display").size(14.0).strong().color(crate::theme::CYAN));
    ui.add_space(6.0);
    egui::Grid::new("ui-grid").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
        ui.label("Screen update rate");
        ComboBox::from_id_salt("ui-fps")
            .selected_text(format!("{} fps", cfg.frame_rate_fps))
            .show_ui(ui, |ui| {
                for f in UiSettings::FPS_OPTIONS {
                    ui.selectable_value(&mut cfg.frame_rate_fps, f, format!("{f} fps"));
                }
            });
        ui.end_row();

        ui.label("Waterfall scroll speed");
        enum_combo(ui, "ui-wf", &mut cfg.waterfall_speed, &Speed::ALL, Speed::label);
        ui.end_row();

        ui.label("Spectrum update speed");
        enum_combo(ui, "ui-spec", &mut cfg.spectrum_speed, &Speed::ALL, Speed::label);
        ui.end_row();
    });
    ui.add_space(8.0);
    ui.label(
        RichText::new(
            "Higher frame rates look smoother but cost more CPU/GPU. Spectrum speed \
             sets how quickly the trace reacts (slower = smoother/more averaged).",
        )
        .weak(),
    );
}

fn enum_combo<T: PartialEq + Copy>(
    ui: &mut egui::Ui,
    id: &str,
    cur: &mut T,
    all: &[T],
    label: impl Fn(T) -> &'static str,
) {
    ComboBox::from_id_salt(id).selected_text(label(*cur)).show_ui(ui, |ui| {
        for &opt in all {
            if ui.selectable_label(*cur == opt, label(opt)).clicked() {
                *cur = opt;
            }
        }
    });
}

/// CAT / Audio interface: serial + PTT parameters (the interface itself is
/// chosen by the selector in `settings_body`).
fn settings_cat_tab(
    ui: &mut egui::Ui,
    serial_ports: &[String],
    radio_edit: &mut Option<sdroxide_types::RadioConfig>,
) {
    use sdroxide_types::{CatFamily, DigiMode, LineState, ModeControl, Parity, PttMethod, SoundFormat, StopBits};
    let Some(cfg) = radio_edit.as_mut() else {
        ui.label("Radio configuration is only available in the native app.");
        return;
    };
    egui::Grid::new("cat-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Sound format");
        enum_combo(ui, "sfmt", &mut cfg.cat.format, &SoundFormat::ALL, SoundFormat::label);
        ui.end_row();

        if matches!(cfg.cat.format, SoundFormat::DemodAudio) {
            ui.label("Panadapter BW");
            ui.add(DragValue::new(&mut cfg.cat.audio_bw_hz).speed(100.0).range(1000.0..=24000.0).suffix(" Hz"));
            ui.end_row();
        }

        ui.label("Serial port");
        let shown = if cfg.cat.serial.path.is_empty() { "— select —".to_string() } else { cfg.cat.serial.path.clone() };
        ComboBox::from_id_salt("serport").width(260.0).selected_text(shown).show_ui(ui, |ui| {
            for p in serial_ports {
                if ui.selectable_label(&cfg.cat.serial.path == p, p).clicked() {
                    cfg.cat.serial.path = p.clone();
                }
            }
        });
        ui.end_row();

        ui.label("CAT family");
        enum_combo(ui, "fam", &mut cfg.cat.family, &CatFamily::ALL, CatFamily::label);
        ui.end_row();

        ui.label("Baud");
        ComboBox::from_id_salt("baud").selected_text(cfg.cat.serial.baud.to_string()).show_ui(ui, |ui| {
            for b in [4800u32, 9600, 19200, 38400, 57600, 115200] {
                if ui.selectable_label(cfg.cat.serial.baud == b, b.to_string()).clicked() {
                    cfg.cat.serial.baud = b;
                }
            }
        });
        ui.end_row();

        ui.label("Data bits");
        ComboBox::from_id_salt("databits").selected_text(cfg.cat.serial.data_bits.to_string()).show_ui(ui, |ui| {
            for d in [7u8, 8] {
                if ui.selectable_label(cfg.cat.serial.data_bits == d, d.to_string()).clicked() {
                    cfg.cat.serial.data_bits = d;
                }
            }
        });
        ui.end_row();

        ui.label("Parity");
        enum_combo(ui, "parity", &mut cfg.cat.serial.parity, &Parity::ALL, Parity::label);
        ui.end_row();

        ui.label("Stop bits");
        enum_combo(ui, "stop", &mut cfg.cat.serial.stop_bits, &StopBits::ALL, StopBits::label);
        ui.end_row();

        ui.label("Force RTS");
        enum_combo(ui, "rts", &mut cfg.cat.serial.force_rts, &LineState::ALL, LineState::label);
        ui.end_row();
        ui.label("Force DTR");
        enum_combo(ui, "dtr", &mut cfg.cat.serial.force_dtr, &LineState::ALL, LineState::label);
        ui.end_row();

        ui.label("PTT method");
        enum_combo(ui, "ptt", &mut cfg.cat.ptt, &PttMethod::ALL, PttMethod::label);
        ui.end_row();

        ui.label("Mode control");
        enum_combo(ui, "modectl", &mut cfg.cat.mode_control, &ModeControl::ALL, ModeControl::label);
        ui.end_row();

        ui.label("Digimode mode");
        enum_combo(ui, "digimode", &mut cfg.cat.digi_mode, &DigiMode::ALL, DigiMode::label);
        ui.end_row();

        ui.label("Poll rate");
        ui.add(DragValue::new(&mut cfg.cat.poll_hz).speed(0.5).range(0.5..=20.0).suffix(" Hz"));
        ui.end_row();

        if matches!(cfg.cat.family, CatFamily::Icom | CatFamily::Xiegu) {
            ui.label("Radio ID (hex)");
            let mut hex = format!("{:02X}", cfg.cat.icom_radio_id);
            let resp = ui.add(egui::TextEdit::singleline(&mut hex).desired_width(48.0));
            if resp.changed() {
                if let Ok(v) = u8::from_str_radix(hex.trim().trim_start_matches("0x"), 16) {
                    cfg.cat.icom_radio_id = v;
                }
            }
            ui.end_row();
        }
    });
    ui.add_space(6.0);
    ui.label(RichText::new("Serial / audio changes take effect on restart.").weak());
}

/// HPSDR interface: network device discovery / manual IP / sample rate (the
/// interface itself is chosen by the selector in `settings_body`).
fn settings_hpsdr_tab(
    ui: &mut egui::Ui,
    devices: &[sdroxide_types::HpsdrDevice],
    radio_edit: &mut Option<sdroxide_types::RadioConfig>,
    discover: &mut bool,
) {
    use sdroxide_types::HpsdrConfig;
    let Some(cfg) = radio_edit.as_mut() else {
        ui.label("Radio configuration is only available in the native app.");
        return;
    };
    egui::Grid::new("hpsdr-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Devices");
        ui.horizontal(|ui| {
            if ui.button("Discover").clicked() {
                *discover = true;
            }
            let shown = cfg.hpsdr.selected_ip.clone().unwrap_or_else(|| "— none —".into());
            ComboBox::from_id_salt("hpsdr_dev").width(320.0).selected_text(shown).show_ui(ui, |ui| {
                if devices.is_empty() {
                    ui.label(RichText::new("no devices — press Discover").weak());
                }
                for d in devices {
                    // Only Protocol 2 devices are selectable; P1 (e.g. HL2) is shown but greyed.
                    if d.supported() {
                        let sel = cfg.hpsdr.selected_ip.as_deref() == Some(d.ip.as_str());
                        if ui.selectable_label(sel, d.label()).clicked() {
                            cfg.hpsdr.selected_ip = Some(d.ip.clone());
                        }
                    } else {
                        ui.label(RichText::new(d.label()).weak());
                    }
                }
            });
        });
        ui.end_row();

        ui.label("Manual IP");
        let mut ip = cfg.hpsdr.manual_ip.clone().unwrap_or_default();
        let resp = ui.add(
            egui::TextEdit::singleline(&mut ip)
                .desired_width(160.0)
                .hint_text("optional, e.g. 192.168.1.50"),
        );
        if resp.changed() {
            let t = ip.trim();
            cfg.hpsdr.manual_ip = if t.is_empty() { None } else { Some(t.to_string()) };
        }
        ui.end_row();

        ui.label("Sample rate");
        // Show only rates valid for the selected device's protocol (P1 ≤ 384 kHz).
        let proto = devices
            .iter()
            .find(|d| Some(d.ip.as_str()) == cfg.hpsdr.selected_ip.as_deref())
            .map(|d| d.protocol)
            .unwrap_or(2);
        let shown = format!("{} kHz", (cfg.hpsdr.sample_rate_hz / 1000.0) as u32);
        ComboBox::from_id_salt("hpsdr_rate").selected_text(shown).show_ui(ui, |ui| {
            for &r in HpsdrConfig::rates_for(proto) {
                let sel = (cfg.hpsdr.sample_rate_hz - r).abs() < 1.0;
                if ui.selectable_label(sel, format!("{} kHz", (r / 1000.0) as u32)).clicked() {
                    cfg.hpsdr.sample_rate_hz = r;
                }
            }
        });
        ui.end_row();
    });
    ui.add_space(6.0);
    ui.label(
        RichText::new(
            "A manual IP overrides discovery. Backend / device / sample-rate changes take effect on restart.",
        )
        .weak(),
    );
}

/// TCI interface: WebSocket server address, IQ sample rate, and a
/// Test-connection button (the interface is chosen by the selector in
/// `settings_body`).
fn settings_tci_tab(
    ui: &mut egui::Ui,
    radio_edit: &mut Option<sdroxide_types::RadioConfig>,
    tci_test: &mut bool,
    test_result: &Option<Result<String, String>>,
) {
    use sdroxide_types::TciConfig;
    let Some(cfg) = radio_edit.as_mut() else {
        ui.label("Radio configuration is only available in the native app.");
        return;
    };
    egui::Grid::new("tci-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Server address");
        ui.add(
            egui::TextEdit::singleline(&mut cfg.tci.address)
                .desired_width(220.0)
                .hint_text("host:port, e.g. 127.0.0.1:50001"),
        );
        ui.end_row();

        ui.label("IQ sample rate");
        let shown = format!("{} kHz", (cfg.tci.iq_sample_rate_hz / 1000.0) as u32);
        ComboBox::from_id_salt("tci_rate").selected_text(shown).show_ui(ui, |ui| {
            for &r in &TciConfig::IQ_RATES {
                let sel = (cfg.tci.iq_sample_rate_hz - r).abs() < 1.0;
                if ui.selectable_label(sel, format!("{} kHz", (r / 1000.0) as u32)).clicked() {
                    cfg.tci.iq_sample_rate_hz = r;
                }
            }
        });
        ui.end_row();

        ui.label("");
        if ui.button("Test connection").clicked() {
            *tci_test = true;
        }
        ui.end_row();
    });
    match test_result {
        Some(Ok(s)) => {
            ui.label(RichText::new(format!("Connected: {s}")).color(Color32::from_rgb(90, 200, 110)));
        }
        Some(Err(e)) => {
            ui.label(RichText::new(format!("Failed: {e}")).color(Color32::from_rgb(230, 90, 80)));
        }
        None => {}
    }
    ui.add_space(6.0);
    ui.label(
        RichText::new(
            "Wideband IQ receive, audio transmit. Address / rate changes take effect on restart.",
        )
        .weak(),
    );
}

impl eframe::App for SdroxideApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let now = ctx.input(|i| i.time);
        while let Some(ev) = self.ctrl.poll_event() {
            match ev {
                RadioEvent::Capabilities(c) => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
                        "sdroxide — {}",
                        c.label
                    )));
                    self.caps = Some(c);
                }
                RadioEvent::State(s) => {
                    let prev_vfo = self.state.active_freq_hz();
                    self.state = s;
                    self.recenter_if_tuned_away(prev_vfo);
                }
                RadioEvent::Spectrum(f) => {
                    self.frame = Some(std::sync::Arc::new(f));
                    self.last_spectrum_at = now;
                }
                RadioEvent::Meters(m) => self.meters = Some(m),
                RadioEvent::Memories(m) => self.memories = m,
                RadioEvent::ConnectionLost(e) => self.error = Some(e),
                RadioEvent::Notice(n) => self.radio_notice = n,
                RadioEvent::Ft8Decodes(d) => {
                    // Prepend newest-slot decodes; keep a rolling window.
                    for dec in d.into_iter().rev() {
                        self.digi_decodes.insert(0, dec);
                    }
                    self.digi_decodes.truncate(200);
                }
                RadioEvent::Ft8Status(s) => {
                    // Seed the editable config from the engine's persisted
                    // value once (later edits are UI-owned so typing sticks).
                    if !self.digi_cfg_seeded {
                        self.digi_cfg_edit = s.config.clone();
                        self.digi_cfg_seeded = true;
                    }
                    self.digi_status = Some(s);
                }
                RadioEvent::Ft8QsoLogged(mut r) => {
                    r.id = self.next_log_id();
                    self.qso_log.push(r);
                    persist_qso_log(&self.qso_log);
                }
                RadioEvent::SkimmerSpots(s) => {
                    // The engine sends the full current set each update; the
                    // stable `id` per spot lets the overlay keep each box (and
                    // its scroll) in place across updates.
                    for spot in &s {
                        // Remember when each spot last keyed, and seed newly
                        // seen ones to now, so alpha starts solid and fades.
                        let e = self.skimmer_active_at.entry(spot.id).or_insert(now);
                        if spot.active {
                            *e = now;
                        }
                    }
                    // Forget timings for spots the engine has dropped.
                    let live: std::collections::HashSet<u64> = s.iter().map(|x| x.id).collect();
                    self.skimmer_active_at.retain(|id, _| live.contains(id));
                    self.skimmer_spots = s;
                }
            }
        }
        // When the skimmer is off the engine stops emitting; drop stale boxes.
        if !self.state.skimmer_enabled && !self.skimmer_spots.is_empty() {
            self.skimmer_spots.clear();
        }

        let mut cmds = Vec::new();
        self.keyboard_shortcuts(&ctx, &mut cmds);

        egui::Panel::top(egui::Id::new("topbar"))
            .frame(
                egui::Frame::new()
                    .fill(crate::theme::BG_DEEP)
                    .inner_margin(egui::Margin::symmetric(8, 6)),
            )
            .show(ui, |ui| {
                crate::chrome::angled_frame(ui, crate::theme::PINK, |ui| {
                    self.top_bar(ui, &mut cmds);
                });
            });
        // A persistent radio-audio warning (input unavailable / mono-for-IQ)
        // rides above the panadapter with a dismiss button, so a silent RX
        // failure is explained rather than reading as "waiting for spectrum".
        if let Some(notice) = self.radio_notice.clone() {
            egui::Frame::new()
                .fill(Color32::from_rgb(60, 45, 10))
                .stroke(egui::Stroke::new(1.0, Color32::from_rgb(210, 160, 40)))
                .inner_margin(egui::Margin::symmetric(8, 5))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new("⚠")
                                .size(15.0)
                                .color(Color32::from_rgb(255, 190, 70)),
                        );
                        ui.label(
                            RichText::new(notice).size(13.0).color(Color32::from_rgb(240, 220, 180)),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("Dismiss").clicked() {
                                self.radio_notice = None;
                            }
                        });
                    });
                });
        }
        // Remaining space: the panadapter (+ FT8/FT4 operating panel).
        if let Some(err) = self.error.clone() {
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new(err).size(18.0).color(Color32::RED));
            });
        } else if self.state.rx[0].mode.is_digital() {
            // Remember the voice-mode view once, so leaving FT8 can restore it
            // instead of leaving the panadapter zoomed to the sub-band.
            if self.pre_digi_view.is_none() {
                self.pre_digi_view = Some((self.view.view_lo_hz, self.view.view_hi_hz));
            }
            // Lock the view to the FT8 sub-band (audio 0..3.5 kHz above dial).
            let dial = self.state.rx_freq_hz();
            self.view.view_lo_hz = dial - 200.0;
            self.view.view_hi_hz = dial + 3500.0;
            let audio_hz = self.digi_status.as_ref().map(|s| s.audio_hz).unwrap_or(1500.0);
            let mode = self.state.rx[0].mode;
            let is_text = mode.is_text_modem();
            // RTTY shows mark/space tuning lines; PSK just the centre marker.
            let markers: Vec<f32> = if mode == Mode::Rtty {
                let sh = self.digi_status.as_ref().map(|s| s.config.rtty_shift_hz).unwrap_or(170.0);
                vec![audio_hz - sh / 2.0, audio_hz + sh / 2.0]
            } else {
                Vec::new()
            };
            // FT8 station callsign boxes (built before the &mut self borrows).
            let (ft8_spots, ft8_alpha) =
                if is_text { (Vec::new(), Vec::new()) } else { self.ft8_overlay() };

            let frame = self.frame.take();
            // Manual vertical split with a draggable divider: the operating
            // panel gets `digi_panel_fraction` of the height, the waterfall the
            // rest. A thin handle between them resizes the split.
            let total = ui.available_height();
            let width = ui.available_width();
            let handle_h = 7.0;
            let panel_h =
                (total * self.view.digi_panel_fraction).clamp(160.0, (total - 140.0).max(160.0));
            let wf_h = (total - panel_h - handle_h).max(80.0);

            let wf_tuning = self.wf_tick(frame.is_some());
            ui.allocate_ui(egui::vec2(width, wf_h), |ui| {
                spectrum_view::show_ext(
                    ui,
                    &mut self.view,
                    &mut self.state,
                    frame.as_ref(),
                    &mut self.peaks,
                    &mut self.spec_smooth,
                    &mut self.trace_cache,
                    Some(audio_hz),
                    &markers,
                    &ft8_spots,
                    &ft8_alpha,
                    wf_tuning,
                    &mut cmds,
                );
            });
            // Resize handle between the waterfall and the FT8/FT4 panel.
            let (hrect, hresp) =
                ui.allocate_exact_size(egui::vec2(width, handle_h), egui::Sense::click_and_drag());
            if hresp.hovered() || hresp.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
            }
            if hresp.dragged() {
                // Drag down shrinks the panel (waterfall grows), drag up grows it.
                let d = hresp.drag_delta().y / total;
                self.view.digi_panel_fraction =
                    (self.view.digi_panel_fraction - d).clamp(0.2, 0.82);
            }
            {
                let p = ui.painter_at(hrect);
                let hot = hresp.hovered() || hresp.dragged();
                p.rect_filled(hrect, 0.0, crate::theme::PANEL);
                let col = if hot { crate::theme::CYAN } else { Color32::from_gray(70) };
                let cx = hrect.center().x;
                let cy = hrect.center().y;
                for dx in [-16.0f32, 0.0, 16.0] {
                    p.line_segment(
                        [egui::pos2(cx + dx - 6.0, cy), egui::pos2(cx + dx + 6.0, cy)],
                        egui::Stroke::new(2.0, col),
                    );
                }
            }
            ui.allocate_ui(egui::vec2(width, panel_h), |ui| {
                egui::Frame::new()
                    .fill(crate::theme::BG_DEEP)
                    .inner_margin(egui::Margin { left: 0, right: 0, top: 6, bottom: 0 })
                    .show(ui, |ui| {
                        crate::chrome::angled_frame(ui, crate::theme::PINK, |ui| {
                            if is_text {
                                self.text_modem_panel(ui, &mut cmds);
                            } else {
                                self.digi_panel(ui, &mut cmds);
                            }
                        });
                    });
            });
            self.frame = frame;
        } else {
            // Restore the pre-FT8 view span once, on the first voice frame
            // after leaving a digital mode.
            if let Some((lo, hi)) = self.pre_digi_view.take() {
                self.view.view_lo_hz = lo;
                self.view.view_hi_hz = hi;
            }
            let (cw_spots, cw_alpha) = self.cw_overlay(now);
            let frame = self.frame.take();
            let wf_tuning = self.wf_tick(frame.is_some());
            spectrum_view::show(
                ui,
                &mut self.view,
                &mut self.state,
                frame.as_ref(),
                &mut self.peaks,
                &mut self.spec_smooth,
                &mut self.trace_cache,
                &cw_spots,
                &cw_alpha,
                wf_tuning,
                &mut cmds,
            );
            self.frame = frame;
        }

        self.memories_window(&ctx, &mut cmds);
        self.settings_window(&ctx, &mut cmds);
        self.digi_settings_window(&ctx, &mut cmds);
        self.logbook_window(&ctx);

        // Debounced spectrum-config updates with pan hysteresis.
        let now = ctx.input(|i| i.time);
        if !self.cfg_still_good() {
            let ideal = self.desired_spectrum_cfg();
            if self.desired_cfg != Some(ideal) {
                self.desired_cfg = Some(ideal);
                self.desired_at = now;
            }
            if self.sent_cfg.is_none() || now - self.desired_at >= CFG_DEBOUNCE_S {
                self.sent_cfg = Some(ideal);
                cmds.push(Command::SetSpectrumCfg(ideal));
            }
        }

        for c in cmds {
            self.ctrl.send(c);
        }

        // Data-driven repaint: redraw immediately when data is already waiting
        // (arrived while this frame was being built — checked after the drain,
        // so this can't busy-loop), otherwise wake at the next expected
        // spectrum frame, or idle-poll when nothing is streaming. User input
        // wakes eframe by itself, so interactivity is unaffected.
        if self.ctrl.wants_repaint_soon() {
            ctx.request_repaint();
        } else {
            let fps = self
                .sent_cfg
                .or(self.desired_cfg)
                .map(|c| c.fps)
                .unwrap_or(SpectrumConfig::default().fps)
                .max(1) as u64;
            let streaming = self.frame.is_some()
                && self.error.is_none()
                && now - self.last_spectrum_at < STREAM_STALE_S;
            // Floor division keeps the poll period <= the stream period, so no
            // frame is ever skipped (the spectrum buffer is latest-wins).
            let wait_ms = if streaming { 1000 / fps } else { IDLE_POLL_MS };
            ctx.request_repaint_after(Duration::from_millis(wait_ms));
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "view", &self.view);
        // On wasm this is the logbook's persistence; on native it's a harmless
        // backup (the authoritative copy is written to the config dir on change).
        eframe::set_value(storage, "qso_log", &self.qso_log);
        // Same split: authoritative on native is config.toml (written on change).
        eframe::set_value(storage, "ui_settings", &self.ui_settings);
    }
}

// ── Logbook persistence (native: config-dir JSON; wasm: eframe storage) ──────
#[cfg(not(target_arch = "wasm32"))]
fn load_qso_log(_storage: Option<&dyn eframe::Storage>) -> Vec<QsoRecord> {
    sdroxide_config::load_qso_log()
}
#[cfg(target_arch = "wasm32")]
fn load_qso_log(storage: Option<&dyn eframe::Storage>) -> Vec<QsoRecord> {
    storage.and_then(|s| eframe::get_value(s, "qso_log")).unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
fn persist_qso_log(log: &[QsoRecord]) {
    if let Err(e) = sdroxide_config::save_qso_log(log) {
        eprintln!("failed to save logbook: {e}");
    }
}
#[cfg(target_arch = "wasm32")]
fn persist_qso_log(_log: &[QsoRecord]) {
    // Written by eframe's periodic `save()` into localStorage.
}

// ── UI/display preferences (native: config.toml [ui]; wasm: eframe storage) ──
#[cfg(not(target_arch = "wasm32"))]
fn load_ui_settings(_storage: Option<&dyn eframe::Storage>) -> sdroxide_types::UiSettings {
    sdroxide_config::load_ui_settings()
}
#[cfg(target_arch = "wasm32")]
fn load_ui_settings(storage: Option<&dyn eframe::Storage>) -> sdroxide_types::UiSettings {
    storage.and_then(|s| eframe::get_value(s, "ui_settings")).unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
fn persist_ui_settings(ui: &sdroxide_types::UiSettings) {
    if let Err(e) = sdroxide_config::save_ui_settings(ui) {
        eprintln!("failed to save UI settings: {e}");
    }
}
#[cfg(target_arch = "wasm32")]
fn persist_ui_settings(_ui: &sdroxide_types::UiSettings) {
    // Written by eframe's periodic `save()` into localStorage.
}

/// Current Unix time (UTC seconds). `SystemTime` panics on wasm, so use JS.
fn now_unix() -> i64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }
    #[cfg(target_arch = "wasm32")]
    {
        (js_sys::Date::now() / 1000.0) as i64
    }
}

/// Current Unix time as fractional UTC seconds (for waterfall time gridlines).
fn now_unix_f64() -> f64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
    }
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() / 1000.0
    }
}

/// Parse `"YYYY-MM-DD"` + `"HH:MM"` (UTC) to a Unix timestamp, falling back to
/// `fallback` if the fields don't fully parse.
fn parse_utc(date: &str, time: &str, fallback: i64) -> i64 {
    let d: Vec<&str> = date.trim().split('-').collect();
    let t: Vec<&str> = time.trim().split(':').collect();
    if d.len() == 3 && t.len() >= 2 {
        if let (Ok(y), Ok(mo), Ok(day), Ok(h), Ok(mi)) =
            (d[0].parse(), d[1].parse(), d[2].parse(), t[0].parse(), t[1].parse())
        {
            return sdroxide_types::ymd_hms_to_unix(y, mo, day, h, mi, 0);
        }
    }
    fallback
}

/// `"YYYY-MM-DD"` for a Unix timestamp (UTC).
fn date_str(unix: i64) -> String {
    let (y, mo, d, ..) = sdroxide_types::utc_ymd_hms(unix);
    format!("{y:04}-{mo:02}-{d:02}")
}

/// `"HH:MM"` for a Unix timestamp (UTC).
fn time_str(unix: i64) -> String {
    let (_, _, _, h, mi, _) = sdroxide_types::utc_ymd_hms(unix);
    format!("{h:02}:{mi:02}")
}

/// Standard FT8/FT4 dial frequencies per HF/6 m band.
/// The standard FT8/FT4 dial frequency for `band`, if one exists for `mode`
/// (matched by which band's edges the frequency falls within).
fn digi_freq_for_band(mode: Mode, band: Band) -> Option<f64> {
    let (lo, hi) = band.edges()?;
    digi_dial_freqs(mode).iter().find(|&&(_, hz)| (lo..=hi).contains(&hz)).map(|&(_, hz)| hz)
}

fn digi_dial_freqs(mode: Mode) -> &'static [(&'static str, f64)] {
    match mode {
        // PSK31 activity centres (USB dial; signals sit ~1 kHz above).
        Mode::Psk => &[
            ("80m", 3_580_000.0),
            ("40m", 7_040_000.0),
            ("30m", 10_142_000.0),
            ("20m", 14_070_000.0),
            ("17m", 18_097_000.0),
            ("15m", 21_070_000.0),
            ("12m", 24_920_000.0),
            ("10m", 28_120_000.0),
        ],
        // RTTY sub-band starts (USB dial).
        Mode::Rtty => &[
            ("80m", 3_580_000.0),
            ("40m", 7_040_000.0),
            ("30m", 10_140_000.0),
            ("20m", 14_080_000.0),
            ("17m", 18_100_000.0),
            ("15m", 21_080_000.0),
            ("12m", 24_920_000.0),
            ("10m", 28_080_000.0),
        ],
        Mode::Ft4 => &[
            ("80m", 3_575_000.0),
            ("40m", 7_047_500.0),
            ("30m", 10_140_000.0),
            ("20m", 14_080_000.0),
            ("17m", 18_104_000.0),
            ("15m", 21_140_000.0),
            ("12m", 24_919_000.0),
            ("10m", 28_180_000.0),
        ],
        // FT8 (and default).
        _ => &[
            ("160m", 1_840_000.0),
            ("80m", 3_573_000.0),
            ("40m", 7_074_000.0),
            ("30m", 10_136_000.0),
            ("20m", 14_074_000.0),
            ("17m", 18_100_000.0),
            ("15m", 21_074_000.0),
            ("12m", 24_915_000.0),
            ("10m", 28_074_000.0),
            ("6m", 50_313_000.0),
        ],
    }
}

/// Pick `(floor, ceil)` dB for best waterfall contrast from a frame's u8
/// `bins` (mapped over `[db_floor, db_ceil]`). Percentile-based so a single
/// strong carrier doesn't over-blow the scale and weak signals stay visible.
/// Returns `None` for an empty or degenerate frame.
fn pick_levels(bins: &[u8], db_floor: f32, db_ceil: f32) -> Option<(f32, f32)> {
    let range = db_ceil - db_floor;
    if bins.is_empty() || range <= 0.0 {
        return None;
    }
    // Reconstruct approximate dB per bin from the u8 mapping and sort.
    let mut db: Vec<f32> = bins.iter().map(|&b| db_floor + (b as f32 / 255.0) * range).collect();
    db.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let pct = |p: f32| -> f32 {
        let i = ((p * (db.len() - 1) as f32).round() as usize).min(db.len() - 1);
        db[i]
    };
    let noise = pct(0.25); // typical noise floor
    let peak = pct(0.99); // strong signals, ignoring the hottest outliers
    let mut floor = noise - 5.0; // noise sits just above the floor (dark)
    let mut ceil = peak + 6.0; // headroom so strong signals don't clip
    // Keep a usable dynamic range even on an empty/flat band.
    let min_range = 24.0;
    if ceil - floor < min_range {
        let mid = 0.5 * (ceil + floor);
        floor = mid - 0.5 * min_range;
        ceil = mid + 0.5 * min_range;
    }
    // Clamp to the same bounds as the manual controls.
    let floor = floor.clamp(-160.0, -40.0);
    let mut ceil = ceil.clamp(-100.0, 20.0);
    if ceil - floor < 10.0 {
        ceil = (floor + 10.0).min(20.0);
    }
    Some((floor, ceil))
}

/// Colour a decode's SNR: green for strong, cyan mid, dimmed for weak.
fn snr_color(snr_db: i16) -> Color32 {
    if snr_db >= 0 {
        crate::theme::GREEN
    } else if snr_db >= -12 {
        crate::theme::CYAN
    } else {
        crate::theme::CYAN_DIM
    }
}

#[cfg(test)]
mod tests {
    use super::pick_levels;

    /// Map a dB value to the u8 code used by a frame spanning `[lo, hi]`.
    fn code(db: f32, lo: f32, hi: f32) -> u8 {
        (((db - lo) / (hi - lo) * 255.0).clamp(0.0, 255.0)) as u8
    }

    #[test]
    fn levels_bracket_noise_and_signals() {
        // Frame mapped over a wide [-120, -20]: mostly noise near -110 with a
        // handful of strong signals near -45.
        let (lo, hi) = (-120.0f32, -20.0f32);
        let mut bins = vec![code(-110.0, lo, hi); 1000];
        bins.extend(std::iter::repeat(code(-45.0, lo, hi)).take(20));
        let (floor, ceil) = pick_levels(&bins, lo, hi).unwrap();
        // Floor just below the noise; ceiling just above the signals.
        assert!((-120.0..-100.0).contains(&floor), "floor {floor}");
        assert!((-55.0..-30.0).contains(&ceil), "ceil {ceil}");
        assert!(ceil - floor >= 24.0, "range {}", ceil - floor);
    }

    #[test]
    fn flat_band_keeps_minimum_range() {
        // A noise-only band still gets a usable contrast window, not a sliver.
        let (lo, hi) = (-120.0f32, -20.0f32);
        let bins = vec![code(-108.0, lo, hi); 512];
        let (floor, ceil) = pick_levels(&bins, lo, hi).unwrap();
        assert!(ceil - floor >= 24.0, "range {}", ceil - floor);
        assert!(floor >= -160.0 && ceil <= 20.0);
    }

    #[test]
    fn empty_frame_returns_none() {
        assert!(pick_levels(&[], -120.0, -20.0).is_none());
        assert!(pick_levels(&[10, 20], -50.0, -50.0).is_none());
    }
}
