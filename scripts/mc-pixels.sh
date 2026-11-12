#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
usage() { echo "usage: scripts/mc-pixels.sh [--prepared DIR] tests/browser/rows/<ROW>.json ...   |   scripts/mc-pixels.sh --prepare DIR tests/browser/rows/<ROW>.json" >&2; exit 2; }
# Q-PREBUILD: --prepare DIR builds and packs the one row (outside the GPU lock) and stops. --prepared DIR
# runs the rows with no build; it refuses (exit 66) when DIR does not match the source (scripts/mc_prepare.py).
PREPARED_ARGS=()
if [[ "${1:-}" == --prepare ]]; then
    (($# == 3)) || usage
    row_path="$ROOT/$3"
    [[ -f "$row_path" ]] || { echo "missing row file: $3" >&2; exit 1; }
    package="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["package"])' "$row_path")"
    exec bash "$ROOT/scripts/mc-browser.sh" "$package" --prepare "$2" --params "$row_path"
fi
if [[ "${1:-}" == --prepared ]]; then
    (($# >= 3)) || usage
    PREPARED_ARGS=(--prepared "$2")
    shift 2
fi
(($# > 0)) || usage
status=0
for row_file in "$@"; do
    row_path="$ROOT/$row_file"
    [[ -f "$row_path" ]] || { echo "missing row file: $row_file" >&2; status=1; continue; }
    mapfile -t fields < <(python3 - "$row_path" <<'PYROW'
import json,sys
r=json.load(open(sys.argv[1]))
print(r["row"]); print(r["package"]); print(r["driver"]); print(r["oracle"]); print(r.get("windows",1)); print(r.get("runs","ci")); print("\n".join(r["viewports"]))
PYROW
)
    row="${fields[0]}" package="${fields[1]}" driver="${fields[2]}" oracle="${fields[3]}" windows="${fields[4]}" runs="${fields[5]}"
    if [[ "$runs" == headed && ( "${MC_PIXELS_HEADED:-0}" != 1 || -z "${DISPLAY:-}" ) ]]; then echo "SKIP $row headed"; continue; fi
    passed=true
    for viewport in "${fields[@]:6}"; do
        for attempt in 1 2; do
            echo "ROW $row viewport=$viewport run=$attempt/2 windows=$windows"
            headed="${MC_BROWSER_HEADED:-0}"
            [[ "$runs" != headed ]] || headed=1
            [[ "$headed" == 0 || "$headed" == 1 ]] || { echo "MC_BROWSER_HEADED must be 0 or 1" >&2; exit 2; }
            echo "BROWSER_PROFILE headed=$headed"
            rc=0
            MC_BROWSER_HEADED="$headed" bash "$ROOT/scripts/mc-browser.sh" "$package" "$driver" "$oracle" "$viewport" --params "$row_path" "${PREPARED_ARGS[@]}" || rc=$?
            # 66: the prepared folder is stale. No later run can pass, so stop with that exit code.
            if ((rc == 66 && ${#PREPARED_ARGS[@]} > 0)); then echo "STALE $row: prepared folder refused; no run was made" >&2; exit 66; fi
            if ((rc != 0)); then passed=false; status=1; fi
        done
    done
    if [[ "$passed" == true ]]; then echo "PASS $row (2 runs per viewport)"; else echo "FAIL $row"; fi
done
exit "$status"
