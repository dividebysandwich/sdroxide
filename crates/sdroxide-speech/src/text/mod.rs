//! Turning radio quantities into something a voice can say.
//!
//! # The output contract
//!
//! **Everything this module returns contains only ASCII letters, spaces,
//! commas, full stops and apostrophes.** No digits, no `-`, `+`, `%`, `/`, `:`,
//! and no `.` inside a number.
//!
//! That is not a style rule. The phonemizer downstream is a dictionary lookup,
//! and a dictionary has no entry for `120`, `-12`, `14.074` or `%`. An unknown
//! token is not mispronounced — it is *dropped*. A stray digit is therefore a
//! silently missing word, which in a frequency read-out is indistinguishable
//! from a correct one until the operator turns the dial and finds the wrong
//! band. [`Speaker::sanitize`] enforces the contract at the end of every method
//! here, and a test drives every rule through it.
//!
//! # Style
//!
//! Modes are read as letters — "U S B", not "upper sideband". That is what an
//! operator says on the air; "upper sideband" is what you write in a log. It is
//! also shorter, which matters when it rides on the end of every band change.
//! The exceptions are the ones nobody spells: `RTTY` is "ritty" and `HELL` is
//! "hell".
//!
//! Digits after a decimal point are read individually — "point zero seven
//! four", never "point seventy-four thousand" — and zero is always "zero",
//! never "oh".

pub mod abbrev;
pub mod filler;
pub mod numbers;
pub mod phonetic;

use sdroxide_types::{
    AgcMode, Band, CallsignStyle, FreqStyle, Meters, Mode, NrLevel, SegmentKind, SpeechSettings,
    Verbosity,
};

use numbers::{cardinal, decimal1, decimals, digits};

/// Everything the operator hears, rendered against their preferences.
///
/// A zero-cost borrow, built per tick. The rules are methods rather than free
/// functions only so that the settings do not have to be threaded through
/// every call site.
#[derive(Clone, Copy)]
pub struct Speaker<'a> {
    pub cfg: &'a SpeechSettings,
}

impl<'a> Speaker<'a> {
    pub fn new(cfg: &'a SpeechSettings) -> Self {
        Speaker { cfg }
    }

    // ── frequency ───────────────────────────────────────────────────────

    /// Read a frequency the way an operator reads a dial.
    ///
    /// Above 1 MHz: megahertz, with the fraction read digit by digit —
    /// `14_074_000` becomes "fourteen point zero seven four". Anything below
    /// the kilohertz digits is read as a second group so the ear can hold it:
    /// `7_142_350` becomes "seven point one four two, three five zero".
    ///
    /// Below 1 MHz — longwave and the broadcast band — kilohertz, because that
    /// is how those frequencies are published.
    pub fn freq(&self, hz: f64) -> String {
        let s = match self.cfg.freq_style {
            FreqStyle::Digits => self.freq_digits(hz),
            FreqStyle::Dial => self.freq_dial(hz, false),
            FreqStyle::Full => self.freq_dial(hz, true),
        };
        Self::sanitize(&s)
    }

    fn freq_digits(&self, hz: f64) -> String {
        let khz = (hz / 1000.0).round() as i64;
        format!("{} kilohertz", digits(&khz.to_string()))
    }

    fn freq_dial(&self, hz: f64, units: bool) -> String {
        let hz = hz.max(0.0);
        if hz < 1_000_000.0 {
            // Longwave and the broadcast band are published in kilohertz, and
            // the unit is not optional down here: "eight hundred ten" alone
            // could be anything.
            return format!("{} kilohertz", decimals((hz / 1000.0) as f32, 3));
        }
        let mhz = (hz / 1_000_000.0).trunc() as i64;
        // Remaining Hz below the megahertz digit, as six digits.
        let rest = (hz - mhz as f64 * 1_000_000.0).round() as i64;
        let six = format!("{rest:06}");
        let (khz_part, hz_part) = six.split_at(3);

        let mut out = cardinal(mhz);
        out.push_str(" point ");
        out.push_str(&digits(khz_part));
        // Trailing Hz only when there are any: a dial sitting on a round
        // kilohertz should read short. All three digits when there are —
        // trimming "350" to "35" would read 350 Hz and 35 Hz alike.
        if hz_part != "000" {
            out.push_str(", ");
            out.push_str(&digits(hz_part));
        }
        if units {
            out.push_str(" megahertz");
        }
        out
    }

