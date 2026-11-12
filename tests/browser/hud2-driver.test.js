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

const { controlsTailRow, humanizeAction } = require("./hud2-oracle.js");

// Fifty-one actions with fixed texts (written out here, not made by the oracle code): 26 order hotkeys
// "key_a".."key_z" (order names "Order A".."Order Z", keys A..Z), 24 "group_add_1".."group_add_24"
// (label "Add to group N", keys Shift+Digit N mod 10), the Toggle guide action (F1) and the zoom action.
const LETTERS = Array.from({ length: 26 }, (_, index) => String.fromCharCode(65 + index));
const ORDER_LABELS = Object.fromEntries(LETTERS.map((letter) => [`key_${letter.toLowerCase()}`, `Order ${letter}`]));
function controlsFixture() {
    const actions = {};
    const texts = {};
    for (const letter of LETTERS) {
        actions[`key_${letter.toLowerCase()}`] = [{ Key: { code: `Key${letter}`, mods: [] } }];
        texts[`key_${letter.toLowerCase()}`] = `Order ${letter}: ${letter}`;
    }
    for (let number = 1; number <= 24; number += 1) {
        actions[`group_add_${number}`] = [{ Key: { code: `Digit${number % 10}`, mods: ["Shift"] } }];
        texts[`group_add_${number}`] = `Add to group ${number}: Shift+${number % 10}`;
    }
    actions.toggle_guide = [{ Key: { code: "F1", mods: [] } }];
    texts.toggle_guide = "Toggle guide: F1";
    actions.camera_zoom = [{ Wheel: "Up" }];
    texts.camera_zoom = "Zoom camera: Wheel up";
    return { actions, texts };
}

// The rows in a scrambled order (index order is the product's, the content order is not required).
function controlsRowsFor(texts, visibleFrom = 46) {
    return Object.values(texts).map((text, index) => ({
        id: `guide.4.bindings/binding[index:${index}]`, kind: "widget", text, rect: [490, 94 + (index - visibleFrom + 20) * 28, 700, 24],
        visible: index >= visibleFrom,
    }));
}

test("the HUD2 Controls check: every row is the label and the keys of one action (review 2140da4, high 1)", () => {
    const { actions, texts } = controlsFixture();
    assert.equal(Object.keys(actions).length, 52);
    const rows = controlsRowsFor(texts);
    rows[51] = { ...rows[51], rect: [490, 640, 700, 24], visible: true };
    const debug = (list) => ({ effective_bindings: { actions }, ui_snapshot: [{ id: "guide.4.bindings", visible: true }, ...list] });
    const edit = (index, change) => debug(rows.map((item, at) => (at === index ? { ...item, ...change } : item)));
    assert.equal(controlsTailRow(debug(rows), ORDER_LABELS).index, 51, "the true list is green");
    const red = (list, pattern) => assert.throws(() => controlsTailRow(list, ORDER_LABELS), pattern);
    red(debug(rows.slice(0, 51)), /one row for each effective action/);
    red(edit(51, { visible: false }), /visibly render its last row/);
    red(edit(51, { rect: [490, 1438, 700, 24] }), /fit the actual visible Controls viewport/);
    // The reviewer's plants: a middle row with a wrong key, the tail row with the keys of another
    // action, a wrong tail label, a wrong middle label, and two rows that swap their keys.
    red(edit(50, { text: "Toggle guide: F12" }), /sorted key texts/);
    red(edit(51, { text: "Add to group 1: Shift+1" }), /sorted key texts/);
    red(edit(51, { text: "Zoom: Wheel up" }), /one effective action/);
    red(edit(50, { text: "Toggle gide: F1" }), /one effective action/);
    const swapped = rows.map((item, at) => (at === 50 ? { ...item, text: "Toggle guide: Wheel up" } : at === 51 ? { ...item, text: "Zoom camera: F1" } : item));
    red(debug(swapped), /one effective action/);
    const duplicated = rows.map((item, at) => (at === 51 ? { ...item, id: "guide.4.bindings/binding[index:50]" } : item));
    red(debug(duplicated), /indexes must run from 0/);
    // The order names come from the package: with no order names, the Order rows are red.
    assert.throws(() => controlsTailRow(debug(rows), {}), /one effective action/);
});

test("the HUD2 action labels are the readable action names (sources.rs humanize_action)", () => {
    assert.equal(humanizeAction("group_assign_1"), "Assign group 1");
    assert.equal(humanizeAction("camera_pan_down"), "Pan camera down");
    assert.equal(humanizeAction("toggle_guide"), "Toggle guide");
    assert.equal(humanizeAction("type_modifier"), "Type modifier");
    assert.equal(humanizeAction("selection_remove"), "Remove from selection");
});
