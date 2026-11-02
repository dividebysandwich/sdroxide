//! **Undocked mode**: parts of a radio's screen pulled out of the main window
//! into OS windows of their own — the spectrum on one monitor, the decoder on
//! another, the controls on a third, in the SDRuno mould.
//!
//! Ported from madmedicnl's sdroxide-brown, where the design grew over the
//! `undocked:` series (21c204fa … be03e18e, a3dc0caa, 078522f8, f562d30c) and
//! its later fixes; this is that design's final shape, fitted to this tree.
//!
//! The pieces:
//!
//! - [`sdroxide_types::DetachableModule`] names a module and its app-id, and
//!   [`sdroxide_types::UiSettings::detached`] remembers which are out and where
//!   their windows were.
//! - The **shell** (`multi.rs`) asks the focused radio which windows it wants
//!   ([`SdroxideApp::detached_wanted`]) and emits them every frame
//!   ([`SdroxideApp::show_detached_module`]). A wanted window is always emitted,
//!   so a radio or mode change never tears one down and has a tiling compositor
//!   re-place it.
//! - [`detached_viewport`] is the one place the multi-window plumbing lives. A
//!   module writes its contents and nothing else.
//!
//! **Native only.** The browser has one window, so every module stays in it and
//! the flags are ignored there.

use crate::i18n::tr;
#[cfg(not(target_arch = "wasm32"))]
use eframe::egui::RichText;
use eframe::egui::{self};
use sdroxide_types::Mode;
#[cfg(not(target_arch = "wasm32"))]
use sdroxide_types::{Command, DetachableModule, Spot};

use crate::app::SdroxideApp;

/// The program's name as a detached window's title carries it.
#[cfg(not(target_arch = "wasm32"))]
const FLAVOR: &str = "SDR Oxide";

/// The plain face a main window shows where every part of the radio is in a
/// window of its own — so the centre is never a black hole that looks broken.
pub(in crate::app) fn empty_centre(ui: &mut egui::Ui) {
    let area = ui.available_rect_before_wrap();
    ui.allocate_rect(area, egui::Sense::hover());
    let p = ui.painter_at(area);
    p.rect_filled(area, 0.0, crate::theme::BG_DEEP());
    let (ink, dim) = (crate::theme::gray(105), crate::theme::gray(85));
    let name = p.layout_no_wrap("SDR Oxide".to_owned(), egui::FontId::proportional(22.0), ink);
    let hint = p.layout_no_wrap(
        tr!("frame-every-module-own-window"),
        egui::FontId::proportional(13.0),
        dim,
    );
    let (name_w, name_h, hint_w) = (name.size().x, name.size().y, hint.size().x);
    let top = area.center().y - (name_h + 8.0 + hint.size().y) / 2.0;
    p.galley(egui::pos2(area.center().x - name_w / 2.0, top), name, ink);
    p.galley(egui::pos2(area.center().x - hint_w / 2.0, top + name_h + 8.0), hint, dim);
}

/// The dial as a large readout string, MHz with trailing zeros trimmed —
/// "27.265", "144.8", "10.1206". The detached spectrum's toolbar carries it.
#[cfg(not(target_arch = "wasm32"))]
fn readout_text(hz: f64) -> String {
    let mhz = format!("{:.6}", hz.max(0.0) / 1e6);
    format!("{} MHz", mhz.trim_end_matches('0').trim_end_matches('.'))
}

/// The detached spectrum's second line: the mode, and the S-meter reading when
/// there is one. Read-only — it shares the main window's meter, so it cannot
/// disagree with it.
#[cfg(not(target_arch = "wasm32"))]
fn detached_status_line(mode: Mode, meters: Option<&sdroxide_types::Meters>) -> String {
    match meters {
        Some(m) => {
            let (s, over) = m.s_units();
            let level = if over >= 1.0 { format!("S{s}+{over:.0}") } else { format!("S{s}") };
            format!("{}  ·  {}  ({} dBm)", mode.label(), level, m.s_dbm.round() as i32)
        }
        None => mode.label().to_string(),
    }
}

/// Height of a detached spectrum's toolbar — the readout's line and the chips
/// beside it. The spectrum is drawn **below** the bar, not under it: a
/// waterfall you cannot see the centre of is worse than a number in the way.
#[cfg(not(target_arch = "wasm32"))]
const SP_TOOLBAR_H: f32 = 32.0;

