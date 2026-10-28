# Chapter 36: Sequence Testbench: 7.0 — figure map

Run with:

```
sim-common/run_sim.sh ch36_sequence_testbench_7_0 tinyalu \
    sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
```

Figure numbers are the **book's** (D110: one numbering space per chapter, code
and transcripts drawn from the same sequence), and the `.rs` captions carry the
same numbers. Figure 1 is the book's SVG handshake drawing, which is why the
crate's first caption is Figure 2.

| Figure | Title | Where |
|---|---|---|
| 1 | The sequencer handshake | book SVG drawing — no code |
| 2 | The driver pulls items instead of being pushed them | `src/ch36_sequence_testbench_7_0.rs` (`Driver`) |
| 3 | One body, three stimulus patterns | `src/ch36_sequence_testbench_7_0.rs` (the `Sequence` impl) |
| 4 | The base sequence sends zeros | `src/ch36_sequence_testbench_7_0.rs` (`BaseSeq`) |
| 5 | Random and maximum operands | `src/ch36_sequence_testbench_7_0.rs` (`RandomSeq`, `MaxSeq`) |
| 6 | The env owns the sequencer and files its handle | `src/ch36_sequence_testbench_7_0.rs` (`AluEnv`) |
| 7 | The test starts a sequence on the sequencer | `src/ch36_sequence_testbench_7_0.rs` (`BaseTest`) |
| 8 | Two more tests, one testbench, no new components | `src/ch36_sequence_testbench_7_0.rs` (`RandomTest`, `MaxTest`) |
| 9 | Testbench 7.0 running | transcript — all three tests |

`AluCommand` and `AluResult` are re-shown near the top of the file without
captions; they are unchanged from earlier chapters.

Three tests, all ending `REGRESSION: PASS`.

## Transcript

**Figure 9.** Verbatim from `sim-common/run_sim.sh ch36_sequence_testbench_7_0
tinyalu sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv`,
`RUSTDV_RANDOM_SEED=1`.

Three tests, one testbench, one set of components. Only the sequence changes —
and the operands with it.

```
      0.00ns INFO     rustdv: found 3 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running BaseTest (1/3)  [ch36-sequence-testbench-7.0/src/ch36_sequence_testbench_7_0.rs:385]
    280.00ns INFO     [BaseTest.env.scoreboard]: PASSED: 00 Add 00 = 0000
    280.00ns INFO     [BaseTest.env.scoreboard]: PASSED: 00 And 00 = 0000
    280.00ns INFO     [BaseTest.env.scoreboard]: PASSED: 00 Xor 00 = 0000
    280.00ns INFO     [BaseTest.env.scoreboard]: PASSED: 00 Mul 00 = 0000
    280.00ns INFO     [BaseTest.env.scoreboard]: Covered all operations
    280.00ns INFO     BaseTest PASSED
    280.00ns INFO     running RandomTest (2/3)  [ch36-sequence-testbench-7.0/src/ch36_sequence_testbench_7_0.rs:426]
    560.00ns INFO     [RandomTest.inner.env.scoreboard]: PASSED: c1 Add 67 = 0128
    560.00ns INFO     [RandomTest.inner.env.scoreboard]: PASSED: 5e And 0b = 000a
    560.00ns INFO     [RandomTest.inner.env.scoreboard]: PASSED: b9 Xor 80 = 0039
    560.00ns INFO     [RandomTest.inner.env.scoreboard]: PASSED: a5 Mul 75 = 4b69
    560.00ns INFO     [RandomTest.inner.env.scoreboard]: Covered all operations
    560.00ns INFO     RandomTest PASSED
    560.00ns INFO     running MaxTest (3/3)  [ch36-sequence-testbench-7.0/src/ch36_sequence_testbench_7_0.rs:440]
    840.00ns INFO     [MaxTest.inner.env.scoreboard]: PASSED: ff Add ff = 01fe
    840.00ns INFO     [MaxTest.inner.env.scoreboard]: PASSED: ff And ff = 00ff
    840.00ns INFO     [MaxTest.inner.env.scoreboard]: PASSED: ff Xor ff = 0000
    840.00ns INFO     [MaxTest.inner.env.scoreboard]: PASSED: ff Mul ff = fe01
    840.00ns INFO     [MaxTest.inner.env.scoreboard]: Covered all operations
    840.00ns INFO     MaxTest PASSED
******************************************************************************
** TEST                                       STATUS  SIM TIME (ns)      **
******************************************************************************
** BaseTest                                     PASS         280.00      **
** RandomTest                                   PASS         280.00      **
** MaxTest                                      PASS         280.00      **
******************************************************************************
REGRESSION: PASS
```

Worth noticing in the log: `BaseTest`'s components sit at `BaseTest.env.*`
while the other two sit at `RandomTest.inner.env.*` — the two later tests wrap
the env in an inner component so the sequence can be swapped by the factory.

## What this chapter proves

- **The structure holds still and the program changes.** Three tests, one set
  of components, no new ones. The only difference between them is which
  sequence gets started.
- **The sequencer is a component; the sequence is not.** The sequencer has a
  place in the tree and a path. A sequence has neither — it has a `body`.
- **Two calls, not one.** `start_item` returns once the sequencer has granted
  this item its turn and the driver is blocked waiting for its contents, so
  everything between `start_item` and `finish_item` happens with the driver
  committed. A single `send(cmd)` could not express that window.

## Preserve these teaching points

Manuscript: [chapter-36-sequence-testbench-7.0](../../../book-pdf/src/chapter-36-sequence-testbench-7.0.md).

- The structure stays fixed while a sequence changes the stimulus program. A
  sequence has no component path/phases; the sequencer is a component whose handle
  can be filed in ConfigDb.
- Teach why start_item and finish_item are separate: after a grant, the driver
  waits while stimulus fields can still be set. A single send call would remove
  that late-setting window.
- Finish_item takes the request by value. Passing cmd and cmd.clone() are both
  legal choices, depending on whether the sequence needs its own copy afterward.
- Keep the connection back to Chapter 31’s concurrent pipeline, the handshake
  drawing, and the source-backed transcript.

Read this crate’s source and this README before changing a quoted
figure or behavioral claim; keep the source, figure map, and transcript aligned.
