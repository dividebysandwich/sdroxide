//! The "PUBLIC SDRS" window: browse the receivers other people have published
//! and add one as a radio of its own, named after its directory entry.
//!
//! Built on the same bones as [`crate::app::spots`], because it is the same
//! problem — a long list from the internet, filter chips, a fuzzy search, and a
//! row that does something when it is clicked.
//!
//! # Where the list comes from
//!
//! Not from here. The window asks
//! [`sdroxide_types::DeviceProbe::PublicSdrs`], which is answered by the
//! machine the radio is attached to — the same lane the device enumerations
//! use. That is what makes this work in a browser: the web client has no HTTP
//! client at all, and could not read either directory across origins if it had.
//! It is also the right end to ask, because it is the end that will hold the
//! connection: a receiver this screen can reach is no use if the station
//! cannot.
//!
//! # Being a guest
//!
//! A receiver that cannot be used is shown greyed with the reason rather than
//! hidden. Two of them matter and neither is obvious from outside: a KiwiSDR
//! whose operator has not opened any channels to non-browser apps will refuse
//! sdroxide however many are free, and a receiver whose channels are all in use
//! is somebody else's for the moment. Being told which is far better than a
//! connection that fails for no visible reason.

use crate::i18n::tr;
use eframe::egui::{self, RichText};
use sdroxide_types::{Command, PublicSdrEntry, PublicSdrNetwork};

use crate::theme::ThemedScroll;
use crate::time::now_unix;

use crate::app::util::fmt_age;
use crate::app::{RadioTabRequest, SdroxideApp};

/// Rows drawn at once. The directories run to about eleven hundred receivers
/// between them and every row is a handful of laid-out labels; past this the
/// list is not a list any more, and the search box is the way through it.
const MAX_ROWS: usize = 300;

/// One receiver row: network badge, name, what it covers, how busy it is,
/// where it is, and the one way to take it: ADD AS RADIO.
///
/// Columns are allocated rather than laid out by content, the way
/// [`crate::app::spots`]'s rows are: a directory is a table, and a name that
/// pushed the frequency column sideways would make it unreadable. The last one
/// takes whatever is left over so a wide window shows more of the place name
/// rather than more empty space.
///
/// Returns whether ADD AS RADIO was pressed on this row.
fn entry_row(ui: &mut egui::Ui, e: &PublicSdrEntry, distance_km: Option<f64>) -> bool {
    let blocked = e.blocked_reason();
    let mut add = false;
    let dim = crate::theme::gray(if blocked.is_some() { 100 } else { 170 });
    let net_col = match e.network {
        PublicSdrNetwork::KiwiSdr => crate::theme::CYAN(),
        PublicSdrNetwork::SpyServer => crate::theme::PINK(),
        PublicSdrNetwork::SdrList => crate::theme::GREEN(),
    };
    /// What the buttons (or the refusal text) need on the right.
    const ACTIONS_W: f32 = 172.0;
    /// The five fixed columns ahead of the place, and the gaps between them.
    const FIXED_W: f32 = 60.0 + 176.0 + 80.0 + 46.0 + 4.0 * 6.0;
    /// The distance column, which is only drawn when a station grid is set.
    const DISTANCE_W: f32 = 56.0 + 6.0;
    /// Narrower than this and a place name is an ellipsis, which is worth less
    /// than the space. The row's tooltip has it either way.
    const PLACE_MIN_W: f32 = 74.0;

    egui::Frame::new()
        .fill(crate::theme::ROW_BG())
        .inner_margin(egui::Margin { left: 8, right: 6, top: 2, bottom: 2 })
        .show(ui, |ui| {
            let row_w = ui.available_width();
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let col = |ui: &mut egui::Ui, w: f32, lbl: egui::Label| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(w, 20.0), egui::Sense::hover());
                    ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(rect)
                            .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    )
                    .add(lbl);
                };
                col(
                    ui,
                    60.0,
                    egui::Label::new(
                        RichText::new(e.network.label()).size(10.0).strong().color(net_col),
                    ),
                );
                let name_col = if blocked.is_some() {
                    crate::theme::gray(130)
                } else {
                    crate::theme::TEXT_STRONG()
                };
                col(
                    ui,
                    176.0,
                    egui::Label::new(RichText::new(&e.name).size(13.0).color(name_col)).truncate(),
                );
                col(
                    ui,
                    80.0,
                    egui::Label::new(RichText::new(e.range_label()).size(11.0).color(dim)),
                );
                // Users, and the one number that decides whether a receiver is
                // available at all.
                let busy = if e.max_users == 0 {
                    "—".to_string()
                } else {
                    format!("{}/{}", e.users, e.max_users)
                };
                let busy_col = match () {
                    _ if blocked.is_some() => crate::theme::ALERT(),
                    _ if e.max_users > 0 && e.users * 2 >= e.max_users => crate::theme::YELLOW(),
                    _ => crate::theme::GREEN(),
                };
                col(ui, 46.0, egui::Label::new(RichText::new(busy).size(11.0).color(busy_col)));
                // The distance stands in where the operator never wrote a
                // place, which on a SpyServer is most of them — it is the only
                // thing the directory knows about where that receiver is.
                // Its own column rather than appended to the place: a name
                // long enough to truncate would otherwise take the distance
                // with it, and on a SpyServer — where the operator usually
                // wrote no place at all — the distance is the only thing the
                // directory knows about where the receiver is.
                // Only where there is one to show: an operator who has not set
                // a grid would otherwise pay a column's width for a run of
                // blanks, and that width is the place name's.
                if let Some(km) = distance_km {
                    col(
                        ui,
                        56.0,
                        egui::Label::new(
                            RichText::new(if km >= 1000.0 {
                                format!("{:.1}k km", km / 1000.0)
                            } else {
                                format!("{km:.0} km")
                            })
                            .size(11.0)
                            .color(dim),
                        ),
                    );
                }
                // The one column that flexes, from the row's own width — which
                // has to be captured before anything is allocated out of it,
                // because `available_width` here describes what is left of the
                // frame rather than the row.
                //
                // On a narrow window it is dropped rather than squeezed: the
                // fixed columns come to more than a small window has, and a
                // place name rendered as a bare ellipsis is worth less than the
                // space it costs. Widen the window and it comes back.
                let used = FIXED_W + if distance_km.is_some() { DISTANCE_W } else { 0.0 };
                let place_w = (row_w - used - ACTIONS_W).min(340.0);
                if place_w >= PLACE_MIN_W {
                    col(
                        ui,
                        place_w,
                        egui::Label::new(RichText::new(&e.location).size(11.0).color(dim))
                            .truncate(),
                    );
                }

                ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| match &blocked {
                        Some(why) => {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(why).size(10.0).color(crate::theme::ALERT()),
                                )
                                .truncate(),
                            );
                        }
                        None => {
                            // One way to take a receiver: as a radio of its own,
                            // named after its directory entry. The "USE" route
                            // (this radio rebuilt as the receiver) is gone.
                            // Why: HDZ 2026-11-04: "use und use new panel ist
                            //      Quatsch, hier müsste rein 'add as radio', und
                            //      gut ist". USE replaced a configured radio and
                            //      needed a three-way confirmation for it (issue
                            //      #254: an IC-9700 lost to one click); "+ TAB"
                            //      did what this does, named after the
                            //      interface. With the original tab strip the
                            //      new radio is a new tab; with Panel Exchange
                            //      it is one more under "Change radio".
                            if crate::chrome::chip(ui, false, tr!("publicsdr-add-as-radio"))
                                .on_hover_text(tr!("publicsdr-add-as-radio-tip"))
                                .clicked()
                            {
                                add = true;
                            }
                        }
                    },
                );
            });
        })
        .response
        // Everything that did not earn a column of its own: the address to
        // connect to, the antenna, and what the receiver says it is.
        .on_hover_text(tr!(
            "publicsdr-antenna",
            address = e.address,
            device = e.device,
            antenna =
                if e.antenna.is_empty() { tr!("publicsdr-not-stated") } else { e.antenna.clone() },
            e3 = match e.snr_db {
                Some(snr) => tr!("publicsdr-noise-floor-score", snr = snr),
                None => tr!("publicsdr-up-khz-i-q", khz = format!("{:.0}", e.max_iq_rate / 1e3)),
            }
        ));
    ui.add_space(1.0);
    add
}

