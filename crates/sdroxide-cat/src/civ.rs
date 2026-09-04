//! Icom CI-V framing — also used by the Xiegu X6100, which speaks a CI-V
//! dialect. A frame is `FE FE <to> <from> <cmd> [data…] FD`.

use sdroxide_types::Mode;

pub const PREAMBLE: u8 = 0xFE;
pub const END: u8 = 0xFD;
/// The rig's "NG" answer: it will not do what the last command asked. Carries
/// no indication of *which* command, and models answer it for every
/// sub-command they don't implement.
pub const NG: u8 = 0xFA;
/// Controller (this software) address — conventional default.
pub const CONTROLLER_ADDR: u8 = 0xE0;

/// Encode a frequency (Hz) as 5 little-endian BCD bytes (CI-V cmd 0x05/0x03).
pub fn encode_freq(hz: f64) -> [u8; 5] {
    let mut v = hz.round().max(0.0) as u64;
    let mut out = [0u8; 5];
    for b in out.iter_mut() {
        let lo = (v % 10) as u8;
        v /= 10;
        let hi = (v % 10) as u8;
        v /= 10;
        *b = (hi << 4) | lo;
    }
    out
}

/// Decode 5 little-endian BCD bytes back to a frequency in Hz.
pub fn decode_freq(bytes: &[u8]) -> Option<f64> {
    if bytes.len() < 5 {
        return None;
    }
    let mut hz: u64 = 0;
    for &b in bytes[..5].iter().rev() {
        let hi = (b >> 4) as u64;
        let lo = (b & 0x0f) as u64;
        if hi > 9 || lo > 9 {
            return None;
        }
        hz = hz * 100 + hi * 10 + lo;
    }
    Some(hz as f64)
}

/// The app's `Mode` → CI-V mode byte. Digital modes ride on their sideband.
pub fn mode_to_civ(m: Mode) -> u8 {
    match m {
        Mode::Lsb | Mode::Digl => 0x00,
        Mode::Usb
        | Mode::Digu
        | Mode::Ft8
        | Mode::Js8
        | Mode::Wspr
        | Mode::Ft4
        | Mode::Ft2
        | Mode::Psk
        | Mode::Rtty
        | Mode::Sstv
        | Mode::Wefax
        | Mode::Navtex
        | Mode::Olivia
        | Mode::Thor
        | Mode::Fsq
        | Mode::Hell
        | Mode::RfPaint
        | Mode::Rade
        // HF packet is 300 baud AFSK on a sideband, like any keyboard mode.
        | Mode::PacketHf => 0x01,
        Mode::Am | Mode::Sam | Mode::Dsb | Mode::Isb | Mode::Drm => 0x02,
        Mode::Cw => 0x03,
        // RIFP is FSK on the carrier, and VHF packet frequency-modulates it,
        // so a CAT rig has to be in FM for the dial to mean what they mean by
        // it.
        // No rig has an ADS-B mode and none ever will: the dial is at
        // 1090 MHz. Grouped with FM so nothing downstream has to special-case
        // a mode a radio can neither be put into nor report back.
        Mode::Nfm
        | Mode::Wfm
        | Mode::Rifp
        | Mode::Packet
        | Mode::Aprs
        | Mode::SstvFm
        | Mode::RttyFm
        | Mode::Adsb
        | Mode::Vdl2
        | Mode::Ais => 0x05,
        Mode::Spec => 0x01,
    }
}

/// CI-V mode byte → the app's `Mode`.
pub fn civ_to_mode(b: u8) -> Option<Mode> {
    Some(match b {
        0x00 => Mode::Lsb,
        0x01 => Mode::Usb,
        0x02 => Mode::Am,
        0x03 | 0x07 => Mode::Cw,
        0x05 | 0x06 => Mode::Nfm,
        _ => return None,
    })
}

/// Build a CI-V frame addressed to `radio`.
pub fn frame(radio: u8, cmd: u8, data: &[u8]) -> Vec<u8> {
    let mut f = Vec::with_capacity(6 + data.len());
    f.extend_from_slice(&[PREAMBLE, PREAMBLE, radio, CONTROLLER_ADDR, cmd]);
    f.extend_from_slice(data);
    f.push(END);
    f
}

pub fn set_freq_frame(radio: u8, hz: f64) -> Vec<u8> {
    frame(radio, 0x05, &encode_freq(hz))
}
pub fn read_freq_frame(radio: u8) -> Vec<u8> {
    frame(radio, 0x03, &[])
}
pub fn set_mode_frame(radio: u8, m: Mode) -> Vec<u8> {
    // mode byte + filter 1; the X6100 accepts the two-byte form.
    frame(radio, 0x06, &[mode_to_civ(m), 0x01])
}
pub fn read_mode_frame(radio: u8) -> Vec<u8> {
    frame(radio, 0x04, &[])
}

/// The mode, and on a model that has one, the DATA-mode switch beside it.
///
/// CI-V has no separate DATA mode: USB and USB-DATA are the same mode byte
/// (`0x01`), and what tells them apart is a second command. Without it a
/// digital-mode over goes out of the *microphone* input with the rig's speech
/// processing and SSB transmit filter in the path — audible on the air as a
/// signal that is wide, compressed and short of the passband its decoder wants.
///
/// `data_sub` is the `1A` sub-command that carries the switch on this model
/// (see `IcomModel::data_mode_sub`), or `None` where there is none — an
/// IC-7000 selects its data input at the radio, and nothing here can reach it.
pub fn set_mode_frames(radio: u8, m: Mode, data_sub: Option<u8>) -> Vec<Vec<u8>> {
    let mut out = vec![set_mode_frame(radio, m)];
    if let Some(sub) = data_sub {
        // Filter 1 alongside the switch, which is the same filter the mode
        // command above selects — so the pair leave the rig somewhere
        // consistent rather than in the mode's filter and the data mode's.
        //
        // Every digital mode, including the ones that key the carrier rather
        // than a sideband. On Icom the DATA switch is *not* a sideband
        // selector — it is the mode's `-D` variant, and FM has one: FM-D takes
        // its modulation from the data source with the audio flat.
        //
        // This used to exclude the carrier-centred modes, on the theory that
        // the switch chose a sideband data path they did not use. It does not,
        // and the cost was exactly what the note above this function describes:
        // a packet or APRS over went out through the microphone path with the
        // rig's speech processing and FM pre-emphasis on it. Pre-emphasis
        // alone tilts a Bell 202 tone pair by about 6 dB, which sounds like
        // packet to a listener and is unreadable to a modem. Kenwood's map has
        // always answered DATA-FM here (`mode_digit`); this is Icom agreeing
        // with it.
        //
        // `Digu`/`Digl` are in it because on this family they *are* nothing but
        // the switch: CI-V has one mode byte for USB and USB-D, so a rig asked
        // for DIGU and sent `06 01` alone lands in plain USB with the switch
        // explicitly cleared. That is what `Digital mode: DIGI` maps every
        // digital mode's sideband onto, so on an Icom the setting did nothing
        // at all — FT8, PSK, SSTV and the rest went out of the microphone
        // input (issue #313). Every other family spells the data position into
        // the mode itself (`MD2;DA1;`, `MD0C;`, `MD6;DT0;`) and has always
        // been right here.
        let on = m.is_digital() || matches!(m, Mode::Digu | Mode::Digl);
        out.push(frame(radio, 0x1A, &[sub, on as u8, 0x01]));
    }
    out
}
pub fn ptt_frame(radio: u8, on: bool) -> Vec<u8> {
    frame(radio, 0x1C, &[0x00, on as u8])
}
/// Read the PTT line (Icom cmd `0x1C` sub `0x00`, no payload) — the read form
/// of the same command [`ptt_frame`] keys with.
///
/// The rig answers `1C 00 <00|01>`, which is the only way to learn about an
/// over the operator started at the radio: CI-V pushes nothing unasked unless
/// the transceive setting is on, and sdroxide deliberately does not rely on
/// that being on.
pub fn read_ptt_frame(radio: u8) -> Vec<u8> {
    frame(radio, 0x1C, &[0x00])
}

/// Parse a transceiver-status reply payload (cmd `0x1C`): the sub-command byte
/// followed by its value. `None` for any sub-command other than `0x00` (PTT) —
/// `0x01` is the ATU/tuner on the same command, and reading its state as a
/// key-down would put the meter into transmit every time the tuner ran.
pub fn parse_ptt_reply(data: &[u8]) -> Option<bool> {
    match data {
        [0x00, v] => Some(*v != 0),
        _ => None,
    }
}

/// Read the SWR meter (Icom cmd `0x15` sub `0x12`). Only meaningful while
/// transmitting; the rig answers with a 0..255 reading (see [`swr_from_reading`]).
pub fn read_swr_frame(radio: u8) -> Vec<u8> {
    frame(radio, 0x15, &[0x12])
}
/// Read the ALC meter (Icom cmd `0x15` sub `0x13`). Only meaningful while
/// transmitting, like the SWR meter beside it.
///
/// ALC is the rig telling us how hard its own automatic level control is
/// working, which is the number that says whether the audio being fed to it is
/// too hot. Nothing on this side can compute it: the drive level SDRoxide
/// measures is what it SENDS, and ALC is what the rig does about it.
pub fn read_alc_frame(radio: u8) -> Vec<u8> {
    frame(radio, 0x15, &[0x13])
}
/// Read the power-output meter (Icom cmd `0x15` sub `0x11`). Only meaningful
/// while transmitting, like the SWR meter beside it; the rig answers with a
/// 0..255 reading (see [`po_from_reading`]).
///
/// This is what the rig is actually putting out, which is a different question
/// from the drive level it was *asked* for (cmd `0x14` sub `0x0A`, read once at
/// connect to place the Drive slider). The two disagree whenever the rig is
/// folding back — heat, a high SWR, or an ATU still searching — and that
/// disagreement is the reason for showing this at all.
pub fn read_po_frame(radio: u8) -> Vec<u8> {
    frame(radio, 0x15, &[0x11])
}

/// Read the S-meter (Icom cmd `0x15` sub `0x02`). The rig answers with a 0..255
/// reading on its own calibrated scale (see [`dbm_from_smeter`]).
///
/// A CAT rig hands us audio it has already demodulated and levelled, so nothing
/// on this side of the sound card can measure a signal strength — the rig's own
/// meter is the only S-meter there is. Only meaningful while receiving.
pub fn read_smeter_frame(radio: u8) -> Vec<u8> {
    frame(radio, 0x15, &[0x02])
}

/// Longest CW message one "send CW" frame carries. The rig buffers what it is
/// given and keys it out at its own speed; more than this in a frame is
/// refused, so the sender chunks to it.
pub const CW_MAX: usize = 30;

