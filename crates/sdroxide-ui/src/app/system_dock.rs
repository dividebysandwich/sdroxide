//! Snap System-strip overlays into a right column beside the waterfall.
//!
//! Mirrors the band/mode dock: a [`egui::Panel::right`] reserves layout space
//! early in the frame; each matching `*_window` later paints into its rect in
//! [`SdroxideApp::system_dock_body_rects`] instead of a floating centred window.
//! Several overlays may be pinned at once — they stack in the column.

use std::collections::HashMap;

use eframe::egui::{self, RichText};

use super::{SdroxideApp, SystemWasOpen};

/// Payload while dragging a docked header to reorder the column.
#[derive(Clone, Copy)]
struct DraggedSystemDock(SystemDock);

/// Which System-strip overlay is snapped into the right panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::app) enum SystemDock {
    Logbook,
    Spots,
    Awards,
    Bands,
    Sat,
    Ism,
    PublicSdrs,
    Mail,
    Memories,
    Scanner,
    Grid,
    Settings,
}

impl SystemDock {
    pub(in crate::app) fn title(self) -> &'static str {
        match self {
            Self::Logbook => "LOGBOOK",
            Self::Spots => "SPOTS",
            Self::Awards => "AWARDS",
            Self::Bands => "BANDS",
            Self::Sat => "SATELLITE",
            Self::Ism => "ISM DEVICES",
            Self::PublicSdrs => "PUBLIC SDRS",
            Self::Mail => "MAIL",
            Self::Memories => "MEMORIES",
            Self::Scanner => "SCANNER",
            Self::Grid => "GRID TRACKER",
            Self::Settings => "SETTINGS",
        }
    }

    fn open_flag(self, app: &SdroxideApp) -> bool {
        match self {
            Self::Logbook => app.show_logbook,
            Self::Spots => app.show_spots,
            Self::Awards => app.show_awards,
            Self::Bands => app.show_bands,
            Self::Sat => app.show_sat,
            Self::Ism => app.show_ism,
            Self::PublicSdrs => app.show_public_sdrs,
            Self::Mail => app.mail.open,
            Self::Memories => app.show_memories,
            Self::Scanner => app.show_scanner,
            Self::Grid => app.show_grid,
            Self::Settings => app.show_settings,
        }
    }

    fn set_open(self, app: &mut SdroxideApp, open: bool) {
        match self {
            Self::Logbook => app.show_logbook = open,
            Self::Spots => app.show_spots = open,
            Self::Awards => app.show_awards = open,
            Self::Bands => app.show_bands = open,
            Self::Sat => app.show_sat = open,
            Self::Ism => app.show_ism = open,
            Self::PublicSdrs => app.show_public_sdrs = open,
            Self::Mail => app.mail.open = open,
            Self::Memories => app.show_memories = open,
            Self::Scanner => app.show_scanner = open,
            Self::Grid => app.show_grid = open,
            Self::Settings => app.show_settings = open,
        }
    }

    fn center_pending_mut(self, app: &mut SdroxideApp) -> &mut bool {
        let p: &mut SystemWasOpen = match self {
            Self::Mail => {
                return &mut app.mail.center_pending;
            }
            _ => &mut app.system_center_pending,
        };
        match self {
            Self::Logbook => &mut p.logbook,
            Self::Spots => &mut p.spots,
            Self::Awards => &mut p.awards,
            Self::Bands => &mut p.bands,
            Self::Sat => &mut p.sat,
            Self::Ism => &mut p.ism,
            Self::PublicSdrs => &mut p.public_sdrs,
            Self::Memories => &mut p.memories,
            Self::Scanner => &mut p.scanner,
            Self::Grid => &mut p.grid,
            Self::Settings => &mut p.settings,
            Self::Mail => unreachable!("handled above"),
        }
    }
}