/// The connection field in front of a picked receiver's row on the map view:
/// ONLINE, SILENT, or CHECKING… until the answer is in; what answered (or why
/// nothing did) on hover.
// Why: HDZ 2026-11-04: "du trägst ja unten Infos ein, da könnte ein Feld für
//      online/silent stehen".
fn check_cell(ui: &mut egui::Ui, check: Option<&crate::publicsdr_map::Check>) {
    use crate::publicsdr_map::Check;
    /// Width of the field, in points: the longest word ("ÇEVRİMİÇİ", "PRÜFE…")
    /// at 11 pt with a little air.
    const W: f32 = 74.0;
    // Not checked yet and on its way read the same: PRÜFE… without a tooltip.
    let (text, colour, tip) = match check {
        Some(Check::Answered { result: Ok(m), .. }) => {
            (tr!("publicsdr-check-online"), crate::theme::GREEN(), Some(m.clone()))
        }
        Some(Check::Answered { result: Err(m), .. }) => {
            (tr!("publicsdr-check-silent"), crate::theme::ALERT(), Some(m.clone()))
        }
        _ => (tr!("publicsdr-check-waiting"), crate::theme::gray(140), None),
    };
    // A fixed rect rather than a label, so the rows' columns line up whatever
    // the word; 20 pt high like `entry_row`'s cells.
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(W, 20.0), egui::Sense::hover());
    ui.painter().text(
        rect.left_center() + egui::vec2(4.0, 0.0),
        egui::Align2::LEFT_CENTER,
        text,
        egui::FontId::proportional(11.0),
        colour,
    );
    if let Some(t) = tip {
        resp.on_hover_text(t);
    }
}

