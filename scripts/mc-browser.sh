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
[[ -n "$PACKAGE" ]] || { echo "usage: scripts/mc-browser.sh <package-dir> [driver.js oracle.js viewport [--params row.json] [--windows N]] [--serve-only]" >&2; exit 2; }
shift
SERVE_ONLY=false
DRIVER="" ORACLE="" VIEWPORT="1280x720" ROW_FILE="" WINDOWS="" WINDOWS_SET=false
while (($#)); do
    case "$1" in
        --serve-only) SERVE_ONLY=true; shift ;;
        --params) (($# >= 2)) || { echo "--params requires a row file" >&2; exit 2; }; ROW_FILE="$2"; shift 2 ;;
        --windows) (($# >= 2)) || { echo "--windows requires a positive integer" >&2; exit 2; }; [[ "$WINDOWS_SET" == false ]] || { echo "--windows may be specified only once" >&2; exit 2; }; WINDOWS="$2"; WINDOWS_SET=true; shift 2 ;;
        *)
            if [[ -z "$DRIVER" ]]; then DRIVER="$1"; elif [[ -z "$ORACLE" ]]; then ORACLE="$1"; else VIEWPORT="$1"; fi
            shift ;;
    esac
done
if [[ "$WINDOWS_SET" == true ]]; then
    python3 - "$WINDOWS" <<'PYWINDOWS' || { echo "--windows requires a positive integer" >&2; exit 2; }
import re, sys
value = sys.argv[1]
if not re.fullmatch(r"[1-9][0-9]*", value) or int(value) > 9007199254740991:
    raise SystemExit(1)
PYWINDOWS
fi
[[ "$SERVE_ONLY" == true || ( -n "$DRIVER" && -n "$ORACLE" ) ]] || { echo "driver and oracle are required" >&2; exit 2; }
[[ "$BUILD_JOBS" =~ ^[1-4]$ ]] || { echo "CARGO_BUILD_JOBS must be between 1 and 4" >&2; exit 2; }
SYNTHETIC_UNITS=""
SYNTHETIC_SET=false
HD1_NATIVE_REFERENCE=""
if [[ -n "$ROW_FILE" ]]; then
    [[ -f "$ROW_FILE" ]] || { echo "missing row file: $ROW_FILE" >&2; exit 2; }
    SYNTHETIC_UNITS="$(python3 - "$ROOT" "$ROW_FILE" "$PACKAGE" <<'PYSYNTHETIC'
import json, pathlib, sys
sys.dont_write_bytecode = True
sys.path.insert(0, str(pathlib.Path(sys.argv[1]) / "scripts"))
from mc_browser_params import actions_file, synthetic_units, validate_task16_fixture, validate_task16_plan
try:
    row = json.loads(pathlib.Path(sys.argv[2]).read_text())
    value = synthetic_units(row)
    actions_file(row, pathlib.Path(sys.argv[1]) / "tests/browser/fixtures")
    validate_task16_plan(row)
    validate_task16_fixture(row, pathlib.Path(sys.argv[1]), sys.argv[3])
except (ValueError, json.JSONDecodeError) as error:
    print(f"invalid row synthetic_units: {error}", file=sys.stderr)
    raise SystemExit(2)
print("" if value is None else value)
PYSYNTHETIC
)" || exit 2
    [[ -z "$SYNTHETIC_UNITS" ]] || SYNTHETIC_SET=true
    ROW_NAME="$(python3 - "$ROW_FILE" <<'PYROW'
import json, pathlib, sys
print(json.loads(pathlib.Path(sys.argv[1]).read_text()).get("row", ""))
PYROW
)"
    if [[ "$ROW_NAME" =~ ^(KM1|KM2|SEL4|SLOT1|CAM2|SH1|PER1)$ && "$WINDOWS_SET" == true ]]; then
        ROW_WINDOWS="$(python3 - "$ROW_FILE" <<'PYROWCOUNT'
