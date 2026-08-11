// Shared rustdv host for Verilated designs.
//
// Verilator's generated --main loop advances only while the RTL model has a
// pending event.  A rustdv testbench can instead be waiting solely on a VPI
// timer, so the generated loop may stop at time zero.  This host treats the
// RTL and VPI queues as peers and implements the simulator phase ordering the
// framework relies on.

#include "Vrustdv_dut.h"
#include "verilated.h"
#include "verilated_vpi.h"

#if VM_TRACE_FST
#include "verilated_fst_c.h"
#endif

#include <algorithm>
#include <chrono>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <limits>
#include <memory>

// Statically linked VPI modules populate this array.  rustdv normally uses
// Verilator 5.050's +verilator+vpi+<library> runtime loader, where it is null.
extern "C" void (*vlog_startup_routines[])() VL_ATTR_WEAK;

namespace {

constexpr std::uint64_t kNoDeadline = std::numeric_limits<std::uint64_t>::max();
constexpr unsigned kSettleLimit = 100000;

struct SchedulerStats {
    using Clock = std::chrono::steady_clock;

    explicit SchedulerStats(double interval_seconds)
        : enabled(interval_seconds > 0.0)
        , interval(interval_seconds)
        , started(Clock::now())
        , last_report(started) {}

    void note_outer_iteration(std::uint64_t sim_time) {
        if (!enabled) return;
        ++outer_iterations;
        // Reading the host clock on every RTL/VPI deadline would distort the
        // benchmark. Sampling once per 65,536 deadlines is frequent enough
        // for progress while keeping the disabled path completely inert.
        if ((outer_iterations & 0xffffU) != 0) return;
        const auto now = Clock::now();
        const double since_report = std::chrono::duration<double>(now - last_report).count();
        if (since_report < interval) return;
        const double wall = std::chrono::duration<double>(now - started).count();
        std::fprintf(
            stderr,
            "RUSTDV_SCHEDULER_PROGRESS wall_s=%.3f sim_time=%llu deadlines=%llu "
            "evals=%llu settle_extra=%llu timed=%llu value=%llu rw=%llu ro=%llu\n",
            wall,
            static_cast<unsigned long long>(sim_time),
            static_cast<unsigned long long>(outer_iterations),
            static_cast<unsigned long long>(eval_calls),
            static_cast<unsigned long long>(settle_extra_iterations),
            static_cast<unsigned long long>(timed_dispatches),
            static_cast<unsigned long long>(value_dispatches),
            static_cast<unsigned long long>(read_write_dispatches),
            static_cast<unsigned long long>(read_only_dispatches));
        last_report = now;
    }

    void report_final(std::uint64_t sim_time) const {
        if (!enabled) return;
        const double wall = std::chrono::duration<double>(Clock::now() - started).count();
        std::fprintf(
            stderr,
            "RUSTDV_SCHEDULER_SUMMARY wall_s=%.3f sim_time=%llu deadlines=%llu "
            "evals=%llu settle_extra=%llu timed=%llu value=%llu rw=%llu ro=%llu\n",
            wall,
            static_cast<unsigned long long>(sim_time),
            static_cast<unsigned long long>(outer_iterations),
            static_cast<unsigned long long>(eval_calls),
            static_cast<unsigned long long>(settle_extra_iterations),
            static_cast<unsigned long long>(timed_dispatches),
            static_cast<unsigned long long>(value_dispatches),
            static_cast<unsigned long long>(read_write_dispatches),
            static_cast<unsigned long long>(read_only_dispatches));
    }

