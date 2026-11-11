// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const { sourceControls } = require("./shell-oracle.js");

const WIDTH = 1280;
const HEIGHT = 720;
const LABEL = [546, 346, 188, 29];
const judge = sourceControls.judgeKm2OverlayPixels;
assert(judge, "the fixed KM2 pixel evaluator must be available to source controls");

function darkImage() {
    const rgba = Buffer.alloc(WIDTH * HEIGHT * 4);
    for (let index = 0; index < rgba.length; index += 4) {
        rgba[index] = 18;
        rgba[index + 1] = 20;
        rgba[index + 2] = 24;
        rgba[index + 3] = 255;
    }
    return { width: WIDTH, height: HEIGHT, rgba };
}

function paint(image, x, y, width, height, value = 250) {
    for (let py = y; py < y + height; py += 1) {
        for (let px = x; px < x + width; px += 1) {
            const index = (py * image.width + px) * 4;
            image.rgba[index] = value;
            image.rgba[index + 1] = value;
            image.rgba[index + 2] = value;
            image.rgba[index + 3] = 255;
        }
    }
}

function baseline() {
    const image = darkImage();
    for (let index = 0; index < 27; index += 1) {
        paint(image, LABEL[0] + 2 + (index % 9) * 17, LABEL[1] + 2 + Math.floor(index / 9) * 7, 1, 1);
    }
    return image;
}

function withGlyphs(image) {
    const result = { width: image.width, height: image.height, rgba: Buffer.from(image.rgba) };
    for (let index = 0; index < 13; index += 1) {
        paint(result, LABEL[0] + 2 + index * 13, LABEL[1] + 4, 8, 16);
    }
    return result;
}

function producerCapture(name, input_call, document_epoch, visible) {
    const node = {
        kind: "focus_overlay", id: "focus_overlay", root_count: visible ? 1 : 0,
        rect: visible ? [0, 0, WIDTH, HEIGHT] : null, visible,
        capture_root: visible, focus_policy_block: visible,
        descendants_truncated: false, text_truncated: false,
        text_nodes: visible ? [{ text: "Click to play", rect: LABEL, visible: true }] : [],
    };
    return {
        name, input_call, screenshot_input_call: input_call, window: 1, document_epoch,
        checkpoint: { document_epoch, debug: { ui_snapshot: [node] } },
    };
}

function attempt(before, after, beforeImage, afterImage) {
    return judge([before, after], new Map([
        [before.name, beforeImage], [after.name, afterImage],
    ]), after);
}

{
    const beforeImage = baseline();
    const afterImage = withGlyphs(beforeImage);
    const result = attempt(producerCapture("joined-focused", 3, 1, false),
        producerCapture("canvas-blurred", 24, 1, true), beforeImage, afterImage);
    assert.equal(result.beforeWhite, 27);
    assert(result.afterWhite > 1000);
    assert(result.addedWhite > 1500);
    assert.equal(result.glyph.components, 13);
}

{
    const beforeImage = baseline();
    const noText = { width: WIDTH, height: HEIGHT, rgba: Buffer.from(beforeImage.rgba) };
    assert.throws(() => attempt(producerCapture("joined-focused", 3, 1, false),
        producerCapture("canvas-blurred", 24, 1, true), beforeImage, noText), (error) => {
        assert.deepEqual(error.task16_semantic, { row: "KM2", clause: "canvas-blur-pixels" });
        return true;
    }, "the actual visible snapshot without rendered label pixels is a named product failure");
}

for (const [name, width, height] of [["6x6 patch", 6, 6], ["flat wide patch", 80, 8]]) {
    const beforeImage = baseline();
    const patchImage = { width: WIDTH, height: HEIGHT, rgba: Buffer.from(beforeImage.rgba) };
    paint(patchImage, LABEL[0] + 40, LABEL[1] + 8, width, height);
    assert.throws(() => attempt(producerCapture("joined-focused", 3, 1, false),
        producerCapture("canvas-blurred", 24, 1, true), beforeImage, patchImage), (error) => {
        assert.deepEqual(error.task16_semantic, { row: "KM2", clause: "canvas-blur-pixels" });
        return true;
    }, `${name} cannot stand in for the distributed actual label glyphs`);
}

{
    const beforeImage = baseline();
    const wrongLocation = { width: WIDTH, height: HEIGHT, rgba: Buffer.from(beforeImage.rgba) };
    paint(wrongLocation, 100, 100, 170, 20);
    assert.throws(() => attempt(producerCapture("joined-focused", 3, 1, false),
        producerCapture("canvas-blurred", 24, 1, true), beforeImage, wrongLocation), (error) => {
        assert.deepEqual(error.task16_semantic, { row: "KM2", clause: "canvas-blur-pixels" });
        return true;
    }, "bright pixels outside the producer-reported label rectangle do not count");
}

{
    const beforeImage = baseline();
    const afterImage = withGlyphs(beforeImage);
    const nearbyCounterfeit = { width: WIDTH, height: HEIGHT, rgba: Buffer.from(afterImage.rgba) };
    paint(nearbyCounterfeit, LABEL[0], LABEL[1] + LABEL[3] + 2, LABEL[2], LABEL[3]);
    assert.throws(() => attempt(producerCapture("joined-focused", 3, 1, false),
        producerCapture("canvas-blurred", 24, 1, true), beforeImage, nearbyCounterfeit), (error) => {
        assert.deepEqual(error.task16_semantic, { row: "KM2", clause: "canvas-blur-pixels" });
        return true;
    }, "the required same-sized nearby negative region cannot contain a duplicate bright patch");
}

{
    const beforeImage = baseline();
    const afterImage = withGlyphs(beforeImage);
    assert.throws(() => attempt(producerCapture("joined-focused", 3, 1, false),
        producerCapture("canvas-blurred", 24, 2, true), beforeImage, afterImage), (error) => {
        assert.equal(error.task16_semantic, undefined);
        assert.match(error.message, /same actual window and document/);
        return true;
    }, "a cross-document same-rectangle control is an instrumentation error");
}
