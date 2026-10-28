# Chapter 25: uvm_env Testbench: 4.0 — figure map

Run with:

```
sim-common/run_sim.sh ch25_uvm_env_testbench_4_0 tinyalu \
    sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
```

Testbench 4.0 is where the pieces become **components**. At 3.0 the test was
the only component, and the tester and scoreboard were ordinary locals
inside `run`. Here each becomes a component with its own phases, and an
*environment* holds them — the reusable unit UVM is built around.

| Figure | Title | Where |
|---|---|---|
| 1 | What varies between testers is only the operands | `Operands` trait |
| 2 | RandomOperands and MaxOperands | `src/ch25_uvm_env_testbench_4_0.rs` |
| 3 | BaseTester implements the phases common to all testers | `BaseTester<T>` |
| 4 | The two testers are type aliases over the base | `RandomTester`, `MaxTester` |
| 5 | The Scoreboard as a component | `Scoreboard` |
| 6 | Launching the monitoring tasks in start_of_simulation | `Scoreboard::start_of_simulation` |
| 7 | Checking results in the check phase | `Scoreboard::check` |
| 8 | The environment builds the scoreboard and a tester | `AluEnv<T>` |
| 9 | RandomEnv and MaxEnv are type aliases | type aliases |
| 10 | Each test builds the environment it wants | `RandomTest`, `MaxTest` |

Transcript (seed 1):

```
      0.00ns INFO     rustdv: found 2 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running RandomTest (1/2)  [ch25-uvm-env-testbench-4.0/src/ch25_uvm_env_testbench_4_0.rs:256]
    150.00ns INFO     [RandomTest.env.scoreboard]: PASSED: c1 Add 67 = 0128
    150.00ns INFO     [RandomTest.env.scoreboard]: PASSED: 5e And 0b = 000a
    150.00ns INFO     [RandomTest.env.scoreboard]: PASSED: b9 Xor 80 = 0039
    150.00ns INFO     [RandomTest.env.scoreboard]: PASSED: a5 Mul 75 = 4b69
    150.00ns INFO     [RandomTest.env.scoreboard]: Covered all operations
    150.00ns INFO     RandomTest PASSED
    150.00ns INFO     running MaxTest (2/2)  [ch25-uvm-env-testbench-4.0/src/ch25_uvm_env_testbench_4_0.rs:271]
    300.00ns INFO     [MaxTest.env.scoreboard]: PASSED: ff Add ff = 01fe
    300.00ns INFO     [MaxTest.env.scoreboard]: PASSED: ff And ff = 00ff
    300.00ns INFO     [MaxTest.env.scoreboard]: PASSED: ff Xor ff = 0000
    300.00ns INFO     [MaxTest.env.scoreboard]: PASSED: ff Mul ff = fe01
    300.00ns INFO     [MaxTest.env.scoreboard]: Covered all operations
    300.00ns INFO     MaxTest PASSED
******************************************************************************
** TEST                                       STATUS  SIM TIME (ns)      **
******************************************************************************
** RandomTest                                   PASS         150.00      **
** MaxTest                                      PASS         150.00      **
******************************************************************************
REGRESSION: PASS
```

## Two things worth reading the code for

**A component asks for the BFM by name.** The tester and the scoreboard are
siblings created in their parent's `build` phase, so neither can be handed the
BFM through a constructor — `build` passes nothing (D6). The test puts one in
the ConfigDb and each component asks for it:

    ConfigDb::set(None, "*", "BFM", Rc::new(bfm));                  // the test
    let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM")?; // a component

This is where the ConfigDb first earns its place; Chapter 27 is the full
treatment. SystemVerilog does exactly this —
`uvm_config_db#(virtual tinyalu_bfm)::set(null, "*", "bfm", bfm)` sits in the
`top.sv` of every UVM testbench — while pyuvm reaches the same place with a
singleton. Both source languages could have threaded a handle through
constructors and neither did (D3); rustdv follows SystemVerilog's answer rather
than pyuvm's, so the reader learns one mechanism instead of two (D101).

The BFM is scoped to **one test**: the runner clears the ConfigDb before each
test, matching pyuvm's `run_test`, so `MaxTest` never inherits `RandomTest`'s
half-drained queues.

**Inheritance becomes one generic written once.** Python needs
`BaseTester` + `RandomTester` + `MaxTester`, and `BaseEnv` + `RandomEnv` +
`MaxEnv` with the subclasses calling `super().build_phase()`. rustdv writes
`BaseTester<T: Operands>` and `AluEnv<T>` once; the four variants are type
aliases (D28).

The scoreboard's path, `[RandomTest.env.scoreboard]`, is derived by the walk
three levels down — nothing stores it.

## Verification

Not vacuous: with `alu_prediction`'s XOR sabotaged to OR, both tests report
the offending transaction and the run ends `REGRESSION: FAIL`.

<!-- verify-transcripts: skip -->
```
150.00ns ERROR    FAILED: b9 Xor 80 = 0039 - predicted 00b9
300.00ns ERROR    FAILED: ff Xor ff = 0000 - predicted 00ff
REGRESSION: FAIL
```

## Preserve these teaching points

Manuscript: [chapter-25-uvm-env-testbench-4.0](../../../book-pdf/src/chapter-25-uvm-env-testbench-4.0.md).

- The BFM is shared through ConfigDb, not a singleton. Multiple interfaces can
  need multiple BFMs; the book does not build that larger example, so do not claim
  it demonstrates one.
- Introduce enough configuration to read set/get here; Chapter 27 is the full
  treatment. State the learning cost of introducing a database, path glob, and
  Result while explaining the environment.

Read this crate’s source and this README before changing a quoted
figure or behavioral claim; keep the source, figure map, and transcript aligned.
