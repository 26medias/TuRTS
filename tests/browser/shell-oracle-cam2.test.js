// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const { sourceControls } = require("./shell-oracle.js");

const judge = sourceControls.judgeCam2;
assert(judge, "the fixed CAM2 physical cursor-lock evaluator must be available to source controls");

function inputs() {
    return [
        { call: "click", args: [500, 430, "left"], window: 1, document_epoch: 1,
            actual_events: [{ type: "pointerdown", seq: 1 }] },
        { call: "wait", args: [250], window: 1, document_epoch: 1, actual_events: [] },
        { call: "key", args: ["F1", []], window: 1, document_epoch: 1,
            actual_events: [{ type: "keydown", code: "F1", seq: 2 }] },
        { call: "wait", args: [250], window: 1, document_epoch: 1, actual_events: [] },
        { call: "screenshot", args: [], window: 1, document_epoch: 1, actual_events: [] },
        { call: "key", args: ["F10", []], window: 1, document_epoch: 1,
            actual_events: [{ type: "keydown", code: "F10", seq: 3 }] },
        { call: "wait", args: [200], window: 1, document_epoch: 1, actual_events: [] },
        { call: "screenshot", args: [], window: 1, document_epoch: 1, actual_events: [] },
        { call: "move", args: [650, 500], window: 1, document_epoch: 1, actual_events: [] },
        { call: "wheel", args: [10000], window: 1, document_epoch: 1,
            actual_events: [{ type: "wheel", seq: 4 }] },
        { call: "wait", args: [250], window: 1, document_epoch: 1, actual_events: [] },
        { call: "click", args: [640, 441, "left"], window: 1, document_epoch: 1,
            actual_events: [{ type: "pointerdown", seq: 5 }] },
        { call: "wait", args: [250], window: 1, document_epoch: 1, actual_events: [] },
        { call: "screenshot", args: [], window: 1, document_epoch: 1, actual_events: [] },
        { call: "key", args: ["F10", []], window: 1, document_epoch: 1,
            actual_events: [{ type: "keydown", code: "F10", seq: 6 }] },
        { call: "wait", args: [200], window: 1, document_epoch: 1, actual_events: [] },
        { call: "click", args: [640, 360, "left"], window: 1, document_epoch: 1,
            actual_events: [{ type: "pointerdown", seq: 7 }] },
        { call: "wait", args: [500], window: 1, document_epoch: 1, actual_events: [] },
        { call: "screenshot", args: [], window: 1, document_epoch: 1, actual_events: [] },
        { call: "move", args: [700, 360], window: 1, document_epoch: 1,
            actual_events: [{ type: "pointermove", target: "CANVAS", canvas_target: true,
                canvas_pointer_locked: true, movement_x: 60, movement_y: 0, seq: 8 }] },
        { call: "screenshot", args: [], window: 1, document_epoch: 1, actual_events: [] },
        { call: "wait", args: [150], window: 1, document_epoch: 1, actual_events: [] },
        { call: "move", args: [1, 360], window: 1, document_epoch: 1,
            actual_events: [{ type: "pointermove", target: "CANVAS", canvas_target: true,
                canvas_pointer_locked: true, movement_x: -699, movement_y: 0, seq: 9 }] },
        { call: "wait", args: [150], window: 1, document_epoch: 1, actual_events: [] },
        { call: "screenshot", args: [], window: 1, document_epoch: 1, actual_events: [] },
    ];
}

function capture(name, inputCall, { label = true, enabled = true, active = true } = {}) {
    const frames = [{ frame_index: 1, named_dispatches_truncated: 0, applied_seq: [3], named_dispatches: [
        { origin: "key", code: "F10", matched_action: "menu" },
    ] }];
    return {
        name, input_call: inputCall, window: 1, document_epoch: 1,
        checkpoint: { browser: { canvas_pointer_locked: active }, debug: {
            frames: { recent: frames, retained_input: [] },
            cursor: { lock_requested: active, lock_active: active },
            ui_snapshot: [{
                kind: "settings_menu", id: "settings_menu", root_count: 1,
                visible: true, capture_root: true, focus_policy_block: true,
                rect: [380, 20, 520, 680], descendants_truncated: false, text_truncated: false,
                text_nodes: [
                    { text: "Settings", visible: true, rect: [402, 42, 90, 26] },
                    ...(label ? [{ text: "Keep the cursor in the game", visible: true, rect: [402, 432, 250, 18] }] : []),
                    ...(enabled ? [{ text: "On", visible: true, rect: [660, 432, 22, 18] }] : []),
                ],
            }],
        } },
    };
}

