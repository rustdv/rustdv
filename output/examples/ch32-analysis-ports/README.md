# Chapter 32: Analysis Ports — figure map

Run with:

```
sim-common/run_sim.sh ch32_analysis_ports playground
```

Figure numbers are the **book's** (D110: one numbering space per chapter, code
and transcripts drawn from the same sequence); the `.rs` captions carry the same
numbers. Figures 5, 7, 10 and 13 are transcripts, which is why the code captions
skip them.

| Figure | Title | Where |
|---|---|---|
| 1 | A subscriber counts what it sees | `src/ch32_analysis_ports.rs` (`Counter`) |
| 2 | A second subscriber on the same stream | `src/ch32_analysis_ports.rs` (`Collector`) |
| 3 | A source holds an analysis port and writes to it | `src/ch32_analysis_ports.rs` (`NumberGen`) |
| 4 | One publisher, two subscribers, one hub | `src/ch32_analysis_ports.rs` (`BroadcastTest`) |
| 5 | Both subscribers see every datum, all at `0.00ns` | transcript — `BroadcastTest` |
| 6 | A hub with no subscribers is legal | `src/ch32_analysis_ports.rs` (`NoSubscribersTest`) |
| 7 | Broadcasting to nobody | transcript — `NoSubscribersTest` |
| 8 | When the subscriber needs *time* | `src/ch32_analysis_ports.rs` (`Inbox`/`SlowChecker`) |
| 9 | The publisher does not wait for the slow subscriber | `src/ch32_analysis_ports.rs` (`SlowSubscriberTest`) |
| 10 | Writes at `0.00ns`, checks at 5, 10 and 15ns | transcript — `SlowSubscriberTest` |
| 11 | A watcher on a FIFO's tap is an ordinary subscriber | `src/ch32_analysis_ports.rs` (`TapLog`/`TapWatcher`) |
| 12 | A tap is wired like any other subscription | `src/ch32_analysis_ports.rs` (`FifoTapTest`) |
| 13 | Every item put, observed and not consumed | transcript — `FifoTapTest` |

Four tests, all ending `REGRESSION: PASS`.

Figures 11–13 came here from Chapter 31 (D117): the taps are analysis machinery
and could not be explained before subscribers were. `FifoTapTest` reuses Chapter
31's `Producer` and `Consumer` verbatim for its data path; they are in the crate
but the chapter does not reprint them.

## What this chapter proves

- **Delivery is synchronous and takes no simulation time.** `ap.write(&n)`
  returns after every subscriber's handler has run; the whole transcript below
  is at `0.00ns` (D87).
- **A subscriber shares its state, not itself.** The handler needs `&mut` its
  data while the publisher's `run` holds `&mut publisher`, and siblings cannot
  reach each other — so the data lives in a `RustdvShared<T>` and the port holds
  a second handle (D88).
- **One connection idiom.** `pub_export()`/`sub_export()` read exactly like
  Chapter 31's `put_export()`/`get_export()`; several subscribers on one
  `sub_export()` is what makes it a broadcast.
- **`AnalysisBus` is not `TlmFifo`, and holds nothing at all.** `write` calls
  every subscriber and returns; there is no queue in the hub, and a datum
  broadcast to nobody is gone (D86/D90). A component that wants to keep the
  traffic keeps it — a tally (Figure 1), a `Vec` (Figure 2), a `TlmFifo` of its
  own (Figure 8), a comparison against a prediction (Chapter 34). Two
  accessors, not three: `pub_export()` and `sub_export()`.
- **A subscriber that needs time buffers for itself.** `write` is synchronous
  and cannot await, so a subscriber whose work *takes* simulation time splits
  the job: `write` does the one instant thing — `try_put` into an unbounded
  `TlmFifo` it owns — and its `run` gets from that FIFO and takes as long as it
  likes. Figures 8–9. Note the FIFO is connected to no port at all; it is an
  ordinary handoff inside one component, between a synchronous method and an
  asynchronous one.
- **...but not for the UVM's reason.** A UVM scoreboard holds a
  `uvm_tlm_analysis_fifo` because a class gets one `write`, so a second stream
  needs the `uvm_analysis_imp_decl` macros and a FIFO per stream is the way
  around them. rustdv declares two `SubscribePort`s and two `Subscriber` impls
  (D20/D88), so that reason is gone. In Figure 8 the reason is time, and only
  time.
