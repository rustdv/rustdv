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
use std::rc::{Rc, Weak};
use std::task::{Context, Poll};

use rustdv_gpi as gpi;
use rustdv_gpi::{BigUint, LogicArray};

use crate::{executor, triggers::TrigShared};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SimPhase {
    Normal,
    ReadWrite,
    ReadOnly,
}

pub(crate) enum WriteVal {
    Bool(bool),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    BigInt(BigUint),
    Logic(LogicArray),
    Real(f64),
    String(String),
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) enum WriteTarget {
    Logic(gpi::LogicHandle),
    Real(gpi::RealHandle),
    String(gpi::StringHandle),
}
impl WriteTarget {
    fn full_name(self) -> String {
        match self {
            Self::Logic(h) => h.full_name(),
            Self::Real(h) => h.full_name(),
            Self::String(h) => h.full_name(),
        }
    }
}
impl From<gpi::LogicHandle> for WriteTarget {
    fn from(h: gpi::LogicHandle) -> Self {
        Self::Logic(h)
    }
}
impl From<gpi::RealHandle> for WriteTarget {
    fn from(h: gpi::RealHandle) -> Self {
        Self::Real(h)
    }
}
impl From<gpi::StringHandle> for WriteTarget {
    fn from(h: gpi::StringHandle) -> Self {
        Self::String(h)
    }
}

struct Hub {
    phase: Cell<SimPhase>,
    rw_waiters: RefCell<Vec<Weak<TrigShared>>>,
    rw_cb: RefCell<Option<gpi::CallbackHandle>>,
    ro_waiters: RefCell<Vec<Weak<TrigShared>>>,
    ro_cb: RefCell<Option<gpi::CallbackHandle>>,
    nt_waiters: RefCell<Vec<Weak<TrigShared>>>,
    nt_cb: RefCell<Option<gpi::CallbackHandle>>,
    writes: RefCell<Vec<(WriteTarget, WriteVal)>>,
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
    HUB.with(|h| {
        h.borrow()
            .clone()
            .expect("rustdv sim context not initialized")
    })
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

fn apply_write(target: WriteTarget, v: &WriteVal) {
    match (target, v) {
        (WriteTarget::Real(h), WriteVal::Real(x)) => {
            h.set_now(*x);
            return;
        }
        (WriteTarget::String(h), WriteVal::String(x)) => {
            h.set_now(x).expect("scheduled string was validated");
            return;
        }
        _ => {}
    }
    let WriteTarget::Logic(h) = target else {
        unreachable!("write target/value mismatch")
    };
    match v {
        WriteVal::Bool(x) => h.set_bool_now(*x),
        WriteVal::U8(x) => h.set_u8_now(*x),
        WriteVal::U16(x) => h.set_u16_now(*x),
        WriteVal::U32(x) => h.set_u32_now(*x),
        WriteVal::U64(x) => h.set_u64_now(*x),
        WriteVal::U128(x) => h.set_u128_now(*x),
        WriteVal::BigInt(x) => h.set_bigint_now(x),
        WriteVal::Logic(x) => h.set_logic_now(x),
        _ => unreachable!("write target/value mismatch"),
    }
}

/// The ReadOnly rule, in one place because both write paths owe it: the
/// scheduled one below and the immediate methods in the `handle` submodules (D108).
pub(crate) fn deny_write_in_read_only(h: impl Into<WriteTarget>) {
    let h = h.into();
    if hub().phase.get() == SimPhase::ReadOnly {
        panic!(
            "illegal write to '{}' during the ReadOnly phase (cocotb rule, design-doc §4.4)",
            h.full_name()
        );
    }
}

pub(crate) fn schedule(h: impl Into<WriteTarget>, v: WriteVal) {
    let h = h.into();
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

// ---------------------------------------------------------------------------
// Priming and firing
// ---------------------------------------------------------------------------

// Detach the batch before waking tasks: waits created during the executor
// drain belong to a subsequent callback. Weak entries do not retain cancelled
// futures or their task wakers, and cancellation never removes shared work.
fn fire_waiters(waiters: &RefCell<Vec<Weak<TrigShared>>>) {
    for waiter in std::mem::take(&mut *waiters.borrow_mut()) {
        if let Some(shared) = waiter.upgrade() {
            shared.fire();
        }
    }
}

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
        fire_waiters(&h.rw_waiters);
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
        fire_waiters(&h.ro_waiters);
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
        fire_waiters(&h.nt_waiters);
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
    shared: Option<Rc<TrigShared>>,
}

impl Future for PhaseFut {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if let Some(shared) = &self.shared {
            if shared.fired() {
                return Poll::Ready(());
            }
            shared.set_waker(cx.waker().clone());
            return Poll::Pending;
        }
        let hub = hub();
        let shared = TrigShared::new();
        shared.set_waker(cx.waker().clone());
        match self.kind {
            PhaseKind::ReadWrite => {
                if hub.phase.get() == SimPhase::ReadOnly {
                    panic!("awaiting ReadWrite from the ReadOnly phase is illegal (cocotb rule)");
                }
                hub.rw_waiters.borrow_mut().push(Rc::downgrade(&shared));
                prime_rw(&hub);
            }
            PhaseKind::ReadOnly => {
                if hub.phase.get() == SimPhase::ReadOnly {
                    panic!("awaiting ReadOnly from the ReadOnly phase is illegal (cocotb rule)");
                }
                hub.ro_waiters.borrow_mut().push(Rc::downgrade(&shared));
                prime_ro(&hub);
            }
            PhaseKind::NextTimeStep => {
                hub.nt_waiters.borrow_mut().push(Rc::downgrade(&shared));
                prime_nt(&hub);
            }
        }
        self.shared = Some(shared);
        Poll::Pending
    }
}

/// Await the next ReadWrite phase (port of `ReadWrite()`).
pub fn read_write() -> PhaseFut {
    PhaseFut {
        kind: PhaseKind::ReadWrite,
        shared: None,
    }
}

/// Await the next ReadOnly phase (port of `ReadOnly()`).
pub fn read_only() -> PhaseFut {
    PhaseFut {
        kind: PhaseKind::ReadOnly,
        shared: None,
    }
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
    PhaseFut {
        kind: PhaseKind::NextTimeStep,
        shared: None,
    }
}
