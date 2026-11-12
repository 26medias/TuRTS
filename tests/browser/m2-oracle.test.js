// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const { test } = require("node:test");
const { appliedAfter, cancelRows, hudClick, produceRow } = require("./m2-oracle.js");

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

// ---- The convergence sampler under the 4 Hz cap of the debug request (L0-DBG, row DBG1) ----------
const { sampleSharedTick } = require("./m2-oracle.js");

// A virtual clock and two windows. A window builds its record in the first frame of its own 16 ms
// frame grid (window 2 is `skew` ms off) that comes at least 250 ms after its last build; the
// request resolves 3 ms later. The tick is 50 ms long (20 Hz). A call made at the same virtual
// time as another one sees the same clock: the clock moves when a promise resolves.
function virtualWindows(skew) {
    const clock = { t: 0 };
    const last = [-1e9, -1e9];
    const view = { bridge: (_name, windowNumber) => {
        const offset = windowNumber === 2 ? skew : 0;
        const ready = Math.max(clock.t, last[windowNumber - 1] + 250);
        const build = Math.ceil((ready - offset) / 16) * 16 + offset;
        last[windowNumber - 1] = build;
        const record = { replicated_world: { tick: Math.floor(build / 50) } };
        return Promise.resolve().then(() => { clock.t = Math.max(clock.t, build + 3); return record; });
    } };
    return { clock, view, now: () => clock.t, sleep: async (ms) => { clock.t += ms; } };
}

const SKEWS = Array.from({ length: 17 }, (_, index) => index * 3);
const sameProjection = () => ({});

test("the shared-tick sampler finds one tick in both windows for every phase of the frame grids", async () => {
    for (const skew of SKEWS) {
        const windows = virtualWindows(skew);
        const shared = await sampleSharedTick(windows.view, { now: windows.now, sleep: windows.sleep, project: sameProjection });
        assert.ok(shared, `no shared tick with window 2 ${skew} ms off`);
        assert.ok(windows.clock.t < 10000, "inside the 10 s limit");
    }
});

test("the sampler waits longer than the 250 ms cap between rounds, and not by the same time in each round", async () => {
    const waits = [];
    const windows = virtualWindows(0);
    // Two windows that never agree, so that the sampler keeps asking.
    const never = { bridge: (name, windowNumber) => windows.view.bridge(name, windowNumber).then((record) => ({ replicated_world: { tick: windowNumber } })) };
    await sampleSharedTick(never, { now: windows.now, sleep: async (ms) => { waits.push(ms); windows.clock.t += ms; }, project: sameProjection });
    assert.ok(waits.length >= 10, `rounds: ${waits.length}`);
    assert.ok(waits.every((ms) => ms > 250), "each wait is longer than the 250 ms cap");
    assert.ok(new Set(waits).size > 2, "the wait changes from round to round");
});

test("the two windows are asked at once, not one after the other", async () => {
    const order = [];
    const view = { bridge: (_name, windowNumber) => { order.push(`ask${windowNumber}`); return Promise.resolve().then(() => { order.push(`got${windowNumber}`); return { replicated_world: { tick: 1 } }; }); } };
    await sampleSharedTick(view, { sleep: async () => {}, project: sameProjection });
    assert.deepEqual(order, ["ask1", "ask2", "got1", "got2"]);
});

test("the sampler needs one projection: equal ticks with different public worlds do not count", async () => {
    const windows = virtualWindows(0);
    let rounds = 0;
    const result = await sampleSharedTick(windows.view, { now: windows.now, sleep: async (ms) => { rounds += 1; windows.clock.t += ms; },
        project: (() => { let calls = 0; return () => ({ n: calls++ }); })(), deadlineMs: 3000 });
    assert.equal(result, null);
    assert.ok(rounds > 3);
});
