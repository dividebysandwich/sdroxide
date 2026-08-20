//! The logbook: the QSO list, the entry form, and the caches over the log.
//!
//! [`LogEditForm`] holds a new or edited entry as text so partial input never
//! fights the operator; it is parsed into a [`QsoRecord`] on save. The awards
//! and worked-entity tallies are cached by log length, because the decode list
//! asks them what would be new on every row of every slot.

use eframe::egui::{self, Color32, RichText};
use sdroxide_types::{LookupProvider, QsoRecord, UploadTarget};

use crate::theme::ThemedScroll;
use crate::time::now_unix;

use crate::app::SdroxideApp;
use crate::app::net::configured_upload_targets;
use crate::app::persist::persist_qso_log;
use crate::app::util::{date_str, parse_utc, time_str};

/// Editable text fields for a manual logbook entry (new or edit). Kept as
/// strings so partial input doesn't fight the user; parsed on save. On edit,
/// `base` holds the original record so fields not shown in the form (QSL flags,
/// resolved DXCC/zones, …) survive a save.
#[derive(Default)]
pub(in crate::app) struct LogEditForm {
    /// 0 = new entry; otherwise the id of the record being edited.
    pub(in crate::app) id: u64,
    /// Timestamp fallback if the date/time fields don't parse.
    pub(in crate::app) seed_utc: i64,
    /// Original record (edit) or default (new); preserves untouched fields.
    pub(in crate::app) base: QsoRecord,
    pub(in crate::app) call: String,
    pub(in crate::app) grid: String,
    pub(in crate::app) freq_mhz: String,
    pub(in crate::app) mode: String,
    pub(in crate::app) rst_sent: String,
    pub(in crate::app) rst_rcvd: String,
    pub(in crate::app) date: String,
    pub(in crate::app) time: String,
    pub(in crate::app) name: String,
    pub(in crate::app) qth: String,
    pub(in crate::app) state: String,
    pub(in crate::app) country: String,
    pub(in crate::app) tx_pwr: String,
    pub(in crate::app) contest_id: String,
    pub(in crate::app) srx: String,
    pub(in crate::app) stx: String,
    pub(in crate::app) comment: String,
}

