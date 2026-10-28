# STATUS — rustdv implementation sprint

**Date:** 2026-07-11 — **COMPLETE: REGRESSION PASSES ON ICARUS**
**Scope:** complete rustdv code base, demonstrated against the TinyALU
(`sim/hdl/tinyalu.sv`) on Icarus Verilog.
**Environment:** Path B (offline toolchain drop into `toolchain-drop/`):
rustc/cargo 1.97.0 aarch64-linux + oss-cad-suite 2026-07-11 (Icarus 14.0
devel), both installed to the VM home directory.

## Build & run

```sh
cd sim && ./run_rustdv.sh          # expect "REGRESSION: PASS"
cargo test --workspace             # pure-Rust unit tests, no simulator
```

- [x] `cargo build -p tinyalu_tb` (debug and release) — clean
- [x] `cargo test --workspace` — 8/8 unit tests pass, no simulator needed
- [x] `sim/run_smoke.sh icarus` → SMOKE: PASS
- [x] `sim/run_rustdv.sh` → **REGRESSION: PASS**
      - `random_ops`: 20 ops driven through the full sequencer handshake,
        scoreboard 20 compared / 0 mismatches, coverage Add=5 And=5 Mul=5
        Xor=5, 625 ns sim time
      - `max_ops`: 4 compared / 0 mismatches, all ops covered, 185 ns
      - xUnit XML written to `sim/build/results.xml`
- [x] **Mutation check (verification of the verification):** with the DUT's
      XOR sabotaged to OR, the scoreboard flags every affected transaction,
      both tests report FAILED with per-transaction diagnostics, and the
      run ends `REGRESSION: FAIL`. The checking is not vacuous.

## What was built

Cargo workspace at `rustdv/`, zero external dependencies:

| Crate | Contents | Design-doc |
|---|---|---|
| `rustdv-gpi-sys` | hand-written VPI FFI subset | §2 D2.1 (deviated, D1 below) |
| `rustdv-gpi` | safe handles/values/callbacks/time; all `unsafe` lives here; RAII `CallbackHandle` | §3.3 |
| `rustdv-sim` | bespoke single-thread executor, 7-state tasks, drop-based cancel, Timer/edges/ReadOnly/ReadWrite/NextTimeStep, buffered writes drained at ReadWrite, Event, FIFO-fair Lock, Queue, first!/join!/with_timeout, Clock, SplitMix64 Rng, sim-time logger | §4 |
| `rustdv-uvm` | Component lifecycle trait + ComponentNode traversal, RAII objections, channel/Sender/Receiver, AnalysisPort/Subscriber/AnalysisFifo, TlmFifo, full sequencer handshake (SeqItem envelope, SeqCtx, SeqItemPort, ResponseQueue) | §5 (R1–R6 revision) |
| `rustdv-macros` | `#[rustdv::test]` (link-section registration), `#[derive(Component)]` (`T`/`Option<T>`/`Vec<T>` children) | §6 |
| `rustdv-runner` | link-time test registry + sentinel, sequential regression, timeouts, expect_fail/skip, seed handling, summary table, xUnit XML, VPI bootstrap | D2.4, §4.5 |
| `rustdv` (facade) | prelude + `vpi_bootstrap!()` | D2.5 |
| `tinyalu_tb` | §7 worked example: transactions (std derives), TinyAluBfm, Driver, CmdMonitor, ResultMonitor, Scoreboard (predict + compare in check), Coverage (Subscriber), AluEnv (ownership tree, Option children), RandomSeq/MaxSeq (late generation at grant), tests `random_ops` + `max_ops` | §7 |

Simulation flow: `sim/run_rustdv.sh` builds the testbench cdylib, copies it
as a `.vpi` module, compiles the DUT with `sim/hdl/timescale.v` (1ns/1ns),
and runs `vvp -M build -m tinyalu_tb`.

## Deviations from the design doc

**D1 — VPI backend instead of cocotb's GPI library (§3.1 D3.1, §3.2 D3.2).**
The sandbox cannot build cocotb's C++ GPI (no cocotb build tooling) and the
zero-dependency constraint rules out bindgen. `rustdv-gpi-sys` therefore
binds the IEEE 1800 VPI C API directly (hand-written subset of
`vpi_user.h`) — the same API cocotb's GPI wraps for Icarus. The safe layer
(`rustdv-gpi`) keeps the GPI-shaped surface from §3.3 (opaque non-null
handles, `Result` acquisition, copied strings, no unwinding across FFI,
one-shot-vs-recurring callback ownership in the type), so swapping in real
`gpi.h` bindings later is contained to the `-sys` crate. Bootstrap is a
plain VPI module (`vlog_startup_routines` via `rustdv::vpi_bootstrap!()`)
instead of the libpygpi entry-symbol scheme. Consequence: v0 runs on
VPI simulators (Icarus); VHPI/FLI arrive with the real GPI reuse (OQ-1/OQ-2
stand).

**D2 — Trigger lifecycle simplified (§4.3/§4.4).** Instead of shared
trigger objects with subscribe/unsubscribe lists and lazy prime/unprime,
each awaited trigger future registers its own VPI callback on first poll
and removes it via RAII on drop. User-facing API is per design
(`sig.rising_edge().await`, `Timer::ns(2).await`); the shared-subscriber
optimization (one VPI callback per signal) is future work. Phase triggers
(ReadOnly/ReadWrite/NextTimeStep) *are* hub-managed singletons per design,
including the write-buffer drain ordering and illegal-transition checks.

**D3 — Proc macros without syn/quote (§6).** Zero-dependency constraint
(Path B: no crates.io). `#[rustdv::test]` and `#[derive(Component)]` are
hand-written token parsers with narrow supported grammar (plain async fns;
non-generic structs with named fields). Link-time registration uses the
ELF `__start_/__stop_` section technique directly (what linkme does),
Linux-only for now — OQ-4's platform caveats apply; the explicit-
registration fallback is the sentinel pattern in `rustdv-runner`.

**D4 — Logging is a minimal built-in (`OQ-8`).** The `tracing` crate
mapping is deferred (no external deps). A small sim-time-stamped logger
reproduces the book's `  2.00ns INFO ...` format with level filtering.

**D5 — `#[rustdv::parametrize]` not implemented (§6.2, OQ-10).** Neither
TinyALU test needs it; the data-driven-loop idiom covers the demo. Planned
follow-up.

**D6 — TinyALU env has no agent layer (§5.4 example).** The testbench
follows the book's 6.0 architecture (§7.2: driver/monitors/scoreboard/
coverage directly in the env). `Active`/`Option<Driver>` and
`enable_coverage`/`Option<Coverage>` still demonstrate the
conditional-children pattern the agent chapter needs.

**D7 — Timeout diagnostics.** On test timeout the runner reports the
timeout and kills surviving tasks, but does not yet print the objection
table (the per-test `RunCtx` lives inside the test body, invisible to the
runner). pyuvm's richer report is future work.

**D8 — `bridge`/blocking-world integration (OQ-7), `wait_modified`,
transport/master/slave composites, grab/lock/priority arbitration,
pack/unpack/recording/policies** — not ported, matching the design doc's
own [gap] list.

**D9 — `rustdv-vpi-stubs` dev-dependency crate (build-flow addition).**
Unit-test *executables* link the whole crate graph, and unlike the cdylib
they cannot carry undefined `vpi_*` symbols. Crates with unit tests take
`rustdv-vpi-stubs` as a dev-dependency (`#[cfg(test)] use ... as _;`): it
defines panicking stubs so test binaries link. It is never linked into the
`.vpi` module, where the real symbols come from the simulator process.
(A first attempt via linker flags — `-z lazy` +
`--unresolved-symbols=ignore-all` — corrupted aarch64 PLT relocations and
was abandoned; the root `.cargo/config.toml` intentionally has no Linux
linker override.)

## Fix log (build/sim loop)

1. `Join2::poll` needed `unsafe get_unchecked_mut` (output types may be
   `!Unpin`; the inner futures are boxed, so this is sound).
