// turts-test-role: unit
"use strict";
const assert = require("node:assert/strict");
const test = require("node:test");
const vm = require("node:vm");
const { createSceneCheckpoints } = require("./scene-checkpoints.js");

function page(identity) {
    const state = {
        scene: { clip_from_world: [identity, 0, 0, 1] },
        debug: { selection: [identity] },
    };
    const window = {
        turts_scene: () => JSON.stringify(state.scene),
        turts_debug: () => JSON.stringify(state.debug),
        get turts_move() { throw new Error("a checkpoint must never access the order bridge"); },
    };
    return { state, evaluate: async (fn) => vm.runInNewContext(`(${fn.toString()})()`, { window }) };
}

const metadata = (window = 1, input_call = 0) => ({ window, input_call, wall_time_ms: 123 });

test("scene checkpoints read the selected page and retain each stage independently", async () => {
    const pages = [page(1), page(2)];
    const checkpoints = createSceneCheckpoints(pages);
    await checkpoints.record(metadata(2, 7));
    pages[1].state.scene.clip_from_world[0] = 9;
    pages[1].state.debug.selection = [];
    await checkpoints.record(metadata(2, 10));
    const rows = checkpoints.snapshots();
    assert.deepEqual(rows[0], { ...metadata(2, 7), scene: { clip_from_world: [2, 0, 0, 1] }, debug: { selection: [2] } });
    assert.deepEqual(rows[1].scene.clip_from_world, [9, 0, 0, 1]);
    rows[0].scene.clip_from_world[0] = -1;
    assert.equal(checkpoints.snapshots()[0].scene.clip_from_world[0], 2);
});

test("checkpoint failures are explicit for absent or malformed bridges", async () => {
    for (const scene of [null, "{broken", "[]"]) {
        const checkpoints = createSceneCheckpoints([{ evaluate: async () => ({ scene, debug: "{}" }) }]);
        await assert.rejects(checkpoints.record(metadata()));
        assert.deepEqual(checkpoints.snapshots(), []);
    }
});

test("checkpoint identity and storage bound reject invalid observations", async () => {
    const checkpoints = createSceneCheckpoints([page(1)], 1);
    for (const value of [metadata(0), metadata(2), metadata(1, -1), { ...metadata(), wall_time_ms: NaN }]) {
        await assert.rejects(checkpoints.record(value), /identity/);
    }
    await checkpoints.record(metadata());
    await assert.rejects(checkpoints.record(metadata(1, 1)), /limit/);
    assert.equal(checkpoints.snapshots().length, 1);
});
