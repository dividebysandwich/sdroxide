//! The Radio tab: one section per radio interface.
//!
//! Which section is drawn follows the interface selector above it, so the
//! dialog only ever shows the settings of the backend being configured. The
//! discovery buttons (HPSDR scan, RTL-SDR rescan, TCI connection test) only
//! set a flag — the blocking work happens after the window closure.

use crate::i18n::{Tr, tr};
use eframe::egui::{self, Color32, ComboBox, DragValue, RichText, Slider};
use sdroxide_types::{Command, Direction};

use crate::app::SdroxideApp;
use crate::app::settings::{device_combo, enum_combo};
use crate::chrome::StyledCombo;

/// Why a discovery or test control is greyed out.
///
/// One wording rather than a dozen: every one of these buttons asks a question
/// about a *machine* — what is on its USB bus, which serial ports it has,
/// whether an address answers from where it stands. That machine is the one the
/// radio is attached to, which may not be this screen, so the question is sent
/// there and the answer comes back ([`sdroxide_types::DeviceProbe`]). These
/// controls are therefore live from a remote or browser client too, and are
/// only dark while an earlier question is still out — or where that machine
/// answers none at all.
const NO_ANSWER_YET: &str = "Waiting for the machine the radio is attached to: these ask about \
                             its hardware and its network, and it answers one at a time.";

/// Draw a control that has to ask the radio's own machine, greyed out and
/// explained while that machine has not answered.
fn probe_only<R>(ui: &mut egui::Ui, can_probe: bool, add: impl FnOnce(&mut egui::Ui) -> R) {
    let group = ui.add_enabled_ui(can_probe, add);
    if !can_probe {
        group.response.on_hover_text(NO_ANSWER_YET);
    }
}

/// CAT / Audio interface: serial + PTT parameters (the interface itself is
/// chosen by the selector in `settings_body`).
/// The antenna-socket row for a radio whose port list is the radio's own —
/// every Icom with a selector, reached over CAT or over the network.
///
/// Drawn from the *capabilities* rather than from the configuration, because
/// there is nothing here to configure: whether the radio has a selector is only
/// known once it has answered the read its control link sends when the session
/// opens, and a radio with one connector NAKs that and gets no row at all
/// (issues #235, #238). Applies immediately, like every other antenna control.
fn radio_antenna_row(
    ui: &mut egui::Ui,
    caps: Option<&sdroxide_types::DeviceCaps>,
    antenna_rx: &str,
    id: &str,
    cmds: &mut Vec<Command>,
) {
    let ports: Vec<String> = caps.map(|c| c.antennas_rx.clone()).unwrap_or_default();
    if ports.len() < 2 {
        return;
    }
    ui.label(tr!("settings-radio-antenna"))
        .on_hover_text(tr!("settings-radio-which-socket-back-radio-using"));
    let shown = if antenna_rx.is_empty() { "—" } else { antenna_rx };
    ComboBox::from_id_salt(id).selected_text(shown).show_styled(ui, |ui| {
        for a in &ports {
            if ui.selectable_label(antenna_rx == a, a).clicked() {
                cmds.push(Command::SetAntenna { dir: Direction::Rx, name: a.clone() });
            }
        }
    });
    ui.end_row();
}

/// The radio's separate *receiving* antenna, for a control link that carries
/// one (issue #229).
///
/// Not the same thing as the row above, and the difference is the whole reason
/// it exists: that one picks between the sockets a transmitter can also use,
/// while this switches an extra receive-only input into the path and leaves the
/// main aerial on transmit throughout. An IC-7300MK2 has one aerial socket and
/// this; an IC-7610 has both.
///
/// Whether the radio has one is learned rather than claimed — the flag rides
/// behind the socket in its antenna reply, and a radio without the connector
/// answers the socket alone — so the row appears a round trip after the session
/// opens, or not at all.
fn radio_rx_antenna_row(
    ui: &mut egui::Ui,
    caps: Option<&sdroxide_types::DeviceCaps>,
    rx_antenna: bool,
    cmds: &mut Vec<Command>,
) {
    if !caps.is_some_and(|c| c.has_rx_antenna) {
        return;
    }
    ui.label(tr!("settings-radio-receive-antenna"))
        .on_hover_text(tr!("settings-radio-radio-s-separate-receiving-antenna"));
    let mut on = rx_antenna;
    if ui.checkbox(&mut on, tr!("settings-radio-receive-path")).changed() {
        cmds.push(Command::SetRxAntenna(on));
    }
    ui.end_row();
}

/// The radio's own power switch, for a control link that carries one.
///
/// Two buttons rather than a toggle, because there is nothing here that *reads*
/// the switch: a radio that is off answers nothing, so the only honest thing a
/// toggle could show is the last thing it was told, and an operator away from
/// the shack would be reading their own last click back. Two buttons say what
/// they do and claim nothing about what the radio is currently doing.
fn radio_power_row(
    ui: &mut egui::Ui,
    caps: Option<&sdroxide_types::DeviceCaps>,
    cmds: &mut Vec<Command>,
) {
    if !caps.is_some_and(|c| c.commands_rig_power) {
        return;
    }
    ui.label(tr!("settings-radio-radio-power"))
        .on_hover_text(tr!("settings-radio-switch-radio-itself-off-back"));
    ui.horizontal(|ui| {
        if ui.button(tr!("settings-radio-on")).clicked() {
            cmds.push(Command::SetRigPower(true));
        }
        if ui.button(tr!("settings-radio-off")).clicked() {
            cmds.push(Command::SetRigPower(false));
        }
    });
    ui.end_row();
}

pub(in crate::app) fn settings_cat_tab(
    ui: &mut egui::Ui,
    serial_ports: &[String],
    radio_edit: &mut Option<sdroxide_types::RadioConfig>,
    caps: Option<&sdroxide_types::DeviceCaps>,
    // Which antenna socket the radio says it is receiving on — for the
    // families here whose rigs have two — and whether its separate receiving
    // antenna, where it has one, is in the receive path.
    antenna_rx: &str,
    rx_antenna: bool,
    // The sound cards on the machine the *rig* is plugged into, once that
    // machine has answered. `None` until it has.
    radio_audio: Option<(&[String], &[String])>,
    can_probe: bool,
    apply: &mut bool,
    cmds: &mut Vec<Command>,
) {
    use sdroxide_types::{
        CAT_SCOPE_MIN_BAUD, CatFamily, CwKeying, DigiMode, Direction, ELAD_CAT_BAUDS,
        ELAD_DEFAULT_CAT_BAUD, EladAntenna, EladTxInput, IcomModel, IcomScopeSpan, KenwoodSend,
        LineState, ModeControl, Parity, PttMethod, QMX_IQ_OFFSET_HZ, QMX_IQ_RATE_HZ,
        RS_HFIQ_CAT_BAUD, SoundFormat, StopBits, TrUsdxAudio,
    };
    let Some(cfg) = radio_edit.as_mut() else {
        ui.label(tr!("settings-radio-waiting-configuration-machine-radio-attached"));
        return;
    };
    egui::Grid::new("cat-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        // What the sound format and the family were before this frame's combo
        // boxes ran. A QMX's I/Q is not centred on its dial and its card runs
        // at one rate, and both are filled in when the operator arrives at that
        // combination — see where they are applied, below the family picker.
        let format_before = cfg.cat.format;
        let family_before = cfg.cat.family;

        ui.label(tr!("settings-radio-sound-format"));
        enum_combo(ui, "sfmt", &mut cfg.cat.format, &SoundFormat::ALL, SoundFormat::tr);
        ui.end_row();

        // Only meaningful for I/Q: demod audio is a real signal, with no
        // sideband to swap.
        if matches!(cfg.cat.format, SoundFormat::Iq) {
            ui.label(tr!("settings-radio-invert-spectrum"));
            crate::chrome::checkbox(
                ui,
                &mut cfg.cat.invert_spectrum,
                tr!("settings-radio-swap-i-q"),
            )
            .on_hover_text(tr!("settings-radio-mirror-panadapter-about-tuned-frequency"));
            ui.end_row();

            ui.label(tr!("settings-radio-i-q-sample-rate"))
                .on_hover_text(tr!("settings-radio-how-fast-radio-s-i"));
            // Labelled with the span each rate buys as well as the rate itself:
            // the width is the reason to touch this, and it is the half of it
            // an operator cannot work out at a glance.
            enum_combo(ui, "iqrate", &mut cfg.cat.iq_rate_hz, &sdroxide_types::CAT_IQ_RATES, |r| {
                match r {
                    48_000 => tr!("settings-radio-t-48-khz-24-khz"),
                    96_000 => tr!("settings-radio-t-96-khz-48-khz"),
                    192_000 => tr!("settings-radio-t-192-khz-96-khz"),
                    384_000 => tr!("settings-radio-t-384-khz-192-khz"),
                    _ => String::from("—"),
                }
            });
            ui.end_row();

            ui.label(tr!("settings-radio-i-q-centre-offset"))
                .on_hover_text(tr!("settings-radio-how-far-radio-s-i"));
            ui.add(
                DragValue::new(&mut cfg.cat.iq_offset_hz)
                    .speed(100.0)
                    .range({
                        let max = sdroxide_types::cat_iq_offset_max_hz(cfg.cat.iq_rate_hz);
                        -max..=max
                    })
                    .suffix(" Hz"),
            );
            ui.end_row();

            ui.label(tr!("settings-radio-iq-correction"))
                .on_hover_text(tr!("settings-radio-cancel-mirror-image-every-signal"));
            crate::chrome::checkbox(
                ui,
                &mut cfg.cat.iq_correction,
                tr!("settings-radio-cancel-mirror-images"),
            );
            ui.end_row();

            ui.label(tr!("settings-radio-dc-notch"))
                .on_hover_text(tr!("settings-radio-widen-hole-taken-out-middle"));
            ui.add(
                DragValue::new(&mut cfg.cat.iq_dc_block_hz)
                    .speed(10.0)
                    .range(0.0..=sdroxide_types::CAT_IQ_DC_BLOCK_MAX_HZ)
                    .suffix(" Hz"),
            );
            ui.end_row();
        }

        if matches!(cfg.cat.format, SoundFormat::DemodAudio) {
            ui.label(tr!("settings-radio-panadapter-bw"));
            ui.add(
                DragValue::new(&mut cfg.cat.audio_bw_hz)
                    .speed(100.0)
                    .range(1000.0..=24000.0)
                    .suffix(" Hz"),
            );
            ui.end_row();
        }

        ui.label(tr!("settings-radio-cat-family"));
        enum_combo(ui, "fam", &mut cfg.cat.family, &CatFamily::ALL, CatFamily::tr);
        ui.end_row();

        // A QMX's I/Q is a superhet's, not a direct-conversion receiver's: the
        // synthesiser sits 12 kHz below the dial, so everything on the sound
        // card is 12 kHz above the middle of the span, and the card itself runs
        // at one rate and no other. Both are filled in the moment the operator
        // arrives at that combination — the same treatment the Icom model gives
        // the CI-V address, and for the same reason: the right value is a fact
        // about the radio, not a preference, and an operator who has to find it
        // out by watching signals land in the wrong place has been let down.
        //
        // Only on a *change*, so somebody who has deliberately moved either one
        // — a converter in front of the radio, a card that will not do 48 k —
        // does not have it put back every time this dialog is drawn.
        let arrived_at_qmx_iq = cfg.cat.family == CatFamily::QrpLabs
            && matches!(cfg.cat.format, SoundFormat::Iq)
            && (cfg.cat.family != family_before || cfg.cat.format != format_before);
        if arrived_at_qmx_iq {
            cfg.cat.iq_offset_hz = QMX_IQ_OFFSET_HZ;
            cfg.cat.iq_rate_hz = QMX_IQ_RATE_HZ;
        }

        // An RS-HFIQ, for the same reason and more of it. Its sound card is
        // complex baseband and nothing else, its serial port is 57600 8N1 in
        // the firmware with no menu to change it, and its only transmit switch
        // is the `*X` command — so all three are facts about the radio rather
        // than preferences, and every one of them left at a default would be a
        // link that does not work or an audio path that is noise (issue #383).
        // The oscillator is on the dial, so the I/Q offset goes to zero:
        // whatever the previous radio needed is not what this one does.
        if cfg.cat.family == CatFamily::RsHfiq && cfg.cat.family != family_before {
            cfg.cat.format = SoundFormat::Iq;
            cfg.cat.serial.baud = RS_HFIQ_CAT_BAUD;
            cfg.cat.ptt = PttMethod::Cat;
            cfg.cat.iq_offset_hz = 0.0;
        }

        // A (tr)uSDX is a USB serial adapter and nothing else: no sound card,
        // so the sound format is demod audio coming down the CAT link, and PTT
        // is a CAT command. The rate is 115200 on firmware 2.00t and up (the
        // older 38400 is still offered below). DTR is the radio's reset line on
        // the common board and the driver holds it high whatever is configured
        // here, so a key-down on DTR is not offered by the profile.
        if cfg.cat.family == CatFamily::TrUsdx && cfg.cat.family != family_before {
            cfg.cat.format = SoundFormat::DemodAudio;
            cfg.cat.ptt = PttMethod::Cat;
            cfg.cat.serial.baud = 115_200;
            cfg.cat.serial.data_bits = 8;
            cfg.cat.serial.parity = Parity::None;
            cfg.cat.serial.stop_bits = StopBits::One;
            cfg.cat.serial.force_dtr = LineState::High;
        }

        // A network family reaches the radio over a socket, so every serial
        // setting below is about a port nothing will open. Drawing them would
        // invite an operator to fix a connection problem by changing a baud
        // rate that has no part in it.
        let serial = !cfg.cat.family.is_network();

        if cfg.cat.family == CatFamily::Rigctld {
            ui.label(tr!("settings-radio-rigctld-address"))
                .on_hover_text(tr!("settings-radio-host-port-running-hamlib-rigctld"));
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut cfg.cat.rigctld_addr)
                    .desired_width(200.0)
                    .hint_text("127.0.0.1:4532"),
            );
            ui.end_row();
        }

        if cfg.cat.family == CatFamily::Flrig {
            ui.label(tr!("settings-radio-flrig-address"))
                .on_hover_text(tr!("settings-radio-host-port-running-flrig-127"));
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut cfg.cat.flrig_addr)
                    .desired_width(200.0)
                    .hint_text("127.0.0.1:12345"),
            );
            ui.end_row();
        }

        if serial {
            ui.label(tr!("settings-radio-serial-port"));
            let shown = if cfg.cat.serial.path.is_empty() {
                tr!("settings-radio-select")
            } else {
                cfg.cat.serial.path.clone()
            };
            // The list is of *this* machine's ports. Where the rig is elsewhere
            // the stored path is still worth showing — it says which port the
            // engine is using — but there is nothing here to choose from.
            probe_only(ui, can_probe, |ui| {
                ComboBox::from_id_salt("serport").width(260.0).selected_text(shown).show_styled(
                    ui,
                    |ui| {
                        for p in serial_ports {
                            if ui.selectable_label(&cfg.cat.serial.path == p, p).clicked() {
                                cfg.cat.serial.path = p.clone();
                            }
                        }
                    },
                );
            });
            ui.end_row();
        }

        if serial {
            ui.label(tr!("settings-radio-baud"));
            // Most families take whatever the operator has set at the radio, so
            // the whole spread is offered. An ELAD is the exception: an FDM-DUO's
            // CAT port has four rates and no others, and at any other one the
            // link is silent both ways rather than merely slow — so offering
            // 19200 here (which is also this block's own default) would be
            // offering a link that cannot work. See `sdroxide_cat::spawn`.
            let bauds: &[u32] = if cfg.cat.family == CatFamily::Elad {
                &ELAD_CAT_BAUDS
            } else if cfg.cat.family == CatFamily::TrUsdx {
                // "38400 / 115200 (2.00t and above)" — the firmware's own two
                // rates, and nothing else it has a setting for.
                &[38_400, 115_200]
            } else {
                &[4800, 9600, 19200, 38400, 57600, 115200]
            };
            ui.horizontal(|ui| {
                ComboBox::from_id_salt("baud")
                    .selected_text(cfg.cat.serial.baud.to_string())
                    .show_styled(ui, |ui| {
                        for &b in bauds {
                            if ui
                                .selectable_label(cfg.cat.serial.baud == b, b.to_string())
                                .clicked()
                            {
                                cfg.cat.serial.baud = b;
                            }
                        }
                    });
                // A rate the combo is showing without offering — which for an
                // ELAD is where an untouched config starts, this block's own
                // default being 19200.
                if !bauds.contains(&cfg.cat.serial.baud) {
                    ui.add(
                        egui::Label::new(
                            RichText::new(tr!(
                                "settings-radio-no-such-baud",
                                baud = cfg.cat.serial.baud,
                                default = ELAD_DEFAULT_CAT_BAUD
                            ))
                            .color(crate::theme::YELLOW()),
                        )
                        .wrap(),
                    );
                }
            });
            ui.end_row();

            ui.label(tr!("settings-radio-data-bits"));
            ComboBox::from_id_salt("databits")
                .selected_text(cfg.cat.serial.data_bits.to_string())
                .show_styled(ui, |ui| {
                    for d in [7u8, 8] {
                        if ui
                            .selectable_label(cfg.cat.serial.data_bits == d, d.to_string())
                            .clicked()
                        {
                            cfg.cat.serial.data_bits = d;
                        }
                    }
                });
            ui.end_row();

            ui.label(tr!("settings-radio-parity"));
            enum_combo(ui, "parity", &mut cfg.cat.serial.parity, &Parity::ALL, Parity::tr);
            ui.end_row();

            ui.label(tr!("settings-radio-stop-bits"));
            enum_combo(ui, "stop", &mut cfg.cat.serial.stop_bits, &StopBits::ALL, StopBits::label);
            ui.end_row();

            ui.label(tr!("settings-radio-force-rts"));
            enum_combo(ui, "rts", &mut cfg.cat.serial.force_rts, &LineState::ALL, LineState::tr);
            ui.end_row();
            ui.label(tr!("settings-radio-force-dtr"));
            enum_combo(ui, "dtr", &mut cfg.cat.serial.force_dtr, &LineState::ALL, LineState::tr);
            ui.end_row();
        }

        ui.label(tr!("settings-radio-ptt-method")).on_hover_text(if serial {
            tr!("settings-radio-how-transmit-keyed")
        } else {
            tr!("settings-radio-how-transmit-keyed-network-link")
        });
        enum_combo(ui, "ptt", &mut cfg.cat.ptt, &PttMethod::ALL, PttMethod::label);
        ui.end_row();

        ui.label(tr!("settings-radio-mode-control"));
        enum_combo(ui, "modectl", &mut cfg.cat.mode_control, &ModeControl::ALL, ModeControl::tr);
        ui.end_row();

        ui.label(tr!("settings-radio-digimode-mode"))
            .on_hover_text(tr!("settings-radio-what-put-radio-mode-sdroxide"));
        enum_combo(ui, "digimode", &mut cfg.cat.digi_mode, &DigiMode::ALL, DigiMode::tr);
        ui.end_row();

        ui.label(tr!("settings-radio-cw-keying"))
            .on_hover_text(tr!("settings-radio-how-cw-panel-s-keyer"));
        enum_combo(ui, "cwkey", &mut cfg.cat.cw_keying, &CwKeying::ALL, CwKeying::tr);
        ui.end_row();

        ui.label(tr!("settings-radio-poll-rate"))
            .on_hover_text(tr!("settings-radio-how-often-radio-asked-what"));
        ui.add(DragValue::new(&mut cfg.cat.poll_hz).speed(0.5).range(0.5..=20.0).suffix(" Hz"));
        ui.end_row();

        if cfg.cat.family == CatFamily::Kenwood {
            ui.label(tr!("settings-radio-send-command"))
                .on_hover_text(tr!("settings-radio-which-transceiver-generation-keys-rig"));
            enum_combo(ui, "kwsend", &mut cfg.cat.kenwood_send, &KenwoodSend::ALL, KenwoodSend::tr);
            ui.end_row();
        }

        if cfg.cat.family == CatFamily::Elecraft {
            ui.label(tr!("settings-radio-radio"));
            ui.label(RichText::new(tr!("settings-radio-k3-k3s-kx3-kx2-k4")).weak())
                .on_hover_text(tr!("settings-radio-one-profile-covers-family-k3"));
            ui.end_row();
        }

        if cfg.cat.family == CatFamily::Elad {
            ui.label(tr!("settings-radio-transmit-input"))
                .on_hover_text(tr!("settings-radio-where-radio-takes-transmit-audio"));
            enum_combo(
                ui,
                "eladtxin",
                &mut cfg.cat.elad_tx_input,
                &EladTxInput::ALL,
                EladTxInput::tr,
            );
            ui.end_row();

            ui.label(tr!("settings-radio-antenna"))
                .on_hover_text(tr!("settings-radio-which-two-sockets-back-receiver"));
            let shown = if antenna_rx.is_empty() { "—" } else { antenna_rx };
            ComboBox::from_id_salt("cat_elad_antenna").selected_text(shown).show_styled(ui, |ui| {
                for a in EladAntenna::ALL {
                    if ui.selectable_label(antenna_rx == a.label(), a.label()).clicked() {
                        cmds.push(Command::SetAntenna {
                            dir: Direction::Rx,
                            name: a.label().to_string(),
                        });
                    }
                }
            });
            ui.end_row();

            ui.label(tr!("settings-radio-radio"));
            ui.label(RichText::new("FDM-DUO · FDM-DUOr").weak())
                .on_hover_text(tr!("settings-radio-one-profile-covers-both-there"));
            ui.end_row();
        }

        if cfg.cat.family == CatFamily::QrpLabs {
            ui.label(tr!("settings-radio-radio"));
            ui.label(RichText::new("QMX · QMX+ · QDX").weak())
                .on_hover_text(tr!("settings-radio-one-profile-covers-range-qrp"));
            ui.end_row();

            ui.label("");
            ui.label(RichText::new(tr!("settings-radio-baud-rate-ignored-port-usb")).weak())
                .on_hover_text(tr!("settings-radio-qmx-serves-own-virtual-com"));
            ui.end_row();

            if matches!(cfg.cat.format, SoundFormat::Iq) {
                ui.label("");
                ui.label(RichText::new(tr!("settings-radio-i-q-mode-switched-radio")).weak())
                    .on_hover_text(tr!("settings-radio-radio-s-sound-card-carries"));
                ui.end_row();
            }
        }

        if cfg.cat.family == CatFamily::RsHfiq {
            ui.label(tr!("settings-radio-radio"));
            ui.label(RichText::new(tr!("settings-radio-rs-hfiq-5-w-hf")).weak())
                .on_hover_text(tr!("settings-radio-hobbypcb-s-rs-hfiq-there"));
            ui.end_row();

            ui.label("");
            ui.label(RichText::new(tr!("settings-radio-sound-format-baud-ptt-are")).weak())
                .on_hover_text(tr!("settings-radio-all-three-have-been-filled"));
            ui.end_row();
        }

        if cfg.cat.family == CatFamily::TrUsdx {
            ui.label(tr!("settings-radio-radio"));
            ui.label(RichText::new("(tr)uSDX · open uSDX").weak())
                .on_hover_text(tr!("settings-radio-dl2man-pe1nnz-s-pocket-qrp"));
            ui.end_row();

            ui.label(tr!("settings-radio-audio"))
                .on_hover_text(tr!("settings-radio-tr-usdx-has-no-sound"));
            enum_combo(
                ui,
                "trusdx_audio",
                &mut cfg.cat.trusdx_audio,
                &TrUsdxAudio::ALL,
                TrUsdxAudio::tr,
            );
            ui.end_row();
        }

        if cfg.cat.family == CatFamily::Icom {
            ui.label(tr!("settings-radio-radio-model"))
                .on_hover_text(tr!("settings-radio-which-icom-two-things-ci"));
            let before = cfg.cat.icom_model;
            enum_combo(ui, "icommodel", &mut cfg.cat.icom_model, &IcomModel::ALL, IcomModel::tr);
            // Only on a change, so an operator who has deliberately re-addressed
            // their radio does not have it overwritten every time this dialog
            // is drawn.
            if cfg.cat.icom_model != before
                && let Some(addr) = cfg.cat.icom_model.civ_addr()
            {
                cfg.cat.icom_radio_id = addr;
            }
            ui.end_row();
        }

        if cfg.cat.family == CatFamily::Icom {
            radio_antenna_row(ui, caps, antenna_rx, "cat_icom_antenna", cmds);
            radio_rx_antenna_row(ui, caps, rx_antenna, cmds);
            radio_power_row(ui, caps, cmds);
        }

        if matches!(cfg.cat.family, CatFamily::Icom | CatFamily::Xiegu) {
            ui.label(tr!("settings-radio-radio-id-hex"));
            let mut hex = format!("{:02X}", cfg.cat.icom_radio_id);
            let resp =
                crate::chrome::field(ui, egui::TextEdit::singleline(&mut hex).desired_width(48.0));
            if resp.changed() {
                if let Ok(v) = u8::from_str_radix(hex.trim().trim_start_matches("0x"), 16) {
                    cfg.cat.icom_radio_id = v;
                }
            }
            ui.end_row();
        }

        if cfg.cat.family == CatFamily::Icom {
            ui.label("");
            crate::chrome::checkbox(
                ui,
                &mut cfg.cat.scope,
                tr!("settings-radio-show-radio-s-spectrum-scope"),
            )
            .on_hover_text(tr!("settings-radio-icom-scope-hint", min_baud = CAT_SCOPE_MIN_BAUD));
            ui.end_row();

            if cfg.cat.scope {
                ui.label(tr!("settings-radio-scope-span"))
                    .on_hover_text(tr!("settings-radio-how-wide-sweep-radio-keeps"));
                ComboBox::from_id_salt("cat_scope_span")
                    .selected_text(cfg.cat.scope_span.tr())
                    .show_styled(ui, |ui| {
                        for sp in IcomScopeSpan::ALL {
                            if ui.selectable_label(cfg.cat.scope_span == sp, sp.tr()).clicked() {
                                cfg.cat.scope_span = sp;
                            }
                        }
                    });
                ui.end_row();
            }
        }
    });
    cat_radio_audio(ui, cfg, radio_audio, can_probe, apply);
    ui.add_space(6.0);
    ui.label(RichText::new(tr!("settings-radio-press-apply-reconnect-switch-without")).weak());
}

/// The rig's own sound card, and how loud what comes back off it is.
///
/// On this tab rather than under General, where it used to live: a sound card
/// belongs to a radio, not to the program. A station running two rigs at once
/// runs two interfaces at once, and one pair of card pickers on a shared page
/// could only ever describe one of them (issue #474). The `UsbAudio` backend's
/// identical pickers have always been on this tab; these are now beside them.
///
/// Only the CAT interface has this: every other backend carries its audio in
/// the same stream as its I/Q.
///
/// The cards offered are the ones on the machine the *radio* is plugged into,
/// asked for by name rather than taken from the audio-device list — that list
/// is this screen's own speaker and microphone, and offering a laptop's
/// built-in mic as the shack transceiver's transmit path would be worse than
/// offering nothing at all.
fn cat_radio_audio(
    ui: &mut egui::Ui,
    cfg: &mut sdroxide_types::RadioConfig,
    devices: Option<(&[String], &[String])>,
    can_probe: bool,
    apply: &mut bool,
) {
    ui.add_space(10.0);
    ui.separator();
    ui.add_space(6.0);
    ui.label(RichText::new(tr!("settings-radio-radio-audio-sound-card")).strong());
    let Some((inputs, outputs)) = devices else {
        ui.label(RichText::new(tr!("settings-radio-waiting-sound-cards-machine-radio")).weak());
        return;
    };
    let (ci, co) = (cfg.radio_audio_in.clone(), cfg.radio_audio_out.clone());
    egui::Grid::new("radio-audio").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr!("settings-radio-radio-rx"));
        probe_only(ui, can_probe, |ui| {
            device_combo(ui, "r-in", inputs, &ci, |n| cfg.radio_audio_in = n)
        });
        ui.end_row();
        ui.label(tr!("settings-radio-radio-tx"));
        probe_only(ui, can_probe, |ui| {
            device_combo(ui, "r-out", outputs, &co, |n| cfg.radio_audio_out = n)
        });
        ui.end_row();
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui
            .button(tr!("settings-radio-apply-reconnect"))
            .on_hover_text(tr!("settings-radio-reopen-cat-rig-these-sound"))
            .clicked()
        {
            *apply = true;
        }
        ui.add(
            egui::Label::new(
                RichText::new(tr!("settings-radio-reconnects-radio-without-restarting")).weak(),
            )
            .wrap(),
        );
    });
    crate::app::settings::general::settings_rx_audio_gain(ui, cfg);
}

/// The sound-card radio: no control cable, so the whole tab is the two card
/// choices (receive in, transmit out) and the reminder that the rig keys itself
/// off VOX. Drawn after the CAT tab so the two sink-holes for audio devices
/// sit together in the file.
pub(in crate::app) fn settings_usb_audio_tab(
    ui: &mut egui::Ui,
    devices: Option<(&[String], &[String])>,
    radio_edit: &mut Option<sdroxide_types::RadioConfig>,
    apply: &mut bool,
    can_probe: bool,
) {
    let Some(cfg) = radio_edit.as_mut() else {
        ui.label(tr!("settings-radio-waiting-configuration-machine-radio-attached"));
        return;
    };
    // The two lists are the cards on the machine the radio is plugged into,
    // and they arrive as an answer. Until then the combos would offer nothing
    // but "System default", which reads as a machine with no sound cards.
    let Some((inputs, outputs)) = devices else {
        ui.label(RichText::new(tr!("settings-radio-waiting-sound-cards-machine-radio")).weak());
        return;
    };
    // Read out before the combos, which hand the fields to their editors.
    let (ci, co) = (cfg.radio_audio_in.clone(), cfg.radio_audio_out.clone());
    egui::Grid::new("usb-audio-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        // Only the combos are greyed: wrapping whole rows would put both of
        // them, `end_row` and all, inside a single cell of the grid.
        ui.label(tr!("settings-radio-receive-radio-pc"))
            .on_hover_text(tr!("settings-radio-sound-card-radio-s-own"));
        probe_only(ui, can_probe, |ui| {
            device_combo(ui, "ua-in", inputs, &ci, |n| cfg.radio_audio_in = n)
        });
        ui.end_row();
        ui.label(tr!("settings-radio-transmit-pc-radio-mic"))
            .on_hover_text(tr!("settings-radio-sound-card-carries-audio-radio"));
        probe_only(ui, can_probe, |ui| {
            device_combo(ui, "ua-out", outputs, &co, |n| cfg.radio_audio_out = n)
        });
        ui.end_row();
    });
    ui.add_space(4.0);
    ui.label(RichText::new(tr!("settings-radio-radio-has-no-control-cable")).weak());
    ui.horizontal(|ui| {
        if ui
            .button(tr!("settings-radio-apply-reconnect"))
            .on_hover_text(tr!("settings-radio-reopen-radio-these-sound-cards"))
            .clicked()
        {
            *apply = true;
        }
        ui.add(
            egui::Label::new(
                RichText::new(tr!("settings-radio-reconnects-radio-without-restarting")).weak(),
            )
            .wrap(),
        );
    });
}

