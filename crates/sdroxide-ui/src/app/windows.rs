//! The two operator overlays opened from the top bar's WIN group: the memory
//! channel list and the voice keyer.
//!
//! Both are thin views over engine state — the memories come from the radio
//! backend, the keyer slots from the audio engine — so all either does is
//! draw the list and push the [`Command`] a click means.

use eframe::egui::{self, Color32, RichText};
use sdroxide_types::{
    BURST_MS_RANGE, CTCSS_TONES, Command, DCS_CODES, MAX_OFFSET_HZ, MemoryChannel, MemoryFolder,
    MemorySort, Mode, RepeaterState, Shift, ToneMode,
};

use crate::app::SdroxideApp;
use crate::chrome::StyledCombo;

/// What a drag out of the memory list carries: the memory's id. Its own type
/// so no unrelated drag could ever be mistaken for one.
struct DraggedMemory(u32);

/// A memory being edited in place: its id and the fields as they are being
/// typed. UI-owned until the edit commits — the engine republishes the whole
/// list on every change, and a field bound straight to it would fight the
/// keyboard — exactly like `mem_folder_edit`.
pub(in crate::app) struct MemoryEdit {
    pub id: u32,
    pub name: String,
    pub freq_hz: f64,
    pub mode: Mode,
    /// The repeater setup stored with the channel — the shift, the tone under
    /// the voice and the 1750 Hz burst. `None` for a memory stored before the
    /// field existed, which is what the RPT chip turns into a real setting the
    /// first time it is opened.
    pub repeater: Option<RepeaterState>,
    /// Whether the repeater controls are unfolded. A memory is usually edited
    /// to fix a typo, and four more rows of chips under every one of those
    /// would make the common case the awkward one.
    pub show_repeater: bool,
    /// The antenna socket stored with the channel, by the name the front end
    /// gives the port — `None` for "recall this channel without moving the
    /// antenna", which is every channel on a receiver with one socket.
    ///
    /// Storing a memory captures whatever the radio was on at the time; this is
    /// where an operator says otherwise, which is the whole of issue #246. It
    /// only appears on a front end with more than one port to choose between.
    pub antenna: Option<String>,
}

impl MemoryEdit {
    fn of(m: &MemoryChannel) -> Self {
        MemoryEdit {
            id: m.id,
            name: m.name.clone(),
            freq_hz: m.freq_hz,
            mode: m.mode,
            repeater: m.repeater,
            // Open on a channel that has something to show, so a repeater
            // memory says what it is without being asked twice.
            show_repeater: m.repeater.is_some_and(|r| r.is_active()),
            antenna: m.antenna.clone(),
        }
    }
}

/// How a stored repeater setup reads in the memory list: the shift, then the
/// tone, then the burst — and nothing at all for a plain simplex channel,
/// which is most of them.
fn repeater_summary(r: Option<RepeaterState>) -> String {
    let Some(r) = r.filter(|r| r.is_active()) else { return String::new() };
    let mut parts = Vec::new();
    if r.shift != Shift::Simplex {
        parts.push(r.shift_label());
    }
    if let Some(t) = r.tx_tone() {
        parts.push(t.label());
    }
    if r.burst_auto {
        parts.push("1750".to_string());
    }
    format!(" {}", parts.join(" "))
}

/// Wrap `add` in a frame that accepts a dragged memory, and say which memory
/// was dropped on it this frame, if any. While a drag is live every target
/// shows a faint outline, and the one under the pointer answers in cyan —
/// hand-rolled rather than `dnd_drop_zone`, which repaints the frame in the
/// widget palette and would put a button-grey slab behind every folder.
fn mem_drop_target<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> (R, Option<u32>) {
    let dragging = egui::DragAndDrop::has_payload_of_type::<DraggedMemory>(ui.ctx());
    let out = egui::Frame::default().inner_margin(4.0).corner_radius(4.0).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        add(ui)
    });
    if dragging {
        let stroke = if out.response.dnd_hover_payload::<DraggedMemory>().is_some() {
            egui::Stroke::new(1.5, crate::theme::CYAN())
        } else {
            egui::Stroke::new(1.0, crate::theme::LINE_LIT())
        };
        ui.painter().rect_stroke(out.response.rect, 4.0, stroke, egui::StrokeKind::Inside);
    }
    let dropped = out.response.dnd_release_payload::<DraggedMemory>().map(|p| p.0);
    (out.inner, dropped)
}

