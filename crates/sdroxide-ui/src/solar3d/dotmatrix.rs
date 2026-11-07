//! A 5×7 dot-matrix display, painted as a grid of discs.
//!
//! Not a font: a real dot matrix shows the *unlit* dots too, faintly, and that
//! is most of what makes it read as a physical display rather than as text in a
//! blocky typeface. Each glyph is five columns of seven bits, LSB at the top.

use eframe::egui::{self, Color32, Painter, Pos2, Rect, Vec2, pos2};

pub const GLYPH_W: usize = 5;
pub const GLYPH_H: usize = 7;
/// Blank columns between glyphs.
const GAP_COLS: usize = 1;

/// Column bitmaps, one `u8` per column, bit 0 = top row.
///
/// Only the characters the clock needs. Anything else renders blank, which is
/// the same thing a real matrix display would do.
fn glyph(c: char) -> Option<[u8; GLYPH_W]> {
    Some(match c {
        '0' => [0x3E, 0x51, 0x49, 0x45, 0x3E],
        '1' => [0x00, 0x42, 0x7F, 0x40, 0x00],
        '2' => [0x42, 0x61, 0x51, 0x49, 0x46],
        '3' => [0x21, 0x41, 0x45, 0x4B, 0x31],
        '4' => [0x18, 0x14, 0x12, 0x7F, 0x10],
        '5' => [0x27, 0x45, 0x45, 0x45, 0x39],
        '6' => [0x3C, 0x4A, 0x49, 0x49, 0x30],
        '7' => [0x01, 0x71, 0x09, 0x05, 0x03],
        '8' => [0x36, 0x49, 0x49, 0x49, 0x36],
        '9' => [0x06, 0x49, 0x49, 0x29, 0x1E],
        ':' => [0x00, 0x00, 0x36, 0x00, 0x00],
        '.' => [0x00, 0x00, 0x60, 0x00, 0x00],
        '-' => [0x08, 0x08, 0x08, 0x08, 0x08],
        '+' => [0x08, 0x08, 0x3E, 0x08, 0x08],
        ' ' => [0x00, 0x00, 0x00, 0x00, 0x00],
        'U' => [0x3F, 0x40, 0x40, 0x40, 0x3F],
        'T' => [0x01, 0x01, 0x7F, 0x01, 0x01],
        'C' => [0x3E, 0x41, 0x41, 0x41, 0x22],
        'Z' => [0x61, 0x51, 0x49, 0x45, 0x43],
        'L' => [0x7F, 0x40, 0x40, 0x40, 0x40],
        'O' => [0x3E, 0x41, 0x41, 0x41, 0x3E],
        'S' => [0x46, 0x49, 0x49, 0x49, 0x31],
        'I' => [0x00, 0x41, 0x7F, 0x41, 0x00],
        'M' => [0x7F, 0x02, 0x0C, 0x02, 0x7F],
        _ => return None,
    })
}

/// Total width in dot cells for a string, including inter-glyph gaps.
pub fn cells_wide(text: &str) -> usize {
    let n = text.chars().count();
    if n == 0 { 0 } else { n * GLYPH_W + (n - 1) * GAP_COLS }
}

/// Pixel size of `text` at a given dot pitch.
pub fn size(text: &str, pitch: f32) -> Vec2 {
    egui::vec2(cells_wide(text) as f32 * pitch, GLYPH_H as f32 * pitch)
}

/// Paint `text` with its top-left at `origin`.
///
/// `off` is the colour of unlit dots — pass something close to the background
/// and the panel reads as a real display rather than as floating glyphs.
pub fn draw(p: &Painter, origin: Pos2, text: &str, pitch: f32, on: Color32, off: Color32) -> Rect {
    let radius = pitch * 0.38;
    let mut x0 = 0usize;
    for ch in text.chars() {
        let cols = glyph(ch.to_ascii_uppercase()).unwrap_or([0; GLYPH_W]);
        for (cx, bits) in cols.iter().enumerate() {
            for cy in 0..GLYPH_H {
                let lit = bits & (1 << cy) != 0;
                if !lit && off == Color32::TRANSPARENT {
                    continue;
                }
                let centre = pos2(
                    origin.x + (x0 + cx) as f32 * pitch + pitch * 0.5,
                    origin.y + cy as f32 * pitch + pitch * 0.5,
                );
                p.circle_filled(centre, radius, if lit { on } else { off });
            }
        }
        x0 += GLYPH_W + GAP_COLS;
    }
    Rect::from_min_size(origin, size(text, pitch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_digit_has_a_glyph_and_none_is_blank() {
        for c in "0123456789".chars() {
            let g = glyph(c).unwrap_or_else(|| panic!("no glyph for {c}"));
            assert!(g.iter().any(|b| *b != 0), "{c} renders blank");
            // Nothing may light row 8 — the matrix is seven high.
            assert!(g.iter().all(|b| b & 0x80 == 0), "{c} overflows the 7-row matrix");
        }
        assert!(glyph(' ').unwrap().iter().all(|b| *b == 0), "space should be blank");
    }

    #[test]
    fn digits_are_distinguishable_from_each_other() {
        // A transposition in the table would silently render, say, 6 as 8.
        let mut seen = std::collections::HashMap::new();
        for c in "0123456789".chars() {
            let g = glyph(c).unwrap();
            if let Some(prev) = seen.insert(g, c) {
                panic!("{c} and {prev} have identical bitmaps");
            }
        }
    }

    #[test]
    fn every_clock_label_character_has_a_glyph() {
        // A missing letter renders blank rather than failing, so "-- SIM" once
        // drew as two dashes and three empty cells.
        for c in "UTC LOC -- SIM 2026-11-01".chars().filter(|c| !c.is_whitespace() && *c != '-') {
            let g = glyph(c).unwrap_or_else(|| panic!("no glyph for {c}"));
            assert!(g.iter().any(|b| *b != 0), "{c} renders blank");
            assert!(g.iter().all(|b| b & 0x80 == 0), "{c} overflows the 7-row matrix");
        }
    }

    #[test]
    fn unknown_characters_do_not_panic() {
        assert_eq!(glyph('%'), None);
        assert_eq!(cells_wide(""), 0);
        assert_eq!(size("", 4.0), egui::vec2(0.0, 28.0));
    }

    #[test]
    fn layout_accounts_for_inter_glyph_gaps() {
        assert_eq!(cells_wide("0"), 5);
        assert_eq!(cells_wide("00"), 11);
        // "12:34:56" — eight glyphs, seven gaps.
        assert_eq!(cells_wide("12:34:56"), 8 * 5 + 7);
        let s = size("12:34:56", 4.0);
        assert_eq!(s.x, 47.0 * 4.0);
        assert_eq!(s.y, 7.0 * 4.0);
    }
}
