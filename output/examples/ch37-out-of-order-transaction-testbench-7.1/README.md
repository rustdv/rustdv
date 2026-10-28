# Chapter 37: Out-of-Order Transactions: Testbench 7.1 — figure map

Run with:

```
sim-common/run_sim.sh ch37_out_of_order_transaction_testbench_7_1 playground
```

Figure numbers are the **book's** (D110: one numbering space per chapter, code
and transcripts drawn from the same sequence); the `.rs` captions carry the same
numbers. The hanging-`get_response` passage is prose in the book, not a numbered
figure, and the crate's comment for it carries no number.

| Figure | Title | Where |
|---|---|---|
| 1 | A request that says how long it takes, and its answer | `src/ch37_out_of_order_transaction_testbench_7_1.rs` (`Req`, `Rsp`) |
| 2 | A driver that accepts work and answers it later | `src/ch37_out_of_order_transaction_testbench_7_1.rs` (`Driver`, `InFlight`) |
| 3 | Send four requests, then collect four answers | `src/ch37_out_of_order_transaction_testbench_7_1.rs` (`Seq`) |
| 4 | A sequencer and a driver, and the test that runs them | `src/ch37_out_of_order_transaction_testbench_7_1.rs` (`Env`, `ResponseTest`) |
| 5 | Answers arrive in the reverse of the order asked | transcript — `ResponseTest` |

One test, ending `REGRESSION: PASS`.

## Transcript

**Figure 5.** Verbatim from `sim-common/run_sim.sh
ch37_out_of_order_transaction_testbench_7_1 playground`, `RUSTDV_RANDOM_SEED=1`.

The chapter's whole argument in one log. `sent` runs `#1, #2, #3, #4` with
descending delays; `answering` and `got` both run `#4, #3, #2, #1` — the exact
reverse — and every answer still carries back its own request's delay.

The collection order is the thing to notice. The sequence polls its outstanding
tickets with `try_get_response` and drops each one as it is answered, so the
answers appear in the order they were *finished*. Had it blocked on `#1`, then
`#2`, then `#3`, the `got` lines would have come out in send order no matter
what the driver did, and the out-of-order work would have been real but
invisible.

Nothing here is random: the delays are chosen so that each request finishes
three ticks ahead of the one before it, which is more than the two ticks it
waits to be accepted, so the reversal is guaranteed rather than seed-dependent.

```
      0.00ns INFO     rustdv: found 1 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running ResponseTest (1/1)  [ch37-out-of-order-transaction-testbench-7.1/src/ch37_out_of_order_transaction_testbench_7_1.rs:231]
     15.00ns INFO     [ResponseTest.env.driver]: accepted #1, delay 10
     15.00ns INFO     [Seq]: sent #1, delay 10
     35.00ns INFO     [ResponseTest.env.driver]: accepted #2, delay 7
     35.00ns INFO     [Seq]: sent #2, delay 7
     55.00ns INFO     [ResponseTest.env.driver]: accepted #3, delay 4
     55.00ns INFO     [Seq]: sent #3, delay 4
     75.00ns INFO     [ResponseTest.env.driver]: accepted #4, delay 1
     75.00ns INFO     [Seq]: sent #4, delay 1
     80.00ns INFO     [ResponseTest.env.driver]: answering #4
     85.00ns INFO     [Seq]: got #4, delay 1
     90.00ns INFO     [ResponseTest.env.driver]: answering #3
     95.00ns INFO     [Seq]: got #3, delay 4
    100.00ns INFO     [ResponseTest.env.driver]: answering #2
    105.00ns INFO     [Seq]: got #2, delay 7
    110.00ns INFO     [ResponseTest.env.driver]: answering #1
    115.00ns INFO     [Seq]: got #1, delay 10
    115.00ns INFO     ResponseTest PASSED
******************************************************************************
** TEST                                       STATUS  SIM TIME (ns)      **
******************************************************************************
** ResponseTest                                 PASS         115.00      **
******************************************************************************
REGRESSION: PASS
```

## What this chapter proves

- **Answers can come back out of order, and the ticket is what matches them.**
  The TinyALU cannot show this — it runs one operation at a time, so nothing
  ever returns out of sequence. That is why the mechanism is taught here first,
  on an empty DUT, before Chapter 38 applies it to the real one.
- **`item_done` releases the sequence before the answer exists.** That is what
  puts four requests in flight at once. Hold the handshake open until the answer
  is ready and only one is ever outstanding — and then the ticket has nothing to
  disambiguate.
- **`try_next_item` lets the driver take new work while old work is still in
  flight.** The driver accepts `#2` at 35 ns while `#1` still has seven ticks
  left on it.
- **Asking for an answer that will never exist hangs forever.** Nothing can
  distinguish "not ready yet" from "never coming", so it is the sequence
  writer's job to ask only for answers that are owed. There is no runnable
  figure for it, because the only way to show it is a test that never ends.

## Preserve these teaching points

Manuscript: [chapter-37-out-of-order-transaction-testbench-7.1](../../../book-pdf/src/chapter-37-out-of-order-transaction-testbench-7.1.md).

- This chapter is the plain out-of-order request/response example, not Fibonacci
  or the retired repair-desk story. The empty playground top deliberately avoids a
  serial DUT that could not demonstrate reordering.
- Requests carry deterministic latencies (10, 7, 4, 1). Preserve a spread large
  enough for completion to overtake acceptance; changing the delays can hide the
  lesson without failing the check.
- The sequence polls outstanding tickets with try_get_response so completion order
  is visible. Blocking on responses in send order would conceal reordering.
- The driver uses nonblocking try_next_item so it can continue aging accepted work
  when no new request is queued. Item_done releases the request handshake before
  the response returns; put_response retains its ticket.

Read this crate’s source and this README before changing a quoted
figure or behavioral claim; keep the source, figure map, and transcript aligned.
