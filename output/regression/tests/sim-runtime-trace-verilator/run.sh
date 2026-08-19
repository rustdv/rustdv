#!/usr/bin/env bash
# Prove runtime FST capture is unavailable without instrumentation and precisely gated in a
# trace-capable Verilator model.
set -euo pipefail
cd "$(dirname "$0")/../../../.."

BUILD="$(mktemp -d /tmp/rustdv-runtime-trace-regression.XXXXXX)"
trap 'rm -rf "$BUILD"' EXIT

SIM=verilator \
RUSTDV_TESTCASE=runtime_trace_uninstrumented_ \
RUSTDV_VERIFY_NO_RUNTIME_TRACE=1 \
RUSTDV_VERILATOR_MODE=framework \
SIM_BUILD_DIR="$BUILD/fast" \
    bash rustdv/framework-tests/run.sh

SIM=verilator \
RUSTDV_TESTCASE=runtime_trace_capture_ \
RUSTDV_VERIFY_RUNTIME_TRACE=1 \
RUSTDV_VERILATOR_MODE=record \
RUSTDV_VERILATOR_FRAMEWORK_VISIBILITY=1 \
SIM_BUILD_DIR="$BUILD/record" \
    bash rustdv/framework-tests/run.sh
