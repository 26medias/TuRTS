#!/usr/bin/env bash
# Run the native rendered-UI proof in a private Xvfb display and persist all evidence.
set -Eeuo pipefail

repo=$(cd "$(dirname "$0")/.." && pwd)
target=${CARGO_TARGET_DIR:-"$repo/target"}
evidence_root=${TASK16_EVIDENCE_DIR:-"$target/task16-native-evidence"}

if [[ ${TASK16_XVFB_INNER:-0} != 1 ]]; then
    export TASK16_XVFB_INNER=1
    exec /usr/bin/xvfb-run --auto-servernum --server-args='-screen 0 1280x720x24' bash "$0" "$@"
fi

if [[ ${DISPLAY:-} == :1 ]]; then
    echo "refusing the user's DISPLAY=:1" >&2
    exit 2
fi

run_id=$(date -u +%Y%m%dT%H%M%SZ)-$$
run_dir="$evidence_root/runs/$run_id"
mkdir -p "$run_dir/logs" "$run_dir/packages"
export CARGO_TARGET_DIR="$target"
export CARGO_BUILD_JOBS=2

server_pid=
client_pid=
cleanup() {
    local status=$?
    if [[ -n "$client_pid" ]] && kill -0 "$client_pid" 2>/dev/null; then
        kill "$client_pid" 2>/dev/null || true
        wait "$client_pid" 2>/dev/null || true
    fi
    if [[ -n "$server_pid" ]] && kill -0 "$server_pid" 2>/dev/null; then
        kill "$server_pid" 2>/dev/null || true
        wait "$server_pid" 2>/dev/null || true
    fi
    {
        printf 'script_exit=%s\n' "$status"
        printf 'client_pid=%s\n' "${client_pid:-none}"
        printf 'server_pid=%s\n' "${server_pid:-none}"
        printf 'private_display=%s\n' "${DISPLAY:-unset}"
        printf 'cleaned_at_utc=%s\n' "$(date -u +%FT%TZ)"
    } > "$run_dir/process-cleanup.txt"
    if [[ -n "$run_dir" && -d "$run_dir" ]]; then
        sha256sum "$repo/crates/turts_client/src/lib.rs" "$repo/crates/turts_client/src/live_view.rs" "$repo/scripts/native-view-proof.sh" > "$run_dir/source-files.sha256" 2>/dev/null || true
        (cd "$run_dir" && find . -type f ! -name sha256sums.txt -print0 | sort -z | xargs -0 -r sha256sum > sha256sums.txt) || true
        for binary in "$target/release/turts_server" "$target/release/turts_client" "$target/release/turts-pack"; do
            if [[ -f "$binary" ]]; then sha256sum "$binary" >> "$run_dir/sha256sums.txt"; fi
        done
        if [[ "$status" -ne 0 ]]; then
            printf 'failed_exit=%s\nfailed_at_utc=%s\n' "$status" "$(date -u +%FT%TZ)" > "$run_dir/failure.txt"
        fi
    fi
    return "$status"
}
trap cleanup EXIT INT TERM

if [[ ${TASK16_SKIP_BUILD:-0} != 1 ]]; then
    cargo build --release -p turts_server
    cargo build --release -p turts_client
    cargo build --release -p turts_pack
fi
server_bin="$target/release/turts_server"
client_bin="$target/release/turts_client"
packer_bin="$target/release/turts-pack"
for bin in "$server_bin" "$client_bin" "$packer_bin"; do
    [[ -x "$bin" ]] || { echo "missing fresh release binary: $bin" >&2; exit 2; }
done

cp -a "$repo/mods/base" "$run_dir/packages/base"
"$packer_bin" "$run_dir/packages/base" | tee "$run_dir/package-pack.log"
package="$run_dir/packages/base.tar"
sha256sum "$package" > "$run_dir/package.sha256"

