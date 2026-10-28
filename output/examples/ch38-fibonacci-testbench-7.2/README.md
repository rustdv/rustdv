# Chapter 38: Fibonacci Testbench: 7.2 — figure map

Run with:

```
sim-common/run_sim.sh ch38_fibonacci_testbench_7_2 tinyalu \
    sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
```

Figure numbers are the **book's** (D110: one numbering space per chapter, code
and transcripts drawn from the same sequence); the `.rs` captions carry the same
numbers, and the transcript is appended as Figure 6.

| Figure | Title | Where |
|---|---|---|
| 1 | The driver sends a command and returns its result | `src/ch38_fibonacci_testbench_7_2.rs` (`Driver`) |
| 2 | Nine numbers, eight of them from the DUT | `src/ch38_fibonacci_testbench_7_2.rs` (`FibonacciSeq`) |
| 3 | No result monitor | `src/ch38_fibonacci_testbench_7_2.rs` (`FibEnv`) |
| 4 | A subscriber checks the DUT actually added | `src/ch38_fibonacci_testbench_7_2.rs` (`SeenResults`, `ResultWatcher`) |
| 5 | The test | `src/ch38_fibonacci_testbench_7_2.rs` (`FibonacciTest`) |
| 6 | The TinyALU computes Fibonacci numbers | transcript — `FibonacciTest` |

`AluCommand` and `AluResult` are re-shown near the top of the file without
captions; they are unchanged from earlier chapters.

One test, ending `REGRESSION: PASS`.

## Transcript

**Figure 6.** Verbatim from `sim-common/run_sim.sh ch38_fibonacci_testbench_7_2
tinyalu sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv`,
`RUSTDV_RANDOM_SEED=1`.

The sequence prints nine numbers; eight of them it did not know until the DUT
answered. The watcher's line is the independent check that the additions really
happened in hardware.

```
      0.00ns INFO     rustdv: found 1 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running FibonacciTest (1/1)  [ch38-fibonacci-testbench-7.2/src/ch38_fibonacci_testbench_7_2.rs:228]
    170.00ns INFO     [FibonacciSeq]: Fibonacci Sequence: [0, 1, 1, 2, 3, 5, 8, 13, 21]
    170.00ns INFO     [FibonacciTest.env.watcher]: adder produced [1, 2, 3, 5, 8, 13, 21]
    170.00ns INFO     FibonacciTest PASSED
******************************************************************************
** TEST                                       STATUS  SIM TIME (ns)      **
******************************************************************************
** FibonacciTest                                PASS         170.00      **
******************************************************************************
REGRESSION: PASS
```

## What this chapter proves

- **Stimulus that needs the DUT's answers.** Each command's operands are the
  previous two results, so the sequence cannot be written ahead of time. This
  is what `get_response` is for.
- **With one command in flight, the id is documentation rather than
  necessity.** Chapter 37 already showed the case where a ticket is
  unavoidable; here it is honest to say the id is not doing load-bearing work,
  and to say why it is still written.
- **Nothing travels backwards.** The driver answers through the response. Both
  source books write the result into the sequence item the sequence still
  holds — that is not a capability, it is what handles look like when two names
  point at one object. Rust has one owner, so there is nothing to restore and
  nothing missing.

## Preserve these teaching points

Manuscript: [chapter-38-fibonacci-testbench-7.2](../../../book-pdf/src/chapter-38-fibonacci-testbench-7.2.md).

- Fibonacci runs on the TinyALU and needs each previous result before issuing the
  next operation. The driver supplies responses through the sequencer; it does not
  mutate an item still owned by the sequence.
- With one outstanding operation, get_response(Some(ticket)) documents the
  relationship rather than proving tickets necessary; Chapter 37 provides the
  multiple-outstanding case.
- Preserve the response-bearing driver, the absence of a separate result monitor,
  and the explanation of why no additional pipeline flush is needed.

Read this crate’s source and this README before changing a quoted
figure or behavioral claim; keep the source, figure map, and transcript aligned.
