use crate::dsp::sine;

pub const UNIT: usize = 2048;

fn morse(c: char) -> Option<&'static str> {
    Some(match c {
        'A' => "10111",
        'B' => "111010101",
        'C' => "11101011101",
        'D' => "1110101",
        'E' => "1",
        'F' => "101011101",
        'G' => "111011101",
        'H' => "1010101",
        'I' => "101",
        'J' => "1011101110111",
        'K' => "111010111",
        'L' => "101110101",
        'M' => "1110111",
        'N' => "11101",
        'O' => "11101110111",
        'P' => "10111011101",
        'Q' => "1110111010111",
        'R' => "1011101",
        'S' => "10101",
        'T' => "111",
        'U' => "1010111",
        'V' => "101010111",
        'W' => "101110111",
        'X' => "11101010111",
        'Y' => "1110101110111",
        'Z' => "11101110101",
        '1' => "10111011101110111",
        '2' => "101011101110111",
        '3' => "1010101110111",
        '4' => "10101010111",
        '5' => "101010101",
        '6' => "11101010101",
        '7' => "1110111010101",
        '8' => "111011101110101",
        '9' => "11101110111011101",
        '0' => "1110111011101110111",
        _ => return None,
    })
}

pub fn units(text: &str) -> Vec<bool> {
    let mut out = Vec::new();
    for c in text.chars().map(|c| c.to_ascii_uppercase()) {
        if c == ' ' {
            out.extend([false; 7]);
        } else if let Some(m) = morse(c) {
            out.extend(m.bytes().map(|b| b == b'1'));
            out.extend([false; 3]);
        }
    }
    out
}

pub fn synth(call: &str, blanks: u32, ra_board: bool) -> Vec<f64> {
    let text = " ".repeat(blanks as usize) + call;
    let tone = |k: u32, a: f64| sine(k, UNIT)[1..].iter().map(move |&s| a * s);
    let mut keyed = false;
    let mut out = Vec::new();
    for u in units(&text) {
        keyed |= u;
        let lo = keyed && !ra_board;
        out.extend(tone(64, if u { 1.6 } else { 0.0 }).zip(tone(32, if lo { 0.8 } else { 0.0 })).map(|(a, b)| a + b));
    }
    out
}