readarray -t ports < <(python3 - <<'PY'
import socket
sockets=[]
for _ in range(2):
    sock=socket.socket()
    sock.bind(("127.0.0.1", 0))
    sockets.append(sock)
for sock in sockets:
    print(sock.getsockname()[1])
for sock in sockets:
    sock.close()
PY
)
http_port=${ports[0]}
ws_port=${ports[1]}
http_url="http://127.0.0.1:$http_port"
ws_url="ws://127.0.0.1:$ws_port"
cat > "$run_dir/server.ron" <<RON
(
    game_package: "$package",
    http_listen: "127.0.0.1:$http_port",
    ws_listen: "127.0.0.1:$ws_port",
    public_url: "$http_url",
    tick_hz: 20,
)
RON

"$server_bin" "$run_dir/server.ron" --stats "$run_dir/server-stats.jsonl" \
    > "$run_dir/logs/server.log" 2>&1 &
server_pid=$!
printf 'server_pid=%s\n' "$server_pid" > "$run_dir/process-pids.txt"
python3 - "$http_url/advertisement.json" <<'PY'
import sys, time, urllib.request
url=sys.argv[1]
deadline=time.monotonic()+45
last=None
while time.monotonic()<deadline:
    try:
        with urllib.request.urlopen(url, timeout=1) as response:
            if response.status == 200:
                print(response.read().decode())
                raise SystemExit(0)
    except Exception as exc:
        last=exc
    time.sleep(.1)
raise SystemExit(f"server HTTP did not become ready: {last}")
PY

client_out="$run_dir/client-out.json"
view="$client_out.view.json"
"$client_bin" --server "$http_url" --ws "$ws_url" --player-id 161627 \
    --out "$client_out" --run-secs 45 > "$run_dir/logs/client.log" 2>&1 &
client_pid=$!
printf 'client_pid=%s\n' "$client_pid" >> "$run_dir/process-pids.txt"

python3 - "$view" <<'PY'
import json, sys, time
path=sys.argv[1]
deadline=time.monotonic()+35
stable=0
last_frame=None
last_shape=None
while time.monotonic()<deadline:
    try:
        with open(path, encoding="utf-8") as stream:
            data=json.load(stream)
        world=data.get("world") or {}
        active=[camera for camera in data.get("cameras", []) if camera.get("active")]
        ui=[row for row in data.get("ui", []) if row.get("visible") and row.get("rect", [0,0,0,0])[2] > 0 and row.get("rect", [0,0,0,0])[3] > 0]
        units=[mesh for mesh in data.get("meshes", []) if mesh.get("kind")=="unit" and mesh.get("projected_ground_center")]
        nodes=[mesh for mesh in data.get("meshes", []) if mesh.get("kind")=="node" and mesh.get("projected_ground_center")]
        frame=(data.get("presentation") or {}).get("frame_ns")
        shape=(len(active), len(ui), len(units), len(nodes), data.get("render_adapter"))
        ready=(str(data.get("phase", "")).startswith("Joined") and world.get("coherent") is True and not world.get("resync_active") and bool(world.get("applied_tick")) and len(active)==1 and bool(ui) and bool(units) and bool(nodes) and data.get("render_adapter"))
        if ready and frame != last_frame and shape == last_shape:
            stable += 1
        elif ready:
            stable = 1
        else:
            stable = 0
        if stable >= 4:
            print(json.dumps({"phase":data["phase"],"applied_tick":world["applied_tick"],"active_cameras":len(active),"visible_text_rows":len(ui),"unit_meshes":len(units),"node_meshes":len(nodes),"render_adapter":data["render_adapter"]},sort_keys=True))
            raise SystemExit(0)
        last_frame, last_shape=frame, shape
    except (FileNotFoundError, json.JSONDecodeError, KeyError, TypeError):
        pass
    time.sleep(.15)
raise SystemExit("did not reach four stable joined frames with coherent world, actual UI, projected meshes, and adapter info")
PY

