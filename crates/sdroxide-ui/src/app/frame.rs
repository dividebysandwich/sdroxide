//! The per-frame loop: drain what the engine sent, then lay the window out.
//!
//! [`eframe::App::update`] is the one entry point the framework calls, and
//! everything else in [`crate::app`] hangs off it. Event handling comes first
//! so the frame draws the newest state, and the repaint request at the end is
//! what keeps the app idling instead of spinning when no stream is flowing.

use eframe::egui::{self, Color32, RichText};
use sdroxide_types::{Band, Command, Mode, RadioEvent, SpectrumConfig, Spot};

use crate::time::now_unix;
use crate::widgets::spectrum_view;

use crate::app::SdroxideApp;
use crate::app::net::auto_upload_adif;
use crate::app::persist::persist_qso_log;
use crate::app::settings::servers::TciServerStatus;
use crate::app::spectrum::CFG_DEBOUNCE_S;

/// Repaint-poll cadence when no spectrum stream is flowing (startup, connection
/// lost, stalled stream) — the app truly idles between these wakes.
const IDLE_POLL_MS: u64 = 250;

/// The stream counts as stalled after this long without a new frame (seconds).
const STREAM_STALE_S: f64 = 1.0;

/// Split the digital-mode area between the waterfall and the operating panel.
///
/// Returns `(waterfall height, panel height)`, which always sum to
/// `total - divider_h`: the panel is clamped and the waterfall takes what is
/// left, rather than both being floored independently and their sum allowed
/// past the height there actually is.
///
/// `divider_h` is everything between the two — the drag handle *and* the two
/// gaps the layout puts either side of it. Leaving the gaps out of it is what
/// let the panel start `2 × item_spacing` below where the arithmetic thought it
/// did, and hang that far off the bottom of the window: ten points on a desktop,
/// which the transcript quietly absorbed, and fourteen on a touched layout,
/// which took the action buttons with it.
///
/// `row_h` is the layout's own row height (`interact_size.y`), and it is what
/// the panel's floor is built from. Every part of the panel that cannot be done
/// without — its header, the station card, the two rows pinned to its bottom
/// edge — is a chip, a button or a text field sized from that, and the touched
/// layouts make each of them half again as tall. The 190 points this used to be
/// was measured on a desktop, so on a 1366×768 screen (which is the tablet tier,
/// touch metrics and all) the panel came up about fifty points short: the
/// transcript's scroll area hit its own minimum, overflowed the allocation it
/// had been given, and painted over the message row underneath — so the row
/// that chooses what goes out next was simply not on the screen, with nothing
/// to say it was missing (issue #231).
fn digi_split(total: f32, divider_h: f32, fraction: f32, row_h: f32) -> (f32, f32) {
    let usable = (total - divider_h).max(0.0);
    // 170 points of fixed furniture plus four rows: measured against a 1290×724
    // window, where 296 points is where the transcript stops being squeezed
    // into less than its own scroll area will accept. Capped at 60% of the
    // area so a floor can never leave the waterfall a sliver.
    let min_panel = (170.0 + 4.0 * row_h.max(18.0)).min(usable * 0.6);
    let min_wf = 80.0_f32.min(usable * 0.5);
    let panel_h = (usable * fraction).clamp(min_panel, (usable - min_wf).max(min_panel));
    (usable - panel_h, panel_h)
}

/// Whether a state update that kept the mode moved the receiver far enough to
/// throw the copied decodes away.
///
/// Band-level rather than a frequency delta, the way WSJT-X's own
/// `band_changed()` is: tuning about within a band — the FT8 slot to the FT4
/// slot, a DXpedition window, the other end of the digital segment — is still
/// the same band opening, and the list is still about it. Crossing into another
/// band is a different opening and a different set of stations, and a decode
/// carries nothing that would say which of the two it came from.
///
/// `now`/`prev` come from the engine's own [`sdroxide_types::RadioState::band`]
/// rather than being derived here, so a QSY made at the radio counts exactly as
/// one made in the program — the engine sets that field from the dial the CAT
/// poll reports, and this comparison never has to know where the change came
/// from.
///
/// WSPR is exempt: its band hopping crosses a band edge once a slot on purpose,
/// and its spots each carry their own RF frequency, so there is nothing
/// ambiguous to clear away.
fn qsy_clears_decodes(prev: Band, now: Band, mode: Mode) -> bool {
    now != prev && !mode.is_wspr()
}

/// The banner palette above the panadapter — (wash, rule, mark, ink), one
/// look shared by the radio warnings and the update notice. Amber wash, amber
/// rule, readable ink; the light-theme set is the same banner turned the
/// other way up, for a theme whose panels are white and which would otherwise
/// get pale text on a dark strip in the middle of a bright page.
fn notice_banner_colors() -> (Color32, Color32, Color32, Color32) {
    if crate::theme::is_light() {
        (
            Color32::from_rgb(255, 243, 205),
            Color32::from_rgb(178, 122, 0),
            Color32::from_rgb(140, 92, 0),
            Color32::from_rgb(58, 44, 8),
        )
    } else {
        (
            Color32::from_rgb(60, 45, 10),
            Color32::from_rgb(210, 160, 40),
            Color32::from_rgb(255, 190, 70),
            Color32::from_rgb(240, 220, 180),
        )
    }
}

