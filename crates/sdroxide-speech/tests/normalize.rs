//! Every rule in `text`, as exact strings, plus the invariant that guards
//! them all.

use sdroxide_speech::text::Speaker;
use sdroxide_types::{
    AgcMode, Band, CallsignStyle, FreqStyle, Meters, Mode, NrLevel, SegmentKind, SpeechSettings,
};

fn cfg() -> SpeechSettings {
    SpeechSettings::default()
}

/// `Meters` has no `Default` — it is always built from a real measurement.
fn meters(s_dbm: f32) -> Meters {
    Meters {
        s_dbm,
        pa_temp_c: None,
        adc_peak_dbfs: -30.0,
        adc_clip: 0.0,
        tx: None,
        stereo: false,
        tone: None,
    }
}

#[test]
fn frequencies_read_like_a_dial() {
    let c = cfg();
    let sp = Speaker::new(&c);
    for (hz, want) in [
        (14_074_000.0, "fourteen point zero seven four"),
        (7_074_000.0, "seven point zero seven four"),
        (14_200_000.0, "fourteen point two zero zero"),
        (145_500_000.0, "one hundred forty five point five zero zero"),
        // Sub-kilohertz digits ride in a second group so the ear can hold it.
        (7_142_350.0, "seven point one four two, three five zero"),
        // ...and all three of them, so 350 Hz and 35 Hz do not read alike.
        (7_142_035.0, "seven point one four two, zero three five"),
        // Below a megahertz: kilohertz, as those frequencies are published.
        (810_000.0, "eight hundred ten kilohertz"),
        (198_000.0, "one hundred ninety eight kilohertz"),
    ] {
        assert_eq!(sp.freq(hz), want, "freq({hz})");
    }
}

#[test]
fn frequency_styles() {
    let mut c = cfg();
    c.freq_style = FreqStyle::Full;
    assert_eq!(Speaker::new(&c).freq(14_074_000.0), "fourteen point zero seven four megahertz");

    c.freq_style = FreqStyle::Digits;
    assert_eq!(Speaker::new(&c).freq(14_074_000.0), "one four zero seven four kilohertz");
}

#[test]
fn swr_values() {
    let c = cfg();
    let sp = Speaker::new(&c);
    assert_eq!(sp.swr(1.0), "one to one");
    assert_eq!(sp.swr(1.4), "one point four to one");
    assert_eq!(sp.swr(2.95), "three to one");
    // Bridges peg; a precise reading up there is not information.
    assert_eq!(sp.swr(9.9), "over ten to one");
    assert_eq!(sp.swr(40.0), "over ten to one");
    assert_eq!(sp.swr(f32::INFINITY), "over ten to one");
}

#[test]
fn levels_and_units() {
    let c = cfg();
    let sp = Speaker::new(&c);
    assert_eq!(sp.percent(0.35), "thirty five percent");
    assert_eq!(sp.percent(0.0), "zero percent");
    assert_eq!(sp.percent(1.0), "one hundred percent");
    assert_eq!(sp.watts(1.0), "one watt");
    assert_eq!(sp.watts(20.0), "twenty watts");
    assert_eq!(sp.watts(0.5), "zero point five watts");
    assert_eq!(sp.watts(1500.0), "one point five kilowatts");
    assert_eq!(sp.decibels(-6.0), "minus six decibels");
    assert_eq!(sp.dbm(-73.0), "minus seventy three d B m");
    assert_eq!(sp.hertz(-250), "minus two hundred fifty hertz");
}

#[test]
fn s_meter_readings() {
    let c = cfg();
    let sp = Speaker::new(&c);
    assert_eq!(sp.s_meter(&meters(-73.0)), "S nine");
    assert_eq!(sp.s_meter(&meters(-61.0)), "S nine plus twelve");
    assert_eq!(sp.s_meter(&meters(-97.0)), "S five");
}

