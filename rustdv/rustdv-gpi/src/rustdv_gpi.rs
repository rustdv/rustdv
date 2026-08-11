//! # rustdv-gpi
//!
//! Safe wrapper over the simulator programming interface (design-doc §3.3).
//! Invariants upheld here so everything above is safe Rust:
//!
//! 1. Handles are opaque and non-null; fallible acquisition is `Result`.
//! 2. Object-handle lifetime = simulation lifetime (freely `Copy`able IDs).
//!    Callback registrations are modeled by RAII ([`CallbackHandle`]):
//!    dropping a live handle removes it, and fired one-shots remove themselves
//!    from inside the trampoline while both supported simulators still accept
//!    the registration handle.
//! 3. Strings are copied at the boundary, every call.
//! 4. No unwinding across FFI: every trampoline wraps the closure in
//!    `catch_unwind`; panics are routed to the panic sink.
//! 5. Callback user-data ownership: an `Rc` whose C-side reference is
//!    reclaimed exactly once (on fire for one-shots, on removal otherwise).
//!
//! Thread affinity (§3.4): all types here hold raw pointers and are
//! therefore `!Send`/`!Sync` — the compiler rejects moving them off the
//! simulator thread.

use std::cell::{Cell, RefCell};
use std::ffi::{CStr, CString};
use std::fmt;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

use rustdv_gpi_sys as sys;

// Test executables need vpi_* symbol definitions (the simulator provides
// them for the real cdylib) — see rustdv-vpi-stubs.
#[cfg(test)]
use rustdv_vpi_stubs as _;

pub mod value;
pub use value::{Logic, LogicArray};

// ===========================================================================
// Errors
// ===========================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandleError {
    /// Name did not resolve (cocotb raises AttributeError here; rustdv
    /// returns this — design-doc §0.6).
    NotFound { name: String, scope: String },
    /// Handle exists but is not the requested kind.
    WrongKind { name: String, expected: &'static str, actual: String },
    NoTopModule,
}

impl fmt::Display for HandleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HandleError::NotFound { name, scope } => {
                write!(f, "no object named '{name}' in scope '{scope}'")
            }
            HandleError::WrongKind { name, expected, actual } => {
                write!(f, "'{name}' is a {actual}, expected {expected}")
            }
            HandleError::NoTopModule => write!(f, "no top-level module found"),
        }
    }
}
impl std::error::Error for HandleError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueError {
    /// Value contains x/z bits and was asked for as an integer.
    FourState(String),
    Width { want: u32, have: usize },
}

impl fmt::Display for ValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValueError::FourState(s) => write!(f, "value '{s}' has x/z bits"),
            ValueError::Width { want, have } => write!(f, "width mismatch: want {want}, have {have}"),
        }
    }
}
impl std::error::Error for ValueError {}

// ===========================================================================
// Handles
// ===========================================================================

/// Raw non-null simulator object handle. Valid for the whole simulation
/// (invariant 2), hence `Copy`. `!Send` because it wraps a raw pointer.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct ObjHandle(sys::vpiHandle);

impl ObjHandle {
    fn new(h: sys::vpiHandle) -> Option<Self> {
        if h.is_null() { None } else { Some(ObjHandle(h)) }
    }
    fn get(self, prop: i32) -> i32 {
        unsafe { sys::vpi_get(prop, self.0) }
    }
    fn get_str(self, prop: i32) -> String {
        // Invariant 3: copy immediately.
        unsafe {
            let p = sys::vpi_get_str(prop, self.0);
            if p.is_null() {
                String::new()
            } else {
                CStr::from_ptr(p).to_string_lossy().into_owned()
            }
        }
    }
}

/// Any simulator object, classified by type (design-doc §3.3 downcasting).
#[derive(Copy, Clone)]
pub enum AnyHandle {
    Hierarchy(HierarchyHandle),
    Logic(LogicHandle),
    Other(ObjHandle),
}

