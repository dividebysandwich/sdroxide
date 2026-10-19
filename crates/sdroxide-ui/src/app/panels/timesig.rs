//! The time-signal panel: which station, how well it is being heard, the
//! minute being read, and how far the station computer's clock is from it.
//!
//! Three rows, top to bottom in the order an operator needs them. The station
//! row tunes: one chip per station, which both picks the decoder and puts the
//! dial a kilohertz below the carrier, and for the short-wave stations one
//! chip per frequency they share. The reading row is what is coming in — the
//! carrier, the second lock, the minute as a ribbon of bits (and WWVB's phase
//! bits under it), and the average second drawn as the keying looks. The
//! clock row is what it is for: the offset, setting the clock from it,
//! calibrating the receiver's latency, and the NTP server.
//!
//! "The clock" is the engine host's throughout. On a remote client that is
//! the station's computer, not the one in front of the operator, and the panel
//! says so rather than letting a SET CLOCK look like it acts on this screen.

use eframe::egui::{self, RichText};
use sdroxide_types::{
    ClockAuthority, Command, TimeCode, TimeSigStatus, TimeStation, Vfo, utc_ymd_hms,
};

use crate::app::SdroxideApp;
use crate::theme;

/// Above this a step needs a second press: see `Command::SetSystemClock`.
const CONFIRM_MS: f64 = 600_000.0;

/// The header row: what is selected, the lock, the counts, CLEAR RX and SAVE.
fn header(
    ui: &mut egui::Ui,
    st: &TimeSigStatus,
    status: Option<&sdroxide_types::DigiStatus>,
    cmds: &mut Vec<Command>,
) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("TIME").strong().color(theme::CYAN()));
        match st.carrier_hz {
            Some(hz) => ui
                .label(
                    RichText::new(format!("carrier {hz:.1} Hz"))
                        .size(10.0)
                        .color(theme::CYAN_DIM()),
                )
                .on_hover_text(
                    "Where the station's carrier is in the receive audio. It should be close to \
                     1000 Hz; a receiver whose reference is off puts it elsewhere, and the \
                     decoder follows it within ±300 Hz.",
                ),
            None => ui
                .label(RichText::new("no carrier").size(10.0).color(theme::YELLOW()))
                .on_hover_text(if st.station.is_hf() {
                    "Nothing stands out of the noise near 1 kHz yet. On short wave, try another \
                     of the station's frequencies: the lower ones carry at night, the higher \
                     ones by day."
                } else {
                    "Nothing stands out of the noise near 1 kHz yet. Long-wave stations need an \
                     antenna and a receiver that reach 40–80 kHz, and are strongest at night."
                }),
        };
        if st.second_lock {
            ui.label(RichText::new("LOCK").strong().color(theme::GREEN()))
                .on_hover_text("The start of each second has been found and is being tracked.");
            ui.label(
                RichText::new(format!("{:.0} dB", st.snr_db)).size(10.0).color(theme::CYAN_DIM()),
            )
            .on_hover_text(
                "How cleanly the keying reads: the gap between full and reduced carrier \
                     against the scatter of a 100 ms reading. Below about 10 dB, minutes start \
                     to fail their checks.",
            );
        } else {
            ui.label(RichText::new("hunting").weak())
                .on_hover_text("Looking for the start of the second — this takes 10–30 s.");
        }
        if let Some(db) = st.phase_snr_db {
            ui.label(
                RichText::new(format!("phase {db:.0} dB")).size(10.0).color(theme::CYAN_DIM()),
            )
            .on_hover_text(
                "How cleanly WWVB's phase code reads: a bit against the noise. It is read \
                     alongside the amplitude code, and holds on to the minute when the \
                     amplitude code's seconds start to go.",
            );
        }
        let counts = if st.frames_phase > 0 {
            format!(
                "{} good ({} by phase) · {} bad",
                st.frames_good, st.frames_phase, st.frames_bad
            )
        } else {
            format!("{} good · {} bad", st.frames_good, st.frames_bad)
        };
        ui.label(RichText::new(counts).size(10.0).color(if st.frames_bad > st.frames_good {
            theme::YELLOW()
        } else {
            theme::CYAN_DIM()
        }))
        .on_hover_text("Minutes that decoded and passed every check, and minutes that did not.");
        crate::chrome::row_tail(ui, |ui| {
            super::save_rx_chip_for(ui, status);
            super::clear_rx_chip_at(ui, cmds, true);
        });
    });
}

