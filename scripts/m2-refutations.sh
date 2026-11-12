#!/usr/bin/env bash
set -euo pipefail
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
if [[ "$#" != 2 || "$1" != "--case" ]]; then
    echo "usage: bash scripts/m2-refutations.sh --case ID[,ID...]|product|all" >&2
    exit 2
fi
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# A case listed in M2_LOCAL_ONLY_CASES needs a real GPU and runs in the local gate. The job says so in
# a plain line and in $GITHUB_STEP_SUMMARY (tests/m2/local_only.py). The ids that are not listed run.
IFS=',' read -r -a requested <<<"$2"
selected=()
for id in "${requested[@]}"; do
    status=0
    python3 "$ROOT/tests/m2/local_only.py" --decide "$id" || status=$?
    case "$status" in
        0) selected+=("$id") ;;
        10) ;;
        *) exit 2 ;;
    esac
done
if [[ "${#selected[@]}" == 0 ]]; then
    echo "m2-refutations: no case ran on this runner; every selected case is local-only (lines above)"
    exit 0
fi
python3 tests/m2/refutations.py --case "$(IFS=,; echo "${selected[*]}")"