/// Reduce `text` to the characters an Icom keyer will send: upper case, the
/// letters, digits and punctuation it has Morse for. A character it does not
/// know would be refused along with the rest of the frame, so it is dropped
/// here instead.
fn cw_text(text: &str) -> String {
    text.trim()
        .chars()
        // A line break is a word break, not nothing: dropping it would run the
        // end of one line into the start of the next and send them as one word.
        .map(|c| if c.is_ascii_whitespace() { ' ' } else { c.to_ascii_uppercase() })
        .filter(|c| {
            c.is_ascii_alphanumeric() || matches!(c, ' ' | '/' | '?' | '.' | ',' | '-' | '=')
        })
        .scan(false, |prev_space, c| {
            let space = c == ' ';
            let keep = !(space && *prev_space);
            *prev_space = space;
            Some(keep.then_some(c))
        })
        .flatten()
        .take(CW_MAX)
        .collect()
}

/// Send `text` as CW from the rig's own keyer (cmd `0x17`). `None` when nothing
/// in `text` is sendable — an empty frame would be a rejected frame.
///
/// The rig keys itself for the length of the message, so this must not be
/// wrapped in PTT. Break-in decides whether it keys at all — see
/// [`set_break_in_frame`].
pub fn send_cw_frame(radio: u8, text: &str) -> Option<Vec<u8>> {
    let msg = cw_text(text);
    (!msg.is_empty()).then(|| frame(radio, 0x17, msg.as_bytes()))
}

/// Stop a message part way through: `0xFF` is the abort payload of the same
/// send-CW command.
pub fn stop_cw_frame(radio: u8) -> Vec<u8> {
    frame(radio, 0x17, &[0xFF])
}

/// Set the keyer speed (cmd `0x14` sub `0x0C`). Icom carries it as its generic
/// 0–255 level over a 6–48 WPM range, so the panel's speed is mapped onto that
/// scale rather than sent as a number of words.
pub fn keyer_speed_frame(radio: u8, wpm: f32) -> Vec<u8> {
    let wpm = wpm.clamp(6.0, 48.0);
    let level = (((wpm - 6.0) * (255.0 / 42.0)).round() as u32).min(255);
    let [hi, lo] = encode_level(level);
    frame(radio, 0x14, &[0x0C, hi, lo])
}

/// The rig's break-in setting, which is what decides whether a message handed
/// to its keyer keys the transmitter at all.
///
/// CI-V `16 47`, on the "various settings" command every current Icom carries:
/// `00` off, `01` semi break-in, `02` full break-in (QSK). A model without it
/// answers NG, which costs one frame and nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakIn {
    Off,
    /// Semi: the rig keys on the first element and drops back to receive after
    /// its delay. What a message sent from a computer wants.
    Semi,
    /// Full (QSK): receive between elements.
    Full,
}

impl BreakIn {
    fn code(self) -> u8 {
        match self {
            BreakIn::Off => 0x00,
            BreakIn::Semi => 0x01,
            BreakIn::Full => 0x02,
        }
    }

    fn from_code(b: u8) -> Option<BreakIn> {
        match b {
            0x00 => Some(BreakIn::Off),
            0x01 => Some(BreakIn::Semi),
            0x02 => Some(BreakIn::Full),
            _ => None,
        }
    }

    /// Whether a message given to the keyer will actually go out on the air.
    pub fn keys_the_transmitter(self) -> bool {
        self != BreakIn::Off
    }
}

/// The `16` sub-command that carries break-in.
const BREAK_IN_SUB: u8 = 0x47;

/// Set the break-in function (cmd `0x16` sub `0x47`).
///
/// Without this a CW message sent with [`send_cw_frame`] is *accepted and not
/// transmitted*: the radio's own reference says a message from the PC goes out
/// only while `[TRANSMIT]` is on, an external TX switch is closed, or break-in
/// is on — and the first two are front-panel things an operator working the
/// radio over a network cannot reach (issue #282).
pub fn set_break_in_frame(radio: u8, bk: BreakIn) -> Vec<u8> {
    frame(radio, 0x16, &[BREAK_IN_SUB, bk.code()])
}

/// Read the break-in setting back (cmd `0x16` sub `0x47`, no value).
pub fn read_break_in_frame(radio: u8) -> Vec<u8> {
    frame(radio, 0x16, &[BREAK_IN_SUB])
}

/// The break-in setting in a `16` reply, or `None` for the several dozen other
/// sub-commands answered on the same command byte.
///
/// The length is what tells an answer from the question: a read carries the
/// sub-command and nothing else — which is exactly what our own frame looks
/// like on its way out — and an answer appends the value.
pub fn parse_break_in_reply(data: &[u8]) -> Option<BreakIn> {
    if data.first() != Some(&BREAK_IN_SUB) || data.len() < 2 {
        return None;
    }
    BreakIn::from_code(data[1])
}

/// Icom's generic 0–255 level as the two BCD bytes its level commands carry
/// (`0000`..`0255`) — the write side of [`decode_meter`].
fn encode_level(level: u32) -> [u8; 2] {
    let level = level.min(255);
    let bcd = |v: u32| (((v / 10) << 4) | (v % 10)) as u8;
    [bcd(level / 100), bcd(level % 100)]
}

/// Set the transmit output power (cmd `0x14` sub `0x0A`), as a 0..1 fraction of
/// what the radio can do. Icom carries power on the same 0–255 level scale as
/// everything else in this command block, spanning the rig's whole range — so
/// the fraction is the setting, whether the radio behind it is a 100 W IC-7300
/// or a 10 W IC-705.
///
/// This is the only transmit level control a CAT rig has that works in *every*
/// mode. The audio we put into its sound card is not one: a rig in CW keys its
/// own transmitter from its own keyer and never looks at the sound card at all,
/// which is why a drive slider that only scaled audio did nothing on CW.
pub fn set_power_frame(radio: u8, frac: f32) -> Vec<u8> {
    let level = (frac.clamp(0.0, 1.0) * 255.0).round() as u32;
    let mut data = vec![0x0A];
    data.extend_from_slice(&encode_level(level));
    frame(radio, 0x14, &data)
}

/// Read the transmit output power back (cmd `0x14` sub `0x0A`, no payload).
pub fn read_power_frame(radio: u8) -> Vec<u8> {
    frame(radio, 0x14, &[0x0A])
}

/// Parse a level reply payload (cmd `0x14`): the sub-command byte followed by
/// the BCD level. Returns the power as a 0..1 fraction, or `None` if the reply
/// is some other level (keyer speed, AF gain…) or is malformed.
pub fn parse_power_reply(data: &[u8]) -> Option<f32> {
    if data.first() != Some(&0x0A) {
        return None;
    }
    Some(decode_meter(&data[1..])?.min(255) as f32 / 255.0)
}

/// Set the receiver's squelch threshold (cmd `0x14` sub `0x03`), as a 0..1
/// fraction of the radio's own scale — `0` fully open, `1` fully closed, which
/// is how the knob on the front panel reads.
///
/// The squelch that matters on a rig sending demodulated audio is the rig's.
/// Nothing on this side of the sound card can open one the radio has closed:
/// what arrives is already gated, so the software squelch could only close
/// further on what got through, and an operator who wanted to hear a weak
/// station had no control that reached the thing muting it (issue #192).
pub fn set_squelch_frame(radio: u8, frac: f32) -> Vec<u8> {
    let level = (frac.clamp(0.0, 1.0) * 255.0).round() as u32;
    let mut data = vec![0x03];
    data.extend_from_slice(&encode_level(level));
    frame(radio, 0x14, &data)
}

/// Read the squelch threshold back (cmd `0x14` sub `0x03`, no payload), so the
/// slider can *adopt* where the operator left the radio rather than impose a
/// remembered level on it — the same reason [`read_power_frame`] exists.
pub fn read_squelch_frame(radio: u8) -> Vec<u8> {
    frame(radio, 0x14, &[0x03])
}

/// Parse a squelch-level reply payload (cmd `0x14` sub `0x03`) as a 0..1
/// fraction. `None` for any other level on the same command, exactly as
/// [`parse_power_reply`] answers `None` for this one.
pub fn parse_squelch_reply(data: &[u8]) -> Option<f32> {
    if data.first() != Some(&0x03) {
        return None;
    }
    Some(decode_meter(&data[1..])?.min(255) as f32 / 255.0)
}

/// Switch the radio off (`18 00`) or on (`18 01`).
///
/// Powering **on** is the awkward direction. A sleeping Icom's CI-V receiver is
/// clocked down, and the first bytes of a frame arriving at it are lost — so
/// Icom's own instruction is to send a run of `0xFE` wake-up bytes in front of
/// the frame, long enough to cover the time the receiver takes to come up. How
/// many depends on how fast they go out, which is why `wake` is a count rather
/// than a constant: [`WAKE_BYTES_FOR`] works it out from the port's baud rate,
/// and a link with no baud rate to speak of — the radio's own network socket —
/// passes the figure Icom's remote-control software uses there.
///
/// Switching **off** needs none of that: the radio is awake and listening.
///
/// Transcribed from the CI-V reference guides (the command is `0x18` on every
/// model that has it) and from wfview, which drives the same two frames over
/// both link types. Not exercised against a radio here (issue #239).
pub fn power_frames(radio: u8, on: bool, wake: usize) -> Vec<Vec<u8>> {
    let f = frame(radio, 0x18, &[u8::from(on)]);
    if !on || wake == 0 {
        return vec![f];
    }
    vec![vec![0xFE; wake], f]
}

/// How many `0xFE` wake-up bytes go in front of a power-on frame at `baud`.
///
/// Icom's figure is a *duration* — the radio's CI-V receiver needs roughly
/// 14 ms of line activity to come up — so the count scales with the rate. The
/// standard rates are the table Icom's own guides print and Hamlib sends; any
/// other rate falls back to the same 14 ms worked out from the line speed, with
/// a floor under it so a very slow port still sends a recognisable run.
pub fn wake_bytes_for(baud: u32) -> usize {
    match baud {
        4800 => 7,
        9600 => 13,
        19200 => 27,
        38400 => 53,
        57600 => 79,
        115200 => 159,
        // Ten bit-times to the byte on an 8N1 line, 14 ms of them.
        b => ((b as f64 * 0.014) / 10.0).round().max(7.0) as usize,
    }
}

/// The wake-up run to send over a link with no baud rate — the radio's own
/// network socket, where the frame goes out in one packet. Icom's remote
/// software sends 150 there and so does wfview.
pub const WAKE_BYTES_LAN: usize = 150;

