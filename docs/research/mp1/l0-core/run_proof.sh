#!/usr/bin/env bash
# L0-CORE purity proof. usage: bash docs/research/mp1/l0-core/run_proof.sh [OLD_COMMIT [NEW_COMMIT]]
# OLD_COMMIT defaults to 9044820 (the tree before L0-CORE); NEW_COMMIT defaults to HEAD.
# 1. Core::step: token equality of the old body and the new body with each method call inlined.
# 2. the self-test of that proof (four mutations, each must be DIFFERENT).
# 3. the test functions: none removed, the added ones listed.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
repo=$(git -C "$here" rev-parse --show-toplevel)
old=${1:-9044820}
new=${2:-HEAD}
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
lib=crates/turts_interaction/src
git -C "$repo" show "$old:$lib/lib.rs" > "$tmp/old.rs"
git -C "$repo" show "$new:$lib/lib.rs" > "$tmp/new.rs"
for hook in placement hints alerts; do git -C "$repo" show "$new:$lib/$hook.rs" > "$tmp/$hook.rs"; done
hooks=(--hook "placement::update=$tmp/placement.rs" --hook "hints::update=$tmp/hints.rs" --hook "alerts::update=$tmp/alerts.rs")
status=0
echo "== Core::step"
python3 -I -B "$here/step_equal.py" --old "$tmp/old.rs" --new "$tmp/new.rs" "${hooks[@]}" || status=1
echo "== self-test of the proof"
python3 -I -B "$here/step_equal.py" --self-test --old "$tmp/old.rs" --new "$tmp/new.rs" "${hooks[@]}" || status=1
echo "== test functions"
(cd "$repo" && python3 -I -B "$here/static_tests.py" "$old" "$new") || status=1
exit $status
