#!/usr/bin/env bash
# Review M1 of mp1/sound-off-default: the probe tests/browser/click-without-hover-probe.js on a
# prepared package. Run it under the headed wrapper (mc_headed_v6.sh), never on your own Chrome:
#   mc_headed_v6.sh --mode pre-merge ... -- bash scripts/click-without-hover.sh --prepared DIR [--package P] [--guide-close]
# It starts scripts/mp1-play.sh (its own Chrome with its own profile), runs the probe on the
# debug port, and stops the play script. The exit code is the probe's.
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
[[ "${1:-}" == --prepared && -n "${2:-}" ]] || { echo "usage: $0 --prepared DIR [--package P] [--guide-close]" >&2; exit 2; }
DIR="$2"; shift 2
PACKAGE=mods/muster; GUIDE=()
while (($#)); do
    case "$1" in
        --package) PACKAGE="$2"; shift 2 ;;
        --guide-close) GUIDE=(--guide-close); shift ;;
        *) echo "unknown argument $1" >&2; exit 2 ;;
    esac
done
LOG="$(mktemp "${TMPDIR:-/tmp}/click-without-hover.XXXXXX")"
bash "$ROOT/scripts/mp1-play.sh" --prepared "$DIR" --package "$PACKAGE" >"$LOG" 2>&1 &
PLAY=$!
PORT=""
for _ in $(seq 1 600); do
    PORT="$(sed -n 's/^DEBUG_PORT: //p' "$LOG" | head -1)"
    [[ -n "$PORT" ]] && break
    kill -0 "$PLAY" 2>/dev/null || break
    sleep 0.2
done
if [[ -z "$PORT" ]]; then echo "no debug port"; cat "$LOG"; rm -f "$LOG"; exit 1; fi
(cd "$ROOT/tests/browser" && node click-without-hover-probe.js "$PORT" "${GUIDE[@]}")
rc=$?
kill -TERM "$PLAY" 2>/dev/null
wait "$PLAY" 2>/dev/null
rm -f "$LOG"
exit "$rc"
