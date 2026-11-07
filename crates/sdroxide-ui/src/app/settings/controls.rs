//! The Controls tab: keyboard, mouse and MIDI bindings.
//!
//! A rebind has no APPLY step — it takes effect on the next frame and is
//! written straight out, because a binding the operator cannot see saved is
//! one they will make again after the next restart. MIDI learn works the same
//! way: arm a row, move the control, and the row captures whatever arrives.

use crate::i18n::{Tr, tr};
use eframe::egui::{self, Color32, ComboBox, RichText};
use sdroxide_types::MemoryChannel;

use crate::app::settings::SettingsIo;
use crate::chrome::StyledCombo;

/// The built-in TCI *server*: this app acting as a TCI rig for third-party
/// clients (WSJT-X's TCI rig type, JTDX, MSHV, skimmers). Distinct from the TCI
/// *client* section on the Radio tab, which connects sdroxide to another rig.

/// A dropdown over every bindable [`Action`], grouped by section. `memories`
/// contributes one recall entry per stored channel, since those are the only
/// actions whose parameter comes from the operator's own data.
fn action_combo(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    action: &mut sdroxide_types::Action,
    memories: &[MemoryChannel],
) -> bool {
    use sdroxide_types::Action;
    let mut changed = false;
    ComboBox::from_id_salt(id).width(210.0).selected_text(action.label()).show_styled(ui, |ui| {
        let mut group = "";
        let all =
            Action::all().into_iter().chain(memories.iter().map(|m| Action::MemoryRecall(m.id)));
        for a in all {
            if a.group() != group {
                group = a.group();
                ui.add_space(4.0);
                ui.label(RichText::new(group).small().weak());
            }
            if ui.selectable_label(*action == a, a.label()).clicked() {
                *action = a;
                changed = true;
            }
        }
    });
    changed
}

