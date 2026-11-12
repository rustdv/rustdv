//! VPI backend: the simulator loads the testbench as a VPI module and owns
//! the `vpi_*` entry points. Every `unsafe` call into the simulator lives
//! here; the crate root builds the typed handle API on top.

use std::borrow::Cow;
use std::cell::Cell;
use std::ffi::{CStr, CString};

use rustdv_gpi_sys as sys;

use crate::ValueError;
use crate::backend::{Backend, CbData, CbToken, ObjHandle, ObjKind, Reason};

/// The backend of a testbench loaded into a VPI simulator. Used whenever no
/// other backend is installed.
pub struct VpiBackend;

fn raw(h: ObjHandle) -> sys::vpiHandle {
    h.as_raw() as sys::vpiHandle
}

fn handle(h: sys::vpiHandle) -> Option<ObjHandle> {
    if h.is_null() {
        None
    } else {
        // SAFETY: a non-null handle the simulator returned, used only by
        // this backend.
        Some(unsafe { ObjHandle::from_raw(h as usize) })
    }
}

fn get(h: ObjHandle, prop: i32) -> i32 {
    unsafe { sys::vpi_get(prop, raw(h)) }
}

fn get_str(h: ObjHandle, prop: i32) -> String {
    // Invariant 3: copy immediately.
    unsafe {
        let p = sys::vpi_get_str(prop, raw(h));
        if p.is_null() {
            String::new()
        } else {
            CStr::from_ptr(p).to_string_lossy().into_owned()
        }
    }
}

/// Every object `vpi_scan` yields from an iterator over `object`.
fn scan(type_: i32, object: sys::vpiHandle) -> Vec<ObjHandle> {
    let mut out = Vec::new();
    unsafe {
        let it = sys::vpi_iterate(type_, object);
        if it.is_null() {
            return out;
        }
        // scan returning NULL frees the iterator
        while let Some(h) = handle(sys::vpi_scan(it)) {
            out.push(h);
        }
    }
    out
}

fn simtime(steps: u64) -> sys::t_vpi_time {
    sys::t_vpi_time {
        type_: sys::vpiSimTime,
        high: (steps >> 32) as u32,
        low: (steps & 0xFFFF_FFFF) as u32,
        real: 0.0,
    }
}

extern "C" fn trampoline(cb: *mut sys::t_cb_data) -> i32 {
    unsafe { CbData::from_ptr((*cb).user_data.cast()).dispatch() };
    0
}

impl Backend for VpiBackend {
    fn top_modules(&self) -> Vec<ObjHandle> {
        scan(sys::vpiModule, std::ptr::null_mut())
    }

    fn handle_by_name(&self, name: &str) -> Option<ObjHandle> {
        let cname = CString::new(name).expect("NUL in object name");
        handle(unsafe { sys::vpi_handle_by_name(cname.as_ptr(), std::ptr::null_mut()) })
    }

    fn kind(&self, h: ObjHandle) -> ObjKind {
        match get(h, sys::vpiType) {
            sys::vpiModule => ObjKind::Hierarchy,
            sys::vpiRealVar => ObjKind::Real,
            sys::vpiStringVar => ObjKind::String,
            sys::vpiStructVar => ObjKind::Struct,
            sys::vpiUnionVar => ObjKind::Union,
            sys::vpiNet
            | sys::vpiReg
            | sys::vpiIntegerVar
            | sys::vpiPort
            | sys::vpiMemory
            | sys::vpiLongIntVar
            | sys::vpiShortIntVar
            | sys::vpiIntVar
            | sys::vpiByteVar
            | sys::vpiEnumVar
            | sys::vpiBitVar => ObjKind::Logic {
                width: get(h, sys::vpiSize).max(0) as u32,
            },
            _ => ObjKind::Other,
        }
    }

    fn name(&self, h: ObjHandle) -> String {
        get_str(h, sys::vpiName)
    }

    fn full_name(&self, h: ObjHandle) -> String {
        get_str(h, sys::vpiFullName)
    }

    fn type_desc(&self, h: ObjHandle) -> String {
        format!("vpiType {}", get(h, sys::vpiType))
    }

    fn child(&self, scope: ObjHandle, name: &str) -> Option<ObjHandle> {
        let cname = CString::new(name).expect("NUL in signal name");
        handle(unsafe { sys::vpi_handle_by_name(cname.as_ptr(), raw(scope)) })
    }

    fn children(&self, scope: ObjHandle) -> Vec<ObjHandle> {
        [sys::vpiModule, sys::vpiNet, sys::vpiReg]
            .into_iter()
            .flat_map(|t| scan(t, raw(scope)))
            .collect()
    }

    fn member(&self, aggregate: ObjHandle, name: &str) -> Option<ObjHandle> {
        let path = CString::new(format!("{}.{}", self.full_name(aggregate), name))
            .expect("NUL in member name");
        handle(unsafe { sys::vpi_handle_by_name(path.as_ptr(), std::ptr::null_mut()) })
    }

    fn members(&self, aggregate: ObjHandle) -> Vec<ObjHandle> {
        scan(sys::vpiMember, raw(aggregate))
    }

