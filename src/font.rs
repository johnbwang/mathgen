//! Advance widths for the two PDF built-in fonts we use, so we can right-align
//! and centre text without embedding or parsing a font file.

/// Helvetica advance widths (1/1000 em) for ASCII 32..=126.
const HELV: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584,
    278, 333, 278, 278, 556, 556, 556, 556, 556, 556, 556, 556,
    556, 556, 278, 278, 584, 584, 584, 556, 1015, 667, 667, 722,
    722, 667, 611, 778, 722, 278, 500, 667, 556, 833, 722, 778,
    667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 278,
    278, 278, 469, 556, 333, 556, 556, 500, 556, 556, 278, 556,
    556, 222, 222, 500, 222, 833, 556, 556, 556, 556, 333, 500,
    278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584,
];

/// Helvetica-Bold advance widths (1/1000 em) for ASCII 32..=126.
const HELV_BOLD: [u16; 95] = [
    278, 333, 474, 556, 556, 889, 722, 238, 333, 333, 389, 584,
    278, 333, 278, 278, 556, 556, 556, 556, 556, 556, 556, 556,
    556, 556, 333, 333, 584, 584, 584, 611, 975, 722, 722, 722,
    722, 667, 611, 778, 722, 278, 556, 722, 611, 833, 722, 778,
    667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 333,
    278, 333, 584, 556, 333, 556, 611, 556, 611, 556, 333, 611,
    611, 278, 278, 556, 278, 889, 611, 611, 611, 611, 389, 556,
    333, 611, 556, 778, 556, 556, 500, 389, 280, 389, 584,
];

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Font {
    Regular,
    Bold,
}

impl Font {
    /// PDF resource name used in the content stream.
    pub fn resource(self) -> &'static str {
        match self {
            Font::Regular => "F1",
            Font::Bold => "F2",
        }
    }

    fn advance(self, c: char) -> u16 {
        let table = match self {
            Font::Regular => &HELV,
            Font::Bold => &HELV_BOLD,
        };
        match c {
            ' '..='~' => table[c as usize - 32],
            // The non-ASCII glyphs we emit, all present in WinAnsiEncoding.
            '\u{2013}' => 556,  // en dash, used as the minus sign
            '\u{2014}' => 1000, // em dash
            '\u{00D7}' => 584,  // multiplication sign
            _ => table[('?' as usize) - 32],
        }
    }
}

/// Width of `text` in points when set at `size`.
pub fn width(text: &str, font: Font, size: f32) -> f32 {
    let total: u32 = text.chars().map(|c| font.advance(c) as u32).sum();
    total as f32 * size / 1000.0
}
