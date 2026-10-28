# The rustdv Toolkit

Every listing from Chapter 15 on begins with the same line:

```rust,ignore
use rustdv::prelude::*;
```

It is the analog of `import uvm_pkg::*` in SystemVerilog and `from pyuvm import *` in Python, and it brings the testbench API into scope. Part I introduced every Rust concept before using it, and this page keeps that promise for the framework: it is the declaration site for the names the glob import hides. Skim it now to learn the shape of what rustdv provides, then return to it whenever a listing uses a name you have not met. Each entry names the chapter that teaches it properly, and Appendix D holds the complete alphabetical reference.

## One crate, four layers

rustdv is a facade: one dependency in `Cargo.toml`, one import in the source. Behind the facade sit four layers, and knowing which layer a name comes from tells you what kind of thing it is.

**rustdv-sim** is the simulation layer — coroutines, triggers, tasks, queues, and signal handles. It does the job cocotb does for Python: it owns the event loop and talks to the simulator's scheduler. Everything in it makes sense in a testbench with no UVM anywhere.

**rustdv-methodology** is the UVM analog: components and phases, the ConfigDb, the factory, TLM ports and FIFOs, analysis broadcasting, and sequences. Everything in it corresponds to something you already know by another name.

**rustdv-runner** finds the registered tests in your compiled testbench and runs them — the job `run_test()` and the plusargs flow do in SystemVerilog, and the job cocotb's test discovery does in Python.

**rustdv-gpi** speaks VPI to the simulator, several layers beneath anything you write. The name is borrowed from cocotb's GPI deliberately: same job, same position in the stack.

Power users can reach whole layers as `rustdv::sim`, `rustdv::runner`, and `rustdv::gpi`. The prelude curates the surface a testbench needs.

## `ctx`: the framework in your hand

rustdv has no globals. There is no `uvm_root`, no singleton pool, no parent pointer to climb — so everything the UVM lets you reach ambiently must be handed to you instead. The handing is done through one argument, conventionally named `ctx`, of type `RustdvCtx`. Every test receives it; every phase method receives it. The nearest UVM analogy is the `uvm_phase phase` argument every phase method already takes — rustdv widens that argument until it carries the whole framework.

`ctx` is also how a component knows *where it is*: it carries the component's path in the tree, which is why every log line arrives stamped with the full path and no component ever stores its own name.

| You write | You get | Chapter |
|---|---|---|
| `ctx.dut()` | the handle to the top of the design | 17 |
| `ctx.info("...")` (and `warn`, `error`, ...) | a log line stamped with time and path | 15, 26 |
| `ctx.rng()` | the seeded per-test random generator | 20 |
| `ctx.raise_objection("why")` | an `ObjectionGuard` — the run phase ends when every guard is dropped | 23 |

## The simulation kit

From rustdv-sim. These are the names of a coroutine testbench, UVM or not.

| Name | What it is, and when you reach for it | Chapter |
|---|---|---|
| `Timer` | the simulated-time trigger: `Timer::ns(2).await` *(SV: `#2ns`; cocotb: `Timer(2, "ns")`)* | 15 |
| `NullTrigger` | the trigger that is ready the next time anyone asks — the smallest possible await | 15 |
| `TestError` | the error a failing test returns; `Ok(())` is a pass | 15 |
| `spawn`, `spawn_named` | launch a concurrent task *(SV: `fork...join_none`; cocotb: `start_soon`)*; the named form stamps the task's log lines | 16 |
| `TaskHandle` | what `spawn` returns — await it for the task's result, or `cancel()` it | 16 |
| `Queue` | the sim-aware mailbox: a bounded `Queue` blocks a full `put` and an empty `get`, in simulated time *(SV: `mailbox#(T)`)* | 16 |
| `Event` | set once, and everyone waiting wakes *(SV: named `event`)* | 16 |
| `Lock` | mutual exclusion with an RAII guard *(SV: a one-key `semaphore`)* | 16 |
| `join2`, `first2`, `join!`, `first!` | run futures together and wait for both, or for the first *(SV: `fork...join` / `join_any`)* | 16 |
| `Clock` | a software clock driver — taught once and then retired, because rustdv BFMs wait on edges rather than make them | 17 |
| `LogicHandle` | a named signal in the design: read it, drive it; asking for a signal that does not exist is an `Err`, not a surprise | 17 |
| `Logic`, `LogicArray` | four-state values, kept out of your arithmetic until you decide what x means | 17 |
| `BigUint` | arbitrary-width unsigned values for buses beyond primitive integer widths | 17 |
| `RealHandle`, `StringHandle` | real and string variables, with scheduled and immediate writes | 17 |
| `AggregateHandle` | named members of unpacked structs and unions exposed through VPI | 17 |
| `HandleChildren`, `HandleEvent` | traits supplying hierarchy/member lookup and logic-edge triggers | 17 |
| `read_only`, `read_write`, `next_time_step` | await settled sampling, writable synchronization, or the next simulator time step | 17 |
| `service_read_only` | run a synchronous service closure at settled ReadOnly while simulation time is held | 40 |
| `HandleError` | what signal access returns instead of a crash | 17, 19 |
| `SimDuration` | an amount of simulated time | 17 |
| `Rng` | the deterministic random source behind `ctx.rng()` — one seed, one reproducible test | 20 |
| `log` | the logging facade the framework routes through `ctx`; policy is set per hierarchy | 15, 26 |

