//! The TLE tab: the operator's satellite additions.
//!
//! Element sets can be pasted in or subscribed to by URL, and either can carry
//! a frequency correction. Edits here are written straight out — there is no
//! APPLY step to hang them off — and the solar window picks up the new `Arc` on
//! its next frame.
//!
//! "Written out" means sent to the engine, which persists `satellites.json` and
//! announces the result. The engine host is where the subscribed listings are
//! fetched and cached, and in server mode its tracker is what feeds the
//! browser's 3D view — so this tab configures the station's satellites from
//! wherever it happens to be open, rather than a set of its own.

use crate::i18n::{Tr, tr};
use eframe::egui::{self, RichText};

use crate::time::now_unix;

use crate::app::settings::SettingsIo;

/// The built-in Hamlib rigctld server: the control surface every "NET rigctl"
/// client speaks.
/// WSJT-X UDP broadcast: what the logging ecosystem listens for.
/// The TLE tab: which satellites the tracker follows, and what they are on.
///
/// Three sections, in the order an operator gets to them: subscriptions (the
/// answer for anything they mean to keep tracking, because a TLE goes stale in
/// days), element sets pasted in by hand (the answer for a one-off), and the
/// frequency table the pass window shows.
pub(in crate::app) fn settings_tle_tab(ui: &mut egui::Ui, io: &mut SettingsIo) {
    use crate::theme;

    ui.label(
        RichText::new(tr!("settings-tle-satellites-element-sets-frequencies"))
            .size(14.0)
            .strong()
            .color(theme::CYAN()),
    );
    ui.add_space(4.0);
    if !io.sat_seeded {
        ui.label(
            RichText::new(tr!("settings-tle-waiting-station-s-satellite-configuration")).weak(),
        );
        return;
    }
    ui.label(RichText::new(tr!("settings-tle-tracker-already-fetches-celestrak-s")).weak());
    if !io.sat_ui.note.is_empty() {
        ui.add_space(4.0);
        ui.label(RichText::new(&io.sat_ui.note).color(theme::YELLOW()).size(11.0));
    }

    ui.add_space(10.0);
    settings_tle_subscriptions(ui, io);
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    settings_tle_pasted(ui, io);
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    settings_tle_freqs(ui, io);
}