window_id=$(xdotool search --onlyvisible --name 'TuRTS' | tail -n 1)
[[ -n "$window_id" ]] || { echo "could not find the TuRTS window on private DISPLAY=$DISPLAY" >&2; exit 1; }
printf 'window_id=%s\n' "$window_id" >> "$run_dir/process-pids.txt"
xdotool windowfocus --sync "$window_id"
import -window "$window_id" "$run_dir/before.png"

read -r harvester_id harvester_x harvester_y < <(python3 - "$view" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as stream: data=json.load(stream)
slot=(data.get("welcome") or {}).get("slot")
unit=next(unit for unit in data["world"]["units"] if unit["owner_slot"]==slot and "harvester" in unit["content_id"])
mesh=next(mesh for mesh in data["meshes"] if mesh["kind"]=="unit" and mesh["id"]==unit["id"])
center=mesh["projected_ground_center"]
scale=data["windows"][0]["scale_factor"]
print(unit["id"], round(center[0]*scale), round(center[1]*scale))
PY
)

wait_for_selection() {
    local id=$1
    local label=$2
    python3 - "$view" "$id" "$label" <<'PY'
import json, sys, time
path, unit_id, label=sys.argv[1], int(sys.argv[2]), sys.argv[3]
deadline=time.monotonic()+8
while time.monotonic()<deadline:
    try:
        with open(path, encoding="utf-8") as stream: data=json.load(stream)
        if data.get("selection")==unit_id and any(row.get("visible") and label in row.get("text", "") for row in data.get("ui", [])):
            print(json.dumps({"selection":unit_id,"label":label}))
            raise SystemExit(0)
    except (FileNotFoundError, json.JSONDecodeError): pass
    time.sleep(.1)
raise SystemExit(f"selection/UI label did not appear: {unit_id} {label}")
PY
}

wait_for_button_pressed() {
    local action_type=$1
    local producer=$2
    local recipe=${3:-}
    python3 - "$view" "$action_type" "$producer" "$recipe" <<'PY'
import json, sys, time
path, kind, producer, recipe=sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
deadline=time.monotonic()+8
while time.monotonic()<deadline:
    try:
        with open(path, encoding="utf-8") as stream: data=json.load(stream)
        for row in data.get("ui", []):
            action=row.get("action") or {}
            if (row.get("visible") and row.get("enabled") and action.get("type")==kind
                    and action.get("producer")==producer
                    and (not recipe or action.get("recipe")==recipe)
                    and row.get("button_pressed") is True and row.get("button_rect")):
                print(json.dumps(row, sort_keys=True))
                raise SystemExit(0)
    except (FileNotFoundError, json.JSONDecodeError): pass
    time.sleep(.05)
raise SystemExit(f"actual enabled {kind} button did not expose a Pressed frame")
PY
}

before_selection_applied=$(python3 - "$view" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as stream: data=json.load(stream)
print(sum(1 for result in data.get("command_results",[]) if result.get("kind")=="applied"))
PY
)
xdotool mousemove --window "$window_id" "$harvester_x" "$harvester_y" click 1
wait_for_selection "$harvester_id" 'Selected: Harvester' | tee "$run_dir/select-harvester.json"
python3 - "$view" "$before_selection_applied" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as stream: data=json.load(stream)
count=sum(1 for result in data.get("command_results",[]) if result.get("kind")=="applied")
if count != int(sys.argv[2]): raise SystemExit("unit selection also issued a world command")
PY

read -r node_id node_x node_y < <(python3 - "$view" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as stream: data=json.load(stream)
node=data["world"]["nodes"][0]
mesh=next(mesh for mesh in data["meshes"] if mesh["kind"]=="node" and mesh["id"]==node["id"])
center=mesh["projected_ground_center"]
scale=data["windows"][0]["scale_factor"]
print(node["id"], round(center[0]*scale), round(center[1]*scale))
PY
)

