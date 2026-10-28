# Appendix D: What rustdv Provides

Every Part II listing opens with `use rustdv::prelude::*` — the analog of `import uvm_pkg::*` and `from pyuvm import *`. This appendix is the complete reference for what that line brings into scope, plus the macros. The Chapter column points to where each name is taught; the Toolkit page (before Chapter 15) groups the same names by job. A dash means the name is provided for completeness but this book's examples never need it.

## The prelude, alphabetically

| Name | What it is | Chapter |
|---|---|---|
| `Active` | the active/passive agent knob, read from the ConfigDb (pyuvm's `is_active` int, as an enum) | 40 |
| `AggregateHandle` | typed member lookup and iteration for unpacked structs and unions exposed through VPI | 17 |
| `AnalysisBus` | the broadcast hub; it stores nothing | 32 |
| `BigUint` | arbitrary-width unsigned values, used by `get_bigint` and `set_bigint` | 17 |
| `build_all` | drive `build` across a component tree (the runner's job) | 24 |
| `channel` | make a `(Sender, Receiver)` queue pair with a capacity | — |
| `check_all` | drive `check` across a tree | 24 |
| `CheckSink` | the collector `check` phases write failures into | 24 |
| `Clock` | a software clock driver — taught once, then retired in favor of BFMs that wait on edges | 17 |
| `Component` | the lifecycle trait: `build`, `connect`, `run`, and the other phase methods | 24 |
| `ComponentNode` | the tree-traversal trait `#[derive(Component)]` implements | 21, 24 |
| `ConfigDb` | path-addressed runtime configuration; `get` returns a `Result` naming the cause | 25, 27 |
| `connect_all` | drive `connect` across a tree | 24 |
| `create_seq` | build a sequence through the sequence factory | 36 |
| `Either` | the answer from racing two differently-typed futures | — |
| `end_of_elaboration_all` | phase driver | 24 |
| `Event` | set once; everyone waiting wakes (SV: named `event`) | 16 |
| `extract_all` | phase driver | 24 |
| `Factory` | the component registry: build by type or name, override by type, name, or instance | 29 |
| `final_all` | phase driver | 24 |
| `first2` | race two futures; the first to finish wins (SV: `fork...join_any`) | 16 |
| `GetPort` | the consuming end of a TLM connection | 31 |
| `HandleChildren` | the lookup and traversal trait for hierarchy and aggregate handles | 17 |
| `HandleError` | what signal access returns instead of a crash | 17, 19 |
| `HandleEvent` | the trait supplying logic-edge and value-change triggers | 17 |
| `HierarchyHandle` | a handle to a scope in the design hierarchy | 17 |
| `join2` | run two futures together; wait for both (SV: `fork...join`) | 16 |
| `Lock` | mutual exclusion with an RAII guard (SV: a one-key `semaphore`) | 16 |
| `log` | the logging facade; policy is per hierarchy | 15, 26 |
| `Logic`, `LogicArray` | four-state values, scalar and vector | 17 |
| `LogicHandle` | a named DUT signal: read it, drive it; a typo'd name is an `Err` | 17 |
| `next_time_step` | trigger for the simulator's next time step | 17 |
| `NullTrigger` | the trigger that is ready the next time it is polled | 15 |
| `ObjectionGuard` | returned by `ctx.raise_objection`; the run phase ends when the last guard drops | 23 |
| `PeekPort` | the look-without-taking end of a TLM connection | 31 |
| `PortName`, `PortOwner` | how the elaboration check names an unconnected port | 31 |
| `print_hierarchy` | dump a component tree | — |
| `PublishPort` | the publishing end a component declares | 32 |
| `PutPort` | the producing end of a TLM connection | 31 |
| `Queue` | the sim-aware mailbox: bounded puts and empty gets block in simulated time (SV: `mailbox#(T)`) | 16 |
| `read_only`, `read_write` | settled sampling and writable synchronization triggers | 17 |
| `RealHandle` | a real variable read/written as `f64` | 17 |
| `Receiver` | the getting end `channel` returns | — |
| `report_all` | phase driver | 24 |
| `Rng` | the deterministic per-test random source behind `ctx.rng()` | 20 |
| `run_component_test` | run a component tree as a self-contained test | — |
| `run_extract_check_report` | drive the closing phases together | — |
| `RustdvComp` | a slot holding any factory-built component | 29 |
| `RustdvCtx` | the context: path, logging, rng, DUT handle, objection — the framework, handed as an argument | 15 |
| `RustdvSeq` | a slot holding any factory-built sequence — `RustdvComp`'s parallel | 36 |
| `RustdvShared` | a cloneable handle to one shared object — `Rc<RefCell>` wearing the framework's name | 32 |
| `Sender` | the putting end `channel` returns | — |
| `SeqCtx` | the context a sequence `body` receives | 36 |
| `SeqError` | what a sequence can fail with | 36 |
| `SeqItem` | the bounds a sequence-item type must meet | 36 |
| `SeqItemExport`, `SeqItemPort` | the driver's side of the sequencer handshake | 36 |
| `Sequence` | the trait with one method, `body` — a test program, not a component | 36 |
| `Sequencer` | grants sequences their turns; feeds the driver | 36 |
| `set_seq_override` | change which sequence `create_seq` builds | 36 |
| `service_read_only` | run a synchronous service closure at settled ReadOnly while simulation time is held | 40 |
| `sim_time_ns` | the current simulated time | — |
| `SimDuration` | an amount of simulated time | 17 |
| `spawn`, `spawn_named` | launch a concurrent task; the named form stamps its log lines | 16 |
| `start_all` | drive the run phase across a tree | 24 |
| `start_of_simulation_all` | phase driver | 24 |
| `StringHandle` | owned string reads and scheduled/immediate writes; embedded NUL bytes are rejected | 17 |
| `SubscribePort` | the subscribing end a component declares | 32 |
| `Subscriber` | the trait a subscriber implements once per stream | 32 |
| `TaskHandle` | what `spawn` returns: await it for the result, or `cancel()` it | 16 |
| `TestError` | the error a failing test returns; `Ok(())` is a pass | 15 |
| `Timer` | the simulated-time trigger: `Timer::ns(2).await` | 15 |
| `TlmFifo` | the FIFO two components share without learning each other's names | 31 |
| `TxnId` | the ticket `finish_item` returns; claims a response | 37, 38 |
| `with_timeout` | wrap an await with a deadline | — |

## The macros

| Name | What it does | Chapter |
|---|---|---|
| `#[rustdv::test]` | registers a test with the runner (cocotb: `@cocotb.test()`) | 15, 21 |
| `#[derive(Component)]` | writes the component-tree plumbing (SV: the `uvm_component_utils` family) | 21, 24 |
| `vpi_bootstrap!()` | one line per testbench crate: exports the entry points the simulator loads | 17 |
| `first!`, `join!` | variadic race and join; call as `rustdv::first!` / `rustdv::join!` or import explicitly | 16 |

## On the surface, outside the prelude

A few names live on the crate but not in the prelude; reach them as `rustdv::Name`.

| Name | What it is | Chapter |
|---|---|---|
| `ConfigError` | why a ConfigDb `get` failed, as a value | 28 |
| `ConnectError` | why a connection could not be made | 31 |
| `TlmFull`, `TlmEmpty`, `TlmError` | the channel layer's refusals: what `Sender::try_send` and `Receiver::try_recv` answer | — |
| `Maker` | the closure type the factory stores per registration | 29 |
| `ResponseQueue` | the store behind `get_response` — responses held for claiming, in order or by ticket (pyuvm's `ResponseQueue`) | — |
| `SimHandle` | the kind-tagged result of general child lookup; `as_*()` conversions return `Result` | 17 |
| `ValueError` | why a value read/conversion failed, including X/Z, width, and unavailable representations | 17 |
| `top_module` | root lookup selected by `RUSTDV_TOP`, falling back to the first root when unset | 17 |
| `TimeoutError`, `TaskError`, `Executor`, `TestRegistration` | infrastructure corners a testbench rarely touches | — |

`HandleBase`, for `name()` and `full_name()`, lives at
`rustdv::sim::handle::HandleBase`; import it explicitly. The low-level GPI
layer has its own `rustdv::gpi::AnyHandle`, while simulator-layer child lookup
returns `SimHandle`.

## Optional simulator services

These names live under `rustdv::sim::simulator_trace`, outside the prelude.
Chapter 40 explains their use with the repository's Verilator host.

| Name | What it does |
|---|---|
| `host_present`, `host_format` | probe the optional host and its advertised waveform format before ReadOnly |
| `start`, `flush`, `stop`, `status` | control or query capture inside `service_read_only` |
| `TraceStatus`, `TraceState` | report availability, capture state, time window, and dump count |
| `TraceFormat` | identify FST or another advertised host format |
| `TraceError` | report an unavailable capability, invalid path, wrong state, or host failure |

The shipped Verilator host uses FST. RECORD mode compiles capture support;
FAST and INSPECT do not. RTL line/expression coverage uses the separate
COVERAGE profile and `RUSTDV_COVERAGE_FILE`, independently of functional
coverage collected by a Rust subscriber.

The whole of each layer is also re-exported for power users: `rustdv::sim`, `rustdv::runner`, `rustdv::gpi`.
