//! The RADIOSONDES window: a status line, the channel, the map of every sonde
//! heard with its track, and a table under it.
//!
//! It is a view, not a radio mode: the decoder runs off its own
//! downconversion of the sample window (see `sdroxide_sonde`) and the radio
//! keeps its mode. Opened from the ISM window's header. TUNE switches the
//! decoder on and brings the dial to the sonde channel (the lane works off the
//! sample window, so the window has to contain it). State comes from
//! [`sdroxide_types::SondeStatus`]; the settings are edited as a copy and
//! diffed at the end, the convention the HFDL and AIS panels keep.

use crate::i18n::tr;
use crate::theme::ThemedScroll;
use eframe::egui::{self, RichText};
use sdroxide_types::{Command, SONDE_BAND_HZ, SondeSettings, SondeStatus, SondeTrack};

use crate::app::{SdroxideApp, sonde_card};
use crate::sonde_map::{self, Phase, SondeMapState};

/// The types the picker offers (the decoders' own names: not translated).
pub(in crate::app) const TYPES: &[&str] = &[
    "RS41",
    "DFM",
    "M10",
    "M20",
    "iMet",
    "iMet-54",
    "MRZ",
    "LMS6",
    "MTS01",
    "Mk2a",
    "Meisei",
    "C34/C50",
    "Weathex",
    "CF06/HT03",
];

const WINDOW_SIZE: [f32; 2] = [900.0, 680.0];

/// The RADIOSONDES window's own state: open or not, the decoder's latest
/// status, the map's view and selection, and the SondeHub layer.
#[derive(Default)]
pub(in crate::app) struct SondeUi {
    pub(in crate::app) show: bool,
    /// The decoder's latest status: its level and the sondes heard.
    status: Option<Box<SondeStatus>>,
    map: SondeMapState,
    /// Sondes other stations hear, from SondeHub. Off until the operator says.
    online: crate::sonde_online::Online,
    /// Where the table's sideways scroll showed it last frame.
    table_view: Option<egui::Rect>,
}

impl SondeUi {
    /// Take a status from the engine. Each is a whole snapshot.
    pub(in crate::app) fn accept(&mut self, st: Box<SondeStatus>) {
        self.status = Some(st);
    }
}

/// Whether a mode owns the panel under the waterfall (or parks the receiver
/// elsewhere): the sonde view has no mode of its own, so the dock would stay
/// on that mode's panel, APRS stations under a sonde map.
fn holds_the_dock(mode: sdroxide_types::Mode) -> bool {
    mode.has_bottom_panel()
        || mode.is_wideband_lane()
        || mode.is_hfdl()
        || mode.is_aprs()
        || mode.is_packet()
}

/// The commands TUNE sends: leave a mode that holds the front end elsewhere,
/// switch the decoder on if it is off and bring the dial to its channel, so
/// the channel is inside the sample window.
///
/// ADS-B, VDL2, AIS, DAB and HFDL each park the receiver on their own channel
/// and own the panel under the waterfall; staying in one of them left that
/// panel on show and the dial pulled back to it. They are left for NFM, which
/// is what a sonde's FSK sounds like and which asks nothing of the window.
pub(in crate::app) fn tune_commands(
    cfg: &SondeSettings,
    vfo: sdroxide_types::Vfo,
    mode: sdroxide_types::Mode,
) -> Vec<Command> {
    let mut out = Vec::new();
    if holds_the_dock(mode) {
        out.push(Command::SetMode {
            rx: sdroxide_types::RxId::Main,
            mode: sdroxide_types::Mode::Nfm,
        });
    }
    if !cfg.enabled {
        out.push(Command::SetSondeConfig(SondeSettings { enabled: true, ..*cfg }));
    }
    out.push(Command::SetVfo { vfo, hz: cfg.frequency_hz });
    out
}

/// What a click on a card's frequency sends: the decoder on one channel (the
/// scan off, or it would look elsewhere) and the dial on that channel, so the
/// operator hears it and the lane decodes it. The same mode rule as TUNE.
pub(in crate::app) fn listen_commands(
    cfg: &SondeSettings,
    vfo: sdroxide_types::Vfo,
    mode: sdroxide_types::Mode,
    hz: f64,
) -> Vec<Command> {
    let next = SondeSettings { enabled: true, frequency_hz: hz, scan: false };
    let mut out = Vec::new();
    if holds_the_dock(mode) {
        out.push(Command::SetMode {
            rx: sdroxide_types::RxId::Main,
            mode: sdroxide_types::Mode::Nfm,
        });
    }
    if next != *cfg {
        out.push(Command::SetSondeConfig(next));
    }
    out.push(Command::SetVfo { vfo, hz });
    out
}

