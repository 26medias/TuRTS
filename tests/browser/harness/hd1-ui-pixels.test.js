// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const test = require("node:test");
const { UI_COLORS, countCellColor, expectedButtonColor } = require("./hd1-ui-pixels.js");

function image(width = 128, height = 128) {
    const rgba = new Uint8Array(width * height * 4);
    for (let index = 3; index < rgba.length; index += 4) rgba[index] = 255;
    return { width, height, rgba };
}

function paint(target, x, y, color) {
    const offset = (y * target.width + x) * 4;
    target.rgba.set([...color, 255], offset);
}

test("HD1 UI pixel palette names the authored order and producer controls", () => {
    const cases = [
        [{ order: "muster:order_move" }, UI_COLORS.move],
        [{ order: "muster:order_stop" }, UI_COLORS.stop],
        [{ order: "muster:order_hold" }, UI_COLORS.hold],
        [{ order: "muster:order_attack" }, UI_COLORS.attack],
        [{ order: "muster:order_gather" }, UI_COLORS.gather],
        [{ order: "muster:order_remove" }, UI_COLORS.remove],
        [{ order: "muster:order_disable" }, UI_COLORS.disable],
        [{ order: "muster:order_enable" }, UI_COLORS.enable],
        [{ order: "muster:order_research", content_id: "muster:make_dummy" }, UI_COLORS.produceBeacon],
        [{ order: "muster:order_cancel", job_id: 1 }, UI_COLORS.cancel],
    ];
    for (const [action, expected] of cases) assert.deepEqual(expectedButtonColor(action), expected);
    assert.throws(() => expectedButtonColor({ order: "unknown:order" }), /no authored button color/);
});

test("HD1 button color count inspects only the cell interior with calibrated tolerance", () => {
    const pixels = image();
    const center = [64, 64];
    const target = UI_COLORS.move;
    // A 64px cell uses a 5px inset, so these six samples are inside its bounded interior.
    for (const [x, y] of [[45, 45], [46, 45], [47, 45], [45, 46], [46, 46], [47, 46]]) {
        paint(pixels, x, y, [target[0] + 3, target[1] - 3, target[2]]);
    }
    paint(pixels, 34, 34, target);
    const observed = countCellColor(pixels, center, 64, target);
    assert.equal(observed.pixels, 6, "tolerance is inclusive and out-of-cell pixels do not count");
    assert.equal(observed.required_pixels, 5);
    assert.deepEqual(observed.region, [37, 37, 91, 91]);
});

test("HD1 queue button color uses its smaller authored 32px cell", () => {
    const pixels = image();
    const center = [64, 64];
    for (const [x, y] of [[60, 60], [61, 60], [62, 60], [60, 61], [61, 61]]) {
        paint(pixels, x, y, UI_COLORS.cancel);
    }
    assert.equal(countCellColor(pixels, center, 32, UI_COLORS.cancel).pixels, 5);
});
