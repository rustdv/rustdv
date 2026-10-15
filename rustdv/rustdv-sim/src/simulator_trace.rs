//! Optional runtime waveform capture supplied by the active simulator host.
//!
//! The API describes lifecycle and capability, not a waveform file format.
//! A simulator may implement the host ABI using its own recording mechanism.
//! Callers should treat [`TraceError::Unavailable`] as a normal result when
//! the current simulator or executable has no runtime capture support.
//!
//! With a trace host present, operations must run on the simulator thread at
//! a settled ReadOnly point, normally inside [`crate::service_read_only`].
//! Host availability and path validation may be reported before phase errors.

use std::ffi::CString;
use std::fmt;
use std::os::raw::{c_char, c_int};
use std::path::Path;

const ABI_VERSION: u32 = 1;
const COMMAND_STATUS: u32 = 0;
const COMMAND_START: u32 = 1;
const COMMAND_STOP: u32 = 2;
const COMMAND_FLUSH: u32 = 3;
const ERROR_CAPACITY: usize = 512;

#[repr(C)]
#[derive(Default)]
struct RawTraceStatus {
    abi_version: u32,
    capability: u32,
    state: u32,
    reserved: u32,
    start_time_steps: u64,
    end_time_steps: u64,
    dump_count: u64,
}

type TraceControl = unsafe extern "C" fn(
    command: u32,
    path: *const c_char,
    status: *mut RawTraceStatus,
    error: *mut c_char,
    error_capacity: usize,
) -> c_int;

#[cfg(unix)]
type TraceFormatProbe = unsafe extern "C" fn() -> u32;

/// Output format advertised by an optional runtime trace host.
///
/// A host with an unrecognized format can still provide the generic control
/// ABI, but consumers must not guess how to decode its output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceFormat {
    Fst,
    Other(u32),
}

/// State reported by the active simulator's trace host.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceState {
    /// The executable has no runtime waveform capture capability.
    Unavailable,
    /// Capture support is present but no capture is active.
    Idle,
    /// A capture is currently accepting settled simulation values.
    Active,
}

/// Runtime trace status returned by the active simulator host.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceStatus {
    pub state: TraceState,
    pub start_time_steps: u64,
    pub end_time_steps: u64,
    pub dump_count: u64,
}

/// Errors from the optional simulator trace host.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TraceError {
    Unavailable(String),
    InvalidPath(String),
    WrongState(String),
    Host(String),
}

impl fmt::Display for TraceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable(message) => write!(formatter, "{message}"),
            Self::InvalidPath(message) => write!(formatter, "{message}"),
            Self::WrongState(message) => write!(formatter, "{message}"),
            Self::Host(message) => write!(formatter, "{message}"),
        }
    }
}

impl std::error::Error for TraceError {}

/// Return the current runtime trace capability and state.
pub fn status() -> Result<TraceStatus, TraceError> {
    control(COMMAND_STATUS, None)
}

/// Whether the process exports the optional simulator trace-control ABI.
///
/// This is a host-presence probe, not a promise that the current model was
/// compiled with tracing. Unlike [`status`], it is safe before ReadOnly and
/// lets a debug transport choose its backend during testbench startup.
pub fn host_present() -> bool {
    load_control().is_ok()
}

/// Identify the host's trace format without entering a simulator phase.
///
/// Older Verilator hosts export only the legacy control symbol; they are
/// known to write FST and are recognized for mixed-version testbenches.
/// Unknown hosts return `None` rather than being mistaken for FST.
pub fn host_format() -> Option<TraceFormat> {
    #[cfg(unix)]
    {
        let library = libloading::os::unix::Library::this();
        // SAFETY: both symbols are supplied by the simulator executable and
        // remain loaded for the lifetime of the testbench library.
        unsafe {
            if let Ok(probe) = library.get::<TraceFormatProbe>(b"rustdv_simulator_trace_format\0") {
                return Some(match probe() {
                    1 => TraceFormat::Fst,
                    other => TraceFormat::Other(other),
                });
            }
            if library
                .get::<TraceControl>(b"rustdv_verilator_trace_control\0")
                .is_ok()
            {
                return Some(TraceFormat::Fst);
            }
        }
    }
    None
}

/// Start capturing the signals retained in the trace-capable model.
/// The backend determines the output format and required build instrumentation.
pub fn start(path: &Path) -> Result<TraceStatus, TraceError> {
    let path = path_to_c_string(path)?;
    control(COMMAND_START, Some(&path))
}

/// Flush the active capture so its bounded on-disk state can be inspected.
pub fn flush() -> Result<TraceStatus, TraceError> {
    control(COMMAND_FLUSH, None)
}

/// Stop and close the active capture at the current settled simulation time.
pub fn stop() -> Result<TraceStatus, TraceError> {
    control(COMMAND_STOP, None)
}

