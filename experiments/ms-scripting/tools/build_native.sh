#!/usr/bin/env bash
# One native binary for each of baseline, rhai, rune, lua: out/bin/ms_native-<name>.
# One candidate per binary, as on wasm32 and as in production: no feature unification
# between candidates. Run under a shared CPU lock.
set -euo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
cd "$here"
mkdir -p out/bin
for name in baseline rhai rune lua rune-full; do
    feat=()
    [ "$name" != baseline ] && feat=(--features "$name")
    cargo build --release -p ms_native --no-default-features "${feat[@]}" 2>&1 | tail -n 1
    cp target/release/ms_native "out/bin/ms_native-$name"
done
ls -l out/bin