/// The antenna sockets an Icom's antenna selector switches between, in the
/// order the command numbers them: `ANT1` is `0x00`.
///
/// Two, not four. The command carries a socket *number* and there is nothing in
/// CI-V that says how many a given model has: an IC-7610 and an IC-7700 have
/// two, the IC-785x line has four, and most of the range — an IC-7300, an
/// IC-705, an IC-9700, whose three sockets are one per band and not selectable
/// — has one and answers [`read_antenna_frame`] with a NAK. Two is what every
/// model with a *selector* has at least, and offering a fourth socket to a
/// radio with two would move a relay to a connector that is not there. See
/// `Protocol::antennas` for how the list is only published once the rig has
/// answered that read at all.
pub const ANTENNAS: [&str; 2] = ["ANT1", "ANT2"];

/// Put the receiver — and the transmitter with it; Icom's selector is one
/// relay — on `name`, one of [`ANTENNAS`]. `None` for a name from another
/// radio's list, which must never be allowed to pick a socket number by
/// accident.
///
/// Three bytes rather than two: `12 <socket> <rx-antenna>`, the form wfview
/// sends and the one the models with a separate receive-antenna connector
/// (IC-7610, IC-7851) expect. The trailing byte is that connector's own
/// setting and is left at `0` — sdroxide does not offer it, and writing
/// anything else here would switch a receive-only input the operator never
/// asked about.
pub fn set_antenna_frame(radio: u8, name: &str) -> Option<Vec<u8>> {
    let n = ANTENNAS.iter().position(|a| a.eq_ignore_ascii_case(name))? as u8;
    Some(frame(radio, 0x12, &[n, 0x00]))
}

/// Ask which socket the rig is on (cmd `0x12`, no payload).
///
/// Sent once when the link opens, and it asks two questions at once: which
/// socket, and — because a radio with one antenna NAKs it — whether there is a
/// selector here at all.
pub fn read_antenna_frame(radio: u8) -> Vec<u8> {
    frame(radio, 0x12, &[])
}

/// Read an antenna reply (cmd `0x12`) back to one of [`ANTENNAS`].
///
/// The payload is the socket number, optionally followed by the receive-antenna
/// connector's setting, which is ignored here. A socket past the two this end
/// offers — a four-socket IC-785x left on ANT3 — answers `None` rather than
/// being rounded onto one of them: it is a real port sdroxide cannot name, and
/// claiming the radio is on ANT1 would be a lie the operator could act on.
pub fn parse_antenna_reply(data: &[u8]) -> Option<&'static str> {
    let n = *data.first()? as usize;
    ANTENNAS.get(n).copied()
}

/// Hand the rig's *own* RIT, ΔTX (XIT) and split back to neutral.
///
/// sdroxide carries all three on the dial itself (see `AudioCatSource`), so an
/// offset the rig is still holding — from a previous session, or from the
/// operator's own RIT knob — would stack on top of ours where nothing in the
/// software could see it. Sent once when the port opens; a rig that doesn't
/// implement these sub-commands just NAKs them, which the parser ignores.
///
/// The repeater shift is the fourth of these and is deliberately not here: it
/// is not a setting the radio holds still, so clearing it once is not enough.
/// See [`simplex_frame`].
pub fn clear_offsets_frames(radio: u8) -> Vec<Vec<u8>> {
    vec![
        // RIT and ΔTX share one offset register (cmd 0x21 sub 0x00): the offset
        // as two little-endian BCD bytes, then a sign byte.
        frame(radio, 0x21, &[0x00, 0x00, 0x00, 0x00]),
        frame(radio, 0x21, &[0x01, 0x00]), // RIT off
        frame(radio, 0x21, &[0x02, 0x00]), // ΔTX (XIT) off
        // Split off. `0x0F` carries two settings on one command: `00`/`01` are
        // split, and `10`/`11`/`12` are the duplex shift — see
        // [`simplex_frame`], which is the other half of this and cannot be
        // sent from here.
        frame(radio, 0x0F, &[0x00]),
    ]
}

/// Put the rig's own repeater shift back to simplex (cmd `0x0F` sub `0x10`).
///
/// The transmit shift is sdroxide's, carried on the dial like RIT, XIT and
/// split (see [`clear_offsets_frames`] and `RadioState::tx_freq_hz`), so a
/// duplex setting still standing in the radio would be added to ours at the
/// key-down and put the over a shift away from where the operator asked for
/// it — and the SIMPLEX chip on screen could not take it off again.
///
/// Unlike the three above, this cannot be cleared once and forgotten. A radio
/// with band stacking registers — an IC-9700 keeps one per band, holding the
/// frequency, the mode *and* the duplex and tone settings — restores whatever
/// that band was last left on the moment the dial crosses into it, which on
/// 70 cm and 23 cm is normally DUP− (issue #192). Icom's own auto-repeater
/// function does the same thing on the USA models. So it is re-asserted
/// whenever the dial moves to another band; a rig with no such command NAKs
/// it, which the parser ignores.
pub fn simplex_frame(radio: u8) -> Vec<u8> {
    frame(radio, 0x0F, &[0x10])
}

/// Decode Icom's 2-byte BCD meter reading (`0000..0255`) to a plain integer.
/// `data` is the payload after the meter sub-command byte.
fn decode_meter(data: &[u8]) -> Option<u32> {
    let bcd = |b: u8| -> Option<u32> {
        let (hi, lo) = ((b >> 4) as u32, (b & 0x0f) as u32);
        (hi <= 9 && lo <= 9).then_some(hi * 10 + lo)
    };
    let (a, b) = (data.first()?, data.get(1)?);
    Some(bcd(*a)? * 100 + bcd(*b)?)
}

/// Map an Icom SWR-meter reading (`0..255`) to an SWR ratio via piecewise-linear
/// interpolation over the standard calibration breakpoints (matching Hamlib's
/// Icom SWR curve: 0→1.0, 48→1.5, 80→2.0, 120→3.0), extended past 3:1 for the
/// rare high-SWR case. Clamped to the table ends.
fn swr_from_reading(reading: u32) -> f32 {
    const CAL: &[(f32, f32)] =
        &[(0.0, 1.0), (48.0, 1.5), (80.0, 2.0), (120.0, 3.0), (160.0, 5.0), (255.0, 10.0)];
    let r = reading as f32;
    if r <= CAL[0].0 {
        return CAL[0].1;
    }
    for w in CAL.windows(2) {
        let (x0, y0) = w[0];
        let (x1, y1) = w[1];
        if r <= x1 {
            return y0 + (y1 - y0) * (r - x0) / (x1 - x0);
        }
    }
    CAL[CAL.len() - 1].1
}

/// Parse an SWR-meter reply payload (Icom cmd `0x15`): the sub-command byte
/// followed by the BCD reading. Returns the SWR ratio, or `None` if the reply
/// isn't the SWR meter (`0x12`) or is malformed.
pub fn parse_swr_reply(data: &[u8]) -> Option<f32> {
    if data.first() != Some(&0x12) {
        return None;
    }
    Some(swr_from_reading(decode_meter(&data[1..])?))
}

/// Parse an ALC-meter reply payload (Icom cmd `0x15`): the sub-command byte
/// followed by the BCD reading. Returns 0.0..=1.0, or `None` if the reply is not
/// the ALC meter (`0x13`) or is malformed.
///
/// ⚠️ **The scale is a straight 0..255 fraction, and that is a decision rather
/// than a calibration.** Icom publishes a curve for the S-meter and the standard
/// breakpoints for SWR, but nothing for ALC: the manual says only that the meter
/// should stay within the ALC zone. So this reports the raw reading as a
/// fraction of full scale and does not pretend to a precision it has not got.
/// Read it as "how far along its own meter the rig says it is", which is what
/// the operator is looking at on the rig's face anyway.
///
/// ✅ **CHECKED ON AIR 18 August 2026 against an IC-7300 and found good enough.**
/// Rodger's verdict, and it settles the open question this comment used to
/// carry: the percentage agrees with the rig's own meter and reads much as
/// WSJT-X 3.2.0's does beside its SWR figure. **It is a GUIDE and is meant to
/// be one** — his words, "if I really am looking for adjusting I would go to the
/// rig". ⛔ **So do not fit a calibration curve here.** There is no published
/// Icom curve to fit it to, a plausible-looking one would be invented precision,
/// and the raw fraction has now been shown to track the rig closely enough for
/// the job it does.
pub fn parse_alc_reply(data: &[u8]) -> Option<f32> {
    if data.first() != Some(&0x13) {
        return None;
    }
    Some((decode_meter(&data[1..])? as f32 / 255.0).clamp(0.0, 1.0))
}

/// Map an Icom power-output reading (`0..255`) to a `0.0..=1.0` fraction of
/// full scale, over the breakpoints Icom publish for the PO meter's face and
/// Hamlib carries for this family (0 → 0%, 143 → 50%, 213 → 100%).
///
/// The scale is not linear in the reading, which is why this is a table and not
/// a divide: half scale sits at 143 rather than at 128, and the meter reaches
/// full scale at 213 with the remaining readings above it pinned there.
///
/// The result is a NEEDLE POSITION, not a wattage, and the deliberate absence
/// of a watts conversion here is the point. Icom calibrate the face in percent
/// of the rig's own maximum, not in watts, and the relationship between the two
/// moves with band, mode, supply voltage and drive setting. Converting would
/// hand the operator a figure that looks measured and is not. See
/// [`sdroxide_types::TxTelemetry::po`].
///
/// ✅ **CHECKED ON AIR 18 August 2026 against an IC-7300.** Rodger's verdict:
/// "perfect, running as expected". The bar tracks the rig's own PO meter, so
/// the breakpoints below are confirmed against the thing they model rather
/// than only against their own tests.
///
/// ⛔ **So do not replace this table with a divide, and do not convert it to
/// watts.** The table is here because the scale is not linear in the reading:
/// half scale is at 143, not at 128. The absent watts conversion is the same
/// decision the ALC parser records, for the same reason — there is no published
/// Icom curve to fit, so a plausible-looking one would be invented precision.
/// A percentage that has been shown to agree with the rig's face is worth more
/// than a wattage that has not.
fn po_from_reading(reading: u32) -> f32 {
    const CAL: &[(f32, f32)] = &[(0.0, 0.0), (143.0, 0.5), (213.0, 1.0)];
    let r = reading as f32;
    if r <= CAL[0].0 {
        return CAL[0].1;
    }
    for w in CAL.windows(2) {
        let (x0, y0) = w[0];
        let (x1, y1) = w[1];
        if r <= x1 {
            return y0 + (y1 - y0) * (r - x0) / (x1 - x0);
        }
    }
    CAL[CAL.len() - 1].1
}

