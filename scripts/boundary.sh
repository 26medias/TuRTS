#!/usr/bin/env bash
# T1 boundary checks: forbidden edges per crate (design 4.1). Run from the repository root.
# The full run prints one "checked:" line on stdout for each rule that passed, then "boundary ok".
#
# Limit (M1 verify): this script checks dependency edges only. The no-content check is the grep
# in crates/turts_checks/tests/no_content.rs, and it names only `base`, `acme`, `rifleman` and
# `mods/`. Content of a new package (for example a new unit name) passes that grep. T2a (each
# package spawns only what its own map places) is the backstop for such content.
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
forbid() { local c=$1 t=$2 m=$3; shift 3; local l; l=$(tree "$c" "$t" "$m"); for n in "$@"; do if names_has "$l" "$n"; then echo "forbidden edge: $c -> $n" >&2; exit 1; fi; done; checked "$c $t: no $*"; }
checked() { if [ -z "$RULE_MODE" ]; then echo "checked: $1"; fi; }
nobevy() { local l b; l=$(tree "$1" "$2" "$3"); b=$(grep -m1 '^bevy' <<<"$l" || true); if [ -n "$b" ]; then echo "forbidden edge: $1 -> $b" >&2; exit 1; fi; checked "$1 $2: no bevy*"; }
workspace() { local out rc=0 l; out=$(cargo tree --workspace -e normal --prefix none 2>"$TMPERR") || { rc=$?; out=$(cat "$TMPERR"); }; if [ "$rc" -ne 0 ]; then echo "cargo tree failed for workspace: $out" >&2; exit 1; fi; l=$(sed 's/ .*//' <<<"$out" | sort -u); for n in "$@"; do if names_has "$l" "$n"; then echo "forbidden edge: workspace -> $n" >&2; exit 1; fi; done; checked "workspace: no $*"; }
# Feature leak (Q1b review follow-up, finding 2). The cargo feature `test-probes` of `turts_sim`
# (the movement probes) is for tests only. The shipped host and client builds must not turn it
# on. `probes_in CRATE EDGES` prints present, absent, or unreachable (the crate has no edge to
# turts_sim, so `cargo tree -i turts_sim` matches no package). The edge kinds EDGES go to
# `cargo tree -e`; the feature rows need `features` too.
probes_in() {
    local crate=$1 edges=$2 out rc=0 g=0
    out=$(cargo tree -p "$crate" -e "$edges,features" -i turts_sim --locked 2>"$TMPERR") || rc=$?
    if [ "$rc" -ne 0 ]; then
        g=0; grep -q 'did not match any packages' "$TMPERR" || g=$?
        if [ "$g" -eq 0 ]; then echo unreachable; return 0; fi
        echo "cargo tree failed for $crate: $(cat "$TMPERR")" >&2; exit 1
    fi
    g=0; grep -qF 'turts_sim feature "test-probes"' <<<"$out" || g=$?
    if [ "$g" -eq 2 ]; then echo "grep failed" >&2; exit 1; fi
    if [ "$g" -eq 0 ]; then echo present; else echo absent; fi
}
noprobes() {
    local state; state=$(probes_in "$1" "$2")
    if [ "$state" = present ]; then echo "forbidden feature: test-probes in $1 (edges $2)" >&2; exit 1; fi
    if [ "$state" != absent ] && [ "$state" != unreachable ]; then echo "probe check gave no answer for $1" >&2; exit 1; fi
    checked "$1: no test-probes ($2 edges, $state)"
}
RULE_MODE=
if [ $# -gt 0 ]; then
    RULE_MODE=1 RULE= C= T=$HOST M= N= E=normal,build
    while [ $# -gt 0 ]; do case $1 in --rule) RULE=$2;; --crate) C=$2;; --target) T=$2;; --manifest) M=$2;; --names) N=$2;; --edges) E=$2;; esac; shift 2; done
    IFS=, read -ra NA <<<"$N"
    case $RULE in forbid) forbid "$C" "$T" "$M" "${NA[@]}";; no-bevy-prefix) nobevy "$C" "$T" "$M";; workspace) workspace "${NA[@]}";; no-probes) noprobes "$C" "$E";; esac
    echo "rule ok"; exit 0