/// Rank what the chips let through against what is in the search box.
///
/// Two tiers, and the second only when the first comes up empty. Every other
/// directory of KiwiSDRs answers a callsign typed into its filter with the one
/// receiver named after it, and [`crate::fuzzy`] on its own does not: a
/// six-character callsign is a scattered subsequence of dozens of the eleven
/// hundred station names, antennas and place names here by pure accident, so
/// the row actually wanted arrives buried in them (issue #347). When anything
/// matches the query literally, then, those are the answer and the accidents
/// are dropped; the fuzzy matches are what is left when nothing does, which is
/// where they earn their keep — a half-remembered name or a typo still finds
/// the receiver.
///
/// The fallback keeps only *tight* fuzzy matches ([`crate::fuzzy::tight_terms`]).
/// A loose subsequence over this long a haystack is an accident, not a near
/// miss: `twente`, which no directory lists, came back as 181 unrelated
/// receivers. A query that nothing carries literally or nearly now answers
/// "no match", which is the truth.
fn ranked<'a>(visible: &[&'a PublicSdrEntry], query: &str) -> Vec<(&'a PublicSdrEntry, i32)> {
    let scored: Vec<(&PublicSdrEntry, i32, bool, bool)> = visible
        .iter()
        .filter_map(|e| {
            let hay = e.haystack();
            let literal = crate::fuzzy::contains_terms(&hay, query);
            let near = literal || crate::fuzzy::tight_terms(&hay, query);
            crate::fuzzy::score_terms(&hay, query).map(|s| (*e, s, literal, near))
        })
        .collect();
    let any_literal = scored.iter().any(|&(_, _, literal, _)| literal);
    let mut rows: Vec<(&PublicSdrEntry, i32)> = scored
        .into_iter()
        .filter(|&(_, _, literal, near)| if any_literal { literal } else { near })
        .map(|(e, score, _, _)| (e, score))
        .collect();
    if !query.is_empty() {
        rows.sort_by_key(|r| std::cmp::Reverse(r.1));
    }
    rows
}