gather_before=$(python3 - "$view" "$harvester_id" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as stream: data=json.load(stream)
world=data["world"]
orders=world.get("economy",{}).get("orders",[])
matching=[order for order in orders if order.get("unit")==int(sys.argv[2])]
if len(matching)!=1 or matching[0].get("stage")!="Idle" or matching[0].get("node") is not None:
    raise SystemExit(f"unexpected initial Gather state: {matching}")
print(sum(1 for result in data.get("command_results",[]) if result.get("kind")=="applied"))
PY
)
python3 - "$view" "$harvester_id" <<'PY' > "$run_dir/gather-idle-before.json"
import json, sys
with open(sys.argv[1], encoding="utf-8") as stream: data=json.load(stream)
order=next(order for order in data["world"]["economy"]["orders"] if order.get("unit")==int(sys.argv[2]))
print(json.dumps(order, sort_keys=True))
PY
xdotool mousemove --window "$window_id" "$node_x" "$node_y" click 3
python3 - "$view" "$harvester_id" "$node_id" "$gather_before" <<'PY'
import json, sys, time
path, unit_id, node_id, before=sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4])
deadline=time.monotonic()+12
while time.monotonic()<deadline:
    try:
        with open(path, encoding="utf-8") as stream: data=json.load(stream)
        economy=(data.get("world") or {}).get("economy") or {}
        orders=economy.get("orders",[])
        matching=[order for order in orders if order.get("unit")==unit_id]
        applied=[result for result in data.get("command_results",[]) if result.get("kind")=="applied"]
        text=[row.get("text","") for row in data.get("ui",[]) if row.get("visible")]
        transitioned=(len(matching)==1 and matching[0].get("node")==node_id and matching[0].get("stage")!="Idle")
        if transitioned and len(applied)>=before+1 and any(line.startswith("Gather:") and "idle" not in line for line in text):
            if len(applied)!=before+1: raise SystemExit(f"real node click issued {len(applied)-before} applied commands; expected one Gather")
            print(json.dumps({"unit":unit_id,"orders":matching,"applied_results":len(applied),"gather_text":[line for line in text if line.startswith("Gather:")]}))
            raise SystemExit(0)
    except (FileNotFoundError, json.JSONDecodeError): pass
    time.sleep(.1)
raise SystemExit("real node click did not change the Idle replica order to the clicked node with an applied result and UI state")
PY

echo "$node_id" > "$run_dir/gathered-node-id.txt"
read -r producer_id producer_x producer_y < <(python3 - "$view" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as stream: data=json.load(stream)
slot=(data.get("welcome") or {}).get("slot")
unit=next(unit for unit in data["world"]["units"] if unit["owner_slot"]==slot and "command_center" in unit["content_id"])
mesh=next(mesh for mesh in data["meshes"] if mesh["kind"]=="unit" and mesh["id"]==unit["id"])
center=mesh["projected_ground_center"]
scale=data["windows"][0]["scale_factor"]
print(unit["id"], round(center[0]*scale), round(center[1]*scale))
PY
)
xdotool mousemove --window "$window_id" "$producer_x" "$producer_y" click 1
wait_for_selection "$producer_id" 'Selected: Command Center' | tee "$run_dir/select-producer.json"

read -r produce_x produce_y < <(python3 - "$view" "$producer_id" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as stream: data=json.load(stream)
producer=int(sys.argv[2])
row=next(row for row in data["ui"] if row.get("visible") and row.get("enabled") and row.get("action") and row["action"].get("type")=="produce" and row["action"].get("producer")==producer and row["action"].get("recipe")=="base:train_rifleman")
rect=row["button_rect"]
if not rect: raise SystemExit("actual Produce button has no computed layout rectangle")
scale=data["windows"][0]["scale_factor"]
print(round((rect[0]+rect[2]/2)*scale), round((rect[1]+rect[3]/2)*scale))
PY
)