fi
forbid turts_sim $HOST "" bevy_render bevy_winit bevy_pbr turts_mod_runtime
forbid turts_server $HOST "" bevy_render bevy_winit bevy_pbr
nobevy turts_mod_api $HOST ""
nobevy turts_mod_runtime $HOST ""
nobevy turts_net_protocol $HOST ""
nobevy turts_interaction $HOST ""
nobevy turts_interaction wasm32-unknown-unknown ""
forbid turts_interaction $HOST "" turts_sim wgpu winit
forbid turts_interaction wasm32-unknown-unknown "" turts_sim wgpu winit
forbid turts_client $HOST "" turts_sim
forbid turts_client wasm32-unknown-unknown "" turts_sim
# The browser entry must not reach the server simulation either (M1 verify).
forbid turts_client_web $HOST "" turts_sim
forbid turts_client_web wasm32-unknown-unknown "" turts_sim
forbid turts_net_lightyear $HOST "" turts_sim turts_mod_runtime
# The audit crate reads the economy log. It must not reach a renderer, a client, the server,
# the packer, the mod runtime, or the transport (pre-MP1 slice design section 3).
AUDIT_FORBIDDEN=(bevy_render bevy_winit bevy_pbr turts_client turts_client_web turts_interaction turts_server turts_pack turts_mod_runtime turts_net_lightyear)
forbid turts_audit $HOST "" "${AUDIT_FORBIDDEN[@]}"
forbid turts_audit wasm32-unknown-unknown "" "${AUDIT_FORBIDDEN[@]}"
# The simulation and the client crates must not read the economy log.
forbid turts_sim $HOST "" turts_audit
forbid turts_client $HOST "" turts_audit
forbid turts_client wasm32-unknown-unknown "" turts_audit
forbid turts_client_web $HOST "" turts_audit
forbid turts_client_web wasm32-unknown-unknown "" turts_audit
forbid turts_interaction $HOST "" turts_audit
forbid turts_interaction wasm32-unknown-unknown "" turts_audit
# The pure rules crate (MP1 design section 9). It takes plain values: no `bevy*` crate, no
# workspace crate, no `wgpu`, no `winit`, on host and on wasm32. The list names every other
# workspace crate; the T1 test `t1x_rules_forbidden_list_is_every_workspace_crate` fails when
# a crate is added and this list is not.
RULES_FORBIDDEN=(turts_audit turts_checks turts_client turts_client_web turts_interaction turts_mod_api turts_mod_runtime turts_net_lightyear turts_net_protocol turts_pack turts_server turts_sim wgpu winit)
nobevy turts_rules $HOST ""
nobevy turts_rules wasm32-unknown-unknown ""
forbid turts_rules $HOST "" "${RULES_FORBIDDEN[@]}"
forbid turts_rules wasm32-unknown-unknown "" "${RULES_FORBIDDEN[@]}"
workspace lightyear_prediction bevy_replicon
# The probe feature must not reach a shipped build (normal and build edges; the dev edge of
# turts_pack and the test builds turn it on on purpose).
noprobes turts_server normal,build
noprobes turts_client normal,build
noprobes turts_client_web normal,build
# Self-check: the same grep must find the feature where it is on. turts_pack turns it on in its
# dev-dependencies (the trace emitter needs it). When this line fails, the grep is blind.
[ "$(probes_in turts_pack normal,build,dev)" = present ] || { echo "probe check not red: test-probes not found on the dev edge of turts_pack" >&2; exit 1; }
checked "self-check: test-probes found on the dev edge of turts_pack"
FIX=${BOUNDARY_FIXTURE_MANIFEST:-tests/boundary-fixture/Cargo.toml}
PKG=${BOUNDARY_FIXTURE_PACKAGE:-turts_boundary_fixture}
l=$(tree "$PKG" $HOST "$FIX")
names_has "$l" bevy_render || { echo "fixture not red" >&2; exit 1; }
echo "boundary ok"
