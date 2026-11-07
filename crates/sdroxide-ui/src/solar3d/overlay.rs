//! The egui chrome drawn inside the solar-system window: a wrapping top bar of
//! captioned modules over the 3D scene, styled with the app's own
//! [`crate::chrome`] widgets so the second window reads as part of sdroxide.

use crate::i18n::tr;
use eframe::egui::{self, RichText};
use sdroxide_solar::{SdoChannel, SolarData, Source, timefmt};

use super::state::{Focus, SolarUi};
use crate::chrome;
use crate::theme;
use crate::view::solar_layer as layer;

/// Layer chips, in bar order.
///
/// The star field and the heliographic graticule are not among them: they are
/// the backdrop and the coordinate frame everything else is read against, so
/// they are always drawn rather than being switches.
fn layers() -> [(u32, String, String); 12] {
    [
        (layer::PROPAGATION, tr!("solar-prop"), tr!("solar-where-signals-are-actually-getting")),
        (layer::ORBITS, tr!("solar-orbits"), tr!("solar-orbital-paths")),
        (layer::CLOUDS, tr!("solar-clouds"), tr!("solar-cloud-cover-noaa-s-global")),
        (layer::PLANETS, tr!("solar-planets"), tr!("solar-other-seven-planets-eighteen-their")),
        (layer::CME, String::from("CME"), tr!("solar-coronal-mass-ejection-trajectory-cones")),
        (layer::SUN_OBS, tr!("solar-sun-obs"), tr!("solar-solar-observations-sun-s-disk")),
        (layer::LABELS, tr!("solar-labels"), tr!("solar-body-region-labels")),
        (
            layer::SMALL_LABELS,
            tr!("solar-small-bodies"),
            tr!("solar-names-asteroids-comets-they-are"),
        ),
        (layer::QSO, String::from("QSO"), tr!("solar-decoded-ft8-ft4-stations-path")),
        (layer::SATS, tr!("solar-sats"), tr!("solar-amateur-radio-satellites-their-orbits")),
        (layer::AURORA, tr!("solar-aurora"), tr!("solar-auroral-oval-noaa-s-ovation")),
        (layer::AWARDS, tr!("solar-awards"), tr!("solar-award-coverage-every-dxcc-entity")),
    ]
}

/// The inset every floating panel keeps from the edge of the viewport.
const MARGIN: f32 = 12.0;

/// Where a readout box goes: parked against the right-hand edge of the scene,
/// or allocated inline in a menu popup.
///
/// A phone has no width to spare down the side for boxes that are read once and
/// then ignored — the aurora and propagation panels together take a third of a
/// 400 pt screen — so on one they move behind a menu chip instead. Same boxes
/// and the same code drawing them: only where they are put changes.
#[derive(Clone, Copy)]
enum Place {
    Corner { scene: egui::Rect, top: f32 },
    Inline,
}

impl Place {
    /// Reserve `size` and return the rect to paint into.
    ///
    /// `None` in the corner when the scene cannot hold the box — the rule every
    /// readout in this window keeps, because a panel clipped by the viewport
    /// edge is worse than no panel. A popup always has room: it is sized to
    /// what it is given.
    fn reserve(self, ui: &mut egui::Ui, size: egui::Vec2) -> Option<egui::Rect> {
        match self {
            Place::Corner { scene, top } => {
                let r = egui::Rect::from_min_size(
                    egui::pos2(scene.right() - size.x - MARGIN, top),
                    size,
                );
                scene.contains_rect(r).then_some(r)
            }
            Place::Inline => Some(ui.allocate_space(size).1),
        }
    }
}

/// How often the view redraws when nothing is happening to it.
///
/// This is a clock, not a document: the Earth turns, the terminator moves, the
/// Moon goes round and the QSO arcs follow the contacts. It has to keep running
/// whether or not the pointer is over it and whether or not the window has
/// focus — a frozen orrery in the corner of the screen is worse than none.
const IDLE_FRAME: std::time::Duration = std::time::Duration::from_millis(33);

pub fn ui(ui: &mut egui::Ui, st: &mut SolarUi) {
    crate::repaint::after(ui.ctx(), IDLE_FRAME);
    // Take a snapshot of the feed's data for this frame. Cloning the `Arc`
    // first means the guard's lifetime is not tied to `st`, which is borrowed
    // mutably by every module below.
    let handle = st.data.clone();
    let guard = handle.as_ref().map(|d| d.lock().unwrap_or_else(|e| e.into_inner()));
    let data = guard.as_deref();

    scene(ui, st, data);
}

// Fade slots in [`SolarUi::menu_since`], in bar order.
const M_VIEW: usize = 0;
const M_LAYERS: usize = 1;
const M_SUN: usize = 2;
const M_WEATHER: usize = 3;
const M_SCALE: usize = 4;
const M_TIME: usize = 5;
const M_ACTIVITY: usize = 6;
const M_PROP: usize = 7;

/// One menu chip's popup, wearing the same fade the main window's menus have: a
/// popup opened and then forgotten takes itself away rather than sitting over
/// the scene until something else is clicked. Off on a touched layout, where
/// there is no hover to hold it open with — that rule lives in
/// [`chrome::popup_fade_alpha`].
///
/// The open time is copied out and back rather than borrowed in place, because
/// the popup body needs `st` mutably too and `&mut st.menu_since[slot]` would
/// hold the whole of it.
fn menu(
    ui: &mut egui::Ui,
    st: &mut SolarUi,
    slot: usize,
    btn: egui::Response,
    add: impl FnOnce(&mut egui::Ui, &mut SolarUi),
) {
    let mut since = st.menu_since[slot];
    chrome::fading_menu_popup(ui, &btn, &mut since, |ui| add(ui, st));
    st.menu_since[slot] = since;
}

/// The menu chips along the top of the scene: one per control box, each opening
/// what that box used to hold.
///
/// Chips over the picture rather than a bar above it, because this window *is*
/// the picture. Six captioned boxes wrapped to three rows on anything narrower
/// than a desktop, which took a third of a phone screen away from the thing
/// being looked at, and all of it to show controls that are set once and then
/// left alone. Everything in them is one tap away instead.
///
/// Laid out from the clock's right edge to the right margin, wrapping as often
/// as it must. Returns the row it came out as, so the space-weather panels can
/// stack underneath rather than be drawn over.
fn menu_bar(
    ui: &mut egui::Ui,
    st: &mut SolarUi,
    data: Option<&SolarData>,
    rect: egui::Rect,
    clock_rect: Option<egui::Rect>,
    phone: bool,
) -> Option<egui::Rect> {
    // Freshness is a fact about the wall clock, not about the scrubbed scene: a
    // view wound forward a month has not made the imagery a month stale. The
    // space weather is read against the scene's own clock, like the panels it
    // stands in for.
    let wall = super::wall_clock_unix();
    let now = wall as i64;
    let sim_now = (wall + st.sim_offset_s) as i64;
    let left = clock_rect.map_or(rect.left() + MARGIN, |r| r.right() + 8.0);
    let width = rect.right() - MARGIN - left;
    // Narrower than a single chip and there is nowhere to put the bar at all.
    if width < 64.0 {
        return None;
    }

    let area = egui::Area::new(egui::Id::new("solar-menu"))
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(left, rect.top() + MARGIN))
        .constrain_to(rect)
        .show(ui.ctx(), |ui| {
            ui.set_max_width(width);
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
            ui.horizontal_wrapped(|ui| {
                let btn = chrome::chip(ui, st.view.auto, tr!("solar-view"))
                    .on_hover_text(tr!("solar-which-body-camera-orbits-animated"));
                menu(ui, st, M_VIEW, btn, view_controls);

                let btn = chrome::chip(ui, false, tr!("solar-layers"))
                    .on_hover_text(tr!("solar-what-drawn-orbits-clouds-cmes"));
                menu(ui, st, M_LAYERS, btn, |ui, st| {
                    chrome::menu_caption(ui, &tr!("solar-layers"));
                    layer_controls(ui, st);
                });

                let btn = chrome::chip(ui, false, "SUN")
                    .on_hover_text(tr!("solar-which-sdo-channel-wraps-sun"));
                menu(ui, st, M_SUN, btn, |ui, st| {
                    chrome::menu_caption(ui, "SUN");
                    sun_controls(ui, st, data, now);
                });

                // Only where the panels themselves cannot fit: on anything
                // wider they are already on screen, and a chip that opens a
                // copy of what is beside it is a chip that does nothing.
                if phone {
                    let btn = chrome::chip(ui, false, tr!("solar-weather"))
                        .on_hover_text(tr!("solar-aurora-propagation-numbers-which-bands"));
                    menu(ui, st, M_WEATHER, btn, |ui, st| {
                        chrome::menu_caption(ui, &tr!("solar-space-weather"));
                        let aurora = aurora_panel(ui, st, data, Place::Inline, sim_now).is_some();
                        let prop = weather_panel(ui, st, data, Place::Inline, sim_now).is_some();
                        let bands = bands_panel(ui, st, Place::Inline).is_some();
                        if !aurora && !prop && !bands {
                            ui.label(
                                RichText::new(tr!("solar-nothing-measured-yet"))
                                    .color(theme::CYAN_DIM())
                                    .size(10.5),
                            );
                        }
                    });
                }

                let btn = chrome::chip(ui, false, tr!("solar-scale"))
                    .on_hover_text(tr!("solar-body-moon-orbit-exaggeration"));
                menu(ui, st, M_SCALE, btn, |ui, st| {
                    chrome::menu_caption(ui, &tr!("solar-scale"));
                    scale_controls(ui, st);
                });

                let btn = chrome::chip(ui, st.sim_offset_s != 0.0, tr!("solar-time"))
                    .on_hover_text(tr!("solar-scrub-whole-scene-forward-back"));
                menu(ui, st, M_TIME, btn, |ui, st| {
                    chrome::menu_caption(ui, &tr!("solar-time"));
                    time_controls(ui, st);
                });

                let btn = chrome::chip(ui, st.lapse_playing, tr!("solar-activity"))
                    .on_hover_text(tr!("solar-replay-last-hour-decodes-globe"));
                menu(ui, st, M_ACTIVITY, btn, |ui, st| {
                    chrome::menu_caption(ui, &tr!("solar-activity"));
                    activity_controls(ui, st);
                });

                let on = st.layer(layer::PROPAGATION);
                let btn = chrome::chip(ui, on, tr!("solar-prop"))
                    .on_hover_text(tr!("solar-what-propagation-heat-map-showing"));
                menu(ui, st, M_PROP, btn, |ui, st| {
                    chrome::menu_caption(ui, &tr!("solar-propagation"));
                    prop_controls(ui, st);
                });
            });
        });
    Some(area.response.rect)
}

/// The animated tour, and the camera target: the whole solar system as chips,
/// laid out the way the system is — the Sun and the Earth–Moon pair first, then
/// one row per planet with its own moons beside it.
fn view_controls(ui: &mut egui::Ui, st: &mut SolarUi) {
    chrome::menu_caption(ui, &tr!("solar-view"));
    if chrome::chip_accent(ui, st.view.auto, tr!("solar-auto"), theme::CYAN(), theme::INK_ON_CYAN())
        .on_hover_text(tr!("solar-fly-spline-through-set-framed"))
        .clicked()
    {
        st.view.auto = !st.view.auto;
        if st.view.auto {
            st.tour.request_resume();
        }
    }

    let mut chosen = None;
    for (caption, targets) in Focus::groups() {
        chrome::menu_caption(ui, &caption);
        ui.horizontal_wrapped(|ui| {
            for f in targets {
                // Moons are dimmer, so a row reads as "this planet, and the
                // things that go round it".
                let text = if f.is_satellite() {
                    RichText::new(f.short()).size(11.5).color(theme::CYAN_DIM())
                } else {
                    RichText::new(f.short()).size(13.0)
                };
                if chrome::chip(ui, st.focus() == f, text).clicked() {
                    chosen = Some(f);
                }
            }
        });
    }
    ui.add_space(2.0);
    ui.label(
        RichText::new(tr!("solar-moons-ride-circular-orbits-fitted"))
            .color(theme::LINE_LIT())
            .size(10.0),
    );
    if let Some(f) = chosen {
        st.set_focus(f);
    }
}

fn layer_controls(ui: &mut egui::Ui, st: &mut SolarUi) {
    ui.horizontal_wrapped(|ui| {
        for (bit, label, hint) in layers() {
            // The award layer has nothing to paint without a logbook to paint
            // it from — the browser tab has none — and a chip that provably
            // does nothing is worse than no chip.
            if bit == layer::AWARDS && st.awards.is_empty() {
                continue;
            }
            // `layer` is already "any of these bits", so a chip standing for a
            // pair lights when either half is on and clears both when clicked.
            let on = st.layer(bit);
            if chrome::chip(ui, on, label).on_hover_text(hint).clicked() {
                st.set_layers(bit, !on);
                // Propagation and awards both wash the entire planet, and two
                // washes at once is neither. Switching one on stands the other
                // down rather than stacking them.
                if !on && (bit == layer::PROPAGATION || bit == layer::AWARDS) {
                    let other =
                        if bit == layer::PROPAGATION { layer::AWARDS } else { layer::PROPAGATION };
                    st.set_layers(other, false);
                }
            }
        }
    });
}