/// Subscribed element-set listings, and the one-click CelesTrak groups.
fn settings_tle_subscriptions(ui: &mut egui::Ui, io: &mut SettingsIo) {
    use crate::theme;

    ui.label(RichText::new(tr!("settings-tle-subscriptions")).strong());
    ui.label(
        RichText::new(tr!("settings-tle-listings-fetched-kept-current-same")).weak().size(11.0),
    );
    ui.add_space(6.0);

    let mut remove = None;
    for (i, sub) in io.sat_edit.subs.iter_mut().enumerate() {
        let st = io.sat_subs.iter().find(|s| s.url.trim() == sub.url.trim());
        ui.push_id(("tle-sub", i), |ui| {
            ui.horizontal(|ui| {
                crate::chrome::checkbox(ui, &mut sub.enabled, "")
                    .on_hover_text(tr!("settings-tle-fetch-track-listing"));
                crate::chrome::field(
                    ui,
                    egui::TextEdit::singleline(&mut sub.name)
                        .desired_width(120.0)
                        .hint_text("name"),
                );
                crate::chrome::field(
                    ui,
                    egui::TextEdit::singleline(&mut sub.url)
                        .desired_width(300.0)
                        .hint_text("https://…"),
                );
                if ui.button("✕").on_hover_text(tr!("settings-tle-remove-subscription")).clicked()
                {
                    remove = Some(i);
                }
            });
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.label(
                    RichText::new(tr!("settings-tle-orbits"))
                        .color(theme::CYAN_DIM())
                        .size(9.5)
                        .strong(),
                )
                .on_hover_text(tr!("settings-tle-which-satellites-listing-get-orbit"));
                // The middle position keys off sdroxide's own curated list,
                // which is ten *amateur* satellites — so for a weather or GNSS
                // listing it would behave exactly like "none". Greyed out once
                // a fetch has proved this listing has none of them, rather than
                // left as a chip that quietly does nothing.
                let no_curated = st.is_some_and(|s| s.fetched_unix > 0 && s.curated == 0);
                for o in sdroxide_types::OrbitRings::ALL {
                    let dead = o == sdroxide_types::OrbitRings::Curated && no_curated;
                    let resp = ui
                        .add_enabled_ui(!dead, |ui| {
                            crate::chrome::chip(ui, sub.orbits == o, o.tr())
                        })
                        .inner;
                    let hint = if dead {
                        tr!("settings-tle-nothing-listing-sdroxide-s-curated")
                    } else {
                        String::from(o.hint())
                    };
                    if resp.on_hover_text(hint).clicked() && !dead {
                        sub.orbits = o;
                    }
                }
                let mut only = sub.only_text();
                let resp = crate::chrome::field(
                    ui,
                    egui::TextEdit::singleline(&mut only)
                        .desired_width(180.0)
                        .hint_text(tr!("settings-tle-all-satellites")),
                )
                .on_hover_text(tr!("settings-tle-catalogue-numbers-keep-comma-separated"));
                if resp.changed() {
                    sub.set_only_text(&only);
                }

                // Status: what the last fetch actually did. Matched by URL
                // rather than by position — the two lists are edited apart.
                let (text, color) = match (sub.problem(), st) {
                    (Some(p), _) => (p.to_string(), theme::ALERT()),
                    (None, None) => (tr!("settings-tle-not-fetched-yet"), theme::LINE_LIT()),
                    (None, Some(s)) => match &s.error {
                        Some(e) => (e.clone(), theme::ALERT()),
                        None if s.fetched_unix == 0 => {
                            (tr!("settings-tle-not-fetched-yet"), theme::LINE_LIT())
                        }
                        None => (
                            tr!(
                                "settings-tle-subscription-age",
                                count = s.count,
                                age = sdroxide_solar::timefmt::age(now_unix() - s.fetched_unix)
                            ),
                            theme::GREEN(),
                        ),
                    },
                };
                ui.label(RichText::new(text).color(color).size(10.5));
            });
        });
        ui.add_space(2.0);
    }
    if let Some(i) = remove {
        io.sat_edit.subs.remove(i);
    }

    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        if ui.button(tr!("settings-tle-subscription")).clicked() {
            io.sat_edit.subs.push(sdroxide_types::TleSubscription::new("New", ""));
        }
        if crate::chrome::chip_accent(
            ui,
            false,
            RichText::new(tr!("settings-tle-update-now")).strong(),
            theme::GREEN(),
            theme::INK_ON_CYAN(),
        )
        .on_hover_text(tr!("settings-tle-ask-radio-engine-fetch-every"))
        .clicked()
        {
            *io.sat_sub_refresh = true;
        }
    });

    ui.add_space(6.0);
    ui.label(
        RichText::new(tr!("settings-tle-celestrak-groups"))
            .color(theme::CYAN_DIM())
            .size(10.0)
            .strong(),
    );
    ui.horizontal_wrapped(|ui| {
        for g in sdroxide_types::CELESTRAK_GROUPS {
            let have = io.sat_edit.has_sub(g.url);
            if crate::chrome::chip(ui, have, g.name).on_hover_text(g.hint).clicked() && !have {
                let mut sub = sdroxide_types::TleSubscription::new(g.name, g.url);
                sub.orbits = g.orbits;
                io.sat_edit.subs.push(sub);
                io.sat_ui.note = tr!("settings-tle-subscribed", name = g.name.to_string());
            }
        }
    });
}

