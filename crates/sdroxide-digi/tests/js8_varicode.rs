//! JS8 message-layer primitives checked against JS8Call itself.
//!
//! The vectors in `js8_varicode_vectors.rs` were produced by linking JS8Call's
//! own `varicode.cpp` and running it (`tools/gen_js8_varicode_vectors.sh`), so
//! these assertions compare our implementation against the reference rather
//! than against a second reading of the reference. The unit tests inside
//! `js8::varicode` prove self-consistency; these prove agreement, and only the
//! second kind can catch a plausible misreading.

mod js8_varicode_vectors;

use js8_varicode_vectors::{CALLSIGNS, DIRECTED, GRIDS};
use sdroxide_digi::js8::frame::{Directed, cmd_code};
use sdroxide_digi::js8::varicode::{pack_callsign, pack_grid, unpack_callsign, unpack_grid};

#[test]
fn callsigns_pack_to_the_same_values_js8call_produces() {
    for (call, expect) in CALLSIGNS {
        let got = pack_callsign(call).unwrap_or_else(|| panic!("{call} did not pack"));
        assert_eq!(got, *expect, "{call}");
    }
}

#[test]
fn callsigns_unpack_back_to_themselves() {
    for (call, packed) in CALLSIGNS {
        assert_eq!(unpack_callsign(*packed, false).as_deref(), Some(*call), "{packed}");
    }
}

#[test]
fn grids_pack_to_the_same_values_js8call_produces() {
    for (grid, expect) in GRIDS {
        assert_eq!(pack_grid(grid), *expect, "{grid}");
    }
}

#[test]
fn grids_unpack_back_to_themselves() {
    for (grid, packed) in GRIDS {
        assert_eq!(unpack_grid(*packed).as_deref(), Some(*grid), "{packed}");
    }
}

#[test]
fn southern_hemisphere_grids_are_not_off_by_one() {
    // The bias-then-truncate ordering in `pack_grid` only matters below the
    // equator, so a port that got it wrong would pass every northern test.
    // QF22 (Melbourne) and PM95 (Tokyo) sit either side of it.
    for (grid, packed) in GRIDS.iter().filter(|(g, _)| ["QF22", "GF15", "RR99"].contains(g)) {
        assert_eq!(pack_grid(grid), *packed, "{grid}");
        assert_eq!(unpack_grid(*packed).as_deref(), Some(*grid), "{grid}");
    }
}

/// The whole directed-frame layout in one assertion: `[3 type][28 from]
/// [28 to][5 cmd][1][1][6 num]`, packed to twelve transport characters.
///
/// Anything wrong in the field widths, the field order, the command codes, the
/// number bias or the six-bit transport mapping moves at least one character.
#[test]
fn directed_frames_pack_exactly_as_js8call_packs_them() {
    let mut checked = 0;
    for (from, text, to, cmd, num, expect) in DIRECTED {
        // JS8Call reports an empty command for plain free text, which this
        // frame layout does not carry — that goes through the data frames.
        let Some(code) = cmd_code(cmd) else { continue };
        let d = Directed {
            from: (*from).into(),
            to: (*to).into(),
            cmd: code,
            num: num.trim().parse::<i32>().ok(),
            portable_from: false,
            portable_to: false,
        };
        let got = d.pack().unwrap_or_else(|| panic!("{from}: {text} did not pack"));
        assert_eq!(got.to_chars(), *expect, "{from}: {text}");
        checked += 1;
    }
    assert!(checked >= 10, "only {checked} vectors exercised");
}

#[test]
fn directed_frames_unpack_back_to_what_js8call_parsed() {
    for (from, text, to, cmd, num, expect) in DIRECTED {
        let Some(code) = cmd_code(cmd) else { continue };
        let payload = sdroxide_digi::js8::message::Js8Payload::from_chars(expect, 0)
            .unwrap_or_else(|| panic!("{expect} is not twelve transport characters"));
        let d = Directed::unpack(&payload).unwrap_or_else(|| panic!("{from}: {text}"));
        assert_eq!(d.from, *from, "{text}");
        assert_eq!(d.to, *to, "{text}");
        assert_eq!(d.cmd, code, "{text}");
        assert_eq!(d.num, num.trim().parse::<i32>().ok(), "{text}");
    }
}

/// Heartbeats use the compound layout: `[3 type][50 callsign][11 num_high]`
/// then `[5 num_low][3 sub]`. The 16-bit `num` straddling the 64/8 boundary is
/// the part most likely to be got wrong, and it only shows in the top bits.
#[test]
fn heartbeat_frames_pack_exactly_as_js8call_packs_them() {
    use js8_varicode_vectors::HEARTBEATS;
    use sdroxide_digi::js8::frame::Compound;

    for (from, text, expect) in HEARTBEATS {
        // The vector text is "@HB HEARTBEAT <grid>".
        let grid = text.split_whitespace().last().expect("grid");
        let hb = Compound::heartbeat(from, Some(grid));
        let got = hb.pack().unwrap_or_else(|| panic!("{from} {text} did not pack"));
        assert_eq!(got.to_chars(), *expect, "{from} {text}");
    }
}

