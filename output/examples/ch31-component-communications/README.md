# Chapter 31: Component Communications — figure map

Run with:

```
sim-common/run_sim.sh ch31_component_communications playground
```

Figure numbers are the **book's** (D110: one numbering space per chapter, code
and transcripts drawn from the same sequence); the `.rs` captions carry the same
numbers. Figures 4, 8, 13 and 15 are transcripts, which is why the code
captions skip them.

| Figure | Title | Where |
|---|---|---|
| 1 | A producer holds a put port and blocks on a full FIFO | `src/ch31_component_communications.rs` (`Producer`) |
| 2 | A consumer that peeks, then gets | `src/ch31_component_communications.rs` (`Consumer`) |
| 3 | The env builds the two components and a FIFO, then wires them | `src/ch31_component_communications.rs` (`PutGetPeekTest`) |
| 4 | Put, peek, get, in order | transcript — `PutGetPeekTest` |
| 5 | A non-blocking producer never waits | `src/ch31_component_communications.rs` (`NbProducer`) |
| 6 | A non-blocking consumer | `src/ch31_component_communications.rs` (`NbConsumer`) |
| 7 | Same wiring, non-blocking components | `src/ch31_component_communications.rs` (`NonBlockingTest`) |
| 8 | Retrying a full FIFO, and the packet coming home | transcript — `NonBlockingTest` |
| 9 | A processing pipeline — y = 2x² | `src/ch31_component_communications.rs` (comment block); the book renders it as an SVG drawing |
| 10 | The first stage squares its input | `src/ch31_component_communications.rs` (`SquareIt`) |
| 11 | The second stage doubles what the first produced | `src/ch31_component_communications.rs` (`TimesTwo`) |
| 12 | The test drives the pipeline and checks the answer | `src/ch31_component_communications.rs` (`MathTest`) |
| 13 | 2, 8, 18, 32 | transcript — `MathTest` |
| 14 | A port left unconnected is an elaboration error | `src/ch31_component_communications.rs` (`UnconnectedTest`) |
| 15 | The whole tree's connection errors, at once | transcript — `UnconnectedTest` |

Four tests, all ending `REGRESSION: PASS`.

The FIFO's built-in taps (`put_ap()`/`get_ap()`) were demonstrated here and now
live in Chapter 32 (D117): they are analysis-layer machinery, and this chapter
only names them in passing.

## What this chapter proves

- **Connection reaches an erased child.** A parent holds children as
  `RustdvComp`, so `producer.put_port` does not exist to be written. Every
  connection here goes through `ComponentNode::port_slot` — a trait method,
  which is reachable through `dyn` where a cast is not (D83b).
- **A parent's `run` is concurrent with its children's.** `MathTest` sends `x`
  and waits for `y` while two worker components wait on it. Under a phaser that
  finishes the children first, this test cannot run at all (D82b).
- **Unconnected ports are found at elaboration, not at first use.**
  `UnconnectedTest` fails before any run phase, naming
  `UnconnectedTest.producer.put_port` (D22/D85).
- **A failed `try_put` gives the transaction back.** Figures 4–6 carry a
  non-`Copy` `Packet` on purpose. `try_put` takes the packet by value, so the
  UVM's bit return would have eaten a packet that was never delivered;
  `Err(back)` is the packet coming home, and the retry loop takes it back.
  Written the tempting way — `while port.try_put(packet).is_err()` — it does
  not compile, because `packet` was moved on the first attempt. A `u32`
  version compiles and teaches the reader a loop that breaks on their first
  real transaction.

## Transcript

Real Icarus output (`RUSTDV_RANDOM_SEED=1`):

