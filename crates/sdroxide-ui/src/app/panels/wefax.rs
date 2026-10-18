//! The weather-fax panel: the chart being painted, and the gallery of saved
//! ones.
//!
//! Like SSTV, a fax arrives one line at a time over several minutes, so what
//! the panel shows is a texture being written to rather than a finished image.
//! The station chip tunes the well-known schedules without leaving the panel.

use eframe::egui::{self, RichText};
use sdroxide_types::Command;

use crate::app::SdroxideApp;

impl SdroxideApp {
    /// The RF Paint (Spectrum Painting) operating panel: a text-paint area and an
    /// image-paint area side by side, each with a scrolling preview waterfall and
    /// a transmit button. Transmit rides the ordinary image-transmit command
    /// (`DigiImageTx`); the `RfPaintController` turns the bitmap into tones.
    /// FreeDV RADE V1 digital voice.
    ///
    /// There is no text or image to show and no tone offset to tune: the whole
    /// operating surface is "am I locked to the far end, how good is the link,
    /// and am I talking".
    /// The weather-fax panel: the chart as it arrives, the controls that decide
    /// its geometry, and the gallery of ones already saved.
    ///
    /// Receive only, so there is no transmit half — the space goes to the
    /// picture instead, which is the whole point of the mode.
    pub(in crate::app) fn wefax_panel(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        panel_h: f32,
    ) {
        use crate::theme;

        let st = self.wefax.status;
        let ctx = ui.ctx().clone();
        // An overloaded front end turns a chart into a flat wash, and the
        // panel would otherwise show a blank picture with no explanation.
        self.front_end_overload_warning(ui);
        // The charts are the engine's; list its store once when the panel first
        // opens. Charts that arrive afterwards are announced one at a time.
        if !self.wefax.listed {
            self.wefax.listed = true;
            self.wefax.page_pending = true;
            cmds.push(Command::ImageList {
                kind: sdroxide_types::ImageKind::Wefax,
                offset: 0,
                count: sdroxide_types::IMAGE_PAGE_MAX,
            });
        }
        // A chart arrives at two lines a second; there is no need to chase it
        // any faster than the eye can follow.
        crate::repaint::after_ms(&ctx, 200);

        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("WEFAX").size(12.0).strong().color(theme::CYAN()));
            self.wefax_station_chip(ui, cmds);
            self.wefax_carrier_note(ui);

            // START / STOP. Starting by hand is the normal way in: a chart runs
            // for a quarter of an hour and you will almost always have tuned to
            // it after the start tone went by.
            let (face, hint) = if st.receiving {
                (" ■ STOP ", "End the chart now and save what has arrived")
            } else {
                (" ● START ", "Start a chart now, without waiting for a start tone")
            };
            if crate::chrome::chip_accent(
                ui,
                st.receiving,
                RichText::new(face).strong(),
                if st.receiving { theme::PINK() } else { theme::GREEN() },
                theme::INK_ON_CYAN(),
            )
            .on_hover_text(hint)
            .clicked()
            {
                cmds.push(if st.receiving { Command::WefaxStop } else { Command::WefaxStart });
            }

            // What the receiver is making of the signal.
            let (text, colour) = if st.phasing {
                ("phasing…".to_string(), theme::YELLOW())
            } else if st.receiving {
                (
                    format!(
                        "receiving {:.0} min · {} lines",
                        f64::from(st.lines) / st.lpm.value(),
                        st.lines
                    ),
                    theme::GREEN(),
                )
            } else if self.wefax.has_live() {
                (format!("{} lines held", self.wefax.live_size().1), theme::CYAN_DIM())
            } else {
                ("listening".to_string(), theme::LINE_LIT())
            };
            ui.label(RichText::new(text).color(colour).size(11.0)).on_hover_text(
                "Waiting for a chart.\n\n\
                 WEFAX is a timetable, not a conversation: a station sends its charts at set \
                 times, so a chosen frequency is quiet for most of the day — that is normal. \
                 Use the STATIONS button to pick one and check the time.\n\n\
                 If a signal is there but the picture comes out one flat tone (all grey, all \
                 black or all white), it is the tuning or the speed — use the +0 Hz readout \
                 and the IOC/speed control, and check the STATIONS entry matches what is \
                 being sent.",
            );

