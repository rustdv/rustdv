# Chapter 19: TinyAluBfm — figure map

The BFM (figures 2–13) lives in the shared crate `../tinyalu-utils/src/ch19_tinyalubfm.rs`;
the test (figures 14–18) is in `src/ch19_tinyalubfm.rs` here. Run with:

```
sim-common/run_sim.sh ch19_tinyalubfm tinyalu \
    sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
```

| Figure | Title | Where |
|---|---|---|
| 1 | Every BFM loop lives on the falling edge | skeleton (fragment; the pattern of figs 5–7) |
| 2 | The TinyAluBfm struct — one owner of the pins | `tinyalu-utils/src/ch19_tinyalubfm.rs` |
| 3 | Initializing the TinyAluBfm | `tinyalu-utils/src/ch19_tinyalubfm.rs` |
| 4 | Centralizing the reset function | `tinyalu-utils/src/ch19_tinyalubfm.rs` |
| 5 | Monitoring the result bus | `tinyalu-utils/src/ch19_tinyalubfm.rs` (`result_mon`) |
| 6 | Monitoring the command signals | `tinyalu-utils/src/ch19_tinyalubfm.rs` (`cmd_mon`) |
| 7 | Driving commands on the falling edge of clk | `tinyalu-utils/src/ch19_tinyalubfm.rs` (`cmd_driver`) |
| 8 | Drive a command when the bus is idle | inside `cmd_driver` |
| 9 | If start is 1 check done | inside `cmd_driver` |
| 10 | Start the BFM tasks | `tinyalu-utils/src/ch19_tinyalubfm.rs` (`start_tasks`) |
| 11 | The get_cmd() coroutine returns the next command | `tinyalu-utils/src/ch19_tinyalubfm.rs` |
| 12 | The get_result() coroutine returns the next result | `tinyalu-utils/src/ch19_tinyalubfm.rs` |
| 13 | send_op puts the command into the command Queue | `tinyalu-utils/src/ch19_tinyalubfm.rs` |
| 14 | Starting a test by resetting the DUT and starting the BFM tasks | `src/ch19_tinyalubfm.rs` (`test_alu`) |
| 15 | Creating a command and sending it | `src/ch19_tinyalubfm.rs` |
| 16 | Wait to get the command from the DUT and store it in the coverage set | `src/ch19_tinyalubfm.rs` |
| 17 | Wait for the result, then create a prediction | `src/ch19_tinyalubfm.rs` |
| 18 | Check the result against the predicted result | `src/ch19_tinyalubfm.rs` |
| 19 | Another successful test | transcript below |

Transcript (RUSTDV_RANDOM_SEED=1) — same operands as Chapter 18, on purpose:

```
      0.00ns INFO     rustdv: found 1 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running test_alu (1/1)  [ch19-tinyalubfm/src/ch19_tinyalubfm.rs:19]
     50.00ns INFO     PASSED: c1 Add 67 = 0128
     70.00ns INFO     PASSED: 5e And 0b = 000a
     90.00ns INFO     PASSED: b9 Xor 80 = 0039
    140.00ns INFO     PASSED: a5 Mul 75 = 4b69
    140.00ns INFO     Covered all operations
    140.00ns INFO     test_alu PASSED
******************************************************************************
** TEST                                       STATUS  SIM TIME (ns)      **
******************************************************************************
** test_alu                                     PASS         140.00      **
******************************************************************************
REGRESSION: PASS
```

## Preserve these teaching points

Manuscript: [chapter-19-tinyalubfm](../../../book-pdf/src/chapter-19-tinyalubfm.md).

- The BFM waits on RTL clock edges; it does not instantiate Clock for the self-
  clocked TinyALU. Edge waiting is the interface that can also be supplied by an
  emulator/transactor.
- Keep the ownership and lifetime explanation aligned with the actual queue-fed
  tasks and their capture behavior. Changes to timing or file/line layout require
  synchronized README/manuscript transcripts.

Read this crate’s source and this README before changing a quoted
figure or behavioral claim; keep the source, figure map, and transcript aligned.
