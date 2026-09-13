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
//! * **PATHS**, **REACH** and **BEST** come from the propagation field: real
//!   receptions, by this station and — when the Reverse Beacon Network is
//!   switched on — by everyone else's. That is a measurement.
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
        });
        ui.add_space(4.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("bands_grid").num_columns(5).spacing([14.0, 3.0]).striped(true).show(
                ui,
                |ui| {
                    ui.label(dim("BAND"));
                    ui.label(dim("CONDX"));
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
            dim("CONDX is a global forecast; the rest is what has actually been heard. \
                 An empty row means nothing was decoded — which may mean the band was \
                 shut, or only that nobody was on it.")
            .italics(),
        );
    }
}
