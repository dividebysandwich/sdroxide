//! The radiosonde panel: what is in the air, and where.
//!
//! - **SONDES** — every sonde still on the list, with a detail card for the
//!   selected one.
//! - **MAP** — positions on the shared worldmap (home = My grid).
//!
//! Decode runs through rs1729 `*mod` binaries on PATH (OpenWebRX style).

use eframe::egui::{self, RichText};
use sdroxide_types::{Command, Rs41Sonde, Rs41Status, SondeKind};

use crate::app::SdroxideApp;
use crate::app::util::fmt_age;
use crate::theme;

impl SdroxideApp {
    pub(in crate::app) fn rs41_panel(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        panel_h: f32,
    ) {
        let st: Rs41Status = match self.rs41_status.as_ref() {
            Some(s) => (**s).clone(),
            None => {
                ui.label(RichText::new("starting the sonde decoder…").weak());
                return;
            }
        };
        let now = crate::time::now_unix();
        let content_bottom = ui.cursor().top() + panel_h - 26.0;

        self.rs41_header(ui, cmds, &st);
        ui.add_space(3.0);

        let avail_h = (content_bottom - ui.cursor().top()).max(80.0);
        let pane = self.phone_pane(ui, self.state.rx[0].mode);
        let full_w = ui.available_width();

        const HANDLE_W: f32 = 7.0;
        let list_w = (full_w * self.view.rs41_split_fraction)
            .clamp(220.0, (full_w - HANDLE_W - 200.0).max(220.0));

        ui.horizontal_top(|ui| {
            if pane.is_none_or(|p| p == 0) {
                ui.allocate_ui_with_layout(
                    egui::vec2(if pane.is_some() { full_w } else { list_w }, avail_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| self.rs41_list(ui, cmds, &st, now, avail_h),
                );
            }
            if pane.is_none() {
                let h = crate::chrome::split_handle(ui, egui::vec2(HANDLE_W, avail_h), None);
                if h.dragged() {
                    self.view.rs41_split_fraction =
                        ((list_w + h.drag_delta().x) / full_w.max(1.0)).clamp(0.15, 0.85);
                }
            }
            if pane.is_none_or(|p| p == 1) {
                ui.vertical(|ui| self.rs41_map_pane(ui, &st, now, avail_h));
            }
        });
    }

    fn rs41_header(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, st: &Rs41Status) {
        let dial = self.state.rx_freq_hz();
        let in_band = dial >= sdroxide_types::RS41_BAND_LO_HZ
            && dial <= sdroxide_types::RS41_BAND_HI_HZ
            && st.unavailable.is_none();

        ui.horizontal_wrapped(|ui| {
            ui.set_min_height(22.0);
            ui.label(RichText::new("SONDE").size(11.0).strong().color(theme::CYAN()));
            ui.label(
                RichText::new(format!("{} · 400–406 MHz", self.state.rs41.kind.label()))
                    .weak()
                    .size(10.5),
            );

            if crate::chrome::chip(ui, in_band, "403.000")
                .on_hover_text(
                    "Tune mid-band (403 MHz). Individual sondes sit elsewhere in \
                     400–406 MHz — click a row to go onto that one.",
                )
                .clicked()
            {
                cmds.push(Command::SetVfo {
                    vfo: self.state.active_vfo,
                    hz: sdroxide_types::RS41_DEFAULT_HZ,
                });
            }

            ui.separator();
            slot(ui, 72.0, &format!("{} sondes", st.sondes.len()), theme::CYAN());
            slot(ui, 74.0, &format!("{} frames", count(st.frames)), theme::gray(150));
            slot(ui, 78.0, &format!("{} bad", count(st.bad_crc)), theme::gray(120));

            if st.window_rate_hz > 0.0 {
                slot(
                    ui,
                    140.0,
                    &format!(
                        "{:.3} MHz / {:.0} kHz",
                        st.window_center_hz / 1e6,
                        st.window_rate_hz / 1e3
                    ),
                    theme::gray(120),
                );
            }

            ui.separator();
            let up = self.net_cfg_edit.sondehub.upload;
            if crate::chrome::chip(ui, up, "UPLOAD")
                .on_hover_text(
                    "Upload radiosonde receptions to SondeHub. Uses My callsign and \
                     My grid from the digimode identity.",
                )
                .clicked()
            {
                self.net_cfg_edit.sondehub.upload = !up;
                cmds.push(Command::SetNetworkConfig(self.net_cfg_edit.clone()));
            }
            if crate::chrome::chip(ui, self.show_rs41_setup, "SETUP")
                .on_hover_text("Sonde type, decoder timeouts, trail length and table size")
                .clicked()
            {
                self.show_rs41_setup = !self.show_rs41_setup;
            }
        });

        if let Some(why) = &st.unavailable {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(why).size(10.5).color(theme::HAZARD()));
                if let Some(hz) = st.suggest_center_hz
                    && (dial - hz).abs() > 1.0
                    && crate::chrome::chip(ui, false, format!("TUNE {:.3}", hz / 1e6)).clicked()
                {
                    cmds.push(Command::SetVfo { vfo: self.state.active_vfo, hz });
                }
            });
        }
    }

    fn rs41_list(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        st: &Rs41Status,
        now: i64,
        avail_h: f32,
    ) {
        ui.horizontal_wrapped(|ui| {
            ui.set_min_height(20.0);
            ui.label(RichText::new("SONDES").strong().size(10.5).color(theme::CYAN()));
            ui.add(
                egui::TextEdit::singleline(&mut self.rs41_filter)
                    .hint_text("filter")
                    .desired_width(70.0),
            );
        });

        let filter = self.rs41_filter.trim().to_ascii_uppercase();
        let mut rows: Vec<&Rs41Sonde> = st
            .sondes
            .iter()
            .filter(|s| filter.is_empty() || s.serial.to_ascii_uppercase().contains(&filter))
            .collect();
        rows.sort_by(|a, b| b.last_at.cmp(&a.last_at));

        let selected = self.rs41_map.selected.clone();
        let card_h = if selected.is_some() {
            (avail_h * self.view.rs41_card_fraction).clamp(90.0, (avail_h - 100.0).max(90.0))
        } else {
            0.0
        };
        let list_h = (avail_h - card_h - if selected.is_some() { 8.0 } else { 0.0 }).max(60.0);

        egui::ScrollArea::vertical()
            .id_salt("rs41-sondes")
            .max_height(list_h)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                for s in &rows {
                    let sel = selected.as_deref() == Some(s.serial.as_str());
                    if rs41_row(ui, s, now, sel) {
                        let serial = s.serial.clone();
                        let hz = s.frequency_hz;
                        if selected.as_deref() == Some(serial.as_str()) {
                            self.rs41_map.selected = None;
                        } else {
                            self.rs41_map.selected = Some(serial);
                            cmds.push(Command::SetVfo { vfo: self.state.active_vfo, hz });
                        }
                    }
                }
            });

        if let Some(serial) = selected
            && let Some(s) = st.sondes.iter().find(|s| s.serial == serial)
        {
            ui.add_space(4.0);
            let h = crate::chrome::split_handle(ui, egui::vec2(ui.available_width(), 6.0), None);
            if h.dragged() {
                self.view.rs41_card_fraction =
                    ((card_h - h.drag_delta().y) / avail_h.max(1.0)).clamp(0.2, 0.7);
            }
            self.rs41_card(ui, s, now, card_h);
        }
    }

    fn rs41_card(&mut self, ui: &mut egui::Ui, s: &Rs41Sonde, now: i64, card_h: f32) {
        // auto_shrink false on X so the card uses the list pane width instead
        // of hugging its text (same trick as the sat list grid).
        egui::ScrollArea::vertical()
            .id_salt("rs41-card")
            .max_height(card_h)
            .auto_shrink([false, false])
            .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new(&s.serial).strong().size(13.0).color(theme::CYAN()));
            let kind_line = match &s.subtype {
                Some(st) if !st.is_empty() => format!("{} ({st})", s.kind.label()),
                _ => s.kind.label().to_string(),
            };
            ui.label(
                RichText::new(format!(
                    "{kind_line} · {:.3} MHz · {} · RSSI {:.0} dBFS",
                    s.frequency_hz / 1e6,
                    fmt_age(now - s.last_at),
                    s.rssi_dbfs
                ))
                .size(10.5)
                .weak(),
            );
            // Two side-by-side key/value columns (four cells per row).
            let fields: [(&str, Option<String>); 12] = [
                ("Type", Some(kind_line)),
                ("Altitude", s.altitude_m.map(|a| format!("{a:.0} m"))),
                ("Ascent", s.ascent_rate_mps.map(|a| format!("{a:+.1} m/s"))),
                ("Speed", s.vel_h_mps.map(|v| format!("{v:.1} m/s"))),
                ("Heading", s.heading_deg.map(|h| format!("{h:.0}°"))),
                ("Temp", s.temperature_c.map(|t| format!("{t:.1} °C"))),
                ("Humidity", s.humidity_pct.map(|h| format!("{h:.0} %"))),
                ("Pressure", s.pressure_hpa.map(|p| format!("{p:.1} hPa"))),
                ("Battery", s.battery_v.map(|b| format!("{b:.2} V"))),
                ("Sats", s.sats.map(|n| format!("{n}"))),
                (
                    "Position",
                    match (s.lat, s.lon) {
                        (Some(lat), Some(lon)) => Some(format!("{lat:.5}, {lon:.5}")),
                        _ => None,
                    },
                ),
                (
                    "Frames",
                    Some(format!(
                        "{}{}",
                        s.frames,
                        s.frame.map(|f| format!(" (#{f})")).unwrap_or_default()
                    )),
                ),
            ];
            const COL_GAP: f32 = 14.0;
            let col_w = ((ui.available_width() - 3.0 * COL_GAP) / 4.0).max(48.0);
            egui::Grid::new("rs41-card-grid")
                .num_columns(4)
                .min_col_width(col_w)
                .spacing([COL_GAP, 1.0])
                .show(ui, |ui| {
                    for pair in fields.chunks(2) {
                        for (key, val) in pair {
                            kv(ui, key, val.as_deref());
                        }
                        if pair.len() == 1 {
                            ui.label("");
                            ui.label("");
                        }
                        ui.end_row();
                    }
                });
        });
    }

    fn rs41_map_pane(&mut self, ui: &mut egui::Ui, st: &Rs41Status, now: i64, h: f32) {
        let home = self.rs41_home();
        let cfg = self.state.rs41;
        if let Some(serial) =
            crate::rs41_map::show(ui, &mut self.rs41_map, &st.sondes, home, now, cfg, h)
        {
            self.rs41_map.selected = Some(serial);
        }
    }

    fn rs41_home(&self) -> Option<(f64, f64)> {
        let grid = self.digi_cfg_edit.my_grid.trim();
        (!grid.is_empty()).then(|| sdroxide_types::grid_to_latlon(grid)).flatten()
    }

    pub(in crate::app) fn rs41_setup_window(
        &mut self,
        ctx: &egui::Context,
        cmds: &mut Vec<Command>,
    ) {
        crate::layout::arm_center_on_open(
            self.show_rs41_setup,
            &mut self.rs41_setup_was_open,
            &mut self.rs41_setup_center_pending,
        );
        if !self.show_rs41_setup {
            return;
        }
        let mut open = self.show_rs41_setup;
        let mut cfg = self.state.rs41;
        let size =
            egui::vec2(crate::layout::window_w(ctx, 420.0), crate::layout::window_h(ctx, 340.0));
        let pending = self.rs41_setup_center_pending;
        let resp = crate::layout::center_on_open(
            egui::Window::new("Sonde Setup")
                .id(crate::layout::salted_id(ctx, "Rs41Setup"))
                .open(&mut open)
                .frame(crate::chrome::window_frame())
                .resizable(false)
                .default_width(size.x),
            ctx,
            pending,
            size,
        )
        .show(ctx, |ui| {
            crate::chrome::window_body_bg(ui);
            egui::Grid::new("rs41-cfg").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label("Type");
                egui::ComboBox::from_id_salt("sonde-kind")
                    .selected_text(cfg.kind.label())
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        for k in SondeKind::ALL {
                            ui.selectable_value(&mut cfg.kind, *k, k.label());
                        }
                    });
                ui.end_row();

                ui.label("Decoder");
                crate::chrome::checkbox(ui, &mut cfg.enabled, "Armed");
                ui.end_row();

                ui.label("Drop from map after");
                ui.add(egui::DragValue::new(&mut cfg.drop_map_s).range(5..=600).suffix(" s"));
                ui.end_row();

                ui.label("Drop from list after");
                ui.add(
                    egui::DragValue::new(&mut cfg.drop_list_s)
                        .range(i64::from(cfg.drop_map_s)..=3600)
                        .suffix(" s"),
                );
                ui.end_row();

                ui.label("Trail length");
                ui.add(
                    egui::DragValue::new(&mut cfg.history_points).range(4..=240).suffix(" points"),
                );
                ui.end_row();

                ui.label("Track at most");
                ui.add(egui::DragValue::new(&mut cfg.max_sondes).range(1..=200).suffix(" sondes"));
                ui.end_row();
            });
            ui.separator();
            ui.label(
                RichText::new(format!(
                    "Needs `{}` beside the sdroxide binary, in SDROXIDE_SONDE_MOD_DIR, \
                     or on PATH (built automatically with `--features sonde`, the \
                     default). SondeHub upload is under Settings → Spots (and the \
                     UPLOAD chip). My callsign and My grid come from the digimode \
                     identity.",
                    cfg.kind.binary()
                ))
                .size(10.0)
                .weak(),
            );
        });
        if let Some(r) = &resp {
            crate::chrome::paint_window_border(ctx, &r.response);
            self.rs41_setup_center_pending = false;
        }
        let cfg = cfg.sane();
        if cfg != self.state.rs41 {
            cmds.push(Command::SetRs41Config(cfg));
        }
        self.show_rs41_setup = open;
    }
}

fn rs41_row(ui: &mut egui::Ui, s: &Rs41Sonde, now: i64, selected: bool) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        let label = format!(
            "{}  {}  {:.3}  {}",
            s.label(),
            s.kind.label(),
            s.frequency_hz / 1e6,
            s.altitude_m.map(|a| format!("{a:.0}m")).unwrap_or_else(|| "——".into())
        );
        let resp = crate::chrome::chip(ui, selected, label);
        if resp.clicked() {
            clicked = true;
        }
        ui.label(RichText::new(fmt_age(now - s.last_at)).size(9.5).weak());
    });
    clicked
}

fn kv(ui: &mut egui::Ui, key: &str, val: Option<&str>) {
    ui.label(RichText::new(key).size(10.5).weak());
    ui.label(RichText::new(val.unwrap_or("——")).size(10.5));
}

fn slot(ui: &mut egui::Ui, w: f32, text: &str, color: egui::Color32) {
    ui.allocate_ui_with_layout(
        egui::vec2(w, 18.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.label(RichText::new(text).size(10.5).color(color));
        },
    );
}

fn count(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1e6)
    } else if n >= 10_000 {
        format!("{:.1}k", n as f64 / 1e3)
    } else {
        n.to_string()
    }
}
