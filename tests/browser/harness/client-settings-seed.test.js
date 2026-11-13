// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const { test } = require("node:test");
const { SETTINGS_KEY, seedStoredClientSettings, clientSettingsSeed, assertStoredClientSettings } = require("./client-settings-seed.js");

function fakeStorage(initial = {}) {
    const items = new Map(Object.entries(initial));
    return { getItem: (key) => (items.has(key) ? items.get(key) : null), setItem: (key, value) => items.set(key, String(value)), items };
}

function withWindow(window, fn) {
    const previous = globalThis.window;
    globalThis.window = window;
    try { return fn(); } finally { globalThis.window = previous; }
}

test("a row without client settings stores nothing", () => {
    assert.equal(clientSettingsSeed({ status: "executable" }), null);
    assert.equal(clientSettingsSeed(undefined), null);
});

test("the seed writes the frame limit into the client settings document of a first start", () => {
    const seed = clientSettingsSeed({ client_settings: { frame_limit: 30, reason: "three clients" } });
    assert.deepEqual(seed, { key: "turts.v1.settings", controls: { frame_limit: 30 } });
    const storage = fakeStorage();
    withWindow({ localStorage: storage }, () => seedStoredClientSettings(seed));
    assert.deepEqual(JSON.parse(storage.items.get(SETTINGS_KEY)), { controls: { frame_limit: 30 } });
});

test("the seed keeps a document that the window already has (reload, copied window)", () => {
    const seed = clientSettingsSeed({ client_settings: { frame_limit: 30, reason: "three clients" } });
    const existing = JSON.stringify({ controls: { frame_limit: 30, scroll_speed: 1.5 } });
    const storage = fakeStorage({ [SETTINGS_KEY]: existing });
    withWindow({ localStorage: storage }, () => seedStoredClientSettings(seed));
    assert.equal(storage.items.get(SETTINGS_KEY), existing);
});

// A fake page with a stored document and a client debug state that reports `applied`.
function fakePage(value, applied) {
    return {
        evaluate: async (fn, key) => withWindow({
            localStorage: fakeStorage(value === null ? {} : { [key]: value }),
            turts_debug: () => JSON.stringify({ frame_limits: applied }),
        }, () => fn(key)),
    };
}

test("the stored check reads the real document and refuses another value", async () => {
    const seed = clientSettingsSeed({ client_settings: { frame_limit: 20, reason: "three clients" } });
    const applied = { focused: 20, background: 60 };
    assert.deepEqual(await assertStoredClientSettings(fakePage(JSON.stringify({ controls: { frame_limit: 20 } }), applied), seed, 1),
        { window: 1, key: SETTINGS_KEY, frame_limit: 20, applied });
    await assert.rejects(assertStoredClientSettings(fakePage(JSON.stringify({ controls: { frame_limit: 0 } }), applied), seed, 2), /window 2 stored frame_limit 0, expected 20/);
    await assert.rejects(assertStoredClientSettings(fakePage(null, applied), seed, 3), /window 3 stored frame_limit null/);
});

// A client that builds its debug state only on a request (design D 9, row DBG1): `turts_debug()`
// is empty until a request, so the check must ask for the record.
test("the stored check asks for a debug record when the page has the request bridge", async () => {
    const seed = clientSettingsSeed({ client_settings: { frame_limit: 20, reason: "three clients" } });
    const applied = { focused: 20, background: 60 };
    const stored = JSON.stringify({ controls: { frame_limit: 20 } });
    const page = { evaluate: async (fn, key) => withWindow({
        localStorage: fakeStorage({ [key]: stored }),
        turts_debug: () => "",
        turts_debug_request: async () => JSON.stringify({ frame_limits: applied }),
    }, () => fn(key)) };
    assert.deepEqual(await assertStoredClientSettings(page, seed, 1), { window: 1, key: SETTINGS_KEY, frame_limit: 20, applied });
});

test("the check goes red when the client ignores the stored frame limit", async () => {
    const seed = clientSettingsSeed({ client_settings: { frame_limit: 20, reason: "three clients" } });
    const stored = JSON.stringify({ controls: { frame_limit: 20 } });
    await assert.rejects(assertStoredClientSettings(fakePage(stored, { focused: 0, background: 60 }), seed, 1),
        /window 1 applied frame limit 0, expected the stored 20/);
    await assert.rejects(assertStoredClientSettings(fakePage(stored, null), seed, 2),
        /window 2 applied frame limit null, expected the stored 20/);
});