python3 - "$view" <<'PY' > "$run_dir/panel-produce-before.json"
import json, sys
with open(sys.argv[1], encoding="utf-8") as stream: data=json.load(stream)
print(sum(1 for result in data.get("command_results",[]) if result.get("kind")=="applied"))
PY
before_applied=$(cat "$run_dir/panel-produce-before.json")
xdotool mousemove --window "$window_id" "$produce_x" "$produce_y"
xdotool mousedown 1
wait_for_button_pressed produce "$producer_id" base:train_rifleman | tee "$run_dir/produce-pressed.json"
xdotool mouseup 1
python3 - "$view" "$producer_id" "$before_applied" <<'PY'
import json, sys, time
path, producer, before=sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
deadline=time.monotonic()+10
while time.monotonic()<deadline:
    try:
        with open(path, encoding="utf-8") as stream: data=json.load(stream)
        queues=(data.get("world") or {}).get("economy",{}).get("queues",[])
        queue=next((queue for queue in queues if queue.get("producer")==producer and queue.get("jobs")),None)
        applied=[result for result in data.get("command_results",[]) if result.get("kind")=="applied"]
        if queue and len(applied)>=before+1:
            time.sleep(.6)
            with open(path, encoding="utf-8") as stream: data=json.load(stream)
            applied=[result for result in data.get("command_results",[]) if result.get("kind")=="applied"]
            if len(applied)!=before+1:
                raise SystemExit(f"panel Produce click generated {len(applied)-before} applied commands; expected exactly one")
            print(json.dumps({"queue":queue,"applied_results":len(applied),"panel_click_extra_world_commands":0}))
            raise SystemExit(0)
    except (FileNotFoundError, json.JSONDecodeError): pass
    time.sleep(.1)
raise SystemExit("actual visible Produce click did not apply one command and create the queue record")
PY

read -r cancel_x cancel_y < <(python3 - "$view" "$producer_id" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as stream: data=json.load(stream)
producer=int(sys.argv[2])
row=next(row for row in data["ui"] if row.get("visible") and row.get("enabled") and row.get("action") and row["action"].get("type")=="cancel" and row["action"].get("producer")==producer)
rect=row["button_rect"]
if not rect: raise SystemExit("actual Cancel button has no computed layout rectangle")
scale=data["windows"][0]["scale_factor"]
print(round((rect[0]+rect[2]/2)*scale), round((rect[1]+rect[3]/2)*scale))
PY
)
python3 - "$view" <<'PY' > "$run_dir/panel-cancel-before.json"
import json, sys
with open(sys.argv[1], encoding="utf-8") as stream: data=json.load(stream)
print(sum(1 for result in data.get("command_results",[]) if result.get("kind")=="applied"))
PY
before_applied=$(cat "$run_dir/panel-cancel-before.json")
xdotool mousemove --window "$window_id" "$cancel_x" "$cancel_y"
xdotool mousedown 1
wait_for_button_pressed cancel "$producer_id" | tee "$run_dir/cancel-pressed.json"
xdotool mouseup 1
python3 - "$view" "$producer_id" "$before_applied" <<'PY'
import json, sys, time
path, producer, before=sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
deadline=time.monotonic()+10
while time.monotonic()<deadline:
    try:
        with open(path, encoding="utf-8") as stream: data=json.load(stream)
        queues=(data.get("world") or {}).get("economy",{}).get("queues",[])
        pending=any(queue.get("producer")==producer and queue.get("jobs") for queue in queues)
        applied=[result for result in data.get("command_results",[]) if result.get("kind")=="applied"]
        if not pending and len(applied)>=before+1:
            time.sleep(.6)
            with open(path, encoding="utf-8") as stream: data=json.load(stream)
            applied=[result for result in data.get("command_results",[]) if result.get("kind")=="applied"]
            if len(applied)!=before+1:
                raise SystemExit(f"panel Cancel click generated {len(applied)-before} applied commands; expected exactly one")
            print(json.dumps({"producer":producer,"queue_empty":True,"applied_results":len(applied),"panel_click_extra_world_commands":0}))
            raise SystemExit(0)
    except (FileNotFoundError, json.JSONDecodeError): pass
    time.sleep(.1)
