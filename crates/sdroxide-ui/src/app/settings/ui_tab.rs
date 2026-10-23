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

/// What the "Screen follows your profile" row asked the app to do. The row is
/// drawn by a free function, so the action travels back to the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileAction {
    Save,
    Revert,
}

pub(in crate::app) fn settings_ui_tab(
    ui: &mut egui::Ui,
    cfg: &mut sdroxide_types::UiSettings,
    radio: Option<&mut sdroxide_types::RadioConfig>,
    cloud_march: Option<&mut bool>,
    profile_action: &std::cell::Cell<Option<ProfileAction>>,
    profile_from: Option<&Option<String>>,
    profile_status: Option<&String>,
    report: &mut sdroxide_types::SwlReportPrefs,
    report_pick: &std::sync::Arc<std::sync::Mutex<Option<String>>>,
) {
    use sdroxide_types::{ChromeStyle, FontSize, LayoutMode, UiSettings, UiTheme};
    ui.label(RichText::new("Display").size(14.0).strong().color(crate::theme::CYAN()));
    {
        // **How many broadcast stations a listener's screen may show** (#80).
        // The window is the panadapter's span, which is right for a label on
        // that waterfall and wrong for a whole-band map: a listener at 6190 kHz
        // with 20 kHz of span was told there were four stations in the world.
        use sdroxide_types::BroadcastScope as B;
        ui.label("Broadcast stations");
        ComboBox::from_id_salt("ui-broadcast-scope")
            .selected_text(cfg.broadcast_scope.label())
            .show_styled(ui, |ui| {
                for scope in [B::InView, B::WholeBand] {
                    ui.selectable_value(&mut cfg.broadcast_scope, scope, scope.label());
                }
            })
            .response
            .on_hover_text(
                "Which broadcasting stations the LISTEN window and the waterfall label may \
                 show.\n\n**On this span** is what every screen did before this was a \
                 choice: half a span either side of the dial, which is what a label on that \
                 waterfall should cover. **The whole band** shows everything the schedule \
                 holds with a position on it, which is what a listener browsing a band is \
                 looking for — at the cost of a busier map.",
            );
    }
    ui.add_space(6.0);
    // Said here because this is where they used to be, and where an operator
    // who remembers them will come looking. They moved to the panadapter's own
    // button, beside the picture they change — see
    // `SdroxideApp::panadapter_controls`.
    super::note_at(
        ui,
        "Panadapter detail, the spectrum's reaction and the waterfall's scroll speed are \
             set from the SPEC button on the display strip — the DISP menu on a narrow window.",
        Some(11.0),
    );
    ui.add_space(6.0);
    egui::Grid::new("ui-grid").num_columns(2).spacing([12.0, 8.0]).min_row_height(ui.spacing().interact_size.y).show(ui, |ui| {
        ui.label("Layout");
        enum_combo(ui, "ui-layout", &mut cfg.layout, &LayoutMode::ALL, LayoutMode::label);
        ui.end_row();

        // The two arrangements of the desktop. It is beside Layout because it
        // is the same kind of choice — where things live on this screen — and
        // not under it, because it is not a size: the column is offered at
        // every desktop size and left out where there is no room for it.
        ui.label("Classic UI").on_hover_text(
            "Put every control in one column down the left with the groups folding away, \
             the way SDR++ and SDR++Brown lay theirs out, and give the picture everything \
             else. Same controls, same settings, arranged the way most short-wave \
             listeners already have in their hands.\n\n\
             It needs a window wide enough to keep the waterfall its share: below that the \
             column is left out and the strip is drawn as usual, and the panel is dragged \
             wider or narrower from its right edge. Off, this program's own strip is used \
             exactly as before.",
        );
        crate::chrome::checkbox(ui, &mut cfg.classic_ui, "controls in a left column");
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

        ui.label("Updates");
        crate::chrome::checkbox(
            ui,
            &mut cfg.check_for_updates,
            "check for a new release at start-up",
        )
        .on_hover_text(
            "Asks the fork's GitHub Releases for the latest release once at start-up, and \
             shows a banner if it is newer than this build. Only full releases count — the \
             nightly is a pre-release — and off means no outbound call at all.",
        );
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
                ui.label(format!("{} window", module.label()));
                ComboBox::from_id_salt(("ui-undocked", module))
                    .selected_text(if on {
                        "Undocked — its own window"
                    } else {
                        "Docked in this window"
                    })
                    .show_styled(ui, |ui| {
                        ui.selectable_value(&mut on, false, "Docked in this window");
                        ui.selectable_value(&mut on, true, "Undocked — its own window");
                    })
                    .response
                    .on_hover_text(format!(
                        "Undocked, this module moves into its own window and the main \
                         window gives the space back to what is left — put the waterfall on \
                         a second monitor, or the controls on a third. Closing its window \
                         docks it again.\n\nOn Wayland the size comes back but the \
                         compositor decides where a window goes: float it and pin it to a \
                         monitor with a window rule matching app-id \"{}\".",
                        module.app_id(),
                    ));
                cfg.set_detached(module, on);
                ui.end_row();
            }
            // Both directions at once, for arranging a whole screen in one
            // press: every module out, or every module home. Shown only while
            // they would do something, so neither is a button that changes
            // nothing.
            let any_out = DetachableModule::ALL.iter().any(|m| cfg.is_detached(*m));
            let any_in = DetachableModule::ALL.iter().any(|m| !cfg.is_detached(*m));
            if any_out || any_in {
                ui.label("");
                ui.horizontal(|ui| {
                    if any_out
                        && crate::chrome::chip(ui, false, "DOCK ALL WINDOWS")
                            .on_hover_text(
                                "Bring every undocked window back into this one.",
                            )
                            .clicked()
                    {
                        for m in DetachableModule::ALL {
                            cfg.set_detached(m, false);
                        }
                    }
                    if any_in
                        && crate::chrome::chip(ui, false, "UNDOCK ALL WINDOWS")
                            .on_hover_text(
                                "Put every module in a window of its own — the main window is \
                                 left as the spectrum and the decoders. A module with nothing to \
                                 show, like the operating panel in a voice mode, opens no window.",
                            )
                            .clicked()
                    {
                        for m in DetachableModule::ALL {
                            cfg.set_detached(m, true);
                        }
                    }
                });
                ui.end_row();
            }
        }

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

        ui.label("Tuning buttons");
        ui.horizontal(|ui| {
            crate::chrome::checkbox(ui, &mut cfg.tune_step_buttons, "Show on phone and tablet")
                .on_hover_text(
                    "A row of step-down / step / step-up buttons under the control strip on a \
                     touched screen. There is no wheel to scroll a digit with and no dial, so \
                     without them the only way to move a known step is to type the whole \
                     frequency in. Never drawn on a desktop.",
                );
            super::note(ui, &format!("Step: {}", cfg.tune_step_label()));
        });
        ui.end_row();

        // Only meaningful with the row above, so it sits under it.
        if cfg.tune_step_buttons {
            ui.label("");
            crate::chrome::checkbox(
                ui,
                &mut cfg.tune_step_round_first,
                "First press snaps to the step",
            )
            .on_hover_text(
                "A dial left between two multiples of the step goes to the next one in the \
                 direction pressed — 7 074 300 goes up to 7 075 000 at a 1 kHz step, down to \
                 7 074 000. After that the buttons move by the step as usual. Off, every press \
                 moves by exactly the step.",
            );
            ui.end_row();
        }

        ui.label("Waterfall smoothing");
        crate::chrome::checkbox(ui, &mut cfg.waterfall_smooth, "Interpolate").on_hover_text(
            "Blend each screen pixel with the bins and rows around it, so a signal looks \
                 continuous where the display is wider than the transform. Turn it off for a \
                 rectangular waterfall — one block per bin, one per row — which is what \
                 reading a signal's shape off the picture needs: an interpolated signal cannot \
                 be told apart from a genuinely wider one. A bigger FFT (the FFT chip) is the \
                 other half of that.",
        );
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

        ui.label("Spectrum line").on_hover_text(
            "The colour under the spectrum line, and how it is shaded: Fixed is one \
             colour throughout, Gradient is full at the bottom and fades out towards \
             the top, Mixed is full in the lower half and fades across the upper half. \
             The line itself stays cyan. The FILL chip in the SPEC popup switches the \
             fill, and its line row picks the shading.",
        );
        ui.horizontal(|ui| {
            crate::chrome::checkbox(ui, &mut cfg.spectrum_fill, "Fill");
            ui.add_enabled_ui(cfg.spectrum_fill, |ui| {
                ui.label("colour");
                ui.color_edit_button_srgb(&mut cfg.spectrum_fill_color);
                ComboBox::from_id_salt("ui-spectrum-fill-style")
                    .selected_text(cfg.spectrum_fill_style.label())
                    .show_styled(ui, |ui| {
                        for s in sdroxide_types::SpectrumFillStyle::ALL {
                            ui.selectable_value(&mut cfg.spectrum_fill_style, s, s.label());
                        }
                    });
            });
        });
        ui.end_row();

        first_line_label(ui, "Spot label colours").on_hover_text(
            "The tint each spot source wears: the boxes along the bottom of the \
             waterfall, the badges in the SPOTS list and the dots on the world map.",
        );
        // Wrapped inside a column, not straight in the grid cell: laid out
        // directly in the cell, a row that wraps onto a second line is not
        // given that line's full height, and its Reset button ran into the
        // picker in the row below.
        ui.vertical(|ui| {
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
        });
        ui.end_row();

        first_line_label(ui, "Band plan colours").on_hover_text(
            "The shade each class of allocation is painted in on the band-plan \
             strip along the bottom of the waterfall. The blocks are drawn \
             semi-transparent over the waterfall, so they land darker there \
             than in the swatch here.",
        );
        ui.vertical(|ui| {
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

        ui.label("Simple UI").on_hover_text(
            "Hide the advanced extras from the top strip: the 3D view, the \
             skimmers, the spectrum/waterfall layer switches, award tracking, \
             satellites, ISM decoding and radio email. The controls a CB \
             operator or a short-wave listener reaches for — tuning, mode, \
             volume, squelch, bandwidth, the waterfall, memories, scanning — \
             all stay.\n\n\
             Nothing is turned off, only hidden: switch this back on to bring \
             the chips back.",
        );
        crate::chrome::checkbox(ui, &mut cfg.simple_ui, "hide advanced chips");
        ui.end_row();

        ui.label("Wake lock").on_hover_text(
            "Keep the screen on while this page is in front, so a phone or \
             tablet left on the waterfall does not dim and lock itself after a \
             minute without a touch. Reception, audio and decoding do not need \
             it — they carry on either way — it is so the screen stays \
             readable.\n\n\
             Pairs with the padlock on the phone and tablet strips: lock the \
             screen against stray fingers, and keep it lit.\n\n\
             The browser takes the lock back whenever the page is hidden — \
             another tab, or the phone's own power button — and it is asked for \
             again as soon as the page is back. It needs a browser that has it \
             and a secure page (https, or localhost); the line beside the box \
             says when it is not available or was refused. It costs battery, so \
             it is off until you ask. The installed program on a computer \
             follows the system's own power settings instead.",
        );
        ui.horizontal(|ui| {
            crate::chrome::checkbox(ui, &mut cfg.keep_screen_on, "keep the screen on");
            let status = crate::wake_lock::status(cfg.keep_screen_on);
            if !status.label().is_empty() {
                let ink = if status.is_problem() {
                    crate::theme::YELLOW()
                } else {
                    ui.visuals().weak_text_color()
                };
                ui.label(egui::RichText::new(status.label()).small().color(ink));
            }
        });
        ui.end_row();

        ui.label("Sound welcome").on_hover_text(
            "When the web page opens — a new session, or a reload with F5 — \
             the browser plays no sound until you touch the page. That is the \
             browser's rule, and without a word on screen a fresh session \
             looks as if it does not work.\n\n\
             With this on, a message in the middle of the screen says so: \
             🔇 Tap, click or press a key anywhere to start the sound. The \
             first click, tap or key — anywhere, with a mouse, a touchpad, a \
             finger or the keyboard — closes it and starts the sound, and does \
             nothing else: it does not tune, press a button or run a \
             shortcut. It is shown only when the page opens, never again \
             during the session, whatever happens to the sound later.\n\n\
             Turn it off once you know what to do: the sound still waits for \
             your first touch, and that touch then works as usual. Kept with \
             your profile on the server. On by default.",
        );
        ui.horizontal(|ui| {
            crate::chrome::checkbox(
                ui,
                &mut cfg.sound_welcome,
                egui::RichText::new("welcome message for the sound").color(crate::theme::YELLOW()),
            );
            if !crate::sound_welcome::available() {
                ui.label(
                    egui::RichText::new(
                        "browser only — the installed program plays sound without a touch",
                    )
                    .small()
                    .color(crate::theme::YELLOW()),
                );
            }
        });
        ui.end_row();


        ui.end_row();

        ui.label("Screen follows your profile").on_hover_text(
            "When you are a remote client of `sdroxide --server`, the *look* of \
             this screen — theme, layout, waterfall and spectrum, fonts, Simple \
             UI, the map layers, the panadapter's own view — is kept with the \
             profile you signed in as, so it comes back on any machine instead \
             of starting from defaults.\n\n\
             **Saved on the server, and saved as you go while you are signed \
             in** — there is nothing to press. That is deliberate: browser \
             storage cannot be relied on, because `persist()` behaves \
             differently in every browser, and a session kept there cannot be \
             reset by anyone who is not at that device. A profile on the server \
             is a file the station's operator can put back.\n\n\
             On a server with **no password** every client is the same profile, \
             so nothing is written unless you press the button below — one \
             operator's theme must not become the next one's. Nothing that \
             belongs to the machine is ever stored, and a local radio is \
             unaffected.",
        );
        ui.horizontal(|ui| {
            if ui.button("Save to profile").clicked() {
                profile_action.set(Some(ProfileAction::Save));
            }
            if ui.button("Back to profile").clicked() {
                profile_action.set(Some(ProfileAction::Revert));
            }
        });
        // Say where the look in force came from, and what the last save did. A
        // save that says nothing is a save nobody can trust.
        ui.label(
            RichText::new(match profile_from {
                Some(Some(name)) => format!("in use: the profile {name}"),
                Some(None) => "in use: this station's default".to_string(),
                None => "not in use yet — nothing stored for this login".to_string(),
            })
            .weak()
            .small(),
        );
        if let Some(status) = profile_status {
            ui.label(RichText::new(status.clone()).small().color(crate::theme::YELLOW()));
        }
        ui.end_row();

        // The opt-in stays, deliberately. The shared keyboard is the one thing
        // in a profile that can hurt another operator, so it is behind an
        // acknowledgement and off by default — and it is **not** to be removed
        // as unnecessary: it is asked for. Someone running their own server and
        // reaching it from several of their own devices wants their keys to
        // follow them. The answer to a shared station is the acknowledgement
        // and the default, not taking the feature away.
        ui.label("Carry control bindings").on_hover_text(
            "Also keep your keyboard and mouse bindings with the profile on the \
             server, so they follow the login between browsers and devices — \
             the same idea as the screen above, for the keys.\n\n\
             **NOT RECOMMENDED, and off by default.** On a station several \
             people share, the keyboard is shared with it: a profile that \
             carries bindings can rebind another operator's PTT, Space or \
             tuning keys the moment it signs in, and turning this on may have \
             other effects that are not obvious now. Turn it on only where the \
             server is yours alone and you reach it from several devices of \
             your own.\n\n\
             Fork-only: this is not an upstream feature. The screen settings \
             above always travel; this is the only opt-in here.",
        );
        {
            // The opt-in is not a plain checkbox: turning it ON must be
            // acknowledged, so the click opens a modal and the flag stays off
            // until the operator confirms. Turning it OFF is immediate.
            let confirm_id = egui::Id::new("client-share-bindings-confirm");
            ui.horizontal(|ui| {
                ui.scope(|ui| {
                    let was = cfg.client_share_bindings;
                    let mut shown = was;
                    let resp = crate::chrome::checkbox(
                        ui,
                        &mut shown,
                        if was {
                            "on — NOT RECOMMENDED"
                        } else {
                            "carry them (not recommended)"
                        },
                    );
                    if resp.clicked() {
                        if shown && !was {
                            ui.data_mut(|d| d.insert_temp(confirm_id, true));
                        } else {
                            cfg.client_share_bindings = shown;
                            // Turning it off is an answer too — "keep mine" —
                            // so the server stops offering the profile's keys.
                            cfg.client_bindings_declined = !shown;
                        }
                    }
                    resp
                });
            });
            let confirming: bool = ui.data(|d| d.get_temp(confirm_id)).unwrap_or(false);
            if confirming {
                let mut enable = false;
                let mut cancel = false;
                egui::Modal::new(egui::Id::new("client-share-bindings-modal")).show(
                    ui.ctx(),
                    |ui| {
                        ui.set_max_width(470.0);
                        ui.heading(
                            RichText::new("Carry control bindings on the server?")
                                .color(crate::theme::ALERT()),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            "This stores your keyboard and mouse bindings with your profile on \
                             the server, so they follow your login between browsers and devices.\n\n\
                             On a station other people use, the keyboard is shared with it. A \
                             profile that carries bindings can rebind another operator's PTT, \
                             Space or tuning keys simply by signing in — and enabling this may \
                             have other effects that are not obvious now.\n\n\
                             That is why it is not recommended. Turn it on only if the server \
                             is yours alone.",
                        );
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            if ui
                                .button(
                                    RichText::new("I understand — carry them anyway")
                                        .color(crate::theme::ALERT()),
                                )
                                .clicked()
                            {
                                enable = true;
                            }
                            if ui.button("Cancel").clicked() {
                                cancel = true;
                            }
                        });
                    },
                );
                if enable {
                    cfg.client_share_bindings = true;
                    cfg.client_bindings_declined = false;
                }
                if enable || cancel {
                    ui.data_mut(|d| d.remove_temp::<bool>(confirm_id));
                }
            }
        }
        ui.end_row();

        ui.label("Start in SWL mode").on_hover_text(
            "Open every session with SWL mode already on, so a listener's \
             screen is what the program comes up as — for every radio, this \
             being the screen's preference rather than a radio's.\n\n\
             The per-radio switch is Settings → Radio → Transmit controls, so \
             a station with one listening radio and one transceiver sets the \
             listening one there and leaves this off. Using that switch turns \
             this seed off for the session. `--swl` always wins.",
        );
        crate::chrome::checkbox(ui, &mut cfg.start_swl, "start with SWL mode on");
        ui.end_row();

        // Issue #146: the LISTEN tab in the band/mode menu carrying the listening
        // screen. **It hides; it never grants.** Off by default, and it asks once
        // — the reporter's own words were "I accept this operating mode at my own
        // risk", and the person who wrote them is the one who has to read what it
        // does not do.
        ui.label("LISTEN tab hides transmit").on_hover_text(
            "Picking **LISTEN** in the band/mode menu hides every transmit control on \
             that tab, and picking **OPERATE** puts back whatever this radio's own \
             setting says.\n\n\
             It cannot turn a transmitter on. A radio configured receive-only stays \
             receive-only, the band lockout and the out-of-band warning are untouched, \
             and the hiding ends with the session — so a tab click can never leave a \
             station unable to transmit tomorrow.\n\n\
             Off by default because it changes what the tab means; it asks once.",
        );
        {
            let confirm_id = egui::Id::new("listen-tab-hides-tx-confirm");
            let mut shown = cfg.listen_tab_hides_tx;
            let resp = crate::chrome::checkbox(ui, &mut shown, "hide TX on the LISTEN tab");
            let was = cfg.listen_tab_hides_tx;
            if resp.changed() {
                if shown && !was {
                    ui.data_mut(|d| d.insert_temp(confirm_id, true));
                } else {
                    cfg.listen_tab_hides_tx = shown;
                }
            }
            let confirming: bool = ui.data(|d| d.get_temp(confirm_id)).unwrap_or(false);
            if confirming {
                let mut enable = false;
                let mut cancel = false;
                egui::Modal::new(egui::Id::new("listen-tab-hides-tx-modal")).show(
                    ui.ctx(),
                    |ui| {
                        ui.set_max_width(470.0);
                        ui.heading(
                            RichText::new("The LISTEN tab will hide the transmit controls")
                                .color(crate::theme::ALERT()),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            "On the LISTEN tab the transmit controls disappear; on OPERATE \
                             they come back exactly as this radio's own setting left them.\n\n\
                             It does not enable anything. It is a view of the same station, \
                             so the band lockout, the out-of-band warning and the transmit \
                             rails are untouched, and the hiding ends when the program does.\n\n\
                             Worth knowing: a station left on LISTEN still receives on \
                             whatever the tab has tuned, and a control that is hidden is \
                             not a control that is off — transmission is off because the \
                             settings say so, not because the button is out of sight.",
                        );
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            if ui
                                .button(
                                    RichText::new("I understand — hide them on LISTEN")
                                        .color(crate::theme::ALERT()),
                                )
                                .clicked()
                            {
                                enable = true;
                            }
                            if ui.button("Cancel").clicked() {
                                cancel = true;
                            }
                        });
                    },
                );
                if enable {
                    cfg.listen_tab_hides_tx = true;
                }
                if enable || cancel {
                    ui.data_mut(|d| d.remove_temp::<bool>(confirm_id));
                }
            }
        }
        ui.end_row();

        // The switch itself, not only the seed: the same per-radio
        // `RadioConfig::hide_tx` that Settings → Radio → Transmit controls
        // writes, reachable here because a listener looks for it in the UI
        // menu. Off `radio` (a remote or browser client with no config yet)
        // there is nothing to set, so the row is left out rather than shown
        // dead.
        if let Some(radio) = radio {
            ui.label("SWL mode").on_hover_text(
                "Hide every transmit control for this radio — the PTT, CALL CQ, TX level, \
                 SEND, BEACON, all of it — and swap the strip's ham extras (spots, awards) \
                 for the listener's (SCHEDULE and SWL LOG, the reception log). The same switch as Settings → Radio \
                 → Transmit controls, and per radio: a listening dongle can sit in this mode \
                 while the transceiver beside it keeps its transmitter.\n\n\
                 The hardware can still transmit; this only hides the controls. `--swl` \
                 forces it on for every radio for the run.\n\nTakes effect on Apply.",
            );
            crate::chrome::checkbox(ui, &mut radio.hide_tx, "hide all transmit controls");
            ui.end_row();
        }

        // ── Reception reports ──
        // What a mailed reception report carries besides the reception itself.
        // Both optional and both genuinely so: a plain-text report is a
        // perfectly good report, so leaving these empty breaks nothing.
        //
        // Native-only, because the picture is a **path on this machine's disk**
        // and a browser tab has no disk to name. Storing a string there could
        // never resolve to a file, so the row is left out rather than shown
        // dead — the same rule the rest of this crate follows for paths.
        if !cfg!(target_arch = "wasm32") {
            ui.label("Report picture").on_hover_text(
                "The picture a reception report carries when you mail it to a broadcaster — \
                 your own card, a photograph, whatever you want the station to see. One \
                 image for the station, used for every report.\n\nLeave it unset and the \
                 report goes as plain text, which is a perfectly good report.",
            );
            // Whatever the picker thread found, taken once and cleared so it is
            // applied exactly once.
            if let Ok(mut inbox) = report_pick.lock()
                && let Some(picked) = inbox.take()
            {
                report.picture = picked;
            }
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut report.picture)
                        .desired_width(240.0)
                        .hint_text("none — the report goes as text"),
                );
                if crate::chrome::chip(ui, false, "Choose…")
                    .on_hover_text("Pick the picture on this computer")
                    .clicked()
                {
                    pick_report_picture(report_pick);
                }
                if !report.picture.is_empty()
                    && crate::chrome::chip(ui, false, "Clear").clicked()
                {
                    report.picture.clear();
                }
            });
            ui.label("Report note").on_hover_text(
                "Your standing note, pre-filled into every reception report — who you are, \
                 what you listen on, anything a station asks once rather than per letter. \
                 Plain text in a plain file: not a secret.",
            );
            crate::chrome::field(
                ui,
                egui::TextEdit::multiline(&mut report.message)
                    .desired_width(380.0)
                    .desired_rows(3)
                    .hint_text("optional — e.g. \"Details on request\""),
            );
        }

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

    let Some(cloud_march) = cloud_march else { return };
    ui.add_space(14.0);
    ui.label(RichText::new("3D view").size(14.0).strong().color(crate::theme::CYAN()));
    ui.add_space(6.0);
    egui::Grid::new("ui-grid-3d")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .min_row_height(ui.spacing().interact_size.y)
        .show(ui, |ui| {
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
    super::note(
        ui,
        "How the CLOUDS layer in the 3D view draws the weather. Layered stacks \
             slices through the troposphere and is the cheap option. Volumetric walks \
             a ray through it instead, so the Sun casts the cloud tops onto the deck \
             below and lightning glows out through the storm making it rather than \
             only brightening its outside — at several times the cost per pixel.",
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
        egui::Grid::new("speech-grid")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .min_row_height(ui.spacing().interact_size.y)
            .show(ui, |ui| {
                ui.label("Voice");
                let shown = if cfg.voice.is_empty() {
                    "Shipped voice".to_string()
                } else {
                    cfg.voice.clone()
                };
                ComboBox::from_id_salt("speech-voice")
                    .width(300.0)
                    .selected_text(shown)
                    .show_styled(ui, |ui| {
                        if ui.selectable_label(cfg.voice.is_empty(), "Shipped voice").clicked() {
                            cfg.voice.clear();
                        }
                        for v in voices {
                            if ui.selectable_label(&cfg.voice == v, v).clicked() {
                                cfg.voice = v.clone();
                            }
                        }
                    });
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
                crate::chrome::slider(
                    ui,
                    egui::Slider::new(&mut cfg.volume, 0.0..=1.0).step_by(0.05),
                );
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
            egui::Grid::new("speech-cats")
                .num_columns(2)
                .spacing([16.0, 4.0])
                .min_row_height(ui.spacing().interact_size.y)
                .show(ui, |ui| {
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
                    crate::chrome::checkbox(
                        ui,
                        &mut c.filters,
                        "Filters, squelch and noise reduction",
                    )
                    .on_hover_text("Off by default: these move constantly while chasing a signal");
                    ui.end_row();
                });

            ui.add_space(8.0);
            ui.label(RichText::new("Decoded messages").strong());
            ui.end_row();
            egui::Grid::new("speech-decodes")
                .num_columns(2)
                .spacing([16.0, 4.0])
                .min_row_height(ui.spacing().interact_size.y)
                .show(ui, |ui| {
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
                        .on_hover_text(
                            "What the sequencer is about to send, and the contact ending",
                        );
                    ui.end_row();
                });

            ui.add_space(8.0);
            ui.label(RichText::new("Reading decoded text aloud").strong());
            ui.end_row();
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
            super::note(
                ui,
                "Both are off by default. A decoder produces text faster than speech reads \
                     it, so anything that falls too far behind the live audio is dropped rather \
                     than queued.",
            );

            ui.add_space(8.0);
            ui.label(RichText::new("Tuning up").strong());
            ui.end_row();
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
            ui.end_row();
            egui::Grid::new("speech-style")
                .num_columns(2)
                .spacing([12.0, 6.0])
                .min_row_height(ui.spacing().interact_size.y)
                .show(ui, |ui| {
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
    super::note(
        ui,
        "Announcements play on their own sound device, so they are never recorded and never \
             sent to anyone listening remotely. Keys for speaking the status, repeating the last \
             announcement and stopping mid-sentence are on the Controls tab.",
    );
}

/// Open a file dialog for the report picture, off the UI thread.
///
/// `rfd`'s dialog is blocking, so doing it here would freeze the frame loop for
/// as long as the operator takes to choose. The path comes back through the
/// inbox rather than a channel, because there is exactly one string and the
/// window that asked for it is the one that reads it.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn pick_report_picture(
    inbox: &std::sync::Arc<std::sync::Mutex<Option<String>>>,
) {
    let inbox = inbox.clone();
    std::thread::Builder::new()
        .name("sdroxide-report-pick".into())
        .spawn(move || {
            if let Some(path) =
                rfd::FileDialog::new().add_filter("Image", &["png", "jpg", "jpeg"]).pick_file()
            {
                if let Ok(mut g) = inbox.lock() {
                    *g = Some(path.to_string_lossy().into_owned());
                }
            }
        })
        .ok();
}

/// The browser cannot name a file on this computer, so there is nothing to pick
/// — see where the row is drawn.
#[cfg(target_arch = "wasm32")]
pub(in crate::app) fn pick_report_picture(
    _inbox: &std::sync::Arc<std::sync::Mutex<Option<String>>>,
) {
}

/// The **Workspaces** section: SDRuno's ten named arrangements. Save the current
/// one, or put a saved one back.
///
/// A workspace is the window arrangement and nothing else — which modules are in
/// their own OS windows and where those windows are. Save captures the
/// arrangement as it stands; a name is a chip that puts it back. Editing the
/// list here rather than behind a shortcut keeps it with the undocked rows it
/// acts on: the same subject, one screen.
///
/// Native only, like the undocked rows above it — the browser has one window and
/// nothing to arrange.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn workspaces_section(
    ui: &mut egui::Ui,
    cfg: &mut sdroxide_types::UiSettings,
    workspaces: &mut Vec<sdroxide_types::Workspace>,
    name: &mut String,
) {
    use sdroxide_types::{WORKSPACE_MAX, Workspace};
    ui.label(RichText::new("Workspaces").size(14.0).strong().color(crate::theme::CYAN()));
    ui.add_space(2.0);
    super::note_at(
        ui,
        "A named window arrangement — which modules are in their own windows and where \
             those windows are. Save the one you have, or put a saved one back. Up to ten.",
        Some(11.0),
    );
    ui.add_space(6.0);

    if workspaces.is_empty() {
        super::note_at(ui, "No workspaces saved yet.", Some(11.0));
    }
    let mut apply: Option<usize> = None;
    let mut delete: Option<usize> = None;
    for (i, ws) in workspaces.iter().enumerate() {
        ui.horizontal(|ui| {
            let out = ws.slots.iter().filter(|s| s.detached).count();
            let chip =
                crate::chrome::chip(ui, false, ws.name.as_str()).on_hover_text(if out == 0 {
                    "Put this arrangement back — every module in the main window".to_string()
                } else {
                    format!("Put this arrangement back — {out} module(s) in their own window(s)")
                });
            if chip.clicked() {
                apply = Some(i);
            }
            if crate::chrome::chip(ui, false, "×").on_hover_text("Forget this workspace").clicked()
            {
                delete = Some(i);
            }
        });
    }

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        crate::chrome::field_sized(
            ui,
            egui::vec2(220.0, 0.0),
            egui::TextEdit::singleline(name).hint_text("name this arrangement"),
        );
        let named = !name.trim().is_empty();
        let full = workspaces.len() >= WORKSPACE_MAX;
        let can_save = named && !full;
        let save = crate::chrome::chip_enabled(ui, can_save, false, "SAVE CURRENT").on_hover_text(
            if full {
                format!("Ten workspaces is the limit SDRuno keeps; delete one first")
            } else if named {
                "Save the arrangement you have now under this name".to_string()
            } else {
                "Type a name first".to_string()
            },
        );
        if save.clicked() {
            let n = name.trim().to_string();
            // Saving over an existing name replaces it rather than making a
            // duplicate row the operator then has to tell apart.
            if let Some(slot) = workspaces.iter_mut().find(|w| w.name == n) {
                *slot = Workspace::capture(n, cfg);
            } else {
                workspaces.push(Workspace::capture(n, cfg));
            }
            name.clear();
        }
    });

    if let Some(i) = delete {
        workspaces.remove(i);
    }
    if let Some(i) = apply {
        workspaces[i].apply(cfg);
    }
}

/// A grid label that sits level with the first line of a wrapped row beside it.
///
/// A grid centres a label on its whole row, so beside a row of swatches that
/// wraps onto a second line it lands between the two lines. Pinned to the top
/// of the cell at one line's height, it reads with the swatches it names.
fn first_line_label(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let line = ui.spacing().interact_size.y;
    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(0.0, line),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| ui.label(text),
        )
        .inner
    })
    .inner
}
