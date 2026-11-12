#!/usr/bin/env bash
# L0-SPLIT statement-level proof. usage: bash docs/research/mp1/l0-split/run_proof.sh [OLD_COMMIT [NEW_COMMIT]]
# The old text is read from OLD_COMMIT (default 149f1ed, the oracle commit: the code before the moves).
# The new text is read from NEW_COMMIT (default HEAD). Both go to a temporary directory.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
repo=$(git -C "$here" rev-parse --show-toplevel)
old=${1:-149f1ed}
new=${2:-HEAD}
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
src=crates/turts_client/src
for side in old new; do
    commit=$old; [ "$side" = new ] && commit=$new
    mkdir -p "$tmp/$side/visuals" "$tmp/$side/core_adapter"
    for f in visuals.rs visuals/health.rs visuals/markers.rs visuals/materials.rs visuals/world_tooltip.rs \
        core_adapter.rs; do
        git -C "$repo" show "$commit:$src/$f" > "$tmp/$side/$f"
    done
done
for f in session hover rings order_markers blocked queued; do
    git -C "$repo" show "$new:$src/visuals/$f.rs" > "$tmp/new/visuals/$f.rs"
done
git -C "$repo" show "$new:$src/core_adapter/effects.rs" > "$tmp/new/core_adapter/effects.rs"
o=$tmp/old; n=$tmp/new
status=0
echo "== move A: draw_world_visuals"
python3 -B "$here/stmt_equal.py" --old "$o/visuals.rs" "$o/visuals/health.rs" "$o/visuals/markers.rs" \
    "$o/visuals/materials.rs" "$o/visuals/world_tooltip.rs" --caller draw_world_visuals \
    --new "$n/visuals.rs" "$n/visuals/health.rs" "$n/visuals/session.rs" "$n/visuals/hover.rs" \
    "$n/visuals/rings.rs" "$n/visuals/order_markers.rs" "$n/visuals/blocked.rs" "$n/visuals/queued.rs" \
    --glue "$here/glue_visuals.json" || status=1
echo "== move B: drive_core_adapter"
python3 -B "$here/stmt_equal.py" --old "$o/core_adapter.rs" --caller drive_core_adapter \
    --new "$n/core_adapter.rs" "$n/core_adapter/effects.rs" || status=1
exit $status
