//! Problem generation, plus the small PRNG that drives it.

use std::collections::HashSet;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Op {
    Add,
    Sub,
    Mul,
}

impl Op {
    pub fn symbol(self) -> &'static str {
        match self {
            Op::Add => "+",
            Op::Sub => "\u{2013}", // en dash reads as a minus in Helvetica
            Op::Mul => "\u{00D7}",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Op::Add => "addition",
            Op::Sub => "subtraction",
            Op::Mul => "multiplication",
        }
    }

    pub fn parse(s: &str) -> Option<Op> {
        match s.trim().to_ascii_lowercase().as_str() {
            "add" | "addition" | "+" => Some(Op::Add),
            "sub" | "subtract" | "subtraction" | "-" => Some(Op::Sub),
            "mul" | "multiply" | "multiplication" | "x" | "*" => Some(Op::Mul),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Problem {
    pub a: i64,
    pub b: i64,
    pub op: Op,
}

impl Problem {
    pub fn answer(&self) -> i64 {
        match self.op {
            Op::Add => self.a + self.b,
            Op::Sub => self.a - self.b,
            Op::Mul => self.a * self.b,
        }
    }

    /// One-line form used by the answer key.
    pub fn inline(&self) -> String {
        format!("{} {} {} = {}", self.a, self.op.symbol(), self.b, self.answer())
    }
}

/// SplitMix64 — tiny, seeds cleanly from any value, plenty for worksheets.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform-ish integer in `lo..=hi`.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }
}

/// Draw one problem, retrying until it satisfies the constraints.
fn make(op: Op, lo: i64, hi: i64, simple: bool, rng: &mut Rng) -> Option<Problem> {
    for _ in 0..4000 {
        let (mut a, mut b) = (rng.range(lo, hi), rng.range(lo, hi));

        match op {
            Op::Add => {
                if simple && (a % 10 + b % 10 > 9 || a / 10 + b / 10 > 9) {
                    continue; // would require carrying
                }
                return Some(Problem { a, b, op });
            }
            Op::Sub => {
                if a < b {
                    std::mem::swap(&mut a, &mut b);
                }
                if a == b {
                    continue; // answers of 0 are a dull worksheet
                }
                if simple && a % 10 < b % 10 {
                    continue; // would require borrowing
                }
                return Some(Problem { a, b, op });
            }
            Op::Mul => return Some(Problem { a, b, op }),
        }
    }
    None
}

/// Build `count` problems, cycling through `ops` and avoiding repeats.
pub fn generate(
    count: usize,
    ops: &[Op],
    lo: i64,
    hi: i64,
    simple: bool,
    rng: &mut Rng,
) -> Result<Vec<Problem>, String> {
    let mut problems = Vec::with_capacity(count);
    let mut seen: HashSet<Problem> = HashSet::new();

    for i in 0..count {
        let op = ops[i % ops.len()];
        let mut chosen = None;

        for _ in 0..200 {
            let p = make(op, lo, hi, simple, rng).ok_or_else(|| {
                format!(
                    "no {} problem fits the range {}-{}{} \u{2014} try widening --min/--max",
                    op.name(),
                    lo,
                    hi,
                    if simple { " with --simple" } else { "" }
                )
            })?;
            chosen = Some(p);
            if seen.insert(p) {
                break;
            }
            // else: try again for a fresh one, but keep this as the fallback
        }

        problems.push(chosen.expect("loop runs at least once"));
    }

    Ok(problems)
}
