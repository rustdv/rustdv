//! The simulator backend: what rustdv needs from a simulator.
//!
//! Every simulator access of this crate goes through the [`Backend`]
//! installed on the current thread. The VPI backend (the `vpi` feature, on
//! by default) is used when none is installed, so a testbench loaded into a
//! VPI simulator needs no setup. Another host, such as a simulator embedding
//! rustdv through its own extension interface, implements [`Backend`] and
//! [`install`]s it before the simulation starts.
//!
//! The interface keeps VPI's shape — vector values are VPI `(aval, bval)`
//! words, write flags are VPI delay modes and callbacks wait for VPI-style
//! [`Reason`]s — because rustdv's semantics are defined in VPI terms; a
//! backend maps them onto its own simulator.

use std::borrow::Cow;
use std::cell::Cell;
use std::ffi::c_void;
use std::marker::PhantomData;

use rustdv_gpi_sys as sys;

use crate::{CbShared, ValueError};

/// Opaque simulator object handle: an ID chosen by the backend. Valid for
/// the whole simulation, hence `Copy`. `0` is reserved for "no object".
/// `!Send` like every handle of this crate: objects belong to the simulator
/// thread.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct ObjHandle {
    raw: usize,
    _thread: PhantomData<*const ()>,
}

impl ObjHandle {
    /// A handle with the backend-chosen ID `raw`, which must not be `0`.
    ///
    /// # Safety
    /// `raw` must be an ID issued by the backend installed on this thread for
    /// an object of the current simulation. Backends may trust their IDs
    /// without checking them: the VPI backend passes them to the simulator
    /// as `vpiHandle`s.
    pub const unsafe fn from_raw(raw: usize) -> Self {
        assert!(raw != 0, "object handle 0 is reserved for no object");
        ObjHandle {
            raw,
            _thread: PhantomData,
        }
    }

    /// The backend-chosen ID.
    pub const fn as_raw(self) -> usize {
        self.raw
    }

    pub(crate) const fn null() -> Self {
        ObjHandle {
            raw: 0,
            _thread: PhantomData,
        }
    }
}

impl std::fmt::Debug for ObjHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ObjHandle({:#x})", self.raw)
    }
}

/// What kind of object a handle refers to.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ObjKind {
    /// A module or other scope.
    Hierarchy,
    /// A value-bearing signal of the given width in bits.
    Logic {
        width: u32,
    },
    Real,
    String,
    Struct,
    Union,
    Other,
}

/// What a callback waits for, in VPI terms.
#[derive(Copy, Clone, Debug)]
pub enum Reason {
    /// After the given number of time precision steps (`cbAfterDelay`).
    AfterDelay(u64),
    /// Any value change of the signal (`cbValueChange`). Recurring.
    ValueChange(ObjHandle),
    /// The next ReadWrite synchronization point (`cbReadWriteSynch`).
    ReadWrite,
    /// The next ReadOnly synchronization point (`cbReadOnlySynch`).
    ReadOnly,
    /// The start of the next time step (`cbNextSimTime`).
    NextSimTime,
    /// The end of the current time step (`cbAtEndOfSimTime`).
    AtEndOfSimTime,
    StartOfSimulation,
    EndOfSimulation,
}

/// Token a backend returns from [`Backend::register_cb`] to identify the
/// registration in [`Backend::remove_cb`]. `0` means "not registered".
pub type CbToken = usize;

/// The user data of a registered callback. The backend hands it back to
/// [`CbData::dispatch`] each time the callback's [`Reason`] occurs.
#[derive(Copy, Clone)]
pub struct CbData(pub(crate) *const CbShared);

impl CbData {
    /// As a C `user_data` pointer, for backends passing it through C.
    pub fn as_ptr(self) -> *mut c_void {
        self.0 as *mut c_void
    }

    /// Back from [`Self::as_ptr`].
    ///
    /// # Safety
    /// `ptr` must come from [`Self::as_ptr`].
    pub unsafe fn from_ptr(ptr: *mut c_void) -> Self {
        CbData(ptr as *const CbShared)
    }

    /// Run the callback.
    ///
    /// # Safety
    /// The registration this data was given with must still be live: not
    /// yet removed through [`Backend::remove_cb`]. A one-shot callback
    /// removes itself while it runs, so it is dispatched at most once.
    pub unsafe fn dispatch(self) {
        unsafe { crate::dispatch(self.0) }
    }
}

