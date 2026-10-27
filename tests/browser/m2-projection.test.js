"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { test } = require("node:test");
const {
    findPressedUiAction,
    findVisibleUiAction,
    publicProjection,
    assertPublicProjectionSchema,
    findVisibleUiText,
    projectGroundPoint,
    projectRenderedEntity,
    uiActionWindowPoint,
} = require("./m2.js");

const capturedUiRows = JSON.parse(fs.readFileSync(
    path.join(__dirname, "fixtures/task16-selected-ui-raw.json"),
    "utf8",
));
const queuedUiRows = JSON.parse(fs.readFileSync(
    path.join(__dirname, "fixtures/task16-queued-ui-raw.json"),
    "utf8",
));
const gatheredUiRows = JSON.parse(fs.readFileSync(
    path.join(__dirname, "fixtures/task16-gather-ui-raw.json"),
    "utf8",
));

const identityScene = {
    coordinate_space: "window-logical-pixels-top-left",
    window_logical_size: [1280, 720],
    viewport_logical_rect: [100, 40, 640, 360],
    clip_from_world: [
        1, 0, 0, 0,
        0, 1, 0, 0,
        0, 0, 1, 0,
        0, 0, 0, 1,
    ],
};

test("projects ground points through camera matrix into the target logical viewport", () => {
    assert.deepEqual(projectGroundPoint(identityScene, [0, 0.5]), [420, 220]);
    assert.equal(projectGroundPoint(identityScene, [2, 0.5]), null);
});

test("cross-checks actual mesh ground centers and requires the catalog pick circle", () => {
    const entry = {
        translation: [0, 0.9, 0.5],
        ground_screen_center: [420, 220],
        pick_radius: 0.5,
    };
    assert.deepEqual(projectRenderedEntity(identityScene, entry), [420, 220]);
    assert.throws(
        () => projectRenderedEntity(identityScene, { ...entry, pick_radius: null }),
        /declared pick radius/,
    );
});

test("rejects projection without camera metadata or outside the near and far planes", () => {
    assert.equal(projectGroundPoint({ ...identityScene, clip_from_world: null }, [0, 0.5]), null);
    assert.equal(projectGroundPoint(identityScene, [0, -0.1]), null);
    assert.equal(projectGroundPoint(identityScene, [0, 1.1]), null);
});

test("maps fully visible UI action rectangles to window-logical pointer coordinates", () => {
    const scene = {
        viewport_physical_rect: [200, 100, 1280, 720],
        scale_factor: 2,
    };
    assert.deepEqual(uiActionWindowPoint(scene, [24, 24, 100, 20]), [137, 67]);
    assert.equal(uiActionWindowPoint(scene, [-1, 24, 100, 20]), null);
    assert.equal(uiActionWindowPoint(scene, [24, 24, 1300, 20]), null);
});


test("selects enabled Produce from the saved real Bevy turts_ui payload", () => {
    const produce = findVisibleUiAction(capturedUiRows, "produce", (action, text) =>
        text === "Produce Train Rifleman" && action.producer === 1 && action.recipe === "base:train_rifleman"
    );
    assert.ok(produce);
    assert.equal(produce.visible, true);
    assert.equal(produce.enabled, true);
    assert.equal(produce.button_pressed, false);
    assert.equal(findPressedUiAction(capturedUiRows, "produce", (action) => action.producer === 1), undefined);

    const disabled = findVisibleUiAction(capturedUiRows, "produce", (action, text) =>
        text === "Produce Build Harvester" && action.producer === 1
    );
    assert.equal(disabled, undefined);
});

test("selects Cancel from the saved real queue payload and requires owner Pressed", () => {
    const cancel = findVisibleUiAction(queuedUiRows, "cancel", (action, text) =>
        text === "Cancel job 2" && action.producer === 1 && action.job === 2
    );
    assert.ok(cancel);
    assert.equal(cancel.visible, true);
    assert.equal(cancel.enabled, true);
    assert.equal(cancel.button_pressed, false);
    assert.equal(findPressedUiAction(queuedUiRows, "cancel", (action) => action.job === 2), undefined);

    const pressedRows = queuedUiRows.map((row) => row === cancel ? { ...row, button_pressed: true } : row);
    assert.equal(findPressedUiAction(pressedRows, "cancel", (action) => action.job === 2).text, "Cancel job 2");
    assert.equal(findVisibleUiAction(queuedUiRows, "produce", (action) => action.job === 2), undefined);
    assert.equal(findPressedUiAction(queuedUiRows.map((row) => ({ ...row, enabled: false })), "cancel"), undefined);
});

test("Gather is actual visible rendered status text, not a UI action or synthetic activation", () => {
    const gather = findVisibleUiText(gatheredUiRows, (text) => text === "Gather: gathering");
    assert.ok(gather);
    assert.equal(gather.visible, true);
    assert.equal(gather.enabled, true);
    assert.equal(gather.action, null);
    assert.equal(gather.button_pressed, null);
    assert.equal(findVisibleUiAction(gatheredUiRows, "gather"), undefined);
    assert.equal(findVisibleUiText(gatheredUiRows.map((row) => ({ ...row, visible: false })), (text) => text.startsWith("Gather:")), undefined);
});

const actualPublicDebug = JSON.parse(fs.readFileSync(
    path.join(__dirname, "fixtures/task16-public-world-debug.json"),
    "utf8",
));

test("actual browser projection exposes all public rows without private DTO fields", () => {
    assertPublicProjectionSchema({ debug: actualPublicDebug });
    const projection = publicProjection({ debug: actualPublicDebug });
    assert.equal(projection.units.length, actualPublicDebug.replicated_world.units.length);
    assert.equal(projection.nodes.length, actualPublicDebug.replicated_world.nodes.length);
    assert.ok(projection.units.every((unit) => Number.isInteger(unit.owner_slot)));
    assert.ok(projection.nodes.every((node) => Number.isFinite(node.remaining)));
});
