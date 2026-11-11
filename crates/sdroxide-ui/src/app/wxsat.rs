//! The WEATHER SATELLITES window: the picture a NOAA APT or Meteor LRPT
//! satellite sends on 137 MHz, drawn as it comes in.
//!
//! It is a view, not a radio mode: the decoder runs off its own
//! downconversion of the sample window (see `sdroxide_wxsat`) and the radio
//! keeps its mode. Opened from the SATELLITE window, either from its header or
//! from an APT / LRPT link of a NOAA or Meteor satellite. State comes from [`sdroxide_types::WxsatStatus`]; the
//! picture is [`sdroxide_types::WxsatPicture`], built from the lines it carries.

use crate::i18n::tr;
use crate::theme::ThemedScroll;
use eframe::egui::{self, RichText};
use sdroxide_types::{
    Command, WXSAT_LINE_PIXELS, WXSAT_LRPT_WIDTH, WxsatKind, WxsatPicture, WxsatSettings,
    wxsat_window_covers,
};

/// The APT satellites still on the air: name and downlink. Names and units are
/// not translated.
const APT_SATELLITES: [(&str, f64); 3] =
    [("NOAA-15", 137_620_000.0), ("NOAA-18", 137_912_500.0), ("NOAA-19", 137_100_000.0)];

/// The Meteor satellites sending LRPT.
const LRPT_SATELLITES: [(&str, f64); 2] =
    [("METEOR-M2-3", 137_900_000.0), ("METEOR-M2-4", 137_100_000.0)];

/// The satellites of a signal.
fn satellites(kind: WxsatKind) -> &'static [(&'static str, f64)] {
    match kind {
        WxsatKind::Apt => &APT_SATELLITES,
        WxsatKind::Lrpt => &LRPT_SATELLITES,
    }
}

/// The WEATHER SATELLITES window's own state: open or not, how the picture is
/// drawn, and the picture the statuses have built.
pub(in crate::app) struct WxsatUi {
    pub(in crate::app) show: bool,
    /// The decoder's latest state (level, lock, line count); its `new_lines`
    /// and `rows` are moved into `picture` and left empty here.
    status: Option<Box<sdroxide_types::WxsatStatus>>,
    /// The picture the statuses have built so far.
    picture: WxsatPicture,
    /// The picture is drawn turned over.
    flip: bool,
    /// The picture's texture, and the key it was built from (`usize::MAX` to
    /// force a rebuild).
    tex: Option<egui::TextureHandle>,
    tex_key: usize,
    /// The LRPT channel shown (an APID, 64 to 69); 0 until one is picked.
    apid: u16,
}

impl Default for WxsatUi {
    fn default() -> Self {
        Self {
            show: false,
            status: None,
            picture: WxsatPicture::default(),
            flip: false,
            tex: None,
            tex_key: usize::MAX,
            apid: 0,
        }
    }
}

impl WxsatUi {
    /// Take a status from the engine: its lines go into the picture, the rest
    /// is kept for the status line.
    pub(in crate::app) fn accept(&mut self, mut st: Box<sdroxide_types::WxsatStatus>) {
        self.picture.apply(&st);
        st.new_lines = Vec::new();
        st.rows = Vec::new();
        self.status = Some(st);
    }
}

/// Which decoder a satellite link's published emission calls for, if any: the
/// SATELLITE window offers the picture on these.
pub(in crate::app) fn kind_for_link(link: &sdroxide_types::SatLink) -> Option<WxsatKind> {
    let m = format!("{} {}", link.label, link.mode).to_ascii_uppercase();
    if m.contains("LRPT") {
        Some(WxsatKind::Lrpt)
    } else if m.contains("APT") {
        Some(WxsatKind::Apt)
    } else {
        None
    }
}

/// Rows of an LRPT picture drawn at once: a texture is at most 8192 pixels
/// high, which is 1000 rows of 8 lines.
const LRPT_SHOWN_ROWS: usize = 1000;

/// One LRPT channel as grey pixels, the newest `LRPT_SHOWN_ROWS` rows at most;
/// `flip` turns it through half a turn.
pub(in crate::app) fn lrpt_gray(
    pic: &WxsatPicture,
    apid: u16,
    flip: bool,
) -> Option<([usize; 2], Vec<u8>)> {
    let rows = pic.channels.get(&apid)?;
    let from = rows.len().saturating_sub(LRPT_SHOWN_ROWS);
    let row_px = WXSAT_LRPT_WIDTH * sdroxide_types::WXSAT_LRPT_ROW_LINES;
    let mut out = vec![MISSING; (rows.len() - from) * row_px];
    for (i, r) in rows[from..].iter().enumerate() {
        if r.len() == row_px {
            out[i * row_px..(i + 1) * row_px].copy_from_slice(r);
        }
    }
    if flip {
        out.reverse();
    }
    Some(([WXSAT_LRPT_WIDTH, (rows.len() - from) * sdroxide_types::WXSAT_LRPT_ROW_LINES], out))
}

