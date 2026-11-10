// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const test = require("node:test");
const vm = require("node:vm");
const { installBoundedRafHold } = require("./bounded-raf.js");

function fakeWindow() {
    const callbacks = new Map();
    let nextId = 0;
    const sandbox = {
        requestAnimationFrame(callback) {
            const id = ++nextId;
            callbacks.set(id, callback);
            return id;
        },
        cancelAnimationFrame(id) { callbacks.delete(id); },
    };
    sandbox.window = sandbox;
    vm.runInNewContext(`(${installBoundedRafHold.toString()})()`, sandbox, { filename: "bounded-raf.js" });
    return {
        window: sandbox,
        runNext(timestamp) {
            const next = callbacks.entries().next().value;
            if (!next) return false;
            const [id, callback] = next;
            callbacks.delete(id);
            callback(timestamp);
            return true;
        },
    };
}

test("bounded hold queues real RAF callbacks and releases them through the native scheduler", () => {
    const page = fakeWindow();
    let frames = 0;
    const appFrame = () => {
        frames += 1;
        page.window.requestAnimationFrame(appFrame);
    };
    page.window.requestAnimationFrame(appFrame);
    assert.equal(page.runNext(10), true);
    assert.equal(frames, 1);

    page.window.__turts_begin_bounded_input();
    assert.equal(page.runNext(20), true);
    assert.equal(frames, 1, "a pending app callback does not run during the bounded hold");
    const result = JSON.parse(JSON.stringify(page.window.__turts_end_bounded_input()));
    assert.deepEqual(result, { held_callbacks: 1, released: true, release_batch: 1 });
    assert.equal(page.runNext(30), true);
    assert.equal(frames, 2, "the callback resumes on the fake native RAF scheduler");
    assert.equal(page.runNext(40), true);
    assert.equal(frames, 3, "the app's recurring RAF loop continues normally");
});

test("canceled callbacks held during input are not replayed after release", () => {
    const page = fakeWindow();
    let frames = 0;
    const id = page.window.requestAnimationFrame(() => { frames += 1; });
    page.window.__turts_begin_bounded_input();
    assert.equal(page.runNext(10), true);
    page.window.cancelAnimationFrame(id);
    const result = JSON.parse(JSON.stringify(page.window.__turts_end_bounded_input()));
    assert.equal(result.held_callbacks, 1);
    assert.equal(result.released, true);
    assert.equal(page.runNext(20), false);
    assert.equal(frames, 0);
});

test("the bounded RAF hold rejects nesting and reports an unmatched release", () => {
    const page = fakeWindow();
    assert.deepEqual(JSON.parse(JSON.stringify(page.window.__turts_end_bounded_input())), { held_callbacks: 0, released: false });
    page.window.__turts_begin_bounded_input();
    assert.throws(() => page.window.__turts_begin_bounded_input(), /already active/);
    assert.equal(page.window.__turts_end_bounded_input().released, true);
});
