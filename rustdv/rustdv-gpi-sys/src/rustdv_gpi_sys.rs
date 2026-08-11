//! # rustdv-gpi-sys
//!
//! Raw FFI declarations for the simulator programming interface.
//!
//! **Deviation from design-doc D2.1** (documented in STATUS.md): the design
//! calls for bindgen-generated bindings to cocotb's `gpi.h`. This sandbox has
//! no cocotb GPI build and a zero-external-dependency constraint (no bindgen),
//! so v0 binds the IEEE 1800 **VPI** C API directly — the same API cocotb's
//! GPI wraps for Icarus. The declarations below are a hand-written subset of
//! `vpi_user.h`, sufficient for: hierarchy lookup, value get/set, time query,
//! and callbacks. The safe layer (`rustdv-gpi`) exposes a GPI-shaped API so
//! that swapping this crate for real gpi.h bindings later is contained.
//!
//! Everything here is `unsafe extern "C"`; no logic lives in this crate
//! (the `-sys` discipline, design-doc D2.1).

#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]

use std::os::raw::{c_char, c_void};

pub type PLI_INT32 = i32;
pub type PLI_UINT32 = u32;
pub type PLI_BYTE8 = c_char;

/// Opaque simulator object handle (`vpiHandle`).
pub type vpiHandle = *mut c_void;

// ---------------------------------------------------------------------------
// Properties (vpi_get / vpi_get_str)
// ---------------------------------------------------------------------------
pub const vpiType: PLI_INT32 = 1;
pub const vpiName: PLI_INT32 = 2;
pub const vpiFullName: PLI_INT32 = 3;
pub const vpiSize: PLI_INT32 = 4;
pub const vpiFile: PLI_INT32 = 5;
pub const vpiLineNo: PLI_INT32 = 6;
pub const vpiTimeUnit: PLI_INT32 = 11;
pub const vpiTimePrecision: PLI_INT32 = 12;

// ---------------------------------------------------------------------------
// Object type codes (vpi_get(vpiType, ...) results / vpi_iterate types)
// ---------------------------------------------------------------------------
pub const vpiConstant: PLI_INT32 = 7;
pub const vpiIntegerVar: PLI_INT32 = 25;
pub const vpiMemory: PLI_INT32 = 29;
pub const vpiModule: PLI_INT32 = 32;
pub const vpiNet: PLI_INT32 = 36;
pub const vpiParameter: PLI_INT32 = 41;
pub const vpiPort: PLI_INT32 = 44;
pub const vpiReg: PLI_INT32 = 48;
pub const vpiRealVar: PLI_INT32 = 47;
// SystemVerilog variable types (sv_vpi_user.h)
pub const vpiLongIntVar: PLI_INT32 = 610;
pub const vpiShortIntVar: PLI_INT32 = 611;
pub const vpiIntVar: PLI_INT32 = 612;
pub const vpiByteVar: PLI_INT32 = 614;
pub const vpiEnumVar: PLI_INT32 = 617;
pub const vpiBitVar: PLI_INT32 = 620;

// ---------------------------------------------------------------------------
// Value formats (t_vpi_value.format)
// ---------------------------------------------------------------------------
pub const vpiBinStrVal: PLI_INT32 = 1;
pub const vpiOctStrVal: PLI_INT32 = 2;
pub const vpiDecStrVal: PLI_INT32 = 3;
pub const vpiHexStrVal: PLI_INT32 = 4;
pub const vpiScalarVal: PLI_INT32 = 5;
pub const vpiIntVal: PLI_INT32 = 6;
pub const vpiRealVal: PLI_INT32 = 7;
pub const vpiStringVal: PLI_INT32 = 8;
pub const vpiVectorVal: PLI_INT32 = 9;
pub const vpiSuppressVal: PLI_INT32 = 10;

// Scalar values
pub const vpi0: PLI_INT32 = 0;
pub const vpi1: PLI_INT32 = 1;
pub const vpiZ: PLI_INT32 = 2;
pub const vpiX: PLI_INT32 = 3;

// ---------------------------------------------------------------------------
// Time types (t_vpi_time.type)
// ---------------------------------------------------------------------------
pub const vpiScaledRealTime: PLI_INT32 = 1;
pub const vpiSimTime: PLI_INT32 = 2;
pub const vpiSuppressTime: PLI_INT32 = 3;

// ---------------------------------------------------------------------------
// vpi_put_value flags
// ---------------------------------------------------------------------------
pub const vpiNoDelay: PLI_INT32 = 1;
pub const vpiInertialDelay: PLI_INT32 = 2;
pub const vpiForceFlag: PLI_INT32 = 5;
pub const vpiReleaseFlag: PLI_INT32 = 6;

