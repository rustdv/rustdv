//! rustdv on a backend other than VPI.
//!
//! `Fake` is a minimal simulator implementing [`Backend`]: a top scope `top`
//! holding a few signals, a clock of simulation time, and the registered
//! callbacks. The test plays the simulator: it changes signals, settles the
//! ReadWrite/ReadOnly phases and moves time forward, the way a host such as
//! a cycle-based simulator does from its own hooks. rustdv's executor,
//! triggers and buffered writes then run on top of it unchanged.

use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use rustdv_gpi::ValueError;
use rustdv_gpi::backend::{self, Backend, CbData, CbToken, ObjHandle, ObjKind, Reason};
use rustdv_gpi_sys::t_vpi_vecval;
use rustdv_sim::handle::{HandleChildren, HandleEvent, top_module};
use rustdv_sim::{Timer, log};

// The VPI backend is still compiled in under `cargo test`; the stubs supply
// its symbols. Nothing here calls it.
use rustdv_vpi_stubs as _;

/// Time precision of the fake: 1 ns.
const PRECISION: i32 = -9;

#[derive(Copy, Clone, PartialEq)]
enum Wait {
    At(u64),
    Change(usize),
    ReadWrite,
    ReadOnly,
    NextSimTime,
    Other,
}

struct Signal {
    name: &'static str,
    width: u32,
    value: u32,
}

#[derive(Default)]
struct State {
    time: u64,
    signals: Vec<Signal>,
    pending: Vec<(CbToken, Wait, CbData)>,
    next_token: CbToken,
    removed: Vec<CbToken>,
    console: Vec<String>,
    failures: Vec<String>,
    finished: bool,
}

/// Handle 1 is the top scope `top`; handle `i + 2` is signal `i`.
struct Fake {
    state: RefCell<State>,
}

const TOP: usize = 1;