/// HPSDR interface: network device discovery / manual IP / sample rate (the
/// interface itself is chosen by the selector in `settings_body`).
pub(in crate::app) fn settings_hpsdr_tab(
    ui: &mut egui::Ui,
    devices: &[sdroxide_types::HpsdrDevice],
    radio_edit: &mut Option<sdroxide_types::RadioConfig>,
    discover: &mut bool,
    can_probe: bool,
    cmds: &mut Vec<Command>,
) {
    use sdroxide_types::HpsdrConfig;
    let Some(cfg) = radio_edit.as_mut() else {
        ui.label(tr!("settings-radio-waiting-configuration-machine-radio-attached"));
        return;
    };
    egui::Grid::new("hpsdr-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr!("settings-radio-devices"));
        // The scan goes out on this machine's LAN; the radio is on the
        // engine's. The manual IP below is typed, so it still works from here.
        probe_only(ui, can_probe, |ui| {
            ui.horizontal(|ui| {
                if ui.button(tr!("settings-radio-discover")).clicked() {
                    *discover = true;
                }
                let shown = cfg.hpsdr.selected_ip.clone().unwrap_or_else(|| "— none —".into());
                ComboBox::from_id_salt("hpsdr_dev").width(320.0).selected_text(shown).show_styled(
                    ui,
                    |ui| {
                        if devices.is_empty() {
                            ui.label(
                                RichText::new(tr!("settings-radio-no-devices-press-discover"))
                                    .weak(),
                            );
                        }
                        for d in devices {
                            // Both protocols are drivable; anything else is greyed out.
                            if d.supported() {
                                let sel = cfg.hpsdr.selected_ip.as_deref() == Some(d.ip.as_str());
                                if ui.selectable_label(sel, d.label()).clicked() {
                                    cfg.hpsdr.selected_ip = Some(d.ip.clone());
                                }
                            } else {
                                ui.label(RichText::new(d.label()).weak());
                            }
                        }
                    },
                );
            });
        });
        ui.end_row();

        ui.label(tr!("settings-radio-manual-ip"));
        let mut ip = cfg.hpsdr.manual_ip.clone().unwrap_or_default();
        let resp = crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut ip)
                .desired_width(160.0)
                .hint_text(tr!("settings-radio-optional-e-g-192-168")),
        );
        if resp.changed() {
            let t = ip.trim();
            cfg.hpsdr.manual_ip = if t.is_empty() { None } else { Some(t.to_string()) };
        }
        ui.end_row();

        ui.label(tr!("settings-radio-sample-rate"));
        // Show only rates valid for the selected device's protocol (P1 ≤ 384 kHz).
        let proto = devices
            .iter()
            .find(|d| Some(d.ip.as_str()) == cfg.hpsdr.selected_ip.as_deref())
            .map(|d| d.protocol)
            .unwrap_or(2);
        let shown = format!("{} kHz", (cfg.hpsdr.sample_rate_hz / 1000.0) as u32);
        ComboBox::from_id_salt("hpsdr_rate").selected_text(shown).show_styled(ui, |ui| {
            for &r in HpsdrConfig::rates_for(proto) {
                let sel = (cfg.hpsdr.sample_rate_hz - r).abs() < 1.0;
                if ui.selectable_label(sel, format!("{} kHz", (r / 1000.0) as u32)).clicked() {
                    cfg.hpsdr.sample_rate_hz = r;
                }
            }
        });
        ui.end_row();

        // Which of the board's DDCs this radio runs. Protocol 2 only — a P1
        // board refuses anything but the first at open, with a clear message.
        // Shown 1-based, stored 0-based as the wire counts.
        ui.label(tr!("settings-radio-receiver-ddc"));
        let shown = format!("DDC{}", cfg.hpsdr.ddc + 1);
        ComboBox::from_id_salt("hpsdr_ddc")
            .selected_text(shown)
            .show_styled(ui, |ui| {
                for ddc in 0u8..4 {
                    if ui
                        .selectable_label(cfg.hpsdr.ddc == ddc, format!("DDC{}", ddc + 1))
                        .clicked()
                    {
                        cfg.hpsdr.ddc = ddc;
                    }
                }
            })
            .response
            .on_hover_text(tr!("settings-radio-protocol-2-board-carries-several"));
        ui.end_row();

        ui.label(tr!("settings-radio-lna-gain"))
            .on_hover_text(tr!("settings-radio-front-end-gain-hermes-lite"));
        // Applies live as well as being persisted: this is the gain an operator
        // retunes per band, and making it wait for Apply/reconnect would mean
        // dropping the stream every time they nudge it.
        if crate::chrome::slider(
            ui,
            Slider::new(
                &mut cfg.hpsdr.lna_gain_db,
                HpsdrConfig::LNA_GAIN_MIN_DB..=HpsdrConfig::LNA_GAIN_MAX_DB,
            )
            .step_by(1.0)
            .suffix(" dB"),
        )
        .changed()
        {
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: sdroxide_types::HpsdrConfig::LNA_GAIN_ELEMENT.to_string(),
                db: cfg.hpsdr.lna_gain_db,
            });
        }
        ui.end_row();

        ui.label(tr!("settings-radio-overload-protection"))
            .on_hover_text(tr!("settings-radio-back-lna-gain-off-itself"));
        crate::chrome::checkbox(
            ui,
            &mut cfg.hpsdr.auto_gain,
            tr!("settings-radio-wind-gain-back-when-adc"),
        )
        .on_hover_text(tr!("settings-radio-applies-apply-reconnect-main-window"));
        ui.end_row();

        if cfg.hpsdr.auto_gain {
            ui.label(tr!("settings-radio-step"))
                .on_hover_text(tr!("settings-radio-how-far-gain-moves-each"));
            ui.add(
                egui::DragValue::new(&mut cfg.hpsdr.auto_gain_step_db)
                    .range(0.5..=12.0)
                    .speed(0.5)
                    .suffix(" dB"),
            );
            ui.end_row();

            ui.label(tr!("settings-radio-attack-decay"))
                .on_hover_text(tr!("settings-radio-how-often-gain-may-come"));
            ui.horizontal(|ui| {
                ui.add(
                    egui::DragValue::new(&mut cfg.hpsdr.auto_gain_attack_ms)
                        .range(20..=5_000)
                        .speed(10)
                        .suffix(" ms"),
                );
                ui.label("/");
                ui.add(
                    egui::DragValue::new(&mut cfg.hpsdr.auto_gain_decay_ms)
                        .range(100..=120_000)
                        .speed(100)
                        .suffix(" ms"),
                );
            });
            ui.end_row();

            ui.label(tr!("settings-radio-range"))
                .on_hover_text(tr!("settings-radio-lowest-highest-gain-loop-may"));
            ui.horizontal(|ui| {
                ui.add(
                    egui::DragValue::new(&mut cfg.hpsdr.auto_gain_min_db)
                        .range(HpsdrConfig::LNA_GAIN_MIN_DB..=HpsdrConfig::LNA_GAIN_MAX_DB)
                        .speed(1.0)
                        .suffix(" dB"),
                );
                ui.label("…");
                ui.add(
                    egui::DragValue::new(&mut cfg.hpsdr.auto_gain_max_db)
                        .range(HpsdrConfig::LNA_GAIN_MIN_DB..=HpsdrConfig::LNA_GAIN_MAX_DB)
                        .speed(1.0)
                        .suffix(" dB"),
                );
            });
            ui.end_row();
        }

        ui.label(tr!("settings-radio-filter-board"))
            .on_hover_text(tr!("settings-radio-accessory-board-hermes-lite-2"));
        ComboBox::from_id_salt("hpsdr_filter")
            .width(220.0)
            .selected_text(cfg.hpsdr.filter_board.tr())
            .show_styled(ui, |ui| {
                for b in sdroxide_types::HpsdrFilterBoard::ALL {
                    if ui.selectable_label(cfg.hpsdr.filter_board == b, b.tr()).clicked() {
                        cfg.hpsdr.filter_board = b;
                    }
                }
            });
        ui.end_row();

        ui.label(tr!("settings-radio-io-board-rx-input"))
            .on_hover_text(tr!("settings-radio-where-n2adr-hl2ioboard-takes-receive"));
        ComboBox::from_id_salt("hpsdr_io_rx")
            .width(220.0)
            .selected_text(cfg.hpsdr.io_rx_input.tr())
            .show_styled(ui, |ui| {
                for m in sdroxide_types::HpsdrIoRxInput::ALL {
                    if ui.selectable_label(cfg.hpsdr.io_rx_input == m, m.tr()).clicked() {
                        cfg.hpsdr.io_rx_input = m;
                    }
                }
            });
        ui.end_row();

        ui.label(tr!("settings-radio-power-amplifier"));
        crate::chrome::checkbox(
            ui,
            &mut cfg.hpsdr.pa_enable,
            tr!("settings-radio-use-hermes-lite-2-s"),
        )
        .on_hover_text(tr!("settings-radio-default-what-you-want-unless"));
        ui.end_row();

        ui.label(tr!("settings-radio-invert-spectrum"));
        crate::chrome::checkbox(ui, &mut cfg.hpsdr.invert_spectrum, tr!("settings-radio-swap-i-q"))
            .on_hover_text(tr!("settings-radio-mirror-board-s-spectrum-about"));
        ui.end_row();

        ui.label(tr!("settings-radio-frequency-correction"))
            .on_hover_text(tr!("settings-radio-crystal-tcxo-error-ppm-applied"));
        let mut ppm = cfg.hpsdr.ppm;
        let resp = ui.add(
            egui::DragValue::new(&mut ppm)
                .range(-100.0..=100.0)
                .speed(0.1)
                .suffix(tr!("settings-radio-ppm")),
        );
        if resp.changed() {
            cfg.hpsdr.ppm = ppm;
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: HpsdrConfig::PPM_ELEMENT.to_string(),
                db: ppm,
            });
        }
        ui.end_row();

        ui.label(tr!("settings-radio-transmit-buffer"));
        ui.add(
            egui::DragValue::new(&mut cfg.hpsdr.tx_latency_ms)
                .range(HpsdrConfig::TX_LATENCY_MS_RANGE)
                .suffix(" ms"),
        )
        .on_hover_text(tr!("settings-radio-how-far-ahead-real-time"));
        ui.end_row();

        // ── PureSignal ──
        ui.label(tr!("settings-radio-puresignal")).on_hover_text(tr!(
            "settings-radio-adaptive-predistortion-linearise-transmitter-sample"
        ));
        crate::chrome::checkbox(
            ui,
            &mut cfg.hpsdr.puresignal,
            tr!("settings-radio-correct-transmitter-receiver-s-own"),
        );
        ui.end_row();

        ui.add_enabled_ui(cfg.hpsdr.puresignal, |ui| {
            ui.label(tr!("settings-radio-table-steps"))
                .on_hover_text(tr!("settings-radio-how-finely-amplifier-s-curve"));
        });
        ui.add_enabled_ui(cfg.hpsdr.puresignal, |ui| {
            let mut bins = i32::from(cfg.hpsdr.ps_bins);
            if ui
                .add(egui::DragValue::new(&mut bins).range(
                    i32::from(sdroxide_types::LimeAuxConfig::PS_MIN_BINS)
                        ..=i32::from(sdroxide_types::LimeAuxConfig::PS_MAX_BINS),
                ))
                .changed()
            {
                cfg.hpsdr.ps_bins = bins as u8;
            }
        });
        ui.end_row();

        ui.add_enabled_ui(cfg.hpsdr.puresignal, |ui| {
            ui.label(tr!("settings-radio-adaptation"))
                .on_hover_text(tr!("settings-radio-how-fast-correction-follows-what"));
        });
        ui.add_enabled_ui(cfg.hpsdr.puresignal, |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::Slider::new(&mut cfg.hpsdr.ps_rate, 0.0..=1.0).show_value(false));
                crate::chrome::checkbox(ui, &mut cfg.hpsdr.ps_frozen, tr!("settings-radio-hold"))
                    .on_hover_text(tr!("settings-radio-stop-adapting-keep-correction-where"));
            });
        });
        ui.end_row();
    });
    hpsdr_oc_table(ui, cfg);
    ui.add_space(6.0);
    ui.label(RichText::new(tr!("settings-radio-manual-ip-overrides-discovery-press")).weak());
}

/// The per-band open-collector table, shown when **Filter board** is *Custom*
/// (issue #296).
///
/// Seven general-purpose outputs, two words per band — one asserted on receive,
/// one while keyed — which is what drives the filter boards, antenna switches,
/// band decoders and transverter sequencers that follow neither the N2ADR nor
/// the Alex convention. Written in hexadecimal because that is how the
/// hardware's documentation states a control word, with the bits spelled out
/// beside it so nobody has to convert in their head.
fn hpsdr_oc_table(ui: &mut egui::Ui, cfg: &mut sdroxide_types::RadioConfig) {
    use sdroxide_types::{Band, HpsdrFilterBoard, HpsdrOcRow};

    if cfg.hpsdr.filter_board != HpsdrFilterBoard::Custom {
        return;
    }
    ui.add_space(8.0);
    ui.separator();
    ui.add_space(4.0);
    ui.label(
        RichText::new(tr!("settings-radio-open-collector-outputs-band"))
            .size(14.0)
            .strong()
            .color(crate::theme::CYAN()),
    );
    ui.add_space(2.0);
    ui.label(RichText::new(tr!("settings-radio-one-control-word-per-band")).weak());
    ui.add_space(6.0);

    ui.horizontal(|ui| {
        for (preset, name) in [
            (HpsdrFilterBoard::N2adr, "N2ADR"),
            (HpsdrFilterBoard::Alex, &tr!("settings-radio-alex-band-code")),
        ] {
            if ui
                .button(tr!("settings-radio-fill-from", name = name.to_string()))
                .on_hover_text(tr!("settings-radio-replace-table-what-preset-would"))
                .clicked()
            {
                cfg.hpsdr.oc_table = HpsdrOcRow::from_preset(preset);
            }
        }
        if ui
            .add_enabled(
                !cfg.hpsdr.oc_table.is_empty(),
                egui::Button::new(tr!("settings-radio-clear")),
            )
            .on_hover_text(tr!("settings-radio-every-output-off-every-band"))
            .clicked()
        {
            cfg.hpsdr.oc_table.clear();
        }
    });
    ui.add_space(6.0);

    // Only the bands the station's own region has: a row for one the operator
    // can never reach is a row that can never do anything. The last row is
    // everything *outside* the ham bands, which is where a short-wave listener
    // sits — without it a custom table would leave them with no filter at all
    // where a preset gives them the nearest one.
    let bands: Vec<Band> =
        Band::ALL.iter().copied().filter(|b| *b == Band::Gen || b.edges().is_some()).collect();
    egui::Grid::new("hpsdr-oc-grid").num_columns(4).spacing([12.0, 4.0]).striped(true).show(
        ui,
        |ui| {
            for h in [
                tr!("settings-radio-band"),
                tr!("settings-radio-rx"),
                tr!("settings-radio-tx"),
                tr!("settings-radio-outputs-asserted"),
            ] {
                ui.label(RichText::new(h).weak().size(10.0));
            }
            ui.end_row();
            for band in bands {
                let i = cfg.hpsdr.oc_table.iter().position(|r| r.band == band);
                let (mut rx, mut tx) = i
                    .map(|i| (cfg.hpsdr.oc_table[i].rx, cfg.hpsdr.oc_table[i].tx))
                    .unwrap_or((0, 0));
                if band == Band::Gen {
                    ui.label(tr!("settings-radio-other")).on_hover_text(tr!(
                        "settings-radio-everywhere-outside-amateur-bands-short"
                    ));
                } else {
                    ui.label(band.label());
                }
                let a = oc_byte_edit(ui, ("oc-rx", band), &mut rx);
                let b = oc_byte_edit(ui, ("oc-tx", band), &mut tx);
                ui.label(RichText::new(oc_pins(rx, tx)).weak());
                if a || b {
                    match i {
                        Some(i) if rx == 0 && tx == 0 => {
                            cfg.hpsdr.oc_table.remove(i);
                        }
                        Some(i) => {
                            cfg.hpsdr.oc_table[i] = HpsdrOcRow { band, rx, tx };
                        }
                        None => cfg.hpsdr.oc_table.push(HpsdrOcRow { band, rx, tx }),
                    }
                }
                ui.end_row();
            }
        },
    );
}

/// One control word, typed and shown as two hexadecimal digits. Held to seven
/// bits, because there is no eighth output.
fn oc_byte_edit(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    word: &mut u8,
) -> bool {
    let mut v = *word as u16;
    let changed = ui
        .push_id(id, |ui| {
            ui.add(
                DragValue::new(&mut v)
                    .speed(1.0)
                    .range(0..=0x7F)
                    .hexadecimal(2, false, true)
                    .prefix("0x"),
            )
            .changed()
        })
        .inner;
    *word = (v & 0x7F) as u8;
    changed
}

/// "1, 3, 7" for the outputs a pair of words asserts — TX in parentheses when
/// it differs, so the two directions can be read at a glance.
fn oc_pins(rx: u8, tx: u8) -> String {
    let list = |w: u8| -> String {
        let pins: Vec<String> =
            (0..7).filter(|b| w & (1 << b) != 0).map(|b| (b + 1).to_string()).collect();
        if pins.is_empty() { "—".into() } else { pins.join(", ") }
    };
    if rx == tx {
        list(rx)
    } else {
        tr!("settings-radio-rx-tx-words", rx = list(rx), tx = list(tx))
    }
}

/// RTL-SDR interface: which dongle, sample rate, gain/AGC, frequency
/// correction, HF reception and the bias tee.
///
/// Gain, AGC, ppm and the bias tee all apply *live* rather than waiting for
/// Apply/reconnect — these are the controls an operator moves while listening,
/// and dropping the stream on every nudge would make them unusable. The dongle
/// selection and sample rate do need a reconnect, since both are fixed when
/// the device is opened.
pub(in crate::app) fn settings_rtlsdr_tab(
    ui: &mut egui::Ui,
    devices: &[sdroxide_types::RtlSdrDevice],
    radio_edit: &mut Option<sdroxide_types::RadioConfig>,
    rescan: &mut bool,
    can_probe: bool,
    cmds: &mut Vec<Command>,
) {
    use sdroxide_types::{RtlSdrAgc, RtlSdrConfig, RtlSdrHfMode};
    let Some(cfg) = radio_edit.as_mut() else {
        ui.label(tr!("settings-radio-waiting-configuration-machine-radio-attached"));
        return;
    };

    egui::Grid::new("rtlsdr-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr!("settings-radio-dongle"));
        // Which dongle is the one row here that names a USB bus rather than the
        // radio. Everything below reaches the dongle wherever it is plugged in.
        probe_only(ui, can_probe, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(tr!("settings-radio-rescan"))
                    .on_hover_text(tr!("settings-radio-re-list-usb-bus-no"))
                    .clicked()
                {
                    *rescan = true;
                }
                let shown =
                    cfg.rtlsdr.serial.clone().unwrap_or_else(|| "— first one found —".into());
                ComboBox::from_id_salt("rtlsdr_dev").width(300.0).selected_text(shown).show_styled(
                    ui,
                    |ui| {
                        if devices.is_empty() {
                            ui.label(
                                RichText::new(tr!("settings-radio-no-dongles-press-rescan")).weak(),
                            );
                        }
                        if ui
                            .selectable_label(
                                cfg.rtlsdr.serial.is_none(),
                                tr!("settings-radio-first-one-found"),
                            )
                            .clicked()
                        {
                            cfg.rtlsdr.serial = None;
                        }
                        for d in devices {
                            // Only a dongle with a serial can be pinned; without
                            // one there is nothing stable to remember, since bus
                            // position changes on every replug.
                            if let Some(sn) = &d.serial {
                                let sel = cfg.rtlsdr.serial.as_deref() == Some(sn.as_str());
                                if ui.selectable_label(sel, d.label()).clicked() {
                                    cfg.rtlsdr.serial = Some(sn.clone());
                                }
                            } else {
                                ui.label(RichText::new(d.label()).weak());
                            }
                        }
                    },
                );
            });
        });
        ui.end_row();

        ui.label(tr!("settings-radio-sample-rate"))
            .on_hover_text(tr!("settings-radio-rtl2832u-s-resampler-reaches-225"));
        let shown = format!("{:.3} Msps", cfg.rtlsdr.sample_rate_hz / 1e6);
        ComboBox::from_id_salt("rtlsdr_rate").selected_text(shown).show_styled(ui, |ui| {
            for &r in &RtlSdrConfig::SAMPLE_RATES {
                let sel = (cfg.rtlsdr.sample_rate_hz - r).abs() < 1.0;
                let mut label = format!("{:.3} Msps", r / 1e6);
                if r >= 3_200_000.0 {
                    label.push_str(&tr!("settings-radio-often-drops-samples"));
                }
                if ui.selectable_label(sel, label).clicked() {
                    cfg.rtlsdr.sample_rate_hz = r;
                }
            }
        });
        ui.end_row();

        ui.label(tr!("settings-radio-agc"))
            .on_hover_text(tr!("settings-radio-manual-setting-measurement-weak-signal"));
        let mut agc = cfg.rtlsdr.agc;
        enum_combo(ui, "rtlsdr_agc", &mut agc, &RtlSdrAgc::ALL, RtlSdrAgc::tr);
        if agc != cfg.rtlsdr.agc {
            cfg.rtlsdr.agc = agc;
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: RtlSdrConfig::AGC_ELEMENT.to_string(),
                db: agc.code() as f64,
            });
        }
        ui.end_row();

        ui.label(tr!("settings-radio-tuner-gain"))
            .on_hover_text(tr!("settings-radio-applies-immediately-no-reconnect-tuner"));
        ui.add_enabled_ui(!cfg.rtlsdr.agc.tuner_auto(), |ui| {
            if crate::chrome::slider(
                ui,
                Slider::new(&mut cfg.rtlsdr.tuner_gain_db, 0.0..=RtlSdrConfig::GAIN_MAX_DB)
                    .step_by(0.1)
                    .suffix(" dB"),
            )
            .changed()
            {
                cmds.push(Command::SetGain {
                    dir: Direction::Rx,
                    element: RtlSdrConfig::TUNER_GAIN_ELEMENT.to_string(),
                    db: cfg.rtlsdr.tuner_gain_db,
                });
            }
        });
        ui.end_row();

        ui.label(tr!("settings-radio-frequency-correction"))
            .on_hover_text(tr!("settings-radio-crystal-error-parts-per-million"));
        let mut ppm = cfg.rtlsdr.ppm;
        if ui
            .add(egui::DragValue::new(&mut ppm).range(-200..=200).suffix(tr!("settings-radio-ppm")))
            .changed()
        {
            cfg.rtlsdr.ppm = ppm;
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: RtlSdrConfig::PPM_ELEMENT.to_string(),
                db: ppm as f64,
            });
        }
        ui.end_row();

        ui.label(tr!("settings-radio-hf-reception"))
            .on_hover_text(tr!("settings-radio-tuner-itself-starts-24-mhz"));
        let mut hf = cfg.rtlsdr.hf_mode;
        enum_combo(ui, "rtlsdr_hf", &mut hf, &RtlSdrHfMode::ALL, RtlSdrHfMode::tr);
        if hf != cfg.rtlsdr.hf_mode {
            cfg.rtlsdr.hf_mode = hf;
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: RtlSdrConfig::HF_MODE_ELEMENT.to_string(),
                db: hf as u8 as f64,
            });
        }
        ui.end_row();

        ui.label(tr!("settings-radio-iq-correction"))
            .on_hover_text(tr!("settings-radio-removes-dongle-s-own-dc"));
        let mut iq = cfg.rtlsdr.iq_correction;
        if crate::chrome::checkbox(
            ui,
            &mut iq,
            tr!("settings-radio-remove-centre-spike-mirror-image"),
        )
        .changed()
        {
            cfg.rtlsdr.iq_correction = iq;
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: RtlSdrConfig::IQ_CORRECTION_ELEMENT.to_string(),
                db: if iq { 1.0 } else { 0.0 },
            });
        }
        ui.end_row();

        ui.label(tr!("settings-radio-bias-tee"));
        let mut bias = cfg.rtlsdr.bias_tee;
        if crate::chrome::checkbox(ui, &mut bias, tr!("settings-radio-feed-4-5-v-dc")).changed() {
            cfg.rtlsdr.bias_tee = bias;
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: RtlSdrConfig::BIAS_TEE_ELEMENT.to_string(),
                db: if bias { 1.0 } else { 0.0 },
            });
        }
        ui.end_row();
    });

    if cfg.rtlsdr.bias_tee {
        ui.add_space(4.0);
        ui.label(
            RichText::new(tr!("settings-radio-bias-tee-never-connect-transceiver"))
                .color(crate::theme::YELLOW()),
        );
    }

    ui.add_space(4.0);
    ui.label(RichText::new(tr!("settings-radio-receive-only-dongle-sample-rate")).weak());
}

/// rtl_tcp interface: the same dongle as the tab above, on another machine.
///
/// Deliberately the same controls in the same order — an operator who moves a
/// dongle from this machine to a Raspberry Pi on the mast should not have to
/// learn a second panel. What differs is at the top (an address, not a USB
/// serial) and in the hover text, which has to say *whose* hardware each knob
/// reaches: everything here is performed by the server, and nothing it does
/// with the request is ever reported back.
pub(in crate::app) fn settings_rtltcp_tab(
    ui: &mut egui::Ui,
    radio_edit: &mut Option<sdroxide_types::RadioConfig>,
    cmds: &mut Vec<Command>,
) {
    use sdroxide_types::{RtlSdrAgc, RtlSdrConfig, RtlSdrHfMode, RtlTcpConfig};
    let Some(cfg) = radio_edit.as_mut() else {
        ui.label(tr!("settings-radio-waiting-configuration-machine-radio-attached"));
        return;
    };

    egui::Grid::new("rtltcp-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr!("settings-radio-server-address"))
            .on_hover_text(tr!("settings-radio-where-rtl-tcp-listening-address"));
        crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut cfg.rtltcp.address)
                .desired_width(220.0)
                .hint_text(tr!("settings-radio-host-host-port-e-g")),
        );
        ui.end_row();

        ui.label(tr!("settings-radio-sample-rate"))
            .on_hover_text(tr!("settings-radio-requested-server-same-resampler-limits"));
        let shown = format!(
            "{:.3} Msps  —  {:.0} Mbit/s",
            cfg.rtltcp.sample_rate_hz / 1e6,
            RtlTcpConfig::link_mbit(cfg.rtltcp.sample_rate_hz),
        );
        ComboBox::from_id_salt("rtltcp_rate").width(260.0).selected_text(shown).show_styled(
            ui,
            |ui| {
                for &r in &RtlSdrConfig::SAMPLE_RATES {
                    let sel = (cfg.rtltcp.sample_rate_hz - r).abs() < 1.0;
                    let mbit = RtlTcpConfig::link_mbit(r);
                    let mut label = format!("{:.3} Msps  —  {mbit:.0} Mbit/s", r / 1e6);
                    // The threshold is where a rate stops fitting comfortably in
                    // what a single WiFi hop delivers in practice, which is well
                    // under its nominal rate.
                    if mbit >= 30.0 {
                        label.push_str(&tr!("settings-radio-wired-link"));
                    }
                    if ui.selectable_label(sel, label).clicked() {
                        cfg.rtltcp.sample_rate_hz = r;
                    }
                }
            },
        );
        ui.end_row();

        ui.label(tr!("settings-radio-agc"))
            .on_hover_text(tr!("settings-radio-runs-server-s-dongle-manual"));
        let mut agc = cfg.rtltcp.agc;
        enum_combo(ui, "rtltcp_agc", &mut agc, &RtlSdrAgc::ALL, RtlSdrAgc::tr);
        if agc != cfg.rtltcp.agc {
            cfg.rtltcp.agc = agc;
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: RtlSdrConfig::AGC_ELEMENT.to_string(),
                db: agc.code() as f64,
            });
        }
        ui.end_row();

        ui.label(tr!("settings-radio-tuner-gain"))
            .on_hover_text(tr!("settings-radio-applies-immediately-no-reconnect-sent"));
        ui.add_enabled_ui(!cfg.rtltcp.agc.tuner_auto(), |ui| {
            if crate::chrome::slider(
                ui,
                Slider::new(&mut cfg.rtltcp.tuner_gain_db, 0.0..=RtlSdrConfig::GAIN_MAX_DB)
                    .step_by(0.1)
                    .suffix(" dB"),
            )
            .changed()
            {
                cmds.push(Command::SetGain {
                    dir: Direction::Rx,
                    element: RtlSdrConfig::TUNER_GAIN_ELEMENT.to_string(),
                    db: cfg.rtltcp.tuner_gain_db,
                });
            }
        });
        ui.end_row();

        ui.label(tr!("settings-radio-frequency-correction"))
            .on_hover_text(tr!("settings-radio-crystal-error-server-s-dongle"));
        let mut ppm = cfg.rtltcp.ppm;
        if ui
            .add(egui::DragValue::new(&mut ppm).range(-200..=200).suffix(tr!("settings-radio-ppm")))
            .changed()
        {
            cfg.rtltcp.ppm = ppm;
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: RtlSdrConfig::PPM_ELEMENT.to_string(),
                db: ppm as f64,
            });
        }
        ui.end_row();

        ui.label(tr!("settings-radio-hf-reception"))
            .on_hover_text(tr!("settings-radio-tuner-starts-24-mhz-below"));
        let mut hf = cfg.rtltcp.hf_mode;
        enum_combo(ui, "rtltcp_hf", &mut hf, &RtlSdrHfMode::ALL, RtlSdrHfMode::tr);
        if hf != cfg.rtltcp.hf_mode {
            cfg.rtltcp.hf_mode = hf;
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: RtlSdrConfig::HF_MODE_ELEMENT.to_string(),
                db: hf as u8 as f64,
            });
        }
        ui.end_row();

        ui.label(tr!("settings-radio-iq-correction"))
            .on_hover_text(tr!("settings-radio-removes-dongle-s-dc-spike"));
        let mut iq = cfg.rtltcp.iq_correction;
        if crate::chrome::checkbox(
            ui,
            &mut iq,
            tr!("settings-radio-remove-centre-spike-mirror-image"),
        )
        .changed()
        {
            cfg.rtltcp.iq_correction = iq;
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: RtlSdrConfig::IQ_CORRECTION_ELEMENT.to_string(),
                db: if iq { 1.0 } else { 0.0 },
            });
        }
        ui.end_row();

        ui.label(tr!("settings-radio-bias-tee"))
            .on_hover_text(tr!("settings-radio-powers-preamp-far-end-s"));
        let mut bias = cfg.rtltcp.bias_tee;
        if crate::chrome::checkbox(ui, &mut bias, tr!("settings-radio-feed-4-5-v-dc-2")).changed() {
            cfg.rtltcp.bias_tee = bias;
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: RtlSdrConfig::BIAS_TEE_ELEMENT.to_string(),
                db: if bias { 1.0 } else { 0.0 },
            });
        }
        ui.end_row();
    });

    if cfg.rtltcp.bias_tee {
        ui.add_space(4.0);
        ui.label(
            RichText::new(tr!("settings-radio-bias-tee-hardware-somewhere-else"))
                .color(crate::theme::YELLOW()),
        );
    }

    ui.add_space(8.0);
    ui.separator();
    ui.label(RichText::new(tr!("settings-radio-sdrplay-server-rsp-tcp")).strong());
    ui.label(RichText::new(tr!("settings-radio-sdrplay-server-greets-exactly-like")).weak());
    ui.add_space(4.0);

    egui::Grid::new("rsptcp-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr!("settings-radio-antenna"));
        ui.horizontal(|ui| {
            for (v, name) in [
                (0u8, tr!("settings-radio-input")),
                (1, tr!("settings-radio-input-b")),
                (2, tr!("settings-radio-hi-z")),
            ] {
                if ui.selectable_label(cfg.rtltcp.rsp_antenna == v, name).clicked() {
                    cfg.rtltcp.rsp_antenna = v;
                    push_gain(cmds, RtlTcpConfig::RSP_ANTENNA_ELEMENT, v as f64);
                }
            }
        });
        ui.end_row();

        ui.label(tr!("settings-radio-lna-state"));
        if crate::chrome::slider(ui, egui::Slider::new(&mut cfg.rtltcp.rsp_lna_state, 0..=9))
            .on_hover_text(tr!("settings-radio-step-index-not-db-figure"))
            .changed()
        {
            push_gain(cmds, RtlTcpConfig::RSP_LNA_STATE_ELEMENT, cfg.rtltcp.rsp_lna_state as f64);
        }
        ui.end_row();

        ui.label(tr!("settings-radio-if-gain-reduction"));
        if crate::chrome::slider(
            ui,
            egui::Slider::new(&mut cfg.rtltcp.rsp_if_gain_reduction, 20..=59).suffix(" dB"),
        )
        .on_hover_text(tr!("settings-radio-reduction-so-more-less-signal"))
        .changed()
        {
            push_gain(
                cmds,
                RtlTcpConfig::RSP_IFGR_ELEMENT,
                cfg.rtltcp.rsp_if_gain_reduction as f64,
            );
        }
        ui.end_row();

        ui.label(tr!("settings-radio-agc"));
        ui.horizontal(|ui| {
            if crate::chrome::checkbox(ui, &mut cfg.rtltcp.rsp_agc, tr!("settings-radio-enable"))
                .changed()
            {
                push_gain(cmds, RtlTcpConfig::RSP_AGC_ELEMENT, cfg.rtltcp.rsp_agc as u8 as f64);
            }
            ui.add_enabled_ui(cfg.rtltcp.rsp_agc, |ui| {
                if ui
                    .add(
                        egui::DragValue::new(&mut cfg.rtltcp.rsp_agc_setpoint)
                            .range(-72..=0)
                            .suffix(tr!("settings-radio-dbfs")),
                    )
                    .on_hover_text(tr!("settings-radio-level-rsp-s-agc-aims"))
                    .changed()
                {
                    push_gain(
                        cmds,
                        RtlTcpConfig::RSP_AGC_SETPOINT_ELEMENT,
                        cfg.rtltcp.rsp_agc_setpoint as f64,
                    );
                }
            });
        });
        ui.end_row();

        ui.label(tr!("settings-radio-notches"));
        ui.horizontal(|ui| {
            let mut mask = cfg.rtltcp.rsp_notch;
            for (bit, name) in [
                (RtlTcpConfig::RSP_NOTCH_AM, tr!("settings-radio-am")),
                (RtlTcpConfig::RSP_NOTCH_BROADCAST, tr!("settings-radio-fm")),
                (RtlTcpConfig::RSP_NOTCH_DAB, tr!("settings-radio-dab")),
                (RtlTcpConfig::RSP_NOTCH_RF, tr!("settings-radio-rf")),
            ] {
                let mut on = mask & bit != 0;
                if crate::chrome::checkbox(ui, &mut on, name).changed() {
                    mask = if on { mask | bit } else { mask & !bit };
                }
            }
            if mask != cfg.rtltcp.rsp_notch {
                cfg.rtltcp.rsp_notch = mask;
                push_gain(cmds, RtlTcpConfig::RSP_NOTCH_ELEMENT, mask as f64);
            }
        });
        ui.end_row();

        ui.label(tr!("settings-radio-reference-out"));
        if ui
            .checkbox(&mut cfg.rtltcp.rsp_ref_out, tr!("settings-radio-t-24-mhz-clock-out"))
            .on_hover_text(tr!("settings-radio-rsp2-rspduo-only"))
            .changed()
        {
            push_gain(cmds, RtlTcpConfig::RSP_REF_OUT_ELEMENT, cfg.rtltcp.rsp_ref_out as u8 as f64);
        }
        ui.end_row();
    });

    ui.add_space(4.0);
    ui.label(RichText::new(tr!("settings-radio-receive-only-address-sample-rate")).weak());
}