/// Which SDO product wraps the Sun, plus the honest freshness readout.
fn sun_controls(ui: &mut egui::Ui, st: &mut SolarUi, data: Option<&SolarData>, now: i64) {
    ui.horizontal_wrapped(|ui| {
        let current = SdoChannel::from_u8(st.view.channel);
        for c in SdoChannel::ALL {
            if chrome::chip(ui, current == c, c.label()).on_hover_text(c.description()).clicked() {
                st.view.channel = c.to_u8();
            }
        }
        if chrome::chip(ui, false, "↻")
            .on_hover_text(tr!("solar-fetch-everything-again-now"))
            .clicked()
        {
            st.refresh_requested = true;
        }
        if chrome::chip(ui, st.view.all_satellites, tr!("solar-all-sats"))
            .on_hover_text(tr!("solar-show-every-satellite-element-set"))
            .clicked()
        {
            st.view.all_satellites = !st.view.all_satellites;
        }

        // Say what is actually being shown. Presenting hours-old cached data as
        // if it were current is the one thing this readout must not do.
        let (text, color) = match data {
            None => (tr!("solar-starting").to_string(), theme::CYAN_DIM()),
            Some(d) => {
                let s = d.status(Source::Sun);
                match (s.age_secs(now), &s.last_error) {
                    (Some(age), None) => (timefmt::age(age), theme::GREEN()),
                    (Some(age), Some(_)) => {
                        (tr!("solar-age-offline", age = timefmt::age(age)), theme::YELLOW())
                    }
                    (None, Some(_)) => ("offline".to_string(), theme::ALERT()),
                    (None, None) => ("…".to_string(), theme::CYAN_DIM()),
                }
            }
        };
        ui.label(RichText::new(text).color(color).size(10.5));
    });
}

/// Size exaggeration. Positions are always real; only radii (and optionally the
/// Moon's orbit) are scaled, or nothing at this distance would be visible.
fn scale_controls(ui: &mut egui::Ui, st: &mut SolarUi) {
    // Names ride *inside* the boxes rather than beside them: a wrapping row
    // breaks wherever it runs out of width, and a "moon orbit" left at the end
    // of one line with its value at the start of the next names nothing.
    ui.horizontal_wrapped(|ui| {
        // The Moon renders *inside* the Earth once the exaggerated radii exceed
        // the (unexaggerated) Earth–Moon distance, so cap body scale against it.
        let max_body = super::max_body_scale(st.view.moon_orbit_scale);
        ui.add(
            egui::DragValue::new(&mut st.view.body_scale)
                .speed(0.25)
                .range(1.0..=max_body as f64)
                .prefix(tr!("solar-body"))
                .suffix("×"),
        )
        .on_hover_text(tr!("solar-radius-exaggeration", max = format!("{max_body:.0}")));
        ui.add(
            egui::DragValue::new(&mut st.view.moon_orbit_scale)
                .speed(0.1)
                .range(1.0..=30.0)
                .prefix(tr!("solar-moon-orbit"))
                .suffix("×"),
        )
        .on_hover_text(tr!("solar-stretch-earth-moon-distance-so"));
        ui.add(
            egui::DragValue::new(&mut st.view.sun_scale)
                .speed(0.1)
                .range(1.0..=20.0)
                .prefix(tr!("solar-sun"))
                .suffix("×"),
        )
        .on_hover_text(tr!("solar-sun-radius-exaggeration-leave-1"));
        st.view.body_scale = st.view.body_scale.clamp(1.0, max_body);
    });
}

/// Scrub the whole scene forward and back in time.
///
/// The month steps are what make the small bodies mean anything. A comet's
/// apparition is weeks long and its orbit is years, so at ±24 h a click you can
/// watch a tail grow but never reach the next one; a month a click walks
/// through a whole apparition in a dozen presses and out to the following
/// perihelion in a few dozen more.
///
/// The Sun's imagery, the aurora and the weather do not travel with it: they
/// are measurements of now, and a scene scrubbed to 2061 shows today's Sun
/// behind a correctly placed Halley. That is already true of the ±24 h steps
/// and the clock says which way it has been moved.
fn time_controls(ui: &mut egui::Ui, st: &mut SolarUi) {
    // A calendar month is not a fixed number of seconds; the scene's clock is,
    // so this is the average one. Over a scrub of years the drift against the
    // calendar is days, which is nothing to an ephemeris and everything to a
    // "+1 mo" that sometimes moved 28 days and sometimes 31.
    const MONTH_S: f64 = 365.2425 / 12.0 * 86_400.0;
    ui.horizontal_wrapped(|ui| {
        if chrome::chip(ui, st.sim_offset_s == 0.0, "NOW").clicked() {
            st.sim_offset_s = 0.0;
        }
        for (label, dt) in [
            ("−1mo", -MONTH_S),
            ("−24h", -86400.0),
            ("−1h", -3600.0),
            ("+1h", 3600.0),
            ("+24h", 86400.0),
            ("+1mo", MONTH_S),
        ] {
            if chrome::chip(ui, false, label).clicked() {
                st.sim_offset_s += dt;
            }
        }
    });
}

/// The FT8/FT4 activity time-lapse: where in the last hour the globe's arcs
/// are being replayed from, how long a trail they leave, and how fast the
/// replay runs.
/// What the propagation heat draws, and how long it remembers.
fn prop_controls(ui: &mut egui::Ui, st: &mut SolarUi) {
    // A popup sizes itself to its content, and with the layer switched off this
    // one's content is a single chip — which wrapped the explanation underneath
    // it into a column the width of the word SHOW. Ask for a readable width up
    // front so the box is the same size whether the layer is on or off, and so
    // the source chips below get a row rather than a stack. Clamped to the
    // viewport because this window opens on phones too; `popup_body` caps the
    // outer width at the same figure.
    let screen_w = ui.ctx().content_rect().width();
    ui.set_min_width((screen_w - 24.0).clamp(160.0, 330.0));

    let on = st.layer(layer::PROPAGATION);
    ui.horizontal_wrapped(|ui| {
        if chrome::chip_accent(ui, on, tr!("solar-show"), theme::CYAN(), theme::INK_ON_CYAN())
            .on_hover_text(tr!("solar-paint-globe-what-getting-through"))
            .clicked()
        {
            st.view.layers ^= layer::PROPAGATION;
            // Two full-globe washes at once are unreadable, so picking one
            // stands the other down rather than stacking them.
            if st.view.layers & layer::PROPAGATION != 0 {
                st.view.layers &= !layer::AWARDS;
            }
        }
    });
    if !on {
        ui.label(RichText::new(tr!("solar-paints-globe-what-getting-through")).size(10.5).weak());
        return;
    }

    chrome::menu_caption(ui, &tr!("solar-display"));
    ui.horizontal_wrapped(|ui| {
        let combined = st.view.prop_mode != 0;
        if chrome::chip(ui, combined, tr!("solar-all-bands"))
            .on_hover_text(tr!("solar-every-band-once-one-hue"))
            .clicked()
        {
            st.view.prop_mode = 1;
        }
        if chrome::chip(ui, !combined, tr!("solar-one-band"))
            .on_hover_text(tr!("solar-one-band-cold-hot-blue"))
            .clicked()
        {
            st.view.prop_mode = 0;
        }
    });

    let live = st.prop.live_bands();
    if st.view.prop_mode == 0 {
        chrome::menu_caption(ui, &tr!("solar-band"));
        ui.horizontal_wrapped(|ui| {
            if live.is_empty() {
                ui.label(RichText::new(tr!("solar-nothing-heard-yet")).size(10.5).weak());
            }
            for b in &live {
                let i = sdroxide_types::Band::ALL.iter().position(|x| x == b).unwrap_or(0) as u8;
                let sw = crate::colormap::band_color(*b);
                if chrome::chip(
                    ui,
                    st.view.prop_band == i,
                    RichText::new(b.label()).color(egui::Color32::from_rgb(sw[0], sw[1], sw[2])),
                )
                .clicked()
                {
                    st.view.prop_band = i;
                }
            }
        });
    } else {
        // The legend, and the only place the colours are named. Only bands with
        // something in them: a key full of dead bands is a key to nothing.
        chrome::menu_caption(ui, &tr!("solar-bands"));
        ui.horizontal_wrapped(|ui| {
            if live.is_empty() {
                ui.label(RichText::new(tr!("solar-nothing-heard-yet")).size(10.5).weak());
            }
            for b in &live {
                let i = sdroxide_types::Band::ALL.iter().position(|x| x == b).unwrap_or(0) as u8;
                let bit = 1u32 << i;
                let sw = crate::colormap::band_color(*b);
                if chrome::chip(
                    ui,
                    st.view.prop_bands & bit != 0,
                    RichText::new(b.label()).color(egui::Color32::from_rgb(sw[0], sw[1], sw[2])),
                )
                .clicked()
                {
                    st.view.prop_bands ^= bit;
                }
            }
        });
    }

    chrome::menu_caption(ui, &tr!("solar-sources"));
    ui.horizontal_wrapped(|ui| {
        let mut src = crate::prop_map::PropSources(st.view.prop_sources);
        for s in sdroxide_types::PropSource::ALL {
            if chrome::chip(ui, src.has(s), s.label())
                .on_hover_text(match s {
                    sdroxide_types::PropSource::Logged => {
                        tr!("solar-contacts-logbook-path-was-open")
                    }
                    sdroxide_types::PropSource::WsprHeardUs => {
                        tr!("solar-stations-reported-hearing-one-downloaded")
                    }
                    sdroxide_types::PropSource::Rbn => {
                        tr!("solar-reverse-beacon-network-skimmers-what")
                    }
                    _ => {
                        tr!("solar-decodes-station-made-each-measured")
                    }
                })
                .clicked()
            {
                src.toggle(s);
                st.view.prop_sources = src.0;
            }
        }
    });

    chrome::menu_caption(ui, &tr!("solar-memory"));
    ui.horizontal_wrapped(|ui| {
        ui.add(
            egui::DragValue::new(&mut st.view.prop_halflife_min)
                .speed(1.0)
                .range(5.0..=240.0)
                .prefix(tr!("solar-half-life"))
                .suffix(" min"),
        )
        .on_hover_text(tr!("solar-how-long-reception-takes-count"));
    });
    // The absolute scale behind the colours. Without it the same colour means
    // different densities on different evenings, and the picture is lying by
    // omission about how much evidence is behind it.
    let peak =
        live.iter().filter_map(|b| st.prop.plane(*b)).map(|p| p.peak()).fold(0.0f32, f32::max);
    if peak > 0.0 {
        ui.label(
            RichText::new(tr!(
                "solar-brightest-cell",
                peak = format!("{peak:.0}"),
                bands = live.len()
            ))
            .size(10.5)
            .weak(),
        );
    }
}

fn activity_controls(ui: &mut egui::Ui, st: &mut SolarUi) {
    ui.horizontal_wrapped(|ui| {
        let hour = crate::digi_map::HISTORY_S as f64;
        if chrome::chip(ui, st.lapse_live() && !st.lapse_playing, tr!("solar-live"))
            .on_hover_text(tr!("solar-follow-band-happens"))
            .clicked()
        {
            st.lapse_playing = false;
            st.set_lapse_back(0.0);
        }
        if chrome::chip_accent(
            ui,
            st.lapse_playing,
            if st.lapse_playing { tr!("solar-replay") } else { tr!("solar-replay-2") },
            theme::CYAN(),
            theme::INK_ON_CYAN(),
        )
        .on_hover_text(tr!("solar-replay-last-hour-decodes-over"))
        .clicked()
        {
            st.lapse_playing = !st.lapse_playing;
            // Starting from the present would replay nothing, so a replay
            // begun while live starts at the top of the hour.
            if st.lapse_playing && st.lapse_live() {
                st.set_lapse_back(hour);
            }
        }

        // The head, as minutes ago. Dragging it is also how you stop the
        // replay running away from where you wanted to look.
        let mut back_min = (st.lapse_back_s / 60.0) as f32;
        let resp = ui.add(
            egui::DragValue::new(&mut back_min)
                .speed(0.5)
                .range(0.0..=(hour / 60.0))
                .suffix(tr!("solar-min-ago")),
        );
        if resp.on_hover_text(tr!("solar-where-last-hour-globe-showing")).changed() {
            st.set_lapse_back(back_min as f64 * 60.0);
            st.lapse_playing = false;
        }

        // Named inside the box, for the reason the scale controls are.
        ui.add(
            egui::DragValue::new(&mut st.view.lapse_trail_min)
                .speed(0.25)
                .range(0.5..=(hour / 60.0))
                .prefix(tr!("solar-trail"))
                .suffix(" min"),
        )
        .on_hover_text(tr!("solar-how-long-decode-s-arc"));

        ui.add(
            egui::DragValue::new(&mut st.view.lapse_speed)
                .speed(1.0)
                .range(1.0..=600.0)
                .prefix(tr!("solar-speed"))
                .suffix("×"),
        )
        .on_hover_text(tr!("solar-how-much-faster-than-real"));

        let hits = st.digi.history.len();
        let (text, color) = if !st.layer(layer::QSO) {
            (tr!("solar-qso-layer-off").to_string(), theme::YELLOW())
        } else if hits == 0 {
            (tr!("solar-no-decodes-yet").to_string(), theme::CYAN_DIM())
        } else if st.lapse_live() {
            (tr!("solar-hits-in-hour", hits = hits), theme::GREEN())
        } else {
            (format!("−{:.0} min", st.lapse_back_s / 60.0), theme::CYAN())
        };
        ui.label(RichText::new(text).color(color).size(10.5));
    });
}

/// Step the activity replay head, at `speed` × real time, looping round the
/// hour. Looping and not stopping at the present on purpose: the point of a
/// replay is to watch the opening come and go more than once.
fn advance_lapse(ui: &egui::Ui, st: &mut SolarUi, dt: f32) {
    if !st.lapse_playing {
        return;
    }
    let step = dt.max(0.0) as f64 * st.view.lapse_speed.clamp(1.0, 600.0) as f64;
    let back = st.lapse_back_s - step;
    st.set_lapse_back(if back <= 0.0 { crate::digi_map::HISTORY_S as f64 } else { back });
    crate::repaint::animate(ui.ctx());
}

