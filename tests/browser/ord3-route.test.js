// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { test } = require("node:test");
const { __test } = require("./orders-oracle.js");

const root = path.resolve(__dirname, "..", "..");
const read = (file) => fs.readFileSync(path.join(root, file), "utf8");

// RUN130, evidence hud2-run130-m-ord3 run 1: the first Append click went out at wall 1794469405846
// and the route-queued checkpoint was read at page time 1794469408301.9. That is 2.46 s on a host
// with load average 13. The unit must still be on its way to A at twice that time.
const LOADED_ROUTE_QUEUE_SECONDS = 2.46;
const MARGIN = 2;

function row() { return JSON.parse(read("tests/browser/rows/ORD3.json")); }

test("ORD3 point A is far enough from every own trooper that a loaded host reads the queue before A", () => {
    const stages = row().stages;
    const speed = Number(/speed:\s*([0-9.]+)/.exec(read("tests/browser/fixtures/order-rows/muster/units/trooper.ron"))[1]);
    const hz = Number(/tick_hz:\s*(\d+)/.exec(read("scripts/mc-browser.sh"))[1]);
    assert(speed > 0 && hz > 0, "the trooper speed and the tick rate are read from the sources");
    const field = read("tests/browser/fixtures/order-rows/muster/maps/field.ron");
    const starts = [...field.matchAll(/unit: "muster:trooper", owner_slot: 0, pos: \(x: ([0-9.]+), y: ([0-9.]+)\)/g)]
        .map((match) => [Number(match[1]), Number(match[2])]);
    assert.equal(starts.length, 40, "the map has its 40 own troopers");
    const a = stages.route[0];
    const nearest = Math.min(...starts.map((start) => Math.hypot(start[0] - a[0], start[1] - a[1])));
    const seconds = nearest / speed;
    assert(seconds >= LOADED_ROUTE_QUEUE_SECONDS * MARGIN,
        `the nearest own trooper needs ${seconds.toFixed(2)} s to reach A; at least ${LOADED_ROUTE_QUEUE_SECONDS * MARGIN} s are needed (tick rate ${hz})`);
});

test("the route-queued checkpoint is late when its tick is at or after the tick of the landing at A", () => {
    const rows = [];
    for (let tick = 90; tick < 140; tick += 1) rows.push({ tick, unit: 40, x: tick < 126 ? 33 : 36, y: tick < 126 ? 23 : 26 });
    const a = [36, 26];
    assert.equal(__test.readBeforeLanding(rows, a, 120), true, "tick 120 is before the landing at tick 126");
    assert.equal(__test.readBeforeLanding(rows, a, 125), true);
    assert.equal(__test.readBeforeLanding(rows, a, 126), false, "tick 126 is the landing tick");
    assert.equal(__test.readBeforeLanding(rows, a, 135), false, "tick 135 is the failing RUN130 checkpoint");
    assert.equal(__test.readBeforeLanding(rows.slice(0, 20), a, 135), true, "a unit that never lands is not late");
});
