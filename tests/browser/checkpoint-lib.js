// turts-test-role: oracle
"use strict";

// Read-only helpers for the oracles of the MC ledger rewrites (t7, m3, pt1, m2). The harness
// writes one scene checkpoint for each io.screenshot() (harness/scene-checkpoints.js); a
// checkpoint holds the scene and debug bridges that the harness read after that screenshot.
const assert = require("node:assert/strict");

function saved(input, filename) {
    return input.some((entry) => entry.call === "save" && entry.args?.[0] === filename);
}

// The input-log index of the last screenshot before the save of `filename`.
function screenshotIndexForSave(input, filename) {
    const saveIndex = input.findIndex((entry) => entry.call === "save" && entry.args?.[0] === filename);
    if (saveIndex < 0) return -1;
    for (let index = saveIndex - 1; index >= 0; index -= 1) {
        if (input[index].call === "screenshot") return index;
    }
    return -1;
}

function checkpointAt(checkpoints, screenshotIndex, label) {
    const checkpoint = checkpoints.find((entry) => entry.input_call === screenshotIndex);
    assert(checkpoint?.debug && checkpoint?.scene, `${label} requires an actual scene checkpoint`);
    return { ...checkpoint, screenshotIndex };
}

function checkpointForSave(input, checkpoints, filename, label = filename) {
    const screenshotIndex = screenshotIndexForSave(input, filename);
    assert(screenshotIndex >= 0, `${label}: missing screenshot ${filename}`);
    return { ...checkpointAt(checkpoints, screenshotIndex, label), saveFile: filename };
}

// The last checkpoint before input-log index `index` in window `windowNumber`.
function checkpointBefore(checkpoints, index, windowNumber = 1) {
    return checkpoints.filter((entry) => entry.input_call < index && entry.window === windowNumber)
        .sort((left, right) => right.input_call - left.input_call)[0] ?? null;
}

function checkpointsBetween(checkpoints, from, to, windowNumber = 1) {
    return checkpoints.filter((entry) => entry.input_call > from && entry.input_call < to && entry.window === windowNumber)
        .sort((left, right) => left.input_call - right.input_call);
}

async function readCheckpoints(view) {
    const checkpoints = JSON.parse(await view.serverFile("scene-checkpoints.json"));
    assert(Array.isArray(checkpoints) && checkpoints.length > 0, "the harness wrote no scene checkpoint");
    return checkpoints;
}

// The tick log (`tick,wall_ns,unit_id,x,y`, crates/turts_server/src/ticklog.rs) for one unit.
function unitTrack(tickLog, unitId) {
    return tickLog.split(/\r?\n/).filter(Boolean).map((line) => line.split(",")).filter((fields) =>
        fields.length === 5 && Number(fields[2]) === unitId).map((fields) => ({
        tick: Number(fields[0]),
        x: Number(fields[3]),
        y: Number(fields[4]),
    })).filter((point) => Number.isInteger(point.tick) && Number.isFinite(point.x) && Number.isFinite(point.y));
}

function widget(debug, predicate) {
    return (debug?.ui_snapshot ?? []).find((row) => row && row.visible === true && predicate(row));
}

function widgets(debug, predicate) {
    return (debug?.ui_snapshot ?? []).filter((row) => row && row.visible === true && predicate(row));
}

function replicaUnit(debug, unitId) {
    return (debug?.replicated_world?.units ?? []).find((unit) => unit.unit_id === unitId) ?? null;
}

// The player label of one binding, in the form that the guide Controls list shows
// (hud2-oracle.js renders the same rows from `effective_bindings`).
function bindingLabel(binding) {
    if (binding?.Key) return [...(binding.Key.mods ?? []), keyName(binding.Key.code)].join("+");
    if (binding?.Pointer) return [...(binding.Pointer.mods ?? []), `${binding.Pointer.button} click`].join("+");
    if (binding?.Wheel) return [...(binding.Wheel.mods ?? []), `Wheel ${binding.Wheel.direction}`].join("+");
    return JSON.stringify(binding);
}

function keyName(code) {
    if (/^Key[A-Z]$/.test(code)) return code.slice(3);
    if (/^Digit\d$/.test(code)) return code.slice(5);
    return code;
}

// A driver stop record (forge-pixels.js stop, m2-driver.js stop, t7-driver.js stop) says that a
// product fault ended the drive early. Each oracle calls this after its own checks, so the
// named check of the fault keeps its text, and a short drive can never pass (RUN123 L2).
async function assertNoDriverStop(view, label) {
    const stop = view.inputLog().find((entry) => entry.call === "save" && /-driver-stop\.json$/.test(entry.args?.[0] ?? ""));
    if (!stop) return;
    let reason = "no reason recorded";
    try { reason = JSON.parse(await view.serverFile(stop.args[0])).reason ?? reason; } catch { /* the record name is enough */ }
    assert.fail(`${label}: the driver stopped early (${stop.args[0]}): ${reason}`);
}

// The server tick clock. The tick log (`tick,wall_ns,unit_id,x,y`) holds one line for each unit and
// tick, and `wall_ns` is the server time of day at the end of the tick. The index keeps the first
// line of each tick: `{ tick, wall_ms }`. A checkpoint carries `page_time_ms`, the browser clock of
// the same read of `turts_debug` (harness/scene-checkpoints.js). Both clocks are the clock of one
// host, so `serverTickAt(index, checkpoint.page_time_ms)` is the last tick that the server had
// finished when the replica reported its own tick.
function serverTickIndex(tickLog) {
    const index = [];
    let previous = null;
    let from = 0;
    while (from < tickLog.length) {
        let end = tickLog.indexOf("\n", from);
        if (end < 0) end = tickLog.length;
        const first = tickLog.indexOf(",", from);
        const second = first < 0 || first >= end ? -1 : tickLog.indexOf(",", first + 1);
        if (first >= 0 && first < end && second > first && second <= end) {
            const tick = Number(tickLog.slice(from, first));
            const wall = Number(tickLog.slice(first + 1, second));
            assert(Number.isSafeInteger(tick) && Number.isFinite(wall), `the server tick log has a bad row: ${tickLog.slice(from, end)}`);
            if (tick !== previous) { index.push({ tick, wall_ms: wall / 1e6 }); previous = tick; }
        }
        from = end + 1;
    }
    return index;
}

