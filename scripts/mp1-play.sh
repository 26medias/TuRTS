#!/usr/bin/env bash
# MP1 play script (plan task L0-DBG, step 2): a person plays in a Chrome that a harness can read.
# The person's own Chrome is never used: this script starts a second Chrome with its own empty
# profile and a remote debugging port that it picks at start.
#
#   scripts/mp1-play.sh --prepare DIR [--package P]          build the server, the package and the client into DIR
#   scripts/mp1-play.sh --prepared DIR [--package P]         serve the game, start Chrome, print the URL and the port
#   scripts/mp1-play.sh --url URL                            start only the Chrome, at the loopback address URL (a game that
#                                                            another command serves; the cold-play tool uses this form)
#   scripts/mp1-play.sh --print-plan                         print the free port and the Chrome command; start nothing
#
# --prepared builds nothing (scripts/mc-browser.sh --serve-only --prepared refuses with exit 66 when DIR
# does not match the source; docs/qa/prepared-runs.md). The default package is mods/muster.
#
# Printed lines (stdout):
#   URL: <game address>            open it in the Chrome that this script started (Ctrl+N gives a second window)
#   DEBUG_PORT: <port>             a free port, picked at each start; never a fixed number
#   PROFILE: <folder>              the non-default Chrome profile; removed when the script ends
#   CAPTURE: node tests/browser/mp1-capture.js <port>      read the debug records of every window
#
# Chrome 136 and later does not honour --remote-debugging-port on the default profile
# (https://developer.chrome.com/blog/remote-debugging-port), so --user-data-dir is always set to a new
# folder. The port listens on 127.0.0.1 (checked with Chrome 157, headless: ss -ltn). This script does not
# mute Chrome and does not change the system sound.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PACKAGE=mods/muster
usage() {
    echo "usage: scripts/mp1-play.sh --prepare DIR [--package P] | --prepared DIR [--package P] | --url URL | --print-plan" >&2
    exit 2
}
(($# >= 1)) || usage
mode="$1"
shift
dir=""
if [[ "$mode" == --prepare || "$mode" == --prepared || "$mode" == --url ]]; then
    (($# >= 1)) || usage
    dir="$1"
    shift
fi
while (($#)); do
    case "$1" in
        --package) (($# >= 2)) || usage; PACKAGE="$2"; shift 2 ;;
        *) usage ;;
    esac
done

# A free port on 127.0.0.1, picked now. Another process can take it before Chrome binds it; then
# Chrome fails to listen and the script says so (see the check after the start).
free_port() {
    python3 -I -B -c 'import socket; s = socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1])'
}

# The Chrome arguments for one port and one profile folder, one per line. The WebGPU flags are the
# ones of the CEO play sheet (vault/Reports/2026-11-12 MC play instructions for the CEO.md).
chrome_args() {
    local port="$1" profile="$2"
    printf '%s\n' "--user-data-dir=$profile" "--remote-debugging-port=$port" \
        --enable-unsafe-webgpu --enable-features=Vulkan --use-angle=vulkan \
        --no-first-run --no-default-browser-check
}

if [[ "$mode" == --print-plan ]]; then
    port="$(free_port)"
    profile="${TMPDIR:-/tmp}/turts-play-profile.PLAN"
    echo "URL: (the server is not started in --print-plan)"
    echo "DEBUG_PORT: $port"
    echo "PROFILE: $profile"
    echo "CHROME: ${CHROME_BIN:-google-chrome} $(chrome_args "$port" "$profile" | tr '\n' ' ')<URL>"
    echo "CAPTURE: node tests/browser/mp1-capture.js $port"
    exit 0
fi

if [[ "$mode" == --prepare ]]; then
    exec bash "$ROOT/scripts/mc-browser.sh" "$PACKAGE" --prepare "$dir"
fi
[[ "$mode" == --prepared || "$mode" == --url ]] || usage
if [[ "$mode" == --url ]]; then
    [[ "$dir" =~ ^https?://(127\.0\.0\.1|localhost)(:[0-9]+)?(/|\?|$) ]] || { echo "mp1-play: --url takes a loopback address (http://127.0.0.1:PORT/...)" >&2; exit 2; }
fi

CHROME_BIN="${CHROME_BIN:-$(command -v google-chrome || command -v chromium || true)}"
[[ -n "$CHROME_BIN" && -x "$CHROME_BIN" ]] || { echo "Chrome binary unavailable; set CHROME_BIN" >&2; exit 1; }
SERVE_LOG="$(mktemp "${TMPDIR:-/tmp}/mp1-play-serve.XXXXXX")"
PROFILE="$(mktemp -d "${TMPDIR:-/tmp}/turts-play-profile.XXXXXX")"
SERVE_PID="" CHROME_PID=""
cleanup() {
    local rc=$?
    trap - EXIT INT TERM
    # Only the processes that this script started.
    for pid in "$CHROME_PID" "$SERVE_PID"; do
        [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null && kill "$pid" 2>/dev/null || true
    done
    for pid in "$CHROME_PID" "$SERVE_PID"; do
        [[ -n "$pid" ]] && wait "$pid" 2>/dev/null || true
    done
    rm -rf "$PROFILE" "$PROFILE.chrome.log" "$SERVE_LOG"
    exit "$rc"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

URL=""
if [[ "$mode" == --url ]]; then
    URL="$dir"
else
    bash "$ROOT/scripts/mc-browser.sh" "$PACKAGE" --serve-only --prepared "$dir" >"$SERVE_LOG" 2>&1 &
    SERVE_PID=$!
    for _ in $(seq 1 300); do
        URL="$(sed -n 's/^URL: //p' "$SERVE_LOG" | head -1)"
        [[ -n "$URL" ]] && break
        kill -0 "$SERVE_PID" 2>/dev/null || break
        sleep 0.2
    done
    if [[ -z "$URL" ]]; then
        echo "mp1-play: the server did not start" >&2
        cat "$SERVE_LOG" >&2
        # a stale prepared folder gives exit 66 from the serve step
        wait "$SERVE_PID" 2>/dev/null && exit 1 || exit $?
    fi
fi

PORT="$(free_port)"
mapfile -t ARGS < <(chrome_args "$PORT" "$PROFILE")
"$CHROME_BIN" "${ARGS[@]}" "$URL" >"$PROFILE.chrome.log" 2>&1 &
CHROME_PID=$!
# Chrome writes the port that it listens on to <profile>/DevToolsActivePort.
for _ in $(seq 1 100); do
    [[ -s "$PROFILE/DevToolsActivePort" ]] && break
    kill -0 "$CHROME_PID" 2>/dev/null || break
    sleep 0.1
done
if [[ "$(head -1 "$PROFILE/DevToolsActivePort" 2>/dev/null || true)" != "$PORT" ]]; then
    echo "mp1-play: Chrome did not listen on port $PORT (see $PROFILE.chrome.log)" >&2
    exit 1
fi
echo "URL: $URL"
echo "DEBUG_PORT: $PORT"
echo "PROFILE: $PROFILE"
echo "CAPTURE: node tests/browser/mp1-capture.js $PORT"
echo "mp1-play: close the Chrome window or press Ctrl+C to stop."
wait "$CHROME_PID" || true
