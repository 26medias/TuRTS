#!/usr/bin/env bash
# Run one real terrain input case from an already built, source-matching bundle.
# This script builds nothing and owns only its case server and browser processes.
set -euo pipefail
if (($# != 3)); then
    echo 'Usage: tests/browser/terrain-work.sh PREPARED PACKAGE OUTPUT' >&2
    exit 2
fi
TERRAIN_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TERRAIN_PREPARED="$(realpath "$1")"
TERRAIN_PACKAGE="$2"
TERRAIN_OUT="$(realpath -m "$3")"
case "$TERRAIN_PACKAGE" in quarry) TERRAIN_SCENARIO=terrain-quarry;; terrain_stockyard) TERRAIN_SCENARIO=terrain-stockyard;; *) echo 'Unknown terrain fixture' >&2; exit 2;; esac
python3 -B "$TERRAIN_ROOT/scripts/mc_prepare.py" check "$TERRAIN_PREPARED" --root "$TERRAIN_ROOT" --package mods \
    --need bin/turts_server --need bin/turts-pack --need dist/index.html
mkdir -p "$TERRAIN_OUT"
if [[ -e "$TERRAIN_OUT/fixture" ]]; then
    echo 'Use a fresh output directory; existing fixture is preserved.' >&2
    exit 2
fi
cp -a "$TERRAIN_ROOT/mods/$TERRAIN_PACKAGE" "$TERRAIN_OUT/fixture"
"$TERRAIN_PREPARED/bin/turts-pack" "$TERRAIN_OUT/fixture" > "$TERRAIN_OUT/pack.log" 2>&1
TERRAIN_URL="$(python3 - "$TERRAIN_PREPARED" "$TERRAIN_OUT" <<'PY'
import json,os,pathlib,socket,sys
prepared=pathlib.Path(sys.argv[1]);out=pathlib.Path(sys.argv[2])
def port():
    with socket.socket() as listener:
        listener.bind(('127.0.0.1',0))
        return listener.getsockname()[1]
http,ws=port(),port()
while http==ws:ws=port()
private=out/'private';private.mkdir(mode=0o700)
seed=private/'seed.txt';seed.write_text('0\n');os.chmod(seed,0o600)
url=f'http://127.0.0.1:{http}'
q=json.dumps
config=f'(game_package:{q(str(out/"fixture.tar"))},http_listen:{q(f"127.0.0.1:{http}")},ws_listen:{q(f"127.0.0.1:{ws}")},public_url:{q(url)},tick_hz:20,static_dir:Some({q(str(prepared/"dist"))}),start_policy:Some(Immediate),terrain_seed_file:Some({q(str(seed))}),terrain_provenance:Some({q(str(private/"terrain.json"))}))\n'
(out/'server.ron').write_text(config)
print(url)
PY
)"
TURTS_COMPUTER_SLOT='' "$TERRAIN_PREPARED/bin/turts_server" "$TERRAIN_OUT/server.ron" > "$TERRAIN_OUT/server.log" 2>&1 &
TERRAIN_SERVER_PID=$!
cleanup_terrain_case() {
    if kill -0 "$TERRAIN_SERVER_PID" 2>/dev/null; then
        kill "$TERRAIN_SERVER_PID" 2>/dev/null || true
    fi
    wait "$TERRAIN_SERVER_PID" 2>/dev/null || true
}
trap cleanup_terrain_case EXIT
python3 - "$TERRAIN_URL" <<'PY'
import sys,time,urllib.request
end=time.monotonic()+30
while time.monotonic()<end:
    try:
        with urllib.request.urlopen(sys.argv[1]+'/advertisement.json',timeout=1) as response:
            if response.status==200:break
    except OSError:pass
    time.sleep(.1)
else:raise SystemExit('Terrain case server did not become ready; see server.log')
PY
node "$TERRAIN_ROOT/tests/browser/terrain-work.js" "$TERRAIN_URL" "$TERRAIN_ROOT/tests/browser/fixtures/$TERRAIN_SCENARIO.json" "$TERRAIN_OUT/browser"
