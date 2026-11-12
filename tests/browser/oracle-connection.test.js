// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const { test } = require("node:test");
const orders = require("./orders-oracle.js");
const mm1 = require("./mm1-oracle.js");

const T0 = 1794490000000;
// A tick log of 600 ticks with one stall of `stallMs` before tick 457, and one unit row for each tick.
function tickLog(stallMs) {
    const lines = [];
    let wall = T0;
    for (let tick = 1; tick <= 600; tick += 1) {
        wall += 50 + (tick === 457 ? stallMs : 0);
        lines.push(`${tick},${BigInt(wall) * 1000000n},1,1.5,2.5`);
    }
    return `${lines.join("\n")}\n`;
}

// A view over fake files. The checkpoint list has one checkpoint that reports a lost connection.
function fakeView(files) {
    return Object.freeze({
        inputLog: () => [],
        serverFile: async (name) => {
            if (!(name in files)) throw new Error(`ENOENT: ${name}`);
            return files[name];
        },
    });
}
const lostCheckpoint = JSON.stringify([{ input_call: 23, window: 1, page_time_ms: T0 + 30000,
    debug: { reconnect_required: true, replicated_world: { tick: 451 } }, scene: {} }]);

test("ORD rows are red with the host fault text when the client lost its connection, before any order check", async () => {
    const view = fakeView({ "scene-checkpoints.json": lostCheckpoint, "server-tick.log": tickLog(17850) });
    await assert.rejects(orders.judge(view, { row: "ORD4" }),
        /ORD4: host fault, the client lost its connection \(reconnect_required\) at checkpoint 23, replica tick 451; the server tick log has a stall of 17\.9 s before tick 457/);
});

test("MM1 is red with the host fault text when the client lost its connection, before any replica or order check", async () => {
    const rect = { x: 1072, y: 512, size: 200 };
    const view = fakeView({ "mm1-driver-steps.json": JSON.stringify({ rect }), "scene-checkpoints.json": lostCheckpoint, "server-tick.log": tickLog(11050) });
    const params = { viewport: { width: 1280, height: 720 }, minimap_inset: 8, minimap_size: 200, map_size: [64, 48] };
    await assert.rejects(mm1.judge(view, params), /MM1: host fault, the client lost its connection .* a stall of 11\.1 s before tick 457/);
});
