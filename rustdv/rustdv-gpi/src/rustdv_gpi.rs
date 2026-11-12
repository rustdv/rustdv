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
//!
//! Backends: every simulator access goes through the [`backend::Backend`]
//! installed on the current thread. By default the simulator loads the
//! testbench as a VPI module (the `vpi` feature); another host
//! installs its own backend. The handle and callback API below is the same
//! for all of them.

use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

pub use num_bigint::BigUint;
use rustdv_gpi_sys::{self as sys, t_vpi_vecval};

// Test executables need vpi_* symbol definitions (the simulator provides
// them for the real cdylib) — see rustdv-vpi-stubs.
#[cfg(all(test, feature = "vpi"))]
use rustdv_vpi_stubs as _;

pub mod value;
pub use value::{Logic, LogicArray};
pub mod util;
pub use util::ToVpiWords;

pub mod backend;
pub use backend::ObjHandle;
use backend::{CbData, ObjKind, Reason};

// Private: its operations trust their arguments (see the `Backend` method
// docs), which only this crate's typed API guarantees.
#[cfg(feature = "vpi")]
mod vpi;

impl ObjHandle {
    fn name(self) -> String {
        backend::current().name(self)
    }
    fn full_name(self) -> String {
        backend::current().full_name(self)
    }
    fn type_desc(self) -> String {
        backend::current().type_desc(self)
    }
    fn get_vector(self, width: u32) -> Result<Cow<'static, [t_vpi_vecval]>, ValueError> {
        backend::current().get_vector(self, width)
    }
    fn put_vector(self, words: &mut [t_vpi_vecval], flags: i32) {
        backend::current().put_vector(self, words, flags)
    }
}

// ===========================================================================
// Time and control
// ===========================================================================

/// Current simulation time in simulator precision steps.
pub fn sim_time_steps() -> u64 {
    backend::current().sim_time_steps()
}

/// Simulator time precision as a power of ten (e.g. -9 = 1 ns).
pub fn time_precision() -> i32 {
    backend::current().time_precision()
}

/// End the simulation.
pub fn finish() {
    backend::current().finish()
}

/// Record that the regression failed. A VPI simulator has no pass/fail
/// verdict of its own; the runner's `REGRESSION:` line carries it.
pub fn report_failure(msg: &str) {
    backend::current().report_failure(msg)
}

/// Write one line of testbench output.
pub fn console(line: &str) {
    backend::current().console(line)
}

// ===========================================================================
// Errors
// ===========================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandleError {
    /// Name did not resolve (cocotb raises AttributeError here; rustdv
    /// returns this — design-doc §0.6).
    NotFound {
        name: String,
        scope: String,
    },
    /// Handle exists but is not the requested kind.
    WrongKind {
        name: String,
        expected: &'static str,
        actual: String,
    },
    NoTopModule,
}

impl fmt::Display for HandleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HandleError::NotFound { name, scope } => {
                write!(f, "no object named '{name}' in scope '{scope}'")
            }
            HandleError::WrongKind {
                name,
                expected,
                actual,
            } => {
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
    /// The simulator did not supply the requested value representation.
    Unavailable,
    /// VPI strings cannot contain embedded NUL bytes.
    InteriorNul,
    Width {
        want: u32,
        have: usize,
    },
}

impl fmt::Display for ValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValueError::FourState(s) => write!(f, "value '{s}' has x/z bits"),
            ValueError::Unavailable => write!(f, "simulator did not return the requested value"),
            ValueError::InteriorNul => write!(f, "VPI string contains an embedded NUL byte"),
            ValueError::Width { want, have } => {
                write!(f, "width mismatch: want {want}, have {have}")
            }
        }
    }
}
impl std::error::Error for ValueError {}

// ===========================================================================
// Handles
// ===========================================================================

/// Any simulator object, classified by type (design-doc §3.3 downcasting).
#[derive(Copy, Clone)]
pub enum AnyHandle {
    Hierarchy(HierarchyHandle),
    Logic(LogicHandle),
    Real(RealHandle),
    String(StringHandle),
    Struct(AggregateHandle),
    Union(AggregateHandle),
    Other(ObjHandle),
}

impl AnyHandle {
    pub fn classify(h: ObjHandle) -> AnyHandle {
        match backend::current().kind(h) {
            ObjKind::Hierarchy => AnyHandle::Hierarchy(HierarchyHandle { h }),
            // Signal width is immutable for the lifetime of an object. Cache
            // it at discovery so every value read/write does not pay for
            // another backend query.
            ObjKind::Logic { width } => AnyHandle::Logic(LogicHandle { h, width }),
            ObjKind::Real => AnyHandle::Real(RealHandle { h }),
            ObjKind::String => AnyHandle::String(StringHandle { h }),
            ObjKind::Struct => AnyHandle::Struct(AggregateHandle { h }),
            ObjKind::Union => AnyHandle::Union(AggregateHandle { h }),
            ObjKind::Other => AnyHandle::Other(h),
        }
    }

