//! Phase triggers (ReadOnly / ReadWrite / NextTimeStep) and the scheduled
//! write buffer (design-doc §4.1(4), §4.4, mapping rows 15 & 22).
//!
//! One hub per sim thread coordinates: writes via `set()` are buffered and
//! drained at the start of the next ReadWrite phase — before ReadWrite
//! awaiters are woken — porting cocotb's write scheduler verbatim
//! (cocotb: handle.py `_apply_scheduled_writes`, `ReadWrite._do_callbacks`).
//! Illegal phase transitions (write/await-ReadWrite from ReadOnly) are
//! runtime errors, exactly as cocotb.

use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use rustdv_gpi as gpi;
use rustdv_gpi::LogicArray;

use crate::executor;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SimPhase {
    Normal,
    ReadWrite,
    ReadOnly,
}

enum WriteVal {
    U64(u64),
    Arr(LogicArray),
}

struct Hub {
    phase: Cell<SimPhase>,
    rw_waiters: RefCell<Vec<(Rc<Cell<bool>>, Waker)>>,
    rw_cb: RefCell<Option<gpi::CallbackHandle>>,
    ro_waiters: RefCell<Vec<(Rc<Cell<bool>>, Waker)>>,
    ro_cb: RefCell<Option<gpi::CallbackHandle>>,
    nt_waiters: RefCell<Vec<(Rc<Cell<bool>>, Waker)>>,
    nt_cb: RefCell<Option<gpi::CallbackHandle>>,
    writes: RefCell<Vec<(gpi::LogicHandle, WriteVal)>>,
}

thread_local! {
    static HUB: RefCell<Option<Rc<Hub>>> = const { RefCell::new(None) };
}

pub(crate) fn init_hub() {
    HUB.with(|h| {
        *h.borrow_mut() = Some(Rc::new(Hub {
            phase: Cell::new(SimPhase::Normal),
            rw_waiters: RefCell::new(Vec::new()),
            rw_cb: RefCell::new(None),
            ro_waiters: RefCell::new(Vec::new()),
            ro_cb: RefCell::new(None),
            nt_waiters: RefCell::new(Vec::new()),
            nt_cb: RefCell::new(None),
            writes: RefCell::new(Vec::new()),
        }));
    });
}

fn hub() -> Rc<Hub> {
    HUB.with(|h| h.borrow().clone().expect("rustdv sim context not initialized"))
}

pub fn current_phase() -> SimPhase {
    hub().phase.get()
}

pub(crate) fn current_phase_if_initialized() -> Option<SimPhase> {
    HUB.with(|hub| hub.borrow().as_ref().map(|hub| hub.phase.get()))
}

// ---------------------------------------------------------------------------
// Leaving ReadOnly (D108)
// ---------------------------------------------------------------------------

/// Return to a region where writing is legal.
///
/// The ReadOnly region belongs to the simulator, not to the test that asked
/// for it. `prime_ro`'s callback sets the phase, wakes the waiters, drains the
/// executor and only *then* restores `Normal` — and the drain is not confined
/// to the waiters it woke. Whatever else the executor has queued runs in the
/// same drain, inside the same `cbReadOnlySynch` callback. When the test that
/// awaited ReadOnly finishes there, the thing that runs next is the regression
/// loop, and after that the next test's first statement (D108).
///
/// One precision step is the whole of it. There is no cheaper exit: a
/// zero-delay `cbAfterDelay` registered from inside the ReadOnly callback
/// would land in the current time step, and Icarus refuses that outright —
/// `SCHEDULER ERROR: read-only sync events created RW events!` and the run
/// stops. Once the read-only region of a time step has begun, that time step
/// has no writable region left, so leaving costs time by construction.
///
/// A no-op when the simulation is not in ReadOnly, which is the usual case: a
/// test whose predecessor ended normally starts exactly where it used to.
pub async fn leave_read_only() {
    if hub().phase.get() != SimPhase::ReadOnly {
        return;
    }
    crate::triggers::Timer::steps(1).await;
}

// ---------------------------------------------------------------------------
// Write scheduling
// ---------------------------------------------------------------------------

fn apply_write(h: gpi::LogicHandle, v: &WriteVal) {
    match v {
        WriteVal::U64(x) => h.set_u64_now(*x),
        WriteVal::Arr(a) => h.set_now(a),
    }
}

/// The ReadOnly rule, in one place because both write paths owe it: the
/// scheduled one below and the immediate one in `handle.rs` (D108).
pub(crate) fn deny_write_in_read_only(h: gpi::LogicHandle) {
    if hub().phase.get() == SimPhase::ReadOnly {
        panic!(
            "illegal write to '{}' during the ReadOnly phase (cocotb rule, design-doc §4.4)",
            h.full_name()
        );
    }
}

fn schedule(h: gpi::LogicHandle, v: WriteVal) {
    let hub = hub();
    match hub.phase.get() {
        SimPhase::ReadOnly => deny_write_in_read_only(h),
        SimPhase::ReadWrite => apply_write(h, &v),
        SimPhase::Normal => {
            {
                let mut writes = hub.writes.borrow_mut();
                if let Some(slot) = writes.iter_mut().find(|(eh, _)| *eh == h) {
                    slot.1 = v; // last write wins, order preserved
                } else {
                    writes.push((h, v));
                }
            }
            prime_rw(&hub);
        }
    }
}

pub(crate) fn schedule_write_u64(h: gpi::LogicHandle, v: u64) {
    schedule(h, WriteVal::U64(v));
}

