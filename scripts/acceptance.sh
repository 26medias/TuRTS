#!/usr/bin/env bash
# Build both public binaries, verify the Task 15 process-test selector, then run
# process acceptance serially in release mode. Existing T5 is repeated afterwards.
set -euo pipefail
cargo build --release -p turts_server
cargo build --release -p turts_client
cargo test --release -p turts_server --test unit_boot -- --ignored --test-threads=1
cargo test --release -p turts_server --test hello -- --ignored --test-threads=1
cargo test --release -p turts_server --test acceptance -- --ignored --test-threads=1 "$@"
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
  cargo test --release -p turts_server --test economy -- --ignored --test-threads=1 --exact "$name"
done
cargo test --release -p turts_server --test acceptance -- --ignored --test-threads=1 --exact t5_convergence
