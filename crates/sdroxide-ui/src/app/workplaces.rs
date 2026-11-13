//! The ZULETZT rows of the band menu: the operator's recent working places.
//!
//! A working place is a band and a mode, with the dial and the filter it was
//! left on. Band buttons recall a band's own stack, and an FT8 entry there
//! used to drag the operator back into FT8 and its panel; this is the other
//! way back, chosen explicitly. Twelve places, no duplicates: coming
//! back to 40 m FT8 refreshes its entry where it stands, and pressing one
//! leaves it in its place. A place counts once it has been worked a while
//! (`DWELL_S`), so tuning or clicking through the bands does not fill it.
//! The digital modes' own setup — the transmit audio offset — is already kept
//! per band in `DigiConfig`, so dial, mode and filter bring all of it back.

use eframe::egui::{self, RichText};
use sdroxide_types::{Band, BandStackEntry, Command, Mode, RxId, Vfo};

use crate::app::SdroxideApp;
use crate::i18n::tr;

/// How many places the rows hold: three rows of four in a usual band menu
/// (HDZ 2026-11-08: „da können weit mehr drauf“).
pub(in crate::app) const SLOTS: usize = 12;
/// Seconds on a place before it enters the rows: long enough that the
/// bands clicked past on the way do not count, short enough that each band an
/// FT8 band chip stopped on does (HDZ 2026-11-08: „nach ca. 3 Sekunden“).
pub(in crate::app) const DWELL_S: f64 = 3.0;

/// A place that has just entered the rows blinks this long, from the first
/// time the rows are drawn after it — so it is seen even when the menu was
/// shut at the moment it came in (HDZ 2026-11-08: „nach 3 Sek blinkend den
/// Eintrag setzen, dann merkt das Auge: hier passiert etwas“).
pub(in crate::app) const BLINK_S: f64 = 1.5;
const BLINK_STEP_S: f64 = 0.25;

/// The newborn place, kept in egui's memory because the rows are drawn by a
/// free function: the place, and when its blink began (`None` until drawn).
pub(in crate::app) fn born_id() -> egui::Id {
    egui::Id::new("sdroxide-workplace-born")
}

/// Whether the newborn chip is drawn inverted `elapsed` seconds into its
/// blink: three flashes, then its own state. `None` once the blink is over.
pub(in crate::app) fn blink_inverted(elapsed: f64) -> Option<bool> {
    (elapsed < BLINK_S).then(|| ((elapsed.max(0.0) / BLINK_STEP_S) as u32).is_multiple_of(2))
}

/// What makes two entries the same place: band and mode.
pub(in crate::app) fn place_of(e: &BandStackEntry) -> (Band, Mode) {
    (Band::containing(e.freq_hz), e.mode)
}

/// Enter a place that has just been left. The same place already in the rows
/// is refreshed where it stands; a new one goes in front and the oldest
/// drops off. True when the rows changed.
pub(in crate::app) fn note_place(list: &mut Vec<BandStackEntry>, e: BandStackEntry) -> bool {
    if let Some(slot) = list.iter_mut().find(|x| place_of(x) == place_of(&e)) {
        if *slot == e {
            return false;
        }
        *slot = e;
        return true;
    }
    list.insert(0, e);
    list.truncate(SLOTS);
    true
}

/// The place being worked now, since when, and whether it is in the rows yet.
#[derive(Default)]
pub(in crate::app) struct WorkplaceTrack {
    since: f64,
    last: Option<BandStackEntry>,
    entered: bool,
}

impl WorkplaceTrack {
    /// Follow the receiver to `cur` at time `now`; returns what to enter in
    /// the rows. A place goes in once it has been worked `DWELL_S` — while
    /// still on it, so the place you are on shows at once (HDZ 2026-11-08:
    /// „da es diese Kombi noch nicht gibt, müsste sie eingetragen werden“) —
    /// and is refreshed with its last dial when it is left.
    pub(in crate::app) fn step(&mut self, cur: BandStackEntry, now: f64) -> Option<BandStackEntry> {
        let out = match self.last {
            Some(prev) if place_of(&prev) != place_of(&cur) => {
                let refresh = self.entered.then_some(prev);
                self.since = now;
                self.entered = false;
                refresh
            }
            None => {
                self.since = now;
                None
            }
            _ if !self.entered && now - self.since >= DWELL_S => {
                self.entered = true;
                Some(cur)
            }
            _ => None,
        };
        self.last = Some(cur);
        out
    }
}

