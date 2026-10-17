//! **Classic UI** — every control in one column down the left.
//!
//! The arrangement SDR++ and SDR++Brown use, and therefore the one most
//! short-wave listeners already know: a column of sections down the left, each
//! folding away, with the picture taking everything else. It is on `main`
//! because it answers the complaint this fork keeps hearing — two readers on
//! r/ShortwavePlus, both SDR++ users, called the program *"too complicated,
//! controls all over the place"*.
//!
//! **The manner is SDR++'s; the controls are ours.** That is the whole design,
//! and it took two attempts to get there. The first drew the *strip's* modules —
//! `vfo_rit_module`, `rx_filter_module`, `tx_condensed` and the rest — inside
//! the column, on the rule that a control should have one implementation. The
//! rule is right for the docked band column and the detached windows, which
//! *are* the same widgets in another place; it is wrong here, because those
//! modules are boxes built for a strip: each has its own margins and a width
//! chosen to make a *row of boxes* come out even. In a resizable column they
//! fight it — the panel's width and the content's width disagree, the drag
//! springs, and a fixed-width box is what the operator sees. The operator's own
//!words: *"it's a complete new section… I would replace it with a complete new
//! container containing all controls nicely categorized, adaptive"*.
//!
//! So: **every row here takes the width it is given.** A row is a caption in a
//! left gutter and controls filling the rest ([`classic_row`]); a run of chips
//! spreads and wraps ([`classic_chips`]); a rail is fitted to what is left of
//! its line (`chrome::slider_readout`). Nothing has a width of its own, which is
//! what makes the panel's edge draggable at all.
//!
//! **Behaviour still lives in one place.** Where a chip carries a rule — the
//! mode chips' band-grey, BIN's sub-receiver gate, the transmit refusals, the
//! passband presets, the noise-reduction engines — this file calls the same
//! function the strip and the RX menu call (`mode_chip_grid`, `rx_chip`,
//! `filter_controls`, `nr_controls`, `rec_controls`, `system_chips_*`,
//! `display_*_chips`, the `tx_*` helpers). So there is one implementation of
//! every *control* and two layouts: this one, and the strip's.
//!
//! Off by default, and desktop only: the strip is what this program's own
//! screens are built around, and a phone has no room for a column at all.

use eframe::egui::{self, RichText, Slider};
use sdroxide_types::{AgcMode, ClassicSection, Command, Direction, RxId};

/// The column's width range, in points.
const CLASSIC_MIN_W: f32 = 232.0;

/// The widest the column may be dragged. Generous on purpose: a drag that stops
/// reads as a drag that does not work, and the operator's first report was
/// exactly that.
const CLASSIC_MAX_W: f32 = 640.0;

/// How much room the picture must keep, so opening the column on a narrow
/// window squeezes the waterfall rather than taking it away.
const CLASSIC_PANADAPTER_MIN_W: f32 = 360.0;

/// The width the column opens at, the first time it is used. The operator's
/// number: the first version opened at 280 and remembered whatever it was
/// dragged to, which is right, but 300 is the start he asked for.
pub(in crate::app) const CLASSIC_DEFAULT_W: f32 = 300.0;

/// What the column folds down to: a rail wide enough for the chip that opens it
/// again, and nothing else. The operator's idea, and the better one — a column
/// that vanishes leaves no way back except a control somewhere else, and
/// somewhere else is what made the old fold chip look out of place.
pub(in crate::app) const CLASSIC_RAIL_W: f32 = 22.0;

/// The caption gutter. Every row's caption starts here and its controls follow,
/// so the rails and fields line up down the column the way SDR++'s do.
const LABEL_W: f32 = 64.0;

/// What the arrangement should draw this frame — **one decision, asked once**.
///
/// The strip has to know it too (wherever the column is *not* drawn, the strip
/// keeps its own controls), and two copies of that decision is how the pair
/// drifted once already: the strip folded while the column was left out for want
/// of room, leaving the bands, the VFO and the tools in neither place.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::app) enum ClassicSurface {
    /// Not in use — the strip is the whole story.
    None,
    /// Folded: a rail with the chip that opens it.
    Rail,
    /// Open, between these widths.
    Column(f32, f32),
}