impl SdroxideApp {
    /// Everything the search and the chips let through, ranked.
    fn public_sdr_rows<'a>(
        &self,
        entries: &'a [PublicSdrEntry],
        dial_hz: f64,
    ) -> Vec<(&'a PublicSdrEntry, i32)> {
        let query = self.public_sdr_search.trim();
        let visible: Vec<&PublicSdrEntry> = entries
            .iter()
            .filter(|e| {
                // Indexed by the network's own position in `ALL`, so adding
                // one is a single edit there rather than an arm here that is
                // easy to forget.
                let net_on = PublicSdrNetwork::ALL
                    .iter()
                    .position(|n| *n == e.network)
                    .and_then(|i| self.public_sdr_nets_shown.get(i).copied())
                    .unwrap_or(true);
                net_on
                    && (!self.public_sdr_free_only || e.blocked_reason().is_none())
                    && (!self.public_sdr_in_band || e.covers(dial_hz))
            })
            .collect();
        ranked(&visible, query)
    }

    /// Browse the public-SDR directories and open one as a radio.
    pub(in crate::app) fn public_sdrs_window(
        &mut self,
        ctx: &egui::Context,
        _cmds: &mut [Command],
    ) {
        if !self.show_public_sdrs {
            // Asked again the next time it opens, so a window reopened after an
            // hour is not showing an hour-old list.
            self.public_sdrs_asked = false;
            return;
        }
        // The cached answer on first open, so the window paints at once; the
        // ⟳ chip is what goes to the network.
        if !self.public_sdrs_asked {
            self.public_sdrs_asked = true;
            self.ask_device(ctx, sdroxide_types::DeviceProbe::PublicSdrs { refresh: false });
        }

        let dial_hz = self.state.active_freq_hz();
        let my_pos = sdroxide_types::grid_to_latlon(&self.my_grid());
        let directory = self.public_sdrs.clone();
        let mut open = self.show_public_sdrs;
        let mut refresh = false;
        // The receiver whose ADD AS RADIO was pressed this frame, acted on
        // after the window has given `self` back.
        let mut picked: Option<PublicSdrEntry> = None;

        let tool_title = tr!("publicsdr-public-sdrs");
        let mut tool_st = self.take_tool_state("PUBLIC SDRS");
        let tool_child = crate::app::undocked::ChildWindow::own(self);
        crate::app::undocked::tool_window(
            ctx,
            crate::app::undocked::ToolWindow::new("PUBLIC SDRS", &tool_title, [980.0, 520.0]),
            &mut tool_st,
            tool_child,
            &mut open,
            |ui| {
                ui.horizontal(|ui| {
                    // LIST | MAP: two views of the same filtered receivers. The
                    // chips right of them apply to both.
                    // Why: HDZ 2026-11-04: "eine Karte für die SDR-Stationen
                    //      dran machen".
                    if crate::chrome::chip(ui, !self.public_sdr_map, tr!("publicsdr-view-list"))
                        .clicked()
                    {
                        self.public_sdr_map = false;
                    }
                    if crate::chrome::chip(ui, self.public_sdr_map, tr!("publicsdr-view-map"))
                        .on_hover_text(tr!("publicsdr-view-map-tip"))
                        .clicked()
                    {
                        self.public_sdr_map = true;
                    }
                    ui.separator();
                    for (i, net) in PublicSdrNetwork::ALL.iter().enumerate() {
                        if crate::chrome::chip(ui, self.public_sdr_nets_shown[i], net.label())
                            .clicked()
                        {
                            self.public_sdr_nets_shown[i] = !self.public_sdr_nets_shown[i];
                        }
                    }
                    if crate::chrome::chip(
                        ui,
                        self.public_sdr_free_only,
                        tr!("publicsdr-available"),
                    )
                    .on_hover_text(tr!("publicsdr-hide-receivers-are-full-ones"))
                    .clicked()
                    {
                        self.public_sdr_free_only = !self.public_sdr_free_only;
                    }
                    if crate::chrome::chip(ui, self.public_sdr_in_band, tr!("publicsdr-band"))
                        .on_hover_text(tr!("publicsdr-only-receivers-cover-current-dial"))
                        .clicked()
                    {
                        self.public_sdr_in_band = !self.public_sdr_in_band;
                    }
                    if crate::chrome::chip(ui, self.public_sdr_low_bw, tr!("publicsdr-low-bw"))
                        .on_hover_text(tr!("publicsdr-take-spyserver-low-bandwidth-shape"))
                        .clicked()
                    {
                        self.public_sdr_low_bw = !self.public_sdr_low_bw;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if crate::chrome::chip(ui, false, tr!("publicsdr-refresh"))
                            .on_hover_text(tr!("publicsdr-fetch-both-directories-again"))
                            .clicked()
                        {
                            refresh = true;
                        }
                    });
                });
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;
                    ui.label(RichText::new("⌕").color(crate::theme::CYAN_DIM()).size(14.0));
                    crate::chrome::field(
                        ui,
                        egui::TextEdit::singleline(&mut self.public_sdr_search)
                            .desired_width(240.0)
                            .hint_text(tr!("publicsdr-name-place-antenna-band"))
                            .text_color(crate::theme::TEXT_STRONG()),
                    );
                    if !self.public_sdr_search.trim().is_empty()
                        && ui.button("✕").on_hover_text(tr!("publicsdr-clear-search")).clicked()
                    {
                        self.public_sdr_search.clear();
                    }
                });

                let Some(dir) = &directory else {
                    ui.separator();
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(if self.probes_answered {
                            tr!("publicsdr-fetching-receiver-lists")
                        } else {
                            tr!("publicsdr-machine-radio-attached-does-not")
                        })
                        .color(crate::theme::gray(140)),
                    );
                    return;
                };

                // Age and any source that failed, on one line: a directory that
                // is quietly an hour old looks exactly like a fresh one.
                ui.horizontal_wrapped(|ui| {
                    let age = if dir.fetched_unix > 0 {
                        fmt_age(now_unix() - dir.fetched_unix)
                    } else {
                        "—".to_string()
                    };
                    let per_net = PublicSdrNetwork::ALL
                        .iter()
                        .map(|n| format!("{} {}", dir.count(*n), n.label()))
                        .collect::<Vec<_>>()
                        .join(" · ");
                    ui.label(
                        RichText::new(tr!(
                            "publicsdr-receivers-fetched-ago",
                            entries = dir.entries.len(),
                            per_net = per_net,
                            age = age
                        ))
                        .size(11.0)
                        .color(crate::theme::gray(150)),
                    );
                    for note in &dir.notes {
                        ui.label(RichText::new(note).size(11.0).color(crate::theme::ALERT()));
                    }
                });
                ui.separator();

                let rows = self.public_sdr_rows(&dir.entries, dial_hz);
                if self.public_sdr_map {
                    if let Some(e) = self.public_sdr_map_view(ui, &dir.entries, &rows, my_pos) {
                        picked = Some(e);
                    }
                    return;
                }
                if !self.public_sdr_search.trim().is_empty() || rows.len() > MAX_ROWS {
                    let shown = rows.len().min(MAX_ROWS);
                    let (text, colour) = match rows.len() {
                        0 => (tr!("publicsdr-no-match").to_string(), crate::theme::ALERT()),
                        n if n > MAX_ROWS => (
                            tr!("publicsdr-showing-search-narrow", shown = shown, n = n),
                            crate::theme::YELLOW(),
                        ),
                        n => (tr!("publicsdr-match", n = n), crate::theme::YELLOW()),
                    };
                    ui.label(RichText::new(text).color(colour).size(10.0));
                }

                egui::ScrollArea::vertical().auto_shrink([false, false]).show_themed(ui, |ui| {
                    for (e, _) in rows.iter().take(MAX_ROWS) {
                        let km = match (my_pos, e.lat, e.lon) {
                            (Some(me), Some(lat), Some(lon)) => Some(sdroxide_types::distance_km(
                                me,
                                (f64::from(lat), f64::from(lon)),
                            )),
                            _ => None,
                        };
                        if entry_row(ui, e, km) {
                            picked = Some((*e).clone());
                        }
                    }
                    if rows.is_empty() {
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(tr!("publicsdr-nothing-matches-try-turning-filter"))
                                .color(crate::theme::gray(120)),
                        );
                    }
                });
            },
        );
        self.put_tool_state("PUBLIC SDRS", tool_st);
        self.show_public_sdrs = open;
        // Closed: the pick goes with the window, so the next opening does not
        // show the last session's receivers with their last answers.
        if !open {
            self.public_sdr_map_state.picked.clear();
            self.public_sdr_map_state.ghost = None;
        }

        if refresh {
            self.public_sdrs = None;
            self.ask_device(ctx, sdroxide_types::DeviceProbe::PublicSdrs { refresh: true });
        }
        if let Some(entry) = picked {
            self.take_public_sdr(&entry);
        }
    }

    /// The MAP view: every receiver `rows` holds as a dot, and below the map
    /// the receivers the last click picked, as list rows with ADD AS RADIO.
    ///
    /// In: the whole directory (picked receivers are looked up there, so a
    /// pick survives a filter change), the filtered rows, home. Out: the
    /// receiver whose ADD AS RADIO was pressed this frame.
    /// Effect: the map fills the window less the rows below it; a status line
    /// says how many filtered receivers have no position and so no dot.
    /// Fails: none.
    // Why: HDZ 2026-11-04: "eine Karte für die SDR-Stationen dran machen". The
    //      rows below are the list's own (`entry_row`), so taking a receiver is
    //      the same button in both views.
    fn public_sdr_map_view(
        &mut self,
        ui: &mut egui::Ui,
        all: &[PublicSdrEntry],
        rows: &[(&PublicSdrEntry, i32)],
        my_pos: Option<(f64, f64)>,
    ) -> Option<PublicSdrEntry> {
        use crate::publicsdr_map::{MAX_PICKED, key, show};
        // A radio asked to be shown here: pick its receiver and look at it.
        self.public_sdr_map_state.take_focus(all, crate::time::now_unix_f64());
        // Everything below stays inside the width the window has now. Outside
        // a scroll area, a row that measures itself by what is free would
        // otherwise widen the window a little every frame, until it filled
        // the screen.
        // Why: HDZ 2026-11-04: "dann rutscht der Kartenzoom, so dass die ganze
        //      Bildschirmbreite langsam vollgemacht wird".
        ui.set_max_width(ui.available_width());
        // What the chips and the search let through, without the ranking the
        // list needs: a map has no order.
        let visible: Vec<&PublicSdrEntry> = rows.iter().map(|(e, _)| *e).collect();
        // Said rather than hidden: a receiver with no position has no dot,
        // and the operator should know the map is not the whole list.
        let unplaced = visible.iter().filter(|e| e.lat.is_none() || e.lon.is_none()).count();
        // The pick, looked up in the whole directory rather than in `rows`, so
        // the rows stay while a chip or the search hides their dots.
        let picked: Vec<&PublicSdrEntry> = self
            .public_sdr_map_state
            .picked
            .iter()
            .filter_map(|k| all.iter().find(|e| key(e) == *k))
            .take(MAX_PICKED)
            .collect();

        // One status line, then the map, then one row per picked receiver
        // (or a line saying how to pick). The window grows to hold the rows,
        // so they need no share of the map's height.
        ui.horizontal(|ui| {
            // Names on the map, on or off. Here rather than in the chip row
            // above, which LIST shares: a chip that appears and vanishes there
            // would shift the filters under the pointer.
            // Why: HDZ 2026-11-04: "gönn der Karte einen Toggle-Knopf 'Labels',
            //      ich will nicht immer Labels sehen".
            let st = &mut self.public_sdr_map_state;
            if crate::chrome::chip(ui, !st.names_off, tr!("publicsdr-map-names"))
                .on_hover_text(tr!("publicsdr-map-names-tip"))
                .clicked()
            {
                st.names_off = !st.names_off;
            }
            if crate::chrome::chip(ui, !st.cities_off, tr!("publicsdr-map-cities"))
                .on_hover_text(tr!("publicsdr-map-cities-tip"))
                .clicked()
            {
                st.cities_off = !st.cities_off;
            }
            ui.label(
                RichText::new(tr!(
                    "publicsdr-map-status",
                    shown = visible.len() - unplaced,
                    unplaced = unplaced
                ))
                .size(10.0)
                .color(crate::theme::gray(150)),
            );
        });
        // The rows below the map first, as a bottom panel of the window, and
        // the map in all the height that leaves: the window's size is the
        // map's, in both directions, and nothing the map does feeds back into
        // the window. The panel measures the rows before the map is laid out,
        // so the map never takes their share and the window never grows.
        // Why: HDZ 2026-11-07: "die Map-Begrenzung stößt in der Höhe an eine
        //      Grenze, auch wenn keine Breite da wäre, und fortan kann ich nur
        //      noch breiter werden, das ist sehr awkward als Fensterverhalten";
        //      before that 2026-11-05: "die Karte rutscht in die Breite", the
        //      loop a map sized from `available_height` started without the
        //      panel.
        // The map's return value (pick changed) is not needed: the rows are
        // drawn from the pick every frame.
        let mut take = None;
        egui::Panel::bottom(ui.id().with("publicsdr_map_rows"))
            .frame(egui::Frame::NONE)
            .resizable(false)
            .show(ui, |ui| {
                take = self.public_sdr_map_rows(ui, all, &picked, my_pos);
            });
        let h = ui.available_height().max(crate::publicsdr_map::MIN_HEIGHT);
        show(ui, &mut self.public_sdr_map_state, &visible, my_pos, h);
        take
    }

    /// The rows under the Public SDRs map: how to pick while nothing is, else
    /// one row per picked receiver with its connection check and ADD AS
    /// RADIO. Out: the receiver whose ADD was pressed.
    fn public_sdr_map_rows(
        &mut self,
        ui: &mut egui::Ui,
        all: &[PublicSdrEntry],
        picked: &[&PublicSdrEntry],
        my_pos: Option<(f64, f64)>,
    ) -> Option<PublicSdrEntry> {
        use crate::publicsdr_map::key;
        ui.add_space(4.0);
        // Nothing picked: say how to pick, and there is nothing to check.
        if picked.is_empty() {
            ui.label(
                RichText::new(tr!("publicsdr-map-pick")).size(11.0).color(crate::theme::gray(130)),
            );
            return None;
        }
        // The picked receivers' connection checks: send the ones that fell
        // due, and keep frames coming until every answer is in.
        let now = crate::time::now_unix_f64();
        let due = self.public_sdr_map_state.due_checks(now, &tr!("publicsdr-check-no-answer"));
        // Asked of the machine with the radio on it, like every probe: it is
        // the end that would hold the connection. The answer comes back
        // through `apply_probe_answer` → `public_sdr_check_answered`.
        for (k, address) in due {
            if let Some(e) = all.iter().find(|e| key(e) == k) {
                // `ask_device` wants the context while `ui` is borrowed.
                let ctx = ui.ctx().clone();
                self.ask_device(
                    &ctx,
                    sdroxide_types::DeviceProbe::PublicSdrCheck { network: e.network, address },
                );
            }
        }
        // Ten frames a second while a check is due or on its way: enough for
        // the 500 ms delay to be kept and an answer to show at once.
        if self.public_sdr_map_state.checks_pending() {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
        }

        // One row per picked receiver: its connection field, then the list's
        // own row with ADD AS RADIO. The first ADD pressed is the answer.
        let mut take = None;
        for e in picked.iter().copied() {
            // Distance only where both ends have a position, as in the list.
            let km = match (my_pos, e.lat, e.lon) {
                (Some(me), Some(lat), Some(lon)) => {
                    Some(sdroxide_types::distance_km(me, (f64::from(lat), f64::from(lon))))
                }
                _ => None,
            };
            let check = self.public_sdr_map_state.checks.get(&key(e));
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                check_cell(ui, check);
                // In a column of its own: `entry_row` ends with a gap meant to
                // go below the row, and in this horizontal row it went to the
                // right instead, widening the window by 5 pt every frame.
                // Why: HDZ 2026-11-05: "beim anklicken eines symbols rutschte es
                //      auf volle breite so pö a pö".
                ui.vertical(|ui| {
                    if entry_row(ui, e, km) {
                        take = Some(e.clone());
                    }
                });
            });
        }
        take
    }

    /// File the answer to a map receiver check under its row.
    ///
    /// In: what `ProbeAnswer::PublicSdrCheck` carries. Effect: the waiting
    /// check on that network and endpoint becomes ONLINE or SILENT
    /// ([`crate::publicsdr_map::PublicSdrMapState::answered`]); an answer
    /// nobody waits for is dropped. Fails: none.
    // Why: a method here rather than a reach into the field from the settings
    //      module, where answers arrive, so the map state stays private to
    //      this window.
    pub(in crate::app) fn public_sdr_check_answered(
        &mut self,
        network: PublicSdrNetwork,
        address: &str,
        result: &Result<String, String>,
    ) {
        self.public_sdr_map_state.answered(network, address, result, crate::time::now_unix_f64());
    }

    /// Whether this radio listens to a public receiver (KiwiSDR or SpyServer),
    /// so that its name on the tab can open the map on it.
    // Pitfall: asked of the controller, not read from `radio_cfg`: that is the
    //          settings page's copy, filled only once the page has been
    //          opened, so a radio added earlier had no link (HDZ 2026-11-04:
    //          "geht noch nicht, und die bestehenden müssten retro gelinkt
    //          werden").
    //
    // Asked every frame for the shown tab, and a local controller answers
    // `radio_config()` by reading `radio.json` from disk, so the answer is
    // kept for a second.
    pub(crate) fn on_public_sdr(&self, now: f64) -> bool {
        let (at, on) = self.on_public_sdr_cache.get();
        if at.is_finite() && (0.0..1.0).contains(&(now - at)) {
            return on;
        }
        let on = self
            .ctrl
            .radio_config()
            .as_ref()
            .and_then(crate::publicsdr_map::radio_target)
            .is_some();
        self.on_public_sdr_cache.set((now, on));
        on
    }

    /// Open Public SDRs on the map, at the receiver this radio listens to.
    ///
    /// In: nothing. Effect: the window opens in MAP with the request held in
    /// [`crate::publicsdr_map::PublicSdrMapState::focus`]; the directory is
    /// asked for as on any opening. Nothing for a radio on no public receiver.
    pub(crate) fn show_self_on_public_map(&mut self) {
        let Some(target) =
            self.ctrl.radio_config().as_ref().and_then(crate::publicsdr_map::radio_target)
        else {
            return;
        };
        self.public_sdr_map_state.focus = Some(target);
        self.public_sdr_map = true;
        self.show_public_sdrs = true;
    }

    /// Add a picked receiver as a radio of its own, named after its entry.
    ///
    /// In: the directory entry. Effect: a [`RadioTabRequest::Add`] with the
    /// receiver's whole configuration and its catalogue name, acted on by the
    /// shell after the frame; a notice says it is on its way. The radio the
    /// operator is on is left exactly as it was.
    /// Fails: none here; a factory that cannot make the radio reports through
    /// the shell's own notice.
    // Why: HDZ 2026-11-04: "dabei wäre statt Kiwisdr … der Sendername aus dem
    //      Katalog zu übernehmen". Without a name the radio was called after
    //      its interface, so every public receiver read "Kiwisdr".
    fn take_public_sdr(&mut self, entry: &PublicSdrEntry) {
        // A brand-new radio has none of this radio's settings, and should not
        // inherit them: it is a different receiver.
        let fresh =
            entry.radio_config(&sdroxide_types::RadioConfig::default(), self.public_sdr_low_bw);
        // Where it stands, for the day the directory no longer lists it.
        if let Some(target) = crate::publicsdr_map::radio_target(&fresh) {
            self.public_sdr_map_state.remember(&target, entry);
        }
        self.radio_tab_requests.push(RadioTabRequest::Add {
            station: self.station_key(),
            preset: Some(Box::new(fresh)),
            name: Some(radio_name(entry)),
        });
        self.show_notice(tr!("publicsdr-opening-another-radio", entry = entry.name));
        // Done here: the receiver is a radio of its own now, and this window
        // belongs to the radio it was opened from. Left open it would stay on
        // that radio, with the receiver still picked.
        // Why: HDZ 2026-11-04: "da die Karte von einem Radio aus geöffnet wird,
        //      bleibt die Karte da dran kleben, obwohl wir ein neues Radio
        //      anlegen … du solltest diese Map richtig schließen"; "da bleiben
        //      die Radios stale".
        self.show_public_sdrs = false;
        self.public_sdr_map_state.picked.clear();
        self.public_sdr_map_state.ghost = None;
    }
}