// The last tick of `index` that ended at or before `wallMs`, or null when none did.
function serverTickAt(index, wallMs) {
    let low = 0;
    let high = index.length - 1;
    let found = null;
    while (low <= high) {
        const middle = (low + high) >> 1;
        if (index[middle].wall_ms <= wallMs) { found = index[middle].tick; low = middle + 1; } else high = middle - 1;
    }
    return found;
}

// The replica of one window must not stand behind the server by more than `maxLag` ticks at a
// checkpoint. The oracle knows the server tick from the tick log and the browser clock, so a replica
// that stands still (RUN142: the stall after a kill) is red at once, whatever the order state shows.
function replicaLag(checkpoint, index, label) {
    const replica = checkpoint?.debug?.replicated_world?.tick;
    assert(Number.isSafeInteger(replica), `${label}: the checkpoint has no replica tick`);
    assert(Number.isFinite(checkpoint.page_time_ms), `${label}: the checkpoint has no browser clock (page_time_ms)`);
    const server = serverTickAt(index, checkpoint.page_time_ms);
    assert(Number.isSafeInteger(server), `${label}: the server tick log has no tick at or before the checkpoint`);
    return { lag: server - replica, replica, server };
}

function assertReplicaLag(checkpoint, index, maxLag, label) {
    const { lag, replica, server } = replicaLag(checkpoint, index, label);
    assert(lag <= maxLag, `${label}: the replica stands ${lag} ticks behind the server (replica tick ${replica}, server tick ${server}); the bound is ${maxLag}`);
    return lag;
}

// A replica that never lost a message asks for no resync. The stall after a kill (RUN142) is a run of
// rejected updates and resync requests: `ingest.resync_requests_sent` counts them for the whole run, so a
// capture that misses the stall window still shows it. Saved MM1 runs: 36 of 36 runs without the repair of
// commit 50eb827 show 9 to 24 requests in window 1, the 8 runs with the repair show 0. A local run has no packet
// loss, so any request is a fault of the product (or of a host stall, which assertConnectionHeld names first).
function assertNoResync(checkpoints, label) {
    for (const entry of checkpoints) {
        const ingest = entry.debug?.ingest;
        assert(ingest && Number.isSafeInteger(ingest.resync_requests_sent),
            `${label}: checkpoint ${entry.input_call} (window ${entry.window}) has no ingest.resync_requests_sent`);
        assert.equal(ingest.resync_requests_sent, 0,
            `${label}: the replica of window ${entry.window} asked the server for ${ingest.resync_requests_sent} resyncs by checkpoint ` +
            `${entry.input_call} (replica tick ${entry.debug?.replicated_world?.tick}); a local run needs none (the kill stall of RUN142)`);
    }
}

// The netcode timeout of the client (crates/turts_client/src/join.rs:345, client_timeout_secs).
const NETCODE_TIMEOUT_MS = 10000;

// The longest gap between two ticks of the server, and the tick that ends it.
function longestTickGap(index) {
    let longest = { gap_ms: 0, before_tick: null };
    for (let at = 1; at < index.length; at += 1) {
        const gap = index[at].wall_ms - index[at - 1].wall_ms;
        if (gap > longest.gap_ms) longest = { gap_ms: gap, before_tick: index[at].tick };
    }
    return longest;
}

// A lost connection is a host fault, not a fault of the row: the client reports `reconnect_required`
// after the transport ended (crates/turts_client/src/join.rs:416-424), and its replica stands still
// from then on. The text names the longest stall of the server tick log, which is the usual cause
// (a stall longer than the netcode timeout ends the connection on both sides; run
// rereview-2df2393-hold-a: stalls of 11.0 s and 17.9 s under three other heavy jobs). Rows that
// disconnect on purpose do not call this.
function assertConnectionHeld(checkpoints, index, label) {
    const lost = checkpoints.find((entry) => entry.debug?.reconnect_required === true);
    if (!lost) return;
    const { gap_ms: gap, before_tick: before } = longestTickGap(index);
    const stall = gap > NETCODE_TIMEOUT_MS
        ? `the server tick log has a stall of ${(gap / 1000).toFixed(1)} s before tick ${before}, longer than the netcode timeout of ${NETCODE_TIMEOUT_MS / 1000} s`
        : `the longest gap of the server tick log is ${(gap / 1000).toFixed(1)} s (before tick ${before})`;
    assert.fail(`${label}: host fault, the client lost its connection (reconnect_required) at checkpoint ${lost.input_call}, replica tick ` +
        `${lost.debug?.replicated_world?.tick}; ${stall}; the run is not a fault of the row`);
}

module.exports = {
    NETCODE_TIMEOUT_MS,
    assertConnectionHeld,
    assertNoDriverStop,
    assertNoResync,
    assertReplicaLag,
    bindingLabel,
    checkpointAt,
    checkpointBefore,
    checkpointForSave,
    checkpointsBetween,
    longestTickGap,
    readCheckpoints,
    replicaLag,
    replicaUnit,
    saved,
    screenshotIndexForSave,
    serverTickAt,
    serverTickIndex,
    unitTrack,
    widget,
    widgets,
};
