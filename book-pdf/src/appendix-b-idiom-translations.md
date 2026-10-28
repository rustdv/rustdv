# Appendix B: Python → Rust Idiom Translations

For readers coming from cocotb and pyuvm (and *Python for RTL Verification*): the working translations this book used, gathered for reference. SystemVerilog readers want Appendix C, this table's twin. Legend: **[C]** cocotb, **[P]** pyuvm.

## Language and runtime

| Python | Rust | Chapter |
|---|---|---|
| `async def` coroutine, resumed via `send(None)` | `async fn` → `Future`, resumed via `poll()` | 15 |
| `@cocotb.test()` | `#[rustdv::test]` | 15, 21 |
| exceptions fail the test | `Result<(), TestError>`; panics = testbench bugs | 9, 18 |
| `cocotb.start_soon(coro)` | `spawn(future) -> TaskHandle<T>` | 16 |
| `await task` | `task.await` → `Result<T, TaskError>` | 16 |
| `task.kill()` | `handle.cancel()` — the future is dropped; cleanup in `Drop` | 16 |
| `Combine(...)` / `First(...)` | `join2`/`join!` / `first2`/`first!` | 16 |
| `try/except QueueFull` | `try_put` → `Result<(), T>` (rejected item handed back) | 16, 31 |
| decorator registration at import time | `linkme` gathers test registrations into a distributed slice | 21 |
| metaclass class registration | `#[derive(Component)]` supplies factory registration | 21, 29 |
| `getattr(obj, name)` dispatch | pass the function/closure itself | 33 |
| `logging` levels + handlers | `log::` levels, `set_level_for(prefix)`, `log_to_file` | 26 |

## cocotb layer

| Python (cocotb) | Rust (rustdv-sim) | Chapter |
|---|---|---|
| `Timer(2, units="ns")` | `Timer::ns(2).await` | 15 |
| `RisingEdge(sig)` / `FallingEdge(sig)` | `sig.rising_edge().await` / `sig.falling_edge().await` | 17 |
| `ClockCycles(clk, n)` | `for _ in 0..n { clk.rising_edge().await; }` | 17 |
| `dut.sig` attribute magic | `dut.signal("sig")` returns `Result<LogicHandle, HandleError>`; `?` propagates failure | 17 |
| `sig.value = x` / `int(sig.value)` | `sig.set_u64(x)` / `sig.get_u64()?` | 17 |
| Python integers for wide buses | `get_u128()?` or arbitrary-width `get_bigint()?`; matching typed setters | 17 |
| four-state signal values | `get_logic()?` / `set_logic(&value)` with `LogicArray` | 17 |
| real and string signal values | `dut.real(name)?` / `dut.string(name)?`, then `get()` / `set()` | 17 |
| hierarchy and aggregate member lookup | `child(name)?` returns `SimHandle`; `as_*()?` checks the kind | 17 |
| `ReadWrite()` / `ReadOnly()` | `read_write().await` / `read_only().await` | 17 |
| `NextTimeStep()` | `next_time_step().await` | 17 |
| `Clock(dut.clk, 10, units="ns").start()` | `Clock::new(&clk, SimDuration::ns(10)).start()` | 17 |
| `cocotb.queue.Queue(maxsize=1)` | `Queue::new(Some(1))`; `Queue::unbounded()` | 16 |
| `Event` / `Lock` | `sim::Event` / `sim::Lock` (FIFO-fair, RAII guard) | 16 |

The integer getter selects an unsigned width and rejects X/Z or an oversized
signal. Use `BigUint` when the width exceeds the primitive types. Traversal and
edge methods come from `HandleChildren` and `HandleEvent`, both imported by
the prelude. Real/string/aggregate access depends on simulator VPI support.
The launcher selects the DUT root with `RUSTDV_TOP`.

For an application-defined inspection service, `service_read_only(closure)`
runs a synchronous operation while holding settled simulation time. Runtime
waveform capture uses `rustdv::sim::simulator_trace` and a trace-capable host;
Chapter 40 describes the Verilator modes and their requirements.

## pyuvm layer

| Python (pyuvm) | Rust (rustdv) | Chapter |
|---|---|---|
| `@pyuvm.test()` on a class, `uvm_test_top` | `#[rustdv::test]` on a struct; the root is named after your test | 23 |
| `raise_objection()`/`drop_objection()` | `ctx.raise_objection(..)` → RAII `ObjectionGuard`; drop releases | 23 |
| `uvm_component(name, parent)` tree | children are struct fields; `#[derive(Component)]`; paths derived | 24 |
| the nine phases, pyuvm's traversal order | the nine phases, same order: `build`, `connect`, ... `final_phase` | 24 |
| `self.logger`, `[uvm_test_top.comp]` | `ctx.info(..)`, same bracket format, path supplied by the walk | 26 |
| `ConfigDB().set/get`, wildcards, globals | `ConfigDb::set/get` — same paths, same globs, `Result` answers | 25, 27 |
| `except UVMConfigItemNotFound` | `match` on `ConfigError::NotFound { .. }` | 28 |
| metaclass registration + `create()` | `#[derive(Component)]` registers; `Foo::create_comp()` | 21, 29 |
| `set_type_override_by_type` | `Factory::set_type_override::<A, B>()` (also by name, by instance) | 29, 30 |
| TLM-1 put/get/peek port classes | `PutPort`/`GetPort`/`PeekPort`, wired export-to-port through a `TlmFifo` | 31 |
| `UVMTLMConnectionError` (lazy, at first use) | elaboration sweep names every unwired port before run | 31 |
| `uvm_analysis_port.write()` | `PublishPort<T>::write(&T)` through an `AnalysisBus` hub | 32 |
| `uvm_subscriber` (one `write` per class) | a `Subscriber<T>` impl per stream — two streams, two impls | 32, 34 |
| `uvm_tlm_analysis_fifo` | absent — the subscriber owns its storage | 32 |
| `uvm_object` do_copy/do_compare/`__str__` | `#[derive(Clone, PartialEq, Debug)]` + hand-written `Display` | 35 |
| `copy(other)` / `clone()` | `clone_from(&mut self, src)` / `clone()` | 35 |
| `uvm_sequence.body()` | `impl Sequence` — `type Req`/`type Rsp`, `async fn body(ctx)` | 36 |
| `start_item`/`finish_item` | `ctx.start_item(&mut req)` / `ctx.finish_item(req)` → ticket | 36 |
| `seq_item_port.get_next_item()` | `port.get_next_item().await` → `SeqItem<REQ>` | 36 |
| `item_done()` / `item_done(rsp)` + `set_id_info` | `item_done(None)` / `item_done(Some(rsp))` — auto-tagged | 36, 38 |
| `get_response()` | `get_response(Some(ticket))` / `try_get_response` — in order or by ticket | 37, 38 |
| *(no pyuvm counterpart)* `try_next_item` | `try_next_item()` → `Option` — the UVM's non-blocking accept, kept | 37 |
| `seq.start(seqr)` / `start(None)` for virtual | `seq.start(&seqr)` / `start_virtual()` | 36, 39 |
| `is_active` int from ConfigDB | `Active` enum from the ConfigDb; a passive env skips building the driver | 40 |
