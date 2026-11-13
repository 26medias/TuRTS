// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const { test } = require("node:test");
const { appliedAfter, cancelRefundShot, cancelRows, convergenceLimits, hudClick, produceRow, waitForSharedTick } = require("./m2-oracle.js");

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
    const before = { input_call: 0, window: 1, wall_time_ms: 1000, debug: { tick_hz: 20, replicated_world: { tick: 5 }, command_results: [] } };
    const after = { input_call: 3, window: 1, wall_time_ms: 1400, debug: { tick_hz: 20, replicated_world: { tick: 12 }, command_results: [{ seq: 1, applied_tick: 9, job_id: 4 }] } };
    const click = hudClick(input, [before, after], "m2-econ-first");
    assert.deepEqual([click.x, click.y, click.clickIndex], [136, 204, 2]);
    assert.equal(appliedAfter([before, after], click), after);
    assert.throws(() => hudClick(input, [before, after], "m2-econ-second"), /did not check the HUD button/);
});

// Application latency is judged in server ticks (review of hosted runs 43242960493 and 43264331478: the wall
// time between two harness checkpoints reached 6.3 s and 6.6 s on the software renderer while the server had
// applied the order 44 and 46 ticks (2.2 s and 2.3 s) after the pre-click checkpoint).
const clickInput = [
    { call: "screenshot" },
    { call: "save", args: ["m2-econ-first-button.json"] },
    { call: "click", args: [136, 204, "left"] },
    { call: "screenshot" },
];
const shotAt = (call, wall, tick, results = [], hz = 20) => ({ input_call: call, window: 1, wall_time_ms: wall,
    debug: { tick_hz: hz, replicated_world: { tick }, command_results: results } });
const applied = (tick, seq = 1) => ({ seq, applied_tick: tick, job_id: 4 });
const clickOf = (before) => hudClick(clickInput, [before], "m2-econ-first");

test("a slow checkpoint is fine when the server applied the order within 6 s of ticks", () => {
    const before = shotAt(0, 20700, 557);
    const slow = shotAt(3, 27300, 685, [applied(601)]);
    assert.equal(appliedAfter([before, slow], clickOf(before)), slow, "6.6 s of wall time, 44 ticks of latency: the failing run of 43242960493");
    const cedar = shotAt(3, 32700, 864, [applied(781)]);
    const cedarBefore = shotAt(0, 26400, 735);
    assert.equal(appliedAfter([cedarBefore, cedar], clickOf(cedarBefore)), cedar, "6.3 s of wall time, 46 ticks: the failing run of 43264331478");
});

test("the bound is 6 s of ticks, read from the run: 120 ticks at 20 Hz, 360 at 60 Hz; 130 ticks at 20 Hz is red", () => {
    const before = shotAt(0, 1000, 500);
    assert.equal(appliedAfter([before, shotAt(3, 1400, 700, [applied(620)])], clickOf(before))?.debug.command_results[0].applied_tick, 620, "exactly 120 ticks");
    assert.equal(appliedAfter([before, shotAt(3, 1400, 700, [applied(630)])], clickOf(before)), null, "130 ticks at 20 Hz");
    const fast = shotAt(0, 1000, 500, [], 60);
    assert.notEqual(appliedAfter([fast, shotAt(3, 1400, 900, [applied(860)], 60)], clickOf(fast)), null, "360 ticks at 60 Hz");
    assert.equal(appliedAfter([fast, shotAt(3, 1400, 900, [applied(861)], 60)], clickOf(fast)), null, "361 ticks at 60 Hz");
});

test("no application is red, and a count that did not grow is red", () => {
    const before = shotAt(0, 1000, 500);
    assert.equal(appliedAfter([before, shotAt(3, 1400, 540, [])], clickOf(before)), null);
    const had = shotAt(0, 1000, 500, [applied(480, 1)]);
    assert.equal(appliedAfter([had, shotAt(3, 1400, 540, [applied(480, 1)])], clickOf(had)), null);
});

test("a result of an earlier command (applied before the pre-click checkpoint tick) does not count for the click", () => {
    const before = shotAt(0, 1000, 500);
    assert.equal(appliedAfter([before, shotAt(3, 1400, 540, [applied(499)])], clickOf(before)), null);
    assert.notEqual(appliedAfter([before, shotAt(3, 1400, 540, [applied(500)])], clickOf(before)), null);
});

test("a lagging replica cannot make a late application look early", () => {
    // The pre-click checkpoint shows replica tick 100 while the server is at 300: the order is applied at 330.
    // The measured latency is 230 ticks (more than the true 30), so it is red, never early.
    const lagging = shotAt(0, 1000, 100);
    assert.equal(appliedAfter([lagging, shotAt(3, 1400, 340, [applied(330)])], clickOf(lagging)), null);
});

