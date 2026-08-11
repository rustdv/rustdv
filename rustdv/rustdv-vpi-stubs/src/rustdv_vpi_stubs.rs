//! Stub definitions of the VPI entry points for **test executables**.
//!
//! Unit-test binaries (design-doc §7.3 convention 5: pure-Rust tests, no
//! simulator) link the whole crate graph, including `rustdv-gpi-sys`'s
//! `extern "C"` declarations. Executables, unlike cdylibs, cannot carry
//! undefined symbols — so tests pull these panicking definitions in as a
//! dev-dependency:
//!
//! ```ignore
//! #[cfg(test)]
//! use rustdv_vpi_stubs as _;
//! ```
//!
//! The `.vpi` module never links this crate; there the real symbols come
//! from the simulator process (vvp) at load time.
//!
//! Most entry points panic when called. Callback registration is the one
//! exception: its small test double models a simulator that removes a fired
//! one-shot from its schedule while retaining the returned handle. That is
//! Verilator's ownership contract and lets `rustdv-gpi` test callback cleanup
//! without loading a simulator. Variadics remain deliberately simplified.

#![allow(non_snake_case, clippy::missing_safety_doc)]

use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::raw::c_void;

use rustdv_gpi_sys::{
    t_cb_data, t_vpi_value, t_vpi_vecval, vpiBinStrVal, vpiHandle, vpiNet, vpiNoDelay,
    vpiSize, vpiType, vpiVectorVal,
};

struct StubCallback {
    data: t_cb_data,
    scheduled: bool,
    handle_live: bool,
}

struct StubSignal {
    width: i32,
    words: Vec<t_vpi_vecval>,
    binstr: CString,
    last_put: Vec<t_vpi_vecval>,
}

impl Default for StubSignal {
    fn default() -> Self {
        Self {
            width: 37,
            words: vec![t_vpi_vecval::default(); 2],
            binstr: CString::new("0".repeat(37)).expect("static binary string contains no NUL"),
            last_put: Vec::new(),
        }
    }
}

thread_local! {
    static CALLBACKS: RefCell<Vec<*mut StubCallback>> = const { RefCell::new(Vec::new()) };
    static PROPERTY_GETS: RefCell<Vec<i32>> = const { RefCell::new(Vec::new()) };
    static SIGNAL: RefCell<StubSignal> = RefCell::new(StubSignal::default());
}

/// Configure the signal value returned by the unit-test VPI double.
pub fn configure_signal(width: i32, words: &[t_vpi_vecval], binstr: &str) {
    assert!(width > 0, "stub signal width must be positive");
    let word_count = (width as usize).div_ceil(32);
    assert!(
        words.len() >= word_count,
        "stub signal needs {word_count} vector word(s), got {}",
        words.len()
    );
    SIGNAL.with(|signal| {
        *signal.borrow_mut() = StubSignal {
            width,
            words: words[..word_count].to_vec(),
            binstr: CString::new(binstr).expect("stub binary string contains NUL"),
            last_put: Vec::new(),
        };
    });
}

/// Return the vector words most recently written by `vpi_put_value`.
pub fn last_put_vector() -> Vec<t_vpi_vecval> {
    SIGNAL.with(|signal| signal.borrow().last_put.clone())
}

/// Clear property-query counters for the calling test thread.
pub fn reset_property_gets() {
    PROPERTY_GETS.with(|gets| gets.borrow_mut().clear());
}

/// Number of times a VPI property was queried on the calling test thread.
pub fn property_get_count(property: i32) -> usize {
    PROPERTY_GETS.with(|gets| {
        gets.borrow()
            .iter()
            .filter(|queried| **queried == property)
            .count()
    })
}

/// Remove all callback test-double state for the calling test thread.
pub fn reset_callbacks() {
    CALLBACKS.with(|callbacks| {
        let mut callbacks = callbacks.borrow_mut();
        let live = callbacks
            .iter()
            .filter(|callback| unsafe { (***callback).handle_live })
            .count();
        assert_eq!(
            live, 0,
            "cannot reset callback test state while {live} handle(s) are live"
        );
        for callback in callbacks.drain(..) {
            unsafe {
                drop(Box::from_raw(callback));
            }
        }
    });
}

/// Fire the next scheduled callback, retaining its returned VPI handle.
pub fn fire_next_callback() {
    CALLBACKS.with(|callbacks| {
        let callbacks = callbacks.borrow();
        let callback = callbacks
            .iter()
            .copied()
            .find(|callback| unsafe { (**callback).scheduled })
            .expect("no scheduled callback to fire");
        unsafe {
            (*callback).scheduled = false;
            let mut data = t_cb_data {
                reason: (*callback).data.reason,
                cb_rtn: (*callback).data.cb_rtn,
                obj: (*callback).data.obj,
                time: (*callback).data.time,
                value: (*callback).data.value,
                index: (*callback).data.index,
                user_data: (*callback).data.user_data,
            };
            if let Some(callback_fn) = data.cb_rtn {
                callback_fn(&mut data);
            }
        }
    });
}

