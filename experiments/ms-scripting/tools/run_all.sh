#!/usr/bin/env bash
# MS measurement run (plan Task 6). Builds under a shared lock, functional probes under a shared
# lock, then all timing and memory runs under one exclusive lock, then the aggregation.
# Usage: tools/run_all.sh OUT_DIR [RUNS]
#
# Entries: name -> (native binary, wasm module, kind). The three candidates and the two kinds
# without a script are the measured set. `rune-full` (Rune with the broad module set of the first
# run) is recorded only, not part of the selection (spec 9, amendment 5).
set -uo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
cd "$here"
out="$(realpath -m "$1")"; runs="${2:-5}"
lock=/tmp/turts-quiet-performance.lock
mkdir -p "$out"/{native,chrome,chrome-l1,probes,meta}

NAMES=(none rust rhai rune lua rune-full)
SCRIPTED=(rhai rune lua rune-full)
bin_of()  { case "$1" in none|rust) echo baseline ;; *) echo "$1" ;; esac; }
kind_of() { case "$1" in rune-full) echo rune ;; *) echo "$1" ;; esac; }
mut_native() { echo none; }
mut_code() { echo 0; }

# 1. Builds (shared lock).
flock -s "$lock" tools/build_native.sh >"$out/meta/build-native.log" 2>&1 || { echo "native build failed"; exit 1; }
flock -s "$lock" tools/build_wasm.sh >"$out/meta/build-wasm.log" 2>&1 || { echo "wasm build failed"; exit 1; }
cp out/wasm/sizes.json "$out/meta/sizes.json"
python3 tools/wasm_imports.py --self-test >"$out/meta/wasm_imports_selftest.txt"
for k in rhai rune lua; do out/bin/ms_native-$k calibrate --kind $k --out "$out/meta/calibrate-$k.json"; done

# 2. Functional probes: 5 processes (native) and 5 launches (Chrome) for each entry. Each run
#    is a new process, as L5 (b) needs. Shared lock.
for n in $(seq 1 "$runs"); do
    flock -s "$lock" tools/native_probes.sh "$out/probes/native-run$n" none rhai rune lua rune-full \
        >"$out/probes/native-run$n.txt" 2>&1
    for e in "${SCRIPTED[@]}"; do
        flock -s "$lock" timeout 1200 node tools/chrome_run.mjs "$(bin_of "$e")" probes "$(kind_of "$e")" "$n" \
            "$out/probes/chrome-$e-run$n.json" "$(mut_code "$e")" >"$out/probes/chrome-$e-run$n.log" 2>&1
    done
done

# 3. Timing and memory (exclusive lock). Native pinned to CPU 4, Chrome to CPUs 4-7.
cat >"$out/meta/measure.sh" <<'INNER'
#!/usr/bin/env bash
set -uo pipefail
out="$1"; runs="$2"
NAMES=(none rust rhai rune lua rune-full)
SCRIPTED=(rhai rune lua rune-full)
bin_of()  { case "$1" in none|rust) echo baseline ;; *) echo "$1" ;; esac; }
kind_of() { case "$1" in rune-full) echo rune ;; *) echo "$1" ;; esac; }
mut_native() { echo none; }
mut_code() { echo 0; }
for n in $(seq 1 "$runs"); do
    for e in "${NAMES[@]}"; do
        echo "native $e run$n load $(cat /proc/loadavg)" >>"$out/meta/load.txt"
        taskset -c 4 "./out/bin/ms_native-$(bin_of "$e")" suite --kind "$(kind_of "$e")" --run "$n" \
            --mutant "$(mut_native "$e")" --out "$out/native/$e-run$n.json" 2>>"$out/meta/native-stderr.txt"
        echo "$e run$n exit $?" >>"$out/meta/native-exit.txt"
    done
    for e in "${SCRIPTED[@]}"; do
        taskset -c 4 "./out/bin/ms_native-$(bin_of "$e")" probe --kind "$(kind_of "$e")" --id L1 \
            --mutant "$(mut_native "$e")" --out "$out/native/$e-L1-run$n.json"
    done
    for e in "${NAMES[@]}"; do
        echo "chrome $e run$n load $(cat /proc/loadavg)" >>"$out/meta/load.txt"
        taskset -c 4-7 timeout 1200 node tools/chrome_run.mjs "$(bin_of "$e")" suite "$(kind_of "$e")" "$n" \
            "$out/chrome/$e-run$n.json" "$(mut_code "$e")" >>"$out/meta/chrome-log.txt" 2>&1
        echo "$e run$n exit $?" >>"$out/meta/chrome-exit.txt"
    done
    for e in "${SCRIPTED[@]}"; do
        echo "chrome-l1 $e run$n load $(cat /proc/loadavg)" >>"$out/meta/load.txt"
        taskset -c 4-7 timeout 600 node tools/chrome_run.mjs "$(bin_of "$e")" probes "$(kind_of "$e")" "$n" \
            "$out/chrome-l1/$e-run$n.json" "$(mut_code "$e")" L1 >>"$out/meta/chrome-l1-log.txt" 2>&1
    done
done
INNER
chmod +x "$out/meta/measure.sh"
date -u +%FT%TZ >"$out/meta/measure-start.txt"
flock -x "$lock" "$out/meta/measure.sh" "$out" "$runs"
date -u +%FT%TZ >"$out/meta/measure-end.txt"

# 4. Environment record.
{
    echo "date: $(date -u +%FT%TZ)"
    echo "host: $(lscpu | grep 'Model name' | sed 's/ \+/ /g')"
    echo "cpus: $(nproc); memory: $(free -g | awk '/Mem:/ {print $2}') GiB"
    echo "os: $(grep PRETTY_NAME /etc/os-release | cut -d= -f2) kernel $(uname -r)"
    echo "rustc: $(rustc --version)"
    echo "wasm-opt: $(wasm-opt --version)"
    echo "chrome: $(google-chrome --version)"
    echo "node: $(node --version)"
    echo "git: $(git -C "$here" rev-parse HEAD) dirty-files: $(git -C "$here" status --short | wc -l)"
} >"$out/meta/env.txt"
python3 tools/licenses.py "$out" >"$out/meta/licenses.txt" 2>&1
python3 tools/aggregate.py "$out" >/dev/null
echo "summary: $out/summary.md"
