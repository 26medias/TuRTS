#!/usr/bin/env bash
# Builds both binaries with -p, then runs every ignored process test serially in release.
set -euo pipefail
cargo build --release -p turts_server
cargo build --release -p turts_client
cargo test --release -p turts_server --test unit_boot -- --ignored --test-threads=1
cargo test --release -p turts_server --test hello -- --ignored --test-threads=1
cargo test --release -p turts_server --test acceptance -- --ignored --test-threads=1 "$@"
cargo test --release -p turts_server --test acceptance -- --ignored --test-threads=1 --exact t5_convergence
