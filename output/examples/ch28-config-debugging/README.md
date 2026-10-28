# Chapter 28: Debugging the ConfigDb — figure map

Run with:

```
sim-common/run_sim.sh ch28_config_debugging playground
```

No DUT. Chapter 27 showed configuration working; this one shows it failing,
which is the more useful skill.

Figure numbers are the **book's** (D110: one numbering space per chapter, code
and transcripts drawn from the same sequence); the `.rs` captions carry the same
numbers. Figures 7, 10 and 12 are transcripts and dump excerpts, which is why
the code captions skip them.

| Figure | Title | Where |
|---|---|---|
| 1 | The logger from Chapter 27, unchanged | `MsgLogger` |
| 2 | A message for only one of the two loggers | `MsgTest` |
| 3 | Misspelling a key | `MsgTestAlmostFixed` |
| 4 | A logger that coped | `NiceMsgLogger` |
| 5 | Printing the ConfigDb | `NiceMsgTest` |
| 6 | Debugging the misspelled key by printing | `NiceMsgTestAlmostFixed` |
| 7 | The dump, with `MSG` beside `MESG` | transcript — dump excerpt |
| 8 | Wildcards behaving, for contrast with the failures | `MultiMsgTest` |
| 9 | Both the env and the test configure `env.loga` | `ConflictTest` |
| 10 | The dump shows both writes and their precedences | transcript — dump excerpt |
| 11 | Tracing every ConfigDb operation | `GlobalTest` |
| 12 | The trace, resolved paths and all | transcript — trace excerpt |

Seven tests, all ending `REGRESSION: PASS`.
Port of the Python book's chapter 32.

## The three transcript figures

Verbatim from `sim-common/run_sim.sh ch28_config_debugging playground`,
`RUSTDV_RANDOM_SEED=1`.

**Figure 7 — the misspelling, caught by the dump.** `NiceMsgTestAlmostFixed`
wrote `MESG` where the logger asks for `MSG`. The dump puts them one line apart
and the mistake is visible; the warning underneath is the logger coping.

```
      0.00ns INFO     running NiceMsgTestAlmostFixed (4/7)  [ch28-config-debugging/src/ch28_config_debugging.rs:179]
      0.00ns INFO     PATH                        : KEY       : DATA
      0.00ns INFO     NiceMsgTestAlmostFixed.env.loga: MSG       : {1000: "LOG A msg"}
      0.00ns INFO     NiceMsgTestAlmostFixed.env.logb: MESG      : {1000: "LOG B msg"}
      0.00ns INFO     [NiceMsgTestAlmostFixed.env.loga]: LOG A msg
      0.00ns WARNING  [NiceMsgTestAlmostFixed.env.logb]: Could not find MSG. Setting to default
      0.00ns INFO     [NiceMsgTestAlmostFixed.env.logb]: No message for you!
      0.00ns INFO     NiceMsgTestAlmostFixed PASSED
```

**Figure 10 — the conflict, with both writes showing.** One entry, two values,
two precedences. The resolved value tells you who won; the dump tells you who
else was trying.

```
      0.00ns INFO     running ConflictTest (6/7)  [ch28-config-debugging/src/ch28_config_debugging.rs:264]
      0.00ns INFO     PATH                        : KEY       : DATA
      0.00ns INFO     ConflictTest.env.loga       : MSG       : {1000: "PARENT RULES!", 999: "CHILD RULES!"}
      0.00ns INFO     [ConflictTest.env.loga]: PARENT RULES!
      0.00ns INFO     ConflictTest PASSED
```

**Figure 12 — the trace.** Every `set` and `get`, with the context it was made
from, the offset asked for, and the path it resolved to. The resolved path is
the thing you actually got wrong when a lookup misses.

