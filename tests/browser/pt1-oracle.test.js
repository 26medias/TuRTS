// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const { test } = require("node:test");
const { assertControlsMatch, humanizeAction, playerLabel } = require("./pt1-oracle.js");
const { freeLetterKey } = require("./pt1-driver.js");

const actions = {
    key_stop: [{ Key: { code: "KeyS", mods: [] } }],
    group_assign_1: [{ Key: { code: "Digit1", mods: ["Ctrl"] } }],
    menu: [{ Key: { code: "F10", mods: [] } }, { Key: { code: "Escape", mods: [] } }],
    zoom_in: [{ Wheel: "Up" }],
};

function debugWith(texts) {
    return { ui_snapshot: texts.map((text, index) => ({ id: `guide.4.bindings/${index}`, visible: true, text })) };
}

test("the Controls rows mirror the Rust labels", () => {
    assert.equal(humanizeAction("key_stop"), "Key stop");
    assert.equal(humanizeAction("group_assign_1"), "Assign group 1");
    assert.equal(humanizeAction("camera_pan_left"), "Pan camera left");
    assert.equal(playerLabel(actions.group_assign_1[0]), "Ctrl+1");
    assert.equal(playerLabel(actions.menu[1]), "Esc");
    assert.equal(playerLabel(actions.zoom_in[0]), "Wheel up");
});

test("guide sheet 5 goes red when a rendered row differs from params.bindings", () => {
    const good = debugWith(["Key stop: S", "Assign group 1: Ctrl+1", "Menu: F10, Esc", "Zoom in: Wheel up"]);
    assert.equal(assertControlsMatch(good, actions, "good").rows.length, 4);
    assert.throws(() => assertControlsMatch(debugWith(["Key stop: A"]), actions, "stale"), /guide-sheet-5: stale: the rendered Controls row "Key stop: A"/);
    assert.throws(() => assertControlsMatch(debugWith([]), actions, "empty"), /shows no binding row/);
});

test("the new Stop key is the first letter that the effective map does not use", () => {
    assert.deepEqual(freeLetterKey({ actions }), { Key: { code: "KeyA", mods: [] } });
    const busy = { actions: { ...actions, other: [{ Key: { code: "KeyA", mods: [] } }] } };
    assert.deepEqual(freeLetterKey(busy), { Key: { code: "KeyB", mods: [] } });
});
