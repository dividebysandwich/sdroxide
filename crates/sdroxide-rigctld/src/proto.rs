//! The rigctld wire protocol, as pure functions.
//!
//! Every compatibility hazard in this feature lives here, so none of it needs
//! a socket — or a radio — to test. The reference is Hamlib's own
//! `tests/rigctl_parse.c` (the writer) and `rigs/dummy/netrigctl.c` (the
//! reader that every "NET rigctl" client runs).
//!
//! **Response shapes.** In the default protocol a *set* replies `RPRT 0` and a
//! *get* replies only its values, one per line. Any failure replies
//! `RPRT <negative errno>`. In the extended protocol — selected per command by
//! a leading punctuation character — the command is echoed with its long name,
//! every value is labelled, and the block always ends `RPRT n`.

use sdroxide_types::{Command, Direction, NrLevel, NrStrength, RigctldConfig, RxId, Vfo};

use crate::state::{
    RigState, agc_from_hamlib, agc_to_hamlib, filter_for, from_hamlib_mode, nr_engine_for,
    nr_on_level, nr_strength_for, nr_to_hamlib, squelch_db_for, to_hamlib_mode,
};

/// Hamlib error codes (`rig.h`), returned negated in `RPRT`.
const OK: i32 = 0;
const EINVAL: i32 = -1;
const ENIMPL: i32 = -4;
const ERJCTED: i32 = -9;
const ENAVAIL: i32 = -11;

/// Hamlib's `RIG_MODE_*` bits (`rig.h`), OR-ed into the mask `\dump_state`
/// advertises. Spelled out rather than pre-multiplied so a future addition is
/// a one-line change that cannot get the arithmetic wrong.
mod rig_mode {
    pub const AM: u64 = 0x1;
    pub const CW: u64 = 0x2;
    pub const USB: u64 = 0x4;
    pub const LSB: u64 = 0x8;
    pub const RTTY: u64 = 0x10;
    pub const FM: u64 = 0x20;
    pub const WFM: u64 = 0x40;
    pub const CWR: u64 = 0x80;
    pub const RTTYR: u64 = 0x100;
    pub const PKTLSB: u64 = 0x400;
    pub const PKTUSB: u64 = 0x800;
    pub const SAM: u64 = 0x1_0000;
    pub const DSB: u64 = 0x8_0000;
    pub const PSK: u64 = 0x4000_0000;
    pub const PSKR: u64 = 0x8000_0000;
    pub const SPEC: u64 = 0x8_0000_0000;
}

/// Every mode we accept, as one bitmask.
const MODE_MASK: u64 = rig_mode::AM
    | rig_mode::CW
    | rig_mode::USB
    | rig_mode::LSB
    | rig_mode::RTTY
    | rig_mode::FM
    | rig_mode::WFM
    | rig_mode::CWR
    | rig_mode::RTTYR
    | rig_mode::PKTLSB
    | rig_mode::PKTUSB
    | rig_mode::SAM
    | rig_mode::DSB
    | rig_mode::PSK
    | rig_mode::PSKR
    | rig_mode::SPEC;

/// `RIG_VFO_A | RIG_VFO_B`.
const VFO_MASK: u32 = 0x3;

/// Hamlib's `RIG_LEVEL_*` bits (`rig.h`). Getting these wrong is not a cosmetic
/// error: a client consults the advertised mask and refuses to even *ask* for a
/// level it thinks we lack.
mod rig_level {
    pub const AF: u64 = 1 << 3;
    pub const RF: u64 = 1 << 4;
    pub const SQL: u64 = 1 << 5;
    pub const NR: u64 = 1 << 8;
    pub const RFPOWER: u64 = 1 << 12;
    pub const MICGAIN: u64 = 1 << 13;
    pub const AGC: u64 = 1 << 17;
    pub const STRENGTH: u64 = 1 << 30;
}

/// Hamlib's `RIG_FUNC_*` bits (`rig.h`).
mod rig_func {
    pub const NB: u64 = 1 << 1;
    pub const ANF: u64 = 1 << 8;
    pub const NR: u64 = 1 << 9;
    pub const MUTE: u64 = 1 << 17;
}

/// Hamlib's `RIG_OP_*` bits (`rig.h`), for the `vfo_ops` line.
mod rig_op {
    pub const CPY: u32 = 1 << 0;
    pub const XCHG: u32 = 1 << 1;
    pub const BAND_UP: u32 = 1 << 7;
    pub const BAND_DOWN: u32 = 1 << 8;
    pub const TUNE: u32 = 1 << 11;
    pub const TOGGLE: u32 = 1 << 12;
}

/// Every VFO operation we implement. Advertising one we then refuse is worse
/// than not advertising it, so this list and the `vfo_op` arm move together.
const VFO_OPS: u32 = rig_op::CPY
    | rig_op::XCHG
    | rig_op::BAND_UP
    | rig_op::BAND_DOWN
    | rig_op::TUNE
    | rig_op::TOGGLE;

/// Levels we can report. `STRENGTH` is a meter, so it is get-only.
const LEVEL_GET: u64 = rig_level::AF
    | rig_level::SQL
    | rig_level::NR
    | rig_level::RFPOWER
    | rig_level::MICGAIN
    | rig_level::AGC
    | rig_level::STRENGTH;
const LEVEL_SET: u64 = rig_level::AF
    | rig_level::SQL
    | rig_level::NR
    | rig_level::RFPOWER
    | rig_level::MICGAIN
    | rig_level::AGC;
const FUNC_MASK: u64 = rig_func::NB | rig_func::ANF | rig_func::NR | rig_func::MUTE;

/// What one connection needs to remember between commands.
#[derive(Debug, Clone, Default)]
pub struct Session {
    /// Set by `\set_vfo_opt 1`: every command then carries a VFO argument.
    pub vfo_opt: bool,
    /// The NR engine and strength last seen on, so `U NR 1` brings back the
    /// one the operator chose rather than a fixed default, and an `L NR`
    /// while NR is off is kept for when it comes on.
    pub nr_last: Option<NrLevel>,
}

/// What the caller should do after handling a line.
#[derive(Debug, Clone, PartialEq)]
pub enum Reply {
    /// Write this back (already newline-terminated).
    Send(String),
    /// The client said `q`: close without answering.
    Close,
}

/// How this command's response is formatted.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Format {
    extended: bool,
    sep: char,
}

impl Format {
    const PLAIN: Format = Format { extended: false, sep: '\n' };
}

