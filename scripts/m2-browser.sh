#!/usr/bin/env bash
# Browser WebGPU proof runner. No arguments runs all four public package fixtures.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EXPECTED_TARGET="$(realpath -m "${M2_CARGO_TARGET_DIR:-$ROOT/target}")"
SERVER_BIN="$EXPECTED_TARGET/release/turts_server"
PACKER_BIN="$EXPECTED_TARGET/release/turts-pack"
DIST="$ROOT/crates/turts_client_web/dist"
STATIC_PORT="${M2_STATIC_PORT:-0}"
HTTP_PORT="${M2_HTTP_PORT:-0}"
WS_PORT="${M2_WS_PORT:-0}"
PACKAGE="${M2_PACKAGE_NAME:-}"
CASE_ID="${M2_CASE_ID:-}"
if [[ $# -eq 1 && "$1" == "--list-cases" ]]; then
    printf '%s\n' \
        E9-browser-base-selection \
        E9-browser-base-economy \
        E9-browser-base-recovery \
        E9-browser-base-public-convergence \
        E9-browser-base-loss-conversion \
        E9-browser-base-loss-charge \
        E9-browser-base-loss-refund \
        E9-browser-base-loss-spawn \
        E9-browser-base-loss-final_change \
        E9-browser-haul-economy \
        E9-browser-haul-public-convergence \
        E9-browser-acme-selection \
        E9-browser-acme-public-convergence \
        E9-browser-cedar-economy \
        E9-browser-cedar-public-convergence
    exit 0
fi
if [[ $# -eq 0 && -z "$PACKAGE" && -z "$CASE_ID" ]]; then
    case_list=$("$0" --list-cases)
    selected_count=$(wc -l <<<"$case_list" | tr -d ' ')
    if [[ "$selected_count" != 15 ]]; then
        echo "expected 15 registered M2 browser cases, found $selected_count" >&2
        exit 1
    fi
    for package in base haul acme cedar; do
        "$0" --package "$package"
    done
    for loss_family in conversion charge refund spawn final_change; do
        "$0" --case "E9-browser-base-loss-$loss_family"
    done
    printf 'M2_DEFAULT_SUITE_SUMMARY {"selected_count":%s,"completed_count":%s,"passed_count":%s,"semantic_failed_count":0,"infrastructure_failed_count":0,"timeout_count":0}\n' \
        "$selected_count" "$selected_count" "$selected_count"
    exit 0
fi
while [[ $# -gt 0 ]]; do
    case "$1" in
        --package)
            [[ $# -ge 2 ]] || { echo "--package requires a value" >&2; exit 2; }
            PACKAGE="$2"
            shift 2
            ;;
        --case)
            [[ $# -ge 2 ]] || { echo "--case requires a value" >&2; exit 2; }
            CASE_ID="$2"
            shift 2
            ;;
        *)
            echo "usage: scripts/m2-browser.sh [--package base|haul|acme|cedar] [--case E9-browser-<package>-<case>]" >&2
            exit 2
            ;;
    esac
done
if [[ -z "$PACKAGE" && -n "$CASE_ID" ]]; then
    if [[ "$CASE_ID" =~ ^E9-browser-(base|haul|acme|cedar)- ]]; then
        PACKAGE="${BASH_REMATCH[1]}"
    else
        echo "cannot infer package from browser case id: $CASE_ID" >&2
        exit 2
    fi
fi
if [[ -z "$PACKAGE" ]]; then
    echo "usage: scripts/m2-browser.sh [--package base|haul|acme|cedar] [--case E9-browser-<package>-<case>]" >&2
    exit 2
fi
case "$PACKAGE" in base|haul|acme|cedar) ;; *) echo "unknown package: $PACKAGE" >&2; exit 2 ;; esac
if [[ -n "$CASE_ID" ]]; then
    case "$PACKAGE:$CASE_ID" in
        base:E9-browser-base-selection|base:E9-browser-base-economy|base:E9-browser-base-recovery|base:E9-browser-base-public-convergence|base:E9-browser-base-loss-conversion|base:E9-browser-base-loss-charge|base:E9-browser-base-loss-refund|base:E9-browser-base-loss-spawn|base:E9-browser-base-loss-final_change|haul:E9-browser-haul-economy|haul:E9-browser-haul-public-convergence|acme:E9-browser-acme-selection|acme:E9-browser-acme-public-convergence|cedar:E9-browser-cedar-economy|cedar:E9-browser-cedar-public-convergence) ;;
        *) echo "unknown browser case $CASE_ID for package $PACKAGE" >&2; exit 2 ;;
    esac
fi
ARTIFACT_ROOT="${M2_ARTIFACT_ROOT:-$ROOT/target/m2-browser/runs}"
BUILD_JOBS="${M2_BUILD_JOBS:-2}"
CHROME_BIN="${CHROME_BIN:-$(command -v google-chrome || command -v chromium || true)}"

if [[ -z "$CHROME_BIN" || ! -x "$CHROME_BIN" ]]; then
    echo "Chrome binary unavailable; set CHROME_BIN" >&2
    exit 1
fi
export CHROME_BIN
if [[ ! "$BUILD_JOBS" =~ ^[1-2]$ ]]; then
    echo "M2_BUILD_JOBS must be 1 or 2" >&2
    exit 2
fi
if [[ -L "$ROOT/target" && "$(realpath "$ROOT/target")" != "$EXPECTED_TARGET" ]]; then
    echo "target guard resolves to unexpected path: $(realpath "$ROOT/target")" >&2
    exit 1
fi
mkdir -p "$ARTIFACT_ROOT"
RUN_STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
ARTIFACT_DIR="$(mktemp -d "$ARTIFACT_ROOT/run-${RUN_STAMP}-XXXXXX")"
BUILD_SOURCE_COMMIT="${M2_SOURCE_COMMIT:-$(git -C "$ROOT" rev-parse HEAD)}"
BUILD_SOURCE_BRANCH="${M2_SOURCE_BRANCH:-$(git -C "$ROOT" branch --show-current)}"
BUILD_SOURCE_DIFF_SHA256="${M2_SOURCE_DIFF_SHA256:-$(git -C "$ROOT" diff --binary HEAD | sha256sum | cut -d ' ' -f 1)}"
BUILD_SOURCE_STATUS="${M2_SOURCE_STATUS:-$(git -C "$ROOT" status --porcelain=v1)}"
WORKSPACE_VERSION="$(sed -n '/^version = /s/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"
STATIC_PID=""
SERVER_PID=""

cleanup() {
    local exit_code=$?
    local static_was_alive=false
    local server_was_alive=false
    if [[ -n "$STATIC_PID" ]] && kill -0 "$STATIC_PID" 2>/dev/null; then
        static_was_alive=true
        kill "$STATIC_PID" 2>/dev/null || true
    fi
    if [[ -n "$SERVER_PID" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
        server_was_alive=true
        kill "$SERVER_PID" 2>/dev/null || true
    fi
    if [[ -n "$STATIC_PID" ]]; then wait "$STATIC_PID" 2>/dev/null || true; fi
    if [[ -n "$SERVER_PID" ]]; then wait "$SERVER_PID" 2>/dev/null || true; fi
    if [[ -n "${CASE_ID:-}" && -n "${ARTIFACT_DIR:-}" && -d "$ARTIFACT_DIR" ]]; then
        if [[ ! -f "$ARTIFACT_DIR/browser.log" ]] || ! grep -q '^M2_CASE_SUMMARY ' "$ARTIFACT_DIR/browser.log"; then
            printf 'M2_CASE_START %s\n' "$CASE_ID"
            printf 'M2_CASE_INFRA_FAIL %s %s\n' "$CASE_ID" '"runner exited before a browser case summary"'
            printf 'M2_CASE_SUMMARY {"selected_count":1,"completed_count":1,"passed_count":0,"semantic_failed_count":0,"infrastructure_failed_count":1,"timeout_count":0}\n'
        fi
    fi
    if [[ -n "${ARTIFACT_DIR:-}" && -d "$ARTIFACT_DIR" ]]; then
        cat > "$ARTIFACT_DIR/process-cleanup.json" <<EOF_CLEANUP
{"static_pid":${STATIC_PID:-null},"server_pid":${SERVER_PID:-null},"static_was_alive_at_cleanup":$static_was_alive,"server_was_alive_at_cleanup":$server_was_alive,"cleanup_sent":true}
EOF_CLEANUP
        printf '{"exit_code":%s,"completed_utc":"%s"}\n' "$exit_code" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$ARTIFACT_DIR/completion.json"
    fi
    return "$exit_code"
}
trap cleanup EXIT

printf '{"scope":"Task16 browser package run","package":"%s","case_id":"%s","branch":"%s","source_commit":"%s","started_utc":"%s","artifact_dir":"%s"}\n' \
    "$PACKAGE" "$CASE_ID" "$BUILD_SOURCE_BRANCH" "$BUILD_SOURCE_COMMIT" "$RUN_STAMP" "$ARTIFACT_DIR" > "$ARTIFACT_DIR/run.json"
printf '%s\n' "$BUILD_SOURCE_DIFF_SHA256" > "$ARTIFACT_DIR/source-diff.sha256"
printf '%s\n' "$BUILD_SOURCE_STATUS" > "$ARTIFACT_DIR/source-status.txt"

# All build outputs and compiler caches stay in the caller-selected target directory.
env CARGO_TARGET_DIR="$EXPECTED_TARGET" CARGO_BUILD_JOBS="$BUILD_JOBS" CARGO_NET_OFFLINE=true \
    cargo build --release -p turts_server > "$ARTIFACT_DIR/server-build.log" 2>&1 || {
        cat "$ARTIFACT_DIR/server-build.log" >&2
        exit 1
    }
env CARGO_TARGET_DIR="$EXPECTED_TARGET" CARGO_BUILD_JOBS="$BUILD_JOBS" CARGO_NET_OFFLINE=true \
    cargo build --release -p turts_pack --bin turts-pack > "$ARTIFACT_DIR/packer-build.log" 2>&1 || {
        cat "$ARTIFACT_DIR/packer-build.log" >&2
        exit 1
    }
(cd "$ROOT/crates/turts_client_web" && \
    env CARGO_TARGET_DIR="$EXPECTED_TARGET" CARGO_BUILD_JOBS="$BUILD_JOBS" CARGO_NET_OFFLINE=true \
        trunk build --release) > "$ARTIFACT_DIR/trunk-build.log" 2>&1 || {
        cat "$ARTIFACT_DIR/trunk-build.log" >&2
        exit 1
    }
for required in "$SERVER_BIN" "$PACKER_BIN" "$DIST/index.html" "$DIST"/*.js "$DIST"/*.wasm; do
    if [[ ! -e "$required" ]]; then
        echo "required immutable server/packer or worktree Trunk artifact missing: $required" >&2
        exit 1
    fi
done

PORTS="$(python3 - "$STATIC_PORT" "$HTTP_PORT" "$WS_PORT" <<'PYPORTS'
import socket
import sys
ports = []
sockets = []
for raw in sys.argv[1:]:
    requested = int(raw)
    sock = socket.socket()
    try:
        sock.bind(("127.0.0.1", requested))
        sockets.append(sock)
        ports.append(sock.getsockname()[1])
    except OSError as error:
        raise SystemExit(f"required loopback port {requested} is unavailable: {error}")
    finally:
        if sock not in sockets:
            sock.close()
print("\n".join(map(str, ports)))
for sock in sockets:
    sock.close()
PYPORTS
)"
mapfile -t SELECTED_PORTS <<< "$PORTS"
STATIC_PORT="${SELECTED_PORTS[0]}"
HTTP_PORT="${SELECTED_PORTS[1]}"
WS_PORT="${SELECTED_PORTS[2]}"
printf '{"static_port":%s,"server_http_port":%s,"server_ws_port":%s}\n' \
    "$STATIC_PORT" "$HTTP_PORT" "$WS_PORT" > "$ARTIFACT_DIR/ports.json"

mkdir -p "$ARTIFACT_DIR/pack"
cp -a "$ROOT/mods/$PACKAGE" "$ARTIFACT_DIR/pack/$PACKAGE"
if [[ "$PACKAGE" == "cedar" ]]; then
    python3 - "$ARTIFACT_DIR/pack/cedar/units/depot.ron" "$ARTIFACT_DIR/pack/cedar/units/worker.ron" <<'PYRENAME'
from pathlib import Path
import sys
replacements = {
    Path(sys.argv[1]): ('name: "Timber Yard"', 'name: "Task16 Cedar Yard"'),
    Path(sys.argv[2]): ('name: "Carrier"', 'name: "Task16 Carrier"'),
}
for path, (old, new) in replacements.items():
    text = path.read_text()
    if text.count(old) != 1:
        raise SystemExit(f"expected one {old!r} in {path}")
    path.write_text(text.replace(old, new, 1))
PYRENAME
    printf '{"fixture":"public display labels renamed in packed scratch clone","units":{"cedar:depot":"Task16 Cedar Yard","cedar:worker":"Task16 Carrier"},"content_ids_unchanged":true}\n' > "$ARTIFACT_DIR/package-profile-overrides.json"
fi

if [[ "$CASE_ID" == "E9-browser-base-loss-final_change" ]]; then
    python3 - "$ARTIFACT_DIR/pack/base/maps/flat.ron" <<'PYBLOCKERS'
from pathlib import Path
import sys
path = Path(sys.argv[1])
text = path.read_text()
anchor = "        (unit: \"base:harvester\", owner_slot: 1, pos: (x: 52.0, y: 8.0), contents: []),"
if text.count(anchor) != 1:
    raise SystemExit("expected one final existing placement before appending public output blockers")
blockers = "\n        (unit: \"base:command_center\", owner_slot: 0, pos: (x: 8.0, y: 12.0), contents: []),\n        (unit: \"base:command_center\", owner_slot: 0, pos: (x: 8.0, y: 4.0), contents: []),"
path.write_text(text.replace(anchor, anchor + blockers, 1))
PYBLOCKERS
    printf '%s\n' '{"fixture":"public base map clone with two appended owner-0 Command Center output blockers","placements":[{"unit":"base:command_center","owner_slot":0,"pos":[8,12]},{"unit":"base:command_center","owner_slot":0,"pos":[8,4]}],"prior_placement_ids_preserved":true,"recipe_and_duration_unchanged":true}' > "$ARTIFACT_DIR/package-profile-overrides.json"
fi

FIXTURE_X_OFFSET=0
case "$PACKAGE" in
    base) FIXTURE_X_OFFSET=18; MAP_RELATIVE_PATH="maps/flat.ron" ;;
    haul) FIXTURE_X_OFFSET=18; MAP_RELATIVE_PATH="maps/field.ron" ;;
    cedar) FIXTURE_X_OFFSET=18; MAP_RELATIVE_PATH="maps/clearing.ron" ;;
    acme) MAP_RELATIVE_PATH="" ;;
esac
if [[ "$FIXTURE_X_OFFSET" == 18 ]]; then
    python3 "$ROOT/tests/m2/browser_fixture.py" \
        --source "$ROOT/mods/$PACKAGE/$MAP_RELATIVE_PATH" \
        --packed "$ARTIFACT_DIR/pack/$PACKAGE/$MAP_RELATIVE_PATH" \
        --receipt "$ARTIFACT_DIR/map-fixture-translation.json" \
        --package "$PACKAGE" --case "$CASE_ID" --offset-x "$FIXTURE_X_OFFSET" \
        > "$ARTIFACT_DIR/map-fixture-translation.log" 2>&1 || {
            cat "$ARTIFACT_DIR/map-fixture-translation.log" >&2
            exit 1
        }
else
    printf '{"package":"%s","case_id":"%s","offset_x":0,"map_translation":"none"}\n' \
        "$PACKAGE" "$CASE_ID" > "$ARTIFACT_DIR/map-fixture-translation.json"
fi

"$PACKER_BIN" "$ARTIFACT_DIR/pack/$PACKAGE" > "$ARTIFACT_DIR/pack.log" 2>&1

cat > "$ARTIFACT_DIR/server.ron" <<EOF_SERVER
(
    game_package: "$ARTIFACT_DIR/pack/$PACKAGE.tar",
    http_listen: "127.0.0.1:$HTTP_PORT",
    ws_listen: "127.0.0.1:$WS_PORT",
    public_url: "http://127.0.0.1:$HTTP_PORT",
    tick_hz: 20,
)
EOF_SERVER

PACKAGE_SOURCE_SHA256="$(cd "$ARTIFACT_DIR/pack/$PACKAGE" && find . -type f -print0 | sort -z | xargs -0 sha256sum | sha256sum | cut -d ' ' -f 1)"
cat > "$ARTIFACT_DIR/provenance.json" <<EOF_PROVENANCE
{
  "scope": "Task 16 real browser package proof; not final acceptance until all registered cases pass",
  "package": "$PACKAGE",
  "case_id": "$CASE_ID",
  "branch": "$BUILD_SOURCE_BRANCH",
  "source_commit": "$BUILD_SOURCE_COMMIT",
  "source_diff_sha256": "$BUILD_SOURCE_DIFF_SHA256",
  "source_package_version": "$WORKSPACE_VERSION",
  "source_status_file": "source-status.txt",
  "server_binary_path": "$SERVER_BIN",
  "server_binary_sha256": "$(sha256sum "$SERVER_BIN" | cut -d ' ' -f 1)",
  "packer_binary_path": "$PACKER_BIN",
  "packer_binary_sha256": "$(sha256sum "$PACKER_BIN" | cut -d ' ' -f 1)",
  "target_dir": "$EXPECTED_TARGET",
  "artifact_dir": "$ARTIFACT_DIR",
  "started_utc": "$RUN_STAMP",
  "build_jobs": $BUILD_JOBS,
  "runner_sha256": "$(sha256sum "$ROOT/scripts/m2-browser.sh" | cut -d ' ' -f 1)",
  "chrome_binary": "$CHROME_BIN",
  "build_logs": ["server-build.log", "packer-build.log", "trunk-build.log"],
  "package_source_sha256": "$PACKAGE_SOURCE_SHA256",
  "package_archive_sha256": "$(sha256sum "$ARTIFACT_DIR/pack/$PACKAGE.tar" | cut -d ' ' -f 1)",
  "private_economy_audit_log": "$( [[ "$PACKAGE" == "base" ]] && printf '%s' "$ARTIFACT_DIR/economy-audit-private.jsonl" || true )",
  "dist_dir": "$DIST",
  "static_port": $STATIC_PORT,
  "server_http_port": $HTTP_PORT,
  "server_ws_port": $WS_PORT,
  "chrome": "$($CHROME_BIN --version)",
  "trunk": "$(trunk --version)",
  "rustc": "$(rustc --version)",
  "cargo": "$(cargo --version)",
  "dist_wasm_sha256": "$(sha256sum "$DIST"/*.wasm | cut -d ' ' -f 1)",
  "dist_js_sha256": "$(sha256sum "$DIST"/*.js | cut -d ' ' -f 1)"
}
EOF_PROVENANCE

python3 -m http.server "$STATIC_PORT" --bind 127.0.0.1 --directory "$DIST" > "$ARTIFACT_DIR/static.log" 2>&1 &
STATIC_PID=$!
SERVER_AUDIT_ARGS=()
if [[ "$PACKAGE" == "base" ]]; then
    SERVER_AUDIT_ARGS=(--economy-log "$ARTIFACT_DIR/economy-audit-private.jsonl")
    printf '%s\n' 'Private trusted all-slot economy audit; use only for owner-filtered same-tick comparison.' > "$ARTIFACT_DIR/economy-audit-scope.txt"
fi
"$SERVER_BIN" "$ARTIFACT_DIR/server.ron" "${SERVER_AUDIT_ARGS[@]}" > "$ARTIFACT_DIR/server.log" 2>&1 &
SERVER_PID=$!
printf '{"static_pid":%s,"server_pid":%s}\n' "$STATIC_PID" "$SERVER_PID" > "$ARTIFACT_DIR/processes.json"

ready=false
for _ in $(seq 1 100); do
    if curl -fsS "http://127.0.0.1:$STATIC_PORT/" > /dev/null 2>&1 && \
        curl -fsS "http://127.0.0.1:$HTTP_PORT/advertisement.json" > /dev/null 2>&1; then
        ready=true
        break
    fi
    if ! kill -0 "$STATIC_PID" 2>/dev/null || ! kill -0 "$SERVER_PID" 2>/dev/null; then
        break
    fi
    sleep 0.1
done
if [[ "$ready" != true ]]; then
    echo "base UI smoke services did not become ready; artifacts: $ARTIFACT_DIR" >&2
    cat "$ARTIFACT_DIR/static.log" "$ARTIFACT_DIR/server.log" >&2
    exit 1
fi

printf 'Package: %s\nArtifacts: %s\n' "$PACKAGE" "$ARTIFACT_DIR"
set +e
M2_PACKAGE_NAME="$PACKAGE" \
M2_CASE_ID="$CASE_ID" \
M2_FIXTURE_X_OFFSET="$FIXTURE_X_OFFSET" \
M2_FIXTURE_TRANSLATION_RECEIPT="$ARTIFACT_DIR/map-fixture-translation.json" \
M2_HEADLESS=false \
M2_CHROME_ARGS_JSON="${M2_CHROME_ARGS_JSON:-[\"--enable-gpu\",\"--use-angle=vulkan\",\"--enable-features=Vulkan\",\"--enable-unsafe-webgpu\"]}" \
M2_IGNORE_DEFAULT_ARGS_JSON="${M2_IGNORE_DEFAULT_ARGS_JSON:-[\"--enable-unsafe-swiftshader\"]}" \
xvfb-run --auto-servernum --server-args='-screen 0 1280x720x24' \
    node "$ROOT/tests/browser/m2.js" "http://127.0.0.1:$STATIC_PORT/" "http://127.0.0.1:$HTTP_PORT" "ws://127.0.0.1:$WS_PORT" "$ARTIFACT_DIR" > "$ARTIFACT_DIR/browser.log" 2>&1
BROWSER_STATUS=$?
set -e
cat "$ARTIFACT_DIR/browser.log"
exit "$BROWSER_STATUS"
