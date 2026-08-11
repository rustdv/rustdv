Licensed under either of Apache License, Version 2.0 or MIT license at your option.

# rustdv

[![CI](https://github.com/rustdv/rustdv/actions/workflows/ci.yml/badge.svg)](https://github.com/rustdv/rustdv/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/rustdv.svg)](https://crates.io/crates/rustdv)
[![Open in GitHub Codespaces](https://github.com/codespaces/badge.svg)](https://codespaces.new/rustdv/rustdv?quickstart=1)

**rustdv** is a hardware verification framework in Rust. It gives you
simulator coroutines — `await` an edge, a timer, a queue — and, on top of
them, a testbench library **inspired by the UVM**: a component tree with
phases, a configuration database, a factory, transaction-level connections,
and sequences.

It talks to simulators over VPI. Your testbench compiles to a native shared
library that the simulator loads; there is no interpreter in the loop and no
Verilog wrapper to write. rustdv has **zero external dependencies** — the
whole framework is `std` and the crates in this repository.

This repository also holds **"Rust for RTL Verification"**, the book that
teaches it — 40 chapters, an interlude, and four appendices — plus every
figure in the book as runnable code.

| | |
|---|---|
| Crate | [`rustdv` on crates.io](https://crates.io/crates/rustdv) (the badge above is the current version) |
| Site | [rustdv.org](https://rustdv.org) |
| Getting started | [`rustdv/getting-started-with-rustdv.md`](rustdv/getting-started-with-rustdv.md) |
| Discussion | [GitHub Discussions](https://github.com/rustdv/rustdv/discussions) |

---

## Why a UVM analog, and not something new

The UVM is the dominant methodology in RTL verification, and its shape is not
an accident. SystemVerilog is a statically typed language, and its designers
had a static alternative on hand three separate times — `mailbox#(T)`,
parameterized classes, module parameters — and chose runtime indirection
anyway, building TLM, a factory, and a configuration database. Three for
three. That indirection is what lets you override a component you did not
write, connect two components that have never heard of each other, and
configure a testbench from outside it.

Rust is statically typed too, and the temptation to replace each of those
mechanisms with a type is strong. rustdv deliberately does not. The late
binding is load-bearing, so rustdv keeps it: `ConfigDb` lookups resolve at
run time, factory overrides are chosen before the component exists, and TLM
endpoints are found by name during the `connect` phase.

rustdv is **inspired by the UVM**. It is not an implementation of IEEE 1800.2
and does not claim conformance to it — where Rust offers something genuinely
better, rustdv takes it, and where it does not, rustdv follows the UVM.

## The UVM concept map

If you know the UVM, this is the whole translation table.

| UVM concept | In rustdv | Notes |
|---|---|---|
| `uvm_component` | `#[derive(Component)]` + `impl Component` | Children are struct fields marked `#[component]`. The derive supplies tree traversal; the trait supplies the phases. |
| `uvm_component_utils` | *nothing* | A component registers with the factory by deriving `Component`. There is no registration macro to forget. |
| Phasing | nine phases: `build`, `connect`, `end_of_elaboration`, `start_of_simulation`, `run`, `extract`, `check`, `report`, `final_phase` | The common phases, no custom phase graph. Every method has a default, so you write only the ones you use. `run` is `async`. |
| `uvm_config_db` | `ConfigDb::set` / `ConfigDb::get` | Path-globbed and typed. `ConfigDb::dump()` prints what is filed where. |
| `uvm_factory` overrides | `Type::create_comp()` and factory overrides | A slot declared `RustdvComp` is erased, so the factory can fill it with anything. |
| `uvm_analysis_port` | `PublishPort<T>` | `write(&t)` broadcasts. |
| `uvm_analysis_export` / `uvm_subscriber` | `SubscribePort<T>` + `impl Subscriber<T>` | A component can implement `Subscriber` for **several** types and get several `write` methods — no `uvm_analysis_imp_decl` needed. |
| `uvm_tlm_analysis_fifo` | `AnalysisBus<T>` + your own storage | The bus holds nothing; it calls each subscriber and returns. Storage is the subscriber's decision. |
| `uvm_tlm_fifo`, get/put ports | `TlmFifo`, `GetPort`, `PutPort`, `PeekPort`, `Queue` | |
| `uvm_sequencer` / `seq_item_port` | `Sequencer<Req, Rsp>` / `SeqItemPort<Req, Rsp>` | Same rendezvous: `get_next_item()` … `item_done()`. |
| `uvm_sequence` | `impl Sequence` with `async fn body` | A sequence is **not** a component: no path, no phases. |
| `uvm_sequence_item` | a plain struct | `Clone` is `do_copy`, `PartialEq` is `do_compare`, `Debug` is `convert2string`. No base class. |
| `uvm_object` reporting | `ctx.info(...)`, `ctx.warning(...)`, `ctx.error(...)` | Messages carry the component's hierarchical path and simulation time. |
| `raise_objection` / `drop_objection` | `ctx.raise_objection("why")` | Returns a guard. Dropping the guard drops the objection, so it cannot be leaked. |
| `run_test()` | `#[rustdv::test]` | Marks a component as a test. The runner finds every one and runs them in order. |
| `import uvm_pkg::*` | `use rustdv::prelude::*` | One dependency, one import. |

## Installation

You need **Rust** and a simulator. rustdv uses **Icarus Verilog** as its
four-state reference and supports **Verilator** for fast two-state functional
simulation. This repository pins its Rust toolchain in
[`rust-toolchain.toml`](rust-toolchain.toml), so `rustup` fetches the right
compiler on its own.

```sh
cargo add rustdv
```

Or clone this repository and run everything in place — the book's figures,
the regression suite, and the TinyALU testbench are all here.

```sh
git clone https://github.com/rustdv/rustdv.git
cd rustdv
```

If you would rather not install anything, click the Codespaces badge above:
you get Rust, Icarus Verilog, and Verilator, ready to run.

A testbench is a library the simulator loads, so its `Cargo.toml` needs a
`[lib]` section on top of what `cargo add` wrote for you:

```toml
[lib]
crate-type = ["cdylib", "rlib"]
```

`cdylib` is what the simulator dlopens; `rlib` is what lets `cargo test` run
your transaction and predictor unit tests with no simulator at all, which is
where a testbench is cheapest to debug. Those test executables cannot carry the
undefined `vpi_*` symbols the cdylib is allowed to have, so add the stub crate
that satisfies them — `cargo add --dev rustdv-vpi-stubs` — and `use
rustdv_vpi_stubs as _;` under `#[cfg(test)]`.

[`rustdv/getting-started-with-rustdv.md`](rustdv/getting-started-with-rustdv.md)
walks the whole path from an empty directory to a passing regression.

## Running the TinyALU

The TinyALU is, as the name implies, a tiny ALU: four operations — ADD, AND,
XOR, MUL — with a `start`/`done` handshake, one cycle for the logic
operations and three for the multiply. It is the design the book verifies
from chapter 18 onward, and the shipped testbench for it is in
[`rustdv/tinyalu_tb/`](rustdv/tinyalu_tb/).

```sh
sim/run_rustdv.sh
sim/run_rustdv.sh release verilator
```

Both commands build the same testbench shared library and load it through VPI.
The first uses Icarus; the second builds the DUT with Verilator and runs it
through rustdv's shared Verilator host. There is no Verilog testbench: rustdv
drives the top module directly.

Two tests run. `RandomTest` sends random operands across every operation five
times each; `MaxTest` sends `0xff op 0xff` once per operation. Here is the
tail of a real run (`RUSTDV_RANDOM_SEED=1`, Linux/Icarus):

```
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 24, b: 60, op: Mul } out=AluResult { result: 1440 } expected=AluResult { result: 1440 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: 20 compared, 0 mismatches
    630.00ns INFO     [RandomTest.inner.env.coverage]: coverage: Add=5 And=5 Mul=5 Xor=5
    630.00ns INFO     RandomTest PASSED
    630.00ns INFO     running MaxTest (2/2)  [tinyalu_tb/src/tinyalu_tb.rs:92]
    700.00ns INFO     [MaxTest.inner.env.result_mon]: result_monitor: AluResult { result: 510 }
    700.00ns INFO     [MaxTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 255, b: 255, op: Add }
    820.00ns INFO     [MaxTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 255, b: 255, op: Mul } out=AluResult { result: 65025 } expected=AluResult { result: 65025 } check=PASS
    820.00ns INFO     [MaxTest.inner.env.scoreboard]: scoreboard: 4 compared, 0 mismatches
    820.00ns INFO     [MaxTest.inner.env.coverage]: coverage: Add=1 And=1 Mul=1 Xor=1
    820.00ns INFO     MaxTest PASSED
******************************************************************************
** TEST                                       STATUS  SIM TIME (ns)      **
******************************************************************************
** RandomTest                                   PASS         630.00      **
** MaxTest                                      PASS         190.00      **
******************************************************************************
REGRESSION: PASS
```

Every message carries the component's hierarchical path —
`RandomTest.inner.env.scoreboard` — the same way `uvm_test_top.env.scoreboard`
does.

## The rustdv testbench

The rest of this section walks the TinyALU testbench from the top down. Every
listing below is real code from
[`rustdv/tinyalu_tb/src/`](rustdv/tinyalu_tb/src/), with the teaching comments
stripped; the files there carry the full commentary.

### Importing rustdv

One dependency, one import, and one macro that exports the VPI entry points
the simulator looks for:

```rust
use rustdv::prelude::*;

rustdv::vpi_bootstrap!();
```

### The test

A test is a component like any other, so it gets the whole phase lifecycle
and the runner drives it. `BaseTest` files the BFM where any component can
find it, builds the environment, and starts whichever sequence the factory
has been told to build.

Points to notice:

* `#[derive(Component)]` supplies the tree traversal. The `#[component]`
  attribute marks a field as a child — no argument, and no registration call.
* `env` is declared `RustdvComp`, an erased slot. That is what makes it
  overridable.
* `build` runs top-down, so anything a test above sets in the `ConfigDb` is
  already there when a child looks for it.
* `raise_objection` returns a guard. The run phase ends when every objection
  has been dropped, and a guard cannot outlive its scope.
* The sequence is built with `create_seq::<BaseSeq>()` — through the factory,
  so a test above chose what it actually is.

```rust
#[derive(Component, Default)]
pub struct BaseTest {
    #[component]
    env: RustdvComp,
}

impl Component for BaseTest {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        let bfm = TinyAluBfm::new(&ctx.dut()).expect("TinyALU signals");
        ConfigDb::set(None, "*", "BFM", Rc::new(bfm));
        self.env = AluEnv::new_comp();
    }

    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("stimulus");

        let seqr: Sequencer<alu_item::AluCommand, alu_item::AluResult> =
            ConfigDb::get(Some(ctx), "", "SEQR")?;

        let mut seq = create_seq::<BaseSeq>();
        seq.start(&seqr).await?;

        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM")?;
        bfm.wait_idle().await;

        ctx.info("sequence complete");
        Ok(())
    }
}
```

The two real tests add nothing but a choice. `#[rustdv::test]` marks them for
the runner, and each sets a factory override so that `BaseSeq` — the type
`BaseTest` asks for — is built as something else. The testbench structure
does not change between them; the **program** does.

```rust
#[rustdv::test(timeout_time = 500, timeout_unit = "us")]
#[derive(Component, Default)]
struct RandomTest {
    #[component]
    inner: RustdvComp,
}

impl Component for RandomTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        set_seq_override::<BaseSeq, RandomSeq>();
        self.inner = BaseTest::new_comp();
    }
}
```

### The environment

`AluEnv` is the container. Two phases do the work a constructor would
otherwise do: `build` creates the children top-down, so a test above can
override any of them before they exist, and `connect` wires them bottom-up,
once they all do. The gap between the two is where configuration, factory
overrides, and TLM connection live.

Points to notice:

* Every child that a test might want to replace is a `RustdvComp`. The
  sequencer and the two analysis buses are concrete, because something has to
  make the `connect` call and plumbing is never an override target.
* Two configuration choices, both with defaults, so the ordinary case
  configures nothing. A passive environment has no driver at all — the slot is
  simply left empty, rather than holding a driver told not to drive.
* The sequencer's handle goes into the `ConfigDb`, which is how a test three
  levels up starts a sequence without knowing where the sequencer lives.
* Every connection has the same shape: an endpoint holder, a named export,
  `connect(component, PORT_NAME)`. It reads the same whether the traffic is a
  sequencer rendezvous or a broadcast, and nothing reaches inside an erased
  child.

```rust
#[derive(Component, Default)]
pub struct AluEnv {
    #[component]
    seqr: Sequencer<AluCommand, AluResult>,
    #[component]
    driver: RustdvComp,
    #[component]
    cmd_mon: RustdvComp,
    #[component]
    result_mon: RustdvComp,
    #[component]
    scoreboard: RustdvComp,
    #[component]
    coverage: RustdvComp,
    #[component]
    cmd_bus: AnalysisBus<AluCommand>,
    #[component]
    result_bus: AnalysisBus<AluResult>,
    is_active: bool,
    with_coverage: bool,
}

impl Component for AluEnv {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        let activity: Active = ConfigDb::get(Some(ctx), "", "IS_ACTIVE").unwrap_or(Active::Active);
        self.is_active = activity == Active::Active;
        self.with_coverage = ConfigDb::get(Some(ctx), "", "WITH_COVERAGE").unwrap_or(true);

        self.seqr = Sequencer::new();
        ConfigDb::set(None, "*", "SEQR", self.seqr.handle());

        if self.is_active {
            self.driver = Driver::create_comp();
        }
        self.cmd_mon = CmdMonitor::create_comp();
        self.result_mon = ResultMonitor::create_comp();
        self.scoreboard = Scoreboard::create_comp();
        if self.with_coverage {
            self.coverage = Coverage::create_comp();
        }

        self.cmd_bus = AnalysisBus::new();
        self.result_bus = AnalysisBus::new();
    }

    fn connect(&mut self, _ctx: &mut RustdvCtx) {
        if self.is_active {
            self.seqr.seq_item_export().connect(&self.driver, Driver::SEQ_ITEM_PORT);
        }

        self.cmd_bus.pub_export().connect(&self.cmd_mon, CmdMonitor::AP);
        self.cmd_bus.sub_export().connect(&self.scoreboard, Scoreboard::CMD_IN);
        if self.with_coverage {
            self.cmd_bus.sub_export().connect(&self.coverage, Coverage::CMD_IN);
        }

        self.result_bus.pub_export().connect(&self.result_mon, ResultMonitor::AP);
        self.result_bus.sub_export().connect(&self.scoreboard, Scoreboard::RESULT_IN);
    }

    fn start_of_simulation(&mut self, ctx: &mut RustdvCtx) {
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM").expect("the test sets BFM");
        bfm.start_tasks();
    }
}
```

The BFM's collector tasks are `spawn`ed in `start_of_simulation` because they
must outlive that phase. Everything above the BFM uses a `run` phase instead —
`spawn` is reserved for exactly this case.

That connect phase builds this:

```text
  sequences --> [seqr] --> Driver --> BFM --> DUT

  CmdMonitor    --pub--> [cmd_bus]    --sub--> Scoreboard
                               \-------sub--> Coverage
  ResultMonitor --pub--> [result_bus] --sub--> Scoreboard
```

### The monitors

A monitor watches the bus and broadcasts what it sees. It knows nothing about
who is listening: the scoreboard and the coverage collector both subscribe to
the same analysis bus, and neither is visible from here.

Notice that `run` is `async` and the BFM call is `await`ed — Rust makes the
distinction between a time-consuming call and an immediate one visible at
every call site. Notice also that the monitor has no constructor arguments:
the BFM arrives from the `ConfigDb`, and the analysis port is wired by the
environment. That is what makes the whole tree overridable.

The monitor's `run` loops forever. It does not need to know when to stop —
each component races the objection-drained event individually, so a monitor
still in its loop is torn down cleanly once the test's objection drops, and
`extract`/`check`/`report` still run.

```rust
#[derive(Component, Default)]
pub struct CmdMonitor {
    #[port(publish)]
    ap: PublishPort<AluCommand>,
}

impl Component for CmdMonitor {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM")?;
        loop {
            let cmd = bfm.get_cmd().await;
            ctx.info(&format!("cmd_monitor: {cmd:?}"));
            self.ap.write(&cmd);
        }
    }
}
```

`ResultMonitor` is the same shape for results.

### The scoreboard

An `AnalysisBus` holds nothing. It calls each subscriber and returns, so
*where the traffic goes* is the subscriber's decision — which is the answer to
"is this an analysis port or an analysis FIFO?" It is a port, and the
subscriber supplies the FIFO if it wants one.

This scoreboard wants both streams in order, so it keeps a `Vec` of each:

```rust
#[derive(Default)]
struct CmdLog {
    cmds: Vec<AluCommand>,
}

impl Subscriber<AluCommand> for CmdLog {
    fn write(&mut self, cmd: &AluCommand) {
        self.cmds.push(cmd.clone());
    }
}

#[derive(Default)]
struct ResultLog {
    results: Vec<AluResult>,
}

impl Subscriber<AluResult> for ResultLog {
    fn write(&mut self, res: &AluResult) {
        self.results.push(res.clone());
    }
}
```

Two streams, two ports, two `write` methods, and no macros: `Subscriber<T>` is
generic over the transaction, so one component can implement it more than
once. This is the job `uvm_analysis_imp_decl` exists to do in SystemVerilog.

The scoreboard checks in `check` and reports in `report`, which run after
`run`. Comparison policy lives on the transaction — `PartialEq` against the
predictor's output — which is where `do_compare()` puts it. Note the explicit
length check: a command with no result is a real failure, and zipping the two
streams would hide it by simply ending early.

```rust
#[derive(Component, Default)]
pub struct Scoreboard {
    #[port(subscribe)]
    cmd_in: SubscribePort<AluCommand>,
    #[port(subscribe)]
    result_in: SubscribePort<AluResult>,
    cmd_log: RustdvShared<CmdLog>,
    result_log: RustdvShared<ResultLog>,
    compared: usize,
    mismatches: usize,
}

impl Component for Scoreboard {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.cmd_in.subscribe(self.cmd_log.clone());
        self.result_in.subscribe(self.result_log.clone());
    }

    fn check(&mut self, ctx: &mut RustdvCtx, errors: &mut CheckSink) {
        let cmd_log = self.cmd_log.get();
        let result_log = self.result_log.get();

        for (cmd, actual) in cmd_log.cmds.iter().zip(result_log.results.iter()) {
            let expected = predict(cmd);
            self.compared += 1;
            if expected != *actual {
                self.mismatches += 1;
                ctx.info(&format!(
                    "scoreboard: in={cmd:?} out={actual:?} expected={expected:?} check=FAIL"
                ));
                errors.error(format!(
                    "scoreboard mismatch: {cmd:?} -> got {actual:?}, expected {expected:?}"
                ));
            } else {
                ctx.info(&format!(
                    "scoreboard: in={cmd:?} out={actual:?} expected={expected:?} check=PASS"
                ));
            }
        }

        if cmd_log.cmds.len() != result_log.results.len() {
            errors.error(format!(
                "scoreboard: saw {} commands and {} results",
                cmd_log.cmds.len(),
                result_log.results.len()
            ));
        }
        if self.compared == 0 {
            errors.error("scoreboard: nothing was compared".to_string());
        }
    }

    fn report(&mut self, ctx: &mut RustdvCtx) {
        ctx.info(&format!(
            "scoreboard: {} compared, {} mismatches",
            self.compared, self.mismatches
        ));
    }
}
```

### Coverage

Functional coverage is a second subscriber on the same command bus. It counts
the operations it saw and raises an error in `check` if any was never
exercised. The command monitor does not know it exists, and neither does the
scoreboard — which is the point of a broadcast.

```rust
#[derive(Default)]
struct CovCollector {
    seen: HashMap<Ops, usize>,
}

impl Subscriber<AluCommand> for CovCollector {
    fn write(&mut self, cmd: &AluCommand) {
        *self.seen.entry(cmd.op).or_insert(0) += 1;
    }
}

#[derive(Component, Default)]
pub struct Coverage {
    #[port(subscribe)]
    cmd_in: SubscribePort<AluCommand>,
    collector: RustdvShared<CovCollector>,
}

impl Component for Coverage {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.cmd_in.subscribe(self.collector.clone());
    }

    fn check(&mut self, _ctx: &mut RustdvCtx, errors: &mut CheckSink) {
        let seen = &self.collector.get().seen;
        for op in Ops::ALL {
            if !seen.contains_key(&op) {
                errors.error(format!("coverage: op {op:?} was never exercised"));
            }
        }
    }

    fn report(&mut self, ctx: &mut RustdvCtx) {
        let seen = &self.collector.get().seen;
        let mut parts: Vec<String> = Ops::ALL
            .iter()
            .map(|op| format!("{op:?}={}", seen.get(op).copied().unwrap_or(0)))
            .collect();
        parts.sort();
        ctx.info(&format!("coverage: {}", parts.join(" ")));
    }
}
```

`report` is what prints the `coverage: Add=5 And=5 Mul=5 Xor=5` line in the
transcript above.

### The driver

The driver pulls items from the sequencer and drives the BFM.
`get_next_item()` returns only when a sequence has an item ready *and* the
driver has asked for it — a rendezvous, not a queue — and `item_done()`
releases the sequence. This testbench compares the two observed streams rather
than routing answers back, so it passes `None` as the response.

```rust
#[derive(Component, Default)]
pub struct Driver {
    #[port(seq_item)]
    seq_item_port: SeqItemPort<AluCommand, AluResult>,
}

impl Component for Driver {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM")?;
        bfm.reset().await;
        loop {
            let item = self.seq_item_port.get_next_item().await;
            bfm.send_op(item.payload().clone()).await;
            self.seq_item_port.item_done(None);
        }
    }
}
```

### The sequences

A sequence is the stimulus as a program. It is **not** a component: no place
in the tree, no path, no phases. One method, `body`, and a `SeqCtx` to run it
against.

The gap between `start_item` and `finish_item` is the point. `start_item`
returns once the sequencer has granted this item its turn and the driver is
blocked waiting for its contents, so everything in between happens with the
driver committed and holding still. That is where late stimulus generation
lives, and it is why the rendezvous takes two calls rather than one.

All three sequences here differ only in how they fill the operands, so the
walk over the operations lives in one function and each sequence supplies a
`set_operands`:

```rust
trait Operands {
    fn set_operands(&mut self, rng: &mut Rng, cmd: &mut AluCommand);
}

async fn all_ops<S: Operands>(
    seq: &mut S,
    ctx: &mut SeqCtx<AluCommand, AluResult>,
    n: usize,
) -> Result<(), SeqError> {
    let mut rng = ctx.rng();
    for _ in 0..n {
        for op in Ops::ALL {
            let mut cmd = AluCommand { a: 0, b: 0, op };
            ctx.start_item(&mut cmd).await?;
            seq.set_operands(&mut rng, &mut cmd);
            ctx.finish_item(cmd).await?;
        }
    }
    Ok(())
}
```

`BaseSeq` is the type a test names. On its own it drives zeros — a legal
stimulus and a poor one; its job is to be the name the factory overrides.

```rust
#[derive(Default)]
pub struct BaseSeq;

impl Operands for BaseSeq {
    fn set_operands(&mut self, _rng: &mut Rng, _cmd: &mut AluCommand) {}
}

impl Sequence for BaseSeq {
    type Req = AluCommand;
    type Rsp = AluResult;

    async fn body(&mut self, ctx: &mut SeqCtx<AluCommand, AluResult>) -> Result<(), SeqError> {
        all_ops(self, ctx, 1).await
    }
}
```

`RandomSeq` randomizes the operands and walks every operation five times, so
coverage is guaranteed by construction rather than hoped for. `MaxSeq` sets
both operands to `0xff` — the corner a random test is unlikely to reach on its
own.

```rust
#[derive(Default)]
pub struct RandomSeq;

impl Operands for RandomSeq {
    fn set_operands(&mut self, rng: &mut Rng, cmd: &mut AluCommand) {
        cmd.a = rng.u8();
        cmd.b = rng.u8();
    }
}

impl Sequence for RandomSeq {
    type Req = AluCommand;
    type Rsp = AluResult;

    async fn body(&mut self, ctx: &mut SeqCtx<AluCommand, AluResult>) -> Result<(), SeqError> {
        all_ops(self, ctx, 5).await
    }
}
```

### The sequence item

The transaction is a plain struct. There is no rustdv base class: the standard
derives do the jobs `uvm_object` methods do — `Clone` is `do_copy`,
`PartialEq` is `do_compare`, `Debug` is `convert2string`.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Ops {
    Add = 1,
    And = 2,
    Xor = 3,
    Mul = 4,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AluCommand {
    pub a: u8,
    pub b: u8,
    pub op: Ops,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AluResult {
    pub result: u16,
}
```

The scoreboard's golden model is an ordinary function over that struct:

```rust
pub fn predict(cmd: &AluCommand) -> AluResult {
    let a = cmd.a as u16;
    let b = cmd.b as u16;
    let result = match cmd.op {
        Ops::Add => a + b,
        Ops::And => a & b,
        Ops::Xor => a ^ b,
        Ops::Mul => a * b,
    };
    AluResult { result }
}
```

### The BFM

The bus functional model is the only part of the testbench that touches
signals. It takes typed handles from the DUT, drives the `start`/`done`
handshake, and feeds three queues that the driver and the two monitors read:

```rust
pub struct TinyAluBfm {
    clk: LogicHandle,
    reset_n: LogicHandle,
    start: LogicHandle,
    done: LogicHandle,
    a: LogicHandle,
    b: LogicHandle,
    op: LogicHandle,
    result: LogicHandle,
    driver_q: Queue<AluCommand>,
    cmd_q: Queue<AluCommand>,
    result_q: Queue<AluResult>,
}
```

The BFM only ever *waits* on clock edges; it never generates one. The clock
comes from the RTL. That is what lets the same testbench run against an
emulator, where nothing in software can drive a clock.

## The book

**"Rust for RTL Verification"** is the third in Ray Salemi's series, after
[*The UVM Primer*](https://www.uvmprimer.com) (SystemVerilog) and
[*Python for RTL Verification*](https://a.co/d/0hTKAJvh). It assumes you are a
UVM verification engineer — from SystemVerilog or from Python; neither earlier
book is a prerequisite — and teaches you Rust chapter by chapter while
rebuilding the TinyALU testbench, versions 1.0 through 8.0.

Chapters 1–14 are Rust itself, taught against verification problems. Then the
Interlude puts the finished testbench in front of you, before the climb.
Chapters 15 onward build it: async and the simulator interface first, then the
methodology layer one capability at a time — components, configuration, the
factory, TLM, transactions — and finally sequences and the complete testbench.

Every figure in the book runs, and every simulation transcript in it is real
Icarus output — both facts are enforced by the regression suite rather than
asserted here.

* Source: [`book-pdf/src/`](book-pdf/src/) (mdBook; `mdbook build book-pdf`)
* The complete testbench, presented before the climb:
  [the Interlude](book-pdf/src/interlude-tinyalu-testbench.md)
* Runnable figures: [`output/examples/`](output/examples/README.md) — each
  chapter README maps its figures to code. The chapters 1–14 READMEs also carry
  a "Try it" Rust Playground link per figure, so you can run those in a browser
  with nothing installed — including the figures that fail to compile on
  purpose, where the error is the lesson.

## What's in this repository

| Path | Contents |
|---|---|
| [`rustdv/`](rustdv/) | The framework workspace: `rustdv-gpi-sys` → `rustdv-gpi` → `rustdv-sim` → `rustdv-methodology` → `rustdv`, plus `tinyalu_tb` |
| [`rustdv/tinyalu_tb/`](rustdv/tinyalu_tb/) | The shipped TinyALU testbench walked above |
| [`book-pdf/src/`](book-pdf/src/) | The manuscript (mdBook markdown) |
| [`output/examples/`](output/examples/README.md) | Every book figure as runnable code |
| [`output/regression/`](output/regression/TESTING.md) | The regression system guarding all of it |
| [`sim/`](sim/README.md) | TinyALU DUT, `run_rustdv.sh`, and simulator smoke tests |
| [`skills/`](skills/) | AI verification skills: spec + RTL → testbench → verified coverage report |
| [`docker/`](docker/) | Reproducible environment: Rust + Icarus + Verilator |
| [`STATUS.md`](STATUS.md) | Implementation history and the deviations log |
| [`TOUR.md`](TOUR.md) | The ten-minute walk-around, for humans and AI alike |

## Simulators

| | Status |
|---|---|
| Icarus Verilog | full simulation; runs in CI, and every transcript in the book comes from it |
| Verilator | full two-state simulation; FAST, DEBUG/FST, scheduler, and mutation paths run in CI |
| VCS / Questa / Xcelium | same script (`sim/run_smoke.sh vcs\|questa\|xcelium`); licenses can't live in public CI, so license-holders run the identical regression locally |
| EDA Playground | HDL side only — it has no Rust toolchain; see [sim/README.md](sim/README.md) |

## Testing

`output/regression/regress.py` is the single gate, wired into the `git push`
hook by `--install-hook`. It runs four suites: framework unit tests, the
book↔examples sync check, every figure's behavior against its blessed golden,
and the simulator tests.

```sh
python3 output/regression/regress.py              # everything
python3 output/regression/regress.py --suite unit # no simulator needed, seconds
python3 output/regression/regress.py --list       # every test id
```

Two of those checks exist to catch documentation drift, which is the failure
this project pays for most:

```sh
python3 output/regression/verify-book-listings.py  # every ch15-40 listing is real code
bash output/regression/verify-transcripts.sh       # every transcript is real output
```

The second one also checks this README: the transcript above and the listings
above are compared against a fresh run and against `tinyalu_tb`'s sources, so
they cannot quietly go stale.

The framework's own tests come in three tiers — no-simulator tests, targeted
simulator tests, and compile-fail cases that each assert their specific
`error[E….]` code.
[`output/regression/TESTING.md`](output/regression/TESTING.md) is the operating
manual.

The checking has teeth: with the DUT's XOR deliberately corrupted to OR, the
scoreboard flags every affected transaction and the regression fails.
That mutation is itself a test in the suite.

## Contributing

rustdv is in its release and evaluation phase.
[Discussions](https://github.com/rustdv/rustdv/discussions) are open and issues
and pull requests are not — the most useful thing you can do right now is say
what you think of the library. See [CONTRIBUTING.md](CONTRIBUTING.md).

## Credits

* Ray Salemi — author of rustdv and of *Rust for RTL Verification*.
* [cocotb](https://cocotb.org) and [pyuvm](https://github.com/pyuvm/pyuvm),
  whose designs rustdv follows on the simulator and methodology sides.
* The IEEE 1800.2 standard and the SystemVerilog UVM, the methodology rustdv
  is inspired by.

## License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
* MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