```
      0.00ns INFO     rustdv: found 4 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running PutGetPeekTest (1/4)  [ch31-component-communications/src/ch31_component_communications.rs:124]
      0.00ns INFO     [PutGetPeekTest.producer]: put 0
      0.00ns INFO     [PutGetPeekTest.consumer]: peeked 0
      0.00ns INFO     [PutGetPeekTest.consumer]: got 0
      0.00ns INFO     [PutGetPeekTest.producer]: put 1
      0.00ns INFO     [PutGetPeekTest.consumer]: peeked 1
      0.00ns INFO     [PutGetPeekTest.consumer]: got 1
      0.00ns INFO     [PutGetPeekTest.producer]: put 2
      0.00ns INFO     [PutGetPeekTest.consumer]: peeked 2
      0.00ns INFO     [PutGetPeekTest.consumer]: got 2
      0.00ns INFO     PutGetPeekTest PASSED
      0.00ns INFO     running NonBlockingTest (2/4)  [ch31-component-communications/src/ch31_component_communications.rs:247]
      0.00ns INFO     [NonBlockingTest.producer]: put 0
      0.00ns INFO     [NonBlockingTest.producer]: FIFO full, retrying
      0.00ns INFO     [NonBlockingTest.consumer]: got pkt0 (n=0)
      1.00ns INFO     [NonBlockingTest.producer]: put 1
      1.00ns INFO     [NonBlockingTest.producer]: FIFO full, retrying
      1.00ns INFO     [NonBlockingTest.consumer]: got pkt1 (n=1)
      2.00ns INFO     [NonBlockingTest.producer]: put 2
      2.00ns INFO     [NonBlockingTest.consumer]: got pkt2 (n=2)
      2.00ns INFO     NonBlockingTest PASSED
      2.00ns INFO     running MathTest (3/4)  [ch31-component-communications/src/ch31_component_communications.rs:345]
      2.00ns INFO     [MathTest.square_it]: 1² = 1
      2.00ns INFO     [MathTest.times_two]: 2 × 1 = 2
      2.00ns INFO     [MathTest]: PASSED: x=1, y=2
      2.00ns INFO     [MathTest.square_it]: 2² = 4
      2.00ns INFO     [MathTest.times_two]: 2 × 4 = 8
      2.00ns INFO     [MathTest]: PASSED: x=2, y=8
      2.00ns INFO     [MathTest.square_it]: 3² = 9
      2.00ns INFO     [MathTest.times_two]: 2 × 9 = 18
      2.00ns INFO     [MathTest]: PASSED: x=3, y=18
      2.00ns INFO     [MathTest.square_it]: 4² = 16
      2.00ns INFO     [MathTest.times_two]: 2 × 16 = 32
      2.00ns INFO     [MathTest]: PASSED: x=4, y=32
      2.00ns INFO     MathTest PASSED
      2.00ns INFO     running UnconnectedTest (4/4)  [ch31-component-communications/src/ch31_component_communications.rs:421]
      2.00ns INFO     UnconnectedTest PASSED
******************************************************************************
** TEST                                       STATUS  SIM TIME (ns)      **
******************************************************************************
** PutGetPeekTest                               PASS           0.00      **
** NonBlockingTest                              PASS           2.00      **
** MathTest                                     PASS           0.00      **
** UnconnectedTest                              PASS           0.00      **
******************************************************************************
REGRESSION: PASS
```

`UnconnectedTest` passes *because* it fails: it is declared
`#[rustdv::test(expect_error = "tlm_unconnected_port")]`, and the elaboration
sweep reports

```
these TLM ports were declared but never connected:
  UnconnectedTest.producer.put_port (put)
```

## Preserve these teaching points

Manuscript: [chapter-31-component-communications](../../../book-pdf/src/chapter-31-component-communications.md).

- Ports, exports, and TlmFifo decouple components through a shared FIFO; channels
  do not replace TLM. Missing connections are diagnosed for the tree during
  elaboration, not through a direction-mismatch compile-fail example.
- Keep the non-Copy Packet try_put example. Err(packet) returns ownership of the
  refused item; a Copy integer would conceal the move/retry rule. Try_get returns
  Option for the corresponding empty case.
- Preserve the self-checking y = 2x² pipeline (SquareIt, TimesTwo, MathTest). It
  demonstrates parent and children running concurrently, a request/response loop,
  the same connection form for self and children, and infinite workers stopped by
  drained objections.
- Teach the pipeline as the DUT-free rehearsal for Chapter 36. Leave FIFO tap
  details to Chapter 32, with only the put_ap/get_ap forward reference here.

Read this crate’s source and this README before changing a quoted
figure or behavioral claim; keep the source, figure map, and transcript aligned.