raise SystemExit("actual visible Cancel click did not apply one command and clear the queue")
PY

import -window "$window_id" "$run_dir/after.png"
python3 - "$run_dir/before.png" "$run_dir/after.png" <<'PY'
import subprocess, sys
for path in sys.argv[1:]:
    value=subprocess.check_output(["convert", path, "-colorspace", "Gray", "-format", "%[fx:mean]", "info:"]).decode().strip()
    mean=float(value)
    print(f"{path}: grayscale_mean={mean}")
    if mean <= 0.01:
        raise SystemExit(f"screenshot is effectively black: {path}")
PY

# Let --run-secs write its final --out report before cleaning up either process.
wait "$client_pid"
client_status=$?
client_pid=
[[ "$client_status" -eq 0 ]] || { echo "native client exit status $client_status" >&2; exit "$client_status"; }
[[ -s "$client_out" ]] || { echo "native final --out report was not written" >&2; exit 1; }

cat > "$run_dir/provenance.txt" <<EOF
source_revision=$(git -C "$repo" rev-parse HEAD)
source_worktree=$(git -C "$repo" rev-parse --show-toplevel)
source_patch_sha256=$(git -C "$repo" diff --binary HEAD -- crates/turts_client/src/lib.rs crates/turts_client/src/live_view.rs scripts/native-view-proof.sh | sha256sum | cut -d' ' -f1)
private_display=$DISPLAY
http_url=$http_url
ws_url=$ws_url
client_pid=${client_pid:-finished}
server_pid=$server_pid
EOF
sha256sum "$server_bin" "$client_bin" "$packer_bin" "$package" "$client_out" "$view" "$run_dir/before.png" "$run_dir/after.png" > "$run_dir/sha256sums.txt"
(nvidia-smi -q > "$run_dir/host-nvidia-smi.txt" 2>&1 || true)
(vulkaninfo --summary > "$run_dir/host-vulkaninfo-summary.txt" 2>&1 || true)
{
    rustc --version --verbose
    cargo --version
    uname -a
    xvfb-run --version 2>&1 || true
    xdotool --version 2>&1 || true
    import -version 2>&1 | head -n 2 || true
} > "$run_dir/tool-versions.txt"
adapter_mode=$(python3 - "$view" <<'PY'
import json, sys
adapter=json.load(open(sys.argv[1], encoding="utf-8"))["render_adapter"]
name=adapter["name"].lower()
kind=adapter["device_type"].lower()
if kind == "cpu" or any(tag in name for tag in ("llvmpipe", "lavapipe", "swiftshader", "software")):
    print("native-software")
elif adapter["backend"] == "Vulkan" and "rtx 3080" in name:
    print("native-vulkan-rtx3080")
elif adapter["backend"] == "Vulkan" and kind == "discretegpu":
    print("native-vulkan-hardware-other")
else:
    print(f"native-{adapter['backend'].lower()}-{kind}")
PY
)
{
    printf 'DISPLAY=%s\n' "$DISPLAY"
    printf 'proof_mode=%s\n' "$adapter_mode"
    printf 'source_revision=%s\n' "$(git -C "$repo" rev-parse HEAD)"
    printf 'adapter=%s\n' "$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1],encoding="utf-8"))["render_adapter"])' "$view")"
    printf 'client_report=%s\n' "$client_out"
    printf 'view_snapshot=%s\n' "$view"
    printf 'screenshot_before=%s\n' "$run_dir/before.png"
    printf 'screenshot_after=%s\n' "$run_dir/after.png"
} > "$run_dir/proof-summary.txt"

printf 'Task16 native proof artifacts: %s\n' "$run_dir"