    fn get_real(&self, h: ObjHandle) -> Result<f64, ValueError> {
        let mut value = sys::t_vpi_value {
            format: sys::vpiRealVal,
            value: sys::u_vpi_value_union { real: 0.0 },
        };
        unsafe {
            sys::vpi_get_value(raw(h), &mut value);
            if value.format != sys::vpiRealVal {
                return Err(ValueError::Unavailable);
            }
            Ok(value.value.real)
        }
    }

    fn put_real(&self, h: ObjHandle, value: f64) {
        let mut value = sys::t_vpi_value {
            format: sys::vpiRealVal,
            value: sys::u_vpi_value_union { real: value },
        };
        unsafe {
            sys::vpi_put_value(raw(h), &mut value, std::ptr::null_mut(), sys::vpiNoDelay);
        }
    }

    fn get_string(&self, h: ObjHandle) -> Result<String, ValueError> {
        let mut value = sys::t_vpi_value {
            format: sys::vpiStringVal,
            value: sys::u_vpi_value_union {
                str_: std::ptr::null_mut(),
            },
        };
        unsafe {
            sys::vpi_get_value(raw(h), &mut value);
            if value.format != sys::vpiStringVal || value.value.str_.is_null() {
                return Err(ValueError::Unavailable);
            }
            Ok(CStr::from_ptr(value.value.str_)
                .to_string_lossy()
                .into_owned())
        }
    }

    fn put_string(&self, h: ObjHandle, value: &str) -> Result<(), ValueError> {
        let text = CString::new(value).map_err(|_| ValueError::InteriorNul)?;
        let mut value = sys::t_vpi_value {
            format: sys::vpiStringVal,
            value: sys::u_vpi_value_union {
                str_: text.as_ptr().cast_mut(),
            },
        };
        unsafe {
            sys::vpi_put_value(raw(h), &mut value, std::ptr::null_mut(), sys::vpiNoDelay);
        }
        Ok(())
    }

    /// Borrowed from the simulator until the next VPI call.
    fn get_vector(
        &self,
        h: ObjHandle,
        width: u32,
    ) -> Result<Cow<'_, [sys::t_vpi_vecval]>, ValueError> {
        let mut val = sys::t_vpi_value {
            format: sys::vpiVectorVal,
            value: sys::u_vpi_value_union {
                vector: std::ptr::null_mut(),
            },
        };
        unsafe {
            sys::vpi_get_value(raw(h), &mut val);
            let words = val.value.vector;
            if words.is_null() {
                return Err(ValueError::Unavailable);
            }
            let word_count = width.div_ceil(32) as usize;
            Ok(Cow::Borrowed(std::slice::from_raw_parts(words, word_count)))
        }
    }

    fn put_vector(&self, h: ObjHandle, words: &mut [sys::t_vpi_vecval], flags: i32) {
        unsafe {
            let mut vpi_value = sys::t_vpi_value {
                format: sys::vpiVectorVal,
                value: sys::u_vpi_value_union {
                    vector: words.as_mut_ptr(),
                },
            };
            sys::vpi_put_value(raw(h), &mut vpi_value, std::ptr::null_mut(), flags);
        }
    }

    fn sim_time_steps(&self) -> u64 {
        let mut t = sys::t_vpi_time {
            type_: sys::vpiSimTime,
            high: 0,
            low: 0,
            real: 0.0,
        };
        unsafe { sys::vpi_get_time(std::ptr::null_mut(), &mut t) };
        ((t.high as u64) << 32) | (t.low as u64)
    }

    fn time_precision(&self) -> i32 {
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

    fn finish(&self) {
        unsafe {
            sys::vpi_control(sys::vpiFinish, 0i32);
        }
    }

    fn register_cb(&self, reason: Reason, data: CbData) -> CbToken {
        let (reason, obj, time) = match reason {
            Reason::AfterDelay(steps) => (
                sys::cbAfterDelay,
                std::ptr::null_mut(),
                Some(simtime(steps)),
            ),
            Reason::ValueChange(sig) => (sys::cbValueChange, raw(sig), Some(simtime(0))),
            Reason::ReadWrite => (
                sys::cbReadWriteSynch,
                std::ptr::null_mut(),
                Some(simtime(0)),
            ),
            Reason::ReadOnly => (sys::cbReadOnlySynch, std::ptr::null_mut(), Some(simtime(0))),
            Reason::NextSimTime => (sys::cbNextSimTime, std::ptr::null_mut(), None),
            Reason::AtEndOfSimTime => (
                sys::cbAtEndOfSimTime,
                std::ptr::null_mut(),
                Some(simtime(self.sim_time_steps())),
            ),
            Reason::StartOfSimulation => (sys::cbStartOfSimulation, std::ptr::null_mut(), None),
            Reason::EndOfSimulation => (sys::cbEndOfSimulation, std::ptr::null_mut(), None),
        };
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
            user_data: data.as_ptr().cast(),
        };
        let vpi_h = unsafe { sys::vpi_register_cb(&mut cb) };
        assert!(!vpi_h.is_null(), "vpi_register_cb failed (reason {reason})");
        vpi_h as CbToken
    }

    fn remove_cb(&self, token: CbToken) {
        unsafe {
            sys::vpi_remove_cb(token as sys::vpiHandle);
        }
    }
}
