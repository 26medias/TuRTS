// turts-test-role: oracle
"use strict";

// Read-only helpers for the oracles of the MC ledger rewrites (t7, m3, pt1, m2). The harness
// writes one scene checkpoint for each io.screenshot() (harness/scene-checkpoints.js); a
// checkpoint holds the scene and debug bridges that the harness read after that screenshot.
const assert = require("node:assert/strict");
const os = require("node:os");

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

// The units that left the server: `{ unit, last_tick }` for each unit whose last line of the tick log is before the
// last tick of the log, in the order of their last tick. A unit that is removed (a kill) stops having a line.
function unitRemovals(tickLog) {
    const lastTick = new Map();
    let maxTick = -1;
    let from = 0;
    while (from < tickLog.length) {
        let end = tickLog.indexOf("\n", from);
        if (end < 0) end = tickLog.length;
        const first = tickLog.indexOf(",", from);
        const second = first < 0 || first >= end ? -1 : tickLog.indexOf(",", first + 1);
        const third = second < 0 || second >= end ? -1 : tickLog.indexOf(",", second + 1);
        if (third > second) {
            const tick = Number(tickLog.slice(from, first));
            const unit = Number(tickLog.slice(second + 1, third));
            assert(Number.isSafeInteger(tick) && Number.isSafeInteger(unit), `the server tick log has a bad row: ${tickLog.slice(from, end)}`);
            lastTick.set(unit, tick);
            if (tick > maxTick) maxTick = tick;
        }
        from = end + 1;
    }
    return [...lastTick].filter(([, tick]) => tick < maxTick).map(([unit, tick]) => ({ unit, last_tick: tick }))
        .sort((left, right) => left.last_tick - right.last_tick || left.unit - right.unit);
}

// What the oracles read of the server of a run: the tick clock, the removals, and (when the launcher started the
// server with `--stats`) the statistics file with `tick_max_us`, the longest compute time of one tick of the run
// (crates/turts_server/src/stats.rs:213-215). `load` is the load average of the host when the oracle judges.
async function readServerEvidence(view, load = os.loadavg()) {
    const tickLog = await view.serverFile("server-tick.log");
    let stats = null;
    try { stats = JSON.parse(await view.serverFile("server-stats.json")); } catch { stats = null; }
    return { index: serverTickIndex(tickLog), removals: unitRemovals(tickLog), stats, load };
}

// A replica that never lost a message asks for no resync. The stall after a kill (RUN142) is a run of
// rejected updates and resync requests: `ingest.resync_requests_sent` counts them for the whole run, so a
// capture that misses the stall window still shows it. Saved MM1 runs: 36 of 36 runs without the repair of
// commit 50eb827 show 9 to 24 requests in window 1, the 8 runs with the repair show 0. A local run has no packet
// loss, so any request is a fault. The text names the first checkpoint with a count, the nearest unit removal of
// the tick log and the longest server tick gap, so the reader can tell a kill (RUN142) from a host stall.
function assertNoResync(checkpoints, label, evidence) {
    let previousTick = null;
    for (const entry of checkpoints) {
        const ingest = entry.debug?.ingest;
        assert(ingest && Number.isSafeInteger(ingest.resync_requests_sent),
            `${label}: checkpoint ${entry.input_call} (window ${entry.window}) has no ingest.resync_requests_sent`);
        const tick = entry.debug?.replicated_world?.tick;
        if (ingest.resync_requests_sent !== 0) {
            const parts = [`first non-zero count at checkpoint ${entry.input_call}, replica tick ${tick}` +
                (previousTick === null ? "" : ` (the checkpoint before it, of the same window, was at tick ${previousTick} with none)`)];
            const removals = evidence?.removals ?? [];
            const nearest = removals.reduce((best, removal) =>
                (best === null || Math.abs(removal.last_tick - tick) < Math.abs(best.last_tick - tick) ? removal : best), null);
            parts.push(nearest === null ? "no unit left the server tick log"
                : `nearest unit removal: unit ${nearest.unit}, last tick ${nearest.last_tick} (${nearest.last_tick - tick} ticks from the replica tick)`);
            if (evidence?.index) {
                const { gap_ms: gap, before_tick: before } = longestTickGap(evidence.index);
                parts.push(`longest server tick gap ${(gap / 1000).toFixed(2)} s before tick ${before}`);
            }
            assert.fail(`${label}: the replica of window ${entry.window} asked the server for ${ingest.resync_requests_sent} resyncs by ` +
                `checkpoint ${entry.input_call} (replica tick ${tick}); a local run needs none (the kill stall of RUN142); ${parts.join("; ")}`);
        }
        if (entry.window === 1) previousTick = tick;
    }
}

