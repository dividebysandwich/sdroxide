//! The solar-system view as a whole application, for the browser tab that the
//! ☀ 3D chip opens.
//!
//! Natively this view is a child viewport of the main window and `Solar3d`
//! drives it from the root pass. In the browser there is no such parent: the
//! tab is its own wasm instance with its own canvas and its own wgpu device.
//! That turns out to be an advantage — the device is free to carry depth and
//! MSAA without the waterfall pipelines in the same process having to match.
//!
//! What this type does is exactly what `Solar3d::viewport` does per frame,
//! minus the viewport: drain the data source, publish the operator's QTH and
//! QSO traffic into [`SolarUi`], call [`overlay::ui`], and forward whatever the
//! overlay asked for back to the data source.

use eframe::egui;

use sdroxide_proto::solar::SolarClientMsg;

use super::net::{Link, SolarClient};
use super::overlay;
use super::state::SolarUi;
use crate::digi_map::DigiStations;
use crate::view::Solar3dView;

/// Storage key for the persisted camera/layer/scale settings.
///
/// Deliberately not `"view"`: that is the main page's whole [`crate::view::ViewState`],
/// and the two tabs share an origin. Same-named keys would have them overwrite
/// each other's settings on every save.
const STORAGE_KEY: &str = "solar3d";

pub struct SolarApp {
    state: SolarUi,
    net: SolarClient,
    stations: DigiStations,
    /// Reported once, so a failed connection does not spam the log every frame.
    reported: Option<Link>,
    /// The sign-in this tab is challenged with, exactly as the main one is.
    /// Usually invisible: the main tab's "remember" box is what lets this one
    /// answer without asking, and both tabs share an origin so both find it.
    login: crate::login::LoginForm,
}

impl SolarApp {
    /// `url` is the `/solar-ws` endpoint. A connection failure is not fatal:
    /// the ephemeris is arithmetic, so the Sun, the planets and the operator's
    /// own QTH still draw. Only the live products go missing, and the overlay's
    /// freshness readouts already say so.
    pub fn new(cc: &eframe::CreationContext<'_>, url: &str) -> Result<Self, String> {
        // The browser's solar tab is its own egui instance, so it reads the
        // operator's theme out of the shared storage before applying — the
        // native 3D window instead shares the main app's context and atomics
        // and never gets here.
        let ui = crate::app::persist::load_ui_settings(cc.storage);
        crate::theme::set_look(ui.theme, ui.button_style, ui.window_style);
        crate::theme::set_font_sizes(
            ui.skimmer_font_size,
            ui.waterfall_font_size,
            ui.menu_font_size,
        );
        crate::theme::set_map_cities(ui.map_cities);
        crate::theme::apply(&cc.egui_ctx);
        if let Some(rs) = cc.wgpu_render_state.as_ref() {
            super::gpu::init(rs);
        }

        let view: Solar3dView =
            cc.storage.and_then(|s| eframe::get_value(s, STORAGE_KEY)).unwrap_or_default();

        let ctx = cc.egui_ctx.clone();
        let net = SolarClient::connect(url, move || crate::repaint::after_ms(&ctx, 33))?;

        let mut state = SolarUi::new(view);
        state.data = Some(net.shared());
        Ok(SolarApp {
            state,
            net,
            stations: DigiStations::default(),
            reported: None,
            login: Default::default(),
        })
    }