impl eframe::App for SdroxideApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let now = ctx.input(|i| i.time);
        // The rate every animation in the tree paces itself to. Published here,
        // before anything draws, so a change in Settings → UI reaches the
        // needle and the waterfall on the same frame it reaches the scheduler.
        crate::repaint::set_frame_period_ms(1000 / u64::from(self.ui_settings.fps().max(1)));
        // Which radio the ids minted below belong to. Split view draws several
        // apps per frame; each declares itself before it draws anything.
        crate::layout::set_radio_salt(&ctx, self.radio_id);
        // Settle the layout for this frame before anything draws. It is a
        // property of the viewport plus the operator's override, and the
        // override lives in settings the context cannot see — so it is decided
        // once here and read from the context by everything downstream. Sized
        // from the `Ui` rather than the viewport: in a split view this app
        // owns one column of the window, and it is the column that must decide
        // whether the roomy layout still fits. For the whole-window case the
        // two are the same rect.
        let tier = crate::layout::tier_for(ui.max_rect().size(), self.ui_settings.layout);
        crate::layout::set_tier(&ctx, tier);
        // …and the two questions the tier does not answer, published the same
        // way and for the same reason — the operator's override lives in
        // settings the context cannot see. `Small screen` asked for on a tall
        // display would otherwise be measured back to "roomy" at the places
        // that draw (issue #211).
        crate::layout::set_tight(
            &ctx,
            crate::layout::tight_for(ui.max_rect().size(), self.ui_settings.layout),
        );
        crate::layout::set_short_tablet(
            &ctx,
            crate::layout::short_tablet_for(ui.max_rect().size(), self.ui_settings.layout),
        );
        if self.tier != tier {
            self.tier = tier;
            crate::theme::apply_metrics(&ctx, tier);
            // The `Ui` we were handed took its copy of the style before this,
            // so the new metrics land next frame; ask for one straight away.
            ctx.request_repaint();
        }
        // Same idea for the theme and chrome styles: the settings dialog wrote
        // them into `ui_settings` at the end of last frame, and the top of the
        // frame is the one place a visuals rewrite cannot race the settings
        // window's ScrollPalette push/pop.
        let look =
            (self.ui_settings.theme, self.ui_settings.button_style, self.ui_settings.window_style);
        if self.applied_look != look {
            self.applied_look = look;
            crate::theme::set_look(look.0, look.1, look.2);
            crate::theme::apply_visuals(&ctx);
            ctx.request_repaint();
        }
        // The panadapter and skimmer font sizes need no visuals rewrite —
        // everything they size is laid out per frame — so they are simply
        // re-stored each frame.
        crate::theme::set_font_sizes(
            self.ui_settings.skimmer_font_size,
            self.ui_settings.waterfall_font_size,
            self.ui_settings.menu_font_size,
        );
        // Same for the spot tints and the band-plan shades: the pickers in the
        // settings window write straight into `ui_settings`, and a colour has
        // to take on the frame it is chosen in or the picker feels dead.
        crate::theme::set_spot_colors(&self.ui_settings.spot_colors);
        crate::theme::set_bandplan_colors(&self.ui_settings.bandplan_colors);
        // …and whether the maps carry cities, which the four of them read
        // through one shared painter.
        crate::theme::set_map_cities(self.ui_settings.map_cities);
        // The interface scale is egui's zoom factor, which egui also lets the
        // operator drive with ctrl+plus / ctrl+minus, so it is written only
        // when the setting itself moves — see `theme::apply_zoom`. It reads
        // the size the call above just stored, so it has to stay below it.
        if self.applied_ui_font != self.ui_settings.menu_font_size {
            self.applied_ui_font = self.ui_settings.menu_font_size;
            crate::theme::apply_zoom(&ctx);
            ctx.request_repaint();
        }
        self.drain_events(&ctx, now);
        self.poll_adif_import();
        self.refresh_band_conditions(now);

        // A server that asks for a password gets the whole window until it has
        // one. Nothing below this point has anything to draw — no capabilities,
        // no state and no spectrum arrive until the sign-in is accepted — and
        // nothing below it may run either: the bindings would happily send PTT
        // to a socket that is not going to read it.
        let phase = self.ctrl.auth_phase();
        // Keyed by station, not by radio: a station challenges each of its
        // radios separately, and the operator signs in to the station.
        self.login.settle(&phase, &self.station_key());
        if phase.is_pending() {
            let rs = frame.wgpu_render_state();
            if let Some(login) = crate::login::screen(ui, &mut self.login, &phase, rs) {
                self.ctrl.send_auth(login.username, login.password);
            }
            // The socket wakes the UI when the server answers; this is only so
            // a spinner-less "CHECKING…" cannot look like a hung window.
            crate::repaint::schedule_ms(&ctx, IDLE_POLL_MS);
            return;
        }

        let mut cmds = Vec::new();
        // A channel list chosen in the memories window: parsed here and sent
        // to the engine, which owns the list and the numbering in it.
        self.poll_chirp_import(&mut cmds);
        // The keyboard, the mouse buttons and the control surface belong to
        // the focused radio alone. In a split view every visible radio runs
        // this frame loop, and without the gate one arrow key would tune all
        // of them at once.
        if self.focused {
            // F1 toggles the manual — handled here (not in
            // `keyboard_shortcuts`) so it works even while a text field has
            // focus.
            if ctx.input(|i| i.key_pressed(egui::Key::F1)) {
                self.help.open = !self.help.open;
            }
            // An open manual takes the scrolling keys before the bindings run,
            // so reading it never tunes the radio at the same time.
            self.help.grab_keys(&ctx);
            self.control_inputs(&ctx, now, &mut cmds);
        }
        // (An unfocused pane's MIDI backlog is discarded in `drain_events`,
        // which ran above, same as for the hidden tabs.)
        // Shutting down with a bound key, a footswitch or the compact layout's
        // PTT still held would otherwise leave the rig transmitting.
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.input.any_held() {
                self.release_held_controls(&mut cmds);
            }
            // Latched as well as held: the chip keeps the transmitter on
            // after the pointer has lifted, and a window closing on a latch is
            // exactly the over nobody is watching.
            if self.ptt.keying() {
                self.ptt = Default::default();
                cmds.push(Command::SetPtt(false));
            }
        }

        egui::Panel::top(crate::layout::salted_id(&ctx, "topbar"))
            .frame(
                egui::Frame::new()
                    .fill(crate::theme::BG_DEEP())
                    .inner_margin(egui::Margin::symmetric(8, 6)),
            )
            .show(ui, |ui| {
                crate::chrome::angled_frame(ui, crate::theme::PINK(), |ui| {
                    self.top_bar(ui, &mut cmds);
                });
            });
        // A persistent radio-audio warning (input unavailable / mono-for-IQ)
        // rides above the panadapter with a dismiss button, so a silent RX
        // failure is explained rather than reading as "waiting for spectrum".
        if let Some(notice) = self.radio_notice.clone() {
            let (wash, rule, mark, ink) = notice_banner_colors();
            egui::Frame::new()
                .fill(wash)
                .stroke(egui::Stroke::new(1.0, rule))
                .inner_margin(egui::Margin::symmetric(8, 5))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("⚠").size(15.0).color(mark));
                        ui.label(RichText::new(notice).size(13.0).color(ink));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // ⛔ A TRIPPED SWR GUARD IS NOT DISMISSIBLE, and the
                            // distinction is the point. Plain "Dismiss" clears
                            // the banner locally and changes nothing in the
                            // engine, which for a latch would be the worst of
                            // both worlds: the warning gone and transmit still
                            // locked out, with nothing left on screen saying
                            // why. The latch therefore gets its own button that
                            // actually clears it in the engine.
                            if let Some(swr) = self.state.tx.swr_tripped {
                                if ui
                                    .button(RichText::new("Acknowledge").size(13.0))
                                    .on_hover_text(format!(
                                        "Re-enables transmit after {swr:.1}:1. Check the antenna \
                                         first: if the fault is still there, the next \
                                         transmission stops too."
                                    ))
                                    .clicked()
                                {
                                    cmds.push(Command::ClearSwrTrip);
                                    self.radio_notice = None;
                                }
                            } else if ui.small_button("Dismiss").clicked() {
                                self.radio_notice = None;
                            }
                        });
                    });
                });
        }
        // The startup version check landing. At most one message ever comes:
        // the worker only sends a release newer than this build that was not
        // already dismissed, then hangs up — as it also does, without
        // sending, when the site is unreachable or nothing is new.
        if let Some(rx) = &self.update_fetch {
            match rx.try_recv() {
                Ok(v) => {
                    self.update_notice = Some(v);
                    self.update_fetch = None;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => self.update_fetch = None,
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
        }
        // A newer release on sdroxide.com — the radio warnings' banner in
        // the same place, but its Dismiss also remembers the version, so the
        // banner stays away until the *next* release ships.
        if let Some(version) = self.update_notice.clone() {
            let (wash, rule, mark, ink) = notice_banner_colors();
            egui::Frame::new()
                .fill(wash)
                .stroke(egui::Stroke::new(1.0, rule))
                .inner_margin(egui::Margin::symmetric(8, 5))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("⚠").size(15.0).color(mark));
                        ui.label(
                            RichText::new(format!(
                                "SDRoxide {version} has been released — this is {}.",
                                env!("CARGO_PKG_VERSION")
                            ))
                            .size(13.0)
                            .color(ink),
                        );
                        ui.hyperlink_to(
                            RichText::new("Get it at sdroxide.com").size(13.0),
                            "https://sdroxide.com/",
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("Dismiss").clicked() {
                                crate::app::persist::persist_dismissed_update(&version);
                                self.update_notice = None;
                            }
                        });
                    });
                });
        }
        // Network-spot overlay (shared by voice + digital panadapter paths). A
        // clicked spot is captured here and pre-filled into a log entry below.
        // The broadcast stations are refreshed first, before anything reads
        // them: the overlay here, the SPOTS list and the world map all do.
        self.refresh_broadcast_spots(now_unix());
        let (net_spots, net_alpha) = self.net_overlay(now_unix());
        let mut clicked_spot: Option<Spot> = None;
        // Built once for both panadapter call sites: the same devices are
        // labelled whichever layout is on screen.
        let ism_labels = self.ism_overlay();
        // Likewise the memory marks along the bottom of the waterfall.
        let mem_marks = self.memory_overlay();
        // Remaining space: the panadapter (+ FT8/FT4 operating panel).
        if let Some(err) = self.error.clone() {
            let offer_retry = self.ctrl.can_reconnect();
            // Seconds until the redial that is already coming, so the screen
            // can say what is going to happen rather than only what went wrong.
            let due_in = self.retry_at.map(|at| (at - crate::time::now_unix_f64()).max(0.0));
            let mut retry = false;
            ui.vertical_centered(|ui| {
                // Roughly where `centered_and_justified` used to put the text,
                // with the button under it rather than beside it.
                ui.add_space(ui.available_height() * 0.4);
                ui.label(RichText::new(err).size(18.0).color(crate::theme::ALERT()));
                if offer_retry {
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(match due_in {
                            Some(s) if s >= 1.0 => format!("Reconnecting in {}s…", s.ceil() as u64),
                            Some(_) => "Reconnecting…".to_string(),
                            None => "Not connected.".to_string(),
                        })
                        .size(13.0)
                        .color(crate::theme::gray(150)),
                    );
                    ui.add_space(10.0);
                    retry = ui.button(RichText::new("Reconnect now").size(16.0)).clicked();
                }
            });
            if retry {
                // Asking by hand puts the wait back to the bottom: the operator
                // has some reason to believe the far end is up, and making them
                // watch out a backoff they did not choose is not an answer.
                self.retry_backoff = super::RETRY_MIN_S;
                self.reconnect_now();
            }
        } else if self.state.rx[0].mode.has_bottom_panel() {
            // Remember the voice-mode view once, so leaving FT8 can restore it
            // instead of leaving the panadapter zoomed to the sub-band.
            if self.view.pre_digi_view.is_none() {
                self.view.pre_digi_view = Some((self.view.view_lo_hz, self.view.view_hi_hz));
            }
            let dial = self.state.rx_freq_hz();
            let mode = self.state.rx[0].mode;
            // The digital sub-band (audio 0..3.5 kHz above dial); for RF Paint,
            // tight onto the 300..3300 Hz painting band so incoming pictures
            // are large enough to read on the waterfall. RIFP straddles the
            // dial rather than sitting above it, so its window is symmetric
            // and as wide as the profile's channel.
            let (sub_lo, sub_hi) = if mode.is_ais() {
                // Both AIS channels and a little either side, not a sub-band:
                // the two are 50 kHz apart and both are being read at once, so
                // framing one of them would be a view of half the traffic.
                (dial - 60_000.0, dial + 60_000.0)
            } else if mode.is_vdl2() {
                // The whole datalink group, not a sub-band: seven channels
                // spread over 325 kHz are all being read at once, and framing
                // one of them would be a view of a sixth of what is happening.
                (dial - 250_000.0, dial + 250_000.0)
            } else if mode.is_adsb() {
                // ADS-B is not worked inside a sub-band at all: a Mode S reply
                // is megahertz wide and the decoder reads the whole receiver
                // window. Framing it like a digital mode would zoom the
                // panadapter to a few kilohertz of one pulse's spectrum, which
                // is a view of nothing. The whole channel instead, which is
                // also all there is on this band.
                (dial - 1_500_000.0, dial + 1_500_000.0)
            } else if mode.is_aprs() {
                // APRS is deliberately *not* framed on its own channel.
                //
                // Every other digital mode is worked inside a sub-band, so
                // framing that sub-band is a service. APRS is one channel on a
                // band an operator has every reason to be watching — the
                // repeater outputs above it, the simplex calling frequency
                // below, a satellite passing over the top — and ±8 kHz around
                // 144.800 would take all of that away in exchange for a view
                // of a channel whose whole content is already in the panel.
                //
                // A window rather than nothing at all, though: this is the one
                // mode that moves the dial by itself, and after a 130 MHz jump
                // a view left where it was is a panadapter showing spectrum
                // from another band entirely. 300 kHz is wide enough to hold
                // the neighbourhood and is only a starting point — zoom and
                // pan stay live, and it is re-applied only on a mode change or
                // a retune that takes the channel off-screen.
                (dial - 150_000.0, dial + 150_000.0)
            } else if mode.is_carrier_centered() {
                let half = (self.state.rx[0].filter_hi - self.state.rx[0].filter_lo).abs() as f64
                    * 0.5
                    * 1.2;
                (dial - half, dial + half)
            } else if mode.is_rf_paint() {
                (dial + 150.0, dial + 3450.0)
            } else if mode.is_lower_sideband_at(dial) {
                // Mirrored, because the mode is: SSTV and RADE keep phone
                // practice and ride the lower sideband on 160/80/40 m, so their
                // sub-band is *below* the dial there. Framed above it, the
                // opening view of the mode is the empty side of the signal.
                (dial - 3500.0, dial + 200.0)
            } else {
                (dial - 200.0, dial + 3500.0)
            };
            // The slotted modes and WSPR work a fixed, narrow allocation, so
            // their view stays locked to the sub-band. Every other digital mode
            // roams the band (SSTV, Hell, wefax…), so the sub-band is only the
            // *starting* view there — zoom and pan stay live, and the fit is
            // re-applied on a mode change or a QSY that takes the sub-band
            // off-screen. Tuning that keeps it in view (a drag-tune, a nudge)
            // keeps the operator's zoom.
            let locked = mode.is_slotted() || mode.is_wspr();
            let center = self.state.center_hz;
            let mode_changed = self.digi_view_fit.map(|(m, _, _)| m) != Some(mode);
            let dial_moved = self.digi_view_fit.map(|(_, d, _)| d) != Some(dial);
            // A retune moved the span the view has to live inside. Paired
            // with `!sub_visible` below rather than forcing a fit on its own:
            // an operator who panned the front end's window somewhere they
            // wanted to look should keep it, and only a window that no longer
            // contains the sub-band at all has to be re-fitted.
            let span_moved = self.digi_view_fit.map(|(_, _, c)| c) != Some(center);
            let sub_visible = self.view.view_lo_hz < sub_hi && self.view.view_hi_hz > sub_lo;
            if locked || mode_changed || ((dial_moved || span_moved) && !sub_visible) {
                self.view.view_lo_hz = sub_lo;
                self.view.view_hi_hz = sub_hi;
            }
            self.digi_view_fit = Some((mode, dial, center));
            // Which side of the dial the mode's audio band is on. Every tone
            // offset below is a distance from the dial and every marker is drawn
            // at dial + offset, so on the bands where the mode rides the lower
            // sideband (SSTV and RADE on 160/80/40 m) they all belong below it.
            // Display only: the controllers go on reporting an offset as a
            // width from the dial, whichever side they are being worked on.
            let side = if mode.is_lower_sideband_at(dial) { -1.0 } else { 1.0 };
            let audio_hz = side * self.digi_status.as_ref().map(|s| s.audio_hz).unwrap_or(1500.0);
            let is_text = mode.is_text_modem();
            // RTTY shows mark/space tuning lines; Olivia the tone-bank edges;
            // PSK just the centre marker.
            let markers: Vec<f32> = if mode == Mode::Rtty {
                let sh = self.digi_status.as_ref().map(|s| s.config.rtty_shift_hz).unwrap_or(170.0);
                vec![audio_hz - sh / 2.0, audio_hz + sh / 2.0]
            } else if mode == Mode::Olivia {
                let bw = self.digi_status.as_ref().map(|s| s.config.olivia_bw_hz).unwrap_or(1000.0);
                vec![audio_hz - bw / 2.0, audio_hz + bw / 2.0]
            } else if mode == Mode::Thor {
                let baud =
                    self.digi_status.as_ref().map(|s| s.config.thor_mode.baud()).unwrap_or(15.625);
                let bw = 18.0 * baud;
                vec![audio_hz - bw / 2.0, audio_hz + bw / 2.0]
            } else if mode == Mode::Js8 {
                // Worth showing: Turbo's 160 Hz footprint against Slow's 25 Hz
                // is what decides whether a frequency is actually free.
                let bw = self
                    .digi_status
                    .as_ref()
                    .and_then(|s| s.js8.as_ref())
                    .map_or(50.0, |j| j.speed.bandwidth_hz());
                vec![audio_hz, audio_hz + bw]
            } else if mode == Mode::Fsq {
                let baud = self.digi_status.as_ref().map(|s| s.config.fsq_baud).unwrap_or(4.5);
                let bw = 33.0 * baud;
                vec![audio_hz - bw / 2.0, audio_hz + bw / 2.0]
            } else if mode == Mode::Hell {
                let v =
                    self.digi_status.as_ref().map(|s| s.config.hell_variant).unwrap_or_default();
                let bw = v.bandwidth_hz() as f32;
                vec![audio_hz - bw / 2.0, audio_hz + bw / 2.0]
            } else if mode == Mode::RfPaint {
                // The painting band edges (300..3300 Hz).
                vec![300.0, 3300.0]
            } else if mode == Mode::Rade {
                // The RADE V1 OFDM carriers, so the operator can see whether the
                // signal is sitting inside the modem's window.
                vec![side * 1062.0, side * 1876.0]
            } else {
                Vec::new()
            };
            // FT8 station callsign boxes (built before the &mut self borrows).
            // Only the slotted modes have them — SSTV / RF Paint share the digi
            // path but must not inherit FT8's overlay.
            let (ft8_spots, ft8_alpha) =
                if mode.is_slotted() { self.ft8_overlay() } else { (Vec::new(), Vec::new()) };

            let frame = self.frame.take();
            // A phone shows one of the three at a time, chosen by a row of
            // chips: the decode list, the QSO area and the waterfall each want
            // the whole width and most of the height, and three slivers would
            // be three things too small to use. Everything wider keeps the
            // waterfall above the panel, split by a draggable divider.
            let phone = tier == crate::layout::Tier::Phone;
            let width = ui.available_width();
            if phone {
                self.digi_tabs(ui, mode);
            }
            // The waterfall is the tab past the last of the mode's own panes.
            let on_waterfall =
                phone && self.digi_pane(mode) == crate::app::panels::panel_panes(mode).len();
            // Both panadapter layers switched off in the SPEC popup means no
            // panadapter: the decode panel takes the whole height rather than
            // sharing it with an empty strip. Never on a phone, which draws the
            // waterfall alone and ignores the layer switches (see
            // `spectrum_view::show_ext`), so they cannot empty its screen.
            let layers = tier.waterfall_only() || self.view.panadapter_visible();
            let show_wf = (!phone || on_waterfall) && layers;
            let show_panel = !phone || !on_waterfall;

            // Manual vertical split with a draggable divider: the operating
            // panel gets `digi_panel_fraction` of the height, the waterfall the
            // rest. A thin handle between them resizes the split.
            let total = ui.available_height();
            let handle_h = if phone { 0.0 } else { 7.0 };
            let (wf_h, panel_h) = if phone {
                // Whichever one is showing gets all of it.
                (total, total)
            } else if !layers {
                (0.0, total)
            } else {
                // The handle plus the gap the layout inserts on each side of it.
                let divider = handle_h + 2.0 * ui.spacing().item_spacing.y;
                digi_split(
                    total,
                    divider,
                    self.view.digi_panel_fraction,
                    ui.spacing().interact_size.y,
                )
            };

            // Scroll only while frames are actually arriving. The last frame is
            // kept for the spectrum line, but rows of it repeated down the
            // waterfall would be time that never happened — a switched-off
            // radio (or any stalled stream) freezes instead.
            let live = frame.is_some() && now - self.last_spectrum_at < STREAM_STALE_S;
            self.note_panadapter_width(ui);
            let wf_tuning = self.wf_tick(live, ui.ctx().pixels_per_point());
            if show_wf {
                ui.allocate_ui(egui::vec2(width, wf_h), |ui| {
                    let pan =
                        spectrum_view::WindowPan::of(self.caps.as_ref(), self.state.center_hz)
                            .with_outer(Some(self.zoom_out_window()), self.state.sample_rate);
                    spectrum_view::show_ext(
                        ui,
                        &mut self.view,
                        &mut self.state,
                        frame.as_ref(),
                        &mut self.peaks,
                        &mut self.spec_smooth,
                        &mut self.trace_cache,
                        &mut self.spec3d,
                        // A click sets the tone offset in the modes that park
                        // on an agreed dial and pick a signal inside the
                        // sub-band. Not in RTTY: its tone pair is a standard
                        // (2125/2295 Hz), stations are spread across the band
                        // rather than stacked in one window, and dragging mark
                        // and space onto each one in turn walks them out of the
                        // transmit filter. There a click tunes the dial so the
                        // signal lands on the pair, exactly as CW does.
                        Some(spectrum_view::AudioCursor {
                            hz: audio_hz,
                            click_sets_offset: !mode.holds_standard_tones(),
                            // CW only for now: RTTY and WEFAX sit off their
                            // dials too and could follow, but each wants
                            // checking against real signals first.
                            line_on_cursor: false,
                        }),
                        if matches!(mode, Mode::Ft8 | Mode::Ft2) {
                            self.digi_status
                                .as_ref()
                                .map(|s| s.config.dxped_mode)
                                .unwrap_or_default()
                        } else {
                            sdroxide_types::DxpedMode::Normal
                        },
                        mode.is_slotted()
                            && self
                                .digi_status
                                .as_ref()
                                .map(|s| s.config.auto_tx_freq)
                                .unwrap_or(true),
                        mode.is_slotted()
                            && self
                                .digi_status
                                .as_ref()
                                .map(|s| s.config.hold_tx_freq)
                                .unwrap_or(false),
                        &markers,
                        &ft8_spots,
                        &ft8_alpha,
                        &net_spots,
                        &net_alpha,
                        &mut clicked_spot,
                        &ism_labels,
                        &mem_marks,
                        self.input.cfg.wheel,
                        pan,
                        wf_tuning,
                        show_panel,
                        &mut cmds,
                    );
                });
            }
            // Only between two things: with the panadapter switched off there
            // is nothing above the panel for the handle to divide, and a drag
            // on it would silently rewrite a split with nothing to show for it.
            if !phone && show_wf {
                // Resize handle between the waterfall and the FT8/FT4 panel.
                let hresp = crate::chrome::split_handle(
                    ui,
                    egui::vec2(width, handle_h),
                    Some(crate::theme::PANEL()),
                );
                if hresp.dragged() {
                    // Drag down shrinks the panel (waterfall grows), drag up grows it.
                    let d = hresp.drag_delta().y / total;
                    self.view.digi_panel_fraction =
                        (self.view.digi_panel_fraction - d).clamp(0.2, 0.82);
                }
            }
            if show_panel {
                ui.allocate_ui(egui::vec2(width, panel_h), |ui| {
                    // The operating panels, pulled in on a small screen — one
                    // call for all of them, see `layout::tighten`.
                    crate::layout::tighten(ui);
                    egui::Frame::new()
                        .fill(crate::theme::BG_DEEP())
                        .inner_margin(egui::Margin { left: 0, right: 0, top: 6, bottom: 0 })
                        .show(ui, |ui| {
                            crate::chrome::angled_frame(ui, crate::theme::PINK(), |ui| {
                                if mode.is_rade() {
                                    self.rade_panel(ui, &mut cmds, panel_h);
                                } else if mode.is_wefax() {
                                    self.wefax_panel(ui, &mut cmds, panel_h);
                                } else if mode == Mode::Navtex {
                                    self.navtex_panel(ui, &mut cmds, panel_h);
                                } else if mode.is_image() {
                                    self.image_panel(ui, &mut cmds, mode);
                                } else if mode.is_rf_paint() {
                                    self.rf_paint_panel(ui, &mut cmds, panel_h);
                                } else if mode.is_adsb() {
                                    self.adsb_panel(ui, &mut cmds, panel_h);
                                } else if mode.is_vdl2() {
                                    self.vdl2_panel(ui, &mut cmds, panel_h);
                                } else if mode.is_ais() {
                                    self.ais_panel(ui, &mut cmds, panel_h);
                                } else if mode.is_aprs() {
                                    self.aprs_panel(ui, &mut cmds, panel_h);
                                } else if mode.is_packet() {
                                    self.packet_panel(ui, &mut cmds, panel_h);
                                } else if mode.is_fsq() {
                                    self.fsq_panel(ui, &mut cmds, panel_h);
                                } else if mode.is_hell() {
                                    self.hell_panel(ui, &mut cmds, panel_h);
                                } else if is_text {
                                    self.text_modem_panel(ui, &mut cmds, panel_h);
                                } else if mode.is_js8() {
                                    self.js8_panel(ui, &mut cmds, panel_h);
                                } else if mode.is_wspr() {
                                    self.wspr_panel(ui, &mut cmds, panel_h);
                                } else {
                                    self.digi_panel(ui, &mut cmds);
                                }
                            });
                        });
                });
            }
            self.frame = frame;
        } else {
            // Restore the pre-FT8 view span once, on the first voice frame
            // after leaving a digital mode.
            if let Some((lo, hi)) = self.view.pre_digi_view.take() {
                self.view.view_lo_hz = lo;
                self.view.view_hi_hz = hi;
            }
            // Forget the digital fit: coming back to the same mode and dial
            // must fit the sub-band afresh, not keep this voice-mode view.
            self.digi_view_fit = None;
            // The full-band strip, above the panadapter and only when a front
            // end actually supplies one — and the operator has left the Display
            // module's WIDE chip on. Never on a phone: 96 pt of strip is a
            // quarter of the height there, taken from the waterfall being
            // listened to for a band view nothing is tuned to.
            if !tier.waterfall_only()
                && self.view.wide_waterfall
                && let Some(wide) = self.wide_frame.clone()
            {
                crate::widgets::wide_spectrum::show(
                    ui,
                    &mut self.wide_wf,
                    &wide,
                    &self.state,
                    self.ui_settings.waterfall_palette,
                    &mut cmds,
                );
                ui.add_space(2.0);
            }
            let (cw_spots, cw_alpha) = self.cw_overlay(now);
            let frame = self.frame.take();
            // As on the digital path: no fresh frames, no scroll.
            let live = frame.is_some() && now - self.last_spectrum_at < STREAM_STALE_S;
            self.note_panadapter_width(ui);
            let wf_tuning = self.wf_tick(live, ui.ctx().pixels_per_point());
            // CW is the one analog mode with a panel under the panadapter. It
            // is not a digital mode and does not take the digital path — the
            // demodulated tone stays audible and the view stays wherever the
            // operator left it — but it has a decoder and a keyboard, and both
            // need somewhere to live. The cursor is the mode's own: a marker at
            // the pitch being copied, which a click on the waterfall moves.
            let cw_mode = self.state.rx[0].mode == Mode::Cw;
            let phone = tier == crate::layout::Tier::Phone;
            // As on the digital path above: no layers, no panadapter. In a mode
            // with nothing under it that leaves the pane empty, which is what
            // switching both off asks for.
            let layers = tier.waterfall_only() || self.view.panadapter_visible();
            let (wf_h, panel_h, show_wf, show_panel) = if !cw_mode {
                (ui.available_height(), 0.0, layers, false)
            } else if phone {
                self.digi_tabs(ui, Mode::Cw);
                let on_wf =
                    self.digi_pane(Mode::Cw) == crate::app::panels::panel_panes(Mode::Cw).len();
                let h = ui.available_height();
                (h, h, on_wf, !on_wf)
            } else if !layers {
                (0.0, ui.available_height(), false, true)
            } else {
                let total = ui.available_height();
                let divider = 7.0 + 2.0 * ui.spacing().item_spacing.y;
                let (w, p) = digi_split(
                    total,
                    divider,
                    self.view.digi_panel_fraction,
                    ui.spacing().interact_size.y,
                );
                (w, p, true, true)
            };
            let width = ui.available_width();
            let cw_pitch = cw_mode.then(|| spectrum_view::AudioCursor {
                hz: self.cw_pitch_hz(),
                // A click tunes the dial so the signal lands on the cursor.
                click_sets_offset: false,
                // With the readout reading the signal, the tuning line follows
                // it there — see `UiSettings::cw_qrg`.
                line_on_cursor: self.ui_settings.cw_qrg,
            });
            if show_wf {
                ui.allocate_ui(egui::vec2(width, wf_h), |ui| {
                    let pan =
                        spectrum_view::WindowPan::of(self.caps.as_ref(), self.state.center_hz)
                            .with_outer(Some(self.zoom_out_window()), self.state.sample_rate);
                    spectrum_view::show_ext(
                        ui,
                        &mut self.view,
                        &mut self.state,
                        frame.as_ref(),
                        &mut self.peaks,
                        &mut self.spec_smooth,
                        &mut self.trace_cache,
                        &mut self.spec3d,
                        cw_pitch,
                        sdroxide_types::DxpedMode::Normal,
                        false,
                        // This waterfall is drawn for the non-slotted modes, so
                        // there is no held FT8 transmit tone for a click to
                        // disturb. The engine's own gate is the authority in any
                        // case; this only decides whether the UI bothers asking.
                        false,
                        &[],
                        &cw_spots,
                        &cw_alpha,
                        &net_spots,
                        &net_alpha,
                        &mut clicked_spot,
                        &ism_labels,
                        &mem_marks,
                        self.input.cfg.wheel,
                        pan,
                        wf_tuning,
                        show_panel,
                        &mut cmds,
                    );
                });
            }
            if show_panel {
                if !phone && show_wf {
                    let hresp = crate::chrome::split_handle(
                        ui,
                        egui::vec2(width, 7.0),
                        Some(crate::theme::PANEL()),
                    );
                    if hresp.dragged() {
                        let d = hresp.drag_delta().y / ui.available_height().max(1.0);
                        self.view.digi_panel_fraction =
                            (self.view.digi_panel_fraction - d).clamp(0.2, 0.82);
                    }
                }
                ui.allocate_ui(egui::vec2(width, panel_h), |ui| {
                    // The operating panels, pulled in on a small screen — one
                    // call for all of them, see `layout::tighten`.
                    crate::layout::tighten(ui);
                    egui::Frame::new()
                        .fill(crate::theme::BG_DEEP())
                        .inner_margin(egui::Margin { left: 0, right: 0, top: 6, bottom: 0 })
                        .show(ui, |ui| {
                            crate::chrome::angled_frame(ui, crate::theme::PINK(), |ui| {
                                self.cw_panel(ui, &mut cmds, panel_h);
                            });
                        });
                });
            }
            self.frame = frame;
        }
        // A spot clicked on the panadapter: pre-fill a log entry (tuning + mode
        // were already issued inside the widget).
        if let Some(spot) = clicked_spot {
            self.prefill_from_spot(&spot);
        }

        self.memories_window(&ctx, &mut cmds);
        self.scanner_window(&ctx, &mut cmds);
        self.ism_window(&ctx, &mut cmds);
        self.adsb_setup_window(&ctx, &mut cmds);
        self.ais_setup_window(&ctx, &mut cmds);
        self.vdl2_setup_window(&ctx, &mut cmds);
        self.rds_window(&ctx);
        self.drm_window(&ctx, &mut cmds);
        self.voice_window(&ctx, &mut cmds);
        self.settings_window(&ctx, &mut cmds);
        self.digi_settings_window(&ctx, &mut cmds);
        self.logbook_window(&ctx, &mut cmds);
        self.mail_window(&ctx, &mut cmds);
        self.mail_log_window(&ctx);
        self.spots_window(&ctx, &mut cmds);
        self.public_sdrs_window(&ctx, &mut cmds);
        self.awards_window(&ctx);
        self.bands_window(&ctx);
        self.sat_window(&ctx, &mut cmds);
        self.help.ui(&ctx);
        // Last, so it lands on top of everything else that opened this frame.
        self.oob_tx_window(&ctx);
        #[cfg(not(target_arch = "wasm32"))]
        {
            let grid = self.my_grid();
            let traffic = self.digi_traffic(ctx.input(|i| i.time));
            // Only while the window is open: walking the whole logbook is not
            // free, and the closed window has nothing to paint it on.
            let awards = if self.solar.open { self.award_heat() } else { Default::default() };
            // Same reasoning: the field is aged when it is read, and a closed
            // window has nothing to age it for. The evidence itself keeps
            // accumulating either way — it is fed from the panels, not here.
            let prop = if self.solar.open {
                self.prop.field(crate::time::now_unix())
            } else {
                Default::default()
            };
            let lock_change = self.solar.viewport(
                &ctx,
                &grid,
                traffic,
                awards,
                prop,
                std::sync::Arc::clone(&self.sat_cfg),
                self.sat_track.as_ref().map(|t| t.norad_id),
            );
            self.view.solar3d = self.solar.persisted();
            // The pass window's LOCK button lands here: the 3D window has no
            // command path of its own, so the request is drained and acted on
            // where one exists.
            match lock_change {
                Some(crate::solar3d::LockChange::Lock(id)) => {
                    self.sat_lock_request(id, &mut cmds);
                }
                Some(crate::solar3d::LockChange::Unlock) => {
                    cmds.push(Command::SetSatLock(None));
                    self.sat_win.sent = None;
                }
                None => {}
            }
        }

        // Debounced spectrum-config updates with pan hysteresis.
        let now = ctx.input(|i| i.time);
        // Ahead of them, so a fit picked from this frame's spectrum rides out
        // with the config update it just changed rather than waiting for the
        // next frame. The panadapter has drawn by now, so any pan, zoom or
        // retune this frame is already in `view` for the tick to notice.
        self.auto_fit_tick(now);
        if !self.cfg_still_good() {
            let ideal = self.desired_spectrum_cfg();
            if self.desired_cfg != Some(ideal) {
                self.desired_cfg = Some(ideal);
                self.desired_at = now;
            }
            if self.sent_cfg.is_none() || now - self.desired_at >= CFG_DEBOUNCE_S {
                self.sent_cfg = Some(ideal);
                cmds.push(Command::SetSpectrumCfg(ideal));
            }
        }

        // The skimmers decode only what is on screen, and on a front end wider
        // than their window it is also what decides which slice of the band they
        // read at all — so they need the real visible span, not
        // `SpectrumConfig::viewport`, which is padded so that panning doesn't
        // clear the waterfall. Debounced on the same timer: a drag would
        // otherwise re-cut the tracked set every frame, and every re-cut throws
        // away decoders that were part-way through a callsign.
        if self.state.skimmer.any_enabled() && !self.view.is_unset() {
            let want = (self.view.view_lo_hz, self.view.view_hi_hz);
            let tol = (want.1 - want.0).abs() * 0.01;
            let moved = self
                .sent_skim_view
                .is_none_or(|(lo, hi)| (lo - want.0).abs() > tol || (hi - want.1).abs() > tol);
            if moved {
                if self.desired_skim_view != Some(want) {
                    self.desired_skim_view = Some(want);
                    self.desired_skim_view_at = now;
                }
                if now - self.desired_skim_view_at >= CFG_DEBOUNCE_S {
                    self.sent_skim_view = Some(want);
                    cmds.push(Command::SetSkimmerView(Some(want)));
                }
            }
        }

        // Flush queued lookups / uploads accumulated during window rendering.
        for call in std::mem::take(&mut self.pending_lookups) {
            cmds.push(Command::LookupCallsign { call });
        }
        for (qso_id, adif, targets) in std::mem::take(&mut self.pending_uploads) {
            cmds.push(Command::UploadQso { qso_id, adif, targets });
        }

        // Run the announcer's timers after the drain, so anything a command
        // just caused is diffed on the snapshot the engine sends back rather
        // than on the optimistic local copy.
        let speech_deadline = self.speech.announcer.tick(&self.state, self.meters.as_ref(), now);
        // Attenuate the receiver while an announcement plays. Only when this
        // client owns the engine: a remote listener taps the same mixer, and
        // ducking their audio for speech they cannot hear would be rude.
        if !self.ctrl.engine_is_remote()
            && let Some(gain) = self.speech.announcer.take_duck_change()
        {
            self.ctrl.send(Command::SetAudioDuck(gain));
        }

        for c in cmds {
            // Marked here rather than at the button, because the settings panel
            // holds borrows of `self` while it draws and cannot take a mutable
            // one. This is the point where the command is definitely going out,
            // which is the honest moment to call the check "in flight" anyway.
            if let Command::TestLogin(t) = &c {
                self.login_tests_pending.insert(*t);
                self.login_tests.remove(t);
            }
            self.ctrl.send(c);
        }

        // Data-driven repaint: wake at the next expected spectrum frame while
        // something is streaming, and idle-poll when nothing is. User input
        // wakes eframe by itself, so interactivity is unaffected.
        //
        // Data already waiting — it arrived while this frame was being built,
        // and is checked after the drain — counts as streaming, so a receiver
        // that is delivering keeps the frame cadence rather than dropping to
        // the idle poll. It does *not* earn a frame early: this used to ask for
        // an unpaced `request_repaint` instead, and because a real radio has
        // something waiting at the end of almost every frame, that branch was
        // the one always taken and the frame-rate setting did nothing at all.
        // A synthetic source never showed it — it has nothing to say between
        // frames — which is exactly why it stood for so long.
        let fps = self
            .sent_cfg
            .or(self.desired_cfg)
            .map(|c| c.fps)
            .unwrap_or(SpectrumConfig::default().fps)
            .max(1) as u64;
        let streaming = self.ctrl.wants_repaint_soon()
            || (self.frame.is_some()
                && self.error.is_none()
                && now - self.last_spectrum_at < STREAM_STALE_S);
        // Floor division keeps the poll period <= the stream period, so no
        // frame is ever skipped (the spectrum buffer is latest-wins).
        let mut wait_ms = if streaming { 1000 / fps } else { IDLE_POLL_MS };
        // A settle timer only fires when a frame runs. Idling at a quarter
        // of a second would smear a 600 ms frequency settle across most of
        // a second, so wake for it instead.
        if let Some(at) = speech_deadline {
            let ms = ((at - now).max(0.0) * 1000.0).ceil() as u64;
            wait_ms = wait_ms.min(ms.max(1));
        }
        crate::repaint::schedule_ms(&ctx, wait_ms);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, &self.view_key, &self.view);
        // The station-wide keys are written by exactly one tab — every tab
        // holds its own copy of these, and the last to save would otherwise
        // overwrite the tab the operator actually changed them on.
        if self.station_writer {
            // On wasm this is the logbook's persistence; on native it's a
            // harmless backup (the authoritative copy is written to the config
            // dir on change).
            eframe::set_value(storage, "qso_log", &self.qso_log);
            // Same split: authoritative on native is config.toml (written on
            // change).
            eframe::set_value(storage, "ui_settings", &self.ui_settings);
            // Control-input bindings: authoritative on native is input.json.
            eframe::set_value(storage, "input", &self.input.cfg);
        }
    }
}

