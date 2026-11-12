//! The S/N guide ([`crate::mean_spectrum`]): the frames it accumulates, and
//! the result handed to the spectrum widget.

use eframe::egui;

use crate::app::SdroxideApp;

impl SdroxideApp {
    /// Add a new panadapter frame to the S/N guide while it is on — also
    /// while the spectrum strip is folded away, so the mean is there when it
    /// opens. Switched off, the mean is dropped: switching on starts afresh.
    pub(in crate::app) fn snr_guide_note_frame(&mut self, f: &sdroxide_types::SpectrumFrame) {
        if !self.view.snr_guide {
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
        self.snr_last = grid.guide().map(std::sync::Arc::new);
    }

    /// Hand the guide to the spectrum widget for this frame; nothing while it
    /// is off. Called once a frame.
    pub(in crate::app) fn snr_guide_publish(&self, ctx: &egui::Context) {
        let g = self.snr_last.clone().filter(|_| self.view.snr_guide);
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