/// Keyboard chords, panadapter mouse behaviour and mouse-button bindings.
///
/// Edits here are live and self-persisting; there is no APPLY chip, because a
/// binding is only useful once it is already in effect.
#[allow(clippy::too_many_arguments)]
pub(in crate::app) fn settings_controls_tab(
    ui: &mut egui::Ui,
    io: &mut SettingsIo,
    memories: &[MemoryChannel],
    midi_in: &[(String, String)],
    midi_out: &[(String, String)],
    midi_status: &crate::input::MidiStatusView,
    last_midi: Option<(sdroxide_types::MidiMsg, u8)>,
) {
    use sdroxide_types::{
        Action, ActionKind, ButtonMode, KeyBinding, MouseButton, MouseButtonBinding, WheelAction,
        WheelSettings,
    };
    let cfg = &mut *io.input_edit;
    let key_capture = &mut *io.key_capture;

    ui.label(
        RichText::new(tr!("settings-controls-keyboard"))
            .size(14.0)
            .strong()
            .color(crate::theme::CYAN()),
    );
    ui.add_space(4.0);
    ui.label(RichText::new(tr!("settings-controls-click-shortcut-rebind-then-press")).weak());
    ui.add_space(6.0);

    let mut remove: Option<usize> = None;
    egui::Grid::new("keys-grid").num_columns(6).spacing([10.0, 6.0]).striped(true).show(ui, |ui| {
        ui.label(RichText::new(tr!("settings-controls-shortcut")).small().weak());
        ui.label(RichText::new(tr!("settings-controls-does")).small().weak());
        ui.label(RichText::new(tr!("settings-controls-step-mode")).small().weak());
        ui.label(RichText::new(tr!("settings-controls-accel")).small().weak());
        ui.label(RichText::new(tr!("settings-controls-on")).small().weak());
        ui.label("");
        ui.end_row();

        for (i, b) in cfg.keys.iter_mut().enumerate() {
            let capturing = *key_capture == Some(i);
            let label =
                if capturing { tr!("settings-controls-press-key") } else { b.chord.label() };
            if crate::chrome::chip(ui, capturing, RichText::new(label).monospace()).clicked() {
                *key_capture = if capturing { None } else { Some(i) };
            }

            if action_combo(ui, ("keyact", i), &mut b.action, memories) {
                b.tuning.step = b.action.default_step();
            }

            match b.action.kind() {
                ActionKind::Continuous => {
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut b.tuning.step)
                                .speed(1.0)
                                .range(0.0001..=1_000_000.0),
                        );
                        // The sign of `value` is the direction, so one
                        // action can have an up key and a down key.
                        let mut down = b.value < 0.0;
                        if crate::chrome::checkbox(ui, &mut down, "down").changed() {
                            b.value = if down { -1.0 } else { 1.0 };
                        }
                    });
                    ui.add(egui::DragValue::new(&mut b.tuning.accel).speed(0.05).range(0.0..=4.0));
                }
                ActionKind::Momentary => {
                    ui.horizontal(|ui| {
                        for m in ButtonMode::ALL {
                            if crate::chrome::chip(ui, b.button == m, m.tr()).clicked() {
                                b.button = m;
                            }
                        }
                    });
                    ui.label("");
                }
            }

            crate::chrome::checkbox(ui, &mut b.enabled, "");
            if ui.small_button("✕").on_hover_text(tr!("settings-controls-remove-binding")).clicked()
            {
                remove = Some(i);
            }
            ui.end_row();
        }
    });
    if let Some(i) = remove {
        cfg.keys.remove(i);
        if *key_capture == Some(i) {
            *key_capture = None;
        }
    }

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if crate::chrome::chip(ui, false, tr!("settings-controls-add-shortcut")).clicked() {
            cfg.keys.push(KeyBinding::default());
            *key_capture = Some(cfg.keys.len() - 1);
        }
        if crate::chrome::chip(ui, false, tr!("settings-controls-restore-defaults")).clicked() {
            cfg.keys = KeyBinding::defaults();
            *key_capture = None;
        }
        // PTT ships unbound on purpose; this is the one-click opt-in.
        let has_ptt = cfg.keys.iter().any(|b| b.action == Action::Ptt);
        if !has_ptt
            && crate::chrome::chip(ui, false, tr!("settings-controls-bind-hold-talk-space"))
                .on_hover_text(tr!("settings-controls-hold-space-transmit-releasing-losing"))
                .clicked()
        {
            cfg.keys.push(KeyBinding {
                chord: sdroxide_types::KeyChord::plain("Space"),
                action: Action::Ptt,
                button: ButtonMode::Momentary,
                ..KeyBinding::default()
            });
        }
    });

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(tr!("settings-controls-unkey-held-ptt-after"));
        ui.add(
            egui::DragValue::new(&mut cfg.ptt_hold_timeout_s)
                .speed(5.0)
                .range(0.0..=3600.0)
                .suffix(" s"),
        );
    })
    .response
    .on_hover_text(tr!("settings-controls-backstop-against-stuck-key-controller"));

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(6.0);
    ui.label(
        RichText::new(tr!("settings-controls-panadapter-mouse"))
            .size(14.0)
            .strong()
            .color(crate::theme::CYAN()),
    );
    ui.add_space(6.0);

    let w = &mut cfg.wheel;
    egui::Grid::new("mouse-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr!("settings-controls-wheel"));
        wheel_action_combo(ui, "wheel-plain", &mut w.wheel);
        ui.end_row();

        ui.label(tr!("settings-controls-wheel-shift"));
        wheel_action_combo(ui, "wheel-shift", &mut w.wheel_shift);
        ui.end_row();

        ui.label(tr!("settings-controls-tune-step"));
        ui.add(
            egui::DragValue::new(&mut w.tune_step_hz).speed(10.0).range(1.0..=1e6).suffix(" Hz"),
        );
        ui.end_row();

        ui.label(tr!("settings-controls-zoom-rate"));
        ui.add(egui::DragValue::new(&mut w.zoom_rate).speed(0.05).range(0.1..=5.0));
        ui.end_row();

        ui.label(tr!("settings-controls-click-tune-rounding"));
        ui.add(
            egui::DragValue::new(&mut w.click_tune_step_hz)
                .speed(1.0)
                .range(1.0..=10_000.0)
                .suffix(" Hz"),
        );
        ui.end_row();
    });
    ui.add_space(4.0);
    crate::chrome::checkbox(ui, &mut w.invert, tr!("settings-controls-invert-wheel-direction"));
    crate::chrome::checkbox(
        ui,
        &mut w.drag_tunes,
        tr!("settings-controls-left-drag-tunes-well-pans"),
    )
    .on_hover_text(tr!("settings-controls-off-makes-left-drag-pan"));
    crate::chrome::checkbox(
        ui,
        &mut w.digit_wheel,
        tr!("settings-controls-scroll-digit-frequency-readout-tune"),
    );
    if w.wheel == WheelAction::Tune && w.wheel_shift == WheelAction::Tune {
        ui.label(
            RichText::new(tr!("settings-controls-both-wheel-actions-are-tune"))
                .color(Color32::from_rgb(230, 170, 60)),
        );
    }
    ui.add_space(6.0);
    if crate::chrome::chip(ui, false, tr!("settings-controls-restore-mouse-defaults")).clicked() {
        cfg.wheel = WheelSettings::default();
    }

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(6.0);
    ui.label(
        RichText::new(tr!("settings-controls-mouse-buttons"))
            .size(14.0)
            .strong()
            .color(crate::theme::CYAN()),
    );
    ui.add_space(4.0);
    ui.label(RichText::new(tr!("settings-controls-left-right-buttons-are-reserved")).weak());
    ui.add_space(6.0);

    let mut remove: Option<usize> = None;
    egui::Grid::new("mousebtn-grid").num_columns(5).spacing([10.0, 6.0]).striped(true).show(
        ui,
        |ui| {
            for (i, b) in cfg.mouse_buttons.iter_mut().enumerate() {
                ComboBox::from_id_salt(("mb", i))
                    .width(130.0)
                    .selected_text(b.button.tr())
                    .show_styled(ui, |ui| {
                        for m in MouseButton::ALL {
                            if ui.selectable_label(b.button == m, m.tr()).clicked() {
                                b.button = m;
                            }
                        }
                    });
                action_combo(ui, ("mbact", i), &mut b.action, memories);
                ui.horizontal(|ui| {
                    for m in ButtonMode::ALL {
                        if crate::chrome::chip(ui, b.button_mode == m, m.tr()).clicked() {
                            b.button_mode = m;
                        }
                    }
                });
                crate::chrome::checkbox(ui, &mut b.enabled, "");
                if ui.small_button("✕").clicked() {
                    remove = Some(i);
                }
                ui.end_row();
            }
        },
    );
    if let Some(i) = remove {
        cfg.mouse_buttons.remove(i);
    }
    ui.add_space(6.0);
    if crate::chrome::chip(ui, false, tr!("settings-controls-add-mouse-button")).clicked() {
        cfg.mouse_buttons.push(MouseButtonBinding::default());
    }

    ui.add_space(8.0);
    ui.label(RichText::new(tr!("settings-controls-f1-always-opens-manual-even")).weak());

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(6.0);
    settings_midi_section(
        ui,
        cfg,
        io.midi_learn,
        io.midi_rescan,
        memories,
        midi_in,
        midi_out,
        midi_status,
        last_midi,
    );
}