            // The tuning readout. A correctly tuned receiver puts the
            // subcarrier's excursions around 1900 Hz; several hundred hertz off
            // and the picture is all black or all white.
            let off = st.subcarrier_hz - 1900.0;
            ui.label(
                RichText::new(format!("{:+.0} Hz", off))
                    .color(if off.abs() < 120.0 { theme::GREEN() } else { theme::YELLOW() })
                    .size(11.0)
                    .monospace(),
            )
            .on_hover_text(
                "Subcarrier offset from 1900 Hz. Tune for roughly zero: the fax carrier is \
                 1500 Hz black to 2300 Hz white, and a receiver a few hundred hertz off clips \
                 the picture to solid black or solid white.",
            );
            self.digi_squelch_slider(ui, cmds);
        });

        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let seeded = self.digi_cfg_seeded;
            let mut changed = false;

            ui.label(RichText::new("LPM").color(theme::CYAN_DIM()).size(9.5).strong());
            for l in sdroxide_types::WefaxLpm::ALL {
                let on = self.digi_cfg_edit.wefax_lpm == l;
                if crate::chrome::chip(ui, on, l.label()).clicked() && !on {
                    self.digi_cfg_edit.wefax_lpm = l;
                    changed = true;
                }
            }
            ui.add_space(8.0);
            ui.label(RichText::new("IOC").color(theme::CYAN_DIM()).size(9.5).strong());
            for i in sdroxide_types::WefaxIoc::ALL {
                let on = self.digi_cfg_edit.wefax_ioc == i;
                if crate::chrome::chip(ui, on, i.value().to_string())
                    .on_hover_text(format!("{} pixels per line", i.width()))
                    .clicked()
                    && !on
                {
                    self.digi_cfg_edit.wefax_ioc = i;
                    changed = true;
                }
            }

            ui.add_space(8.0);
            let auto_start = self.digi_cfg_edit.wefax_auto_start;
            if crate::chrome::chip(ui, auto_start, "AUTO START")
                .on_hover_text(
                    "Begin a chart when the 300 Hz (IOC 576) or 675 Hz start tone is heard \
                     — and end the one in progress and phase the new one when the next \
                     transmission arrives, whether or not a stop tone came first.",
                )
                .clicked()
            {
                self.digi_cfg_edit.wefax_auto_start = !auto_start;
                changed = true;
            }
            let auto_stop = self.digi_cfg_edit.wefax_auto_stop;
            if crate::chrome::chip(ui, auto_stop, "AUTO STOP")
                .on_hover_text(
                    "End it on the 450 Hz stop tone. Turn off to keep recording through a \
                     station that sends several charts back to back.",
                )
                .clicked()
            {
                self.digi_cfg_edit.wefax_auto_stop = !auto_stop;
                changed = true;
            }

            ui.add_space(8.0);
            // Phase nudge: for a chart whose phasing pulse was missed, which is
            // every chart you tune into halfway through.
            ui.label(RichText::new("PHASE").color(theme::CYAN_DIM()).size(9.5).strong());
            // Labelled with the shift itself rather than with arrows. The
            // media glyphs this used to wear (⏪ ◀ ▶ ⏩) are not in Chakra
            // Petch, and the fallback the browser substitutes measures
            // narrower than it draws — so a wrapped row decided they fitted
            // when they did not, and the last of them was clipped at the edge
            // of a phone. The amount is what the hover text said anyway.
            for (face, px) in [("−100", -100), ("−10", -10), ("+10", 10), ("+100", 100)] {
                if crate::chrome::chip(ui, false, face)
                    .on_hover_text(format!("Shift the picture {px} pixels"))
                    .clicked()
                {
                    // The decoder moves the lines still to come; this moves the
                    // part of the chart already on screen, so the whole picture
                    // shifts together (issue #439).
                    self.wefax.shift_phase(px);
                    cmds.push(Command::WefaxNudge(px));
                }
            }

            ui.add_space(8.0);
            ui.label(RichText::new("SLANT").color(theme::CYAN_DIM()).size(9.5).strong());
            let mut ppm = self.digi_cfg_edit.wefax_slant_ppm;
            if ui
                .add(
                    egui::DragValue::new(&mut ppm)
                        .speed(0.5)
                        .range(-500.0..=500.0)
                        .suffix(" ppm")
                        .fixed_decimals(1),
                )
                .on_hover_text(
                    "Sample-clock trim. If the chart leans to the left, increase this; to the \
                     right, decrease it. A sound card a hundred ppm off walks a quarter-hour \
                     chart most of a line sideways.",
                )
                .changed()
            {
                self.digi_cfg_edit.wefax_slant_ppm = ppm;
                changed = true;
            }

            // How the chart in progress is displayed. Nothing here touches the
            // decoder — a chart takes a quarter of an hour, and waiting for it
            // to finish before being allowed to look at the top of it is the
            // single most irritating thing about receiving fax.
            ui.add_space(8.0);
            ui.label(RichText::new("VIEW").color(theme::CYAN_DIM()).size(9.5).strong());
            use crate::wefax::Zoom;
            let zoom = self.wefax.zoom;
            for (face, z, hint) in [
                ("FIT", Zoom::FitWidth, "Scale the chart to the panel width"),
                ("WHOLE", Zoom::Whole, "Shrink the chart until all of it is in view at once"),
                ("50%", Zoom::Fixed(0.5), "Half size"),
                ("1:1", Zoom::Fixed(1.0), "One screen pixel per fax pixel — scroll for detail"),
                ("2×", Zoom::Fixed(2.0), "Twice size; scroll to move around the chart"),
            ] {
                if crate::chrome::chip(ui, zoom == z, face).on_hover_text(hint).clicked() {
                    self.wefax.zoom = z;
                }
            }

            // Vertical stretch. A chart comes out the wrong shape when the line
            // rate is not what the station is actually sending — 90 taken for
            // 120 makes it a third too tall — and this pulls it back while the
            // operator works out which rate that is.
            ui.label(RichText::new("HEIGHT").color(theme::CYAN_DIM()).size(9.5).strong());
            let mut aspect = self.wefax.aspect;
            if ui
                .add(
                    egui::DragValue::new(&mut aspect)
                        .speed(0.01)
                        .range(crate::wefax::MIN_ASPECT..=crate::wefax::MAX_ASPECT)
                        .prefix("×")
                        .fixed_decimals(2),
                )
                .on_hover_text(
                    "Stretch the picture vertically. A chart that came out squashed or stretched \
                     is usually being decoded at the wrong line rate — this makes it readable, \
                     and the LPM chips fix it properly. Double-click to type a value.",
                )
                .changed()
            {
                self.wefax.aspect = aspect;
            }
            if (aspect - 1.0).abs() > 0.001
                && crate::chrome::chip(ui, false, "RESET")
                    .on_hover_text("Back to the picture's own proportions")
                    .clicked()
            {
                self.wefax.aspect = 1.0;
            }

            let follow = self.wefax.follow;
            if crate::chrome::chip(ui, follow, "FOLLOW")
                .on_hover_text(
                    "Keep the newest lines in view. Scrolling up turns this off so you can read \
                     what has already arrived; scrolling back to the bottom turns it on again.",
                )
                .clicked()
            {
                self.wefax.follow = !follow;
            }

            if changed && seeded {
                cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
            }
        });

        ui.add_space(6.0);

        // The picture. Everything left of the gallery strip, scrollable, with
        // the newest rows kept in view while a chart is being received.
        //
        // Height from what is actually left rather than a fixed subtraction, so
        // the control rows wrapping in a narrow window shortens the picture
        // instead of pushing it out of the panel.
        let avail_h = ui.available_height().max(80.0).min(panel_h);
        let handle_w = 7.0;
        // A phone takes the chart and the gallery in turns: a 120 pt strip of
        // thumbnails beside a chart leaves neither wide enough to read, and a
        // weather chart is the one thing on this screen worth the whole width.
        let pane = self.phone_pane(ui, self.state.rx[0].mode);
        ui.horizontal_top(|ui| {
            // The gallery takes a user-draggable fraction of the width, the
            // chart the rest; each keeps enough to stay useful. The strip is
            // worth widening to read the labels that tell a morning's
            // identical-looking surface analyses apart, and worth shrinking
            // back out of the way while a chart is being read at 1:1.
            let total_w = ui.available_width();
            let gap = ui.spacing().item_spacing.x;
            let (img_w, gallery_w) = match pane {
                // Whichever one is up takes the row.
                Some(0) => (total_w, 0.0),
                Some(_) => (0.0, total_w),
                None => {
                    let g = (total_w * self.view.wefax_gallery_fraction)
                        .clamp(120.0, (total_w - handle_w - 2.0 * gap - 200.0).max(120.0));
                    ((total_w - g - handle_w - 2.0 * gap).max(120.0), g)
                }
            };
            // Both columns are explicitly top-down: `allocate_ui` inherits the
            // surrounding layout, which is the horizontal one that puts the two
            // columns side by side, and a gallery laid out left-to-right walks
            // its thumbnails off the edge of the window.
            if pane.is_none_or(|p| p == 0) {
                ui.allocate_ui_with_layout(
                    egui::vec2(img_w, avail_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        let receiving = st.receiving;
                        let (w, h) = self.wefax.live_size();
                        // Cloned rather than borrowed: the follow flag is updated from
                        // the scroll position afterwards, which needs the state back.
                        let tex = self.wefax.live_texture(&ctx).cloned();
                        match tex {
                            Some(tex) => {
                                // The scroll area's own width, less the bar it will
                                // put down the side, is what the picture has to fit
                                // into; using the column width would leave a chart
                                // fitted "to the width" a scrollbar too wide.
                                let bar = ui.spacing().scroll.bar_width + 4.0;
                                let view = (img_w - bar, avail_h - bar);
                                let size = crate::wefax::live_size(
                                    self.wefax.zoom,
                                    self.wefax.aspect,
                                    view,
                                    (w, h),
                                );
                                let out = egui::ScrollArea::both()
                                    .id_salt("wefax-live")
                                    .auto_shrink([false, false])
                                    // Follow the newest rows only while the operator is
                                    // at the bottom. Sticking regardless would snap the
                                    // view back every time a line arrived, which is
                                    // exactly what makes a chart unreadable until it has
                                    // finished.
                                    .stick_to_bottom(receiving && self.wefax.follow)
                                    .show(ui, |ui| {
                                        ui.add(
                                            egui::Image::new(&tex)
                                                .fit_to_exact_size(egui::vec2(size.0, size.1))
                                                // The size above already carries the
                                                // aspect the operator asked for;
                                                // preserving the texture's own would
                                                // undo the stretch control.
                                                .maintain_aspect_ratio(false),
                                        );
                                    });
                                // Where they left the view decides whether we keep
                                // following: at the bottom means "show me the new
                                // lines", anywhere else means "I am reading".
                                let slack = (out.content_size.y - out.inner_rect.height()).max(0.0);
                                self.wefax.follow = out.state.offset.y >= slack - 8.0;
                            }
                            None => {
                                ui.centered_and_justified(|ui| {
                                    ui.label(
                                    RichText::new(
                                        "Tune a fax schedule in USB and wait for a start tone, or \
                                     press START to begin mid-chart.",
                                    )
                                    .color(theme::LINE_LIT())
                                    .size(11.5),
                                );
                                });
                            }
                        }
                    },
                );
            }

            // Draggable vertical divider between the chart and the gallery.
            if pane.is_none() {
                let hresp = crate::chrome::split_handle(ui, egui::vec2(handle_w, avail_h), None);
                if hresp.dragged() {
                    // Dragging right shrinks the gallery (grows the chart).
                    let d = hresp.drag_delta().x / total_w.max(1.0);
                    self.view.wefax_gallery_fraction =
                        (self.view.wefax_gallery_fraction - d).clamp(0.1, 0.5);
                }
            }

            if pane.is_none_or(|p| p != 0) {
                ui.allocate_ui_with_layout(
                    egui::vec2(gallery_w, avail_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_max_width(gallery_w);
                        self.wefax_gallery(ui, gallery_w, cmds);
                    },
                );
            }
        });

        self.wefax_viewer(&ctx, cmds);
    }

    /// The gallery of saved charts: a thumbnail per chart, labelled with when it
    /// was received and which station it came from, and clickable to open it
    /// full size.
    ///
    /// The labels are the point. A quarter of a station's daily output is
    /// surface analyses that look identical at thumbnail size, and picking out
    /// "yesterday's 06Z from Pinneberg" without a date on it means opening them
    /// one at a time.
    fn wefax_gallery(&mut self, ui: &mut egui::Ui, width: f32, cmds: &mut Vec<Command>) {
        use crate::theme;

        let dir = self.wefax.dir.clone();
        // The path names a directory on the machine the radio is plugged into,
        // which is not this one when the engine is somewhere else. Copying it is
        // still the right thing to offer — it is exactly what you want on the
        // clipboard when you are about to go and look at it over there.
        let where_ = self.store_where(&dir);
        ui.horizontal(|ui| {
            ui.label(RichText::new("SAVED").color(theme::CYAN_DIM()).size(9.5).strong());
            if self.wefax.total > 0 {
                ui.label(
                    RichText::new(format!("{}", self.wefax.total))
                        .color(theme::LINE_LIT())
                        .size(9.5),
                );
            }
            if !dir.is_empty()
                && crate::chrome::chip(ui, false, RichText::new("PATH").size(9.5))
                    .on_hover_text(format!("Charts are saved {where_}\n\nClick to copy the path"))
                    .clicked()
            {
                ui.ctx().copy_text(dir.clone());
            }
        });

        if self.wefax.gallery.is_empty() {
            ui.label(
                RichText::new(if self.wefax.page_pending {
                    "Reading the radio's charts…".to_string()
                } else if dir.is_empty() {
                    "Finished charts collect here.".to_string()
                } else {
                    format!("Finished charts are saved {where_} and collect here.")
                })
                .color(theme::LINE_LIT())
                .size(10.0),
            );
            return;
        }

        let thumb_w = (width - 20.0).max(60.0);
        // A chart is about three times wider than it is tall, so a thumbnail
        // that keeps its shape is roughly a third of its width; charts cut short
        // are shorter still and simply take less room.
        let thumb_h = thumb_w * 0.36;
        let mut open = None;
        let mut more = false;
        // A card's context menu asked for a chart to go. By name, not by index:
        // the answer comes back asynchronously and a chart may have arrived at
        // the front of the gallery by then.
        let mut delete: Option<String> = None;
        egui::ScrollArea::vertical().id_salt("wefax-gallery").auto_shrink([false, false]).show(
            ui,
            |ui| {
                for (i, c) in self.wefax.gallery.iter().enumerate() {
                    let selected = self.wefax.viewing == Some(i);
                    // The whole card — picture and label together — is the
                    // target, so a click near the date is not a miss.
                    let card = ui.scope_builder(
                        egui::UiBuilder::new().sense(egui::Sense::click()),
                        |ui| {
                            ui.set_width(thumb_w);
                            ui.add(
                                egui::Image::new(&c.texture)
                                    .fit_to_exact_size(egui::vec2(thumb_w, thumb_h))
                                    .maintain_aspect_ratio(true),
                            );
                            match c.meta {
                                Some(m) => {
                                    ui.label(
                                        RichText::new(m.when_label())
                                            .color(if selected {
                                                theme::CYAN()
                                            } else {
                                                theme::TEXT()
                                            })
                                            .size(10.0)
                                            .monospace(),
                                    );
                                    // A chart saved before the name carried a
                                    // frequency has nothing to say about where
                                    // it came from; its size is at least true.
                                    ui.label(
                                        RichText::new(m.where_label().unwrap_or_else(|| {
                                            format!("{} × {}", c.size.0, c.size.1)
                                        }))
                                        .color(theme::CYAN_DIM())
                                        .size(9.5),
                                    );
                                }
                                // Something in the directory that this program
                                // did not name: show what there is rather than
                                // inventing a date for it.
                                None => {
                                    ui.label(RichText::new(&c.name).color(theme::TEXT()).size(9.5));
                                }
                            }
                        },
                    );
                    let resp = card.response;
                    if selected || resp.hovered() {
                        ui.painter().rect_stroke(
                            resp.rect.expand(2.0),
                            2.0,
                            egui::Stroke::new(
                                1.0,
                                if selected { theme::CYAN() } else { theme::LINE_LIT() },
                            ),
                            egui::StrokeKind::Inside,
                        );
                    }
                    let resp = resp.on_hover_text(format!(
                        "{}\n{} × {} pixels\n{}\n\nRight-click to delete",
                        c.meta.map_or_else(|| c.name.clone(), |m| m.when_full()),
                        c.size.0,
                        c.size.1,
                        c.name
                    ));
                    if resp.clicked() {
                        open = Some(i);
                    }
                    // A menu rather than a button on the card: the whole card
                    // is the target for opening it, and a delete sharing that
                    // target would be pressed by accident.
                    resp.context_menu(|ui| {
                        ui.label(RichText::new(c.title()).color(theme::CYAN_DIM()).size(10.0));
                        if ui.button("Delete this chart").clicked() {
                            delete = Some(c.name.clone());
                            ui.close();
                        }
                    });
                    ui.add_space(6.0);
                }
                // The rest of the store is a request away. Exact, not a guess:
                // the engine counted the directory.
                let older = self.wefax.total.saturating_sub(self.wefax.gallery.len() as u32);
                if older > 0 && !self.wefax.can_page() {
                    // Not a collection that has been lost — the store still has
                    // them and this says how many. This client simply stops
                    // holding thumbnails somewhere.
                    ui.label(
                        RichText::new(format!("{older} older in the store"))
                            .color(theme::LINE_LIT())
                            .size(9.5),
                    );
                } else if older > 0 {
                    let label = if self.wefax.page_pending {
                        "loading…".to_string()
                    } else {
                        format!("{older} older — load more")
                    };
                    if crate::chrome::chip(ui, false, RichText::new(label).size(9.5)).clicked()
                        && !self.wefax.page_pending
                    {
                        more = true;
                    }
                }
            },
        );
        if open.is_some() {
            self.wefax.viewing = open;
        }
        if more {
            self.wefax.page_pending = true;
            cmds.push(Command::ImageList {
                kind: sdroxide_types::ImageKind::Wefax,
                offset: self.wefax.gallery.len() as u32,
                count: sdroxide_types::IMAGE_PAGE_MAX,
            });
        }
        // The card goes when the engine says the file has, not here: the store
        // is on the radio's machine, and a thumbnail that vanished from a
        // delete that then failed would be a lie.
        if let Some(name) = delete {
            cmds.push(Command::ImageDelete { kind: sdroxide_types::ImageKind::Wefax, name });
        }
    }

    /// The radiofax schedules, as a chip that opens a station picker.
    ///
    /// These transmitters are not on any band plan and their frequencies are
    /// not the kind anyone remembers, so without this the mode starts with a
    /// trip to a web page. Picking one tunes the **dial**, which is 1.9 kHz
    /// below the published carrier — the subtraction that otherwise produces a
    /// blank page and no clue why.
    fn wefax_station_chip(&self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        use sdroxide_types::{WEFAX_STATIONS, WefaxStation};
        let dial = self.state.active_freq_hz();
        // The station we are on, if any, so the chip reads as a position.
        let here = WefaxStation::at_dial(dial);
        // No emoji in the face: the default fonts have no radio-mast glyph and
        // it comes out as an empty box.
        let face = match &here {
            Some((s, f)) => {
                format!("{} · {:.1}", s.name.split_whitespace().next().unwrap_or(""), f)
            }
            None => "STATIONS".to_string(),
        };
        let btn = crate::chrome::chip(ui, here.is_some(), RichText::new(face).size(11.0))
            .on_hover_text("Broadcast radiofax schedules — picking one tunes the dial");

        let mut pick = None;
        let resp = egui::Popup::from_toggle_button_response(&btn)
            .frame(crate::chrome::window_frame())
            .close_behavior(egui::PopupCloseBehavior::CloseOnClick)
            .show(|ui| {
                crate::chrome::window_body_bg(ui);
                ui.set_max_width(420.0);
                for s in WEFAX_STATIONS {
                    ui.label(
                        RichText::new(s.name).color(crate::theme::CYAN_DIM()).size(10.0).strong(),
                    );
                    ui.horizontal_wrapped(|ui| {
                        for &f in s.carriers_khz {
                            let d = WefaxStation::dial_hz(f);
                            let on = (d - dial).abs() < WefaxStation::NEAR_HZ;
                            if crate::chrome::chip(ui, on, format!("{f:.1}"))
                                .on_hover_text(format!(
                                    "Published carrier {f:.1} kHz → dial {:.1} kHz USB",
                                    d / 1000.0
                                ))
                                .clicked()
                            {
                                pick = Some(d);
                            }
                        }
                    });
                    ui.add_space(2.0);
                }
                ui.label(
                    RichText::new(
                        "Frequencies are the published carrier; the dial goes 1.9 kHz below it, \
                         which is done for you. Schedules change and stations close — treat this \
                         as where to start looking, not a timetable.",
                    )
                    .color(crate::theme::LINE_LIT())
                    .size(10.0),
                );
            });
        if let Some(r) = &resp {
            crate::chrome::paint_popup_cut_border(ui.ctx(), &r.response, 1.0);
        }
        if let Some(hz) = pick {
            cmds.push(Command::SetVfo { vfo: self.state.active_vfo, hz });
        }
    }

    /// Why the dial does not read the frequency on the station chip.
    ///
    /// A radiofax carrier is upper-sideband, so the dial sits 1.9 kHz *below*
    /// the published carrier — pick GYA at 4610.0 and the dial lands on 4608.1.
    /// That is correct and the popup says so, but the only number the operator
    /// watches is the dial, and a dial that disagrees with the chip they just
    /// clicked reads as a fault (issue #527). Show both together, with the
    /// subtraction spelled out, while a station is under the dial.
    fn wefax_carrier_note(&self, ui: &mut egui::Ui) {
        use sdroxide_types::WefaxStation;
        let dial = self.state.active_freq_hz();
        let Some((_, carrier_khz)) = WefaxStation::at_dial(dial) else { return };
        ui.label(
            RichText::new(format!(
                "carrier {carrier_khz:.1} · dial {:.1} kHz (USB −1.9k)",
                dial / 1000.0
            ))
            .size(10.0)
            .color(crate::theme::CYAN_DIM()),
        )
        .on_hover_text(
            "The station chip names the published carrier; the radiofax subcarrier sits \
             1900 Hz above it, so the dial is deliberately 1.9 kHz lower. Tune the dial to \
             the carrier itself and the signal falls outside the passband.",
        );
    }

    /// A saved chart, full size, in its own window.
    ///
    /// A weather chart is unreadable at gallery size — the whole value of it is
    /// the fronts and the isobars — so opening one properly is not optional.
    /// Stepping between charts from inside the window matters for the same
    /// reason: comparing this run against the last one is most of what the
    /// charts are for.
    fn wefax_viewer(&mut self, ctx: &egui::Context, cmds: &mut Vec<Command>) {
        let Some(i) = self.wefax.viewing else { return };
        let n = self.wefax.gallery.len();
        let Some(chart) = self.wefax.gallery.get(i) else {
            self.wefax.viewing = None;
            return;
        };
        // Two megapixels of chart live in the engine's store; the thumbnail
        // stands in until the real one arrives, so the window opens on
        // something rather than on nothing. Asked once per chart, whatever the
        // answer — a fetch that fails must not become a request every frame.
        if self.wefax.needs_full(&chart.name, chart.full.is_some()) {
            self.wefax.full_asked = Some(chart.name.clone());
            self.wefax.full_gone = false;
            cmds.push(Command::ImageGet {
                kind: sdroxide_types::ImageKind::Wefax,
                name: chart.name.clone(),
            });
        }
        let chart = &self.wefax.gallery[i];
        let (name, size) = (chart.name.clone(), chart.size);
        let loaded = chart.full.is_some();
        let tex = chart.full.clone().unwrap_or_else(|| chart.texture.clone());
        let savable = self.wefax.full_png.as_ref().is_some_and(|(n, _)| *n == name);
        let gone = self.wefax.full_gone;
        let armed = self.wefax.confirm_delete.as_deref() == Some(name.as_str());
        // A chart can be stored before the first listing has come back, so the
        // directory is not always known to name.
        let del_hint = if self.wefax.dir.is_empty() {
            "Delete this chart from the store".to_string()
        } else {
            format!("Delete this chart {}", self.store_where(&self.wefax.dir))
        };
        let title = chart.title();
        let mut open = true;
        let mut save = false;
        let mut pressed_delete = false;
        let mut step = 0i32;
        let resp = egui::Window::new(format!("{title}  ·  {}×{}", size.0, size.1))
            .id(crate::layout::salted_id(ctx, "wefax-viewer"))
            .open(&mut open)
            .frame(crate::chrome::window_frame())
            .default_size([
                crate::layout::window_w(ctx, 900.0),
                crate::layout::window_h(ctx, 640.0),
            ])
            .show(ctx, |ui| {
                crate::chrome::window_body_bg(ui);
                ui.horizontal(|ui| {
                    // Newer is up the list, older is down it.
                    if crate::chrome::chip(ui, false, "◀ NEWER")
                        .on_hover_text("The chart received after this one")
                        .clicked()
                    {
                        step = -1;
                    }
                    ui.label(
                        RichText::new(format!("{} of {n}", i + 1))
                            .color(crate::theme::CYAN_DIM())
                            .size(10.0),
                    );
                    if crate::chrome::chip(ui, false, "OLDER ▶")
                        .on_hover_text("The chart received before this one")
                        .clicked()
                    {
                        step = 1;
                    }
                    ui.add_space(8.0);
                    if gone {
                        ui.label(
                            RichText::new("no longer in the store")
                                .color(crate::theme::YELLOW())
                                .size(10.0),
                        );
                    } else if !loaded {
                        ui.label(
                            RichText::new("loading full size…")
                                .color(crate::theme::CYAN_DIM())
                                .size(10.0),
                        );
                    } else if savable
                        && crate::chrome::chip(ui, false, RichText::new("SAVE").size(9.5))
                            .on_hover_text("Save a copy of this chart on this computer")
                            .clicked()
                    {
                        save = true;
                    }
                    // Two presses, the second one red: the file goes from the
                    // radio's disk and there is nothing to undo it with.
                    let del = if armed {
                        crate::chrome::chip_accent(
                            ui,
                            true,
                            RichText::new("SURE?").size(9.5),
                            crate::theme::PINK(),
                            crate::theme::INK_ON_CYAN(),
                        )
                        .on_hover_text("Click again to delete this chart for good")
                    } else {
                        crate::chrome::chip(ui, false, RichText::new("DELETE").size(9.5))
                            .on_hover_text(&del_hint)
                    };
                    if del.clicked() {
                        pressed_delete = true;
                    }
                    ui.label(RichText::new(&name).color(crate::theme::LINE_LIT()).size(10.0))
                        .on_hover_text("The file this chart was saved as");
                });
                ui.add_space(4.0);
                egui::ScrollArea::both()
                    .id_salt("wefax-viewer-scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // Sized from the chart, not the texture: a thumbnail
                        // standing in must not shrink the window and then jump.
                        let native = egui::vec2(f32::from(size.0.max(1)), f32::from(size.1.max(1)));
                        ui.add(egui::Image::new(&tex).fit_to_exact_size(native));
                    });
            });
        if let Some(r) = &resp {
            crate::chrome::paint_window_border(ctx, &r.response);
        }
        if save {
            if let Some((name, png)) = &self.wefax.full_png {
                crate::download::save_as(name, png, crate::download::Mime::Png);
            }
        }
        // First press arms the chip, second sends it. The card stays until the
        // engine confirms the file is gone.
        if pressed_delete {
            if armed {
                self.wefax.confirm_delete = None;
                cmds.push(Command::ImageDelete {
                    kind: sdroxide_types::ImageKind::Wefax,
                    name: name.clone(),
                });
            } else {
                self.wefax.confirm_delete = Some(name.clone());
            }
        }
        if step != 0 && n > 0 {
            self.wefax.viewing = Some((i as i32 + step).clamp(0, n as i32 - 1) as usize);
            // Stepping to another chart means the outstanding fetch, if any, is
            // for one nobody is looking at any more — and an armed DELETE is
            // for a chart nobody is looking at any more either.
            self.wefax.full_asked = None;
            self.wefax.full_gone = false;
            self.wefax.confirm_delete = None;
        }
        if !open {
            self.wefax.viewing = None;
            self.wefax.full_asked = None;
            self.wefax.full_gone = false;
            self.wefax.confirm_delete = None;
        }
    }
}
