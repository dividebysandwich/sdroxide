//! The Scanner window: what to scan, how loud a signal has to be to stop it,
//! and what to do once it has stopped.

use crate::i18n::{Tr, tr};
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
        let mut clear_strip = false;
        let tool_title = tr!("scanner-scanner");
        let mut tool_st = self.take_tool_state("Scanner");
        let tool_child = crate::app::undocked::ChildWindow::own(self);
        crate::app::undocked::tool_window(
            ctx,
            crate::app::undocked::ToolWindow::new("Scanner", &tool_title, [420.0, 0.0]),
            &mut tool_st,
            tool_child,
            &mut open,
            |ui| self.scanner_body(ui, &mut cfg, cmds, &mut clear_strip),
        );
        if clear_strip {
            self.wide_wf.clear();
            self.wide_frame = None;
        }
        self.put_tool_state("Scanner", tool_st);
        if cfg != self.scanner {
            self.scanner = cfg.clone();
            cmds.push(Command::SetScannerConfig(cfg));
        }
        self.show_scanner = open;
    }

    fn scanner_body(
        &self,
        ui: &mut egui::Ui,
        cfg: &mut ScannerConfig,
        cmds: &mut Vec<Command>,
        clear_strip: &mut bool,
    ) {
        let scan = self.state.scan;

        // What to scan.
        ui.horizontal(|ui| {
            for kind in ScanKind::ALL {
                if crate::chrome::chip(ui, cfg.kind == kind, kind.tr())
                    .on_hover_text(match kind {
                        ScanKind::Memories => tr!("scanner-work-through-stored-memory-channels"),
                        ScanKind::Range => tr!("scanner-work-through-slice-band-channel"),
                        ScanKind::Panorama => tr!("scanner-sweep-range-into-panorama"),
                    })
                    .clicked()
                {
                    cfg.kind = kind;
                }
            }
            crate::chrome::row_tail(ui, |ui| {
                let label = if scan.running { tr!("scanner-stop") } else { tr!("scanner-start") };
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
            ScanKind::Panorama => self.scanner_panorama(ui, cfg),
        }

        ui.separator();
        if cfg.kind == ScanKind::Panorama {
            self.scanner_panorama_settle(ui, cfg);
        } else {
            self.scanner_thresholds(ui, cfg);
        }
        ui.separator();
        self.scanner_status(ui, cfg, cmds, clear_strip);
    }

    fn scanner_range(&self, ui: &mut egui::Ui, cfg: &mut ScannerConfig) {
        egui::Grid::new("scan-range").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            ui.label(tr!("scanner-band"));
            band_pick(ui, cfg);
            ui.end_row();

            ui.label(tr!("scanner-from"));
            ui.horizontal(|ui| {
                mhz_edit(ui, &mut cfg.range_lo_hz);
                ui.label("to");
                mhz_edit(ui, &mut cfg.range_hi_hz);
                ui.label(RichText::new("MHz").weak());
            });
            ui.end_row();

            ui.label(tr!("scanner-step"));
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

            ui.label(tr!("scanner-mode"));
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
                RichText::new(tr!("scanner-range-empty-high-edge-has"))
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
    /// A panorama: only the range. It stops nowhere, so the grid, the mode and
    /// the thresholds a range scan judges channels by do not apply.
    fn scanner_panorama(&self, ui: &mut egui::Ui, cfg: &mut ScannerConfig) {
        egui::Grid::new("scan-pano").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            ui.label(tr!("scanner-band"));
            band_pick(ui, cfg);
            ui.end_row();

            ui.label(tr!("scanner-from"));
            ui.horizontal(|ui| {
                mhz_edit(ui, &mut cfg.range_lo_hz);
                ui.label("to");
                mhz_edit(ui, &mut cfg.range_hi_hz);
                ui.label(RichText::new("MHz").weak());
            });
            ui.end_row();
        });
        if !cfg.range_is_usable() {
            ui.label(
                RichText::new(tr!("scanner-range-empty-high-edge-has"))
                    .color(crate::theme::ALERT()),
            );
        }
        ui.label(RichText::new(tr!("scanner-panorama-hint")).weak());
    }

    /// How long each panorama slice waits for the front end before it is read.
    fn scanner_panorama_settle(&self, ui: &mut egui::Ui, cfg: &mut ScannerConfig) {
        egui::Grid::new("scan-pano-settle").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            ui.label(tr!("scanner-settles"));
            ui.add(
                egui::DragValue::new(&mut cfg.dwell_ms).speed(5.0).range(40..=2000).suffix(" ms"),
            )
            .on_hover_text(tr!("scanner-settles-hint"));
            ui.end_row();
        });
    }

    fn scanner_range_skips(&self, ui: &mut egui::Ui, cfg: &mut ScannerConfig) {
        if cfg.skip_freq_hz.is_empty() {
            ui.label(RichText::new(tr!("scanner-skip-while-holding-pass-over")).size(10.0).weak());
            return;
        }
        let listed: Vec<f64> = cfg.skip_freq_hz.clone();
        let mut drop_at: Option<usize> = None;
        let mut clear = false;
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr!("scanner-skipping")).size(10.5).weak());
            if crate::chrome::chip(ui, false, tr!("scanner-clear"))
                .on_hover_text(tr!("scanner-stop-passing-over-any-them"))
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
                        .on_hover_text(tr!("scanner-stop-passing-over-channel"))
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
            ui.label(RichText::new(tr!("scanner-no-memory-channels-stored-yet")).weak());
            return;
        }
        self.scanner_folders(ui, cfg);
        self.scanner_fast(ui, cfg);
        ui.label(RichText::new(tr!("scanner-skip-channel-pass-over")).size(10.0).weak());
        // Resolved before the loop: the rows borrow `cfg` mutably to toggle a
        // skip, so the filter cannot still be holding it.
        let listed: Vec<&sdroxide_types::MemoryChannel> =
            self.memories.iter().filter(|m| cfg.scans_folder(self.filed_under(m))).collect();
        egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
            egui::Grid::new("scan-mems").num_columns(3).spacing([8.0, 4.0]).show(ui, |ui| {
                for m in listed {
                    let skipped = cfg.skip.contains(&m.id);
                    if crate::chrome::chip(ui, skipped, tr!("scanner-skip")).clicked() {
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
                        RichText::new(format!(
                            "{e6:.6} MHz {mode}",
                            e6 = m.freq_hz / 1e6,
                            mode = m.mode.label()
                        ))
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
                tr!("scanner-look-all-channels-fall-inside")
            } else {
                tr!("scanner-radio-hands-over-demodulated-audio")
            };
            let resp = ui.add_enabled_ui(can_sweep, |ui| {
                crate::chrome::chip(ui, cfg.mem_fast && can_sweep, tr!("scanner-fast"))
            });
            if resp.inner.on_hover_text(hint).clicked() {
                cfg.mem_fast = !cfg.mem_fast;
            }
            ui.label(
                RichText::new(tr!("scanner-read-list-off-spectrum-instead")).size(10.0).weak(),
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
            ui.label(RichText::new(tr!("scanner-folders")).size(10.5).weak());
            if crate::chrome::chip(ui, cfg.folders.is_empty(), "ALL")
                .on_hover_text(tr!("scanner-scan-every-folder-every-folder"))
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
                toggle(ui, None, tr!("scanner-unfiled").to_string(), unfiled);
            }
        });
        if !cfg.folders.is_empty()
            && !self.memories.iter().any(|m| cfg.scans_folder(self.filed_under(m)))
        {
            ui.label(
                RichText::new(tr!("scanner-nothing-filed-under-folders-you"))
                    .color(crate::theme::ALERT()),
            );
        }
    }

    fn scanner_thresholds(&self, ui: &mut egui::Ui, cfg: &mut ScannerConfig) {
        egui::Grid::new("scan-levels").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            ui.label(tr!("scanner-stops"));
            ui.horizontal(|ui| {
                // One of three, as chips: the squelch, a fixed level, or S/N.
                let level = !cfg.follow_squelch && !cfg.stop_snr;
                let snr = !cfg.follow_squelch && cfg.stop_snr;
                if crate::chrome::chip(ui, cfg.follow_squelch, "SQL")
                    .on_hover_text(tr!("scanner-use-receiver-s-own-squelch"))
                    .clicked()
                {
                    cfg.follow_squelch = true;
                    cfg.stop_snr = false;
                }
                if crate::chrome::chip(ui, level, "dBFS")
                    .on_hover_text(tr!("scanner-channel-power-signal-has-reach"))
                    .clicked()
                {
                    cfg.follow_squelch = false;
                    cfg.stop_snr = false;
                }
                if crate::chrome::chip(ui, snr, "S/N")
                    .on_hover_text(tr!("scanner-stop-snr"))
                    .clicked()
                {
                    cfg.follow_squelch = false;
                    cfg.stop_snr = true;
                }
                if cfg.follow_squelch {
                    let sql = self.state.rx[0].squelch_db;
                    ui.label(
                        RichText::new(if sql <= SQUELCH_OPEN_DB + 1.0 {
                            tr!("scanner-squelch-off-scan-will-stop").to_string()
                        } else {
                            format!("{sql:.0} dBFS", sql = sql)
                        })
                        .weak(),
                    );
                } else if cfg.stop_snr {
                    // A slider with its own number box: drag for a feel, type
                    // for a figure.
                    ui.add(
                        egui::Slider::new(&mut cfg.snr_db, 0.0..=40.0).step_by(0.5).suffix(" dB"),
                    )
                    .on_hover_text(tr!("scanner-stop-snr"));
                } else {
                    ui.add(
                        egui::Slider::new(&mut cfg.threshold_db, -140.0..=-10.0)
                            .step_by(0.5)
                            .suffix(" dBFS"),
                    )
                    .on_hover_text(tr!("scanner-channel-power-signal-has-reach"));
                }
            });
            ui.end_row();

            ui.label(tr!("scanner-listens"));
            ui.add(
                egui::DragValue::new(&mut cfg.dwell_ms).speed(5.0).range(40..=2000).suffix(" ms"),
            )
            .on_hover_text(tr!("scanner-how-long-stay-candidate-before"));
            ui.end_row();

            ui.label(tr!("scanner-resumes"));
            ui.horizontal(|ui| {
                for r in ScanResume::ALL {
                    if crate::chrome::chip(ui, cfg.resume == r, r.tr())
                        .on_hover_text(match r {
                            ScanResume::Carrier => tr!("scanner-carry-once-signal-drops"),
                            ScanResume::Timed => tr!("scanner-carry-after-fixed-time-regardless"),
                            ScanResume::Manual => tr!("scanner-stay-until-you-press-next"),
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
                        ScanResume::Timed => tr!("scanner-how-long-stay"),
                        // Long enough to ride out the gap between overs, or the
                        // scan leaves in the middle of a conversation.
                        _ => tr!("scanner-how-long-wait-after-signal"),
                    });
                }
            });
            ui.end_row();
        });
    }

    fn scanner_status(
        &self,
        ui: &mut egui::Ui,
        cfg: &ScannerConfig,
        cmds: &mut Vec<Command>,
        clear_strip: &mut bool,
    ) {
        let scan = self.state.scan;
        let pano = cfg.kind == ScanKind::Panorama;
        ui.horizontal(|ui| {
            if !scan.running {
                ui.label(RichText::new("stopped").weak());
            } else if pano {
                // A panorama stops nowhere: what it covers and how far it has
                // got say more than the dial it happens to be passing.
                let (lo, hi) = cfg.range();
                let range = format!("{:.3}–{:.3}", lo / 1e6, hi / 1e6);
                let text = if scan.sweeps == 0 {
                    tr!("scanner-pano-first", range = range)
                } else {
                    tr!(
                        "scanner-pano-sweeps",
                        range = range,
                        n = scan.sweeps,
                        secs = format!("{:.1}", scan.sweep_ms as f64 / 1000.0)
                    )
                };
                ui.label(RichText::new(text).color(crate::theme::CYAN()).strong());
            } else {
                let here = self.state.active_freq_hz() / 1e6;
                let (text, colour) = if scan.holding {
                    (
                        tr!("scanner-holding-mhz", here = format!("{:.6}", here)),
                        crate::theme::GREEN(),
                    )
                } else {
                    (
                        tr!("scanner-scanning-mhz", here = format!("{:.6}", here)),
                        crate::theme::CYAN(),
                    )
                };
                ui.label(RichText::new(text).color(colour).strong());
            }
            crate::chrome::row_tail(ui, |ui| {
                if pano {
                    // Throw the picture away and start the sweep afresh.
                    if crate::chrome::chip(ui, false, "CLR")
                        .on_hover_text(tr!("scanner-pano-clear"))
                        .clicked()
                    {
                        *clear_strip = true;
                        if scan.running {
                            cmds.push(Command::SetScanning(false));
                            cmds.push(Command::SetScanning(true));
                        }
                    }
                    return;
                }
                if !scan.running {
                    return;
                }
                // Right to left: SKIP, then ▶ and ◀ read left to right.
                if crate::chrome::chip(ui, false, tr!("scanner-skip"))
                    .on_hover_text(tr!("scanner-move-don-t-stop-channel"))
                    .clicked()
                {
                    cmds.push(Command::ScanSkip);
                }
                if crate::chrome::chip(ui, false, "▶")
                    .on_hover_text(tr!("scanner-forward"))
                    .clicked()
                {
                    cmds.push(Command::ScanNext);
                }
                if crate::chrome::chip(ui, false, "◀")
                    .on_hover_text(tr!("scanner-backward"))
                    .clicked()
                {
                    cmds.push(Command::ScanBack);
                }
            });
        });
    }
}

