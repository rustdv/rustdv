# Chapter 23: uvm_test Testbench: 3.0 — figure map

Run with:

```
sim-common/run_sim.sh ch23_uvm_test_testbench_3_0 tinyalu \
    sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
```

The 2.0 classes (`Tester`, `RandomTester`, `MaxTester`, `Scoreboard`) are
**re-shown in this chapter's file**, marked "Copied from testbench 2.0", as
the Python book does (D45). They used to be imported from
a `tb2` module in `tinyalu-utils`, since deleted; the import went because
these classes evolve — `Tester` is a plain trait at 3.0 and a component at
4.0 — and hiding them behind an import hides the change the book is about.
`tinyalu_utils` now carries infrastructure only: the BFM, `Ops`,
`CmdTuple`, `alu_prediction`.

There is no software clock: the RTL self-clocks (D42) and the BFM only
waits on edges.

| Figure | Title | Where |
|---|---|---|
| 1 | The basic rustdv-UVM use model in hello_world | `src/ch23_uvm_test_testbench_3_0.rs` (`HelloWorldTest`) |
| 2 | Hello, world! | transcript |
| 3 | The uvm_test tower, and its rustdv equivalent | text diagram |
| 4 | alu_test — the shared run phase of every test | `src/ch23_uvm_test_testbench_3_0.rs` |
| 5 | The tests choose a tester and share alu_test | `src/ch23_uvm_test_testbench_3_0.rs` |
| 6 | RandomTest passes | transcript |
| 7 | MaxTest maxes all the operands | transcript |

All 3 tests end `REGRESSION: PASS` (RUSTDV_RANDOM_SEED=1):

```
      0.00ns INFO     rustdv: found 3 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running HelloWorldTest (1/3)  [ch23-uvm-test-testbench-3.0/src/ch23_uvm_test_testbench_3_0.rs:149]
      0.00ns INFO     [HelloWorldTest]: Hello, world.
      0.00ns INFO     HelloWorldTest PASSED
      0.00ns INFO     running RandomTest (2/3)  [ch23-uvm-test-testbench-3.0/src/ch23_uvm_test_testbench_3_0.rs:188]
    150.00ns INFO     PASSED: ce Add 42 = 0110
    150.00ns INFO     PASSED: 2f And 64 = 0024
    150.00ns INFO     PASSED: 29 Xor b3 = 009a
    150.00ns INFO     PASSED: 86 Mul 83 = 4492
    150.00ns INFO     Covered all operations
    150.00ns INFO     RandomTest PASSED
    150.00ns INFO     running MaxTest (3/3)  [ch23-uvm-test-testbench-3.0/src/ch23_uvm_test_testbench_3_0.rs:199]
    300.00ns INFO     PASSED: ff Add ff = 01fe
    300.00ns INFO     PASSED: ff And ff = 00ff
    300.00ns INFO     PASSED: ff Xor ff = 0000
    300.00ns INFO     PASSED: ff Mul ff = fe01
    300.00ns INFO     Covered all operations
    300.00ns INFO     MaxTest PASSED
******************************************************************************
** TEST                                       STATUS  SIM TIME (ns)      **
******************************************************************************
** HelloWorldTest                               PASS           0.00      **
** RandomTest                                   PASS         150.00      **
** MaxTest                                      PASS         150.00      **
******************************************************************************
REGRESSION: PASS
```

`[HelloWorldTest]` is the component path (D49): rustdv names the root after
the test you registered, where UVM always says `uvm_test_top`.

## Preserve these teaching points

Manuscript: [chapter-23-uvm-test-testbench-3.0](../../../book-pdf/src/chapter-23-uvm-test-testbench-3.0.md).

- Testbench 3.0 has the test as its only component; the BFM and scoreboard are
  ordinary locals. The component tree belongs to Chapter 24.
- Both test forms are first-class: an async function and a component/type
  annotated with #[rustdv::test]. Neither is temporary training code.
- Re-show evolving Tester/RandomTester/MaxTester/Scoreboard definitions rather
  than importing an old chapter. Explain that re-showing makes the changes
  visible.
- Test registrations retain their Rust type names, and log roots are named after
  the test rather than a fixed uvm_test_top. This chapter is the Part II prose
  exemplar.

Read this crate’s source and this README before changing a quoted
figure or behavioral claim; keep the source, figure map, and transcript aligned.
