#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
: "${CARGO_TARGET_DIR:?CARGO_TARGET_DIR must name the assigned lane cache}"
export CARGO_TARGET_DIR
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"
# Plain cargo output. With CARGO_TERM_COLOR=always (hosted runners, rust-cache) an escape code sits
# between "Finished" and "`release` profile", and hd1_reference.py rejects the server build log
# ("locked release server build did not complete successfully", exit 2; hosted run 43174011989).
export CARGO_TERM_COLOR=never
[[ "$CARGO_BUILD_JOBS" =~ ^[1-4]$ ]] || { echo "CARGO_BUILD_JOBS must be 1..4" >&2; exit 2; }
cd "$ROOT"
CARGO_TARGET_DIR="$(PYTHONDONTWRITEBYTECODE=1 python3 scripts/hd1_reference.py target "$ROOT")"
export CARGO_TARGET_DIR
EVIDENCE="${MC_HD1_NATIVE_REFERENCE_DIR:-}"
[[ -n "$EVIDENCE" ]] || { echo "MC_HD1_NATIVE_REFERENCE_DIR is required" >&2; exit 2; }
[[ ! -e "$EVIDENCE" ]] || { echo "native reference output already exists: $EVIDENCE" >&2; exit 2; }
mkdir -p "$EVIDENCE"
RUN_LOG="$EVIDENCE/native-process.log"
SERVER_BUILD_LOG="$EVIDENCE/server-build.log"
RUN_DIR="$CARGO_TARGET_DIR/tmp/hd1"

set +e
cargo build --release -p turts_server --locked 2>&1 | tee "$SERVER_BUILD_LOG"
server_build_status=${PIPESTATUS[0]}
set -e
printf 'exit=%s\n' "$server_build_status" | tee "$EVIDENCE/server-build-status.txt"
if [[ "$server_build_status" -ne 0 ]]; then exit "$server_build_status"; fi

PYTHONDONTWRITEBYTECODE=1 python3 "$ROOT/scripts/hd1_reference.py" snapshot "$ROOT" "$EVIDENCE/source-before.json"
python3 - "$EVIDENCE/launcher.json" <<'PYLAUNCHER'
import json, os, pathlib, sys
keys = ("CARGO_TARGET_DIR", "CARGO_PROFILE_DEV_DEBUG", "CARGO_PROFILE_TEST_DEBUG", "CARGO_INCREMENTAL", "CARGO_BUILD_JOBS", "RUSTFLAGS", "CHROME_BIN")
data = {
    "argv": ["cargo", "test", "--release", "-p", "turts_client", "--test", "hd1", "headless_actions_drive_core_and_report_per_unit_results_on_muster", "--locked", "--", "--ignored", "--exact", "--test-threads=1"],
    "server_build_argv": ["cargo", "build", "--release", "-p", "turts_server", "--locked"],
    "environment": {key: os.environ[key] for key in keys if key in os.environ},
    "working_directory": os.getcwd(),
}
pathlib.Path(sys.argv[1]).write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")
PYLAUNCHER

set +e
cargo test --release -p turts_client --test hd1 headless_actions_drive_core_and_report_per_unit_results_on_muster --locked -- --ignored --exact --test-threads=1 2>&1 | tee "$RUN_LOG"
test_status=${PIPESTATUS[0]}
set -e
printf 'exit=%s\n' "$test_status" | tee "$EVIDENCE/native-process-status.txt"
PYTHONDONTWRITEBYTECODE=1 python3 "$ROOT/scripts/hd1_reference.py" snapshot "$ROOT" "$EVIDENCE/source-after.json"
if ! cmp -s "$EVIDENCE/source-before.json" "$EVIDENCE/source-after.json"; then
    echo "source changed while native HD1 process test was running" >&2
    exit 2
fi
if [[ "$test_status" -ne 0 ]]; then exit "$test_status"; fi
PYTHONDONTWRITEBYTECODE=1 python3 "$ROOT/scripts/hd1_reference.py" create "$ROOT" "$RUN_DIR" "$EVIDENCE/reference" "$EVIDENCE"
cp "$RUN_LOG" "$EVIDENCE/reference/native-process.log"
cp "$EVIDENCE/native-process-status.txt" "$EVIDENCE/reference/native-process-status.txt"
cp "$SERVER_BUILD_LOG" "$EVIDENCE/reference/server-build.log"
cp "$EVIDENCE/server-build-status.txt" "$EVIDENCE/reference/server-build-status.txt"
cp "$EVIDENCE/launcher.json" "$EVIDENCE/reference/launcher.json"
cp "$EVIDENCE/source-before.json" "$EVIDENCE/reference/source-before.json"
cp "$EVIDENCE/source-after.json" "$EVIDENCE/reference/source-after.json"
PYTHONDONTWRITEBYTECODE=1 python3 "$ROOT/scripts/hd1_reference.py" validate "$ROOT" "$EVIDENCE/reference"
echo "Native HD1 reference: $EVIDENCE/reference"
