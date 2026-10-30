#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
(($# > 0)) || { echo "usage: scripts/mc-pixels.sh tests/browser/rows/<ROW>.json ..." >&2; exit 2; }
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
            headed=0
            [[ "$runs" != headed ]] || headed=1
            if ! MC_BROWSER_HEADED="$headed" bash "$ROOT/scripts/mc-browser.sh" "$package" "$driver" "$oracle" "$viewport" --params "$row_path"; then passed=false; status=1; fi
        done
    done
    if [[ "$passed" == true ]]; then echo "PASS $row (2 runs per viewport)"; else echo "FAIL $row"; fi
done
exit "$status"
