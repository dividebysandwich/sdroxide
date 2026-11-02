//! What each band is doing: the published forecast beside the measured field.
//!
//! Two answers to one question, and they are not the same answer.
//!
//! * **CONDX** is N0NBH's calculated verdict — one word per band group per half
//!   of the day, computed globally from the solar indices. It says nothing
//!   about this station, this antenna or any particular path, and it covers
//!   80 m through 10 m and nothing else.
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

use crate::i18n::tr;
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
        let tool_title = tr!("bands-bands");
        let mut tool_st = self.take_tool_state("BANDS");
        let tool_child = crate::app::undocked::ChildWindow::own(self);
        crate::app::undocked::tool_window(
            ctx,
            crate::app::undocked::ToolWindow::new("BANDS", &tool_title, [520.0, 460.0]),
            &mut tool_st,
            tool_child,
            &mut open,
            |ui| self.bands_body(ui),
        );
        self.put_tool_state("BANDS", tool_st);
        self.show_bands = open;
        if self.show_bands {
            // The IBP beacon and its countdown move every second; keep painting
            // while the window is open rather than showing a stale slot.
            crate::repaint::after_ms(ctx, 1000);
        }
    }

    fn bands_body(&mut self, ui: &mut egui::Ui) {
        let field = self.prop.peek();
        let dim = |s: &str| RichText::new(s).size(9.5).color(dim_ink());

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(if self.daylight {
                    tr!("bands-daylight")
                } else {
                    tr!("bands-night")
                })
                .size(10.0)
                .color(crate::theme::CYAN_DIM()),
            );
            match conditions_age(self) {
                Some(a) => {
                    ui.label(dim(&tr!("bands-forecast-hamqsl-com-old", value = a)));
                }
                None => {
                    ui.label(dim(&tr!("bands-no-forecast-yet-fetching-hamqsl")));
                }
            }
        });
        ui.add_space(4.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("bands_grid").num_columns(5).spacing([14.0, 3.0]).striped(true).show(
                ui,
                |ui| {
                    ui.label(dim(&tr!("bands-band")));
                    ui.label(dim(&tr!("bands-condx")));
                    ui.label(dim(&tr!("bands-paths")));
                    ui.label(dim(&tr!("bands-reach")));
                    ui.label(dim(&tr!("bands-best")));
                    ui.end_row();

                    for b in Band::ALL {
                        if b == Band::Gen {
                            continue;
                        }
                        ui.label(RichText::new(b.label()).size(11.0).strong());

                        // The forecast. Blank where none is published — see the
                        // module note; 160 m, 60 m and everything above 10 m
                        // live here permanently.
                        match self
                            .band_conditions
                            .as_ref()
                            .and_then(|c| c.for_band(b, self.daylight))
                        {
                            Some(v) => {
                                let t = RichText::new(v).size(10.5);
                                ui.label(match rating_color(BandRating::of(v)) {
                                    Some(c) => t.color(c),
                                    None => t,
                                });
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
            self.ibp_section(ui);
            self.meteor_section(ui);
        });

        ui.add_space(6.0);
        ui.label(dim(&tr!("bands-condx-global-forecast-rest-what")).italics());
    }

    /// The IBP beacon list at the foot of the BANDS window.
    ///
    /// The schedule is deterministic — eighteen beacons, five bands, a
    /// three-minute cycle — so this is a clock and a table rather than a feed.
    /// A beacon you can hear is a path that is open, measured rather than
    /// forecast.
    fn ibp_section(&self, ui: &mut egui::Ui) {
        let now = crate::time::now_unix();
        let home = sdroxide_types::grid_to_latlon(&self.my_grid());
        let active = sdroxide_types::ibp_active_at(now, home);
        let dim = |s: &str| RichText::new(s.to_string()).size(9.5).color(dim_ink());
        ui.add_space(12.0);
        ui.label(
            RichText::new(tr!("bands-ibp-beacons"))
                .size(10.0)
                .strong()
                .color(crate::theme::CYAN_DIM()),
        );
        ui.add_space(2.0);
        ui.label(dim(&tr!(
            "bands-ncdxf-iaru-next-beacon-s",
            now = sdroxide_types::ibp_seconds_left(now)
        )));
        ui.add_space(3.0);
        for a in &active {
            ui.horizontal(|ui| {
                ui.label(RichText::new(a.band.label).size(10.5).strong());
                ui.label(
                    RichText::new(format!("{e6:.3} MHz", e6 = a.band.freq_hz / 1e6)).size(10.5),
                );
                ui.label(RichText::new(a.beacon.callsign).size(11.0).strong());
                ui.label(dim(a.beacon.location));
                if let (Some(b), Some(d)) = (a.bearing_deg, a.distance_km) {
                    ui.label(dim(&format!(
                        "{} · {:.0} km",
                        sdroxide_solar::satellites::compass(b),
                        d
                    )));
                }
            })
            .response
            .on_hover_text(tr!(
                "bands-hearing-means-path-open-each",
                callsign = a.beacon.callsign,
                location = a.beacon.location,
                band = a.band.label
            ));
        }
    }

    /// The meteor-shower list at the foot of the BANDS window.
    ///
    /// The one propagation forecast that has nothing to do with the ionosphere:
    /// a shower's radiant and its date window are fixed, so what is shown is
    /// whether it is active now, how strong its peak is, and whether the radiant
    /// is above this station's horizon. Radiants are placed for the operator's
    /// own locator, which is what makes "is it up" true here rather than on
    /// average.
    fn meteor_section(&self, ui: &mut egui::Ui) {
        let Some((lat, lon)) = sdroxide_types::grid_to_latlon(&self.my_grid()) else {
            return;
        };
        let active = sdroxide_solar::active_at(lat, lon, crate::time::now_unix());
        if active.is_empty() {
            return;
        }
        let dim = |s: &str| RichText::new(s.to_string()).size(9.5).color(dim_ink());
        ui.add_space(12.0);
        ui.label(
            RichText::new(tr!("bands-meteor-showers"))
                .size(10.0)
                .strong()
                .color(crate::theme::CYAN_DIM()),
        );
        ui.add_space(2.0);
        ui.label(dim(&tr!(
            "bands-radiants-placed",
            lat = format!("{:.0}", lat),
            lon = format!("{:.0}", lon)
        )));
        ui.add_space(3.0);
        for a in &active {
            let s = a.shower;
            ui.horizontal(|ui| {
                ui.label(RichText::new(s.name).size(11.0).strong());
                ui.label(dim(s.code));
                ui.label(RichText::new(tr!("bands-zhr", zhr = s.zhr)).size(10.5));
                if a.at_peak() {
                    ui.label(
                        RichText::new(tr!("bands-peak"))
                            .size(9.5)
                            .strong()
                            .color(crate::theme::GREEN()),
                    );
                }
                if a.radiant_up() {
                    ui.label(
                        RichText::new(tr!(
                            "bands-radiant",
                            alt_deg = format!("{:.0}", a.alt_deg),
                            az_deg = sdroxide_solar::satellites::compass(a.az_deg)
                        ))
                        .size(10.5),
                    );
                } else {
                    ui.label(
                        RichText::new(tr!(
                            "bands-radiant-down",
                            alt_deg = format!("{:.0}", a.alt_deg)
                        ))
                        .size(10.5)
                        .color(dim_ink()),
                    );
                }
            })
            .response
            .on_hover_text(tr!(
                "bands-km-s-parent-peak-rate",
                value = s.name,
                code = s.code,
                velocity_kms = s.velocity_kms,
                parent = s.parent,
                zhr = s.zhr,
                peak = peak_label(s.peak)
            ));
        }
    }
}

/// "3 Jan" from a `(month, day)` pair, for the hover.
fn peak_label(peak: (u32, u32)) -> String {
    tr!("bands-peak-date", day = peak.1, month = peak.0)
}
