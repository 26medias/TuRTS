// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const { test } = require("node:test");
const { beginOwnerHold, endOwnerHold, firstClientError } = require("./slot1-owner-hold.js");

// A fake page. evaluate runs the function with a window that has the hooks and the error value.
function fakePage(name, { error = null, closed = false, log = [] } = {}) {
    const window = {
        __turts_begin_bounded_input() { log.push(`${name}:begin`); },
        __turts_end_bounded_input() { log.push(`${name}:end`); return { held_callbacks: 1, released: true, release_batch: 1 }; },
        turts_error: error,
    };
    return {
        name, log,
        isClosed: () => closed,
        async evaluate(fn) {
            if (closed) throw new Error("page.evaluate: Target page, context or browser has been closed");
            const previous = globalThis.window;
            globalThis.window = window;
            try { return fn(); } finally { globalThis.window = previous; }
        },
    };
}

test("the owner hold begins on both owner pages, then ends on both", async () => {
    const log = [];
    const pages = [fakePage("A", { log }), fakePage("B", { log })];
    const hold = await beginOwnerHold(pages);
    assert.deepEqual(log.slice().sort(), ["A:begin", "B:begin"]);
    const record = await endOwnerHold(pages, hold);
    assert.deepEqual(log.slice().sort(), ["A:begin", "A:end", "B:begin", "B:end"]);
    assert.equal(record.windows, 2);
    assert.deepEqual(record.released.map((entry) => entry.released), [true, true]);
    assert.ok(record.held_ms >= 0);
});

test("the owner hold fails loudly when a page has no frame hook", async () => {
    const broken = { isClosed: () => false, evaluate: async (fn) => { globalThis.window = {}; try { return fn(); } finally { delete globalThis.window; } } };
    await assert.rejects(beginOwnerHold([broken]), /bounded input frame hook is unavailable/);
});

test("the client error check skips a window that the row closed (SLOT1 closes window 3)", async () => {
    const pages = [fakePage("A"), fakePage("B"), fakePage("C", { closed: true })];
    assert.equal(await firstClientError(pages), null, "no error and no read of the closed page");
});

test("the client error check still reports the first error of an open window", async () => {
    const pages = [fakePage("A"), fakePage("B", { error: "wgpu lost" }), fakePage("C", { closed: true })];
    assert.deepEqual(await firstClientError(pages), { window: 2, error: "wgpu lost" });
});