/// Where each image strip starts in a line, and how wide it is.
const STRIP_A: usize = 86;
const STRIP_B: usize = 1040 + 86;
const STRIP_W: usize = 909;
/// Dark pixels between the two strips.
const GAP: usize = 6;
/// Width of the picture as drawn: both strips and the gap.
pub(in crate::app) const PICTURE_W: usize = 2 * STRIP_W + GAP;
/// Grey of a line that never arrived.
const MISSING: u8 = 40;
const WINDOW_SIZE: [f32; 2] = [860.0, 640.0];

/// The picture as grey pixels, both image strips side by side, each line one
/// row. `flip` turns it through half a turn (a satellite flying south is
/// drawn upside down). Returns the size and the pixels.
pub(in crate::app) fn picture_gray(pic: &WxsatPicture, flip: bool) -> ([usize; 2], Vec<u8>) {
    let h = pic.lines.len();
    let mut out = vec![MISSING; PICTURE_W * h];
    for (y, line) in pic.lines.iter().enumerate() {
        if line.len() != WXSAT_LINE_PIXELS {
            continue;
        }
        let row = &mut out[y * PICTURE_W..(y + 1) * PICTURE_W];
        row[..STRIP_W].copy_from_slice(&line[STRIP_A..STRIP_A + STRIP_W]);
        row[STRIP_W..STRIP_W + GAP].fill(0);
        row[STRIP_W + GAP..].copy_from_slice(&line[STRIP_B..STRIP_B + STRIP_W]);
    }
    if flip {
        out.reverse();
    }
    ([PICTURE_W, h], out)
}

/// The commands "Tune here" sends: leave a mode that holds the front end
/// elsewhere, switch the decoder on if it is off, and bring the dial to the
/// channel so that it is inside the sample window.
pub(in crate::app) fn tune_commands(
    cfg: &WxsatSettings,
    vfo: sdroxide_types::Vfo,
    mode: sdroxide_types::Mode,
) -> Vec<Command> {
    let mut out = Vec::new();
    if mode.is_wideband_lane() || mode.is_hfdl() {
        out.push(Command::SetMode {
            rx: sdroxide_types::RxId::Main,
            mode: sdroxide_types::Mode::Nfm,
        });
    }
    if !cfg.enabled {
        out.push(Command::SetWxsatConfig(WxsatSettings { enabled: true, ..*cfg }));
    }
    out.push(Command::SetVfo { vfo, hz: cfg.frequency_hz });
    out
}

/// The name of the satellite on `hz`, or the frequency in MHz.
fn channel_name(hz: f64, kind: WxsatKind) -> String {
    satellites(kind)
        .iter()
        .find(|(_, f)| (f - hz).abs() < 1_000.0)
        .map_or_else(|| format!("{:.4} MHz", hz / 1e6), |(n, f)| format!("{n}  {:.4} MHz", f / 1e6))
}

impl super::SdroxideApp {
    pub(in crate::app) fn wxsat_window(&mut self, ctx: &egui::Context, cmds: &mut Vec<Command>) {
        if !self.wxsat.show {
            return;
        }
        let mut open = self.wxsat.show;
        let tool_title = tr!("wxsat-title");
        let mut tool_st = self.take_tool_state("WXSAT");
        let tool_child = crate::app::undocked::ChildWindow::own(self);
        crate::app::undocked::tool_window(
            ctx,
            crate::app::undocked::ToolWindow::new("WXSAT", &tool_title, WINDOW_SIZE),
            &mut tool_st,
            tool_child,
            &mut open,
            |ui| self.wxsat_body(ui, cmds),
        );
        self.put_tool_state("WXSAT", tool_st);
        self.wxsat.show = open;
    }