    fn object(self) -> ObjHandle {
        match self {
            Self::Hierarchy(h) => h.h,
            Self::Logic(h) => h.h,
            Self::Real(h) => h.h,
            Self::String(h) => h.h,
            Self::Struct(h) | Self::Union(h) => h.h,
            Self::Other(h) => h,
        }
    }

    fn wrong_kind(self, expected: &'static str) -> HandleError {
        let h = self.object();
        HandleError::WrongKind {
            name: h.full_name(),
            expected,
            actual: h.type_desc(),
        }
    }

    pub fn as_logic(self) -> Result<LogicHandle, HandleError> {
        match self {
            Self::Logic(h) => Ok(h),
            other => Err(other.wrong_kind("signal")),
        }
    }

    pub fn as_hierarchy(self) -> Result<HierarchyHandle, HandleError> {
        match self {
            Self::Hierarchy(h) => Ok(h),
            other => Err(other.wrong_kind("module")),
        }
    }

    pub fn as_real(self) -> Result<RealHandle, HandleError> {
        match self {
            Self::Real(h) => Ok(h),
            other => Err(other.wrong_kind("real")),
        }
    }

    pub fn as_string(self) -> Result<StringHandle, HandleError> {
        match self {
            Self::String(h) => Ok(h),
            other => Err(other.wrong_kind("string")),
        }
    }

    pub fn as_aggregate(self) -> Result<AggregateHandle, HandleError> {
        match self {
            Self::Struct(h) | Self::Union(h) => Ok(h),
            other => Err(other.wrong_kind("struct or union")),
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
        HierarchyHandle {
            h: ObjHandle::null(),
        }
    }

    /// Dynamic child lookup: `dut.child("clk")?` (design-doc OQ-6 lean).
    pub fn child(&self, name: &str) -> Result<AnyHandle, HandleError> {
        match backend::current().child(self.h, name) {
            Some(h) => Ok(AnyHandle::classify(h)),
            None => Err(HandleError::NotFound {
                name: name.into(),
                scope: self.full_name(),
            }),
        }
    }

    /// Shorthand: child that must be a signal.
    pub fn signal(&self, name: &str) -> Result<LogicHandle, HandleError> {
        self.child(name)?.as_logic()
    }

    pub fn name(&self) -> String {
        self.h.name()
    }
    pub fn full_name(&self) -> String {
        self.h.full_name()
    }

    /// Iterate child objects (modules, nets, regs) — serves the
    /// `visit_children` debug-print role at the DUT level.
    pub fn children(&self) -> Vec<AnyHandle> {
        backend::current()
            .children(self.h)
            .into_iter()
            .map(AnyHandle::classify)
            .collect()
    }

    pub fn real(&self, name: &str) -> Result<RealHandle, HandleError> {
        self.child(name)?.as_real()
    }
    pub fn string(&self, name: &str) -> Result<StringHandle, HandleError> {
        self.child(name)?.as_string()
    }
    pub fn aggregate(&self, name: &str) -> Result<AggregateHandle, HandleError> {
        self.child(name)?.as_aggregate()
    }
}

/// A simulator real variable.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct RealHandle {
    h: ObjHandle,
}

impl RealHandle {
    pub fn name(&self) -> String {
        self.h.name()
    }
    pub fn full_name(&self) -> String {
        self.h.full_name()
    }
    pub fn get(&self) -> Result<f64, ValueError> {
        backend::current().get_real(self.h)
    }
    pub fn set_now(&self, value: f64) {
        backend::current().put_real(self.h, value);
    }
}

/// A simulator string variable.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct StringHandle {
    h: ObjHandle,
}

impl StringHandle {
    pub fn name(&self) -> String {
        self.h.name()
    }
    pub fn full_name(&self) -> String {
        self.h.full_name()
    }
    pub fn get(&self) -> Result<String, ValueError> {
        backend::current().get_string(self.h)
    }
    pub fn set_now(&self, value: &str) -> Result<(), ValueError> {
        backend::current().put_string(self.h, value)
    }
}

/// An unpacked struct or union. Read and write through its typed members.
#[derive(Copy, Clone)]
pub struct AggregateHandle {
    h: ObjHandle,
}

