//! mathgen — generate printable PDF worksheets of vertical (column-format)
//! double-digit math problems, sized for young kids.

mod font;
mod layout;
mod pdf;
mod problem;

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use pdf::Pdf;
use problem::{Op, Rng};

const VERSION_LINE: &str = concat!("mathgen ", env!("CARGO_PKG_VERSION"), "\n");

const USAGE: &str = "\
mathgen — printable PDF worksheets of stacked double-digit math problems

USAGE:
    mathgen [OPTIONS]

OPTIONS:
    -n, --count <N>        how many problems to generate [default: 18]
    -o, --output <PATH>    output PDF path [default: math-worksheet.pdf]
        --ops <LIST>       comma-separated: add, sub, mul [default: add]
        --min <N>          smallest operand [default: 10]
        --max <N>          largest operand [default: 99]
        --simple           no carrying or borrowing — easiest for beginners
        --cols <N>         problems per row [default: 3]
        --rows <N>         rows per page [default: 3]
        --font-size <PT>   digit size; shrinks automatically to fit [default: 52]
        --paper <NAME>     letter or a4 [default: letter]
        --title <TEXT>     worksheet heading [default: Math Practice]
        --subtitle <TEXT>  optional line under the heading
        --answers          append an answer key
        --seed <N>         random seed, for reproducible worksheets
    -h, --help             show this help
    -V, --version          show version

EXAMPLES:
    mathgen                                  18 addition problems
    mathgen --ops add,sub -n 36 --answers    mixed, with an answer key
    mathgen --simple                         no carrying or borrowing
    mathgen --cols 2 --rows 3 --font-size 80 fewer, even bigger problems
    mathgen --ops mul --min 2 --max 12       times tables
";

struct Config {
    count: usize,
    output: PathBuf,
    ops: Vec<Op>,
    lo: i64,
    hi: i64,
    simple: bool,
    cols: usize,
    rows: usize,
    font_size: f32,
    paper: (f32, f32),
    title: String,
    subtitle: Option<String>,
    answers: bool,
    seed: Option<u64>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            count: 18,
            output: PathBuf::from("math-worksheet.pdf"),
            ops: vec![Op::Add],
            lo: 10,
            hi: 99,
            simple: false,
            cols: 3,
            rows: 3,
            font_size: 52.0,
            paper: (612.0, 792.0),
            title: "Math Practice".to_string(),
            subtitle: None,
            answers: false,
            seed: None,
        }
    }
}

/// What `parse` decided we should do.
enum Action {
    Run(Box<Config>),
    Print(&'static str),
}

fn parse(args: Vec<String>) -> Result<Action, String> {
    let mut cfg = Config::default();
    let mut it = args.into_iter().peekable();

    // Pull the value for a flag, accepting both `--flag value` and `--flag=value`.
    fn value(
        flag: &str,
        inline: Option<String>,
        it: &mut std::iter::Peekable<std::vec::IntoIter<String>>,
    ) -> Result<String, String> {
        inline
            .or_else(|| it.next())
            .ok_or_else(|| format!("{flag} needs a value"))
    }

    fn number<T: std::str::FromStr>(flag: &str, raw: &str) -> Result<T, String> {
        raw.parse()
            .map_err(|_| format!("{flag} expects a number, got {raw:?}"))
    }

    while let Some(arg) = it.next() {
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f.to_string(), Some(v.to_string())),
            _ => (arg, None),
        };

        match flag.as_str() {
            "-h" | "--help" => return Ok(Action::Print(USAGE)),
            "-V" | "--version" => return Ok(Action::Print(VERSION_LINE)),
            "--simple" => cfg.simple = true,
            "--answers" => cfg.answers = true,
            "-n" | "--count" => {
                cfg.count = number("--count", &value(&flag, inline, &mut it)?)?;
                if cfg.count == 0 {
                    return Err("--count must be 1 or greater".into());
                }
            }
            "-o" | "--output" => cfg.output = PathBuf::from(value(&flag, inline, &mut it)?),
            "--ops" => {
                let raw = value(&flag, inline, &mut it)?;
                let mut ops = Vec::new();
                for part in raw.split(',').filter(|s| !s.trim().is_empty()) {
                    let op = Op::parse(part).ok_or_else(|| {
                        format!("unknown operation {part:?} (choose from add, sub, mul)")
                    })?;
                    if !ops.contains(&op) {
                        ops.push(op);
                    }
                }
                if ops.is_empty() {
                    return Err("--ops needs at least one operation".into());
                }
                cfg.ops = ops;
            }
            "--min" => cfg.lo = number("--min", &value(&flag, inline, &mut it)?)?,
            "--max" => cfg.hi = number("--max", &value(&flag, inline, &mut it)?)?,
            "--cols" => cfg.cols = number("--cols", &value(&flag, inline, &mut it)?)?,
            "--rows" => cfg.rows = number("--rows", &value(&flag, inline, &mut it)?)?,
            "--font-size" => cfg.font_size = number("--font-size", &value(&flag, inline, &mut it)?)?,
            "--paper" => {
                let raw = value(&flag, inline, &mut it)?;
                cfg.paper = match raw.to_ascii_lowercase().as_str() {
                    "letter" => (612.0, 792.0),
                    "a4" => (595.28, 841.89),
                    other => return Err(format!("unknown paper {other:?} (choose letter or a4)")),
                };
            }
            "--title" => cfg.title = value(&flag, inline, &mut it)?,
            "--subtitle" => cfg.subtitle = Some(value(&flag, inline, &mut it)?),
            "--seed" => cfg.seed = Some(number("--seed", &value(&flag, inline, &mut it)?)?),
            other => {
                return Err(format!(
                    "unrecognized argument {other:?}\n\nRun `mathgen --help` for usage."
                ))
            }
        }
    }

    if cfg.lo > cfg.hi {
        return Err("--min cannot be greater than --max".into());
    }
    if cfg.lo < 0 {
        return Err("--min cannot be negative".into());
    }
    if cfg.cols == 0 || cfg.rows == 0 {
        return Err("--cols and --rows must be 1 or greater".into());
    }
    if !(cfg.font_size.is_finite() && cfg.font_size > 0.0) {
        return Err("--font-size must be a positive number".into());
    }

    Ok(Action::Run(Box::new(cfg)))
}

