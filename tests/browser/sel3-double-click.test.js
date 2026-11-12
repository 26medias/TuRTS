// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const { deflateSync } = require("node:zlib");
const { test } = require("node:test");
const { drive } = require("./sel-driver.js");
const { __test } = require("./sel-oracle.js");

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

const WIDTH = 1280;
const HEIGHT = 720;

function png(rects) {
    const raw = Buffer.alloc((WIDTH * 4 + 1) * HEIGHT);
    for (let y = 0; y < HEIGHT; y += 1) {
        for (let x = 0; x < WIDTH; x += 1) {
            const offset = y * (WIDTH * 4 + 1) + 1 + x * 4;
            raw[offset] = 20; raw[offset + 1] = 30; raw[offset + 2] = 40; raw[offset + 3] = 255;
        }
    }
    for (const [x0, y0, w, h, rgb] of rects) {
        for (let y = y0; y < y0 + h; y += 1) {
            for (let x = x0; x < x0 + w; x += 1) {
                const offset = y * (WIDTH * 4 + 1) + 1 + x * 4;
                raw[offset] = rgb[0]; raw[offset + 1] = rgb[1]; raw[offset + 2] = rgb[2];
            }
        }
    }
    const header = Buffer.alloc(13);
    header.writeUInt32BE(WIDTH, 0);
    header.writeUInt32BE(HEIGHT, 4);
    header[8] = 8;
    header[9] = 6;
    return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk("IHDR", header), chunk("IDAT", deflateSync(raw)), chunk("IEND", Buffer.alloc(0))]);
}

// Own-colour trooper column near the centre (the first and the zoomed view) and a body at the
// camera target (width - 160, height / 2).
const IMAGE = png([[634, 330, 12, 60, [0, 114, 178]], [1114, 355, 12, 12, [0, 114, 178]]]);

function fakeIo() {
    const calls = [];
    return {
        calls,
        async key(code, mods, options) { calls.push({ call: "key", code, options }); },
        async screenshot() { calls.push({ call: "screenshot" }); return IMAGE; },
        async save(name) { calls.push({ call: "save", name }); },
        async drag(...args) { calls.push({ call: "drag", args }); },
        async wheel(dy) { calls.push({ call: "wheel", dy }); },
        async wait(ms) { calls.push({ call: "wait", ms }); },
        async click(x, y, button, options) { calls.push({ call: "click", args: [x, y, button], options }); },
    };
}

const SEL3 = {
    row: "SEL3",
    stages: [
        { label: "sel3-ctrl-type", gesture: "click", selector: "trooper-column-slot0", modifiers: ["Control"] },
        { label: "sel3-double-type", gesture: "double-click", selector: "trooper-column-slot0" },
    ],
};

test("SEL3 sends the double click as one harness click with count 2 and no wait in it", async () => {
    const io = fakeIo();
    await drive(io, SEL3);
    const clicks = io.calls.filter((entry) => entry.call === "click");
    assert.equal(clicks.length, 2, "one click for the Control stage and one for the double click stage");
    assert.deepEqual(clicks[1].options, { count: 2 }, "the double click is one action with count 2");
    const index = io.calls.indexOf(clicks[1]);
    assert.notEqual(io.calls[index - 1].call, "wait", "no wait directly before the double click");
    assert.notEqual(io.calls[index + 1].call, "wait", "no wait directly after the double click");
});

function press(type, seq, time, x = 1120, y = 360) { return { type, seq, wall_time_ms: time, client_x: x, client_y: y, canvas_target: true }; }
function action(gapMs, x2 = 1120) {
    return {
        call: "click", args: [1120, 360, "left", { count: 2 }],
        actual_events: [press("pointerdown", 7, 1000), press("pointerup", 8, 1010), press("pointerdown", 9, 1000 + gapMs, x2), press("pointerup", 10, 1010 + gapMs, x2)],
    };
}
const stage = { label: "sel3-double-type" };

test("the SEL3 oracle accepts two presses 60 ms apart", () => {
    __test.assertDoubleClick(action(60), stage);
});

test("the SEL3 oracle rejects presses outside the 400 ms window (the RUN130 gap was 468 ms)", () => {
    assert.throws(() => __test.assertDoubleClick(action(468), stage), /400 ms double-click window/);
    assert.throws(() => __test.assertDoubleClick(action(600), stage), /400 ms double-click window/);
    __test.assertDoubleClick(action(400), stage);
});

test("the SEL3 oracle rejects presses more than 4 px apart, two separate clicks and a missing count", () => {
    assert.throws(() => __test.assertDoubleClick(action(60, 1125), stage), /4 px double-click radius/);
    const single = { ...action(60), args: [1120, 360, "left"] };
    assert.throws(() => __test.assertDoubleClick(single, stage), /one harness action with count 2/);
    const oneClick = { ...action(60), actual_events: action(60).actual_events.slice(0, 2) };
    assert.throws(() => __test.assertDoubleClick(oneClick, stage), /two actual press and release pairs/);
});