/// TCI interface: WebSocket server address, IQ sample rate, and a
/// Test-connection button (the interface is chosen by the selector in
/// `settings_body`).
/// Settings → Radio for a SpyServer, in either of the two interfaces that
/// reach one.
///
/// One function for both, because the server, the handshake and every control
/// below are the same; `vfo` only changes which config block is edited and
/// what the explanatory text says the interface is for.
///
/// No device list and no rate list in Hz. This protocol publishes the receiver
/// behind it — its ladder, its tuning range, its gain stages — but only once a
/// connection is open, and this screen may be running in a browser on the far
/// side of the world from the machine that will do the connecting. So what is
/// stored is the *decimation stage*, which is what the wire carries and what
/// stays meaningful when the same settings are pointed at another server, and
/// the Test button is how an operator finds out what the stages mean here.
pub(in crate::app) fn settings_spyserver_tab(
    ui: &mut egui::Ui,
    radio_edit: &mut Option<sdroxide_types::RadioConfig>,
    vfo: bool,
    test: &mut bool,
    test_result: &Option<crate::app::settings::TestOutcome>,
    can_probe: bool,
    cmds: &mut Vec<Command>,
) {
    use sdroxide_types::{SpyServerConfig, SpyServerFormat};
    let Some(radio) = radio_edit.as_mut() else {
        ui.label(tr!("settings-radio-waiting-configuration-machine-radio-attached"));
        return;
    };
    let cfg = if vfo { &mut radio.spyserver_vfo } else { &mut radio.spyserver };
    let salt = if vfo { "spyservervfo" } else { "spyserver" };

    egui::Grid::new(format!("{salt}-grid")).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr!("settings-radio-server-address"))
            .on_hover_text(tr!("settings-radio-where-spyserver-listening-address-address"));
        crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut cfg.address)
                .desired_width(220.0)
                .hint_text(tr!("settings-radio-host-host-port-e-g-2")),
        );
        ui.end_row();

        ui.label(tr!("settings-radio-i-q-bandwidth")).on_hover_text(if vfo {
            tr!("settings-radio-how-wide-slice-band-arrives")
        } else {
            tr!("settings-radio-which-stage-server-s-own")
        });
        let shown = if cfg.iq_decimation < 0 {
            let target = if vfo {
                SpyServerConfig::VFO_TARGET_RATE_HZ
            } else {
                SpyServerConfig::WIDEBAND_TARGET_RATE_HZ
            };
            tr!("settings-radio-decimation-auto", khz = format!("{:.0}", target / 1e3))
        } else {
            tr!(
                "settings-radio-decimation-stage-server",
                stage = cfg.iq_decimation,
                divisor = 1u32 << cfg.iq_decimation.min(20)
            )
        };
        ComboBox::from_id_salt(format!("{salt}_decim"))
            .width(260.0)
            .selected_text(shown)
            .show_styled(ui, |ui| {
                if ui
                    .selectable_label(cfg.iq_decimation < 0, tr!("settings-radio-automatic"))
                    .clicked()
                {
                    cfg.iq_decimation = SpyServerConfig::AUTO_DECIMATION;
                }
                // The ladder's real depth is the server's, and it is not known
                // here. Sixteen stages is past what any of them offer; a stage
                // this server has not got falls back to the nearest it has, and
                // says so in the log.
                for stage in 0..16i32 {
                    let sel = cfg.iq_decimation == stage;
                    let label = tr!(
                        "settings-radio-decimation-stage",
                        stage = stage,
                        divisor = 1u32 << stage.min(20)
                    );
                    if ui.selectable_label(sel, label).clicked() {
                        cfg.iq_decimation = stage;
                    }
                }
            });
        ui.end_row();

        ui.label(tr!("settings-radio-sample-format"))
            .on_hover_text(tr!("settings-radio-bits-per-component-wire-so"));
        let mut fmt = cfg.iq_format;
        enum_combo(
            ui,
            &format!("{salt}_fmt"),
            &mut fmt,
            &SpyServerFormat::ALL,
            SpyServerFormat::tr,
        );
        cfg.iq_format = fmt;
        ui.end_row();

        ui.label(tr!("settings-radio-gain"))
            .on_hover_text(tr!("settings-radio-server-s-gain-stage-index"));
        let mut gain = cfg.gain_index as i32;
        if ui
            .add(egui::DragValue::new(&mut gain).range(0..=45).prefix(tr!("settings-radio-index")))
            .changed()
        {
            cfg.gain_index = gain.max(0) as u32;
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: SpyServerConfig::GAIN_ELEMENT.to_string(),
                db: f64::from(cfg.gain_index),
            });
        }
        ui.end_row();

        ui.label(tr!("settings-radio-digital-gain"))
            .on_hover_text(tr!("settings-radio-how-far-server-scales-samples"));
        ui.horizontal(|ui| {
            let mut auto = cfg.auto_digital_gain;
            if crate::chrome::checkbox(ui, &mut auto, tr!("settings-radio-automatic")).changed() {
                cfg.auto_digital_gain = auto;
                cmds.push(Command::SetGain {
                    dir: Direction::Rx,
                    element: SpyServerConfig::AUTO_DIGITAL_GAIN_ELEMENT.to_string(),
                    db: f64::from(u8::from(auto)),
                });
            }
            ui.add_enabled_ui(!cfg.auto_digital_gain, |ui| {
                if ui
                    .add(egui::DragValue::new(&mut cfg.digital_gain_db).range(0..=60).suffix(" dB"))
                    .changed()
                {
                    cmds.push(Command::SetGain {
                        dir: Direction::Rx,
                        element: SpyServerConfig::DIGITAL_GAIN_ELEMENT.to_string(),
                        db: cfg.digital_gain_db,
                    });
                }
            });
        });
        ui.end_row();

        ui.label(tr!("settings-radio-full-band-strip")).on_hover_text(if vfo {
            tr!("settings-radio-server-s-own-fft-whole")
        } else {
            tr!("settings-radio-ask-server-low-rate-fft")
        });
        ui.horizontal(|ui| {
            let mut on = cfg.fft_enabled;
            if crate::chrome::checkbox(ui, &mut on, tr!("settings-radio-show")).changed() {
                cfg.fft_enabled = on;
                cmds.push(Command::SetGain {
                    dir: Direction::Rx,
                    element: SpyServerConfig::FFT_ENABLED_ELEMENT.to_string(),
                    db: f64::from(u8::from(on)),
                });
            }
            ui.add_enabled_ui(cfg.fft_enabled, |ui| {
                let shown = if cfg.fft_decimation == 0 {
                    tr!("settings-radio-whole-band")
                } else {
                    format!("÷ {}", 1u32 << cfg.fft_decimation.min(20))
                };
                ComboBox::from_id_salt(format!("{salt}_fftdec"))
                    .width(130.0)
                    .selected_text(shown)
                    .show_styled(ui, |ui| {
                        for stage in 0..8u32 {
                            let sel = cfg.fft_decimation == stage;
                            let label = if stage == 0 {
                                tr!("settings-radio-whole-band")
                            } else {
                                format!("÷ {}", 1u32 << stage)
                            };
                            if ui.selectable_label(sel, label).clicked() {
                                cfg.fft_decimation = stage;
                                cmds.push(Command::SetGain {
                                    dir: Direction::Rx,
                                    element: SpyServerConfig::FFT_DECIMATION_ELEMENT.to_string(),
                                    db: f64::from(stage),
                                });
                            }
                        }
                    })
                    .response
                    .on_hover_text(tr!("settings-radio-how-much-receiver-strip-covers"));
            });
        });
        ui.end_row();

        ui.label(tr!("settings-radio-strip-db-window"))
            .on_hover_text(tr!("settings-radio-server-quantises-fft-into-window"));
        ui.horizontal(|ui| {
            ui.add_enabled_ui(cfg.fft_enabled, |ui| {
                if ui
                    .add(
                        egui::DragValue::new(&mut cfg.fft_db_offset)
                            .range(
                                SpyServerConfig::FFT_DB_OFFSET_MIN
                                    ..=SpyServerConfig::FFT_DB_OFFSET_MAX,
                            )
                            .prefix(tr!("settings-radio-top"))
                            .suffix(" dB"),
                    )
                    .changed()
                {
                    cmds.push(Command::SetGain {
                        dir: Direction::Rx,
                        element: SpyServerConfig::FFT_DB_OFFSET_ELEMENT.to_string(),
                        db: cfg.fft_db_offset,
                    });
                }
                if ui
                    .add(
                        egui::DragValue::new(&mut cfg.fft_db_range)
                            .range(
                                SpyServerConfig::FFT_DB_RANGE_MIN
                                    ..=SpyServerConfig::FFT_DB_RANGE_MAX,
                            )
                            .prefix(tr!("settings-radio-range-2"))
                            .suffix(" dB"),
                    )
                    .changed()
                {
                    cmds.push(Command::SetGain {
                        dir: Direction::Rx,
                        element: SpyServerConfig::FFT_DB_RANGE_ELEMENT.to_string(),
                        db: cfg.fft_db_range,
                    });
                }
            });
        });
        ui.end_row();

        ui.label(tr!("settings-radio-i-q-correction"))
            .on_hover_text(tr!("settings-radio-remove-dc-spike-mirror-image"));
        let mut iq = cfg.iq_correction;
        if crate::chrome::checkbox(ui, &mut iq, tr!("settings-radio-enabled")).changed() {
            cfg.iq_correction = iq;
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: SpyServerConfig::IQ_CORRECTION_ELEMENT.to_string(),
                db: f64::from(u8::from(iq)),
            });
        }
        ui.end_row();

        ui.label("");
        // The test connects from wherever it is pressed, so a green answer in a
        // browser would only say *this screen* can reach the server — a
        // different question from the one being asked.
        probe_only(ui, can_probe, |ui| {
            if ui
                .button(tr!("settings-radio-test-connection"))
                .on_hover_text(tr!("settings-radio-connect-read-what-server-says"))
                .clicked()
            {
                *test = true;
            }
        });
        ui.end_row();
    });
    test_result_line(ui, test_result);
    ui.add_space(6.0);
    ui.label(
        RichText::new(if vfo {
            tr!("settings-radio-receive-only-panadapter-narrow-i")
        } else {
            tr!("settings-radio-receive-only-wideband-i-q")
        })
        .weak(),
    );
}

/// The KiwiSDR / Web-888 interface.
///
/// Short, because there is very little to decide: the receiver owns the front
/// end and the rate, and what is left is where it is, who to say we are, and
/// how much of the link to spend on the band view.
pub(in crate::app) fn settings_kiwisdr_tab(
    ui: &mut egui::Ui,
    radio_edit: &mut Option<sdroxide_types::RadioConfig>,
    test: &mut bool,
    test_result: &Option<crate::app::settings::TestOutcome>,
    can_probe: bool,
    cmds: &mut Vec<Command>,
) {
    use sdroxide_types::KiwiConfig;
    let Some(radio) = radio_edit.as_mut() else {
        ui.label(tr!("settings-radio-waiting-configuration-machine-radio-attached"));
        return;
    };
    let cfg = &mut radio.kiwi;

    egui::Grid::new("kiwi-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr!("settings-radio-receiver-address"))
            .on_hover_text(tr!("settings-radio-where-receiver-serves-web-page"));
        crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut cfg.address)
                .desired_width(240.0)
                .hint_text(tr!("settings-radio-host-host-port-e-g-3")),
        );
        ui.end_row();

        ui.label(tr!("settings-radio-password"))
            .on_hover_text(tr!("settings-radio-receiver-s-user-password-where"));
        crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut cfg.password).password(true).desired_width(160.0),
        );
        ui.end_row();

        ui.label(tr!("settings-radio-announce"))
            .on_hover_text(tr!("settings-radio-name-connection-shows-up-under"));
        crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut cfg.ident)
                .desired_width(160.0)
                .hint_text(tr!("settings-radio-blank-station-callsign")),
        );
        ui.end_row();

        ui.label(tr!("settings-radio-band-view"))
            .on_hover_text(tr!("settings-radio-ask-receiver-own-waterfall-well"));
        let mut wide = cfg.wide_lane;
        crate::chrome::checkbox(ui, &mut wide, tr!("settings-radio-show-receiver-s-0-30"));
        cfg.wide_lane = wide;
        ui.end_row();

        ui.label(tr!("settings-radio-band-view-speed"))
            .on_hover_text(tr!("settings-radio-how-often-receiver-sends-waterfall"));
        let mut speed = cfg.wf_speed.clamp(KiwiConfig::WF_SPEED_MIN, KiwiConfig::WF_SPEED_MAX);
        if ui
            .add_enabled(
                cfg.wide_lane,
                egui::Slider::new(&mut speed, KiwiConfig::WF_SPEED_MIN..=KiwiConfig::WF_SPEED_MAX),
            )
            .changed()
        {
            cfg.wf_speed = speed;
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: KiwiConfig::WF_SPEED_ELEMENT.to_string(),
                db: f64::from(speed),
            });
        }
        ui.end_row();

        ui.label(tr!("settings-radio-band-view-span"))
            .on_hover_text(tr!("settings-radio-how-much-band-receiver-s"));
        let mut zoom = cfg.wf_zoom.min(KiwiConfig::WF_ZOOM_MAX);
        let span_label = |z: u8, bw: f64| {
            let hz = bw / f64::from(1u32 << z);
            if hz >= 1e6 { format!("{:.1} MHz", hz / 1e6) } else { format!("{:.0} kHz", hz / 1e3) }
        };
        if ui
            .add_enabled(
                cfg.wide_lane,
                egui::Slider::new(&mut zoom, 0..=KiwiConfig::WF_ZOOM_MAX)
                    // The receiver's own band is not known until it answers, so
                    // the label is against the commonest one: a 30 MHz Kiwi.
                    .custom_formatter(|z, _| span_label(z as u8, 30e6)),
            )
            .changed()
        {
            cfg.wf_zoom = zoom;
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: KiwiConfig::WF_ZOOM_ELEMENT.to_string(),
                db: f64::from(zoom),
            });
        }
        ui.end_row();

        ui.label(tr!("settings-radio-receiver-agc"))
            .on_hover_text(tr!("settings-radio-receiver-s-agc-which-far"));
        let mut agc = cfg.agc;
        if crate::chrome::checkbox(ui, &mut agc, tr!("settings-radio-let-receiver-ride-own-gain"))
            .changed()
        {
            cfg.agc = agc;
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: KiwiConfig::AGC_ELEMENT.to_string(),
                db: f64::from(u8::from(agc)),
            });
        }
        ui.end_row();

        ui.label(tr!("settings-radio-manual-gain"))
            .on_hover_text(tr!("settings-radio-fixed-gain-when-receiver-s"));
        let mut gain = cfg.man_gain;
        if ui.add_enabled(!cfg.agc, egui::Slider::new(&mut gain, 0..=90)).changed() {
            cfg.man_gain = gain;
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: KiwiConfig::MAN_GAIN_ELEMENT.to_string(),
                db: f64::from(gain),
            });
        }
        ui.end_row();

        ui.label("");
        // The test reads the receiver's `/status` page from wherever the radio
        // is, and deliberately does not open a session: a receiver has only
        // four or eight channels and taking one to ask a question would, on a
        // busy receiver, be the reason it was full.
        probe_only(ui, can_probe, |ui| {
            if ui
                .button(tr!("settings-radio-test-connection"))
                .on_hover_text(tr!("settings-radio-read-receiver-s-status-page"))
                .clicked()
            {
                *test = true;
            }
        });
        ui.end_row();
    });
    test_result_line(ui, test_result);
    ui.add_space(6.0);
    ui.label(
        RichText::new(tr!(
            "settings-radio-kiwi-receive-only",
            rate = format!("{:.0}", cfg.link_kbytes_s())
        ))
        .weak(),
    );
}

pub(in crate::app) fn settings_tci_tab(
    ui: &mut egui::Ui,
    radio_edit: &mut Option<sdroxide_types::RadioConfig>,
    tci_test: &mut bool,
    test_result: &Option<crate::app::settings::TestOutcome>,
    can_probe: bool,
) {
    use sdroxide_types::TciConfig;
    let Some(cfg) = radio_edit.as_mut() else {
        ui.label(tr!("settings-radio-waiting-configuration-machine-radio-attached"));
        return;
    };
    egui::Grid::new("tci-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr!("settings-radio-server-address"));
        crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut cfg.tci.address)
                .desired_width(220.0)
                .hint_text(tr!("settings-radio-host-port-e-g-127")),
        );
        ui.end_row();

        ui.label(tr!("settings-radio-iq-sample-rate"));
        let shown = format!("{} kHz", (cfg.tci.iq_sample_rate_hz / 1000.0) as u32);
        ComboBox::from_id_salt("tci_rate").selected_text(shown).show_styled(ui, |ui| {
            for &r in &TciConfig::IQ_RATES {
                let sel = (cfg.tci.iq_sample_rate_hz - r).abs() < 1.0;
                if ui.selectable_label(sel, format!("{} kHz", (r / 1000.0) as u32)).clicked() {
                    cfg.tci.iq_sample_rate_hz = r;
                }
            }
        });
        ui.end_row();

        // Which of the rig's receivers this radio runs. Offered as the two a
        // SunSDR2DX has; the rig reports its real count when the connection
        // opens, and asking for one it doesn't have is refused with that
        // count. Shown 1-based, stored 0-based as the wire counts.
        ui.label(tr!("settings-radio-receiver"));
        let shown = format!("RX{}", cfg.tci.rx + 1);
        ComboBox::from_id_salt("tci_rx")
            .selected_text(shown)
            .show_styled(ui, |ui| {
                for rx in 0u32..2 {
                    if ui.selectable_label(cfg.tci.rx == rx, format!("RX{}", rx + 1)).clicked() {
                        cfg.tci.rx = rx;
                    }
                }
            })
            .response
            .on_hover_text(tr!("settings-radio-rig-two-receivers-sunsdr2dx-can"));
        ui.end_row();

        // How far the rig's IQ runs behind the `dds:` that moved it. The
        // panadapter labels each frame with the centre its *samples* were taken
        // at, and this is the only number that says which centre that was — so
        // it belongs to the rig, not to sdroxide, and the default is a
        // measurement of one particular rig rather than a constant.
        ui.label(tr!("settings-radio-stream-delay"));
        crate::chrome::slider(
            ui,
            egui::Slider::new(&mut cfg.tci.stream_delay_ms, 0.0..=400.0).suffix(" ms").step_by(1.0),
        )
        .on_hover_text(tr!("settings-radio-how-long-rig-s-iq"));
        ui.end_row();

        ui.label("");
        // The test opens its own socket from wherever it is pressed, so a
        // green answer here would only say this screen can reach the rig — a
        // different question from the one being asked.
        probe_only(ui, can_probe, |ui| {
            if ui.button(tr!("settings-radio-test-connection")).clicked() {
                *tci_test = true;
            }
        });
        ui.end_row();
    });
    test_result_line(ui, test_result);
    ui.add_space(6.0);
    ui.label(RichText::new(tr!("settings-radio-wideband-iq-receive-audio-transmit")).weak());
}

/// Settings → Radio for an Icom on its LAN or WiFi port.
///
/// No Discover button: an Icom does not announce itself on the network, so the
/// address is always typed in.
pub(in crate::app) fn settings_icomnet_tab(
    ui: &mut egui::Ui,
    radio_edit: &mut Option<sdroxide_types::RadioConfig>,
    caps: Option<&sdroxide_types::DeviceCaps>,
    antenna_rx: &str,
    rx_antenna: bool,
    test: &mut bool,
    copy_report: &mut bool,
    test_result: &Option<crate::app::settings::TestOutcome>,
    can_probe: bool,
    cmds: &mut Vec<Command>,
) {
    use sdroxide_types::{CwKeying, IcomNetConfig, IcomRxSource, IcomScopeSpan};
    let Some(cfg) = radio_edit.as_mut() else {
        ui.label(tr!("settings-radio-waiting-configuration-machine-radio-attached"));
        return;
    };
    let net = &mut cfg.icomnet;

    egui::Grid::new("icomnet-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr!("settings-radio-radio-address"))
            .on_hover_text(tr!("settings-radio-address-shown-radio-under-set"));
        crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut net.address)
                .desired_width(220.0)
                .hint_text(tr!("settings-radio-host-ip-e-g-192")),
        );
        ui.end_row();

        ui.label(tr!("settings-radio-control-port"));
        ui.add(egui::DragValue::new(&mut net.control_port).range(1..=65535))
            .on_hover_text(tr!("settings-radio-t-50001-unless-has-been-changed"));
        ui.end_row();

        ui.label(tr!("settings-radio-network-user"));
        crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut net.username).desired_width(220.0),
        );
        ui.end_row();

        ui.label(tr!("settings-radio-password"))
            .on_hover_text(tr!("settings-radio-stored-clear-radio-json-protocol"));
        crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut net.password).password(true).desired_width(220.0),
        );
        ui.end_row();

        ui.label(tr!("settings-radio-receive"))
            .on_hover_text(tr!("settings-radio-af-radio-demodulates-sdroxide-shows"));
        ComboBox::from_id_salt("icomnet_rx_source").selected_text(net.rx_source.tr()).show_styled(
            ui,
            |ui| {
                for s in IcomRxSource::ALL {
                    if ui.selectable_label(net.rx_source == s, s.tr()).clicked() {
                        net.rx_source = s;
                    }
                }
            },
        );
        ui.end_row();

        radio_antenna_row(ui, caps, antenna_rx, "icomnet_antenna", cmds);
        radio_rx_antenna_row(ui, caps, rx_antenna, cmds);
        radio_power_row(ui, caps, cmds);

        ui.label(tr!("settings-radio-audio-sample-rate"));
        ComboBox::from_id_salt("icomnet_rate")
            .selected_text(format!("{} Hz", net.sample_rate_hz))
            .show_styled(ui, |ui| {
                for r in IcomNetConfig::SAMPLE_RATES {
                    if ui.selectable_label(net.sample_rate_hz == r, format!("{r} Hz")).clicked() {
                        net.sample_rate_hz = r;
                    }
                }
            });
        ui.end_row();

        // The 12 kHz IF cannot fit below 48 kHz, and silently falling back to AF
        // without saying so would look like the setting had not taken.
        if net.rx_source == IcomRxSource::If12k && !net.if_mode_usable() {
            ui.label("");
            ui.colored_label(
                egui::Color32::from_rgb(0xd0, 0x90, 0x30),
                tr!("settings-radio-t-12-khz-if-needs-48000"),
            );
            ui.end_row();
        }

        // Only on the IF path: the AF path is audio the radio has already
        // demodulated, and there is nothing left in it to mirror.
        if net.rx_source == IcomRxSource::If12k && net.if_mode_usable() {
            ui.label(tr!("settings-radio-if-spectrum"))
                .on_hover_text(tr!("settings-radio-which-way-round-radio-s"));
            ComboBox::from_id_salt("icomnet_invert_if")
                .selected_text(match net.invert_if {
                    None => tr!("settings-radio-automatic"),
                    Some(false) => tr!("settings-radio-normal"),
                    Some(true) => tr!("settings-radio-mirrored"),
                })
                .show_styled(ui, |ui| {
                    for (v, label) in [
                        (None, tr!("settings-radio-automatic")),
                        (Some(false), tr!("settings-radio-normal")),
                        (Some(true), tr!("settings-radio-mirrored")),
                    ] {
                        if ui.selectable_label(net.invert_if == v, label).clicked() {
                            net.invert_if = v;
                        }
                    }
                });
            ui.end_row();
        }

        if net.rx_source == IcomRxSource::Af {
            ui.label(tr!("settings-radio-displayed-bandwidth"));
            ui.add(
                egui::DragValue::new(&mut net.audio_bw_hz).range(1000.0..=24_000.0).suffix(" Hz"),
            )
            .on_hover_text(tr!("settings-radio-width-audio-band-panadapter-cat"));
            ui.end_row();
        }

        ui.label(tr!("settings-radio-cw-keying"))
            .on_hover_text(tr!("settings-radio-how-cw-panel-s-keyer-2"));
        ComboBox::from_id_salt("icomnet_cw").selected_text(net.cw_keying.tr()).show_styled(
            ui,
            |ui| {
                for k in CwKeying::ALL {
                    if ui.selectable_label(net.cw_keying == k, k.tr()).clicked() {
                        net.cw_keying = k;
                    }
                }
            },
        );
        ui.end_row();

        ui.label(tr!("settings-radio-transmit-buffer"));
        ui.add(egui::DragValue::new(&mut net.tx_latency_ms).range(20..=1000).suffix(" ms"))
            .on_hover_text(tr!("settings-radio-how-much-audio-radio-holds"));
        ui.end_row();

        ui.label("");
        crate::chrome::checkbox(
            ui,
            &mut net.scope,
            tr!("settings-radio-show-radio-s-spectrum-scope"),
        )
        .on_hover_text(tr!("settings-radio-streams-radio-s-own-475"));
        ui.end_row();

        if net.scope {
            ui.label(tr!("settings-radio-scope-span"))
                .on_hover_text(tr!("settings-radio-how-wide-sweep-only-wide"));
            ComboBox::from_id_salt("icomnet_scope_span")
                .selected_text(net.scope_span.tr())
                .show_styled(ui, |ui| {
                    for sp in IcomScopeSpan::ALL {
                        if ui.selectable_label(net.scope_span == sp, sp.tr()).clicked() {
                            net.scope_span = sp;
                        }
                    }
                });
            ui.end_row();
        }

        ui.label("");
        crate::chrome::checkbox(
            ui,
            &mut net.set_mod_input_on_open,
            tr!("settings-radio-switch-modulation-input-lan"),
        )
        .on_hover_text(tr!("settings-radio-transmit-audio-only-heard-when"));
        ui.end_row();

        ui.label("");
        // Both reach for this machine: the test opens its own socket from here,
        // and the trace is of the session *this* process ran. The engine's own
        // is on the engine's machine.
        probe_only(ui, can_probe, |ui| {
            ui.horizontal(|ui| {
                if ui.button(tr!("settings-radio-test-connection")).clicked() {
                    *test = true;
                }
                if ui
                    .button(tr!("settings-radio-copy-diagnostic-report"))
                    .on_hover_text(tr!("settings-radio-copies-radio-s-last-session"))
                    .clicked()
                {
                    *copy_report = true;
                }
            });
        });
        ui.end_row();
    });

    test_result_line(ui, test_result);

    ui.add_space(6.0);
    ui.label(RichText::new(tr!("settings-radio-control-audio-radio-s-own")).weak());
}

/// The outcome line under a "Test connection" button.
///
/// A successful test gets a second, weak line pointing at Apply / reconnect:
/// the test opens its own short-lived connection and the engine keeps running
/// whatever interface it had, but a green "Connected" on its own reads as
/// "done". A field report came from exactly that gap — a tested Pluto, an
/// unpressed Apply, and a blank screen.
///
/// The waiting line matters for the same kind of reason: the connection is made
/// from the machine the radio is on, so from a remote client the press and the
/// answer are seconds and a network apart, and a button that showed nothing in
/// between would be pressed again.
fn test_result_line(ui: &mut egui::Ui, result: &Option<crate::app::settings::TestOutcome>) {
    use crate::app::settings::TestOutcome;
    let result = match result {
        None => return,
        Some(TestOutcome::Waiting) => {
            ui.label(RichText::new(tr!("settings-radio-testing")).weak());
            return;
        }
        Some(TestOutcome::Done(r)) => r,
    };
    match result {
        Ok(s) => {
            ui.label(
                RichText::new(tr!("settings-radio-connected-detail", detail = s.to_string()))
                    .color(Color32::from_rgb(90, 200, 110)),
            );
            ui.label(RichText::new(tr!("settings-radio-was-only-check-press-apply")).weak());
        }
        Err(e) => {
            ui.label(
                RichText::new(tr!("settings-radio-failed", error = e.to_string()))
                    .color(Color32::from_rgb(230, 90, 80)),
            );
        }
    }
}

