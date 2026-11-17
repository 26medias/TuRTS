#!/usr/bin/env bash
# B9 on Chrome: L1 alone, 5 fresh launches for each candidate. The caller holds the
# exclusive CPU lock. Usage: tools/chrome_l1.sh OUT_DIR [RUNS]
set -uo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
cd "$here"
out="$1"; runs="${2:-5}"
mkdir -p "$out/chrome-l1"
for n in $(seq 1 "$runs"); do
    for k in rhai rune lua; do
        echo "chrome-l1 $k run$n load $(cat /proc/loadavg)" >>"$out/meta/load.txt"
        taskset -c 4-7 timeout 600 node tools/chrome_run.mjs "$k" probes "$k" "$n" "$out/chrome-l1/$k-run$n.json" 0 L1 \
            >>"$out/meta/chrome-l1-log.txt" 2>&1
    done
done