#[test]
fn callsigns_in_every_style() {
    let mut c = cfg();
    assert_eq!(Speaker::new(&c).callsign("K1ABC"), "kilo one alpha bravo charlie");
    assert_eq!(
        Speaker::new(&c).callsign("DL/K1ABC"),
        "delta lima stroke kilo one alpha bravo charlie"
    );
    c.callsign_style = CallsignStyle::Letters;
    assert_eq!(Speaker::new(&c).callsign("K1ABC"), "kay one ay bee see");
    c.callsign_style = CallsignStyle::AsIs;
    // Letters as written, but the digit still becomes a word: leaving it a
    // digit would not read it faster, it would drop it.
    assert_eq!(Speaker::new(&c).callsign("K1ABC"), "k one abc");
}

#[test]
fn grids_and_reports() {
    let c = cfg();
    let sp = Speaker::new(&c);
    assert_eq!(sp.grid("JN47"), "juliett november four seven");
    assert_eq!(sp.grid("JN47ab"), "juliett november four seven alpha bravo");
    assert_eq!(sp.snr(-12), "minus twelve");
    assert_eq!(sp.snr(3), "plus three");
    assert_eq!(sp.rst("599"), "five nine nine");
    // Cut numbers are what was meant, not what was sent.
    assert_eq!(sp.rst("5NN"), "five nine nine");
}

#[test]
fn modes_are_letters_not_words() {
    let c = cfg();
    let sp = Speaker::new(&c);
    // What an operator says on the air. "upper sideband" is for the log.
    assert_eq!(sp.mode(Mode::Usb), "U S B");
    assert_eq!(sp.mode(Mode::Cw), "C W");
    assert_eq!(sp.mode(Mode::Ft8), "F T eight");
    // The two nobody spells out.
    assert_eq!(sp.mode(Mode::Rtty), "ritty");
    assert_eq!(sp.mode(Mode::Hell), "hell");
}

#[test]
fn bands_and_settings() {
    let c = cfg();
    let sp = Speaker::new(&c);
    assert_eq!(sp.band(Band::M20), "twenty meters");
    assert_eq!(sp.band(Band::M160), "one sixty meters");
    assert_eq!(sp.band(Band::M70), "seventy centimeters");
    assert_eq!(sp.band(Band::Gen), "general coverage");
    assert_eq!(sp.agc(AgcMode::Med), "A G C medium");
    assert_eq!(sp.nr(NrLevel::DfHigh), "deep filter high");
    assert_eq!(sp.segment(Some(SegmentKind::Digi)), "data");
    assert_eq!(sp.segment(None), "");
}

#[test]
fn free_text_abbreviations() {
    let c = cfg();
    let sp = Speaker::new(&c);
    assert_eq!(
        sp.message("CQ CQ DE K1ABC K1ABC K"),
        "C Q C Q from kilo one alpha bravo charlie kilo one alpha bravo charlie over"
    );
    assert_eq!(sp.message("RR73"), "roger roger seventy three");
    assert_eq!(sp.message("TU 5NN 73"), "thank you five nine nine seventy three");
    assert_eq!(
        sp.message("K1ABC W3XYZ -12"),
        "kilo one alpha bravo charlie whiskey three xray yankee zulu minus twelve"
    );
    assert_eq!(sp.message("UR RST 599 ES 73"), "your R S T five nine nine and seventy three");
}

#[test]
fn free_text_falls_back_sensibly() {
    let c = cfg();
    let sp = Speaker::new(&c);
    // Grid, signed integer, initialism, ordinary word.
    assert_eq!(sp.message("JN47"), "juliett november four seven");
    assert_eq!(sp.message("+03"), "plus three");
    assert_eq!(sp.message("SSB"), "S S B");
    assert_eq!(sp.message("hello"), "hello");
    // A capital on its own is a letter to spell, not a word — the same
    // convention as "U S B" and "C W".
    assert_eq!(sp.message("K test"), "K test");
}

