// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const test = require("node:test");
const { findScout, findGround, isVisibleGround } = require("./click9-pixels.js");

function image() {
    const width = 160;
    const height = 100;
    const rgba = Buffer.alloc(width * height * 4);
    for (let index = 0; index < width * height; index += 1) rgba[index * 4 + 3] = 255;
    return { width, height, rgba };
}

function fill(frame, x, y, width, height, colour) {
    for (let py = y; py < y + height; py += 1) {
        for (let px = x; px < x + width; px += 1) {
            for (let channel = 0; channel < 4; channel += 1) frame.rgba[(py * frame.width + px) * 4 + channel] = colour[channel];
        }
    }
}

test("blank screenshots never yield a unit or ground", () => {
    const frame = image();
    assert.equal(findScout(frame), null);
    assert.equal(findGround(frame), null);
});

for (const colour of [[200, 75, 50, 255], [0, 114, 178, 255]]) {
    test("unit location follows screenshot pixels near the silhouette foot: " + colour, () => {
        const frame = image();
        fill(frame, 0, 0, 160, 100, [90, 140, 85, 255]);
        fill(frame, 24, 18, 9, 15, colour);
        const first = findScout(frame);
        assert(first);
        assert(first.x >= 24 && first.x < 33 && first.y >= 29 && first.y < 33);
        fill(frame, 24, 18, 9, 15, [90, 140, 85, 255]);
        fill(frame, 80, 40, 9, 15, colour);
        const second = findScout(frame);
        assert(second);
        assert(second.x >= 80 && second.y >= 51);
    });
}

test("green ground, white text and a large coloured UI region are not a unit", () => {
    const frame = image();
    fill(frame, 0, 0, 160, 100, [90, 140, 85, 255]);
    fill(frame, 10, 5, 8, 8, [255, 255, 255, 255]);
    fill(frame, 10, 20, 100, 12, [200, 75, 50, 255]);
    assert.equal(findScout(frame), null);
});

test("ambiguous unit-sized colour components are not silently resolved by a fixed coordinate", () => {
    const frame = image();
    fill(frame, 15, 15, 8, 14, [200, 75, 50, 255]);
    fill(frame, 80, 40, 8, 14, [200, 75, 50, 255]);
    assert.equal(findScout(frame), null);
});

test("ground must be visibly green, including when the preferred pixel becomes obscured", () => {
    const frame = image();
    fill(frame, 0, 0, 160, 100, [90, 140, 85, 255]);
    const preferred = findGround(frame);
    assert(preferred && isVisibleGround(frame, preferred));
    fill(frame, preferred.x, preferred.y, 1, 1, [0, 0, 0, 255]);
    assert.equal(isVisibleGround(frame, preferred), false);
    const other = findGround(frame);
    assert(other && isVisibleGround(frame, other));
    assert.notDeepEqual(other, preferred);
});
