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

module.exports = {
    assertNoDriverStop,
    bindingLabel,
    checkpointAt,
    checkpointBefore,
    checkpointForSave,
    checkpointsBetween,
    readCheckpoints,
    replicaUnit,
    saved,
    screenshotIndexForSave,
    unitTrack,
    widget,
    widgets,
};
