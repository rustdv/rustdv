# Chapter 24: Components — figure map

Run with:

```
sim-common/run_sim.sh ch24_components playground
```

No DUT: structure is the subject. `playground` is an empty top module.

## First half — running the phases (done)

The nine UVM phases are real `Component` methods again. `build` and
`connect` were destroyed in the first rustdv pass — demoted to "constructor
conventions" — and are restored here (D5/D6/D51). The test is a component;
the runner drives every phase in order, the way `@pyuvm.test()` hands the
class to its phaser. Each phase logs under the path the walk derived (D7),
so the lines read `[PhaseTest]`.

| Figure | Title | Where |
|---|---|---|
| 1 | A uvm_test demonstrating the nine phase methods | `src/ch24_components.rs` (`PhaseTest`) |
| 2 | The lifecycle runs in order | transcript below |

Port of the Python book's chapter 28 Figure 1. Transcript (seed 1):

```
      0.00ns INFO     rustdv: found 2 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running PhaseTest (1/2)  [ch24-components/src/ch24_components.rs:38]
      0.00ns INFO     [PhaseTest]: 1 build
      0.00ns INFO     [PhaseTest]: 2 connect
      0.00ns INFO     [PhaseTest]: 3 end_of_elaboration
      0.00ns INFO     [PhaseTest]: 4 start_of_simulation
      0.00ns INFO     [PhaseTest]: 5 run
      0.00ns INFO     [PhaseTest]: 6 extract
      0.00ns INFO     [PhaseTest]: 7 check
      0.00ns INFO     [PhaseTest]: 8 report
      0.00ns INFO     [PhaseTest]: 9 final
      0.00ns INFO     PhaseTest PASSED
******************************************************************************
** TEST                                       STATUS  SIM TIME (ns)      **
******************************************************************************
** PhaseTest                                    PASS           0.00      **
******************************************************************************
REGRESSION: PASS
```

Phase order is pyuvm's, not SV UVM's (D34): build top-down, connect
bottom-up, run bottom-up, the elaboration and post-run phases top-down. A
single component does not show direction — the three-level hierarchy does.

## Second half — building the hierarchy (done)

`TestTop -> mc -> bc`, each parent **creating its child in its own build
phase** (D6's two-stage construction — a child is an `Option<T>` field,
filled in during `build`). The phaser descends into whatever `build`
created, so the tree grows top-down as it is walked.

| Figure | Title | Where |
|---|---|---|
| 4 | The test builds the middle component | `src/ch24_components.rs` (`TestTop`) |
| 5 | The middle component builds the bottom component | `MiddleComp` |
| 6 | The bottom component's run phase | `BottomComp` |

Transcript (seed 1):

```
      0.00ns INFO     running TestTop (2/2)  [ch24-components/src/ch24_components.rs:118]
      0.00ns INFO     [TestTop]: build phase
      0.00ns INFO     [TestTop.mc]: end of elaboration phase
      0.00ns INFO     [TestTop.mc.bc]: run phase
      0.00ns INFO     [TestTop]: final phase
      0.00ns INFO     TestTop PASSED
```

The paths `[TestTop.mc]` and `[TestTop.mc.bc]` are **derived by the walk**,
never stored (D7): `TestTop.build` creates `mc`, the phaser recurses and
`mc.build` creates `bc`, and each logs under the path the traversal
accumulated. Move a component in the tree and its path follows, because
nothing hand-typed it.

**Run is sequential, not concurrent.** Each component's `run` is awaited to
completion before the next — right while run bodies raise, act, and drop
their own objection, as here. Concurrent run phases (spawned `'static`
tasks) are a later increment; see the design-decisions log.

## Preserve these teaching points

Manuscript: [chapter-24-components](../../../book-pdf/src/chapter-24-components.md).

- Build and connect are real phases: the gap between a component existing and its
  children being built permits configuration and factory overrides. Build is top-
  down; connect is bottom-up.
- Use the current phase traversal implementation to explain differences from
  SystemVerilog for the other phases. Do not claim phase legality is enforced by
  context types; RustdvCtx is shared and illegal operations fail at runtime.
- Component::run is an async trait method; the framework uses a dyn-safe mirror
  for erased traversal. Do not imply async traits are freely object-safe.
- Bare #[component] marks a field as a child. Preserve the intentional figure-
  number gaps filled by transcripts and the book-only tree drawing.

Read this crate’s source and this README before changing a quoted
figure or behavioral claim; keep the source, figure map, and transcript aligned.
