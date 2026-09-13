//! The JS8Call panel: heard list, conversation and composer.
//!
//! JS8 has no QSO sequencer, so the panel is built around the message log
//! instead: what each station last said, what a reply to it would be, and who
//! the composer is addressing. Locators mostly never arrive on the air — only
//! heartbeats and CQs carry one — so heard stations are looked up over HTTP,
//! one at a time, to give the map something to place them by.

use eframe::egui::{self, Color32, RichText};
use sdroxide_types::{Command, Decode};

use crate::theme::ThemedScroll;
use crate::time::now_unix;

use crate::app::panels::widgets::{row_cell, row_cell_ui, snr_color, station_card};
use crate::app::{SdroxideApp, rx_only_hint, tx_gated};

/// Roughly how many JS8 frames a message will take.
///
/// The panel shows this before the operator presses send, because JS8's most
/// surprising property to someone new is that a sentence can occupy a minute of
/// air time. It is an estimate: the real encoder chooses per frame between
/// Huffman and dictionary compression, so the true count is often lower. Being
/// pessimistic here is the right way round — a message that finishes early is a
/// pleasant surprise, one that runs long is not.
fn js8_frame_estimate(text: &str) -> u8 {
    const PER_FRAME: usize = 13;
    let n = text.trim().len();
    (n.div_ceil(PER_FRAME).max(1)).min(255) as u8
}

/// How long a JS8 station stays lit on the maps after it was last heard.
///
/// The mode's own convention is a heartbeat every ten or fifteen minutes, so
/// FT8's two-minute fade would leave the map blank between them.
const JS8_STATION_FADE_S: f64 = 900.0;

/// True when a message was aimed at *us* — our callsign, or a group we joined —
/// as opposed to at the whole band.
///
/// `@ALLCALL` reaches us and the assembler marks it `to_me` accordingly, but a
/// heartbeat is not addressed to anyone in particular: colouring every one of
/// them gold would leave nothing for a real call to stand out against.
fn js8_personally_addressed(m: &sdroxide_types::Js8Msg) -> bool {
    m.to_me && !m.to.eq_ignore_ascii_case("@ALLCALL")
}

/// What the composer can quote about our own station when it drafts a reply.
struct Js8Me {
    grid: String,
    status: String,
    /// Callsigns heard recently, most recent first — the answer to `HEARING?`.
    hearing: Vec<String>,
    /// The last thing we transmitted, which is what `AGN?` is asking for.
    last_sent: String,
}

/// One heard station as a [`Decode`], so the FT8 hover card can describe it
/// without learning a second station type.
fn js8_station_decode(
    h: &sdroxide_types::Js8Heard,
    grid: Option<String>,
    msg: Option<&sdroxide_types::Js8Msg>,
) -> Decode {
    Decode {
        slot_utc: h.last_utc,
        snr_db: h.snr_db,
        dt: 0.0,
        audio_hz: h.audio_hz,
        message: msg.map(js8_msg_summary).unwrap_or_default(),
        to: msg.map(|m| m.to.clone()).filter(|t| !t.is_empty()),
        from: Some(h.call.clone()),
        grid,
        is_cq: msg.is_some_and(|m| m.cmd.as_deref() == Some("CQ")),
        cq_to: None,
        rr73_to: None,
        free_text: false,
    }
}

/// A heard station's last transmission on one line: who it was for, then the
/// command, then the text.
///
/// The recipient comes first because without it a directed message is
/// ambiguous — on a busy channel half a dozen stations answer the same
/// heartbeat within a minute, and `HEARTBEAT SNR -02` alone says nothing about
/// which of them was being answered. This is the order JS8Call prints too.
fn js8_msg_summary(m: &sdroxide_types::Js8Msg) -> String {
    let mut s = String::new();
    let to = m.to.trim();
    if !to.is_empty() {
        s.push_str(to);
    }
    if let Some(c) = &m.cmd {
        if !s.is_empty() {
            s.push(' ');
        }
        s.push_str(c);
    }
    let text = m.text.trim();
    if !text.is_empty() {
        if !s.is_empty() {
            s.push(' ');
        }
        s.push_str(text);
    }
    if !m.complete {
        s.push('…');
    }
    s
}

fn non_empty(s: &str) -> Option<&str> {
    let t = s.trim();
    (!t.is_empty()).then_some(t)
}

/// The reply a standard JS8 exchange expects to this message, if there is one.
///
/// JS8 carries a conversation, so this only ever *offers*: it fills the
/// composer and the operator is free to rewrite it before pressing send. What
/// it encodes is the handful of turns that are the same in every contact — a
/// heartbeat or a CQ is asking "can anyone hear me?" and wants a report back, a
/// question wants its answer, `HW CPY?` wants a report — so the routine part is
/// one click and everything else is still a text box.
///
/// `None` means "nothing standard to say", which is the answer for free text
/// and therefore for most of a rag-chew. The caller still selects the station,
/// so the composer is aimed at them with nothing typed in it.
fn js8_reply_for(msg: &sdroxide_types::Js8Msg, me: &Js8Me) -> Option<String> {
    let snr = msg.snr_db;
    Some(match msg.cmd.as_deref()? {
        // A heartbeat is answered with a heartbeat report, which is a distinct
        // command from a plain report: it says "this is an answer to your
        // beacon", not "we are in a QSO".
        "HB" => format!("HEARTBEAT SNR {snr}"),
        "CQ" | "SNR?" | "HW CPY?" | "HEARTBEAT SNR" => format!("SNR {snr}"),
        "GRID?" => format!("GRID {}", non_empty(&me.grid)?),
        "STATUS?" | "INFO?" => format!("STATUS {}", non_empty(&me.status)?),
        "HEARING?" => format!("HEARING {}", non_empty(&me.hearing.join(" "))?),
        // They answered us. Acknowledge, and from here it is a conversation.
        "SNR" | "GRID" | "STATUS" | "INFO" | "HEARING" | "FB" | "ACK" => "RR".into(),
        "QSL?" => "QSL".into(),
        "QSL" | "RR" => "73".into(),
        "73" | "SK" => "73".into(),
        // "Say again" wants the same words back, not a new sentence.
        "AGN?" => non_empty(&me.last_sent)?.to_string(),
        _ => return None,
    })
}

