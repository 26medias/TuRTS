// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const { sourceControls } = require("./shell-oracle.js");

const valid = () => ({
    ui_snapshot: [{
        kind: "focus_overlay", id: "focus_overlay", root_count: 1,
        rect: [0, 0, 1280, 720], visible: true, capture_root: true, focus_policy_block: true,
        descendants_visited: 1, descendants_truncated: false, text_truncated: false,
        text_nodes: [{ text: "Click to play", rect: [540, 340, 200, 40], visible: true }],
    }],
});

const judge = sourceControls.judgeKm2FocusOverlay;
assert(judge, "the fixed KM2 focus-overlay evaluator must be available to source controls");

{
    const result = judge(valid());
    assert.equal(result.root_count, 1);
}
{
    const debug = valid();
    debug.ui_snapshot[0].root_count = 0;
    debug.ui_snapshot[0].rect = null;
    debug.ui_snapshot[0].visible = false;
    assert.throws(() => judge(debug), (error) => {
        assert.deepEqual(error.task16_semantic, { row: "KM2", clause: "canvas-blur-overlay" });
        return true;
    }, "an actual absent focus overlay must be a named product outcome");
}
{
    const debug = valid();
    debug.ui_snapshot[0].visible = false;
    assert.throws(() => judge(debug), (error) => {
        assert.deepEqual(error.task16_semantic, { row: "KM2", clause: "canvas-blur-overlay" });
        return true;
    }, "an actual laid-out but hidden focus overlay must fail the product outcome");
}
{
    const debug = valid();
    debug.ui_snapshot[0].text_nodes[0].visible = false;
    assert.throws(() => judge(debug), (error) => {
        assert.deepEqual(error.task16_semantic, { row: "KM2", clause: "canvas-blur-overlay" });
        return true;
    }, "an offscreen focus-overlay label must not count as visible");
}
{
    const debug = valid();
    debug.ui_snapshot[0].text_truncated = true;
    assert.throws(() => judge(debug), (error) => {
        assert.equal(error.task16_semantic, undefined);
        assert.match(error.message, /text observation overflowed/);
        return true;
    }, "snapshot overflow must remain an instrumentation error");
}
{
    const debug = valid();
    debug.ui_snapshot.push({ kind: "focus_overlay_overflow", count_at_least: 5 });
    assert.throws(() => judge(debug), (error) => {
        assert.equal(error.task16_semantic, undefined);
        assert.match(error.message, /root observation overflowed/);
        return true;
    }, "root overflow must not be accepted as absence or presence");
}
