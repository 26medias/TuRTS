#!/usr/bin/env bash
# Probe neutrality (pre-MP1 slice, task Q1b review follow-up, finding 2).
#
# The cargo feature `test-probes` of `turts_sim` must change no simulation result (design 9.20).
# Since Q1b, `cargo test --workspace` builds `turts_sim` with the feature on, so no workspace run
# tests the build without it. This script runs the test `crates/turts_sim/tests/frame_hash.rs`
# twice, with the feature off and with `--features test-probes`, takes the hash lines that the
# test prints (`frame_chains=...` and `muster_world=...`, one line each), and compares them.
#
# Exit 0: both runs passed, each printed both lines, and the lines are byte-equal.
# Exit 1: a run failed, a hash line is missing, or the two sets of lines differ.
# Run it from any directory. It needs a build of `turts_sim` with and without the feature.
#
# The values are not pinned in the test: they use the libm of the platform (`cos` and `sin` in the
# fixture, `atan2` in `movement/mod.rs` and `spread.rs`), and only one machine was run. The
# comparison needs no pin: both runs happen on the same machine.
set -euo pipefail
cd "$(dirname "$0")/../.."
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
failed=0

# run LABEL [CARGO ARGS...]: run the frame_hash test and keep its hash lines in $TMP/LABEL.hash.
run() {
    local label=$1
    shift
    if ! cargo test -p turts_sim --locked "$@" --test frame_hash -- --nocapture --test-threads=1 \
        >"$TMP/$label.log" 2>&1; then
        echo "probe_neutral: the frame_hash test failed (feature $label)" >&2
        tail -40 "$TMP/$label.log" >&2
        failed=1
    fi
    # grep exits 1 for no match; the count check below turns that into a clear message.
    grep -E '^(frame_chains|muster_world)=' "$TMP/$label.log" | sort >"$TMP/$label.hash" || true
    local count
    count=$(wc -l <"$TMP/$label.hash")
    if [ "$count" -ne 2 ]; then
        echo "probe_neutral: feature $label printed $count hash lines, expected 2" >&2
        failed=1
    fi
}

run off
run on --features test-probes

if ! cmp -s "$TMP/off.hash" "$TMP/on.hash"; then
    echo "probe_neutral: the feature test-probes changes a simulation result" >&2
    echo "--- feature off" >&2
    cat "$TMP/off.hash" >&2
    echo "--- feature on" >&2
    cat "$TMP/on.hash" >&2
    failed=1
fi
if [ "$failed" -ne 0 ]; then
    exit 1
fi
echo "probe_neutral: feature off equals feature on"
cat "$TMP/off.hash"
