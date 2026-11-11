//! The S/N guide ([`crate::mean_spectrum`]): the frames it accumulates, and
//! the result handed to the spectrum widget.

use eframe::egui;

use crate::app::SdroxideApp;

impl SdroxideApp {
    /// Whether anything needs the S/N mean: the S/N or SK line, LABELS BY
    /// S/N, or the S/N filter in SPOTS. Each runs on its own switch.
    // Why: HDZ 2026-11-08: "für die Spot-Beschriftungen deren Regler ist
    //      unerreichbar, weil aus" — S/N switched off greyed LABELS BY S/N
    //      and S/N MIN, once SK could run without it.
    pub(in crate::app) fn snr_mean_wanted(&self) -> bool {
        self.view.snr_guide
            || self.view.sk_guide
            || self.view.snr_label_filter
            || (self.view.spots_snr_only && self.spot_in_view_only)
    }

    /// Whether spot labels are being judged by S/N: LABELS BY S/N, or the
    /// S/N line on (which also judges guesses and catalogue entries).
    pub(in crate::app) fn snr_judging(&self) -> bool {
        self.view.snr_guide || self.view.snr_label_filter
    }

    /// Add a new panadapter frame to the S/N guide while it is on — also
    /// while the spectrum strip is folded away, so the mean is there when it
    /// opens. Switched off, the mean is dropped: switching on starts afresh.
    pub(in crate::app) fn snr_guide_note_frame(&mut self, f: &sdroxide_types::SpectrumFrame) {
        if !self.snr_mean_wanted() {
            self.snr_grid = None;
            self.snr_last = None;
            return;
        }
        match self.snr_grid.as_ref() {
            Some(g) if g.fits(f) => {}
            Some(g) => self.snr_grid = g.relaid(f),
            None => self.snr_grid = crate::mean_spectrum::MeanGrid::new(f),
        }
        let Some(grid) = self.snr_grid.as_mut() else { return };
        grid.push(f, crate::time::now_unix_f64());
        grid.set_sk_tau(f64::from(self.view.sk_tau_s));
        self.snr_last = grid.guide().map(std::sync::Arc::new);
    }

    /// The same S/N and SK means over the full-band strip, while either line
    /// is on: the panadapter shows a slice, the strip the whole band, and the
    /// kurtosis there points at whoever is on the air anywhere in it.
    // Why: HDZ 2026-11-09: "SK oder S/N ein Inlay-Spektrum drüber … könnte
    //      interessant sein, Funker zu finden, Linien sehe ich ja schon".
    pub(in crate::app) fn wide_guide_note_frame(&mut self, f: &sdroxide_types::SpectrumFrame) {
        if !(self.view.snr_guide || self.view.sk_guide) {
            self.wide_grid = None;
            self.wide_guide = None;
            return;
        }
        match self.wide_grid.as_ref() {
            Some(g) if g.fits(f) => {}
            Some(g) => self.wide_grid = g.relaid(f),
            None => self.wide_grid = crate::mean_spectrum::MeanGrid::new(f),
        }
        let Some(grid) = self.wide_grid.as_mut() else { return };
        grid.push(f, crate::time::now_unix_f64());
        grid.set_sk_tau(f64::from(self.view.sk_tau_s));
        self.wide_guide = grid.guide().map(std::sync::Arc::new);
    }

    /// What the strip draws of [`Self::wide_guide_note_frame`]'s means: the
    /// lines that are on, on the panadapter's scales.
    pub(in crate::app) fn wide_guide_lines(
        &self,
    ) -> Option<crate::widgets::wide_spectrum::GuideLines> {
        let g = self.wide_guide.clone()?;
        Some(crate::widgets::wide_spectrum::GuideLines {
            guide: g,
            snr: self.view.snr_guide,
            sk: self.view.sk_guide,
            snr_top_db: self.view.snr_guide_top_db.clamp(
                *crate::view::SNR_GUIDE_TOP_RANGE.start(),
                *crate::view::SNR_GUIDE_TOP_RANGE.end(),
            ),
            sk_bottom_db: crate::view::SK_GUIDE_BASE_DB,
            sk_top_db: self.view.sk_guide_top_db.clamp(
                *crate::view::SK_GUIDE_TOP_RANGE.start(),
                *crate::view::SK_GUIDE_TOP_RANGE.end(),
            ),
        })
    }

    /// Hand the guide to the spectrum widget for this frame; nothing while it
    /// is off. Called once a frame.
    ///
    /// Also left in plain types — low edge Hz, cell Hz, S/N dB per cell,
    /// frames — under `"snr-guide-cells"`, for other windows ("What is
    /// this?", ⌘I) to read without depending on this module.
    // Why: HDZ 2026-11-07: "wenn ich jetzt am Cursor ⌘I drücke, ist der Wert
    //      S/N zu holen".
    pub(in crate::app) fn snr_guide_publish(&self, ctx: &egui::Context) {
        let g = self.snr_last.clone().filter(|_| self.view.snr_guide || self.view.sk_guide);
        let cells = egui::Id::new("snr-guide-cells");
        let snr_cells = g.clone().filter(|_| self.view.snr_guide);
        ctx.data_mut(|d| match &snr_cells {
            Some(g) => {
                let plain = (g.lo_hz, g.cell_hz, g.snr_db.clone(), g.frames);
                d.insert_temp(cells, std::sync::Arc::new(plain));
            }
            None => d.remove::<std::sync::Arc<(f64, f64, Vec<f32>, u32)>>(cells),
        });
        ctx.data_mut(|d| match g {
            Some(g) => {
                d.insert_temp(crate::mean_spectrum::guide_id(), g);
            }
            None => d.remove::<std::sync::Arc<crate::mean_spectrum::Guide>>(
                crate::mean_spectrum::guide_id(),
            ),
        });
    }
}