impl SdroxideApp {
    /// FSQ panel: the decoded stream + the directed (FSQCALL) layer — a heard
    /// list, a directed compose row (To: + message), and a contacts book.
    /// `panel_h` is the real bounded height (the frame reports an unbounded
    /// `available_height`).
    /// The JS8 panel: what is on the band, and the conversation.
    ///
    /// Shaped like the FT8 panel — an activity list on the left, a draggable
    /// split, a working area on the right — but the right-hand side is a chat
    /// log rather than a QSO sequencer, because that is what JS8 carries.
    pub(in crate::app) fn js8_panel(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        panel_h: f32,
    ) {
        use sdroxide_types::Js8Speed;

        let content_bottom = ui.cursor().top() + panel_h - 26.0;
        let status = self.digi_status.clone();
        let audio_hz = status.as_ref().map(|s| s.audio_hz).unwrap_or(1500.0);
        let transmitting = status.as_ref().map(|s| s.transmitting).unwrap_or(false);
        let js8 = status.as_ref().and_then(|s| s.js8.clone()).unwrap_or_default();

        // ── Header: speed, tuning, queue depth ──────────────────────────────
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("JS8").size(11.0).strong().color(crate::theme::CYAN()));
            let multi = self.digi_cfg_edit.js8_multi_decode;
            // Slowest first: these are one dial from 30-second slots 25 Hz wide
            // to 6-second slots 160 Hz wide, and the declaration order runs
            // NORMAL FAST TURBO SLOW, which is the order JS8Call happened to
            // ship the submodes in and a scale of nothing (issue #389).
            for speed in Js8Speed::UI_ORDER {
                // A lit chip is a speed being *decoded*, which with MULTI on is
                // all four of them — the row is the honest answer to "what am I
                // hearing", and a single lit chip while four waveforms were
                // being read said otherwise. Which one goes out is then the
                // marked one: it is still one speed, and the mark has to
                // survive the whole row being lit.
                let tx = js8.speed == speed;
                let face = if multi && tx {
                    format!("▸{}", speed.label())
                } else {
                    speed.label().to_string()
                };
                if crate::chrome::chip(ui, multi || tx, face)
                    .on_hover_text(if multi && tx {
                        format!(
                            "Every speed is being decoded; ▸ marks {}, the one you transmit at",
                            speed.label()
                        )
                    } else if multi {
                        format!(
                            "Every speed is being decoded. Click to transmit at {} instead",
                            speed.label()
                        )
                    } else {
                        format!("Transmit and decode at {}", speed.label())
                    })
                    .clicked()
                    && !tx
                {
                    self.digi_cfg_edit.js8_speed = speed;
                    cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
                }
            }
            // Decoding all four rather than the one being worked (issue #358).
            // Beside the speed chips because that is the setting it qualifies:
            // without it a station on another speed is simply not there, and
            // nothing on this screen would say so — but set apart from them,
            // because it is not a fifth speed and a row of five identical chips
            // reads as one (issue #389).
            ui.add_space(10.0);
            if crate::chrome::chip(ui, multi, "MULTI")
                .on_hover_text(
                    "Decode every JS8 speed, not only the one you transmit at. The four \
                     speeds share the sub-band and are four different waveforms, so without \
                     this a station on another speed is invisible — and an exchange between \
                     two speeds cannot happen at all. Costs about four times the receive CPU.",
                )
                .clicked()
            {
                self.digi_cfg_edit.js8_multi_decode = !multi;
                cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
            }
            ui.add_space(6.0);
            ui.label(RichText::new(format!("{audio_hz:.0} Hz")).monospace());
            if crate::chrome::chip(ui, false, "−").clicked() {
                cmds.push(Command::SetDigiAudioFreq((audio_hz - 10.0).clamp(200.0, 3500.0)));
            }
            if crate::chrome::chip(ui, false, "+").clicked() {
                cmds.push(Command::SetDigiAudioFreq((audio_hz + 10.0).clamp(200.0, 3500.0)));
            }
            self.digi_freq_chip(ui, cmds);
            if !self.ui_settings.swl {
                // Beacon state. An unattended transmitter must say so where the
                // operator is already looking, and say when it will key next — a
                // countdown is the difference between "armed" and "hung".
                let hb_min = self.digi_cfg_edit.js8_heartbeat_min;
                // Lit by what the engine is *doing*, not by what is configured: at
                // Turbo the interval is set and nothing beacons, and a chip that
                // claimed otherwise would be the one place this must not be wrong.
                let hb_on = crate::chrome::chip(ui, js8.next_hb_in_s.is_some(), "HB AUTO")
                    .on_hover_text(match js8.next_hb_in_s {
                        Some(_) => format!("Beaconing every {hb_min} min — click to stop"),
                        None if js8.speed == Js8Speed::Turbo => {
                            "Turbo does not beacon — it is the local and VHF speed".to_string()
                        }
                        None => "Beacon your callsign and grid every 15 minutes".to_string(),
                    })
                    .clicked();
                if hb_on {
                    // Off if it was on; otherwise the interval most of the band
                    // uses, which SETUP can then change.
                    self.digi_cfg_edit.js8_heartbeat_min = if hb_min > 0 { 0 } else { 15 };
                    cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
                }
                if let Some(left) = js8.next_hb_in_s {
                    ui.label(
                        RichText::new(format!("{}:{:02}", left / 60, left % 60))
                            .monospace()
                            .color(crate::theme::CYAN_DIM()),
                    )
                    .on_hover_text("Until the next heartbeat");
                }
                // Beacons do not go out on the working frequency, so the waterfall
                // shows a burst where the panel's marker is not. Saying where it
                // went is the difference between that reading as a bug and as the
                // sub-band convention working.
                if let Some(hz) = js8.hb_hz {
                    ui.label(
                        RichText::new(format!("HB {hz:.0} Hz"))
                            .monospace()
                            .color(crate::theme::GREEN()),
                    )
                    .on_hover_text(format!(
                        "The last beacon went out at {hz:.0} Hz — a free slot in the {:.0}–{:.0} Hz \
                         heartbeat sub-band, chosen so it lands clear of the signals being decoded.",
                        sdroxide_types::HB_BAND_LO_HZ,
                        sdroxide_types::HB_BAND_HI_HZ,
                    ));
                }
            }
            crate::chrome::row_tail(ui, |ui| {
                // Every setting this mode has — callsign, groups, auto-reply,
                // the beacon interval, the status message — lives in that
                // window, and the JS8 panel is the only one with no other way
                // in: FT8 reaches it from the QSO area, and the keyboard modes
                // keep their parameters in the header instead.
                if crate::chrome::chip(ui, self.show_digi_settings, "⚙ SETUP").clicked() {
                    self.show_digi_settings = !self.show_digi_settings;
                }
                if transmitting {
                    ui.label(RichText::new("● TX").color(crate::theme::ALERT()).strong());
                }
                // A long message takes minutes, not seconds. Saying so while it
                // is going out is the difference between "stuck" and "working".
                if js8.tx_frames_total > 0 {
                    let left = f64::from(js8.tx_frames_pending) * js8.speed.slot_s();
                    ui.label(
                        RichText::new(format!(
                            "{}/{} frames · {left:.0}s",
                            js8.tx_frames_total - js8.tx_frames_pending,
                            js8.tx_frames_total
                        ))
                        .monospace()
                        .color(crate::theme::YELLOW()),
                    );
                }
                self.digi_squelch_slider(ui, cmds);
            });
        });
        ui.add_space(4.0);
        // The slot clock, under the header and across both columns. JS8's turn
        // is the speed's, not fifteen seconds by definition, so this is also
        // where the difference between Slow and Turbo becomes visible.
        self.slot_progress(ui);
        ui.add_space(4.0);

        // Locate the heard stations and hand them to the maps. Done before the
        // list is drawn so a row and its dot on the globe agree this frame.
        let now_t = ui.input(|i| i.time);
        self.js8_observe(&js8.heard, now_t);

        let avail_h = (content_bottom - ui.cursor().top()).max(80.0);
        let total_w = ui.available_width();

        // A phone gets one of the two at a time: the heard list wants 160
        // points and the conversation 200 before either has drawn anything.
        if let Some(pane) = self.phone_pane(ui, self.state.rx[0].mode) {
            if pane == 0 {
                self.js8_heard_list(ui, &js8, avail_h, total_w);
            } else {
                self.js8_chat(ui, cmds, &js8, avail_h);
            }
            return;
        }

        // Floored at the lower bound — see the same guard in the WSPR panel.
        let left_w =
            (total_w * self.view.js8_split_fraction).clamp(160.0, (total_w - 200.0).max(160.0));
        ui.horizontal_top(|ui| {
            // ── Left: who is on the band ────────────────────────────────────
            ui.vertical(|ui| {
                ui.set_width(left_w);
                ui.label(
                    RichText::new("HEARD").size(10.5).strong().color(crate::theme::CYAN_DIM()),
                );
                self.js8_heard_list(ui, &js8, avail_h - 18.0, left_w);
            });

            // ── The drag handle ─────────────────────────────────────────────
            let resp = crate::chrome::split_handle(ui, egui::vec2(7.0, avail_h), None);
            if resp.dragged() {
                let dx = resp.drag_delta().x;
                self.view.js8_split_fraction = ((left_w + dx) / total_w).clamp(0.22, 0.72);
            }

            // ── Right: the conversation ─────────────────────────────────────
            ui.vertical(|ui| {
                ui.set_height(avail_h);
                self.js8_chat(ui, cmds, &js8, avail_h);
            });
        });
    }

    /// The conversation and the controls under it.
    ///
    /// Laid out bottom-up so the controls claim their real height and the
    /// conversation takes whatever is left. Reserving a guessed number of
    /// pixels for them instead clips the bottom row as soon as a chip is added
    /// or the theme's spacing changes.
    fn js8_chat(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        js8: &sdroxide_types::Js8Status,
        avail_h: f32,
    ) {
        ui.allocate_ui(egui::vec2(ui.available_width(), avail_h), |ui| {
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                // First declared is lowest in a bottom-up layout, so this is
                // the gap between the controls and the panel edge. Without it
                // they sit flush against the frame.
                if !self.ui_settings.swl {
                    ui.add_space(8.0);
                    self.js8_compose(ui, cmds, js8);
                    ui.add_space(4.0);
                }
                // Back to normal order for the scrolling part.
                ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                    self.js8_conversation(ui, js8);
                });
            });
        });
    }

    // ── JS8: locating stations ──────────────────────────────────────────────

    /// Where a JS8 station is, if anything knows.
    ///
    /// On-air first — a heartbeat, a CQ or a ` GRID` reply carries a real
    /// locator — then whatever callsign lookup resolved. Most JS8 traffic
    /// carries no grid at all, which is the whole reason the lookup path is
    /// here: without it the map would show only the stations that happened to
    /// beacon while we were listening.
    pub(in crate::app) fn js8_grid_for(
        &self,
        call: &str,
        heard: &[sdroxide_types::Js8Heard],
    ) -> Option<String> {
        if call.is_empty() {
            return None;
        }
        heard
            .iter()
            .find(|h| h.call.eq_ignore_ascii_case(call))
            .and_then(|h| h.grid.clone())
            .or_else(|| {
                self.callsign_cache.get(&call.to_ascii_uppercase()).and_then(|i| i.grid.clone())
            })
            .filter(|g| sdroxide_types::grid_to_latlon(g).is_some())
    }

    /// Feed the heard list to the maps, and ask the lookup service where the
    /// stations that never sent a locator actually are.
    ///
    /// The flat map and the globe both draw [`crate::digi_map::DigiStations`],
    /// which speaks in [`Decode`]s — so the heard stations are handed over in
    /// that shape rather than teaching the map a second kind of station.
    fn js8_observe(&mut self, heard: &[sdroxide_types::Js8Heard], now_t: f64) {
        // JS8's own convention is a heartbeat every ten or fifteen minutes, so
        // FT8's two-minute fade would leave the map blank between them.
        self.digi_stations.set_fade_s(JS8_STATION_FADE_S);
        let located: Vec<Decode> = heard
            .iter()
            .filter_map(|h| {
                let grid = self.js8_grid_for(&h.call, heard)?;
                Some(Decode {
                    slot_utc: h.last_utc,
                    snr_db: h.snr_db,
                    dt: 0.0,
                    audio_hz: h.audio_hz,
                    message: String::new(),
                    to: None,
                    from: Some(h.call.clone()),
                    grid: Some(grid),
                    is_cq: false,
                    cq_to: None,
                    rr73_to: None,
                    free_text: false,
                })
            })
            .collect();
        self.digi_stations.observe(&located, now_t, now_unix());

        // One lookup at a time. Each is an HTTP round trip on a thread of its
        // own, and a busy band puts fifty stations in this list at once.
        if !self.grid_lookup_due(now_t) {
            return;
        }
        let next = heard.iter().map(|h| h.call.to_ascii_uppercase()).find(|c| {
            !c.is_empty()
                && !c.starts_with('@')
                && !self.grid_looked_up.contains(c)
                && self.js8_grid_for(c, heard).is_none()
        });
        self.queue_grid_lookup(next, now_t);
    }

    // ── JS8: the heard list ─────────────────────────────────────────────────

    /// Who is on the band, as the same styled rows the FT8 decode list uses.
    ///
    /// Deliberately the same shape — signal, frequency, callsign, what they'd
    /// be worth, where they are, and a REPLY button — because it is the same
    /// judgement being made, and an operator who has learned to read one list
    /// should not have to learn a second.
    fn js8_heard_list(
        &mut self,
        ui: &mut egui::Ui,
        js8: &sdroxide_types::Js8Status,
        max_h: f32,
        col_w: f32,
    ) {
        let my_grid = self.my_grid();
        let dial_hz = self.state.active_freq_hz();
        let band = if dial_hz > 0.0 { sdroxide_types::adif_band(dial_hz) } else { "" };
        self.log_index();
        // The last thing each station said: what the row shows, and what the
        // REPLY button drafts an answer to.
        let last_msg: std::collections::HashMap<&str, &sdroxide_types::Js8Msg> = js8
            .messages
            .iter()
            .filter(|m| !m.from.is_empty())
            .map(|m| (m.from.as_str(), m))
            .collect();
        let me = self.js8_me(js8);
        // A narrow column — the split dragged in, or a phone's whole screen —
        // takes the row on two lines instead: the fixed columns alone would
        // otherwise run the REPLY button off the right edge. The geography the
        // wide row gives columns rides the message line's dim tail there.
        let wide = col_w > 430.0;

        // Staged, because the row closures borrow `self` immutably.
        let mut pick: Option<(String, Option<String>)> = None;
        // Lifted out and put back below: the row closures hold `self` shared
        // for the grid and log lookups, and the flag cache has to be written
        // to as it fills. Moving it costs one map header — the textures in it
        // are handles, and none of them are re-uploaded.
        let mut flags = std::mem::take(&mut self.flags);

        egui::ScrollArea::vertical()
            .id_salt("js8-heard")
            .max_height(max_h)
            .auto_shrink([false, false])
            .show_themed(ui, |ui| {
                if js8.heard.is_empty() {
                    ui.label(RichText::new("— nothing heard yet —").weak());
                }
                for (i, h) in js8.heard.iter().enumerate() {
                    let msg = last_msg.get(h.call.as_str()).copied();
                    let to_me = msg.is_some_and(js8_personally_addressed);
                    // A heartbeat or a CQ is an invitation, which is what FT8's
                    // red CQ row means too.
                    let calling =
                        msg.is_some_and(|m| matches!(m.cmd.as_deref(), Some("CQ") | Some("HB")));
                    let selected = self.js8_target.eq_ignore_ascii_case(&h.call);
                    let grid = self.js8_grid_for(&h.call, &js8.heard);
                    let dist_km = (!my_grid.is_empty())
                        .then(|| {
                            grid.as_deref()
                                .and_then(|g| sdroxide_types::grid_distance_km(&my_grid, g))
                        })
                        .flatten();
                    let entity = sdroxide_types::resolve_callsign(&h.call);
                    let continent = entity.map(|e| e.continent).unwrap_or("");
                    let flag = entity.map(|e| e.flag).unwrap_or("");
                    let novelty = self.log_index_cache.as_ref().expect("just refreshed").1.novelty(
                        &h.call,
                        grid.as_deref(),
                        band,
                    );
                    let (badge, badge_col) = match novelty.highlight() {
                        Some(sdroxide_types::Highlight::NewDxcc) => ("DXCC", crate::theme::PINK()),
                        Some(sdroxide_types::Highlight::NewDxccBand) => {
                            ("BAND", crate::theme::YELLOW())
                        }
                        Some(sdroxide_types::Highlight::NewGrid) => ("GRID", crate::theme::CYAN()),
                        Some(sdroxide_types::Highlight::NewCall) => {
                            ("NEW", crate::theme::CYAN_DIM())
                        }
                        Some(sdroxide_types::Highlight::Dupe) => ("DUPE", crate::theme::gray(85)),
                        None => ("", Color32::TRANSPARENT),
                    };
                    let dupe = novelty.dupe;
                    // A grid nobody sent is a guess from the callsign database,
                    // and the row says so rather than passing it off as heard.
                    let looked_up = grid.is_some() && h.grid.is_none();
                    let mut reply = false;
                    let mut reply_left: Option<f32> = None;

                    // Everything the row shows, built once and placed by
                    // whichever of the two layouts below runs.
                    let grid_txt = grid.clone().unwrap_or_default();
                    let dist_txt = dist_km.map(|km| format!("{km:.0} km")).unwrap_or_default();
                    let snr_lbl = egui::Label::new(
                        RichText::new(format!("{:+}", h.snr_db))
                            .monospace()
                            .size(13.0)
                            .color(snr_color(h.snr_db)),
                    );
                    let call_lbl = egui::Label::new(
                        RichText::new(&h.call).size(15.0).strong().color(if to_me {
                            crate::theme::YELLOW()
                        } else if dupe {
                            crate::theme::gray(105)
                        } else if calling {
                            crate::theme::GREEN()
                        } else {
                            crate::theme::TEXT_STRONG()
                        }),
                    )
                    .truncate();
                    let badge_lbl =
                        egui::Label::new(RichText::new(badge).size(9.5).strong().color(badge_col));
                    let cont_lbl = egui::Label::new(
                        RichText::new(continent).monospace().size(11.0).strong().color(if dupe {
                            crate::theme::gray(85)
                        } else {
                            crate::theme::continent_color(continent)
                        }),
                    );
                    let said = msg.map(js8_msg_summary).unwrap_or_default();
                    let msg_lbl =
                        egui::Label::new(RichText::new(said).monospace().size(12.5).color(
                            if dupe { crate::theme::gray(95) } else { crate::theme::TEXT() },
                        ))
                        .truncate();
                    let reply_btn = |ui: &mut egui::Ui| {
                        crate::chrome::chip_accent(
                            ui,
                            false,
                            RichText::new("REPLY").size(12.0).strong(),
                            if to_me {
                                crate::theme::YELLOW()
                            } else if calling {
                                crate::theme::GREEN()
                            } else {
                                crate::theme::CYAN()
                            },
                            crate::theme::INK_ON_CYAN(),
                        )
                    };

                    let inner = egui::Frame::new()
                        .fill(if to_me {
                            crate::theme::TOME_BG()
                        } else if calling {
                            crate::theme::CQ_BG()
                        } else {
                            crate::theme::ROW_BG()
                        })
                        .inner_margin(egui::Margin { left: 11, right: 6, top: 6, bottom: 6 })
                        .show(ui, |ui| {
                            let ch = 22.0;
                            if !wide {
                                // Two lines: who (and REPLY) above, what they
                                // last said below with grid and distance in the
                                // tail. The audio-frequency column drops; the
                                // hover card still carries it.
                                ui.spacing_mut().item_spacing.y = 2.0;
                                ui.horizontal(|ui| {
                                    ui.set_min_height(ch);
                                    ui.spacing_mut().item_spacing.x = 7.0;
                                    row_cell(ui, 28.0, ch, true, snr_lbl);
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            let resp = if self.ui_settings.swl {
                                                ui.label("")
                                            } else {
                                                reply_btn(ui)
                                            };
                                            reply = resp.clicked();
                                            if !self.ui_settings.swl {
                                                reply_left = Some(resp.rect.left());
                                            }
                                            ui.with_layout(
                                                egui::Layout::left_to_right(egui::Align::Center),
                                                |ui| {
                                                    // The callsign gets what the
                                                    // badges leave, and truncates
                                                    // before it pushes them out.
                                                    let call_w = (ui.available_width()
                                                        - (34.0 + 22.0 + 24.0 + 3.0 * 7.0))
                                                        .max(40.0);
                                                    row_cell(ui, call_w, ch, false, call_lbl);
                                                    row_cell(ui, 34.0, ch, false, badge_lbl);
                                                    row_cell_ui(ui, 22.0, ch, |ui| {
                                                        flags.show(ui, flag, 12.0);
                                                    });
                                                    row_cell(ui, 24.0, ch, false, cont_lbl);
                                                },
                                            );
                                        },
                                    );
                                });
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 7.0;
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            let tail = [grid_txt.as_str(), dist_txt.as_str()]
                                                .iter()
                                                .filter(|s| !s.is_empty())
                                                .copied()
                                                .collect::<Vec<_>>()
                                                .join(" · ");
                                            if !tail.is_empty() {
                                                ui.label(
                                                    RichText::new(tail)
                                                        .monospace()
                                                        .size(11.0)
                                                        // Dimmer for a grid the database
                                                        // supplied rather than the air.
                                                        .color(if looked_up {
                                                            crate::theme::gray(110)
                                                        } else {
                                                            crate::theme::CYAN_DIM()
                                                        }),
                                                );
                                            }
                                            ui.with_layout(
                                                egui::Layout::left_to_right(egui::Align::Center),
                                                |ui| {
                                                    ui.add(msg_lbl);
                                                },
                                            );
                                        },
                                    );
                                });
                                return;
                            }
                            ui.horizontal(|ui| {
                                ui.set_min_height(ch);
                                ui.spacing_mut().item_spacing.x = 7.0;
                                row_cell(ui, 28.0, ch, true, snr_lbl);
                                row_cell(
                                    ui,
                                    40.0,
                                    ch,
                                    true,
                                    egui::Label::new(
                                        RichText::new(format!("{:.0}", h.audio_hz))
                                            .monospace()
                                            .size(12.0)
                                            .color(crate::theme::gray(120)),
                                    ),
                                );
                                row_cell(ui, 92.0, ch, false, call_lbl);
                                row_cell(ui, 34.0, ch, false, badge_lbl);
                                row_cell_ui(ui, 22.0, ch, |ui| {
                                    flags.show(ui, flag, 12.0);
                                });
                                row_cell(ui, 24.0, ch, false, cont_lbl);
                                row_cell(
                                    ui,
                                    50.0,
                                    ch,
                                    false,
                                    egui::Label::new(
                                        RichText::new(&grid_txt)
                                            .monospace()
                                            .size(12.0)
                                            // Dimmer for a grid the database
                                            // supplied rather than the air.
                                            .color(if looked_up {
                                                crate::theme::gray(110)
                                            } else {
                                                crate::theme::CYAN_DIM()
                                            }),
                                    ),
                                );
                                row_cell(
                                    ui,
                                    58.0,
                                    ch,
                                    true,
                                    egui::Label::new(
                                        RichText::new(&dist_txt)
                                            .monospace()
                                            .size(11.0)
                                            .color(crate::theme::YELLOW()),
                                    ),
                                );
                                // What they last said fills the rest, with the
                                // REPLY button pinned right.
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let resp = if self.ui_settings.swl {
                                            ui.label("")
                                        } else {
                                            reply_btn(ui)
                                        };
                                        reply = resp.clicked();
                                        if !self.ui_settings.swl {
                                            reply_left = Some(resp.rect.left());
                                        }
                                        ui.with_layout(
                                            egui::Layout::left_to_right(egui::Align::Center),
                                            |ui| {
                                                ui.add(msg_lbl);
                                            },
                                        );
                                    },
                                );
                            });
                        });

                    let r = inner.response.rect;
                    let (accent, aw) = if to_me {
                        (crate::theme::YELLOW(), 4.0)
                    } else if calling {
                        (crate::theme::PINK(), 2.5)
                    } else {
                        (crate::theme::CYAN_DIM(), 2.5)
                    };
                    ui.painter().rect_filled(
                        egui::Rect::from_min_max(
                            r.left_top(),
                            egui::pos2(r.left() + aw, r.bottom()),
                        ),
                        0.0,
                        accent,
                    );
                    let body_right = reply_left.map(|x| x - 2.0).unwrap_or(r.right());
                    let body =
                        egui::Rect::from_min_max(r.left_top(), egui::pos2(body_right, r.bottom()));
                    let row = ui
                        .interact(body, ui.id().with(("js8h", i)), egui::Sense::click())
                        .on_hover_ui(|ui| {
                            let d = js8_station_decode(h, grid.clone(), msg);
                            station_card(
                                ui, &mut flags, &d, entity, dist_km, &my_grid, novelty, band,
                                false, calling,
                            );
                        });
                    if selected {
                        // The composer is aimed here: the same amber outline the
                        // FT8 list uses for the decode it is previewing.
                        ui.painter().rect_stroke(
                            r,
                            0.0,
                            egui::Stroke::new(1.4, crate::theme::YELLOW()),
                            egui::StrokeKind::Inside,
                        );
                    } else if row.hovered() {
                        ui.painter().rect_stroke(
                            r,
                            0.0,
                            egui::Stroke::new(1.0, crate::theme::CYAN_DIM()),
                            egui::StrokeKind::Inside,
                        );
                    }
                    // REPLY drafts the answer this exchange expects; a plain
                    // click only aims the composer, so it never overwrites a
                    // half-typed sentence.
                    if reply {
                        pick = Some((
                            h.call.clone(),
                            msg.and_then(|m| js8_reply_for(m, &me)).or(Some(String::new())),
                        ));
                    } else if row.clicked() {
                        pick = Some((h.call.clone(), None));
                    }
                    ui.add_space(3.0);
                }
            });

        self.flags = flags;

        if let Some((call, draft)) = pick {
            self.js8_select(&call, draft, &js8.heard);
        }
    }

    /// The conversation: every reassembled transmission, newest at the bottom.
    ///
    /// Rows are clickable — that is where a heartbeat, a CQ or a `HW CPY?`
    /// turns into the reply it expects.
    fn js8_conversation(&mut self, ui: &mut egui::Ui, js8: &sdroxide_types::Js8Status) {
        let me = self.js8_me(js8);
        let multi = self.digi_cfg_edit.js8_multi_decode;
        let mut pick: Option<(String, Option<String>)> = None;
        egui::ScrollArea::vertical()
            .id_salt("js8-convo")
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .show_themed(ui, |ui| {
                if js8.messages.is_empty() {
                    ui.label(RichText::new("— no messages —").weak());
                }
                for (i, m) in js8.messages.iter().enumerate() {
                    let selected =
                        !m.from.is_empty() && self.js8_target.eq_ignore_ascii_case(&m.from);
                    let to_me = js8_personally_addressed(m);
                    let inner = egui::Frame::new()
                        .fill(if to_me { crate::theme::TOME_BG() } else { crate::theme::ROW_BG() })
                        .inner_margin(egui::Margin { left: 8, right: 5, top: 3, bottom: 3 })
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.set_min_width(ui.available_width());
                                ui.spacing_mut().item_spacing.x = 4.0;
                                let (_, _, _, h, mi, _) =
                                    sdroxide_types::utc_ymd_hms(m.last_slot_utc);
                                ui.label(
                                    RichText::new(format!("{h:02}:{mi:02}")).monospace().weak(),
                                );
                                // Which of the four waveforms carried this.
                                // Only where it is a question: with one speed
                                // running every row is that speed and a column
                                // saying so is noise, but with MULTI on the
                                // list was the one place that did not say
                                // (issue #389). A row decoded before the
                                // setting was turned off keeps its tag, so
                                // nothing already on screen quietly loses the
                                // answer it was showing.
                                if multi || m.speed != js8.speed {
                                    ui.label(
                                        RichText::new(m.speed.tag())
                                            .monospace()
                                            .color(crate::theme::PINK()),
                                    )
                                    .on_hover_text(format!("Decoded at {}", m.speed.label()));
                                }
                                if to_me {
                                    ui.label(RichText::new("★").color(crate::theme::YELLOW()));
                                }
                                let who = if m.from.is_empty() { "…" } else { &m.from };
                                ui.label(
                                    RichText::new(format!("{who}:")).monospace().strong().color(
                                        if to_me {
                                            crate::theme::CYAN()
                                        } else {
                                            crate::theme::CYAN_DIM()
                                        },
                                    ),
                                );
                                // Who it was addressed to. Without this a
                                // directed message reads as if it were meant
                                // for everyone: on a busy channel several
                                // stations answer the same heartbeat inside a
                                // minute and every one of them says
                                // "HEARTBEAT SNR ..".
                                let to = m.to.trim();
                                if !to.is_empty() {
                                    ui.label(RichText::new(to).monospace().strong().color(
                                        if to_me {
                                            crate::theme::YELLOW()
                                        } else {
                                            crate::theme::GREEN()
                                        },
                                    ));
                                }
                                if let Some(c) = &m.cmd {
                                    ui.label(
                                        RichText::new(c).monospace().color(crate::theme::PINK()),
                                    );
                                }
                                let body = RichText::new(&m.text).monospace();
                                // An incomplete message is still arriving; greying
                                // it stops a half-sentence reading as the whole one.
                                ui.label(if m.complete { body } else { body.weak() });
                                if !m.complete {
                                    ui.label(
                                        RichText::new(format!("… ({} frames)", m.frames)).weak(),
                                    );
                                }
                            });
                        });

                    let r = inner.response.rect;
                    ui.painter().rect_filled(
                        egui::Rect::from_min_max(
                            r.left_top(),
                            egui::pos2(r.left() + if to_me { 3.0 } else { 2.0 }, r.bottom()),
                        ),
                        0.0,
                        if to_me { crate::theme::YELLOW() } else { crate::theme::CYAN_DIM() },
                    );
                    let mut row = ui.interact(r, ui.id().with(("js8m", i)), egui::Sense::click());
                    // Drafted only for the row under the cursor: the log holds
                    // two hundred of these and every draft is an allocation.
                    if !m.from.is_empty() && (row.hovered() || row.clicked()) {
                        let draft = js8_reply_for(m, &me);
                        row = row.on_hover_text(match &draft {
                            Some(d) => format!("Reply to {}: “{d}”", m.from),
                            None => format!("Address the composer at {}", m.from),
                        });
                        if row.clicked() {
                            pick = Some((m.from.clone(), draft));
                        }
                    }
                    if selected || row.hovered() {
                        ui.painter().rect_stroke(
                            r,
                            0.0,
                            egui::Stroke::new(
                                1.0,
                                if selected {
                                    crate::theme::YELLOW()
                                } else {
                                    crate::theme::CYAN_DIM()
                                },
                            ),
                            egui::StrokeKind::Inside,
                        );
                    }
                    ui.add_space(2.0);
                }
            });
        if let Some((call, draft)) = pick {
            self.js8_select(&call, draft, &js8.heard);
        }
    }

    /// Facts about our own station the reply drafts may quote.
    fn js8_me(&self, js8: &sdroxide_types::Js8Status) -> Js8Me {
        let cfg = self.digi_status.as_ref().map(|s| &s.config).unwrap_or(&self.digi_cfg_edit);
        Js8Me {
            grid: cfg.my_grid.to_uppercase(),
            status: cfg.js8_status.clone(),
            hearing: js8.heard.iter().take(4).map(|h| h.call.clone()).collect(),
            last_sent: self.js8_last_sent.clone(),
        }
    }

    /// Aim the composer at a station, optionally with a draft in it, and put
    /// them on the map as the preview marker.
    fn js8_select(
        &mut self,
        call: &str,
        draft: Option<String>,
        heard: &[sdroxide_types::Js8Heard],
    ) {
        self.js8_target = call.to_string();
        if let Some(d) = draft {
            self.text_tx = d;
        }
        let ll = self.js8_grid_for(call, heard).as_deref().and_then(sdroxide_types::grid_to_latlon);
        self.digi_preview = ll.map(|ll| (call.to_string(), ll));
    }

    /// The two rows under the JS8 conversation: the actions, and the composer.
    ///
    /// **Declared bottom-first.** The caller lays this out with
    /// [`egui::Layout::bottom_up`] so the controls claim their true height and
    /// the conversation gets the remainder, which means the first row written
    /// here is the one that appears lowest.
    fn js8_compose(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        js8: &sdroxide_types::Js8Status,
    ) {
        let has_target = !self.js8_target.is_empty();
        // Every chip in this row puts a frame on the air, so a receiver greys
        // the lot of them — the CLEAR TO chip below only forgets a selection
        // and stays live.
        let tx_ok = self.tx_capable();

        // Actions — the lower of the two rows. Wrapped, because the right
        // column can be dragged narrow and a chip that does not fit must move
        // to the next line rather than be clipped off the edge.
        ui.horizontal_wrapped(|ui| {
            // `chip_enabled` rather than a scope around the pair: this row
            // wraps, and a child `Ui` in a wrapping row does not.
            if rx_only_hint(crate::chrome::chip_enabled(ui, tx_ok, false, " CQ "), tx_ok).clicked()
            {
                cmds.push(Command::DigiCallCq);
            }
            if rx_only_hint(crate::chrome::chip_enabled(ui, tx_ok, false, " HB "), tx_ok).clicked()
            {
                cmds.push(Command::DigiSendText("@ALLCALL HB".into()));
            }
            // The queries address whichever station is selected. Shown greyed
            // rather than hidden when there is none: a row that changes shape
            // as you click around is hard to aim at, and chips that only exist
            // sometimes are chips nobody discovers.
            ui.add_enabled_ui(has_target && tx_ok, |ui| {
                for q in ["SNR?", "GRID?", "HEARING?", "STATUS?", "HW CPY?"] {
                    if rx_only_hint(crate::chrome::chip(ui, false, q), tx_ok).clicked() {
                        let full = format!("{} {q}", self.js8_target);
                        self.js8_last_sent = full.clone();
                        cmds.push(Command::DigiSendText(full));
                    }
                }
                // The two that close a contact. Worth a button of their own:
                // they are the most-typed things on the band, and typing them
                // is the one moment an operator is not watching the panel.
                for q in ["RR", "73"] {
                    if rx_only_hint(crate::chrome::chip(ui, false, q), tx_ok).clicked() {
                        let full = format!("{} {q}", self.js8_target);
                        self.js8_last_sent = full.clone();
                        cmds.push(Command::DigiSendText(full));
                    }
                }
            });
            if has_target && crate::chrome::chip(ui, false, " CLEAR TO ").clicked() {
                self.js8_target.clear();
            }
            // Empties the conversation above, not the selection — which is what
            // the chip beside it does, hence the different words for it.
            self.clear_rx_chip(ui, cmds);
        });

        // The gap between the two rows. In a bottom-up layout this space sits
        // above what was just written, so it separates the actions from the
        // composer rather than pushing them into the panel edge.
        ui.add_space(6.0);

        // Compose. The buttons are declared right-to-left first so they always
        // get their width, and the text box takes whatever is left over.
        let target = if has_target { self.js8_target.clone() } else { "@ALLCALL".to_string() };
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{target}:")).monospace().color(crate::theme::CYAN_DIM()),
            );
            let mut send = false;
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Stop lives next to send: they are the two things you reach
                // for in a hurry, and a long message takes minutes to drain.
                if crate::chrome::chip(ui, false, " STOP ").clicked() {
                    cmds.push(Command::DigiAbortTx);
                }
                send = tx_gated(ui, tx_ok, |ui| {
                    crate::chrome::chip_accent(
                        ui,
                        false,
                        " SEND ",
                        crate::theme::ALERT(),
                        crate::theme::INK_ON_CYAN(),
                    )
                })
                .clicked();
                // Before pressing send, say how long it will take. JS8's most
                // surprising property to a new operator is that a sentence can
                // occupy a minute of air time.
                if !self.text_tx.trim().is_empty() {
                    let frames = js8_frame_estimate(&self.text_tx);
                    ui.label(
                        RichText::new(format!(
                            "{frames}f · {:.0}s",
                            f64::from(frames) * js8.speed.slot_s()
                        ))
                        .monospace()
                        .weak(),
                    );
                }
                let resp = crate::chrome::field(
                    ui,
                    egui::TextEdit::singleline(&mut self.text_tx)
                        .desired_width(ui.available_width().max(60.0))
                        .hint_text("Message…"),
                );
                // Return does what SEND does, so it has to be shut off with it.
                send |= tx_ok && resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            });
            if send && !self.text_tx.trim().is_empty() {
                let body = self.text_tx.trim();
                let full = if has_target {
                    format!("{} {body}", self.js8_target)
                } else {
                    body.to_string()
                };
                // Kept so `AGN?` — "say again" — has something to draft from.
                self.js8_last_sent = full.clone();
                cmds.push(Command::DigiSendText(full));
                self.text_tx.clear();
            }
        });
    }
}