impl SdroxideApp {
    /// Once a frame: enter the place being worked, or refresh the one left.
    pub(in crate::app) fn track_workplace(&mut self, ctx: &egui::Context, now: f64) {
        let rx = self.state.rx[0];
        let cur = BandStackEntry {
            freq_hz: self.state.active_freq_hz(),
            mode: rx.mode,
            filter_lo: rx.filter_lo,
            filter_hi: rx.filter_hi,
        };
        if let Some(e) = self.workplace_track.step(cur, now) {
            let new = !self.workplaces.iter().any(|x| place_of(x) == place_of(&e));
            if note_place(&mut self.workplaces, e) {
                crate::app::persist::save_workplaces(&self.workplaces);
            }
            if new {
                let (b, m) = place_of(&e);
                ctx.data_mut(|d| d.insert_temp(born_id(), (b, m, None::<f64>)));
            }
        }
    }
}

/// The rows, above BAND. Always every slot: an empty one is drawn dim and says
/// when it fills, because rows that only appeared after the first place had
/// been left were rows nobody found (HDZ 2026-11-08: „wo sind die buttons?“).
/// The slots share the width evenly, as many to a row as a label fits.
pub(in crate::app) fn workplace_rows(
    ui: &mut egui::Ui,
    list: &[BandStackEntry],
    here: (Band, Mode),
    vfo: Vfo,
    cmds: &mut Vec<Command>,
) {
    const SIZE: f32 = 11.0;
    crate::chrome::menu_caption(ui, &tr!("topbar-recent"));
    let label = |e: &BandStackEntry| {
        let (b, m) = place_of(e);
        format!("{} {}", b.label(), m.label())
    };
    // Band and mode only: the place is band and mode, and the dial it was
    // left on is in the tooltip (HDZ 2026-11-08: „die feine Frequenz ist nicht
    // nötig“). The widest label decides the columns, a long one of the usual
    // kind sets the floor, so the grid does not jump when an entry arrives.
    let widest = list
        .iter()
        .map(|e| crate::chrome::chip_width(ui, &label(e), Some(SIZE)))
        .fold(crate::chrome::chip_width(ui, "70CM SSTV-FM", Some(SIZE)), f32::max);
    let gap = 4.0;
    let avail = ui.available_width();
    let fit = (((avail + gap) / (widest + gap)).floor() as usize).max(1);
    // Columns that divide the slots, so every row is full at every window
    // size: 12 goes in 2, 3, 4 or 6 (HDZ: „durch 2, 3, 4 teilbar“).
    let cols = (1..=SLOTS).rev().find(|c| SLOTS.is_multiple_of(*c) && *c <= fit).unwrap_or(1);
    let w = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).floor();
    let size = egui::vec2(w, crate::chrome::chip_height(ui, Some(SIZE)));
    // The newborn's blink, started the first time it is drawn here.
    let now = ui.input(|i| i.time);
    let mut blink: Option<((Band, Mode), bool)> = None;
    if let Some((b, m, start)) =
        ui.ctx().data(|d| d.get_temp::<(Band, Mode, Option<f64>)>(born_id()))
    {
        let t0 = start.unwrap_or(now);
        match blink_inverted(now - t0) {
            Some(inv) => {
                if start.is_none() {
                    ui.ctx().data_mut(|d| d.insert_temp(born_id(), (b, m, Some(t0))));
                }
                blink = Some(((b, m), inv));
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
            }
            None => ui.ctx().data_mut(|d| d.remove::<(Band, Mode, Option<f64>)>(born_id())),
        }
    }
    for row in 0..SLOTS.div_ceil(cols) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for i in row * cols..((row + 1) * cols).min(SLOTS) {
                match list.get(i) {
                    Some(e) => {
                        let on = (place_of(e) == here)
                            ^ blink.is_some_and(|(p, inv)| p == place_of(e) && inv);
                        let text = RichText::new(label(e)).size(SIZE);
                        let resp = crate::chrome::chip_sized(ui, on, text, size)
                            .on_hover_text(format!("{:.3} MHz", e.freq_hz / 1e6));
                        if resp.clicked() {
                            cmds.push(Command::SetVfo { vfo, hz: e.freq_hz });
                            cmds.push(Command::SetMode { rx: RxId::Main, mode: e.mode });
                            cmds.push(Command::SetFilter {
                                rx: RxId::Main,
                                lo: e.filter_lo,
                                hi: e.filter_hi,
                            });
                        }
                    }
                    None => {
                        ui.add_enabled_ui(false, |ui| {
                            crate::chrome::chip_sized(
                                ui,
                                false,
                                RichText::new("· · ·").size(SIZE),
                                size,
                            )
                        })
                        .inner
                        .on_disabled_hover_text(tr!("topbar-recent-empty"));
                    }
                }
            }
        });
        ui.add_space(gap);
    }
    ui.add_space(2.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(hz: f64, mode: Mode) -> BandStackEntry {
        BandStackEntry { freq_hz: hz, mode, filter_lo: 100.0, filter_hi: 2800.0 }
    }

    #[test]
    fn a_newborn_place_flashes_three_times_then_rests() {
        let flips: Vec<bool> =
            (0..6).map(|k| blink_inverted(k as f64 * BLINK_STEP_S + 0.01).unwrap()).collect();
        assert_eq!(flips, [true, false, true, false, true, false]);
        assert_eq!(blink_inverted(BLINK_S + 0.01), None, "still blinking after the blink");
    }

    /// The blink starts when the rows are first drawn, not when the place came
    /// in, and is forgotten once it is over.
    #[test]
    fn the_blink_starts_when_drawn_and_is_then_forgotten() {
        let ctx = egui::Context::default();
        ctx.data_mut(|d| d.insert_temp(born_id(), (Band::M40, Mode::Ft8, None::<f64>)));
        let list = vec![at(7_074_000.0, Mode::Ft8)];
        let draw = |t: f64| {
            ctx.run_ui(egui::RawInput { time: Some(t), ..Default::default() }, |ui| {
                workplace_rows(ui, &list, (Band::M40, Mode::Ft8), Vfo::A, &mut Vec::new());
            })
            .drop_without_applying_deltas();
        };
        draw(100.0);
        let born = ctx.data(|d| d.get_temp::<(Band, Mode, Option<f64>)>(born_id()));
        assert_eq!(
            born,
            Some((Band::M40, Mode::Ft8, Some(100.0))),
            "the blink did not start on drawing"
        );
        draw(100.0 + BLINK_S + 0.1);
        let born = ctx.data(|d| d.get_temp::<(Band, Mode, Option<f64>)>(born_id()));
        assert_eq!(born, None, "the blink was not forgotten");
    }

    #[test]
    fn a_place_comes_back_refreshed_where_it_stands() {
        let mut l = Vec::new();
        note_place(&mut l, at(7_074_000.0, Mode::Ft8));
        note_place(&mut l, at(14_215_000.0, Mode::Usb));
        note_place(&mut l, at(3_700_000.0, Mode::Lsb));
        // 40 m FT8 again, a little higher: same place, same slot.
        assert!(note_place(&mut l, at(7_075_000.0, Mode::Ft8)));
        assert_eq!(l.len(), 3);
        assert_eq!(l[2].freq_hz, 7_075_000.0);
        assert_eq!(l[0].mode, Mode::Lsb);
    }

    #[test]
    fn the_rows_hold_twelve_and_the_oldest_drops() {
        let mut l = Vec::new();
        let hz = [1.84e6, 3.7e6, 7.1e6, 10.12e6, 14.2e6, 18.12e6, 21.25e6];
        // Seven bands in two modes each: fourteen places for twelve slots.
        for m in [Mode::Lsb, Mode::Usb] {
            for f in hz {
                note_place(&mut l, at(f, m));
            }
        }
        assert_eq!(l.len(), SLOTS);
        assert_eq!(place_of(&l[0]), (Band::M15, Mode::Usb));
        // The two oldest, 160 m and 80 m in LSB, have dropped off.
        assert!(!l.iter().any(|e| place_of(e) == (Band::M160, Mode::Lsb)));
        assert!(!l.iter().any(|e| place_of(e) == (Band::M80, Mode::Lsb)));
    }

    #[test]
    fn a_place_goes_in_while_it_is_worked_and_is_refreshed_on_leaving() {
        let mut t = WorkplaceTrack::default();
        assert_eq!(t.step(at(7_074_000.0, Mode::Ft8), 0.0), None);
        assert_eq!(t.step(at(7_074_000.0, Mode::Ft8), 2.0), None);
        // Three seconds on 40 m FT8: in, while still there, and only once.
        assert_eq!(t.step(at(7_074_000.0, Mode::Ft8), 3.0), Some(at(7_074_000.0, Mode::Ft8)));
        assert_eq!(t.step(at(7_075_000.0, Mode::Ft8), 9.0), None);
        // Leaving refreshes it with the last dial.
        assert_eq!(t.step(at(14_074_000.0, Mode::Ft8), 10.0), Some(at(7_075_000.0, Mode::Ft8)));
        // Clicked past 20 m within two seconds: never entered, nothing to refresh.
        assert_eq!(t.step(at(14_074_000.0, Mode::Ft8), 11.0), None);
        assert_eq!(t.step(at(21_074_000.0, Mode::Ft8), 12.0), None);
    }

    /// In a band menu as wide as eight band chips the twelve slots stand four
    /// to a row, equally wide, and none runs past the menu's edge.
    #[test]
    fn twelve_slots_fill_whole_rows() {
        let ctx = egui::Context::default();
        let list = vec![at(24_915_000.0, Mode::Ft8), at(7_155_000.0, Mode::Lsb)];
        let mut rects = Vec::new();
        let width = 795.0;
        let mut first = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 900.0),
                )),
                ..Default::default()
            },
            |ui| {
                ui.allocate_ui(egui::vec2(width, 600.0), |ui| {
                    ui.set_max_width(width);
                    let mut cmds = Vec::new();
                    workplace_rows(ui, &list, (Band::M12, Mode::Ft8), Vfo::A, &mut cmds);
                    rects.push(ui.min_rect());
                });
            },
        );
        first.textures_delta.clear();
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 900.0),
                )),
                ..Default::default()
            },
            |ui| {
                ui.allocate_ui(egui::vec2(width, 600.0), |ui| {
                    ui.set_max_width(width);
                    let mut cmds = Vec::new();
                    workplace_rows(ui, &list, (Band::M12, Mode::Ft8), Vfo::A, &mut cmds);
                });
            },
        );
        out.textures_delta.clear();
        let mut texts: Vec<egui::Rect> = out
            .shapes
            .iter()
            .filter_map(|c| match &c.shape {
                egui::epaint::Shape::Text(t)
                    if t.galley.text().contains('·')
                        || t.galley.text().contains("12M")
                        || t.galley.text().contains("40M") =>
                {
                    Some(t.visual_bounding_rect())
                }
                _ => None,
            })
            .collect();
        texts.sort_by(|a, b| a.top().partial_cmp(&b.top()).unwrap());
        let mut rows: Vec<f32> = texts.iter().map(|r| r.top().round()).collect();
        rows.dedup();
        assert_eq!(texts.len(), SLOTS, "twelve slots");
        assert!(SLOTS % rows.len() == 0 && rows.len() <= 3, "full rows, at most three: {rows:?}");
        assert!(texts.iter().all(|r| r.right() <= width + 1.0), "{texts:?}");
        let _ = rects;
    }
}