/// Number of callback handles not yet released on the calling test thread.
pub fn live_callback_handles() -> usize {
    CALLBACKS.with(|callbacks| {
        callbacks
            .borrow()
            .iter()
            .filter(|callback| unsafe { (***callback).handle_live })
            .count()
    })
}

macro_rules! stub {
    ($($name:ident ( $($arg:ident : $ty:ty),* ) -> $ret:ty;)*) => {
        $(
            #[no_mangle]
            pub extern "C" fn $name($(_: $ty),*) -> $ret {
                panic!(concat!(
                    stringify!($name),
                    " called outside a simulator (rustdv-vpi-stubs is for unit tests only)"
                ));
            }
        )*
    };
}

stub! {
    vpi_handle_by_name(a: *const i8, b: *mut c_void) -> *mut c_void;
    vpi_handle_by_index(a: *mut c_void, b: i32) -> *mut c_void;
    vpi_iterate(a: i32, b: *mut c_void) -> *mut c_void;
    vpi_scan(a: *mut c_void) -> *mut c_void;
    vpi_get_str(a: i32, b: *mut c_void) -> *mut i8;
    vpi_get_time(a: *mut c_void, b: *mut c_void) -> ();
    vpi_free_object(a: *mut c_void) -> i32;
    vpi_control(a: i32) -> i32;
    vpi_printf(a: *const i8) -> i32;
}

#[no_mangle]
pub extern "C" fn vpi_get(property: i32, _handle: *mut c_void) -> i32 {
    PROPERTY_GETS.with(|gets| gets.borrow_mut().push(property));
    if property == vpiType {
        vpiNet
    } else if property == vpiSize {
        SIGNAL.with(|signal| signal.borrow().width)
    } else {
        panic!("unsupported vpi_get property {property} in test stub")
    }
}

#[no_mangle]
pub unsafe extern "C" fn vpi_get_value(_handle: vpiHandle, value: *mut t_vpi_value) {
    assert!(!value.is_null(), "vpi_get_value called with null value");
    SIGNAL.with(|signal| {
        let signal = signal.borrow();
        let value = unsafe { &mut *value };
        match value.format {
            format if format == vpiVectorVal => {
                value.value.vector = signal.words.as_ptr().cast_mut();
            }
            format if format == vpiBinStrVal => {
                value.value.str_ = signal.binstr.as_ptr().cast_mut();
            }
            format => panic!("unsupported vpi_get_value format {format} in test stub"),
        }
    });
}

#[no_mangle]
pub unsafe extern "C" fn vpi_put_value(
    handle: vpiHandle,
    value: *mut t_vpi_value,
    _time: *mut rustdv_gpi_sys::t_vpi_time,
    flags: i32,
) -> vpiHandle {
    assert!(!value.is_null(), "vpi_put_value called with null value");
    assert_eq!(flags, vpiNoDelay, "test stub only supports immediate writes");
    SIGNAL.with(|signal| {
        let mut signal = signal.borrow_mut();
        let value = unsafe { &mut *value };
        match value.format {
            format if format == vpiVectorVal => {
                let word_count = (signal.width as usize).div_ceil(32);
                let words = unsafe { std::slice::from_raw_parts(value.value.vector, word_count) };
                signal.last_put = words.to_vec();
            }
            format if format == vpiBinStrVal => {
                let text = unsafe { CStr::from_ptr(value.value.str_) };
                signal.binstr = CString::new(text.to_bytes())
                    .expect("written stub binary string contains NUL");
            }
            format => panic!("unsupported vpi_put_value format {format} in test stub"),
        }
    });
    handle
}

#[no_mangle]
pub unsafe extern "C" fn vpi_register_cb(data: *mut t_cb_data) -> vpiHandle {
    assert!(!data.is_null(), "vpi_register_cb called with null data");
    let callback = Box::new(StubCallback {
        data: unsafe {
            t_cb_data {
                reason: (*data).reason,
                cb_rtn: (*data).cb_rtn,
                obj: (*data).obj,
                time: (*data).time,
                value: (*data).value,
                index: (*data).index,
                user_data: (*data).user_data,
            }
        },
        scheduled: true,
        handle_live: true,
    });
    let callback = Box::into_raw(callback);
    CALLBACKS.with(|callbacks| callbacks.borrow_mut().push(callback));
    callback.cast::<c_void>()
}

#[no_mangle]
pub unsafe extern "C" fn vpi_remove_cb(handle: vpiHandle) -> i32 {
    assert!(!handle.is_null(), "vpi_remove_cb called with null handle");
    let callback = handle.cast::<StubCallback>();
    unsafe {
        if !(*callback).handle_live {
            return 0;
        }
        (*callback).scheduled = false;
        (*callback).handle_live = false;
    }
    1
}