    bool enabled = false;
    double interval = 0.0;
    Clock::time_point started;
    Clock::time_point last_report;
    std::uint64_t outer_iterations = 0;
    std::uint64_t eval_calls = 0;
    std::uint64_t settle_extra_iterations = 0;
    std::uint64_t timed_dispatches = 0;
    std::uint64_t value_dispatches = 0;
    std::uint64_t read_write_dispatches = 0;
    std::uint64_t read_only_dispatches = 0;
};

double progress_interval_seconds() {
    const char* value = std::getenv("RUSTDV_VERILATOR_PROGRESS_SECONDS");
    if (!value || !*value) return 0.0;
    char* end = nullptr;
    const double parsed = std::strtod(value, &end);
    if (!end || *end || parsed <= 0.0) {
        std::fprintf(
            stderr,
            "rustdv: RUSTDV_VERILATOR_PROGRESS_SECONDS must be positive, got %s\n",
            value);
        return -1.0;
    }
    return parsed;
}

std::uint32_t runtime_threads() {
    const char* value = std::getenv("RUSTDV_VERILATOR_THREADS");
    if (!value || !*value) return 1;
    char* end = nullptr;
    const unsigned long parsed = std::strtoul(value, &end, 10);
    if (!end || *end || parsed == 0 || parsed > 4) {
        std::fprintf(stderr, "rustdv: invalid RUSTDV_VERILATOR_THREADS=%s\n", value);
        return 0;
    }
    return static_cast<std::uint32_t>(parsed);
}

bool settle(Vrustdv_dut& dut, SchedulerStats* stats) {
    for (unsigned iteration = 0; iteration < kSettleLimit; ++iteration) {
        // Apply any inertially delayed VPI writes before evaluating the RTL.
        // Clear the dirty flag because this eval consumes all writes made so
        // far; writes made by callbacks below set it again.
        VerilatedVpi::doInertialPuts();
        VerilatedVpi::clearEvalNeeded();
        dut.eval();
        if (stats) {
            ++stats->eval_calls;
            if (iteration != 0) ++stats->settle_extra_iterations;
        }

        // RTL changes wake edge/value awaiters. AtEnd callbacks precede the
        // ReadWrite and ReadOnly regions in Verilator's generated host; either
        // callback class may queue writes, so ReadOnly is not legal until this
        // reaches a fixed point.
        VerilatedVpi::callValueCbs();
        VerilatedVpi::callCbs(cbAtEndOfSimTime);
        VerilatedVpi::callCbs(cbReadWriteSynch);
        if (stats) {
            ++stats->value_dispatches;
            ++stats->read_write_dispatches;
        }

        if (!VerilatedVpi::evalNeeded()
            && !VerilatedVpi::hasCbs(cbReadWriteSynch)) {
            VerilatedVpi::callCbs(cbReadOnlySynch);
            if (stats) ++stats->read_only_dispatches;
            return true;
        }
    }

    std::fprintf(stderr,
                 "rustdv: Verilator scheduler did not settle after %u iterations at time %llu\n",
                 kSettleLimit,
                 static_cast<unsigned long long>(Verilated::time()));
    return false;
}

}  // namespace

int main(int argc, char** argv, char**) {
    Verilated::debug(0);
    const std::unique_ptr<VerilatedContext> contextp{new VerilatedContext};
    const std::uint32_t threads = runtime_threads();
    const double progress_interval = progress_interval_seconds();
    if (threads == 0 || progress_interval < 0.0) return 2;
    SchedulerStats stats(progress_interval);
    SchedulerStats* const statsp = stats.enabled ? &stats : nullptr;
    contextp->threads(threads);
    contextp->commandArgs(argc, argv);

    const std::unique_ptr<Vrustdv_dut> dutp{new Vrustdv_dut{contextp.get(), ""}};

#if VM_TRACE_FST
    std::unique_ptr<VerilatedFstC> tracep;
    if (const char* const path = std::getenv("RUSTDV_FST")) {
        contextp->traceEverOn(true);
        tracep = std::make_unique<VerilatedFstC>();
        dutp->trace(tracep.get(), 99);
        tracep->open(path);
    }
#else
    if (std::getenv("RUSTDV_FST")) {
        std::fprintf(stderr,
                     "rustdv: RUSTDV_FST was set, but this model was not built with --trace-fst\n");
        return 2;
    }
#endif

    if (vlog_startup_routines) {
        for (auto routinep = &vlog_startup_routines[0]; *routinep; ++routinep) {
            (*routinep)();
        }
    }
    VerilatedVpi::callCbs(cbStartOfSimulation);

    bool scheduler_ok = true;
    while (!contextp->gotFinish()) {
        if (statsp) stats.note_outer_iteration(contextp->time());
        // Timed callbacks run before model evaluation at their deadline.
        VerilatedVpi::callTimedCbs();
        VerilatedVpi::callCbs(cbNextSimTime);
        VerilatedVpi::callCbs(cbAtStartOfSimTime);
        if (statsp) ++stats.timed_dispatches;

        if (!settle(*dutp, statsp)) {
            scheduler_ok = false;
            break;
        }

#if VM_TRACE_FST
        if (tracep) tracep->dump(contextp->time());
#endif

        if (contextp->gotFinish()) break;

        const std::uint64_t rtl_deadline
            = dutp->eventsPending() ? dutp->nextTimeSlot() : kNoDeadline;
        const std::uint64_t vpi_deadline = VerilatedVpi::cbNextDeadline();
        const std::uint64_t deadline = std::min(rtl_deadline, vpi_deadline);

        if (deadline == kNoDeadline) break;
        if (deadline < contextp->time()) {
            std::fprintf(stderr,
                         "rustdv: Verilator scheduler deadline moved backwards (%llu < %llu)\n",
                         static_cast<unsigned long long>(deadline),
                         static_cast<unsigned long long>(contextp->time()));
            scheduler_ok = false;
            break;
        }
        contextp->time(deadline);
    }

    dutp->final();
    VerilatedVpi::callCbs(cbEndOfSimulation);
    stats.report_final(contextp->time());

#if VM_TRACE_FST
    if (tracep) tracep->close();
#endif

    contextp->statsPrintSummary();
    return scheduler_ok ? 0 : 1;
}
