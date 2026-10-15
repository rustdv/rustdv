//! # rustdv
//!
//! A hardware verification framework in Rust: cocotb-style simulator
//! coroutines plus a UVM-analog testbench library, rethought for Rust's
//! ownership model.
//!
//! This is the facade crate (design-doc D2.5): it re-exports the public
//! API so a testbench needs one dependency and one import:
//!
//! ```ignore
//! use rustdv::prelude::*;
//!
//! rustdv::vpi_bootstrap!();
//!
//! #[rustdv::test]
//! async fn my_test(ctx: RustdvCtx) -> Result<(), TestError> {
//!     let dut = ctx.dut();
//!     // ...
//!     Ok(())
//! }
//! ```

// --- sub-crates, re-exported whole for power users -------------------------
pub use rustdv_gpi as gpi;
pub use rustdv_runner as runner;
pub use rustdv_sim as sim;
// The methodology layer is NOT re-exported as a whole-crate module: its
// public items are on the curated surface below and in the prelude, so
// testbenches reach them as `rustdv::Factory`, `rustdv::ConfigDb`, etc.
// (No `rustdv::uvm` — rustdv is not an implementation of IEEE 1800.2.)

// --- macros -----------------------------------------------------------------
/// `#[derive(Component)]` — ComponentNode traversal (design-doc §6.3).
pub use rustdv_macros::Component;
/// `#[rustdv::test]` — port of `@cocotb.test()` (design-doc §6.1).
pub use rustdv_macros::test;

// first!/join! are #[macro_export]ed by rustdv-sim at its crate root.
pub use rustdv_sim::{first, join};

// --- the curated surface ----------------------------------------------------
pub use rustdv_runner::{TEST_REGISTRATIONS, TestRegistration};

pub use rustdv_sim::handle::top_module;
pub use rustdv_sim::log;
pub use rustdv_sim::{
    AggregateHandle, BigUint, Clock, Either, Event, Executor, HandleError, HierarchyHandle, Lock,
    LockGuard, Logic, LogicArray, LogicHandle, NullTrigger, Queue, RealHandle, Rng, RustdvPath,
    SimDuration, SimHandle, StringHandle, TaskError, TaskHandle, TaskState, TimeoutError, Timer,
    ValueError, first2,
    handle::{HandleChildren, HandleEvent},
    join2, next_time_step, read_only, read_write, service_read_only, sim_time_ns, sim_time_steps,
    spawn, spawn_named, with_timeout,
};

pub use rustdv_methodology::{
    Active, AnalysisBus, COMPONENT_REGISTRATIONS, CheckSink, Component as ComponentTrait,
    ComponentNode, ComponentReg, ConfigDb, ConfigError, ConnectError, DynPhases, Factory,
    GetExport, GetIf, GetPort, Maker, ObjectionGuard, ObjectionRegistry, PeekExport, PeekIf,
    PeekPort, Port, PortField, PortInfo, PortName, PortOwner, PublishExport, PublishIf,
    PublishPort, PutExport, PutIf, PutPort, Receiver, ResponseQueue, RustdvComp, RustdvSeq,
    RustdvShared, Sender, SeqCtx, SeqError, SeqItem, SeqItemExport, SeqItemIf, SeqItemPort,
    Sequence, Sequencer, SinkHandle, SubscribeExport, SubscribePort, Subscriber, TapExport,
    TestError, TlmEmpty, TlmError, TlmFifo, TlmFull, TxnId, build_all, channel, check_all,
    check_connections, connect_all, create_seq, end_of_elaboration_all, extract_all, final_all,
    print_hierarchy, report_all, run_all, run_component_test, run_extract_check_report,
    set_seq_override, start_all, start_of_simulation_all, unconnected_ports,
};

/// Implementation details used by rustdv's procedural-macro expansions.
#[doc(hidden)]
pub mod __private {
    pub use linkme;
}

// The lifecycle trait under its design-doc name, in the type namespace.
// (The derive macro of the same name lives in the macro namespace; Rust
// resolves them independently.)
pub use rustdv_methodology::Component;

/// One-line import for testbenches (the `from pyuvm import *` analog).
pub mod prelude {
    pub use crate::log;
    pub use crate::{
        Active, AggregateHandle, AnalysisBus, BigUint, CheckSink, Clock, Component, ComponentNode,
        ConfigDb, Either, Event, Factory, GetPort, HandleChildren, HandleError, HandleEvent,
        HierarchyHandle, Lock, Logic, LogicArray, LogicHandle, NullTrigger, ObjectionGuard,
        PeekPort, PortName, PortOwner, PublishPort, PutPort, Queue, RealHandle, Receiver, Rng,
        RustdvComp, RustdvCtx, RustdvSeq, RustdvShared, Sender, SeqCtx, SeqError, SeqItem,
        SeqItemExport, SeqItemPort, Sequence, Sequencer, SimDuration, StringHandle, SubscribePort,
        Subscriber, TaskHandle, TestError, Timer, TlmFifo, TxnId, build_all, channel, check_all,
        connect_all, create_seq, end_of_elaboration_all, extract_all, final_all, first2, join2,
        next_time_step, print_hierarchy, read_only, read_write, report_all, run_component_test,
        run_extract_check_report, service_read_only, set_seq_override, sim_time_ns, spawn,
        spawn_named, start_all, start_of_simulation_all, with_timeout,
    };
}

/// The one context every testbench is handed (D47: `TestCtx` and `RunCtx`
/// merged). Named for the framework, not for a phase, because Part II
/// teaches testbenches that have no phases.
pub use rustdv_methodology::RustdvCtx;

/// Export the VPI entry points from the testbench cdylib. The simulator
/// (vvp) dlopens the library and calls each routine in
/// `vlog_startup_routines`; ours registers the start-of-simulation
/// callback that launches the regression (design-doc §3.2, as deviated —
/// see rustdv-runner docs and STATUS.md).
#[macro_export]
macro_rules! vpi_bootstrap {
    () => {
        #[unsafe(no_mangle)]
        pub extern "C" fn __rustdv_vpi_entry() {
            $crate::runner::vpi_startup();
        }

        #[unsafe(no_mangle)]
        #[allow(non_upper_case_globals)]
        pub static vlog_startup_routines: [::core::option::Option<extern "C" fn()>; 2] = [
            ::core::option::Option::Some(__rustdv_vpi_entry),
            ::core::option::Option::None,
        ];
    };
}
