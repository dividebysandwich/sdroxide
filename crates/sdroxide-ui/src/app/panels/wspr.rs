//! The WSPR panel: what got through, where it went, and what the beacon is
//! doing.
//!
//! Three panes. `SPOTS` is the list of receptions — one row per beacon heard,
//! or per station that heard *us* once the WSPRnet download is on. `MAP` is the
//! same world map the FT8 panel draws, so the two modes place stations
//! identically, with the propagation-heat controls above it. `STATUS` is the
//! beacon itself: the slot clock, the transmit duty cycle, the band-hop
//! schedule and what is being reported to WSPRnet.
//!
//! Wide enough, the receptions run full height down the left and the map takes
//! the top right with the status under it — the map is what an operator glances
//! at, so it goes where a glance lands. Narrower, the tab row picks one.
//!
//! There is no QSO pane because there is no QSO. Every control here is about
//! measurement rather than contact, which is the whole difference between this
//! mode and the one next to it.

use eframe::egui::{self, Color32, RichText};
use sdroxide_types::{Band, Command, WSPR_SLOT_S, WsprSpot, power_label};

use crate::app::panels::widgets::{SlotState, row_cell, slot_bar, slot_phase_s};
use crate::app::util::fmt_age;
use crate::app::{SdroxideApp, rx_only_hint};
use crate::chrome::StyledCombo;
use crate::theme::ThemedScroll;
use crate::time::{now_unix, now_unix_f64};

/// How many receptions the panel keeps.
///
/// A full night rather than the two hours 400 rows used to cover. WSPR is the
/// mode people leave running unattended and read back in the morning, and a
/// list that stopped counting after two hours read as the decoder having given
/// up (issue #316) — the counter beside the heading is the number of rows, so a
/// cap is indistinguishable from a station that went deaf.
///
/// Affordable because the list is virtualised — [`SdroxideApp::wspr_spot_list`]
/// lays out only the rows the viewport covers — and because the arrival path
/// de-duplicates through a set rather than a scan per spot. Without both, this
/// number is a frame-rate setting.
pub(in crate::app) const WSPR_SPOT_ROWS: usize = 5_000;

/// The height every cell in a reception row is built to.
const WSPR_ROW_H: f32 = 19.0;

/// The painted height of one reception row, excluding the spacing between them.
///
/// `ScrollArea::show_rows` works out which rows the viewport covers from this
/// number alone, without laying any of them out, so a row that is not really
/// this tall drifts further from its slot the further down the list you go. It
/// must NOT include `item_spacing.y`, which `show_rows` adds back itself — the
/// same trap the logbook's `row_height` documents at length.
///
/// A row is the frame's 3-point top and bottom margins around a run of
/// [`row_cell`]s built to [`WSPR_ROW_H`] — except that egui never lays a row out
/// shorter than the style's interaction size, which `theme::apply_metrics` sets
/// to 18 points on desktop and 34 on a touch tier and RE-APPLIES whenever the
/// window crosses the breakpoint. So this is a function of the live style and
/// not a constant: 25 points on desktop, 40 on a phone. With a constant,
/// resizing across that breakpoint would silently desynchronise the scroll bar
/// from the list — which is what `the_reception_row_fits_its_slot` catches.
fn wspr_row_slot(ui: &egui::Ui) -> f32 {
    WSPR_ROW_H.max(ui.spacing().interact_size.y) + 6.0
}

/// A WSPR beacon is heard every few minutes at best — a duty cycle of twenty
/// per cent means a given station transmits five times an hour. A map that
/// emptied after the FT8 fade would be blank almost always, which reads as the
/// mode not working rather than as the band being quiet. Ten minutes is about
/// three chances to be heard again.
const WSPR_STATION_FADE_S: f64 = 600.0;

