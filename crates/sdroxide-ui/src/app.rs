use std::sync::{Arc, Mutex};
use std::time::Duration;

use eframe::egui::{self, Color32, ComboBox, DragValue, RichText, Slider};
use sdroxide_types::{
    AgcMode, AudioDevices, Band, Command, Decode, DeviceCaps, DigiStatus, Direction,
    MemoryChannel, Meters, Mode, QsoRecord, RadioController, RadioEvent, RadioState, RxId,
    SkimmerKind, SkimmerSpot, SpectrumConfig, SpectrumFrame, SstvMode, SstvStatus, Vfo,
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
    General,
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
    /// SSTV image-mode panel state (gallery, TX slots, message, textures).
    sstv: SstvUi,
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
            sstv: SstvUi::default(),
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
        let s = &self.ui_settings;
        let gradient = s.spectrum_gradient.then(|| {
            let [tr, tg, tb] = s.gradient_top;
            let [br, bg, bb] = s.gradient_bottom;
            (Color32::from_rgb(tr, tg, tb), Color32::from_rgb(br, bg, bb))
        });
        spectrum_view::WfTuning {
            rows_to_write,
            rows_per_sec,
            now_unix: now,
            spectrum_alpha,
            palette: s.waterfall_palette,
            gradient,
        }
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
                self.vfo_rit_module(ui, cmds);
                self.rx_filter_module(ui, cmds);
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
        // The 10-digit readout is fixed width, so measure it (via the same fonts
        // freq_display uses) and size the box to hug its contents — that keeps the
        // right column against the box edge (no empty space) and lets the readout
        // be centred vertically by exact geometry rather than a fragile layout hint.
        let font40 = egui::FontId::monospace(40.0);
        let digit = ui
            .painter()
            .layout_no_wrap("0".to_owned(), font40.clone(), Color32::WHITE)
            .size();
        let dot_w = ui
            .painter()
            .layout_no_wrap(".".to_owned(), font40, Color32::WHITE)
            .size()
            .x;
        let hz_w = ui
            .painter()
            .layout_no_wrap(" Hz".to_owned(), egui::FontId::proportional(12.0), Color32::WHITE)
            .size()
            .x;
        // 10 digits + 3 group separators + " Hz", with freq_display's 1px spacing.
        let readout_w = 10.0 * digit.x + 3.0 * dot_w + hz_w + 13.0;
        let readout_h = digit.y;

        let ab_w = 68.0;
        let right_w = 96.0;
        let box_w = 8.0 + ab_w + 10.0 + readout_w + 12.0 + right_w + 8.0;

        crate::chrome::module_bare_h(ui, box_w, crate::chrome::MODULE_TALL_H, |ui| {
            ui.spacing_mut().item_spacing.x = 0.0; // control every gap explicitly
            let active = self.state.active_vfo;
            let full_h = ui.available_height();

            // VFO A/B selector, vertically centred in the full box height.
            let mut sel = None;
            ui.allocate_ui_with_layout(
                egui::vec2(ab_w, full_h),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    for (v, label) in [(Vfo::A, "A"), (Vfo::B, "B")] {
                        if crate::chrome::chip(ui, active == v, RichText::new(label).size(15.0))
                            .clicked()
                        {
                            sel = Some(v);
                        }
                    }
                },
            );
            if let Some(v) = sel {
                cmds.push(Command::SelectVfo(v));
            }
            ui.add_space(10.0);

            // Big frequency readout, centred vertically by measured height.
            let mut new_hz = None;
            ui.allocate_ui_with_layout(
                egui::vec2(readout_w, full_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.add_space(((full_h - readout_h) / 2.0).max(0.0));
                    new_hz = freq_display::show(
                        ui,
                        egui::Id::new("main-freq"),
                        self.state.active_freq_hz(),
                    );
                },
            );
            if let Some(hz) = new_hz {
                cmds.push(Command::SetVfo { vfo: active, hz });
            }
            ui.add_space(12.0);

            // Right column: inactive VFO frequency anchored top-right, band/mode
            // selector anchored bottom-right, hard against the box edge.
            let inactive_hz = match active {
                Vfo::A => self.state.vfo_b_hz,
                Vfo::B => self.state.vfo_a_hz,
            };
            ui.allocate_ui_with_layout(
                egui::vec2(right_w, full_h),
                egui::Layout::top_down(egui::Align::Max),
                |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.label(
                        RichText::new(format!("{:.6} MHz", inactive_hz / 1e6))
                            .monospace()
                            .size(12.0)
                            .color(Color32::from_gray(120)),
                    );
                    let pad = (ui.available_height() - 24.0).max(0.0);
                    ui.add_space(pad);
                    self.band_mode_button(ui, cmds);
                },
            );
        });
    }

    /// The S-meter in a label-less box, always pinned top-right. Clicking it
    /// toggles between the bar and analog-needle styles.
    fn smeter_module(&mut self, ui: &mut egui::Ui) {
        crate::chrome::module_bare_h(ui, 250.0, crate::chrome::MODULE_TALL_H, |ui| {
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

    /// Combined VFO + RIT/XIT box: the VFO A/B utility chips on top, with the
    /// RIT/XIT tuning-offset controls stacked underneath. Bare and tall — this
    /// replaces the separate VFO and RIT/XIT boxes.
    fn vfo_rit_module(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let tx_capable = self.caps.as_ref().is_some_and(|c| c.is_transmit_capable());
        // Fixed field width, wide enough for a signed 4-digit offset plus " Hz".
        let hz_field = egui::vec2(74.0, 22.0);
        crate::chrome::module_bare_h(ui, 270.0, crate::chrome::MODULE_TALL_H, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(5.0, 5.0);
                // VFO utility chips.
                ui.horizontal(|ui| {
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
                // RIT / XIT tuning offsets.
                ui.horizontal(|ui| {
                    let rit = self.state.rit;
                    if crate::chrome::chip(ui, rit.enabled, "RIT").clicked() {
                        cmds.push(Command::SetRit { enabled: !rit.enabled, hz: rit.hz });
                    }
                    let mut rit_hz = rit.hz;
                    if ui
                        .add_sized(
                            hz_field,
                            DragValue::new(&mut rit_hz).speed(5).range(-9999..=9999).suffix(" Hz"),
                        )
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
                            .add_sized(
                                hz_field,
                                DragValue::new(&mut xit_hz)
                                    .speed(5)
                                    .range(-9999..=9999)
                                    .suffix(" Hz"),
                            )
                            .changed()
                        {
                            cmds.push(Command::SetXit { enabled: xit.enabled, hz: xit_hz });
                        }
                    }
                });
            });
        });
    }

    /// The band/mode selector button plus the floating popup with the band +
    /// mode + digital button rows.
    fn band_mode_button(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
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
    }

    /// Combined Receiver + Filter/Noise box: AGC / volume / mute on top, with the
    /// squelch + noise-blanker + noise-reduction controls stacked underneath.
    /// Bare and tall, like the VFO/RIT box — replaces the separate Receiver and
    /// Filter boxes.
    fn rx_filter_module(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        crate::chrome::module_bare_h(ui, 328.0, crate::chrome::MODULE_TALL_H, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(5.0, 5.0);
                // Receiver: volume, AGC, mute.
                ui.horizontal(|ui| {
                    let mut vol = self.state.rx[0].volume;
                    ui.label("Vol");
                    if crate::chrome::slider(ui, Slider::new(&mut vol, 0.0..=1.0).show_value(false))
                        .changed()
                    {
                        self.state.rx[0].volume = vol; // optimistic echo
                        cmds.push(Command::SetVolume { rx: RxId::Main, v: vol });
                    }
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
                    let muted = self.state.rx[0].muted;
                    if crate::chrome::chip_accent(ui, muted, "MUTE", crate::theme::PINK, Color32::WHITE)
                        .clicked()
                    {
                        cmds.push(Command::SetMute { rx: RxId::Main, muted: !muted });
                    }
                });
                // Filter / Noise: squelch, noise blanker.
                ui.horizontal(|ui| {
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
                    // Spectral noise reduction — cycles Off → Low → Med → High.
                    let nr = self.state.rx[0].noise_reduction;
                    let nr_label =
                        if nr.is_on() { format!("NR {}", nr.label()) } else { "NR".to_string() };
                    if crate::chrome::chip(ui, nr.is_on(), nr_label)
                        .on_hover_text(
                            "Spectral noise reduction (voice) — click to cycle Off / Low / Med / High",
                        )
                        .clicked()
                    {
                        let next = nr.next();
                        self.state.rx[0].noise_reduction = next; // optimistic echo
                        cmds.push(Command::SetNoiseReduction { rx: RxId::Main, level: next });
                    }
                });
            });
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
        crate::chrome::module(ui, "Display", 284.0, |ui| {
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
            // Floor/ceiling + FFT size live in a popup off this button.
            let fft_btn = crate::chrome::chip(ui, false, "FFT")
                .on_hover_text("Spectrum floor / ceiling and FFT size");
            egui::Popup::from_toggle_button_response(&fft_btn)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .show(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                    ui.label(
                        RichText::new("SPECTRUM").color(crate::theme::CYAN_DIM).size(9.5).strong(),
                    );
                    ui.horizontal(|ui| {
                        ui.label("floor");
                        ui.add(
                            DragValue::new(&mut self.view.db_floor)
                                .speed(1.0)
                                .range(-160.0..=-40.0)
                                .suffix(" dB"),
                        );
                        ui.label("ceil");
                        ui.add(
                            DragValue::new(&mut self.view.db_ceil)
                                .speed(1.0)
                                .range(-100.0..=20.0)
                                .suffix(" dB"),
                        );
                    });
                    // Chips rather than a ComboBox: the combo opens a second popup
                    // layer, and clicking it counts as "outside" and closes this one.
                    ui.label(
                        RichText::new("FFT SIZE").color(crate::theme::CYAN_DIM).size(9.5).strong(),
                    );
                    ui.horizontal_wrapped(|ui| {
                        for n in [2048u32, 4096, 8192, 16384, 32768] {
                            if crate::chrome::chip(ui, self.view.fft_size == n, format!("{n}"))
                                .clicked()
                            {
                                self.view.fft_size = n;
                            }
                        }
                    });
                });
        });
    }

    fn windows_module(&mut self, ui: &mut egui::Ui) {
        crate::chrome::module(ui, "System", 220.0, |ui| {
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
        let mut digi_edit = self.digi_cfg_edit.clone();
        let digi_seeded = self.digi_cfg_seeded;

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
                    &mut digi_edit,
                    digi_seeded,
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
        // Callsign/grid from the General tab — same store as the FT8/SSTV setup
        // dialog. Only apply once seeded so we can't overwrite the engine's saved
        // config with defaults.
        if digi_seeded && digi_edit != self.digi_cfg_edit {
            self.digi_cfg_edit = digi_edit;
            cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
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
        digi_edit: &mut sdroxide_types::DigiConfig,
        digi_seeded: bool,
        tab: &mut SettingsTab,
    ) {
        use sdroxide_types::Backend;

        ui.horizontal(|ui| {
            for (t, label) in [
                (SettingsTab::General, "General"),
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
            SettingsTab::General => {
                ui.label(RichText::new("Station").size(14.0).strong().color(crate::theme::CYAN));
                ui.add_space(6.0);
                if !digi_seeded {
                    ui.label(
                        RichText::new(
                            "Enter a digital mode (FT8 / SSTV / …) once to load the saved values.",
                        )
                        .weak(),
                    );
                }
                ui.add_enabled_ui(digi_seeded, |ui| {
                    egui::Grid::new("general-grid").num_columns(2).spacing([12.0, 8.0]).show(
                        ui,
                        |ui| {
                            ui.label("Callsign");
                            if ui.text_edit_singleline(&mut digi_edit.my_call).changed() {
                                digi_edit.my_call = digi_edit.my_call.to_uppercase();
                            }
                            ui.end_row();
                            ui.label("Grid square");
                            ui.text_edit_singleline(&mut digi_edit.my_grid);
                            ui.end_row();
                        },
                    );
                });
                ui.add_space(8.0);
                ui.label(
                    RichText::new(
                        "Your callsign and grid, shared across FT8/FT4, SSTV image headers, and \
                         the logbook. Also editable from the FT8 / SSTV setup dialog.",
                    )
                    .weak(),
                );
            }
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

        ui.label("Waterfall palette");
        ComboBox::from_id_salt("ui-palette")
            .selected_text(colormap::NAMES[cfg.waterfall_palette.min(colormap::NAMES.len() - 1)])
            .show_ui(ui, |ui| {
                for (i, name) in colormap::NAMES.iter().enumerate() {
                    ui.selectable_value(&mut cfg.waterfall_palette, i, *name);
                }
            });
        ui.end_row();

        ui.label("Spectrum background");
        ui.horizontal(|ui| {
            ui.checkbox(&mut cfg.spectrum_gradient, "Gradient");
            ui.add_enabled_ui(cfg.spectrum_gradient, |ui| {
                ui.label("top");
                ui.color_edit_button_srgb(&mut cfg.gradient_top);
                ui.label("bottom");
                ui.color_edit_button_srgb(&mut cfg.gradient_bottom);
            });
        });
        ui.end_row();
    });
    ui.add_space(8.0);
    ui.label(
        RichText::new(
            "Higher frame rates look smoother but cost more CPU/GPU. Spectrum speed \
             sets how quickly the trace reacts (slower = smoother/more averaged). The \
             background gradient fills the spectrum area from the top colour down to \
             the bottom colour.",
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
                RadioEvent::SstvLine { image_id, y, rgb } => {
                    self.sstv.on_line(image_id, y, &rgb, &ctx);
                }
                RadioEvent::SstvImage { image_id, mode, w, h, png } => {
                    self.sstv.on_image(image_id, mode, w, h, &png, &ctx);
                }
                RadioEvent::SstvStatus(s) => {
                    // Adopt a *newly* detected RX mode for the next transmit, but
                    // don't re-apply a steady detection every frame — that would
                    // fight the operator's manual mode selection.
                    if s.detected != self.sstv.last_detected {
                        if let Some(m) = s.detected {
                            self.sstv.tx_mode = m;
                            self.sstv.preview_dirty = true;
                        }
                        self.sstv.last_detected = s.detected;
                    }
                    self.sstv.status = s;
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
                            if mode.is_sstv() {
                                self.sstv_panel(ui, &mut cmds);
                            } else if is_text {
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
        // SSTV calling frequencies (USB).
        Mode::Sstv => &[
            ("80m", 3_730_000.0),
            ("40m", 7_171_000.0),
            ("20m", 14_230_000.0),
            ("15m", 21_340_000.0),
            ("10m", 28_680_000.0),
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

// ───────────────────────────── SSTV panel ──────────────────────────────

/// A transmit-image slot: the (bounded) source picture plus its thumbnail.
struct SstvSlot {
    src_rgb: Vec<u8>,
    sw: u16,
    sh: u16,
    tex: egui::TextureHandle,
}

/// A received-image gallery entry.
#[allow(dead_code)] // not used on wasm
struct SstvRecv {
    mode: Option<SstvMode>,
    tex: egui::TextureHandle,
}

/// SSTV panel state: received gallery, in-progress incoming image, transmit
/// slots, the overlay message, the current mode, and cached textures.
struct SstvUi {
    tx_mode: SstvMode,
    /// Operator callsign for the transmit-image header (mirrors the digi config).
    callsign: String,
    /// Auto mode: RX auto-detects the mode; TX defaults to Martin 1 until a mode
    /// is heard or the operator picks one.
    auto: bool,
    /// Overlay message per image slot (index-aligned with `slots`). The message
    /// box edits the entry for `selected_slot`, so switching slots swaps the
    /// text — and each is persisted alongside its picture.
    slot_messages: Vec<String>,
    slots: Vec<Option<SstvSlot>>,
    selected_slot: usize,
    received: Vec<SstvRecv>,
    /// In-progress incoming image (painted line-by-line).
    rx_color: Option<egui::ColorImage>,
    rx_tex: Option<egui::TextureHandle>,
    rx_id: u32,
    status: SstvStatus,
    /// Received-gallery index currently shown enlarged in an overlay window.
    enlarged: Option<usize>,
    /// Last VIS/free-run-detected mode we auto-applied to `tx_mode`, so a steady
    /// detection doesn't keep overriding the operator's manual mode choice.
    last_detected: Option<SstvMode>,
    preview_tex: Option<egui::TextureHandle>,
    preview_dirty: bool,
    loaded_disk: bool,
    /// File-picker result inbox (raw image bytes), filled by the picker task.
    inbox: Arc<Mutex<Option<Vec<u8>>>>,
    pick_target: Option<usize>,
}

impl Default for SstvUi {
    fn default() -> Self {
        SstvUi {
            tx_mode: SstvMode::Martin1,
            callsign: String::new(),
            auto: true,
            slot_messages: vec![String::new(); 5],
            slots: (0..5).map(|_| None).collect(),
            selected_slot: 0,
            received: Vec::new(),
            rx_color: None,
            rx_tex: None,
            rx_id: 0,
            status: SstvStatus::default(),
            enlarged: None,
            last_detected: None,
            preview_tex: None,
            preview_dirty: true,
            loaded_disk: false,
            inbox: Arc::new(Mutex::new(None)),
            pick_target: None,
        }
    }
}

impl SstvUi {
    /// A decoded scanline arrived: paint it into the in-progress image.
    fn on_line(&mut self, id: u32, y: u16, rgb: &[u8], ctx: &egui::Context) {
        let Some(mode) = self.status.detected else { return };
        let (w, h) = mode.dimensions();
        if self.rx_id != id || self.rx_color.is_none() {
            self.rx_id = id;
            self.rx_color = Some(crate::sstv::color_image(&vec![0u8; w as usize * h as usize * 3], w, h));
        }
        let Some(ci) = self.rx_color.as_mut() else { return };
        let (w, h) = (w as usize, h as usize);
        if (y as usize) < h && rgb.len() >= w * 3 {
            let row = y as usize * w;
            for x in 0..w {
                ci.pixels[row + x] = Color32::from_rgb(rgb[x * 3], rgb[x * 3 + 1], rgb[x * 3 + 2]);
            }
        }
        self.rx_tex =
            Some(ctx.load_texture("sstv_rx", ci.clone(), egui::TextureOptions::NEAREST));
    }

    /// A completed image arrived: decode and add it to the gallery.
    fn on_image(&mut self, _id: u32, mode: SstvMode, _w: u16, _h: u16, png: &[u8], ctx: &egui::Context) {
        if let Some((rgb, w, h)) = crate::sstv::decode_image(png) {
            let ci = crate::sstv::color_image(&rgb, w, h);
            let tex = ctx.load_texture("sstv_recv", ci, egui::TextureOptions::NEAREST);
            self.received.insert(0, SstvRecv { mode: Some(mode), tex });
            self.received.truncate(60);
        }
        self.rx_color = None;
        self.rx_tex = None;
    }

    /// The overlay message for the slot currently being edited.
    fn current_message(&self) -> &str {
        self.slot_messages.get(self.selected_slot).map(String::as_str).unwrap_or("")
    }

    /// Persist the per-slot overlay messages to the config file (native only).
    fn save_messages(&self) {
        sstv_save_messages(&self.slot_messages);
    }

    /// Rebuild the transmit preview when the mode, slot, or message changed.
    fn ensure_preview(&mut self, ctx: &egui::Context) {
        if !self.preview_dirty {
            return;
        }
        self.preview_dirty = false;
        let message = self.current_message().to_string();
        match self.slots.get(self.selected_slot).and_then(|s| s.as_ref()) {
            Some(slot) => {
                let (rgb, w, h) = crate::sstv::compose(
                    self.tx_mode,
                    &slot.src_rgb,
                    slot.sw,
                    slot.sh,
                    &message,
                    &self.callsign,
                );
                let ci = crate::sstv::color_image(&rgb, w, h);
                self.preview_tex =
                    Some(ctx.load_texture("sstv_preview", ci, egui::TextureOptions::NEAREST));
            }
            None => self.preview_tex = None,
        }
    }

    /// The composed PNG for the current selection, for transmit.
    fn compose_png(&self) -> Option<Vec<u8>> {
        let slot = self.slots.get(self.selected_slot).and_then(|s| s.as_ref())?;
        let (rgb, w, h) = crate::sstv::compose(
            self.tx_mode,
            &slot.src_rgb,
            slot.sw,
            slot.sh,
            self.current_message(),
            &self.callsign,
        );
        crate::sstv::encode_png(&rgb, w, h)
    }

    /// Accept a picked image file into `slot`, building a thumbnail texture.
    fn set_slot(&mut self, slot: usize, bytes: &[u8], ctx: &egui::Context) {
        let Some((rgb, w, h)) = crate::sstv::load_source_bounded(bytes, 1024) else { return };
        let ci = crate::sstv::color_image(&rgb, w, h);
        let tex = ctx.load_texture("sstv_slot", ci, egui::TextureOptions::LINEAR);
        if let Some(cell) = self.slots.get_mut(slot) {
            *cell = Some(SstvSlot { src_rgb: rgb, sw: w, sh: h, tex });
        }
        self.selected_slot = slot;
        self.preview_dirty = true;
        sstv_save_slot(slot, bytes);
    }
}

impl SdroxideApp {
    fn sstv_panel(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let ctx = ui.ctx().clone();
        self.sstv_load_disk_once(&ctx);
        // Drain a completed file-pick (only consume the target once bytes arrive).
        let picked = self.sstv.inbox.lock().ok().and_then(|mut g| g.take());
        if let Some(bytes) = picked {
            if let Some(target) = self.sstv.pick_target.take() {
                self.sstv.set_slot(target, &bytes, &ctx);
            }
        }
        // Keep the header callsign in sync with the operator config.
        if self.sstv.callsign != self.digi_cfg_edit.my_call {
            self.sstv.callsign = self.digi_cfg_edit.my_call.clone();
            self.sstv.preview_dirty = true;
        }
        self.sstv.ensure_preview(&ctx);
        ctx.request_repaint_after(Duration::from_millis(120));

        let st = self.sstv.status;
        let tx_active = st.tx_active;

        // Whole-panel size. The mode/signal/slant controls sit in a boxed strip
        // on the left above LIVE + RECEIVED; the transmit compositor spans the
        // full height on the right, reclaiming the space the old full-width
        // control rows used to leave empty at the top.
        let avail = ui.available_size();
        let full_h = avail.y;
        let tx_w = 460.0_f32.min((avail.x * 0.40).max(320.0));
        let left_w = (avail.x - tx_w - 8.0).max(300.0);
        let gallery_w = 264.0_f32.min(left_w * 0.42);
        let live_w = (left_w - gallery_w - 8.0).max(160.0);

        ui.horizontal_top(|ui| {
            // A received thumbnail was clicked → enlarge it (applied after the row).
            let mut enlarge: Option<usize> = None;

            // ── LEFT: boxed controls, then LIVE + RECEIVED ──
            ui.allocate_ui_with_layout(
                egui::vec2(left_w, full_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    egui::Frame::new()
                        .fill(crate::theme::ROW_BG)
                        .stroke(egui::Stroke::new(1.0, crate::theme::LINE_LIT))
                        .inner_margin(egui::Margin { left: 8, right: 8, top: 6, bottom: 7 })
                        .show(ui, |ui| {
                            ui.set_min_width(left_w - 16.0);
                            ui.set_max_width(left_w - 16.0);

                            // Mode selection: Auto + the per-mode chips.
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    RichText::new("SSTV")
                                        .size(12.0)
                                        .strong()
                                        .color(crate::theme::CYAN),
                                );
                                let auto_label = if self.sstv.auto {
                                    format!("Auto ({})", self.sstv.tx_mode.label())
                                } else {
                                    "Auto".to_string()
                                };
                                if crate::chrome::chip(ui, self.sstv.auto, &auto_label).clicked() {
                                    self.sstv.auto = true;
                                    self.sstv.tx_mode = SstvMode::Martin1;
                                    self.sstv.preview_dirty = true;
                                    cmds.push(Command::SstvSetMode(None));
                                }
                                for m in SstvMode::ALL {
                                    let active = !self.sstv.auto && self.sstv.tx_mode == m;
                                    if crate::chrome::chip(ui, active, m.label()).clicked() {
                                        self.sstv.auto = false;
                                        self.sstv.tx_mode = m;
                                        self.sstv.preview_dirty = true;
                                        cmds.push(Command::SstvSetMode(Some(m)));
                                    }
                                }
                            });
                            ui.add_space(5.0);

                            // Signal meter + activity, and the TX-slant trim.
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new("Signal").size(10.0).weak());
                                sstv_level_bar(ui, st.signal);
                                if tx_active {
                                    ui.label(
                                        RichText::new(format!("● TX {:.0}%", st.progress * 100.0))
                                            .size(11.0)
                                            .strong()
                                            .color(crate::theme::PINK),
                                    );
                                } else if st.rx_active {
                                    ui.label(
                                        RichText::new(format!("● RX {:.0}%", st.progress * 100.0))
                                            .size(11.0)
                                            .strong()
                                            .color(crate::theme::GREEN),
                                    );
                                } else if let Some(m) = st.detected {
                                    ui.label(
                                        RichText::new(format!("last: {}", m.label()))
                                            .size(10.0)
                                            .weak(),
                                    );
                                } else {
                                    ui.label(RichText::new("listening…").size(10.0).weak());
                                }

                                ui.add_space(12.0);
                                ui.separator();
                                ui.label(RichText::new("TX slant").size(10.0).weak()).on_hover_text(
                                    "Transmit clock trim (ppm) to remove slant on the far-end decoder",
                                );
                                ui.add_enabled_ui(self.digi_cfg_seeded, |ui| {
                                    ui.spacing_mut().slider_width = 130.0;
                                    let resp = ui.add(
                                        egui::Slider::new(
                                            &mut self.digi_cfg_edit.sstv_tx_ppm,
                                            -5000.0..=5000.0,
                                        )
                                        .suffix(" ppm")
                                        .fixed_decimals(0),
                                    );
                                    if resp.drag_stopped() || (resp.changed() && !resp.dragged()) {
                                        cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
                                    }
                                    if ui
                                        .small_button("0")
                                        .on_hover_text("Reset to 0 ppm")
                                        .clicked()
                                    {
                                        self.digi_cfg_edit.sstv_tx_ppm = 0.0;
                                        cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
                                    }
                                });
                            });
                        });
                    ui.add_space(6.0);

                    // LIVE + RECEIVED fill the remaining height of the left column.
                    let row_h = ui.available_height().max(160.0);
                    ui.horizontal_top(|ui| {
                        // LIVE: the picture currently decoding, shown large.
                        sstv_section(ui, "LIVE", egui::vec2(live_w, row_h), |ui| {
                            ui.centered_and_justified(|ui| {
                                if let Some(tex) = &self.sstv.rx_tex {
                                    ui.add(
                                        egui::Image::new(tex)
                                            .max_height(row_h - 34.0)
                                            .max_width(live_w - 16.0),
                                    );
                                } else {
                                    let msg = if st.signal > 0.0008 {
                                        "waiting for a signal…"
                                    } else {
                                        "no / low audio"
                                    };
                                    ui.label(RichText::new(msg).size(11.0).weak());
                                }
                            });
                        });
                        ui.add_space(8.0);

                        // RECEIVED: narrow multi-column gallery of decoded pictures.
                        sstv_section(ui, "RECEIVED", egui::vec2(gallery_w, row_h), |ui| {
                            if self.sstv.received.is_empty() {
                                ui.label(
                                    RichText::new("Decoded pictures collect here.")
                                        .size(11.0)
                                        .weak(),
                                );
                                return;
                            }
                            let thumb = egui::vec2(112.0, 90.0);
                            egui::ScrollArea::vertical()
                                .id_salt("sstv-gallery")
                                .max_height(row_h - 24.0)
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.spacing_mut().item_spacing = egui::vec2(5.0, 5.0);
                                        for (i, r) in self.sstv.received.iter().enumerate() {
                                            let resp = ui
                                                .add(
                                                    egui::Image::new(&r.tex)
                                                        .fit_to_exact_size(thumb)
                                                        .corner_radius(2.0)
                                                        .sense(egui::Sense::click()),
                                                )
                                                .on_hover_text("Click to enlarge");
                                            if resp.clicked() {
                                                enlarge = Some(i);
                                            }
                                        }
                                    });
                                });
                        });
                    });
                },
            );

            ui.add_space(8.0);

            // ── RIGHT: fixed-width transmit compositor, full height ──
            ui.allocate_ui(egui::vec2(tx_w, full_h), |ui| {
                sstv_section(ui, "TRANSMIT", egui::vec2(tx_w, full_h), |ui| {
                    let inner_w = tx_w - 16.0;

                    // Five source slots — the highlighted one acts as the active
                    // "tab" whose message the box below edits.
                    ui.label(
                        RichText::new("Image slots — click one to edit its message")
                            .size(9.5)
                            .weak(),
                    );
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 5.0;
                        for i in 0..self.sstv.slots.len() {
                            let sel = self.sstv.selected_slot == i;
                            let size = egui::vec2(70.0, 54.0);
                            let resp = if let Some(slot) = &self.sstv.slots[i] {
                                ui.add(
                                    egui::Image::new(&slot.tex)
                                        .fit_to_exact_size(size)
                                        .corner_radius(2.0)
                                        .sense(egui::Sense::click()),
                                )
                            } else {
                                let (rect, resp) =
                                    ui.allocate_exact_size(size, egui::Sense::click());
                                ui.painter().rect_stroke(
                                    rect,
                                    2.0,
                                    egui::Stroke::new(1.0, Color32::from_gray(70)),
                                    egui::StrokeKind::Inside,
                                );
                                ui.painter().text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "+",
                                    egui::FontId::proportional(22.0),
                                    Color32::from_gray(110),
                                );
                                resp
                            };
                            // Active-tab highlight: a cyan wash + heavier border so
                            // it is obvious which slot the message box targets.
                            if sel {
                                ui.painter().rect_filled(
                                    resp.rect,
                                    2.0,
                                    Color32::from_rgba_unmultiplied(0x00, 0xd0, 0xf4, 34),
                                );
                                ui.painter().rect_stroke(
                                    resp.rect,
                                    2.0,
                                    egui::Stroke::new(2.5, crate::theme::CYAN),
                                    egui::StrokeKind::Outside,
                                );
                            }
                            // Slot number badge (1..5), like a tab label.
                            let badge = egui::Rect::from_min_size(
                                resp.rect.left_top() + egui::vec2(2.0, 2.0),
                                egui::vec2(15.0, 13.0),
                            );
                            ui.painter().rect_filled(badge, 2.0, Color32::from_black_alpha(150));
                            ui.painter().text(
                                badge.center(),
                                egui::Align2::CENTER_CENTER,
                                format!("{}", i + 1),
                                egui::FontId::proportional(10.0),
                                if sel { crate::theme::CYAN } else { Color32::from_gray(170) },
                            );
                            let resp = resp.on_hover_text(
                                "Click to edit this slot's message · double-click to load an image",
                            );
                            if resp.double_clicked() {
                                self.sstv.pick_target = Some(i);
                                pick_image(self.sstv.inbox.clone());
                            } else if resp.clicked() && !sel {
                                self.sstv.save_messages(); // flush the slot we leave
                                self.sstv.selected_slot = i;
                                self.sstv.preview_dirty = true;
                            }
                        }
                    });
                    ui.add_space(5.0);

                    // Explicit image load button for the active slot.
                    ui.horizontal(|ui| {
                        let sel = self.sstv.selected_slot;
                        let has_img =
                            self.sstv.slots.get(sel).map(|s| s.is_some()) == Some(true);
                        let label = if has_img { "Change image…" } else { "Load image…" };
                        if crate::chrome::chip(ui, false, label).clicked() {
                            self.sstv.pick_target = Some(sel);
                            pick_image(self.sstv.inbox.clone());
                        }
                    });
                    ui.add_space(6.0);

                    // Preview gets a capped share of the height; the message box
                    // grows to fill whatever's left above the buttons.
                    let btn_h = 42.0;
                    let gap = 6.0;
                    ui.label(RichText::new("Preview (what is transmitted)").size(9.5).weak());
                    let preview_h = (ui.available_height() * 0.45).clamp(80.0, 260.0);
                    egui::Frame::new()
                        .fill(Color32::from_gray(6))
                        .stroke(egui::Stroke::new(1.0, crate::theme::LINE_LIT))
                        .inner_margin(2.0)
                        .show(ui, |ui| {
                            ui.set_min_size(egui::vec2(inner_w, preview_h));
                            ui.set_max_size(egui::vec2(inner_w, preview_h));
                            ui.centered_and_justified(|ui| {
                                if let Some(tex) = &self.sstv.preview_tex {
                                    ui.add(
                                        egui::Image::new(tex)
                                            .max_height(preview_h - 4.0)
                                            .max_width(inner_w - 4.0),
                                    );
                                } else {
                                    ui.label(
                                        RichText::new("Load an image into this slot →")
                                            .size(11.0)
                                            .weak(),
                                    );
                                }
                            });
                        });
                    ui.add_space(gap);

                    // Overlay message for the active slot — fills the height above
                    // the buttons; persisted when focus leaves the box or the slot
                    // changes. A per-slot id keeps each tab's cursor independent.
                    let sel = self.sstv.selected_slot;
                    let msg_h = (ui.available_height() - btn_h - gap).max(48.0);
                    let resp = ui
                        .push_id(sel, |ui| {
                            ui.add_sized(
                                egui::vec2(inner_w, msg_h),
                                egui::TextEdit::multiline(&mut self.sstv.slot_messages[sel])
                                    .hint_text("Drawn on this slot's image"),
                            )
                        })
                        .inner;
                    if resp.changed() {
                        self.sstv.preview_dirty = true;
                    }
                    if resp.lost_focus() {
                        self.sstv.save_messages();
                    }
                    ui.add_space(gap);

                    // Large cut-corner TX / ABORT buttons.
                    ui.horizontal(|ui| {
                        let can_tx = self.sstv.slots.get(self.sstv.selected_slot).map(|s| s.is_some())
                            == Some(true)
                            && !tx_active;
                        let tx = ui
                            .add_enabled_ui(can_tx, |ui| {
                                crate::chrome::chip_accent(
                                    ui,
                                    can_tx,
                                    RichText::new("   TX   ").size(16.0).strong(),
                                    crate::theme::PINK,
                                    Color32::WHITE,
                                )
                            })
                            .inner;
                        if tx.clicked() {
                            self.sstv.save_messages(); // capture any unfocused edit
                            if let Some(png) = self.sstv.compose_png() {
                                cmds.push(Command::SstvTx { mode: self.sstv.tx_mode, png });
                            }
                        }
                        ui.add_space(8.0);
                        let abort = ui
                            .add_enabled_ui(tx_active, |ui| {
                                crate::chrome::chip(
                                    ui,
                                    false,
                                    RichText::new(" ABORT TX ").size(15.0).strong(),
                                )
                            })
                            .inner;
                        if abort.clicked() {
                            cmds.push(Command::DigiAbortTx);
                        }
                    });
                });
            });

            if let Some(i) = enlarge {
                self.sstv.enlarged = Some(i);
            }
        });

        // Enlarged view of a clicked received image (overlay window).
        if let Some(idx) = self.sstv.enlarged {
            let mut open = true;
            if let Some(r) = self.sstv.received.get(idx) {
                egui::Window::new("Received image")
                    .open(&mut open)
                    .collapsible(false)
                    .resizable(true)
                    .default_size([660.0, 528.0])
                    .frame(crate::chrome::window_frame())
                    .show(&ctx, |ui| {
                        // Scale up to fill the window width (preserving aspect).
                        let native = r.tex.size_vec2();
                        let avail_w = ui.available_width().min(1000.0);
                        let scale = (avail_w / native.x.max(1.0)).clamp(1.0, 4.0);
                        ui.add(egui::Image::new(&r.tex).fit_to_exact_size(native * scale));
                    });
            } else {
                open = false;
            }
            if !open {
                self.sstv.enlarged = None;
            }
        }
    }

    /// On first entry, load any persisted transmit slots and received gallery
    /// from disk (native only).
    fn sstv_load_disk_once(&mut self, ctx: &egui::Context) {
        if self.sstv.loaded_disk {
            return;
        }
        self.sstv.loaded_disk = true;
        for (i, entry) in sstv_load_slots().into_iter().enumerate() {
            if let Some((rgb, w, h)) = entry {
                let ci = crate::sstv::color_image(&rgb, w, h);
                let tex = ctx.load_texture("sstv_slot", ci, egui::TextureOptions::LINEAR);
                if let Some(cell) = self.sstv.slots.get_mut(i) {
                    *cell = Some(SstvSlot { src_rgb: rgb, sw: w, sh: h, tex });
                }
            }
        }
        // Restore the per-slot overlay messages (padded to the slot count).
        for (i, msg) in sstv_load_messages().into_iter().enumerate() {
            if let Some(cell) = self.sstv.slot_messages.get_mut(i) {
                *cell = msg;
            }
        }
        for (rgb, w, h) in sstv_load_gallery() {
            let ci = crate::sstv::color_image(&rgb, w, h);
            let tex = ctx.load_texture("sstv_recv", ci, egui::TextureOptions::NEAREST);
            self.sstv.received.push(SstvRecv { mode: None, tex });
        }
    }
}

