#!/usr/bin/env bash
# Sourced by the --prepare branch of mc-browser.sh, t7.sh, m2-browser.sh and m3-browser.sh (Q-PREBUILD).
#
# mc_prepare_begin DIR ROOT PACKAGE [ROW_FILE]
#   - takes an exclusive lock on DIR.lock for the rest of the shell: a second prepare into the same folder
#     is refused (exit 2) and cannot empty the folder under the first one;
#   - refuses (exit 2) a folder that is not empty and is not a prepared folder;
#   - empties DIR, and writes DIR/.begin.json (the state of the source before the build).
mc_prepare_begin() {
    local dir="$1" root="$2" package="$3" row="${4:-}"
    mkdir -p "$(dirname "$dir")"
    exec {MC_PREPARE_LOCK_FD}>"$dir.lock"
    flock -n "$MC_PREPARE_LOCK_FD" || {
        echo "--prepare: another prepare step holds $dir.lock; two prepare steps into one folder are refused" >&2
        exit 2
    }
    if [[ -n "$(ls -A "$dir" 2>/dev/null)" && ! -f "$dir/manifest.json" && ! -f "$dir/.begin.json" ]]; then
        echo "--prepare: $dir is not empty and is not a prepared folder" >&2
        exit 2
    fi
    rm -rf "$dir"
    mkdir -p "$dir"
    python3 -B "$root/scripts/mc_prepare.py" begin --root "$root" --out "$(cd "$dir" && pwd)" --package "$package" ${row:+--row "$row"}
}