/// The 3D scene: mouse interaction, the wgpu paint callback, then the readouts
/// painted over it.
fn scene(ui: &mut egui::Ui, st: &mut SolarUi, data: Option<&SolarData>) {
    let rect = ui.available_rect_before_wrap();
    if rect.width() < 4.0 || rect.height() < 4.0 {
        return;
    }
    let resp = ui.allocate_rect(rect, egui::Sense::click_and_drag());
    let sim_now = super::wall_clock_unix() + st.sim_offset_s;
    interact(ui, st, &resp, sim_now);

    let ppp = ui.ctx().pixels_per_point();
    let px = [(rect.width() * ppp).round().max(1.0), (rect.height() * ppp).round().max(1.0)];
    let anim = ui.input(|i| i.time);
    // One elapsed-time step for everything animated off the wall clock, so the
    // tour and the activity replay cannot disagree about how long a frame was.
    let dt =
        if st.last_frame_time <= 0.0 { 1.0 / 60.0 } else { (anim - st.last_frame_time) as f32 };
    st.last_frame_time = anim;
    advance_tour(ui, st, data, sim_now, dt);
    advance_lapse(ui, st, dt);
    reframe(st, sim_now);
    let mut scene = super::scene::build(st, data, sim_now, px, anim as f32);
    // Labels and pick targets are egui's business, not the GPU's, so they come
    // out before the rest of the scene is handed to the paint callback.
    let labels = std::mem::take(&mut scene.labels);
    let picks = std::mem::take(&mut scene.picks);
    let qso_apex = scene.qso_apex;
    let view_proj = scene.globals.view_proj;

    let (sun_img, sun_gen, aurora, aurora_gen, clouds, clouds_gen) = match data {
        Some(d) => (
            d.sun.clone(),
            d.sun_gen,
            d.aurora.clone(),
            d.aurora_gen,
            d.clouds.clone(),
            d.clouds_gen,
        ),
        None => (None, 0, None, 0, None, 0),
    };
    // Resolve the propagation field to pixels, once per change rather than per
    // frame. Deliberately the same function the flat map calls: the two views
    // upload identical bytes, so there is no way for them to disagree about
    // what a cell means.
    let (prop, prop_gen) = if st.layer(crate::view::solar_layer::PROPAGATION) {
        let band = sdroxide_types::Band::ALL
            .get(st.view.prop_band as usize)
            .copied()
            .unwrap_or(sdroxide_types::Band::M20);
        let mode = if st.view.prop_mode == 0 {
            crate::prop_map::PropMode::PerBand
        } else {
            crate::prop_map::PropMode::AllBands
        };
        let key = (st.prop.generation, st.view.prop_mode, st.view.prop_band, st.view.prop_bands);
        if st.prop_rgba_key != Some(key) {
            let (rgba, _, _) =
                crate::prop_map::PropHeat::rgba(&st.prop, mode, band, st.view.prop_bands);
            st.prop_rgba = std::sync::Arc::new(rgba);
            st.prop_rgba_key = Some(key);
            st.prop_gen = st.prop_gen.wrapping_add(1);
        }
        (Some(std::sync::Arc::clone(&st.prop_rgba)), st.prop_gen)
    } else {
        (None, st.prop_gen)
    };
    ui.painter().add(crate::egui_wgpu::Callback::new_paint_callback(
        rect,
        super::gpu::SolarCallback {
            scene: std::sync::Arc::new(scene),
            px_size: [px[0] as u32, px[1] as u32],
            sun_img,
            sun_gen,
            aurora,
            aurora_gen,
            clouds,
            clouds_gen,
            prop,
            prop_gen,
        },
    ));

    let took_click = draw_labels(ui, st, rect, &view_proj, &labels, &resp);
    // Only if a label did not already take it: a name sits on top of its own
    // body, and clicking the text should not be ambiguous.
    pick_bodies(ui, st, rect, &view_proj, &picks, &resp, took_click);
    // After the labels, so the contact reads over the traffic around it rather
    // than through it — a callsign printed across the card is the one place on
    // this globe where decluttering by rank is not enough.
    qso_card(ui, st, rect, &view_proj, qso_apex, anim);
    // A phone puts the space weather behind the menu bar's WEATHER chip instead
    // of down the right-hand edge; see [`Place`].
    //
    // Measured from this window rather than read from `layout::tier`, which is
    // the whole app's: natively this view is a child viewport sharing the main
    // window's context, so the app-wide tier says how big the *radio* window is.
    // The override still counts — an operator who asked for the phone layout
    // meant everywhere — so it is either that or this window's own size.
    let phone = crate::layout::tier(ui.ctx()) == crate::layout::Tier::Phone
        || crate::layout::tier_for(rect.size(), sdroxide_types::LayoutMode::Auto)
            == crate::layout::Tier::Phone;
    let clock_rect = clock(ui, rect, sim_now, st.sim_offset_s != 0.0);
    // The menu row owns the top band outright, and everything else starts under
    // it: the alternative is measuring a panel that has not been drawn yet to
    // find out whether the chips would have reached it.
    let menu_rect = menu_bar(ui, st, data, rect, clock_rect, phone);
    find_box(ui, st, data, rect, clock_rect);
    let top = menu_rect.map_or(rect.top() + MARGIN, |r| r.bottom() + 8.0);
    if !phone {
        let aurora = aurora_panel(ui, st, data, Place::Corner { scene: rect, top }, sim_now as i64);
        let below = aurora.map_or(top, |r| r.bottom() + 8.0);
        let weather =
            weather_panel(ui, st, data, Place::Corner { scene: rect, top: below }, sim_now as i64);
        let below = weather.map_or(below, |r| r.bottom() + 8.0);
        let _ = bands_panel(ui, st, Place::Corner { scene: rect, top: below });
    }
    // The award key in the bottom-right corner; the date is on the clock.
    award_panel(ui, st, rect, rect.bottom() - MARGIN);
    clouds_note(ui, st, data, rect, sim_now as i64);
    impact_banner(ui, data, rect, sim_now as i64);
    pass_window(ui, st, data, sim_now);

    if st.qth.is_none() {
        ui.painter().text(
            rect.right_top() + egui::vec2(-12.0, 12.0),
            egui::Align2::RIGHT_TOP,
            tr!("solar-qth-not-set-enter-your"),
            egui::FontId::proportional(12.5),
            theme::scope().warn,
        );
    }
}

/// Project a world point into the widget. `None` when it is behind the eye.
fn project(view_proj: &[[f32; 4]; 4], rect: egui::Rect, world: [f32; 3]) -> Option<egui::Pos2> {
    // Column-major, so column i scales input component i.
    let mut o = [0.0f32; 4];
    for (r, out) in o.iter_mut().enumerate() {
        *out = view_proj[0][r] * world[0]
            + view_proj[1][r] * world[1]
            + view_proj[2][r] * world[2]
            + view_proj[3][r];
    }
    if o[3] <= 0.0 {
        return None;
    }
    Some(egui::pos2(
        rect.left() + (o[0] / o[3] * 0.5 + 0.5) * rect.width(),
        rect.top() + (0.5 - o[1] / o[3] * 0.5) * rect.height(),
    ))
}

/// How much clear space a label keeps around itself before another may be
/// painted next to it, in pixels.
const LABEL_GAP: egui::Vec2 = egui::vec2(3.0, 1.5);

/// Project the scene's labels to screen space, paint them, and let one be
/// clicked. Returns whether the click was consumed.
///
/// The 3D pass has no text rendering, and adding one for a dozen short strings
/// would be far more machinery than projecting a dozen points with the same
/// matrix the vertex shaders use.
///
/// Crowding is settled here rather than in the scene, because it is a question
/// about *pixels*: two callsigns a thousand kilometres apart share a patch of
/// screen at one zoom and are half the window apart at the next. The scene says
/// which label matters more (`rank`); this decides which of them fits.
fn draw_labels(
    ui: &egui::Ui,
    st: &mut SolarUi,
    rect: egui::Rect,
    view_proj: &[[f32; 4]; 4],
    labels: &[super::scene::Label],
    resp: &egui::Response,
) -> bool {
    use super::scene::{Click, RANK_ALWAYS};

    let font = egui::FontId::proportional(11.5);
    let pointer = ui.input(|i| i.pointer.interact_pos());
    // A press-and-release without dragging: a drag is the camera's, a click is
    // the label's.
    let clicked = resp.clicked();
    let mut hit: Option<Click> = None;
    let p = ui.painter();

    // Best first, so the important names have taken their space by the time the
    // rest ask for what is left. A stable sort, so labels of equal rank keep the
    // order the scene built them in and a tie does not flicker between frames.
    let mut order: Vec<usize> = (0..labels.len()).collect();
    order.sort_by_key(|&i| labels[i].rank);
    let mut taken: Vec<egui::Rect> = Vec::with_capacity(labels.len());

    for l in order.into_iter().map(|i| &labels[i]) {
        let Some(anchor) = project(view_proj, rect, l.world) else { continue };
        let pos = anchor + egui::vec2(l.offset[0], l.offset[1]);
        if !rect.contains(pos) {
            continue;
        }

        let galley = p.layout_no_wrap(l.text.clone(), font.clone(), l.color);
        let text_rect =
            egui::Rect::from_min_size(pos - egui::vec2(0.0, galley.size().y * 0.5), galley.size());
        // Two callsigns overprinted are less use than one, so a name that would
        // land on one already painted is dropped instead of stacked. Zoom in and
        // it comes back, which is the same answer the map gives to every other
        // "there is too much here to read".
        let claim = text_rect.expand2(LABEL_GAP);
        if l.rank != RANK_ALWAYS && taken.iter().any(|r| r.intersects(claim)) {
            continue;
        }
        taken.push(claim);
        let hovered =
            l.click != Click::None && pointer.is_some_and(|q| text_rect.expand(4.0).contains(q));
        if hovered {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            // A backing plate, so it reads as something you can press.
            p.rect_filled(
                text_rect.expand2(egui::vec2(4.0, 2.0)),
                0,
                theme::FILL().gamma_multiply(0.85),
            );
            if clicked {
                hit = Some(l.click);
            }
        }

        // A dark halo, so a label stays readable over the bright solar disk as
        // well as over empty space.
        let color = if hovered { theme::scope().ink_strong } else { l.color };
        let shadow = egui::Color32::from_black_alpha(color.a().saturating_sub(40));
        p.text(
            pos + egui::vec2(1.0, 1.0),
            egui::Align2::LEFT_CENTER,
            &l.text,
            font.clone(),
            shadow,
        );
        p.text(pos, egui::Align2::LEFT_CENTER, &l.text, font.clone(), color);
    }

    match hit {
        // Clicking the open satellite's label again closes the table.
        Some(Click::Sat(id)) => {
            st.selected_sat = if st.selected_sat == Some(id) { None } else { Some(id) };
            true
        }
        Some(Click::Focus(f)) => {
            st.set_focus(f);
            true
        }
        _ => false,
    }
}

/// How long one line of the contact card takes to type itself out.
const CARD_LINE_S: f64 = 0.5;
/// Gap between the arc's apex and the bottom of the card, in pixels — enough
/// that the leader reads as a leader and the arc is not hidden under its own
/// caption.
const CARD_LEADER_PX: f32 = 18.0;
/// The block that trails the text as it is typed. A real terminal cursor rather
/// than an underscore: it has to be visible over a bright limb as well as over
/// space.
const CARD_CURSOR: &str = "▌";

/// One typed row of the contact card.
struct CardLine {
    text: String,
    font: egui::FontId,
    color: egui::Color32,
    /// Extra space under this row — the heading gets a little air.
    gap: f32,
}

