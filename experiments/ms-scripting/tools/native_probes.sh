#!/usr/bin/env bash
# Run every probe for each candidate natively, one process for each probe.
# Usage: tools/native_probes.sh OUT_DIR [MUTANT] [KINDS...]
# The caller holds the CPU lock (a shared lock is enough: probes are functional; L1 stop time
# is acceptance evidence only from the exclusive-lock run in tools/run_all.sh).
set -uo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
out="$1"; mutant="${2:-none}"; shift 2 || shift $#
kinds=("$@"); [ ${#kinds[@]} -eq 0 ] && kinds=(rhai rune lua)
mkdir -p "$out"
# Protect the shared machine: a runaway allocation fails at 8 GiB instead of waking the OOM killer.
ulimit -v 8388608
ids=(L1 L2 L3a L3b L3c-map L3c-list L3d L3e L4 L5b L7a L7b L8 L9a L9b)
for k in "${kinds[@]}"; do
    for id in "${ids[@]}"; do
        base="$out/$k-$id"
        timeout 300 "$here/target/release/ms_native" probe --kind "$k" --id "$id" --mutant "$mutant" \
            --out "$base.json" >"$base.stdout" 2>"$base.stderr"
        echo "$?" >"$base.exit"
    done
done
python3 "$here/tools/probe_table.py" "$out"