/// A simulator as rustdv sees it. All methods are called on the simulator
/// thread.
pub trait Backend {
    /// All top-level scopes of the design.
    fn top_modules(&self) -> Vec<ObjHandle>;

    /// An object by its full hierarchical name.
    fn handle_by_name(&self, name: &str) -> Option<ObjHandle>;

    fn kind(&self, h: ObjHandle) -> ObjKind;

    fn name(&self, h: ObjHandle) -> String;

    fn full_name(&self, h: ObjHandle) -> String;

    /// The simulator's description of the object's type, for diagnostics.
    fn type_desc(&self, h: ObjHandle) -> String;

    /// A child of a scope by relative name.
    fn child(&self, scope: ObjHandle, name: &str) -> Option<ObjHandle>;

    /// The child scopes and signals of a scope.
    fn children(&self, scope: ObjHandle) -> Vec<ObjHandle>;

    /// A member of an unpacked struct or union by name.
    fn member(&self, _aggregate: ObjHandle, _name: &str) -> Option<ObjHandle> {
        None
    }

    /// The members of an unpacked struct or union.
    fn members(&self, _aggregate: ObjHandle) -> Vec<ObjHandle> {
        Vec::new()
    }

    fn get_real(&self, _h: ObjHandle) -> Result<f64, ValueError> {
        Err(ValueError::Unavailable)
    }

    fn put_real(&self, h: ObjHandle, _value: f64) {
        panic!(
            "cannot write '{}': real values are not supported",
            self.full_name(h)
        );
    }

    fn get_string(&self, _h: ObjHandle) -> Result<String, ValueError> {
        Err(ValueError::Unavailable)
    }

    fn put_string(&self, h: ObjHandle, _value: &str) -> Result<(), ValueError> {
        panic!(
            "cannot write '{}': string values are not supported",
            self.full_name(h)
        );
    }

    /// Current value of a signal as `width.div_ceil(32)` VPI vector words.
    /// A borrowed slice must stay valid until the next backend call.
    ///
    /// rustdv calls it with the signal's own width, and copies a borrowed
    /// slice before calling the backend again; a backend may rely on both.
    fn get_vector(
        &self,
        h: ObjHandle,
        width: u32,
    ) -> Result<Cow<'_, [sys::t_vpi_vecval]>, ValueError>;

    /// Write VPI vector words, already sized and masked to the signal width,
    /// with a VPI delay mode (`vpiNoDelay`, `vpiInertialDelay`, ...).
    ///
    /// rustdv passes at least `width.div_ceil(32)` words for the signal's
    /// width; a backend may rely on it.
    fn put_vector(&self, h: ObjHandle, words: &mut [sys::t_vpi_vecval], flags: i32);

    /// Current simulation time in time precision steps.
    fn sim_time_steps(&self) -> u64;

    /// Time precision as a power of ten (e.g. -9 = 1 ns).
    fn time_precision(&self) -> i32;

    /// End the simulation.
    fn finish(&self);

    /// Record that the regression failed, for a host with a pass/fail
    /// verdict of its own. The runner's `REGRESSION:` line carries it too.
    fn report_failure(&self, _msg: &str) {}

    /// Write one line of testbench output.
    fn console(&self, line: &str) {
        println!("{line}");
    }

    /// Register a callback for `reason`; the backend calls
    /// [`CbData::dispatch`] when it occurs, until the registration is
    /// removed. Returns a non-zero token.
    fn register_cb(&self, reason: Reason, data: CbData) -> CbToken;

    /// Remove a registration. Called exactly once per registration, with a
    /// token this backend returned, possibly from inside its own dispatch.
    /// It is called on the backend that registered the callback, even if
    /// another backend has been installed since.
    fn remove_cb(&self, token: CbToken);
}

thread_local! {
    static BACKEND: Cell<Option<&'static dyn Backend>> = const { Cell::new(None) };
}

/// Install the backend for the current thread, replacing any previous one.
pub fn install(backend: &'static dyn Backend) {
    BACKEND.with(|b| b.set(Some(backend)));
}

/// Remove the backend installed on the current thread.
pub fn uninstall() {
    BACKEND.with(|b| b.set(None));
}

/// The backend of the current thread.
pub(crate) fn current() -> &'static dyn Backend {
    if let Some(backend) = BACKEND.with(Cell::get) {
        return backend;
    }
    #[cfg(feature = "vpi")]
    {
        &crate::vpi::VpiBackend
    }
    #[cfg(not(feature = "vpi"))]
    {
        panic!("no rustdv backend is installed on this thread")
    }
}
