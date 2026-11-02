//! The DRM window: what a Digital Radio Mondiale broadcast says about itself.
//!
//! Two things are worth looking at, and they answer different questions. The
//! **sync row** is for tuning one in: it shows how far up the chain the decoder
//! has got, so a station that is present but not decoding says *where* it
//! stopped rather than just staying silent. Everything below it is for once
//! that has succeeded — who is broadcasting, how, and what they are saying
//! about the programme.
//!
//! DRM's own latency is seconds — 400 ms or 2 s of time interleaving before
//! the decoder even starts — so nothing here reacts as quickly as an analog
//! S-meter does. That is the transmission, not the display.

use crate::i18n::tr;
use eframe::egui::{self, Color32, RichText};
use sdroxide_types::{Command, DrmChannel, DrmStatus, DrmSync, Mode};

use crate::app::SdroxideApp;

/// How many status snapshots the quality history keeps. The engine sends four
/// a second, so this is about a minute — long enough to watch a fade come and
/// go, which is the thing worth watching on shortwave.
const HISTORY_LEN: usize = 240;

/// Field labels and anything the reader is not meant to look at first.
fn dim_ink() -> Color32 {
    crate::theme::gray(110)
}

/// The colour of one stage's indicator. Deliberately not a red/green pair:
/// "arriving with errors" is the interesting middle state while tuning, and it
/// is the one that says the signal is real but not yet good enough.
fn sync_ink(s: DrmSync) -> Color32 {
    match s {
        DrmSync::Absent => crate::theme::gray(90),
        DrmSync::CrcError => Color32::from_rgb(220, 90, 70),
        DrmSync::DataError => Color32::from_rgb(220, 180, 70),
        DrmSync::Ok => Color32::from_rgb(90, 200, 120),
    }
}

