#!/usr/bin/env bash
set -euo pipefail
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
if [[ "$#" != 2 || "$1" != "--case" ]]; then
    echo "usage: bash scripts/m3-refutations.sh --case ID[,ID...]|product|all" >&2
    exit 2
fi
python3 tests/m3/refutations.py --case "$2"
