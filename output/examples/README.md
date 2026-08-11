# Rust for RTL Verification — Runnable Examples

Every figure in the book, organized so the file name (or chapter crate)
tells you exactly which figure you are looking at.

**Part I (ch 1–14):** every figure extracted verbatim into per-figure files,
verified by the book-sync regression suite.

**Parts II–V (ch 15–41):** most figures need a simulator, so each chapter is
a crate whose `#[rustdv::test]` functions are the figures. Run a chapter with
`sim-common/run_sim.sh <crate> <top> [hdl...]` (each chapter README gives the
exact command); every chapter ends `REGRESSION: PASS` on Icarus, enforced by
the `custom/sim-chNN` regression tests. Shared testbench code lives in
`tinyalu-utils/` (the Rust `tinyalu_utils`), pure-Rust figures in
`src/bin/`, and intentional compile errors in `compile-fail/` as before. 105 figures total: 70 normal programs, 20 intentional
compile errors, 3 intentional panics, 1 unit-test figure, 3 non-runnable
fragments, 2 Python contrast figures, and 6 shell transcripts.

Icarus remains the source of the book's four-state transcripts. Set
`SIM=verilator` when using `sim-common/run_sim.sh` to run the same Rust
testbench in Verilator's faster two-state mode.

## Naming convention

```
ch03-rust-basics/                      chapter 3 (slug matches the book source file)
├── README.md                          figure → file map for the chapter
├── src/bin/
│   └── ch03_fig03_mutable_binding.rs  Chapter 3, Figure 3
└── compile-fail/
    └── fig02_assigning_twice_immutable_binding/
        ├── src/main.rs                Chapter 3, Figure 2 (fails to compile on purpose)
        └── EXPECTED.txt               the compiler error the book shows
```

`chNN_figMM_*` = Chapter NN, Figure MM. Every `.rs` file begins with a header
comment giving the chapter, figure number, title, run command, and the exact
output (or compiler error) the book shows. `manifest.json` is a
machine-readable map of every figure.

Some figures build on definitions from earlier figures. Those files carry a
clearly marked block:

```rust
// ---- Context from Chapter 8, Figure 1 (definitions this figure needs) ----
...
// ---- Figure code (verbatim from the book) ----
```

Every figure matches the book source verbatim. **ERRATA.md** records three
manuscript bugs found (and fixed) in the process of making the figures run.

## How to run things

| What | Command (from this directory) |
|---|---|
| One figure | `cargo run --bin ch03_fig03_mutable_binding` |
| Build everything runnable | `cargo build --workspace` |
| A compile-error figure | `cd ch03-rust-basics/compile-fail/fig02_* && cargo build` — read the error, compare with `EXPECTED.txt` |
| The unit-test figure (Ch 14, Fig 6) | `cargo test -p ch14_modules_crates_cargo` |
| Python contrast figures (Ch 5 Fig 1, Ch 13 Fig 2) | `python3 ch05-ownership/python/fig01_*.py` |
| Everything, checked against the book | `./check.sh` |

Figures that panic on purpose (Ch 9 Figs 2 and 7, Ch 13 Fig 6) run with
`cargo run` like any other; the nonzero exit is the lesson, and the header
comment says so.

Shell-transcript figures (e.g. `cargo new`, `cargo test` runs) are reproduced
in each chapter's README rather than as source files.

## check.sh

Verifies the book's promise that the examples run:

1. `cargo build --workspace` — all 70 runnable figures compile
2. runs every binary — normal figures must exit 0, panic figures must not
3. builds all 20 compile-fail crates — each must fail with the error code in its `EXPECTED.txt`
4. `cargo test` on the Chapter 14 test figure — tests must pass
5. (informational) diffs each program's stdout against the output printed in the book

Requires a Rust toolchain (`rustup`, stable, edition 2021 / rustc ≥ 1.60).

## Open questions / caveats

- Generated and cross-checked against the book source, but **not yet
  compile-verified in the generation environment** (no Rust toolchain
  available there). `./check.sh` on your machine is the authoritative check.
- A few figures print `HashMap` contents, whose iteration order is
  unspecified; step 5 of `check.sh` compares those order-insensitively and
  reports differences as warnings, not failures.
- Chapter 10 Fig 4 and Chapter 11 Figs 5–6 are marked in the book as
  signature-only previews ("you never see this"), so they are kept as
  non-building fragments in `fragments/` rather than forced to compile.