function captures(options) {
    const worldReady = {
        name: "world-ready", input_call: 4, window: 1, document_epoch: 1,
        checkpoint: { debug: { ui_snapshot: [{ kind: "widget", id: "guide", visible: false }] } },
    };
    return [worldReady, capture("settings-menu-top", 7, options), capture("settings-menu", 13, options), {
        name: "joined-cursor", input_call: 18, window: 1, document_epoch: 1,
        checkpoint: { browser: { canvas_pointer_locked: options?.active !== false }, debug: { cursor: {
            lock_requested: options?.active !== false, lock_active: options?.active !== false,
            frame_index: 10, engine_position: [640, 360], relative_motion: [0, 0],
        } }, scene: { clip_from_world: Array(16).fill(0) } },
    }, {
        name: "summed-motion", input_call: 20, window: 1, document_epoch: 1,
        checkpoint: { browser: { canvas_pointer_locked: true }, debug: { cursor: {
            lock_requested: true, lock_active: true, frame_index: 20,
            engine_position: [700, 360], relative_motion: [0, 0],
        } }, scene: { clip_from_world: Array(16).fill(0) } },
    }, {
        name: "edge-move", input_call: 24, window: 1, document_epoch: 1,
        checkpoint: { browser: { canvas_pointer_locked: true }, debug: { cursor: {
            lock_requested: true, lock_active: true, frame_index: 30,
            engine_position: [1, 360], relative_motion: [-699, 0],
        } }, scene: { clip_from_world: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1] } },
    }];
}

{
    const actualCaptures = captures();
    delete actualCaptures[3].checkpoint.browser;
    assert.throws(() => judge(inputs(), actualCaptures), (error) => {
        assert.deepEqual(error.task16_semantic, { row: "CAM2", clause: "pointer-lock" });
        return true;
    }, "a missing actual DOM lock receipt cannot pass via requested state alone");
}

{
    const actualInputs = inputs();
    actualInputs[19].actual_events[0].movement_x = 0;
    assert.throws(() => judge(actualInputs, captures()), (error) => {
        assert.deepEqual(error.task16_semantic, { row: "CAM2", clause: "summed-motion-cursor" });
        return true;
    }, "engine-position change without nonzero actual locked movement is not proof of summed motion");
}

{
    const actualInputs = inputs();
    actualInputs[22].actual_events[0].canvas_pointer_locked = false;
    assert.throws(() => judge(actualInputs, captures()), (error) => {
        assert.deepEqual(error.task16_semantic, { row: "CAM2", clause: "edge-band-pan" });
        return true;
    }, "an edge position reached without actual locked motion cannot prove locked edge pan");
}

{
    assert.deepEqual([...judge(inputs(), captures())], ["pointer-lock", "summed-motion-cursor", "edge-band-pan", "f10-menu"]);
}

{
    assert.throws(() => judge(inputs(), captures({ label: false })), (error) => {
        assert.deepEqual(error.task16_semantic, { row: "CAM2", clause: "pointer-lock" });
        assert.match(error.message, /no visible Keep the cursor in the game opt-in/);
        return true;
    }, "only an actual menu with complete visible controls can establish the missing-label baseline");
}

{
    const actualInputs = inputs();
    actualInputs[9].actual_events = [];
    assert.throws(() => judge(actualInputs, captures()), (error) => {
        assert.equal(error.task16_semantic, undefined);
        assert.match(error.message, /actual physical wheel receipt/);
        return true;
    }, "missing physical scroll is setup failure, not missing product UI");
}

{
    assert.throws(() => judge(inputs(), captures({ active: false })), (error) => {
        assert.deepEqual(error.task16_semantic, { row: "CAM2", clause: "pointer-lock" });
        assert.match(error.message, /did not acquire actual browser pointer lock/);
        return true;
    }, "a real opted-in canvas click with no actual pointer-lock acquisition is a named semantic failure");
}

{
    const actualCaptures = captures();
    actualCaptures[1].checkpoint.debug.ui_snapshot = [];
    assert.throws(() => judge(inputs(), actualCaptures), (error) => {
        assert.equal(error.task16_semantic, undefined);
        assert.match(error.message, /root is absent or duplicated/);
        return true;
    }, "missing actual settings-menu snapshot data remains an instrumentation failure");
}

{
    const actualInputs = inputs();
    actualInputs[5].actual_events = [];
    assert.throws(() => judge(actualInputs, captures()), (error) => {
        assert.equal(error.task16_semantic, undefined);
        assert.match(error.message, /F10 open lacks an actual keydown sequence/);
        return true;
    }, "a missing physical F10 receipt is instrumentation failure");
}
