# Appendix C: SystemVerilog-UVM → rustdv Translations

For readers coming from SystemVerilog UVM (and *The UVM Primer*): where each piece of your working vocabulary went. Python readers want Appendix B, this table's twin.

## Language level

| SystemVerilog | Rust | Chapter |
|---|---|---|
| `byte`, `shortint`, `int` | `u8`/`i8`, `u16`/`i16`, `u32`/`i32` — no silent truncation | 3 |
| `logic` four-state values | `Logic` enum / `LogicArray` — no x in arithmetic | 7, 17 |
| Unsigned logic vectors read as integers | `get_u8()` through `get_u128()`, or `get_bigint()` for arbitrary widths; reads return `Result` | 17 |
| `real` / `string` variables | `RealHandle` (`f64`) / `StringHandle` (owned strings), with scheduled and immediate writes | 17 |
| Unpacked struct/union members | `AggregateHandle::child()` / `children()` through simulator VPI | 17 |
| A settled sampling point | `read_only().await`; writes are illegal until a later writable time step | 17 |
| A writable synchronization point | `read_write().await`; buffered writes apply before its waiters resume | 17 |
| `typedef enum` (an int in disguise) | `enum` — a real type; exhaustively matched | 7 |
| `case` + `default` (+ `unique` warnings) | `match` — missing cases are compile errors | 4 |
| `class ... extends`, `virtual`, `super.new()` | traits, default methods, composition + delegation | 10 |
| `pure virtual function` in a virtual class | a required trait method, checked at the `impl` | 10 |
| parameterized class `#(type T = int)` | generics `<T: Bound>`, checked at definition | 11 |
| `class ... #(type REQ, type RSP = REQ)` | `SeqItemPort<REQ, RSP = REQ>` — same convention | 11 |
| `local` / `protected` | private-by-default, `pub` to export | 14 |
| `null` handle, `$cast` | `Option<T>`, exhaustive `match` — no null, no cast | 9 |
| status flags and sentinel returns | `Result<T, E>` + `?` — failure in the signature | 9 |
| `$sformatf` | `format!` | 8 |
| `fork` / `join_none` / `disable` | `spawn(future)` → `TaskHandle`; `handle.cancel()` | 16 |
| `forever` | `loop` (an expression — it can `break` with a value) | 4 |
| `mailbox #(T)`, `try_put`/`try_get` | `sim::Queue<T>` — same names, `Result`/`Option` answers | 16 |
| named `event`, `->done`, `@(done)` | `sim::Event` — `set()` / `wait().await` | 16 |
| `semaphore` (one key) | `sim::Lock` — FIFO-fair, RAII guard | 16 |
| `@(posedge clk)`, `#2ns` | `clk.rising_edge().await`, `Timer::ns(2).await` | 15, 17 |
| `` `define ``-style codegen (`` `uvm_*_utils ``) | attribute + derive macros — syntax trees, not text | 21 |
| package + `.f` file + vendor tarball | crate + `Cargo.toml` + crates.io | 14 |
| *(no equivalent)* | `cargo test` — unit tests with no simulator | 14 |

These signal accesses use the simulator's VPI objects, rather than a compiled
virtual interface. `RUSTDV_TOP` names the root and `HandleChildren` supplies
name lookup. `SimHandle::as_*()` reports wrong-kind objects as `HandleError`.
Integer access is unsigned and rejects X/Z or an oversized signal; use
`LogicArray` to preserve four-state values. Packed aggregates exposed as logic
use `LogicHandle`; unpacked member traversal requires simulator support.

Functional coverage in the book's collector is independent of Verilator RTL
line/expression coverage. Chapter 40 describes COVERAGE mode, optional waveform
capture, and `service_read_only()` for application-defined inspection at a
settled point. These services depend on the repository's simulator host/build
configuration as well as the Rust API.

## Methodology level

