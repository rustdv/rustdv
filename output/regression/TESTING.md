# Regression testing for Rust for RTL Verification

One command guards the whole project:

```
output/regression/regress.py
```

Exit 0 means nothing you care about has changed behavior. Four suites run, in
this order:

| Suite | Question it answers | Needs Rust? |
|---|---|---|
| `unit` | Do the framework's own rules still hold? (`cargo test`, no simulator, ~2s) | yes |
| `book-sync` | Does every book figure still have a matching, verbatim example file? | no |
| `examples` | Does every example still build, run, print, panic, or fail-to-compile exactly as blessed? | yes |
| `custom` | Do the simulator tests and the drop-in tests still pass? | per test |

`unit` runs first on purpose. A ConfigDb precedence bug should fail in two
seconds with the rule's name on it, not four minutes later as "sim-ch27 went
red".

## One-time setup

```
output/regression/regress.py --bless          # freeze today's known-good outputs
output/regression/regress.py --install-hook   # refuse to `git push` a broken state
```

`--bless` runs every figure binary and records its stdout under
`goldens/`. Commit the goldens — they are the baseline every future run is
compared against. The hook can be bypassed in an emergency with
`git push --no-verify`.

## Daily use

```
output/regression/regress.py                       # everything
output/regression/regress.py --suite book-sync     # fast, no cargo needed
output/regression/regress.py --filter ch09         # just chapter 9 tests
output/regression/regress.py --list                # see all test ids
```

## The workflow that keeps bugs out

**Changed a figure in the book?** `book-sync` fails for that figure, telling
you the example is now stale. Update the example file (keep the figure code
verbatim; context blocks and the header comment are yours to maintain), then
re-run. If the program's output changed too, re-bless just that figure:

```
output/regression/regress.py --bless --filter ch09_fig03
```

**Added a new figure?** Add the example file following the naming convention
(`chNN_figMM_slug`), add an entry to `output/examples/manifest.json`, add a
row to the chapter README, then `--bless --filter chNN_figMM`. The book-sync
coverage test fails until book and examples agree.

**Renumbered figures?** `book-sync/coverage` lists exactly which figure
numbers no longer line up on each side.

**Changed example code (or upgraded Rust)?** Run the full suite. Output
diffs, exit-code changes, and changed compiler-error codes all surface as
failures. Bless only what you intended to change — an unintended diff IS the
regression.

**Adding functionality (e.g. the rustdv crate)?** Two options:

- Rust unit tests: put `#[test]` functions in the crate, then add the package
  name to `cargo_test_packages` in `regress.json`.
- Anything else: copy `tests/example-template/`, rename it, edit `test.json`
  (see the comment field for all options), remove `"disabled": true`. Each test
  declares a command plus expectations: exit code, required output substrings,
  and/or a golden stdout file. A `skip_if_missing` field names a required
  executable — the test skips (rather than fails) on machines without it.
  The simulator smoke tests (`tests/sim-*`) use this: they run where
  Icarus/Verilator are installed (including CI) and skip elsewhere.

## Continuous integration

`.github/workflows/ci.yml` runs the full no-simulator regression plus a
free-simulator matrix. Linux runs the Icarus and Verilator entries; macOS
repeats the Verilator runtime, scheduler, DEBUG/FST, and mutation entries.
Commercial simulators cannot run in public CI; license-holders run
`sim/run_smoke.sh <sim>` locally.

The rule of thumb: **every new piece of functionality lands together with the
test that would catch its removal.** The pre-push hook then makes it
structurally hard to break something silently.

## Special cases the suite knows about

- **Intentional compile errors** (20 figures): the test asserts compilation
  *fails* with the book's error code (e.g. E0382). If a Rust upgrade changes
  an error code, the test names both codes.
- **Intentional panics** (3 figures): the test asserts a nonzero exit.
- **HashMap-order figures**: listed in `regress.json` under `compare_modes`.
  `unordered_lines` ignores line order (map iterated one entry per line);
  `normalized_maps` sorts the entries inside `{...}` on each line (map
  printed inline with `{:?}`).
