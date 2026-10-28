# Chapter 15: async/await and the Executor — figure map

Part II figures split into two kinds: **pure-Rust binaries** (run with
`cargo run --bin ...` from `output/examples/`) and **simulator figures**
(functions in `src/ch15_async_await_executor.rs`, run together on Icarus with the chapter's sim
command below).

Sim command:

```
sim-common/run_sim.sh ch15_async_await_executor playground
```

| Figure | Title | Where |
|---|---|---|
| 1 | Hello world as a test | `src/ch15_async_await_executor.rs` (`hello_world`) — sim |
| 2 | Polling a future by hand | `src/bin/ch15_fig02_polling_a_future_by_hand.rs` |
| 3 | A future that says Pending — the trigger's whole job | `src/bin/ch15_fig03_a_future_that_says_pending.rs` |
| 4 | An event loop in a page | `src/bin/ch15_fig04_an_event_loop_in_a_page.rs` |
| 5 | VHDL waits for 2 nanoseconds | text figure (VHDL, not runnable here) |
| 6 | SystemVerilog waits for two nanoseconds | text figure (SystemVerilog, not runnable here) |
| 7 | Rust waits for 2 nanoseconds | `src/ch15_async_await_executor.rs` (`wait_2ns`) — sim |

Full sim transcript (RUSTDV_RANDOM_SEED=1):

```
      0.00ns INFO     rustdv: found 2 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running hello_world (1/2)  [ch15-async-await-executor/src/ch15_async_await_executor.rs:12]
      0.00ns INFO     Hello, world.
      0.00ns INFO     hello_world PASSED
      0.00ns INFO     running wait_2ns (2/2)  [ch15-async-await-executor/src/ch15_async_await_executor.rs:20]
      2.00ns INFO     I am DONE waiting!
      2.00ns INFO     wait_2ns PASSED
******************************************************************************
** TEST                                       STATUS  SIM TIME (ns)      **
******************************************************************************
** hello_world                                  PASS           0.00      **
** wait_2ns                                     PASS           2.00      **
******************************************************************************
REGRESSION: PASS
```

## Preserve these teaching points

Manuscript: [chapter-15-async-await-executor](../../../book-pdf/src/chapter-15-async-await-executor.md).

- The unnumbered Toolkit precedes this chapter so the first prelude import and
  test attribute have already been introduced. Keep new API names declared there
  or here before using them.
- Teach async/future polling and the executor before simulator mechanisms.
  RustdvCtx is the one context; do not introduce phase-specific context types.

Read this crate’s source and this README before changing a quoted
figure or behavioral claim; keep the source, figure map, and transcript aligned.