    /// Open the window on a satellite's channel: the decoder switched to its
    /// signal and channel, on, and the dial brought there. What the SATELLITE
    /// window's PICTURE chip does on an APT or LRPT link.
    pub(in crate::app) fn wxsat_open_on(
        &mut self,
        kind: WxsatKind,
        hz: f64,
        cmds: &mut Vec<Command>,
    ) {
        self.wxsat.show = true;
        let cfg = WxsatSettings { enabled: true, frequency_hz: hz, kind }.sane();
        if cfg.kind != self.state.wxsat.kind {
            self.wxsat.apid = 0;
            self.wxsat.tex_key = usize::MAX;
        }
        cmds.push(Command::SetWxsatConfig(cfg));
        cmds.extend(
            tune_commands(&cfg, self.state.active_vfo, self.state.rx[0].mode)
                .into_iter()
                .filter(|c| !matches!(c, Command::SetWxsatConfig(_))),
        );
    }

    fn wxsat_body(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        crate::chrome::window_body_bg(ui);
        let cfg = self.state.wxsat;
        let mut edit = cfg;
        ui.horizontal_wrapped(|ui| {
            crate::chrome::checkbox(ui, &mut edit.enabled, tr!("wxsat-decode"));
            ui.label(tr!("wxsat-signal"));
            ui.selectable_value(&mut edit.kind, WxsatKind::Apt, tr!("wxsat-kind-apt"));
            ui.selectable_value(&mut edit.kind, WxsatKind::Lrpt, tr!("wxsat-kind-lrpt"));
            if edit.kind != cfg.kind {
                // Another signal, another channel: the first satellite of its list.
                edit.frequency_hz = satellites(edit.kind)[0].1;
            }
            ui.label(tr!("wxsat-channel"));
            egui::ComboBox::from_id_salt("wxsat-channel")
                .selected_text(channel_name(edit.frequency_hz, edit.kind))
                .show_ui(ui, |ui| {
                    for (name, hz) in satellites(edit.kind) {
                        let label = format!("{name}  {:.4} MHz", hz / 1e6);
                        ui.selectable_value(&mut edit.frequency_hz, *hz, label);
                    }
                });
            if crate::chrome::chip(ui, false, tr!("wxsat-tune"))
                .on_hover_text(tr!("wxsat-tune-hint"))
                .clicked()
            {
                cmds.extend(tune_commands(&edit, self.state.active_vfo, self.state.rx[0].mode));
                edit.enabled = true;
            }
        });
        if edit != cfg && !cmds.iter().any(|c| matches!(c, Command::SetWxsatConfig(_))) {
            cmds.push(Command::SetWxsatConfig(edit));
        }
        if edit.kind != cfg.kind {
            self.wxsat.apid = 0;
            self.wxsat.tex_key = usize::MAX;
        }

        // The status line.
        let st = self.wxsat.status.as_deref();
        ui.horizontal_wrapped(|ui| {
            let dim = |s: String| RichText::new(s).size(11.0).color(crate::theme::gray(160));
            match st {
                Some(st) if cfg.enabled && st.running && st.kind == cfg.kind => {
                    ui.label(dim(tr!("wxsat-level", db = format!("{:.0}", st.level_dbfs))));
                    match cfg.kind {
                        WxsatKind::Apt => {
                            ui.label(dim(if st.locked {
                                tr!("wxsat-locked")
                            } else {
                                tr!("wxsat-searching")
                            }));
                            ui.label(dim(tr!("wxsat-lines", n = st.total_lines)));
                        }
                        WxsatKind::Lrpt => {
                            ui.label(dim(if st.locked {
                                tr!("wxsat-frame-locked")
                            } else {
                                tr!("wxsat-searching-frame")
                            }));
                            ui.label(dim(tr!("wxsat-frames", n = st.frames_ok)));
                            ui.label(dim(tr!("wxsat-rows", n = st.total_lines / 8)));
                        }
                    }
                }
                _ => {
                    ui.label(dim(tr!("wxsat-state-off")));
                }
            }
        });
        if cfg.enabled
            && !wxsat_window_covers(
                self.state.center_hz,
                self.state.sample_rate,
                cfg.frequency_hz,
                cfg.kind,
            )
        {
            ui.label(
                RichText::new(tr!("wxsat-outside-window"))
                    .size(11.0)
                    .color(crate::theme::gray(200)),
            );
        }

        // The channels an LRPT pass has sent, one picture each.
        if cfg.kind == WxsatKind::Lrpt {
            let have: Vec<u16> = self.wxsat.picture.channels.keys().copied().collect();
            if !have.contains(&self.wxsat.apid) {
                self.wxsat.apid = have.first().copied().unwrap_or(0);
            }
            ui.horizontal(|ui| {
                for apid in have {
                    let before = self.wxsat.apid;
                    ui.selectable_value(
                        &mut self.wxsat.apid,
                        apid,
                        tr!("wxsat-image-channel", n = apid - 63),
                    );
                    if before != self.wxsat.apid {
                        self.wxsat.tex_key = usize::MAX;
                    }
                }
            });
        }

        // The picture's own controls.
        let lrpt = cfg.kind == WxsatKind::Lrpt;
        ui.horizontal(|ui| {
            if crate::chrome::chip(ui, false, tr!("wxsat-clear")).clicked() {
                self.wxsat.picture.clear();
                self.wxsat.tex_key = usize::MAX;
            }
            let before = self.wxsat.flip;
            crate::chrome::checkbox(ui, &mut self.wxsat.flip, tr!("wxsat-flip"))
                .on_hover_text(tr!("wxsat-flip-hint"));
            if before != self.wxsat.flip {
                self.wxsat.tex_key = usize::MAX;
            }
            let have = !self.wxsat.picture.is_empty();
            if ui.add_enabled(have, egui::Button::new(tr!("wxsat-save"))).clicked() {
                let shot = if lrpt {
                    self.wxsat.picture.channel_gray(self.wxsat.apid).map(|(size, g)| {
                        (size, if self.wxsat.flip { g.into_iter().rev().collect() } else { g })
                    })
                } else {
                    Some(picture_gray(&self.wxsat.picture, self.wxsat.flip))
                };
                if let Some(([w, h], gray)) = shot {
                    let rgb: Vec<u8> = gray.iter().flat_map(|&g| [g, g, g]).collect();
                    if let Some(png) = crate::sstv::encode_png(&rgb, w as u16, h as u16) {
                        crate::download::save_as(
                            &wxsat_file_name(lrpt.then_some(self.wxsat.apid)),
                            &png,
                            crate::download::Mime::Png,
                        );
                    }
                }
            }
        });
        ui.separator();

        // What to draw, and a number that changes when it does.
        let (key, shot) = if lrpt {
            let rows = self.wxsat.picture.channels.get(&self.wxsat.apid);
            let key =
                rows.map_or(0, |r| r.len() * 4096 + r.iter().filter(|x| !x.is_empty()).count() + 1);
            (key, lrpt_gray(&self.wxsat.picture, self.wxsat.apid, self.wxsat.flip))
        } else {
            let n = self.wxsat.picture.lines.len();
            (n, (n > 0).then(|| picture_gray(&self.wxsat.picture, self.wxsat.flip)))
        };
        let Some(([w, h], gray)) = shot else {
            let waiting = if lrpt { tr!("wxsat-waiting-lrpt") } else { tr!("wxsat-waiting") };
            ui.label(RichText::new(waiting).color(crate::theme::gray(160)));
            return;
        };
        // The texture follows the picture when it has changed.
        if self.wxsat.tex_key != key || self.wxsat.tex.is_none() {
            let img = egui::ColorImage::from_gray([w, h], &gray);
            match self.wxsat.tex.as_mut() {
                Some(t) => t.set(img, egui::TextureOptions::LINEAR),
                None => {
                    self.wxsat.tex = Some(ui.ctx().load_texture(
                        "wxsat-picture",
                        img,
                        egui::TextureOptions::LINEAR,
                    ));
                }
            }
            self.wxsat.tex_key = key;
        }
        if let Some(tex) = &self.wxsat.tex {
            let size = tex.size_vec2();
            let scale = ui.available_width() / size.x;
            egui::ScrollArea::vertical().stick_to_bottom(!self.wxsat.flip).show_themed(ui, |ui| {
                ui.image(egui::load::SizedTexture::new(tex.id(), size * scale));
            });
        }
    }
}

