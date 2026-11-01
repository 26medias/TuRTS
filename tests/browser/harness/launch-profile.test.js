// turts-test-role: test
"use strict";

const assert = require("node:assert/strict");
const test = require("node:test");
const { launchArgsForDpr } = require("./run.js");

const base = ["--enable-gpu", "--use-angle=vulkan"];

test("launch profile matches DPR 1 with a physical-pixel scale flag", () => {
    assert.deepEqual(launchArgsForDpr(base, 1), [...base, "--force-device-scale-factor=1"]);
});

test("launch profile matches DPR 2 with a physical-pixel scale flag", () => {
    assert.deepEqual(launchArgsForDpr(base, 2), [...base, "--force-device-scale-factor=2"]);
});

test("launch profile preserves an explicit matching scale flag", () => {
    const args = [...base, "--force-device-scale-factor=2"];
    assert.deepEqual(launchArgsForDpr(args, 2), args);
});

test("launch profile rejects a scale flag that conflicts with the context DPR", () => {
    assert.throws(() => launchArgsForDpr([...base, "--force-device-scale-factor=1"], 2),
        /conflicts with effective DPR 2/);
});

test("launch profile rejects duplicate scale flags", () => {
    assert.throws(() => launchArgsForDpr([...base, "--force-device-scale-factor=1", "--force-device-scale-factor=2"], 2),
        /conflicting explicit/);
});
