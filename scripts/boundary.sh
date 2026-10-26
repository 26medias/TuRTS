#!/usr/bin/env bash
# T1 boundary checks: forbidden edges per crate (design 4.1). Run from the repository root.
set -euo pipefail
TMPERR=$(mktemp)
trap 'rm -f "$TMPERR"' EXIT
HOST=$(rustc -vV | sed -n 's/^host: //p')
tree() {
    local crate=$1 target=$2 manifest=${3:-} out rc=0
    out=$(cargo tree -p "$crate" -e normal --prefix none --target "$target" ${manifest:+--manifest-path "$manifest"} 2>"$TMPERR") || { rc=$?; out=$(cat "$TMPERR"); }
    if [ "$rc" -ne 0 ]; then echo "cargo tree failed for $crate: $out" >&2; exit 1; fi
    if [[ "$(head -n1 <<<"$out")" != "$crate v"* ]]; then echo "cargo tree output does not start with $crate" >&2; echo "$out" | head -3 >&2; exit 1; fi
    sed 's/ .*//' <<<"$out" | sort -u
}
names_has() { local rc=0; grep -qx "$2" <<<"$1" || rc=$?; if [ "$rc" -eq 2 ]; then echo "grep failed" >&2; exit 1; fi; return $rc; }
forbid() { local c=$1 t=$2 m=$3; shift 3; local l; l=$(tree "$c" "$t" "$m"); for n in "$@"; do if names_has "$l" "$n"; then echo "forbidden edge: $c -> $n" >&2; exit 1; fi; done; }
nobevy() { local l b; l=$(tree "$1" "$2" "$3"); b=$(grep -m1 '^bevy' <<<"$l" || true); if [ -n "$b" ]; then echo "forbidden edge: $1 -> $b" >&2; exit 1; fi; }
workspace() { local out rc=0 l; out=$(cargo tree --workspace -e normal --prefix none 2>"$TMPERR") || { rc=$?; out=$(cat "$TMPERR"); }; if [ "$rc" -ne 0 ]; then echo "cargo tree failed for workspace: $out" >&2; exit 1; fi; l=$(sed 's/ .*//' <<<"$out" | sort -u); for n in "$@"; do if names_has "$l" "$n"; then echo "forbidden edge: workspace -> $n" >&2; exit 1; fi; done; }
if [ $# -gt 0 ]; then
    RULE= C= T=$HOST M= N=
    while [ $# -gt 0 ]; do case $1 in --rule) RULE=$2;; --crate) C=$2;; --target) T=$2;; --manifest) M=$2;; --names) N=$2;; esac; shift 2; done
    IFS=, read -ra NA <<<"$N"
    case $RULE in forbid) forbid "$C" "$T" "$M" "${NA[@]}";; no-bevy-prefix) nobevy "$C" "$T" "$M";; workspace) workspace "${NA[@]}";; esac
    echo "rule ok"; exit 0
fi
forbid turts_sim $HOST "" bevy_render bevy_winit bevy_pbr turts_mod_runtime
forbid turts_server $HOST "" bevy_render bevy_winit bevy_pbr
nobevy turts_mod_api $HOST ""
nobevy turts_net_protocol $HOST ""
forbid turts_client $HOST "" turts_sim
forbid turts_client wasm32-unknown-unknown "" turts_sim
forbid turts_net_lightyear $HOST "" turts_sim turts_mod_runtime
workspace lightyear_prediction bevy_replicon
FIX=${BOUNDARY_FIXTURE_MANIFEST:-tests/boundary-fixture/Cargo.toml}
PKG=${BOUNDARY_FIXTURE_PACKAGE:-turts_boundary_fixture}
l=$(tree "$PKG" $HOST "$FIX")
names_has "$l" bevy_render || { echo "fixture not red" >&2; exit 1; }
echo "boundary ok"
