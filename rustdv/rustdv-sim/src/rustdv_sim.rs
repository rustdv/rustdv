//! # rustdv-sim
//!
//! The cocotb-analog core (design-doc §4): a bespoke single-threaded
//! executor driven by simulator callbacks (D4.1 — not tokio), the task
//! model, the trigger inventory, typed signal handles with buffered
//! writes, `Clock`, and sim-aware `Event`/`Lock`/`Queue`.
//!
//! Control-flow contract (§4.1): a GPI/VPI callback fires → subscribed
//! tasks are woken → the run queue is drained to exhaustion
//! ([`Executor::run_until_idle`]) → control returns to the simulator.

// Test executables need vpi_* symbol definitions (the simulator provides
// them for the real cdylib) — see rustdv-vpi-stubs.
#[cfg(test)]
use rustdv_vpi_stubs as _;

pub mod clock;
pub mod combinators;
pub mod executor;
pub mod handle;
pub mod log;
pub mod path;
pub mod phase;
pub mod queue;
pub mod rng;
pub mod sync;
pub mod testing;
pub mod time;
pub mod triggers;
pub mod verilator_trace;

pub use clock::Clock;
pub use combinators::{first2, join2, join_all, with_timeout, Either, TimeoutError};
pub use executor::{spawn, spawn_named, Executor, TaskError, TaskHandle, TaskId, TaskState};
pub use path::RustdvPath;
pub use handle::{AnyHandle, HierarchyHandle, LogicHandle};
pub use phase::{next_time_step, read_only, read_write, service_read_only};
pub use queue::Queue;
pub use rng::Rng;
pub use sync::{Event, Lock, LockGuard};
pub use time::{sim_time_ns, sim_time_steps, SimDuration};
pub use triggers::{NullTrigger, Timer};

pub use rustdv_gpi::{HandleError, Logic, LogicArray, ValueError};

/// Initialize the sim context: install a fresh executor and phase hub on
/// this thread. Called once by the runner at start-of-simulation.
pub fn init() -> Executor {
    phase::init_hub();
    executor::init()
}