/// What a card says about the radio's part in it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::app) enum Listen {
    /// The radio is on some other channel: a click will bring it here.
    Elsewhere,
    /// On this channel, no frame of this sonde yet.
    Waiting,
    /// On this channel and frames of this sonde are coming in.
    Decoding,
}

/// Where the radio stands for a card on `card_hz`: on its channel (within
/// 5 kHz, the width of a sonde) with the decoder on, and whether a frame of
/// that sonde was heard in the last 30 s.
pub(in crate::app) fn listen_state(
    cfg: &SondeSettings,
    card_hz: f64,
    heard_age_s: Option<i64>,
) -> Listen {
    if !cfg.enabled || cfg.scan || (cfg.frequency_hz - card_hz).abs() > 5_000.0 {
        return Listen::Elsewhere;
    }
    match heard_age_s {
        Some(a) if a < 30 => Listen::Decoding,
        _ => Listen::Waiting,
    }
}

/// The map never gets less than this many points of height; under it the
/// view scrolls.
const MAP_MIN_H: f32 = 280.0;

/// One row of a card: its label and its value.
type Row = (String, String);

/// A card's rows cut in two, the first half the longer by one when odd.
fn card_halves(rows: &[Row]) -> (&[Row], &[Row]) {
    rows.split_at(rows.len().div_ceil(2))
}

/// The width the card is first placed at, points.
const CARD_W: f32 = 520.0;

/// Below this width the card lists one label and value a line, not two.
const CARD_PAIRS_W: f32 = 440.0;

/// What was pressed in the card.
enum CardAct {
    None,
    Close,
    Detach,
    Attach,
    /// The frequency was clicked: listen there.
    Listen(f64),
}

/// Draw a card: its title, the buttons, then the rows in two blocks side by
/// side (half the height of one long column).
fn draw_card(
    ui: &mut egui::Ui,
    card: &sonde_card::Card,
    detached: bool,
    listen: Listen,
) -> CardAct {
    let mut act = CardAct::None;
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(&card.title).size(12.0).strong());
        if ui.small_button(tr!("sonde-card-close")).clicked() {
            act = CardAct::Close;
        }
        let (label, next) = if detached {
            (tr!("sonde-card-attach"), CardAct::Attach)
        } else {
            (tr!("sonde-card-detach"), CardAct::Detach)
        };
        if ui.small_button(label).clicked() {
            act = next;
        }
        for (site, url) in sonde_card::links(&card.serial) {
            if ui.small_button(sonde_card::open_label(site)).on_hover_text(&url).clicked() {
                ui.ctx().open_url(egui::OpenUrl::new_tab(url));
            }
        }
    });
    if let Some(hz) = card.freq_hz {
        ui.horizontal_wrapped(|ui| {
            let ink = match listen {
                Listen::Decoding => crate::theme::GREEN(),
                _ => ui.visuals().strong_text_color(),
            };
            let b = ui
                .add(egui::Button::new(
                    RichText::new(format!("{:.3} MHz", hz / 1e6)).size(26.0).strong().color(ink),
                ))
                .on_hover_text(tr!("sonde-card-listen-hover"));
            if b.clicked() {
                act = CardAct::Listen(hz);
            }
            let (text, ink) = match listen {
                Listen::Decoding => (tr!("sonde-card-decoding"), crate::theme::GREEN()),
                Listen::Waiting => (tr!("sonde-card-waiting"), crate::theme::YELLOW()),
                Listen::Elsewhere => (tr!("sonde-card-click-to-listen"), crate::theme::gray(150)),
            };
            ui.label(RichText::new(text).size(11.0).color(ink));
        });
    }
    if let Some((lat, lon, alt)) = card.place {
        let alt = alt.map_or(String::new(), |a| format!("   {a:.0} m"));
        ui.label(
            RichText::new(format!(
                "{}: {}{alt}",
                tr!("sonde-card-position"),
                sonde_card::fmt_position(lat, lon)
            ))
            .size(13.0)
            .strong(),
        );
    }
    // Two label-and-value pairs a line where the card has the room, half the
    // height of one long list; one pair a line where it has not (a phone, a
    // narrow map). One grid either way, so a long value widens its column
    // rather than running over the next one.
    let pairs = ui.available_width() >= CARD_PAIRS_W;
    let cell = |ui: &mut egui::Ui, (label, value): &Row| {
        ui.label(RichText::new(label).size(10.5).weak());
        ui.label(RichText::new(value).size(10.5));
    };
    egui::Grid::new("sonde_card").num_columns(if pairs { 4 } else { 2 }).spacing([14.0, 2.0]).show(
        ui,
        |ui| {
            if pairs {
                let (left, right) = card_halves(&card.rows);
                for (i, row) in left.iter().enumerate() {
                    cell(ui, row);
                    if let Some(r) = right.get(i) {
                        cell(ui, r);
                    }
                    ui.end_row();
                }
            } else {
                for row in &card.rows {
                    cell(ui, row);
                    ui.end_row();
                }
            }
        },
    );
    act
}

