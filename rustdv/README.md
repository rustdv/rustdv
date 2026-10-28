# rustdv

[![crates.io](https://img.shields.io/crates/v/rustdv.svg)](https://crates.io/crates/rustdv)

rustdv combines simulator-driven Rust coroutines with a UVM-inspired
verification library. This guide takes you from a working simulator to your
own testbench, then covers typed signal access and API migration. It assumes
no previous use of rustdv; *Rust for RTL Verification* teaches the Rust along
the way.

The setup below uses a repository clone, which includes the TinyALU design,
worked testbench, runnable book examples, simulator launchers, and verification
skills. To depend on the published framework instead, run `cargo add rustdv`
and `cargo add --dev rustdv-vpi-stubs` in your testbench crate. The library
configuration and simulator-launch requirements still apply.

- [Install the tools](#install-the-tools)
- [Run the example](#run-the-example)
- [Set up your project](#set-up-your-project)
- [Build your first testbench](#build-your-first-testbench)
- [Read and write simulator values](#reading-and-writing-simulator-values)
- [Choose simulator features](#simulator-features)
- [Grow and verify the testbench](#grow-and-verify-the-testbench)
- [Update an existing testbench](#updating-an-existing-testbench)

Repository paths and launch commands below are relative to the clone’s root,
except dependency paths, which are relative to the testbench manifest.

## Install the tools

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
   SystemVerilog-2012 support. CI uses the pinned Icarus build installed by
   `ci/install-iverilog.sh`; use that build if an older package fails the
   next-time-step checks.
3. **Verilator** — optional, and the fast two-state backend:
   ```sh
   sudo apt install verilator zlib1g-dev liblz4-dev   # Debian/Ubuntu
   brew install verilator lz4                          # macOS
   verilator --version
   ```
   The shared host requires Verilator 5.050 or newer for runtime VPI loading.
   **Install lz4 if you want waveforms.** Both DEBUG and RECORD modes build
   Verilator's FST writer and need its development headers and library.
4. Optional but recommended: **VS Code + rust-analyzer**,
   an **AI coding assistant**, and **GTKWave** for waveforms.

> Platform note: rustdv's simulator backend is VPI-based and is developed and
> tested on Linux and macOS with Icarus and Verilator. Windows users should
> work inside WSL, and commercial simulators are not supported today.
> `STATUS.md` in the repository records what has actually been run, and where.

## Run the example

Run the known-good example before using your own RTL, so toolchain problems
are separate from testbench bugs.

```sh
git clone https://github.com/rustdv/rustdv.git
cd rustdv
sim/run_smoke.sh icarus      # checks the simulator alone   → SMOKE: PASS
sim/run_rustdv.sh            # full Rust testbench on TinyALU → REGRESSION: PASS
cargo test --workspace       # unit tests, no simulator      → all green
```

These commands check the Icarus/Rust path and pure-Rust unit tests. For
Verilator, also run `sim/run_rustdv.sh release verilator` before using your own
design. Resolve any setup failure before growing the testbench.

## Set up your project

Keep your project **outside** the rustdv clone, next to it:

```
hobby/
├── rustdv/                  ← the framework clone
└── my-core/                 ← yours
    ├── hdl/                 ← the RTL you downloaded
    ├── my_core_tb/          ← your Rust testbench crate
    └── sim/
        ├── timescale.v
        └── run.sh
```

Use `rustdv/tinyalu_tb/Cargo.toml` and its `src/` directory as the worked
example. For the Icarus launch sequence, use
`output/examples/sim-common/run_sim.sh` as a reference and adapt its crate,
top-module, and RTL paths for your project.

Create a library crate at `my-core/my_core_tb`. Add these sections to its
`Cargo.toml`; dependency paths are relative to that manifest:

```toml
[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
rustdv = { path = "../../rustdv/rustdv" }   # → the clone's facade crate

[dev-dependencies]
rustdv-vpi-stubs = { path = "../../rustdv/rustdv/rustdv-vpi-stubs" }
```

The dev-dependency satisfies VPI symbols when unit tests run without a
simulator. Copy the repository’s `rust-toolchain.toml` into your external
project if you want the same compiler. On macOS, also use its
`.cargo/config.toml` linker settings; a sibling framework clone’s Cargo
configuration is not inherited by your testbench.

## Build your first testbench

A rustdv testbench is a Rust library that the simulator loads. Your project
needs:

1. **`Cargo.toml`** — two settings matter and both are required:
   `crate-type = ["cdylib", "rlib"]`, plus the `rustdv-vpi-stubs`
   dev-dependency so `cargo test` links. Add `use rustdv_vpi_stubs as _;`
   under `#[cfg(test)]` in the crate root.
2. **`src/lib.rs`** — starts with `use rustdv::prelude::*;`,
   one `rustdv::vpi_bootstrap!();`, and one `#[rustdv::test]` function
   that touches a signal and starts a clock if your DUT needs one. A design
   that already supplies its own clock needs only edge waits. Get this passing
   before adding the rest of the verification code.
3. **`sim/timescale.v`** — sets time units for RTL that does not declare its
   own. Follow the repository launcher's convention of compiling it first so
   delay units match the testbench.
4. **`sim/run.sh`** — builds the library (`.so` on Linux, `.dylib` on macOS),
   copies it as `.vpi`, compiles your RTL, and runs `vvp`. Keep generated
   libraries/designs under `/tmp/rustdv-$(id -u)/`. Set the crate name, top
   module name, and RTL file list. Export the selected module as
   `RUSTDV_TOP` before starting the simulator; when it is unset, rustdv uses
   the first top-level module returned by VPI.
5. **Your RTL**, unmodified, in `hdl/`.

Run `sim/run.sh` and check that the smoke test passes. Then grow the BFM,
driver, monitors, scoreboard, and sequences. The complete worked example is
`rustdv/tinyalu_tb/`; the book's Interlude walks through its files.

## Reading and writing simulator values

Start with `use rustdv::prelude::*;`. It brings `HandleChildren` into scope for
hierarchy/member lookup and `HandleEvent` for logic edges. Import
`rustdv::sim::handle::HandleBase` when you need `name()` or `full_name()`.

### Choosing a handle

| Simulator object | Lookup from a hierarchy or aggregate | Handle |
|---|---|---|
| Logic, bit, integer, or enum variable | `signal(name)` | `LogicHandle` |
| Real variable | `real(name)` | `RealHandle` |
| String variable | `string(name)` | `StringHandle` |
| Unpacked struct or union | `aggregate(name)` | `AggregateHandle` |
| Child module | `hierarchy(name)` | `HierarchyHandle` |

`child(name)` returns a `SimHandle`. Its `as_signal()`, `as_real()`,
`as_string()`, `as_aggregate()`, and `as_hierarchy()` conversions return
`Result`; a wrong kind reports `HandleError::WrongKind`. `children()`
enumerates the objects the simulator exposes.

VPI visibility depends on the simulator and its build settings. Unpacked
aggregates require `vpiStructVar`, `vpiUnionVar`, and `vpiMember` support;
the [framework type tests](https://github.com/rustdv/rustdv/tree/HEAD/rustdv/framework-tests#typed-variable-access) describe that requirement.
Packed aggregates exposed as logic variables use `LogicHandle`. Verilator
uses two-state values; use Icarus for X/Z behavior.

### Logic and integer values

Logic handles provide `get_bool()`, `get_u8()`, `get_u16()`, `get_u32()`,
`get_u64()`, `get_u128()`, and `get_bigint()`. Each returns a `Result` and
rejects X/Z; a fixed-width read also rejects a signal wider than its target
type. `get_bool()` requires a one-bit signal. Integer access is unsigned.

Each type has scheduled and immediate setters, such as `set_u32()` and
`set_u32_now()`. Use `BigUint` for values wider than the primitive integer
types. This example expects a writable 160-bit `data` signal:

```rust
use rustdv::prelude::*;

pub async fn write_wide_data(dut: HierarchyHandle) -> Result<(), TestError> {
    let data = dut.signal("data")?;
    let expected = BigUint::from(1u8) << 159usize;
    data.set_bigint(&expected);
    read_write().await;
    assert_eq!(data.get_bigint()?, expected);
    Ok(())
}
```

Use `get_logic()` and `set_logic()` to preserve all four logic states, and
`get_binstr()` for an MSB-first string. `LogicArray::from_binstr()` and
`to_binstr()` use that same order; indexed access is LSB-first (`value[0]`
is the least significant bit and returns `Option<Logic>`).
`LogicArray::from_vpi_words()` and `to_vpi_words()` preserve the exact width
with VPI's `(aval, bval)` word representation.

Convert a `LogicArray` with `u64::try_from(&value)` or the corresponding
primitive/`BigUint` conversion. These reject X/Z. Fixed-width conversions of
an already-read `LogicArray` keep the low bits; use the handle's typed getter
when you want a width check.

### Real, string, and aggregate values

Real handles read and write `f64`. String reads return owned Rust strings;
string writes copy their input and return an error for embedded NUL bytes.
Read and write aggregate values through their named members, including nested
aggregates. Union members share storage.

This example uses the objects in the framework's `probe.sv`:

```rust
use rustdv::prelude::*;

pub async fn write_record(dut: HierarchyHandle) -> Result<(), TestError> {
    let gain = dut.real("real_sig")?;
    let label = dut.string("string_sig")?;
    let record = dut.aggregate("record_sig")?;
    gain.set(1.25);
    label.set("ready")?;
    record.signal("number")?.set_u32(42);
    read_write().await;
    assert_eq!(gain.get()?, 1.25);
    assert_eq!(label.get()?, "ready");
    assert_eq!(record.signal("number")?.get_u32()?, 42);
    Ok(())
}
```

Scheduled writes apply at ReadWrite, retain the last write to each handle,
and preserve the ordering of distinct handles. A write made during ReadWrite
applies immediately. Immediate setters skip buffering: logic uses names such
as `set_u32_now()` and `set_logic_now()`, while real/string use `set_now()`.
Both paths reject writes from ReadOnly.

### Selecting the DUT root

Export `RUSTDV_TOP` before loading the testbench to select an exact root module
name. Without it, rustdv uses the first root returned by VPI. Select the DUT
explicitly when you also load a waveform-dump module. The shipped run scripts
already export their selected DUT name.

### Updating an existing testbench

| Earlier spelling or behavior | Current API |
|---|---|
| Simulator-layer `AnyHandle` | `SimHandle` (the low-level GPI enum remains `AnyHandle`) |
| `handle.as_logic()` | `handle.as_signal()?` |
| Optional handle conversions | `Result`-returning `as_*()` conversions with kind errors |
| Logic `get()` / `set()` / `set_now()` | `get_logic()?` / `set_logic()` / `set_logic_now()` |
| Infallible `get_binstr()` | `get_binstr()?` |
| `LogicArray::to_u64()` | `u64::try_from(&value)?` |
| MSB-first `LogicArray::bit(i)` | LSB-first `value[i]`; adjust index calculations |
| Inherent hierarchy, edge, and naming methods | Bring `HandleChildren`, `HandleEvent`, and `HandleBase` into scope as needed |

## Simulator features

Use Icarus for four-state behavior and the book’s reference transcripts.
For a VCD, add a dump module and compile it as an additional root with
`-s dump`, alongside the selected DUT root. For a DUT named `my_top`:

```verilog
module dump;
  initial begin $dumpfile("waves.vcd"); $dumpvars(0, my_top); end
endmodule
```

Keep `RUSTDV_TOP=my_top` when starting that simulation so the framework
selects the DUT rather than the dump module.

The repository’s Verilator launcher offers these modes:

| Mode | Use it for |
|---|---|
| FAST | Two-state functional runs with top-level port visibility. |
| DEBUG | Selected internal VPI visibility and FST from time zero. |
| RECORD | Runtime-controlled FST capture windows; no file opens until requested. |
| INSPECT | Selected internal VPI visibility without waveform instrumentation. |
| COVERAGE | RTL line/expression coverage, separate from functional coverage collected in Rust. |

`service_read_only()` runs a synchronous operation after the design settles
while holding simulation time. Runtime capture uses
`rustdv::sim::simulator_trace` at that settled point; blocking service waits
need a wall-clock deadline or cooperative cancellation. Simulation-time
timeouts cannot release a blocked closure.

The [simulator guide](https://github.com/rustdv/rustdv/tree/HEAD/sim#running)
documents mode selection, control files, trace lifecycle, and coverage commands.
These modes require the repository’s scripts and C++ host in addition to the
Rust crates. DEBUG and RECORD need lz4 for FST; waveform and RTL-coverage
instrumentation use separate builds.

## Grow and verify the testbench

Use rust-analyzer for type and ownership diagnostics. Missing runtime
connections are reported during elaboration. Grow the bench in this order:

1. Transactions and a predictor, with ordinary `#[test]` unit tests and
   `cargo test`; no simulator is needed for this logic.
2. The BFM’s reset and one operation, exercised by a directed simulator test.
3. Monitors and a scoreboard, then sequences and random tests.

A stage is complete when its regression passes and its checks detect an
intentionally corrupted DUT. A scoreboard that compares no transactions is not
proof of correct hardware. For a hang, reproduce the printed
`RUSTDV_RANDOM_SEED` and inspect the first failure in the log.

The [verification skills](https://github.com/rustdv/rustdv/tree/HEAD/skills)
provide a plan-to-testbench-to-mutation-check workflow. Use their `SKILL.md`
files as human checklists or give them to an AI assistant. Keep the framework
clone and your project accessible together, and start with a smoke test before
asking the assistant to build a full bench. For example:

> Build a Rust testbench for the SPI core in `my-core/hdl/`, using the
> verification skills and TinyALU testbench as references. First prove that
> clock and reset work, then grow the BFM and scoreboard. Show real run output
> and verify that a deliberately broken DUT fails.

## Further reading

- [The worked TinyALU testbench](https://github.com/rustdv/rustdv/tree/HEAD/rustdv/tinyalu_tb).
- [The repository overview and UVM concept map](https://github.com/rustdv/rustdv#the-uvm-concept-map).
- [The book](https://github.com/rustdv/rustdv/tree/HEAD/book-pdf): Rust foundations,
  followed by the complete testbench and its components.
- [The changelog](https://github.com/rustdv/rustdv/blob/HEAD/CHANGELOG.md) for
  release-specific changes, and [STATUS.md](https://github.com/rustdv/rustdv/blob/HEAD/STATUS.md)
  for implementation and validation history.
