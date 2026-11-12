#!/usr/bin/env bash
# Q-COLD: one cold-play session (docs/qa/cold-play.md). It starts a game and a headed Chrome through
# scripts/mp1-play.sh, then runs the session process (tests/browser/cold-session.js) until the session
# limit or the `end` call. The evidence folder is DIR.
#
#   scripts/cold-play.sh --session DIR --prepared PREP [--package P] [options]    start the game from a prepared folder
#   scripts/cold-play.sh --session DIR --url URL [options]                        use a game that another command serves
#
# Options: --limit-secs N (1 to 1500, default 300)   --viewport WxH|none (default 1280x720)   --build TEXT
#          --probe first-click|buttons   a scripted probe with no agent (tests/browser/cold-probe.js); the session
#                                        ends when the probe ends. A probe needs a game that no one has joined
#                                        (a slot is permanent): one fresh game for each probe and for the agent session.
#          --control close-layer         with --probe first-click: the tool closes the layer that the start click
#                                        met, then probes (the positive control: the probe can pass)
# A readiness run is: --probe first-click, then --probe buttons, then (only if both pass) the agent session, each on
# a fresh game; then scripts/qa/cold_play_check.py <folder> --also <folder> --also <folder>.
#
# The headed Chrome runs only under the lock wrapper (the GPU lock). The caller does:
#   bash tests/browser (npm ci) and scripts/mp1-play.sh --prepare PREP       outside the lock
#   mc_headed_v6.sh --mode pre-merge <exec_in_lane_v4 flags> -- bash scripts/cold-play.sh --session DIR ...
# The session folder must be outside the lane's git tree (the lane runner checks that the source did not
# change) and must not exist: a session never writes into an old folder.
# The session ends by itself at the limit: it must end before the wrapper's hold limit (1,800 s).
# Exit: 0 the session ended (limit or end call); 2 wrong arguments; other: the start failed (see DIR/internal).
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
usage() {
    echo "usage: scripts/cold-play.sh --session DIR (--prepared PREP [--package P] | --url URL) [--limit-secs N] [--viewport WxH|none] [--build TEXT] [--probe first-click|buttons [--control close-layer]]" >&2
    exit 2
}
SESSION="" PREP="" PACKAGE="" URL_ARG="" LIMIT=300 VIEWPORT=1280x720 BUILD="" PROBE="" CONTROL=""
while (($#)); do
    (($# >= 2)) || usage
    case "$1" in
        --session) SESSION="$2" ;;
        --prepared) PREP="$2" ;;
        --package) PACKAGE="$2" ;;
        --url) URL_ARG="$2" ;;
        --limit-secs) LIMIT="$2" ;;
        --viewport) VIEWPORT="$2" ;;
        --build) BUILD="$2" ;;
        --probe) PROBE="$2" ;;
        --control) CONTROL="$2" ;;
        *) usage ;;
    esac
    shift 2
done
[[ -n "$SESSION" ]] || usage
[[ -n "$PREP" || -n "$URL_ARG" ]] || usage
[[ -z "$PREP" || -z "$URL_ARG" ]] || usage
[[ -z "$PACKAGE" || -n "$PREP" ]] || usage
[[ "$LIMIT" =~ ^[0-9]+$ ]] && ((LIMIT >= 1 && LIMIT <= 1500)) || { echo "cold-play: --limit-secs must be a whole number from 1 to 1500" >&2; exit 2; }
[[ "$VIEWPORT" == none || "$VIEWPORT" =~ ^[0-9]{3,5}x[0-9]{3,5}$ ]] || { echo "cold-play: --viewport takes WIDTHxHEIGHT or none" >&2; exit 2; }
[[ -z "$PROBE" || "$PROBE" == first-click || "$PROBE" == buttons ]] || { echo "cold-play: --probe takes first-click or buttons" >&2; exit 2; }
[[ -z "$CONTROL" || ( "$CONTROL" == close-layer && "$PROBE" == first-click ) ]] || { echo "cold-play: --control close-layer goes with --probe first-click" >&2; exit 2; }
[[ ! -e "$SESSION" ]] || { echo "cold-play: $SESSION exists; a session writes a new folder" >&2; exit 2; }
[[ -d "$ROOT/tests/browser/node_modules/playwright-core" ]] || { echo "cold-play: run npm ci in tests/browser first" >&2; exit 2; }

mkdir -p "$SESSION/internal"
PLAY_LOG="$SESSION/internal/play.log"
PLAY_PID="" NODE_PID=""
cleanup() {
    local rc=$?
    trap - EXIT INT TERM
    # Only the processes that this script started. mp1-play.sh ends its own Chrome and server.
    for pid in "$NODE_PID" "$PLAY_PID"; do
        [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null && kill "$pid" 2>/dev/null || true
    done
    for pid in "$NODE_PID" "$PLAY_PID"; do
        [[ -n "$pid" ]] && wait "$pid" 2>/dev/null || true
    done
    exit "$rc"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

if [[ -n "$PREP" ]]; then
    play_args=(--prepared "$PREP")
    [[ -z "$PACKAGE" ]] || play_args+=(--package "$PACKAGE")
else
    play_args=(--url "$URL_ARG")
fi
bash "$ROOT/scripts/mp1-play.sh" "${play_args[@]}" >"$PLAY_LOG" 2>&1 &
PLAY_PID=$!
URL="" PORT=""
for _ in $(seq 1 600); do
    URL="$(sed -n 's/^URL: //p' "$PLAY_LOG" | head -1)"
    PORT="$(sed -n 's/^DEBUG_PORT: //p' "$PLAY_LOG" | head -1)"
    [[ -n "$URL" && -n "$PORT" ]] && break
    kill -0 "$PLAY_PID" 2>/dev/null || break
    sleep 0.2
done
if [[ -z "$URL" || -z "$PORT" ]]; then
    echo "cold-play: the game or Chrome did not start" >&2
    cat "$PLAY_LOG" >&2
    wait "$PLAY_PID" 2>/dev/null && exit 1 || exit $?
fi

node_args=(--port "$PORT" --session "$SESSION" --url "$URL" --limit-secs "$LIMIT" --viewport "$VIEWPORT")
[[ -z "$BUILD" ]] || node_args+=(--build "$BUILD")
[[ -z "$PROBE" ]] || node_args+=(--probe "$PROBE")
[[ -z "$CONTROL" ]] || node_args+=(--control "$CONTROL")
node "$ROOT/tests/browser/cold-session.js" "${node_args[@]}" >"$SESSION/internal/session.log" 2>&1 &
NODE_PID=$!
wait "$NODE_PID" || { rc=$?; echo "cold-play: the session process failed (exit $rc); see $SESSION/internal/session.log" >&2; cat "$SESSION/internal/session.log" >&2; NODE_PID=""; exit "$rc"; }
NODE_PID=""
echo "cold-play: the session ended; evidence in $SESSION"
