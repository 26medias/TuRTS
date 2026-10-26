#!/usr/bin/env bash
# User-level toolchain for this repository. Idempotent. No sudo.
set -euo pipefail
rustup update stable
rustup target add wasm32-unknown-unknown
rustup component add clippy rustfmt
cargo install --locked trunk wasm-bindgen-cli wasm-opt flamegraph cargo-llvm-lines cargo-bloat
echo "done"