| SystemVerilog UVM | rustdv | Chapter |
|---|---|---|
| `class my_test extends uvm_test` + `run_test()` | `#[rustdv::test]` on a struct; the runner drives its phases | 23 |
| `phase.raise_objection(this)` / `drop_objection` | `ctx.raise_objection(..)` → RAII `ObjectionGuard`; drop releases | 23 |
| `uvm_component(name, parent)` tree | children are struct fields; `#[derive(Component)]`; paths derived by the walk | 24 |
| `build_phase` (top-down) / `connect_phase` (bottom-up) | `fn build(&mut self, ctx)` / `fn connect(&mut self, ctx)` — real phases, same directions | 24 |
| `run_phase` (objection-gated task) | `async fn run` — concurrent across the tree; ends when objections drain | 24, 31 |
| elaboration + post-run phases | same names; **top-down**, where SV runs them bottom-up | 24 |
| `` `uvm_info(id, msg, verbosity) `` | `ctx.info(..)` — same time/level/`[path]` line format | 26 |
| `set_report_verbosity_level_hier()` | `ctx.set_logging_level_hier(..)` | 26 |
| `uvm_config_db#(T)::set/get`, wildcards | `ConfigDb::set(ctx, glob, key, v)` / `get` → `Result` — one key, no type in the address | 25, 27 |
| virtual interface via config database | `Rc<TinyAluBfm>` in the ConfigDb | 25 |
| a failed `get()` (silent `return 0`) | `ConfigError` naming which failure; `#[must_use]` | 27, 28 |
| `print_config()` / `+UVM_CONFIG_DB_TRACE` | `ConfigDb::print()` / `ConfigDb::set_tracing(true)` | 28 |
| `` `uvm_component_utils `` registration | `#[derive(Component)]` registers by name, universally | 21, 29 |
| `type_id::create("name", this)` | `Foo::create_comp()` — overridable (`new_comp()` = `new`, fixed) | 29 |
| `set_type_override_by_type` / `_by_name` / instance | `Factory::set_type_override::<A, B>()` / `_by_name` / `set_inst_override` | 29, 30 |
| `uvm_factory::get().print()` | `Factory::print()` | 29 |
| TLM-1 put/get/peek port + export + `connect()` | `PutPort`/`GetPort`/`PeekPort` + `fifo.put_export().connect(comp, PORT_NAME)` | 31 |
| `uvm_tlm_fifo` (with built-in taps) | `TlmFifo<T>` (with `put_ap()`/`get_ap()`) | 31 |
| `try_put()` returns a bit | `try_put(T)` → `Result<(), T>` — a refused item comes back | 31 |
| unconnected port found at first use | elaboration sweep names every unwired port before run | 31 |
| `uvm_analysis_port.write()` | `PublishPort<T>::write(&T)`, brokered by an `AnalysisBus` hub | 32 |
| `uvm_subscriber` (one `write` per class) | a `Subscriber<T>` impl per stream — two streams, two impls, no `imp_decl` | 32, 34 |
| `uvm_tlm_analysis_fifo` in scoreboards | absent — the subscriber owns its storage | 32 |
| `uvm_agent` + `is_active` | env reads `Active` from the ConfigDb; a passive env leaves the driver slot empty | 40 |
| `do_copy` / `do_compare` / `convert2string` | `#[derive(Clone, PartialEq, Debug)]` + hand-written `Display` | 10, 35 |
| `copy(other)` / `clone()` | `clone_from(&mut self, src)` / `clone()` | 35 |
| `uvm_field_*` macros (runtime field walking) | `derive` — the same generation, at compile time | 21, 35 |
| `uvm_sequence #(REQ, RSP)`, `body()` | `impl Sequence` — `type Req`/`type Rsp`, `async fn body(ctx)` | 36 |
| `start_item(req)` / `finish_item(req)` | `ctx.start_item(&mut req)` / `ctx.finish_item(req)` → ticket | 36 |
| `seq_item_port.get_next_item()` / `item_done()` | same names; `item_done(Some(rsp))` answers | 36, 38 |
| `try_next_item()` (absent from pyuvm) | `try_next_item()` → `Option<SeqItem<REQ>>` | 37 |
| `rsp.set_id_info(req)` + `get_response()` | auto-tagged; `get_response(Some(ticket))` / `try_get_response` | 37, 38 |
| `seq.start(seqr)` / virtual sequence with no sequencer | `seq.start(&seqr)` / `start_virtual()` | 36, 39 |
| sequencer grab/lock/priority arbitration | unported (FIFO arbitration only) — a recorded gap | 36 |
| `assert` (prints, simulation continues) | `assert!` (panic = test fails, on the spot) | 9 |
| `uvm_error` vs `uvm_fatal` (convention) | `CheckSink::error` = DUT check; `panic!` = testbench bug | 9, 24 |
