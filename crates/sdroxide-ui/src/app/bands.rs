//! What each band is doing: the published forecast beside the measured field.
//!
//! Two answers to one question, and they are not the same answer.
//!
//! * **CONDX** is N0NBH's calculated verdict — one word per band group per half
//!   of the day, computed globally from the solar indices. It says nothing
//!   about this station, this antenna or any particular path, and it covers
//!   80 m through 10 m and nothing else. Three bands outside that span — 160 m,
//!   60 m and 11 m — read the nearest published group as a stand-in and are
//!   marked with "≈", hover for the explanation.
//! *   **WSPR** and **PSK** are the world's networks on the band over the last
//!     fifteen minutes: WSPR's weak-signal beacons from
//!     [wspr.live](https://wspr.live), the activity modes — FT8, FT4, the
//!     CW/RTTY reporting — from [PSK Reporter](https://pskreporter.info). A
//!     measurement that needs none of our own receivers to have heard anything,
//!     which is what makes it useful on a night when the operator's own antenna
//!     is deaf to the band.
//! * **PATHS**, **REACH** and **BEST** come from the propagation field: real
//!   receptions, by this station and — when the Reverse Beacon Network is
//!   switched on — by everyone else's. That is a measurement too, of this
//!   station's own sky rather than the world's.
//!
//! PATHS and REACH are both shown because either alone misleads: a contest
//! pile-up on one bearing is a great many paths through a very small piece of
//! sky, and a band that is quietly open everywhere is the reverse.
//!
//! Where they disagree the measurement is right, which is why they are shown
//! side by side rather than reconciled into one number.
//!
//! The empty cell is load-bearing throughout. A band with no verdict has none
//! published; a band with no paths was either shut or unattended, and this
//! window does not claim to know which.

use eframe::egui::{self, Color32, RichText};
use sdroxide_solar::BandRating;
use sdroxide_types::Band;

use crate::app::SdroxideApp;

/// Column headings and everything the reader is not meant to look at first —
/// the same grey the propagation chip row uses for its scale caption.
fn dim_ink() -> Color32 {
    crate::theme::gray(110)
}

/// The colour a verdict is shown in.
///
/// `None` for a band nothing is published about, and for a wording this build
/// does not recognise — in both cases the neutral text colour is the honest
/// one, and the words are printed either way.
pub(in crate::app) fn rating_color(r: BandRating) -> Option<Color32> {
    match r {
        BandRating::Good => Some(crate::theme::GREEN()),
        BandRating::Fair => Some(crate::theme::YELLOW()),
        BandRating::Poor => Some(crate::theme::PINK()),
        BandRating::Closed => Some(dim_ink()),
        BandRating::Unknown => None,
    }
}

/// How old the verdicts are, as words, or `None` if there are none.
pub(in crate::app) fn conditions_age(app: &SdroxideApp) -> Option<String> {
    let c = app.band_conditions.as_ref()?;
    if c.observed_unix <= 0 {
        return None;
    }
    Some(sdroxide_solar::timefmt::age(crate::time::now_unix() - c.observed_unix))
}

/// How old the WSPR activity snapshot is, as words, or `None` if there is none.
fn activity_age(app: &SdroxideApp) -> Option<String> {
    let a = app.band_activity.as_ref()?;
    if a.observed_unix <= 0 {
        return None;
    }
    Some(sdroxide_solar::timefmt::age(crate::time::now_unix() - a.observed_unix))
}

/// How old the PSK Reporter activity snapshot is, as words, or `None` if there
/// is none.
fn psk_activity_age(app: &SdroxideApp) -> Option<String> {
    let a = app.psk_activity.as_ref()?;
    if a.observed_unix <= 0 {
        return None;
    }
    Some(sdroxide_solar::timefmt::age(crate::time::now_unix() - a.observed_unix))
}

/// A reception-report count as a short string: 19 397 becomes "19.4k", 932
/// stays "932".
fn count_short(n: u64) -> String {
    if n >= 10_000 {
        format!("{:.0}k", n as f64 / 1000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1000.0)
    } else {
        n.to_string()
    }
}

/// The colour the WSPR count is shown in — brighter the busier the band is
/// globally.
///
/// Deliberately *not* the CONDX palette: this is a magnitude, not a verdict.
/// A band the whole world is working reads green, one with a steady stream
/// reads cyan, one that is barely there reads dim — and a low count on 160 m,
/// where few people run WSPR, is ordinary rather than a fault, which is why
/// the ramp is absolute and the tooltip says what the number is.
fn activity_color(paths: u64) -> Color32 {
    if paths >= 1_000 {
        crate::theme::GREEN()
    } else if paths >= 100 {
        crate::theme::CYAN()
    } else {
        crate::theme::CYAN_DIM()
    }
}

/// One measured-activity cell: the band's report count, coloured by activity,
/// with the three figures and the source on hover. `table` is `None` until the
/// first fetch lands, and a band with no WSPR/PSK allocation stays blank.
fn activity_cell(
    ui: &mut egui::Ui,
    table: Option<&sdroxide_solar::BandActivityTable>,
    band: Band,
    source: &str,
    note: &str,
) {
    match table.and_then(|t| t.for_band(band)) {
        Some(a) => {
            ui.label(
                RichText::new(count_short(a.paths)).size(10.5).color(activity_color(a.paths)),
            )
            .on_hover_text(format!(
                "Global activity on {} in the last 15 minutes, from {source}:\n\
                 {} reception reports · {} transmitters · {} receivers\n\n\
                 {note}\n\n\
                 Shown brighter the busier the band is; it says the band is being heard \
                 somewhere, not that it is open to you.",
                band.label(),
                a.paths,
                a.tx,
                a.rx,
            ));
        }
        None => {
            ui.label(dim_ink_text());
        }
    }
}