/// The current minute as a strip of sixty cells, and the second being read.
fn ribbon(ui: &mut egui::Ui, st: &TimeSigStatus) {
    let resp = cells(ui, &st.symbols, st.second_of_minute, 1.6);
    resp.on_hover_text(
        "This minute, one cell a second: dim for a 0, green for a 1 (MSF's two bits show green \
         for either), cyan for a marker — and for WWV's empty second 0 — red for a second that \
         could not be read. One red cell costs the minute.",
    );
    if !st.phase_symbols.is_empty() {
        let resp = cells(ui, &st.phase_symbols, st.second_of_minute, 0.8);
        resp.on_hover_text(
            "WWVB's phase code for the same seconds: dim for a 0, green for a 1, red for a second \
             too noisy to call. Its minute is found by its own sync word and checked by a Hamming \
             code, so a red cell or two here does not lose the minute.",
        );
    }
}

/// Sixty cells, one per second, `height` cells tall.
fn cells(ui: &mut egui::Ui, symbols: &[u8], now: Option<u8>, height: f32) -> egui::Response {
    let w = ui.available_width().min(720.0);
    let cell = (w / 61.0).floor().max(4.0);
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(cell * 60.0, cell * height), egui::Sense::hover());
    let p = ui.painter_at(rect);
    for i in 0..60usize {
        let r = egui::Rect::from_min_size(
            rect.min + egui::vec2(i as f32 * cell, 0.0),
            egui::vec2(cell - 1.0, rect.height()),
        );
        let color = match symbols.get(i) {
            None => theme::ROW_BG(),
            Some(&TimeSigStatus::MARKER | &TimeSigStatus::HOLE) => theme::CYAN(),
            Some(&TimeSigStatus::UNKNOWN) => theme::ALERT(),
            Some(0) => theme::CYAN_DIM().gamma_multiply(0.45),
            Some(_) => theme::GREEN(),
        };
        p.rect_filled(r, 1.0, color);
        if now == Some(i as u8) {
            p.rect_stroke(
                r,
                1.0,
                egui::Stroke::new(1.5, theme::TEXT_STRONG()),
                egui::StrokeKind::Outside,
            );
        }
    }
    resp
}

/// The average second as the keying looks: high where the carrier is full,
/// low where it is reduced. What an operator tunes by.
fn scope(ui: &mut egui::Ui, st: &TimeSigStatus) {
    let size = egui::vec2(200.0, 40.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 2.0, theme::ROW_BG());
    if st.scope.len() >= 2 {
        let n = st.scope.len() as f32;
        let pts: Vec<egui::Pos2> = st
            .scope
            .iter()
            .enumerate()
            .map(|(i, &v)| {
                egui::pos2(
                    rect.left() + rect.width() * i as f32 / (n - 1.0),
                    rect.bottom() - 3.0 - (rect.height() - 6.0) * f32::from(v) / 255.0,
                )
            })
            .collect();
        p.add(egui::Shape::line(pts, egui::Stroke::new(1.5, theme::CYAN())));
    }
    resp.on_hover_text(if st.station.is_hf() {
        "The 100 Hz time code through one second, averaged over the last fifteen or so, \
         starting at the tick. A clean station shows the pulse rising just after the left edge \
         and falling at 0.2, 0.5 or 0.8 s; noise shows a ragged line."
    } else {
        "The carrier through one second, averaged over the last fifteen or so, starting at the \
         second. A clean station shows a sharp step at the left edge and the keying after it; \
         noise shows a ragged line."
    });
}