/// One memory: recall, name/frequency/mode, edit, delete. The label is the
/// drag handle — deliberately not the whole row, whose drag sense would sit
/// over the buttons and turn a sloppy RCL or DEL press into a drag.
///
/// `edit` is the edit in progress anywhere in the list; when it is this
/// memory's, the row is replaced by the editor.
fn memory_row(
    ui: &mut egui::Ui,
    m: &MemoryChannel,
    edit: &mut Option<MemoryEdit>,
    focus: &mut bool,
    antennas: &[String],
    cmds: &mut Vec<Command>,
) {
    if matches!(edit, Some(e) if e.id == m.id) {
        memory_edit_row(ui, edit, focus, antennas, cmds);
        return;
    }
    ui.horizontal(|ui| {
        if crate::chrome::chip(ui, false, "RCL").on_hover_text("Recall").clicked() {
            cmds.push(Command::RecallMemory(m.id));
        }
        // The buttons are placed before the label rather than after it, so that
        // what is left over is the label's and it truncates inside it. Drawn
        // the other way round the label claims the width its text wants — a
        // memory list is monospace and wide — and the row's buttons end up
        // painted over the end of it.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if crate::chrome::chip_accent(
                ui,
                false,
                RichText::new("DEL").size(11.0),
                crate::theme::PINK(),
                Color32::WHITE,
            )
            .on_hover_text("Delete")
            .clicked()
            {
                cmds.push(Command::DeleteMemory(m.id));
            }
            if crate::chrome::chip(ui, false, RichText::new("EDT").size(11.0))
                .on_hover_text("Edit the name, frequency, mode, antenna and repeater setup")
                .clicked()
            {
                *edit = Some(MemoryEdit::of(m));
                *focus = true;
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.dnd_drag_source(
                    crate::layout::salted_id(ui.ctx(), "mem-drag").with(m.id),
                    DraggedMemory(m.id),
                    |ui| {
                        // An RTTY memory recalls its modem setup with it; show that
                        // setup so two memories on the same dial read as the
                        // different stations they are (f32's Display keeps 45.45
                        // as-is and 170.0 as "170").
                        let rtty = m.rtty.map_or(String::new(), |r| {
                            format!(
                                " {}/{}{}",
                                r.baud,
                                r.shift_hz,
                                if r.reverse { " R" } else { "" }
                            )
                        });
                        // …and the repeater setup, for the same reason: two
                        // memories on one dial are different channels if one
                        // shifts and the other does not.
                        let rpt = repeater_summary(m.repeater);
                        // …and the antenna, which is a third way two channels
                        // on one dial can be different channels. Nothing at all
                        // for a memory that says nothing about antennas, which
                        // is every one stored on a receiver with a single
                        // socket.
                        let ant = m.antenna.as_deref().map_or(String::new(), |a| format!(" [{a}]"));
                        // Laid out rather than added: the whole of what is left
                        // is the drag handle, and the text is pinned to the left
                        // of it so the columns line up down the list. `add_sized`
                        // would centre each row's text in its own box, which puts
                        // a short name in a different place from a long one.
                        let w = ui.available_width();
                        let h = ui.spacing().interact_size.y;
                        ui.allocate_ui_with_layout(
                            egui::vec2(w, h),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(format!(
                                            "{:<10} {:>11.6} MHz {}{}{}{}",
                                            m.name,
                                            m.freq_hz / 1e6,
                                            m.mode.label(),
                                            rtty,
                                            rpt,
                                            ant
                                        ))
                                        .monospace(),
                                    )
                                    .truncate(),
                                );
                            },
                        );
                    },
                );
            });
        });
    });
}

