'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { SettledTracker, DistantOrderTracker } = require('./settled-tracker');
const at = (tick, ownMovers = [], bodies = []) => ({tick, ownMovers, bodies});
// A tracker with two units of one move order; 'a' settles first.
function order() {
    const tracker = new SettledTracker();
    tracker.begin(['a', 'b']); tracker.commit(['a', 'b'], 'move');
    tracker.observe('a', true, [10, 10], 0.5, at(0));
    tracker.observe('b', true, [20, 20], 0.5, at(1));
    return tracker;
}
test('(1) contact while own units move is allowed, at any distance', () => {
    const tracker = order();
    tracker.observe('b', true, [23, 20], 0.5, at(5, [[24, 20]]));
    tracker.observe('b', true, [20.4, 20], 0.5, at(40, [[22, 20]]));
    tracker.observe('b', true, [21.5, 20], 0.5, at(41, [], [[23, 20]]));
    tracker.observe('b', true, [21.5, 20], 0.5, at(99, [], [[23, 20]]));
    tracker.observe('b', true, [20.2, 20], 0.5, at(100));
});
test('(1) at rest (60 quiet ticks) a unit beyond 2 x radius fails; inside passes', () => {
    const near = order();
    near.observe('b', true, [20.9, 20], 0.5, at(10, [[22, 20]]));
    near.observe('b', true, [20.9, 20], 0.5, at(11));
    assert.throws(() => near.observe('b', true, [20.9, 20], 0.5, at(72)), /did not go back/);
    const far = order();
    far.observe('b', true, [21.1, 20], 0.5, at(10, [[22, 20]]));
    far.observe('b', true, [21.1, 20], 0.5, at(11, [], [[20.5, 20]]));
    assert.throws(() => far.observe('b', true, [21.1, 20], 0.5, at(71, [], [[22.5, 20]])), /at rest is more than one body diameter/);
});
test('(1) an occupied settle point excuses a later arrival but never the first arrival of its order', () => {
    const tracker = order();
    tracker.observe('b', true, [21.5, 20], 0.5, at(10, [[23, 20]]));
    tracker.observe('b', true, [21.5, 20], 0.5, at(11, [], [[20.2, 20]]));
    tracker.observe('b', true, [21.5, 20], 0.5, at(200, [], [[20.2, 20]]));
    tracker.observe('a', true, [11.5, 10], 0.5, at(10, [[13, 10]]));
    tracker.observe('a', true, [11.5, 10], 0.5, at(11, [], [[10.2, 10]]));
    assert.throws(() => tracker.observe('a', true, [11.5, 10], 0.5, at(71, [], [[10.2, 10]])), /first arrival of its order/);
});
test('(2) a pushed unit that goes back within one radius in 60 quiet ticks passes', () => {
    const tracker = order();
    tracker.observe('b', true, [20.8, 20], 0.5, at(10, [[21.5, 20]]));
    tracker.observe('b', true, [20.8, 20], 0.5, at(11));
    tracker.observe('b', true, [20.8, 20], 0.5, at(71));
    tracker.observe('b', true, [20.3, 20], 0.5, at(72));
    tracker.observe('b', true, [20.3, 20], 0.5, at(500));
});
test('(2) the quiet time starts again when an own mover comes within two diameters', () => {
    const tracker = order();
    tracker.observe('b', true, [20.8, 20], 0.5, at(11));
    tracker.observe('b', true, [20.8, 20], 0.5, at(60, [[22.7, 20]]));
    tracker.observe('b', true, [20.8, 20], 0.5, at(61));
    tracker.observe('b', true, [20.8, 20], 0.5, at(121));
    assert.throws(() => tracker.observe('b', true, [20.8, 20], 0.5, at(122)), /did not go back/);
});
test('(2) is measured from the FIRST settle point: small steps that add up are caught', () => {
    const tracker = order();
    for (const [i, x] of [20.2, 20.4, 20.6].entries()) tracker.observe('b', true, [x, 20], 0.5, at(2 + i, [[x + 1, 20]]));
    tracker.observe('b', true, [20.6, 20], 0.5, at(5));
    assert.throws(() => tracker.observe('b', true, [20.6, 20], 0.5, at(66)), /did not go back/);
});
test('a new order clears the settle point; a unit needs its radius and the context', () => {
    const tracker = order();
    tracker.begin(['b']); tracker.commit(['b'], 'move');
    tracker.observe('b', true, [30, 30], 0.5, at(100));
    tracker.observe('b', true, [30, 30], 0.5, at(300));
    assert.throws(() => tracker.observe('b', true, [30, 30], 0, at(301)), /needs its body radius/);
    assert.throws(() => tracker.observe('b', true, [30, 30], 0.5), /needs the tick/);
    assert.throws(() => tracker.commit(['other'], 'move'), /No pending order/);
});

test('distant order must be admitted by tick 340 and stay unchanged', () => {
    const tracker = new DistantOrderTracker();
    tracker.observe(20, null, [4, 46]);
    const order = { order: 'quarry:order_move', target: [22, 2] };
    tracker.observe(21, order, [4, 46]);
    tracker.observe(22, order, [4.1, 45.9]);
    assert.equal(tracker.moved, true);
    assert.throws(() => tracker.observe(23, null, [4.1, 45.9]), /order changed/);
    const missing = new DistantOrderTracker();
    assert.throws(() => missing.observe(341, null, [4, 46]), /not observed by tick 340/);
});