/// Element sets pasted in by hand.
fn settings_tle_pasted(ui: &mut egui::Ui, io: &mut SettingsIo) {
    use crate::theme;

    ui.label(RichText::new(tr!("settings-tle-pasted-element-sets")).strong());
    ui.label(RichText::new(tr!("settings-tle-one-off-these-do-not")).weak().size(11.0));
    ui.add_space(6.0);

    let now = now_unix();
    let mut remove = None;
    for (i, t) in io.sat_edit.tles.iter_mut().enumerate() {
        ui.push_id(("tle-set", i), |ui| {
            ui.horizontal(|ui| {
                crate::chrome::checkbox(ui, &mut t.enabled, "")
                    .on_hover_text(tr!("settings-tle-track-one"));
                crate::chrome::field(
                    ui,
                    egui::TextEdit::singleline(&mut t.name).desired_width(180.0).hint_text("name"),
                );
                match t.problem() {
                    Some(p) => {
                        ui.label(RichText::new(p).color(theme::ALERT()).size(10.5));
                    }
                    None => {
                        let age = tle_epoch_age(t, now);
                        let (text, color) = match age {
                            // Past where SGP4 is worth anything, which is the
                            // whole reason a paste is a stopgap.
                            Some(a) if a > 14 * 86_400 => (
                                tr!(
                                    "settings-tle-norad-too-stale",
                                    norad = t.norad_id().unwrap_or(0),
                                    age = sdroxide_solar::timefmt::age(a)
                                ),
                                theme::ALERT(),
                            ),
                            Some(a) if a > 3 * 86_400 => (
                                tr!(
                                    "settings-tle-norad-age",
                                    norad = t.norad_id().unwrap_or(0),
                                    age = sdroxide_solar::timefmt::age(a)
                                ),
                                theme::YELLOW(),
                            ),
                            Some(a) => (
                                tr!(
                                    "settings-tle-norad-age",
                                    norad = t.norad_id().unwrap_or(0),
                                    age = sdroxide_solar::timefmt::age(a)
                                ),
                                theme::GREEN(),
                            ),
                            None => (
                                tr!("settings-tle-norad-n", norad = t.norad_id().unwrap_or(0)),
                                theme::LINE_LIT(),
                            ),
                        };
                        ui.label(RichText::new(text).color(color).size(10.5));
                    }
                }
                if ui
                    .button("✎")
                    .on_hover_text(tr!("settings-tle-show-two-element-lines"))
                    .clicked()
                {
                    io.sat_ui.open_tle = (io.sat_ui.open_tle != Some(i)).then_some(i);
                }
                if ui.button("✕").on_hover_text(tr!("settings-tle-remove")).clicked() {
                    remove = Some(i);
                }
            });
            if io.sat_ui.open_tle == Some(i) {
                // Monospace: the format is column-addressed, so a proportional
                // font makes a misaligned paste impossible to see.
                for line in [&mut t.line1, &mut t.line2] {
                    crate::chrome::field(
                        ui,
                        egui::TextEdit::singleline(line)
                            .desired_width(560.0)
                            .font(egui::TextStyle::Monospace),
                    );
                }
            }
        });
    }
    if let Some(i) = remove {
        io.sat_edit.tles.remove(i);
        io.sat_ui.open_tle = None;
    }

    ui.add_space(6.0);
    crate::chrome::field(
        ui,
        egui::TextEdit::multiline(&mut io.sat_ui.paste)
            .desired_rows(3)
            .desired_width(600.0)
            .font(egui::TextStyle::Monospace)
            .hint_text(tr!("settings-tle-paste-two-three-line-element")),
    );
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui.button(tr!("settings-tle-add-pasted")).clicked() {
            let found = sdroxide_types::parse_tle_block(&io.sat_ui.paste);
            io.sat_ui.note =
                match found.len() {
                    0 => tr!("settings-tle-nothing-paste-box-looked-like"),
                    n => {
                        // Replace rather than duplicate: pasting a fresher set for
                        // a satellite already listed is the common case, and a
                        // second entry for the same catalogue number would leave
                        // whichever came first winning at random.
                        let mut replaced = 0;
                        for t in found {
                            match io.sat_edit.tles.iter().position(|e| {
                                e.norad_id().is_some() && e.norad_id() == t.norad_id()
                            }) {
                                Some(k) => {
                                    // Keep the operator's own name and their
                                    // enabled/disabled choice; take the elements.
                                    io.sat_edit.tles[k].line1 = t.line1;
                                    io.sat_edit.tles[k].line2 = t.line2;
                                    replaced += 1;
                                }
                                None => io.sat_edit.tles.push(t),
                            }
                        }
                        io.sat_ui.paste.clear();
                        match replaced {
                            0 => tr!("settings-tle-added", n = n),
                            r => tr!("settings-tle-added-refreshed", added = n - r, r = r),
                        }
                    }
                };
        }
        if ui.button(tr!("settings-tle-clear-box")).clicked() {
            io.sat_ui.paste.clear();
        }
    });
}

