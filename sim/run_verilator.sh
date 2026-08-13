#!/usr/bin/env bash
# Build one rustdv VPI testbench against a Verilated DUT and run it.
#
# Usage: run_verilator.sh <vpi-library> <top-module> <build-dir> <hdl...>
#
# RUSTDV_VERILATOR_MODE:
#   fast       top-level DUT ports only (default)
#   debug      selected internals from RUSTDV_VERILATOR_CONTROL_FILE + FST
#   framework  all signals visible; regression probes only
set -euo pipefail

if [ "$#" -lt 4 ]; then
    echo "usage: $0 <vpi-library> <top-module> <build-dir> <hdl...>" >&2
    exit 2
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
LIB="$1"
TOP="$2"
BUILD="$3"
shift 3
HDL=("$@")
MODE="${RUSTDV_VERILATOR_MODE:-fast}"
THREADS="${RUSTDV_VERILATOR_THREADS:-1}"

case "$THREADS" in
    1|2|4) ;;
    *) echo "rustdv: RUSTDV_VERILATOR_THREADS must be 1, 2, or 4" >&2; exit 2 ;;
esac

case "${RUSTDV_VERILATOR_OPT:-default}" in
    default) OPT_FAST="-Os" ;;
    o3) OPT_FAST="-O3" ;;
    *) echo "rustdv: RUSTDV_VERILATOR_OPT must be default or o3" >&2; exit 2 ;;
esac
if [ "${RUSTDV_VERILATOR_NATIVE:-0}" = 1 ]; then
    OPT_FAST="$OPT_FAST -march=native"
fi

if [[ ! "$TOP" =~ ^[A-Za-z_][A-Za-z0-9_\$]*$ ]]; then
    echo "rustdv: unsupported Verilator top-module name: $TOP" >&2
    exit 2
fi

if [ ! -f "$LIB" ]; then
    echo "rustdv: VPI library not found: $LIB" >&2
    exit 2
fi

version="$(verilator --version | awk '{print $2}')"
if [[ ! "$version" =~ ^([0-9]+)\.([0-9]+) ]]; then
    echo "rustdv: cannot parse Verilator version: $version" >&2
    exit 2
fi
major=$((10#${BASH_REMATCH[1]}))
minor=$((10#${BASH_REMATCH[2]}))
if (( major < 5 || (major == 5 && minor < 50) )); then
    echo "rustdv: Verilator >= 5.050 is required for runtime VPI loading (found $version)" >&2
    exit 2
fi

mkdir -p "$BUILD"
OBJ="$BUILD/obj_dir"
EXE="$OBJ/rustdv_sim"
LOG="$BUILD/verilator.log"

FLAGS=(
    --cc --exe
    -sv --timing --vpi
    --threads "$THREADS"
    -CFLAGS "$OPT_FAST"
    --prefix Vrustdv_dut
    --top-module "$TOP"
    --Mdir "$OBJ"
    -o rustdv_sim
)

INPUTS=()
case "$MODE" in
    fast)
        if [ -n "${RUSTDV_FST:-}" ]; then
            echo "rustdv: FST tracing belongs to debug mode (set RUSTDV_VERILATOR_MODE=debug)" >&2
            exit 2
        fi
        ;;
    debug)
        CONTROL="${RUSTDV_VERILATOR_CONTROL_FILE:-}"
        if [ -z "$CONTROL" ] || [ ! -f "$CONTROL" ]; then
            echo "rustdv: debug mode requires RUSTDV_VERILATOR_CONTROL_FILE=<file.vlt>" >&2
            exit 2
        fi
        INPUTS+=("$CONTROL")
        FLAGS+=(--trace-fst)
        if command -v pkg-config >/dev/null 2>&1 && pkg-config --exists liblz4; then
            LZ4_CFLAGS="$(pkg-config --cflags liblz4)"
            LZ4_LIBS="$(pkg-config --libs liblz4)"
            # pkg-config omits system include directories on Linux, so
            # --cflags may legitimately be empty. Passing `-CFLAGS ""` makes
            # Verilator consume the following option incorrectly.
            if [ -n "$LZ4_CFLAGS" ]; then
                FLAGS+=(-CFLAGS "$LZ4_CFLAGS")
            fi
            if [ -n "$LZ4_LIBS" ]; then
                FLAGS+=(-LDFLAGS "$LZ4_LIBS")
            fi
        fi
        export RUSTDV_FST="${RUSTDV_FST:-$BUILD/${TOP}.fst}"
        ;;
    framework)
        FLAGS+=(--public-flat-rw)
        ;;
    *)
        echo "rustdv: unknown RUSTDV_VERILATOR_MODE '$MODE' (use fast|debug|framework)" >&2
        exit 2
        ;;
esac

# C++ top-level ports are public by default, but Verilator's VPI namespace is
# generated only for objects marked public_flat_rw.  Mark just the selected
# DUT module's ports; this is deliberately narrower than --public-flat-rw.
PORT_CONTROL="$BUILD/rustdv-top-ports.vlt"
printf '%s\n' \
    '`verilator_config' \
    "public_flat_rw -module \"$TOP\" -port \"*\"" \
    > "$PORT_CONTROL"
if [ "${#INPUTS[@]}" -gt 0 ]; then
    INPUTS=("$PORT_CONTROL" "${INPUTS[@]}")
else
    INPUTS=("$PORT_CONTROL")
fi
INPUTS+=("${HDL[@]}")
verilator "${FLAGS[@]}" "${INPUTS[@]}" "$SCRIPT_DIR/verilator_main.cpp"
make -C "$OBJ" -f Vrustdv_dut.mk -j "${RUSTDV_VERILATOR_JOBS:-1}" \
    OPT_FAST="$OPT_FAST" OPT_GLOBAL="$OPT_FAST" OPT_SLOW="-O0"

set +e
"$EXE" "+verilator+vpi+$LIB" 2>&1 | tee "$LOG"
sim_status=${PIPESTATUS[0]}
set -e

if [ "$sim_status" -ne 0 ]; then
    echo "rustdv: Verilator host exited $sim_status" >&2
    exit "$sim_status"
fi
if grep -q "REGRESSION: FAIL" "$LOG"; then
    echo "rustdv: regression reported failure" >&2
    exit 1
fi
if ! grep -q "REGRESSION: PASS" "$LOG"; then
    echo "rustdv: simulator exited without a regression verdict" >&2
    exit 1
fi
