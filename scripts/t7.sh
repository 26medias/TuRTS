#!/usr/bin/env bash
# T7 browser join (design section 7; plan Task 12): the trunk-built client joins the real
# server in headless Chrome, and the segment oracle checks what the browser presented.
#
# Needs: cargo, trunk 0.22.0, brotli, node with `npm ci` done in tests/browser, python3, curl,
# and Google Chrome (CHROME_BIN, default /usr/bin/google-chrome).
# Output: target/t7/{server-tick.log,frames.jsonl,result.json,server.log,server.ron}.
# The script picks two free ports, so it can run beside another server.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${ROOT}/target/m3-t16}"
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
cd "$ROOT"
OUT="${M3_T7_OUT:-${ROOT}/target/m3-evidence/t16-builder/t7/run-$(date -u +%Y%m%dT%H%M%SZ)-$$}"
DIST="$ROOT/crates/turts_client_web/dist"

cargo build --release -p turts_server
(cd crates/turts_client_web && trunk build --release)

# Serve each big file precompressed. A .br file is made again only when its source is newer.
for file in "$DIST"/*.wasm "$DIST"/*.js; do
    if [ ! -e "$file.br" ] || [ "$file" -nt "$file.br" ]; then
        brotli -q 11 -k -f "$file"
    fi
done

mkdir -p "$OUT/pack"
# `turts-pack` writes `<dir>.tar` next to the directory, so pack a copy: `mods/` stays clean.
cp -r mods/acme "$OUT/pack/acme"
cargo run --release --quiet -p turts_pack --bin turts-pack -- "$OUT/pack/acme"

read -r HTTP_PORT WS_PORT < <(python3 - <<'EOF'
import socket
sockets = [socket.socket() for _ in range(2)]
for s in sockets:
    s.bind(("127.0.0.1", 0))
print(*(s.getsockname()[1] for s in sockets))
EOF
)
cat > "$OUT/server.ron" <<EOF
(
    game_package: "$OUT/pack/acme.tar",
    http_listen: "127.0.0.1:$HTTP_PORT",
    ws_listen: "127.0.0.1:$WS_PORT",
    public_url: "http://127.0.0.1:$HTTP_PORT",
    tick_hz: 20,
    static_dir: "$DIST",
)
EOF

"$CARGO_TARGET_DIR/release/turts_server" "$OUT/server.ron" --tick-log "$OUT/server-tick.log" \
    > "$OUT/server.log" 2>&1 &
SERVER_PID=$!
cleanup() {
    local status=$?
    trap - EXIT INT TERM
    if [[ -n "${SERVER_PID:-}" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
        kill "$SERVER_PID" 2>/dev/null || true
    fi
    if [[ -n "${SERVER_PID:-}" ]]; then
        wait "$SERVER_PID" 2>/dev/null || true
    fi
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

for _ in $(seq 1 100); do
    if curl -fsS "http://127.0.0.1:$HTTP_PORT/advertisement.json" > /dev/null 2>&1; then
        break
    fi
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then
        echo "turts_server exited early:" >&2
        cat "$OUT/server.log" >&2
        exit 1
    fi
    sleep 0.1
done

T7_COMMAND=(node tests/browser/t7.js "http://127.0.0.1:$HTTP_PORT/" "$OUT"
    --ws "ws://127.0.0.1:$WS_PORT" --expect-content-id acme:scout --expect-final 20,8)
if [[ "${M2_HEADLESS:-true}" == "false" ]]; then
    if ! command -v xvfb-run >/dev/null 2>&1; then
        echo "xvfb-run is required when M2_HEADLESS=false" >&2
        exit 1
    fi
    xvfb-run --auto-servernum --server-args='-screen 0 1280x720x24' "${T7_COMMAND[@]}"
else
    "${T7_COMMAND[@]}"
fi

# The oracle reads the two logs while the server still runs: it needs a tick after the last
# frame. T7_DIR names the folder of the logs.
T7_DIR="$OUT" cargo test --release -p turts_server --test t7_oracle -- --ignored --nocapture