/// The card over the QSO arc: what the contact is, typed out a line at a time.
///
/// It is pinned to the arc's apex in *world* space and reprojected every frame,
/// so it rides the arc as the globe turns rather than sitting at a screen
/// position that was right once. Behind the eye, or off the edge of the widget,
/// it simply is not drawn — a caption clamped to the edge no longer points at
/// anything, and the arc is what it is a caption for.
///
/// The typing is not decoration alone. A contact is a thing that happens over a
/// minute or two, and a card that assembles itself as the QSO runs reads as
/// live, where the same nine lines appearing at once read as a static panel
/// that happens to be over the globe.
fn qso_card(
    ui: &egui::Ui,
    st: &mut SolarUi,
    rect: egui::Rect,
    view_proj: &[[f32; 4]; 4],
    apex: Option<[f32; 3]>,
    now: f64,
) {
    let Some(info) = st.digi.qso.clone() else {
        // No contact: forget where the typing had got to, so the next one
        // starts from a blank card instead of finishing the last one's text.
        st.qso_card = None;
        return;
    };
    let Some(anchor) = apex.and_then(|w| project(view_proj, rect, w)) else { return };

    // Working the same station twice is two contacts, and the second gets its
    // own card — so the start time is part of the identity, not just the call.
    let key = format!("{}|{}", info.call, info.started_utc.unwrap_or(0));
    let t0 = match &st.qso_card {
        Some((k, t)) if *k == key => *t,
        _ => {
            st.qso_card = Some((key, now));
            now
        }
    };
    let elapsed = now - t0;

    let head_font = egui::FontId::monospace(13.0);
    let font = egui::FontId::monospace(11.0);
    let mut lines = vec![CardLine {
        text: info.call.clone(),
        font: head_font,
        color: theme::GREEN(),
        gap: 5.0,
    }];
    // A fixed-width label column, which is what makes a monospace card line up
    // without laying out two galleys per row and measuring between them.
    let mut row = |label: &str, value: String| {
        lines.push(CardLine {
            text: format!("{label:<5}{value}"),
            font: font.clone(),
            color: theme::TEXT(),
            gap: 1.0,
        });
    };
    let mode = match (info.mode.as_str(), info.band.as_str()) {
        (m, "") => m.to_string(),
        (m, b) => format!("{m} · {b}"),
    };
    row(&tr!("solar-mode"), mode);
    if let Some(km) = info.distance_km {
        row(
            &tr!("solar-path"),
            match info.bearing_deg {
                Some(b) => format!("{km:.0} km · {b:.0}°"),
                None => format!("{km:.0} km"),
            },
        );
    }
    if let Some(started) = info.started_utc {
        row(&tr!("solar-time"), elapsed_hms((crate::time::now_unix() - started).max(0)));
    }
    // The two halves of the exchange, and then the live number. `SENT` and
    // `RCVD` are what the contact settled on; `SIG` is what they are doing
    // right now, which keeps moving after the reports are agreed.
    if let Some(r) = info.rpt_sent {
        row(&tr!("solar-sent"), format!("{r:+} dB"));
    }
    if let Some(r) = info.rpt_rcvd {
        row(&tr!("solar-rcvd"), format!("{r:+} dB"));
    }
    if let Some(s) = info.snr_db {
        row("SIG", format!("{s:+} dB"));
    }
    if let Some(g) = &info.grid {
        row(&tr!("solar-grid"), g.clone());
    }
    if let Some(e) = info.entity {
        row(&tr!("solar-dxcc"), e.to_string());
    }

    // Lay the whole card out at its finished size before typing a character of
    // it. A box that grows as the text arrives is a box that moves the text
    // already on it, and the reader loses their place every half second.
    let p = ui.painter();
    let galleys: Vec<_> =
        lines.iter().map(|l| p.layout_no_wrap(l.text.clone(), l.font.clone(), l.color)).collect();
    const PAD: egui::Vec2 = egui::vec2(11.0, 8.0);
    // Room for the cursor past the longest line, so it never sits on the border.
    let cursor_w = p.layout_no_wrap(CARD_CURSOR.into(), font.clone(), theme::GREEN()).size().x;
    let w = galleys.iter().map(|g| g.size().x).fold(0.0, f32::max) + cursor_w + PAD.x * 2.0;
    let h = galleys.iter().zip(&lines).map(|(g, l)| g.size().y + l.gap).sum::<f32>() + PAD.y * 2.0;

    let Some((card, above)) = card_rect(rect, anchor, egui::vec2(w, h)) else { return };

    // A leader to the apex, so the card is visibly *this* arc's. It leaves from
    // whichever edge faces the anchor, and from the point on that edge nearest
    // to it — the card may have been slid sideways to stay on screen.
    let foot = egui::pos2(
        anchor.x.clamp(card.left() + 6.0, card.right() - 6.0),
        if above { card.bottom() } else { card.top() },
    );
    p.line_segment([foot, anchor], egui::Stroke::new(1.0, theme::GREEN().gamma_multiply(0.7)));
    p.circle_filled(anchor, 2.0, theme::GREEN());
    p.rect_filled(card, 0, theme::BG_DEEP().gamma_multiply(0.82));
    chrome::paint_cut_border(p, card, theme::GREEN(), egui::Color32::TRANSPARENT);

    // Type it. Line `i` runs from `i * CARD_LINE_S` to the end of its own
    // half-second, so the whole card is a steady rhythm rather than long lines
    // taking longer than short ones.
    let mut y = card.top() + PAD.y;
    let x = card.left() + PAD.x;
    let mut typing = false;
    for (i, (l, g)) in lines.iter().zip(&galleys).enumerate() {
        let row_h = g.size().y;
        let total = l.text.chars().count();
        let Some(shown) = typed_chars(elapsed, i, total) else {
            break; // and every line below this one is still to come
        };
        let head: String = l.text.chars().take(shown).collect();
        let galley = p.layout_no_wrap(head, l.font.clone(), l.color);
        let end = x + galley.size().x;
        p.galley(egui::pos2(x, y), galley, l.color);
        // The cursor rides the line being typed; when the card is finished it
        // parks at the end of the last line and blinks, which is what says the
        // card is live and not a screenshot.
        let last = i == lines.len() - 1;
        if shown < total {
            typing = true;
            p.text(
                egui::pos2(end, y),
                egui::Align2::LEFT_TOP,
                CARD_CURSOR,
                l.font.clone(),
                theme::GREEN(),
            );
        } else if last && (now * 1.6).fract() < 0.5 {
            p.text(
                egui::pos2(end, y),
                egui::Align2::LEFT_TOP,
                CARD_CURSOR,
                l.font.clone(),
                theme::GREEN(),
            );
        }
        y += row_h + l.gap;
    }
    // The window animates the globe anyway; this only guarantees the cursor
    // keeps its rhythm if that ever stops.
    if typing || elapsed < lines.len() as f64 * CARD_LINE_S + 1.0 {
        crate::repaint::animate(ui.ctx());
    } else {
        crate::repaint::after_ms(ui.ctx(), 120);
    }
}

/// Where a card of `size` sits when it hangs off `anchor` inside `bounds`, and
/// whether it ended up above the anchor. `None` when it fits neither way.
///
/// Above the anchor by preference, because that is where it belongs — but under
/// it when there is no room above, which there very often is: the apex is the
/// *highest* point of the arc, so it is the point most likely to be near the top
/// of the widget, and a card that disappears whenever the operator zooms in on
/// their own contact is no card at all. Sideways it slides along the edge rather
/// than being dropped; the leader still lands on the anchor, which is what says
/// which arc it belongs to.
fn card_rect(
    bounds: egui::Rect,
    anchor: egui::Pos2,
    size: egui::Vec2,
) -> Option<(egui::Rect, bool)> {
    let above = anchor.y - CARD_LEADER_PX - size.y;
    let below = anchor.y + CARD_LEADER_PX;
    let (top, is_above) = if above >= bounds.top() {
        (above, true)
    } else if below + size.y <= bounds.bottom() {
        (below, false)
    } else {
        return None; // no room either way — the widget is smaller than the card
    };
    let left = (anchor.x - size.x * 0.5)
        .clamp(bounds.left(), (bounds.right() - size.x).max(bounds.left()));
    let card = egui::Rect::from_min_size(egui::pos2(left, top), size);
    bounds.contains_rect(card).then_some((card, is_above))
}

/// A running QSO's length: `M:SS` until it passes the hour, then `H:MM:SS`.
fn elapsed_hms(secs: i64) -> String {
    let (h, m, s) = (secs / 3600, (secs / 60) % 60, secs % 60);
    if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m}:{s:02}") }
}

/// How much of line `line` is on screen `elapsed` seconds after the card
/// opened, out of `total` characters. `None` before the line's turn comes.
///
/// Every line gets the same [`CARD_LINE_S`] whatever its length, so the card
/// types at a steady beat: a nine-character line finishing in a third of the
/// time a thirty-character one takes would read as stuttering rather than as
/// typing.
fn typed_chars(elapsed: f64, line: usize, total: usize) -> Option<usize> {
    let t = (elapsed - line as f64 * CARD_LINE_S) / CARD_LINE_S;
    if t <= 0.0 {
        return None;
    }
    // `ceil`, so a line begins with its first character rather than with an
    // empty string under a cursor.
    Some(if t >= 1.0 { total } else { ((t * total as f64).ceil() as usize).min(total) })
}

/// Let the bodies themselves be clicked, not only their names.
///
/// Picking is done in screen space against the projected centres rather than by
/// reading back a depth or id buffer: there are under thirty candidates, and a
/// GPU readback would stall the frame for something a dot product answers.
fn pick_bodies(
    ui: &egui::Ui,
    st: &mut SolarUi,
    rect: egui::Rect,
    view_proj: &[[f32; 4]; 4],
    picks: &[super::scene::Pick],
    resp: &egui::Response,
    consumed: bool,
) {
    let Some(pointer) = ui.input(|i| i.pointer.interact_pos()) else { return };
    if !rect.contains(pointer) {
        return;
    }

    // Nearest hit wins, measured in fractions of each body's own radius, so a
    // moon in front of its planet is reachable rather than swallowed by it.
    let mut best: Option<(f32, &super::scene::Pick, egui::Pos2)> = None;
    for pick in picks {
        if st.focus() == pick.focus {
            continue; // already the target; nothing to click it for
        }
        let Some(pos) = project(view_proj, rect, pick.world) else { continue };
        let d = pos.distance(pointer);
        if d > pick.radius_px {
            continue;
        }
        let score = d / pick.radius_px;
        if best.as_ref().is_none_or(|(b, _, _)| score < *b) {
            best = Some((score, pick, pos));
        }
    }
    let Some((_, pick, pos)) = best else { return };

    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    // A reticle, so it is obvious what the click will grab.
    let r = pick.radius_px.clamp(9.0, 90.0);
    ui.painter().circle_stroke(pos, r, egui::Stroke::new(1.2, theme::CYAN().gamma_multiply(0.8)));
    ui.painter().text(
        pos + egui::vec2(0.0, -r - 4.0),
        egui::Align2::CENTER_BOTTOM,
        pick.focus.short(),
        egui::FontId::proportional(11.0),
        theme::CYAN(),
    );
    if resp.clicked() && !consumed {
        st.set_focus(pick.focus);
    }
}

/// Frame the camera on a newly chosen target.
///
/// Without this, picking Io while framed on the whole system leaves the camera
/// 2 AU away pointed at something 3600 km across, which reads as the click
/// having done nothing. The distance is set from the target's own radius, and
/// the orientation is left alone so the move feels like a pan rather than a
/// reset.
fn reframe(st: &mut SolarUi, sim_now: f64) {
    if !std::mem::take(&mut st.retarget) {
        return;
    }
    let b = super::scene::bodies(st, sim_now);
    let (_, radius) = b.focus(st.focus());
    let (lo, hi) = super::camera::dist_range(radius);
    st.view.dist = (radius * 9.0).clamp(lo, hi);
}

/// The pass table for the selected satellite.
///
/// Prediction is expensive — stepping a whole orbit and bisecting each horizon
/// crossing — so it is computed once per selection and refreshed only as it
/// ages out, never per frame.
fn pass_window(ui: &egui::Ui, st: &mut SolarUi, data: Option<&SolarData>, sim_now: f64) {
    use sdroxide_solar::{PassSearch, satellites::compass};

    let Some(id) = st.selected_sat else { return };
    let Some(d) = data else { return };
    let Some((lat, lon)) = st.qth else {
        st.selected_sat = None;
        return;
    };
    let Some(sat) = d.satellites().find(|s| s.norad_id == id) else {
        // The element set was replaced and no longer has it.
        st.selected_sat = None;
        return;
    };

    // Recompute on a change of satellite or QTH, or once the prediction has
    // aged enough that its first pass may have gone by.
    let stale = st.sat_passes.as_ref().is_none_or(|c| {
        c.norad_id != id || c.qth != (lat, lon) || (sim_now - c.computed_unix).abs() > 300.0
    });
    if stale {
        st.sat_passes = Some(super::state::SatPasses {
            norad_id: id,
            name: sat.name.clone(),
            qth: (lat, lon),
            computed_unix: sim_now,
            // Two days ahead is enough for "the next few" without spending long
            // in the search: a LEO satellite gives four to six usable passes.
            result: sat.next_passes(lat, lon, sim_now, 48.0 * 3600.0, 6),
        });
    }
    let Some(cache) = &st.sat_passes else { return };

    let mut open = true;
    egui::Window::new(tr!(
        "solar-passes-from",
        name = cache.name.to_string(),
        grid = st.qth_grid.to_string()
    ))
    .id(egui::Id::new("solar-passes"))
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .frame(chrome::window_frame())
    .default_pos(ui.max_rect().center() - egui::vec2(230.0, 120.0))
    .show(ui.ctx(), |ui| {
        crate::chrome::window_body_bg(ui);
        ui.label(
            RichText::new(tr!(
                "solar-elements-age",
                age = timefmt::age(sat.element_age_s(sim_now) as i64)
            ))
            .color(theme::CYAN_DIM())
            .size(10.0),
        );
        ui.add_space(4.0);

        match &cache.result {
            PassSearch::AlwaysVisible { elevation, azimuth } => {
                ui.label(
                    RichText::new(tr!("solar-geostationary-always-above-your-horizon"))
                        .color(theme::GREEN())
                        .size(12.5),
                );
                ui.label(
                    RichText::new(tr!(
                        "solar-point-at",
                        az = format!("{azimuth:.0}"),
                        compass = compass(*azimuth).to_string(),
                        el = format!("{elevation:.0}")
                    ))
                    .color(theme::TEXT()),
                );
            }
            PassSearch::NeverVisible => {
                ui.label(
                    RichText::new(tr!("solar-no-passes-next-48-hours")).color(theme::YELLOW()),
                );
            }
            PassSearch::Passes(passes) => {
                egui::Grid::new("solar-pass-grid").num_columns(6).spacing([14.0, 3.0]).show(
                    ui,
                    |ui| {
                        for h in [
                            tr!("solar-start"),
                            String::from("END"),
                            String::from("DUR"),
                            String::from("AOS"),
                            String::from("LOS"),
                            tr!("solar-max-el"),
                        ] {
                            ui.label(RichText::new(h).color(theme::CYAN_DIM()).size(9.5).strong());
                        }
                        ui.end_row();
                        for p in passes {
                            let soon = p.rise_unix as f64 - sim_now;
                            // The one happening now, or next, is the one the
                            // operator cares about.
                            let color = if soon < 0.0 {
                                theme::GREEN()
                            } else if soon < 3600.0 {
                                theme::YELLOW()
                            } else {
                                theme::TEXT()
                            };
                            ui.label(RichText::new(timefmt::ymd_hm(p.rise_unix)).color(color));
                            ui.label(RichText::new(hhmm(p.set_unix)).color(color));
                            ui.label(
                                RichText::new(format!("{} min", p.duration_s() / 60)).color(color),
                            );
                            ui.label(
                                RichText::new(format!("{:.0}° {}", p.rise_az, compass(p.rise_az)))
                                    .color(color),
                            );
                            ui.label(
                                RichText::new(format!("{:.0}° {}", p.set_az, compass(p.set_az)))
                                    .color(color),
                            );
                            ui.label(
                                RichText::new(format!("{:.0}°  {}", p.max_el, p.quality())).color(
                                    match p.max_el {
                                        e if e >= 30.0 => theme::GREEN(),
                                        e if e >= 15.0 => theme::TEXT(),
                                        _ => theme::CYAN_DIM(),
                                    },
                                ),
                            );
                            ui.end_row();
                        }
                    },
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new(tr!("solar-aos-los-are-azimuths-horizon"))
                        .color(theme::LINE_LIT())
                        .size(10.0),
                );
            }
        }

        // Lock on from right here — the main window activates satellite
        // mode when the root pass drains the request. Browser tab: the
        // relay carries no commands, so the button would be a lie.
        #[cfg(not(target_arch = "wasm32"))]
        {
            ui.add_space(6.0);
            if st.sat_lock == Some(id) {
                if chrome::chip_accent(
                    ui,
                    true,
                    egui::RichText::new(tr!("solar-locked-unlock")).strong(),
                    theme::GREEN(),
                    theme::INK_ON_CYAN(),
                )
                .on_hover_text(tr!("solar-release-satellite-lock"))
                .clicked()
                {
                    st.unlock_requested = true;
                }
            } else if chrome::chip_accent(
                ui,
                false,
                egui::RichText::new(tr!("solar-lock")).strong(),
                theme::GREEN(),
                theme::INK_ON_CYAN(),
            )
            .on_hover_text(tr!("solar-track-satellite-main-window-doppler"))
            .clicked()
            {
                st.lock_requested = Some(id);
            }
        }

        // The operator's own entry wins over the built-in table: they have
        // either corrected it or added a satellite it never knew about.
        let (freqs, mine) = match st.sat_cfg.freqs_for(id) {
            Some(f) => (Some(f), true),
            None => (sdroxide_solar::satfreq::builtin_for(id), false),
        };
        freq_table(ui, freqs, mine, &cache.name);
    });
    if !open {
        st.selected_sat = None;
    }
}

