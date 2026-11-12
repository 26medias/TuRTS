// turts-test-role: harness
"use strict";

// The one place that calls Playwright's `waitForFunction` (tests/browser/harness/wait-sync-scan.test.js
// fails on any other file that names it).
//
// Playwright does not await the predicate: a Promise result is true, so an `async` predicate ends
// the wait at once (playwright-core 1.64.1; review-l0-dbg.md, finding 1). This helper stops it in
// two layers. Before anything is sent, it refuses an `async` function of any form (arrow, method,
// factory result). In the page, it throws when the predicate returns a value with `.then` (a
// function that is not `async` but returns a Promise), and the wait rejects in its first poll.
const vm = require("node:vm");

function assertSyncFunction(predicate) {
    if (typeof predicate !== "function") throw new TypeError("waitForSync: the predicate must be a function");
    // The tag, not `instanceof`: a function of another realm (a vm context) is async too.
    if (Object.prototype.toString.call(predicate) === "[object AsyncFunction]") throw new TypeError("waitForSync: the predicate is an async function; Playwright does not await it");
}

// The source of a function as one JavaScript expression: an arrow, a `function`, or a method of an
// object (`name() {}`), which is not an expression until it is put in an object.
function functionExpression(predicate) {
    const source = predicate.toString();
    for (const candidate of [`(${source})`, `({${source}})[${JSON.stringify(predicate.name)}]`]) {
        try { new vm.Script(candidate); return candidate; } catch { /* the next form */ }
    }
    throw new TypeError("waitForSync: the predicate has a form that cannot be sent to the page");
}

// `functionSource` is the JavaScript text of a function of one argument. Playwright is given ONE
// EXPRESSION that calls it: a string that only gives a function is true at once and is never called
// (checked with Playwright 1.64.1 in Chrome), and a function that Node passes cannot carry a
// predicate that this helper builds. So the argument is written into the expression (it must be JSON
// data), and the page evaluates the expression at each poll.
function syncWait(page, functionSource, argument, options) {
    const literal = argument === undefined ? "undefined" : JSON.stringify(argument);
    const expression = `((argument) => {
        const result = (${functionSource})(argument);
        if (result && typeof result.then === "function") {
            throw new Error("waitForSync: the predicate returned a Promise; Playwright does not await it");
        }
        return result;
    })(${literal})`;
    return page.waitForFunction(expression, undefined, options);
}

// Wait for a sync predicate: `waitForFunction(predicate, argument, options)` of Playwright.
async function waitForSync(page, predicate, argument, options) {
    assertSyncFunction(predicate);
    return syncWait(page, functionExpression(predicate), argument, options);
}

// Wait for a sync check of the debug record that the client publishes on request (design D 9, row
// DBG1). Each poll starts a request only when none is pending: with a held frame (the SH1 phase
// capture) a poll every 20 ms would otherwise queue one waiter and one 10 s timer at each poll.
// The check reads `window.turts_debug()` and `window.turts_scene()`. A page with no request bridge
// is an older build that publishes in each frame, so the check alone is right for it.
async function waitForDebugRecord(page, check, argument, options) {
    assertSyncFunction(check);
    const functionSource = `(argument) => {
        const asked = window;
        if (typeof asked.turts_debug_request === "function" && !asked.__turts_wait_request_pending) {
            asked.__turts_wait_request_pending = true;
            asked.turts_debug_request().catch(() => {}).then(() => { asked.__turts_wait_request_pending = false; });
        }
        return (${functionExpression(check)})(argument);
    }`;
    return syncWait(page, functionSource, argument, options);
}

module.exports = { waitForSync, waitForDebugRecord };