/// Narrowest dock column that still fits typical System overlays.
const SYSTEM_DOCK_MIN_W: f32 = 220.0;
/// Widest share of the main content width the System dock may take when dragged.
/// The panadapter / operating column always keeps the other half.
const SYSTEM_DOCK_MAX_FRACTION: f32 = 0.5;
/// Width of [`crate::chrome::split_handle`] on the dock's inner edge.
/// Wider than the digi / JS8 7 pt strips so the three dashes sit clearly in
/// the middle of the gutter instead of hugging the main / dock edges.
const SYSTEM_DOCK_HANDLE_W: f32 = 14.0;
/// Header-only height for a collapsed docked overlay (triangle + title + chips).
const SYSTEM_DOCK_COLLAPSED_H: f32 = 24.0;
/// Horizontal [`crate::chrome::split_handle`] under each expanded docked body.
const SYSTEM_DOCK_ROW_HANDLE_H: f32 = 7.0;
/// Thin rule between the header and the body inside an expanded slot.
const SYSTEM_DOCK_SEP_H: f32 = 6.0;
/// Expanded slot floor: three header heights (header + body + grip).
const SYSTEM_DOCK_SLOT_MIN_H: f32 = SYSTEM_DOCK_COLLAPSED_H * 3.0;

impl SdroxideApp {
    /// Whether this overlay is snapped into the right panel (expanded or collapsed).
    ///
    /// Collapsed items still count as docked so their floating window stays
    /// suppressed; only [`Self::system_dock_host`] cares whether a body rect
    /// was reserved this frame.
    pub(in crate::app) fn system_docked_as(&self, kind: SystemDock) -> bool {
        self.system_dock.contains(&kind)
    }

    /// Snap `kind` into the right panel. Already-docked overlays stay; this one
    /// is appended if it is not already there (and starts expanded).
    pub(in crate::app) fn dock_system(&mut self, kind: SystemDock) {
        kind.set_open(self, true);
        if !self.system_dock.contains(&kind) {
            self.system_dock.push(kind);
        }
        self.system_dock_collapsed.remove(&kind);
        // Match the mean of existing body shares so a new pin does not start
        // as a 1 px stub next to absolute pixel weights from prior drags.
        if !self.system_dock_weights.contains_key(&kind) {
            let mean = if self.system_dock_weights.is_empty() {
                1.0
            } else {
                self.system_dock_weights.values().sum::<f32>()
                    / self.system_dock_weights.len() as f32
            };
            self.system_dock_weights.insert(kind, mean.max(1.0));
        }
    }

    /// Return one docked overlay to a floating window and re-center it.
    pub(in crate::app) fn undock_system(&mut self, kind: SystemDock) {
        self.system_dock.retain(|k| *k != kind);
        self.system_dock_body_rects.remove(&kind);
        self.system_dock_collapsed.remove(&kind);
        self.system_dock_weights.remove(&kind);
        kind.set_open(self, true);
        *kind.center_pending_mut(self) = true;
    }

    /// Close one docked overlay and drop it from the column.
    pub(in crate::app) fn close_system_dock(&mut self, kind: SystemDock) {
        self.system_dock.retain(|k| *k != kind);
        self.system_dock_body_rects.remove(&kind);
        self.system_dock_collapsed.remove(&kind);
        self.system_dock_weights.remove(&kind);
        kind.set_open(self, false);
    }

    /// Move `from` next to `onto` in the dock stack (`before` → above).
    fn reorder_system_dock(&mut self, from: SystemDock, onto: SystemDock, before: bool) {
        if from == onto {
            return;
        }
        let Some(from_i) = self.system_dock.iter().position(|k| *k == from) else {
            return;
        };
        self.system_dock.remove(from_i);
        let Some(onto_i) = self.system_dock.iter().position(|k| *k == onto) else {
            self.system_dock.insert(from_i.min(self.system_dock.len()), from);
            return;
        };
        let insert_at = if before { onto_i } else { onto_i + 1 };
        self.system_dock.insert(insert_at, from);
    }

