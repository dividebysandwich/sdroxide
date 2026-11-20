//! Pure text translation for the server: inbound client commands become
//! [`ClientReq`]s, and a changed [`TciStateSnapshot`] becomes the status lines
//! to broadcast. No I/O and no shared state, so every rule here is unit-testable.

use sdroxide_types::{Command, RxId, Vfo};

use super::TciStateSnapshot;
use crate::protocol as p;

/// One decoded request from a client.
// See the note on `ServerRequest`: `Command` travels unboxed everywhere else.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ClientReq {
    /// Translate straight into `Engine::apply`.
    Cmd(Command),
    /// `trx:<rx>,<bool>[,tci]` — key or unkey with the client as audio source.
    Key(bool),
    /// `iq_start` / `iq_stop`.
    Iq(bool),
    /// `iq_samplerate:<hz>`.
    IqRate(u32),
    /// `audio_start` / `audio_stop`.
    Audio(bool),
    /// `tx_sensors_enable:<bool>`.
    TxSensors(bool),
    /// Understood but deliberately not acted on. The caller re-broadcasts the
    /// current truth so the client isn't left believing it took effect.
    Ignored,
    /// `spot:<call>,<mode>,<hz>,<argb>,<text>` — put a label on the panorama.
    Spot(TciSpot),
    /// `spot_delete:<call>` — take that callsign's label down.
    SpotDelete(String),
    /// `spot_clear` — take every label down.
    SpotClear,
}

/// One `SPOT:` as a client sent it, checked and normalised.
///
/// TCI Protocol 2.1 §4.3: `SPOT:arg1,arg2,arg3,arg4,arg5;` — callsign, mode,
/// frequency in Hz, ARGB colour, additional text. "The callsign, the frequency
/// and the colour are mandatory, otherwise the command is ignored."
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TciSpot {
    /// Upper case, so `SPOT_DELETE` finds it however either was typed — the
    /// spec's own `CLICKED_ON_SPOT` reports the callsign in upper case.
    pub call: String,
    /// Upper case, as the network spots carry it; may be empty.
    pub mode: String,
    pub freq_hz: f64,
    /// RGB, or `None` for the kind's tint — see [`spot_color`].
    pub color: Option<[u8; 3]>,
    /// May be empty.
    pub text: String,
}

/// Receiver index we serve. sdroxide has a single tunable VFO pair, so we
/// advertise one transceiver with two channels (A and B) — see the module doc.
const RX: u32 = 0;

fn flag(s: &str) -> Option<bool> {
    match s.trim().to_lowercase().as_str() {
        "true" | "1" | "on" => Some(true),
        "false" | "0" | "off" => Some(false),
        _ => None,
    }
}

fn vfo_of(channel: &str) -> Option<Vfo> {
    match channel.trim() {
        "0" => Some(Vfo::A),
        "1" => Some(Vfo::B),
        _ => None,
    }
}

/// Split `args` on commas, trimmed.
fn parts(args: &str) -> Vec<&str> {
    args.split(',').map(str::trim).collect()
}

/// A percentage argument, accepting both the TCI ≥ 1.5 `<trx>,<pct>` form and
/// the bare `<pct>` older servers and clients still use.
fn pct(args: &[&str]) -> Option<f32> {
    let raw: f32 = match args {
        [v] => v.parse().ok()?,
        [_trx, v, ..] => v.parse().ok()?,
        _ => return None,
    };
    Some((raw / 100.0).clamp(0.0, 1.0))
}

/// Likewise for a boolean that may or may not be prefixed by a receiver index.
fn indexed_flag(args: &[&str]) -> Option<bool> {
    match args {
        [v] => flag(v),
        [_rx, v, ..] => flag(v),
        _ => None,
    }
}

