#!/usr/bin/env bash
# Prove DEBUG mode accepts selected visibility and emits a non-empty FST.
set -euo pipefail
cd "$(dirname "$0")/../../../.."

BUILD="/tmp/rustdv-$(id -u)/verilator-debug-regression"
SIM_BUILD_DIR="$BUILD" RUSTDV_VERILATOR_MODE=debug \
    bash sim/run_rustdv.sh release verilator

FST="$BUILD/verilator/tinyalu.fst"
if [ ! -s "$FST" ]; then
    echo "FST: FAIL — missing or empty $FST" >&2
    exit 1
fi
echo "FST: PASS ($FST)"

INVALID_FST="$BUILD/missing-parent/capture.fst"
rm -rf "$(dirname "$INVALID_FST")"
set +e
SIM_BUILD_DIR="$BUILD" RUSTDV_VERILATOR_MODE=debug RUSTDV_FST="$INVALID_FST" \
    bash sim/run_rustdv.sh release verilator
INVALID_STATUS=$?
set -e
if [ "$INVALID_STATUS" -eq 0 ] || [ -e "$INVALID_FST" ]; then
    echo "FST OPEN FAILURE: FAIL — invalid destination was accepted" >&2
    exit 1
fi
echo "FST OPEN FAILURE: PASS"
