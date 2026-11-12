// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const { test } = require("node:test");
const { appliedAfter, cancelRows, hudClick, produceRow, waitForSharedTick } = require("./m2-oracle.js");

// Row shapes of a retained headed HUD2 run (target/mc-evidence/run94-hud2-headed-v3): list rows
// are "<list>/<item>[<key>]" buttons with a typed widget_action; row children keep their ids.
const produce = { kind: "button", id: "m2_build_rows/m2_build_item[Action:{}]", visible: true, enabled: true, text: "Produce Train Rifleman",
    rect: [16, 190, 240, 28], widget_action: { kind: "produce", producer: 1, content_id: "base:train_rifleman" } };
const cancel = { kind: "button", id: "m2_job_cancel", visible: true, enabled: true, text: "Cancel", rect: [192, 258, 64, 28],
    widget_action: { kind: "cancel_production", producer: 1, job: 7 } };

test("the Produce row is found only when it is visible, enabled and for the selected producer", () => {
    const debug = { ui_snapshot: [produce, cancel] };
    assert.equal(produceRow(debug, 1, "base:train_rifleman"), produce);
    assert.equal(produceRow(debug, 2, "base:train_rifleman"), null);
    assert.equal(produceRow({ ui_snapshot: [{ ...produce, enabled: false }] }, 1, null), null);
    assert.equal(produceRow({ ui_snapshot: [] }, 1, null), null, "a missing BuildMenu row must make the case red");
    assert.deepEqual(cancelRows(debug, 1).map((row) => row.widget_action.job), [7]);
});

test("a HUD click is the click after its button check, and the applied result must follow it", () => {
    const input = [
        { call: "screenshot" },
        { call: "save", args: ["m2-econ-first-button.json"] },
        { call: "click", args: [136, 204, "left"] },
        { call: "screenshot" },
    ];
    const before = { input_call: 0, window: 1, wall_time_ms: 1000, debug: { command_results: [] } };
    const after = { input_call: 3, window: 1, wall_time_ms: 1400, debug: { command_results: [{ seq: 1, applied_tick: 9, job_id: 4 }] } };
    const click = hudClick(input, [before, after], "m2-econ-first");
    assert.deepEqual([click.x, click.y, click.clickIndex], [136, 204, 2]);
    assert.equal(appliedAfter([before, after], click), after);
    assert.equal(appliedAfter([before, { ...after, wall_time_ms: 9000 }], click), null, "a result later than 6 s does not count");
    assert.throws(() => hudClick(input, [before, after], "m2-econ-second"), /did not check the HUD button/);
});

const { drive } = require("./m2-driver.js");
const { judge } = require("./m2-oracle.js");
const { deflateSync } = require("node:zlib");

function blankPng(width = 64, height = 36) {
    const crcTable = Array.from({ length: 256 }, (_, n) => {
        let c = n;
        for (let k = 0; k < 8; k += 1) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
        return c >>> 0;
    });
    const crc = (bytes) => { let c = 0xffffffff; for (const b of bytes) c = crcTable[(c ^ b) & 255] ^ (c >>> 8); return (c ^ 0xffffffff) >>> 0; };
    const chunk = (type, data) => {
        const head = Buffer.alloc(4); head.writeUInt32BE(data.length);
        const tail = Buffer.alloc(4); tail.writeUInt32BE(crc(Buffer.concat([Buffer.from(type), data])));
        return Buffer.concat([head, Buffer.from(type), data, tail]);
    };
    const header = Buffer.alloc(13); header.writeUInt32BE(width, 0); header.writeUInt32BE(height, 4); header.set([8, 2, 0, 0, 0], 8);
    const rows = Buffer.alloc((width * 3 + 1) * height);
    return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk("IHDR", header), chunk("IDAT", deflateSync(rows)), chunk("IEND", Buffer.alloc(0))]);
}