    // ── radio settings ──────────────────────────────────────────────────

    /// A mode, as letters — see the module note on style.
    pub fn mode(&self, m: Mode) -> &'static str {
        match m {
            Mode::Lsb => "L S B",
            Mode::Usb => "U S B",
            Mode::Cw => "C W",
            Mode::Am => "A M",
            Mode::Sam => "synchronous A M",
            Mode::Nfm => "narrow F M",
            Mode::Wfm => "wide F M",
            Mode::Digu => "digital upper",
            Mode::Digl => "digital lower",
            Mode::Dsb => "D S B",
            Mode::Isb => "I S B",
            Mode::Spec => "spectrum",
            Mode::Ft8 => "F T eight",
            Mode::Ft4 => "F T four",
            Mode::Ft2 => "F T two",
            Mode::Psk => "P S K",
            // The two nobody spells out.
            Mode::Rtty => "ritty",
            Mode::Hell => "hell",
            Mode::Sstv => "S S T V",
            // Which radio it is on matters here as much as the picture does,
            // so it is said rather than spelt at: "S S T V F M" run together
            // is a fifth and sixth letter of the mode's name.
            Mode::SstvFm => "S S T V on F M",
            Mode::RttyFm => "ritty on F M",
            Mode::Olivia => "olivia",
            Mode::Thor => "thor",
            Mode::Fsq => "F S Q",
            Mode::RfPaint => "R F paint",
            Mode::Rade => "rade",
            Mode::Rifp => "R I F P",
            Mode::Packet => "packet",
            Mode::PacketHf => "H F packet",
            // A word, not four letters: nobody says "A P R S" out loud.
            Mode::Aprs => "aprs",
            Mode::Wefax => "wefax",
            Mode::Navtex => "nav tex",
            Mode::Js8 => "J S eight",
            // Pronounced "whisper" — the joke the name is built on.
            Mode::Wspr => "whisper",
            Mode::Drm => "D R M",
            Mode::Adsb => "A D S B",
            Mode::Vdl2 => "V D L 2",
            Mode::Ais => "A I S",
        }
    }

    /// A band, in the words its edges are known by.
    pub fn band(&self, b: Band) -> &'static str {
        match b {
            Band::M160 => "one sixty meters",
            Band::M80 => "eighty meters",
            Band::M60 => "sixty meters",
            Band::M40 => "forty meters",
            Band::M30 => "thirty meters",
            Band::M20 => "twenty meters",
            Band::M17 => "seventeen meters",
            Band::M15 => "fifteen meters",
            Band::M12 => "twelve meters",
            Band::M10 => "ten meters",
            Band::M6 => "six meters",
            Band::M4 => "four meters",
            Band::M2 => "two meters",
            // Said the way an operator says it. "One point two five meters" is
            // what the label reads; nobody says it out loud.
            Band::M125 => "one and a quarter meters",
            Band::M70 => "seventy centimeters",
            Band::Cm33 => "thirty three centimeters",
            Band::Cm23 => "twenty three centimeters",
            Band::Cm13 => "thirteen centimeters",
            Band::Cm9 => "nine centimeters",
            // The one band whose name depends on where the station is — six
            // centimeters under the Region 1 plan, five in the other two. Same
            // rule as the on-screen label; see `Band::label_in`.
            Band::Cm6 => match sdroxide_types::region() {
                sdroxide_types::Region::R1 => "six centimeters",
                sdroxide_types::Region::R2 | sdroxide_types::Region::R3 => "five centimeters",
            },
            Band::Cm3 => "three centimeters",
            Band::Gen => "general coverage",
        }
    }

    pub fn agc(&self, a: AgcMode) -> String {
        let which = match a {
            AgcMode::Off => "off",
            AgcMode::Slow => "slow",
            AgcMode::Med => "medium",
            AgcMode::Fast => "fast",
        };
        format!("A G C {which}")
    }

    /// Noise reduction, engine and strength.
    ///
    /// The engine name matters: an operator who has picked DeepFilterNet wants
    /// to know that is what came on, and the four engines sound nothing alike.
    pub fn nr(&self, n: NrLevel) -> String {
        let s = match n {
            NrLevel::Off => "noise reduction off".to_string(),
            NrLevel::Low => "noise reduction low".into(),
            NrLevel::Medium => "noise reduction medium".into(),
            NrLevel::High => "noise reduction high".into(),
            NrLevel::RnnLow => "R N N noise reduction low".into(),
            NrLevel::RnnMed => "R N N noise reduction medium".into(),
            NrLevel::RnnHigh => "R N N noise reduction high".into(),
            NrLevel::SpecLow => "spectral noise reduction low".into(),
            NrLevel::SpecMed => "spectral noise reduction medium".into(),
            NrLevel::SpecHigh => "spectral noise reduction high".into(),
            NrLevel::DfLow => "deep filter low".into(),
            NrLevel::DfMed => "deep filter medium".into(),
            NrLevel::DfHigh => "deep filter high".into(),
        };
        Self::sanitize(&s)
    }

    /// The band segment a frequency sits in — what actually keeps an operator
    /// legal, and the reason a frequency announcement can carry it.
    pub fn segment(&self, k: Option<SegmentKind>) -> &'static str {
        match k {
            Some(SegmentKind::Cw) => "C W",
            Some(SegmentKind::Digi) => "data",
            Some(SegmentKind::Phone) => "phone",
            Some(SegmentKind::Beacon) => "beacon",
            Some(SegmentKind::All) => "all modes",
            None => "",
        }
    }

    // ── measurements ────────────────────────────────────────────────────

    /// Standing wave ratio, always "to one".
    ///
    /// Bridges peg, and "ninety nine point nine to one" is not information —
    /// anything from about ten upwards is simply bad.
    pub fn swr(&self, v: f32) -> String {
        let s = if !v.is_finite() || v >= 9.9 {
            "over ten to one".to_string()
        } else {
            format!("{} to one", decimal1(v.max(1.0)))
        };
        Self::sanitize(&s)
    }

    pub fn watts(&self, w: f32) -> String {
        let s = if w >= 1000.0 {
            format!("{} kilowatts", decimal1(w / 1000.0))
        } else if (w - 1.0).abs() < 0.05 {
            "one watt".to_string()
        } else if w < 1.0 {
            format!("{} watts", decimal1(w))
        } else {
            format!("{} watts", cardinal(w.round() as i64))
        };
        Self::sanitize(&s)
    }

    /// A relative level, in decibels.
    pub fn decibels(&self, db: f32) -> String {
        Self::sanitize(&format!("{} decibels", cardinal(db.round() as i64)))
    }

    /// An absolute level. Spelled `d B m`, which is how it is said.
    pub fn dbm(&self, dbm: f32) -> String {
        Self::sanitize(&format!("{} d B m", cardinal(dbm.round() as i64)))
    }

    /// The S-meter reading: "S five", or "S nine plus twelve" over S9.
    pub fn s_meter(&self, m: &Meters) -> String {
        let (units, over) = m.s_units();
        let s = if over >= 1.0 {
            format!("S {} plus {}", cardinal(units as i64), cardinal(over.round() as i64))
        } else {
            format!("S {}", cardinal(units as i64))
        };
        Self::sanitize(&s)
    }

    /// A 0..=1 fraction as a percentage.
    pub fn percent(&self, frac: f32) -> String {
        let pct = (frac.clamp(0.0, 1.0) * 100.0).round() as i64;
        Self::sanitize(&format!("{} percent", cardinal(pct)))
    }

    pub fn hertz(&self, hz: i32) -> String {
        Self::sanitize(&format!("{} hertz", cardinal(hz as i64)))
    }

    /// A signal report, as a bare signed number: "minus twelve".
    pub fn snr(&self, db: i16) -> String {
        let s = if db > 0 { format!("plus {}", cardinal(db as i64)) } else { cardinal(db as i64) };
        Self::sanitize(&s)
    }

    /// An RST report, digit by digit. Cut numbers are expanded first —
    /// `5NN` is "five nine nine", which is what was meant.
    pub fn rst(&self, rst: &str) -> String {
        let expanded: String = rst.chars().map(abbrev::expand_cut).collect();
        Self::sanitize(&digits(&expanded))
    }

    // ── on-the-air text ─────────────────────────────────────────────────

    pub fn callsign(&self, call: &str) -> String {
        Self::sanitize(&phonetic::callsign(call, self.cfg.callsign_style))
    }

    pub fn grid(&self, grid: &str) -> String {
        Self::sanitize(&phonetic::grid(grid, self.cfg.callsign_style))
    }

    /// Free text off the air: an FT8 message body, a JS8 or FSQ message, a CW
    /// or RTTY tail.
    ///
    /// Token by token. Multi-word shorthand first, then the abbreviation table,
    /// then a series of shape tests, and finally the token itself lowercased.
    /// The order matters: `73` has to be caught before the "bare integer" rule
    /// turns it into "seventy three" *as a number*, which happens to agree here
    /// but would not for `88`.
    ///
    /// # What gets spelled
    ///
    /// A decoded line is not a sentence written for a listener. It is upper
    /// case whether or not anything in it is an initialism — Baudot has no
    /// lower case at all — and it is a mixture of callsigns, shorthand, units
    /// and ordinary prose. So the pass here spells only what it can *justify*
    /// spelling: a token shaped like a callsign ([`phonetic::looks_like_callsign`]),
    /// or one that cannot be pronounced ([`abbrev::reads_as_letters`]).
    /// Anything else is handed on as a word, and the phonemizer spells it only
    /// if the dictionary has never heard of it. That is what keeps `GOOD` a
    /// word and `DDK2` a callsign in the same line of a DWD bulletin.
    pub fn message(&self, text: &str) -> String {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut out: Vec<String> = Vec::new();
        let mut i = 0;
        while i < words.len() {
            if let Some((phrase, used)) = abbrev::lookup_phrase(&words, i) {
                // The punctuation the phrase match ignored still carries the
                // prosody, and it sat on the last token of the phrase.
                let tail = if words[i + used - 1].ends_with(['.', '?', '!']) { "." } else { "" };
                out.push(format!("{phrase}{tail}"));
                i += used;
                continue;
            }
            let raw = words[i];
            let last = i + 1 == words.len();
            out.push(self.token(raw, last));
            i += 1;
        }
        Self::sanitize(&out.join(" "))
    }

    /// One free-text token.
    fn token(&self, raw: &str, is_last: bool) -> String {
        // Trailing sentence punctuation carries prosody; keep it and read the
        // rest.
        let core = raw.trim_end_matches(['.', ',', '!', '?', ':', ';']);
        let tail = if core.len() < raw.len() && raw.ends_with(['.', '?', '!']) { "." } else { "" };
        // A teleprinter idle is not text. Dropping it costs nothing, and
        // reading it costs the bulletin that follows.
        if core.is_empty() || filler::is_filler(core) {
            return String::new();
        }

        let body = if let Some(sub) = abbrev::lookup(core) {
            sub.to_string()
        } else if is_last && core.eq_ignore_ascii_case("K") {
            // Only at the end of a transmission does a lone K mean "over";
            // anywhere else it is a callsign letter or a mode.
            "over".to_string()
        } else if abbrev::looks_like_rst(core) {
            // Before the callsign and integer rules: `5NN` is letters and a
            // digit, and `599` is an integer, but in a decoded line both are
            // a signal report and are read digit by digit.
            self.rst(core)
        } else if phonetic::looks_like_callsign(core) {
            phonetic::callsign(core, self.cfg.callsign_style)
        } else if phonetic::looks_like_grid(core) {
            phonetic::grid(core, self.cfg.callsign_style)
        } else if is_zero_led_group(core) {
            // A leading zero means a group of digits rather than a quantity:
            // `0530` is half past five and `0000` is midnight, and neither is
            // a number anybody would say as one. A signed `+03` is a report,
            // and keeps the sign path below.
            digits(core)
        } else if let Some(n) = parse_signed(core) {
            if n > 0 && core.starts_with('+') {
                format!("plus {}", cardinal(n))
            } else {
                cardinal(n)
            }
        } else if let Some(s) = parse_decimal(core) {
            s
        } else if let Some(s) = self.compound(core) {
            s
        } else if abbrev::reads_as_letters(core) {
            abbrev::spaced(core)
        } else {
            core.to_ascii_lowercase()
        };
        format!("{body}{tail}")
    }

    /// Letters and digits welded together: `10100KHZ`, `1013HPA`, `0530UTC`,
    /// `21C`.
    ///
    /// Nothing above claimed it, so it is not a callsign, a grid, a report or a
    /// number — and yet it is the shape a bulletin writes a measurement in. Read
    /// by parts: each digit run as a number, each letter run through the same
    /// rules as any other token. `None` when the token is not that shape.
    ///
    /// This exists because of [`Self::sanitize`]. Left to fall through to the
    /// "ordinary word" arm, `10100KHZ` would reach the contract with digits in
    /// it, and the contract would delete them — leaving the operator a
    /// confident "kilohertz" with no frequency attached.
    fn compound(&self, core: &str) -> Option<String> {
        if !core.bytes().all(|b| b.is_ascii_alphanumeric()) {
            return None;
        }
        let mut out: Vec<String> = Vec::new();
        let mut rest = core;
        while !rest.is_empty() {
            let alpha = rest.starts_with(|c: char| c.is_ascii_alphabetic());
            let end = rest.find(|c: char| c.is_ascii_alphabetic() != alpha).unwrap_or(rest.len());
            let (run, tail) = rest.split_at(end);
            rest = tail;
            out.push(if alpha {
                let upper = run.to_ascii_uppercase();
                match abbrev::lookup(run) {
                    Some(sub) => sub.to_string(),
                    None if abbrev::reads_as_letters(&upper) => abbrev::spaced(run),
                    None => run.to_ascii_lowercase(),
                }
            } else if is_zero_led_group(run) {
                digits(run)
            } else {
                match run.parse::<i64>() {
                    Ok(n) => cardinal(n),
                    // Longer than an i64 — a serial number, or noise.
                    Err(_) => digits(run),
                }
            });
        }
        // One run is not a compound: that is an ordinary word or an ordinary
        // number, and the rules above have already had their say.
        (out.len() > 1).then(|| out.join(" "))
    }

    // ── the contract ────────────────────────────────────────────────────

    /// Reduce a string to what the phonemizer can actually pronounce.
    ///
    /// The last line of defence for the module contract: letters, space, comma,
    /// full stop and apostrophe survive; everything else is dropped, and runs
    /// of whitespace collapse. Every public method above ends here, so a rule
    /// that forgets to spell a number produces a missing word rather than a
    /// silently swallowed one — and the invariant test finds it.
    pub fn sanitize(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut pending_space = false;
        for ch in s.chars() {
            match ch {
                c if c.is_ascii_alphabetic() => {
                    if pending_space && !out.is_empty() {
                        out.push(' ');
                    }
                    pending_space = false;
                    out.push(c);
                }
                '\'' => {
                    if pending_space && !out.is_empty() {
                        out.push(' ');
                    }
                    pending_space = false;
                    out.push('\'');
                }
                ',' | '.' => {
                    // Punctuation attaches to the word before it.
                    pending_space = false;
                    if !out.is_empty() && !out.ends_with([',', '.', ' ']) {
                        out.push(ch);
                    }
                }
                _ => pending_space = true,
            }
        }
        out.trim().to_string()
    }

    // ── convenience ─────────────────────────────────────────────────────

    pub fn verbosity(&self) -> Verbosity {
        self.cfg.verbosity
    }

    pub fn style(&self) -> CallsignStyle {
        self.cfg.callsign_style
    }
}