/// The dash an empty cell wears.
fn dim_ink_text() -> RichText {
    RichText::new("—").size(9.5).color(dim_ink())
}

impl SdroxideApp {
    /// The BANDS window: one row per band, forecast beside evidence.
    pub(in crate::app) fn bands_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_bands;
        let resp = egui::Window::new("BANDS")
            .id(crate::layout::salted_id(ctx, "BANDS"))
            .open(&mut open)
            .frame(crate::chrome::window_frame())
            .resizable(true)
            .default_width(crate::layout::window_w(ctx, 520.0))
            .default_height(crate::layout::window_h(ctx, 460.0))
            .show(ctx, |ui| {
                crate::chrome::window_body_bg(ui);
                self.bands_body(ui)
            });
        if let Some(r) = &resp {
            crate::chrome::paint_window_border(ctx, &r.response);
        }
        self.show_bands = open;
    }

    fn bands_body(&mut self, ui: &mut egui::Ui) {
        let field = self.prop.peek();
        let dim = |s: &str| RichText::new(s).size(9.5).color(dim_ink());

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(if self.daylight { "☀ DAYLIGHT" } else { "☾ NIGHT" })
                    .size(10.0)
                    .color(crate::theme::CYAN_DIM()),
            );
            match conditions_age(self) {
                Some(a) => {
                    ui.label(dim(&format!("· forecast from HAMQSL.com, {a} old")));
                }
                None => {
                    ui.label(dim("· no forecast yet — fetching from HAMQSL.com"));
                }
            }
            if let Some(a) = activity_age(self) {
                ui.label(dim(&format!("· WSPR activity {a} old (wspr.live)")));
            }
            if let Some(a) = psk_activity_age(self) {
                ui.label(dim(&format!("· PSK activity {a} old")));
            }
        });
        ui.add_space(4.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("bands_grid").num_columns(7).spacing([14.0, 3.0]).striped(true).show(
                ui,
                |ui| {
                    ui.label(dim("BAND"));
                    ui.label(dim("CONDX"));
                    ui.label(dim("WSPR"));
                    ui.label(dim("PSK"));
                    ui.label(dim("PATHS"));
                    ui.label(dim("REACH"));
                    ui.label(dim("BEST"));
                    ui.end_row();

                    for b in Band::ALL {
                        if b == Band::Gen {
                            continue;
                        }
                        ui.label(RichText::new(b.label()).size(11.0).strong());

                        // The forecast. Derived bands — 160 m, 60 m and 11 m,
                        // which HAMQSL.com publishes nothing about — read the
                        // nearest published group and are marked with "≈", so a
                        // band that has no verdict of its own never appears to
                        // have won one.
                        match self
                            .band_conditions
                            .as_ref()
                            .and_then(|c| c.verdict_for(b, self.daylight))
                        {
                            Some(v) => {
                                let shown = if v.derived {
                                    format!("≈{}", v.verdict)
                                } else {
                                    v.verdict.to_string()
                                };
                                let t = RichText::new(shown).size(10.5);
                                let t = match rating_color(BandRating::of(v.verdict)) {
                                    Some(c) => t.color(c),
                                    None => t,
                                };
                                let resp = ui.label(t);
                                if v.derived {
                                    resp.on_hover_text(format!(
                                        "{} is not one of the bands HAMQSL.com grades; this \
                                         is the published {} group's verdict, read as the \
                                         nearest stand-in.\n\nA forecast, not a measurement \
                                         of your own path.",
                                        b.label(),
                                        v.group,
                                    ));
                                }
                            }
                            None => {
                                ui.label(dim("—"));
                            }
                        }

                        // The world's WSPR network, then its activity-mode
                        // counterpart: two measurements that need none of our
                        // own receivers to have heard anything.
                        activity_cell(
                            ui,
                            self.band_activity.as_ref(),
                            b,
                            "wspr.live",
                            "The world's WSPR network — weak-signal beacons.",
                        );
                        activity_cell(
                            ui,
                            self.psk_activity.as_ref(),
                            b,
                            "PSK Reporter",
                            "Reception reports from the activity modes: FT8, FT4 and the \
                             CW/RTTY reporting that WSPR's beacons do not cover.",
                        );

                        // The evidence.
                        match field.plane(b).filter(|p| !p.is_empty()) {
                            Some(p) => {
                                ui.label(
                                    RichText::new(format!("{:.0}", p.total_paths())).size(10.5),
                                );
                                ui.label(
                                    RichText::new(format!("{:.0}%", p.reach() * 100.0)).size(10.5),
                                );
                                ui.label(match p.best_margin_db() {
                                    Some(m) => RichText::new(format!("{m:+.0} dB")).size(10.5),
                                    // Paths but no reports: a plane built from
                                    // logged contacts alone. An RST is not an
                                    // SNR, so there is no number to print.
                                    None => dim("—"),
                                });
                            }
                            None => {
                                // Not "closed": nothing was heard, and nobody
                                // may have been transmitting. The two look
                                // identical from here and must not be conflated.
                                ui.label(dim("—"));
                                ui.label(dim("—"));
                                ui.label(dim("—"));
                            }
                        }
                        ui.end_row();
                    }
                },
            );
        });

        ui.add_space(6.0);
        ui.label(
            dim("CONDX is a global forecast. WSPR is what the world's WSPR network heard \
                 (wspr.live); PATHS, REACH and BEST are what this station and the RBN heard. \
                 An empty row means nothing was decoded — which may mean the band was shut, \
                 or only that nobody was on it.")
            .italics(),
        );
    }
}
