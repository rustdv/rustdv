#!/usr/bin/env bash
# Prove the public RustDV Verilator runner emits line/expression coverage.
set -euo pipefail
cd "$(dirname "$0")/../../../.."

BUILD="$(mktemp -d /tmp/rustdv-coverage-regression.XXXXXX)"
trap 'rm -rf "$BUILD"' EXIT
CONTROL="$PWD/output/regression/tests/sim-coverage-verilator/coverage.vlt"

COVERAGE_FILE="$BUILD/pass/coverage.dat"
SIM_BUILD_DIR="$BUILD/pass" \
RUSTDV_VERILATOR_MODE=coverage \
RUSTDV_COVERAGE_FILE="$COVERAGE_FILE" \
RUSTDV_VERILATOR_CONTROL_FILE="$CONTROL" \
    bash sim/run_rustdv.sh release verilator

if [ ! -s "$COVERAGE_FILE" ]; then
    echo "COVERAGE: FAIL — missing or empty $COVERAGE_FILE" >&2
    exit 1
fi
SUMMARY="$(verilator_coverage --report summary "$COVERAGE_FILE")"
if ! grep -Eq '^  line[[:space:]]+:.*\([[:space:]]*[1-9][0-9]*/' <<< "$SUMMARY"; then
    echo "COVERAGE: FAIL — database contains no line points" >&2
    exit 1
fi
if ! grep -Eq '^  expr[[:space:]]+:.*\([[:space:]]*[1-9][0-9]*/' <<< "$SUMMARY"; then
    echo "COVERAGE: FAIL — database contains no expression points" >&2
    exit 1
fi
if grep -q 'tinyalu.mult' <<< "$(verilator_coverage --report hier "$COVERAGE_FILE")"; then
    echo "COVERAGE: FAIL — bench control file did not exclude three_cycle" >&2
    exit 1
fi

echo "COVERAGE PASS PATH: PASS"

FINAL_BUILD="$BUILD/final"
FINAL_COVERAGE_FILE="$FINAL_BUILD/coverage.dat"
SIM=verilator \
RUSTDV_TESTCASE=clock_period_is_what_was_asked_for \
RUSTDV_VERILATOR_FRAMEWORK_VISIBILITY=1 \
SIM_BUILD_DIR="$FINAL_BUILD" \
RUSTDV_VERILATOR_MODE=coverage \
RUSTDV_COVERAGE_FILE="$FINAL_COVERAGE_FILE" \
    bash rustdv/framework-tests/run.sh

FINAL_ANNOTATED="$BUILD/final-annotated"
verilator_coverage --annotate "$FINAL_ANNOTATED" --annotate-all --annotate-points \
    "$FINAL_COVERAGE_FILE" >/dev/null
if ! grep -Eq '000001.*final \$display\("RTL FINAL: PASS"\)' \
    "$FINAL_ANNOTATED/probe.sv"; then
    echo "COVERAGE: FAIL — coverage was not written after the RTL final block" >&2
    exit 1
fi

echo "COVERAGE POST-FINAL ORDER: PASS"

FAIL_COVERAGE_FILE="$BUILD/failure/coverage.dat"
set +e
FAIL_OUTPUT="$(
    SIM_BUILD_DIR="$BUILD/pass" \
    RUSTDV_TESTCASE=coverage_deliberate_missing_test_ \
    RUSTDV_VERILATOR_MODE=coverage \
    RUSTDV_COVERAGE_FILE="$FAIL_COVERAGE_FILE" \
    RUSTDV_VERILATOR_CONTROL_FILE="$CONTROL" \
        bash sim/run_rustdv.sh release verilator 2>&1
)"
FAIL_STATUS=$?
set -e
if [ "$FAIL_STATUS" -eq 0 ]; then
    echo "COVERAGE: FAIL — deliberate RustDV regression failure returned success" >&2
    exit 1
fi
if ! grep -q "REGRESSION: FAIL" <<< "$FAIL_OUTPUT"; then
    echo "COVERAGE: FAIL — deliberate failure produced no RustDV verdict" >&2
    exit 1
fi
if [ ! -s "$FAIL_COVERAGE_FILE" ]; then
    echo "COVERAGE: FAIL — regression failure did not preserve coverage" >&2
    exit 1
fi