impl AnyHandle {
    pub fn classify(h: ObjHandle) -> AnyHandle {
        match h.get(sys::vpiType) {
            sys::vpiModule => AnyHandle::Hierarchy(HierarchyHandle { h }),
            sys::vpiNet | sys::vpiReg | sys::vpiIntegerVar | sys::vpiPort | sys::vpiMemory
            | sys::vpiLongIntVar | sys::vpiShortIntVar | sys::vpiIntVar | sys::vpiByteVar
            | sys::vpiEnumVar | sys::vpiBitVar => {
                // Signal width is immutable for the lifetime of a VPI
                // object. Cache it at discovery so every value read/write
                // does not pay for another vpi_get(vpiSize) crossing.
                let width = h.get(sys::vpiSize).max(0) as u32;
                AnyHandle::Logic(LogicHandle { h, width })
            }
            _ => AnyHandle::Other(h),
        }
    }

    pub fn as_logic(self) -> Result<LogicHandle, HandleError> {
        match self {
            AnyHandle::Logic(l) => Ok(l),
            AnyHandle::Hierarchy(h) => Err(HandleError::WrongKind {
                name: h.full_name(),
                expected: "signal",
                actual: "module".into(),
            }),
            AnyHandle::Other(o) => Err(HandleError::WrongKind {
                name: o.get_str(sys::vpiFullName),
                expected: "signal",
                actual: format!("vpiType {}", o.get(sys::vpiType)),
            }),
        }
    }

    pub fn as_hierarchy(self) -> Result<HierarchyHandle, HandleError> {
        match self {
            AnyHandle::Hierarchy(h) => Ok(h),
            AnyHandle::Logic(l) => Err(HandleError::WrongKind {
                name: l.full_name(),
                expected: "module",
                actual: "signal".into(),
            }),
            AnyHandle::Other(o) => Err(HandleError::WrongKind {
                name: o.get_str(sys::vpiFullName),
                expected: "module",
                actual: format!("vpiType {}", o.get(sys::vpiType)),
            }),
        }
    }
}

/// A module/scope handle. Port of cocotb's HierarchyObject
/// (cocotb: handle.py), with `Result`-returning lookup (mapping row 19).
#[derive(Copy, Clone)]
pub struct HierarchyHandle {
    h: ObjHandle,
}

impl HierarchyHandle {
    /// A handle to nothing, for unit tests that need a `RustdvCtx` but never
    /// touch the DUT. Any VPI call through it goes to `rustdv-vpi-stubs`,
    /// which panics — so a test that *does* touch the DUT fails loudly
    /// instead of reading garbage.
    pub fn null_for_test() -> HierarchyHandle {
        HierarchyHandle { h: ObjHandle(std::ptr::null_mut()) }
    }

    /// Dynamic child lookup: `dut.child("clk")?` (design-doc OQ-6 lean).
    pub fn child(&self, name: &str) -> Result<AnyHandle, HandleError> {
        let cname = CString::new(name).expect("NUL in signal name");
        let h = unsafe { sys::vpi_handle_by_name(cname.as_ptr(), self.h.0) };
        match ObjHandle::new(h) {
            Some(h) => Ok(AnyHandle::classify(h)),
            None => Err(HandleError::NotFound { name: name.into(), scope: self.full_name() }),
        }
    }

    /// Shorthand: child that must be a signal.
    pub fn signal(&self, name: &str) -> Result<LogicHandle, HandleError> {
        self.child(name)?.as_logic()
    }

    pub fn name(&self) -> String {
        self.h.get_str(sys::vpiName)
    }
    pub fn full_name(&self) -> String {
        self.h.get_str(sys::vpiFullName)
    }

    /// Iterate child objects (modules, nets, regs) — serves the
    /// `visit_children` debug-print role at the DUT level.
    pub fn children(&self) -> Vec<AnyHandle> {
        let mut out = Vec::new();
        for t in [sys::vpiModule, sys::vpiNet, sys::vpiReg] {
            unsafe {
                let it = sys::vpi_iterate(t, self.h.0);
                if it.is_null() {
                    continue;
                }
                loop {
                    let c = sys::vpi_scan(it);
                    if c.is_null() {
                        break; // scan returning NULL frees the iterator
                    }
                    if let Some(h) = ObjHandle::new(c) {
                        out.push(AnyHandle::classify(h));
                    }
                }
            }
        }
        out
    }
}

