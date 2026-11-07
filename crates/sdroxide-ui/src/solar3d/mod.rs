//! Solar-system 3D view: Sun, Earth and Moon with their orbits, the operator's
//! QTH on the globe, live SDO solar imagery, and DONKI sunspot / CME data.
//!
//! The scene, camera, overlay and renderer are shared by both targets. What
//! differs is only how the view is hosted and where its data comes from:
//!
//! * **Native** — [`Solar3d`] emits a deferred egui viewport, which desktop
//!   eframe turns into a real second OS window, and owns a [`SolarFeed`] that
//!   fetches from NASA and NOAA directly.
//!   [`SolarFeed`]: sdroxide_solar::SolarFeed
//! * **Browser** — [`SolarApp`] is the whole app of a second tab, and its data
//!   arrives over the server's `/solar-ws` relay. There is no viewport to
//!   manage: the browser owns the tab.
//!
//! Both end in the same [`overlay::ui`] call against the same [`SolarUi`], so
//! there is one implementation of the view itself.
//!
//! Unlike the rest of this crate, this module is *not* written to WebGL2
//! downlevel limits: it uses a depth buffer, MSAA and vertex buffers. All of it
//! is confined to an offscreen pass, so the shared egui render pass — which on
//! the web must stay within those limits — is untouched.

#[cfg(feature = "remote")]
mod app;
mod camera;
mod dotmatrix;
mod gpu;
mod math;
mod mesh;
#[cfg(feature = "remote")]
mod net;
mod overlay;
mod scene;
mod state;

// Everything below is the native host's: the browser's is in `app.rs`.
#[cfg(not(target_arch = "wasm32"))]
use crate::i18n::tr;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Arc, Mutex, MutexGuard};

#[cfg(not(target_arch = "wasm32"))]
use eframe::egui;

#[cfg(not(target_arch = "wasm32"))]
use crate::egui_wgpu::RenderState;
#[cfg(not(target_arch = "wasm32"))]
use crate::view::Solar3dView;

#[cfg(feature = "remote")]
pub use app::SolarApp;
#[cfg(not(target_arch = "wasm32"))]
use state::SolarUi;
pub use state::{DigiTraffic, QsoInfo};

/// Stable id of the child viewport, distinct per radio: a split view can
/// have two radios' solar windows up at once, and one id would be one window
/// both fought over. Radio 0 keeps the historical hash. Not a `const` because
/// the id hashes at runtime; it is a couple of instructions per frame.
#[cfg(not(target_arch = "wasm32"))]
fn viewport_id(salt: u32) -> egui::ViewportId {
    if salt == 0 {
        egui::ViewportId::from_hash_of("sdroxide-solar3d")
    } else {
        egui::ViewportId::from_hash_of(("sdroxide-solar3d", salt))
    }
}

/// The size and position the solar-system window opens at: the operator's last
/// geometry if there is one, otherwise 1180×760, either way shrunk to fit the
/// monitor it is about to open on.
///
/// A window wider than the screen loses the controls along its right edge, and
/// on some Linux systems a window larger than the monitor crashes the toolkit —
/// the same bargain [`crate::layout::fit_inner_size`] strikes for the floating
/// windows. The frame around the drawing area is not known before the window
/// exists, so a title bar and borders are guessed. Never grows a window: an
/// operator who has sized theirs is not asking for it back.
///
/// The position comes back as it was given, only where the platform gave one
/// (not Wayland), and is not clamped. It is in desktop coordinates, and a
/// second monitor starts wherever the desktop puts it — often at x = 1920, or
/// at a negative x left of the primary — while `monitor` is only a size. A
/// clamp to `0..monitor` put a window left on the second screen back on the
/// first; the window manager already keeps a window on some screen.
#[cfg(not(target_arch = "wasm32"))]
fn solar3d_geometry(
    monitor: Option<egui::Vec2>,
    seed: Option<sdroxide_types::Solar3dWindow>,
) -> (egui::Vec2, Option<egui::Pos2>) {
    let want = seed.map_or(egui::vec2(1180.0, 760.0), |s| egui::Vec2::from(s.size));
    let Some(monitor) = monitor.filter(|m| m.x > 1.0 && m.y > 1.0) else {
        return (want, seed.and_then(|s| s.pos).map(egui::Pos2::from));
    };
    let chrome = egui::vec2(16.0, 40.0);
    let size =
        crate::layout::fit_inner_size(monitor, want + chrome, want, egui::vec2(520.0, 340.0))
            .unwrap_or(want);
    (size, seed.and_then(|s| s.pos).map(egui::Pos2::from))
}