// ── File picker (native thread / wasm async) ──

#[cfg(not(target_arch = "wasm32"))]
fn pick_image(inbox: Arc<Mutex<Option<Vec<u8>>>>) {
    std::thread::spawn(move || {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Image", &["png", "jpg", "jpeg"])
            .pick_file()
        {
            if let Ok(bytes) = std::fs::read(&path) {
                if let Ok(mut g) = inbox.lock() {
                    *g = Some(bytes);
                }
            }
        }
    });
}

#[cfg(target_arch = "wasm32")]
fn pick_image(inbox: Arc<Mutex<Option<Vec<u8>>>>) {
    wasm_bindgen_futures::spawn_local(async move {
        if let Some(file) = rfd::AsyncFileDialog::new()
            .add_filter("Image", &["png", "jpg", "jpeg"])
            .pick_file()
            .await
        {
            let bytes = file.read().await;
            if let Ok(mut g) = inbox.lock() {
                *g = Some(bytes);
            }
        }
    });
}

// ── Disk persistence (native only) ──

#[cfg(not(target_arch = "wasm32"))]
fn sstv_save_slot(i: usize, png_bytes: &[u8]) {
    if let Ok(dir) = sdroxide_config::sstv_tx_dir() {
        let _ = std::fs::write(dir.join(format!("slot{i}.png")), png_bytes);
    }
}
#[cfg(target_arch = "wasm32")]
fn sstv_save_slot(_i: usize, _png_bytes: &[u8]) {}