// The netcode timeout of the client (crates/turts_client/src/join.rs:345, client_timeout_secs).
const NETCODE_TIMEOUT_MS = 10000;
// A tick whose compute time is at least this share of the stall gap is a slow tick of the server itself.
const SLOW_TICK_SHARE = 0.5;

// The longest gap between two ticks of the server, and the tick that ends it.
function longestTickGap(index) {
    let longest = { gap_ms: 0, before_tick: null };
    for (let at = 1; at < index.length; at += 1) {
        const gap = index[at].wall_ms - index[at - 1].wall_ms;
        if (gap > longest.gap_ms) longest = { gap_ms: gap, before_tick: index[at].tick };
    }
    return longest;
}

// The client reports `reconnect_required` after the transport ended (crates/turts_client/src/join.rs:416-424), and its
// replica stands still from then on. The cause is told apart from the evidence of the run, never assumed:
// - the server stalled longer than the netcode timeout (gap in the tick log) and its slowest tick computed in a
//   small part of that gap (`tick_max_us` of --stats): "host fault", the server was not scheduled (run
//   rereview-2df2393-hold-a: stalls of 11.0 s and 17.9 s under three other heavy jobs);
// - a tick computed for at least half of the gap: "slow server tick", a product suspect;
// - no statistics file, or a connection lost with no server stall: undecided, a product suspect. The row is red in
//   every case. The load average is the one of the host when the oracle judges.
function assertConnectionHeld(checkpoints, evidence, label) {
    const lost = checkpoints.find((entry) => entry.debug?.reconnect_required === true);
    if (!lost) return;
    const { gap_ms: gap, before_tick: before } = longestTickGap(evidence.index);
    const maxTickMs = Number.isFinite(evidence.stats?.tick_max_us) ? evidence.stats.tick_max_us / 1000 : null;
    const gapText = `longest server tick gap ${(gap / 1000).toFixed(1)} s before tick ${before} (netcode timeout ${NETCODE_TIMEOUT_MS / 1000} s)`;
    const computeText = maxTickMs === null ? "no server-stats.json: the tick compute time is unknown"
        : `slowest tick compute ${maxTickMs.toFixed(1)} ms (tick_max_us of --stats)`;
    const loadText = Array.isArray(evidence.load) ? `; load average of the host at judgement ${evidence.load.map((value) => value.toFixed(2)).join(" ")}` : "";
    let verdict;
    if (gap <= NETCODE_TIMEOUT_MS) verdict = "connection lost with no server stall: cause undecided, a product suspect";
    else if (maxTickMs === null) verdict = "stall of the server with unknown tick compute time: cause undecided, a product suspect";
    else if (maxTickMs >= gap * SLOW_TICK_SHARE) verdict = "slow server tick: a product suspect (one tick computed for most of the gap)";
    else verdict = "host fault: the server was not scheduled (tick compute is small against the gap)";
    assert.fail(`${label}: the client lost its connection (reconnect_required) at checkpoint ${lost.input_call}, replica tick ` +
        `${lost.debug?.replicated_world?.tick}; ${gapText}; ${computeText}${loadText}; ${verdict}`);
}

// The kill path of MM1 (RUN142): an enemy unit must leave the server before the capture that follows the minimap
// right click. A run in which no enemy dies has not run the path that stalled the replica, so it is void, not green.
// `enemyIds` are the units of the other slot that the replica showed.
function assertKillPathRan(enemyIds, removals, index, rightCapture, label) {
    const rightTick = serverTickAt(index, rightCapture.page_time_ms);
    assert(Number.isSafeInteger(rightTick), `${label}: the after-right capture has no server tick`);
    const killed = removals.filter((removal) => enemyIds.has(removal.unit) && removal.last_tick < rightTick);
    assert(killed.length > 0, `${label}: void run, the kill path did not run: no enemy unit (${[...enemyIds].join(",")}) left the server tick log ` +
        `before server tick ${rightTick} of the after-right capture; the run proves nothing about the replica stall of RUN142`);
    return killed;
}

module.exports = {
    NETCODE_TIMEOUT_MS,
    assertConnectionHeld,
    assertKillPathRan,
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
    readServerEvidence,
    replicaLag,
    replicaUnit,
    saved,
    screenshotIndexForSave,
    serverTickAt,
    serverTickIndex,
    unitRemovals,
    unitTrack,
    widget,
    widgets,
};
