# Getting Started with rustdv

You have some RTL — maybe an open-source core you just downloaded — and you
want to verify it with a testbench written in Rust. This document takes you
from nothing to a running regression, step by step. It assumes you have
never used rustdv before; it does not assume you know Rust well (the book
*Rust for RTL Verification* teaches that part).

Start by cloning the repository, in Step 1. It brings the framework, the
TinyALU design and its worked testbench, starter templates, the book's runnable
figures, the regression suite, and the skills that let Claude write a testbench
for you. Every step here uses it.

There is one choice to make and it comes later: whether you write the testbench
yourself or have Claude write it. Both are covered at the end, and they mix
well.

---

## Step 0: Install the tools (once per machine)

1. **Rust** — one command, from [rustup.rs](https://rustup.rs):
   ```sh
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustc --version
   ```
   The repository pins its compiler in `rust-toolchain.toml`, so inside a
   clone `rustup` fetches the right one on its own.
2. **Icarus Verilog** — the four-state reference simulator:
   ```sh
   sudo apt install iverilog     # Debian/Ubuntu
   brew install icarus-verilog   # macOS
   # or the prebuilt oss-cad-suite: github.com/YosysHQ/oss-cad-suite-build
   iverilog -V | head -1
   ```
   The run scripts compile with `-g2012`, so you need a build with
   SystemVerilog-2012 support. Any current Icarus has it; the oss-cad-suite
   build is the one this project is tested against.
3. **Verilator** — optional, and the fast two-state backend:
   ```sh
   sudo apt install verilator zlib1g-dev liblz4-dev   # Debian/Ubuntu
   brew install verilator lz4                          # macOS
   verilator --version
   ```
   **lz4 is not optional if you want waveforms.** Verilator's FST writer links
   `liblz4`, so `RUSTDV_VERILATOR_MODE=debug` fails to build without it. Every
   other Verilator mode runs fine.
4. Optional but recommended: **VS Code + rust-analyzer** (use model 1) or
   **Claude Code / Cowork** (use model 2), and **GTKWave** for waveforms.

> Platform note: rustdv's simulator backend is VPI-based and is developed and
> tested on Linux and macOS with Icarus and Verilator. Windows users should
> work inside WSL, and commercial simulators are not supported today.
> `STATUS.md` in the repository records what has actually been run, and where.

## Step 1: Clone rustdv and prove your setup works

Before touching your own core, run the known-good example. This isolates
"my tools are broken" from "my testbench is wrong" — the most valuable
distinction in verification.

```sh
git clone https://github.com/rustdv/rustdv.git
cd rustdv
sim/run_smoke.sh icarus      # checks the simulator alone   → SMOKE: PASS
sim/run_rustdv.sh            # full Rust testbench on TinyALU → REGRESSION: PASS
cd rustdv && cargo test      # unit tests, no simulator      → all green
```

If all three pass, everything you need works. If one fails, fix it now —
nothing later will work until it does.

## Step 2: Set up your project

Keep your project **outside** the rustdv clone, next to it:

```
hobby/
├── rustdv/                  ← the clone from step 1
└── my-core/                 ← yours
    ├── hdl/                 ← the RTL you downloaded
    ├── my_core_tb/          ← the Rust testbench crate (step 3)
    └── sim/
        ├── timescale.v
        └── run.sh
```

Ready-made starting files live in the repository at
`.claude/skills/new-rustdv-testbench/templates/` — a `Cargo.toml`,
`lib.rs`, `run.sh`, and `timescale.v`. Copy them and rename the
placeholders.

Your testbench points at the clone. Two lines in `my_core_tb/Cargo.toml`, and
the paths are relative to that file:

```toml
[dependencies]
rustdv = { path = "../../rustdv/rustdv" }   # → the clone's rustdv/ workspace dir

[dev-dependencies]
rustdv-vpi-stubs = { path = "../../rustdv/rustdv/rustdv-vpi-stubs" }
```

The second one only matters for `cargo test`; Step 3 explains it.

## Step 3: The five files of a first testbench

A rustdv testbench is a Rust library that the simulator loads — there is
no Verilog testbench at all. The minimum is one crate with:

1. **`Cargo.toml`** — two settings matter and both are required:
   `crate-type = ["cdylib", "rlib"]`, plus the `rustdv-vpi-stubs`
   dev-dependency so `cargo test` links. The template has them.
2. **`src/lib.rs`** — starts from the template: `use rustdv::prelude::*;`,
   one `rustdv::vpi_bootstrap!();`, and one `#[rustdv::test]` function
   that starts a clock and touches a signal. Get *this* passing before
   writing any real verification code.
3. **`sim/timescale.v`** — three lines; must be compiled first or Icarus
   defaults to 1-second precision and your nanosecond clock goes wrong.
4. **`sim/run.sh`** — builds the crate, renames the `.so` to `.vpi`,
   compiles your RTL, runs `vvp`. Adapt the template's three variables:
   crate name, top module name, RTL file list.
5. **Your RTL**, unmodified, in `hdl/`.

Run `sim/run.sh`. When you see your smoke test pass, you have a live
Rust-to-simulator connection, and everything from here on is ordinary
(book-shaped) testbench work: BFM → driver and monitors → scoreboard →
sequences. The complete worked example is `rustdv/tinyalu_tb/` in this
repo, and the book's Interlude chapter walks through every file of it.

Two tips for the growth phase:

- Want waveforms? Add a `dump.v` next to your RTL and pass `-s dump` as a
  second top module to iverilog:
  ```verilog
  module dump;
    initial begin $dumpfile("waves.vcd"); $dumpvars(0, my_top); end
  endmodule
  ```
  For Verilator, use DEBUG mode with a `.vlt` control file for a conventional
  FST from time zero. Debug services which need bounded historical windows use
  RECORD mode: the model is trace-capable, but capture is armed and stopped at
  runtime and remains independent of selected VPI visibility.
- A test hangs? The run prints a seed (`RUSTDV_RANDOM_SEED=...`); re-run
  with the same seed and read the log from the top — a panic earlier in
  the log is the cause of a hang later in it, nine times out of ten.

## Use model 1: writing the Rust yourself

Open `my_core_tb/` in your IDE with rust-analyzer. The compiler is your
co-pilot here more than in any language you've used: most testbench wiring
mistakes (wrong transaction type on a port, missing connection, sharing a
value two tasks both mutate) are compile errors, and rust-analyzer shows
them as you type, before any simulation.

Work in this order, and don't skip step 1:

1. Transactions + a predictor function, with `#[test]` unit tests —
   verified in milliseconds with `cargo test`, no simulator.
2. The BFM (reset + one operation), exercised by a directed test.
3. Monitors and a scoreboard; then sequences and random tests.

The `SKILL.md` files under `.claude/skills/` are written for AI agents but
read perfectly well as human checklists — especially `write-a-bfm` (timing
conventions) and `debug-a-regression`.

## Use model 2: letting Claude write it

This repository ships skills that teach Claude rustdv's setup steps,
idioms, and debugging playbook. To use them:

1. Put the rustdv clone and your project in one folder (the `hobby/`
   layout above) and open that folder in Claude Code or Cowork.
2. Claude discovers `.claude/skills/` automatically when working in the
   repo. If your project lives outside it, copy the `.claude/` directory
   to your project root — the skills travel well.
3. Ask for what you want, and name the constraint that matters:

   > Build a rustdv testbench for the SPI core in `my-core/hdl/`, using
   > the skills in this repo and `rustdv/tinyalu_tb` as the reference.
   > Start with a smoke test that proves the clock and reset work, show
   > me the run output, and only then grow the BFM and scoreboard.

4. Hold Claude to the same standard you'd hold yourself: every stage ends
   with a *running* regression (`REGRESSION: PASS` is the only acceptable
   "done"), and a scoreboard isn't finished until a deliberately broken
   DUT makes it fail. The `debug-a-regression` skill shows the mutation
   test; ask Claude to run it.

The two models mix well: let Claude scaffold the project and the BFM's
timing loops, then write the interesting parts — the predictor, the
sequences, the checks — yourself. That's where the verification thinking
lives, and it's the part worth learning by hand.

## Where to go next

- **The book** (`book-pdf/`): chapters 1–14 teach the Rust; the Interlude
  walks the complete TinyALU testbench end to end.
- **`rustdv/tinyalu_tb/`**: the living reference — every pattern in the
  skills appears there in context.
- **The repository `README.md`**: the same testbench walked component by
  component, with the UVM-concept map.
- **`STATUS.md`**: the implementation history and the deviations log — what has
  been run, on what, and what is still missing.