/// A value-bearing signal handle (net/reg/var). Port of cocotb's
/// LogicObject surface: explicit `get()`/`set()` (mapping row 20).
/// Buffered-write semantics live a layer up in rustdv-sim; the methods
/// here apply values immediately.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct LogicHandle {
    h: ObjHandle,
    width: u32,
}

impl LogicHandle {
    pub fn name(&self) -> String {
        self.h.get_str(sys::vpiName)
    }
    pub fn full_name(&self) -> String {
        self.h.get_str(sys::vpiFullName)
    }
    pub fn size(&self) -> u32 {
        self.width
    }

    /// Current value as a binary string, e.g. "0101", "xxxx".
    pub fn get_binstr(&self) -> String {
        let mut val = sys::t_vpi_value {
            format: sys::vpiBinStrVal,
            value: sys::u_vpi_value_union { integer: 0 },
        };
        unsafe {
            sys::vpi_get_value(self.h.0, &mut val);
            let p = val.value.str_;
            if p.is_null() {
                String::new()
            } else {
                CStr::from_ptr(p).to_string_lossy().into_owned()
            }
        }
    }

    /// Current value as a LogicArray (4-state).
    pub fn get(&self) -> LogicArray {
        LogicArray::from_binstr(&self.get_binstr())
    }

    /// Current value as u64; `Err` if any bit is x/z (design-doc §0.6:
    /// conversion failures are Results, not exceptions).
    pub fn get_u64(&self) -> Result<u64, ValueError> {
        let width = self.size().max(1);
        if width > 64 {
            return Err(ValueError::Width {
                want: width,
                have: 64,
            });
        }
        let mut val = sys::t_vpi_value {
            format: sys::vpiVectorVal,
            value: sys::u_vpi_value_union {
                vector: std::ptr::null_mut(),
            },
        };
        unsafe {
            sys::vpi_get_value(self.h.0, &mut val);
            let words = val.value.vector;
            if words.is_null() {
                return Ok(0);
            }
            let word_count = width.div_ceil(32) as usize;
            let mut result = 0u64;
            for index in 0..word_count {
                let word = *words.add(index);
                let used_bits = if index + 1 == word_count && !width.is_multiple_of(32) {
                    width % 32
                } else {
                    32
                };
                let mask = if used_bits == 32 {
                    u32::MAX
                } else {
                    (1u32 << used_bits) - 1
                };
                if word.bval & mask != 0 {
                    return Err(ValueError::FourState(self.get_binstr()));
                }
                result |= ((word.aval & mask) as u64) << (index * 32);
            }
            Ok(result)
        }
    }

    fn put_binstr_flags(&self, bin: &str, flags: i32) {
        let c = CString::new(bin).expect("NUL in binstr");
        let mut val = sys::t_vpi_value {
            format: sys::vpiBinStrVal,
            value: sys::u_vpi_value_union { str_: c.as_ptr() as *mut _ },
        };
        unsafe {
            sys::vpi_put_value(self.h.0, &mut val, std::ptr::null_mut(), flags);
        }
    }

    fn put_vector_words(&self, words: &mut [sys::t_vpi_vecval]) {
        let mut val = sys::t_vpi_value {
            format: sys::vpiVectorVal,
            value: sys::u_vpi_value_union {
                vector: words.as_mut_ptr(),
            },
        };
        unsafe {
            sys::vpi_put_value(self.h.0, &mut val, std::ptr::null_mut(), sys::vpiNoDelay);
        }
    }