test("a far wall-clock guard still catches a dead run: a checkpoint 60 s later does not count", () => {
    const before = shotAt(0, 1000, 500);
    assert.notEqual(appliedAfter([before, shotAt(3, 1000 + 60000, 600, [applied(560)])], clickOf(before)), null, "60 s is the limit, it is allowed");
    assert.equal(appliedAfter([before, shotAt(3, 1000 + 60001, 600, [applied(560)])], clickOf(before)), null);
});

test("the tick rate and the pre-click tick must exist, or the check refuses to judge", () => {
    const noRate = { ...shotAt(0, 1000, 500), debug: { replicated_world: { tick: 500 }, command_results: [] } };
    assert.throws(() => appliedAfter([noRate, shotAt(3, 1400, 540, [applied(520)])], clickOf(noRate)), /tick_hz/);
    const noTick = { ...shotAt(0, 1000, 500), debug: { tick_hz: 20, replicated_world: {}, command_results: [] } };
    assert.throws(() => appliedAfter([noTick, shotAt(3, 1400, 540, [applied(520)])], clickOf(noTick)), /replicated tick/);
});

test("the Cancel refund window uses the same tick bound (cancelRefundShot)", () => {
    const before = shotAt(0, 1000, 800, [applied(500, 1)]);
    const refunded = (tick, appliedTick, wall) => ({ ...shotAt(3, wall, tick, [applied(500, 1), applied(appliedTick, 2)]),
        debug: { tick_hz: 20, replicated_world: { tick, economy: { queues: [{ producer: 1, jobs: [] }], balances: [{ resource: "base:credits", on_hand: 350 }] } },
            command_results: [applied(500, 1), applied(appliedTick, 2)], ui_snapshot: [] } });
    const cancelClick = { clickIndex: 2, before };
    assert.notEqual(cancelRefundShot([before, refunded(900, 840, 7700)], cancelClick, 1, 2, 350), null, "6.7 s of wall time, 40 ticks");
    assert.equal(cancelRefundShot([before, refunded(1100, 930, 7700)], cancelClick, 1, 2, 350), null, "130 ticks late");
    assert.equal(cancelRefundShot([before, refunded(900, 840, 7700)], cancelClick, 1, 2, 400), null, "the credits must still be exact");
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
// MP1 (L0-DBG) tested its own sampler `sampleSharedTick`. The merge with MC keeps one sampler,
// `waitForSharedTick` (the MC judgement and profiles with the MP1 sampling). These cases run it with
// the gate profile and a limit of `deadlineMs`, and read a red judgement as `null` (no shared tick).
async function sampleSharedTick(view, { deadlineMs = 10000, ...hooks } = {}) {
    try {
        return await waitForSharedTick(view, { ...convergenceLimits({}), min_ms: deadlineMs, limit_ms: deadlineMs }, hooks);
    } catch (error) {
        if (error instanceof assert.AssertionError) return null;
        throw error;
    }
}

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

// Limits of the unit tests. cross_poll: true is the compare mode of the wide profile.
const FAST = { min_ms: 0, stall_ms: 200, limit_ms: 2000, poll_ms: 1, cross_poll: true };

test("the shared-tick wait compares the windows at a tick that each showed at a different poll", async () => {
    // Window 1 shows tick 20 at the third read; window 2 showed tick 20 at the first read only.
    const view = fakeView([[world(10, 4), world(15, 4), world(20, 5)], [world(20, 5), world(30, 6)]]);
    const result = await waitForSharedTick(view, FAST);
    assert.equal(result.tick, 20);
    // The time to agreement and the counters are part of the result (the harness log prints them).
    assert.equal(result.profile, "custom");
    assert.equal(result.compare, "cross_poll");
    assert.equal(result.limit_ms, FAST.limit_ms);
    assert.ok(Number.isInteger(result.agreed_after_ms) && result.agreed_after_ms >= 0 && result.agreed_after_ms < FAST.limit_ms, JSON.stringify(result));
    assert.equal(result.shared_ticks, 1);
    assert.equal(result.unstable_ticks, 0);
});

test("the shared-tick wait stays red when the projections differ at each shared tick", async () => {
    const view = fakeView([[world(10, 4), world(20, 5)], [world(10, 9), world(20, 9)]]);
    await assert.rejects(waitForSharedTick(view, FAST), /differ between owners at a shared coherent tick .* 2 shared ticks, 2 of them different/);
});

test("the shared-tick wait stays red when the windows never show the same tick", async () => {
    const view = fakeView([[world(10, 4), world(12, 4)], [world(11, 4), world(13, 4)]]);
    await assert.rejects(waitForSharedTick(view, FAST), /0 shared ticks/);
});

// A view that never ends: each read of window n shows the next tick (the two windows share every
// tick) and `project(n)` gives its x. It throws after `maxReads` reads, so a wait that ignores its
// limit ends the test with an error and not with a hang.
function endlessView(project, maxReads) {
    const reads = [0, 0];
    return { async bridge(name, n) {
        assert.equal(name, "turts_debug");
        reads[n - 1] += 1;
        if (reads[0] + reads[1] > maxReads) throw new Error(`the wait ignored its limit: ${maxReads} reads`);
        return world(reads[n - 1], project(n));
    } };
}

test("the shared-tick wait ends at its limit, with the named failure, on a stream that differs for ever", { timeout: 20000 }, async () => {
    // Both windows advance at each read, so the stall rule never ends the wait: only the limit can.
    const limits = { name: "wide", min_ms: 0, stall_ms: 1000000, limit_ms: 300, poll_ms: 1, cross_poll: true };
    const started = Date.now();
    await assert.rejects(waitForSharedTick(endlessView((n) => (n === 1 ? 4 : 9), 5000), limits),
        /differ between owners at a shared coherent tick \(profile wide, compare cross_poll, limit 300 ms, \d+ ms, \d+ polls, [1-9]\d* shared ticks, [1-9]\d* of them different/);
    const elapsed = Date.now() - started;
    assert.ok(elapsed >= 300 && elapsed < 5000, `the wait took ${elapsed} ms for a limit of 300 ms`);
});

test("the gate profile is the default and keeps the limit of 10 s; the wide profile needs the environment value", () => {
    const gate = convergenceLimits({});
    assert.deepEqual([gate.name, gate.min_ms, gate.limit_ms, gate.cross_poll], ["gate", 10000, 10000, false]);
    assert.equal(convergenceLimits({ TURTS_M2_CONVERGENCE_LIMIT_MS: "" }), gate);
    const wide = convergenceLimits({ TURTS_M2_CONVERGENCE_LIMIT_MS: "180000" });
    assert.deepEqual([wide.name, wide.min_ms, wide.limit_ms, wide.cross_poll], ["wide", 10000, 180000, true]);
    for (const bad of ["0", "-5", "abc", "1.5", "10s", " 10"]) {
        assert.throws(() => convergenceLimits({ TURTS_M2_CONVERGENCE_LIMIT_MS: bad }), /TURTS_M2_CONVERGENCE_LIMIT_MS must be a whole number/, bad);
    }
});

// The real profiles of convergenceLimits with short waits (the compare mode comes from the profile).
const GATE_SHORT = { ...convergenceLimits({}), min_ms: 0, stall_ms: 200, limit_ms: 2000, poll_ms: 1 };
const WIDE_SHORT = { ...convergenceLimits({ TURTS_M2_CONVERGENCE_LIMIT_MS: "2000" }), min_ms: 0, stall_ms: 200, poll_ms: 1 };

test("the gate profile compares at one poll, the wide profile also across polls", async () => {
    // Window 1 shows tick 20 (x 5) at the third read. Window 2 showed tick 20 (x 5) at the first
    // read, and then tick 30. The windows never report one tick at the same poll.
    const stream = () => fakeView([[world(10, 4), world(15, 4), world(20, 5)], [world(20, 5), world(30, 6)]]);
    await assert.rejects(waitForSharedTick(stream(), GATE_SHORT), /profile gate, compare same_poll, .* 0 shared ticks, 0 of them different/);
    const result = await waitForSharedTick(stream(), WIDE_SHORT);
    assert.equal(result.tick, 20);
    assert.equal(result.compare, "cross_poll");
});

test("the gate profile agrees at a tick that both windows report at the same poll, and it names its compare mode", async () => {
    const result = await waitForSharedTick(fakeView([[world(10, 4), world(20, 5)], [world(11, 4), world(20, 5)]]), GATE_SHORT);
    assert.equal(result.tick, 20);
    assert.equal(result.compare, "same_poll");
    assert.equal(result.profile, "gate");
});

test("in both profiles a window that shows a wrong projection and then a right one at one tick stays wrong", async () => {
    for (const limits of [GATE_SHORT, WIDE_SHORT]) {
        await assert.rejects(waitForSharedTick(fakeView([[world(10, 4), world(10, 5)], [world(10, 5)]]), limits),
            /differ between owners at a shared coherent tick .* 1 shared ticks, 1 of them different, [1-9]\d* unstable/, limits.name);
    }
});