/// One axis-aligned rectangle, as two triangles, into a mesh that is drawn in a
/// single pass. Flat-shaded and unfeathered — see the note at the call site.
fn push_quad(mesh: &mut egui::Mesh, rect: egui::Rect, color: Color32) {
    let base = mesh.vertices.len() as u32;
    for pos in [rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom()] {
        mesh.vertices.push(egui::epaint::Vertex { pos, uv: egui::epaint::WHITE_UV, color });
    }
    mesh.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

/// Distance from a received symbol to the nearest ideal one.
fn nearest_error(re: f32, im: f32, levels: &[f32]) -> f32 {
    // The constellation is a square grid symmetric about both axes, so the
    // nearest point is found per axis independently.
    let axis =
        |v: f32| levels.iter().map(|&l| (v - l.copysign(v)).abs()).fold(f32::INFINITY, f32::min);
    let (dx, dy) = (axis(re), axis(im));
    (dx * dx + dy * dy).sqrt()
}

/// Green where a symbol is comfortably inside its own decision region, amber
/// as it approaches the boundary, red past it — which is a symbol that would
/// have been decoded as its neighbour had the FEC not caught it.
fn error_ink(err: f32) -> Color32 {
    let t = err.clamp(0.0, 1.5) / 1.5;
    let (r, g, b) = if t < 0.5 {
        let k = t / 0.5;
        (90.0 + k * 130.0, 200.0 + k * 10.0, 120.0 - k * 50.0)
    } else {
        let k = (t - 0.5) / 0.5;
        (220.0, 210.0 - k * 120.0, 70.0 - k * 10.0)
    };
    Color32::from_rgb(r as u8, g as u8, b as u8)
}

impl SdroxideApp {
    pub(in crate::app) fn on_drm(&mut self, data: DrmStatus) {
        // Only while locked: the figures mean nothing otherwise, and plotting
        // the zeros a dropout leaves would draw a cliff that says "the signal
        // got worse" when what happened is that it went away.
        if data.locked {
            if self.drm_history.len() >= HISTORY_LEN {
                self.drm_history.pop_front();
            }
            self.drm_history.push_back((data.snr_db, data.wmer_db));
        } else if !self.drm_history.is_empty() && !data.time_sync.is_ok() {
            // Sync well and truly gone — start the trace again rather than
            // joining across the gap.
            self.drm_history.clear();
        }
        self.drm = Some(data);
    }

    pub(in crate::app) fn drm_window(&mut self, ctx: &egui::Context, cmds: &mut Vec<Command>) {
        // The decoder copies a frame of cells under its own lock to answer
        // this, so it is asked only while the plot is actually on screen.
        let want =
            (self.show_drm && self.state.rx[0].mode == Mode::Drm).then_some(self.drm_channel);
        if want != self.drm_const_req {
            self.drm_const_req = want;
            cmds.push(Command::SetDrmConstellation { channel: want });
        }

        if !self.show_drm {
            return;
        }
        let mut open = self.show_drm;
        let tool_title = "DRM".to_string();
        let mut tool_st = self.take_tool_state("DRM");
        let tool_child = crate::app::undocked::ChildWindow::own(self);
        let resp = crate::app::undocked::tool_window(
            ctx,
            crate::app::undocked::ToolWindow::new("DRM", &tool_title, [420.0, 380.0]),
            &mut tool_st,
            tool_child,
            &mut open,
            |ui| self.drm_body(ui),
        );
        self.put_tool_state("DRM", tool_st);
        if let Some(c) = resp {
            cmds.extend(c);
        }
        self.show_drm = open;
    }

    fn drm_body(&mut self, ui: &mut egui::Ui) -> Vec<Command> {
        let mut cmds = Vec::new();
        let dim = |s: &str| RichText::new(s).size(9.5).color(dim_ink());

        let Some(d) = self.drm.clone() else {
            ui.label(dim(&tr!("drm-waiting-receiver")));
            return cmds;
        };

        if self.state.rx[0].mode != Mode::Drm {
            ui.label(dim(&tr!("drm-not-drm-set-mode-drm")));
            ui.add_space(6.0);
        }

        self.drm_sync_row(ui, &d);
        ui.add_space(8.0);

        if !d.locked {
            ui.label(dim(&tr!("drm-no-drm-signal-locked-decoder")));
            return cmds;
        }

        self.drm_signal(ui, &d);
        ui.add_space(8.0);
        // Above the service block rather than below it: this is the reason the
        // station named there cannot be heard.
        if d.service.codec.is_some() && !d.service.codec_supported {
            ui.label(dim(&tr!("drm-station-s-audio-codec-not")));
            ui.add_space(8.0);
        }
        self.drm_service(ui, &d, &mut cmds);
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(6.0);
        self.drm_quality_history(ui);
        ui.add_space(8.0);
        self.drm_constellation(ui);
        cmds
    }

    /// SNR and MER over the last minute.
    ///
    /// Two traces on one scale, because the gap between them is itself the
    /// reading: SNR is what the receiver measures of the channel, MER what the
    /// demodulator actually achieved on it. They track each other on a clean
    /// path, and MER falls away from SNR when something the noise figure does
    /// not describe — multipath, a drifting transmitter, an overloaded front
    /// end — is costing the decoder margin.
    fn drm_quality_history(&self, ui: &mut egui::Ui) {
        let dim = |s: &str| RichText::new(s).size(9.5).color(dim_ink());
        ui.horizontal(|ui| {
            ui.label(dim(&tr!("drm-quality")));
            ui.label(RichText::new("SNR").size(9.0).color(Color32::from_rgb(90, 190, 230)));
            ui.label(RichText::new("MER").size(9.0).color(Color32::from_rgb(150, 210, 120)));
            ui.label(dim(&tr!("drm-last-minute")));
        });

        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 54.0), egui::Sense::hover());
        if !ui.is_rect_visible(rect) {
            return;
        }
        let p = ui.painter_at(rect);
        p.rect_filled(rect, 2.0, crate::theme::gray(24));

        if self.drm_history.len() < 2 {
            p.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                tr!("drm-waiting-locked-signal"),
                egui::FontId::proportional(10.0),
                dim_ink(),
            );
            return;
        }

        // A fixed 0–35 dB window, widened only if the signal goes past it.
        // Autoscaling to the data would make a steady signal look like it was
        // moving, which is the opposite of what a history is for.
        let top =
            self.drm_history.iter().flat_map(|&(s, m)| [s, m]).fold(35.0f32, f32::max).min(60.0);
        // Newest at the right edge, so "now" is always in the same place and a
        // history that has not filled the minute yet leaves the gap behind it
        // rather than in front of it.
        let newest = self.drm_history.len().saturating_sub(1);
        let x_at = |i: usize| {
            rect.right() - rect.width() * ((newest - i) as f32 / (HISTORY_LEN - 1) as f32)
        };
        let y_at = |db: f32| rect.bottom() - rect.height() * (db.clamp(0.0, top) / top);

        // Ten-dB rules, so the height can be read without an axis.
        let mut db = 10.0;
        while db < top {
            let y = y_at(db);
            p.line_segment(
                [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                egui::Stroke::new(1.0, crate::theme::gray(40)),
            );
            p.text(
                egui::pos2(rect.left() + 3.0, y),
                egui::Align2::LEFT_BOTTOM,
                format!("{db:.0}"),
                egui::FontId::proportional(8.0),
                crate::theme::gray(70),
            );
            db += 10.0;
        }

        for (pick, color) in
            [(0usize, Color32::from_rgb(90, 190, 230)), (1usize, Color32::from_rgb(150, 210, 120))]
        {
            let pts: Vec<egui::Pos2> = self
                .drm_history
                .iter()
                .enumerate()
                .map(|(i, &(s, m))| egui::pos2(x_at(i), y_at(if pick == 0 { s } else { m })))
                .collect();
            p.add(egui::Shape::line(pts, egui::Stroke::new(1.2, color)));
        }
    }

    /// How far up the chain the decoder has got, left to right in the order the
    /// stages lock.
    fn drm_sync_row(&self, ui: &mut egui::Ui, d: &DrmStatus) {
        // Where the row stops is the diagnosis, and this is the one stage that
        // can stop for a reason the chain itself does not show.
        let audio_hover = if d.service.codec.is_some() && !d.service.codec_supported {
            tr!("drm-audio-frames-decoding-but-station")
        } else {
            tr!("drm-audio-frames-decoding")
        };
        let stages = [
            ("IO", d.io, tr!("drm-samples-reaching-decoder")),
            (&tr!("drm-time"), d.time_sync, tr!("drm-symbol-timing-recovered")),
            (&tr!("drm-frame"), d.frame_sync, tr!("drm-transmission-frames-found")),
            ("FAC", d.fac, tr!("drm-fast-access-channel-what-transmission")),
            ("SDC", d.sdc, tr!("drm-service-description-channel-what-services")),
            (&tr!("drm-audio"), d.audio, audio_hover),
        ];
        ui.horizontal_wrapped(|ui| {
            for (label, state, hover) in stages {
                let text = RichText::new(label).size(9.5).color(if state.is_ok() {
                    crate::theme::gray(200)
                } else {
                    dim_ink()
                });
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 3.0;
                    // Painted rather than a text bullet: the bundled fonts have
                    // no U+25CF, and a missing glyph draws as a hollow box —
                    // which reads as its own indicator state.
                    let (dot, _) =
                        ui.allocate_exact_size(egui::vec2(7.0, 7.0), egui::Sense::hover());
                    ui.painter().circle_filled(dot.center(), 3.5, sync_ink(state));
                    ui.label(text);
                })
                .response
                .on_hover_text(hover);
                ui.add_space(6.0);
            }
        });
    }

    /// The constellation of one logical channel — the picture of how well the
    /// signal is being decoded rather than whether it is.
    ///
    /// Tight clusters on the ideal points mean margin. A cloud that has grown
    /// until neighbouring clusters touch is a decoder at its limit, and is what
    /// a rising bit error rate looks like before the audio starts breaking up.
    /// A ring, or a cloud rotated off the reference points, is an equaliser
    /// that has not resolved the channel rather than a weak signal.
    fn drm_constellation(&mut self, ui: &mut egui::Ui) {
        let dim = |s: &str| RichText::new(s).size(9.5).color(dim_ink());

        ui.horizontal(|ui| {
            ui.label(dim(&tr!("drm-constellation")));
            for ch in DrmChannel::ALL {
                if crate::chrome::chip(ui, self.drm_channel == ch, ch.label())
                    .on_hover_text(ch.describes())
                    .clicked()
                {
                    self.drm_channel = ch;
                }
            }
            if let Some(c) = self.drm.as_ref().and_then(|d| d.constellation.as_ref()) {
                ui.label(dim(&format!("{}-QAM", c.qam)));
            }
        });

        // Square: the two axes are the same quantity and a stretched plot would
        // make a round cloud look elliptical, which is a real fault it must not
        // be able to fake.
        let side = ui.available_width().clamp(120.0, 260.0);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(side, side), egui::Sense::hover());
        if !ui.is_rect_visible(rect) {
            return;
        }
        let p = ui.painter_at(rect);
        p.rect_filled(rect, 2.0, crate::theme::gray(24));

        let Some(c) = self.drm.as_ref().and_then(|d| d.constellation.as_ref()) else {
            p.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                tr!("drm-waiting-decoded-symbols"),
                egui::FontId::proportional(10.0),
                dim_ink(),
            );
            return;
        };
        if c.is_empty() {
            p.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                tr!("drm-not-decoding", channel = c.channel.label()),
                egui::FontId::proportional(10.0),
                dim_ink(),
            );
            return;
        }

        let extent = c.plot_extent().max(1e-6);
        let half = rect.width() * 0.5;
        let centre = rect.center();
        let to_px = |re: f32, im: f32| {
            // Imaginary counts upwards, so the screen's y is negated.
            egui::pos2(centre.x + re / extent * half, centre.y - im / extent * half)
        };

        // Axes through the origin.
        let axis = egui::Stroke::new(1.0, crate::theme::gray(45));
        p.line_segment(
            [egui::pos2(rect.left(), centre.y), egui::pos2(rect.right(), centre.y)],
            axis,
        );
        p.line_segment(
            [egui::pos2(centre.x, rect.top()), egui::pos2(centre.x, rect.bottom())],
            axis,
        );

        // The ideal symbols, as faint rings the received cloud should sit in.
        // Sixty-four of them at most, and they are what makes the plot legible,
        // so they stay proper stroked circles.
        let levels = c.ideal_levels();
        let ideal_ink = crate::theme::gray(70);
        for &lx in &levels {
            for &ly in &levels {
                for (sx, sy) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
                    p.circle_stroke(
                        to_px(lx * sx, ly * sy),
                        2.5,
                        egui::Stroke::new(1.0, ideal_ink),
                    );
                }
            }
        }

        // The symbols themselves go into one mesh of flat quads rather than a
        // shape apiece.
        //
        // There are 512 of them, and as separate `circle_filled` shapes each is
        // tessellated into a feathered antialiased fan — every frame, for a
        // picture that changes four times a second. How often it is drawn is
        // the frame rate's business (Settings → UI), and it is paced by it;
        // how much work each of those frames costs on a thin machine should
        // not also be left to chance. Two triangles apiece in a single draw
        // takes about a quarter off the panel's cost, and at two pixels a
        // hard-edged square and a feathered disc are the same few pixels.
        let mut mesh = egui::Mesh::default();

        // Half the spacing between neighbouring ideal points: the distance at
        // which a symbol is as close to its neighbour as to its own point, and
        // therefore the natural unit for "how wrong is this one".
        let tolerance = levels.first().copied().unwrap_or(0.5).max(1e-6);
        let size = egui::Vec2::splat(if c.len() > 300 { 2.0 } else { 2.8 });
        for (re, im) in c.iter() {
            let err = nearest_error(re, im, &levels) / tolerance;
            push_quad(&mut mesh, egui::Rect::from_center_size(to_px(re, im), size), error_ink(err));
        }
        p.add(egui::Shape::mesh(mesh));

        ui.label(dim(&tr!(
            "drm-symbols",
            value = c.len(),
            frame = if c.channel == DrmChannel::Msc {
                tr!("drm-sampled-across-frame")
            } else {
                tr!("drm-whole-frame")
            }
        )));
    }

    fn drm_signal(&self, ui: &mut egui::Ui, d: &DrmStatus) {
        let dim = |s: &str| RichText::new(s).size(9.5).color(dim_ink());
        let val = |s: String| RichText::new(s).size(11.0);

        egui::Grid::new("drm-signal").num_columns(4).spacing([14.0, 3.0]).show(ui, |ui| {
            ui.label(dim("SNR"));
            ui.label(val(format!("{:.1} dB", d.snr_db)));
            ui.label(dim("MER"));
            ui.label(val(format!("{:.1} dB", d.wmer_db)));
            ui.end_row();

            ui.label(dim(&tr!("drm-mode")));
            ui.label(val(format!(
                "{} / {}",
                d.robustness.map(|r| r.label()).unwrap_or("?"),
                d.bandwidth_khz
                    .map(|b| format!("{value} kHz", value = b))
                    .unwrap_or_else(|| "?".into()),
            )))
            .on_hover_text(tr!("drm-robustness-mode-channel-width-ground"));
            ui.label(dim(&tr!("drm-interleave")));
            ui.label(val(if d.interleaver_long { "2 s".into() } else { "400 ms".into() }))
                .on_hover_text(tr!("drm-how-far-transmission-spreads-each"));
            ui.end_row();

            ui.label(dim(&tr!("drm-protection")));
            ui.label(val(format!("B {} / A {}", d.protection_b, d.protection_a)));
            ui.label(dim(&tr!("drm-offset")));
            ui.label(val(format!("{:+.0} Hz", d.sample_offset_hz)))
                .on_hover_text(tr!("drm-residual-sample-clock-error-against"));
            ui.end_row();

            if let Some(dop) = d.doppler_hz {
                ui.label(dim(&tr!("drm-doppler")));
                ui.label(val(format!("{dop:.1} Hz")));
                ui.label(dim(&tr!("drm-delay")));
                ui.label(val(format!("{:.1} ms", d.delay_ms)))
                    .on_hover_text(tr!("drm-doppler-delay-spread-path-how"));
                ui.end_row();
            }
        });
    }

    fn drm_service(&self, ui: &mut egui::Ui, d: &DrmStatus, cmds: &mut Vec<Command>) {
        let dim = |s: &str| RichText::new(s).size(9.5).color(dim_ink());

        if !d.service.label.is_empty() {
            ui.label(RichText::new(&d.service.label).size(15.0).strong());
        }

        let mut line = Vec::new();
        if !d.service.country.is_empty() {
            line.push(d.service.country.to_uppercase());
        }
        if !d.service.language.is_empty() {
            line.push(d.service.language.clone());
        }
        if let Some(c) = d.service.codec {
            line.push(if d.service.codec_supported {
                c.label().to_string()
            } else {
                tr!("drm-not-decodable", value = c.label())
            });
        }
        if d.service.bitrate_kbps > 0.0 {
            line.push(format!("{bitrate_kbps:.1} kbps", bitrate_kbps = d.service.bitrate_kbps));
        }
        line.push(if d.service.stereo { "stereo".into() } else { "mono".into() });
        if !line.is_empty() {
            ui.label(dim(&line.join(" \u{00b7} ")));
        }

        // Only worth a control when the multiplex actually carries a choice.
        if d.audio_services > 1 {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(dim(&tr!("drm-service")));
                for i in 0..d.audio_services {
                    let on = i == d.current_service;
                    if crate::chrome::chip(ui, on, (i + 1).to_string())
                        .on_hover_text(tr!("drm-decode-service-multiplex"))
                        .clicked()
                        && !on
                    {
                        cmds.push(Command::SetDrmService { service: i });
                    }
                }
            });
        }

        if !d.service.text.is_empty() {
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(4.0);
            // The broadcaster's own message, whatever they put in it. Wrapped
            // rather than scrolled: it is a couple of lines at most, and the
            // standard caps it at 128 characters.
            ui.label(RichText::new(&d.service.text).size(11.0));
        }

        if let Some(t) = d.time {
            ui.add_space(6.0);
            ui.label(dim(&tr!(
                "drm-broadcaster-s-clock-utc",
                year = format!("{:04}", t.year),
                month = format!("{:02}", t.month),
                day = format!("{:02}", t.day),
                hour = format!("{:02}", t.hour),
                minute = format!("{:02}", t.minute)
            )));
        }
    }
}
