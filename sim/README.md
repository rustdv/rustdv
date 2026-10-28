# Simulator scaffold

The TinyALU DUT, the shared Verilator host, and smoke/regression entry points.
Both free simulators are exercised in CI so the native VPI loading path cannot
rot unnoticed.

## Files

- `hdl/tinyalu.sv` — the TinyALU derived from the pyuvm project
  (`reference/pyuvm-master/examples/TinyALU/hdl/verilog/tinyalu.sv`,
  Apache-2.0, © Ray Salemi). Its self-generated clock is exposed as a
  read-only top-level port so a FAST VPI testbench needs no internal visibility.
- `tb/smoke_tb.sv` — minimal self-checking testbench: five operations,
  prints `SMOKE: PASS` on success.
- `run_smoke.sh` — HDL-only simulator selector.
- `run_rustdv.sh` — builds and runs the Rust TinyALU testbench.
- `run_verilator.sh` / `verilator_main.cpp` — shared Verilator build wrapper
  and correctness-critical event loop.
- `verilator-debug.vlt` — TinyALU's deliberately selected DEBUG internals.

## Running

| Simulator | Command | Notes |
|---|---|---|
| Icarus Verilog | `sim/run_smoke.sh icarus` | free; runs in CI |
| Verilator | `sim/run_smoke.sh verilator` | free; runs the self-checking RTL smoke test in CI |
| Synopsys VCS | `sim/run_smoke.sh vcs` | needs license; run locally |
| Siemens Questa | `sim/run_smoke.sh questa` | needs license; run locally |
| Cadence Xcelium | `sim/run_smoke.sh xcelium` | needs license; run locally |

The same tests run through the regression system
(`output/regression/regress.py --suite custom --filter sim`) and skip
automatically on machines without the simulator installed.

Run the Rust testbench with either backend:

```sh
sim/run_rustdv.sh release icarus
sim/run_rustdv.sh release verilator
RUSTDV_VERILATOR_MODE=debug sim/run_rustdv.sh release verilator
RUSTDV_VERILATOR_MODE=record sim/run_rustdv.sh release verilator
RUSTDV_VERILATOR_MODE=inspect sim/run_rustdv.sh release verilator
RUSTDV_VERILATOR_MODE=coverage \
RUSTDV_COVERAGE_FILE=/tmp/rustdv-coverage/tinyalu.dat \
  sim/run_rustdv.sh release verilator
```

DEBUG writes an FST under `/tmp/rustdv-$(id -u)/`. FAST exposes only top-level
ports; DEBUG adds the signals in a Verilator control file and records an FST
from time zero. RECORD compiles all-signal FST instrumentation but does not
open a trace until `rustdv::sim::simulator_trace` arms a private runtime
capture at settled ReadOnly. INSPECT adds selected VPI visibility without FST
instrumentation. Calling runtime trace control in FAST or INSPECT returns a
structured unsupported-capability error. The small framework probe alone uses
full VPI visibility. Verilator 5.050 or newer is required for runtime VPI
library loading.

Useful Verilator controls are:

| Variable | Values | Purpose |
|---|---|---|
| `RUSTDV_VERILATOR_MODE` | `fast`, `debug`, `record`, `inspect`, `coverage`, `framework` | visibility, tracing, and coverage policy |
| `RUSTDV_VERILATOR_CONTROL_FILE` | path to `.vlt` | selected DEBUG/RECORD/INSPECT internals |
| `RUSTDV_FST` | output path | DEBUG waveform location |
| `RUSTDV_COVERAGE_FILE` | output path | required COVERAGE database destination |
| `RUSTDV_VERILATOR_OPT` | `default`, `o3` | generated-model optimization |
| `RUSTDV_VERILATOR_THREADS` | `1`, `2`, `4` | Verilated model threads |
| `RUSTDV_VERILATOR_NATIVE` | `0`, `1` | opt in to `-march=native` |
| `RUSTDV_VERILATOR_PROGRESS_SECONDS` | positive seconds | optional scheduler progress |

The coverage profile also accepts a control file for coverage-selection rules.
DEBUG and RECORD need the lz4 development headers/library for FST output.
The run scripts export `RUSTDV_TOP` to select the DUT root; export it yourself
when using a custom launcher, especially if you load additional root modules.