fn run(cfg: &Config) -> Result<String, String> {
    let seed = cfg.seed.unwrap_or_else(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x5EED)
    });
    let mut rng = Rng::new(seed);

    let problems = problem::generate(cfg.count, &cfg.ops, cfg.lo, cfg.hi, cfg.simple, &mut rng)?;

    let mut doc = Pdf::new(cfg.paper.0, cfg.paper.1);
    let (pages, size) = layout::build(
        &mut doc,
        &problems,
        &layout::Options {
            cols: cfg.cols,
            rows: cfg.rows,
            font_size: cfg.font_size,
            title: &cfg.title,
            subtitle: cfg.subtitle.as_deref(),
            answers: cfg.answers,
        },
    );

    doc.save(&cfg.output)
        .map_err(|e| format!("could not write {}: {e}", cfg.output.display()))?;

    let names: Vec<&str> = cfg.ops.iter().map(|o| o.name()).collect();
    let n = problems.len();
    Ok(format!(
        "{}: {n} {} problem{} across {pages} page{}{} ({size:.0}pt digits)",
        cfg.output.display(),
        names.join(", "),
        if n == 1 { "" } else { "s" },
        if pages == 1 { "" } else { "s" },
        if cfg.answers { " + answer key" } else { "" },
    ))
}

fn main() -> ExitCode {
    match parse(std::env::args().skip(1).collect()) {
        Ok(Action::Print(text)) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Ok(Action::Run(cfg)) => match run(&cfg) {
            Ok(summary) => {
                println!("{summary}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::from(1)
            }
        },
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn problems(ops: &[Op], simple: bool, seed: u64) -> Vec<problem::Problem> {
        let mut rng = Rng::new(seed);
        problem::generate(40, ops, 10, 99, simple, &mut rng).expect("generates")
    }

    #[test]
    fn answers_are_correct() {
        for seed in 0..200 {
            for p in problems(&[Op::Add, Op::Sub, Op::Mul], false, seed) {
                let expected = match p.op {
                    Op::Add => p.a + p.b,
                    Op::Sub => p.a - p.b,
                    Op::Mul => p.a * p.b,
                };
                assert_eq!(p.answer(), expected);
            }
        }
    }

    #[test]
    fn subtraction_is_never_negative_or_zero() {
        for seed in 0..200 {
            for p in problems(&[Op::Sub], false, seed) {
                assert!(p.answer() > 0, "{} - {}", p.a, p.b);
            }
        }
    }

    #[test]
    fn simple_never_carries_or_borrows() {
        for seed in 0..200 {
            for p in problems(&[Op::Add, Op::Sub], true, seed) {
                match p.op {
                    Op::Add => {
                        assert!(p.a % 10 + p.b % 10 <= 9, "carry in {} + {}", p.a, p.b);
                        assert!(p.answer() <= 99, "3 digits in {} + {}", p.a, p.b);
                    }
                    Op::Sub => assert!(p.a % 10 >= p.b % 10, "borrow in {} - {}", p.a, p.b),
                    Op::Mul => {}
                }
            }
        }
    }

    #[test]
    fn operands_stay_in_range() {
        for seed in 0..100 {
            let mut rng = Rng::new(seed);
            for p in problem::generate(50, &[Op::Add, Op::Sub], 20, 40, false, &mut rng).unwrap() {
                assert!((20..=40).contains(&p.a) && (20..=40).contains(&p.b));
            }
        }
    }

    #[test]
    fn problems_are_unique_when_the_range_allows() {
        let mut rng = Rng::new(7);
        let ps = problem::generate(30, &[Op::Add], 10, 99, false, &mut rng).unwrap();
        let unique: std::collections::HashSet<_> = ps.iter().collect();
        assert_eq!(unique.len(), ps.len());
    }

    #[test]
    fn seeds_are_reproducible() {
        let a = problems(&[Op::Add], false, 42);
        let b = problems(&[Op::Add], false, 42);
        assert!(a.iter().zip(&b).all(|(x, y)| x.a == y.a && x.b == y.b));
    }

    #[test]
    fn impossible_constraints_report_an_error() {
        let mut rng = Rng::new(1);
        assert!(problem::generate(5, &[Op::Add], 90, 99, true, &mut rng).is_err());
    }

    #[test]
    fn rejects_bad_arguments() {
        let bad = [
            vec!["--min", "50", "--max", "10"],
            vec!["--ops", "divide"],
            vec!["--count", "0"],
            vec!["--cols", "0"],
            vec!["--nonsense"],
            vec!["--count"],
        ];
        for args in bad {
            let owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
            assert!(parse(owned).is_err(), "expected {args:?} to fail");
        }
    }

    #[test]
    fn accepts_both_flag_spellings() {
        let joined = parse(vec!["--count=7".into(), "--ops=add,sub".into()]).expect("parses");
        let split = parse(vec!["--count".into(), "7".into(), "--ops".into(), "add,sub".into()])
            .expect("parses");
        for action in [joined, split] {
            match action {
                Action::Run(cfg) => {
                    assert_eq!(cfg.count, 7);
                    assert_eq!(cfg.ops, vec![Op::Add, Op::Sub]);
                }
                Action::Print(_) => panic!("expected a run"),
            }
        }
    }
}