#[cfg(test)]
mod js8_panel_tests {
    use super::js8_frame_estimate;

    #[test]
    fn short_messages_take_one_frame() {
        assert_eq!(js8_frame_estimate("HELLO"), 1);
        assert_eq!(js8_frame_estimate("HI"), 1);
    }

    #[test]
    fn the_estimate_grows_with_the_message() {
        let short = js8_frame_estimate("HELLO WORLD");
        let long = js8_frame_estimate(
            "HELLO WORLD THIS IS A CONSIDERABLY LONGER MESSAGE THAT WILL SPAN SEVERAL FRAMES",
        );
        assert!(long > short, "{long} should exceed {short}");
    }

    #[test]
    fn an_empty_message_still_reads_as_one_frame_not_zero() {
        // The label is only shown for non-empty input, but a zero here would
        // render "0f · 0s" if that ever changed.
        assert_eq!(js8_frame_estimate(""), 1);
        assert_eq!(js8_frame_estimate("   "), 1);
    }

    use super::{Js8Me, js8_msg_summary, js8_personally_addressed, js8_reply_for};
    use sdroxide_types::Js8Msg;

    fn me() -> Js8Me {
        Js8Me {
            grid: "FN42".into(),
            status: "PORTABLE".into(),
            hearing: vec!["KN4CRD".into(), "VK3ABC".into()],
            last_sent: "KN4CRD HELLO FROM THE HILLS".into(),
        }
    }

