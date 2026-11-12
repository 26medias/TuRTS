// turts-test-role: unit
"use strict";

// Row DBG1 (design D 9): the client builds the debug state only when the harness asks for it.
// These tests give the harness a page that has `window.turts_debug_request`, as the web client
// now has. Each test fails on the harness before L0-DBG: it reads the stale `turts_debug()`.
const assert = require("node:assert/strict");
const test = require("node:test");
const vm = require("node:vm");
const { createView } = require("./view.js");
const { createSceneCheckpoints } = require("./scene-checkpoints.js");
const { installBoundedRafHold } = require("./bounded-raf.js");
const { installSh1PhaseCapture } = require("./sh1-lifecycle.js");
const { recordStartup, waitForJoinedRole } = require("./run.js");

// A page whose `evaluate` runs the function in a sandbox with the given window.
function pageWith(window, extra = {}) {
    return {
        evaluate: async (fn, argument) => vm.runInNewContext(`(${fn.toString()})(argument)`, {
            window, argument, JSON, String, performance: { timeOrigin: 1000, now: () => 5 },
            document: { hasFocus: () => true, activeElement: null, pointerLockElement: null,
                getElementById: () => null, querySelector: () => null }, ...extra,
        }),
    };
}

function requestingWindow() {
    const calls = { requests: [], reads: 0 };
    const window = {
        turts_debug: () => { calls.reads += 1; return JSON.stringify({ record: "stale" }); },
        turts_debug_request: (minInterval) => {
            calls.requests.push(minInterval);
            return Promise.resolve(JSON.stringify({ record: "fresh" }));
        },
        turts_scene: () => JSON.stringify({ scene: true }),
    };
    return { window, calls };
}

test("view.bridge asks for one fresh debug record and returns it, not the stale record", async () => {
    const { window, calls } = requestingWindow();
    const view = createView([pageWith(window)], [], "/tmp");
    assert.deepEqual(await view.bridge("turts_debug"), { record: "fresh" });
    assert.deepEqual(calls.requests, [undefined], "one request, with the default cap");
    assert.equal(calls.reads, 0, "the stale record is not read");
    assert.deepEqual(await view.bridge("turts_scene"), { scene: true });
    assert.equal(calls.requests.length, 1, "another bridge asks for nothing");
});

test("view.bridge in a held frame reads the published record and does not wait for a frame", async () => {
    const { window, calls } = requestingWindow();
    window.__turts_input_held = () => true;
    const view = createView([pageWith(window)], [], "/tmp");
    assert.deepEqual(await view.bridge("turts_debug"), { record: "stale" });
    assert.deepEqual(calls.requests, [], "a held frame runs no callback, so a request would wait 10 s and reject");
});

test("view.bridge reads the record as before from a page with no request bridge", async () => {
    const { window } = requestingWindow();
    delete window.turts_debug_request;
    const view = createView([pageWith(window)], [], "/tmp");
    assert.deepEqual(await view.bridge("turts_debug"), { record: "stale" });
});

test("a scene checkpoint asks for the debug record and reads it with the scene of the same call", async () => {
    const { window, calls } = requestingWindow();
    const checkpoints = createSceneCheckpoints([pageWith(window)]);
    await checkpoints.record({ window: 1, input_call: 0, wall_time_ms: 1 });
    const [row] = checkpoints.snapshots();
    assert.deepEqual(row.debug, { record: "fresh" });
    assert.deepEqual(row.scene, { scene: true });
    assert.deepEqual(calls.requests, [undefined]);
});

test("a scene checkpoint in a held frame reads the published record and does not wait for a frame", async () => {
    const { window, calls } = requestingWindow();
    window.__turts_input_held = () => true;
    const checkpoints = createSceneCheckpoints([pageWith(window)]);
    await checkpoints.record({ window: 1, input_call: 0, wall_time_ms: 1 });
    assert.deepEqual(checkpoints.snapshots()[0].debug, { record: "stale" });
    assert.deepEqual(calls.requests, [], "a held frame runs no callback, so a request would wait");
});

