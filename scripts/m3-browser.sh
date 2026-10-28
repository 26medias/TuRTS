#!/usr/bin/env bash
# F10 acceptance against a real Forge server and rendered browser controls.
# Linux defaults to a private headed Chromium context under Xvfb with Vulkan; callers may set
# M2_HEADLESS=true or override the Chrome profile variables without changing their desktop DISPLAY.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${ROOT}/target/m3-t16}"
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
M2_HEADLESS="${M2_HEADLESS:-false}"
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
# Keep each build serial: the server binary and packer do not share mutable build invocations.
env CARGO_BUILD_JOBS="$BUILD_JOBS" CARGO_NET_OFFLINE=true cargo build --release -p turts_server > "$OUT/server-build.log" 2>&1 || { cat "$OUT/server-build.log" >&2; exit 1; }
env CARGO_BUILD_JOBS="$BUILD_JOBS" CARGO_NET_OFFLINE=true cargo build --release -p turts_pack --bin turts-pack > "$OUT/packer-build.log" 2>&1 || { cat "$OUT/packer-build.log" >&2; exit 1; }
(cd crates/turts_client_web && env CARGO_BUILD_JOBS="$BUILD_JOBS" CARGO_NET_OFFLINE=true trunk build --release) > "$OUT/trunk-build.log" 2>&1 || { cat "$OUT/trunk-build.log" >&2; exit 1; }
for file in "$DIST"/*.wasm "$DIST"/*.js; do
    [[ -e "$file.br" && "$file" -nt "$file.br" ]] || brotli -q 11 -k -f "$file"
done
(cd tests/browser && npm ci --offline) > "$OUT/npm.log" 2>&1 || { cat "$OUT/npm.log" >&2; exit 1; }
mkdir -p "$OUT/pack"
cp -a mods/forge "$OUT/pack/forge"
"$CARGO_TARGET_DIR/release/turts-pack" "$OUT/pack/forge" > "$OUT/pack.log" 2>&1 || { cat "$OUT/pack.log" >&2; exit 1; }
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
"$CARGO_TARGET_DIR/release/turts_server" "$OUT/server.ron" --economy-log "$OUT/economy.jsonl" > "$OUT/server.log" 2>&1 &
SERVER_PID=$!
for _ in $(seq 1 120); do
    if curl -fsS "http://127.0.0.1:$HTTP_PORT/advertisement.json" >/dev/null 2>&1 && curl -fsS "http://127.0.0.1:$STATIC_PORT/" >/dev/null 2>&1; then break; fi
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then cat "$OUT/server.log" >&2; exit 1; fi
    sleep 0.1
done
BROWSER_COMMAND=(node tests/browser/m3.js "http://127.0.0.1:$STATIC_PORT/" "http://127.0.0.1:$HTTP_PORT" "ws://127.0.0.1:$WS_PORT" "$OUT")
if [[ "$M2_HEADLESS" == false ]]; then
    command -v xvfb-run >/dev/null 2>&1 || { echo "xvfb-run is required for the private headed M3 browser" >&2; exit 1; }
    xvfb-run --auto-servernum --server-args='-screen 0 1280x720x24' "${BROWSER_COMMAND[@]}" | tee "$OUT/browser.log"
else
    "${BROWSER_COMMAND[@]}" | tee "$OUT/browser.log"
fi
printf 'M3_BROWSER_EVIDENCE %s\n' "$OUT"
