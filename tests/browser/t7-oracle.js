// turts-test-role: oracle
"use strict";

// T7 oracle (QA0 design 4.6 option C, conditions 1 to 4; MC design 13.3). It also keeps every
// assertion of the legacy tests/browser/t7.js: one active 3D camera and one rendered unit at the
// presented position in each recorded checkpoint, the expected content id, and the final
// presented position. t7FramesFromCheckpoints() gives scripts/t7.sh the frames.jsonl and
// result.json that the Rust segment oracle (crates/turts_server/tests/t7_oracle.rs) reads.
const assert = require("node:assert/strict");
const { assertNoDriverStop, checkpointBefore, readCheckpoints, unitTrack } = require("./checkpoint-lib.js");
const { unprojectGround } = require("./hud4-oracle.js");

// QA0D:434-436: SCENE_TOLERANCE 0.01 (unprojection) plus POS_TOLERANCE 0.001 (arrival).
const GROUND_TOLERANCE = 0.011;
const POS_TOLERANCE = 1e-3;
const SCENE_TOLERANCE = 0.01;
const REST_TICKS = 3;

function sceneProblem(debug, scene) {
    if (scene === null || typeof scene !== "object" || !Array.isArray(scene.units)) {
        return `turts_scene gave ${JSON.stringify(scene)}`;
    }
    if (scene.cameras_3d !== 1) return `expected exactly 1 active 3D camera, saw ${JSON.stringify(scene.cameras_3d)}`;
    if (!Array.isArray(debug?.units) || debug.units.length !== 1) return `expected 1 presented unit, saw ${debug?.units?.length}`;
    if (scene.units.length !== 1) return `expected 1 scene entity with a mesh and a unit id, saw ${scene.units.length}`;
    const unit = debug.units[0];
    const entity = scene.units[0];
    if (entity.unit_id !== unit.unit_id) return `scene unit_id ${entity.unit_id} != presented unit_id ${unit.unit_id}`;
    const [x, , z] = entity.translation;
    const off = [Math.abs(x - unit.pos[0]), Math.abs(z - unit.pos[1])];
    if (off.some((d) => !(d <= SCENE_TOLERANCE))) {
        return `scene translation ${JSON.stringify(entity.translation)} is not within ${SCENE_TOLERANCE} of presented pos ${JSON.stringify(unit.pos)} (x -> x, y -> z)`;
    }
    return null;
}

// One frames.jsonl line per checkpoint with a new debug record (the legacy poll recorded each
// distinct debug string once). `at_ms` is the page clock that the checkpoint read in the same
// evaluate as the debug record (harness/scene-checkpoints.js), as t7.js:133-137 did, so it is not
// earlier than the frame: the lag that the Rust oracle works out stays an upper bound.
function t7FramesFromCheckpoints(checkpoints) {
    const frames = [];
    let previous = null;
    let sceneChecked = 0;
    let sceneFailure = null;
    for (const checkpoint of [...checkpoints].sort((left, right) => left.input_call - right.input_call)) {
        if (checkpoint.window !== 1) continue;
        const debug = checkpoint.debug;
        const text = JSON.stringify(debug);
        if (text === previous || !debug?.last_frame || !Array.isArray(debug.units) || debug.units.length === 0) continue;
        previous = text;
        if (!Number.isFinite(checkpoint.page_time_ms)) {
            throw new Error(`checkpoint ${checkpoint.input_call} has no browser clock (page_time_ms); the frame time is NOT TRACED`);
        }
        frames.push(JSON.stringify({ at_ms: checkpoint.page_time_ms, last_frame: debug.last_frame, units: debug.units, scene: checkpoint.scene }));
        const problem = sceneProblem(debug, checkpoint.scene);
        if (problem === null) sceneChecked += 1;
        else if (sceneFailure === null) sceneFailure = `checkpoint ${checkpoint.input_call}: ${problem}`;
    }
    const last = [...checkpoints].filter((entry) => entry.window === 1).sort((left, right) => right.input_call - left.input_call)[0];
    const units = last?.debug?.units ?? [];
    return {
        frames,
        sceneFailure,
        result: {
            welcomed: typeof last?.debug?.phase === "string" && last.debug.phase.startsWith("Joined"),
            units: units.length,
            content_id: units.length === 1 ? units[0].content_id : null,
            shape: units.length === 1 ? units[0].shape ?? null : null,
            final_pos: units.length === 1 ? units[0].pos : null,
            frames: frames.length,
            phase: last?.debug?.phase ?? null,
            scene_checked: sceneChecked,
        },
    };
}