impl AggregateHandle {
    pub fn name(&self) -> String {
        self.h.name()
    }
    pub fn full_name(&self) -> String {
        self.h.full_name()
    }
    pub fn child(&self, name: &str) -> Result<AnyHandle, HandleError> {
        backend::current()
            .member(self.h, name)
            .map(AnyHandle::classify)
            .ok_or_else(|| HandleError::NotFound {
                name: name.into(),
                scope: self.full_name(),
            })
    }
    pub fn children(&self) -> Vec<AnyHandle> {
        backend::current()
            .members(self.h)
            .into_iter()
            .map(AnyHandle::classify)
            .collect()
    }
    pub fn signal(&self, name: &str) -> Result<LogicHandle, HandleError> {
        self.child(name)?.as_logic()
    }
}

impl AggregateHandle {
    pub fn real(&self, name: &str) -> Result<RealHandle, HandleError> {
        self.child(name)?.as_real()
    }
    pub fn string(&self, name: &str) -> Result<StringHandle, HandleError> {
        self.child(name)?.as_string()
    }
    pub fn aggregate(&self, name: &str) -> Result<AggregateHandle, HandleError> {
        self.child(name)?.as_aggregate()
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
        self.h.name()
    }
    pub fn full_name(&self) -> String {
        self.h.full_name()
    }
    pub fn size(&self) -> u32 {
        self.width
    }