/// PlutoSDR interface: address, front-end settings, and the diagnostic report.
///
/// Two things about the layout are deliberate. The gain, AGC and ppm controls
/// apply *as you move them* (they push `SetGain` straight through), while the
/// address, sample rate and filter wait for Apply — the first group are things
/// you adjust while listening to a signal, the second are things that rebuild
/// the stream. And the tuning range is not stated here at all: a stock AD9363
/// board and one unlocked to AD9364 differ by an octave and a half, so the
/// number comes from the device, through Test connection.
#[allow(clippy::too_many_arguments)]
pub(in crate::app) fn settings_pluto_tab(
    ui: &mut egui::Ui,
    devices: &[sdroxide_types::PlutoDevice],
    radio_edit: &mut Option<sdroxide_types::RadioConfig>,
    discover: &mut bool,
    test: &mut bool,
    copy_report: &mut bool,
    test_result: &Option<crate::app::settings::TestOutcome>,
    can_probe: bool,
    cmds: &mut Vec<Command>,
) {
    use sdroxide_types::{PlutoAgc, PlutoConfig, PlutoDuplex, PlutoPtt};
    let Some(cfg) = radio_edit.as_mut() else {
        ui.label(tr!("settings-radio-waiting-configuration-machine-radio-attached"));
        return;
    };

    egui::Grid::new("pluto-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr!("settings-radio-radios"))
            .on_hover_text(tr!("settings-radio-asks-network-iio-devices-also"));
        // The mDNS query and the USB-gadget probe both go out from here, and a
        // Pluto on a USB cable is only reachable from the machine it is plugged
        // into. The Address row below is typed, so it still works from here.
        probe_only(ui, can_probe, |ui| {
            ui.horizontal(|ui| {
                if ui.button(tr!("settings-radio-discover")).clicked() {
                    *discover = true;
                }
                let shown = cfg.pluto.selected_ip.clone().unwrap_or_else(|| "— none —".into());
                ComboBox::from_id_salt("pluto_dev").width(340.0).selected_text(shown).show_styled(
                    ui,
                    |ui| {
                        if devices.is_empty() {
                            ui.label(
                                RichText::new(tr!("settings-radio-no-radios-press-discover"))
                                    .weak(),
                            );
                        }
                        for d in devices {
                            let sel = cfg.pluto.selected_ip.as_deref() == Some(d.ip.as_str());
                            if ui.selectable_label(sel, d.label()).clicked() {
                                cfg.pluto.selected_ip = Some(d.ip.clone());
                                // The typed address wins over a selection, so a
                                // click here has no visible effect until it is
                                // cleared. Do that for the operator.
                                cfg.pluto.address.clear();
                            }
                        }
                    },
                );
            });
        });
        ui.end_row();

        ui.label(tr!("settings-radio-address"))
            .on_hover_text(tr!("settings-radio-overrides-selection-above-usb-cable"));
        crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut cfg.pluto.address)
                .desired_width(220.0)
                .hint_text(PlutoConfig::DEFAULT_ADDRESS),
        );
        ui.end_row();

        // Which receive chain of the AD9361 this radio runs. Unlike TCI or
        // HPSDR the chains are not independently tunable — one synthesiser
        // serves both — so RX2 is a second *antenna*, not a second frequency.
        ui.label(tr!("settings-radio-receiver"));
        let shown = format!("RX{}", cfg.pluto.rx + 1);
        ComboBox::from_id_salt("pluto_rx")
            .selected_text(shown)
            .show_styled(ui, |ui| {
                for rx in 0u8..2 {
                    if ui.selectable_label(cfg.pluto.rx == rx, format!("RX{}", rx + 1)).clicked() {
                        cfg.pluto.rx = rx;
                    }
                }
            })
            .response
            .on_hover_text(tr!("settings-radio-pluto-revision-c-pluto-unlocked"));
        ui.end_row();

        ui.label(tr!("settings-radio-sample-rate"))
            .on_hover_text(tr!("settings-radio-width-spectrum-sdroxide-receives-ad9361"));
        let shown = format!("{:.3} Msps", cfg.pluto.sample_rate_hz / 1e6);
        ComboBox::from_id_salt("pluto_rate").selected_text(shown).show_styled(ui, |ui| {
            for &r in &PlutoConfig::SAMPLE_RATES {
                let sel = (cfg.pluto.sample_rate_hz - r).abs() < 1.0;
                let mut label = format!("{:.3} Msps", r / 1e6);
                if r >= 3_840_000.0 {
                    label.push_str(&tr!("settings-radio-more-than-usb-2-will"));
                } else if r < PlutoConfig::NO_FIR_FLOOR_HZ {
                    label.push_str(&tr!("settings-radio-stock-pluto-runs-2-084"));
                }
                if ui.selectable_label(sel, label).clicked() {
                    cfg.pluto.sample_rate_hz = r;
                }
            }
        });
        ui.end_row();

        ui.label(tr!("settings-radio-analog-filter"))
            .on_hover_text(tr!("settings-radio-ad9361-s-baseband-filter-leave"));
        let mut bw_khz = (cfg.pluto.rf_bandwidth_hz / 1000.0).round() as i64;
        if ui
            .add(DragValue::new(&mut bw_khz).range(0..=56_000).suffix(" kHz").custom_formatter(
                |v, _| {
                    if v <= 0.0 { "auto".to_string() } else { format!("{v:.0}") }
                },
            ))
            .changed()
        {
            cfg.pluto.rf_bandwidth_hz = bw_khz.max(0) as f64 * 1000.0;
        }
        ui.end_row();

        ui.label(tr!("settings-radio-agc"))
            .on_hover_text(tr!("settings-radio-ad9361-has-four-modes-not"));
        let mut agc = cfg.pluto.agc;
        enum_combo(ui, "pluto_agc", &mut agc, &PlutoAgc::ALL, PlutoAgc::tr);
        if agc != cfg.pluto.agc {
            cfg.pluto.agc = agc;
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: PlutoConfig::AGC_ELEMENT.to_string(),
                db: agc.code(),
            });
        }
        ui.end_row();

        ui.label(tr!("settings-radio-rx-gain"))
            .on_hover_text(tr!("settings-radio-applies-immediately-no-reconnect-ignored"));
        ui.add_enabled_ui(cfg.pluto.agc == PlutoAgc::Manual, |ui| {
            if crate::chrome::slider(
                ui,
                Slider::new(&mut cfg.pluto.rx_gain_db, 0.0..=71.0).step_by(1.0).suffix(" dB"),
            )
            .changed()
            {
                cmds.push(Command::SetGain {
                    dir: Direction::Rx,
                    element: PlutoConfig::RF_GAIN_ELEMENT.to_string(),
                    db: cfg.pluto.rx_gain_db,
                });
            }
        });
        ui.end_row();

        ui.label(tr!("settings-radio-tx-gain"))
            .on_hover_text(tr!("settings-radio-negative-because-ad9361-expresses-transmit"));
        if crate::chrome::slider(
            ui,
            Slider::new(&mut cfg.pluto.tx_gain_db, -89.75..=0.0).step_by(0.25).suffix(" dB"),
        )
        .changed()
        {
            cmds.push(Command::SetGain {
                dir: Direction::Tx,
                element: PlutoConfig::TX_GAIN_ELEMENT.to_string(),
                db: cfg.pluto.tx_gain_db,
            });
        }
        ui.end_row();

        ui.label(tr!("settings-radio-frequency-correction"))
            .on_hover_text(tr!("settings-radio-reference-error-parts-per-million"));
        let mut ppm = cfg.pluto.ppm;
        if ui
            .add(
                DragValue::new(&mut ppm)
                    .range(-200.0..=200.0)
                    .speed(0.1)
                    .suffix(tr!("settings-radio-ppm")),
            )
            .changed()
        {
            cfg.pluto.ppm = ppm;
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: PlutoConfig::PPM_ELEMENT.to_string(),
                db: ppm,
            });
        }
        ui.end_row();

        // Not a property of the board — the AD9361 does this natively — but of
        // the network it is on, which is why it is asked rather than detected:
        // the failure mode is a starved transmit buffer, heard on the air as a
        // chopped envelope rather than reported as an error.
        ui.label(tr!("settings-radio-full-duplex"));
        crate::chrome::checkbox(
            ui,
            &mut cfg.pluto.full_duplex,
            tr!("settings-radio-keep-receiving-while-transmitting"),
        )
        .on_hover_text(tr!("settings-radio-leave-off-pluto-reached-over"));
        ui.end_row();

        // Below Full duplex because the two argue: TDD is one direction at a
        // time in the silicon, so it settles the question the checkbox above
        // asks about the link.
        ui.label(tr!("settings-radio-duplex"))
            .on_hover_text(tr!("settings-radio-whether-ad9361-runs-both-directions"));
        let mut duplex = cfg.pluto.duplex;
        enum_combo(ui, "pluto_duplex", &mut duplex, &PlutoDuplex::ALL, PlutoDuplex::tr);
        cfg.pluto.duplex = duplex;
        ui.end_row();

        ui.label(tr!("settings-radio-ptt-pins"))
            .on_hover_text(tr!("settings-radio-pluto-s-four-gpo-test"));
        let mut ptt = cfg.pluto.ptt_gpo;
        enum_combo(ui, "pluto_ptt", &mut ptt, &PlutoPtt::ALL, PlutoPtt::tr);
        cfg.pluto.ptt_gpo = ptt;
        ui.end_row();

        // Next to Full duplex because it belongs to the same subject: both are
        // about what the link between here and the board can carry, not about
        // what the AD9361 can do. This one used to be reachable only by hand-
        // editing radio.json, which is no use to the operator who needs it —
        // the one whose log is reporting a replaced receive socket.
        ui.label(tr!("settings-radio-buffer-size"))
            .on_hover_text(tr!("settings-radio-how-much-device-holds-before"));
        ui.horizontal(|ui| {
            let mut samples = cfg.pluto.buffer_samples as i64;
            if ui
                .add(
                    DragValue::new(&mut samples)
                        .range(1024..=1_048_576)
                        .speed(256.0)
                        .suffix(tr!("settings-radio-samples")),
                )
                .changed()
            {
                // Kept a multiple of 1024, as the transmit buffer is, so the
                // byte count stays comfortably aligned for the device's DMA.
                cfg.pluto.buffer_samples =
                    (samples.clamp(1024, 1_048_576) as usize / 1024 * 1024).max(1024);
            }
            // Samples are the unit the device takes; milliseconds are the unit
            // the decision is made in. Worked out against the rate set two rows
            // up rather than left to the operator.
            let rate = cfg.pluto.sample_rate_hz.max(1.0);
            ui.label(
                RichText::new(tr!(
                    "settings-radio-pluto-buffer",
                    ms = format!("{:.1}", cfg.pluto.buffer_samples as f64 / rate * 1e3),
                    kib = cfg.pluto.buffer_samples * 4 / 1024
                ))
                .weak(),
            );
        });
        ui.end_row();

        ui.label(tr!("settings-radio-rx-tx-port"))
            .on_hover_text(tr!("settings-radio-ad9361-s-rf-port-select"));
        ui.horizontal(|ui| {
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut cfg.pluto.rx_port)
                    .desired_width(120.0)
                    .hint_text("A_BALANCED"),
            );
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut cfg.pluto.tx_port)
                    .desired_width(80.0)
                    .hint_text("A"),
            );
        });
        ui.end_row();

        ui.label("");
        // Both run here: the test opens the radio from this machine, and the
        // trace is of this process's own session.
        probe_only(ui, can_probe, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(tr!("settings-radio-test-connection"))
                    .on_hover_text(tr!("settings-radio-opens-radio-reads-what-says"))
                    .clicked()
                {
                    *test = true;
                }
                if ui
                    .button(tr!("settings-radio-copy-diagnostic-report"))
                    .on_hover_text(tr!("settings-radio-copies-last-session-s-protocol"))
                    .clicked()
                {
                    *copy_report = true;
                }
            });
        });
        ui.end_row();
    });

    test_result_line(ui, test_result);

    ui.add_space(6.0);
    ui.label(RichText::new(tr!("settings-radio-wideband-iq-receive-transmit-half")).weak());
    ui.label(
        RichText::new(tr!("settings-radio-not-yet-verified-against-real"))
            .color(crate::theme::YELLOW()),
    );
}

/// SmartSDR (FlexRadio) interface: radio selection, DAX IQ stream settings, and
/// the diagnostic report.
///
/// The report button is not decoration. This backend has never been run against
/// a FLEX, so the first people to use it are the ones who can say whether it
/// works — and asking them to reproduce a fault with the right `RUST_LOG`
/// filter set is asking them to reproduce it twice. The trace is always
/// recorded; this copies it out.
#[allow(clippy::too_many_arguments)]
pub(in crate::app) fn settings_smartsdr_tab(
    ui: &mut egui::Ui,
    devices: &[sdroxide_types::SmartSdrDevice],
    radio_edit: &mut Option<sdroxide_types::RadioConfig>,
    discover: &mut bool,
    test: &mut bool,
    copy_report: &mut bool,
    test_result: &Option<crate::app::settings::TestOutcome>,
    can_probe: bool,
) {
    use sdroxide_types::SmartSdrConfig;
    let Some(cfg) = radio_edit.as_mut() else {
        ui.label(tr!("settings-radio-waiting-configuration-machine-radio-attached"));
        return;
    };

    egui::Grid::new("smartsdr-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr!("settings-radio-radios"))
            .on_hover_text(tr!("settings-radio-flexradio-announces-itself-local-network"));
        // The broadcasts a FLEX sends reach its own network segment, which is
        // the engine's, not this screen's. The Address row below is typed, so
        // it still works from here.
        probe_only(ui, can_probe, |ui| {
            ui.horizontal(|ui| {
                if ui.button(tr!("settings-radio-discover")).clicked() {
                    *discover = true;
                }
                let shown = cfg.smartsdr.selected_ip.clone().unwrap_or_else(|| "— none —".into());
                ComboBox::from_id_salt("flex_dev").width(340.0).selected_text(shown).show_styled(
                    ui,
                    |ui| {
                        if devices.is_empty() {
                            ui.label(
                                RichText::new(tr!("settings-radio-no-radios-press-discover"))
                                    .weak(),
                            );
                        }
                        for d in devices {
                            let sel = cfg.smartsdr.selected_ip.as_deref() == Some(d.ip.as_str());
                            // A radio that is already claimed and has multiFLEX off
                            // will refuse us, so it is shown but not selectable.
                            if d.joinable {
                                if ui.selectable_label(sel, d.label()).clicked() {
                                    cfg.smartsdr.selected_ip = Some(d.ip.clone());
                                }
                            } else {
                                ui.label(RichText::new(d.label()).weak()).on_hover_text(tr!(
                                    "settings-radio-another-gui-client-has-radio"
                                ));
                            }
                        }
                    },
                );
            });
        });
        ui.end_row();

        ui.label(tr!("settings-radio-address"))
            .on_hover_text(tr!("settings-radio-overrides-selection-above-use-radio"));
        crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut cfg.smartsdr.address)
                .desired_width(220.0)
                .hint_text(tr!("settings-radio-optional-e-g-192-168")),
        );
        ui.end_row();

        ui.label(tr!("settings-radio-iq-sample-rate"))
            .on_hover_text(tr!("settings-radio-width-spectrum-sdroxide-receives-192"));
        let shown = format!("{} kHz", (cfg.smartsdr.iq_sample_rate_hz / 1000.0) as u32);
        ComboBox::from_id_salt("flex_rate").selected_text(shown).show_styled(ui, |ui| {
            for &r in &SmartSdrConfig::IQ_RATES {
                let sel = (cfg.smartsdr.iq_sample_rate_hz - r).abs() < 1.0;
                if ui.selectable_label(sel, format!("{} kHz", (r / 1000.0) as u32)).clicked() {
                    cfg.smartsdr.iq_sample_rate_hz = r;
                }
            }
        });
        ui.end_row();

        // Said out loud rather than left in the tooltip above: "why is the band
        // only 192 kHz wide?" is the question this interface gets asked (issue
        // #184), and an operator wondering that is looking at this row, not
        // hovering it.
        ui.label("");
        ui.label(RichText::new(tr!("settings-radio-t-192-khz-widest-dax-iq")).weak());
        ui.end_row();

        ui.label(tr!("settings-radio-dax-iq-channel"))
            .on_hover_text(tr!("settings-radio-radio-has-four-change-only"));
        ComboBox::from_id_salt("flex_ch")
            .selected_text(cfg.smartsdr.iq_channel.to_string())
            .show_styled(ui, |ui| {
                for ch in SmartSdrConfig::IQ_CHANNELS {
                    let sel = cfg.smartsdr.iq_channel == ch;
                    if ui.selectable_label(sel, ch.to_string()).clicked() {
                        cfg.smartsdr.iq_channel = ch;
                    }
                }
            });
        ui.end_row();

        ui.label(tr!("settings-radio-station-name"))
            .on_hover_text(tr!("settings-radio-shown-against-session-radio-s"));
        crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut cfg.smartsdr.station).desired_width(160.0),
        );
        ui.end_row();

        ui.label(tr!("settings-radio-gui-client-id"))
            .on_hover_text(tr!("settings-radio-identity-radio-restores-session-s"));
        crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut cfg.smartsdr.gui_client_id)
                .desired_width(280.0)
                .hint_text(tr!("settings-radio-optional-e-g-uuid-your")),
        );
        ui.end_row();

        ui.label(tr!("settings-radio-invert-spectrum"));
        crate::chrome::checkbox(ui, &mut cfg.smartsdr.swap_iq, tr!("settings-radio-swap-i-q"))
            .on_hover_text(tr!("settings-radio-mirror-radio-s-i-q"));
        ui.end_row();

        ui.label(tr!("settings-radio-network-mtu"))
            .on_hover_text(tr!("settings-radio-largest-datagram-radio-may-send"));
        let mut mtu = cfg.smartsdr.network_mtu as f64;
        if crate::chrome::field(
            ui,
            egui::DragValue::new(&mut mtu)
                .speed(10.0)
                .range(576.0..=9000.0)
                .suffix(tr!("settings-radio-b")),
        )
        .changed()
        {
            cfg.smartsdr.network_mtu = mtu.round() as u32;
        }
        ui.end_row();

        ui.label("");
        // Both run here: the test opens its own connection from this machine,
        // and the trace is of this process's own session.
        probe_only(ui, can_probe, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(tr!("settings-radio-test-connection"))
                    .on_hover_text(tr!("settings-radio-checks-radio-answers-without-registering"))
                    .clicked()
                {
                    *test = true;
                }
                if ui
                    .button(tr!("settings-radio-copy-diagnostic-report"))
                    .on_hover_text(tr!("settings-radio-copies-last-session-s-protocol"))
                    .clicked()
                {
                    *copy_report = true;
                }
            });
        });
        ui.end_row();
    });

    test_result_line(ui, test_result);

    ui.add_space(6.0);
    ui.label(RichText::new(tr!("settings-radio-wideband-iq-receive-over-dax")).weak());
    ui.label(
        RichText::new(tr!("settings-radio-not-yet-verified-against-real-2"))
            .color(Color32::from_rgb(220, 170, 70)),
    );
}

/// What SoapySDR can see on this machine, and what it means.
///
/// The SoapySDR interface has no device picker — the device is chosen by
/// `--device` or `device_args` in `config.toml`, and until that is typed the
/// first enumerated device wins. That made this list worth showing: on a bundle
/// install (PothosSDR ships every module) the winner can be the sound card, and
/// nothing on screen says so. A driver with a native interface is called out
/// too, because reaching an RSP or a dongle through SoapySDR gives up every
/// model-specific control sdroxide has for it.
pub(in crate::app) fn settings_soapy_devices(
    ui: &mut egui::Ui,
    devices: Option<&[sdroxide_types::SoapyDeviceInfo]>,
    rescan: &mut bool,
    can_probe: bool,
) {
    use sdroxide_types::SoapyDeviceInfo;

    // The list is what the *radio's* machine found, which is where the modules
    // are installed and where `device_args` is read from. Until it has
    // answered there is nothing here worth drawing a Rescan button beside.
    if !can_probe && devices.is_none() {
        ui.label(RichText::new(tr!("settings-radio-devices-soapysdr-can-see")).strong());
        ui.label(RichText::new(tr!("settings-radio-waiting-machine-radio-attached-where")).weak());
        return;
    }

    ui.horizontal(|ui| {
        ui.label(RichText::new(tr!("settings-radio-devices-soapysdr-can-see")).strong());
        probe_only(ui, can_probe, |ui| {
            if ui
                .button(tr!("settings-radio-rescan"))
                .on_hover_text(tr!("settings-radio-ask-every-installed-soapysdr-module"))
                .clicked()
            {
                *rescan = true;
            }
        });
    });

    let Some(devices) = devices else {
        ui.label(RichText::new(tr!("settings-radio-not-enumerated-yet-press-rescan")).weak());
        return;
    };
    if devices.is_empty() {
        ui.label(RichText::new(tr!("settings-radio-no-soapysdr-devices-found-check")).weak());
        return;
    }

    for d in devices {
        ui.horizontal(|ui| {
            ui.label(RichText::new(d.label()).monospace());
            if d.is_pseudo() {
                ui.label(
                    RichText::new(tr!("settings-radio-not-radio"))
                        .color(Color32::from_rgb(220, 170, 70)),
                );
            }
        });
    }

    // The sound-card trap: named, with the reason and the way out.
    if devices.iter().any(SoapyDeviceInfo::is_pseudo) {
        ui.add_space(4.0);
        ui.label(
            RichText::new(tr!("settings-radio-sound-card-listed-above-if"))
                .color(Color32::from_rgb(220, 170, 70)),
        );
    }

    // Hardware with a native interface: say so, because the native one is
    // strictly better and the operator has no way to know it exists. Named
    // once each — two RSPs are still one interface to switch to — and by
    // `contains` rather than `dedup`, which would keep a repeat that another
    // driver happens to sit between.
    let mut native: Vec<sdroxide_types::Backend> = Vec::new();
    for b in devices.iter().filter_map(SoapyDeviceInfo::native_backend) {
        if !native.contains(&b) {
            native.push(b);
        }
    }
    if !native.is_empty() {
        ui.add_space(4.0);
        let names = native.iter().map(|b| b.tr()).collect::<Vec<_>>().join(", ");
        ui.label(
            RichText::new(tr!("settings-radio-soapy-native", names = names.to_string()))
                .color(crate::theme::CYAN()),
        );
    }

    ui.add_space(4.0);
    ui.label(RichText::new(tr!("settings-radio-which-one-opens-set-device")).weak());
}

impl SdroxideApp {
    /// SoapySDR RX/TX gains + antenna (empty for a CAT rig).
    pub(in crate::app) fn settings_device_tab(&self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let Some(caps) = &self.caps else {
            ui.label(tr!("settings-radio-no-device"));
            return;
        };
        ui.label(RichText::new(&caps.label).size(14.0).strong().color(crate::theme::CYAN()));
        ui.add_space(6.0);
        if caps.gains.iter().all(|g| g.direction != Direction::Rx) {
            ui.label(RichText::new(tr!("settings-radio-rig-has-no-software-adjustable")).weak());
        }
        ui.label(RichText::new(tr!("settings-radio-rx-gains")).strong());
        egui::Grid::new("gains").num_columns(2).show(ui, |ui| {
            for g in caps.gains.iter().filter(|g| g.direction == Direction::Rx) {
                ui.label(&g.name);
                let mut db = self
                    .state
                    .gains
                    .iter()
                    .find(|(n, _)| *n == g.name)
                    .map(|(_, d)| *d)
                    .unwrap_or(g.min_db);
                let step = if g.step_db > 0.0 { g.step_db } else { 1.0 };
                if crate::chrome::slider(
                    ui,
                    Slider::new(&mut db, g.min_db..=g.max_db).step_by(step).suffix(" dB"),
                )
                .changed()
                {
                    cmds.push(Command::SetGain { dir: Direction::Rx, element: g.name.clone(), db });
                }
                ui.end_row();
            }
        });
        if caps.gains.iter().any(|g| g.direction == Direction::Tx) {
            ui.separator();
            ui.label(
                RichText::new(tr!("settings-radio-tx-gains"))
                    .strong()
                    .color(Color32::from_rgb(240, 90, 60)),
            );
            egui::Grid::new("tx-gains").num_columns(2).show(ui, |ui| {
                for g in caps.gains.iter().filter(|g| g.direction == Direction::Tx) {
                    ui.label(&g.name);
                    let mut db = self
                        .state
                        .tx_gains
                        .iter()
                        .find(|(n, _)| *n == g.name)
                        .map(|(_, d)| *d)
                        .unwrap_or(g.min_db);
                    let step = if g.step_db > 0.0 { g.step_db } else { 1.0 };
                    if crate::chrome::slider(
                        ui,
                        Slider::new(&mut db, g.min_db..=g.max_db).step_by(step).suffix(" dB"),
                    )
                    .changed()
                    {
                        cmds.push(Command::SetGain {
                            dir: Direction::Tx,
                            element: g.name.clone(),
                            db,
                        });
                    }
                    ui.end_row();
                }
            });
        }
        // Only worth a control where there is a choice to make: a front end with
        // one port has nothing to switch to, and a row saying so is noise.
        let rx_ports = caps.antennas_rx.len() > 1;
        let tx_ports = caps.antennas_tx.len() > 1;
        if rx_ports || tx_ports {
            ui.separator();
            ui.label(RichText::new(tr!("settings-radio-antennas")).strong());
            egui::Grid::new("antennas").num_columns(2).show(ui, |ui| {
                if rx_ports {
                    ui.label(tr!("settings-radio-rx"));
                    ComboBox::from_id_salt("ant-rx")
                        .selected_text(self.state.antenna_rx.clone())
                        .show_styled(ui, |ui| {
                            for a in &caps.antennas_rx {
                                if ui.selectable_label(self.state.antenna_rx == *a, a).clicked() {
                                    cmds.push(Command::SetAntenna {
                                        dir: Direction::Rx,
                                        name: a.clone(),
                                    });
                                }
                            }
                        });
                    ui.end_row();
                }
                if tx_ports {
                    ui.label(
                        RichText::new(tr!("settings-radio-tx"))
                            .color(Color32::from_rgb(240, 90, 60)),
                    );
                    ComboBox::from_id_salt("ant-tx")
                        .selected_text(self.state.antenna_tx.clone())
                        .show_styled(ui, |ui| {
                            for a in &caps.antennas_tx {
                                if ui.selectable_label(self.state.antenna_tx == *a, a).clicked() {
                                    cmds.push(Command::SetAntenna {
                                        dir: Direction::Tx,
                                        name: a.clone(),
                                    });
                                }
                            }
                        });
                    ui.end_row();
                }
            });
            ui.label(RichText::new(tr!("settings-radio-remembered-next-start")).weak());
        }
    }
}

/// A frequency in the unit an operator reads it in, with the unit spelled by
/// the caller — the same number is "2.000 Msps" as a rate and "2.000 MHz" as a
/// filter width, and calling one by the other's name is how a panel starts
/// quietly misinforming people.
fn scaled_label(hz: f64, mega: &str, kilo: &str) -> String {
    if hz >= 1e6 { format!("{:.3} {mega}", hz / 1e6) } else { format!("{:.0} {kilo}", hz / 1e3) }
}

fn rate_label(hz: f64) -> String {
    scaled_label(hz, "Msps", "ksps")
}

fn bw_label(hz: f64) -> String {
    scaled_label(hz, "MHz", "kHz")
}

/// The SoapySDR device's own controls, drawn from what it says about itself.
///
/// Nothing on this panel is device-specific code. The rates, the filter widths
/// and every switch below them come from the driver's own answers
/// (`DeviceCaps::sample_rates`, `bandwidths`, `settings`), so a radio nobody
/// here has ever run still gets the controls its author wrote for it — which is
/// the point of reaching it through SoapySDR rather than through a backend of
/// its own.
///
/// Two speeds, deliberately. A driver setting is written to the running radio
/// the moment it is touched, because most of them are switches — a bias tee, a
/// notch, a direct-sampling branch — and one that took effect only after a
/// restart is one the operator cannot tell is working. The rate and the filter
/// need the DSP chain rebuilt around them, so those ask for a reopen.
///
/// Every control's "leave it alone" option is the default and comes first. A
/// driver knows more about its own hardware than this panel does, and an
/// operator who has not chosen should not silently be overriding it.
pub(in crate::app) fn settings_soapy_tab(
    ui: &mut egui::Ui,
    caps: Option<&sdroxide_types::DeviceCaps>,
    radio_edit: &mut Option<sdroxide_types::RadioConfig>,
    apply: &mut bool,
    cmds: &mut Vec<Command>,
) {
    use sdroxide_types::SettingKind;

    let Some(cfg) = radio_edit.as_mut() else {
        ui.label(tr!("settings-radio-waiting-configuration-machine-radio-attached"));
        return;
    };
    let Some(caps) = caps else {
        ui.label(RichText::new(tr!("settings-radio-these-controls-appear-once-soapysdr")).weak());
        return;
    };

    if !caps.sample_rates.is_empty() || !caps.rate_ranges.is_empty() {
        ui.label(RichText::new(tr!("settings-radio-stream")).strong());
        egui::Grid::new("soapy-stream").num_columns(2).show(ui, |ui| {
            ui.label(tr!("settings-radio-sample-rate"));
            let shown = if cfg.soapy.sample_rate_hz > 0.0 {
                rate_label(cfg.soapy.sample_rate_hz)
            } else {
                tr!("settings-radio-follow-app-setting")
            };
            ComboBox::from_id_salt("soapy-rate").selected_text(shown).show_styled(ui, |ui| {
                if ui
                    .selectable_label(
                        cfg.soapy.sample_rate_hz <= 0.0,
                        tr!("settings-radio-follow-app-setting"),
                    )
                    .clicked()
                {
                    cfg.soapy.sample_rate_hz = 0.0;
                    *apply = true;
                }
                for &r in &caps.sample_rates {
                    let sel = (cfg.soapy.sample_rate_hz - r).abs() < 1.0;
                    if ui.selectable_label(sel, rate_label(r)).clicked() {
                        cfg.soapy.sample_rate_hz = r;
                        *apply = true;
                    }
                }
            });
            ui.end_row();

            // A driver that publishes a continuous range rather than a list —
            // plenty do — has nothing to put in the combo above, so it gets a
            // number to type instead. Applied on release rather than per
            // keystroke: every digit typed would otherwise reopen the radio.
            if !caps.rate_ranges.is_empty() {
                let (lo, hi) = caps
                    .rate_ranges
                    .iter()
                    .fold((f64::MAX, 0.0f64), |(a, b), &(l, h)| (a.min(l), b.max(h)));
                ui.label(tr!("settings-radio-msps"));
                let mut msps = cfg.soapy.sample_rate_hz / 1e6;
                let r =
                    ui.add(egui::DragValue::new(&mut msps).speed(0.05).range(lo / 1e6..=hi / 1e6));
                if r.changed() {
                    cfg.soapy.sample_rate_hz = msps * 1e6;
                }
                if r.drag_stopped() || r.lost_focus() {
                    *apply = true;
                }
                ui.end_row();
            }

            if !caps.bandwidths.is_empty() {
                ui.label(tr!("settings-radio-baseband-filter"));
                let shown = if cfg.soapy.bandwidth_hz > 0.0 {
                    bw_label(cfg.soapy.bandwidth_hz)
                } else {
                    tr!("settings-radio-let-driver-choose")
                };
                ComboBox::from_id_salt("soapy-bw").selected_text(shown).show_styled(ui, |ui| {
                    if ui
                        .selectable_label(
                            cfg.soapy.bandwidth_hz <= 0.0,
                            tr!("settings-radio-let-driver-choose"),
                        )
                        .clicked()
                    {
                        cfg.soapy.bandwidth_hz = 0.0;
                        *apply = true;
                    }
                    for &b in &caps.bandwidths {
                        let sel = (cfg.soapy.bandwidth_hz - b).abs() < 1.0;
                        if ui.selectable_label(sel, bw_label(b)).clicked() {
                            cfg.soapy.bandwidth_hz = b;
                            *apply = true;
                        }
                    }
                });
                ui.end_row();
            }
        });
        ui.label(RichText::new(tr!("settings-radio-changing-either-reopens-radio")).weak());
    }

    if caps.settings.is_empty() {
        ui.add_space(4.0);
        ui.label(RichText::new(tr!("settings-radio-driver-publishes-no-settings-own")).weak());
        return;
    }

    ui.separator();
    ui.label(
        RichText::new(tr!("settings-radio-driver-settings", driver = caps.driver.to_string()))
            .strong(),
    );
    egui::Grid::new("soapy-settings").num_columns(2).show(ui, |ui| {
        for st in &caps.settings {
            let label = ui.label(&st.name);
            if !st.description.is_empty() {
                label.on_hover_text(&st.description);
            }
            // What the operator chose, or failing that what the radio answered
            // when it was asked at open.
            let current = cfg.soapy.setting(&st.key).unwrap_or(&st.value).to_string();
            let mut chosen: Option<String> = None;

            match st.kind {
                SettingKind::Bool => {
                    // SoapySDR spells booleans "true"/"false" on the wire.
                    let mut on = matches!(current.as_str(), "true" | "1" | "True");
                    if crate::chrome::checkbox(ui, &mut on, "").changed() {
                        chosen = Some(if on { "true".into() } else { "false".into() });
                    }
                }
                _ if !st.options.is_empty() => {
                    ComboBox::from_id_salt(format!("soapy-set-{}", st.key))
                        .selected_text(current.clone())
                        .show_styled(ui, |ui| {
                            for o in &st.options {
                                if ui.selectable_label(current == *o, o).clicked() {
                                    chosen = Some(o.clone());
                                }
                            }
                        });
                }
                _ => {
                    // Numbers and free text alike: the driver is the only thing
                    // that knows what it will accept, so this passes the text
                    // through and lets it refuse — which surfaces as a notice
                    // rather than as a silently clamped value.
                    let mut text = current.clone();
                    let r = ui.add(egui::TextEdit::singleline(&mut text).desired_width(120.0));
                    if r.lost_focus() && text != current {
                        chosen = Some(text);
                    }
                }
            }
            if !st.units.is_empty() {
                ui.label(RichText::new(&st.units).weak());
            }

            if let Some(v) = chosen {
                // Straight to the running radio, and remembered for next time.
                cmds.push(Command::SetDeviceSetting { key: st.key.clone(), value: v.clone() });
                cfg.soapy.set_setting(&st.key, &v);
            }
            ui.end_row();
        }
    });
    ui.label(RichText::new(tr!("settings-radio-applied-once-remembered-next-start")).weak());
}

