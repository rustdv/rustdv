# Chapter 34: Connections — Testbench 6.0 — figure map

Run with:

```
sim-common/run_sim.sh ch34_connections_testbench_6_0 tinyalu \
    sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
```

**This crate carries two chapters (D91).** Chapter 33 defines the six
components and has nothing to run; Chapter 34 wires them up and runs. The
captions in the source carry the chapter number, so both live here without
colliding. Chapter 33's figure map is in
[`../ch33-components-testbench-6.0/README.md`](../ch33-components-testbench-6.0/README.md).

Figure numbers are the **book's** (D110: one numbering space per chapter, code
and transcripts drawn from the same sequence); the `.rs` captions carry the same
numbers. Figure 1 is the book's SVG architecture drawing, which is why the
crate's first Chapter 34 caption is Figure 2.

| Figure | Title | Where |
|---|---|---|
| 1 | The 6.0 architecture | book SVG drawing — no code |
| 2 | Build the components and the FIFOs; connect in one place | `src/ch34_connections_testbench_6_0.rs` (`AluEnv`) |
| 3 | The test is just the env | `src/ch34_connections_testbench_6_0.rs` (`AluTest`) |
| 4 | Testbench 6.0 running | transcript — `AluTest` |

One test, ending `REGRESSION: PASS`. The transcript below is the whole
testbench — Chapter 33's components included — since they only run once
Chapter 34 has connected them.

## Architecture

```
  Tester --put--> [cmd_fifo] --get--> Driver --> BFM --> DUT

  CmdMonitor --pub--> [cmd_bus] --sub--> Scoreboard
                           \----sub----> Coverage

  ResultMonitor --pub--> [result_bus] --sub--> Scoreboard
```

Every connection is the same shape — a concrete FIFO, a named export,
`connect(component, PORT_NAME)` — whether the traffic is point-to-point
(`TlmFifo`) or broadcast (`AnalysisBus`).

## The Rust win worth noting

The Scoreboard needs **two** analysis streams. SystemVerilog cannot give one
class two `write` methods, so it needs the `uvm_analysis_imp_decl` macros;
pyuvm cannot express it at all with one `write` per class. Here each stream gets
its own `SubscribePort` and its own sink struct — and it works the same way when
both streams carry the same type, which is the case the SV macros exist for
(D20/D88).

## Transcript

Real Icarus output (`RUSTDV_RANDOM_SEED=1`):

```
      0.00ns INFO     rustdv: found 1 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running AluTest (1/1)  [ch34-connections-testbench-6.0/src/ch34_connections_testbench_6_0.rs:346]
    240.00ns INFO     [AluTest.env.scoreboard]: PASSED: c1 Add 67 = 0128
    240.00ns INFO     [AluTest.env.scoreboard]: PASSED: 5e And 0b = 000a
    240.00ns INFO     [AluTest.env.scoreboard]: PASSED: b9 Xor 80 = 0039
    240.00ns INFO     [AluTest.env.scoreboard]: PASSED: a5 Mul 75 = 4b69
    240.00ns INFO     [AluTest.env.scoreboard]: Covered all operations
    240.00ns INFO     [AluTest.env.coverage]: coverage saw 4 of 4 ops
    240.00ns INFO     AluTest PASSED
******************************************************************************
** TEST                                       STATUS  SIM TIME (ns)      **
******************************************************************************
** AluTest                                      PASS         240.00      **
******************************************************************************
REGRESSION: PASS
```

## Why the Tester waits before dropping its objection

`put` returns when the FIFO accepts a command, not when the DUT has answered it.
Dropping the objection at that point ends the run phase with commands still in
the pipeline, and the scoreboard checks fewer results than it saw commands — it
does not fail, it silently checks less. The Python testbench waits ten clocks
after its last put; this one waits twenty, because the multiply is the last
operation and takes the longest to come back.

This chapter is what caught D82c: before it, the objection race dropped the whole
run tree, taking the components with it, and `check` never ran at all.

## Preserve these teaching points

Manuscript: [chapter-33-components-testbench-6.0](../../../book-pdf/src/chapter-33-components-testbench-6.0.md), [chapter-34-connections-testbench-6.0](../../../book-pdf/src/chapter-34-connections-testbench-6.0.md).

- This crate holds both Chapter 33 component definitions and Chapter 34
  environment/test wiring. Preserve separate Chapter 33 and Chapter 34 source
  captions and README figure maps.
- Chapter 33 teaches the single-purpose Tester, Driver, monitors, Scoreboard, and
  Coverage definitions; Chapter 34 builds and connects them. Do not duplicate the
  environment in the definitions chapter.
- The scoreboard has two SubscribePorts and two Subscriber implementations.
  Preserve each subscriber’s own storage and connect through AnalysisBus rather
  than restoring an analysis FIFO API.
- The chapter’s Tester retains its objection for the source-defined flush after
  the last put (currently twenty clocks); the slow multiply is still in flight. Do
  not shorten the wait or confuse it with the shipped TinyALU BFM’s wait_idle. A
  premature end can silently under-check results.

Read this crate’s source and this README before changing a quoted
figure or behavioral claim; keep the source, figure map, and transcript aligned.