// Scene units are gigametres (10⁶ km): 1 AU ≈ 149.6, the Sun ≈ 0.696, the Earth
// ≈ 0.0064. Everything the camera deals with then sits in [1e-3, 1e3], well
// inside f32 range, while the ephemeris itself stays f64.
/// Earth mean radius, Gm.
pub const EARTH_R: f32 = 0.006_371;
/// Moon mean radius, Gm.
pub const MOON_R: f32 = 0.001_737_4;
/// Mean Earth–Moon distance, Gm.
pub const MOON_DIST: f32 = 0.384_4;

/// Largest body exaggeration that still keeps the Moon outside the Earth.
///
/// Radii are exaggerated but the Earth–Moon distance is not, so past this the
/// two spheres interpenetrate and the view becomes nonsense. Raising
/// `moon_orbit_scale` is what buys headroom.
pub fn max_body_scale(moon_orbit_scale: f32) -> f32 {
    0.45 * moon_orbit_scale * MOON_DIST / (EARTH_R + MOON_R)
}

/// A satellite-lock change requested inside the 3D window, handed back to the
/// root pass — which owns the command path to the engine that this window,
/// living behind an `Arc<Mutex<..>>`, cannot reach.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LockChange {
    Lock(u64),
    Unlock,
}

/// App-side handle to the solar-system window.
///
/// Native only: it owns a [`sdroxide_solar::SolarFeed`] and a child viewport,
/// neither of which the browser build has. There, [`SolarApp`] is the host.
#[cfg(not(target_arch = "wasm32"))]
pub struct Solar3d {
    /// Whether the window should exist this frame. Toggled by the Display-box
    /// chip and cleared when the OS window is closed.
    pub open: bool,
    state: Arc<Mutex<SolarUi>>,
    /// Shared wgpu state, stashed so the GPU resources can be built on first
    /// open rather than at app construction — most sessions never open this
    /// window, and it allocates a few tens of MB.
    render_state: Option<RenderState>,
    gpu_ready: bool,
    /// The background data feed. Started when the window opens and **dropped
    /// when it closes**, which is what confines this feature's network activity
    /// to the window's lifetime: never opening it makes no request at all.
    feed: Option<sdroxide_solar::SolarFeed>,
    /// Channel/resolution the running feed was started with, so a change in the
    /// UI can be forwarded rather than restarting the thread.
    feed_channel: (u8, u16),
    /// The custom element sets the running feed was last told about. SGP4
    /// constants are not free to rebuild, so the listing is only re-sent when
    /// the operator actually changes it.
    feed_tles: String,
    /// Likewise for the subscribed listings: re-sending these would refetch
    /// nothing (the cache decides that) but would still reparse every one.
    feed_subs: Vec<sdroxide_types::TleSubscription>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Solar3d {
    pub fn new(render_state: Option<RenderState>, view: Solar3dView) -> Self {
        Solar3d {
            open: view.open,
            state: Arc::new(Mutex::new(SolarUi::new(view))),
            render_state,
            gpu_ready: false,
            feed: None,
            feed_channel: (view.channel, view.resolution),
            feed_tles: String::new(),
            feed_subs: Vec::new(),
        }
    }

    /// Lock the shared state, recovering from a poisoned mutex: a panic inside
    /// the render closure should not turn into a panic on every later frame.
    fn lock(&self) -> MutexGuard<'_, SolarUi> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The settings to persist in `ViewState`.
    pub fn persisted(&self) -> Solar3dView {
        let mut v = self.lock().view;
        v.open = self.open;
        v
    }

    /// Publish the geometry the window should open at, from the screen's
    /// settings. Consulted only when the window is (re)built.
    pub fn set_window_seed(&self, window: Option<sdroxide_types::Solar3dWindow>) {
        self.lock().window_seed = window;
    }

    /// The geometry the window is actually at, for the host to keep in the
    /// screen's settings.
    pub fn window_now(&self) -> Option<sdroxide_types::Solar3dWindow> {
        self.lock().window_now
    }

    /// Whether the clouds are marched rather than sliced. Lives here rather than
    /// in `UiSettings` because it rides `Solar3dView`, which is the state the
    /// browser tab persists too — the same switch has to reach both.
    pub fn cloud_march(&self) -> bool {
        self.lock().view.cloud_march
    }

    pub fn set_cloud_march(&self, on: bool) {
        self.lock().view.cloud_march = on;
    }

    /// The published band conditions, if this window's feed has fetched them.
    ///
    /// The one product of this feed the *main* window reads, because it is the
    /// one that answers a question asked from the band menu rather than from
    /// here. `None` whenever the window has not been open, and that is not a
    /// problem: the main window fetches these itself on its own hourly timer
    /// (see `app::persist::spawn_band_conditions_fetch`) and both go through
    /// the same disk cache, so this is only ever the fresher of two copies
    /// rather than the only one.
    pub fn band_conditions(&self) -> Option<sdroxide_solar::BandConditions> {
        let st = self.lock();
        let shared = st.data.as_ref()?;
        let d = shared.lock().unwrap_or_else(|e| e.into_inner());
        d.weather.band_conditions.clone()
    }

    /// Emit the window for this frame (or not, when closed). Call once per root
    /// pass, after the main UI. `grid` is the operator's Maidenhead locator and
    /// `awards` the log's DXCC coverage for the "what is still missing" layer.
    /// `sat_lock` is the engine's satellite lock, by catalogue number.
    ///
    /// Returns a lock change requested inside the window (the pass window's
    /// LOCK/UNLOCK button) — the root pass owns the command path this window
    /// cannot reach.
    pub fn viewport(
        &mut self,
        ctx: &egui::Context,
        grid: &str,
        traffic: DigiTraffic,
        awards: Arc<Vec<sdroxide_types::EntitySlot>>,
        prop: Arc<sdroxide_types::PropField>,
        sat_cfg: Arc<sdroxide_types::SatConfig>,
        sat_lock: Option<u64>,
    ) -> Option<LockChange> {
        if !self.open {
            // Dropping the feed disconnects the worker's channel, which is how
            // it learns to stop. Closing the window therefore ends all network
            // activity, which is the behaviour the manual promises.
            if self.feed.take().is_some() {
                self.lock().data = None;
                // The next feed is a new thread that knows nothing, so forget
                // what the old one was told or the custom sets would never be
                // re-sent after a close and reopen.
                self.feed_tles.clear();
                self.feed_subs.clear();
            }
            return None;
        }

        if !self.gpu_ready {
            if let Some(rs) = &self.render_state {
                gpu::init(rs);
            }
            self.gpu_ready = true;
        }
        self.ensure_feed(ctx);
        self.ensure_custom_tles(&sat_cfg);

        // Publish this frame's inputs, then drop the guard: the deferred
        // callback takes the same lock, and on the embedded-viewport path it
        // would run synchronously inside `show_viewport_deferred`.
        {
            let mut st = self.lock();
            st.set_qth(grid);
            st.digi = traffic;
            st.awards = awards;
            st.prop = prop;
            st.sat_cfg = sat_cfg;
            st.sat_lock = sat_lock;
        }

        let state = Arc::clone(&self.state);
        let vid = viewport_id(crate::layout::radio_salt(ctx));
        // The window is (re)built on the frame egui has no pass count for it —
        // which is where the size and place come from. Every later frame leaves
        // the geometry alone, so a live resize is not fought by our own request.
        let mut builder = egui::ViewportBuilder::default()
            .with_title(tr!("solarwin-sdroxide-solar-system"))
            .with_min_inner_size([520.0, 340.0])
            .with_clamp_size_to_monitor_size(true);
        if ctx.cumulative_pass_nr_for(vid) == 0 {
            let seed = self.lock().window_seed;
            let monitor = ctx.input(|i| i.viewport().monitor_size);
            let (size, pos) = solar3d_geometry(monitor, seed);
            builder = builder.with_inner_size(size);
            if let Some(pos) = pos {
                builder = builder.with_position(pos);
            }
        }
        ctx.show_viewport_deferred(vid, builder, move |ui, _class| {
            let mut st = state.lock().unwrap_or_else(|e| e.into_inner());
            // Keep the geometry for the next rebuild. `content_rect` is the
            // inner size as the toolkit reports it — the only size on Wayland,
            // where `viewport().inner_rect` is `None` because a Wayland client
            // is given no absolute position; `outer_rect` carries the position
            // where the platform has one, and is `None` on Wayland too.
            let size = ui.ctx().content_rect().size();
            if size.x > 1.0 && size.y > 1.0 {
                let pos = ui.ctx().input(|i| i.viewport().outer_rect).map(|r| r.min);
                let geom = sdroxide_types::Solar3dWindow {
                    size: [size.x.round(), size.y.round()],
                    pos: pos.map(|p| [p.x.round(), p.y.round()]),
                    pos_px: None,
                };
                if st.window_now != Some(geom) {
                    st.window_now = Some(geom);
                }
            }
            if ui.ctx().input(|i| i.viewport().close_requested()) {
                st.close_requested = true;
                // Wake the root pass so the chip un-lights promptly.
                ui.ctx().request_repaint_of(egui::ViewportId::ROOT);
                return;
            }
            overlay::ui(ui, &mut st);
        });

        // egui tears the window down when we stop emitting the viewport — do
        // *not* send `ViewportCommand::Close` as well.
        let (close, refresh, lock_req, unlock_req) = {
            let mut st = self.lock();
            (
                std::mem::take(&mut st.close_requested),
                std::mem::take(&mut st.refresh_requested),
                std::mem::take(&mut st.lock_requested),
                std::mem::take(&mut st.unlock_requested),
            )
        };
        if close {
            self.open = false;
        }
        if refresh {
            self.refresh();
        }
        if unlock_req {
            return Some(LockChange::Unlock);
        }
        lock_req.map(LockChange::Lock)
    }

    /// Start the data feed on first open, and forward channel/resolution
    /// changes to the running one rather than restarting the thread.
    fn ensure_feed(&mut self, ctx: &egui::Context) {
        let (channel, resolution) = {
            let st = self.lock();
            (st.view.channel, st.view.resolution)
        };

        if self.feed.is_none() {
            let wake_ctx = ctx.clone();
            // Captured now: the closure fires from the feed thread, long after
            // this frame's salt has been overwritten by another radio's.
            let vid = viewport_id(crate::layout::radio_salt(ctx));
            let feed = sdroxide_solar::SolarFeed::start(
                sdroxide_solar::SdoChannel::from_u8(channel),
                resolution as u32,
                // Wake only this window: the SDR UI has no reason to redraw
                // because a solar image arrived.
                move || wake_ctx.request_repaint_of(vid),
            );
            self.lock().data = Some(feed.shared());
            self.feed = Some(feed);
            self.feed_channel = (channel, resolution);
            return;
        }

        if self.feed_channel != (channel, resolution) {
            if let Some(feed) = &self.feed {
                if self.feed_channel.0 != channel {
                    feed.send(sdroxide_solar::FeedCmd::SetChannel(
                        sdroxide_solar::SdoChannel::from_u8(channel),
                    ));
                }
                if self.feed_channel.1 != resolution {
                    feed.send(sdroxide_solar::FeedCmd::SetResolution(resolution as u32));
                }
            }
            self.feed_channel = (channel, resolution);
        }
    }

    /// Push the operator's own element sets into the feed when they change.
    ///
    /// The listing is compared rather than the config, so editing a frequency
    /// entry — which the tracker does not care about — costs nothing.
    fn ensure_custom_tles(&mut self, cfg: &sdroxide_types::SatConfig) {
        let Some(feed) = &self.feed else { return };
        let text = cfg.tle_text();
        if text != self.feed_tles {
            feed.send(sdroxide_solar::FeedCmd::SetCustomTles(text.clone()));
            self.feed_tles = text;
        }
        let subs: Vec<_> = cfg.live_subs().cloned().collect();
        if subs != self.feed_subs {
            feed.send(sdroxide_solar::FeedCmd::SetTleSubs(subs.clone()));
            self.feed_subs = subs;
        }
    }

    /// What each TLE subscription's last fetch did, for the settings dialog.
    ///
    /// Empty while the window has never been open this session: the feed — and
    /// with it every fetch — only exists while the window does. The settings
    /// dialog falls back to reading the disk cache in that case, so a
    /// subscription still reports its age with the window shut.
    pub fn tle_sub_status(&self) -> Vec<sdroxide_solar::SubStatus> {
        let st = self.lock();
        let Some(data) = &st.data else { return Vec::new() };
        data.lock().unwrap_or_else(|e| e.into_inner()).tle_subs.clone()
    }

    /// Re-read the subscribed listings from the disk cache.
    ///
    /// Used after the settings dialog fetches them itself: the feed shares the
    /// cache but has no way to know somebody else just wrote to it, so without
    /// this the window would keep showing the previous listing until its own
    /// six-hourly refresh came round.
    pub fn reload_tle_subs(&mut self) {
        if let Some(feed) = &self.feed {
            feed.send(sdroxide_solar::FeedCmd::SetTleSubs(self.feed_subs.clone()));
        }
    }

    /// Ask the feed to re-fetch everything now (the overlay's ↻ button).
    pub fn refresh(&self) {
        if let Some(feed) = &self.feed {
            feed.send(sdroxide_solar::FeedCmd::RefreshAll);
        }
    }
}

/// Seconds since the Unix epoch. The ephemeris takes an explicit timestamp
/// everywhere, so the whole scene can be scrubbed by offsetting this.
fn wall_clock_unix() -> f64 {
    crate::time::now_unix_f64()
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    /// The default opens on a screen with room for it, and is shrunk to fit one
    /// that is smaller rather than hanging off the edge — the tab-switch report
    /// was a window wider than the screen coming straight back on every rebuild.
    #[test]
    fn the_solar_window_opens_within_the_monitor() {
        let default = egui::vec2(1180.0, 760.0);
        // Room to spare: the default is left alone...
        assert_eq!(solar3d_geometry(Some(egui::vec2(1920.0, 1080.0)), None), (default, None));
        // ...as it is before any monitor has been reported.
        assert_eq!(solar3d_geometry(None, None), (default, None));
        // A small screen shrinks it instead of overflowing.
        let (small, _) = solar3d_geometry(Some(egui::vec2(1280.0, 720.0)), None);
        assert!(small.x <= 1280.0 && small.y <= 720.0, "{small:?} does not fit 1280x720");
        assert!(small.x < default.x || small.y < default.y, "a small screen must shrink it");
    }

    /// A geometry the operator gave the window is honoured, but never grown
    /// past a screen that has since become smaller.
    #[test]
    fn a_remembered_geometry_is_honoured_and_its_size_still_fits() {
        let seed = sdroxide_types::Solar3dWindow {
            size: [900.0, 600.0],
            pos: Some([100.0, 80.0]),
            pos_px: None,
        };
        assert_eq!(
            solar3d_geometry(Some(egui::vec2(2560.0, 1440.0)), Some(seed)),
            (egui::vec2(900.0, 600.0), Some(egui::pos2(100.0, 80.0)))
        );
        // A shrunken screen shrinks the window to fit it.
        let (size, _) = solar3d_geometry(
            Some(egui::vec2(800.0, 600.0)),
            Some(sdroxide_types::Solar3dWindow {
                size: [1600.0, 1200.0],
                pos: Some([700.0, 500.0]),
                pos_px: None,
            }),
        );
        assert!(size.x <= 800.0 && size.y <= 600.0, "{size:?} does not fit 800x600");
    }

    /// The position is in desktop coordinates, and `monitor` is only a size:
    /// a window left on a second screen — right of a 1920-wide primary, or
    /// left of it at a negative x — comes back there rather than being pulled
    /// onto the first.
    #[test]
    fn a_window_on_a_second_monitor_comes_back_there() {
        let screen = Some(egui::vec2(1920.0, 1080.0));
        for at in [[2100.0, 120.0], [-1500.0, 200.0]] {
            let seed =
                sdroxide_types::Solar3dWindow { size: [900.0, 600.0], pos: Some(at), pos_px: None };
            let (_, pos) = solar3d_geometry(screen, Some(seed));
            assert_eq!(pos, Some(egui::Pos2::from(at)), "moved off the monitor it was left on");
        }
    }
}
