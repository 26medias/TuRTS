// turts-test-role: unit
"use strict";

// Run: node --test tests/browser/egpu-stats.test.js
// Each case names the build that makes it fail: the stats code with the named change.
const test = require("node:test");
const assert = require("node:assert/strict");
const { summarize, secondBins, minWindowFrames, percentile } = require("./egpu-stats.js");

// A steady 60 frames each second for `seconds` seconds, as timestamps in ms.
function steady(seconds, fps = 60) {
    const out = [];
    for (let i = 0; i < seconds * fps; i += 1) out.push((i * 1000) / fps);
    return out;
}

test("a steady 60 fps window is not starved and its p50 is 16.7 ms", () => {
    const result = summarize(steady(30), 30);
    assert.equal(result.starved, false);
    assert.equal(result.lowest_5s_fps, 60);
    assert.ok(Math.abs(result.interval_ms.p50 - 1000 / 60) < 0.01);
});

test("p50 and p95 are the intervals at floor(p * (n - 1)) of the sorted intervals", () => {
    // 18 intervals of 10 ms and 2 of 50 ms: sorted index 9 is 10 ms, index 18 is 50 ms.
    const stamps = [0];
    for (const gap of [...new Array(9).fill(10), 50, ...new Array(9).fill(10), 50]) stamps.push(stamps[stamps.length - 1] + gap);
    const result = summarize(stamps, 1);
    assert.equal(result.interval_ms.p50, 10);
    assert.equal(result.interval_ms.p95, 50);
    assert.equal(result.interval_ms.max, 50);
});

test("a 6 s stall in the middle is starved (fails on a build that never reports starved)", () => {
    const frames = steady(30).filter((ts) => ts < 10000 || ts >= 16000);
    const result = summarize(frames, 30);
    assert.equal(result.starved, true);
    assert.equal(result.longest_run_of_empty_seconds, 6);
    assert.equal(result.min_frames_in_5s, 0);
});

test("a 4 s stall is below the 5 s width: the empty run is 4 but the 5 s window still holds frames", () => {
    const frames = steady(30).filter((ts) => ts < 10000 || ts >= 14000);
    const result = summarize(frames, 30);
    assert.equal(result.longest_run_of_empty_seconds, 4);
    assert.equal(result.starved, false);
});

test("a slow window of 0.8 frames each second is starved by the 5 s average (fails if only empty runs count)", () => {
    // 4 frames in every 5 s, 1.25 s apart: one empty second in each block, average 0.8 fps.
    const frames = [];
    for (let block = 0; block < 6; block += 1) for (const offset of [0, 1250, 2500, 3750]) frames.push(block * 5000 + offset);
    const result = summarize(frames, 30);
    assert.equal(result.longest_run_of_empty_seconds, 1);
    assert.equal(result.min_frames_in_5s, 4);
    assert.equal(result.starved, true);
});

test("an empty window is starved (a test on an empty world must not pass)", () => {
    const result = summarize([], 30);
    assert.equal(result.frames, 0);
    assert.equal(result.starved, true);
    assert.equal(result.interval_ms.p50, null);
});

test("a window shorter than 5 s cannot be judged and is starved", () => {
    assert.equal(summarize(steady(3), 3).starved, true);
});

test("secondBins ignores timestamps outside the period, minWindowFrames slides by 1 s", () => {
    assert.deepEqual(secondBins([-5, 0, 999, 1000, 2500, 3000], 3), [2, 1, 1]);
    assert.equal(minWindowFrames([5, 5, 0, 5, 5, 5], 3), 10);
});

test("percentile is the sorted value at floor(p * (n - 1))", () => {
    assert.equal(percentile([1, 2, 3, 4, 5, 6, 7, 8, 9, 10], 0.95), 9);
    assert.equal(percentile([], 0.5), null);
});