    fn msg(cmd: Option<&str>, to: &str) -> Js8Msg {
        Js8Msg {
            from: "KN4CRD".into(),
            to: to.into(),
            text: String::new(),
            cmd: cmd.map(str::to_string),
            snr_db: -12,
            audio_hz: 1500.0,
            first_slot_utc: 1000,
            last_slot_utc: 1000,
            frames: 1,
            complete: true,
            to_me: true,
            speed: sdroxide_types::Js8Speed::Normal,
        }
    }

    #[test]
    fn an_announcement_drafts_the_report_it_is_asking_for() {
        // A heartbeat and a CQ are both "can anyone hear me?"; JS8's answer is
        // a signal report, and a heartbeat gets the report command that says
        // "this answers your beacon".
        assert_eq!(
            js8_reply_for(&msg(Some("HB"), "@ALLCALL"), &me()).as_deref(),
            Some("HEARTBEAT SNR -12")
        );
        assert_eq!(js8_reply_for(&msg(Some("CQ"), "@ALLCALL"), &me()).as_deref(), Some("SNR -12"));
        assert_eq!(
            js8_reply_for(&msg(Some("HW CPY?"), "N0JDS"), &me()).as_deref(),
            Some("SNR -12")
        );
    }

    #[test]
    fn a_question_drafts_its_answer() {
        for (cmd, want) in [
            ("SNR?", "SNR -12"),
            ("GRID?", "GRID FN42"),
            ("STATUS?", "STATUS PORTABLE"),
            ("HEARING?", "HEARING KN4CRD VK3ABC"),
            // "Say again" wants the same words back, not a new sentence.
            ("AGN?", "KN4CRD HELLO FROM THE HILLS"),
        ] {
            assert_eq!(
                js8_reply_for(&msg(Some(cmd), "N0JDS"), &me()).as_deref(),
                Some(want),
                "{cmd}"
            );
        }
    }