impl Fake {
    /// A fake with the given `(name, width)` signals, alive for the rest of
    /// the test as a backend must be.
    fn leak(signals: &[(&'static str, u32)]) -> &'static Fake {
        let signals = signals
            .iter()
            .map(|&(name, width)| Signal {
                name,
                width,
                value: 0,
            })
            .collect();
        Box::leak(Box::new(Fake {
            state: RefCell::new(State {
                signals,
                next_token: 1,
                ..State::default()
            }),
        }))
    }

    fn handle(raw: usize) -> ObjHandle {
        // SAFETY: IDs this backend issues itself.
        unsafe { ObjHandle::from_raw(raw) }
    }

    fn signal_index(h: ObjHandle) -> Option<usize> {
        h.as_raw().checked_sub(2)
    }

    fn index_of(&self, name: &str) -> usize {
        self.state
            .borrow()
            .signals
            .iter()
            .position(|s| s.name == name)
            .unwrap_or_else(|| panic!("no signal {name}"))
    }

    fn value(&self, name: &str) -> u32 {
        let i = self.index_of(name);
        self.state.borrow().signals[i].value
    }

    /// Run the pending callbacks matching `pred`. Returns whether any ran.
    fn fire(&self, pred: impl Fn(Wait) -> bool) -> bool {
        let due: Vec<(CbToken, CbData)> = self
            .state
            .borrow()
            .pending
            .iter()
            .filter(|p| pred(p.1))
            .map(|p| (p.0, p.2))
            .collect();
        for &(token, data) in &due {
            let live = self.state.borrow().pending.iter().any(|p| p.0 == token);
            if live {
                // SAFETY: still registered.
                unsafe { data.dispatch() };
            }
        }
        !due.is_empty()
    }

    /// Run ReadWrite, then ReadOnly callbacks until none is pending.
    fn settle(&self) {
        while self.fire(|w| w == Wait::ReadWrite) || self.fire(|w| w == Wait::ReadOnly) {}
    }

    /// Change a signal, as the design would, and run what it wakes.
    fn drive(&self, name: &str, value: u32) {
        let i = self.index_of(name);
        self.state.borrow_mut().signals[i].value = value;
        self.fire(|w| w == Wait::Change(i));
        self.settle();
    }

    /// Move time to `t`, running each timer at its own deadline.
    fn advance_to(&self, t: u64) {
        loop {
            let next = self
                .state
                .borrow()
                .pending
                .iter()
                .filter_map(|p| match p.1 {
                    Wait::At(d) if d <= t => Some(d),
                    _ => None,
                })
                .min();
            let Some(d) = next else { break };
            self.step_to(d);
            self.fire(|w| w == Wait::At(d));
            self.settle();
        }
        self.step_to(t);
    }

    fn step_to(&self, t: u64) {
        let moved = {
            let mut s = self.state.borrow_mut();
            let moved = t > s.time;
            s.time = s.time.max(t);
            moved
        };
        if moved {
            self.fire(|w| w == Wait::NextSimTime);
            self.settle();
        }
    }
}

impl Backend for Fake {
    fn top_modules(&self) -> Vec<ObjHandle> {
        vec![Fake::handle(TOP)]
    }

    fn handle_by_name(&self, name: &str) -> Option<ObjHandle> {
        match name.strip_prefix("top") {
            Some("") => Some(Fake::handle(TOP)),
            Some(rest) => self.child(Fake::handle(TOP), rest.strip_prefix('.')?),
            None => None,
        }
    }

    fn kind(&self, h: ObjHandle) -> ObjKind {
        match Fake::signal_index(h) {
            None => ObjKind::Hierarchy,
            Some(i) => ObjKind::Logic {
                width: self.state.borrow().signals[i].width,
            },
        }
    }

    fn name(&self, h: ObjHandle) -> String {
        match Fake::signal_index(h) {
            None => "top".to_owned(),
            Some(i) => self.state.borrow().signals[i].name.to_owned(),
        }
    }

    fn full_name(&self, h: ObjHandle) -> String {
        match Fake::signal_index(h) {
            None => "top".to_owned(),
            Some(_) => format!("top.{}", self.name(h)),
        }
    }

    fn type_desc(&self, h: ObjHandle) -> String {
        match Fake::signal_index(h) {
            None => "module".to_owned(),
            Some(_) => "logic".to_owned(),
        }
    }

    fn child(&self, scope: ObjHandle, name: &str) -> Option<ObjHandle> {
        if scope.as_raw() != TOP {
            return None;
        }
        let i = self
            .state
            .borrow()
            .signals
            .iter()
            .position(|s| s.name == name)?;
        Some(Fake::handle(i + 2))
    }

    fn children(&self, scope: ObjHandle) -> Vec<ObjHandle> {
        if scope.as_raw() != TOP {
            return Vec::new();
        }
        let n = self.state.borrow().signals.len();
        (0..n).map(|i| Fake::handle(i + 2)).collect()
    }

    fn get_vector(&self, h: ObjHandle, _width: u32) -> Result<Cow<'_, [t_vpi_vecval]>, ValueError> {
        let i = Fake::signal_index(h).expect("a signal");
        let aval = self.state.borrow().signals[i].value;
        Ok(Cow::Owned(vec![t_vpi_vecval { aval, bval: 0 }]))
    }

    fn put_vector(&self, h: ObjHandle, words: &mut [t_vpi_vecval], _flags: i32) {
        let i = Fake::signal_index(h).expect("a signal");
        self.state.borrow_mut().signals[i].value = words[0].aval;
    }

    fn sim_time_steps(&self) -> u64 {
        self.state.borrow().time
    }

    fn time_precision(&self) -> i32 {
        PRECISION
    }

    fn finish(&self) {
        self.state.borrow_mut().finished = true;
    }

    fn report_failure(&self, msg: &str) {
        self.state.borrow_mut().failures.push(msg.to_owned());
    }

    fn console(&self, line: &str) {
        self.state.borrow_mut().console.push(line.to_owned());
    }

    fn register_cb(&self, reason: Reason, data: CbData) -> CbToken {
        let mut s = self.state.borrow_mut();
        let wait = match reason {
            Reason::AfterDelay(steps) => Wait::At(s.time + steps),
            Reason::ValueChange(h) => Wait::Change(Fake::signal_index(h).expect("a signal")),
            Reason::ReadWrite => Wait::ReadWrite,
            Reason::ReadOnly => Wait::ReadOnly,
            Reason::NextSimTime => Wait::NextSimTime,
            _ => Wait::Other,
        };
        let token = s.next_token;
        s.next_token += 1;
        s.pending.push((token, wait, data));
        token
    }

    fn remove_cb(&self, token: CbToken) {
        let mut s = self.state.borrow_mut();
        s.pending.retain(|p| p.0 != token);
        s.removed.push(token);
    }
}

/// Removes the installed backend when a test ends, even on failure.
struct Installed;

impl Installed {
    fn new(fake: &'static Fake) -> Installed {
        backend::install(fake);
        Installed
    }
}

impl Drop for Installed {
    fn drop(&mut self) {
        backend::uninstall();
    }
}

#[test]
fn a_task_runs_on_an_installed_backend() {
    let fake = Fake::leak(&[("clk", 1), ("d", 8), ("q", 8)]);
    let _installed = Installed::new(fake);
    let ex = rustdv_sim::init();

    let done = Rc::new(Cell::new(false));
    let done2 = done.clone();
    ex.spawn_named(
        async move {
            let top = top_module().unwrap();
            let clk = top.signal("clk").unwrap();
            let d = top.signal("d").unwrap();
            let q = top.signal("q").unwrap();
            clk.rising_edge().await;
            q.set_u64(d.get_u64().unwrap() + 1);
            Timer::ns(5).await;
            log::info("timer done");
            done2.set(true);
        },
        Some("tb"),
    );
    ex.run_until_idle();

    fake.drive("d", 41);
    assert_eq!(fake.value("q"), 0, "nothing happens before the edge");
    fake.drive("clk", 1);
    assert_eq!(
        fake.value("q"),
        42,
        "the write lands in the edge's ReadWrite phase"
    );

    fake.advance_to(4);
    assert!(!done.get());
    fake.advance_to(5);
    assert!(done.get());
    assert!(
        fake.state
            .borrow()
            .console
            .iter()
            .any(|l| l.contains("timer done")),
        "log output goes to the backend's console"
    );
}

#[test]
fn a_callback_is_removed_through_the_backend_that_registered_it() {
    let first = Fake::leak(&[]);
    let second = Fake::leak(&[]);
    let _installed = Installed::new(first);

    let cb = rustdv_gpi::register_timer(10, Box::new(|| {}));
    let token = first.state.borrow().pending[0].0;
    backend::install(second);
    drop(cb);

    assert_eq!(first.state.borrow().removed, vec![token]);
    assert!(first.state.borrow().pending.is_empty());
    assert!(second.state.borrow().removed.is_empty());
}

#[test]
fn finish_failure_and_console_reach_the_backend() {
    let fake = Fake::leak(&[]);
    let _installed = Installed::new(fake);

    rustdv_gpi::console("hello");
    rustdv_gpi::report_failure("broken");
    rustdv_gpi::finish();

    let s = fake.state.borrow();
    assert_eq!(s.console, vec!["hello"]);
    assert_eq!(s.failures, vec!["broken"]);
    assert!(s.finished);
}
