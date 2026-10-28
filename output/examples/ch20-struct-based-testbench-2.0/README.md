# Chapter 20: Struct-Based Testbench: 2.0 — figure map

Run with:

```
sim-common/run_sim.sh ch20_struct_based_testbench_2_0 tinyalu \
    sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
```

| Figure | Title | Where |
|---|---|---|
| 1 | Tester structure | text diagram |
| 2 | Common behavior across all testers | `src/ch20_struct_based_testbench_2_0.rs` (`Tester`) |
| 3 | RandomTester overrides get_operands() | `src/ch20_struct_based_testbench_2_0.rs` |
| 4 | MaxTester overrides get_operands() | `src/ch20_struct_based_testbench_2_0.rs` |
| 5 | Initializing the Scoreboard | `src/ch20_struct_based_testbench_2_0.rs` |
| 6 | The Scoreboard's data-gathering tasks | `src/ch20_struct_based_testbench_2_0.rs` (`start_tasks`) |
| 7 | The check_results() phase | `src/ch20_struct_based_testbench_2_0.rs` |
| 8 | The Scoreboard checks functional coverage | inside `check_results` |
| 9 | The execute_test coroutine starts the tasks | `src/ch20_struct_based_testbench_2_0.rs` |
| 10 | Execute the tester | inside `execute_test` |
| 11 | The tests launch execute_test with a tester | `src/ch20_struct_based_testbench_2_0.rs` (`random_test`) |
| 12 | The max test differs only in its tester | `src/ch20_struct_based_testbench_2_0.rs` (`max_test`) |
| 13 | Two tests, one testbench | transcript below |

Transcript (RUSTDV_RANDOM_SEED=1):

```
      0.00ns INFO     rustdv: found 2 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running random_test (1/2)  [ch20-struct-based-testbench-2.0/src/ch20_struct_based_testbench_2_0.rs:144]
    150.00ns INFO     PASSED: c1 Add 67 = 0128
    150.00ns INFO     PASSED: 5e And 0b = 000a
    150.00ns INFO     PASSED: b9 Xor 80 = 0039
    150.00ns INFO     PASSED: a5 Mul 75 = 4b69
    150.00ns INFO     Covered all operations
    150.00ns INFO     random_test PASSED
    150.00ns INFO     running max_test (2/2)  [ch20-struct-based-testbench-2.0/src/ch20_struct_based_testbench_2_0.rs:157]
    300.00ns INFO     PASSED: ff Add ff = 01fe
    300.00ns INFO     PASSED: ff And ff = 00ff
    300.00ns INFO     PASSED: ff Xor ff = 0000
    300.00ns INFO     PASSED: ff Mul ff = fe01
    300.00ns INFO     Covered all operations
    300.00ns INFO     max_test PASSED
******************************************************************************
** TEST                                       STATUS  SIM TIME (ns)      **
******************************************************************************
** random_test                                  PASS         150.00      **
** max_test                                     PASS         150.00      **
******************************************************************************
REGRESSION: PASS
```

## Preserve these teaching points

Manuscript: [chapter-20-struct-based-testbench-2.0](../../../book-pdf/src/chapter-20-struct-based-testbench-2.0.md).

- Testbench 2.0 is a useful component-free destination, not disposable scaffolding
  for the UVM chapters. A reader should be able to build a complete testbench
  using handles, a BFM, ordinary structs/traits, and tasks.
- The Tester trait and its provided execute method recur in later testbench
  versions; preserve the reader’s recognition of that structure.

Read this crate’s source and this README before changing a quoted
figure or behavioral claim; keep the source, figure map, and transcript aligned.