/// Age of a pasted element set, in seconds, from the epoch in columns 19–32 of
/// line 1.
///
/// Its own parse rather than SGP4's, because this has to work on an entry the
/// propagator would reject — the whole point is to say *why* it is being
/// rejected.
fn tle_epoch_age(t: &sdroxide_types::CustomTle, now_unix: i64) -> Option<i64> {
    let l1 = t.line1.as_bytes();
    if l1.len() < 32 {
        return None;
    }
    let field = std::str::from_utf8(&l1[18..32]).ok()?.trim();
    let yy: i64 = field.get(..2)?.parse().ok()?;
    let doy: f64 = field.get(2..)?.parse().ok()?;
    // Two-digit years: 57–99 are 1957 onwards, 00–56 are 2000 onwards. That is
    // the convention the format itself carries.
    let year = if yy < 57 { 2000 + yy } else { 1900 + yy };
    let jan1 = sdroxide_types::ymd_hms_to_unix(year, 1, 1, 0, 0, 0);
    Some(now_unix - (jan1 + ((doy - 1.0) * 86_400.0) as i64))
}

/// The frequency table the pass window shows: the operator's entries, which
/// override the built-in one satellite for satellite.
fn settings_tle_freqs(ui: &mut egui::Ui, io: &mut SettingsIo) {
    use crate::theme;

    ui.label(RichText::new(tr!("settings-tle-frequencies")).strong());
    ui.label(RichText::new(tr!("settings-tle-shown-under-pass-table-solar")).weak().size(11.0));
    ui.add_space(6.0);

    let mut remove = None;
    for (i, f) in io.sat_edit.freqs.iter_mut().enumerate() {
        ui.push_id(("sat-freq", i), |ui| {
            ui.horizontal(|ui| {
                let open = io.sat_ui.open_freq == Some(i);
                if ui.button(if open { "▼" } else { "▶" }).clicked() {
                    io.sat_ui.open_freq = (!open).then_some(i);
                }
                ui.label(
                    RichText::new(tr!("settings-tle-norad-n", norad = f.norad_id))
                        .color(theme::CYAN_DIM()),
                );
                crate::chrome::field(
                    ui,
                    egui::TextEdit::singleline(&mut f.name).desired_width(180.0).hint_text("name"),
                );
                ui.label(
                    RichText::new(tr!("settings-tle-links", n = f.links.len()))
                        .color(theme::LINE_LIT())
                        .size(10.5),
                );
                if ui
                    .button("✕")
                    .on_hover_text(tr!("settings-tle-remove-satellite-s-entry"))
                    .clicked()
                {
                    remove = Some(i);
                }
            });
            if io.sat_ui.open_freq != Some(i) {
                return;
            }
            let mut drop_link = None;
            egui::Grid::new("sat-links").num_columns(6).spacing([8.0, 4.0]).show(ui, |ui| {
                for h in [
                    tr!("settings-tle-link"),
                    tr!("settings-tle-downlink"),
                    tr!("settings-tle-uplink"),
                    tr!("settings-tle-mode"),
                    tr!("settings-tle-note"),
                    String::from(""),
                ] {
                    ui.label(RichText::new(h).color(theme::CYAN_DIM()).size(9.5).strong());
                }
                ui.end_row();
                for (k, l) in f.links.iter_mut().enumerate() {
                    crate::chrome::field(
                        ui,
                        egui::TextEdit::singleline(&mut l.label)
                            .desired_width(120.0)
                            .hint_text(tr!("settings-tle-fm-repeater")),
                    );
                    freq_box(ui, (k, "down"), &mut l.downlink, "145.800");
                    freq_box(ui, (k, "up"), &mut l.uplink, "435.250");
                    crate::chrome::field(
                        ui,
                        egui::TextEdit::singleline(&mut l.mode)
                            .desired_width(90.0)
                            .hint_text(tr!("settings-tle-fm")),
                    );
                    crate::chrome::field(
                        ui,
                        egui::TextEdit::singleline(&mut l.note)
                            .desired_width(180.0)
                            .hint_text(tr!("settings-tle-ctcss-67-0-hz")),
                    );
                    if ui.button("✕").clicked() {
                        drop_link = Some(k);
                    }
                    ui.end_row();
                }
            });
            if let Some(k) = drop_link {
                f.links.remove(k);
            }
            ui.horizontal(|ui| {
                if ui.button(tr!("settings-tle-link-2")).clicked() {
                    f.links.push(Default::default());
                }
                // The built-in row is almost always what you want to start
                // from: correcting one frequency should not mean retyping the
                // beacon, the telemetry and the transponder as well.
                if let Some(b) = sdroxide_solar::satfreq::builtin_for(f.norad_id) {
                    if ui
                        .button(tr!("settings-tle-copy-built"))
                        .on_hover_text(tr!("settings-tle-replace-links", name = b.name.to_string()))
                        .clicked()
                    {
                        f.links = b.links.clone();
                        if f.name.trim().is_empty() {
                            f.name = b.name.clone();
                        }
                    }
                }
            });
            ui.add_space(4.0);
            ui.label(
                RichText::new(tr!("settings-tle-frequency-either-one-number-145"))
                    .weak()
                    .size(10.0),
            );
        });
        ui.add_space(2.0);
    }
    if let Some(i) = remove {
        io.sat_edit.freqs.remove(i);
        io.sat_ui.open_freq = None;
    }

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut io.sat_ui.new_freq_id)
                .desired_width(80.0)
                .hint_text(tr!("settings-tle-norad")),
        );
        crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut io.sat_ui.new_freq_name)
                .desired_width(160.0)
                .hint_text("name"),
        );
        if ui.button(tr!("settings-tle-satellite")).clicked() {
            match io.sat_ui.new_freq_id.trim().parse::<u64>() {
                Ok(id) if id > 0 => {
                    let name = io.sat_ui.new_freq_name.trim().to_string();
                    let existed = io.sat_edit.freqs_for(id).is_some();
                    let entry = io.sat_edit.freqs_for_mut(id, &name);
                    // Seed from the built-in table when there is one: an entry
                    // that starts empty shadows it, which reads as the
                    // frequencies having been deleted.
                    if !existed {
                        if let Some(b) = sdroxide_solar::satfreq::builtin_for(id) {
                            entry.links = b.links.clone();
                            if entry.name.trim().is_empty() {
                                entry.name = b.name.clone();
                            }
                        } else {
                            entry.links.push(Default::default());
                        }
                    }
                    io.sat_ui.open_freq = io.sat_edit.freqs.iter().position(|f| f.norad_id == id);
                    io.sat_ui.new_freq_id.clear();
                    io.sat_ui.new_freq_name.clear();
                    io.sat_ui.note.clear();
                }
                _ => io.sat_ui.note = tr!("settings-tle-frequency-entry-needs-satellite-s"),
            }
        }
    });
}

/// A frequency box that edits an optional passband in place.
///
/// Kept as text only while it is being typed into: parsing on every keystroke
/// would fight a half-typed "145." by turning it into 145.000 under the cursor.
fn freq_box(
    ui: &mut egui::Ui,
    salt: impl std::hash::Hash + std::fmt::Debug,
    band: &mut Option<sdroxide_types::Passband>,
    hint: &str,
) {
    let id = ui.id().with(("freqbox", salt));
    let mut text = ui
        .data_mut(|d| d.get_temp::<String>(id))
        .unwrap_or_else(|| band.map(|b| b.to_string()).unwrap_or_default());
    let resp = crate::chrome::field(
        ui,
        egui::TextEdit::singleline(&mut text).desired_width(110.0).hint_text(hint),
    );
    if resp.changed() {
        *band = sdroxide_types::Passband::parse(&text);
        ui.data_mut(|d| d.insert_temp(id, text));
    } else if resp.lost_focus() {
        // Drop the in-progress text so the box re-derives from what was
        // actually stored — a half-typed "145." must not keep showing as if it
        // were a frequency the table holds.
        ui.data_mut(|d| d.remove_temp::<String>(id));
    }
}