/// The satellite's published frequencies, under the pass table.
///
/// Knowing when a bird comes over is only half of working it; the other half is
/// what to tune to, and that is a table an operator would otherwise have open
/// in a browser next to the radio.
fn freq_table(
    ui: &mut egui::Ui,
    freqs: Option<&sdroxide_solar::SatFreqs>,
    mine: bool,
    tracked_name: &str,
) {
    let Some(freqs) = freqs.filter(|f| f.usable_links().next().is_some()) else {
        ui.add_space(6.0);
        ui.label(
            RichText::new(tr!("solar-no-frequencies-file-one-add"))
                .color(theme::LINE_LIT())
                .size(10.0),
        );
        return;
    };

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(4.0);
    let heading = if mine { tr!("solar-frequencies-yours") } else { tr!("solar-frequencies") };
    ui.label(RichText::new(heading).color(theme::CYAN_DIM()).size(9.5).strong());
    // The published designator can differ from what the element set calls it;
    // showing it means a table entry keyed to the wrong catalogue number is
    // visible rather than quietly presenting the wrong satellite's frequencies.
    if !freqs.name.trim().is_empty() && !freqs.name.eq_ignore_ascii_case(tracked_name) {
        ui.label(
            RichText::new(tr!("solar-published-as", name = freqs.name.to_string()))
                .color(theme::LINE_LIT())
                .size(10.0),
        );
    }
    ui.add_space(2.0);

    egui::Grid::new("solar-freq-grid").num_columns(4).spacing([14.0, 3.0]).show(ui, |ui| {
        for h in [
            tr!("solar-link"),
            tr!("solar-downlink-mhz"),
            tr!("solar-uplink-mhz"),
            tr!("solar-mode"),
        ] {
            ui.label(RichText::new(h).color(theme::CYAN_DIM()).size(9.5).strong());
        }
        ui.end_row();
        for l in freqs.links.iter().filter(|l| !l.is_empty()) {
            let mut label = RichText::new(&l.label).color(theme::TEXT());
            if !l.note.is_empty() {
                label = label.color(theme::TEXT_STRONG());
            }
            let resp = ui.label(label);
            if !l.note.is_empty() {
                resp.on_hover_text(&l.note);
            }
            // The downlink is what gets tuned first, so it leads.
            ui.label(
                RichText::new(l.downlink.map_or_else(|| "—".into(), |b| b.to_string()))
                    .color(theme::GREEN()),
            );
            ui.label(
                RichText::new(l.uplink.map_or_else(|| "—".into(), |b| b.to_string()))
                    .color(theme::YELLOW()),
            );
            ui.label(RichText::new(&l.mode).color(theme::TEXT()));
            ui.end_row();
        }
    });

    // Everything an operator has to know before keying up sits in the notes, so
    // they go on screen rather than only in a tooltip.
    for l in freqs.links.iter().filter(|l| !l.note.is_empty() && !l.is_empty()) {
        ui.label(
            RichText::new(format!("{} — {}", l.label, l.note)).color(theme::LINE_LIT()).size(10.0),
        );
    }
    ui.add_space(2.0);
    ui.label(
        RichText::new(tr!("solar-doppler-shifts-these-few-khz"))
            .color(theme::LINE_LIT())
            .size(10.0),
    );
}

/// `HH:MMZ` — the date is already on the start column. Marked UTC like the
/// start column, so the end is not read as a local time (#635).
fn hhmm(unix: i64) -> String {
    let (_, _, _, h, m, _) = sdroxide_types::utc_ymd_hms(unix);
    format!("{h:02}:{m:02}Z")
}

/// A dot-matrix UTC clock in the top-left corner, with the same instant in the
/// operator's own zone on a smaller second row.
///
/// UTC leads because everything else in the window is UTC: the ephemeris, the
/// DONKI timestamps, the arrival estimates and the FT8 slot boundaries. Local
/// time is the row underneath because "is that pass at a civilised hour?" is a
/// question about the wall clock in the shack, and doing that subtraction in
/// your head is how a pass gets missed.
///
/// Each row carries its own date, so the two zones falling on different days
/// reads at a glance, beside the time it belongs to (#686).
fn clock(ui: &egui::Ui, rect: egui::Rect, sim_now: f64, scrubbed: bool) -> Option<egui::Rect> {
    use super::dotmatrix;

    let hms = |unix: i64| {
        let (_, _, _, h, m, s) = sdroxide_types::utc_ymd_hms(unix);
        format!("{h:02}:{m:02}:{s:02}")
    };
    // ISO order: the matrix has digits and a dash, and no month names.
    let ymd = |unix: i64| {
        let (y, mo, d, _, _, _) = sdroxide_types::utc_ymd_hms(unix);
        format!("{y:04}-{mo:02}-{d:02}")
    };
    let utc = sim_now as i64;
    let text = hms(utc);
    // The scrubbed instant, not the wall clock: two clocks side by side that
    // disagreed about when "now" is would read as one of them being broken.
    let local = utc + crate::time::local_offset_seconds();
    let local_text = hms(local);
    let (date, local_date) = (ymd(utc), ymd(local));

    // Scale with the window, but never so large it competes with the scene —
    // and the panel's own paddings scale with the dots, so the whole box grows
    // and shrinks as one piece.
    let pitch = (rect.width() * 0.004_25).clamp(1.3, 3.5);
    let size = dotmatrix::size(&text, pitch);
    let local_pitch = pitch * 0.58;
    let local_size = dotmatrix::size(&local_text, local_pitch);
    let label_pitch = pitch * 0.42;
    let label = if scrubbed { tr!("solar-sim") } else { String::from("UTC") };
    let label_size = dotmatrix::size(&label, label_pitch);
    let local_label_size = dotmatrix::size("LOC", label_pitch);
    // A floor, so the date still reads on a phone-sized view.
    let date_pitch = (pitch * 0.5).max(1.1);
    let date_size = dotmatrix::size(&date, date_pitch);

    // Both labels share one column to the right of the digits, so UTC and LOC
    // line up however wide the two readouts come out.
    const GAP: f32 = 5.0;
    let label_x = size.x.max(local_size.x) + GAP;
    let date_x = label_x + label_size.x.max(local_label_size.x) + GAP * 2.0;
    let row_gap = 3.5;

    let pad = egui::vec2(6.0, 4.5);
    let panel = egui::Rect::from_min_size(
        rect.left_top() + egui::vec2(MARGIN, MARGIN),
        egui::vec2(date_x + date_size.x, size.y + row_gap + local_size.y) + pad * 2.0,
    );
    if !rect.contains_rect(panel) {
        return None;
    }

    ui.painter().rect_filled(panel, 0, theme::BG_DEEP().gamma_multiply(0.72));
    chrome::paint_cut_border(
        ui.painter(),
        panel,
        if scrubbed { theme::YELLOW() } else { theme::LINE_LIT() },
        egui::Color32::TRANSPARENT,
    );

    // Unlit dots at a low alpha are what make this read as a physical display
    // rather than as text in a blocky face.
    let on = if scrubbed { theme::YELLOW() } else { theme::CYAN() };
    let off = on.gamma_multiply(0.11);
    let origin = panel.min + pad;
    let p = ui.painter();
    // Each label sits on the bottom row of the digits it names.
    dotmatrix::draw(p, origin, &text, pitch, on, off);
    dotmatrix::draw(
        p,
        origin + egui::vec2(label_x, size.y - label_size.y),
        &label,
        label_pitch,
        on.gamma_multiply(0.6),
        egui::Color32::TRANSPARENT,
    );
    dotmatrix::draw(
        p,
        origin + egui::vec2(date_x, size.y - date_size.y),
        &date,
        date_pitch,
        on.gamma_multiply(0.72),
        egui::Color32::TRANSPARENT,
    );
    let local_y = size.y + row_gap;
    dotmatrix::draw(
        p,
        origin + egui::vec2(0.0, local_y),
        &local_text,
        local_pitch,
        on.gamma_multiply(0.72),
        off,
    );
    dotmatrix::draw(
        p,
        origin + egui::vec2(label_x, local_y + local_size.y - local_label_size.y),
        "LOC",
        label_pitch,
        on.gamma_multiply(0.6),
        egui::Color32::TRANSPARENT,
    );
    dotmatrix::draw(
        p,
        origin + egui::vec2(date_x, local_y + local_size.y - date_size.y),
        &local_date,
        date_pitch,
        on.gamma_multiply(0.72),
        egui::Color32::TRANSPARENT,
    );
    Some(panel)
}

/// A `size`-sized box tucked against the right edge of `rect`, with its base on
/// `bottom` and [`MARGIN`] clear of the edge.
fn bottom_right(rect: egui::Rect, bottom: f32, size: egui::Vec2) -> egui::Rect {
    egui::Rect::from_min_size(egui::pos2(rect.right() - MARGIN - size.x, bottom - size.y), size)
}

/// The find box, under the clock.
///
/// Two populations of dots that cannot be found by reading labels, and one box
/// for both. Ninety satellites around the Earth, of which the ones outside the
/// curated set have no label at all until `ALL SATS` is on — at which point
/// there are ninety unlabelled dots. And forty small bodies strung across fifty
/// AU: five dwarf planets, twenty asteroids and fifteen comets, which are
/// deliberately *not* all named at once because that would bury the planets.
///
/// Typing pulls a match out of either crowd with its orbit and its name,
/// whether or not it was being drawn a moment ago. Enter commits: on a
/// satellite that means its pass table, which is what you looked it up for; on
/// a body it means pointing the camera at it, which is the same thing.
///
/// Hidden only when *neither* population is being drawn, because a search that
/// highlights things nothing is drawing would look broken.
fn find_box(
    ui: &egui::Ui,
    st: &mut SolarUi,
    data: Option<&SolarData>,
    rect: egui::Rect,
    clock_rect: Option<egui::Rect>,
) {
    let Some(clock_rect) = clock_rect else { return };
    let (sats, bodies) = (st.layer(layer::SATS), st.layer(layer::PLANETS));
    if !sats && !bodies {
        // Leaving text behind in a hidden box would keep highlighting after the
        // layers came back, with nothing on screen to say why.
        st.search.clear();
        return;
    }

    // Exactly the clock's width, so the two make one column down the left edge
    // and the box stays clear of the menu chips beside them. Anything wider
    // reaches into the chips' column, and on a phone — where the row wraps down
    // the window — that pushed the box below however many rows of chips there
    // were, which is no longer under the clock at all.
    let width = clock_rect.width();
    let area = egui::Rect::from_min_size(
        clock_rect.left_bottom() + egui::vec2(0.0, 6.0),
        egui::vec2(width, 30.0),
    );
    if !rect.contains_rect(area) {
        return;
    }

    // Counted from the same predicates the scene highlights with, so "3 of 94"
    // can never disagree with what is lit up.
    let (sat_hits, sat_total) = match data.filter(|_| sats) {
        Some(d) => d.satellites().fold((0usize, 0usize), |(h, n), sat| {
            (h + st.sat_hit(&sat.name, sat.norad_id) as usize, n + 1)
        }),
        None => (0, 0),
    };
    let body_hits: Vec<usize> =
        if bodies { st.small_hits().map(|(i, _)| i).collect() } else { Vec::new() };
    let body_total = if bodies { sdroxide_solar::smallbody::BODIES.len() } else { 0 };

    let query = st.search.trim().to_string();
    let mut clear = false;
    let mut open: Option<u64> = None;
    let mut go: Option<Focus> = None;
    let hint = match (sats, bodies) {
        (true, true) => tr!("solar-satellite-planet-comet"),
        (true, false) => String::from("satellite"),
        _ => tr!("solar-planet-asteroid-comet"),
    };

    egui::Area::new(egui::Id::new("solar-find"))
        .order(egui::Order::Foreground)
        .fixed_pos(area.min)
        .show(ui.ctx(), |ui| {
            egui::Frame::new()
                .fill(theme::BG_DEEP().gamma_multiply(0.72))
                .inner_margin(egui::Margin::symmetric(8, 5))
                .show(ui, |ui| {
                    ui.set_width(width - 16.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 5.0;
                        // 🔍 and × over ⌕ and ✕: the latter pair is in none of
                        // the bundled faces — not Chakra Petch, not egui's
                        // Ubuntu/Noto Emoji fallbacks — so both drew as tofu.
                        ui.label(RichText::new("🔍").color(theme::CYAN_DIM()).size(12.0));
                        let edit = crate::chrome::field(
                            ui,
                            egui::TextEdit::singleline(&mut st.search)
                                .desired_width(width - 62.0)
                                .hint_text(hint)
                                .text_color(theme::TEXT_STRONG()),
                        );
                        // Enter on a single match commits to it. A body wins a
                        // tie against a satellite only when it is the sole
                        // match overall, so neither can hijack the other's key.
                        if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            match (sat_hits, body_hits.len()) {
                                (1, 0) => {
                                    open = data.and_then(|d| {
                                        d.satellites()
                                            .find(|s| st.sat_hit(&s.name, s.norad_id))
                                            .map(|s| s.norad_id)
                                    })
                                }
                                (0, 1) => go = Some(Focus::Small(body_hits[0])),
                                _ => {}
                            }
                        }
                        if !query.is_empty()
                            && ui.button("×").on_hover_text(tr!("solar-clear-search")).clicked()
                        {
                            clear = true;
                        }
                    });
                    if !query.is_empty() {
                        let (text, colour) = match (sat_hits, body_hits.len()) {
                            (0, 0) => (tr!("solar-no-match").to_string(), theme::ALERT()),
                            // Named outright when there is exactly one, because
                            // "1 of 40" is a worse answer than "Apophis".
                            (0, 1) => (
                                tr!(
                                    "solar-fly-there",
                                    name = sdroxide_solar::smallbody::BODIES[body_hits[0]]
                                        .designation
                                        .to_string()
                                ),
                                theme::YELLOW(),
                            ),
                            (s, 0) => (
                                tr!("solar-sats-tracked", n = s, total = sat_total),
                                theme::YELLOW(),
                            ),
                            (0, b) => (
                                tr!("solar-bodies-found", n = b, total = body_total),
                                theme::YELLOW(),
                            ),
                            (s, b) => (
                                tr!(
                                    "solar-sats-and-bodies",
                                    n = s,
                                    total = sat_total,
                                    bodies = b,
                                    bodies_total = body_total
                                ),
                                theme::YELLOW(),
                            ),
                        };
                        ui.label(RichText::new(text).color(colour).size(10.0));
                    }
                });
        });

    if clear {
        st.search.clear();
    }
    if let Some(id) = open {
        st.selected_sat = Some(id);
    }
    if let Some(f) = go {
        st.set_focus(f);
    }
}

