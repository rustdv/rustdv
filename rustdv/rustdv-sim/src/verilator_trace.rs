//! Compatibility path for the original Verilator trace-control API.
//!
//! New testbenches should use [`crate::simulator_trace`]. The active simulator
//! supplies that API; Verilator's implementation still uses runtime-gated FST.

pub use crate::simulator_trace::{
    TraceError, TraceState, TraceStatus, flush, host_present, start, status, stop,
};
