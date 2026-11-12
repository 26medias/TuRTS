#!/usr/bin/env bash
# L0-CORE purity proof. usage: bash docs/research/mp1/l0-core/run_proof.sh [OLD_COMMIT [NEW_COMMIT]]
# OLD_COMMIT defaults to 9044820 (the tree before L0-CORE); NEW_COMMIT defaults to HEAD.
# 1. Core::step: token equality of the old body and the new body with each method call inlined,
#    with the arguments checked against the parameter names and the hooks checked in place.
# 2. the self-test of that proof (twelve mutations, each must be DIFFERENT).
# 3. the move of the arm CancelProduction into cancel::activate.
# 4. nothing in the production source of turts_interaction sets `taking_input` (no assignment, no
#    literal, no manual Default: `taking_input.sh`), so the intercept hook returns `false` (its body
#    is `core.placement.taking_input`, checked in step 1). The test
#    `hook_tests::intercept_stub_takes_no_event_in_a_fresh_core` owns the dynamic case.
# 5. the test functions: none removed, the added ones listed.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
repo=$(git -C "$here" rev-parse --show-toplevel)
old=${1:-9044820}
new=${2:-HEAD}
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
lib=crates/turts_interaction/src
mkdir -p "$tmp/src"
git -C "$repo" show "$old:$lib/lib.rs" > "$tmp/old.rs"
for f in lib placement hints alerts; do git -C "$repo" show "$new:$lib/$f.rs" > "$tmp/src/$f.rs"; done
opts=(--hook "placement::update=$tmp/src/placement.rs" --hook "hints::update=$tmp/src/hints.rs"
    --hook "alerts::update=$tmp/src/alerts.rs" --intercept "placement::intercept=$tmp/src/placement.rs"
    --arm-hook "Placement=placement::on_input=$tmp/src/placement.rs"
    --arm-hook "Hint=hints::on_input=$tmp/src/hints.rs" --arm-hook "Alert=alerts::on_input=$tmp/src/alerts.rs")
status=0
echo "== Core::step"
python3 -I -B "$here/step_equal.py" --old "$tmp/old.rs" --new "$tmp/src/lib.rs" "${opts[@]}" || status=1
echo "== self-test of the proof"
python3 -I -B "$here/step_equal.py" --self-test --old "$tmp/old.rs" --new "$tmp/src/lib.rs" "${opts[@]}" || status=1
echo "== move of the CancelProduction arm"
git -C "$repo" show "$old:crates/turts_client/src/widgets/activation.rs" > "$tmp/activation_old.rs"
git -C "$repo" show "$new:crates/turts_client/src/widgets/activation/cancel.rs" > "$tmp/cancel.rs"
python3 -I -B "$here/move_equal.py" "$tmp/activation_old.rs" "$tmp/cancel.rs" || status=1
echo "== taking_input is never set in the production source"
bash "$here/taking_input.sh" "$new" || status=1
echo "== test functions"
(cd "$repo" && python3 -I -B "$here/static_tests.py" "$old" "$new") || status=1
exit $status
