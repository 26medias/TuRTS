// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const { test } = require("node:test");
const { actionLabel, assertControlsMatch, humanizeAction, playerLabel } = require("./pt1-oracle.js");
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
    assert.equal(actionLabel("key_stop"), "Stop selected units", "a mod action row takes the name of its order");
    assert.equal(actionLabel("menu"), "Menu");
    assert.equal(humanizeAction("key_stop"), "Key stop");
    assert.equal(humanizeAction("group_assign_1"), "Assign group 1");
    assert.equal(humanizeAction("camera_pan_left"), "Pan camera left");
    assert.equal(playerLabel(actions.group_assign_1[0]), "Ctrl+1");
    assert.equal(playerLabel(actions.menu[1]), "Esc");
    assert.equal(playerLabel(actions.zoom_in[0]), "Wheel up");
});

test("guide sheet 5 goes red when a rendered row differs from params.bindings", () => {
    const good = debugWith(["Stop selected units: S", "Assign group 1: Ctrl+1", "Menu: F10, Esc", "Zoom in: Wheel up"]);
    assert.equal(assertControlsMatch(good, actions, "good").rows.length, 4);
    assert.throws(() => assertControlsMatch(debugWith(["Stop selected units: A"]), actions, "stale"), /guide-sheet-5: stale: the rendered Controls row "Stop selected units: A"/);
    assert.throws(() => assertControlsMatch(debugWith(["Key stop: S"]), actions, "old label"), /matches no binding/);
    assert.throws(() => assertControlsMatch(debugWith([]), actions, "empty"), /shows no binding row/);
});

test("the new Stop key is the first letter that the effective map does not use", () => {
    assert.deepEqual(freeLetterKey({ actions }), { Key: { code: "KeyA", mods: [] } });
    const busy = { actions: { ...actions, other: [{ Key: { code: "KeyA", mods: [] } }] } };
    assert.deepEqual(freeLetterKey(busy), { Key: { code: "KeyB", mods: [] } });
});

const { layoutLabels } = require("./pt1-oracle.js");

test("guide sheet 5 uses the recorded keyboard layout labels, as the client does", () => {
    const labels = layoutLabels({ KeyS: "o", KeyA: "a", Digit1: "&", Space: " " });
    assert.deepEqual(labels, { KeyS: "O", KeyA: "A", Digit1: "&" });
    assert.equal(playerLabel(actions.key_stop[0], labels), "O");
    assert.equal(playerLabel(actions.group_assign_1[0], labels), "Ctrl+&");
    const azerty = debugWith(["Stop selected units: O", "Assign group 1: Ctrl+&"]);
    assert.equal(assertControlsMatch(azerty, actions, "azerty", labels).rows.length, 2);
    assert.throws(() => assertControlsMatch(azerty, actions, "us"), /matches no binding/);
});

const { presses } = require("./pt1-oracle.js");

test("a press of a binding is found in the input log with its exact modifiers, in its window", () => {
    const input = [
        { call: "key", window: 2, args: ["Digit1", ["Ctrl"], {}] },
        { call: "key", window: 2, args: ["Digit1", [], {}] },
        { call: "key", window: 1, args: ["Digit1", ["ControlLeft"], {}] },
        { call: "key", window: 2, args: ["KeyS", [], { action: "up" }] },
    ];
    assert.deepEqual(presses(input, actions.group_assign_1[0], 2).map(({ index }) => index), [0]);
    assert.deepEqual(presses(input, actions.group_assign_1[0], 1).map(({ index }) => index), [2], "ControlLeft counts as Ctrl");
    assert.deepEqual(presses(input, actions.key_stop[0], 2), [], "a key-up is not a press");
});