#[cfg(not(target_arch = "wasm32"))]
fn sstv_save_messages(messages: &[String]) {
    let _ = sdroxide_config::save_sstv_messages(messages);
}
#[cfg(target_arch = "wasm32")]
fn sstv_save_messages(_messages: &[String]) {}

#[cfg(not(target_arch = "wasm32"))]
fn sstv_load_messages() -> Vec<String> {
    sdroxide_config::load_sstv_messages()
}
#[cfg(target_arch = "wasm32")]
fn sstv_load_messages() -> Vec<String> {
    Vec::new()
}

#[cfg(not(target_arch = "wasm32"))]
fn sstv_load_slots() -> Vec<Option<(Vec<u8>, u16, u16)>> {
    let mut out = Vec::new();
    let dir = match sdroxide_config::sstv_tx_dir() {
        Ok(d) => d,
        Err(_) => return (0..5).map(|_| None).collect(),
    };
    for i in 0..5 {
        let entry = std::fs::read(dir.join(format!("slot{i}.png")))
            .ok()
            .and_then(|b| crate::sstv::load_source_bounded(&b, 1024));
        out.push(entry);
    }
    out
}
#[cfg(target_arch = "wasm32")]
fn sstv_load_slots() -> Vec<Option<(Vec<u8>, u16, u16)>> {
    (0..5).map(|_| None).collect()
}

