#!/usr/bin/env bash
# T7 browser join (design section 7; plan Task 12): the trunk-built client joins the real
# server in headless Chrome, and the segment oracle checks what the browser presented.
# MC task 21: tests/browser/t7-driver.js acts through the harness io only; t7-oracle.js judges.
#
# Needs: cargo, trunk 0.22.0, brotli, node with `npm ci` done in tests/browser, python3, curl,
# and Google Chrome (CHROME_BIN, default /usr/bin/google-chrome).
# Output: $OUT/{server-tick.log,scene-checkpoints.json,input-log.json,frames.jsonl,result.json,
# server.log,server.ron}.
# The script picks two free ports, so it can run beside another server.
#
# Q-PREBUILD: `t7.sh --prepare DIR` builds the server, the wasm client and the oracle test, and packs
# acme, into DIR, then stops (no browser, no GPU lock). `t7.sh --prepared DIR` builds nothing: it
# refuses (exit 66) when DIR does not match the source (scripts/mc_prepare.py). With no flag the
# script builds inside the run, as before (CI).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PREPARE_DIR="" PREPARED=""
while (($#)); do
    case "$1" in
        --prepare) (($# >= 2)) || { echo "--prepare requires a folder" >&2; exit 2; }; PREPARE_DIR="$(realpath -m "$2")"; shift 2 ;;
        --prepared) (($# >= 2)) || { echo "--prepared requires a folder" >&2; exit 2; }; PREPARED="$(realpath -m "$2")"; shift 2 ;;
        *) echo "usage: scripts/t7.sh [--prepare DIR | --prepared DIR]" >&2; exit 2 ;;
    esac
done
[[ -z "$PREPARE_DIR" || -z "$PREPARED" ]] || { echo "--prepare and --prepared cannot be used together" >&2; exit 2; }
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${ROOT}/target/m3-t16}"
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
cd "$ROOT"
OUT="${M3_T7_OUT:-${ROOT}/target/m3-evidence/t16-builder/t7/run-$(date -u +%Y%m%dT%H%M%SZ)-$$}"
DIST="$ROOT/crates/turts_client_web/dist"
SERVER_BIN="$CARGO_TARGET_DIR/release/turts_server"
PACKAGE=mods/acme
# The harness launches CHROME_BIN (tests/browser/harness/run.js); t7.js used /usr/bin/google-chrome.
CHROME_BIN="${CHROME_BIN:-$(command -v google-chrome || command -v chromium || true)}"
if [[ -z "$CHROME_BIN" || ! -x "$CHROME_BIN" ]]; then echo "Chrome binary unavailable; set CHROME_BIN" >&2; exit 1; fi
export CHROME_BIN

if [[ -n "$PREPARED" ]]; then
    # The stale check comes first. A prepared run starts no cargo and no trunk.
    python3 -B scripts/mc_prepare.py check "$PREPARED" --root "$ROOT" --package "$PACKAGE" \
        --need bin/turts_server --need bin/t7_oracle --need dist/index.html --need pack/acme.tar || exit $?
    DIST="$PREPARED/dist"
    SERVER_BIN="$PREPARED/bin/turts_server"
    mkdir -p "$OUT/pack"
    cp -a "$PREPARED/pack/." "$OUT/pack/"
else
    if [[ -n "$PREPARE_DIR" ]]; then
        source "$ROOT/scripts/mc_prepare_lib.sh"
        mc_prepare_begin "$PREPARE_DIR" "$ROOT" "$PACKAGE"
        OUT="$PREPARE_DIR"
    fi
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
fi
if [[ -n "$PREPARE_DIR" ]]; then
    # The oracle test is built here too, so the run starts the binary and not cargo.
    ORACLE_EXE="$(cargo test --release -p turts_server --test t7_oracle --no-run --message-format=json | python3 -c '
import json, sys
for line in sys.stdin:
    if not line.startswith("{"): continue
    entry = json.loads(line)
    if entry.get("reason") == "compiler-artifact" and entry.get("target", {}).get("name") == "t7_oracle" and entry.get("executable"):
        print(entry["executable"])
')"
    [[ -x "$ORACLE_EXE" ]] || { echo "t7 prepare: cargo gave no t7_oracle test executable" >&2; exit 1; }
    mkdir -p "$OUT/bin" "$OUT/dist"
    cp "$ORACLE_EXE" "$OUT/bin/t7_oracle"
    cp "$SERVER_BIN" "$OUT/bin/turts_server"
    cp -a "$DIST"/. "$OUT/dist"/
    python3 -B scripts/mc_prepare.py seal --root "$ROOT" --out "$OUT" --package "$PACKAGE"
    echo "PREPARED: $OUT"
    exit 0
fi

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

"$SERVER_BIN" "$OUT/server.ron" --tick-log "$OUT/server-tick.log" \
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

# MC task 21: the pixels-only pair t7-driver.js + t7-oracle.js runs in the harness (QA0 design
# 4.6 option C). The page is the server's own HTTP address, as before.
PAGE_URL="$(python3 - "http://127.0.0.1:$HTTP_PORT" "ws://127.0.0.1:$WS_PORT" <<'EOF'
from urllib.parse import urlencode
import sys
print(sys.argv[1] + "/?" + urlencode({"server": sys.argv[1], "ws": sys.argv[2]}))
EOF
)"
python3 - "$OUT/params.json" "$PAGE_URL" "http://127.0.0.1:$HTTP_PORT" "ws://127.0.0.1:$WS_PORT" <<'EOF'
import json, pathlib, sys
out, url, http, ws = sys.argv[1:]
pathlib.Path(out).write_text(json.dumps({
    "row": "T7", "package": "acme", "url": url, "server_url": http, "ws_url": ws, "windows": 1,
    "capture_scene_checkpoints": True, "expect_content_id": "acme:scout",
    "viewport": {"width": 1280, "height": 720, "device_scale_factor": 1},
}, sort_keys=True) + "\n")
EOF
# The M2_* names come from CI. The MC_* names come from the headed wrapper (mc_headed_v4.sh),
# and they hold `--alsa-output-device=null`. An MC_* value that is already set wins, so T7 keeps
# the wrapper's arguments. With no MC_* value, the M2_* value is used, as before.
[[ -z "${M2_CHROME_ARGS_JSON:-}" || -n "${MC_CHROME_ARGS_JSON:-}" ]] \
    || export MC_CHROME_ARGS_JSON="$M2_CHROME_ARGS_JSON"
[[ -z "${M2_IGNORE_DEFAULT_ARGS_JSON:-}" || -n "${MC_CHROME_IGNORE_DEFAULT_ARGS_JSON:-}" ]] \
    || export MC_CHROME_IGNORE_DEFAULT_ARGS_JSON="$M2_IGNORE_DEFAULT_ARGS_JSON"
T7_COMMAND=(node tests/browser/harness/run.js "$PAGE_URL" "$OUT"
    tests/browser/t7-driver.js tests/browser/t7-oracle.js --params "$OUT/params.json" --viewport 1280x720)
if [[ "${M2_HEADLESS:-true}" == "false" ]]; then
    if ! command -v xvfb-run >/dev/null 2>&1; then
        echo "xvfb-run is required when M2_HEADLESS=false" >&2
        exit 1
    fi
    MC_BROWSER_HEADED=1 xvfb-run --auto-servernum --server-args='-screen 0 1280x720x24' "${T7_COMMAND[@]}" | tee "$OUT/browser.log"
else
    "${T7_COMMAND[@]}" | tee "$OUT/browser.log"
fi
# The segment oracle reads frames.jsonl and result.json, made from the harness checkpoints.
node tests/browser/t7-frames.js "$OUT" "$("$CHROME_BIN" --version)"

# The oracle reads the two logs while the server still runs: it needs a tick after the last
# frame. T7_DIR names the folder of the logs.
if [[ -n "$PREPARED" ]]; then
    (cd crates/turts_server && T7_DIR="$OUT" "$PREPARED/bin/t7_oracle" --ignored --nocapture)
else
    T7_DIR="$OUT" cargo test --release -p turts_server --test t7_oracle -- --ignored --nocapture
fi