For a long local functional run, enable the same runtime-oriented build knobs
used during performance work (native code is intentionally opt-in because the
resulting executable is host-specific):

```sh
RUSTDV_VERILATOR_OPT=o3 \
RUSTDV_VERILATOR_NATIVE=1 \
RUSTDV_VERILATOR_THREADS=2 \
sim/run_rustdv.sh release verilator
```

Verilator is a two-state simulator; use Icarus for X/Z behavior and exact book
transcripts. The shared host is correctness-critical: it advances to the
earliest RTL event or VPI deadline and settles VPI writes before ReadOnly.

COVERAGE is a separate line/expression-instrumented build. It writes the
Verilator coverage database after the RTL `final` block and end-of-simulation
callbacks on every orderly termination path. A failed RustDV regression still
returns failure, but its database is retained for diagnosis. A bench may set
`RUSTDV_VERILATOR_CONTROL_FILE` to a Verilator control file containing
`coverage_on`/`coverage_off` rules. FST tracing is deliberately rejected in
this profile; use separate build directories for FAST, waves, and coverage.
Inspect a database with, for example:

```sh
verilator_coverage --report summary /tmp/rustdv-coverage/tinyalu.dat
verilator_coverage --annotate coverage-html /tmp/rustdv-coverage/tinyalu.dat
```

FAST remains entirely uninstrumented and rejects `RUSTDV_COVERAGE_FILE` so a
misconfigured task cannot silently claim to have collected coverage.

## Runtime waveform capture and service operations

Build with `RUSTDV_VERILATOR_MODE=record` for runtime FST capture. The model
includes waveform instrumentation, but no file is opened until the testbench
starts a capture. Selected VPI visibility and the signals retained for FST
are separate controls. The RECORD command above runs the ordinary TinyALU
tests without creating a waveform unless a test calls the trace API.

Use `rustdv::sim::simulator_trace::{start, flush, stop, status}` inside
`service_read_only()`. The closure runs on the simulator thread after pending
writes and combinational logic settle. For example, call this helper from
an async test to capture a window at a caller-supplied path:

```rust
use rustdv::{Timer, service_read_only};
use rustdv::sim::simulator_trace as trace;
use std::path::Path;

pub async fn capture_window(path: &Path) -> Result<(), trace::TraceError> {
    service_read_only(|| trace::start(path)).await?;
    Timer::ns(100).await;
    service_read_only(trace::flush).await?;
    service_read_only(trace::stop).await?;
    Ok(())
}
```

Create the destination directory first; capture owns and may truncate the
specified file. `status()` reports capability/state, start/end times, and dump
count. `host_present()` and `host_format()` can be called before ReadOnly to
choose a backend. The generic API is simulator-neutral; the shipped host
implements FST on Verilator. Missing hosts and uninstrumented builds report
`TraceError::Unavailable`, and incorrect lifecycle/phase use reports
`TraceError::WrongState`.

`service_read_only()` also supports testbench-defined inspection services.
Simulation time cannot advance until its synchronous closure returns, so any
blocking wait must have a wall-clock deadline or cooperative cancellation.
Simulation-time timeouts cannot release a blocked closure. The application
supplies its own communication channels and service transport.

For inspection without recording, use `RUSTDV_VERILATOR_MODE=inspect` and a
`.vlt` file selecting internal signals with `public_flat_rw`, as in
[`verilator-debug.vlt`](verilator-debug.vlt). Runtime capture is unavailable
in this mode. The build modes require the scripts and C++ host in this
repository; adding the Rust crate alone does not install a simulator launcher.

Commercial-simulator invocations are the standard ones but **untested here**
— public CI cannot hold EDA licenses. If you have a license and the command
needs adjusting for your site, that's expected; the testbench itself is
plain SystemVerilog and should run anywhere.

## Why no EDA Playground?

EDA Playground runs SystemVerilog and Python/cocotb, but has no Rust
toolchain, so rustdv testbenches can't execute there. The HDL side (this
DUT, this smoke test) pastes into EDA Playground fine. For the book's
pure-Rust figures, use the per-figure Rust Playground links in the chapter
READMEs, or open the repo in GitHub Codespaces for the full environment.