    /// Current value as a slice of VPI vector words. `Err` if the simulator
    /// did not supply a vector representation (design-doc §0.6: conversion
    /// failures are Results, not exceptions).
    fn get_vpi_words(&self) -> Result<Cow<'_, [sys::t_vpi_vecval]>, ValueError> {
        self.h.get_vector(self.size())
    }

    /// Masks one VPI vector word to the signal width. `Err` if any used bit is
    /// x/z (design-doc §0.6: conversion failures are Results, not exceptions).
    ///
    /// # Arguments
    ///
    /// * `word` - Individual VPI vector word from [`Self::get_vpi_words`].
    /// * `width` - Width of the entire signal in bits.
    /// * `last` - Whether `word` is the signal's final VPI vector word.
    fn get_vpi_word(&self, word: &t_vpi_vecval, width: u32, last: bool) -> Result<u32, ValueError> {
        let used_bits = if last && !width.is_multiple_of(32) {
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
            return Err(ValueError::FourState(self.get_logic()?.to_binstr()));
        }
        Ok(word.aval & mask)
    }

    /// Check that the signal width is at most `want` bits. Returns `Err` if
    /// the signal is wider than `want` (design-doc §0.6: conversion failures
    /// are Results, not exceptions).
    fn check_size(&self, want: u32) -> Result<(), ValueError> {
        if self.size() > want {
            return Err(ValueError::Width {
                want,
                have: self.size() as usize,
            });
        }
        Ok(())
    }

    /// Current value as bool; `Err` if the signal is wider than one bit or its
    /// value is x/z (design-doc §0.6: conversion failures are Results, not
    /// exceptions).
    pub fn get_bool(&self) -> Result<bool, ValueError> {
        self.check_size(1)?;
        Ok(self.get_vpi_word(&self.get_vpi_words()?[0], self.size(), true)? != 0)
    }

    /// Current value as u8; `Err` if any bit is x/z (design-doc §0.6:
    /// conversion failures are Results, not exceptions).
    pub fn get_u8(&self) -> Result<u8, ValueError> {
        self.check_size(u8::BITS)?;
        Ok(self.get_vpi_word(&self.get_vpi_words()?[0], self.size(), true)? as u8)
    }

    /// Current value as u16; `Err` if any bit is x/z (design-doc §0.6:
    /// conversion failures are Results, not exceptions).
    pub fn get_u16(&self) -> Result<u16, ValueError> {
        self.check_size(u16::BITS)?;
        Ok(self.get_vpi_word(&self.get_vpi_words()?[0], self.size(), true)? as u16)
    }

    /// Current value as u32; `Err` if any bit is x/z (design-doc §0.6:
    /// conversion failures are Results, not exceptions).
    pub fn get_u32(&self) -> Result<u32, ValueError> {
        self.check_size(u32::BITS)?;
        self.get_vpi_word(&self.get_vpi_words()?[0], self.size(), true)
    }

    /// Current value as u64; `Err` if any bit is x/z (design-doc §0.6:
    /// conversion failures are Results, not exceptions).
    pub fn get_u64(&self) -> Result<u64, ValueError> {
        self.check_size(u64::BITS)?;
        let words = self.get_vpi_words()?;
        let mut result = 0;
        for (index, word) in words.iter().enumerate() {
            let word = self.get_vpi_word(word, self.size(), index + 1 == words.len())?;
            result |= (word as u64) << (index * 32);
        }
        Ok(result)
    }

    /// Current value as u128; `Err` if any bit is x/z (design-doc §0.6:
    /// conversion failures are Results, not exceptions).
    pub fn get_u128(&self) -> Result<u128, ValueError> {
        self.check_size(u128::BITS)?;
        let words = self.get_vpi_words()?;
        let mut result = 0;
        for (index, word) in words.iter().enumerate() {
            let word = self.get_vpi_word(word, self.size(), index + 1 == words.len())?;
            result |= (word as u128) << (index * 32);
        }
        Ok(result)
    }

    /// Current value as an unsigned arbitrary-precision integer; `Err` if
    /// any bit is x/z.
    pub fn get_bigint(&self) -> Result<BigUint, ValueError> {
        let words = self.get_vpi_words()?;
        let mut digits = Vec::with_capacity(words.len());
        for (index, word) in words.iter().enumerate() {
            digits.push(self.get_vpi_word(word, self.size(), index + 1 == words.len())?);
        }
        Ok(BigUint::from_slice(&digits))
    }

    /// Current value as a LogicArray (4-state). `Err` if the simulator does
    /// not supply a VPI vector representation.
    pub fn get_logic(&self) -> Result<LogicArray, ValueError> {
        let words = self.get_vpi_words()?;
        let words: Vec<(u32, u32)> = words.iter().map(|word| (word.aval, word.bval)).collect();
        Ok(LogicArray::from_vpi_words(&words, self.size() as usize))
    }

    /// Mask unused bits in the final VPI vector word to the signal width.
    fn mask_vector_words(&self, words: &mut [sys::t_vpi_vecval]) {
        if !self.size().is_multiple_of(32) {
            let mask = (1u32 << (self.size() % 32)) - 1;
            let last = words.last_mut().expect("partial width has a vector word");
            last.aval &= mask;
            last.bval &= mask;
        }
    }

    /// Resize vector words to the signal width and hand them to the backend.
    /// Words can be any length: they are truncated or zero-padded first.
    fn set_vector_words(&self, words: &mut [sys::t_vpi_vecval], flags: i32) {
        let word_count = self.size().div_ceil(32) as usize;
        if words.len() >= word_count {
            // Truncate to the signal width and mask the final word
            let words = &mut words[..word_count];
            self.mask_vector_words(words);
            self.h.put_vector(words, flags);
        } else {
            // Word length is less, pad with zeros to the signal width
            let mut words = words.to_vec();
            words.resize(word_count, sys::t_vpi_vecval::default());
            self.h.put_vector(&mut words, flags);
        }
    }

    /// Write a bool with the supplied VPI flag, zero-extended to the signal
    /// width.
    pub fn set_bool(&self, v: bool, flags: i32) {
        self.set_vector_words(&mut v.to_vpi_words(), flags);
    }

    /// Immediate (NoDelay) write of a bool.
    pub fn set_bool_now(&self, v: bool) {
        self.set_bool(v, sys::vpiNoDelay);
    }

    /// Write a u8 with the supplied VPI flag, zero-extended / truncated to the
    /// signal width.
    pub fn set_u8(&self, v: u8, flags: i32) {
        self.set_vector_words(&mut v.to_vpi_words(), flags);
    }

    /// Immediate (NoDelay) write of a u8.
    pub fn set_u8_now(&self, v: u8) {
        self.set_u8(v, sys::vpiNoDelay);
    }

    /// Write a u16 with the supplied VPI flag, zero-extended / truncated to
    /// the signal width.
    pub fn set_u16(&self, v: u16, flags: i32) {
        self.set_vector_words(&mut v.to_vpi_words(), flags);
    }

    /// Immediate (NoDelay) write of a u16.
    pub fn set_u16_now(&self, v: u16) {
        self.set_u16(v, sys::vpiNoDelay);
    }

    /// Write a u32 with the supplied VPI flag, zero-extended / truncated to
    /// the signal width.
    pub fn set_u32(&self, v: u32, flags: i32) {
        self.set_vector_words(&mut v.to_vpi_words(), flags);
    }

    /// Immediate (NoDelay) write of a u32.
    pub fn set_u32_now(&self, v: u32) {
        self.set_u32(v, sys::vpiNoDelay);
    }

    /// Write a u64 with the supplied VPI flag, zero-extended / truncated to
    /// the signal width.
    pub fn set_u64(&self, v: u64, flags: i32) {
        self.set_vector_words(&mut v.to_vpi_words(), flags);
    }

    /// Immediate (NoDelay) write of a u64.
    pub fn set_u64_now(&self, v: u64) {
        self.set_u64(v, sys::vpiNoDelay);
    }

    /// Write a u128 with the supplied VPI flag, zero-extended / truncated to
    /// the signal width.
    pub fn set_u128(&self, v: u128, flags: i32) {
        self.set_vector_words(&mut v.to_vpi_words(), flags);
    }

    /// Immediate (NoDelay) write of a u128.
    pub fn set_u128_now(&self, v: u128) {
        self.set_u128(v, sys::vpiNoDelay);
    }

    /// Write an arbitrary-precision integer with the supplied VPI flag,
    /// zero-extended / truncated to the signal width.
    pub fn set_bigint(&self, v: &BigUint, flags: i32) {
        let mut words: Vec<sys::t_vpi_vecval> = v
            .iter_u32_digits()
            .map(|aval| sys::t_vpi_vecval { aval, bval: 0 })
            .collect();
        self.set_vector_words(&mut words, flags);
    }

    /// Immediate (NoDelay) write of an arbitrary-precision integer.
    pub fn set_bigint_now(&self, v: &BigUint) {
        self.set_bigint(v, sys::vpiNoDelay);
    }

    /// Write a 4-state value with the supplied VPI flag.
    pub fn set_logic(&self, v: &LogicArray, flags: i32) {
        let mut words: Vec<sys::t_vpi_vecval> = v
            .to_vpi_words()
            .into_iter()
            .map(|(aval, bval)| sys::t_vpi_vecval { aval, bval })
            .collect();
        self.set_vector_words(&mut words, flags);
    }

    /// Immediate (NoDelay) write of a 4-state value.
    pub fn set_logic_now(&self, v: &LogicArray) {
        self.set_logic(v, sys::vpiNoDelay);
    }
}

