// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const { sourceControls } = require("./shell-oracle.js");

const required = [
    "Settings", "Key bindings", "Change", "Reset", "Audio levels",
    "Master", "Voice", "Alerts", "Effects", "Scroll speed", "Edge scroll",
    "UI scale", "Health bars", "Keep the cursor in the game", "Double-click time", "Double-click range",
];

function frame(seq, code, frameIndex, matchedAction = "menu") {
    return {
        frame_index: frameIndex, applied_seq: [seq], named_dispatches_truncated: 0,
        named_dispatches: [{ origin: "key", code, matched_action: matchedAction }],
    };
}

function menuNode(labels, overrides = {}) {
    return {
        kind: "settings_menu", id: "settings_menu", visible: true, capture_root: true,
        focus_policy_block: true, rect: [380, 20, 520, 680], descendants_truncated: false,
        text_truncated: false,
        text_nodes: labels.map((text) => ({ text, visible: true, rect: [390, 40, 100, 20] })),
        ...overrides,
    };
}

function fixture() {
    const input = [
        { call: "key", args: ["F10", []], window: 1, document_epoch: 1,
            actual_events: [{ type: "keydown", code: "F10", seq: 10 }] },
        { call: "screenshot", args: [], window: 1, document_epoch: 1 },
        { call: "move", args: [640, 360], window: 1, document_epoch: 1 },
        { call: "wheel", args: [900], window: 1, document_epoch: 1,
            actual_events: [{ type: "wheel", seq: 11 }] },
        { call: "screenshot", args: [], window: 1, document_epoch: 1 },
        { call: "key", args: ["Escape", []], window: 1, document_epoch: 1,
            actual_events: [{ type: "keydown", code: "Escape", seq: 12 }] },
        { call: "screenshot", args: [], window: 1, document_epoch: 1 },
    ];
    const top = menuNode(["Settings", "Key bindings", "Change", "Reset"]);
    const bottom = menuNode([
        "Audio levels", "Master", "Voice", "Alerts", "Effects", "Scroll speed", "Edge scroll", "On",
        "UI scale", "Health bars", "Damaged or selected", "Keep the cursor in the game", "Off", "Double-click time", "Double-click range",
    ]);
    const toggleY = new Map([
        ["Edge scroll", 302], ["On", 302], ["Health bars", 390], ["Damaged or selected", 390],
        ["Keep the cursor in the game", 432], ["Off", 432],
    ]);
    bottom.text_nodes = bottom.text_nodes.map((node, index) => {
        const state = ["On", "Off", "Damaged or selected"].includes(node.text);
        return { ...node, rect: [state ? 661 : 410, toggleY.get(node.text) ?? 40 + index * 20, state ? 130 : 150, 20] };
    });
    const capture = (input_call, node, frames = []) => ({
        input_call, window: 1, document_epoch: 1,
        checkpoint: { debug: { ui_snapshot: [node], frames: { recent: frames, retained_input: [] } } },
    });
    const captures = [
        capture(1, top, [frame(10, "F10", 1)]),
        capture(4, bottom),
        capture(6, null, [frame(12, "Escape", 2)]),
    ];
    captures[2].checkpoint.debug.ui_snapshot = [];
    return { input, captures, top, bottom };
}

const judge = sourceControls.judgeSh1MenuControls;

assert(judge, "the fixed SH1 menu evaluator must be available to source controls");
{
    const { input, captures } = fixture();
    assert.deepEqual([...judge(input, captures)], ["menu-controls"],
        "visible labels across top/bottom scroll captures in one F10-to-Escape episode should pass this source control");
}
{
    const { input, captures } = fixture();
    captures[1].document_epoch = 2;
    assert.throws(() => judge(input, captures), /needs actual top and bottom screenshots/,
        "menu labels from a different document epoch must not be combined");
}
{
    const { input, captures } = fixture();
    input.splice(5, 0, { call: "key", args: ["F10", []], window: 1, document_epoch: 1,
        actual_events: [{ type: "keydown", code: "F10", seq: 12 }] });
    input[6].actual_events[0].seq = 13;
    captures[2].checkpoint.debug.frames.recent = [frame(13, "Escape", 2)];
    captures[2].input_call = 7;
    assert.throws(() => judge(input, captures), /cannot combine multiple F10 open\/close episodes/,
        "an unobserved F10 close/reopen must split the menu episode instead of combining snapshots");
}
{
    const { input, captures } = fixture();
    input[3].actual_events = [];
    assert.throws(() => judge(input, captures), (error) => {
        assert.match(error.message, /missing the actual same-document wheel receipt/);
        assert.equal(error.task16_semantic, undefined, "missing physical input evidence must not be labeled a product semantic failure");
        return true;
    }, "top and bottom captures without an actual wheel receipt must fail as an instrument error");
}
{
    const { input, captures } = fixture();
    captures.splice(1, 1);
    assert.throws(() => judge(input, captures), /needs actual top and bottom screenshots/,
        "a missing bottom screenshot must fail closed");
}
{
    const { input, captures } = fixture();
    captures[1].checkpoint.debug.ui_snapshot[0].visible = false;
    assert.throws(() => judge(input, captures), /was not visible with its bounded capture root/,
        "a hidden menu root must not count labels from that capture");
}
{
    const { input, captures } = fixture();
    const texts = captures[1].checkpoint.debug.ui_snapshot[0].text_nodes;
    texts.find((node) => node.text === "Scroll speed").visible = false;
    assert.throws(() => judge(input, captures), /missing visible label "Scroll speed"/,
        "an offscreen required label must not count toward the visible inventory");
}
{
    const { input, captures } = fixture();
    captures[1].checkpoint.debug.ui_snapshot[0].text_truncated = true;
    assert.throws(() => judge(input, captures), /text observation overflowed/,
        "truncated UI text must be an instrumentation failure");
}
