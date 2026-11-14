#!/usr/bin/env bash
# Play against the computer, or use --human to host two human players locally.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
args=()
export TURTS_COMPUTER_SLOT=1
export TURTS_HUMAN_SKIRMISH=""
for arg in "$@"; do
    if [[ "$arg" == --human ]]; then
        export TURTS_COMPUTER_SLOT=""
        export TURTS_HUMAN_SKIRMISH=1
    else
        args+=("$arg")
    fi
done
exec bash "$ROOT/scripts/mp1-play.sh" "${args[@]}" --package mods/skirmish