/// Everything about a module's **own OS window** that is not its contents: the
/// ids, the app-id a Wayland rule matches on, the title and the opening size.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) struct DetachedWindowSpec {
    /// Stable viewport id — **one window per module for the whole station**,
    /// not per radio. The window shows the focused radio's; a stable id across
    /// radio and pane switches lets the same OS window be reused rather than
    /// torn down and remapped, which a tiling compositor treats as a *new*
    /// window sized by it rather than by the operator.
    viewport_id: egui::ViewportId,
    /// egui memory key for the last-seen geometry, used to persist once a drag
    /// settles instead of once per frame.
    settle_id: egui::Id,
    /// The Wayland/Windows app-id a compositor rule matches on. Owned, because
    /// a tool window's is built at runtime (`sdroxide-tool-<id>`).
    app_id: String,
    title: String,
    default_size: [f32; 2],
    min_size: [f32; 2],
}

/// The window spec for a module. The app-ids are a public contract — an
/// operator's compositor rules match on them.
#[cfg(not(target_arch = "wasm32"))]
fn detached_spec(module: DetachableModule) -> DetachedWindowSpec {
    use DetachableModule as M;
    let app_id = module.app_id();
    let (name, default_size) = match module {
        M::Panadapter => (tr!("frame-module-panadapter"), [960.0, 540.0]),
        M::Panel => (tr!("frame-module-panel"), [520.0, 700.0]),
        M::Controls => (tr!("frame-module-controls"), [1280.0, 640.0]),
        M::AuxPanadapter => (tr!("frame-module-aux-panadapter"), [960.0, 540.0]),
        // Wide enough for the mode sections side by side — the reason it is a
        // window and not only the docked column.
        M::BandMenu => (tr!("frame-module-band-mode"), [560.0, 760.0]),
    };
    DetachedWindowSpec {
        viewport_id: egui::ViewportId::from_hash_of(app_id),
        settle_id: egui::Id::new(format!("{app_id}-geom")),
        app_id: app_id.to_string(),
        title: tr!("frame-detached-title", flavor = FLAVOR, name = name),
        default_size,
        min_size: [360.0, 240.0],
    }
}

/// The size and position a (re)built detached window opens at: the operator's
/// last geometry if there is one, otherwise the module's default, either way
/// shrunk to fit the monitor it is about to open on. Never grows a window.
///
/// The saved position is brought back onto the screen, right and bottom edges
/// only: a window saved on a display that is no longer there — a laptop taken
/// off a second screen, a resolution or scaling change — would otherwise open
/// where nothing can reach it. A negative x or y is a monitor to the left of or
/// above the primary, a real place, and is left alone. `None` on Wayland, which
/// gives a client no absolute position.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn detached_geometry(
    monitor: Option<egui::Vec2>,
    seed: Option<sdroxide_types::DetachedWindow>,
    default_size: [f32; 2],
) -> (egui::Vec2, Option<egui::Pos2>) {
    let want = seed.map_or(egui::Vec2::from(default_size), |s| egui::Vec2::from(s.size));
    let Some(monitor) = monitor.filter(|m| m.x > 1.0 && m.y > 1.0) else {
        return (want, seed.and_then(|s| s.pos).map(egui::Pos2::from));
    };
    let chrome = egui::vec2(16.0, 40.0);
    let size =
        crate::layout::fit_inner_size(monitor, want + chrome, want, egui::vec2(360.0, 240.0))
            .unwrap_or(want);
    let pos = seed.and_then(|s| s.pos).map(|[x, y]| {
        let max_x = (monitor.x - 80.0).max(0.0);
        let max_y = (monitor.y - 60.0).max(0.0);
        egui::pos2(x.min(max_x), y.min(max_y))
    });
    (size, pos)
}

/// What a detached window left behind this frame.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) struct DetachedOutcome {
    /// The operator closed the window — which means "dock me again", not "come
    /// back next frame".
    close_requested: bool,
    /// Where the window ended up, for the settle-persist in
    /// [`SdroxideApp::handle_detached_outcome`].
    geometry: Option<sdroxide_types::DetachedWindow>,
}

