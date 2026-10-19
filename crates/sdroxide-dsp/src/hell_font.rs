//! The Hellschreiber transmit font: 7 columns x 14 dot rows per character,
//! the last column being the inter-character gap.
//!
//! Authored for this project. Classic Hell fonts are caps-only, so lowercase is
//! folded to uppercase at lookup time rather than given its own glyphs.
//!
//! The art below is the source of truth; [`HELL_FONT`] is parsed from it at
//! compile time, so what you read is exactly what goes on the air. Each glyph is
//! 14 lines of 7 characters, `#` lit and space blank, scanned top to bottom then
//! left to right — the order Hell transmits them in.

use sdroxide_types::HellVariant;

/// Cell geometry, re-exported from the variant table so there is one source.
pub const ROWS: usize = HellVariant::ROWS;
pub const COLS: usize = HellVariant::CELL_COLS;

/// Number of glyphs in [`HELL_FONT`]: ASCII 0x20..=0x5F, then the five
/// non-letter symbols above it (see [`glyph_index`]).
pub const GLYPHS: usize = 69;

/// One `u16` per column, **bit 0 = the top dot row**.
///
/// Column-major because that is the transmit scan order: emitting a character
/// is a bit test per dot with no transposition.
pub const HELL_FONT: [[u16; COLS]; GLYPHS] = parse_font(FONT_SRC);

/// Table index for `c`, or `None` for anything unrenderable (which the modem
/// sends as a blank cell).
pub const fn glyph_index(c: char) -> Option<usize> {
    let c = c.to_ascii_uppercase();
    match c {
        ' '..='_' => Some(c as usize - 0x20),
        '`' => Some(0x40),
        '{' => Some(0x41),
        '|' => Some(0x42),
        '}' => Some(0x43),
        '~' => Some(0x44),
        _ => None,
    }
}

/// The dot columns for `c`, bit 0 = top row. Unrenderable characters give a
/// blank cell, so a stray control code costs one character of space rather than
/// desynchronising the column stream.
pub fn glyph(c: char) -> [u16; COLS] {
    match glyph_index(c) {
        Some(i) => HELL_FONT[i],
        None => [0; COLS],
    }
}

/// Parse [`FONT_SRC`] at compile time. Panics during const evaluation if the
/// art is malformed, so a mis-typed glyph fails the build rather than going on
/// the air as garbage.
const fn parse_font(src: &str) -> [[u16; COLS]; GLYPHS] {
    let b = src.as_bytes();
    // Each row is COLS characters plus a newline; each glyph is ROWS rows.
    assert!(b.len() == GLYPHS * ROWS * (COLS + 1), "font art has the wrong length");
    let mut font = [[0u16; COLS]; GLYPHS];
    let mut g = 0;
    while g < GLYPHS {
        let mut r = 0;
        while r < ROWS {
            let line = (g * ROWS + r) * (COLS + 1);
            let mut c = 0;
            while c < COLS {
                match b[line + c] {
                    // The art is drawn on the baseline the eye wants, one
                    // element higher than the transmitted cell. Every on-air
                    // font leaves a blank pixel above and below the glyph, and
                    // the receiver finds the cell by that gap: the bottom
                    // padding is sent first and the top padding last, so the
                    // two blanks are adjacent in the cyclic element order and
                    // the boundary sits at a known distance from them (see
                    // GAP_OFFSET in `hell.rs`). Sitting the body one element
                    // down puts the padding where every other transmitter puts
                    // it, which is what lets our own signal be tracked, and it
                    // is why the last art row must stay blank to fall off.
                    b'#' => font[g][c] |= 1 << (r + 1),
                    b' ' => {}
                    _ => panic!("font art may only contain '#' and ' '"),
                }
                c += 1;
            }
            assert!(b[line + COLS] == b'\n', "font art row is the wrong width");
            r += 1;
        }
        // The bottom art row must be blank, or its own bit would fall off the
        // end of the cell.
        let mut c = 0;
        while c < COLS {
            assert!(font[g][c] >> ROWS == 0, "the bottom art row must be blank");
            c += 1;
        }
        g += 1;
    }
    font
}

