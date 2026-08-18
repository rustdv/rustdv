#!/usr/bin/env bash
# Prove INSPECT mode exposes selected VPI state without building or emitting a waveform.
set -euo pipefail
cd "$(dirname "$0")/../../../.."

BUILD="$(mktemp -d /tmp/rustdv-inspect-regression.XXXXXX)"
SIM_BUILD_DIR="$BUILD" RUSTDV_VERILATOR_MODE=inspect \
    bash sim/run_rustdv.sh release verilator

FST="$(find "$BUILD" -type f -name '*.fst' -print -quit)"
if [ -n "$FST" ]; then
    echo "INSPECT NO FST: FAIL — emitted $FST" >&2
    exit 1
fi
echo "INSPECT NO FST: PASS"

# The mode is deliberately no-waveform: reject an FST request before doing a
# second Verilator build. The successful run above has already built the VPI
# library in the shared cargo target directory.
TARGET_ROOT="${CARGO_TARGET_DIR:-/tmp/rustdv-$(id -u)/target}"
LIB="$TARGET_ROOT/release/libtinyalu_tb.so"
[ -f "$LIB" ] || LIB="$TARGET_ROOT/release/libtinyalu_tb.dylib"
set +e
REJECT_OUTPUT="$({
    RUSTDV_VERILATOR_MODE=inspect \
    RUSTDV_VERILATOR_CONTROL_FILE="$PWD/sim/verilator-debug.vlt" \
    RUSTDV_FST="$BUILD/unwanted.fst" \
        bash sim/run_verilator.sh "$LIB" tinyalu "$BUILD/reject-fst" \
        sim/hdl/timescale.v sim/hdl/tinyalu.sv
} 2>&1)"
REJECT_STATUS=$?
set -e
if [ "$REJECT_STATUS" -ne 2 ] || [[ "$REJECT_OUTPUT" != *"FST tracing belongs to debug mode"* ]]; then
    echo "INSPECT FST REJECTION: FAIL — status=$REJECT_STATUS" >&2
    echo "$REJECT_OUTPUT" >&2
    exit 1
fi
echo "INSPECT FST REJECTION: PASS"
