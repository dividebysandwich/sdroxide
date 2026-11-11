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