/// MIDI control surfaces: port selection, a live message readout, and the
/// binding table with its LEARN capture.
#[allow(clippy::too_many_arguments)]
fn settings_midi_section(
    ui: &mut egui::Ui,
    cfg: &mut sdroxide_types::InputSettings,
    learn: &mut Option<crate::input::MidiLearn>,
    rescan: &mut bool,
    memories: &[MemoryChannel],
    midi_in: &[(String, String)],
    midi_out: &[(String, String)],
    status: &crate::input::MidiStatusView,
    last_midi: Option<(sdroxide_types::MidiMsg, u8)>,
) {
    use sdroxide_types::{ActionKind, ButtonMode, MidiBinding, RelativeMode};

    ui.label(
        RichText::new(tr!("settings-controls-midi-controller"))
            .size(14.0)
            .strong()
            .color(crate::theme::CYAN()),
    );
    ui.add_space(4.0);
    if !status.supported {
        ui.label(RichText::new(tr!("settings-controls-midi-controllers-need-native-app")).weak());
        return;
    }
    ui.label(RichText::new(tr!("settings-controls-any-class-compliant-midi-surface")).weak());
    ui.add_space(6.0);
    crate::chrome::checkbox(ui, &mut cfg.midi.enabled, tr!("settings-controls-enable"));
    ui.add_space(6.0);

    ui.add_enabled_ui(cfg.midi.enabled, |ui| {
        egui::Grid::new("midi-ports").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label(tr!("settings-controls-controller"));
            midi_port_combo(
                ui,
                "midi-in",
                midi_in,
                &mut cfg.midi.in_port_id,
                &mut cfg.midi.in_port_name,
            );
            ui.end_row();

            ui.label(tr!("settings-controls-feedback"));
            midi_port_combo(
                ui,
                "midi-out",
                midi_out,
                &mut cfg.midi.out_port_id,
                &mut cfg.midi.out_port_name,
            );
            ui.end_row();
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if crate::chrome::chip(ui, false, tr!("settings-controls-rescan-ports")).clicked() {
                *rescan = true;
            }
            if status.connected {
                ui.label(
                    RichText::new(format!("● {}", status.port))
                        .color(Color32::from_rgb(90, 200, 110)),
                );
            } else if let Some(e) = &status.error {
                ui.label(RichText::new(e).color(Color32::from_rgb(230, 90, 80)));
            } else {
                ui.label(RichText::new(tr!("settings-controls-not-connected")).weak());
            }
        });
        ui.add_space(4.0);
        // Naming the control that just moved is what makes an unlabelled
        // surface bindable at all.
        match last_midi {
            Some((msg, v)) => ui.label(
                RichText::new(tr!("settings-controls-last-midi", message = msg.label(), value = v))
                    .weak(),
            ),
            None => ui.label(RichText::new(tr!("settings-controls-move-control-see-here")).weak()),
        };
    });

    ui.add_space(8.0);
    let mut remove: Option<usize> = None;
    egui::Grid::new("midi-grid").num_columns(7).spacing([10.0, 6.0]).striped(true).show(ui, |ui| {
        ui.label(RichText::new(tr!("settings-controls-control")).small().weak());
        ui.label(RichText::new(tr!("settings-controls-does")).small().weak());
        ui.label(RichText::new(tr!("settings-controls-reads")).small().weak());
        ui.label(RichText::new(tr!("settings-controls-step-mode")).small().weak());
        ui.label(RichText::new(tr!("settings-controls-led")).small().weak());
        ui.label(RichText::new(tr!("settings-controls-on")).small().weak());
        ui.label("");
        ui.end_row();

        for (i, b) in cfg.midi.bindings.iter_mut().enumerate() {
            let learning = learn.map(|l| l.row) == Some(i);
            let label = if learning { tr!("settings-controls-move") } else { b.msg.label() };
            if crate::chrome::chip(ui, learning, RichText::new(label).monospace())
                .on_hover_text(tr!("settings-controls-click-then-move-control-you"))
                .clicked()
            {
                *learn = if learning { None } else { Some(crate::input::MidiLearn { row: i }) };
            }

            if action_combo(ui, ("midiact", i), &mut b.action, memories) {
                b.tuning.step = b.action.default_step();
            }

            match b.action.kind() {
                ActionKind::Continuous => {
                    ComboBox::from_id_salt(("midirel", i))
                        .width(170.0)
                        .selected_text(b.relative.tr())
                        .show_styled(ui, |ui| {
                            for m in RelativeMode::ALL {
                                if ui.selectable_label(b.relative == m, m.tr()).clicked() {
                                    b.relative = m;
                                }
                            }
                        });
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut b.tuning.step)
                                .speed(1.0)
                                .range(0.0001..=1_000_000.0),
                        );
                        ui.add(
                            egui::DragValue::new(&mut b.tuning.accel)
                                .speed(0.05)
                                .range(0.0..=4.0)
                                .prefix("×"),
                        )
                        .on_hover_text(tr!("settings-controls-speed-sensitivity-spin-faster-tune"));
                        // Sign/magnitude and 64-centred encoders are
                        // indistinguishable from small movements, so a wrong
                        // guess shows up as a knob that turns the wrong way.
                        crate::chrome::checkbox(ui, &mut b.tuning.invert, "rev");
                    });
                }
                ActionKind::Momentary => {
                    ui.label("");
                    ui.horizontal(|ui| {
                        for m in ButtonMode::ALL {
                            if crate::chrome::chip(ui, b.button_mode == m, m.tr()).clicked() {
                                b.button_mode = m;
                            }
                        }
                    });
                }
            }

            crate::chrome::checkbox(ui, &mut b.feedback, "")
                .on_hover_text(tr!("settings-controls-send-current-value-back-light"));
            crate::chrome::checkbox(ui, &mut b.enabled, "");
            if ui.small_button("✕").clicked() {
                remove = Some(i);
            }
            ui.end_row();
        }
    });
    if let Some(i) = remove {
        cfg.midi.bindings.remove(i);
        if learn.map(|l| l.row) == Some(i) {
            *learn = None;
        }
    }

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if crate::chrome::chip(ui, false, tr!("settings-controls-add-midi-control")).clicked() {
            cfg.midi.bindings.push(MidiBinding::default());
            *learn = Some(crate::input::MidiLearn { row: cfg.midi.bindings.len() - 1 });
        }
        if !cfg.midi.bindings.is_empty()
            && crate::chrome::chip(ui, false, tr!("settings-controls-clear-all")).clicked()
        {
            cfg.midi.bindings.clear();
            *learn = None;
        }
    });

    ui.add_space(8.0);
    ui.label(RichText::new(tr!("settings-controls-endless-jog-encoders-send-relative")).weak());
}