/// A dated name for the saved picture; an LRPT one names its channel.
fn wxsat_file_name(channel: Option<u16>) -> String {
    match channel {
        Some(apid) => {
            format!("wxsat-ch{}-{}.png", apid.saturating_sub(63), crate::time::now_unix())
        }
        None => format!("wxsat-{}.png", crate::time::now_unix()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(v: u8) -> Vec<u8> {
        let mut l = vec![0u8; WXSAT_LINE_PIXELS];
        l[STRIP_A..STRIP_A + STRIP_W].fill(v);
        l[STRIP_B..STRIP_B + STRIP_W].fill(255 - v);
        l
    }

    #[test]
    fn the_two_strips_sit_side_by_side_with_a_gap() {
        let pic = WxsatPicture { pass: 1, lines: vec![line(10)], ..Default::default() };
        let ([w, h], g) = picture_gray(&pic, false);
        assert_eq!((w, h), (PICTURE_W, 1));
        assert_eq!(g[0], 10);
        assert_eq!(g[STRIP_W], 0);
        assert_eq!(g[STRIP_W + GAP], 245);
        assert_eq!(g[PICTURE_W - 1], 245);
    }

    #[test]
    fn a_missing_line_is_grey_and_flip_turns_the_picture_half_a_turn() {
        let pic = WxsatPicture { pass: 1, lines: vec![line(10), Vec::new()], ..Default::default() };
        let (_, g) = picture_gray(&pic, false);
        assert_eq!(g[PICTURE_W], MISSING);
        let (_, f) = picture_gray(&pic, true);
        assert_eq!(f[0], MISSING);
        // The first line is now last, and its pixels run backwards.
        assert_eq!(f[PICTURE_W], 245);
        assert_eq!(f[2 * PICTURE_W - 1], 10);
    }

    #[test]
    fn an_lrpt_channel_is_drawn_from_its_rows_newest_last() {
        let row_px = WXSAT_LRPT_WIDTH * 8;
        let mut pic = WxsatPicture::default();
        pic.channels.insert(65, vec![vec![200u8; row_px], Vec::new(), vec![50u8; row_px]]);
        let ([w, h], g) = lrpt_gray(&pic, 65, false).unwrap();
        assert_eq!((w, h), (1568, 24));
        assert_eq!((g[0], g[8 * 1568], g[16 * 1568]), (200, MISSING, 50));
        let (_, f) = lrpt_gray(&pic, 65, true).unwrap();
        assert_eq!(f[0], 50, "turned through half a turn");
        assert!(lrpt_gray(&pic, 66, false).is_none());
        // A long pass shows its newest rows only.
        pic.channels.insert(64, vec![vec![1u8; row_px]; LRPT_SHOWN_ROWS + 30]);
        assert_eq!(lrpt_gray(&pic, 64, false).unwrap().0[1], LRPT_SHOWN_ROWS * 8);
        assert!(wxsat_file_name(Some(65)).starts_with("wxsat-ch2-"));
        assert!(
            wxsat_file_name(None).starts_with("wxsat-1")
                || wxsat_file_name(None).starts_with("wxsat-")
        );
    }

    #[test]
    fn tune_here_leaves_a_wideband_mode_and_switches_the_decoder_on() {
        let cfg = WxsatSettings::default();
        let cmds = tune_commands(&cfg, sdroxide_types::Vfo::A, sdroxide_types::Mode::Adsb);
        assert!(matches!(cmds[0], Command::SetMode { .. }));
        assert!(cmds.iter().any(|c| matches!(c, Command::SetWxsatConfig(s) if s.enabled)));
        assert!(matches!(cmds.last(), Some(Command::SetVfo { hz, .. }) if *hz == cfg.frequency_hz));
        // Already on, in an ordinary mode: only the dial moves.
        let on = WxsatSettings { enabled: true, ..cfg };
        let cmds = tune_commands(&on, sdroxide_types::Vfo::A, sdroxide_types::Mode::Nfm);
        assert_eq!(cmds.len(), 1);
    }

    #[test]
    fn a_satellite_link_names_its_decoder() {
        let pb = sdroxide_types::Passband::at(137.62);
        let apt = sdroxide_types::SatLink::down("APT", "FM / APT 2400 Hz", pb);
        let lrpt = sdroxide_types::SatLink::down("LRPT", "QPSK / LRPT 72k", pb);
        let voice = sdroxide_types::SatLink::down("FM voice", "FM", pb);
        assert_eq!(kind_for_link(&apt), Some(WxsatKind::Apt));
        assert_eq!(kind_for_link(&lrpt), Some(WxsatKind::Lrpt));
        assert_eq!(kind_for_link(&voice), None);
    }

    #[test]
    fn channel_names_know_the_satellites() {
        assert!(channel_name(137_100_000.0, WxsatKind::Apt).starts_with("NOAA-19"));
        assert!(channel_name(137_900_000.0, WxsatKind::Lrpt).starts_with("METEOR-M2-3"));
        // The same frequency is a different satellite in another signal.
        assert!(channel_name(137_100_000.0, WxsatKind::Lrpt).starts_with("METEOR-M2-4"));
        assert_eq!(channel_name(137_500_000.0, WxsatKind::Apt), "137.5000 MHz");
    }
}
