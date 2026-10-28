# Chapter 40: The Complete TinyALU Testbench

The Interlude showed you this testbench before you could read it, and asked only for recognition. Thirty-nine chapters later, the deal completes: the same `tinyalu_tb` crate, walked with nothing left on faith. This is also the chapter to use as a template — the shipped testbench in the rustdv repository, the one its regression runs, organized the way a real project's would be.

## Project layout

```text
# Figure 1: The testbench crate

tinyalu_tb/src/
├── tinyalu_tb.rs    the crate root: BaseTest, RandomTest, MaxTest
├── alu_item.rs      transactions, Ops, and the predictor — plus unit tests
├── alu_bfm.rs       the BFM: pins, protocol loops, queue-fed methods
├── sequences.rs     BaseSeq, RandomSeq, MaxSeq over one shared walk
├── components.rs    Driver, two monitors, Scoreboard, Coverage
└── env.rs           AluEnv: build, connect, and two ConfigDb knobs
```

One file per concern, and the crate root named after the crate — no file in this project is named `lib.rs`, so a stack trace or a log line always says *which* crate it came from. `alu_item.rs` ends with `#[cfg(test)]` unit tests: the predictor and the transaction derives are checked by `cargo test` on every build, no simulator anywhere — Chapter 14's capability, earning its keep in shipping code.

## The tests

```rust
// Figure 2: BaseTest — build files the BFM; run starts whatever the factory chose
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

Every line is a chapter. `build` constructs the BFM from the DUT handle and files it in the ConfigDb under `"*"` — the whole tree gets this one, and there is no singleton anywhere in the crate: the database asserts "one BFM under this name for this subtree," which is a promise a two-interface testbench can keep, where a singleton's "one BFM in the world" is not (Chapter 25). `run` finds the sequencer by name, builds its sequence *through the factory*, and starts it (Chapter 36).

No clock appears anywhere in it, and that is the point Chapter 19 made: `sim/hdl/tinyalu.sv` clocks itself, exactly as the chapters' copy of the design does, so this testbench only ever *waits* on edges. A BFM built that way ports to an emulation transactor unchanged; one that drives edges does not. The shipped testbench is not an exception to the discipline the book taught — it is the discipline, running.

One detail does differ from the chapters. The end of stimulus is `bfm.wait_idle().await`, not the twenty-clock flush of Chapters 34 and 36. Counting clocks worked, but it encoded a magic number — twenty, because the multiply is slowest — that would quietly go stale if the DUT grew a slower operation. `wait_idle` asks the *protocol* instead: it watches for the driver queue empty and the handshake quiet for two consecutive falling edges (two, because a command already popped but not yet driven must not fool it), then gives the monitors one more edge to flush. Same job, no magic number, and it moves with the DUT.

```rust
// Figure 3: Two tests, one testbench, no new components
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

#[rustdv::test(timeout_time = 500, timeout_unit = "us")]
#[derive(Component, Default)]
struct MaxTest {
    #[component]
    inner: RustdvComp,
}

impl Component for MaxTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        set_seq_override::<BaseSeq, MaxSeq>();
        self.inner = BaseTest::new_comp();
    }
}
```

Each registered test is one override plus `BaseTest` — Chapter 36's pattern, shipping. `RandomSeq` runs every operation five times with seeded random operands, so coverage is guaranteed by construction rather than hoped for; `MaxSeq` drives the `0xff op 0xff` corner once each — the corner random stimulus is unlikely to find on its own. Adding a third stimulus pattern to this testbench is a sequence and a six-line test; no component changes, which is the measure the whole book has been building toward. The `timeout_time` attributes are the runner's safety net: a hung handshake fails loudly at 500 microseconds instead of running forever.

## The environment's two knobs

The Interlude showed `AluEnv` in full. The walk stops at its opening lines, because they are the ConfigDb doing structural work:

```rust
// Figure 4: Two choices a test can make from outside (env.rs, build)
    fn build(&mut self, ctx: &mut RustdvCtx) {
        let activity: Active = ConfigDb::get(Some(ctx), "", "IS_ACTIVE").unwrap_or(Active::Active);
        self.is_active = activity == Active::Active;
        self.with_coverage = ConfigDb::get(Some(ctx), "", "WITH_COVERAGE").unwrap_or(true);

        self.seqr = Sequencer::new();
        ConfigDb::set(None, "*", "SEQR", self.seqr.handle());

        if self.is_active {
            self.driver = Driver::create_comp();
        }
        // ...
    }
