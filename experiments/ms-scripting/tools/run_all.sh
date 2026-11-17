#!/usr/bin/env bash
# MS measurement run (plan Task 6). Builds under a shared lock, then all timing and memory
# runs under one exclusive lock. Functional probes run under a shared lock.
# Usage: tools/run_all.sh OUT_DIR [RUNS]
set -uo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
cd "$here"
out="$(realpath -m "$1")"; runs="${2:-5}"
lock=/tmp/turts-quiet-performance.lock
mkdir -p "$out"/{native,chrome,probes,meta}

# 1. Builds (shared lock).
flock -s "$lock" cargo build --release -p ms_native >"$out/meta/build-native.log" 2>&1 || { echo "native build failed"; exit 1; }
flock -s "$lock" tools/build_wasm.sh >"$out/meta/build-wasm.log" 2>&1 || { echo "wasm build failed"; exit 1; }
cp out/wasm/sizes.json "$out/meta/sizes.json"
python3 tools/wasm_imports.py --self-test >"$out/meta/wasm_imports_selftest.txt"

# 2. Functional probes, 5 processes (native) and 5 launches (Chrome) for each candidate:
#    L5 (b) needs each run in a new process. Shared lock.
for n in $(seq 1 "$runs"); do
    flock -s "$lock" tools/native_probes.sh "$out/probes/native-run$n" none >"$out/probes/native-run$n.txt" 2>&1
    for k in rhai rune lua; do
        flock -s "$lock" timeout 1200 node tools/chrome_run.mjs "$k" probes "$k" "$n" "$out/probes/chrome-$k-run$n.json" 0 \
            >"$out/probes/chrome-$k-run$n.log" 2>&1
    done
done

# 3. Timing and memory (exclusive lock). Native pinned to CPU 4, Chrome to CPUs 4-7.
cat >"$out/meta/measure.sh" <<'INNER'
#!/usr/bin/env bash
set -uo pipefail
out="$1"; runs="$2"
for n in $(seq 1 "$runs"); do
    for k in none rust rhai rune lua; do
        echo "native $k run$n load $(cat /proc/loadavg)" >>"$out/meta/load.txt"
        taskset -c 4 ./target/release/ms_native suite --kind "$k" --run "$n" --out "$out/native/$k-run$n.json" \
            2>>"$out/meta/native-stderr.txt"
        echo "$k run$n exit $?" >>"$out/meta/native-exit.txt"
    done
    for k in rhai rune lua; do
        taskset -c 4 ./target/release/ms_native probe --kind "$k" --id L1 --out "$out/native/$k-L1-run$n.json"
    done
    for k in none rust rhai rune lua; do
        w="$k"; [ "$k" = none ] || [ "$k" = rust ] && w=baseline
        echo "chrome $k run$n load $(cat /proc/loadavg)" >>"$out/meta/load.txt"
        taskset -c 4-7 timeout 1200 node tools/chrome_run.mjs "$w" suite "$k" "$n" "$out/chrome/$k-run$n.json" 0 \
            >>"$out/meta/chrome-log.txt" 2>&1
        echo "$k run$n exit $?" >>"$out/meta/chrome-exit.txt"
    done
done
INNER
chmod +x "$out/meta/measure.sh"
date -u +%FT%TZ >"$out/meta/measure-start.txt"
flock -x "$lock" "$out/meta/measure.sh" "$out" "$runs"
# B9 on Chrome: L1 alone, under the exclusive lock.
flock -x "$lock" tools/chrome_l1.sh "$out" "$runs"
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
    echo "git: $(git -C "$here" rev-parse HEAD)"
} >"$out/meta/env.txt"
python3 tools/aggregate.py "$out"
