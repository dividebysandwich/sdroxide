//! The Scanner window: what to scan, how loud a signal has to be to stop it,
//! and what to do once it has stopped.

use eframe::egui::{self, RichText};
use sdroxide_types::{
    Command, Mode, SCAN_STEPS_HZ, SQUELCH_OPEN_DB, ScanKind, ScanResume, ScannerConfig,
};

use crate::app::SdroxideApp;
use crate::chrome::StyledCombo;

impl SdroxideApp {
    pub(in crate::app) fn scanner_window(&mut self, ctx: &egui::Context, cmds: &mut Vec<Command>) {
        if !self.show_scanner {
            return;
        }
        let mut open = self.show_scanner;
        // Edited in place and sent whole on any change, the way the skimmer's
        // settings are; the engine persists it and echoes it back.
        let mut cfg = self.scanner.clone();
        let resp = egui::Window::new("Scanner")
            .id(crate::layout::salted_id(ctx, "Scanner"))
            .open(&mut open)
            .frame(crate::chrome::window_frame())
            .resizable(true)
            .default_width(crate::layout::window_w(ctx, 420.0))
            .show(ctx, |ui| {
                crate::chrome::window_body_bg(ui);
                self.scanner_body(ui, &mut cfg, cmds)
            });
        if let Some(r) = &resp {
            crate::chrome::paint_window_border(ctx, &r.response);
        }
        if cfg != self.scanner {
            self.scanner = cfg.clone();
            cmds.push(Command::SetScannerConfig(cfg));
        }
        self.show_scanner = open;
    }

    fn scanner_body(&self, ui: &mut egui::Ui, cfg: &mut ScannerConfig, cmds: &mut Vec<Command>) {
        let scan = self.state.scan;

        // What to scan.
        ui.horizontal(|ui| {
            for kind in ScanKind::ALL {
                if crate::chrome::chip(ui, cfg.kind == kind, kind.label())
                    .on_hover_text(match kind {
                        ScanKind::Memories => "Work through the stored memory channels",
                        ScanKind::Range => "Work through a slice of a band on a channel grid",
                    })
                    .clicked()
                {
                    cfg.kind = kind;
                }
            }
            crate::chrome::row_tail(ui, |ui| {
                let label = if scan.running { "STOP" } else { "START" };
                if crate::chrome::chip_accent(
                    ui,
                    scan.running,
                    label,
                    crate::theme::GREEN(),
                    crate::theme::INK_ON_BRIGHT(),
                )
                .clicked()
                {
                    cmds.push(Command::SetScanning(!scan.running));
                }
            });
        });
        ui.separator();

        match cfg.kind {
            ScanKind::Range => self.scanner_range(ui, cfg),
            ScanKind::Memories => self.scanner_memories(ui, cfg),
        }

        ui.separator();
        self.scanner_thresholds(ui, cfg);
        ui.separator();
        self.scanner_status(ui, cmds);
    }