pub(crate) fn schedule_write_arr(h: gpi::LogicHandle, v: LogicArray) {
    schedule(h, WriteVal::Arr(v));
}

// ---------------------------------------------------------------------------
// Priming and firing
// ---------------------------------------------------------------------------

fn prime_rw(hub: &Rc<Hub>) {
    if hub.rw_cb.borrow().is_some() {
        return;
    }
    let h = hub.clone();
    let cb = gpi::register_read_write(Box::new(move || {
        h.rw_cb.borrow_mut().take(); // fired: safe to drop (released)
        h.phase.set(SimPhase::ReadWrite);
        // Drain the write buffer FIRST (cocotb ReadWrite._do_callbacks).
        let writes: Vec<_> = h.writes.borrow_mut().drain(..).collect();
        for (sig, val) in &writes {
            apply_write(*sig, val);
        }
        for (fired, w) in h.rw_waiters.borrow_mut().drain(..) {
            fired.set(true);
            w.wake();
        }
        executor::current().run_until_idle();
        h.phase.set(SimPhase::Normal);
    }));
    *hub.rw_cb.borrow_mut() = Some(cb);
}

fn prime_ro(hub: &Rc<Hub>) {
    if hub.ro_cb.borrow().is_some() {
        return;
    }
    let h = hub.clone();
    let cb = gpi::register_read_only(Box::new(move || {
        h.ro_cb.borrow_mut().take();
        h.phase.set(SimPhase::ReadOnly);
        for (fired, w) in h.ro_waiters.borrow_mut().drain(..) {
            fired.set(true);
            w.wake();
        }
        executor::current().run_until_idle();
        h.phase.set(SimPhase::Normal);
    }));
    *hub.ro_cb.borrow_mut() = Some(cb);
}

fn prime_nt(hub: &Rc<Hub>) {
    if hub.nt_cb.borrow().is_some() {
        return;
    }
    let h = hub.clone();
    let cb = gpi::register_next_sim_time(Box::new(move || {
        h.nt_cb.borrow_mut().take();
        for (fired, w) in h.nt_waiters.borrow_mut().drain(..) {
            fired.set(true);
            w.wake();
        }
        executor::current().run_until_idle();
    }));
    *hub.nt_cb.borrow_mut() = Some(cb);
}

// ---------------------------------------------------------------------------
// Awaitables (singleton-trigger analogs, mapping row 15)
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, PartialEq, Eq)]
enum PhaseKind {
    ReadWrite,
    ReadOnly,
    NextTimeStep,
}

pub struct PhaseFut {
    kind: PhaseKind,
    fired: Option<Rc<Cell<bool>>>,
}

impl Future for PhaseFut {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if let Some(fired) = &self.fired {
            return if fired.get() {
                Poll::Ready(())
            } else {
                Poll::Pending
            };
        }
        let hub = hub();
        let fired = Rc::new(Cell::new(false));
        match self.kind {
            PhaseKind::ReadWrite => {
                if hub.phase.get() == SimPhase::ReadOnly {
                    panic!("awaiting ReadWrite from the ReadOnly phase is illegal (cocotb rule)");
                }
                hub.rw_waiters
                    .borrow_mut()
                    .push((fired.clone(), cx.waker().clone()));
                prime_rw(&hub);
            }
            PhaseKind::ReadOnly => {
                if hub.phase.get() == SimPhase::ReadOnly {
                    panic!("awaiting ReadOnly from the ReadOnly phase is illegal (cocotb rule)");
                }
                hub.ro_waiters
                    .borrow_mut()
                    .push((fired.clone(), cx.waker().clone()));
                prime_ro(&hub);
            }
            PhaseKind::NextTimeStep => {
                hub.nt_waiters
                    .borrow_mut()
                    .push((fired.clone(), cx.waker().clone()));
                prime_nt(&hub);
            }
        }
        self.fired = Some(fired);
        Poll::Pending
    }
}

/// Await the next ReadWrite phase (port of `ReadWrite()`).
pub fn read_write() -> PhaseFut {
    PhaseFut { kind: PhaseKind::ReadWrite, fired: None }
}

/// Await the next ReadOnly phase (port of `ReadOnly()`).
pub fn read_only() -> PhaseFut {
    PhaseFut { kind: PhaseKind::ReadOnly, fired: None }
}

/// Run one synchronous service operation at a settled ReadOnly point.
///
/// The closure runs on the simulator thread from inside the simulator's
/// ReadOnly callback, after pending writes and combinational logic have
/// reached a fixed point. It may communicate with other OS threads through
/// ordinary synchronization primitives. While it is waiting, the simulator
/// callback cannot return, so simulation time remains frozen. Returning the
/// closure's value releases the simulator to advance normally.
///
/// # Blocking and cancellation
///
/// Any wait in `service` must be bounded by wall-clock time or use a
/// cooperative cancellation signal. A simulation-time timeout cannot fire
/// while this function holds ReadOnly, and RustDV cannot forcibly recover an
/// arbitrary blocking closure. An unbounded wait will therefore wedge the
/// simulation.
///
/// This is opt-in and transport-neutral: RustDV does not create a worker,
/// scheduler, or server. A testbench supplies both the service closure and any
/// channels or synchronization it needs.
pub async fn service_read_only<F, R>(service: F) -> R
where
    F: FnOnce() -> R,
{
    read_only().await;
    service()
}

/// Await the next simulator time step (port of `NextTimeStep()`).
pub fn next_time_step() -> PhaseFut {
    PhaseFut { kind: PhaseKind::NextTimeStep, fired: None }
}
