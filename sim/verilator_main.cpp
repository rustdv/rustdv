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

#if VM_COVERAGE
#include "verilated_cov.h"
#endif

#if VM_TRACE_FST
#include "verilated_fst_c.h"
#endif

#include <algorithm>
#include <chrono>
#include <cstdint>
#include <cstring>
#include <cstdio>
#include <cstdlib>
#include <limits>
#include <memory>

// Statically linked VPI modules populate this array.  rustdv normally uses
// Verilator 5.050's +verilator+vpi+<library> runtime loader, where it is null.
extern "C" void (*vlog_startup_routines[])() VL_ATTR_WEAK;

#if defined(_WIN32)
#define RUSTDV_EXPORT __declspec(dllexport)
#else
#define RUSTDV_EXPORT __attribute__((visibility("default")))
#endif

struct RustdvTraceStatus {
    std::uint32_t abi_version;
    std::uint32_t capability;
    std::uint32_t state;
    std::uint32_t reserved;
    std::uint64_t start_time_steps;
    std::uint64_t end_time_steps;
    std::uint64_t dump_count;
};

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

class RuntimeTrace final {
  public:
    RuntimeTrace(VerilatedContext& context, Vrustdv_dut& dut)
        : context_{context}, dut_{dut} {
#if VM_TRACE_FST
        // Verilator requires traceEverOn before time zero.  This only executes
        // in models compiled with --trace-fst; FAST remains uninstrumented.
        context_.traceEverOn(true);
#endif
    }

    bool available() const {
#if VM_TRACE_FST
        return true;
#else
        return false;
#endif
    }

    bool active() const {
#if VM_TRACE_FST
        return static_cast<bool>(tracep_);
#else
        return false;
#endif
    }

    std::uint64_t start_time() const { return start_time_; }
    std::uint64_t end_time() const { return end_time_; }
    std::uint64_t dump_count() const { return dump_count_; }

    bool start(const char* path, bool dump_now, const char*& error, int& error_code) {
#if VM_TRACE_FST
        error_code = 4;
        if (tracep_) {
            error = "an FST capture is already active";
            return false;
        }
        if (!path || !*path) {
            error = "an FST capture path is required";
            error_code = 3;
            return false;
        }

        // Verilator 5.050's isOpen() only reports that its writer object was
        // allocated; it does not prove the underlying stream opened.  Probe
        // the exact destination first.  Starting a capture owns and truncates
        // this path anyway, so the probe has the same file semantics as the
        // trace writer and covers both runtime and legacy DEBUG starts.
        std::FILE* const probep = std::fopen(path, "wb");
        if (!probep) {
            error = "failed to create the requested FST capture file";
            error_code = 2;
            return false;
        }
        if (std::fclose(probep) != 0) {
            std::remove(path);
            error = "failed to close the requested FST capture file probe";
            error_code = 2;
            return false;
        }

        tracep_ = std::make_unique<VerilatedFstC>();
        dut_.trace(tracep_.get(), 99);
        tracep_->open(path);
        if (!tracep_->isOpen()) {
            tracep_.reset();
            std::remove(path);
            error = "failed to open the requested FST capture path";
            error_code = 2;
            return false;
        }
        start_time_ = context_.time();
        end_time_ = start_time_;
        dump_count_ = 0;
        has_last_dump_time_ = false;
        if (dump_now) {
            dump_settled_time();
            tracep_->flush();
        }
        return true;
#else
        (void)path;
        (void)dump_now;
        (void)error_code;
        error = "this Verilator model was not built with --trace-fst";
        return false;
#endif
    }

    bool flush(const char*& error) {
#if VM_TRACE_FST
        if (!tracep_) {
            error = "no FST capture is active";
            return false;
        }
        dump_settled_time();
        tracep_->flush();
        return true;
#else
        error = "this Verilator model was not built with --trace-fst";
        return false;
#endif
    }