/// Split off a leading response-separator character.
///
/// `+` selects the extended protocol with newline separators; any other
/// punctuation *except* `\ _ # ( ) ?` selects it with that character as the
/// separator. Those five are excluded because they begin real commands
/// (`\dump_state`, `_` for get_info, `#` comments).
fn split_format(line: &str) -> (Format, &str) {
    let mut chars = line.chars();
    match chars.next() {
        Some('+') => (Format { extended: true, sep: '\n' }, chars.as_str()),
        Some(c) if c.is_ascii_punctuation() && !matches!(c, '\\' | '_' | '#' | '(' | ')' | '?') => {
            (Format { extended: true, sep: c }, chars.as_str())
        }
        _ => (Format::PLAIN, line),
    }
}

/// Map a single-character command onto its long name. Anything not here is
/// either a `\long_name` or unknown.
fn long_name(short: &str) -> Option<&'static str> {
    Some(match short {
        "f" => "get_freq",
        "F" => "set_freq",
        "m" => "get_mode",
        "M" => "set_mode",
        "t" => "get_ptt",
        "T" => "set_ptt",
        "v" => "get_vfo",
        "V" => "set_vfo",
        "s" => "get_split_vfo",
        "S" => "set_split_vfo",
        "i" => "get_split_freq",
        "I" => "set_split_freq",
        "x" => "get_split_mode",
        "X" => "set_split_mode",
        "j" => "get_rit",
        "J" => "set_rit",
        "z" => "get_xit",
        "Z" => "set_xit",
        "l" => "get_level",
        "L" => "set_level",
        "u" => "get_func",
        "U" => "set_func",
        "G" => "vfo_op",
        "_" => "get_info",
        "1" => "dump_caps",
        "2" => "power2mW",
        "4" => "mW2power",
        "q" | "Q" => "quit",
        _ => return None,
    })
}

fn parse_vfo(s: &str) -> Option<Vfo> {
    match s.to_ascii_uppercase().as_str() {
        // "currVFO"/"VFO" mean "whatever is selected"; the caller substitutes.
        "VFOA" | "MAIN" | "A" => Some(Vfo::A),
        "VFOB" | "SUB" | "B" => Some(Vfo::B),
        _ => None,
    }
}

fn vfo_name(v: Vfo) -> &'static str {
    match v {
        Vfo::A => "VFOA",
        Vfo::B => "VFOB",
    }
}

/// Builds a reply in whichever protocol the command selected.
struct Out {
    fmt: Format,
    body: String,
}

impl Out {
    fn new(fmt: Format, long: &str, echo_args: &str) -> Self {
        let mut body = String::new();
        if fmt.extended {
            // The echo header is what tells an extended-protocol client which
            // command a block belongs to.
            body.push_str(long);
            body.push(':');
            if !echo_args.is_empty() {
                body.push(' ');
                body.push_str(echo_args);
            }
            body.push(fmt.sep);
        }
        Out { fmt, body }
    }

    /// One returned value. `label` is used only in the extended protocol.
    fn value(&mut self, label: &str, v: impl std::fmt::Display) {
        if self.fmt.extended {
            self.body.push_str(&format!("{label}: {v}{}", self.fmt.sep));
        } else {
            self.body.push_str(&format!("{v}\n"));
        }
    }

    /// Finish with a return code. The default protocol prints `RPRT` only for
    /// sets and failures; the extended protocol always does.
    fn finish(mut self, code: i32, is_get: bool) -> Reply {
        if self.fmt.extended {
            self.body.push_str(&format!("RPRT {code}"));
            // A non-newline separator still needs the block to end with one.
            self.body.push('\n');
        } else if code != OK || !is_get {
            self.body.push_str(&format!("RPRT {code}\n"));
        }
        Reply::Send(self.body)
    }
}

/// Handle one received line.
///
/// Commands the engine must carry out are pushed onto `cmds`; everything
/// answerable from `st` is answered directly.
pub fn handle(
    line: &str,
    st: &RigState,
    cfg: &RigctldConfig,
    sess: &mut Session,
    cmds: &mut Vec<Command>,
) -> Reply {
    let line = line.trim_end_matches(['\r', '\n']);
    let (fmt, rest) = split_format(line);
    let rest = rest.trim();
    if rest.is_empty() {
        return Reply::Send(String::new());
    }

    let mut tokens = rest.split_whitespace();
    let head = tokens.next().unwrap_or_default();
    let long = if let Some(stripped) = head.strip_prefix('\\') {
        stripped.to_string()
    } else if let Some(l) = long_name(head) {
        l.to_string()
    } else {
        // An unrecognised head is a protocol error, not a silent no-op: a
        // client that gets no answer at all blocks until it times out.
        return Out::new(fmt, head, "").finish(EINVAL, false);
    };

    let mut args: Vec<&str> = tokens.collect();
    // In VFO mode every command carries a leading VFO token. Accept one even
    // when we are not in VFO mode: clients send it anyway, and dropping the
    // command over it is the classic "it half works" report.
    let mut vfo = st.active_vfo;
    if !args.is_empty() && long != "set_vfo" && long != "get_vfo_info" {
        if let Some(v) = parse_vfo(args[0]) {
            vfo = v;
            args.remove(0);
        } else if args[0].eq_ignore_ascii_case("currVFO") || args[0].eq_ignore_ascii_case("VFO") {
            args.remove(0);
        }
    }
    let echo = args.join(" ");
    dispatch(&long, &args, &echo, fmt, vfo, st, cfg, sess, cmds)
}