impl SdroxideApp {
    pub(in crate::app) fn wspr_panel(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        panel_h: f32,
    ) {
        let pane = self.digi_pane(sdroxide_types::Mode::Wspr);
        let phone = crate::layout::tier(ui.ctx()) == crate::layout::Tier::Phone;
        // Two columns need room for two: below this the tab row picks one, the
        // way a phone does. Squeezing both into 400 points gives a reception
        // list too narrow to read a callsign in and a status pane too narrow to
        // read a frequency in.
        const TWO_COLUMN_MIN_W: f32 = 520.0;
        if phone || ui.available_width() < TWO_COLUMN_MIN_W {
            match pane {
                0 => self.wspr_spot_list(ui, panel_h),
                1 => {
                    self.wspr_map(ui, panel_h);
                }
                _ => self.wspr_status_scroll(ui, cmds),
            }
            return;
        }
        // Two columns, each an explicit vertical `Ui`.
        //
        // `allocate_ui` inherits the *parent's* layout, so building the columns
        // with it inside `horizontal_top` lays their contents out left-to-right
        // as well — every reception row ends up on one line on top of the next.
        // `ui.vertical` is what turns the direction back, and it is why every
        // other split panel here is written this way.
        //
        // The receptions run full height down the left, because that is the
        // list you scroll; the map takes the top right, where it is the first
        // thing in view, with the beacon's own state under it.
        let total_w = ui.available_width();
        // The upper bound is floored at the lower one: on a window too narrow
        // for both halves it drops below it, and `clamp` panics on min > max.
        let left_w =
            (total_w * self.view.digi_split_fraction).clamp(240.0, (total_w - 260.0).max(240.0));
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(left_w);
                ui.set_height(panel_h);
                self.wspr_spot_list(ui, panel_h);
            });
            let resp = crate::chrome::split_handle(ui, egui::vec2(7.0, panel_h), None);
            if resp.dragged() {
                let dx = resp.drag_delta().x;
                self.view.digi_split_fraction = ((left_w + dx) / total_w).clamp(0.25, 0.75);
            }
            ui.vertical(|ui| {
                ui.set_height(panel_h);
                let map_h = self.wspr_map(ui, panel_h);
                if map_h > 0.0 {
                    ui.add_space(6.0);
                }
                self.wspr_status_scroll(ui, cmds);
            });
        });
    }

    /// The world map and the propagation-heat controls that belong with it.
    ///
    /// Returns the height it took, so the caller knows whether to leave a gap.
    /// Zero when the panel is too short for a map — one two rows tall is not a
    /// map, and the beacon's state is worth more than a smear.
    fn wspr_map(&mut self, ui: &mut egui::Ui, panel_h: f32) -> f32 {
        let map_lo = crate::widgets::worldmap::MIN_HEIGHT;
        if panel_h < map_lo * 2.0 {
            return 0.0;
        }
        let now = now_unix();
        let my_grid =
            self.digi_status.as_ref().map(|s| s.config.my_grid.clone()).unwrap_or_default();
        let home = sdroxide_types::grid_to_latlon(&my_grid);

        // The map gets a draggable share of the column, with the beacon's state
        // keeping the rest.
        let map_hi = (panel_h * 0.62).min(ui.available_width()).max(map_lo);
        let map_budget = map_lo + (map_hi - map_lo) * self.view.digi_map_fraction;

        // The heat controls live with the map rather than tucked under the
        // list, because they are controls *for the map* — put anywhere else
        // they read as unrelated chips and go unfound.
        self.prop_map_controls(ui);

        let now_t = ui.input(|i| i.time);
        self.digi_stations.set_fade_s(WSPR_STATION_FADE_S);
        self.digi_stations.observe_wspr(&self.wspr_spots, now_t, now);
        let stations = self.digi_stations.stations(now_t);
        let heat = self.prop_texture(ui.ctx(), self.state.rx_freq_hz());
        crate::widgets::worldmap::show(
            ui,
            &mut self.map_view,
            home,
            None,
            None,
            self.digi_hover_ll,
            &stations,
            &[],
            heat,
            self.digi_status.as_ref().map(|s| s.transmitting).unwrap_or(false),
            map_budget,
        );
        let legend = wspr_band_legend(ui, &stations);
        // Draggable edge under the map, as the FT8 panel has.
        let resp = crate::chrome::split_handle(ui, egui::vec2(ui.available_width(), 7.0), None);
        if resp.dragged() {
            let df = resp.drag_delta().y / (map_hi - map_lo).max(1.0);
            self.view.digi_map_fraction = (self.view.digi_map_fraction + df).clamp(0.0, 1.0);
        }
        map_budget + crate::chrome::chip_height(ui, Some(9.5)) + legend + 11.0
    }

    /// The reception list, filling its column.
    fn wspr_spot_list(&mut self, ui: &mut egui::Ui, _panel_h: f32) {
        let now = now_unix();
        let my_grid =
            self.digi_status.as_ref().map(|s| s.config.my_grid.clone()).unwrap_or_default();
        let home = sdroxide_types::grid_to_latlon(&my_grid);

        ui.horizontal(|ui| {
            ui.label(
                RichText::new("RECEPTIONS").size(9.5).strong().color(crate::theme::CYAN_DIM()),
            );
            crate::chrome::row_tail(ui, |ui| {
                let heard_us = self.wspr_spots.iter().filter(|s| s.is_heard_by_other()).count();
                let label = if heard_us > 0 {
                    format!("{} rx · {heard_us} heard us", self.wspr_spots.len() - heard_us)
                } else {
                    format!("{} rx", self.wspr_spots.len())
                };
                ui.label(RichText::new(label).size(10.0).color(crate::theme::gray(120)));
            });
        });

        // Virtualised: only the rows the viewport covers are laid out, so a
        // night of receptions costs the same per frame as an hour of them.
        let spots = std::mem::take(&mut self.wspr_spots);
        egui::ScrollArea::vertical()
            .id_salt("wspr-receptions")
            .max_height(ui.available_height())
            .auto_shrink([false, false])
            .show_rows_themed(ui, wspr_row_slot(ui), spots.len(), |ui, rows| {
                for s in &spots[rows] {
                    wspr_row(ui, s, home, now);
                }
            });
        self.wspr_spots = spots;
    }

    /// The beacon's own state, and the two settings an operator reaches for
    /// most: whether it transmits, and whether it moves between bands.
    /// [`Self::wspr_status_pane`] with a scrollbar under it.
    ///
    /// The beacon's state is a fixed stack of rows — the band, the transmit
    /// switch, the duty cycle, the power — and none of them can be dropped
    /// without dropping a control. On a screen with less height than the stack
    /// needs they used to run off the bottom edge and simply not be there: a
    /// 1366×768 laptop, which is the tablet tier with its taller rows, lost the
    /// transmit controls outright (issue #231). Scrolled, the same stack is all
    /// still reachable, and on a screen with the room the bar never appears.
    fn wspr_status_scroll(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        egui::ScrollArea::vertical()
            .id_salt("wspr-status")
            .auto_shrink([false, false])
            .show_themed(ui, |ui| self.wspr_status_pane(ui, cmds));
    }

    fn wspr_status_pane(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let status = self.digi_status.clone();
        let w = status.as_ref().and_then(|s| s.wspr.clone()).unwrap_or_default();
        let transmitting = status.as_ref().map(|s| s.transmitting).unwrap_or(false);
        // What the *radio* is set to, not what this window last asked for.
        //
        // The chips below edit `digi_cfg_edit` and send a command; the engine
        // echoes its own config back in the status. Reading the edit buffer for
        // the readouts made the panel report the operator's intention as though
        // it were the station's state — so a setting that never reached the
        // engine looked applied, and the duty cycle could sit there reading 50%
        // beside an engine-supplied "not this slot" that would never change.
        let live = status.as_ref().map(|s| s.config.clone()).unwrap_or_default();

        // The engine's `slot_utc` is the slot whose audio is actually being
        // recorded, and it anchors the bar whenever this window's clock agrees
        // with it — WSPR is the one mode whose status carries it. `slot_bar`
        // asks for the frames that keep the clock moving, which is what keeps
        // the countdown beside it ticking too.
        let timing = sdroxide_types::Mode::Wspr.slot_timing().expect("WSPR is slotted");
        let into_slot = slot_phase_s(now_unix_f64(), timing.slot_s, w.slot_utc);

        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("WSPR").size(11.0).strong().color(crate::theme::CYAN()));
            ui.label(
                RichText::new("weak-signal propagation beacon")
                    .size(10.5)
                    .color(crate::theme::CYAN_DIM()),
            );
            // WSPR has one agreed dial per band and thirteen bands, and a
            // beacon operator moves between them by hand — so the list is
            // exactly the thing that was otherwise looked up (issue #210).
            self.digi_freq_chip(ui, cmds);
            crate::chrome::row_tail(ui, |ui| {
                // What is left of the slot, at the end of the row where the eye
                // comes back to it. It is the one figure here that means
                // something in every slot — receiving, transmitting or
                // decoding — so it is not conditional on any of them.
                let left = (WSPR_SLOT_S - into_slot).max(0.0).round() as i64;
                ui.label(
                    RichText::new(format!("{}:{:02}", left / 60, left % 60))
                        .size(10.5)
                        .color(crate::theme::gray(140)),
                )
                .on_hover_text(
                    "Time left in this two-minute slot. At the end of it the recording goes to \
                     the decoder and the next transmit decision lands.",
                );
                if transmitting {
                    ui.label(
                        RichText::new("● TX").size(11.0).strong().color(crate::theme::ALERT()),
                    );
                } else if w.decoding {
                    ui.label(RichText::new("decoding…").size(10.5).color(crate::theme::YELLOW()));
                }
            });
        });
        ui.add_space(4.0);
        slot_bar(
            ui,
            timing,
            into_slot,
            if transmitting {
                SlotState::Transmitting
            } else if w.decoding {
                SlotState::Decoding
            } else {
                SlotState::Listening
            },
        );
        ui.add_space(6.0);

        crate::chrome::red_panel(ui, |ui| {
            let dial = self.state.rx_freq_hz();
            row(ui, "Band", &format!("{} · {:.6} MHz", Band::containing(dial).label(), dial / 1e6));
            row(
                ui,
                "Last slot",
                &if w.decoding {
                    "decoding…".to_string()
                } else if w.last_slot_spots == 0 {
                    "nothing heard".to_string()
                } else {
                    format!(
                        "{} beacon{}",
                        w.last_slot_spots,
                        if w.last_slot_spots == 1 { "" } else { "s" }
                    )
                },
            );
            // Whether the *next* slot transmits is decided before it starts, so
            // this is a promise rather than a guess — see the controller's duty
            // cycle.
            let tx_txt = if live.wspr_tx_percent == 0 {
                "receive only".to_string()
            } else if w.tx_next {
                format!(
                    "beaconing — {}, {}% duty",
                    power_label(live.wspr_power_dbm),
                    live.wspr_tx_percent
                )
            } else {
                format!("listening ({}% duty)", live.wspr_tx_percent)
            };
            row(ui, "Next slot", &tx_txt);
            // The one place the two copies can be seen to differ. A command
            // that did not reach the engine is otherwise invisible: the chips
            // would show the new setting and the beacon would keep to the old.
            if live.wspr_tx_percent != self.digi_cfg_edit.wspr_tx_percent
                || live.wspr_power_dbm != self.digi_cfg_edit.wspr_power_dbm
            {
                ui.label(
                    RichText::new("waiting for the radio to take the change…")
                        .size(10.0)
                        .color(crate::theme::YELLOW()),
                );
            }
            if let Some(hz) = w.next_dial_hz {
                row(
                    ui,
                    "Hop next",
                    &format!("{} · {:.4} MHz", Band::containing(hz).label(), hz / 1e6),
                );
            }
            if let Some(why) = &w.hop_blocked {
                ui.label(RichText::new(why).size(10.0).color(crate::theme::YELLOW()));
            }
        });

        ui.add_space(8.0);
        // Transmit, where the operator is already looking.
        //
        // Whether an unattended beacon is on the air is the single thing about
        // this mode worth being able to see and change at a glance, so it is a
        // row of chips here and not in a dialog. The duty cycle is the setting
        // *and* the switch: nought is receive-only, so there is no second
        // control that could disagree with it about what this station is doing.
        // A receiver keeps OFF — the duty cycle is the switch as well as the
        // setting, so leaving the resting state live is what lets a beacon that
        // was on be turned off after the radio was swapped for one that cannot
        // key. The percentages go grey.
        let tx_ok = self.tx_capable();
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.label(RichText::new("TRANSMIT").size(9.5).color(crate::theme::CYAN_DIM()));
            let cur = self.digi_cfg_edit.wspr_tx_percent;
            for (pct, label) in [(0u8, "OFF"), (10, "10%"), (20, "20%"), (33, "33%"), (50, "50%")] {
                // Off is the resting state and reads as one; anything else puts
                // a carrier on the air, so it is accented like the other
                // controls in this program that do.
                let resp = if pct == 0 {
                    crate::chrome::chip(ui, cur == 0, RichText::new(label).size(10.5))
                } else {
                    rx_only_hint(
                        crate::chrome::chip_accent_enabled(
                            ui,
                            tx_ok,
                            cur == pct,
                            label,
                            Some(10.5),
                            crate::theme::PINK(),
                            crate::theme::INK_ON_CYAN(),
                        ),
                        tx_ok,
                    )
                };
                if resp
                    .on_hover_text(if pct == 0 {
                        "Receive only.".to_string()
                    } else {
                        format!(
                            "Beacon in one two-minute slot out of every {} — {pct}% of them, \
                             evenly spaced — at {}. Which slot the cycle starts on is drawn from \
                             your callsign, so two stations running this program do not start \
                             together.",
                            (100 + pct as u32 / 2) / pct as u32,
                            power_label(self.digi_cfg_edit.wspr_power_dbm),
                        )
                    })
                    .clicked()
                    && cur != pct
                    && self.digi_cfg_seeded
                {
                    self.digi_cfg_edit.wspr_tx_percent = pct;
                    cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
                }
            }
        });

        // Power, in the unit an operator sets their radio in.
        //
        // A picker rather than a free number: WSPR's fifty-bit message can name
        // exactly nineteen levels, so anything else would have to be rounded
        // silently — and the figure goes out on the air, where everybody who
        // hears it uses it to judge the path. Offering only what can be said is
        // what keeps the announcement true.
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.label(RichText::new("POWER").size(9.5).color(crate::theme::CYAN_DIM()));
            let cur = sdroxide_types::round_power_dbm(self.digi_cfg_edit.wspr_power_dbm);
            let mut chosen = None;
            egui::ComboBox::from_id_salt("wspr-power")
                .selected_text(RichText::new(power_label(cur)).size(10.5))
                .width(84.0)
                .show_styled(ui, |ui| {
                    for p in sdroxide_types::WSPR_POWERS_DBM {
                        if ui.selectable_label(cur == p, power_label(p)).clicked() {
                            chosen = Some(p);
                        }
                    }
                });
            if let Some(p) = chosen
                && p != self.digi_cfg_edit.wspr_power_dbm
                && self.digi_cfg_seeded
            {
                self.digi_cfg_edit.wspr_power_dbm = p;
                cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
            }
            ui.label(
                RichText::new("what you actually radiate").size(9.5).color(crate::theme::gray(110)),
            )
            .on_hover_text(
                "This goes out in the message, and everyone who hears you judges the path by \
                 it — so an optimistic figure here makes their measurements wrong as well as \
                 yours.",
            );

            // Moving inside the 200 Hz window is the WSPR convention, and it is
            // the only other thing about a transmission worth a switch.
            let auto = self.digi_cfg_edit.auto_tx_freq;
            let resp = crate::chrome::chip(ui, auto, RichText::new("ROAM").size(10.5));
            if resp.clicked() && self.digi_cfg_seeded {
                self.digi_cfg_edit.auto_tx_freq = !auto;
                cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
            }
            resp.on_hover_text(
                "Pick a different offset inside the 200 Hz window for every transmission. \
                 This is the convention: two hundred hertz shared by everyone only works if \
                 nobody parks in the middle of it. Off holds where the cursor is.",
            );
        });

        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let hop = self.digi_cfg_edit.wspr_hop;
            let resp = crate::chrome::chip(ui, hop, RichText::new("BAND HOP").size(10.5));
            if resp.clicked() && self.digi_cfg_seeded {
                self.digi_cfg_edit.wspr_hop = !hop;
                cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
            }
            resp.on_hover_text(
                "Move the dial from band to band between slots, so one receiver samples the \
                 whole spectrum instead of one slice of it. Turning the VFO yourself pauses \
                 it — press this twice to resume.",
            );

            let up = self.net_cfg_edit.wspr.upload;
            let resp = crate::chrome::chip(ui, up, RichText::new("UPLOAD").size(10.5));
            if resp.clicked() && self.net_cfg_seeded {
                self.net_cfg_edit.wspr.upload = !up;
                cmds.push(Command::SetNetworkConfig(self.net_cfg_edit.clone()));
            }
            resp.on_hover_text(
                "Send what this station decodes to wsprnet.org. Reporting what you hear is \
                 what makes a WSPR receiver part of the network; it puts nothing on the air.",
            );

            let dl = self.net_cfg_edit.wspr.download_heard_us;
            let resp = crate::chrome::chip(ui, dl, RichText::new("WHO HEARD ME").size(10.5));
            if resp.clicked() && self.net_cfg_seeded {
                self.net_cfg_edit.wspr.download_heard_us = !dl;
                cmds.push(Command::SetNetworkConfig(self.net_cfg_edit.clone()));
            }
            resp.on_hover_text(
                "Ask wsprnet.org every few minutes which stations decoded this one. WSPR has \
                 no acknowledgement of any kind, so this is the only way a beacon learns \
                 anything about its own reach.",
            );
        });

        // The bands the hop cycle visits, shown only when it is running: a row
        // of nine chips is a lot to carry for a station that is staying put.
        if self.digi_cfg_edit.wspr_hop {
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                ui.label(RichText::new("BANDS").size(9.5).color(crate::theme::CYAN_DIM()));
                let mut mask = self.digi_cfg_edit.wspr_hop_bands;
                let mut hit = false;
                // Only bands with a WSPR dial: the rest have nothing to hop to.
                for (i, b) in Band::ALL.iter().enumerate() {
                    if !sdroxide_types::WSPR_DIALS.iter().any(|&hz| Band::containing(hz) == *b) {
                        continue;
                    }
                    let bit = 1u16 << i;
                    if crate::chrome::chip(ui, mask & bit != 0, RichText::new(b.label()).size(9.5))
                        .clicked()
                    {
                        mask ^= bit;
                        hit = true;
                    }
                }
                if hit && self.digi_cfg_seeded {
                    self.digi_cfg_edit.wspr_hop_bands = mask;
                    cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
                }
                if mask == 0 {
                    ui.label(
                        RichText::new("none selected — hopping has nowhere to go")
                            .size(9.5)
                            .color(crate::theme::YELLOW()),
                    );
                }
            });
        }

        ui.add_space(6.0);
        // The identity is the station's, not this mode's, so it is not asked
        // for twice — this only says where it is kept.
        let call = self.digi_cfg_edit.my_call.trim();
        let grid = self.digi_cfg_edit.my_grid.trim();
        ui.label(
            RichText::new(if call.is_empty() || grid.is_empty() {
                "Set your callsign and grid on the General tab of Settings before transmitting."
                    .to_string()
            } else {
                // The locator as it will go out: a six-character one is sent as
                // its first four, and saying so here is less surprising than
                // letting the operator find it on a spot page.
                let sent = sdroxide_types::wspr_grid4(grid).unwrap_or_else(|| grid.to_string());
                format!("Transmitting as {call} in {sent} — set on the General tab of Settings.")
            })
            .size(10.0)
            .color(if call.is_empty() || grid.is_empty() {
                crate::theme::YELLOW()
            } else {
                crate::theme::gray(110)
            }),
        );
        // The engine's verdict, not the panel's guess: it is whatever the
        // message packer actually refused, so it can never disagree with what
        // the transmitter does. Shown only when transmitting is asked for —
        // a receive-only station has nothing to fix.
        if self.digi_cfg_edit.wspr_tx_percent > 0
            && let Some(why) = &w.tx_blocked
        {
            ui.add_space(4.0);
            ui.label(RichText::new(why).size(10.0).color(crate::theme::YELLOW()));
        }
    }
}

