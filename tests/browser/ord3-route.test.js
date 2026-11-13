// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { test } = require("node:test");
const { deflateSync } = require("node:zlib");
const { __test } = require("./orders-oracle.js");
const { drive } = require("./orders-driver.js");

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

// RUN130, evidence hud2-fix4-ord3-a run 2: the queue-start click went out at +0 s and the
// queue-overflow-final checkpoint was read at +17.77 s; the 17 waits of 150 ms and their records
// took 0.395 s each, so the driver without them needs 17.77 - 17 * 0.395 = 11.1 s. The active
// point must stay unreached at 1.5 times that time.
const LOADED_OVERFLOW_SECONDS = 11.1;
const OVERFLOW_MARGIN = 1.5;

test("ORD3 queue-overflow start point is far enough that the unit cannot reach it during the 17 appends", () => {
    const { stages } = row();
    const speed = Number(/speed:\s*([0-9.]+)/.exec(read("tests/browser/fixtures/order-rows/muster/units/trooper.ron"))[1]);
    const seconds = Math.hypot(stages.queue[0][0] - stages.hold_point[0], stages.queue[0][1] - stages.hold_point[1]) / speed;
    assert(seconds >= LOADED_OVERFLOW_SECONDS * OVERFLOW_MARGIN,
        `the held trooper needs ${seconds.toFixed(2)} s to reach the queue start; at least ${LOADED_OVERFLOW_SECONDS * OVERFLOW_MARGIN} s are needed`);
    for (const point of stages.queue) assert(point[0] >= 0.5 && point[1] >= 0.5 && point[0] <= 63.5 && point[1] <= 47.5, "the queue points are inside the 64 by 48 map");
});

function crc32(bytes) {
    let crc = 0xffffffff;
    for (const byte of bytes) {
        crc ^= byte;
        for (let bit = 0; bit < 8; bit += 1) crc = (crc >>> 1) ^ (0xedb88320 & -(crc & 1));
    }
    return (crc ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
    const length = Buffer.alloc(4);
    length.writeUInt32BE(data.length);
    const body = Buffer.concat([Buffer.from(type, "ascii"), data]);
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(crc32(body));
    return Buffer.concat([length, body, crc]);
}

// A 1280x720 PNG with the shapes that the ORD3 driver finds by pixels: an exposed trooper column,
// a worker pixel at the authored gather seed and a gold ore node.
function scenePng() {
    const width = 1280;
    const height = 720;
    const raw = Buffer.alloc((width * 4 + 1) * height);
    const fill = (x0, y0, w, h, rgb) => {
        for (let y = y0; y < y0 + h; y += 1) {
            for (let x = x0; x < x0 + w; x += 1) {
                const offset = y * (width * 4 + 1) + 1 + x * 4;
                raw[offset] = rgb[0]; raw[offset + 1] = rgb[1]; raw[offset + 2] = rgb[2]; raw[offset + 3] = 255;
            }
        }
    };
    fill(0, 0, width, height, [20, 30, 40]);
    fill(600, 300, 12, 60, [0, 114, 178]);
    fill(423, 285, 6, 6, [0, 114, 178]);
    fill(800, 300, 20, 20, [215, 188, 95]);
    const header = Buffer.alloc(13);
    header.writeUInt32BE(width, 0);
    header.writeUInt32BE(height, 4);
    header[8] = 8;
    header[9] = 6;
    return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk("IHDR", header), chunk("IDAT", deflateSync(raw)), chunk("IEND", Buffer.alloc(0))]);
}

test("the ORD3 driver sends the 17 queue-overflow appends with no wait between them", async () => {
    const image = scenePng();
    const calls = [];
    const io = {
        async key(code, mods, options) { calls.push(`key:${code}:${options?.action || "press"}`); },
        async click(x, y, button) { calls.push(`click:${button}`); },
        async wait(ms) { calls.push(`wait:${ms}`); },
        async screenshot() { return image; },
        async save() {},
    };
    await drive(io, { ...row(), row: "ORD3" });
    let longest = 0;
    let run = 0;
    for (const call of calls) {
        run = call === "click:right" ? run + 1 : 0;
        longest = Math.max(longest, run);
    }
    assert(longest >= 17, `17 right clicks run back to back; the longest run is ${longest}`);
});

// RUN130, evidence hud2-fix4-ord3-b run 1: the Gather was accepted at tick 1692 and its held-Shift
// Append reached the server at tick 1733: 2.05 s at 20 ticks a second. With the waits kept the
// Gather must still be active at 1.2 times that time.
const LOADED_GATHER_APPEND_SECONDS = 2.05;
const GATHER_MARGIN = 1.2;

test("the ORD3 Gather order lasts longer than the held-Shift Append needs on a loaded host", () => {
    const worker = read("tests/browser/fixtures/order-rows/muster/units/worker.ron");
    const number = (pattern) => Number(pattern.exec(worker)[1]);
    const rate = number(/gather_per_second:\s*(\d+)/);
    const speed = number(/speed:\s*([0-9.]+)/);
    const range = number(/interaction_range:\s*([0-9.]+)/);
    const workerRadius = number(/footprint:\s*Circle\(radius:\s*([0-9.]+)\)/);
    const nodeRadius = Number(/radius:\s*([0-9.]+)\)/.exec(read("tests/browser/fixtures/order-rows/muster/nodes/ore_node.ron"))[1]);
    const field = read("tests/browser/fixtures/order-rows/muster/maps/field.ron");
    const node = /node: "muster:ore_node", pos: \(x: ([0-9.]+), y: ([0-9.]+)\), remaining: (\d+)/.exec(field);
    const nodeAt = [Number(node[1]), Number(node[2])];
    const remaining = Number(node[3]);
    const workers = [...field.matchAll(/unit: "muster:worker", owner_slot: 0, pos: \(x: ([0-9.]+), y: ([0-9.]+)\)/g)];
    assert(workers.length >= 1, "the map has own workers");
    const lifetimes = workers.map((match) => {
        const walk = Math.max(0, Math.hypot(Number(match[1]) - nodeAt[0], Number(match[2]) - nodeAt[1]) - (range + nodeRadius + workerRadius)) / speed;
        return walk + remaining / rate;
    });
    const shortest = Math.min(...lifetimes);
    assert(shortest >= LOADED_GATHER_APPEND_SECONDS * GATHER_MARGIN,
        `the shortest Gather lasts ${shortest.toFixed(2)} s; at least ${(LOADED_GATHER_APPEND_SECONDS * GATHER_MARGIN).toFixed(2)} s are needed`);
});

test("the ORD3 driver sends the Gather node click, the Shift down and the Append click with no wait between them", async () => {
    const image = scenePng();
    const calls = [];
    const io = {
        async key(code, mods, options) { calls.push(`key:${code}:${options?.action || "press"}`); },
        async click(x, y, button) { calls.push(`click:${button}`); },
        async wait(ms) { calls.push(`wait:${ms}`); },
        async screenshot() { return image; },
        async save() {},
    };
    await drive(io, { ...row(), row: "ORD3" });
    const gather = calls.indexOf("key:KeyG:press");
    assert(gather >= 0, "the driver presses G");
    assert.deepEqual(calls.slice(gather + 1, gather + 5), ["wait:100", "click:left", "key:ShiftLeft:down", "click:right"],
        "after the node click come the Shift down and the Append click with no wait");
});