2. Test-binary linking → D9 above.
3. Icarus rejects `vpiSuppressVal` on value-change callbacks ("value
   format 10 not supported"); callback registration now passes a NULL
   value pointer (closures read signals themselves).
4. `Executor::cancel_after` off-by-one: watermark comparison must be `>=`
   so a timed-out test's own task is killed (caught in self-review).

That was the entire loop — four fixes from first compile to passing
regression.

## Open items

- Shared-subscriber trigger optimization (D2) and the runner-side
  objection dump on timeout (D7) are the two nearest-term improvements.
- Portability beyond Linux/ELF for the link-section test registry (OQ-4)
  and beyond VPI/Icarus for the backend (D1/OQ-1/OQ-2) are the two
  structural follow-ups before the book can claim multi-simulator support.

## Re-verification — 2026-07-13 (fresh VM)

Independent end-to-end rerun in a new session/VM, from the Path B
toolchain drop. No source changes were needed; results reproduce the
2026-07-11 run exactly.

- Toolchain reinstalled from `toolchain-drop/`: rustc/cargo 1.97.0
  aarch64-linux, Icarus 14.0 (devel, s20260301). Install note: extract
  the tarballs from a VM-local copy (`cp` to `/tmp` first) — extracting
  directly from the mount is ~15× slower and exceeds shell timeouts.
- [x] `sim/run_smoke.sh icarus` → SMOKE: PASS
- [x] `cargo build --workspace` — clean; `cargo test --workspace` — 8/8
- [x] `sim/run_rustdv.sh` → **REGRESSION: PASS** (`random_ops` 20/0
  mismatches, coverage Add=5 And=5 Mul=5 Xor=5, 625 ns; `max_ops` 4/0,
  all ops, 185 ns); fresh `sim/build/results.xml` committed
- [x] Mutation check repeated (XOR→OR at `tinyalu.sv:48`): scoreboard
  flags all 5 + 1 affected transactions, both tests FAIL, run ends
  `REGRESSION: FAIL`; DUT restored and clean run re-confirmed PASS

## Book completion — 2026-07-13

The manuscript (`book-pdf/src/`) is now **complete**: chapters 15–41 plus
Appendices A (chapter map) and B (idiom translations) written, SUMMARY.md
updated. Every figure in the new chapters runs: 20 new sim-chapter example
crates under `output/examples/` (each a cdylib whose `#[rustdv::test]` fns
are the chapter's figures, ending `REGRESSION: PASS` on Icarus), plus
pure-Rust bins and compile-fail figures in the Part I style. Shared
testbench code (the Rust `tinyalu_utils`, testbench versions 2.0–8.0)
lives in `output/examples/tinyalu-utils/`.

Regression: `output/regression/regress.py` — book-sync now scoped to
Part I (Part II+ figures live inside sim crates), `custom/sim-ch15..39`
tests run every sim chapter, examples suite unchanged. Full run on this
VM: 107 book-sync + 95 examples + 20 sim chapters + smoke, all green.

Library changes made for the book (all regression-verified):
- **rustdv-sim**: `Debug` for `LogicHandle`/`HierarchyHandle`;
  `#[must_use]` on Timer/Edge/NullTrigger (ch17's "forgot the await"
  warning); `log::critical`; hierarchical per-target log levels
  (`Logger`, `set_level_for`) and a file handler (`log_to_file`) — ch26.
- **rustdv-gpi(-sys)**: SV variable vpiTypes (610–620) classify as logic
  signals (counter.sv's `byte unsigned` port).
- **rustdv-uvm**: `print_hierarchy` exported in facade/prelude; `Debug`
  on `TlmFull`.
- **rustdv-macros**: `#[derive(Component)]` now supports generic structs
  and fields with generic types containing commas.
- **tinyalu_tb / examples BFMs**: `wait_idle` hardened to require two
  consecutive idle edges (fixed a race that could end a test with the
  last command undriven; interlude transcript timings updated 625→635,
  810→830 ns accordingly).

Not done in the VM: regenerating `book-pdf/book/` (HTML/PDF) — mdBook
isn't available offline here. `mdbook build book-pdf` on the host (or the
docker flow) will rebuild the rendered book from the updated sources.

### macOS portability fixes — 2026-07-13

Ray's pre-push regression on macOS exposed two Linux-isms in the new
example code; both fixed, Linux sweep re-verified green:

- **VPI cdylibs wouldn't link on Mach-O** (undefined `vpi_*` symbols are
  a load-time feature on Linux, an error on macOS). Added
  `-undefined dynamic_lookup` for the two Apple targets in
  `output/examples/.cargo/config.toml` and `.cargo/config.toml`
  (target-scoped: Linux builds untouched, D9's caveat still respected).
  `sim-common/run_sim.sh` and `sim/run_rustdv.sh` now fall back from
  `lib*.so` to `lib*.dylib`.
- **ch21 fig04 (ELF `__start_/__stop_` section demo)** is inherently
  Linux-only; the registration/collect machinery is now
  `#[cfg(target_os = "linux")]` with a non-Linux `main` that says so.
  Behavior on Linux unchanged.
- **The test registry itself (OQ-4) now has Mach-O spellings.** rustc
  rejects free-form `link_section` names on Apple targets, so the
  `#[rustdv::test]` macro and the runner's sentinel emit
  `__DATA,rustdv_tests` under `cfg(target_vendor = "apple")`, and
  `collect_tests` reaches the section bounds through
  `section$start$/section$end$` link_names (the linkme technique).
  ELF path is byte-for-byte what it was; Linux workspace tests +
  regression re-verified green. OQ-4's remaining platform is Windows.

With these fixes the **full pre-push suite passes on macOS/arm64**
(Ray's machine, 2026-07-13) as well as Linux — rustdv and all 20 book
sim chapters now run on two platforms.

---

## 2026-07-21 — the UVM restoration, step 4: `RustdvCtx` and the test macro

Decisions and reasoning are in `output/.design-decisions.md` (D46–D49, which
strike D8); this is the implementation record.

**A test is now a component.** `Component` gained
`async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError>`,
and `#[rustdv::test]` accepts a struct or type alias as well as a free
`async fn`. Chapter 23's aspirational example — written before the
framework could compile it — now compiles and runs unmodified except for
one `&`.

- **`TestCtx` (runner) + `RunCtx` (uvm) → `RustdvCtx` (uvm).** One
  universal context: `dut()`, `seed()`, `rng()`, `path()`, the objection
  registry, and path-aware `info()`/`warning()`/… . It is `Clone` with the
  objection registry `Rc`-shared.
- **`TestError` moved from `rustdv-runner` to `rustdv-uvm`**
  (`rustdv-uvm/src/error.rs`), since `Component::run` returns it and the
  runner sits above the UVM crate. Re-exported from the runner and the
  facade, so `::rustdv::TestError` is unchanged.
- **Deviation — `async fn` in a trait is not dyn-compatible.** Adding `run`
  to `Component` broke `&mut dyn ComponentNode`, whose supertrait it was.
  The synchronous phases are therefore mirrored onto a new dyn-safe
  `DynPhases` trait (blanket impl over every `Component`, distinct method
  names so `component.extract()` stays unambiguous), and `ComponentNode`
  requires that instead. Users write `Component` exactly as before.
- **`Component::start` survives, transitionally.** ch24+ and `tinyalu_tb`
  still use the old spawn hook; ch24 folds it into `run`.
- **The runner** builds `RustdvCtx` with the test's registered name as the
  root path (so logs read `[HelloWorldTest]`, not UVM's `uvm_test_top`),
  runs the test, then awaits objection consensus — guarded on
  `ObjectionRegistry::ever_raised()`, so a cocotb-shaped test that never
  objects does not trigger pyuvm's "you never objected" warning.
- **`#[derive(Component)]` learned unit structs** (`struct HelloWorldTest;`).
- **`tinyalu_utils` is infrastructure only** (D45): `tb2`, `tb4`, `tb6`,
  `tb7`, `env7`, `bfm7`, `alu_item` are no longer compiled. ~~The files stay
  in `src/` so each chapter can lift its copy into the chapter file as it
  converts; ch23 has done so.~~ **Struck 2026-08-05: every chapter converted
  and the seven files were deleted** — see the D116/D117 entry at the end.
- **Mechanical rename** of `TestCtx`/`RunCtx` → `RustdvCtx` across 49
  files: examples, `tinyalu_tb`, `getting-started-with-rustdv.md`, and the
  book chapters. No `file:line` moved, so no transcript regeneration (D31).

**Verified.**

- `output/examples/sim-common/run_sim.sh ch23_uvm_test_testbench_3_0 tinyalu …`
  → 3/3 tests, `REGRESSION: PASS` (seed 1). Transcript in the chapter README.
- `regress.py --suite book-sync` → 108/108; `--suite examples` → 96/96;
  `--suite custom` → 6 passed, 0 failed (sim-ch15/16/17 and the new
  sim-ch23 among them; ch18+ still quarantined).
- `cargo build` and `cargo test` clean in the `rustdv` workspace,
  `cargo build -p tinyalu_tb` clean.
- `tinyalu_utils` and `ch23_uvm_test_testbench_3_0` removed from the
  `regress.json` quarantine block.

**Open, recorded rather than guessed:** struct tests register under their
*type* name (`RandomTest`), not the snake_case `random_test` D27 wrote —
see Q15 in the decisions log. Deciding it before ch24 is free; after ch24
it is a transcript regeneration.

## 2026-07-28 — TLM lands: ports, exports, analysis, and ch31/32/34 un-quarantined

The TLM layer is built and three chapters came out of quarantine. Full
regression: **223 passed, 0 failed** (book-sync + examples + custom, with
ch36–ch39 still quarantined); `cargo test --workspace` clean.

**Connection is a trait method, not a registry (D83b).** The path-keyed port
registry designed on 2026-07-24 was built, and then struck the same week: it
could address a child but not the connecting component itself, because a
component does not know its own path (D7). Ray's correction — *"I guess you now
see why `uvm_test` is a `uvm_component`"* — is that the UVM's uniformity is
load-bearing. `ComponentNode::port_slot(name) -> Option<Rc<dyn Any>>`, generated
by the derive, is reachable through `dyn` and so answers for an erased child and
a concrete `self` alike. The registry, the thread-local store, the per-test
clear and the walk-time path stamping all went away with it; `RustdvPath`
(D83a) stays, since the logger uses it.

**What is new in the framework**

- `rustdv-methodology/src/port.rs` — `PutIf`/`GetIf`/`PeekIf`/`PublishIf`,
  `Port<I>` with the `PutPort`/`GetPort`/`PeekPort`/`PublishPort`/
  `SubscribePort` aliases, `PortName<I>` (carries the *interface*, so a `get`
  export aimed at a `put` port is a compile error), `PortOwner`, `PortField`.
- `rustdv-methodology/src/fifo.rs` — `TlmFifo` rebuilt on a shared inner:
  `put_export()`, `get_export()`, `peek_export()`, and the D23 analysis taps
  `put_ap()`/`get_ap()`. `TlmFifo::new(usize)` replaced `new(Option<usize>)`;
  `unbounded()` is the other constructor.
- `rustdv-methodology/src/analysis.rs` — `AnalysisFifo` is now the broadcast
  hub (renamed `AnalysisBus` on 2026-07-29, D103 — read it as that
  below): `pub_export()` and `sub_export()`. (This bullet first read
  "…, `get_export()`", written before D90 struck the hub's queue later the same
  day; the deviations list below is the correct record. Corrected 2026-07-28.)
  The pre-hub `AnalysisPort`/`Subscriber` stay for the Part IV testbenches not
  yet rebuilt.
- `rustdv-methodology/src/shared.rs` — `RustdvShared<T>`, the state a
  subscriber shares with its port so `write` can be synchronous (D87/D88).
- `rustdv-sim/src/queue.rs` — `peek`/`try_peek` (need `T: Clone`),
  `has_space`, and `wait_for_space` (the FIFO taps need the wait and the
  handover separated, since `try_put` takes ownership).
- `#[derive(Component)]` — `#[port(put|get|peek|publish|subscribe)]` fields
  generate the port lookup, the elaboration report, a typed `PORT_NAME`
  constant, and a `PortOwner` impl. `#[component(fifo)]` is recognised as a
  child.
- Elaboration sweeps the tree and reports **every** unconnected required port
  at once, classified `tlm_unconnected_port` so a test can assert on it.

**A silent bug this work exposed (D82c).** `run_component_test` raced the whole
`run_all` future against the objection-drained event. Losing a race means being
dropped — so at consensus the entire tree's future was dropped, taking with it
the children D82b moves *out* of their slots, and extract/check/report then
walked a tree whose components had been destroyed. ch34 "passed" with its
scoreboard never running. Each component now races the drained event
individually (`run_one`), so every level returns normally and restores its
children.

**Chapters un-quarantined** (all `REGRESSION: PASS` on Icarus, seed 1;
transcripts in each README):

- **ch31** — 5 tests, including the y = 2x² pipeline (a parent's `run`
  concurrent with its children's) and the FIFO taps.
- **ch32** — 3 tests. The broadcast transcript is at `0.00ns`, which is the
  point; the slow-subscriber test then shows writes at `0.00ns` and checks at
  5/10/15ns, which is the other point.
- **ch34 / TB 6.0** — the TinyALU testbench wired with `TlmFifo` and two
  `AnalysisFifo` buses; scoreboard checks all four ops, coverage sees 4 of 4.

**Deviations recorded rather than hidden.**

- `PutPort::try_put` returns `Result<(), T>` and `try_get` returns `Option<T>`,
  not the UVM's bit — `put` takes ownership, so a refusal that kept the item
  would lose it (D89). Found because ch31 Figure 4 demonstrated it over a `u32`,
  where the tempting `while port.try_put(n).is_err()` compiles *because `u32` is
  `Copy`* and breaks on the reader's first real transaction. Figures 4–6 now
  carry a non-`Copy` `Packet` and thread the item back through `Err`. **Rule for
  every future figure: demonstrate an ownership property with a non-`Copy`
  type**, or the compiler stops being the check the book claims it is.
- ch34's Tester holds its objection for twenty clocks after its last `put`,
  where the Python testbench waits ten — the multiply is last and takes longest
  to come back. Without the wait the scoreboard silently checks fewer results
  than it saw commands.
- `AnalysisFifo` holds **nothing** — no queue, no `get_export()` (D90, Ray).
  It broadcasts and returns; a datum nobody subscribed to is lost. An earlier
  build gave it a queue that switched on when `get_export()` was called, which
  was invisible state and left `try_get()` returning `None` forever. Storage now
  belongs to the subscriber: `AnalysisPort::connect_fifo()` hands back an
  unbounded `TlmFifo`. `tinyalu_tb`'s scoreboard fields changed type
  accordingly. ch32's Figure 6 was cut: the UVM's analysis FIFO in a scoreboard
  is a workaround for one `write` per class (`uvm_analysis_imp_decl`), and
  rustdv's two ports and two `WriteSink`s mean there is nothing to work around
  (Ray). **Book obligation recorded:** the subscriber-owns-the-storage pattern
  is new and must be taught — see D90 and the Fable brief. ch32 Figures 6–7 were
  then written to give that teaching runnable code, motivated the honest way: a
  subscriber whose work takes simulation time, since `write` cannot await. ch32
  is seven figures, three tests.

## 2026-07-29 — the test suite: three tiers under the chapter runs

`output/test-plan.md` is the plan and the reasoning; §8 of it records where
the built suite differs from the plan as written. What landed:

- **110 no-simulator tests** (was 12), in `#[cfg(test)]` modules beside the
  code they test, run first by `regress.py --suite unit` in about two seconds.
  The methodology layer — ConfigDb, factory, ports, FIFO, analysis bus,
  objections, the phase walk, the whole sequencer handshake — had no unit test
  at all before this; a bug there surfaced as "some chapter went red" and the
  bisect was manual. `rustdv_sim::testing::block_on` is what makes it possible
  and is also the enforcement: a future still pending when the run queue
  empties panics with an explanation, so a test that quietly needed a
  simulator fails loudly instead of hanging.
- **38 targeted simulator tests** in `rustdv/framework-tests/`, against a new
  `hdl/probe.sv`. One cdylib, six regression entries, selected by a new
  `RUSTDV_TESTCASE` filter in the runner (cocotb's `TESTCASE`, widened to
  case-insensitive substrings; a filter matching nothing is an error, not a
  green run of zero tests). `test.json` grew an `"env"` key to drive it.
- **`sim-mutation`**: the TinyALU's XOR is corrupted to an OR, two testbenches
  are required to fail, and then required to pass again on the real RTL. The
  second half is the part that matters — without it a testbench that had
  stopped compiling would "catch" every mutation.
- **5 compile-fail cases** for the methodology layer, each asserting its
  `error[E….]` code. One of them was vacuous when first written: `let _ =
  cmd.a` after `finish_item(cmd)` **compiles**, because `let _` does not
  evaluate a place expression. The use-after-move only appears if the value is
  really read. That is the argument for asserting the code rather than the
  failure.

Regression: 236 entries, green — 228 before, plus the six targeted groups,
`sim-mutation` and `compile-fail-methodology`.

### Two runner behaviours the tests had to work around

Recorded rather than fixed — both want a decision (test-plan §8).

- **The simulator phase survives a test.** A test ending inside ReadOnly
  leaves the next one starting there, where a write panics on the cocotb rule,
  so a test can pass or fail depending on what ran before it.
  `framework_tests::fresh_phase()` steps out of it. The per-test reset in
  `run_one` — which already clears the ConfigDb and the logging config — is
  where a real fix would go.
- **A clock's write can land inside a ReadOnly callback.** `read_only().await`
  runs the executor from within the ReadOnly callback; a `Clock` task due in
  the same step then writes, and Icarus prints "attempted to put a value to
  variable 'clk' during a read-only synch callback". A diagnostic, not a
  failure — but the pattern that provokes it is await-an-edge-then-`read_only`,
  which is the monitor pattern the book teaches.

## 2026-07-29 — the TinyALU refactor: the last technical debt is gone

`rustdv/tinyalu_tb` was the one thing left running on the pre-restoration
shape — `AluEnv::new(config)` with the BFM and every TLM endpoint injected
through constructors, `#[component(no_factory)]` on all five components, the
legacy `AnalysisPort`, work started from `start` as spawned tasks, and a
hand-rolled `start_all` + `run_extract_check_report` in place of the phaser. It
is now the same testbench the book's chapters teach (D109):

- **Phases.** `AluEnv::build` creates the children and `AluEnv::connect` wires
  them; the tests are `#[rustdv::test]` **structs**, so the runner's phaser
  drives them and there is no hand-rolled phasing anywhere in the crate.
- **The BFM comes from the ConfigDb** (D101), filed once by `BaseTest::build`.
  `TinyAluBfm` gained a hand-written `Debug` for `ConfigDb::dump`.
- **Every component is factory-built** with no constructor arguments —
  `Driver::create_comp()` and friends — so all five `no_factory` opt-outs came
  off. The children are `RustdvComp` slots.
- **TLM through `port_slot`** (D83b): one `AnalysisBus` per stream, a
  `#[component(sequencer)]` sequencer, and connect lines with the same shape as
  ch34's.
- **The subscriber owns the storage** (D90). The scoreboard holds two
  `RustdvShared` logs behind two `SubscribePort`s and two `WriteSink` impls;
  coverage is a third subscriber on the command bus. No `AnalysisPort`, no
  `connect_fifo`.
- **Work happens in `run`**, concurrently (D82), instead of in tasks spawned
  from `start`. `spawn` is left where it belongs: the BFM's own collector loops,
  started in the env's `start_of_simulation`.
- **Stimulus is a program, swapped by the factory.** Two tests, `RandomTest` and
  `MaxTest`, each `set_seq_override::<BaseSeq, _>()` and then share one
  `BaseTest` body that runs `create_seq::<BaseSeq>()`. The old `random_ops` /
  `max_ops` free functions are gone; a struct test registers under its type name
  (D102), which is why the transcript now says `RandomTest`.

**Behaviour is unchanged, and that was the check.** Same counts, same simulated
times, and the log text of every line is the same: RandomTest 20 compared / 0
mismatches with `Add=5 And=5 Mul=5 Xor=5`, at 635.00ns; MaxTest 4 / 0 with
`Add=1 And=1 Mul=1 Xor=1`, at 195.00ns. The only difference in the transcript is
that each line now carries the component's path — `[RandomTest.inner.env.scoreboard]`
— because the framework supplies it instead of the component hand-typing a label
into the message (D49/D62).

**Two things found while doing it.**

- **The scoreboard still has teeth.** Corrupting the XOR to an OR in
  `sim/hdl/tinyalu.sv` makes it report five mismatches and fail the test. Worth
  recording because the storage moved (hub → subscriber) and a scoreboard that
  quietly stopped receiving would have looked identical to one that passed.
- **`sim/hdl/tinyalu.sv` is the bare DUT and takes `clk` in**, unlike the book's
  copy under `output/examples/sim-common/hdl/`, which self-clocks. So this
  testbench drives the clock and the chapters do not (D42). It is in
  `BaseTest::start_of_simulation` with the reason written next to it.
  **Retired 2026-07-30 — see D112 in the decision log.** The exception closes:
  `tinyalu_tb`'s DUT becomes self-clocking and the `Clock::new` line comes out.
  This is also what narrows D108's bug 2 down to ch17's taught `Clock` idiom
  and `framework-tests/hdl/probe.sv`. Transcript timings moved 5ns earlier
  (RandomTest 635→630ns, MaxTest 195→190ns) because the software `Clock`
  started high and the self-clocking RTL starts low; counts and coverage are
  unchanged and `custom/sim-tinyalu-tb` doesn't assert timing, so nothing else
  needed updating.

**The shipped testbench is now in the regression** as `custom/sim-tinyalu-tb`.
Until today nothing in the suite ran it: `cargo test --workspace` proved it
compiled, and "the TinyALU regression passes" rested on a human running
`sim/run_rustdv.sh`. The entry asserts `REGRESSION: PASS` plus the three counts
above, so a scoreboard that stopped comparing fails instead of passing quietly.
Regression: **237 entries** (unit 1, book-sync 108, examples 96, custom 32),
green, `MUTATION: CAUGHT`.

## 2026-07-30 — D108: both runner bugs, and a test that was passing because of one of them

Both landed together — they turned out to be one mechanism, not two. Full
reasoning and the two rejected approaches are in `output/.design-decisions.md`
§36 (D108); this is the short version.

`rustdv_sim::phase::leave_read_only()` (new) is the first thing `run_one`
does: a no-op unless the simulator is currently in the ReadOnly region,
otherwise one precision step's wait to leave it (fix 1, the phase leak).
`deny_write_in_read_only()` (new, factored out of `schedule()`) is now called
by `set_u64_now`/`set_now` (`rustdv-sim/src/handle.rs`) the same way it was
already called for the scheduled-write path — an immediate write caught in
ReadOnly now panics instead of silently reaching Icarus, which used to print
a diagnostic and drop the write (fix 2). `framework_tests::fresh_phase()` and
its four manual call sites are gone; the runner does it for every test now.

**A real finding along the way:** `framework-tests/src/clocks.rs`'s
`clock_two_are_independent` (a 2ns and a 10ns clock run together) was passing
*because* of the bug — Icarus silently dropping both clocks' opening writes
delayed the test's sync point just enough to dodge a tie between two
harmonic clocks' coincident edges. Fixed the write-drop and the test's own
latent tie-break showed up (9 edges instead of 10, not a flake — reproduced
every run). The test now bounds its measurement window with the slow clock's
falling edges instead of rising ones, which is 1ns clear of every fast edge
on both sides; same assertions (`dt == 20.0`, `fast_edges == 10`), argued for
instead of coincidental.

**Also fixed, found while verifying the above, unrelated to D108:**
`custom/sim-smoke-icarus` was red on arrival. `sim/tb/smoke_tb.sv` still drove
its own `clk` into a DUT that no longer takes one as a port (D112 made
`sim/hdl/tinyalu.sv` self-clocking — the smoke test has its own tiny DUT and
had the same pre-D112 shape), and `sim/run_smoke.sh` never compiled
`timescale.v` first, so the smoke test's own clock ran at Icarus's 1s/1s
default instead of 1ns/1ns and its watchdog fired. `smoke_tb.sv` now takes
`wire clk = dut.clk`; `run_smoke.sh` compiles `timescale.v` first for every
simulator entry.

**Caveat on that verification: it was Linux only.** See the 2026-07-30 entry
below on D113 — the macOS run was failing for an unrelated reason at the time
these claims were made, and the claims were stated more broadly than the
evidence supported.

Verified: every `framework-tests` group (`clock`, `trig`, `sig_`, `conc`,
`elab`, `runner_`) and the unfiltered all-39 run, all green, zero `VPI
error`/`SCHEDULER ERROR` lines. `regress.py --suite unit` (110 tests),
`--filter sim-tinyalu-tb`, and `--filter sim-smoke-icarus` all green.
`output/examples` has no `set_u64_now`/`set_now`/`read_only()` calls, so
`leave_read_only()` is a no-op for every existing book chapter and none of
their transcripts needed regenerating — D108's own scope note expected a wider
blast radius than this turned out to need, because D112 (landed the same day)
had already narrowed `Clock`'s real users down to `framework-tests` and one
book chapter (ch17), neither of which this fix touches behaviorally.

## 2026-07-30 — D113: `sim/build/` was poisoning the host's simulator

`sim/run_rustdv.sh` failed on Ray's Mac with `Killed: 9` and **no output at
all**, on the branch, while passing in the Linux sandbox against the same
commit and passing on `master`. It read as a regression in D108/D112. It was
not.

`sim/run_rustdv.sh` and `sim/run_smoke.sh` were the last two scripts writing
their build products into the repo (`mkdir -p build`, `cp "$LIB"
build/tinyalu_tb.vpi`). `sim/build/` is gitignored, so it survives every branch
switch, and the folder is shared with a Linux sandbox — so the file macOS `vvp`
was told to `dlopen` was an **ELF shared object**, byte-identical (same
BuildID) to the sandbox's `libtinyalu_tb.so`. macOS will not load a foreign
image and on Apple Silicon says so with SIGKILL before anything prints, which
is why there was no output to diagnose from. The branch/`master` split was the
sync race resolving differently either side of a rebuild, not a property of the
code.

Both scripts now build under `/tmp/rustdv-$(id -u)/` with `SIM_BUILD_DIR`
overriding — the convention `rustdv/framework-tests/run.sh` and
`output/examples/sim-common/run_sim.sh` already followed, which is exactly why
the 21 chapter entries and the six targeted groups never showed this. Verified
after the change: `sim/run_rustdv.sh release` → `REGRESSION: PASS`,
`sim/run_smoke.sh icarus` → `SMOKE: PASS`, `regress.py --suite custom` → 32
passed / 0 failed, and `sim/build/` untouched by any of it.

**Stale artifacts must be deleted by hand once**, on any machine that ran the
old scripts: `rm -f sim/build/tinyalu_tb.vpi sim/build/tinyalu_rustdv.vvp
sim/build/smoke.vvp` (individual files, not the directory — deleting a
directory inside the synced folder forks a `dir 2/` duplicate).

**The process lesson, recorded because it cost an afternoon.** The Linux
verification of D108/D112 was real but was reported as if it covered the
project's supported platforms; it did not, and the first macOS failure was
argued away as environmental on evidence that could not support that. A green
sandbox run is evidence about the sandbox. macOS/arm64 is a shipping platform
for this project (STATUS, 2026-07-13) and nothing is verified there until it is
run there.

## 2026-07-30 — the renumbering pass

`book-pdf/renumbering-spec.md` applied to the `.rs` captions in the eight
crates it lists: ch27, ch28, ch31, ch32, ch34, ch36, ch37, ch39. Every edit is
a digit change inside an existing comment line, so each file's insertions equal
its deletions — required, because transcripts embed `file:line` and a caption
edit that moved a line would invalidate every transcript in that crate. All
eight crates rebuilt clean afterwards (Linux sandbox).

Two things the spec did not list, both handled: ch34's module doc comment said
"Chapter 34 owns Figures 1–2" and used `// Chapter 34, Figure 1:` as a syntax
example, both stale once the captions moved to 2–3; and ch37's "Figure 4 is a
paragraph, not code" comment was reworded to drop the number, since the book
renders that passage as prose rather than a numbered figure.

**A consequence the spec did not anticipate.** The captions were renumbered into
the *book's* numbering, but the chapter READMEs mapped the *crate's*. Every
chapter the pass touched therefore had a README whose figure numbers no longer
matched its own source — including ch28, ch31, ch32 and ch34, which the spec
lists as needing no work. All eight were rebuilt in the book's numbering, so a
README row number is now the figure number.

## 2026-07-30 — sims rerun, transcripts and READMEs regenerated

Ran on Linux (sandbox), `RUSTDV_RANDOM_SEED=1`, everything built under
`/tmp/rustdv-$(id -u)/` per D113 — `sim/build/` was untouched and stayed empty.
All green: ch27 (4 tests), ch28 (7), ch36 (3), ch37 (1), ch38 (1), ch39 (3),
and `sim/run_rustdv.sh` (RandomTest + MaxTest, 4 compared / 0 mismatches on
MaxTest, `REGRESSION: PASS`).

Rebuilt the READMEs for ch27, ch28, ch31, ch32, ch34, ch36, ch37, ch38, ch39 —
figure maps in book numbering, transcripts pasted verbatim under a labelled
heading so the prose pass can copy them into the manuscript's 13
`[TRANSCRIPT NEEDED]` markers.

Two of them were badly wrong, not merely stale, and had already misled the prose
pass once (HANDOFF.md records it): **ch37's README was titled "Fibonacci
Testbench: 7.1"** and described `FibonacciSeq`/`RspDriver`/`FibEnv`, none of
which exist in that crate — it is the repair desk. **ch38's was titled
"get_response Testbench: 7.2"** and described a `CherryPickSeq` that does not
exist either. Both now match their sources. ch37's run command was also wrong
(`tinyalu`; the crate runs on `playground`).

**ch15–ch21 done too.** All six sim chapters rerun; 52 `src/lib.rs` references
across ch15–ch20 repointed at the real crate roots, and ch21's two repointed at
`rustdv/rustdv-macros/src/rustdv_macros.rs`. Then every transcript line in those
READMEs was checked against the fresh run, which caught three that were not
merely mis-pathed but **wrong**: ch18, ch19 and ch20 carried simulated times 5ns
early throughout (ch18 35/55/75/125 → 40/60/80/130; ch19 45→50 and so on; ch20
145→150, 290→300). The data is identical — same operands, same results, same
seed — so this is D112 landing: the DUT self-clocks now, and the first edge
arrives 5ns later than it did against the externally-clocked version. All three
replaced with the real output and re-verified line by line.

`ch14-modules-crates-cargo/README.md` still says `src/lib.rs` and is **correct**
— that crate really does have one, because it is the chapter that teaches Cargo's
conventions. It is the one deliberate exception to D29, and Part I is frozen.

**New: `rustdv/tinyalu_tb/README.md`.** The shipped testbench had no README, so
the Interlude's and ch40's transcripts had nowhere to be copied from. It now
carries the full `sim/run_rustdv.sh` run verbatim, plus the file list and the
counts, and it says plainly that the crate carries no figure captions.

**New: `output/regression/verify-transcripts.sh`.** One command that reruns all
13 sims and checks every transcript line in every README against the fresh
output. It exists because these transcripts were generated on Linux and go into
a printed book, while macOS/arm64 is a shipping platform.

**Verified on both platforms, 2026-07-30 — 251 transcript lines, character for
character:**

| | Linux/aarch64 (sandbox) | Darwin/arm64 (Ray's Mac) |
|---|---|---|
| Icarus | 14.0 devel | **13.0 stable** |
| Result | all match | all match |

The two runs used *different Icarus versions* and still agreed on every line, so
these transcripts are a property of the framework, not of one toolchain. That is
a stronger result than the check was designed to get.

**The first version of this script was broken, and the Mac run is what exposed
it.** Two defects, both of the same family — a checker that cannot fail:

1. **It compared against output that was never produced.** Every sim failed on
   macOS (no `timeout`; it is GNU coreutils, and the script wrapped every run in
   it), and the script proceeded to diff 13 READMEs against empty files, printing
   a wall of "MISMATCH" that looked like a framework finding and was noise. It
   now aborts before comparing and prints the tail of the failing log inline.
2. **It passed vacuously.** ch16 and ch17 reported "all transcript lines match"
   *in the same run where they had failed to execute* — their READMEs carry no
   timestamped lines, so the comparison loop iterated zero times and reported
   success. Zero transcript lines is now a failure everywhere else, and for those
   two the script verifies the claim they do make ("All 9 tests end `REGRESSION:
   PASS`" → the run must find exactly 9 and pass).

The fixes were mutation-tested rather than assumed: corrupting one timestamp,
inflating ch16's claimed test count, and deleting a transcript wholesale are each
caught, and the clean run still exits 0. A verification script that has never
been made to fail is not evidence — the same argument the mutation check in the
regression rests on, applied to the tool doing the checking.

## 2026-08-04 — the transcript check extended to every chapter, and what it found

`verify-transcripts.sh` originally covered only the 13 chapters that *owed* new
transcripts. Nine more carried README transcripts nothing checked. It now runs
**22 chapters, 431 transcript lines**, and the extension immediately found four
stale READMEs and two other problems:

- **ch23, ch26** — `file:line` references had drifted (ch23 `:152`→`:149`,
  ch26 `:117`→`:124`, and similarly through both files). The code moved; the
  transcripts did not.
- **ch24** — the README documented **one** test; the crate now has **two**
  (`PhaseTest (1/1)` → `(1/2)`). A test was added and the README never caught up.
- **ch29, ch31, ch32** — the READMEs had elided the `[file:line]` suffix from
  their `running …` lines, so they were paraphrases rather than real output,
  unlike every other chapter. Restored.
- **ch25 — a false positive worth keeping.** Its "Verification" block is a
  *mutation demo*: what the run prints with `alu_prediction`'s XOR sabotaged to
  OR. That output cannot appear in a clean run, by design. Rather than
  special-case the chapter, the script now honours an explicit
  `<!-- verify-transcripts: skip -->` marker before a fenced block, and ch25
  carries one. Counterfactual output stays in the README and stays out of the
  comparison.
- **ch26 — a block that could not be checked is now checked.** Its `FileTest`
  deliberately writes to `rustdv_ch26_log.txt` instead of the console, and the
  README quotes that file. The script now folds the log into ch26's captured
  output, so the quote is verified against the real artifact rather than skipped.

All 22 green on Linux/aarch64 after the fixes. **This wants rerunning on the Mac**
— the nine newly-covered chapters have never been checked there.

Also: `TOUR.md`'s "Notes for AI sessions" now carries D113, the
platform-evidence rule, the line-count-neutral caption rule and the
`book-pdf/src` ownership rule, so pointing a new thread at TOUR is sufficient
and prompts no longer restate them.

**The check is now a gate, not a habit.** `custom/readme-transcripts` runs
`verify-transcripts.sh` in the regression, so the pre-push hook fails on
README drift the day it happens. Verified to fail: corrupting one `file:line`
in ch23's README turns the suite red, and restoring it turns it green.

That is the actual lesson from this stretch of work, and it is now a standing
rule in `CLAUDE.md`: **the drift went unnoticed for months while a full green
regression ran over it every single push.** ch23/ch24/ch26 were wrong, and
nothing in 237 passing entries was looking. A rule written in a document is one
someone has to remember; a rule in the regression is enforced. When a class of
error turns up that nothing catches, add the check — and make it fail once on
purpose before trusting it.

## 2026-08-04 — the manuscript's own listings are now checked (ch15–40)

`book-sync` compares ch1–14 listings byte-for-byte against their example files
and has run on every push for months. **It covers Part I only.** Nothing
compared the ch15–40 manuscript — the part being finished right now — against
the crates it quotes. `output/regression/verify-book-listings.py` closes that,
wired in as `custom/book-listings`.

The comparison is code-only: comments and indentation are dropped from both
sides, because the book deliberately strips the crates' teaching comments and
re-wraps. Of 185 listings, **168 verify**, 17 are exempt, 0 are new drift.

**What it found on its first run — a factual error in five places.** The book
still prints `Clock::new(...)` at the top of ch18, ch19, ch20, ch40 and the
Interlude. D112 removed the software clock: the DUT self-clocks, the BFM only
waits on edges, and no crate runs a `Clock` any more. `chapter-notes.md` had
flagged it for ch19 alone. A reader typing any of those five openings gets a
testbench that does not match the one that runs. Recorded for the prose pass in
`chapter-notes.md` and `fable-restart.md`, and held in the script's debt
register so it is counted rather than forgotten.

Mutation-tested: changing one line of ch32's first listing turns the suite red.

### Everything it found is fixed (same day)

The first version of this script excused ten listings as "legitimate
exemptions". Most were not — they were the checker being weak, and the register
was hiding that. Fixed properly:

- **The checker compares one whitespace-normalised token stream**, not
  line-by-line, so the book re-wrapping a long signature is no longer "drift"
  (that alone accounted for ch18/9 and ch35/5). `#[allow(...)]` and
  `#[cfg_attr(...)]` are dropped from the crate side — they silence warnings
  about example code and carry nothing a reader needs. A listing that presents
  two non-adjacent excerpts now passes if **each** excerpt is real.
- **The five `Clock::new(...)` openings are gone from the manuscript** —
  ch18, ch19, ch20, ch40 and the Interlude now show what the crates show. ch40
  and the Interlude also had a whole `start_of_simulation` phase that no longer
  exists in `tinyalu_tb`, and ch40 carried a **paragraph** explaining that the
  shipped testbench drives a clock because it runs against a bare DUT. D112
  retired that exception; the paragraph was false and is rewritten.
- **The D114 `#[component]` sweep is applied** to all 17 manuscript files, so
  the temporary normalisation is deleted and those 50 listings are compared
  strictly.
- **ch21/3** omitted the `#[cfg(target_os = "linux")]` the crate carries;
  added. **ch35/2** silently skipped a `Display` impl; the skip is now marked
  `// ...` and is honest.

**Result: 176 verbatim, 3 spliced-but-real, 2 quoted, 4 elided, 0 drift.**

**The debt register is empty, and the word "register" was itself doing harm.**
It held ten entries, then three, then two — and Ray's objection to the last two
was right: they are *quotations*, not debt. `ch15/2` is the `Future` trait's one
method copied from the standard library; `ch21/2` is a macro expansion the
chapter says outright is tidied. Neither has a crate to match because neither
was ever rustdv's code, and parking them under "must only ever shrink" made
permanent, legitimate content read as unfinished work — exactly the false signal
that teaches people to stop reading a tool's output. They now sit in `QUOTED`,
counted and reported as what they are. `KNOWN_DRIFT`, meaning book and crate
genuinely disagree, is **empty**.

`ch19/1` left by a better route: its BFM skeleton had a placeholder body, so
marking it `// ...` let the generic elision rule take it. Prefer that to an
allowlist entry — a listing that declares itself a fragment needs no
bookkeeping at all.

Full regression after all of it: unit 1, book-sync 108, examples 96, custom 34
— **239 entries, 0 failed**, on Linux/aarch64.

## 2026-08-04 — the manuscript's transcripts, checked and then completed

Extending the transcript gate to the book itself found that **the manuscript's
own transcripts were stale in exactly the ways the READMEs had been** — nothing
had ever compared them either. Forty lines across eight chapters: ch15 quoting
the pre-rename `src/lib.rs`, ch19 and ch20 five nanoseconds early (D112), ch23
and ch26 with drifted `file:line`, ch29/ch31/ch32 with the `[file:line]` suffix
elided so the "transcript" was a paraphrase. **ch24 documented one test where
the crate has two** — `PhaseTest (1/1)` against a real `(1/2)`, with a summary
table listing one row. All corrected from verified output.

`verify-transcripts.sh` now checks every timestamped line in `book-pdf/src`
against the live runs, not just the READMEs. Both are gated by
`custom/readme-transcripts`.

**The 13 `[TRANSCRIPT NEEDED]` placeholders are filled.** Ray reserved the prose
pass for writing, not clerical work, so a code thread ran every sim and pasted
the real output: ch27 ×4, ch28, ch36, ch37, ch38, ch39 ×3, and the full
`sim/run_rustdv.sh` run into both ch40 and the Interlude. The manuscript now
carries **549 transcript lines, every one verified against a live run.** No
placeholder of any kind remains in `book-pdf/src`.

## 2026-08-04 — the end-to-end read

Done mechanically where a machine can be exhaustive, by eye where it cannot.

Checked and clean: figure numbering runs 1..N in order in **every** chapter once
`<figcaption>` SVG figures are counted (ch31's fig 9, ch34's fig 1 and ch36's
fig 1 are drawings, which is why the code captions appear to skip); `SUMMARY.md`
matches the files on disk exactly both ways; no reference to the cut Chapter 41;
no chapter reference out of range; every internal link resolves; every repo path
quoted in the book exists; all 12 `error[EXXXX]` codes reconcile against the 20
checked compile-fail cases; every name in Appendix D and the Toolkit page exists
in the framework; no leftover editorial artifact anywhere; all 50 rendered HTML
pages carry real content.

Two things it found, both fixed:

- **Two stale `src/lib.rs` references survived the crate-root renames** — ch17
  quoting a compiler warning's path, ch21 quoting a runner log line. Neither is
  a Rust listing, so `book-listings` could not see them.
- **A hole the `Clock` fix opened.** Removing `Clock::new(...)` from ch18/19/20
  left only a code comment explaining where the clock went; a reader who met
  `Clock` in ch17 and never saw it again got no reason. Chapter 17 now closes
  its `Clock` figure with the reason — the counter is a bare design, the TinyALU
  clocks itself, and a BFM that waits on edges is the same code on an emulator
  while one that drives them is not.

`ch02`'s two figures both captioned "The corrected program" are intentional
(two mistakes, two corrections) and match the frozen manifest.

Final: unit 1, book-sync 108, examples 96, custom 34 — **239 entries, 0 failed**.

## 2026-08-04 — GitHub CI never honoured the toolchain pin

`custom/compile-fail-methodology` failed on GitHub while passing everywhere
else. It is not a flaky test and the goldens are not stale.

`rust-toolchain.toml` pins rustc to **1.97.0**, and its own comment says why:
"the book's regression checks exact transcript output, so a floating compiler
can break book-sync even when the code is correct." CI used
`dtolnay/rust-toolchain@stable`, which exports `RUSTUP_TOOLCHAIN` — and that
takes precedence over `rust-toolchain.toml`. **CI had therefore been building
with whatever `stable` happened to be, ignoring the pin since the day it was
written.** The compile-fail cases assert exact error codes (E0277, E0308,
E0382, E0599), so they are the first thing a compiler upgrade reclassifies, and
they were the first to break.

Fixed in `.github/workflows/ci.yml`:

- The channel is **read out of `rust-toolchain.toml`** and handed to
  `dtolnay/rust-toolchain@master`, so the pin has one home and CI cannot drift
  from a developer's machine.
- A **guard step** compares `rustc --version` against the pin and fails with an
  explicit message if they differ — the mismatch can no longer be silent.
- `sim-smoke` installed **no Rust at all** and used whatever the runner image
  shipped; it now uses the same pinned toolchain.
- Both jobs `tee` their output, print the tail on failure, re-run the
  compile-fail cases verbosely, and upload the log as an artifact. The original
  failure gave one line — `exit 1, expected 0` — and nothing to diagnose from.

`rustdv/framework-tests/compile-fail/run.sh` now prints the running rustc and
the pinned channel on every run, and dumps the **full** cargo output on a
mismatch instead of three grepped lines, with a note pointing at the pin as the
first thing to check. Mutation-tested: changing an expected code to `E9999`
produces the diagnosis and exits 1.

### The actual cause, which the new logging exposed: ANSI colour

The pin was a real bug but not this one. With full output visible, the next CI
run reported `FAIL port_attr_on_non_port compiled — no longer rejected` **while
printing `error[E0277]` directly underneath it.** The harness and its own log
contradicted each other.

`run.sh` decided "did it compile?" with `grep -E '^error'`. GitHub's cargo emits
**ANSI colour**, so the line does not begin with `error` — it begins with an
escape sequence, and `^error` never matches. Reproduced exactly:
`CARGO_TERM_COLOR=always cargo build` in `port_attr_on_non_port` exits 101 while
that grep finds nothing.

The fix is to stop parsing text for a fact the process already reports:

- **`run.sh` uses cargo's exit status** for "did it compile", passes
  `--color=never`, and strips ANSI anyway before reading the error code. The
  code match is now `E[0-9]{4}` rather than a literal `error[...]` string.
- **`regress.py` had the identical latent bug** at its own compile-fail check —
  `f"error[{code}]" not in r.stderr`, a literal that colour breaks the same way.
  It has not fired yet, which is luck. `run()` now exports
  `CARGO_TERM_COLOR=never` for every subprocess and a `decolour()` helper
  strips escapes before matching.

Verified by running both compile-fail suites with `CARGO_TERM_COLOR=always`,
which is the condition that broke CI: 20 examples cases and the methodology
case all pass. Before the fix, that same command reproduced the failure.

*The general lesson, and it is the same one as the vacuous-checker bug earlier
today:* a test that decides pass/fail by pattern-matching human-readable output
can be defeated by formatting. Ask the process, not the prose — the exit code
was there the whole time.

CI's environment confirms the diagnosis: the failing run's log shows
`CARGO_TERM_COLOR: always` set for the step.

### `custom/sim-lint-verilator` — D112's self-clocking needed `--timing`

Fixed. The extra CI logging paid for itself immediately:

```
%Error-NEEDTIMINGOPT: hdl/tinyalu.sv:15:11: Use --timing or --no-timing to
   15 |    always #5 clk = ~clk;
Verilator 5.020 2024-01-01 rev (Debian 5.020-1)
```

**D112 caused it.** Retiring the bare-DUT exception gave `tinyalu.sv` its own
`always #5 clk = ~clk;`, and from Verilator 5.020 a design containing delays
must state how they are handled or lint fails outright. The oss-cad-suite build
in the sandbox (5.051 devel) is lenient about it, so the change looked clean
locally and only Debian's 5.020 objected — the same shape as the toolchain pin:
a version difference nobody had pinned or noticed.

`sim/run_smoke.sh` now passes `--timing` to the lint. That is the correct
answer rather than a suppression: the delay is real and deliberate, and the
flag says so on every Verilator version instead of leaving it to the release.
Verified `LINT: PASS` on 5.051, and `sim/hdl/tinyalu.sv` is byte-identical to
`output/examples/sim-common/hdl/tinyalu.sv`, so the one lint covers both.

The lint was **not** relaxed. Running it on a stricter tool than the author's
is the reason it is in CI.

*Platform note:* this is a Linux run. macOS/arm64 is a shipping platform and
these transcripts go into the book, so they want confirming on the Mac — a diff,
not a re-read. Fixed seed, single-threaded executor and simulated time should
make them byte-identical; any line that differs is a real finding.

## 2026-07-30 — D114: the child attribute takes no argument

`#[component(child)]`, `#[component(fifo)]` and `#[component(sequencer)]` are
gone; the attribute is bare `#[component]`. Full reasoning in
`output/.design-decisions.md` §42; the short version is that the derive never
read the word. It tested only that one of the three appeared in the attribute
text and set a single flag; what a field becomes is decided by its Rust type
(`RustdvComp` → factory slot, `Option<T>` → declared-but-not-yet-built,
`Vec<T>` → list, otherwise a plain child), and that dispatch reads `f.ty`.

This closes D106's tail, which had asked whether the D84 carve-out should get
one role word or per-type spellings. Neither answer was available: both would
have named something the compiler never saw. `AnalysisBus` declared
`#[component(fifo)]` — the wart D106 recorded — was what prompted reading the
macro, and it was wrong precisely because nothing could catch it.

Changed: all 21 declaration sites under `rustdv/`, every site in the 20 example
crates under `output/examples/`, `skills/rustdv-testbench/SKILL.md`,
the then-current `wire-an-env` helper (subsequently removed), and the derive itself. The workspace
builds clean (Linux sandbox). The derive still matches on
`starts_with("component")`, so an old spelling would compile; none survives in
code and none should return.

**The manuscript is deliberately not changed** — 17 files in `book-pdf/src`
still print the old form, flagged in `book-pdf/chapter-notes.md` for the prose
pass, which is the only pass that edits the book.

~~*Noticed alongside, not fixed:* `#[component(no_factory)]` is now the only
argument the derive parses, `struct_attr_contains` exists to serve it, and no
struct in the tree applies it. Live logic, no call site.~~ **Struck 2026-08-05:
both were deleted afterwards** — neither identifier survives anywhere in
`rustdv/`, and the derive parses no argument. This note was later quoted as
current fact; verify against the code, not against this file.

## The analysis layer gets the UVM's own word (D116/D117, 2026-08-05)

`WriteSink` is now `Subscriber`, and `SubscribePort::on_write()` is now
`subscribe()`. `WriteSink` was an invented name that mapped to nothing a UVM
engineer knows; `uvm_subscriber` is the role's actual name, ch10's prose had
been promising `Subscriber` all along, and a legacy `Subscriber<T>` trait with
the identical signature was sitting in `analysis.rs` marked superseded — the
good name was on the deprecated trait. Reasoning in `.design-decisions.md` §44.

**The legacy surface was confirmed dead before deletion, not assumed dead.**
`Subscriber`, `AnalysisPort`, `connect_fifo` and `FifoAdapter` had exactly
three callers — `tinyalu-utils/src/{tb6,tb7,env7}.rs` — and D45 commented those
modules out of the crate root, so none of them had compiled in months.

**Those files, and the four beside them, are now deleted.** `tb2.rs`, `tb4.rs`,
`tb6.rs`, `tb7.rs`, `alu_item.rs`, `bfm7.rs` and `env7.rs` were kept in `src/`
under D45 so each chapter could lift its copy back as it converted; every
chapter has converted. Their git log is the argument for removing them: `tb6`
and `tb7` were edited on 2026-07-29 to follow the `AnalysisFifo` →
`AnalysisBus` rename and `env7` on 2026-07-30 to follow the `#[component]`
cleanup — maintenance paid on code no compiler ever checked, while `env7.rs`
went on describing constructor injection ("Constructors do build and connect —
review-memo R3") as the design, three months after the restoration reversed it.
Dead code that is still being edited is worse than dead code.

Renamed with the trait, for one vocabulary rather than two:
`ConnectError::NoSink` → `NoSubscriber`, `SubscribePort::sink()` →
`subscriber()`, and the forgot-to-call panic text, which now reads *"has no
subscriber — call `self.<name>.subscribe(handle)` in <owner>'s build phase"*.
**Not** renamed, deliberately: the internal `SinkHandle` trait and `sink_of()`,
which name the erased handle rather than the role and appear in no listing.

No identifier is named `analysis_fifo` (D116). The ch32 crate's three bus
fields are `bus`. `uvm_tlm_analysis_fifo` survives throughout — it is UVM's
class name, and the chapters contrast `AnalysisBus` against it.

The FIFO-tap demonstration moved from the ch31 crate to ch32's (D117), now
Figures 11–13. It reuses ch31's `Producer` and `Consumer` verbatim for its data
path; they sit in the ch32 crate uncaptioned, because reprinting put/get inside
the analysis chapter would re-teach the previous chapter's subject (D115). ch31
is down to four tests and ch32 up to four, so both READMEs and all six affected
transcripts were regenerated from real Icarus runs.

Changed: `port.rs`, `analysis.rs`, `fifo.rs`, both re-export lists,
`tinyalu_tb/components.rs`, six example crates, both example READMEs, the
listings and reference tables in five manuscript files, and
`skills/rustdv-testbench/SKILL.md`. Full regression green on Linux sandbox;
**not yet verified on macOS/arm64.**

**The manuscript's running prose is deliberately not changed.** ch32's section
headings (`## WriteSink: what an arriving item does`, `## on_write and
connect`) and the paragraphs under them still use the retired names, as does
one paragraph each in ch33 and ch34. Only listings, code comments and mapping
tables were touched here; the vocabulary rewrite is the prose pass's, and
`book-pdf/FABLE.md` already carries the instruction.

*Noticed alongside:* the old `wire-an-env` skill still taught the
**pre-restoration** design wholesale — constructor injection, "no factory, no
ConfigDB, no string paths", connection as a compile error. Its dead
`AnalysisPort`/`connect_fifo` lines were a symptom, not the problem. **Ray
deleted the skill on 2026-08-05** rather than repair it; `CLAUDE.md`'s folder
map no longer lists it. Nothing replaced it, and nothing needs to: the current
shape of an environment is in `rustdv/tinyalu_tb/src/` and
`output/examples/ch31…ch34`, which the compiler checks.

## The ch32 prose catches up with its vocabulary (2026-08-05)

**The rewrite the previous entry left to the prose pass is done, same day.**
ch32's section headings are now `## Subscriber: what an arriving item does` and
`## subscribe and connect`, and the paragraphs under them, the counter/collector
commentary, the bus section and the Summary all use the new names. The word
"sink" is retired from the manuscript's running prose entirely (Ray's call,
asked rather than assumed); the internal `SinkHandle`/`sink_of` keep it in
code, deliberately, and appear in no listing. The `uvm_subscriber`-is-a-component
/ rustdv-`Subscriber`-is-plain-data contrast is stated once in the trait
section and echoed in the Summary, and the parent-`connect`s /
component-`subscribe`s split is drawn where the two calls are compared. The
retired-name paragraphs in ch33 (two spots — the FABLE note said one) and ch34
are repaired; ch33's "promise kept" phrasing went with them, per FABLE's
standing rule.

**The FIFO-tap section is written**: "The FIFO's built-in taps" sits between
the slow-subscriber section and the Summary, carrying Figures 11–13 — listings
lifted from the crate, transcript copied verbatim from the README — with
`Producer`/`Consumer` referred back to Chapter 31 rather than reprinted (D115).
`chapter-notes.md`'s ch32 row and its long-form tap item are marked done.

After the edits: `verify-book-listings.py` 0 drift, `verify-transcripts.sh`
all lines match, full regression 239/0 — on the Linux sandbox; **not yet
verified on macOS/arm64.** mdBook HTML builds clean (built to `/tmp`; the
mount refuses deletion of its stale `book/` output). FABLE.md's ch32 top note
is satisfied; striking it is Ray's. Its two remaining top items — the Buy Me a
Coffee header and the final sweep — are untouched, and the coffee link cannot
be done under the prose-pass file boundary: a per-page header lives in the
mdBook theme, not in `src/*.md`.

## The repository README becomes a teaching document, and two checks grow to cover it (2026-08-07)

**The landing page was the stalest document in the tree.** It described the
manuscript as "chapters 1–14", called rustdv "the groundwork for" a framework,
and said nothing about what a rustdv testbench looks like — written before the
restoration, never revisited through it. Ray's comparison was pyuvm's README,
which teaches: it walks a working testbench from the test down to the sequence
item and explains each piece as it goes.

`README.md` is now that walk, built from `rustdv/tinyalu_tb/`. It opens with
what rustdv is and why it keeps the UVM's late binding rather than replacing it
with types (§0.1–0.4, D3, in reader-facing words), carries a UVM-concept map —
capability by capability, **no 1800.2 section numbers and no conformance claim**
(D118) — then installation, a real `sim/run_rustdv.sh` transcript, and fifteen
listings taken verbatim from the crate: the test, the two factory-overriding
tests, the env's `build`/`connect`/`start_of_simulation`, a monitor, the two
`Subscriber` logs, the scoreboard, coverage, the driver, the three sequences,
the transactions, the predictor and the BFM's handles.

**No version numbers in the prose.** Ray's point mid-session: a version written
into a file is a staleness source. The crates.io badge carries the current
version, `rust-toolchain.toml` carries the compiler, and the README points at
both instead of restating them. The MSRV sentence inherited from
`getting-started-with-rustdv.md` ("1.75 or newer") was unverifiable against
anything in the tree and is gone rather than corrected.

**Both drift checks now sweep the file**, because a teaching README quoting a
crate rots exactly the way a chapter does:

- `verify-book-listings.py` yields `("RM", "README.md")` from `chapters()` and
  maps `RM` to `rustdv/tinyalu_tb/src/*.rs`, so README listings compare against
  the crate under ids `chRM/N`. It caught two of them on its first run — the env
  and coverage listings had been trimmed mid-`impl`, which the contiguity rule
  correctly rejects; both were extended to the real block ends rather than
  exempted, and nothing went into `KNOWN_DRIFT`, which stays empty.
- `verify-transcripts.sh`'s manuscript sweep now iterates
  `book-pdf/src/*.md README.md`.

**Both were mutation-tested.** `pub result: u16` → `u32` in a README listing
produced `chRM/13: listing is not in the chapter's crate`, exit 1; `20 compared`
→ `21 compared` in the README transcript produced a MISMATCH naming the line.
Restored, both pass. The two `test.json` files were updated in the same edit —
`book-listings` matches on the summary line, which changed to
`book listings (ch15-40 + Interlude + README):`, and asserting on human-readable
output is the thing that made that a one-line fix rather than a silent pass.

Full regression 239/0 afterward, **on the Linux sandbox; not yet verified on
macOS/arm64.**

**Left standing, deliberately:** `rustdv/getting-started-with-rustdv.md` is the
crates.io front page and still says the published crate is "the 0.0.1 name
reservation" and that `cargo add rustdv` works "once rustdv 0.1 ships". Both
sentences were true when written and are not now. Out of scope for this
session's brief; it is the next obvious documentation job.

## Version numbers leave the prose, and a check keeps them out (2026-08-07)

**Ray's rule, stated mid-session and worth recording as a rule rather than a
correction: a version written into a file is a staleness source.** It is right
the day it is typed and wrong at the next release, and nothing about releasing
prompts anyone to go and fix it. `getting-started-with-rustdv.md` is the
crates.io front page and had been telling readers since the 0.1 launch that
crates.io held "the 0.0.1 name reservation" and that `cargo add rustdv` would
work "once rustdv 0.1 ships" — the two sentences most likely to make a new
reader take the long path for no reason.

That file is rewritten around the live mechanisms. ~~`cargo add rustdv` leads
and the clone follows, described by what it carries that the crate does not (the
DUT, the worked testbench, the figures, the regression, the skills) rather than
by a release that has already happened.~~ **Struck the same day — see the next
entry. Ray's call: the guide is for beginners and offers one path, the clone.**
The two options had been lettered
**A**/**B**; the letters are gone. They were referenced exactly three times,
all inside the two sentences under the table, so they were a lookup key for a
table nobody needed to look back at — and worse, **`Path A`/`Path B` already
meant something else in this repository**: the then-current sandbox toolchain
choice (A = open network access, B = the offline drop), cited that way in the
old setup instructions and offline installer.
Reversing the letters while a second meaning was live in the same tree was a
collision waiting to mislead somebody. The neighbouring "use model 1 / model 2"
numbering stays: it is a real forward reference to two section headings.
Also gone from the file:
the unverifiable `rustc --version # expect 1.75 or newer` (no `rust-version`
field exists anywhere in the tree to check it against — `rust-toolchain.toml`
is the real answer and the text now points there); `iverilog -V # expect
version 11 or newer`, replaced by the actual constraint, which is that the run
scripts pass `-g2012`; "Verilator and commercial simulators are on the roadmap
(see `/STATUS.md`)", which pointed at a roadmap this file has never contained —
STATUS.md is a record of what ran, and now says so; and an incorrect count of
the then-current helper `SKILL.md` files: the guide said four when six existed.
The instruction folder map listed five and was corrected in the same pass
(it was missing `start-thread`). Those helpers have since been retired.

**`custom/no-stale-versions` is the new check** —
`output/regression/verify-no-stale-versions.py`, instant and needing no
toolchain. It scans 95 reader-facing documents for two shapes of the same
mistake: a pinned `rustdv = "0.x"` a reader would copy, and a `rustdv 0.x.y` in
running prose. **`STATUS.md` and the design log are deliberately out of scope**
— they are dated records and are supposed to name the version that ran; scoping
them in would have made every historical entry a violation and trained everyone
to ignore the check. Its `ALLOW` list is empty, is documented as not being a
debt register, and had its one speculative entry removed when the file it
exempted turned out not to contain a version at all.

Mutation-tested both ways before being trusted: appending
`rustdv 0.1.1 is published on crates.io.` to README.md and a
`rustdv = "0.1"` TOML block to the getting-started file each produced exit 1
naming the file, line and reason; restored, both pass.

**The git-lock hazard is now in TOUR.md's "Notes for AI sessions".** Until file
deletion is granted for the mounted folder, the sandbox denies `unlink` there,
and `unlink` is how git removes the lock it takes for *any* index operation —
`git status` included. The failed cleanup prints one warning line and the
leftover zero-byte `.git/index.lock` then fails every later
`git add`/`commit`/`checkout` with `File exists.` Ray does the commits, so the
breakage lands on him with nothing pointing back at a sandbox session. This
session stranded one and, while probing it, briefly turned one stranded file
into three: a same-directory `mv` renames the lock without removing it, a
cross-device `mv` copies and then fails to unlink, and `: > file` makes a fresh
one. All cleared; the note says to request deletion and `rm` rather than
improvise, and records that read-only git never takes the lock.

Full regression 240/0 afterward (239 + the new check), **on the Linux sandbox;
not yet verified on macOS/arm64.**

**Historical setup-document debt — closed 2026-10-26.** The old sandbox setup
guide still described missing tools after an offline installation had solved
that problem. The user later chose deletion; its obsolete guidance and
references were removed.

## The getting-started guide gets one path, and the notes learn to lead with the instruction (2026-08-07)

**`getting-started-with-rustdv.md` offers exactly one way in: clone the
repository.** Ray's call, and the reasoning is that it is a beginner guide —
someone who has never written a rustdv testbench does not need a choice, and
`cargo add rustdv` is a distinction they cannot yet evaluate. The earlier
rewrite this same day had presented the crate and the clone as two things the
guide used together; before that, the file had presented them as two lettered
alternatives. Both framings are gone. Step 2 now gives the two path
dependencies outright — `../../rustdv/rustdv` for the facade and
`../../rustdv/rustdv/rustdv-vpi-stubs` for the test-linking stub, both resolved
against the tree and confirmed. The template `Cargo.toml` and
`new-rustdv-testbench/SKILL.md` were flipped to match, since the guide tells the
reader to copy them; `cargo add` survives in both only as a trailing comment for
someone not working from a clone. Ray: crates.io readers can get their own
instructions later.

**The larger job was the writing itself.** Ray, on finding a memory whose header
said the agent configuration directory could not be edited while its own appended correction said the
opposite: *"You seem to have a writing style that likes to say, 'there is this
problem, but I cleverly fixed it,' and then later you read 'there is this
problem' and stop reading. Stop writing stories about your own cleverness and
start plainly telling future threads what to do."* He is right, and the failure
mode is mechanical: a narrative's opening sentence is a problem statement, so a
reader who stops after one sentence has read the problem and not the answer.

Every memory file and every instruction document was swept. Sixteen memories
opened with a story and now open with the instruction, with the reason — where
it survives — as one `Why:` line at the end. Three were worse than badly
written: `rustdv-open-questions-for-ray` told each session to open by putting a
list of resolved questions to Ray; `dual-audience-book-edit` described a
finished July edit, pointed at three retired style documents, and named a git
branch twice; `book-completion-workflow` claimed "chapters 15–41 + Appendix A,
B" when ch41 was cut by D111 and there are four appendices. The two dead files
are tombstones reading "Ignore this file" and are out of the index. Hard-coded
counts came out of `rustdv-test-suite` — it said 236 regression entries against
an actual 240 — replaced by a pointer to `regress.py --list`, the same move as
`no-stale-versions` makes for version numbers.

In the repo, only two documents offended, and they are the two every session
runs: `start-thread` opened with the history of how it replaced a handoff
prompt, and `close-out-thread` with a paragraph on why a checklist beats good
intentions. Both now open with "Do every step below, then report once."
`skills/rustdv-verify-cover` was reordered from "a green regression proves
nothing" to "make every checker fail on purpose before you report coverage."
CLAUDE.md, `write-a-bfm`, `debug-a-regression` and `rtl-spec-analysis` already
led with instructions and were left alone.

Also struck this session: the word "honest" in `getting-started-with-rustdv.md`
and TOUR.md's three uses. Ray: *"Do you want praise for being honest? Were you
lying before?"* `book-pdf/FABLE.md` has forbidden it since the prose pass. About
29 instances survive in `book-pdf/src` and `output/`; no check was added for
it, because some are direct address the manuscript earns and a check failing on
all of them would be ignored.

Full regression 240/0, both drift checks and `index-decisions.py` green, **on
the Linux sandbox; not yet verified on macOS/arm64.**

## Chapter 35's figures join the regression (2026-08-13)

**Run `regress.py --suite examples --filter ch35 --bless` after any intended
change to a ch35 figure's output.** The seven figures are goldened now; a
re-bless is how you accept a change, and a failure without one is a real
finding.

`ch35-transactions` was the one example crate whose figures nothing ran.
`examples/run/*` entries come from `output/examples/manifest.json`, which held
only Part I bins (chapters 1–14); Parts II–V are simulation chapters reached by
`custom/sim-chNN`. ch35 is neither — it is a Part III chapter of standalone
binaries, so `examples/workspace-build` compiled it and nothing executed it.
Its expected output lived in a header comment per figure and in the chapter's
output blocks, and no check compared either against a run. `custom/book-listings`
covers the *code* in those listings, and `verify-transcripts.sh` matches only
lines shaped `12.34ns INFO …`, so plain `println!` output fell between them.

The seven bins are now manifest entries with goldens under
`output/regression/goldens/`. They were verified against the header comments
before being blessed — all seven already matched, so this closes a gap in
coverage and fixes no drift. The examples suite goes 96 → 103.

**Adding them to the manifest broke `book-sync` on contact**, which was the
useful part. `book-sync` compares Part I figures byte-for-byte and filtered by
chapter on the *book* side only (`parse_book` skips `ch > 14`); the manifest side
was unfiltered and correct only for as long as the manifest held nothing past
ch14. Seven new entries produced seven `figure vanished from the book —
renumbering?` failures. Both sides now read one constant, `BOOK_SYNC_MAX_CH`,
and `--list` applies it too. `book-sync` was re-mutation-tested afterwards — a
one-character edit to a ch13 figure file still fails `book-sync/ch13_fig01` —
because a filter added to silence a failure is exactly how a check loses its
teeth.

**Mutation-checked**, and the first attempt was a no-op that reported success:
`sed s/true/false/` on `ch35_fig03`'s golden changed nothing, because that
figure's output contains only `false`. Changing `ops seen: 4` to `5` failed the
check as it should, and restoring it passed. A mutation test that never mutated
is the failure this project keeps meeting in another costume — check that the
mutation landed, not just that the run came back green.

**The goldens then exposed a second gap, and it got a check rather than a
note.** A golden proves the *program* still prints what was blessed; nothing
proved the *book* still quotes it. `custom/book-listings` compares code only and
`verify-transcripts.sh` matches simulator lines shaped `12.34ns INFO …`, so a
hand-edited `println!` output block in ch35 would have drifted silently between
them. `output/regression/verify-figure-output.py` (`custom/book-figure-output`)
compares each chapter output block — the ```text fence opening with `--` after a
captioned listing — against the golden for that figure. Seven blocks, no drift.
Mutation-checked both ways: editing the chapter fails it, editing a golden fails
it. It reads the manifest, so any future chapter of standalone bins is covered
without touching the script.

`playground_links.py` was fixed in the same pass, before it could do damage. It
widened a figure table's header row and then found the separator by counting
`---` against a hardcoded five, so ch35's three-column table would have gained a
column in the header and not in the separator — a broken table on the next run
of a script nobody runs often. It now matches the separator by position. The
script has not been run against the tree; ch35's README has no "Try it" column
yet, and all seven figures are `std`-only, so they would work in the Playground.

Full regression 248/0 on the Linux sandbox; not yet verified on macOS/arm64.

## Registries and procedural macros move to maintained crates (2026-08-29)

Both link-time registries now use `linkme` distributed slices. The runner and
factory no longer maintain sentinels, platform-specific section symbols, or
unsafe pointer walkers. Chapter 21's runnable registration example uses the
same abstraction and is no longer Linux-only.

`rustdv-macros` now parses with `syn` and emits with `quote`/`proc-macro2`.
The macro surface is unchanged; parsing now handles Rust fields, attributes,
types, generics, and where clauses structurally. D119 records that the Path B
zero-dependency constraint is retired.

## All crates use Rust edition 2024 (2026-08-29)

Every framework, example, and compile-fail package manifest now declares
edition 2024. The migration makes foreign blocks explicitly `unsafe`, wraps
`no_mangle` in `unsafe(...)`, and uses `+ use<>` on the three TinyALU BFM task
factories so their spawned futures capture no borrowed input lifetime.

The chapter 19 listings and explanation match the migrated BFM. Generated Rust
Playground links now select edition 2024, and their generator refreshes existing
links as well as creating the column. Rustfmt remains on its 2021 *style*
edition so a language-edition change does not rewrite the book's source figures.

Framework tests: 118/0. Compile-fail tests: 21/0. The full available regression
reported 221 passes; Icarus entries were skipped because `iverilog` is absent.
The lone sandbox failure, callback RSS sampling, passed outside the sandbox at
48 KiB growth over one million callbacks (16 MiB limit). Other available
Verilator regressions passed.

## Phase waits complete on callback delivery (2026-09-06)

`PhaseFut` now keeps per-registration `TrigShared` completion state. Unrelated
parent polls remain pending and replace the waker until the simulator services
that registration. The hub holds weak registrations, releases each callback's
batch before waking/draining tasks, and retains shared callbacks for surviving
waiters and scheduled writes. ReadWrite flushing and ReadOnly restrictions are
unchanged.

Six new simulator tests cover repeated polls, sibling wakeups in `join_all` and
the component runner, settled combinational data, scheduled-write ordering,
next-timestep advancement, cancellation and waker release/replacement,
consecutive callbacks, and illegal ReadOnly operations. All pass on macOS with
Icarus and Verilator. Restoring the original phase implementation made the
regression runner reject `custom/sim-triggers`, including the component case.

Workspace unit/doc tests, formatting, Clippy (existing warnings), book listings,
and the no-stale-versions check pass. The Reptilia dependency update and removal
of its local workaround are outside this repository change.

Full regression: 252 passed in the sandbox; the sole failure was callback
stress RSS sampling (`ps` is prohibited there). The same million-cycle stress
entry passed when rerun outside the sandbox, completing all 253 enabled checks.
Transcript verification and both simulator scheduler regressions passed.

## CI pins Icarus 13 (2026-09-08)

Icarus 12 delivers next-time callbacks registered from an active callback batch
at the same timestamp. Reproduced the phase-wait regression failure with that
release; Icarus 13 passes with the existing phase implementation. CI now builds
and caches Icarus 13.0 from a checksum-verified release archive on both simulator
platforms. No runtime compatibility workaround is added.

Missing-output failures now include captured command output so simulator
assertions are visible in CI logs.

Validation: built Icarus 13.0 from the pinned archive on macOS using the new
installer, verified its already-installed path, and passed `custom/sim-triggers`
with the original phase implementation. Workflow YAML and shell syntax checks
pass. The macOS build supplies the Apple executable-path header through CFLAGS
because the release driver does not include its declaration.

## 2026-09-06 — typed VPI real/string and aggregate handles

Added `RealHandle` and `StringHandle` with immediate reads, scheduled writes,
and immediate writes. The shared write scheduler preserves ReadOnly guards for
all value types; strings own their data and reject embedded NUL bytes.
`AggregateHandle` exposes named and iterated members of unpacked structs/unions,
including nested aggregates. The facade exports these handles, and wrong-kind
lookups return `HandleError`.

Six `sv_types_` simulator tests run through the Verilator-only feature and
`custom/sim-sv-types-verilator` regression entry. Verified on the active macOS
Verilator development build: real precision, string round trips, scheduling,
ReadOnly guards, mixed/nested struct members, and union aliasing. Existing
Verilator scheduler regression and Icarus signal tests also passed, along with
workspace unit/doc tests, Clippy (existing warnings), and book-listing checks.
API examples and the unpacked-aggregate VPI requirement are documented in
`rustdv/framework-tests/README.md`.

## 2026-09-06 — simulator handle modules

Moved `HierarchyHandle`, `LogicHandle`, `RealHandle`, `StringHandle`, and
`AggregateHandle` into individual modules under `rustdv-sim/src/handle/`.
`AnyHandle` remains in `handle.rs`, which re-exports all concrete handles and
`top_module` so existing imports keep working. Preserved the logic-handle
`From` conversion and lookup behavior. Workspace tests and Clippy pass
(existing warnings only).

## 2026-09-06 — fallible simulator handle conversions

Changed the five `SimHandle::as_*` conversions from `Option` to `Result`.
Wrong-kind errors include the object name and expected/actual kinds; the
metadata-free `Other` variant reports an unknown name. Updated the SV tests
and API examples for the `SimHandle` rename and aggregate child conversions.
Added simulator assertions for all five wrong-kind conversion paths, successful
conversions, and `Other` rejection. Workspace tests, Clippy (existing warnings),
and book-listing checks pass.

## 2026-10-26 — feature documentation and version-bump audit

Compared the published rustdv 0.2.0 archive (source commit
`e2b634f0d06a22e3de666a9f1ca8181e91fdbd9d`) with the prepared 0.3.0 source.
README now summarizes typed/wide integer access, real/string and aggregate
handles, explicit DUT selection, settled inspection, runtime waveforms, and
Verilator RTL coverage. Added `rustdv/signal-access.md` with typed access,
write/conversion semantics, simulator requirements, and breaking-API migration.
The packaged getting-started guide links to it and distinguishes the Rust APIs
from the repository's simulator host and launchers.

Expanded `sim/README.md` with the coverage controls, a runtime capture example,
capability/lifecycle errors, and the wall-clock cancellation requirement for
settled service closures. Updated lz4 guidance for both waveform build modes.
Replaced getting-started references to removed templates/helpers with the
shipped crates, launcher, and verification skills. TOUR reflects these features
and the workspace location. CLAUDE now requires a published-source feature and
documentation audit on every framework version bump and carries a session
close-out checklist in place of the removed helper.

Validation on macOS/arm64: all three new Rust snippets compile against the
workspace with its locked dependency versions; all seven selected signal,
variable-type, settled-service, runtime-trace, inspection, and coverage regression
entries pass. Every manuscript/README transcript matches a real Icarus run,
book listings report zero drift, local Markdown links resolve, and the
no-stale-versions check passes. Added the signal-access guide to that check's
scope and confirmed an injected hardcoded version fails before restoring the
guide. `cargo package --list` confirms the new guide is included in the facade
crate. No Rust implementation or manuscript listings changed.

## 2026-10-26 — book feature audit and release instructions

Updated the Toolkit and Appendices B/C/D for typed and wide integer access,
real/string/aggregate handles, handle traits and conversions, scheduler-region
triggers, settled service operations, and optional runtime tracing. Appendix D
now matches all 79 actual prelude names; simulator-layer `SimHandle` and
`HandleBase` are documented at their correct explicit-import paths, while the
low-level GPI `AnyHandle` remains distinguished. Corrected the Python table's
component-registration description and the distinction between a lookup's
`Result` and its unwrapped handle.

Chapter 17 explains root selection, unsigned width and X/Z errors, four-state
array ordering, richer variable types, and settled sampling. Chapter 40 adds
optional Verilator inspection, capture lifecycle and service constraints, and
the difference between transaction-level functional coverage and RTL
line/expression coverage. The teaching sequence, code/output blocks, and figure
captions are unchanged. Recorded the audit in chapter-notes.md and corrected
its obsolete Appendix D and software-clock guidance for the affected chapters.

CLAUDE.md's release checklist now explicitly requires the book audit on every
release, including actual prelude/facade exports, import paths, affected prose,
and chapter notes; passing existing example checks does not prove complete
feature documentation. TOUR.md points to that requirement.

Validation: all 108 book-sync checks pass, the book-listing check reports zero
drift, the no-stale-versions check passes, and local links resolve. Verified
byte-for-byte preservation of every fenced code/output block and figure
caption in all six edited book pages. Built the HTML book successfully with
the official macOS mdBook binary into per-user temporary storage. PDF output
was skipped because the optional mdbook-pdf renderer is not installed.

## 2026-10-26 — versioned changelog

Added CHANGELOG.md with prepared 0.3.0 features, fixes, tooling changes, and
breaking API migrations. Confirmed from crates.io that 0.3.0 is not yet
published and kept its heading unreleased. Reconstructed concise 0.2.0 and
0.1.0 notes from source history, and included the intervening 0.1.1 README
packaging patch. Historical publication dates and source commits were read
from crates.io and each archive's `.cargo_vcs_info.json`; the first published
release's source commit is newer than its local tag, so links use the actual
published commit where necessary.

Linked the changelog from README and TOUR. CLAUDE's release checklist now
requires updating release notes, preserving version history, and confirming
publication before assigning a release date and replacing the prospective
comparison link. Documented versioned history as intentionally outside the
guide-only no-stale-versions rule. Checked historical source references,
local guide links, entry headings/dates, the existing version checker, and
diff whitespace. No implementation or book listings changed in this step.

## 2026-10-26 — paired release handoff and prepared GitHub notes

Added the crates.io/GitHub release sequence to CLAUDE.md and a pointer in TOUR:
validate and publish the intended clean source, verify all eight crate versions,
tag that exact source commit, and use `gh release create --verify-tag` with
prepared notes. Later changelog bookkeeping does not move the source tag.
Existing user authorization still controls commit, push, and publication.

Generated the user-requested `/tmp/release-notes.md` from the 0.3.0 changelog
body, excluding the header, introduction, and older releases. Documentation
links and the comparison URL in the copied notes use the intended release
tag. The instructions require refreshing this file and handing the user a
concrete release command, with publication/tag prerequisites stated.
Validated the extracted section, normalized links, file contents, and diff
whitespace. No tag, crate publication, or GitHub release was created.

## 2026-10-26 — consolidated book agent instructions

Consolidated FABLE.md, fable-prompt.md, fable-restart.md, and HANDOFF.md from
973 lines into the 204-line book-pdf/CLAUDE.md. Preserved the audience and
tone study, source/verification rules, chapter exemplars and workflow,
durable terminology/teaching decisions, figure conventions, and release audit.
Removed completed work orders, model-specific restart routing, duplicated
progress, and contradictory historical reminders. The existing renumbering
spec remains explicitly historical; chapter notes and STATUS retain ongoing
chapter work and validation rather than creating another handoff register.

Added book-pdf/AGENTS.md pointing to the consolidated guide and parent rules.
Removed the four superseded files and updated active references in root
instructions, TOUR, the book README, chapter notes, and renumbering spec.
Checked delegation and local links, found no active references to deleted
guides, passed listing/version/whitespace checks, and rebuilt HTML successfully.
The optional PDF renderer remains unavailable. No manuscript content changed
in this consolidation; earlier uncommitted edits are preserved.

## 2026-10-26 — chapter-local instructions and caption ledger

Folded the remaining chapter-notes and renumbering-spec guidance into the new
instruction structure. Shared book rules and a compact, explicitly historical
caption-reconciliation table live in book-pdf/CLAUDE.md. Chapter-specific
teaching constraints now live in 25 CLAUDE.md files beside the relevant example
crates and the shipped TinyALU testbench, indexed by the book guide. The
Chapter 33 folder has a pointer to the shared Chapter 34 source/instructions;
the Interlude and Chapter 40 share the TinyALU guide. Chapters with no distinct
local constraints continue to use the shared rules.

Retained the non-Copy try_put handback, concurrent pipeline, subscriber-owned
storage and slow inbox, FIFO taps, transaction equality/printing, sequencer
handshake/ownership, and deterministic out-of-order teaching requirements.
Discarded obsolete rewrite commands, stale listing counts, and completed
transcript reminders. Removed chapter-notes.md and renumbering-spec.md and
updated active parent/orientation references; root instructions now direct
example edits to the relevant local guide.

Validated all local links across 30 instruction/orientation files, confirmed
no active references to the two retired files, passed listing/version/diff
checks, and rebuilt HTML. Manuscript code/output blocks are unchanged. The
optional PDF renderer is still unavailable. All earlier uncommitted work is
preserved; no commit or push was made.

## 2026-10-26 — removed agent pointer files

Inspected both repository AGENTS.md files (root and book-pdf). Both only
directed agents to their CLAUDE.md instructions; neither carried additional
rules. Removed both at the user's request and updated active references in
root instructions, TOUR, and the book README. A full repository walk excluding
Git internals confirms no AGENTS.md files remain. Changes are uncommitted.

## 2026-10-26 — renamed instruction files to AGENTS.md

At the user's subsequent request, renamed all 28 repository CLAUDE.md files
to AGENTS.md, including the full root rules, shared book guide, chapter-local
guides, and TinyALU instructions. Updated active links and filename references
in the guides, TOUR, and book README. Historical records retain their original
filenames as history. Verified no CLAUDE.md files remain, all instruction links
resolve, and version/whitespace checks pass. No commit or push was made.

## 2026-10-26 — consolidated crate README

Merged getting-started-with-rustdv.md and signal-access.md into rustdv/README.md
and removed the old guides. The combined guide covers installation, the known
example, external project/library setup, typed values and migration, simulator
modes, and staged verification with optional AI assistance. Preserved both
signal-access Rust examples, removed overlapping introductions, added section
navigation, and clarified external toolchain/linker configuration and native
library names. The repository overview remains separate.

Changed the facade manifest's readme field to README.md and updated root
README, TOUR, release instructions, changelog links, and the version check's
scope/commentary. Refreshed /tmp/release-notes.md with the new migration link.
Historical STATUS entries keep the old filenames as history.

Validation: both Rust examples compile offline against workspace dependencies;
the packaged file list includes README.md and neither retired guide. Local
file/section links resolve, book listings report zero drift, and the version
guard passes. Injecting a version in the new README makes that guard fail,
then restoring the guide returns it to passing. Task-file whitespace checks
pass. No Rust implementation changed, and unrelated pending edits are preserved.

## 2026-10-26 — retired skill-path references

Removed the old agent-configuration-directory paths from TOUR and historical
STATUS/design notes. Current verification skills are in skills/. Historical
environment-wiring/helper references are described as retired rather than
redirected to nonexistent replacements. Confirmed the three current SKILL.md
files exist and no obsolete directory-path references remain in source docs.
Unrelated external documentation URLs and pending user edits are preserved.

## 2026-10-26 — chapter teaching notes moved into READMEs

Moved all 24 “Preserve these teaching points” sections under output/examples
into the neighboring READMEs, including their manuscript links. Removed the
24 AGENTS.md files whose remaining content was shared boilerplate. Kept
Chapter 33's AGENTS.md because it has separate instructions about the shared
Chapter 34 source. Updated its link, the book's chapter index, repository
instructions, TOUR, and the example overview to find the relocated notes.

Validation: all original chapter README content, code/output fences, teaching
bullets, and manuscript links are preserved; 63 local links resolve. Book-sync
passes all 108 checks, book listings report zero drift, and the stale-version
guard passes. No source code changed, and no commit or push was made.

## 2026-10-27 — script to extract release notes

Added scripts/release-notes.py, a Python standard-library command taking a
version with or without a leading v. It extracts that exact changelog entry
into /tmp/release-notes.md by default, with an optional --output destination.
It resolves relative documentation links at the release tag and updates the
prospective GitHub comparison. Missing, duplicate, empty, or invalid versions
fail before writing. Updated AGENTS.md's release procedure and TOUR to use
the script, and regenerated the current release notes.

Validation: exercised all four changelog releases, the leading-v form, execution
from another directory, exact heading boundaries, fenced headings, final-entry
extraction, link normalization, and error cases preserving existing output.
Generated notes match the previously prepared release notes. The stale-version
guard and task-file whitespace checks pass. No commit or push was made.

## 2026-10-27 — release notes use stdout

Changed scripts/release-notes.py to print only the extracted notes to stdout;
removed the output-file option and default temporary-file write. Errors still
go to stderr and exit nonzero. Updated AGENTS.md and TOUR to capture the script
output directly for gh release create --notes, with extraction success gating
release creation. This replaces the earlier temporary-file workflow.

Validation: all four releases and the leading-v form produce only notes on
stdout; invalid/missing versions produce only stderr and fail. Confirmed failed
extraction stops the following shell command and no temporary file is written.
The stale-version guard and whitespace checks pass. No commit or push was made.

## 2026-10-27 — repository tour consolidated

Removed TOUR.md after consolidating its useful material. AGENTS.md now contains
environment/build safeguards, targeted test commands, the intentional Chapter 21
simulator exclusion, and the rule to verify documentation claims against code.
README.md keeps reader orientation, adds the Fibonacci entry point and testing
overview, and indexes the existing guides. Its existing Interlude, compile-error
examples, simulator features, and TinyALU transcript already cover the remaining
reader-facing material. Updated the design log's current-status pointer. Older
plain-text mentions of the retired tour remain historical records; current
instructions are in AGENTS.md, reader orientation in README.md, and dated
implementation/validation history here.

The retired tour summarized completed restoration work: component phases,
runtime configuration/factory/TLM, the shipped TinyALU regression, and the
Subscriber/subscribe vocabulary and Chapter 32 FIFO-tap demonstration. It also
recorded the August 5 manuscript publication sweep. Those completions belong to
the existing dated records, not another live work queue. This consolidation does
not claim a new simulator run or publication.

Retired environment assumptions included a networkless Cowork VM, fixed
toolchain-drop paths, fixed process/disk limits, and a separate deletion
permission tool. Preserve the underlying lessons in AGENTS.md: use per-user
temporary builds, keep foreign binaries out of synced checkouts, diagnose Git
lock permissions, check lz4/PDF prerequisites, and report actual platform
coverage. Use current tools and permissions instead of replaying old VM setup.

Validation: all 32 local links in README.md and AGENTS.md resolve; no active
references or links to the deleted tour remain. Existing README code/output
blocks are unchanged. Book listings report zero drift, all 108 book-sync checks
pass, and the stale-version and task-file whitespace checks pass. No Rust or
simulator code changed; no commit or push was made.

## 2026-10-27 — release-note type annotations

Annotated the release-note entry lists, optional parser state, and regex
replacement callback in scripts/release-notes.py. Pyright strict mode now
reports zero errors or warnings with Python 3.9 compatibility. Compared stdout,
stderr, and exit status before/after for all four releases, the leading-v form,
and invalid/missing versions; behavior is unchanged. Whitespace checks pass.
No commit or push was made.

## 2026-10-27 — finalize the changelog in the release commit

At the user's request, dated the 0.3.0 changelog entry 2026-10-27 and changed
its comparison endpoint from HEAD to v0.3.0 before the release commit. Updated
the changelog introduction and AGENTS.md so release preparation finalizes the
intended date and tag comparison together with the published source, removing
the routine post-publication bookkeeping commit. Publication and tag/source
verification remain required; dating the entry does not perform publication.

Validation: extracted 0.3.0 release notes are byte-for-byte unchanged after the
date/comparison update. The stale-version guard and whitespace checks pass.
No commit, push, tag creation, or publication was performed.

## 2026-10-27 — rebuilt the distributed book PDF

Rebuilt the current manuscript with mdBook 0.5.4, mdbook-pdf 0.1.13, and local
Chrome on macOS, then replaced Rust4RTLVerif.pdf with the verified output.
The distributed book now has 472 pages (previously 468) and includes the
updated Toolkit, Chapters 17/40, and Appendices B–D. Build products, backend
installation, and review images stayed under per-user temporary storage.

Release preparation in AGENTS.md now requires a fresh PDF build, visual review,
and replacement of the root PDF before the release commit. The book's guide
and instructions explain the same requirement and reject an HTML-only build
as release completion. Corrected the documented backend output to
book/pdf/output.pdf and replaced obsolete offline-VM assumptions with current
prerequisites and a fresh-directory build/check/copy procedure.

Validation: checked all 472 pages contain extracted text; confirmed the updated
signal, scheduler, trace, and RTL-coverage APIs appear. Rendered and visually
reviewed 26 pages covering the changed sections and page-number footers, with
full-size checks of dense tables and new simulator guidance. No characters
extend outside the reviewed page bounds. The copied PDF's SHA-256 matches the
fresh build. All 108 book-sync checks, zero-drift book listings, the stale-version
guard, and task-file whitespace checks pass. No commit or push was made.

## 2026-10-27 — release PR preparation

Prepared the complete pending documentation, release tooling, instruction
consolidation, contribution guidance, and rebuilt PDF changes for the requested
release PR. The framework versions were already bumped to 0.3.0 in the earlier
merged release-version PR. Aligned README's contribution policy with the
updated CONTRIBUTING.md and added the separate website update/docs.rs checks
to the release procedure. Removed trailing whitespace in the edited guides.

Validation on macOS: formatting and strict workspace/all-target Clippy pass;
the full regression reports 259 passed, 0 failed, including Icarus and Verilator
simulator paths. Book listings report zero drift, the stale-version guard
passes, and strict Pyright reports zero diagnostics for the release-notes
script. Extraction works for all four changelog versions and rejects empty or
duplicate entries. The facade package list includes the consolidated README;
the distributed PDF is the reviewed 472-page build. No crates, release tags,
GitHub releases, or website deployments were published during preparation.