/// Which band each dot's hue means, for the bands actually on the map.
///
/// The dots are coloured by band because a hopping beacon puts several on one
/// picture (issue #316), and a colour nobody can name is decoration. Built from
/// the stations themselves rather than from the band-hop schedule, so it lists
/// what is on screen now — including bands heard before the beacon moved on,
/// which are exactly the ones the schedule no longer mentions.
///
/// Returns the height it took, so the map's caller can account for it. Nothing
/// at all when there is only one band up: a legend of one is a label for a
/// picture that has nothing to distinguish.
fn wspr_band_legend(ui: &mut egui::Ui, stations: &[crate::digi_map::DigiStation]) -> f32 {
    let mut bands: Vec<Band> = stations.iter().filter_map(|s| s.band).collect();
    bands.sort_by_key(|b| b.index());
    bands.dedup();
    if bands.len() < 2 {
        return 0.0;
    }
    let before = ui.cursor().top();
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        for b in bands {
            let [r, g, bl] = crate::colormap::band_color(b);
            ui.label(
                RichText::new(format!("● {}", b.label()))
                    .size(9.0)
                    .color(crate::theme::data_ink((r, g, bl))),
            );
        }
    });
    (ui.cursor().top() - before).max(0.0)
}

/// One reception. Reads left to right the way the operator asks the question:
/// who, from where, how far, how well, on how much power.
fn wspr_row(ui: &mut egui::Ui, s: &WsprSpot, home: Option<(f64, f64)>, now: i64) {
    let heard_us = s.is_heard_by_other();
    // The station at the far end of the path: the beacon we decoded, or the
    // station that decoded us.
    let (who, where_) = if heard_us {
        (s.reporter.as_deref().unwrap_or("?"), s.reporter_grid.as_deref())
    } else {
        (s.call.as_str(), s.grid.as_deref())
    };
    let km = home
        .zip(where_.and_then(sdroxide_types::grid_to_latlon))
        .map(|(a, b)| sdroxide_types::distance_km(a, b));

    let bg = if heard_us { crate::theme::TOME_BG() } else { crate::theme::ROW_BG() };
    egui::Frame::new()
        .fill(bg)
        .inner_margin(egui::Margin { left: 6, right: 6, top: 3, bottom: 3 })
        .show(ui, |ui| {
            // Fixed-width columns, so the list reads down the page as well as
            // across it: a callsign and a distance that shuffle sideways from
            // row to row are far harder to scan than ones that line up.
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.set_min_height(WSPR_ROW_H);
                ui.spacing_mut().item_spacing.x = 5.0;
                // An arrow, because the direction is the whole meaning of the
                // row: ← we heard them, → they heard us.
                row_cell(
                    ui,
                    10.0,
                    WSPR_ROW_H,
                    false,
                    egui::Label::new(
                        RichText::new(if heard_us { "→" } else { "←" }).size(11.0).strong().color(
                            if heard_us { crate::theme::YELLOW() } else { crate::theme::CYAN() },
                        ),
                    ),
                );
                row_cell(
                    ui,
                    74.0,
                    WSPR_ROW_H,
                    false,
                    egui::Label::new(
                        RichText::new(who).size(11.0).strong().color(crate::theme::TEXT_STRONG()),
                    )
                    .truncate(),
                );
                row_cell(
                    ui,
                    46.0,
                    WSPR_ROW_H,
                    false,
                    egui::Label::new(
                        RichText::new(where_.unwrap_or(""))
                            .size(10.0)
                            .color(crate::theme::CYAN_DIM()),
                    )
                    .truncate(),
                );
                row_cell(
                    ui,
                    40.0,
                    WSPR_ROW_H,
                    true,
                    egui::Label::new(
                        RichText::new(format!("{:+} dB", s.snr_db))
                            .size(10.5)
                            .strong()
                            .color(snr_color(s.snr_db)),
                    ),
                );
                row_cell(
                    ui,
                    52.0,
                    WSPR_ROW_H,
                    true,
                    egui::Label::new(
                        RichText::new(km.map(|k| format!("{k:.0} km")).unwrap_or_default())
                            .size(9.5)
                            .color(crate::theme::gray(150)),
                    ),
                );
                row_cell(
                    ui,
                    50.0,
                    WSPR_ROW_H,
                    true,
                    egui::Label::new(
                        RichText::new(power_label(s.power_dbm))
                            .size(9.5)
                            .color(crate::theme::gray(130)),
                    ),
                );
                // When, in UTC, which is the only clock this mode has.
                //
                // A WSPR slot *is* a two-minute block of UTC, so the timestamp
                // is exact rather than a rounding of one — and it is what every
                // other client in the network prints beside a spot, so it is
                // the thing an operator compares against WSJT-X or a WSPRnet
                // page to see whether the two heard the same transmission. The
                // relative age it replaces could not be compared with anything.
                //
                // It goes last and takes what is left, so a narrow panel drops
                // it rather than squeezing the columns that matter.
                crate::chrome::row_tail(ui, |ui| {
                    let t = s.slot_utc.rem_euclid(86_400);
                    ui.label(
                        RichText::new(format!("{:02}:{:02}", t / 3600, (t % 3600) / 60))
                            .size(9.5)
                            .monospace()
                            .color(crate::theme::gray(120)),
                    )
                    .on_hover_text(format!(
                        "{:02}:{:02} UTC — {} ago",
                        t / 3600,
                        (t % 3600) / 60,
                        fmt_age(now - s.slot_utc)
                    ));
                });
            });
        });
}

