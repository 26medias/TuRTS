// turts-test-role: oracle
"use strict";

const assert = require("node:assert/strict");
const EPSILON = 1;
const ROOT_WIDGETS = Object.freeze(["card", "minimap", "ore"]);

async function judge(view, params) {
    const input = view.inputLog();
    const selectedUnitClick = input.find((entry) =>
        entry.call === "click" && entry.args?.[2] === "left" && entry.event_seq?.length >= 3);
    assert(selectedUnitClick, "HUD1 must use an actual harness left click to select a visible unit");

    const debug = await view.bridge("turts_debug");
    const selection = debug?.selection;
    assert(Array.isArray(selection) && selection.length > 0,
        "HUD1 actual debug selection must be nonempty before widget placement is judged");

    const scene = await view.bridge("turts_scene");
    const legacyUi = await view.bridge("turts_ui");
    const faults = [];
    const viewport = params?.viewport;
    const expectedWidth = viewport?.width;
    const expectedHeight = viewport?.height;
    const expectedDpr = params?.device_scale_factor ?? viewport?.device_scale_factor;

    if (!scene || scene.cameras_3d !== 1) faults.push("HUD1 requires one active 3D camera in the actual scene bridge");
    if (!scene || scene.coordinate_space !== "window-logical-pixels-top-left") {
        faults.push("HUD1 scene coordinates must be window-logical-pixels-top-left");
    }
    if (!Array.isArray(scene?.window_logical_size) || scene.window_logical_size.length !== 2 ||
        !scene.window_logical_size.every(Number.isFinite) ||
        !near(scene.window_logical_size[0], expectedWidth) || !near(scene.window_logical_size[1], expectedHeight)) {
        faults.push("HUD1 actual window size does not match the requested logical viewport");
    }
    if (!validRect(scene?.viewport_logical_rect) ||
        !rectNear(scene.viewport_logical_rect, [0, 0, expectedWidth, expectedHeight])) {
        faults.push("HUD1 actual camera viewport does not fill the requested logical canvas");
    }
    if (!Number.isFinite(scene?.scale_factor) || scene.scale_factor <= 0 ||
        !Number.isFinite(expectedDpr) || Math.abs(scene.scale_factor - expectedDpr) > 1e-6) {
        faults.push("HUD1 actual window scale factor does not match the requested device scale factor");
    }

    if (!Array.isArray(legacyUi)) {
        faults.push("HUD1 actual UI bridge is not an array of rendered UI rows");
    } else if (legacyUi.some((row) => row?.kind === "panel" && row.visible === true)) {
        faults.push("engine panel present");
    }

    const widgets = scene?.widgets;
    const occurrences = new Map();
    const byId = new Map();
    if (Array.isArray(widgets)) {
        for (const widget of widgets) {
            if (!widget || typeof widget.id !== "string" || !widget.id.trim()) {
                faults.push("HUD1 rendered widget has a missing or invalid id");
                continue;
            }
            occurrences.set(widget.id, (occurrences.get(widget.id) || 0) + 1);
            if (!validRect(widget.rect)) faults.push(`HUD1 widget ${widget.id} has a non-finite, zero-size, or invalid rect`);
            else byId.set(widget.id, widget.rect);
        }
    } else {
        faults.push("HUD1 actual scene bridge has no rendered widgets array");
    }
    for (const [id, count] of occurrences) {
        if (count > 1) faults.push(`HUD1 duplicate rendered widget id: ${id}`);
    }
    for (const id of ROOT_WIDGETS) {
        const widget = Array.isArray(widgets) ? widgets.find((entry) => entry?.id === id) : undefined;
        if (!widget) {
            faults.push(id === "card" ? "widget card missing" : `widget ${id} missing`);
            continue;
        }
    }

    if (Number.isFinite(expectedWidth) && Number.isFinite(expectedHeight)) {
        const expected = {
            card: [8, 8, 272, 204],
            minimap: [expectedWidth - 208, expectedHeight - 208, 200, 200],
            ore: [(expectedWidth - 160) / 2, expectedHeight - 32, 160, 28],
        };
        for (const id of ROOT_WIDGETS) {
            const actual = byId.get(id);
            if (actual && !rectNear(actual, expected[id])) {
                faults.push(`HUD1 widget ${id} rect ${JSON.stringify(actual)} differs from authored logical rect ${JSON.stringify(expected[id])} by more than 1 px`);
            }
        }
    }

    if (faults.length) throw new Error(`HUD1: ${faults.join("; ")}`);
}

function near(actual, expected) {
    return Number.isFinite(actual) && Number.isFinite(expected) && Math.abs(actual - expected) <= EPSILON;
}

function validRect(rect) {
    return Array.isArray(rect) && rect.length === 4 && rect.every(Number.isFinite) &&
        rect[2] > 0 && rect[3] > 0;
}

function rectNear(actual, expected) {
    return validRect(actual) && expected.every(Number.isFinite) &&
        actual.every((value, index) => Math.abs(value - expected[index]) <= EPSILON);
}

module.exports = { judge };
