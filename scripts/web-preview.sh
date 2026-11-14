#!/usr/bin/env bash
# Build a browser smoke preview without the delivery profile's whole-program optimisation.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
[[ $# == 1 ]] || { echo 'usage: scripts/web-preview.sh OUTPUT_DIRECTORY' >&2; exit 2; }
output="$(realpath -m "$1")"
entry="$(mktemp "$root/crates/turts_client_web/.preview-XXXXXX.html")"
trap 'rm -f "$entry"' EXIT
python3 - "$root/crates/turts_client_web/index.html" "$entry" <<'PY'
from pathlib import Path
import sys
source=Path(sys.argv[1]).read_text()
assert 'data-cargo-profile="wasm-release"' in source
source=source.replace('data-cargo-profile="wasm-release"','data-cargo-profile="wasm-preview"').replace('data-wasm-opt="3"','data-wasm-opt="0"')
Path(sys.argv[2]).write_text(source)
PY
cd "$root/crates/turts_client_web"
trunk build "$entry" --release --dist "$output"
if [[ -f "$output/$(basename "$entry")" ]]; then mv "$output/$(basename "$entry")" "$output/index.html"; fi
[[ -f "$output/index.html" ]]
