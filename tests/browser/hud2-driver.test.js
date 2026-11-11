// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const { deflateSync } = require("node:zlib");
const { test } = require("node:test");
const { drive } = require("./hud2-driver.js");

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

// A 1280x720 PNG whose Controls region holds one bright line at a row that depends on the scroll.
function screenshot(scroll) {
    const width = 1280;
    const height = 720;
    const raw = Buffer.alloc((width * 4 + 1) * height);
    const line = 100 + (scroll % 500);
    for (let x = 500; x < 1100; x += 1) {
        const offset = line * (width * 4 + 1) + 1 + x * 4;
        raw[offset] = 255;
        raw[offset + 1] = 255;
        raw[offset + 2] = 255;
        raw[offset + 3] = 255;
    }
    for (let y = 0; y < height; y += 1) {
        for (let x = 0; x < width; x += 1) raw[y * (width * 4 + 1) + 1 + x * 4 + 3] ||= 255;
    }
    const header = Buffer.alloc(13);
    header.writeUInt32BE(width, 0);
    header.writeUInt32BE(height, 4);
    header[8] = 8;
    header[9] = 6;
    return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk("IHDR", header), chunk("IDAT", deflateSync(raw)), chunk("IEND", Buffer.alloc(0))]);
}

// A fake page: the overlay eats input until the first click; then each wheel step scrolls the
// list by its amount until the list end (maxScroll). The drive stops at the tail screenshot.
function fakeIo(maxScroll) {
    const calls = [];
    let overlay = true;
    let scroll = 0;
    const io = {
        calls,
        async screenshot() { calls.push("screenshot"); return screenshot(scroll); },
        async save(name) { calls.push(`save:${name}`); if (name === "hud2-controls-bottom.png") throw new Error("STOP"); },
        async click() { calls.push("click"); overlay = false; },
        async move() { calls.push("move"); },
        async wheel(amount) { calls.push(`wheel:${amount}`); if (!overlay) scroll = Math.max(0, Math.min(maxScroll, scroll + amount)); },
        async wait() {},
        async key() { calls.push("key"); },
        scroll: () => scroll,
    };
    return io;
}

test("HUD2 dismisses the play overlay, then scrolls the Controls list until it stops moving (RUN130)", async () => {
    for (const maxScroll of [840, 1500]) {
        const io = fakeIo(maxScroll);
        await assert.rejects(drive(io, { row: "HUD2", package: "mods/forge" }), /STOP/);
        const firstClick = io.calls.indexOf("click");
        const firstWheel = io.calls.findIndex((call) => call.startsWith("wheel:"));
        assert.ok(firstClick >= 0 && firstClick < firstWheel, `a click must dismiss the overlay before the first wheel: ${io.calls.slice(0, 8)}`);
        assert.equal(io.scroll(), maxScroll, `the list must reach its end before the tail screenshot (max ${maxScroll})`);
    }
});

const { controlsTailRow } = require("./hud2-oracle.js");

test("the HUD2 Controls tail is the last row shown, whatever its label and place (RUN130)", () => {
    const actions = Object.fromEntries(Array.from({ length: 50 }, (_, index) => [`a${index}`, [{ Key: { code: `Key${String.fromCharCode(65 + (index % 26))}`, mods: index >= 26 ? ["Shift"] : [] } }]]));
    actions.zoom = [{ Wheel: "Up" }];
    const row = (index, text, y, visible = true) => ({ id: `guide.4.bindings/binding[index:${index}]`, kind: "widget", text, rect: [490, y, 700, 24], visible });
    const rows = Object.entries(actions).map(([name, bindings], index) => row(index, `Any label ${name}: ${bindings.map((binding) => binding.Key ? [...binding.Key.mods, binding.Key.code.slice(3)].join("+") : "Wheel up").join(", ")}`, 94 + index * 28 - 900, false));
    rows[50] = { ...rows[50], rect: [490, 640, 700, 24], visible: true };
    const debug = (list) => ({ effective_bindings: { actions }, ui_snapshot: [{ id: "guide.4.bindings", visible: true }, ...list] });
    assert.equal(controlsTailRow(debug(rows)).index, 50);
    assert.throws(() => controlsTailRow(debug(rows.slice(0, 50))), /one row for each effective action/);
    assert.throws(() => controlsTailRow(debug(rows.map((item, index) => (index === 50 ? { ...item, visible: false } : item)))), /visibly render its last row/);
    assert.throws(() => controlsTailRow(debug(rows.map((item, index) => (index === 50 ? { ...item, rect: [490, 1438, 700, 24] } : item)))), /fit the actual visible Controls viewport/);
    assert.throws(() => controlsTailRow(debug(rows.map((item, index) => (index === 50 ? { ...item, text: "Zoom: F12" } : item)))), /keys of an effective action/);
});