    /// Relative body share for `kind` (any positive scale — layout normalizes).
    fn system_dock_weight(&self, kind: SystemDock) -> f32 {
        self.system_dock_weights.get(&kind).copied().unwrap_or(1.0).max(0.05)
    }

    /// Body height floor so the whole expanded slot stays ≥ [`SYSTEM_DOCK_SLOT_MIN_H`].
    fn system_dock_body_min() -> f32 {
        (SYSTEM_DOCK_SLOT_MIN_H
            - SYSTEM_DOCK_COLLAPSED_H
            - SYSTEM_DOCK_SEP_H
            - SYSTEM_DOCK_ROW_HANDLE_H)
            .max(SYSTEM_DOCK_COLLAPSED_H)
    }

    /// Split `body_pool` across expanded overlays by weight, honoring `body_min`
    /// whenever the pool is large enough.
    fn system_dock_body_heights(
        &self,
        expanded: &[SystemDock],
        body_pool: f32,
        body_min: f32,
    ) -> HashMap<SystemDock, f32> {
        let mut out = HashMap::new();
        let n = expanded.len();
        if n == 0 || body_pool <= 0.0 {
            return out;
        }
        let n_f = n as f32;
        // When the column cannot fit every min, share the pool evenly.
        if body_pool < body_min * n_f {
            let each = body_pool / n_f;
            for &k in expanded {
                out.insert(k, each);
            }
            return out;
        }

        let weight_sum: f32 =
            expanded.iter().map(|k| self.system_dock_weight(*k)).sum::<f32>().max(0.05);
        let mut heights: Vec<(SystemDock, f32)> = expanded
            .iter()
            .map(|&k| {
                let h = body_pool * (self.system_dock_weight(k) / weight_sum);
                (k, h)
            })
            .collect();

        // Lift anyone under the floor; take the deficit from those above it.
        let mut deficit = 0.0_f32;
        for (_, h) in &mut heights {
            if *h < body_min {
                deficit += body_min - *h;
                *h = body_min;
            }
        }
        if deficit > 0.0 {
            let donors: Vec<usize> = heights
                .iter()
                .enumerate()
                .filter(|(_, (_, h))| *h > body_min + 0.5)
                .map(|(i, _)| i)
                .collect();
            let donor_extra: f32 =
                donors.iter().map(|&i| heights[i].1 - body_min).sum::<f32>().max(0.05);
            for i in donors {
                let take = deficit * ((heights[i].1 - body_min) / donor_extra);
                heights[i].1 = (heights[i].1 - take).max(body_min);
            }
        }

        // Exact fill — float error or donor shortfall leaves a remainder.
        let used: f32 = heights.iter().map(|(_, h)| *h).sum();
        let rem = body_pool - used;
        if rem.abs() > 0.25 {
            if let Some((_, h)) = heights
                .iter_mut()
                .filter(|(_, h)| *h + rem >= body_min)
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            {
                *h += rem;
            }
        }

        for (k, h) in heights {
            out.insert(k, h.max(0.0));
        }
        out
    }