impl SdroxideApp {
    /// Drain everything this radio's engine sent since the last visit.
    ///
    /// Runs at the top of every drawn frame — and, in a multi-radio session,
    /// once per frame for every *hidden* tab too (from
    /// [`crate::multi::MultiApp`]), so a background radio keeps decoding,
    /// logging and reconnecting while another tab is up front. The unbounded
    /// event channel must never be left to back up.
    pub(crate) fn drain_events(&mut self, ctx: &egui::Context, now: f64) {
        // Answers to the settings dialog's device questions. Drained here
        // rather than in the dialog: they come from another machine, so one can
        // land in the frame after it was closed, and an answer left in the
        // queue would be applied to whatever the dialog was asking next time.
        while let Some(answer) = self.ctrl.poll_probe() {
            self.apply_probe_answer(ctx, answer);
        }
        // Whether a panadapter frame has already landed in this pass, so the
        // next one knows it is superseding a picture nothing will ever draw —
        // see the `Spectrum` arm below.
        let mut superseded = false;
        while let Some(ev) = self.ctrl.poll_event() {
            match ev {
                RadioEvent::Capabilities(c) => {
                    // The window title belongs to the focused tab; a hidden
                    // radio reconnecting must not retitle the window over it.
                    if self.focused {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
                            "sdroxide — {}",
                            c.label
                        )));
                    }
                    // A different interface on this radio: the stored zoom
                    // was taken in another span and means nothing in this one.
                    if self.view.adopt_driver(&c.driver) {
                        tracing::debug!(
                            target: "sdroxide::panadapter",
                            driver = %c.driver,
                            "new front end — refitting the panadapter window"
                        );
                    }
                    self.caps = Some(c);
                    // A session the far end accepted: whatever the link was
                    // doing, it is doing it again, so the next drop starts its
                    // backoff from the bottom rather than from wherever the
                    // last outage left it.
                    self.retry_backoff = super::RETRY_MIN_S;
                    // A new source may have no full-band lane at all. Without
                    // this the strip from the previous backend stays on screen
                    // for good, showing a band nothing is receiving any more.
                    self.wide_frame = None;
                    self.wide_wf.clear();
                    // This also fires on a reconnect, and pictures may have
                    // come in while the link was down. Read the stores again
                    // rather than leaving a gallery that stops at the moment
                    // the connection dropped.
                    self.sstv.listed = false;
                    self.wefax.listed = false;
                    // Same reasoning for speech: a reconnect must not make the
                    // radio recite its whole configuration.
                    self.speech.announcer.reseed();
                    self.speech.announcer.reset_decodes();
                }
                // The *same* front end, revising what it said about itself: an
                // antenna list learned from a rig that has only now answered,
                // a gain ladder as long as the band the dial has just crossed
                // into. Only the capabilities move — everything the arm above
                // throws away belongs to this radio and is still current, and
                // throwing it away on a QSY would blank the wideband strip and
                // re-read every image store for nothing.
                RadioEvent::CapabilitiesUpdated(c) => self.caps = Some(c),
                RadioEvent::State(s) => {
                    let prev_vfo = self.state.active_freq_hz();
                    let prev_rate = self.state.sample_rate;
                    let prev_mode = self.state.rx[0].mode;
                    let prev_band = self.state.band;
                    self.state = s;
                    if self.state.rx[0].mode != prev_mode {
                        self.clear_digi_rx();
                        // The outgoing buffer too, and only here: the engine
                        // rebuilds its controller for the new mode and its
                        // sent-character count restarts from zero, so text left
                        // over from the last mode would be redrawn as unsent and
                        // keyed again the moment transmit came on. A QSY rebuilds
                        // nothing, and takes the half-typed over with it.
                        self.text_tx.clear();
                    } else if qsy_clears_decodes(prev_band, self.state.band, self.state.rx[0].mode)
                    {
                        self.clear_digi_band_rx();
                    }
                    self.recenter_if_tuned_away(prev_vfo, prev_rate);
                    // The announcer diffs *this* — the engine's own snapshot —
                    // rather than `self.state` per frame, because the UI
                    // mutates that optimistically before the engine confirms
                    // and the engine has the last word on where the radio
                    // actually went. Focused tab only, here and below: two
                    // radios narrating over each other is worse than silence,
                    // and the reseed on focus gain skips the backlog.
                    if self.focused {
                        self.speech.announcer.on_state(&self.state, now);
                    }
                }
                RadioEvent::Spectrum(mut f) => {
                    // This runs once per repaint and only the frame it leaves
                    // here is drawn, so anything it replaces mid-pass is a
                    // picture nobody sees. Its *rows* are another matter: the
                    // waterfall's time axis is spaced on the assumption that
                    // every row the engine clocked reaches the texture, so
                    // dropping them makes the timestamps outrun the picture by
                    // exactly the time thrown away, and go on doing it. They
                    // ride on with the frame that superseded them instead.
                    //
                    // A client redrawing more slowly than it asked the engine
                    // to publish is the ordinary case for this, and the browser
                    // client is where it shows: its socket delivers in bursts
                    // between animation frames, so several frames routinely
                    // arrive in one pass.
                    if superseded && let Some(prev) = self.frame.as_deref() {
                        f.carry_rows_from(prev);
                    }
                    superseded = true;
                    self.frame = Some(std::sync::Arc::new(f));
                    self.last_spectrum_at = now;
                }
                // Deliberately does not touch `last_spectrum_at`: that drives
                // the "waiting for spectrum" notice, and a full-band frame is
                // not evidence that the tuned receiver is producing samples.
                RadioEvent::WideSpectrum(f) => {
                    self.wide_frame = Some(std::sync::Arc::new(f));
                }
                RadioEvent::Meters(m) => {
                    if self.focused {
                        self.speech.announcer.on_meters(&m, &self.state, now);
                    }
                    self.meters = Some(m);
                }
                RadioEvent::Memories(m) => self.memories = m,
                RadioEvent::MemoryFolders(f) => self.mem_folders = f,
                RadioEvent::Scanner(c) => self.scanner = c,
                RadioEvent::ConnectionLost(e) => {
                    if self.focused {
                        self.speech.announcer.on_error(&e, now);
                    }
                    self.error = Some(e);
                    self.arm_retry(now);
                }
                RadioEvent::LoginTest(r) => {
                    self.login_tests_pending.remove(&r.target);
                    self.push_net_log(format!(
                        "{}: {}",
                        r.target.label(),
                        if r.ok { format!("ok, {}", r.message) } else { r.message.clone() }
                    ));
                    self.login_tests.insert(r.target, r);
                }
                RadioEvent::Notice(n) => {
                    if self.focused {
                        self.speech.announcer.on_notice(n.as_deref(), now);
                    }
                    self.radio_notice = n;
                }
                RadioEvent::Ft8Decodes(d) => {
                    if let Some(st) = self.digi_status.as_ref()
                        && self.focused
                    {
                        self.speech.announcer.on_ft8(&d, st, now);
                    }
                    // Prepend newest-slot decodes; keep a rolling window.
                    for dec in d.into_iter().rev() {
                        self.digi_decodes.insert(0, dec);
                    }
                    self.digi_decodes.truncate(200);
                }
                RadioEvent::WsprSpots(s) => {
                    // Newest first, and de-duplicated against what is already
                    // held: the same reception arrives twice whenever a slot we
                    // decoded ourselves also comes back from WSPRnet, and two
                    // rows for one beacon reads as the mode double-counting.
                    //
                    // Through a set rather than a scan per arrival: the list
                    // holds a night of receptions now, and a linear search
                    // through it for every spot in a WSPRnet download is the
                    // one shape of this that grows with the square of a busy
                    // band (issue #316).
                    let mut held: std::collections::HashSet<_> =
                        self.wspr_spots.iter().map(|e| e.dedup_key()).collect();
                    for spot in s.into_iter().rev() {
                        if !held.insert(spot.dedup_key()) {
                            continue;
                        }
                        self.wspr_spots.insert(0, spot);
                    }
                    self.wspr_spots.truncate(crate::app::panels::wspr::WSPR_SPOT_ROWS);
                }
                RadioEvent::Ft8Status(s) => {
                    // Seed the editable config from the engine's persisted
                    // value once (later edits are UI-owned so typing sticks).
                    if !self.digi_cfg_seeded {
                        self.digi_cfg_edit = s.config.clone();
                        self.digi_cfg_seeded = true;
                    }
                    if self.focused {
                        self.speech.announcer.on_digi(&s, &self.state, now);
                    }
                    self.digi_status = Some(s);
                }
                RadioEvent::Ft8QsoLogged(mut r) => {
                    // Another tab may have logged since this copy was read;
                    // appending to a stale copy would write its QSOs away.
                    if self.shared_log {
                        self.qso_log = crate::app::persist::load_qso_log(None);
                    }
                    r.id = self.next_log_id();
                    let call = r.call.clone();
                    let adif = auto_upload_adif(&self.net_cfg_edit, &r);
                    self.qso_log.push(r);
                    self.session_qsos += 1;
                    persist_qso_log(&self.qso_log);
                    // Enrich + optionally upload the freshly logged QSO.
                    self.queue_lookup(call);
                    if let Some((qso_id, adif, targets)) = adif {
                        self.pending_uploads.push((qso_id, adif, targets));
                    }
                }
                RadioEvent::SstvLine { image_id, y, rgb } => {
                    self.sstv.on_line(image_id, y, &rgb, &ctx);
                }
                RadioEvent::SstvImage { png, .. } => self.sstv.on_image(&png, &ctx),
                RadioEvent::DigiImage { png } => {
                    if let Some((rgb, w, h)) = crate::sstv::decode_image(&png) {
                        let ci = crate::sstv::color_image(&rgb, w, h);
                        let tex = ctx.load_texture("fsq_rx", ci, egui::TextureOptions::LINEAR);
                        self.fsq_rx_images.insert(0, tex);
                        self.fsq_rx_images.truncate(30);
                    }
                }
                RadioEvent::HellColumns { seq, rows, cols } => {
                    self.hell.on_columns(seq, rows, &cols, &self.view.hell, &ctx);
                }
                RadioEvent::WefaxLine { image_id, y, gray } => {
                    self.wefax.push_line(image_id, y, &gray);
                }
                RadioEvent::WefaxImage { png, .. } => {
                    // The chart is held rather than filed: the engine names the
                    // file it wrote and announces it a moment later, and that
                    // name is the chart's whole metadata. Guessing it here — as
                    // this once did, off a second clock — would sooner or later
                    // label a chart a second away from the file it is.
                    self.wefax.hold_fresh(&ctx, &png);
                    self.wefax.clear_live();
                }
                RadioEvent::WefaxStatus(s) => self.wefax.status = s,
                RadioEvent::Rds(d) => self.on_rds(d),
                RadioEvent::Drm(d) => self.on_drm(d),
                // A whole-table snapshot, so it replaces rather than merges: the
                // engine's table is authoritative and already carries the history
                // — first heard, times heard — that a merge here would be
                // reconstructing badly.
                RadioEvent::IsmReports(r) => self.ism_reports = r,
                RadioEvent::IsmStatus(st) => self.ism_status = Some(st),
                RadioEvent::AdsbStatus(st) => self.adsb_status = Some(st),
                RadioEvent::Vdl2Status(st) => self.vdl2_status = Some(st),
                RadioEvent::AisStatus(st) => self.ais_status = Some(st),
                RadioEvent::Qo100Status(st) => self.qo100_status = Some(st),
                RadioEvent::SstvStatus(s) => {
                    // Adopt a *newly* detected RX mode for the next transmit, but
                    // don't re-apply a steady detection every frame — that would
                    // fight the operator's manual mode selection.
                    if s.detected != self.sstv.last_detected {
                        if let Some(m) = s.detected {
                            self.sstv.tx_mode = m;
                            self.sstv.preview_dirty = true;
                        }
                        self.sstv.last_detected = s.detected;
                    }
                    self.sstv.status = s;
                }
                RadioEvent::RifpRows { image_id, y, w, h, rows } => {
                    self.sstv.on_rifp_rows(image_id, y, w, h, &rows, &ctx);
                }
                RadioEvent::RifpImage { png, .. } => self.sstv.on_rifp_image(&png, &ctx),
                RadioEvent::RifpStatus(s) => {
                    self.sstv.rifp = s;
                }
                RadioEvent::SkimmerSpots(s) => {
                    // The engine sends the full current set each update; the
                    // stable `id` per spot lets the overlay keep each box (and
                    // its scroll) in place across updates.
                    for spot in &s {
                        // Remember when each spot last keyed, and seed newly
                        // seen ones to now, so alpha starts solid and fades.
                        let e = self.skimmer_active_at.entry(spot.id).or_insert(now);
                        if spot.active {
                            *e = now;
                        }
                    }
                    // Forget timings for spots the engine has dropped.
                    let live: std::collections::HashSet<u64> = s.iter().map(|x| x.id).collect();
                    self.skimmer_active_at.retain(|id, _| live.contains(id));
                    self.skimmer_spots = s;
                }
                RadioEvent::Spots(s) => self.spots = s,
                RadioEvent::NetStatus(s) => self.net_status = s,
                RadioEvent::TciServerStatus { running, addr, clients, error } => {
                    self.tci_srv_status = Some(TciServerStatus { running, addr, clients, error });
                }
                RadioEvent::RigctldStatus { running, addr, clients, error } => {
                    self.rigctld_status = Some(TciServerStatus { running, addr, clients, error });
                }
                RadioEvent::VoiceStatus(v) => self.voice = v,
                RadioEvent::ImagePresets(p) => self.sstv.on_presets(p, &ctx),
                RadioEvent::ImageSlotSource { slot, version, png } => {
                    self.sstv.on_slot_source(slot, version, &png);
                }
                // One store per mode: SSTV and RIFP share a gallery, charts
                // have their own.
                RadioEvent::ImageListing(l) => match l.kind {
                    sdroxide_types::ImageKind::Sstv => self.sstv.on_listing(l, &ctx),
                    sdroxide_types::ImageKind::Wefax => self.wefax.on_listing(l, &ctx),
                },
                RadioEvent::ImageFile { kind, name, png } => match kind {
                    sdroxide_types::ImageKind::Sstv => self.sstv.on_file(&name, &png, &ctx),
                    sdroxide_types::ImageKind::Wefax => self.wefax.on_file(&name, &png, &ctx),
                },
                // What the station is set up to do, from the machine the engine
                // runs on. Seeded once, like the digi config above: later edits
                // are the dialog's, so typing sticks, and the engine echoes
                // every applied change back here anyway.
                RadioEvent::StationConfig(c) => {
                    if !self.net_cfg_seeded {
                        self.net_cluster_cmds = c.net.cluster.commands.join("\n");
                        self.net_rbn_cmds = c.net.rbn.commands.join("\n");
                        self.net_cfg_edit = c.net.clone();
                        self.net_cfg_seeded = true;
                    }
                    if !self.rigctld_seeded {
                        self.rigctld_edit = c.rigctld.clone();
                        self.rigctld_seeded = true;
                    }
                    if !self.tci_srv_seeded {
                        self.tci_srv_edit = c.tci_server.clone();
                        self.tci_srv_seeded = true;
                    }
                    if !self.wsjtx_seeded {
                        self.wsjtx_edit = c.wsjtx.clone();
                        self.wsjtx_seeded = true;
                    }
                    if !self.sat_cfg_seeded {
                        self.sat_cfg_edit = c.sat.clone();
                        self.sat_cfg = std::sync::Arc::new(c.sat.clone());
                        self.sat_cfg_seeded = true;
                    }
                    if !self.rot_cfg_seeded {
                        self.rot_cfg_edit = c.rotator.clone();
                        self.rot_cfg_seeded = true;
                    }
                    if !self.relay_seeded {
                        self.relay_edit = c.relay.clone();
                        self.relay_seeded = true;
                    }
                    // Adopted on every announcement rather than seeded once:
                    // these are not dialog buffers the operator types into but
                    // the band plan the whole client draws with, and the
                    // station is its only authority. A remote client that kept
                    // its own would show European band edges for an American
                    // radio.
                    self.region_edit = c.region;
                    sdroxide_types::set_region(c.region);
                    // Only when it actually changed: installing leaks the
                    // previous plan, and this bundle arrives on every
                    // station-config edit — a password change must not cost an
                    // allocation that is never freed.
                    if sdroxide_types::band_plan() != &c.band_plan {
                        sdroxide_types::set_band_plan(c.band_plan.clone());
                    }
                    // The operator's own additions to the modes' frequency
                    // tables, on the same terms and for the same reason: the
                    // station owns the list, and this client draws the picker
                    // from it. Guarded against the same leak.
                    if sdroxide_types::digi_presets() != c.digi_presets.as_slice() {
                        sdroxide_types::set_digi_presets(c.digi_presets.clone());
                    }
                }
                RadioEvent::TleSubStatus(s) => self.on_tle_sub_status(s),
                RadioEvent::SatTrack(t) => self.sat_track = t.map(|t| *t),
                RadioEvent::RotatorStatus { connected, az_deg, el_deg, error } => {
                    self.rotator_status = Some((connected, az_deg, el_deg, error));
                }
                RadioEvent::RelayStatus(st) => self.relay_status = *st,
                RadioEvent::ImageSaved(e) => match e.kind {
                    sdroxide_types::ImageKind::Sstv => self.sstv.on_saved(e, &ctx),
                    sdroxide_types::ImageKind::Wefax => self.wefax.on_saved(e, &ctx),
                },
                // Broadcast, so this is as likely to be another screen's delete
                // as our own — either way the picture has gone.
                RadioEvent::ImageDeleted { kind, name } => match kind {
                    sdroxide_types::ImageKind::Sstv => self.sstv.on_deleted(&name),
                    sdroxide_types::ImageKind::Wefax => self.wefax.on_deleted(&name),
                },
                RadioEvent::WinlinkStatus(st) => self.mail.on_status(st),
                RadioEvent::MailListing(l) => self.mail.on_listing(l),
                RadioEvent::MailMessage(m) => self.mail.on_message(m),
                RadioEvent::MailSaved(mid) => self.mail.on_saved(mid),
                RadioEvent::MailDeleted { folder, mid } => self.mail.on_deleted(folder, &mid),
                RadioEvent::CallsignResult(info) => self.apply_callsign(info),
                RadioEvent::Upload(r) => self.on_upload_result(r),
                RadioEvent::Confirmations(recs) => self.apply_confirmations(recs),
                // Folded here rather than queued for the map's own pass: these
                // arrive whether or not a map is on screen, and a queue nobody
                // drained would grow all session. The display settings are
                // applied first because they decide whether this source counts
                // at all — `prop_texture` does the same before its own folds.
                RadioEvent::PropPaths(paths) => {
                    let v = self.view.solar3d;
                    self.prop.set_halflife_min(v.prop_halflife_min);
                    self.prop.set_sources(crate::prop_map::PropSources(v.prop_sources));
                    self.prop.observe_paths(
                        &paths,
                        sdroxide_types::PropSource::Rbn,
                        crate::time::now_unix(),
                    );
                }
                // Consumed by the controller, not here: the settings dialog
                // asks for the interface configuration through
                // `RadioController::radio_config` when it opens, and both
                // controllers answer from the authority nearest them — the
                // local one reads the file, the remote one the copy this
                // announcement left it. There is nothing to seed on the way
                // past.
                RadioEvent::RadioConfig(_) => {}
            }
        }
        // A switched-off skimmer stops emitting, so its last boxes would sit on
        // the waterfall until something else replaced them; drop them per kind.
        if !self.skimmer_spots.is_empty() {
            self.skimmer_spots.retain(|s| self.state.skimmer.enabled(s.kind));
        }
        // A hidden tab has no frame of its own to flush these on, and its
        // digital modes keep logging: send them now. MIDI is discarded, not
        // queued — a control surface drives the focused radio, and a backlog
        // of knob turns firing on tab switch would retune it at random.
        if !self.focused {
            for call in std::mem::take(&mut self.pending_lookups) {
                self.ctrl.send(Command::LookupCallsign { call });
            }
            for (qso_id, adif, targets) in std::mem::take(&mut self.pending_uploads) {
                self.ctrl.send(Command::UploadQso { qso_id, adif, targets });
            }
            #[cfg(not(target_arch = "wasm32"))]
            self.input.discard_midi();
        }
    }

    /// The awards dashboard: DXCC / WAS / WAZ / grid counts (worked vs
    /// confirmed) with a band filter, plus the WAS state grid and WAZ zone grid.
    /// The out-of-band transmit warning.
    ///
    /// Modal and dismissed by hand, because the band-edge lockout is the last
    /// thing between a mistyped frequency and an out-of-band transmission, and
    /// an operator who does not know it is off is exactly the operator who will
    /// find out the expensive way. Dismissing it is a one-shot acknowledgement,
    /// not a preference: it comes back next launch, because the flag has to be
    /// passed again next launch.
    ///
    /// Driven off the *engine's* state rather than off this process's arguments
    /// so a remote client is warned too — the licence at risk belongs to
    /// whoever is at the controls, who need not be whoever started the engine.
    fn oob_tx_window(&mut self, ctx: &egui::Context) {
        if !self.state.oob_tx || self.oob_tx_ack {
            return;
        }
        let mut dismissed = false;
        let resp = egui::Window::new("⚠  TRANSMIT LOCKOUT DISABLED")
            .id(crate::layout::salted_id(ctx, "oob-tx-window"))
            .frame(crate::chrome::window_frame())
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                crate::chrome::window_body_bg(ui);
                ui.set_max_width(crate::layout::window_w(ctx, 430.0));
                ui.label(
                    RichText::new(
                        "This engine was started with --oob-tx. The amateur-band lockout is \
                         off: it will key the transmitter on any frequency the hardware \
                         supports.",
                    )
                    .color(crate::theme::TEXT_STRONG())
                    .size(13.0),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new(
                        "Transmitting outside your licence is an offence in every country that \
                         issues one. Only continue if you are authorised to use the frequencies \
                         you are about to key on — a MARS/CAP or commercial licence, an \
                         experimental permit, or a dummy load.",
                    )
                    .color(crate::theme::TEXT()),
                );
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if crate::chrome::chip_accent(
                        ui,
                        false,
                        RichText::new("  I UNDERSTAND  ").strong(),
                        crate::theme::ALERT(),
                        crate::theme::TEXT_STRONG(),
                    )
                    .clicked()
                    {
                        dismissed = true;
                    }
                    ui.label(
                        RichText::new("Restart without --oob-tx to put the lockout back.")
                            .color(crate::theme::LINE_LIT())
                            .size(10.5),
                    );
                });
            });
        if let Some(r) = &resp {
            crate::chrome::paint_window_border(ctx, &r.response);
        }
        if dismissed {
            self.oob_tx_ack = true;
        }
    }

    /// Dispatch keyboard and mouse-button bindings for this frame.
    ///
    /// The bindings themselves live in `input.json` (see
    /// [`crate::input::InputRuntime`]); the shipped defaults reproduce the
    /// shortcuts that used to be hardcoded here — ←/→ ±100 Hz (Shift: ±10),
    /// ↑/↓ ±1 kHz, PgUp/PgDn ±10 kHz, M mute, N noise blanker, F fit span.
    fn control_inputs(&mut self, ctx: &egui::Context, now: f64, cmds: &mut Vec<Command>) {
        let mut speech_acts: Vec<sdroxide_types::Action> = Vec::new();
        // Destructured rather than borrowed field-by-field: the runtime needs
        // `state` and the window flags mutably at the same time, and they are
        // disjoint parts of `self`.
        let SdroxideApp {
            input,
            state,
            view,
            help,
            show_settings,
            show_logbook,
            show_spots,
            show_memories,
            show_voice,
            caps,
            wide_frame,
            ..
        } = self;
        // Read before the borrow below, which takes `self` apart.
        let rig_squelch = caps.as_ref().is_some_and(|c| c.commands_squelch);
        // How far a bound zoom-out may go: the full-band lane where the front
        // end has one, else its passband. Read here for the same reason
        // `rig_squelch` is — after this the borrow has taken `self` apart.
        let stated = caps.as_ref().map_or(0.0, |c| c.wide_span_hz);
        let seen = wide_frame.as_ref().map(|f| (f.center_hz, f.span_hz));
        let zoom_out = match (seen, stated > state.sample_rate) {
            (Some((c, span)), _) if span.max(stated) > state.sample_rate => (c, span.max(stated)),
            (None, true) => (state.center_hz, stated),
            _ => (state.center_hz, state.sample_rate),
        };
        let mut sink = crate::input::UiSink {
            view,
            help: &mut help.open,
            settings: show_settings,
            logbook: show_logbook,
            spots: show_spots,
            memories: show_memories,
            voice: show_voice,
            speech: &mut speech_acts,
            rig_squelch,
            zoom_out,
        };
        input.poll_pointer_and_keys(ctx, state, &mut sink, cmds);
        #[cfg(not(target_arch = "wasm32"))]
        input.poll_midi(ctx, state, &mut sink, cmds);
        drop(sink);
        for act in speech_acts {
            self.apply_speech_action(act, now);
        }
    }

    /// Answer a speech hotkey.
    fn apply_speech_action(&mut self, act: sdroxide_types::Action, now: f64) {
        use sdroxide_types::Action::*;
        match act {
            SpeakStatus => self.speech.announcer.say_status(&self.state, self.meters.as_ref(), now),
            SpeakRepeat => self.speech.announcer.repeat_last(now),
            SpeechSilence => self.speech.announcer.silence(),
            SpeechToggle => {
                let mut cfg = self.speech.settings().clone();
                cfg.enabled = !cfg.enabled;
                let turning_on = cfg.enabled;
                self.speech.set_settings(cfg.clone());
                crate::app::persist::persist_speech_settings(&cfg);
                // Confirm out loud when switching on; switching off is its own
                // confirmation, and speaking after being told to stop would be
                // a poor joke.
                if turning_on {
                    self.speech.announcer.say_sample(now);
                }
            }
            // `apply_action` only ever collects the four above.
            _ => {}
        }
    }

    /// De-assert every held control. Closing the window while a footswitch or
    /// a bound key is down must not leave the transmitter keyed.
    pub(in crate::app) fn release_held_controls(&mut self, cmds: &mut Vec<Command>) {
        let SdroxideApp {
            input,
            state,
            view,
            help,
            show_settings,
            show_logbook,
            show_spots,
            show_memories,
            show_voice,
            caps,
            ..
        } = self;
        let rig_squelch = caps.as_ref().is_some_and(|c| c.commands_squelch);
        let mut sink = crate::input::UiSink {
            view,
            help: &mut help.open,
            settings: show_settings,
            logbook: show_logbook,
            spots: show_spots,
            memories: show_memories,
            voice: show_voice,
            speech: &mut Vec::new(),
            rig_squelch,
            // Releasing held keys never pans or zooms, so the passband will do.
            zoom_out: (state.center_hz, state.sample_rate),
        };
        input.release_all(state, &mut sink, cmds);
    }
}