    /// Immediate (NoDelay) write of an integer value, zero-extended /
    /// truncated to the signal width. This is the "setimmediatevalue"
    /// analog; scheduled writes are layered above (design-doc §4.1(4)).
    pub fn set_u64_now(&self, v: u64) {
        let width = self.size().max(1);
        let word_count = width.div_ceil(32) as usize;
        let mut words = if word_count <= 2 {
            let words = [
                sys::t_vpi_vecval {
                    aval: v as u32,
                    bval: 0,
                },
                sys::t_vpi_vecval {
                    aval: (v >> 32) as u32,
                    bval: 0,
                },
            ];
            words[..word_count].to_vec()
        } else {
            let mut words = vec![sys::t_vpi_vecval::default(); word_count];
            words[0].aval = v as u32;
            words[1].aval = (v >> 32) as u32;
            words
        };
        if !width.is_multiple_of(32) {
            let mask = (1u32 << (width % 32)) - 1;
            let last = words.last_mut().expect("positive width has a vector word");
            last.aval &= mask;
            last.bval &= mask;
        }
        self.put_vector_words(&mut words);
    }

    /// Immediate write of a 4-state value.
    pub fn set_now(&self, v: &LogicArray) {
        self.put_binstr_flags(&v.to_binstr(), sys::vpiNoDelay);
    }
}

/// All top-level modules in the design.
pub fn top_modules() -> Vec<HierarchyHandle> {
    let mut out = Vec::new();
    unsafe {
        let it = sys::vpi_iterate(sys::vpiModule, std::ptr::null_mut());
        if it.is_null() {
            return out;
        }
        loop {
            let m = sys::vpi_scan(it);
            if m.is_null() {
                break;
            }
            if let Some(h) = ObjHandle::new(m) {
                out.push(HierarchyHandle { h });
            }
        }
    }
    out
}

/// The first top-level module (the DUT in single-top designs).
pub fn top_module() -> Result<HierarchyHandle, HandleError> {
    top_modules().into_iter().next().ok_or(HandleError::NoTopModule)
}

// ===========================================================================
// Time
// ===========================================================================

/// Current simulation time in simulator precision steps.
pub fn sim_time_steps() -> u64 {
    let mut t = sys::t_vpi_time { type_: sys::vpiSimTime, high: 0, low: 0, real: 0.0 };
    unsafe { sys::vpi_get_time(std::ptr::null_mut(), &mut t) };
    ((t.high as u64) << 32) | (t.low as u64)
}

/// Simulator time precision as a power of ten (e.g. -9 = 1 ns).
pub fn time_precision() -> i32 {
    thread_local! {
        static PREC: Cell<Option<i32>> = const { Cell::new(None) };
    }
    PREC.with(|p| match p.get() {
        Some(v) => v,
        None => {
            let v = unsafe { sys::vpi_get(sys::vpiTimePrecision, std::ptr::null_mut()) };
            p.set(Some(v));
            v
        }
    })
}

/// End the simulation (vpi_control(vpiFinish)).
pub fn finish() {
    unsafe {
        sys::vpi_control(sys::vpiFinish, 0i32);
    }
}

// ===========================================================================
// Panic sink (invariant 4)
// ===========================================================================

thread_local! {
    static PANIC_SINK: RefCell<Option<Box<dyn Fn(String)>>> = const { RefCell::new(None) };
}

/// Install the handler invoked when a callback closure panics (the runner
/// routes this to "fail the current test", mirroring cocotb catching
/// BaseException per task).
pub fn set_panic_sink(f: Box<dyn Fn(String)>) {
    PANIC_SINK.with(|s| *s.borrow_mut() = Some(f));
}

fn report_panic(payload: Box<dyn std::any::Any + Send>) {
    let msg = if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "panic (non-string payload)".to_string()
    };
    PANIC_SINK.with(|s| {
        if let Some(f) = s.borrow().as_ref() {
            f(msg.clone());
        } else {
            eprintln!("rustdv: panic in simulator callback: {msg}");
        }
    });
}

// ===========================================================================
// Callbacks (invariant 5)
// ===========================================================================

enum CbKind {
    OneShot,
    Recurring,
}

