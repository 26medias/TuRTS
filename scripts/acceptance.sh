#!/usr/bin/env bash
# Build both public binaries, verify the Task 15 process-test selector, then run
# process acceptance serially in release mode. Existing T5 is repeated afterwards.
set -euo pipefail

log_dir="${TASK15_ACCEPTANCE_LOG_DIR:-target/task15-acceptance-logs/$(date -u +%Y%m%dT%H%M%SZ)}"
mkdir -p "$log_dir"
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
    if ! rg -q '^test result: ok\. 1 passed; 0 failed; 0 ignored;' "$log"; then
      printf 'expected exactly one passing test and no failures: %s (log: %s)\n' "$label" "$log" >&2
      return 1
    fi
  else
    if ! rg -q '^test result: ok\. [1-9][0-9]* passed; 0 failed;' "$log"; then
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
  if ! rg -q "^${name}: test$" <<<"$listing"; then
    printf 'missing Task 15 process acceptance test: %s\n' "$name" >&2
    exit 1
  fi
done
selected=$(rg -c '^(e8_commands_are_authoritative_and_acked|e9_two_owners_late_join_and_output_lifecycle|e9_loss_and_small_buffer_converge|e10_cedar_uses_public_pack_and_boot): test$' <<<"$listing")
if [[ "$selected" != 4 ]]; then
  printf 'expected exactly four Task 15 process acceptance tests, found %s\n' "$selected" >&2
  exit 1
fi
for name in "${required[@]}"; do
  run_cargo_test "$name" exact -p turts_server --test economy -- --ignored --test-threads=1 --exact "$name"
done
run_cargo_test t5_convergence exact -p turts_server --test acceptance -- --ignored --test-threads=1 --exact t5_convergence