echo "COVERAGE FAILURE PATH: PASS"

UNINSTRUMENTED_FILE="$BUILD/fast/should-not-exist.dat"
set +e
UNINSTRUMENTED_OUTPUT="$(
    SIM_BUILD_DIR="$BUILD/fast" \
    RUSTDV_VERILATOR_MODE=fast \
    RUSTDV_COVERAGE_FILE="$UNINSTRUMENTED_FILE" \
        bash sim/run_rustdv.sh release verilator 2>&1
)"
UNINSTRUMENTED_STATUS=$?
set -e
if [ "$UNINSTRUMENTED_STATUS" -eq 0 ]; then
    echo "COVERAGE: FAIL — FAST silently accepted a coverage destination" >&2
    exit 1
fi
if ! grep -q "RUSTDV_COVERAGE_FILE requires RUSTDV_VERILATOR_MODE=coverage" \
    <<< "$UNINSTRUMENTED_OUTPUT"; then
    echo "COVERAGE: FAIL — FAST rejection was not explicit" >&2
    exit 1
fi
if [ -e "$UNINSTRUMENTED_FILE" ]; then
    echo "COVERAGE: FAIL — FAST created a coverage artifact" >&2
    exit 1
fi

echo "COVERAGE UNINSTRUMENTED REJECTION: PASS"

FAST_BUILD="$BUILD/fast-build"
SIM_BUILD_DIR="$FAST_BUILD" \
RUSTDV_TESTCASE=MaxTest \
RUSTDV_VERILATOR_MODE=fast \
    bash sim/run_rustdv.sh release verilator
FAST_CLASSES_MK="$FAST_BUILD/verilator/obj_dir/Vrustdv_dut_classes.mk"
if [ ! -f "$FAST_CLASSES_MK" ]; then
    echo "COVERAGE: FAIL — FAST build produced no Verilator class manifest" >&2
    exit 1
fi
if ! grep -Eq '^VM_COVERAGE[[:space:]]*=[[:space:]]*0$' "$FAST_CLASSES_MK"; then
    echo "COVERAGE: FAIL — FAST build has coverage instrumentation" >&2
    exit 1
fi

echo "COVERAGE FAST UNINSTRUMENTED: PASS"

INVALID_DESTINATION="$BUILD/destination-is-directory"
mkdir -p "$INVALID_DESTINATION"
set +e
INVALID_OUTPUT="$(
    SIM_BUILD_DIR="$BUILD/invalid-output" \
    RUSTDV_VERILATOR_MODE=coverage \
    RUSTDV_COVERAGE_FILE="$INVALID_DESTINATION" \
        bash sim/run_rustdv.sh release verilator 2>&1
)"
INVALID_STATUS=$?
set -e
if [ "$INVALID_STATUS" -eq 0 ]; then
    echo "COVERAGE: FAIL — directory destination returned success" >&2
    exit 1
fi
if ! grep -q "coverage output path is a directory" <<< "$INVALID_OUTPUT"; then
    echo "COVERAGE: FAIL — invalid destination error was not explicit" >&2
    exit 1
fi

echo "COVERAGE DESTINATION REJECTION: PASS"

LOCKED_DIR="$BUILD/locked"
mkdir -p "$LOCKED_DIR"
chmod 500 "$LOCKED_DIR"
set +e
UNWRITABLE_OUTPUT="$(
    SIM_BUILD_DIR="$BUILD/pass" \
    RUSTDV_VERILATOR_MODE=coverage \
    RUSTDV_COVERAGE_FILE="$LOCKED_DIR/coverage.dat" \
    RUSTDV_VERILATOR_CONTROL_FILE="$CONTROL" \
        bash sim/run_rustdv.sh release verilator 2>&1
)"
UNWRITABLE_STATUS=$?
set -e
chmod 700 "$LOCKED_DIR"
if [ "$UNWRITABLE_STATUS" -eq 0 ]; then
    echo "COVERAGE: FAIL — unwritable destination returned success" >&2
    exit 1
fi
if ! grep -q "Can't write" <<< "$UNWRITABLE_OUTPUT"; then
    echo "COVERAGE: FAIL — unwritable destination error was not explicit" >&2
    exit 1
fi

echo "COVERAGE UNWRITABLE REJECTION: PASS"
