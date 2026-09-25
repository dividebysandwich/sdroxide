//! The Controls tab: keyboard, mouse, MIDI and Icom RC-28 bindings.
//!
//! A rebind has no APPLY step — it takes effect on the next frame and is
//! written straight out, because a binding the operator cannot see saved is
//! one they will make again after the next restart. MIDI learn works the same
//! way: arm a row, move the control, and the row captures whatever arrives.

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
    rc28_status: &crate::input::Rc28StatusView,
    last_rc28: Option<&str>,
) {
    use sdroxide_types::{
        Action, ActionKind, ButtonMode, KeyBinding, MouseButton, MouseButtonBinding, WheelAction,
        WheelSettings,
    };
    let cfg = &mut *io.input_edit;
    let key_capture = &mut *io.key_capture;

    ui.label(RichText::new("Keyboard").size(14.0).strong().color(crate::theme::CYAN()));
    ui.add_space(4.0);
    ui.label(
        RichText::new(
            "Click a shortcut to rebind it, then press the key combination (Esc cancels). \
             Bindings are ignored while you are typing in a text field.",
        )
        .weak(),
    );
    ui.add_space(6.0);

    let mut remove: Option<usize> = None;
    egui::Grid::new("keys-grid").num_columns(6).spacing([10.0, 6.0]).striped(true).show(ui, |ui| {
        ui.label(RichText::new("Shortcut").small().weak());
        ui.label(RichText::new("Does").small().weak());
        ui.label(RichText::new("Step / mode").small().weak());
        ui.label(RichText::new("Accel").small().weak());
        ui.label(RichText::new("On").small().weak());
        ui.label("");
        ui.end_row();

        for (i, b) in cfg.keys.iter_mut().enumerate() {
            let capturing = *key_capture == Some(i);
            let label = if capturing { "press a key…".to_string() } else { b.chord.label() };
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
                            if crate::chrome::chip(ui, b.button == m, m.label()).clicked() {
                                b.button = m;
                            }
                        }
                    });
                    ui.label("");
                }
            }

            crate::chrome::checkbox(ui, &mut b.enabled, "");
            if ui.small_button("✕").on_hover_text("Remove this binding").clicked() {
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
        if crate::chrome::chip(ui, false, "+ Add shortcut").clicked() {
            cfg.keys.push(KeyBinding::default());
            *key_capture = Some(cfg.keys.len() - 1);
        }
        if crate::chrome::chip(ui, false, "Restore defaults").clicked() {
            cfg.keys = KeyBinding::defaults();
            *key_capture = None;
        }
        // PTT ships unbound on purpose; this is the one-click opt-in.
        let has_ptt = cfg.keys.iter().any(|b| b.action == Action::Ptt);
        if !has_ptt
            && crate::chrome::chip(ui, false, "Bind hold-to-talk to Space")
                .on_hover_text(
                    "Hold Space to transmit; releasing it — or losing window focus — unkeys",
                )
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
        ui.label("Unkey a held PTT after");
        ui.add(
            egui::DragValue::new(&mut cfg.ptt_hold_timeout_s)
                .speed(5.0)
                .range(0.0..=3600.0)
                .suffix(" s"),
        );
    })
    .response
    .on_hover_text(
        "Backstop against a stuck key or a controller that stops reporting. 0 disables.",
    );

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(6.0);
    ui.label(RichText::new("Panadapter mouse").size(14.0).strong().color(crate::theme::CYAN()));
    ui.add_space(6.0);

    let w = &mut cfg.wheel;
    egui::Grid::new("mouse-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Wheel");
        wheel_action_combo(ui, "wheel-plain", &mut w.wheel);
        ui.end_row();

        ui.label("Wheel + Shift");
        wheel_action_combo(ui, "wheel-shift", &mut w.wheel_shift);
        ui.end_row();

        ui.label("Tune step");
        ui.add(
            egui::DragValue::new(&mut w.tune_step_hz).speed(10.0).range(1.0..=1e6).suffix(" Hz"),
        );
        ui.end_row();

        ui.label("Zoom rate");
        ui.add(egui::DragValue::new(&mut w.zoom_rate).speed(0.05).range(0.1..=5.0));
        ui.end_row();

        ui.label("Click-tune rounding");
        ui.add(
            egui::DragValue::new(&mut w.click_tune_step_hz)
                .speed(1.0)
                .range(1.0..=10_000.0)
                .suffix(" Hz"),
        );
        ui.end_row();
    });
    ui.add_space(4.0);
    crate::chrome::checkbox(ui, &mut w.invert, "Invert wheel direction");
    crate::chrome::checkbox(ui, &mut w.drag_tunes, "Left-drag tunes as well as pans")
        .on_hover_text("Off makes left-drag pan the view only, like right-drag.");
    crate::chrome::checkbox(
        ui,
        &mut w.digit_wheel,
        "Scroll a digit on the frequency readout to tune it",
    );
    if w.wheel == WheelAction::Tune && w.wheel_shift == WheelAction::Tune {
        ui.label(
            RichText::new("Both wheel actions are Tune — there is no way left to zoom.")
                .color(Color32::from_rgb(230, 170, 60)),
        );
    }
    ui.add_space(6.0);
    if crate::chrome::chip(ui, false, "Restore mouse defaults").clicked() {
        cfg.wheel = WheelSettings::default();
    }

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(6.0);
    ui.label(RichText::new("Mouse buttons").size(14.0).strong().color(crate::theme::CYAN()));
    ui.add_space(4.0);
    ui.label(
        RichText::new(
            "The left and right buttons are reserved for tuning and panning; the middle and \
             extra buttons are free. A side button held for PTT behaves like a footswitch.",
        )
        .weak(),
    );
    ui.add_space(6.0);

    let mut remove: Option<usize> = None;
    egui::Grid::new("mousebtn-grid").num_columns(5).spacing([10.0, 6.0]).striped(true).show(
        ui,
        |ui| {
            for (i, b) in cfg.mouse_buttons.iter_mut().enumerate() {
                ComboBox::from_id_salt(("mb", i))
                    .width(130.0)
                    .selected_text(b.button.label())
                    .show_styled(ui, |ui| {
                        for m in MouseButton::ALL {
                            if ui.selectable_label(b.button == m, m.label()).clicked() {
                                b.button = m;
                            }
                        }
                    });
                action_combo(ui, ("mbact", i), &mut b.action, memories);
                ui.horizontal(|ui| {
                    for m in ButtonMode::ALL {
                        if crate::chrome::chip(ui, b.button_mode == m, m.label()).clicked() {
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
    if crate::chrome::chip(ui, false, "+ Add mouse button").clicked() {
        cfg.mouse_buttons.push(MouseButtonBinding::default());
    }

    ui.add_space(8.0);
    ui.label(
        RichText::new("F1 always opens this manual, even while typing, so it is not rebindable.")
            .weak(),
    );

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

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(6.0);
    settings_rc28_section(ui, cfg, io.rc28_choose, memories, rc28_status, last_rc28);
}

/// The Icom RC-28: on/off, the connection, and one row per physical control.
fn settings_rc28_section(
    ui: &mut egui::Ui,
    cfg: &mut sdroxide_types::InputSettings,
    choose: &mut bool,
    memories: &[MemoryChannel],
    status: &crate::input::Rc28StatusView,
    last: Option<&str>,
) {
    use sdroxide_types::{ActionKind, ButtonMode};

    ui.label(RichText::new("Icom RC-28").size(14.0).strong().color(crate::theme::CYAN()));
    ui.add_space(4.0);
    if let Some(why) = status.unsupported {
        ui.label(RichText::new(why).weak());
        return;
    }
    ui.label(
        RichText::new(
            "Icom's USB remote encoder: the knob tunes, TRANSMIT keys the rig while held, and \
             F-1 and F-2 do whatever is chosen below. The LED over a button lights while its \
             action is on; LINK lights while sdroxide has the device.",
        )
        .weak(),
    );
    ui.add_space(6.0);
    let rc = &mut cfg.rc28;
    ui.horizontal(|ui| {
        crate::chrome::checkbox(ui, &mut rc.enabled, "Enable");
        if rc.enabled
            && status.choose
            && crate::chrome::chip(ui, false, "Choose device…")
                .on_hover_text(
                    "The browser only lets a page use a USB device picked here. It remembers \
                     the choice, so this is needed once.",
                )
                .clicked()
        {
            *choose = true;
        }
    });
    // On a line of its own, where it can wrap: the USB product name is long,
    // and beside the checkbox the firmware version fell off the window's edge.
    let s = &status.status;
    if rc.enabled {
        ui.add_space(4.0);
        if s.connected {
            let fw = if s.firmware.is_empty() {
                String::new()
            } else {
                format!(", firmware {}", s.firmware)
            };
            ui.label(
                RichText::new(format!("● {}{fw}", s.name)).color(Color32::from_rgb(90, 200, 110)),
            );
        } else if let Some(e) = &s.error {
            ui.label(RichText::new(e).color(Color32::from_rgb(230, 90, 80)));
        } else if status.choose {
            ui.label(RichText::new("Not connected — choose the device once.").weak());
        } else {
            ui.label(RichText::new("Not connected — plug it in.").weak());
        }
    }
    if rc.enabled {
        ui.add_space(4.0);
        ui.label(
            RichText::new(last.map_or("Turn the knob or press a button to see it here.", |l| l))
                .weak(),
        );
    }
    ui.add_space(6.0);

    ui.add_enabled_ui(rc.enabled, |ui| {
        egui::Grid::new("rc28-grid").num_columns(5).spacing([10.0, 6.0]).striped(true).show(
            ui,
            |ui| {
                ui.label(RichText::new("Control").small().weak());
                ui.label(RichText::new("Does").small().weak());
                ui.label(RichText::new("Step / mode").small().weak());
                ui.label(RichText::new("LED").small().weak());
                ui.label(RichText::new("On").small().weak());
                ui.end_row();

                ui.label(RichText::new("Knob").monospace());
                if action_combo(ui, "rc28-knob", &mut rc.knob, memories) {
                    rc.knob_tuning.step = rc.knob.default_step();
                }
                if rc.knob.kind() == ActionKind::Continuous {
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut rc.knob_tuning.step)
                                .speed(1.0)
                                .range(0.0001..=1_000_000.0),
                        )
                        .on_hover_text("Per detent — about 67 to the turn");
                        ui.add(
                            egui::DragValue::new(&mut rc.knob_tuning.accel)
                                .speed(0.05)
                                .range(0.0..=4.0)
                                .prefix("×"),
                        )
                        .on_hover_text("Speed sensitivity: spin faster to tune faster");
                        crate::chrome::checkbox(ui, &mut rc.knob_tuning.invert, "rev");
                    });
                } else {
                    ui.label(RichText::new("pick a knob action").weak());
                }
                ui.label("");
                ui.label("");
                ui.end_row();

                for (label, b) in
                    [("TRANSMIT", &mut rc.transmit), ("F-1", &mut rc.f1), ("F-2", &mut rc.f2)]
                {
                    ui.label(RichText::new(label).monospace());
                    action_combo(ui, ("rc28-btn", label), &mut b.action, memories);
                    if b.action.kind() == ActionKind::Momentary {
                        ui.horizontal(|ui| {
                            for m in ButtonMode::ALL {
                                if crate::chrome::chip(ui, b.button_mode == m, m.label()).clicked()
                                {
                                    b.button_mode = m;
                                }
                            }
                        });
                    } else {
                        ui.label(RichText::new("pick a button action").weak());
                    }
                    crate::chrome::checkbox(ui, &mut b.led, "")
                        .on_hover_text("Light the LED over the button while its action is on");
                    crate::chrome::checkbox(ui, &mut b.enabled, "");
                    ui.end_row();
                }
            },
        );
    });
    ui.add_space(8.0);
    ui.label(
        RichText::new(
            "TRANSMIT keeps the rig keyed while you type in another window, and lets go if the \
             RC-28 is unplugged. Leave it on Hold unless you mean to latch the transmitter.",
        )
        .weak(),
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

    ui.label(RichText::new("MIDI controller").size(14.0).strong().color(crate::theme::CYAN()));
    ui.add_space(4.0);
    if !status.supported {
        ui.label(
            RichText::new(
                "MIDI controllers need the native app — the browser client has no MIDI access.",
            )
            .weak(),
        );
        return;
    }
    ui.label(
        RichText::new(
            "Any class-compliant MIDI surface works: a DJ controller's jog wheel makes a fine              VFO knob, its pads make PTT and band buttons, its faders make gain controls.",
        )
        .weak(),
    );
    ui.add_space(6.0);
    crate::chrome::checkbox(ui, &mut cfg.midi.enabled, "Enable");
    ui.add_space(6.0);

    ui.add_enabled_ui(cfg.midi.enabled, |ui| {
        egui::Grid::new("midi-ports").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label("Controller");
            midi_port_combo(
                ui,
                "midi-in",
                midi_in,
                &mut cfg.midi.in_port_id,
                &mut cfg.midi.in_port_name,
            );
            ui.end_row();

            ui.label("Feedback to");
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
            if crate::chrome::chip(ui, false, "Rescan ports").clicked() {
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
                ui.label(RichText::new("Not connected.").weak());
            }
        });
        ui.add_space(4.0);
        // Naming the control that just moved is what makes an unlabelled
        // surface bindable at all.
        match last_midi {
            Some((msg, v)) => {
                ui.label(RichText::new(format!("Last message: {}  value {v}", msg.label())).weak())
            }
            None => ui.label(RichText::new("Move a control to see it here.").weak()),
        };
    });

    ui.add_space(8.0);
    let mut remove: Option<usize> = None;
    egui::Grid::new("midi-grid").num_columns(7).spacing([10.0, 6.0]).striped(true).show(ui, |ui| {
        ui.label(RichText::new("Control").small().weak());
        ui.label(RichText::new("Does").small().weak());
        ui.label(RichText::new("Reads as").small().weak());
        ui.label(RichText::new("Step / mode").small().weak());
        ui.label(RichText::new("LED").small().weak());
        ui.label(RichText::new("On").small().weak());
        ui.label("");
        ui.end_row();

        for (i, b) in cfg.midi.bindings.iter_mut().enumerate() {
            let learning = learn.map(|l| l.row) == Some(i);
            let label = if learning { "move it…".to_string() } else { b.msg.label() };
            if crate::chrome::chip(ui, learning, RichText::new(label).monospace())
                .on_hover_text("Click, then move the control you want to bind")
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
                        .selected_text(b.relative.label())
                        .show_styled(ui, |ui| {
                            for m in RelativeMode::ALL {
                                if ui.selectable_label(b.relative == m, m.label()).clicked() {
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
                        .on_hover_text("Speed sensitivity: spin faster to tune faster");
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
                            if crate::chrome::chip(ui, b.button_mode == m, m.label()).clicked() {
                                b.button_mode = m;
                            }
                        }
                    });
                }
            }

            crate::chrome::checkbox(ui, &mut b.feedback, "")
                .on_hover_text("Send the current value back, to light an LED or move a fader");
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
        if crate::chrome::chip(ui, false, "+ Add MIDI control").clicked() {
            cfg.midi.bindings.push(MidiBinding::default());
            *learn = Some(crate::input::MidiLearn { row: cfg.midi.bindings.len() - 1 });
        }
        if !cfg.midi.bindings.is_empty() && crate::chrome::chip(ui, false, "Clear all").clicked() {
            cfg.midi.bindings.clear();
            *learn = None;
        }
    });

    ui.add_space(8.0);
    ui.label(
        RichText::new(
            "Endless (jog) encoders send a relative step rather than a position, in one of three              encodings that look alike from small movements. LEARN guesses from a clockwise turn;              if the knob then tunes the wrong way, tick \u{201c}rev\u{201d}.",
        )
        .weak(),
    );
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
    let shown = if sel_name.is_empty() { "— none —" } else { sel_name.as_str() };
    ComboBox::from_id_salt(id).width(280.0).selected_text(shown).show_styled(ui, |ui| {
        if ui.selectable_label(sel_name.is_empty(), "— none —").clicked() {
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
    ComboBox::from_id_salt(id).width(130.0).selected_text(act.label()).show_styled(ui, |ui| {
        for a in sdroxide_types::WheelAction::ALL {
            if ui.selectable_label(*act == a, a.label()).clicked() {
                *act = a;
            }
        }
    });
}
