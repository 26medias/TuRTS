// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const test = require("node:test");
const browserLaunchOptions = require("./t7-launch-options");

test("T7 preserves its headless default when no M2 launch environment is supplied", () => {
    assert.deepEqual(browserLaunchOptions({}), { headless: true });
});

test("T7 accepts the headed Vulkan profile and SwiftShader exclusion", () => {
    assert.deepEqual(browserLaunchOptions({
        M2_HEADLESS: "false",
        M2_CHROME_ARGS_JSON: '["--enable-gpu","--use-angle=vulkan","--enable-features=Vulkan","--enable-unsafe-webgpu"]',
        M2_IGNORE_DEFAULT_ARGS_JSON: '["--enable-unsafe-swiftshader"]',
    }), {
        headless: false,
        args: ["--enable-gpu", "--use-angle=vulkan", "--enable-features=Vulkan", "--enable-unsafe-webgpu"],
        ignoreDefaultArgs: ["--enable-unsafe-swiftshader"],
    });
});

test("T7 rejects invalid headless values and malformed launch arrays", () => {
    assert.throws(() => browserLaunchOptions({ M2_HEADLESS: "0" }), /M2_HEADLESS must be exactly/);
    assert.throws(() => browserLaunchOptions({ M2_CHROME_ARGS_JSON: "{}" }), /JSON array of strings/);
    assert.throws(() => browserLaunchOptions({ M2_IGNORE_DEFAULT_ARGS_JSON: '["ok",1]' }), /JSON array of strings/);
});
