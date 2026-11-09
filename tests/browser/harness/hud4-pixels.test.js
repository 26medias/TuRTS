// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const test = require("node:test");
const { classify, findOpenGround, foot, isOpenGround, readTargets } = require("./hud4-pixels.js");

function syntheticHud() {
    const image = { width: 400, height: 300, rgba: new Uint8Array(400 * 300 * 4) };
    for (let index = 0; index < 400 * 300; index += 1) {
        image.rgba[index * 4 + 3] = 255;
        image.rgba[index * 4] = 52;
        image.rgba[index * 4 + 1] = 66;
        image.rgba[index * 4 + 2] = 77;
    }
    rectangle(image, 20, 90, 360, 180, [103, 148, 101, 255]);
    rectangle(image, 50, 30, 9, 24, [200, 60, 40, 255]);
    rectangle(image, 115, 45, 15, 15, [200, 60, 40, 255]);
    ellipse(image, 190, 46, 16, [200, 60, 40, 255]);
    rectangle(image, 270, 55, 76, 20, [200, 60, 40, 255]);
    return image;
}

function rectangle(image, x, y, width, height, color) {
    for (let py = y; py < y + height; py += 1) {
        for (let px = x; px < x + width; px += 1) setPixel(image, px, py, color);
    }
}

function ellipse(image, x, y, radius, color) {
    for (let py = y - radius; py <= y + radius; py += 1) {
        for (let px = x - radius; px <= x + radius; px += 1) {
            if ((px - x) ** 2 + (py - y) ** 2 <= radius ** 2) setPixel(image, px, py, color);
        }
    }
}

function setPixel(image, x, y, rgba) {
    const offset = (y * image.width + x) * 4;
    image.rgba.set(rgba, offset);
}

test("HUD4 pixel helper identifies distinct silhouettes and open ground in pixels", () => {
    const result = readTargets(syntheticHud());
    assert.ok(result, "all four fixture silhouettes should be unique");
    assert.deepEqual(result.targets.attacker.bounds, [50, 30, 9, 24]);
    assert.deepEqual(result.targets.worker.bounds, [115, 45, 15, 15]);
    assert.deepEqual(result.targets.foreign.bounds, [174, 30, 33, 33]);
    assert.deepEqual(result.targets.node.bounds, [270, 55, 76, 20]);
    assert.ok(result.ground.x >= 0 && result.ground.x < 400);
    assert.ok(result.ground.y >= 0 && result.ground.y < 300);
    assert.deepEqual(result.ground, { x: 200, y: 165 }, "candidate comes from visibly green floor, not backdrop");
});

test("HUD4 open-ground pixels accept visible floor and reject backdrop, targets, and text", () => {
    const image = syntheticHud();
    assert.equal(isOpenGround(image, { x: 30, y: 100 }, []), true, "green floor is a real input target");
    assert.equal(isOpenGround(image, { x: 10, y: 10 }, []), false, "dark backdrop is not ground");
    assert.equal(isOpenGround(image, { x: 52, y: 35 }, []), false, "a unit pixel is not ground");
    rectangle(image, 350, 100, 10, 10, [220, 220, 220, 255]);
    assert.equal(isOpenGround(image, { x: 355, y: 105 }, []), false, "white text or UI is not ground");
    rectangle(image, 365, 100, 10, 10, [200, 160, 100, 255]);
    assert.equal(isOpenGround(image, { x: 370, y: 105 }, []), false, "resource-node colors are not ground");
});

test("HUD4 open-ground search exposes a second screenshot-derived floor point", () => {
    const image = syntheticHud();
    const first = readTargets(image);
    const avoid = Object.values(first.targets).map((target) => ({
        minX: target.bounds[0], minY: target.bounds[1],
        maxX: target.bounds[0] + target.bounds[2], maxY: target.bounds[1] + target.bounds[3],
    }));
    avoid.push({ minX: first.ground.x - 2, minY: first.ground.y - 2, maxX: first.ground.x + 2, maxY: first.ground.y + 2 });
    const second = findOpenGround(image, avoid);
    assert.ok(second, "the existing PNG helper returns another open floor point");
    assert.notDeepEqual(second, first.ground);
    assert.ok(isOpenGround(image, second, avoid), "the second point remains visible open ground");
});

test("HUD4 roles match the actual perspective-rendered fixture component sizes", () => {
    assert.equal(classify({ width: 25, height: 38, pixels: 636 }), "attacker");
    assert.equal(classify({ width: 18, height: 22, pixels: 372 }), "worker");
    assert.equal(classify({ width: 35, height: 38, pixels: 973 }), "foreign");
    assert.equal(classify({ width: 76, height: 20, pixels: 1172 }), "node");
    assert.equal(foot({ minX: 783, minY: 336, maxX: 858, maxY: 355, width: 76, height: 20 }, "node").y, 342);
});
