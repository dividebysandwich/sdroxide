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
use crate::i18n::{Tr, tr};

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
    ui.label(
        RichText::new(tr!("settings-ui-display")).size(14.0).strong().color(crate::theme::CYAN()),
    );
    ui.add_space(6.0);
    // Said here because this is where they used to be, and where an operator
    // who remembers them will come looking. They moved to the panadapter's own
    // button, beside the picture they change — see
    // `SdroxideApp::panadapter_controls`.
    ui.label(RichText::new(tr!("settings-ui-moved-note")).size(11.0).weak());
    ui.add_space(6.0);
    egui::Grid::new("ui-grid").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
        ui.label(tr!("settings-ui-language")).on_hover_text(tr!("settings-ui-language", "hint"));
        language_combo(ui, &mut cfg.language);
        ui.end_row();

        ui.label(tr!("settings-ui-layout"));
        enum_combo(ui, "ui-layout", &mut cfg.layout, &LayoutMode::ALL, LayoutMode::tr);
        ui.end_row();

        ui.label(tr!("settings-ui-theme"));
        enum_combo(ui, "ui-theme", &mut cfg.theme, &UiTheme::ALL, UiTheme::tr);
        ui.end_row();

        ui.label(tr!("settings-ui-button-style"));
        enum_combo(ui, "ui-btn-style", &mut cfg.button_style, &ChromeStyle::ALL, ChromeStyle::tr);
        ui.end_row();

        ui.label(tr!("settings-ui-window-style"));
        enum_combo(ui, "ui-win-style", &mut cfg.window_style, &ChromeStyle::ALL, ChromeStyle::tr);
        ui.end_row();

        // The **undocked** modules. A window arrangement, chosen once and kept,
        // rather than something reached for mid-QSO — so it lives here with the
        // other display settings. One row per [`DetachableModule`], so the next
        // detachable module appears here for free. Native only: the browser
        // keeps every module in its one window.
        #[cfg(not(target_arch = "wasm32"))]
        {
            use sdroxide_types::DetachableModule;
            for module in DetachableModule::ALL {
                let mut on = cfg.is_detached(module);
                ui.label(tr!("settings-ui-window-label", module = module_label(module)));
                ComboBox::from_id_salt(("ui-undocked", module))
                    .selected_text(if on {
                        tr!("settings-ui-undocked")
                    } else {
                        tr!("settings-ui-docked")
                    })
                    .show_styled(ui, |ui| {
                        ui.selectable_value(&mut on, false, tr!("settings-ui-docked"));
                        ui.selectable_value(&mut on, true, tr!("settings-ui-undocked"));
                    })
                    .response
                    .on_hover_text(tr!("settings-ui-undock-hint", app_id = module.app_id()));
                cfg.set_detached(module, on);
                ui.end_row();
            }
            // Both directions at once, for arranging a whole screen in one
            // press: every module out, or every module home. Shown only while
            // they would do something, so neither is a button that changes
            // nothing.
            let any_out = DetachableModule::ALL.iter().any(|m| cfg.is_detached(*m));
            let any_in = DetachableModule::ALL.iter().any(|m| !cfg.is_detached(*m));
            ui.label("");
            ui.horizontal(|ui| {
                if any_out
                    && crate::chrome::chip(ui, false, tr!("multi-dock-all-windows"))
                        .on_hover_text(tr!("settings-ui-dock-all-hint"))
                        .clicked()
                {
                    for m in DetachableModule::ALL {
                        cfg.set_detached(m, false);
                    }
                }
                if any_in
                    && crate::chrome::chip(ui, false, tr!("multi-undock-all-windows"))
                        .on_hover_text(tr!("settings-ui-undock-all-hint"))
                        .clicked()
                {
                    for m in DetachableModule::ALL {
                        cfg.set_detached(m, true);
                    }
                }
            });
            ui.end_row();
        }

        ui.label(tr!("settings-ui-frame-rate"));
        ComboBox::from_id_salt("ui-fps")
            .selected_text(format!("{fps} fps", fps = cfg.frame_rate_fps))
            .show_styled(ui, |ui| {
                for f in UiSettings::FPS_OPTIONS {
                    ui.selectable_value(&mut cfg.frame_rate_fps, f, format!("{fps} fps", fps = f));
                }
            });
        ui.end_row();

        ui.label(tr!("settings-ui-palette"));
        ComboBox::from_id_salt("ui-palette")
            .selected_text(colormap::NAMES[cfg.waterfall_palette.min(colormap::NAMES.len() - 1)])
            .show_styled(ui, |ui| {
                for (i, name) in colormap::NAMES.iter().enumerate() {
                    ui.selectable_value(&mut cfg.waterfall_palette, i, *name);
                }
            });
        ui.end_row();

        ui.label(tr!("settings-ui-tuning-buttons"));
        ui.horizontal(|ui| {
            crate::chrome::checkbox(
                ui,
                &mut cfg.tune_step_buttons,
                tr!("settings-ui-tuning-buttons-show"),
            )
            .on_hover_text(tr!("settings-ui-tuning-buttons-show", "hint"));
            ui.label(
                RichText::new(tr!("settings-ui-tuning-step", step = cfg.tune_step_label())).weak(),
            );
        });
        ui.end_row();

        // Only meaningful with the row above, so it sits under it.
        if cfg.tune_step_buttons {
            ui.label("");
            crate::chrome::checkbox(
                ui,
                &mut cfg.tune_step_round_first,
                tr!("settings-ui-tuning-snap"),
            )
            .on_hover_text(tr!("settings-ui-tuning-snap", "hint"));
            ui.end_row();
        }

        ui.label(tr!("settings-ui-smoothing"));
        crate::chrome::checkbox(
            ui,
            &mut cfg.waterfall_smooth,
            tr!("settings-ui-smoothing-interpolate"),
        )
        .on_hover_text(tr!("settings-ui-smoothing-interpolate", "hint"));
        ui.end_row();

        ui.label(tr!("settings-ui-clock"));
        crate::chrome::checkbox(ui, &mut cfg.panadapter_clock, tr!("settings-ui-clock-show"))
            .on_hover_text(tr!("settings-ui-clock-show", "hint"));
        ui.end_row();

        ui.label(tr!("settings-ui-background"));
        ui.horizontal(|ui| {
            crate::chrome::checkbox(
                ui,
                &mut cfg.spectrum_gradient,
                tr!("settings-ui-background-gradient"),
            );
            ui.add_enabled_ui(cfg.spectrum_gradient, |ui| {
                ui.label(tr!("settings-ui-background-top"));
                ui.color_edit_button_srgb(&mut cfg.gradient_top);
                ui.label(tr!("settings-ui-background-bottom"));
                ui.color_edit_button_srgb(&mut cfg.gradient_bottom);
            });
        });
        ui.end_row();

        ui.label(tr!("settings-ui-spot-colours"))
            .on_hover_text(tr!("settings-ui-spot-colours", "hint"));
        ui.horizontal_wrapped(|ui| {
            for kind in SpotKind::ALL {
                ui.color_edit_button_srgb(&mut cfg.spot_colors[kind.index()])
                    .on_hover_text(tr!("settings-ui-spot-colour", kind = kind.label()));
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
                .button(tr!("settings-ui-reset"))
                .on_hover_text(tr!("settings-ui-spot-colours-reset"))
                .clicked()
            {
                for kind in SpotKind::ALL {
                    let (r, g, b) = kind.default_color();
                    cfg.spot_colors[kind.index()] = [r, g, b];
                }
            }
        });
        ui.end_row();

        ui.label(tr!("settings-ui-bandplan-colours"))
            .on_hover_text(tr!("settings-ui-bandplan-colours", "hint"));
        ui.horizontal_wrapped(|ui| {
            for kind in BandplanKind::ALL {
                ui.color_edit_button_srgb(&mut cfg.bandplan_colors[kind.index()])
                    .on_hover_text(tr!("settings-ui-bandplan-colour", kind = kind.tr()));
                let [r, g, b] = cfg.bandplan_colors[kind.index()];
                ui.label(
                    RichText::new(kind.tr())
                        .size(11.0)
                        .strong()
                        .color(crate::theme::data_ink((r, g, b))),
                );
                ui.add_space(6.0);
            }
            if ui
                .button(tr!("settings-ui-reset"))
                .on_hover_text(tr!("settings-ui-bandplan-colours-reset"))
                .clicked()
            {
                for kind in BandplanKind::ALL {
                    let (r, g, b) = kind.default_color();
                    cfg.bandplan_colors[kind.index()] = [r, g, b];
                }
            }
        });
        ui.end_row();

        ui.label(tr!("settings-ui-skimmer-font"));
        enum_combo(ui, "ui-skim-font", &mut cfg.skimmer_font_size, &FontSize::ALL, FontSize::tr);
        ui.end_row();

        ui.label(tr!("settings-ui-waterfall-font"));
        enum_combo(ui, "ui-wf-font", &mut cfg.waterfall_font_size, &FontSize::ALL, FontSize::tr);
        ui.end_row();

        ui.label(tr!("settings-ui-interface-font"))
            .on_hover_text(tr!("settings-ui-interface-font", "hint"));
        enum_combo(ui, "ui-menu-font", &mut cfg.menu_font_size, &FontSize::ALL, FontSize::tr);
        ui.end_row();

        ui.label(tr!("settings-ui-cities")).on_hover_text(tr!("settings-ui-cities", "hint"));
        crate::chrome::checkbox(ui, &mut cfg.map_cities, tr!("settings-ui-cities-show"));
        ui.end_row();
    });

    // Native only: the check is about *this binary* being out of date, and
    // the browser client has no binary of its own to be behind — its station
    // serves it fresh every time.
    #[cfg(not(target_arch = "wasm32"))]
    {
        ui.add_space(10.0);
        crate::chrome::checkbox(ui, &mut cfg.update_check, tr!("settings-ui-update-check"))
            .on_hover_text(tr!("settings-ui-update-check", "hint"));
    }

    let Some(cloud_march) = cloud_march else { return };
    ui.add_space(14.0);
    ui.label(
        RichText::new(tr!("settings-ui-3d-view")).size(14.0).strong().color(crate::theme::CYAN()),
    );
    ui.add_space(6.0);
    egui::Grid::new("ui-grid-3d").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
        ui.label(tr!("settings-ui-clouds"));
        let layered = tr!("settings-ui-clouds-layered");
        let volumetric = tr!("settings-ui-clouds-volumetric");
        ComboBox::from_id_salt("ui-cloud-march")
            .selected_text(if *cloud_march { &volumetric } else { &layered })
            .show_styled(ui, |ui| {
                ui.selectable_value(cloud_march, false, layered.as_str());
                ui.selectable_value(cloud_march, true, volumetric.as_str());
            });
        ui.end_row();
    });
    ui.add_space(8.0);
    ui.label(RichText::new(tr!("settings-ui-clouds-note")).weak());
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
    ui.label(RichText::new(tr!("speech-title")).size(14.0).strong().color(crate::theme::CYAN()));
    ui.add_space(6.0);
    crate::chrome::checkbox(ui, &mut cfg.enabled, tr!("speech-enabled"))
        .on_hover_text(tr!("speech-enabled", "hint"));

    ui.add_enabled_ui(cfg.enabled, |ui| {
        egui::Grid::new("speech-grid").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
            ui.label(tr!("speech-voice"));
            let shown =
                if cfg.voice.is_empty() { tr!("speech-voice-shipped") } else { cfg.voice.clone() };
            ComboBox::from_id_salt("speech-voice").width(300.0).selected_text(shown).show_styled(
                ui,
                |ui| {
                    if ui
                        .selectable_label(cfg.voice.is_empty(), tr!("speech-voice-shipped"))
                        .clicked()
                    {
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

            ui.label(tr!("speech-speed"));
            crate::chrome::slider(
                ui,
                egui::Slider::new(&mut cfg.rate, SpeechSettings::RATE_RANGE)
                    .step_by(0.1)
                    .suffix("×"),
            )
            .on_hover_text(tr!("speech-speed", "hint"));
            ui.end_row();

            ui.label(tr!("speech-volume"));
            crate::chrome::slider(ui, egui::Slider::new(&mut cfg.volume, 0.0..=1.0).step_by(0.05));
            ui.end_row();

            ui.label(tr!("speech-output"));
            // `device_combo` borrows the current selection while handing the
            // new one to the closure, so the two cannot both be `cfg.device`.
            let cur = cfg.device.clone();
            device_combo(ui, "speech-out", outputs, &cur, |n| cfg.device = n);
            ui.end_row();

            ui.label(tr!("speech-detail"));
            crate::app::settings::enum_combo(
                ui,
                "speech-verbosity",
                &mut cfg.verbosity,
                &Verbosity::ALL,
                Verbosity::tr,
            );
            ui.end_row();

            ui.label(tr!("speech-duck"));
            ui.horizontal(|ui| {
                crate::chrome::checkbox(ui, &mut cfg.duck_rx, tr!("speech-duck-while"));
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
            if ui.button(tr!("speech-test")).clicked() {
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
        // Pinned id: the header's text is translated, and egui would otherwise
        // remember whether it was open under the text.
        egui::CollapsingHeader::new(tr!("speech-what"))
            .id_salt("speech-what")
            .default_open(false)
            .show(ui, |ui| {
                egui::Grid::new("speech-cats").num_columns(2).spacing([16.0, 4.0]).show(ui, |ui| {
                    let c = &mut cfg.cat;
                    crate::chrome::checkbox(ui, &mut c.frequency, tr!("speech-cat-frequency"));
                    crate::chrome::checkbox(ui, &mut c.mode_band, tr!("speech-cat-mode-band"));
                    ui.end_row();
                    crate::chrome::checkbox(ui, &mut c.vfo_split, tr!("speech-cat-vfo-split"));
                    crate::chrome::checkbox(ui, &mut c.agc_gain, tr!("speech-cat-agc-gain"));
                    ui.end_row();
                    crate::chrome::checkbox(ui, &mut c.levels, tr!("speech-cat-levels"));
                    crate::chrome::checkbox(ui, &mut c.ptt, tr!("speech-cat-ptt"));
                    ui.end_row();
                    crate::chrome::checkbox(ui, &mut c.rit_xit, tr!("speech-cat-rit-xit"));
                    crate::chrome::checkbox(ui, &mut c.memory_scan, tr!("speech-cat-memory-scan"));
                    ui.end_row();
                    crate::chrome::checkbox(ui, &mut c.band_edge, tr!("speech-cat-band-edge"));
                    crate::chrome::checkbox(ui, &mut c.notices, tr!("speech-cat-notices"));
                    ui.end_row();
                    crate::chrome::checkbox(ui, &mut c.filters, tr!("speech-cat-filters"))
                        .on_hover_text(tr!("speech-cat-filters", "hint"));
                    ui.end_row();
                });

                ui.add_space(8.0);
                ui.label(RichText::new(tr!("speech-decoded")).strong());
                egui::Grid::new("speech-decodes").num_columns(2).spacing([16.0, 4.0]).show(
                    ui,
                    |ui| {
                        let d = &mut cfg.decodes;
                        crate::chrome::checkbox(ui, &mut d.ft8_to_me, tr!("speech-ft8-to-me"));
                        crate::chrome::checkbox(ui, &mut d.ft8_cq_for_me, tr!("speech-ft8-cq"))
                            .on_hover_text(tr!("speech-ft8-cq", "hint"));
                        ui.end_row();
                        crate::chrome::checkbox(ui, &mut d.js8, tr!("speech-js8"));
                        crate::chrome::checkbox(ui, &mut d.js8_allcall, tr!("speech-js8-allcall"));
                        ui.end_row();
                        crate::chrome::checkbox(ui, &mut d.fsq, tr!("speech-fsq"));
                        crate::chrome::checkbox(ui, &mut d.include_snr, tr!("speech-include-snr"))
                            .on_hover_text(tr!("speech-include-snr", "hint"));
                        ui.end_row();
                        crate::chrome::checkbox(ui, &mut d.ft8_qso, tr!("speech-ft8-qso"))
                            .on_hover_text(tr!("speech-ft8-qso", "hint"));
                        ui.end_row();
                    },
                );

                ui.add_space(8.0);
                ui.label(RichText::new(tr!("speech-reading-text")).strong());
                ui.horizontal(|ui| {
                    crate::chrome::checkbox(ui, &mut cfg.text.cw, tr!("speech-text-cw"));
                    crate::chrome::checkbox(
                        ui,
                        &mut cfg.text.rtty_psk,
                        tr!("speech-text-rtty-psk"),
                    );
                });
                crate::chrome::checkbox(
                    ui,
                    &mut cfg.text.cw_only_when_locked,
                    tr!("speech-text-cw-locked"),
                )
                .on_hover_text(tr!("speech-text-cw-locked", "hint"));
                ui.label(RichText::new(tr!("speech-text-note")).weak());

                ui.add_space(8.0);
                ui.label(RichText::new(tr!("speech-tuning-up")).strong());
                crate::chrome::checkbox(
                    ui,
                    &mut cfg.tune.swr_while_tuning,
                    tr!("speech-swr-while-tuning"),
                );
                ui.add_enabled_ui(cfg.tune.swr_while_tuning, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(tr!("speech-swr-every"));
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
                    tr!("speech-swr-summary"),
                );
                crate::chrome::checkbox(ui, &mut cfg.tune.alarm_always, tr!("speech-swr-alarm"));

                ui.add_space(8.0);
                ui.label(RichText::new(tr!("speech-how-read")).strong());
                egui::Grid::new("speech-style").num_columns(2).spacing([12.0, 6.0]).show(
                    ui,
                    |ui| {
                        ui.label(tr!("speech-frequencies"));
                        crate::app::settings::enum_combo(
                            ui,
                            "speech-freq-style",
                            &mut cfg.freq_style,
                            &FreqStyle::ALL,
                            FreqStyle::tr,
                        );
                        ui.end_row();
                        ui.label(tr!("speech-callsigns"));
                        crate::app::settings::enum_combo(
                            ui,
                            "speech-call-style",
                            &mut cfg.callsign_style,
                            &CallsignStyle::ALL,
                            CallsignStyle::tr,
                        );
                        ui.end_row();
                    },
                );

                ui.add_space(6.0);
                crate::chrome::checkbox(ui, &mut cfg.duck_on_ptt, tr!("speech-quiet-on-tx"))
                    .on_hover_text(tr!("speech-quiet-on-tx", "hint"));
            });
    });

    ui.add_space(8.0);
    ui.label(RichText::new(tr!("speech-device-note")).weak());
}

/// The language picker: the system's language first, then every language a
/// translation is carried for, each under its own name.
fn language_combo(ui: &mut egui::Ui, cur: &mut sdroxide_types::UiLanguage) {
    use sdroxide_types::UiLanguage;
    let system = tr!("settings-ui-language-system", language = crate::i18n::system_language_name());
    let langs = crate::i18n::languages();
    let shown = match cur.tag() {
        None => system.clone(),
        Some(tag) => {
            langs.iter().find(|l| l.tag == tag).map_or(tag.to_string(), |l| l.name.clone())
        }
    };
    ComboBox::from_id_salt("ui-language").selected_text(shown).show_styled(ui, |ui| {
        ui.selectable_value(cur, UiLanguage::SYSTEM, system);
        for lang in langs {
            if let Some(pick) = UiLanguage::from_tag(&lang.tag) {
                ui.selectable_value(cur, pick, &lang.name);
            }
        }
    });
}

/// A detachable module's name in the settings rows, in the interface language.
#[cfg(not(target_arch = "wasm32"))]
fn module_label(m: sdroxide_types::DetachableModule) -> String {
    use sdroxide_types::DetachableModule as M;
    match m {
        M::Panadapter => tr!("settings-ui-module-panadapter"),
        M::Panel => tr!("multi-module-panel"),
        M::Controls => tr!("settings-ui-module-controls"),
        M::AuxPanadapter => tr!("settings-ui-module-aux"),
        M::BandMenu => tr!("settings-ui-module-band-menu"),
    }
}
