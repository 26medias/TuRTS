// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const { test } = require("node:test");
const { assertConservation, portHasCapacity } = require("./m3-oracle.js");

const shell = { package: "forge", local: "shell" };
function state(onHand) {
    return { economy: { economy: { balances: [], inventories: [{ unit: 1, items: [{ resource: shell, on_hand: onHand, reserved: 0 }] }] } } };
}
function tick(n, before, after, events) {
    return { record: "tick", data: { tick: n, before: state(before), state: state(after), events } };
}

test("conservation accepts a change with an event and refuses a change without one", () => {
    assert.equal(assertConservation([tick(1, 2, 2, []), tick(2, 2, 3, [{ tick: 2 }])]), 2);
    assert.throws(() => assertConservation([tick(1, 2, 3, [])]), /forge:shell changed at tick 1 with no economy event/);
    assert.throws(() => assertConservation([tick(1, 2, -1, [{ tick: 1 }])]), /forge:shell is -1/);
    assert.throws(() => assertConservation([{ record: "initial", data: {} }]), /no completed tick/);
});

test("the press panel line check needs the port name and its catalog capacity", () => {
    const lines = ["Main: Ore 2 available (2 on hand) (capacity 6)", "Output: Shell 0 available (capacity 4)"];
    assert.equal(portHasCapacity(lines, "Main", 6), true);
    assert.equal(portHasCapacity(lines, "Output", 4), true);
    assert.equal(portHasCapacity(lines, "Output", 6), false);
    assert.equal(portHasCapacity(["Main inventory 0 available (capacity 6)"], "Main", 6), false);
});
