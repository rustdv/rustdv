# Chapter 27: Configuration — figure map

Run with:

```
sim-common/run_sim.sh ch27_configuration playground
```

Figure numbers are the **book's** (D110: one numbering space per chapter, code
and transcripts drawn from the same sequence). The `.rs` captions carry the same
numbers, which is why this map has no gaps but the crate's captions do.

| Figure | Title | Where |
|---|---|---|
| 1 | Logging a message we get from the ConfigDb | `src/ch27_configuration.rs` (`MsgLogger`) |
| 2 | Two loggers in the environment | `src/ch27_configuration.rs` (`MsgEnv`) |
| 3 | Giving loga and logb different messages | `src/ch27_configuration.rs` (`MsgTest`) |
| 4 | Each logger says its own thing | transcript — `MsgTest` |
| 5 | Adding talka and talkb to the environment | `src/ch27_configuration.rs` (`MultiMsgEnv`) |
| 6 | Using a wildcard to configure both talkers at once | `src/ch27_configuration.rs` (`MultiMsgTest`) |
| 7 | One `set` reaches both talkers | transcript — `MultiMsgTest` |
| 8 | Adding gtalk, which nobody configures by name | `src/ch27_configuration.rs` (`GlobalEnv`) |
| 9 | Storing a message for everybody | `src/ch27_configuration.rs` (`GlobalTest`) |
| 10 | The unnamed component gets the global message | transcript — `GlobalTest` |
| 11 | The env configures its own child | `src/ch27_configuration.rs` (`ConflictEnv`) |
| 12 | The test configures the same component by a longer path | `src/ch27_configuration.rs` (`ConflictTest`) |
| 13 | The parent wins | transcript — `ConflictTest` |

Four tests, all ending `REGRESSION: PASS`.

## Transcripts

Verbatim from `sim-common/run_sim.sh ch27_configuration playground`,
`RUSTDV_RANDOM_SEED=1`.

**Figure 4 — `MsgTest`.** Two loggers, two different messages, each found by its
own path.

```
      0.00ns INFO     running MsgTest (1/4)  [ch27-configuration/src/ch27_configuration.rs:99]
      0.00ns INFO     [MsgTest.env.loga]: LOG A msg
      0.00ns INFO     [MsgTest.env.logb]: LOG B msg
      0.00ns INFO     MsgTest PASSED
```

**Figure 7 — `MultiMsgTest`.** One wildcard `set` configures both talkers; the
two loggers still have their own messages.

```
      0.00ns INFO     running MultiMsgTest (2/4)  [ch27-configuration/src/ch27_configuration.rs:150]
      0.00ns INFO     [MultiMsgTest.env.loga]: LOG A msg
      0.00ns INFO     [MultiMsgTest.env.logb]: LOG B msg
      0.00ns INFO     [MultiMsgTest.env.talka]: TALK TALK
      0.00ns INFO     [MultiMsgTest.env.talkb]: TALK TALK
      0.00ns INFO     MultiMsgTest PASSED
```

**Figure 10 — `GlobalTest`.** `gtalk` is configured by nobody in particular and
picks up the global message.

```
      0.00ns INFO     running GlobalTest (3/4)  [ch27-configuration/src/ch27_configuration.rs:207]
      0.00ns INFO     [GlobalTest.env.loga]: LOG A msg
      0.00ns INFO     [GlobalTest.env.logb]: LOG B msg
      0.00ns INFO     [GlobalTest.env.talka]: TALK TALK
      0.00ns INFO     [GlobalTest.env.talkb]: TALK TALK
      0.00ns INFO     [GlobalTest.env.gtalk]: GLOBAL
      0.00ns INFO     GlobalTest PASSED
```

**Figure 13 — `ConflictTest`.** Two `set` calls address the same component by
different paths. The parent's wins.

```
      0.00ns INFO     running ConflictTest (4/4)  [ch27-configuration/src/ch27_configuration.rs:256]
      0.00ns INFO     [ConflictTest.env.loga]: PARENT RULES!
      0.00ns INFO     ConflictTest PASSED
```

## What this chapter proves

- **Configuration resolves at run time, by path.** A component asks the
  ConfigDb for what it needs and gets a `Result`. There is no compile-time
  check here and none is claimed — that is the design (D68), and Chapter 28 is
  the debugger's answer to it.
- **A wildcard is a path glob, not a type.** `"*"` reaches every component whose
  path matches, which is how two talkers share one `set`.
- **The higher setter wins a conflict, not the later one.** `ConflictTest` and
  `ConflictEnv` both set `MSG` on `env.loga` — the test by the longer path
  `"env.loga"`, the env by `"loga"` — and both resolve to the same component.
  The **test's** value lands: the transcript prints `PARENT RULES!`. Build runs
  top-down, so the test wrote *first*, which under "last write wins" would make
  it lose; precedence is by depth instead, and Chapter 28's dump shows the two
  entries side by side with their numbers (1000 for the test at depth 0, 999
  for the env at depth 1).

## Preserve these teaching points

Manuscript: [chapter-27-configuration](../../../book-pdf/src/chapter-27-configuration.md).

- ConfigDb is a path-addressed runtime database. Preserve its runtime failure
  taxonomy; do not replace it with compile-time configuration or a type-conflict
  compile-fail lesson.
- Explain the SystemVerilog typed-config mismatch and silent failed lookup
  carefully. Rustdv returns a Result naming the cause.
- Do not claim the UVM or pyuvm lacks wait_modified, or that nobody uses it. Check
  upstream implementations before making such claims.

Read this crate’s source and this README before changing a quoted
figure or behavioral claim; keep the source, figure map, and transcript aligned.