/// Parse a power-output reply payload (Icom cmd `0x15`): the sub-command byte
/// followed by the BCD reading. Returns the `0.0..=1.0` fraction, or `None` if
/// the reply isn't the PO meter (`0x11`) or is malformed.
///
/// The sub-command guard is load-bearing. SWR, ALC, PO and the S-meter are all
/// answered on command `0x15` and are told apart by nothing but this byte, so a
/// parser that skipped it would happily report an SWR reading as a power level.
pub fn parse_po_reply(data: &[u8]) -> Option<f32> {
    if data.first() != Some(&0x11) {
        return None;
    }
    Some(po_from_reading(decode_meter(&data[1..])?))
}

/// Map an Icom S-meter reading (`0..255`) to dBm, over the calibration Icom
/// states and Hamlib carries for this family: reading 0 is S0, 120 is S9, and
/// 241 is S9+60 dB. S9 is −73 dBm and an S-unit is 6 dB, so the three points are
/// −127, −73 and −13 dBm; between and beyond them the scale is interpolated and
/// clamped, as the SWR curve is.
///
/// This is the rig's *own* meter, in the units its manufacturer calibrated it
/// in — not a level derived from the sound card — so it needs no dBFS→dBm
/// offset applying on top.
fn dbm_from_smeter(reading: u32) -> f32 {
    const CAL: &[(f32, f32)] = &[(0.0, -127.0), (120.0, -73.0), (241.0, -13.0)];
    let r = reading as f32;
    if r <= CAL[0].0 {
        return CAL[0].1;
    }
    for w in CAL.windows(2) {
        let (x0, y0) = w[0];
        let (x1, y1) = w[1];
        if r <= x1 {
            return y0 + (y1 - y0) * (r - x0) / (x1 - x0);
        }
    }
    CAL[CAL.len() - 1].1
}

/// Parse an S-meter reply payload (Icom cmd `0x15`): the sub-command byte
/// followed by the BCD reading. Returns the signal level in dBm, or `None` if
/// the reply isn't the S-meter (`0x02`) or is malformed.
pub fn parse_smeter_reply(data: &[u8]) -> Option<f32> {
    if data.first() != Some(&0x02) {
        return None;
    }
    Some(dbm_from_smeter(decode_meter(&data[1..])?))
}

// ---------------------------------------------------------------------------
// Set-mode menu items
// ---------------------------------------------------------------------------

/// Set the receive filter width (`1A 03 <index>`), from audio-band edges in Hz.
///
/// Icom's filter is an index rather than a bandwidth, but unlike Yaesu's it is
/// a *formula* rather than a per-model table, and the same one on every rig
/// that has the command: on a sideband, in CW and in RTTY, indices 0..9 are
/// 50 Hz to 500 Hz in fifty-hertz steps and 10..40 are 600 Hz to 3600 Hz in
/// hundreds. AM has a scale of its own — 0..49 for 200 Hz to 10 kHz in
/// two-hundreds — which is why the mode comes in.
///
/// FM has no such setting at all (its filter follows the mode's own
/// wide/narrow), and neither has WFM, so both answer `None` rather than an
/// index that would land in some other scale entirely.
pub fn set_filter_frame(radio: u8, mode: Mode, lo_hz: f32, hi_hz: f32) -> Option<Vec<u8>> {
    // Sideband lives in the sign of the edges on this side; the rig's filter is
    // a width and knows nothing of that.
    let want = (hi_hz - lo_hz).abs().round().max(0.0) as u32;
    let index = match mode {
        // No filter-width setting: the mode itself picks the filter.
        Mode::Nfm | Mode::Wfm | Mode::Rifp | Mode::Packet | Mode::Aprs => return None,
        Mode::Am | Mode::Sam | Mode::Dsb => {
            // 200 Hz .. 10 kHz in 200 Hz steps, rounded up so the passband is
            // never quietly narrower than the one on screen.
            want.div_ceil(200).clamp(1, 50) - 1
        }
        _ => {
            if want <= 500 {
                // 50 Hz .. 500 Hz, index 0..9.
                want.div_ceil(50).clamp(1, 10) - 1
            } else {
                // 600 Hz .. 3600 Hz, index 10..40.
                want.div_ceil(100).clamp(6, 36) + 4
            }
        }
    };
    Some(frame(radio, 0x1A, &[0x03, to_bcd(index as u8)]))
}

/// A 0..99 value as the packed BCD byte Icom's level and index fields take.
fn to_bcd(v: u8) -> u8 {
    ((v / 10) << 4) | (v % 10)
}

/// Write a Set-mode menu item: `1A 05 <hi> <lo> <value…>`.
///
/// The item number is *not* portable between models — Icom renumbers this
/// block, and the IC-7300MK2 moved several relative to the original IC-7300.
/// Callers must take the number from the model at hand rather than from
/// another rig's manual: the numbers around the modulation-input entries
/// include the calibration marker, so a wrong write puts a signal on the
/// receiver that is not on the antenna.
pub fn set_menu_frame(radio: u8, item: u16, value: &[u8]) -> Vec<u8> {
    let mut data = vec![0x05, (item >> 8) as u8, item as u8];
    data.extend_from_slice(value);
    frame(radio, 0x1A, &data)
}

/// Read a Set-mode menu item back.
pub fn read_menu_frame(radio: u8, item: u16) -> Vec<u8> {
    frame(radio, 0x1A, &[0x05, (item >> 8) as u8, item as u8])
}

/// The item number and value in a `1A` reply — what the radio answers a
/// [`read_menu_frame`] with.
///
/// The length is what tells an answer from a question: a read carries
/// `05 <hi> <lo>` and nothing more, which is exactly what our own frame looks
/// like on its way out, and an answer appends the value to it. `None` for the
/// other `1A` sub-commands (`00`-`04` are the memory-keyer and band-edge
/// blocks) and for a reply too short to hold a value.
pub fn parse_menu_reply(data: &[u8]) -> Option<(u16, &[u8])> {
    if data.first() != Some(&0x05) || data.len() < 4 {
        return None;
    }
    Some(((u16::from(data[1]) << 8) | u16::from(data[2]), &data[3..]))
}

// ---------------------------------------------------------------------------
// Spectrum scope
// ---------------------------------------------------------------------------

/// Turn the radio's own spectrum scope on or off (`27 10`).
pub fn scope_on_frame(radio: u8, on: bool) -> Vec<u8> {
    frame(radio, 0x27, &[0x10, u8::from(on)])
}

/// Start or stop the scope sending its sweeps to us (`27 11`).
///
/// Separate from [`scope_on_frame`] on purpose: the scope can be running on the
/// radio's own display without streaming, and a client that only turns the
/// display on waits forever for data.
pub fn scope_output_frame(radio: u8, on: bool) -> Vec<u8> {
    frame(radio, 0x27, &[0x11, u8::from(on)])
}

/// The centre-mode spans an Icom scope offers, as the *half* widths its own
/// menu labels — "±2.5k" through "±500k" — in Hz.
///
/// One list for the whole family: the eight are identical in the IC-705's CI-V
/// reference guide and in the IC-7760's, two generations and two bin counts
/// apart, and a radio that does not offer a particular one answers `FA` and
/// keeps the span it had.
pub const SCOPE_SPANS_HZ: [f64; 8] =
    [2_500.0, 5_000.0, 10_000.0, 25_000.0, 50_000.0, 100_000.0, 250_000.0, 500_000.0];

/// Put the scope into one of its four layouts (`27 14`).
///
/// Centre mode is the one worth commanding: it follows the VFO, so the wide
/// waterfall stays under the dial instead of sitting on a fixed slice of band
/// the operator last chose on the radio.
pub fn scope_mode_frame(radio: u8, mode: ScopeMode) -> Vec<u8> {
    // The four-digit settings in this block are two bytes, not one. The first
    // is `00=MAIN, 01=SUB` on a radio that has two scopes and a fixed `00` on
    // one that does not, so the main scope is what `00` means either way.
    frame(radio, 0x27, &[0x14, 0x00, mode.to_byte()])
}

/// Set the centre-mode span (`27 15`), as the ± half width Icom's menu labels.
///
/// Only the eight values in [`SCOPE_SPANS_HZ`] exist; anything else is refused
/// by the radio, so the nearest one is sent instead of a value that would be
/// silently dropped.
pub fn scope_span_frame(radio: u8, half_span_hz: f64) -> Vec<u8> {
    let span = SCOPE_SPANS_HZ
        .iter()
        .copied()
        .min_by(|a, b| (a - half_span_hz).abs().total_cmp(&(b - half_span_hz).abs()))
        .unwrap_or(SCOPE_SPANS_HZ[0]);
    let mut data = vec![0x15, 0x00];
    data.extend_from_slice(&encode_freq(span));
    frame(radio, 0x27, &data)
}

/// How the radio is laying its scope out. Only the frequency reporting differs:
/// centred modes report a centre and a span, fixed modes report both edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeMode {
    Center,
    Fixed,
    ScrollCenter,
    ScrollFixed,
}

impl ScopeMode {
    fn from_byte(b: u8) -> Option<ScopeMode> {
        Some(match b {
            0x00 => ScopeMode::Center,
            0x01 => ScopeMode::Fixed,
            0x02 => ScopeMode::ScrollCenter,
            0x03 => ScopeMode::ScrollFixed,
            _ => return None,
        })
    }
    fn to_byte(self) -> u8 {
        match self {
            ScopeMode::Center => 0x00,
            ScopeMode::Fixed => 0x01,
            ScopeMode::ScrollCenter => 0x02,
            ScopeMode::ScrollFixed => 0x03,
        }
    }
    /// Whether this mode reports a centre and span rather than two edges.
    fn is_centred(self) -> bool {
        matches!(self, ScopeMode::Center | ScopeMode::ScrollCenter)
    }
}

/// What a sweep covers. Only the first division of a sweep carries it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScopeInfo {
    pub mode: ScopeMode,
    pub center_hz: f64,
    /// Full width, low edge to high edge.
    ///
    /// In the centred modes the radio reports a *half* span — its own menu
    /// labels these settings "±2.5k" through "±500k" — so the reported value is
    /// doubled here to give one meaning in all four modes.
    pub span_hz: f64,
    /// The radio is telling us the trace is off the top of its scale.
    pub out_of_range: bool,
}

