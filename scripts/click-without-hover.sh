#!/usr/bin/env bash
# Review M1 of mp1/sound-off-default: the probe tests/browser/click-without-hover-probe.js on a
# prepared package. Run it under the headed wrapper (mc_headed_v6.sh), never on your own Chrome:
#   mc_headed_v6.sh --mode pre-merge ... -- bash scripts/click-without-hover.sh --prepared DIR [--package P] [--guide-close] [--map-press-first]
# It serves the prepared package (scripts/mc-browser.sh --serve-only --prepared DIR), runs the probe
# on the URL (the probe starts its own Chrome with the MC browser profile), and stops the server.
# The exit code is the probe's.
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
[[ "${1:-}" == --prepared && -n "${2:-}" ]] || { echo "usage: $0 --prepared DIR [--package P] [--guide-close] [--map-press-first]" >&2; exit 2; }
DIR="$2"; shift 2
PACKAGE=mods/muster; GUIDE=()
while (($#)); do
    case "$1" in
        --package) PACKAGE="$2"; shift 2 ;;
        --guide-close) GUIDE+=(--guide-close); shift ;;
        --map-press-first) GUIDE+=(--map-press-first); shift ;;
        *) echo "unknown argument $1" >&2; exit 2 ;;
    esac
done
LOG="$(mktemp "${TMPDIR:-/tmp}/click-without-hover.XXXXXX")"
bash "$ROOT/scripts/mc-browser.sh" "$PACKAGE" --serve-only --prepared "$DIR" >"$LOG" 2>&1 &
SERVE=$!
URL=""
for _ in $(seq 1 300); do
    URL="$(sed -n 's/^URL: //p' "$LOG" | head -1)"
    [[ -n "$URL" ]] && break
    kill -0 "$SERVE" 2>/dev/null || break
    sleep 0.2
done
if [[ -z "$URL" ]]; then echo "the server did not start"; cat "$LOG"; rm -f "$LOG"; exit 1; fi
(cd "$ROOT/tests/browser" && node click-without-hover-probe.js "$URL" ${GUIDE[@]+"${GUIDE[@]}"})
rc=$?
kill -TERM "$SERVE" 2>/dev/null
wait "$SERVE" 2>/dev/null
rm -f "$LOG"
exit "$rc"