/// The font, as pictures. See the module docs for the format.
const FONT_SRC: &str = concat!(
    // 0x20  space
    "       \n       \n       \n       \n       \n       \n       \n",
    "       \n       \n       \n       \n       \n       \n       \n",
    // 0x21  !
    "       \n  ##   \n  ##   \n  ##   \n  ##   \n  ##   \n  ##   \n",
    "  ##   \n       \n  ##   \n  ##   \n       \n       \n       \n",
    // 0x22  quote
    "       \n ## ## \n ## ## \n ## ## \n       \n       \n       \n",
    "       \n       \n       \n       \n       \n       \n       \n",
    // 0x23  #
    "       \n ## ## \n ## ## \n###### \n ## ## \n ## ## \n###### \n",
    " ## ## \n ## ## \n       \n       \n       \n       \n       \n",
    // 0x24  $
    "       \n  ##   \n ####  \n## ##  \n## ##  \n ####  \n   ##  \n",
    "## ##  \n## ##  \n ####  \n  ##   \n       \n       \n       \n",
    // 0x25  %
    "       \n##  ## \n##  ## \n    ## \n   ##  \n  ##   \n  ##   \n",
    " ##    \n##     \n##  ## \n##  ## \n       \n       \n       \n",
    // 0x26  &
    "       \n ###   \n## ##  \n## ##  \n## ##  \n ###   \n ###   \n",
    "## ##  \n##  ## \n##  ## \n ## ## \n       \n       \n       \n",
    // 0x27  '
    "       \n  ##   \n  ##   \n  ##   \n       \n       \n       \n",
    "       \n       \n       \n       \n       \n       \n       \n",
    // 0x28  (
    "       \n   ##  \n  ##   \n ##    \n ##    \n ##    \n ##    \n",
    " ##    \n ##    \n  ##   \n   ##  \n       \n       \n       \n",
    // 0x29  )
    "       \n ##    \n  ##   \n   ##  \n   ##  \n   ##  \n   ##  \n",
    "   ##  \n   ##  \n  ##   \n ##    \n       \n       \n       \n",
    // 0x2a  *
    "       \n       \n       \n##  ## \n ####  \n###### \n ####  \n",
    "##  ## \n       \n       \n       \n       \n       \n       \n",
    // 0x2b  +
    "       \n       \n       \n  ##   \n  ##   \n###### \n###### \n",
    "  ##   \n  ##   \n       \n       \n       \n       \n       \n",
    // 0x2c  ,
    "       \n       \n       \n       \n       \n       \n       \n",
    "       \n       \n  ##   \n  ##   \n ##    \n ##    \n       \n",
    // 0x2d  -
    "       \n       \n       \n       \n       \n###### \n###### \n",
    "       \n       \n       \n       \n       \n       \n       \n",
    // 0x2e  .
    "       \n       \n       \n       \n       \n       \n       \n",
    "       \n       \n  ##   \n  ##   \n       \n       \n       \n",
    // 0x2f  /
    "       \n    ## \n    ## \n   ##  \n   ##  \n  ##   \n  ##   \n",
    " ##    \n ##    \n##     \n##     \n       \n       \n       \n",
    // 0x30  0
    "       \n ####  \n##  ## \n##  ## \n## ### \n## ### \n### ## \n",
    "### ## \n##  ## \n##  ## \n ####  \n       \n       \n       \n",
    // 0x31  1
    "       \n  ##   \n ###   \n####   \n  ##   \n  ##   \n  ##   \n",
    "  ##   \n  ##   \n  ##   \n###### \n       \n       \n       \n",
    // 0x32  2
    "       \n ####  \n##  ## \n    ## \n    ## \n   ##  \n  ##   \n",
    " ##    \n##     \n##     \n###### \n       \n       \n       \n",
    // 0x33  3
    "       \n ####  \n##  ## \n    ## \n    ## \n  ###  \n  ###  \n",
    "    ## \n    ## \n##  ## \n ####  \n       \n       \n       \n",
    // 0x34  4
    "       \n   ##  \n  ###  \n ####  \n## ##  \n## ##  \n###### \n",
    "   ##  \n   ##  \n   ##  \n   ##  \n       \n       \n       \n",
    // 0x35  5
    "       \n###### \n##     \n##     \n##     \n#####  \n    ## \n",
    "    ## \n    ## \n##  ## \n ####  \n       \n       \n       \n",
    // 0x36  6
    "       \n  ###  \n ##    \n##     \n##     \n#####  \n##  ## \n",
    "##  ## \n##  ## \n##  ## \n ####  \n       \n       \n       \n",
    // 0x37  7
    "       \n###### \n    ## \n    ## \n   ##  \n   ##  \n  ##   \n",
    "  ##   \n ##    \n ##    \n ##    \n       \n       \n       \n",
    // 0x38  8
    "       \n ####  \n##  ## \n##  ## \n##  ## \n ####  \n##  ## \n",
    "##  ## \n##  ## \n##  ## \n ####  \n       \n       \n       \n",
    // 0x39  9
    "       \n ####  \n##  ## \n##  ## \n##  ## \n##  ## \n ##### \n",
    "    ## \n    ## \n  ###  \n ###   \n       \n       \n       \n",
    // 0x3a  :
    "       \n       \n       \n  ##   \n  ##   \n       \n       \n",
    "       \n  ##   \n  ##   \n       \n       \n       \n       \n",
    // 0x3b  ;
    "       \n       \n       \n  ##   \n  ##   \n       \n       \n",
    "       \n       \n  ##   \n  ##   \n ##    \n ##    \n       \n",
    // 0x3c  <
    "       \n       \n    ## \n   ##  \n  ##   \n ##    \n ##    \n",
    "  ##   \n   ##  \n    ## \n       \n       \n       \n       \n",
    // 0x3d  =
    "       \n       \n       \n       \n###### \n###### \n       \n",
    "###### \n###### \n       \n       \n       \n       \n       \n",
    // 0x3e  >
    "       \n       \n##     \n ##    \n  ##   \n   ##  \n   ##  \n",
    "  ##   \n ##    \n##     \n       \n       \n       \n       \n",
    // 0x3f  ?
    "       \n ####  \n##  ## \n    ## \n    ## \n   ##  \n  ##   \n",
    "  ##   \n       \n  ##   \n  ##   \n       \n       \n       \n",
    // 0x40  @
    "       \n ####  \n##  ## \n##  ## \n## ### \n## ### \n## ### \n",
    "##     \n##     \n##  ## \n ####  \n       \n       \n       \n",
    // 0x41  A
    "       \n  ##   \n ####  \n##  ## \n##  ## \n##  ## \n###### \n",
    "##  ## \n##  ## \n##  ## \n##  ## \n       \n       \n       \n",
    // 0x42  B
    "       \n#####  \n##  ## \n##  ## \n##  ## \n#####  \n##  ## \n",
    "##  ## \n##  ## \n##  ## \n#####  \n       \n       \n       \n",
    // 0x43  C
    "       \n ####  \n##  ## \n##     \n##     \n##     \n##     \n",
    "##     \n##     \n##  ## \n ####  \n       \n       \n       \n",
    // 0x44  D
    "       \n#####  \n##  ## \n##  ## \n##  ## \n##  ## \n##  ## \n",
    "##  ## \n##  ## \n##  ## \n#####  \n       \n       \n       \n",
    // 0x45  E
    "       \n###### \n##     \n##     \n##     \n#####  \n##     \n",
    "##     \n##     \n##     \n###### \n       \n       \n       \n",
    // 0x46  F
    "       \n###### \n##     \n##     \n##     \n#####  \n##     \n",
    "##     \n##     \n##     \n##     \n       \n       \n       \n",
    // 0x47  G
    "       \n ####  \n##  ## \n##     \n##     \n##     \n## ### \n",
    "##  ## \n##  ## \n##  ## \n ####  \n       \n       \n       \n",
    // 0x48  H
    "       \n##  ## \n##  ## \n##  ## \n##  ## \n###### \n##  ## \n",
    "##  ## \n##  ## \n##  ## \n##  ## \n       \n       \n       \n",
    // 0x49  I
    "       \n ####  \n  ##   \n  ##   \n  ##   \n  ##   \n  ##   \n",
    "  ##   \n  ##   \n  ##   \n ####  \n       \n       \n       \n",
    // 0x4a  J
    "       \n   ### \n    ## \n    ## \n    ## \n    ## \n    ## \n",
    "    ## \n##  ## \n##  ## \n ####  \n       \n       \n       \n",
    // 0x4b  K
    "       \n##  ## \n##  ## \n## ##  \n####   \n###    \n###    \n",
    "####   \n## ##  \n##  ## \n##  ## \n       \n       \n       \n",
    // 0x4c  L
    "       \n##     \n##     \n##     \n##     \n##     \n##     \n",
    "##     \n##     \n##     \n###### \n       \n       \n       \n",
    // 0x4d  M
    "       \n##  ## \n###### \n###### \n##  ## \n##  ## \n##  ## \n",
    "##  ## \n##  ## \n##  ## \n##  ## \n       \n       \n       \n",
    // 0x4e  N
    "       \n##  ## \n### ## \n### ## \n###### \n###### \n## ### \n",
    "## ### \n##  ## \n##  ## \n##  ## \n       \n       \n       \n",
    // 0x4f  O
    "       \n ####  \n##  ## \n##  ## \n##  ## \n##  ## \n##  ## \n",
    "##  ## \n##  ## \n##  ## \n ####  \n       \n       \n       \n",
    // 0x50  P
    "       \n#####  \n##  ## \n##  ## \n##  ## \n##  ## \n#####  \n",
    "##     \n##     \n##     \n##     \n       \n       \n       \n",
    // 0x51  Q
    "       \n ####  \n##  ## \n##  ## \n##  ## \n##  ## \n##  ## \n",
    "##  ## \n##  ## \n## ##  \n ####  \n    ## \n    ## \n       \n",
    // 0x52  R
    "       \n#####  \n##  ## \n##  ## \n##  ## \n##  ## \n#####  \n",
    "## ##  \n## ##  \n##  ## \n##  ## \n       \n       \n       \n",
    // 0x53  S
    "       \n ####  \n##  ## \n##     \n##     \n ####  \n    ## \n",
    "    ## \n    ## \n##  ## \n ####  \n       \n       \n       \n",
    // 0x54  T
    "       \n###### \n  ##   \n  ##   \n  ##   \n  ##   \n  ##   \n",
    "  ##   \n  ##   \n  ##   \n  ##   \n       \n       \n       \n",
    // 0x55  U
    "       \n##  ## \n##  ## \n##  ## \n##  ## \n##  ## \n##  ## \n",
    "##  ## \n##  ## \n##  ## \n ####  \n       \n       \n       \n",
    // 0x56  V
    "       \n##  ## \n##  ## \n##  ## \n##  ## \n##  ## \n##  ## \n",
    "##  ## \n ####  \n ####  \n  ##   \n       \n       \n       \n",
    // 0x57  W
    "       \n##  ## \n##  ## \n##  ## \n##  ## \n##  ## \n##  ## \n",
    "##  ## \n##  ## \n###### \n###### \n       \n       \n       \n",
    // 0x58  X
    "       \n##  ## \n##  ## \n##  ## \n ####  \n  ##   \n  ##   \n",
    " ####  \n##  ## \n##  ## \n##  ## \n       \n       \n       \n",
    // 0x59  Y
    "       \n##  ## \n##  ## \n##  ## \n ####  \n  ##   \n  ##   \n",
    "  ##   \n  ##   \n  ##   \n  ##   \n       \n       \n       \n",
    // 0x5a  Z
    "       \n###### \n    ## \n    ## \n   ##  \n  ##   \n  ##   \n",
    " ##    \n##     \n##     \n###### \n       \n       \n       \n",
    // 0x5b  [
    "       \n ####  \n ##    \n ##    \n ##    \n ##    \n ##    \n",
    " ##    \n ##    \n ##    \n ####  \n       \n       \n       \n",
    // 0x5c  backslash
    "       \n##     \n##     \n ##    \n ##    \n  ##   \n  ##   \n",
    "   ##  \n   ##  \n    ## \n    ## \n       \n       \n       \n",
    // 0x5d  ]
    "       \n ####  \n   ##  \n   ##  \n   ##  \n   ##  \n   ##  \n",
    "   ##  \n   ##  \n   ##  \n ####  \n       \n       \n       \n",
    // 0x5e  ^
    "       \n  ##   \n ####  \n##  ## \n       \n       \n       \n",
    "       \n       \n       \n       \n       \n       \n       \n",
    // 0x5f  _
    "       \n       \n       \n       \n       \n       \n       \n",
    "       \n       \n       \n###### \n       \n       \n       \n",
    // 0x60  `
    "       \n ##    \n ##    \n  ##   \n       \n       \n       \n",
    "       \n       \n       \n       \n       \n       \n       \n",
    // 0x7b  {
    "       \n   ##  \n  ##   \n  ##   \n  ##   \n ##    \n ##    \n",
    "  ##   \n  ##   \n  ##   \n   ##  \n       \n       \n       \n",
    // 0x7c  |
    "       \n  ##   \n  ##   \n  ##   \n  ##   \n  ##   \n  ##   \n",
    "  ##   \n  ##   \n  ##   \n  ##   \n       \n       \n       \n",
    // 0x7d  }
    "       \n ##    \n  ##   \n  ##   \n  ##   \n   ##  \n   ##  \n",
    "  ##   \n  ##   \n  ##   \n ##    \n       \n       \n       \n",
    // 0x7e  ~
    "       \n       \n       \n       \n ## ## \n###### \n## ##  \n",
    "       \n       \n       \n       \n       \n       \n       \n",
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_cell_ends_with_a_gap() {
        // Without the trailing blank column, adjacent characters would touch and
        // the raster would read as one continuous smear.
        for (i, g) in HELL_FONT.iter().enumerate() {
            assert_eq!(g[COLS - 1], 0, "glyph {i} has ink in its gap column");
        }
    }

    #[test]
    fn every_dot_is_inside_the_cell() {
        for (i, g) in HELL_FONT.iter().enumerate() {
            for (c, &col) in g.iter().enumerate() {
                assert_eq!(col >> ROWS, 0, "glyph {i} column {c} has dots below row {ROWS}");
            }
        }
    }

    #[test]
    fn only_space_is_blank() {
        for c in ' '..='~' {
            let ink = glyph(c).iter().any(|&v| v != 0);
            assert_eq!(ink, c != ' ', "{c:?} blankness is wrong");
        }
    }

    #[test]
    fn lowercase_folds_to_uppercase() {
        for c in 'a'..='z' {
            assert_eq!(glyph(c), glyph(c.to_ascii_uppercase()), "{c:?}");
        }
    }

    #[test]
    fn unrenderable_characters_are_blank_cells() {
        for c in ['\u{0}', '\n', '\t', '\u{7f}', '\u{e9}', '\u{4e2d}'] {
            assert_eq!(glyph(c), [0; COLS], "{c:?}");
        }
    }
}