impl LogEditForm {
    /// A blank new entry seeded with the current time, band, and mode.
    pub(in crate::app) fn new_entry(now: i64, freq_hz: f64, mode: &str) -> LogEditForm {
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

    pub(in crate::app) fn from_record(r: &QsoRecord) -> LogEditForm {
        let (y, mo, d, h, mi, _) = sdroxide_types::utc_ymd_hms(r.start_utc);
        LogEditForm {
            id: r.id,
            seed_utc: r.start_utc,
            base: r.clone(),
            call: r.call.clone(),
            grid: r.grid.clone().unwrap_or_default(),
            freq_mhz: if r.freq_hz > 0.0 {
                format!("{:.4}", r.freq_hz / 1e6)
            } else {
                String::new()
            },
            mode: r.mode.clone(),
            rst_sent: r.rst_sent.map(|v| v.to_string()).unwrap_or_default(),
            rst_rcvd: r.rst_rcvd.map(|v| v.to_string()).unwrap_or_default(),
            date: format!("{y:04}-{mo:02}-{d:02}"),
            time: format!("{h:02}:{mi:02}"),
            name: r.name.clone(),
            qth: r.qth.clone(),
            state: r.state.clone(),
            country: r.country.clone(),
            tx_pwr: r.tx_pwr.map(|v| v.to_string()).unwrap_or_default(),
            contest_id: r.contest_id.clone(),
            srx: r.srx.map(|v| v.to_string()).unwrap_or_default(),
            stx: r.stx.map(|v| v.to_string()).unwrap_or_default(),
            comment: r.comment.clone(),
        }
    }

    /// Parse into a record, or `None` if the callsign is empty. Starts from
    /// `base` so unshown fields are preserved across an edit.
    pub(in crate::app) fn to_record(&self, my_call: &str, my_grid: &str) -> Option<QsoRecord> {
        let call = self.call.trim().to_uppercase();
        if call.is_empty() {
            return None;
        }
        let freq_hz = self.freq_mhz.trim().parse::<f64>().ok().map(|m| m * 1e6).unwrap_or(0.0);
        let band = if freq_hz > 0.0 {
            sdroxide_types::adif_band(freq_hz).to_string()
        } else {
            String::new()
        };
        let start = parse_utc(&self.date, &self.time, self.seed_utc);
        let mode = {
            let m = self.mode.trim().to_uppercase();
            if m.is_empty() { "SSB".into() } else { m }
        };
        let mut rec = self.base.clone();
        rec.id = self.id;
        rec.call = call;
        rec.grid = {
            let g = self.grid.trim();
            (!g.is_empty()).then(|| g.to_uppercase())
        };
        rec.rst_sent = self.rst_sent.trim().parse().ok();
        rec.rst_rcvd = self.rst_rcvd.trim().parse().ok();
        rec.freq_hz = freq_hz;
        rec.mode = mode;
        rec.band = band;
        rec.start_utc = start;
        rec.end_utc = if rec.end_utc > start { rec.end_utc } else { start };
        rec.my_call = my_call.to_string();
        rec.my_grid = my_grid.to_string();
        rec.name = self.name.trim().to_string();
        rec.qth = self.qth.trim().to_string();
        rec.state = self.state.trim().to_uppercase();
        rec.country = self.country.trim().to_string();
        rec.tx_pwr = self.tx_pwr.trim().parse().ok();
        rec.contest_id = self.contest_id.trim().to_uppercase();
        rec.srx = self.srx.trim().parse().ok();
        rec.stx = self.stx.trim().parse().ok();
        rec.comment = self.comment.trim().to_string();
        Some(rec)
    }
}

/// The height, in points, of every item in the logbook list: a QSO row and a day
/// header alike.
///
/// One height for both kinds is what makes the list virtualisable with
/// `ScrollArea::show_rows`, which works out which items the viewport covers from
/// this number alone, without laying any of them out. The alternative,
/// `show_viewport` with prefix-summed per-item offsets, would keep the header at
/// its natural smaller height, and was rejected: its correctness depends on
/// predicting egui's layout to the pixel for every item, and a few points of
/// drift per item, over the tens of thousands this exists to cope with, is a
/// scroll bar that lies about where it is.
///
/// The cost is that a day header now occupies the same vertical slot as a row,
/// so there is a little more space around the date than there used to be.
///
/// ⚠️ It is a FUNCTION and not a constant, and that is the whole point. The
/// fixed parts are the frame's 5-point top and bottom margins, its 22-point
/// content box, and the 2-point gap after it. The part that is not fixed is the
/// item spacing egui inserts between them, which `theme::apply_metrics` sets to
/// 5 on desktop and 7 on touch and RE-APPLIES whenever the window crosses a
/// breakpoint. A hardcoded 34 was written here first and
/// `row_height_matches_the_live_style` failed it immediately, which is exactly
/// what that test is for: with a constant, resizing the window across the touch
/// breakpoint would have silently desynchronised the scroll bar from the list.
pub(in crate::app) fn row_height(ui: &egui::Ui) -> f32 {
    // The row's own 22-point minimum, unless the style's interaction size is
    // larger, which it is on a touch tier: 34 against 18. Then the frame's
    // 5-point top and bottom margins, the 2-point gap after it, and the item
    // spacing egui inserts, which is 5 on desktop and 7 on touch.
    22.0f32.max(ui.spacing().interact_size.y) + 12.0 + ui.spacing().item_spacing.y
}

/// One entry in the flat display list the logbook scrolls through.
#[derive(Clone, Copy)]
pub(in crate::app) enum LogItem {
    /// Index into [`LogView::groups`].
    Header(usize),
    /// Index into [`LogView::order`].
    Qso(usize),
}

/// What the operator pressed on a QSO row.
enum RowAction {
    Edit(u64),
    Delete(u64),
    Upload(u64),
}

/// One day's worth of QSOs in [`LogView::order`], as a half-open range plus the
/// three values the session header prints.
pub(in crate::app) struct LogGroup {
    pub day: String,
    pub oldest: i64,
    pub newest: i64,
    pub start: usize,
    pub end: usize,
}

/// Cached newest-first ordering and day grouping of the QSO log.
///
/// The logbook list used to sort every index and re-derive every day boundary
/// on EVERY FRAME the window was open. That is invisible at a few hundred QSOs
/// and crippling at a real logbook: measured at 22,185 QSOs on a Raspberry Pi
/// 500, the sort plus the grouping alone cost 11.3 ms per frame, capping the
/// UI at 88 fps before a single row was drawn, and the waterfall shares that
/// thread. The grouping was the more wasteful half: it called `date_str` about
/// twice per QSO, roughly 44,000 heap-allocated `String`s a frame, purely to
/// find boundaries that had not moved since the log was loaded.
#[derive(Default)]
pub(in crate::app) struct LogView {
    pub order: Vec<usize>,
    pub groups: Vec<LogGroup>,
    /// Headers and rows flattened into one list, which is what the virtualised
    /// scroll area indexes: it asks for items 900 to 920, not for a group.
    pub items: Vec<LogItem>,
    /// What the cache was built from. See [`Self::refresh`].
    signature: Option<(usize, u64)>,
}

impl LogView {
    /// Rebuild the ordering and grouping if, and only if, the log has changed.
    ///
    /// Change is detected by hashing the log rather than by a generation counter
    /// bumped at each mutation site. A counter is cheaper still, and it was
    /// rejected deliberately: the log is mutated from several places (import,
    /// delete, edit, a QSO logged by the engine) and a site that forgets to bump
    /// it shows up as a list that is silently stale, which is a worse fault than
    /// the slowness this fixes and a much harder one to notice. The hash cannot
    /// go stale by omission.
    ///
    /// It folds `id` and `start_utc`, which together cover everything the view
    /// depends on: `id` catches an addition or a deletion, `start_utc` catches
    /// an edit that moves a QSO to another day. A change to a field the view
    /// does not order or group by, a comment say, does not invalidate it and
    /// does not need to, because those are read from `qso_log` at draw time.
    ///
    /// The fold is O(n) with no allocation. Measured at 22,185 QSOs on a
    /// Raspberry Pi 500: 0.124 ms, against the 11.3 ms of sorting and grouping
    /// it replaces on an unchanged log, so roughly ninety times cheaper.
    pub fn refresh(&mut self, log: &[QsoRecord]) {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for r in log {
            h ^= r.id;
            h = h.wrapping_mul(0x1000_0000_01b3);
            h ^= r.start_utc as u64;
            h = h.wrapping_mul(0x1000_0000_01b3);
        }
        let sig = (log.len(), h);
        if self.signature == Some(sig) {
            return;
        }
        self.signature = Some(sig);

        self.order.clear();
        self.order.extend(0..log.len());
        self.order.sort_by(|&a, &b| log[b].start_utc.cmp(&log[a].start_utc));

        self.groups.clear();
        let mut i = 0;
        while i < self.order.len() {
            let day = date_str(log[self.order[i]].start_utc);
            let mut j = i;
            // `date_str` is called once per QSO here rather than twice, by
            // comparing against the day already computed for the group.
            while j < self.order.len() && date_str(log[self.order[j]].start_utc) == day {
                j += 1;
            }
            self.groups.push(LogGroup {
                day,
                oldest: log[self.order[j - 1]].start_utc,
                newest: log[self.order[i]].start_utc,
                start: i,
                end: j,
            });
            i = j;
        }

        self.items.clear();
        self.items.reserve(self.order.len() + self.groups.len());
        for (gi, g) in self.groups.iter().enumerate() {
            self.items.push(LogItem::Header(gi));
            self.items.extend((g.start..g.end).map(LogItem::Qso));
        }
    }
}

impl SdroxideApp {
    /// The operator's grid square. Prefers the engine's copy but falls back to
    /// the UI's edit buffer: `digi_status` only arrives once the engine sends
    /// its first `DigiStatus`, and never at all in sessions with no digi engine.
    pub(in crate::app) fn my_grid(&self) -> String {
        self.digi_status
            .as_ref()
            .map(|s| s.config.my_grid.clone())
            .filter(|g| !g.is_empty())
            .unwrap_or_else(|| self.digi_cfg_edit.my_grid.clone())
    }

