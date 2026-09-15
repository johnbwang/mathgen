//! A deliberately small PDF writer: enough to place text and rules on pages
//! using the two built-in Helvetica faces. No compression, no embedded fonts.

use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::Path;

use crate::font::{self, Font};

pub struct Pdf {
    width: f32,
    height: f32,
    pages: Vec<String>,
}

/// Handle to the page currently being drawn.
pub struct Page<'a> {
    content: &'a mut String,
}

impl Pdf {
    pub fn new(width: f32, height: f32) -> Self {
        Pdf { width, height, pages: Vec::new() }
    }

    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn height(&self) -> f32 {
        self.height
    }

    /// Begin a new page and return a handle for drawing on it.
    pub fn page(&mut self) -> Page<'_> {
        self.pages.push(String::new());
        Page { content: self.pages.last_mut().expect("just pushed") }
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        fs::write(path, self.render())
    }

    fn render(&self) -> Vec<u8> {
        // Object ids: 1 catalog, 2 page tree, 3/4 fonts, then two per page.
        let page_obj = |i: usize| 5 + 2 * i;
        let content_obj = |i: usize| 6 + 2 * i;
        let total_objs = 4 + 2 * self.pages.len();

        let mut out: Vec<u8> = Vec::new();
        let mut offsets: Vec<usize> = vec![0; total_objs + 1];

        out.extend_from_slice(b"%PDF-1.4\n");
        // A comment with high bytes marks the file as binary for transfer tools.
        out.extend_from_slice(b"%\xE2\xE3\xCF\xD3\n");

        let obj = |out: &mut Vec<u8>, offsets: &mut Vec<usize>, id: usize, body: &str| {
            offsets[id] = out.len();
            out.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
        };

        obj(&mut out, &mut offsets, 1, "<< /Type /Catalog /Pages 2 0 R >>");

        let kids: String = (0..self.pages.len())
            .map(|i| format!("{} 0 R ", page_obj(i)))
            .collect();
        obj(
            &mut out,
            &mut offsets,
            2,
            &format!(
                "<< /Type /Pages /Count {} /Kids [ {}] >>",
                self.pages.len(),
                kids
            ),
        );

        obj(&mut out, &mut offsets, 3,
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>");
        obj(&mut out, &mut offsets, 4,
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>");

        for (i, content) in self.pages.iter().enumerate() {
            let page = format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 {} {} ] \
                 /Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> /Contents {} 0 R >>",
                num(self.width),
                num(self.height),
                content_obj(i)
            );
            obj(&mut out, &mut offsets, page_obj(i), &page);

            // Content streams carry WinAnsi bytes, so build this one by hand.
            let bytes = to_winansi(content);
            let id = content_obj(i);
            offsets[id] = out.len();
            out.extend_from_slice(
                format!("{id} 0 obj\n<< /Length {} >>\nstream\n", bytes.len()).as_bytes(),
            );
            out.extend_from_slice(&bytes);
            out.extend_from_slice(b"\nendstream\nendobj\n");
        }

        let startxref = out.len();
        out.extend_from_slice(format!("xref\n0 {}\n", total_objs + 1).as_bytes());
        out.extend_from_slice(b"0000000000 65535 f \n");
        for id in 1..=total_objs {
            // Every entry must be exactly 20 bytes.
            out.extend_from_slice(format!("{:010} 00000 n \n", offsets[id]).as_bytes());
        }
        out.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
                total_objs + 1,
                startxref
            )
            .as_bytes(),
        );

        out
    }
}

impl Page<'_> {
    /// Set the fill (text) colour as a gray level, 0 = black, 1 = white.
    pub fn fill_gray(&mut self, g: f32) {
        let _ = writeln!(self.content, "{} g", num(g));
    }

    /// Draw `text` with its left edge at `x` and its baseline at `y`.
    pub fn text(&mut self, text: &str, f: Font, size: f32, x: f32, y: f32) {
        let _ = writeln!(
            self.content,
            "BT /{} {} Tf {} {} Td ({}) Tj ET",
            f.resource(),
            num(size),
            num(x),
            num(y),
            escape(text)
        );
    }

    /// Draw `text` with its right edge at `x`.
    pub fn text_right(&mut self, text: &str, f: Font, size: f32, x: f32, y: f32) {
        self.text(text, f, size, x - font::width(text, f, size), y);
    }

    /// Draw `text` centred on `x`.
    pub fn text_centre(&mut self, text: &str, f: Font, size: f32, x: f32, y: f32) {
        self.text(text, f, size, x - font::width(text, f, size) / 2.0, y);
    }

    pub fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, weight: f32, gray: f32) {
        let _ = writeln!(
            self.content,
            "{} G {} w {} {} m {} {} l S",
            num(gray),
            num(weight),
            num(x1),
            num(y1),
            num(x2),
            num(y2)
        );
    }
}

/// Format a number for a content stream: trimmed to 2 decimals, no exponent.
fn num(v: f32) -> String {
    let mut s = format!("{v:.2}");
    if s.contains('.') {
        s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    }
    if s == "-0" {
        s = "0".to_string();
    }
    s
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(c, '(' | ')' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Encode to WinAnsiEncoding, which is what both font objects declare.
fn to_winansi(s: &str) -> Vec<u8> {
    s.chars()
        .map(|c| match c {
            '\u{0000}'..='\u{007E}' => c as u8,
            '\u{2013}' => 0x96,
            '\u{2014}' => 0x97,
            '\u{2018}' => 0x91,
            '\u{2019}' => 0x92,
            '\u{201C}' => 0x93,
            '\u{201D}' => 0x94,
            '\u{2022}' => 0x95,
            // Latin-1 maps straight through in WinAnsi above U+00A0.
            '\u{00A0}'..='\u{00FF}' => c as u32 as u8,
            _ => b'?',
        })
        .collect()
}
