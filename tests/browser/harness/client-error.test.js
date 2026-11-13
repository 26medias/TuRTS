// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const { test } = require("node:test");
const { firstClientError, assertNoClientErrorBeforeClose } = require("./client-error.js");

// A fake page. evaluate runs the function with a window that has the error value.
function fakePage({ error = null, closed = false } = {}) {
    const window = { turts_error: error };
    return {
        isClosed: () => closed,
        async evaluate(fn) {
            if (closed) throw new Error("page.evaluate: Target page, context or browser has been closed");
            const previous = globalThis.window;
            globalThis.window = window;
            try { return fn(); } finally { globalThis.window = previous; }
        },
    };
}

test("the client error check skips a window that the row closed (SLOT1 closes window 3)", async () => {
    const pages = [fakePage(), fakePage(), fakePage({ closed: true })];
    assert.equal(await firstClientError(pages), null, "no error and no read of the closed page");
});

test("the client error check still reports the first error of an open window", async () => {
    const pages = [fakePage(), fakePage({ error: "wgpu lost" }), fakePage({ closed: true })];
    assert.deepEqual(await firstClientError(pages), { window: 2, error: "wgpu lost" });
});

test("a window that the row closes is read before its close", async () => {
    await assertNoClientErrorBeforeClose(fakePage(), 3);
    await assert.rejects(assertNoClientErrorBeforeClose(fakePage({ error: "device lost" }), 3),
        /browser client failed in window 3 before its close: device lost/);
});