/// All top-level modules in the design.
pub fn top_modules() -> Vec<HierarchyHandle> {
    backend::current()
        .top_modules()
        .into_iter()
        .map(|h| HierarchyHandle { h })
        .collect()
}

fn select_top_module(explicit_name: Option<&str>) -> Result<HierarchyHandle, HandleError> {
    if let Some(name) = explicit_name {
        let h = backend::current()
            .handle_by_name(name)
            .ok_or_else(|| HandleError::NotFound {
                name: name.to_owned(),
                scope: "<top-level>".to_owned(),
            })?;
        return AnyHandle::classify(h).as_hierarchy();
    }

    top_modules()
        .into_iter()
        .next()
        .ok_or(HandleError::NoTopModule)
}

/// The configured top-level module, or the first simulator root when no top
/// was configured.
///
/// `RUSTDV_TOP` selects an exact root by name. Without it, the first top-level
/// module returned by VPI is used.
pub fn top_module() -> Result<HierarchyHandle, HandleError> {
    let explicit_name = std::env::var("RUSTDV_TOP")
        .ok()
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty());
    select_top_module(explicit_name.as_deref())
}

// ===========================================================================
// Panic sink (invariant 4)
// ===========================================================================

type PanicSink = RefCell<Option<Box<dyn Fn(String)>>>;

thread_local! {
    static PANIC_SINK: PanicSink = const { RefCell::new(None) };
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

pub(crate) struct CbShared {
    kind: CbKind,
    /// Registration token returned by the backend. One-shots remove it from
    /// inside the trampoline, while it is valid on both supported VPI
    /// simulators: Icarus reaps the active callback after it returns and
    /// Verilator releases its separately-owned handle object immediately.
    token: Cell<backend::CbToken>,
    /// The backend that registered the callback, which removes it: another
    /// backend may have been installed since.
    backend: &'static dyn backend::Backend,
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
            self.shared.backend.remove_cb(self.shared.token.get());
            // Reclaim the C-side reference.
            unsafe { drop(Rc::from_raw(self.raw)) };
        }
    }
}

/// Run the callback behind `ud`, the user data a backend was given at
/// registration. The trampoline body shared by every backend.
///
/// # Safety
/// `ud` must be null or a pointer from [`register`] whose C-side reference
/// has not been reclaimed yet.
pub(crate) unsafe fn dispatch(ud: *const CbShared) {
    if ud.is_null() {
        return;
    }
    unsafe {
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
                    shared.backend.remove_cb(shared.token.get());
                    // Reclaim the C-side reference before running user code.
                    drop(Rc::from_raw(ud));
                    if let Some(f) = f
                        && let Err(p) = catch_unwind(AssertUnwindSafe(f))
                    {
                        report_panic(p);
                    }
                }
            }
            CbKind::Recurring => {
                let mut guard = shared.repeat.borrow_mut();
                if let Some(f) = guard.as_mut()
                    && let Err(p) = catch_unwind(AssertUnwindSafe(f))
                {
                    report_panic(p);
                }
            }
        }
        drop(shared);
    }
}

fn register(
    kind: CbKind,
    once: Option<Box<dyn FnOnce()>>,
    repeat: Option<Box<dyn FnMut()>>,
    reason: Reason,
) -> CallbackHandle {
    let backend = backend::current();
    let shared = Rc::new(CbShared {
        kind,
        token: Cell::new(0),
        backend,
        released: Cell::new(false),
        once: RefCell::new(once),
        repeat: RefCell::new(repeat),
    });
    // C-side reference:
    let raw = Rc::into_raw(shared.clone());
    shared.token.set(backend.register_cb(reason, CbData(raw)));
    CallbackHandle {
        shared,
        raw,
        detached: false,
    }
}