/// Settings for the RX-888 direct-sampling receiver.
///
/// The layout follows the signal path: which receiver, how fast to clock the
/// ADC, then the two analogue gain stages, then the switches. The ADC rate is
/// the one setting an operator can get badly wrong — it decides both how much
/// spectrum is visible and how much USB bandwidth is needed — so it says what it
/// costs rather than just listing numbers.
pub(in crate::app) fn settings_rx888_tab(
    ui: &mut egui::Ui,
    devices: &[sdroxide_types::Rx888Device],
    radio_edit: &mut Option<sdroxide_types::RadioConfig>,
    rescan: &mut bool,
    apply: &mut bool,
    can_probe: bool,
    // Where the dial is, so a width the tuner's IF cannot fill can say so while
    // the receiver is actually up there rather than only in the abstract.
    dial_hz: f64,
    cmds: &mut Vec<Command>,
) {
    use sdroxide_types::Rx888Config;
    let Some(cfg) = radio_edit.as_mut() else {
        ui.label(tr!("settings-radio-waiting-configuration-machine-radio-attached"));
        return;
    };

    // Everything on this panel takes effect as soon as it is touched. The gain
    // stages ride `SetGain` straight to the running device; the rest need the
    // DSP chain rebuilt around a new sample rate, so they ask for a reopen
    // instead of leaving the operator to find a button. That is affordable here
    // in a way it is not for other backends — the device is already programmed,
    // so reopening it costs about a millisecond plus the firmware's own start
    // latency, measured at ~150 ms end to end.
    //
    // The ADC clock is *not* in this tuple: its free-entry field changes the
    // value on every frame of a drag, and a reopen per pixel would restart the
    // receiver hundreds of times. Its combo and entry field push `apply`
    // themselves, on the click and on release.
    let before = (cfg.rx888.serial.clone(), cfg.rx888.randomize, cfg.rx888.ddc_bins);

    egui::Grid::new("rx888-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr!("settings-radio-receiver"));
        // Which receiver is this panel's one row about a USB bus; everything
        // below reaches the device wherever it is plugged in.
        probe_only(ui, can_probe, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(tr!("settings-radio-rescan"))
                    .on_hover_text(tr!("settings-radio-re-list-usb-bus-no"))
                    .clicked()
                {
                    *rescan = true;
                }
                let shown = if cfg.rx888.serial.is_empty() {
                    tr!("settings-radio-first-one-found")
                } else {
                    cfg.rx888.serial.clone()
                };
                ComboBox::from_id_salt("rx888_dev").width(300.0).selected_text(shown).show_styled(
                    ui,
                    |ui| {
                        if devices.is_empty() {
                            ui.label(tr!("settings-radio-no-rx-888-found-press"));
                        }
                        ui.selectable_value(
                            &mut cfg.rx888.serial,
                            String::new(),
                            tr!("settings-radio-first-one-found"),
                        );
                        for d in devices {
                            let serial = d.serial.clone().unwrap_or_default();
                            ui.selectable_value(&mut cfg.rx888.serial, serial, d.label());
                        }
                    },
                );
            });
        });
        ui.end_row();

        ui.label(tr!("settings-radio-adc-clock"));
        ui.horizontal(|ui| {
            let rate = cfg.rx888.adc_rate_hz;
            ComboBox::from_id_salt("rx888_rate")
                .width(150.0)
                .selected_text(format!("{:.1} Msps", rate / 1e6))
                .show_styled(ui, |ui| {
                    for r in Rx888Config::ADC_RATES {
                        if ui
                            .selectable_value(
                                &mut cfg.rx888.adc_rate_hz,
                                r,
                                format!("{:.1} Msps", r / 1e6),
                            )
                            .changed()
                        {
                            *apply = true;
                        }
                    }
                });
            // Inside a grid (and a horizontal row) labels default to Extend,
            // which pushes the row off the window edge instead of wrapping.
            ui.add(
                egui::Label::new(
                    egui::RichText::new(tr!(
                        "settings-radio-rx888-coverage",
                        mhz = format!("{:.1}", rate / 2e6),
                        mbytes = format!("{:.0}", rate * 2.0 / 1e6)
                    ))
                    .weak(),
                )
                .wrap(),
            );
        });
        ui.end_row();

        // The Si5351 synthesises nearly any clock in range, so the list above
        // is the common set, not a limit. Applied on release rather than per
        // keystroke — every digit typed would otherwise reopen the radio.
        ui.label(tr!("settings-radio-msps"));
        ui.horizontal(|ui| {
            let mut msps = cfg.rx888.adc_rate_hz / 1e6;
            let r = ui.add(
                egui::DragValue::new(&mut msps)
                    .speed(0.1)
                    .range(Rx888Config::MIN_ADC_HZ / 1e6..=Rx888Config::MAX_ADC_HZ / 1e6),
            );
            if r.changed() {
                cfg.rx888.adc_rate_hz = (msps * 1e6).round();
            }
            if r.drag_stopped() || r.lost_focus() {
                *apply = true;
            }
        });
        ui.end_row();
        ui.label("");
        ui.add(
            egui::Label::new(
                egui::RichText::new(tr!("settings-radio-t-129-6-msps-needs-superspeed")).weak(),
            )
            .wrap(),
        );
        ui.end_row();

        ui.label(tr!("settings-radio-panadapter-width"));
        ui.horizontal(|ui| {
            let rate = cfg.rx888.adc_rate_hz;
            // A width the tuner's IF cannot fill is marked rather than hidden:
            // it is a perfectly good HF setting, and on a receiver that spends
            // its life below 65 MHz there is nothing wrong with it at all.
            let width_label = |bins: u32| {
                format!(
                    "{} — 1/{}{}",
                    bw_label(Rx888Config::ddc_out_rate_hz(rate, bins)),
                    Rx888Config::DDC_BLOCK / bins.max(1),
                    if Rx888Config::width_works_on_vhf(rate, bins) {
                        String::new()
                    } else {
                        tr!("settings-radio-hf-only")
                    },
                )
            };
            let bins = cfg.rx888.ddc_bins;
            ComboBox::from_id_salt("rx888_width")
                .width(210.0)
                .selected_text(width_label(if bins == 0 { 256 } else { bins }))
                .show_styled(ui, |ui| {
                    for b in Rx888Config::DDC_BIN_CHOICES {
                        ui.selectable_value(&mut cfg.rx888.ddc_bins, b, width_label(b))
                            .on_hover_text(if Rx888Config::width_works_on_vhf(rate, b) {
                                tr!("settings-radio-usable-both-front-ends")
                            } else {
                                tr!("settings-radio-below-vhf-crossover-ordinary-width")
                            });
                    }
                });
            ui.add(
                egui::Label::new(
                    egui::RichText::new(tr!(
                        "settings-radio-rx888-digitised",
                        mhz = format!("{:.1}", rate / 2e6)
                    ))
                    .weak(),
                )
                .wrap(),
            );
        });
        ui.end_row();
        ui.label("");
        let rate = cfg.rx888.adc_rate_hz;
        ui.vertical(|ui| {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(tr!(
                        "settings-radio-rx888-width-hint",
                        crossover = format!("{:.1}", Rx888Config::vhf_crossover_hz(rate) / 1e6),
                        if_centre = format!("{:.2}", Rx888Config::VHF_IF_CENTER_HZ / 1e6),
                        widest = bw_label(2.0 * Rx888Config::VHF_IF_CENTER_HZ)
                    ))
                    .weak(),
                )
                .wrap(),
            );
            // Said out loud while it is actually happening: the symptom — every
            // signal crowded onto the right-hand half — looks like a broken
            // receiver rather than a setting.
            if dial_hz >= Rx888Config::vhf_crossover_hz(rate)
                && !Rx888Config::width_works_on_vhf(rate, cfg.rx888.ddc_bins)
            {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(tr!("settings-radio-dial-above-crossover-now-so"))
                            .color(crate::theme::YELLOW()),
                    )
                    .wrap(),
                );
            }
        });
        ui.end_row();

        ui.label(tr!("settings-radio-vga-gain"));
        if crate::chrome::slider(
            ui,
            egui::Slider::new(&mut cfg.rx888.vga_db, -6.0..=34.0).suffix(" dB"),
        )
        .on_hover_text(tr!("settings-radio-ad8370-variable-gain-amplifier-ahead"))
        .changed()
        {
            cmds.push(Command::SetGain {
                dir: sdroxide_types::Direction::Rx,
                element: Rx888Config::VGA_ELEMENT.into(),
                db: cfg.rx888.vga_db,
            });
        }
        ui.end_row();

        ui.label(tr!("settings-radio-attenuator"));
        if crate::chrome::slider(
            ui,
            egui::Slider::new(&mut cfg.rx888.attenuator_db, -31.5..=0.0).suffix(" dB"),
        )
        .on_hover_text(tr!("settings-radio-pe4304-step-attenuator-0-5"))
        .changed()
        {
            cmds.push(Command::SetGain {
                dir: sdroxide_types::Direction::Rx,
                element: Rx888Config::ATT_ELEMENT.into(),
                db: cfg.rx888.attenuator_db,
            });
        }
        ui.end_row();

        ui.label(tr!("settings-radio-adc-range"));
        if ui
            .checkbox(&mut cfg.rx888.pga, tr!("settings-radio-wide-2-25-vp-p"))
            .on_hover_text(tr!("settings-radio-selects-adc-s-wider-input"))
            .changed()
        {
            cmds.push(Command::SetGain {
                dir: sdroxide_types::Direction::Rx,
                element: Rx888Config::PGA_ELEMENT.into(),
                db: cfg.rx888.pga as u8 as f64,
            });
        }
        ui.end_row();

        ui.label(tr!("settings-radio-dither"));
        if ui
            .checkbox(&mut cfg.rx888.dither, tr!("settings-radio-enable"))
            .on_hover_text(tr!("settings-radio-adds-small-dither-signal-ahead"))
            .changed()
        {
            cmds.push(Command::SetGain {
                dir: sdroxide_types::Direction::Rx,
                element: Rx888Config::DITHER_ELEMENT.into(),
                db: cfg.rx888.dither as u8 as f64,
            });
        }
        ui.end_row();

        ui.label(tr!("settings-radio-randomiser"));
        crate::chrome::checkbox(ui, &mut cfg.rx888.randomize, tr!("settings-radio-enable"))
            .on_hover_text(tr!("settings-radio-adc-scrambles-output-so-digital"));
        ui.end_row();

        ui.label(tr!("settings-radio-bias-tee"));
        if ui
            .checkbox(&mut cfg.rx888.bias_tee_hf, tr!("settings-radio-dc-hf-antenna-port"))
            .on_hover_text(tr!("settings-radio-powers-active-antenna-preamp-down"))
            .changed()
        {
            cmds.push(Command::SetGain {
                dir: sdroxide_types::Direction::Rx,
                element: Rx888Config::BIAS_TEE_ELEMENT.into(),
                db: cfg.rx888.bias_tee_hf as u8 as f64,
            });
        }
        ui.end_row();

        ui.label(tr!("settings-radio-vhf-tuner-gain"));
        ui.horizontal(|ui| {
            let slider = crate::chrome::slider_enabled(
                ui,
                !cfg.rx888.tuner_agc,
                egui::Slider::new(
                    &mut cfg.rx888.tuner_gain_db,
                    0.0..=Rx888Config::TUNER_GAIN_MAX_DB,
                )
                .suffix(" dB"),
            );
            if slider.on_hover_text(tr!("settings-radio-r828d-rf-gain-used-above")).changed() {
                cmds.push(Command::SetGain {
                    dir: sdroxide_types::Direction::Rx,
                    element: Rx888Config::TUNER_GAIN_ELEMENT.into(),
                    db: cfg.rx888.tuner_gain_db,
                });
            }
            if ui
                .checkbox(&mut cfg.rx888.tuner_agc, tr!("settings-radio-auto"))
                .on_hover_text(tr!("settings-radio-let-tuner-run-own-lna"))
                .changed()
            {
                cmds.push(Command::SetGain {
                    dir: sdroxide_types::Direction::Rx,
                    element: Rx888Config::TUNER_AGC_ELEMENT.into(),
                    db: cfg.rx888.tuner_agc as u8 as f64,
                });
            }
        });
        ui.end_row();

        ui.label(tr!("settings-radio-bias-tee-vhf"));
        if ui
            .checkbox(&mut cfg.rx888.bias_tee_vhf, tr!("settings-radio-dc-vhf-antenna-port"))
            .on_hover_text(tr!("settings-radio-powers-active-antenna-preamp-down"))
            .changed()
        {
            cmds.push(Command::SetGain {
                dir: sdroxide_types::Direction::Rx,
                element: Rx888Config::BIAS_TEE_VHF_ELEMENT.into(),
                db: cfg.rx888.bias_tee_vhf as u8 as f64,
            });
        }
        ui.end_row();

        ui.label(tr!("settings-radio-clock-trim"));
        let r = ui
            .add(
                egui::DragValue::new(&mut cfg.rx888.ppm)
                    .speed(0.1)
                    .range(-200.0..=200.0)
                    .suffix(tr!("settings-radio-ppm")),
            )
            .on_hover_text(tr!("settings-radio-corrects-reference-oscillator-applied-when"));
        if r.drag_stopped() || r.lost_focus() {
            *apply = true;
        }
        ui.end_row();
    });

    if before != (cfg.rx888.serial.clone(), cfg.rx888.randomize, cfg.rx888.ddc_bins) {
        *apply = true;
    }

    ui.add_space(6.0);
    ui.label(egui::RichText::new(tr!("settings-radio-receive-only-below-adc-s")).weak());
}

/// Queue a receive-side gain (or pseudo-gain) change.
///
/// The Airspy HF+ panel drives seven of these — one real gain and six switches
/// riding `SetGain` so the backend needs no `Command` variants of its own — and
/// seven copies of the struct literal would bury the settings among them.
fn push_gain(cmds: &mut Vec<Command>, element: &str, db: f64) {
    cmds.push(Command::SetGain { dir: Direction::Rx, element: element.to_string(), db });
}

/// Airspy HF+ interface: receiver, rate, and the front end's own controls.
///
/// The rate list is the interesting part. Which rates an HF+ has depends on the
/// model *and* the firmware together, and only an opened receiver knows — so
/// once one is connected its own list is shown, and before that the union of
/// everything any HF+ is known to offer, annotated with who each one belongs to.
///
/// The report button is not decoration: this backend has never been run against
/// a real receiver, so the first people to use it are the ones who can say
/// whether it works.
#[allow(clippy::too_many_arguments)]
/// ELAD FDM-DUO / FDM-S1 / FDM-S2.
///
/// One tab for what is physically three USB devices, which is why it reaches
/// into three config blocks: `elad` for the receiver, `cat` for the
/// transceiver's serial control link (the same block the CAT / Audio interface
/// uses, so a DUO already working there keeps its port and baud rate), and — by
/// pointing at the General tab — `radio_audio_out` for transmit audio.
#[allow(clippy::too_many_arguments)]
pub(in crate::app) fn settings_elad_tab(
    ui: &mut egui::Ui,
    devices: &[sdroxide_types::EladDevice],
    serial_ports: &[String],
    // Which antenna socket the radio says it is receiving on. The rig's own
    // setting, read when the control port opens — not a config field, because
    // the radio remembers it across power cycles and a third copy of it here
    // could only ever disagree.
    antenna_rx: &str,
    audio_outputs: Option<&[String]>,
    radio_edit: &mut Option<sdroxide_types::RadioConfig>,
    rescan: &mut bool,
    copy_report: &mut bool,
    can_probe: bool,
    cmds: &mut Vec<Command>,
) {
    use sdroxide_types::{
        Direction, ELAD_ATTENUATOR_DB, ELAD_CAT_BAUDS, ELAD_DEFAULT_CAT_BAUD, ELAD_SAMPLE_RATES,
        EladAntenna, EladConfig, EladTxInput, ModeControl, PttMethod,
    };
    let Some(cfg) = radio_edit.as_mut() else {
        ui.label(tr!("settings-radio-waiting-configuration-machine-radio-attached"));
        return;
    };

    // What rebuilds the session. The rate is in here whichever model this is:
    // on a sampler it is the FPGA image to load, and on a transceiver the engine
    // still builds its whole downconversion chain around `IqSource::sample_rate`,
    // so reading the stream differently means opening the source again.
    let before = (
        cfg.elad.serial.clone(),
        cfg.elad.sample_rate_hz,
        cfg.cat.serial.path.clone(),
        cfg.cat.serial.baud,
    );

    egui::Grid::new("elad-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr!("settings-radio-device"));
        probe_only(ui, can_probe, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(tr!("settings-radio-rescan"))
                    .on_hover_text(tr!("settings-radio-re-list-usb-bus-no"))
                    .clicked()
                {
                    *rescan = true;
                }
                let shown = if cfg.elad.serial.is_empty() {
                    tr!("settings-radio-first-one-found")
                } else {
                    cfg.elad.serial.clone()
                };
                ComboBox::from_id_salt("elad_dev").width(300.0).selected_text(shown).show_styled(
                    ui,
                    |ui| {
                        if devices.is_empty() {
                            ui.label(
                                RichText::new(tr!("settings-radio-no-devices-press-rescan")).weak(),
                            );
                        }
                        ui.selectable_value(
                            &mut cfg.elad.serial,
                            String::new(),
                            tr!("settings-radio-first-one-found"),
                        );
                        // Nothing here can be pinned by serial: ELAD keeps the
                        // number in the device's EEPROM rather than in its USB
                        // descriptor, so listing it would mean claiming every
                        // device on the bus — including one that is streaming.
                        // The entries name the model and where it is plugged in.
                        for d in devices {
                            ui.label(RichText::new(d.label()).weak());
                        }
                    },
                );
            });
        });
        ui.end_row();

        ui.label(tr!("settings-radio-sample-rate"))
            .on_hover_text(tr!("settings-radio-six-rates-are-six-different"));
        ui.horizontal(|ui| {
            let shown = format!("{:.0} kHz", cfg.elad.sample_rate_hz as f64 / 1e3);
            ComboBox::from_id_salt("elad_rate").width(150.0).selected_text(shown).show_styled(
                ui,
                |ui| {
                    for r in ELAD_SAMPLE_RATES {
                        // The top rate is not just a wider window: the samples
                        // themselves are half as wide, so picking it wrongly
                        // produces noise rather than a mis-scaled spectrum.
                        let label = if r >= 6_144_000 {
                            tr!(
                                "settings-radio-elad-16-bit",
                                khz = format!("{:.0}", r as f64 / 1e3)
                            )
                        } else {
                            format!("{:.0} kHz", r as f64 / 1e3)
                        };
                        if ui.selectable_label(cfg.elad.sample_rate_hz == r, label).clicked() {
                            cfg.elad.sample_rate_hz = r;
                        }
                    }
                },
            );
            ui.add(
                egui::Label::new(
                    RichText::new(tr!("settings-radio-fdm-s-fpga-image-loaded")).weak(),
                )
                .wrap(),
            );
        });
        ui.end_row();

        ui.label(tr!("settings-radio-attenuator"))
            .on_hover_text(tr!("settings-radio-receiver-s-input-pad-one"));
        let mut att = cfg.elad.attenuator;
        if crate::chrome::checkbox(
            ui,
            &mut att,
            tr!("settings-radio-elad-pad", db = format!("{ELAD_ATTENUATOR_DB:.0}")),
        )
        .changed()
        {
            cfg.elad.attenuator = att;
            push_gain(cmds, EladConfig::ATT_ELEMENT, if att { -ELAD_ATTENUATOR_DB } else { 0.0 });
        }
        ui.end_row();

        ui.label(tr!("settings-radio-pre-selection-filters"))
            .on_hover_text(tr!("settings-radio-low-pass-bank-front-adc"));
        let mut lpf = cfg.elad.preselector;
        if crate::chrome::checkbox(ui, &mut lpf, tr!("settings-radio-filters-circuit")).changed() {
            cfg.elad.preselector = lpf;
            push_gain(cmds, EladConfig::LPF_ELEMENT, f64::from(u8::from(lpf)));
        }
        ui.end_row();
    });

    ui.add_space(8.0);
    ui.label(RichText::new(tr!("settings-radio-rig-control-fdm-duo-only")).strong());
    ui.label(RichText::new(tr!("settings-radio-transceiver-s-cat-usb-port")).weak());
    ui.label(RichText::new(tr!("settings-radio-port-set-radio-s-display")).weak());

    egui::Grid::new("elad-cat-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr!("settings-radio-serial-port"));
        let shown = if cfg.cat.serial.path.is_empty() {
            tr!("settings-radio-none-usb-control-only")
        } else {
            cfg.cat.serial.path.clone()
        };
        probe_only(ui, can_probe, |ui| {
            ComboBox::from_id_salt("elad_serport").width(260.0).selected_text(shown).show_styled(
                ui,
                |ui| {
                    ui.selectable_value(
                        &mut cfg.cat.serial.path,
                        String::new(),
                        tr!("settings-radio-none-usb-control-only"),
                    );
                    for p in serial_ports {
                        if ui.selectable_label(&cfg.cat.serial.path == p, p).clicked() {
                            cfg.cat.serial.path = p.clone();
                        }
                    }
                },
            );
        });
        ui.end_row();

        ui.label(tr!("settings-radio-baud"))
            .on_hover_text(tr!("settings-radio-must-match-menu-70-cat"));
        ui.horizontal(|ui| {
            ComboBox::from_id_salt("elad_baud")
                .selected_text(cfg.cat.serial.baud.to_string())
                .show_styled(ui, |ui| {
                    // The four the FDM-DUO's own menu offers, and only those:
                    // anything else is a rate the radio will not answer at.
                    for b in ELAD_CAT_BAUDS {
                        if ui.selectable_label(cfg.cat.serial.baud == b, b.to_string()).clicked() {
                            cfg.cat.serial.baud = b;
                        }
                    }
                });
            // Which the combo above can be *showing* without offering, because
            // this is the `cat` block the CAT / Audio interface uses and its own
            // default is 19200 — a rate no FDM-DUO has. A configuration that has
            // never had this field touched lands here, so it is worth a line on
            // screen rather than a number that looks set and is not.
            if !ELAD_CAT_BAUDS.contains(&cfg.cat.serial.baud) {
                ui.add(
                    egui::Label::new(
                        RichText::new(tr!(
                            "settings-radio-no-such-baud",
                            baud = cfg.cat.serial.baud,
                            default = ELAD_DEFAULT_CAT_BAUD
                        ))
                        .color(crate::theme::YELLOW()),
                    )
                    .wrap(),
                );
            }
        });
        ui.end_row();

        ui.label(tr!("settings-radio-ptt-method"))
            .on_hover_text(tr!("settings-radio-how-transmit-keyed-cat-sends"));
        enum_combo(ui, "elad_ptt", &mut cfg.cat.ptt, &PttMethod::ALL, PttMethod::label);
        ui.end_row();

        ui.label(tr!("settings-radio-transmit-input"))
            .on_hover_text(tr!("settings-radio-where-radio-takes-transmit-audio-2"));
        enum_combo(ui, "elad_txin", &mut cfg.cat.elad_tx_input, &EladTxInput::ALL, EladTxInput::tr);
        ui.end_row();

        ui.label(tr!("settings-radio-antenna"))
            .on_hover_text(tr!("settings-radio-which-two-sockets-back-receiver-2"));
        // Live state rather than a config field, so what is shown is where the
        // radio actually is. Blank until the rig has answered (or an operator
        // has chosen), which is exactly the moment nothing here is known.
        let shown = if antenna_rx.is_empty() { "—" } else { antenna_rx };
        ComboBox::from_id_salt("elad_antenna").selected_text(shown).show_styled(ui, |ui| {
            for a in EladAntenna::ALL {
                if ui.selectable_label(antenna_rx == a.label(), a.label()).clicked() {
                    cmds.push(Command::SetAntenna {
                        dir: Direction::Rx,
                        name: a.label().to_string(),
                    });
                }
            }
        });
        ui.end_row();

        ui.label(tr!("settings-radio-mode-control"));
        enum_combo(
            ui,
            "elad_modectl",
            &mut cfg.cat.mode_control,
            &ModeControl::ALL,
            ModeControl::tr,
        );
        ui.end_row();

        ui.label(tr!("settings-radio-poll-rate"))
            .on_hover_text(tr!("settings-radio-how-often-radio-asked-what-2"));
        ui.add(DragValue::new(&mut cfg.cat.poll_hz).speed(0.5).range(0.5..=20.0).suffix(" Hz"));
        ui.end_row();

        ui.label(tr!("settings-radio-transmit-audio"));
        match audio_outputs {
            Some(outs) => {
                let shown =
                    cfg.radio_audio_out.clone().unwrap_or_else(|| "— system default —".to_string());
                ComboBox::from_id_salt("elad_txaudio")
                    .width(300.0)
                    .selected_text(shown)
                    .show_styled(ui, |ui| {
                        ui.selectable_value(
                            &mut cfg.radio_audio_out,
                            None,
                            tr!("settings-radio-system-default"),
                        );
                        for o in outs {
                            ui.selectable_value(&mut cfg.radio_audio_out, Some(o.clone()), o);
                        }
                    })
                    .response
                    .on_hover_text(tr!("settings-radio-fdm-duo-s-own-usb"));
            }
            None => {
                ui.label(RichText::new(tr!("settings-radio-press-rescan-list-sound-cards")).weak());
            }
        }
        ui.end_row();

        ui.label("");
        probe_only(ui, can_probe, |ui| {
            if ui
                .button(tr!("settings-radio-copy-diagnostic-report"))
                .on_hover_text(tr!("settings-radio-copies-last-session-s-trace"))
                .clicked()
            {
                *copy_report = true;
            }
        });
        ui.end_row();
    });

    ui.add_space(4.0);
    ui.label(
        RichText::new(tr!("settings-radio-only-receiving-fdm-duo-has"))
            .color(crate::theme::YELLOW()),
    );
    ui.label(RichText::new(tr!("settings-radio-cw-keyed-radio-s-own")).weak());

    if (
        cfg.elad.serial.clone(),
        cfg.elad.sample_rate_hz,
        cfg.cat.serial.path.clone(),
        cfg.cat.serial.baud,
    ) != before
    {
        // No `apply` flag here: the interface row's own Apply is what reopens
        // the source, and these four are the settings that need it.
        ui.add_space(4.0);
        ui.label(RichText::new(tr!("settings-radio-press-apply-reopen-radio")).weak());
    }
}

pub(in crate::app) fn settings_airspyhf_tab(
    ui: &mut egui::Ui,
    devices: &[sdroxide_types::AirspyHfDevice],
    caps: Option<&sdroxide_types::DeviceCaps>,
    radio_edit: &mut Option<sdroxide_types::RadioConfig>,
    rescan: &mut bool,
    copy_report: &mut bool,
    apply: &mut bool,
    can_probe: bool,
    cmds: &mut Vec<Command>,
) {
    use sdroxide_types::AirspyHfConfig;
    let Some(cfg) = radio_edit.as_mut() else {
        ui.label(tr!("settings-radio-waiting-configuration-machine-radio-attached"));
        return;
    };

    // Only the receiver and the rate rebuild the session — the rate because the
    // engine builds its whole downconversion chain around it. Everything else
    // rides `SetGain` straight to the running device.
    let before = (cfg.airspyhf.serial.clone(), cfg.airspyhf.sample_rate_hz);

    // Once a receiver has been opened, its own list is the honest one. Before
    // that, offer everything any HF+ is known to do.
    let queried = caps
        .filter(|c| c.driver == "airspyhf" && !c.sample_rates.is_empty())
        .map(|c| c.sample_rates.as_slice());
    let rates = queried.unwrap_or(&AirspyHfConfig::SAMPLE_RATES);
    // The attenuator's range comes from the receiver too — the models differ.
    let att_max = caps
        .filter(|c| c.driver == "airspyhf")
        .and_then(|c| c.gains.first())
        .map(|g| -g.min_db)
        .unwrap_or(AirspyHfConfig::ATT_MAX_DB);
    let att_step = caps
        .filter(|c| c.driver == "airspyhf")
        .and_then(|c| c.gains.first())
        .map(|g| g.step_db)
        .unwrap_or(AirspyHfConfig::ATT_STEP_DB);

    egui::Grid::new("airspyhf-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr!("settings-radio-receiver"));
        // Which receiver is this panel's one row about a USB bus; everything
        // below reaches the device wherever it is plugged in.
        probe_only(ui, can_probe, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(tr!("settings-radio-rescan"))
                    .on_hover_text(tr!("settings-radio-re-list-usb-bus-no"))
                    .clicked()
                {
                    *rescan = true;
                }
                let shown = if cfg.airspyhf.serial.is_empty() {
                    tr!("settings-radio-first-one-found")
                } else {
                    cfg.airspyhf.serial.clone()
                };
                ComboBox::from_id_salt("airspyhf_dev")
                    .width(300.0)
                    .selected_text(shown)
                    .show_styled(ui, |ui| {
                        if devices.is_empty() {
                            ui.label(
                                RichText::new(tr!("settings-radio-no-receivers-press-rescan"))
                                    .weak(),
                            );
                        }
                        ui.selectable_value(
                            &mut cfg.airspyhf.serial,
                            String::new(),
                            tr!("settings-radio-first-one-found"),
                        );
                        for d in devices {
                            // Only a receiver whose serial parsed can be pinned;
                            // without one there is nothing stable to remember.
                            match &d.serial {
                                Some(sn) => {
                                    ui.selectable_value(
                                        &mut cfg.airspyhf.serial,
                                        sn.clone(),
                                        d.label(),
                                    );
                                }
                                None => {
                                    ui.label(RichText::new(d.label()).weak());
                                }
                            }
                        }
                    });
            });
        });
        ui.end_row();

        ui.label(tr!("settings-radio-sample-rate"))
            .on_hover_text(tr!("settings-radio-which-rates-receiver-has-depends"));
        ui.horizontal(|ui| {
            let shown = format!("{:.0} kSPS", cfg.airspyhf.sample_rate_hz / 1e3);
            ComboBox::from_id_salt("airspyhf_rate").width(150.0).selected_text(shown).show_styled(
                ui,
                |ui| {
                    for &r in rates {
                        let label = if queried.is_some() {
                            format!("{:.0} kSPS", r / 1e3)
                        } else {
                            format!("{:.0} kSPS  ({})", r / 1e3, AirspyHfConfig::rate_note(r))
                        };
                        if ui
                            .selectable_label((cfg.airspyhf.sample_rate_hz - r).abs() < 1.0, label)
                            .clicked()
                        {
                            cfg.airspyhf.sample_rate_hz = r;
                        }
                    }
                },
            );
            if queried.is_none() {
                // Inside a horizontal row a label defaults to Extend, which
                // pushes the row off the window edge instead of wrapping.
                ui.add(
                    egui::Label::new(
                        RichText::new(tr!("settings-radio-every-rate-any-hf-model")).weak(),
                    )
                    .wrap(),
                );
            }
        });
        ui.end_row();

        ui.label(tr!("settings-radio-agc"))
            .on_hover_text(tr!("settings-radio-receiver-s-own-gain-control"));
        let mut agc = cfg.airspyhf.agc;
        if crate::chrome::checkbox(ui, &mut agc, tr!("settings-radio-automatic")).changed() {
            cfg.airspyhf.agc = agc;
            push_gain(cmds, AirspyHfConfig::AGC_ELEMENT, f64::from(u8::from(agc)));
        }
        ui.end_row();

        ui.label(tr!("settings-radio-agc-threshold"));
        ui.add_enabled_ui(cfg.airspyhf.agc, |ui| {
            let mut high = cfg.airspyhf.agc_threshold_high;
            if ui
                .checkbox(&mut high, tr!("settings-radio-high"))
                .on_hover_text(tr!("settings-radio-high-trades-little-sensitivity-headroom"))
                .changed()
            {
                cfg.airspyhf.agc_threshold_high = high;
                push_gain(cmds, AirspyHfConfig::AGC_THRESHOLD_ELEMENT, f64::from(u8::from(high)));
            }
        });
        ui.end_row();

        ui.label(tr!("settings-radio-attenuator"))
            .on_hover_text(tr!("settings-radio-front-end-attenuation-gain-0"));
        ui.add_enabled_ui(!cfg.airspyhf.agc, |ui| {
            let mut db = cfg.airspyhf.attenuator_db;
            if crate::chrome::slider(
                ui,
                Slider::new(&mut db, -att_max..=0.0).step_by(att_step).suffix(" dB"),
            )
            .changed()
            {
                cfg.airspyhf.attenuator_db = db;
                push_gain(cmds, AirspyHfConfig::ATT_ELEMENT, db);
            }
        });
        ui.end_row();

        ui.label(tr!("settings-radio-preamp"))
            .on_hover_text(tr!("settings-radio-hf-low-noise-amplifier-buys"));
        let mut lna = cfg.airspyhf.lna;
        if crate::chrome::checkbox(ui, &mut lna, tr!("settings-radio-lna")).changed() {
            cfg.airspyhf.lna = lna;
            push_gain(cmds, AirspyHfConfig::LNA_ELEMENT, f64::from(u8::from(lna)));
        }
        ui.end_row();

        ui.label(tr!("settings-radio-frequency-calibration"))
            .on_hover_text(tr!("settings-radio-parts-per-billion-receiver-s"));
        ui.horizontal(|ui| {
            let mut stored = cfg.airspyhf.calibration_ppb.is_none();
            if crate::chrome::checkbox(
                ui,
                &mut stored,
                tr!("settings-radio-use-receiver-s-stored-value"),
            )
            .changed()
            {
                cfg.airspyhf.calibration_ppb = if stored { None } else { Some(0) };
                if let Some(ppb) = cfg.airspyhf.calibration_ppb {
                    push_gain(cmds, AirspyHfConfig::PPB_ELEMENT, ppb as f64);
                } else {
                    // Back to the receiver's own figure needs a reopen: the
                    // flash value is only read when the device is opened.
                    *apply = true;
                }
            }
            if let Some(ppb) = cfg.airspyhf.calibration_ppb.as_mut()
                && ui.add(DragValue::new(ppb).speed(10).suffix(tr!("settings-radio-ppb"))).changed()
            {
                push_gain(cmds, AirspyHfConfig::PPB_ELEMENT, *ppb as f64);
            }
        });
        ui.end_row();

        ui.label(tr!("settings-radio-bias-tee"));
        let mut bias = cfg.airspyhf.bias_tee;
        if ui
            .checkbox(&mut bias, tr!("settings-radio-feed-dc-up-coax"))
            .on_hover_text(tr!("settings-radio-not-every-hf-has-one"))
            .changed()
        {
            cfg.airspyhf.bias_tee = bias;
            push_gain(cmds, AirspyHfConfig::BIAS_TEE_ELEMENT, f64::from(u8::from(bias)));
        }
        ui.end_row();

        ui.label(tr!("settings-radio-host-dsp"))
            .on_hover_text(tr!("settings-radio-image-balancer-zero-if-offset"));
        let mut dsp = cfg.airspyhf.lib_dsp;
        if crate::chrome::checkbox(ui, &mut dsp, tr!("settings-radio-correct-image-fine-tune"))
            .changed()
        {
            cfg.airspyhf.lib_dsp = dsp;
            push_gain(cmds, AirspyHfConfig::LIB_DSP_ELEMENT, f64::from(u8::from(dsp)));
        }
        ui.end_row();

        ui.label("");
        // The trace is of the session *this* process ran; the engine's own is
        // on the engine's machine.
        probe_only(ui, can_probe, |ui| {
            if ui
                .button(tr!("settings-radio-copy-diagnostic-report"))
                .on_hover_text(tr!("settings-radio-copies-last-session-s-trace-2"))
                .clicked()
            {
                *copy_report = true;
            }
        });
        ui.end_row();
    });

    if cfg.airspyhf.bias_tee {
        ui.add_space(4.0);
        ui.label(
            RichText::new(tr!("settings-radio-bias-tee-never-connect-transceiver"))
                .color(crate::theme::YELLOW()),
        );
    }

    if (cfg.airspyhf.serial.clone(), cfg.airspyhf.sample_rate_hz) != before {
        *apply = true;
    }

    ui.add_space(4.0);
    ui.label(RichText::new(tr!("settings-radio-receive-only-no-soapysdr-no")).weak());
    ui.label(
        RichText::new(tr!("settings-radio-not-yet-verified-against-real-3"))
            .color(crate::theme::YELLOW()),
    );
}

