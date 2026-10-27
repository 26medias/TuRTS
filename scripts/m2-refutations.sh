#!/usr/bin/env bash
set -euo pipefail
if [[ "$#" != 2 || "$1" != "--case" ]]; then
    echo "usage: bash scripts/m2-refutations.sh --case ID[,ID...]|product|all" >&2
    exit 2
fi
python3 tests/m2/refutations.py --case "$2"