/// Colour a WSPR report by how much margin it had.
///
/// The scale is WSPR's, not FT8's: −29 dB is the decode floor here, so −20 is a
/// good path and anything above −10 is a strong one. Using FT8's thresholds
/// would paint the whole band red.
fn snr_color(db: i16) -> Color32 {
    match db {
        d if d >= -10 => crate::theme::GREEN(),
        d if d >= -20 => crate::theme::CYAN(),
        d if d >= -26 => crate::theme::YELLOW(),
        _ => crate::theme::PINK(),
    }
}

/// A label/value line in the status card.
fn row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(10.0).color(crate::theme::CYAN_DIM()));
        crate::chrome::row_tail(ui, |ui| {
            ui.label(RichText::new(value).size(10.5).color(crate::theme::TEXT()));
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spot() -> WsprSpot {
        WsprSpot {
            slot_utc: 1_784_937_600,
            call: "G4MQL".into(),
            grid: Some("IO91".into()),
            power_dbm: 23,
            freq_hz: 14_097_100.0,
            snr_db: -22,
            dt: 0.3,
            drift_hz: 0.0,
            reporter: None,
            reporter_grid: None,
        }
    }

    /// `ScrollArea::show_rows` places every reception from [`wspr_row_slot`]
    /// alone, without laying any of them out — so a row that is not really that
    /// tall drifts further out of its slot the further down the list you scroll,
    /// and the scroll bar lies about where it is.
    ///
    /// Both tiers, because `theme::apply_metrics` gives them different
    /// `interact_size.y` and re-applies it whenever the window crosses the
    /// breakpoint, so the slot is 25 points on desktop and 40 on a phone.
    #[test]
    fn the_reception_row_fits_its_slot() {
        for tier in [crate::layout::Tier::Desktop, crate::layout::Tier::Phone] {
            let ctx = egui::Context::default();
            crate::theme::apply_metrics(&ctx, tier);
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(420.0, 400.0));
            let (s, home) = (spot(), Some((51.5, -0.1)));
            let (mut h, mut slot) = (0.0f32, 0.0f32);
            ctx.run_ui(egui::RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
                // The painted extent, NOT the cursor advance: the advance
                // includes the item spacing `show_rows` adds back itself.
                slot = wspr_row_slot(ui);
                h = ui.scope(|ui| wspr_row(ui, &s, home, s.slot_utc)).response.rect.height();
            })
            .drop_without_applying_deltas();
            assert!(
                (h - slot).abs() < 0.5,
                "a {tier:?} reception row paints {h} points into a {slot}-point slot; \
                 the virtualised list would drift"
            );
        }
    }

    /// The colours are read at a glance to mean "how good was that path", so
    /// they have to follow WSPR's floor rather than FT8's.
    #[test]
    fn the_report_colour_follows_wsprs_own_scale() {
        assert_eq!(snr_color(-5), crate::theme::GREEN());
        assert_eq!(snr_color(-18), crate::theme::CYAN());
        assert_eq!(snr_color(-24), crate::theme::YELLOW());
        // Below the nominal floor: remarkable, and coloured as such.
        assert_eq!(snr_color(-28), crate::theme::PINK());
    }
}
