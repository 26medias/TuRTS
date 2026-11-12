#!/usr/bin/env bash
# L0-CORE: the intercept hook returns `false` until `UI-PLACE` builds it.
# usage: bash docs/research/mp1/l0-core/taking_input.sh [REV]    (no REV: the working tree)
# The only lines of the production source of turts_interaction that may name `taking_input` are the
# field declaration `pub taking_input: bool,`, the body of `placement::intercept`
# (`core.placement.taking_input`), and comments. Any other line (`taking_input = true`,
# `taking_input: true` in a literal or in a manual `Default`, a `.taking_input` write) is refused.
# `placement::State` derives `Default`, and a `bool` defaults to `false`. The tests own the dynamic
# case: `hook_tests::intercept_stub_takes_no_event_in_a_fresh_core` is red for a core that starts
# with the flag set.
set -eu
repo=$(git rev-parse --show-toplevel)
src=crates/turts_interaction/src
if [ $# -ge 1 ]; then
    lines=$(git -C "$repo" grep -n 'taking_input' "$1" -- "$src" ':!*hook_tests.rs' | sed "s/^$1://" || true)
else
    lines=$(cd "$repo" && grep -rn 'taking_input' "$src" --include=*.rs --exclude=hook_tests.rs || true)
fi
bad=$(printf '%s\n' "$lines" | grep -v '^$' \
    | grep -vE ':[0-9]+:\s*//' \
    | grep -vE ':[0-9]+:\s*pub taking_input: bool,\s*$' \
    | grep -vE ':[0-9]+:\s*core\.placement\.taking_input\s*$' || true)
if [ -n "$bad" ]; then
    printf '%s\n' "$bad"
    echo "TAKING-INPUT-USED: a line other than the declaration, the intercept body and comments"
    exit 1
fi
echo "taking_input appears only in its declaration, the intercept body and comments (RESULT OK)"