/// A decimal number token: `14.074`, `1.5`, `-0.5`.
///
/// Read like a dial — the fraction digit by digit, so "fourteen point zero
/// seven four" cannot be confused with "fourteen point seventy four". A
/// frequency written into free text (`QSY 14.074`) is the common case, and
/// before this rule existed [`Speaker::sanitize`] threw the whole token away.
fn parse_decimal(s: &str) -> Option<String> {
    let body = s.strip_prefix(['+', '-']).unwrap_or(s);
    let (int, frac) = body.split_once('.')?;
    let digity = |p: &str| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit());
    if !digity(int) || !digity(frac) {
        return None;
    }
    let whole: i64 = int.parse().ok()?;
    let sign = if s.starts_with('-') { "minus " } else { "" };
    Some(format!("{sign}{} point {}", cardinal(whole), digits(frac)))
}

/// Digits with a leading zero: `0530`, `007`, `0000`.
fn is_zero_led_group(s: &str) -> bool {
    s.len() > 1 && s.starts_with('0') && s.bytes().all(|b| b.is_ascii_digit())
}

/// A signed or unsigned integer token: `-12`, `+03`, `250`.
fn parse_signed(s: &str) -> Option<i64> {
    let body = s.strip_prefix(['+', '-']).unwrap_or(s);
    if body.is_empty() || !body.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let n: i64 = body.parse().ok()?;
    Some(if s.starts_with('-') { -n } else { n })
}