```
      0.00ns INFO     running GlobalTest (7/7)  [ch28-config-debugging/src/ch28_config_debugging.rs:316]
      0.00ns INFO     CFGDB/SET context=GlobalTest offset="env.loga" -> GlobalTest.env.loga MSG="LOG A msg"
      0.00ns INFO     CFGDB/SET context=GlobalTest offset="env.logb" -> GlobalTest.env.logb MSG="LOG B msg"
      0.00ns INFO     CFGDB/SET context=GlobalTest offset="env.t*" -> GlobalTest.env.t* MSG="TALK TALK"
      0.00ns INFO     CFGDB/SET context=<none> offset="*" -> * MSG="GLOBAL"
      0.00ns INFO     CFGDB/GET context=GlobalTest.env.loga offset="" -> GlobalTest.env.loga MSG="LOG A msg"
      0.00ns INFO     [GlobalTest.env.loga]: LOG A msg
      0.00ns INFO     CFGDB/GET context=GlobalTest.env.logb offset="" -> GlobalTest.env.logb MSG="LOG B msg"
      0.00ns INFO     [GlobalTest.env.logb]: LOG B msg
      0.00ns INFO     CFGDB/GET context=GlobalTest.env.talka offset="" -> GlobalTest.env.talka MSG="TALK TALK"
      0.00ns INFO     [GlobalTest.env.talka]: TALK TALK
      0.00ns INFO     CFGDB/GET context=GlobalTest.env.talkb offset="" -> GlobalTest.env.talkb MSG="TALK TALK"
      0.00ns INFO     [GlobalTest.env.talkb]: TALK TALK
      0.00ns INFO     CFGDB/GET context=GlobalTest.env.gtalk offset="" -> GlobalTest.env.gtalk MSG="GLOBAL"
      0.00ns INFO     [GlobalTest.env.gtalk]: GLOBAL
      0.00ns INFO     GlobalTest PASSED
```

## The three debug tools

**A `Result` that names the cause.** `ConfigError::NotFound` and
`TypeMismatch` are separate variants, so Figure 4 can recover from a missing
value while still failing on a wrong type — defaulting past a type mismatch
would bury a real bug. SystemVerilog collapses both into `return 0`.

**A dump that shows the competition.** `ConfigDb::print()` lists every entry
with its precedences, because a resolved value tells you who won but not who
else was trying:

```
PATH                        : KEY       : DATA
ConflictTest.env.loga       : MSG       : {1000: "PARENT RULES!", 999: "CHILD RULES!"}
```

The test wrote at depth 0 (precedence 1000) and the env at depth 1 (999), so
the parent wins — and the numbers say so rather than asking you to trust the
rule. Figure 6 uses the same dump to spot `MSG` beside `MESG`.

**A tracer.** `ConfigDb::set_tracing(true)` logs every operation with the
context, the offset, and the path they resolved to — which is the thing you
actually got wrong when a lookup misses:

```
CFGDB/SET context=GlobalTest offset="env.loga" -> GlobalTest.env.loga MSG="LOG A msg"
CFGDB/SET context=<none> offset="*" -> * MSG="GLOBAL"
CFGDB/GET context=GlobalTest.env.gtalk offset="" -> GlobalTest.env.gtalk MSG="GLOBAL"
```

## Expected failures

Figures 2 and 3 are declared
`#[rustdv::test(expect_error = "config_not_found")]` — they pass only if they
fail *that way*. A test that failed for some other reason is still reported
as a failure, which a plain "expected to fail" flag could not tell you.

Verified: changing one expectation to the wrong kind produces

```
MsgTest FAILED: expected error 'config_type_mismatch', got config_not_found: ...
```

## What this chapter replaced

It was previously three programs that failed to compile on purpose, arguing
that configuration bugs are caught by the compiler — Figure 1 demonstrated a
"wrong path" by misspelling a *struct field*. A wrong path in a config
database is `"env.loga"` versus `"env.logA"`: a string, resolved at run time,
that no compiler will ever check. Late binding costs you compile-time
checking and buys you these tools instead. That trade is the honest version.

## Preserve these teaching points

Manuscript: [chapter-28-config-debugging](../../../book-pdf/src/chapter-28-config-debugging.md).

- Teach ConfigError variants, dump output, tracing, and expect_error as tools for
  runtime configuration. Do not resurrect compile-fail demonstrations for runtime
  config conflicts.
- Take the complete current dump/trace excerpts and figure map from this crate’s
  README, including the MESG/MSG spelling comparison and precedence behavior.

Read this crate’s source and this README before changing a quoted
figure or behavioral claim; keep the source, figure map, and transcript aligned.
