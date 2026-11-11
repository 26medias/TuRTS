// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const { test } = require("node:test");
const { appliedAfter, cancelRows, hudClick, produceRow } = require("./m2-oracle.js");

// Row shapes of a retained headed HUD2 run (target/mc-evidence/run94-hud2-headed-v3): list rows
// are "<list>/<item>[<key>]" buttons with a typed widget_action; row children keep their ids.
const produce = { kind: "button", id: "m2_build_rows/m2_build_item[Action:{}]", visible: true, enabled: true, text: "Produce Train Rifleman",
    rect: [16, 190, 240, 28], widget_action: { kind: "produce", producer: 1, content_id: "base:train_rifleman" } };
const cancel = { kind: "button", id: "m2_job_cancel", visible: true, enabled: true, text: "Cancel", rect: [192, 258, 64, 28],
    widget_action: { kind: "cancel_production", producer: 1, job: 7 } };

test("the Produce row is found only when it is visible, enabled and for the selected producer", () => {
    const debug = { ui_snapshot: [produce, cancel] };
    assert.equal(produceRow(debug, 1, "base:train_rifleman"), produce);
    assert.equal(produceRow(debug, 2, "base:train_rifleman"), null);
    assert.equal(produceRow({ ui_snapshot: [{ ...produce, enabled: false }] }, 1, null), null);
    assert.equal(produceRow({ ui_snapshot: [] }, 1, null), null, "a missing BuildMenu row must make the case red");
    assert.deepEqual(cancelRows(debug, 1).map((row) => row.widget_action.job), [7]);
});

test("a HUD click is the click after its button check, and the applied result must follow it", () => {
    const input = [
        { call: "screenshot" },
        { call: "save", args: ["m2-econ-first-button.json"] },
        { call: "click", args: [136, 204, "left"] },
        { call: "screenshot" },
    ];
    const before = { input_call: 0, window: 1, wall_time_ms: 1000, debug: { command_results: [] } };
    const after = { input_call: 3, window: 1, wall_time_ms: 1400, debug: { command_results: [{ seq: 1, applied_tick: 9, job_id: 4 }] } };
    const click = hudClick(input, [before, after], "m2-econ-first");
    assert.deepEqual([click.x, click.y, click.clickIndex], [136, 204, 2]);
    assert.equal(appliedAfter([before, after], click), after);
    assert.equal(appliedAfter([before, { ...after, wall_time_ms: 9000 }], click), null, "a result later than 6 s does not count");
    assert.throws(() => hudClick(input, [before, after], "m2-econ-second"), /did not check the HUD button/);
});