```

`IS_ACTIVE` is the UVM's active/passive knob, typed: `Active` is an enum, so an illegal value cannot be filed, and `unwrap_or(Active::Active)` makes the ordinary case configure nothing. Look at what a passive env *is*: the driver slot is simply left empty — no driver constructed and told not to drive, no `None` checks downstream, just a component that does not exist and a `connect` that (three lines later) skips its wiring. This is why `build` had to be a phase: whether the driver exists is decided by configuration that must arrive *before* the children do. `WITH_COVERAGE` works the same way for the coverage collector. And this is also the reason every component takes no constructor arguments — the factory's makers cannot supply any (Chapter 29), so everything a component needs arrives by name after it exists, which is exactly what makes the whole tree overridable.

## The scoreboard's guards

The Interlude showed the scoreboard whole; the walk stops at the end of its `check`, on two guards that a lesser scoreboard omits:

```rust
// Figure 5: A scoreboard that cannot pass vacuously (components.rs, check)
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
```

The comparison loop zips commands against results, and a zip cannot complain about what never arrived — the shorter stream just ends the comparison, which is how a scoreboard passes while checking less than it saw (Chapter 34's warning). The first guard makes the count mismatch an error in its own right. The second refuses a clean pass with zero comparisons — the oldest trap in verification, a checker that never ran. Both guards exist because the failure they catch is silent by construction: the scoreboard owns its own storage (Chapter 32's rule — two `RustdvShared` logs behind two `SubscribePort`s), so a scoreboard that quietly stopped receiving would look identical to one that passed. The standing proof that the checking has teeth is a mutation run — corrupt the DUT's XOR into an OR and the scoreboard flags every affected transaction — verification of the verification, and these guards are what make that check stay meaningful.

## The run

```text
# Figure 6: The shipped testbench running

      0.00ns INFO     rustdv: found 2 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running RandomTest (1/2)  [rustdv/tinyalu_tb/src/tinyalu_tb.rs:77]
     70.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 296 }
     70.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 193, b: 103, op: Add }
     90.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 10 }
     90.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 94, b: 11, op: And }
    110.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 57 }
    110.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 185, b: 128, op: Xor }
    130.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 165, b: 117, op: Mul }
    160.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 19305 }
    180.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 168, b: 150, op: Add }
    180.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 318 }
    200.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 97, b: 254, op: And }
    200.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 96 }
    220.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 192, b: 138, op: Xor }
    220.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 74 }
    240.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 168, b: 59, op: Mul }
    270.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 9912 }
    290.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 340 }
    290.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 99, b: 241, op: Add }
    310.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 8 }
    310.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 238, b: 8, op: And }
    330.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 218 }
    330.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 70, b: 156, op: Xor }
    350.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 205, b: 172, op: Mul }
    380.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 35260 }
    400.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 159, b: 247, op: Add }
    400.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 406 }
    420.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 53, b: 171, op: And }
    420.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 33 }
    440.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 39, b: 138, op: Xor }
    440.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 173 }
    460.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 132, b: 186, op: Mul }
    490.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 24552 }
    510.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 137 }
    510.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 109, b: 28, op: Add }
    530.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 4 }
    530.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 23, b: 12, op: And }
    550.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 52 }
    550.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 245, b: 193, op: Xor }
    570.00ns INFO     [RandomTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 24, b: 60, op: Mul }
    600.00ns INFO     [RandomTest.inner.env.result_mon]: result_monitor: AluResult { result: 1440 }
    630.00ns INFO     [RandomTest.inner]: sequence complete
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 193, b: 103, op: Add } out=AluResult { result: 296 } expected=AluResult { result: 296 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 94, b: 11, op: And } out=AluResult { result: 10 } expected=AluResult { result: 10 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 185, b: 128, op: Xor } out=AluResult { result: 57 } expected=AluResult { result: 57 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 165, b: 117, op: Mul } out=AluResult { result: 19305 } expected=AluResult { result: 19305 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 168, b: 150, op: Add } out=AluResult { result: 318 } expected=AluResult { result: 318 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 97, b: 254, op: And } out=AluResult { result: 96 } expected=AluResult { result: 96 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 192, b: 138, op: Xor } out=AluResult { result: 74 } expected=AluResult { result: 74 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 168, b: 59, op: Mul } out=AluResult { result: 9912 } expected=AluResult { result: 9912 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 99, b: 241, op: Add } out=AluResult { result: 340 } expected=AluResult { result: 340 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 238, b: 8, op: And } out=AluResult { result: 8 } expected=AluResult { result: 8 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 70, b: 156, op: Xor } out=AluResult { result: 218 } expected=AluResult { result: 218 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 205, b: 172, op: Mul } out=AluResult { result: 35260 } expected=AluResult { result: 35260 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 159, b: 247, op: Add } out=AluResult { result: 406 } expected=AluResult { result: 406 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 53, b: 171, op: And } out=AluResult { result: 33 } expected=AluResult { result: 33 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 39, b: 138, op: Xor } out=AluResult { result: 173 } expected=AluResult { result: 173 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 132, b: 186, op: Mul } out=AluResult { result: 24552 } expected=AluResult { result: 24552 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 109, b: 28, op: Add } out=AluResult { result: 137 } expected=AluResult { result: 137 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 23, b: 12, op: And } out=AluResult { result: 4 } expected=AluResult { result: 4 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 245, b: 193, op: Xor } out=AluResult { result: 52 } expected=AluResult { result: 52 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 24, b: 60, op: Mul } out=AluResult { result: 1440 } expected=AluResult { result: 1440 } check=PASS
    630.00ns INFO     [RandomTest.inner.env.scoreboard]: scoreboard: 20 compared, 0 mismatches
    630.00ns INFO     [RandomTest.inner.env.coverage]: coverage: Add=5 And=5 Mul=5 Xor=5
    630.00ns INFO     RandomTest PASSED
    630.00ns INFO     running MaxTest (2/2)  [rustdv/tinyalu_tb/src/tinyalu_tb.rs:92]
    700.00ns INFO     [MaxTest.inner.env.result_mon]: result_monitor: AluResult { result: 510 }
    700.00ns INFO     [MaxTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 255, b: 255, op: Add }
    720.00ns INFO     [MaxTest.inner.env.result_mon]: result_monitor: AluResult { result: 255 }
    720.00ns INFO     [MaxTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 255, b: 255, op: And }
    740.00ns INFO     [MaxTest.inner.env.result_mon]: result_monitor: AluResult { result: 0 }
    740.00ns INFO     [MaxTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 255, b: 255, op: Xor }
    760.00ns INFO     [MaxTest.inner.env.cmd_mon]: cmd_monitor: AluCommand { a: 255, b: 255, op: Mul }
    790.00ns INFO     [MaxTest.inner.env.result_mon]: result_monitor: AluResult { result: 65025 }
    820.00ns INFO     [MaxTest.inner]: sequence complete
    820.00ns INFO     [MaxTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 255, b: 255, op: Add } out=AluResult { result: 510 } expected=AluResult { result: 510 } check=PASS
    820.00ns INFO     [MaxTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 255, b: 255, op: And } out=AluResult { result: 255 } expected=AluResult { result: 255 } check=PASS
    820.00ns INFO     [MaxTest.inner.env.scoreboard]: scoreboard: in=AluCommand { a: 255, b: 255, op: Xor } out=AluResult { result: 0 } expected=AluResult { result: 0 } check=PASS
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