/// See [`ClassicSurface`].
pub(in crate::app) fn classic_surface(
    tier: crate::layout::Tier,
    window_w: f32,
    collapsed: bool,
) -> ClassicSurface {
    if tier == crate::layout::Tier::Phone {
        return ClassicSurface::None;
    }
    if collapsed {
        return ClassicSurface::Rail;
    }
    match classic_column_width(window_w) {
        Some((min, max)) => ClassicSurface::Column(min, max),
        None => ClassicSurface::None,
    }
}

/// The width the column opens at, and may be dragged between, for a window this
/// wide.
///
/// Pure, so the arithmetic that stops the column eating the panadapter is a
/// thing a test can hold rather than a property of a running window. Returns
/// `None` when there is no room at all.
pub(in crate::app) fn classic_column_width(window_w: f32) -> Option<(f32, f32)> {
    let room = window_w - CLASSIC_PANADAPTER_MIN_W;
    if room < CLASSIC_MIN_W {
        return None;
    }
    let max = room.min(CLASSIC_MAX_W);
    Some((CLASSIC_MIN_W, max))
}

impl super::SdroxideApp {
    /// The column: a heading, then a scroll area of sections, each folding away.
    ///
    /// `egui::CollapsingHeader` rather than a hand-rolled fold, because it is
    /// what this program already uses for a fold of settings and it keeps its
    /// own open-or-closed state between sessions.
    pub(in crate::app) fn classic_column(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        // **The same border the picture has.** The column is a panel like the
        // waterfall and the operating panel, and `angled_frame` is what draws
        // those — the cut-corner outline and the fill. Without it the column
        // read as a region of the window rather than as a panel in it.
        crate::chrome::angled_frame(ui, crate::theme::PINK(), |ui| {
            // **Left-aligned, said out loud.** A `Frame`'s content layout
            // defaults to `top_down(Center)` — egui's own default — so without
            // this every row in the column is *centred*: the headings sit in the
            // middle of their bars, and a chip run a few points wider than the
            // column hangs out of both sides at once, cut at the left. It looked
            // like padding that was missing and was in fact the layout.
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                self.classic_column_inner(ui, cmds);
            });
        });
    }

    /// The column's contents: the heading row and the sections, in the order the
    /// operator put them in.
    fn classic_column_inner(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        // The heading carries the fold, on the column's own right edge — where
        // the thing it moves is. `◀` folds it to the rails; the `▶` on the rails
        // brings it back. Nothing of this is drawn when Classic UI is off.
        ui.horizontal(|ui| {
            ui.label(RichText::new("CONTROLS").size(10.5).strong().color(crate::theme::CYAN_DIM()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if crate::chrome::chip(ui, false, RichText::new('◀').size(11.0))
                    .on_hover_text(
                        "Fold the column away to a rail on the left. The readout and the meter \
                         stay on the bar, and the control that brings this back is on the rail.",
                    )
                    .clicked()
                {
                    self.classic_collapsed = true;
                }
            });
        });
        ui.separator();
        // A bounded width, and that is load-bearing: a horizontal scroll area
        // makes `available_width` *infinite*, so every row would stretch to
        // infinity and be clipped at the viewport — which reads as chips that
        // need wrapping and is in fact nothing being given a width at all.
        // **The bar floats, and the content has air.** A scroll bar that
        // allocates takes its width off every row it appears beside — the
        // operator's *"it takes a lot of realestate and the padding is non
        // existent with the chips"*, which is one complaint: the chips end up
        // flush against the edge with the bar on top of them. Floating, the bar
        // is drawn over the margin instead of beside the chips.
        {
            let style = ui.style_mut();
            style.spacing.scroll.floating = true;
            style.spacing.scroll.bar_width = 8.0;
            style.spacing.scroll.content_margin =
                egui::Margin { left: 6, right: 10, top: 2, bottom: 8 };
        }
        egui::ScrollArea::vertical().auto_shrink([false, false]).id_salt("classic-column").show(
            ui,
            |ui| {
                let order = ClassicSection::order(&self.ui_settings.classic_sections);
                let mut moved: Option<(usize, usize)> = None;
                for sec in order.iter().copied() {
                    // **A section with nothing under it is worse than no
                    // section**, so a control that does not apply takes its
                    // heading with it: a listening screen (SWL mode, or a radio
                    // that cannot transmit) has no Transmit fold at all. The
                    // gating question is asked once, of the app, so the column
                    // and the strip's own Transmit box cannot disagree.
                    if sec == ClassicSection::Transmit && !self.tx_capable() {
                        continue;
                    }
                    if let Some(m) = self.classic_section_at(ui, cmds, sec, &order) {
                        moved = Some(m);
                    }
                }
                // Applied after the whole column is drawn, not inside the loop:
                // moving a section while its own index is being walked would
                // renumber everything below it mid-frame.
                if let Some((from, to)) = moved {
                    let mut list = order;
                    let item = list.remove(from);
                    list.insert(to.min(list.len()), item);
                    self.ui_settings.classic_sections = ClassicSection::order_text(&list);
                    crate::app::persist::persist_ui_settings(&self.ui_settings);
                }
            },
        );
    }

    /// **One section**, and the move the operator made of it this frame.
    ///
    /// Dragged by its heading and dropped on another section's box, which is the
    /// gesture a list of folds already teaches. The order is stored as the list
    /// of section keys ([`ClassicSection::order_text`]), so it survives a
    /// restart and a build that adds a section.
    fn classic_section_at(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        sec: ClassicSection,
        order: &[ClassicSection],
    ) -> Option<(usize, usize)> {
        let id = ui.make_persistent_id(("classic-section", sec.key()));
        let state = egui::collapsing_header::CollapsingState::load_with_default_open(
            ui.ctx(),
            id,
            sec.default_open(),
        );
        let id_drag = ui.make_persistent_id(("classic-drag", sec.key()));
        // **`dnd_drop_zone` is deliberately not used**, and the reason is
        // visible: it *overrides* the frame it is given with the widget style's
        // fill and stroke (`egui::Ui::dnd_drop_zone`, "the color is ignored"), so
        // every section came out as a filled, bordered box — the operator's
        // "the sections have their own border too, ditch them, they look warped
        // in the layout". The drop target is this section's own heading bar,
        // brightened while a section is being dragged over it, which is a mark
        // that belongs to the column rather than a box around it.
        //
        // The rect is the *previous* frame's, because a section can only be
        // known to be under the pointer once it has been drawn: one frame behind
        // on a drag nobody can see, and it costs no state but egui's own temp
        // store.
        let rect_id = id.with("drop-rect");
        let dragging = egui::DragAndDrop::has_payload_of_type::<ClassicSection>(ui.ctx());
        let pointer = ui.ctx().pointer_interact_pos();
        let last: Option<egui::Rect> = ui.ctx().data(|d| d.get_temp(rect_id));
        let over = dragging && pointer.is_some_and(|p| last.is_some_and(|r| r.contains(p)));

        let inner = ui
            .scope(|ui| {
                let was_open = state.is_open();
                let mut toggled = false;
                let mut header = state.show_header(ui, |ui| {
                    // **An accent bar behind the heading**, so where a section
                    // starts is visible without reading: the operator's ask,
                    // and the thing a column of folds needs more than a strip
                    // of boxes does — a fold's own arrow is a small mark.
                    //
                    // The bar is reserved as a paint slot before the text is
                    // drawn and filled in afterwards, so it lands *under* the
                    // words — the same trick `angled_frame` uses for its
                    // gradient.
                    let slot = ui.painter().add(egui::Shape::Noop);
                    let row = ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), 16.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.add_space(2.0);
                            // **The bold face, asked for by name.** egui's
                            // `strong()` is a *colour*, not a weight, so a
                            // heading that says `strong()` is exactly as thin
                            // as the text beside it; `chakra-bold` is the
                            // family the theme already installs for headings.
                            ui.label(
                                RichText::new(sec.label())
                                    .size(11.5)
                                    .family(bold_family())
                                    .color(crate::theme::TEXT_STRONG()),
                            );
                        },
                    );
                    // **The whole bar is the control.** A click folds the
                    // section — the fold's own arrow is a small target on a
                    // heading that is the width of the column, and the
                    // operator asked for the bar itself to work — and a
                    // *drag* moves it, which egui tells apart from a click by
                    // the distance travelled.
                    let bar =
                        ui.interact(row.response.rect, id_drag, egui::Sense::click_and_drag());
                    if bar.clicked() {
                        toggled = true;
                    }
                    if bar.dragged() {
                        egui::DragAndDrop::set_payload(ui.ctx(), sec);
                    }
                    bar.on_hover_text(
                        "Click to open or close this section. Drag it onto another \
                             section to move it — the order is remembered.",
                    );
                    let bar_rect = egui::Rect::from_min_max(
                        egui::pos2(ui.max_rect().left(), row.response.rect.top() - 2.0),
                        egui::pos2(ui.max_rect().right(), row.response.rect.bottom() + 2.0),
                    );
                    ui.painter()
                        .set(slot, egui::Shape::rect_filled(bar_rect, 2.0, header_fill(over)));
                });
                // The click is applied *before* the body is drawn, or the fold
                // would answer a frame late.
                if toggled {
                    header.set_open(!was_open);
                }
                header.body(|ui| self.classic_section_body(sec, ui, cmds));
            })
            .response;
        ui.ctx().data_mut(|d| d.insert_temp(rect_id, inner.rect));

        let wrapped = if over && ui.input(|i| i.pointer.any_released()) {
            egui::DragAndDrop::take_payload::<ClassicSection>(ui.ctx())
        } else {
            None
        };
        let payload = wrapped?;
        let from = order.iter().position(|s| *s == *payload)?;
        let to = order.iter().position(|s| *s == sec)?;
        (from != to).then_some((from, to))
    }

    /// What one section holds. Every body is the same body the strip and the RX
    /// menu draw, so a rule on a control is written once.
    fn classic_section_body(
        &mut self,
        sec: ClassicSection,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
    ) {
        match sec {
            ClassicSection::Source => self.classic_source_body(ui, cmds),
            ClassicSection::Bands => self.classic_bands_body(ui, cmds),
            ClassicSection::Receiver => self.classic_receiver_body(ui, cmds),
            ClassicSection::Tuning => self.classic_tuning_body(ui, cmds),
            ClassicSection::Transmit => self.classic_tx_body(ui, cmds),
            ClassicSection::Recorder => self.classic_recorder_body(ui, cmds),
            ClassicSection::Display => self.classic_display_body(ui, cmds),
            ClassicSection::Tools => self.classic_system_body(ui, cmds),
        }
    }

    /// The folded column: a rail with one chip on it.
    pub(in crate::app) fn classic_rail(&mut self, ui: &mut egui::Ui, _cmds: &mut Vec<Command>) {
        // At the top, where the column's heading was, so the rail reads as that
        // heading folded rather than as a new control somewhere.
        if crate::chrome::chip(ui, true, RichText::new('▶').size(11.0))
            .on_hover_text("Bring the control column back.")
            .clicked()
        {
            self.classic_collapsed = false;
        }
    }

    /// **Source** — which radio this is, and what it is doing.
    ///
    /// SDR++'s first section is the front end itself: the device, the rate, the
    /// gain. Ours is chosen on Settings → Radio, so this reads rather than
    /// edits and puts the route beside the values. Saying what is configured and
    /// where to change it is the honest half of the section; a second gain
    /// slider here would be a second opinion about the same setting.
    fn classic_source_body(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let label =
            self.caps.as_ref().map(|c| c.label.clone()).unwrap_or_else(|| "no radio".to_string());
        source_row(ui, "Radio", &label);
        source_row(ui, "Rate", &format!("{:.3} Msps", self.state.sample_rate / 1e6));
        source_row(ui, "Dial", &crate::app::frame::readout_text(self.state.rx_freq_hz()));
        source_row(ui, "Mode", self.state.rx[0].mode.label());
        if self.swl_mode() {
            source_row(ui, "Receive", "listening screen");
        }
        ui.add_space(4.0);
        if crate::chrome::chip(ui, self.show_settings, "RADIO SETTINGS")
            .on_hover_text(
                "The backend, the sample rate, the aerial, the converter and this \
                 radio's own calibration — Settings → Radio, where they have always \
                 been. The column shows what they are set to rather than a second copy \
                 of the controls.",
            )
            .clicked()
        {
            self.show_settings = true;
        }
        let _ = cmds;
    }

    /// **Band & mode** — the band list, the metre-band shortcuts and the mode
    /// matrix, which is `band_menu_body`: the same body the docked column and the
    /// detached band window draw, so there is one band selector in the program
    /// and not three. It is already a wrapped, adaptive body — it is the popup's
    /// — so it needs nothing from this file.
    fn classic_bands_body(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        self.band_menu_body(ui, cmds);
    }

    /// **Receiver** — the rails, one to a row, then the chip run, then the
    /// passband and the noise reduction.
    ///
    /// One rail per row is the SDR++ shape and it is also what makes the column
    /// adapt: a row with one rail and a readout has no fixed width to defend, so
    /// the panel's edge follows the drag. The strip puts volume and gain on one
    /// row because a strip is a plane of boxes; a column is a list.
    fn classic_receiver_body(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        classic_row(ui, "Vol", |ui| {
            let mut vol = self.state.rx[0].volume;
            let resp = crate::chrome::slider_readout(
                ui,
                crate::chrome::text_width(ui, "-888.8", egui::TextStyle::Body.resolve(ui.style()))
                    + 22.0,
                Slider::new(&mut vol, 0.0..=1.0).show_value(false),
            )
            .on_hover_text("Volume");
            if resp.changed() {
                self.state.rx[0].volume = vol; // optimistic echo
                cmds.push(Command::SetVolume { rx: RxId::Main, v: vol });
            }
        });
        // The front-end gain stage, if this rig has one — and inert, saying
        // so, where the radio's own AGC is driving it (issue #417).
        if let Some(g) = self.rx_gains().first().cloned() {
            let hardware_agc = self.pluto_agc_owns_gain(&g.name);
            let mut hint = format!(
                "Front-end RX gain ({}). Too much clips the receiver's ADC and smears \
                 spurious signals across the band; too little and it goes deaf.",
                g.name
            );
            if g.unit == sdroxide_types::GainUnit::Step {
                hint.push_str(
                    "\n\nCounted in steps, not decibels: what one step is worth is the \
                     receiver's own business, and on an SDRplay it depends on the band as \
                     well. Right is still more gain.",
                );
            }
            if hardware_agc {
                hint.push_str(
                    "\n\nThe PlutoSDR's own AGC is setting this gain, so moving the slider \
                     does nothing. Set Settings → Radio → AGC to Manual to control it here.",
                );
            }
            classic_row(ui, "Gain", |ui| {
                let mut db = self
                    .state
                    .gains
                    .iter()
                    .find(|(n, _)| *n == g.name)
                    .map(|(_, d)| *d)
                    .unwrap_or(g.min_db);
                let step = if g.step_db > 0.0 { g.step_db } else { 1.0 };
                let resp = ui
                    .add_enabled_ui(!hardware_agc, |ui| {
                        crate::chrome::slider_readout(
                            ui,
                            crate::chrome::text_width(
                                ui,
                                "-888.8 dB",
                                egui::TextStyle::Body.resolve(ui.style()),
                            ) + 22.0,
                            Slider::new(&mut db, g.min_db..=g.max_db)
                                .step_by(step)
                                .suffix(g.suffix()),
                        )
                    })
                    .inner
                    .on_hover_text(&hint)
                    .on_disabled_hover_text(&hint);
                if resp.changed() {
                    match self.state.gains.iter_mut().find(|(n, _)| *n == g.name) {
                        Some((_, d)) => *d = db,
                        None => self.state.gains.push((g.name.clone(), db)),
                    }
                    cmds.push(Command::SetGain { dir: Direction::Rx, element: g.name.clone(), db });
                }
            });
        }
        classic_row(ui, "Filter", |ui| {
            self.decim_chip(ui, cmds);
            // The AGC chip rides this row rather than one of its own: it is
            // a state light with a cycle behind it, not a value, and four
            // settings are few enough to walk through.
            if self.state.rx[0].mode.audio_agc() {
                let agc = self.state.rx[0].agc;
                if crate::chrome::chip(ui, agc != AgcMode::Off, format!("AGC {}", agc.label()))
                    .on_hover_text("AGC hang time — click to cycle: Off / Slow / Med / Fast")
                    .clicked()
                {
                    cmds.push(Command::SetAgc { rx: RxId::Main, agc: agc.next() });
                }
            }
        });
        // With the AGC off the audio rides on this fixed gain instead — the
        // demodulator's output unlevelled is whatever the band handed it.
        if self.state.rx[0].mode.audio_agc() && self.state.rx[0].agc == AgcMode::Off {
            classic_row(ui, "Man", |ui| {
                let mut db = self.state.rx[0].manual_gain_db;
                let resp = crate::chrome::slider_readout(
                    ui,
                    crate::chrome::text_width(
                        ui,
                        "-888.8 dB",
                        egui::TextStyle::Body.resolve(ui.style()),
                    ) + 22.0,
                    Slider::new(&mut db, 0.0..=sdroxide_types::MAX_MANUAL_GAIN_DB)
                        .step_by(1.0)
                        .suffix(" dB"),
                )
                .on_hover_text(
                    "Manual audio gain, used while the AGC is off. Seeded from the level \
                     the AGC was holding when it was switched off.",
                );
                if resp.changed() {
                    self.state.rx[0].manual_gain_db = db; // optimistic echo
                    cmds.push(Command::SetManualGain { rx: RxId::Main, db });
                }
            });
        }
        classic_row(ui, "Squelch", |ui| {
            let sql = self.state.rx[0].squelch_db;
            let mut v = sql;
            let resp = crate::chrome::slider_readout(
                ui,
                crate::chrome::text_width(ui, "-888", egui::TextStyle::Body.resolve(ui.style()))
                    + 22.0,
                Slider::new(&mut v, -150.0..=0.0).show_value(false).custom_formatter(|v, _| {
                    if v <= -149.5 { "off".into() } else { format!("{v:.0} dB") }
                }),
            )
            .on_hover_text(
                "Gate the audio below this power in the receive passband, in dBFS. Left is \
                 open. Set it above the noise and below the signal.",
            );
            if resp.changed() {
                self.state.rx[0].squelch_db = v; // optimistic echo
                cmds.push(Command::SetSquelch { rx: RxId::Main, db: v });
            }
        });
        // The receiver's switches — every one of them the strip's own chips,
        // drawn by the strip's own function so a gate cannot be missed here.
        classic_chips(ui, |ui| {
            for c in super::top_bar::rx_chips(self.state.rx[0].mode, self.listener_screen()) {
                self.rx_chip(ui, cmds, c, true);
            }
        });
        // Passband presets, edges and the noise-reduction engines: the RX
        // menu's own bodies, which are written for a narrow column already.
        ui.add_space(4.0);
        self.filter_controls(ui, cmds);
        self.nr_controls(ui, cmds);
    }

    /// **Tuning** — the VFO, split and the RIT/XIT offsets.
    fn classic_tuning_body(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        classic_chips(ui, |ui| {
            self.vfo_util_chips(ui, cmds, 0.0);
        });
        ui.horizontal_wrapped(|ui| {
            self.vfo_offset_row(ui, cmds, false, 0.0);
        });
    }

    /// **Transmit** — the key, the drive and the levels.
    ///
    /// Every chip here is the strip's own, so the refusals that matter — a rig
    /// that keys itself, a station that cannot transmit at all — are the same
    /// refusals with the same hover text saying why.
    fn classic_tx_body(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        classic_row(ui, "Key", |ui| {
            self.tx_ptt_chip(ui, cmds);
            self.tx_keyer_chip(ui);
        });
        classic_row(ui, "Drive", |ui| {
            self.tx_drive(ui, cmds);
        });
        classic_row(ui, "Tune", |ui| {
            self.tx_tune_chip(ui, cmds);
            self.tx_tune_level(ui, cmds);
        });
        // One of these two, never both: in a digital mode the microphone
        // reaches nothing, so the level rail takes its place rather than
        // sitting dead beside a live one (issue #186).
        if self.digi_tx_level_applies() {
            let mut db = self.digi_tx_level_db();
            let hover = self.digi_tx_level_hover();
            classic_row(ui, "Level", |ui| {
                let resp = crate::chrome::slider_readout(
                    ui,
                    crate::chrome::text_width(
                        ui,
                        "-888 dB",
                        egui::TextStyle::Body.resolve(ui.style()),
                    ) + 22.0,
                    Slider::new(&mut db, sdroxide_types::TX_AUDIO_LEVEL_MIN_DB..=0.0).suffix(" dB"),
                )
                .on_hover_text(hover);
                if resp.changed() {
                    self.set_digi_tx_level(db, cmds);
                }
            });
        } else {
            classic_row(ui, "Mic", |ui| {
                let mut mic = self.state.tx.mic_gain;
                let resp = crate::chrome::slider_readout(
                    ui,
                    crate::chrome::text_width(
                        ui,
                        "-888",
                        egui::TextStyle::Body.resolve(ui.style()),
                    ) + 22.0,
                    Slider::new(&mut mic, 0.0..=1.0).show_value(false),
                )
                .on_hover_text("Microphone gain");
                if resp.changed() {
                    self.state.tx.mic_gain = mic;
                    cmds.push(Command::SetMicGain(mic));
                }
            });
        }
        if self.cessb_applies() {
            let mut db = self.state.tx.cessb_db;
            classic_row(ui, "CESSB", |ui| {
                let resp = crate::chrome::slider_readout(
                    ui,
                    crate::chrome::text_width(
                        ui,
                        "-88 dB",
                        egui::TextStyle::Body.resolve(ui.style()),
                    ) + 22.0,
                    Slider::new(&mut db, 0.0..=sdroxide_types::CESSB_MAX_DB).suffix(" dB"),
                )
                .on_hover_text(
                    "Controlled-envelope SSB: how much of the speech processing to \
                     apply. Up is louder and flatter, at the cost of some splatter \
                     either side.",
                );
                if resp.changed() {
                    cmds.push(Command::SetCessb(db));
                }
            });
        }
    }

    /// **Recorder** — what is written to disk, and how. The RX menu's own body:
    /// record audio, quick clip, auto-record, spectrum and playback.
    fn classic_recorder_body(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        self.rec_controls(ui, cmds);
    }

    /// **Display** — what the picture shows. The strip's own chip rows, spread
    /// across the column and wrapped rather than sitting in a box.
    fn classic_display_body(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        classic_chips(ui, |ui| {
            self.display_view_chips(ui, false, 0.0);
            self.display_tool_chips(ui, cmds, false, 0.0);
        });
    }

    /// **Windows & tools** — every window this radio can open, which is what
    /// SDR++ calls its plugin list. The strip's own chips again: SWL, the log,
    /// the schedule, the decoders, the tools and the settings.
    fn classic_system_body(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        classic_chips(ui, |ui| {
            self.system_chips_top(ui, 0.0);
            self.system_chips_bottom(ui, 0.0, cmds);
        });
    }
}

