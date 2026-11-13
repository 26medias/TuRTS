// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const { deflateSync } = require("node:zlib");
const { test } = require("node:test");
const { drive } = require("./t7-driver.js");

function crc32(bytes) {
    let value = 0xffffffff;
    for (const byte of bytes) {
        value ^= byte;
        for (let bit = 0; bit < 8; bit += 1) value = value & 1 ? (value >>> 1) ^ 0xedb88320 : value >>> 1;
    }
    return (value ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
    const length = Buffer.alloc(4);
    length.writeUInt32BE(data.length);
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(crc32(Buffer.concat([Buffer.from(type), data])));
    return Buffer.concat([length, Buffer.from(type), data, crc]);
}

// A 320 x 180 RGB screenshot: green ground and, if `unitX` is a number, one 12 x 12 slot-0 unit.
function screenshot(unitX) {
    const width = 320;
    const height = 180;
    const rows = [];
    for (let y = 0; y < height; y += 1) {
        const row = Buffer.alloc(1 + width * 3);
        for (let x = 0; x < width; x += 1) {
            const unit = unitX !== null && x >= unitX && x < unitX + 12 && y >= 40 && y < 52;
            row.set(unit ? [0, 114, 178] : [40, 120, 40], 1 + x * 3);
        }
        rows.push(row);
    }
    const header = Buffer.alloc(13);
    header.writeUInt32BE(width, 0);
    header.writeUInt32BE(height, 4);
    header.set([8, 2, 0, 0, 0], 8);
    return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk("IHDR", header),
        chunk("IDAT", deflateSync(Buffer.concat(rows))), chunk("IEND", Buffer.alloc(0))]);
}

function fakeIo(unitAt) {
    const calls = [];
    let shots = 0;
    const io = {
        async screenshot() { const bytes = screenshot(unitAt(shots)); shots += 1; calls.push("screenshot"); return bytes; },
        async save(name) { calls.push(`save:${name}`); },
        async click(x, y, button) { calls.push(`click:${button}`); },
        async wait() {},
    };
    return { io, calls, shots: () => shots };
}

test("the T7 driver stops with a record, not a throw, when no unit is in view", async () => {
    const fake = fakeIo(() => null);
    await drive(fake.io);
    assert.ok(fake.calls.includes("save:t7-driver-stop.json"));
    assert.ok(fake.calls.includes("save:t7-find-last.png"), "the stop keeps the last screenshot for diagnosis");
    assert.deepEqual(fake.calls.filter((call) => call.startsWith("click")), ["click:left"], "only the overlay click");
});

test("the T7 driver takes at most 250 screenshots, under the 256 checkpoint limit", async () => {
    const fake = fakeIo((shot) => 20 + (shot % 50));
    await drive(fake.io);
    assert.ok(fake.shots() <= 250, `took ${fake.shots()} screenshots`);
    assert.deepEqual(fake.calls.filter((call) => call.startsWith("click")), ["click:left", "click:left", "click:right"]);
});

test("a unit that shows late and never rests still keeps the driver inside the checkpoint limit", async () => {
    const fake = fakeIo((shot) => (shot < 70 ? null : 20 + (shot % 50)));
    await drive(fake.io);
    assert.ok(fake.shots() <= 250, `took ${fake.shots()} screenshots`);
});