Twenty-four operations across two tests, each line stamped with the path of the component that wrote it, ending in the summary table. This is the same run the repository's regression asserts — the counts above are checked mechanically on every push, so the transcript you produce and the one in this book can only agree.

## Inspection, waveforms, and RTL coverage

The Icarus run above is the four-state reference. From the repository root, `sim/run_rustdv.sh release verilator` runs the same Rust testbench with the shared Verilator host. Its default FAST mode exposes the DUT's top-level ports without waveform or coverage instrumentation. The launcher exports `RUSTDV_TOP=tinyalu`, selecting the DUT even when other root modules exist.

For internal visibility, set `RUSTDV_VERILATOR_MODE=inspect` and use `RUSTDV_VERILATOR_CONTROL_FILE` to name a `.vlt` file selecting signals with `public_flat_rw`. The TinyALU launcher supplies `sim/verilator-debug.vlt` by default. INSPECT adds those VPI-visible internals without recording waves. DEBUG uses the selected visibility and writes an FST from time zero; `RUSTDV_FST` chooses its destination. Both DEBUG and RECORD need the lz4 development headers and library used by Verilator's FST writer.

RECORD compiles waveform instrumentation but opens no file until the testbench requests a capture. Under `rustdv::sim::simulator_trace`, `start(path)`, `flush()`, `stop()`, and `status()` control that capture and report its state. Each operation runs inside `service_read_only(...)`: the supplied synchronous closure executes on the simulator thread after writes and combinational logic settle. Simulation time is held until it returns. A service that waits on another OS thread therefore needs a wall-clock deadline or cooperative cancellation; a simulation-time timeout cannot release it. `host_present()` and `host_format()` allow a service to choose its backend before entering ReadOnly. The API reports unavailable capabilities and invalid lifecycle use as errors. The supplied Verilator host records FST; the generic Rust API does not assume every simulator can do so.

