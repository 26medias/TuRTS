#!/usr/bin/env bash
# F10 acceptance against a real Forge server and rendered browser controls.
# Linux defaults to a private headed Chromium context under Xvfb with Vulkan; callers may set
# M2_HEADLESS=true or override the Chrome profile variables without changing their desktop DISPLAY.
# Q-PREBUILD: `m3-browser.sh --prepare DIR` builds the server, the packer and the wasm client, and packs
# forge, into DIR, then stops. `m3-browser.sh --prepared DIR` builds nothing; it refuses (exit 66) when DIR
# does not match the source (scripts/mc_prepare.py). With no flag the script builds inside the run (CI).
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PREPARE_DIR="" PREPARED=""
while (($#)); do
    case "$1" in
        --prepare) (($# >= 2)) || { echo "--prepare requires a folder" >&2; exit 2; }; PREPARE_DIR="$(realpath -m "$2")"; shift 2 ;;
        --prepared) (($# >= 2)) || { echo "--prepared requires a folder" >&2; exit 2; }; PREPARED="$(realpath -m "$2")"; shift 2 ;;
        *) echo "usage: scripts/m3-browser.sh [--prepare DIR | --prepared DIR]" >&2; exit 2 ;;
    esac
done
[[ -z "$PREPARE_DIR" || -z "$PREPARED" ]] || { echo "--prepare and --prepared cannot be used together" >&2; exit 2; }
cd "$ROOT"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${ROOT}/target/m3-t16}"
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
M2_HEADLESS="${M2_HEADLESS:-false}"
# The Chrome flags: a caller's M2_* wins, then the MC_* of the gate runner (mc_headed_v3.sh), then the default.
M3_CALLER_CHROME_ARGS="${M2_CHROME_ARGS_JSON:-${MC_CHROME_ARGS_JSON:-}}"
M3_CALLER_IGNORE_ARGS="${M2_IGNORE_DEFAULT_ARGS_JSON:-${MC_CHROME_IGNORE_DEFAULT_ARGS_JSON:-}}"
M2_CHROME_ARGS_JSON="${M2_CHROME_ARGS_JSON:-[\"--enable-gpu\",\"--use-angle=vulkan\",\"--enable-features=Vulkan\",\"--enable-unsafe-webgpu\"]}"
M2_IGNORE_DEFAULT_ARGS_JSON="${M2_IGNORE_DEFAULT_ARGS_JSON:-[\"--enable-unsafe-swiftshader\"]}"
export M2_HEADLESS M2_CHROME_ARGS_JSON M2_IGNORE_DEFAULT_ARGS_JSON
case "$M2_HEADLESS" in
    true|false) ;;
    *) echo "M2_HEADLESS must be true or false" >&2; exit 2 ;;
esac
BUILD_JOBS="${M3_BUILD_JOBS:-1}"
[[ "$BUILD_JOBS" == 1 ]] || { echo "M3_BUILD_JOBS must be 1" >&2; exit 2; }
CHROME_BIN="${CHROME_BIN:-$(command -v google-chrome || command -v chromium || true)}"
[[ -n "$CHROME_BIN" && -x "$CHROME_BIN" ]] || { echo "Chrome binary unavailable; set CHROME_BIN" >&2; exit 1; }
export CHROME_BIN
OUT_ROOT="${M3_ARTIFACT_ROOT:-${ROOT}/target/m3-evidence/t16-builder/browser}"
mkdir -p "$OUT_ROOT"
OUT="$(mktemp -d "$OUT_ROOT/run-$(date -u +%Y%m%dT%H%M%SZ)-XXXXXX")"
STATIC_PID=""
SERVER_PID=""
cleanup() {
    local status=$?
    for pid in "$SERVER_PID" "$STATIC_PID"; do
        if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then kill "$pid" 2>/dev/null || true; fi
    done
    for pid in "$SERVER_PID" "$STATIC_PID"; do
        if [[ -n "$pid" ]]; then wait "$pid" 2>/dev/null || true; fi
    done
    printf '{"exit_code":%s,"server_pid":%s,"static_pid":%s}\n' "$status" "${SERVER_PID:-null}" "${STATIC_PID:-null}" > "$OUT/cleanup.json"
    return "$status"
}
trap cleanup EXIT
PORTS="$(python3 - <<'PY'
import socket
sockets=[]
ports=[]
for _ in range(3):
 s=socket.socket(); s.bind(('127.0.0.1',0)); sockets.append(s); ports.append(s.getsockname()[1])
