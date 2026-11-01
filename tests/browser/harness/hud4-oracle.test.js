// turts-test-role: test
"use strict";

const assert = require("node:assert/strict");
const test = require("node:test");
const row = require("../rows/HUD4.json");
const { validateFixtureCatalog, verifyInput, verifyStage } = require("../hud4-oracle.js");

const clipFromWorld = [
    1, 0, 0, 0,
    0, 0, 1, 0,
    0, 1, 0, 0,
    0, 0, 0, 1,
];

function foreignAttackCheckpoint({ ownerSlot = 1, contentId = "muster:dummy", appliedIcon = "muster_cursor_attack", radius = 0.1, matrix = clipFromWorld } = {}) {
    return {
        scene: {
            coordinate_space: "window-logical-pixels-top-left",
            viewport_logical_rect: [0, 0, 400, 200],
            clip_from_world: matrix,
            units: [
                { unit_id: 11, ground_screen_center: [50, 60], translation: [-0.75, 0, -0.4], pick_radius: 0.1 },
                { unit_id: 12, ground_screen_center: [220, 100], translation: [0.1, 0, 0], pick_radius: radius },
            ],
            nodes: [{ node_id: 21, ground_screen_center: [350, 60], translation: [0.75, 0, 0.4], pick_radius: 0.12 }],
            widgets: [],
        },
        debug: {
            selection: [11],
            cursor: { context: "attack", image_id: "muster_cursor_attack", applied_icon: appliedIcon },
            replicated_world: {
                units: [
                    { unit_id: 11, content_id: "muster:trooper", owner_slot: 0 },
                    { unit_id: 12, content_id: contentId, owner_slot: ownerSlot },
                ],
                nodes: [{ id: 21, content_id: "muster:ore_node" }],
            },
        },
    };
}

function foreignAttackStage() {
    return row.stages.find((stage) => stage.label === "hover-foreign-attack");
}

test("HUD4 oracle accepts a pixel whose captured camera ray intersects the actual pick footprint", () => {
    validateFixtureCatalog(row.fixture_catalog);
    assert.doesNotThrow(() => verifyStage(
        foreignAttackCheckpoint(), foreignAttackStage(), { x: 220, y: 100 }, row,
    ));
});

test("HUD4 oracle rejects a near pixel outside the actual pick radius even inside 48 CSS pixels", () => {
    // x=250 maps to world x=0.25: 30 CSS px from the entity center, but 0.15 world units away.
    assert.throws(() => verifyStage(
        foreignAttackCheckpoint(), foreignAttackStage(), { x: 250, y: 100 }, row,
    ), /misses the actual rendered unit pick footprint/);
});

test("HUD4 oracle rejects a wrong-role or wrong-owner pixel target", () => {
    assert.throws(() => verifyStage(
        foreignAttackCheckpoint({ ownerSlot: 0 }), foreignAttackStage(), { x: 220, y: 100 }, row,
    ), /public owner_slot mismatch/);
    assert.throws(() => verifyStage(
        foreignAttackCheckpoint({ ownerSlot: 1, contentId: "muster:worker" }), foreignAttackStage(), { x: 220, y: 100 }, row,
    ), /content role mismatch/);
});

test("HUD4 oracle rejects a mod cursor image whose CursorIcon was not actually applied", () => {
    assert.throws(() => verifyStage(
        foreignAttackCheckpoint({ appliedIcon: "System(CursorIcon::Default)" }), foreignAttackStage(), { x: 220, y: 100 }, row,
    ), /actual applied CursorIcon mismatch/);
});

test("HUD4 default cursor stage requires cleared selection and the actually applied default asset", () => {
    const stage = row.stages.find((entry) => entry.label === "default-ground");
    const checkpoint = foreignAttackCheckpoint({ appliedIcon: "muster_cursor_default" });
    checkpoint.debug.selection = [];
    checkpoint.debug.cursor = {
        context: "default",
        image_id: "muster_cursor_default",
        applied_icon: "muster_cursor_default",
    };
    assert.doesNotThrow(() => verifyStage(checkpoint, stage, { x: 300, y: 20 }, row));

    const staleSelection = foreignAttackCheckpoint({ appliedIcon: "muster_cursor_default" });
    staleSelection.debug.cursor = checkpoint.debug.cursor;
    assert.throws(() => verifyStage(staleSelection, stage, { x: 300, y: 20 }, row), /must prove the selection is cleared/);

    const staleCursor = structuredClone(checkpoint);
    staleCursor.debug.cursor.image_id = "muster_cursor_move";
    staleCursor.debug.cursor.applied_icon = "muster_cursor_move";
    assert.throws(() => verifyStage(staleCursor, stage, { x: 300, y: 20 }, row), /cursor context mismatch|mod cursor image mismatch/);

    const capturedByWidget = structuredClone(checkpoint);
    capturedByWidget.scene.widgets = [{ id: "hud.card", rect: [290, 10, 40, 30] }];
    assert.throws(() => verifyStage(capturedByWidget, stage, { x: 300, y: 20 }, row), /falls inside rendered widget hud.card/);
});

test("HUD4 selection-clearing click requires ordered real canvas pointerdown and pointerup", () => {
    const stage = row.stages.find((entry) => entry.label === "default-ground");
    const pixel = { x: 300, y: 20 };
    const click = {
        call: "click",
        args: [pixel.x, pixel.y, "left"],
        actual_events: [
            { type: "pointerdown", target: "CANVAS", canvas_target: true, seq: 41 },
            { type: "pointerup", target: "CANVAS", canvas_target: true, seq: 42 },
        ],
    };
    assert.doesNotThrow(() => verifyInput([click], stage, pixel));
    for (const invalid of [
        { ...click, actual_events: click.actual_events.slice(0, 1) },
        { ...click, actual_events: click.actual_events.map((event) => ({ ...event, target: "BODY", canvas_target: false })) },
        { ...click, actual_events: click.actual_events.map((event) => ({ ...event, seq: 0 })) },
        { ...click, args: [pixel.x + 1, pixel.y, "left"] },
    ]) {
        assert.throws(() => verifyInput([invalid], stage, pixel), /ordered canvas left click/);
    }
});

test("HUD4 oracle rejects missing, malformed, and singular camera matrices", () => {
    for (const matrix of [null, [1, 2, 3], new Array(16).fill(0)]) {
        const checkpoint = foreignAttackCheckpoint();
        if (matrix === null) delete checkpoint.scene.clip_from_world;
        else checkpoint.scene.clip_from_world = matrix;
        assert.throws(() => verifyStage(checkpoint, foreignAttackStage(), { x: 220, y: 100 }, row),
            /clip_from_world matrix|camera matrix is singular/);
    }
});

test("HUD4 oracle rejects missing, non-finite, or non-positive pick geometry", () => {
    for (const mutate of [
        (entry) => { delete entry.pick_radius; },
        (entry) => { entry.pick_radius = Number.NaN; },
        (entry) => { entry.pick_radius = 0; },
        (entry) => { entry.translation[2] = Number.POSITIVE_INFINITY; },
    ]) {
        const checkpoint = foreignAttackCheckpoint();
        mutate(checkpoint.scene.units[1]);
        assert.throws(() => verifyStage(checkpoint, foreignAttackStage(), { x: 220, y: 100 }, row),
            /pick radius|world translation/);
    }
});