/// A band to scan, picked from the region's allocations — amateur, broadcast,
/// CB, AM — rather than typed: it sets both edges at once. Shows the band the
/// edges are on, or a dash once they have been typed elsewhere.
// Why: HDZ 2026-11-12: "ein Popdown, der die Ham- plus Radiobänder kennt …
//      Knöpfe sollten dem Bandselektor vorbehalten sein".
fn band_pick(ui: &mut egui::Ui, cfg: &mut ScannerConfig) {
    let bands = crate::widgets::bandplan::allocations();
    let (lo, hi) = cfg.range();
    let on = bands.iter().find(|(l, h, _)| (l - lo).abs() < 1.0 && (h - hi).abs() < 1.0);
    let shown = on.map_or("—".to_string(), |(_, _, name)| (*name).to_string());
    egui::ComboBox::from_id_salt("scan-band").selected_text(shown).height(400.0).show_ui(
        ui,
        |ui| {
            for (l, h, name) in &bands {
                let text = format!("{name}   {:.3}–{:.3}", l / 1e6, h / 1e6);
                if ui.selectable_label(on.is_some_and(|b| b.2 == *name), text).clicked() {
                    cfg.range_lo_hz = *l;
                    cfg.range_hi_hz = *h;
                }
            }
        },
    );
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