/// The propagation numbers, under the aurora: MUF at the QTH, K and A, the
/// 10.7 cm flux and the current GOES X-ray level.
fn weather_panel(
    ui: &mut egui::Ui,
    st: &SolarUi,
    data: Option<&SolarData>,
    place: Place,
    now: i64,
) -> Option<egui::Rect> {
    let d = data?;
    let w = &d.weather;

    // (label, value, colour). Colours say what the number means for the bands,
    // which is the only reason an operator is reading them.
    let mut rows: Vec<(String, String, egui::Color32)> = Vec::new();

    if let Some((lat, lon)) = st.qth {
        match sdroxide_solar::indices::estimate_muf(&w.ionosondes, lat, lon, now) {
            Some(m) => rows.push((
                "MUF".into(),
                format!("{:.1} MHz", m.muf_mhz),
                match m.muf_mhz {
                    f if f >= 24.0 => theme::GREEN(),
                    f if f >= 14.0 => theme::CYAN(),
                    _ => theme::YELLOW(),
                },
            )),
            None => rows.push(("MUF".into(), tr!("solar-no-sounder"), theme::LINE_LIT())),
        }
        // What this station has actually heard get through, near the QTH.
        //
        // A floor, and labelled as one twice over — `HEARD ≥` and a `≥` on the
        // value — because it is a different kind of statement from the row
        // above it. The sounder measures and interpolates; this observes and
        // bounds. Neither replaces the other: the sounder has dreadful spatial
        // coverage and this has none at all over an ocean nobody works, and
        // where they disagree the disagreement is the useful part.
        if let Some(m) = st.prop.muf_at(lat, lon) {
            rows.push((
                tr!("solar-heard"),
                format!("≥ {:.1} MHz", m.floor_mhz),
                match m.floor_mhz {
                    f if f >= 24.0 => theme::GREEN(),
                    f if f >= 14.0 => theme::CYAN(),
                    _ => theme::YELLOW(),
                },
            ));
        }
    }
    if let Some(g) = &w.geomagnetic {
        let color = match g.kp {
            k if k >= 5.0 => theme::PINK(),
            k if k >= 4.0 => theme::YELLOW(),
            _ => theme::GREEN(),
        };
        rows.push((tr!("solar-kp"), format!("{:.1} / {:.0}", g.kp, g.a_index), color));
    }
    if let Some(f) = &w.flux {
        rows.push((
            "F10.7".into(),
            format!("{:.0} sfu", f.sfu),
            match f.sfu {
                v if v >= 150.0 => theme::GREEN(),
                v if v >= 90.0 => theme::CYAN(),
                _ => theme::YELLOW(),
            },
        ));
    }
    if let Some(x) = &w.xray {
        rows.push((
            tr!("solar-x-ray"),
            x.class.clone(),
            if x.causes_hf_absorption() { theme::PINK() } else { theme::CYAN_DIM() },
        ));
    }
    if rows.is_empty() {
        return None;
    }

    let font = egui::FontId::proportional(12.0);
    let small = egui::FontId::proportional(10.0);
    // Cloned so the box can be laid out before the space for it is reserved:
    // `Place::Inline` needs the `Ui` back to allocate from.
    let p = ui.painter().clone();
    let laid: Vec<_> = rows
        .iter()
        .map(|(k, v, c)| {
            (
                p.layout_no_wrap(k.clone(), small.clone(), theme::CYAN_DIM()),
                p.layout_no_wrap(v.clone(), font.clone(), *c),
            )
        })
        .collect();
    let key_w = laid.iter().map(|(k, _)| k.size().x).fold(0.0f32, f32::max);
    let val_w = laid.iter().map(|(_, v)| v.size().x).fold(0.0f32, f32::max);
    let row_h = laid.iter().map(|(_, v)| v.size().y + 3.0).fold(0.0f32, f32::max);

    // A one-line caveat under the MUF: it is interpolated from ionosondes that
    // may be a long way off, and saying so costs one line.
    let sounder = st
        .qth
        .and_then(|(lat, lon)| sdroxide_solar::indices::estimate_muf(&w.ionosondes, lat, lon, now));
    let observed = st.qth.and_then(|(lat, lon)| st.prop.muf_at(lat, lon));
    let mut notes: Vec<egui::text::Galley> = Vec::new();
    if let Some(m) = &sounder {
        notes.push(
            (*p.layout_no_wrap(
                format!("{} · {:.0} km", m.confidence(), m.nearest_km),
                small.clone(),
                theme::LINE_LIT(),
            ))
            .clone(),
        );
    }
    if let Some(o) = &observed {
        notes.push(
            (*p.layout_no_wrap(
                tr!(
                    "solar-confidence-on-band",
                    confidence = o.confidence().to_string(),
                    band = o.band.label()
                ),
                small.clone(),
                theme::LINE_LIT(),
            ))
            .clone(),
        );
        // The one comparison worth more than either number on its own. The
        // sounder is a model of a place it is not standing in; a path that got
        // through is a fact. When the fact is above the model, the band is
        // better than the model thinks — which is the most actionable thing
        // this panel can say.
        if sounder.as_ref().is_some_and(|m| o.floor_mhz > m.muf_mhz + 1.0) {
            notes.push(
                (*p.layout_no_wrap(
                    tr!("solar-observed-above-sounder-better-than"),
                    small.clone(),
                    theme::GREEN(),
                ))
                .clone(),
            );
        }
    }

    let pad = 10.0;
    let width = key_w + val_w + 18.0 + pad * 2.0;
    let width = notes.iter().fold(width, |acc, n| acc.max(n.size().x + pad * 2.0));
    let height =
        rows.len() as f32 * row_h + notes.iter().map(|n| n.size().y + 4.0).sum::<f32>() + pad * 2.0;
    let panel = place.reserve(ui, egui::vec2(width, height))?;

    p.rect_filled(panel, 0, theme::FILL().gamma_multiply(0.82));
    chrome::paint_cut_border(&p, panel, theme::LINE_LIT(), egui::Color32::TRANSPARENT);
    let mut y = panel.top() + pad;
    for ((key, val), (_, _, color)) in laid.iter().zip(&rows) {
        p.galley(egui::pos2(panel.left() + pad, y + 2.0), key.clone(), theme::CYAN_DIM());
        p.galley(egui::pos2(panel.right() - pad - val.size().x, y), val.clone(), *color);
        y += row_h;
    }
    for n in notes {
        let h = n.size().y + 4.0;
        p.galley(
            egui::pos2(panel.left() + pad, y + 2.0),
            std::sync::Arc::new(n),
            theme::LINE_LIT(),
        );
        y += h;
    }
    Some(panel)
}

/// Aurora: how much power is going into each oval, how far towards the equator
/// it reaches, whether it is over your head, and what the planetary K series
/// did over the last day and is forecast to do over the next.
///
/// The colours here mean the same thing they do everywhere else in the window —
/// green quiet, yellow worth watching, pink a storm.
///
/// Returns the box it drew, or `None` if it drew nothing — the propagation
/// numbers stack under it either way.
fn aurora_panel(
    ui: &mut egui::Ui,
    st: &SolarUi,
    data: Option<&SolarData>,
    place: Place,
    now: i64,
) -> Option<egui::Rect> {
    use sdroxide_solar::{HemisphericPower, aurora};

    let d = data?;
    // Nothing to say until one of the three aurora feeds has landed. Drawing an
    // empty box would imply the aurora had been measured and found absent.
    if d.aurora.is_none() && d.aurora_power.is_none() && d.kp_forecast.is_empty() {
        return None;
    }
    let kp_color = |kp: f64| match kp {
        k if k >= 5.0 => theme::PINK(),
        k if k >= 4.0 => theme::YELLOW(),
        _ => theme::GREEN(),
    };

    // Both hemispheres are reported throughout: without a QTH neither is more
    // relevant than the other, and with one the QTH row already says what
    // matters locally.
    let mut rows: Vec<(String, String, egui::Color32)> = Vec::new();
    if let Some(power) = &d.aurora_power {
        let worst = power.north_gw.max(power.south_gw);
        let color = match HemisphericPower::index(worst) {
            8..=10 => theme::PINK(),
            6..=7 => theme::YELLOW(),
            _ => theme::GREEN(),
        };
        rows.push((
            tr!("solar-power-n-s"),
            format!("{:.0} / {:.0} GW", power.north_gw, power.south_gw),
            color,
        ));
        rows.push((
            "activity".into(),
            format!("{} · HPI {}", HemisphericPower::words(worst), HemisphericPower::index(worst)),
            color,
        ));
    }
    if let Some(oval) = &d.aurora {
        // The edge is where it stops being worth looking, so it is the number
        // an operator compares their own latitude against.
        let edge = |n: bool| {
            oval.equatorward_edge(n, aurora::EDGE_PCT)
                .map(|lat| format!("{:.0}°{}", lat.abs(), if n { "N" } else { "S" }))
        };
        // An oval too weak to reach the contour anywhere in one hemisphere is a
        // real state, and one worth reading as a dash rather than as a row that
        // silently disappeared.
        let (n, s) = (edge(true), edge(false));
        if n.is_some() || s.is_some() {
            let show = |e: Option<String>| e.unwrap_or_else(|| "—".into());
            rows.push((tr!("solar-edge-n-s"), format!("{} / {}", show(n), show(s)), theme::CYAN()));
        }
        if let Some((lat, lon)) = st.qth {
            let pct = oval.probability(lat, lon);
            let color = match pct {
                p if p >= 25.0 => theme::PINK(),
                p if p >= aurora::EDGE_PCT => theme::YELLOW(),
                p if p >= aurora::NOISE_FLOOR_PCT => theme::GREEN(),
                _ => theme::LINE_LIT(),
            };
            rows.push((st.qth_grid.clone(), format!("{pct:.0} %"), color));
        }
    }
    // The forecast half: the worst three-hour bin still ahead of us, and how
    // far away it is.
    let peak = aurora::peak_forecast(&d.kp_forecast, now, 24 * 3600);
    if let Some(p) = peak {
        let in_h = (p.unix - now).max(0) as f64 / 3600.0;
        rows.push((
            tr!("solar-kp-peak-24-h"),
            if in_h < 1.5 {
                tr!("solar-kp-now", kp = format!("{:.1}", p.kp))
            } else {
                tr!("solar-kp-in", kp = format!("{:.1}", p.kp), hours = format!("{in_h:.0}"))
            },
            kp_color(p.kp),
        ));
        rows.push((
            "viewline".into(),
            format!("{:.0}° geomag", aurora::viewline_geomagnetic_lat(p.kp)),
            kp_color(p.kp),
        ));
    }
    if rows.is_empty() {
        return None;
    }

    // The forecast strip: one bar per three-hour bin over the next day. Eight
    // numbers in a column is a table nobody reads; the same eight as a shape is
    // "it picks up after midnight" at a glance.
    // The trend: the observed bins behind `now` and the forecast ahead of it in
    // one row, so the shape of the last day flows into the shape of the next.
    // The forecast bars are filled lighter — a measurement and a prediction
    // must never be read as the same thing.
    let mut series: Vec<&sdroxide_solar::KpPoint> =
        aurora::recent(&d.kp_forecast, now, 24 * 3600).collect();
    let history_len = series.len();
    series.extend(aurora::upcoming(&d.kp_forecast, now, 24 * 3600).take(8));
    const BAR_W: f32 = 9.0;
    const BAR_GAP: f32 = 2.0;
    const STRIP_H: f32 = 26.0;

    let font = egui::FontId::proportional(12.0);
    let small = egui::FontId::proportional(10.0);
    // Cloned for the reason the propagation box clones it: laid out first, and
    // only then handed a place to sit.
    let p = ui.painter().clone();
    let laid: Vec<_> = rows
        .iter()
        .map(|(k, v, c)| {
            (
                p.layout_no_wrap(k.clone(), small.clone(), theme::CYAN_DIM()),
                p.layout_no_wrap(v.clone(), font.clone(), *c),
            )
        })
        .collect();
    let key_w = laid.iter().map(|(k, _)| k.size().x).fold(0.0f32, f32::max);
    let val_w = laid.iter().map(|(_, v)| v.size().x).fold(0.0f32, f32::max);
    let row_h = laid.iter().map(|(_, v)| v.size().y + 3.0).fold(0.0f32, f32::max);

    // What the picture is valid for, never what time it is now: OVATION is a
    // forecast for about forty minutes ahead and the grid may be an hour old.
    let footer = d.aurora.as_ref().map(|o| {
        let age = (now - o.observed_unix).max(0);
        p.layout_no_wrap(
            tr!(
                "solar-valid-age",
                valid = timefmt::ymd_hm(o.forecast_unix),
                age = timefmt::age(age)
            ),
            small.clone(),
            theme::LINE_LIT(),
        )
    });

    let title = p.layout_no_wrap(tr!("solar-aurora"), small.clone(), theme::CYAN_DIM());
    let pad = 10.0;
    let strip_w =
        if series.is_empty() { 0.0 } else { series.len() as f32 * (BAR_W + BAR_GAP) - BAR_GAP };
    let width =
        (key_w + val_w + 18.0).max(strip_w).max(footer.as_ref().map_or(0.0, |f| f.size().x))
            + pad * 2.0;
    let height = title.size().y
        + 5.0
        + rows.len() as f32 * row_h
        // Strip: the gap above it, the bars, and the hour stamps under them.
        + if series.is_empty() { 0.0 } else { STRIP_H + 18.0 }
        + footer.as_ref().map_or(0.0, |f| f.size().y + 4.0)
        + pad * 2.0;

    let panel = place.reserve(ui, egui::vec2(width, height))?;

    p.rect_filled(panel, 0, theme::FILL().gamma_multiply(0.82));
    chrome::paint_cut_border(&p, panel, theme::LINE_LIT(), egui::Color32::TRANSPARENT);

    let mut y = panel.top() + pad;
    let title_h = title.size().y;
    p.galley(egui::pos2(panel.left() + pad, y), title, theme::CYAN_DIM());
    y += title_h + 5.0;
    for ((key, val), (_, _, color)) in laid.iter().zip(&rows) {
        p.galley(egui::pos2(panel.left() + pad, y + 2.0), key.clone(), theme::CYAN_DIM());
        p.galley(egui::pos2(panel.right() - pad - val.size().x, y), val.clone(), *color);
        y += row_h;
    }

    if !series.is_empty() {
        y += 6.0;
        let base = y + STRIP_H;
        for (i, bin) in series.iter().enumerate() {
            let x = panel.left() + pad + i as f32 * (BAR_W + BAR_GAP);
            let h = (bin.kp / 9.0) as f32 * STRIP_H;
            // An unlit socket under every bar, so a quiet stretch still reads
            // as a scale rather than as missing data.
            p.rect_filled(
                egui::Rect::from_min_max(egui::pos2(x, y), egui::pos2(x + BAR_W, base)),
                0,
                theme::LINE().gamma_multiply(0.55),
            );
            // The measured half is solid, the predicted half a wash: the two
            // must not be read as one continuous observation.
            let fill = if bin.predicted {
                kp_color(bin.kp).gamma_multiply(0.45)
            } else {
                kp_color(bin.kp)
            };
            p.rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(x, base - h.max(1.0)),
                    egui::pos2(x + BAR_W, base),
                ),
                0,
                fill,
            );
        }
        // Where measurement gives way to prediction. The first forecast bar may
        // also be the bin containing `now`, so the line lands where the two
        // halves meet rather than on a clock face.
        if history_len > 0 && history_len < series.len() {
            let x = panel.left() + pad + history_len as f32 * (BAR_W + BAR_GAP) - BAR_GAP / 2.0;
            p.line_segment(
                [egui::pos2(x, y - 2.0), egui::pos2(x, base)],
                egui::Stroke::new(1.0, theme::LINE_LIT()),
            );
        }
        // Hours under the ends, so the strip has a time axis without sixteen
        // labels fighting for room.
        let stamp = |unix: i64| {
            let (_, _, _, h, _, _) = sdroxide_types::utc_ymd_hms(unix);
            format!("{h:02}z")
        };
        p.text(
            egui::pos2(panel.left() + pad, base + 2.0),
            egui::Align2::LEFT_TOP,
            stamp(series[0].unix),
            small.clone(),
            theme::LINE_LIT(),
        );
        p.text(
            egui::pos2(panel.left() + pad + strip_w, base + 2.0),
            egui::Align2::RIGHT_TOP,
            stamp(series[series.len() - 1].unix),
            small.clone(),
            theme::LINE_LIT(),
        );
        // What the two halves are, once, under the axis — only when both are
        // actually present.
        if history_len > 0 && history_len < series.len() {
            p.text(
                egui::pos2(panel.left() + pad + strip_w / 2.0, base + 2.0),
                egui::Align2::CENTER_TOP,
                tr!("solar-observed-forecast"),
                small.clone(),
                theme::LINE_LIT(),
            );
        }
        y = base + 12.0;
    }

    if let Some(f) = footer {
        p.galley(egui::pos2(panel.left() + pad, y + 2.0), f, theme::LINE_LIT());
    }
    Some(panel)
}