/// The name a radio taken from the directory is given.
///
/// In: the directory entry. Out: [`catalogue_name`] of its published name; for
/// a placeholder name ("anonymous", "unknown", empty — see [`is_placeholder`])
/// the place where it is given, else the device and the host, as in
/// `"AirspyOne 47.144.9.193"`.
// Why: HDZ 2026-11-04: "die SpyServer sind alle anonymous, dann sollte wenigstens
//      die IP als Name rein". Measured on the cached directory: 113 of 212
//      SpyServers are "anonymous", 3 of those give a place. The device goes with
//      the host so that a hundred of them can be told apart in "Change radio".
//      The port is left off: it is 5555 almost everywhere.
fn radio_name(entry: &PublicSdrEntry) -> String {
    if !is_placeholder(&entry.name) {
        return catalogue_name(&entry.name);
    }
    if !entry.location.trim().is_empty() {
        return catalogue_name(&entry.location);
    }
    // Host without the port; a bracketed IPv6 literal keeps its brackets off.
    let host = match entry.address.rsplit_once(':') {
        Some((h, p)) if p.chars().all(|c| c.is_ascii_digit()) => h,
        _ => entry.address.as_str(),
    };
    let host = host.trim_start_matches('[').trim_end_matches(']');
    let device = entry.device.trim();
    if device.is_empty() { host.to_string() } else { format!("{device} {host}") }
}

