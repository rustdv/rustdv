#!/usr/bin/env bash
# Prove the scoreboards can fail.
#
# Every testbench in the book ends "REGRESSION: PASS", and that sentence is
# only worth anything if the same testbench says FAIL when the design is
# wrong. A scoreboard with a comparison that can never be false passes every
# regression there is, forever, and looks exactly like a working one.
#
# So: break the TinyALU's XOR into an OR, run two testbenches that check
# results against a prediction, and require both to fail. Then run them once
# more against the real RTL and require both to pass — otherwise a testbench
# that failed for some unrelated reason would count as "the mutation was
# caught".
#
# Success criterion: prints "MUTATION: CAUGHT".
set -uo pipefail
cd "$(dirname "$0")/../../../.."       # repo root
SIM="${SIM:-icarus}"
# run_sim.sh cds to output/examples, so every path handed to it must be
# absolute — a relative one would be resolved against the wrong directory.
EXAMPLES="$PWD/output/examples"
TIMESCALE="$EXAMPLES/sim-common/hdl/timescale.v"

RUSTDV_TMP="/tmp/rustdv-$(id -u)"
WORK="$RUSTDV_TMP/mutation-$SIM"
mkdir -p "$WORK"

GOOD="$EXAMPLES/sim-common/hdl/tinyalu.sv"
BAD="$WORK/tinyalu_mutated.sv"

# The mutation: XOR becomes OR on the 3'b011 opcode. One character, chosen
# because `a ^ b` and `a | b` agree whenever the operands share no bits — so
# a testbench driving only small or sparse operands would not notice, and
# this is a check on the stimulus as much as on the scoreboard.
sed 's|3.b011 : result <= {8.d0,A} \^ {8.d0,B};|3'"'"'b011 : result <= {8'"'"'d0,A} \| {8'"'"'d0,B};|' \
    "$GOOD" > "$BAD"

if diff -q "$GOOD" "$BAD" > /dev/null; then
    echo "MUTATION: NOT APPLIED — the XOR line in $GOOD no longer matches the pattern in this script" >&2
    exit 1
fi

# Two testbenches, one from each half of the book: the last structural one
# and the last sequence one. A mutation caught by only one of them would say
# something about that testbench; caught by both, it says something about the
# scoreboard design they share.
CASES=(
    "ch34_connections_testbench_6_0"
    "ch39_virtual_sequence_testbench_8_0"
)

status=0
for crate in "${CASES[@]}"; do
    # Separate build roots so the mutated .vvp never overwrites the good one
    # that the sim-ch* entries run.
    out_bad=$(SIM="$SIM" SIM_BUILD_DIR="$WORK/bad" bash "$EXAMPLES/sim-common/run_sim.sh" \
        "$crate" tinyalu "$TIMESCALE" "$BAD" 2>&1)
    bad_status=$?
    if grep -q "REGRESSION: FAIL" <<< "$out_bad"; then
        if [ "$SIM" = verilator ] && [ "$bad_status" -eq 0 ]; then
            echo "  FAIL $crate printed FAIL but the Verilator wrapper returned zero" >&2
            status=1
        else
            echo "  ok   $crate failed against the mutated RTL"
        fi
    elif ! grep -q "REGRESSION:" <<< "$out_bad"; then
        # No verdict at all: the build or the simulator fell over, and
        # "it did not pass" would be the wrong thing to conclude.
        echo "  FAIL $crate produced no verdict against the mutated RTL" >&2
        echo "$out_bad" | tail -20 >&2
        status=1
    else
        echo "  FAIL $crate passed against a broken XOR — its scoreboard cannot fail" >&2
        status=1
    fi

    # The control. Without it, a testbench that had stopped compiling would
    # "catch" every mutation.
    out_good=$(SIM="$SIM" SIM_BUILD_DIR="$WORK/good" bash "$EXAMPLES/sim-common/run_sim.sh" \
        "$crate" tinyalu "$TIMESCALE" "$GOOD" 2>&1)
    good_status=$?
    if grep -q "REGRESSION: PASS" <<< "$out_good" && [ "$good_status" -eq 0 ]; then
        echo "  ok   $crate passed against the real RTL"
    else
        echo "  FAIL $crate did not pass against the real RTL — the run above proved nothing" >&2
        echo "$out_good" | tail -20 >&2
        status=1
    fi
done

if [ "$status" -eq 0 ]; then
    echo "MUTATION: CAUGHT"
else
    echo "MUTATION: MISSED"
fi
exit "$status"
