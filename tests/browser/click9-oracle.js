// turts-test-role: oracle
"use strict";

const assert = require("node:assert/strict");

function unitPositions(tickLog, unitId) {
    return tickLog.split(/\r?\n/).filter(Boolean).map((line) => line.split(",")).filter((fields) => Number(fields[2]) === unitId).map((fields) => ({
        tick: Number(fields[0]),
        x: Number(fields[3]),
        y: Number(fields[4]),
    })).filter((position) => Number.isFinite(position.x) && Number.isFinite(position.y));
}

async function judge(view) {
    const input = view.inputLog();
    const scoutClick = input.find((entry) => entry.call === "click" && entry.args[2] === "left");
    const groundClick = input.find((entry) => entry.call === "click" && entry.args[2] === "right");
    assert(scoutClick, "CLICK9 must use the harness to left-click the Acme scout");
    assert(groundClick, "CLICK9 must use the harness to right-click ground");
    const mouseUpSeq = scoutClick.event_seq.at(-1);
    assert(Number.isSafeInteger(mouseUpSeq), "the scout pointerup must have a harness sequence number");

    const debug = await view.bridge("turts_debug");
    assert.deepEqual(debug.selection, [1], "the real left click must select the Acme scout");
    assert(Array.isArray(debug.results), "turts_debug.results must expose per-unit order results");
    assert(debug.results.some((result) => result.unit === 1 && result.status === "accepted"), "the real ground right-click must produce an accepted result for the scout");

    const retained = debug.frames?.retained_input;
    assert(Array.isArray(retained), "turts_debug.frames.retained_input must contain frames with applied input sequences since frames_clear");
    assert(retained.length > 0 && retained.length <= 4096, "retained input frames must be bounded at 4,096 records");
    const clickFrames = retained.filter((frame) => frame.applied_seq?.includes(mouseUpSeq));
    assert.equal(clickFrames.length, 1, "the scout pointerup sequence must apply in exactly one retained frame");
    const clickFrame = clickFrames[0];
    assert.equal(clickFrame.applied_seq.filter((seq) => seq === mouseUpSeq).length, 1, "the scout pointerup sequence must be drained once");
    assert.deepEqual(clickFrame.selection, [1], "the scout must be selected in its pointerup frame");
    const groundUpSeq = groundClick.event_seq.at(-1);
    assert(Number.isSafeInteger(groundUpSeq), "the ground pointerup must have a harness sequence number");
    const groundFrames = retained.filter((frame) => frame.applied_seq?.includes(groundUpSeq));
    assert.equal(groundFrames.length, 1, "the ground pointerup sequence must apply in exactly one retained frame");
    assert.equal(groundFrames[0].applied_seq.filter((seq) => seq === groundUpSeq).length, 1, "the ground pointerup sequence must be drained once");
    const recent = debug.frames.recent;
    assert(Array.isArray(recent) && recent.length > 0 && recent.length <= 64, "the recent frame ring must retain at most 64 frames");
    assert(recent.at(-1).frame_index - clickFrame.frame_index >= 200, "retention proof requires 200 actual presented frames after the pointerup, not an assumed frame rate");

    const positions = unitPositions(await view.serverFile("server-tick.log"), 1);
    assert(positions.length >= 2, "the server tick log must contain multiple scout positions after the order");
    const moved = positions.at(-1);
    const initial = positions[0];
    assert(Math.hypot(moved.x - initial.x, moved.y - initial.y) > 0.1, "the accepted ground order must move the scout in the server tick log");
}

module.exports = { judge };
