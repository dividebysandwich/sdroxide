//! The UI tab: layout, theme and font sizes, the frame rate, the waterfall's
//! palette and the colours the overlays are drawn in, and the spoken
//! announcements.
//!
//! These take effect live — the frame rate reaches the engine through the
//! spectrum-config diff on the next frame, and the rest is read straight out of
//! the settings each frame.
//!
//! The panadapter's own settings — its detail, the spectrum's reaction and the
//! waterfall's scroll speed — used to be here and are now on the SPEC popup,
//! beside the picture they change (`SdroxideApp::panadapter_controls`). They
//! are still the same [`sdroxide_types::UiSettings`] fields, written and
//! persisted the same way.

use eframe::egui::{self, Color32, ComboBox, RichText};
use sdroxide_types::{BandplanKind, CallsignStyle, FreqStyle, SpeechSettings, SpotKind, Verbosity};

use crate::colormap;

use crate::app::settings::enum_combo;
use crate::app::settings::general::device_combo;
use crate::app::speech::SpeechStatus;
use crate::chrome::StyledCombo;

pub(in crate::app) fn settings_ui_tab(
    ui: &mut egui::Ui,
    cfg: &mut sdroxide_types::UiSettings,
    cloud_march: Option<&mut bool>,
) {
    use sdroxide_types::{ChromeStyle, FontSize, LayoutMode, UiSettings, UiTheme};
    ui.label(RichText::new("Display").size(14.0).strong().color(crate::theme::CYAN()));
    ui.add_space(6.0);
    // Said here because this is where they used to be, and where an operator
    // who remembers them will come looking. They moved to the panadapter's own
    // button, beside the picture they change — see
    // `SdroxideApp::panadapter_controls`.
    ui.label(
        RichText::new(
            "Panadapter detail, the spectrum's reaction and the waterfall's scroll speed are \
             set from the SPEC button on the display strip — the DISP menu on a narrow window.",
        )
        .size(11.0)
        .weak(),
    );
    ui.add_space(6.0);
    egui::Grid::new("ui-grid").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
        ui.label("Layout");
        enum_combo(ui, "ui-layout", &mut cfg.layout, &LayoutMode::ALL, LayoutMode::label);
        ui.end_row();

        ui.label("Theme");
        enum_combo(ui, "ui-theme", &mut cfg.theme, &UiTheme::ALL, UiTheme::label);
        ui.end_row();

        ui.label("Button style");
        enum_combo(
            ui,
            "ui-btn-style",
            &mut cfg.button_style,
            &ChromeStyle::ALL,
            ChromeStyle::label,
        );
        ui.end_row();

        ui.label("Window style");
        enum_combo(
            ui,
            "ui-win-style",
            &mut cfg.window_style,
            &ChromeStyle::ALL,
            ChromeStyle::label,
        );
        ui.end_row();

        ui.label("Screen update rate");
        ComboBox::from_id_salt("ui-fps")
            .selected_text(format!("{} fps", cfg.frame_rate_fps))
            .show_styled(ui, |ui| {
                for f in UiSettings::FPS_OPTIONS {
                    ui.selectable_value(&mut cfg.frame_rate_fps, f, format!("{f} fps"));
                }
            });
        ui.end_row();

        ui.label("Waterfall palette");
        ComboBox::from_id_salt("ui-palette")
            .selected_text(colormap::NAMES[cfg.waterfall_palette.min(colormap::NAMES.len() - 1)])
            .show_styled(ui, |ui| {
                for (i, name) in colormap::NAMES.iter().enumerate() {
                    ui.selectable_value(&mut cfg.waterfall_palette, i, *name);
                }
            });
        ui.end_row();

        ui.label("Spectrum background");
        ui.horizontal(|ui| {
            crate::chrome::checkbox(ui, &mut cfg.spectrum_gradient, "Gradient");
            ui.add_enabled_ui(cfg.spectrum_gradient, |ui| {
                ui.label("top");
                ui.color_edit_button_srgb(&mut cfg.gradient_top);
                ui.label("bottom");
                ui.color_edit_button_srgb(&mut cfg.gradient_bottom);
            });
        });
        ui.end_row();

        ui.label("Spot label colours").on_hover_text(
            "The tint each spot source wears: the boxes along the bottom of the \
             waterfall, the badges in the SPOTS list and the dots on the world map.",
        );
        ui.horizontal_wrapped(|ui| {
            for kind in SpotKind::ALL {
                ui.color_edit_button_srgb(&mut cfg.spot_colors[kind.index()])
                    .on_hover_text(format!("Colour for {} spots", kind.label()));
                let [r, g, b] = cfg.spot_colors[kind.index()];
                // Tinted with what was just picked, so the row is its own
                // preview — a colour that vanishes into the panel here would
                // vanish into the waterfall too.
                ui.label(
                    RichText::new(kind.label())
                        .size(11.0)
                        .strong()
                        .color(crate::theme::data_ink((r, g, b))),
                );
                ui.add_space(6.0);
            }
            if ui
                .button("Reset")
                .on_hover_text("Put every spot colour back to its default")
                .clicked()
            {
                for kind in SpotKind::ALL {
                    let (r, g, b) = kind.default_color();
                    cfg.spot_colors[kind.index()] = [r, g, b];
                }
            }
        });
        ui.end_row();

        ui.label("Band plan colours").on_hover_text(
            "The shade each class of allocation is painted in on the band-plan \
             strip along the bottom of the waterfall. The blocks are drawn \
             semi-transparent over the waterfall, so they land darker there \
             than in the swatch here.",
        );
        ui.horizontal_wrapped(|ui| {
            for kind in BandplanKind::ALL {
                ui.color_edit_button_srgb(&mut cfg.bandplan_colors[kind.index()])
                    .on_hover_text(format!("Colour for {} allocations", kind.label()));
                let [r, g, b] = cfg.bandplan_colors[kind.index()];
                ui.label(
                    RichText::new(kind.label())
                        .size(11.0)
                        .strong()
                        .color(crate::theme::data_ink((r, g, b))),
                );
                ui.add_space(6.0);
            }
            if ui
                .button("Reset")
                .on_hover_text("Put every band-plan colour back to its default")
                .clicked()
            {
                for kind in BandplanKind::ALL {
                    let (r, g, b) = kind.default_color();
                    cfg.bandplan_colors[kind.index()] = [r, g, b];
                }
            }
        });
        ui.end_row();

        ui.label("Skimmer font size");
        enum_combo(ui, "ui-skim-font", &mut cfg.skimmer_font_size, &FontSize::ALL, FontSize::label);
        ui.end_row();

        ui.label("Waterfall / spectrum font size");
        enum_combo(ui, "ui-wf-font", &mut cfg.waterfall_font_size, &FontSize::ALL, FontSize::label);
        ui.end_row();

        ui.label("Interface font size").on_hover_text(
            "Scales the whole interface — menus, dialogs, windows, the radio \
             tabs, the top bar and its buttons. The two sizes above are \
             relative to it.",
        );
        enum_combo(ui, "ui-menu-font", &mut cfg.menu_font_size, &FontSize::ALL, FontSize::label);
        ui.end_row();

        ui.label("Cities on maps").on_hover_text(
            "Draw the world's cities — a dot per place, with its name beside it \
             where there is room — on the flat maps: FT8/WSPR, APRS, ADS-B and \
             AIS.\n\n\
             They are most of what says *where* a dot is on a map of the whole \
             world, and they are also the busiest thing on it. Turn them off if \
             the names are getting in the way of the stations you are reading. \
             The 3D globe is unaffected — its cities are night-side lights \
             rather than markers, and nothing there is written across a \
             contact.",
        );
        crate::chrome::checkbox(ui, &mut cfg.map_cities, "show cities and their names");
        ui.end_row();
    });

    // Native only: the check is about *this binary* being out of date, and
    // the browser client has no binary of its own to be behind — its station
    // serves it fresh every time.
    #[cfg(not(target_arch = "wasm32"))]
    {
        ui.add_space(10.0);
        crate::chrome::checkbox(ui, &mut cfg.update_check, "Check for updates at startup")
            .on_hover_text(
                "Asks sdroxide.com once per start whether a newer release has been \
                 published, and says so in a dismissable banner above the waterfall. \
                 Nothing is sent beyond the request itself.",
            );
    }

    let Some(cloud_march) = cloud_march else { return };
    ui.add_space(14.0);
    ui.label(RichText::new("3D view").size(14.0).strong().color(crate::theme::CYAN()));
    ui.add_space(6.0);
    egui::Grid::new("ui-grid-3d").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
        ui.label("Cloud rendering");
        ComboBox::from_id_salt("ui-cloud-march")
            .selected_text(if *cloud_march { "Volumetric" } else { "Layered" })
            .show_styled(ui, |ui| {
                ui.selectable_value(cloud_march, false, "Layered");
                ui.selectable_value(cloud_march, true, "Volumetric");
            });
        ui.end_row();
    });
    ui.add_space(8.0);
    ui.label(
        RichText::new(
            "How the CLOUDS layer in the 3D view draws the weather. Layered stacks \
             slices through the troposphere and is the cheap option. Volumetric walks \
             a ray through it instead, so the Sun casts the cloud tops onto the deck \
             below and lightning glows out through the storm making it rather than \
             only brightening its outside — at several times the cost per pixel.",
        )
        .weak(),
    );
}