    /// Pin control in a floating System window's title bar, in the slot
    /// immediately to the right of egui's collapse triangle.
    ///
    /// Call after `.show()` with that window's response. Hidden when the layout
    /// cannot dock. Returns whether the operator clicked it this frame.
    pub(in crate::app) fn system_title_pin(
        &self,
        ctx: &egui::Context,
        win: &egui::Response,
    ) -> bool {
        if self.band_dock_room.is_none() {
            return false;
        }
        // Match egui 0.36 `title_ui` exactly: the outer window frame's inner
        // margin is zeroed before the title draws, and the title AtomLayout
        // uses `Frame::window` → `spacing.window_margin`. Collapse is a
        // heading-height square; the triangle is then centered (shrunk) inside
        // it to `icon_width`. Same slot, one gap to the right, same top.
        let style = ctx.style_of(ctx.theme());
        let heading_h =
            ctx.fonts_mut(|f| f.row_height(&egui::TextStyle::Heading.resolve(style.as_ref())));
        let margin = style.spacing.window_margin;
        let spacing = style.spacing.item_spacing.x;
        let icon = style.spacing.icon_width;
        let alloc = heading_h;
        let pin_min = egui::pos2(
            win.rect.left() + margin.leftf() + alloc + spacing,
            win.rect.top() + margin.topf(),
        );

        let mut clicked = false;
        let inner = egui::Area::new(win.id.with("system-title-pin"))
            // Same order as `Window` (Middle). `set_sublayer` panics if the
            // child order differs from the parent — Foreground over Middle
            // trips egui's debug assert.
            .order(egui::Order::Middle)
            .fixed_pos(pin_min)
            .sense(egui::Sense::click())
            .show(ctx, |ui| {
                // Exact allocation size (no Button chrome) so the hit target
                // matches the collapse square; glyph centered like the triangle.
                let (rect, resp) =
                    ui.allocate_exact_size(egui::vec2(alloc, alloc), egui::Sense::click());
                let font = egui::FontId::proportional((icon * 0.95).clamp(11.0, 16.0));
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "📌",
                    font,
                    crate::theme::TEXT_STRONG(),
                );
                if resp.on_hover_text("Snap into the right panel beside the waterfall").clicked() {
                    clicked = true;
                }
            });
        // Ride the window's layer so the pin hides/moves with it rather than
        // stacking as a separate Middle window above everything else.
        ctx.set_sublayer(win.layer_id, inner.response.layer_id);
        clicked
    }

    /// Floating-window builder replaced by a fixed host inside the dock column.
    ///
    /// The host is clipped and size-locked to `kind`'s body rect so stacked
    /// overlays cannot paint over each other.
    pub(in crate::app) fn system_dock_host<'a>(
        &self,
        title: &'a str,
        id: egui::Id,
        kind: SystemDock,
    ) -> Option<egui::Window<'a>> {
        let rect = *self.system_dock_body_rects.get(&kind)?;
        Some(
            egui::Window::new(title)
                // Separate id from the floating window so persisted size/pos
                // does not fight the dock host. Salted per kind so stacked
                // hosts do not share one AreaState.
                .id(id.with("system-dock-host").with(kind.title()))
                .title_bar(false)
                .fixed_pos(rect.min)
                .fixed_size(rect.size())
                .max_size(rect.size())
                .constrain_to(rect)
                .resizable(false)
                .frame(egui::Frame::NONE)
                .fade_in(false)
                .fade_out(false),
        )
    }

    /// Clip the docked host's content to its reserved body rect.
    ///
    /// Call at the start of every docked `.show()` closure so scroll areas and
    /// tall content cannot paint into the slot below.
    pub(in crate::app) fn clip_system_dock_body(&self, ui: &mut egui::Ui, kind: SystemDock) {
        let Some(rect) = self.system_dock_body_rects.get(&kind).copied() else {
            return;
        };
        ui.set_clip_rect(ui.clip_rect().intersect(rect));
        ui.set_max_size(rect.size());
    }

    /// Reserve the right column and record a body rect per docked overlay.
    ///
    /// Call after the top bar (and beside / before the band dock) so the
    /// waterfall shrinks around the column. Slots partition the column height
    /// exactly — they never grow past their share, so they cannot overlap.
    pub(in crate::app) fn system_dock_panel(&mut self, ui: &mut egui::Ui) {
        self.system_dock_body_rects.clear();
        // Chip (or ×) closed an overlay — drop it from the column.
        let still_open: Vec<SystemDock> =
            self.system_dock.iter().copied().filter(|k| k.open_flag(self)).collect();
        self.system_dock_collapsed.retain(|k| still_open.contains(k));
        self.system_dock_weights.retain(|k, _| still_open.contains(k));
        self.system_dock = still_open;
        if self.system_dock.is_empty() {
            return;
        }
        // `band_dock_room` only answers "is the layout wide enough to dock".
        // Width is driven by `system_dock_fraction`, capped at half the content
        // so the main panel always keeps ≥ 50%.
        if self.band_dock_room.is_none() {
            // Too narrow / phone: keep the dock list so widening restores the
            // column; matching windows paint floating until then.
            return;
        }
        let content_w = ui.available_width();
        let max_w = (content_w * SYSTEM_DOCK_MAX_FRACTION).max(1.0);
        let min_w = SYSTEM_DOCK_MIN_W.min(max_w);
        let dock_w = (content_w * self.system_dock_fraction).clamp(min_w, max_w);
        // Keep the stored fraction honest after a window resize clamps the px.
        self.system_dock_fraction = dock_w / content_w.max(1.0);
        let docked = self.system_dock.clone();

        let mut close: Option<SystemDock> = None;
        let mut undock: Option<SystemDock> = None;
        let mut toggle_collapse: Option<SystemDock> = None;
        // (dragged kind, drop target, insert above target)
        let mut reorder: Option<(SystemDock, SystemDock, bool)> = None;
        let mut body_rects = HashMap::new();
        let mut drag_dx = 0.0_f32;
        // (upper expanded, lower expanded, pixel delta — drag down grows upper)
        let mut height_drag: Option<(SystemDock, SystemDock, f32)> = None;
        let collapsed = self.system_dock_collapsed.clone();
        let expanded: Vec<SystemDock> =
            docked.iter().copied().filter(|k| !collapsed.contains(k)).collect();
        let body_min = Self::system_dock_body_min();
        // Chrome inside an expanded slot: header + separator + row grip.
        let expand_chrome = SYSTEM_DOCK_COLLAPSED_H + SYSTEM_DOCK_SEP_H + SYSTEM_DOCK_ROW_HANDLE_H;

        // Handle sits inside the panel on the inner (main-facing) edge — egui's
        // own Panel resize separator is a plain line, not the three-dash chrome
        // used everywhere else in the app.
        egui::Panel::right(crate::layout::salted_id(ui.ctx(), "system-dock"))
            .resizable(false)
            .exact_size(dock_w + SYSTEM_DOCK_HANDLE_W)
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(crate::theme::PANEL())
                    .inner_margin(egui::Margin::ZERO),
            )
            .show(ui, |ui| {
                let avail_h = ui.available_height().max(0.0);
                // No gap between handle and body — spacing used to steal the
                // 7 pt strip and the vertical grip vanished.
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.horizontal(|ui| {
                    let hresp = crate::chrome::split_handle(
                        ui,
                        egui::vec2(SYSTEM_DOCK_HANDLE_W, avail_h),
                        None,
                    );
                    if hresp.dragged() {
                        // Drag toward the main column (negative x) widens the dock.
                        drag_dx = hresp.drag_delta().x;
                    }

                    ui.allocate_ui_with_layout(
                        egui::vec2(dock_w, avail_h),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            egui::Frame::new()
                                .inner_margin(egui::Margin::symmetric(9, 7))
                                .show(ui, |ui| {
                                    let n = docked.len().max(1);
                                    let gap = 4.0;
                                    let gaps = gap * (n.saturating_sub(1) as f32);
                                    let col_h = ui.available_height().max(0.0);
                                    let avail_w = ui.available_width();
                                    let n_expanded = expanded.len();
                                    let n_collapsed = n - n_expanded;
                                    let collapsed_h =
                                        SYSTEM_DOCK_COLLAPSED_H * n_collapsed as f32;
                                    let body_pool = (col_h
                                        - gaps
                                        - collapsed_h
                                        - expand_chrome * n_expanded as f32)
                                        .max(0.0);
                                    let body_heights = self.system_dock_body_heights(
                                        &expanded,
                                        body_pool,
                                        body_min,
                                    );

                                    for (i, kind) in docked.iter().copied().enumerate() {
                                        let is_expanded = !collapsed.contains(&kind);
                                        let body_h = if is_expanded {
                                            body_heights.get(&kind).copied().unwrap_or(0.0)
                                        } else {
                                            0.0
                                        };
                                        let slot_h = if is_expanded {
                                            expand_chrome + body_h
                                        } else {
                                            SYSTEM_DOCK_COLLAPSED_H
                                        };
                                        let (slot_rect, slot_resp) = ui.allocate_exact_size(
                                            egui::vec2(avail_w, slot_h),
                                            egui::Sense::hover(),
                                        );
                                        // Drop target: insert above/below based on
                                        // pointer Y vs the slot mid-line.
                                        let drop_before = ui
                                            .input(|inp| inp.pointer.interact_pos())
                                            .map(|p| p.y < slot_rect.center().y)
                                            .unwrap_or(true);
                                        if let Some(payload) =
                                            slot_resp.dnd_hover_payload::<DraggedSystemDock>()
                                        {
                                            if payload.0 != kind {
                                                let y = if drop_before {
                                                    slot_rect.top()
                                                } else {
                                                    slot_rect.bottom() - 1.0
                                                };
                                                ui.painter().hline(
                                                    slot_rect.x_range(),
                                                    y,
                                                    egui::Stroke::new(
                                                        2.0,
                                                        crate::theme::CYAN(),
                                                    ),
                                                );
                                            }
                                        }
                                        if let Some(payload) =
                                            slot_resp.dnd_release_payload::<DraggedSystemDock>()
                                        {
                                            if payload.0 != kind {
                                                reorder =
                                                    Some((payload.0, kind, drop_before));
                                            }
                                        }
                                        ui.scope_builder(
                                            egui::UiBuilder::new().max_rect(slot_rect),
                                            |ui| {
                                                ui.set_clip_rect(slot_rect);
                                                ui.spacing_mut().item_spacing.y = 0.0;
                                                // Full header is the reorder bar.
                                                // Collapse / 📌 / × are painted first
                                                // and excluded from the drag sense so
                                                // their hover/click never starts a drag.
                                                let (header_rect, _) = ui
                                                    .allocate_exact_size(
                                                        egui::vec2(
                                                            avail_w,
                                                            SYSTEM_DOCK_COLLAPSED_H,
                                                        ),
                                                        egui::Sense::hover(),
                                                    );
                                                let drag_id = crate::layout::salted_id(
                                                    ui.ctx(),
                                                    "system-dock-drag",
                                                )
                                                .with(kind.title());
                                                let mut over_controls = false;
                                                ui.scope_builder(
                                                    egui::UiBuilder::new()
                                                        .max_rect(header_rect),
                                                    |ui| {
                                                        ui.spacing_mut().item_spacing.x = 4.0;
                                                        ui.horizontal(|ui| {
                                                            let icon =
                                                                ui.spacing().icon_width;
                                                            // Reserved grip slot — painted
                                                            // only while the drag bar is
                                                            // hovered (not over buttons).
                                                            let (grip_rect, _) = ui
                                                                .allocate_exact_size(
                                                                    egui::vec2(icon, icon),
                                                                    egui::Sense::hover(),
                                                                );
                                                            let (_, icon_resp) = ui
                                                                .allocate_exact_size(
                                                                    egui::vec2(icon, icon),
                                                                    egui::Sense::click(),
                                                                );
                                                            let openness = if is_expanded {
                                                                1.0
                                                            } else {
                                                                0.0
                                                            };
                                                            egui::collapsing_header::paint_default_icon(
                                                                ui, openness, &icon_resp,
                                                            );
                                                            if icon_resp.contains_pointer() {
                                                                over_controls = true;
                                                            }
                                                            if icon_resp
                                                                .on_hover_text(
                                                                    if is_expanded {
                                                                        "Collapse"
                                                                    } else {
                                                                        "Expand"
                                                                    },
                                                                )
                                                                .clicked()
                                                            {
                                                                toggle_collapse = Some(kind);
                                                            }
                                                            ui.label(
                                                                RichText::new(kind.title())
                                                                    .size(11.0)
                                                                    .strong()
                                                                    .color(
                                                                        crate::theme::CYAN(),
                                                                    ),
                                                            );
                                                            ui.with_layout(
                                                                egui::Layout::right_to_left(
                                                                    egui::Align::Center,
                                                                ),
                                                                |ui| {
                                                                    let close_r =
                                                                        crate::chrome::chip(
                                                                            ui, false, "×",
                                                                        )
                                                                        .on_hover_text(
                                                                            "Close",
                                                                        );
                                                                    if close_r.clicked() {
                                                                        close = Some(kind);
                                                                    }
                                                                    if close_r
                                                                        .contains_pointer()
                                                                    {
                                                                        over_controls = true;
                                                                    }
                                                                    let pin_r =
                                                                        crate::chrome::chip(
                                                                            ui, true, "📌",
                                                                        )
                                                                        .on_hover_text(
                                                                            "Return to a floating window",
                                                                        );
                                                                    if pin_r.clicked() {
                                                                        undock = Some(kind);
                                                                    }
                                                                    if pin_r
                                                                        .contains_pointer()
                                                                    {
                                                                        over_controls = true;
                                                                    }
                                                                },
                                                            );

                                                            let dragging = ui
                                                                .ctx()
                                                                .is_being_dragged(drag_id);
                                                            if dragging {
                                                                egui::DragAndDrop::set_payload(
                                                                    ui.ctx(),
                                                                    DraggedSystemDock(kind),
                                                                );
                                                                ui.ctx().request_repaint();
                                                                ui.ctx().set_cursor_icon(
                                                                    egui::CursorIcon::Grabbing,
                                                                );
                                                                // Follow-the-pointer title
                                                                // chip so the stack does
                                                                // not look frozen mid-drag.
                                                                if let Some(pos) = ui
                                                                    .ctx()
                                                                    .pointer_interact_pos()
                                                                {
                                                                    egui::Area::new(
                                                                        drag_id.with(
                                                                            "drag-preview",
                                                                        ),
                                                                    )
                                                                    .order(
                                                                        egui::Order::Tooltip,
                                                                    )
                                                                    .fixed_pos(
                                                                        pos + egui::vec2(
                                                                            12.0, 8.0,
                                                                        ),
                                                                    )
                                                                    .show(
                                                                        ui.ctx(),
                                                                        |ui| {
                                                                            egui::Frame::new()
                                                                                .fill(
                                                                                    crate::theme::PANEL(),
                                                                                )
                                                                                .stroke(
                                                                                    egui::Stroke::new(
                                                                                        1.0,
                                                                                        crate::theme::CYAN(),
                                                                                    ),
                                                                                )
                                                                                .inner_margin(6.0)
                                                                                .show(ui, |ui| {
                                                                                    ui.label(
                                                                                        RichText::new(
                                                                                            kind.title(),
                                                                                        )
                                                                                        .size(11.0)
                                                                                        .strong()
                                                                                        .color(
                                                                                            crate::theme::CYAN(),
                                                                                        ),
                                                                                    );
                                                                                });
                                                                        },
                                                                    );
                                                                }
                                                            } else if !over_controls {
                                                                let drag_resp = ui
                                                                    .interact(
                                                                        header_rect,
                                                                        drag_id,
                                                                        egui::Sense::drag(),
                                                                    )
                                                                    .on_hover_cursor(
                                                                        egui::CursorIcon::Grab,
                                                                    )
                                                                    .on_hover_text(
                                                                        "Drag to reorder",
                                                                    );
                                                                if drag_resp.hovered()
                                                                    || drag_resp
                                                                        .contains_pointer()
                                                                {
                                                                    // Six-dot grip — only
                                                                    // while the bar (not a
                                                                    // button) is hovered.
                                                                    ui.painter().text(
                                                                        grip_rect.center(),
                                                                        egui::Align2::CENTER_CENTER,
                                                                        "⠿",
                                                                        egui::FontId::proportional(
                                                                            icon * 0.95,
                                                                        ),
                                                                        crate::theme::gray(90),
                                                                    );
                                                                }
                                                            }
                                                        });
                                                    },
                                                );
                                                if is_expanded {
                                                    // Fixed-height rule so chrome
                                                    // math matches slot allocation.
                                                    let (sep_rect, _) = ui
                                                        .allocate_exact_size(
                                                            egui::vec2(
                                                                avail_w,
                                                                SYSTEM_DOCK_SEP_H,
                                                            ),
                                                            egui::Sense::hover(),
                                                        );
                                                    let y = sep_rect.center().y;
                                                    ui.painter().hline(
                                                        sep_rect.x_range(),
                                                        y,
                                                        egui::Stroke::new(
                                                            1.0,
                                                            crate::theme::gray(55),
                                                        ),
                                                    );

                                                    let (rect, _) = ui.allocate_exact_size(
                                                        egui::vec2(avail_w, body_h),
                                                        egui::Sense::hover(),
                                                    );
                                                    body_rects.insert(kind, rect);

                                                    // Horizontal three-dash grip under
                                                    // the body. Interactive only when
                                                    // another expanded item sits below
                                                    // — then a drag trades height 1:1.
                                                    let next = expanded
                                                        .iter()
                                                        .skip_while(|k| **k != kind)
                                                        .nth(1)
                                                        .copied();
                                                    let hresp = crate::chrome::split_handle(
                                                        ui,
                                                        egui::vec2(
                                                            avail_w,
                                                            SYSTEM_DOCK_ROW_HANDLE_H,
                                                        ),
                                                        None,
                                                    );
                                                    if let Some(next) = next {
                                                        let dy = hresp.drag_delta().y;
                                                        if hresp
                                                            .on_hover_text(
                                                                "Drag to resize this docked panel",
                                                            )
                                                            .dragged()
                                                        {
                                                            height_drag =
                                                                Some((kind, next, dy));
                                                            ui.ctx().request_repaint();
                                                        }
                                                    }
                                                }
                                            },
                                        );
                                        if i + 1 < n {
                                            ui.add_space(gap);
                                        }
                                    }
                                });
                        },
                    );
                });
            });

        if drag_dx != 0.0 {
            let new_w = (dock_w - drag_dx).clamp(min_w, max_w);
            self.system_dock_fraction = new_w / content_w.max(1.0);
        }

        if let Some((a, b, dy)) = height_drag {
            if dy != 0.0 {
                // Promote every laid-out body to an absolute weight first so a
                // prior equal-share `1.0` does not sit next to pixel heights
                // and collapse the rest of the stack.
                for (k, rect) in &body_rects {
                    self.system_dock_weights.insert(*k, rect.height().max(0.05));
                }
                // Trade body pixels 1:1 — same feel as digi / waterfall splits.
                let ha = body_rects.get(&a).map(|r| r.height()).unwrap_or(body_min);
                let hb = body_rects.get(&b).map(|r| r.height()).unwrap_or(body_min);
                let pair_px = (ha + hb).max(1.0);
                let lo = body_min.min(pair_px * 0.5);
                let new_ha = (ha + dy).clamp(lo, pair_px - lo);
                let new_hb = pair_px - new_ha;
                self.system_dock_weights.insert(a, new_ha.max(0.05));
                self.system_dock_weights.insert(b, new_hb.max(0.05));
            }
        }

        if let Some(kind) = toggle_collapse {
            if !self.system_dock_collapsed.remove(&kind) {
                self.system_dock_collapsed.insert(kind);
            }
        }

        if let Some((from, onto, before)) = reorder {
            self.reorder_system_dock(from, onto, before);
        }

        self.system_dock_body_rects = body_rects;

        if let Some(kind) = close {
            self.close_system_dock(kind);
        } else if let Some(kind) = undock {
            self.undock_system(kind);
        }
    }
}