#[allow(clippy::too_many_arguments)]
fn dispatch(
    long: &str,
    args: &[&str],
    echo: &str,
    fmt: Format,
    vfo: Vfo,
    st: &RigState,
    cfg: &RigctldConfig,
    sess: &mut Session,
    cmds: &mut Vec<Command>,
) -> Reply {
    let mut out = Out::new(fmt, long, echo);
    if st.nr_level.is_on() {
        sess.nr_last = Some(st.nr_level);
    }
    let num = |i: usize| args.get(i).and_then(|s| s.parse::<f64>().ok());
    let int = |i: usize| args.get(i).and_then(|s| s.parse::<i32>().ok());

    match long {
        "quit" => Reply::Close,

        // ── Frequency ────────────────────────────────────────────────────────
        "get_freq" => {
            out.value("Frequency", st.freq(vfo).round() as i64);
            out.finish(OK, true)
        }
        "set_freq" => match num(0) {
            Some(hz) if hz >= 0.0 => {
                cmds.push(Command::SetVfo { vfo, hz });
                out.finish(OK, false)
            }
            _ => out.finish(EINVAL, false),
        },

        // ── Mode ─────────────────────────────────────────────────────────────
        "get_mode" => {
            out.value("Mode", to_hamlib_mode(st.mode));
            out.value("Passband", st.passband_hz());
            out.finish(OK, true)
        }
        "set_mode" => set_mode(args, st, cmds, out),

        // ── PTT ──────────────────────────────────────────────────────────────
        "get_ptt" => {
            out.value("PTT", i32::from(st.ptt || st.tune));
            out.finish(OK, true)
        }
        "set_ptt" => match int(0) {
            Some(v) => {
                if !cfg.allow_tx {
                    // Refused rather than ignored, so the operator sees why in
                    // the client's log instead of a rig that never keys.
                    return out.finish(ERJCTED, false);
                }
                cmds.push(Command::SetPtt(v != 0));
                out.finish(OK, false)
            }
            None => out.finish(EINVAL, false),
        },

        // ── VFO / split ──────────────────────────────────────────────────────
        "get_vfo" => {
            out.value("VFO", vfo_name(st.active_vfo));
            out.finish(OK, true)
        }
        "set_vfo" => match args.first().and_then(|s| parse_vfo(s)) {
            Some(v) => {
                cmds.push(Command::SelectVfo(v));
                out.finish(OK, false)
            }
            None => out.finish(EINVAL, false),
        },
        "get_split_vfo" => {
            out.value("Split", i32::from(st.split));
            out.value("TX VFO", vfo_name(st.tx_vfo()));
            out.finish(OK, true)
        }
        "set_split_vfo" => match int(0) {
            Some(v) => {
                cmds.push(Command::SetSplit(v != 0));
                out.finish(OK, false)
            }
            None => out.finish(EINVAL, false),
        },
        "get_split_freq" => {
            out.value("TX Frequency", st.freq(st.tx_vfo()).round() as i64);
            out.finish(OK, true)
        }
        "set_split_freq" => match num(0) {
            Some(hz) if hz >= 0.0 => {
                cmds.push(Command::SetVfo { vfo: st.tx_vfo(), hz });
                out.finish(OK, false)
            }
            _ => out.finish(EINVAL, false),
        },
        "get_split_mode" => {
            // One receiver, so the transmit mode is the receive mode.
            out.value("TX Mode", to_hamlib_mode(st.mode));
            out.value("TX Passband", st.passband_hz());
            out.finish(OK, true)
        }
        "set_split_mode" => set_mode(args, st, cmds, out),

        // ── RIT / XIT ────────────────────────────────────────────────────────
        "get_rit" => {
            out.value("RIT", st.rit_hz);
            out.finish(OK, true)
        }
        "set_rit" => match int(0) {
            Some(hz) => {
                cmds.push(Command::SetRit { enabled: hz != 0, hz: hz.clamp(-9999, 9999) });
                out.finish(OK, false)
            }
            None => out.finish(EINVAL, false),
        },
        "get_xit" => {
            out.value("XIT", st.xit_hz);
            out.finish(OK, true)
        }
        "set_xit" => match int(0) {
            Some(hz) => {
                cmds.push(Command::SetXit { enabled: hz != 0, hz: hz.clamp(-9999, 9999) });
                out.finish(OK, false)
            }
            None => out.finish(EINVAL, false),
        },

        // ── Levels and functions ─────────────────────────────────────────────
        "get_level" => match args.first().map(|s| s.to_ascii_uppercase()) {
            Some(l) if l == "RFPOWER" => {
                out.value("Level Value", format!("{:.6}", st.drive));
                out.finish(OK, true)
            }
            Some(l) if l == "AF" => {
                out.value("Level Value", format!("{:.6}", st.volume));
                out.finish(OK, true)
            }
            Some(l) if l == "MICGAIN" => {
                out.value("Level Value", format!("{:.6}", st.mic_gain));
                out.finish(OK, true)
            }
            Some(l) if l == "SQL" => {
                out.value("Level Value", format!("{:.6}", st.sql_level()));
                out.finish(OK, true)
            }
            // The strength running, or the one NR comes back on at.
            Some(l) if l == "NR" => {
                let level = if st.nr_level.is_on() { Some(st.nr_level) } else { sess.nr_last };
                let strength = level.and_then(|x| x.strength()).unwrap_or(NrStrength::Med);
                out.value("Level Value", format!("{:.6}", nr_to_hamlib(strength)));
                out.finish(OK, true)
            }
            // An integer, as Hamlib's AGC level is: a `RIG_AGC_*` value.
            Some(l) if l == "AGC" => {
                out.value("Level Value", agc_to_hamlib(st.agc));
                out.finish(OK, true)
            }
            // The front-end gain, where the radio has one: 0.0..=1.0 over the
            // stage's range.
            Some(l) if l == "RF" => match &st.rf_gain {
                Some(g) => {
                    out.value("Level Value", format!("{:.6}", g.level()));
                    out.finish(OK, true)
                }
                None => out.finish(ENAVAIL, true),
            },
            Some(l) if l == "STRENGTH" => {
                // Hamlib's STRENGTH is S-units above S9, in dB: S9 = -73 dBm.
                out.value("Level Value", st.strength_dbm + 73);
                out.finish(OK, true)
            }
            Some(_) => out.finish(ENAVAIL, true),
            None => out.finish(EINVAL, true),
        },
        "set_level" => {
            let name = args.first().map(|s| s.to_ascii_uppercase()).unwrap_or_default();
            match (name.as_str(), num(1)) {
                ("RFPOWER", Some(v)) => {
                    cmds.push(Command::SetTxDrive(v.clamp(0.0, 1.0) as f32));
                    out.finish(OK, false)
                }
                ("AF", Some(v)) => {
                    cmds.push(Command::SetVolume { rx: RxId::Main, v: v.clamp(0.0, 1.0) as f32 });
                    out.finish(OK, false)
                }
                ("MICGAIN", Some(v)) => {
                    cmds.push(Command::SetMicGain(v.clamp(0.0, 1.0) as f32));
                    out.finish(OK, false)
                }
                ("SQL", Some(v)) => {
                    cmds.push(Command::SetSquelch { rx: RxId::Main, db: squelch_db_for(v) });
                    out.finish(OK, false)
                }
                // The strength only: the engine stays the one chosen. While
                // NR is off it is kept for when it comes on, not switched on.
                ("NR", Some(v)) => {
                    let level = nr_engine_for(st, sess.nr_last).at(nr_strength_for(v));
                    if st.nr_level.is_on() {
                        cmds.push(Command::SetNoiseReduction { rx: RxId::Main, level });
                    } else {
                        sess.nr_last = Some(level);
                    }
                    out.finish(OK, false)
                }
                ("AGC", Some(v)) => match agc_from_hamlib(v.round() as i32) {
                    Some(agc) => {
                        cmds.push(Command::SetAgc { rx: RxId::Main, agc });
                        out.finish(OK, false)
                    }
                    None => out.finish(EINVAL, false),
                },
                ("RF", Some(v)) => match &st.rf_gain {
                    Some(g) => {
                        cmds.push(Command::SetGain {
                            dir: Direction::Rx,
                            element: g.element.clone(),
                            db: g.value_for(v),
                        });
                        out.finish(OK, false)
                    }
                    None => out.finish(ENAVAIL, false),
                },
                ("", _) => out.finish(EINVAL, false),
                (_, None) => out.finish(EINVAL, false),
                _ => out.finish(ENAVAIL, false),
            }
        }
        "get_func" => {
            let on = match args.first().map(|s| s.to_ascii_uppercase()).unwrap_or_default().as_str()
            {
                "NB" => st.noise_blanker,
                "NR" => st.noise_reduction,
                "ANF" => st.auto_notch,
                "MUTE" => st.muted,
                "" => return out.finish(EINVAL, true),
                _ => return out.finish(ENAVAIL, true),
            };
            out.value("Func Status", i32::from(on));
            out.finish(OK, true)
        }
        "set_func" => {
            let name = args.first().map(|s| s.to_ascii_uppercase()).unwrap_or_default();
            let Some(v) = int(1) else { return out.finish(EINVAL, false) };
            let on = v != 0;
            let rx = RxId::Main;
            match name.as_str() {
                "NB" => cmds.push(Command::SetNoiseBlanker(on)),
                // On: the engine and strength last seen on, not always the
                // spectral NR at medium, which undid the operator's choice.
                "NR" => cmds.push(Command::SetNoiseReduction {
                    rx,
                    level: if on { nr_on_level(sess.nr_last) } else { NrLevel::Off },
                }),
                "ANF" => cmds.push(Command::SetAutoNotch { rx, on }),
                "MUTE" => cmds.push(Command::SetMute { rx, muted: on }),
                "" => return out.finish(EINVAL, false),
                _ => return out.finish(ENAVAIL, false),
            }
            out.finish(OK, false)
        }

        // ── VFO operations ───────────────────────────────────────────────────
        "vfo_op" => {
            match args.first().map(|s| s.to_ascii_uppercase()).unwrap_or_default().as_str() {
                "XCHG" => cmds.push(Command::SwapVfos),
                "CPY" => cmds.push(Command::CopyAtoB),
                "TOGGLE" => cmds.push(Command::SelectVfo(match st.active_vfo {
                    Vfo::A => Vfo::B,
                    Vfo::B => Vfo::A,
                })),
                "TUNE" => {
                    if !cfg.allow_tx {
                        return out.finish(ERJCTED, false);
                    }
                    cmds.push(Command::SetTune(true));
                }
                op @ ("BAND_UP" | "BAND_DOWN") => match step_band(st.band, op == "BAND_UP") {
                    Some(b) => cmds.push(Command::SetBand(b)),
                    // Outside the ham bands there is no "next band" to go to.
                    None => return out.finish(ERJCTED, false),
                },
                "" => return out.finish(EINVAL, false),
                _ => return out.finish(ENAVAIL, false),
            }
            out.finish(OK, false)
        }

        // ── Voice keyer ──────────────────────────────────────────────────────
        // Hamlib's `send_voice_mem` / `stop_voice_mem`. Channels are 1-based on
        // the wire and 0-based inside sdroxide. This keys the transmitter, so it
        // obeys the same `allow_tx` gate as PTT and TUNE.
        "send_voice_mem" => {
            if !cfg.allow_tx {
                return out.finish(ERJCTED, false);
            }
            match int(0) {
                Some(ch) if (1..=sdroxide_types::VOICE_SLOTS as i32).contains(&ch) => {
                    cmds.push(Command::VoicePlay(Some(ch as u8 - 1)));
                    out.finish(OK, false)
                }
                _ => out.finish(EINVAL, false),
            }
        }
        "stop_voice_mem" => {
            cmds.push(Command::VoicePlay(None));
            out.finish(OK, false)
        }

        // ── Identity and capabilities ────────────────────────────────────────
        "get_info" => {
            out.value("Info", format!("{} {}", cfg.rig_name, env!("CARGO_PKG_VERSION")));
            out.finish(OK, true)
        }
        "dump_caps" => {
            // netrigctl never parses this; it exists for `rigctl -m 2 1`.
            out.value("Caps", format!("{} rigctld-compatible", cfg.rig_name));
            out.finish(OK, true)
        }
        "dump_state" => Reply::Send(dump_state(st, cfg)),
        "chk_vfo" => {
            // Must be a bare integer: netrigctl reads it with `sscanf("%d")`,
            // so a "CHKVFO 0" answer leaves it reading garbage.
            if fmt.extended {
                Reply::Send(format!("ChkVFO: {}{}\n", i32::from(sess.vfo_opt), fmt.sep))
            } else {
                Reply::Send(format!("{}\n", i32::from(sess.vfo_opt)))
            }
        }
        "set_vfo_opt" => match int(0) {
            Some(v) => {
                sess.vfo_opt = v != 0;
                out.finish(OK, false)
            }
            None => out.finish(EINVAL, false),
        },
        "get_powerstat" => {
            out.value("Power Status", 1);
            out.finish(OK, true)
        }
        "set_powerstat" => match int(0) {
            // We are always on; a request to stay on succeeds, one to power
            // off is refused rather than silently ignored.
            Some(1) => out.finish(OK, false),
            Some(_) => out.finish(ENIMPL, false),
            None => out.finish(EINVAL, false),
        },
        "get_vfo_info" => {
            let v = args.first().and_then(|s| parse_vfo(s)).unwrap_or(st.active_vfo);
            out.value("Freq", st.freq(v).round() as i64);
            out.value("Mode", to_hamlib_mode(st.mode));
            out.value("Width", st.passband_hz());
            out.value("Split", i32::from(st.split));
            out.value("SatMode", 0);
            out.finish(OK, true)
        }
        "get_rig_info" => {
            let line = format!(
                "VFO={} Freq={} Mode={} Width={} RX=1 TX={}\nSplit={} SatMode=0\nRig={} Version={} App=sdroxide\n",
                vfo_name(st.active_vfo),
                st.freq(st.active_vfo).round() as i64,
                to_hamlib_mode(st.mode),
                st.passband_hz(),
                i32::from(st.ptt || st.tune),
                i32::from(st.split),
                cfg.rig_name,
                env!("CARGO_PKG_VERSION"),
            );
            Reply::Send(line)
        }
        "get_lock_mode" => {
            out.value("Locked", 0);
            out.finish(OK, true)
        }
        "set_lock_mode" => out.finish(OK, false),
        "get_clock" | "set_clock" | "power2mW" | "mW2power" => out.finish(ENIMPL, false),

        _ => out.finish(EINVAL, false),
    }
}