print(*ports)
for s in sockets: s.close()
PY
)"
read -r STATIC_PORT HTTP_PORT WS_PORT <<<"$PORTS"
DIST="$ROOT/crates/turts_client_web/dist"
SERVER_BIN="$CARGO_TARGET_DIR/release/turts_server"
mkdir -p "$OUT/pack"
if [[ -n "$PREPARED" ]]; then
    # The stale check comes first. A prepared run starts no cargo, no trunk, no brotli and no npm.
    python3 -B scripts/mc_prepare.py check "$PREPARED" --root "$ROOT" --package mods/forge \
        --need bin/turts_server --need dist/index.html --need pack/forge.tar || exit $?
    (cd tests/browser && node -e 'require.resolve("playwright-core")') >/dev/null 2>&1 \
        || { echo "prepared run: tests/browser has no node_modules; the prepare step runs npm ci" >&2; exit 66; }
    DIST="$PREPARED/dist"
    SERVER_BIN="$PREPARED/bin/turts_server"
    cp -a "$PREPARED/pack/." "$OUT/pack/"
else
    if [[ -n "$PREPARE_DIR" ]]; then
        if [[ -n "$(ls -A "$PREPARE_DIR" 2>/dev/null)" && ! -f "$PREPARE_DIR/manifest.json" && ! -f "$PREPARE_DIR/.begin.json" ]]; then
            echo "--prepare: $PREPARE_DIR is not empty and is not a prepared folder" >&2
            exit 2
        fi
        rm -rf "$PREPARE_DIR"
        mkdir -p "$PREPARE_DIR"
        python3 -B scripts/mc_prepare.py begin --root "$ROOT" --out "$PREPARE_DIR" --package mods/forge
    fi
    # Keep each build serial: the server binary and packer do not share mutable build invocations.
    env CARGO_BUILD_JOBS="$BUILD_JOBS" CARGO_NET_OFFLINE=true cargo build --release -p turts_server > "$OUT/server-build.log" 2>&1 || { cat "$OUT/server-build.log" >&2; exit 1; }
    env CARGO_BUILD_JOBS="$BUILD_JOBS" CARGO_NET_OFFLINE=true cargo build --release -p turts_pack --bin turts-pack > "$OUT/packer-build.log" 2>&1 || { cat "$OUT/packer-build.log" >&2; exit 1; }
    (cd crates/turts_client_web && env CARGO_BUILD_JOBS="$BUILD_JOBS" CARGO_NET_OFFLINE=true trunk build --release) > "$OUT/trunk-build.log" 2>&1 || { cat "$OUT/trunk-build.log" >&2; exit 1; }
    for file in "$DIST"/*.wasm "$DIST"/*.js; do
        [[ -e "$file.br" && "$file" -nt "$file.br" ]] || brotli -q 11 -k -f "$file"
    done
    (cd tests/browser && npm ci --offline) > "$OUT/npm.log" 2>&1 || { cat "$OUT/npm.log" >&2; exit 1; }
    cp -a mods/forge "$OUT/pack/forge"
    "$CARGO_TARGET_DIR/release/turts-pack" "$OUT/pack/forge" > "$OUT/pack.log" 2>&1 || { cat "$OUT/pack.log" >&2; exit 1; }
    if [[ -n "$PREPARE_DIR" ]]; then
        mkdir -p "$PREPARE_DIR/bin"
        cp "$SERVER_BIN" "$CARGO_TARGET_DIR/release/turts-pack" "$PREPARE_DIR/bin/"
        cp -a "$DIST" "$PREPARE_DIR/dist"
        cp -a "$OUT/pack" "$PREPARE_DIR/pack"
        python3 -B scripts/mc_prepare.py seal --root "$ROOT" --out "$PREPARE_DIR" --package mods/forge
        echo "PREPARED: $PREPARE_DIR"
        exit 0
    fi
fi
cat > "$OUT/server.ron" <<EOF
(
    game_package: "$OUT/pack/forge.tar",
    http_listen: "127.0.0.1:$HTTP_PORT",
    ws_listen: "127.0.0.1:$WS_PORT",
    public_url: "http://127.0.0.1:$HTTP_PORT",
    tick_hz: 20,
    static_dir: "$DIST",
)
EOF
python3 -m http.server "$STATIC_PORT" --bind 127.0.0.1 --directory "$DIST" > "$OUT/static.log" 2>&1 &
STATIC_PID=$!
"$SERVER_BIN" "$OUT/server.ron" --economy-log "$OUT/economy.jsonl" --tick-log "$OUT/server-tick.log" > "$OUT/server.log" 2>&1 &
SERVER_PID=$!
for _ in $(seq 1 120); do
    if curl -fsS "http://127.0.0.1:$HTTP_PORT/advertisement.json" >/dev/null 2>&1 && curl -fsS "http://127.0.0.1:$STATIC_PORT/" >/dev/null 2>&1; then break; fi
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then cat "$OUT/server.log" >&2; exit 1; fi
    sleep 0.1
done
# MC task 21: m3-driver.js acts through the harness io only (window 1 slot 0, window 2 slot 1);
# m3-oracle.js judges from the checkpoints, the tick log and the economy log.
PAGE_URL="$(python3 - "http://127.0.0.1:$STATIC_PORT/" "http://127.0.0.1:$HTTP_PORT" "ws://127.0.0.1:$WS_PORT" <<'EOF'
from urllib.parse import urlencode
import sys
print(sys.argv[1] + "?" + urlencode({"server": sys.argv[2], "ws": sys.argv[3]}))
EOF
)"
python3 - "$OUT/params.json" "$PAGE_URL" "http://127.0.0.1:$HTTP_PORT" "ws://127.0.0.1:$WS_PORT" <<'EOF'
import json, pathlib, sys
out, url, http, ws = sys.argv[1:]
pathlib.Path(out).write_text(json.dumps({
    "row": "M3", "package": "forge", "url": url, "server_url": http, "ws_url": ws, "windows": 2,
    "capture_scene_checkpoints": True,
    "viewport": {"width": 1280, "height": 720, "device_scale_factor": 1},
}, sort_keys=True) + "\n")
EOF
export MC_CHROME_ARGS_JSON="${M3_CALLER_CHROME_ARGS:-$M2_CHROME_ARGS_JSON}" MC_CHROME_IGNORE_DEFAULT_ARGS_JSON="${M3_CALLER_IGNORE_ARGS:-$M2_IGNORE_DEFAULT_ARGS_JSON}"
BROWSER_COMMAND=(node tests/browser/harness/run.js "$PAGE_URL" "$OUT" tests/browser/m3-driver.js tests/browser/m3-oracle.js
    --params "$OUT/params.json" --viewport 1280x720)
if [[ "$M2_HEADLESS" == false ]]; then
    command -v xvfb-run >/dev/null 2>&1 || { echo "xvfb-run is required for the private headed M3 browser" >&2; exit 1; }
    MC_BROWSER_HEADED=1 xvfb-run --auto-servernum --server-args='-screen 0 1280x720x24' "${BROWSER_COMMAND[@]}" | tee "$OUT/browser.log"
else
    "${BROWSER_COMMAND[@]}" | tee "$OUT/browser.log"
fi
# m3.js failed on any page error; the harness records them after the oracle runs.
python3 - "$OUT/browser-diagnostics.json" <<'EOF'
import json, sys
counts = json.load(open(sys.argv[1])).get("counts_by_kind", {})
if counts.get("pageerror", 0):
    raise SystemExit(f"browser page error: {counts['pageerror']} pageerror events in {sys.argv[1]}")
EOF
printf 'M3_BROWSER_EVIDENCE %s\n' "$OUT"