/// One-shot callback after `steps` precision units (cbAfterDelay).
pub fn register_timer(steps: u64, f: Box<dyn FnOnce()>) -> CallbackHandle {
    register(CbKind::OneShot, Some(f), None, Reason::AfterDelay(steps))
}

/// Recurring callback on any value change of `sig` (cbValueChange). The
/// closure reads the signal itself; edge filtering happens in rustdv-sim.
pub fn register_value_change(sig: LogicHandle, f: Box<dyn FnMut()>) -> CallbackHandle {
    register(CbKind::Recurring, None, Some(f), Reason::ValueChange(sig.h))
}

/// One-shot callback at the next ReadWrite synch point.
pub fn register_read_write(f: Box<dyn FnOnce()>) -> CallbackHandle {
    register(CbKind::OneShot, Some(f), None, Reason::ReadWrite)
}

/// One-shot callback at the next ReadOnly synch point.
pub fn register_read_only(f: Box<dyn FnOnce()>) -> CallbackHandle {
    register(CbKind::OneShot, Some(f), None, Reason::ReadOnly)
}

/// One-shot callback at the next simulation time step.
pub fn register_next_sim_time(f: Box<dyn FnOnce()>) -> CallbackHandle {
    register(CbKind::OneShot, Some(f), None, Reason::NextSimTime)
}

/// One-shot callback at the end of the current simulation time step.
pub fn register_at_end_of_sim_time(f: Box<dyn FnOnce()>) -> CallbackHandle {
    register(CbKind::OneShot, Some(f), None, Reason::AtEndOfSimTime)
}

/// One-shot callback at start of simulation (the bootstrap hook).
pub fn register_start_of_simulation(f: Box<dyn FnOnce()>) -> CallbackHandle {
    register(CbKind::OneShot, Some(f), None, Reason::StartOfSimulation)
}

/// One-shot callback at end of simulation.
pub fn register_end_of_simulation(f: Box<dyn FnOnce()>) -> CallbackHandle {
    register(CbKind::OneShot, Some(f), None, Reason::EndOfSimulation)
}

#[cfg(test)]
mod vpi_word_tests {
    use super::*;

    fn assert_single_word<T: ToVpiWords<1>>(value: T, expected: u32) {
        let words = value.to_vpi_words();
        assert_eq!(words[0].aval, expected);
        assert_eq!(words[0].bval, 0);
    }

    #[test]
    fn scalar_values_convert_to_one_vpi_word() {
        assert_single_word(false, 0);
        assert_single_word(true, 1);
        assert_single_word(0xa5u8, 0xa5);
        assert_single_word(0xa5b6u16, 0xa5b6);
        assert_single_word(0xa5b6_c7d8u32, 0xa5b6_c7d8);
    }

    #[test]
    fn wide_values_convert_in_least_significant_word_first_order() {
        let words = 0x0123_4567_89ab_cdefu64.to_vpi_words();
        assert_eq!(words.map(|word| word.aval), [0x89ab_cdef, 0x0123_4567]);
        assert!(words.iter().all(|word| word.bval == 0));

        let words = 0x0123_4567_89ab_cdef_fedc_ba98_7654_3210u128.to_vpi_words();
        assert_eq!(
            words.map(|word| word.aval),
            [0x7654_3210, 0xfedc_ba98, 0x89ab_cdef, 0x0123_4567]
        );
        assert!(words.iter().all(|word| word.bval == 0));
    }
}

#[cfg(all(test, feature = "vpi"))]
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

#[cfg(all(test, feature = "vpi"))]
mod handle_tests {
    use super::*;

    #[test]
    fn sv_integral_variables_support_vector_access() {
        for object_type in [
            sys::vpiLongIntVar,
            sys::vpiShortIntVar,
            sys::vpiIntVar,
            sys::vpiByteVar,
            sys::vpiEnumVar,
            sys::vpiBitVar,
        ] {
            rustdv_vpi_stubs::configure_signal(
                64,
                &[
                    sys::t_vpi_vecval {
                        aval: 0x89ab_cdef,
                        bval: 0,
                    },
                    sys::t_vpi_vecval {
                        aval: 0x1234_5678,
                        bval: 0,
                    },
                ],
                "0",
            );
            rustdv_vpi_stubs::configure_signal_type(object_type);
            let signal = AnyHandle::classify(unsafe { ObjHandle::from_raw(1) })
                .as_logic()
                .unwrap();
            assert_eq!(signal.size(), 64);
            assert_eq!(signal.get_u64(), Ok(0x1234_5678_89ab_cdef));
            signal.set_u64_now(0xfedc_ba98_7654_3210);
            let words = rustdv_vpi_stubs::last_put_vector();
            assert_eq!(words[0].aval, 0x7654_3210);
            assert_eq!(words[1].aval, 0xfedc_ba98);
        }
        rustdv_vpi_stubs::configure_signal_type(sys::vpiNet);
    }