/// Everything a **child window** needs to answer the layout questions for
/// itself instead of for the main window — see
/// [`crate::layout::ViewportScope`]: whose radio it is, which layout the
/// operator asked for, and what shape of window it is.
#[derive(Clone, Copy)]
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(in crate::app) struct ChildWindow {
    salt: u32,
    mode: sdroxide_types::LayoutMode,
    shape: crate::layout::WindowShape,
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
impl ChildWindow {
    /// A window whose own width earns its tier — a spectrum, a tool.
    pub(in crate::app) fn own(app: &SdroxideApp) -> Self {
        Self::of(app, crate::layout::WindowShape::Own)
    }

    /// A desk tool, desktop-shaped whatever its height — the RX control
    /// console, the band selector.
    pub(in crate::app) fn desk(app: &SdroxideApp) -> Self {
        Self::of(app, crate::layout::WindowShape::Desk)
    }

    /// An **operating panel**: one pane at a time when the window is too narrow
    /// for two, with the window drawing the pane chips itself.
    pub(in crate::app) fn panes(app: &SdroxideApp) -> Self {
        Self::of(app, crate::layout::WindowShape::Panes)
    }

    fn of(app: &SdroxideApp, shape: crate::layout::WindowShape) -> Self {
        Self { salt: app.radio_id, mode: app.ui_settings.layout, shape }
    }

    /// Publish this window's answers for as long as the guard lives.
    pub(in crate::app) fn scope(self, ui: &egui::Ui) -> crate::layout::ViewportScope {
        crate::layout::ViewportScope::enter(
            ui.ctx(),
            ui.max_rect().size(),
            self.mode,
            self.shape,
            self.salt,
        )
    }
}

/// Show a module's window and draw `draw` into it, returning how it closed.
///
/// The one place the multi-window plumbing lives, so a new detachable module
/// writes its contents and nothing else. `show_viewport_immediate` rather than
/// the deferred form the 3D window uses: a module draws from `&mut self`'s live
/// state and a click, zoom or drag has to flow straight back, which the deferred
/// closure's `Send + Sync + 'static` bound would forbid.
///
/// Where the toolkit only offers *embedded* viewports (the headless test
/// harness, and any platform without native multi-viewport support), a window
/// inside the main one is not the feature, so this draws nothing and reports
/// nothing; the caller has already given the space away, so the main window
/// still lays out.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn detached_viewport(
    ctx: &egui::Context,
    spec: &DetachedWindowSpec,
    seed: Option<sdroxide_types::DetachedWindow>,
    child: ChildWindow,
    draw: impl FnOnce(&mut egui::Ui),
) -> DetachedOutcome {
    if ctx.embed_viewports() {
        return DetachedOutcome { close_requested: false, geometry: None };
    }
    let vid = spec.viewport_id;
    let mut builder = egui::ViewportBuilder::default()
        // The app-id is not decoration: it is the one handle a Wayland window
        // rule has to float this window and pin it to a monitor.
        .with_app_id(spec.app_id.clone())
        .with_title(spec.title.clone())
        .with_min_inner_size(spec.min_size);
    // Seed the size and place only on the frame the window is (re)built, so a
    // live resize is never fought by our own request.
    if ctx.cumulative_pass_nr_for(vid) == 0 {
        let monitor = ctx.input(|i| i.viewport().monitor_size);
        let (size, pos) = detached_geometry(monitor, seed, spec.default_size);
        builder = builder.with_inner_size(size);
        if let Some(pos) = pos {
            builder = builder.with_position(pos);
        }
    }
    let mut close_requested = false;
    let mut geometry: Option<sdroxide_types::DetachedWindow> = None;
    // `show_viewport_immediate` wants an `FnMut`; the draw is a one-shot, so it
    // rides an `Option` and is taken on the frame it is drawn (and dropped
    // un-called on a frame the window is only closing).
    let mut draw = Some((child, draw));
    ctx.show_viewport_immediate(vid, builder, |ui, _class| {
        let ictx = ui.ctx().clone();
        // `content_rect` is the inner size as the toolkit reports it — the only
        // size on Wayland, where there is no absolute position. `outer_rect`
        // carries that position where the platform has one.
        let size = ictx.content_rect().size();
        if size.x > 1.0 && size.y > 1.0 {
            let pos = ictx.input(|i| i.viewport().outer_rect).map(|r| r.min);
            geometry = Some(sdroxide_types::DetachedWindow {
                // Whole points, so a sub-point wobble is not a resize.
                size: [size.x.round(), size.y.round()],
                pos: pos.map(|p| [p.x.round(), p.y.round()]),
            });
        }
        if ictx.input(|i| i.viewport().close_requested()) {
            close_requested = true;
            // Wake the root pass so the main window reclaims the space promptly
            // rather than on the next unrelated repaint.
            ictx.request_repaint_of(egui::ViewportId::ROOT);
            return;
        }
        if let Some((child, draw)) = draw.take() {
            // **This window's** answers, not the main window's, for as long as
            // the draw runs; the guard puts the main window's back when it drops.
            let _scope = child.scope(ui);
            draw(ui);
        }
    });
    DetachedOutcome { close_requested, geometry }
}