/// `SPOT:`'s colour argument: `None` when it is missing, which the spec makes
/// a reason to ignore the whole spot; `Some(None)` when it is there but gives
/// no usable colour, so the label wears the kind's tint; `Some(Some(rgb))`
/// otherwise.
///
/// The alpha byte is not used. The specification's own example,
/// `SPOT:RN6LHF,CW,7100000,16711680,ANY_TEXT;`, sends 16711680 = 0x00FF0000:
/// red with alpha 0. Read as alpha, that example would be an invisible label,
/// so clients that copy it send alpha 0 meaning "opaque", and honouring alpha
/// would hide their spots. Black (RGB 0) is taken as no colour: it is what a
/// client with nothing to choose sends, and a black callsign on the dark label
/// box could not be read.
///
/// The value is decimal, as in the example. A negative one is accepted too:
/// a 32-bit ARGB held in a signed integer — what C#'s `Color.ToArgb()` and
/// Qt's `QColor::rgba()` stored in an `int` give — prints negative whenever
/// alpha is 0x80 or more, so an opaque colour from such a client arrives as
/// e.g. `-65536` (0xFFFF0000, red).
fn spot_color(arg: Option<&&str>) -> Option<Option<[u8; 3]>> {
    let raw = arg.filter(|a| !a.is_empty())?;
    let Ok(v) = raw.parse::<i64>() else { return Some(None) };
    if !(i64::from(i32::MIN)..=i64::from(u32::MAX)).contains(&v) {
        return Some(None);
    }
    let argb = v as u32; // two's complement keeps the low 32 bits
    let rgb = [(argb >> 16) as u8, (argb >> 8) as u8, argb as u8];
    Some((rgb != [0, 0, 0]).then_some(rgb))
}

/// Parse `SPOT:`'s arguments; `None` when a mandatory one is missing or the
/// frequency is not a positive number of hertz.
///
/// The text is everything after the fourth comma. The spec reserves `,` and
/// does not allow it inside an argument, but a client that puts one in its
/// comment anyway gets the comment back whole rather than cut at the comma.
fn parse_spot(args: &str) -> Option<TciSpot> {
    // Split at most four times, so commas past the fourth stay in the text.
    let a: Vec<&str> = args.splitn(5, ',').map(str::trim).collect();
    let call = a.first().filter(|c| !c.is_empty())?.to_uppercase();
    let freq_hz: f64 = a.get(2)?.parse().ok().filter(|f: &f64| f.is_finite() && *f > 0.0)?;
    let color = spot_color(a.get(3))?;
    Some(TciSpot {
        call,
        mode: a.get(1).map(|m| m.to_uppercase()).unwrap_or_default(),
        freq_hz,
        color,
        text: a.get(4).map(|t| t.to_string()).unwrap_or_default(),
    })
}

