# mathgen

A single-binary CLI that generates printable PDF math worksheets with problems
stacked vertically (column format) in a big, kid-friendly font.

```
     51            79
 +   29        −   26
 ─────────     ─────────
```

Written in Rust with **no dependencies** — not even a PDF crate. It emits PDF
directly using the two built-in Helvetica faces, so the binary is self-contained
and there is nothing to install at runtime.

## Install

```sh
cargo install --path .
```

That puts `mathgen` in `~/.cargo/bin`, which is already on your PATH. Or just
build it and use the binary in place:

```sh
cargo build --release
./target/release/mathgen
```

## Usage

```sh
mathgen                                    # 18 addition problems -> math-worksheet.pdf
mathgen --ops add,sub -n 36 --answers      # mixed, with an answer key
mathgen --simple                           # no carrying or borrowing
mathgen --cols 2 --rows 3 --font-size 80   # fewer, even bigger problems
mathgen --ops mul --min 2 --max 12 -n 30   # times tables
```

## Options

| Flag | Default | What it does |
| --- | --- | --- |
| `-n, --count <N>` | `18` | How many problems to generate |
| `-o, --output <PATH>` | `math-worksheet.pdf` | Output path |
| `--ops <LIST>` | `add` | Comma-separated: `add`, `sub`, `mul` — cycled in order |
| `--min <N>` / `--max <N>` | `10` / `99` | Operand range (default is double digits) |
| `--simple` | off | No carrying or borrowing — easiest for beginners |
| `--cols <N>` / `--rows <N>` | `3` / `3` | Grid shape, so 9 problems per page |
| `--font-size <PT>` | `52` | Digit size in points; shrinks automatically to fit |
| `--paper <NAME>` | `letter` | `letter` or `a4` |
| `--title <TEXT>` | `Math Practice` | Worksheet heading |
| `--subtitle <TEXT>` | — | Optional line under the heading |
| `--answers` | off | Append an answer key |
| `--seed <N>` | — | Reproducible worksheets |

Both `--flag value` and `--flag=value` spellings work.

## Notes

- Subtraction never produces a negative answer (operands are swapped if needed)
  and never produces zero.
- `--simple` guarantees no regrouping: addition columns never carry (so sums
  stay two digits) and subtraction columns never borrow.
- Problems are unique within a worksheet where the chosen range allows it.
- `--font-size` is a ceiling, not a promise: if the requested size won't fit the
  `--cols` x `--rows` grid, mathgen shrinks it until it does and reports the
  size it used.

## Layout

| Module | Role |
| --- | --- |
| `src/main.rs` | Argument parsing, wiring, tests |
| `src/problem.rs` | Problem generation and the seeded PRNG |
| `src/layout.rs` | Worksheet geometry — grid, stacked problems, answer key |
| `src/pdf.rs` | Minimal PDF writer (text, rules, xref table) |
| `src/font.rs` | Helvetica advance widths, for right-aligning digits |

```sh
cargo test   # 9 tests covering arithmetic, constraints, and CLI parsing
```