/// "2026-10-18 12:37 UTC (14:37 local at the station)".
fn frame_text(f: &sdroxide_types::TimeFrame) -> String {
    let (y, mo, d, h, mi, _) = utc_ymd_hms(f.utc);
    let mut s = format!("{y}-{mo:02}-{d:02} {h:02}:{mi:02} UTC");
    if f.zone_min != 0 {
        let (_, _, _, lh, lm, _) = utc_ymd_hms(f.utc + i64::from(f.zone_min) * 60);
        s.push_str(&format!(
            " ({lh:02}:{lm:02} station time{})",
            if f.dst { ", summer" } else { "" }
        ));
    }
    s
}

impl SdroxideApp {
    pub(in crate::app) fn timesig_panel(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        _panel_h: f32,
    ) {
        let st: Option<TimeSigStatus> =
            self.digi_status.as_ref().and_then(|s| s.timesig.as_deref().cloned());
        let Some(st) = st else {
            ui.label(RichText::new("starting the time-signal receiver…").weak());
            return;
        };
        header(ui, &st, self.digi_status.as_ref(), cmds);
        self.timesig_stations(ui, cmds, &st);
        ui.separator();

        ui.horizontal_wrapped(|ui| {
            scope(ui, &st);
            ui.vertical(|ui| {
                ribbon(ui, &st);
                let by = match (st.station, st.last_code) {
                    (TimeStation::Wwvb, TimeCode::Both) => " (both codes)",
                    (TimeStation::Wwvb, TimeCode::Phase) => " (phase code)",
                    (TimeStation::Wwvb, TimeCode::Amplitude) => " (amplitude code)",
                    _ => "",
                };
                let line = match (&st.last_frame, st.confirmed) {
                    (Some(f), n) if n >= 2 => {
                        format!("{}{by} · {n} minutes agree", frame_text(f))
                    }
                    (Some(f), _) => {
                        format!("{}{by} · waiting for the next minute to agree", frame_text(f))
                    }
                    (None, _) if st.second_lock => "reading the minute…".to_string(),
                    (None, _) => String::new(),
                };
                ui.label(RichText::new(line).monospace().size(11.0).color(theme::TEXT()));
                if let Some(f) = &st.last_frame {
                    let mut notes = Vec::new();
                    if f.dst_change_soon {
                        notes.push("summer-time change announced".to_string());
                    }
                    if f.leap_second_soon {
                        notes.push("leap second announced".to_string());
                    }
                    if let Some(d) = f.dut1_ms {
                        notes.push(format!("DUT1 {:+.1} s", f64::from(d) / 1000.0));
                    }
                    if !notes.is_empty() {
                        ui.label(RichText::new(notes.join(" · ")).size(10.0).weak());
                    }
                }
            });
        });
        ui.separator();
        self.timesig_clock(ui, cmds, &st);
    }

