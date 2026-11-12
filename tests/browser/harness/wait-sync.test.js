// turts-test-role: unit
"use strict";

// The one helper of every Playwright wait (harness/wait-sync.js). The fake page below does what
// Playwright 1.64.1 does with `waitForFunction`: it evaluates a function or an expression that gives
// a function, calls it with the argument at each poll, and stops at the first TRUE result. A Promise
// is true, so a Promise result ends the wait at once. A throw in the predicate rejects the wait.
const assert = require("node:assert/strict");
const test = require("node:test");
const vm = require("node:vm");
const { waitForSync, waitForDebugRecord } = require("./wait-sync.js");

function fakePage(window, { maxPolls = 1000 } = {}) {
    const page = { calls: [], polls: 0 };
    page.waitForFunction = async (pageFunction, argument, options) => {
        page.calls.push({ pageFunction, argument, options });
        const isFunction = typeof pageFunction === "function";
        const source = isFunction ? `(${pageFunction.toString()})` : pageFunction;
        for (let poll = 0; poll < maxPolls; poll += 1) {
            page.polls += 1;
            let value = vm.runInNewContext(source, { window, JSON, Promise });
            // A function is called with the argument. A string is an expression and is NOT called
            // (a string that gives a function is true at once): Playwright 1.64.1, checked in Chrome.
            if (isFunction) value = value(argument);
            if (value) return value;
            await new Promise((resolve) => setImmediate(resolve));
        }
        throw new Error("fake page: timeout");
    };
    return page;
}

test("a sync predicate is polled until it is true, with its argument and options", async () => {
    const window = { count: 0 };
    const page = fakePage(window);
    const result = await waitForSync(page, (limit) => ++window.count >= limit, 3, { timeout: 30000, polling: 20 });
    assert.equal(result, true);
    assert.equal(page.polls, 3);
    assert.equal(page.calls.length, 1);
    assert.deepEqual(page.calls[0].options, { timeout: 30000, polling: 20 });
    assert.equal(page.calls[0].argument, undefined, "the argument is written into the expression");
    assert.match(page.calls[0].pageFunction, /\)\(3\)$/);
});

test("a method of an object works", async () => {
    const window = { ready: true };
    const preds = { joined() { return window.ready; } };
    assert.equal(await waitForSync(fakePage(window), preds.joined), true);
});

// Layer 1, before anything is sent: an async function of any form.
test("an async predicate is refused before it reaches the page: arrow, method, factory", async () => {
    const preds = { async joined() { return true; } };
    const make = () => async () => true;
    async function named() { return true; }
    for (const [name, predicate] of Object.entries({ arrow: async () => true, method: preds.joined, factory: make(), named })) {
        const page = fakePage({});
        await assert.rejects(waitForSync(page, predicate), /async function/, name);
        assert.equal(page.calls.length, 0, `${name}: nothing was sent`);
    }
});

// Layer 2, at run time in the page: a function that is not async but returns a Promise.
test("a predicate that returns a Promise or a thenable throws in the page, in the first poll", async () => {
    for (const [name, predicate] of Object.entries({
        promise: () => Promise.resolve(true),
        factory: (() => { const make = () => () => Promise.resolve(true); return make(); })(),
        thenable: () => ({ then() {} }),
    })) {
        const page = fakePage({});
        await assert.rejects(waitForSync(page, predicate), /returned a Promise/, name);
        assert.equal(page.polls, 1, name);
    }
});

test("a value that is not a function is refused", async () => {
    await assert.rejects(waitForSync(fakePage({}), "window.x"), TypeError);
    await assert.rejects(waitForSync(fakePage({}), undefined), TypeError);
});

// L3: one request at a time, whatever the polling rate.
function requestingWindow() {
    const calls = { requests: 0 };
    const window = { turts_debug: () => (calls.settle ? "{\"phase\":\"Joined\"}" : ""), calls };
    window.turts_debug_request = () => {
        calls.requests += 1;
        return new Promise((resolve, reject) => { calls.resolve = resolve; calls.reject = reject; });
    };
    return window;
}