import json, pathlib, sys
print(json.loads(pathlib.Path(sys.argv[1]).read_text()).get("windows", 1))
PYROWCOUNT
)"
        [[ "$WINDOWS" == "$ROW_WINDOWS" ]] || { echo "--windows cannot override the declared Task16 window count" >&2; exit 2; }
    fi
    if [[ "$ROW_NAME" == "HD1" ]]; then
        HD1_NATIVE_REFERENCE="${MC_HD1_NATIVE_REFERENCE_DIR:-}"
        [[ -n "$HD1_NATIVE_REFERENCE" && -d "$HD1_NATIVE_REFERENCE" ]] || { echo "HD1 requires an explicit MC_HD1_NATIVE_REFERENCE_DIR with a native binding" >&2; exit 2; }
    fi
fi
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
if [[ "${ROW_NAME:-}" == "KM1" ]]; then
    RESERVED_PACKAGE="$OUT/km1-reserved-forge"
    cp -a "$PACK/$PACKAGE_NAME" "$RESERVED_PACKAGE"
    PYTHONDONTWRITEBYTECODE=1 python3 - "$RESERVED_PACKAGE" "$OUT/km1-reserved-pack.json" "$TARGET/release/turts-pack" <<'PYKM1PACK'
import hashlib, json, pathlib, subprocess, sys
package, result_path, binary = map(pathlib.Path, sys.argv[1:])
source = package / "input" / "bindings.ron"
data = source.read_bytes()
needle = b'code: "KeyM"'
if data.count(needle) != 1:
    raise SystemExit("KM1 copied Forge source must contain exactly one KeyM default binding")
mutant = data.replace(needle, b'code: "F12"', 1)
source.write_bytes(mutant)
argv = [str(binary), str(package)]
binary_hash = hashlib.sha256(binary.read_bytes()).hexdigest()
completed = subprocess.run(argv, text=True, capture_output=True, check=False)
record = {
    "source_file": "input/bindings.ron",
    "source_sha256": hashlib.sha256(mutant).hexdigest(),
    "reserved_code": "F12",
    "argv": argv,
    "binary_sha256": binary_hash,
    "command_sha256": hashlib.sha256(json.dumps({"argv": argv, "binary_sha256": binary_hash}, sort_keys=True, separators=(",", ":")).encode()).hexdigest(),
    "exit_code": completed.returncode,
    "stdout": completed.stdout,
    "stderr": completed.stderr,
}
pathlib.Path(result_path).write_text(json.dumps(record, sort_keys=True) + "\n")
PYKM1PACK
fi
(cd "$ROOT/crates/turts_client_web" && env CARGO_BUILD_JOBS="$BUILD_JOBS" trunk build --release) >"$OUT/trunk-build.log" 2>&1 || { cat "$OUT/trunk-build.log" >&2; exit 1; }
if [[ -n "$HD1_NATIVE_REFERENCE" ]]; then
    PYTHONDONTWRITEBYTECODE=1 python3 "$ROOT/scripts/hd1_reference.py" validate "$ROOT" "$HD1_NATIVE_REFERENCE"
    mkdir -p "$OUT/native-reference"
    cp -a "$HD1_NATIVE_REFERENCE"/. "$OUT/native-reference"/
fi

if [[ "${ROW_NAME:-}" == "PER1" ]]; then
    PYTHONDONTWRITEBYTECODE=1 python3 "$ROOT/scripts/per1_native_reference.py" create "$ROOT" "$OUT/native-reference"
fi

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
SERVER_ARGS=("$TARGET/release/turts_server" "$OUT/server.ron")
if [[ "${ROW_NAME:-}" == "SLOT1" ]]; then
    SERVER_ARGS+=(--stats "$OUT/server-stats.json")
fi
if [[ "$SYNTHETIC_SET" == true ]]; then
    SERVER_ARGS+=(--synthetic-units "$SYNTHETIC_UNITS")
fi
if [[ "$SYNTHETIC_SET" == false || "$SYNTHETIC_UNITS" == 0 ]]; then
    SERVER_ARGS+=(--tick-log "$OUT/server-tick.log")