/// The editor that replaces a row while that memory is being edited.
///
/// In the list rather than in a dialog of its own: correcting a typo is the
/// smallest thing anyone does to a memory, and a window that has to be opened
/// and dismissed for it is most of the reason it was easier to delete the
/// channel and store it again.
///
/// Wrapped rather than one line: with a name, a dial and a mode picker on it
/// the row is wider than the window's default, and the alternative to wrapping
/// is a horizontal scrollbar under every list.
fn memory_edit_row(
    ui: &mut egui::Ui,
    edit: &mut Option<MemoryEdit>,
    focus: &mut bool,
    antennas: &[String],
    cmds: &mut Vec<Command>,
) {
    let Some(e) = edit.as_mut() else { return };
    let mut commit = false;
    let mut cancel = false;
    ui.horizontal_wrapped(|ui| {
        let name = crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut e.name).hint_text("name").desired_width(110.0),
        );
        if *focus {
            name.request_focus();
            *focus = false;
        }
        // Enter commits from the name field, the way it does from every other
        // single-line edit here. Losing focus does *not*: the operator moving
        // on to the frequency or the mode is still editing.
        if name.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            commit = true;
        }
        // What there is to save: a memory with no name is a row nobody can
        // pick out of a list, and a dial of zero is what an emptied field
        // leaves behind.
        let valid = !e.name.trim().is_empty() && e.freq_hz > 0.0;
        // Six decimals because the field is in MHz and a memory is a dial
        // frequency to the Hz — four would round 14.070150 to 14.0702 and
        // store what it showed.
        let mut mhz = e.freq_hz / 1e6;
        if crate::chrome::field(
            ui,
            egui::DragValue::new(&mut mhz)
                .speed(0.001)
                .range(0.0..=6000.0)
                .max_decimals(6)
                .fixed_decimals(6),
        )
        .changed()
        {
            e.freq_hz = mhz * 1e6;
        }
        ui.label(RichText::new("MHz").weak());
        egui::ComboBox::from_id_salt(crate::layout::salted_id(ui.ctx(), "mem-edit-mode"))
            .width(70.0)
            .selected_text(e.mode.label())
            .show_styled(ui, |ui| {
                for m in Mode::ALL {
                    ui.selectable_value(&mut e.mode, m, m.label());
                }
            });
        memory_antenna_picker(ui, e, antennas);
        // The repeater setup folds out rather than always being there: most
        // memories are a name, a dial and a mode, and this is four more rows.
        let has_rpt = e.repeater.is_some_and(|r| r.is_active());
        if crate::chrome::chip(ui, e.show_repeater || has_rpt, RichText::new("RPT").size(11.0))
            .on_hover_text("The repeater shift, tone and 1750 Hz burst stored with this channel")
            .clicked()
        {
            e.show_repeater = !e.show_repeater;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if crate::chrome::chip(ui, false, RichText::new("✖").size(11.0))
                .on_hover_text("Abandon the edit (Escape)")
                .clicked()
            {
                cancel = true;
            }
            // Greyed rather than silently doing nothing while the name is
            // empty or the dial is zero: an emptied field is a state the
            // operator is passing through, and the answer to it is to show
            // that there is nothing yet to save.
            if crate::chrome::chip_accent_enabled(
                ui,
                valid,
                false,
                "SAVE",
                Some(11.0),
                crate::theme::GREEN(),
                crate::theme::INK_ON_BRIGHT(),
            )
            .on_hover_text("Keep the changes (Enter)")
            .clicked()
            {
                commit = true;
            }
        });
    });
    if let Some(e) = edit.as_mut().filter(|e| e.show_repeater) {
        memory_repeater_rows(ui, e);
    }
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        cancel = true;
    }
    // Enter on an empty name leaves the editor open rather than throwing the
    // edit away — the same answer the greyed SAVE gives.
    if commit && let Some(e) = edit.take_if(|e| !e.name.trim().is_empty() && e.freq_hz > 0.0) {
        cmds.push(Command::EditMemory {
            id: e.id,
            name: e.name.trim().to_string(),
            freq_hz: e.freq_hz,
            mode: e.mode,
            repeater: e.repeater,
            antenna: e.antenna.clone(),
        });
    } else if cancel {
        *edit = None;
    }
}

/// The antenna picker in the memory editor: which socket this channel is
/// listened to on, or `—` for "leave the antenna alone".
///
/// Drawn only where the front end has more than one port. On a receiver with a
/// single antenna there is nothing to choose, every memory stores `None`
/// anyway, and a picker with one entry beside a dash is a control that can only
/// be set wrong.
///
/// The dash is not the same as the first port and is the default: an operator
/// working down a list of channels that never mentioned antennas must not have
/// a relay moved on every recall — see [`MemoryChannel::antenna`].
fn memory_antenna_picker(ui: &mut egui::Ui, e: &mut MemoryEdit, antennas: &[String]) {
    if antennas.len() < 2 {
        return;
    }
    egui::ComboBox::from_id_salt(crate::layout::salted_id(ui.ctx(), "mem-edit-ant"))
        .width(90.0)
        .selected_text(e.antenna.as_deref().unwrap_or("—"))
        .show_styled(ui, |ui| {
            ui.selectable_value(&mut e.antenna, None, "—")
                .on_hover_text("Recall this channel without moving the antenna");
            for a in antennas {
                ui.selectable_value(&mut e.antenna, Some(a.clone()), a);
            }
        })
        .response
        .on_hover_text(
            "The antenna to switch to when this channel is recalled. — leaves whatever the \
             radio is already on.",
        );
}