/// Translate one `(command, args)` pair from [`crate::protocol::parse_status`].
/// `state` supplies the context an absolute translation needs (the IQ centre
/// for `if:`, the active mode for nothing yet). Unknown commands return `None`
/// so the caller can stay silent rather than guess.
pub(crate) fn translate(cmd: &str, args: &str, state: &TciStateSnapshot) -> Option<ClientReq> {
    let a = parts(args);
    Some(match cmd {
        // --- Tuning -------------------------------------------------------
        "vfo" => match a.as_slice() {
            [_rx, ch, hz, ..] => {
                ClientReq::Cmd(Command::SetVfo { vfo: vfo_of(ch)?, hz: hz.parse().ok()? })
            }
            _ => return None,
        },
        // The IQ stream centre is our hardware centre frequency.
        "dds" => match a.as_slice() {
            [_rx, hz, ..] => ClientReq::Cmd(Command::SetCenter(hz.parse().ok()?)),
            _ => return None,
        },
        // An offset from the IQ centre; we tune the VFO to the absolute result.
        "if" => match a.as_slice() {
            [_rx, ch, hz, ..] => {
                let off: f64 = hz.parse().ok()?;
                ClientReq::Cmd(Command::SetVfo { vfo: vfo_of(ch)?, hz: state.center_hz + off })
            }
            _ => return None,
        },
        "modulation" => match a.as_slice() {
            [_rx, m, ..] => {
                ClientReq::Cmd(Command::SetMode { rx: RxId::Main, mode: p::tci_to_mode(m)? })
            }
            _ => return None,
        },
        "split_enable" => ClientReq::Cmd(Command::SetSplit(indexed_flag(&a)?)),

        // --- Transmit -----------------------------------------------------
        // `trx:<rx>,true,tci` keys with the client's audio stream as the
        // source. Without the `tci` suffix a client is asking us to key from
        // our own audio source, which is the operator's microphone — treat it
        // the same, since the client is still the one deciding to transmit.
        "trx" => match a.as_slice() {
            [_rx, on, ..] => ClientReq::Key(flag(on)?),
            _ => return None,
        },
        "tune" => ClientReq::Cmd(Command::SetTune(indexed_flag(&a)?)),
        "drive" => ClientReq::Cmd(Command::SetTxDrive(pct(&a)?)),
        "tune_drive" => ClientReq::Cmd(Command::SetTuneDrive(pct(&a)?)),
        // `<enable>[,<period ms>]` — the second field is a reporting interval,
        // not a receiver index, so this one is not `indexed_flag`.
        "tx_sensors_enable" => ClientReq::TxSensors(flag(a.first()?)?),
        // We have no per-receiver S-meter reporting; accept it so the client
        // gets its acknowledgement rather than a silence it may time out on.
        "rx_sensors_enable" => ClientReq::Ignored,

        // --- Audio --------------------------------------------------------
        "mute" => ClientReq::Cmd(Command::SetMute { rx: RxId::Main, muted: indexed_flag(&a)? }),
        // TCI's volume is a master AF level in dB; sdroxide's is a 0..1 factor.
        "volume" => {
            let db: f32 = a.first()?.parse().ok()?;
            ClientReq::Cmd(Command::SetVolume { rx: RxId::Main, v: db_to_volume(db) })
        }

        // --- Streams ------------------------------------------------------
        // Only receiver 0 has streams; a request for any other is ignored.
        "iq_start" | "iq_stop" => match a.first().and_then(|r| r.parse::<u32>().ok()) {
            Some(RX) | None => ClientReq::Iq(cmd == "iq_start"),
            _ => ClientReq::Ignored,
        },
        "audio_start" | "audio_stop" => match a.first().and_then(|r| r.parse::<u32>().ok()) {
            Some(RX) | None => ClientReq::Audio(cmd == "audio_start"),
            _ => ClientReq::Ignored,
        },
        "iq_samplerate" => ClientReq::IqRate(a.first()?.parse().ok()?),
        // Our RX audio is always 48 kHz; the echo tells the client the truth.
        "audio_samplerate" => ClientReq::Ignored,
        // Disabling receiver 0 would silence the operator's own radio.
        "rx_enable" => ClientReq::Ignored,
        // Global stream gate; our streams follow the per-stream commands above.
        "start" | "stop" => ClientReq::Ignored,

        // --- Spots --------------------------------------------------------
        "spot" => ClientReq::Spot(parse_spot(args)?),
        "spot_delete" => match a.first() {
            Some(c) if !c.is_empty() => ClientReq::SpotDelete(c.to_uppercase()),
            _ => return None,
        },
        "spot_clear" => ClientReq::SpotClear,

        _ => return None,
    })
}

/// TCI master volume is −60..0 dB; sdroxide's is a 0..1 amplitude factor that
/// the engine squares. −60 dB is TCI's floor and maps to silence.
pub(crate) fn db_to_volume(db: f32) -> f32 {
    if db <= -60.0 { 0.0 } else { 10f32.powf(db.min(0.0) / 20.0).clamp(0.0, 1.0) }
}

/// The inverse, for the outbound broadcast.
pub(crate) fn volume_to_db(v: f32) -> i32 {
    if v <= 0.0 { -60 } else { (20.0 * v.clamp(0.0, 1.0).log10()).round().clamp(-60.0, 0.0) as i32 }
}