/// The wash behind a section heading.
///
/// The same 0.14 the notice bars use ([`crate::chrome::ribbon`]): a colour at
/// that strength reads as a bar with the heading's own ink on it, where a solid
/// accent would fight the words and a plain `ROW_BG` would not be found at all.
/// The theme's bold face, by the name it is installed under
/// (`theme::install_fonts`). egui's `RichText::strong()` is a *colour*, not a
/// weight: a heading that says `strong()` is exactly as thin as the text beside
/// it, which is why the operator asked for bold by name.
fn bold_family() -> egui::FontFamily {
    egui::FontFamily::Name("chakra-bold".into())
}

fn header_fill(over: bool) -> egui::Color32 {
    // Brighter while a section is being dragged over this one: the drop target
    // is the heading itself, not a box around the section.
    crate::theme::CYAN().gamma_multiply(if over { 0.34 } else { 0.16 })
}

/// A full-width row: a caption in the column's left gutter, its controls in the
/// rest of the line.
///
/// **Wrapped, and the slider's width is left to be fitted to the line**
/// (`chrome::slider_readout`): that is what makes the row adapt to whatever the
/// column's edge is dragged to. A row that pinned its own width is exactly what
/// the first version of this column did, and it is why the drag felt wrong.
fn classic_row(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal_wrapped(|ui| {
        // Generous, not a size: `fit_rail` takes the smaller of this and what is
        // left of the line, so the rail fills a wide column and yields in a
        // narrow one.
        ui.spacing_mut().slider_width = 520.0;
        gutter_label(ui, label);
        add(ui);
    });
}