- **Marked deviations from the book**: if an example must intentionally
  differ from the manuscript, list it under `deviations` in `regress.json`
  and put a `deviation from the book` comment at the changed line — book-sync
  then checks for the marker instead of demanding verbatim match. (Currently
  none; the original two were resolved by fixing the manuscript — see
  `output/examples/ERRATA.md`.)

## The three tiers of framework test

The chapter runs (`custom/sim-ch*`) prove a chapter's testbench still works
end to end. They are good tests and they cannot tell you *which* rule broke.
Three narrower tiers can:

**No simulator — `cargo test`, in `rustdv/`.** Anything built from `Event`s
and `Queue`s: the ConfigDb, the factory, port binding, the analysis
broadcast, the whole sequencer handshake. These live in `#[cfg(test)] mod
tests` beside the code they test and run in milliseconds. The line is
enforced by `rustdv_sim::testing::block_on`, which panics with an explanation
if the future is still pending when the run queue empties — a test that
awaits `Timer` or touches a signal fails there rather than hanging, and
belongs in the next tier.

**Targeted simulator tests — `rustdv/framework-tests/`.** One cdylib against
`hdl/probe.sv` (a clock, signals of known width, one counter). Each module
prefixes its test names, and `RUSTDV_TESTCASE` selects a group, so
`sim-triggers`, `sim-clock`, `sim-signals`, `sim-concurrency`,
`sim-elaboration` and `sim-runner` are six lines in the regression report off
one build:

```
rustdv/framework-tests/run.sh          # all of them
rustdv/framework-tests/run.sh conc     # one group
```

`sim-mutation` is separate and is the check with teeth: it corrupts the
TinyALU's XOR to an OR, requires two testbenches to **fail**, then requires
both to pass again on the real RTL. A scoreboard that cannot fail is not a
scoreboard.

The Verilator entries add a host-scheduler contract, the full TinyALU Rust
testbench, selected-internal DEBUG/FST output, and the same mutation check.
The scheduler test proves that VPI-only timers keep the model alive, the
earlier of RTL and VPI deadlines wins, a ReadWrite VPI write settles through
combinational RTL before ReadOnly, and final/end-of-simulation hooks run.

**Compile-fail — `rustdv/framework-tests/compile-fail/`.** Claims the book
makes about what the compiler rejects. Each case asserts its `error[E….]`
code, not merely that the build failed; without the code a case that started
failing for an unrelated reason would keep passing and stop testing anything.

### Two things a simulator test has to know

Both are properties of the runner, not quirks of these tests:

- **The simulator phase outlives the test — the runner, not the test, deals
  with it.** A test ending inside ReadOnly used to leave the next one starting
  inside ReadOnly, where a write is a panic, and every test that wrote a signal
  opened with a `fresh_phase()` call to step out. Since D108 the runner does it:
  `run_one` awaits `phase::leave_read_only()` before anything else, which costs
  one precision step when the predecessor ended in ReadOnly and nothing at all
  otherwise. Write a test as if it were the only one running. The cost is
  visible: a test that follows a ReadOnly-ending test starts one step later
  than its own arithmetic suggests, so measure elapsed time between two
  `sim_time_ns()` reads rather than from an assumed start.
- **A simulator exits when every relevant event queue empties.** A test with no clock and no
  pending timer that awaits `next_time_step()` will not be woken — the
  simulator quits and the rest of the regression never runs. Keep a clock or a
  timer alive. The Verilator host considers both RTL events and VPI deadlines.

### Adding a `test.json` option

`test.json` accepts an `"env"` object whose keys are set in the test's
environment. The six targeted entries use it for `RUSTDV_TESTCASE`.

## Relationship to check.sh

`output/examples/check.sh` answers "do the examples match **the book's
transcripts**?" — useful when editing the manuscript. `regress.py` answers
"did anything change since the **last blessed state**?" — that's the
regression guard. check.sh compares against prose that may contain errata;
regress.py compares against reality you approved.