/// `set_mode` / `set_split_mode`, sharing the no-op rule.
///
/// **The rule that keeps FT8 alive:** WSJT-X reads the mode, then re-sends what
/// it read every few seconds. Since every digital mode reports `PKTUSB`, taking
/// that literally would switch a running FT8 session to plain DIGU — killing
/// the decoder — on a timer. So a request for the mode we already *report* is
/// a success that changes nothing.
fn set_mode(args: &[&str], st: &RigState, cmds: &mut Vec<Command>, out: Out) -> Reply {
    let Some(name) = args.first() else { return out.finish(EINVAL, false) };
    let Some(mode) = from_hamlib_mode(name) else { return out.finish(EINVAL, false) };
    if name.eq_ignore_ascii_case(to_hamlib_mode(st.mode)) {
        return out.finish(OK, false);
    }
    cmds.push(Command::SetMode { rx: RxId::Main, mode });
    // A width of 0 means "leave the filter alone" — that is how clients that
    // only care about the mode say so.
    if let Some(w) = args.get(1).and_then(|s| s.parse::<i32>().ok())
        && w > 0
    {
        let (lo, hi) = filter_for(mode, w);
        cmds.push(Command::SetFilter { rx: RxId::Main, lo, hi });
    }
    out.finish(OK, false)
}

