#!/usr/bin/env bash
# E-GPU (MP1 task E-GPU): two browser windows of one match on one GPU. See tests/browser/egpu-run.js.
#
#   scripts/mp1-egpu.sh --prepare DIR              build the server, the package and the wasm client into DIR (no GPU lock)
#   scripts/mp1-egpu.sh --prepared DIR [--secs N]  serve from DIR and run the two windows for N seconds (default 600)
#
# --prepared builds nothing: scripts/mc-browser.sh --serve-only --prepared refuses (exit 66) when DIR does not match the
# source (docs/qa/prepared-runs.md). Run the second form under mc_headed_v6.sh (the GPU lock) with MC_JOBS 1 to 4.
# Without MC_BROWSER_HEADED=1 the browser is headless (a smoke run with software rendering needs MC_CHROME_ARGS_JSON).
# Output: <evidence>/egpu-result.json, the frame time files, nvidia-smi.csv (1 sample each second, when nvidia-smi exists),
# server.log and server-tick.log. Exit: 0 PASS, 1 FAIL, 3 no verdict, 66 stale prepared folder.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PACKAGE=mods/muster
usage() { echo "usage: scripts/mp1-egpu.sh --prepare DIR | --prepared DIR [--secs N]" >&2; exit 2; }
(($# >= 2)) || usage
mode="$1" dir="$2"
shift 2
if [[ "$mode" == --prepare ]]; then
    (($# == 0)) || usage
    exec bash "$ROOT/scripts/mc-browser.sh" "$PACKAGE" --prepare "$dir"
fi
[[ "$mode" == --prepared ]] || usage
SECS=600
while (($#)); do
    case "$1" in
        --secs) (($# >= 2)) || usage; SECS="$2"; shift 2 ;;
        *) usage ;;
    esac
done
[[ "$SECS" =~ ^[0-9]+$ ]] || usage

SERVE_LOG="$(mktemp "${TMPDIR:-/tmp}/mp1-egpu-serve.XXXXXX")"
SERVE_PID="" SMI_PID=""
cleanup() {
    local rc=$?
    trap - EXIT INT TERM
    for pid in "$SMI_PID" "$SERVE_PID"; do
        [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null && kill "$pid" 2>/dev/null || true
    done
    for pid in "$SMI_PID" "$SERVE_PID"; do
        [[ -n "$pid" ]] && wait "$pid" 2>/dev/null || true
    done
    rm -f "$SERVE_LOG"
    exit "$rc"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

bash "$ROOT/scripts/mc-browser.sh" "$PACKAGE" --serve-only --prepared "$dir" >"$SERVE_LOG" 2>&1 &
SERVE_PID=$!
URL="" EVIDENCE=""
for _ in $(seq 1 300); do
    URL="$(sed -n 's/^URL: //p' "$SERVE_LOG" | head -1)"
    EVIDENCE="$(sed -n 's/^Evidence: //p' "$SERVE_LOG" | head -1)"
    [[ -n "$URL" && -n "$EVIDENCE" ]] && break
    kill -0 "$SERVE_PID" 2>/dev/null || break
    sleep 0.2
done
if [[ -z "$URL" || -z "$EVIDENCE" ]]; then
    echo "mp1-egpu: the server did not start" >&2
    cat "$SERVE_LOG" >&2
    # a stale prepared folder gives exit 66 from the serve step
    wait "$SERVE_PID" 2>/dev/null && exit 1 || exit $?
fi
echo "URL: $URL"
echo "Evidence: $EVIDENCE"
if command -v nvidia-smi >/dev/null 2>&1; then
    nvidia-smi --query-gpu=timestamp,name,driver_version,utilization.gpu,utilization.memory,memory.used,memory.total,clocks.sm,power.draw,temperature.gpu \
        --format=csv -l 1 >"$EVIDENCE/nvidia-smi.csv" 2>"$EVIDENCE/nvidia-smi.err" &
    SMI_PID=$!
fi
rc=0
node "$ROOT/tests/browser/egpu-run.js" "$URL" "$EVIDENCE" --secs "$SECS" || rc=$?
echo "mp1-egpu: egpu-run exit=$rc"
exit "$rc"