/// Port dropdown that keeps both the stable id and the human name — the id
/// reconnects across a replug, the name is the label and the fallback.
fn midi_port_combo(
    ui: &mut egui::Ui,
    id: &str,
    ports: &[(String, String)],
    sel_id: &mut String,
    sel_name: &mut String,
) {
    let shown = if sel_name.is_empty() {
        tr!("settings-controls-none")
    } else {
        String::from(sel_name.as_str())
    };
    ComboBox::from_id_salt(id).width(280.0).selected_text(shown).show_styled(ui, |ui| {
        if ui.selectable_label(sel_name.is_empty(), tr!("settings-controls-none")).clicked() {
            sel_id.clear();
            sel_name.clear();
        }
        for (pid, name) in ports {
            if ui.selectable_label(sel_name == name, name).clicked() {
                *sel_id = pid.clone();
                *sel_name = name.clone();
            }
        }
    });
}

/// Dropdown over [`WheelAction`].
fn wheel_action_combo(ui: &mut egui::Ui, id: &str, act: &mut sdroxide_types::WheelAction) {
    ComboBox::from_id_salt(id).width(130.0).selected_text(act.tr()).show_styled(ui, |ui| {
        for a in sdroxide_types::WheelAction::ALL {
            if ui.selectable_label(*act == a, a.tr()).clicked() {
                *act = a;
            }
        }
    });
}
