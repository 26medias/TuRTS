// turts-test-role: unit
'use strict';
const test = require('node:test'), assert = require('node:assert/strict');
const {orderAccepted} = require('./terrain-fps-orders');
const MOVE = ':order_move';
const state = (orders, results, positions = [[0, 0], [0, 0]]) => ({
    command_results: results,
    replicated_world: {unit_orders: orders, units: [{unit_id: 1, pos: positions[0]}, {unit_id: 2, pos: positions[1]}]},
});
const moving = [{unit: 1, current: {order: 'pkg:order_move'}}, {unit: 2, current: {order: 'pkg:order_move'}}];
const idle = [{unit: 1, current: null}, {unit: 2, current: null}];
const before = new Set([4]);
test('an order in progress is accepted (the old rule)', () => {
    assert.equal(orderAccepted(before, state(moving, []), [1, 2], MOVE), 'in progress');
});
test('a move that started and ended between two reads is accepted by its new command result', () => {
    const done = state(idle, [{seq: 4, accepted: [1, 2], refused: [], applied_tick: 10}, {seq: 5, accepted: [1, 2], refused: [], applied_tick: 20}]);
    assert.equal(orderAccepted(before, done, [1, 2], MOVE), 'accepted result');
});
test('an old command result does not count', () => {
    const old = state(idle, [{seq: 4, accepted: [1, 2], refused: [], applied_tick: 10}]);
    assert.equal(orderAccepted(before, old, [1, 2], MOVE), null);
});
test('a new result that accepts only one group unit does not count', () => {
    const half = state(idle, [{seq: 5, accepted: [1], refused: [], applied_tick: 20}]);
    assert.equal(orderAccepted(before, half, [1, 2], MOVE), null);
});
test('a planted refusal of a group unit fails at once (real result shapes)', () => {
    const partly = state(idle, [{seq: 5, applied_tick: 20, job_id: null, accepted: [1], refused: [{unit: 2, reason: 'blocked'}]}]);
    assert.throws(() => orderAccepted(before, partly, [1, 2], MOVE), /Group order refused.*"unit":2/);
    const none = state(idle, [{seq: 5, rejected_tick: 20, rejected: 'not_owner', accepted: [], refused: [{unit: 1, reason: 'not_owner'}, {unit: 2, reason: 'not_owner'}]}]);
    assert.throws(() => orderAccepted(before, none, [1, 2], MOVE), /Group order refused/);
    const command = state(idle, [{seq: 5, rejected_tick: 20, rejected: 'invalid_target'}]);
    assert.throws(() => orderAccepted(before, command, [1, 2], MOVE), /Group order refused.*invalid_target/);
    const other = state(moving, [{seq: 5, applied_tick: 20, job_id: null, accepted: [1, 2], refused: [{unit: 9, reason: 'not_owner'}]}]);
    assert.equal(orderAccepted(before, other, [1, 2], MOVE), 'in progress');
});
test('units at the goal count; idle units away from the goal with no new result do not', () => {
    const atGoal = p => Math.hypot(p[0] - 14, p[1] - 32) < 2;
    assert.equal(orderAccepted(before, state(idle, [], [[14, 32], [13, 31]]), [1, 2], MOVE, atGoal), 'at goal');
    assert.equal(orderAccepted(before, state(idle, [], [[14, 32], [20, 20]]), [1, 2], MOVE, atGoal), null);
});