    /// One chip per station, and for the short-wave stations one per
    /// frequency. A station's chip picks the decoder and tunes the dial; a
    /// frequency's only tunes.
    fn timesig_stations(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, st: &TimeSigStatus) {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("STATION").size(10.0).weak());
            for s in TimeStation::ALL {
                let khz = |hz: f64| hz / 1000.0;
                let tip = if s.is_hf() {
                    let freqs: Vec<String> =
                        s.carriers_hz().iter().map(|f| format!("{}", f / 1e6)).collect();
                    format!(
                        "{} — {}, on {} MHz. Tunes to {} MHz with the dial 1 kHz below, in USB. \
                         Its seconds are timed by the station's own ticks; the other NIST station \
                         on the same frequencies sends the same time code.",
                        s.label(),
                        s.site(),
                        freqs.join(", "),
                        s.carrier_hz() / 1e6
                    )
                } else {
                    format!(
                        "{} kHz — {}. Tunes the dial to {} kHz USB, which puts the carrier at 1 kHz.",
                        khz(s.carrier_hz()),
                        s.site(),
                        khz(s.dial_hz())
                    )
                };
                let chip = crate::chrome::chip(ui, st.station == s, RichText::new(s.label()).size(10.5))
                    .on_hover_text(tip);
                if chip.clicked() && self.digi_cfg_seeded {
                    self.digi_cfg_edit.timesig.station = s;
                    cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
                    // Staying on a frequency the station shares with the one
                    // just left: the press is about which ticks to time, not
                    // where to listen.
                    let dial = self.state.rx_freq_hz();
                    let here = s.carriers_hz().iter().any(|&f| TimeStation::dial_for(f) == dial);
                    if !here {
                        cmds.push(Command::SetVfo { vfo: Vfo::A, hz: s.dial_hz() });
                    }
                }
            }
            if let (Some(km), Some(ms)) = (st.distance_km, st.propagation_ms) {
                let hops = match st.propagation_hops {
                    Some(1) => " · 1 hop".to_string(),
                    Some(n) => format!(" · {n} hops"),
                    None => String::new(),
                };
                let text = format!("{km:.0} km · +{ms:.2} ms{hops}");
                ui.label(RichText::new(text).size(10.0).weak())
                    .on_hover_text(if st.station.is_hf() {
                        "The distance from your locator to the transmitter, and the signal's \
                         travel time by sky wave — up to the ionosphere's F2 layer and down, in \
                         as few hops as reach — already added to the measurement. The layer \
                         moves with the time of day, so this is good to half a millisecond or so."
                    } else {
                        "The distance from your locator to the transmitter, and the signal's \
                         travel time over it, already added to the measurement."
                    });
            } else {
                ui.label(RichText::new("no locator").size(10.0).color(theme::YELLOW()))
                    .on_hover_text(
                        "Set your grid square (Settings → General) and the signal's travel time from \
                         the transmitter is added in — about 3.3 ms per 1000 km.",
                    );
            }
        });
        if st.station.is_hf() {
            self.timesig_frequencies(ui, cmds, st);
        }
    }

    /// The short-wave station's frequencies, and what the receiver hears of
    /// the station that shares them.
    fn timesig_frequencies(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        st: &TimeSigStatus,
    ) {
        let dial = self.state.rx_freq_hz();
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("MHz").size(10.0).weak());
            for &f in st.station.carriers_hz() {
                let on = TimeStation::dial_for(f) == dial;
                let chip =
                    crate::chrome::chip(ui, on, RichText::new(format!("{}", f / 1e6)).size(10.5))
                        .on_hover_text(format!(
                            "Tune to {} MHz: the dial {} kHz, USB.",
                            f / 1e6,
                            TimeStation::dial_for(f) / 1000.0
                        ));
                if chip.clicked() {
                    cmds.push(Command::SetVfo { vfo: Vfo::A, hz: TimeStation::dial_for(f) });
                }
            }
            match (st.sibling_heard, st.ticks_heard) {
                (Some(other), false) => {
                    ui.label(
                        RichText::new(format!("only {}'s ticks are heard here", other.label()))
                            .size(10.0)
                            .color(theme::YELLOW()),
                    );
                    let switch = crate::chrome::chip(
                        ui,
                        false,
                        RichText::new(format!("LISTEN TO {}", other.label())).size(10.0),
                    )
                    .on_hover_text(
                        "The time code is the same from both stations; the ticks say whose time \
                         is being measured, and so whose distance to add. Switching keeps this \
                         frequency.",
                    );
                    if switch.clicked() && self.digi_cfg_seeded {
                        self.digi_cfg_edit.timesig.station = other;
                        cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
                    }
                }
                (Some(other), true) => {
                    ui.label(
                        RichText::new(format!("{}'s ticks are heard too", other.label()))
                            .size(10.0)
                            .weak(),
                    )
                    .on_hover_text(
                        "Both NIST stations are coming in on this frequency. Each is timed by its \
                         own ticks, so the measurement is this station's; the shared time code \
                         can read a little worse where the two arrive out of step.",
                    );
                }
                _ => {}
            }
        });
    }

    /// The offset, and everything that acts on it.
    fn timesig_clock(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>, st: &TimeSigStatus) {
        let remote = self.ctrl.engine_is_remote();
        let whose = if remote { "The station's clock" } else { "This computer's clock" };
        ui.horizontal_wrapped(|ui| match (st.offset_ms, st.jitter_ms) {
            (Some(off), Some(j)) => {
                let (word, color) = if off.abs() < 1.0 {
                    ("right".to_string(), theme::GREEN())
                } else if off > 0.0 {
                    (format!("{} slow", human(off)), theme::YELLOW())
                } else {
                    (format!("{} fast", human(-off)), theme::YELLOW())
                };
                ui.label(RichText::new(format!("{whose} is {word}")).strong().color(color));
                ui.label(
                    RichText::new(format!("{off:+.1} ms ±{j:.1}")).monospace().size(10.5).weak(),
                )
                .on_hover_text(
                    "The median of the last minute's measurements, one a second, and \
                             their scatter.",
                );
            }
            _ => {
                ui.label(RichText::new(format!("{whose}: not measured yet")).weak()).on_hover_text(
                    "Two consecutive minutes have to decode and agree before the received \
                         time is believed.",
                );
            }
        });

        let clock = &st.clock;
        ui.horizontal_wrapped(|ui| {
            let can = st.offset_ms.is_some()
                && clock.authority != ClockAuthority::Unavailable
                && !clock.step_pending;
            let confirm_id = ui.id().with("timesig-confirm");
            let big = st.offset_ms.is_some_and(|o| o.abs() > CONFIRM_MS);
            // Armed for this step only: a measurement that has since come back
            // small (or gone) needs no confirming.
            let armed = big && ui.data(|d| d.get_temp(confirm_id)).unwrap_or(false);
            let label = if armed { " CONFIRM SET CLOCK " } else { " SET CLOCK " };
            let resp = crate::chrome::chip_accent_enabled(
                ui,
                can,
                armed,
                label,
                Some(10.5),
                theme::CYAN(),
                theme::INK_ON_CYAN(),
            );
            let resp = if can {
                resp.on_hover_text(match clock.authority {
                    ClockAuthority::Direct => "Move the clock to the received time now.",
                    _ => {
                        "Move the clock to the received time. The system asks for an administrator \
                          password first."
                    }
                })
            } else {
                resp.on_disabled_hover_text(if clock.step_pending {
                    "Waiting for the administrator password."
                } else if clock.authority == ClockAuthority::Unavailable {
                    "This program cannot set the clock on that computer: run it as an \
                     administrator, or let the NTP server hand the time to the system's own \
                     time service."
                } else {
                    "Nothing to set from until the received time is believed."
                })
            };
            if resp.clicked() {
                if big && !armed {
                    // A clock that far out is a clock never set or a decode
                    // gone wrong; the operator is the one who knows which.
                    ui.data_mut(|d| d.insert_temp(confirm_id, true));
                } else {
                    ui.data_mut(|d| d.insert_temp(confirm_id, false));
                    cmds.push(Command::SetSystemClock { force: big });
                }
            }
            if armed {
                ui.label(
                    RichText::new(format!(
                        "that moves the clock by {} — press again to confirm",
                        human(st.offset_ms.unwrap_or(0.0).abs())
                    ))
                    .size(10.0)
                    .color(theme::YELLOW()),
                );
            }

            let auto_ok = clock.authority == ClockAuthority::Direct;
            let auto = self.digi_cfg_edit.timesig.auto_set;
            let auto_resp =
                crate::chrome::chip_enabled(ui, auto_ok && self.digi_cfg_seeded, auto, "AUTO");
            let auto_resp = if auto_ok {
                auto_resp.on_hover_text(format!(
                    "Correct the clock by itself whenever it is more than {:.0} ms out, at most \
                     once every ten minutes. Never a step that would need confirming.",
                    self.digi_cfg_edit.timesig.auto_threshold_ms
                ))
            } else {
                auto_resp.on_disabled_hover_text(
                    "Only when the program may set the clock without a password: running as an \
                     administrator, or on Linux with CAP_SYS_TIME.",
                )
            };
            if auto_resp.clicked() && auto_ok && self.digi_cfg_seeded {
                self.digi_cfg_edit.timesig.auto_set = !auto;
                cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
            }

            // Calibration: the operator vouches that the clock is right now
            // (kept by NTP, say), so whatever the measurement still says is the
            // receiver's own delay.
            let cal_ok = st.offset_ms.is_some() && self.digi_cfg_seeded;
            let cal = crate::chrome::chip_enabled(ui, cal_ok, false, "CALIBRATE");
            let cal = cal.on_hover_text(format!(
                "Only when {} is already known to be right — kept by NTP from the internet, \
                 say. Takes whatever the measurement still shows as the receiver's own delay \
                 (buffers and filters), and subtracts it from now on. Now {:.1} ms.",
                whose.to_lowercase(),
                self.digi_cfg_edit.timesig.latency_ms
            ));
            if cal.clicked()
                && cal_ok
                && let Some(off) = st.offset_ms
            {
                self.digi_cfg_edit.timesig.latency_ms -= off;
                cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
            }
            if self.digi_cfg_edit.timesig.latency_ms != 0.0 && self.digi_cfg_seeded {
                let reset = crate::chrome::chip(
                    ui,
                    false,
                    RichText::new(format!(
                        "latency {:.1} ms ✕",
                        self.digi_cfg_edit.timesig.latency_ms
                    ))
                    .size(10.0),
                )
                .on_hover_text("The calibrated receiver latency. Press to forget it.");
                if reset.clicked() {
                    self.digi_cfg_edit.timesig.latency_ms = 0.0;
                    cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
                }
            }
        });

        let mut notes: Vec<(String, egui::Color32)> = Vec::new();
        if let Some(last) = &clock.last_step {
            notes.push((last.clone(), theme::CYAN_DIM()));
        }
        if let Some(d) = &clock.sync_daemon {
            notes
                .push((format!("{d} also keeps this clock and may move it back"), theme::YELLOW()));
        }
        if remote {
            notes.push(("acts on the station's computer, not this one".into(), theme::CYAN_DIM()));
        }
        let ntp = &clock.ntp;
        notes.push(if ntp.running {
            (
                format!(
                    "NTP server on {} · {} · {} answered",
                    ntp.addr,
                    if ntp.synced { "serving radio time" } else { "unsynchronised" },
                    ntp.requests
                ),
                if ntp.synced { theme::GREEN() } else { theme::CYAN_DIM() },
            )
        } else if let Some(e) = &ntp.error {
            (format!("NTP server could not start: {e}"), theme::ALERT())
        } else {
            ("NTP server off (Settings → Servers)".into(), theme::CYAN_DIM())
        });
        for (text, color) in notes {
            ui.label(RichText::new(text).size(10.0).color(color));
        }
    }
}

/// A duration for a sentence: "3.2 ms", "1.25 s", "1 h 0 min".
fn human(ms: f64) -> String {
    if ms < 1000.0 {
        format!("{ms:.1} ms")
    } else if ms < 120_000.0 {
        format!("{:.2} s", ms / 1000.0)
    } else if ms < 7_200_000.0 {
        format!("{:.0} min", ms / 60_000.0)
    } else {
        let h = (ms / 3_600_000.0).floor();
        format!("{h:.0} h {:.0} min", (ms - h * 3_600_000.0) / 60_000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_read_as_words() {
        assert_eq!(human(3.25), "3.2 ms");
        assert_eq!(human(1250.0), "1.25 s");
        assert_eq!(human(600_000.0), "10 min");
        assert_eq!(human(3_660_000.0), "61 min");
        assert_eq!(human(7_260_000.0), "2 h 1 min");
    }
}
