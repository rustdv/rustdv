# Chapter 39: Virtual Sequence Testbench: 8.0 — figure map

Run with:

```
sim-common/run_sim.sh ch39_virtual_sequence_testbench_8_0 tinyalu \
    sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
```

Figure numbers are the **book's** (D110: one numbering space per chapter, code
and transcripts drawn from the same sequence); the `.rs` captions carry the same
numbers. The book's presentation reorders the crate, so the captions do not run
in file order — Figure 2 is near the bottom of the file.

| Figure | Title | Where |
|---|---|---|
| 1 | A virtual sequence starts other sequences | `src/ch39_virtual_sequence_testbench_8_0.rs` (`TestAllSeq`) |
| 2 | A test that starts a virtual sequence — no sequencer | `src/ch39_virtual_sequence_testbench_8_0.rs` (`AluTest`) |
| 3 | Running RandomSeq, then MaxSeq | transcript — `AluTest` |
| 4 | The same two sequences, at the same time | `src/ch39_virtual_sequence_testbench_8_0.rs` (`TestAllParallelSeq`) |
| 5 | The two sequences interleave at the sequencer | transcript — `ParallelTest` |
| 6 | One operation, as a sequence | `src/ch39_virtual_sequence_testbench_8_0.rs` (`OpSeq`) |
| 7 | The TinyALU programming interface | `src/ch39_virtual_sequence_testbench_8_0.rs` (`do_add` and friends) |
| 8 | Fibonacci, written as a program | `src/ch39_virtual_sequence_testbench_8_0.rs` (`FibonacciProgramSeq`) |
| 9 | The program runs | transcript — `FibonacciProgramTest` |

`ParallelTest` and `FibonacciProgramTest` are uncaptioned in the crate and
unlisted in the book; their output is Figures 5 and 9. `AluCommand` and
`AluResult` are re-shown without captions.

Three tests, all ending `REGRESSION: PASS`.

## Transcripts

All three are one run of the command above, `RUSTDV_RANDOM_SEED=1`, split by
test.

**Figure 3 — `AluTest`.** The virtual sequence runs `RandomSeq`, then `MaxSeq`.
Eight commands, four random and four maximum, in that order.

```
      0.00ns INFO     rustdv: found 3 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running AluTest (1/3)  [ch39-virtual-sequence-testbench-8.0/src/ch39_virtual_sequence_testbench_8_0.rs:440]
    250.00ns INFO     [TestAllSeq]: ran random, then max
    250.00ns INFO     [AluTest.env.scoreboard]: PASSED: c1 Add 67 = 0128
    250.00ns INFO     [AluTest.env.scoreboard]: PASSED: 5e And 0b = 000a
    250.00ns INFO     [AluTest.env.scoreboard]: PASSED: b9 Xor 80 = 0039
    250.00ns INFO     [AluTest.env.scoreboard]: PASSED: a5 Mul 75 = 4b69
    250.00ns INFO     [AluTest.env.scoreboard]: PASSED: ff Add ff = 01fe
    250.00ns INFO     [AluTest.env.scoreboard]: PASSED: ff And ff = 00ff
    250.00ns INFO     [AluTest.env.scoreboard]: PASSED: ff Xor ff = 0000
    250.00ns INFO     [AluTest.env.scoreboard]: PASSED: ff Mul ff = fe01
    250.00ns INFO     [AluTest.env.scoreboard]: Covered all operations
    250.00ns INFO     AluTest PASSED
```

**Figure 5 — `ParallelTest`.** The same two sequences, started together. The
operands are identical to Figure 3; only the **order** differs, because the
sequencer interleaves the two sequences' items one for one — random Add, max
Add, random And, max And, and so on.

