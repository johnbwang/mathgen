//! Worksheet geometry: stacked problems arranged on a grid, plus the answer key.

use crate::font::{self, Font};
use crate::pdf::{Page, Pdf};
use crate::problem::Problem;

pub const MARGIN: f32 = 0.6 * 72.0;

pub struct Options<'a> {
    pub cols: usize,
    pub rows: usize,
    pub font_size: f32,
    pub title: &'a str,
    pub subtitle: Option<&'a str>,
    pub answers: bool,
}

/// (block width, gap between operator and number) for one problem.
fn block_metrics(size: f32, digits: usize, symbol: &str) -> (f32, f32) {
    let digits_w = font::width(&"8".repeat(digits), Font::Bold, size);
    let op_w = font::width(symbol, Font::Bold, size);
    let gap = size * 0.45;
    (op_w + gap + digits_w, gap)
}

/// Total vertical space one problem needs, including the blank answer area.
fn block_height(size: f32) -> f32 {
    size * 1.15 * 3.0 + size * 0.35
}

/// Shrink the requested size until a problem fits its grid cell.
fn fit_font_size(requested: f32, cell_w: f32, cell_h: f32, digits: usize) -> f32 {
    let mut size = requested;
    while size > 8.0 {
        let (w, _) = block_metrics(size, digits, "\u{2013}");
        if w <= cell_w * 0.88 && block_height(size) <= cell_h * 0.92 {
            return size;
        }
        size -= 1.0;
    }
    size
}

/// Draw the page header; returns the y coordinate where the grid may start.
fn header(page: &mut Page, width: f32, height: f32, title: &str, subtitle: Option<&str>,
          name_line: bool) -> f32 {
    let mut y = height - MARGIN;

    page.fill_gray(0.0);
    page.text(title, Font::Bold, 20.0, MARGIN, y - 16.0);

    if name_line {
        page.fill_gray(0.35);
        page.text_right(
            "Name: ______________________   Date: ____________",
            Font::Regular,
            11.0,
            width - MARGIN,
            y - 16.0,
        );
        page.fill_gray(0.0);
    }

    y -= 26.0;
    if let Some(sub) = subtitle {
        page.fill_gray(0.45);
        page.text(sub, Font::Regular, 10.0, MARGIN, y - 10.0);
        page.fill_gray(0.0);
        y -= 14.0;
    }

    page.line(MARGIN, y - 6.0, width - MARGIN, y - 6.0, 1.0, 0.8);
    y - 6.0
}

/// Draw one stacked problem; `(x, y_top)` is the top-left of its cell.
fn problem(page: &mut Page, p: &Problem, x: f32, y_top: f32, cell_w: f32, size: f32,
           digits: usize) {
    let (block_w, _) = block_metrics(size, digits, p.op.symbol());
    let line_h = size * 1.15;
    let left = x + (cell_w - block_w) / 2.0;
    let right = left + block_w;

    let baseline1 = y_top - size * 1.05;
    let baseline2 = baseline1 - line_h;

    page.text_right(&p.a.to_string(), Font::Bold, size, right, baseline1);
    page.text(p.op.symbol(), Font::Bold, size, left, baseline2);
    page.text_right(&p.b.to_string(), Font::Bold, size, right, baseline2);

    let rule_y = baseline2 - size * 0.30;
    let overhang = size * 0.14;
    page.line(
        left - overhang,
        rule_y,
        right + overhang,
        rule_y,
        (size * 0.045).max(1.5),
        0.0,
    );
}

/// Append answer-key pages, filling columns evenly left to right.
fn answer_key(pdf: &mut Pdf, problems: &[Problem], title: &str) {
    let (width, height) = (pdf.width(), pdf.height());
    let heading = format!("{title} \u{2014} Answer Key");
    let cols = 4;
    let col_w = (width - 2.0 * MARGIN) / cols as f32;
    let line_h = 20.0;

    // Measure the usable rows once, off a probe of the same header geometry.
    let probe_top = height - MARGIN - 26.0 - 6.0;
    let max_rows = (((probe_top - 24.0 - MARGIN) / line_h) as usize).max(1);
    let per_page = cols * max_rows;

    for (page_index, chunk) in problems.chunks(per_page).enumerate() {
        let mut page = pdf.page();
        let top = header(&mut page, width, height, &heading, None, false);
        // Balance the columns rather than filling the first one to the bottom.
        let rows = max_rows.min(chunk.len().div_ceil(cols));

        for (i, p) in chunk.iter().enumerate() {
            let (col, row) = (i / rows, i % rows);
            let x = MARGIN + col as f32 * col_w;
            let y = top - 24.0 - row as f32 * line_h;
            let n = page_index * per_page + i + 1;
            page.text(&format!("{n}.  {}", p.inline()), Font::Regular, 12.0, x, y);
        }
    }
}

/// Build the whole document. Returns (pages of problems, font size actually used).
pub fn build(pdf: &mut Pdf, problems: &[Problem], opts: &Options) -> (usize, f32) {
    let (width, height) = (pdf.width(), pdf.height());
    let digits = problems
        .iter()
        .map(|p| p.a.to_string().len().max(p.b.to_string().len()))
        .max()
        .unwrap_or(2);

    let per_page = opts.cols * opts.rows;
    let page_count = problems.len().div_ceil(per_page);

    // Size the grid off page one so every page shares the same geometry.
    let probe_top = height - MARGIN - if opts.subtitle.is_some() { 40.0 } else { 26.0 } - 6.0;
    let cell_w = (width - 2.0 * MARGIN) / opts.cols as f32;
    let size = fit_font_size(opts.font_size, cell_w, (probe_top - MARGIN) / opts.rows as f32, digits);

    for (page_index, chunk) in problems.chunks(per_page).enumerate() {
        let mut page = pdf.page();
        let top = header(&mut page, width, height, opts.title, opts.subtitle, true);
        let cell_h = (top - MARGIN) / opts.rows as f32;

        for (i, p) in chunk.iter().enumerate() {
            let (col, row) = (i % opts.cols, i / opts.cols);
            let x = MARGIN + col as f32 * cell_w;
            let y_top = top - row as f32 * cell_h - (cell_h - block_height(size)) / 2.0;
            problem(&mut page, p, x, y_top, cell_w, size, digits);
        }

        if page_count > 1 {
            page.fill_gray(0.55);
            let label = format!("Page {} of {}", page_index + 1, page_count);
            page.text_centre(&label, Font::Regular, 9.0, width / 2.0, MARGIN * 0.55);
            page.fill_gray(0.0);
        }
    }

    if opts.answers {
        answer_key(pdf, problems, opts.title);
    }

    (page_count, size)
}