/// Whether a published name says nothing about the receiver: empty, or one of
/// the words directories put there when the operator gave none (any case).
fn is_placeholder(name: &str) -> bool {
    let n = name.trim().to_ascii_lowercase();
    n.is_empty() || matches!(n.as_str(), "anonymous" | "unknown" | "n/a" | "none")
}

/// The published name made fit for the strip: the entry's own name,
/// with the decoration directory names carry at their ends removed and the
/// length kept to what the strip shows whole.
///
/// In: the directory name, as published (`"######ZL2J 0-30 MHz SDR"`).
/// Out: `"ZL2J 0-30 MHz SDR"`; at most [`NAME_MAX`] characters, cut at a word
/// where there is one. An entry that is nothing but decoration keeps its name
/// as published, so the radio is never nameless.
// Why: operators pad their listing to sort first or to stand out ("######",
//      "- ", "| "); the strip is where the name is read, and padding there is
//      noise. Inner punctuation stays: "0 - 30 MHz | Svartklubbudden" is what
//      the operator called it.
fn catalogue_name(published: &str) -> String {
    let trimmed = published.trim_matches(|c: char| !c.is_alphanumeric());
    if trimmed.is_empty() {
        return published.trim().to_string();
    }
    if trimmed.chars().count() <= NAME_MAX {
        return trimmed.to_string();
    }
    let cut: String = trimmed.chars().take(NAME_MAX).collect();
    // The cut already ends a word: keep it whole.
    if trimmed.chars().nth(NAME_MAX).is_some_and(|c| c == ' ') {
        return cut.trim_end_matches(|c: char| !c.is_alphanumeric()).to_string();
    }
    // Back to the last word boundary, unless that would leave too little.
    // `rfind(' ')` is a byte index on an ASCII space, so the slice is safe.
    match cut.rfind(' ') {
        Some(i) if i >= NAME_MAX / 2 => {
            cut[..i].trim_end_matches(|c: char| !c.is_alphanumeric()).to_string()
        }
        _ => cut,
    }
}

