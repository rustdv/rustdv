//! `trig_` — the trigger layer, which nothing else tests directly.
//!
//! Every chapter example awaits a `Timer` or a clock edge, so a broken
//! trigger fails twenty testbenches at once and names none of them. These
//! tests fail one at a time, with the mechanism in the test's name.

use std::cell::Cell;
use std::rc::Rc;

use rustdv::prelude::*;

use crate::steps_per_ns;

// ---------------------------------------------------------------------------
// Timer
// ---------------------------------------------------------------------------

// `Timer::ns(n)` advances exactly n, and consecutive timers accumulate.
//
// The claim every transcript in the book rests on: a figure that says
// "@20ns" is only right if this is.
#[rustdv::test]
async fn trig_timer_advances_exactly(_ctx: RustdvCtx) -> Result<(), TestError> {
    let t0 = sim_time_ns();
    Timer::ns(10).await;
    let dt = sim_time_ns() - t0;
    check!(dt == 10.0, "Timer::ns(10) advanced {dt} ns");

    let t1 = sim_time_ns();
    Timer::ns(3).await;
    Timer::ns(7).await;
    let dt = sim_time_ns() - t1;
    check!(dt == 10.0, "3ns then 7ns advanced {dt} ns, not 10");
    Ok(())
}

// The two time functions describe the same instant.
//
// `sim_time_ns` is a convenience over `sim_time_steps`; if they ever
// disagreed, every log line would be right and every measurement wrong.
#[rustdv::test]
async fn trig_time_units_agree(_ctx: RustdvCtx) -> Result<(), TestError> {
    let per_ns = steps_per_ns().await;
    check!(per_ns > 0, "1 ns advanced {per_ns} time steps");

    let (s0, n0) = (rustdv::sim_time_steps(), sim_time_ns());
    Timer::ns(7).await;
    let steps = rustdv::sim_time_steps() - s0;
    let ns = sim_time_ns() - n0;
    check!(
        steps == 7 * per_ns && ns == 7.0,
        "7ns read as {steps} steps ({} expected) and {ns} ns",
        7 * per_ns
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Edges
// ---------------------------------------------------------------------------

// Rising and falling fire on their own transitions, and on no others.
#[rustdv::test]
async fn trig_edges_fire_on_the_right_transition(ctx: RustdvCtx) -> Result<(), TestError> {
    let clk = ctx.dut().signal("clk")?;
    Clock::new(&clk, SimDuration::ns(2)).start();

    // Land on a known edge first, so the measurement below starts from a
    // defined phase of the clock rather than from wherever the test began.
    clk.rising_edge().await;
    let t0 = sim_time_ns();
    for _ in 0..5 {
        clk.rising_edge().await;
    }
    let dt = sim_time_ns() - t0;
    check!(dt == 10.0, "five 2ns rising edges took {dt} ns");
    check!(clk.is_high(), "rising_edge returned with the clock low");

    // A falling edge is half a period past a rising one.
    let t1 = sim_time_ns();
    clk.falling_edge().await;
    let dt = sim_time_ns() - t1;
    check!(dt == 1.0, "the falling edge came {dt} ns after the rising edge");
    check!(clk.is_low(), "falling_edge returned with the clock high");
    Ok(())
}

// `value_change` sees both edges: two per period, where each directed
// trigger sees one.
#[rustdv::test]
async fn trig_value_change_sees_both_edges(ctx: RustdvCtx) -> Result<(), TestError> {
    let clk = ctx.dut().signal("clk")?;
    Clock::new(&clk, SimDuration::ns(2)).start();

    clk.rising_edge().await;
    let t0 = sim_time_ns();
    for _ in 0..4 {
        clk.value_change().await;
    }
    let dt = sim_time_ns() - t0;
    check!(dt == 4.0, "four value changes on a 2ns clock took {dt} ns, not 4");
    Ok(())
}

// A signal nobody drives produces no edge — the negative half of the claim.
//
// Without this, an edge trigger that fired on every simulator callback
// regardless of value would pass every test above.
#[rustdv::test]
async fn trig_no_edge_without_a_change(ctx: RustdvCtx) -> Result<(), TestError> {
    let clk = ctx.dut().signal("clk")?;
    Clock::new(&clk, SimDuration::ns(2)).start();
    let quiet = ctx.dut().signal("never_driven")?;

    // The clock is running, so the simulator is busy; `never_driven` is not
    // part of that traffic and must stay silent through all of it.
    let r = with_timeout(quiet.value_change(), SimDuration::ns(50)).await;
    check!(r.is_err(), "a signal nothing assigns reported a value change");
    Ok(())
}

// ---------------------------------------------------------------------------
// RAII deregistration
// ---------------------------------------------------------------------------

// Dropping a `CallbackHandle` removes the callback from the simulator.
//
// This is the leak the trigger design exists to prevent: every abandoned
// `with_timeout`, every cancelled task, every `first2` loser drops a
// registration, and a simulator holding thousands of dead callbacks slows to
// a crawl and fires closures into freed state.
//
// The mechanism is `CallbackHandle`'s `Drop`, and it is tested here rather
// than through `Timer` because a live callback is only observable if it does
// something — so these two do, into a counter.
#[rustdv::test]
async fn trig_dropped_callback_deregisters(_ctx: RustdvCtx) -> Result<(), TestError> {
    let per_ns = steps_per_ns().await;

    let kept = Rc::new(Cell::new(0u32));
    let dropped = Rc::new(Cell::new(0u32));

    let k = kept.clone();
    let _live = rustdv::gpi::register_timer(
        5 * per_ns,
        Box::new(move || k.set(k.get() + 1)),
    );

    let d = dropped.clone();
    let doomed = rustdv::gpi::register_timer(
        5 * per_ns,
        Box::new(move || d.set(d.get() + 1)),
    );
    drop(doomed);

    Timer::ns(20).await;

    // The control matters as much as the assertion: if neither fired, the
    // test would pass while proving nothing at all.
    check!(kept.get() == 1, "the live callback fired {} time(s), not once", kept.get());
    check!(dropped.get() == 0, "a deregistered callback fired {} time(s)", dropped.get());
    Ok(())
}

// ---------------------------------------------------------------------------
// with_timeout
// ---------------------------------------------------------------------------

// The timeout fires, and the abandoned inner future stops costing anything.
#[rustdv::test]
async fn trig_with_timeout_fires(_ctx: RustdvCtx) -> Result<(), TestError> {
    let t0 = sim_time_ns();
    let r = with_timeout(Timer::ns(100), SimDuration::ns(5)).await;
    check!(r.is_err(), "a 100ns timer beat a 5ns timeout");
    let dt = sim_time_ns() - t0;
    check!(dt == 5.0, "the 5ns timeout returned after {dt} ns");
    Ok(())
}

// The inner future wins when it is faster, and its value comes back.
#[rustdv::test]
async fn trig_with_timeout_inner_wins(_ctx: RustdvCtx) -> Result<(), TestError> {
    let t0 = sim_time_ns();
    let r = with_timeout(
        async {
            Timer::ns(3).await;
            42u32
        },
        SimDuration::ns(100),
    )
    .await;
    check!(r == Ok(42), "the inner future's value did not come back: {r:?}");
    let dt = sim_time_ns() - t0;
    check!(dt == 3.0, "the inner future finished at {dt} ns, not 3");
    Ok(())
}

// The Verilator host owns two independent time queues: delayed RTL events and
// VPI timers. It must always advance to the earlier deadline rather than
// starving one queue behind the other.
#[rustdv::test]
async fn trig_earliest_rtl_or_vpi_deadline_wins(ctx: RustdvCtx) -> Result<(), TestError> {
    let request = ctx.dut().signal("rtl_event_request")?;
    let done = ctx.dut().signal("rtl_event_done")?;
    request.set_u64_now(0);
    request.set_u64(1);
    read_write().await;

    let t0 = sim_time_ns();
    Timer::ns(3).await;
    check!(sim_time_ns() - t0 == 3.0, "the earlier VPI deadline did not win");
    check!(!done.is_high(), "the 7ns RTL event fired before the 3ns VPI timer");

    let edge = with_timeout(done.rising_edge(), SimDuration::ns(5)).await;
    check!(edge.is_ok(), "the earlier RTL event lost to a later VPI timeout");
    check!(sim_time_ns() - t0 == 7.0, "the RTL event did not fire after 7ns");
    Ok(())
}

// Verilator's callback regions are AtEnd -> ReadWrite -> ReadOnly. An AtEnd
// callback may write the DUT, and the host must re-evaluate that write before
// any ReadOnly waiter observes the combinational result.
#[rustdv::test]
async fn trig_at_end_write_settles_before_read_only(ctx: RustdvCtx) -> Result<(), TestError> {
    let input = ctx.dut().signal("comb_in")?;
    let output = ctx.dut().signal("comb_out")?;
    input.set_u64_now(0);
    read_write().await;

    rustdv::gpi::register_at_end_of_sim_time(Box::new(move || {
        input.set_u64_now(0x5A);
    }))
    .forget();
    read_only().await;

    let got = output.get_u64().unwrap_or(0);
    check!(got == (0x5A ^ 0xA5), "ReadOnly saw {got:#x} before the AtEnd write settled");
    Ok(())
}

// The test itself passes before simulation shutdown. The marker printed by
// this detached callback proves the host delivered cbEndOfSimulation.
#[rustdv::test]
async fn trig_end_of_simulation_callback_is_delivered(
    _ctx: RustdvCtx,
) -> Result<(), TestError> {
    rustdv::gpi::register_end_of_simulation(Box::new(|| {
        println!("END OF SIMULATION CALLBACK: PASS");
    }))
    .forget();
    Ok(())
}

// ---------------------------------------------------------------------------
// Phase triggers
// ---------------------------------------------------------------------------

// A `set` is buffered and applied at the next ReadWrite phase.
//
// This is cocotb's scheduling rule and the reason a driver can write a signal
// in the same delta a monitor reads it without either seeing the other's
// half-finished work. `set_u64_now` is the escape hatch that skips it.
#[rustdv::test]
async fn trig_writes_are_scheduled_not_immediate(ctx: RustdvCtx) -> Result<(), TestError> {
    let sig = ctx.dut().signal("byte_sig")?;
    sig.set_u64_now(0);

    sig.set_u64(0xA5);
    let immediate = sig.get_u64().unwrap_or(0);
    check!(immediate == 0, "a scheduled write landed immediately ({immediate:#x})");

    // The buffer drains at the *start* of the next ReadWrite phase, before
    // any ReadWrite callback runs — so this is the first moment the write is
    // visible, and it is still the same time step.
    read_write().await;
    let settled = sig.get_u64().unwrap_or(0);
    check!(settled == 0xA5, "the scheduled write never landed (read {settled:#x})");

    // The immediate form skips the buffer entirely.
    sig.set_u64_now(0x5A);
    let now = sig.get_u64().unwrap_or(0);
    check!(now == 0x5A, "set_u64_now did not take effect immediately (read {now:#x})");
    Ok(())
}

// A VPI write made in ReadWrite must reach the RTL and settle before
// ReadOnly.  Verilator hosts have to perform the intervening eval explicitly;
// without it this reads the previous value even though phase callbacks fire.
#[rustdv::test]
async fn trig_read_only_sees_settled_rtl(ctx: RustdvCtx) -> Result<(), TestError> {
    let input = ctx.dut().signal("comb_in")?;
    let output = ctx.dut().signal("comb_out")?;

    input.set_u64(0x3C);
    read_write().await;
    read_only().await;

    let got = output.get_u64().unwrap_or(0);
    check!(got == (0x3C ^ 0xA5), "ReadOnly saw comb_out={got:#x} before RTL settled");
    Ok(())
}

// ReadWrite and ReadOnly land inside the same time step; NextTimeStep does not.
#[rustdv::test]
async fn trig_phase_order_within_a_step(ctx: RustdvCtx) -> Result<(), TestError> {
    // `next_time_step` waits for the simulator to have a next step, and a
    // simulator with an empty event queue does not have one — vvp exits
    // instead. A running clock guarantees there is always another step to
    // wait for. Nothing here reads the clock; it is there to keep the
    // simulation alive, which is a real constraint on any test that awaits
    // a phase rather than a duration.
    let clk = ctx.dut().signal("clk")?;
    Clock::new(&clk, SimDuration::ns(2)).start();

    next_time_step().await;
    let t0 = sim_time_ns();

    read_write().await;
    check!(sim_time_ns() == t0, "ReadWrite advanced time from {t0} to {}", sim_time_ns());

    read_only().await;
    check!(sim_time_ns() == t0, "ReadOnly advanced time from {t0} to {}", sim_time_ns());

    next_time_step().await;
    check!(sim_time_ns() > t0, "NextTimeStep did not advance past {t0}");
    Ok(())
}