    /// Next free logbook id.
    pub(in crate::app) fn next_log_id(&self) -> u64 {
        self.qso_log.iter().map(|q| q.id).max().unwrap_or(0) + 1
    }

    /// Match downloaded QSL confirmations against the log by call + band (and,
    /// when both have one, mode) within a day, and OR the confirmation flags
    /// onto the local record.
    pub(in crate::app) fn apply_confirmations(&mut self, recs: Vec<QsoRecord>) {
        let mut matched = 0usize;
        let mut changed = false;
        for c in &recs {
            if c.call.trim().is_empty() {
                continue;
            }
            if let Some(local) = self.qso_log.iter_mut().find(|q| {
                q.call.eq_ignore_ascii_case(&c.call)
                    && q.band.eq_ignore_ascii_case(&c.band)
                    && (c.mode.is_empty() || q.mode.eq_ignore_ascii_case(&c.mode))
                    && (q.start_utc - c.start_utc).abs() < 86_400
            }) {
                let before = (local.lotw_rcvd, local.eqsl_rcvd, local.qsl_rcvd);
                local.lotw_rcvd |= c.lotw_rcvd;
                local.eqsl_rcvd |= c.eqsl_rcvd;
                local.qsl_rcvd |= c.qsl_rcvd;
                if before != (local.lotw_rcvd, local.eqsl_rcvd, local.qsl_rcvd) {
                    changed = true;
                    matched += 1;
                }
            }
        }
        if changed {
            persist_qso_log(&self.qso_log);
            self.log_content_changed();
        }
        self.push_net_log(format!(
            "Confirmations: {} downloaded, {matched} newly confirmed",
            recs.len()
        ));
    }