/// Longest name [`catalogue_name`] gives a radio, in characters: about what the
/// strip shows at 16 pt beside its buttons in half a laptop screen.
const NAME_MAX: usize = 32;

#[cfg(test)]
mod tests {
    use super::*;

    /// A receiver with just enough filled in for the search box to chew on.
    fn entry(name: &str, location: &str) -> PublicSdrEntry {
        PublicSdrEntry {
            network: PublicSdrNetwork::KiwiSdr,
            name: name.into(),
            location: location.into(),
            antenna: String::new(),
            device: "KiwiSDR".into(),
            address: "example.com:8073".into(),
            lat: None,
            lon: None,
            grid: String::new(),
            min_hz: 0.0,
            max_hz: 30e6,
            users: 0,
            max_users: 4,
            api_channels: Some(2),
            max_iq_rate: 12_000.0,
            full_control: true,
            session_limit_min: 0,
            snr_db: None,
        }
    }

    /// Typing a callsign finds the receiver named after it and nothing else,
    /// the way every other KiwiSDR directory answers the same query — issue
    /// #347 is that fuzzy matching alone buried it under a dozen names whose
    /// letters happened to fall in that order.
    #[test]
    fn a_callsign_typed_in_full_finds_only_the_receiver_named_after_it() {
        let entries = [
            entry("R7KJG SDR", "Rostov-on-Don"),
            entry("Rugby 7 Kilo Juliet Golf", "somewhere else entirely"),
            entry("Radio 7 Kranj Jesenice Gorica", "Slovenia"),
        ];
        let visible: Vec<&PublicSdrEntry> = entries.iter().collect();
        let rows = ranked(&visible, "r7kjg");
        assert_eq!(rows.len(), 1, "one literal match means one row");
        assert_eq!(rows[0].0.name, "R7KJG SDR");
    }

