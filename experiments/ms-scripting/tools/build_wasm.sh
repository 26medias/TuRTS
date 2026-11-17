#!/usr/bin/env bash
# Build one wasm32 module for each of baseline, rhai, rune, lua (plan Task 4).
# Output: out/wasm/<name>.wasm (after wasm-opt -O3), .wasm.gz (gzip -9 -n), .wasm.br (brotli -q 11),
# out/wasm/sizes.json, and the import list of each module. Run under a shared CPU lock.
set -euo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
cd "$here"
mkdir -p out/wasm
echo "[" > out/wasm/sizes.json.tmp
first=1
for name in baseline rhai rune lua rune-full; do
    feat=()
    [ "$name" != baseline ] && feat=(--features "$name")
    cargo build -p ms_web --profile wasm-release --target wasm32-unknown-unknown "${feat[@]}" 2>&1 | tail -n 2
    raw="target/wasm32-unknown-unknown/wasm-release/ms_web.wasm"
    cp "$raw" "out/wasm/$name.pre-opt.wasm"
    wasm-opt -O3 "out/wasm/$name.pre-opt.wasm" -o "out/wasm/$name.wasm"
    gzip -9 -n -c "out/wasm/$name.wasm" > "out/wasm/$name.wasm.gz"
    brotli -q 11 -c "out/wasm/$name.wasm" > "out/wasm/$name.wasm.br"
    imports="$(python3 tools/wasm_imports.py "out/wasm/$name.wasm")"
    [ $first -eq 0 ] && echo "," >> out/wasm/sizes.json.tmp
    first=0
    printf '{"name":"%s","pre_opt_bytes":%d,"raw_bytes":%d,"gzip_bytes":%d,"brotli_bytes":%d,"imports":%s,"sha256":"%s"}' \
        "$name" "$(stat -c %s "out/wasm/$name.pre-opt.wasm")" "$(stat -c %s "out/wasm/$name.wasm")" \
        "$(stat -c %s "out/wasm/$name.wasm.gz")" "$(stat -c %s "out/wasm/$name.wasm.br")" \
        "$imports" "$(sha256sum "out/wasm/$name.wasm" | cut -d' ' -f1)" >> out/wasm/sizes.json.tmp
done
echo "]" >> out/wasm/sizes.json.tmp
mv out/wasm/sizes.json.tmp out/wasm/sizes.json
cat out/wasm/sizes.json