fn control(command: u32, path: Option<&CString>) -> Result<TraceStatus, TraceError> {
    let function = load_control()?;
    if crate::phase::current_phase_if_initialized() != Some(crate::phase::SimPhase::ReadOnly) {
        return Err(TraceError::WrongState(
            "simulator trace control must run on the simulator thread at settled ReadOnly"
                .to_owned(),
        ));
    }
    let mut raw = RawTraceStatus::default();
    let mut error = [0_u8; ERROR_CAPACITY];
    let path = path.map_or(std::ptr::null(), |value| value.as_ptr());

    // SAFETY: the dynamically loaded function is checked against the stable
    // C ABI declared above.  All pointers remain valid for the duration of
    // the call and the host is required to NUL-terminate its error buffer.
    let result = unsafe {
        function(
            command,
            path,
            &mut raw,
            error.as_mut_ptr().cast(),
            error.len(),
        )
    };
    let message = error_message(&error);

    match result {
        0 => decode_status(raw),
        1 => Err(TraceError::Unavailable(message)),
        3 => Err(TraceError::InvalidPath(message)),
        4 => Err(TraceError::WrongState(message)),
        _ => Err(TraceError::Host(message)),
    }
}

fn decode_status(raw: RawTraceStatus) -> Result<TraceStatus, TraceError> {
    if raw.abi_version != ABI_VERSION {
        return Err(TraceError::Host(format!(
            "unsupported simulator trace ABI {} (expected {ABI_VERSION})",
            raw.abi_version
        )));
    }
    let state = match (raw.capability, raw.state) {
        (0, _) => TraceState::Unavailable,
        (1, 0) => TraceState::Idle,
        (1, 1) => TraceState::Active,
        (capability, state) => {
            return Err(TraceError::Host(format!(
                "invalid simulator trace status capability={capability} state={state}"
            )));
        }
    };
    Ok(TraceStatus {
        state,
        start_time_steps: raw.start_time_steps,
        end_time_steps: raw.end_time_steps,
        dump_count: raw.dump_count,
    })
}

fn error_message(buffer: &[u8]) -> String {
    let terminator = buffer
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(buffer.len());
    let message = String::from_utf8_lossy(&buffer[..terminator]).into_owned();
    if message.is_empty() {
        "simulator trace host returned an unspecified error".to_owned()
    } else {
        message
    }
}

#[cfg(unix)]
fn path_to_c_string(path: &Path) -> Result<CString, TraceError> {
    use std::os::unix::ffi::OsStrExt;
    CString::new(path.as_os_str().as_bytes())
        .map_err(|_| TraceError::InvalidPath("trace path contains an interior NUL byte".to_owned()))
}

#[cfg(not(unix))]
fn path_to_c_string(path: &Path) -> Result<CString, TraceError> {
    CString::new(path.to_string_lossy().as_bytes())
        .map_err(|_| TraceError::InvalidPath("trace path contains an interior NUL byte".to_owned()))
}

#[cfg(unix)]
fn load_control() -> Result<TraceControl, TraceError> {
    let library = libloading::os::unix::Library::this();
    // SAFETY: copying the function pointer detaches it from the short-lived
    // symbol guard.  The symbol belongs to the simulator executable, which
    // necessarily outlives the loaded VPI testbench.
    resolve_control(|name| unsafe { library.get::<TraceControl>(name).ok().map(|symbol| *symbol) })
        .ok_or_else(|| {
            TraceError::Unavailable(
                "the current simulator host does not provide runtime waveform capture".to_owned(),
            )
        })
}

#[cfg(unix)]
fn resolve_control(mut lookup: impl FnMut(&[u8]) -> Option<TraceControl>) -> Option<TraceControl> {
    lookup(b"rustdv_simulator_trace_control\0")
        .or_else(|| lookup(b"rustdv_verilator_trace_control\0"))
}

#[cfg(not(unix))]
fn load_control() -> Result<TraceControl, TraceError> {
    Err(TraceError::Unavailable(
        "runtime simulator trace control is currently supported on Unix hosts only".to_owned(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    unsafe extern "C" fn legacy_control(
        _: u32,
        _: *const c_char,
        _: *mut RawTraceStatus,
        _: *mut c_char,
        _: usize,
    ) -> c_int {
        0
    }

    #[cfg(unix)]
    #[test]
    fn legacy_verilator_host_symbol_remains_supported() {
        let mut looked_up = Vec::new();
        let control = resolve_control(|name| {
            looked_up.push(name.to_vec());
            (name == b"rustdv_verilator_trace_control\0").then_some(legacy_control as TraceControl)
        });
        assert!(std::ptr::fn_addr_eq(
            control.expect("legacy symbol must resolve"),
            legacy_control as TraceControl
        ));
        assert_eq!(
            looked_up,
            [
                b"rustdv_simulator_trace_control\0".to_vec(),
                b"rustdv_verilator_trace_control\0".to_vec(),
            ]
        );
    }

    #[test]
    fn ordinary_rust_test_reports_missing_trace_host() {
        assert!(!host_present());
        assert_eq!(host_format(), None);
        let error = status().unwrap_err();
        assert!(matches!(error, TraceError::Unavailable(_)));
    }
}