- **A FIFO's taps are subscriptions like any other.** `put_ap()` and `get_ap()`
  are the port of `uvm_tlm_fifo`'s built-in analysis ports (D23). The data path
  stays a queue — one consumer takes each item, the producer blocks when it is
  full — while the taps observe alongside: every subscriber sees every item,
  nothing is consumed, nobody is delayed. Figures 11–13.

## Transcript

Real Icarus output (`RUSTDV_RANDOM_SEED=1`):

```
      0.00ns INFO     rustdv: found 4 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running BroadcastTest (1/4)  [ch32-analysis-ports/src/ch32_analysis_ports.rs:193]
      0.00ns INFO     [BroadcastTest.source]: wrote 0
      0.00ns INFO     [BroadcastTest.source]: wrote 1
      0.00ns INFO     [BroadcastTest.source]: wrote 2
      0.00ns INFO     [BroadcastTest.counter]: counted 3 items
      0.00ns INFO     [BroadcastTest.collector]: collected [0, 1, 2]
      0.00ns INFO     BroadcastTest PASSED
      0.00ns INFO     running NoSubscribersTest (2/4)  [ch32-analysis-ports/src/ch32_analysis_ports.rs:231]
      0.00ns INFO     [NoSubscribersTest.source]: wrote 0
      0.00ns INFO     [NoSubscribersTest.source]: wrote 1
      0.00ns INFO     [NoSubscribersTest.source]: wrote 2
      0.00ns INFO     NoSubscribersTest PASSED
      0.00ns INFO     running SlowSubscriberTest (3/4)  [ch32-analysis-ports/src/ch32_analysis_ports.rs:325]
      0.00ns INFO     [SlowSubscriberTest.source]: wrote 0
      0.00ns INFO     [SlowSubscriberTest.source]: wrote 1
      0.00ns INFO     [SlowSubscriberTest.source]: wrote 2
      5.00ns INFO     [SlowSubscriberTest.checker]: checked 0
     10.00ns INFO     [SlowSubscriberTest.checker]: checked 1
     15.00ns INFO     [SlowSubscriberTest.checker]: checked 2
     15.00ns INFO     SlowSubscriberTest PASSED
     15.00ns INFO     running FifoTapTest (4/4)  [ch32-analysis-ports/src/ch32_analysis_ports.rs:444]
     15.00ns INFO     [FifoTapTest.producer]: put 0
     15.00ns INFO     [FifoTapTest.consumer]: got 0
     15.00ns INFO     [FifoTapTest.producer]: put 1
     15.00ns INFO     [FifoTapTest.consumer]: got 1
     15.00ns INFO     [FifoTapTest.producer]: put 2
     15.00ns INFO     [FifoTapTest.consumer]: got 2
     15.00ns INFO     [FifoTapTest.watcher]: tap saw [0, 1, 2]
     15.00ns INFO     FifoTapTest PASSED
******************************************************************************
** TEST                                       STATUS  SIM TIME (ns)      **
******************************************************************************
** BroadcastTest                                PASS           0.00      **
** NoSubscribersTest                            PASS           0.00      **
** SlowSubscriberTest                           PASS          15.00      **
** FifoTapTest                                  PASS           0.00      **
******************************************************************************
REGRESSION: PASS
```

The last test is the one to read for timing: the source's three writes all land
at `0.00ns` — a publisher is never held up by what a subscriber does with an
item — while the checker's results come out at 5, 10 and 15ns as it works
through its own queue.

## Preserve these teaching points

Manuscript: [chapter-32-analysis-ports](../../../book-pdf/src/chapter-32-analysis-ports.md).

- AnalysisBus stores no transactions: it synchronously calls subscribers and
  returns; a value with no subscriber is gone. Storage belongs to the subscriber.
- The plain struct implementing write is the Subscriber; a component hosts it.
  Connect chooses a stream, subscribe supplies a receiver. Explain the difference
  from uvm_subscriber as a component.
- A subscriber needing simulated time enqueues into its own unbounded FIFO during
  write, and its component run processes the inbox asynchronously. The FIFO is an
  internal handoff, not a port connection; bounded storage cannot provide analysis
  backpressure.
- Preserve the slow-subscriber transcript’s distinction between immediate writes
  and later processing. Chapter 34 demonstrates two input streams and two
  Subscriber implementations without imp_decl macros.
- Keep the FIFO-tap example near the end, after subscribers: TapLog/TapWatcher,
  FifoTapTest, and its transcript. Reuse the already-taught Producer/Consumer
  without reteaching their implementations. Taps observe traffic without taking
  items or delaying the queue.

Read this crate’s source and this README before changing a quoted
figure or behavioral claim; keep the source, figure map, and transcript aligned.