    #[test]
    fn non_logic_variables_have_typed_handles() {
        for object_type in [
            sys::vpiRealVar,
            sys::vpiStringVar,
            sys::vpiStructVar,
            sys::vpiUnionVar,
        ] {
            rustdv_vpi_stubs::configure_signal_type(object_type);
            assert!(matches!(
                AnyHandle::classify(unsafe { ObjHandle::from_raw(1) }),
                AnyHandle::Real(_)
                    | AnyHandle::String(_)
                    | AnyHandle::Struct(_)
                    | AnyHandle::Union(_)
            ));
        }
        rustdv_vpi_stubs::configure_signal_type(sys::vpiNet);
    }

    #[test]
    fn top_selection_prefers_explicit_name_and_otherwise_uses_first_root() {
        rustdv_vpi_stubs::configure_top_modules(&["first", "dut"]);
        assert_eq!(select_top_module(Some("dut")).unwrap().name(), "dut");
        assert_eq!(select_top_module(None).unwrap().name(), "first");
        assert!(matches!(
            select_top_module(Some("missing")),
            Err(HandleError::NotFound { .. })
        ));
        rustdv_vpi_stubs::configure_top_modules(&[]);
        assert!(matches!(
            select_top_module(None),
            Err(HandleError::NoTopModule)
        ));
    }