```
    250.00ns INFO     running ParallelTest (2/3)  [ch39-virtual-sequence-testbench-8.0/src/ch39_virtual_sequence_testbench_8_0.rs:461]
    500.00ns INFO     [TestAllParallelSeq]: ran random and max together
    500.00ns INFO     [ParallelTest.env.scoreboard]: PASSED: c1 Add 67 = 0128
    500.00ns INFO     [ParallelTest.env.scoreboard]: PASSED: ff Add ff = 01fe
    500.00ns INFO     [ParallelTest.env.scoreboard]: PASSED: 5e And 0b = 000a
    500.00ns INFO     [ParallelTest.env.scoreboard]: PASSED: ff And ff = 00ff
    500.00ns INFO     [ParallelTest.env.scoreboard]: PASSED: b9 Xor 80 = 0039
    500.00ns INFO     [ParallelTest.env.scoreboard]: PASSED: ff Xor ff = 0000
    500.00ns INFO     [ParallelTest.env.scoreboard]: PASSED: a5 Mul 75 = 4b69
    500.00ns INFO     [ParallelTest.env.scoreboard]: PASSED: ff Mul ff = fe01
    500.00ns INFO     [ParallelTest.env.scoreboard]: Covered all operations
    500.00ns INFO     ParallelTest PASSED
```

**Figure 9 — `FibonacciProgramTest`.** The testbench as a programming
interface: `do_add(&seqr, a, b)` returns a number, and nine of them make a
Fibonacci sequence. Coverage is deliberately not required here — the program
only ever adds.

```
    500.00ns INFO     running FibonacciProgramTest (3/3)  [ch39-virtual-sequence-testbench-8.0/src/ch39_virtual_sequence_testbench_8_0.rs:482]
    670.00ns INFO     [FibonacciProgramSeq]: Fibonacci Sequence: [0, 1, 1, 2, 3, 5, 8, 13, 21]
    670.00ns INFO     [FibonacciProgramTest.env.scoreboard]: PASSED: 00 Add 01 = 0001
    670.00ns INFO     [FibonacciProgramTest.env.scoreboard]: PASSED: 01 Add 01 = 0002
    670.00ns INFO     [FibonacciProgramTest.env.scoreboard]: PASSED: 01 Add 02 = 0003
    670.00ns INFO     [FibonacciProgramTest.env.scoreboard]: PASSED: 02 Add 03 = 0005
    670.00ns INFO     [FibonacciProgramTest.env.scoreboard]: PASSED: 03 Add 05 = 0008
    670.00ns INFO     [FibonacciProgramTest.env.scoreboard]: PASSED: 05 Add 08 = 000d
    670.00ns INFO     [FibonacciProgramTest.env.scoreboard]: saw 1 of 4 ops (coverage not required)
    670.00ns INFO     FibonacciProgramTest PASSED
******************************************************************************
** TEST                                       STATUS  SIM TIME (ns)      **
******************************************************************************
** AluTest                                      PASS         250.00      **
** ParallelTest                                 PASS         250.00      **
** FibonacciProgramTest                         PASS         170.00      **
******************************************************************************
REGRESSION: PASS
```

## What this chapter proves

- **A virtual sequence is started without a sequencer.** It sends no items of
  its own; it starts other sequences. `start_virtual()` exists rather than
  `start(None)` because Rust has no default arguments and `None` does not say
  "virtual".
- **Figures 3 and 5 differ only in order.** Same operands, same results,
  different interleaving — which is the clearest evidence that the sequencer,
  not the sequence, decides who gets the driver next.
- **The testbench becomes a programming interface.** `OpSeq` plus four
  functions turn stimulus into `do_add(&seqr, a, b) -> u16`.
- **Deliberately absent: a separate `VirtualSequence` trait.** It would make
  `start_item` inside a virtual sequence a compile error instead of a run-time
  one, and would forbid a shape the UVM allows. A better error message, at the
  cost of a capability.

## Preserve these teaching points

Manuscript: [chapter-39-virtual-sequence-testbench-8.0](../../../book-pdf/src/chapter-39-virtual-sequence-testbench-8.0.md).

- A virtual sequence starts other sequences and sends no items of its own;
  start_virtual is the explicit no-sequencer entry point. Preserve both sequential
  and parallel composition.
- Do not add a separate VirtualSequence trait that forbids shapes the UVM permits.
  Explain restrictions and errors at the point the source actually enforces them.
- Keep OpSeq and the operation helpers as a programming interface, including the
  Fibonacci composition. The chapter is more than another stimulus override.
- Preserve the deliberate presentation order of tests/helpers and the existing
  uncaptioned listings; check the current README instead of replaying historical
  caption shifts.

Read this crate’s source and this README before changing a quoted
figure or behavioral claim; keep the source, figure map, and transcript aligned.
