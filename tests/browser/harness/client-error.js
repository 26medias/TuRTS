// turts-test-role: harness
"use strict";

// The client error (`window.turts_error`) of each window. The harness reads it at the end of a
// run, and before it closes a window on purpose (SLOT1 closes window 3 twice), so an error in a
// closed window is not lost.

// The first client error of an open window. A window that the row closed on purpose was read
// before its close (assertNoClientErrorBeforeClose).
async function firstClientError(pages) {
    for (const [index, page] of pages.entries()) {
        if (typeof page.isClosed === "function" && page.isClosed()) continue;
        const error = await page.evaluate(() => window.turts_error || null);
        if (error) return { window: index + 1, error };
    }
    return null;
}

// Read the client error of one window before the row closes it. Throws on an error.
async function assertNoClientErrorBeforeClose(page, windowNumber) {
    const error = await page.evaluate(() => window.turts_error || null);
    if (error) throw new Error(`browser client failed in window ${windowNumber} before its close: ${error}`);
}

module.exports = { firstClientError, assertNoClientErrorBeforeClose };
