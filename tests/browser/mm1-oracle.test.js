// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const { test } = require("node:test");
const { assertOrderFromInput } = require("./mm1-oracle.js");

const UNIT = 40;
const POINT = { x: 61.2, y: 2.4 };
const MOVE = "muster:order_move";
const ATTACK = "muster:order_attack";

// One capture (the debug of one checkpoint). The replica is at `replicaTick` and shows `order` for
// the unit. The retained frames hold the applied click and the server result (accepted at tick 494).
function capture(replicaTick, order, { status = "accepted", point = POINT } = {}) {
    return {
        frames: {
            retained_input: [
                { applied_seq: [5], emitted_orders: [{ order: MOVE, seq: 2, units: [UNIT], target: { Point: POINT } }], results: [] },
                { applied_seq: [], emitted_orders: [], results: [{ seq: 2, status, tick: 494, unit: UNIT }] },
            ],
        },
        replicated_world: { tick: replicaTick, unit_orders: [{ unit: UNIT, current: { order, target: { Point: point } } }] },
    };
}

const CLICK = { event_seq: [3, 4, 5] };
const check = (debug, later = []) => assertOrderFromInput(debug, CLICK, MOVE, UNIT, { point: [POINT.x, POINT.y] }, "move", later);

test("MM1 reads the replicated order from the first capture that reached the accept tick (RUN142)", () => {
    assert.equal(check(capture(500, MOVE)).tick, 494, "the first capture is already past the accept tick");
    // The replica stood still at 491 (the kill stall): the later capture is the first one that counts.
    assert.equal(check(capture(491, ATTACK), [capture(520, MOVE)]).tick, 494);
    assert.equal(check(capture(491, ATTACK), [capture(493, ATTACK), capture(525, MOVE)]).tick, 494);
});

test("MM1 stays red when the replica never reaches the accept tick or never shows the move (the build that lacks the fact)", () => {
    // A replica that stays at 491 in every capture.
    assert.throws(() => check(capture(491, ATTACK), [capture(491, ATTACK), capture(491, ATTACK)]), /did not reach the server accept tick 494 in 3 captures/);
    // A product that never replicates the move: every capture past the accept tick still shows the attack.
    assert.throws(() => check(capture(491, ATTACK), [capture(520, ATTACK), capture(540, ATTACK)]), /must still name the accepted order/);
    // The first capture past the accept tick decides: a later capture with the move does not rescue it.
    assert.throws(() => check(capture(494, ATTACK), [capture(540, MOVE)]), /must still name the accepted order/);
    // The move on the wrong ground point, or a refused result.
    assert.throws(() => check(capture(520, MOVE, { point: { x: 10, y: 10 } })), /must match the accepted point/);
    assert.throws(() => check(capture(520, MOVE, { status: "rejected" })), /server must accept the exact emitted sequence/);
});
