#!/usr/bin/env bash
# Build a chapter's sim-figure crate as a VPI module and run it on a simulator.
# Usage: sim-common/run_sim.sh <crate-name> <top-module> [hdl files...]
#
# Part II+ figures run inside a simulator; each sim chapter is a cdylib
# crate whose #[rustdv::test] functions are the chapter's figures.
set -euo pipefail
cd "$(dirname "$0")/.."
CRATE="$1"; TOP="$2"; shift 2
SIM="${SIM:-icarus}"
HDL=("$@"); [ ${#HDL[@]} -eq 0 ] && HDL=(sim-common/hdl/timescale.v "sim-common/hdl/${TOP}.sv")
REPO_ROOT="$(cd ../.. && pwd)"
# Everything scratch lives under one per-user root, /tmp/rustdv-<uid>/.
# A bare /tmp/rustdv-... path is shared by every account on the machine, so a
# directory left by someone else makes the build unwritable and the failure
# reads as a broken testbench. One root also means one cleanup:
#     rm -rf /tmp/rustdv-$(id -u)
# An explicit CARGO_TARGET_DIR or SIM_BUILD_DIR in the environment still wins.
RUSTDV_TMP="/tmp/rustdv-$(id -u)"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$RUSTDV_TMP/examples-target}"
cargo build --release -p "$CRATE" --quiet
BUILD="${SIM_BUILD_DIR:-$RUSTDV_TMP/sim}"; mkdir -p "$BUILD"
# Linux builds lib<crate>.so; macOS builds lib<crate>.dylib
LIB="$CARGO_TARGET_DIR/release/lib${CRATE}.so"
[ -f "$LIB" ] || LIB="$CARGO_TARGET_DIR/release/lib${CRATE}.dylib"
export RUSTDV_RANDOM_SEED="${RUSTDV_RANDOM_SEED:-1}"

case "$SIM" in
  icarus)
    cp "$LIB" "$BUILD/${CRATE}.vpi"
    iverilog -g2012 -o "$BUILD/${CRATE}.vvp" -s "$TOP" "${HDL[@]}"
    vvp -M "$BUILD" -m "$CRATE" "$BUILD/${CRATE}.vvp"
    ;;
  verilator)
    "$REPO_ROOT/sim/run_verilator.sh" "$LIB" "$TOP" "$BUILD/${CRATE}-verilator" "${HDL[@]}"
    ;;
  *)
    echo "unknown simulator: $SIM (use icarus|verilator)" >&2
    exit 2
    ;;
esac