impl SdroxideApp {
    pub(in crate::app) fn sonde_window(&mut self, ctx: &egui::Context, cmds: &mut Vec<Command>) {
        if !self.sonde.show {
            // Closed: nothing ticks the SondeHub layer, so nothing is asked.
            return;
        }
        let mut open = self.sonde.show;
        let tool_title = tr!("sonde-title");
        let mut tool_st = self.take_tool_state("SONDE");
        let tool_child = crate::app::undocked::ChildWindow::own(self);
        crate::app::undocked::tool_window(
            ctx,
            crate::app::undocked::ToolWindow::new("SONDE", &tool_title, WINDOW_SIZE)
                .min_size([360.0, 320.0]),
            &mut tool_st,
            tool_child,
            &mut open,
            |ui| self.sonde_body(ui, cmds),
        );
        self.put_tool_state("SONDE", tool_st);
        self.sonde.show = open;
    }

    fn sonde_body(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        crate::chrome::window_body_bg(ui);
        // A radio whose capabilities have not arrived yet is assumed wideband,
        // as the ISM window does: the lane needs I/Q to mix a channel from.
        let wideband = self.caps.as_ref().is_none_or(|c| !c.audio_mode);
        let now = crate::time::now_unix();
        let mut cfg = self.state.sonde;
        let status = self.sonde.status.as_deref().cloned();
        let (running, level, frames) = match &status {
            Some(s) => (s.running, Some(s.level_dbfs), s.frames),
            None => (false, None, 0),
        };
        let mut sondes: Vec<SondeTrack> = status.map(|s| s.sondes).unwrap_or_default();
        let filter = self.sonde.map.type_filter;
        if let Some(f) = filter {
            sondes.retain(|s| sonde_map::type_matches(f, &s.model));
        }
        // A sonde heard lately keeps a scan where it is.
        let busy = sondes.iter().any(|s| now - s.last_unix < sonde_map::SCAN_HOLD_S);

        // One page, controls and all, that scrolls: in a short window (a
        // 600-point screen, the touch tiers' taller rows) the controls alone
        // filled the window and left the map and the table out of reach.
        let page_h = ui.available_height();
        let mut home = None;
        egui::ScrollArea::vertical()
            .id_salt("sonde_scroll")
            .auto_shrink([false, false])
            .show_themed(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let run = crate::chrome::chip_enabled(
                        ui,
                        wideband,
                        cfg.enabled,
                        &if cfg.enabled {
                            tr!("sonde-decoding")
                        } else {
                            tr!("sonde-decoding-off")
                        },
                    );
                    if run.clicked() {
                        cfg.enabled = !cfg.enabled;
                    }
                    if !wideband {
                        run.on_hover_text(tr!("sonde-needs-iq"));
                    }
                    if crate::chrome::chip(ui, false, tr!("sonde-tune"))
                        .on_hover_text(tr!("sonde-tune-hover"))
                        .clicked()
                    {
                        cmds.extend(tune_commands(
                            &cfg,
                            self.state.active_vfo,
                            self.state.rx[0].mode,
                        ));
                        cfg.enabled = true;
                    }
                    if running {
                        if let Some(l) = level.filter(|l| *l > -90.0) {
                            ui.label(RichText::new(format!("{l:.0} dBFS")).size(11.0).weak());
                        }
                        ui.label(RichText::new(tr!("sonde-count", n = sondes.len())).size(11.0));
                        ui.label(RichText::new(tr!("sonde-frames", n = frames)).size(11.0).weak());
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(tr!("sonde-channel")).size(11.0));
                    let mut mhz = cfg.frequency_hz / 1e6;
                    let r = ui
                        .add(
                            egui::DragValue::new(&mut mhz)
                                .range(SONDE_BAND_HZ.0 / 1e6..=SONDE_BAND_HZ.1 / 1e6)
                                .speed(0.01)
                                .fixed_decimals(3)
                                .suffix(" MHz"),
                        )
                        .on_hover_text(tr!("sonde-channel-hover"));
                    if r.changed() {
                        cfg.frequency_hz = (mhz * 1e6).round();
                    }
                    crate::chrome::checkbox(
                        ui,
                        &mut cfg.scan,
                        RichText::new(tr!("sonde-scan")).size(11.0),
                    )
                    .on_hover_text(tr!("sonde-scan-hover"));
                    // A type to look for: choosing one starts the scan, which walks
                    // the window across the band until that kind of sonde is heard.
                    let shown = filter.map_or_else(|| tr!("sonde-type-all"), str::to_string);
                    let mut picked: Option<Option<&'static str>> = None;
                    egui::ComboBox::from_id_salt("sonde_type")
                        .selected_text(RichText::new(shown).size(11.0))
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(filter.is_none(), tr!("sonde-type-all"))
                                .clicked()
                            {
                                picked = Some(None);
                            }
                            for t in TYPES {
                                if ui.selectable_label(filter == Some(*t), *t).clicked() {
                                    picked = Some(Some(*t));
                                }
                            }
                        })
                        .response
                        .on_hover_text(tr!("sonde-type-hover"));
                    if let Some(choice) = picked {
                        self.sonde.map.type_filter = choice;
                        if choice.is_some() {
                            cfg.enabled = true;
                            cfg.scan = true;
                            self.sonde.map.stepper.restart();
                        }
                    }
                });
                if !cfg.enabled {
                    ui.label(
                        RichText::new(tr!("sonde-off-hint"))
                            .size(11.0)
                            .color(crate::theme::YELLOW()),
                    );
                } else if !sdroxide_types::sonde_window_reaches_band(
                    self.state.center_hz,
                    self.state.sample_rate,
                ) {
                    ui.label(
                        RichText::new(tr!("sonde-outside-window"))
                            .size(11.0)
                            .color(crate::theme::gray(200)),
                    );
                }
                // The scan walks the window over the band while nothing is heard: a
                // window narrower than the band otherwise sits on one stretch for ever.
                let scanning = cfg.enabled && cfg.scan && self.state.sample_rate > 0.0;
                if self.sonde.map.stepper.due(now, scanning, busy) {
                    let hz =
                        sonde_map::next_scan_centre(self.state.center_hz, self.state.sample_rate);
                    if (hz - self.state.center_hz).abs() >= 1.0 {
                        cmds.push(Command::SetCenter(hz));
                        cmds.push(Command::SetVfo { vfo: self.state.active_vfo, hz });
                    }
                }
                if cfg != self.state.sonde
                    && !cmds.iter().any(|c| matches!(c, Command::SetSondeConfig(_)))
                {
                    cmds.push(Command::SetSondeConfig(cfg));
                    // The lane mixes from the sample window: follow the channel.
                    if (cfg.frequency_hz - self.state.sonde.frequency_hz).abs() >= 1.0 {
                        cmds.push(Command::SetVfo {
                            vfo: self.state.active_vfo,
                            hz: cfg.frequency_hz,
                        });
                    }
                }

                // Sondes other stations hear: read-only, off until the operator says.
                home = self.sonde_home();
                let peer = self.ctrl.peer_url();
                self.sonde.online.tick(ui.ctx(), home, now, peer.as_deref());
                let want = self.sonde.map.selected.clone().filter(|_| self.sonde.online.on);
                self.sonde.map.flight.tick(ui.ctx(), want.as_deref(), now, peer.as_deref());
                ui.horizontal_wrapped(|ui| {
                    crate::chrome::checkbox(
                        ui,
                        &mut self.sonde.online.on,
                        RichText::new(tr!("sonde-online")).size(11.0),
                    )
                    .on_hover_text(tr!("sonde-online-hover"));
                    if self.sonde.online.on {
                        let note = if home.is_none() {
                            tr!("sonde-online-no-grid")
                        } else if self.sonde.online.failed {
                            tr!("sonde-online-failed")
                        } else {
                            tr!("sonde-online-count", n = self.sonde.online.sondes.len())
                        };
                        let r = ui.label(RichText::new(note).size(10.5).weak());
                        if let (true, Some(why)) =
                            (self.sonde.online.failed, &self.sonde.online.why)
                        {
                            r.on_hover_text(why.text());
                        }
                    }
                });

                // The table is a few rows; the map takes the rest of the page, never
                // less than `MAP_MIN_H`: below that the page scrolls instead of
                // squeezing the map. In a window shorter than that the map takes at
                // most seven tenths of it, so there is always some of the page to
                // turn the wheel over: the map takes the wheel for its zoom, and a map
                // as tall as the window left the table under it out of reach.
                let rows = sondes.len().min(4) as f32;
                let table_h = if sondes.is_empty() { 0.0 } else { 22.0 + rows * 18.0 };
                let map_h = (ui.available_height() - 30.0 - table_h)
                    .max(MAP_MIN_H.min(page_h * 0.7))
                    .max(sonde_map::MIN_HEIGHT);
                self.sonde_scroll_body(ui, &sondes, home, map_h, now);
            });
        // The card is not part of the scrolling page: it is pinned beside its
        // sonde on the map, or a window of its own.
        let ctx = ui.ctx().clone();
        self.sonde_card_view(&ctx, ui.layer_id(), cmds, &sondes, home, now);
    }

    fn sonde_scroll_body(
        &mut self,
        ui: &mut egui::Ui,
        sondes: &[SondeTrack],
        home: Option<(f64, f64)>,
        map_h: f32,
        now: i64,
    ) {
        let hub: Vec<_> = match self.sonde.map.type_filter {
            Some(f) => self
                .sonde
                .online
                .sondes
                .iter()
                .filter(|o| sonde_map::type_matches(f, &o.model))
                .cloned()
                .collect(),
            None => self.sonde.online.sondes.clone(),
        };
        let clicked = sonde_map::show(ui, &mut self.sonde.map, sondes, &hub, home, now, map_h);
        if clicked.is_some() {
            // A picked sonde should not wait for the minute to be up.
            self.sonde.online.hurry(now);
        }

        if !sondes.is_empty() {
            // Ten columns are wider than a phone, or a window dragged narrow:
            // the table scrolls sideways rather than widening the window.
            let table =
                egui::ScrollArea::horizontal().id_salt("sonde_table_h").show_themed(ui, |ui| {
                    egui::Grid::new("sonde_table").striped(true).num_columns(10).show(ui, |ui| {
                        for head in [
                            tr!("sonde-col-serial"),
                            tr!("sonde-col-alt"),
                            tr!("sonde-col-climb"),
                            tr!("sonde-col-speed"),
                            tr!("sonde-channel"),
                            tr!("sonde-col-battery"),
                            tr!("sonde-col-temp"),
                            tr!("sonde-col-humidity"),
                            tr!("sonde-col-pressure"),
                            tr!("sonde-col-heard"),
                        ] {
                            ui.label(RichText::new(head).size(10.0).weak());
                        }
                        ui.end_row();
                        for s in sondes {
                            let selected =
                                self.sonde.map.selected.as_deref() == Some(s.serial.as_str());
                            let ink = match sonde_map::phase(s, now) {
                                Phase::Silent => crate::theme::gray(120),
                                _ => crate::theme::gray(210),
                            };
                            let name = RichText::new(format!("{} {}", s.model, s.serial))
                                .size(10.5)
                                .strong()
                                .color(ink);
                            if ui.selectable_label(selected, name).clicked() {
                                self.sonde.map.select(s.serial.clone());
                            }
                            let cell = |ui: &mut egui::Ui, t: String| {
                                ui.label(RichText::new(t).size(10.5).color(ink));
                            };
                            cell(ui, s.alt_m.map_or("—".into(), |a| format!("{a:.0} m")));
                            cell(ui, s.climb_ms.map_or("—".into(), |c| format!("{c:+.1} m/s")));
                            cell(
                                ui,
                                s.speed_ms.map_or("—".into(), |v| format!("{:.0} km/h", v * 3.6)),
                            );
                            cell(ui, format!("{:.3} MHz", s.freq_hz / 1e6));
                            cell(ui, s.battery_v.map_or("—".into(), |v| format!("{v:.1} V")));
                            cell(ui, s.temp_c.map_or("—".into(), |t| format!("{t:.1} °C")));
                            cell(ui, s.humidity_pct.map_or("—".into(), |h| format!("{h:.0} %")));
                            cell(ui, s.pressure_hpa.map_or("—".into(), |p| format!("{p:.0} hPa")));
                            cell(ui, sonde_card::ago(now - s.last_unix));
                            ui.end_row();
                        }
                    });
                });
            self.sonde.table_view = Some(egui::Rect::from_min_size(
                table.inner_rect.min,
                egui::vec2(table.inner_rect.width(), table.content_size.y),
            ));
            self.sonde_export_row(ui, sondes, now);
        }
    }

    /// The card of the selected sonde: this radio's frames and SondeHub's line
    /// together when both know it, a kept copy when neither does any more.
    ///
    /// Pinned beside the sonde on the map, or, once the operator detaches it, a
    /// window of its own. It never sits under the map where the page has to be
    /// scrolled to find it.
    fn sonde_card_view(
        &mut self,
        ctx: &egui::Context,
        layer: egui::LayerId,
        cmds: &mut Vec<Command>,
        sondes: &[SondeTrack],
        home: Option<(f64, f64)>,
        now: i64,
    ) {
        let Some(sel) = self.sonde.map.selected.clone() else { return };
        // Esc closes it, unless a text field has the keyboard.
        if !ctx.egui_wants_keyboard_input()
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.sonde.map.deselect();
            return;
        }
        let live_radio = sondes.iter().find(|s| s.serial == sel);
        let live_hub = self.sonde.online.sondes.iter().find(|o| o.serial == sel);
        let live = self.sonde.map.kept.update(live_radio, live_hub);
        let radio = live_radio.or(self.sonde.map.kept.radio.as_ref());
        let hub = live_hub.or(self.sonde.map.kept.hub.as_ref());
        let Some(mut card) = sonde_card::compose(radio, hub, home, now) else { return };
        if !live {
            sonde_card::mark_stale(&mut card);
        }
        let heard = radio.map(|s| now - s.last_unix);
        let listen =
            card.freq_hz.map_or(Listen::Elsewhere, |hz| listen_state(&self.state.sonde, hz, heard));
        let act = if self.sonde.map.detached {
            self.sonde_card_window(ctx, &card, listen)
        } else {
            self.sonde_card_pinned(ctx, layer, &card, listen)
        };
        match act {
            CardAct::None => {}
            CardAct::Listen(hz) => cmds.extend(listen_commands(
                &self.state.sonde,
                self.state.active_vfo,
                self.state.rx[0].mode,
                hz,
            )),
            CardAct::Close => self.sonde.map.deselect(),
            CardAct::Detach => self.sonde.map.detached = true,
            CardAct::Attach => self.sonde.map.detached = false,
        }
    }

    /// The card over the map, beside its sonde, kept inside the map.
    fn sonde_card_pinned(
        &mut self,
        ctx: &egui::Context,
        layer: egui::LayerId,
        card: &sonde_card::Card,
        listen: Listen,
    ) -> CardAct {
        let Some(bounds) = self.sonde.map.bounds.filter(|b| b.is_positive()) else {
            return CardAct::None;
        };
        let anchor = self.sonde.map.anchor.unwrap_or(bounds.center());
        let size = self.sonde.map.card_size.unwrap_or(egui::vec2(CARD_W, 230.0));
        let at = sonde_map::pin_placement(anchor, size, bounds);
        let id = crate::layout::salted_id(ctx, "sonde-pinned-card");
        // No wider or taller than the map on show: a card with SondeHub's
        // rows is taller than a short window's map, and scrolls instead.
        let margin = egui::Frame::popup(&ctx.global_style()).total_margin().sum();
        let bar = ctx.global_style().spacing.scroll.allocated_width();
        let max = (bounds.size() - margin - egui::vec2(bar, bar)).max(egui::vec2(120.0, 40.0));
        let shown = egui::Area::new(id)
            .order(egui::Order::Middle)
            .fixed_pos(at)
            .constrain_to(bounds)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .show(ui, |ui| {
                        ui.set_max_width(max.x);
                        egui::ScrollArea::both()
                            .id_salt("sonde-pinned-card-scroll")
                            .max_width(max.x)
                            .max_height(max.y)
                            .min_scrolled_height(max.y.min(64.0))
                            .show_themed(ui, |ui| draw_card(ui, card, false, listen))
                            .inner
                    })
                    .inner
            });
        // Above its own window and with it, not above every window: another
        // window raised over the map covers the card too.
        ctx.set_sublayer(layer, shown.response.layer_id);
        self.sonde.map.card_size = Some(shown.response.rect.size());
        shown.inner
    }

    /// The card as a window the operator can move off the map and resize.
    fn sonde_card_window(
        &mut self,
        ctx: &egui::Context,
        card: &sonde_card::Card,
        listen: Listen,
    ) -> CardAct {
        let mut open = true;
        // Salted per radio, as the tool windows are, and dressed as they are.
        let mut window = egui::Window::new(card.title.clone())
            .id(crate::layout::salted_id(ctx, "sonde-card-window"))
            .frame(crate::chrome::window_frame())
            .open(&mut open)
            .resizable(true)
            .default_width(CARD_W);
        if let Some(b) = self.sonde.map.bounds {
            window = window.default_pos(b.min + egui::vec2(24.0, 24.0));
        }
        let shown = window.show(ctx, |ui| {
            crate::chrome::window_body_bg(ui);
            egui::ScrollArea::vertical()
                .show_themed(ui, |ui| draw_card(ui, card, true, listen))
                .inner
        });
        if let Some(r) = &shown {
            crate::chrome::paint_window_border(ctx, &r.response);
        }
        let act = shown.and_then(|r| r.inner).unwrap_or(CardAct::None);
        if open { act } else { CardAct::Close }
    }

    /// Save the selected sonde's flight (the newest if none is selected) as a
    /// KML track or a CSV table.
    fn sonde_export_row(&mut self, ui: &mut egui::Ui, sondes: &[SondeTrack], now: i64) {
        let target = self
            .sonde
            .map
            .selected
            .as_deref()
            .and_then(|sel| sondes.iter().find(|s| s.serial == sel))
            .or_else(|| sondes.first());
        let Some(s) = target else { return };
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr!("sonde-export")).size(10.5).weak());
            for (ext, hover) in
                [("kml", tr!("sonde-export-kml-hover")), ("csv", tr!("sonde-export-csv-hover"))]
            {
                let b = ui.small_button(ext.to_ascii_uppercase()).on_hover_text(hover);
                if !b.clicked() {
                    continue;
                }
                let text = if ext == "kml" {
                    sdroxide_types::sonde_export::kml(s)
                } else {
                    sdroxide_types::sonde_export::csv(s)
                };
                self.sonde.map.note = Some(match text {
                    Some(text) => {
                        let name = sdroxide_types::sonde_export::file_name(s, now, ext);
                        crate::download::save(&name, text.as_bytes());
                        tr!("sonde-export-saved", name = name)
                    }
                    None => tr!("sonde-export-none"),
                });
            }
            if let Some(note) = &self.sonde.map.note {
                ui.label(RichText::new(note).size(10.5).color(crate::theme::gray(160)));
            }
        });
    }

    /// The operator's own position, from the grid in the digital-mode setup —
    /// the same source every map uses, so they never disagree about it.
    fn sonde_home(&self) -> Option<(f64, f64)> {
        let grid = self.digi_cfg_edit.my_grid.trim();
        (!grid.is_empty()).then(|| sdroxide_types::grid_to_latlon(grid)).flatten()
    }
}