async function judgeRun(view, params) {
    const input = view.inputLog();
    const lefts = input.map((entry, index) => ({ entry, index })).filter(({ entry }) => entry.call === "click" && entry.args?.[2] === "left");
    const rights = input.map((entry, index) => ({ entry, index })).filter(({ entry }) => entry.call === "click" && entry.args?.[2] === "right");
    // Condition 1 (QA0D:422-425): one right click at (px, py) after the left click. The first click
    // of a fresh page goes to the "Click to play" overlay (crates/turts_client/src/client_shell.rs
    // sync_focus_overlay: audio locked until a gesture), so the selecting left click is the last
    // left click before the right click; at most one more left click (the overlay) comes first.
    assert.equal(rights.length, 1, "T7 condition 1: the input log must hold exactly one right click");
    const selecting = lefts.filter(({ index }) => index < rights[0].index);
    assert(selecting.length >= 1, "T7 condition 1: the right click must come after the left click");
    assert(lefts.length <= 2 && lefts.every(({ index }) => index < rights[0].index),
        "T7 condition 1: the input log holds at most the overlay click and one left click, both before the right click");
    const [px, py] = rights[0].entry.args;
    assert(Number.isFinite(px) && Number.isFinite(py), "T7 condition 1: the right click has no pixel");

    const checkpoints = await readCheckpoints(view);
    const atClick = checkpointBefore(checkpoints, rights[0].index, 1);
    assert(atClick?.debug && atClick?.scene, "T7 needs the checkpoint of the screenshot before the right click");
    assert(Array.isArray(atClick.debug.selection) && atClick.debug.selection.length === 1,
        "T7 the left click on the unit colour must select exactly one unit");
    const unitId = atClick.debug.selection[0];
    const clickTick = atClick.debug.replicated_world?.tick ?? atClick.debug.last_frame?.latest_tick;
    assert(Number.isInteger(clickTick), "T7 the checkpoint before the right click has no replicated tick");

    // Condition 2: one command result with an applied tick after the right click.
    const debug = await view.bridge("turts_debug");
    const applied = (debug.command_results ?? []).filter((result) => Number.isInteger(result.applied_tick));
    assert.equal(applied.length, 1, `T7 condition 2: expected one applied command result, saw ${JSON.stringify(debug.command_results)}`);
    assert(applied[0].applied_tick > clickTick,
        `T7 condition 2: applied tick ${applied[0].applied_tick} is not after the right-click tick ${clickTick}`);

    // Condition 3: the ground point G under (px, py) from the camera of the same checkpoint;
    // [x, z] of the scene is (x, y) of the map (crates/turts_client/src/input.rs:14-21).
    const ground = unprojectGround(atClick.scene, { x: px, y: py });

    // Condition 4: the unit at rest in the server tick log stands within 0.011 of G.
    const track = unitTrack(await view.serverFile("server-tick.log"), unitId);
    assert(track.length > REST_TICKS, "T7 condition 4: the server tick log has too few lines for the unit");
    const tail = track.slice(-REST_TICKS);
    assert(tail.every((point) => point.x === tail[0].x && point.y === tail[0].y),
        `T7 condition 4: the unit is not at rest in the last ${REST_TICKS} tick-log lines: ${JSON.stringify(tail)}`);
    const rest = tail.at(-1);
    const [groundX, groundY] = ground;
    const distance = Math.hypot(rest.x - groundX, rest.y - groundY);
    assert(distance <= GROUND_TOLERANCE,
        `T7 condition 4: the unit rests at (${rest.x}, ${rest.y}), ${distance.toFixed(5)} from G (${groundX}, ${groundY}); limit ${GROUND_TOLERANCE}`);

    // The legacy t7.js checks, now on the checkpoints.
    const { frames, sceneFailure, result } = t7FramesFromCheckpoints(checkpoints);
    assert(frames.length > 0, "no frame was recorded");
    assert.equal(sceneFailure, null, `scene: ${sceneFailure} (${frames.length - result.scene_checked} of ${frames.length} checkpoints failed)`);
    assert.equal(result.units, 1, `expected 1 presented unit, saw ${result.units}`);
    assert.equal(result.content_id, params.expect_content_id, `content_id ${result.content_id} != expected ${params.expect_content_id}`);
    const off = result.final_pos.map((value, index) => Math.abs(value - [rest.x, rest.y][index]));
    assert(off.every((d) => d <= POS_TOLERANCE),
        `final position ${JSON.stringify(result.final_pos)} is not within ${POS_TOLERANCE} of the server rest position ${JSON.stringify([rest.x, rest.y])}`);
}

// The run checks first, then the driver stop record (RUN123 L2).
async function judge(view, params) {
    const result = await judgeRun(view, params);
    await assertNoDriverStop(view, "T7");
    return result;
}

module.exports = { judge, sceneProblem, t7FramesFromCheckpoints };