test("the bounded frame hold reports whether it is active", () => {
    const frames = [];
    const sandbox = { requestAnimationFrame: (callback) => frames.push(callback), cancelAnimationFrame() {} };
    sandbox.window = sandbox;
    vm.runInNewContext(`(${installBoundedRafHold.toString()})()`, sandbox);
    assert.equal(sandbox.__turts_input_held(), false);
    sandbox.__turts_begin_bounded_input();
    assert.equal(sandbox.__turts_input_held(), true);
    sandbox.__turts_end_bounded_input();
    assert.equal(sandbox.__turts_input_held(), false);
});

test("the load-screen sampler asks for a record at each sample", async () => {
    const phases = ["FetchPackages", "VerifyPackage", "Joining", "Joined { slot: 1 }"];
    let index = 0;
    const requests = [];
    const window = {
        turts_debug: () => JSON.stringify({ phase: "stale" }),
        turts_debug_request: () => {
            requests.push(index);
            return Promise.resolve(JSON.stringify({ phase: phases[Math.min(index, phases.length - 1)], ui_snapshot: [] }));
        },
    };
    const page = { ...pageWith(window), waitForTimeout: async () => { index += 1; } };
    const observations = [];
    await recordStartup(page, 1, observations, async () => { while (index < 3) await new Promise((resolve) => setTimeout(resolve, 1)); });
    assert.ok(requests.length >= 2, "the sampler asked for fresh records");
    assert.ok(observations.some((item) => item.phase === "VerifyPackage"), "the phases come from the requested records");
    assert.ok(observations.every((item) => item.phase !== "stale"));
});

test("the SH1 phase capture asks for the record of the next frame, with no wait, in each frame", () => {
    const callbacks = [];
    const requests = [];
    const phases = [];
    const window = {
        requestAnimationFrame: (callback) => callbacks.push(callback),
        turts_debug: () => JSON.stringify({ phase: phases[phases.length - 1] }),
        turts_debug_request: (minInterval) => { requests.push(minInterval); return Promise.resolve("{}"); },
        __turts_begin_bounded_input: () => {},
    };
    window.window = window;
    vm.runInNewContext(`(${installSh1PhaseCapture.toString()})()`, { window, Date, performance: { now: () => 1, timeOrigin: 0 }, document: { URL: "u" }, Set, JSON });
    window.requestAnimationFrame(() => {});
    // Before it is enabled, the capture asks for nothing.
    callbacks.shift()(0);
    assert.deepEqual(requests, []);
    window.__sh1_phase_capture.enabled = true;
    for (const phase of ["FetchPackages", "VerifyPackage"]) {
        phases.push(phase);
        window.requestAnimationFrame(() => {});
        callbacks.shift()(0);
    }
    assert.deepEqual(requests, [0, 0], "one request with no wait for each enabled frame");
    assert.equal(window.__sh1_phase_capture.held, "verify", "the frame of the phase is held");
});

// A page whose record is empty until a request has been served, and whose request resolves one
// turn later. The fake `waitForFunction` is Playwright's: it polls a predicate and, as Playwright
// does, tests the result as it is, so a Promise result passes at once.
test("the join wait polls until a requested record shows Joined (a Promise result would pass at once)", async () => {
    let record = "";
    let requests = 0;
    const window = {
        turts_scene: () => JSON.stringify({ cameras_3d: 1 }),
        turts_debug: () => record,
        turts_debug_request: () => {
            requests += 1;
            return new Promise((resolve) => setImmediate(() => {
                record = JSON.stringify({ phase: requests < 3 ? "Connecting" : "Joined { slot: 0 }", last_frame: { frame_index: 1 } });
                resolve(record);
            }));
        },
    };
    let polls = 0;
    const page = { waitForFunction: async (fn, argument) => {
        for (;;) {
            polls += 1;
            // A string is an expression and is not called (Playwright 1.64.1); a function is called.
            const result = typeof fn === "string" ? vm.runInNewContext(fn, { window, JSON })
                : vm.runInNewContext(`(${fn.toString()})(argument)`, { window, argument, JSON });
            if (result) return;
            await new Promise((resolve) => setImmediate(resolve));
        }
    } };
    await waitForJoinedRole(page);
    assert.ok(polls >= 3, `the wait polled ${polls} times; it must wait for the Joined record`);
    assert.match(record, /Joined/);
});
