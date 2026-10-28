# Chapter 17: Simulating with rustdv-sim

Sixteen chapters in, we touch a design. This chapter connects everything Part II has built — futures, the executor, tasks — to an actual DUT in an actual simulator: getting handles to signals, reading and writing values, and waiting on clock edges. By its end you will have verified a piece of hardware in Rust, which means Chapter 18 gets to verify the piece of hardware this book is actually about.

> **In the UVM...** we reached the DUT through a handle. SystemVerilog testbenches got a virtual interface, delivered through the config database: `vif.reset_n <= 0` set a signal, `@(negedge vif.clk)` synchronized with the design. cocotb handed the test the top of the hierarchy as an argument named `dut`: `dut.reset_n.value = 0`, `get_int(dut.count)`, `await FallingEdge(dut.clk)` — same jobs, Python spellings.

## One continuity story before the code

Under cocotb sits a C++ layer called the GPI — the *Generic Procedural Interface* — that abstracts the simulators' native APIs (VPI for Verilog simulators, VHPI and FLI for VHDL) behind one set of calls: get a handle by name, read a value, write a value, register a callback. A decade of simulator quirks lives in that layer, and it is the unsung reason cocotb runs everywhere.

rustdv speaks to the simulator through the same procedural interfaces, and its layering copies cocotb's on purpose: a `-sys` crate of raw simulator bindings at the bottom, a safe wrapper crate above it that turns null pointers into `Err` and untracked lifetimes into owned types, and `rustdv-sim` — everything you met in Chapters 15 and 16 — on top.¹ The mechanical difference from cocotb is *what* the simulator loads: where cocotb's makefiles arranged for the simulator to start an embedded Python interpreter that imports your test module, a rustdv testbench **compiles to a shared library** that the simulator loads directly, the way it would load any VPI plugin. That is the story behind the two ceremony lines from Chapter 15: `vpi_bootstrap!()` exports the entry points the simulator calls at startup, and the chapter run scripts hand `vvp` (Icarus's runtime) our compiled library alongside the compiled design. No interpreter starts, because there is nothing to interpret; the testbench *is* native code, checked before the simulator ever ran.

> ¹ The supplied Icarus and Verilator backends both bind VPI directly. Icarus provides the four-state reference runs in this book; Verilator provides two-state functional simulation. The cocotb layering explains the names, but it does not imply that rustdv implements cocotb's VHPI or FLI backends.

## Verifying a counter

Our DUT for the day, reprinted from the Python book down to the timescale:

```text
# Figure 1: A SystemVerilog counter

`timescale 1ns/1ns
module counter(input bit clk,
               input bit reset_n,
               output byte unsigned count);
   always @(posedge clk)
     count <= reset_n ? count + 1 : 'b0;
endmodule
```

Synchronous reset: hold `reset_n` low and `count` clears on each clock; raise it and the counter counts. Two behaviors, two tests.

## The dut handle, without the magic

A rustdv test receives a `RustdvCtx`, and `ctx.dut()` returns a `HierarchyHandle` to the top of the hierarchy — the counterpart of cocotb's `dut` argument. Getting at a signal is where the languages part company. In Python, `dut.reset_n` worked because `__getattr__` invented the attribute on demand by asking the simulator — pure runtime dynamism, and if you typed `dut.rst_n`, you found out via `AttributeError` deep into the run. Rust cannot invent struct fields at runtime, and would not want to: rustdv's spelling is `dut.signal("reset_n")`, and it returns — you knew before you read it — a `Result`:

```rust
// Figure 2: A typo'd signal name is an Err, not a surprise

#[rustdv::test]
async fn name_lookup(ctx: RustdvCtx) -> Result<(), TestError> {
    // Show what child()/signal() return
    let dut = ctx.dut();
    let good = dut.signal("reset_n");
    let bad = dut.signal("rst_n"); // the classic typo
    log::info(&format!("reset_n -> {good:?}"));
    log::info(&format!("rst_n   -> {bad:?}"));
    Ok(())
}
```

```text
--
      0.00ns INFO     reset_n -> Ok(LogicHandle("counter.reset_n"))
      0.00ns INFO     rst_n   -> Err(NotFound { name: "rst_n", scope: "counter" })
```

In real code nobody matches on these by hand — you write `let reset_n = dut.signal("reset_n")?;` and the `?` from Chapter 9 turns a bad name into an immediate, test-failing error naming the signal *and* the scope, at time zero, before anything subtle has had a chance to happen. `dut.child("name")` is the general form for descending the hierarchy (submodules and all); `signal()` is `child()` plus an is-it-a-signal check.²

The launcher selects the DUT root by exporting `RUSTDV_TOP` with its module name. Without that variable, rustdv uses the first top-level module VPI returns. Select the root explicitly when another top-level module also exists, such as a waveform dumper; the shipped launchers already do this.

The prelude brings the handle traits into scope: `HandleChildren` supplies lookup and traversal, and `HandleEvent` supplies edge triggers. A general `child()` lookup returns a `SimHandle`; `as_signal()?` or `as_hierarchy()?` checks its kind and reports a `HandleError` on a mismatch. To call a handle's `name()` or `full_name()`, also import `rustdv::sim::handle::HandleBase`.

The handle you get back, `LogicHandle`, is typed: it reads and writes logic values and offers edge triggers, and that is all it does. Two figures from the Python chapter dissolve entirely here — the `sys.path` bootstrap (Chapter 14 deleted it with prejudice) and the logger setup (`rustdv::log` initializes itself, and Chapter 26 covers levels). The third `tinyalu_utils` resident, `get_int()`, is worth porting for the pleasure of it:

```rust
// Figure 3: get_int() ports to one line of unwrap_or

fn get_int(signal: &LogicHandle) -> u64 {
    // x or z becomes 0, as tinyalu_utils decided
    signal.get_u64().unwrap_or(0)
}
```

`get_u64()` returns `Result<u64, ValueError>` because a signal holding `x` or `z` has no integer value — the fact Python expressed by `int()` raising `ValueError`, and SystemVerilog expressed by letting the `x` ride silently into your arithmetic. The Python version needed a four-line `try/except`; Rust's `unwrap_or(0)` says "the value, or zero" in one expression. The policy remains testbench-specific: a testbench that would rather die on `x` writes `get_u64()?` instead, and either way the x-handling decision is visible in the code, per read, instead of ambient in the semantics.

## Choosing the value representation

The counter fits in `u64`, but the handle API also provides `get_bool()`, `get_u8()`, `get_u16()`, `get_u32()`, and `get_u128()`, with matching setters such as `set_u8()`. These integer reads are unsigned. They reject X/Z and reject a signal wider than the requested type; `get_bool()` requires one bit. `get_bigint()` and `set_bigint(&value)` use `BigUint` for arbitrary-width buses. A width error and an unknown bit are both errors, so a helper such as `get_int()` above deliberately maps either one to zero. Use `?` when either should fail the test.

To keep all four states, read `get_logic()?` into a `LogicArray` and write with `set_logic(&value)`. `get_binstr()?` and `LogicArray::to_binstr()` produce MSB-first strings, while indexed access is LSB-first: `value[0]` returns the least significant bit as an `Option<Logic>`. `u64::try_from(&value)` rejects X/Z but keeps the low bits of an oversized array; use the handle's typed getter when you want width checking as well.

The same lookup scheme covers variables that are not logic vectors. `dut.real("gain")?` returns a `RealHandle` with `f64` values; `dut.string("label")?` returns a `StringHandle` with owned Rust strings. Both provide `get()`, scheduled `set()`, and immediate `set_now()`; string writes return a `Result` and reject embedded NUL bytes. `dut.aggregate("record")?` returns an `AggregateHandle` whose `child()` and `children()` reach unpacked struct or union members, including nested aggregates. Read and write each member through its typed handle. A packed aggregate exposed as a logic vector uses `LogicHandle`.

Each of these lookups depends on the simulator exposing that object through VPI. Unpacked aggregates require member-level VPI support; the framework's variable-type regression exercises this on Verilator. Verilator is two-state, so it cannot supply the X/Z behavior demonstrated by the book's Icarus runs.

## Testing reset

```rust
// Figure 4: Starting the clock, lowering reset

#[rustdv::test]
async fn no_count(ctx: RustdvCtx) -> Result<(), TestError> {
    // Test no count if reset is 0
    let dut = ctx.dut();
    let clk = dut.signal("clk")?;
    Clock::new(&clk, SimDuration::ns(2)).start();
    let reset_n = dut.signal("reset_n")?;
    reset_n.set_u64(0);
```

`Clock::new(&clk, SimDuration::ns(2)).start()` is `Clock(dut.clk, 2, units="ns")` plus `start_soon` in one breath: it spawns the square-wave task and returns its handle, which we let fall — a free-running clock is the legitimate fire-and-forget from Chapter 16. `set_u64(0)` is the assignment `dut.reset_n.value = 0`, and it inherits cocotb's careful semantics: the write is *scheduled*, applied at the simulator's next read-write phase, not jammed into the middle of a delta cycle. (An immediate variant, `set_u64_now`, exists for the rare moment you need it — cocotb's `setimmediatevalue`, made greppable.)

This is the only chapter that starts a clock, so it is worth saying why now rather than letting you notice its absence later. The counter here is a bare design with `clk` as an input, and something has to drive it. The TinyALU, from Chapter 18 onward, clocks itself — and every testbench in the rest of the book therefore only ever *waits* on edges. That is a deliberate discipline, not a property of the design: a BFM that waits on edges is the same code whether a simulator or an emulator supplies the edges, while one that drives them can only ever run in a simulator. `Clock` stays in the toolkit for the designs that need it. You will not see it again.

```rust
// Figure 5: Wait for five clocks and check the output

    for _ in 0..5 {
        clk.rising_edge().await;
    }
    let count = get_int(&dut.signal("count")?);
    log::info(&format!("After 5 clocks count is {count}"));
    assert_eq!(count, 0);
    Ok(())
}
```

```text
--
      8.00ns INFO     After 5 clocks count is 0
      8.00ns INFO     no_count PASSED
```

Triggers ride on the handles now — `clk.rising_edge().await`, `clk.falling_edge().await`, `clk.value_change().await` — which is where cocotb 2.x was headed anyway with `signal.rising_edge`. One import vanished in the move: there is no `ClockCycles` in rustdv, because `for _ in 0..5 { clk.rising_edge().await; }` *is* the loop, visible, and needing no `rising=` keyword to flip its polarity — you call the edge you mean. The assertion at the end follows Chapter 9's taxonomy: `assert_eq!` marks a claim whose failure means the test fails, and the runner scores a panicking test as FAILED with the assertion's message in the log.

## Checking that the counter counts

Set and sample on the falling edge — the DUT works on the rising edge, so the quiet half-cycle is ours, the discipline every UVM BFM has always followed and every BFM in this book will too:

```rust
// Figure 6: Testing that the counter counts

#[rustdv::test]
async fn three_count(ctx: RustdvCtx) -> Result<(), TestError> {
    // Test that we count up as expected
    let dut = ctx.dut();
    let clk = dut.signal("clk")?;
    Clock::new(&clk, SimDuration::ns(2)).start();
    let reset_n = dut.signal("reset_n")?;
    reset_n.set_u64(0);
    clk.falling_edge().await;
    reset_n.set_u64(1);
    for _ in 0..3 {
        clk.falling_edge().await;
    }
    let count = get_int(&dut.signal("count")?);
    log::info(&format!("After 3 clocks, count is {count}"));
    assert_eq!(count, 3);
    Ok(())
}
```

```text
--
     15.00ns INFO     After 3 clocks, count is 3
     15.00ns INFO     three_count PASSED
```

## Reading after the design settles

An edge tells you that a signal changed; it does not promise that all the logic reacting to that edge has finished. `read_write().await` waits for the writable synchronization point, where buffered writes are applied before its waiters resume. `read_only().await` waits until pending writes and combinational logic have settled, which is the point to sample an output that depends on them. The counter tests above sample on the opposite clock edge, so they do not need another synchronization step.

ReadOnly is a sampling region. Driving a signal, or awaiting another ReadOnly or ReadWrite there, is illegal. Wait for a later time step before driving again; `next_time_step().await` waits for the simulator's next time step, while a `Timer` requests a particular delay. Chapter 40 also uses the settled ReadOnly point to explain optional inspection and waveform services.

## A common coroutine mistake, demoted

The Python chapter closed with a warning: forget the `await` on a trigger and the simulator will create the coroutine, never run it, and let your test sail on without waiting — a bug you diagnose from a testbench that "doesn't seem to be advancing." Let's make the same mistake in Rust:

```rust
// Figure 7: Forgetting the await is now a compiler warning

#[rustdv::test]
async fn oops(ctx: RustdvCtx) -> Result<(), TestError> {
    // Demonstrate the coroutine mistake
    let dut = ctx.dut();
    let clk = dut.signal("clk")?;
    Clock::new(&clk, SimDuration::ns(2)).start();
    clk.rising_edge(); // forgot .await
    log::info("Did not await");
    Ok(())
}
```

```text
--
warning: unused `Edge` that must be used
  --> ch17-simulating-with-rustdv-sim/src/ch17_simulating_with_rustdv_sim.rs:77:5
   |
77 |     clk.rising_edge(); // forgot .await
   |     ^^^^^^^^^^^^^^^^^
   |
   = note: triggers do nothing unless you .await them
```

The program still compiles and still exhibits the bug — time does not advance, same as a forgotten `await` in Python. The difference is that *the compiler told you*, at the exact line, with a note written for this exact mistake, before any simulation ran. Chapter 15 taught why the bug exists (a future does nothing until polled; laziness is structural); Rust's `#[must_use]` machinery turns that structural fact into a diagnostic. Earlier books asked you to keep this mistake in mind. This book asks you to keep your build log clean, which is easier.

## Summary

This chapter put rustdv-sim in front of a simulator. The testbench compiles to a shared library the simulator loads — no interpreter, no environment, the continuity with cocotb living one layer down in the procedural interfaces both frameworks speak. `ctx.dut()` yields a `HierarchyHandle`; `dut.signal("name")?` looks up signals dynamically and returns `Result`, converting the classic typo from a mid-run `AttributeError` into a time-zero error with the scope and name attached. `LogicHandle` reads with `get_u64()` (a `Result`, because `x` and `z` are real) and writes with `set_u64()` (scheduled, applied at the read-write phase, as cocotb taught us all). Triggers are methods on handles — `rising_edge()`, `falling_edge()`, `value_change()` — awaited in ordinary loops that replace `ClockCycles`. `Clock` ports the clock generator. And the forgotten-`await` mistake, which Python discovered at runtime by symptom, is now a targeted compiler warning.

The value representation is a choice at the read: unsigned integers, arbitrary-width `BigUint`, or four-state `LogicArray`. Real/string variables and aggregate members use their own typed handles. The prelude supplies traversal and edge methods, `RUSTDV_TOP` selects the root, and ReadOnly provides a settled sampling point.

The counter was practice. Next chapter, the TinyALU comes back — same DUT, same `start`/`done` protocol, same timing diagram — and gets its first Rust testbench: version 1.0, a single loop with random operands, a prediction function, and a check. The climb proper begins.