#[cfg(test)]
mod tests {
    use sdroxide_types::{Band, Mode};

    use super::{digi_split, qsy_clears_decodes};

    /// The reported bug: 20 m to 40 m left the previous band's decodes in the
    /// list, because the only thing that cleared it was a mode change and the
    /// digital-mode band buttons deliberately keep the mode.
    #[test]
    fn crossing_into_another_band_clears_the_decodes() {
        assert!(qsy_clears_decodes(Band::M20, Band::M40, Mode::Ft8));
        assert!(qsy_clears_decodes(Band::M40, Band::M20, Mode::Ft4));
        // Out of the amateur bands entirely is still somewhere else.
        assert!(qsy_clears_decodes(Band::M20, Band::Gen, Mode::Ft8));
    }

    /// Band-level, not a delta on the dial. Moving from the FT8 slot to the FT4
    /// slot, or into a DXpedition window, is the same band opening and the same
    /// stations — clearing there would empty the list every time the operator
    /// looked somewhere else in the segment.
    #[test]
    fn tuning_about_inside_a_band_clears_nothing() {
        assert!(!qsy_clears_decodes(Band::M20, Band::M20, Mode::Ft8));
        // …and the frequencies that motivates: both 20 m slots are one band.
        assert_eq!(Band::containing(14_074_000.0), Band::containing(14_080_000.0));
        // The pair that must *not* fold together, from the same reasoning.
        assert_ne!(Band::containing(14_074_000.0), Band::containing(7_074_000.0));
    }

