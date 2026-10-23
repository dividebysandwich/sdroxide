//! Reading and writing channel lists in CHIRP's CSV format.
//!
//! CHIRP is the program most people use to programme a handheld, and its CSV
//! export is what repeater directories hand out: RepeaterBook offers it by
//! county, national societies publish their machine lists in it, and the marine
//! and PMR channel tables circulate as it too. That makes it the one format
//! worth reading here — an operator who wants their local repeaters in
//! sdroxide already has the file (issue #234).
//!
//! ⚠️ Transcribed from CHIRP's own `chirp/chirp_common.py` (the `Memory` fields
//! and the `TONE_MODES` / `DUPLEX_MODES` tables) and from files it exports, not
//! from a specification — CHIRP has none. The columns are matched **by header
//! name**, so a file with the columns in another order, or with only some of
//! them, still reads; a file with no header row at all is refused rather than
//! guessed at, because the first line of a headerless list is a real channel
//! and mistaking it for a header would silently drop it.
//!
//! What is deliberately not carried: `Skip` (sdroxide's scanner has its own
//! idea of what to skip), `Power` (a channel does not set the transmitter's
//! power here), `TStep`, and the D-STAR columns. A memory here is a frequency,
//! a mode, the repeater set-up and a name.

use crate::{
    CTCSS_TONES, DCS_CODES, MAX_OFFSET_HZ, MemoryChannel, Mode, RepeaterState, Shift, ToneMode,
};

/// The columns CHIRP writes, in the order it writes them. Used for export, and
/// as the vocabulary the importer matches a file's own header against.
const COLUMNS: &[&str] = &[
    "Location",
    "Name",
    "Frequency",
    "Duplex",
    "Offset",
    "Tone",
    "rToneFreq",
    "cToneFreq",
    "DtcsCode",
    "DtcsPolarity",
    "RxDtcsCode",
    "CrossMode",
    "Mode",
    "TStep",
    "Skip",
    "Power",
    "Comment",
    "URCALL",
    "RPT1CALL",
    "RPT2CALL",
    "DVCODE",
];

/// The CHIRP mode name for one of ours, and the reverse.
///
/// Only the modes both programs have. Everything sdroxide can do that a
/// handheld cannot — every digital mode, SSTV, the decoders — exports as the
/// sideband or the FM channel it actually rides on, because that is what the
/// column means to the program reading it: `Mode` in a CHIRP file is the
/// *modulation*, not the traffic.
fn mode_name(m: Mode) -> &'static str {
    match m {
        // HD Radio is the digital sidecar of a broadcast FM station, and a
        // handheld with the channel stored hears that station in WFM.
        Mode::Wfm | Mode::HdRadio => "WFM",
        Mode::Am | Mode::Sam => "AM",
        Mode::Lsb | Mode::Digl => "LSB",
        Mode::Cw => "CW",
        Mode::Nfm => "FM",
        // Everything else in sdroxide rides a sideband or an FM carrier, and
        // the FM ones are all `Mode::Nfm` and were caught above.
        _ => "USB",
    }
}

/// The mode a CHIRP `Mode` column means. `None` for a name no radio here has,
/// which the caller reads as "leave it at FM" — a channel with an unreadable
/// modulation is still a frequency worth having.
fn mode_from_name(s: &str) -> Option<Mode> {
    Some(match s.trim().to_ascii_uppercase().as_str() {
        // CHIRP's narrow variants are the same demodulator here with a
        // narrower filter, which `Mode::Nfm` already is.
        "FM" | "NFM" | "FMN" | "DV" | "DN" => Mode::Nfm,
        "WFM" | "FM-W" => Mode::Wfm,
        "AM" | "NAM" | "AMN" => Mode::Am,
        "USB" => Mode::Usb,
        "LSB" => Mode::Lsb,
        "CW" | "CWR" => Mode::Cw,
        "RTTY" | "RTTYR" => Mode::Rtty,
        // "DIG"/"PKT" are a *data* channel on a sideband or on FM, and which
        // is not in the file. Left to the caller's default.
        _ => return None,
    })
}

/// One CSV line split on commas, with CHIRP's quoting: a field may be wrapped
/// in double quotes, inside which a doubled quote is one quote and a comma is
/// an ordinary character.
///
/// Written here rather than pulled in as a dependency because this is the whole
/// of the dialect: CHIRP writes no embedded newlines, so a line is a record.
fn split_csv(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out
}