/// The caption in the column's left gutter: the width reserved, and the words at
/// its *left*.
///
/// `add_sized` — which is what this was — centres what it is given, so every
/// caption in the column sat in the middle of its own gutter.
pub(in crate::app) fn gutter_label(ui: &mut egui::Ui, label: &str) {
    ui.allocate_ui_with_layout(
        egui::vec2(LABEL_W, 16.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.label(RichText::new(label).size(10.5).color(crate::theme::gray(150)));
        },
    );
}

/// A run of chips: they spread across the column and wrap. No fixed widths.
fn classic_chips(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal_wrapped(add);
}

/// One `label — value` line, the shape every row of the Source section takes.
///
/// Wrapped, and the value is elided at its *end* if it still does not fit.
/// Right-aligning it — which this did — clips a long value at its **left**: the
/// operator's column at 250 points read `SDRplay RSP1 (serial 0000000001) @ 3.000
/// Msps` as `RSP1 (serial 0000000001) @ 3.000 Msps`, and the wrong end is lost.
/// A value that does not fit now wraps under its caption and truncates only if
/// the column is narrower than the value alone.
fn source_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal_wrapped(|ui| {
        gutter_label(ui, label);
        ui.add(
            egui::Label::new(RichText::new(value).size(11.0).color(crate::theme::TEXT_STRONG()))
                .truncate(),
        )
        .on_hover_text(value);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Where the column is drawn, and where the strip keeps its own controls.**
    /// One decision, asked once: the frame draws the surface this names, and the
    /// strip folds its boxes only where a surface is actually drawn.
    #[test]
    fn the_surface_is_the_columns_own_room() {
        use crate::layout::{Tier, tier_for};
        use sdroxide_types::LayoutMode;
        let desktop = tier_for(egui::vec2(1400.0, 900.0), LayoutMode::Auto);
        assert_eq!(classic_surface(desktop, 1400.0, false), ClassicSurface::Column(232.0, 640.0));
        assert_eq!(classic_surface(desktop, 1400.0, true), ClassicSurface::Rail);
        // 592 pt is the floor: 232 for the column and 360 for the picture.
        assert_eq!(classic_surface(desktop, 591.0, false), ClassicSurface::None);
        assert_eq!(classic_surface(desktop, 591.0, true), ClassicSurface::Rail);
        assert_eq!(
            classic_surface(Tier::Phone, 400.0, false),
            ClassicSurface::None,
            "a phone is never given a column"
        );
        assert_eq!(
            classic_surface(Tier::Phone, 400.0, true),
            ClassicSurface::None,
            "not even a rail: there is nowhere for it to go"
        );
    }

    /// **The column never eats the picture.** The room runs out at 592 pt — 232
    /// for the column, 360 for the picture — and below that the column is left
    /// out and the strip is drawn as usual. That is the #643 shape: a column
    /// that grows past what it was given takes the thing beside it off the
    /// screen.
    #[test]
    fn the_column_yields_to_the_panadapter() {
        assert!(classic_column_width(360.0).is_none());
        assert!(classic_column_width(592.0).is_some());
        for w in [600.0, 900.0, 1280.0, 1920.0] {
            let (min, max) = classic_column_width(w).expect("room");
            assert!(min >= CLASSIC_MIN_W && max >= min, "{w}: {min}..{max}");
            assert!(max <= CLASSIC_MAX_W, "{w}: opens wider than the ceiling");
            assert!(
                w < 1000.0 || w - max >= CLASSIC_PANADAPTER_MIN_W,
                "{w}: the ceiling binds before the panadapter's floor does"
            );
        }
    }
}
