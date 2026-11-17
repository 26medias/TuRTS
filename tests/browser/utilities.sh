#!/usr/bin/env bash
# turts-test-role: instrument
# Run the M6 UT8 browser case from an already built, source-matching prepared bundle.
# This script builds nothing and owns only its case server and browser processes.
# Usage: tests/browser/utilities.sh PREPARED OUTPUT
set -euo pipefail
if (($# != 2)); then
    echo 'Usage: tests/browser/utilities.sh PREPARED OUTPUT' >&2
    exit 2
fi
UT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
UT_PREPARED="$(realpath "$1")"
UT_OUT="$(realpath -m "$2")"
if [[ -e "$UT_OUT" ]]; then
    echo 'Use a fresh output directory; existing evidence is preserved.' >&2
    exit 2
fi
mkdir -p "$UT_OUT"
cp -a "$UT_ROOT/mods/utility_yard" "$UT_OUT/fixture"
"$UT_PREPARED/bin/turts-pack" "$UT_OUT/fixture" > "$UT_OUT/pack.log" 2>&1
UT_URL="$(python3 -I -B - "$UT_PREPARED" "$UT_OUT" <<'PY'
import json, os, pathlib, socket, sys
prepared = pathlib.Path(sys.argv[1])
out = pathlib.Path(sys.argv[2])
def port():
    with socket.socket() as listener:
        listener.bind(('127.0.0.1', 0))
        return listener.getsockname()[1]
http, ws = port(), port()
while http == ws:
    ws = port()
private = out / 'private'
private.mkdir(mode=0o700)
seed = private / 'seed.txt'
seed.write_text('0\n')
os.chmod(seed, 0o600)
url = f'http://127.0.0.1:{http}'
q = json.dumps
config = (
    f'(game_package:{q(str(out / "fixture.tar"))},http_listen:{q(f"127.0.0.1:{http}")},'
    f'ws_listen:{q(f"127.0.0.1:{ws}")},public_url:{q(url)},tick_hz:20,'
    f'static_dir:Some({q(str(prepared / "dist"))}),start_policy:Some(Immediate),'
    f'terrain_seed_file:Some({q(str(seed))}),'
    f'terrain_provenance:Some({q(str(private / "terrain.json"))}))\n'
)
(out / 'server.ron').write_text(config)
print(url + '/?server=' + url + '&ws=' + f'ws://127.0.0.1:{ws}')
PY
)"
TURTS_COMPUTER_SLOT='' "$UT_PREPARED/bin/turts_server" "$UT_OUT/server.ron" > "$UT_OUT/server.log" 2>&1 &
UT_SERVER_PID=$!
cleanup_utility_case() {
    if kill -0 "$UT_SERVER_PID" 2>/dev/null; then
        kill "$UT_SERVER_PID" 2>/dev/null || true
    fi
    wait "$UT_SERVER_PID" 2>/dev/null || true
}
trap cleanup_utility_case EXIT
python3 -I -B - "$UT_URL" <<'PY'
import sys, time, urllib.request
end = time.monotonic() + 30
while time.monotonic() < end:
    try:
        with urllib.request.urlopen(sys.argv[1].split('/?')[0] + '/advertisement.json', timeout=1) as response:
            if response.status == 200:
                break
    except OSError:
        pass
    time.sleep(.1)
else:
    raise SystemExit('Utility case server did not become ready; see server.log')
PY
node "$UT_ROOT/tests/browser/utilities.js" "$UT_URL" "$UT_OUT/browser"