// ---------------------------------------------------------------------------
// vpi_control operations
// ---------------------------------------------------------------------------
pub const vpiStop: PLI_INT32 = 66;
pub const vpiFinish: PLI_INT32 = 67;
pub const vpiReset: PLI_INT32 = 68;

// ---------------------------------------------------------------------------
// Callback reasons (t_cb_data.reason)
// ---------------------------------------------------------------------------
pub const cbValueChange: PLI_INT32 = 1;
pub const cbStmt: PLI_INT32 = 2;
pub const cbForce: PLI_INT32 = 3;
pub const cbRelease: PLI_INT32 = 4;
pub const cbAtStartOfSimTime: PLI_INT32 = 5;
pub const cbReadWriteSynch: PLI_INT32 = 6;
pub const cbReadOnlySynch: PLI_INT32 = 7;
pub const cbNextSimTime: PLI_INT32 = 8;
pub const cbAfterDelay: PLI_INT32 = 9;
pub const cbEndOfCompile: PLI_INT32 = 10;
pub const cbStartOfSimulation: PLI_INT32 = 11;
pub const cbEndOfSimulation: PLI_INT32 = 12;
pub const cbAtEndOfSimTime: PLI_INT32 = 31;

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct t_vpi_time {
    /// One of vpiScaledRealTime / vpiSimTime / vpiSuppressTime.
    /// (Named `type` in vpi_user.h; `type` is reserved in Rust.)
    pub type_: PLI_INT32,
    pub high: PLI_UINT32,
    pub low: PLI_UINT32,
    pub real: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union u_vpi_value_union {
    pub str_: *mut PLI_BYTE8,
    pub scalar: PLI_INT32,
    pub integer: PLI_INT32,
    pub real: f64,
    pub time: *mut t_vpi_time,
    pub vector: *mut t_vpi_vecval,
    pub strength: *mut c_void,
    pub misc: *mut PLI_BYTE8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct t_vpi_value {
    pub format: PLI_INT32,
    pub value: u_vpi_value_union,
}

/// One 32-bit word of a four-state VPI vector. `aval` carries the value bits;
/// a set bit in `bval` marks the corresponding bit as X/Z.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct t_vpi_vecval {
    pub aval: PLI_UINT32,
    pub bval: PLI_UINT32,
}

#[repr(C)]
pub struct t_cb_data {
    pub reason: PLI_INT32,
    pub cb_rtn: Option<extern "C" fn(*mut t_cb_data) -> PLI_INT32>,
    pub obj: vpiHandle,
    pub time: *mut t_vpi_time,
    pub value: *mut t_vpi_value,
    pub index: PLI_INT32,
    pub user_data: *mut PLI_BYTE8,
}

// ---------------------------------------------------------------------------
// Functions. Resolved at load time against the simulator (vvp exports them
// to dlopen'ed VPI modules).
// ---------------------------------------------------------------------------
extern "C" {
    pub fn vpi_handle_by_name(name: *const PLI_BYTE8, scope: vpiHandle) -> vpiHandle;
    pub fn vpi_handle_by_index(object: vpiHandle, index: PLI_INT32) -> vpiHandle;
    pub fn vpi_iterate(type_: PLI_INT32, ref_: vpiHandle) -> vpiHandle;
    pub fn vpi_scan(iterator: vpiHandle) -> vpiHandle;
    pub fn vpi_get(property: PLI_INT32, object: vpiHandle) -> PLI_INT32;
    pub fn vpi_get_str(property: PLI_INT32, object: vpiHandle) -> *mut PLI_BYTE8;
    pub fn vpi_get_value(expr: vpiHandle, value_p: *mut t_vpi_value);
    pub fn vpi_put_value(
        object: vpiHandle,
        value_p: *mut t_vpi_value,
        time_p: *mut t_vpi_time,
        flags: PLI_INT32,
    ) -> vpiHandle;
    pub fn vpi_get_time(object: vpiHandle, time_p: *mut t_vpi_time);
    pub fn vpi_register_cb(cb_data_p: *mut t_cb_data) -> vpiHandle;
    pub fn vpi_remove_cb(cb_obj: vpiHandle) -> PLI_INT32;
    pub fn vpi_free_object(object: vpiHandle) -> PLI_INT32;
    pub fn vpi_control(operation: PLI_INT32, ...) -> PLI_INT32;
    pub fn vpi_printf(format: *const PLI_BYTE8, ...) -> PLI_INT32;
}