/// How open each band is, as one bar per band.
///
/// The same question the heat map answers geographically, reduced to a number
/// per band so it can be read without turning the globe: **how much of the
/// world is this band getting through to right now**. A bar chart because the
/// answer is a comparison — an operator picking a band wants the shape of the
/// spectrum, not five figures to subtract from each other — and in band order
/// rather than ranked, so that shape is the familiar one and the bars do not
/// swap places every time a decode lands.
///
/// What is measured is [`BandPlane::reach`]: the share of the world with
/// evidence in it, area-weighted. Deliberately not a path count. Forty contacts
/// into one corner of Europe are one direction open, and a count would call
/// that the best band of the evening.
///
/// It inherits the propagation field's memory exactly, because it is read off
/// that field rather than accumulated beside it: whatever the half-life is set
/// to is how long a band stays on this chart after it shuts.
fn bands_panel(ui: &mut egui::Ui, st: &SolarUi, place: Place) -> Option<egui::Rect> {
    use sdroxide_types::Band;

    // Only bands with something in them. A chart of dead bands is a chart of
    // where this station has not been listening, which is not what it claims
    // to be — see the footer, which says so out loud.
    let rows: Vec<(Band, f32)> = st
        .prop
        .live_bands()
        .into_iter()
        .filter_map(|b| st.prop.plane(b).map(|p| (b, p.reach())))
        .filter(|(_, reach)| *reach > 0.0)
        .collect();
    if rows.is_empty() {
        return None;
    }

    // Autoranged in decades rather than normalised to the best band, and the
    // top of the scale is printed underneath. Normalising would draw the same
    // full bar for a band working a quarter of the planet and for one that has
    // heard a single neighbour, and the chart would silently change meaning
    // every time the leader changed.
    const SCALE_STEPS: [f32; 5] = [0.01, 0.03, 0.10, 0.30, 1.00];
    let best = rows.iter().map(|(_, r)| *r).fold(0.0f32, f32::max);
    let scale = SCALE_STEPS.iter().copied().find(|s| best <= *s).unwrap_or(1.0);
    // Two significant figures wherever the value happens to sit: a band down to
    // its last cell is worth a hundredth of a per cent, and rounding that to
    // "0.0 %" would print a band that is open as a band that is not.
    let pct = |v: f32| {
        let v = v * 100.0;
        match v {
            v if v >= 10.0 => format!("{v:.0} %"),
            v if v >= 1.0 => format!("{v:.1} %"),
            v => format!("{v:.2} %"),
        }
    };

    // The narrowest the bars are allowed to be before the box grows for them.
    const MIN_BAR_W: f32 = 74.0;
    const BAR_H: f32 = 7.0;
    let font = egui::FontId::proportional(11.5);
    let small = egui::FontId::proportional(10.0);
    // Laid out before a place for it is reserved, for the reason the two panels
    // above it do the same.
    let p = ui.painter().clone();
    let laid: Vec<_> = rows
        .iter()
        .map(|(band, reach)| {
            let c = crate::colormap::band_color(*band);
            let hue = egui::Color32::from_rgb(c[0], c[1], c[2]);
            (
                p.layout_no_wrap(band.label().into(), small.clone(), hue),
                p.layout_no_wrap(pct(*reach), font.clone(), theme::CYAN()),
                hue,
                *reach,
            )
        })
        .collect();
    let key_w = laid.iter().map(|(k, ..)| k.size().x).fold(0.0f32, f32::max);
    let val_w = laid.iter().map(|(_, v, ..)| v.size().x).fold(0.0f32, f32::max);
    let row_h = laid.iter().map(|(_, v, ..)| v.size().y + 3.0).fold(BAR_H + 5.0, f32::max);

    let title = p.layout_no_wrap(tr!("solar-bands-open"), small.clone(), theme::CYAN_DIM());
    let notes = [
        tr!("solar-full-bar", share = pct(scale)),
        tr!("solar-own-paths-memory", minutes = format!("{:.0}", st.prop.halflife_s / 60.0)),
    ]
    .map(|t| p.layout_no_wrap(t, small.clone(), theme::LINE_LIT()));

    let pad = 10.0;
    // The footer is the longest line in the box and always has been, so rather
    // than leave the chart short in a box sized by a sentence, the bars take
    // whatever width that sentence bought.
    let gap = 8.0;
    let width = (key_w + MIN_BAR_W + val_w + gap * 2.0 + 4.0)
        .max(title.size().x)
        .max(notes.iter().map(|n| n.size().x).fold(0.0f32, f32::max))
        + pad * 2.0;
    let height = title.size().y
        + 5.0
        + rows.len() as f32 * row_h
        + notes.iter().map(|n| n.size().y + 3.0).sum::<f32>()
        + 3.0
        + pad * 2.0;
    let panel = place.reserve(ui, egui::vec2(width, height))?;

    p.rect_filled(panel, 0, theme::FILL().gamma_multiply(0.82));
    chrome::paint_cut_border(&p, panel, theme::LINE_LIT(), egui::Color32::TRANSPARENT);

    let mut y = panel.top() + pad;
    let title_h = title.size().y;
    p.galley(egui::pos2(panel.left() + pad, y), title, theme::CYAN_DIM());
    y += title_h + 5.0;
    let bar_x = panel.left() + pad + key_w + gap;
    let bar_w = (panel.right() - pad - val_w - gap - bar_x).max(MIN_BAR_W);
    for (key, val, hue, reach) in laid {
        let key_y = y + (row_h - key.size().y) * 0.5;
        p.galley(egui::pos2(panel.left() + pad, key_y), key, hue);
        // An unlit socket behind every bar, the same as the aurora forecast
        // strip: without it a quiet band reads as missing rather than as quiet.
        let top = y + (row_h - BAR_H) * 0.5;
        p.rect_filled(
            egui::Rect::from_min_size(egui::pos2(bar_x, top), egui::vec2(bar_w, BAR_H)),
            0,
            theme::LINE().gamma_multiply(0.55),
        );
        // The band's own hue: what the combined view paints it in on the globe,
        // and what the legend names it by. The chart and the map share one key,
        // so neither has to be learned twice.
        let w = (reach / scale).clamp(0.0, 1.0) * bar_w;
        p.rect_filled(
            egui::Rect::from_min_size(egui::pos2(bar_x, top), egui::vec2(w.max(1.0), BAR_H)),
            0,
            hue,
        );
        p.galley(
            egui::pos2(panel.right() - pad - val.size().x, y + (row_h - val.size().y) * 0.5),
            val,
            theme::CYAN(),
        );
        y += row_h;
    }
    y += 3.0;
    for n in notes {
        let h = n.size().y + 3.0;
        p.galley(egui::pos2(panel.left() + pad, y), n, theme::LINE_LIT());
        y += h;
    }
    Some(panel)
}

/// The award layer's key, bottom right: what each colour on the globe means,
/// and how many entities are in each state.
///
/// A heat map with no legend is decoration. This one is small, and it only
/// exists while the layer it explains is switched on. `bottom` is what the date
/// readout in the same corner left of it.
fn award_panel(ui: &egui::Ui, st: &SolarUi, rect: egui::Rect, bottom: f32) {
    if !st.layer(layer::AWARDS) || st.awards.is_empty() {
        return;
    }
    let (missing, worked, confirmed) = sdroxide_types::coverage_counts(&st.awards);
    let rows = [
        ("missing", missing, egui::Color32::from_rgb(0xff, 0x5a, 0x28)),
        ("worked", worked, theme::YELLOW()),
        ("confirmed", confirmed, theme::GREEN()),
    ];

    let p = ui.painter();
    let font = egui::FontId::proportional(11.5);
    let cap = egui::FontId::proportional(9.5);
    let galleys: Vec<_> = rows
        .iter()
        .map(|(label, n, _)| p.layout_no_wrap(format!("{label}  {n}"), font.clone(), theme::TEXT()))
        .collect();
    let title = p.layout_no_wrap(tr!("solar-dxcc-coverage"), cap, theme::CYAN_DIM());

    const SWATCH: f32 = 9.0;
    let w = galleys.iter().map(|g| g.size().x).fold(title.size().x, f32::max) + SWATCH + 26.0;
    let h = galleys.iter().map(|g| g.size().y + 3.0).sum::<f32>() + title.size().y + 18.0;
    let panel = bottom_right(rect, bottom, egui::vec2(w, h));
    if !rect.contains_rect(panel) {
        return; // too small a window to be worth crowding
    }
    p.rect_filled(panel, 0, theme::FILL().gamma_multiply(0.82));
    chrome::paint_cut_border(p, panel, theme::LINE_LIT(), egui::Color32::TRANSPARENT);

    let mut y = panel.top() + 7.0;
    let x = panel.left() + 10.0;
    let title_h = title.size().y;
    p.galley(egui::pos2(x, y), title, theme::CYAN_DIM());
    y += title_h + 4.0;
    for (g, (_, _, color)) in galleys.into_iter().zip(rows) {
        let dy = g.size().y + 3.0;
        p.circle_filled(egui::pos2(x + SWATCH * 0.5, y + g.size().y * 0.5), SWATCH * 0.42, color);
        p.galley(egui::pos2(x + SWATCH + 8.0, y), g, theme::TEXT());
        y += dy;
    }
}