A handful of other names — `with_timeout`, `sim_time_ns`, `Either`, `HierarchyHandle` — are in the prelude for completeness and cataloged in Appendix D. General child lookups return `rustdv::SimHandle`, which is outside the prelude. Name metadata comes from `rustdv::sim::handle::HandleBase`. Optional waveform control lives in `rustdv::sim::simulator_trace`; Chapter 40 explains its use with the Verilator host.

## The structure kit

From rustdv-methodology: the component tree and its lifecycle.

| Name | What it is, and when you reach for it | Chapter |
|---|---|---|
| `Component` (trait) | the lifecycle: `build`, `connect`, and the other phase methods a component may implement | 24 |
| `#[derive(Component)]` | writes the tree-traversal plumbing so your struct's children are found by the phases | 21, 24 |
| `ComponentNode` | what the derive implements — the thing a tree of components is made of | 21, 24 |
| `ObjectionGuard` | returned by `ctx.raise_objection`; the run phase ends when the last one drops | 23 |
| `CheckSink` | the collector a `check` phase writes failures into; one error in it fails the test | 24 |
| `start_all` | drives a phase across a whole tree — the runner's job, never yours to call (its siblings `build_all`, `connect_all`, and the rest are in Appendix D) | 24 |
| `Active` | the active/passive knob an agent reads from the ConfigDb *(pyuvm's `is_active` int, as an enum)* | 40 |

## Configuration and the factory

| Name | What it is, and when you reach for it | Chapter |
|---|---|---|
| `ConfigDb` | path-addressed runtime configuration: `set` by path and key, `get` returns a `Result` that names what went wrong | 25, 27 |
| `Factory`, `RustdvComp` | build components through a registry so a test can override *what* gets built — by type, by name, or by instance | 29 |
| `create_seq`, `set_seq_override`, `RustdvSeq` | the same idea for sequences: a slot the factory fills | 36 |

## The TLM kit

| Name | What it is, and when you reach for it | Chapter |
|---|---|---|
| `PutPort`, `GetPort`, `PeekPort` | the directional ends a component declares; `connect` wires them at elaboration | 31 |
| `TlmFifo` | the FIFO two components share without ever learning each other's names — the point of decoupling | 31 |
| `RustdvShared` | a cloneable handle to one shared object — `Rc<RefCell>` wearing the framework's name | 32 |
| `PortName`, `PortOwner` | how the elaboration check names an unconnected port when it reports the whole tree at once | 31 |

## The analysis kit

| Name | What it is, and when you reach for it | Chapter |
|---|---|---|
| `AnalysisBus` | the broadcast hub — it stores nothing; `write` calls every subscriber and returns | 32 |
| `PublishPort`, `SubscribePort` | the publishing and subscribing ends | 32 |
| `Subscriber` | the trait a subscriber implements per stream — two streams, two impls, no macros | 32 |

## The sequence kit

| Name | What it is, and when you reach for it | Chapter |
|---|---|---|
| `Sequence` | the trait with one method: `body` — a test program, not a component | 36 |
| `Sequencer` | the component that grants sequences their turns and feeds the driver | 36 |
| `SeqItem`, `SeqCtx`, `SeqError` | the item's bounds, the sequence's context, and what can go wrong | 36 |
| `SeqItemPort`, `SeqItemExport` | the driver's side of the handshake | 36 |
| `TxnId` | the ticket `finish_item` returns; `get_response` claims its answer, in order or out of it | 37, 38 |

## The macros

| Name | What it does | Chapter |
|---|---|---|
| `#[rustdv::test]` | registers a test with the runner *(cocotb: `@cocotb.test()`; SV: `+UVM_TESTNAME` machinery)* | 15, 21 |
| `#[derive(Component)]` | writes the component plumbing *(SV: the `uvm_component_utils` family)* | 21, 24 |
| `vpi_bootstrap!()` | one line per testbench crate: exports the entry points the simulator loads | 17 |
| `first!`, `join!` | the variadic forms of `first2`/`join2` | 16 |

That is the toolkit. You do not need to hold it all — you need to know it is here, and that no listing from here on uses a name this page or an earlier chapter has not declared. When one seems to, that is a defect in the book, not in your memory.
