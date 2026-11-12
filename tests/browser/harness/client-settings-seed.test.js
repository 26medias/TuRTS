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

test("the stored check reads the real document and refuses another value", async () => {
    const seed = clientSettingsSeed({ client_settings: { frame_limit: 30, reason: "three clients" } });
    const page = (value) => ({ evaluate: async (fn, key) => withWindow({ localStorage: fakeStorage(value === null ? {} : { [key]: value }) }, () => fn(key)) });
    assert.deepEqual(await assertStoredClientSettings(page(JSON.stringify({ controls: { frame_limit: 30 } })), seed, 1),
        { window: 1, key: SETTINGS_KEY, frame_limit: 30 });
    await assert.rejects(assertStoredClientSettings(page(JSON.stringify({ controls: { frame_limit: 0 } })), seed, 2), /window 2 stored frame_limit 0, expected 30/);
    await assert.rejects(assertStoredClientSettings(page(null), seed, 3), /window 3 stored frame_limit null/);
});