RTL coverage answers a different question from the `Coverage` component in this testbench. That component counts observed operations: did the test exercise ADD, AND, XOR, and MUL? Verilator's COVERAGE build instruments RTL lines and expressions. Run `RUSTDV_VERILATOR_MODE=coverage RUSTDV_COVERAGE_FILE=/tmp/rustdv-coverage/tinyalu.dat sim/run_rustdv.sh release verilator`, then inspect the database with `verilator_coverage --report summary /tmp/rustdv-coverage/tinyalu.dat`. The host writes it after final blocks and end-of-simulation callbacks, retaining it even when a RustDV check fails. It supplements the operation collector; setting `WITH_COVERAGE` controls only that functional collector.

Waveform and RTL-coverage builds are separate profiles. The scripts reject FST in COVERAGE mode and reject a coverage destination in other modes. These profiles use the repository's launchers and C++ simulator host in addition to the Rust crates. The repository's simulator guide documents their controls, and its targeted regressions exercise capture windows, settled inspection, and coverage output. The testbench above needs none of these optional services to run.

## What to take with you

Use this crate as the template it is. Transactions are plain structs with derives and a hand-written `Display`, plus unit tests beside them (Chapter 35, Chapter 14). The BFM owns the pins, speaks the protocol on falling edges, and exposes queue-fed async methods — and hides one hand-written `Debug` impl so `ConfigDb::dump` names it as `TinyAluBfm` instead of dumping eight signal handles (a small kindness Chapter 28 makes you glad of). Sequences share one walk and vary one method; components take nothing at construction and ask the ConfigDb for what they need; the env reads its knobs before building, builds through the factory, and wires everything in `connect` with the one idiom; the tests are overrides on a shared base. Every piece was a chapter; together they are a working, checked, regression-guarded testbench — and now they are yours.

The climb the Interlude promised is done. What follows are the appendices: the maps back to the earlier books, the idiom translations from both source languages, and the full catalog of what rustdv provides.
