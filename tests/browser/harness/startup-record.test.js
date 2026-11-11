// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const test = require("node:test");
const vm = require("node:vm");
const { recordStartup } = require("./run.js");

test("the harness records the load screen while a window joins, then the joined state", async () => {
    const phases = ["FetchPackages", "VerifyPackage", "Joining", "Joined { slot: 1 }"];
    let index = 0;
    const window = { turts_debug: () => JSON.stringify({ phase: phases[Math.min(index, phases.length - 1)],
        ui_snapshot: [{ id: "shell.load.progress.track", visible: true }, { id: "hud_x", visible: true }] }) };
    const page = {
        evaluate: async (fn) => vm.runInNewContext(`(${fn.toString()})()`, { window, JSON, String }),
        waitForTimeout: async () => { index += 1; },
    };
    const observations = [];
    await recordStartup(page, 2, observations, async () => { while (index < 3) await new Promise((resolve) => setTimeout(resolve, 1)); });
    assert.ok(observations.length >= 2);
    assert.ok(observations.every((item) => item.window === 2));
    assert.ok(observations.some((item) => item.phase === "VerifyPackage"));
    const loads = observations.filter((item) => Array.isArray(item.rows));
    assert.ok(loads.every((item) => item.rows.every((row) => row.id.startsWith("shell.load"))), "only load rows are kept");
    assert.equal(observations.at(-1).keyboard_layout, null, "the layout record is present; this fake browser has no keyboard API");
});
