// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const { test } = require("node:test");
const { createDriverIo } = require("./driver-io.js");
const { createSceneCheckpoints } = require("./scene-checkpoints.js");

// A page whose bridges exist `readyAfterMs` after the reload (the wasm client starts late), as in
// RUN142 AUD1: the first screenshot after the reload had no `turts_scene`.
function lateBridgePage(readyAfterMs) {
    const state = { ready: false, epoch: 1, reloads: 0, waits: [] };
    const page = {
        state,
        evaluate: async (fn) => {
            const source = String(fn);
            if (source.includes("turts_scene")) {
                return state.ready ? { page_time_ms: 1, client_error: null, scene: "{}", debug: "{}", browser: "{}" }
                    : { page_time_ms: 1, client_error: null, scene: null, debug: null, browser: "{}" };
            }
            if (source.includes("__turts_io_actual_events") && source.includes("__turts_io_downstream_events")) return { actual: 0, downstream: 0 };
            return undefined;
        },
        screenshot: async () => Buffer.from([1, 2, 3]),
        waitForTimeout: async (ms) => new Promise((resolve) => setTimeout(resolve, ms)),
        reload: async () => {
            state.reloads += 1;
            state.epoch += 1;
            state.ready = false;
            setTimeout(() => { state.ready = true; }, readyAfterMs).unref();
        },
        // Playwright waitForFunction: polls the predicate; here the predicate is the page-side function.
        waitForFunction: async (fn, _arg, options) => {
            state.waits.push(options);
            const deadline = Date.now() + options.timeout;
            while (Date.now() < deadline) {
                if (state.ready) return;
                await new Promise((resolve) => setTimeout(resolve, 5));
            }
            throw new Error("timeout");
        },
        mouse: {}, keyboard: {}, bringToFront: async () => {},
    };
    return page;
}

function rig(page, options = {}) {
    const documentEpochs = [1];
    const original = page.reload;
    page.reload = async () => { await original(); documentEpochs[0] = page.state.epoch; };
    const checkpoints = createSceneCheckpoints([page], 16, documentEpochs);
    const { io, inputLog } = createDriverIo([page], { documentEpochs, afterScreenshot: checkpoints.record, ...options });
    return { io, inputLog, checkpoints };
}

test("reload waits for the page bridges before the first checkpointed screenshot (RUN142 AUD1)", async () => {
    const page = lateBridgePage(60);
    const { io, inputLog, checkpoints } = rig(page);
    await io.reload();
    assert.equal(page.state.ready, true, "reload returns only when the bridges exist");
    assert.equal(inputLog.find((entry) => entry.call === "reload").bridges_ready_after_reload, true);
    await io.screenshot();
    assert.equal(checkpoints.snapshots().length, 1, "the first screenshot after the reload has its checkpoint");
});

test("reload does not hide a page that never gets its bridges: bounded, no throw, and the screenshot stays red", async () => {
    const page = lateBridgePage(1e9);
    const { io, inputLog } = rig(page);
    // Short bound for the test: the harness bound is read from the call options of the fake page.
    page.waitForFunction = async (_fn, _arg, options) => { page.state.waits.push(options); throw new Error("timeout"); };
    await io.reload();
    assert.equal(inputLog.find((entry) => entry.call === "reload").bridges_ready_after_reload, false);
    await assert.rejects(io.screenshot(), /missing scene checkpoint bridge/);
    assert.equal(page.state.waits[0].timeout, 20000, "the wait is bounded at 20 s");
});

test("reload with no scene checkpoints, or with the load observation on, does not wait for bridges", async () => {
    const plain = lateBridgePage(1);
    plain.waitForFunction = async () => { throw new Error("must not be called"); };
    const documentEpochs = [1];
    const original = plain.reload;
    plain.reload = async () => { await original(); documentEpochs[0] = plain.state.epoch; };
    const { io } = createDriverIo([plain], { documentEpochs });
    await io.reload();
    const observed = lateBridgePage(1);
    const observedEpochs = [1];
    const originalObserved = observed.reload;
    observed.reload = async () => { await originalObserved(); observedEpochs[0] = observed.state.epoch; };
    observed.waitForFunction = async () => { throw new Error("must not be called"); };
    const checkpoints = createSceneCheckpoints([observed], 4, observedEpochs);
    const withLoad = createDriverIo([observed], { documentEpochs: observedEpochs, afterScreenshot: checkpoints.record, observeReloadLoad: true });
    observed.evaluate = async () => ({ phase: "Joined", joined: true, rows: [] });
    await withLoad.io.reload();
});