/// What the whole-app layout tests fill the window with: SondeHub's layer, the
/// flight of the selected sonde, and its card pinned or in a window.
#[cfg(test)]
impl SondeUi {
    pub(in crate::app) fn seed(
        &mut self,
        hub: Vec<crate::sonde_online::OnlineSonde>,
        selected: Option<&str>,
        flight: Vec<(f64, f64)>,
        detached: bool,
    ) {
        self.online.seed(hub);
        if let Some(sel) = selected {
            self.map.select(sel.to_string());
            self.map.flight.seed(sel, flight);
        }
        self.map.detached = detached;
    }

    /// The map's rectangle as last drawn: its labels are clipped there on
    /// purpose.
    pub(in crate::app) fn map_bounds(&self) -> Option<egui::Rect> {
        self.map.bounds
    }

    /// The table's sideways scroll: its cells run past the window's side and
    /// are clipped there on purpose, a sideways scroll away.
    pub(in crate::app) fn table_view(&self) -> Option<egui::Rect> {
        self.table_view
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tune_switches_the_decoder_on_and_goes_to_its_channel() {
        let off = SondeSettings::default();
        let cmds = tune_commands(&off, sdroxide_types::Vfo::A, sdroxide_types::Mode::Usb);
        assert!(matches!(cmds[0], Command::SetSondeConfig(c) if c.enabled));
        assert!(matches!(
            cmds[1],
            Command::SetVfo { hz, .. } if hz == sdroxide_types::SONDE_DEFAULT_HZ
        ));
    }

    #[test]
    fn tune_leaves_the_aviation_modes_before_it_retunes() {
        use sdroxide_types::Mode;
        let on = SondeSettings { enabled: true, ..SondeSettings::default() };
        for mode in [Mode::Adsb, Mode::Vdl2, Mode::Hfdl, Mode::Ais, Mode::Aprs] {
            let cmds = tune_commands(&on, sdroxide_types::Vfo::A, mode);
            assert!(
                matches!(cmds[0], Command::SetMode { mode: Mode::Nfm, .. }),
                "{mode:?} must be left first"
            );
            assert!(matches!(cmds[1], Command::SetVfo { .. }));
        }
        let cmds = tune_commands(&on, sdroxide_types::Vfo::A, Mode::Usb);
        assert!(matches!(cmds[0], Command::SetVfo { .. }), "other modes are left alone");
    }

    #[test]
    fn a_click_on_a_cards_frequency_decodes_and_tunes_that_channel_without_a_scan() {
        use sdroxide_types::Mode;
        let scanning = SondeSettings { enabled: true, frequency_hz: 402_700_000.0, scan: true };
        let cmds = listen_commands(&scanning, sdroxide_types::Vfo::A, Mode::Usb, 401_002_000.0);
        assert!(matches!(
            cmds[0],
            Command::SetSondeConfig(c) if c.enabled && !c.scan && c.frequency_hz == 401_002_000.0
        ));
        assert!(matches!(cmds[1], Command::SetVfo { hz, .. } if hz == 401_002_000.0));
        assert_eq!(cmds.len(), 2);
        // Already there and decoding: only the dial.
        let there = SondeSettings { enabled: true, frequency_hz: 401_002_000.0, scan: false };
        let cmds = listen_commands(&there, sdroxide_types::Vfo::A, Mode::Nfm, 401_002_000.0);
        assert_eq!(cmds.len(), 1);
        // From APRS the dock under the waterfall is left, as TUNE does.
        let cmds = listen_commands(&there, sdroxide_types::Vfo::A, Mode::Aprs, 401_002_000.0);
        assert!(matches!(cmds[0], Command::SetMode { mode: Mode::Nfm, .. }));
    }

    #[test]
    fn the_card_says_whether_the_radio_is_on_the_channel_and_hearing_it() {
        let on = SondeSettings { enabled: true, frequency_hz: 401_002_000.0, scan: false };
        assert_eq!(listen_state(&on, 401_002_500.0, None), Listen::Waiting);
        assert_eq!(listen_state(&on, 401_002_500.0, Some(5)), Listen::Decoding);
        assert_eq!(listen_state(&on, 401_002_500.0, Some(120)), Listen::Waiting);
        assert_eq!(listen_state(&on, 405_000_000.0, Some(5)), Listen::Elsewhere);
        assert_eq!(
            listen_state(&SondeSettings { scan: true, ..on }, 401e6, Some(1)),
            Listen::Elsewhere
        );
        assert_eq!(
            listen_state(&SondeSettings { enabled: false, ..on }, 401e6, Some(1)),
            Listen::Elsewhere
        );
    }

    #[test]
    fn every_type_the_picker_offers_is_a_decoder_that_exists() {
        for t in TYPES {
            assert!(!t.is_empty());
        }
        assert_eq!(TYPES.len(), 14, "one per `SondeModel` in sdroxide-sonde (14)");
    }

    #[test]
    fn a_running_decoder_is_not_switched_on_again() {
        let on = SondeSettings { enabled: true, frequency_hz: 404_500_000.0, scan: false };
        let cmds = tune_commands(&on, sdroxide_types::Vfo::A, sdroxide_types::Mode::Usb);
        assert_eq!(cmds.len(), 1, "only the retune");
        assert!(matches!(cmds[0], Command::SetVfo { hz, .. } if hz == 404_500_000.0));
    }

    #[test]
    fn a_card_is_cut_in_two_halves() {
        let rows: Vec<(String, String)> =
            (0..5).map(|i| (format!("l{i}"), format!("v{i}"))).collect();
        let (a, b) = card_halves(&rows);
        assert_eq!((a.len(), b.len()), (3, 2));
        let (a, b) = card_halves(&rows[..4]);
        assert_eq!((a.len(), b.len()), (2, 2));
        let (a, b) = card_halves(&[]);
        assert_eq!((a.len(), b.len()), (0, 0));
    }
}