/// One `27 00` frame.
///
/// Over LAN the whole sweep arrives at once and `divisions` is 1. Over USB the
/// radio splits it: division 1 carries the wave information and no bins, and
/// the rest carry bins only. Both forms parse here, because the split is a
/// property of the *transport*, and this parser has no idea which one it is
/// looking at until it reads the field.
#[derive(Debug, Clone, PartialEq)]
pub struct ScopeSweep {
    /// 1-based.
    pub division: u8,
    /// 1 over LAN; 11 or 15 over USB, depending on the model.
    pub divisions: u8,
    /// Present on the first division only.
    pub info: Option<ScopeInfo>,
    /// Amplitudes, on a scale the *model* sets — `0..=160` across the IC-7300
    /// generation, `0..=200` on an IC-7610 or IC-7760. Nothing on the wire says
    /// which, so the top of the scale comes from `IcomModel::scope_full_scale`
    /// rather than from here. Empty on the first division of a split sweep.
    pub bins: Vec<u8>,
}

impl ScopeSweep {
    /// The whole sweep arrived in this one frame.
    pub fn is_complete(&self) -> bool {
        self.divisions <= 1 && !self.bins.is_empty()
    }
}

/// The two-digit BCD counters the sweep uses for its division numbers. A
/// division of 11 is sent as `0x11`, not as 11.
fn from_bcd(b: u8) -> Option<u8> {
    let (hi, lo) = (b >> 4, b & 0x0f);
    (hi <= 9 && lo <= 9).then_some(hi * 10 + lo)
}

/// Bytes of wave information on the first division: scope mode, two 5-byte
/// frequencies, and the out-of-range flag.
const SCOPE_INFO_LEN: usize = 12;

/// Parse the payload of a `27 00` reply — `data` as [`CivReply`] delivers it,
/// beginning with the `0x00` sub-command byte.
pub fn parse_scope_frame(data: &[u8]) -> Option<ScopeSweep> {
    if data.first() != Some(&0x00) || data.len() < 4 {
        return None;
    }
    // data[1] is the selected/unselected VFO, which this backend does not use:
    // it drives one receiver and the radio only reports the selected one.
    let division = from_bcd(data[2])?;
    let divisions = from_bcd(data[3])?;
    if division == 0 || divisions == 0 || division > divisions {
        return None;
    }
    let rest = &data[4..];

    // Only the first division carries the wave information.
    if division != 1 {
        return Some(ScopeSweep { division, divisions, info: None, bins: rest.to_vec() });
    }
    if rest.len() < SCOPE_INFO_LEN {
        return None;
    }
    let mode = ScopeMode::from_byte(rest[0])?;
    let a = decode_freq(&rest[1..6])?;
    let b = decode_freq(&rest[6..11])?;
    let (center_hz, span_hz) =
        if mode.is_centred() { (a, b * 2.0) } else { ((a + b) / 2.0, (b - a).abs()) };
    Some(ScopeSweep {
        division,
        divisions,
        info: Some(ScopeInfo { mode, center_hz, span_hz, out_of_range: rest[11] != 0 }),
        bins: rest[SCOPE_INFO_LEN..].to_vec(),
    })
}

/// Reassembles a sweep that arrives in pieces.
///
/// Over LAN this is a pass-through — one frame is one sweep — but the USB path
/// exists on the same radios and a client that only handled the whole form
/// would show nothing at all there rather than showing something wrong.
#[derive(Debug, Default)]
pub struct ScopeAssembler {
    info: Option<ScopeInfo>,
    bins: Vec<u8>,
    expect: u8,
    next: u8,
}

impl ScopeAssembler {
    /// Feed one frame. Returns a finished sweep when the last division lands.
    pub fn push(&mut self, sweep: ScopeSweep) -> Option<(ScopeInfo, Vec<u8>)> {
        if sweep.division == 1 {
            self.info = sweep.info;
            self.bins.clear();
            self.expect = sweep.divisions;
            self.next = 2;
            self.bins.extend_from_slice(&sweep.bins);
            if sweep.divisions <= 1 {
                return self.finish();
            }
            return None;
        }
        // A continuation with no start, or out of order: the sweep is not
        // recoverable, and half a sweep drawn as a whole one is worse than a
        // dropped frame.
        if self.info.is_none() || sweep.division != self.next {
            self.info = None;
            self.bins.clear();
            return None;
        }
        self.bins.extend_from_slice(&sweep.bins);
        self.next += 1;
        if sweep.division >= self.expect { self.finish() } else { None }
    }

    fn finish(&mut self) -> Option<(ScopeInfo, Vec<u8>)> {
        let info = self.info.take()?;
        Some((info, std::mem::take(&mut self.bins)))
    }
}

/// A parsed reply from the rig (payload after `<cmd>`, addresses stripped).
#[derive(Debug, Clone, PartialEq)]
pub struct CivReply {
    pub from: u8,
    pub to: u8,
    pub cmd: u8,
    pub data: Vec<u8>,
}