impl SdroxideApp {
    /// Undock or dock one module, and keep the choice.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) fn set_module_detached(&mut self, module: DetachableModule, on: bool) {
        self.ui_settings.set_detached(module, on);
        crate::app::persist::persist_ui_settings(&self.ui_settings);
    }

    /// Whether the focused radio draws its operating panel into its own window
    /// this frame. Only a mode that *has* a panel can — a digital mode's
    /// operating panel, or CW's keyboard — so a voice mode ignores the setting;
    /// the preference stays and applies again in a mode with a panel.
    pub(in crate::app) fn panel_detached(&self, mode: Mode) -> bool {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = mode;
            false
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.ui_settings.is_detached(DetachableModule::Panel)
                && self.focused
                && (mode.has_bottom_panel() || mode == Mode::Cw)
        }
    }

    /// Whether the focused radio draws its **control strip** in its own window
    /// this frame. Every mode has one, so the only gates are the browser and the
    /// focus, exactly as for the panadapter.
    pub(in crate::app) fn controls_detached(&self) -> bool {
        #[cfg(target_arch = "wasm32")]
        {
            false
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.ui_settings.is_detached(DetachableModule::Controls) && self.focused
        }
    }

    /// The panadapter's window should exist this frame: undocked on the focused
    /// radio *and* there is a panadapter to show (its layers are on). The same
    /// rule the frame loop gives the space away by, so the two cannot disagree.
    pub(in crate::app) fn panadapter_window_wanted(&self) -> bool {
        #[cfg(target_arch = "wasm32")]
        {
            false
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.module_window_wanted(DetachableModule::Panadapter)
        }
    }

    /// The same rule for any spectrum module — SP1 and AUX SP — so the two
    /// cannot disagree about when a spectrum window exists. A second one that
    /// appeared without its layers would be an empty window on a monitor.
    #[cfg(not(target_arch = "wasm32"))]
    fn module_window_wanted(&self, module: DetachableModule) -> bool {
        self.ui_settings.is_detached(module) && self.focused && self.view.panadapter_visible()
    }

    /// The band/mode selector's window should exist this frame: undocked on the
    /// focused radio. It needs nothing else — every mode has a band and a mode.
    #[cfg(not(target_arch = "wasm32"))]
    fn band_menu_detached(&self) -> bool {
        self.ui_settings.is_detached(DetachableModule::BandMenu) && self.focused
    }

    /// Which of this radio's modules the shell should emit as their own windows
    /// this frame, by [`DetachableModule::index`].
    ///
    /// The **app** decides — it knows the layers, the mode and the focus, and
    /// the shell does not — and the shell emits the true ones, once, for the
    /// focused radio, every frame. Nothing while the radio has no screen of its
    /// own to show: a sign-in still pending, or a link that is down.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn detached_wanted(&self) -> [bool; DetachableModule::COUNT] {
        let mut want = [false; DetachableModule::COUNT];
        if self.error.is_some() || self.ctrl.auth_phase().is_pending() {
            return want;
        }
        want[DetachableModule::Panadapter.index()] = self.panadapter_window_wanted();
        want[DetachableModule::Panel.index()] = self.panel_detached(self.state.rx[0].mode);
        want[DetachableModule::Controls.index()] = self.controls_detached();
        want[DetachableModule::AuxPanadapter.index()] =
            self.module_window_wanted(DetachableModule::AuxPanadapter);
        want[DetachableModule::BandMenu.index()] = self.band_menu_detached();
        want
    }

    /// A detached spectrum's toolbar: the frequency and the mode with the
    /// S-meter reading across the top, the layer menu and DOCK at the right.
    ///
    /// Display only. A detached spectrum may be on a second monitor with the
    /// main window showing something else, so the one number that has to be
    /// legible there is the dial — but the tuning strip stays in the main window
    /// (or the controls window), so there is a single owner for the frequency
    /// and the two windows cannot disagree. A click or drag on the spectrum
    /// itself still tunes, as it does in the main window.
    #[cfg(not(target_arch = "wasm32"))]
    fn spectrum_toolbar(&mut self, ui: &mut egui::Ui, module: DetachableModule) {
        let area = ui.available_rect_before_wrap();
        ui.allocate_ui(egui::vec2(area.width(), SP_TOOLBAR_H), |ui| {
            let bar = ui.max_rect();
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(10.0, 0.0);
                ui.label(
                    RichText::new(readout_text(self.state.rx_freq_hz()))
                        .monospace()
                        .size(21.0)
                        .color(egui::Color32::from_rgb(255, 209, 66)),
                );
                ui.label(
                    RichText::new(detached_status_line(
                        self.state.rx[0].mode,
                        self.meters.as_ref(),
                    ))
                    .size(12.0)
                    .color(egui::Color32::LIGHT_GRAY),
                );
                crate::chrome::row_tail(ui, |ui| {
                    if crate::chrome::chip_window_icon(ui, crate::chrome::WindowIcon::In)
                        .on_hover_text(tr!("frame-dock-spectrum-hint"))
                        .clicked()
                    {
                        self.set_module_detached(module, false);
                    }
                });
            });
            // A hairline under the bar, so it reads as a bar and not as a row of
            // text floating over the picture.
            ui.painter().hline(
                egui::Rangef::new(bar.left(), bar.right()),
                bar.bottom(),
                egui::Stroke::new(1.0, crate::theme::LINE()),
            );
        });
    }

    /// One spectrum window's contents — SP1's or AUX SP's. The caller has put
    /// the right `self.view` in place (AUX swaps its own in around this call),
    /// so the draw itself is shared.
    #[cfg(not(target_arch = "wasm32"))]
    fn draw_detached_panadapter(
        &mut self,
        module: DetachableModule,
        ctx: &egui::Context,
        spec: &DetachedWindowSpec,
        seed: Option<sdroxide_types::DetachedWindow>,
        now: f64,
        cmds: &mut Vec<Command>,
    ) {
        let live =
            self.frame.is_some() && now - self.last_spectrum_at < crate::app::frame::STREAM_STALE_S;
        let frame = self.frame.clone();
        let inputs = self.panadapter_inputs(now);
        let mut clicked_spot: Option<Spot> = None;
        let child = ChildWindow::own(self);
        let outcome = detached_viewport(ctx, spec, seed, child, |ui| {
            // The toolbar across the top, then the spectrum below it.
            // `panel_below` is false: the operating panel is not under this
            // window, so the band-plan strip may use the rest of the height.
            self.spectrum_toolbar(ui, module);
            self.note_panadapter_width(ui);
            let wf_tuning = self.wf_tick(live, ui.ctx().pixels_per_point());
            let w = ui.available_width();
            let h = ui.available_height();
            self.draw_panadapter(
                ui,
                w,
                h,
                frame.as_ref(),
                &inputs,
                &mut clicked_spot,
                false,
                wf_tuning,
                cmds,
            );
        });
        self.handle_detached_outcome(ctx, module, spec, outcome);
        if let Some(spot) = clicked_spot {
            self.prefill_from_spot(&spot);
        }
    }

    /// Draw one module into its **own OS window**, from the focused radio's
    /// state.
    ///
    /// The **shell** calls this once per undocked module per frame — see
    /// [`Self::detached_wanted`] — so a window's existence is the shell's
    /// business and a module only has to say how to draw itself.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn show_detached_module(
        &mut self,
        module: DetachableModule,
        ctx: &egui::Context,
        now: f64,
    ) {
        use DetachableModule as M;
        let spec = detached_spec(module);
        let seed = self.ui_settings.detached_state(module).window;
        let mut cmds: Vec<Command> = Vec::new();
        match module {
            // SP1 and AUX SP are the same window drawn twice: one spectrum and
            // waterfall, its own toolbar, app-id and place on the desk. AUX is a
            // second view of the same receiver, but not a mirror of it — the
            // draw reads `self.view`, so AUX is drawn with its **own** view
            // swapped in, and the two can sit at different zooms and layers.
            M::Panadapter => {
                self.draw_detached_panadapter(module, ctx, &spec, seed, now, &mut cmds)
            }
            M::AuxPanadapter => {
                std::mem::swap(&mut self.view, &mut self.aux_view);
                self.draw_detached_panadapter(module, ctx, &spec, seed, now, &mut cmds);
                std::mem::swap(&mut self.view, &mut self.aux_view);
            }
            M::Panel => {
                let mode = self.state.rx[0].mode;
                let mut dock = false;
                // **This window's pane is its own**, swapped in around the draw:
                // the panel window may sit on the decode list while the main
                // window shows something else. A stored index from another mode,
                // or the waterfall tab, is clamped to a real pane first.
                let panes = crate::app::panels::panel_panes(mode).len();
                self.panel_window_pane = self.panel_window_pane.min(panes.saturating_sub(1));
                std::mem::swap(&mut self.view.digi_pane, &mut self.panel_window_pane);
                let child = ChildWindow::panes(self);
                let outcome = detached_viewport(ctx, &spec, seed, child, |ui| {
                    // **A title bar of its own.** A tiling window manager draws
                    // none, so the window's content carries the panel's name and
                    // the one control a window of its own needs: dock.
                    egui::Frame::new()
                        .fill(crate::theme::PANEL())
                        .inner_margin(egui::Margin::symmetric(6, 6))
                        .show(ui, |ui| {
                            ui.set_min_size(ui.available_size());
                            ui.allocate_ui_with_layout(
                                egui::vec2(ui.available_width(), crate::chrome::BAR_H),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new(mode.label())
                                            .size(crate::chrome::BAR_FONT)
                                            .color(crate::theme::CYAN()),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            dock = crate::chrome::chip_window_icon_sized(
                                                ui,
                                                crate::chrome::WindowIcon::Dock,
                                                egui::vec2(
                                                    crate::chrome::BAR_CHIP,
                                                    crate::chrome::BAR_CHIP,
                                                ),
                                            )
                                            .on_hover_text(tr!("frame-dock-panel-hint"))
                                            .clicked();
                                        },
                                    );
                                },
                            );
                            ui.separator();
                            // Narrow enough for one pane at a time: the chips that
                            // reach the others are drawn **here**, without the
                            // waterfall tab — there is no picture in this window.
                            if crate::layout::tier(ui.ctx()) == crate::layout::Tier::Phone {
                                self.digi_tabs(ui, mode, false);
                            }
                            let h = ui.available_height();
                            self.draw_operating_panel(ui, &mut cmds, mode, h);
                        });
                });
                std::mem::swap(&mut self.view.digi_pane, &mut self.panel_window_pane);
                self.handle_detached_outcome(ctx, module, &spec, outcome);
                if dock {
                    self.set_module_detached(M::Panel, false);
                }
            }
            M::Controls => {
                let child = ChildWindow::desk(self);
                let outcome = detached_viewport(ctx, &spec, seed, child, |ui| {
                    // The console is **desktop-shaped whatever its own height**:
                    // the compact strips are for a phone, and a control window
                    // that flipped to one the moment it opened would be the first
                    // thing anyone noticed. Closing the window docks it.
                    egui::Frame::new()
                        .fill(crate::theme::BG_DEEP())
                        .inner_margin(egui::Margin::symmetric(8, 6))
                        .show(ui, |ui| {
                            ui.set_min_size(ui.available_size());
                            crate::chrome::angled_frame(ui, crate::theme::PINK(), |ui| {
                                self.top_bar(ui, &mut cmds);
                            });
                        });
                });
                self.handle_detached_outcome(ctx, module, &spec, outcome);
            }
            M::BandMenu => {
                let child = ChildWindow::desk(self);
                let outcome = detached_viewport(ctx, &spec, seed, child, |ui| {
                    egui::Frame::new()
                        .fill(crate::theme::BG_DEEP())
                        .inner_margin(egui::Margin::symmetric(8, 6))
                        .show(ui, |ui| {
                            ui.set_min_size(ui.available_size());
                            // A header with its own DOCK chip, like the spectrum's
                            // toolbar: bring the selector home without hunting for
                            // the window's close box.
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(tr!("topbar-band-mode"))
                                        .size(11.0)
                                        .strong()
                                        .color(crate::theme::CYAN()),
                                );
                                crate::chrome::row_tail(ui, |ui| {
                                    if crate::chrome::chip_window_icon(
                                        ui,
                                        crate::chrome::WindowIcon::In,
                                    )
                                    .on_hover_text(tr!("frame-dock-band-menu-hint"))
                                    .clicked()
                                    {
                                        self.set_module_detached(M::BandMenu, false);
                                    }
                                });
                            });
                            ui.separator();
                            // The same body the docked column draws, so the
                            // window and the dock cannot come apart.
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .id_salt("band-window-scroll")
                                .show(ui, |ui| {
                                    self.band_menu_body(ui, &mut cmds);
                                });
                        });
                });
                self.handle_detached_outcome(ctx, module, &spec, outcome);
            }
        }
        self.dispatch_commands(cmds);
    }

    /// Fold a detached window's outcome into the settings: remember its geometry
    /// once a drag settles, and dock it again when the operator closes it. The
    /// one place every module's outcome is handled, so their behaviour cannot
    /// drift apart.
    #[cfg(not(target_arch = "wasm32"))]
    fn handle_detached_outcome(
        &mut self,
        ctx: &egui::Context,
        module: DetachableModule,
        spec: &DetachedWindowSpec,
        outcome: DetachedOutcome,
    ) {
        let mut persist = false;
        {
            let state = &mut self.ui_settings.detached[module.index()];
            // Persist once a drag settles — the frame after the geometry stops
            // changing — so a resize in progress is not a config write a frame.
            if let Some(g) = outcome.geometry {
                let last: Option<sdroxide_types::DetachedWindow> =
                    ctx.data(|d| d.get_temp(spec.settle_id));
                if last == Some(g) && state.window != Some(g) {
                    state.window = Some(g);
                    persist = true;
                }
                ctx.data_mut(|d| d.insert_temp(spec.settle_id, g));
            }
            if outcome.close_requested {
                // Closing the window is docking it again, not an invitation to
                // re-open it next frame — and the frame it closed on still
                // carries the geometry to keep.
                if let Some(g) = outcome.geometry {
                    state.window = Some(g);
                }
                state.detached = false;
                persist = true;
            }
        }
        if persist {
            crate::app::persist::persist_ui_settings(&self.ui_settings);
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn the_detached_readout_trims_to_the_dial() {
        assert_eq!(readout_text(27_265_000.0), "27.265 MHz");
        assert_eq!(readout_text(144_800_000.0), "144.8 MHz");
        assert_eq!(readout_text(10_120_600.0), "10.1206 MHz");
        assert_eq!(readout_text(14_000_000.0), "14 MHz");
    }

    /// A window saved on a monitor that is no longer there opens where it can be
    /// reached; one saved on a monitor to the left (negative x) is left alone.
    #[test]
    fn a_window_saved_off_the_screen_comes_back_onto_it() {
        let monitor = Some(egui::vec2(1920.0, 1080.0));
        let seed = |x: f32, y: f32| {
            Some(sdroxide_types::DetachedWindow { size: [800.0, 600.0], pos: Some([x, y]) })
        };
        let (_, pos) = detached_geometry(monitor, seed(3000.0, 1500.0), [960.0, 540.0]);
        let pos = pos.unwrap();
        assert!(pos.x <= 1920.0 - 80.0 && pos.y <= 1080.0 - 60.0, "still off screen: {pos:?}");
        let (_, pos) = detached_geometry(monitor, seed(-1500.0, 100.0), [960.0, 540.0]);
        assert_eq!(pos, Some(egui::pos2(-1500.0, 100.0)), "a monitor to the left was undone");
    }

    /// The window opens fitted to the monitor it is about to open on, and never
    /// larger than it was asked to be.
    #[test]
    fn the_detached_window_opens_fitting_the_monitor() {
        let (size, _) = detached_geometry(Some(egui::vec2(1280.0, 720.0)), None, [1280.0, 640.0]);
        assert!(size.x <= 1280.0 && size.y <= 720.0, "{size:?} does not fit 1280x720");
        let (size, _) = detached_geometry(None, None, [960.0, 540.0]);
        assert_eq!(size, egui::vec2(960.0, 540.0), "no monitor reported: the default stands");
    }
}
