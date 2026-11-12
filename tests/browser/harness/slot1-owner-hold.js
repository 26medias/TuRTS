// turts-test-role: harness
"use strict";

// SLOT1 starts three windows one after the other. While windows 1 and 2 (the owners) draw frames,
// the GPU request of window 3 can stay pending for 30 s on this host (hud2-fix4-slot1-idle: owners
// fulfilled, window 3 pending, 2 runs of 2). Chrome documentation and ruling C319 say that a pause of
// the owner frames lets the pending request settle. The harness holds the owner animation frames
// (the bounded-input hook of bounded-raf.js) from before window 3 opens until its role is known, then
// releases them. The owners keep their sockets, their slots and their page state; only their frames wait.
async function beginOwnerHold(pages) {
    const started = Date.now();
    const begun = await Promise.all(pages.map((page) => page.evaluate(() => {
        if (typeof window.__turts_begin_bounded_input !== "function") throw new Error("bounded input frame hook is unavailable");
        window.__turts_begin_bounded_input();
        return true;
    })));
    return { started_ms: started, windows: begun.length };
}

async function endOwnerHold(pages, hold) {
    const released = await Promise.all(pages.map((page) => page.evaluate(() => window.__turts_end_bounded_input())));
    return { ...hold, ended_ms: Date.now(), held_ms: Date.now() - hold.started_ms, released };
}

// The first client error of an open window. A window that the row closed on purpose has no client to read.
async function firstClientError(pages) {
    for (const [index, page] of pages.entries()) {
        if (typeof page.isClosed === "function" && page.isClosed()) continue;
        const error = await page.evaluate(() => window.turts_error || null);
        if (error) return { window: index + 1, error };
    }
    return null;
}

module.exports = { beginOwnerHold, endOwnerHold, firstClientError };