    bool stop(const char*& error) {
#if VM_TRACE_FST
        if (!tracep_) {
            error = "no FST capture is active";
            return false;
        }
        dump_settled_time();
        tracep_->close();
        tracep_.reset();
        end_time_ = context_.time();
        return true;
#else
        error = "this Verilator model was not built with --trace-fst";
        return false;
#endif
    }

    void dump_settled_time() {
#if VM_TRACE_FST
        if (!tracep_) return;
        const std::uint64_t now = context_.time();
        if (has_last_dump_time_ && last_dump_time_ == now) return;
        tracep_->dump(now);
        last_dump_time_ = now;
        has_last_dump_time_ = true;
        end_time_ = now;
        ++dump_count_;
#endif
    }

  private:
    VerilatedContext& context_;
    Vrustdv_dut& dut_;
#if VM_TRACE_FST
    std::unique_ptr<VerilatedFstC> tracep_;
#endif
    std::uint64_t start_time_{0};
    std::uint64_t end_time_{0};
    std::uint64_t dump_count_{0};
    std::uint64_t last_dump_time_{0};
    bool has_last_dump_time_{false};
};

RuntimeTrace* runtime_tracep = nullptr;

void fill_trace_status(RustdvTraceStatus* status) {
    if (!status) return;
    std::memset(status, 0, sizeof(*status));
    status->abi_version = 1;
    if (!runtime_tracep) return;
    status->capability = runtime_tracep->available() ? 1 : 0;
    status->state = runtime_tracep->active() ? 1 : 0;
    status->start_time_steps = runtime_tracep->start_time();
    status->end_time_steps = runtime_tracep->end_time();
    status->dump_count = runtime_tracep->dump_count();
}

void write_trace_error(char* destination, std::size_t capacity, const char* message) {
    if (!destination || capacity == 0) return;
    const char* const source = message ? message : "Verilator trace host error";
    std::snprintf(destination, capacity, "%s", source);
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

extern "C" RUSTDV_EXPORT int rustdv_verilator_trace_control(
    std::uint32_t command,
    const char* path,
    RustdvTraceStatus* status,
    char* error,
    std::size_t error_capacity) {
    fill_trace_status(status);
    if (!runtime_tracep || !runtime_tracep->available()) {
        write_trace_error(error,
                          error_capacity,
                          "this Verilator model was not built with --trace-fst");
        return 1;
    }

    const char* message = nullptr;
    bool ok = false;
    int failure_code = 4;
    switch (command) {
    case 0:
        return 0;
    case 1:
        ok = runtime_tracep->start(path, true, message, failure_code);
        break;
    case 2:
        ok = runtime_tracep->stop(message);
        break;
    case 3:
        ok = runtime_tracep->flush(message);
        break;
    default:
        write_trace_error(error, error_capacity, "unknown Verilator trace command");
        return 2;
    }

    fill_trace_status(status);
    if (ok) return 0;
    write_trace_error(error, error_capacity, message);
    return command == 1 ? failure_code : 4;
}

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
    RuntimeTrace runtime_trace{*contextp, *dutp};
    runtime_tracep = &runtime_trace;

    if (const char* const path = std::getenv("RUSTDV_FST")) {
        const char* error = nullptr;
        int error_code = 4;
        if (!runtime_trace.start(path, false, error, error_code)) {
            std::fprintf(stderr, "rustdv: %s\n", error);
            return 2;
        }
    }

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

        runtime_trace.dump_settled_time();

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

    if (runtime_trace.active()) {
        const char* error = nullptr;
        if (!runtime_trace.stop(error)) {
            std::fprintf(stderr, "rustdv: failed to close FST capture: %s\n", error);
            scheduler_ok = false;
        }
    }
    runtime_tracep = nullptr;

#if VM_COVERAGE
    contextp->coveragep()->write();
#endif

    contextp->statsPrintSummary();
    return scheduler_ok ? 0 : 1;
}
