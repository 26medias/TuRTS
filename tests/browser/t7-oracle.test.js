// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const { test } = require("node:test");
const { sceneProblem, t7FramesFromCheckpoints } = require("./t7-oracle.js");

function checkpoint(inputCall, x, extra = {}) {
    return {
        input_call: inputCall,
        window: 1,
        wall_time_ms: 1000 + inputCall,
        debug: {
            phase: "Joined { slot: 0 }",
            last_frame: { t: inputCall, lo_tick: 1, hi_tick: 2, fraction: 0.5, latest_tick: 2 },
            units: [{ unit_id: 1, content_id: "acme:scout", pos: [x, 4] }],
            ...extra.debug,
        },
        scene: { cameras_3d: 1, units: [{ unit_id: 1, translation: [x, 0.5, 4] }], ...extra.scene },
    };
}

test("one frames.jsonl line per new debug record and the scene check counts each line", () => {
    const out = t7FramesFromCheckpoints([checkpoint(0, 4), checkpoint(1, 4), checkpoint(2, 5), checkpoint(3, 6)]);
    assert.equal(out.frames.length, 4);
    assert.equal(out.result.scene_checked, 4);
    assert.deepEqual(out.result.final_pos, [6, 4]);
    assert.equal(out.result.content_id, "acme:scout");
    assert.equal(out.result.welcomed, true);
    assert.equal(out.sceneFailure, null);
    const repeated = { ...checkpoint(3, 6), input_call: 4 };
    assert.equal(t7FramesFromCheckpoints([checkpoint(3, 6), repeated]).frames.length, 1,
        "a repeated debug record is not a new frame");
});

test("the scene check goes red on two cameras, a second entity, or an off-position mesh", () => {
    const good = checkpoint(0, 4);
    assert.equal(sceneProblem(good.debug, good.scene), null);
    assert.match(sceneProblem(good.debug, { ...good.scene, cameras_3d: 2 }), /exactly 1 active 3D camera/);
    assert.match(sceneProblem(good.debug, { ...good.scene, units: [...good.scene.units, { unit_id: 2, translation: [0, 0, 0] }] }), /1 scene entity/);
    assert.match(sceneProblem(good.debug, { ...good.scene, units: [{ unit_id: 1, translation: [4.02, 0.5, 4] }] }), /not within 0.01/);
    const out = t7FramesFromCheckpoints([good, checkpoint(1, 5, { scene: { cameras_3d: 0 } })]);
    assert.equal(out.result.scene_checked, 1);
    assert.match(out.sceneFailure, /checkpoint 1/);
});
