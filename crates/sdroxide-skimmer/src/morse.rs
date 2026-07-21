//! A streaming Morse decoder. Fed the durations of key-down (mark) and key-up
//! (gap) intervals, it keeps a running estimate of the dit length and decodes
//! characters as they complete. Robust to speed because the dit estimate
//! adapts; the first character or two of a fast signal may garble until it
//! converges (the usual CW-decoder "leading dropout").

/// International Morse table: (character, code) with `.`=dit, `-`=dah.
const TABLE: &[(char, &str)] = &[
    ('A', ".-"), ('B', "-..."), ('C', "-.-."), ('D', "-.."), ('E', "."),
    ('F', "..-."), ('G', "--."), ('H', "...."), ('I', ".."), ('J', ".---"),
    ('K', "-.-"), ('L', ".-.."), ('M', "--"), ('N', "-."), ('O', "---"),
    ('P', ".--."), ('Q', "--.-"), ('R', ".-."), ('S', "..."), ('T', "-"),
    ('U', "..-"), ('V', "...-"), ('W', ".--"), ('X', "-..-"), ('Y', "-.--"),
    ('Z', "--.."),
    ('0', "-----"), ('1', ".----"), ('2', "..---"), ('3', "...--"), ('4', "....-"),
    ('5', "....."), ('6', "-...."), ('7', "--..."), ('8', "---.."), ('9', "----."),
    ('.', ".-.-.-"), (',', "--..--"), ('?', "..--.."), ('/', "-..-."),
    ('=', "-...-"), ('+', ".-.-."), ('-', "-....-"), ('(', "-.--."), (')', "-.--.-"),
];

/// Decode a `.-` code string to a character, or `None` if unknown.
pub fn decode_symbol(code: &str) -> Option<char> {
    TABLE.iter().find(|(_, c)| *c == code).map(|(ch, _)| *ch)
}

/// Encode a character to its `.-` code (test-only synthesis helper).
#[cfg(test)]
pub fn encode_char(ch: char) -> Option<&'static str> {
    let up = ch.to_ascii_uppercase();
    TABLE.iter().find(|(c, _)| *c == up).map(|(_, code)| *code)
}

pub struct MorseDecoder {
    /// Adaptive dit-length estimate (ms).
    dit_ms: f32,
    /// The current character's elements so far ("" until a mark arrives).
    symbol: String,
    /// Rolling decoded text tail.
    text: String,
    max_text: usize,
}

impl Default for MorseDecoder {
    fn default() -> Self {
        MorseDecoder {
            dit_ms: 60.0, // 20 WPM
            symbol: String::new(),
            text: String::new(),
            max_text: 64,
        }
    }
}

impl MorseDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// A key-down interval of `ms` milliseconds.
    pub fn on_mark(&mut self, ms: f32) {
        if ms <= 0.0 {
            return;
        }
        if ms > self.dit_ms * 2.0 {
            self.symbol.push('-');
            self.adapt(ms / 3.0); // a dah is ~3 dits
        } else {
            self.symbol.push('.');
            self.adapt(ms);
        }
        // A real character is ≤ ~6 elements; drop runaway noise.
        if self.symbol.len() > 8 {
            self.symbol.clear();
        }
    }

    /// A key-up interval of `ms` milliseconds.
    pub fn on_gap(&mut self, ms: f32) {
        if ms < self.dit_ms * 2.0 {
            return; // element gap within a character
        }
        self.commit_symbol(); // character boundary
        if ms >= self.dit_ms * 5.0 {
            self.push_char(' '); // word boundary
        }
    }

    /// Force-decode any pending character (e.g. the signal went silent).
    pub fn flush(&mut self) {
        self.commit_symbol();
    }

    fn adapt(&mut self, dit_estimate: f32) {
        let e = dit_estimate.clamp(20.0, 240.0); // 5..60 WPM
        self.dit_ms = 0.75 * self.dit_ms + 0.25 * e;
    }

    fn commit_symbol(&mut self) {
        if self.symbol.is_empty() {
            return;
        }
        let c = decode_symbol(&self.symbol).unwrap_or('?');
        self.symbol.clear();
        self.push_char(c);
    }

    fn push_char(&mut self, c: char) {
        if c == ' ' && (self.text.is_empty() || self.text.ends_with(' ')) {
            return; // collapse leading/repeated spaces
        }
        self.text.push(c);
        if self.text.len() > self.max_text {
            let drop = self.text.len() - self.max_text;
            self.text.drain(..drop);
        }
    }

    pub fn text(&self) -> &str {
        self.text.trim_end()
    }

    pub fn wpm(&self) -> u16 {
        (1200.0 / self.dit_ms).round().clamp(1.0, 99.0) as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Synthesize the alternating (is_mark, ms) timing for `text` at `wpm`.
    fn timings(text: &str, wpm: f32) -> Vec<(bool, f32)> {
        let dit = 1200.0 / wpm;
        let mut out = Vec::new();
        let words: Vec<&str> = text.split(' ').filter(|w| !w.is_empty()).collect();
        for (wi, word) in words.iter().enumerate() {
            let chars: Vec<char> = word.chars().collect();
            for (ci, ch) in chars.iter().enumerate() {
                let code = encode_char(*ch).unwrap();
                let n = code.chars().count();
                for (ei, el) in code.chars().enumerate() {
                    out.push((true, if el == '-' { dit * 3.0 } else { dit }));
                    if ei + 1 < n {
                        out.push((false, dit)); // element gap
                    }
                }
                if ci + 1 < chars.len() {
                    out.push((false, dit * 3.0)); // char gap
                }
            }
            if wi + 1 < words.len() {
                out.push((false, dit * 7.0)); // word gap
            }
        }
        out
    }

    fn decode(text: &str, wpm: f32) -> String {
        let mut d = MorseDecoder::new();
        for (mark, ms) in timings(text, wpm) {
            if mark {
                d.on_mark(ms);
            } else {
                d.on_gap(ms);
            }
        }
        d.flush();
        d.text().to_string()
    }

    #[test]
    fn decodes_clean_20wpm_exact() {
        assert_eq!(decode("CQ TEST DE W1AW W1AW K", 20.0), "CQ TEST DE W1AW W1AW K");
    }

    #[test]
    fn decodes_across_speeds_tail() {
        // Faster/slower signals start from the wrong scale but converge; the
        // callsign (late in the string) must come through.
        for wpm in [15.0, 25.0, 30.0, 40.0] {
            let got = decode("CQ TEST DE W1AW W1AW K", wpm);
            assert!(got.contains("W1AW"), "wpm {wpm}: {got:?}");
        }
    }

    #[test]
    fn tables_are_invertible() {
        for &(ch, code) in TABLE {
            assert_eq!(decode_symbol(code), Some(ch));
            assert_eq!(encode_char(ch), Some(code));
        }
    }
}
