//! The Radio tab: one section per radio interface.
//!
//! Which section is drawn follows the interface selector above it, so the
//! dialog only ever shows the settings of the backend being configured. The
//! discovery buttons (HPSDR scan, RTL-SDR rescan, TCI connection test) only
//! set a flag — the blocking work happens after the window closure.

use eframe::egui::{self, Color32, ComboBox, DragValue, RichText, Slider};
use sdroxide_types::{Command, Direction};

use crate::app::SdroxideApp;
use crate::app::settings::enum_combo;
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
    ui.label("Antenna").on_hover_text(
        "Which socket on the back the radio is using — its own ANT command, the \
         same setting as the ANT button on the front panel.\n\n\
         Applies immediately, and it is the radio's setting rather than a copy \
         kept here: the socket the radio is on is read back when the session \
         opens.\n\n\
         The choice is remembered per band. Switch to the beam on 2 m and the \
         wire on 40, and each comes back the next time the dial crosses into \
         that band — and the next time sdroxide starts. A memory channel \
         stored here carries its socket too.",
    );
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
    ui.label("Radio power").on_hover_text(
        "Switch the radio itself off, and back on again, over the control \
         link — not sdroxide's own on/off, which closes the interface and \
         leaves the radio running.\n\n\
         For the switch back on to reach anything, the radio's control end has \
         to stay awake while it is off. Over the network that is what \
         Network Control does. Over a serial cable it is the radio's CI-V \
         port, which stays powered from the mains supply on a set that is \
         switched off at the front rather than unplugged; sdroxide sends the \
         wake-up run Icom's own documentation asks for in front of the \
         power-on.\n\n\
         Switching off ends the audio and the meters — there is no radio \
         behind them — and sdroxide keeps the control link open so the \
         switch back on has somewhere to go.",
    );
    ui.horizontal(|ui| {
        if ui.button("On").clicked() {
            cmds.push(Command::SetRigPower(true));
        }
        if ui.button("Off").clicked() {
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
    // families here whose rigs have two.
    antenna_rx: &str,
    can_probe: bool,
    cmds: &mut Vec<Command>,
) {
    use sdroxide_types::{
        CAT_SCOPE_MIN_BAUD, CatFamily, CwKeying, DigiMode, Direction, ELAD_CAT_BAUDS,
        ELAD_DEFAULT_CAT_BAUD, EladAntenna, EladTxInput, IcomModel, IcomScopeSpan, KenwoodSend,
        LineState, ModeControl, Parity, PttMethod, QMX_IQ_OFFSET_HZ, QMX_IQ_RATE_HZ, SoundFormat,
        StopBits,
    };
    let Some(cfg) = radio_edit.as_mut() else {
        ui.label("Waiting for the configuration of the machine the radio is attached to.");
        return;
    };
    egui::Grid::new("cat-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        // What the sound format and the family were before this frame's combo
        // boxes ran. A QMX's I/Q is not centred on its dial and its card runs
        // at one rate, and both are filled in when the operator arrives at that
        // combination — see where they are applied, below the family picker.
        let format_before = cfg.cat.format;
        let family_before = cfg.cat.family;

        ui.label("Sound format");
        enum_combo(ui, "sfmt", &mut cfg.cat.format, &SoundFormat::ALL, SoundFormat::label);
        ui.end_row();

        // Only meaningful for I/Q: demod audio is a real signal, with no
        // sideband to swap.
        if matches!(cfg.cat.format, SoundFormat::Iq) {
            ui.label("Invert spectrum");
            crate::chrome::checkbox(ui, &mut cfg.cat.invert_spectrum, "Swap I/Q").on_hover_text(
                "Mirror the panadapter about the tuned frequency, for a rig that \
                 carries I and Q the other way round on its sound card. The \
                 giveaway is a waterfall full of convincing signals that are all \
                 on the wrong side of the dial, with SSB coming out on the \
                 opposite sideband — swapping the two cables at the sound card \
                 would fix it just as well.\n\n\
                 Receive only: transmit hands the radio one real audio signal, \
                 which has no sideband to invert.",
            );
            ui.end_row();

            ui.label("I/Q sample rate").on_hover_text(
                "How fast the radio's I/Q sound card is run — and so how much \
                 band the panadapter shows, because a quadrature stream spans \
                 its whole sample rate: 48 kHz gives ±24 kHz either side of the \
                 dial, 192 kHz gives ±96.\n\n\
                 The card has the final say. One that cannot do the rate picked \
                 here is opened at the nearest it can, and the panadapter is \
                 that much narrower — the log says so at startup when it \
                 happens.\n\n\
                 A faster card is also more work: the machine has proportionally \
                 less time to empty it, and one that cannot keep up drops \
                 samples. That shows up as audio breaking up while the waterfall \
                 still looks perfect, and it too is logged. If you see it, come \
                 back down a step.",
            );
            // Labelled with the span each rate buys as well as the rate itself:
            // the width is the reason to touch this, and it is the half of it
            // an operator cannot work out at a glance.
            enum_combo(ui, "iqrate", &mut cfg.cat.iq_rate_hz, &sdroxide_types::CAT_IQ_RATES, |r| {
                match r {
                    48_000 => "48 kHz (±24 kHz)",
                    96_000 => "96 kHz (±48 kHz)",
                    192_000 => "192 kHz (±96 kHz)",
                    384_000 => "384 kHz (±192 kHz)",
                    _ => "—",
                }
            });
            ui.end_row();

            ui.label("I/Q centre offset").on_hover_text(
                "How far the radio's I/Q output is centred above its own dial, \
                 for a rig whose receive I.F. has been moved off zero — on an \
                 Elecraft, MENU:RX SHFT set to 8.0 instead of NOR, which takes \
                 the dial off the mixer's DC spike (and stops a strong nearby \
                 SSB/AM station being AM-detected).\n\n\
                 Not a converter: the radio still displays and transmits on the \
                 real frequency, and nothing here is ever sent to it. This only \
                 says where the samples on the sound card sit, and the stream is \
                 shifted back onto the dial as it arrives.\n\n\
                 Leave at 0 unless you have turned such a menu entry on. If \
                 signals land twice the offset away, the sign is the other way \
                 round.",
            );
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

            ui.label("IQ correction").on_hover_text(
                "Cancel the mirror image of every signal, and the DC spike in \
                 the middle of the waterfall.\n\n\
                 A radio's I/Q output is two analogue paths that are never \
                 quite equal in gain nor exactly 90° apart, and what that \
                 leaves is a copy of every signal reflected about the tuned \
                 frequency, usually 30-40 dB down — strong enough to look like \
                 a station that is not there, and to be decoded as one. Radios \
                 with front-panel balance trimmers are adjusting exactly this; \
                 here it is measured off the received noise and needs no \
                 setting.\n\n\
                 Leave it on. Turn it off if you are listening to AM tuned \
                 dead on the carrier — the carrier is DC, so it goes with the \
                 spike — or to check whether a signal is real by watching \
                 whether it survives.",
            );
            crate::chrome::checkbox(ui, &mut cfg.cat.iq_correction, "Cancel mirror images");
            ui.end_row();

            ui.label("DC notch").on_hover_text(
                "Widen the hole taken out of the middle of the span, for a \
                 radio whose centre spike is broader than the offset \
                 underneath it.\n\n\
                 0 leaves the ordinary blocker, which is a few tens of hertz \
                 wide and removes the offset without touching anything else. \
                 Wind it up and the bottom of the span goes with it: a \
                 first-order high-pass, 3 dB down at the figure set here and \
                 further in below it.\n\n\
                 It is centred where the radio's I/Q is centred, which is the \
                 dial unless the offset above has moved it — so anything tuned \
                 there goes too. A CW note at 600 Hz is inside a 600 Hz \
                 setting.",
            );
            ui.add(
                DragValue::new(&mut cfg.cat.iq_dc_block_hz)
                    .speed(10.0)
                    .range(0.0..=sdroxide_types::CAT_IQ_DC_BLOCK_MAX_HZ)
                    .suffix(" Hz"),
            );
            ui.end_row();
        }

        if matches!(cfg.cat.format, SoundFormat::DemodAudio) {
            ui.label("Panadapter BW");
            ui.add(
                DragValue::new(&mut cfg.cat.audio_bw_hz)
                    .speed(100.0)
                    .range(1000.0..=24000.0)
                    .suffix(" Hz"),
            );
            ui.end_row();
        }

        ui.label("CAT family");
        enum_combo(ui, "fam", &mut cfg.cat.family, &CatFamily::ALL, CatFamily::label);
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

        // A network family reaches the radio over a socket, so every serial
        // setting below is about a port nothing will open. Drawing them would
        // invite an operator to fix a connection problem by changing a baud
        // rate that has no part in it.
        let serial = !cfg.cat.family.is_network();

        if cfg.cat.family == CatFamily::Rigctld {
            ui.label("rigctld address").on_hover_text(
                "host:port of a running Hamlib rigctld — 127.0.0.1:4532 is its \
                 own default, on this machine.\n\n\
                 Start one with, for example, \
                 `rigctld -m 2028 -r /dev/ttyUSB0 -s 38400`, where -m is the \
                 Hamlib model number for your radio (`rigctl -l` lists them).\n\n\
                 This is the catch-all: it reaches the frequency, mode, PTT, \
                 power, S-meter and SWR and nothing else. Where one of the \
                 native families above fits your radio it does more — keying \
                 from the rig's own text buffer, the receive filter, and \
                 per-model meter scales.",
            );
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut cfg.cat.rigctld_addr)
                    .desired_width(200.0)
                    .hint_text("127.0.0.1:4532"),
            );
            ui.end_row();
        }

        if cfg.cat.family == CatFamily::Flrig {
            ui.label("flrig address").on_hover_text(
                "host:port of a running flrig — 127.0.0.1:12345 is its own \
                 default, on this machine. flrig serves XML-RPC whenever it is \
                 running; the port is under its Config → Setup → Server.\n\n\
                 Like the Hamlib option this drives a daemon rather than the \
                 radio, but through flrig's own per-model driver — on a number \
                 of rigs its power and filter handling is the more faithful of \
                 the two. It reaches the frequency, mode, PTT, transmit power \
                 (in whole watts), the receive bandwidth, the S-meter, SWR and \
                 power-out. CW keys through flrig's own cwio port, which must \
                 be configured in flrig itself.",
            );
            crate::chrome::field(
                ui,
                egui::TextEdit::singleline(&mut cfg.cat.flrig_addr)
                    .desired_width(200.0)
                    .hint_text("127.0.0.1:12345"),
            );
            ui.end_row();
        }

        if serial {
            ui.label("Serial port");
            let shown = if cfg.cat.serial.path.is_empty() {
                "— select —".to_string()
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
            ui.label("Baud");
            // Most families take whatever the operator has set at the radio, so
            // the whole spread is offered. An ELAD is the exception: an FDM-DUO's
            // CAT port has four rates and no others, and at any other one the
            // link is silent both ways rather than merely slow — so offering
            // 19200 here (which is also this block's own default) would be
            // offering a link that cannot work. See `sdroxide_cat::spawn`.
            let bauds: &[u32] = if cfg.cat.family == CatFamily::Elad {
                &ELAD_CAT_BAUDS
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
                            RichText::new(format!(
                                "the radio has no {} baud setting — {ELAD_DEFAULT_CAT_BAUD} \
                                 will be used instead",
                                cfg.cat.serial.baud,
                            ))
                            .color(crate::theme::YELLOW()),
                        )
                        .wrap(),
                    );
                }
            });
            ui.end_row();

            ui.label("Data bits");
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

            ui.label("Parity");
            enum_combo(ui, "parity", &mut cfg.cat.serial.parity, &Parity::ALL, Parity::label);
            ui.end_row();

            ui.label("Stop bits");
            enum_combo(ui, "stop", &mut cfg.cat.serial.stop_bits, &StopBits::ALL, StopBits::label);
            ui.end_row();

            ui.label("Force RTS");
            enum_combo(ui, "rts", &mut cfg.cat.serial.force_rts, &LineState::ALL, LineState::label);
            ui.end_row();
            ui.label("Force DTR");
            enum_combo(ui, "dtr", &mut cfg.cat.serial.force_dtr, &LineState::ALL, LineState::label);
            ui.end_row();
        }

        ui.label("PTT method").on_hover_text(if serial {
            "How transmit is keyed."
        } else {
            "How transmit is keyed. A network link has no control lines, so \
             DTR and RTS key nothing here — use CAT, which asks the daemon to \
             key the radio, or VOX."
        });
        enum_combo(ui, "ptt", &mut cfg.cat.ptt, &PttMethod::ALL, PttMethod::label);
        ui.end_row();

        ui.label("Mode control");
        enum_combo(ui, "modectl", &mut cfg.cat.mode_control, &ModeControl::ALL, ModeControl::label);
        ui.end_row();

        ui.label("Digimode mode");
        enum_combo(ui, "digimode", &mut cfg.cat.digi_mode, &DigiMode::ALL, DigiMode::label);
        ui.end_row();

        ui.label("CW keying").on_hover_text(
            "How the CW panel's keyer transmits. \"Rig keyer\" puts the radio in CW \
             and hands it the text to send with its own keyer. It uses the rig's \
             keyer speed (set from the panel's WPM), and on Yaesu \
             it sends by way of keyer memory 1, overwriting whatever was stored in it.\n\n\
             It needs break-in on at the radio, which sdroxide turns on itself with each \
             message on Icom, Yaesu and Kenwood — a rig with break-in off takes the message \
             and transmits nothing. It is left on afterwards, and a full break-in is never \
             turned down to semi.\n\n\
             \"Sound card\" sends the keyed sidetone as audio instead (MCW), a tone at \
             dial + pitch — and because a rig in CW would ignore its sound card \
             entirely, selecting CW then follows the Digimode mode setting (USB, DATA, \
             or Radio controlled) instead of switching the rig to CW. Pick \"Radio \
             controlled\" there for rigs whose data position can't be commanded over \
             CAT (a Xiegu's U-D) and park the rig on it yourself, as for FT8.",
        );
        enum_combo(ui, "cwkey", &mut cfg.cat.cw_keying, &CwKeying::ALL, CwKeying::label);
        ui.end_row();

        ui.label("Poll rate").on_hover_text(
            "How often the radio is asked what it is doing — its dial, its mode \
             and its meters. This is the half of the control link that runs from \
             the radio back to sdroxide: turn the rig's own VFO knob or change \
             its mode on the front panel and the readout, the band and the \
             panadapter follow within one poll.\n\n\
             It is also the whole of the control traffic this end generates, \
             and on a modern Icom that traffic is not free: the CI-V port and \
             the sound card sit behind one USB hub inside the radio, so frames \
             asked for here are bus time the audio does not get — heard as \
             dropouts that look like a DSP fault and are not one. The default \
             of 2 Hz is half a second behind the knob and quiet enough to stay \
             out of the audio's way. Raise it where the control port is its own \
             device (a separate USB-serial adapter, a network rigctld); lower \
             it if the radio's audio breaks up.\n\n\
             An Icom with CI-V Transceive switched on in its own menu needs \
             none of this for the dial: it reports the knob the instant it \
             moves, and sdroxide stands the dial poll down as soon as it sees \
             it do so — and picks it back up again if the radio ever moves \
             without saying so.\n\n\
             The rate is the dial's. The mode rides along with only every \
             fourth poll, since it is a setting that changes a few times in an \
             evening rather than one that follows a knob.",
        );
        ui.add(DragValue::new(&mut cfg.cat.poll_hz).speed(0.5).range(0.5..=20.0).suffix(" Hz"));
        ui.end_row();

        if cfg.cat.family == CatFamily::Kenwood {
            ui.label("Send command").on_hover_text(
                "Which transceiver generation keys the rig, for PTT method \
                 \"CAT\". The two disagree about what the TX parameter means \
                 and nothing on the wire tells them apart, so pick the one \
                 that matches your radio.\n\n\
                 \"TS-2000 style (TX;)\" — TS-480, TS-570, TS-870, TS-2000: \
                 the ordinary send, on the main band.\n\n\
                 \"TS-590 style (TX1;)\" — TS-590S/SG, TS-890, TS-990: DATA \
                 SEND, which keys with the ACC2/USB audio input live. On these \
                 rigs the plain send selects the microphone instead and mutes \
                 the audio sdroxide transmits.\n\n\
                 Set wrong, a TS-590 transmits silence — but a TS-2000 \
                 transmits on the sub-band, which is another band entirely.",
            );
            enum_combo(
                ui,
                "kwsend",
                &mut cfg.cat.kenwood_send,
                &KenwoodSend::ALL,
                KenwoodSend::label,
            );
            ui.end_row();
        }

        if cfg.cat.family == CatFamily::Elecraft {
            ui.label("Radio");
            ui.label(RichText::new("K3 · K3S · KX3 · KX2 · K4").weak()).on_hover_text(
                "One profile covers the family: the K3 command set, which the \
                 KX3, KX2 and K4 all answer. There is nothing to pick here — \
                 how many watts the Drive slider spans (12 on a bare KX2 or \
                 KX3, 110 with a KPA3 or a KXPA100) is read from the rig's own \
                 option-module query when the port opens.\n\n\
                 Note the baud rates above: the K3, K3S, KX3 and KX2 go no \
                 faster than 38400, and a rig set below the rate chosen here \
                 answers nothing at all.",
            );
            ui.end_row();
        }

        if cfg.cat.family == CatFamily::Elad {
            ui.label("Transmit input").on_hover_text(
                "Where the radio takes its transmit audio from — the rig's TI \
                 command, which is menu 32 \"TX IN\" at the front panel. \
                 Asserted when the port opens.\n\n\
                 \"USB audio\" is what makes this interface work: the FDM-DUO \
                 transmits what sdroxide puts into its USB sound card. A radio \
                 left on \"Microphone\" sends the room instead, and nothing on \
                 screen says so.\n\n\
                 \"Auto\" lets the rig choose — the microphone for a PTT press \
                 on the microphone, the USB port for a CAT or RTS key-down.\n\n\
                 \"Leave as set on the radio\" sends no TI at all, for an \
                 operator who sets this at the front panel.",
            );
            enum_combo(
                ui,
                "eladtxin",
                &mut cfg.cat.elad_tx_input,
                &EladTxInput::ALL,
                EladTxInput::label,
            );
            ui.end_row();

            ui.label("Antenna").on_hover_text(
                "Which of the two sockets on the back the receiver listens on — \
                 the rig's AN command, which is menu 31 \"ANTENNAS\" at the \
                 front panel and the \"ANT 1 2\" indicator on its display.\n\n\
                 \"RTX\" is one antenna doing both jobs, on the M-type socket \
                 that also carries transmit. \"RX only\" moves receive to the \
                 second socket and leaves transmit on RTX — a receiving \
                 antenna, a loop or a beverage, with the beam still on the \
                 transmitter.\n\n\
                 Applies immediately, and is read back from the radio when the \
                 port opens: this is the rig's own setting rather than a copy \
                 kept here.",
            );
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

            ui.label("Radio");
            ui.label(RichText::new("FDM-DUO · FDM-DUOr").weak()).on_hover_text(
                "One profile covers both. There is nothing to pick here — the \
                 radio names itself when the port opens.\n\n\
                 This is the CAT half only: it drives the dial, the mode, PTT, \
                 the S-meter, the SWR and the transmit power over the rig's \
                 CAT USB port, and takes audio from its USB Audio port. The \
                 FDM-DUO's third USB port carries wideband I/Q, which this \
                 interface does not read — select the ELAD interface above for \
                 that.\n\n\
                 Note the baud rate: menu 70 \"CAT BAUD\" on the radio must \
                 match, and it ships at 38400.\n\n\
                 CW is keyed by the radio's own key or paddle. The FDM-DUO has \
                 no command that accepts text, so the CW panel cannot key it \
                 over CAT.",
            );
            ui.end_row();
        }

        if cfg.cat.family == CatFamily::QrpLabs {
            ui.label("Radio");
            ui.label(RichText::new("QMX · QMX+ · QDX").weak()).on_hover_text(
                "One profile covers the range: QRP Labs' own command set, which \
                 is a subset of the Kenwood TS-480's with a good deal added. \
                 There is nothing to pick here — the radio names itself and its \
                 firmware version when the port opens, and the version is what \
                 decides whether the SWR-protection read is asked for at all.\n\n\
                 Not the Kenwood profile: on a QMX the PC command is the power \
                 METER rather than the power control, so a radio driven as a \
                 Kenwood would have its meter read written to as if it were a \
                 setting, and MD8 — a Kenwood mode — is SWR Tune here.\n\n\
                 The transmit power is set at the radio. There is no CAT command \
                 for it, so the Drive slider only reaches the level of the audio \
                 going into the sound card.\n\n\
                 The receive filter is the radio's too: it reports the width its \
                 mode implies (3.2 kHz in Digi, 300 Hz in CW) and offers nothing \
                 to change it with.",
            );
            ui.end_row();

            ui.label("");
            ui.label(RichText::new("Baud rate is ignored — the port is USB").weak()).on_hover_text(
                "A QMX serves its own virtual COM ports over USB, so the rate \
                 set above has no effect on either end. It offers up to three of \
                 them; if the radio answers nothing, the most likely reason is \
                 the wrong one.\n\n\
                 One thing does matter about the port: a carriage return sent to \
                 it switches the radio into terminal mode for the rest of the \
                 session. sdroxide never sends one — but a terminal program left \
                 open on the same port will, and CAT stops working the moment it \
                 does.",
            );
            ui.end_row();

            if matches!(cfg.cat.format, SoundFormat::Iq) {
                ui.label("");
                ui.label(RichText::new("I/Q mode is switched on at the radio").weak())
                    .on_hover_text(
                        "The radio's sound card carries either demodulated audio \
                         or the raw I/Q its ADC sees, and sdroxide asserts \
                         whichever the Sound format above asks for when the port \
                         opens (the radio's Q9 command, the \"IQ mode\" menu \
                         entry).\n\n\
                         The centre offset and sample rate above have been set to \
                         what a QMX needs: its receiver is a superhet with a \
                         12 kHz I.F., so the synthesiser sits 12 kHz below the \
                         dial and everything on the card is 12 kHz above the \
                         middle of the span. The card runs at 48 kHz, which makes \
                         the panadapter 48 kHz wide.\n\n\
                         In CW the radio moves that I.F. by a further ~700 Hz, \
                         which one figure cannot follow — add it by hand if you \
                         run the panadapter with the radio in CW. In Digi, where \
                         a radio used as an I/Q front end normally sits, there is \
                         nothing to add.\n\n\
                         QRP Labs note that I/Q mode is not suitable for WSJT-X \
                         and other programs that expect demodulated audio; here \
                         the demodulation happens on this side, so it is.",
                    );
                ui.end_row();
            }
        }

        if cfg.cat.family == CatFamily::Icom {
            ui.label("Radio model").on_hover_text(
                "Which Icom, for the two things CI-V does not do the same way \
                 on all of them.\n\n\
                 The transceiver address below is filled in from it: every \
                 model ships with a different one, and a frame sent to the \
                 wrong address is simply ignored — a radio that answers \
                 nothing, with no error anywhere to say why.\n\n\
                 It also decides whether sdroxide can select DATA mode. On \
                 CI-V, USB and USB-DATA are the same mode byte and a separate \
                 command tells them apart; without it a digital-mode over goes \
                 out through the microphone input, with the rig's speech \
                 processing and SSB filter in the path.\n\n\
                 \"Other\" leaves the address to you and sends no DATA-mode \
                 command at all.",
            );
            let before = cfg.cat.icom_model;
            enum_combo(ui, "icommodel", &mut cfg.cat.icom_model, &IcomModel::ALL, IcomModel::label);
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
            radio_power_row(ui, caps, cmds);
        }

        if matches!(cfg.cat.family, CatFamily::Icom | CatFamily::Xiegu) {
            ui.label("Radio ID (hex)");
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
            crate::chrome::checkbox(ui, &mut cfg.cat.scope, "Show the radio's spectrum scope")
                .on_hover_text(format!(
                    "Streams the radio's own scope sweep over the CI-V link and draws it \
                     as the panadapter — the only picture of the band a demod-audio rig \
                     can give, since the audio it sends is what already came through its \
                     filter.\n\n\
                     Needs the radio set up for it: CI-V USB Baud Rate to 115200 and \
                     CI-V USB Port to \"Unlink from [REMOTE]\" (both under SET > \
                     Connectors > CI-V), and the baud rate above set to match — below \
                     {CAT_SCOPE_MIN_BAUD} the sweeps do not fit down the link and the \
                     scope stays off.\n\n\
                     The sweeps share the radio's internal USB bus with its own sound \
                     card. If received audio starts to drop out with the scope on, this \
                     box is the first thing to try turning off.",
                ));
            ui.end_row();

            if cfg.cat.scope {
                ui.label("Scope span").on_hover_text(
                    "How wide to sweep it. The radio keeps whatever span was last chosen \
                     on its own screen — often a few kHz. Setting a span here also puts \
                     the scope into centre mode, so it follows the dial. It changes the \
                     radio's own display too; \"As set on the radio\" leaves it alone.",
                );
                ComboBox::from_id_salt("cat_scope_span")
                    .selected_text(cfg.cat.scope_span.label())
                    .show_styled(ui, |ui| {
                        for sp in IcomScopeSpan::ALL {
                            if ui.selectable_label(cfg.cat.scope_span == sp, sp.label()).clicked() {
                                cfg.cat.scope_span = sp;
                            }
                        }
                    });
                ui.end_row();
            }
        }
    });
    ui.add_space(6.0);
    ui.label(RichText::new("Press \"Apply / reconnect\" to switch without a restart.").weak());
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
        ui.label("Waiting for the configuration of the machine the radio is attached to.");
        return;
    };
    egui::Grid::new("hpsdr-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Devices");
        // The scan goes out on this machine's LAN; the radio is on the
        // engine's. The manual IP below is typed, so it still works from here.
        probe_only(ui, can_probe, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Discover").clicked() {
                    *discover = true;
                }
                let shown = cfg.hpsdr.selected_ip.clone().unwrap_or_else(|| "— none —".into());
                ComboBox::from_id_salt("hpsdr_dev").width(320.0).selected_text(shown).show_styled(
                    ui,
                    |ui| {
                        if devices.is_empty() {
                            ui.label(RichText::new("no devices — press Discover").weak());
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

        ui.label("Manual IP");
        let mut ip = cfg.hpsdr.manual_ip.clone().unwrap_or_default();
        let resp = crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut ip)
                .desired_width(160.0)
                .hint_text("optional, e.g. 192.168.1.50"),
        );
        if resp.changed() {
            let t = ip.trim();
            cfg.hpsdr.manual_ip = if t.is_empty() { None } else { Some(t.to_string()) };
        }
        ui.end_row();

        ui.label("Sample rate");
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
        ui.label("Receiver (DDC)");
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
            .on_hover_text(
                "A Protocol 2 board carries several independently tunable receivers (DDCs) on \
                 one connection — run this radio on DDC1 and another radio, same address, on \
                 DDC2. The transmitter belongs to the DDC1 radio. Protocol 1 boards have DDC1 \
                 only.",
            );
        ui.end_row();

        ui.label("LNA gain").on_hover_text(
            "Front-end gain of a Hermes-Lite 2. Takes effect immediately — no reconnect — \
             and is remembered as the level the radio starts at. Too high clips the ADC and \
             the whole band looks distorted; too low and the receiver goes deaf.",
        );
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

        ui.label("Filter board").on_hover_text(
            "Accessory board on the Hermes-Lite 2's J16 header (or the open-collector \
             outputs of any other openHPSDR board — Protocol 2 included). \"N2ADR\" picks \
             one relay per band; \"Alex / Hermes band code\" puts the band number on \
             outputs 1-4, which is what an ANAN's Alex board, a Zeus SDR, a HiQSDR and \
             Quisk expect. \"Custom\" opens a table below where you state the seven \
             outputs yourself, band by band, on receive and on transmit — for an antenna \
             switch, an amplifier's band decoder or anything else neither preset fits; \
             either preset can be poured into it as a starting point. Leave this at \
             \"None\" unless something really is fitted: those seven pins are \
             general-purpose open-collector outputs, and operators also wire them to \
             amplifier PTT, antenna relays and transverter switching. Driving them from \
             band data would start operating whatever is connected. Applies on \
             Apply / reconnect.",
        );
        ComboBox::from_id_salt("hpsdr_filter")
            .width(220.0)
            .selected_text(cfg.hpsdr.filter_board.label())
            .show_styled(ui, |ui| {
                for b in sdroxide_types::HpsdrFilterBoard::ALL {
                    if ui.selectable_label(cfg.hpsdr.filter_board == b, b.label()).clicked() {
                        cfg.hpsdr.filter_board = b;
                    }
                }
            });
        ui.end_row();

        ui.label("IO board RX input").on_hover_text(
            "Where an N2ADR HL2IOBoard takes its receive signal from. The board itself is found \
             automatically and needs no setting; this one exists only for operators who have \
             wired its own SMA jacks. Leave it at \"Radio's own input\" otherwise — selecting the \
             IO board's J9 with nothing connected to it leaves the receiver deaf. Applies on \
             Apply / reconnect.",
        );
        ComboBox::from_id_salt("hpsdr_io_rx")
            .width(220.0)
            .selected_text(cfg.hpsdr.io_rx_input.label())
            .show_styled(ui, |ui| {
                for m in sdroxide_types::HpsdrIoRxInput::ALL {
                    if ui.selectable_label(cfg.hpsdr.io_rx_input == m, m.label()).clicked() {
                        cfg.hpsdr.io_rx_input = m;
                    }
                }
            });
        ui.end_row();

        ui.label("Power amplifier");
        crate::chrome::checkbox(ui, &mut cfg.hpsdr.pa_enable, "Use the Hermes-Lite 2's onboard PA")
            .on_hover_text(
                "On by default, and what you want unless an external amplifier is driven from the \
             board's low-power RF1 output. With it off the radio still keys — the T/R relay \
             throws and any accessory board follows — but the antenna jack makes no power at \
             all, and the relay is deliberately held in receive. Ignored on boards other than a \
             Hermes-Lite.",
            );
        ui.end_row();

        ui.label("Invert spectrum");
        crate::chrome::checkbox(ui, &mut cfg.hpsdr.invert_spectrum, "Swap I/Q").on_hover_text(
            "Mirror the board's spectrum about the tuned frequency, on transmit as well \
             as receive. On by default: a Hermes-Lite 2 needs it. Turn it off only if \
             signals show up on the wrong side of the dial and nothing decodes — the \
             giveaway is a waterfall full of convincing traces while SSB lands on the \
             wrong sideband and FT8 returns no decodes at all.",
        );
        ui.end_row();

        ui.label("Frequency correction")
            .on_hover_text("Crystal/TCXO error in ppm, applied to RX and TX. Applies immediately.");
        let mut ppm = cfg.hpsdr.ppm;
        let resp =
            ui.add(egui::DragValue::new(&mut ppm).range(-100.0..=100.0).speed(0.1).suffix(" ppm"));
        if resp.changed() {
            cfg.hpsdr.ppm = ppm;
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: HpsdrConfig::PPM_ELEMENT.to_string(),
                db: ppm,
            });
        }
        ui.end_row();

        ui.label("Transmit buffer");
        ui.add(
            egui::DragValue::new(&mut cfg.hpsdr.tx_latency_ms)
                .range(HpsdrConfig::TX_LATENCY_MS_RANGE)
                .suffix(" ms"),
        )
        .on_hover_text(
            "How far ahead of real time transmit audio is fed to the board over the network, \
             before sdroxide slows down to match it. The board itself holds no such buffer — \
             this only widens sdroxide's own margin. Raise it on a WiFi link or a VPN, where \
             the low default (right for a direct wired connection) is not enough headroom \
             against jitter and the transmitted audio or PTT stutters; higher costs transmit \
             latency. Takes effect on APPLY, which reconnects to the board.",
        );
        ui.end_row();

        // ── PureSignal ──
        ui.label("PureSignal").on_hover_text(
            "Adaptive predistortion: linearise the transmitter from a sample of what it actually \
             emitted, for twenty-odd decibels less intermodulation at the same power. The \
             receiver is the feedback path — the board keeps receiving through an over — so a \
             directional coupler and an attenuator have to put a sample of the amplifier's \
             output into an input the T/R switch does not take away on transmit. On a \
             Hermes-Lite 2 that means the IO board's PureSignal jack, and the receive input \
             above set to match. With nothing coupled in, the loop never locks and the \
             transmitter is left exactly as it would have been. Applies on Apply / reconnect, \
             and only on the radio that owns the transmitter (DDC1).",
        );
        crate::chrome::checkbox(
            ui,
            &mut cfg.hpsdr.puresignal,
            "Correct the transmitter from the receiver's own feedback",
        );
        ui.end_row();

        ui.add_enabled_ui(cfg.hpsdr.puresignal, |ui| {
            ui.label("  Table steps").on_hover_text(
                "How finely the amplifier's curve is modelled. More steps follow a sharper knee \
                 and take longer to fill in; 32 is a sensible start.",
            );
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
            ui.label("  Adaptation").on_hover_text(
                "How fast the correction follows what the coupler reports. Slow is right — this \
                 is averaging an amplifier's curve, which does not move, out of a feedback path \
                 that has noise in it.",
            );
        });
        ui.add_enabled_ui(cfg.hpsdr.puresignal, |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::Slider::new(&mut cfg.hpsdr.ps_rate, 0.0..=1.0).show_value(false));
                crate::chrome::checkbox(ui, &mut cfg.hpsdr.ps_frozen, "Hold").on_hover_text(
                    "Stop adapting and keep the correction where it is — for measuring, and for \
                     an operator happy with what it has learned.",
                );
            });
        });
        ui.end_row();
    });
    hpsdr_oc_table(ui, cfg);
    ui.add_space(6.0);
    ui.label(
        RichText::new(
            "A manual IP overrides discovery. Press \"Apply / reconnect\" to switch without a restart.",
        )
        .weak(),
    );
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
        RichText::new("Open-collector outputs by band")
            .size(14.0)
            .strong()
            .color(crate::theme::CYAN()),
    );
    ui.add_space(2.0);
    ui.label(
        RichText::new(
            "One control word per band, as your hardware's documentation states it: bit 0 is \
             output 1, bit 6 is output 7. RX is asserted while receiving on that band and TX \
             while the transmitter is keyed — give them the same value for a filter, and \
             different ones for anything that belongs on one side of the changeover only \
             (an amplifier's key line, a receive preamplifier's bypass). Bands left at 00 \
             assert nothing. Applies on Apply / reconnect.",
        )
        .weak(),
    );
    ui.add_space(6.0);

    ui.horizontal(|ui| {
        for (preset, name) in
            [(HpsdrFilterBoard::N2adr, "N2ADR"), (HpsdrFilterBoard::Alex, "ALEX BAND CODE")]
        {
            if ui
                .button(format!("FILL FROM {name}"))
                .on_hover_text(
                    "Replace the table with what this preset would send on every band, as a \
                     starting point to edit. Nothing else in the configuration changes.",
                )
                .clicked()
            {
                cfg.hpsdr.oc_table = HpsdrOcRow::from_preset(preset);
            }
        }
        if ui
            .add_enabled(!cfg.hpsdr.oc_table.is_empty(), egui::Button::new("CLEAR"))
            .on_hover_text("Every output off on every band, which is what \"None\" does.")
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
            for h in ["Band", "RX", "TX", "Outputs asserted"] {
                ui.label(RichText::new(h).weak().size(10.0));
            }
            ui.end_row();
            for band in bands {
                let i = cfg.hpsdr.oc_table.iter().position(|r| r.band == band);
                let (mut rx, mut tx) = i
                    .map(|i| (cfg.hpsdr.oc_table[i].rx, cfg.hpsdr.oc_table[i].tx))
                    .unwrap_or((0, 0));
                if band == Band::Gen {
                    ui.label("Other").on_hover_text(
                        "Everywhere outside the amateur bands — short-wave listening, and \
                         anything a transverter's dial lands on that no band covers.",
                    );
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
    if rx == tx { list(rx) } else { format!("{} / TX {}", list(rx), list(tx)) }
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
        ui.label("Waiting for the configuration of the machine the radio is attached to.");
        return;
    };

    egui::Grid::new("rtlsdr-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Dongle");
        // Which dongle is the one row here that names a USB bus rather than the
        // radio. Everything below reaches the dongle wherever it is plugged in.
        probe_only(ui, can_probe, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button("Rescan")
                    .on_hover_text(
                        "Re-list the USB bus. No device is opened, so this is safe \
                         to press while receiving.",
                    )
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
                            ui.label(RichText::new("no dongles — press Rescan").weak());
                        }
                        if ui
                            .selectable_label(cfg.rtlsdr.serial.is_none(), "— first one found —")
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

        ui.label("Sample rate").on_hover_text(
            "The RTL2832U's resampler reaches 225–300 kHz and 900 kHz–3.2 MHz, \
             nothing between. Takes effect on Apply.",
        );
        let shown = format!("{:.3} Msps", cfg.rtlsdr.sample_rate_hz / 1e6);
        ComboBox::from_id_salt("rtlsdr_rate").selected_text(shown).show_styled(ui, |ui| {
            for &r in &RtlSdrConfig::SAMPLE_RATES {
                let sel = (cfg.rtlsdr.sample_rate_hz - r).abs() < 1.0;
                let mut label = format!("{:.3} Msps", r / 1e6);
                if r >= 3_200_000.0 {
                    label.push_str("  (often drops samples)");
                }
                if ui.selectable_label(sel, label).clicked() {
                    cfg.rtlsdr.sample_rate_hz = r;
                }
            }
        });
        ui.end_row();

        ui.label("AGC").on_hover_text(
            "Manual is the setting for measurement and weak-signal digital modes. \
             The tuner and the demodulator have independent automatic loops.",
        );
        let mut agc = cfg.rtlsdr.agc;
        enum_combo(ui, "rtlsdr_agc", &mut agc, &RtlSdrAgc::ALL, RtlSdrAgc::label);
        if agc != cfg.rtlsdr.agc {
            cfg.rtlsdr.agc = agc;
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: RtlSdrConfig::AGC_ELEMENT.to_string(),
                db: agc.code() as f64,
            });
        }
        ui.end_row();

        ui.label("Tuner gain").on_hover_text(
            "Applies immediately — no reconnect. The tuner has 29 discrete steps, \
             so the value snaps to the nearest one it can produce. Ignored while \
             the tuner AGC is running.",
        );
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

        ui.label("Frequency correction").on_hover_text(
            "Crystal error in parts per million. Run with \
             RUST_LOG=sdroxide_rtlsdr=debug and the log prints the measured \
             clock error after about 20 seconds — that is the number to enter. \
             Applies immediately.",
        );
        let mut ppm = cfg.rtlsdr.ppm;
        if ui.add(egui::DragValue::new(&mut ppm).range(-200..=200).suffix(" ppm")).changed() {
            cfg.rtlsdr.ppm = ppm;
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: RtlSdrConfig::PPM_ELEMENT.to_string(),
                db: ppm as f64,
            });
        }
        ui.end_row();

        ui.label("HF reception").on_hover_text(
            "The tuner itself starts at 24 MHz. An RTL-SDR Blog V4 upconverts \
             below that in hardware; other dongles reach HF only by sampling the \
             ADC directly, through the V3's HF port. Automatic hands everything \
             below 24 MHz to the ADC and everything above it to the tuner.\n\n\
             Direct sampling covers every HF band, 17 m and 15 m included — they \
             arrive in the ADC's second Nyquist zone, the right way up. Nothing \
             filters the ADC's input, though, so whatever is at 28.8 MHz minus \
             the dial comes with them: 10.7 MHz under 17 m, 7.726 MHz under \
             15 m.\n\n\
             Switching modes briefly interrupts the stream.",
        );
        let mut hf = cfg.rtlsdr.hf_mode;
        enum_combo(ui, "rtlsdr_hf", &mut hf, &RtlSdrHfMode::ALL, RtlSdrHfMode::label);
        if hf != cfg.rtlsdr.hf_mode {
            cfg.rtlsdr.hf_mode = hf;
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: RtlSdrConfig::HF_MODE_ELEMENT.to_string(),
                db: hf as u8 as f64,
            });
        }
        ui.end_row();

        ui.label("IQ correction").on_hover_text(
            "Removes the dongle's own DC spike from the centre of the span, and \
             the mirror image every signal leaves reflected about it, by \
             measuring the imbalance in the samples themselves — no calibration, \
             and it applies immediately. The tuner has no offset-tuning mode, so \
             this is the only way to clear the centre.\n\n\
             An AM carrier tuned dead on the dial is at DC too, so it goes with \
             the spike: tune a kilohertz off it, or switch this off.",
        );
        let mut iq = cfg.rtlsdr.iq_correction;
        if crate::chrome::checkbox(ui, &mut iq, "Remove the centre spike and mirror image")
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

        ui.label("Bias tee");
        let mut bias = cfg.rtlsdr.bias_tee;
        if crate::chrome::checkbox(ui, &mut bias, "Feed ~4.5 V DC up the coax").changed() {
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
            RichText::new(
                "Bias tee is ON. Never connect a transceiver, a grounded antenna, \
                 or a preamp powered from elsewhere while this is enabled — the DC \
                 goes straight down the feedline.",
            )
            .color(crate::theme::YELLOW()),
        );
    }

    ui.add_space(4.0);
    ui.label(
        RichText::new(
            "Receive only. The dongle and sample rate take effect on Apply; \
             everything else applies as you change it.",
        )
        .weak(),
    );
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
        ui.label("Waiting for the configuration of the machine the radio is attached to.");
        return;
    };

    egui::Grid::new("rtltcp-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Server address").on_hover_text(
            "Where rtl_tcp is listening: an address, or an address and port. \
             The port defaults to 1234, which is rtl_tcp's own default.\n\n\
             On the far end, start it as `rtl_tcp -a 0.0.0.0` — bound to \
             127.0.0.1, which is what it does with no -a, it only accepts \
             connections from that same machine.\n\nTakes effect on Apply.",
        );
        crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut cfg.rtltcp.address)
                .desired_width(220.0)
                .hint_text("host or host:port, e.g. raspberrypi.local:1234"),
        );
        ui.end_row();

        ui.label("Sample rate").on_hover_text(
            "Requested of the server, with the same resampler limits as a local \
             dongle — it is the same silicon on the far end.\n\n\
             The figure beside each rate is what it costs on the link: the \
             samples are sent uncompressed, and a rate the network cannot carry \
             makes rtl_tcp drop the connection rather than degrade. \
             Takes effect on Apply.",
        );
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
                        label.push_str("  (wired link)");
                    }
                    if ui.selectable_label(sel, label).clicked() {
                        cfg.rtltcp.sample_rate_hz = r;
                    }
                }
            },
        );
        ui.end_row();

        ui.label("AGC").on_hover_text(
            "Runs on the server's dongle. Manual is the setting for measurement \
             and weak-signal digital modes.",
        );
        let mut agc = cfg.rtltcp.agc;
        enum_combo(ui, "rtltcp_agc", &mut agc, &RtlSdrAgc::ALL, RtlSdrAgc::label);
        if agc != cfg.rtltcp.agc {
            cfg.rtltcp.agc = agc;
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: RtlSdrConfig::AGC_ELEMENT.to_string(),
                db: agc.code() as f64,
            });
        }
        ui.end_row();

        ui.label("Tuner gain").on_hover_text(
            "Applies immediately — no reconnect. Sent in tenths of a dB and \
             snapped by the server to a step its tuner has; the protocol has no \
             replies, so what it settled on cannot be read back and this slider \
             keeps showing what was asked for. Ignored while the tuner AGC is \
             running.",
        );
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

        ui.label("Frequency correction").on_hover_text(
            "Crystal error of the *server's* dongle, in parts per million — a \
             property of that hardware, so it is set here and not on this \
             machine's dongles. Applies immediately.\n\n\
             The measured clock error the USB interface prints is not available \
             here: over a network what that measurement sees is the buffering, \
             not the crystal, and it is wrong by thousands of ppm. Calibrate the \
             dongle on USB once and carry the number over, or tune a broadcast \
             station of known frequency and adjust until it sits on the dial.",
        );
        let mut ppm = cfg.rtltcp.ppm;
        if ui.add(egui::DragValue::new(&mut ppm).range(-200..=200).suffix(" ppm")).changed() {
            cfg.rtltcp.ppm = ppm;
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: RtlSdrConfig::PPM_ELEMENT.to_string(),
                db: ppm as f64,
            });
        }
        ui.end_row();

        ui.label("HF reception").on_hover_text(
            "The tuner starts at 24 MHz; below that the far end needs help. A \
             Blog V4 upconverts by itself and Automatic leaves it alone — which \
             is the only thing it can do, since the protocol reports the tuner \
             chip and nothing else, and a V4 is indistinguishable from a plain \
             R828D over the wire.\n\n\
             On a V3 or any other dongle, Automatic switches the server to \
             direct sampling below the tuner's own 24 MHz floor — which covers \
             every HF band, 17 m and 15 m arriving in the ADC's second Nyquist \
             zone with 28.8 MHz minus the dial folded on top. Choose Direct \
             sampling explicitly for a plain R828D that hears nothing on HF. \
             Switching briefly interrupts the stream.",
        );
        let mut hf = cfg.rtltcp.hf_mode;
        enum_combo(ui, "rtltcp_hf", &mut hf, &RtlSdrHfMode::ALL, RtlSdrHfMode::label);
        if hf != cfg.rtltcp.hf_mode {
            cfg.rtltcp.hf_mode = hf;
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: RtlSdrConfig::HF_MODE_ELEMENT.to_string(),
                db: hf as u8 as f64,
            });
        }
        ui.end_row();

        ui.label("IQ correction").on_hover_text(
            "Removes the dongle's DC spike and mirror image here, from the \
             samples as they arrive — they are artefacts of the hardware, so \
             they travel over the network with everything else. Applies \
             immediately.\n\n\
             An AM carrier tuned dead on the dial sits at DC too, so it goes \
             with the spike: tune a kilohertz off it, or switch this off.",
        );
        let mut iq = cfg.rtltcp.iq_correction;
        if crate::chrome::checkbox(ui, &mut iq, "Remove the centre spike and mirror image")
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

        ui.label("Bias tee").on_hover_text(
            "Powers a preamp from the far end's dongle. Older servers do not \
             implement the command and ignore it silently — the protocol has no \
             way to say no — so a bias tee that does not come on is not \
             necessarily this end's doing.",
        );
        let mut bias = cfg.rtltcp.bias_tee;
        if crate::chrome::checkbox(ui, &mut bias, "Feed ~4.5 V DC up the remote coax").changed() {
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
            RichText::new(
                "Bias tee is ON, on hardware that is somewhere else. Whatever is \
                 on the other end of that feedline — a transceiver, a grounded \
                 antenna, a preamp already powered — is not in front of you to \
                 check.",
            )
            .color(crate::theme::YELLOW()),
        );
    }

    ui.add_space(8.0);
    ui.separator();
    ui.label(RichText::new("SDRplay server (rsp_tcp)").strong());
    ui.label(
        RichText::new(
            "An SDRplay server greets exactly like a dongle, so these are shown \
             always rather than when one is detected. Against an ordinary \
             rtl_tcp server they are simply ignored — the protocol has no \
             replies and discards commands it does not know. The Device tab \
             names the server as rsp_tcp once one has identified itself.",
        )
        .weak(),
    );
    ui.add_space(4.0);

    egui::Grid::new("rsptcp-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Antenna");
        ui.horizontal(|ui| {
            for (v, name) in [(0u8, "Input A"), (1, "Input B"), (2, "Hi-Z")] {
                if ui.selectable_label(cfg.rtltcp.rsp_antenna == v, name).clicked() {
                    cfg.rtltcp.rsp_antenna = v;
                    push_gain(cmds, RtlTcpConfig::RSP_ANTENNA_ELEMENT, v as f64);
                }
            }
        });
        ui.end_row();

        ui.label("LNA state");
        if crate::chrome::slider(ui, egui::Slider::new(&mut cfg.rtltcp.rsp_lna_state, 0..=9))
            .on_hover_text(
                "A step index, not a dB figure: how much each step is worth \
                 depends on the RSP model and the band. 0 is the most gain.",
            )
            .changed()
        {
            push_gain(cmds, RtlTcpConfig::RSP_LNA_STATE_ELEMENT, cfg.rtltcp.rsp_lna_state as f64);
        }
        ui.end_row();

        ui.label("IF gain reduction");
        if crate::chrome::slider(
            ui,
            egui::Slider::new(&mut cfg.rtltcp.rsp_if_gain_reduction, 20..=59).suffix(" dB"),
        )
        .on_hover_text(
            "A reduction, so more is less signal. Only obeyed with the RSP's \
                 AGC off.",
        )
        .changed()
        {
            push_gain(
                cmds,
                RtlTcpConfig::RSP_IFGR_ELEMENT,
                cfg.rtltcp.rsp_if_gain_reduction as f64,
            );
        }
        ui.end_row();

        ui.label("AGC");
        ui.horizontal(|ui| {
            if crate::chrome::checkbox(ui, &mut cfg.rtltcp.rsp_agc, "Enable").changed() {
                push_gain(cmds, RtlTcpConfig::RSP_AGC_ELEMENT, cfg.rtltcp.rsp_agc as u8 as f64);
            }
            ui.add_enabled_ui(cfg.rtltcp.rsp_agc, |ui| {
                if ui
                    .add(
                        egui::DragValue::new(&mut cfg.rtltcp.rsp_agc_setpoint)
                            .range(-72..=0)
                            .suffix(" dBfs"),
                    )
                    .on_hover_text("The level the RSP's AGC aims to hold.")
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

        ui.label("Notches");
        ui.horizontal(|ui| {
            let mut mask = cfg.rtltcp.rsp_notch;
            for (bit, name) in [
                (RtlTcpConfig::RSP_NOTCH_AM, "AM"),
                (RtlTcpConfig::RSP_NOTCH_BROADCAST, "FM"),
                (RtlTcpConfig::RSP_NOTCH_DAB, "DAB"),
                (RtlTcpConfig::RSP_NOTCH_RF, "RF"),
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

        ui.label("Reference out");
        if ui
            .checkbox(&mut cfg.rtltcp.rsp_ref_out, "24 MHz clock out")
            .on_hover_text("RSP2 and RSPduo only.")
            .changed()
        {
            push_gain(cmds, RtlTcpConfig::RSP_REF_OUT_ELEMENT, cfg.rtltcp.rsp_ref_out as u8 as f64);
        }
        ui.end_row();
    });

    ui.add_space(4.0);
    ui.label(
        RichText::new(
            "Receive only. The address and sample rate take effect on Apply; \
             everything else applies as you change it. A dropped connection is \
             retried on its own, so a server that is restarted comes back \
             without touching anything here.",
        )
        .weak(),
    );
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
        ui.label("Waiting for the configuration of the machine the radio is attached to.");
        return;
    };
    let cfg = if vfo { &mut radio.spyserver_vfo } else { &mut radio.spyserver };
    let salt = if vfo { "spyservervfo" } else { "spyserver" };

    egui::Grid::new(format!("{salt}-grid")).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Server address").on_hover_text(
            "Where spyserver is listening: an address, or an address and port. \
             The port defaults to 5555, which is spyserver's own default.\n\n\
             On the far end, check that its config file binds an address other \
             machines can reach — bound to 127.0.0.1 it only accepts \
             connections from that same machine.\n\nTakes effect on Apply.",
        );
        crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut cfg.address)
                .desired_width(220.0)
                .hint_text("host or host:port, e.g. raspberrypi.local:5555"),
        );
        ui.end_row();

        ui.label("I/Q bandwidth").on_hover_text(if vfo {
            "How wide a slice of the band arrives as I/Q — and so how much of \
             the link this uses. This is the *demodulated* window: it follows \
             the dial, and everything wider than it is the server's FFT in the \
             strip above the panadapter.\n\n\
             Automatic aims at about 96 kHz, which carries every mode here \
             including wide FM and still fits down a cellular uplink. \
             Takes effect on Apply."
        } else {
            "Which stage of the server's own rate ladder to ask for. Every \
             receiver has a different ladder — it is its maximum rate halved \
             stage by stage — so this is stored as the stage rather than as a \
             figure in hertz, and the same setting still means something \
             sensible pointed at a different server.\n\n\
             Automatic aims at about 1 Msps. Press Test connection to see what \
             the stages come to on this server. Takes effect on Apply."
        });
        let shown = if cfg.iq_decimation < 0 {
            let target = if vfo {
                SpyServerConfig::VFO_TARGET_RATE_HZ
            } else {
                SpyServerConfig::WIDEBAND_TARGET_RATE_HZ
            };
            format!("Automatic (nearest {:.0} kHz)", target / 1e3)
        } else {
            format!(
                "Stage {} — the server's rate ÷ {}",
                cfg.iq_decimation,
                1u32 << cfg.iq_decimation.min(20)
            )
        };
        ComboBox::from_id_salt(format!("{salt}_decim"))
            .width(260.0)
            .selected_text(shown)
            .show_styled(ui, |ui| {
                if ui.selectable_label(cfg.iq_decimation < 0, "Automatic").clicked() {
                    cfg.iq_decimation = SpyServerConfig::AUTO_DECIMATION;
                }
                // The ladder's real depth is the server's, and it is not known
                // here. Sixteen stages is past what any of them offer; a stage
                // this server has not got falls back to the nearest it has, and
                // says so in the log.
                for stage in 0..16i32 {
                    let sel = cfg.iq_decimation == stage;
                    let label = format!("Stage {stage} — rate ÷ {}", 1u32 << stage.min(20));
                    if ui.selectable_label(sel, label).clicked() {
                        cfg.iq_decimation = stage;
                    }
                }
            });
        ui.end_row();

        ui.label("Sample format").on_hover_text(
            "Bits per component on the wire, and so what this costs on the \
             link: 16-bit is twice 8-bit, and 32-bit float is four times it for \
             no more information than the receiver's ADC had.\n\n\
             8-bit is what makes a remote receiver work over a domestic uplink \
             and is right for almost everything. A server may override this — \
             some are configured to insist on one format — and says so in the \
             log when it does. Takes effect on Apply.",
        );
        let mut fmt = cfg.iq_format;
        enum_combo(
            ui,
            &format!("{salt}_fmt"),
            &mut fmt,
            &SpyServerFormat::ALL,
            SpyServerFormat::label,
        );
        cfg.iq_format = fmt;
        ui.end_row();

        ui.label("Gain").on_hover_text(
            "The server's gain stage, as an index — not a number of decibels. \
             What each index means belongs to the receiver on the far end and \
             changes with the band, and the protocol never says, so nothing \
             here can turn it into dB without inventing a figure.\n\n\
             The real range is the server's and is only known once connected; \
             an index past it is clamped. Applies immediately — no reconnect.",
        );
        let mut gain = cfg.gain_index as i32;
        if ui.add(egui::DragValue::new(&mut gain).range(0..=45).prefix("index ")).changed() {
            cfg.gain_index = gain.max(0) as u32;
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: SpyServerConfig::GAIN_ELEMENT.to_string(),
                db: f64::from(cfg.gain_index),
            });
        }
        ui.end_row();

        ui.label("Digital gain").on_hover_text(
            "How far the server scales its samples up before quantising them \
             for the wire. Automatic computes it the way every other client \
             does — from the receiver type, the gain index and the decimation \
             stage — and is almost always right.\n\n\
             It matters most at 8 bits: a signal sitting far below full scale \
             loses its lower bits to the quantiser, and the scaling is what \
             puts it back. Applies immediately.",
        );
        ui.horizontal(|ui| {
            let mut auto = cfg.auto_digital_gain;
            if crate::chrome::checkbox(ui, &mut auto, "Automatic").changed() {
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

        ui.label("Full-band strip").on_hover_text(if vfo {
            "The server's own FFT of the whole band, drawn in the strip above \
             the panadapter. In this interface it is the only band view there \
             is — the panadapter itself is only as wide as the I/Q being \
             received.\n\n\
             Switching it off leaves a receiver with no way to see anything it \
             is not already tuned to. Applies immediately, with a short break \
             in the audio while the server changes what it is sending."
        } else {
            "Ask the server for a low-rate FFT of the whole band as well as \
             the I/Q, and draw it in the strip above the panadapter.\n\n\
             It costs almost nothing — a couple of kilobytes a frame, a dozen \
             or so times a second — and it shows the whole receiver rather \
             than the slice being demodulated. Worth switching off only on a \
             link where every byte counts. Applies immediately, with a short \
             break in the audio."
        });
        ui.horizontal(|ui| {
            let mut on = cfg.fft_enabled;
            if crate::chrome::checkbox(ui, &mut on, "Show").changed() {
                cfg.fft_enabled = on;
                cmds.push(Command::SetGain {
                    dir: Direction::Rx,
                    element: SpyServerConfig::FFT_ENABLED_ELEMENT.to_string(),
                    db: f64::from(u8::from(on)),
                });
            }
            ui.add_enabled_ui(cfg.fft_enabled, |ui| {
                let shown = if cfg.fft_decimation == 0 {
                    "whole band".to_string()
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
                                "whole band".to_string()
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
                    .on_hover_text(
                        "How much of the receiver the strip covers. The whole \
                         band is the widest view there is; narrowing it puts \
                         the same number of bins across less spectrum, which \
                         is finer detail over a smaller stretch.",
                    );
            });
        });
        ui.end_row();

        ui.label("Strip dB window").on_hover_text(
            "The server quantises its FFT into this window before sending it, \
             one byte a bin — so these decide how finely the strip is \
             measured, not just how it is drawn. The floor and ceiling the \
             strip is *displayed* with are set by the engine's own \
             auto-levelling and are a separate thing.\n\n\
             The default 150 dB is the whole protocol range and needs no \
             attention. Narrowing it around the noise floor buys resolution on \
             a receiver whose signals all sit in a small part of the scale. \
             Applies immediately.",
        );
        ui.horizontal(|ui| {
            ui.add_enabled_ui(cfg.fft_enabled, |ui| {
                if ui
                    .add(
                        egui::DragValue::new(&mut cfg.fft_db_offset)
                            .range(
                                SpyServerConfig::FFT_DB_OFFSET_MIN
                                    ..=SpyServerConfig::FFT_DB_OFFSET_MAX,
                            )
                            .prefix("top ")
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
                            .prefix("range ")
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

        ui.label("I/Q correction").on_hover_text(
            "Remove the DC spike and the mirror image in DSP, on this side. \
             Whether the receiver on the far end needs it depends on what it \
             is — an Airspy HF+ does not, an RTL-SDR does — and the protocol \
             does not say which it is talking to. Applies immediately.",
        );
        let mut iq = cfg.iq_correction;
        if crate::chrome::checkbox(ui, &mut iq, "Enabled").changed() {
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
                .button("Test connection")
                .on_hover_text(
                    "Connect, read what the server says about its receiver, and \
                     disconnect again — without starting a stream, so it is safe \
                     to press against a server somebody else is using.",
                )
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
            "Receive only. The panadapter is the narrow I/Q window, which follows the dial; \
             the band view is the server's FFT in the strip above it. Press \"Apply / \
             reconnect\" to switch without a restart."
        } else {
            "Receive only. Wideband I/Q, the same as any local SDR. Press \"Apply / reconnect\" \
             to switch without a restart."
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
        ui.label("Waiting for the configuration of the machine the radio is attached to.");
        return;
    };
    let cfg = &mut radio.kiwi;

    egui::Grid::new("kiwi-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Receiver address").on_hover_text(
            "Where the receiver serves its web page: an address, or an address \
             and port. The port defaults to 8073, which is a KiwiSDR's own.\n\n\
             Note that a receiver reached through the project's proxy \
             (something.proxy.kiwisdr.com — nearly half the public ones) answers \
             on port 80 instead, so give it explicitly. Picking a receiver from \
             \"Public SDRs\" fills this in correctly.\n\nTakes effect on Apply.",
        );
        crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut cfg.address)
                .desired_width(240.0)
                .hint_text("host or host:port, e.g. kiwi.example.org:8073"),
        );
        ui.end_row();

        ui.label("Password").on_hover_text(
            "The receiver's *user* password, where its operator has set one. \
             Blank on almost every public receiver. Not the admin password.\n\n\
             Takes effect on Apply.",
        );
        crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut cfg.password).password(true).desired_width(160.0),
        );
        ui.end_row();

        ui.label("Announce as").on_hover_text(
            "The name this connection shows up under, which the receiver's \
             owner and everybody else listening to it can see.\n\n\
             Left blank it is your station callsign, which is the network's own \
             convention — some operators do block clients that will not \
             identify. Takes effect on Apply.",
        );
        crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut cfg.ident)
                .desired_width(160.0)
                .hint_text("blank = station callsign"),
        );
        ui.end_row();

        ui.label("Band view").on_hover_text(
            "Ask the receiver for its own waterfall as well as its I/Q, and \
             draw it in the strip above the panadapter.\n\n\
             Worth having: the I/Q is only about 12 kHz wide, so without this \
             there is nothing to tune *by*. It is a second connection carrying \
             roughly 20 kB/s at full speed, against the I/Q's 44 kB/s.\n\n\
             Takes effect on Apply.",
        );
        let mut wide = cfg.wide_lane;
        crate::chrome::checkbox(ui, &mut wide, "show the receiver's 0-30 MHz waterfall");
        cfg.wide_lane = wide;
        ui.end_row();

        ui.label("Band view speed").on_hover_text(
            "How often the receiver sends a waterfall row, 1 (slowest) to 4. \
             The only setting here that changes what the link costs while \
             running, so turn it down on a metered connection.\n\n\
             Applies immediately.",
        );
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

        ui.label("Band view span").on_hover_text(
            "How much band the receiver's waterfall covers. Its full 0-30 MHz \
             at the left of the slider, halving with each step to the right.\n\n\
             The waterfall is always 1024 bins wide however wide the window is, \
             so this is really a resolution control: the whole band is 29 kHz to \
             a bin, which is a band *map* rather than a picture of anything, \
             while a few hundred kilohertz shows individual stations in a \
             broadcast band. Zoomed in, the window follows your dial.\n\n\
             The whole band is the default and is what makes the strip a thing \
             to tune by; the panadapter below it is the ~12 kHz of I/Q. \
             Applies immediately.",
        );
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

        ui.label("Receiver AGC").on_hover_text(
            "The *receiver's* AGC, which is on the far side of the link and \
             ahead of the I/Q — so it acts before anything sdroxide does.\n\n\
             On by default, unlike every local SDR here, because the measured \
             alternative was worse: on a live receiver the manual gain was not \
             monotonic across its range and its top end clipped the I/Q at full \
             scale, while the AGC held about -24 dBFS with 7 dB of headroom.\n\n\
             It does mean the sample amplitude is not a signal level, which is \
             why the S-meter is read from the receiver's own figure instead. \
             Applies immediately.",
        );
        let mut agc = cfg.agc;
        if crate::chrome::checkbox(ui, &mut agc, "let the receiver ride its own gain").changed() {
            cfg.agc = agc;
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: KiwiConfig::AGC_ELEMENT.to_string(),
                db: f64::from(u8::from(agc)),
            });
        }
        ui.end_row();

        ui.label("Manual gain").on_hover_text(
            "Fixed gain when the receiver's AGC is off, on its own 0-90 scale. \
             Not decibels of anything stated — the protocol calls it manGain \
             and says no more, and it was measured not to be monotonic, so \
             treat it as a dial to find a good spot on rather than a \
             calibration. Applies immediately.",
        );
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
                .button("Test connection")
                .on_hover_text(
                    "Read the receiver's status page: what it is, what it \
                     covers, how many channels are free, and whether its \
                     operator allows connections from apps other than a \
                     browser. Takes none of its channels.",
                )
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
        RichText::new(format!(
            "Receive only — this is somebody else's antenna. The panadapter is the ~12 kHz \
             the receiver sends as I/Q, which follows the dial; the band view above it is its \
             own waterfall, and tuning across it retunes the receiver. About {:.0} kB/s while \
             connected. Press \"Apply / reconnect\" to switch without a restart.",
            cfg.link_kbytes_s(),
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
        ui.label("Waiting for the configuration of the machine the radio is attached to.");
        return;
    };
    egui::Grid::new("tci-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Server address");
        crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut cfg.tci.address)
                .desired_width(220.0)
                .hint_text("host:port, e.g. 127.0.0.1:50001"),
        );
        ui.end_row();

        ui.label("IQ sample rate");
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
        ui.label("Receiver");
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
            .on_hover_text(
                "A rig with two receivers (SunSDR2DX) can serve two radio tabs from one \
                 connection — run this radio on RX1 and another on RX2. The transmitter \
                 belongs to the RX1 radio.",
            );
        ui.end_row();

        // How far the rig's IQ runs behind the `dds:` that moved it. The
        // panadapter labels each frame with the centre its *samples* were taken
        // at, and this is the only number that says which centre that was — so
        // it belongs to the rig, not to sdroxide, and the default is a
        // measurement of one particular rig rather than a constant.
        ui.label("Stream delay");
        crate::chrome::slider(
            ui,
            egui::Slider::new(&mut cfg.tci.stream_delay_ms, 0.0..=400.0).suffix(" ms").step_by(1.0),
        )
        .on_hover_text(
            "How long the rig's IQ takes to arrive on a new centre after sdroxide moves it. \
             It is the rig's own DSP pipeline, not the network — a server on this machine \
             has one too — and while the panadapter is dragged fully zoomed out, being wrong \
             by this much moves the newest waterfall rows sideways of the history by the \
             error times the speed of the drag. Too high displaces them exactly as far as \
             too low, the other way. Measure it with `cargo run --release -p sdroxide-tci \
             --example retune_latency`; the default is a SunSDR2DX on ExpertSDR3 at 192 kHz.",
        );
        ui.end_row();

        ui.label("");
        // The test opens its own socket from wherever it is pressed, so a
        // green answer here would only say this screen can reach the rig — a
        // different question from the one being asked.
        probe_only(ui, can_probe, |ui| {
            if ui.button("Test connection").clicked() {
                *tci_test = true;
            }
        });
        ui.end_row();
    });
    test_result_line(ui, test_result);
    ui.add_space(6.0);
    ui.label(
        RichText::new(
            "Wideband IQ receive, audio transmit. Press \"Apply / reconnect\" to switch without a restart.",
        )
        .weak(),
    );
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
    test: &mut bool,
    copy_report: &mut bool,
    test_result: &Option<crate::app::settings::TestOutcome>,
    can_probe: bool,
    cmds: &mut Vec<Command>,
) {
    use sdroxide_types::{CwKeying, IcomNetConfig, IcomRxSource, IcomScopeSpan};
    let Some(cfg) = radio_edit.as_mut() else {
        ui.label("Waiting for the configuration of the machine the radio is attached to.");
        return;
    };
    let net = &mut cfg.icomnet;

    egui::Grid::new("icomnet-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Radio address").on_hover_text(
            "The address shown on the radio under SET > Network. Network Control has to \
             be on there, and the radio needs a network user name and password set.",
        );
        crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut net.address)
                .desired_width(220.0)
                .hint_text("host or IP, e.g. 192.168.1.50"),
        );
        ui.end_row();

        ui.label("Control port");
        ui.add(egui::DragValue::new(&mut net.control_port).range(1..=65535))
            .on_hover_text("50001 unless it has been changed on the radio.");
        ui.end_row();

        ui.label("Network user");
        crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut net.username).desired_width(220.0),
        );
        ui.end_row();

        ui.label("Password").on_hover_text(
            "Stored in the clear in radio.json. The protocol obfuscates it reversibly on \
             the wire, so nothing here would make it a secret.",
        );
        crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut net.password).password(true).desired_width(220.0),
        );
        ui.end_row();

        ui.label("Receive from").on_hover_text(
            "AF: the radio demodulates and sdroxide shows the audio band. \
             12 kHz IF: the radio sends its DRM intermediate frequency instead and \
             sdroxide demodulates, which brings its own filters, noise reduction and \
             decoders to bear over about ±12 kHz. Either way the wide waterfall is the \
             radio's own scope — no Icom outputs I/Q.",
        );
        ComboBox::from_id_salt("icomnet_rx_source")
            .selected_text(net.rx_source.label())
            .show_styled(ui, |ui| {
                for s in IcomRxSource::ALL {
                    if ui.selectable_label(net.rx_source == s, s.label()).clicked() {
                        net.rx_source = s;
                    }
                }
            });
        ui.end_row();

        radio_antenna_row(ui, caps, antenna_rx, "icomnet_antenna", cmds);
        radio_power_row(ui, caps, cmds);

        ui.label("Audio sample rate");
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
                "A 12 kHz IF needs the 48000 Hz stream — at this rate the radio's \
                 demodulated audio is used instead.",
            );
            ui.end_row();
        }

        // Only on the IF path: the AF path is audio the radio has already
        // demodulated, and there is nothing left in it to mirror.
        if net.rx_source == IcomRxSource::If12k && net.if_mode_usable() {
            ui.label("IF spectrum").on_hover_text(
                "Which way round the radio's 12 kHz IF runs. \"Automatic\" uses what \
                 the model is known to do — mirrored on an IC-7760, normal on every \
                 other Icom. Set it by hand if SSB comes out on the wrong sideband: \
                 the giveaway is having to select USB where the band runs LSB while \
                 the radio's own mode display agrees with sdroxide throughout.",
            );
            ComboBox::from_id_salt("icomnet_invert_if")
                .selected_text(match net.invert_if {
                    None => "Automatic",
                    Some(false) => "Normal",
                    Some(true) => "Mirrored",
                })
                .show_styled(ui, |ui| {
                    for (v, label) in
                        [(None, "Automatic"), (Some(false), "Normal"), (Some(true), "Mirrored")]
                    {
                        if ui.selectable_label(net.invert_if == v, label).clicked() {
                            net.invert_if = v;
                        }
                    }
                });
            ui.end_row();
        }

        if net.rx_source == IcomRxSource::Af {
            ui.label("Displayed bandwidth");
            ui.add(
                egui::DragValue::new(&mut net.audio_bw_hz).range(1000.0..=24_000.0).suffix(" Hz"),
            )
            .on_hover_text(
                "Width of the audio-band panadapter, as for a CAT rig. Used in the \
                 digital modes and where the radio's scope is off; otherwise the \
                 panadapter is the scope, and Scope span sets its width.",
            );
            ui.end_row();
        }

        ui.label("CW keying").on_hover_text(
            "How the CW panel's keyer transmits. \"Rig keyer\" puts the radio in CW \
             and hands it the text to send with its own keyer (CI-V 0x17). \"Sound \
             card\" sends the keyed sidetone over the LAN as audio instead (MCW), a \
             tone at dial + pitch — and because a rig in CW would ignore its \
             modulator input, selecting CW then keeps the radio in plain USB, the \
             same mode the digital modes ride.",
        );
        ComboBox::from_id_salt("icomnet_cw").selected_text(net.cw_keying.label()).show_styled(
            ui,
            |ui| {
                for k in CwKeying::ALL {
                    if ui.selectable_label(net.cw_keying == k, k.label()).clicked() {
                        net.cw_keying = k;
                    }
                }
            },
        );
        ui.end_row();

        ui.label("Transmit buffer");
        ui.add(egui::DragValue::new(&mut net.tx_latency_ms).range(20..=1000).suffix(" ms"))
            .on_hover_text(
                "How much audio the radio holds before modulating. More survives a worse \
                 network, at the cost of transmit latency.",
            );
        ui.end_row();

        ui.label("");
        crate::chrome::checkbox(ui, &mut net.scope, "Show the radio's spectrum scope")
            .on_hover_text(
                "Streams the radio's own 475-bin sweep. On AF it is the panadapter — the \
             audio the radio sends is what came through its filter, not a picture of \
             the band — and on the 12 kHz IF it is the full-band waterfall above one. \
             Either way it is the radio's picture, not sdroxide's DSP: there is no I/Q \
             to compute one from.",
            );
        ui.end_row();

        if net.scope {
            ui.label("Scope span").on_hover_text(
                "How wide to sweep it. This is the only wide view an Icom has, and the \
                 radio keeps whatever span was last chosen on its own screen — often a \
                 few kHz, which is why the strip can come up no wider than the \
                 panadapter under it. Setting a span here also puts the scope into \
                 centre mode, so it follows the dial. It changes the radio's own \
                 display too; \"As set on the radio\" leaves it alone.",
            );
            ComboBox::from_id_salt("icomnet_scope_span")
                .selected_text(net.scope_span.label())
                .show_styled(ui, |ui| {
                    for sp in IcomScopeSpan::ALL {
                        if ui.selectable_label(net.scope_span == sp, sp.label()).clicked() {
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
            "Switch modulation input to LAN",
        )
        .on_hover_text(
            "Transmit audio is only heard when the radio's MOD input is set to LAN. \
                 sdroxide can write that on a model whose menu numbering it knows; on any \
                 other it says so and leaves the menu alone. It is a loan: whatever the \
                 radio held there is read first and put back when the session ends, so a \
                 rig used on its own afterwards still hears its own microphone.",
        );
        ui.end_row();

        ui.label("");
        // Both reach for this machine: the test opens its own socket from here,
        // and the trace is of the session *this* process ran. The engine's own
        // is on the engine's machine.
        probe_only(ui, can_probe, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Test connection").clicked() {
                    *test = true;
                }
                if ui
                    .button("Copy diagnostic report")
                    .on_hover_text(
                        "Copies this radio's last session — its handshake and CI-V trace — \
                         to the clipboard, for a bug report. This radio's, not the station's: \
                         with two Icoms on the LAN each tab answers about its own address.",
                    )
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
    ui.label(
        RichText::new(
            "Control, audio and the radio's own scope over one network connection — no \
             serial cable and no sound card. Press \"Apply / reconnect\" to switch without \
             a restart.",
        )
        .weak(),
    );
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
            ui.label(RichText::new("Testing…").weak());
            return;
        }
        Some(TestOutcome::Done(r)) => r,
    };
    match result {
        Ok(s) => {
            ui.label(
                RichText::new(format!("Connected: {s}")).color(Color32::from_rgb(90, 200, 110)),
            );
            ui.label(
                RichText::new(
                    "That was only a check — press Apply / reconnect below to start \
                     using this radio.",
                )
                .weak(),
            );
        }
        Err(e) => {
            ui.label(RichText::new(format!("Failed: {e}")).color(Color32::from_rgb(230, 90, 80)));
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
        ui.label("Waiting for the configuration of the machine the radio is attached to.");
        return;
    };

    egui::Grid::new("pluto-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Radios").on_hover_text(
            "Asks the network for IIO devices, and also tries 192.168.2.1 directly — \
             a Pluto on the end of a USB cable is often unreachable by multicast even \
             though the address works.",
        );
        // The mDNS query and the USB-gadget probe both go out from here, and a
        // Pluto on a USB cable is only reachable from the machine it is plugged
        // into. The Address row below is typed, so it still works from here.
        probe_only(ui, can_probe, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Discover").clicked() {
                    *discover = true;
                }
                let shown = cfg.pluto.selected_ip.clone().unwrap_or_else(|| "— none —".into());
                ComboBox::from_id_salt("pluto_dev").width(340.0).selected_text(shown).show_styled(
                    ui,
                    |ui| {
                        if devices.is_empty() {
                            ui.label(RichText::new("no radios — press Discover").weak());
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

        ui.label("Address").on_hover_text(
            "Overrides the selection above. The USB cable presents the Pluto as a \
             network adapter, not a serial port, so this is an IP address even when \
             the radio is on your desk.",
        );
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
        ui.label("Receiver");
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
            .on_hover_text(
                "A Pluto+ or a revision-C Pluto unlocked to 2R2T can serve two radio \
                 tabs from one box — this radio on RX1 and another on RX2, each on its \
                 own antenna. The two chains share the one oscillator, so retuning \
                 either radio moves both; what RX2 buys is a second antenna on the \
                 same spectrum (diversity), not a second band. The transmitter belongs \
                 to the RX1 radio. A stock 1R1T Pluto refuses RX2 when it connects.",
            );
        ui.end_row();

        ui.label("Sample rate").on_hover_text(
            "Width of the spectrum sdroxide receives. The AD9361 reaches 61.44 Msps; \
             the USB network link does not, which is what this list is scaled to. \
             The lowest rates need a filter configuration loaded into the AD9361, \
             which sdroxide does not do — a stock Pluto runs them at about 2.084 \
             Msps instead and says so when it connects. Takes effect on Apply.",
        );
        let shown = format!("{:.3} Msps", cfg.pluto.sample_rate_hz / 1e6);
        ComboBox::from_id_salt("pluto_rate").selected_text(shown).show_styled(ui, |ui| {
            for &r in &PlutoConfig::SAMPLE_RATES {
                let sel = (cfg.pluto.sample_rate_hz - r).abs() < 1.0;
                let mut label = format!("{:.3} Msps", r / 1e6);
                if r >= 3_840_000.0 {
                    label.push_str("  (more than USB 2 will carry)");
                } else if r < PlutoConfig::NO_FIR_FLOOR_HZ {
                    label.push_str("  (a stock Pluto runs at 2.084)");
                }
                if ui.selectable_label(sel, label).clicked() {
                    cfg.pluto.sample_rate_hz = r;
                }
            }
        });
        ui.end_row();

        ui.label("Analog filter").on_hover_text(
            "The AD9361's baseband filter. Leave at 0 for automatic, which opens it \
             to nine tenths of the sample rate — wide on purpose, because the \
             receiver parks its oscillator a quarter of a span off the dial to keep \
             signals clear of the DC spike, and a narrow filter would cut off exactly \
             the part it moved them to. Takes effect on Apply.",
        );
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

        ui.label("AGC").on_hover_text(
            "The AD9361 has four modes, not an on/off switch. Slow attack suits SSB \
             and CW; fast attack suits bursty signals; manual is the setting for \
             measurement and weak-signal digital modes. Applies immediately.",
        );
        let mut agc = cfg.pluto.agc;
        enum_combo(ui, "pluto_agc", &mut agc, &PlutoAgc::ALL, PlutoAgc::label);
        if agc != cfg.pluto.agc {
            cfg.pluto.agc = agc;
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: PlutoConfig::AGC_ELEMENT.to_string(),
                db: agc.code(),
            });
        }
        ui.end_row();

        ui.label("RX gain").on_hover_text(
            "Applies immediately — no reconnect. Ignored unless the AGC is set to \
             manual, which is the AD9361's own behaviour, not sdroxide's.",
        );
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

        ui.label("TX gain").on_hover_text(
            "Negative because the AD9361 expresses transmit level as attenuation: \
             0 dB is full output. Applies immediately. The transmitter is set to its \
             quietest before this value is applied on connect, so nothing the \
             previous program left behind can be live.",
        );
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

        ui.label("Frequency correction").on_hover_text(
            "Reference error in parts per million. Run with \
             RUST_LOG=sdroxide_pluto=debug and the log prints the measured clock \
             error after about 20 seconds — that is the number to enter. Applied by \
             sdroxide, not written to the radio's own persistent trim. Applies \
             immediately.",
        );
        let mut ppm = cfg.pluto.ppm;
        if ui
            .add(DragValue::new(&mut ppm).range(-200.0..=200.0).speed(0.1).suffix(" ppm"))
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
        ui.label("Full duplex");
        crate::chrome::checkbox(
            ui,
            &mut cfg.pluto.full_duplex,
            "Keep receiving while transmitting",
        )
        .on_hover_text(
            "Leave this off on a Pluto reached over its USB cable. That link cannot \
                 carry a megasample per second in both directions at once, and an over \
                 that starves the transmit buffer goes out chopped — so by default \
                 receive stops for the length of an over.\n\nTurn it on for a board on \
                 real Ethernet (a LibreSDR, a Pluto on a gigabit adapter), where there \
                 is room for both: you then hear the receiver through your own \
                 transmission, which is how a QO-100 station listens to its own \
                 downlink.\n\nAn over needs twice the sample rate's worth of link — \
                 2.5 Msps is 10 MB/s each way — so lower the rate if the log starts \
                 saying the link is not carrying it. The panadapter still shows the \
                 transmitted signal during an over; it is the audio that keeps \
                 coming.\n\nTakes effect on Apply.",
        );
        ui.end_row();

        // Below Full duplex because the two argue: TDD is one direction at a
        // time in the silicon, so it settles the question the checkbox above
        // asks about the link.
        ui.label("Duplex").on_hover_text(
            "Whether the AD9361 runs both directions at once (FDD, which is how a Pluto \
             boots and what sdroxide has always left it in) or one at a time (TDD).\n\n\
             Leave this on FDD unless you want the PTT pins below — TDD is what those \
             key from, and it rules out Full duplex above. Takes effect on Apply.",
        );
        let mut duplex = cfg.pluto.duplex;
        enum_combo(ui, "pluto_duplex", &mut duplex, &PlutoDuplex::ALL, PlutoDuplex::label);
        cfg.pluto.duplex = duplex;
        ui.end_row();

        ui.label("PTT pins").on_hover_text(
            "The Pluto's four GPO test points can key an external power amplifier, LNA \
             or transmit-receive switch by themselves. Pick a pair and one pin is high \
             the whole time the radio receives, the other the whole time it transmits — \
             no host software in the loop and no serial PTT line to wire.\n\nThis puts \
             the radio in TDD whatever the Duplex row says, because the pins follow the \
             AD9361's enable lines and in FDD both of those are asserted the entire \
             session. Full duplex goes off with it, so leave both at their defaults if \
             you work satellites and need to hear your own downlink.\n\nAnalog Devices' \
             own note puts an external LNA on GPO0/GPO1, so use GPO2/GPO3 if your board \
             is wired that way. The pins are about 1.3 V at a few milliamps: drive a \
             transistor or an opto-isolator with them, never a relay coil.\n\nTakes \
             effect on Apply.",
        );
        let mut ptt = cfg.pluto.ptt_gpo;
        enum_combo(ui, "pluto_ptt", &mut ptt, &PlutoPtt::ALL, PlutoPtt::label);
        cfg.pluto.ptt_gpo = ptt;
        ui.end_row();

        // Next to Full duplex because it belongs to the same subject: both are
        // about what the link between here and the board can carry, not about
        // what the AD9361 can do. This one used to be reachable only by hand-
        // editing radio.json, which is no use to the operator who needs it —
        // the one whose log is reporting a replaced receive socket.
        ui.label("Buffer size").on_hover_text(
            "How much the device holds before each transfer, in complex samples. \
             The default of 32768 is about 16 ms at 2 Msps: long enough that the \
             per-transfer round trip is not the bottleneck, short enough that a \
             retune is not visibly late.\n\nHalve it if the log reports the receive \
             socket being replaced. A marginal link — a Pluto reached over its USB \
             cable at a high sample rate is the usual one — stalls part-way through a \
             transfer, and a smaller transfer is both less likely to be caught mid-\
             flight and quicker to make good afterwards. Raise it to trade retune \
             latency for fewer round trips.\n\nTakes effect on Apply.",
        );
        ui.horizontal(|ui| {
            let mut samples = cfg.pluto.buffer_samples as i64;
            if ui
                .add(
                    DragValue::new(&mut samples)
                        .range(1024..=1_048_576)
                        .speed(256.0)
                        .suffix(" samples"),
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
                RichText::new(format!(
                    "≈ {:.1} ms, {} KiB per transfer",
                    cfg.pluto.buffer_samples as f64 / rate * 1e3,
                    cfg.pluto.buffer_samples * 4 / 1024,
                ))
                .weak(),
            );
        });
        ui.end_row();

        ui.label("RX / TX port").on_hover_text(
            "The AD9361's rf_port_select. A stock Pluto wires one of each, so leave \
             these empty unless you have a board that does not. Takes effect on Apply.",
        );
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
                    .button("Test connection")
                    .on_hover_text(
                        "Opens the radio, reads what it says about itself, and reports the \
                         tuning range this particular board has. Does not start a stream.",
                    )
                    .clicked()
                {
                    *test = true;
                }
                if ui
                    .button("Copy diagnostic report")
                    .on_hover_text(
                        "Copies the last session's protocol trace to the clipboard, for a \
                         bug report.",
                    )
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
    ui.label(
        RichText::new(
            "Wideband IQ receive and transmit. Half duplex — receive stops for the \
             length of an over, because the USB network link will not carry both at \
             once. No SoapySDR and no libiio needed.",
        )
        .weak(),
    );
    ui.label(
        RichText::new(
            "Not yet verified against real hardware. If it misbehaves, please send the \
             diagnostic report — it contains every command exchanged with the radio, \
             and the first bytes of the sample stream.",
        )
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
        ui.label("Waiting for the configuration of the machine the radio is attached to.");
        return;
    };

    egui::Grid::new("smartsdr-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Radios").on_hover_text(
            "A FlexRadio announces itself on the local network about once a second. \
             A radio reached through a router or a VPN never broadcasts to you — \
             enter its address below instead.",
        );
        // The broadcasts a FLEX sends reach its own network segment, which is
        // the engine's, not this screen's. The Address row below is typed, so
        // it still works from here.
        probe_only(ui, can_probe, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Discover").clicked() {
                    *discover = true;
                }
                let shown = cfg.smartsdr.selected_ip.clone().unwrap_or_else(|| "— none —".into());
                ComboBox::from_id_salt("flex_dev").width(340.0).selected_text(shown).show_styled(
                    ui,
                    |ui| {
                        if devices.is_empty() {
                            ui.label(RichText::new("no radios — press Discover").weak());
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
                                ui.label(RichText::new(d.label()).weak()).on_hover_text(
                                    "Another GUI client has this radio and multiFLEX is \
                                     disabled. Disconnect that client, or enable multiFLEX \
                                     on the radio.",
                                );
                            }
                        }
                    },
                );
            });
        });
        ui.end_row();

        ui.label("Address").on_hover_text(
            "Overrides the selection above. Use this for a radio on another subnet, \
             behind a VPN, or on a non-standard port.",
        );
        crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut cfg.smartsdr.address)
                .desired_width(220.0)
                .hint_text("optional, e.g. 192.168.1.50"),
        );
        ui.end_row();

        ui.label("IQ sample rate").on_hover_text(
            "Width of the spectrum sdroxide receives. 192 kHz is the radio's maximum \
             for a DAX IQ stream, and so the widest span this interface can show.",
        );
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
        ui.label(
            RichText::new(
                "192 kHz is the widest DAX IQ stream a FLEX will send, and the radio ties \
                 the panadapter's span to it — so that is the whole span, and no setting \
                 here widens it.",
            )
            .weak(),
        );
        ui.end_row();

        ui.label("DAX IQ channel").on_hover_text(
            "The radio has four. Change this only if something else on the network \
             is already using channel 1 — the radio refuses a channel twice over.",
        );
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

        ui.label("Station name").on_hover_text(
            "Shown against this session in the radio's client list. The radio also \
             remembers a client by it, so renaming makes the radio treat sdroxide as \
             a new one.",
        );
        crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut cfg.smartsdr.station).desired_width(160.0),
        );
        ui.end_row();

        ui.label("GUI client ID").on_hover_text(
            "The identity the radio restores this session's slices and panadapters to. \
             Left empty it is derived from the station name — stable, but the same on \
             every sdroxide that kept the default name, and a radio settles a duplicate \
             by throwing the earlier client off. sdroxide spots that on the wire and \
             falls back to a one-session identity, which costs the restore; put a UUID \
             of your own here to keep it.",
        );
        crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut cfg.smartsdr.gui_client_id)
                .desired_width(280.0)
                .hint_text("optional, e.g. a UUID of your own"),
        );
        ui.end_row();

        ui.label("Network MTU").on_hover_text(
            "Largest datagram the radio may send. 1450 is what SmartSDR itself asks for. \
             Lower it on a path with a smaller MTU — a VPN or a tunnel — where the \
             fragments are dropped and no spectrum arrives at all.",
        );
        let mut mtu = cfg.smartsdr.network_mtu as f64;
        if crate::chrome::field(
            ui,
            egui::DragValue::new(&mut mtu).speed(10.0).range(576.0..=9000.0).suffix(" B"),
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
                    .button("Test connection")
                    .on_hover_text(
                        "Checks the radio answers, without registering as a GUI client — \
                         so it will not disturb a SmartSDR session already running.",
                    )
                    .clicked()
                {
                    *test = true;
                }
                if ui
                    .button("Copy diagnostic report")
                    .on_hover_text(
                        "Copies the last session's protocol trace to the clipboard, for a \
                         bug report.",
                    )
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
    ui.label(
        RichText::new(
            "Wideband IQ receive over DAX, audio transmit the radio modulates. Press \
             \"Apply / reconnect\" to switch without a restart.",
        )
        .weak(),
    );
    ui.label(
        RichText::new(
            "Not yet verified against real hardware. If it misbehaves, please send the \
             diagnostic report — it contains every protocol line exchanged with the radio.",
        )
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
        ui.label(RichText::new("Devices SoapySDR can see").strong());
        ui.label(
            RichText::new(
                "Waiting for the machine the radio is attached to, where the modules are \
                 installed. Which device it opens is `device_args` in that machine's \
                 config.toml, or its --device.",
            )
            .weak(),
        );
        return;
    }

    ui.horizontal(|ui| {
        ui.label(RichText::new("Devices SoapySDR can see").strong());
        probe_only(ui, can_probe, |ui| {
            if ui
                .button("Rescan")
                .on_hover_text(
                    "Ask every installed SoapySDR module to scan. Nothing is opened, \
                     so this is safe while receiving — but it can take a moment.",
                )
                .clicked()
            {
                *rescan = true;
            }
        });
    });

    let Some(devices) = devices else {
        ui.label(RichText::new("Not enumerated yet — press Rescan.").weak());
        return;
    };
    if devices.is_empty() {
        ui.label(
            RichText::new(
                "No SoapySDR devices found. Check that the module for your radio is \
                 installed and that you may access the device.",
            )
            .weak(),
        );
        return;
    }

    for d in devices {
        ui.horizontal(|ui| {
            ui.label(RichText::new(d.label()).monospace());
            if d.is_pseudo() {
                ui.label(RichText::new("not a radio").color(Color32::from_rgb(220, 170, 70)));
            }
        });
    }

    // The sound-card trap: named, with the reason and the way out.
    if devices.iter().any(SoapyDeviceInfo::is_pseudo) {
        ui.add_space(4.0);
        ui.label(
            RichText::new(
                "A sound card is listed above as if it were an SDR. It accepts any \
                 frequency and ignores it, so opening it shows the sound card's input \
                 instead of the band. sdroxide does not pick those automatically — but \
                 a device_args line naming one is still obeyed.",
            )
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
        let names = native.iter().map(|b| b.label()).collect::<Vec<_>>().join(", ");
        ui.label(
            RichText::new(format!(
                "Hardware above is supported directly by sdroxide: {names}. Selecting that \
                 interface above gives you its own settings — gain stages, filters and \
                 notches SoapySDR cannot express — and needs no SoapySDR module.",
            ))
            .color(crate::theme::CYAN()),
        );
    }

    ui.add_space(4.0);
    ui.label(
        RichText::new(
            "Which one opens is set by --device or device_args in config.toml; with \
             neither, the first radio listed wins.",
        )
        .weak(),
    );
}

impl SdroxideApp {
    /// SoapySDR RX/TX gains + antenna (empty for a CAT rig).
    pub(in crate::app) fn settings_device_tab(&self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let Some(caps) = &self.caps else {
            ui.label("no device");
            return;
        };
        ui.label(RichText::new(&caps.label).size(14.0).strong().color(crate::theme::CYAN()));
        ui.add_space(6.0);
        if caps.gains.iter().all(|g| g.direction != Direction::Rx) {
            ui.label(RichText::new("This rig has no software-adjustable gains.").weak());
        }
        ui.label(RichText::new("RX gains").strong());
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
            ui.label(RichText::new("TX gains").strong().color(Color32::from_rgb(240, 90, 60)));
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
            ui.label(RichText::new("Antennas").strong());
            egui::Grid::new("antennas").num_columns(2).show(ui, |ui| {
                if rx_ports {
                    ui.label("RX");
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
                    ui.label(RichText::new("TX").color(Color32::from_rgb(240, 90, 60)));
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
            ui.label(RichText::new("Remembered for the next start.").weak());
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
        ui.label("Waiting for the configuration of the machine the radio is attached to.");
        return;
    };
    let Some(caps) = caps else {
        ui.label(
            RichText::new(
                "These controls appear once a SoapySDR device is open — they are the \
                 device's own, and it has to be asked.",
            )
            .weak(),
        );
        return;
    };

    if !caps.sample_rates.is_empty() || !caps.rate_ranges.is_empty() {
        ui.label(RichText::new("Stream").strong());
        egui::Grid::new("soapy-stream").num_columns(2).show(ui, |ui| {
            ui.label("Sample rate");
            let shown = if cfg.soapy.sample_rate_hz > 0.0 {
                rate_label(cfg.soapy.sample_rate_hz)
            } else {
                "Follow the app setting".to_string()
            };
            ComboBox::from_id_salt("soapy-rate").selected_text(shown).show_styled(ui, |ui| {
                if ui
                    .selectable_label(cfg.soapy.sample_rate_hz <= 0.0, "Follow the app setting")
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
                ui.label("or, in Msps");
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
                ui.label("Baseband filter");
                let shown = if cfg.soapy.bandwidth_hz > 0.0 {
                    bw_label(cfg.soapy.bandwidth_hz)
                } else {
                    "Let the driver choose".to_string()
                };
                ComboBox::from_id_salt("soapy-bw").selected_text(shown).show_styled(ui, |ui| {
                    if ui
                        .selectable_label(cfg.soapy.bandwidth_hz <= 0.0, "Let the driver choose")
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
        ui.label(RichText::new("Changing either reopens the radio.").weak());
    }

    if caps.settings.is_empty() {
        ui.add_space(4.0);
        ui.label(RichText::new("This driver publishes no settings of its own.").weak());
        return;
    }

    ui.separator();
    ui.label(RichText::new(format!("{} settings", caps.driver)).strong());
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
    ui.label(RichText::new("Applied at once, and remembered for the next start.").weak());
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
        ui.label("Waiting for the configuration of the machine the radio is attached to.");
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
        ui.label("Receiver");
        // Which receiver is this panel's one row about a USB bus; everything
        // below reaches the device wherever it is plugged in.
        probe_only(ui, can_probe, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button("Rescan")
                    .on_hover_text(
                        "Re-list the USB bus. No device is opened, so this is safe \
                         to press while receiving.",
                    )
                    .clicked()
                {
                    *rescan = true;
                }
                let shown = if cfg.rx888.serial.is_empty() {
                    "— first one found —".to_string()
                } else {
                    cfg.rx888.serial.clone()
                };
                ComboBox::from_id_salt("rx888_dev").width(300.0).selected_text(shown).show_styled(
                    ui,
                    |ui| {
                        if devices.is_empty() {
                            ui.label("No RX-888 found — press Rescan");
                        }
                        ui.selectable_value(
                            &mut cfg.rx888.serial,
                            String::new(),
                            "— first one found —",
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

        ui.label("ADC clock");
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
                    egui::RichText::new(format!(
                        "0–{:.1} MHz coverage, {:.0} MB/s over USB",
                        rate / 2e6,
                        rate * 2.0 / 1e6
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
        ui.label("or, in Msps");
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
                egui::RichText::new(
                    "129.6 Msps needs a SuperSpeed link and a fast host; 64.8 is the \
                     safe default. Changing it reopens the receiver automatically, \
                     which takes a moment but needs no restart.",
                )
                .weak(),
            )
            .wrap(),
        );
        ui.end_row();

        ui.label("Panadapter width");
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
                    if Rx888Config::width_works_on_vhf(rate, bins) { "" } else { "  · HF only" },
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
                                "Usable on both front ends."
                            } else {
                                "Below the VHF crossover this is an ordinary width. Above it \
                                 the tuner's 8 MHz IF cannot be centred in a window this \
                                 wide, so the live spectrum sits off to one side of the \
                                 panadapter with nothing beside it."
                            });
                    }
                });
            ui.add(
                egui::Label::new(
                    egui::RichText::new(format!("of the {:.1} MHz digitised", rate / 2e6)).weak(),
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
                    egui::RichText::new(format!(
                        "How much of the digitised spectrum the panadapter shows at once. \
                         The whole DSP chain runs at this width, so wider costs \
                         proportionally more CPU — 1/2 is the entire band in the \
                         waterfall, and a serious amount of arithmetic. Above the VHF \
                         crossover ({:.1} MHz) the panadapter is not looking at the \
                         antenna but at the tuner's IF, which is 8 MHz wide and parked \
                         at {:.2} MHz — so anything wider than {} cannot be centred on \
                         it, and the extra width is dead spectrum beside the signal \
                         rather than more of it.",
                        Rx888Config::vhf_crossover_hz(rate) / 1e6,
                        Rx888Config::VHF_IF_CENTER_HZ / 1e6,
                        bw_label(2.0 * Rx888Config::VHF_IF_CENTER_HZ),
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
                        egui::RichText::new(
                            "The dial is above the crossover now, so this width is showing \
                             the tuner's 8 MHz off-centre with dead spectrum beside it. \
                             Narrow the width to fill the panadapter.",
                        )
                        .color(crate::theme::YELLOW()),
                    )
                    .wrap(),
                );
            }
        });
        ui.end_row();

        ui.label("VGA gain");
        if crate::chrome::slider(
            ui,
            egui::Slider::new(&mut cfg.rx888.vga_db, -6.0..=34.0).suffix(" dB"),
        )
        .on_hover_text("AD8370 variable-gain amplifier ahead of the ADC.")
        .changed()
        {
            cmds.push(Command::SetGain {
                dir: sdroxide_types::Direction::Rx,
                element: Rx888Config::VGA_ELEMENT.into(),
                db: cfg.rx888.vga_db,
            });
        }
        ui.end_row();

        ui.label("Attenuator");
        if crate::chrome::slider(
            ui,
            egui::Slider::new(&mut cfg.rx888.attenuator_db, -31.5..=0.0).suffix(" dB"),
        )
        .on_hover_text("PE4304 step attenuator, in 0.5 dB steps.")
        .changed()
        {
            cmds.push(Command::SetGain {
                dir: sdroxide_types::Direction::Rx,
                element: Rx888Config::ATT_ELEMENT.into(),
                db: cfg.rx888.attenuator_db,
            });
        }
        ui.end_row();

        ui.label("ADC range");
        if ui
            .checkbox(&mut cfg.rx888.pga, "Wide (2.25 Vp-p)")
            .on_hover_text(
                "Selects the ADC's wider input range: more headroom for strong \
                 broadcast signals, fewer counts for weak ones. Off selects the \
                 more sensitive 1.5 Vp-p range.",
            )
            .changed()
        {
            cmds.push(Command::SetGain {
                dir: sdroxide_types::Direction::Rx,
                element: Rx888Config::PGA_ELEMENT.into(),
                db: cfg.rx888.pga as u8 as f64,
            });
        }
        ui.end_row();

        ui.label("Dither");
        if ui
            .checkbox(&mut cfg.rx888.dither, "Enable")
            .on_hover_text(
                "Adds a small dither signal ahead of the ADC: costs a little \
                 noise floor, buys spurious-free dynamic range.",
            )
            .changed()
        {
            cmds.push(Command::SetGain {
                dir: sdroxide_types::Direction::Rx,
                element: Rx888Config::DITHER_ELEMENT.into(),
                db: cfg.rx888.dither as u8 as f64,
            });
        }
        ui.end_row();

        ui.label("Randomiser");
        crate::chrome::checkbox(ui, &mut cfg.rx888.randomize, "Enable").on_hover_text(
            "The ADC scrambles its output so the digital bus stops radiating \
                 into the front end; the driver unscrambles it. Leave this on \
                 unless you are debugging. Applies on reconnect.",
        );
        ui.end_row();

        ui.label("Bias tee");
        if ui
            .checkbox(&mut cfg.rx888.bias_tee_hf, "DC on the HF antenna port")
            .on_hover_text("Powers an active antenna or preamp down the coax.")
            .changed()
        {
            cmds.push(Command::SetGain {
                dir: sdroxide_types::Direction::Rx,
                element: Rx888Config::BIAS_TEE_ELEMENT.into(),
                db: cfg.rx888.bias_tee_hf as u8 as f64,
            });
        }
        ui.end_row();

        ui.label("VHF tuner gain");
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
            if slider
                .on_hover_text(
                    "R828D RF gain, used above the automatic HF/VHF crossover. \
                     29 discrete steps; the nearest is used.",
                )
                .changed()
            {
                cmds.push(Command::SetGain {
                    dir: sdroxide_types::Direction::Rx,
                    element: Rx888Config::TUNER_GAIN_ELEMENT.into(),
                    db: cfg.rx888.tuner_gain_db,
                });
            }
            if ui
                .checkbox(&mut cfg.rx888.tuner_agc, "Auto")
                .on_hover_text("Let the tuner run its own LNA and mixer loops.")
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

        ui.label("Bias tee (VHF)");
        if ui
            .checkbox(&mut cfg.rx888.bias_tee_vhf, "DC on the VHF antenna port")
            .on_hover_text("Powers an active antenna or preamp down the coax.")
            .changed()
        {
            cmds.push(Command::SetGain {
                dir: sdroxide_types::Direction::Rx,
                element: Rx888Config::BIAS_TEE_VHF_ELEMENT.into(),
                db: cfg.rx888.bias_tee_vhf as u8 as f64,
            });
        }
        ui.end_row();

        ui.label("Clock trim");
        let r = ui
            .add(
                egui::DragValue::new(&mut cfg.rx888.ppm)
                    .speed(0.1)
                    .range(-200.0..=200.0)
                    .suffix(" ppm"),
            )
            .on_hover_text(
                "Corrects the reference oscillator. Applied when you let go of \
                 the value — reopening on every pixel of a drag would restart \
                 the receiver hundreds of times.",
            );
        if r.drag_stopped() || r.lost_focus() {
            *apply = true;
        }
        ui.end_row();
    });

    if before != (cfg.rx888.serial.clone(), cfg.rx888.randomize, cfg.rx888.ddc_bins) {
        *apply = true;
    }

    ui.add_space(6.0);
    ui.label(
        egui::RichText::new(
            "Receive only. Below the ADC's Nyquist limit the antenna is sampled \
             directly and retuning is instant, because there is no hardware \
             downconverter — the full ADC stream is converted to baseband on the \
             host. Above it the receiver switches to its R828D tuner and the VHF \
             SMA automatically, so both antennas need to be connected. VHF needs \
             an ADC clock of 32.4 Msps or more for the tuner's IF to fit, and at \
             clocks below 48 Msps there is a gap between the two ranges that \
             nothing can reach. Every setting here applies straight away — there \
             is no Apply button to press.",
        )
        .weak(),
    );
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
        ui.label("Waiting for the configuration of the machine the radio is attached to.");
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
        ui.label("Device");
        probe_only(ui, can_probe, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button("Rescan")
                    .on_hover_text(
                        "Re-list the USB bus. No device is opened, so this is safe \
                         to press while receiving.",
                    )
                    .clicked()
                {
                    *rescan = true;
                }
                let shown = if cfg.elad.serial.is_empty() {
                    "— first one found —".to_string()
                } else {
                    cfg.elad.serial.clone()
                };
                ComboBox::from_id_salt("elad_dev").width(300.0).selected_text(shown).show_styled(
                    ui,
                    |ui| {
                        if devices.is_empty() {
                            ui.label(RichText::new("no devices — press Rescan").weak());
                        }
                        ui.selectable_value(
                            &mut cfg.elad.serial,
                            String::new(),
                            "— first one found —",
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

        ui.label("Sample rate").on_hover_text(
            "The six rates are six different FPGA images, not a register — which \
             is why nothing in ELAD's protocol selects between them, and why what \
             this setting means depends on the model.\n\n\
             FDM-S1 / FDM-S2: a command. Their FPGA is loaded by the computer at \
             every power-up, and sdroxide runs ELAD's own elad-firmware loader to \
             put this rate's image in. Without that loader installed a sampler \
             sends no samples at all, and sdroxide says so on screen.\n\n\
             FDM-DUO: not a command. The radio boots its own FPGA and arrives at \
             whatever rate it powered up in or FDM-SW2 last left it in (192 kHz on \
             a fresh one), and this only says how the stream is READ. Get it wrong \
             and the panadapter is simply the wrong width, with every frequency \
             inside it scaled to match.\n\n\
             The stream's real rate is measured a couple of seconds after it \
             starts, and a mismatch is reported on screen. Takes effect on Apply.",
        );
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
                            format!("{:.0} kHz  (16-bit samples)", r as f64 / 1e3)
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
                    RichText::new("FDM-S: the FPGA image loaded — FDM-DUO: how the stream is read")
                        .weak(),
                )
                .wrap(),
            );
        });
        ui.end_row();

        ui.label("Attenuator").on_hover_text(
            "The receiver's input pad — one step, in or out. The same control as \
             the main window's Gain slider.",
        );
        let mut att = cfg.elad.attenuator;
        if crate::chrome::checkbox(ui, &mut att, format!("{ELAD_ATTENUATOR_DB:.0} dB pad in"))
            .changed()
        {
            cfg.elad.attenuator = att;
            push_gain(cmds, EladConfig::ATT_ELEMENT, if att { -ELAD_ATTENUATOR_DB } else { 0.0 });
        }
        ui.end_row();

        ui.label("Pre-selection filters").on_hover_text(
            "The low-pass bank in front of the ADC. Bypassing it gives the widest \
             view and the worst behaviour near strong out-of-band signals — a \
             broadcast transmitter a few miles away will put images across the \
             band. Leave it on unless you are deliberately listening outside the \
             filtered range.",
        );
        let mut lpf = cfg.elad.preselector;
        if crate::chrome::checkbox(ui, &mut lpf, "Filters in circuit").changed() {
            cfg.elad.preselector = lpf;
            push_gain(cmds, EladConfig::LPF_ELEMENT, f64::from(u8::from(lpf)));
        }
        ui.end_row();
    });

    ui.add_space(8.0);
    ui.label(RichText::new("Rig control — FDM-DUO only").strong());
    ui.label(
        RichText::new(
            "The transceiver's CAT USB port, which is a separate cable from the \
             receive one. Leave the port unset on an FDM-S1 or FDM-S2, which have \
             none — and on an FDM-DUO you would rather drive through its receive \
             cable alone, where it can still be tuned and keyed but no meter can \
             be read.",
        )
        .weak(),
    );
    ui.label(
        RichText::new(
            "With the port set, the radio's display and this dial are the same \
             number in both directions: the FDM-DUO's receive window is built \
             around its VFO, so tuning here moves the front panel and turning the \
             front-panel knob moves the readout here. The panadapter re-centres on \
             the dial as it goes. With no port — or none the radio answers on — \
             there is no VFO to command, and tuning stays inside the window the \
             radio is already sending.",
        )
        .weak(),
    );

    egui::Grid::new("elad-cat-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Serial port");
        let shown = if cfg.cat.serial.path.is_empty() {
            "— none (USB control only) —".to_string()
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
                        "— none (USB control only) —",
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

        ui.label("Baud").on_hover_text(
            "Must match menu 70 \"CAT BAUD\" on the radio, which ships at 38400. The \
             radio has these four rates and no others, and a port opened at any \
             other one is silent both ways — the radio ignores the dial and refuses \
             to key, with nothing to say why.",
        );
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
                        RichText::new(format!(
                            "the radio has no {} baud setting — {ELAD_DEFAULT_CAT_BAUD} will be \
                             used instead",
                            cfg.cat.serial.baud,
                        ))
                        .color(crate::theme::YELLOW()),
                    )
                    .wrap(),
                );
            }
        });
        ui.end_row();

        ui.label("PTT method").on_hover_text(
            "How transmit is keyed. \"CAT\" sends the radio's own TX command and \
             needs nothing set up on the rig; RTS needs menu 54 \"PTT\" set to \
             PTT+RTS.",
        );
        enum_combo(ui, "elad_ptt", &mut cfg.cat.ptt, &PttMethod::ALL, PttMethod::label);
        ui.end_row();

        ui.label("Transmit input").on_hover_text(
            "Where the radio takes its transmit audio from — the rig's TI command, \
             which is menu 32 \"TX IN\" at the front panel. Asserted when the port \
             opens.\n\n\
             \"USB audio\" is what makes transmit work here: the FDM-DUO sends \
             what sdroxide puts into its USB sound card. A radio left on \
             \"Microphone\" sends the room instead, and nothing on screen says so.",
        );
        enum_combo(
            ui,
            "elad_txin",
            &mut cfg.cat.elad_tx_input,
            &EladTxInput::ALL,
            EladTxInput::label,
        );
        ui.end_row();

        ui.label("Antenna").on_hover_text(
            "Which of the two sockets on the back the receiver listens on — the \
             rig's AN command, which is menu 31 \"ANTENNAS\" at the front panel \
             and the \"ANT 1 2\" indicator on its display.\n\n\
             \"RTX\" is one antenna doing both jobs, on the M-type socket that \
             also carries transmit. \"RX only\" moves receive to the second \
             socket and leaves transmit on RTX — a receiving antenna, a loop or \
             a beverage, with the beam still on the transmitter.\n\n\
             It moves this whole receiver: the panadapter, the demodulators and \
             the rig's own audio all come from the socket selected here.\n\n\
             Applies immediately, and is read back from the radio when the \
             control port opens — this is the rig's own setting, not a copy of \
             it kept here.",
        );
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

        ui.label("Mode control");
        enum_combo(
            ui,
            "elad_modectl",
            &mut cfg.cat.mode_control,
            &ModeControl::ALL,
            ModeControl::label,
        );
        ui.end_row();

        ui.label("Poll rate").on_hover_text(
            "How often the radio is asked what it is doing — its dial, its mode \
             and its meters. Turn the rig's own VFO knob and the readout follows \
             within one poll. It is also the whole of the control traffic this \
             end generates, so lower it if the radio's audio breaks up.",
        );
        ui.add(DragValue::new(&mut cfg.cat.poll_hz).speed(0.5).range(0.5..=20.0).suffix(" Hz"));
        ui.end_row();

        ui.label("Transmit audio");
        match audio_outputs {
            Some(outs) => {
                let shown =
                    cfg.radio_audio_out.clone().unwrap_or_else(|| "— system default —".to_string());
                ComboBox::from_id_salt("elad_txaudio")
                    .width(300.0)
                    .selected_text(shown)
                    .show_styled(ui, |ui| {
                        ui.selectable_value(&mut cfg.radio_audio_out, None, "— system default —");
                        for o in outs {
                            ui.selectable_value(&mut cfg.radio_audio_out, Some(o.clone()), o);
                        }
                    })
                    .response
                    .on_hover_text(
                        "The FDM-DUO's own USB Audio port. Left on the system default \
                     it is almost never the radio — it is the machine's speakers.",
                    );
            }
            None => {
                ui.label(RichText::new("press Rescan to list the sound cards").weak());
            }
        }
        ui.end_row();

        ui.label("");
        probe_only(ui, can_probe, |ui| {
            if ui
                .button("Copy diagnostic report")
                .on_hover_text(
                    "Copies the last session's trace to the clipboard, for a bug \
                     report: every command exchanged with the device, and the \
                     first bytes of the sample stream.",
                )
                .clicked()
            {
                *copy_report = true;
            }
        });
        ui.end_row();
    });

    ui.add_space(4.0);
    ui.label(
        RichText::new(
            "ELAD support has not been verified against real hardware. If it \
             misbehaves, please attach the diagnostic report to a bug report.",
        )
        .color(crate::theme::YELLOW()),
    );
    ui.label(
        RichText::new(
            "CW is keyed by the radio's own key or paddle: the FDM-DUO has no \
             command that accepts text, so the CW panel cannot key it.",
        )
        .weak(),
    );

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
        ui.label(RichText::new("Press Apply to reopen the radio.").weak());
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
        ui.label("Waiting for the configuration of the machine the radio is attached to.");
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
        ui.label("Receiver");
        // Which receiver is this panel's one row about a USB bus; everything
        // below reaches the device wherever it is plugged in.
        probe_only(ui, can_probe, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button("Rescan")
                    .on_hover_text(
                        "Re-list the USB bus. No device is opened, so this is safe \
                         to press while receiving.",
                    )
                    .clicked()
                {
                    *rescan = true;
                }
                let shown = if cfg.airspyhf.serial.is_empty() {
                    "— first one found —".to_string()
                } else {
                    cfg.airspyhf.serial.clone()
                };
                ComboBox::from_id_salt("airspyhf_dev")
                    .width(300.0)
                    .selected_text(shown)
                    .show_styled(ui, |ui| {
                        if devices.is_empty() {
                            ui.label(RichText::new("no receivers — press Rescan").weak());
                        }
                        ui.selectable_value(
                            &mut cfg.airspyhf.serial,
                            String::new(),
                            "— first one found —",
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

        ui.label("Sample rate").on_hover_text(
            "Which rates a receiver has depends on the model and the firmware. \
             Takes effect on Apply.",
        );
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
                        RichText::new(
                            "every rate any HF+ model offers — connect one to see its own",
                        )
                        .weak(),
                    )
                    .wrap(),
                );
            }
        });
        ui.end_row();

        ui.label("AGC").on_hover_text(
            "The receiver's own gain control. Leave it on for general listening; \
             turn it off to set the attenuator by hand for measurement.",
        );
        let mut agc = cfg.airspyhf.agc;
        if crate::chrome::checkbox(ui, &mut agc, "Automatic").changed() {
            cfg.airspyhf.agc = agc;
            push_gain(cmds, AirspyHfConfig::AGC_ELEMENT, f64::from(u8::from(agc)));
        }
        ui.end_row();

        ui.label("AGC threshold");
        ui.add_enabled_ui(cfg.airspyhf.agc, |ui| {
            let mut high = cfg.airspyhf.agc_threshold_high;
            if ui
                .checkbox(&mut high, "High")
                .on_hover_text(
                    "High trades a little sensitivity for headroom against strong \
                     neighbours — the right setting on a crowded band at night.",
                )
                .changed()
            {
                cfg.airspyhf.agc_threshold_high = high;
                push_gain(cmds, AirspyHfConfig::AGC_THRESHOLD_ELEMENT, f64::from(u8::from(high)));
            }
        });
        ui.end_row();

        ui.label("Attenuator").on_hover_text(
            "Front-end attenuation, as a gain — 0 dB is none. Only obeyed with \
             the AGC off.",
        );
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

        ui.label("Preamp").on_hover_text(
            "The HF low-noise amplifier. Buys sensitivity at the cost of \
             intermodulation, so it is off by default — which is usually right \
             on a real antenna.",
        );
        let mut lna = cfg.airspyhf.lna;
        if crate::chrome::checkbox(ui, &mut lna, "LNA on").changed() {
            cfg.airspyhf.lna = lna;
            push_gain(cmds, AirspyHfConfig::LNA_ELEMENT, f64::from(u8::from(lna)));
        }
        ui.end_row();

        ui.label("Frequency calibration").on_hover_text(
            "Parts per billion — this receiver's own unit, a thousand times finer \
             than the ppm an RTL-SDR uses. Nothing here is ever written to the \
             receiver's flash: this overrides the stored value for the session only.",
        );
        ui.horizontal(|ui| {
            let mut stored = cfg.airspyhf.calibration_ppb.is_none();
            if crate::chrome::checkbox(ui, &mut stored, "Use the receiver's stored value").changed()
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
                && ui.add(DragValue::new(ppb).speed(10).suffix(" ppb")).changed()
            {
                push_gain(cmds, AirspyHfConfig::PPB_ELEMENT, *ppb as f64);
            }
        });
        ui.end_row();

        ui.label("Bias tee");
        let mut bias = cfg.airspyhf.bias_tee;
        if ui
            .checkbox(&mut bias, "Feed DC up the coax")
            .on_hover_text("Not every HF+ has one; on a receiver without, this does nothing.")
            .changed()
        {
            cfg.airspyhf.bias_tee = bias;
            push_gain(cmds, AirspyHfConfig::BIAS_TEE_ELEMENT, f64::from(u8::from(bias)));
        }
        ui.end_row();

        ui.label("Host DSP").on_hover_text(
            "The image balancer, the zero-IF offset and the fine-tuning \
             oscillator. Turn it off only to see raw hardware output — with it \
             off, the mirror image appears on the zero-IF rates and the dial is \
             accurate only to the nearest kilohertz.",
        );
        let mut dsp = cfg.airspyhf.lib_dsp;
        if crate::chrome::checkbox(ui, &mut dsp, "Correct the image and fine-tune").changed() {
            cfg.airspyhf.lib_dsp = dsp;
            push_gain(cmds, AirspyHfConfig::LIB_DSP_ELEMENT, f64::from(u8::from(dsp)));
        }
        ui.end_row();

        ui.label("");
        // The trace is of the session *this* process ran; the engine's own is
        // on the engine's machine.
        probe_only(ui, can_probe, |ui| {
            if ui
                .button("Copy diagnostic report")
                .on_hover_text(
                    "Copies the last session's trace to the clipboard, for a bug \
                     report: every command exchanged with the receiver, and the \
                     first bytes of the sample stream.",
                )
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
            RichText::new(
                "Bias tee is ON. Never connect a transceiver, a grounded antenna, \
                 or a preamp powered from elsewhere while this is enabled — the DC \
                 goes straight down the feedline.",
            )
            .color(crate::theme::YELLOW()),
        );
    }

    if (cfg.airspyhf.serial.clone(), cfg.airspyhf.sample_rate_hz) != before {
        *apply = true;
    }

    ui.add_space(4.0);
    ui.label(
        RichText::new(
            "Receive only. No SoapySDR and no libairspyhf needed. Below the \
             synthesiser's floor the host oscillator does the tuning, which is how \
             this receiver reaches VLF. The receiver and sample rate take effect on \
             Apply; everything else applies as you change it.",
        )
        .weak(),
    );
    ui.label(
        RichText::new(
            "Not yet verified against real hardware. If it misbehaves, please send the \
             diagnostic report — it contains every command exchanged with the receiver, \
             and the first bytes of the sample stream decoded as I/Q pairs.",
        )
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
        ui.label("Waiting for the configuration of the machine the radio is attached to.");
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
        ui.label("Receiver");
        probe_only(ui, can_probe, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button("Rescan")
                    .on_hover_text(
                        "Re-list the USB bus. No device is opened, so this is safe \
                         to press while receiving.",
                    )
                    .clicked()
                {
                    *rescan = true;
                }
                let shown = if cfg.airspy.serial.is_empty() {
                    "— first one found —".to_string()
                } else {
                    cfg.airspy.serial.clone()
                };
                ComboBox::from_id_salt("airspy_dev").width(300.0).selected_text(shown).show_styled(
                    ui,
                    |ui| {
                        if devices.is_empty() {
                            ui.label("No Airspy R2 or Mini found — press Rescan");
                        }
                        ui.selectable_value(
                            &mut cfg.airspy.serial,
                            String::new(),
                            "— first one found —",
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

        ui.label("Sample rate");
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
                        "read from this receiver".to_string()
                    } else {
                        "an R2 and a Mini offer different rates and cannot be told \
                         apart until one is open"
                            .to_string()
                    })
                    .weak(),
                )
                .wrap(),
            );
        });
        ui.end_row();
        ui.label("");
        ui.add(
            egui::Label::new(
                RichText::new(
                    "This is the rate you get. The receiver's ADC runs at twice it — \
                     it digitises a real signal and sdroxide makes complex baseband \
                     from it on the host.",
                )
                .weak(),
            )
            .wrap(),
        );
        ui.end_row();

        ui.label("Gain curve");
        ui.horizontal(|ui| {
            for c in AirspyGain::ALL {
                if ui.selectable_label(cfg.airspy.gain_curve == c, c.label()).clicked()
                    && cfg.airspy.gain_curve != c
                {
                    cfg.airspy.gain_curve = c;
                    push_gain(cmds, AirspyConfig::CURVE_ELEMENT, c.code() as f64);
                }
            }
        });
        ui.end_row();

        ui.label("Gain");
        if crate::chrome::slider(
            ui,
            egui::Slider::new(&mut cfg.airspy.gain_step, 0..=(AirspyConfig::GAIN_STEPS - 1))
                .text("step"),
        )
        .on_hover_text(
            "A step along the curve above, not a dB figure — the tuner's LNA, \
                 mixer and VGA move together, and how much each step is worth \
                 depends on the curve and the band. 0 is the quiet end.",
        )
        .changed()
        {
            push_gain(cmds, AirspyConfig::GAIN_ELEMENT, cfg.airspy.gain_step as f64);
        }
        ui.end_row();

        ui.label("Tuner AGC");
        ui.horizontal(|ui| {
            if ui
                .checkbox(&mut cfg.airspy.lna_agc, "LNA")
                .on_hover_text("Hands the LNA to the tuner's own loop, overriding the curve.")
                .changed()
            {
                push_gain(cmds, AirspyConfig::LNA_AGC_ELEMENT, cfg.airspy.lna_agc as u8 as f64);
            }
            if ui
                .checkbox(&mut cfg.airspy.mixer_agc, "Mixer")
                .on_hover_text("The same for the mixer stage.")
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
                    RichText::new(
                        "With a loop running, the gain slider no longer sets the stage \
                         it owns — the loop overwrites it.",
                    )
                    .weak(),
                )
                .wrap(),
            );
            ui.end_row();
        }

        ui.label("Bias tee");
        if ui
            .checkbox(&mut cfg.airspy.bias_tee, "DC on the antenna port")
            .on_hover_text("Powers an active antenna or preamp down the coax.")
            .changed()
        {
            push_gain(cmds, AirspyConfig::BIAS_TEE_ELEMENT, cfg.airspy.bias_tee as u8 as f64);
        }
        ui.end_row();

        ui.label("12-bit packing");
        ui.horizontal(|ui| {
            crate::chrome::checkbox(ui, &mut cfg.airspy.packing, "Enable");
            ui.add(
                egui::Label::new(
                    RichText::new(
                        "A third less USB traffic. Leave it on: this is a USB 2.0 \
                         device and the top rate is 30 MB/s packed against 40 \
                         unpacked. Applies on reconnect.",
                    )
                    .weak(),
                )
                .wrap(),
            );
        });
        ui.end_row();

        ui.label("DC removal");
        if ui
            .checkbox(&mut cfg.airspy.dc_block, "Remove the ADC's offset")
            .on_hover_text(
                "Turn it off to see raw hardware output. Worth knowing where the \
                 spur goes: the offset lands at the edge of the span, not its \
                 centre, because the signal is translated by a quarter of the \
                 sample rate on the way through.",
            )
            .changed()
        {
            push_gain(cmds, AirspyConfig::DC_BLOCK_ELEMENT, cfg.airspy.dc_block as u8 as f64);
        }
        ui.end_row();
    });

    if cfg.airspy.bias_tee {
        ui.add_space(4.0);
        ui.label(
            RichText::new(
                "Bias tee is ON. Never connect a transceiver, a grounded antenna, \
                 or a preamp powered from elsewhere while this is enabled — the DC \
                 goes straight down the feedline.",
            )
            .color(crate::theme::YELLOW()),
        );
    }

    if (cfg.airspy.serial.clone(), cfg.airspy.sample_rate_hz, cfg.airspy.packing) != before {
        *apply = true;
    }

    ui.add_space(6.0);
    probe_only(ui, can_probe, |ui| {
        if ui
            .button("Copy diagnostic report")
            .on_hover_text(
                "Every command exchanged with the receiver, the sample-rate \
                 arithmetic, and the first samples decoded as I/Q.",
            )
            .clicked()
        {
            *copy_report = true;
        }
    });

    ui.add_space(4.0);
    ui.label(
        RichText::new(
            "Receive only, 24–1800 MHz. No SoapySDR and no libairspy needed. The \
             receiver, sample rate and packing take effect on Apply; everything \
             else applies as you change it.",
        )
        .weak(),
    );
    ui.label(
        RichText::new(
            "Not yet verified against real hardware. If it misbehaves, please send the \
             diagnostic report — it contains every command exchanged with the receiver, \
             the rate arithmetic, and the first samples decoded as I/Q pairs.",
        )
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
        ui.label("Waiting for the configuration of the machine the radio is attached to.");
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
        ui.label("Receiver");
        probe_only(ui, can_probe, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button("Rescan")
                    .on_hover_text(
                        "Re-list the USB bus. No device is opened, so this is safe \
                         to press while receiving.",
                    )
                    .clicked()
                {
                    *rescan = true;
                }
                let shown = if cfg.hydrasdr.serial.is_empty() {
                    "— first one found —".to_string()
                } else {
                    cfg.hydrasdr.serial.clone()
                };
                ComboBox::from_id_salt("hydrasdr_dev")
                    .width(340.0)
                    .selected_text(shown)
                    .show_styled(ui, |ui| {
                        if devices.is_empty() {
                            ui.label("No HydraSDR RFOne found — press Rescan");
                        }
                        ui.selectable_value(
                            &mut cfg.hydrasdr.serial,
                            String::new(),
                            "— first one found —",
                        );
                        for d in devices {
                            let serial = d.serial.clone().unwrap_or_default();
                            ui.selectable_value(&mut cfg.hydrasdr.serial, serial, d.label());
                        }
                    });
            });
        });
        ui.end_row();

        ui.label("Sample rate");
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
                        "these are the rates this receiver turned out to have".to_string()
                    } else {
                        "the receiver only reports three of these; the rest are in the \
                         firmware's alternate table and an older build may not carry them"
                            .to_string()
                    })
                    .weak(),
                )
                .wrap(),
            );
        });
        ui.end_row();
        ui.label("");
        ui.add(
            egui::Label::new(
                RichText::new(
                    "This is the rate you get. The receiver's ADC runs at twice it — \
                     it digitises a real signal and sdroxide makes complex baseband \
                     from it on the host.",
                )
                .weak(),
            )
            .wrap(),
        );
        ui.end_row();

        ui.label("RF input");
        ui.horizontal(|ui| {
            for p in HydraSdrPort::ALL {
                let hover = if p.has_bias_tee() {
                    "The antenna SMA — the only socket with the bias tee behind it."
                } else {
                    "A cable socket. No bias tee here; the hardware puts it on ANT alone."
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

        ui.label("Gain curve");
        ui.horizontal(|ui| {
            for c in HydraSdrGain::ALL {
                if ui.selectable_label(cfg.hydrasdr.gain_curve == c, c.label()).clicked()
                    && cfg.hydrasdr.gain_curve != c
                {
                    cfg.hydrasdr.gain_curve = c;
                    push_gain(cmds, HydraSdrConfig::CURVE_ELEMENT, c.code() as f64);
                }
            }
        });
        ui.end_row();

        ui.label("Gain");
        if crate::chrome::slider(
            ui,
            egui::Slider::new(&mut cfg.hydrasdr.gain_step, 0..=(HydraSdrConfig::GAIN_STEPS - 1))
                .text("step"),
        )
        .on_hover_text(
            "A step along the curve above, not a dB figure — the tuner's LNA, \
             mixer and VGA move together, and how much each step is worth \
             depends on the curve and the band. 0 is the quiet end.",
        )
        .changed()
        {
            push_gain(cmds, HydraSdrConfig::GAIN_ELEMENT, cfg.hydrasdr.gain_step as f64);
        }
        ui.end_row();

        ui.label("Tuner AGC");
        ui.horizontal(|ui| {
            if ui
                .checkbox(&mut cfg.hydrasdr.lna_agc, "LNA")
                .on_hover_text("Hands the LNA to the tuner's own loop, overriding the curve.")
                .changed()
            {
                push_gain(cmds, HydraSdrConfig::LNA_AGC_ELEMENT, cfg.hydrasdr.lna_agc as u8 as f64);
            }
            if ui
                .checkbox(&mut cfg.hydrasdr.mixer_agc, "Mixer")
                .on_hover_text("The same for the mixer stage.")
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
                    RichText::new(
                        "With a loop running, the gain slider no longer sets the stage \
                         it owns — the loop overwrites it.",
                    )
                    .weak(),
                )
                .wrap(),
            );
            ui.end_row();
        }

        ui.label("Bias tee");
        ui.add_enabled_ui(cfg.hydrasdr.rf_port.has_bias_tee(), |ui| {
            if ui
                .checkbox(&mut cfg.hydrasdr.bias_tee, "DC on the antenna port")
                .on_hover_text(
                    "Powers an active antenna or preamp down the coax. Only the ANT \
                     socket has one — the two cable ports are plain inputs.",
                )
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

        ui.label("12-bit packing");
        ui.horizontal(|ui| {
            crate::chrome::checkbox(ui, &mut cfg.hydrasdr.packing, "Enable");
            ui.add(
                egui::Label::new(
                    RichText::new(
                        "A third less USB traffic. Leave it on: this is a USB 2.0 \
                         device and the top rate is 36 MB/s packed against 48 \
                         unpacked. Applies on reconnect.",
                    )
                    .weak(),
                )
                .wrap(),
            );
        });
        ui.end_row();

        ui.label("DC removal");
        if ui
            .checkbox(&mut cfg.hydrasdr.dc_block, "Remove the ADC's offset")
            .on_hover_text(
                "Turn it off to see raw hardware output. Worth knowing where the \
                 spur goes: the offset lands at the edge of the span, not its \
                 centre, because the signal is translated by a quarter of the \
                 sample rate on the way through.",
            )
            .changed()
        {
            push_gain(cmds, HydraSdrConfig::DC_BLOCK_ELEMENT, cfg.hydrasdr.dc_block as u8 as f64);
        }
        ui.end_row();
    });

    if cfg.hydrasdr.bias_tee && cfg.hydrasdr.rf_port.has_bias_tee() {
        ui.add_space(4.0);
        ui.label(
            RichText::new(
                "Bias tee is ON. Never connect a transceiver, a grounded antenna, \
                 or a preamp powered from elsewhere while this is enabled — the DC \
                 goes straight down the feedline.",
            )
            .color(crate::theme::YELLOW()),
        );
    }

    if devices.iter().any(|d| d.legacy_usb_id) {
        ui.add_space(4.0);
        ui.label(
            RichText::new(
                "One of these boards is on 1d50:60a1, the USB id HydraSDR's prototypes \
                 share with the Airspy R2 and Mini. sdroxide checks the firmware after \
                 opening and will say so if the wrong interface has been picked, in \
                 either direction.",
            )
            .weak(),
        );
    }

    if (cfg.hydrasdr.serial.clone(), cfg.hydrasdr.sample_rate_hz, cfg.hydrasdr.packing) != before {
        *apply = true;
    }

    ui.add_space(6.0);
    probe_only(ui, can_probe, |ui| {
        if ui
            .button("Copy diagnostic report")
            .on_hover_text(
                "Every command exchanged with the receiver, the sample-rate \
                 arithmetic, and the first samples decoded as I/Q.",
            )
            .clicked()
        {
            *copy_report = true;
        }
    });

    ui.add_space(4.0);
    ui.label(
        RichText::new(
            "Receive only, 24–1800 MHz. No SoapySDR and no libhydrasdr needed. The \
             receiver, sample rate and packing take effect on Apply; everything \
             else applies as you change it.",
        )
        .weak(),
    );
    ui.label(
        RichText::new(
            "Not yet verified against real hardware. If it misbehaves, please send the \
             diagnostic report — it contains every command exchanged with the receiver, \
             the rate arithmetic, and the first samples decoded as I/Q pairs.",
        )
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
        ui.label("Waiting for the configuration of the machine the radio is attached to.");
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
        ui.label("Receiver");
        probe_only(ui, can_probe, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button("Rescan")
                    .on_hover_text(
                        "Re-list Fobos SDRs. No device is opened, so this is safe to \
                         press while receiving.",
                    )
                    .clicked()
                {
                    *rescan = true;
                }
                let shown = if cfg.fobos.serial.is_empty() {
                    "— first one found —".to_string()
                } else {
                    cfg.fobos.serial.clone()
                };
                ComboBox::from_id_salt("fobos_dev").width(300.0).selected_text(shown).show_styled(
                    ui,
                    |ui| {
                        if devices.is_empty() {
                            ui.label("No Fobos SDR found — press Rescan");
                        }
                        ui.selectable_value(
                            &mut cfg.fobos.serial,
                            String::new(),
                            "— first one found —",
                        );
                        for d in devices {
                            ui.selectable_value(&mut cfg.fobos.serial, d.serial.clone(), d.label());
                        }
                    },
                );
            });
        });
        ui.end_row();

        ui.label("Input");
        ui.horizontal(|ui| {
            for p in FobosPort::ALL {
                let hover = match p {
                    FobosPort::Rf => {
                        "Through the tuner: LNA, VGA, and the mixer's own frequency range."
                    }
                    FobosPort::Hf1 => {
                        "Direct sampling on the HF1 antenna input — no tuner, no LNA/VGA \
                         gain, tuned entirely in software."
                    }
                    FobosPort::Hf2 => "Direct sampling on the HF2 antenna input, same as HF1.",
                    FobosPort::HfDual => {
                        "Both real ADC channels at once, combined by the diversity filter \
                         below — a noise source nulled, or two fading paths combined."
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
                    RichText::new(
                        "Direct sampling — the tuner (and so LNA/VGA gain) is not in the \
                         path. The rate below is a target for the software downconverter, \
                         not a hardware setting, so the achieved rate can differ from it.",
                    )
                    .weak(),
                )
                .wrap(),
            );
            ui.end_row();
        }

        ui.label("Sample rate");
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
                        "this receiver's own reported rates".to_string()
                    } else {
                        "measured on one real unit — connect this receiver and Rescan to \
                         see its own list"
                            .to_string()
                    })
                    .weak(),
                )
                .wrap(),
            );
        });
        ui.end_row();

        ui.label("LNA gain");
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

        ui.label("VGA gain");
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

        ui.label("Clock source");
        if ui.checkbox(&mut cfg.fobos.clk_external, "External reference").changed() {
            push_gain(cmds, FobosConfig::CLK_EXTERNAL_ELEMENT, cfg.fobos.clk_external as u8 as f64);
        }
        ui.end_row();

        // Which way it combines, how fast it chases, and holding it are on
        // the main window's own DIV strip once this radio is selected (see
        // `sdroxide_types::DIV_MODE_ELEMENT`'s own doc comment) — filter
        // length is the one control that stays here, the same split every
        // other diversity-capable backend's settings tab uses.
        if cfg.fobos.port == FobosPort::HfDual {
            ui.label("Diversity taps");
            ui.horizontal(|ui| {
                if ui
                    .add(
                        DragValue::new(&mut cfg.fobos.div_taps)
                            .speed(1.0)
                            .range(1..=FobosConfig::DIV_TAPS_MAX)
                            .suffix(" taps"),
                    )
                    .on_hover_text(
                        "One tap is a gain and a phase — a null at one frequency that \
                         gets worse either side of it, which is all an analogue phaser \
                         can do. Each further tap buys one sample period of the path \
                         difference between HF1 and HF2 that the filter can equalise, \
                         which is what turns that notch into a band quiet all the way \
                         across.",
                    )
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
    ui.label(
        RichText::new(
            "Receive only. RF port (tuner), HF1/HF2 (direct sampling, decoded in software) \
             and HF1+HF2 (both at once, combined by the diversity filter) all work. The \
             receiver, input and sample rate take effect on Apply; gain, clock source and \
             taps apply as you change them — the diversity filter's mode, adapt rate and \
             hold are on the main window's own DIV box once this radio is selected.",
        )
        .weak(),
    );
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
        ui.label("Waiting for the configuration of the machine the radio is attached to.");
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
        ui.label("Radio");
        probe_only(ui, can_probe, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button("Rescan")
                    .on_hover_text(
                        "Re-list the USB bus. No device is opened, so this is safe \
                         to press while receiving.",
                    )
                    .clicked()
                {
                    *rescan = true;
                }
                let shown = if cfg.hackrf.serial.is_empty() {
                    "— first one found —".to_string()
                } else {
                    cfg.hackrf.serial.clone()
                };
                ComboBox::from_id_salt("hackrf_dev").width(300.0).selected_text(shown).show_styled(
                    ui,
                    |ui| {
                        if devices.is_empty() {
                            ui.label("No HackRF found — press Rescan");
                        }
                        ui.selectable_value(
                            &mut cfg.hackrf.serial,
                            String::new(),
                            "— first one found —",
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

        ui.label("Sample rate");
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

        ui.label("LNA gain");
        if crate::chrome::slider(
            ui,
            egui::Slider::new(&mut cfg.hackrf.lna_db, 0.0..=40.0).step_by(8.0).suffix(" dB"),
        )
        .on_hover_text(
            "Front-end amplifier, in 8 dB steps. This is the stage that \
                 changes sensitivity — and the stage that overloads first on a \
                 real antenna.",
        )
        .changed()
        {
            push_gain(cmds, HackRfConfig::LNA_ELEMENT, cfg.hackrf.lna_db);
        }
        ui.end_row();

        ui.label("VGA gain");
        if crate::chrome::slider(
            ui,
            egui::Slider::new(&mut cfg.hackrf.vga_db, 0.0..=62.0).step_by(2.0).suffix(" dB"),
        )
        .on_hover_text(
            "Baseband amplifier after the mixer, in 2 dB steps. Turn this up \
                 for a weak signal before reaching for the LNA.",
        )
        .changed()
        {
            push_gain(cmds, HackRfConfig::VGA_ELEMENT, cfg.hackrf.vga_db);
        }
        ui.end_row();

        ui.label("RF amp");
        if ui
            .checkbox(
                &mut cfg.hackrf.amp,
                format!("{:.0} dB preamp on receive", HackRfConfig::AMP_DB),
            )
            .on_hover_text(
                "One switch, shared with the transmit setting below — the radio \
                 applies whichever belongs to the direction it is entering. Off \
                 is usually right on a real antenna.",
            )
            .changed()
        {
            push_gain(cmds, HackRfConfig::AMP_ELEMENT, cfg.hackrf.amp as u8 as f64);
        }
        ui.end_row();

        ui.label("Baseband filter");
        ui.horizontal(|ui| {
            let shown = if cfg.hackrf.filter_bw_hz <= 0.0 {
                "Automatic".to_string()
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
                        ui.selectable_value(&mut picked, 0.0, "Automatic");
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
                        "A HackRF Pro chooses this itself — three quarters of the \
                         sample rate, filtered in the FPGA — and ignores anything \
                         the host asks for."
                    } else {
                        "Leave on Automatic. Choosing one too narrow does not just \
                         soften the band edges — it withdraws the LO offset that \
                         keeps the DC spike off your signal."
                    })
                    .weak(),
                )
                .wrap(),
            );
        });
        ui.end_row();

        ui.label("Bias tee");
        if ui
            .checkbox(&mut cfg.hackrf.bias_tee, "DC on the antenna port")
            .on_hover_text(
                "About 3 V at 50 mA down the coax, for an active antenna or a \
                 preamp. A HackRF One or Pro only; the Jawbreaker and rad1o have \
                 no such circuit.",
            )
            .changed()
        {
            push_gain(cmds, HackRfConfig::BIAS_TEE_ELEMENT, cfg.hackrf.bias_tee as u8 as f64);
        }
        ui.end_row();

        ui.label("IQ correction");
        if ui
            .checkbox(&mut cfg.hackrf.iq_correction, "Remove DC and the mirror image")
            .on_hover_text(
                "This is a zero-IF radio, so its own LO leakage sits mid-span and \
                 the mixer's quadrature error puts a mirror image across it. \
                 Turning this off shows raw hardware output, which is the quick \
                 way to tell a driver problem from a DSP one.",
            )
            .changed()
        {
            push_gain(
                cmds,
                HackRfConfig::IQ_CORRECTION_ELEMENT,
                cfg.hackrf.iq_correction as u8 as f64,
            );
        }
        ui.end_row();

        ui.label("Clock trim");
        if ui
            .add(
                egui::DragValue::new(&mut cfg.hackrf.ppm)
                    .speed(0.1)
                    .range(-200.0..=200.0)
                    .suffix(" ppm"),
            )
            .on_hover_text("Corrects the reference oscillator.")
            .changed()
        {
            push_gain(cmds, HackRfConfig::PPM_ELEMENT, cfg.hackrf.ppm);
        }
        ui.end_row();
    });

    if cfg.hackrf.bias_tee {
        ui.add_space(4.0);
        ui.label(
            RichText::new(
                "Bias tee is ON. Never connect a transceiver, a grounded antenna, \
                 or a preamp powered from elsewhere while this is enabled — the DC \
                 goes straight down the feedline.",
            )
            .color(crate::theme::YELLOW()),
        );
    }

    ui.add_space(8.0);
    ui.separator();
    ui.label(RichText::new("Transmit").strong());
    ui.add_space(4.0);

    egui::Grid::new("hackrf-tx-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Transmitter");
        crate::chrome::checkbox(ui, &mut cfg.hackrf.tx_enabled, "Enabled").on_hover_text(
            "Off by default. While this is off the backend publishes no transmit \
             channel at all, so nothing can key the radio. Applies on reconnect.",
        );
        ui.end_row();

        if cfg.hackrf.tx_enabled {
            ui.label("TX VGA gain");
            if crate::chrome::slider(
                ui,
                egui::Slider::new(&mut cfg.hackrf.txvga_db, 0.0..=47.0).step_by(1.0).suffix(" dB"),
            )
            .on_hover_text(
                "The transmit driver amplifier. Drive is applied digitally \
                     before this stage, so leave the drive high and set output \
                     level here — turning drive down instead runs the DAC at a \
                     fraction of full scale and raises intermodulation.",
            )
            .changed()
            {
                cmds.push(Command::SetGain {
                    dir: Direction::Tx,
                    element: HackRfConfig::TXVGA_ELEMENT.into(),
                    db: cfg.hackrf.txvga_db,
                });
            }
            ui.end_row();

            ui.label("RF amp");
            if ui
                .checkbox(
                    &mut cfg.hackrf.tx_amp,
                    format!("{:.0} dB preamp on transmit", HackRfConfig::AMP_DB),
                )
                .on_hover_text(
                    "The same switch as the receive setting above, applied when \
                     the radio changes direction.",
                )
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
            RichText::new(
                "Transmit is armed. Into a dummy load until you have measured it: \
                 a HackRF's harmonics are strong enough to need an external \
                 low-pass filter for the band you are on, and it is half duplex, \
                 so receive stops for the length of every over.",
            )
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
                .button("Copy diagnostic report")
                .on_hover_text(
                    "Every command exchanged with the radio this session, in order, \
                     including the sequence around each key-down.",
                )
                .clicked()
            {
                *copy_report = true;
            }
        });
    });

    ui.add_space(4.0);
    ui.label(
        RichText::new(
            "DC – 7.25 GHz, half duplex. No SoapySDR, no libusb and no libhackrf \
             needed. The radio and the sample rate take effect on Apply; \
             everything else applies as you change it. The band edges are the \
             firmware's: a HackRF is specified for 1 MHz – 6 GHz (100 kHz on a \
             Pro) and is heavily attenuated outside that, but it does tune \
             there.",
        )
        .weak(),
    );
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
        ui.label("Waiting for the configuration of the machine the radio is attached to.");
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
            RichText::new(format!(
                "Serial {} is not among the receivers the SDRplay service reports. Pick one \
                 of the listed receivers, or replug this one and press Rescan.",
                cfg.sdrplay.serial.trim()
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
        ui.label("Receiver");
        // The service that answers this is the one on the engine's machine;
        // everything below reaches the RSP through it.
        probe_only(ui, can_probe, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button("Rescan")
                    .on_hover_text(
                        "Ask the SDRplay API service for its device list. Nothing is \
                         opened, so this is safe to press while receiving.",
                    )
                    .clicked()
                {
                    *rescan = true;
                }
                let shown = if cfg.sdrplay.serial.is_empty() {
                    "— first one found —".to_string()
                } else {
                    cfg.sdrplay.serial.clone()
                };
                ComboBox::from_id_salt("sdrplay_dev")
                    .width(300.0)
                    .selected_text(shown)
                    .show_styled(ui, |ui| {
                        if devices.is_empty() {
                            ui.label(
                                RichText::new(
                                    "no RSPs — press Rescan (needs the SDRplay API service)",
                                )
                                .weak(),
                            );
                        }
                        ui.selectable_value(
                            &mut cfg.sdrplay.serial,
                            String::new(),
                            "— first one found —",
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
        ui.label("Sample rate").on_hover_text(if dual {
            "With both tuners running, the API fixes the ADC at 6 MHz and hands back \
             2 Msps from a low IF — so 2 Msps is the widest span, and the narrower ones \
             are that decimated. Takes effect on Apply."
        } else {
            "Rates below 2 Msps run the ADC at 2 Msps and decimate in the service. \
             Takes effect on Apply."
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
                    label.push_str("  (reduced ADC resolution)");
                }
                if ui.selectable_label(sel, label).clicked() {
                    cfg.sdrplay.sample_rate_hz = r;
                }
            }
        });
        ui.end_row();

        ui.label("IF bandwidth").on_hover_text(
            "The tuner's analog filter. Auto picks the widest one that fits \
             the sample rate. Takes effect on Apply.",
        );
        let shown = if cfg.sdrplay.bw_khz == 0 {
            "Auto".to_string()
        } else {
            format!("{} kHz", cfg.sdrplay.bw_khz)
        };
        ComboBox::from_id_salt("sdrplay_bw").selected_text(shown).show_styled(ui, |ui| {
            if ui.selectable_label(cfg.sdrplay.bw_khz == 0, "Auto").clicked() {
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

        ui.label("AGC").on_hover_text(
            "The RSP's own IF-gain loop, run by the API service. Off hands the \
             IF gain slider back to you — the setting for measurement and \
             weak-signal digital modes.",
        );
        let mut agc = cfg.sdrplay.agc;
        enum_combo(ui, "sdrplay_agc", &mut agc, &SdrPlayAgc::ALL, SdrPlayAgc::label);
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
            ui.label("AGC set point").on_hover_text(
                "Signal level the loop holds the ADC at. Lower leaves more \
                 headroom for signals off-channel. How low it may go depends on \
                 the sample rate — above 8.064 Msps the converter has less \
                 headroom to give, and above 9.216 Msps less still.",
            );
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

        ui.label("IF gain reduction").on_hover_text(
            "The RSP's native gain unit: 20 dB is maximum gain (0 with the extended \
             range below), 59 dB minimum. \
             Applies immediately. Ignored while the AGC is running — the loop \
             owns this value then, and the S-meter shows what it settled on.",
        );
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

        ui.label("Extended IF range").on_hover_text(
            "Lets the IF gain reduction go below 20 dB, down to 0 — the last 20 dB \
             of gain the receiver has. Off is the API's own default and the right \
             one for ordinary listening, because the bottom of the range is where \
             an RSP is easiest to overload. Worth having on for weak signals with \
             the LNA already at 0. It also lets the AGC set point go up to 0 dBFS. \
             Applies immediately.",
        );
        {
            let mut on = cfg.sdrplay.extended_if_gr;
            if ui.checkbox(&mut on, "Allow IF gain reduction below 20 dB").changed() {
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

        ui.label("LNA state").on_hover_text(
            "Front-end attenuation in steps: 0 is maximum gain, each step up \
             switches more attenuation in. Some bands have fewer steps — the \
             driver clamps and keeps your choice for when you tune back, and \
             the rail here ends at what the band the radio is on will take. A \
             higher choice made on another band is kept and still shown beside \
             it. Applies immediately.",
        );
        // Clamped on edit only. The rail is this band's, but the value is a
        // stored preference that outlives the band: clamping it on sight would
        // have merely opening this tab on 40 m quietly trim a setting chosen
        // for 2 m — against the promise in the hover text right above.
        if crate::chrome::slider(
            ui,
            Slider::new(&mut cfg.sdrplay.lna_state, 0..=max_lna)
                .clamping(egui::SliderClamping::Edits),
        )
        .on_hover_text("0 = max gain")
        .changed()
        {
            cmds.push(Command::SetGain {
                dir: Direction::Rx,
                element: SdrPlayConfig::LNA_ELEMENT.to_string(),
                db: -(cfg.sdrplay.lna_state as f64),
            });
        }
        ui.end_row();

        ui.label("Frequency correction").on_hover_text(
            "Reference error in parts per million, applied by the device \
             itself. Applies immediately.",
        );
        let mut ppm = cfg.sdrplay.ppm;
        if ui
            .add(DragValue::new(&mut ppm).speed(0.1).range(-200.0..=200.0).suffix(" ppm"))
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
            ui.label(if cfg.sdrplay.duo.enabled { "This radio's tuner" } else { "Tuner" })
                .on_hover_text(match (cfg.sdrplay.duo.enabled, cfg.sdrplay.duo.role) {
                    (true, sdroxide_types::SdrPlayDuoRole::SecondRadio) => {
                        "Which of the RSPduo's two tuners this radio listens on. The other \
                         one belongs to the radio configured for it — give that one the \
                         same receiver and the other tuner. Takes effect on Apply."
                    }
                    (true, _) => {
                        "Which of the RSPduo's two tuners carries the aerial you are \
                         listening to. The other one carries the second aerial. Takes \
                         effect on Apply."
                    }
                    (false, _) => {
                        "Which of the RSPduo's two tuners to run (one at a time). Takes \
                         effect on Apply."
                    }
                });
            let mut tuner = cfg.sdrplay.duo_tuner;
            enum_combo(
                ui,
                "sdrplay_duo_tuner",
                &mut tuner,
                &SdrPlayDuoTuner::ALL,
                SdrPlayDuoTuner::label,
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
            ui.label("Antenna").on_hover_text("Applies immediately.");
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
            ui.label("FM broadcast notch");
            let mut on = cfg.sdrplay.rf_notch;
            if ui
                .checkbox(&mut on, "Enable")
                .on_hover_text(
                    "Hardware notch over the 88–108 MHz broadcast band, for \
                     when a local transmitter overloads everything else. \
                     Applies immediately.",
                )
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
            ui.label("DAB notch");
            let mut on = cfg.sdrplay.dab_notch;
            if ui
                .checkbox(&mut on, "Enable")
                .on_hover_text(
                    "Hardware notch over the 165–230 MHz DAB band. Applies \
                     immediately.",
                )
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
            ui.label("HDR mode");
            let mut on = cfg.sdrplay.hdr;
            if ui
                .checkbox(&mut on, "Enable below 2 MHz")
                .on_hover_text(
                    "The RSPdx's high-dynamic-range path for LF/MF. Applies \
                     immediately.\n\nIt does not work yet: on the one RSPdx it has been \
                     tried against, switching it on silences the receiver — at every \
                     centre, filter, port, gain and rate tried. Whatever the mode needs \
                     is not on the documented API surface, and finding it is somebody's \
                     next job. Leave this off unless you are the one looking.\n\nWhat is \
                     known: it is not a mode that follows the dial. The path has a fixed \
                     analog filter, built only at the centres listed under the filter \
                     below, so tuning anywhere else could not work even once the rest \
                     does.",
                )
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
                ui.label("HDR filter").on_hover_text(format!(
                    "The analog filter in front of the HDR path — a different control \
                     from the receiver's own bandwidth. Each setting is built at a \
                     fixed set of centres and can do nothing elsewhere; this one is \
                     built at {}. Applies immediately.\n\nThe path itself does not work \
                     yet — see the switch above — so this chooses a filter for a mode \
                     that is currently silent.",
                    cfg.sdrplay.hdr_bw.centres_label(),
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
            ui.label("Bias tee");
            let mut on = cfg.sdrplay.bias_tee;
            if ui
                .checkbox(&mut on, "Feed ~4.7 V DC up the coax")
                .on_hover_text("Powers an active antenna or preamp down the coax.")
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
        ui.label(RichText::new("Second tuner").strong());
        ui.label(
            RichText::new(
                "An RSPduo is two whole tuners on one board, clocked from one reference. \
                 Run both and they hear their spans at the same instant from the same \
                 clock — which makes two aerials on them coherent, with a relative phase \
                 set by the feedlines rather than by chance, and lets the pair be combined. \
                 Or leave them apart: the tuners tune separately, so the other one can be a \
                 second radio on its own band.",
            )
            .weak(),
        );
        if crate::chrome::checkbox(ui, &mut cfg.sdrplay.duo.enabled, "Run both tuners")
            .on_hover_text(
                "Puts the RSPduo in the API's dual-tuner mode, where the ADC is fixed at \
                 6 MHz and the widest span is 2 Msps. Takes effect on Apply, because the \
                 mode is chosen when the device is opened.",
            )
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
                    ui.label("Used for").on_hover_text(
                        "What the other tuner is for. Takes effect on Apply — like running \
                         both at all, this is chosen when the board is opened.",
                    );
                    let mut role = cfg.sdrplay.duo.role;
                    enum_combo(
                        ui,
                        "sdrplay_duo_role",
                        &mut role,
                        &SdrPlayDuoRole::ALL,
                        SdrPlayDuoRole::label,
                    );
                    cfg.sdrplay.duo.role = role;
                    ui.end_row();
                },
            );
        }

        if cfg.sdrplay.duo.enabled && cfg.sdrplay.duo.role == SdrPlayDuoRole::SecondRadio {
            ui.label(
                RichText::new(
                    "Add a second radio (Settings → Radio → +), give it this same receiver \
                     and the RSPduo's other tuner, and set it to run both tuners too — \
                     whichever radio opens the board puts it in dual-tuner mode, so both \
                     have to be expecting it. The two then tune independently: HF in one \
                     tab and VHF in the other, off one board. They share one ADC clock, so \
                     both run at the sample rate whichever radio opened the board asked \
                     for, and neither can transmit.",
                )
                .weak(),
            );
        }

        if cfg.sdrplay.duo.enabled && cfg.sdrplay.duo.role == SdrPlayDuoRole::Diversity {
            let div = &mut cfg.sdrplay.duo;
            egui::Grid::new("sdrplay-div-grid").num_columns(2).spacing([12.0, 6.0]).show(
                ui,
                |ui| {
                    ui.label("What to do with it");
                    ComboBox::from_id_salt("sdrplay_div_mode")
                        .selected_text(div.mode.label())
                        .show_styled(ui, |ui| {
                            for m in DiversityMode::ALL {
                                if ui.selectable_label(div.mode == m, m.label()).clicked() {
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

                    ui.label("Its LNA state").on_hover_text(
                        "The second tuner's own front-end attenuation. Set so both aerials \
                         show about the same noise floor: this is the adjustment everything \
                         else rests on, because combining weights the two branches by their \
                         noise, and a second front end driven into overload hands the filter \
                         a distorted copy of the interference — which cannot be subtracted \
                         from an undistorted one. Applies immediately.",
                    );
                    // Clamped on edit only, for the same reason as the first
                    // tuner's above: the rail is this band's, the value is a
                    // preference that outlives it.
                    if crate::chrome::slider(
                        ui,
                        Slider::new(&mut div.lna_state, 0..=max_lna)
                            .clamping(egui::SliderClamping::Edits),
                    )
                    .on_hover_text("0 = max gain")
                    .changed()
                    {
                        push_gain(cmds, SdrPlayConfig::AUX_LNA_ELEMENT, -(div.lna_state as f64));
                    }
                    ui.end_row();

                    ui.label("Its IF gain reduction").on_hover_text(
                        "The second tuner's IF gain, in the RSP's native unit: 20 dB is \
                         maximum gain. Ignored while the AGC is running — and a steady gain \
                         is what the filter wants, so switching the AGC off is worth it for \
                         a null you intend to hold.",
                    );
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

                    ui.label("Filter length");
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                DragValue::new(&mut div.taps)
                                    .speed(1.0)
                                    .range(1..=DIVERSITY_MAX_TAPS)
                                    .suffix(" taps"),
                            )
                            .on_hover_text(
                                "One tap is a gain and a phase — a null at one frequency \
                                 that gets worse either side of it, which is all an analogue \
                                 phaser can do. Each further tap buys one sample period of \
                                 the path difference between the two aerials that the filter \
                                 can equalise, which is what turns that notch into a band \
                                 quiet all the way across.",
                            )
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

                    ui.label("Adaptation");
                    ui.horizontal(|ui| {
                        if crate::chrome::slider(
                            ui,
                            Slider::new(&mut div.rate, 0.0..=1.0).show_value(false),
                        )
                        .on_hover_text(
                            "Slow and steady at the left, converging inside a fraction of a \
                             second and visibly hunting at the right. Start fast to find the \
                             null, then hold it.",
                        )
                        .changed()
                        {
                            push_gain(cmds, SdrPlayConfig::DIV_RATE_ELEMENT, f64::from(div.rate));
                        }
                        if ui
                            .checkbox(&mut div.frozen, "Hold")
                            .on_hover_text(
                                "Stop the filter moving. Reach for this the moment a null \
                                 appears: a filter left adapting will re-aim itself at \
                                 whatever becomes loudest, which on a quiet band is the \
                                 station you are listening to.",
                            )
                            .changed()
                        {
                            push_gain(
                                cmds,
                                SdrPlayConfig::DIV_FREEZE_ELEMENT,
                                f64::from(u8::from(div.frozen)),
                            );
                        }
                        if ui
                            .button("Restart")
                            .on_hover_text("Zero the filter and find the null again.")
                            .clicked()
                        {
                            push_gain(cmds, SdrPlayConfig::DIV_RESET_ELEMENT, 1.0);
                        }
                    });
                    ui.end_row();
                },
            );
            ui.label(
                RichText::new(
                    "Nothing here can tell a wanted signal from an unwanted one — the filter \
                     only knows what the two aerials have in common. In Cancel, the second \
                     aerial wants to hear the noise source and as little of the band as \
                     possible, or it will dutifully cancel the station too. In Combine, both \
                     want to hear the same station. How deep the null is going runs to the \
                     log every few seconds.",
                )
                .weak(),
            );
        }

        // Both of the second tuner's jobs rest on the same unverified
        // dual-tuner mode, and both are equally undone by a board that has
        // only one tuner — so these two say so wherever the setting is on.
        if cfg.sdrplay.duo.enabled {
            ui.label(
                RichText::new(
                    "Not yet verified against an RSPduo: dual-tuner operation here is \
                     written from SDRplay's API rather than measured on the hardware.",
                )
                .color(Color32::from_rgb(220, 170, 70)),
            );
            if !maybe_duo {
                ui.label(
                    RichText::new(format!(
                        "Only an RSPduo has a second tuner, and the receiver selected here \
                         is an {}, so this will do nothing.",
                        model.label()
                    ))
                    .color(Color32::from_rgb(220, 170, 70)),
                );
            } else if devices.is_empty() {
                ui.label(
                    RichText::new(
                        "No receiver is listed to check this against — press Rescan, or read \
                         the log for what actually opened.",
                    )
                    .weak(),
                );
            }
        }
    }

    if cfg.sdrplay.bias_tee && model.has_bias_tee() {
        ui.add_space(4.0);
        ui.label(
            RichText::new(
                "Bias tee is ON. Never connect a transceiver, a grounded antenna, \
                 or a preamp powered from elsewhere while this is enabled — the DC \
                 goes straight down the feedline.",
            )
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
    ui.label(
        RichText::new(
            "Receive only, 1 kHz–2 GHz. Needs the vendor's SDRplay API service \
             (sdrplay.com/api) — the RSPs after the original RSP1 have no open \
             protocol. Device, sample rate, bandwidth and RSPduo tuner take \
             effect on Apply; everything else applies as you change it.",
        )
        .weak(),
    );
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
        ui.label("Waiting for the configuration of the machine the radio is attached to.");
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
        ui.label("Board");
        probe_only(ui, can_probe, |ui| {
            ui.horizontal(|ui| {
                let current = devices
                    .iter()
                    .find(|d| d.matches(&cfg.lime.device))
                    .map(|d| d.label())
                    .unwrap_or_else(|| {
                        if cfg.lime.device.trim().is_empty() {
                            "First one found".to_string()
                        } else {
                            format!("{} (not found)", cfg.lime.device)
                        }
                    });
                egui::ComboBox::from_id_salt("lime-dev").selected_text(current).show_styled(
                    ui,
                    |ui| {
                        ui.selectable_value(&mut cfg.lime.device, String::new(), "First one found");
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
                if ui.button("Rescan").clicked() {
                    *rescan = true;
                }
            });
        });
        ui.end_row();

        // Which of the board's two front ends. Only worth a control on a board
        // that has two: on a Mini there is one chain and one set of sockets,
        // and a picker with a single entry is furniture.
        if chains > 1 || cfg.lime.channel > 0 {
            ui.label("Receive chain");
            ui.horizontal(|ui| {
                let text = format!("Chain {} (RX{}_*)", cfg.lime.channel + 1, cfg.lime.channel + 1);
                egui::ComboBox::from_id_salt("lime-chain").selected_text(text).show_styled(
                    ui,
                    |ui| {
                        for c in 0..chains.max(usize::from(cfg.lime.channel) + 1) as u8 {
                            ui.selectable_value(
                                &mut cfg.lime.channel,
                                c,
                                format!("Chain {} (RX{}_* / TX{}_*)", c + 1, c + 1, c + 1),
                            );
                        }
                    },
                );
                ui.label(
                    egui::RichText::new(
                        "Both chains tune together — they share one synthesiser — but they are \
                         separate front ends on separate sockets. Pick the one your aerial is \
                         on, which is the setting to reach for when one chain has had the HF \
                         matching modification and the other is stock.",
                    )
                    .weak(),
                );
            });
            ui.end_row();
        }

        ui.label("Sample rate");
        ui.horizontal(|ui| {
            let text = format!("{:.3} Msps", cfg.lime.sample_rate_hz / 1e6);
            egui::ComboBox::from_id_salt("lime-rate").selected_text(text).show_styled(ui, |ui| {
                for r in LimeConfig::SAMPLE_RATES {
                    let label = match LimeConfig::rate_note(r) {
                        Some(note) => format!("{:.3} Msps — {note}", r / 1e6),
                        None => format!("{:.3} Msps", r / 1e6),
                    };
                    ui.selectable_value(&mut cfg.lime.sample_rate_hz, r, label);
                }
            });
        });
        ui.end_row();

        ui.label("Receive gain");
        if crate::chrome::slider(
            ui,
            egui::Slider::new(
                &mut cfg.lime.rx_gain_db,
                LimeConfig::GAIN_MIN_DB..=LimeConfig::GAIN_MAX_DB,
            )
            .suffix(" dB")
            .step_by(1.0),
        )
        .on_hover_text(
            "One combined figure, which LimeSuite distributes across the LNA, the TIA and \
                 the PGA itself. It takes whole decibels, so anything finer is truncated.",
        )
        .changed()
        {
            push_gain(cmds, LimeConfig::RX_GAIN_ELEMENT, cfg.lime.rx_gain_db);
        }
        ui.end_row();

        ui.label("Receive port");
        ui.horizontal(|ui| {
            let text = if cfg.lime.antenna_rx.is_empty() {
                "Automatic".to_string()
            } else {
                LimeConfig::port_label(cfg.lime.channel, &cfg.lime.antenna_rx, false)
            };
            let before_rx = cfg.lime.antenna_rx.clone();
            let chan = cfg.lime.channel;
            egui::ComboBox::from_id_salt("lime-antrx").selected_text(text).show_styled(ui, |ui| {
                ui.selectable_value(&mut cfg.lime.antenna_rx, String::new(), "Automatic");
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
                egui::RichText::new(
                    "Automatic follows the frequency: LNAL low, LNAH high — unless a LimeRFE \
                     is connected below, which is one cable into one socket, and then it is \
                     LNAW at every frequency. Name the socket yours is wired to if it is not \
                     that one. Which chain the socket belongs to is the picker above.",
                )
                .weak(),
            );
        });
        ui.end_row();

        ui.label("Analog filter");
        ui.horizontal(|ui| {
            let mut mhz = cfg.lime.lpf_rx_hz / 1e6;
            if ui
                .add(egui::DragValue::new(&mut mhz).speed(0.1).range(0.0..=130.0).suffix(" MHz"))
                .on_hover_text(
                    "0 follows the sample rate. Worth leaving there: a filter narrower than a \
                     quarter of the span silently withdraws the zero-IF LO offset, which puts \
                     the LO leakage back on top of the signal you are listening to.",
                )
                .changed()
            {
                cfg.lime.lpf_rx_hz = mhz * 1e6;
                push_gain(cmds, LimeConfig::LPF_RX_ELEMENT, cfg.lime.lpf_rx_hz);
            }
            if cfg.lime.lpf_rx_hz == 0.0 {
                ui.label(egui::RichText::new("following the rate").weak());
            }
        });
        ui.end_row();

        ui.label("Corrections");
        ui.horizontal(|ui| {
            if ui
                .checkbox(&mut cfg.lime.iq_correction, "Host IQ / DC correction")
                .on_hover_text(
                    "Adaptive image and DC removal on this side, on top of the chip's own \
                     calibration. Turning it off is the one-click way to tell a driver problem \
                     from a DSP one.",
                )
                .changed()
            {
                push_gain(
                    cmds,
                    LimeConfig::IQ_CORRECTION_ELEMENT,
                    f64::from(u8::from(cfg.lime.iq_correction)),
                );
            }
            crate::chrome::checkbox(ui, &mut cfg.lime.calibrate, "Calibrate automatically")
                .on_hover_text(
                    "Runs the chip's own DC-offset and image calibration when the radio is \
                     opened, and again once the dial has settled on a new band or a different \
                     socket. Those numbers are measured at one frequency and are wrong \
                     elsewhere, which is what a carrier sitting in the middle of the span \
                     usually is. Costs about a second each time.",
                );
            if ui
                .button("Calibrate now")
                .on_hover_text("Stalls the receiver for the better part of a second.")
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
        ui.label(egui::RichText::new("Second aerial").strong());
        ui.label(
            egui::RichText::new(format!(
                "The board's other receive chain, on the {} sockets. It shares the \
                 synthesiser, so it hears the same span at the same instant as the first — \
                 which is what lets it carry a second aerial, or a sample of your own \
                 transmitter.",
                format_args!("RX{}_*", cfg.lime.aux_channel() + 1)
            ))
            .weak(),
        );
        let aux_chan = cfg.lime.aux_channel();
        egui::Grid::new("lime-aux-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label("Used for");
            egui::ComboBox::from_id_salt("lime-aux-role")
                .selected_text(cfg.lime.aux.role.label())
                .show_styled(ui, |ui| {
                    for r in LimeAuxRole::ALL {
                        ui.selectable_value(&mut cfg.lime.aux.role, r, r.label());
                    }
                });
            ui.end_row();

            if cfg.lime.aux.role != LimeAuxRole::Off {
                ui.label("Its socket");
                ui.horizontal(|ui| {
                    let text = if cfg.lime.aux.antenna.is_empty() {
                        "Same as the first".to_string()
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
                                "Same as the first",
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

                ui.label("Its gain");
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
                    "Set this LOW. The coupled sample of your own transmitter is a strong \
                     signal, and a feedback chain driven into compression measures the \
                     amplifier's curve wrongly — it teaches the correction its own \
                     distortion. Start at the bottom and use the coupler's attenuator."
                } else {
                    "Set so both aerials show about the same noise floor. This is the \
                     adjustment the whole thing rests on: combining weights the two branches \
                     by their noise, and a second chain driven into compression hands the \
                     filter a distorted copy of the interference — which cannot be subtracted \
                     from an undistorted one."
                })
                .changed()
                {
                    push_gain(cmds, LimeConfig::AUX_GAIN_ELEMENT, cfg.lime.aux.gain_db);
                }
                ui.end_row();
            }

            if cfg.lime.aux.role == LimeAuxRole::Diversity {
                ui.label("What to do with it");
                egui::ComboBox::from_id_salt("lime-div-mode")
                    .selected_text(cfg.lime.aux.mode.label())
                    .show_styled(ui, |ui| {
                        for m in DiversityMode::ALL {
                            if ui.selectable_label(cfg.lime.aux.mode == m, m.label()).clicked() {
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

                ui.label("Filter length");
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::DragValue::new(&mut cfg.lime.aux.taps)
                                .speed(1.0)
                                .range(1..=LimeAuxConfig::MAX_TAPS)
                                .suffix(" taps"),
                        )
                        .on_hover_text(
                            "One tap is a gain and a phase — a null at one frequency that gets \
                             worse either side of it, which is all an analogue phaser can do. \
                             Each further tap buys one sample period of the path difference \
                             between the two aerials that the filter can equalise, which is \
                             what turns that notch into a band quiet all the way across.",
                        )
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

                ui.label("Adaptation");
                ui.horizontal(|ui| {
                    if crate::chrome::slider(
                        ui,
                        egui::Slider::new(&mut cfg.lime.aux.rate, 0.0..=1.0).show_value(false),
                    )
                    .on_hover_text(
                        "Slow and steady at the left, converging inside a fraction of a second \
                         and visibly hunting at the right. Start fast to find the null, then \
                         hold it.",
                    )
                    .changed()
                    {
                        push_gain(cmds, LimeConfig::DIV_RATE_ELEMENT, f64::from(cfg.lime.aux.rate));
                    }
                    if ui
                        .checkbox(&mut cfg.lime.aux.frozen, "Hold")
                        .on_hover_text(
                            "Stop the filter moving. Reach for this the moment a null appears: \
                             a filter left adapting will re-aim itself at whatever becomes \
                             loudest, which on a quiet band is the station you are listening \
                             to.",
                        )
                        .changed()
                    {
                        push_gain(
                            cmds,
                            LimeConfig::DIV_FREEZE_ELEMENT,
                            f64::from(u8::from(cfg.lime.aux.frozen)),
                        );
                    }
                    if ui
                        .button("Restart")
                        .on_hover_text("Zero the filter and find the null again.")
                        .clicked()
                    {
                        push_gain(cmds, LimeConfig::DIV_RESET_ELEMENT, 1.0);
                    }
                });
                ui.end_row();
            }

            if cfg.lime.aux.role == LimeAuxRole::PureSignal {
                ui.label("Table steps");
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::DragValue::new(&mut cfg.lime.aux.ps_bins)
                                .speed(1.0)
                                .range(LimeAuxConfig::PS_MIN_BINS..=LimeAuxConfig::PS_MAX_BINS),
                        )
                        .on_hover_text(
                            "How finely the correction follows the amplifier's curve. More \
                             steps track a sharper knee, but each one has to be learned from \
                             the samples that landed in it — and the top of a speech \
                             amplitude histogram is thin. Thirty-two is enough for the smooth \
                             curve an HF amplifier actually has. Changing it starts the \
                             correction again.",
                        )
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

                ui.label("Adaptation");
                ui.horizontal(|ui| {
                    if crate::chrome::slider(
                        ui,
                        egui::Slider::new(&mut cfg.lime.aux.ps_rate, 0.0..=1.0).show_value(false),
                    )
                    .on_hover_text(
                        "How hard each block of feedback moves the correction. An \
                         amplifier's curve does not change, so there is no need to hurry — \
                         the middle averages several overs' worth of noise out of it.",
                    )
                    .changed()
                    {
                        push_gain(
                            cmds,
                            LimeConfig::PS_RATE_ELEMENT,
                            f64::from(cfg.lime.aux.ps_rate),
                        );
                    }
                    if ui
                        .checkbox(&mut cfg.lime.aux.ps_frozen, "Hold")
                        .on_hover_text(
                            "Keep the correction as it is. A curve learned on a clean over is \
                             worth holding, and the amplifier will not have changed by the \
                             next one.",
                        )
                        .changed()
                    {
                        push_gain(
                            cmds,
                            LimeConfig::PS_FREEZE_ELEMENT,
                            f64::from(u8::from(cfg.lime.aux.ps_frozen)),
                        );
                    }
                    if ui
                        .button("Restart")
                        .on_hover_text("Forget the correction and learn it again.")
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
                egui::RichText::new(
                    "A directional coupler on the amplifier's output goes into this chain, \
                     and the transmitter compares what came back with what it meant to send \
                     — then sends the inverse of the difference, so what leaves the amplifier \
                     is straight. Twenty-odd decibels less intermodulation on other people's \
                     QSOs, with the amplifier keeping its power. The correction stays at \
                     unity until the feedback lines up with the transmission, so a coupler \
                     that is not connected costs nothing; and it can never ask the converter \
                     for more than full scale, so a feedback path reading nonsense cannot \
                     over-drive anything. How it is getting on runs to the log while you \
                     transmit.",
                )
                .weak(),
            );
            if !cfg.lime.tx_enabled {
                ui.label(
                    egui::RichText::new(
                        "Transmit is not armed, so there is nothing here to correct.",
                    )
                    .color(egui::Color32::from_rgb(220, 170, 70)),
                );
            }
        }
        if cfg.lime.aux.role == LimeAuxRole::Diversity {
            ui.label(
                egui::RichText::new(
                    "Nothing here can tell a wanted signal from an unwanted one — the filter \
                     only knows what the two aerials have in common. In Cancel, the second \
                     aerial wants to hear the noise source and as little of the band as \
                     possible, or it will dutifully cancel the station too. In Combine, both \
                     want to hear the same station. How deep the null is going runs to the \
                     log every few seconds.",
                )
                .weak(),
            );
        }
    }

    ui.add_space(6.0);
    ui.separator();
    ui.label(egui::RichText::new("Transmit").strong());
    crate::chrome::checkbox(ui, &mut cfg.lime.tx_enabled, "Enabled").on_hover_text(
        "With this off the interface publishes no transmit channel at all, so nothing can key \
         the radio.",
    );
    if cfg.lime.tx_enabled {
        egui::Grid::new("lime-tx-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label("Transmit gain");
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

            ui.label("Transmit port").on_hover_text(
                "The board's two transmit sockets are two different matching networks, not \
                 two jacks onto the same one: BAND1 (TX_1) carries 30 MHz to 1.9 GHz and \
                 BAND2 (TX_2) is the 13 cm port. Keying an amateur band below 23 cm out of \
                 BAND2 puts the over into a matching network that passes almost none of it — \
                 every setting reads correct, the drive is whatever you set, and the power \
                 meter stays at zero. Leave this on Automatic and it follows the dial.",
            );
            let text = if cfg.lime.antenna_tx.is_empty() {
                "Automatic".to_string()
            } else {
                LimeConfig::tx_port_label(cfg.lime.channel, &cfg.lime.antenna_tx)
            };
            let before_tx = cfg.lime.antenna_tx.clone();
            let chan = cfg.lime.channel;
            egui::ComboBox::from_id_salt("lime-anttx").selected_text(text).show_styled(ui, |ui| {
                ui.selectable_value(&mut cfg.lime.antenna_tx, String::new(), "Automatic");
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
                egui::RichText::new(format!(
                    "Transmit gain is {:.0} dB, at the bottom of its range — a few microwatts \
                     out of the board, which will read as nothing on a power meter whatever is \
                     downstream of it. That is the default so that an armed transmitter cannot \
                     surprise you; raise it before you key.",
                    cfg.lime.tx_gain_db
                ))
                .color(egui::Color32::from_rgb(220, 170, 70)),
            );
        }
        ui.label(
            egui::RichText::new(
                "A LimeSDR transmits from about 100 kHz to 3.8 GHz with no filtering of its \
                 own. Use a low-pass filter, an appropriate LimeRFE channel, or a dummy load.",
            )
            .color(egui::Color32::from_rgb(220, 170, 70)),
        );
    }

    ui.add_space(6.0);
    ui.separator();
    ui.label(egui::RichText::new("LimeRFE front end").strong());

    // Everything below reaches an open board through one setting rather than a
    // control at a time — see `LimeConfig::RFE_SETTING`. Snapshotted here and
    // compared once at the end of the section, so a control added to the panel
    // is live without any plumbing of its own; before this the connectors, the
    // band and the relay mode had no door at all and only took effect on the
    // next restart (issue #94).
    let rfe_before = cfg.lime.rfe.clone();

    egui::Grid::new("lime-rfe-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Connected by");
        egui::ComboBox::from_id_salt("lime-rfe-link")
            .selected_text(cfg.lime.rfe.link.label())
            .show_styled(ui, |ui| {
                for l in RfeLink::ALL {
                    ui.selectable_value(&mut cfg.lime.rfe.link, l, l.label());
                }
            });
        ui.end_row();

        if cfg.lime.rfe.link == RfeLink::Serial {
            ui.label("Serial port");
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
                        egui::RichText::new(
                            "The LimeRFE's own micro-USB port, not the radio's. 9600 baud, \
                             fixed by its firmware.",
                        )
                        .weak(),
                    );
                });
            });
            ui.end_row();
        }
    });

    if cfg.lime.rfe.link != RfeLink::Off {
        egui::Grid::new("lime-rfe-grid2").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label("Receive connector");
            egui::ComboBox::from_id_salt("lime-rfe-prx")
                .selected_text(cfg.lime.rfe.port_rx.label())
                .show_styled(ui, |ui| {
                    for p in RfePort::RX_PORTS {
                        ui.selectable_value(&mut cfg.lime.rfe.port_rx, p, p.label());
                    }
                });
            ui.end_row();

            ui.label("Transmit connector");
            egui::ComboBox::from_id_salt("lime-rfe-ptx")
                .selected_text(cfg.lime.rfe.port_tx.label())
                .show_styled(ui, |ui| {
                    for p in RfePort::TX_PORTS {
                        ui.selectable_value(&mut cfg.lime.rfe.port_tx, p, p.label());
                    }
                });
            ui.end_row();

            ui.label("Band");
            ui.horizontal(|ui| {
                crate::chrome::checkbox(ui, &mut cfg.lime.rfe.follow_band, "Follow the dial")
                    .on_hover_text(
                        "Switch the filters to match the operating frequency, before any RF \
                     appears. Tuning within one band puts nothing on the control link.",
                    );
                if !cfg.lime.rfe.follow_band {
                    egui::ComboBox::from_id_salt("lime-rfe-chan")
                        .selected_text(cfg.lime.rfe.channel.label())
                        .show_styled(ui, |ui| {
                            for c in RfeChannel::ALL {
                                ui.selectable_value(&mut cfg.lime.rfe.channel, c, c.label());
                            }
                        });
                }
            });
            ui.end_row();

            ui.label("Relays");
            egui::ComboBox::from_id_salt("lime-rfe-mode")
                .selected_text(cfg.lime.rfe.mode.label())
                .show_styled(ui, |ui| {
                    for m in RfeModeControl::ALL {
                        ui.selectable_value(&mut cfg.lime.rfe.mode, m, m.label());
                    }
                });
            ui.end_row();

            ui.label("Receive attenuator");
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

            ui.label("Other");
            ui.horizontal(|ui| {
                crate::chrome::checkbox(ui, &mut cfg.lime.rfe.notch, "Notch filter");
                ui.checkbox(&mut cfg.lime.rfe.fan, "Fan")
                    .on_hover_text("Worth having on for any sustained transmitting.");
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
                egui::RichText::new(format!("Transmit is blocked: {refusal}"))
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
                    lines.push(format!(
                        "Receiving on {}, these fall back to the unfiltered wideband path: {}.",
                        cfg.lime.rfe.port_rx.label(),
                        unreachable_rx.join(", ")
                    ));
                }
                if !unreachable_tx.is_empty() {
                    lines.push(format!(
                        "Transmitting on {}, these fall back to the wideband path — no band \
                         amplifier and no filtering: {}.",
                        cfg.lime.rfe.port_tx.label(),
                        unreachable_tx.join(", ")
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
            egui::RichText::new(
                "On Automatic the board receives, and is switched to transmit at key-down and \
                 back at key-up — on either cabling. Its amateur channels have one filter \
                 with a transmit/receive switch either side of it, so a board asked for both \
                 at once puts that switch on the transmitter and stops hearing anything. \
                 The 30 MHz channel is reachable only through J5, which is one connector for \
                 both directions; above it, transmitting from J4 keeps the receive path off \
                 the connector the amplifier is driving.",
            )
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
            .button("Copy diagnostic report")
            .on_hover_text(
                "LimeSDR support has not been verified against hardware. This is the last \
                 session's trace, for an issue report.",
            )
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
        egui::RichText::new(
            "Gains, filters, corrections, the antenna sockets and every LimeRFE control — its \
             connectors, band, relays, attenuator, notch and fan — apply immediately. The \
             board, the receive chain, the sample rate, arming transmit and the LimeRFE's \
             connection take effect on Apply.",
        )
        .weak(),
    );
}