    /// WSPR hops bands by itself once a slot when `wspr_hop` is set, and its
    /// spots each carry the frequency they were heard on, so there is nothing
    /// ambiguous to throw away. Clearing here would empty the list every two
    /// minutes.
    #[test]
    fn wspr_band_hopping_is_exempt() {
        assert!(!qsy_clears_decodes(Band::M20, Band::M40, Mode::Wspr));
        assert!(!qsy_clears_decodes(Band::M40, Band::M30, Mode::Wspr));
    }

    #[test]
    fn the_digi_split_always_fits_the_height_it_was_given() {
        // 277 pt is what the old independent clamps needed (190 panel + 80
        // waterfall + 7 handle); every height below it used to overflow, and a
        // phone in landscape has around 250 to give.
        //
        // The divider is the handle plus the gap either side of it — 7 + 2×5 on
        // a desktop, 7 + 2×7 on a touched layout. Both are checked: leaving the
        // gaps out is what hung the panel off the bottom of the window.
        for divider in [7.0f32, 17.0, 21.0] {
            for total in [120.0f32, 180.0, 200.0, 250.0, 277.0, 400.0, 900.0] {
                for fraction in [0.2f32, 0.5, 0.82] {
                    for row_h in [18.0f32, 34.0] {
                        let (wf, panel) = digi_split(total, divider, fraction, row_h);
                        assert!(wf >= 0.0 && panel >= 0.0, "negative split at {total}/{fraction}");
                        let used = wf + panel + divider;
                        assert!(
                            used <= total + 0.01,
                            "{total} pt tall, {fraction} panel, {divider} divider, {row_h} rows: \
                             asked for {used}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_roomy_window_still_honours_the_fraction_and_the_floors() {
        let (wf, panel) = digi_split(900.0, 7.0, 0.5, 18.0);
        assert!((panel - (893.0 * 0.5)).abs() < 0.01, "panel {panel} ignored the fraction");
        assert!(wf >= 80.0, "waterfall {wf} below its floor");
        // Dragged all the way down, the waterfall keeps its 80 pt minimum.
        let (wf, panel) = digi_split(900.0, 7.0, 0.99, 18.0);
        assert!(wf >= 80.0, "waterfall {wf} squeezed out");
        assert!(panel >= 242.0, "panel {panel} below its floor");
    }

    /// Issue #231: a 1366×768 screen is the *tablet* tier — touch metrics, rows
    /// half again as tall — and the panel's floor was a figure measured on a
    /// desktop. Fifty points short is all it takes: the transcript is handed
    /// less than its own scroll area will accept, overflows what it was given,
    /// and paints over the message row underneath.
    #[test]
    fn a_touched_layout_gets_a_taller_floor_under_the_panel() {
        // The window in the report: 1290×724, of which the digital area is 609
        // and the divider 21.
        let (_, desktop) = digi_split(609.0, 21.0, 0.46, 18.0);
        let (wf, touch) = digi_split(609.0, 21.0, 0.46, 34.0);
        assert!(touch > desktop, "the touched layout needs the taller floor");
        assert!(touch >= 295.0, "{touch} still squeezes the transcript out of its own minimum");
        assert!(wf >= 80.0, "waterfall {wf} squeezed out to pay for it");
        // A window with room to spare is untouched: the floor is a floor, not a
        // share.
        let (_, roomy) = digi_split(900.0, 21.0, 0.46, 34.0);
        assert!((roomy - (879.0 * 0.46)).abs() < 0.01, "the fraction still decides {roomy}");
        // And the floor can never take more than three fifths of the area,
        // however tall the rows: a waterfall is what the operator is watching.
        let (wf, panel) = digi_split(300.0, 21.0, 0.2, 34.0);
        assert!(panel <= 0.6 * 279.0 + 0.01, "floor {panel} ate the waterfall");
        assert!(wf > 0.0);
    }
}