    #[test]
    fn a_contact_winds_itself_down() {
        for (cmd, want) in
            [("SNR", "RR"), ("QSL?", "QSL"), ("RR", "73"), ("73", "73"), ("SK", "73")]
        {
            assert_eq!(
                js8_reply_for(&msg(Some(cmd), "N0JDS"), &me()).as_deref(),
                Some(want),
                "{cmd}"
            );
        }
    }

    #[test]
    fn free_text_drafts_nothing_so_the_composer_is_left_alone() {
        // The point of JS8 is the rag-chew: there is no standard answer to
        // "GOOD MORNING FROM VIENNA", and guessing one would be in the way.
        assert_eq!(js8_reply_for(&msg(None, "N0JDS"), &me()), None);
        // Nor to traffic this station deliberately does not handle.
        for cmd in [">", "MSG TO:", "QUERY MSGS", "YES", "NO"] {
            assert_eq!(js8_reply_for(&msg(Some(cmd), "N0JDS"), &me()), None, "{cmd}");
        }
    }

    #[test]
    fn a_draft_is_dropped_rather_than_sent_empty() {
        // "GRID" with no grid says "I am here" and answers nothing, at the cost
        // of a full transmission.
        let blank = Js8Me { grid: String::new(), status: String::new(), ..me() };
        assert_eq!(js8_reply_for(&msg(Some("GRID?"), "N0JDS"), &blank), None);
        assert_eq!(js8_reply_for(&msg(Some("STATUS?"), "N0JDS"), &blank), None);
        let deaf = Js8Me { hearing: Vec::new(), last_sent: String::new(), ..me() };
        assert_eq!(js8_reply_for(&msg(Some("HEARING?"), "N0JDS"), &deaf), None);
        assert_eq!(js8_reply_for(&msg(Some("AGN?"), "N0JDS"), &deaf), None);
    }

