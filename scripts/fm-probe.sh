#!/usr/bin/env bash
# fm-probe.sh --prepared DIR --package mods/<quarry_ridge|terrain_stockyard_ridge> --out DIR
# Browser measure of the rows FM1 and FM2 (T-7) on one public ridge package: outbound move, ordinary dig,
# return move of all 36 units. It starts the game through scripts/mp1-play.sh --prepared (silent form: a fresh
# Chrome profile and --alsa-output-device=null) and runs tests/browser/terrain-group/fm-probe.js on its debug port.
# It takes the quiet lock (exclusive) and then the GPU lock for the whole run.
# The movement mode is the environment of the caller (TURTS_MOVEMENT); the log records what the server has.
# Exit: the exit of the probe (0 = all judged FM rows pass), 66 when the prepared folder is stale, 2 for usage.
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PREP="" PACKAGE="" OUT=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --prepared) PREP="$2"; shift 2 ;;
        --package) PACKAGE="$2"; shift 2 ;;
        --out) OUT="$2"; shift 2 ;;
        *) echo "usage: scripts/fm-probe.sh --prepared DIR --package mods/<ridge package> --out DIR" >&2; exit 2 ;;
    esac
done
[[ -n "$PREP" && -n "$PACKAGE" && -n "$OUT" ]] || { echo "usage: scripts/fm-probe.sh --prepared DIR --package mods/<ridge package> --out DIR" >&2; exit 2; }
PKG="$(basename "$PACKAGE")"; BASE="${PKG%_ridge}"
[[ "$PKG" == quarry_ridge || "$PKG" == terrain_stockyard_ridge ]] || { echo "fm-probe: the package must be quarry_ridge or terrain_stockyard_ridge" >&2; exit 2; }
mkdir -p "$OUT"; LOG="$OUT/fm-probe.log"
echo "waiting for locks $(date -Is)" > "$LOG"
exec 8>/tmp/turts-quiet-performance.lock; flock -x 8
exec 9>/tmp/turts-gpu.lock; flock -x 9
echo "locks held $(date -Is)" >> "$LOG"
{ git -C "$ROOT" rev-parse HEAD; git -C "$ROOT" status --short; google-chrome --version; sha256sum "$PREP/manifest.json"; echo "TURTS_MOVEMENT of the caller: ${TURTS_MOVEMENT:-<not set>}"; } >> "$LOG" 2>&1
# The frozen GM2-narrow layout of the ridge packages gives the actor labels and the profiles.
python3 -B "$ROOT/tests/browser/terrain-group/generate.py" --root "$ROOT" --package "$BASE" --scene GM2-narrow --output "$OUT/fixture" > "$OUT/fixture.log" 2>&1 || { echo "fixture exit=$?" >> "$LOG"; exit 1; }
(cd "$ROOT" && exec bash scripts/mp1-play.sh --prepared "$PREP" --package "$PACKAGE" --silent) > "$OUT/launcher.log" 2>&1 &
PLAY=$!; PORT=""
for _ in $(seq 1 300); do
    PORT="$(sed -n 's/^DEBUG_PORT: //p' "$OUT/launcher.log" | head -1)"
    [[ -n "$PORT" ]] && grep -q '^mp1-play: close' "$OUT/launcher.log" && break
    kill -0 "$PLAY" 2>/dev/null || break
    sleep 0.2
done
if [[ -z "$PORT" ]]; then wait "$PLAY"; CODE=$?; echo "launcher did not start, exit=$CODE" >> "$LOG"; exit "$CODE"; fi
for PID in $(pgrep -f "$PREP/bin/turts_server"); do
    echo "turts_server pid=$PID TURTS_MOVEMENT=$(tr '\0' '\n' < "/proc/$PID/environ" | sed -n 's/^TURTS_MOVEMENT=//p' | head -1 | grep . || echo '<not set>')" >> "$LOG"
done
(cd "$ROOT/tests/browser" && TURTS_ROOT="$ROOT" node terrain-group/fm-probe.js "$PORT" "$OUT/fixture/fixture.json" "$PKG" "$OUT/run") > "$OUT/probe.log" 2>&1
CODE=$?
echo "probe exit=$CODE $(date -Is)" >> "$LOG"
kill -INT "$PLAY"; wait "$PLAY"
echo "released $(date -Is)" >> "$LOG"
exit "$CODE"