struct CbShared {
    kind: CbKind,
    /// Registration handle returned by vpi_register_cb. One-shots remove it
    /// from inside the trampoline, while it is valid on both supported
    /// simulators: Icarus reaps the active callback after it returns and
    /// Verilator releases its separately-owned handle object immediately.
    vpi_h: Cell<sys::vpiHandle>,
    /// True once the C-side Rc reference has been reclaimed (fired one-shot
    /// or removed callback). Guards against double-free.
    released: Cell<bool>,
    once: RefCell<Option<Box<dyn FnOnce()>>>,
    repeat: RefCell<Option<Box<dyn FnMut()>>>,
}

/// RAII callback registration. Dropping an unfired/live handle removes the
/// simulator callback — this is what makes drop-based task cancellation
/// (design-doc §4.6) clean up trigger registrations for free.
pub struct CallbackHandle {
    shared: Rc<CbShared>,
    raw: *const CbShared,
    detached: bool,
}

impl CallbackHandle {
    /// Detach Rust ownership without leaking it. A detached one-shot keeps its
    /// C-side reference until it fires and then self-cleans; a detached
    /// recurring callback lives for the rest of the simulation.
    pub fn forget(mut self) {
        self.detached = true;
    }
}

impl Drop for CallbackHandle {
    fn drop(&mut self) {
        if self.detached {
            return;
        }
        if !self.shared.released.get() {
            self.shared.released.set(true);
            unsafe {
                sys::vpi_remove_cb(self.shared.vpi_h.get());
                // Reclaim the C-side reference.
                drop(Rc::from_raw(self.raw));
            }
        }
    }
}

extern "C" fn trampoline(cb: *mut sys::t_cb_data) -> i32 {
    unsafe {
        let ud = (*cb).user_data as *const CbShared;
        if ud.is_null() {
            return 0;
        }
        // Hold our own reference for the duration of the call so that
        // closures dropping the CallbackHandle can't free us mid-flight.
        Rc::increment_strong_count(ud);
        let shared: Rc<CbShared> = Rc::from_raw(ud);
        match shared.kind {
            CbKind::OneShot => {
                if !shared.released.get() {
                    shared.released.set(true);
                    let f = shared.once.borrow_mut().take();
                    // The returned callback handle has different post-fire
                    // ownership across simulators. Remove it while the active
                    // callback is still valid on both: Icarus marks it for
                    // self-reaping, while Verilator deletes the retained
                    // VerilatedVpioReasonCb handle.
                    sys::vpi_remove_cb(shared.vpi_h.get());
                    // Reclaim the C-side reference before running user code.
                    drop(Rc::from_raw(ud));
                    if let Some(f) = f {
                        if let Err(p) = catch_unwind(AssertUnwindSafe(f)) {
                            report_panic(p);
                        }
                    }
                }
            }
            CbKind::Recurring => {
                let mut guard = shared.repeat.borrow_mut();
                if let Some(f) = guard.as_mut() {
                    if let Err(p) = catch_unwind(AssertUnwindSafe(|| f())) {
                        report_panic(p);
                    }
                }
            }
        }
        drop(shared);
    }
    0
}

fn register(
    kind: CbKind,
    once: Option<Box<dyn FnOnce()>>,
    repeat: Option<Box<dyn FnMut()>>,
    reason: i32,
    obj: sys::vpiHandle,
    time: Option<sys::t_vpi_time>,
) -> CallbackHandle {
    let shared = Rc::new(CbShared {
        kind,
        vpi_h: Cell::new(std::ptr::null_mut()),
        released: Cell::new(false),
        once: RefCell::new(once),
        repeat: RefCell::new(repeat),
    });
    // C-side reference:
    let raw = Rc::into_raw(shared.clone());

    let mut t = time.unwrap_or(sys::t_vpi_time {
        type_: sys::vpiSuppressTime,
        high: 0,
        low: 0,
        real: 0.0,
    });
    // value: NULL — closures read signal values themselves; Icarus rejects
    // vpiSuppressVal on value-change callbacks ("format 10 not supported").
    let mut cb = sys::t_cb_data {
        reason,
        cb_rtn: Some(trampoline),
        obj,
        time: &mut t,
        value: std::ptr::null_mut(),
        index: 0,
        user_data: raw as *mut _,
    };
    let vpi_h = unsafe { sys::vpi_register_cb(&mut cb) };
    assert!(!vpi_h.is_null(), "vpi_register_cb failed (reason {reason})");
    shared.vpi_h.set(vpi_h);
    CallbackHandle { shared, raw, detached: false }
}

