#!/usr/bin/env bash
# Build both public binaries, verify the Task 15 process-test selector, then run
# process acceptance serially in release mode. Existing T5 is repeated afterwards.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${ROOT}/target/m3-t16}"
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0

log_dir="${M3_ACCEPTANCE_LOG_DIR:-${ROOT}/target/m3-evidence/t16-builder/acceptance/$(date -u +%Y%m%dT%H%M%SZ)}"
mkdir -p "$log_dir"
retain_f11_artifacts() {
  local source="$CARGO_TARGET_DIR/tmp/f11-m3-private-research"
  local destination="$log_dir/f11-evidence"
  if [[ ! -d "$source" ]]; then
    printf 'F11 artifact directory missing: %s\n' "$source" >&2
    return 1
  fi
  mkdir -p "$destination"
  cp -a "$source/." "$destination/"
  local server_evidence="$destination/server-process"
  local server_dir_file="$server_evidence/server-source-directory.txt"
  if [[ ! -f "$server_dir_file" ]]; then
    printf 'F11 spawned server directory marker missing: %s\n' "$server_dir_file" >&2
    return 1
  fi
  local server_dir
  server_dir="$(cat "$server_dir_file")"
  case "$server_dir" in
    "$CARGO_TARGET_DIR"/tmp/server_*) ;;
    *) printf 'F11 server directory is outside the private target tmp root: %s\n' "$server_dir" >&2; return 1 ;;
  esac
  if [[ ! -f "$server_dir/server.log" ]]; then
    printf 'F11 server log missing in identified server directory: %s\n' "$server_dir" >&2
    return 1
  fi
  cp -a "$server_dir/." "$server_evidence/"
  printf 'server_source_directory=%s\nretained_server_log=server-process/server.log\n' "$server_dir" > "$destination/server-directory-provenance.txt"
  git -C "$ROOT" diff --binary HEAD > "$destination/source.diff"
  {
    printf 'source_head=%s\n' "$(git -C "$ROOT" rev-parse HEAD)"
    printf 'source_branch=%s\n' "$(git -C "$ROOT" branch --show-current)"
    printf 'source_status_begin\n'
    git -C "$ROOT" status --porcelain=v1
    printf 'source_status_end\n'
    printf 'source_diff_sha256='
    sha256sum "$destination/source.diff" | cut -d ' ' -f 1
    printf 'command=cargo test --release -p turts_server --test m3_process -- --ignored --test-threads=1 --exact f11_private_research_coordinator_releases_ordered_opponent_move_and_health_trace\n'
    printf 'binary_hashes_begin\n'
    for binary in "$CARGO_TARGET_DIR/release/turts_server" "$CARGO_TARGET_DIR/release/turts_client"; do
      if [[ -f "$binary" ]]; then sha256sum "$binary"; fi
    done
    find "$CARGO_TARGET_DIR/release/deps" -maxdepth 1 -type f -name 'm3_process-*' -perm /111 -print0 | sort -z | xargs -0 -r sha256sum
    printf 'binary_hashes_end\n'
  } > "$destination/provenance.txt"
  find "$destination" -type f ! -name artifact-sha256.txt -print0 | sort -z | xargs -0 -r sha256sum > "$destination/artifact-sha256.txt"
  printf 'F11 artifacts retained at %s\n' "$destination"
}

run_cargo_test() {
  local label="$1"
  local expected="$2"
  shift 2
  local log="$log_dir/$label.log"
  if ! cargo test --release "$@" 2>&1 | tee "$log"; then
    printf 'cargo test failed: %s (log: %s)\n' "$label" "$log" >&2
    return 1
  fi
  if [[ "$expected" == exact ]]; then
    if ! grep -Eq '^test result: ok\. 1 passed; 0 failed; 0 ignored;' "$log"; then
      printf 'expected exactly one passing test and no failures: %s (log: %s)\n' "$label" "$log" >&2
      return 1
    fi
  else
    if ! grep -Eq '^test result: ok\. [1-9][0-9]* passed; 0 failed;' "$log"; then
      printf 'expected at least one passing test and no failures: %s (log: %s)\n' "$label" "$log" >&2
      return 1
    fi
  fi
}

cargo build --release -p turts_server
cargo build --release -p turts_client
run_cargo_test unit_boot aggregate -p turts_server --test unit_boot -- --ignored --test-threads=1
run_cargo_test hello aggregate -p turts_server --test hello -- --ignored --test-threads=1
run_cargo_test acceptance aggregate -p turts_server --test acceptance -- --ignored --test-threads=1 "$@"
listing=$(cargo test --release -p turts_server --test economy -- --list)
required=(
  e8_commands_are_authoritative_and_acked
  e9_two_owners_late_join_and_output_lifecycle
  e9_loss_and_small_buffer_converge
  e10_cedar_uses_public_pack_and_boot
)
for name in "${required[@]}"; do
  if ! grep -Eq "^${name}: test$" <<<"$listing"; then
    printf 'missing Task 15 process acceptance test: %s\n' "$name" >&2
    exit 1
  fi
done
selected=$(grep -Ec '^(e8_commands_are_authoritative_and_acked|e9_two_owners_late_join_and_output_lifecycle|e9_loss_and_small_buffer_converge|e10_cedar_uses_public_pack_and_boot): test$' <<<"$listing")
if [[ "$selected" != 4 ]]; then
  printf 'expected exactly four Task 15 process acceptance tests, found %s\n' "$selected" >&2
  exit 1
fi
for name in "${required[@]}"; do
  run_cargo_test "$name" exact -p turts_server --test economy -- --ignored --test-threads=1 --exact "$name"
done
run_cargo_test t5_convergence exact -p turts_server --test acceptance -- --ignored --test-threads=1 --exact t5_convergence
listing_m3=$(cargo test --release -p turts_server --test m3_process -- --list)
if ! grep -Eq '^f11_private_research_coordinator_releases_ordered_opponent_move_and_health_trace: test$' <<<"$listing_m3"; then
  echo "missing Task 16 F11 process acceptance test" >&2
  exit 1
fi
selected_m3=$(grep -Ec '^f11_private_research_coordinator_releases_ordered_opponent_move_and_health_trace: test$' <<<"$listing_m3")
if [[ "$selected_m3" != 1 ]]; then
  printf 'expected exactly one F11 process test, found %s\n' "$selected_m3" >&2
  exit 1
fi
f11_status=0
TURTS_SERVER_EVIDENCE_DIR="$log_dir/f11-evidence/server-process" run_cargo_test f11_private_research exact -p turts_server --test m3_process -- --ignored --test-threads=1 --exact f11_private_research_coordinator_releases_ordered_opponent_move_and_health_trace || f11_status=$?
retain_f11_artifacts || f11_status=1
if [[ "$f11_status" != 0 ]]; then exit "$f11_status"; fi