    fn make_signal(width: i32, words: &[sys::t_vpi_vecval], binstr: &str) -> LogicHandle {
        rustdv_vpi_stubs::configure_signal(width, words, binstr);
        let AnyHandle::Logic(signal) = AnyHandle::classify(unsafe { ObjHandle::from_raw(1) })
        else {
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
                aval: 0x5e,
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
    fn logic_array_read_uses_vpi_vector_words() {
        let signal = make_signal(
            4,
            &[sys::t_vpi_vecval {
                aval: 0b0110,
                bval: 0b0011,
            }],
            "0000",
        );

        assert_eq!(signal.get_logic().unwrap().to_binstr(), "01xz");
    }

    #[test]
    fn vector_read_returns_numeric_zero_from_a_vector_word() {
        let signal = make_signal(8, &[sys::t_vpi_vecval { aval: 0, bval: 0 }], "00000000");

        assert_eq!(signal.get_u64(), Ok(0));
    }

    #[test]
    fn bool_read_converts_single_bit_values() {
        let low = make_signal(1, &[sys::t_vpi_vecval { aval: 0, bval: 0 }], "0");
        assert_eq!(low.get_bool(), Ok(false));

        let high = make_signal(1, &[sys::t_vpi_vecval { aval: 1, bval: 0 }], "1");
        assert_eq!(high.get_bool(), Ok(true));
    }

    #[test]
    fn bool_read_rejects_wide_and_four_state_values() {
        let wide = make_signal(2, &[sys::t_vpi_vecval { aval: 1, bval: 0 }], "01");
        assert_eq!(wide.get_bool(), Err(ValueError::Width { want: 1, have: 2 }));

        let unknown = make_signal(1, &[sys::t_vpi_vecval { aval: 1, bval: 1 }], "x");
        assert_eq!(
            unknown.get_bool(),
            Err(ValueError::FourState("x".to_owned()))
        );
    }

    #[test]
    fn vector_read_reports_an_unavailable_vpi_result() {
        let signal = make_signal(8, &[sys::t_vpi_vecval { aval: 0, bval: 0 }], "00000000");
        rustdv_vpi_stubs::make_vector_value_unavailable();

        assert_eq!(signal.get_u64(), Err(ValueError::Unavailable));
        assert_eq!(signal.get_bigint(), Err(ValueError::Unavailable));
        assert_eq!(signal.get_logic(), Err(ValueError::Unavailable));
    }

    #[test]
    fn bigint_read_supports_values_wider_than_u64() {
        let signal = make_signal(
            70,
            &[
                sys::t_vpi_vecval {
                    aval: 0x89ab_cdef,
                    bval: 0,
                },
                sys::t_vpi_vecval {
                    aval: 0x0123_4567,
                    bval: 0,
                },
                sys::t_vpi_vecval {
                    aval: 0xffff_ffea,
                    bval: 0xffff_ffc0,
                },
            ],
            "1010100000000100100011010001010110011110001001101010111100110111101111",
        );

        assert_eq!(
            signal.get_bigint().map(|value| value.to_str_radix(16)),
            Ok("2a0123456789abcdef".to_owned())
        );
    }

    #[test]
    fn bigint_read_reports_xz_in_used_bits() {
        let signal = make_signal(
            65,
            &[
                sys::t_vpi_vecval::default(),
                sys::t_vpi_vecval::default(),
                sys::t_vpi_vecval { aval: 1, bval: 1 },
            ],
            &format!("x{}", "0".repeat(64)),
        );

        assert_eq!(
            signal.get_bigint(),
            Err(ValueError::FourState(format!("x{}", "0".repeat(64))))
        );
    }

    #[test]
    fn vector_write_truncates_and_zero_extends_to_signal_width() {
        let narrow = make_signal(8, &[sys::t_vpi_vecval::default()], "00000000");
        narrow.set_u64_now(0xffff_ffff_ffff_ffa5);
        let words = rustdv_vpi_stubs::last_put_vector();
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].aval, 0xa5);
        assert_eq!(words[0].bval, 0);

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

    #[test]
    fn typed_integer_setters_write_vpi_words() {
        let signal = make_signal(1, &[sys::t_vpi_vecval::default()], "0");
        signal.set_bool_now(true);
        assert_eq!(rustdv_vpi_stubs::last_put_vector()[0].aval, 1);

        let signal = make_signal(8, &[sys::t_vpi_vecval::default()], "00000000");
        signal.set_u8(0xa5, sys::vpiNoDelay);
        assert_eq!(rustdv_vpi_stubs::last_put_vector()[0].aval, 0xa5);

        let signal = make_signal(16, &[sys::t_vpi_vecval::default()], &"0".repeat(16));
        signal.set_u16_now(0xa5b6);
        assert_eq!(rustdv_vpi_stubs::last_put_vector()[0].aval, 0xa5b6);

        let signal = make_signal(32, &[sys::t_vpi_vecval::default()], &"0".repeat(32));
        signal.set_u32_now(0xa5b6_c7d8);
        assert_eq!(rustdv_vpi_stubs::last_put_vector()[0].aval, 0xa5b6_c7d8);

        let signal = make_signal(128, &[sys::t_vpi_vecval::default(); 4], &"0".repeat(128));
        signal.set_u128_now(0x0123_4567_89ab_cdef_fedc_ba98_7654_3210);
        let words = rustdv_vpi_stubs::last_put_vector();
        assert_eq!(
            words.iter().map(|word| word.aval).collect::<Vec<_>>(),
            [0x7654_3210, 0xfedc_ba98, 0x89ab_cdef, 0x0123_4567]
        );
        assert!(words.iter().all(|word| word.bval == 0));
    }

    #[test]
    fn bigint_setter_writes_and_zero_extends_vpi_words() {
        let signal = make_signal(192, &[sys::t_vpi_vecval::default(); 6], &"0".repeat(192));
        let value = BigUint::from_slice(&[
            0x7654_3210,
            0xfedc_ba98,
            0x89ab_cdef,
            0x0123_4567,
            0xa5a5_5a5a,
        ]);

        signal.set_bigint_now(&value);

        let words = rustdv_vpi_stubs::last_put_vector();
        assert_eq!(
            words.iter().map(|word| word.aval).collect::<Vec<_>>(),
            [
                0x7654_3210,
                0xfedc_ba98,
                0x89ab_cdef,
                0x0123_4567,
                0xa5a5_5a5a,
                0,
            ]
        );
        assert!(words.iter().all(|word| word.bval == 0));
    }

    #[test]
    fn vector_word_mask_clears_unused_aval_and_bval_bits() {
        let signal = make_signal(37, &[sys::t_vpi_vecval::default(); 2], &"0".repeat(37));
        let mut words = [
            sys::t_vpi_vecval {
                aval: u32::MAX,
                bval: u32::MAX,
            },
            sys::t_vpi_vecval {
                aval: u32::MAX,
                bval: u32::MAX,
            },
        ];

        signal.mask_vector_words(&mut words);

        assert_eq!(words[0].aval, u32::MAX);
        assert_eq!(words[0].bval, u32::MAX);
        assert_eq!(words[1].aval, 0x1f);
        assert_eq!(words[1].bval, 0x1f);
    }

    #[test]
    fn logic_array_write_uses_vpi_vector_words() {
        let signal = make_signal(
            4,
            &[sys::t_vpi_vecval::default()],
            "binary-string path must not be used",
        );
        signal.set_logic_now(&LogicArray::from_binstr("01xz"));

        let words = rustdv_vpi_stubs::last_put_vector();
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].aval, 0b0110);
        assert_eq!(words[0].bval, 0b0011);
    }
}