    /// Drop everything derived from the logbook.
    ///
    /// The caches below key on the log's *length*, which catches a QSO being
    /// added or deleted but not one being edited in place — and a confirmation
    /// arriving, or a lookup filling in a grid, is exactly that. Without this
    /// the awards tally (and the globe's heat layer, which is the same tally
    /// placed on the Earth) would keep showing yesterday's answer until the
    /// next QSO happened to change the length.
    pub(in crate::app) fn log_content_changed(&mut self) {
        self.awards_cache = None;
        self.awards_heat = None;
        self.worked_entities_cache = None;
        self.log_index_cache = None;
    }

    /// The set of worked DXCC entity names (cached; recomputed when the log
    /// length changes). Used to flag "new entity" spots.
    pub(in crate::app) fn worked_entities(&mut self) -> &std::collections::HashSet<String> {
        let len = self.qso_log.len();
        let stale = self.worked_entities_cache.as_ref().map(|(l, _)| *l != len).unwrap_or(true);
        if stale {
            let set: std::collections::HashSet<String> = self
                .qso_log
                .iter()
                .filter_map(|q| sdroxide_types::entity_name(&q.call).map(str::to_string))
                .collect();
            self.worked_entities_cache = Some((len, set));
        }
        &self.worked_entities_cache.as_ref().unwrap().1
    }

    /// Membership sets over the log (cached; rebuilt when the log length
    /// changes), so every decode row can be judged new-or-dupe for free.
    pub(in crate::app) fn log_index(&mut self) -> &sdroxide_types::LogIndex {
        let len = self.qso_log.len();
        if self.log_index_cache.as_ref().map(|(l, _)| *l != len).unwrap_or(true) {
            self.log_index_cache = Some((len, sdroxide_types::LogIndex::build(&self.qso_log)));
        }
        &self.log_index_cache.as_ref().unwrap().1
    }

