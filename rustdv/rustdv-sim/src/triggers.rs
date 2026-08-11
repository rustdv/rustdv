//! GPI-backed awaitables: `Timer`, edge triggers, `NullTrigger`
//! (design-doc §4.4). Each future registers its simulator callback lazily
//! on first poll and removes it via RAII when dropped — cancellation of a
//! waiting task cleans up its trigger registration for free (§4.6).

use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use rustdv_gpi as gpi;

use crate::executor;
use crate::time::SimDuration;

/// Shared fire/waker cell — the `TriggerWaker` role (design-doc §4.3).
pub(crate) struct TrigShared {
    fired: Cell<bool>,
    waker: RefCell<Option<Waker>>,
}

impl TrigShared {
    pub(crate) fn new() -> Rc<TrigShared> {
        Rc::new(TrigShared { fired: Cell::new(false), waker: RefCell::new(None) })
    }
    pub(crate) fn fire(&self) {
        self.fired.set(true);
        if let Some(w) = self.waker.borrow_mut().take() {
            w.wake();
        }
    }
    pub(crate) fn fired(&self) -> bool {
        self.fired.get()
    }
    pub(crate) fn set_waker(&self, w: Waker) {
        *self.waker.borrow_mut() = Some(w);
    }
}

// ---------------------------------------------------------------------------
// Timer
// ---------------------------------------------------------------------------

/// One-shot timed trigger (port of cocotb `Timer`, mapping row 13).
/// Construction rejects zero durations, as cocotb's does.
#[must_use = "triggers do nothing unless you .await them"]
pub struct Timer {
    steps: u64,
    shared: Option<Rc<TrigShared>>,
    _cb: Option<gpi::CallbackHandle>,
}

impl Timer {
    pub fn new(d: SimDuration) -> Timer {
        assert!(d.steps > 0, "Timer duration must be positive (cocotb rule)");
        Timer { steps: d.steps, shared: None, _cb: None }
    }
    pub fn steps(steps: u64) -> Timer {
        Self::new(SimDuration::steps(steps))
    }
    pub fn ns(n: u64) -> Timer {
        Self::new(SimDuration::ns(n))
    }
    pub fn us(n: u64) -> Timer {
        Self::new(SimDuration::us(n))
    }
    pub fn ms(n: u64) -> Timer {
        Self::new(SimDuration::ms(n))
    }
}

impl Future for Timer {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        match &self.shared {
            None => {
                let sh = TrigShared::new();
                sh.set_waker(cx.waker().clone());
                let sh2 = sh.clone();
                let cb = gpi::register_timer(
                    self.steps,
                    Box::new(move || {
                        sh2.fire();
                        executor::current().run_until_idle();
                    }),
                );
                self.shared = Some(sh);
                self._cb = Some(cb);
                Poll::Pending
            }
            Some(sh) => {
                if sh.fired() {
                    Poll::Ready(())
                } else {
                    sh.set_waker(cx.waker().clone());
                    Poll::Pending
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Edges
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) enum EdgeKind {
    Rising,
    Falling,
    AnyChange,
}

/// Edge trigger on a signal (ports of RisingEdge/FallingEdge/ValueChange,
/// mapping row 14 — exposed as methods on typed handles).
#[must_use = "triggers do nothing unless you .await them"]
pub struct Edge {
    sig: gpi::LogicHandle,
    kind: EdgeKind,
    shared: Option<Rc<TrigShared>>,
    _cb: Option<gpi::CallbackHandle>,
}

impl Edge {
    pub(crate) fn new(sig: gpi::LogicHandle, kind: EdgeKind) -> Edge {
        Edge { sig, kind, shared: None, _cb: None }
    }
}

impl Future for Edge {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        match &self.shared {
            None => {
                let sh = TrigShared::new();
                sh.set_waker(cx.waker().clone());
                let sh2 = sh.clone();
                let sig = self.sig;
                let kind = self.kind;
                let cb = gpi::register_value_change(
                    sig,
                    Box::new(move || {
                        if sh2.fired() {
                            return; // already matched; awaiting removal
                        }
                        let hit = match kind {
                            EdgeKind::AnyChange => true,
                            EdgeKind::Rising => sig.size() == 1 && sig.get_u64() == Ok(1),
                            EdgeKind::Falling => sig.size() == 1 && sig.get_u64() == Ok(0),
                        };
                        if hit {
                            sh2.fire();
                            executor::current().run_until_idle();
                        }
                    }),
                );
                self.shared = Some(sh);
                self._cb = Some(cb);
                Poll::Pending
            }
            Some(sh) => {
                if sh.fired() {
                    Poll::Ready(())
                } else {
                    sh.set_waker(cx.waker().clone());
                    Poll::Pending
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// NullTrigger
// ---------------------------------------------------------------------------

/// Yield once to the scheduler. Kept for parity but documented as a smell —
/// prefer `Event` (cocotb NullTrigger docstring; book: Coroutines chapter).
#[must_use = "triggers do nothing unless you .await them"]
pub struct NullTrigger {
    yielded: bool,
}

impl NullTrigger {
    #[allow(clippy::new_without_default)]
    pub fn new() -> NullTrigger {
        NullTrigger { yielded: false }
    }
}

impl Future for NullTrigger {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.yielded {
            Poll::Ready(())
        } else {
            self.yielded = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}
