# Chapter 26: Logging — figure map

Run with:

```
sim-common/run_sim.sh ch26_logging playground
```

No DUT: the subject is what a component *says*, not what it drives.

| Figure | Title | Where |
|---|---|---|
| 1 | Logging messages of all levels | `src/ch26_logging.rs` (`LogComp`) |
| 2 | The logging policy is the only thing that varies | `LogPolicy`, `LogTest<P>` |
| 3 | The default — no configuration at all | `DefaultLogging` |
| 4 | Setting the logging level for a hierarchy | `DebugLogging` |
| 5 | Writing log entries to a file | `FileLogging` |
| 6 | Disabling logging for a hierarchy | `NoLogging` |
| 7 | The four tests are type aliases over one base | `LogTestDefault`, `DebugTest`, `FileTest`, `NoLog` |

Port of the Python book's chapter 30.

## Transcript (seed 1)

```
      0.00ns INFO     rustdv: found 4 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running LogTestDefault (1/4)  [ch26-logging/src/ch26_logging.rs:124]
      0.00ns INFO     [LogTestDefault.comp]: This is info
      0.00ns WARNING  [LogTestDefault.comp]: This is warning
      0.00ns ERROR    [LogTestDefault.comp]: This is error
      0.00ns CRITICAL [LogTestDefault.comp]: This is critical
      0.00ns INFO     LogTestDefault PASSED
      0.00ns INFO     running DebugTest (2/4)  [ch26-logging/src/ch26_logging.rs:127]
      0.00ns DEBUG    [DebugTest.comp]: This is debug
      0.00ns INFO     [DebugTest.comp]: This is info
      0.00ns WARNING  [DebugTest.comp]: This is warning
      0.00ns ERROR    [DebugTest.comp]: This is error
      0.00ns CRITICAL [DebugTest.comp]: This is critical
      0.00ns INFO     DebugTest PASSED
      0.00ns INFO     running FileTest (3/4)  [ch26-logging/src/ch26_logging.rs:130]
      0.00ns INFO     FileTest PASSED
      0.00ns INFO     running NoLog (4/4)  [ch26-logging/src/ch26_logging.rs:133]
      0.00ns INFO     NoLog PASSED
******************************************************************************
** TEST                                       STATUS  SIM TIME (ns)      **
******************************************************************************
** LogTestDefault                               PASS           0.00      **
** DebugTest                                    PASS           0.00      **
** FileTest                                     PASS           0.00      **
** NoLog                                        PASS           0.00      **
******************************************************************************
REGRESSION: PASS
```

`FileTest` prints nothing — its subtree was taken off the console — but
`rustdv_ch26_log.txt`, written beside wherever you ran the simulation,
receives it:

```
      0.00ns INFO     [FileTest.comp]: This is info
      0.00ns WARNING  [FileTest.comp]: This is warning
      0.00ns ERROR    [FileTest.comp]: This is error
      0.00ns CRITICAL [FileTest.comp]: This is critical
```

## What the figures do not contain

**A path.** `LogComp` is one type, written once, and it logs as
`[LogTestDefault.comp]`, `[DebugTest.comp]`, `[FileTest.comp]` — whichever
is true for the test it was built under. Nothing in the component says so.
`ctx.info(..)` is attributed to the caller, and
`ctx.set_logging_level_hier(..)` addresses the caller's subtree, because the
context carries the path the phase walk derived (D7).

The alternative is what this chapter used to do:
`Logger::new("uvm_test_top.comp")` and `set_level_for("uvm_test_top", ..)`,
typed by hand. Those strings keep compiling — and keep lying — after a
component is renamed or moved.

**A cleanup step.** The old version restored the log level manually so the
next test wasn't affected. The runner now resets logging configuration
between tests, as pyuvm's `run_test` does. The evidence is in the log file:
`DebugTest` ran immediately before `FileTest` and set the level to Debug,
yet the file has no `This is debug` line.

**Inheritance.** Python writes `class DebugTest(LogTest)` three times, each
overriding `end_of_elaboration_phase`. Here the policy is a type parameter
and the four tests are type aliases over one base (D28) — the same move as
the testers in Chapter 25.

## Preserve these teaching points

Manuscript: [chapter-26-logging](../../../book-pdf/src/chapter-26-logging.md).

- Logging flows through the component context and hierarchy; components do not
  store their own path merely for reporting.
- Keep log-level and log-file examples synchronized with source and output,
  including the runner resetting logging state between tests.

Read this crate’s source and this README before changing a quoted
figure or behavioral claim; keep the source, figure map, and transcript aligned.