/// Airspy R2 / Mini interface: receiver, rate, and the tuner's gain curves.
///
/// The interesting part is the rate list. An R2 offers 10 and 2.5 Msps and a
/// Mini 6 and 3, and the two are indistinguishable on the USB bus — same
/// product id, same product string. So once a receiver is connected its own
/// list is shown, and before that the union of both, annotated with which model
/// each rate belongs to.
#[allow(clippy::too_many_arguments)]
pub(in crate::app) fn settings_airspy_tab(
    ui: &mut egui::Ui,
    devices: &[sdroxide_types::AirspyDevice],
    caps: Option<&sdroxide_types::DeviceCaps>,
    radio_edit: &mut Option<sdroxide_types::RadioConfig>,
    rescan: &mut bool,
    copy_report: &mut bool,
    apply: &mut bool,
    can_probe: bool,
    cmds: &mut Vec<Command>,
) {
    use sdroxide_types::{AirspyConfig, AirspyGain};
    let Some(cfg) = radio_edit.as_mut() else {
        ui.label(tr!("settings-radio-waiting-configuration-machine-radio-attached"));
        return;
    };

    // What cannot change under a running stream. The rate stops and restarts
    // the receiver, and packing decides how every completion is decoded.
    let before = (cfg.airspy.serial.clone(), cfg.airspy.sample_rate_hz, cfg.airspy.packing);

    // The receiver's own rates when one is connected, the union of both models'
    // before that.
    let rates: Vec<f64> = match caps {
        Some(c) if !c.sample_rates.is_empty() => c.sample_rates.clone(),
        _ => AirspyConfig::SAMPLE_RATES.to_vec(),
    };
    let from_device = caps.is_some_and(|c| !c.sample_rates.is_empty());

    egui::Grid::new("airspy-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr!("settings-radio-receiver"));
        probe_only(ui, can_probe, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(tr!("settings-radio-rescan"))
                    .on_hover_text(tr!("settings-radio-re-list-usb-bus-no"))
                    .clicked()
                {
                    *rescan = true;
                }
                let shown = if cfg.airspy.serial.is_empty() {
                    tr!("settings-radio-first-one-found")
                } else {
                    cfg.airspy.serial.clone()
                };
                ComboBox::from_id_salt("airspy_dev").width(300.0).selected_text(shown).show_styled(
                    ui,
                    |ui| {
                        if devices.is_empty() {
                            ui.label(tr!("settings-radio-no-airspy-r2-mini-found"));
                        }
                        ui.selectable_value(
                            &mut cfg.airspy.serial,
                            String::new(),
                            tr!("settings-radio-first-one-found"),
                        );
                        for d in devices {
                            let serial = d.serial.clone().unwrap_or_default();
                            ui.selectable_value(&mut cfg.airspy.serial, serial, d.label());
                        }
                    },
                );
            });
        });
        ui.end_row();

        ui.label(tr!("settings-radio-sample-rate"));
        ui.horizontal(|ui| {
            ComboBox::from_id_salt("airspy_rate")
                .width(180.0)
                .selected_text(format!("{:.3} Msps", cfg.airspy.sample_rate_hz / 1e6))
                .show_styled(ui, |ui| {
                    for r in &rates {
                        let note = AirspyConfig::rate_note(*r);
                        let text = if from_device || note.is_empty() {
                            format!("{:.3} Msps", r / 1e6)
                        } else {
                            format!("{:.3} Msps — {note}", r / 1e6)
                        };
                        ui.selectable_value(&mut cfg.airspy.sample_rate_hz, *r, text);
                    }
                });
            ui.add(
                egui::Label::new(
                    RichText::new(if from_device {
                        tr!("settings-radio-read-receiver")
                    } else {
                        tr!("settings-radio-r2-mini-offer-different-rates")
                    })
                    .weak(),
                )
                .wrap(),
            );
        });
        ui.end_row();
        ui.label("");
        ui.add(
            egui::Label::new(RichText::new(tr!("settings-radio-rate-you-get-receiver-s")).weak())
                .wrap(),
        );
        ui.end_row();

        ui.label(tr!("settings-radio-gain-curve"));
        ui.horizontal(|ui| {
            for c in AirspyGain::ALL {
                if ui.selectable_label(cfg.airspy.gain_curve == c, c.tr()).clicked()
                    && cfg.airspy.gain_curve != c
                {
                    cfg.airspy.gain_curve = c;
                    push_gain(cmds, AirspyConfig::CURVE_ELEMENT, c.code() as f64);
                }
            }
        });
        ui.end_row();

        ui.label(tr!("settings-radio-gain"));
        if crate::chrome::slider(
            ui,
            egui::Slider::new(&mut cfg.airspy.gain_step, 0..=(AirspyConfig::GAIN_STEPS - 1))
                .text("step"),
        )
        .on_hover_text(tr!("settings-radio-step-along-curve-above-not"))
        .changed()
        {
            push_gain(cmds, AirspyConfig::GAIN_ELEMENT, cfg.airspy.gain_step as f64);
        }
        ui.end_row();

        ui.label(tr!("settings-radio-tuner-agc"));
        ui.horizontal(|ui| {
            if ui
                .checkbox(&mut cfg.airspy.lna_agc, tr!("settings-radio-lna-2"))
                .on_hover_text(tr!("settings-radio-hands-lna-tuner-s-own"))
                .changed()
            {
                push_gain(cmds, AirspyConfig::LNA_AGC_ELEMENT, cfg.airspy.lna_agc as u8 as f64);
            }
            if ui
                .checkbox(&mut cfg.airspy.mixer_agc, tr!("settings-radio-mixer"))
                .on_hover_text(tr!("settings-radio-same-mixer-stage"))
                .changed()
            {
                push_gain(cmds, AirspyConfig::MIXER_AGC_ELEMENT, cfg.airspy.mixer_agc as u8 as f64);
            }
        });
        ui.end_row();
        if cfg.airspy.lna_agc || cfg.airspy.mixer_agc {
            ui.label("");
            ui.add(
                egui::Label::new(
                    RichText::new(tr!("settings-radio-loop-running-gain-slider-no")).weak(),
                )
                .wrap(),
            );
            ui.end_row();
        }

        ui.label(tr!("settings-radio-bias-tee"));
        if ui
            .checkbox(&mut cfg.airspy.bias_tee, tr!("settings-radio-dc-antenna-port"))
            .on_hover_text(tr!("settings-radio-powers-active-antenna-preamp-down"))
            .changed()
        {
            push_gain(cmds, AirspyConfig::BIAS_TEE_ELEMENT, cfg.airspy.bias_tee as u8 as f64);
        }
        ui.end_row();

        ui.label(tr!("settings-radio-t-12-bit-packing"));
        ui.horizontal(|ui| {
            crate::chrome::checkbox(ui, &mut cfg.airspy.packing, tr!("settings-radio-enable"));
            ui.add(
                egui::Label::new(
                    RichText::new(tr!("settings-radio-third-less-usb-traffic-leave")).weak(),
                )
                .wrap(),
            );
        });
        ui.end_row();

        ui.label(tr!("settings-radio-dc-removal"));
        if ui
            .checkbox(&mut cfg.airspy.dc_block, tr!("settings-radio-remove-adc-s-offset"))
            .on_hover_text(tr!("settings-radio-turn-off-see-raw-hardware"))
            .changed()
        {
            push_gain(cmds, AirspyConfig::DC_BLOCK_ELEMENT, cfg.airspy.dc_block as u8 as f64);
        }
        ui.end_row();
    });

    if cfg.airspy.bias_tee {
        ui.add_space(4.0);
        ui.label(
            RichText::new(tr!("settings-radio-bias-tee-never-connect-transceiver"))
                .color(crate::theme::YELLOW()),
        );
    }

    if (cfg.airspy.serial.clone(), cfg.airspy.sample_rate_hz, cfg.airspy.packing) != before {
        *apply = true;
    }

    ui.add_space(6.0);
    probe_only(ui, can_probe, |ui| {
        if ui
            .button(tr!("settings-radio-copy-diagnostic-report"))
            .on_hover_text(tr!("settings-radio-every-command-exchanged-receiver-sample"))
            .clicked()
        {
            *copy_report = true;
        }
    });

    ui.add_space(4.0);
    ui.label(RichText::new(tr!("settings-radio-receive-only-24-1800-mhz")).weak());
    ui.label(
        RichText::new(tr!("settings-radio-not-yet-verified-against-real-4"))
            .color(crate::theme::YELLOW()),
    );
}

/// HydraSDR RFOne interface: receiver, rate, the RF socket, and the tuner's
/// gain curves.
///
/// Two things here that the Airspy panel next door does not have, and both are
/// this radio's own. **Three RF sockets**, only one of which has the bias tee
/// behind it — so the two controls are tied together rather than left to
/// contradict each other. And **a rate list with a catch**: the receiver
/// reports three of its seven rates and says nothing about the four in the
/// firmware's alternate table, so the menu covers both and marks which is
/// which. An alternate an older firmware turns out not to have falls back to a
/// listed rate at open, and the panel says so once a receiver is connected.
#[allow(clippy::too_many_arguments)]
pub(in crate::app) fn settings_hydrasdr_tab(
    ui: &mut egui::Ui,
    devices: &[sdroxide_types::HydraSdrDevice],
    caps: Option<&sdroxide_types::DeviceCaps>,
    radio_edit: &mut Option<sdroxide_types::RadioConfig>,
    rescan: &mut bool,
    copy_report: &mut bool,
    apply: &mut bool,
    can_probe: bool,
    cmds: &mut Vec<Command>,
) {
    use sdroxide_types::{HydraSdrConfig, HydraSdrGain, HydraSdrPort};
    let Some(cfg) = radio_edit.as_mut() else {
        ui.label(tr!("settings-radio-waiting-configuration-machine-radio-attached"));
        return;
    };

    // What cannot change under a running stream. The rate stops and restarts
    // the receiver, and packing decides how every completion is decoded.
    let before = (cfg.hydrasdr.serial.clone(), cfg.hydrasdr.sample_rate_hz, cfg.hydrasdr.packing);

    // The rates this particular board turned out to have once one is connected,
    // the full seven before that.
    let rates: Vec<f64> = match caps {
        Some(c) if c.driver == "hydrasdr" && !c.sample_rates.is_empty() => c.sample_rates.clone(),
        _ => HydraSdrConfig::SAMPLE_RATES.to_vec(),
    };
    let from_device = caps.is_some_and(|c| c.driver == "hydrasdr" && !c.sample_rates.is_empty());

    egui::Grid::new("hydrasdr-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr!("settings-radio-receiver"));
        probe_only(ui, can_probe, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(tr!("settings-radio-rescan"))
                    .on_hover_text(tr!("settings-radio-re-list-usb-bus-no"))
                    .clicked()
                {
                    *rescan = true;
                }
                let shown = if cfg.hydrasdr.serial.is_empty() {
                    tr!("settings-radio-first-one-found")
                } else {
                    cfg.hydrasdr.serial.clone()
                };
                ComboBox::from_id_salt("hydrasdr_dev")
                    .width(340.0)
                    .selected_text(shown)
                    .show_styled(ui, |ui| {
                        if devices.is_empty() {
                            ui.label(tr!("settings-radio-no-hydrasdr-rfone-found-press"));
                        }
                        ui.selectable_value(
                            &mut cfg.hydrasdr.serial,
                            String::new(),
                            tr!("settings-radio-first-one-found"),
                        );
                        for d in devices {
                            let serial = d.serial.clone().unwrap_or_default();
                            ui.selectable_value(&mut cfg.hydrasdr.serial, serial, d.label());
                        }
                    });
            });
        });
        ui.end_row();

        ui.label(tr!("settings-radio-sample-rate"));
        ui.horizontal(|ui| {
            ComboBox::from_id_salt("hydrasdr_rate")
                .width(180.0)
                .selected_text(format!("{:.3} Msps", cfg.hydrasdr.sample_rate_hz / 1e6))
                .show_styled(ui, |ui| {
                    for r in &rates {
                        let note = HydraSdrConfig::rate_note(*r);
                        let text = if note.is_empty() {
                            format!("{:.3} Msps", r / 1e6)
                        } else {
                            format!("{:.3} Msps — {note}", r / 1e6)
                        };
                        ui.selectable_value(&mut cfg.hydrasdr.sample_rate_hz, *r, text);
                    }
                });
            ui.add(
                egui::Label::new(
                    RichText::new(if from_device {
                        tr!("settings-radio-these-are-rates-receiver-turned")
                    } else {
                        tr!("settings-radio-receiver-only-reports-three-these")
                    })
                    .weak(),
                )
                .wrap(),
            );
        });
        ui.end_row();
        ui.label("");
        ui.add(
            egui::Label::new(RichText::new(tr!("settings-radio-rate-you-get-receiver-s")).weak())
                .wrap(),
        );
        ui.end_row();

        ui.label(tr!("settings-radio-rf-input"));
        ui.horizontal(|ui| {
            for p in HydraSdrPort::ALL {
                let hover = if p.has_bias_tee() {
                    tr!("settings-radio-antenna-sma-only-socket-bias")
                } else {
                    tr!("settings-radio-cable-socket-no-bias-tee")
                };
                if ui
                    .selectable_label(cfg.hydrasdr.rf_port == p, p.name())
                    .on_hover_text(hover)
                    .clicked()
                    && cfg.hydrasdr.rf_port != p
                {
                    cfg.hydrasdr.rf_port = p;
                    push_gain(cmds, HydraSdrConfig::RF_PORT_ELEMENT, p.code() as f64);
                    // The bias tee belongs to ANT alone, and the driver drops
                    // it on the way to a cable port. Following that here keeps
                    // the switch from claiming DC is on a socket that has none.
                    if !p.has_bias_tee() && cfg.hydrasdr.bias_tee {
                        cfg.hydrasdr.bias_tee = false;
                        push_gain(cmds, HydraSdrConfig::BIAS_TEE_ELEMENT, 0.0);
                    }
                }
            }
        });
        ui.end_row();

        ui.label(tr!("settings-radio-gain-curve"));
        ui.horizontal(|ui| {
            for c in HydraSdrGain::ALL {
                if ui.selectable_label(cfg.hydrasdr.gain_curve == c, c.tr()).clicked()
                    && cfg.hydrasdr.gain_curve != c
                {
                    cfg.hydrasdr.gain_curve = c;
                    push_gain(cmds, HydraSdrConfig::CURVE_ELEMENT, c.code() as f64);
                }
            }
        });
        ui.end_row();

        ui.label(tr!("settings-radio-gain"));
        if crate::chrome::slider(
            ui,
            egui::Slider::new(&mut cfg.hydrasdr.gain_step, 0..=(HydraSdrConfig::GAIN_STEPS - 1))
                .text("step"),
        )
        .on_hover_text(tr!("settings-radio-step-along-curve-above-not"))
        .changed()
        {
            push_gain(cmds, HydraSdrConfig::GAIN_ELEMENT, cfg.hydrasdr.gain_step as f64);
        }
        ui.end_row();

        ui.label(tr!("settings-radio-tuner-agc"));
        ui.horizontal(|ui| {
            if ui
                .checkbox(&mut cfg.hydrasdr.lna_agc, tr!("settings-radio-lna-2"))
                .on_hover_text(tr!("settings-radio-hands-lna-tuner-s-own"))
                .changed()
            {
                push_gain(cmds, HydraSdrConfig::LNA_AGC_ELEMENT, cfg.hydrasdr.lna_agc as u8 as f64);
            }
            if ui
                .checkbox(&mut cfg.hydrasdr.mixer_agc, tr!("settings-radio-mixer"))
                .on_hover_text(tr!("settings-radio-same-mixer-stage"))
                .changed()
            {
                push_gain(
                    cmds,
                    HydraSdrConfig::MIXER_AGC_ELEMENT,
                    cfg.hydrasdr.mixer_agc as u8 as f64,
                );
            }
        });
        ui.end_row();
        if cfg.hydrasdr.lna_agc || cfg.hydrasdr.mixer_agc {
            ui.label("");
            ui.add(
                egui::Label::new(
                    RichText::new(tr!("settings-radio-loop-running-gain-slider-no")).weak(),
                )
                .wrap(),
            );
            ui.end_row();
        }

        ui.label(tr!("settings-radio-bias-tee"));
        ui.add_enabled_ui(cfg.hydrasdr.rf_port.has_bias_tee(), |ui| {
            if ui
                .checkbox(&mut cfg.hydrasdr.bias_tee, tr!("settings-radio-dc-antenna-port"))
                .on_hover_text(tr!("settings-radio-powers-active-antenna-preamp-down-2"))
                .changed()
            {
                push_gain(
                    cmds,
                    HydraSdrConfig::BIAS_TEE_ELEMENT,
                    cfg.hydrasdr.bias_tee as u8 as f64,
                );
            }
        });
        ui.end_row();

        ui.label(tr!("settings-radio-t-12-bit-packing"));
        ui.horizontal(|ui| {
            crate::chrome::checkbox(ui, &mut cfg.hydrasdr.packing, tr!("settings-radio-enable"));
            ui.add(
                egui::Label::new(
                    RichText::new(tr!("settings-radio-third-less-usb-traffic-leave-2")).weak(),
                )
                .wrap(),
            );
        });
        ui.end_row();

        ui.label(tr!("settings-radio-dc-removal"));
        if ui
            .checkbox(&mut cfg.hydrasdr.dc_block, tr!("settings-radio-remove-adc-s-offset"))
            .on_hover_text(tr!("settings-radio-turn-off-see-raw-hardware"))
            .changed()
        {
            push_gain(cmds, HydraSdrConfig::DC_BLOCK_ELEMENT, cfg.hydrasdr.dc_block as u8 as f64);
        }
        ui.end_row();
    });

    if cfg.hydrasdr.bias_tee && cfg.hydrasdr.rf_port.has_bias_tee() {
        ui.add_space(4.0);
        ui.label(
            RichText::new(tr!("settings-radio-bias-tee-never-connect-transceiver"))
                .color(crate::theme::YELLOW()),
        );
    }

    if devices.iter().any(|d| d.legacy_usb_id) {
        ui.add_space(4.0);
        ui.label(RichText::new(tr!("settings-radio-one-these-boards-1d50-60a1")).weak());
    }

    if (cfg.hydrasdr.serial.clone(), cfg.hydrasdr.sample_rate_hz, cfg.hydrasdr.packing) != before {
        *apply = true;
    }

    ui.add_space(6.0);
    probe_only(ui, can_probe, |ui| {
        if ui
            .button(tr!("settings-radio-copy-diagnostic-report"))
            .on_hover_text(tr!("settings-radio-every-command-exchanged-receiver-sample"))
            .clicked()
        {
            *copy_report = true;
        }
    });

    ui.add_space(4.0);
    ui.label(RichText::new(tr!("settings-radio-receive-only-24-1800-mhz-2")).weak());
    ui.label(
        RichText::new(tr!("settings-radio-not-yet-verified-against-real-4"))
            .color(crate::theme::YELLOW()),
    );
}

/// Fobos SDR interface: receiver, input, sample rate, gain, clock source.
///
/// The receiver, input and sample rate all reopen the device rather than
/// applying live: the input because `Rf` and the HF ports are different
/// hardware paths (`fobos_rx_set_direct_sampling`) and the HF ports build a
/// software downconverter at open time, the rate because `IqSource` has no
/// live rate setter to carry one — the driver crate has the stop/reconfigure/
/// restart machinery (`sdroxide_fobos::handle::Ctrl::Rate`), but nothing
/// short of a reopen reaches it, so this is a reopen like the other two and
/// not a live control that happens to be slow. LNA/VGA gain and the clock
/// source do apply immediately — the gain sliders only doing anything on
/// `Rf`, see [`sdroxide_types::FobosPort`]'s own doc comment for why.
#[allow(clippy::too_many_arguments)]
pub(in crate::app) fn settings_fobos_tab(
    ui: &mut egui::Ui,
    devices: &[sdroxide_types::FobosDevice],
    caps: Option<&sdroxide_types::DeviceCaps>,
    radio_edit: &mut Option<sdroxide_types::RadioConfig>,
    rescan: &mut bool,
    apply: &mut bool,
    can_probe: bool,
    cmds: &mut Vec<Command>,
) {
    use sdroxide_types::{FobosConfig, FobosPort, diversity_cost_note};
    let Some(cfg) = radio_edit.as_mut() else {
        ui.label(tr!("settings-radio-waiting-configuration-machine-radio-attached"));
        return;
    };

    let before = (cfg.fobos.serial.clone(), cfg.fobos.port, cfg.fobos.sample_rate_hz);

    // The rates this particular unit turned out to have once one is
    // connected, else a list measured on the one real unit this backend was
    // verified against — see `FobosConfig::SAMPLE_RATES`'s own doc comment.
    let rates: Vec<f64> = match caps {
        Some(c) if c.driver == "fobos" && !c.sample_rates.is_empty() => c.sample_rates.clone(),
        _ => FobosConfig::SAMPLE_RATES.to_vec(),
    };
    let from_device = caps.is_some_and(|c| c.driver == "fobos" && !c.sample_rates.is_empty());

    egui::Grid::new("fobos-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr!("settings-radio-receiver"));
        probe_only(ui, can_probe, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(tr!("settings-radio-rescan"))
                    .on_hover_text(tr!("settings-radio-re-list-fobos-sdrs-no"))
                    .clicked()
                {
                    *rescan = true;
                }
                let shown = if cfg.fobos.serial.is_empty() {
                    tr!("settings-radio-first-one-found")
                } else {
                    cfg.fobos.serial.clone()
                };
                ComboBox::from_id_salt("fobos_dev").width(300.0).selected_text(shown).show_styled(
                    ui,
                    |ui| {
                        if devices.is_empty() {
                            ui.label(tr!("settings-radio-no-fobos-sdr-found-press"));
                        }
                        ui.selectable_value(
                            &mut cfg.fobos.serial,
                            String::new(),
                            tr!("settings-radio-first-one-found"),
                        );
                        for d in devices {
                            ui.selectable_value(&mut cfg.fobos.serial, d.serial.clone(), d.label());
                        }
                    },
                );
            });
        });
        ui.end_row();

        ui.label(tr!("settings-radio-input-2"));
        ui.horizontal(|ui| {
            for p in FobosPort::ALL {
                let hover = match p {
                    FobosPort::Rf => {
                        tr!("settings-radio-through-tuner-lna-vga-mixer")
                    }
                    FobosPort::Hf1 => {
                        tr!("settings-radio-direct-sampling-hf1-antenna-input")
                    }
                    FobosPort::Hf2 => tr!("settings-radio-direct-sampling-hf2-antenna-input"),
                    FobosPort::HfDual => {
                        tr!("settings-radio-both-real-adc-channels-once")
                    }
                };
                ui.selectable_value(&mut cfg.fobos.port, p, p.name()).on_hover_text(hover);
            }
        });
        ui.end_row();
        if cfg.fobos.port != FobosPort::Rf {
            ui.label("");
            ui.add(
                egui::Label::new(
                    RichText::new(tr!("settings-radio-direct-sampling-tuner-so-lna")).weak(),
                )
                .wrap(),
            );
            ui.end_row();
        }

        ui.label(tr!("settings-radio-sample-rate"));
        ui.horizontal(|ui| {
            ComboBox::from_id_salt("fobos_rate")
                .width(150.0)
                .selected_text(format!("{:.3} Msps", cfg.fobos.sample_rate_hz / 1e6))
                .show_styled(ui, |ui| {
                    for r in &rates {
                        ui.selectable_value(
                            &mut cfg.fobos.sample_rate_hz,
                            *r,
                            format!("{:.3} Msps", r / 1e6),
                        );
                    }
                });
            ui.add(
                egui::Label::new(
                    RichText::new(if from_device {
                        tr!("settings-radio-receiver-s-own-reported-rates")
                    } else {
                        tr!("settings-radio-measured-one-real-unit-connect")
                    })
                    .weak(),
                )
                .wrap(),
            );
        });
        ui.end_row();

        ui.label(tr!("settings-radio-lna-gain"));
        ui.add_enabled_ui(cfg.fobos.port == FobosPort::Rf, |ui| {
            if crate::chrome::slider(
                ui,
                egui::Slider::new(&mut cfg.fobos.lna_gain, 0..=FobosConfig::LNA_GAIN_MAX),
            )
            .changed()
            {
                push_gain(cmds, FobosConfig::LNA_GAIN_ELEMENT, cfg.fobos.lna_gain as f64);
            }
        });
        ui.end_row();

        ui.label(tr!("settings-radio-vga-gain"));
        ui.add_enabled_ui(cfg.fobos.port == FobosPort::Rf, |ui| {
            if crate::chrome::slider(
                ui,
                egui::Slider::new(&mut cfg.fobos.vga_gain, 0..=FobosConfig::VGA_GAIN_MAX),
            )
            .changed()
            {
                push_gain(cmds, FobosConfig::VGA_GAIN_ELEMENT, cfg.fobos.vga_gain as f64);
            }
        });
        ui.end_row();

        ui.label(tr!("settings-radio-clock-source"));
        if ui
            .checkbox(&mut cfg.fobos.clk_external, tr!("settings-radio-external-reference"))
            .changed()
        {
            push_gain(cmds, FobosConfig::CLK_EXTERNAL_ELEMENT, cfg.fobos.clk_external as u8 as f64);
        }
        ui.end_row();

        // Which way it combines, how fast it chases, and holding it are on
        // the main window's own DIV strip once this radio is selected (see
        // `sdroxide_types::DIV_MODE_ELEMENT`'s own doc comment) — filter
        // length is the one control that stays here, the same split every
        // other diversity-capable backend's settings tab uses.
        if cfg.fobos.port == FobosPort::HfDual {
            ui.label(tr!("settings-radio-diversity-taps"));
            ui.horizontal(|ui| {
                if ui
                    .add(
                        DragValue::new(&mut cfg.fobos.div_taps)
                            .speed(1.0)
                            .range(1..=FobosConfig::DIV_TAPS_MAX)
                            .suffix(tr!("settings-radio-taps")),
                    )
                    .on_hover_text(tr!("settings-radio-one-tap-gain-phase-null"))
                    .changed()
                {
                    push_gain(
                        cmds,
                        sdroxide_types::DIV_TAPS_ELEMENT,
                        f64::from(cfg.fobos.div_taps),
                    );
                }
                ui.label(
                    RichText::new(diversity_cost_note(
                        cfg.fobos.div_taps,
                        cfg.fobos.sample_rate_hz,
                    ))
                    .weak(),
                );
            });
            ui.end_row();
        }
    });

    if (cfg.fobos.serial.clone(), cfg.fobos.port, cfg.fobos.sample_rate_hz) != before {
        *apply = true;
    }

    ui.add_space(6.0);
    ui.label(RichText::new(tr!("settings-radio-receive-only-rf-port-tuner")).weak());
}