fn simtime(steps: u64) -> sys::t_vpi_time {
    sys::t_vpi_time {
        type_: sys::vpiSimTime,
        high: (steps >> 32) as u32,
        low: (steps & 0xFFFF_FFFF) as u32,
        real: 0.0,
    }
}

/// One-shot callback after `steps` precision units (cbAfterDelay).
pub fn register_timer(steps: u64, f: Box<dyn FnOnce()>) -> CallbackHandle {
    register(CbKind::OneShot, Some(f), None, sys::cbAfterDelay, std::ptr::null_mut(), Some(simtime(steps)))
}

/// Recurring callback on any value change of `sig` (cbValueChange). The
/// closure reads the signal itself; edge filtering happens in rustdv-sim.
pub fn register_value_change(sig: LogicHandle, f: Box<dyn FnMut()>) -> CallbackHandle {
    register(
        CbKind::Recurring,
        None,
        Some(f),
        sys::cbValueChange,
        sig.h.0,
        Some(simtime(0)),
    )
}

/// One-shot callback at the next ReadWrite synch point.
pub fn register_read_write(f: Box<dyn FnOnce()>) -> CallbackHandle {
    register(CbKind::OneShot, Some(f), None, sys::cbReadWriteSynch, std::ptr::null_mut(), Some(simtime(0)))
}

/// One-shot callback at the next ReadOnly synch point.
pub fn register_read_only(f: Box<dyn FnOnce()>) -> CallbackHandle {
    register(CbKind::OneShot, Some(f), None, sys::cbReadOnlySynch, std::ptr::null_mut(), Some(simtime(0)))
}

/// One-shot callback at the next simulation time step.
pub fn register_next_sim_time(f: Box<dyn FnOnce()>) -> CallbackHandle {
    register(CbKind::OneShot, Some(f), None, sys::cbNextSimTime, std::ptr::null_mut(), None)
}

/// One-shot callback at the end of the current simulation time step.
pub fn register_at_end_of_sim_time(f: Box<dyn FnOnce()>) -> CallbackHandle {
    register(
        CbKind::OneShot,
        Some(f),
        None,
        sys::cbAtEndOfSimTime,
        std::ptr::null_mut(),
        Some(simtime(sim_time_steps())),
    )
}

/// One-shot callback at start of simulation (the bootstrap hook).
pub fn register_start_of_simulation(f: Box<dyn FnOnce()>) -> CallbackHandle {
    register(CbKind::OneShot, Some(f), None, sys::cbStartOfSimulation, std::ptr::null_mut(), None)
}

/// One-shot callback at end of simulation.
pub fn register_end_of_simulation(f: Box<dyn FnOnce()>) -> CallbackHandle {
    register(CbKind::OneShot, Some(f), None, sys::cbEndOfSimulation, std::ptr::null_mut(), None)
}

#[cfg(test)]
mod callback_tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn fired_one_shot_releases_its_vpi_handle() {
        rustdv_vpi_stubs::reset_callbacks();
        let fired = Rc::new(Cell::new(false));
        let fired_in_callback = fired.clone();
        let handle = register_timer(1, Box::new(move || fired_in_callback.set(true)));

        assert_eq!(rustdv_vpi_stubs::live_callback_handles(), 1);
        rustdv_vpi_stubs::fire_next_callback();
        assert!(fired.get());
        drop(handle);