    fn scanner_range(&self, ui: &mut egui::Ui, cfg: &mut ScannerConfig) {
        // Broadcast bands as one-click ranges: a listener walks 49 m, not a
        // pair of typed frequencies. The metre table is the schedule's, so the
        // band named here is the band named there. LW and MW lead it.
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Broadcast band").weak());
            let mut preset = |ui: &mut egui::Ui, name: &str, lo: f64, hi: f64| {
                let on = (cfg.range_lo_hz - lo).abs() < 1.0 && (cfg.range_hi_hz - hi).abs() < 1.0;
                if crate::chrome::chip(ui, on, name)
                    .on_hover_text(format!("Scan {name} ({:.0}-{:.0} kHz)", lo / 1e3, hi / 1e3))
                    .clicked()
                {
                    cfg.range_lo_hz = lo;
                    cfg.range_hi_hz = hi;
                    cfg.mode = Mode::Am;
                }
            };
            preset(ui, "LW", 148_500.0, 283_500.0);
            preset(ui, "MW", 526_500.0, 1_606_500.0);
            for &(name, lo, hi) in sdroxide_types::broadcast::METRE_BANDS {
                preset(ui, name, lo * 1e3, hi * 1e3);
            }
        });
        ui.add_space(4.0);
        egui::Grid::new("scan-range").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            ui.label("From");
            ui.horizontal(|ui| {
                mhz_edit(ui, &mut cfg.range_lo_hz);
                ui.label("to");
                mhz_edit(ui, &mut cfg.range_hi_hz);
                ui.label(RichText::new("MHz").weak());
            });
            ui.end_row();

            ui.label("Step");
            ui.horizontal(|ui| {
                for step in SCAN_STEPS_HZ {
                    let label = if step >= 10_000.0 {
                        format!("{:.0}k", step / 1000.0)
                    } else {
                        format!("{:.2}k", step / 1000.0)
                    };
                    if crate::chrome::chip(ui, (cfg.step_hz - step).abs() < 1.0, label).clicked() {
                        cfg.step_hz = step;
                    }
                }
            });
            ui.end_row();

            ui.label("Mode");
            egui::ComboBox::from_id_salt("scan-mode").selected_text(cfg.mode.label()).show_styled(
                ui,
                |ui| {
                    // The modes anyone scans in. A range scan sets one mode for
                    // the whole range; a memory scan takes each channel's own.
                    for m in [Mode::Nfm, Mode::Am, Mode::Wfm, Mode::Usb, Mode::Lsb] {
                        ui.selectable_value(&mut cfg.mode, m, m.label());
                    }
                },
            );
            ui.end_row();
        });
        if !cfg.range_is_usable() {
            ui.label(
                RichText::new(
                    "That range is empty — the high edge has to be at least one \
                               channel above the low one.",
                )
                .color(crate::theme::ALERT()),
            );
        }
        // Done after the edits above, so retuning the range takes its skipped
        // channels off the screen at once rather than a round trip later.
        cfg.forget_stale_skips();
        self.scanner_range_skips(ui, cfg);
    }

    /// The channels a range scan has been told to pass over, and the way back.
    ///
    /// Shown rather than merely remembered: a scanner that silently refuses to
    /// stop somewhere is indistinguishable from a scanner that cannot hear it,
    /// and the operator who pressed SKIP three passes ago is exactly the person
    /// who will wonder why the repeater never comes up.
    fn scanner_range_skips(&self, ui: &mut egui::Ui, cfg: &mut ScannerConfig) {
        if cfg.skip_freq_hz.is_empty() {
            ui.label(
                RichText::new("SKIP while it is holding to pass over that channel from now on")
                    .size(10.0)
                    .weak(),
            );
            return;
        }
        let listed: Vec<f64> = cfg.skip_freq_hz.clone();
        let mut drop_at: Option<usize> = None;
        let mut clear = false;
        ui.horizontal(|ui| {
            ui.label(RichText::new("Skipping").size(10.5).weak());
            if crate::chrome::chip(ui, false, "CLEAR")
                .on_hover_text("Stop passing over any of them")
                .clicked()
            {
                clear = true;
            }
        });
        // Bounded: an evening spent dismissing a busy band's pagers and data
        // channels runs to dozens of them, and they must not push the rest of
        // the window off the bottom.
        egui::ScrollArea::vertical().id_salt("scan-skips").max_height(72.0).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (i, f) in listed.iter().enumerate() {
                    if crate::chrome::chip(ui, true, format!("{:.4}", f / 1e6))
                        .on_hover_text("Stop passing over this channel")
                        .clicked()
                    {
                        drop_at = Some(i);
                    }
                }
            });
        });
        if clear {
            cfg.skip_freq_hz.clear();
        } else if let Some(i) = drop_at {
            cfg.skip_freq_hz.remove(i);
        }
    }

    fn scanner_memories(&self, ui: &mut egui::Ui, cfg: &mut ScannerConfig) {
        if self.memories.is_empty() {
            ui.label(
                RichText::new("No memory channels stored yet — store some in the MEM window.")
                    .weak(),
            );
            return;
        }
        self.scanner_folders(ui, cfg);
        self.scanner_fast(ui, cfg);
        ui.label(RichText::new("SKIP a channel to pass over it").size(10.0).weak());
        // Resolved before the loop: the rows borrow `cfg` mutably to toggle a
        // skip, so the filter cannot still be holding it.
        let listed: Vec<&sdroxide_types::MemoryChannel> =
            self.memories.iter().filter(|m| cfg.scans_folder(self.filed_under(m))).collect();
        egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
            egui::Grid::new("scan-mems").num_columns(3).spacing([8.0, 4.0]).show(ui, |ui| {
                for m in listed {
                    let skipped = cfg.skip.contains(&m.id);
                    if crate::chrome::chip(ui, skipped, "SKIP").clicked() {
                        if skipped {
                            cfg.skip.retain(|&id| id != m.id);
                        } else {
                            cfg.skip.push(m.id);
                        }
                    }
                    let name =
                        if m.name.is_empty() { format!("#{}", m.id) } else { m.name.clone() };
                    let text = RichText::new(name);
                    ui.label(if skipped { text.weak() } else { text });
                    ui.label(
                        RichText::new(format!("{:.6} MHz  {}", m.freq_hz / 1e6, m.mode.label()))
                            .weak(),
                    );
                    ui.end_row();
                }
            });
        });
    }

    /// The FAST switch: read the channels off the wideband spectrum instead of
    /// visiting each one (issue #228).
    ///
    /// Greyed rather than hidden on a front end that has no span to search — a
    /// CAT rig on a sound card — for the reason every greyed control here is:
    /// a row that comes and goes with the radio is a row nobody can find
    /// twice, and what it says is still true, it just cannot be had *here*.
    fn scanner_fast(&self, ui: &mut egui::Ui, cfg: &mut ScannerConfig) {
        // The same test the engine makes: a demod-audio front end delivers no
        // spectrum to read the channels off.
        let can_sweep = !self.caps.as_ref().is_some_and(|c| c.audio_mode);
        ui.horizontal_wrapped(|ui| {
            let hint = if can_sweep {
                "Look for all the channels that fall inside one receiver window on the same \
                 transform the panadapter is made from, and only tune to the ones something is \
                 on. A list on one band then costs one tune a lap however long it is, instead \
                 of a settling time per channel. The scan still listens on each candidate \
                 before stopping, so what stops it is unchanged — but the sweep measures \
                 through the FFT rather than through the receiver's filter, so check the \
                 threshold if it starts stopping on nothing."
            } else {
                "This radio hands over demodulated audio and has no spectrum of its own to \
                 search, so its memory scan visits every channel either way."
            };
            let resp = ui.add_enabled_ui(can_sweep, |ui| {
                crate::chrome::chip(ui, cfg.mem_fast && can_sweep, "FAST")
            });
            if resp.inner.on_hover_text(hint).clicked() {
                cfg.mem_fast = !cfg.mem_fast;
            }
            ui.label(
                RichText::new("read the list off the spectrum instead of visiting every channel")
                    .size(10.0)
                    .weak(),
            );
        });
    }

    /// Which folder a memory reads as being in — `None` for the top level, and
    /// for one whose folder has gone from under it, exactly as the memory
    /// window lists it and as the engine's own scan filter resolves it.
    fn filed_under(&self, m: &sdroxide_types::MemoryChannel) -> Option<u32> {
        m.folder.filter(|id| self.mem_folders.iter().any(|f| f.id == *id))
    }

    /// Which folders the memory scan runs over: **ALL**, then one chip per
    /// folder, then the unfiled channels (issue #236).
    ///
    /// Drawn only where there is a folder to choose — with everything at the
    /// top level there is one place for a channel to be, and a row offering it
    /// is a control that can only be set wrong.
    ///
    /// ALL is not "every chip lit": it is an empty selection, which is also
    /// what a folder made tomorrow falls into. Lighting every folder instead
    /// would quietly leave the next one out of the scan.
    fn scanner_folders(&self, ui: &mut egui::Ui, cfg: &mut ScannerConfig) {
        if self.mem_folders.is_empty() {
            return;
        }
        // The counts are what makes the row readable: a folder with nothing in
        // it, or one whose every channel is skipped, is worth seeing before the
        // scan says it has nothing to visit.
        let count = |folder: Option<u32>| {
            self.memories.iter().filter(|m| self.filed_under(m) == folder).count()
        };
        let unfiled = count(None);
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Folders").size(10.5).weak());
            if crate::chrome::chip(ui, cfg.folders.is_empty(), "ALL")
                .on_hover_text(
                    "Scan every folder, and every folder made from now on. Pick folders \
                     instead to scan only those.",
                )
                .clicked()
            {
                cfg.folders.clear();
            }
            let mut toggle = |ui: &mut egui::Ui, which: Option<u32>, label: String, n: usize| {
                let on = cfg.folders.contains(&which);
                let text = RichText::new(format!("{label} ({n})")).size(11.0);
                if crate::chrome::chip(ui, on, text).clicked() {
                    if on {
                        cfg.folders.retain(|f| *f != which);
                    } else {
                        cfg.folders.push(which);
                    }
                }
            };
            for f in &self.mem_folders {
                toggle(ui, Some(f.id), f.name.clone(), count(Some(f.id)));
            }
            if unfiled > 0 {
                toggle(ui, None, "Unfiled".to_string(), unfiled);
            }
        });
        if !cfg.folders.is_empty()
            && !self.memories.iter().any(|m| cfg.scans_folder(self.filed_under(m)))
        {
            ui.label(
                RichText::new("Nothing is filed under the folders you picked.")
                    .color(crate::theme::ALERT()),
            );
        }
    }

    fn scanner_thresholds(&self, ui: &mut egui::Ui, cfg: &mut ScannerConfig) {
        egui::Grid::new("scan-levels").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            ui.label("Stops at");
            ui.horizontal(|ui| {
                if crate::chrome::chip(ui, cfg.follow_squelch, "SQL")
                    .on_hover_text(
                        "Use the receiver's own squelch, so the scan stops exactly where the \
                         audio would open",
                    )
                    .clicked()
                {
                    cfg.follow_squelch = !cfg.follow_squelch;
                }
                if cfg.follow_squelch {
                    let sql = self.state.rx[0].squelch_db;
                    ui.label(
                        RichText::new(if sql <= SQUELCH_OPEN_DB + 1.0 {
                            "squelch is off — the scan will stop on the first thing it looks at"
                                .to_string()
                        } else {
                            format!("{sql:.0} dBFS")
                        })
                        .weak(),
                    );
                } else {
                    ui.add(
                        egui::DragValue::new(&mut cfg.threshold_db)
                            .speed(0.5)
                            .range(-140.0..=-10.0)
                            .suffix(" dBFS"),
                    )
                    .on_hover_text("Channel power a signal has to reach to stop the scan");
                }
            });
            ui.end_row();

            ui.label("Listens for");
            ui.add(
                egui::DragValue::new(&mut cfg.dwell_ms).speed(5.0).range(40..=2000).suffix(" ms"),
            )
            .on_hover_text(
                "How long to stay on a candidate before judging it. Below about a tenth of a \
                 second the level meter has not settled and weak signals get missed",
            );
            ui.end_row();

            ui.label("Resumes");
            ui.horizontal(|ui| {
                for r in ScanResume::ALL {
                    if crate::chrome::chip(ui, cfg.resume == r, r.label())
                        .on_hover_text(match r {
                            ScanResume::Carrier => "Carry on once the signal drops",
                            ScanResume::Timed => "Carry on after a fixed time, regardless",
                            ScanResume::Manual => "Stay until you press NEXT",
                        })
                        .clicked()
                    {
                        cfg.resume = r;
                    }
                }
                if cfg.resume != ScanResume::Manual {
                    ui.add(
                        egui::DragValue::new(&mut cfg.resume_ms)
                            .speed(50.0)
                            .range(200..=60_000)
                            .suffix(" ms"),
                    )
                    .on_hover_text(match cfg.resume {
                        ScanResume::Timed => "How long to stay",
                        // Long enough to ride out the gap between overs, or the
                        // scan leaves in the middle of a conversation.
                        _ => "How long to wait after the signal drops",
                    });
                }
            });
            ui.end_row();
        });
    }

    fn scanner_status(&self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let scan = self.state.scan;
        ui.horizontal(|ui| {
            if !scan.running {
                ui.label(RichText::new("stopped").weak());
                return;
            }
            let here = self.state.active_freq_hz() / 1e6;
            let (text, colour) = if scan.holding {
                (format!("holding {here:.6} MHz"), crate::theme::GREEN())
            } else {
                (format!("scanning · {here:.6} MHz"), crate::theme::CYAN())
            };
            ui.label(RichText::new(text).color(colour).strong());
            // Name what it stopped on, from the broadcast table, so a listener
            // does not have to know 6.185 MHz is Taiwan.
            if scan.holding
                && let Some(st) = sdroxide_types::broadcast::at_dial(
                    &self.broadcast,
                    self.state.active_freq_hz(),
                    crate::time::now_unix(),
                )
            {
                ui.label(
                    RichText::new(format!("· {}", st.name))
                        .color(crate::theme::TEXT())
                        .strong(),
                );
            }
            crate::chrome::row_tail(ui, |ui| {
                if crate::chrome::chip(ui, false, "SKIP")
                    .on_hover_text("Move on, and don't stop on this channel again")
                    .clicked()
                {
                    cmds.push(Command::ScanSkip);
                }
                if crate::chrome::chip(ui, false, "NEXT").on_hover_text("Move on now").clicked() {
                    cmds.push(Command::ScanNext);
                }
            });
        });
    }
}

/// A frequency box in MHz, which is how an operator says one.
fn mhz_edit(ui: &mut egui::Ui, hz: &mut f64) {
    let mut mhz = *hz / 1e6;
    if ui
        .add(
            egui::DragValue::new(&mut mhz)
                .speed(0.01)
                .range(0.0..=6000.0)
                .max_decimals(6)
                .fixed_decimals(4),
        )
        .changed()
    {
        *hz = mhz * 1e6;
    }
}