    /// The logbook overlay: a session-grouped list of all QSOs (digital and
    /// manual), with add / edit / delete and ADIF/TXT export.
    pub(in crate::app) fn logbook_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_logbook;
        let resp = egui::Window::new("LOGBOOK")
            .id(crate::layout::salted_id(ctx, "LOGBOOK"))
            .open(&mut open)
            .frame(crate::chrome::window_frame())
            .resizable(true)
            .default_width(crate::layout::window_w(ctx, 720.0))
            .default_height(crate::layout::window_h(ctx, 560.0))
            .show(ctx, |ui| {
                crate::chrome::window_body_bg(ui);
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
                        #[cfg(not(target_arch = "wasm32"))]
                        if crate::chrome::chip(ui, false, "IMPORT")
                            .on_hover_text("Import QSOs from an ADIF (.adi) file")
                            .clicked()
                        {
                            crate::download::load_text(
                                "ADIF",
                                "adi",
                                self.adif_import_inbox.clone(),
                            );
                        }
                        ui.label(
                            RichText::new(format!("{} QSO", self.qso_log.len()))
                                .size(11.0)
                                .color(crate::theme::gray(150)),
                        );
                    });
                });
                if self.log_edit.is_some() {
                    ui.add_space(4.0);
                    self.log_entry_form(ui);
                }
                ui.separator();
                // Virtualised: show_rows lays out only the items the viewport
                // covers, so a 22,000 QSO log costs the same per frame as a
                // twenty QSO one. It needs the item count up front, which is
                // why the view is refreshed before the scroll area rather than
                // inside it.
                self.log_view.refresh(&self.qso_log);
                let n_items = self.log_view.items.len();
                let rh = row_height(ui);
                egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows_themed(
                    ui,
                    rh,
                    n_items,
                    |ui, rows| self.log_list(ui, rows),
                );
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
        // "Worked before" (same call + band) — computed before the mutable
        // borrow of the form below, against the current log.
        let (dupe, dupe_band) = {
            let f = self.log_edit.as_ref().unwrap();
            let freq_hz = f.freq_mhz.trim().parse::<f64>().ok().map(|m| m * 1e6).unwrap_or(0.0);
            let band = if freq_hz > 0.0 {
                sdroxide_types::adif_band(freq_hz).to_string()
            } else {
                String::new()
            };
            let dupe = !band.is_empty()
                && !f.call.trim().is_empty()
                && sdroxide_types::worked_before(&self.qso_log, f.call.trim(), &band, "", f.id);
            (dupe, band)
        };
        let auto_lookup = self.net_cfg_edit.auto_lookup;
        let has_provider = self.net_cfg_edit.lookup_provider != LookupProvider::None;
        let mut lookup_call: Option<String> = None;
        {
            let f = self.log_edit.as_mut().unwrap();
            egui::Frame::new()
                .fill(crate::theme::ROW_BG())
                .stroke(egui::Stroke::new(1.0, crate::theme::RED_DEEP()))
                .inner_margin(egui::Margin::same(9))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(if f.id == 0 { "NEW QSO" } else { "EDIT QSO" })
                                .size(11.0)
                                .strong()
                                .color(crate::theme::CYAN()),
                        );
                        if dupe {
                            ui.label(
                                RichText::new(format!("⚠ WORKED BEFORE ({dupe_band})"))
                                    .size(11.0)
                                    .strong()
                                    .color(crate::theme::PINK()),
                            );
                        }
                    });
                    ui.add_space(4.0);
                    // Horizontal rows (not a Grid) so each field keeps its
                    // explicit width — a Grid redistributes column widths and
                    // squashes the narrow-looking ones.
                    let lbl = |ui: &mut egui::Ui, text: &str| {
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(72.0, 24.0), egui::Sense::hover());
                        ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(rect)
                                .layout(egui::Layout::left_to_right(egui::Align::Center)),
                        )
                        .label(text);
                    };
                    let field = |ui: &mut egui::Ui, w: f32, s: &mut String| {
                        ui.add(egui::TextEdit::singleline(s).desired_width(w));
                    };
                    ui.horizontal(|ui| {
                        lbl(ui, "Call");
                        let cr =
                            ui.add(egui::TextEdit::singleline(&mut f.call).desired_width(150.0));
                        if has_provider
                            && crate::chrome::chip(ui, false, "LOOKUP")
                                .on_hover_text("Look up name / QTH / grid")
                                .clicked()
                            && !f.call.trim().is_empty()
                        {
                            lookup_call = Some(f.call.trim().to_string());
                        }
                        lbl(ui, "Grid");
                        field(ui, 110.0, &mut f.grid);
                        // Auto-lookup when the call field loses focus.
                        if cr.lost_focus()
                            && auto_lookup
                            && has_provider
                            && !f.call.trim().is_empty()
                        {
                            lookup_call = Some(f.call.trim().to_string());
                        }
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
                        lbl(ui, "Name");
                        field(ui, 150.0, &mut f.name);
                        lbl(ui, "QTH");
                        field(ui, 120.0, &mut f.qth);
                    });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        lbl(ui, "State");
                        field(ui, 150.0, &mut f.state);
                        lbl(ui, "Country");
                        field(ui, 120.0, &mut f.country);
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
                        lbl(ui, "Pwr W");
                        field(ui, 60.0, &mut f.tx_pwr);
                        lbl(ui, "Contest");
                        field(ui, 96.0, &mut f.contest_id);
                        lbl(ui, "S# sent");
                        field(ui, 56.0, &mut f.stx);
                        lbl(ui, "S# rcvd");
                        field(ui, 56.0, &mut f.srx);
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
                            crate::theme::GREEN(),
                            crate::theme::INK_ON_CYAN(),
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
        if let Some(c) = lookup_call {
            self.pending_lookups.push(c);
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
                            // A hand-entered contact is one worked this session
                            // too; an ADIF import is not, and does not count.
                            self.session_qsos += 1;
                        } else if let Some(e) = self.qso_log.iter_mut().find(|q| q.id == rec.id) {
                            // An edit can change the call, the grid or the QSL
                            // flags, none of which move the log's length.
                            *e = rec;
                            self.log_content_changed();
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

    /// Draw one day's session header into the slot the list has allocated.
    ///
    /// Split out of `log_list` when the list was virtualised: only visible items
    /// are drawn now, so each kind of item has to be drawable on its own.
    fn log_day_header(ui: &mut egui::Ui, g: &LogGroup, count: usize) {
        let newest = g.newest;
        let oldest = g.oldest;
        // Session header.
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(&g.day).size(12.0).strong().color(crate::theme::CYAN()));
            ui.label(
                RichText::new(format!(
                    "{}–{} UTC · {} QSO",
                    time_str(oldest),
                    time_str(newest),
                    count
                ))
                .size(10.5)
                .color(crate::theme::gray(130)),
            );
        });
        ui.add_space(2.0);
    }

    /// Draw one QSO row, returning whatever the operator pressed on it.
    ///
    /// Returns rather than mutating, because the caller holds `&self.qso_log`
    /// while drawing and cannot also take the mutable borrow the actions need.
    fn log_qso_row(
        ui: &mut egui::Ui,
        r: &QsoRecord,
        up_targets: &[UploadTarget],
    ) -> Option<RowAction> {
        let mut to_edit: Option<u64> = None;
        let mut to_delete: Option<u64> = None;
        let mut to_upload: Option<u64> = None;
        let inner = egui::Frame::new()
            .fill(crate::theme::ROW_BG())
            .inner_margin(egui::Margin { left: 10, right: 6, top: 5, bottom: 5 })
            .show(ui, |ui| {
                ui.set_min_height(22.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    let col = |ui: &mut egui::Ui, w: f32, lbl: egui::Label| {
                        let (rect, _) = ui
                            .allocate_exact_size(egui::vec2(w, 20.0), egui::Sense::hover());
                        let mut c = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(rect)
                                .layout(egui::Layout::left_to_right(egui::Align::Center)),
                        );
                        c.add(lbl);
                    };
                    let gray = crate::theme::gray(150);
                    col(
                        ui,
                        40.0,
                        egui::Label::new(
                            RichText::new(time_str(r.start_utc))
                                .monospace()
                                .size(12.0)
                                .color(gray),
                        ),
                    );
                    col(
                        ui,
                        92.0,
                        egui::Label::new(
                            RichText::new(&r.call)
                                .size(14.0)
                                .strong()
                                .color(crate::theme::TEXT_STRONG()),
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
                        egui::Label::new(
                            RichText::new(rst).monospace().size(11.5).color(gray),
                        ),
                    );
                    col(
                        ui,
                        48.0,
                        egui::Label::new(
                            RichText::new(r.grid.as_deref().unwrap_or(""))
                                .monospace()
                                .size(11.5)
                                .color(crate::theme::CYAN_DIM()),
                        ),
                    );
                    // QSL / confirmation status: green ✓ when confirmed,
                    // dim ↑ when uploaded-but-unconfirmed, else blank.
                    let (qsl_txt, qsl_col) = if r.is_confirmed() {
                        ("✓", crate::theme::GREEN())
                    } else if r.lotw_sent || r.eqsl_sent || r.qrz_sent || r.clublog_sent {
                        ("↑", crate::theme::gray(140))
                    } else {
                        ("", gray)
                    };
                    let mut qsl_tip = String::new();
                    for (on, name) in [
                        (r.lotw_rcvd, "LoTW ✓"),
                        (r.eqsl_rcvd, "eQSL ✓"),
                        (r.qsl_rcvd, "card ✓"),
                        (r.lotw_sent, "LoTW ↑"),
                        (r.eqsl_sent, "eQSL ↑"),
                        (r.qrz_sent, "QRZ ↑"),
                        (r.clublog_sent, "Club Log ↑"),
                    ] {
                        if on {
                            if !qsl_tip.is_empty() {
                                qsl_tip.push_str(", ");
                            }
                            qsl_tip.push_str(name);
                        }
                    }
                    {
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(16.0, 20.0),
                            egui::Sense::hover(),
                        );
                        let mut c = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(rect)
                                .layout(egui::Layout::left_to_right(egui::Align::Center)),
                        );
                        let resp = c.add(egui::Label::new(
                            RichText::new(qsl_txt).size(13.0).strong().color(qsl_col),
                        ));
                        if !qsl_tip.is_empty() {
                            resp.on_hover_text(qsl_tip);
                        }
                    }
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            if crate::chrome::chip_accent(
                                ui,
                                false,
                                RichText::new("DEL").size(11.0),
                                crate::theme::PINK(),
                                Color32::WHITE,
                            )
                            .on_hover_text("Delete this entry")
                            .clicked()
                            {
                                to_delete = Some(r.id);
                            }
                            if crate::chrome::chip(
                                ui,
                                false,
                                RichText::new("EDIT").size(11.0),
                            )
                            .clicked()
                            {
                                to_edit = Some(r.id);
                            }
                            if !up_targets.is_empty()
                                && crate::chrome::chip(
                                    ui,
                                    false,
                                    RichText::new("UP").size(11.0),
                                )
                                .on_hover_text("Upload this QSO to configured logs")
                                .clicked()
                            {
                                to_upload = Some(r.id);
                            }
                            if !r.comment.is_empty() {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&r.comment)
                                            .size(11.5)
                                            .color(crate::theme::gray(120)),
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
            egui::Rect::from_min_max(
                rr.left_top(),
                egui::pos2(rr.left() + 2.0, rr.bottom()),
            ),
            0.0,
            crate::theme::CYAN_DIM(),
        );
        ui.add_space(2.0);
        if let Some(id) = to_delete {
            Some(RowAction::Delete(id))
        } else if let Some(id) = to_edit {
            Some(RowAction::Edit(id))
        } else {
            to_upload.map(RowAction::Upload)
        }
    }
    /// The QSO list, grouped into daily sessions (newest first).
    fn log_list(&mut self, ui: &mut egui::Ui, rows: std::ops::Range<usize>) {
        if self.qso_log.is_empty() {
            ui.add_space(8.0);
            ui.label(
                RichText::new("no QSOs yet — run FT8/FT4 or add a manual entry")
                    .color(crate::theme::gray(120)),
            );
            return;
        }
        // Already refreshed by the caller, which needed the item count to size
        // the scroll area. Taken out of `self` so drawing can borrow the log.
        let view = std::mem::take(&mut self.log_view);

        // Only the visible slice is drawn. See ROW_H for why every item is
        // given the same height.
        let up_targets = configured_upload_targets(&self.net_cfg_edit);
        let mut action: Option<RowAction> = None;
        for &item in &view.items[rows.clone()] {
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), row_height(ui)),
                egui::Sense::hover(),
            );
            let mut c = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(rect)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            match item {
                LogItem::Header(gi) => {
                    let g = &view.groups[gi];
                    Self::log_day_header(&mut c, g, g.end - g.start);
                }
                LogItem::Qso(oi) => {
                    if let Some(a) =
                        Self::log_qso_row(&mut c, &self.qso_log[view.order[oi]], &up_targets)
                    {
                        action = Some(a);
                    }
                }
            }
        }

        // Put the cache back. It was taken out for the duration of the loop
        // because drawing needs &self.qso_log while the view borrows from self.
        self.log_view = view;

        let (mut to_edit, mut to_delete, mut to_upload) = (None, None, None);
        match action {
            Some(RowAction::Edit(id)) => to_edit = Some(id),
            Some(RowAction::Delete(id)) => to_delete = Some(id),
            Some(RowAction::Upload(id)) => to_upload = Some(id),
            None => {}
        }

        if let Some(id) = to_delete {
            self.qso_log.retain(|q| q.id != id);
            persist_qso_log(&self.qso_log);
        } else if let Some(id) = to_edit {
            if let Some(r) = self.qso_log.iter().find(|q| q.id == id) {
                self.log_edit = Some(LogEditForm::from_record(r));
            }
        } else if let Some(id) = to_upload {
            if let Some(r) = self.qso_log.iter().find(|q| q.id == id) {
                let adif = sdroxide_types::qso_log_to_adif(std::slice::from_ref(r));
                self.pending_uploads.push((id, adif, up_targets.clone()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A QSO at a given time, which is all [`LogView`] orders or groups by.
    fn qso(id: u64, start_utc: i64) -> QsoRecord {
        QsoRecord { id, start_utc, call: format!("G{id}AAA"), ..Default::default() }
    }

    /// 2024-01-01 00:00:00 UTC, so the arithmetic below reads as clock time.
    const DAY0: i64 = 1_704_067_200;

    #[test]
    fn log_view_orders_newest_first_and_groups_by_day() {
        // Deliberately out of order on input: the log is not sorted on disk.
        let log = vec![
            qso(1, DAY0 + 3_600),           // day 0, 01:00
            qso(2, DAY0 + 86_400 + 7_200),  // day 1, 02:00
            qso(3, DAY0 + 7_200),           // day 0, 02:00
            qso(4, DAY0 + 86_400 + 3_600),  // day 1, 01:00
        ];
        let mut v = LogView::default();
        v.refresh(&log);

        assert_eq!(v.groups.len(), 2, "two distinct days");
        // Newest day first, and newest QSO first within it.
        assert_eq!(v.groups[0].day, "2024-01-02");
        assert_eq!(log[v.order[v.groups[0].start]].id, 2);
        assert_eq!(v.groups[0].newest, DAY0 + 86_400 + 7_200);
        assert_eq!(v.groups[0].oldest, DAY0 + 86_400 + 3_600);
        assert_eq!(v.groups[1].day, "2024-01-01");
        assert_eq!(log[v.order[v.groups[1].start]].id, 3);
        // The ranges tile the whole ordering with no gap and no overlap.
        assert_eq!(v.groups[0].start, 0);
        assert_eq!(v.groups[0].end, v.groups[1].start);
        assert_eq!(v.groups.last().unwrap().end, log.len());
    }

    #[test]
    fn log_view_does_not_rebuild_when_nothing_changed() {
        // The whole point of the cache. Proven by mutating the built result and
        // showing a second refresh leaves it alone, which is the only way to see
        // from outside that no work was done.
        let log = vec![qso(1, DAY0), qso(2, DAY0 + 86_400)];
        let mut v = LogView::default();
        v.refresh(&log);
        v.groups[0].day = "sentinel".into();
        v.refresh(&log);
        assert_eq!(v.groups[0].day, "sentinel", "rebuilt when it did not need to");
    }

    #[test]
    fn log_view_rebuilds_when_the_log_changes() {
        let mut log = vec![qso(1, DAY0), qso(2, DAY0 + 86_400)];
        let mut v = LogView::default();

        // An addition.
        v.refresh(&log);
        v.groups[0].day = "sentinel".into();
        log.push(qso(3, DAY0 + 2 * 86_400));
        v.refresh(&log);
        assert_eq!(v.groups.len(), 3);
        assert_eq!(v.groups[0].day, "2024-01-03");

        // A deletion. Length changes, so this would be caught by length alone.
        log.remove(0);
        v.refresh(&log);
        assert_eq!(v.order.len(), 2);

        // An edit that moves a QSO to a different day, with the length and the
        // set of ids unchanged. This is the case a length check cannot see and
        // the reason start_utc is folded into the signature.
        v.groups[0].day = "sentinel".into();
        log[0].start_utc += 30 * 86_400;
        v.refresh(&log);
        assert_ne!(v.groups[0].day, "sentinel", "an edited time did not invalidate");
        assert_eq!(v.groups[0].day, "2024-02-01");
    }

    #[test]
    fn log_view_flattens_groups_and_rows_into_one_item_list() {
        // The virtualised scroll area indexes this list, not the groups, so it
        // has to be a header followed by exactly that group's rows, in order.
        let log = vec![qso(1, DAY0), qso(2, DAY0 + 3_600), qso(3, DAY0 + 86_400)];
        let mut v = LogView::default();
        v.refresh(&log);

        assert_eq!(v.items.len(), log.len() + v.groups.len(), "one item per QSO plus one per day");
        let shape: Vec<&str> = v
            .items
            .iter()
            .map(|i| match i {
                LogItem::Header(_) => "H",
                LogItem::Qso(_) => "Q",
            })
            .collect();
        // Newest day (one QSO) first, then the older day's two.
        assert_eq!(shape, ["H", "Q", "H", "Q", "Q"]);
    }

    /// Lay a QSO row out headlessly and measure it, in both layout tiers.
    ///
    /// `ScrollArea::show_rows` works out which items the viewport covers from
    /// the row height alone, so if a row is not really that tall the scroll bar
    /// and the content drift apart, worsening the further down the list you go.
    ///
    /// Both tiers are checked because `theme::apply_metrics` gives them
    /// different `item_spacing.y`, and re-runs whenever the window crosses the
    /// breakpoint. A constant written here passed by arithmetic and failed this
    /// test at once: the row is 34 points plus that spacing, not 34.
    #[test]
    fn row_height_matches_the_live_style() {
        for tier in [crate::layout::Tier::Desktop, crate::layout::Tier::Phone] {
            let ctx = egui::Context::default();
            crate::theme::apply_metrics(&ctx, tier);
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(900.0, 400.0));
            let r = QsoRecord {
                id: 1,
                start_utc: DAY0,
                call: "G4MQL".into(),
                band: "20m".into(),
                mode: "FT8".into(),
                ..Default::default()
            };
            let (mut measured, mut claimed) = (0.0f32, 0.0f32);
            let _ = ctx.run_ui(
                egui::RawInput { screen_rect: Some(screen), ..Default::default() },
                |ui| {
                    claimed = row_height(ui);
                    let before = ui.cursor().top();
                    SdroxideApp::log_qso_row(ui, &r, &[]);
                    measured = ui.cursor().top() - before;
                },
            );
            assert!(
                (measured - claimed).abs() < 0.5,
                "{tier:?}: a QSO row measured {measured} points but row_height said {claimed}; \
                 the virtualised list would drift"
            );
        }
    }

    #[test]
    fn log_view_handles_an_empty_log() {
        let mut v = LogView::default();
        v.refresh(&[]);
        assert!(v.order.is_empty());
        assert!(v.groups.is_empty());
    }
}