/// HackRF interface: radio, rate, the front end, and — behind its own switch —
/// the transmitter.
///
/// The transmit group is separated and defaults to off on purpose. This is the
/// only native USB backend here that can key up, it is a wideband transmitter
/// with poor harmonic suppression, and somebody who plugged one in to listen
/// should not be one PTT away from radiating.
#[allow(clippy::too_many_arguments)]
pub(in crate::app) fn settings_hackrf_tab(
    ui: &mut egui::Ui,
    devices: &[sdroxide_types::HackRfDevice],
    radio_edit: &mut Option<sdroxide_types::RadioConfig>,
    rescan: &mut bool,
    copy_report: &mut bool,
    apply: &mut bool,
    can_probe: bool,
    cmds: &mut Vec<Command>,
) {
    use sdroxide_types::HackRfConfig;
    let Some(cfg) = radio_edit.as_mut() else {
        ui.label(tr!("settings-radio-waiting-configuration-machine-radio-attached"));
        return;
    };

    // What cannot be changed under a running stream. The rate re-programmes the
    // clock generator and the baseband filter together, and arming the
    // transmitter changes how many channels the backend publishes — which is
    // what the engine's transmit gate reads.
    let before = (
        cfg.hackrf.serial.clone(),
        cfg.hackrf.sample_rate_hz,
        cfg.hackrf.tx_enabled,
        cfg.hackrf.transfers,
        cfg.hackrf.transfer_kib,
    );

    // Which board is selected decides two rows below: a HackRF Pro takes rates
    // no other HackRF can run, and it picks its own baseband filter. Read off
    // the *selected* device rather than the open one, so the menu is right
    // before Apply as well as after — and off the USB product string, because
    // the Pro and the One share a product id. With nothing enumerated yet this
    // is false, which offers the conservative menu; a Pro owner sees their
    // extra rates as soon as the list arrives.
    // Matched on the suffix, the same rule the driver opens by, so a serial
    // typed by hand into `radio.json` selects the same radio here as there.
    let is_pro =
        devices.iter().find(|d| d.matches_serial(&cfg.hackrf.serial)).is_some_and(|d| d.is_pro());

    egui::Grid::new("hackrf-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr!("settings-radio-radio"));
        probe_only(ui, can_probe, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(tr!("settings-radio-rescan"))
                    .on_hover_text(tr!("settings-radio-re-list-usb-bus-no"))
                    .clicked()
                {
                    *rescan = true;
                }
                let shown = if cfg.hackrf.serial.is_empty() {
                    tr!("settings-radio-first-one-found")
                } else {
                    cfg.hackrf.serial.clone()
                };
                ComboBox::from_id_salt("hackrf_dev").width(300.0).selected_text(shown).show_styled(
                    ui,
                    |ui| {
                        if devices.is_empty() {
                            ui.label(tr!("settings-radio-no-hackrf-found-press-rescan"));
                        }
                        ui.selectable_value(
                            &mut cfg.hackrf.serial,
                            String::new(),
                            tr!("settings-radio-first-one-found"),
                        );
                        for d in devices {
                            let serial = d.serial.clone().unwrap_or_default();
                            ui.selectable_value(&mut cfg.hackrf.serial, serial, d.label());
                        }
                    },
                );
            });
        });
        ui.end_row();

        ui.label(tr!("settings-radio-sample-rate"));
        ui.horizontal(|ui| {
            ComboBox::from_id_salt("hackrf_rate")
                .width(150.0)
                .selected_text(hackrf_rate_label(cfg.hackrf.sample_rate_hz))
                .show_styled(ui, |ui| {
                    for r in HackRfConfig::rates_for(is_pro) {
                        ui.selectable_value(
                            &mut cfg.hackrf.sample_rate_hz,
                            r,
                            format!("{} — {}", hackrf_rate_label(r), HackRfConfig::rate_note(r)),
                        );
                    }
                });
            ui.add(
                egui::Label::new(
                    RichText::new(HackRfConfig::rate_note(cfg.hackrf.sample_rate_hz)).weak(),
                )
                .wrap(),
            );
        });
        ui.end_row();

        ui.label(tr!("settings-radio-lna-gain"));
        if crate::chrome::slider(
            ui,
            egui::Slider::new(&mut cfg.hackrf.lna_db, 0.0..=40.0).step_by(8.0).suffix(" dB"),
        )
        .on_hover_text(tr!("settings-radio-front-end-amplifier-8-db"))
        .changed()
        {
            push_gain(cmds, HackRfConfig::LNA_ELEMENT, cfg.hackrf.lna_db);
        }
        ui.end_row();

        ui.label(tr!("settings-radio-vga-gain"));
        if crate::chrome::slider(
            ui,
            egui::Slider::new(&mut cfg.hackrf.vga_db, 0.0..=62.0).step_by(2.0).suffix(" dB"),
        )
        .on_hover_text(tr!("settings-radio-baseband-amplifier-after-mixer-2"))
        .changed()
        {
            push_gain(cmds, HackRfConfig::VGA_ELEMENT, cfg.hackrf.vga_db);
        }
        ui.end_row();

        ui.label(tr!("settings-radio-rf-amp"));
        if ui
            .checkbox(
                &mut cfg.hackrf.amp,
                tr!("settings-radio-hackrf-rx-preamp", db = format!("{:.0}", HackRfConfig::AMP_DB)),
            )
            .on_hover_text(tr!("settings-radio-one-switch-shared-transmit-setting"))
            .changed()
        {
            push_gain(cmds, HackRfConfig::AMP_ELEMENT, cfg.hackrf.amp as u8 as f64);
        }
        ui.end_row();

        ui.label(tr!("settings-radio-baseband-filter"));
        ui.horizontal(|ui| {
            let shown = if cfg.hackrf.filter_bw_hz <= 0.0 {
                tr!("settings-radio-automatic")
            } else {
                format!("{:.2} MHz", cfg.hackrf.filter_bw_hz / 1e6)
            };
            let mut picked = cfg.hackrf.filter_bw_hz;
            // A Pro derives its filter from the sample rate and discards what
            // the host asks for, so the driver does not send the request at
            // all. A control that cannot do anything is worse than no control:
            // grey it out and say which it is.
            ui.add_enabled_ui(!is_pro, |ui| {
                ComboBox::from_id_salt("hackrf_bbfilt")
                    .width(150.0)
                    .selected_text(shown)
                    .show_styled(ui, |ui| {
                        ui.selectable_value(&mut picked, 0.0, tr!("settings-radio-automatic"));
                        for bw in [
                            1.75e6, 2.5e6, 3.5e6, 5.0e6, 5.5e6, 6.0e6, 7.0e6, 8.0e6, 9.0e6, 10.0e6,
                            12.0e6, 14.0e6, 15.0e6, 20.0e6, 24.0e6, 28.0e6,
                        ] {
                            ui.selectable_value(&mut picked, bw, format!("{:.2} MHz", bw / 1e6));
                        }
                    });
            });
            if picked != cfg.hackrf.filter_bw_hz {
                cfg.hackrf.filter_bw_hz = picked;
                push_gain(cmds, HackRfConfig::FILTER_ELEMENT, picked);
            }
            ui.add(
                egui::Label::new(
                    RichText::new(if is_pro {
                        tr!("settings-radio-hackrf-pro-chooses-itself-three")
                    } else {
                        tr!("settings-radio-leave-automatic-choosing-one-too")
                    })
                    .weak(),
                )
                .wrap(),
            );
        });
        ui.end_row();

        ui.label(tr!("settings-radio-bias-tee"));
        if ui
            .checkbox(&mut cfg.hackrf.bias_tee, tr!("settings-radio-dc-antenna-port"))
            .on_hover_text(tr!("settings-radio-about-3-v-50-ma"))
            .changed()
        {
            push_gain(cmds, HackRfConfig::BIAS_TEE_ELEMENT, cfg.hackrf.bias_tee as u8 as f64);
        }
        ui.end_row();

        ui.label(tr!("settings-radio-iq-correction"));
        if ui
            .checkbox(&mut cfg.hackrf.iq_correction, tr!("settings-radio-remove-dc-mirror-image"))
            .on_hover_text(tr!("settings-radio-zero-if-radio-so-own"))
            .changed()
        {
            push_gain(
                cmds,
                HackRfConfig::IQ_CORRECTION_ELEMENT,
                cfg.hackrf.iq_correction as u8 as f64,
            );
        }
        ui.end_row();

        ui.label(tr!("settings-radio-clock-trim"));
        if ui
            .add(
                egui::DragValue::new(&mut cfg.hackrf.ppm)
                    .speed(0.1)
                    .range(-200.0..=200.0)
                    .suffix(tr!("settings-radio-ppm")),
            )
            .on_hover_text(tr!("settings-radio-corrects-reference-oscillator"))
            .changed()
        {
            push_gain(cmds, HackRfConfig::PPM_ELEMENT, cfg.hackrf.ppm);
        }
        ui.end_row();
    });

    if cfg.hackrf.bias_tee {
        ui.add_space(4.0);
        ui.label(
            RichText::new(tr!("settings-radio-bias-tee-never-connect-transceiver"))
                .color(crate::theme::YELLOW()),
        );
    }

    ui.add_space(8.0);
    ui.separator();
    ui.label(RichText::new(tr!("settings-radio-transmit")).strong());
    ui.add_space(4.0);

    egui::Grid::new("hackrf-tx-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr!("settings-radio-transmitter"));
        crate::chrome::checkbox(ui, &mut cfg.hackrf.tx_enabled, tr!("settings-radio-enabled"))
            .on_hover_text(tr!("settings-radio-off-default-while-off-backend"));
        ui.end_row();

        if cfg.hackrf.tx_enabled {
            ui.label(tr!("settings-radio-tx-vga-gain"));
            if crate::chrome::slider(
                ui,
                egui::Slider::new(&mut cfg.hackrf.txvga_db, 0.0..=47.0).step_by(1.0).suffix(" dB"),
            )
            .on_hover_text(tr!("settings-radio-transmit-driver-amplifier-drive-applied"))
            .changed()
            {
                cmds.push(Command::SetGain {
                    dir: Direction::Tx,
                    element: HackRfConfig::TXVGA_ELEMENT.into(),
                    db: cfg.hackrf.txvga_db,
                });
            }
            ui.end_row();

            ui.label(tr!("settings-radio-rf-amp"));
            if ui
                .checkbox(
                    &mut cfg.hackrf.tx_amp,
                    tr!(
                        "settings-radio-hackrf-tx-preamp",
                        db = format!("{:.0}", HackRfConfig::AMP_DB)
                    ),
                )
                .on_hover_text(tr!("settings-radio-same-switch-receive-setting-above"))
                .changed()
            {
                cmds.push(Command::SetGain {
                    dir: Direction::Tx,
                    element: HackRfConfig::TXAMP_ELEMENT.into(),
                    db: cfg.hackrf.tx_amp as u8 as f64,
                });
            }
            ui.end_row();
        }
    });

    if cfg.hackrf.tx_enabled {
        ui.add_space(4.0);
        ui.label(
            RichText::new(tr!("settings-radio-transmit-armed-into-dummy-load"))
                .color(crate::theme::YELLOW()),
        );
    }

    if (
        cfg.hackrf.serial.clone(),
        cfg.hackrf.sample_rate_hz,
        cfg.hackrf.tx_enabled,
        cfg.hackrf.transfers,
        cfg.hackrf.transfer_kib,
    ) != before
    {
        *apply = true;
    }

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        probe_only(ui, can_probe, |ui| {
            if ui
                .button(tr!("settings-radio-copy-diagnostic-report"))
                .on_hover_text(tr!("settings-radio-every-command-exchanged-radio-session"))
                .clicked()
            {
                *copy_report = true;
            }
        });
    });

    ui.add_space(4.0);
    ui.label(RichText::new(tr!("settings-radio-dc-7-25-ghz-half")).weak());
}

/// A HackRF sample rate for the settings combo.
///
/// Sub-megahertz rates exist only on the Pro, and `{:.1} Msps` renders 250 ksps
/// as "0.2 Msps" — wrong to two different decimal places at once. Below a
/// megasample the useful unit is kilosamples.
fn hackrf_rate_label(rate_hz: f64) -> String {
    if rate_hz < 1.0e6 {
        format!("{:.0} ksps", rate_hz / 1e3)
    } else {
        format!("{:.1} Msps", rate_hz / 1e6)
    }
}

/// SDRplay RSP interface: device, rate, and the RSP's gain model (IF gain
/// reduction + LNA state + hardware AGC), with the rows a given model lacks
/// hidden.
pub(in crate::app) fn settings_sdrplay_tab(
    ui: &mut egui::Ui,
    devices: &[sdroxide_types::SdrPlayDevice],
    caps: Option<&sdroxide_types::DeviceCaps>,
    radio_edit: &mut Option<sdroxide_types::RadioConfig>,
    rescan: &mut bool,
    apply: &mut bool,
    can_probe: bool,
    cmds: &mut Vec<Command>,
) {
    use sdroxide_types::{SdrPlayAgc, SdrPlayConfig, SdrPlayDuoTuner, SdrPlayHdrBw, SdrPlayModel};
    let Some(cfg) = radio_edit.as_mut() else {
        ui.label(tr!("settings-radio-waiting-configuration-machine-radio-attached"));
        return;
    };

    // Device, rate, bandwidth and the RSPduo's tuner arrangement rebuild the
    // session; the rest rides `SetGain` (or `SetAntenna`) straight to the
    // running device.
    let before = (
        cfg.sdrplay.serial.clone(),
        cfg.sdrplay.sample_rate_hz,
        cfg.sdrplay.bw_khz,
        cfg.sdrplay.duo_tuner,
        cfg.sdrplay.duo.enabled,
        cfg.sdrplay.duo.role,
    );

    // Which rows to draw comes from the *selected* device's model, and with
    // nothing enumerated (service down, mid-replug) from the RSP1A/1B feature
    // set: the driver ignores a switch the real hardware lacks, whereas a
    // hidden switch cannot be un-hidden by an operator whose service just
    // isn't running yet.
    //
    // A named serial that the list does not carry is *not* licence to describe
    // some other receiver: on a station with two RSPs that put the RSPdx's
    // serial in the picker and the RSPduo's antenna ports, tuner rows and LNA
    // ladder underneath it (issue #259). Only "— first one found —" falls back
    // to whatever is first.
    let listed = if cfg.sdrplay.serial.trim().is_empty() {
        devices.first()
    } else {
        devices.iter().find(|d| d.serial == cfg.sdrplay.serial)
    };
    let model = listed.map(|d| d.model()).unwrap_or(SdrPlayModel::Rsp1b);
    // ...and the same rule, kept rather than dropped, is what decides the
    // RSPduo's own rows. An empty device list is *not* evidence that this is
    // not an RSPduo — the service may not have been asked yet, may be down, or
    // may have been asked while another application held the board — so the
    // second-tuner rows stay up unless a listed device says otherwise. They
    // vanishing on a rescan that came back empty is issue #165.
    let maybe_duo = listed.is_none_or(|d| d.model() == SdrPlayModel::RspDuo);

    // Except that RSP1B is the one model with *no* antenna ports and the
    // shortest LNA ladder, so falling back to it does the very thing the rule
    // above forbids: it hides controls. Where a receiver is already open its
    // own capabilities are the honest account of what it has, and they hold
    // whether the service enumerated nothing, listed no serial to match, or
    // left out the device it has already handed to us.
    let open = caps.filter(|c| c.driver == "sdrplay" && listed.is_none());
    let open_ports = open.map(|c| c.antennas_rx.as_slice()).filter(|p| !p.is_empty());

    // A device listed without a serial number (or an unrecognised hardware
    // version) is the signature of a USB communication problem: it opens and
    // streams, but often deaf. Say so here, where the operator is already
    // looking for what went wrong — picking such an entry also stores an
    // empty serial, indistinguishable from "first one found".
    if let Some(w) = devices.iter().find_map(|d| d.identity_warning()) {
        ui.label(RichText::new(w).color(Color32::from_rgb(220, 170, 70)));
        ui.add_space(6.0);
    }

    // A serial pinned to a receiver that is not there any more — unplugged,
    // switched off in Device Manager, or held by another application. Said
    // here rather than left for Apply to fail on, because the rows below now
    // describe nothing in particular and the picker looks, misleadingly, as
    // though a receiver were selected.
    if listed.is_none() && !cfg.sdrplay.serial.trim().is_empty() && !devices.is_empty() {
        ui.label(
            RichText::new(tr!(
                "settings-radio-sdrplay-serial-missing",
                serial = cfg.sdrplay.serial.trim()
            ))
            .color(Color32::from_rgb(220, 170, 70)),
        );
        ui.add_space(6.0);
    }

    // Same story as the ports: an RSPdx guessed to be an RSP1B would lose two
    // thirds of its LNA range. The open device publishes the real one — and
    // publishes it *per band*, re-sent on every retune, so this follows the
    // dial rather than offering a ladder the current band does not have.
    // Hoisted out of the grid because the second tuner's ladder is the same
    // ladder.
    //
    // The fallback is the model's widest, which is all that can be said about
    // a receiver that is not open: there is no band yet to narrow it to.
    let max_lna = open
        .and_then(|c| c.gains.iter().find(|g| g.name == SdrPlayConfig::LNA_ELEMENT))
        .map(|g| (-g.min_db).round().clamp(0.0, 255.0) as u8)
        .filter(|&n| n > 0)
        .unwrap_or_else(|| model.max_lna_state());

    egui::Grid::new("sdrplay-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr!("settings-radio-receiver"));
        // The service that answers this is the one on the engine's machine;
        // everything below reaches the RSP through it.
        probe_only(ui, can_probe, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(tr!("settings-radio-rescan"))
                    .on_hover_text(tr!("settings-radio-ask-sdrplay-api-service-device"))
                    .clicked()
                {
                    *rescan = true;
                }
                let shown = if cfg.sdrplay.serial.is_empty() {
                    tr!("settings-radio-first-one-found")
                } else {
                    cfg.sdrplay.serial.clone()
                };
                ComboBox::from_id_salt("sdrplay_dev")
                    .width(300.0)
                    .selected_text(shown)
                    .show_styled(ui, |ui| {
                        if devices.is_empty() {
                            ui.label(
                                RichText::new(tr!("settings-radio-no-rsps-press-rescan-needs"))
                                    .weak(),
                            );
                        }
                        ui.selectable_value(
                            &mut cfg.sdrplay.serial,
                            String::new(),
                            tr!("settings-radio-first-one-found"),
                        );
                        for d in devices {
                            ui.selectable_value(
                                &mut cfg.sdrplay.serial,
                                d.serial.clone(),
                                d.label(),
                            );
                        }
                    });
            });
        });
        ui.end_row();

        // Two tuners share one ADC at a fixed clock, so the ladder they can
        // reach is a different — and much shorter — one. Offering the wide
        // rates would offer spans this configuration cannot open.
        let dual = maybe_duo && cfg.sdrplay.duo.enabled;
        ui.label(tr!("settings-radio-sample-rate")).on_hover_text(if dual {
            tr!("settings-radio-both-tuners-running-api-fixes")
        } else {
            tr!("settings-radio-rates-below-2-msps-run")
        });
        let shown = format!("{:.3} Msps", cfg.sdrplay.sample_rate_hz / 1e6);
        ComboBox::from_id_salt("sdrplay_rate").selected_text(shown).show_styled(ui, |ui| {
            let rates: &[f64] =
                if dual { &SdrPlayConfig::DUAL_SAMPLE_RATES } else { &SdrPlayConfig::SAMPLE_RATES };
            for &r in rates {
                let sel = (cfg.sdrplay.sample_rate_hz - r).abs() < 1.0;
                let mut label = format!("{:.3} Msps", r / 1e6);
                if r > 6_048_000.0 {
                    // The ADC trades resolution for speed past 6.048 Msps.
                    label.push_str(&tr!("settings-radio-reduced-adc-resolution"));
                }
                if ui.selectable_label(sel, label).clicked() {
                    cfg.sdrplay.sample_rate_hz = r;
                }
            }
        });
        ui.end_row();

        ui.label(tr!("settings-radio-if-bandwidth"))
            .on_hover_text(tr!("settings-radio-tuner-s-analog-filter-auto"));
        let shown = if cfg.sdrplay.bw_khz == 0 {
            tr!("settings-radio-auto")
        } else {
            format!("{} kHz", cfg.sdrplay.bw_khz)
        };
        ComboBox::from_id_salt("sdrplay_bw").selected_text(shown).show_styled(ui, |ui| {
            if ui.selectable_label(cfg.sdrplay.bw_khz == 0, tr!("settings-radio-auto")).clicked() {
                cfg.sdrplay.bw_khz = 0;
            }
            for &khz in &SdrPlayConfig::BANDWIDTHS_KHZ {
                // Filters wider than the rate would only alias; don't offer them.
                if (khz as f64) * 1000.0 > cfg.sdrplay.sample_rate_hz {
                    continue;
                }
                // The low IF two tuners work from has no room for the wide ones.
                if dual && khz > SdrPlayConfig::DUAL_MAX_BW_KHZ {
                    continue;
                }
                if ui.selectable_label(cfg.sdrplay.bw_khz == khz, format!("{khz} kHz")).clicked() {
                    cfg.sdrplay.bw_khz = khz;
                }
            }
        });
        ui.end_row();

        ui.label(tr!("settings-radio-agc")).on_hover_text(tr!("settings-radio-rsp-s-own-if-gain"));
        let mut agc = cfg.sdrplay.agc;
        enum_combo(ui, "sdrplay_agc", &mut agc, &SdrPlayAgc::ALL, SdrPlayAgc::tr);
        if agc != cfg.sdrplay.agc {
            cfg.sdrplay.agc = agc;
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: SdrPlayConfig::AGC_ELEMENT.to_string(),
                db: agc.code(),
            });
        }
        ui.end_row();

        if cfg.sdrplay.agc != SdrPlayAgc::Off {
            ui.label(tr!("settings-radio-agc-set-point"))
                .on_hover_text(tr!("settings-radio-signal-level-loop-holds-adc"));
            // The range the *rate* allows, not the widest any rate allows: the
            // converter trades headroom for speed, and the API refuses a set
            // point below what the rate can reach. At 10 Msps the floor is
            // -48, and this backend offers 10 Msps.
            let sp = cfg.sdrplay.agc_setpoint_range();
            let (sp_lo, sp_hi) = (*sp.start(), *sp.end());
            cfg.sdrplay.agc_setpoint_dbfs = cfg.sdrplay.agc_setpoint_dbfs.clamp(sp_lo, sp_hi);
            if crate::chrome::slider(
                ui,
                Slider::new(&mut cfg.sdrplay.agc_setpoint_dbfs, sp_lo..=sp_hi).suffix(" dBFS"),
            )
            .changed()
            {
                cmds.push(Command::SetGain {
                    dir: Direction::Rx,
                    element: SdrPlayConfig::AGC_SETPOINT_ELEMENT.to_string(),
                    db: cfg.sdrplay.agc_setpoint_dbfs as f64,
                });
            }
            ui.end_row();
        }

        ui.label(tr!("settings-radio-if-gain-reduction"))
            .on_hover_text(tr!("settings-radio-rsp-s-native-gain-unit"));
        // Read before the borrow below: the floor is a property of the whole
        // config, and the slider takes one field of it mutably.
        let if_gr_min = cfg.sdrplay.if_gr_min();
        ui.add_enabled_ui(cfg.sdrplay.agc == SdrPlayAgc::Off, |ui| {
            if crate::chrome::slider(
                ui,
                Slider::new(&mut cfg.sdrplay.if_gr_db, if_gr_min..=SdrPlayConfig::IF_GR_MAX)
                    .suffix(" dB"),
            )
            .changed()
            {
                cmds.push(Command::SetGain {
                    dir: Direction::Rx,
                    element: SdrPlayConfig::IF_GAIN_ELEMENT.to_string(),
                    db: -(cfg.sdrplay.if_gr_db as f64),
                });
            }
        });
        ui.end_row();

        ui.label(tr!("settings-radio-extended-if-range"))
            .on_hover_text(tr!("settings-radio-lets-if-gain-reduction-go"));
        {
            let mut on = cfg.sdrplay.extended_if_gr;
            if ui.checkbox(&mut on, tr!("settings-radio-allow-if-gain-reduction-below")).changed() {
                cfg.sdrplay.extended_if_gr = on;
                // The floor moves under the value, so put it back in range
                // before the slider above redraws with the new bound.
                let min = cfg.sdrplay.if_gr_min();
                cfg.sdrplay.if_gr_db = cfg.sdrplay.if_gr_db.max(min);
                cmds.push(Command::SetGain {
                    dir: Direction::Rx,
                    element: SdrPlayConfig::EXTENDED_IF_GR_ELEMENT.to_string(),
                    db: if on { 1.0 } else { 0.0 },
                });
            }
        }
        ui.end_row();

        ui.label(tr!("settings-radio-lna-state"))
            .on_hover_text(tr!("settings-radio-front-end-attenuation-steps-0"));
        // Clamped on edit only. The rail is this band's, but the value is a
        // stored preference that outlives the band: clamping it on sight would
        // have merely opening this tab on 40 m quietly trim a setting chosen
        // for 2 m — against the promise in the hover text right above.
        if crate::chrome::slider(
            ui,
            Slider::new(&mut cfg.sdrplay.lna_state, 0..=max_lna)
                .clamping(egui::SliderClamping::Edits),
        )
        .on_hover_text(tr!("settings-radio-t-0-max-gain"))
        .changed()
        {
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: SdrPlayConfig::LNA_ELEMENT.to_string(),
                db: -(cfg.sdrplay.lna_state as f64),
            });
        }
        ui.end_row();

        ui.label(tr!("settings-radio-frequency-correction"))
            .on_hover_text(tr!("settings-radio-reference-error-parts-per-million-2"));
        let mut ppm = cfg.sdrplay.ppm;
        if ui
            .add(
                DragValue::new(&mut ppm)
                    .speed(0.1)
                    .range(-200.0..=200.0)
                    .suffix(tr!("settings-radio-ppm")),
            )
            .changed()
        {
            cfg.sdrplay.ppm = ppm;
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: SdrPlayConfig::PPM_ELEMENT.to_string(),
                db: ppm,
            });
        }
        ui.end_row();

        if maybe_duo {
            ui.label(if cfg.sdrplay.duo.enabled {
                tr!("settings-radio-radio-s-tuner")
            } else {
                tr!("settings-radio-tuner")
            })
            .on_hover_text(match (cfg.sdrplay.duo.enabled, cfg.sdrplay.duo.role) {
                (true, sdroxide_types::SdrPlayDuoRole::SecondRadio) => {
                    tr!("settings-radio-which-rspduo-s-two-tuners")
                }
                (true, _) => {
                    tr!("settings-radio-which-rspduo-s-two-tuners-2")
                }
                (false, _) => {
                    tr!("settings-radio-which-rspduo-s-two-tuners-3")
                }
            });
            let mut tuner = cfg.sdrplay.duo_tuner;
            enum_combo(
                ui,
                "sdrplay_duo_tuner",
                &mut tuner,
                &SdrPlayDuoTuner::ALL,
                SdrPlayDuoTuner::tr,
            );
            if tuner != cfg.sdrplay.duo_tuner {
                cfg.sdrplay.duo_tuner = tuner;
                // The port list belongs to the tuner; a remembered tuner-1
                // port name means nothing on tuner 2.
                cfg.sdrplay.antenna = String::new();
            }
            ui.end_row();
        }

        let antennas: Vec<&str> = match open_ports {
            Some(ports) => ports.iter().map(String::as_str).collect(),
            None => model.antennas(cfg.sdrplay.duo_tuner).to_vec(),
        };
        if !antennas.is_empty() {
            ui.label(tr!("settings-radio-antenna"))
                .on_hover_text(tr!("settings-radio-applies-immediately"));
            let shown = if cfg.sdrplay.antenna.is_empty() {
                antennas[0].to_string()
            } else {
                cfg.sdrplay.antenna.clone()
            };
            ComboBox::from_id_salt("sdrplay_antenna").selected_text(shown).show_styled(ui, |ui| {
                for a in &antennas {
                    if ui.selectable_label(cfg.sdrplay.antenna == *a, *a).clicked() {
                        cfg.sdrplay.antenna = a.to_string();
                        cmds.push(Command::SetAntenna { dir: Direction::Rx, name: a.to_string() });
                    }
                }
            });
            ui.end_row();
        }

        if model.has_rf_notch() {
            ui.label(tr!("settings-radio-fm-broadcast-notch"));
            let mut on = cfg.sdrplay.rf_notch;
            if ui
                .checkbox(&mut on, tr!("settings-radio-enable"))
                .on_hover_text(tr!("settings-radio-hardware-notch-over-88-108"))
                .changed()
            {
                cfg.sdrplay.rf_notch = on;
                cmds.push(Command::SetGain {
                    dir: Direction::Rx,
                    element: SdrPlayConfig::RF_NOTCH_ELEMENT.to_string(),
                    db: if on { 1.0 } else { 0.0 },
                });
            }
            ui.end_row();
        }

        if model.has_dab_notch() {
            ui.label(tr!("settings-radio-dab-notch"));
            let mut on = cfg.sdrplay.dab_notch;
            if ui
                .checkbox(&mut on, tr!("settings-radio-enable"))
                .on_hover_text(tr!("settings-radio-hardware-notch-over-165-230"))
                .changed()
            {
                cfg.sdrplay.dab_notch = on;
                cmds.push(Command::SetGain {
                    dir: Direction::Rx,
                    element: SdrPlayConfig::DAB_NOTCH_ELEMENT.to_string(),
                    db: if on { 1.0 } else { 0.0 },
                });
            }
            ui.end_row();
        }

        if model.has_hdr() {
            ui.label(tr!("settings-radio-hdr-mode"));
            let mut on = cfg.sdrplay.hdr;
            if ui
                .checkbox(&mut on, tr!("settings-radio-enable-below-2-mhz"))
                .on_hover_text(tr!("settings-radio-rspdx-s-high-dynamic-range"))
                .changed()
            {
                cfg.sdrplay.hdr = on;
                cmds.push(Command::SetGain {
                    dir: Direction::Rx,
                    element: SdrPlayConfig::HDR_ELEMENT.to_string(),
                    db: if on { 1.0 } else { 0.0 },
                });
            }
            ui.end_row();

            // Only with the path switched on: the filter is a property of a
            // mode that is otherwise off, and a control that does nothing is
            // worse than one that is not there.
            if cfg.sdrplay.hdr {
                ui.label(tr!("settings-radio-hdr-filter")).on_hover_text(tr!(
                    "settings-radio-hdr-filter-hint",
                    centres = cfg.sdrplay.hdr_bw.centres_label()
                ));
                let mut bw = cfg.sdrplay.hdr_bw;
                enum_combo(ui, "sdrplay_hdr_bw", &mut bw, &SdrPlayHdrBw::ALL, SdrPlayHdrBw::label);
                if bw != cfg.sdrplay.hdr_bw {
                    cfg.sdrplay.hdr_bw = bw;
                    cmds.push(Command::SetGain {
                        dir: Direction::Rx,
                        element: SdrPlayConfig::HDR_BW_ELEMENT.to_string(),
                        db: f64::from(bw.code()),
                    });
                }
                ui.end_row();
            }
        }

        if model.has_bias_tee() {
            ui.label(tr!("settings-radio-bias-tee"));
            let mut on = cfg.sdrplay.bias_tee;
            if ui
                .checkbox(&mut on, tr!("settings-radio-feed-4-7-v-dc"))
                .on_hover_text(tr!("settings-radio-powers-active-antenna-preamp-down"))
                .changed()
            {
                cfg.sdrplay.bias_tee = on;
                cmds.push(Command::SetGain {
                    dir: Direction::Rx,
                    element: SdrPlayConfig::BIAS_TEE_ELEMENT.to_string(),
                    db: if on { 1.0 } else { 0.0 },
                });
            }
            ui.end_row();
        }
    });

    // ---- the second tuner (issues #153, #165) ------------------------------
    // Shown for an RSPduo, for anything the device list cannot rule out, and
    // for anything still carrying the setting: a switch that cannot be seen is
    // a switch that cannot be turned off, and this one survives moving the
    // configuration to another receiver.
    if maybe_duo || cfg.sdrplay.duo.enabled {
        use sdroxide_types::{
            DIVERSITY_MAX_TAPS, DiversityMode, SdrPlayDuoRole, diversity_cost_note,
        };

        ui.add_space(6.0);
        ui.separator();
        ui.label(RichText::new(tr!("settings-radio-second-tuner")).strong());
        ui.label(RichText::new(tr!("settings-radio-rspduo-two-whole-tuners-one")).weak());
        if crate::chrome::checkbox(
            ui,
            &mut cfg.sdrplay.duo.enabled,
            tr!("settings-radio-run-both-tuners"),
        )
        .on_hover_text(tr!("settings-radio-puts-rspduo-api-s-dual"))
        .changed()
        {
            // The wide rates do not exist in dual-tuner mode; carrying one in
            // would open at 2 Msps and leave the panel claiming otherwise.
            if cfg.sdrplay.duo.enabled {
                cfg.sdrplay.sample_rate_hz = cfg.sdrplay.sample_rate_hz.min(2_000_000.0);
                cfg.sdrplay.bw_khz = 0;
            }
        }

        if cfg.sdrplay.duo.enabled {
            egui::Grid::new("sdrplay-duo-grid").num_columns(2).spacing([12.0, 6.0]).show(
                ui,
                |ui| {
                    ui.label(tr!("settings-radio-used"))
                        .on_hover_text(tr!("settings-radio-what-other-tuner-takes-effect"));
                    let mut role = cfg.sdrplay.duo.role;
                    enum_combo(
                        ui,
                        "sdrplay_duo_role",
                        &mut role,
                        &SdrPlayDuoRole::ALL,
                        SdrPlayDuoRole::tr,
                    );
                    cfg.sdrplay.duo.role = role;
                    ui.end_row();
                },
            );
        }

        if cfg.sdrplay.duo.enabled && cfg.sdrplay.duo.role == SdrPlayDuoRole::SecondRadio {
            ui.label(RichText::new(tr!("settings-radio-add-second-radio-settings-radio")).weak());
        }

        if cfg.sdrplay.duo.enabled && cfg.sdrplay.duo.role == SdrPlayDuoRole::Diversity {
            let div = &mut cfg.sdrplay.duo;
            egui::Grid::new("sdrplay-div-grid").num_columns(2).spacing([12.0, 6.0]).show(
                ui,
                |ui| {
                    ui.label(tr!("settings-radio-what-do"));
                    ComboBox::from_id_salt("sdrplay_div_mode")
                        .selected_text(div.mode.tr())
                        .show_styled(ui, |ui| {
                            for m in DiversityMode::ALL {
                                if ui.selectable_label(div.mode == m, m.tr()).clicked() {
                                    div.mode = m;
                                    push_gain(
                                        cmds,
                                        SdrPlayConfig::DIV_MODE_ELEMENT,
                                        f64::from(u8::from(m == DiversityMode::Combine)),
                                    );
                                }
                            }
                        });
                    ui.end_row();

                    ui.label(tr!("settings-radio-lna-state-2"))
                        .on_hover_text(tr!("settings-radio-second-tuner-s-own-front"));
                    // Clamped on edit only, for the same reason as the first
                    // tuner's above: the rail is this band's, the value is a
                    // preference that outlives it.
                    if crate::chrome::slider(
                        ui,
                        Slider::new(&mut div.lna_state, 0..=max_lna)
                            .clamping(egui::SliderClamping::Edits),
                    )
                    .on_hover_text(tr!("settings-radio-t-0-max-gain"))
                    .changed()
                    {
                        push_gain(cmds, SdrPlayConfig::AUX_LNA_ELEMENT, -(div.lna_state as f64));
                    }
                    ui.end_row();

                    ui.label(tr!("settings-radio-if-gain-reduction-2"))
                        .on_hover_text(tr!("settings-radio-second-tuner-s-if-gain"));
                    ui.add_enabled_ui(cfg.sdrplay.agc == SdrPlayAgc::Off, |ui| {
                        if crate::chrome::slider(
                            ui,
                            Slider::new(
                                &mut div.if_gr_db,
                                SdrPlayConfig::IF_GR_MIN..=SdrPlayConfig::IF_GR_MAX,
                            )
                            .suffix(" dB"),
                        )
                        .changed()
                        {
                            push_gain(
                                cmds,
                                SdrPlayConfig::AUX_IF_GAIN_ELEMENT,
                                -(div.if_gr_db as f64),
                            );
                        }
                    });
                    ui.end_row();

                    ui.label(tr!("settings-radio-filter-length"));
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                DragValue::new(&mut div.taps)
                                    .speed(1.0)
                                    .range(1..=DIVERSITY_MAX_TAPS)
                                    .suffix(tr!("settings-radio-taps")),
                            )
                            .on_hover_text(tr!("settings-radio-one-tap-gain-phase-null-2"))
                            .changed()
                        {
                            push_gain(cmds, SdrPlayConfig::DIV_TAPS_ELEMENT, f64::from(div.taps));
                        }
                        ui.label(
                            RichText::new(diversity_cost_note(
                                div.taps,
                                cfg.sdrplay.sample_rate_hz.min(2_000_000.0),
                            ))
                            .weak(),
                        );
                    });
                    ui.end_row();

                    ui.label(tr!("settings-radio-adaptation-2"));
                    ui.horizontal(|ui| {
                        if crate::chrome::slider(
                            ui,
                            Slider::new(&mut div.rate, 0.0..=1.0).show_value(false),
                        )
                        .on_hover_text(tr!("settings-radio-slow-steady-left-converging-inside"))
                        .changed()
                        {
                            push_gain(cmds, SdrPlayConfig::DIV_RATE_ELEMENT, f64::from(div.rate));
                        }
                        if ui
                            .checkbox(&mut div.frozen, tr!("settings-radio-hold"))
                            .on_hover_text(tr!("settings-radio-stop-filter-moving-reach-moment"))
                            .changed()
                        {
                            push_gain(
                                cmds,
                                SdrPlayConfig::DIV_FREEZE_ELEMENT,
                                f64::from(u8::from(div.frozen)),
                            );
                        }
                        if ui
                            .button(tr!("settings-radio-restart"))
                            .on_hover_text(tr!("settings-radio-zero-filter-find-null-again"))
                            .clicked()
                        {
                            push_gain(cmds, SdrPlayConfig::DIV_RESET_ELEMENT, 1.0);
                        }
                    });
                    ui.end_row();
                },
            );
            ui.label(RichText::new(tr!("settings-radio-nothing-here-can-tell-wanted")).weak());
        }

        // Both of the second tuner's jobs rest on the same unverified
        // dual-tuner mode, and both are equally undone by a board that has
        // only one tuner — so these two say so wherever the setting is on.
        if cfg.sdrplay.duo.enabled {
            ui.label(
                RichText::new(tr!("settings-radio-not-yet-verified-against-rspduo"))
                    .color(Color32::from_rgb(220, 170, 70)),
            );
            if !maybe_duo {
                ui.label(
                    RichText::new(tr!("settings-radio-rspduo-only", model = model.label()))
                        .color(Color32::from_rgb(220, 170, 70)),
                );
            } else if devices.is_empty() {
                ui.label(
                    RichText::new(tr!("settings-radio-no-receiver-listed-check-against")).weak(),
                );
            }
        }
    }

    if cfg.sdrplay.bias_tee && model.has_bias_tee() {
        ui.add_space(4.0);
        ui.label(
            RichText::new(tr!("settings-radio-bias-tee-never-connect-transceiver"))
                .color(crate::theme::YELLOW()),
        );
    }

    if before
        != (
            cfg.sdrplay.serial.clone(),
            cfg.sdrplay.sample_rate_hz,
            cfg.sdrplay.bw_khz,
            cfg.sdrplay.duo_tuner,
            cfg.sdrplay.duo.enabled,
            cfg.sdrplay.duo.role,
        )
    {
        *apply = true;
    }

    ui.add_space(4.0);
    ui.label(RichText::new(tr!("settings-radio-receive-only-1-khz-2")).weak());
}