/// Spoken announcements.
///
/// Sized so the UI tab stays scannable: the controls an operator sets up
/// once are visible, and the two dozen per-category switches live behind a
/// collapsing header. Every value is written as it is changed, like the control
/// bindings — the rate and volume take effect on the next utterance without a
/// restart, and there is no separate step for an APPLY to stand for.
pub(in crate::app) fn speech_settings(
    ui: &mut egui::Ui,
    cfg: &mut SpeechSettings,
    voices: &[String],
    outputs: &[String],
    status: &SpeechStatus,
    test: &mut bool,
) {
    ui.label(RichText::new("Voice announcements").size(14.0).strong().color(crate::theme::CYAN()));
    ui.add_space(6.0);
    crate::chrome::checkbox(ui, &mut cfg.enabled, "Speak changes to the radio")
        .on_hover_text("Reads out what changed, so the radio can be operated without seeing it");

    ui.add_enabled_ui(cfg.enabled, |ui| {
        egui::Grid::new("speech-grid").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
            ui.label("Voice");
            let shown =
                if cfg.voice.is_empty() { "Shipped voice".to_string() } else { cfg.voice.clone() };
            ComboBox::from_id_salt("speech-voice").width(300.0).selected_text(shown).show_styled(
                ui,
                |ui| {
                    if ui.selectable_label(cfg.voice.is_empty(), "Shipped voice").clicked() {
                        cfg.voice.clear();
                    }
                    for v in voices {
                        if ui.selectable_label(&cfg.voice == v, v).clicked() {
                            cfg.voice = v.clone();
                        }
                    }
                },
            );
            ui.end_row();

            ui.label("Speed");
            crate::chrome::slider(
                ui,
                egui::Slider::new(&mut cfg.rate, SpeechSettings::RATE_RANGE)
                    .step_by(0.1)
                    .suffix("×"),
            )
            .on_hover_text(
                "The voice stretches or compresses its own phrasing, so the pitch does not \
                 change. Past about 2× it stops getting shorter.",
            );
            ui.end_row();

            ui.label("Volume");
            crate::chrome::slider(ui, egui::Slider::new(&mut cfg.volume, 0.0..=1.0).step_by(0.05));
            ui.end_row();

            ui.label("Output");
            // `device_combo` borrows the current selection while handing the
            // new one to the closure, so the two cannot both be `cfg.device`.
            let cur = cfg.device.clone();
            device_combo(ui, "speech-out", outputs, &cur, |n| cfg.device = n);
            ui.end_row();

            ui.label("Detail");
            crate::app::settings::enum_combo(
                ui,
                "speech-verbosity",
                &mut cfg.verbosity,
                &Verbosity::ALL,
                Verbosity::label,
            );
            ui.end_row();

            ui.label("Duck receiver");
            ui.horizontal(|ui| {
                crate::chrome::checkbox(ui, &mut cfg.duck_rx, "While speaking");
                ui.add_enabled_ui(cfg.duck_rx, |ui| {
                    crate::chrome::slider(
                        ui,
                        egui::Slider::new(&mut cfg.duck_level, 0.0..=1.0).step_by(0.05),
                    );
                });
            });
            ui.end_row();
        });

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.button("Test").clicked() {
                *test = true;
            }
            if let Some(note) = status.note() {
                let text = RichText::new(note);
                ui.label(if status.is_failed() {
                    text.color(Color32::from_rgb(0xE0, 0x6C, 0x4B))
                } else {
                    text.weak()
                });
            }
        });

        ui.add_space(4.0);
        egui::CollapsingHeader::new("What to announce").default_open(false).show(ui, |ui| {
            egui::Grid::new("speech-cats").num_columns(2).spacing([16.0, 4.0]).show(ui, |ui| {
                let c = &mut cfg.cat;
                crate::chrome::checkbox(ui, &mut c.frequency, "Frequency");
                crate::chrome::checkbox(ui, &mut c.mode_band, "Mode and band");
                ui.end_row();
                crate::chrome::checkbox(ui, &mut c.vfo_split, "VFO and split");
                crate::chrome::checkbox(ui, &mut c.agc_gain, "AGC and gain");
                ui.end_row();
                crate::chrome::checkbox(ui, &mut c.levels, "Drive, tune and mic");
                crate::chrome::checkbox(ui, &mut c.ptt, "Transmit and receive");
                ui.end_row();
                crate::chrome::checkbox(ui, &mut c.rit_xit, "RIT and XIT");
                crate::chrome::checkbox(ui, &mut c.memory_scan, "Memories and scanning");
                ui.end_row();
                crate::chrome::checkbox(ui, &mut c.band_edge, "Leaving an amateur band");
                crate::chrome::checkbox(ui, &mut c.notices, "Warnings and messages");
                ui.end_row();
                crate::chrome::checkbox(ui, &mut c.filters, "Filters, squelch and noise reduction")
                    .on_hover_text("Off by default: these move constantly while chasing a signal");
                ui.end_row();
            });

            ui.add_space(8.0);
            ui.label(RichText::new("Decoded messages").strong());
            egui::Grid::new("speech-decodes").num_columns(2).spacing([16.0, 4.0]).show(ui, |ui| {
                let d = &mut cfg.decodes;
                crate::chrome::checkbox(ui, &mut d.ft8_to_me, "FT8 calls to me");
                crate::chrome::checkbox(ui, &mut d.ft8_cq_for_me, "FT8 CQs I could answer")
                    .on_hover_text(
                        "A busy evening on twenty metres is a hundred of these a minute",
                    );
                ui.end_row();
                crate::chrome::checkbox(ui, &mut d.js8, "JS8 messages to me");
                crate::chrome::checkbox(ui, &mut d.js8_allcall, "JS8 @ALLCALL too");
                ui.end_row();
                crate::chrome::checkbox(ui, &mut d.fsq, "FSQ messages to me");
                crate::chrome::checkbox(ui, &mut d.include_snr, "Include the report")
                    .on_hover_text(
                        "Only where the message carries none of its own — a decode that already \
                     reports a number does not also get ours",
                    );
                ui.end_row();
                crate::chrome::checkbox(ui, &mut d.ft8_qso, "My own FT8 exchange")
                    .on_hover_text("What the sequencer is about to send, and the contact ending");
                ui.end_row();
            });

            ui.add_space(8.0);
            ui.label(RichText::new("Reading decoded text aloud").strong());
            ui.horizontal(|ui| {
                crate::chrome::checkbox(ui, &mut cfg.text.cw, "CW");
                crate::chrome::checkbox(ui, &mut cfg.text.rtty_psk, "RTTY, PSK, Olivia, THOR, FSQ");
            });
            crate::chrome::checkbox(
                ui,
                &mut cfg.text.cw_only_when_locked,
                "CW only while the decoder is locked",
            )
            .on_hover_text("Reading an unlocked decoder's output is worse than silence");
            ui.label(
                RichText::new(
                    "Both are off by default. A decoder produces text faster than speech reads \
                     it, so anything that falls too far behind the live audio is dropped rather \
                     than queued.",
                )
                .weak(),
            );

            ui.add_space(8.0);
            ui.label(RichText::new("Tuning up").strong());
            crate::chrome::checkbox(
                ui,
                &mut cfg.tune.swr_while_tuning,
                "Read the SWR out while TUNE is held",
            );
            ui.add_enabled_ui(cfg.tune.swr_while_tuning, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Every");
                    crate::chrome::slider(
                        ui,
                        egui::Slider::new(&mut cfg.tune.period_s, 1.0..=10.0)
                            .step_by(0.5)
                            .suffix(" s"),
                    );
                });
            });
            crate::chrome::checkbox(
                ui,
                &mut cfg.tune.summary_after_tune,
                "Report the best match on release",
            );
            crate::chrome::checkbox(
                ui,
                &mut cfg.tune.alarm_always,
                "Warn about high SWR during any transmission",
            );

            ui.add_space(8.0);
            ui.label(RichText::new("How things are read").strong());
            egui::Grid::new("speech-style").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                ui.label("Frequencies");
                crate::app::settings::enum_combo(
                    ui,
                    "speech-freq-style",
                    &mut cfg.freq_style,
                    &FreqStyle::ALL,
                    FreqStyle::label,
                );
                ui.end_row();
                ui.label("Callsigns");
                crate::app::settings::enum_combo(
                    ui,
                    "speech-call-style",
                    &mut cfg.callsign_style,
                    &CallsignStyle::ALL,
                    CallsignStyle::label,
                );
                ui.end_row();
            });

            ui.add_space(6.0);
            crate::chrome::checkbox(ui, &mut cfg.duck_on_ptt, "Stay quiet while transmitting")
                .on_hover_text(
                    "Speech goes to your speakers, and therefore into your microphone. High-SWR \
                 warnings still get through.",
                );
        });
    });

    ui.add_space(8.0);
    ui.label(
        RichText::new(
            "Announcements play on their own sound device, so they are never recorded and never \
             sent to anyone listening remotely. Keys for speaking the status, repeating the last \
             announcement and stopping mid-sentence are on the Controls tab.",
        )
        .weak(),
    );
}