/// The banner that justifies the whole window: a CME whose cone contains the
/// Earth, with an arrival estimate.
/// One line saying what the cloud deck is, and what about it is not measured.
///
/// The hour it shows is the hour the mosaic is *of*, which is over an hour
/// behind the clock — the same discipline the aurora footer keeps, and for the
/// same reason: a picture presented as current when it is not is worse than no
/// picture. And it says the lightning is simulated, which is not optional. The
/// storms are real and come out of the imagery; the individual flashes are
/// invented, and a globe that flickers with plausible-looking strikes has to
/// admit that rather than let anyone read them as strikes.
fn clouds_note(ui: &egui::Ui, st: &SolarUi, data: Option<&SolarData>, rect: egui::Rect, now: i64) {
    if !st.layer(layer::CLOUDS) {
        return;
    }
    let Some(field) = data.and_then(|d| d.clouds.as_ref()) else { return };

    let channels = if field.has_visible { tr!("solar-ir-vis") } else { tr!("solar-ir-only") };
    let storms = match field.cells.len() {
        0 => tr!("solar-no-deep-convection").to_string(),
        1 => tr!("solar-t-1-storm").to_string(),
        n => tr!("solar-n-storms", n = n),
    };
    let text = tr!(
        "solar-clouds-legend",
        time = timefmt::ymd_hm(field.frame_unix),
        age = timefmt::age((now - field.frame_unix).max(0)),
        channels = channels.to_string(),
        storms = storms.to_string()
    );

    let p = ui.painter();
    let galley = p.layout_no_wrap(text, egui::FontId::proportional(10.5), theme::scope().line);
    if galley.size().x > rect.width() - 40.0 {
        return;
    }
    p.galley(
        egui::pos2(rect.left() + 14.0, rect.bottom() - galley.size().y - 8.0),
        galley,
        theme::scope().line,
    );
}

fn impact_banner(ui: &egui::Ui, data: Option<&SolarData>, rect: egui::Rect, now: i64) {
    let Some(d) = data else { return };
    // The soonest arrival that has not already happened.
    let mut best: Option<(&sdroxide_solar::CmeEvent, sdroxide_solar::Impact)> = None;
    for e in &d.cmes {
        let Some(a) = &e.analysis else { continue };
        let Some(hit) = sdroxide_solar::earth_impact(a) else { continue };
        // Keep it on screen for a few hours past the estimate, since arrival
        // estimates are routinely off by that much.
        if hit.eta_unix < now - 6 * 3600 {
            continue;
        }
        if best.as_ref().is_none_or(|(_, b)| hit.eta_unix < b.eta_unix) {
            best = Some((e, hit));
        }
    }
    let Some((event, hit)) = best else { return };
    let a = event.analysis.as_ref().expect("filtered above");

    let hours = (hit.eta_unix - now) as f64 / 3600.0;
    let when = if hours >= 0.0 {
        tr!("solar-cme-eta", eta = timefmt::ymd_hm(hit.eta_unix), hours = format!("{hours:.0}"))
    } else {
        tr!(
            "solar-cme-arrived",
            eta = timefmt::ymd_hm(hit.eta_unix),
            hours = format!("{:.0}", -hours)
        )
    };
    let glancing = if hit.directness(a.half_angle_deg) < 0.35 {
        tr!("solar-glancing")
    } else {
        String::from("")
    };
    let estimated = if a.estimated { tr!("solar-direction-estimated") } else { String::from("") };
    let text = tr!(
        "solar-cme-earth-directed",
        time = timefmt::ymd_hm(a.t21_5_unix),
        speed = format!("{:.0}", a.speed_km_s),
        when = when.to_string(),
        glancing = glancing,
        estimated = estimated
    );

    // Hazard-striped tabs at both ends, the same mark the manual uses on every
    // section header. A CME arrival is the one thing in this window that wants
    // to be noticed without being read first.
    const TAB_W: f32 = 22.0;
    let font = egui::FontId::proportional(13.0);
    let galley = ui.painter().layout_no_wrap(text, font, theme::TEXT_STRONG());
    let size = galley.size() + egui::vec2(28.0 + TAB_W * 2.0, 15.0);
    if size.x > rect.width() - 24.0 {
        return;
    }
    let banner = egui::Rect::from_center_size(
        egui::pos2(rect.center().x, rect.bottom() - size.y * 0.5 - 14.0),
        size,
    );
    let p = ui.painter();
    p.rect_filled(banner, 0, theme::CQ_BG().gamma_multiply(0.94));
    for tab in [
        egui::Rect::from_min_size(banner.min, egui::vec2(TAB_W, banner.height())),
        egui::Rect::from_min_size(
            egui::pos2(banner.right() - TAB_W, banner.top()),
            egui::vec2(TAB_W, banner.height()),
        ),
    ] {
        chrome::hazard_stripes(p, tab, 7.0);
    }
    chrome::paint_cut_border(p, banner, theme::PINK(), egui::Color32::TRANSPARENT);
    p.galley(
        egui::pos2(banner.left() + TAB_W + 14.0, banner.top() + 7.0),
        galley,
        theme::TEXT_STRONG(),
    );
}

/// Fly the AUTO tour, using real elapsed time so the pacing is frame-rate
/// independent.
///
/// `data` is the frame's snapshot, passed in because the caller already holds
/// the [`SolarData`] guard for the whole pass — taking the mutex again here
/// would be this thread waiting on itself.
fn advance_tour(ui: &egui::Ui, st: &mut SolarUi, data: Option<&SolarData>, sim_now: f64, dt: f32) {
    if !st.view.auto {
        // Hand the pivot back to the focus chips.
        st.focus_override = None;
        return;
    }
    let b = super::scene::bodies(st, sim_now);
    // The contact being worked pre-empts the tour — but only while its arc is
    // actually on screen: with the layer switched off there is nothing there
    // for the camera to fly to.
    let qso = st
        .layer(crate::view::solar_layer::QSO)
        .then(|| st.qth.zip(st.digi.dx))
        .flatten()
        .map(|(home, dx)| super::camera::QsoPath { home, dx });
    // The satellite lock outranks the contact, under the same layer rule: the
    // camera only chases what is actually drawn.
    let sat = st
        .layer(crate::view::solar_layer::SATS)
        .then(|| st.sat_lock.zip(st.qth))
        .flatten()
        .and_then(|(id, home)| {
            let state = data?.satellites().find(|s| s.norad_id == id)?.at(sim_now)?;
            Some(super::camera::SatPath {
                home,
                sat_world: b.earth
                    + super::math::V3::from_f64(state.dir_ecliptic)
                        * (b.earth_r * state.radii as f32),
            })
        });
    // `Tour` is `Copy`, so step a local and write it back rather than fighting
    // the borrow of `st.view` inside it.
    let mut tour = st.tour;
    let pivot = tour.step(&mut st.view, &b, dt, qso, sat, st.qth);
    st.tour = tour;
    st.focus_override = Some(pivot);
    crate::repaint::animate(ui.ctx());
}

/// Drag to rotate, scroll or pinch to zoom, double-click to reframe. Any of them
/// cancels the animated tour — the user taking the controls is the signal to
/// stop.
fn interact(ui: &egui::Ui, st: &mut SolarUi, resp: &egui::Response, sim_now: f64) {
    let mut touched = false;

    // The camera never draws closer than the focus body's floor, but the tour
    // can leave the stored distance far inside it. A zoom then only moved that
    // hidden value, and the wheel seemed dead until a focus chip reframed
    // (#702). Pulled up to the floor before a zoom applies.
    let to_floor = |st: &mut SolarUi| {
        let (_, r) = super::scene::bodies(st, sim_now).focus(st.focus());
        st.view.dist = st.view.dist.max(super::camera::dist_range(r).0);
    };

    // Two fingers on the scene zoom it: on a screen with no wheel there is no
    // other way to change the camera distance.
    //
    // First in the chain, and it swallows the drag, for the reason the
    // panadapter's pinch does: the browser keeps reporting a one-finger drag
    // from the *first* finger down for the whole gesture, so without this a
    // pinch would spin the camera at the same time as it zoomed.
    let pinch = ui.input(|i| i.multi_touch()).filter(|mt| resp.rect.contains(mt.center_pos));
    if let Some(mt) = pinch {
        if mt.zoom_delta > 0.0 && mt.zoom_delta != 1.0 {
            to_floor(st);
            // Fingers apart is zoom *in*, i.e. a shorter distance to the focus.
            // Multiplicative like the wheel, so a gesture covers the same visual
            // fraction whether you are 3 Gm or 3 AU out.
            st.view.dist /= mt.zoom_delta;
            touched |= mt.zoom_delta != 1.0;
        }
    } else if resp.dragged_by(egui::PointerButton::Primary) {
        let d = resp.drag_delta();
        st.view.yaw -= d.x * 0.006;
        st.view.pitch = (st.view.pitch + d.y * 0.006)
            .clamp(-super::camera::PITCH_LIMIT, super::camera::PITCH_LIMIT);
        touched |= d != egui::Vec2::ZERO;
    }

    if resp.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            to_floor(st);
            // Multiplicative, so a wheel click covers the same visual fraction
            // whether you are 3 Gm or 3 AU out.
            st.view.dist *= (1.0 - scroll * 0.0022).clamp(0.4, 2.5);
            touched = true;
        }
    }

    // Clamping needs the focus radius, which only the scene knows; `Camera`
    // re-clamps anyway, so here just keep the stored value sane.
    st.view.dist = st.view.dist.clamp(1e-5, super::camera::MAX_DIST);

    if touched {
        st.view.auto = false;
    }
    // Continuous repaint only while something is actually moving; otherwise the
    // window idles and is woken by input or by the data feed.
    if touched || st.view.auto || resp.is_pointer_button_down_on() {
        crate::repaint::animate(ui.ctx());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bottom-right box hangs off the corner, [`MARGIN`] clear of both edges.
    #[test]
    fn a_bottom_right_box_sits_in_the_corner() {
        let view = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
        let award = bottom_right(view, view.bottom() - MARGIN, egui::vec2(140.0, 60.0));
        assert_eq!(award.right(), 788.0);
        assert_eq!(award.bottom(), 588.0);
    }

    /// Half a second a line, whatever the line says — which is the whole of the
    /// timing the operator asked for.
    #[test]
    fn the_contact_card_types_one_line_every_half_second() {
        // Nothing at all before the card opens.
        assert_eq!(typed_chars(0.0, 0, 10), None);
        // The first line starts with a character rather than a bare cursor.
        assert_eq!(typed_chars(0.001, 0, 10), Some(1));
        assert_eq!(typed_chars(0.25, 0, 10), Some(5));
        assert_eq!(typed_chars(0.5, 0, 10), Some(10));

        // The second line has not begun while the first is still going, and is
        // finished half a second after it starts.
        assert_eq!(typed_chars(0.4, 1, 10), None);
        assert_eq!(typed_chars(0.75, 1, 10), Some(5));
        assert_eq!(typed_chars(1.0, 1, 10), Some(10));

        // Length does not change the beat: a long line and a short one both
        // finish on their own half-second.
        assert_eq!(typed_chars(1.5, 2, 40), Some(40));
        assert_eq!(typed_chars(1.5, 2, 3), Some(3));
        // …and neither ever overruns its own length.
        assert_eq!(typed_chars(99.0, 0, 7), Some(7));

        // A nine-line card is fully typed in four and a half seconds.
        assert_eq!(typed_chars(9.0 * CARD_LINE_S, 8, 12), Some(12));
    }

    /// The card follows the apex, and the apex is the point most likely to be
    /// near the top of the view — so "no room above" has to mean "go below",
    /// not "vanish".
    #[test]
    fn the_contact_card_flips_below_the_apex_rather_than_disappearing() {
        let view = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
        let size = egui::vec2(160.0, 130.0);

        // Room above: the card sits over the apex, centred on it.
        let (card, above) = card_rect(view, egui::pos2(400.0, 400.0), size).unwrap();
        assert!(above);
        assert_eq!(card.center().x, 400.0);
        assert_eq!(card.bottom(), 400.0 - CARD_LEADER_PX);

        // Apex near the top: the card goes under it instead, still centred.
        let (card, above) = card_rect(view, egui::pos2(400.0, 20.0), size).unwrap();
        assert!(!above);
        assert_eq!(card.center().x, 400.0);
        assert_eq!(card.top(), 20.0 + CARD_LEADER_PX);

        // Apex against the left edge: slid inside rather than dropped.
        let (card, _) = card_rect(view, egui::pos2(4.0, 400.0), size).unwrap();
        assert_eq!(card.left(), 0.0);
        assert!(view.contains_rect(card));
        // …and the right edge behaves the same way.
        let (card, _) = card_rect(view, egui::pos2(796.0, 400.0), size).unwrap();
        assert_eq!(card.right(), 800.0);
        assert!(view.contains_rect(card));

        // A view too short for the card either way gets none.
        let squat = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 120.0));
        assert!(card_rect(squat, egui::pos2(400.0, 60.0), size).is_none());
    }

    #[test]
    fn a_running_contact_shows_a_clock_that_grows_a_field_at_the_hour() {
        assert_eq!(elapsed_hms(0), "0:00");
        assert_eq!(elapsed_hms(9), "0:09");
        assert_eq!(elapsed_hms(84), "1:24");
        assert_eq!(elapsed_hms(3599), "59:59");
        assert_eq!(elapsed_hms(3600), "1:00:00");
        assert_eq!(elapsed_hms(3661), "1:01:01");
    }
}