    /// ...but a query nothing carries literally still gets the fuzzy list
    /// rather than an empty window, so a half-remembered name or a typo is
    /// still worth typing.
    #[test]
    fn a_query_nobody_matches_literally_falls_back_to_the_fuzzy_list() {
        let entries = [entry("R7KJG SDR", "Rostov-on-Don"), entry("Twente", "Enschede")];
        let visible: Vec<&PublicSdrEntry> = entries.iter().collect();
        let rows = ranked(&visible, "rstv");
        assert!(!rows.is_empty(), "a near miss should still show something");
        assert_eq!(rows[0].0.name, "R7KJG SDR", "the closest one first");
    }

    /// ...and a query that nothing carries literally or nearly answers nothing:
    /// letters that merely fall in order across a long description are not a
    /// near miss (`twente`, a WebSDR no directory lists, returned 181 rows).
    #[test]
    fn a_query_only_scattered_letters_match_answers_nothing() {
        let entries =
            [entry("0-30 MHz SDR", "South Wenatchee, WA"), entry("80m Dipole", "Chichester UK")];
        let visible: Vec<&PublicSdrEntry> = entries.iter().collect();
        assert!(ranked(&visible, "twente").is_empty());
    }

    /// An empty box is not a query at all: every row the chips let through
    /// stays, in the order the directory gave them.
    #[test]
    fn an_empty_search_box_keeps_every_row() {
        let entries = [entry("R7KJG SDR", "Rostov-on-Don"), entry("Twente", "Enschede")];
        let visible: Vec<&PublicSdrEntry> = entries.iter().collect();
        let rows = ranked(&visible, "");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].0.name, "R7KJG SDR");
    }

    /// Proves: a directory name loses the padding at its ends and keeps what
    /// the operator wrote inside it ([`catalogue_name`]).
    #[test]
    fn a_catalogue_name_loses_its_padding_and_keeps_its_words() {
        assert_eq!(catalogue_name("######ZL2J 0-30 MHz SDR"), "ZL2J 0-30 MHz SDR");
        assert_eq!(catalogue_name("- 0-30MHz HF SDR PLONSK-"), "0-30MHz HF SDR PLONSK");
        assert_eq!(catalogue_name("0 - 30 MHz | Svartklubbudden"), "0 - 30 MHz | Svartklubbudden");
        // Nothing but decoration: kept as published rather than emptied.
        assert_eq!(catalogue_name(" ### "), "###");
    }

    /// Proves: a SpyServer listed as "anonymous" is named after its place where
    /// it gives one, else after its device and host ([`radio_name`]).
    #[test]
    fn an_anonymous_receiver_is_named_after_its_place_or_its_host() {
        let mut e = entry("anonymous", "");
        e.device = "AirspyOne".into();
        e.address = "47.144.9.193:5555".into();
        assert_eq!(radio_name(&e), "AirspyOne 47.144.9.193");
        e.address = "[2001:db8::1]:5555".into();
        assert_eq!(radio_name(&e), "AirspyOne 2001:db8::1");
        e.location = "Ventura, California".into();
        assert_eq!(radio_name(&e), "Ventura, California");
        // A real name is kept, whatever else the entry says.
        e.name = "######ZL2J 0-30 MHz SDR".into();
        assert_eq!(radio_name(&e), "ZL2J 0-30 MHz SDR");
    }

    /// Proves: a long name is cut to [`NAME_MAX`] at a word, never mid-word
    /// when a word boundary is in reach.
    #[test]
    fn a_long_catalogue_name_is_cut_at_a_word() {
        let n = catalogue_name("0,5-30 MHz SDR, Bad Münstereifel Eifel Germany active loop");
        assert!(n.chars().count() <= NAME_MAX, "{n}");
        assert_eq!(n, "0,5-30 MHz SDR, Bad Münstereifel");
    }
}