/// LimeSDR family, through LimeSuite, and the LimeRFE in front of it.
///
/// Two halves in one tab because they are one radio to the operator. The upper
/// half is the board; the lower is the front end, which is off until it is
/// declared — the same default-inert rule the HPSDR filter board follows, and
/// for the same reason: this accessory switches a power amplifier.
///
/// The band readout in the LimeRFE section is worth the space it takes. It
/// resolves the current dial through exactly the code the driver uses, so an
/// operator can see which filter a frequency picks and which connector it
/// needs *before* keying, rather than finding out from a refusal.
#[allow(clippy::too_many_arguments)]
pub(in crate::app) fn settings_lime_tab(
    ui: &mut egui::Ui,
    devices: &[sdroxide_types::LimeDevice],
    serial_ports: &[String],
    radio_edit: &mut Option<sdroxide_types::RadioConfig>,
    rescan: &mut bool,
    copy_report: &mut bool,
    apply: &mut bool,
    can_probe: bool,
    cmds: &mut Vec<Command>,
) {
    use sdroxide_types::{
        DiversityMode, LimeAuxConfig, LimeAuxRole, LimeConfig, RFE_ATTEN_MAX_STEPS,
        RFE_ATTEN_STEP_DB, RfeChannel, RfeLink, RfeModeControl, RfePort,
    };
    let Some(cfg) = radio_edit.as_mut() else {
        ui.label(tr!("settings-radio-waiting-configuration-machine-radio-attached"));
        return;
    };

    // What forces the session to be rebuilt rather than adjusted in place.
    let before = (
        cfg.lime.device.clone(),
        cfg.lime.channel,
        cfg.lime.sample_rate_hz,
        cfg.lime.oversample,
        cfg.lime.tx_enabled,
        cfg.lime.fifo_ksamples,
        cfg.lime.rfe.link,
        cfg.lime.rfe.serial.path.clone(),
        // The second chain's stream is created at open and bound to its
        // channel, so turning it on or off is a rebuild. Everything else about
        // it is live.
        cfg.lime.aux.role,
    );

    // How many receive chains the chosen board has. From the name: the
    // enumeration never opens a device, so there is nothing to ask.
    let chains = devices
        .iter()
        .find(|d| d.matches(&cfg.lime.device))
        .map(|d| d.rx_channels())
        .unwrap_or(if cfg.lime.channel > 0 { 2 } else { 1 });

    egui::Grid::new("lime-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr!("settings-radio-board"));
        probe_only(ui, can_probe, |ui| {
            ui.horizontal(|ui| {
                let current = devices
                    .iter()
                    .find(|d| d.matches(&cfg.lime.device))
                    .map(|d| d.label())
                    .unwrap_or_else(|| {
                        if cfg.lime.device.trim().is_empty() {
                            tr!("settings-radio-first-one-found-2")
                        } else {
                            tr!(
                                "settings-radio-device-not-found",
                                device = cfg.lime.device.to_string()
                            )
                        }
                    });
                egui::ComboBox::from_id_salt("lime-dev").selected_text(current).show_styled(
                    ui,
                    |ui| {
                        ui.selectable_value(
                            &mut cfg.lime.device,
                            String::new(),
                            tr!("settings-radio-first-one-found-2"),
                        );
                        for d in devices {
                            let sel = d.matches(&cfg.lime.device) && !cfg.lime.device.is_empty();
                            if ui.selectable_label(sel, d.label()).clicked() {
                                // Pin by serial where there is one: a device
                                // string carries the bus address, which changes
                                // when the cable moves.
                                cfg.lime.device = if d.serial.is_empty() {
                                    d.info.clone()
                                } else {
                                    d.serial.clone()
                                };
                            }
                        }
                    },
                );
                if ui.button(tr!("settings-radio-rescan")).clicked() {
                    *rescan = true;
                }
            });
        });
        ui.end_row();

        // Which of the board's two front ends. Only worth a control on a board
        // that has two: on a Mini there is one chain and one set of sockets,
        // and a picker with a single entry is furniture.
        if chains > 1 || cfg.lime.channel > 0 {
            ui.label(tr!("settings-radio-receive-chain"));
            ui.horizontal(|ui| {
                let text = tr!("settings-radio-lime-chain", n = cfg.lime.channel + 1);
                egui::ComboBox::from_id_salt("lime-chain").selected_text(text).show_styled(
                    ui,
                    |ui| {
                        for c in 0..chains.max(usize::from(cfg.lime.channel) + 1) as u8 {
                            ui.selectable_value(
                                &mut cfg.lime.channel,
                                c,
                                tr!("settings-radio-lime-chain-tx", n = c + 1),
                            );
                        }
                    },
                );
                ui.label(
                    egui::RichText::new(tr!("settings-radio-both-chains-tune-together-they"))
                        .weak(),
                );
            });
            ui.end_row();
        }

        ui.label(tr!("settings-radio-sample-rate"));
        ui.horizontal(|ui| {
            let text = format!("{:.3} Msps", cfg.lime.sample_rate_hz / 1e6);
            // The Mini's USB link underruns at 1 Msps on transmit (issue #609),
            // so it gets a lower-rate list; every other board gets the full one.
            let rates = devices
                .iter()
                .find(|d| d.matches(&cfg.lime.device))
                .map(|d| LimeConfig::rates_for(&d.name))
                .unwrap_or(&LimeConfig::SAMPLE_RATES);
            egui::ComboBox::from_id_salt("lime-rate").selected_text(text).show_styled(ui, |ui| {
                for &r in rates {
                    let label = match LimeConfig::rate_note(r) {
                        Some(note) => format!("{:.3} Msps — {note}", r / 1e6),
                        None => format!("{:.3} Msps", r / 1e6),
                    };
                    ui.selectable_value(&mut cfg.lime.sample_rate_hz, r, label);
                }
            });
        });
        ui.end_row();

        ui.label(tr!("settings-radio-receive-gain"));
        if crate::chrome::slider(
            ui,
            egui::Slider::new(
                &mut cfg.lime.rx_gain_db,
                LimeConfig::GAIN_MIN_DB..=LimeConfig::GAIN_MAX_DB,
            )
            .suffix(" dB")
            .step_by(1.0),
        )
        .on_hover_text(tr!("settings-radio-one-combined-figure-which-limesuite"))
        .changed()
        {
            push_gain(cmds, LimeConfig::RX_GAIN_ELEMENT, cfg.lime.rx_gain_db);
        }
        ui.end_row();

        ui.label(tr!("settings-radio-receive-port"));
        ui.horizontal(|ui| {
            let text = if cfg.lime.antenna_rx.is_empty() {
                tr!("settings-radio-automatic")
            } else {
                LimeConfig::port_label(cfg.lime.channel, &cfg.lime.antenna_rx, false)
            };
            let before_rx = cfg.lime.antenna_rx.clone();
            let chan = cfg.lime.channel;
            egui::ComboBox::from_id_salt("lime-antrx").selected_text(text).show_styled(ui, |ui| {
                ui.selectable_value(
                    &mut cfg.lime.antenna_rx,
                    String::new(),
                    tr!("settings-radio-automatic"),
                );
                for a in ["LNAH", "LNAL", "LNAW"] {
                    // Named by the socket as well as the chip's port: `LNAL`
                    // is the same word on both chains, and the connector is
                    // the end the aerial goes into.
                    ui.selectable_value(
                        &mut cfg.lime.antenna_rx,
                        a.to_string(),
                        LimeConfig::port_label(chan, a, false),
                    );
                }
            });
            // Move the socket now rather than at the next start. Which socket
            // the aerial is in is exactly the thing an operator changes while
            // listening to find out whether it is the right one, and a control
            // that saved the answer for tomorrow read as a control that does
            // nothing. Automatic is the one choice with no port to command, so
            // it goes back through the reopen.
            if cfg.lime.antenna_rx != before_rx {
                if cfg.lime.antenna_rx.is_empty() {
                    *apply = true;
                } else {
                    cmds.push(Command::SetAntenna {
                        dir: Direction::Rx,
                        name: cfg.lime.antenna_rx.clone(),
                    });
                }
            }
            ui.label(
                egui::RichText::new(tr!("settings-radio-automatic-follows-frequency-lnal-low"))
                    .weak(),
            );
        });
        ui.end_row();

        ui.label(tr!("settings-radio-analog-filter"));
        ui.horizontal(|ui| {
            let mut mhz = cfg.lime.lpf_rx_hz / 1e6;
            if ui
                .add(egui::DragValue::new(&mut mhz).speed(0.1).range(0.0..=130.0).suffix(" MHz"))
                .on_hover_text(tr!("settings-radio-t-0-follows-sample-rate-worth"))
                .changed()
            {
                cfg.lime.lpf_rx_hz = mhz * 1e6;
                push_gain(cmds, LimeConfig::LPF_RX_ELEMENT, cfg.lime.lpf_rx_hz);
            }
            if cfg.lime.lpf_rx_hz == 0.0 {
                ui.label(egui::RichText::new(tr!("settings-radio-following-rate")).weak());
            }
        });
        ui.end_row();

        ui.label(tr!("settings-radio-corrections"));
        ui.horizontal(|ui| {
            if ui
                .checkbox(&mut cfg.lime.iq_correction, tr!("settings-radio-host-iq-dc-correction"))
                .on_hover_text(tr!("settings-radio-adaptive-image-dc-removal-side"))
                .changed()
            {
                push_gain(
                    cmds,
                    LimeConfig::IQ_CORRECTION_ELEMENT,
                    f64::from(u8::from(cfg.lime.iq_correction)),
                );
            }
            crate::chrome::checkbox(
                ui,
                &mut cfg.lime.calibrate,
                tr!("settings-radio-calibrate-automatically"),
            )
            .on_hover_text(tr!("settings-radio-runs-chip-s-own-dc"));
            if ui
                .button(tr!("settings-radio-calibrate-now"))
                .on_hover_text(tr!("settings-radio-stalls-receiver-better-part-second"))
                .clicked()
            {
                push_gain(cmds, LimeConfig::CALIBRATE_ELEMENT, 1.0);
            }
        });
        ui.end_row();
    });

    // ---- the second receive chain (issue #98) ------------------------------
    if chains > 1 || cfg.lime.aux.role != LimeAuxRole::Off {
        ui.add_space(6.0);
        ui.separator();
        ui.label(egui::RichText::new(tr!("settings-radio-second-aerial")).strong());
        ui.label(
            egui::RichText::new(tr!(
                "settings-radio-lime-second-chain",
                sockets = format!("RX{}_*", cfg.lime.aux_channel() + 1)
            ))
            .weak(),
        );
        let aux_chan = cfg.lime.aux_channel();
        egui::Grid::new("lime-aux-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label(tr!("settings-radio-used"));
            egui::ComboBox::from_id_salt("lime-aux-role")
                .selected_text(cfg.lime.aux.role.tr())
                .show_styled(ui, |ui| {
                    for r in LimeAuxRole::ALL {
                        ui.selectable_value(&mut cfg.lime.aux.role, r, r.tr());
                    }
                });
            ui.end_row();

            if cfg.lime.aux.role != LimeAuxRole::Off {
                ui.label(tr!("settings-radio-socket"));
                ui.horizontal(|ui| {
                    let text = if cfg.lime.aux.antenna.is_empty() {
                        tr!("settings-radio-same-first")
                    } else {
                        LimeConfig::port_label(aux_chan, &cfg.lime.aux.antenna, false)
                    };
                    let before_aux = cfg.lime.aux.antenna.clone();
                    egui::ComboBox::from_id_salt("lime-aux-ant").selected_text(text).show_styled(
                        ui,
                        |ui| {
                            ui.selectable_value(
                                &mut cfg.lime.aux.antenna,
                                String::new(),
                                tr!("settings-radio-same-first"),
                            );
                            for a in ["LNAH", "LNAL", "LNAW"] {
                                ui.selectable_value(
                                    &mut cfg.lime.aux.antenna,
                                    a.to_string(),
                                    LimeConfig::port_label(aux_chan, a, false),
                                );
                            }
                        },
                    );
                    // Immediately, like the main chain's: finding out which
                    // socket the noise aerial should be in is done by trying
                    // one and listening.
                    if cfg.lime.aux.antenna != before_aux {
                        if cfg.lime.aux.antenna.is_empty() {
                            *apply = true;
                        } else {
                            cmds.push(Command::SetDeviceSetting {
                                key: LimeConfig::AUX_ANTENNA_SETTING.to_string(),
                                value: cfg.lime.aux.antenna.clone(),
                            });
                        }
                    }
                });
                ui.end_row();

                ui.label(tr!("settings-radio-gain-2"));
                if crate::chrome::slider(
                    ui,
                    egui::Slider::new(
                        &mut cfg.lime.aux.gain_db,
                        LimeConfig::GAIN_MIN_DB..=LimeConfig::GAIN_MAX_DB,
                    )
                    .suffix(" dB")
                    .step_by(1.0),
                )
                .on_hover_text(if cfg.lime.aux.role == LimeAuxRole::PureSignal {
                    tr!("settings-radio-set-low-coupled-sample-your")
                } else {
                    tr!("settings-radio-set-so-both-aerials-show")
                })
                .changed()
                {
                    push_gain(cmds, LimeConfig::AUX_GAIN_ELEMENT, cfg.lime.aux.gain_db);
                }
                ui.end_row();
            }

            if cfg.lime.aux.role == LimeAuxRole::Diversity {
                ui.label(tr!("settings-radio-what-do"));
                egui::ComboBox::from_id_salt("lime-div-mode")
                    .selected_text(cfg.lime.aux.mode.tr())
                    .show_styled(ui, |ui| {
                        for m in DiversityMode::ALL {
                            if ui.selectable_label(cfg.lime.aux.mode == m, m.tr()).clicked() {
                                cfg.lime.aux.mode = m;
                                push_gain(
                                    cmds,
                                    LimeConfig::DIV_MODE_ELEMENT,
                                    f64::from(u8::from(m == DiversityMode::Combine)),
                                );
                            }
                        }
                    });
                ui.end_row();

                ui.label(tr!("settings-radio-filter-length"));
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::DragValue::new(&mut cfg.lime.aux.taps)
                                .speed(1.0)
                                .range(1..=LimeAuxConfig::MAX_TAPS)
                                .suffix(tr!("settings-radio-taps")),
                        )
                        .on_hover_text(tr!("settings-radio-one-tap-gain-phase-null-2"))
                        .changed()
                    {
                        push_gain(cmds, LimeConfig::DIV_TAPS_ELEMENT, f64::from(cfg.lime.aux.taps));
                    }
                    ui.label(
                        egui::RichText::new(LimeAuxConfig::cost_note(
                            cfg.lime.aux.taps,
                            cfg.lime.sample_rate_hz,
                        ))
                        .weak(),
                    );
                });
                ui.end_row();

                ui.label(tr!("settings-radio-adaptation-2"));
                ui.horizontal(|ui| {
                    if crate::chrome::slider(
                        ui,
                        egui::Slider::new(&mut cfg.lime.aux.rate, 0.0..=1.0).show_value(false),
                    )
                    .on_hover_text(tr!("settings-radio-slow-steady-left-converging-inside"))
                    .changed()
                    {
                        push_gain(cmds, LimeConfig::DIV_RATE_ELEMENT, f64::from(cfg.lime.aux.rate));
                    }
                    if ui
                        .checkbox(&mut cfg.lime.aux.frozen, tr!("settings-radio-hold"))
                        .on_hover_text(tr!("settings-radio-stop-filter-moving-reach-moment"))
                        .changed()
                    {
                        push_gain(
                            cmds,
                            LimeConfig::DIV_FREEZE_ELEMENT,
                            f64::from(u8::from(cfg.lime.aux.frozen)),
                        );
                    }
                    if ui
                        .button(tr!("settings-radio-restart"))
                        .on_hover_text(tr!("settings-radio-zero-filter-find-null-again"))
                        .clicked()
                    {
                        push_gain(cmds, LimeConfig::DIV_RESET_ELEMENT, 1.0);
                    }
                });
                ui.end_row();
            }

            if cfg.lime.aux.role == LimeAuxRole::PureSignal {
                ui.label(tr!("settings-radio-table-steps-2"));
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::DragValue::new(&mut cfg.lime.aux.ps_bins)
                                .speed(1.0)
                                .range(LimeAuxConfig::PS_MIN_BINS..=LimeAuxConfig::PS_MAX_BINS),
                        )
                        .on_hover_text(tr!(
                            "settings-radio-how-finely-correction-follows-amplifier"
                        ))
                        .changed()
                    {
                        push_gain(
                            cmds,
                            LimeConfig::PS_BINS_ELEMENT,
                            f64::from(cfg.lime.aux.ps_bins),
                        );
                    }
                });
                ui.end_row();

                ui.label(tr!("settings-radio-adaptation-2"));
                ui.horizontal(|ui| {
                    if crate::chrome::slider(
                        ui,
                        egui::Slider::new(&mut cfg.lime.aux.ps_rate, 0.0..=1.0).show_value(false),
                    )
                    .on_hover_text(tr!("settings-radio-how-hard-each-block-feedback"))
                    .changed()
                    {
                        push_gain(
                            cmds,
                            LimeConfig::PS_RATE_ELEMENT,
                            f64::from(cfg.lime.aux.ps_rate),
                        );
                    }
                    if ui
                        .checkbox(&mut cfg.lime.aux.ps_frozen, tr!("settings-radio-hold"))
                        .on_hover_text(tr!("settings-radio-keep-correction-curve-learned-clean"))
                        .changed()
                    {
                        push_gain(
                            cmds,
                            LimeConfig::PS_FREEZE_ELEMENT,
                            f64::from(u8::from(cfg.lime.aux.ps_frozen)),
                        );
                    }
                    if ui
                        .button(tr!("settings-radio-restart"))
                        .on_hover_text(tr!("settings-radio-forget-correction-learn-again"))
                        .clicked()
                    {
                        push_gain(cmds, LimeConfig::PS_RESET_ELEMENT, 1.0);
                    }
                });
                ui.end_row();
            }
        });
        if cfg.lime.aux.role == LimeAuxRole::PureSignal {
            ui.label(
                egui::RichText::new(tr!("settings-radio-directional-coupler-amplifier-s-output"))
                    .weak(),
            );
            if !cfg.lime.tx_enabled {
                ui.label(
                    egui::RichText::new(tr!("settings-radio-transmit-not-armed-so-there"))
                        .color(egui::Color32::from_rgb(220, 170, 70)),
                );
            }
        }
        if cfg.lime.aux.role == LimeAuxRole::Diversity {
            ui.label(
                egui::RichText::new(tr!("settings-radio-nothing-here-can-tell-wanted")).weak(),
            );
        }
    }

    ui.add_space(6.0);
    ui.separator();
    ui.label(egui::RichText::new(tr!("settings-radio-transmit")).strong());
    crate::chrome::checkbox(ui, &mut cfg.lime.tx_enabled, tr!("settings-radio-enabled"))
        .on_hover_text(tr!("settings-radio-off-interface-publishes-no-transmit"));
    if cfg.lime.tx_enabled {
        egui::Grid::new("lime-tx-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label(tr!("settings-radio-transmit-gain"));
            if crate::chrome::slider(
                ui,
                egui::Slider::new(
                    &mut cfg.lime.tx_gain_db,
                    LimeConfig::GAIN_MIN_DB..=LimeConfig::GAIN_MAX_DB,
                )
                .suffix(" dB")
                .step_by(1.0),
            )
            .changed()
            {
                cmds.push(Command::SetGain {
                    dir: Direction::Tx,
                    element: LimeConfig::TX_GAIN_ELEMENT.to_string(),
                    db: cfg.lime.tx_gain_db,
                });
            }
            ui.end_row();

            ui.label(tr!("settings-radio-transmit-port"))
                .on_hover_text(tr!("settings-radio-board-s-two-transmit-sockets"));
            let text = if cfg.lime.antenna_tx.is_empty() {
                tr!("settings-radio-automatic")
            } else {
                LimeConfig::tx_port_label(cfg.lime.channel, &cfg.lime.antenna_tx)
            };
            let before_tx = cfg.lime.antenna_tx.clone();
            let chan = cfg.lime.channel;
            egui::ComboBox::from_id_salt("lime-anttx").selected_text(text).show_styled(ui, |ui| {
                ui.selectable_value(
                    &mut cfg.lime.antenna_tx,
                    String::new(),
                    tr!("settings-radio-automatic"),
                );
                for a in ["BAND1", "BAND2"] {
                    ui.selectable_value(
                        &mut cfg.lime.antenna_tx,
                        a.to_string(),
                        LimeConfig::tx_port_label(chan, a),
                    );
                }
            });
            // Immediately, for the same reason the receive socket is.
            if cfg.lime.antenna_tx != before_tx {
                if cfg.lime.antenna_tx.is_empty() {
                    *apply = true;
                } else {
                    cmds.push(Command::SetAntenna {
                        dir: Direction::Tx,
                        name: cfg.lime.antenna_tx.clone(),
                    });
                }
            }
            ui.end_row();
        });
        // The drive default is the bottom of the range, on purpose — an armed
        // transmitter should not be able to surprise anybody. Left there it is
        // a radio that keys, reports no error and puts out microwatts, which
        // reads downstream as a transmitter that does not work at all.
        if cfg.lime.tx_gain_db < LimeConfig::LOW_DRIVE_DB {
            ui.label(
                egui::RichText::new(tr!(
                    "settings-radio-lime-tx-gain-low",
                    db = format!("{:.0}", cfg.lime.tx_gain_db)
                ))
                .color(egui::Color32::from_rgb(220, 170, 70)),
            );
        }
        ui.label(
            egui::RichText::new(tr!("settings-radio-limesdr-transmits-about-100-khz"))
                .color(egui::Color32::from_rgb(220, 170, 70)),
        );
    }

    ui.add_space(6.0);
    ui.separator();
    ui.label(egui::RichText::new(tr!("settings-radio-limerfe-front-end")).strong());

    // Everything below reaches an open board through one setting rather than a
    // control at a time — see `LimeConfig::RFE_SETTING`. Snapshotted here and
    // compared once at the end of the section, so a control added to the panel
    // is live without any plumbing of its own; before this the connectors, the
    // band and the relay mode had no door at all and only took effect on the
    // next restart (issue #94).
    let rfe_before = cfg.lime.rfe.clone();

    egui::Grid::new("lime-rfe-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr!("settings-radio-connected"));
        egui::ComboBox::from_id_salt("lime-rfe-link")
            .selected_text(cfg.lime.rfe.link.tr())
            .show_styled(ui, |ui| {
                for l in RfeLink::ALL {
                    ui.selectable_value(&mut cfg.lime.rfe.link, l, l.tr());
                }
            });
        ui.end_row();

        if cfg.lime.rfe.link == RfeLink::Serial {
            ui.label(tr!("settings-radio-serial-port"));
            probe_only(ui, can_probe, |ui| {
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("lime-rfe-port")
                        .selected_text(if cfg.lime.rfe.serial.path.is_empty() {
                            "—".to_string()
                        } else {
                            cfg.lime.rfe.serial.path.clone()
                        })
                        .show_styled(ui, |ui| {
                            for p in serial_ports {
                                ui.selectable_value(&mut cfg.lime.rfe.serial.path, p.clone(), p);
                            }
                        });
                    ui.label(
                        egui::RichText::new(tr!("settings-radio-limerfe-s-own-micro-usb")).weak(),
                    );
                });
            });
            ui.end_row();
        }
    });

    if cfg.lime.rfe.link != RfeLink::Off {
        egui::Grid::new("lime-rfe-grid2").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label(tr!("settings-radio-receive-connector"));
            egui::ComboBox::from_id_salt("lime-rfe-prx")
                .selected_text(cfg.lime.rfe.port_rx.label())
                .show_styled(ui, |ui| {
                    for p in RfePort::RX_PORTS {
                        ui.selectable_value(&mut cfg.lime.rfe.port_rx, p, p.label());
                    }
                });
            ui.end_row();

            ui.label(tr!("settings-radio-transmit-connector"));
            egui::ComboBox::from_id_salt("lime-rfe-ptx")
                .selected_text(cfg.lime.rfe.port_tx.label())
                .show_styled(ui, |ui| {
                    for p in RfePort::TX_PORTS {
                        ui.selectable_value(&mut cfg.lime.rfe.port_tx, p, p.label());
                    }
                });
            ui.end_row();

            ui.label(tr!("settings-radio-band"));
            ui.horizontal(|ui| {
                crate::chrome::checkbox(
                    ui,
                    &mut cfg.lime.rfe.follow_band,
                    tr!("settings-radio-follow-dial"),
                )
                .on_hover_text(tr!("settings-radio-switch-filters-match-operating-frequency"));
                if !cfg.lime.rfe.follow_band {
                    egui::ComboBox::from_id_salt("lime-rfe-chan")
                        .selected_text(cfg.lime.rfe.channel.tr())
                        .show_styled(ui, |ui| {
                            for c in RfeChannel::ALL {
                                ui.selectable_value(&mut cfg.lime.rfe.channel, c, c.tr());
                            }
                        });
                }
            });
            ui.end_row();

            ui.label(tr!("settings-radio-relays"));
            egui::ComboBox::from_id_salt("lime-rfe-mode")
                .selected_text(cfg.lime.rfe.mode.tr())
                .show_styled(ui, |ui| {
                    for m in RfeModeControl::ALL {
                        ui.selectable_value(&mut cfg.lime.rfe.mode, m, m.tr());
                    }
                });
            ui.end_row();

            ui.label(tr!("settings-radio-receive-attenuator"));
            let mut steps = cfg.lime.rfe.atten_steps;
            if crate::chrome::slider(
                ui,
                egui::Slider::new(&mut steps, 0..=RFE_ATTEN_MAX_STEPS)
                    .custom_formatter(|v, _| format!("{} dB", v as u8 * RFE_ATTEN_STEP_DB)),
            )
            .changed()
            {
                cfg.lime.rfe.atten_steps = steps;
            }
            ui.end_row();

            ui.label(tr!("settings-radio-other"));
            ui.horizontal(|ui| {
                crate::chrome::checkbox(
                    ui,
                    &mut cfg.lime.rfe.notch,
                    tr!("settings-radio-notch-filter"),
                );
                ui.checkbox(&mut cfg.lime.rfe.fan, tr!("settings-radio-fan"))
                    .on_hover_text(tr!("settings-radio-worth-having-any-sustained-transmitting"));
            });
            ui.end_row();
        });

        // What the current cabling costs, said before it is discovered by
        // keying into a closed relay.
        if let Some(note) = cfg.lime.rfe.switching_note() {
            ui.add_space(4.0);
            ui.label(egui::RichText::new(note).color(egui::Color32::from_rgb(220, 170, 70)));
        }
        // Not amber: two connectors is what the board is for, and the default.
        // It is said at all because with one antenna it is also the quietest
        // way to have a transmitter that reaches nothing.
        if let Some(note) = cfg.lime.rfe.connector_note() {
            ui.add_space(4.0);
            ui.label(egui::RichText::new(note).weak());
        }
        if let Some(refusal) = cfg.lime.rfe.tx_refusal() {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(tr!("settings-radio-tx-blocked", reason = refusal.to_string()))
                    .color(egui::Color32::from_rgb(220, 170, 70)),
            );
        }

        // What this cabling can actually reach, resolved through exactly the
        // functions the driver runs — so what it says here is what the board
        // will be told. A channel the chosen connector cannot reach is not an
        // error the operator should have to discover by keying.
        if cfg.lime.rfe.follow_band {
            let mut unreachable_rx: Vec<&str> = Vec::new();
            let mut unreachable_tx: Vec<&str> = Vec::new();
            for (label, hz) in [
                ("HF", 14.2e6),
                ("6 m", 50.2e6),
                ("2 m", 145.5e6),
                ("1.25 m", 222.0e6),
                ("70 cm", 432.1e6),
                ("33 cm", 915.0e6),
                ("23 cm", 1296.0e6),
                ("13 cm", 2400.0e6),
                ("9 cm", 3400.0e6),
            ] {
                let want = sdroxide_types::channel_for(hz);
                if sdroxide_types::rx_port_check(cfg.lime.rfe.port_rx, want) != want {
                    unreachable_rx.push(label);
                }
                if sdroxide_types::tx_port_check(cfg.lime.rfe.port_tx, want) != want {
                    unreachable_tx.push(label);
                }
            }
            if !unreachable_rx.is_empty() || !unreachable_tx.is_empty() {
                ui.add_space(4.0);
                let mut lines = Vec::new();
                if !unreachable_rx.is_empty() {
                    lines.push(tr!(
                        "settings-radio-rfe-rx-fallback",
                        port = cfg.lime.rfe.port_rx.label(),
                        bands = unreachable_rx.join(", ")
                    ));
                }
                if !unreachable_tx.is_empty() {
                    lines.push(tr!(
                        "settings-radio-rfe-tx-fallback",
                        port = cfg.lime.rfe.port_tx.label(),
                        bands = unreachable_tx.join(", ")
                    ));
                }
                ui.label(
                    egui::RichText::new(lines.join("\n"))
                        .color(egui::Color32::from_rgb(220, 170, 70)),
                );
            }
        }
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(tr!("settings-radio-automatic-board-receives-switched-transmit"))
                .weak(),
        );
    }

    // The link and the port are what force a rebuild (they are in `before`
    // below), so they are deliberately not compared here: everything else about
    // the front end applies to the board that is already open.
    if cfg.lime.rfe != rfe_before
        && cfg.lime.rfe.link == rfe_before.link
        && cfg.lime.rfe.serial.path == rfe_before.serial.path
    {
        cmds.push(Command::SetDeviceSetting {
            key: LimeConfig::RFE_SETTING.to_string(),
            value: cfg.lime.rfe.to_setting(),
        });
    }

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if ui
            .button(tr!("settings-radio-copy-diagnostic-report"))
            .on_hover_text(tr!("settings-radio-limesdr-support-has-not-been"))
            .clicked()
        {
            *copy_report = true;
        }
    });

    let after = (
        cfg.lime.device.clone(),
        cfg.lime.channel,
        cfg.lime.sample_rate_hz,
        cfg.lime.oversample,
        cfg.lime.tx_enabled,
        cfg.lime.fifo_ksamples,
        cfg.lime.rfe.link,
        cfg.lime.rfe.serial.path.clone(),
        // The second chain's stream is created at open and bound to its
        // channel, so turning it on or off is a rebuild. Everything else about
        // it is live.
        cfg.lime.aux.role,
    );
    if after != before {
        *apply = true;
    }

    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(tr!("settings-radio-gains-filters-corrections-antenna-sockets")).weak(),
    );
}