/// Next or previous amateur band, skipping the general-coverage pseudo-band and
/// any band this region's band plan does not have (4 m outside Region 1).
fn step_band(cur: sdroxide_types::Band, up: bool) -> Option<sdroxide_types::Band> {
    use sdroxide_types::Band;
    let ham: Vec<Band> = Band::ALL.iter().copied().filter(|b| b.edges().is_some()).collect();
    let i = ham.iter().position(|b| *b == cur)?;
    let n = ham.len();
    Some(if up { ham[(i + 1) % n] } else { ham[(i + n - 1) % n] })
}

/// One `start end 0xmodes low_pwr high_pwr 0xvfo 0xant` frequency-range line.
///
/// Hamlib reads these with `%lf %lf %llx %d %d %x %x`, so the field *types*
/// matter: modes/vfo/ant hex with the `0x` prefix, powers plain decimal.
fn range_line(lo: f64, hi: f64, tx: bool) -> String {
    let (low_pwr, high_pwr) = if tx { (1, 100) } else { (-1, -1) };
    format!("{lo:.6} {hi:.6} 0x{MODE_MASK:x} {low_pwr} {high_pwr} 0x{VFO_MASK:x} 0x0\n")
}

/// The `\dump_state` block every Hamlib "NET rigctl" client parses on connect.
///
/// This is where rigctld emulations usually break, so the invariants are worth
/// stating outright:
///
/// * The RX and TX sentinels need **exactly seven** integers and the tuning-step
///   and filter sentinels **exactly two** — Hamlib checks the field count and
///   fails the whole connection when it is wrong.
/// * The RX range list must be **non-empty**. An immediate sentinel leaves
///   Hamlib's `mode_list` at zero, after which it rejects every `set_mode` as
///   unsupported.
/// * Preamp and attenuator lines must be present even when there is nothing to
///   list — emit a bare `0`.
/// * Protocol version 1 **must** be followed by the trailing `done`. Without it
///   the client blocks in `read_string` until its timeout: the notorious
///   "rig control read failed after N seconds".
fn dump_state(st: &RigState, cfg: &RigctldConfig) -> String {
    let mut s = String::new();
    // Protocol version, then the model and the deprecated ITU region.
    s.push_str("1\n2\n0\n");

    // RX ranges. Falling back to full coverage rather than emitting an empty
    // list, because an empty one disables mode setting client-side.
    let rx: Vec<(f64, f64)> =
        if st.rx_ranges.is_empty() { vec![(0.0, 6_000_000_000.0)] } else { st.rx_ranges.clone() };
    for (lo, hi) in &rx {
        s.push_str(&range_line(*lo, *hi, false));
    }
    s.push_str("0 0 0 0 0 0 0\n");

    // TX ranges. Empty when transmit is off or unsupported, which is what makes
    // Hamlib itself refuse to key rather than asking and being rejected.
    if st.can_tx && cfg.allow_tx {
        let tx: Vec<(f64, f64)> =
            if st.tx_ranges.is_empty() { rx.clone() } else { st.tx_ranges.clone() };
        for (lo, hi) in &tx {
            s.push_str(&range_line(*lo, *hi, true));
        }
    }
    s.push_str("0 0 0 0 0 0 0\n");

    // Tuning steps, then the sentinel (exactly two fields).
    s.push_str(&format!("0x{MODE_MASK:x} 1\n"));
    s.push_str("0 0\n");

    // Filters: one line per mode group, then the sentinel (exactly two fields).
    s.push_str(&format!("0x{:x} 2700\n", rig_mode::USB | rig_mode::LSB));
    s.push_str(&format!("0x{:x} 500\n", rig_mode::CW | rig_mode::CWR));
    s.push_str(&format!("0x{:x} 6000\n", rig_mode::AM | rig_mode::SAM));
    s.push_str(&format!("0x{:x} 16000\n", rig_mode::FM));
    s.push_str(&format!("0x{:x} 3100\n", rig_mode::PKTUSB | rig_mode::PKTLSB));
    s.push_str("0 0\n");

    s.push_str("9999\n"); // max_rit
    s.push_str("9999\n"); // max_xit
    s.push_str("0\n"); // max_ifshift
    s.push_str("0\n"); // announces
    s.push_str("0\n"); // preamp list (empty, but the line must exist)
    s.push_str("0\n"); // attenuator list

    // get_func, set_func, get_level, set_level, get_parm, set_parm.
    s.push_str(&format!("0x{FUNC_MASK:x}\n0x{FUNC_MASK:x}\n"));
    // RF only where there is a gain stage to set: a client that sees it
    // advertised offers the control, and one refused every time looks broken.
    let rf = if st.rf_gain.is_some() { rig_level::RF } else { 0 };
    s.push_str(&format!("0x{:x}\n0x{:x}\n", LEVEL_GET | rf, LEVEL_SET | rf));
    s.push_str("0x0\n0x0\n"); // get/set parm

    // The trailing key=value block.
    s.push_str(&format!("vfo_ops=0x{VFO_OPS:x}\n"));
    let ptt_type = if st.can_tx && cfg.allow_tx { "0x1" } else { "0x0" };
    s.push_str(&format!("ptt_type={ptt_type}\n"));
    s.push_str("targetable_vfo=0x1\n");
    s.push_str("has_set_vfo=1\n");
    s.push_str("has_get_vfo=1\n");
    s.push_str("has_set_freq=1\n");
    s.push_str("has_get_freq=1\n");
    s.push_str("has_set_conf=0\n");
    s.push_str("has_get_conf=0\n");
    s.push_str("has_power2mW=0\n");
    s.push_str("has_mW2power=0\n");
    s.push_str("has_get_ant=0\n");
    s.push_str("has_set_ant=0\n");
    // Voice keyer. Hamlib ignores unknown keys here, so advertising it costs
    // nothing on clients that don't look — and the ones that do can tell.
    let voice = i32::from(st.can_tx && cfg.allow_tx);
    s.push_str(&format!("has_send_voice_mem={voice}\n"));
    s.push_str("timeout=2000\n");
    s.push_str("rig_model=2\n");
    s.push_str(&format!("rigctld_version={} {}\n", cfg.rig_name, env!("CARGO_PKG_VERSION")));
    s.push_str("done\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_types::{AgcMode, Mode};

    fn st() -> RigState {
        RigState {
            vfo_a_hz: 14_074_000.0,
            vfo_b_hz: 7_074_000.0,
            mode: Mode::Usb,
            filter_lo: 150.0,
            filter_hi: 2850.0,
            can_tx: true,
            rx_ranges: vec![(100_000.0, 30_000_000.0)],
            tx_ranges: vec![(1_800_000.0, 2_000_000.0)],
            ..RigState::default()
        }
    }

    fn run(line: &str) -> (String, Vec<Command>) {
        run_with(line, &st(), &RigctldConfig::default())
    }

    fn run_with(line: &str, s: &RigState, cfg: &RigctldConfig) -> (String, Vec<Command>) {
        let mut cmds = Vec::new();
        let mut sess = Session::default();
        match handle(line, s, cfg, &mut sess, &mut cmds) {
            Reply::Send(t) => (t, cmds),
            Reply::Close => ("<close>".into(), cmds),
        }
    }

    #[test]
    fn plain_get_returns_only_the_value() {
        assert_eq!(run("f").0, "14074000\n");
    }

    #[test]
    fn plain_set_returns_rprt_zero() {
        let (reply, cmds) = run("F 14076000");
        assert_eq!(reply, "RPRT 0\n");
        assert_eq!(cmds, vec![Command::SetVfo { vfo: Vfo::A, hz: 14_076_000.0 }]);
    }

    #[test]
    fn long_command_names_work_too() {
        assert_eq!(run("\\get_freq").0, "14074000\n");
        assert_eq!(
            run("\\set_freq 7100000").1,
            vec![Command::SetVfo { vfo: Vfo::A, hz: 7_100_000.0 }]
        );
    }

    #[test]
    fn extended_protocol_echoes_and_labels() {
        assert_eq!(run("+f").0, "get_freq:\nFrequency: 14074000\nRPRT 0\n");
    }

    /// A punctuation prefix other than `+` becomes the separator itself, and
    /// the block still ends with a real newline.
    #[test]
    fn extended_protocol_honours_a_custom_separator() {
        assert_eq!(run(";f").0, "get_freq:;Frequency: 14074000;RPRT 0\n");
        assert_eq!(run("|f").0, "get_freq:|Frequency: 14074000|RPRT 0\n");
    }

    /// `\` starts a long command name and must never be eaten as a separator.
    #[test]
    fn backslash_is_not_a_separator() {
        assert_eq!(run("\\dump_state").0.lines().next(), Some("1"));
        assert_eq!(run("_").0, format!("{} {}\n", "sdroxide", env!("CARGO_PKG_VERSION")));
    }

    /// netrigctl reads this with `sscanf("%d")`, so anything but a bare integer
    /// leaves it parsing garbage — the bug behind Hamlib's chk_vfo warnings
    /// against other SDR emulations.
    #[test]
    fn chk_vfo_is_a_bare_integer() {
        assert_eq!(run("\\chk_vfo").0, "0\n");
    }

    #[test]
    fn mode_reports_name_and_passband() {
        assert_eq!(run("m").0, "USB\n2700\n");
    }

    /// The FT8 survival rule: WSJT-X re-sends the mode it just read, every few
    /// seconds. Taking that literally would drop a running FT8 session to DIGU.
    #[test]
    fn resetting_the_reported_mode_changes_nothing() {
        let s = RigState { mode: Mode::Ft8, ..st() };
        let (reply, cmds) = run_with("M PKTUSB 3000", &s, &RigctldConfig::default());
        assert_eq!(reply, "RPRT 0\n");
        assert!(cmds.is_empty(), "FT8 must survive WSJT-X's periodic mode write");
    }

    #[test]
    fn a_genuine_mode_change_is_applied() {
        let (_, cmds) = run("M CW 500");
        assert_eq!(cmds[0], Command::SetMode { rx: RxId::Main, mode: Mode::Cw });
        assert!(matches!(cmds[1], Command::SetFilter { .. }));
    }

    /// Width 0 is how a client says "mode only, leave my filter alone".
    #[test]
    fn zero_width_leaves_the_filter_alone() {
        let (_, cmds) = run("M CW 0");
        assert_eq!(cmds.len(), 1);
    }

    #[test]
    fn ptt_round_trips() {
        assert_eq!(run("t").0, "0\n");
        assert_eq!(run("T 1").1, vec![Command::SetPtt(true)]);
    }

    /// With transmit disallowed, keying is refused outright rather than
    /// quietly dropped, and `\dump_state` also stops advertising any TX range
    /// so Hamlib refuses before it even asks.
    #[test]
    fn transmit_can_be_denied() {
        let cfg = RigctldConfig { allow_tx: false, ..RigctldConfig::default() };
        let (reply, cmds) = run_with("T 1", &st(), &cfg);
        assert_eq!(reply, "RPRT -9\n");
        assert!(cmds.is_empty());

        let dump = run_with("\\dump_state", &st(), &cfg).0;
        assert!(dump.contains("ptt_type=0x0"));
        let tx_lines = dump.lines().filter(|l| l.contains("1 100")).count();
        assert_eq!(tx_lines, 0, "no TX ranges may be advertised");
    }

    /// `send_voice_mem` is 1-based on the wire; out-of-range channels are an
    /// error rather than a silently clamped transmission.
    #[test]
    fn voice_memories_play_and_stop() {
        assert_eq!(run("\\send_voice_mem 1").1, vec![Command::VoicePlay(Some(0))]);
        assert_eq!(run("\\send_voice_mem 10").1, vec![Command::VoicePlay(Some(9))]);
        assert_eq!(run("\\stop_voice_mem").1, vec![Command::VoicePlay(None)]);

        assert_eq!(run("\\send_voice_mem 0").0, "RPRT -1\n");
        assert_eq!(run("\\send_voice_mem 11").0, "RPRT -1\n");
        assert!(run("\\send_voice_mem 11").1.is_empty());

        // Keying is keying: the transmit gate applies.
        let cfg = RigctldConfig { allow_tx: false, ..RigctldConfig::default() };
        let (reply, cmds) = run_with("\\send_voice_mem 1", &st(), &cfg);
        assert_eq!(reply, "RPRT -9\n");
        assert!(cmds.is_empty());
    }

    #[test]
    fn split_reports_the_other_vfo() {
        let s = RigState { split: true, active_vfo: Vfo::A, ..st() };
        assert_eq!(run_with("s", &s, &RigctldConfig::default()).0, "1\nVFOB\n");
        assert_eq!(run_with("i", &s, &RigctldConfig::default()).0, "7074000\n");
        let (_, cmds) = run_with("I 7080000", &s, &RigctldConfig::default());
        assert_eq!(cmds, vec![Command::SetVfo { vfo: Vfo::B, hz: 7_080_000.0 }]);
    }

    /// Clients send a VFO argument whether or not VFO mode is on. Rejecting it
    /// is the classic "works, but only half the commands" failure.
    #[test]
    fn a_leading_vfo_argument_is_accepted_outside_vfo_mode() {
        assert_eq!(run("f VFOB").0, "7074000\n");
        let (_, cmds) = run("F VFOA 21074000");
        assert_eq!(cmds, vec![Command::SetVfo { vfo: Vfo::A, hz: 21_074_000.0 }]);
    }

    #[test]
    fn set_vfo_keeps_its_own_argument() {
        assert_eq!(run("V VFOB").1, vec![Command::SelectVfo(Vfo::B)]);
    }

    #[test]
    fn unknown_commands_and_levels_report_rather_than_hang() {
        assert_eq!(run("Y").0, "RPRT -1\n");
        assert_eq!(run("l NOSUCHLEVEL").0, "RPRT -11\n");
        assert_eq!(run("u NOSUCHFUNC").0, "RPRT -11\n");
    }

    #[test]
    fn levels_and_functions_round_trip() {
        let s = RigState { drive: 0.25, volume: 0.5, noise_blanker: true, ..st() };
        let cfg = RigctldConfig::default();
        assert_eq!(run_with("l RFPOWER", &s, &cfg).0, "0.250000\n");
        assert_eq!(run_with("u NB", &s, &cfg).0, "1\n");
        assert_eq!(run_with("L RFPOWER 0.8", &s, &cfg).1, vec![Command::SetTxDrive(0.8)]);
        assert_eq!(run_with("U NB 0", &s, &cfg).1, vec![Command::SetNoiseBlanker(false)]);
    }

    /// SQL is the squelch across its own rail — 0 open, 1 closed — and AGC a
    /// `RIG_AGC_*` integer, both readable and settable.
    #[test]
    fn squelch_and_agc_round_trip() {
        let s = RigState { squelch_db: -75.0, agc: AgcMode::Slow, ..st() };
        let cfg = RigctldConfig::default();
        assert_eq!(run_with("l SQL", &s, &cfg).0, "0.500000\n");
        assert_eq!(run_with("l AGC", &s, &cfg).0, "3\n");
        assert_eq!(
            run_with("L SQL 0", &s, &cfg).1,
            vec![Command::SetSquelch { rx: RxId::Main, db: sdroxide_types::SQUELCH_OPEN_DB }]
        );
        assert_eq!(
            run_with("L SQL 1", &s, &cfg).1,
            vec![Command::SetSquelch { rx: RxId::Main, db: sdroxide_types::SQUELCH_CLOSED_DB }]
        );
        assert_eq!(
            run_with("L AGC 0", &s, &cfg).1,
            vec![Command::SetAgc { rx: RxId::Main, agc: AgcMode::Off }]
        );
        assert_eq!(
            run_with("L AGC 5", &s, &cfg).1,
            vec![Command::SetAgc { rx: RxId::Main, agc: AgcMode::Med }]
        );
        assert_eq!(run_with("L AGC 9", &s, &cfg).0, "RPRT -1\n");
    }

    /// An RTL-SDR's tuner stage: 0..=49.6 dB in 0.1 dB steps, set at 29.7.
    fn with_gain() -> RigState {
        RigState {
            rf_gain: Some(crate::state::RfGain {
                element: "TUNER".into(),
                min: 0.0,
                max: 49.6,
                step: 0.1,
                value: 29.7,
            }),
            ..st()
        }
    }

    /// The level mask lines of `\dump_state`: get, then set.
    fn level_masks(d: &str) -> (u64, u64) {
        let lines: Vec<&str> = d.lines().collect();
        let ops = lines.iter().position(|l| l.starts_with("vfo_ops=")).expect("vfo_ops");
        let hex = |l: &str| u64::from_str_radix(l.trim_start_matches("0x"), 16).expect("hex");
        (hex(lines[ops - 4]), hex(lines[ops - 3]))
    }

    /// RF is the front-end gain across the stage's own range, read and set on
    /// its own steps.
    #[test]
    fn rf_gain_round_trips_over_the_stage_range() {
        let s = with_gain();
        let cfg = RigctldConfig::default();
        assert_eq!(run_with("l RF", &s, &cfg).0, "0.598790\n");
        let set = |line: &str| run_with(line, &s, &cfg).1;
        let gain =
            |db: f64| vec![Command::SetGain { dir: Direction::Rx, element: "TUNER".into(), db }];
        assert_eq!(set("L RF 0"), gain(0.0));
        assert_eq!(set("L RF 1"), gain(49.6));
        assert_eq!(set("L RF 7"), gain(49.6), "clamped to the top of the range");
        // Half of 49.6 is 24.8, a whole step already.
        let (_, cmds) = run_with("L RF 0.5", &s, &cfg);
        let Command::SetGain { db, .. } = &cmds[0] else { panic!("{cmds:?}") };
        assert!((db - 24.8).abs() < 1e-9, "{db}");
        // Between steps it lands on one.
        let (_, cmds) = run_with("L RF 0.3333", &s, &cfg);
        let Command::SetGain { db, .. } = &cmds[0] else { panic!("{cmds:?}") };
        assert!((db * 10.0 - (db * 10.0).round()).abs() < 1e-6, "{db} is not on a 0.1 dB step");
    }

    /// No gain stage, no RF level: refused, and not advertised either.
    #[test]
    fn rf_gain_is_absent_without_a_gain_stage() {
        let cfg = RigctldConfig::default();
        assert_eq!(run("l RF").0, "RPRT -11\n");
        assert_eq!(run("L RF 0.5"), ("RPRT -11\n".to_string(), Vec::new()));
        let rf = 1u64 << 4;
        let (get, set) = level_masks(&run("\\dump_state").0);
        assert_eq!((get & rf, set & rf), (0, 0));
        let (get, set) = level_masks(&run_with("\\dump_state", &with_gain(), &cfg).0);
        assert_eq!((get & rf, set & rf), (rf, rf));
    }

    /// NR's strength is Hamlib's `NR` level; the engine stays the one chosen.
    #[test]
    fn nr_strength_keeps_the_engine() {
        let s = RigState { noise_reduction: true, nr_level: NrLevel::DfMed, ..st() };
        let cfg = RigctldConfig::default();
        assert_eq!(run_with("l NR", &s, &cfg).0, "0.666667\n");
        assert_eq!(
            run_with("L NR 1", &s, &cfg).1,
            vec![Command::SetNoiseReduction { rx: RxId::Main, level: NrLevel::DfHigh }]
        );
        assert_eq!(
            run_with("L NR 0.2", &s, &cfg).1,
            vec![Command::SetNoiseReduction { rx: RxId::Main, level: NrLevel::DfLow }]
        );
    }

    /// Off and on again over rigctl brings back the engine and strength the
    /// operator had, and a strength set while off waits for NR to come on.
    #[test]
    fn nr_comes_back_as_it_was() {
        let cfg = RigctldConfig::default();
        let mut sess = Session::default();
        let mut send = |line: &str, s: &RigState| {
            let mut cmds = Vec::new();
            handle(line, s, &cfg, &mut sess, &mut cmds);
            cmds
        };
        let on = RigState { noise_reduction: true, nr_level: NrLevel::RnnHigh, ..st() };
        let off = RigState { noise_reduction: false, nr_level: NrLevel::Off, ..st() };
        assert_eq!(
            send("U NR 0", &on),
            vec![Command::SetNoiseReduction { rx: RxId::Main, level: NrLevel::Off }]
        );
        assert_eq!(
            send("U NR 1", &off),
            vec![Command::SetNoiseReduction { rx: RxId::Main, level: NrLevel::RnnHigh }]
        );
        assert!(send("L NR 0.3", &off).is_empty(), "a strength set while off is kept, not applied");
        assert_eq!(
            send("U NR 1", &off),
            vec![Command::SetNoiseReduction { rx: RxId::Main, level: NrLevel::RnnLow }]
        );
    }

    /// A client that never saw NR on still gets what `U NR 1` always gave.
    #[test]
    fn nr_on_from_nothing_is_the_old_default() {
        assert_eq!(
            run("U NR 1").1,
            vec![Command::SetNoiseReduction { rx: RxId::Main, level: NrLevel::Medium }]
        );
    }

    /// Every AGC setting we have survives the trip through Hamlib's numbers.
    #[test]
    fn agc_maps_both_ways() {
        for agc in AgcMode::ALL {
            assert_eq!(agc_from_hamlib(agc_to_hamlib(agc)), Some(agc));
        }
    }

    #[test]
    fn vfo_ops_map_to_commands() {
        assert_eq!(run("G XCHG").1, vec![Command::SwapVfos]);
        assert_eq!(run("G CPY").1, vec![Command::CopyAtoB]);
        assert_eq!(run("G TOGGLE").1, vec![Command::SelectVfo(Vfo::B)]);
        assert_eq!(run("G NOPE").0, "RPRT -11\n");
    }

    #[test]
    fn quit_closes_without_answering() {
        assert_eq!(run("q").0, "<close>");
        assert_eq!(run("Q").0, "<close>");
    }

    #[test]
    fn carriage_returns_are_tolerated() {
        assert_eq!(run("f\r\n").0, "14074000\n");
        assert_eq!(run("f\r").0, "14074000\n");
    }

    // ── \dump_state: the eight ways this breaks clients ─────────────────────

    #[test]
    fn dump_state_sentinels_have_the_exact_field_counts() {
        let d = run("\\dump_state").0;
        let seven = d.lines().filter(|l| *l == "0 0 0 0 0 0 0").count();
        assert_eq!(seven, 2, "one RX and one TX sentinel, seven fields each");
        let two = d.lines().filter(|l| *l == "0 0").count();
        assert_eq!(two, 2, "tuning-step and filter sentinels, two fields each");
    }

    #[test]
    fn dump_state_range_fields_parse_as_hamlib_reads_them() {
        let d = run("\\dump_state").0;
        let range = d.lines().nth(3).expect("first RX range");
        let f: Vec<&str> = range.split_whitespace().collect();
        assert_eq!(f.len(), 7);
        assert!(f[0].parse::<f64>().is_ok(), "start is %lf");
        assert!(f[1].parse::<f64>().is_ok(), "end is %lf");
        assert!(f[2].starts_with("0x"), "modes are %llx");
        assert!(f[3].parse::<i32>().is_ok(), "powers are plain %d");
        assert!(f[5].starts_with("0x") && f[6].starts_with("0x"), "vfo/ant are %x");
    }

    /// An empty RX list leaves Hamlib's mode_list at zero, after which it
    /// rejects every set_mode as unsupported.
    #[test]
    fn dump_state_always_lists_at_least_one_rx_range() {
        let bare = RigState { rx_ranges: Vec::new(), ..RigState::default() };
        let d = run_with("\\dump_state", &bare, &RigctldConfig::default()).0;
        assert_ne!(d.lines().nth(3), Some("0 0 0 0 0 0 0"));
    }

    /// Protocol version 1 without the trailing `done` is the "read failed
    /// after N seconds" hang.
    #[test]
    fn dump_state_declares_version_one_and_terminates_it() {
        let d = run("\\dump_state").0;
        assert_eq!(d.lines().next(), Some("1"));
        assert_eq!(d.lines().last(), Some("done"));
    }

    #[test]
    fn dump_state_lists_preamp_and_attenuator_even_when_empty() {
        let d = run("\\dump_state").0;
        let after_filters = d.split("0 0\n").last().unwrap();
        let head: Vec<&str> = after_filters.lines().take(6).collect();
        assert_eq!(head, vec!["9999", "9999", "0", "0", "0", "0"]);
    }

    #[test]
    fn dump_state_carries_the_expected_tail() {
        let d = run("\\dump_state").0;
        for key in [
            "vfo_ops=",
            "ptt_type=",
            "targetable_vfo=",
            "has_set_vfo=1",
            "has_get_freq=1",
            "rig_model=2",
            "rigctld_version=",
        ] {
            assert!(d.contains(key), "missing {key}");
        }
    }

    #[test]
    fn dump_state_uses_only_newlines() {
        assert!(!run("\\dump_state").0.contains('\r'));
    }
}