test("a held frame: 500 polls start one request, not 500 waiters", async () => {
    const window = requestingWindow();
    const page = fakePage(window, { maxPolls: 500 });
    await assert.rejects(waitForDebugRecord(page, () => window.turts_debug() !== "", null, { polling: 20 }), /timeout/);
    assert.equal(page.polls, 500);
    assert.equal(window.calls.requests, 1, "the frame is held, so the first request never resolves");
});

test("a new request starts after the last one resolved or was rejected", async () => {
    const window = requestingWindow();
    const page = fakePage(window, { maxPolls: 5 });
    const check = () => false;
    await assert.rejects(waitForDebugRecord(page, check), /timeout/);
    assert.equal(window.calls.requests, 1);
    window.calls.resolve("{}");
    await new Promise((resolve) => setImmediate(resolve));
    await assert.rejects(waitForDebugRecord(page, check), /timeout/);
    assert.equal(window.calls.requests, 2, "after the first resolved, one more request starts");
    window.calls.reject(new Error("no frame served the request"));
    await new Promise((resolve) => setImmediate(resolve));
    await assert.rejects(waitForDebugRecord(page, check), /timeout/);
    assert.equal(window.calls.requests, 3, "after a rejection, one more request starts, and the rejection is caught");
});

test("the check reads the published record, and an older page with no request bridge still works", async () => {
    const window = requestingWindow();
    window.calls.settle = true;
    const joined = (pattern) => /^Joined/.test(JSON.parse(window.turts_debug()).phase) && pattern === "x";
    assert.equal(await waitForDebugRecord(fakePage(window), joined, "x"), true);
    const old = { turts_debug: () => "{\"phase\":\"Joined\"}" };
    assert.equal(await waitForDebugRecord(fakePage(old), () => /^Joined/.test(JSON.parse(window.turts_debug()).phase)), true);
});

test("the check of a debug wait must be sync as well", async () => {
    await assert.rejects(waitForDebugRecord(fakePage(requestingWindow()), async () => true), /async function/);
    await assert.rejects(waitForDebugRecord(fakePage(requestingWindow()), () => Promise.resolve(true)), /returned a Promise/);
});

// ---- Playwright itself, in a real Chrome (skipped when there is no Chrome binary) ----------------
const { spawnSync } = require("node:child_process");
const CHROME = process.env.CHROME_BIN || ["google-chrome", "chromium"].map((name) => spawnSync("which", [name], { encoding: "utf8" }).stdout.trim()).find(Boolean);

test("in a real Chrome: a sync wait polls, a Promise predicate throws, a held request is asked once", { skip: !CHROME && "no Chrome binary" }, async () => {
    const { chromium } = require("playwright-core");
    const browser = await chromium.launch({ executablePath: CHROME, headless: true });
    try {
        const page = await browser.newPage();
        await page.setContent("<title>wait</title>");
        await page.evaluate(() => {
            window.polls = 0;
            window.asked = 0;
            window.turts_debug = () => "";
            window.turts_debug_request = () => { window.asked += 1; return new Promise(() => {}); };
        });
        await waitForSync(page, (limit) => { window.polls += 1; return window.polls >= limit; }, 4, { timeout: 5000, polling: 20 });
        assert.equal(await page.evaluate(() => window.polls), 4, "the wait polled until its predicate was true");
        await assert.rejects(waitForSync(page, () => Promise.resolve(true), null, { timeout: 5000 }), /returned a Promise/);
        await assert.rejects(waitForSync(page, async () => true, null, { timeout: 5000 }), /async function/);
        await assert.rejects(waitForDebugRecord(page, () => window.turts_debug() !== "", null, { timeout: 600, polling: 20 }), /Timeout/);
        assert.equal(await page.evaluate(() => window.asked), 1, "about 30 polls with the frame held ask once");
    } finally { await browser.close(); }
});
