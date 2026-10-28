# Changelog

User-facing changes to the framework crates and the companion repository tools.
Historical entries were reconstructed from published crate source and Git
history. Release dates and comparison links are finalized in the release
commit. Historical dates follow publication of the `rustdv` facade crate.

## 0.3.0 — 2026-10-27

### Added

- Typed logic reads and scheduled/immediate writes for `bool`, `u8`, `u16`,
  `u32`, and `u128`, alongside the existing `u64` API. `BigUint` and
  `get_bigint`/`set_bigint` support arbitrary-width unsigned buses.
- `RealHandle` and `StringHandle` for real/string variables, plus
  `AggregateHandle` for named and iterated unpacked struct/union members,
  including nested aggregates. Access depends on simulator VPI support.
- `RUSTDV_TOP` to select the DUT root explicitly when multiple top-level
  modules are loaded.
- `service_read_only()` for a synchronous service operation after pending
  writes and combinational logic settle, while simulation time is held.
  Blocking services need wall-clock deadlines or cooperative cancellation.
- Runtime waveform control through `rustdv::sim::simulator_trace`, including
  start, flush, stop, status, and host/format probes. Verilator RECORD mode
  compiles FST instrumentation and records only when capture is requested.
- Verilator INSPECT mode for selected internal VPI visibility without FST
  instrumentation.
- Verilator COVERAGE mode for RTL line/expression coverage, with a database
  destination set by `RUSTDV_COVERAGE_FILE` and optional selection rules.
  Coverage is retained after a failing regression and is separate from the
  Rust testbench's transaction-level functional coverage.

The Verilator modes require the repository's launchers and C++ simulator host
in addition to the Rust crates. DEBUG and RECORD require the lz4 development
headers and library for FST output. Verilator remains a two-state backend;
use Icarus for X/Z behavior.

### Changed

- Migrated framework, example, and compile-fail crates to Rust edition 2024.
- Replaced handwritten procedural-macro parsing/generation with `syn` and
  `quote`, and platform-specific registration code with `linkme` distributed
  slices. Test and component attribute spellings are unchanged.
- Moved the framework Cargo workspace manifest to the repository root.
  Book examples retain their separate workspace.
- Expanded the user guides, book API reference, and simulator documentation
  for the signal APIs, scheduling, tracing, inspection, and RTL coverage.
- Added warning-free Clippy checks and expanded macOS CI to exercise both
  Icarus and Verilator. CI uses a pinned Icarus build for next-time-step
  callback behavior.

### Fixed

- Phase futures now complete only after their own simulator callback fires;
  unrelated polls or sibling wakeups no longer complete them early.
- Added regression coverage for phase cancellation, waker replacement,
  scheduled-write ordering, and settled sampling on both simulators.

### Breaking API changes

- Simulator-layer `AnyHandle` is now `SimHandle`; the low-level GPI enum
  remains `rustdv::gpi::AnyHandle`.
- General signal conversion is `as_signal()` rather than `as_logic()`.
  Simulator-layer `as_*()` conversions return `Result` with kind errors
  instead of `Option`.
- Logic `get()`, `set()`, and `set_now()` are now `get_logic()`,
  `set_logic()`, and `set_logic_now()`. `get_logic()` and `get_binstr()`
  return `Result`.
- Replace `LogicArray::to_u64()` with `u64::try_from(&value)`.
  MSB-first `LogicArray::bit(i)` is replaced by LSB-first indexed access,
  `value[i]`. Check index calculations when migrating.
- Hierarchy/member lookup and edge helpers require the `HandleChildren`
  and `HandleEvent` traits, both in the prelude. Name metadata requires an
  explicit `rustdv::sim::handle::HandleBase` import.

See the [crate guide and migration notes](rustdv/README.md#updating-an-existing-testbench) and
[simulator guide](sim/README.md) for usage and build requirements.

[Compare changes on GitHub](https://github.com/rustdv/rustdv/compare/v0.2.0...v0.3.0)

## 0.2.0 — 2026-08-14

### Added

- Verilator two-state simulation through a shared C++ host and runtime VPI
  library loading. FAST mode exposes top-level ports; DEBUG adds selected
  internals and FST output. The framework probe has a full-visibility mode.
- Verilator build controls for optimization, native CPU tuning, model
  threads, and scheduler progress reporting.
- Simulator regression checks for callback lifetime, scheduling, and a
  deliberately corrupted DUT.

### Fixed

- One-shot VPI callbacks release their registration handles and detached
  callback state after firing, preventing growth in long-running simulations.
- Logic integer access uses VPI vector words, preserves word order and X/Z
  checks, and masks unused bits. Cached signal widths avoid repeated VPI
  metadata lookups.
- Verilator DEBUG build flags and macOS lz4 discovery/linking.

[Compare changes on GitHub](https://github.com/rustdv/rustdv/compare/d9294f77b19b72773fcf21a6d70d5253881524dd...v0.2.0)

## 0.1.1 — 2026-08-06

### Changed

- Packaged the getting-started guide as the facade crate's crates.io README.

[Compare changes on GitHub](https://github.com/rustdv/rustdv/compare/02afcc71fa21909802fd9276a5a29aa95f185ff9...d9294f77b19b72773fcf21a6d70d5253881524dd)

## 0.1.0 — 2026-08-06

### Added

- Initial framework release on crates.io, with native Rust testbench
  libraries loaded through Icarus VPI.
- Simulator-driven async tasks, edge/timer and scheduler-region triggers,
  scheduled signal writes, four-state values, queues, events, and locks.
- A UVM-inspired component lifecycle, runtime configuration database,
  factory overrides, objections, TLM ports/FIFOs, and analysis broadcasting.
- Sequences and sequencers, including request/response handshakes.
- Test/component macros and a regression runner with reproducible seeds,
  timeout/expectation controls, logging, and optional xUnit XML output.
- The worked TinyALU testbench and runnable examples accompanying
  *Rust for RTL Verification*.

[Release source on GitHub](https://github.com/rustdv/rustdv/commit/02afcc71fa21909802fd9276a5a29aa95f185ff9)