    /// The contact card's content, from what the relay has told this tab.
    ///
    /// The same shape the main window builds, and deliberately from the same
    /// two ends: the operator's grid and the DX's, so the distance describes
    /// the arc actually drawn. The live `SIG` comes out of the relayed decode
    /// list, which is the same list the dots are drawn from — this tab does its
    /// own fade bookkeeping on it and can read a signal report off it too.
    fn qso_card_info(&self) -> Option<super::QsoInfo> {
        let call = self.net.dx_call.clone().filter(|c| !c.trim().is_empty())?;
        let home = sdroxide_types::grid_to_latlon(&self.net.my_grid);
        let dx = self.net.dx_grid.as_deref().and_then(sdroxide_types::grid_to_latlon);
        Some(super::QsoInfo {
            entity: sdroxide_types::entity_name(&call),
            snr_db: self
                .net
                .decodes
                .iter()
                .find(|d| d.from.as_deref().is_some_and(|f| f.eq_ignore_ascii_case(&call)))
                .map(|d| d.snr_db),
            call,
            mode: self.net.mode.clone(),
            grid: self.net.dx_grid.clone().filter(|g| !g.trim().is_empty()),
            distance_km: home.zip(dx).map(|(a, b)| sdroxide_types::distance_km(a, b)),
            bearing_deg: home.zip(dx).map(|(a, b)| sdroxide_types::bearing_deg(a, b)),
            started_utc: self.net.qso.map(|q| q.started_utc).filter(|t| *t > 0),
            rpt_sent: self.net.qso.and_then(|q| q.rpt_sent),
            rpt_rcvd: self.net.qso.and_then(|q| q.rpt_rcvd),
            band: if self.net.freq_hz > 0.0 {
                sdroxide_types::adif_band(self.net.freq_hz).to_string()
            } else {
                String::new()
            },
        })
    }
}

impl eframe::App for SolarApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.net.poll();

        // The same door as the main tab. Drawn in place of the view rather
        // than over it: without the feed there is no Sun, no aurora and no
        // satellites, so what is behind this is a scene of exactly one planet.
        let phase = self.net.auth.clone();
        // The same door as the radios of this station, so the sign-in the
        // operator made in a radio tab lets the map in too.
        let station = crate::login::station_key(self.net.url());
        self.login.settle(&phase, &station);
        if phase.is_pending() {
            let rs = frame.wgpu_render_state();
            if let Some(login) = crate::login::screen(ui, &mut self.login, &phase, rs) {
                self.net.send_auth(login.username, login.password);
            }
            crate::repaint::after_ms(ui.ctx(), 250);
            return;
        }

        // Report each transition once, not every frame. The overlay already
        // tells the honest story on its own — with no link, no source ever
        // reports a successful fetch and every freshness readout says so — so
        // this is for the console, not for the operator.
        if self.reported.as_ref() != Some(&self.net.link) {
            if let Link::Down(e) = &self.net.link {
                eprintln!("sdroxide: solar feed link down: {e}");
            }
            self.reported = Some(self.net.link.clone());
        }

        // The fade bookkeeping is the panel map's, run here against the relayed
        // decodes so the browser's globe shows the same stations the main tab
        // does. `preview` has no equivalent: a clicked-but-unanswered decode is
        // state of the main window, which this tab is not.
        let now_t = ui.input(|i| i.time);
        self.stations.observe(&self.net.decodes, now_t, crate::time::now_unix());
        self.state.set_qth(&self.net.my_grid.clone());
        self.state.digi =
            self.stations.traffic(now_t, self.net.dx_grid.as_deref(), None, self.net.transmitting);
        self.state.digi.qso = self.qso_card_info();
        // The operator's satellite frequencies, as the server last relayed
        // them. Compared by pointer: the client replaces the whole `Arc` when a
        // new table arrives, so this is one word per frame rather than a walk
        // of the table.
        if !std::sync::Arc::ptr_eq(&self.state.sat_cfg, &self.net.sat_cfg) {
            self.state.sat_cfg = std::sync::Arc::clone(&self.net.sat_cfg);
        }
        // Same pointer comparison, same reason: the client replaces the whole
        // field when a new one lands, so this is a word rather than a walk of a
        // few hundred kilobytes.
        if !std::sync::Arc::ptr_eq(&self.state.prop, &self.net.prop) {
            self.state.prop = std::sync::Arc::clone(&self.net.prop);
        }

        overlay::ui(ui, &mut self.state);

        // Drain what the overlay asked for. `close_requested` is never set: the
        // browser owns the tab, and there is no window for us to stop emitting.
        if std::mem::take(&mut self.state.refresh_requested) {
            self.net.send(&SolarClientMsg::RefreshAll);
        }
        self.net.sync_channel(self.state.view.channel, self.state.view.resolution);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, STORAGE_KEY, &self.state.view);
    }
}