/// Status lines describing `next`. With `prev` given, only the lines whose
/// underlying value actually changed — so an unchanged tick sends nothing and a
/// client dragging the waterfall can't make us flood the wire. With `prev`
/// `None`, every line, which is the state part of the connect burst.
pub(crate) fn diff_lines(prev: Option<&TciStateSnapshot>, next: &TciStateSnapshot) -> Vec<String> {
    let mut out = Vec::new();
    let changed = |f: fn(&TciStateSnapshot) -> bool| prev.is_none_or(|p| f(p) != f(next));

    if prev.is_none_or(|p| p.center_hz != next.center_hz) {
        out.push(p::dds(RX, next.center_hz));
    }
    if prev.is_none_or(|p| p.vfo_a_hz != next.vfo_a_hz) {
        out.push(p::vfo(RX, 0, next.vfo_a_hz));
        out.push(p::if_offset(RX, 0, next.vfo_a_hz - next.center_hz));
    }
    if prev.is_none_or(|p| p.vfo_b_hz != next.vfo_b_hz) {
        out.push(p::vfo(RX, 1, next.vfo_b_hz));
        out.push(p::if_offset(RX, 1, next.vfo_b_hz - next.center_hz));
    }
    if prev.is_none_or(|p| p.mode != next.mode) {
        out.push(p::modulation(RX, p::mode_to_tci(next.mode)));
    }
    if changed(|s| s.split) {
        out.push(p::split_enable(RX, next.split));
    }
    if changed(|s| s.ptt) {
        out.push(p::trx(RX, next.ptt, false));
    }
    if changed(|s| s.tune) {
        out.push(p::tune(RX, next.tune));
    }
    if prev.is_none_or(|p| p.drive_pct != next.drive_pct) {
        out.push(p::drive(RX, next.drive_pct));
    }
    if prev.is_none_or(|p| p.tune_drive_pct != next.tune_drive_pct) {
        out.push(p::tune_drive(RX, next.tune_drive_pct));
    }
    if changed(|s| s.muted) {
        out.push(p::mute(RX, next.muted));
    }
    if prev.is_none_or(|p| p.volume_db != next.volume_db) {
        out.push(p::volume(next.volume_db));
    }
    // The rate we actually achieved, which is a Ddc-snapped divisor of the
    // device rate and so rarely the round number the client asked for.
    if prev.is_none_or(|p| p.iq_rate != next.iq_rate) && next.iq_rate > 0 {
        out.push(p::iq_samplerate(next.iq_rate));
    }
    out
}