/// Pull complete CI-V frames out of a rolling byte buffer, consuming them.
/// Tolerates junk between frames (CI-V is a shared bus with echoes).
pub fn parse_frames(buf: &mut Vec<u8>) -> Vec<CivReply> {
    let mut out = Vec::new();
    loop {
        // Find a preamble pair.
        let Some(start) = buf.windows(2).position(|w| w == [PREAMBLE, PREAMBLE]) else {
            // No frame start; keep at most the last byte (could be a lone FE).
            if buf.len() > 1 {
                let keep = buf.split_off(buf.len() - 1);
                buf.clear();
                buf.extend_from_slice(&keep);
            }
            break;
        };
        // Find the terminator after the preamble.
        let Some(rel_end) = buf[start + 2..].iter().position(|&b| b == END) else {
            // Incomplete frame — drop everything before `start` and wait.
            if start > 0 {
                buf.drain(..start);
            }
            break;
        };
        let end = start + 2 + rel_end;
        // Frame body is buf[start+2 ..= end]; need at least to,from,cmd.
        if end >= start + 5 {
            let to = buf[start + 2];
            let from = buf[start + 3];
            let cmd = buf[start + 4];
            let data = buf[start + 5..end].to_vec();
            out.push(CivReply { from, to, cmd, data });
        }
        buf.drain(..=end);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freq_roundtrips() {
        for hz in [14_074_000.0, 7_055_000.0, 28_500_000.0, 1_800_000.0, 145_500_000.0] {
            let b = encode_freq(hz);
            assert_eq!(decode_freq(&b), Some(hz), "freq {hz}");
        }
    }

    #[test]
    fn known_freq_bytes() {
        // 14.074000 MHz → little-endian BCD.
        assert_eq!(encode_freq(14_074_000.0), [0x00, 0x40, 0x07, 0x14, 0x00]);
    }

    #[test]
    fn set_freq_frame_shape() {
        let f = set_freq_frame(0x70, 14_074_000.0);
        assert_eq!(f, vec![0xFE, 0xFE, 0x70, 0xE0, 0x05, 0x00, 0x40, 0x07, 0x14, 0x00, 0xFD]);
    }

    #[test]
    fn parses_freq_reply_amid_echo() {
        // An echo of our own read request, then the rig's freq answer.
        let mut buf = Vec::new();
        buf.extend_from_slice(&read_freq_frame(0x70)); // echo (to=70,from=E0)
        buf.extend_from_slice(&frame(0x70, 0x03, &encode_freq(7_055_000.0))); // "reply"
        let frames = parse_frames(&mut buf);
        // Both parse; the freq one is cmd 0x03 with 5 data bytes.
        let freqs: Vec<f64> = frames
            .iter()
            .filter(|f| f.cmd == 0x03 && f.data.len() >= 5)
            .filter_map(|f| decode_freq(&f.data))
            .collect();
        assert_eq!(freqs, vec![7_055_000.0]);
        assert!(buf.is_empty());
    }

    /// On CI-V, USB and USB-DATA are the *same* mode byte. What separates them
    /// is a second command — and whether the rig has it, and which sub-command
    /// carries it, is a property of the model.
    #[test]
    fn data_mode_rides_beside_the_mode_rather_than_inside_it() {
        let mode = |m, sub| set_mode_frames(0x94, m, sub);
        // An IC-7300: USB for the mode, then the DATA switch on `1A 06`.
        assert_eq!(
            mode(Mode::Ft8, Some(0x06)),
            vec![
                vec![0xFE, 0xFE, 0x94, 0xE0, 0x06, 0x01, 0x01, 0xFD],
                vec![0xFE, 0xFE, 0x94, 0xE0, 0x1A, 0x06, 0x01, 0x01, 0xFD],
            ]
        );
        // Plain USB is the same mode byte with the switch explicitly *off* —
        // without that, a rig left in DATA by the last over would stay there.
        assert_eq!(
            mode(Mode::Usb, Some(0x06)),
            vec![
                vec![0xFE, 0xFE, 0x94, 0xE0, 0x06, 0x01, 0x01, 0xFD],
                vec![0xFE, 0xFE, 0x94, 0xE0, 0x1A, 0x06, 0x00, 0x01, 0xFD],
            ]
        );
        // The IC-7200 does the same job from sub-command 04.
        assert_eq!(mode(Mode::Ft8, Some(0x04))[1][5], 0x04);
        // An IC-7000 has no such command, and a Xiegu is not an Icom at all:
        // both get the mode and nothing else.
        assert_eq!(mode(Mode::Ft8, None).len(), 1);
        assert_eq!(mode(Mode::Ft8, None)[0], set_mode_frame(0x94, Mode::Ft8));

        // A carrier-centred mode is data over FM: the rig goes to FM *and* the
        // DATA switch goes on, which on an Icom is the mode's `-D` variant
        // rather than a sideband selector. FM-D takes its modulation from the
        // data source with the audio flat; plain FM puts the microphone path's
        // speech processing and pre-emphasis on it, and a Bell 202 tone pair
        // tilted 6 dB is unreadable to every modem on the channel.
        for m in [Mode::Rifp, Mode::Packet, Mode::Aprs] {
            assert_eq!(mode(m, Some(0x06))[0][5], 0x05, "{} is not FM", m.label());
            assert_eq!(mode(m, Some(0x06))[1][6], 0x01, "{} left DATA off", m.label());
        }
    }

    /// Icom's filter is an index, but unlike Yaesu's it comes from a formula
    /// that is the same on every rig with the command — so no per-model table
    /// is needed, only the mode, because AM has a scale of its own.
    #[test]
    fn the_filter_width_maps_onto_icoms_two_index_scales() {
        let idx = |mode: Mode, lo: f32, hi: f32| {
            set_filter_frame(0x94, mode, lo, hi).map(|f| f[f.len() - 2])
        };
        // Sideband/CW/RTTY: 0..9 is 50..500 Hz in fifties, 10..40 is 600..3600
        // in hundreds. The index goes on the wire as BCD.
        assert_eq!(idx(Mode::Cw, 0.0, 50.0), Some(0x00));
        assert_eq!(idx(Mode::Cw, 0.0, 500.0), Some(0x09));
        assert_eq!(idx(Mode::Usb, 0.0, 600.0), Some(0x10));
        assert_eq!(idx(Mode::Usb, 200.0, 2600.0), Some(0x28)); // 2400 Hz
        assert_eq!(idx(Mode::Usb, 0.0, 3600.0), Some(0x40));
        // Rounded up, so the passband is never quietly narrower than the one on
        // screen, and clamped rather than wrapped past the ends.
        assert_eq!(idx(Mode::Usb, 0.0, 2450.0), Some(0x29)); // 2500 Hz
        assert_eq!(idx(Mode::Usb, 0.0, 99_000.0), Some(0x40));
        assert_eq!(idx(Mode::Cw, 0.0, 1.0), Some(0x00));
        // LSB arrives with its edges negative; the width is the same.
        assert_eq!(idx(Mode::Lsb, -2600.0, -200.0), Some(0x28));
        // AM runs on its own scale: 200 Hz .. 10 kHz in two-hundreds.
        assert_eq!(idx(Mode::Am, 0.0, 6000.0), Some(0x29));
        assert_eq!(idx(Mode::Am, 0.0, 200.0), Some(0x00));
        // FM picks its filter with the mode; there is no width to set.
        assert_eq!(idx(Mode::Nfm, 0.0, 12_000.0), None);
        assert_eq!(idx(Mode::Wfm, 0.0, 200_000.0), None);
        // And the whole frame is the documented sub-command.
        assert_eq!(
            set_filter_frame(0x94, Mode::Usb, 200.0, 2600.0).unwrap(),
            vec![0xFE, 0xFE, 0x94, 0xE0, 0x1A, 0x03, 0x28, 0xFD]
        );
    }

    #[test]
    fn swr_meter_decodes_and_scales() {
        // Reply payload = sub-command 0x12 followed by the 2-byte BCD reading.
        // Calibration breakpoints map exactly: 0→1.0, 48→1.5, 80→2.0, 120→3.0.
        let swr = |reading_bcd: [u8; 2]| parse_swr_reply(&[0x12, reading_bcd[0], reading_bcd[1]]);
        assert_eq!(swr([0x00, 0x00]), Some(1.0)); // reading 0
        assert_eq!(swr([0x00, 0x48]), Some(1.5)); // reading 48
        assert_eq!(swr([0x00, 0x80]), Some(2.0)); // reading 80
        assert_eq!(swr([0x01, 0x20]), Some(3.0)); // reading 120
        // Midpoint of the 0..48 segment interpolates linearly to 1.25.
        assert_eq!(swr([0x00, 0x24]), Some(1.25)); // reading 24
        // A malformed reading (bad BCD nibble) yields None, not a bogus SWR.
        assert_eq!(swr([0x00, 0x0f]), None);
        // The wrong meter sub-command is ignored (we only read SWR / 0x12).
        assert_eq!(parse_swr_reply(&[0x11, 0x00, 0x50]), None);
        // The S-meter shares command 0x15 and must not be read as an SWR.
        assert_eq!(parse_swr_reply(&[0x02, 0x01, 0x20]), None);
    }

    #[test]
    fn the_alc_meter_is_read_and_parsed_on_its_own_sub_command() {
        // The frame asks for sub-command 0x13, not the SWR meter's 0x12. Getting
        // this wrong would return an SWR reading dressed as ALC, which is the
        // kind of error that looks plausible on screen.
        let f = read_alc_frame(0x94);
        assert_eq!(f[f.len() - 3..f.len() - 1], [0x15, 0x13], "wrong meter requested");

        // Raw 0..255 as a fraction of full scale: see the parser's note on why
        // there is no calibration curve here.
        let alc = |bcd: [u8; 2]| parse_alc_reply(&[0x13, bcd[0], bcd[1]]);
        assert_eq!(alc([0x00, 0x00]), Some(0.0));
        assert_eq!(alc([0x02, 0x55]), Some(1.0));
        assert!((alc([0x01, 0x28]).unwrap() - 0.5).abs() < 0.01, "midscale");

        // ⛔ The three TX/RX meters all answer on command 0x15 and are told
        // apart ONLY by the sub-command byte, so each parser must refuse the
        // others. Without this an ALC reply would fall through to the SWR
        // parser and be reported as an SWR ratio, which the guard acts on.
        assert_eq!(parse_alc_reply(&[0x12, 0x00, 0x48]), None, "took an SWR reply");
        assert_eq!(parse_alc_reply(&[0x02, 0x01, 0x20]), None, "took an S-meter reply");
        assert_eq!(parse_swr_reply(&[0x13, 0x01, 0x28]), None, "SWR parser took an ALC reply");
    }

    #[test]
    fn po_meter_decodes_and_scales() {
        // Reply payload = sub-command 0x11 followed by the 2-byte BCD reading.
        let po = |reading_bcd: [u8; 2]| parse_po_reply(&[0x11, reading_bcd[0], reading_bcd[1]]);
        // The calibration breakpoints map exactly.
        assert_eq!(po([0x00, 0x00]), Some(0.0)); // reading 0
        assert_eq!(po([0x01, 0x43]), Some(0.5)); // reading 143, half scale
        assert_eq!(po([0x02, 0x13]), Some(1.0)); // reading 213, full scale
        // Half scale sits at 143, not at 128, so the curve is not a divide:
        // reading 128 must come out BELOW 50%, which a linear /255 would miss.
        let mid = po([0x01, 0x28]).unwrap();
        assert!(mid < 0.5, "reading 128 should be under half scale, got {mid}");
        assert!((mid - 0.4476).abs() < 0.001, "interpolated 0..143 segment, got {mid}");
        // Readings above the top breakpoint pin at full scale rather than
        // running past it, as the SWR curve clamps at its own end.
        assert_eq!(po([0x02, 0x55]), Some(1.0)); // reading 255
        // A malformed reading (bad BCD nibble) yields None, not a bogus level.
        assert_eq!(po([0x00, 0x0f]), None);
        // Every other meter shares command 0x15 and must not be read as power.
        // Getting this wrong would put an SWR ratio on the PO bar, which is the
        // failure the sub-command guard exists to prevent.
        assert_eq!(parse_po_reply(&[0x12, 0x00, 0x50]), None); // SWR
        assert_eq!(parse_po_reply(&[0x13, 0x00, 0x50]), None); // ALC
        assert_eq!(parse_po_reply(&[0x02, 0x01, 0x20]), None); // S-meter
        // And the converse: a PO reply must not be parsed as any of them.
        assert_eq!(parse_swr_reply(&[0x11, 0x02, 0x13]), None);
        assert_eq!(parse_smeter_reply(&[0x11, 0x02, 0x13]), None);
    }

    #[test]
    fn po_meter_frame_asks_for_its_own_sub_command() {
        // Sub-command 0x11, not the SWR meter's 0x12: asking for the wrong one
        // returns a plausible number on the wrong scale.
        assert_eq!(read_po_frame(0x94), vec![0xFE, 0xFE, 0x94, 0xE0, 0x15, 0x11, 0xFD]);
    }

    #[test]
    fn s_meter_decodes_to_dbm() {
        let dbm =
            |reading_bcd: [u8; 2]| parse_smeter_reply(&[0x02, reading_bcd[0], reading_bcd[1]]);
        // The calibration points: S0, S9, S9+60.
        assert_eq!(dbm([0x00, 0x00]), Some(-127.0));
        assert_eq!(dbm([0x01, 0x20]), Some(-73.0));
        assert_eq!(dbm([0x02, 0x41]), Some(-13.0));
        // S9 is 120 and an S-unit is 6 dB, so half of S9's reading is S4.5 —
        // 27 dB below S9 on the interpolated scale.
        assert_eq!(dbm([0x00, 0x60]), Some(-100.0));
        // Past the top of the table the reading clamps rather than running on.
        assert_eq!(dbm([0x02, 0x55]), Some(-13.0));
        // A malformed reading is no reading, and the SWR sub-meter is not ours.
        assert_eq!(dbm([0x00, 0x0f]), None);
        assert_eq!(parse_smeter_reply(&[0x12, 0x00, 0x48]), None);
    }

    #[test]
    fn the_two_meter_reads_are_distinct_frames() {
        assert_eq!(read_smeter_frame(0x94), vec![0xFE, 0xFE, 0x94, 0xE0, 0x15, 0x02, 0xFD]);
        assert_eq!(read_swr_frame(0x94), vec![0xFE, 0xFE, 0x94, 0xE0, 0x15, 0x12, 0xFD]);

        // The PTT read is the key-down command with its value left off, and the
        // reply carries the sub-command that says which status it answers.
        assert_eq!(read_ptt_frame(0x94), vec![0xFE, 0xFE, 0x94, 0xE0, 0x1C, 0x00, 0xFD]);
        assert_eq!(parse_ptt_reply(&[0x00, 0x01]), Some(true));
        assert_eq!(parse_ptt_reply(&[0x00, 0x00]), Some(false));
        // The tuner rides the same command one sub-command along, and must not
        // be read as a transmitter.
        assert_eq!(parse_ptt_reply(&[0x01, 0x01]), None);
        assert_eq!(parse_ptt_reply(&[0x00]), None);
    }

    #[test]
    fn clearing_offsets_neutralises_rit_xit_and_split() {
        let f = clear_offsets_frames(0x70);
        let body = |i: usize| f[i][4..f[i].len() - 1].to_vec();
        assert_eq!(f.len(), 4);
        // Every frame is addressed to the rig from the controller.
        for frame in &f {
            assert_eq!(frame[..4], [0xFE, 0xFE, 0x70, 0xE0]);
            assert_eq!(*frame.last().unwrap(), 0xFD);
        }
        assert_eq!(body(0), vec![0x21, 0x00, 0x00, 0x00, 0x00], "RIT/ΔTX offset → 0 Hz, +");
        assert_eq!(body(1), vec![0x21, 0x01, 0x00], "RIT off");
        assert_eq!(body(2), vec![0x21, 0x02, 0x00], "ΔTX (XIT) off");
        assert_eq!(body(3), vec![0x0F, 0x00], "simplex");
    }

    /// The repeater shift the rig applies on its own has to be off, because
    /// sdroxide carries the shift on the dial — see [`simplex_frame`]. It is
    /// the *duplex* half of command `0x0F`, not the split half beside it in
    /// [`clear_offsets_frames`], and the two must not be confused: `0x0F 0x00`
    /// switches split off and leaves DUP− exactly where it was.
    #[test]
    fn the_duplex_shift_is_cleared_with_its_own_sub_command() {
        assert_eq!(simplex_frame(0x94), vec![0xFE, 0xFE, 0x94, 0xE0, 0x0F, 0x10, 0xFD]);
        // And is not one of the once-per-connection offset clears, which is
        // the whole point: a band stacking register puts it back.
        assert!(!clear_offsets_frames(0x94).contains(&simplex_frame(0x94)));
    }

    /// The radio's own power switch is command `0x18`, and a power-*on* has to
    /// wake the radio's control receiver first (issue #239).
    #[test]
    fn a_power_on_is_preceded_by_a_wake_up_run() {
        // Off: one frame, nothing in front of it. The radio is awake.
        assert_eq!(
            power_frames(0x94, false, 27),
            vec![vec![0xFE, 0xFE, 0x94, 0xE0, 0x18, 0x00, 0xFD]]
        );
        // On: the wake-up run, then the frame.
        let on = power_frames(0x94, true, 27);
        assert_eq!(on.len(), 2);
        assert_eq!(on[0], vec![0xFE; 27]);
        assert_eq!(on[1], vec![0xFE, 0xFE, 0x94, 0xE0, 0x18, 0x01, 0xFD]);
        // A link that needs no wake sends the frame alone.
        assert_eq!(
            power_frames(0x94, true, 0),
            vec![vec![0xFE, 0xFE, 0x94, 0xE0, 0x18, 0x01, 0xFD]]
        );
        // The run is a duration, so it scales with the line rate — Icom's own
        // table, which is Hamlib's too.
        assert_eq!(wake_bytes_for(4800), 7);
        assert_eq!(wake_bytes_for(9600), 13);
        assert_eq!(wake_bytes_for(19200), 27);
        assert_eq!(wake_bytes_for(38400), 53);
        assert_eq!(wake_bytes_for(57600), 79);
        assert_eq!(wake_bytes_for(115200), 159);
        // An off-table rate is worked out rather than refused, and never so
        // short that it could not wake anything.
        assert_eq!(wake_bytes_for(230400), 323);
        assert_eq!(wake_bytes_for(300), 7);
    }

    /// The antenna selector is command `0x12`: a socket number, and the
    /// receive-antenna byte behind it that sdroxide always leaves at zero
    /// (issue #238).
    #[test]
    fn the_antenna_selector_carries_a_socket_number() {
        assert_eq!(
            set_antenna_frame(0x94, "ANT1"),
            Some(vec![0xFE, 0xFE, 0x94, 0xE0, 0x12, 0x00, 0x00, 0xFD])
        );
        assert_eq!(
            set_antenna_frame(0x94, "ANT2"),
            Some(vec![0xFE, 0xFE, 0x94, 0xE0, 0x12, 0x01, 0x00, 0xFD])
        );
        // A name off another radio's list must never pick a socket by accident.
        assert_eq!(set_antenna_frame(0x94, "LNAW"), None);
        assert_eq!(set_antenna_frame(0x94, ""), None);
        // The read asks for the socket and, by being answered at all, for
        // whether there is a selector here.
        assert_eq!(read_antenna_frame(0x94), vec![0xFE, 0xFE, 0x94, 0xE0, 0x12, 0xFD]);
        // The reply is the socket, optionally with the receive-antenna
        // connector's own setting behind it, which is not ours to read.
        assert_eq!(parse_antenna_reply(&[0x00]), Some("ANT1"));
        assert_eq!(parse_antenna_reply(&[0x01, 0x00]), Some("ANT2"));
        assert_eq!(parse_antenna_reply(&[0x01, 0x01]), Some("ANT2"));
        // A four-socket rig left on ANT3 is a real port this end cannot name,
        // and calling it ANT1 would be a lie the operator could act on.
        assert_eq!(parse_antenna_reply(&[0x02]), None);
        assert_eq!(parse_antenna_reply(&[]), None);
    }

    /// Squelch is a level on command `0x14`, sub-command `0x03`, on the same
    /// 0–255 scale as the power beside it.
    #[test]
    fn squelch_is_a_fraction_of_the_rigs_own_scale() {
        let level = |frac: f32| {
            let f = set_squelch_frame(0x94, frac);
            assert_eq!(f[..4], [0xFE, 0xFE, 0x94, 0xE0], "addressed to the rig");
            assert_eq!(f[4..6], [0x14, 0x03], "squelch level");
            assert_eq!(*f.last().unwrap(), 0xFD);
            decode_meter(&f[6..f.len() - 1]).unwrap()
        };
        assert_eq!(level(0.0), 0, "fully open");
        assert_eq!(level(1.0), 255, "fully closed");
        assert_eq!(level(0.5), 128);
        // A rail cannot ask for more than the radio has.
        assert_eq!(level(2.0), 255);
        assert_eq!(level(-1.0), 0);
        // The read carries no payload, so it cannot be mistaken for a set.
        assert_eq!(read_squelch_frame(0x94), vec![0xFE, 0xFE, 0x94, 0xE0, 0x14, 0x03, 0xFD]);
    }

    /// Both levels are read on command `0x14`, so the sub-command byte in the
    /// reply is the only thing that says which one arrived. Each parser has to
    /// refuse the other's, or the opening pair of reads would set the drive
    /// slider from the squelch knob.
    #[test]
    fn the_two_levels_on_command_14_are_told_apart_by_their_sub_command() {
        assert_eq!(parse_squelch_reply(&[0x03, 0x00, 0x00]), Some(0.0));
        assert_eq!(parse_squelch_reply(&[0x03, 0x02, 0x55]), Some(1.0));
        // Round trip through the frame we would send.
        let f = set_squelch_frame(0x94, 0.75);
        let back = parse_squelch_reply(&f[5..f.len() - 1]).unwrap();
        assert!((back - 0.75).abs() < 0.005, "{back}");
        // Neither parser answers for the other's level, nor for a third.
        assert_eq!(parse_squelch_reply(&[0x0A, 0x02, 0x55]), None, "that is the power");
        assert_eq!(parse_power_reply(&[0x03, 0x02, 0x55]), None, "that is the squelch");
        assert_eq!(parse_squelch_reply(&[0x0C, 0x00, 0x85]), None, "that is the keyer speed");
        // A malformed BCD reading is nothing at all.
        assert_eq!(parse_squelch_reply(&[0x03, 0x00, 0x0f]), None);
    }

    #[test]
    fn cw_is_sent_as_text_the_rig_can_key() {
        // Command 0x17 with the message as plain upper-case ASCII.
        let f = send_cw_frame(0x94, "cq de w1aw").unwrap();
        assert_eq!(f[..5], [0xFE, 0xFE, 0x94, 0xE0, 0x17]);
        assert_eq!(&f[5..f.len() - 1], b"CQ DE W1AW");
        assert_eq!(*f.last().unwrap(), 0xFD);
        // Nothing sendable is no frame at all, not an empty one.
        assert!(send_cw_frame(0x94, "  <>  ").is_none());
        // A line break is a word break — dropping it would send one word.
        let wrapped = send_cw_frame(0x94, "tnx fer call\nur 599").unwrap();
        assert_eq!(&wrapped[5..wrapped.len() - 1], b"TNX FER CALL UR 599");
        // A message longer than one frame carries is cut to fit.
        let long = send_cw_frame(0x94, &"a".repeat(50)).unwrap();
        assert_eq!(long.len() - 6, CW_MAX);
        // Abort is the same command with the escape payload.
        assert_eq!(stop_cw_frame(0x94), vec![0xFE, 0xFE, 0x94, 0xE0, 0x17, 0xFF, 0xFD]);
    }

    /// Issue #282: a message handed to the keyer with break-in off is accepted
    /// and never transmitted, and over a network there is no `[TRANSMIT]` key
    /// to press instead.
    #[test]
    fn break_in_is_the_switch_a_keyed_message_needs() {
        // `16 47` with the position as the value.
        assert_eq!(
            set_break_in_frame(0x94, BreakIn::Semi),
            vec![0xFE, 0xFE, 0x94, 0xE0, 0x16, 0x47, 0x01, 0xFD]
        );
        assert_eq!(set_break_in_frame(0x94, BreakIn::Off)[6], 0x00);
        assert_eq!(set_break_in_frame(0x94, BreakIn::Full)[6], 0x02);
        // The read is the same frame with no value — which is why the parser
        // needs the length to tell our own question from the radio's answer.
        assert_eq!(read_break_in_frame(0x94), vec![0xFE, 0xFE, 0x94, 0xE0, 0x16, 0x47, 0xFD]);
        assert_eq!(parse_break_in_reply(&[0x47]), None, "that is the question, not the answer");
        assert_eq!(parse_break_in_reply(&[0x47, 0x00]), Some(BreakIn::Off));
        assert_eq!(parse_break_in_reply(&[0x47, 0x02]), Some(BreakIn::Full));
        // Any of the several dozen other `16` sub-commands is not this one.
        assert_eq!(parse_break_in_reply(&[0x46, 0x01]), None, "that is VOX");
        assert_eq!(parse_break_in_reply(&[0x47, 0x09]), None, "no such position");
        assert!(!BreakIn::Off.keys_the_transmitter());
        assert!(BreakIn::Semi.keys_the_transmitter() && BreakIn::Full.keys_the_transmitter());
    }

    /// Transmit power is a level on the same 0–255 scale as everything else in
    /// command 0x14, spanning whatever the rig can do — so the slider's
    /// fraction *is* the setting, and no wattage has to be known here.
    #[test]
    fn transmit_power_is_a_fraction_of_the_rigs_own_scale() {
        let level = |frac: f32| {
            let f = set_power_frame(0x94, frac);
            assert_eq!(f[..4], [0xFE, 0xFE, 0x94, 0xE0], "addressed to the rig");
            assert_eq!(f[4..6], [0x14, 0x0A], "RF power level");
            assert_eq!(*f.last().unwrap(), 0xFD);
            decode_meter(&f[6..f.len() - 1]).unwrap()
        };
        assert_eq!(level(0.0), 0);
        assert_eq!(level(0.5), 128);
        assert_eq!(level(1.0), 255);
        // A slider cannot ask for more than the radio has.
        assert_eq!(level(2.0), 255);
        assert_eq!(level(-1.0), 0);
        // The read carries no payload, so it cannot be mistaken for a set.
        assert_eq!(read_power_frame(0x94), vec![0xFE, 0xFE, 0x94, 0xE0, 0x14, 0x0A, 0xFD]);
    }

    #[test]
    fn the_power_the_rig_reports_comes_back_as_the_fraction_that_set_it() {
        // Reply payload = sub-command 0x0A followed by the 2-byte BCD level.
        assert_eq!(parse_power_reply(&[0x0A, 0x00, 0x00]), Some(0.0));
        assert_eq!(parse_power_reply(&[0x0A, 0x02, 0x55]), Some(1.0));
        // Round trip: what we would send for 30 % reads back as 30 %, to
        // within the rig's own 1/255 grid.
        let f = set_power_frame(0x94, 0.30);
        let back = parse_power_reply(&f[5..f.len() - 1]).unwrap();
        assert!((back - 0.30).abs() < 0.005, "{back}");
        // Another level on the same command is not the power (the keyer speed
        // shares it), and a malformed BCD reading is nothing at all.
        assert_eq!(parse_power_reply(&[0x0C, 0x00, 0x85]), None);
        assert_eq!(parse_power_reply(&[0x0A, 0x00, 0x0f]), None);
    }

    #[test]
    fn keyer_speed_maps_wpm_onto_the_rigs_level_scale() {
        let level = |wpm: f32| {
            let f = keyer_speed_frame(0x94, wpm);
            assert_eq!(f[4..6], [0x14, 0x0C]);
            decode_meter(&f[6..f.len() - 1]).unwrap()
        };
        // The ends of the rig's range, and a speed in the middle of it. The
        // reference table Hamlib carries has 20 WPM at 84.
        assert_eq!(level(6.0), 0);
        assert_eq!(level(48.0), 255);
        assert_eq!(level(20.0), 85);
        // Speeds the panel offers that the rig's keyer does not reach clamp
        // rather than wrapping round its 0..255 scale.
        assert_eq!(level(4.0), 0);
        assert_eq!(level(60.0), 255);
    }

    #[test]
    fn handles_partial_frame() {
        let full = set_freq_frame(0x70, 14_074_000.0);
        let (head, tail) = full.split_at(6);
        let mut buf = head.to_vec();
        assert!(parse_frames(&mut buf).is_empty()); // incomplete, buffered
        buf.extend_from_slice(tail);
        assert_eq!(parse_frames(&mut buf).len(), 1);
    }

    // -- scope and menu ----------------------------------------------------

    /// The whole-sweep form a LAN Icom sends: division 1 of 1, all 475 bins.
    fn lan_sweep(bins: usize) -> Vec<u8> {
        let mut d = vec![0x00, 0x00, 0x01, 0x01, 0x00];
        d.extend_from_slice(&encode_freq(14_100_000.0)); // centre
        d.extend_from_slice(&encode_freq(50_000.0)); // ±50 kHz
        d.push(0x00); // in range
        d.extend((0..bins).map(|i| (i % 161) as u8));
        d
    }

    #[test]
    fn a_lan_sweep_parses_whole() {
        let s = parse_scope_frame(&lan_sweep(475)).unwrap();
        assert_eq!(s.division, 1);
        assert_eq!(s.divisions, 1);
        assert!(s.is_complete());
        assert_eq!(s.bins.len(), 475);
        let i = s.info.unwrap();
        assert_eq!(i.mode, ScopeMode::Center);
        assert_eq!(i.center_hz, 14_100_000.0);
        // The radio reports a half span; the parser gives the full width.
        assert_eq!(i.span_hz, 100_000.0);
        assert!(!i.out_of_range);
    }

    #[test]
    fn a_fixed_mode_sweep_reports_edges_and_becomes_a_centre_and_width() {
        let mut d = vec![0x00, 0x00, 0x01, 0x01, 0x01];
        d.extend_from_slice(&encode_freq(14_000_000.0));
        d.extend_from_slice(&encode_freq(14_350_000.0));
        d.push(0x00);
        d.extend(std::iter::repeat_n(0x20u8, 475));
        let i = parse_scope_frame(&d).unwrap().info.unwrap();
        assert_eq!(i.mode, ScopeMode::Fixed);
        assert_eq!(i.center_hz, 14_175_000.0);
        assert_eq!(i.span_hz, 350_000.0);
    }

    #[test]
    fn the_out_of_range_flag_is_carried() {
        // sub, vfo, div, divmax, mode, 5 centre, 5 span, out-of-range.
        let mut d = lan_sweep(0);
        d[15] = 0x01;
        assert!(parse_scope_frame(&d).unwrap().info.unwrap().out_of_range);
    }

    #[test]
    fn division_counters_are_bcd_not_binary() {
        // "11 of 11" over USB is 0x11, which read as binary would be 17.
        let mut d = lan_sweep(0);
        d[2] = 0x01;
        d[3] = 0x11;
        let s = parse_scope_frame(&d).unwrap();
        assert_eq!(s.divisions, 11);
        assert!(!s.is_complete(), "a split sweep is not finished by its first frame");
    }

    #[test]
    fn a_malformed_sweep_is_rejected_rather_than_half_read() {
        // Not the 27 00 sub-command.
        assert!(parse_scope_frame(&[0x10, 0x01]).is_none());
        // Truncated before the wave information is complete.
        assert!(parse_scope_frame(&[0x00, 0x00, 0x01, 0x01, 0x00, 0x00]).is_none());
        // Division 0, and a division past the total.
        let mut d = lan_sweep(4);
        d[2] = 0x00;
        assert!(parse_scope_frame(&d).is_none());
        let mut d = lan_sweep(4);
        d[2] = 0x03;
        assert!(parse_scope_frame(&d).is_none());
    }

    #[test]
    fn the_assembler_passes_a_whole_sweep_straight_through() {
        let mut a = ScopeAssembler::default();
        let (info, bins) = a.push(parse_scope_frame(&lan_sweep(475)).unwrap()).unwrap();
        assert_eq!(bins.len(), 475);
        assert_eq!(info.center_hz, 14_100_000.0);
    }

    #[test]
    fn the_assembler_joins_a_split_sweep() {
        let mut a = ScopeAssembler::default();
        // Division 1 of 3: information only.
        let mut first = lan_sweep(0);
        first[3] = 0x03;
        assert!(a.push(parse_scope_frame(&first).unwrap()).is_none());
        // Divisions 2 and 3: bins only.
        let cont = |div: u8, fill: u8| {
            let mut d = vec![0x00, 0x00, div, 0x03];
            d.extend(std::iter::repeat_n(fill, 50));
            parse_scope_frame(&d).unwrap()
        };
        assert!(a.push(cont(0x02, 0x11)).is_none());
        let (_, bins) = a.push(cont(0x03, 0x22)).unwrap();
        assert_eq!(bins.len(), 100);
        assert_eq!(bins[0], 0x11);
        assert_eq!(bins[99], 0x22);
    }

    #[test]
    fn the_assembler_drops_a_sweep_it_cannot_trust() {
        let mut a = ScopeAssembler::default();
        let mut first = lan_sweep(0);
        first[3] = 0x03;
        a.push(parse_scope_frame(&first).unwrap());
        // Division 3 with 2 missing: half a sweep drawn as a whole one would
        // be a lie, so nothing comes out.
        let mut third = vec![0x00, 0x00, 0x03, 0x03];
        third.extend(std::iter::repeat_n(0x22u8, 50));
        assert!(a.push(parse_scope_frame(&third).unwrap()).is_none());
        // And it stays dropped rather than resyncing mid-sweep.
        let mut second = vec![0x00, 0x00, 0x02, 0x03];
        second.extend(std::iter::repeat_n(0x11u8, 50));
        assert!(a.push(parse_scope_frame(&second).unwrap()).is_none());
    }

    #[test]
    fn scope_control_frames_are_the_documented_commands() {
        assert_eq!(
            scope_on_frame(0xB6, true),
            vec![0xFE, 0xFE, 0xB6, 0xE0, 0x27, 0x10, 0x01, 0xFD]
        );
        assert_eq!(
            scope_output_frame(0xB6, true),
            vec![0xFE, 0xFE, 0xB6, 0xE0, 0x27, 0x11, 0x01, 0xFD]
        );
        assert_eq!(
            scope_output_frame(0xB6, false),
            vec![0xFE, 0xFE, 0xB6, 0xE0, 0x27, 0x11, 0x00, 0xFD]
        );
        // `27 14` and `27 15` are the two-byte form: a leading 00 for the main
        // scope, then the setting.
        assert_eq!(
            scope_mode_frame(0xA4, ScopeMode::Center),
            vec![0xFE, 0xFE, 0xA4, 0xE0, 0x27, 0x14, 0x00, 0x00, 0xFD]
        );
        // ±100 kHz, as five little-endian BCD bytes: 100000 Hz.
        assert_eq!(
            scope_span_frame(0xA4, 100_000.0),
            vec![0xFE, 0xFE, 0xA4, 0xE0, 0x27, 0x15, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0xFD]
        );
        // The widest the family offers, and the one the operator reaches for.
        assert_eq!(
            scope_span_frame(0xA4, 500_000.0),
            vec![0xFE, 0xFE, 0xA4, 0xE0, 0x27, 0x15, 0x00, 0x00, 0x00, 0x50, 0x00, 0x00, 0xFD]
        );
    }

    #[test]
    fn a_span_the_radio_does_not_have_becomes_the_nearest_one_it_does() {
        // Rather than a value the radio answers `FA` to and quietly ignores.
        assert_eq!(scope_span_frame(0xA4, 120_000.0), scope_span_frame(0xA4, 100_000.0));
        assert_eq!(scope_span_frame(0xA4, 1_000_000.0), scope_span_frame(0xA4, 500_000.0));
        assert_eq!(scope_span_frame(0xA4, 0.0), scope_span_frame(0xA4, 2_500.0));
    }

    #[test]
    fn a_menu_write_splits_the_item_into_two_bytes() {
        // IC-7300MK2 DATA OFF MOD = 1A 05 00 84, value 05 = LAN.
        assert_eq!(
            set_menu_frame(0xB6, 0x0084, &[0x05]),
            vec![0xFE, 0xFE, 0xB6, 0xE0, 0x1A, 0x05, 0x00, 0x84, 0x05, 0xFD]
        );
        assert_eq!(
            read_menu_frame(0xB6, 0x0079),
            vec![0xFE, 0xFE, 0xB6, 0xE0, 0x1A, 0x05, 0x00, 0x79, 0xFD]
        );
    }

    #[test]
    fn a_menu_answer_is_told_from_the_question_by_its_length() {
        // The answer to "what is in 0084?" — MIC, on an IC-7300MK2.
        assert_eq!(parse_menu_reply(&[0x05, 0x00, 0x84, 0x00]), Some((0x0084, &[0x00][..])));
        // The question itself, which is what our own frame looks like coming
        // back off the bus: no value, so nothing to record.
        assert_eq!(parse_menu_reply(&[0x05, 0x00, 0x84]), None);
        // And another `1A` sub-command is not a menu item at all.
        assert_eq!(parse_menu_reply(&[0x06, 0x01, 0x01, 0x00]), None);
    }
}
