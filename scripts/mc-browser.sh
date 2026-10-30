#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET="${CARGO_TARGET_DIR:-$ROOT/target/mc-t20}"
export CARGO_TARGET_DIR="$TARGET"
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
BUILD_JOBS="${CARGO_BUILD_JOBS:-3}"
EVIDENCE_ROOT="${MC_EVIDENCE_ROOT:-$ROOT/target/mc-evidence/t20-builder}"
OUT_ROOT="${MC_BROWSER_OUT:-$EVIDENCE_ROOT/runs}"
CHROME_BIN="${CHROME_BIN:-$(command -v google-chrome || command -v chromium || true)}"
if [[ -z "$CHROME_BIN" || ! -x "$CHROME_BIN" ]]; then echo "Chrome binary unavailable; set CHROME_BIN" >&2; exit 1; fi
export CHROME_BIN
PACKAGE="${1:-}"
[[ -n "$PACKAGE" ]] || { echo "usage: scripts/mc-browser.sh <package-dir> [driver.js oracle.js viewport [--params row.json]] [--serve-only]" >&2; exit 2; }
shift
SERVE_ONLY=false
DRIVER="" ORACLE="" VIEWPORT="1280x720" ROW_FILE=""
while (($#)); do
    case "$1" in
        --serve-only) SERVE_ONLY=true; shift ;;
        --params) (($# >= 2)) || { echo "--params requires a row file" >&2; exit 2; }; ROW_FILE="$2"; shift 2 ;;
        *)
            if [[ -z "$DRIVER" ]]; then DRIVER="$1"; elif [[ -z "$ORACLE" ]]; then ORACLE="$1"; else VIEWPORT="$1"; fi
            shift ;;
    esac
done
[[ "$SERVE_ONLY" == true || ( -n "$DRIVER" && -n "$ORACLE" ) ]] || { echo "driver and oracle are required" >&2; exit 2; }
[[ "$BUILD_JOBS" =~ ^[1-4]$ ]] || { echo "CARGO_BUILD_JOBS must be between 1 and 4" >&2; exit 2; }
DIST="$ROOT/crates/turts_client_web/dist"
mkdir -p "$OUT_ROOT"
OUT="$(mktemp -d "$OUT_ROOT/run-$(date -u +%Y%m%dT%H%M%SZ)-XXXXXX")"
PACK="$OUT/pack"
mkdir -p "$PACK"
PACKAGE_NAME="$(basename "$PACKAGE")"
cp -a "$ROOT/$PACKAGE" "$PACK/$PACKAGE_NAME"
if ! (cd "$ROOT/tests/browser" && node -e 'require.resolve("playwright-core")') >"$OUT/node-check.log" 2>&1; then
    (cd "$ROOT/tests/browser" && npm ci) >"$OUT/npm-ci.log" 2>&1 || { cat "$OUT/npm-ci.log" >&2; exit 1; }
fi

# Keep each cargo invocation sequential; this runner never starts a second compiler.
env CARGO_BUILD_JOBS="$BUILD_JOBS" cargo build --release -p turts_server >"$OUT/server-build.log" 2>&1 || { cat "$OUT/server-build.log" >&2; exit 1; }
env CARGO_BUILD_JOBS="$BUILD_JOBS" cargo build --release -p turts_pack --bin turts-pack >"$OUT/packer-build.log" 2>&1 || { cat "$OUT/packer-build.log" >&2; exit 1; }
env CARGO_BUILD_JOBS="$BUILD_JOBS" cargo run --release -p turts_pack --bin turts-pack -- "$PACK/$PACKAGE_NAME" >"$OUT/pack.log" 2>&1 || { cat "$OUT/pack.log" >&2; exit 1; }
(cd "$ROOT/crates/turts_client_web" && env CARGO_BUILD_JOBS="$BUILD_JOBS" trunk build --release) >"$OUT/trunk-build.log" 2>&1 || { cat "$OUT/trunk-build.log" >&2; exit 1; }

read -r STATIC_PORT HTTP_PORT WS_PORT < <(python3 - <<'PYPORTS'
import socket
sockets=[socket.socket() for _ in range(3)]
try:
    for sock in sockets: sock.bind(("127.0.0.1",0))
    print(*(sock.getsockname()[1] for sock in sockets))
finally:
    for sock in sockets: sock.close()
PYPORTS
)
cat >"$OUT/server.ron" <<EOF_RON
(
    game_package: "$PACK/$PACKAGE_NAME.tar",
    http_listen: "127.0.0.1:$HTTP_PORT",
    ws_listen: "127.0.0.1:$WS_PORT",
    public_url: "http://127.0.0.1:$HTTP_PORT",
    tick_hz: 20,
    static_dir: "$DIST",
)
EOF_RON
"$TARGET/release/turts_server" "$OUT/server.ron" --tick-log "$OUT/server-tick.log" >"$OUT/server.log" 2>&1 &
SERVER_PID=$!
python3 -m http.server "$STATIC_PORT" --bind 127.0.0.1 --directory "$DIST" >"$OUT/static.log" 2>&1 &
STATIC_PID=$!
cleanup() {
    local rc=$?
    trap - EXIT INT TERM
    for pid in "${SERVER_PID:-}" "${STATIC_PID:-}"; do
        [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null && kill "$pid" 2>/dev/null || true
    done
    for pid in "${SERVER_PID:-}" "${STATIC_PID:-}"; do
        [[ -n "$pid" ]] && wait "$pid" 2>/dev/null || true
    done
    printf '{"exit_code":%s,"completed_utc":"%s"}\n' "$rc" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" >"$OUT/completion.json"
    exit "$rc"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
ready=false
for _ in $(seq 1 100); do
    if curl -fsS "http://127.0.0.1:$STATIC_PORT/" >/dev/null 2>&1 && curl -fsS "http://127.0.0.1:$HTTP_PORT/advertisement.json" >/dev/null 2>&1; then ready=true; break; fi
    if ! kill -0 "$SERVER_PID" 2>/dev/null || ! kill -0 "$STATIC_PID" 2>/dev/null; then break; fi
    sleep 0.1
done
[[ "$ready" == true ]] || { echo "harness services failed to become ready; evidence: $OUT" >&2; cat "$OUT/static.log" "$OUT/server.log" >&2; exit 1; }
STATIC_URL="http://127.0.0.1:$STATIC_PORT/"
GAME_QUERY="$(python3 - "http://127.0.0.1:$HTTP_PORT" "ws://127.0.0.1:$WS_PORT" <<'PYQUERY'
from urllib.parse import urlencode
import sys
print(urlencode({"server":sys.argv[1],"ws":sys.argv[2]}))
PYQUERY
)"
URL="$STATIC_URL?$GAME_QUERY"
echo "URL: $URL"
echo "Evidence: $OUT"
if [[ "$SERVE_ONLY" == true ]]; then wait; fi

PARAMS="$OUT/params.json"
python3 - "$PACKAGE_NAME" "$URL" "http://127.0.0.1:$HTTP_PORT" "ws://127.0.0.1:$WS_PORT" "$ROW_FILE" "$PARAMS" "$VIEWPORT" <<'PYPARAMS'
import json, pathlib, re, sys
package,url,http,ws,row,out,viewport=sys.argv[1:]
match=re.fullmatch(r"(\d+)x(\d+)(?:@([0-9]+(?:\.[0-9]+)?))?",viewport)
if not match: raise SystemExit("invalid viewport: "+viewport)
data={"package":package,"url":url,"server_url":http,"ws_url":ws,"windows":1,"device_scale_factor":float(match.group(3) or 1)}
if row:
    row_data=json.loads(pathlib.Path(row).read_text())
    data["windows"]=row_data.get("windows",1)
    data["device_scale_factor"]=row_data.get("device_scale_factor",data["device_scale_factor"])
    data["row"]=row_data.get("row")
data["viewport"]={"width":int(match.group(1)),"height":int(match.group(2)),"device_scale_factor":data["device_scale_factor"]}
pathlib.Path(out).write_text(json.dumps(data,sort_keys=True)+"\n")
PYPARAMS
resolve_script() {
    local candidate="$1"
    if [[ -f "$ROOT/$candidate" ]]; then printf '%s\n' "$ROOT/$candidate"
    elif [[ -f "$ROOT/tests/browser/$candidate" ]]; then printf '%s\n' "$ROOT/tests/browser/$candidate"
    else printf '%s\n' "$ROOT/$candidate"; fi
}
DRIVER_PATH="$(resolve_script "$DRIVER")"
ORACLE_PATH="$(resolve_script "$ORACLE")"
node "$ROOT/tests/browser/harness/run.js" "$URL" "$OUT" "$DRIVER_PATH" "$ORACLE_PATH" --params "$PARAMS" --viewport "$VIEWPORT" | tee "$OUT/browser.log"
