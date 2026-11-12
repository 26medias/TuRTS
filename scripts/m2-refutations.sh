#!/usr/bin/env bash
set -euo pipefail
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
if [[ "$#" != 2 || "$1" != "--case" ]]; then
    echo "usage: bash scripts/m2-refutations.sh --case ID[,ID...]|product|all" >&2
    exit 2
fi
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# The two hosted-only values (see scripts/m2-browser.sh): a local run with one of them set is an error.
if [[ "${GITHUB_ACTIONS:-}" != "true" && ( -n "${M2_LOCAL_ONLY_CASES:-}" || -n "${M2_RECIPE_DURATION_SCALE:-}" ) ]]; then
    echo "scripts/m2-refutations.sh: M2_LOCAL_ONLY_CASES or M2_RECIPE_DURATION_SCALE is set and GITHUB_ACTIONS is not 'true'." >&2
    echo "These two values are for the hosted job only. Unset them for a local gate run, which runs every case with the true recipe times." >&2
    exit 2
fi
# A case listed in M2_LOCAL_ONLY_CASES needs a real GPU and runs in the local gate. The job says so in
# a plain line, a GitHub annotation and $GITHUB_STEP_SUMMARY (tests/m2/local_only.py). The ids that are not listed run.
IFS=',' read -r -a requested <<<"$2"
selected=()
skipped=()
for id in "${requested[@]}"; do
    status=0
    python3 "$ROOT/tests/m2/local_only.py" --decide "$id" || status=$?
    case "$status" in
        0) selected+=("$id") ;;
        10) skipped+=("$id") ;;
        *) exit 2 ;;
    esac
done
run_status=0
if [[ "${#selected[@]}" == 0 ]]; then
    echo "m2-refutations: no case ran on this runner; every selected case is local-only (lines above)"
else
    python3 tests/m2/refutations.py --case "$(IFS=,; echo "${selected[*]}")" || run_status=$?
fi
if [[ "${#skipped[@]}" -gt 0 ]]; then
    echo "::warning title=M2 local-only::${#skipped[@]} case(s) were not run on this runner and need the local GPU gate: $(IFS=,; echo "${skipped[*]}")"
fi
echo "m2-refutations: skipped_local_only_count=${#skipped[@]} ran_count=${#selected[@]}"
exit "$run_status"
