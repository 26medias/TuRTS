#!/usr/bin/env bash
# Build or play the first skirmish against the computer. Use --silent for every agent run.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export TURTS_COMPUTER_SLOT=1
exec bash "$ROOT/scripts/mp1-play.sh" "$@" --package mods/skirmish