test("a product fault in the M2 driver ends the drive with a record, not a throw", async () => {
    const saved = [];
    const clicked = [];
    const io = {
        use: async () => {}, wait: async () => {}, move: async () => {}, click: async (x, y, button) => clicked.push([x, y, button]),
        screenshot: async () => blankPng(1280, 720), save: async (name) => saved.push(name),
    };
    await drive(io, { m2: { case_id: "E9-browser-base-economy", package: "base" } });
    assert.ok(saved.includes("m2-producer-driver-stop.json"), JSON.stringify(saved));
    assert.deepEqual(clicked, [[640, 690, "left"]], "only the Click to play overlay click");
});

test("a stopped M2 drive gives a semantic failure that names the case", async () => {
    const joined = {
        input_call: 0, window: 1, wall_time_ms: 1,
        scene: { cameras_3d: 1, clip_from_world: Array(16).fill(0), units: [{ unit_id: 1 }],
            render_adapter: { backend: "BrowserWebGpu", name: "gpu", device_type: "DiscreteGpu" } },
        debug: { phase: "Joined { slot: 0 }", slot: 0, units: [{ content_id: "base:command_center" }], command_results: [],
            replicated_world: { tick: 3, belts: [], economy: { balances: [], inventories: [], orders: [], queues: [] }, extractors: [],
                health_trace: [], healths: [], inserters: [], nodes: [], research: [], unit_orders: [], units: [], weapons: [] } },
    };
    const view = {
        inputLog: () => [{ call: "screenshot" }, { call: "save", args: ["m2-joined.png"] }],
        serverFile: async (name) => name === "scene-checkpoints.json" ? JSON.stringify([joined])
            : JSON.stringify({ package: "base", case_id: "E9-browser-base-economy", offset_x: 18, map_relative_path: "maps/flat.ron",
                packed_after_translation_sha256: "0".repeat(64) }),
    };
    await assert.rejects(judge(view, { m2: { case_id: "E9-browser-base-economy", package: "base" } }), (error) =>
        error instanceof assert.AssertionError && error.task16_semantic?.row === "M2" && error.task16_semantic.clause === "E9-browser-base-economy");
});

// A replicated world with the permitted keys only (m2-oracle-lib.js PUBLIC_WORLD_KEYS) and one unit.
function world(tick, x) {
    return { slot: 0, replicated_world: { belts: [], economy: { balances: [], inventories: [], orders: [], queues: [] }, extractors: [],
        health_trace: [], healths: [], inserters: [], nodes: [], research: [], tick, unit_orders: [], weapons: [],
        units: [{ content_id: "acme:scout", facing: 0, max_health: 10, owner_slot: 0, pos: [x, 4], unit_id: 1 }] } };
}

// A fake view: window `n` shows the next entry of `frames[n - 1]` at each read (the last entry stays).
function fakeView(frames) {
    const reads = [0, 0];
    return { async bridge(name, n) {
        assert.equal(name, "turts_debug");
        const list = frames[n - 1];
        const entry = list[Math.min(reads[n - 1], list.length - 1)];
        reads[n - 1] += 1;
        return entry;
    } };
}

const FAST = { min_ms: 0, stall_ms: 200, limit_ms: 2000, poll_ms: 1 };

test("the shared-tick wait compares the windows at a tick that each showed at a different poll", async () => {
    // Window 1 shows tick 20 at the third read; window 2 showed tick 20 at the first read only.
    const view = fakeView([[world(10, 4), world(15, 4), world(20, 5)], [world(20, 5), world(30, 6)]]);
    assert.deepEqual(await waitForSharedTick(view, FAST), { tick: 20 });
});

test("the shared-tick wait stays red when the projections differ at each shared tick", async () => {
    const view = fakeView([[world(10, 4), world(20, 5)], [world(10, 9), world(20, 9)]]);
    await assert.rejects(waitForSharedTick(view, FAST), /differ between owners at a shared coherent tick .* 2 shared ticks, 2 of them different/);
});

test("the shared-tick wait stays red when the windows never show the same tick", async () => {
    const view = fakeView([[world(10, 4), world(12, 4)], [world(11, 4), world(13, 4)]]);
    await assert.rejects(waitForSharedTick(view, FAST), /0 shared ticks/);
});
