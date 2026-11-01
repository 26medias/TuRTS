// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const vm = require("node:vm");

const htmlPath = path.join(__dirname, "../../../crates/turts_client_web/index.html");
const html = fs.readFileSync(htmlPath, "utf8");
const scripts = [...html.matchAll(/<script\b[^>]*>([\s\S]*?)<\/script>/g)]
    .map((match) => match[1]).filter((source) => source.includes("__turts_seq_queue"));
assert.equal(scripts.length, 1, "the production page must install one input-sequence capture script");

function capture(type, target, seq) {
    const listeners = new Map();
    const canvas = { id: "turts", tagName: "CANVAS" };
    const outside = { id: "outside", tagName: "INPUT" };
    const document = {
        getElementById: (id) => id === "turts" ? canvas : null,
        addEventListener(kind, listener, capturePhase) {
            assert.equal(capturePhase, true, "input sequence capture must precede canvas handlers");
            listeners.set(kind, listener);
        },
    };
    const window = { __turts_io_seq: seq };
    vm.runInNewContext(scripts[0], { window, document }, { filename: htmlPath });
    assert.equal(typeof listeners.get(type), "function", `${type} must have a capture listener`);
    listeners.get(type)({ type, target: target === "canvas" ? canvas : outside });
    return JSON.parse(JSON.stringify(window.__turts_seq_queue));
}

for (const type of ["pointerdown", "pointerup", "pointermove", "keydown", "keyup", "wheel"]) {
    test(`canvas ${type} retains its harness sequence and event type`, () => {
        assert.deepEqual(capture(type, "canvas", 17), [{ seq: 17, type }]);
    });
    test(`outside ${type} is not claimed as canvas input`, () => {
        assert.deepEqual(capture(type, "outside", 18), []);
    });
}

for (const seq of [null, undefined, NaN, Infinity, 1.25, -1]) {
    test(`invalid input sequence ${String(seq)} is ignored`, () => {
        assert.deepEqual(capture("pointerup", "canvas", seq), []);
    });
}