/// The full connect burst: identity, limits, then the state lines.
pub(crate) fn connect_burst(device_name: &str, state: &TciStateSnapshot) -> Vec<String> {
    let mut out = vec![
        p::protocol_line("sdroxide", super::TCI_VERSION),
        p::device(device_name),
        p::receive_only(!state.can_tx),
        p::trx_count(1),
        p::channels_count(2),
        p::vfo_limits(state.vfo_lo_hz, state.vfo_hi_hz),
        p::if_limits(-state.if_span_hz, state.if_span_hz),
        p::modulations_list(&p::MODULATIONS),
    ];
    // An audio-mode source (a CAT rig on a sound card) has no wideband IQ, so
    // the IQ stream is not advertised at all rather than advertised and silent.
    if state.iq_rate > 0 {
        out.push(p::iq_samplerate(state.iq_rate));
    }
    out.push(p::audio_samplerate(super::AUDIO_RATE_HZ));
    out.push(p::rx_enable(RX, true));
    out.extend(diff_lines(None, state));
    out.push(p::start().to_string());
    out.push(p::ready().to_string());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_types::Mode;

    fn snap() -> TciStateSnapshot {
        TciStateSnapshot {
            center_hz: 14_100_000.0,
            vfo_a_hz: 14_074_000.0,
            vfo_b_hz: 14_080_000.0,
            mode: Mode::Usb,
            ..TciStateSnapshot::default()
        }
    }

    fn tr(line: &str) -> Option<ClientReq> {
        let s = snap();
        let parsed = p::parse_status(line);
        let (c, a) = parsed.first()?;
        translate(c, a, &s)
    }

    #[test]
    fn tuning_commands() {
        assert_eq!(
            tr("vfo:0,0,7100000;"),
            Some(ClientReq::Cmd(Command::SetVfo { vfo: Vfo::A, hz: 7_100_000.0 }))
        );
        assert_eq!(
            tr("vfo:0,1,7105000;"),
            Some(ClientReq::Cmd(Command::SetVfo { vfo: Vfo::B, hz: 7_105_000.0 }))
        );
        assert_eq!(tr("dds:0,14100000;"), Some(ClientReq::Cmd(Command::SetCenter(14_100_000.0))));
        // `if` is relative to the IQ centre; we tune to the absolute result.
        assert_eq!(
            tr("if:0,0,-26000;"),
            Some(ClientReq::Cmd(Command::SetVfo { vfo: Vfo::A, hz: 14_074_000.0 }))
        );
        assert_eq!(
            tr("modulation:0,digu;"),
            Some(ClientReq::Cmd(Command::SetMode { rx: RxId::Main, mode: Mode::Digu }))
        );
        assert_eq!(tr("split_enable:0,true;"), Some(ClientReq::Cmd(Command::SetSplit(true))));
    }

    #[test]
    fn transmit_commands() {
        assert_eq!(tr("trx:0,true,tci;"), Some(ClientReq::Key(true)));
        assert_eq!(tr("trx:0,true;"), Some(ClientReq::Key(true)));
        assert_eq!(tr("trx:0,false;"), Some(ClientReq::Key(false)));
        assert_eq!(tr("tune:0,true;"), Some(ClientReq::Cmd(Command::SetTune(true))));
        // Both the indexed (TCI >= 1.5) and bare percentage forms.
        assert_eq!(tr("drive:0,40;"), Some(ClientReq::Cmd(Command::SetTxDrive(0.4))));
        assert_eq!(tr("drive:40;"), Some(ClientReq::Cmd(Command::SetTxDrive(0.4))));
        assert_eq!(tr("tune_drive:0,10;"), Some(ClientReq::Cmd(Command::SetTuneDrive(0.1))));
        assert_eq!(tr("tx_sensors_enable:true;"), Some(ClientReq::TxSensors(true)));
    }

    #[test]
    fn stream_commands() {
        assert_eq!(tr("iq_start:0;"), Some(ClientReq::Iq(true)));
        assert_eq!(tr("iq_stop:0;"), Some(ClientReq::Iq(false)));
        assert_eq!(tr("iq_samplerate:192000;"), Some(ClientReq::IqRate(192_000)));
        assert_eq!(tr("audio_start:0;"), Some(ClientReq::Audio(true)));
        assert_eq!(tr("audio_stop:0;"), Some(ClientReq::Audio(false)));
        // Only receiver 0 carries streams.
        assert_eq!(tr("iq_start:1;"), Some(ClientReq::Ignored));
        assert_eq!(tr("audio_start:1;"), Some(ClientReq::Ignored));
        // Disabling our own receiver is refused, not obeyed.
        assert_eq!(tr("rx_enable:0,false;"), Some(ClientReq::Ignored));
    }

    #[test]
    fn junk_is_rejected() {
        assert_eq!(tr("bogus:1,2,3;"), None);
        assert_eq!(tr("vfo:0;"), None); // truncated
        assert_eq!(tr("vfo:0,7,7100000;"), None); // no such channel
        assert_eq!(tr("modulation:0,zzz;"), None); // no such mode
        assert_eq!(tr("trx:0,maybe;"), None);
        assert_eq!(tr("drive:0,abc;"), None);
        assert!(p::parse_status("").is_empty());
    }

    fn spot(line: &str) -> Option<TciSpot> {
        match tr(line)? {
            ClientReq::Spot(s) => Some(s),
            other => panic!("{line} parsed as {other:?}"),
        }
    }

    #[test]
    fn the_spec_example_spot_parses() {
        // TCI Protocol 2.1, `SPOT`, word for word.
        assert_eq!(
            spot("SPOT:RN6LHF,CW,7100000,16711680,ANY_TEXT;"),
            Some(TciSpot {
                call: "RN6LHF".into(),
                mode: "CW".into(),
                freq_hz: 7_100_000.0,
                // 0x00FF0000: red, although its alpha byte is 0.
                color: Some([255, 0, 0]),
                text: "ANY_TEXT".into(),
            })
        );
        // Case of the command and of the call does not matter.
        let s = spot("spot:rn6lhf,cw,7100000,16711680,any text;").unwrap();
        assert_eq!((s.call.as_str(), s.mode.as_str()), ("RN6LHF", "CW"));
        assert_eq!(s.text, "any text", "the text keeps the case it was sent in");
    }

    #[test]
    fn spot_colours() {
        let col = |c: &str| spot(&format!("spot:DL1ABC,SSB,14200000,{c},x;")).unwrap().color;
        assert_eq!(col("4294901760"), Some([255, 0, 0]), "0xFFFF0000 opaque red");
        assert_eq!(col("-65536"), Some([255, 0, 0]), "the same red from a signed int");
        assert_eq!(col("65280"), Some([0, 255, 0]));
        // Present but unusable: the label wears the kind's tint.
        assert_eq!(col("0"), None, "black is no colour");
        assert_eq!(col("-16777216"), None, "opaque black likewise");
        assert_eq!(col("red"), None);
        assert_eq!(col("#FF0000"), None);
        assert_eq!(col("4294967296"), None, "wider than 32 bits");
    }

    #[test]
    fn spot_text_and_optional_fields() {
        // Mode and text may be empty, and the text may be left off.
        let s = spot("spot:DL1ABC,,14200000,65280;").unwrap();
        assert_eq!((s.mode.as_str(), s.text.as_str()), ("", ""));
        let s = spot("spot:DL1ABC,FT8,14074000,65280,;").unwrap();
        assert_eq!(s.text, "");
        // A comma in the text is outside the spec, but the comment comes back
        // whole rather than cut.
        let s = spot("spot:DL1ABC,FT8,14074000,65280,new one, CQ DX;").unwrap();
        assert_eq!(s.text, "new one, CQ DX");
        // A fractional frequency is still hertz.
        assert_eq!(spot("spot:DL1ABC,CW,7012345.5,65280;").unwrap().freq_hz, 7_012_345.5);
    }

    #[test]
    fn spots_missing_a_mandatory_field_are_ignored() {
        assert_eq!(tr("spot:;"), None);
        assert_eq!(tr("spot;"), None);
        assert_eq!(tr("spot:,CW,7100000,16711680,x;"), None, "no callsign");
        assert_eq!(tr("spot:RN6LHF,CW,,16711680,x;"), None, "no frequency");
        assert_eq!(tr("spot:RN6LHF,CW;"), None, "no frequency");
        assert_eq!(tr("spot:RN6LHF,CW,7100000;"), None, "no colour");
        assert_eq!(tr("spot:RN6LHF,CW,7100000,,x;"), None, "empty colour");
        // A frequency that is not a positive number of hertz.
        assert_eq!(tr("spot:RN6LHF,CW,7.1MHz,16711680;"), None);
        assert_eq!(tr("spot:RN6LHF,CW,-7100000,16711680;"), None);
        assert_eq!(tr("spot:RN6LHF,CW,0,16711680;"), None);
        assert_eq!(tr("spot:RN6LHF,CW,NaN,16711680;"), None);
        assert_eq!(tr("spot:RN6LHF,CW,inf,16711680;"), None);
    }

    #[test]
    fn spot_delete_and_clear() {
        assert_eq!(tr("SPOT_DELETE:RN6LHF;"), Some(ClientReq::SpotDelete("RN6LHF".into())));
        assert_eq!(tr("spot_delete: rn6lhf ;"), Some(ClientReq::SpotDelete("RN6LHF".into())));
        assert_eq!(tr("spot_delete:;"), None);
        assert_eq!(tr("spot_delete;"), None);
        assert_eq!(tr("SPOT_CLEAR;"), Some(ClientReq::SpotClear));
    }

    #[test]
    fn volume_roundtrip() {
        assert_eq!(volume_to_db(1.0), 0);
        assert_eq!(volume_to_db(0.0), -60);
        assert!((db_to_volume(0.0) - 1.0).abs() < 1e-6);
        assert_eq!(db_to_volume(-60.0), 0.0);
        // -20 dB is a factor of 0.1, and back again.
        assert!((db_to_volume(-20.0) - 0.1).abs() < 1e-6);
        assert_eq!(volume_to_db(0.1), -20);
    }

    #[test]
    fn diff_emits_only_changes() {
        let a = snap();
        // An unchanged tick must produce nothing, or a client that never stops
        // asking would flood the wire.
        assert!(diff_lines(Some(&a), &a).is_empty());

        let mut b = a.clone();
        b.vfo_a_hz = 14_075_000.0;
        let lines = diff_lines(Some(&a), &b);
        assert_eq!(lines, vec!["vfo:0,0,14075000;", "if:0,0,-25000;"]);

        let mut c = a.clone();
        c.ptt = true;
        assert_eq!(diff_lines(Some(&a), &c), vec!["trx:0,true;"]);

        // No previous state = the whole picture.
        assert!(diff_lines(None, &a).len() > 5);
    }

    #[test]
    fn burst_ends_with_ready() {
        let s = snap();
        let burst = connect_burst("sdroxide", &s);
        assert_eq!(burst.last().map(String::as_str), Some("ready;"));
        assert!(burst.iter().any(|l| l == "device:sdroxide;"));
        assert!(burst.iter().any(|l| l.starts_with("modulations_list:")));
        // No wideband IQ on this snapshot, so the stream is not advertised.
        assert!(!burst.iter().any(|l| l.starts_with("iq_samplerate:")));

        let mut s = s;
        s.iq_rate = 192_000;
        assert!(connect_burst("sdroxide", &s).iter().any(|l| l == "iq_samplerate:192000;"));
    }
}