#[cfg(not(target_arch = "wasm32"))]
fn sstv_load_gallery() -> Vec<(Vec<u8>, u16, u16)> {
    let mut entries: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(dir) = sdroxide_config::sstv_rx_dir() {
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) == Some("png") {
                    entries.push(p);
                }
            }
        }
    }
    // Newest first by filename (timestamps), cap the count.
    entries.sort();
    entries.reverse();
    entries.truncate(40);
    entries
        .into_iter()
        .filter_map(|p| std::fs::read(&p).ok().and_then(|b| crate::sstv::decode_image(&b)))
        .collect()
}
#[cfg(target_arch = "wasm32")]
fn sstv_load_gallery() -> Vec<(Vec<u8>, u16, u16)> {
    Vec::new()
}

/// A titled, bordered section box of a fixed size, for the SSTV panel's LIVE /
/// RECEIVED / TRANSMIT areas.
fn sstv_section<R>(
    ui: &mut egui::Ui,
    title: &str,
    size: egui::Vec2,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    // Force a top-down layout: `allocate_ui` would otherwise inherit the parent's
    // horizontal layout (we're inside a `horizontal_top`), laying the section's
    // contents out side by side instead of stacked.
    ui.allocate_ui_with_layout(size, egui::Layout::top_down(egui::Align::Min), |ui| {
        egui::Frame::new()
            .fill(crate::theme::ROW_BG)
            .stroke(egui::Stroke::new(1.0, crate::theme::LINE_LIT))
            .inner_margin(egui::Margin { left: 8, right: 8, top: 5, bottom: 7 })
            .show(ui, |ui| {
                ui.set_min_size(egui::vec2(size.x - 16.0, size.y - 12.0));
                ui.set_max_width(size.x - 16.0);
                ui.label(RichText::new(title).size(9.5).strong().color(crate::theme::CYAN_DIM));
                ui.add_space(3.0);
                add(ui)
            })
            .inner
    })
    .inner
}

/// A small horizontal signal-activity meter (level ~0..1), so the operator can
/// confirm receive audio is reaching the SSTV decoder.
fn sstv_level_bar(ui: &mut egui::Ui, level: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(90.0, 10.0), egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 2.0, Color32::from_gray(20));
    // Log scale (~ -60..0 dBFS mean-abs) so weak-but-decodable signals still show.
    let db = 20.0 * level.max(1e-6).log10();
    let frac = ((db + 60.0) / 60.0).clamp(0.0, 1.0);
    let mut fill = rect;
    fill.set_width(rect.width() * frac);
    let col = if frac > 0.06 { crate::theme::GREEN } else { Color32::from_gray(45) };
    p.rect_filled(fill, 2.0, col);
    p.rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0, Color32::from_gray(60)),
        egui::StrokeKind::Inside,
    );
}
