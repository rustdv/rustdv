//! Targeted simulator tests for rustdv itself (test-plan §3).
//!
//! ```text
//! framework-tests/run.sh              every test
//! framework-tests/run.sh trig         one group
//! ```
//!
//! **These are not book examples.** The 21 `sim-ch*` regression entries are
//! the end-to-end tier: each proves a chapter's testbench compiles, runs on
//! Icarus and reports `REGRESSION: PASS`. They are good tests and they cannot
//! tell you *which* rule broke. These can — each one exercises a single
//! mechanism against a real simulator and fails with that mechanism's name.
//!
//! Everything here needs a simulator, which is the only reason it is not in
//! `cargo test`. The rule is in `rustdv_sim::testing::block_on`: a test that
//! awaits simulated time or touches a signal belongs here; everything else —
//! the ConfigDb, the factory, port binding, the analysis broadcast, the whole
//! sequencer handshake — is a `#[cfg(test)]` module beside the code it tests
//! and runs in milliseconds.
//!
//! ## Naming is the interface
//!
//! Each module prefixes its test names, and the regression selects a group
//! with `RUSTDV_TESTCASE`. So `sim-triggers` and `sim-concurrency` are
//! separate lines in the regression report but share one build and one
//! elaboration. The long `callback_stress_` group is selected only by its
//! dedicated Verilator regression; an unfiltered manual run uses its short
//! default cycle count.
//!
//! | prefix | module | what it pins down |
//! |---|---|---|
//! | `trig_` | [`triggers`] | `Timer`, edges, phase order, RAII deregistration |
//! | `clock_` | [`clocks`] | `Clock` produces the period it was asked for |
//! | `sig_` | [`signals`] | handle read/write, widths, X/Z, write scheduling |
//! | `conc_` | [`concurrency`] | the D82 family, against real time |
//! | `elab_` | [`elaboration`] | unconnected ports fail before the run phase |
//! | `runner_` | [`runner`] | timeouts, `expect_error`, per-test freshness |
//! | `callback_stress_` | [`callback_lifecycle`] | fired one-shot handles reach an RSS plateau |
//!
//! The DUT is `hdl/probe.sv`: a clock, signals of known widths that nothing
//! drives, and one counter so an edge trigger has something to trigger on.

use rustdv::prelude::*;

// The `cargo test` binary links this crate's rlib and so needs the `vpi_*`
// symbols defined, even though there are no `#[test]` functions in here and
// nothing will ever call them. The simulator provides them for the real
// cdylib; `rustdv-vpi-stubs` provides panicking placeholders for the test
// build, which is what turns "this test secretly needed a simulator" into a
// loud failure instead of a silent one.
#[cfg(test)]
use rustdv_vpi_stubs as _;

rustdv::vpi_bootstrap!();

/// Fail the test with a formatted message.
///
/// `Result` for checks, panics for testbench bugs (§7.3's taxonomy), and a
/// check that fires should say what it expected — `assert_eq!` would panic,
/// which is the wrong half of the taxonomy and reads as a framework crash in
/// the transcript.
///
/// Declared before the `mod` lines because `macro_rules!` is textually
/// scoped: a module declared above this point would not see it.
#[macro_export]
macro_rules! check {
    ($cond:expr, $($arg:tt)*) => {
        if !($cond) {
            return ::core::result::Result::Err(
                ::rustdv::TestError::new(format!($($arg)*)));
        }
    };
}

pub mod callback_lifecycle;
pub mod clocks;
pub mod concurrency;
pub mod elaboration;
pub mod runner;
pub mod signals;
pub mod triggers;

/// How many simulator time steps make one nanosecond, measured rather than
/// assumed.
///
/// `probe.sv` says `1ns/1ns`, but a test that hard-codes the ratio breaks
/// silently the day someone changes the timescale — it would still pass, on
/// the wrong numbers. Advancing a known 1ns and reading both clocks gives the
/// ratio and proves the two time functions agree on the way past.
pub async fn steps_per_ns() -> u64 {
    let (t0_steps, t0_ns) = (rustdv::sim_time_steps(), sim_time_ns());
    Timer::ns(1).await;
    let _ = t0_ns;
    rustdv::sim_time_steps() - t0_steps
}
