#!/usr/bin/env bash
# The reference build of the cold-play tool (check C5): MC commit 9b3a7bd, package forge, the build that a person could
# not play. docs/qa/cold-play.md has the whole procedure.
#
#   scripts/cold-reference.sh rebuild OUT            run in a detached worktree of 9b3a7bd, with CARGO_TARGET_DIR set (below):
#                                                     builds the server, the packer and the wasm, packs mods/forge, and copies
#                                                     turts_server, forge.tar and dist/ to OUT
#   scripts/cold-reference.sh serve REF STATE        start a NEW game from REF (a folder with turts_server, forge.tar, dist/):
#                                                     two loopback servers on free ports; prints `URL: <address>` and
#                                                     `PIDS: <file>`; stops on TERM or INT. A slot is permanent: one game for
#                                                     each probe and each agent session.
#   scripts/cold-reference.sh verify REF             sha256sum -c of REF/SHA256SUMS
set -euo pipefail
usage() { echo "usage: scripts/cold-reference.sh rebuild OUT | serve REF STATE | verify REF" >&2; exit 2; }
(($# >= 2)) || usage
cmd="$1"
case "$cmd" in
    rebuild)
        OUT="$2"
        [[ "$(git rev-parse HEAD 2>/dev/null)" == 9b3a7bdd* ]] || { echo "cold-reference: run this in a worktree at commit 9b3a7bd" >&2; exit 2; }
        [[ -n "${CARGO_TARGET_DIR:-}" ]] || { echo "cold-reference: set CARGO_TARGET_DIR to a cache folder" >&2; exit 2; }
        mkdir -p "$OUT"
        cargo build --release -p turts_server
        cargo build --release -p turts_pack --bin turts-pack
        "$CARGO_TARGET_DIR/release/turts-pack" mods/forge | tee "$OUT/pack.out"
        (cd crates/turts_client_web && trunk build --release)
        cp "$CARGO_TARGET_DIR/release/turts_server" "$OUT/turts_server"
        cp mods/forge.tar "$OUT/forge.tar"
        rm -rf "$OUT/dist"
        cp -r crates/turts_client_web/dist "$OUT/dist"
        (cd "$OUT" && sha256sum turts_server forge.tar dist/*.wasm dist/*.js dist/index.html | tee SHA256SUMS)
        ;;
    verify)
        (cd "$2" && sha256sum -c SHA256SUMS)
        ;;
    serve)
        (($# == 3)) || usage
        REF="$(cd "$2" && pwd)"
        STATE="$3"
        [[ -x "$REF/turts_server" && -f "$REF/forge.tar" && -f "$REF/dist/index.html" ]] || { echo "cold-reference: $REF has no turts_server, forge.tar and dist/" >&2; exit 2; }
        mkdir -p "$STATE"
        read -r STATIC_PORT HTTP_PORT WS_PORT < <(python3 -I -B -c '
import socket
s = [socket.socket() for _ in range(3)]
[x.bind(("127.0.0.1", 0)) for x in s]
print(*(x.getsockname()[1] for x in s))
[x.close() for x in s]')
        cat >"$STATE/server.ron" <<RON
(
    game_package: "$REF/forge.tar",
    http_listen: "127.0.0.1:$HTTP_PORT",
    ws_listen: "127.0.0.1:$WS_PORT",
    public_url: "http://127.0.0.1:$HTTP_PORT",
    tick_hz: 20,
    static_dir: "$REF/dist",
)
RON
        "$REF/turts_server" "$STATE/server.ron" >"$STATE/server.log" 2>&1 &
        SERVER=$!
        python3 -I -B -m http.server "$STATIC_PORT" --bind 127.0.0.1 --directory "$REF/dist" >"$STATE/static.log" 2>&1 &
        STATIC=$!
        trap 'kill "$SERVER" "$STATIC" 2>/dev/null || true; wait "$SERVER" "$STATIC" 2>/dev/null || true' EXIT
        trap 'exit 143' TERM
        trap 'exit 130' INT
        for _ in $(seq 1 100); do
            curl -fsS "http://127.0.0.1:$STATIC_PORT/" >/dev/null 2>&1 && curl -fsS "http://127.0.0.1:$HTTP_PORT/advertisement.json" >/dev/null 2>&1 && break
            sleep 0.1
        done
        echo "$SERVER $STATIC" >"$STATE/pids.txt"
        echo "URL: http://127.0.0.1:$STATIC_PORT/?server=http%3A%2F%2F127.0.0.1%3A$HTTP_PORT&ws=ws%3A%2F%2F127.0.0.1%3A$WS_PORT"
        echo "PIDS: $STATE/pids.txt"
        wait "$SERVER"
        ;;
    *) usage ;;
esac