/// A RTTY weather bulletin, which is what most of the RTTY band actually is.
///
/// Baudot has no lower case, so capitalisation cannot be the signal for "spell
/// this one out" — under the old rule every short word in this line was read
/// letter by letter.
#[test]
fn a_weather_bulletin_is_read_as_words() {
    let c = cfg();
    let sp = Speaker::new(&c);
    assert_eq!(
        sp.message("GOOD MORNING PSE QSL 10100 KHZ"),
        "good morning please Q S L ten thousand one hundred kilohertz"
    );
    // Units and quantities welded together, the way a bulletin writes them.
    assert_eq!(sp.message("1013 HPA"), "one thousand thirteen hectopascals");
    assert_eq!(sp.message("10100KHZ"), "ten thousand one hundred kilohertz");
    assert_eq!(sp.message("WIND SW 5 TO 6"), "wind southwest five to six");
    // A leading zero is a group, not a quantity: half past five, not 530 —
    // welded to its unit or standing on its own.
    assert_eq!(sp.message("0530UTC"), "zero five three zero U T C");
    assert_eq!(sp.message("AT 0530 UTC"), "at zero five three zero U T C");
    // The station's own callsigns still get spelled — that is the whole point
    // of the exercise, and DWD signs with three of them.
    assert_eq!(sp.message("DDK2"), "delta delta kilo two");
    assert_eq!(sp.message("DDH7 DDK9"), "delta delta hotel seven delta delta kilo nine");
}

/// The separator between bulletins: over a minute of "romeo yankee" if it is
/// read, and nothing lost if it is not.
#[test]
fn the_teleprinter_idle_is_not_read() {
    let c = cfg();
    let sp = Speaker::new(&c);
    assert_eq!(sp.message("RYRYRYRYRYRYRYRYRYRYRYRYRYRYRYRY"), "");
    assert_eq!(sp.message("RYRYRY DDK2 RYRYRY"), "delta delta kilo two");
    // An empty message is what tells `TextPump` there is nothing to queue.
    assert_eq!(sp.message("RY RY RY"), "");
}

/// Real English words that are not initialisms, and initialisms that are.
#[test]
fn spelling_is_earned_not_assumed() {
    let c = cfg();
    let sp = Speaker::new(&c);
    // Unpronounceable: an initialism whatever the table knows about it.
    assert_eq!(sp.message("SSB DWD"), "S S B D W D");
    // The whole Q-code family, not just the listed ones.
    assert_eq!(sp.message("QRK QSA"), "Q R K Q S A");
    // Pronounceable, so a word — the dictionary downstream decides how it
    // sounds, and spells it only if it has never seen it.
    assert_eq!(sp.message("GOOD SEA GALE"), "good sea gale");
    // Shorthand welded to a sign-off number is shorthand, not a station.
    assert_eq!(sp.message("TNX73"), "thanks seventy three");
    // A frequency in free text used to vanish at the sanitizer.
    assert_eq!(sp.message("QSY 14.074"), "Q S Y fourteen point zero seven four");
}