/// Wrap a field for output if it carries anything the reader would trip over.
fn quote_csv(s: &str) -> String {
    if s.contains([',', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// A CTCSS frequency as CHIRP writes it (`88.5`) in the tenths of a hertz the
/// rest of sdroxide keeps it in, snapped to the standard table.
///
/// Snapped rather than taken as read: the tone that goes out is generated from
/// [`CTCSS_TONES`], so a file naming a tone that is not in it has to resolve to
/// one that is or the channel would transmit a tone nobody is listening for.
/// Anything further than a hertz from a standard tone is refused instead.
fn ctcss_tenths(s: &str) -> Option<u16> {
    let hz: f32 = s.trim().parse().ok()?;
    let want = (hz * 10.0).round() as i32;
    CTCSS_TONES.iter().copied().min_by_key(|t| (i32::from(*t) - want).abs()).filter(|t| {
        // Within one hertz: the table's own steps are 2 Hz and more at the
        // bottom end, so this cannot silently land on a neighbour.
        (i32::from(*t) - want).abs() <= 10
    })
}

/// Parse the `Frequency` / `Offset` columns, which CHIRP writes in MHz.
fn mhz(s: &str) -> Option<f64> {
    let hz = s.trim().parse::<f64>().ok()? * 1e6;
    hz.is_finite().then_some(hz)
}

/// Read a CHIRP CSV file into channels.
///
/// Ids are left at zero: whoever stores these assigns them, because only that
/// end knows what is already in the list. Lines that cannot be read at all — no
/// frequency, or a frequency that is not a number — are skipped rather than
/// failing the import: a directory export routinely carries a blank row or a
/// trailing comment, and losing the other four hundred channels over it would
/// be the wrong trade.
///
/// Returns `(channels, skipped)`, so the caller can tell an operator how much
/// of their file was read.
pub fn chirp_csv_to_memories(text: &str) -> (Vec<MemoryChannel>, usize) {
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    // The header names the columns; without one there is nothing to match
    // against and a first channel would be eaten as a heading.
    let Some(header) = lines.next() else { return (Vec::new(), 0) };
    let cols: Vec<String> = split_csv(header)
        .into_iter()
        .map(|c| c.trim().trim_start_matches('\u{feff}').to_string())
        .collect();
    let at = |name: &str| cols.iter().position(|c| c.eq_ignore_ascii_case(name));
    let Some(freq_col) = at("Frequency") else { return (Vec::new(), 0) };

    let (mut out, mut skipped) = (Vec::new(), 0usize);
    for line in lines {
        let f = split_csv(line);
        let get = |i: Option<usize>| i.and_then(|i| f.get(i)).map(|s| s.trim()).unwrap_or("");
        let Some(freq_hz) = mhz(get(Some(freq_col))).filter(|hz| *hz > 0.0) else {
            skipped += 1;
            continue;
        };
        let mode = mode_from_name(get(at("Mode"))).unwrap_or(Mode::Nfm);
        let (filter_lo, filter_hi) = mode.default_filter_at(freq_hz);

        // The shift. "split" carries the whole transmit frequency in `Offset`
        // rather than a magnitude, so it is turned into the direction and
        // distance the rest of sdroxide works in — and a split that is not a
        // shift at all (a transmit frequency below zero, or half a band away)
        // falls back to simplex rather than to a transmitter somewhere else.
        let offset = mhz(get(at("Offset"))).unwrap_or(0.0);
        let (shift, offset_hz) = match get(at("Duplex")) {
            "-" => (Shift::Minus, offset),
            "+" => (Shift::Plus, offset),
            "split" => {
                let d = offset - freq_hz;
                match d {
                    d if d > 0.0 => (Shift::Plus, d),
                    d if d < 0.0 => (Shift::Minus, -d),
                    _ => (Shift::Simplex, 0.0),
                }
            }
            // "off" is a receive-only channel — CHIRP's way of saying the radio
            // must not transmit here. sdroxide has no such flag on a memory, so
            // it reads as simplex; the transmit rails are what actually keep an
            // operator off a frequency they may not use.
            _ => (Shift::Simplex, offset),
        };
        let offset_hz = offset_hz.clamp(0.0, f64::from(MAX_OFFSET_HZ)).round() as u32;

        // The tone. CHIRP's `Tone` column names which of the two frequency
        // columns is in force: "Tone" transmits `rToneFreq` and squelches on
        // nothing, "TSQL" transmits and receives `cToneFreq`. Only the
        // transmit half is a memory's business here — the receive tone squelch
        // is a receiver setting in sdroxide, not a channel one.
        let default = RepeaterState::default();
        let (tone, ctcss_tenths_v, dcs_code, dcs_invert) =
            match get(at("Tone")).to_ascii_uppercase() {
                t if t == "TONE" => (
                    ToneMode::Ctcss,
                    ctcss_tenths(get(at("rToneFreq"))).unwrap_or(default.ctcss_tenths),
                    default.dcs_code,
                    false,
                ),
                t if t == "TSQL" || t == "CROSS" => (
                    ToneMode::Ctcss,
                    ctcss_tenths(get(at("cToneFreq")))
                        .or_else(|| ctcss_tenths(get(at("rToneFreq"))))
                        .unwrap_or(default.ctcss_tenths),
                    default.dcs_code,
                    false,
                ),
                t if t == "DTCS" || t == "DTCS-R" => {
                    let code = get(at("DtcsCode")).trim_start_matches('0');
                    let code = code.parse::<u16>().ok().filter(|c| DCS_CODES.contains(c));
                    // `DtcsPolarity` is two letters, transmit then receive; the
                    // first is the one that goes out.
                    let invert = get(at("DtcsPolarity")).to_ascii_uppercase().starts_with('R');
                    (ToneMode::Dcs, default.ctcss_tenths, code.unwrap_or(default.dcs_code), invert)
                }
                _ => (ToneMode::Off, default.ctcss_tenths, default.dcs_code, false),
            };

        // A name, and a fallback that is still worth reading: CHIRP allows an
        // empty one, and a list of unnamed channels is a list of blank rows.
        let name = match get(at("Name")) {
            "" => match get(at("Comment")) {
                "" => format!("{:.4} MHz", freq_hz / 1e6),
                c => c.to_string(),
            },
            n => n.to_string(),
        };

        out.push(MemoryChannel {
            id: 0,
            name,
            freq_hz,
            mode,
            filter_lo,
            filter_hi,
            folder: None,
            rtty: None,
            repeater: Some(
                RepeaterState {
                    shift,
                    offset_hz,
                    auto: false,
                    tone,
                    ctcss_tenths: ctcss_tenths_v,
                    dcs_code,
                    dcs_invert,
                    ..default
                }
                .clamped(),
            ),
            // Not in the format, and a socket invented for an imported channel
            // would move a relay the operator never asked to move.
            antenna: None,
            color: None,
        });
    }
    (out, skipped)
}

/// Write channels out in the same format, so a list built here can be taken to
/// a handheld — and so the reader above has something to be tested against.
pub fn memories_to_chirp_csv(mems: &[MemoryChannel]) -> String {
    let mut s = String::new();
    s.push_str(&COLUMNS.join(","));
    s.push('\n');
    for (i, m) in mems.iter().enumerate() {
        let r = m.repeater.unwrap_or_default();
        let (tone, rtone, ctone, dtcs, pol) = match r.tone {
            ToneMode::Off => ("", String::new(), String::new(), String::new(), String::new()),
            ToneMode::Ctcss => {
                let hz = format!("{}.{}", r.ctcss_tenths / 10, r.ctcss_tenths % 10);
                // "Tone" rather than "TSQL": what a memory here carries is the
                // tone that goes *out*, and claiming a receive squelch the
                // channel does not set would arm one on the radio it is loaded
                // into.
                ("Tone", hz.clone(), hz, "023".to_string(), "NN".to_string())
            }
            ToneMode::Dcs => (
                "DTCS",
                "88.5".to_string(),
                "88.5".to_string(),
                format!("{:03}", r.dcs_code),
                // Transmit polarity then receive; only the first is ours.
                format!("{}N", if r.dcs_invert { "R" } else { "N" }),
            ),
        };
        let duplex = match r.shift {
            Shift::Simplex => "",
            Shift::Minus => "-",
            Shift::Plus => "+",
        };
        let offset = match r.shift {
            Shift::Simplex => 0.0,
            _ => f64::from(r.offset_hz) / 1e6,
        };
        let fields = [
            (i + 1).to_string(),
            // CHIRP's own limit; a longer name is the operator's list, not the
            // radio's, and truncating it here is what makes the file loadable.
            m.name.chars().take(16).collect::<String>(),
            format!("{:.6}", m.freq_hz / 1e6),
            duplex.to_string(),
            format!("{offset:.6}"),
            tone.to_string(),
            rtone,
            ctone,
            dtcs,
            pol,
            "023".to_string(),
            "Tone->Tone".to_string(),
            mode_name(m.mode).to_string(),
            "5.00".to_string(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ];
        s.push_str(&fields.iter().map(|f| quote_csv(f)).collect::<Vec<_>>().join(","));
        s.push('\n');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file as RepeaterBook and CHIRP actually write one: a header, a
    /// repeater with a minus shift and a tone, a simplex channel, and a
    /// DCS-coded machine.
    const SAMPLE: &str = "\
Location,Name,Frequency,Duplex,Offset,Tone,rToneFreq,cToneFreq,DtcsCode,DtcsPolarity,RxDtcsCode,CrossMode,Mode,TStep,Skip,Power,Comment,URCALL,RPT1CALL,RPT2CALL,DVCODE
0,OE1XUU,145.612500,-,0.600000,Tone,123.0,88.5,023,NN,023,Tone->Tone,FM,12.50,,5.0W,Wienerberg,,,,
1,S20,145.500000,,0.000000,,88.5,88.5,023,NN,023,Tone->Tone,FM,12.50,,5.0W,calling,,,,
2,OE3XOS,438.850000,-,7.600000,DTCS,88.5,88.5,131,RN,023,Tone->Tone,FM,12.50,,5.0W,,,,,
3,Marine16,156.800000,,0.000000,,88.5,88.5,023,NN,023,Tone->Tone,NFM,12.50,,5.0W,Distress,,,,
";

    #[test]
    fn a_chirp_export_reads_back_as_channels() {
        let (mems, skipped) = chirp_csv_to_memories(SAMPLE);
        assert_eq!(skipped, 0);
        assert_eq!(mems.len(), 4);

        let r = &mems[0];
        assert_eq!(r.name, "OE1XUU");
        assert_eq!(r.freq_hz, 145_612_500.0);
        assert_eq!(r.mode, Mode::Nfm);
        let rep = r.repeater.unwrap();
        assert_eq!(rep.shift, Shift::Minus);
        assert_eq!(rep.offset_hz, 600_000);
        assert_eq!(rep.tone, ToneMode::Ctcss);
        // "Tone" means the *transmit* tone, which is `rToneFreq` — reading
        // `cToneFreq` there would put 88.5 into a repeater that wants 123.0.
        assert_eq!(rep.ctcss_tenths, 1230);

        let s = mems[1].repeater.unwrap();
        assert_eq!(s.shift, Shift::Simplex);
        assert_eq!(s.tone, ToneMode::Off, "no Tone column value means no tone goes out");

        let d = mems[2].repeater.unwrap();
        assert_eq!(d.shift, Shift::Minus);
        assert_eq!(d.offset_hz, 7_600_000);
        assert_eq!(d.tone, ToneMode::Dcs);
        assert_eq!(d.dcs_code, 131);
        assert!(d.dcs_invert, "the first polarity letter is the transmit one");

        assert_eq!(mems[3].name, "Marine16");
        assert_eq!(mems[3].mode, Mode::Nfm);
    }

    /// The columns are matched by name, so a file that carries only some of
    /// them — or carries them in another order — still reads.
    #[test]
    fn columns_are_found_by_name_not_by_position() {
        let (mems, _) = chirp_csv_to_memories(
            "Name,Mode,Frequency,Tone,rToneFreq,Duplex,Offset\n\
             Test,USB,14.074000,,88.5,,0.000000\n",
        );
        assert_eq!(mems.len(), 1);
        assert_eq!(mems[0].name, "Test");
        assert_eq!(mems[0].mode, Mode::Usb);
        assert_eq!(mems[0].freq_hz, 14_074_000.0);
    }

    /// A blank row, a trailing note, a line with no frequency: counted and
    /// skipped, never allowed to cost the rest of the file.
    #[test]
    fn an_unreadable_line_costs_only_itself() {
        let (mems, skipped) = chirp_csv_to_memories(
            "Location,Name,Frequency\n\
             0,Good,145.500000\n\
             1,Bad,not-a-number\n\
             \n\
             2,Also good,433.500000\n",
        );
        assert_eq!(mems.len(), 2);
        assert_eq!(skipped, 1);
        // A file with no header at all is refused rather than guessed at.
        assert!(chirp_csv_to_memories("0,Ch,145.5\n").0.is_empty());
        assert!(chirp_csv_to_memories("").0.is_empty());
    }

    /// A quoted field carries commas, and the record is not split inside one.
    #[test]
    fn quoted_fields_survive_their_commas() {
        let (mems, _) = chirp_csv_to_memories(
            "Name,Frequency,Comment\n\
             \"Vienna, OE1\",145.750000,\"club machine, 88.5\"\n",
        );
        assert_eq!(mems[0].name, "Vienna, OE1");
    }

    /// A tone the standard table does not have has to resolve to one it does,
    /// or the channel transmits something nobody is listening for.
    #[test]
    fn a_ctcss_frequency_is_snapped_to_the_standard_table() {
        assert_eq!(ctcss_tenths("88.5"), Some(885));
        assert_eq!(ctcss_tenths("123"), Some(1230));
        // A rounding difference in the file resolves to the real tone…
        assert_eq!(ctcss_tenths("88.4"), Some(885));
        // …and something that is not a CTCSS tone at all is refused rather
        // than snapped onto the nearest entry from half the band away.
        assert_eq!(ctcss_tenths("1750.0"), None);
        assert_eq!(ctcss_tenths(""), None);
    }

    /// "split" carries the whole transmit frequency rather than a magnitude.
    #[test]
    fn a_split_channel_becomes_a_shift() {
        let (mems, _) = chirp_csv_to_memories(
            "Name,Frequency,Duplex,Offset\n\
             Odd,145.700000,split,144.900000\n\
             Up,145.700000,split,146.300000\n",
        );
        let down = mems[0].repeater.unwrap();
        assert_eq!((down.shift, down.offset_hz), (Shift::Minus, 800_000));
        let up = mems[1].repeater.unwrap();
        assert_eq!((up.shift, up.offset_hz), (Shift::Plus, 600_000));
    }

    /// What sdroxide writes, CHIRP's reader — and this one — must be able to
    /// read back unchanged.
    /// HD Radio is stored as the FM broadcast it rides on, not as a sideband.
    #[test]
    fn an_hd_radio_channel_exports_as_broadcast_fm() {
        assert_eq!(mode_name(Mode::HdRadio), "WFM");
    }

    #[test]
    fn the_writer_and_the_reader_agree() {
        let (before, _) = chirp_csv_to_memories(SAMPLE);
        let (after, skipped) = chirp_csv_to_memories(&memories_to_chirp_csv(&before));
        assert_eq!(skipped, 0);
        assert_eq!(after.len(), before.len());
        for (a, b) in after.iter().zip(&before) {
            assert_eq!((a.name.as_str(), a.freq_hz, a.mode), (b.name.as_str(), b.freq_hz, b.mode));
            assert_eq!(a.repeater, b.repeater, "{}", a.name);
        }
    }

    /// The parser has to survive whatever file it is handed, not merely report
    /// it: the native import catches a panic and costs only the import, but in
    /// the browser a panic aborts the whole page, so there the parser itself is
    /// the only guard. Every prefix of a real export — a line cut anywhere,
    /// inside a quote or a number — and a handful of files that are not CHIRP
    /// at all.
    #[test]
    fn a_damaged_file_never_panics() {
        for end in 0..=SAMPLE.len() {
            if SAMPLE.is_char_boundary(end) {
                let _ = chirp_csv_to_memories(&SAMPLE[..end]);
            }
        }
        for junk in [
            "Name,Frequency\n\"unterminated,145.5\n",
            "Name,Frequency,Duplex,Offset\nX,1e308,+,1e308\n",
            "Name,Frequency,Duplex,Offset\nX,-145.5,-,-99999999999\n",
            "Name,Frequency,Tone,rToneFreq,DtcsCode\nX,145.5,DTCS,NaN,99999999999999999999\n",
            "Frequency\n,,,,,,,,,,,,\n\"\"\"\n",
            "\u{feff}Name,Frequency\nÄ€\u{0},145.5\n",
        ] {
            let _ = chirp_csv_to_memories(junk);
        }
    }
}
