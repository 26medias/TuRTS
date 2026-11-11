// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const { test } = require("node:test");
const { assertConservationStream, portHasCapacity } = require("./m3-oracle.js");

const shell = { package: "forge", local: "shell" };
function state(tickValue, onHand) {
    return { economy: { tick: tickValue, economy: { balances: [], inventories: [{ unit: 1, items: [{ resource: shell, on_hand: onHand, reserved: 0 }] }] } } };
}
function lines(records) {
    return (async function* () { for (const record of records) yield typeof record === "string" ? record : JSON.stringify(record); })();
}
const initial = { record: "initial", data: state(0, 2) };
const tick = (n, before, after) => ({ record: "tick", data: { tick: n, before: state(n - 1, before), state: state(n, after), events: [] } });
const operation = '{"record":"operation","data":{}}';

test("streamed conservation: continuity, no negative amount, and no change without a logged operation", async () => {
    assert.equal(await assertConservationStream(lines([initial, tick(1, 2, 2), operation, tick(2, 2, 3)])), 2);
    await assert.rejects(assertConservationStream(lines([initial, tick(1, 2, 3)])), /forge:shell changed at tick 1 with no logged operation/);
    await assert.rejects(assertConservationStream(lines([initial, operation, tick(1, 2, -1)])), /forge:shell is -1/);
    await assert.rejects(assertConservationStream(lines([initial, tick(2, 2, 2)])), /discontinuous/);
    await assert.rejects(assertConservationStream(lines([initial, tick(1, 5, 5)])), /before-state differs/);
    await assert.rejects(assertConservationStream(lines([initial])), /no completed tick/);
    assert.equal(await assertConservationStream(lines([initial, tick(1, 2, 2), '{"record":"ti'])), 1, "a torn last line is cut");
    await assert.rejects(assertConservationStream(lines([initial, '{"record":"ti', tick(1, 2, 2)])), /invalid economy audit line/);
});

test("the press panel line check needs the port name and its catalog capacity", () => {
    const lines = ["Main: Ore 2 available (2 on hand) (capacity 6)", "Output: Shell 0 available (capacity 4)"];
    assert.equal(portHasCapacity(lines, "Main", 6), true);
    assert.equal(portHasCapacity(lines, "Output", 4), true);
    assert.equal(portHasCapacity(lines, "Output", 6), false);
    assert.equal(portHasCapacity(["Main inventory 0 available (capacity 6)"], "Main", 6), false);
});

const { firstPassing } = require("./m3-oracle.js");

test("an M3 check passes on the first passing screenshot of its series and fails when none passes", () => {
    const input = [{ call: "screenshot" }, { call: "save", args: ["m3-x-00.png"] }, { call: "screenshot" }, { call: "save", args: ["m3-x-01.png"] }];
    const checkpoints = [{ input_call: 0, window: 1, scene: {}, debug: { done: false } }, { input_call: 2, window: 1, scene: {}, debug: { done: true } }];
    assert.equal(firstPassing(input, checkpoints, "m3-x", (shot) => shot.debug.done, "late result").input_call, 2);
    assert.throws(() => firstPassing(input, checkpoints, "m3-x", () => false, "no result"), /no result/);
    assert.throws(() => firstPassing(input, checkpoints, "m3-y", () => true, "no series"), /no screenshot after the action/);
});