fi
python3 - "$OUT/server-argv.json" "${SERVER_ARGS[@]}" <<'PYSERVERARGV'
import json, pathlib, sys
pathlib.Path(sys.argv[1]).write_text(json.dumps(sys.argv[2:], indent=2) + "\n")
PYSERVERARGV
"${SERVER_ARGS[@]}" >"$OUT/server.log" 2>&1 &
SERVER_PID=$!
if [[ "${ROW_NAME:-}" == "SLOT1" ]]; then
    python3 - "$OUT/server-launch.json" "$SERVER_PID" "$OUT/server-stats.json.sessions.jsonl" "${SERVER_ARGS[@]}" <<'PYSERVERLAUNCH'
import hashlib, json, pathlib, sys
path, pid, observer, *argv = sys.argv[1:]
stats_arg = argv[argv.index("--stats") + 1]
record = {"server_pid": int(pid), "stats_arg": stats_arg, "observer_path": str(pathlib.Path(observer).resolve()), "argv": argv}
record["argv_sha256"] = hashlib.sha256(json.dumps(argv, sort_keys=True, separators=(",", ":")).encode()).hexdigest()
pathlib.Path(path).write_text(json.dumps(record, sort_keys=True) + "\n")
pathlib.Path(path).chmod(0o600)
PYSERVERLAUNCH
fi
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
python3 - "$ROOT" "$PACKAGE_NAME" "$URL" "http://127.0.0.1:$HTTP_PORT" "ws://127.0.0.1:$WS_PORT" "$ROW_FILE" "$PARAMS" "$VIEWPORT" "$WINDOWS" <<'PYPARAMS'
import json, pathlib, re, sys
sys.dont_write_bytecode = True
root,package,url,http,ws,row,out,viewport,windows=sys.argv[1:]
sys.path.insert(0, str(pathlib.Path(root) / "scripts"))
from mc_browser_params import add_actions_param, add_synthetic_param, add_task16_param
match=re.fullmatch(r"(\d+)x(\d+)(?:@([0-9]+(?:\.[0-9]+)?))?",viewport)
if not match: raise SystemExit("invalid viewport: "+viewport)
data={"package":package,"url":url,"server_url":http,"ws_url":ws,"windows":1,"device_scale_factor":float(match.group(3) or 1)}
if row:
    row_data=json.loads(pathlib.Path(row).read_text())
    data["windows"]=row_data.get("windows",1)
    data["device_scale_factor"]=row_data.get("device_scale_factor",data["device_scale_factor"])
    data["row"]=row_data.get("row")
    if "instrument_status" in row_data: data["instrument_status"]=row_data["instrument_status"]
    for key in ("fixture_tree_sha256", "fixture_catalog", "pick_world_epsilon", "stages",
                "map_size", "camera_vertical_fov_degrees", "slot_colours", "ping_colour",
                "frame_colour", "background_colour", "minimap_inset", "minimap_size"):
        if key in row_data: data[key]=row_data[key]
    capture=row_data.get("capture_scene_checkpoints",False)
    if not isinstance(capture,bool): raise SystemExit("capture_scene_checkpoints must be a boolean")
    data["capture_scene_checkpoints"]=capture
    data=add_synthetic_param(data,row_data)
    data=add_task16_param(data,row_data)
    if row_data.get("row") not in {"KM1", "KM2", "SEL4", "SLOT1", "CAM2", "SH1", "PER1"}:
        data=add_actions_param(data,row_data,pathlib.Path(root)/"tests/browser/fixtures")
if data.get("row") == "PER1":
    import hashlib
    from per1_native_reference import validate
    reference = pathlib.Path(out).parent / "native-reference"
    validate(pathlib.Path(root).resolve(), reference)
    data["per1_native_reference"] = {
        "validated_before_physical_run": True,
        "files": {file.name: hashlib.sha256(file.read_bytes()).hexdigest()
                  for file in sorted(reference.iterdir()) if file.is_file()},
    }
if windows: data["windows"]=int(windows)
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
RUN_ARGS=("$URL" "$OUT" "$DRIVER_PATH" "$ORACLE_PATH" --params "$PARAMS" --viewport "$VIEWPORT")
[[ "$WINDOWS_SET" == false ]] || RUN_ARGS+=(--windows "$WINDOWS")
node "$ROOT/tests/browser/harness/run.js" "${RUN_ARGS[@]}" | tee "$OUT/browser.log"