/// The invariant the whole module exists to keep.
///
/// Piper's phonemizer is a dictionary lookup: a token it does not know is
/// *dropped*, not mispronounced. So a digit or a symbol that survives
/// normalisation is a silently missing word — in a frequency read-out,
/// indistinguishable from a correct one until the operator finds themselves on
/// the wrong band. Sweep every rule and assert nothing unspeakable escapes.
#[test]
fn every_rule_honours_the_output_contract() {
    let mut c = cfg();
    let mut out: Vec<String> = Vec::new();

    for style in [FreqStyle::Dial, FreqStyle::Digits, FreqStyle::Full] {
        c.freq_style = style;
        let sp = Speaker::new(&c);
        for hz in [810_000.0, 1_840_000.0, 7_142_350.0, 14_074_320.0, 145_500_000.0, 432_174_000.0]
        {
            out.push(sp.freq(hz));
        }
    }
    c.freq_style = FreqStyle::Dial;

    for cs in CallsignStyle::ALL {
        c.callsign_style = cs;
        let sp = Speaker::new(&c);
        for call in ["K1ABC", "DL/K1ABC", "9A1AAA", "VP2E/W3XYZ", "OE3ABC"] {
            out.push(sp.callsign(call));
        }
        for g in ["JN47", "JN47ab"] {
            out.push(sp.grid(g));
        }
        for msg in [
            "CQ DX K1ABC JN47",
            "K1ABC W3XYZ RR73",
            "TU 5NN 73",
            "DL/9A1AAA -12 +03 100%",
            "QSY 14.074 pse",
            "wx here 21C hi hi",
            // The bulletin shapes: a compound token that leaks a digit past
            // the contract is a frequency the operator never hears.
            "DDK2 10100KHZ 1013HPA 0530UTC RYRYRYRY",
        ] {
            out.push(sp.message(msg));
        }
    }
    c.callsign_style = CallsignStyle::Phonetic;

    let sp = Speaker::new(&c);
    // Exhaustive `match`es upstream mean a new enum variant is a compile
    // error; sweeping ALL here means it is also covered by this test.
    for m in Mode::ALL {
        out.push(sp.mode(m).to_string());
    }
    for b in Band::ALL {
        out.push(sp.band(b).to_string());
    }
    for a in AgcMode::ALL {
        out.push(sp.agc(a));
    }
    for n in [
        NrLevel::Off,
        NrLevel::Low,
        NrLevel::Medium,
        NrLevel::High,
        NrLevel::RnnLow,
        NrLevel::RnnMed,
        NrLevel::RnnHigh,
        NrLevel::SpecLow,
        NrLevel::SpecMed,
        NrLevel::SpecHigh,
        NrLevel::DfLow,
        NrLevel::DfMed,
        NrLevel::DfHigh,
    ] {
        out.push(sp.nr(n));
    }
    for k in [SegmentKind::Cw, SegmentKind::Digi, SegmentKind::Phone, SegmentKind::Beacon] {
        out.push(sp.segment(Some(k)).to_string());
    }
    for v in [1.0f32, 1.43, 2.95, 9.9, 40.0, f32::INFINITY] {
        out.push(sp.swr(v));
    }
    for w in [0.5f32, 1.0, 5.0, 100.0, 1500.0] {
        out.push(sp.watts(w));
    }
    for db in [-120.0f32, -6.0, 0.0, 90.0] {
        out.push(sp.decibels(db));
        out.push(sp.dbm(db));
    }
    for f in [0.0f32, 0.35, 1.0] {
        out.push(sp.percent(f));
    }
    for hz in [-250i32, 0, 9999] {
        out.push(sp.hertz(hz));
    }
    for db in [-24i16, -1, 0, 20] {
        out.push(sp.snr(db));
    }
    for r in ["599", "5NN", "339"] {
        out.push(sp.rst(r));
    }
    for dbm in [-121.0f32, -73.0, -50.0] {
        out.push(sp.s_meter(&meters(dbm)));
    }

    for s in &out {
        assert!(
            s.chars().all(|ch| ch.is_ascii_alphabetic() || matches!(ch, ' ' | ',' | '.' | '\'')),
            "unspeakable output: {s:?}"
        );
    }
    // An empty string is a silently missing announcement; only `segment(None)`
    // is allowed to be empty, and it is not in this sweep.
    for s in &out {
        assert!(!s.trim().is_empty(), "empty output in the sweep");
    }
}

#[test]
fn sanitize_is_the_last_line_of_defence() {
    // Whatever a rule forgets, this is what the phonemizer would have seen.
    assert_eq!(Speaker::sanitize("14.074 MHz"), "MHz");
    assert_eq!(Speaker::sanitize("split  on"), "split on");
    assert_eq!(Speaker::sanitize("  a, b.  "), "a, b.");
    assert_eq!(Speaker::sanitize("don't"), "don't");
    assert_eq!(Speaker::sanitize("100%"), "");
}