/// The repeater setup under the memory editor: the shift, the tone that goes
/// out under the voice, and the 1750 Hz burst.
///
/// Pickers rather than the grids of chips the live TONE popup uses. That popup
/// is a control panel and has a whole popup to spend; this is three rows
/// inside a list row, and 104 DCS chips in it would bury the memory being
/// edited.
///
/// Opening the section on a memory that has no setup gives it the default one
/// — plain simplex with no tone — rather than leaving it `None`. That is the
/// difference between "this channel says nothing" and "this channel says
/// simplex", and the operator who opened these controls meant the second.
fn memory_repeater_rows(ui: &mut egui::Ui, e: &mut MemoryEdit) {
    let r = e.repeater.get_or_insert_with(RepeaterState::default);
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Shift").weak().size(11.0));
        for s in Shift::ALL {
            if crate::chrome::chip(ui, r.shift == s, RichText::new(s.label()).size(11.0)).clicked()
            {
                r.shift = s;
            }
        }
        let mut khz = f64::from(r.offset_hz) / 1e3;
        if crate::chrome::field(
            ui,
            egui::DragValue::new(&mut khz)
                .speed(5.0)
                .range(0.0..=f64::from(MAX_OFFSET_HZ) / 1e3)
                .max_decimals(4)
                .suffix(" kHz"),
        )
        .changed()
        {
            r.offset_hz = (khz * 1e3).round().clamp(0.0, f64::from(MAX_OFFSET_HZ)) as u32;
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Tone").weak().size(11.0));
        for m in ToneMode::ALL {
            if crate::chrome::chip(ui, r.tone == m, RichText::new(m.label()).size(11.0)).clicked() {
                r.tone = m;
            }
        }
        match r.tone {
            ToneMode::Off => {}
            ToneMode::Ctcss => {
                egui::ComboBox::from_id_salt(crate::layout::salted_id(ui.ctx(), "mem-edit-ctcss"))
                    .width(70.0)
                    .selected_text(format!("{}.{}", r.ctcss_tenths / 10, r.ctcss_tenths % 10))
                    .show_styled(ui, |ui| {
                        for tenths in CTCSS_TONES {
                            ui.selectable_value(
                                &mut r.ctcss_tenths,
                                tenths,
                                format!("{}.{}", tenths / 10, tenths % 10),
                            );
                        }
                    });
            }
            ToneMode::Dcs => {
                egui::ComboBox::from_id_salt(crate::layout::salted_id(ui.ctx(), "mem-edit-dcs"))
                    .width(70.0)
                    .selected_text(format!("{:03}", r.dcs_code))
                    .show_styled(ui, |ui| {
                        for code in DCS_CODES {
                            ui.selectable_value(&mut r.dcs_code, code, format!("{code:03}"));
                        }
                    });
                if crate::chrome::chip(
                    ui,
                    r.dcs_invert,
                    RichText::new(if r.dcs_invert { "INVERT" } else { "NORMAL" }).size(11.0),
                )
                .on_hover_text("DCS polarity")
                .clicked()
                {
                    r.dcs_invert = !r.dcs_invert;
                }
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        if crate::chrome::chip(ui, r.burst_auto, RichText::new("1750 Hz").size(11.0))
            .on_hover_text("Open every over on the tone burst while this channel is recalled")
            .clicked()
        {
            r.burst_auto = !r.burst_auto;
        }
        if r.burst_auto {
            let mut ms = r.burst_ms;
            if crate::chrome::field(
                ui,
                egui::DragValue::new(&mut ms).speed(10).range(BURST_MS_RANGE).suffix(" ms"),
            )
            .changed()
            {
                r.burst_ms = ms;
            }
        }
    });
}

impl SdroxideApp {
    pub(in crate::app) fn memories_window(&mut self, ctx: &egui::Context, cmds: &mut Vec<Command>) {
        let mut open = self.show_memories;
        let resp = egui::Window::new("Memories")
            .id(crate::layout::salted_id(ctx, "Memories"))
            .open(&mut open)
            .frame(crate::chrome::window_frame())
            .resizable(true)
            // Wide enough for a row: the recall button, a name, a dial to the
            // Hz, a mode, and the edit and delete buttons. Narrower than this
            // and every name is drawn with an ellipsis on it.
            .default_width(crate::layout::window_w(ctx, 460.0))
            // A real starting height, and a floor under what egui remembers:
            // the window used to hug its (often short) list and come up a few
            // rows tall. See the voice keyer below for why the minimum matters
            // as much as the default.
            .default_height(crate::layout::window_h(ctx, 460.0))
            .min_height(crate::layout::window_h(ctx, 280.0))
            .show(ctx, |ui| {
                crate::chrome::window_body_bg(ui);
                self.memories_ui(ui, cmds)
            });
        if let Some(r) = &resp {
            crate::chrome::paint_window_border(ctx, &r.response);
        }
        self.show_memories = open;
    }

    fn memories_ui(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        // An edit whose memory has gone — deleted here, or from another screen
        // on the same station — has nothing left to draw itself against, and
        // would otherwise sit in the state until the window was reopened.
        if let Some(e) = &self.mem_edit
            && !self.memories.iter().any(|m| m.id == e.id)
        {
            self.mem_edit = None;
        }
        ui.horizontal(|ui| {
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut self.mem_name)
                    .hint_text("memory name")
                    .desired_width(ui.available_width() - 100.0),
            );
            let name_ok = !self.mem_name.trim().is_empty();
            if ui.add_enabled(name_ok, egui::Button::new("Store")).clicked() {
                cmds.push(Command::StoreMemory { name: self.mem_name.trim().to_string() });
                self.mem_name.clear();
            }
        });
        ui.horizontal(|ui| {
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut self.mem_folder_name)
                    .hint_text("folder name")
                    .desired_width(ui.available_width() - 100.0),
            );
            let name_ok = !self.mem_folder_name.trim().is_empty();
            if ui.add_enabled(name_ok, egui::Button::new("New folder")).clicked() {
                cmds.push(Command::CreateMemoryFolder {
                    name: self.mem_folder_name.trim().to_string(),
                });
                self.mem_folder_name.clear();
            }
        });
        // A channel list from somewhere else. CHIRP's CSV because that is what
        // every repeater directory hands out — RepeaterBook exports it by
        // county, and the marine and PMR tables circulate as it — so an
        // operator who wants their local machines in here already has the file
        // (issue #234).
        ui.horizontal(|ui| {
            if crate::chrome::chip(ui, false, "IMPORT")
                .on_hover_text(
                    "Read a channel list from a CHIRP CSV file (.csv) — a repeater \
                     directory export, a marine or PMR channel table, or anything \
                     else CHIRP can write.\n\n\
                     Each channel brings its frequency, mode, repeater shift and \
                     CTCSS or DCS tone with it. Channels already on this list — same \
                     frequency, same mode — are skipped, so re-importing an updated \
                     directory adds what is new rather than doubling what is not.",
                )
                .clicked()
            {
                crate::download::load_text("CHIRP CSV", "csv", self.chirp_import_inbox.clone());
            }
            let have = !self.memories.is_empty();
            ui.add_enabled_ui(have, |ui| {
                if crate::chrome::chip(ui, false, "EXPORT")
                    .on_hover_text(
                        "Write this list out as a CHIRP CSV file, to load into a \
                         handheld or to keep as a backup.",
                    )
                    .clicked()
                {
                    let csv = sdroxide_types::memories_to_chirp_csv(&self.memories);
                    crate::download::save("sdroxide-memories.csv", csv.as_bytes());
                }
            });
            ui.label(
                RichText::new(format!("{} channel", self.memories.len()))
                    .size(11.0)
                    .color(crate::theme::gray(150)),
            );
        });
        self.memory_sort_bar(ui);
        ui.separator();
        if self.memories.is_empty() && self.mem_folders.is_empty() {
            ui.label(RichText::new("no memories yet").color(crate::theme::gray(120)));
        } else if !self.mem_folders.is_empty() {
            ui.label(
                RichText::new("drag a memory onto a folder to file it, below them to unfile it")
                    .weak()
                    .size(11.0),
            );
        }
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            // While a memory is being dragged, holding it near the top or
            // bottom edge crawls the list, so a folder outside the visible
            // area can still be reached. Instant scrolling (no animation):
            // this runs every frame, and easing would fight the next frame's
            // delta. The repaint request keeps it crawling while the pointer
            // sits still at the edge, which is the whole gesture.
            if egui::DragAndDrop::has_payload_of_type::<DraggedMemory>(ui.ctx())
                && let Some(pos) = ui.ctx().pointer_interact_pos()
            {
                const MARGIN: f32 = 28.0;
                const SPEED: f32 = 600.0; // pt/s right at the edge
                let view = ui.clip_rect();
                if pos.x >= view.left() - 40.0 && pos.x <= view.right() + 40.0 {
                    let step = SPEED * ui.input(|i| i.stable_dt).min(0.1);
                    let delta = if pos.y < view.top() + MARGIN {
                        ((view.top() + MARGIN - pos.y) / MARGIN).min(1.0) * step
                    } else if pos.y > view.bottom() - MARGIN {
                        -((pos.y - (view.bottom() - MARGIN)) / MARGIN).min(1.0) * step
                    } else {
                        0.0
                    };
                    if delta != 0.0 {
                        ui.scroll_with_delta_animation(
                            egui::vec2(0.0, delta),
                            egui::style::ScrollAnimation::none(),
                        );
                        ui.ctx().request_repaint();
                    }
                }
            }
            // Worked out once for the whole window, so every folder and the
            // top level agree on the order and none of them sorts twice.
            let order = self
                .ui_settings
                .memory_sort
                .order(&self.memories, self.ui_settings.memory_sort_desc);
            // The sockets this front end has, for the editor's antenna picker.
            // Cloned once for the window rather than read per row: it is a
            // handful of short strings, and the rows borrow `self` mutably for
            // the edit in progress.
            let antennas: Vec<String> =
                self.caps.as_ref().map(|c| c.antennas_rx.clone()).unwrap_or_default();
            let folders = self.mem_folders.clone();
            for f in &folders {
                self.folder_section(ui, f, &order, &antennas, cmds);
            }
            // The top level: everything unfiled — including anything whose
            // folder has gone from under it — and, while a drag is live, the
            // place to drop a memory to take it out of its folder.
            let (_, dropped) = mem_drop_target(ui, |ui| {
                let known = |id: u32| folders.iter().any(|f| f.id == id);
                let mut any = false;
                for &i in &order {
                    let m = &self.memories[i];
                    if m.folder.is_none_or(|id| !known(id)) {
                        memory_row(
                            ui,
                            m,
                            &mut self.mem_edit,
                            &mut self.mem_edit_focus,
                            &antennas,
                            cmds,
                        );
                        any = true;
                    }
                }
                if !any && egui::DragAndDrop::has_payload_of_type::<DraggedMemory>(ui.ctx()) {
                    ui.label(RichText::new("drop here to unfile").weak().size(11.0));
                }
            });
            if let Some(id) = dropped
                && self.memories.iter().any(|m| m.id == id && m.folder.is_some())
            {
                cmds.push(Command::MoveMemoryToFolder { id, folder: None });
            }
        });
    }

    /// The order the list is drawn in: four chips and a direction.
    ///
    /// Remembered in `[ui]` with the rest of this screen's preferences, and
    /// applied the moment it is clicked — there is nothing to apply engine-side
    /// and nothing to send: the store keeps its channels in the order they were
    /// stored, a memory scan works through them in that order whatever this
    /// says, and the operator at the next screen sorts the same station's list
    /// their own way.
    fn memory_sort_bar(&mut self, ui: &mut egui::Ui) {
        let (was_sort, was_desc) =
            (self.ui_settings.memory_sort, self.ui_settings.memory_sort_desc);
        let (mut sort, mut desc) = (was_sort, was_desc);
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Sort").weak().size(11.0));
            for s in MemorySort::ALL {
                if crate::chrome::chip(ui, sort == s, RichText::new(s.label()).size(11.0))
                    .on_hover_text(match s {
                        MemorySort::Stored => "In the order they were stored",
                        MemorySort::Name => "By name, ignoring case",
                        MemorySort::Freq => "By frequency",
                        MemorySort::Band => "By band, then by frequency inside each",
                    })
                    .clicked()
                {
                    sort = s;
                }
            }
            if crate::chrome::chip(
                ui,
                false,
                RichText::new(if desc { "▼" } else { "▲" }).size(11.0),
            )
            .on_hover_text(if desc {
                "Descending — click for ascending"
            } else {
                "Ascending — click for descending"
            })
            .clicked()
            {
                desc = !desc;
            }
        });
        if (sort, desc) != (was_sort, was_desc) {
            self.ui_settings.memory_sort = sort;
            self.ui_settings.memory_sort_desc = desc;
            crate::app::persist::persist_ui_settings(&self.ui_settings);
        }
    }

    /// One folder of the memory list: a collapsible section whose header
    /// carries the rename and delete controls, the whole of it a drop target.
    fn folder_section(
        &mut self,
        ui: &mut egui::Ui,
        f: &MemoryFolder,
        order: &[usize],
        antennas: &[String],
        cmds: &mut Vec<Command>,
    ) {
        let count = self.memories.iter().filter(|m| m.folder == Some(f.id)).count();
        let (_, dropped) = mem_drop_target(ui, |ui| {
            let state = egui::collapsing_header::CollapsingState::load_with_default_open(
                ui.ctx(),
                ui.make_persistent_id(("mem-folder", f.id)),
                true,
            );
            state
                .show_header(ui, |ui| {
                    let editing = matches!(&self.mem_folder_edit, Some((id, _)) if *id == f.id);
                    if editing {
                        let (_, text) = self.mem_folder_edit.as_mut().expect("checked above");
                        let edit = crate::chrome::field(
                            ui,
                            egui::TextEdit::singleline(text).desired_width(150.0),
                        );
                        if self.mem_folder_focus {
                            edit.request_focus();
                            self.mem_folder_focus = false;
                        }
                        // Escape abandons the edit; Enter or clicking away
                        // commits it (Enter surrenders focus in egui).
                        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            self.mem_folder_edit = None;
                        } else if edit.lost_focus() {
                            let (id, name) = self.mem_folder_edit.take().expect("checked above");
                            let name = name.trim().to_string();
                            if !name.is_empty() && name != f.name {
                                cmds.push(Command::RenameMemoryFolder { id, name });
                            }
                        }
                    } else {
                        ui.label(RichText::new(&f.name).strong());
                        ui.label(RichText::new(format!("({count})")).weak().size(11.0));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if crate::chrome::chip_accent(
                            ui,
                            false,
                            RichText::new("DEL").size(11.0),
                            crate::theme::PINK(),
                            Color32::WHITE,
                        )
                        .on_hover_text("Delete folder — its memories move to the top level")
                        .clicked()
                        {
                            cmds.push(Command::DeleteMemoryFolder(f.id));
                        }
                        if crate::chrome::chip(ui, false, RichText::new("REN").size(11.0))
                            .on_hover_text("Rename folder")
                            .clicked()
                        {
                            self.mem_folder_edit = Some((f.id, f.name.clone()));
                            self.mem_folder_focus = true;
                        }
                    });
                })
                .body(|ui| {
                    if count == 0 {
                        ui.label(RichText::new("empty — drop memories here").weak().size(11.0));
                    }
                    for &i in order {
                        let m = &self.memories[i];
                        if m.folder == Some(f.id) {
                            memory_row(
                                ui,
                                m,
                                &mut self.mem_edit,
                                &mut self.mem_edit_focus,
                                antennas,
                                cmds,
                            );
                        }
                    }
                });
        });
        if let Some(id) = dropped
            && self.memories.iter().any(|m| m.id == id && m.folder != Some(f.id))
        {
            cmds.push(Command::MoveMemoryToFolder { id, folder: Some(f.id) });
        }
    }

    /// The voice keyer: ten recorded messages with record / transmit / erase
    /// per slot.
    ///
    /// Everything the window shows comes from the engine (it owns the
    /// recordings and the transmitter), so the buttons only ever send commands
    /// — there is no local latch that could disagree with what is on the air.
    pub(in crate::app) fn voice_window(&mut self, ctx: &egui::Context, cmds: &mut Vec<Command>) {
        // Entering a digital mode other than RADE takes the feature away; the
        // window goes with it rather than sitting there doing nothing.
        if !self.state.rx[0].mode.allows_voice_keyer() {
            self.show_voice = false;
            return;
        }
        let mut open = self.show_voice;
        let recording = self.voice.recording;
        let playing = self.voice.playing;
        let previewing = self.voice.previewing;
        let pos = self.voice.position_s;
        let max_len = self.voice.max_len_s;
        // TUNE holds the transmitter at the tune level, so a message would go
        // nowhere; the engine refuses, and the buttons say so up front.
        let tuning = self.state.tx.tune;
        let slots: Vec<sdroxide_types::VoiceSlotInfo> = self.voice.slots.clone();

        let resp = egui::Window::new("Voice keyer")
            .id(crate::layout::salted_id(ctx, "Voice keyer"))
            .open(&mut open)
            .frame(crate::chrome::window_frame())
            .resizable(false)
            // `min_width` as well as `default_width`: the default only applies
            // the first time the window is ever shown, and egui persists its
            // size — without the minimum, a build that shipped a narrower
            // window would keep squeezing the slot-name fields forever.
            //
            // Both are held inside the viewport, for the same reason in
            // reverse: a keyer opened at 600 pt on a desktop would otherwise
            // stay 600 pt wide on the phone that later loads the same storage,
            // with a third of it off the side of the screen.
            .default_width(crate::layout::window_w(ctx, 600.0))
            .min_width(crate::layout::window_w(ctx, 600.0))
            .show(ctx, |ui| {
                crate::chrome::window_body_bg(ui);
                ui.label(
                    RichText::new(
                        "REC records from your microphone, PLAY lets you listen to what you \
                         recorded, TX puts it on the air — as does a numpad key, a MIDI pad, \
                         or rigctld's send_voice_mem.",
                    )
                    .weak()
                    .size(11.5),
                );
                ui.add_space(6.0);
                egui::Grid::new("voice-grid")
                    .num_columns(6)
                    .spacing([8.0, 6.0])
                    .striped(true)
                    .show(ui, |ui| {
                        for (i, slot) in slots.iter().enumerate() {
                            let is_rec = recording == Some(i as u8);
                            let is_play = playing == Some(i as u8);
                            let is_prev = previewing == Some(i as u8);

                            ui.label(
                                RichText::new(format!("{:>2}", i + 1))
                                    .monospace()
                                    .color(crate::theme::CYAN_DIM()),
                            );

                            // The slot label. Only the row being typed into is
                            // UI-owned; every other row shows the engine's copy.
                            let mut text = match &self.voice_name_edit {
                                Some((row, s)) if *row == i => s.clone(),
                                _ => slot.name.clone(),
                            };
                            // `add_sized`, not `desired_width`: inside a Grid a
                            // desired width is clamped by the column width egui
                            // measured (and persisted) last frame, so a field
                            // that once came up narrow would stay narrow.
                            let edit = crate::chrome::field_sized(
                                ui,
                                [190.0, 20.0],
                                egui::TextEdit::singleline(&mut text)
                                    .hint_text(format!("Slot {}", i + 1)),
                            );
                            if edit.changed() {
                                self.voice_name_edit = Some((i, text.clone()));
                            }
                            if edit.lost_focus()
                                && let Some((row, name)) = self.voice_name_edit.take()
                                && row == i
                            {
                                cmds.push(Command::VoiceRename { slot: i as u8, name });
                            }

                            // REC — starts/stops recording this slot. Refused
                            // while the transmitter is up (same microphone).
                            let busy_elsewhere = (recording.is_some() && !is_rec)
                                || playing.is_some()
                                || previewing.is_some()
                                || self.state.tx.ptt
                                || tuning;
                            let rec = ui
                                .add_enabled_ui(!busy_elsewhere, |ui| {
                                    crate::chrome::chip_accent(
                                        ui,
                                        is_rec,
                                        RichText::new("REC").size(11.5),
                                        crate::theme::ALERT(),
                                        Color32::WHITE,
                                    )
                                })
                                .inner
                                .on_hover_text(if is_rec {
                                    "Stop and store".to_string()
                                } else {
                                    format!("Record from the microphone (up to {max_len:.0} s)")
                                });
                            if rec.clicked() {
                                cmds.push(Command::VoiceRecord(if is_rec {
                                    None
                                } else {
                                    Some(i as u8)
                                }));
                            }

                            // PLAY — listen to the message locally. Nothing goes
                            // on the air, so this is safe to press any time the
                            // receiver is running.
                            let can_prev = !slot.is_empty()
                                && recording.is_none()
                                && !self.state.tx.ptt
                                && !tuning
                                && (is_prev || previewing.is_none());
                            let prev = ui
                                .add_enabled_ui(can_prev || is_prev, |ui| {
                                    crate::chrome::chip(
                                        ui,
                                        is_prev,
                                        RichText::new(if is_prev { "STOP" } else { "PLAY" })
                                            .size(11.5),
                                    )
                                })
                                .inner
                                .on_hover_text(if is_prev {
                                    "Stop listening"
                                } else if slot.is_empty() {
                                    "Nothing recorded in this slot"
                                } else if self.state.tx.ptt || tuning {
                                    "Not while transmitting"
                                } else {
                                    "Listen to this message — nothing is transmitted"
                                });
                            if prev.clicked() {
                                cmds.push(if is_prev {
                                    Command::VoicePreview(None)
                                } else {
                                    Command::VoicePreview(Some(i as u8))
                                });
                            }

                            // TX — puts the message on the air.
                            let can_play = !slot.is_empty()
                                && recording.is_none()
                                && !tuning
                                && (is_play || playing.is_none());
                            let play = ui
                                .add_enabled_ui(can_play || is_play, |ui| {
                                    crate::chrome::chip_accent(
                                        ui,
                                        is_play,
                                        RichText::new(if is_play { "STOP" } else { "TX" })
                                            .size(11.5),
                                        crate::theme::ALERT(),
                                        Color32::WHITE,
                                    )
                                })
                                .inner
                                .on_hover_text(if is_play {
                                    "Stop transmitting"
                                } else if slot.is_empty() {
                                    "Nothing recorded in this slot"
                                } else if tuning {
                                    "TUNE is active — switch it off first"
                                } else {
                                    "Transmit this message"
                                });
                            if play.clicked() {
                                cmds.push(if is_play {
                                    Command::VoicePlay(None)
                                } else {
                                    Command::VoicePlay(Some(i as u8))
                                });
                            }

                            // Length, or the running position of whichever of
                            // record / listen / transmit this row owns.
                            ui.horizontal(|ui| {
                                let (text, colour) = if is_rec {
                                    (format!("● {pos:.1} s"), crate::theme::ALERT())
                                } else if is_play {
                                    (
                                        format!("▶ {pos:.1} / {:.1} s", slot.len_s),
                                        crate::theme::ALERT(),
                                    )
                                } else if is_prev {
                                    (
                                        format!("▶ {pos:.1} / {:.1} s", slot.len_s),
                                        crate::theme::CYAN(),
                                    )
                                } else if slot.is_empty() {
                                    ("—".to_string(), crate::theme::gray(110))
                                } else {
                                    (format!("{:.1} s", slot.len_s), crate::theme::gray(170))
                                };
                                ui.add_sized(
                                    [88.0, 18.0],
                                    egui::Label::new(
                                        RichText::new(text).monospace().size(11.5).color(colour),
                                    )
                                    .selectable(false),
                                );
                                let erasable = !slot.is_empty() && !is_rec && !is_play && !is_prev;
                                if ui
                                    .add_enabled_ui(erasable, |ui| {
                                        crate::chrome::chip_accent(
                                            ui,
                                            false,
                                            RichText::new("DEL").size(11.0),
                                            crate::theme::PINK(),
                                            Color32::WHITE,
                                        )
                                    })
                                    .inner
                                    .on_hover_text("Erase this recording")
                                    .clicked()
                                {
                                    cmds.push(Command::VoiceClear(i as u8));
                                }
                            });
                            ui.end_row();
                        }
                    });

                if self.state.rx[0].mode.is_rade() {
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(
                            "RADE: the message is encoded by the digital-voice codec, \
                             exactly as a live over would be.",
                        )
                        .weak()
                        .size(11.0),
                    );
                }
            });
        if let Some(r) = &resp {
            crate::chrome::paint_window_border(ctx, &r.response);
        }
        // Keep the position readout moving while something is running; the app
        // otherwise idles between spectrum frames.
        if self.voice.busy() {
            crate::repaint::after_ms(ctx, 100);
        }
        self.show_voice = open;
    }
}