        assert_eq!(rustdv_vpi_stubs::live_callback_handles(), 0);
        rustdv_vpi_stubs::reset_callbacks();
    }

    #[test]
    fn detached_one_shot_releases_shared_state_after_firing() {
        rustdv_vpi_stubs::reset_callbacks();
        let handle = register_timer(1, Box::new(|| {}));
        let shared = Rc::downgrade(&handle.shared);

        handle.forget();
        assert!(shared.upgrade().is_some());
        rustdv_vpi_stubs::fire_next_callback();

        assert_eq!(rustdv_vpi_stubs::live_callback_handles(), 0);
        assert!(shared.upgrade().is_none());
        rustdv_vpi_stubs::reset_callbacks();
    }

    #[test]
    fn callback_stub_refuses_to_reset_a_live_handle() {
        rustdv_vpi_stubs::reset_callbacks();
        let handle = register_timer(1, Box::new(|| {}));

        let reset = catch_unwind(AssertUnwindSafe(rustdv_vpi_stubs::reset_callbacks));
        assert!(reset.is_err());

        drop(handle);
        rustdv_vpi_stubs::reset_callbacks();
    }
}

#[cfg(test)]
mod handle_tests {
    use super::*;

    fn make_signal(width: i32, words: &[sys::t_vpi_vecval], binstr: &str) -> LogicHandle {
        rustdv_vpi_stubs::configure_signal(width, words, binstr);
        let raw = 1usize as sys::vpiHandle;
        let AnyHandle::Logic(signal) = AnyHandle::classify(ObjHandle(raw)) else {
            panic!("stub object was not classified as a signal");
        };
        signal
    }

    #[test]
    fn signal_width_is_cached_when_the_handle_is_classified() {
        rustdv_vpi_stubs::reset_property_gets();
        let signal = make_signal(37, &[sys::t_vpi_vecval::default(); 2], &"0".repeat(37));

        assert_eq!(signal.size(), 37);
        assert_eq!(signal.size(), 37);
        assert_eq!(rustdv_vpi_stubs::property_get_count(sys::vpiType), 1);
        assert_eq!(rustdv_vpi_stubs::property_get_count(sys::vpiSize), 1);
        rustdv_vpi_stubs::reset_property_gets();
    }

    #[test]
    fn vector_read_uses_vpi_word_order_and_masks_unused_bits() {
        let signal = make_signal(
            37,
            &[
                sys::t_vpi_vecval {
                    aval: 0x89ab_cdef,
                    bval: 0,
                },
                sys::t_vpi_vecval {
                    aval: 0xffff_fff5,
                    bval: 0xffff_ffe0,
                },
            ],
            "101011001101010111100110111101111",
        );

        assert_eq!(signal.get_u64(), Ok(0x0000_0015_89ab_cdef));
    }

    #[test]
    fn vector_read_reports_xz_in_used_bits() {
        let signal = make_signal(
            8,
            &[sys::t_vpi_vecval {
                aval: 0x5a,
                bval: 0x04,
            }],
            "01011x10",
        );

        assert_eq!(
            signal.get_u64(),
            Err(ValueError::FourState("01011x10".to_owned()))
        );
    }

    #[test]
    fn vector_write_truncates_and_zero_extends_to_signal_width() {
        let signal = make_signal(37, &[sys::t_vpi_vecval::default(); 2], &"0".repeat(37));
        signal.set_u64_now(0xffff_fff5_89ab_cdef);
        let words = rustdv_vpi_stubs::last_put_vector();

        assert_eq!(words.len(), 2);
        assert_eq!(words[0].aval, 0x89ab_cdef);
        assert_eq!(words[1].aval, 0x15);
        assert_eq!(words[0].bval, 0);
        assert_eq!(words[1].bval, 0);

        let wide = make_signal(70, &[sys::t_vpi_vecval::default(); 3], &"0".repeat(70));
        wide.set_u64_now(0x0123_4567_89ab_cdef);
        let words = rustdv_vpi_stubs::last_put_vector();
        assert_eq!(words.len(), 3);
        assert_eq!(words[0].aval, 0x89ab_cdef);
        assert_eq!(words[1].aval, 0x0123_4567);
        assert_eq!(words[2].aval, 0);
    }
}