#[test]
fn heartbeat_frames_unpack_back_to_callsign_and_grid() {
    use js8_varicode_vectors::HEARTBEATS;
    use sdroxide_digi::js8::frame::Compound;
    use sdroxide_digi::js8::message::Js8Payload;

    for (from, text, packed) in HEARTBEATS {
        let grid = text.split_whitespace().last().expect("grid");
        let payload = Js8Payload::from_chars(packed, 0).expect("twelve chars");
        let hb = Compound::unpack(&payload).unwrap_or_else(|| panic!("{packed}"));
        assert_eq!(hb.call, *from, "{packed}");
        assert_eq!(hb.grid().as_deref(), Some(grid), "{packed}");
        assert!(!hb.is_cq(), "a heartbeat is not a CQ");
    }
}

/// The Huffman table in `js8::huff` must be the reference table, entry for
/// entry. Forty-four rows of variable-length codes is exactly the sort of table
/// that gets copied one row wrong, and a single wrong code produces text that
/// is subtly corrupted rather than obviously broken.
#[test]
fn the_huffman_table_matches_js8calls() {
    use js8_varicode_vectors::HUFF as REFERENCE;
    use sdroxide_digi::js8::huff::HUFF as OURS;

    assert_eq!(OURS.len(), REFERENCE.len(), "table sizes differ");
    for (ch, code) in REFERENCE {
        let c = ch.chars().next().expect("one character");
        let ours = OURS
            .iter()
            .find(|(k, _)| *k == c)
            .unwrap_or_else(|| panic!("we have no code for {ch:?}"));
        assert_eq!(ours.1, *code, "code for {ch:?}");
    }
}

/// Free-text frames, against JS8Call's own `packDataMessage`.
///
/// This exercises the choice between the two codecs as well as both codecs
/// themselves: upstream keeps whichever carries more characters, so agreeing on
/// the frame means agreeing on the encoding *and* on which one to use.
#[test]
fn data_frames_pack_exactly_as_js8call_packs_them() {
    use js8_varicode_vectors::DATA_FRAMES;
    use sdroxide_digi::js8::frame::pack_data;

    for (text, expect_used, expect) in DATA_FRAMES {
        let (payload, used) = pack_data(text).unwrap_or_else(|| panic!("{text:?} did not pack"));
        assert_eq!(used as i32, *expect_used, "characters consumed for {text:?}");
        assert_eq!(payload.to_chars(), *expect, "{text:?}");
    }
}

#[test]
fn data_frames_unpack_back_to_their_text() {
    use js8_varicode_vectors::DATA_FRAMES;
    use sdroxide_digi::js8::frame::unpack_data;
    use sdroxide_digi::js8::message::Js8Payload;

    for (text, expect_used, packed) in DATA_FRAMES {
        let payload = Js8Payload::from_chars(packed, 0).expect("twelve chars");
        let got = unpack_data(&payload).unwrap_or_else(|| panic!("{packed}"));
        let want: String = text.chars().take(*expect_used as usize).collect();
        assert_eq!(got, want, "{text:?}");
    }
}

/// What we make of JS8Call's own transmissions to custom groups and compound
/// callsigns. Two bugs hid here:
/// * Every compound frame was read as a heartbeat, so a message to @MAGNET
///   arrived as "HB from @MAGNET".
/// * The reserved-group table stopped at @NTS, so a directed frame to @EMCOMM
///   did not unpack at all and was dropped.
#[test]
fn messages_js8call_frames_reassemble_with_the_right_sender_and_addressee() {
    use js8_varicode_vectors::MESSAGES;
    use sdroxide_digi::js8::assembler::Js8Assembler;
    use sdroxide_digi::js8::decode::Js8Decode;
    use sdroxide_digi::js8::message::Js8Payload;
    use sdroxide_types::Js8Speed;

    for (from, _grid, text, frames) in MESSAGES {
        let mut a = Js8Assembler::new("N0JDS");
        a.set_my_groups(vec!["@MAGNET".into()]);
        let mut got = None;
        for (i, f) in frames.split(' ').enumerate() {
            let (chars, flags) = f.split_once('/').expect("chars/flags");
            let payload = Js8Payload::from_chars(chars, flags.parse().expect("flags"))
                .unwrap_or_else(|| panic!("{text}: {chars} is not a frame"));
            let d = Js8Decode {
                payload,
                audio_hz: 1500.0,
                dt_sec: 0.0,
                snr_db: -8.0,
                sync_score: 3.0,
                hard_errors: 0,
                speed: Js8Speed::Normal,
            };
            got = a.push(&d, 1000 + 15 * i as i64);
        }
        let msg = got.unwrap_or_else(|| panic!("{text}: never completed"));
        let (to, rest) = text.split_once(' ').expect("addressed");
        assert_eq!(msg.from, *from, "{text}: sender");
        assert_eq!(msg.to, to, "{text}: addressee");
        assert!(msg.complete, "{text}");
        // The command (if any) and the text together say what was typed.
        let said = format!("{} {}", msg.cmd.as_deref().unwrap_or(""), msg.text);
        let norm = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(norm(&said), norm(rest), "{text}: content");
        let for_me = to == "N0JDS" || to == "@MAGNET";
        assert_eq!(msg.to_me, for_me, "{text}: to_me");
    }
}
