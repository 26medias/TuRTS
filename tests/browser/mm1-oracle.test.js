// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { test } = require("node:test");
const { assertOrderFromInput, MAX_REPLICA_LAG_TICKS, INTERP_DELAY_TICKS } = require("./mm1-oracle.js");

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
const check = (debug, ...rest) => assertOrderFromInput(debug, CLICK, MOVE, UNIT, { point: [POINT.x, POINT.y] }, "move", ...rest);

test("MM1 accepts the move order from the one capture when its replica has reached the accept tick", () => {
    assert.equal(check(capture(500, MOVE)).tick, 494);
    assert.equal(check(capture(494, MOVE)).tick, 494, "a replica exactly at the accept tick counts");
});

test("MM1 is red for a replica behind the accept tick, and a later capture does not rescue it (RUN142, the build without the stall repair)", () => {
    // The replica stood still at 491 (the kill stall). Before: the oracle waited for a later capture. Now: red.
    assert.throws(() => check(capture(491, ATTACK)), /did not reach the server accept tick 494 \(replica tick 491\)/);
    assert.throws(() => check(capture(491, ATTACK), [capture(520, MOVE)]), /did not reach the server accept tick 494 \(replica tick 491\)/);
    // A replica that reached the accept tick but shows another order is red.
    assert.throws(() => check(capture(520, ATTACK)), /must still name the accepted order/);
    // The move on the wrong ground point, or a refused result.
    assert.throws(() => check(capture(520, MOVE, { point: { x: 10, y: 10 } })), /must match the accepted point/);
    assert.throws(() => check(capture(520, MOVE, { status: "rejected" })), /server must accept the exact emitted sequence/);
});

test("the replica lag bound is the interpolation delay plus 2 ticks, and the delay is the value in the client source", () => {
    const source = fs.readFileSync(path.join(__dirname, "..", "..", "crates", "turts_client", "src", "interp.rs"), "utf8");
    const delay = /pub const INTERP_DELAY_TICKS: f64 = ([0-9.]+);/.exec(source);
    assert.ok(delay, "crates/turts_client/src/interp.rs declares INTERP_DELAY_TICKS");
    assert.equal(Number(delay[1]), INTERP_DELAY_TICKS, "the oracle copy of the interpolation delay equals interp.rs:10");
    assert.equal(MAX_REPLICA_LAG_TICKS, INTERP_DELAY_TICKS + 2);
});