    #[test]
    fn a_broadcast_is_not_a_message_addressed_to_us() {
        // Every heartbeat on the band is `to_me`; colouring them all gold would
        // leave nothing for a station actually calling us to stand out against.
        assert!(!js8_personally_addressed(&msg(Some("HB"), "@ALLCALL")));
        assert!(js8_personally_addressed(&msg(Some("SNR?"), "N0JDS")));
        assert!(js8_personally_addressed(&msg(Some("STATUS?"), "@JS8NET")));
    }

    #[test]
    fn a_stations_last_word_reads_as_one_line() {
        let mut m = msg(Some("HB"), "@ALLCALL");
        m.text = "EM73".into();
        assert_eq!(js8_msg_summary(&m), "@ALLCALL HB EM73");
        m.cmd = None;
        assert_eq!(js8_msg_summary(&m), "@ALLCALL EM73");
        // Still arriving, and the row has to say so.
        m.complete = false;
        assert_eq!(js8_msg_summary(&m), "@ALLCALL EM73…");
    }

    #[test]
    fn a_reply_names_the_station_it_answers() {
        // The point of issue #372: on a busy channel a dozen stations answer
        // the same heartbeat, and "HEARTBEAT SNR -02" on its own does not say
        // which of them was being answered.
        let mut m = msg(Some("HEARTBEAT SNR"), "OH8STN");
        m.text = "-02".into();
        assert_eq!(js8_msg_summary(&m), "OH8STN HEARTBEAT SNR -02");
        // An undirected transmission has no recipient to name.
        let mut free = msg(None, "");
        free.text = "GOOD MORNING FROM VIENNA".into();
        assert_eq!(js8_msg_summary(&free), "GOOD MORNING FROM VIENNA");
    }
}
